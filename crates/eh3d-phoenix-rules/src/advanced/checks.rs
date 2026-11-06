// SPDX-License-Identifier: GPL-3.0-or-later
//! The items the advanced checks measure and the checks on rings, edges, drill classes, acid
//! traps, duplicate vias, pads on no net and parts outside the outline. The checks on silkscreen,
//! necks, stubs, isolation and islands are in `checks_more`.

use std::collections::BTreeMap;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, Segment, Via};
use eh3d_phoenix_model::Point;

use super::checks_more;
use super::grid::{pairs, Bx, Index};
use super::{
    make, not_judged, AdvancedConfig, IndexMode, RuleInfo, R_ACID_TRAP, R_COPPER_EDGE,
    R_DRILL_CLASS, R_OUTSIDE, R_PAD_EDGE, R_PAD_NO_NET, R_RING, R_VIA_DUP,
};
use crate::geometry::{inside_polygon, outline_edges, pad_min_side, Dist2, Prim};
use crate::items::{pad_prim, placed_pads, track_prim, via_prim, PlacedPad};
use crate::report::Report;

/// What kind of copper an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Track,
    Via,
    Pad,
}

/// A track, a via or a pad, as copper that can be measured.
pub(super) struct Cu {
    pub(super) kind: Kind,
    /// The identity of a track or a via.
    pub(super) id: Option<Id>,
    pub(super) label: String,
    pub(super) net: Option<Id>,
    pub(super) layers: Vec<Id>,
    pub(super) prim: Prim,
    /// The box of the item, made as large as the copper reaches.
    pub(super) bx: Bx,
}

/// Everything the checks read.
pub(super) struct Ctx<'a> {
    pub(super) board: &'a Board,
    pub(super) cfg: &'a AdvancedConfig,
    pub(super) mode: IndexMode,
    pub(super) items: Vec<Cu>,
    pub(super) pads: Vec<PlacedPad>,
    pub(super) index: Index,
}

/// The box of a piece of copper, as large as the copper reaches.
pub(super) fn box_of(prim: &Prim) -> Bx {
    let b = prim.bbox();
    let e = prim.hi / 2 + 1;
    [b[0] - e, b[1] - e, b[2] + e, b[3] + e]
}

/// The copper layers a via joins: all the layers of the stack between its two ends, or its two
/// ends when the stack is not given.
pub(super) fn via_layers(board: &Board, via: &Via) -> Vec<Id> {
    let stack = &board.copper_stack;
    let a = stack.iter().position(|l| *l == via.from_layer);
    let b = stack.iter().position(|l| *l == via.to_layer);
    match (a, b) {
        (Some(a), Some(b)) => {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            stack[lo..=hi].to_vec()
        }
        _ => vec![via.from_layer, via.to_layer],
    }
}

/// True when the via is drilled through the whole board.
pub(super) fn via_is_through(board: &Board, via: &Via) -> bool {
    match (board.copper_stack.first(), board.copper_stack.last()) {
        (Some(first), Some(last)) => {
            (via.from_layer == *first && via.to_layer == *last)
                || (via.from_layer == *last && via.to_layer == *first)
        }
        _ => true,
    }
}

/// The name of a net, when there is one.
pub(super) fn net_name(board: &Board, net: Option<Id>) -> Option<&str> {
    net.and_then(|n| board.nets.get(&n))
        .map(|n| n.name.as_str())
}

/// Do two items have a copper layer in common?
pub(super) fn shares_layer(a: &Cu, b: &Cu) -> bool {
    a.layers.iter().any(|l| b.layers.contains(l))
}

/// Two labels in order, joined: `a and b`.
pub(super) fn both(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a} and {b}")
    } else {
        format!("{b} and {a}")
    }
}

fn make_cu(
    kind: Kind,
    id: Option<Id>,
    label: String,
    net: Option<Id>,
    layers: Vec<Id>,
    prim: Prim,
) -> Cu {
    let bx = box_of(&prim);
    Cu {
        kind,
        id,
        label,
        net,
        layers,
        prim,
        bx,
    }
}

