// SPDX-License-Identifier: GPL-3.0-or-later
//! From a part of the design (`R5`, `U3`) to its data sheet: which part is chosen in a drawing,
//! how the shared panel «Attachments» is asked for the files of that part, which of them is shown
//! first and how the chosen attachment is linked to the part. No window: the panel is drawn by the
//! shared crate and the window only calls these.

use std::path::Path;

use eh3d_attachpanel::model::component_link;
use eh3d_attachpanel::{Item as Attached, LinkForm, Model, Piece, Request};
use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_schematic::{Id as SchId, Schematic};
use eh3d_project::AttachmentKind;

use crate::scene::DocRef;
use crate::workspace::{Doc, Workspace};

/// A part of a board or of a schematic as the attachments know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRef {
    /// The designator (`R5`).
    pub refdes: String,
    /// The value (`10k`), which may be empty.
    pub value: String,
}

impl PartRef {
    /// The designator and the value as the window writes them: `R5 (10k)`.
    pub fn label(&self) -> String {
        if self.value.trim().is_empty() {
            self.refdes.clone()
        } else {
            format!("{} ({})", self.refdes, self.value.trim())
        }
    }

    /// The part as the panel of the attachments asks for it: by designator and by value, so that
    /// a data sheet linked to the value (every `LM358`) is listed too.
    pub fn piece(&self) -> Piece {
        let value = self.value.trim();
        Piece {
            reference: Some(self.refdes.clone()),
            value: (!value.is_empty()).then(|| value.to_owned()),
            part_number: None,
            manufacturer: None,
        }
    }

    /// The request that links an attachment to the designator of this part; none for a part with
    /// no designator.
    pub fn link_request(&self, attachment: &str) -> Option<Request> {
        component_link(LinkForm::Designator, &self.refdes, "")
            .map(|link| Request::AddComponent(attachment.to_owned(), link))
    }
}

fn named(refdes: &str, value: &str) -> Option<PartRef> {
    let refdes = refdes.trim();
    (!refdes.is_empty()).then(|| PartRef {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
    })
}

/// The component of a board.
pub fn of_component(board: &Board, id: Id) -> Option<PartRef> {
    let component = board.components.get(&id)?;
    named(&component.refdes, &component.value)
}

/// The part of a schematic.
pub fn of_schematic_part(schematic: &Schematic, id: SchId) -> Option<PartRef> {
    let part = schematic.parts.get(&id)?;
    named(&part.refdes, &part.value)
}

/// The part that is selected in the drawing of the window: a component or one of its pads, a part
/// of a schematic or one of its pins. Anything else that is selected has no part.
pub fn of_workspace(workspace: &Workspace) -> Option<PartRef> {
    let index = workspace.viewer.selected? as usize;
    let object = workspace.viewer.scene.items.get(index)?.doc;
    match (&workspace.doc, object) {
        (Doc::Board(session), DocRef::Component(id)) => of_component(&session.board, id),
        (Doc::Board(session), DocRef::Pad(pad)) => of_component(&session.board, pad.component),
        (Doc::Schematic(session), DocRef::Part(id)) => of_schematic_part(&session.schematic, id),
        (Doc::Schematic(session), DocRef::Pin(pin)) => {
            of_schematic_part(&session.schematic, pin.part)
        }
        _ => None,
    }
}

/// Which of the attachments of a part is shown first: a PDF inside the project, else any file
/// inside it, else the first one (a link). The list comes with the most specific first.
pub fn first_sheet(listed: &[&Attached]) -> Option<String> {
    listed
        .iter()
        .find(|a| !a.is_link && a.kind == AttachmentKind::Pdf)
        .or_else(|| listed.iter().find(|a| !a.is_link))
        .or_else(|| listed.first())
        .map(|a| a.id.clone())
}

