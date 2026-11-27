// SPDX-License-Identifier: GPL-3.0-or-later
//! Push and shove (prototype): a track is routed and the tracks of other nets that its clearance
//! would invade are pushed aside. Moving pieces in parallel is tried first; when that is not
//! enough, the walk of `shove_walk` goes around obstacles and moves vias.
//!
//! In the parallel push only straight pieces that are horizontal, vertical or at 45 degrees are pushed. An end of a
//! pushed track, or a vertex, that lies on a pad, a via or another track of the same net stays
//! where it is and a corner joins it to the pushed piece; an end that is joined to nothing moves
//! with its piece. Pads, vias, the edge of the board and
//! the pieces that cannot be pushed (arcs, other angles) are fixed. A pushed piece may push the
//! next one, down to the depth and the number of tracks that `config/boardedit.cfg` allows. The
//! result of either way is all or nothing: it is tried on a copy and checked by the design rules
//! and the connectivity of every net, and a push that would leave a rule error that was not there,
//! or cut a connection, is refused with a `PHOENIX-SHOVE-<NNN>` code. The clearance of the net
//! classes of the two nets is kept when it is larger than the general one.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::SQRT_2;

use eh3d_phoenix_board::connectivity::{pad_position, ConnectivityReport, Item as Copper};
use eh3d_phoenix_board::{Board, Id, Operation, PadRef, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::{diff, findings, findings_after, track_width};
use eh3d_phoenix_rules::{Finding, Severity};

use crate::codes::BoardEditError;
use crate::copper::{
    fixed_items, gap_to, point_segment, segment_items, segment_segment, segments_cross,
    track_items, vp, Item, EPS, V,
};
use crate::hull::Turn;
use crate::ids::IdSupply;
use crate::plan::{
    copper_layer, corner_orders, geometry, net_name, refused, replace_track, shape_of, single, Env,
    Plan,
};
use crate::shape::{legs, lines_through, pt, route_pieces, Shape};
use crate::shove_codes::{
    SHOVE_ADDS_ERROR, SHOVE_BLOCKED, SHOVE_BREAKS_NET, SHOVE_CROSSING, SHOVE_NOT_PUSHABLE,
    SHOVE_TOO_DEEP, SHOVE_TOO_MANY,
};
use crate::shove_walk::walk_shove;

/// What a route with a push asks for.
pub(crate) struct ShoveRequest<'a> {
    /// The net of the new track.
    pub net: Id,
    /// The copper layer.
    pub layer: Id,
    /// The points of the route.
    pub points: &'a [Point],
    /// The shape; the setting decides when left out.
    pub shape: Option<Shape>,
    /// The width in nanometres; the width of the net class when left out.
    pub width_nm: Option<i64>,
    /// Whether a corner goes first along x; both orders are tried when left out.
    pub horizontal_first: Option<bool>,
}

/// A track that a push moves: its pieces before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShoveMove {
    /// The track.
    pub track: Id,
    /// Its pieces before the push.
    pub before: Vec<Segment>,
    /// Its pieces after the push.
    pub after: Vec<Segment>,
}

/// A via that a push moves: its place before and after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShoveViaMove {
    /// The via.
    pub via: Id,
    /// Its place before the push.
    pub before: Point,
    /// Its place after the push.
    pub after: Point,
    /// The diameter of its pad.
    pub diameter: Nm,
}

/// What a push would do, to be shown before it is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShovePreview {
    /// The pieces of the new track.
    pub route: Vec<Segment>,
    /// The tracks that move.
    pub moves: Vec<ShoveMove>,
    /// The vias that move.
    pub vias: Vec<ShoveViaMove>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Horizontal or vertical.
    Axis,
    /// At 45 degrees.
    Diagonal,
    /// Cannot be pushed: the new track, an arc, any other angle.
    Fixed,
}

fn kind_of(a: Point, b: Point) -> Kind {
    let dx = b.x_nm.get() - a.x_nm.get();
    let dy = b.y_nm.get() - a.y_nm.get();
    if (dx == 0) != (dy == 0) {
        Kind::Axis
    } else if dx != 0 && dx.abs() == dy.abs() {
        Kind::Diagonal
    } else {
        Kind::Fixed
    }
}

