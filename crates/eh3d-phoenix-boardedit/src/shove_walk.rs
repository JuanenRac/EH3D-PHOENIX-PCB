// SPDX-License-Identifier: GPL-3.0-or-later
//! The second way of the push (prototype), used when moving pieces in parallel is not enough: the
//! new track and the tracks it crowds are walked around obstacles, and vias are moved out of the
//! way with the tracks that end on them.
//!
//! The new track goes around the pads, the vias that cannot move and the arcs of other nets. Every
//! track it crowds is walked around the octagons of what has already found its place (the new track
//! and what was pushed before it) and of the fixed copper, and then may crowd the next one. A via
//! that is crowded goes to the nearest place that is clear on every layer it joins, and the tracks
//! that end on it follow. Nothing is moved twice by the same neighbour, so the work ends; the
//! depth and the number of items that move have the limits of the settings.
//!
//! The planning here is careful but is not the judge: the caller tries the result on a copy of the
//! board, with the design rules and the connectivity of every net.
//!
//! The pads, vias and tracks near a place are looked up in grids (`eh3d-phoenix-spatial`), so
//! that the time of a push grows little with the size of the board. The grids only leave out what
//! is certainly too far, and what is kept keeps its order, so the push is the one that comparing
//! every object gives; a test compares the two.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment, Side, Track};
use eh3d_phoenix_spatial::{inflate, Bx, Index, SpatialConfig};

use crate::codes::BoardEditError;
use crate::copper::{point_segment, segment_items, segment_segment, segments_cross};
use crate::hull::{around_piece, length, tidy, walk, Oct, Turn, P};
use crate::plan::{refused, Env};
use crate::shape::{legs, pt, Shape};
use crate::shove_codes::{SHOVE_BLOCKED, SHOVE_CROSSING, SHOVE_TOO_DEEP, SHOVE_TOO_MANY};

/// How far around a line obstacles are first looked for, in nanometres.
const REACH_NM: i64 = 3_000_000;

fn key(p: eh3d_phoenix_model::Point) -> P {
    (p.x_nm.get(), p.y_nm.get())
}

fn float(p: P) -> (f64, f64) {
    (p.0 as f64, p.1 as f64)
}

/// The points of a track of straight pieces; `None` when it has an arc or a gap.
fn points_of(path: &[Segment]) -> Option<Vec<P>> {
    let mut out: Vec<P> = Vec::with_capacity(path.len() + 1);
    for segment in path {
        let Segment::Line { start, end } = *segment else {
            return None;
        };
        match out.last() {
            None => out.push(key(start)),
            Some(last) if *last != key(start) => return None,
            Some(_) => {}
        }
        out.push(key(end));
    }
    (out.len() >= 2).then_some(out)
}

fn distance(p: P, a: P, b: P) -> f64 {
    point_segment(float(p), float(a), float(b)).0
}

/// A pad as the planner sees it.
struct PadItem {
    net: Option<Id>,
    at: P,
    /// The octagon around its copper.
    shape: Oct,
    /// The radius of the circle inside it, to tell what touches it.
    inner: i64,
    layers: Vec<Id>,
    drill: Option<i64>,
}

struct ViaItem {
    net: Id,
    at: P,
    radius: i64,
    drill: i64,
    layers: Vec<Id>,
    /// Whether the push may move it.
    movable: bool,
}

struct Wire {
    net: Id,
    layer: Id,
    half: i64,
    /// The points, when every piece is straight.
    points: Option<Vec<P>>,
    /// The octagons of a track with arcs, around its centre line, without any clearance.
    fixed: Vec<Oct>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum What {
    Track(Id),
    Via(Id),
}

struct Job {
    depth: usize,
    near: i64,
    order: usize,
    what: What,
}

/// What a push with walks and vias moves.
pub(crate) struct Walked {
    /// The new track, walked around what cannot move.
    pub route: Vec<P>,
    /// The tracks that change, with their new points.
    pub tracks: BTreeMap<Id, Vec<P>>,
    /// The vias that move, with their new places.
    pub vias: BTreeMap<Id, P>,
}

/// Where the pads, the vias and the tracks are, to look at the near ones only.
struct Places {
    pads: Index,
    vias: Index,
    via_ids: Vec<Id>,
    wires: Index,
    wire_ids: Vec<Id>,
    /// The largest half width of a track, radius of a via and clearance between two nets (with the
    /// margin of the push): what a search has to be widened by to miss nothing.
    half: i64,
    radius: i64,
    gap: i64,
}

/// An index over boxes, or the one that gives everything when the grid is refused or not wanted.
fn grid_of(boxes: &[Bx], exhaustive: bool) -> Index {
    if exhaustive {
        return Index::naive(boxes.len());
    }
    Index::grid(boxes, &SpatialConfig::default()).unwrap_or_else(|_| Index::naive(boxes.len()))
}

/// The indexes whose box may be within `by` of the box, in ascending order.
fn around_box(index: &Index, b: Bx, by: i64) -> Vec<usize> {
    index.candidates(&inflate(&b, i128::from(by.max(0)) + 2))
}

struct World<'a> {
    env: &'a Env<'a>,
    places: Places,
    class: BTreeMap<Id, i64>,
    pads: Vec<PadItem>,
    vias: BTreeMap<Id, ViaItem>,
    wires: BTreeMap<Id, Wire>,
    edges: Vec<(P, P)>,
    route_net: Id,
    route_layer: Id,
    route_half: i64,
    route: Vec<P>,
    /// Tracks and vias that have found their place.
    placed_tracks: BTreeMap<Id, Vec<P>>,
    placed_vias: BTreeMap<Id, P>,
    /// Tracks whose end followed a via, still to be placed.
    followed: BTreeMap<Id, Vec<P>>,
    jobs: Vec<Job>,
    order: usize,
    moved: BTreeSet<What>,
}

