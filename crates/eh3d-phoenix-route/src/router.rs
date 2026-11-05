// SPDX-License-Identifier: GPL-3.0-or-later
//! The interactive router: from an element of the board (a pad, a via or the end of a track) to
//! another of the same net, with tracks at 0, 45 and 90 degrees.
//!
//! * `Mode::Stop` walks the straight way (one piece, or two when the angle is not a multiple of 45
//!   degrees) and stops, to the nanometre, in front of the first obstacle;
//! * `Mode::Around` searches a coarse mesh anchored at the start with a priority queue, where the
//!   cost is the length plus a penalty for every corner and every via, and finishes with one or two
//!   straight pieces to the exact end. It changes layer with a via when the ends are on different
//!   layers or the way is shorter.
//!
//! The width of the track and the size of the vias come from the net class and the rules. Nothing
//! is added to the board by the router: it returns a plan with the operation that adds the tracks
//! and vias. A plan is made only if the rules, run on the board with the plan applied, find no new
//! error, and a complete plan only if it really joins the two elements; otherwise the result is an
//! error with a code and the board is as it was.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use eh3d_phoenix_board::connectivity::{pad_position, Item};
use eh3d_phoenix_board::{Board, Id, LayerKind, Operation, PadRef, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};

use crate::codes::{
    RouteError, ELEMENT_NOT_FOUND, ELEMENT_TEXT_INVALID, ID_UNAVAILABLE, LAYER_UNUSABLE,
    LIMIT_EXCEEDED, NET_MISMATCH, NOT_CONNECTED, NO_PATH, OPERATION_FAILED, OUT_OF_RANGE,
    RULES_REFUSE, SAME_PLACE, STOPPED,
};
use crate::geom::{continues, half_up, is_octilinear, pair, seg_len, P};
use crate::live::{diff, findings, LiveDiff, RuleContext};
use crate::settings::RouteSettings;
use crate::space::{Space, SpaceSpec};

const DIRS: [P; 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const NO_DIR: usize = 8;

/// A part of the board that a track starts or ends at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    /// A pad, by the ids of its component and of its pad.
    Pad(PadRef),
    /// A pad, by the reference designator of its component and its number (`R1.2`).
    PadName {
        /// The reference designator.
        refdes: String,
        /// The number of the pad.
        number: String,
    },
    /// A via.
    Via(Id),
    /// An end of a track: the last point of its path, or the first when `end` is false.
    Track {
        /// The track.
        id: Id,
        /// True for the last point of the path.
        end: bool,
    },
}

fn parse_id(text: &str) -> Result<Id, RouteError> {
    text.trim().parse::<Id>().map_err(|e| {
        RouteError::new(
            ELEMENT_TEXT_INVALID,
            format!("the id {text:?} is not valid: {e}"),
        )
    })
}

impl Element {
    /// Reads the text of the command line: `pad:<component id>/<pad id>`, `pad:R1.2`,
    /// `via:<id>`, `track:<id>` (its last point) or `track:<id>:start`.
    pub fn parse(text: &str) -> Result<Element, RouteError> {
        let invalid = || {
            RouteError::new(
                ELEMENT_TEXT_INVALID,
                format!(
                    "{text:?} is not pad:<ids or refdes.number>, via:<id> or track:<id>[:start]"
                ),
            )
        };
        let (kind, rest) = text.split_once(':').ok_or_else(invalid)?;
        match kind {
            "pad" => {
                if let Some((component, pad)) = rest.split_once('/') {
                    Ok(Element::Pad(PadRef {
                        component: parse_id(component)?,
                        pad: parse_id(pad)?,
                    }))
                } else {
                    let (refdes, number) = rest.rsplit_once('.').ok_or_else(invalid)?;
                    if refdes.is_empty() || number.is_empty() {
                        return Err(invalid());
                    }
                    Ok(Element::PadName {
                        refdes: refdes.to_owned(),
                        number: number.to_owned(),
                    })
                }
            }
            "via" => Ok(Element::Via(parse_id(rest)?)),
            "track" => match rest.split_once(':') {
                None => Ok(Element::Track {
                    id: parse_id(rest)?,
                    end: true,
                }),
                Some((id, "start")) => Ok(Element::Track {
                    id: parse_id(id)?,
                    end: false,
                }),
                Some((id, "end")) => Ok(Element::Track {
                    id: parse_id(id)?,
                    end: true,
                }),
                Some(_) => Err(invalid()),
            },
            _ => Err(invalid()),
        }
    }
}

