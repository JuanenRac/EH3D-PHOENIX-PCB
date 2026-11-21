// SPDX-License-Identifier: GPL-3.0-or-later
//! The plans of the editing operations that change what is already on the board: the width of a
//! track, a via, the cut and the join of tracks, the tidy of the pieces of a track, the value, the
//! designator and the net names, the merge of two nets and the snap of components to the grid.
//! Each plan reads the board and changes nothing; the editor tries it on a copy and checks it by
//! the design rules like every other operation.

use eh3d_phoenix_board::{Board, Id, Operation, PadRef, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::codes::BoardEditError;
use crate::plan::{check_unlocked, geometry, refused, replace_track, single, Env, Plan};

fn nm(value: i64) -> Result<Nm, BoardEditError> {
    Nm::new(value).map_err(|e| geometry(e.to_string()))
}

fn track_of(board: &Board, id: Id) -> Result<&Track, BoardEditError> {
    board
        .tracks
        .get(&id)
        .ok_or_else(|| refused(format!("no track {id}")))
}

fn via_of(board: &Board, id: Id) -> Result<&Via, BoardEditError> {
    board
        .vias
        .get(&id)
        .ok_or_else(|| refused(format!("no via {id}")))
}

pub(crate) fn set_track_width(
    board: &Board,
    track: Id,
    width_nm: i64,
) -> Result<Plan, BoardEditError> {
    let old = track_of(board, track)?;
    if width_nm <= 0 {
        return Err(geometry("the width of a track is above zero"));
    }
    let width = nm(width_nm)?;
    if old.width == width {
        return Err(refused("the track has this width already"));
    }
    let mut changed = old.clone();
    changed.width = width;
    Ok(single(replace_track(track, changed)))
}

pub(crate) fn move_via(board: &Board, via: Id, to: Point) -> Result<Plan, BoardEditError> {
    let old = via_of(board, via)?;
    if old.at == to {
        return Err(refused("the via is at this place already"));
    }
    let mut changed = old.clone();
    changed.at = to;
    Ok(single(Operation::Batch {
        ops: vec![
            Operation::RemoveVia { id: via },
            Operation::AddVia {
                id: via,
                via: changed,
            },
        ],
    }))
}

pub(crate) fn resize_via(
    board: &Board,
    via: Id,
    drill_nm: i64,
    diameter_nm: i64,
) -> Result<Plan, BoardEditError> {
    let old = via_of(board, via)?;
    if drill_nm <= 0 || diameter_nm <= drill_nm {
        return Err(geometry(
            "the pad of a via is larger than its drill, which is above zero",
        ));
    }
    let (drill, diameter) = (nm(drill_nm)?, nm(diameter_nm)?);
    if old.drill == drill && old.diameter == diameter {
        return Err(refused("the via has these sizes already"));
    }
    let mut changed = old.clone();
    changed.drill = drill;
    changed.diameter = diameter;
    Ok(single(Operation::Batch {
        ops: vec![
            Operation::RemoveVia { id: via },
            Operation::AddVia {
                id: via,
                via: changed,
            },
        ],
    }))
}

/// Cuts a track after one of its pieces: the first part keeps the id, the second one is new.
pub(crate) fn split_track(
    board: &Board,
    track: Id,
    piece: usize,
    new_id: Id,
) -> Result<Plan, BoardEditError> {
    let old = track_of(board, track)?;
    if piece + 1 >= old.path.len() {
        return Err(refused(format!(
            "the track has {} pieces: it can be cut after piece 0 to {}",
            old.path.len(),
            old.path.len().saturating_sub(2)
        )));
    }
    let mut first = old.clone();
    let mut second = old.clone();
    first.path.truncate(piece + 1);
    second.path = old.path[piece + 1..].to_vec();
    let mut plan = single(Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id: track },
            Operation::AddTrack {
                id: track,
                track: first,
            },
            Operation::AddTrack {
                id: new_id,
                track: second,
            },
        ],
    });
    plan.created.push(new_id);
    Ok(plan)
}

/// Joins two tracks of the same net, layer and width whose ends meet: `b` goes on at the end of
/// `a` (or the other way round; `b` is turned round when that is what makes the ends meet).
pub(crate) fn join_tracks(board: &Board, a: Id, b: Id) -> Result<Plan, BoardEditError> {
    if a == b {
        return Err(refused("a track cannot be joined to itself"));
    }
    let (ta, tb) = (track_of(board, a)?, track_of(board, b)?);
    if ta.net != tb.net || ta.layer != tb.layer || ta.width != tb.width {
        return Err(refused(
            "the tracks differ in net, layer or width and cannot be joined",
        ));
    }
    let (Some(a_first), Some(a_last), Some(b_first), Some(b_last)) = (
        ta.path.first(),
        ta.path.last(),
        tb.path.first(),
        tb.path.last(),
    ) else {
        return Err(refused("a track without pieces cannot be joined"));
    };
    let mut path: Vec<Segment>;
    if a_last.end() == b_first.start() {
        path = ta.path.clone();
        path.extend(tb.path.iter().cloned());
    } else if b_last.end() == a_first.start() {
        path = tb.path.clone();
        path.extend(ta.path.iter().cloned());
    } else if a_last.end() == b_last.end() {
        path = ta.path.clone();
        path.extend(reversed(&tb.path));
    } else if a_first.start() == b_first.start() {
        path = reversed(&ta.path);
        path.extend(tb.path.iter().cloned());
    } else {
        return Err(refused("the ends of the two tracks do not meet"));
    }
    let mut joined = ta.clone();
    joined.path = path;
    Ok(single(Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id: a },
            Operation::RemoveTrack { id: b },
            Operation::AddTrack {
                id: a,
                track: joined,
            },
        ],
    }))
}

