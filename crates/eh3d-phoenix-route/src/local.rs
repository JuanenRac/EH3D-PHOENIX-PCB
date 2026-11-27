// SPDX-License-Identifier: GPL-3.0-or-later
//! The check of the zone that an edit touches: the findings of a board that an edit made from a
//! board whose findings are known, without judging the whole board again.
//!
//! Every rule of [`crate::live::findings`] is about one object, or about two objects that are near
//! each other (the edge of the board counts as one of them and is always there). So two small
//! boards are made, one from the board before and one from the board after, each with the objects
//! that the edit changed and with the objects that did not change and are near one of them; the
//! findings that the two small boards do not share are exactly the ones that the edit removes and
//! adds on the whole board. The result is the same list as the check of the whole board.
//!
//! One kind of finding is not local: copper of several nets that is joined through a pad with no
//! net. When the edit could be part of one, or when anything else cannot be judged this way,
//! [`findings_near`] answers `None` and the whole board has to be checked.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Component, Id, PadRef, PadShape, Segment, SilkItem, Track, Via};
use eh3d_phoenix_rules::{codes, Finding};

use crate::live::{findings, order, RuleContext};

/// The most boxes of changed objects that are looked around; an edit that changes more is checked
/// on the whole board.
const MAX_CHANGED_BOXES: usize = 4_096;

/// An edit that changes more objects than this, and more than one object in four, is checked on
/// the whole board: that costs less than two small boards.
const MANY_CHANGED: usize = 400;

/// A box as `[left, bottom, right, top]`, in nanometres.
type Square = [i64; 4];

/// Copper that is certainly there: a straight piece (a point when both ends are the same) and
/// half its width.
#[derive(Clone, Copy)]
struct Core {
    a: (f64, f64),
    b: (f64, f64),
    r: f64,
}

fn around(ax: i64, ay: i64, bx: i64, by: i64, r: i64) -> Square {
    [
        ax.min(bx).saturating_sub(r),
        ay.min(by).saturating_sub(r),
        ax.max(bx).saturating_add(r),
        ay.max(by).saturating_add(r),
    ]
}

fn meet(a: &Square, b: &Square) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

/// The places of the changed objects, grown by the reach of the rules.
struct Zone {
    boxes: Vec<Square>,
    all: Square,
}

impl Zone {
    fn new() -> Zone {
        Zone {
            boxes: Vec::new(),
            all: [i64::MAX, i64::MAX, i64::MIN, i64::MIN],
        }
    }

    fn add(&mut self, b: Square, reach: i64) {
        let grown = [
            b[0].saturating_sub(reach),
            b[1].saturating_sub(reach),
            b[2].saturating_add(reach),
            b[3].saturating_add(reach),
        ];
        self.all = [
            self.all[0].min(grown[0]),
            self.all[1].min(grown[1]),
            self.all[2].max(grown[2]),
            self.all[3].max(grown[3]),
        ];
        self.boxes.push(grown);
    }

    fn touches(&self, b: &Square) -> bool {
        meet(&self.all, b) && self.boxes.iter().any(|c| meet(c, b))
    }
}

/// The boxes of the pieces of a track; `None` when it has an arc, whose place the end points do
/// not tell.
fn track_boxes(track: &Track) -> Option<Vec<Square>> {
    let r = track.width.get() / 2 + 1;
    track
        .path
        .iter()
        .map(|segment| match segment {
            Segment::Line { start, end } => Some(around(
                start.x_nm.get(),
                start.y_nm.get(),
                end.x_nm.get(),
                end.y_nm.get(),
                r,
            )),
            Segment::Arc { .. } => None,
        })
        .collect()
}

fn via_box(via: &Via) -> Square {
    let (x, y) = (via.at.x_nm.get(), via.at.y_nm.get());
    around(x, y, x, y, via.diameter.get().max(via.drill.get()) / 2 + 1)
}

fn sides(shape: PadShape) -> (i64, i64) {
    match shape {
        PadShape::Circle { diameter } => (diameter.get(), diameter.get()),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            (width.get(), height.get())
        }
    }
}

/// A box for each pad of a component, large enough for the pad at any angle and for its hole.
fn component_boxes(board: &Board, id: Id, component: &Component) -> Vec<Square> {
    let Some(footprint) = board.footprints.get(&component.footprint) else {
        return Vec::new();
    };
    footprint
        .pads
        .iter()
        .filter_map(|(pad_id, pad)| {
            let at = pad_position(
                board,
                PadRef {
                    component: id,
                    pad: *pad_id,
                },
            )?;
            let (w, h) = sides(pad.shape);
            let reach = w.saturating_add(h).max(pad.drill.map_or(0, |d| d.get()));
            let (x, y) = (at.x_nm.get(), at.y_nm.get());
            Some(around(x, y, x, y, reach / 2 + 1))
        })
        .collect()
}