/// A straight piece on the layer, with the shift a push gave it.
#[derive(Clone)]
struct Piece {
    track: Option<Id>,
    index: usize,
    net: Id,
    half: f64,
    a: V,
    b: V,
    shift: (i64, i64),
    depth: usize,
    kind: Kind,
}

impl Piece {
    fn current(&self) -> (V, V) {
        let (sx, sy) = (self.shift.0 as f64, self.shift.1 as f64);
        (
            (self.a.0 + sx, self.a.1 + sy),
            (self.b.0 + sx, self.b.1 + sy),
        )
    }
}

fn key(p: Point) -> (i64, i64) {
    (p.x_nm.get(), p.y_nm.get())
}

/// The vertices of a track that must stay where they are when a piece next to them is pushed: those
/// that lie on copper of the same net that is not pushed with it (a pad, a via or another track).
/// An end that is joined to nothing is free and moves with its piece.
fn pinned_vertices(
    board: &Board,
    track_id: Id,
    track: &Track,
    fixed: &[Item],
    steps: usize,
) -> BTreeSet<(i64, i64)> {
    let mut others: Vec<Item> = Vec::new();
    for (id, other) in &board.tracks {
        if *id != track_id && other.net == track.net && other.layer == track.layer {
            let half = other.width.get() as f64 / 2.0;
            for segment in &other.path {
                others.extend(segment_items(segment, half, other.net, steps));
            }
        }
    }
    let attached = |p: Point| {
        let v = vp(p);
        fixed
            .iter()
            .chain(others.iter())
            .filter(|item| item.net == Some(track.net))
            .any(|item| point_segment(v, item.a, item.b).0 <= item.half + 1.0)
    };
    let mut pins = BTreeSet::new();
    for segment in &track.path {
        if attached(segment.start()) {
            pins.insert(key(segment.start()));
        }
    }
    if let Some(last) = track.path.last() {
        if attached(last.end()) {
            pins.insert(key(last.end()));
        }
    }
    pins
}

/// True when something of the same net joins the piece along its length (a via, a pad or the end
/// of another track): pushing the piece would cut that connection.
fn has_branch(board: &Board, track: Id, net: Id, a: V, b: V) -> bool {
    let on_inside = |p: V| p != a && p != b && point_segment(p, a, b).0 <= 1.0;
    if board
        .vias
        .values()
        .any(|v| v.net == net && on_inside(vp(v.at)))
    {
        return true;
    }
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for pad_id in footprint.pads.keys() {
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            if board.net_of(pad_ref) == Some(net) {
                if let Some(at) = pad_position(board, pad_ref) {
                    if on_inside(vp(at)) {
                        return true;
                    }
                }
            }
        }
    }
    board
        .tracks
        .iter()
        .filter(|(id, t)| **id != track && t.net == net)
        .any(|(_, t)| match (t.path.first(), t.path.last()) {
            (Some(first), Some(last)) => on_inside(vp(first.start())) || on_inside(vp(last.end())),
            _ => false,
        })
}