/// The items of a board, in a fixed order (tracks, vias, pads), and the way to find neighbours.
pub(super) fn build<'a>(board: &'a Board, cfg: &'a AdvancedConfig, mode: IndexMode) -> Ctx<'a> {
    let pads = placed_pads(board);
    let mut items = Vec::new();
    for (id, track) in &board.tracks {
        if let Some(prim) = track_prim(track) {
            items.push(make_cu(
                Kind::Track,
                Some(*id),
                format!("track {id}"),
                Some(track.net),
                vec![track.layer],
                prim,
            ));
        }
    }
    for (id, via) in &board.vias {
        if let Some(prim) = via_prim(via) {
            items.push(make_cu(
                Kind::Via,
                Some(*id),
                format!("via {id}"),
                Some(via.net),
                via_layers(board, via),
                prim,
            ));
        }
    }
    for pad in &pads {
        if let Some(prim) = pad_prim(pad) {
            items.push(make_cu(
                Kind::Pad,
                None,
                pad.label.clone(),
                pad.net,
                pad.layers.clone(),
                prim,
            ));
        }
    }
    let boxes: Vec<Bx> = items.iter().map(|c| c.bx).collect();
    let index = make_index(mode, &boxes, cfg);
    Ctx {
        board,
        cfg,
        mode,
        items,
        pads,
        index,
    }
}

/// The index over some boxes, in the chosen way.
pub(super) fn make_index(mode: IndexMode, boxes: &[Bx], cfg: &AdvancedConfig) -> Index {
    match mode {
        IndexMode::Grid => Index::grid(boxes, i128::from(cfg.grid_cell_nm)),
        IndexMode::Naive => Index::naive(boxes.len()),
    }
}

impl Ctx<'_> {
    /// The pairs of items whose boxes are within `margin` of each other.
    pub(super) fn pairs(&self, margin: i128) -> Vec<(usize, usize)> {
        let boxes: Vec<Bx> = self.items.iter().map(|c| c.bx).collect();
        pairs(&self.index, &boxes, margin)
    }
}

/// What can be said about two pieces of copper and a limit.
pub(super) enum Verdict {
    /// Closer than the limit, proven; the gap in nanometres.
    Violates(i64),
    /// Closer than the limit only if the pad is read as large as it can be.
    Maybe,
    /// Not closer than the limit.
    Clear,
    /// The numbers do not fit in the exact test.
    Unknown,
}

/// Is the gap between two items smaller than `limit`? Exact: the squared distance is compared
/// with the limit without a root.
pub(super) fn judge(a: &Prim, b: &Prim, limit: i128) -> Verdict {
    let Some(d) = a.distance(b) else {
        return Verdict::Unknown;
    };
    match d.doubled_below(2 * limit + a.lo + b.lo) {
        None => Verdict::Unknown,
        Some(true) => {
            let gap = d.nm_f64() - (a.lo + b.lo) as f64 / 2.0;
            Verdict::Violates(gap.round().max(0.0) as i64)
        }
        Some(false) => match d.doubled_below(2 * limit + a.hi + b.hi) {
            None => Verdict::Unknown,
            Some(true) => Verdict::Maybe,
            Some(false) => Verdict::Clear,
        },
    }
}

/// Runs every check in its fixed order.
pub(super) fn run_all(ctx: &Ctx<'_>, out: &mut Report) {
    rings(ctx, out);
    edge_rule(ctx, out, &R_COPPER_EDGE, false);
    edge_rule(ctx, out, &R_PAD_EDGE, true);
    drill_classes(ctx, out);
    checks_more::silk_holes(ctx, out);
    checks_more::necks(ctx, out);
    acid_traps(ctx, out);
    checks_more::isolation(ctx, out);
    checks_more::islands(ctx, out);
    checks_more::stubs(ctx, out);
    via_duplicates(ctx, out);
    pads_without_net(ctx, out);
    outside_outline(ctx, out);
}

pub(super) fn to_i64(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(0)
}

fn rings(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.annular_ring_level else {
        return;
    };
    let min = i128::from(ctx.cfg.annular_ring_min_nm);
    for (id, via) in &ctx.board.vias {
        let ring2 = i128::from(via.diameter.get()) - i128::from(via.drill.get());
        if ring2 < 2 * min {
            out.confirmed.push(
                make(&R_RING, severity, format!("via {id}"))
                    .at(via.at.x_nm.get(), via.at.y_nm.get())
                    .values(
                        to_i64(ring2.div_euclid(2).max(0)),
                        ctx.cfg.annular_ring_min_nm,
                    ),
            );
        }
    }
    for pad in &ctx.pads {
        let Some(drill) = pad.drill else {
            continue;
        };
        let ring2 = pad_min_side(pad.shape) - i128::from(drill.get());
        if ring2 < 2 * min {
            out.confirmed.push(
                make(&R_RING, severity, pad.label.clone())
                    .at(pad.at.x_nm.get(), pad.at.y_nm.get())
                    .values(
                        to_i64(ring2.div_euclid(2).max(0)),
                        ctx.cfg.annular_ring_min_nm,
                    ),
            );
        }
    }
}

