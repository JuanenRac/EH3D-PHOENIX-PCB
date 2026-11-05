// SPDX-License-Identifier: GPL-3.0-or-later
//! How the board editor hands its board to the workspace. The editor (`eh3d-phoenix-boardedit`)
//! keeps its own steps for undo and redo; after each change the window gives the workspace the
//! board as it is now, so that the list of findings, the ratsnest, the drawing of the viewer and
//! the saving all work on what was edited.

use eh3d_phoenix_board::Board;

use crate::workspace::Workspace;

impl Workspace {
    /// Puts an edited board in the place of the one that is open (nothing happens when the open
    /// document is a schematic). The rules are checked again, the ratsnest is made again, the
    /// drawing is made again and the document counts as changed.
    pub fn replace_board(&mut self, board: Board) {
        let Some(session) = self.board_session_mut() else {
            return;
        };
        session.replace_board(board);
        self.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp};
    use eh3d_phoenix_route::{RouteSettings, RuleContext};

    use crate::load::{Document, Loaded};
    use crate::sample;
    use crate::settings::ViewerSettings;
    use crate::workspace::Workspace;

    fn workspace(document: Document) -> Workspace {
        let loaded = Loaded {
            document,
            unread: 0,
        };
        Workspace::new(
            loaded,
            None,
            ViewerSettings::default(),
            RouteSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn an_edited_board_replaces_the_open_one_and_the_session_counts_as_changed() {
        let mut w = workspace(Document::Board(Box::new(sample::board())));
        assert!(!w.dirty());
        let original = w.board_session().unwrap().board.clone();
        let mut edit = BoardEdit::new(
            original.clone(),
            RuleContext::default(),
            BoardEditSettings::default(),
        )
        .unwrap();
        edit.apply(&EditOp::Via {
            net: sample::id(10),
            at: sample::point(20.0, 15.0),
            from_layer: None,
            to_layer: None,
        })
        .unwrap();
        w.replace_board(edit.board().clone());
        let session = w.board_session().unwrap();
        assert_eq!(session.board.vias.len(), original.vias.len() + 1);
        assert!(w.dirty());
        assert!(!w.can_undo());
    }

    #[test]
    fn a_schematic_ignores_a_board() {
        let mut w = workspace(Document::Schematic(Box::new(sample::schematic())));
        w.replace_board(sample::board());
        assert!(!w.is_board());
        assert!(!w.dirty());
    }
}
