// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `si` on files written by the tests.

use std::path::Path;

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Side, Track};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point};

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x_mm: i64, y_nm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_nm).unwrap()
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

/// A board with two layers and two nets, A along y = 10 mm and B along y = 10.4 mm, both from
/// x = 10 mm to x = 30 mm and 0.2 mm wide.
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
    for (n, name) in [(10, "A"), (11, "B")] {
        board.nets.insert(
            id(n),
            Net {
                name: name.to_owned(),
            },
        );
    }
    for (track, net, y_nm) in [(40, 10, 10_000_000), (41, 11, 10_400_000)] {
        board.tracks.insert(
            id(track),
            Track {
                net: id(net),
                layer: id(1),
                width: Nm::new(200_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(10, y_nm),
                    end: pt(30, y_nm),
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

#[test]
fn the_crosstalk_of_two_tracks_side_by_side_is_reported_and_nothing_is_written() {
    let f = files();
    let (code, out, err) = call(&["si", "crosstalk", &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("crosstalk: 1 coupled pairs of nets"), "{out}");
    assert!(out.contains("A -> B on Top"), "{out}");
    assert!(out.contains("NEXT 4.707 %"), "{out}");
    assert_eq!(std::fs::read_dir(f.dir.path()).unwrap().count(), 1);
}

#[test]
fn the_json_has_the_rows_and_the_findings() {
    let f = files();
    let (code, out, _) = call(&["si", "crosstalk", "--json", &f.board]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(value["command"], "crosstalk");
    assert_eq!(value["crosstalk"]["rows"][0]["aggressor"], "A");
    assert_eq!(value["crosstalk"]["rows"][0]["victim"], "B");
    assert_eq!(value["crosstalk"]["rows"][0]["parallel_mm"], 20.0);
    assert!(value.get("nets").is_none());
}

#[test]
fn a_faster_edge_in_the_settings_is_an_error() {
    let f = files();
    let config = write(f.dir.path(), "si.cfg", "rise_time_ps = 200\n");
    let (code, out, _) = call(&["si", "crosstalk", "--config", &config, &f.board]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error: PHOENIX-SI-003"), "{out}");
    let (_, spanish, _) = call(&[
        "si",
        "crosstalk",
        "--lang",
        "es",
        "--config",
        &config,
        &f.board,
    ]);
    assert!(
        spanish.contains("La diafonía cercana supera el límite"),
        "{spanish}"
    );
}

#[test]
fn reflect_eye_and_check_run_on_the_same_board() {
    let f = files();
    let (code, out, err) = call(&["si", "reflect", "--net", "A", &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("net A (Top)"), "{out}");
    assert!(!out.contains("net B"), "{out}");
    let (code, out, err) = call(&["si", "eye", &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("eye A: 100 Mbit/s"), "{out}");
    assert!(out.contains("PRBS7 of 127 bits"), "{out}");
    let (code, out, err) = call(&["si", "check", &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("return path: 0 tracks looked at over copper"),
        "{out}"
    );
}

#[test]
fn the_report_is_written_only_to_a_new_file() {
    let f = files();
    let target = f.dir.path().join("report.json");
    let target = target.to_str().unwrap();
    let (code, out, err) = call(&["si", "crosstalk", "--out", target, &f.board]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("written: "), "{out}");
    let written = std::fs::read_to_string(target).unwrap();
    let value: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(value["command"], "crosstalk");
    let (code, _, err) = call(&["si", "crosstalk", "--out", target, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SI-015"), "{err}");
}

#[test]
fn the_arguments_and_the_files_are_checked() {
    let f = files();
    let (code, _, err) = call(&["si"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("phoenix si <crosstalk|reflect|eye|check>"),
        "{err}"
    );
    let (code, _, err) = call(&["si", "noise", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SI-014"), "{err}");
    let (code, _, err) = call(&["si", "eye", "--bogus", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("unknown option --bogus"), "{err}");
    let (code, _, err) = call(&["si", "eye"]);
    assert_eq!(code, 2);
    assert!(err.contains("give exactly one board file"), "{err}");
    let (code, _, err) = call(&["si", "eye", "no-such-board.json"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SI-016"), "{err}");
    let (code, _, err) = call(&["si", "eye", "--net", "MISSING", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SI-002"), "{err}");
    let (_, _, spanish) = call(&["si", "eye", "--lang", "es", "--net", "MISSING", &f.board]);
    assert!(spanish.contains("La red no está en la placa"), "{spanish}");
    let config = write(f.dir.path(), "bad.cfg", "prbs = 8\n");
    let (code, _, err) = call(&["si", "eye", "--config", &config, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SI-001"), "{err}");
}
