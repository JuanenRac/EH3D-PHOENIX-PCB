// SPDX-License-Identifier: GPL-3.0-or-later
//! The planner: turns an [`EditOp`] into the reversible operations of the board. It reads the
//! board and changes nothing. A plan holds one or more alternatives (another order of the corner,
//! another side of the meanders); the editor keeps the first that the board accepts and the rules
//! pass.

use std::collections::BTreeSet;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{
    Board, Component, Id, LayerKind, Operation, PadRef, Segment, Side, Track,
};
use eh3d_phoenix_lengths::{measure_nets, LengthSettings};
use eh3d_phoenix_model::{Nm, Point, Udeg, ANGLE_LIMIT_UDEG};
use eh3d_phoenix_route::edit as route_edit;
use eh3d_phoenix_route::{copper_layers, track_width, RouteSettings, RuleContext};
use eh3d_phoenix_rules::Lang;

use crate::arc;
use crate::codes::{
    BoardEditError, COMPONENT_LOCKED, GEOMETRY_INVALID, OPERATION_REFUSED, TUNE_IMPOSSIBLE,
};
use crate::ids::IdSupply;
use crate::ops::EditOp;
use crate::outline;
use crate::pair_plan;
use crate::pairs::PairConfig;
use crate::settings::BoardEditSettings;
use crate::shape::{
    legs, lines_through, piece_length, route_pieces, serpentine, MeanderSide, Shape, Tuning,
};
use crate::shove;

/// What the planner reads besides the board.
pub struct Env<'a> {
    /// The rules and the net classes.
    pub ctx: &'a RuleContext,
    /// The settings of the router (the width and the via come from the settings of the editor).
    pub route: &'a RouteSettings,
    /// The settings of the editor.
    pub settings: &'a BoardEditSettings,
    /// The components that are locked.
    pub locked: &'a BTreeSet<Id>,
    /// The differential pairs and their settings.
    pub pairs: &'a PairConfig,
}

/// The length that a tuning has to reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expect {
    /// The net.
    pub net: Id,
    /// The length asked for, in nanometres.
    pub target_nm: i64,
    /// How far from it the result may be.
    pub tolerance_nm: i64,
    /// The length before the operation.
    pub before_nm: i64,
}

/// What an operation will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The ways to do it, in the order they are tried. An empty batch does nothing.
    pub alternatives: Vec<Operation>,
    /// The ids of the parts that the operation makes.
    pub created: Vec<Id>,
    /// Things to tell the person.
    pub notes: Vec<String>,
    /// The length that the result has to have, for a tuning.
    pub expect: Option<Expect>,
}

pub(crate) fn refused(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(OPERATION_REFUSED, what)
}

pub(crate) fn geometry(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(GEOMETRY_INVALID, what)
}

pub(crate) fn single(op: Operation) -> Plan {
    Plan {
        alternatives: vec![op],
        created: Vec::new(),
        notes: Vec::new(),
        expect: None,
    }
}

fn check_unlocked(env: &Env, id: Id) -> Result<(), BoardEditError> {
    if env.locked.contains(&id) {
        return Err(BoardEditError::new(
            COMPONENT_LOCKED,
            format!("component {id} is locked"),
        ));
    }
    Ok(())
}

pub(crate) fn copper_layer(board: &Board, layer: Id) -> Result<(), BoardEditError> {
    match board.layers.get(&layer) {
        Some(l) if l.kind == LayerKind::Copper => Ok(()),
        Some(_) => Err(refused(format!("the layer {layer} is not a copper layer"))),
        None => Err(refused(format!("no layer {layer}"))),
    }
}

pub(crate) fn net_name(board: &Board, net: Id) -> Result<String, BoardEditError> {
    board
        .nets
        .get(&net)
        .map(|n| n.name.clone())
        .ok_or_else(|| refused(format!("no net {net}")))
}

pub(crate) fn replace_track(id: Id, track: Track) -> Operation {
    Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id },
            Operation::AddTrack { id, track },
        ],
    }
}

pub(crate) fn shape_of(env: &Env, shape: Option<Shape>) -> Shape {
    shape.unwrap_or(if env.settings.diagonal_routes {
        Shape::Diagonal
    } else {
        Shape::Orthogonal
    })
}