/// How far a piece has to move, along its normal, to be `need` away from the pusher: the shift in
/// whole nanometres on each axis (a 45-degree piece moves along both axes, to stay on its grid).
fn separate(pusher: &Piece, victim: &Piece, need: f64) -> Result<(i64, i64), BoardEditError> {
    let (ia, ib) = pusher.current();
    let (ja, jb) = victim.current();
    let (d0, near_pusher, near_victim) = segment_segment(ia, ib, ja, jb);
    let towards = (near_victim.0 - near_pusher.0, near_victim.1 - near_pusher.1);
    let mid = |a: V, b: V| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let (mp, mv) = (mid(ia, ib), mid(ja, jb));
    let side = |primary: f64, fallback: f64| -> i64 {
        if primary > 0.5 {
            1
        } else if primary < -0.5 {
            -1
        } else if fallback >= 0.0 {
            1
        } else {
            -1
        }
    };
    let (unit, per_step): ((i64, i64), f64) = if victim.kind == Kind::Diagonal {
        let (dx, dy) = (jb.0 - ja.0, jb.1 - ja.1);
        let (nx, ny) = (-dy.signum(), dx.signum());
        let facing = nx * towards.0 + ny * towards.1;
        let flip = if facing.abs() > 0.5 {
            facing < 0.0
        } else {
            nx * (mv.0 - mp.0) + ny * (mv.1 - mp.1) < 0.0
        };
        let f = if flip { -1 } else { 1 };
        ((nx as i64 * f, ny as i64 * f), SQRT_2)
    } else if (jb.1 - ja.1).abs() < 0.5 {
        ((0, side(towards.1, mv.1 - mp.1)), 1.0)
    } else {
        ((side(towards.0, mv.0 - mp.0), 0), 1.0)
    };
    let mut k: i64 = 0;
    let mut dist = d0;
    for _ in 0..8 {
        k += ((need - dist) / per_step).ceil() as i64 + 1;
        let (sx, sy) = ((unit.0 * k) as f64, (unit.1 * k) as f64);
        let (d, _, _) = segment_segment(ia, ib, (ja.0 + sx, ja.1 + sy), (jb.0 + sx, jb.1 + sy));
        if d + EPS >= need {
            return Ok((unit.0 * k, unit.1 * k));
        }
        dist = d;
    }
    Err(BoardEditError::new(
        SHOVE_BLOCKED,
        "the track cannot be pushed far enough",
    ))
}

/// Refuses a piece that, where a push put it, is too close to fixed copper.
fn check_fixed(piece: &Piece, fixed: &[Item], gaps: &Gaps) -> Result<(), BoardEditError> {
    let (a, b) = piece.current();
    for item in fixed {
        let clearance = gaps.between(piece.net, item.net);
        let Some(gap) = gap_to(item, piece.net, clearance, gaps.edge) else {
            continue;
        };
        let (d, _, _) = segment_segment(a, b, item.a, item.b);
        if d + EPS < piece.half + item.half + gap {
            let (x, y) = (item.a.0.round() as i64, item.a.1.round() as i64);
            return Err(BoardEditError::new(
                SHOVE_BLOCKED,
                format!("the pushed track would be too close to fixed copper near ({x}, {y}) nm"),
            ));
        }
    }
    Ok(())
}

fn shifted(p: Point, s: (i64, i64)) -> Result<Point, BoardEditError> {
    pt(p.x_nm.get() + s.0, p.y_nm.get() + s.1)
}

fn join(out: &mut Vec<Segment>, cursor: &mut Point, to: Point) -> Result<(), BoardEditError> {
    if *cursor == to {
        return Ok(());
    }
    out.extend(lines_through(&legs(*cursor, to, Shape::Diagonal, true)?));
    *cursor = to;
    Ok(())
}

fn collinear(a: Point, b: Point, c: Point) -> bool {
    let (abx, aby) = (
        i128::from(b.x_nm.get() - a.x_nm.get()),
        i128::from(b.y_nm.get() - a.y_nm.get()),
    );
    let (bcx, bcy) = (
        i128::from(c.x_nm.get() - b.x_nm.get()),
        i128::from(c.y_nm.get() - b.y_nm.get()),
    );
    abx * bcy - aby * bcx == 0
}

/// Joins two straight pieces in a row that lie on one line (also when the second goes back over
/// the first), except at a vertex that has to stay.
fn merge_collinear(path: Vec<Segment>, pins: &BTreeSet<(i64, i64)>) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for segment in path {
        let mut current = segment;
        loop {
            let (
                Segment::Line { start: b, end: c },
                Some(Segment::Line {
                    start: a,
                    end: before,
                }),
            ) = (current, out.last().copied())
            else {
                out.push(current);
                break;
            };
            if before != b || !collinear(a, b, c) || pins.contains(&key(b)) {
                out.push(current);
                break;
            }
            out.pop();
            if a == c {
                break;
            }
            current = Segment::Line { start: a, end: c };
        }
    }
    out
}

