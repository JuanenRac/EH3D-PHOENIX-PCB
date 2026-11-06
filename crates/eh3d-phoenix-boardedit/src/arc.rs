// SPDX-License-Identifier: GPL-3.0-or-later
//! Arcs in tracks (prototype): the operations `route_arc` (an arc of a given radius between two
//! straight pieces of a track) and `round_corners` (every corner of a track or of a net rounded
//! with the largest radius that fits, never below the smallest radius of the settings).
//!
//! The arc is stored in the board as the native arc piece (end points and signed sweep) that the
//! model has, tangent to both straight pieces. The design rules see an arc by its end points only,
//! so the arc is also checked here: cut into short chords, widened by their sagitta, and kept the
//! clearance of the rules away from the copper of other nets, the pads, the vias and the edge of
//! the board (`PHOENIX-ARC-003`). The numbers are floating point for planning only; the points that
//! are kept are whole nanometres.

use std::f64::consts::PI;

use eh3d_phoenix_board::{Board, Id, Operation, Segment, Track};
use eh3d_phoenix_model::{Arc, Point, Udeg};

use crate::codes::BoardEditError;
use crate::copper::{arc_points, fixed_items, gap_to, segment_segment, track_items, vp, EPS, V};
use crate::plan::{refused, replace_track, single, Env, Plan};
use crate::shape::pt;
use crate::shove_codes::{ARC_CLEARANCE, ARC_NO_CORNER, ARC_RADIUS};

/// The largest radius that is accepted, in nanometres.
const RADIUS_LIMIT_NM: i64 = 1 << 29;

fn no_corner(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(ARC_NO_CORNER, what)
}

/// A corner between two straight pieces.
#[derive(Debug, Clone)]
struct Corner {
    at: V,
    u1: V,
    u2: V,
    len1: f64,
    len2: f64,
    turn: f64,
    left: bool,
}

impl Corner {
    fn tan_half(&self) -> f64 {
        (self.turn / 2.0).tan()
    }
}

/// A rounded corner: where the arc starts and ends, and the arc piece.
#[derive(Debug, Clone, Copy)]
struct Rounded {
    start: Point,
    end: Point,
    sweep: Udeg,
}

impl Rounded {
    fn piece(&self) -> Segment {
        Segment::Arc {
            start: self.start,
            end: self.end,
            sweep: self.sweep,
        }
    }
}

fn corner(path: &[Segment], vertex: usize) -> Result<Corner, BoardEditError> {
    if vertex == 0 || vertex >= path.len() {
        return Err(no_corner(format!(
            "the track has pieces 0 to {}, there is no vertex {vertex}",
            path.len().saturating_sub(1)
        )));
    }
    let (
        Segment::Line {
            start: before,
            end: at,
        },
        Segment::Line { end: after, .. },
    ) = (path[vertex - 1], path[vertex])
    else {
        return Err(no_corner(
            "a corner is rounded between two straight pieces, and one of them is an arc",
        ));
    };
    let (p0, at, p1) = (vp(before), vp(at), vp(after));
    let d1 = (at.0 - p0.0, at.1 - p0.1);
    let d2 = (p1.0 - at.0, p1.1 - at.1);
    let (len1, len2) = (d1.0.hypot(d1.1), d2.0.hypot(d2.1));
    if len1 <= 0.0 || len2 <= 0.0 {
        return Err(no_corner("a piece has no length"));
    }
    let (u1, u2) = ((d1.0 / len1, d1.1 / len1), (d2.0 / len2, d2.1 / len2));
    let dot = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0);
    let cross = u1.0 * u2.1 - u1.1 * u2.0;
    let turn = cross.abs().atan2(dot);
    if !(1e-3..=PI - 1e-3).contains(&turn) {
        return Err(no_corner(
            "the two pieces run on one line or turn back on themselves",
        ));
    }
    Ok(Corner {
        at,
        u1,
        u2,
        len1,
        len2,
        turn,
        left: cross > 0.0,
    })
}

