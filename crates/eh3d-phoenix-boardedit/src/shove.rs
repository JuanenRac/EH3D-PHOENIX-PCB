// SPDX-License-Identifier: GPL-3.0-or-later
//! Push and shove (prototype): a track is routed and the tracks of other nets that its clearance
//! would invade are pushed aside in parallel.
//!
//! Only straight pieces that are horizontal, vertical or at 45 degrees are pushed. An end of a
//! pushed track, or a vertex, that lies on a pad, a via or another track of the same net stays
//! where it is and a corner joins it to the pushed piece; an end that is joined to nothing moves
//! with its piece. Pads, vias, the edge of the board and
//! the pieces that cannot be pushed (arcs, other angles) are fixed. A pushed piece may push the
//! next one, down to the depth and the number of tracks that `config/boardedit.cfg` allows. The
//! result is all or nothing: it is tried on a copy and checked by the design rules, and a push that
//! would leave a rule error that was not there is refused with a `PHOENIX-SHOVE-<NNN>` code.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::SQRT_2;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, Operation, PadRef, Segment, Track};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::{diff, findings, track_width};
use eh3d_phoenix_rules::Severity;

use crate::codes::BoardEditError;
use crate::copper::{
    fixed_items, gap_to, point_segment, segment_items, segment_segment, segments_cross, vp, Item,
    EPS, V,
};
use crate::ids::IdSupply;
use crate::plan::{
    copper_layer, corner_orders, geometry, net_name, refused, replace_track, shape_of, single, Env,
    Plan,
};
use crate::shape::{legs, lines_through, pt, route_pieces, Shape};
use crate::shove_codes::{
    SHOVE_ADDS_ERROR, SHOVE_BLOCKED, SHOVE_CROSSING, SHOVE_NOT_PUSHABLE, SHOVE_TOO_DEEP,
    SHOVE_TOO_MANY,
};

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

/// What a push would do, to be shown before it is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShovePreview {
    /// The pieces of the new track.
    pub route: Vec<Segment>,
    /// The tracks that move.
    pub moves: Vec<ShoveMove>,
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
fn check_fixed(
    piece: &Piece,
    fixed: &[Item],
    clearance: f64,
    edge_clearance: f64,
) -> Result<(), BoardEditError> {
    let (a, b) = piece.current();
    for item in fixed {
        let Some(gap) = gap_to(item, piece.net, clearance, edge_clearance) else {
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

/// The settings of a push, in floating point.
struct Gaps {
    clearance: f64,
    edge: f64,
}

/// Pushes the tracks that the new track invades; the operation that does it all, what moved and
/// the notes for the person.
fn try_shove(
    board: &Board,
    env: &Env,
    route: &Track,
    route_id: Id,
) -> Result<(Operation, ShovePreview, Vec<String>), BoardEditError> {
    let settings = env.settings;
    let gaps = Gaps {
        clearance: (env.ctx.rules.min_clearance.get() + settings.shove_margin_nm) as f64,
        edge: (env.ctx.rules.min_edge_clearance.get() + settings.shove_margin_nm) as f64,
    };
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
            let need = pusher.half + pieces[j].half + gaps.clearance;
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
            check_fixed(&pieces[j], &fixed, gaps.clearance, gaps.edge)?;
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
        moves: Vec::new(),
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
    ops.push(Operation::AddTrack {
        id: route_id,
        track: route.clone(),
    });
    let batch = Operation::Batch { ops };
    let mut trial = board.clone();
    trial.apply(&batch).map_err(|e| refused(e.to_string()))?;
    let changes = diff(&findings(board, env.ctx), &findings(&trial, env.ctx));
    if let Some(first) = changes.added.iter().find(|f| f.severity == Severity::Error) {
        return Err(BoardEditError::new(
            SHOVE_ADDS_ERROR,
            format!("the result would add {} at {}", first.code, first.location),
        ));
    }
    Ok((batch, preview, notes))
}

/// Plans a route with a push. Each order of the corner is tried in turn; the first that works
/// gives the plan and the preview, else the first error is returned.
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
    let mut first_error: Option<BoardEditError> = None;
    let mut tried: Vec<Vec<Segment>> = Vec::new();
    for h in corner_orders(env, request.horizontal_first) {
        let path = route_pieces(request.points, shape, h)?;
        if path.is_empty() {
            return Err(geometry("the route has no length"));
        }
        if tried.contains(&path) {
            continue;
        }
        tried.push(path.clone());
        let route = Track {
            net: request.net,
            layer: request.layer,
            width,
            path,
        };
        match try_shove(board, env, &route, id) {
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
    Err(first_error.unwrap_or_else(|| refused("no way to push")))
}