/// The boxes of the printed pieces of a silkscreen item; `None` when it has an arc. An item that
/// cannot be drawn gives no finding and has no box.
fn silk_boxes(item: &SilkItem) -> Option<Vec<Square>> {
    let Ok(drawn) = item.drawn() else {
        return Some(Vec::new());
    };
    let r = drawn.width.get() / 2 + 1;
    drawn
        .pieces
        .iter()
        .map(|segment| match segment {
            Segment::Line { start, end } => Some(around(
                start.x_nm.get(),
                start.y_nm.get(),
                end.x_nm.get(),
                end.y_nm.get(),
                r,
            )),
            Segment::Arc { .. } => None,
        })
        .collect()
}

/// The keys whose value is not the same in the two maps.
fn changed_keys<V: PartialEq>(before: &BTreeMap<Id, V>, after: &BTreeMap<Id, V>) -> BTreeSet<Id> {
    let mut out: BTreeSet<Id> = before
        .iter()
        .filter(|(id, value)| after.get(*id) != Some(*value))
        .map(|(id, _)| *id)
        .collect();
    out.extend(after.keys().filter(|id| !before.contains_key(*id)));
    out
}

/// What is not the same in the two boards.
struct Changed {
    components: BTreeSet<Id>,
    tracks: BTreeSet<Id>,
    vias: BTreeSet<Id>,
    silk: BTreeSet<Id>,
    zones: bool,
}

/// What changed, or `None` when something changed that the whole board depends on.
fn changed(before: &Board, after: &Board) -> Option<Changed> {
    // every field is named, so that a new one cannot be forgotten here
    let Board {
        name: _,
        outline,
        outline_cutouts,
        outline_arcs,
        layers,
        copper_stack,
        nets,
        footprints,
        components,
        pad_nets,
        tracks,
        vias,
        zones,
        silkscreen,
        test_points: _,
        dimensions: _,
    } = before;
    if *outline != after.outline
        || *outline_cutouts != after.outline_cutouts
        || *outline_arcs != after.outline_arcs
        || *layers != after.layers
        || *copper_stack != after.copper_stack
        || *nets != after.nets
        || *footprints != after.footprints
    {
        return None;
    }
    let mut parts = changed_keys(components, &after.components);
    if *pad_nets != after.pad_nets {
        let old: BTreeSet<(Id, Id, Id)> = pad_nets
            .iter()
            .map(|p| (p.component, p.pad, p.net))
            .collect();
        let new: BTreeSet<(Id, Id, Id)> = after
            .pad_nets
            .iter()
            .map(|p| (p.component, p.pad, p.net))
            .collect();
        parts.extend(old.symmetric_difference(&new).map(|p| p.0));
    }
    Some(Changed {
        components: parts,
        tracks: changed_keys(tracks, &after.tracks),
        vias: changed_keys(vias, &after.vias),
        silk: changed_keys(silkscreen, &after.silkscreen),
        zones: *zones != after.zones,
    })
}

/// The places of the changed objects as they are on one board.
fn mark(zone: &mut Zone, board: &Board, what: &Changed, reach: i64) -> Option<()> {
    for id in &what.tracks {
        if let Some(track) = board.tracks.get(id) {
            for b in track_boxes(track)? {
                zone.add(b, reach);
            }
        }
    }
    for id in &what.vias {
        if let Some(via) = board.vias.get(id) {
            zone.add(via_box(via), reach);
        }
    }
    for id in &what.components {
        if let Some(component) = board.components.get(id) {
            for b in component_boxes(board, *id, component) {
                zone.add(b, reach);
            }
        }
    }
    for id in &what.silk {
        if let Some(item) = board.silkscreen.get(id) {
            for b in silk_boxes(item)? {
                zone.add(b, reach);
            }
        }
    }
    (zone.boxes.len() <= MAX_CHANGED_BOXES).then_some(())
}

/// The objects of a small board.
#[derive(Default)]
struct Picked {
    components: BTreeSet<Id>,
    tracks: BTreeSet<Id>,
    vias: BTreeSet<Id>,
    silk: BTreeSet<Id>,
}

