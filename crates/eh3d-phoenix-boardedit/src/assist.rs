// SPDX-License-Identifier: GPL-3.0-or-later
//! The help while a person draws a route or drags: what a route that stops at the first obstacle
//! (the advance) or that pushes the tracks of other nets would do if the next click were under the
//! pointer, and what the design rules would say about an operation before it is done. Nothing here
//! changes the board except the click itself.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::Id;
use eh3d_phoenix_model::Point;

use crate::codes::{BoardEditError, OPERATION_REFUSED};
use crate::edit::{BoardEdit, DrcReport};
use crate::hit::pad_near;
use crate::ops::EditOp;
use crate::shape::Shape;
use crate::shove::{ShoveMove, ShoveViaMove};
use crate::tool::{EditState, Tool};

/// What the route tool does with a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RouteMode {
    /// The corners are where the person clicks; the rules judge the route when it is finished.
    #[default]
    Free,
    /// The route goes toward the pointer up to the last place that breaks no rule.
    Stop,
    /// Each click lays the track to the pointer and pushes the tracks of other nets out of the way.
    Push,
}

impl RouteMode {
    /// Every mode, in the order of a list.
    pub const ALL: [RouteMode; 3] = [RouteMode::Free, RouteMode::Stop, RouteMode::Push];

    /// The mode after this one (the key that goes round the modes).
    pub fn next(self) -> RouteMode {
        match self {
            RouteMode::Free => RouteMode::Stop,
            RouteMode::Stop => RouteMode::Push,
            RouteMode::Push => RouteMode::Free,
        }
    }
}

/// What the next click would add to the route in progress.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RouteAssist {
    /// The corners of the new part, from the last point of the route.
    pub points: Vec<Point>,
    /// True when the new part reaches the pointer.
    pub complete: bool,
    /// The tracks that a push would move.
    pub moves: Vec<ShoveMove>,
    /// The vias that a push would move.
    pub via_moves: Vec<ShoveViaMove>,
    /// Why the push cannot be done.
    pub refusal: Option<BoardEditError>,
}

impl BoardEdit {
    /// What the rules would say about an operation, without doing it. The operation is applied to
    /// a copy of the editor with the check on and with new errors allowed, so that they are listed
    /// and not refused.
    pub fn trial(&self, op: &EditOp) -> Result<DrcReport, BoardEditError> {
        let mut copy = self.clone();
        if !copy.settings.live_drc {
            copy = BoardEdit::new(
                self.board().clone(),
                self.context().clone(),
                crate::settings::BoardEditSettings {
                    live_drc: true,
                    ..self.settings.clone()
                },
            )?;
        }
        copy.settings.reject_new_errors = false;
        copy.apply(op).map(|outcome| outcome.drc)
    }
}

impl EditState {
    /// The net, the layer and the last point of the route in progress.
    fn route_end(&self) -> Option<(Id, Id, Point)> {
        if self.tool != Tool::Route {
            return None;
        }
        Some((
            self.route_net?,
            self.active_layer?,
            *self.route_points.last()?,
        ))
    }

    /// Where a click at `p` takes the route: the centre of a pad of the net under the pointer
    /// (then true), else the point on the grid.
    pub fn route_target(&self, p: Point) -> (Point, bool) {
        let board = self.board();
        let pad = pad_near(board, p, self.edit.settings.pick_tolerance_nm)
            .filter(|pad| self.route_net.is_some() && board.net_of(*pad) == self.route_net);
        match pad.and_then(|pad| pad_position(board, pad)) {
            Some(at) => (at, true),
            None => (self.snap(p), false),
        }
    }

    fn push_op(&self, net: Id, layer: Id, from: Point, to: Point) -> EditOp {
        EditOp::RouteShove {
            net,
            layer,
            points: vec![from, to],
            shape: Some(if self.diagonal {
                Shape::Diagonal
            } else {
                Shape::Orthogonal
            }),
            width_nm: None,
            horizontal_first: Some(self.horizontal_first),
        }
    }