/// The arc of `radius` (nanometres) that is tangent to both pieces of a corner.
fn make_arc(c: &Corner, radius: f64) -> Result<Rounded, BoardEditError> {
    let t = radius * c.tan_half();
    let round = |v: V| pt(v.0.round() as i64, v.1.round() as i64);
    let start = round((c.at.0 - c.u1.0 * t, c.at.1 - c.u1.1 * t))?;
    let end = round((c.at.0 + c.u2.0 * t, c.at.1 + c.u2.1 * t))?;
    let degrees = c.turn.to_degrees() * if c.left { 1.0 } else { -1.0 };
    let sweep =
        Udeg::new((degrees * 1_000_000.0).round() as i64).map_err(|e| no_corner(e.to_string()))?;
    Arc::new(start, end, sweep).map_err(|e| no_corner(e.to_string()))?;
    Ok(Rounded { start, end, sweep })
}

fn check_radius(env: &Env, radius_nm: i64) -> Result<(), BoardEditError> {
    let least = env.settings.arc_min_radius_nm;
    if !(least..=RADIUS_LIMIT_NM).contains(&radius_nm) {
        return Err(BoardEditError::new(
            ARC_RADIUS,
            format!(
                "the radius must be between {least} and {RADIUS_LIMIT_NM} nm, it is {radius_nm}"
            ),
        ));
    }
    Ok(())
}