/// The objects that did not change and are near a changed one.
fn neighbours(board: &Board, what: &Changed, zone: &Zone) -> Picked {
    let mut picked = Picked::default();
    for (id, track) in &board.tracks {
        let near = !what.tracks.contains(id)
            && track_boxes(track).is_none_or(|boxes| boxes.iter().any(|b| zone.touches(b)));
        if near {
            picked.tracks.insert(*id);
        }
    }
    for (id, via) in &board.vias {
        if !what.vias.contains(id) && zone.touches(&via_box(via)) {
            picked.vias.insert(*id);
        }
    }
    for (id, component) in &board.components {
        if !what.components.contains(id)
            && component_boxes(board, *id, component)
                .iter()
                .any(|b| zone.touches(b))
        {
            picked.components.insert(*id);
        }
    }
    for (id, item) in &board.silkscreen {
        let near = !what.silk.contains(id)
            && silk_boxes(item).is_none_or(|boxes| boxes.iter().any(|b| zone.touches(b)));
        if near {
            picked.silk.insert(*id);
        }
    }
    picked
}

/// The board with only the picked objects and the changed ones, as they are on `board`. The
/// zones, which the rules only compare with each other, are there when one of them changed.
fn small(board: &Board, what: &Changed, near: &Picked) -> Board {
    fn some<V: Clone>(
        all: &BTreeMap<Id, V>,
        changed: &BTreeSet<Id>,
        near: &BTreeSet<Id>,
    ) -> BTreeMap<Id, V> {
        changed
            .iter()
            .chain(near)
            .filter_map(|id| all.get(id).map(|v| (*id, v.clone())))
            .collect()
    }
    let components = some(&board.components, &what.components, &near.components);
    let tracks = some(&board.tracks, &what.tracks, &near.tracks);
    let vias = some(&board.vias, &what.vias, &near.vias);
    let footprints = components
        .values()
        .filter_map(|c| {
            board
                .footprints
                .get(&c.footprint)
                .map(|f| (c.footprint, f.clone()))
        })
        .collect();
    let pad_nets: Vec<_> = board
        .pad_nets
        .iter()
        .filter(|p| components.contains_key(&p.component))
        .cloned()
        .collect();
    let zones = if what.zones {
        board.zones.clone()
    } else {
        BTreeMap::new()
    };
    let nets = pad_nets
        .iter()
        .map(|p| p.net)
        .chain(tracks.values().map(|t| t.net))
        .chain(vias.values().map(|v| v.net))
        .chain(zones.values().map(|z| z.net))
        .filter_map(|id| board.nets.get(&id).map(|n| (id, n.clone())))
        .collect();
    Board {
        name: String::new(),
        outline: board.outline.clone(),
        outline_cutouts: board.outline_cutouts.clone(),
        outline_arcs: board.outline_arcs.clone(),
        layers: board.layers.clone(),
        copper_stack: board.copper_stack.clone(),
        nets,
        footprints,
        silkscreen: some(&board.silkscreen, &what.silk, &near.silk),
        components,
        pad_nets,
        tracks,
        vias,
        zones,
        test_points: Vec::new(),
        dimensions: BTreeMap::new(),
    }
}

/// Pieces of certain copper, each with whether it belongs to a changed object.
type Cores = Vec<(Core, bool)>;

/// The certain copper of the objects of a small board, and apart that of the pads that have no
/// net.
fn cores(board: &Board, what: &Changed) -> (Cores, Cores) {
    let mut netted = Vec::new();
    let mut loose = Vec::new();
    let at = |x: i64, y: i64| (x as f64, y as f64);
    for (id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let new = what.components.contains(id);
        for (pad_id, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *id,
                pad: *pad_id,
            };
            let Some(place) = pad_position(board, pad_ref) else {
                continue;
            };
            let (w, h) = sides(pad.shape);
            let p = at(place.x_nm.get(), place.y_nm.get());
            let core = Core {
                a: p,
                b: p,
                r: w.min(h) as f64 / 2.0,
            };
            if board.net_of(pad_ref).is_some() {
                netted.push((core, new));
            } else {
                loose.push((core, new));
            }
        }
    }
    for (id, track) in &board.tracks {
        let new = what.tracks.contains(id);
        for segment in &track.path {
            let (s, e) = (segment.start(), segment.end());
            netted.push((
                Core {
                    a: at(s.x_nm.get(), s.y_nm.get()),
                    b: at(e.x_nm.get(), e.y_nm.get()),
                    r: track.width.get() as f64 / 2.0,
                },
                new,
            ));
        }
    }
    for (id, via) in &board.vias {
        let p = at(via.at.x_nm.get(), via.at.y_nm.get());
        netted.push((
            Core {
                a: p,
                b: p,
                r: via.diameter.get() as f64 / 2.0,
            },
            what.vias.contains(id),
        ));
    }
    (netted, loose)
}