/// The orders of the corner to try: the one asked for, or the setting and then the other.
pub(crate) fn corner_orders(env: &Env, asked: Option<bool>) -> Vec<bool> {
    match asked {
        Some(h) => vec![h],
        None => {
            let h = env.settings.corner_horizontal_first;
            vec![h, !h]
        }
    }
}

/// Plans an operation.
pub fn plan(
    op: &EditOp,
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
) -> Result<Plan, BoardEditError> {
    match op {
        EditOp::Place {
            id,
            refdes,
            value,
            footprint,
            at,
            rotation_udeg,
            side,
        } => place(
            board,
            ids,
            *id,
            (refdes.as_str(), value.as_str()),
            *footprint,
            *at,
            *rotation_udeg,
            *side,
        ),
        EditOp::Move {
            id,
            at,
            rotation_udeg,
            follow,
        } => {
            let old = component(board, *id)?;
            let mut changed = old.clone();
            changed.at = *at;
            if let Some(r) = rotation_udeg {
                changed.rotation = Udeg::new(*r)
                    .map_err(|e| geometry(e.to_string()))?
                    .normalised();
            }
            move_like(board, env, *id, changed, *follow)
        }
        EditOp::Rotate {
            id,
            delta_udeg,
            follow,
        } => {
            let old = component(board, *id)?;
            let turned = old
                .rotation
                .get()
                .saturating_add(*delta_udeg)
                .rem_euclid(ANGLE_LIMIT_UDEG);
            let mut changed = old.clone();
            changed.rotation = Udeg::new(turned).map_err(|e| geometry(e.to_string()))?;
            move_like(board, env, *id, changed, *follow)
        }
        EditOp::Mirror { id, follow } => {
            let old = component(board, *id)?;
            let mut changed = old.clone();
            changed.side = match old.side {
                Side::Top => Side::Bottom,
                Side::Bottom => Side::Top,
            };
            move_like(board, env, *id, changed, *follow)
        }
        EditOp::Remove { id } => {
            check_unlocked(env, *id)?;
            component(board, *id)?;
            Ok(single(Operation::RemoveComponent { id: *id }))
        }
        EditOp::Route {
            net,
            layer,
            points,
            shape,
            width_nm,
            horizontal_first,
        } => route(
            board,
            env,
            ids,
            (*net, *layer),
            points,
            shape_of(env, *shape),
            *width_nm,
            *horizontal_first,
        ),
        EditOp::RouteShove {
            net,
            layer,
            points,
            shape,
            width_nm,
            horizontal_first,
        } => shove::plan_shove(
            board,
            env,
            ids,
            &shove::ShoveRequest {
                net: *net,
                layer: *layer,
                points,
                shape: *shape,
                width_nm: *width_nm,
                horizontal_first: *horizontal_first,
            },
        )
        .map(|(plan, _)| plan),
        EditOp::RouteArc {
            track,
            vertex,
            radius_nm,
        } => arc::plan_route_arc(board, env, *track, *vertex, *radius_nm),
        EditOp::RoundCorners {
            track,
            net,
            radius_nm,
        } => arc::plan_round_corners(board, env, *track, *net, *radius_nm),
        EditOp::Via {
            net,
            at,
            from_layer,
            to_layer,
        } => via(board, env, ids, *net, *at, *from_layer, *to_layer),
        EditOp::ChangeLayer {
            track,
            vertex,
            layer,
        } => change_layer(board, env, ids, *track, *vertex, *layer),
        EditOp::DeleteSegment { track, index } => delete_segment(board, ids, *track, *index),
        EditOp::DeleteTrack { track } => {
            if !board.tracks.contains_key(track) {
                return Err(refused(format!("no track {track}")));
            }
            Ok(single(Operation::RemoveTrack { id: *track }))
        }
        EditOp::DeleteVia { via } => {
            if !board.vias.contains_key(via) {
                return Err(refused(format!("no via {via}")));
            }
            Ok(single(Operation::RemoveVia { id: *via }))
        }
        EditOp::DeleteNet { net } => delete_net(board, *net),
        EditOp::RouteBetween {
            from,
            to,
            layer,
            shape,
        } => route_between(board, env, ids, (*from, *to), *layer, shape_of(env, *shape)),
        EditOp::PasteNet {
            name,
            pads,
            layer,
            shape,
        } => paste_net(board, env, ids, name, pads, *layer, shape_of(env, *shape)),
        EditOp::TuneLength {
            net,
            target_nm,
            track,
            side,
        } => tune_request(
            board,
            env,
            &TuneRequest {
                net: *net,
                target_nm: *target_nm,
                only: *track,
                side: *side,
                tolerance_nm: env.settings.length_tolerance_nm,
                away_from: None,
            },
        ),
        EditOp::RoutePair {
            pair,
            layer,
            points,
            shape,
            width_nm,
            gap_nm,
            horizontal_first,
            layer_changes,
        } => pair_plan::route_pair(
            board,
            env,
            ids,
            &pair_plan::RoutePairRequest {
                pair,
                layer: *layer,
                points,
                shape: shape_of(env, *shape),
                width_nm: *width_nm,
                gap_nm: *gap_nm,
                horizontal_first: *horizontal_first,
                layer_changes,
            },
        ),
        EditOp::TunePair { pair, side } => pair_plan::tune_pair(board, env, pair, *side),
        EditOp::FixPairGap { pair } => pair_plan::fix_pair_gap(board, env, pair),
        EditOp::OutlineFromDxf {
            dxf,
            layer,
            join_tolerance_nm,
            settings,
        } => outline::plan_outline(
            board,
            dxf,
            layer.as_deref(),
            *join_tolerance_nm,
            settings.as_deref(),
        ),
        EditOp::Lock { .. }
        | EditOp::Unlock { .. }
        | EditOp::Group { .. }
        | EditOp::Forced { .. }
        | EditOp::ZoneAdd { .. }
        | EditOp::ZoneEdit { .. }
        | EditOp::ZoneRemove { .. }
        | EditOp::ZoneFill { .. }
        | EditOp::ZonePriority { .. }
        | EditOp::ZoneCutout { .. }
        | EditOp::PasteGroup { .. } => Err(refused(
            "this operation is planned by the zone planner, not by the track planner",
        )),
    }
}