/// An element found on the board.
struct Anchor {
    at: Point,
    net: Id,
    layers: Vec<usize>,
    item: Item,
}

fn layer_numbers(copper: &[Id], layers: &[Id]) -> Vec<usize> {
    copper
        .iter()
        .enumerate()
        .filter(|(_, id)| layers.contains(id))
        .map(|(i, _)| i)
        .collect()
}

fn not_found(what: String) -> RouteError {
    RouteError::new(ELEMENT_NOT_FOUND, what)
}

fn resolve(element: &Element, board: &Board, copper: &[Id]) -> Result<Anchor, RouteError> {
    match element {
        Element::PadName { refdes, number } => {
            let mut found: Vec<PadRef> = Vec::new();
            for (cid, component) in &board.components {
                if component.refdes != *refdes {
                    continue;
                }
                let Some(footprint) = board.footprints.get(&component.footprint) else {
                    continue;
                };
                for (pid, pad) in &footprint.pads {
                    if pad.number == *number {
                        found.push(PadRef {
                            component: *cid,
                            pad: *pid,
                        });
                    }
                }
            }
            match found.as_slice() {
                [one] => resolve(&Element::Pad(*one), board, copper),
                [] => Err(not_found(format!("no pad {refdes}.{number} on the board"))),
                _ => Err(not_found(format!("{refdes}.{number} names several pads"))),
            }
        }
        Element::Pad(pad_ref) => {
            let component = board
                .components
                .get(&pad_ref.component)
                .ok_or_else(|| not_found(format!("no component {}", pad_ref.component)))?;
            let pad = board
                .footprints
                .get(&component.footprint)
                .and_then(|f| f.pads.get(&pad_ref.pad))
                .ok_or_else(|| not_found(format!("no pad {}", pad_ref.pad)))?;
            let at = pad_position(board, *pad_ref)
                .ok_or_else(|| not_found(format!("pad {} has no place", pad_ref.pad)))?;
            let net = board.net_of(*pad_ref).ok_or_else(|| {
                RouteError::new(
                    NET_MISMATCH,
                    format!("the pad {}.{} has no net", component.refdes, pad.number),
                )
            })?;
            Ok(Anchor {
                at,
                net,
                layers: layer_numbers(copper, &pad.layers),
                item: Item::Pad(*pad_ref),
            })
        }
        Element::Via(id) => {
            let via = board
                .vias
                .get(id)
                .ok_or_else(|| not_found(format!("no via {id}")))?;
            Ok(Anchor {
                at: via.at,
                net: via.net,
                layers: layer_numbers(copper, &[via.from_layer, via.to_layer]),
                item: Item::Via(*id),
            })
        }
        Element::Track { id, end } => {
            let track = board
                .tracks
                .get(id)
                .ok_or_else(|| not_found(format!("no track {id}")))?;
            let wanted = if *end {
                track.path.last()
            } else {
                track.path.first()
            };
            let segment = wanted.ok_or_else(|| not_found(format!("the track {id} has no path")))?;
            Ok(Anchor {
                at: if *end { segment.end() } else { segment.start() },
                net: track.net,
                layers: layer_numbers(copper, &[track.layer]),
                item: Item::Track(*id),
            })
        }
    }
}

/// The copper layers of a board from the top to the bottom: the stack when the board gives one,
/// the copper layers in the order of their ids otherwise.
pub fn copper_layers(board: &Board) -> Vec<Id> {
    if !board.copper_stack.is_empty() {
        return board.copper_stack.clone();
    }
    board
        .layers
        .iter()
        .filter(|(_, layer)| layer.kind == LayerKind::Copper)
        .map(|(id, _)| *id)
        .collect()
}

/// The width of a track of a net: the width its class asks for, or the default of the settings,
/// and never below the narrowest track of the rules.
pub fn track_width(ctx: &RuleContext, settings: &RouteSettings, net_name: &str) -> i64 {
    let class_width = ctx
        .classes
        .class_of(net_name)
        .map_or(0, |c| c.min_track_width.get());
    let base = if class_width > 0 {
        class_width
    } else {
        settings.default_width_nm
    };
    base.max(ctx.rules.min_track_width.get())
}

