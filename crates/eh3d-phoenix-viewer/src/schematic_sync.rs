// SPDX-License-Identifier: GPL-3.0-or-later
//! How the schematic editor hands its schematic to the workspace. The editor
//! (`eh3d-phoenix-schedit`) keeps its own steps for undo and redo; after each change the window
//! gives the workspace the schematic as it is now, so that the list of findings, the transfer to
//! the board, the drawing of the viewer and the saving all work on what was edited.

use eh3d_phoenix_schematic::Schematic;

use crate::workspace::Workspace;

impl Workspace {
    /// Puts an edited schematic in the place of the one that is open (nothing happens when the
    /// open document is a board). The rule check is run again, a package that was ready is thrown
    /// away, the drawing is made again and the document counts as changed.
    pub fn replace_schematic(&mut self, schematic: Schematic) {
        let Some(session) = self.schematic_session_mut() else {
            return;
        };
        session.replace_schematic(schematic);
        self.rebuild();
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_erc::{self as erc, ErcConfig};
    use eh3d_phoenix_route::RouteSettings;
    use eh3d_phoenix_schedit::{SchEdit, SchEditSettings};
    use eh3d_phoenix_schematic::Item;

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
    fn an_edited_schematic_replaces_the_open_one_and_the_session_counts_as_changed() {
        let mut w = workspace(Document::Schematic(Box::new(sample::schematic())));
        assert!(!w.dirty());
        let original = w.schematic_session().unwrap().schematic.clone();
        let part = *original.parts.keys().next().unwrap();
        let mut edit = SchEdit::new(
            original.clone(),
            SchEditSettings::default(),
            ErcConfig::default(),
        );
        edit.selection.insert(Item::Part(part));
        edit.set_value("12k").unwrap();
        assert_eq!(edit.revision(), 1);
        w.replace_schematic(edit.schematic().clone());
        let session = w.schematic_session().unwrap();
        assert_eq!(session.schematic.parts[&part].value, "12k");
        assert_eq!(
            session.erc,
            erc::check(&session.schematic, &ErcConfig::default())
        );
        assert!(w.dirty());
        assert!(!w.can_undo());
    }

    #[test]
    fn a_board_ignores_a_schematic() {
        let mut w = workspace(Document::Board(Box::new(sample::board())));
        w.replace_schematic(sample::schematic());
        assert!(w.is_board());
        assert!(!w.dirty());
    }
}
