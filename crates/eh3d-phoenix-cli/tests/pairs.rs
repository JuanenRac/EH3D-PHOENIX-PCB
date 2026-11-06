// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `pairs` and the pair operations of `boardedit` on files written by the tests.

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

/// A board with two copper layers and two nets whose tracks run side by side: SIG along y = 10 mm
/// and GND along y = 11 mm, both from x = 10 mm to x = 30 mm and 0.25 mm wide.
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
    for (track, net, y) in [(40, 10, 10), (41, 11, 11)] {
        board.tracks.insert(
            id(track),
            Track {
                net: id(net),
                layer: id(1),
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(10, y),
                    end: pt(30, y),
                }],
            },
        );
    }
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

fn pairs_file(f: &Files, gap_nm: i64) -> String {
    let text = json!({
        "format": "phoenix-diffpairs/1",
        "pairs": [{
            "name": "p", "positive": "SIG", "negative": "GND",
            "width_nm": 250000, "gap_nm": gap_nm
        }]
    });
    write(f.dir.path(), "pairs.json", &text.to_string())
}

#[test]
fn a_pair_that_runs_side_by_side_is_reported_without_error() {
    let f = files();
    let pairs = pairs_file(&f, 750_000);
    let (code, out, err) = call(&["pairs", "--pairs", &pairs, &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("pairs: 1, errors: 0, warnings: 0"), "{out}");
    assert!(out.contains("pair p (SIG / GND)"), "{out}");
    // nothing was written
    assert_eq!(std::fs::read_dir(f.dir.path()).unwrap().count(), 2);
}

#[test]
fn a_gap_out_of_tolerance_is_an_error_with_its_code_in_text_and_in_json() {
    let f = files();
    let pairs = pairs_file(&f, 300_000);
    let (code, out, _) = call(&["pairs", "--pairs", &pairs, &f.board]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-PAIR-003"), "{out}");
    let (code, out, _) = call(&["pairs", "--json", "--pairs", &pairs, &f.board]);
    assert_eq!(code, 1);
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(value["schema"], "phoenix-pairs/1");
    assert_eq!(value["errors"], 1);
    assert_eq!(value["findings"][0]["code"], "PHOENIX-PAIR-003");
    assert_eq!(value["pairs"][0]["min_gap_nm"], 750_000);
    let (_, spanish, _) = call(&["pairs", "--lang", "es", "--pairs", &pairs, &f.board]);
    assert!(
        spanish.contains("La separación del par está fuera de tolerancia"),
        "{spanish}"
    );
}

#[test]
fn the_arguments_and_the_files_are_checked() {
    let f = files();
    for args in [
        vec!["pairs"],
        vec!["pairs", "--bogus", &f.board],
        vec!["pairs", "--lang", "xx", &f.board],
        vec!["pairs", "--pairs", "missing.json", &f.board],
    ] {
        let (code, _, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(!err.is_empty());
    }
    let bad = write(f.dir.path(), "bad.json", "{}");
    let (code, _, err) = call(&["pairs", "--pairs", &bad, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PAIR-001"), "{err}");
    let config = write(f.dir.path(), "pairs.cfg", "unknown = 1\n");
    let (code, _, err) = call(&["pairs", "--config", &config, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PAIR-001"), "{err}");
}

#[test]
fn a_pair_in_the_settings_file_is_checked_like_one_of_the_json_file() {
    let f = files();
    let config = write(
        f.dir.path(),
        "pairs.cfg",
        "pair.p.positive = SIG\npair.p.negative = GND\npair.p.width_nm = 250000\npair.p.gap_nm = 750000\n",
    );
    let (code, out, err) = call(&["pairs", "--config", &config, &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("pair p (SIG / GND)"), "{out}");
}

fn route_pair_op() -> serde_json::Value {
    json!({
        "op": "route_pair",
        "pair": "p",
        "layer": id(1).to_string(),
        "points": [
            {"x_nm": 10_000_000, "y_nm": 20_000_000},
            {"x_nm": 30_000_000, "y_nm": 20_000_000}
        ],
    })
}

#[test]
fn boardedit_draws_a_pair_when_it_is_given_the_pairs() {
    let f = files();
    let pairs = pairs_file(&f, 750_000);
    let ops = write(
        f.dir.path(),
        "ops.json",
        &json!([route_pair_op()]).to_string(),
    );
    let (code, out, err) = call(&["boardedit", "--pairs", &pairs, "--apply", &ops, &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("operations applied: 1"), "{out}");
    assert!(out.contains("tracks: 2 -> 4"), "{out}");
    // without the pairs the pair is not known
    let (code, _, err) = call(&["boardedit", "--apply", &ops, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PAIR-011"), "{err}");
}

#[test]
fn boardedit_tunes_and_corrects_a_pair_and_writes_a_new_board_that_the_check_accepts() {
    let f = files();
    let pairs = pairs_file(&f, 750_000);
    let ops = write(
        f.dir.path(),
        "ops.json",
        &json!([
            route_pair_op(),
            {"op": "tune_pair", "pair": "p"},
            {"op": "fix_pair_gap", "pair": "p"}
        ])
        .to_string(),
    );
    let out_path = f.dir.path().join("out.json");
    let out_str = out_path.to_str().unwrap();
    let (code, out, err) = call(&[
        "boardedit",
        "--pairs",
        &pairs,
        "--apply",
        &ops,
        "--out",
        out_str,
        &f.board,
    ]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("operations applied: 3"), "{out}");
    let (code, report, err) = call(&["pairs", "--pairs", &pairs, out_str]);
    assert_eq!(code, 0, "{err}{report}");
}
