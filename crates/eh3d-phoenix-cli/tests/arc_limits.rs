// SPDX-License-Identifier: GPL-3.0-or-later
//! The cut of the arcs of the outline into chords (`config/outline_arcs.cfg`) is read from the
//! option `--limits FILE` and reaches the commands that need the polygon of the outline.

use std::path::PathBuf;

use eh3d_phoenix_board::{ArcSettings, Board, Layer, LayerKind, OutlineArc};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_dxf::{read_dxf, DxfSettings, Shape};
use eh3d_phoenix_model::{ObjectId, Point, Udeg};

const MM: i64 = 1_000_000;

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * MM, y_mm * MM).unwrap()
}

fn write(dir: &tempfile::TempDir, name: &str, text: &[u8]) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn repo_file(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name)
        .to_str()
        .unwrap()
        .to_owned()
}

/// A board of 60 by 40 mm whose corner at (60, 40) is cut by a quarter circle of radius 8 mm: the
/// edge from (60, 32) to (52, 40) is an arc of 90 degrees.
fn rounded_board() -> Board {
    let id = ObjectId::from_parts(1_700_000_000_000, 1, 1).unwrap();
    let mut board = Board {
        name: "T".to_owned(),
        outline: vec![pt(0, 0), pt(60, 0), pt(60, 32), pt(52, 40), pt(0, 40)],
        outline_arcs: vec![OutlineArc {
            edge: 2,
            sweep: Udeg::new(90 * 1_000_000).unwrap(),
        }],
        ..Board::default()
    };
    board.layers.insert(
        id,
        Layer {
            name: "Top".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.copper_stack = vec![id];
    board
}

/// The number of points of the outline that `dxf-export` wrote, with the arguments before the
/// board and the output name.
fn outline_points(dir: &tempfile::TempDir, board: &str, name: &str, extra: &[&str]) -> usize {
    let target = dir.path().join(name).to_str().unwrap().to_owned();
    let mut args = vec!["dxf-export", "board", board, "--out", &target];
    args.extend_from_slice(extra);
    let (code, out, err) = call(&args);
    assert_eq!(code, 0, "{out}{err}");
    let text = std::fs::read_to_string(&target).unwrap();
    let read = read_dxf(&text, &DxfSettings::default()).unwrap();
    let outline: Vec<_> = read.drawing.on_layer("OUTLINE").collect();
    assert_eq!(outline.len(), 1);
    match &outline[0].shape {
        Shape::Path { segs, closed } => {
            assert!(*closed);
            segs.len()
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_different_tolerance_in_the_limits_file_changes_the_chords_of_the_written_outline() {
    let dir = tempfile::tempdir().unwrap();
    let board = write(&dir, "in.json", &rounded_board().to_bytes().unwrap());
    // radius 8 mm: with 1 micrometre a chord spans at most 2 * acos(1 - 1/8000000) rad, so the
    // quarter circle needs 50 chords; with 0.1 mm it needs 5 (2 * acos(0.9875) = 0.31628 rad)
    let plain = outline_points(&dir, &board, "plain.dxf", &[]);
    assert_eq!(plain, 5 + 49);
    let coarse = write(&dir, "coarse.cfg", b"tolerance_nm = 100000\n");
    let coarse_points = outline_points(&dir, &board, "coarse.dxf", &["--limits", &coarse]);
    assert_eq!(coarse_points, 5 + 4);
    // the option may stand before the command, and a cap of pieces wins over the tolerance
    let capped = write(&dir, "capped.cfg", b"max_pieces = 2\n");
    let capped_points = outline_points(&dir, &board, "capped.dxf", &["--limits", &capped]);
    assert_eq!(capped_points, 5 + 1);
    // the shipped file is the default and changes nothing
    let shipped = repo_file("config/outline_arcs.cfg");
    let same = outline_points(&dir, &board, "same.dxf", &["--limits", &shipped]);
    assert_eq!(same, plain);
    // the settings are put back when the command is over
    assert_eq!(ArcSettings::current(), ArcSettings::default());
}

#[test]
fn the_keys_of_the_arcs_share_a_limits_file_with_the_other_limits() {
    let dir = tempfile::tempdir().unwrap();
    let board = write(&dir, "in.json", &rounded_board().to_bytes().unwrap());
    let mixed = write(
        &dir,
        "mixed.cfg",
        b"max_polygon_points = 500\ntolerance_nm = 100000\nmax_bus_checks = 9\n",
    );
    let points = outline_points(&dir, &board, "mixed.dxf", &["--limits", &mixed]);
    assert_eq!(points, 5 + 4);
}

#[test]
fn a_value_of_the_arcs_out_of_its_range_exits_2_and_names_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let board = write(&dir, "in.json", &rounded_board().to_bytes().unwrap());
    for (text, line, key) in [
        ("tolerance_nm = 0\n", "line 1", "tolerance_nm"),
        (
            "max_bus_checks = 5\ntolerance_nm = 1000001\n",
            "line 2",
            "tolerance_nm",
        ),
        ("\nmax_pieces = 0\n", "line 2", "max_pieces"),
        ("max_pieces = 65537\n", "line 1", "max_pieces"),
        ("max_pieces = many\n", "line 1", "max_pieces"),
    ] {
        let bad = write(&dir, "bad.cfg", text.as_bytes());
        let (code, out, err) = call(&["dxf-export", "board", &board, "--limits", &bad]);
        assert_eq!(code, 2, "{text}: {out}");
        assert!(err.contains(line) && err.contains(key), "{text}: {err}");
    }
}
