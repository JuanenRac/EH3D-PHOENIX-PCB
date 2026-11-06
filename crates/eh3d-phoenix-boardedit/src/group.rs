// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations on a group of objects: move, delete and align (and distribute) the objects of a
//! selection ([`crate::selection`]), each as one step of the history. The planner makes one
//! `Operation::Batch` of the board with its exact inverse, so undo gives back the same bytes.
//!
//! * `move_items` moves each object by its own offset; `move_selection` moves the selection by
//!   one offset that snaps to the grid of the settings (the offset is rounded to a multiple of
//!   the grid, halves upwards) unless snapping is turned off for the operation. A component keeps
//!   its rotation; a track, a silkscreen item and a via are drawn again at the new place with the
//!   same id; a zone is moved by the zone editor with its cutouts, and its fill is dropped (it
//!   has to be filled again). The tracks that are not selected do not follow a component: select
//!   the net with `{"select": "net"}` to carry the copper along.
//! * `delete_items` and `delete_selection` remove the objects (a component with the nets of its
//!   pads, a zone with its properties and fill).
//! * `align_selection` lines the extents of the objects up with the edges or the centre of the
//!   box that holds them all (left, right, top and bottom, y pointing up), or spreads three or
//!   more objects so that the gaps between their extents are equal along x or along y; the
//!   first and the last (by centre) stay where they are.
//!
//! A locked component refuses the whole operation (`PHOENIX-BEDIT-006`), an object that the board
//! does not have is `PHOENIX-BEDIT-010`, an empty selection is `PHOENIX-BEDIT-009` and too few
//! objects to align are `PHOENIX-BEDIT-011`; the board is left as it was.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Operation, Segment, SilkItem};
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::codes::{BoardEditError, ALIGN_TOO_FEW, ITEM_MISSING, SELECTION_EMPTY};
use crate::ops::EditOp;
use crate::plan::{check_unlocked, geometry, refused, replace_track, single, Env, Plan};
use crate::selection::{item_bounds, Bounds, Item, Selection};
use crate::settings::BoardEditSettings;
use crate::zone::{ZoneChanges, ZoneData};

/// An object and how far it goes, in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemMove {
    /// The object.
    pub item: Item,
    /// The offset along x.
    pub dx_nm: i64,
    /// The offset along y (up is positive).
    pub dy_nm: i64,
}

/// How `align_selection` lines the objects up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignMode {
    /// The smallest x of each object goes to the smallest x of the group.
    Left,
    /// The largest x of each object goes to the largest x of the group.
    Right,
    /// The largest y of each object goes to the largest y of the group.
    Top,
    /// The smallest y of each object goes to the smallest y of the group.
    Bottom,
    /// The centre of each object goes to the centre of the group along x, to within a nanometre.
    CenterX,
    /// The centre of each object goes to the centre of the group along y, to within a nanometre.
    CenterY,
    /// Equal gaps between the extents along x (three objects or more).
    DistributeX,
    /// Equal gaps between the extents along y (three objects or more).
    DistributeY,
}

fn missing(item: Item) -> BoardEditError {
    BoardEditError::new(ITEM_MISSING, format!("the board has no {item:?}"))
}

/// A point moved by an offset.
fn shift(p: Point, dx: i64, dy: i64) -> Result<Point, BoardEditError> {
    let x = p
        .x_nm
        .get()
        .checked_add(dx)
        .ok_or_else(|| geometry("the offset is out of range"))?;
    let y = p
        .y_nm
        .get()
        .checked_add(dy)
        .ok_or_else(|| geometry("the offset is out of range"))?;
    Point::from_nm(x, y).map_err(|e| geometry(e.to_string()))
}

fn shift_segment(piece: &Segment, dx: i64, dy: i64) -> Result<Segment, BoardEditError> {
    Ok(match piece {
        Segment::Line { start, end } => Segment::Line {
            start: shift(*start, dx, dy)?,
            end: shift(*end, dx, dy)?,
        },
        Segment::Arc { start, end, sweep } => Segment::Arc {
            start: shift(*start, dx, dy)?,
            end: shift(*end, dx, dy)?,
            sweep: *sweep,
        },
    })
}