fn blocked(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(SHOVE_BLOCKED, what)
}

fn pads_of(board: &Board) -> Vec<PadItem> {
    let layers: Vec<Id> = board.layers.keys().copied().collect();
    let mut out = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            let Some(at) = pad_position(board, pad_ref).map(key) else {
                continue;
            };
            let turned = (pad.rotation.get() + component.rotation.get()).rem_euclid(180_000_000);
            let (w, h, round) = match pad.shape {
                PadShape::Circle { diameter } => (diameter.get(), diameter.get(), true),
                PadShape::Oblong { width, height } | PadShape::Rect { width, height } => {
                    (width.get(), height.get(), false)
                }
            };
            let shape = if round {
                Oct::around(&[at]).grown(w / 2)
            } else if turned == 0 || turned == 90_000_000 {
                let (hx, hy) = if turned == 0 {
                    (w / 2, h / 2)
                } else {
                    (h / 2, w / 2)
                };
                Oct::around(&[
                    (at.0 - hx, at.1 - hy),
                    (at.0 + hx, at.1 - hy),
                    (at.0 + hx, at.1 + hy),
                    (at.0 - hx, at.1 + hy),
                ])
            } else {
                Oct::around(&[at]).grown((w as f64).hypot(h as f64).ceil() as i64 / 2)
            };
            let on = if pad.drill.is_some() || component.side == Side::Bottom {
                layers.clone()
            } else {
                pad.layers.clone()
            };
            out.push(PadItem {
                net: board.net_of(pad_ref),
                at,
                shape,
                inner: w.min(h) / 2,
                layers: on,
                drill: pad.drill.map(|d| d.get()),
            });
        }
    }
    out
}

