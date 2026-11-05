// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `boardedit` on files written by the tests: a dry run, a new output file, the
//! refusals, the rules and the JSON report.

use std::path::Path;

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Side, Track};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use serde_json::json;

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
}

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// A board with two copper layers, two nets and one track of the signal net along y = 10 mm.
fn board() -> Board {
    let mut board = Board {
        name: "T".to_owned(),
        ..Board::default()
    };
    for (n, name, side) in [(1, "Top", Side::Top), (2, "Bottom", Side::Bottom)] {
        board.layers.insert(
            id(n),
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: Some(side),
            },
        );
    }
    board.copper_stack = vec![id(1), id(2)];
    for (n, name) in [(10, "SIG"), (11, "GND")] {
        board.nets.insert(
            id(n),
            Net {
                name: name.to_owned(),
            },
        );
    }
    board.tracks.insert(
        id(40),
        Track {
            net: id(10),
            layer: id(1),
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(10, 10),
                end: pt(30, 10),
            }],
        },
    );
    board
}

struct Files {
    dir: tempfile::TempDir,
    board: String,
}

fn files() -> Files {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("in.json");
    std::fs::write(&path, board().to_bytes().unwrap()).unwrap();
    Files {
        board: path.to_str().unwrap().to_owned(),
        dir,
    }
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn route_gnd(points: &[(i64, i64)]) -> serde_json::Value {
    json!({
        "op": "route",
        "net": id(11).to_string(),
        "layer": id(1).to_string(),
        "points": points
            .iter()
            .map(|(x, y)| json!({"x_nm": x * 1_000_000, "y_nm": y * 1_000_000}))
            .collect::<Vec<_>>(),
        "shape": "orthogonal",
        "horizontal_first": true,
    })
}

fn ops_file(f: &Files, ops: &[serde_json::Value]) -> String {
    write(f.dir.path(), "ops.json", &json!(ops).to_string())
}

#[test]
fn without_out_nothing_is_written_and_the_report_says_what_changed() {
    let f = files();
    let ops = ops_file(&f, &[route_gnd(&[(5, 14), (15, 14)])]);
    let (code, out, err) = call(&["boardedit", "--apply", &ops, &f.board]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("operations applied: 1"), "{out}");
    assert!(out.contains("tracks: 1 -> 2"), "{out}");
    assert!(out.contains("dry run"), "{out}");
    assert_eq!(std::fs::read_dir(f.dir.path()).unwrap().count(), 2);
}

#[test]
fn out_writes_a_new_board_that_is_read_back_and_never_a_file_that_exists() {
    let f = files();
    let ops = ops_file(&f, &[route_gnd(&[(5, 14), (15, 14)])]);
    let out_path = f.dir.path().join("out.json");
    let out_str = out_path.to_str().unwrap();
    let (code, out, err) = call(&["boardedit", "--apply", &ops, "--out", out_str, &f.board]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let written = Board::from_bytes(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(written.tracks.len(), 2);
    // the input is as it was
    let input = Board::from_bytes(&std::fs::read(&f.board).unwrap()).unwrap();
    assert_eq!(input.tracks.len(), 1);
    // a second run, the input as the output and a format that is not written are refused
    for target in [out_str, f.board.as_str(), "x.txt"] {
        let (code, _, err) = call(&["boardedit", "--apply", &ops, "--out", target, &f.board]);
        assert_eq!(code, 2, "{target}");
        assert!(err.contains("PHOENIX-BEDIT-003"), "{err}");
    }
}

#[test]
fn a_refused_operation_names_its_number_and_writes_nothing() {
    let f = files();
    let missing = json!({"op": "delete_track", "track": id(99).to_string()});
    let ops = ops_file(&f, &[route_gnd(&[(5, 14), (15, 14)]), missing]);
    let out_path = f.dir.path().join("out.json");
    let (code, _, err) = call(&[
        "boardedit",
        "--apply",
        &ops,
        "--out",
        out_path.to_str().unwrap(),
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BEDIT-004"), "{err}");
    assert!(err.contains("operation 2 was refused"), "{err}");
    assert!(!out_path.exists());
}

#[test]
fn an_operation_that_adds_a_rule_error_is_refused_unless_it_is_forced() {
    let f = files();
    let rules = write(
        f.dir.path(),
        "rules.json",
        r#"{"format":"phoenix-rules/1","rules":{"min_clearance_nm":200000}}"#,
    );
    let across = route_gnd(&[(20, 8), (20, 12)]);
    let ops = ops_file(&f, std::slice::from_ref(&across));
    let (code, _, err) = call(&["boardedit", "--rules", &rules, "--apply", &ops, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BEDIT-005"), "{err}");
    let forced = json!({"op": "forced", "with": across});
    let ops = ops_file(&f, &[forced]);
    let (code, out, err) = call(&[
        "boardedit",
        "--rules",
        &rules,
        "--json",
        "--apply",
        &ops,
        &f.board,
    ]);
    assert_eq!(code, 1, "{err}");
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(value["schema"], "phoenix-boardedit/1");
    assert_eq!(value["result"]["forced"], 1);
    assert!(value["rule_errors"].as_u64().unwrap() > 0);
    assert!(value["written"].is_null());
}

#[test]
fn the_arguments_are_checked() {
    let f = files();
    let ops = ops_file(&f, &[]);
    for args in [
        vec!["boardedit"],
        vec!["boardedit", "--apply", &ops],
        vec!["boardedit", "--bogus", &f.board],
        vec!["boardedit", "--lang", "xx", "--apply", &ops, &f.board],
    ] {
        let (code, _, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(!err.is_empty());
    }
    let bad = write(f.dir.path(), "bad.json", "{}");
    let (code, _, err) = call(&["boardedit", "--apply", &bad, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BEDIT-002"), "{err}");
}
