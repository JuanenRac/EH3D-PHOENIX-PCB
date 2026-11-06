// SPDX-License-Identifier: GPL-3.0-or-later
//! The pair tool of the board editor, without a screen: the pair that is chosen, the points of the
//! centre line that were clicked, the operations that draw the pair, make its two nets as long as
//! each other and correct its gap (all of them go through the editor of `eh3d-phoenix-boardedit`,
//! so the design rules are checked before and after), and the report of the checks of the pairs.
//! The window draws this state in `board_pair_gui`, behind the `gui` feature.

use eh3d_phoenix_boardedit::codes::BoardEditError;
use eh3d_phoenix_boardedit::pair_codes::PAIR_OPERATION_REFUSED;
use eh3d_phoenix_boardedit::{check_pairs, EditOp, EditState, MeanderSide, PairReport, Shape};
use eh3d_phoenix_model::Point;

/// The state of the pair tool.
#[derive(Debug, Default)]
pub struct PairTool {
    /// Whether a click puts a point of the centre line of a pair.
    pub on: bool,
    /// The place of the chosen pair in the list of the pairs.
    pub selected: usize,
    /// The points of the centre line that were clicked.
    pub points: Vec<Point>,
    /// The report of the last check of the pairs.
    pub report: Option<PairReport>,
}