fn point_to_piece(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// Whether two cores may touch within the tolerance. It never says no when they do.
fn may_touch(x: &Core, y: &Core, tolerance: f64) -> bool {
    let side = |a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    };
    let crossing = side(x.a, x.b, y.a) * side(x.a, x.b, y.b) < 0.0
        && side(y.a, y.b, x.a) * side(y.a, y.b, x.b) < 0.0;
    crossing
        || point_to_piece(x.a, y.a, y.b)
            .min(point_to_piece(x.b, y.a, y.b))
            .min(point_to_piece(y.a, x.a, x.b))
            .min(point_to_piece(y.b, x.a, x.b))
            <= x.r + y.r + tolerance + 4.0
}

/// Whether a changed object of the small board after the edit may touch a pad with no net, or a
/// changed pad with no net may touch anything: the one way an edit can join two nets without a
/// contact between them.
fn joins_through_a_loose_pad(board: &Board, what: &Changed, tolerance: f64) -> bool {
    let (netted, loose) = cores(board, what);
    loose.iter().enumerate().any(|(i, (pad, new))| {
        netted
            .iter()
            .any(|(other, other_new)| (*new || *other_new) && may_touch(pad, other, tolerance))
            || loose.iter().enumerate().any(|(j, (other, other_new))| {
                i != j && (*new || *other_new) && may_touch(pad, other, tolerance)
            })
    })
}

/// The members of `a` that `b` does not have, counting repeats. Both are in the order of
/// [`order`].
fn not_in(a: &[Finding], b: &[Finding]) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut j = 0;
    for finding in a {
        while j < b.len() && order(&b[j], finding).is_lt() {
            j += 1;
        }
        if j < b.len() && b[j] == *finding {
            j += 1;
        } else {
            out.push(finding.clone());
        }
    }
    out
}

/// The findings of `after`, worked out from the findings `known` of `before` by checking only the
/// zone that differs. `known` must be what [`findings`] gives for `before`. `None` when this
/// cannot be done and the whole board has to be checked; when it can, the list is the one that
/// [`findings`] gives for `after`.
pub fn findings_near(
    before: &Board,
    known: &[Finding],
    after: &Board,
    ctx: &RuleContext,
) -> Option<Vec<Finding>> {
    if known.iter().any(|f| f.code == codes::COPPER_SHORT) {
        return None;
    }
    let what = changed(before, after)?;
    let count = what.components.len() + what.tracks.len() + what.vias.len() + what.silk.len();
    let total = after.components.len() + after.tracks.len() + after.vias.len();
    if count > MANY_CHANGED && count.saturating_mul(4) > total {
        return None;
    }
    let rules = &ctx.rules;
    let reach = rules
        .min_clearance
        .get()
        .max(rules.min_drill_spacing.get())
        .max(rules.min_hole_clearance.get())
        .max(
            ctx.classes
                .classes()
                .iter()
                .map(|c| c.min_clearance.get())
                .max()
                .unwrap_or(0),
        )
        .max(rules.min_mask_bridge.get())
        .max(rules.connect_tolerance.get())
        .max(0)
        .saturating_add(1_000);
    let mut zone = Zone::new();
    mark(&mut zone, before, &what, reach)?;
    mark(&mut zone, after, &what, reach)?;
    // an object that did not change is the same on both boards
    let near = neighbours(after, &what, &zone);
    let was = small(before, &what, &near);
    let is = small(after, &what, &near);
    if joins_through_a_loose_pad(&is, &what, rules.connect_tolerance.get() as f64) {
        return None;
    }
    let was = findings(&was, ctx);
    let is = findings(&is, ctx);
    let gone = not_in(&was, &is);
    let came = not_in(&is, &was);
    let mut out = not_in(known, &gone);
    if out.len() + gone.len() != known.len() {
        // what should go away is not among the known findings: they are of another board
        return None;
    }
    out.extend(came);
    out.sort_by(order);
    Some(out)
}

/// The findings of `after`: by the zone that differs from `before` when that can be done, else by
/// checking the whole board. The list is the same both ways.
pub fn findings_after(
    before: &Board,
    known: &[Finding],
    after: &Board,
    ctx: &RuleContext,
) -> Vec<Finding> {
    findings_near(before, known, after, ctx).unwrap_or_else(|| findings(after, ctx))
}
