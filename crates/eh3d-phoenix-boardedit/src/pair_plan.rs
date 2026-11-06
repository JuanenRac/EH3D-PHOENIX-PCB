// SPDX-License-Identifier: GPL-3.0-or-later
//! The planner of the operations on a differential pair: the two tracks drawn side by side along a
//! centre line (`route_pair`), the shorter net made as long as the other with meanders
//! (`tune_pair`) and the pieces whose gap is out of tolerance moved to the gap of the pair
//! (`fix_pair_gap`). Like the rest of the planner it reads the board and changes nothing; the
//! editor tries the plan on a copy and checks it with the design rules before it is kept.

use eh3d_phoenix_board::{Board, Id, Operation, Segment, Track};
use eh3d_phoenix_lengths::{measure_nets, LengthSettings};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::edit as route_edit;
use eh3d_phoenix_rules::Lang;

use crate::codes::{BoardEditError, TUNE_IMPOSSIBLE};
use crate::ids::IdSupply;
use crate::ops::PairLayerChange;
use crate::pair_codes::PAIR_OPERATION_REFUSED;
use crate::pair_geom::{direction, offset_path};
use crate::pairs::{resolve, ResolvedPair};
use crate::plan::{
    copper_layer, corner_orders, geometry, refused, replace_track, single, tune_request, Env, Plan,
    TuneRequest,
};
use crate::shape::{lines_through, pt, route_pieces, MeanderSide, Shape};

fn pair_refused(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(PAIR_OPERATION_REFUSED, what)
}

/// The pair with a name, with its values filled in and its nets found.
fn find_pair(board: &Board, env: &Env, name: &str) -> Result<ResolvedPair, BoardEditError> {
    let def = env
        .pairs
        .find(name)
        .ok_or_else(|| pair_refused(format!("there is no pair named {name}")))?;
    resolve(board, def, &env.pairs.settings)
}

/// What `route_pair` asks for.
pub(crate) struct RoutePairRequest<'a> {
    /// The name of the pair.
    pub pair: &'a str,
    /// The copper layer of the first part.
    pub layer: Id,
    /// The points of the centre line.
    pub points: &'a [Point],
    /// The shape of the centre line.
    pub shape: Shape,
    /// The width of the tracks, or the one of the pair.
    pub width_nm: Option<i64>,
    /// The gap between the edges, or the one of the pair.
    pub gap_nm: Option<i64>,
    /// Whether a corner goes first along x.
    pub horizontal_first: Option<bool>,
    /// The changes of layer.
    pub layer_changes: &'a [PairLayerChange],
}

/// The distances of the two paths from the centre line, the width and the layers of the runs.
struct Layout<'a> {
    width: Nm,
    near_nm: i64,
    far_nm: i64,
    layers: &'a [Id],
    changes: &'a [PairLayerChange],
}

/// The ids a pair route uses: the tracks (positive net first) and then the vias.
struct RouteIds {
    tracks: Vec<Id>,
    vias: Vec<Id>,
}

/// The operations of a pair along the points of a centre line.
fn pair_operations(
    board: &Board,
    env: &Env,
    pair: &ResolvedPair,
    vertices: &[Point],
    layout: &Layout,
    ids: &RouteIds,
) -> Result<Vec<Operation>, BoardEditError> {
    let positive = offset_path(vertices, layout.near_nm)?;
    let mut negative = offset_path(vertices, -layout.far_nm)?;
    let stagger = env.pairs.settings.via_stagger_nm;
    for change in layout.changes {
        let v = change.vertex;
        let before = direction(vertices[v - 1], vertices[v]);
        let after = direction(vertices[v], vertices[v + 1]);
        let Some(dir) = before.filter(|d| Some(*d) == after) else {
            return Err(geometry(
                "a change of layer must be inside a straight piece of the pair",
            ));
        };
        if stagger > 0 {
            let moved = pt(
                negative[v].x_nm.get() + dir.0 * stagger,
                negative[v].y_nm.get() + dir.1 * stagger,
            )?;
            if direction(negative[v - 1], moved) != Some(dir)
                || direction(moved, negative[v + 1]) != Some(dir)
            {
                return Err(geometry(
                    "the piece is too short to move the via of the second net",
                ));
            }
            negative[v] = moved;
        }
    }
    let mut bounds: Vec<usize> = vec![0];
    bounds.extend(layout.changes.iter().map(|c| c.vertex));
    bounds.push(vertices.len() - 1);
    let runs = layout.layers.len();
    let nets = [(pair.positive, &positive), (pair.negative, &negative)];
    let mut ops: Vec<Operation> = Vec::new();
    for (which, (net, path)) in nets.iter().enumerate() {
        for (run, layer) in layout.layers.iter().enumerate() {
            let pieces = lines_through(&path[bounds[run]..=bounds[run + 1]]);
            if pieces.is_empty() {
                return Err(geometry("a part of the pair has no length"));
            }
            ops.push(Operation::AddTrack {
                id: ids.tracks[which * runs + run],
                track: Track {
                    net: *net,
                    layer: *layer,
                    width: layout.width,
                    path: pieces,
                },
            });
        }
    }
    for (k, change) in layout.changes.iter().enumerate() {
        for (which, (net, path)) in nets.iter().enumerate() {
            let via = route_edit::via_for_layer_change(
                board,
                env.ctx,
                env.route,
                *net,
                path[change.vertex],
                (layout.layers[k], layout.layers[k + 1]),
            )
            .map_err(|e| refused(e.to_string()))?;
            ops.push(route_edit::add_via(ids.vias[k * 2 + which], via));
        }
    }
    Ok(ops)
}

