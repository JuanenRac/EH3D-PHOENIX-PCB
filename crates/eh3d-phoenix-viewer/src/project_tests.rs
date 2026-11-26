// SPDX-License-Identifier: GPL-3.0-or-later
//! The window opens a board or a schematic of an `.eh3d` project and saves it into one.

use std::fs;

use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_erc::ErcConfig;
use eh3d_phoenix_project::pcb_branch::read_pcb;
use eh3d_phoenix_route::{RouteSettings, RuleContext};
use eh3d_phoenix_sync::SyncSettings;

use crate::codes::SAVE_REFUSED;
use crate::load::{read_document, Document};
use crate::sample;
use crate::session::{BoardSession, SchematicSession};
use crate::settings::ViewerSettings;

fn session() -> BoardSession {
    BoardSession::new(
        sample::board(),
        RuleContext::default(),
        RouteSettings::default(),
    )
    .unwrap()
}

#[test]
fn a_board_is_saved_into_a_project_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.eh3d");
    let mut board = session();
    let saved = board.save_as(&path, None, ".bak").unwrap();
    assert_eq!(saved.backup, None);
    let loaded = read_document(&path, &ViewerSettings::default()).unwrap();
    assert_eq!(loaded.document, Document::Board(Box::new(sample::board())));
    // saving again replaces it and keeps a copy of what was there
    let again = board.save_as(&path, None, ".bak").unwrap();
    assert!(again.backup.is_some_and(|b| b.is_file()));
    // the input is never written, and a file of another hand is not replaced
    let refused = session().save_as(&path, Some(&path), ".bak").unwrap_err();
    assert_eq!(refused.code, SAVE_REFUSED);
    let refused = session().save_as(&path, None, ".bak").unwrap_err();
    assert_eq!(refused.code, SAVE_REFUSED);
}

#[test]
fn an_eagle_input_is_kept_in_a_new_project() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.brd");
    let text = write_board(&sample::board()).text;
    fs::write(&input, &text).unwrap();
    let path = dir.path().join("out.eh3d");
    session().save_as(&path, Some(&input), ".bak").unwrap();
    let origin = read_pcb(&path).unwrap().origin.unwrap();
    assert_eq!(
        (origin.name.as_str(), origin.format.as_str()),
        ("in.brd", "eagle-brd")
    );
    assert_eq!(origin.bytes, text.as_bytes());
}

#[test]
fn a_schematic_joins_the_board_in_the_same_project() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.eh3d");
    session().save_as(&first, None, ".bak").unwrap();
    let second = dir.path().join("second.eh3d");
    let mut schematic = SchematicSession::new(
        sample::schematic(),
        ErcConfig::default(),
        SyncSettings::default(),
    )
    .unwrap();
    schematic
        .save_schematic_as(&second, Some(&first), ".bak")
        .unwrap();
    let parts = read_pcb(&second).unwrap();
    assert!(parts.board.is_some() && parts.schematic.is_some());
    assert!(read_pcb(&first).unwrap().schematic.is_none());
}