fn component(board: &Board, id: Id) -> Result<&Component, BoardEditError> {
    board
        .components
        .get(&id)
        .ok_or_else(|| refused(format!("no component {id}")))
}

#[allow(clippy::too_many_arguments)]
fn place(
    board: &Board,
    ids: &mut IdSupply,
    id: Option<Id>,
    text: (&str, &str),
    footprint: Id,
    at: Point,
    rotation_udeg: i64,
    side: Option<Side>,
) -> Result<Plan, BoardEditError> {
    let (refdes, value) = text;
    if refdes.trim().is_empty() {
        return Err(refused("the reference designator is empty"));
    }
    if !board.footprints.contains_key(&footprint) {
        return Err(refused(format!("no footprint {footprint}")));
    }
    if board.components.values().any(|c| c.refdes == refdes) {
        return Err(refused(format!("{refdes} is already on the board")));
    }
    let id = match id {
        Some(given) => {
            if board.components.contains_key(&given) {
                return Err(refused(format!("the id {given} is already used")));
            }
            given
        }
        None => ids.next_for(board),
    };
    let rotation = Udeg::new(rotation_udeg)
        .map_err(|e| geometry(e.to_string()))?
        .normalised();
    let component = Component {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol: None,
        footprint,
        at,
        rotation,
        side: side.unwrap_or(Side::Top),
    };
    let mut plan = single(Operation::AddComponent { id, component });
    plan.created.push(id);
    Ok(plan)
}

/// What a track does when a pad it ends on moves: every end on an old place goes to the new one.
fn moved_track(track: &Track, map: &[(Point, Point)]) -> Option<Track> {
    let shift = |p: Point| {
        map.iter()
            .find(|(old, _)| *old == p)
            .map_or(p, |(_, new)| *new)
    };
    let path: Vec<Segment> = track
        .path
        .iter()
        .map(|s| match *s {
            Segment::Line { start, end } => Segment::Line {
                start: shift(start),
                end: shift(end),
            },
            Segment::Arc { start, end, sweep } => Segment::Arc {
                start: shift(start),
                end: shift(end),
                sweep,
            },
        })
        .collect();
    if path == track.path {
        return None;
    }
    let mut changed = track.clone();
    changed.path = path;
    Some(changed)
}