    /// What a click at `p` would add in a mode; `None` when no route is in progress, in the free
    /// mode (whose preview is [`EditState::route_preview`]) or when the pointer is on the last
    /// point. The board is not changed.
    pub fn route_assist(&self, p: Point, mode: RouteMode) -> Option<RouteAssist> {
        let (net, layer, from) = self.route_end()?;
        let (to, _) = self.route_target(p);
        if to == from {
            return None;
        }
        match mode {
            RouteMode::Free => None,
            RouteMode::Stop => {
                let shape = if self.diagonal {
                    Shape::Diagonal
                } else {
                    Shape::Orthogonal
                };
                Some(
                    match self.edit.advance_route_shaped(
                        net,
                        layer,
                        from,
                        to,
                        self.horizontal_first,
                        shape,
                    ) {
                        Ok(found) => RouteAssist {
                            points: found.points,
                            complete: found.complete,
                            ..RouteAssist::default()
                        },
                        Err(e) => RouteAssist {
                            points: vec![from],
                            refusal: Some(e),
                            ..RouteAssist::default()
                        },
                    },
                )
            }
            RouteMode::Push => Some(
                match self.edit.preview_shove(&self.push_op(net, layer, from, to)) {
                    Ok(preview) => {
                        let mut points = vec![from];
                        points.extend(preview.route.iter().map(|piece| piece.end()));
                        RouteAssist {
                            points,
                            complete: true,
                            moves: preview.moves,
                            via_moves: preview.vias,
                            refusal: None,
                        }
                    }
                    Err(e) => RouteAssist {
                        points: vec![from],
                        refusal: Some(e),
                        ..RouteAssist::default()
                    },
                },
            ),
        }
    }

