// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced checks in the live check of the editor: off by default, on by a setting.

use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::shape::Shape;
use crate::testutil::{fixture, strict_rules, Fixture};

/// A track of the ground net that joins no pad.
fn lone_ground_track(f: &Fixture) -> EditOp {
    EditOp::Route {
        net: f.gnd,
        layer: f.top,
        points: vec![f.pt(5, 14), f.pt(15, 20)],
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: Some(true),
    }
}

fn editor(f: &Fixture, advanced: bool) -> BoardEdit {
    let settings = BoardEditSettings {
        advanced_drc: advanced,
        ..BoardEditSettings::default()
    };
    BoardEdit::new(f.board.clone(), strict_rules(), settings).unwrap()
}

#[test]
fn the_advanced_checks_are_off_unless_the_setting_turns_them_on() {
    assert!(!BoardEditSettings::default().advanced_drc);
    let f = fixture();
    let mut off = editor(&f, false);
    let out = off.apply(&lone_ground_track(&f)).unwrap();
    assert!(out.changed);
    assert!(out
        .drc
        .added
        .iter()
        .all(|a| !a.code.starts_with("PHOENIX-DRC-")));
    assert!(off
        .findings()
        .iter()
        .all(|a| !a.code.starts_with("PHOENIX-DRC-")));
}

#[test]
fn with_the_setting_on_the_findings_they_add_and_remove_are_in_the_report() {
    let f = fixture();
    let baseline = editor(&f, false).findings().len();
    let mut on = editor(&f, true);
    // the fixture has nothing for the advanced checks to find
    assert_eq!(on.findings().len(), baseline);
    let out = on.apply(&lone_ground_track(&f)).unwrap();
    let added: Vec<&str> = out
        .drc
        .added
        .iter()
        .map(|a| a.code)
        .filter(|c| c.starts_with("PHOENIX-DRC-"))
        .collect();
    // a copper island that joins no pad, which is a warning and so does not refuse the operation
    assert_eq!(added, vec!["PHOENIX-DRC-009"]);
    assert!(!out.drc.adds_error());
    let with_track = on.findings().len();
    // undo takes it away again
    assert!(on.undo());
    assert_eq!(on.findings().len(), baseline);
    assert!(on.redo());
    assert_eq!(on.findings().len(), with_track);
    assert!(on.findings().iter().any(|a| a.code == "PHOENIX-DRC-009"));
}

#[test]
fn the_setting_is_read_from_the_settings_text() {
    let s = BoardEditSettings::parse("advanced_drc = true\n").unwrap();
    assert!(s.advanced_drc);
    assert!(s.live_drc);
}
