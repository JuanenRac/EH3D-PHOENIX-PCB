// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `dxf-import` and `dxf-export` on files written by the tests: a dry run, a new
//! output file, the refusals, the JSON reports and the exit codes.

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track, Via};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_dxf::{read_dxf, write_dxf, Drawing, DxfSettings, Shape};
use eh3d_phoenix_model::{Nm, ObjectId, Point};

const MM: i64 = 1_000_000;

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * MM, y_mm * MM).unwrap()
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

/// A board 60 by 40 mm with one copper layer, a track along y = 10 mm and a via at (30, 20) mm.
fn board() -> Board {
    let mut board = Board {
        name: "T".to_owned(),
        outline: vec![pt(0, 0), pt(60, 0), pt(60, 40), pt(0, 40)],
        ..Board::default()
    };
    board.layers.insert(
        id(1),
        Layer {
            name: "Top".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.copper_stack = vec![id(1)];
    board.nets.insert(
        id(10),
        Net {
            name: "SIG".to_owned(),
        },
    );
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
    board.vias.insert(
        id(41),
        Via {
            net: id(10),
            at: pt(30, 20),
            drill: Nm::new(400_000).unwrap(),
            diameter: Nm::new(1_000_000).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    board
}

struct Files {
    dir: tempfile::TempDir,
    board: String,
}

impl Files {
    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }

    fn write(&self, name: &str, text: &str) -> String {
        let path = self.path(name);
        std::fs::write(&path, text).unwrap();
        path
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

/// A rectangle of four loose lines on the layer EDGE.
fn rectangle_dxf(width_mm: i64, height_mm: i64) -> String {
    let mut d = Drawing::new();
    let (w, h) = (width_mm * MM, height_mm * MM);
    d.line("EDGE", (0, 0), (w, 0));
    d.line("EDGE", (w, 0), (w, h));
    d.line("EDGE", (w, h), (0, h));
    d.line("EDGE", (0, h), (0, 0));
    write_dxf(&d)
}

#[test]
fn the_outline_of_a_drawing_is_shown_in_a_dry_run_and_nothing_is_written() {
    let f = files();
    let dxf = f.write("edge.dxf", &rectangle_dxf(50, 30));
    let (code, out, err) = call(&["dxf-import", &dxf, "--board", &f.board, "--layer", "edge"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("entities read: LINE 4"), "{out}");
    assert!(
        out.contains(
            "pieces: 4, chains: 1, closed contours: 1, open chains: 0, duplicates dropped: 0"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "outline: 4 corners, 50.000 by 30.000 mm (it was 4 corners, 60.000 by 40.000 mm)"
        ),
        "{out}"
    );
    assert!(out.contains("rule errors on the result: 0"), "{out}");
    assert!(out.contains("dry run: nothing was written"), "{out}");
    // the board that was read is untouched
    assert_eq!(
        std::fs::read(&f.board).unwrap(),
        board().to_bytes().unwrap()
    );
}

#[test]
fn the_outline_is_written_to_a_new_file_and_never_over_one_that_exists() {
    let f = files();
    let dxf = f.write("edge.dxf", &rectangle_dxf(50, 30));
    let out_path = f.path("out.json");
    let (code, out, err) = call(&["dxf-import", &dxf, "--board", &f.board, "--out", &out_path]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(&format!("written: {out_path}")), "{out}");
    let written = Board::from_bytes(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(
        written.outline,
        vec![pt(0, 0), pt(50, 0), pt(50, 30), pt(0, 30)]
    );
    // the same name again, the board as an output and a name that is not .json
    for bad in [out_path.clone(), f.board.clone(), f.path("out.txt")] {
        let (code, _, err) = call(&["dxf-import", &dxf, "--board", &f.board, "--out", &bad]);
        assert_eq!(code, 2, "{bad}");
        assert!(err.contains("PHOENIX-DXF-009"), "{err}");
    }
}

#[test]
fn a_drawing_that_cannot_give_an_outline_is_refused_and_listed() {
    let f = files();
    // three of the four lines: the chain is open
    let mut d = Drawing::new();
    d.line("EDGE", (0, 0), (50 * MM, 0));
    d.line("EDGE", (50 * MM, 0), (50 * MM, 30 * MM));
    d.line("EDGE", (50 * MM, 30 * MM), (0, 30 * MM));
    let dxf = f.write("open.dxf", &write_dxf(&d));
    let (code, out, err) = call(&["dxf-import", &dxf, "--board", &f.board]);
    assert_eq!(code, 2);
    assert!(out.contains("PHOENIX-DXF-011"), "{out}");
    assert!(err.contains("PHOENIX-DXF-014"), "{err}");
    // text that is not DXF
    let junk = f.write("junk.dxf", "nothing\n");
    let (code, _, err) = call(&["dxf-import", &junk, "--board", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DXF-003"), "{err}");
    // a file that is not there and the options that are missing
    let (code, _, err) = call(&["dxf-import", &f.path("none.dxf"), "--board", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DXF-010"), "{err}");
    assert_eq!(call(&["dxf-import", &dxf]).0, 2);
    assert_eq!(
        call(&["dxf-import", &dxf, "--board", &f.board, "--nope"]).0,
        2
    );
}

#[test]
fn the_json_report_of_the_import_holds_the_counts_the_cutouts_and_the_outlines() {
    let f = files();
    let mut d = Drawing::new();
    d.polygon(
        "EDGE",
        &[(0, 0), (50 * MM, 0), (50 * MM, 30 * MM), (0, 30 * MM)],
    );
    d.circle("EDGE", (40 * MM, 20 * MM), 2 * MM);
    d.text("NOTES", (0, 0), MM, "x");
    let dxf = f.write("edge.dxf", &write_dxf(&d));
    let (code, out, err) = call(&["dxf-import", &dxf, "--board", &f.board, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["schema"], "phoenix-dxf-import/1");
    assert_eq!(value["cutouts"], 1);
    assert_eq!(value["contours"], 2);
    assert_eq!(value["outline_before"]["width_nm"], 60_000_000);
    assert_eq!(value["outline_after"]["width_nm"], 50_000_000);
    assert_eq!(value["outline_after"]["height_nm"], 30_000_000);
    assert_eq!(value["entities"]["CIRCLE"], 1);
    let codes: Vec<&str> = value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"PHOENIX-DXF-004"), "{codes:?}");
    assert!(codes.contains(&"PHOENIX-DXF-015"), "{codes:?}");
    assert_eq!(value["written"], serde_json::Value::Null);
}

#[test]
fn a_board_is_written_as_a_drawing_that_the_reader_gives_back() {
    let f = files();
    let (code, out, _) = call(&["dxf-export", "board", &f.board]);
    assert_eq!(code, 0);
    assert!(out.contains("layer OUTLINE: 1"), "{out}");
    assert!(out.contains("layer DRILLS: 1"), "{out}");
    assert!(out.contains("PHOENIX-DXF-019"), "{out}");
    assert!(out.contains("dry run: nothing was written"), "{out}");
    let target = f.path("board.dxf");
    let (code, out, err) = call(&["dxf-export", "placa", &f.board, "--out", &target]);
    assert_eq!(code, 0, "{out}{err}");
    let text = std::fs::read_to_string(&target).unwrap();
    assert!(text.starts_with("  0\nSECTION\n  2\nHEADER\n"));
    let read = read_dxf(&text, &DxfSettings::default()).unwrap();
    let outline: Vec<_> = read.drawing.on_layer("OUTLINE").collect();
    assert_eq!(outline.len(), 1);
    match &outline[0].shape {
        Shape::Path { segs, closed } => {
            assert!(*closed);
            assert_eq!(segs.len(), 4);
            assert_eq!(segs[2].a, (60 * MM, 40 * MM));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        read.drawing.on_layer("DRILLS").next().unwrap().shape,
        Shape::Circle {
            centre: (30 * MM, 20 * MM),
            radius: 200_000
        }
    );
    // never over a file that exists, never to a name that is not .dxf
    let (code, _, err) = call(&["dxf-export", "board", &f.board, "--out", &target]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DXF-009"), "{err}");
    let (code, _, err) = call(&["dxf-export", "board", &f.board, "--out", &f.path("b.txt")]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DXF-009"), "{err}");
    // the file written can be the outline of another board
    let (code, out, err) = call(&[
        "dxf-import",
        &target,
        "--board",
        &f.board,
        "--layer",
        "OUTLINE",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("the outline is the one the board has"),
        "{out}"
    );
}

#[test]
fn the_names_of_the_layers_come_from_the_settings_file() {
    let f = files();
    let config = f.write("dxf.cfg", "layer_outline = EDGE\nlayer_drills = HOLES\n");
    let target = f.path("board.dxf");
    let (code, out, err) = call(&[
        "dxf-export",
        "board",
        &f.board,
        "--config",
        &config,
        "--out",
        &target,
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("layer EDGE: 1") && out.contains("layer HOLES: 1"),
        "{out}"
    );
    let bad = f.write("bad.cfg", "layer_outline = A/B\n");
    let (code, _, err) = call(&["dxf-export", "board", &f.board, "--config", &bad]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DXF-001"), "{err}");
}

#[test]
fn the_wrong_arguments_of_the_export_exit_2() {
    let f = files();
    assert_eq!(call(&["dxf-export"]).0, 2);
    assert_eq!(call(&["dxf-export", "schematic", &f.board]).0, 2);
    assert_eq!(call(&["dxf-export", "board"]).0, 2);
    assert_eq!(call(&["dxf-export", "board", &f.board, "--nope"]).0, 2);
    assert_eq!(call(&["dxf-export", "board", &f.path("none.json")]).0, 2);
}