/// The drill and the pad diameter of a via of a net: the settings, raised to what the rules and the
/// net class ask for, and the diameter raised to the drill plus two annular rings.
pub fn via_size(ctx: &RuleContext, settings: &RouteSettings, net_name: &str) -> (i64, i64) {
    let (class_drill, class_diameter) = ctx.classes.class_of(net_name).map_or((0, 0), |c| {
        (c.min_via_drill.get(), c.min_via_diameter.get())
    });
    let drill = settings
        .via_drill_nm
        .max(ctx.rules.min_drill.get())
        .max(class_drill);
    let mut diameter = settings
        .via_diameter_nm
        .max(ctx.rules.min_via_diameter.get())
        .max(class_diameter);
    let needed = drill.saturating_add(2 * ctx.rules.min_annular_ring.get());
    if diameter < needed {
        diameter = needed;
    }
    (drill, diameter)
}

/// How the router treats an obstacle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Stop in front of the first obstacle.
    Stop,
    /// Go around obstacles, and change layer with a via when that is shorter or needed.
    Around,
}

/// What to route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRequest {
    /// Where the track starts.
    pub from: Element,
    /// Where it should end.
    pub to: Element,
    /// The layer to start on; none lets the router choose among the layers of the start.
    pub layer: Option<Id>,
    /// What to do at an obstacle.
    pub mode: Mode,
}

/// The result of a route: the operation to apply and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePlan {
    /// The operation that adds the tracks and vias (one batch, all or nothing).
    pub operation: Operation,
    /// The tracks, in order from the start.
    pub tracks: Vec<(Id, Track)>,
    /// The vias, in order from the start.
    pub vias: Vec<(Id, Via)>,
    /// True when the plan reaches the second element.
    pub complete: bool,
    /// The length of the tracks in nanometres.
    pub length_nm: i64,
    /// The corners of the tracks.
    pub corners: usize,
    /// Codes that qualify the result (`PHOENIX-ROUTE-008` for a route that stopped).
    pub notes: Vec<&'static str>,
    /// What the rules say changes when the plan is applied.
    pub diff: LiveDiff,
}

fn dir_of(a: P, b: P) -> Option<usize> {
    if !is_octilinear(a, b) {
        return None;
    }
    let d = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    DIRS.iter().position(|x| *x == d)
}

/// Whether a turn from direction `d` to direction `k` is allowed.
fn turn_ok(d: usize, k: usize, acute: bool) -> bool {
    if d == NO_DIR {
        return true;
    }
    match (k + 8 - d) % 8 {
        0 | 1 | 2 | 6 | 7 => true,
        4 => false,
        _ => acute,
    }
}

/// The ways from `q` to `e` with pieces at 0, 45 and 90 degrees and at most one corner.
fn final_candidates(q: P, e: P) -> Vec<Vec<P>> {
    if q == e {
        return vec![vec![q]];
    }
    let dx = e.0 - q.0;
    let dy = e.1 - q.1;
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        return vec![vec![q, e]];
    }
    let m = dx.abs().min(dy.abs());
    let (sx, sy) = (dx.signum(), dy.signum());
    let diagonal_first = (q.0 + sx * m, q.1 + sy * m);
    let straight_first = if dx.abs() > dy.abs() {
        (q.0 + sx * (dx.abs() - m), q.1)
    } else {
        (q.0, q.1 + sy * (dy.abs() - m))
    };
    vec![vec![q, diagonal_first, e], vec![q, straight_first, e]]
}

/// What a search needs.
struct Job<'a> {
    space: &'a Space,
    settings: &'a RouteSettings,
    a: P,
    b: P,
    starts: &'a [usize],
    ends: &'a [usize],
    layers: usize,
}

impl Job<'_> {
    /// The cost of finishing along `cand` on layer `l` after arriving in direction `d`, or none
    /// when a piece is blocked or a turn is not allowed.
    fn candidate_cost(&self, l: usize, cand: &[P], d: usize) -> Option<i64> {
        let mut cost = 0_i64;
        let mut before = d;
        for w in cand.windows(2) {
            let k = dir_of(w[0], w[1])?;
            if !turn_ok(before, k, self.settings.allow_acute) {
                return None;
            }
            if !self.space.seg_clear(l, w[0], w[1]) {
                return None;
            }
            cost += seg_len(w[0], w[1]);
            if before != NO_DIR && before != k {
                cost += self.settings.corner_penalty_nm;
            }
            before = k;
        }
        Some(cost)
    }
}

/// How the states of the search are numbered.
struct Dims {
    w: usize,
    layers: usize,
}

impl Dims {
    fn enc(&self, ix: usize, iy: usize, l: usize, d: usize) -> usize {
        ((iy * self.w + ix) * self.layers + l) * 9 + d
    }

