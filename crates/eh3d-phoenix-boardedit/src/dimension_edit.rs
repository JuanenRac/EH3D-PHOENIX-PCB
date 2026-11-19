// SPDX-License-Identifier: GPL-3.0-or-later
//! The dimension tool beyond the first drawing: the dimension in progress as the pointer would
//! leave it, and the editing of a dimension already on the board (moving one of its points and
//! changing its kind). Every change of a placed dimension is one step that undo takes back.
//!
//! A dimension is edited with shift and a click on one of its points (the two points measured,
//! the third point of an angle, and the middle of the dimension line or the end of the leader),
//! then a click puts the point where it is wanted. The measure is never stored, so it follows.

use eh3d_phoenix_board::{Dimension, DimensionKind, Id};
use eh3d_phoenix_model::{Nm, Point};

use crate::codes::{BoardEditError, OPERATION_REFUSED};
use crate::ops::EditOp;
use crate::tool::EditState;

/// A point of a dimension that the person can take and move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DimensionHandle {
    /// The first point measured (the centre of a radius, the vertex of an angle).
    From,
    /// The second point measured.
    To,
    /// The point on the second ray of an angle.
    Third,
    /// The dimension line: its middle, or the end of the leader of a radius or a diameter.
    Line,
}

/// A point of a dimension that was taken and waits for the click that puts it down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionGrab {
    /// The dimension.
    pub dimension: Id,
    /// The point of it.
    pub handle: DimensionHandle,
}

/// A dimension of `kind` from the points a person gives (`kind.clicks()` of them), with the
/// sizes of the text and of the lines; `None` when they measure nothing or do not fit.
pub fn build_dimension(
    kind: DimensionKind,
    points: &[Point],
    layer: Id,
    text_height: Nm,
    stroke: Nm,
) -> Option<Dimension> {
    if points.len() != kind.clicks() {
        return None;
    }
    let (from, to, through) = (points[0], points[1], points[2]);
    let (third, offset) = if kind == DimensionKind::Angle {
        (Some(through), Dimension::distance(from, points[3])?)
    } else {
        (None, Dimension::offset_through(kind, from, to, through)?)
    };
    let dimension = Dimension {
        layer,
        kind,
        from,
        to,
        third,
        offset,
        text_height,
        stroke,
    };
    dimension.items().ok().map(|_| dimension)
}

/// The points of a dimension that can be taken, where they are.
pub fn handles(d: &Dimension) -> Vec<(DimensionHandle, Point)> {
    let mut found = vec![(DimensionHandle::From, d.from), (DimensionHandle::To, d.to)];
    if let Some(third) = d.third {
        found.push((DimensionHandle::Third, third));
    } else if matches!(d.kind, DimensionKind::Radius | DimensionKind::Diameter) {
        if let Ok(end) = d.leader_end() {
            found.push((DimensionHandle::Line, end));
        }
    } else if let (Ok(a), Ok(b)) = (d.line_point(), d.leader_end()) {
        let middle = |u: i64, v: i64| ((i128::from(u) + i128::from(v)) / 2) as i64;
        if let Ok(mid) = Point::from_nm(
            middle(a.x_nm.get(), b.x_nm.get()),
            middle(a.y_nm.get(), b.y_nm.get()),
        ) {
            found.push((DimensionHandle::Line, mid));
        }
    }
    found
}

/// The dimension with one of its points put at `to`; `None` when the result measures nothing,
/// does not fit, or the point does not exist in this kind.
pub fn moved(d: &Dimension, handle: DimensionHandle, to: Point) -> Option<Dimension> {
    let mut n = d.clone();
    match handle {
        DimensionHandle::From => n.from = to,
        DimensionHandle::To => n.to = to,
        DimensionHandle::Third => {
            n.third?;
            n.third = Some(to);
        }
        DimensionHandle::Line => {
            if d.kind == DimensionKind::Angle {
                return None;
            }
            n.offset = Dimension::offset_through(d.kind, d.from, d.to, to)?;
        }
    }
    n.items().ok().map(|_| n)
}

/// The dimension changed to another kind with the same points and the dimension line where it
/// was. An angle needs a third point that the others do not have, so neither a change to an angle
/// nor from one can keep the line; a change from an angle puts the line through the second point.
pub fn with_kind(d: &Dimension, kind: DimensionKind) -> Option<Dimension> {
    if kind == d.kind {
        return Some(d.clone());
    }
    if kind == DimensionKind::Angle {
        return None;
    }
    let through = if d.kind == DimensionKind::Angle {
        d.to
    } else {
        handles(d)
            .into_iter()
            .find(|(h, _)| *h == DimensionHandle::Line)
            .map(|(_, p)| p)?
    };
    let mut n = d.clone();
    n.kind = kind;
    n.third = None;
    n.offset = Dimension::offset_through(kind, d.from, d.to, through)?;
    n.items().ok().map(|_| n)
}