/// Asks the panel for the attachments of a part of the design of `project` and chooses the one
/// that is shown. Returns how many the part has.
pub fn show_sheets(model: &mut Model, project: Option<&Path>, part: &PartRef) -> usize {
    model.set_project(project);
    model.set_filter(Some(part.piece()));
    let (chosen, count) = {
        let listed = model.visible();
        (first_sheet(&listed), listed.len())
    };
    model.select(chosen.as_deref());
    count
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::{Duration, Instant};

    use eh3d_attachpanel::Program;
    use eh3d_phoenix_project::pcb_branch::{write_pcb, NewPcbProject, PcbParts};

    use super::*;
    use crate::load::{Document, Loaded};
    use crate::sample;
    use crate::settings::ViewerSettings;

    fn workspace(document: Document) -> Workspace {
        Workspace::new(
            Loaded {
                document,
                unread: 0,
                binary: None,
            },
            None,
            ViewerSettings::default(),
            Default::default(),
        )
        .unwrap()
    }

    fn select(workspace: &mut Workspace, wanted: impl Fn(DocRef) -> bool) {
        let index = workspace
            .viewer
            .scene
            .items
            .iter()
            .position(|item| wanted(item.doc))
            .unwrap();
        workspace.viewer.select_item(index as u32);
    }

    fn wait(model: &mut Model) {
        let until = Instant::now() + Duration::from_secs(60);
        while model.busy() {
            assert!(Instant::now() < until, "the panel did not finish");
            thread::sleep(Duration::from_millis(10));
            model.poll();
        }
    }

    #[test]
    fn a_component_a_pad_a_part_and_a_pin_name_their_part() {
        let mut board = workspace(Document::Board(Box::new(sample::board())));
        assert_eq!(of_workspace(&board), None);
        select(&mut board, |d| matches!(d, DocRef::Component(_)));
        let from_component = of_workspace(&board).unwrap();
        select(&mut board, |d| matches!(d, DocRef::Pad(_)));
        assert_eq!(of_workspace(&board).unwrap(), from_component);
        // a track is not a part
        select(&mut board, |d| matches!(d, DocRef::Track(_)));
        assert_eq!(of_workspace(&board), None);

        let mut schematic = workspace(Document::Schematic(Box::new(sample::schematic())));
        select(&mut schematic, |d| matches!(d, DocRef::Part(_)));
        let part = of_workspace(&schematic).unwrap();
        assert!(!part.refdes.is_empty());
        select(&mut schematic, |d| matches!(d, DocRef::Pin(_)));
        assert!(of_workspace(&schematic).is_some());
        select(&mut schematic, |d| matches!(d, DocRef::Wire(_)));
        assert_eq!(of_workspace(&schematic), None);
    }

    #[test]
    fn the_words_of_a_part_leave_out_an_empty_value() {
        let bare = PartRef {
            refdes: "U3".to_owned(),
            value: "  ".to_owned(),
        };
        assert_eq!(bare.label(), "U3");
        assert_eq!(bare.piece().value, None);
        let full = PartRef {
            refdes: "R5".to_owned(),
            value: "10k".to_owned(),
        };
        assert_eq!(full.label(), "R5 (10k)");
        assert_eq!(full.piece().reference.as_deref(), Some("R5"));
        assert_eq!(full.piece().value.as_deref(), Some("10k"));
        assert_eq!(named("  ", "10k"), None);
    }

    #[test]
    fn a_data_sheet_is_linked_to_a_part_and_opened_from_it() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("demo.eh3d");
        let parts = PcbParts {
            board: Some(sample::board().to_bytes().unwrap()),
            ..PcbParts::default()
        };
        write_pcb(&project, None, &parts, &NewPcbProject::default(), "0.0.0").unwrap();
        let sheet = dir.path().join("lm358.pdf");
        fs::write(&sheet, b"%PDF-1.4\n").unwrap();
        let note = dir.path().join("notes.txt");
        fs::write(&note, b"notes\n").unwrap();

        let part = PartRef {
            refdes: "U3".to_owned(),
            value: "LM358".to_owned(),
        };
        let mut model = Model::new(Program::new("pcb", "0.0.0"));
        assert_eq!(show_sheets(&mut model, Some(&project), &part), 0);
        for file in [&note, &sheet] {
            model.request(Request::AttachFile(file.clone()));
            wait(&mut model);
        }
        model.set_filter(None);
        let ids: Vec<String> = model.visible().iter().map(|a| a.id.clone()).collect();
        assert_eq!(ids.len(), 2);
        for id in &ids {
            model.request(part.link_request(id).unwrap());
            wait(&mut model);
        }
        model.select(None);

        assert_eq!(show_sheets(&mut model, Some(&project), &part), 2);
        assert_eq!(model.selected().unwrap().name, "lm358.pdf");
        let other = PartRef {
            refdes: "R5".to_owned(),
            value: "10k".to_owned(),
        };
        assert_eq!(show_sheets(&mut model, Some(&project), &other), 0);
        assert!(model.selected().is_none());
        // with no project there is nothing to show and nothing fails
        assert_eq!(show_sheets(&mut model, None, &part), 0);
    }
}