/// Copper (`pads` false: tracks and vias) or pads (`pads` true) against the edge of the board.
fn edge_rule(ctx: &Ctx<'_>, out: &mut Report, rule: &RuleInfo, pads: bool) {
    let (level, limit_nm) = if pads {
        (ctx.cfg.pad_edge_level, ctx.cfg.pad_edge_min_nm)
    } else {
        (ctx.cfg.copper_edge_level, ctx.cfg.copper_edge_min_nm)
    };
    let Some(severity) = level else {
        return;
    };
    if !ctx.items.iter().any(|c| (c.kind == Kind::Pad) == pads) {
        return;
    }
    let edges = outline_edges(ctx.board);
    if edges.is_empty() {
        out.inconclusive.push(
            not_judged(
                "board outline".to_owned(),
                "the board has no closed outline, so the distance to its edge is not judged",
            )
            .limit(limit_nm),
        );
        return;
    }
    let limit = i128::from(limit_nm);
    for item in ctx.items.iter().filter(|c| (c.kind == Kind::Pad) == pads) {
        let at = item.prim.first_point();
        let mut proven: Option<i64> = None;
        let mut unsure = false;
        for edge in &edges {
            match judge(&item.prim, edge, limit) {
                Verdict::Violates(gap) => {
                    proven = Some(proven.map_or(gap, |g| g.min(gap)));
                }
                Verdict::Maybe | Verdict::Unknown => unsure = true,
                Verdict::Clear => {}
            }
        }
        if let Some(gap) = proven {
            out.confirmed.push(
                make(rule, severity, item.label.clone())
                    .at(at.x_nm.get(), at.y_nm.get())
                    .values(gap, limit_nm),
            );
        } else if unsure {
            out.inconclusive.push(
                not_judged(
                    item.label.clone(),
                    "the shape is too coarse, or the numbers too large, to judge the distance to the edge",
                )
                .at(at.x_nm.get(), at.y_nm.get())
                .limit(limit_nm),
            );
        }
    }
}

fn drill_classes(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.drill_class_level else {
        return;
    };
    for (id, via) in &ctx.board.vias {
        let name = net_name(ctx.board, Some(via.net));
        let (min, class) = ctx.cfg.drill_min_for(name);
        if min > 0 && via.drill.get() < min {
            out.confirmed.push(
                make(&R_DRILL_CLASS, severity, format!("via {id}"))
                    .at(via.at.x_nm.get(), via.at.y_nm.get())
                    .values(via.drill.get(), min)
                    .why(&class_text(class)),
            );
        }
    }
    for pad in &ctx.pads {
        let Some(drill) = pad.drill else {
            continue;
        };
        let (min, class) = ctx.cfg.drill_min_for(net_name(ctx.board, pad.net));
        if min > 0 && drill.get() < min {
            out.confirmed.push(
                make(&R_DRILL_CLASS, severity, pad.label.clone())
                    .at(pad.at.x_nm.get(), pad.at.y_nm.get())
                    .values(drill.get(), min)
                    .why(&class_text(class)),
            );
        }
    }
}

fn class_text(class: Option<&str>) -> String {
    match class {
        Some(name) => format!("minimum of the class {name}"),
        None => "default minimum".to_owned(),
    }
}

/// The key of a place where pieces of tracks meet: the net, the layer and the point.
type JointKey = (Id, Id, i64, i64);
/// One piece of a track that leaves a joint: the track and the direction.
type Arm = (Id, i128, i128);