/// Plans two tracks side by side along a centre line. The positive net runs on the left of the
/// direction of the centre line and the negative net on the right; where the pair changes layer
/// each net gets a via, the one of the negative net a little further along the pair.
pub(crate) fn route_pair(
    board: &Board,
    env: &Env,
    ids: &mut IdSupply,
    request: &RoutePairRequest,
) -> Result<Plan, BoardEditError> {
    let pair = find_pair(board, env, request.pair)?;
    copper_layer(board, request.layer)?;
    if request.points.len() < 2 {
        return Err(geometry("a route needs at least two points"));
    }
    let width = request.width_nm.unwrap_or(pair.width_nm);
    let gap = request.gap_nm.unwrap_or(pair.gap_nm);
    if width <= 0 || gap <= 0 {
        return Err(geometry(
            "the width and the gap of the pair must be above zero",
        ));
    }
    let pitch = width
        .checked_add(gap)
        .ok_or_else(|| geometry("the width and the gap of the pair are too large"))?;
    let width = Nm::new(width).map_err(|e| geometry(e.to_string()))?;
    let near_nm = pitch / 2;
    let far_nm = pitch - near_nm;

    let mut layers: Vec<Id> = vec![request.layer];
    let mut last_vertex = 0usize;
    for change in request.layer_changes {
        if change.vertex <= last_vertex {
            return Err(geometry(
                "the vertices of the changes of layer must be above zero and grow",
            ));
        }
        copper_layer(board, change.layer)?;
        if layers.last() == Some(&change.layer) {
            return Err(refused("a change of layer must go to another layer"));
        }
        layers.push(change.layer);
        last_vertex = change.vertex;
    }
    let runs = layers.len();
    let route_ids = RouteIds {
        tracks: (0..runs * 2).map(|_| ids.next_for(board)).collect(),
        vias: (0..(runs - 1) * 2).map(|_| ids.next_for(board)).collect(),
    };
    let layout = Layout {
        width,
        near_nm,
        far_nm,
        layers: &layers,
        changes: request.layer_changes,
    };

    let mut alternatives: Vec<Operation> = Vec::new();
    let mut first_error: Option<BoardEditError> = None;
    for h in corner_orders(env, request.horizontal_first) {
        let centre = route_pieces(request.points, request.shape, h)?;
        if centre.is_empty() {
            return Err(geometry("the route has no length"));
        }
        let mut vertices: Vec<Point> = vec![centre[0].start()];
        vertices.extend(centre.iter().map(Segment::end));
        if let Some(last) = request.layer_changes.last() {
            if last.vertex >= vertices.len() - 1 {
                return Err(geometry(format!(
                    "the vertex of a change of layer must be between 1 and {}",
                    vertices.len() - 2
                )));
            }
        }
        match pair_operations(board, env, &pair, &vertices, &layout, &route_ids) {
            Ok(ops) => {
                let op = Operation::Batch { ops };
                if !alternatives.contains(&op) {
                    alternatives.push(op);
                }
            }
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    if alternatives.is_empty() {
        return Err(first_error.unwrap_or_else(|| geometry("the pair cannot be drawn")));
    }
    let mut created = route_ids.tracks;
    created.extend(route_ids.vias);
    Ok(Plan {
        alternatives,
        created,
        notes: Vec::new(),
        expect: None,
    })
}

/// Plans meanders on the shorter net of a pair until the two lengths are within the tolerance of
/// the pair. Unless a side is asked for, the meanders bend away from the other net.
pub(crate) fn tune_pair(
    board: &Board,
    env: &Env,
    name: &str,
    side: MeanderSide,
) -> Result<Plan, BoardEditError> {
    let pair = find_pair(board, env, name)?;
    let impossible = |what: String| BoardEditError::new(TUNE_IMPOSSIBLE, what);
    let (lengths, _) = measure_nets(board, &LengthSettings::default(), None)
        .map_err(|e| impossible(e.message(Lang::En)))?;
    let length_of = |net: Id| lengths.iter().find(|n| n.net == net).map(|n| n.total_nm);
    let (Some(positive), Some(negative)) = (length_of(pair.positive), length_of(pair.negative))
    else {
        return Err(impossible(
            "a net of the pair has no copper to measure".to_owned(),
        ));
    };
    let skew = (positive - negative).abs();
    if skew <= pair.length_tolerance_nm {
        let mut plan = single(Operation::Batch { ops: Vec::new() });
        plan.notes.push(format!(
            "the nets of the pair differ by {skew} nm, within the tolerance"
        ));
        return Ok(plan);
    }
    let (short, long, target) = if positive < negative {
        (pair.positive, pair.negative, negative)
    } else {
        (pair.negative, pair.positive, positive)
    };
    let mut plan = tune_request(
        board,
        env,
        &TuneRequest {
            net: short,
            target_nm: target,
            only: None,
            side,
            tolerance_nm: pair.length_tolerance_nm,
            away_from: Some(long),
        },
    )?;
    plan.notes.push(format!(
        "the nets of the pair differed by {skew} nm, the tolerance is {} nm",
        pair.length_tolerance_nm
    ));
    Ok(plan)
}

/// A straight horizontal or vertical piece of a track.
struct Flat {
    layer: Id,
    width: i64,
    horizontal: bool,
    perp: i64,
    lo: i64,
    hi: i64,
}

/// For a horizontal or vertical line piece: whether it is horizontal, the coordinate across it and
/// the range along it.
fn flat_of(segment: &Segment) -> Option<(bool, i64, i64, i64)> {
    let Segment::Line { start, end } = *segment else {
        return None;
    };
    let (x0, y0) = (start.x_nm.get(), start.y_nm.get());
    let (x1, y1) = (end.x_nm.get(), end.y_nm.get());
    if y0 == y1 && x0 != x1 {
        Some((true, y0, x0.min(x1), x0.max(x1)))
    } else if x0 == x1 && y0 != y1 {
        Some((false, x0, y0.min(y1), y0.max(y1)))
    } else {
        None
    }
}

fn flats_of(board: &Board, net: Id) -> Vec<Flat> {
    let mut out = Vec::new();
    for track in board.tracks.values().filter(|t| t.net == net) {
        for segment in &track.path {
            if let Some((horizontal, perp, lo, hi)) = flat_of(segment) {
                out.push(Flat {
                    layer: track.layer,
                    width: track.width.get(),
                    horizontal,
                    perp,
                    lo,
                    hi,
                });
            }
        }
    }
    out
}

/// The gap that is asked for and how it is judged, in nanometres.
#[derive(Clone, Copy)]
struct Want {
    gap_nm: i64,
    tolerance_nm: i64,
    range_nm: i64,
}

/// What can be done with a piece of the negative net.
enum Verdict {
    /// Not a piece that runs coupled.
    Skip,
    /// A coupled piece with the gap asked for.
    Fine,
    /// A coupled piece with the wrong gap that cannot be moved.
    Cannot,
    /// The three pieces (before, the piece, after) as they are after the move.
    Move([Segment; 3]),
}

fn judge_piece(
    path: &[Segment],
    i: usize,
    layer: Id,
    width: i64,
    positive: &[Flat],
    want: Want,
) -> Verdict {
    let Some((horizontal, perp, lo, hi)) = flat_of(&path[i]) else {
        return Verdict::Skip;
    };
    let mut best: Option<(i64, &Flat)> = None;
    for f in positive
        .iter()
        .filter(|f| f.layer == layer && f.horizontal == horizontal)
    {
        if hi.min(f.hi) - lo.max(f.lo) <= 0 {
            continue;
        }
        // twice the gap between the edges, to stay in whole nanometres
        let twice = 2 * (perp - f.perp).abs() - (f.width + width);
        if best.is_none_or(|(least, _)| twice < least) {
            best = Some((twice, f));
        }
    }
    let Some((twice, f)) = best else {
        return Verdict::Skip;
    };
    if twice > 2 * want.range_nm {
        return Verdict::Skip;
    }
    if (twice - 2 * want.gap_nm).abs() <= 2 * want.tolerance_nm {
        return Verdict::Fine;
    }
    if i == 0 || i + 1 >= path.len() {
        return Verdict::Cannot;
    }
    let (Some(before), Some(after)) = (flat_of(&path[i - 1]), flat_of(&path[i + 1])) else {
        return Verdict::Cannot;
    };
    if before.0 == horizontal || after.0 == horizontal {
        return Verdict::Cannot;
    }
    let sign = if perp >= f.perp { 1 } else { -1 };
    let new_perp = f.perp + sign * (want.gap_nm + (f.width + width) / 2);
    let delta = new_perp - perp;
    let moved = |p: Point| -> Option<Point> {
        if horizontal {
            Point::from_nm(p.x_nm.get(), p.y_nm.get() + delta).ok()
        } else {
            Point::from_nm(p.x_nm.get() + delta, p.y_nm.get()).ok()
        }
    };
    let (prev_start, prev_end) = (path[i - 1].start(), path[i - 1].end());
    let (next_start, next_end) = (path[i + 1].start(), path[i + 1].end());
    let (Some(piece_start), Some(piece_end), Some(prev_end_new), Some(next_start_new)) = (
        moved(path[i].start()),
        moved(path[i].end()),
        moved(prev_end),
        moved(next_start),
    ) else {
        return Verdict::Cannot;
    };
    if direction(prev_start, prev_end_new) != direction(prev_start, prev_end)
        || direction(next_start_new, next_end) != direction(next_start, next_end)
    {
        return Verdict::Cannot;
    }
    Verdict::Move([
        Segment::Line {
            start: prev_start,
            end: prev_end_new,
        },
        Segment::Line {
            start: piece_start,
            end: piece_end,
        },
        Segment::Line {
            start: next_start_new,
            end: next_end,
        },
    ])
}

/// Plans the move of the horizontal and vertical pieces of the negative net that run coupled to the
/// positive net with a gap out of tolerance. A piece moves across, to the gap of the pair, and the
/// two pieces at its ends (which must be perpendicular to it) grow or shrink with it; a piece at
/// the end of a track, with a neighbour at another angle or too short to follow, is not moved.
pub(crate) fn fix_pair_gap(board: &Board, env: &Env, name: &str) -> Result<Plan, BoardEditError> {
    let pair = find_pair(board, env, name)?;
    let settings = &env.pairs.settings;
    let want = Want {
        gap_nm: pair.gap_nm,
        tolerance_nm: settings.gap_tolerance_nm,
        range_nm: settings.coupling_range_nm,
    };
    let positive = flats_of(board, pair.positive);
    let mut wrong = 0usize;
    let mut moved = 0usize;
    let mut ops: Vec<Operation> = Vec::new();
    for (track_id, track) in board.tracks.iter().filter(|(_, t)| t.net == pair.negative) {
        let mut path = track.path.clone();
        let mut changed = false;
        let mut i = 0usize;
        while i < path.len() {
            match judge_piece(&path, i, track.layer, track.width.get(), &positive, want) {
                Verdict::Skip | Verdict::Fine => {}
                Verdict::Cannot => wrong += 1,
                Verdict::Move([before, piece, after]) => {
                    wrong += 1;
                    moved += 1;
                    changed = true;
                    path[i - 1] = before;
                    path[i] = piece;
                    path[i + 1] = after;
                }
            }
            i += 1;
        }
        if changed {
            let mut changed_track = track.clone();
            changed_track.path = path;
            ops.push(replace_track(*track_id, changed_track));
        }
    }
    if wrong == 0 {
        let mut plan = single(Operation::Batch { ops: Vec::new() });
        plan.notes.push(
            "the gap is within the tolerance on every straight piece that runs coupled".to_owned(),
        );
        return Ok(plan);
    }
    if moved == 0 {
        return Err(pair_refused(format!(
            "{wrong} pieces have a gap out of tolerance and none of them can be moved"
        )));
    }
    let mut plan = single(Operation::Batch { ops });
    plan.notes.push(format!(
        "{moved} of {wrong} pieces with a wrong gap were moved"
    ));
    Ok(plan)
}
