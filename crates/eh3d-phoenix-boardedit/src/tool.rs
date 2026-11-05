// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the editing tool: what the person has chosen and is doing. The window passes on
//! pointer events and keys; every decision is taken here, in code that is tested without a screen.

use std::collections::BTreeSet;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, Operation};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_route::{copper_layers, RatLine};

use crate::codes::{BoardEditError, OPERATION_REFUSED};
use crate::edit::{BoardEdit, Outcome};
use crate::hit::{in_rectangle, pad_near, pick, Picked};
use crate::ops::EditOp;
use crate::shape::{legs, MeanderSide, Shape};

/// The tool that a click uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Pick, select by rectangle and drag.
    Select,
    /// Place the footprint that was chosen.
    Place,
    /// Draw a track, one click for each corner.
    Route,
    /// Put a via on the net under the pointer.
    Via,
    /// Delete what is under the pointer.
    Delete,
    /// Tune the net under the pointer to the target length.
    Tune,
}

/// A drag of components in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drag {
    /// The components that are moving.
    pub components: Vec<Id>,
    /// Where the drag began (on the grid).
    pub from: Point,
    /// Where the pointer is now (on the grid).
    pub to: Point,
}

impl Drag {
    /// The offset of the drag in nanometres.
    pub fn offset(&self) -> (i64, i64) {
        (
            self.to.x_nm.get() - self.from.x_nm.get(),
            self.to.y_nm.get() - self.from.y_nm.get(),
        )
    }
}

/// The board editor with the state of its tools.
pub struct EditState {
    /// The board, its rules, its history.
    pub edit: BoardEdit,
    /// The tool.
    pub tool: Tool,
    /// What is selected.
    pub selection: BTreeSet<Picked>,
    /// The copper layer that tracks are drawn on.
    pub active_layer: Option<Id>,
    /// The net that is lit.
    pub highlight_net: Option<Id>,
    /// The drag in progress.
    pub drag: Option<Drag>,
    /// The rectangle being drawn to select: its two corners.
    pub marquee: Option<(Point, Point)>,
    /// The net of the route in progress.
    pub route_net: Option<Id>,
    /// The points of the route in progress.
    pub route_points: Vec<Point>,
    /// The footprint the place tool places.
    pub place_footprint: Option<Id>,
    /// The reference designator for the next component; one is made when it is empty.
    pub place_refdes: String,
    /// The value for the next component.
    pub place_value: String,
    /// The length the tune tool asks for, in nanometres.
    pub tune_target_nm: i64,
    /// Whether routes use 45-degree pieces.
    pub diagonal: bool,
    /// Whether a corner goes first along x.
    pub horizontal_first: bool,
    /// Why the last action was refused.
    pub error: Option<BoardEditError>,
    /// What the last operation did.
    pub last: Option<Outcome>,
}

impl EditState {
    /// The state of the tools for an editor.
    pub fn new(edit: BoardEdit) -> EditState {
        let active_layer = copper_layers(edit.board()).first().copied();
        let place_footprint = edit.board().footprints.keys().next().copied();
        let diagonal = edit.settings.diagonal_routes;
        let horizontal_first = edit.settings.corner_horizontal_first;
        EditState {
            edit,
            tool: Tool::Select,
            selection: BTreeSet::new(),
            active_layer,
            highlight_net: None,
            drag: None,
            marquee: None,
            route_net: None,
            route_points: Vec::new(),
            place_footprint,
            place_refdes: String::new(),
            place_value: String::new(),
            tune_target_nm: 0,
            diagonal,
            horizontal_first,
            error: None,
            last: None,
        }
    }

    /// The board as it is now.
    pub fn board(&self) -> &Board {
        self.edit.board()
    }

    /// A point on the grid when snapping is on, else the point itself.
    pub fn snap(&self, p: Point) -> Point {
        if !self.edit.settings.snap {
            return p;
        }
        let grid = self.edit.settings.grid_nm.max(1);
        let round = |v: i64| (v + grid / 2).div_euclid(grid) * grid;
        Point::from_nm(round(p.x_nm.get()), round(p.y_nm.get())).unwrap_or(p)
    }

    /// Chooses a tool and drops what the old one had in progress.
    pub fn set_tool(&mut self, tool: Tool) {
        self.cancel();
        self.tool = tool;
    }