fn acid_traps(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.acid_trap_level else {
        return;
    };
    let min = ctx.cfg.acid_trap_min_angle_deg as f64;
    if min <= 0.0 {
        return;
    }
    let mut joints: BTreeMap<JointKey, Vec<Arm>> = BTreeMap::new();
    for (id, track) in &ctx.board.tracks {
        for segment in &track.path {
            let Segment::Line { start, end } = segment else {
                continue;
            };
            let (sx, sy) = (start.x_nm.get(), start.y_nm.get());
            let (ex, ey) = (end.x_nm.get(), end.y_nm.get());
            let dx = i128::from(ex) - i128::from(sx);
            let dy = i128::from(ey) - i128::from(sy);
            if dx == 0 && dy == 0 {
                continue;
            }
            joints
                .entry((track.net, track.layer, sx, sy))
                .or_default()
                .push((*id, dx, dy));
            joints
                .entry((track.net, track.layer, ex, ey))
                .or_default()
                .push((*id, -dx, -dy));
        }
    }
    for ((_, _, x, y), arms) in &joints {
        for (i, a) in arms.iter().enumerate() {
            for b in &arms[i + 1..] {
                let dot = a.1 * b.1 + a.2 * b.2;
                let na = ((a.1 * a.1 + a.2 * a.2) as f64).sqrt();
                let nb = ((b.1 * b.1 + b.2 * b.2) as f64).sqrt();
                let cos = (dot as f64 / (na * nb)).clamp(-1.0, 1.0);
                let angle = cos.acos().to_degrees();
                if angle < min - 1e-9 {
                    let location = if a.0 == b.0 {
                        format!("track {}", a.0)
                    } else {
                        both(&format!("track {}", a.0), &format!("track {}", b.0))
                    };
                    out.confirmed
                        .push(
                            make(&R_ACID_TRAP, severity, location)
                                .at(*x, *y)
                                .why(&format!(
                                    "angle {angle:.3} degrees, minimum {} degrees",
                                    ctx.cfg.acid_trap_min_angle_deg
                                )),
                        );
                }
            }
        }
    }
}

fn via_duplicates(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.via_duplicate_level else {
        return;
    };
    let max = i128::from(ctx.cfg.via_duplicate_max_distance_nm);
    for (i, j) in ctx.pairs(max) {
        let (a, b) = (&ctx.items[i], &ctx.items[j]);
        if a.kind != Kind::Via || b.kind != Kind::Via || !shares_layer(a, b) {
            continue;
        }
        let (pa, pb) = (a.prim.first_point(), b.prim.first_point());
        let d = Dist2::of_points(pa, pb);
        if d.doubled_at_most(2 * max) != Some(true) {
            continue;
        }
        let same = if a.net == b.net {
            "same net"
        } else {
            "different nets"
        };
        out.confirmed.push(
            make(&R_VIA_DUP, severity, both(&a.label, &b.label))
                .at(pa.x_nm.get(), pa.y_nm.get())
                .values(
                    d.nm_f64().round() as i64,
                    ctx.cfg.via_duplicate_max_distance_nm,
                )
                .why(same),
        );
    }
}

fn pads_without_net(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.pad_no_net_level else {
        return;
    };
    for pad in ctx.pads.iter().filter(|p| p.net.is_none()) {
        out.confirmed.push(
            make(&R_PAD_NO_NET, severity, pad.label.clone())
                .at(pad.at.x_nm.get(), pad.at.y_nm.get()),
        );
    }
}

fn outside_outline(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.outside_outline_level else {
        return;
    };
    let board = ctx.board;
    if board.components.is_empty() {
        return;
    }
    if board.outline.len() < 3 {
        out.inconclusive.push(not_judged(
            "board outline".to_owned(),
            "the board has no closed outline, so the parts are not judged against it",
        ));
        return;
    }
    for (cid, component) in &board.components {
        let mut centres: Vec<Point> = Vec::new();
        if let Some(footprint) = board.footprints.get(&component.footprint) {
            for pid in footprint.pads.keys() {
                let pad = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                if let Some(p) = pad_position(board, pad) {
                    centres.push(p);
                }
            }
        }
        if centres.is_empty() {
            centres.push(component.at);
        }
        let outside: Vec<&Point> = centres
            .iter()
            .filter(|p| !inside_polygon(**p, &board.outline))
            .collect();
        if let Some(first) = outside.first() {
            out.confirmed.push(
                make(&R_OUTSIDE, severity, component.refdes.clone())
                    .at(first.x_nm.get(), first.y_nm.get())
                    .why(&format!(
                        "{} of {} pads outside",
                        outside.len(),
                        centres.len()
                    )),
            );
        }
    }
}
