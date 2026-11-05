// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing as reversible operations of the board: move and turn a component, delete a track, move
//! a vertex of a track, change a width and add a via, and an editor that applies them with undo
//! and redo and says, after each one, which findings of the rules the edit added and removed.
//!
//! Every function here builds an [`Operation`] from the state of the board and changes nothing.
//! The operations that replace a track are a batch (remove it, add it back changed with the same
//! id), so they are all or nothing and their inverse brings the board back byte for byte.

use eh3d_phoenix_board::{Board, History, Id, Operation, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point, Udeg, ANGLE_LIMIT_UDEG};

use crate::codes::{
    RouteError, ANGLE_BROKEN, EDIT_INVALID, EDIT_REJECTED, OPERATION_FAILED, OUT_OF_RANGE,
};
use crate::geom::{is_octilinear, pair};
use crate::live::{diff, findings, LiveDiff, RuleContext};
use crate::router::via_size;
use crate::settings::RouteSettings;

fn invalid(what: String) -> RouteError {
    RouteError::new(EDIT_INVALID, what)
}

/// Moves a component, keeping its rotation.
pub fn move_component(board: &Board, id: Id, at: Point) -> Result<Operation, RouteError> {
    let component = board
        .components
        .get(&id)
        .ok_or_else(|| invalid(format!("no component {id}")))?;
    Ok(Operation::MoveComponent {
        id,
        at,
        rotation: component.rotation,
    })
}

/// Turns a component by an angle in micro-degrees (positive is counter-clockwise), keeping its
/// place. The result is kept inside one turn.
pub fn rotate_component(board: &Board, id: Id, delta_udeg: i64) -> Result<Operation, RouteError> {
    let component = board
        .components
        .get(&id)
        .ok_or_else(|| invalid(format!("no component {id}")))?;
    let turned = component
        .rotation
        .get()
        .saturating_add(delta_udeg)
        .rem_euclid(ANGLE_LIMIT_UDEG);
    let rotation = Udeg::new(turned).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?;
    Ok(Operation::MoveComponent {
        id,
        at: component.at,
        rotation,
    })
}

/// Deletes a track.
pub fn delete_track(board: &Board, id: Id) -> Result<Operation, RouteError> {
    if !board.tracks.contains_key(&id) {
        return Err(invalid(format!("no track {id}")));
    }
    Ok(Operation::RemoveTrack { id })
}

/// Adds a via; the board checks the net and the layers when the operation is applied.
pub fn add_via(id: Id, via: Via) -> Operation {
    Operation::AddVia { id, via }
}

/// The via that a change of layer needs at a place: the size comes from the rules and the net class
/// of its net, through [`via_size`].
pub fn via_for_layer_change(
    board: &Board,
    ctx: &RuleContext,
    settings: &RouteSettings,
    net: Id,
    at: Point,
    layers: (Id, Id),
) -> Result<Via, RouteError> {
    let name = board
        .nets
        .get(&net)
        .map(|n| n.name.clone())
        .ok_or_else(|| invalid(format!("no net {net}")))?;
    for layer in [layers.0, layers.1] {
        if !board.layers.contains_key(&layer) {
            return Err(invalid(format!("no layer {layer}")));
        }
    }
    let (drill, diameter) = via_size(ctx, settings, &name);
    Ok(Via {
        net,
        at,
        drill: Nm::new(drill).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
        diameter: Nm::new(diameter).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
        from_layer: layers.0,
        to_layer: layers.1,
    })
}

fn replace_track(id: Id, track: Track) -> Operation {
    Operation::Batch {
        ops: vec![
            Operation::RemoveTrack { id },
            Operation::AddTrack { id, track },
        ],
    }
}

/// Changes the width of a track.
pub fn set_track_width(board: &Board, id: Id, width: Nm) -> Result<Operation, RouteError> {
    let track = board
        .tracks
        .get(&id)
        .ok_or_else(|| invalid(format!("no track {id}")))?;
    let mut changed = track.clone();
    changed.width = width;
    Ok(replace_track(id, changed))
}

fn with_start(segment: Segment, to: Point) -> Segment {
    match segment {
        Segment::Line { end, .. } => Segment::Line { start: to, end },
        Segment::Arc { end, sweep, .. } => Segment::Arc {
            start: to,
            end,
            sweep,
        },
    }
}

