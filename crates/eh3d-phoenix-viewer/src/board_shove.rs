// SPDX-License-Identifier: GPL-3.0-or-later
//! The push tool of the board editor, without a screen: the net that is chosen, the points of the
//! new track that were clicked, the preview of the tracks that the push would move (computed by
//! the editor of `eh3d-phoenix-boardedit` without changing the board), and the acceptance of the
//! push, which goes through the editor like any operation, so the design rules are checked before
//! and after. The window draws this state in `board_shove_gui`, behind the `gui` feature.

use eh3d_phoenix_board::Id;
use eh3d_phoenix_boardedit::codes::{BoardEditError, OPERATION_REFUSED};
use eh3d_phoenix_boardedit::{EditOp, EditState, Shape, ShovePreview};
use eh3d_phoenix_model::Point;

/// The state of the push tool.
#[derive(Debug, Default)]
pub struct ShoveTool {
    /// Whether a click puts a point of the new track.
    pub on: bool,
    /// The place of the chosen net in the list of the nets, sorted by name.
    pub selected: usize,
    /// The points of the new track that were clicked.
    pub points: Vec<Point>,
    /// What the push would do, when there are two points or more and the push can be done.
    pub preview: Option<ShovePreview>,
}

impl ShoveTool {
    /// The nets of the board with their names, sorted by name.
    pub fn nets(state: &EditState) -> Vec<(Id, String)> {
        let mut nets: Vec<(Id, String)> = state
            .board()
            .nets
            .iter()
            .map(|(id, net)| (*id, net.name.clone()))
            .collect();
        nets.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        nets
    }

    /// The chosen net; the last one when the choice is past the end of the list.
    pub fn selected_net(&self, state: &EditState) -> Option<Id> {
        let nets = Self::nets(state);
        let last = nets.len().checked_sub(1)?;
        nets.get(self.selected.min(last)).map(|(id, _)| *id)
    }

    /// Puts a point of the new track (on the grid when snapping is on); the same point twice in a
    /// row counts once. The preview is made again.
    pub fn click(&mut self, state: &mut EditState, at: Point) {
        let at = state.snap(at);
        if self.points.last() != Some(&at) {
            self.points.push(at);
        }
        self.refresh(state);
    }

    /// Drops the points that were clicked and the preview.
    pub fn clear(&mut self) {
        self.points.clear();
        self.preview = None;
    }

    /// Drops the last point that was clicked.
    pub fn remove_last(&mut self, state: &mut EditState) {
        self.points.pop();
        self.refresh(state);
    }

    fn refuse(state: &mut EditState, what: &str) -> BoardEditError {
        let error = BoardEditError::new(OPERATION_REFUSED, what);
        state.error = Some(error.clone());
        error
    }

    /// The operation that routes the points with a push, on the active layer.
    pub fn operation(&self, state: &mut EditState) -> Result<EditOp, BoardEditError> {
        let Some(net) = self.selected_net(state) else {
            return Err(Self::refuse(state, "the board has no net"));
        };
        if self.points.len() < 2 {
            return Err(Self::refuse(state, "click at least two points"));
        }
        let Some(layer) = state.active_layer else {
            return Err(Self::refuse(state, "the board has no copper layer"));
        };
        Ok(EditOp::RouteShove {
            net,
            layer,
            points: self.points.clone(),
            shape: Some(if state.diagonal {
                Shape::Diagonal
            } else {
                Shape::Orthogonal
            }),
            width_nm: None,
            horizontal_first: Some(state.horizontal_first),
        })
    }

    /// Makes the preview again: what the push would move, or why it cannot be done (kept in the
    /// error of the state). Nothing is changed on the board.
    pub fn refresh(&mut self, state: &mut EditState) {
        self.preview = None;
        if self.points.len() < 2 {
            return;
        }
        let Ok(op) = self.operation(state) else {
            return;
        };
        match state.edit.preview_shove(&op) {
            Ok(preview) => {
                state.error = None;
                self.preview = Some(preview);
            }
            Err(e) => state.error = Some(e),
        }
    }

    /// How many tracks the push would move; zero without a preview.
    pub fn moved(&self) -> usize {
        self.preview.as_ref().map_or(0, |p| p.moves.len())
    }