fn shift_path(path: &[Segment], dx: i64, dy: i64) -> Result<Vec<Segment>, BoardEditError> {
    path.iter().map(|p| shift_segment(p, dx, dy)).collect()
}

fn shift_points(points: &[Point], dx: i64, dy: i64) -> Result<Vec<Point>, BoardEditError> {
    points.iter().map(|p| shift(*p, dx, dy)).collect()
}

/// A value rounded to a multiple of the grid, halves upwards.
pub fn snap_to_grid(value: i64, grid_nm: i64) -> i64 {
    let grid = grid_nm.max(1);
    value
        .saturating_add(grid / 2)
        .div_euclid(grid)
        .saturating_mul(grid)
}

/// Every object exists and none is named twice.
fn validate(board: &Board, items: &[Item]) -> Result<(), BoardEditError> {
    let mut seen = BTreeSet::new();
    for item in items {
        if !item.exists(board) {
            return Err(missing(*item));
        }
        if !seen.insert(*item) {
            return Err(BoardEditError::new(
                ITEM_MISSING,
                format!("{item:?} is named twice"),
            ));
        }
    }
    Ok(())
}

fn require_selected(selection: &Selection) -> Result<(), BoardEditError> {
    if selection.is_empty() {
        return Err(BoardEditError::new(
            SELECTION_EMPTY,
            "the operation works on the selection and nothing is selected",
        ));
    }
    Ok(())
}

/// The offsets that line the objects up (see [`AlignMode`]).
pub fn align_moves(
    board: &Board,
    items: &[Item],
    mode: AlignMode,
) -> Result<Vec<ItemMove>, BoardEditError> {
    let needed = if matches!(mode, AlignMode::DistributeX | AlignMode::DistributeY) {
        3
    } else {
        2
    };
    let too_few = || {
        BoardEditError::new(
            ALIGN_TOO_FEW,
            format!(
                "{} objects given and at least {needed} are needed",
                items.len()
            ),
        )
    };
    if items.len() < needed {
        return Err(too_few());
    }
    validate(board, items)?;
    let mut boxes: Vec<(Item, Bounds)> = Vec::with_capacity(items.len());
    for item in items {
        let bounds = item_bounds(board, *item)
            .ok_or_else(|| geometry(format!("{item:?} covers nothing that can be aligned")))?;
        boxes.push((*item, bounds));
    }
    let Some(all) = boxes.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b)) else {
        return Err(too_few());
    };
    let horizontal = match mode {
        AlignMode::DistributeX => Some(true),
        AlignMode::DistributeY => Some(false),
        _ => None,
    };
    let Some(horizontal) = horizontal else {
        return Ok(boxes
            .iter()
            .map(|(item, b)| {
                let (dx_nm, dy_nm) = edge_offset(mode, &all, b);
                ItemMove {
                    item: *item,
                    dx_nm,
                    dy_nm,
                }
            })
            .collect());
    };
    let low = |b: &Bounds| if horizontal { b.min_x } else { b.min_y };
    let high = |b: &Bounds| if horizontal { b.max_x } else { b.max_y };
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|i| (low(&boxes[*i].1) + high(&boxes[*i].1), boxes[*i].0));
    let first_low = low(&boxes[order[0]].1);
    let last_high = high(&boxes[order[order.len() - 1]].1);
    let sizes: i64 = order
        .iter()
        .map(|i| high(&boxes[*i].1) - low(&boxes[*i].1))
        .sum();
    let free = last_high - first_low - sizes;
    let gaps = i64::try_from(order.len() - 1).unwrap_or(1);
    let mut before = 0_i64;
    let mut moves = Vec::with_capacity(order.len());
    for (k, i) in order.iter().enumerate() {
        let (item, b) = &boxes[*i];
        let k = i64::try_from(k).unwrap_or(0);
        let target = first_low + before + free.saturating_mul(k).div_euclid(gaps);
        let delta = target - low(b);
        before += high(b) - low(b);
        moves.push(if horizontal {
            ItemMove {
                item: *item,
                dx_nm: delta,
                dy_nm: 0,
            }
        } else {
            ItemMove {
                item: *item,
                dx_nm: 0,
                dy_nm: delta,
            }
        });
    }
    Ok(moves)
}