    fn dec(&self, s: usize) -> (usize, usize, usize, usize) {
        let d = s % 9;
        let rest = s / 9;
        let l = rest % self.layers;
        let cell = rest / self.layers;
        (cell % self.w, cell / self.w, l, d)
    }
}

fn limit(what: &str) -> RouteError {
    RouteError::new(LIMIT_EXCEEDED, what.to_owned())
}

/// The search on the mesh. Returns the points of the way with the layer number of each; two
/// consecutive points at the same place on different layers are a via.
fn search(job: &Job) -> Result<Vec<(P, usize)>, RouteError> {
    let s = job.settings;
    let (a, b) = (job.a, job.b);
    let max_c = (1_i64 << 30) - 1;
    let m = s.search_margin_nm;
    let mut lo = (
        (a.0.min(b.0) - m).max(-max_c),
        (a.1.min(b.1) - m).max(-max_c),
    );
    let mut hi = ((a.0.max(b.0) + m).min(max_c), (a.1.max(b.1) + m).min(max_c));
    let outline = job.space.outline();
    if !outline.is_empty() {
        let x0 = outline.iter().map(|p| p.0).min().unwrap_or(lo.0);
        let x1 = outline.iter().map(|p| p.0).max().unwrap_or(hi.0);
        let y0 = outline.iter().map(|p| p.1).min().unwrap_or(lo.1);
        let y1 = outline.iter().map(|p| p.1).max().unwrap_or(hi.1);
        lo.0 = lo.0.max(x0).min(a.0.min(b.0));
        lo.1 = lo.1.max(y0).min(a.1.min(b.1));
        hi.0 = hi.0.min(x1).max(a.0.max(b.0));
        hi.1 = hi.1.min(y1).max(a.1.max(b.1));
    }
    let mut g = s.grid_nm;
    let (g, i0, j0, w, h) = loop {
        let i0 = -((a.0 - lo.0).div_euclid(g));
        let i1 = (hi.0 - a.0).div_euclid(g);
        let j0 = -((a.1 - lo.1).div_euclid(g));
        let j1 = (hi.1 - a.1).div_euclid(g);
        let w = i128::from(i1 - i0 + 1);
        let h = i128::from(j1 - j0 + 1);
        if w * h * job.layers as i128 <= s.max_cells as i128 {
            break (g, i0, j0, w as usize, h as usize);
        }
        g = g.saturating_mul(2);
        if g >= 1 << 30 {
            return Err(limit(
                "the area does not fit the mesh even with the coarsest pitch",
            ));
        }
    };
    let dims = Dims {
        w,
        layers: job.layers,
    };
    let point =
        |ix: usize, iy: usize| -> P { (a.0 + (i0 + ix as i64) * g, a.1 + (j0 + iy as i64) * g) };
    let total = w * h * job.layers * 9;
    let mut dist: Vec<i64> = vec![i64::MAX; total];
    let mut prev: Vec<u32> = vec![u32::MAX; total];
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    let (sx, sy) = ((-i0) as usize, (-j0) as usize);
    for l in job.starts {
        let state = dims.enc(sx, sy, *l, NO_DIR);
        dist[state] = 0;
        heap.push(Reverse((0, state)));
    }
    let diagonal = {
        let n = 2 * i128::from(g) * i128::from(g);
        let r = crate::geom::isqrt(n);
        i64::try_from(if r * r < n { r + 1 } else { r }).unwrap_or(i64::MAX)
    };
    let reach = s.final_reach_cells.saturating_mul(g);
    let mut best_end = i64::MAX;
    let mut end_info: Option<(usize, Vec<P>)> = None;
    let mut expansions = 0_usize;
    while let Some(Reverse((cost, state))) = heap.pop() {
        if cost >= best_end {
            break;
        }
        if cost > dist[state] {
            continue;
        }
        expansions += 1;
        if expansions > s.max_expansions {
            return Err(limit("the search took more steps than max_expansions"));
        }
        let (ix, iy, l, d) = dims.dec(state);
        let q = point(ix, iy);
        if job.ends.contains(&l) && (q.0 - b.0).abs() <= reach && (q.1 - b.1).abs() <= reach {
            for cand in final_candidates(q, b) {
                if let Some(extra) = job.candidate_cost(l, &cand, d) {
                    let total_cost = cost + extra;
                    if total_cost < best_end {
                        best_end = total_cost;
                        end_info = Some((state, cand));
                    }
                }
            }
        }
        for (k, dir) in DIRS.iter().enumerate() {
            if !turn_ok(d, k, s.allow_acute) {
                continue;
            }
            let nx = ix as i64 + dir.0;
            let ny = iy as i64 + dir.1;
            if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                continue;
            }
            let q2 = point(nx as usize, ny as usize);
            if !job.space.seg_clear(l, q, q2) {
                continue;
            }
            let step = if dir.0 != 0 && dir.1 != 0 {
                diagonal
            } else {
                g
            };
            let mut next_cost = cost + step;
            if d != NO_DIR && k != d {
                next_cost += s.corner_penalty_nm;
            }
            let next = dims.enc(nx as usize, ny as usize, l, k);
            if next_cost < dist[next] {
                dist[next] = next_cost;
                prev[next] = state as u32;
                heap.push(Reverse((next_cost, next)));
            }
        }
        for l2 in 0..job.layers {
            if l2 == l || !job.space.via_clear(l, l2, q) {
                continue;
            }
            let next = dims.enc(ix, iy, l2, NO_DIR);
            let next_cost = cost + s.via_penalty_nm;
            if next_cost < dist[next] {
                dist[next] = next_cost;
                prev[next] = state as u32;
                heap.push(Reverse((next_cost, next)));
            }
        }
    }
    let Some((last, cand)) = end_info else {
        return Err(RouteError::new(
            NO_PATH,
            "no way at 0, 45 and 90 degrees keeps the clearances between the two ends",
        ));
    };
    let mut chain = vec![last];
    let mut current = last;
    while prev[current] != u32::MAX {
        current = prev[current] as usize;
        chain.push(current);
    }
    chain.reverse();
    let mut points: Vec<(P, usize)> = chain
        .iter()
        .map(|state| {
            let (ix, iy, l, _) = dims.dec(*state);
            (point(ix, iy), l)
        })
        .collect();
    let last_layer = points.last().map_or(0, |p| p.1);
    for q in cand.iter().skip(1) {
        points.push((*q, last_layer));
    }
    points.dedup();
    Ok(points)
}