fn with_end(segment: Segment, to: Point) -> Segment {
    match segment {
        Segment::Line { start, .. } => Segment::Line { start, end: to },
        Segment::Arc { start, sweep, .. } => Segment::Arc {
            start,
            end: to,
            sweep,
        },
    }
}

/// Moves a vertex of a track. The vertices are the start of the first piece and the end of every
/// piece, numbered from zero; the pieces on both sides follow the vertex. With `keep_angles` a
/// straight piece that ran at 0, 45 or 90 degrees must still do so. A piece that would shrink to a
/// point is refused.
pub fn move_vertex(
    board: &Board,
    track: Id,
    index: usize,
    to: Point,
    keep_angles: bool,
) -> Result<Operation, RouteError> {
    let original = board
        .tracks
        .get(&track)
        .ok_or_else(|| invalid(format!("no track {track}")))?;
    let count = original.path.len();
    if index > count {
        return Err(invalid(format!(
            "the track {track} has {} vertices, there is no vertex {index}",
            count + 1
        )));
    }
    let mut path = original.path.clone();
    let mut touched: Vec<usize> = Vec::new();
    if index > 0 {
        path[index - 1] = with_end(path[index - 1], to);
        touched.push(index - 1);
    }
    if index < count {
        path[index] = with_start(path[index], to);
        touched.push(index);
    }
    for i in touched {
        let (before, after) = (original.path[i], path[i]);
        if after.start() == after.end() {
            return Err(invalid(
                "a piece of the track would shrink to a point".to_owned(),
            ));
        }
        if keep_angles {
            if let (Segment::Line { start, end }, Segment::Line { .. }) = (before, after) {
                let was = is_octilinear(pair(start), pair(end));
                let is = is_octilinear(pair(after.start()), pair(after.end()));
                if was && !is {
                    return Err(RouteError::new(
                        ANGLE_BROKEN,
                        format!(
                            "the piece {i} of the track {track} would leave 0, 45 and 90 degrees"
                        ),
                    ));
                }
            }
        }
    }
    let mut changed = original.clone();
    changed.path = path;
    Ok(replace_track(track, changed))
}

/// Applies operations to a board with undo and redo and reports, after each one, what the rules say
/// changed.
#[derive(Debug, Clone)]
pub struct Editor {
    history: History,
    ctx: RuleContext,
    strict: bool,
}

impl Editor {
    /// An editor with an empty history. With `strict_edits` in the settings an edit that would add
    /// a rule error is refused.
    pub fn new(ctx: RuleContext, settings: &RouteSettings) -> Editor {
        Editor {
            history: History::new(),
            ctx,
            strict: settings.strict_edits,
        }
    }

    /// The history of the edits that are done.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Applies an operation, all or nothing, and returns the findings it added and removed. The
    /// operation is tried on a copy first: when it fails, or when it adds a rule error in strict
    /// mode, the board is untouched and nothing is recorded.
    pub fn apply(&mut self, board: &mut Board, op: Operation) -> Result<LiveDiff, RouteError> {
        let before = findings(board, &self.ctx);
        let mut trial = board.clone();
        trial
            .apply(&op)
            .map_err(|e| RouteError::new(OPERATION_FAILED, e.to_string()))?;
        let after = findings(&trial, &self.ctx);
        let changes = diff(&before, &after);
        if self.strict {
            if let Some(first) = changes
                .added
                .iter()
                .find(|f| f.severity == eh3d_phoenix_rules::Severity::Error)
            {
                return Err(RouteError::new(
                    EDIT_REJECTED,
                    format!("{} at {}", first.code, first.location),
                ));
            }
        }
        self.history
            .execute(board, op)
            .map_err(|e| RouteError::new(OPERATION_FAILED, e.to_string()))?;
        Ok(changes)
    }

    /// Undoes the last edit; none when there is nothing to undo.
    pub fn undo(&mut self, board: &mut Board) -> Option<LiveDiff> {
        let before = findings(board, &self.ctx);
        if !self.history.undo(board) {
            return None;
        }
        Some(diff(&before, &findings(board, &self.ctx)))
    }

    /// Redoes the last undone edit; none when there is nothing to redo.
    pub fn redo(&mut self, board: &mut Board) -> Option<LiveDiff> {
        let before = findings(board, &self.ctx);
        if !self.history.redo(board) {
            return None;
        }
        Some(diff(&before, &findings(board, &self.ctx)))
    }
}