/// The offset of one object to the edge or the centre of the group.
fn edge_offset(mode: AlignMode, all: &Bounds, b: &Bounds) -> (i64, i64) {
    match mode {
        AlignMode::Left => (all.min_x - b.min_x, 0),
        AlignMode::Right => (all.max_x - b.max_x, 0),
        AlignMode::Top => (0, all.max_y - b.max_y),
        AlignMode::Bottom => (0, all.min_y - b.min_y),
        AlignMode::CenterX => ((all.min_x + all.max_x - b.min_x - b.max_x).div_euclid(2), 0),
        AlignMode::CenterY => (0, (all.min_y + all.max_y - b.min_y - b.max_y).div_euclid(2)),
        AlignMode::DistributeX | AlignMode::DistributeY => (0, 0),
    }
}

/// Turns an operation on the selection into the operation on the objects it holds: `move_selection`
/// into `move_items` (with the offset on the grid), `delete_selection` into `delete_items` and
/// `align_selection` into `move_items` with the offsets that line them up.
pub(crate) fn expand(
    op: &EditOp,
    board: &Board,
    selection: &Selection,
    settings: &BoardEditSettings,
) -> Result<EditOp, BoardEditError> {
    require_selected(selection)?;
    match op {
        EditOp::MoveSelection { dx_nm, dy_nm, snap } => {
            let (dx, dy) = if snap.unwrap_or(settings.snap) {
                (
                    snap_to_grid(*dx_nm, settings.grid_nm),
                    snap_to_grid(*dy_nm, settings.grid_nm),
                )
            } else {
                (*dx_nm, *dy_nm)
            };
            Ok(EditOp::MoveItems {
                moves: selection
                    .items()
                    .iter()
                    .map(|item| ItemMove {
                        item: *item,
                        dx_nm: dx,
                        dy_nm: dy,
                    })
                    .collect(),
            })
        }
        EditOp::DeleteSelection => Ok(EditOp::DeleteItems {
            items: selection.items().to_vec(),
        }),
        EditOp::AlignSelection { mode } => Ok(EditOp::MoveItems {
            moves: align_moves(board, selection.items(), *mode)?,
        }),
        _ => Err(refused("this is not an operation on the selection")),
    }
}

/// Plans `move_items` for everything but zones: one batch of the board.
pub(crate) fn plan_moves(
    board: &Board,
    env: &Env,
    moves: &[ItemMove],
) -> Result<Plan, BoardEditError> {
    let items: Vec<Item> = moves.iter().map(|m| m.item).collect();
    validate(board, &items)?;
    let mut ops: Vec<Operation> = Vec::new();
    for m in moves.iter().filter(|m| (m.dx_nm, m.dy_nm) != (0, 0)) {
        let (dx, dy) = (m.dx_nm, m.dy_nm);
        match m.item {
            Item::Component(id) => {
                check_unlocked(env, id)?;
                let c = board.components.get(&id).ok_or_else(|| missing(m.item))?;
                ops.push(Operation::MoveComponent {
                    id,
                    at: shift(c.at, dx, dy)?,
                    rotation: c.rotation,
                });
            }
            Item::Track(id) => {
                let mut track = board
                    .tracks
                    .get(&id)
                    .ok_or_else(|| missing(m.item))?
                    .clone();
                track.path = shift_path(&track.path, dx, dy)?;
                ops.push(replace_track(id, track));
            }
            Item::Via(id) => {
                let mut via = board.vias.get(&id).ok_or_else(|| missing(m.item))?.clone();
                via.at = shift(via.at, dx, dy)?;
                ops.push(Operation::RemoveVia { id });
                ops.push(Operation::AddVia { id, via });
            }
            Item::Silk(id) => {
                let mut silk = board
                    .silkscreen
                    .get(&id)
                    .ok_or_else(|| missing(m.item))?
                    .clone();
                match &mut silk {
                    SilkItem::Text { at, .. } => *at = shift(*at, dx, dy)?,
                    SilkItem::Stroke { path, .. } => *path = shift_path(path, dx, dy)?,
                }
                ops.push(Operation::RemoveSilk { id });
                ops.push(Operation::AddSilk { id, item: silk });
            }
            Item::Zone(_) => {
                return Err(refused("a zone is moved by the zone editor"));
            }
        }
    }
    Ok(single(Operation::Batch { ops }))
}