/// Walks a polyline on layer `l` and stops, to the nanometre, in front of the first obstacle.
fn walk(space: &Space, l: usize, cand: &[P]) -> (Vec<P>, bool) {
    let mut out = vec![cand[0]];
    for w in cand.windows(2) {
        let (p, q) = (w[0], w[1]);
        if space.seg_clear(l, p, q) {
            out.push(q);
            continue;
        }
        if !space.seg_clear(l, p, p) {
            return (out, false);
        }
        let n = (q.0 - p.0).abs().max((q.1 - p.1).abs());
        let u = ((q.0 - p.0).signum(), (q.1 - p.1).signum());
        let at = |m: i64| -> P { (p.0 + u.0 * m, p.1 + u.1 * m) };
        // the piece up to `lo` is clear and the piece up to `hi` is not
        let (mut lo, mut hi) = (0_i64, n);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if space.seg_clear(l, p, at(mid)) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo > 0 {
            out.push(at(lo));
        }
        return (out, false);
    }
    (out, true)
}

fn polyline_length(points: &[P]) -> i64 {
    points
        .windows(2)
        .map(|w| seg_len(w[0], w[1]))
        .fold(0_i64, |acc, v| acc.saturating_add(v))
}

/// The way of the mode that stops: the best of the straight ways from `a` to `b` on one layer.
fn stop_path(job: &Job, layer: usize) -> Result<(Vec<(P, usize)>, bool), RouteError> {
    let mut best: Option<(Vec<P>, bool, i64)> = None;
    for cand in final_candidates(job.a, job.b) {
        let (points, complete) = walk(job.space, layer, &cand);
        let length = polyline_length(&points);
        let better = match &best {
            None => true,
            Some((_, best_complete, best_length)) => {
                if complete != *best_complete {
                    complete
                } else if complete {
                    length < *best_length
                } else {
                    length > *best_length
                }
            }
        };
        if better {
            best = Some((points, complete, length));
        }
    }
    let Some((points, complete, length)) = best else {
        return Err(RouteError::new(NO_PATH, "there is no straight way"));
    };
    if !complete && length < job.settings.stop_min_length_nm {
        return Err(RouteError::new(
            NO_PATH,
            "the way is blocked at the start, before the shortest track that is made",
        ));
    }
    Ok((points.into_iter().map(|p| (p, layer)).collect(), complete))
}