/// The pieces of a track after some of its pieces were shifted: the ends of the track stay, and
/// a corner joins each shifted piece to its neighbours.
fn rebuild(
    track: &Track,
    shifts: &BTreeMap<usize, (i64, i64)>,
    pins: &BTreeSet<(i64, i64)>,
) -> Result<Vec<Segment>, BoardEditError> {
    let (Some(first_piece), Some(last_piece)) = (track.path.first(), track.path.last()) else {
        return Err(refused("a track has no piece"));
    };
    let (first, last) = (first_piece.start(), last_piece.end());
    let shift_of = |i: usize| shifts.get(&i).copied().unwrap_or((0, 0));
    // an end that is joined to something stays; a free end moves with its piece
    let first = if pins.contains(&key(first)) {
        first
    } else {
        shifted(first, shift_of(0))?
    };
    let last = if pins.contains(&key(last)) {
        last
    } else {
        shifted(last, shift_of(track.path.len() - 1))?
    };
    let mut out: Vec<Segment> = Vec::new();
    let mut cursor = first;
    for (i, segment) in track.path.iter().enumerate() {
        let s = shift_of(i);
        let vertex = segment.start();
        if i > 0 && (s != (0, 0) || shift_of(i - 1) != (0, 0)) && pins.contains(&key(vertex)) {
            join(&mut out, &mut cursor, vertex)?;
        }
        match *segment {
            Segment::Line { start, end } => {
                let (a, b) = (shifted(start, s)?, shifted(end, s)?);
                join(&mut out, &mut cursor, a)?;
                out.push(Segment::Line { start: a, end: b });
                cursor = b;
            }
            Segment::Arc { start, end, sweep } => {
                join(&mut out, &mut cursor, start)?;
                out.push(Segment::Arc { start, end, sweep });
                cursor = end;
            }
        }
    }
    join(&mut out, &mut cursor, last)?;
    let merged = merge_collinear(out, pins);
    let ends_kept = merged.first().map(Segment::start) == Some(first)
        && merged.last().map(Segment::end) == Some(last)
        && merged.windows(2).all(|w| w[0].end() == w[1].start());
    if !ends_kept {
        return Err(refused("the pushed track would not keep its ends"));
    }
    Ok(merged)
}

/// The gaps a push keeps: the clearance of the rules, or the larger one of the classes of the two
/// nets, plus the margin.
struct Gaps {
    clearance: i64,
    classes: BTreeMap<Id, i64>,
    margin: i64,
    edge: f64,
}

impl Gaps {
    fn new(board: &Board, env: &Env) -> Gaps {
        let margin = env.settings.shove_margin_nm;
        let classes = board
            .nets
            .iter()
            .filter_map(|(id, net)| {
                let class = env.ctx.classes.class_of(&net.name)?;
                Some((*id, class.min_clearance.get()))
            })
            .collect();
        Gaps {
            clearance: env.ctx.rules.min_clearance.get(),
            classes,
            margin,
            edge: (env.ctx.rules.min_edge_clearance.get() + margin) as f64,
        }
    }

    fn between(&self, a: Id, b: Option<Id>) -> f64 {
        let of = |net: Id| self.classes.get(&net).copied().unwrap_or(0);
        let class = of(a).max(b.map_or(0, of));
        (self.clearance.max(class) + self.margin) as f64
    }
}

/// What one way of pushing would change, before the rules have judged it.
struct Pushed {
    route: Track,
    ops: Vec<Operation>,
    preview: ShovePreview,
    notes: Vec<String>,
}

