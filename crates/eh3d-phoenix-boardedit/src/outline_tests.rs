// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the operation `outline_from_dxf`: the outline from the contour of a drawing, the cutouts
//! listed and not kept, a drawing that is refused, the rules checked, undo and redo.

use eh3d_phoenix_dxf::{write_dxf, Drawing};
use eh3d_phoenix_model::Point;

use crate::codes::GEOMETRY_INVALID;
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, strict_rules};

const MM: i64 = 1_000_000;

fn editor() -> BoardEdit {
    BoardEdit::new(
        fixture().board,
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
}

fn rectangle(layer: &str, width_mm: i64, height_mm: i64) -> Drawing {
    let mut d = Drawing::new();
    d.polygon(
        layer,
        &[
            (0, 0),
            (width_mm * MM, 0),
            (width_mm * MM, height_mm * MM),
            (0, height_mm * MM),
        ],
    );
    d
}

fn from(drawing: &Drawing, layer: Option<&str>) -> EditOp {
    EditOp::OutlineFromDxf {
        dxf: write_dxf(drawing),
        layer: layer.map(str::to_owned),
        join_tolerance_nm: None,
        settings: None,
    }
}

#[test]
fn the_outline_is_taken_from_the_layer_and_one_undo_gives_the_old_one_back() {
    let mut e = editor();
    let before = e.board().to_bytes().unwrap();
    let mut drawing = rectangle("EDGE", 35, 25);
    // a bigger rectangle on another layer is not looked at
    drawing.polygon(
        "OTHER",
        &[(0, 0), (90 * MM, 0), (90 * MM, 90 * MM), (0, 90 * MM)],
    );
    let outcome = e.apply(&from(&drawing, Some("edge"))).unwrap();
    assert!(outcome.changed);
    assert!(!outcome.drc.adds_error());
    let p = |x: i64, y: i64| Point::from_nm(x * MM, y * MM).unwrap();
    assert_eq!(
        e.board().outline,
        vec![p(0, 0), p(35, 0), p(35, 25), p(0, 25)]
    );
    assert!(
        outcome.notes.iter().any(|n| n
            == "outline: 4 corners, 35.000 by 25.000 mm (it was 4 corners, 60.000 by 40.000 mm)"),
        "{:?}",
        outcome.notes
    );
    assert!(e.undo());
    assert_eq!(e.board().to_bytes().unwrap(), before);
    assert!(e.redo());
    assert_eq!(e.board().outline[2], p(35, 25));
}

#[test]
fn a_drawing_with_cutouts_gives_the_outline_and_lists_the_cutouts() {
    let mut e = editor();
    let mut drawing = rectangle("EDGE", 40, 30);
    drawing.circle("EDGE", (20 * MM, 20 * MM), 2 * MM);
    let outcome = e.apply(&from(&drawing, None)).unwrap();
    assert_eq!(e.board().outline.len(), 4);
    assert!(outcome
        .notes
        .iter()
        .any(|n| n.contains("PHOENIX-DXF-015") && n.contains("1 cutouts")));
}

#[test]
fn a_drawing_that_is_refused_leaves_the_board_as_it_was() {
    let mut e = editor();
    let before = e.board().to_bytes().unwrap();
    // a bow tie crosses itself
    let mut bow = Drawing::new();
    bow.polygon(
        "EDGE",
        &[(0, 0), (10 * MM, 10 * MM), (10 * MM, 0), (0, 5 * MM)],
    );
    let error = e.apply(&from(&bow, None)).unwrap_err();
    assert_eq!(error.code, GEOMETRY_INVALID);
    assert!(error.detail.contains("PHOENIX-DXF-013"), "{}", error.detail);
    // nothing on the layer asked for
    let error = e
        .apply(&from(&rectangle("EDGE", 10, 10), Some("OUTLINE")))
        .unwrap_err();
    assert_eq!(error.code, GEOMETRY_INVALID);
    assert!(error.detail.contains("PHOENIX-DXF-014"), "{}", error.detail);
    // text that is not DXF
    let error = e
        .apply(&EditOp::OutlineFromDxf {
            dxf: "not a drawing".to_owned(),
            layer: None,
            join_tolerance_nm: None,
            settings: None,
        })
        .unwrap_err();
    assert_eq!(error.code, GEOMETRY_INVALID);
    assert!(error.detail.contains("PHOENIX-DXF-003"), "{}", error.detail);
    // a tolerance below 0
    let error = e
        .apply(&EditOp::OutlineFromDxf {
            dxf: write_dxf(&rectangle("EDGE", 10, 10)),
            layer: None,
            join_tolerance_nm: Some(-1),
            settings: None,
        })
        .unwrap_err();
    assert_eq!(error.code, GEOMETRY_INVALID);
    // a settings text that is not valid
    let error = e
        .apply(&EditOp::OutlineFromDxf {
            dxf: write_dxf(&rectangle("EDGE", 10, 10)),
            layer: None,
            join_tolerance_nm: None,
            settings: Some("nope = 1".to_owned()),
        })
        .unwrap_err();
    assert_eq!(error.code, GEOMETRY_INVALID);
    assert!(error.detail.contains("PHOENIX-DXF-001"), "{}", error.detail);
    assert_eq!(e.board().to_bytes().unwrap(), before);
    assert!(!e.can_undo());
}

#[test]
fn the_outline_the_board_has_is_not_a_change() {
    let mut e = editor();
    let outcome = e.apply(&from(&rectangle("EDGE", 60, 40), None)).unwrap();
    assert!(!outcome.changed);
    assert!(!e.can_undo());
    assert!(outcome
        .notes
        .iter()
        .any(|n| n == "the outline is the one the board has"));
}

#[test]
fn the_operation_is_written_and_read_back_and_its_defaults_are_the_layers_and_the_setting() {
    let text = r#"{"op":"outline_from_dxf","dxf":"x"}"#;
    let op: EditOp = serde_json::from_str(text).unwrap();
    assert_eq!(
        op,
        EditOp::OutlineFromDxf {
            dxf: "x".to_owned(),
            layer: None,
            join_tolerance_nm: None,
            settings: None
        }
    );
    let full = EditOp::OutlineFromDxf {
        dxf: "y".to_owned(),
        layer: Some("EDGE".to_owned()),
        join_tolerance_nm: Some(5),
        settings: Some(
            "write_text = true
"
            .to_owned(),
        ),
    };
    let back = serde_json::to_string(&full).unwrap();
    assert_eq!(serde_json::from_str::<EditOp>(&back).unwrap(), full);
}

#[test]
fn it_goes_through_a_group_with_the_other_operations_all_or_nothing() {
    let mut e = editor();
    let before = e.board().to_bytes().unwrap();
    let good = from(&rectangle("EDGE", 35, 25), None);
    let bad = EditOp::Lock {
        id: crate::testutil::id(999),
    };
    assert!(e
        .apply(&EditOp::Group {
            ops: vec![good.clone(), bad]
        })
        .is_err());
    assert_eq!(e.board().to_bytes().unwrap(), before);
    e.apply(&EditOp::Group { ops: vec![good] }).unwrap();
    assert_eq!(e.board().outline.len(), 4);
    assert_eq!(e.steps(), 1);
}