impl<'a> World<'a> {
    /// The ids of the vias that may be within `by` of the box, in the order of the board.
    fn vias_near(&self, b: Bx, by: i64) -> impl Iterator<Item = Id> + '_ {
        around_box(&self.places.vias, b, by)
            .into_iter()
            .map(|i| self.places.via_ids[i])
    }

    /// The ids of the tracks that may be within `by` of the box, in the order of the board.
    fn wires_near(&self, b: Bx, by: i64) -> impl Iterator<Item = Id> + '_ {
        around_box(&self.places.wires, b, by)
            .into_iter()
            .map(|i| self.places.wire_ids[i])
    }

    /// The pads that may be within `by` of the box, in their order.
    fn pads_near(&self, b: Bx, by: i64) -> impl Iterator<Item = &PadItem> + '_ {
        around_box(&self.places.pads, b, by)
            .into_iter()
            .map(|i| &self.pads[i])
    }

    fn new(
        board: &Board,
        env: &'a Env<'a>,
        route: &Track,
        points: Vec<P>,
        exhaustive: bool,
    ) -> World<'a> {
        let mut class = BTreeMap::new();
        for (id, net) in &board.nets {
            if let Some(found) = env.ctx.classes.class_of(&net.name) {
                class.insert(*id, found.min_clearance.get());
            }
        }
        let steps = env.settings.arc_steps_per_quarter;
        let wires: BTreeMap<Id, Wire> = board
            .tracks
            .iter()
            .map(|(id, track)| {
                let half = track.width.get() / 2;
                let points = points_of(&track.path);
                let fixed = if points.is_some() {
                    Vec::new()
                } else {
                    track
                        .path
                        .iter()
                        .flat_map(|s| segment_items(s, half as f64, track.net, steps))
                        .flat_map(|item| {
                            around_piece(
                                (item.a.0.round() as i64, item.a.1.round() as i64),
                                (item.b.0.round() as i64, item.b.1.round() as i64),
                                item.half.ceil() as i64,
                            )
                        })
                        .collect()
                };
                (
                    *id,
                    Wire {
                        net: track.net,
                        layer: track.layer,
                        half,
                        points,
                        fixed,
                    },
                )
            })
            .collect();
        let pads = pads_of(board);
        let mut vias: BTreeMap<Id, ViaItem> = board
            .vias
            .iter()
            .map(|(id, via)| {
                (
                    *id,
                    ViaItem {
                        net: via.net,
                        at: key(via.at),
                        radius: via.diameter.get() / 2,
                        drill: via.drill.get(),
                        layers: board.via_layers(via),
                        movable: env.settings.shove_vias,
                    },
                )
            })
            .collect();
        let via_ids: Vec<Id> = vias.keys().copied().collect();
        let wire_ids: Vec<Id> = wires.keys().copied().collect();
        let pad_boxes: Vec<Bx> = pads.iter().map(|p| p.shape.bx()).collect();
        let via_boxes: Vec<Bx> = vias
            .values()
            .map(|v| Oct::around(&[v.at]).grown(v.radius).bx())
            .collect();
        let wire_boxes: Vec<Bx> = wires
            .values()
            .map(|w| {
                let mut all = Oct::around(w.points.as_deref().unwrap_or(&[]));
                for o in &w.fixed {
                    all = all.union(o);
                }
                if w.points.is_some() {
                    all = all.grown(w.half);
                }
                // a track with no piece has nothing that could be near
                if w.points.is_none() && w.fixed.is_empty() {
                    return [0; 4];
                }
                all.bx()
            })
            .collect();
        let widest_class = class.values().copied().max().unwrap_or(0);
        let places = Places {
            pads: grid_of(&pad_boxes, exhaustive),
            vias: grid_of(&via_boxes, exhaustive),
            via_ids,
            wires: grid_of(&wire_boxes, exhaustive),
            wire_ids,
            half: wires
                .values()
                .map(|w| w.half)
                .max()
                .unwrap_or(0)
                .max(route.width.get() / 2),
            radius: vias.values().map(|v| v.radius).max().unwrap_or(0),
            gap: env
                .ctx
                .rules
                .min_clearance
                .get()
                .max(widest_class)
                .saturating_add(env.settings.shove_margin_nm),
        };
        // a via in a pad of its net, or in the middle of a piece of its net, stays
        for via in vias.values_mut().filter(|v| v.movable) {
            let here = Oct::around(&[via.at]).bx();
            let in_pad = around_box(&places.pads, here, 2)
                .into_iter()
                .map(|i| &pads[i])
                .any(|p| p.net == Some(via.net) && p.shape.grown(1).holds_inside(via.at));
            let reach = via.radius as f64 + 1.0;
            let passes = around_box(&places.wires, here, via.radius.saturating_add(2))
                .into_iter()
                .filter_map(|i| wires.get(&places.wire_ids[i]))
                .filter(|w| w.net == via.net && via.layers.contains(&w.layer))
                .any(|w| match &w.points {
                    Some(points) => {
                        let ends = [points[0], points[points.len() - 1]];
                        !ends.iter().any(|e| distance(via.at, *e, *e) <= reach)
                            && points
                                .windows(2)
                                .any(|s| distance(via.at, s[0], s[1]) <= reach)
                    }
                    None => w
                        .fixed
                        .iter()
                        .any(|o| o.grown(via.radius).holds_inside(via.at)),
                });
            via.movable = !in_pad && !passes;
        }
        let mut edges = Vec::new();
        let outline = board.outline_polygon(&eh3d_phoenix_board::ArcSettings::current());
        for ring in std::iter::once(&outline).chain(board.outline_cutouts.iter()) {
            if ring.len() >= 3 {
                for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                    edges.push((key(*a), key(*b)));
                }
            }
        }
        World {
            env,
            places,
            class,
            pads,
            vias,
            wires,
            edges,
            route_net: route.net,
            route_layer: route.layer,
            route_half: route.width.get() / 2,
            route: points,
            placed_tracks: BTreeMap::new(),
            placed_vias: BTreeMap::new(),
            followed: BTreeMap::new(),
            jobs: Vec::new(),
            order: 0,
            moved: BTreeSet::new(),
        }
    }

    /// The clearance of the rules between two nets, with the larger one of their classes.
    fn clearance(&self, a: Id, b: Option<Id>) -> i64 {
        let of = |net: Id| self.class.get(&net).copied().unwrap_or(0);
        self.env
            .ctx
            .rules
            .min_clearance
            .get()
            .max(of(a))
            .max(b.map_or(0, of))
    }

    /// The same with the margin of the push, for copper that is being placed.
    fn gap(&self, a: Id, b: Id) -> i64 {
        self.clearance(a, Some(b)) + self.env.settings.shove_margin_nm
    }

    /// The octagons that copper of `net`, `half` wide on each side, keeps its centre line out of
    /// because of what has found its place on a layer.
    fn pushers(&self, net: Id, half: i64, layer: Id, area: &Oct, out: &mut Vec<Oct>) {
        let mut line = |other: Id, other_half: i64, points: &[P]| {
            if other == net {
                return;
            }
            let radius = half + other_half + self.gap(net, other);
            for w in points.windows(2) {
                if Oct::around(&[w[0], w[1]]).near(area, radius) {
                    out.extend(around_piece(w[0], w[1], radius));
                }
            }
        };
        if layer == self.route_layer {
            line(self.route_net, self.route_half, &self.route);
        }
        for (id, points) in &self.placed_tracks {
            if let Some(wire) = self.wires.get(id).filter(|w| w.layer == layer) {
                line(wire.net, wire.half, points);
            }
        }
        for (id, at) in &self.placed_vias {
            if let Some(via) = self.vias.get(id) {
                if via.net != net && via.layers.contains(&layer) {
                    let shape =
                        Oct::around(&[*at]).grown(via.radius + half + self.gap(net, via.net));
                    if shape.near(area, 0) {
                        out.push(shape);
                    }
                }
            }
        }
    }

    /// The octagons around the fixed copper of other nets on a layer.
    fn fixed(&self, net: Id, half: i64, layer: Id, area: &Oct, out: &mut Vec<Oct>) {
        let looked = area.bx();
        for pad in self.pads_near(looked, half + self.places.gap) {
            if pad.net != Some(net) && pad.layers.contains(&layer) {
                let radius = half + self.clearance(net, pad.net);
                if pad.shape.near(area, radius) {
                    out.push(pad.shape.grown(radius));
                }
            }
        }
        for id in self.vias_near(looked, half + self.places.radius + self.places.gap) {
            let Some(via) = self.vias.get(&id) else {
                continue;
            };
            if !via.movable && via.net != net && via.layers.contains(&layer) {
                let radius = via.radius + half + self.clearance(net, Some(via.net));
                let shape = Oct::around(&[via.at]).grown(radius);
                if shape.near(area, 0) {
                    out.push(shape);
                }
            }
        }
        for id in self.wires_near(looked, half + self.places.gap) {
            let Some(wire) = self.wires.get(&id) else {
                continue;
            };
            if wire.points.is_none() && wire.layer == layer && wire.net != net {
                let radius = half + self.clearance(net, Some(wire.net));
                out.extend(
                    wire.fixed
                        .iter()
                        .filter(|o| o.near(area, radius))
                        .map(|o| o.grown(radius)),
                );
            }
        }
    }

    /// Whether the piece keeps the distance of the rules from the edge of the board.
    fn keeps_edge(&self, a: P, b: P, half: i64) -> bool {
        let need = self.env.ctx.rules.min_edge_clearance.get() + half;
        self.edges.iter().all(|(c, d)| {
            let (found, _, _) = segment_segment(float(a), float(b), float(*c), float(*d));
            found >= need as f64 && !segments_cross(float(a), float(b), float(*c), float(*d))
        })
    }

    /// The points along a track where something of its net joins it between its ends.
    fn pins(&self, id: Id, points: &[P]) -> Vec<(usize, P)> {
        let Some(wire) = self.wires.get(&id) else {
            return Vec::new();
        };
        let mut joined: Vec<(P, i64)> = Vec::new();
        let along = Oct::around(points).bx();
        let reach = wire
            .half
            .max(self.places.radius)
            .saturating_add(self.places.half)
            .saturating_add(2);
        for other_id in self.wires_near(along, reach) {
            let Some(other) = self.wires.get(&other_id) else {
                continue;
            };
            if other_id != id && other.net == wire.net && other.layer == wire.layer {
                if let Some(theirs) = &other.points {
                    joined.push((theirs[0], wire.half + 1));
                    joined.push((theirs[theirs.len() - 1], wire.half + 1));
                }
            }
        }
        for via in self
            .vias_near(along, reach)
            .filter_map(|v| self.vias.get(&v))
        {
            if via.net == wire.net && via.layers.contains(&wire.layer) {
                joined.push((via.at, via.radius));
            }
        }
        // the radius inside a pad is no more than the half of its box
        for pad in self.pads_near(along, reach) {
            if pad.net == Some(wire.net) && pad.layers.contains(&wire.layer) {
                joined.push((pad.at, pad.inner));
            }
        }
        let ends = [points[0], points[points.len() - 1]];
        let mut out: Vec<(usize, P)> = Vec::new();
        for (at, reach) in joined {
            // what joins an end of the track keeps its place with that end
            if ends.iter().any(|e| distance(at, *e, *e) <= reach as f64) {
                continue;
            }
            for (i, w) in points.windows(2).enumerate() {
                if distance(at, w[0], w[1]) <= reach as f64 {
                    out.push((i, project(at, w[0], w[1])));
                }
            }
        }
        out
    }

    /// The line walked from pin to pin around the obstacles.
    fn walk_pinned(
        points: &[P],
        pins: &[(usize, P)],
        obstacles: &[Oct],
        turn: Turn,
    ) -> Option<Vec<P>> {
        let mut parts: Vec<Vec<P>> = vec![vec![points[0]]];
        for (i, w) in points.windows(2).enumerate() {
            let mut here: Vec<P> = pins.iter().filter(|(k, _)| *k == i).map(|x| x.1).collect();
            let along = |p: &P| (p.0 - w[0].0).abs().max((p.1 - w[0].1).abs());
            here.sort_by_key(along);
            here.dedup();
            for pin in here {
                if let Some(part) = parts.last_mut() {
                    if part.last() != Some(&pin) {
                        part.push(pin);
                    }
                }
                parts.push(vec![pin]);
            }
            if let Some(part) = parts.last_mut() {
                if part.last() != Some(&w[1]) {
                    part.push(w[1]);
                }
            }
        }
        let mut out: Vec<P> = Vec::new();
        for part in parts {
            if part.len() < 2 {
                continue;
            }
            let walked = walk(&part, obstacles, turn, &[])?;
            let skip = usize::from(out.last() == walked.first());
            out.extend(walked.into_iter().skip(skip));
        }
        (out.len() >= 2).then_some(out)
    }

    /// Walks a line of a net on a layer around what has found its place and, when the settings say
    /// so, around the fixed copper. `original` is where the line was: fixed copper that it already
    /// went into does not count (the rules judge it).
    fn go_around(
        &self,
        (net, half, layer): (Id, i64, Id),
        base: &[P],
        original: &[P],
        pins: &[(usize, P)],
        turn: Turn,
    ) -> Result<Vec<P>, BoardEditError> {
        let around = self.env.settings.shove_walk_around;
        let mut reach = REACH_NM;
        let mut area = Oct::around(base).grown(reach);
        for _ in 0..3 {
            let mut fixed = Vec::new();
            self.fixed(net, half, layer, &area, &mut fixed);
            let ends = [base[0], base[base.len() - 1]];
            fixed.retain(|o| {
                !ends.iter().any(|e| o.holds_inside(*e))
                    && original.windows(2).all(|w| o.span(w[0], w[1]).is_none())
            });
            let mut obstacles = Vec::new();
            self.pushers(net, half, layer, &area, &mut obstacles);
            if around {
                obstacles.extend_from_slice(&fixed);
            }
            let walked = Self::walk_pinned(base, pins, &obstacles, turn)
                .ok_or_else(|| blocked("there is no way around what is in the way"))?;
            let box_of_walk = Oct::around(&walked);
            if box_of_walk.union(&area) != area {
                // the walk left the part of the board that was looked at
                reach *= 4;
                area = area.union(&box_of_walk).grown(reach);
                continue;
            }
            if !around
                && walked
                    .windows(2)
                    .any(|w| fixed.iter().any(|o| o.span(w[0], w[1]).is_some()))
            {
                return Err(blocked("fixed copper is in the way"));
            }
            for w in walked.windows(2) {
                let unchanged = original.windows(2).any(|o| o == w);
                if !unchanged && !self.keeps_edge(w[0], w[1], half) {
                    return Err(blocked("the way around would come too close to the edge"));
                }
            }
            return Ok(walked);
        }
        Err(blocked("the way around what is in the way is too long"))
    }

    /// How far an item is from the new track.
    fn near_route(&self, points: &[P]) -> i64 {
        points
            .iter()
            .flat_map(|p| self.route.windows(2).map(|w| distance(*p, w[0], w[1])))
            .fold(f64::INFINITY, f64::min) as i64
    }

    /// Asks for an item to be placed.
    fn ask(&mut self, what: What, depth: usize) -> Result<(), BoardEditError> {
        if self.jobs.iter().any(|j| j.what == what) {
            return Ok(());
        }
        if depth > self.env.settings.shove_max_depth {
            return Err(BoardEditError::new(
                SHOVE_TOO_DEEP,
                format!(
                    "the push goes deeper than {} levels",
                    self.env.settings.shove_max_depth
                ),
            ));
        }
        self.moved.insert(what);
        if self.moved.len() > self.env.settings.shove_max_moved {
            return Err(BoardEditError::new(
                SHOVE_TOO_MANY,
                format!(
                    "the push would move more than {} tracks and vias",
                    self.env.settings.shove_max_moved
                ),
            ));
        }
        let near = match what {
            What::Track(id) => self
                .wires
                .get(&id)
                .and_then(|w| w.points.as_ref())
                .map_or(0, |p| self.near_route(p)),
            What::Via(id) => self.vias.get(&id).map_or(0, |v| self.near_route(&[v.at])),
        };
        self.order += 1;
        self.jobs.push(Job {
            depth,
            near,
            order: self.order,
            what,
        });
        Ok(())
    }

    /// Asks for what pieces that have just found their place now crowd.
    fn crowded_by_line(
        &mut self,
        (net, half, layer): (Id, i64, Id),
        pieces: &[(P, P)],
        depth: usize,
    ) -> Result<(), BoardEditError> {
        let mut found: Vec<What> = Vec::new();
        let Some(looked) = pieces
            .iter()
            .map(|(a, b)| Oct::around(&[*a, *b]))
            .reduce(|all, one| all.union(&one))
            .map(|all| all.bx())
        else {
            return Ok(());
        };
        // the tracks that went with a via are no longer where the index has them
        let mut near: BTreeSet<Id> = self
            .wires_near(looked, half + self.places.half + self.places.gap)
            .collect();
        near.extend(self.followed.keys().copied());
        for (id, wire) in near
            .iter()
            .filter_map(|id| self.wires.get(id).map(|w| (id, w)))
        {
            if wire.net == net || wire.layer != layer || self.placed_tracks.contains_key(id) {
                continue;
            }
            let Some(points) = self.followed.get(id).or(wire.points.as_ref()) else {
                continue;
            };
            let radius = half + wire.half + self.clearance(net, Some(wire.net));
            let theirs = Oct::around(points);
            let crowded = pieces.iter().any(|(a, b)| {
                Oct::around(&[*a, *b]).near(&theirs, radius)
                    && around_piece(*a, *b, radius)
                        .iter()
                        .any(|o| points.windows(2).any(|w| o.span(w[0], w[1]).is_some()))
            });
            if crowded {
                found.push(What::Track(*id));
            }
        }
        for id in self.vias_near(looked, half + self.places.radius + self.places.gap) {
            let Some(via) = self.vias.get(&id) else {
                continue;
            };
            if via.net == net || !via.layers.contains(&layer) || self.placed_vias.contains_key(&id)
            {
                continue;
            }
            let radius = half + via.radius + self.clearance(net, Some(via.net));
            let crowded = pieces.iter().any(|(a, b)| {
                around_piece(*a, *b, radius)
                    .iter()
                    .any(|o| o.holds_inside(via.at))
            });
            if crowded {
                if !via.movable {
                    // a fixed via that the walk did not go around is judged by the rules
                    continue;
                }
                found.push(What::Via(id));
            }
        }
        for what in found {
            self.ask(what, depth)?;
        }
        Ok(())
    }

    fn place_track(&mut self, id: Id, depth: usize, turn: Turn) -> Result<(), BoardEditError> {
        let Some(wire) = self.wires.get(&id) else {
            return Ok(());
        };
        let Some(original) = wire.points.clone() else {
            return Ok(());
        };
        let (net, half, layer) = (wire.net, wire.half, wire.layer);
        if net != self.route_net && layer == self.route_layer {
            let crosses = original.windows(2).any(|a| {
                self.route
                    .windows(2)
                    .any(|b| segments_cross(float(a[0]), float(a[1]), float(b[0]), float(b[1])))
            });
            if crosses {
                return Err(BoardEditError::new(
                    SHOVE_CROSSING,
                    "the route crosses a track of another net",
                ));
            }
        }
        let base = self
            .followed
            .remove(&id)
            .unwrap_or_else(|| original.clone());
        let pins = self.pins(id, &base);
        // a track gives way to the new track on its own side: it does not jump over it. When the
        // way asked for would, each way round is tried for this track
        let over = |walked: &[P]| {
            net != self.route_net
                && layer == self.route_layer
                && jumps(&original, walked, &self.route)
        };
        let mut walked = self.go_around((net, half, layer), &base, &original, &pins, turn)?;
        if over(&walked) {
            walked = [Turn::Left, Turn::Right]
                .into_iter()
                .filter_map(|way| {
                    self.go_around((net, half, layer), &base, &original, &pins, way)
                        .ok()
                })
                .filter(|other| !over(other))
                .min_by(|a, b| length(a).total_cmp(&length(b)))
                .ok_or_else(|| blocked("a track would have to go over the new track"))?;
        }
        let changed: Vec<(P, P)> = walked
            .windows(2)
            .filter(|w| !original.windows(2).any(|o| o == *w))
            .map(|w| (w[0], w[1]))
            .collect();
        if walked == original {
            self.moved.remove(&What::Track(id));
        }
        self.placed_tracks.insert(id, walked);
        self.crowded_by_line((net, half, layer), &changed, depth + 1)
    }

    /// The nearest place of a via that is outside every octagon, going one of eight ways.
    fn free_place(&self, at: P, obstacles: &[Oct], radius: i64) -> Option<P> {
        const WAYS: [(i64, i64); 8] = [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
            (1, -1),
        ];
        let most = self.env.settings.shove_max_via_shift_nm;
        let mut best: Option<(i128, P)> = None;
        for way in WAYS {
            let mut p = at;
            let mut steps = 0;
            while let Some(hull) = obstacles.iter().find(|o| o.holds_inside(p)) {
                p = leave(hull, p, way);
                steps += 1;
                if steps > 32 || (p.0 - at.0).abs().max((p.1 - at.1).abs()) > most {
                    break;
                }
            }
            let (dx, dy) = (i128::from(p.0 - at.0), i128::from(p.1 - at.1));
            let far = dx * dx + dy * dy;
            if far > i128::from(most) * i128::from(most)
                || obstacles.iter().any(|o| o.holds_inside(p))
                || !self.keeps_edge(p, p, radius)
                || self
                    .edges
                    .iter()
                    .any(|(c, d)| segments_cross(float(at), float(p), float(*c), float(*d)))
            {
                continue;
            }
            if best.is_none_or(|(b, _)| far < b) {
                best = Some((far, p));
            }
        }
        best.map(|(_, p)| p)
    }

    fn place_via(&mut self, id: Id, depth: usize) -> Result<(), BoardEditError> {
        let Some(via) = self.vias.get(&id) else {
            return Ok(());
        };
        let (net, old, radius, drill, layers) =
            (via.net, via.at, via.radius, via.drill, via.layers.clone());
        let most = self.env.settings.shove_max_via_shift_nm;
        let spacing = self.env.ctx.rules.min_drill_spacing.get();
        let margin = self.env.settings.shove_margin_nm;
        let area = Oct::around(&[old]).grown(most.saturating_add(radius));
        let mut obstacles: Vec<Oct> = Vec::new();
        for layer in &layers {
            self.pushers(net, radius, *layer, &area, &mut obstacles);
            self.fixed(net, radius, *layer, &area, &mut obstacles);
        }
        // a hole keeps away from other holes, also of its own net
        let hole = |at: P, other: i64, obstacles: &mut Vec<Oct>| {
            if spacing > 0 {
                let apart = (drill + other) / 2 + spacing + margin;
                let shape = Oct::around(&[at]).grown(apart);
                if shape.near(&area, 0) {
                    obstacles.push(shape);
                }
            }
        };
        let looked = area.bx();
        let apart = drill
            .saturating_add(spacing.max(0))
            .saturating_add(margin)
            .saturating_add(self.places.radius);
        // a via that was moved is at most `most` from where the index has it
        let mut holes: BTreeSet<Id> = self
            .vias_near(looked, apart.saturating_add(most).saturating_add(drill))
            .collect();
        holes.extend(self.placed_vias.keys().copied());
        for (other_id, other) in holes
            .iter()
            .filter_map(|v| self.vias.get(v).map(|other| (v, other)))
        {
            if *other_id == id {
                continue;
            }
            if let Some(at) = self.placed_vias.get(other_id) {
                hole(*at, other.drill, &mut obstacles);
            } else if !other.movable {
                hole(other.at, other.drill, &mut obstacles);
            }
        }
        let widest_drill = self.pads.iter().filter_map(|p| p.drill).max().unwrap_or(0);
        let pad_reach = apart
            .saturating_add(widest_drill)
            .saturating_add(self.places.gap);
        for pad in self.pads_near(looked, pad_reach) {
            if let Some(other) = pad.drill {
                hole(pad.at, other, &mut obstacles);
            }
            // a via is not moved into a pad of its own net
            if pad.net == Some(net) && pad.layers.iter().any(|l| layers.contains(l)) {
                let shape = pad.shape.grown(radius);
                if shape.near(&area, 0) && !shape.holds_inside(old) {
                    obstacles.push(shape);
                }
            }
        }
        let new = self
            .free_place(old, &obstacles, radius)
            .ok_or_else(|| blocked("a via in the way has no free place near it"))?;
        if new == old {
            self.moved.remove(&What::Via(id));
            self.placed_vias.insert(id, old);
            return Ok(());
        }
        self.placed_vias.insert(id, new);
        // the tracks that end on the via go with it
        let mut follow: Vec<(Id, Vec<P>)> = Vec::new();
        let reach = radius as f64 + 1.0;
        let mut ending: BTreeSet<Id> = self
            .wires_near(Oct::around(&[old]).bx(), radius.saturating_add(2))
            .collect();
        ending.extend(self.placed_tracks.keys().copied());
        ending.extend(self.followed.keys().copied());
        for (track_id, wire) in ending
            .iter()
            .filter_map(|t| self.wires.get(t).map(|w| (t, w)))
        {
            if wire.net != net || !layers.contains(&wire.layer) {
                continue;
            }
            let Some(points) = self
                .placed_tracks
                .get(track_id)
                .or_else(|| self.followed.get(track_id))
                .or(wire.points.as_ref())
            else {
                continue;
            };
            let mut points = points.clone();
            let mut changed = false;
            for _ in 0..2 {
                points.reverse();
                let n = points.len();
                if distance(old, points[n - 1], points[n - 1]) <= reach {
                    points.truncate(n - 1);
                    join(&mut points, new)?;
                    changed = true;
                }
            }
            if changed {
                follow.push((*track_id, tidy(&points, &[])));
            }
        }
        for (track_id, points) in follow {
            self.placed_tracks.remove(&track_id);
            self.followed.insert(track_id, points);
            self.ask(What::Track(track_id), depth)?;
        }
        // what the via now crowds
        for layer in layers {
            self.crowded_by_line((net, radius, layer), &[(new, new)], depth + 1)?;
        }
        Ok(())
    }

    fn run(&mut self, turn: Turn) -> Result<(), BoardEditError> {
        let (net, half, layer) = (self.route_net, self.route_half, self.route_layer);
        // the new track goes around what cannot move
        if self.env.settings.shove_walk_around {
            let mut fixed = Vec::new();
            let area = Oct::around(&self.route).grown(REACH_NM);
            self.fixed(net, half, layer, &area, &mut fixed);
            let ends = [self.route[0], self.route[self.route.len() - 1]];
            fixed.retain(|o| !ends.iter().any(|e| o.holds_inside(*e)));
            let walked = walk(&self.route, &fixed, turn, &[])
                .ok_or_else(|| blocked("the new track has no way around fixed copper"))?;
            for w in walked.windows(2) {
                if !self.route.windows(2).any(|o| o == w) && !self.keeps_edge(w[0], w[1], half) {
                    return Err(blocked("the way around would come too close to the edge"));
                }
            }
            if length(&walked) > 4.0 * length(&self.route) + 4.0 * REACH_NM as f64 {
                return Err(blocked("the way around fixed copper is too long"));
            }
            self.route = walked;
        }
        let pieces: Vec<(P, P)> = self.route.windows(2).map(|w| (w[0], w[1])).collect();
        self.crowded_by_line((net, half, layer), &pieces, 1)?;
        let budget = 8 * self.env.settings.shove_max_moved;
        for _ in 0..budget {
            let Some(next) = (0..self.jobs.len()).min_by_key(|i| {
                let j = &self.jobs[*i];
                (j.depth, j.near, j.order)
            }) else {
                return Ok(());
            };
            let job = self.jobs.swap_remove(next);
            match job.what {
                What::Track(id) => self.place_track(id, job.depth, turn)?,
                What::Via(id) => self.place_via(id, job.depth)?,
            }
        }
        Err(BoardEditError::new(
            SHOVE_TOO_MANY,
            "the push does not come to rest",
        ))
    }
}