fn push_vertex(vertices: &mut Vec<P>, p: P) {
    let n = vertices.len();
    if n >= 2 && continues(vertices[n - 2], vertices[n - 1], p) {
        vertices[n - 1] = p;
    } else {
        vertices.push(p);
    }
}

/// The runs of one layer (as the layer number and the corners of the run) and the vias (the place,
/// the layer left and the layer entered).
type Runs = (Vec<(usize, Vec<P>)>, Vec<(P, usize, usize)>);

fn runs_of(points: &[(P, usize)]) -> Runs {
    let mut runs: Vec<(usize, Vec<P>)> = vec![(points[0].1, vec![points[0].0])];
    let mut vias: Vec<(P, usize, usize)> = Vec::new();
    for w in points.windows(2) {
        let (p0, l0) = w[0];
        let (p1, l1) = w[1];
        if l0 == l1 {
            if p0 == p1 {
                continue;
            }
            if let Some(run) = runs.last_mut() {
                push_vertex(&mut run.1, p1);
            }
        } else {
            vias.push((p0, l0, l1));
            runs.push((l1, vec![p0]));
        }
    }
    (runs, vias)
}

fn to_point(p: P) -> Result<Point, RouteError> {
    Point::from_nm(p.0, p.1).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))
}

fn fresh_id(
    board: &Board,
    new_id: &mut dyn FnMut() -> Id,
    taken: &mut BTreeSet<Id>,
    attempts: usize,
) -> Result<Id, RouteError> {
    for _ in 0..attempts {
        let id = new_id();
        if !board.tracks.contains_key(&id) && !board.vias.contains_key(&id) && taken.insert(id) {
            return Ok(id);
        }
    }
    Err(RouteError::new(
        ID_UNAVAILABLE,
        format!("{attempts} ids were tried and all were in use"),
    ))
}