fn reversed(path: &[Segment]) -> Vec<Segment> {
    path.iter()
        .rev()
        .map(|s| match s {
            Segment::Line { start, end } => Segment::Line {
                start: *end,
                end: *start,
            },
            Segment::Arc { start, end, sweep } => Segment::Arc {
                start: *end,
                end: *start,
                sweep: Udeg::new(-sweep.get()).unwrap_or(*sweep),
            },
        })
        .collect()
}

fn direction(a: Point, b: Point) -> (i128, i128) {
    (
        i128::from(b.x_nm.get()) - i128::from(a.x_nm.get()),
        i128::from(b.y_nm.get()) - i128::from(a.y_nm.get()),
    )
}

/// The pieces of a track without the ones of no length, and with two straight pieces in a row that
/// point the same way made into one.
pub fn tidied_path(path: &[Segment]) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for piece in path {
        if let Segment::Line { start, end } = piece {
            if start == end {
                continue;
            }
            if let Some(Segment::Line { start: s0, end: e0 }) = out.last().cloned() {
                let (d0, d1) = (direction(s0, e0), direction(*start, *end));
                let cross = d0.0 * d1.1 - d0.1 * d1.0;
                let dot = d0.0 * d1.0 + d0.1 * d1.1;
                if e0 == *start && cross == 0 && dot > 0 {
                    out.pop();
                    out.push(Segment::Line {
                        start: s0,
                        end: *end,
                    });
                    continue;
                }
            }
        }
        out.push(*piece);
    }
    out
}

/// Tidies the tracks of a net (every net when none is given).
pub(crate) fn tidy_tracks(board: &Board, net: Option<Id>) -> Result<Plan, BoardEditError> {
    if let Some(net) = net {
        if !board.nets.contains_key(&net) {
            return Err(refused(format!("no net {net}")));
        }
    }
    let mut ops: Vec<Operation> = Vec::new();
    for (id, track) in &board.tracks {
        if net.is_some_and(|n| n != track.net) {
            continue;
        }
        let tidy = tidied_path(&track.path);
        if tidy == track.path {
            continue;
        }
        if tidy.is_empty() {
            ops.push(Operation::RemoveTrack { id: *id });
        } else {
            let mut changed = track.clone();
            changed.path = tidy;
            ops.push(replace_track(*id, changed));
        }
    }
    if ops.is_empty() {
        return Err(refused("the tracks are tidy already"));
    }
    Ok(single(Operation::Batch { ops }))
}

pub(crate) fn set_value(
    board: &Board,
    env: &Env,
    component: Id,
    value: &str,
) -> Result<Plan, BoardEditError> {
    let old = board
        .components
        .get(&component)
        .ok_or_else(|| refused(format!("no component {component}")))?;
    check_unlocked(env, component)?;
    if old.value == value {
        return Err(refused("the component has this value already"));
    }
    Ok(single(Operation::SetComponentFields {
        id: component,
        refdes: old.refdes.clone(),
        value: value.to_owned(),
        footprint: old.footprint,
    }))
}

pub(crate) fn set_refdes(
    board: &Board,
    env: &Env,
    component: Id,
    refdes: &str,
) -> Result<Plan, BoardEditError> {
    let old = board
        .components
        .get(&component)
        .ok_or_else(|| refused(format!("no component {component}")))?;
    check_unlocked(env, component)?;
    if refdes.trim().is_empty() {
        return Err(refused("a designator cannot be empty"));
    }
    if old.refdes == refdes {
        return Err(refused("the component has this designator already"));
    }
    if board
        .components
        .iter()
        .any(|(id, c)| *id != component && c.refdes == refdes)
    {
        return Err(refused(format!("the designator {refdes} is used already")));
    }
    Ok(single(Operation::SetComponentFields {
        id: component,
        refdes: refdes.to_owned(),
        value: old.value.clone(),
        footprint: old.footprint,
    }))
}