fn move_like(
    board: &Board,
    env: &Env,
    id: Id,
    changed: Component,
    follow: Option<bool>,
) -> Result<Plan, BoardEditError> {
    check_unlocked(env, id)?;
    let old = component(board, id)?;
    let mut ops: Vec<Operation> = Vec::new();
    if changed.side != old.side {
        // the board has no operation for the side: the component is taken off and put back with
        // the nets of its pads
        ops.push(Operation::RemoveComponent { id });
        ops.push(Operation::AddComponent {
            id,
            component: changed.clone(),
        });
        for p in board.pad_nets.iter().filter(|p| p.component == id) {
            ops.push(Operation::SetPadNet {
                pad: p.pad_ref(),
                net: Some(p.net),
            });
        }
    } else {
        ops.push(Operation::MoveComponent {
            id,
            at: changed.at,
            rotation: changed.rotation,
        });
    }
    if follow.unwrap_or(env.settings.follow_tracks) {
        let mut trial = board.clone();
        trial
            .apply(&Operation::Batch { ops: ops.clone() })
            .map_err(|e| refused(e.to_string()))?;
        let mut moves: Vec<(Id, Point, Point)> = Vec::new();
        if let Some(footprint) = board.footprints.get(&old.footprint) {
            for pad in footprint.pads.keys() {
                let pad_ref = PadRef {
                    component: id,
                    pad: *pad,
                };
                let Some(net) = board.net_of(pad_ref) else {
                    continue;
                };
                if let (Some(before), Some(after)) =
                    (pad_position(board, pad_ref), pad_position(&trial, pad_ref))
                {
                    if before != after {
                        moves.push((net, before, after));
                    }
                }
            }
        }
        for (track_id, track) in &board.tracks {
            let map: Vec<(Point, Point)> = moves
                .iter()
                .filter(|(net, _, _)| *net == track.net)
                .map(|(_, before, after)| (*before, *after))
                .collect();
            if map.is_empty() {
                continue;
            }
            if let Some(moved) = moved_track(track, &map) {
                ops.push(replace_track(*track_id, moved));
            }
        }
    }
    Ok(single(Operation::Batch { ops }))
}

#[allow(clippy::too_many_arguments)]
fn route(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    net_layer: (Id, Id),
    points: &[Point],
    shape: Shape,
    width_nm: Option<i64>,
    horizontal_first: Option<bool>,
) -> Result<Plan, BoardEditError> {
    let (net, layer) = net_layer;
    let name = net_name(board, net)?;
    copper_layer(board, layer)?;
    if points.len() < 2 {
        return Err(geometry("a route needs at least two points"));
    }
    let width = width_nm.unwrap_or_else(|| track_width(env.ctx, env.route, &name));
    if width <= 0 {
        return Err(geometry("the width of the track is not above zero"));
    }
    let width = Nm::new(width).map_err(|e| geometry(e.to_string()))?;
    let id = ids.next_for(board);
    let mut alternatives: Vec<Operation> = Vec::new();
    for h in corner_orders(env, horizontal_first) {
        let path = route_pieces(points, shape, h)?;
        if path.is_empty() {
            return Err(geometry("the route has no length"));
        }
        let op = Operation::AddTrack {
            id,
            track: Track {
                net,
                layer,
                width,
                path,
            },
        };
        if !alternatives.contains(&op) {
            alternatives.push(op);
        }
    }
    Ok(Plan {
        alternatives,
        created: vec![id],
        notes: Vec::new(),
        expect: None,
    })
}

fn via(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    net: Id,
    at: Point,
    from_layer: Option<Id>,
    to_layer: Option<Id>,
) -> Result<Plan, BoardEditError> {
    net_name(board, net)?;
    let copper = copper_layers(board);
    let from = from_layer.or_else(|| copper.first().copied());
    let to = to_layer.or_else(|| copper.last().copied());
    let (Some(from), Some(to)) = (from, to) else {
        return Err(refused("the board has no copper layer"));
    };
    if from == to {
        return Err(refused("a via needs two different layers"));
    }
    copper_layer(board, from)?;
    copper_layer(board, to)?;
    let via = route_edit::via_for_layer_change(board, env.ctx, env.route, net, at, (from, to))
        .map_err(|e| refused(e.to_string()))?;
    let id = ids.next_for(board);
    let mut plan = single(route_edit::add_via(id, via));
    plan.created.push(id);
    Ok(plan)
}