/// Pushes in parallel the tracks that the new track invades.
fn parallel_push(board: &Board, env: &Env, route: &Track) -> Result<Pushed, BoardEditError> {
    let settings = env.settings;
    let gaps = Gaps::new(board, env);
    let route_half = route.width.get() as f64 / 2.0;
    let mut pieces: Vec<Piece> = Vec::new();
    for (index, segment) in route.path.iter().enumerate() {
        pieces.push(Piece {
            track: None,
            index,
            net: route.net,
            half: route_half,
            a: vp(segment.start()),
            b: vp(segment.end()),
            shift: (0, 0),
            depth: 0,
            kind: Kind::Fixed,
        });
    }
    let route_count = pieces.len();
    for (track_id, track) in board.tracks.iter().filter(|(_, t)| t.layer == route.layer) {
        let half = track.width.get() as f64 / 2.0;
        for (index, segment) in track.path.iter().enumerate() {
            match *segment {
                Segment::Line { start, end } => pieces.push(Piece {
                    track: Some(*track_id),
                    index,
                    net: track.net,
                    half,
                    a: vp(start),
                    b: vp(end),
                    shift: (0, 0),
                    depth: 0,
                    kind: kind_of(start, end),
                }),
                Segment::Arc { .. } => {
                    for item in
                        segment_items(segment, half, track.net, settings.arc_steps_per_quarter)
                    {
                        pieces.push(Piece {
                            track: Some(*track_id),
                            index,
                            net: track.net,
                            half: item.half,
                            a: item.a,
                            b: item.b,
                            shift: (0, 0),
                            depth: 0,
                            kind: Kind::Fixed,
                        });
                    }
                }
            }
        }
    }
    let fixed = fixed_items(board, route.layer);
    let mut moved: BTreeSet<Id> = BTreeSet::new();
    let mut queue: VecDeque<usize> = (0..route_count).collect();
    let budget = 64 * settings.shove_max_moved;
    let mut pushes = 0usize;
    while let Some(i) = queue.pop_front() {
        for j in 0..pieces.len() {
            if j == i || pieces[j].net == pieces[i].net {
                continue;
            }
            let pusher = pieces[i].clone();
            let (ia, ib) = pusher.current();
            let (ja, jb) = pieces[j].current();
            let between = gaps.between(pusher.net, Some(pieces[j].net));
            let need = pusher.half + pieces[j].half + between;
            let (d, _, _) = segment_segment(ia, ib, ja, jb);
            if d + EPS >= need {
                continue;
            }
            if d < 1.0 || segments_cross(ia, ib, ja, jb) {
                return Err(BoardEditError::new(
                    SHOVE_CROSSING,
                    "the route crosses or touches a track of another net",
                ));
            }
            let (Some(victim_track), true) = (pieces[j].track, pieces[j].kind != Kind::Fixed)
            else {
                return Err(BoardEditError::new(
                    SHOVE_NOT_PUSHABLE,
                    "a track in the way is an arc or has a piece that is not horizontal, vertical or at 45 degrees",
                ));
            };
            if has_branch(board, victim_track, pieces[j].net, pieces[j].a, pieces[j].b) {
                return Err(BoardEditError::new(
                    SHOVE_NOT_PUSHABLE,
                    "a branch of the net of the track in the way joins it along its length",
                ));
            }
            let depth = pusher.depth + 1;
            if depth > settings.shove_max_depth {
                return Err(BoardEditError::new(
                    SHOVE_TOO_DEEP,
                    format!(
                        "the push goes deeper than {} levels",
                        settings.shove_max_depth
                    ),
                ));
            }
            moved.insert(victim_track);
            pushes += 1;
            if moved.len() > settings.shove_max_moved || pushes > budget {
                return Err(BoardEditError::new(
                    SHOVE_TOO_MANY,
                    format!(
                        "the push would move more than {} tracks",
                        settings.shove_max_moved
                    ),
                ));
            }
            let extra = separate(&pusher, &pieces[j], need)?;
            let victim = &mut pieces[j];
            victim.shift = (victim.shift.0 + extra.0, victim.shift.1 + extra.1);
            victim.depth = victim.depth.max(depth);
            check_fixed(&pieces[j], &fixed, &gaps)?;
            queue.push_back(j);
        }
    }
    let mut shifts: BTreeMap<Id, BTreeMap<usize, (i64, i64)>> = BTreeMap::new();
    for piece in &pieces {
        if let Some(track_id) = piece.track {
            if piece.shift != (0, 0) {
                shifts
                    .entry(track_id)
                    .or_default()
                    .insert(piece.index, piece.shift);
            }
        }
    }
    let mut ops: Vec<Operation> = Vec::new();
    let mut preview = ShovePreview {
        route: route.path.clone(),
        ..ShovePreview::default()
    };
    let mut notes: Vec<String> = Vec::new();
    for (track_id, by_index) in &shifts {
        let Some(track) = board.tracks.get(track_id) else {
            continue;
        };
        let pins = pinned_vertices(
            board,
            *track_id,
            track,
            &fixed,
            settings.arc_steps_per_quarter,
        );
        let path = rebuild(track, by_index, &pins)?;
        let largest = by_index
            .values()
            .map(|(x, y)| x.abs().max(y.abs()))
            .max()
            .unwrap_or(0);
        notes.push(format!(
            "shove: track {track_id} moved, {} piece(s) pushed, largest shift {largest} nm",
            by_index.len()
        ));
        preview.moves.push(ShoveMove {
            track: *track_id,
            before: track.path.clone(),
            after: path.clone(),
        });
        let mut changed = track.clone();
        changed.path = path;
        ops.push(replace_track(*track_id, changed));
    }
    notes.insert(
        0,
        if shifts.is_empty() {
            "shove: nothing was in the way".to_owned()
        } else {
            format!("shove: {} track(s) moved", shifts.len())
        },
    );
    Ok(Pushed {
        route: route.clone(),
        ops,
        preview,
        notes,
    })
}