/// True when a part of `other` lies between the line as it was and the line as it is now.
fn jumps(was: &[P], now: &[P], other: &[P]) -> bool {
    let ring: Vec<(f64, f64)> = was
        .iter()
        .chain(now.iter().rev())
        .map(|p| float(*p))
        .collect();
    let inside = |p: (f64, f64)| {
        let mut odd = false;
        for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0) {
                odd = !odd;
            }
        }
        odd
    };
    other.windows(2).any(|w| {
        let (a, b) = (float(w[0]), float(w[1]));
        [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .any(|t| inside((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))))
    })
}

/// The nearest point of a piece to `at`, on the piece also when it is at 45 degrees.
fn project(at: P, a: P, b: P) -> P {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx as f64).powi(2) + (dy as f64).powi(2);
    if len2 <= 0.0 {
        return a;
    }
    let t = (((at.0 - a.0) as f64 * dx as f64 + (at.1 - a.1) as f64 * dy as f64) / len2)
        .clamp(0.0, 1.0);
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        let long = dx.abs().max(dy.abs());
        let k = (t * long as f64).round() as i64;
        return (a.0 + k * dx.signum(), a.1 + k * dy.signum());
    }
    (
        a.0 + (t * dx as f64).round() as i64,
        a.1 + (t * dy as f64).round() as i64,
    )
}