fn change_layer(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    track_id: Id,
    vertex: usize,
    layer: Id,
) -> Result<Plan, BoardEditError> {
    let track = board
        .tracks
        .get(&track_id)
        .ok_or_else(|| refused(format!("no track {track_id}")))?;
    copper_layer(board, layer)?;
    if layer == track.layer {
        return Err(refused("the track is already on that layer"));
    }
    if vertex == 0 || vertex >= track.path.len() {
        return Err(geometry(format!(
            "the vertex must be between 1 and {}",
            track.path.len().saturating_sub(1)
        )));
    }
    let at = track.path[vertex].start();
    let via = route_edit::via_for_layer_change(
        board,
        env.ctx,
        env.route,
        track.net,
        at,
        (track.layer, layer),
    )
    .map_err(|e| refused(e.to_string()))?;
    let mut first = track.clone();
    first.path.truncate(vertex);
    let mut second = track.clone();
    second.layer = layer;
    second.path = track.path[vertex..].to_vec();
    let second_id = ids.next_for(board);
    let via_id = ids.next_for(board);
    let mut plan = single(Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id: track_id },
            Operation::AddTrack {
                id: track_id,
                track: first,
            },
            Operation::AddTrack {
                id: second_id,
                track: second,
            },
            Operation::AddVia { id: via_id, via },
        ],
    });
    plan.created = vec![second_id, via_id];
    Ok(plan)
}

fn delete_segment(
    board: &Board,
    ids: &mut IdSupply,
    track_id: Id,
    index: usize,
) -> Result<Plan, BoardEditError> {
    let track = board
        .tracks
        .get(&track_id)
        .ok_or_else(|| refused(format!("no track {track_id}")))?;
    if index >= track.path.len() {
        return Err(geometry(format!(
            "the track has {} pieces, there is no piece {index}",
            track.path.len()
        )));
    }
    let mut ops = vec![Operation::RemoveTrack { id: track_id }];
    let mut created = Vec::new();
    if index > 0 {
        let mut left = track.clone();
        left.path.truncate(index);
        ops.push(Operation::AddTrack {
            id: track_id,
            track: left,
        });
    }
    if index + 1 < track.path.len() {
        let mut right = track.clone();
        right.path = track.path[index + 1..].to_vec();
        let right_id = if index > 0 {
            let fresh = ids.next_for(board);
            created.push(fresh);
            fresh
        } else {
            track_id
        };
        ops.push(Operation::AddTrack {
            id: right_id,
            track: right,
        });
    }
    let mut plan = single(Operation::Batch { ops });
    plan.created = created;
    Ok(plan)
}

fn delete_net(board: &Board, net: Id) -> Result<Plan, BoardEditError> {
    net_name(board, net)?;
    let mut ops: Vec<Operation> = Vec::new();
    for (id, track) in &board.tracks {
        if track.net == net {
            ops.push(Operation::RemoveTrack { id: *id });
        }
    }
    for (id, via) in &board.vias {
        if via.net == net {
            ops.push(Operation::RemoveVia { id: *id });
        }
    }
    if ops.is_empty() {
        return Err(refused("the net has no track and no via"));
    }
    Ok(single(Operation::Batch { ops }))
}

/// The net that joins two pads, and the assignments the pads still need.
fn net_for_pair(
    board: &Board,
    from: PadRef,
    to: PadRef,
) -> Result<(Id, Vec<Operation>), BoardEditError> {
    match (board.net_of(from), board.net_of(to)) {
        (Some(a), Some(b)) if a != b => Err(refused("the pads are on different nets")),
        (Some(a), Some(_)) => Ok((a, Vec::new())),
        (Some(a), None) => Ok((
            a,
            vec![Operation::SetPadNet {
                pad: to,
                net: Some(a),
            }],
        )),
        (None, Some(b)) => Ok((
            b,
            vec![Operation::SetPadNet {
                pad: from,
                net: Some(b),
            }],
        )),
        (None, None) => Err(refused("neither pad has a net: paste a net first")),
    }
}

fn pad_at(board: &Board, pad: PadRef) -> Result<Point, BoardEditError> {
    pad_position(board, pad).ok_or_else(|| refused("a pad is not on the board"))
}

