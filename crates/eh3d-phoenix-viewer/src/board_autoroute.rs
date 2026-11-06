// SPDX-License-Identifier: GPL-3.0-or-later
//! The autorouter in the board editor, without a screen: the plan that the autorouter
//! (`eh3d-phoenix-autoroute`) makes for the board as it is now, kept as a preview until it is
//! accepted or dropped, and its acceptance, which goes through the editor like any operation: one
//! step of the history, checked by the design rules before and after. A preview belongs to one
//! state of the board: as soon as the board changes it is no longer offered. The window draws
//! this state in `board_autoroute_gui`, behind the `gui` feature.

use eh3d_phoenix_autoroute::{autoroute, AutorouteConfig, AutorouteError, AutoroutePlan, Request};
use eh3d_phoenix_boardedit::EditState;

/// The settings file of the autorouter.
const CONFIG_PATH: &str = "config/autoroute_native.cfg";

/// The settings of the autorouter: `config/autoroute_native.cfg` when it is there and good, else
/// the built-in ones.
pub fn load_config() -> AutorouteConfig {
    let small = std::fs::metadata(CONFIG_PATH).is_ok_and(|m| m.len() <= 1 << 16);
    if !small {
        return AutorouteConfig::default();
    }
    std::fs::read_to_string(CONFIG_PATH)
        .ok()
        .and_then(|text| AutorouteConfig::parse(&text).ok())
        .unwrap_or_default()
}

/// The state of the autorouter in the editor.
#[derive(Debug, Default)]
pub struct AutorouteTool {
    plan: Option<(u64, AutoroutePlan)>,
    /// Why the last run could not be made, when it could not.
    pub error: Option<AutorouteError>,
    config: AutorouteConfig,
}

impl AutorouteTool {
    /// The tool with these settings.
    pub fn with_config(config: AutorouteConfig) -> AutorouteTool {
        AutorouteTool {
            plan: None,
            error: None,
            config,
        }
    }

    /// Routes the open connections of the board as it is now and keeps the plan as a preview.
    /// Nothing is changed on the board.
    pub fn preview(&mut self, state: &EditState) {
        let route = state.edit.settings.route_settings();
        let result = autoroute(
            state.board(),
            state.edit.context(),
            &route,
            &self.config,
            &Request::default(),
        );
        match result {
            Ok(plan) => {
                self.plan = Some((state.edit.revision(), plan));
                self.error = None;
            }
            Err(error) => {
                self.plan = None;
                self.error = Some(error);
            }
        }
    }

    /// The preview, when the board is still as it was when it was made.
    pub fn current(&self, state: &EditState) -> Option<&AutoroutePlan> {
        match &self.plan {
            Some((revision, plan)) if *revision == state.edit.revision() => Some(plan),
            _ => None,
        }
    }

    /// Drops the preview and the last error.
    pub fn clear(&mut self) {
        self.plan = None;
        self.error = None;
    }

    /// Accepts the preview: the tracks and vias are added as one step of the editor. The preview
    /// is dropped when it was done.
    pub fn accept(&mut self, state: &mut EditState) -> bool {
        let op = match self.current(state) {
            Some(plan) => plan.edit_ops(),
            None => return false,
        };
        let done = state.run(&op);
        if done {
            self.clear();
        }
        done
    }

    /// The key of the tool: with Shift the preview is accepted, without it the preview is made.
    pub fn key(&mut self, state: &mut EditState, shift: bool) {
        if shift {
            self.accept(state);
        } else {
            self.preview(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_board::{Component, PadNet, Side};
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp};
    use eh3d_phoenix_model::Udeg;
    use eh3d_phoenix_route::RuleContext;

    use super::*;
    use crate::sample;

    /// The sample board with a second component whose pads are on the same nets, so that
    /// connections are open.
    fn state() -> EditState {
        let mut board = sample::board();
        let first = board.components[&sample::id(30)].clone();
        board.components.insert(
            sample::id(31),
            Component {
                refdes: "R2".to_string(),
                at: sample::point(22.0, 6.0),
                rotation: Udeg::ZERO,
                side: Side::Top,
                ..first
            },
        );
        board.pad_nets.push(PadNet {
            component: sample::id(31),
            pad: sample::id(21),
            net: sample::id(10),
        });
        board.pad_nets.push(PadNet {
            component: sample::id(31),
            pad: sample::id(22),
            net: sample::id(11),
        });
        board.pad_nets.sort_by_key(PadNet::pad_ref);
        let settings = BoardEditSettings {
            live_drc: false,
            snap: false,
            ..BoardEditSettings::default()
        };
        let edit = BoardEdit::new(board, RuleContext::default(), settings).unwrap();
        EditState::new(edit)
    }

    #[test]
    fn nothing_is_offered_before_the_autorouter_has_run() {
        let s = state();
        let mut tool = AutorouteTool::default();
        assert!(tool.current(&s).is_none());
        assert!(!tool.accept(&mut state()));
        tool.clear();
        assert!(tool.error.is_none());
    }

    #[test]
    fn the_preview_changes_nothing_and_is_dropped_when_the_board_changes() {
        let mut s = state();
        let before = s.board().to_bytes().unwrap();
        let mut tool = AutorouteTool::with_config(AutorouteConfig::default());
        tool.preview(&s);
        assert_eq!(s.board().to_bytes().unwrap(), before);
        let plan = tool.current(&s).expect("a plan for the open connections");
        assert!(plan.report.connections_total > 0);
        // an edit of the board makes the preview stale
        let edit = EditOp::Route {
            net: sample::id(10),
            layer: sample::id(1),
            points: vec![sample::point(3.0, 3.0), sample::point(8.0, 3.0)],
            shape: None,
            width_nm: None,
            horizontal_first: None,
        };
        assert!(s.run(&edit), "{:?}", s.error);
        assert!(tool.current(&s).is_none());
        assert!(!tool.accept(&mut s));
        tool.clear();
        assert!(tool.error.is_none());
    }

    #[test]
    fn accepting_the_preview_is_one_step_that_can_be_undone() {
        let mut s = state();
        let before = s.board().to_bytes().unwrap();
        let mut tool = AutorouteTool::with_config(AutorouteConfig::default());
        tool.key(&mut s, false);
        let added = {
            let plan = tool.current(&s).expect("a plan");
            plan.tracks.len() + plan.vias.len()
        };
        let steps = s.edit.steps();
        tool.key(&mut s, true);
        // the editor may refuse a plan; when it took it, it is one step and it can be undone
        if tool.current(&s).is_none() && added > 0 {
            assert_eq!(s.edit.steps(), steps + 1, "{:?}", s.error);
            assert!(s.undo());
            assert_eq!(s.board().to_bytes().unwrap(), before);
        }
    }

    #[test]
    fn a_board_that_cannot_be_routed_is_an_error_with_a_code() {
        let mut board = sample::board();
        board.outline.clear();
        let edit =
            BoardEdit::new(board, RuleContext::default(), BoardEditSettings::default()).unwrap();
        let s = EditState::new(edit);
        let mut tool = AutorouteTool::default();
        tool.preview(&s);
        assert!(tool.current(&s).is_none());
        assert_eq!(
            tool.error.as_ref().map(|e| e.code),
            Some(eh3d_phoenix_autoroute::codes::BOARD_NOT_ROUTABLE)
        );
    }

    #[test]
    fn the_shipped_settings_are_the_default() {
        assert_eq!(
            AutorouteConfig::parse(include_str!("../../../config/autoroute_native.cfg")).unwrap(),
            AutorouteConfig::default()
        );
    }
}