/// Walks the new track and what it crowds around obstacles and moves vias out of the way.
fn walked_push(
    board: &Board,
    env: &Env,
    route: &Track,
    turn: Turn,
) -> Result<Pushed, BoardEditError> {
    let found = walk_shove(board, env, route, turn)?;
    let pieces = |points: &[(i64, i64)]| -> Result<Vec<Segment>, BoardEditError> {
        let points: Result<Vec<Point>, _> = points.iter().map(|p| pt(p.0, p.1)).collect();
        Ok(lines_through(&points?))
    };
    let mut new_route = route.clone();
    new_route.path = pieces(&found.route)?;
    let went_around = new_route.path != route.path;
    let mut preview = ShovePreview {
        route: new_route.path.clone(),
        ..ShovePreview::default()
    };
    let mut ops: Vec<Operation> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for (id, at) in &found.vias {
        let Some(via) = board.vias.get(id) else {
            continue;
        };
        let after = pt(at.0, at.1)?;
        let shift = (at.0 - via.at.x_nm.get())
            .abs()
            .max((at.1 - via.at.y_nm.get()).abs());
        notes.push(format!("shove: via {id} moved, largest shift {shift} nm"));
        preview.vias.push(ShoveViaMove {
            via: *id,
            before: via.at,
            after,
            diameter: via.diameter,
        });
        ops.push(Operation::RemoveVia { id: *id });
        ops.push(Operation::AddVia {
            id: *id,
            via: Via {
                at: after,
                ..via.clone()
            },
        });
    }
    for (id, points) in &found.tracks {
        let Some(track) = board.tracks.get(id) else {
            continue;
        };
        let path = pieces(points)?;
        if path.is_empty() {
            return Err(refused("a pushed track would have no length"));
        }
        notes.push(format!(
            "shove: track {id} moved, now {} piece(s)",
            path.len()
        ));
        preview.moves.push(ShoveMove {
            track: *id,
            before: track.path.clone(),
            after: path.clone(),
        });
        let mut changed = track.clone();
        changed.path = path;
        ops.push(replace_track(*id, changed));
    }
    let (moved, vias) = (preview.moves.len(), preview.vias.len());
    let mut first = if moved == 0 && vias == 0 {
        "shove: nothing had to move".to_owned()
    } else {
        format!("shove: {moved} track(s) moved, {vias} via(s) moved")
    };
    if went_around {
        first.push_str(", the new track goes around what cannot move");
    }
    notes.insert(0, first);
    Ok(Pushed {
        route: new_route,
        ops,
        preview,
        notes,
    })
}