fn distance_sq(a: Point, b: Point) -> i128 {
    let dx = i128::from(a.x_nm.get()) - i128::from(b.x_nm.get());
    let dy = i128::from(a.y_nm.get()) - i128::from(b.y_nm.get());
    dx * dx + dy * dy
}

impl EditState {
    fn documentation_layer(&self) -> Option<Id> {
        self.board()
            .layers
            .iter()
            .find(|(_, l)| l.kind == eh3d_phoenix_board::LayerKind::Documentation)
            .map(|(id, _)| *id)
    }

    /// The dimension as the pointer at `cursor` would leave it: the one being drawn with the
    /// pointer as its next point (as soon as it has two points; an angle, three), or the one
    /// whose point was taken with the pointer as that point. `None` when there is nothing to
    /// show or what there is measures nothing. The board is not touched.
    pub fn dimension_preview(&self, cursor: Point) -> Option<Dimension> {
        let at = self.snap(cursor);
        if let Some(grab) = self.dimension_grab {
            let d = self.board().dimensions.get(&grab.dimension)?;
            return moved(d, grab.handle, at);
        }
        if self.dimension_points.is_empty() {
            return None;
        }
        let mut points = self.dimension_points.clone();
        points.push(at);
        let kind = self.dimension_kind;
        if points.len() < kind.clicks() {
            // not all the points yet: the line passes through the last one for the straight
            // kinds, and the arc of an angle through the first ray
            let last = points[points.len() - 1];
            let first_ray = points.get(1).copied().unwrap_or(last);
            while points.len() < kind.clicks() {
                points.push(if kind == DimensionKind::Angle {
                    first_ray
                } else {
                    last
                });
            }
        }
        let layer = self.documentation_layer()?;
        let settings = &self.edit.settings;
        build_dimension(
            kind,
            &points,
            layer,
            Nm::new(settings.dimension_text_height_nm).ok()?,
            Nm::new(settings.dimension_stroke_nm).ok()?,
        )
    }

    /// The point of a dimension under `p`, if any is within the pick tolerance; the nearest one
    /// when several are.
    pub fn dimension_handle_at(&self, p: Point) -> Option<(Id, DimensionHandle)> {
        let tolerance = i128::from(self.edit.settings.pick_tolerance_nm.max(1));
        let mut best: Option<(i128, Id, DimensionHandle)> = None;
        for (id, d) in &self.board().dimensions {
            for (handle, at) in handles(d) {
                let gap = distance_sq(p, at);
                if gap <= tolerance * tolerance && best.is_none_or(|(g, _, _)| gap < g) {
                    best = Some((gap, *id, handle));
                }
            }
        }
        best.map(|(_, id, handle)| (id, handle))
    }

    /// Takes the point of a dimension under `p` so that the next click puts it down; false (and
    /// the error says so) when there is none.
    pub fn grab_dimension_point(&mut self, p: Point) -> bool {
        match self.dimension_handle_at(p) {
            Some((dimension, handle)) => {
                self.selected_dimension = Some(dimension);
                self.dimension_grab = Some(DimensionGrab { dimension, handle });
                self.error = None;
                true
            }
            None => {
                self.selected_dimension = None;
                self.dimension_grab = None;
                self.fail(BoardEditError::new(
                    OPERATION_REFUSED,
                    "click a point of a dimension",
                ));
                false
            }
        }
    }

    /// Puts a point of a dimension at `to` (on the grid) as one step; false when it was refused.
    pub fn move_dimension_point(&mut self, id: Id, handle: DimensionHandle, to: Point) -> bool {
        let to = self.snap(to);
        let Some(old) = self.board().dimensions.get(&id) else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                format!("no dimension {id}"),
            ));
            return false;
        };
        let Some(new) = moved(old, handle, to) else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                "the point does not make a dimension",
            ));
            return false;
        };
        self.replace_dimension(id, new)
    }

    /// Changes a dimension to another kind, keeping its points and its line, as one step; false
    /// when it was refused (to or from an angle).
    pub fn change_dimension_kind(&mut self, id: Id, kind: DimensionKind) -> bool {
        let Some(old) = self.board().dimensions.get(&id) else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                format!("no dimension {id}"),
            ));
            return false;
        };
        if old.kind == kind {
            return true;
        }
        let Some(new) = with_kind(old, kind) else {
            self.fail(BoardEditError::new(
                OPERATION_REFUSED,
                "a dimension cannot change to or from an angle: it has a point more",
            ));
            return false;
        };
        self.replace_dimension(id, new)
    }

    /// Puts `new` in the place of the dimension `id` as one step (removing one and adding one).
    /// The new dimension is the selected one afterwards.
    pub fn replace_dimension(&mut self, id: Id, new: Dimension) -> bool {
        self.dimension_grab = None;
        let done = self.run(&EditOp::Group {
            ops: vec![
                EditOp::RemoveDimension { dimension: id },
                EditOp::AddDimension { dimension: new },
            ],
        });
        if done {
            self.selected_dimension = self
                .last
                .as_ref()
                .and_then(|o| o.created.last().copied())
                .filter(|c| self.board().dimensions.contains_key(c));
        }
        done
    }
}
