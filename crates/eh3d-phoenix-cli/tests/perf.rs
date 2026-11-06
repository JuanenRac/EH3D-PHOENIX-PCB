// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `perf` on boards written by the tests: the counts of comparisons with the index and
//! of every pair, the same report every time, the settings file and the refusals.

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use serde_json::Value;

const MM: i64 = 1_000_000;
const TRACKS: u32 = 300;

fn id(n: u32) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
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

/// Short tracks on a lattice, ten millimetres apart, in two nets.
fn board() -> Board {
    let mut board = Board::default();
    board.layers.insert(
        id(1),
        Layer {
            name: "Top".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.copper_stack = vec![id(1)];
    for k in 0..2_u32 {
        board.nets.insert(
            id(10 + k),
            Net {
                name: format!("N{k}"),
            },
        );
    }
    let columns = 18_i64;
    for k in 0..TRACKS {
        let (col, row) = (i64::from(k) % columns, i64::from(k) / columns);
        let (x, y) = (5 * MM + col * 10 * MM, 5 * MM + row * 10 * MM);
        board.tracks.insert(
            id(1000 + k),
            Track {
                net: id(10 + k % 2),
                layer: id(1),
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(x, y),
                    end: pt(x + MM, y),
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

impl Files {
    fn write(&self, name: &str, text: &str) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.to_string_lossy().into_owned()
    }
}

fn files() -> Files {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("in.json");
    std::fs::write(&path, board().to_bytes().unwrap()).unwrap();
    Files {
        board: path.to_string_lossy().into_owned(),
        dir,
    }
}

fn module<'a>(value: &'a Value, name: &str) -> &'a Value {
    value["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["module"] == name)
        .unwrap()
}

#[test]
fn the_report_counts_the_comparisons_with_the_index_and_of_every_pair() {
    let f = files();
    let (code, out, err) = call(&["perf", &f.board, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["format"], "phoenix-perf/1");
    assert_eq!(value["objects"]["tracks"], 300);
    assert_eq!(value["objects"]["nets"], 2);
    let clearance = module(&value, "clearance");
    assert_eq!(clearance["items"], 300);
    assert_eq!(clearance["comparisons_every_pair"], 90_000);
    let indexed = clearance["comparisons_indexed"].as_u64().unwrap();
    assert!(indexed <= 4 * 300, "{indexed}");
    assert_eq!(clearance["refused"], Value::Null);
    let ratsnest = module(&value, "ratsnest");
    assert_eq!(ratsnest["error"], Value::Null);
    assert_eq!(ratsnest["anchors"], 600);
    // nothing in the report is a time, so the same board gives the same bytes
    let (_, again, _) = call(&["perf", &f.board, "--json"]);
    assert_eq!(out, again);
}

#[test]
fn the_text_report_names_the_objects_and_the_modules() {
    let f = files();
    let (code, out, _) = call(&["perf", &f.board, "--clearance-mm", "0.3"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("objects: 300 tracks, 0 vias"), "{out}");
    assert!(
        out.contains("clearance: comparisons with the index"),
        "{out}"
    );
    assert!(out.contains("of every pair 90000"), "{out}");
    assert!(
        out.contains("ratsnest: comparisons with the index"),
        "{out}"
    );
}

#[test]
fn a_settings_file_that_cannot_hold_the_grid_makes_the_command_exit_with_1() {
    let f = files();
    let tight = f.write("tight.cfg", "cell_nm = 100000\nmax_cells = 10\n");
    let (code, out, err) = call(&["perf", &f.board, "--config", &tight, "--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("PHOENIX-SPATIAL-002"), "{err}");
    let value: Value = serde_json::from_str(&out).unwrap();
    let clearance = module(&value, "clearance");
    assert_eq!(clearance["refused"], "PHOENIX-SPATIAL-002");
    // the comparison of every pair stands in for the grid
    assert_eq!(clearance["comparisons_indexed"], 90_000);
}

#[test]
fn bad_arguments_and_files_are_refused_with_their_codes() {
    let f = files();
    let bad_key = f.write("bad.cfg", "cells = 3\n");
    let (code, _, err) = call(&["perf", &f.board, "--config", &bad_key]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SPATIAL-001"), "{err}");
    let (code, _, err) = call(&["perf", &f.board, "--nope"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SPATIAL-004"), "{err}");
    let (code, _, err) = call(&["perf", &f.board, "--clearance-mm", "0"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SPATIAL-004"), "{err}");
    let (code, _, err) = call(&["perf"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SPATIAL-004"), "{err}");
    let missing = f
        .dir
        .path()
        .join("missing.json")
        .to_string_lossy()
        .into_owned();
    let (code, _, err) = call(&["perf", &missing, "--lang", "pt"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SPATIAL-005"), "{err}");
    assert!(err.contains("ficheiro"), "{err}");
}