/// Routes from one element to another of the same net and returns the plan. The board is not
/// changed. `new_id` gives the ids of the new parts; ids that are in use are skipped.
pub fn route(
    board: &Board,
    ctx: &RuleContext,
    settings: &RouteSettings,
    request: &RouteRequest,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<RoutePlan, RouteError> {
    let copper = copper_layers(board);
    if copper.is_empty() {
        return Err(RouteError::new(
            LAYER_UNUSABLE,
            "the board has no copper layer",
        ));
    }
    let from = resolve(&request.from, board, &copper)?;
    let to = resolve(&request.to, board, &copper)?;
    if from.net != to.net {
        return Err(RouteError::new(
            NET_MISMATCH,
            "the two elements are on different nets",
        ));
    }
    let (a, b) = (pair(from.at), pair(to.at));
    if a == b {
        return Err(RouteError::new(
            SAME_PLACE,
            "the two elements are at the same point",
        ));
    }
    let mut starts = from.layers.clone();
    if let Some(wanted) = request.layer {
        let number = copper
            .iter()
            .position(|l| *l == wanted)
            .ok_or_else(|| RouteError::new(LAYER_UNUSABLE, "the layer is not a copper layer"))?;
        if !starts.contains(&number) {
            return Err(RouteError::new(
                LAYER_UNUSABLE,
                "the first element has no copper on that layer",
            ));
        }
        starts = vec![number];
    }
    if starts.is_empty() || to.layers.is_empty() {
        return Err(RouteError::new(
            LAYER_UNUSABLE,
            "an element has no copper on a layer of the stack",
        ));
    }
    let net_name = board
        .nets
        .get(&from.net)
        .map(|n| n.name.clone())
        .unwrap_or_default();
    let width = track_width(ctx, settings, &net_name);
    let (via_drill, via_diameter) = via_size(ctx, settings, &net_name);
    let class_clearance = ctx
        .classes
        .class_of(&net_name)
        .map_or(0, |c| c.min_clearance.get());
    let clear = ctx
        .rules
        .min_clearance
        .get()
        .max(class_clearance)
        .max(ctx.rules.connect_tolerance.get().saturating_add(1))
        .saturating_add(settings.clearance_margin_nm);
    let space = Space::build(
        board,
        &SpaceSpec {
            layers: copper.clone(),
            net: from.net,
            half_track: half_up(width),
            via_half: half_up(via_diameter),
            clear,
            edge_clear: ctx
                .rules
                .min_edge_clearance
                .get()
                .saturating_add(settings.clearance_margin_nm),
            bucket: settings.bucket_nm,
        },
    );
    let job = Job {
        space: &space,
        settings,
        a,
        b,
        starts: &starts,
        ends: &to.layers,
        layers: copper.len(),
    };
    let (points, complete) = match request.mode {
        Mode::Around => (search(&job)?, true),
        Mode::Stop => {
            let layer = starts
                .iter()
                .copied()
                .find(|l| to.layers.contains(l))
                .unwrap_or(starts[0]);
            let (points, complete) = stop_path(&job, layer)?;
            if complete && !to.layers.contains(&layer) {
                return Err(RouteError::new(
                    NO_PATH,
                    "the second element is on another layer: use the mode that goes around",
                ));
            }
            (points, complete)
        }
    };
    let (runs, via_places) = runs_of(&points);
    let mut taken: BTreeSet<Id> = BTreeSet::new();
    let mut ops: Vec<Operation> = Vec::new();
    let mut vias: Vec<(Id, Via)> = Vec::new();
    for (place, l0, l1) in &via_places {
        let id = fresh_id(board, new_id, &mut taken, settings.id_attempts)?;
        let via = Via {
            net: from.net,
            at: to_point(*place)?,
            drill: Nm::new(via_drill).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
            diameter: Nm::new(via_diameter)
                .map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
            from_layer: copper[*l0],
            to_layer: copper[*l1],
        };
        ops.push(Operation::AddVia {
            id,
            via: via.clone(),
        });
        vias.push((id, via));
    }
    let mut tracks: Vec<(Id, Track)> = Vec::new();
    let mut length_nm = 0_i64;
    let mut corners = 0_usize;
    for (layer, vertices) in &runs {
        if vertices.len() < 2 {
            continue;
        }
        let mut path: Vec<Segment> = Vec::new();
        for w in vertices.windows(2) {
            path.push(Segment::Line {
                start: to_point(w[0])?,
                end: to_point(w[1])?,
            });
        }
        length_nm = length_nm.saturating_add(polyline_length(vertices));
        corners += vertices.len() - 2;
        let id = fresh_id(board, new_id, &mut taken, settings.id_attempts)?;
        let track = Track {
            net: from.net,
            layer: copper[*layer],
            width: Nm::new(width).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
            path,
        };
        ops.push(Operation::AddTrack {
            id,
            track: track.clone(),
        });
        tracks.push((id, track));
    }
    let operation = Operation::Batch { ops };
    // the plan is judged on a copy of the board, by the same rules the board is checked with
    let before = findings(board, ctx);
    let mut trial = board.clone();
    trial
        .apply(&operation)
        .map_err(|e| RouteError::new(OPERATION_FAILED, e.to_string()))?;
    let after = findings(&trial, ctx);
    let changes = diff(&before, &after);
    if let Some(first) = changes
        .added
        .iter()
        .find(|f| f.severity == eh3d_phoenix_rules::Severity::Error)
    {
        return Err(RouteError::new(
            RULES_REFUSE,
            format!("{} at {}", first.code, first.location),
        ));
    }
    if complete {
        let report = trial.connectivity();
        let joined = report
            .nets
            .iter()
            .find(|n| n.net == from.net)
            .is_some_and(|n| {
                let island_of = |item: Item| n.islands.iter().position(|i| i.items.contains(&item));
                match (island_of(from.item), island_of(to.item)) {
                    (Some(x), Some(y)) => x == y,
                    _ => false,
                }
            });
        if !joined {
            return Err(RouteError::new(
                NOT_CONNECTED,
                "the copper of the plan does not touch both elements",
            ));
        }
    }
    let notes = if complete { Vec::new() } else { vec![STOPPED] };
    Ok(RoutePlan {
        operation,
        tracks,
        vias,
        complete,
        length_nm,
        corners,
        notes,
        diff: changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_are_read_from_their_text() {
        let id = "01234567-89ab-7cde-bfed-cba987654321";
        assert_eq!(
            Element::parse("pad:R1.2").unwrap(),
            Element::PadName {
                refdes: "R1".into(),
                number: "2".into()
            }
        );
        assert_eq!(
            Element::parse(&format!("via:{id}")).unwrap(),
            Element::Via(id.parse().unwrap())
        );
        assert_eq!(
            Element::parse(&format!("track:{id}:start")).unwrap(),
            Element::Track {
                id: id.parse().unwrap(),
                end: false
            }
        );
        assert_eq!(
            Element::parse(&format!("track:{id}")).unwrap(),
            Element::Track {
                id: id.parse().unwrap(),
                end: true
            }
        );
        assert!(matches!(
            Element::parse(&format!("pad:{id}/{id}")).unwrap(),
            Element::Pad(_)
        ));
        for bad in [
            "",
            "pad",
            "pad:R1",
            "pad:.2",
            "via:xyz",
            "track:x:middle",
            "zone:1",
        ] {
            let e = Element::parse(bad).unwrap_err();
            assert_eq!(e.code, ELEMENT_TEXT_INVALID, "{bad}");
        }
    }

    #[test]
    fn the_candidates_for_the_last_leg_have_one_corner_at_most() {
        assert_eq!(final_candidates((0, 0), (0, 0)), vec![vec![(0, 0)]]);
        assert_eq!(final_candidates((0, 0), (5, 5)), vec![vec![(0, 0), (5, 5)]]);
        assert_eq!(
            final_candidates((0, 0), (0, -7)),
            vec![vec![(0, 0), (0, -7)]]
        );
        assert_eq!(
            final_candidates((0, 0), (10, 4)),
            vec![vec![(0, 0), (4, 4), (10, 4)], vec![(0, 0), (6, 0), (10, 4)]]
        );
        assert_eq!(
            final_candidates((0, 0), (-3, 8)),
            vec![
                vec![(0, 0), (-3, 3), (-3, 8)],
                vec![(0, 0), (0, 5), (-3, 8)]
            ]
        );
    }

    #[test]
    fn turns_follow_the_angle_rule() {
        assert!(turn_ok(NO_DIR, 3, false));
        assert!(turn_ok(0, 0, false));
        assert!(turn_ok(0, 1, false));
        assert!(turn_ok(0, 7, false));
        assert!(turn_ok(0, 2, false));
        assert!(!turn_ok(0, 3, false));
        assert!(turn_ok(0, 3, true));
        assert!(!turn_ok(0, 4, true));
        assert_eq!(dir_of((0, 0), (3, 3)), Some(1));
        assert_eq!(dir_of((0, 0), (0, -2)), Some(6));
        assert_eq!(dir_of((0, 0), (3, 1)), None);
    }

    #[test]
    fn states_are_numbered_both_ways() {
        let dims = Dims { w: 7, layers: 3 };
        for (ix, iy, l, d) in [(0, 0, 0, 0), (6, 4, 2, 8), (3, 1, 1, 5)] {
            assert_eq!(dims.dec(dims.enc(ix, iy, l, d)), (ix, iy, l, d));
        }
    }

    #[test]
    fn runs_merge_straight_pieces_and_split_at_vias() {
        let points = [
            ((0, 0), 0),
            ((10, 0), 0),
            ((20, 0), 0),
            ((30, 10), 0),
            ((30, 10), 1),
            ((30, 30), 1),
        ];
        let (runs, vias) = runs_of(&points);
        assert_eq!(vias, vec![((30, 10), 0, 1)]);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], (0, vec![(0, 0), (20, 0), (30, 10)]));
        assert_eq!(runs[1], (1, vec![(30, 10), (30, 30)]));
    }

    #[test]
    fn widths_and_vias_follow_the_rules_and_the_class() {
        use eh3d_phoenix_rules::{NetClass, NetClasses, RuleValues};
        let nm = |v: i64| Nm::new(v).unwrap();
        let settings = RouteSettings::default();
        let rules = RuleValues {
            min_track_width: nm(300_000),
            min_drill: nm(350_000),
            min_via_diameter: nm(500_000),
            min_annular_ring: nm(200_000),
            ..RuleValues::default()
        };
        let classes = NetClasses::new(vec![NetClass {
            name: "power".into(),
            nets: vec!["VCC".into()],
            min_track_width: nm(500_000),
            min_clearance: nm(0),
            min_via_diameter: nm(900_000),
            min_via_drill: nm(0),
        }])
        .unwrap();
        let ctx = RuleContext { rules, classes };
        // the default width is below the narrowest track of the rules, a class asks for more
        assert_eq!(track_width(&ctx, &settings, "SIG"), 300_000);
        assert_eq!(track_width(&ctx, &settings, "VCC"), 500_000);
        // the drill is raised to the rules; the diameter to the drill plus two rings
        assert_eq!(via_size(&ctx, &settings, "SIG"), (350_000, 750_000));
        assert_eq!(via_size(&ctx, &settings, "VCC"), (350_000, 900_000));
    }
}
