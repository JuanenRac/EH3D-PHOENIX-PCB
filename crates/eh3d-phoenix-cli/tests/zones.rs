// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `zones` and `lengths` on boards written by the test itself.

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

/// A 20 mm board with one copper layer, the nets GND and SIG, a zone of GND from 1 to 19 mm and a
/// via of SIG in the middle of it.
fn board() -> Board {
    let mut b = Board {
        name: "zones".into(),
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
    b.nets.insert(id(2), Net { name: "GND".into() });
    b.nets.insert(id(3), Net { name: "SIG".into() });
    b.zones.insert(
        id(10),
        Zone {
            net: id(2),
            layer: id(1),
            outline: vec![
                pt(MM, MM),
                pt(19 * MM, MM),
                pt(19 * MM, 19 * MM),
                pt(MM, 19 * MM),
            ],
            clearance: Nm::new(200_000).unwrap(),
        },
    );
    b.vias.insert(
        id(20),
        Via {
            net: id(3),
            at: pt(10 * MM, 10 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(MM).unwrap(),
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

const CONFIG: &str = "keep_unconnected_islands = true\nmin_island_area_nm2 = 0\n";

#[test]
fn a_zone_is_filled_without_writing_anything_unless_asked() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let (code, out, err) = command(&["zones", "fill", "--config", &config, &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("filled 1 zones: "), "{out}");
    assert!(out.contains("net GND layer Top"), "{out}");
    // only the board and the settings are in the folder
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn the_fill_is_written_to_a_new_file_and_never_over_an_old_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let fill = dir.path().join("fill.json");
    let fill = fill.to_str().unwrap();
    let (code, out, err) = command(&["zones", "fill", "--config", &config, "--out", fill, &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written: "), "{out}");
    let text = fs::read_to_string(fill).unwrap();
    assert!(text.contains("phoenix-zone-fill/1"), "{text}");
    let (again, _, err) = command(&["zones", "fill", "--config", &config, "--out", fill, &path]);
    assert_eq!(again, 2);
    assert!(err.contains("PHOENIX-ZONE-016"), "{err}");
    assert_eq!(fs::read_to_string(fill).unwrap(), text);
}

#[test]
fn a_written_fill_passes_the_check_and_a_changed_one_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let fill = dir.path().join("fill.json");
    let fill = fill.to_str().unwrap();
    let (code, _, err) = command(&["zones", "fill", "--config", &config, "--out", fill, &path]);
    assert_eq!(code, 0, "{err}");
    let (code, out, err) = command(&["zones", "check", "--config", &config, "--fill", fill, &path]);
    assert_eq!(code, 0, "{err}\n{out}");
    assert!(out.starts_with("check: 0 errors"), "{out}");
    // the zone made to cover the whole square, over the via
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fill).unwrap()).unwrap();
    value["zones"][0]["polygons"] = serde_json::json!([[
        [1_000_000, 1_000_000],
        [19_000_000, 1_000_000],
        [19_000_000, 19_000_000],
        [1_000_000, 19_000_000]
    ]]);
    let changed = write_text(dir.path(), "changed.json", &value.to_string());
    let (code, out, _) = command(&[
        "zones", "check", "--config", &config, "--fill", &changed, "--json", &path,
    ]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-ZONE-006"), "{out}");
    // without a file the fill is made and measured
    let (code, out, err) = command(&["zones", "check", "--config", &config, &path]);
    assert_eq!(code, 0, "{err}\n{out}");
}

#[test]
fn the_messages_come_in_the_language_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let fill = dir.path().join("fill.json");
    let fill = fill.to_str().unwrap();
    assert_eq!(
        command(&["zones", "fill", "--config", &config, "--out", fill, &path]).0,
        0
    );
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fill).unwrap()).unwrap();
    value["zones"][0]["polygons"] = serde_json::json!([[
        [1_000_000, 1_000_000],
        [19_000_000, 1_000_000],
        [19_000_000, 19_000_000],
        [1_000_000, 19_000_000]
    ]]);
    let changed = write_text(dir.path(), "changed.json", &value.to_string());
    let (code, out, _) = command(&[
        "zones", "check", "--config", &config, "--fill", &changed, "--lang", "pt", &path,
    ]);
    assert_eq!(code, 1);
    assert!(
        out.contains("O preenchimento está mais perto de outro cobre do que a distância"),
        "{out}"
    );
}

#[test]
fn bad_arguments_and_bad_input_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    assert_eq!(command(&["zones"]).0, 2);
    assert_eq!(command(&["zones", "bogus", &path]).0, 2);
    assert_eq!(command(&["zones", "fill"]).0, 2);
    assert_eq!(command(&["zones", "fill", "--lang", "xx", &path]).0, 2);
    assert_eq!(command(&["zones", "fill", "--bogus", &path]).0, 2);
    assert_eq!(command(&["zones", "fill", "--fill", "x", &path]).0, 2);
    assert_eq!(command(&["zones", "check", "--out", "x", &path]).0, 2);
    let missing = dir.path().join("none.json");
    assert_eq!(command(&["zones", "fill", missing.to_str().unwrap()]).0, 2);
    let bad = write_text(dir.path(), "bad.cfg", "colour = red\n");
    let (code, _, err) = command(&["zones", "fill", "--config", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ZONE-001"), "{err}");
}

fn lengths_board() -> Board {
    let mut b = Board {
        name: "lengths".into(),
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
    b.nets.insert(id(2), Net { name: "D0".into() });
    b.nets.insert(id(3), Net { name: "D1".into() });
    for (n, net, end) in [(10_u32, 2_u32, 10 * MM), (11, 3, 10_600_000)] {
        b.tracks.insert(
            id(n),
            Track {
                net: id(net),
                layer: id(1),
                width: Nm::new(150_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(0, 0),
                    end: pt(end, 0),
                }],
            },
        );
    }
    b
}

const GROUPS: &str = r#"{"format":"phoenix-lengths/1","groups":[{"name":"data","nets":["D0","D1"],"tolerance_nm":500000}]}"#;

#[test]
fn the_lengths_of_the_nets_are_listed_and_a_broken_group_exits_with_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &lengths_board());
    let (code, out, err) = command(&["lengths", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("net D0: 10.000000 mm"), "{out}");
    assert!(out.contains("net D1: 10.600000 mm"), "{out}");
    let groups = write_text(dir.path(), "groups.json", GROUPS);
    let (code, out, _) = command(&["lengths", "--groups", &groups, &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("group data: 2 nets"), "{out}");
    assert!(out.contains("PHOENIX-LEN-004"), "{out}");
    let (code, out, _) = command(&["lengths", "--groups", &groups, "--lang", "de", &path]);
    assert_eq!(code, 1);
    assert!(
        out.contains("Die Netze einer Gruppe weichen in der Länge mehr als die Toleranz ab"),
        "{out}"
    );
}

#[test]
fn the_report_is_written_to_a_new_file_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &lengths_board());
    let target = dir.path().join("report.json");
    let target = target.to_str().unwrap();
    let (code, out, err) = command(&["lengths", "--out", target, &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written: "), "{out}");
    let text = fs::read_to_string(target).unwrap();
    assert!(text.contains("\"total_nm\": 10600000"), "{text}");
    let (again, _, err) = command(&["lengths", "--out", target, &path]);
    assert_eq!(again, 2);
    assert!(err.contains("PHOENIX-LEN-015"), "{err}");
    let (json_code, json_out, _) = command(&["lengths", "--json", &path]);
    assert_eq!(json_code, 0);
    assert_eq!(json_out, text);
}

#[test]
fn bad_arguments_of_lengths_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &lengths_board());
    assert_eq!(command(&["lengths"]).0, 2);
    assert_eq!(command(&["lengths", "--bogus", &path]).0, 2);
    assert_eq!(command(&["lengths", "--lang", "xx", &path]).0, 2);
    assert_eq!(command(&["lengths", "--groups"]).0, 2);
    let bad = write_text(dir.path(), "bad.json", "{}");
    let (code, _, err) = command(&["lengths", "--groups", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LEN-002"), "{err}");
    let missing = dir.path().join("none.json");
    assert_eq!(command(&["lengths", missing.to_str().unwrap()]).0, 2);
}