/// What the board was before the push, worked out once for every way that is tried.
#[derive(Default)]
struct Before {
    findings: Option<Vec<Finding>>,
}

/// The nets of the tracks and vias that are not the same on the two boards: the only ones whose
/// connections a push can cut.
fn touched_nets(board: &Board, trial: &Board) -> BTreeSet<Id> {
    let mut nets = BTreeSet::new();
    for (one, other) in [(board, trial), (trial, board)] {
        for (id, track) in &one.tracks {
            if other.tracks.get(id) != Some(track) {
                nets.insert(track.net);
            }
        }
        for (id, via) in &one.vias {
            if other.vias.get(id) != Some(via) {
                nets.insert(via.net);
            }
        }
    }
    nets
}

/// A net that no longer joins every pair of items that it joined before, if there is one.
fn cut_net(before: &ConnectivityReport, after: &ConnectivityReport) -> Option<Id> {
    let key = |net: Id, item: &Copper| match *item {
        Copper::Pad(pad) => (net, 0_u8, pad.component, pad.pad),
        Copper::Track(id) => (net, 1, id, id),
        Copper::Via(id) => (net, 2, id, id),
        Copper::Zone(id) => (net, 3, id, id),
    };
    let mut island_of: BTreeMap<(Id, u8, Id, Id), usize> = BTreeMap::new();
    for net in &after.nets {
        for (n, island) in net.islands.iter().enumerate() {
            for item in &island.items {
                island_of.insert(key(net.net, item), n);
            }
        }
    }
    for net in &before.nets {
        for island in &net.islands {
            let mut found = island
                .items
                .iter()
                .map(|item| island_of.get(&key(net.net, item)));
            if let Some(first) = found.next() {
                if first.is_none() || found.any(|other| other != first) {
                    return Some(net.net);
                }
            }
        }
    }
    None
}

/// True when copper that the push placed is nearer to copper of another net than the clearance of
/// the net classes of the two asks. The design rules do not check the clearance of a class, so the
/// push does.
fn crowds_a_class(board: &Board, env: &Env, trial: &Board) -> bool {
    let gaps = Gaps::new(board, env);
    if gaps.classes.values().all(|c| *c <= gaps.clearance) {
        return false;
    }
    let steps = env.settings.arc_steps_per_quarter;
    // the pieces and vias that are not where they were
    let mut placed: BTreeMap<Id, Vec<Item>> = BTreeMap::new();
    for (id, track) in &trial.tracks {
        let old = board.tracks.get(id);
        let half = track.width.get() as f64 / 2.0;
        for segment in &track.path {
            if old.is_none_or(|t| !t.path.contains(segment)) {
                placed
                    .entry(track.layer)
                    .or_default()
                    .extend(segment_items(segment, half, track.net, steps));
            }
        }
    }
    for (id, via) in &trial.vias {
        if board.vias.get(id).is_none_or(|v| v.at != via.at) {
            for layer in trial.via_layers(via) {
                placed.entry(layer).or_default().push(Item {
                    a: vp(via.at),
                    b: vp(via.at),
                    half: via.diameter.get() as f64 / 2.0,
                    net: Some(via.net),
                    edge: false,
                });
            }
        }
    }
    placed.iter().any(|(layer, items)| {
        let mut others = fixed_items(trial, *layer);
        others.extend(track_items(trial, *layer, steps));
        items.iter().any(|item| {
            let Some(net) = item.net else {
                return false;
            };
            others.iter().any(|other| {
                if other.edge || other.net == Some(net) {
                    return false;
                }
                let asked = gaps.between(net, other.net) - gaps.margin as f64;
                if asked <= gaps.clearance as f64 {
                    return false;
                }
                let (found, _, _) = segment_segment(item.a, item.b, other.a, other.b);
                found + 1.0 < item.half + other.half + asked
            })
        })
    })
}