    /// Chooses the copper layer tracks are drawn on; false when the layer is not copper.
    pub fn set_active_layer(&mut self, layer: Id) -> bool {
        let copper = self
            .board()
            .layers
            .get(&layer)
            .is_some_and(|l| l.kind == LayerKind::Copper);
        if copper {
            self.active_layer = Some(layer);
        }
        copper
    }

    /// Lights a net, or none.
    pub fn set_highlight(&mut self, net: Option<Id>) {
        self.highlight_net = net;
    }

    /// Drops the route, the drag and the rectangle in progress.
    pub fn cancel(&mut self) {
        self.drag = None;
        self.marquee = None;
        self.route_points.clear();
        self.route_net = None;
    }

    /// What lies under a point.
    pub fn pick_at(&self, p: Point) -> Option<Picked> {
        pick(
            self.board(),
            p,
            self.edit.settings.pick_tolerance_nm,
            self.edit.settings.part_padding_nm,
        )
    }

    /// The net under a point: the net of the pad, the track or the via there, else the lit net.
    pub fn net_at(&self, p: Point) -> Option<Id> {
        let board = self.board();
        if let Some(pad) = pad_near(board, p, self.edit.settings.pick_tolerance_nm) {
            if let Some(net) = board.net_of(pad) {
                return Some(net);
            }
        }
        self.pick_at(p)
            .and_then(|item| item.net(board))
            .or(self.highlight_net)
    }

    fn fail(&mut self, error: BoardEditError) {
        self.error = Some(error);
    }

    fn prune(&mut self) {
        let board = self.edit.board();
        self.selection.retain(|item| item.exists(board));
    }

    /// Applies an operation and keeps what it said; false when it was refused.
    pub fn run(&mut self, op: &EditOp) -> bool {
        match self.edit.apply(op) {
            Ok(outcome) => {
                self.error = None;
                self.last = Some(outcome);
                self.prune();
                true
            }
            Err(e) => {
                self.fail(e);
                false
            }
        }
    }

    /// Undoes the last step.
    pub fn undo(&mut self) -> bool {
        let done = self.edit.undo();
        self.prune();
        done
    }

    /// Redoes the last undone step.
    pub fn redo(&mut self) -> bool {
        let done = self.edit.redo();
        self.prune();
        done
    }

    /// The connections still to be routed.
    pub fn ratsnest(&self) -> Vec<RatLine> {
        self.edit.ratsnest()
    }