/// Plans `delete_items` for everything but zones: one batch of the board.
pub(crate) fn plan_delete(
    board: &Board,
    env: &Env,
    items: &[Item],
) -> Result<Plan, BoardEditError> {
    validate(board, items)?;
    let mut ops: Vec<Operation> = Vec::new();
    for item in items {
        ops.push(match *item {
            Item::Component(id) => {
                check_unlocked(env, id)?;
                Operation::RemoveComponent { id }
            }
            Item::Track(id) => Operation::RemoveTrack { id },
            Item::Via(id) => Operation::RemoveVia { id },
            Item::Silk(id) => Operation::RemoveSilk { id },
            Item::Zone(_) => return Err(refused("a zone is removed by the zone editor")),
        });
    }
    Ok(single(Operation::Batch { ops }))
}

/// When an operation on objects holds zones, the operations that do it as one group: the zones
/// by the zone editor (which keeps the properties and the fill in step) and the other objects
/// by the planner of this module. None when it holds no zone.
pub(crate) fn zone_split(
    op: &EditOp,
    board: &Board,
    data: &ZoneData,
) -> Result<Option<Vec<EditOp>>, BoardEditError> {
    match op {
        EditOp::MoveItems { moves } => {
            if !moves.iter().any(|m| matches!(m.item, Item::Zone(_))) {
                return Ok(None);
            }
            let items: Vec<Item> = moves.iter().map(|m| m.item).collect();
            validate(board, &items)?;
            let mut ops: Vec<EditOp> = Vec::new();
            let mut rest: Vec<ItemMove> = Vec::new();
            for m in moves {
                match m.item {
                    Item::Zone(id) => {
                        if (m.dx_nm, m.dy_nm) != (0, 0) {
                            ops.extend(zone_move(board, data, id, m.dx_nm, m.dy_nm)?);
                        }
                    }
                    _ => rest.push(*m),
                }
            }
            if !rest.is_empty() {
                ops.insert(0, EditOp::MoveItems { moves: rest });
            }
            Ok(Some(ops))
        }
        EditOp::DeleteItems { items } => {
            if !items.iter().any(|i| matches!(i, Item::Zone(_))) {
                return Ok(None);
            }
            validate(board, items)?;
            let mut ops: Vec<EditOp> = Vec::new();
            let mut rest: Vec<Item> = Vec::new();
            for item in items {
                match item {
                    Item::Zone(id) => ops.push(EditOp::ZoneRemove { zone: *id }),
                    _ => rest.push(*item),
                }
            }
            if !rest.is_empty() {
                ops.insert(0, EditOp::DeleteItems { items: rest });
            }
            Ok(Some(ops))
        }
        _ => Ok(None),
    }
}

/// The zone operations that move one zone: its cutouts come off, the outline moves, and the
/// cutouts go back at their new places in the same order.
fn zone_move(
    board: &Board,
    data: &ZoneData,
    id: eh3d_phoenix_board::Id,
    dx: i64,
    dy: i64,
) -> Result<Vec<EditOp>, BoardEditError> {
    let zone = board
        .zones
        .get(&id)
        .ok_or_else(|| missing(Item::Zone(id)))?;
    if !zone.cutouts.is_empty() {
        return Err(refused(format!(
            "zone {id} has cutouts in the model of the board and they cannot be moved with it"
        )));
    }
    let holes = data.props_of(id).cutouts;
    let mut ops: Vec<EditOp> = Vec::new();
    for _ in &holes {
        ops.push(EditOp::ZoneEdit {
            zone: id,
            net: None,
            layer: None,
            outline: None,
            clearance_nm: None,
            props: ZoneChanges {
                remove_cutout: Some(0),
                ..ZoneChanges::default()
            },
        });
    }
    ops.push(EditOp::ZoneEdit {
        zone: id,
        net: None,
        layer: None,
        outline: Some(shift_points(&zone.outline, dx, dy)?),
        clearance_nm: None,
        props: ZoneChanges::default(),
    });
    for hole in &holes {
        ops.push(EditOp::ZoneCutout {
            zone: id,
            polygon: shift_points(hole, dx, dy)?,
        });
    }
    Ok(ops)
}