fn route_between(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    pads: (PadRef, PadRef),
    layer: Id,
    shape: Shape,
) -> Result<Plan, BoardEditError> {
    let (from, to) = pads;
    copper_layer(board, layer)?;
    if from == to {
        return Err(refused("the two pads are the same pad"));
    }
    let (a, b) = (pad_at(board, from)?, pad_at(board, to)?);
    let (net, pre) = net_for_pair(board, from, to)?;
    let name = net_name(board, net)?;
    let width =
        Nm::new(track_width(env.ctx, env.route, &name)).map_err(|e| geometry(e.to_string()))?;
    let id = ids.next_for(board);
    let mut alternatives: Vec<Operation> = Vec::new();
    for h in corner_orders(env, None) {
        let path = lines_through(&legs(a, b, shape, h)?);
        if path.is_empty() {
            return Err(geometry("the two pads are at the same place"));
        }
        let mut ops = pre.clone();
        ops.push(Operation::AddTrack {
            id,
            track: Track {
                net,
                layer,
                width,
                path,
            },
        });
        let op = Operation::Batch { ops };
        if !alternatives.contains(&op) {
            alternatives.push(op);
        }
    }
    Ok(Plan {
        alternatives,
        created: vec![id],
        notes: Vec::new(),
        expect: None,
    })
}

fn paste_net(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    name: &str,
    pads: &[PadRef],
    layer: Id,
    shape: Shape,
) -> Result<Plan, BoardEditError> {
    copper_layer(board, layer)?;
    if name.trim().is_empty() {
        return Err(refused("the name of the net is empty"));
    }
    if pads.len() < 2 {
        return Err(refused("a net needs at least two pads"));
    }
    for (i, pad) in pads.iter().enumerate() {
        if pads[..i].contains(pad) {
            return Err(refused("a pad is named twice"));
        }
    }
    let mut points: Vec<Point> = Vec::new();
    for pad in pads {
        points.push(pad_at(board, *pad)?);
    }
    let existing = board
        .nets
        .iter()
        .find(|(_, n)| n.name == name)
        .map(|(id, _)| *id);
    let net = existing.unwrap_or_else(|| ids.next_for(board));
    let mut pre: Vec<Operation> = Vec::new();
    let mut created: Vec<Id> = Vec::new();
    if existing.is_none() {
        pre.push(Operation::AddNet {
            id: net,
            name: name.to_owned(),
        });
        created.push(net);
    }
    for pad in pads {
        match board.net_of(*pad) {
            Some(other) if other != net => {
                return Err(refused("a pad is already on another net"));
            }
            Some(_) => {}
            None => pre.push(Operation::SetPadNet {
                pad: *pad,
                net: Some(net),
            }),
        }
    }
    let width =
        Nm::new(track_width(env.ctx, env.route, name)).map_err(|e| geometry(e.to_string()))?;
    let track_ids: Vec<Id> = (1..pads.len()).map(|_| ids.next_for(board)).collect();
    let mut alternatives: Vec<Operation> = Vec::new();
    for h in corner_orders(env, None) {
        let mut ops = pre.clone();
        for (i, pair) in points.windows(2).enumerate() {
            let path = lines_through(&legs(pair[0], pair[1], shape, h)?);
            if path.is_empty() {
                return Err(geometry("two pads are at the same place"));
            }
            ops.push(Operation::AddTrack {
                id: track_ids[i],
                track: Track {
                    net,
                    layer,
                    width,
                    path,
                },
            });
        }
        let op = Operation::Batch { ops };
        if !alternatives.contains(&op) {
            alternatives.push(op);
        }
    }
    created.extend(track_ids);
    Ok(Plan {
        alternatives,
        created,
        notes: Vec::new(),
        expect: None,
    })
}

/// A straight piece that a meander can replace.
struct Piece {
    track: Id,
    index: usize,
    start: Point,
    end: Point,
    len: i64,
}

/// What a tuning asks for.
#[derive(Clone, Copy)]
pub(crate) struct TuneRequest {
    /// The net to make longer.
    pub net: Id,
    /// The length asked for, in nanometres.
    pub target_nm: i64,
    /// The only track to put meanders on.
    pub only: Option<Id>,
    /// The side of the meanders.
    pub side: MeanderSide,
    /// How far from the target the length may end, in nanometres.
    pub tolerance_nm: i64,
    /// A net that the meanders bend away from when the side is left to the editor.
    pub away_from: Option<Id>,
}