pub(crate) fn rename_net(board: &Board, net: Id, name: &str) -> Result<Plan, BoardEditError> {
    let old = board
        .nets
        .get(&net)
        .ok_or_else(|| refused(format!("no net {net}")))?;
    if name.trim().is_empty() {
        return Err(refused("the name of a net cannot be empty"));
    }
    if old.name == name {
        return Err(refused("the net has this name already"));
    }
    if board
        .nets
        .iter()
        .any(|(id, n)| *id != net && n.name == name)
    {
        return Err(refused(format!("a net is called {name} already")));
    }
    Ok(single(Operation::RenameNet {
        id: net,
        name: name.to_owned(),
    }))
}

/// Moves everything of the net `from` to the net `into` and removes `from`: the nets of the pads,
/// the tracks and the vias.
pub(crate) fn merge_nets(board: &Board, from: Id, into: Id) -> Result<Plan, BoardEditError> {
    if from == into {
        return Err(refused("a net cannot be merged into itself"));
    }
    for net in [from, into] {
        if !board.nets.contains_key(&net) {
            return Err(refused(format!("no net {net}")));
        }
    }
    let mut ops: Vec<Operation> = Vec::new();
    for pad_net in &board.pad_nets {
        if pad_net.net == from {
            ops.push(Operation::SetPadNet {
                pad: PadRef {
                    component: pad_net.component,
                    pad: pad_net.pad,
                },
                net: Some(into),
            });
        }
    }
    for (id, track) in &board.tracks {
        if track.net == from {
            let mut changed = track.clone();
            changed.net = into;
            ops.push(replace_track(*id, changed));
        }
    }
    for (id, via) in &board.vias {
        if via.net == from {
            let mut changed = via.clone();
            changed.net = into;
            ops.push(Operation::RemoveVia { id: *id });
            ops.push(Operation::AddVia {
                id: *id,
                via: changed,
            });
        }
    }
    if board.zones.values().any(|z| z.net == from) {
        return Err(refused(
            "a zone is on the net: change the net of the zone first",
        ));
    }
    ops.push(Operation::RemoveNet { id: from });
    Ok(single(Operation::Batch { ops }))
}

fn snapped(value: i64, grid: i64) -> i64 {
    let half = grid / 2;
    (value + half).div_euclid(grid) * grid
}

/// Puts each component on the nearest point of the grid (the pads move with it).
pub(crate) fn snap_to_grid(
    board: &Board,
    env: &Env,
    components: &[Id],
    grid_nm: i64,
) -> Result<Plan, BoardEditError> {
    if grid_nm <= 0 {
        return Err(geometry("the grid is above zero"));
    }
    let mut ops: Vec<Operation> = Vec::new();
    for id in components {
        let c = board
            .components
            .get(id)
            .ok_or_else(|| refused(format!("no component {id}")))?;
        check_unlocked(env, *id)?;
        let at = Point::from_nm(
            snapped(c.at.x_nm.get(), grid_nm),
            snapped(c.at.y_nm.get(), grid_nm),
        )
        .map_err(|e| geometry(e.to_string()))?;
        if at != c.at {
            ops.push(Operation::MoveComponent {
                id: *id,
                at,
                rotation: c.rotation,
            });
        }
    }
    if ops.is_empty() {
        return Err(refused("the components are on the grid already"));
    }
    Ok(single(Operation::Batch { ops }))
}

/// The place of `at` on a straight piece, when it is on it exactly.
fn on_piece(a: Point, b: Point, at: Point) -> bool {
    let (d, e) = (direction(a, b), direction(a, at));
    let cross = d.0 * e.1 - d.1 * e.0;
    let dot = d.0 * e.0 + d.1 * e.1;
    cross == 0 && dot >= 0 && dot <= d.0 * d.0 + d.1 * d.1
}

/// Cuts a track at a point of one of its straight pieces.
pub(crate) fn split_track_at(
    board: &Board,
    track: Id,
    at: Point,
    new_id: Id,
) -> Result<Plan, BoardEditError> {
    let old = track_of(board, track)?;
    for (i, piece) in old.path.iter().enumerate() {
        if piece.end() == at {
            return split_track(board, track, i, new_id);
        }
    }
    let Some(i) = old.path.iter().position(|s| match s {
        Segment::Line { start, end } => on_piece(*start, *end, at) && *start != at,
        Segment::Arc { .. } => false,
    }) else {
        return Err(refused("the point is not on a straight piece of the track"));
    };
    let mut first = old.clone();
    let mut second = old.clone();
    let (start, end) = (old.path[i].start(), old.path[i].end());
    first.path.truncate(i);
    first.path.push(Segment::Line { start, end: at });
    second.path = vec![Segment::Line { start: at, end }];
    second.path.extend(old.path[i + 1..].iter().cloned());
    let mut plan = single(Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id: track },
            Operation::AddTrack {
                id: track,
                track: first,
            },
            Operation::AddTrack {
                id: new_id,
                track: second,
            },
        ],
    });
    plan.created.push(new_id);
    Ok(plan)
}