/// The point where going `way` from a point inside an octagon reaches its edge.
fn leave(hull: &Oct, p: P, way: (i64, i64)) -> P {
    // double the steps until outside, then halve the difference
    let at = |k: i64| (p.0 + way.0 * k, p.1 + way.1 * k);
    let mut high = 1_000_i64;
    while hull.holds_inside(at(high)) && high < (1 << 40) {
        high *= 2;
    }
    let mut low = 0_i64;
    while low + 1 < high {
        let mid = low + (high - low) / 2;
        if hull.holds_inside(at(mid)) {
            low = mid;
        } else {
            high = mid;
        }
    }
    at(high)
}

/// Adds the pieces from the last point to `to`, at 0, 45 and 90 degrees.
fn join(points: &mut Vec<P>, to: P) -> Result<(), BoardEditError> {
    let Some(&from) = points.last() else {
        return Err(refused("a track has no piece"));
    };
    let corners = legs(pt(from.0, from.1)?, pt(to.0, to.1)?, Shape::Diagonal, true)?;
    points.extend(corners.into_iter().skip(1).map(key));
    Ok(())
}

/// Finds what has to move for the new track. `turn` says which way round the obstacles are
/// walked.
pub(crate) fn walk_shove(
    board: &Board,
    env: &Env,
    route: &Track,
    turn: Turn,
) -> Result<Walked, BoardEditError> {
    walk_with(board, env, route, turn, false)
}

