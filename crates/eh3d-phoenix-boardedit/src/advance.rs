// SPDX-License-Identifier: GPL-3.0-or-later
//! The advance of a route: a track that goes on from a point toward the pointer for as long as
//! it breaks no rule, and stops at the last place that is clear. The search is made on a copy of
//! the editor, so the board is not changed; the person then keeps the clear part (a `route` of
//! the points that this gives) or pushes on with the push tool.

use eh3d_phoenix_board::Id;
use eh3d_phoenix_model::Point;

use crate::codes::BoardEditError;
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::shape::{legs, pt, Shape};

/// What the advance found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advance {
    /// The corners of the clear part of the route, from the start; the last is where it stops.
    pub points: Vec<Point>,
    /// True when the whole route to the target is clear.
    pub complete: bool,
}

fn unit(v: i64) -> i64 {
    v.signum()
}

impl BoardEdit {
    fn clear_route(&self, trial: &mut BoardEdit, net: Id, layer: Id, points: &[Point]) -> bool {
        let before = trial.steps();
        let op = EditOp::Route {
            net,
            layer,
            points: points.to_vec(),
            shape: Some(Shape::Diagonal),
            width_nm: None,
            horizontal_first: None,
        };
        let clear = match trial.apply(&op) {
            Ok(outcome) => !outcome.drc.adds_error(),
            Err(_) => false,
        };
        while trial.steps() > before {
            if !trial.undo() {
                break;
            }
        }
        clear
    }

    /// Goes from `from` toward `to` with the shape of a route (`shape`, the corner first along x
    /// when `horizontal_first`) and returns the clear part: every corner that is reached and the
    /// last clear point of the leg that is blocked, to the nearest nanometre of the search that
    /// the setting `advance_iterations` allows. A route that is blocked at its start gives only
    /// the start.
    pub fn advance_route(
        &self,
        net: Id,
        layer: Id,
        from: Point,
        to: Point,
        horizontal_first: bool,
    ) -> Result<Advance, BoardEditError> {
        self.advance_route_shaped(net, layer, from, to, horizontal_first, Shape::Diagonal)
    }

    /// The advance of a route whose corners follow `shape` (orthogonal or with 45 degrees).
    pub fn advance_route_shaped(
        &self,
        net: Id,
        layer: Id,
        from: Point,
        to: Point,
        horizontal_first: bool,
        shape: Shape,
    ) -> Result<Advance, BoardEditError> {
        let mut settings = self.settings.clone();
        settings.live_drc = true;
        settings.reject_new_errors = false;
        let mut trial = BoardEdit::new(self.board().clone(), self.context().clone(), settings)?;
        let corners = legs(from, to, shape, horizontal_first)?;
        let mut kept = vec![from];
        for pair in corners.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a == b {
                continue;
            }
            let mut candidate = kept.clone();
            candidate.push(b);
            if self.clear_route(&mut trial, net, layer, &candidate) {
                kept.push(b);
                continue;
            }
            // the leg is blocked: search the last clear step along it
            let (dx, dy) = (b.x_nm.get() - a.x_nm.get(), b.y_nm.get() - a.y_nm.get());
            let (ux, uy) = (unit(dx), unit(dy));
            let (mut lo, mut hi) = (0i64, dx.abs().max(dy.abs()));
            for _ in 0..self.settings.advance_iterations {
                if hi - lo <= 1 {
                    break;
                }
                let mid = lo + (hi - lo) / 2;
                let p = pt(a.x_nm.get() + ux * mid, a.y_nm.get() + uy * mid)?;
                let mut candidate = kept.clone();
                candidate.push(p);
                if self.clear_route(&mut trial, net, layer, &candidate) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            if lo > 0 {
                kept.push(pt(a.x_nm.get() + ux * lo, a.y_nm.get() + uy * lo)?);
            }
            return Ok(Advance {
                points: kept,
                complete: false,
            });
        }
        Ok(Advance {
            points: kept,
            complete: true,
        })
    }
}