    /// Accepts the push: the track is routed and the neighbours move, as one step of the editor.
    /// The points and the preview are dropped when it was done.
    pub fn accept(&mut self, state: &mut EditState) -> bool {
        let Ok(op) = self.operation(state) else {
            return false;
        };
        let done = state.run(&op);
        if done {
            self.clear();
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings};
    use eh3d_phoenix_route::RuleContext;

    use super::*;
    use crate::sample;

    /// The editor of the sample board, with no snapping so that the points land where they are
    /// given, and without the live check (the rules are off, the sample is a small board).
    fn state() -> EditState {
        let settings = BoardEditSettings {
            live_drc: false,
            snap: false,
            ..BoardEditSettings::default()
        };
        let edit = BoardEdit::new(sample::board(), RuleContext::default(), settings).unwrap();
        EditState::new(edit)
    }

    /// The tool with VCC chosen: the nets are GND and VCC, sorted by name.
    fn vcc_tool() -> ShoveTool {
        ShoveTool {
            selected: 1,
            ..ShoveTool::default()
        }
    }

    #[test]
    fn the_nets_are_sorted_by_name_and_the_choice_stays_inside_the_list() {
        let s = state();
        let names: Vec<String> = ShoveTool::nets(&s).into_iter().map(|(_, n)| n).collect();
        assert_eq!(names, ["GND", "VCC"]);
        let mut tool = ShoveTool::default();
        assert_eq!(tool.selected_net(&s), Some(sample::id(10)));
        tool.selected = 1;
        assert_eq!(tool.selected_net(&s), Some(sample::id(11)));
        tool.selected = 99;
        assert_eq!(tool.selected_net(&s), Some(sample::id(11)));
    }

    #[test]
    fn a_click_snaps_to_the_grid_and_the_same_point_twice_counts_once() {
        let edit = BoardEdit::new(
            sample::board(),
            RuleContext::default(),
            BoardEditSettings::default(),
        )
        .unwrap();
        let mut s = EditState::new(edit);
        let mut tool = ShoveTool::default();
        tool.click(&mut s, sample::point(5.04, 15.1));
        tool.click(&mut s, sample::point(5.0, 15.0));
        assert_eq!(tool.points, vec![sample::point(5.0, 15.0)]);
        assert!(tool.preview.is_none());
        tool.click(&mut s, sample::point(8.0, 15.0));
        assert_eq!(tool.points.len(), 2);
        tool.remove_last(&mut s);
        assert_eq!(tool.points.len(), 1);
        tool.clear();
        assert!(tool.points.is_empty() && tool.preview.is_none());
    }

    #[test]
    fn the_preview_shows_the_track_that_would_move_and_the_push_is_accepted_as_one_step() {
        let mut s = state();
        let mut tool = vcc_tool();
        let tracks = s.board().tracks.len();
        let before = s.board().to_bytes().unwrap();
        // the ground track runs along y = 5 from x = 5 to x = 9; the new track passes 0.2 mm above
        tool.click(&mut s, sample::point(3.0, 5.2));
        assert!(tool.preview.is_none(), "one point has no preview");
        tool.click(&mut s, sample::point(11.0, 5.2));
        assert!(s.error.is_none(), "{:?}", s.error);
        let preview = tool.preview.as_ref().unwrap();
        assert_eq!(tool.moved(), 1);
        assert_eq!(preview.moves[0].track, sample::id(40));
        assert_eq!(preview.moves[0].after.len(), 1);
        // the preview changed nothing
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert!(tool.accept(&mut s), "{:?}", s.error);
        assert_eq!(s.board().tracks.len(), tracks + 1);
        assert_eq!(s.board().tracks[&sample::id(40)].path.len(), 1);
        assert!(tool.points.is_empty() && tool.preview.is_none());
        // one step to undo
        assert!(s.undo());
        assert_eq!(s.board().to_bytes().unwrap(), before);
    }

    #[test]
    fn what_cannot_be_done_is_said_and_changes_nothing() {
        let mut s = state();
        let before = s.board().to_bytes().unwrap();
        let mut tool = vcc_tool();
        // no points
        assert!(!tool.accept(&mut s));
        assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
        // a track across the ground track cannot be pushed aside
        tool.click(&mut s, sample::point(7.0, 3.0));
        tool.click(&mut s, sample::point(7.0, 7.0));
        assert!(tool.preview.is_none());
        assert_eq!(
            s.error.as_ref().unwrap().code,
            eh3d_phoenix_boardedit::shove_codes::SHOVE_CROSSING
        );
        assert!(!tool.accept(&mut s));
        assert_eq!(s.board().to_bytes().unwrap(), before);
    }
}