impl PairTool {
    /// The names of the pairs of the editor, in order.
    pub fn pair_names(state: &EditState) -> Vec<String> {
        state
            .edit
            .pair_config()
            .pairs
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// The name of the chosen pair; the last one when the choice is past the end of the list.
    pub fn selected_name(&self, state: &EditState) -> Option<String> {
        let pairs = &state.edit.pair_config().pairs;
        let last = pairs.len().checked_sub(1)?;
        pairs.get(self.selected.min(last)).map(|p| p.name.clone())
    }

    /// Puts a point of the centre line (on the grid when snapping is on); the same point twice in a
    /// row counts once.
    pub fn click(&mut self, state: &EditState, at: Point) {
        let at = state.snap(at);
        if self.points.last() != Some(&at) {
            self.points.push(at);
        }
    }

    /// Drops the points that were clicked.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Drops the last point that was clicked.
    pub fn remove_last(&mut self) {
        self.points.pop();
    }

    fn refuse(state: &mut EditState, what: &str) -> bool {
        state.error = Some(BoardEditError::new(PAIR_OPERATION_REFUSED, what));
        false
    }

    /// Draws the chosen pair along the points that were clicked, on the active layer. The points
    /// are dropped when the pair was drawn.
    pub fn route(&mut self, state: &mut EditState) -> bool {
        let Some(pair) = self.selected_name(state) else {
            return Self::refuse(state, "no pair is defined");
        };
        if self.points.len() < 2 {
            return Self::refuse(state, "click at least two points of the centre line");
        }
        let Some(layer) = state.active_layer else {
            return Self::refuse(state, "the board has no copper layer");
        };
        let op = EditOp::RoutePair {
            pair,
            layer,
            points: self.points.clone(),
            shape: Some(if state.diagonal {
                Shape::Diagonal
            } else {
                Shape::Orthogonal
            }),
            width_nm: None,
            gap_nm: None,
            horizontal_first: None,
            layer_changes: Vec::new(),
        };
        let done = state.run(&op);
        if done {
            self.points.clear();
            self.check(state);
        }
        done
    }

    /// Makes the shorter net of the chosen pair as long as the other with meanders.
    pub fn tune(&mut self, state: &mut EditState) -> bool {
        let Some(pair) = self.selected_name(state) else {
            return Self::refuse(state, "no pair is defined");
        };
        let done = state.run(&EditOp::TunePair {
            pair,
            side: MeanderSide::Auto,
        });
        if done {
            self.check(state);
        }
        done
    }

    /// Moves the pieces of the chosen pair whose gap is out of tolerance.
    pub fn fix_gap(&mut self, state: &mut EditState) -> bool {
        let Some(pair) = self.selected_name(state) else {
            return Self::refuse(state, "no pair is defined");
        };
        let done = state.run(&EditOp::FixPairGap { pair });
        if done {
            self.check(state);
        }
        done
    }

    /// Checks the pairs on the board as it is now and keeps the report.
    pub fn check(&mut self, state: &mut EditState) {
        match check_pairs(state.board(), state.edit.pair_config(), None) {
            Ok(report) => {
                self.report = Some(report);
            }
            Err(e) => {
                self.report = None;
                state.error = Some(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, PairConfig};
    use eh3d_phoenix_route::RuleContext;

    use super::*;
    use crate::sample;

    /// The editor of the sample board, without the rule check (the sample is a small board of
    /// other tests), with a pair of its two nets.
    fn state(pairs: &str) -> EditState {
        let settings = BoardEditSettings {
            live_drc: false,
            ..BoardEditSettings::default()
        };
        let edit = BoardEdit::new(sample::board(), RuleContext::default(), settings)
            .unwrap()
            .with_pairs(PairConfig::parse(pairs).unwrap());
        EditState::new(edit)
    }

    const PAIR: &str = "pair.clk.positive = GND\npair.clk.negative = VCC\n\
                        pair.clk.width_nm = 200000\npair.clk.gap_nm = 200000\n";

    #[test]
    fn the_names_come_from_the_pairs_of_the_editor_and_the_choice_stays_inside_the_list() {
        let s = state(&format!(
            "{PAIR}pair.abc.positive = GND\npair.abc.negative = VCC\n"
        ));
        assert_eq!(PairTool::pair_names(&s), ["abc", "clk"]);
        let mut tool = PairTool::default();
        assert_eq!(tool.selected_name(&s).as_deref(), Some("abc"));
        tool.selected = 1;
        assert_eq!(tool.selected_name(&s).as_deref(), Some("clk"));
        tool.selected = 99;
        assert_eq!(tool.selected_name(&s).as_deref(), Some("clk"));
        assert_eq!(PairTool::default().selected_name(&state("")), None);
    }

    #[test]
    fn a_click_puts_a_point_on_the_grid_and_the_same_point_twice_counts_once() {
        let s = state(PAIR);
        let mut tool = PairTool::default();
        tool.click(&s, sample::point(5.04, 15.1));
        tool.click(&s, sample::point(5.0, 15.0));
        assert_eq!(tool.points, vec![sample::point(5.0, 15.0)]);
        tool.click(&s, sample::point(25.0, 15.0));
        assert_eq!(tool.points.len(), 2);
        tool.remove_last();
        assert_eq!(tool.points.len(), 1);
        tool.clear();
        assert!(tool.points.is_empty());
    }

    #[test]
    fn the_pair_is_drawn_along_the_points_and_they_are_dropped() {
        let mut s = state(PAIR);
        let mut tool = PairTool::default();
        let tracks = s.board().tracks.len();
        tool.click(&s, sample::point(5.0, 16.0));
        tool.click(&s, sample::point(25.0, 16.0));
        assert!(tool.route(&mut s), "{:?}", s.error);
        assert_eq!(s.board().tracks.len(), tracks + 2);
        assert!(tool.points.is_empty());
        assert!(s.error.is_none());
        // the check ran after the drawing
        assert!(tool.report.is_some());
        assert_eq!(tool.report.as_ref().unwrap().pairs.len(), 1);
        assert!(s.undo());
        assert_eq!(s.board().tracks.len(), tracks);
    }

    #[test]
    fn what_cannot_be_done_is_said_and_changes_nothing() {
        let mut s = state(PAIR);
        let mut tool = PairTool::default();
        let before = s.board().to_bytes().unwrap();
        // no point yet
        assert!(!tool.route(&mut s));
        assert_eq!(s.error.as_ref().unwrap().code, PAIR_OPERATION_REFUSED);
        // no pair
        let mut none = state("");
        assert!(!tool.tune(&mut none));
        assert!(!tool.fix_gap(&mut none));
        assert_eq!(none.error.as_ref().unwrap().code, PAIR_OPERATION_REFUSED);
        assert_eq!(s.board().to_bytes().unwrap(), before);
    }

    #[test]
    fn the_check_keeps_a_report_of_the_pairs() {
        let mut s = state(PAIR);
        let mut tool = PairTool::default();
        assert!(tool.report.is_none());
        tool.check(&mut s);
        let report = tool.report.as_ref().unwrap();
        assert_eq!(report.pairs.len(), 1);
        assert_eq!(report.pairs[0].name, "clk");
    }
}
