// SPDX-License-Identifier: GPL-3.0-or-later
//! The window on a binary board of EAGLE made by hand for the tests: it opens with the report of
//! its conversion, and a project of the suite that imports it keeps the binary original.

#[path = "../../eh3d-phoenix-eaglebin/tests/builder/board.rs"]
mod builder;

use std::fs;

use eh3d_phoenix_project::pcb_branch::{read_pcb, write_pcb, NewPcbProject, PcbParts};
use eh3d_phoenix_viewer::load::{read_document, Document};
use eh3d_phoenix_viewer::settings::ViewerSettings;
use eh3d_phoenix_viewer::suite_menu::{Outcome, SuiteState};

#[test]
fn a_binary_board_opens_with_its_report_and_its_original_is_kept_in_a_project() {
    let folder = tempfile::tempdir().unwrap();
    let binary = folder.path().join("old.brd");
    let bytes = builder::BoardFile::new((5, 11)).bytes();
    fs::write(&binary, &bytes).unwrap();
    let settings = ViewerSettings::default();

    let loaded = read_document(&binary, &settings).unwrap();
    match &loaded.document {
        Document::Board(board) => assert_eq!(board.components.len(), 2),
        other => panic!("{other:?}"),
    }
    let report = loaded.binary.as_deref().unwrap();
    assert_eq!(report.version, "5.11");
    assert_eq!(report.read_of("element"), 2);

    let project = folder.path().join("demo.eh3d");
    let named = NewPcbProject {
        name: "demo".to_owned(),
        ..NewPcbProject::default()
    };
    write_pcb(&project, None, &PcbParts::default(), &named, "0.0.0").unwrap();
    let mut suite = SuiteState::default();
    suite.adopt(&project);
    assert_eq!(suite.import(&binary, None, &settings), Outcome::Done);
    let origin = read_pcb(&project).unwrap().origin.unwrap();
    assert_eq!(origin.bytes, bytes);
    assert_eq!(origin.format, "eagle-brd");
    assert_eq!(fs::read(&binary).unwrap(), bytes);

    // a damaged one is refused with the code of the reader
    fs::write(&binary, &bytes[..200]).unwrap();
    let error = read_document(&binary, &settings).unwrap_err();
    let message = error.message(eh3d_phoenix_rules::Lang::En);
    assert!(message.contains("PHOENIX-EAGLEBIN-004"), "{message}");
}