/// The side (1 left, -1 right) of a straight piece that faces away from a net: the opposite of the
/// side of the nearest piece of that net, to the middle of the piece.
fn away_side(board: &Board, start: Point, end: Point, from: Id) -> i64 {
    let mid = (
        i128::from(start.x_nm.get() + end.x_nm.get()) / 2,
        i128::from(start.y_nm.get() + end.y_nm.get()) / 2,
    );
    let mut best: Option<(i128, (i128, i128))> = None;
    for track in board.tracks.values().filter(|t| t.net == from) {
        for segment in &track.path {
            let (a, b) = (segment.start(), segment.end());
            let m = (
                i128::from(a.x_nm.get() + b.x_nm.get()) / 2,
                i128::from(a.y_nm.get() + b.y_nm.get()) / 2,
            );
            let d = (m.0 - mid.0) * (m.0 - mid.0) + (m.1 - mid.1) * (m.1 - mid.1);
            if best.is_none_or(|(least, _)| d < least) {
                best = Some((d, m));
            }
        }
    }
    let Some((_, m)) = best else {
        return 1;
    };
    let ux = i128::from((end.x_nm.get() - start.x_nm.get()).signum());
    let uy = i128::from((end.y_nm.get() - start.y_nm.get()).signum());
    let cross = ux * (m.1 - mid.1) - uy * (m.0 - mid.0);
    if cross > 0 {
        -1
    } else {
        1
    }
}