/// [`walk_shove`] with the grids, or comparing every object when `exhaustive`.
fn walk_with(
    board: &Board,
    env: &Env,
    route: &Track,
    turn: Turn,
    exhaustive: bool,
) -> Result<Walked, BoardEditError> {
    let points = points_of(&route.path).ok_or_else(|| refused("the route is not straight"))?;
    let mut world = World::new(board, env, route, points, exhaustive);
    world.run(turn)?;
    let tracks = world
        .placed_tracks
        .into_iter()
        .filter(|(id, points)| world.wires.get(id).and_then(|w| w.points.as_ref()) != Some(points))
        .collect();
    let vias = world
        .placed_vias
        .into_iter()
        .filter(|(id, at)| world.vias.get(id).is_some_and(|v| v.at != *at))
        .collect();
    Ok(Walked {
        route: world.route,
        tracks,
        vias,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use eh3d_phoenix_board::{Net, Via};
    use eh3d_phoenix_model::Nm;
    use eh3d_phoenix_route::RouteSettings;

    use super::*;
    use crate::settings::BoardEditSettings;
    use crate::testutil::{fixture, id, strict_rules};

    /// What a push says, so that two can be compared.
    fn said(result: Result<Walked, BoardEditError>) -> String {
        match result {
            Ok(w) => format!("{:?} {:?} {:?}", w.route, w.tracks, w.vias),
            Err(e) => format!("{} {}", e.code, e.detail),
        }
    }

    #[test]
    fn the_grids_give_the_push_that_comparing_every_object_gives() {
        let mut state = 41_u64;
        let mut next = move |n: i64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as i64 % n
        };
        let (mut done, mut moved) = (0, 0);
        for _ in 0..60 {
            let f = fixture();
            let mut board = f.board.clone();
            // tracks and vias of nets of their own around the middle of the board
            for k in 0..24_u16 {
                let net = id(200 + k);
                board.nets.insert(
                    net,
                    Net {
                        name: format!("N{k}"),
                    },
                );
                let x = 12_000_000 + next(30) * 500_000;
                let y = 11_000_000 + next(40) * 250_000;
                if k % 4 == 0 {
                    board.vias.insert(
                        id(300 + k),
                        Via {
                            net,
                            at: pt(x, y).unwrap(),
                            drill: Nm::new(300_000).unwrap(),
                            diameter: Nm::new(600_000).unwrap(),
                            from_layer: f.top,
                            to_layer: f.bottom,
                        },
                    );
                } else {
                    let long = 500_000 + next(8) * 500_000;
                    board.tracks.insert(
                        id(400 + k),
                        Track {
                            net,
                            layer: f.top,
                            width: Nm::new(250_000).unwrap(),
                            path: vec![Segment::Line {
                                start: pt(x, y).unwrap(),
                                end: pt(x + long, y).unwrap(),
                            }],
                        },
                    );
                }
            }
            let y = 11_000_000 + next(40) * 250_000 + 100_000;
            let route = Track {
                net: f.gnd,
                layer: f.top,
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(14_000_000, y).unwrap(),
                    end: pt(24_000_000, y).unwrap(),
                }],
            };
            let ctx = strict_rules();
            let settings = BoardEditSettings {
                shove_max_depth: 6,
                shove_max_moved: 40,
                ..BoardEditSettings::default()
            };
            let (routes, locked, pairs) = (
                RouteSettings::default(),
                BTreeSet::new(),
                Default::default(),
            );
            let env = Env {
                ctx: &ctx,
                route: &routes,
                settings: &settings,
                locked: &locked,
                pairs: &pairs,
                known: None,
            };
            for turn in [Turn::Left, Turn::Right] {
                let fast = walk_with(&board, &env, &route, turn, false);
                if let Ok(w) = &fast {
                    done += 1;
                    moved += w.tracks.len() + w.vias.len();
                }
                assert_eq!(
                    said(fast),
                    said(walk_with(&board, &env, &route, turn, true))
                );
            }
        }
        // the boards are such that pushes are made and move things
        assert!(done > 10 && moved >= 8, "{done} done, {moved} moved");
    }
}