/// Puts the new track in and tries the whole push on a copy: the connectivity, the clearance of
/// the net classes and the rules judge it.
fn judge(
    board: &Board,
    env: &Env,
    before: &mut Before,
    pushed: Pushed,
    route_id: Id,
) -> Result<(Operation, ShovePreview, Vec<String>), BoardEditError> {
    let moved = !pushed.ops.is_empty();
    let mut ops = pushed.ops;
    ops.push(Operation::AddTrack {
        id: route_id,
        track: pushed.route,
    });
    let batch = Operation::Batch { ops };
    let mut trial = board.clone();
    trial.apply(&batch).map_err(|e| refused(e.to_string()))?;
    if moved {
        let nets = touched_nets(board, &trial);
        let was = board.connectivity_of(&nets);
        if let Some(net) = cut_net(&was, &trial.connectivity_of(&nets)) {
            return Err(BoardEditError::new(
                SHOVE_BREAKS_NET,
                format!("the result would cut a connection of net {net}"),
            ));
        }
    }
    if crowds_a_class(board, env, &trial) {
        return Err(BoardEditError::new(
            SHOVE_ADDS_ERROR,
            "the result would be nearer than the clearance of a net class",
        ));
    }
    let was: &[Finding] = match env.known {
        Some(known) => known,
        None => before
            .findings
            .get_or_insert_with(|| findings(board, env.ctx)),
    };
    let now = if env.settings.local_checks {
        findings_after(board, was, &trial, env.ctx)
    } else {
        findings(&trial, env.ctx)
    };
    let changes = diff(was, &now);
    if let Some(first) = changes.added.iter().find(|f| f.severity == Severity::Error) {
        return Err(BoardEditError::new(
            SHOVE_ADDS_ERROR,
            format!("the result would add {} at {}", first.code, first.location),
        ));
    }
    Ok((batch, pushed.preview, pushed.notes))
}

/// Plans a route with a push. Each order of the corner is tried with the parallel push and then,
/// when the settings allow walks or via moves, walking around obstacles (the shorter way, then one
/// way round and the other). The first that works gives the plan and the preview, else the first
/// error is returned.
pub(crate) fn plan_shove(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    request: &ShoveRequest,
) -> Result<(Plan, ShovePreview), BoardEditError> {
    let name = net_name(board, request.net)?;
    copper_layer(board, request.layer)?;
    if request.points.len() < 2 {
        return Err(geometry("a route needs at least two points"));
    }
    let width = request
        .width_nm
        .unwrap_or_else(|| track_width(env.ctx, env.route, &name));
    if width <= 0 {
        return Err(geometry("the width of the track is not above zero"));
    }
    let width = Nm::new(width).map_err(|e| geometry(e.to_string()))?;
    let shape = shape_of(env, request.shape);
    let id = ids.next_for(board);
    let mut routes: Vec<Track> = Vec::new();
    for h in corner_orders(env, request.horizontal_first) {
        let path = route_pieces(request.points, shape, h)?;
        if path.is_empty() {
            return Err(geometry("the route has no length"));
        }
        if routes.iter().all(|r| r.path != path) {
            routes.push(Track {
                net: request.net,
                layer: request.layer,
                width,
                path,
            });
        }
    }
    // `None` is the parallel push
    let mut ways: Vec<Option<Turn>> = vec![None];
    if env.settings.shove_walk_around || env.settings.shove_vias {
        ways.extend([Turn::Shortest, Turn::Left, Turn::Right].map(Some));
    }
    let mut before = Before::default();
    let mut first_error: Option<BoardEditError> = None;
    let mut judged: Vec<ShovePreview> = Vec::new();
    for way in ways {
        for route in &routes {
            let pushed = match way {
                None => parallel_push(board, env, route),
                Some(turn) => walked_push(board, env, route, turn),
            };
            let result = pushed.and_then(|pushed| {
                if judged.contains(&pushed.preview) {
                    return Err(refused("this way was refused already"));
                }
                judged.push(pushed.preview.clone());
                judge(board, env, &mut before, pushed, id)
            });
            match result {
                Ok((batch, preview, notes)) => {
                    let mut plan = single(batch);
                    plan.created = vec![id];
                    plan.notes = notes;
                    return Ok((plan, preview));
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
    }
    Err(first_error.unwrap_or_else(|| refused("no way to push")))
}