    /// A click of the route tool at `p` in a mode. The first click of a route is the same in
    /// every mode. In the stop mode the clear part of the way is added to the route, and the
    /// route is finished when it reaches a pad of the net; in the push mode the track to the
    /// pointer is laid at once as one step and the route goes on from there. True when the click
    /// changed the route or the board.
    pub fn route_click(&mut self, p: Point, mode: RouteMode) -> bool {
        if mode == RouteMode::Free || self.route_end().is_none() {
            let before = (self.route_points.len(), self.edit.revision());
            self.pointer_pressed(p, false);
            return before != (self.route_points.len(), self.edit.revision());
        }
        let Some((net, layer, from)) = self.route_end() else {
            return false;
        };
        let (to, on_pad) = self.route_target(p);
        if to == from {
            return false;
        }
        if mode == RouteMode::Push {
            let op = self.push_op(net, layer, from, to);
            if !self.run(&op) {
                return false;
            }
            self.route_points.clear();
            if on_pad {
                self.route_net = None;
            } else {
                self.route_points.push(to);
            }
            return true;
        }
        let Some(found) = self.route_assist(p, RouteMode::Stop) else {
            return false;
        };
        if found.points.len() < 2 {
            self.error = Some(found.refusal.unwrap_or_else(|| {
                BoardEditError::new(OPERATION_REFUSED, "the way is blocked at its start")
            }));
            return false;
        }
        self.error = None;
        self.route_points.extend(found.points.into_iter().skip(1));
        if found.complete && on_pad {
            self.finish_route();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::BoardEditSettings;
    use crate::testutil::{fixture, strict_rules, Fixture};

    fn state(f: &Fixture) -> EditState {
        let edit = BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap();
        let mut state = EditState::new(edit);
        state.set_tool(Tool::Route);
        state
    }

    fn mm(x: i64, y: i64) -> Point {
        Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
    }

    /// A route of GND in progress from a point above the track of SIG, which runs along
    /// y = 10 mm.
    fn ground_route(f: &Fixture) -> EditState {
        let mut s = state(f);
        s.route_net = Some(f.gnd);
        s.route_points.push(mm(20, 20));
        s
    }

    #[test]
    fn the_modes_go_round() {
        let mut mode = RouteMode::default();
        for expected in RouteMode::ALL {
            assert_eq!(mode, expected);
            mode = mode.next();
        }
        assert_eq!(mode, RouteMode::Free);
    }

    #[test]
    fn nothing_is_proposed_without_a_route_in_progress() {
        let f = fixture();
        let s = state(&f);
        for mode in RouteMode::ALL {
            assert_eq!(s.route_assist(mm(20, 5), mode), None);
        }
        let s = ground_route(&f);
        assert_eq!(s.route_assist(mm(20, 20), RouteMode::Stop), None);
        assert_eq!(s.route_assist(mm(20, 5), RouteMode::Free), None);
    }

    #[test]
    fn the_stop_mode_stops_in_front_of_the_other_net_and_the_click_keeps_that_part() {
        let f = fixture();
        let mut s = ground_route(&f);
        let before = s.board().to_bytes().unwrap();
        let found = s.route_assist(mm(20, 5), RouteMode::Stop).unwrap();
        assert!(!found.complete);
        let last = *found.points.last().unwrap();
        assert!((10_449_000..=10_452_000).contains(&last.y_nm.get()));
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert!(s.route_click(mm(20, 5), RouteMode::Stop));
        assert_eq!(s.route_points, vec![mm(20, 20), last]);
        // the route is only in progress: finishing it is the step, and it breaks no rule
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert!(s.finish_route(), "{:?}", s.error);
        assert!(!s.last.as_ref().unwrap().drc.adds_error());
    }

    #[test]
    fn a_way_blocked_at_its_start_adds_nothing_and_says_so() {
        let f = fixture();
        let mut s = ground_route(&f);
        let near = Point::from_nm(20_000_000, 10_300_000).unwrap();
        s.edit.settings.snap = false;
        s.route_points = vec![near];
        assert!(!s.route_click(mm(20, 5), RouteMode::Stop));
        assert_eq!(s.route_points, vec![near]);
        assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    }

    #[test]
    fn the_push_mode_lays_each_click_as_one_step_and_goes_on_from_there() {
        let f = fixture();
        let mut s = state(&f);
        s.route_net = Some(f.gnd);
        s.route_points.push(mm(15, 20));
        let tracks = s.board().tracks.len();
        let found = s.route_assist(mm(25, 20), RouteMode::Push).unwrap();
        assert!(
            found.complete && found.refusal.is_none(),
            "{:?}",
            found.refusal
        );
        assert_eq!(found.points.last(), Some(&mm(25, 20)));
        assert_eq!(s.board().tracks.len(), tracks);
        assert!(s.route_click(mm(25, 20), RouteMode::Push), "{:?}", s.error);
        assert_eq!(s.board().tracks.len(), tracks + 1);
        assert_eq!(s.route_points, vec![mm(25, 20)]);
        assert!(s.undo());
        assert_eq!(s.board().tracks.len(), tracks);
    }

    #[test]
    fn a_push_that_cannot_be_done_is_said_and_changes_nothing() {
        let f = fixture();
        let mut s = ground_route(&f);
        let before = s.board().to_bytes().unwrap();
        // straight across the track of SIG, which a push cannot move aside
        let found = s.route_assist(mm(20, 5), RouteMode::Push).unwrap();
        assert!(found.refusal.is_some());
        assert!(!s.route_click(mm(20, 5), RouteMode::Push));
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert_eq!(s.route_points, vec![mm(20, 20)]);
    }

    #[test]
    fn the_trial_of_an_operation_lists_what_the_rules_would_say_and_changes_nothing() {
        let f = fixture();
        let s = state(&f);
        let before = s.board().to_bytes().unwrap();
        let op = EditOp::Route {
            net: f.gnd,
            layer: f.top,
            points: vec![mm(20, 20), mm(20, 5)],
            shape: Some(Shape::Diagonal),
            width_nm: None,
            horizontal_first: None,
        };
        let report = s.edit.trial(&op).unwrap();
        assert!(report.adds_error());
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert_eq!(s.edit.steps(), 0);
        // the same with the live check switched off in the settings
        let mut off = state(&f);
        off.edit.settings.live_drc = false;
        assert!(off.edit.trial(&op).unwrap().adds_error());
    }

    #[test]
    fn random_pointers_never_make_the_stop_mode_break_a_rule() {
        // a fixed seed: the same points on every run
        let f = fixture();
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move |span: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % span) as i64
        };
        for _ in 0..25 {
            let mut s = ground_route(&f);
            let to =
                Point::from_nm(1_000_000 + next(58_000_000), 1_000_000 + next(38_000_000)).unwrap();
            if s.route_click(to, RouteMode::Stop) && s.route_points.len() >= 2 {
                assert!(s.finish_route(), "{to:?}: {:?}", s.error);
                assert!(!s.last.as_ref().unwrap().drc.adds_error(), "{to:?}");
            }
        }
    }
}
