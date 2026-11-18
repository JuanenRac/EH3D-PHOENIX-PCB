// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `stats` on a board written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track, Via, Zone};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point};

const MM: i64 = 1_000_000;

fn command(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A 20 mm square board with one copper layer, a track of 10 mm and a via.
fn board() -> Board {
    let mut b = Board {
        name: "stats".into(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 20 * MM),
            pt(0, 20 * MM),
        ],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.nets.insert(id(2), Net { name: "SIG".into() });
    b.tracks.insert(
        id(10),
        Track {
            net: id(2),
            layer: id(1),
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(5 * MM, 5 * MM),
                end: pt(15 * MM, 5 * MM),
            }],
        },
    );
    b.vias.insert(
        id(11),
        Via {
            net: id(2),
            at: pt(10 * MM, 10 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    b
}

fn write_board(dir: &Path, b: &Board) -> String {
    let path = dir.join("board.json");
    fs::write(&path, b.to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn the_statistics_of_a_board_are_printed_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, err) = command(&["stats", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with(
            "Board statistics: stats
"
        ),
        "{out}"
    );
    assert!(
        out.contains("Board: 20.000000 x 20.000000 mm; 400.000000 mm2"),
        "{out}"
    );
    assert!(out.contains("Tracks: 1 (length: 10.000000 mm)"), "{out}");
    assert!(out.contains("  Top: copper 2.761144 mm2"), "{out}");
    assert!(
        out.contains("0.300000 mm: 1 Vias; 0 plated pads; 0 unplated"),
        "{out}"
    );
    // only the board is in the folder
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn the_json_report_has_the_numbers_of_the_board() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, err) = command(&["stats", "--json", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"format\": \"phoenix-stats/1\""), "{out}");
    assert!(out.contains("\"area_um2\": 400000000"), "{out}");
    assert!(out.contains("\"track_length_nm\": 10000000"), "{out}");
    let (again, same, _) = command(&["stats", "--json", &path]);
    assert_eq!(again, 0);
    assert_eq!(same, out);
}

#[test]
fn the_titles_come_in_the_language_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, _) = command(&["stats", "--lang", "es", &path]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with(
            "Estadísticas de la placa: stats
"
        ),
        "{out}"
    );
    let (_, pt_out, _) = command(&["stats", "--lang", "pt-PT", &path]);
    assert!(
        pt_out.starts_with(
            "Estatísticas da placa: stats
"
        ),
        "{pt_out}"
    );
}

#[test]
fn the_settings_file_limits_the_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let config = write_text(
        dir.path(),
        "stats.cfg",
        "max_items = 1
",
    );
    let (code, _, err) = command(&["stats", "--config", &config, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STATS-005"), "{err}");
    let bad = write_text(
        dir.path(),
        "bad.cfg",
        "max_items = x
",
    );
    let (code, _, err) = command(&["stats", "--config", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STATS-001"), "{err}");
}

#[test]
fn bad_arguments_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    assert_eq!(command(&["stats"]).0, 2);
    assert_eq!(command(&["stats", "--bogus", &path]).0, 2);
    assert_eq!(command(&["stats", "--lang", "xx", &path]).0, 2);
    assert_eq!(command(&["stats", "--config"]).0, 2);
    assert_eq!(command(&["stats", &path, &path]).0, 2);
    let missing = dir.path().join("none.json");
    let (code, _, err) = command(&["stats", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STATS-003"), "{err}");
}

#[test]
fn the_help_of_the_command_is_in_the_table() {
    let (code, out, _) = command(&["help", "stats", "--lang", "de"]);
    assert_eq!(code, 0);
    assert!(out.contains("phoenix stats"), "{out}");
}

/// The board with a second copper layer that a zone covers from edge to edge.
fn uneven_board() -> Board {
    let mut b = board();
    b.layers.insert(
        id(3),
        Layer {
            name: "Bottom".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.copper_stack = vec![id(1), id(3)];
    b.zones.insert(
        id(12),
        Zone {
            net: id(2),
            layer: id(3),
            outline: vec![
                pt(0, 0),
                pt(20 * MM, 0),
                pt(20 * MM, 20 * MM),
                pt(0, 20 * MM),
            ],
            clearance: Nm::new(200_000).unwrap(),
            cutouts: Vec::new(),
        },
    );
    b
}

#[test]
fn layers_with_uneven_copper_are_reported_and_the_severity_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &uneven_board());
    let (code, out, err) = command(&["stats", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("Copper balance:"), "{out}");
    assert!(
        out.contains("  Top / Bottom: 0.69 % against 100.00 %, difference 99.31 points"),
        "{out}"
    );
    assert!(out.contains("warning: PHOENIX-STATS-008"), "{out}");
    let strict = write_text(dir.path(), "strict.cfg", "balance_severity = error\n");
    let (code, out, _) = command(&["stats", "--config", &strict, &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error: PHOENIX-STATS-008"), "{out}");
    let off = write_text(dir.path(), "off.cfg", "balance_check = false\n");
    let (code, out, _) = command(&["stats", "--config", &off, &path]);
    assert_eq!(code, 0);
    assert!(!out.contains("Copper balance"), "{out}");
}