/// Keeps an arc the clearance of the rules away from other copper.
fn check_arc(
    board: &Board,
    env: &Env,
    track: &Track,
    rounded: &Rounded,
) -> Result<(), BoardEditError> {
    let steps = env.settings.arc_steps_per_quarter;
    let (points, sagitta) = arc_points(rounded.start, rounded.end, rounded.sweep.get(), steps)
        .ok_or_else(|| no_corner("the arc has no shape"))?;
    let half = track.width.get() as f64 / 2.0 + sagitta + 1.0;
    let margin = env.settings.arc_margin_nm;
    let clearance = (env.ctx.rules.min_clearance.get() + margin) as f64;
    let edge = (env.ctx.rules.min_edge_clearance.get() + margin) as f64;
    let mut items = fixed_items(board, track.layer);
    items.extend(track_items(board, track.layer, steps));
    for chord in points.windows(2) {
        for item in &items {
            let Some(gap) = gap_to(item, track.net, clearance, edge) else {
                continue;
            };
            let (d, _, _) = segment_segment(chord[0], chord[1], item.a, item.b);
            if d + EPS < half + item.half + gap {
                return Err(BoardEditError::new(
                    ARC_CLEARANCE,
                    format!(
                        "the arc from ({}, {}) to ({}, {}) nm would be too close to other copper",
                        rounded.start.x_nm.get(),
                        rounded.start.y_nm.get(),
                        rounded.end.x_nm.get(),
                        rounded.end.y_nm.get()
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The pieces of a track with the corners rounded: `rounded[v]` is the corner at vertex `v`.
fn assemble(track: &Track, rounded: &[Option<Rounded>]) -> Vec<Segment> {
    let mut path: Vec<Segment> = Vec::new();
    for (i, segment) in track.path.iter().enumerate() {
        match *segment {
            Segment::Line { start, end } => {
                let from = rounded.get(i).copied().flatten().map_or(start, |r| r.end);
                let to = rounded
                    .get(i + 1)
                    .copied()
                    .flatten()
                    .map_or(end, |r| r.start);
                if from != to {
                    path.push(Segment::Line {
                        start: from,
                        end: to,
                    });
                }
                if let Some(r) = rounded.get(i + 1).copied().flatten() {
                    path.push(r.piece());
                }
            }
            Segment::Arc { .. } => path.push(*segment),
        }
    }
    path
}

fn track_of(board: &Board, id: Id) -> Result<&Track, BoardEditError> {
    board
        .tracks
        .get(&id)
        .ok_or_else(|| refused(format!("no track {id}")))
}

/// Plans an arc of a given radius at one vertex of a track.
pub(crate) fn plan_route_arc(
    board: &Board,
    env: &Env,
    track_id: Id,
    vertex: usize,
    radius_nm: i64,
) -> Result<Plan, BoardEditError> {
    check_radius(env, radius_nm)?;
    let track = track_of(board, track_id)?;
    let c = corner(&track.path, vertex)?;
    let radius = radius_nm as f64;
    let reach = radius * c.tan_half();
    if reach > c.len1 + EPS || reach > c.len2 + EPS {
        return Err(no_corner(format!(
            "an arc of radius {radius_nm} nm needs {:.0} nm of each piece, the pieces are {:.0} and {:.0} nm long",
            reach, c.len1, c.len2
        )));
    }
    let made = make_arc(&c, radius)?;
    check_arc(board, env, track, &made)?;
    let mut rounded: Vec<Option<Rounded>> = vec![None; track.path.len() + 1];
    rounded[vertex] = Some(made);
    let mut changed = track.clone();
    changed.path = assemble(track, &rounded);
    let mut plan = single(replace_track(track_id, changed));
    plan.notes
        .push(format!("arc: radius {radius_nm} nm at vertex {vertex}"));
    Ok(plan)
}

/// Rounds the corners of one track; the changed track and how many corners were rounded.
fn round_track(
    board: &Board,
    env: &Env,
    track_id: Id,
    radius_nm: i64,
    notes: &mut Vec<String>,
) -> Result<(Track, usize), BoardEditError> {
    let track = track_of(board, track_id)?;
    let n = track.path.len();
    let mut corners: Vec<Option<Corner>> = vec![None; n + 1];
    for (v, slot) in corners.iter_mut().enumerate().take(n).skip(1) {
        *slot = corner(&track.path, v).ok();
    }
    let mut rounded: Vec<Option<Rounded>> = vec![None; n + 1];
    let mut count = 0;
    for v in 1..n {
        let Some(c) = &corners[v] else {
            continue;
        };
        let share_before = v >= 2 && corners[v - 1].is_some();
        let share_after = v + 1 < n && corners[v + 1].is_some();
        let room1 = if share_before { c.len1 / 2.0 } else { c.len1 };
        let room2 = if share_after { c.len2 / 2.0 } else { c.len2 };
        let radius = (radius_nm as f64).min(room1.min(room2) / c.tan_half());
        if radius + EPS < env.settings.arc_min_radius_nm as f64 {
            notes.push(format!(
                "arc: corner {v} of track {track_id} was left as it is, the room is for a radius of {radius:.0} nm"
            ));
            continue;
        }
        let made = make_arc(c, radius)?;
        check_arc(board, env, track, &made)?;
        rounded[v] = Some(made);
        count += 1;
    }
    let mut changed = track.clone();
    changed.path = assemble(track, &rounded);
    Ok((changed, count))
}

/// Plans the rounding of the corners of a track, or of every track of a net.
pub(crate) fn plan_round_corners(
    board: &Board,
    env: &Env,
    track: Option<Id>,
    net: Option<Id>,
    radius_nm: i64,
) -> Result<Plan, BoardEditError> {
    check_radius(env, radius_nm)?;
    let ids: Vec<Id> = match (track, net) {
        (Some(t), wanted) => {
            let found = track_of(board, t)?;
            if wanted.is_some_and(|n| n != found.net) {
                return Err(refused("the track is not on that net"));
            }
            vec![t]
        }
        (None, Some(n)) => {
            if !board.nets.contains_key(&n) {
                return Err(refused(format!("no net {n}")));
            }
            board
                .tracks
                .iter()
                .filter(|(_, t)| t.net == n)
                .map(|(id, _)| *id)
                .collect()
        }
        (None, None) => return Err(refused("round_corners needs a track or a net")),
    };
    let mut notes: Vec<String> = Vec::new();
    let mut ops: Vec<Operation> = Vec::new();
    let mut total = 0;
    for id in ids {
        let (changed, count) = round_track(board, env, id, radius_nm, &mut notes)?;
        if count > 0 {
            total += count;
            ops.push(replace_track(id, changed));
        }
    }
    if total == 0 {
        let why = notes
            .first()
            .cloned()
            .unwrap_or_else(|| "there is no corner between two straight pieces".to_owned());
        return Err(no_corner(format!("no corner could be rounded: {why}")));
    }
    let mut plan = single(Operation::Batch { ops });
    plan.notes
        .insert(0, format!("arc: {total} corner(s) rounded"));
    plan.notes.extend(notes);
    Ok(plan)
}