pub(crate) fn tune_request(
    board: &Board,
    env: &Env,
    request: &TuneRequest,
) -> Result<Plan, BoardEditError> {
    let TuneRequest {
        net,
        target_nm,
        only,
        side,
        tolerance_nm: tolerance,
        away_from,
    } = *request;
    let impossible = |what: String| BoardEditError::new(TUNE_IMPOSSIBLE, what);
    net_name(board, net)?;
    if target_nm <= 0 {
        return Err(impossible(
            "the length asked for is not above zero".to_owned(),
        ));
    }
    let (lengths, _) = measure_nets(board, &LengthSettings::default(), None)
        .map_err(|e| impossible(e.message(Lang::En)))?;
    let current = lengths
        .iter()
        .find(|n| n.net == net)
        .map(|n| n.total_nm)
        .ok_or_else(|| impossible("the net has no copper to measure".to_owned()))?;
    let extra = target_nm - current;
    if extra.abs() <= tolerance {
        let mut plan = single(Operation::Batch { ops: Vec::new() });
        plan.notes.push(format!(
            "the net is {current} nm long, within the tolerance"
        ));
        return Ok(plan);
    }
    if extra < 0 {
        return Err(impossible(format!(
            "the net is {} nm longer than the target",
            -extra
        )));
    }
    if let Some(track) = only {
        match board.tracks.get(&track) {
            Some(t) if t.net == net => {}
            Some(_) => return Err(refused("the track is on another net")),
            None => return Err(refused(format!("no track {track}"))),
        }
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for (track_id, track) in &board.tracks {
        if track.net != net || only.is_some_and(|t| t != *track_id) {
            continue;
        }
        for (index, segment) in track.path.iter().enumerate() {
            if let Segment::Line { start, end } = *segment {
                if let Some(len) = piece_length(start, end) {
                    pieces.push(Piece {
                        track: *track_id,
                        index,
                        start,
                        end,
                        len,
                    });
                }
            }
        }
    }
    pieces.sort_by(|a, b| {
        b.len
            .cmp(&a.len)
            .then(a.track.cmp(&b.track))
            .then(a.index.cmp(&b.index))
    });
    pieces.truncate(env.settings.meander_candidates);
    if pieces.is_empty() {
        return Err(impossible(
            "the net has no horizontal, vertical or 45-degree piece to put meanders on".to_owned(),
        ));
    }
    let fixed_sides: &[i64] = match side {
        MeanderSide::Auto => &[1, -1],
        MeanderSide::Left => &[1],
        MeanderSide::Right => &[-1],
    };
    let tuning = Tuning {
        extra_nm: extra,
        amplitude_nm: env.settings.meander_amplitude_nm,
        max_amplitude_nm: env.settings.meander_max_amplitude_nm,
        pitch_nm: env.settings.meander_pitch_nm,
    };
    let mut alternatives: Vec<Operation> = Vec::new();
    let mut first_error: Option<BoardEditError> = None;
    for piece in &pieces {
        let Some(track) = board.tracks.get(&piece.track) else {
            continue;
        };
        let sides: Vec<i64> = match (side, away_from) {
            (MeanderSide::Auto, Some(other)) => {
                let first = away_side(board, piece.start, piece.end, other);
                vec![first, -first]
            }
            _ => fixed_sides.to_vec(),
        };
        for s in &sides {
            match serpentine(piece.start, piece.end, tuning, *s) {
                Ok((segments, _)) => {
                    let mut changed = track.clone();
                    let mut path = track.path[..piece.index].to_vec();
                    path.extend(segments);
                    path.extend_from_slice(&track.path[piece.index + 1..]);
                    changed.path = path;
                    alternatives.push(replace_track(piece.track, changed));
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
    }
    if alternatives.is_empty() {
        return Err(first_error.unwrap_or_else(|| impossible("no place for meanders".to_owned())));
    }
    Ok(Plan {
        alternatives,
        created: Vec::new(),
        notes: Vec::new(),
        expect: Some(Expect {
            net,
            target_nm,
            tolerance_nm: tolerance,
            before_nm: current,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;

    fn env_parts() -> (RuleContext, RouteSettings, BoardEditSettings, BTreeSet<Id>) {
        let settings = BoardEditSettings::default();
        (
            RuleContext::default(),
            settings.route_settings(),
            settings,
            BTreeSet::new(),
        )
    }

    #[test]
    fn a_place_makes_a_component_with_a_new_id_and_refuses_a_second_refdes() {
        let f = fixture();
        let (ctx, route, settings, locked) = env_parts();
        let pairs = PairConfig::default();
        let env = Env {
            ctx: &ctx,
            route: &route,
            settings: &settings,
            locked: &locked,
            pairs: &pairs,
        };
        let mut ids = IdSupply::new().unwrap();
        let op = EditOp::Place {
            id: None,
            refdes: "R9".into(),
            value: "1k".into(),
            footprint: f.footprint,
            at: f.pt(1, 1),
            rotation_udeg: 0,
            side: None,
        };
        let plan = plan(&op, &f.board, &env, &mut ids).unwrap();
        assert_eq!(plan.created.len(), 1);
        assert_eq!(plan.alternatives.len(), 1);
        let taken = EditOp::Place {
            id: None,
            refdes: "R1".into(),
            value: String::new(),
            footprint: f.footprint,
            at: f.pt(1, 1),
            rotation_udeg: 0,
            side: None,
        };
        let error = super::plan(&taken, &f.board, &env, &mut ids).unwrap_err();
        assert_eq!(error.code, OPERATION_REFUSED);
    }

    #[test]
    fn a_route_has_both_orders_of_the_corner_unless_one_is_asked_for() {
        let f = fixture();
        let (ctx, route_settings, settings, locked) = env_parts();
        let pairs = PairConfig::default();
        let env = Env {
            ctx: &ctx,
            route: &route_settings,
            settings: &settings,
            locked: &locked,
            pairs: &pairs,
        };
        let mut ids = IdSupply::new().unwrap();
        let mut op = EditOp::Route {
            net: f.sig,
            layer: f.top,
            points: vec![f.pt(0, 0), f.pt(4, 3)],
            shape: Some(Shape::Orthogonal),
            width_nm: None,
            horizontal_first: None,
        };
        assert_eq!(
            plan(&op, &f.board, &env, &mut ids)
                .unwrap()
                .alternatives
                .len(),
            2
        );
        if let EditOp::Route {
            horizontal_first, ..
        } = &mut op
        {
            *horizontal_first = Some(true);
        }
        assert_eq!(
            plan(&op, &f.board, &env, &mut ids)
                .unwrap()
                .alternatives
                .len(),
            1
        );
    }

    #[test]
    fn a_locked_component_is_not_moved() {
        let f = fixture();
        let (ctx, route, settings, mut locked) = env_parts();
        locked.insert(f.r1);
        let pairs = PairConfig::default();
        let env = Env {
            ctx: &ctx,
            route: &route,
            settings: &settings,
            locked: &locked,
            pairs: &pairs,
        };
        let mut ids = IdSupply::new().unwrap();
        let op = EditOp::Move {
            id: f.r1,
            at: f.pt(5, 5),
            rotation_udeg: None,
            follow: None,
        };
        let error = plan(&op, &f.board, &env, &mut ids).unwrap_err();
        assert_eq!(error.code, COMPONENT_LOCKED);
    }
}