    /// The components that are selected.
    pub fn selected_components(&self) -> Vec<Id> {
        self.selection
            .iter()
            .filter_map(|item| match item {
                Picked::Component(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Lights the net of the selected track or via.
    pub fn highlight_selected_net(&mut self) {
        let board = self.edit.board();
        let net = self.selection.iter().find_map(|item| item.net(board));
        if net.is_some() {
            self.highlight_net = net;
        }
    }

    /// Deletes what is selected, as one step.
    pub fn delete_selection(&mut self) -> bool {
        let ops: Vec<EditOp> = self
            .selection
            .iter()
            .map(|item| match item {
                Picked::Component(id) => EditOp::Remove { id: *id },
                Picked::Track(id) => EditOp::DeleteTrack { track: *id },
                Picked::Via(id) => EditOp::DeleteVia { via: *id },
            })
            .collect();
        if ops.is_empty() {
            return false;
        }
        self.run(&EditOp::Group { ops })
    }

    /// Turns the selected components by `quarters` quarter turns (negative: clockwise).
    pub fn rotate_selection(&mut self, quarters: i64) -> bool {
        let ops: Vec<EditOp> = self
            .selected_components()
            .into_iter()
            .map(|id| EditOp::Rotate {
                id,
                delta_udeg: quarters.saturating_mul(90_000_000),
                follow: None,
            })
            .collect();
        if ops.is_empty() {
            return false;
        }
        self.run(&EditOp::Group { ops })
    }

    /// Puts the selected components on the other side of the board.
    pub fn mirror_selection(&mut self) -> bool {
        let ops: Vec<EditOp> = self
            .selected_components()
            .into_iter()
            .map(|id| EditOp::Mirror { id, follow: None })
            .collect();
        if ops.is_empty() {
            return false;
        }
        self.run(&EditOp::Group { ops })
    }

    /// Locks the selected components that are free, or unlocks them when all are locked.
    pub fn toggle_lock_selection(&mut self) -> bool {
        let ids = self.selected_components();
        if ids.is_empty() {
            return false;
        }
        let all_locked = ids.iter().all(|id| self.edit.is_locked(*id));
        let ops: Vec<EditOp> = ids
            .into_iter()
            .map(|id| {
                if all_locked {
                    EditOp::Unlock { id }
                } else {
                    EditOp::Lock { id }
                }
            })
            .collect();
        self.run(&EditOp::Group { ops })
    }

    fn next_refdes(&self) -> String {
        let board = self.board();
        let mut n = board.components.len() + 1;
        loop {
            let candidate = format!("U{n}");
            if !board.components.values().any(|c| c.refdes == candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Places the chosen footprint at a point (on the grid when snapping is on).
    pub fn place_at(&mut self, p: Point) -> bool {
        let Some(footprint) = self.place_footprint else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                "no footprint is chosen",
            ));
            return false;
        };
        let refdes = if self.place_refdes.trim().is_empty() {
            self.next_refdes()
        } else {
            self.place_refdes.clone()
        };
        let op = EditOp::Place {
            id: None,
            refdes,
            value: self.place_value.clone(),
            footprint,
            at: self.snap(p),
            rotation_udeg: 0,
            side: None,
        };
        let done = self.run(&op);
        if done {
            self.place_refdes.clear();
        }
        done
    }

    /// Finishes the route in progress: one track through the points that were clicked.
    pub fn finish_route(&mut self) -> bool {
        let points = std::mem::take(&mut self.route_points);
        let net = self.route_net.take();
        if points.len() < 2 {
            return false;
        }
        let (Some(net), Some(layer)) = (net, self.active_layer) else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                "the route has no net or no layer",
            ));
            return false;
        };
        let shape = if self.diagonal {
            Shape::Diagonal
        } else {
            Shape::Orthogonal
        };
        self.run(&EditOp::Route {
            net,
            layer,
            points,
            shape: Some(shape),
            width_nm: None,
            horizontal_first: Some(self.horizontal_first),
        })
    }

    /// The corners of the route in progress as it would be drawn if the next click were at `p`.
    pub fn route_preview(&self, p: Point) -> Vec<Point> {
        let mut points = self.route_points.clone();
        if points.is_empty() {
            return points;
        }
        points.push(self.snap(p));
        let shape = if self.diagonal {
            Shape::Diagonal
        } else {
            Shape::Orthogonal
        };
        let mut out: Vec<Point> = Vec::new();
        for pair in points.windows(2) {
            if let Ok(corners) = legs(pair[0], pair[1], shape, self.horizontal_first) {
                for corner in corners {
                    if out.last() != Some(&corner) {
                        out.push(corner);
                    }
                }
            }
        }
        out
    }

    /// The board as the drag in progress would leave it (the components moved, the tracks left
    /// where they are), for drawing.
    pub fn drag_preview(&self) -> Option<Board> {
        let drag = self.drag.as_ref()?;
        let (dx, dy) = drag.offset();
        let mut board = self.board().clone();
        for id in &drag.components {
            let component = board.components.get(id)?;
            let at = Point::from_nm(
                component.at.x_nm.get().checked_add(dx)?,
                component.at.y_nm.get().checked_add(dy)?,
            )
            .ok()?;
            let rotation = component.rotation;
            board
                .apply(&Operation::MoveComponent {
                    id: *id,
                    at,
                    rotation,
                })
                .ok()?;
        }
        Some(board)
    }

    /// The pointer went down at `p`. With shift, the thing picked is added to the selection.
    pub fn pointer_pressed(&mut self, p: Point, shift: bool) {
        match self.tool {
            Tool::Select => self.press_select(p, shift),
            Tool::Place => {
                self.place_at(p);
            }
            Tool::Route => self.press_route(p),
            Tool::Via => match self.net_at(p) {
                Some(net) => {
                    let at = self.snap(p);
                    self.run(&EditOp::Via {
                        net,
                        at,
                        from_layer: None,
                        to_layer: None,
                    });
                }
                None => self.fail(BoardEditError::new(
                    OPERATION_REFUSED,
                    "click a pad, a track or a via of the net",
                )),
            },
            Tool::Delete => {
                if let Some(item) = self.pick_at(p) {
                    self.selection.clear();
                    self.selection.insert(item);
                    self.delete_selection();
                }
            }
            Tool::Tune => match self.pick_at(p).and_then(|item| item.net(self.board())) {
                Some(net) => {
                    self.run(&EditOp::TuneLength {
                        net,
                        target_nm: self.tune_target_nm,
                        track: None,
                        side: MeanderSide::Auto,
                    });
                }
                None => self.fail(BoardEditError::new(
                    OPERATION_REFUSED,
                    "click a track or a via of the net",
                )),
            },
        }
    }

    fn press_select(&mut self, p: Point, shift: bool) {
        match self.pick_at(p) {
            Some(item) => {
                if shift {
                    if !self.selection.remove(&item) {
                        self.selection.insert(item);
                    }
                } else if !self.selection.contains(&item) {
                    self.selection.clear();
                    self.selection.insert(item);
                }
                self.highlight_selected_net();
                let free: Vec<Id> = self
                    .selected_components()
                    .into_iter()
                    .filter(|id| !self.edit.is_locked(*id))
                    .collect();
                if matches!(item, Picked::Component(_))
                    && !free.is_empty()
                    && self.selection.contains(&item)
                {
                    let at = self.snap(p);
                    self.drag = Some(Drag {
                        components: free,
                        from: at,
                        to: at,
                    });
                }
            }
            None => {
                if !shift {
                    self.selection.clear();
                }
                self.marquee = Some((p, p));
            }
        }
    }

    fn press_route(&mut self, p: Point) {
        let board = self.edit.board();
        let tolerance = self.edit.settings.pick_tolerance_nm;
        let pad = pad_near(board, p, tolerance);
        let pad_point = pad.and_then(|pad| pad_position(board, pad));
        if self.route_points.is_empty() {
            match self.net_at(p) {
                Some(net) => {
                    self.route_net = Some(net);
                    self.highlight_net = Some(net);
                    let start = pad_point.unwrap_or_else(|| self.snap(p));
                    self.route_points.push(start);
                }
                None => self.fail(BoardEditError::new(
                    OPERATION_REFUSED,
                    "start on a pad, a track or a via of the net",
                )),
            }
            return;
        }
        let ends_on_pad = match (pad, pad_point) {
            (Some(pad), Some(_)) => board.net_of(pad) == self.route_net,
            _ => false,
        };
        let next = if ends_on_pad {
            pad_point.unwrap_or_else(|| self.snap(p))
        } else {
            self.snap(p)
        };
        self.route_points.push(next);
        if ends_on_pad {
            self.finish_route();
        }
    }

    /// The pointer moved to `p`.
    pub fn pointer_moved(&mut self, p: Point) {
        let at = self.snap(p);
        if let Some(drag) = self.drag.as_mut() {
            drag.to = at;
        }
        if let Some((_, to)) = self.marquee.as_mut() {
            *to = p;
        }
    }

    /// The pointer went up at `p`: a drag moves the components, a rectangle selects.
    pub fn pointer_released(&mut self, p: Point) {
        self.pointer_moved(p);
        if let Some(drag) = self.drag.take() {
            let (dx, dy) = drag.offset();
            if dx != 0 || dy != 0 {
                self.move_components(&drag.components, dx, dy);
            }
        }
        if let Some((a, b)) = self.marquee.take() {
            let found = in_rectangle(self.board(), a, b, self.edit.settings.part_padding_nm);
            self.selection.extend(found);
            self.highlight_selected_net();
        }
    }

    /// Moves components by an offset, as one step, with the tracks on their pads following when
    /// the settings say so.
    pub fn move_components(&mut self, ids: &[Id], dx: i64, dy: i64) -> bool {
        let mut ops: Vec<EditOp> = Vec::new();
        for id in ids {
            let Some(component) = self.board().components.get(id) else {
                continue;
            };
            let moved = Point::from_nm(
                component.at.x_nm.get().saturating_add(dx),
                component.at.y_nm.get().saturating_add(dy),
            );
            match moved {
                Ok(at) => ops.push(EditOp::Move {
                    id: *id,
                    at,
                    rotation_udeg: None,
                    follow: None,
                }),
                Err(e) => {
                    self.fail(BoardEditError::new(
                        crate::codes::GEOMETRY_INVALID,
                        e.to_string(),
                    ));
                    return false;
                }
            }
        }
        if ops.is_empty() {
            return false;
        }
        self.run(&EditOp::Group { ops })
    }
}
