// SPDX-License-Identifier: GPL-3.0-or-later
//! The zones in the commands `boardedit` and `zones` (`--edit`), on a board written by the test:
//! the dry run of editing operations before the fill, the zone data and the poured board written
//! to new files only, and the way the poured copper goes through the export and comes back from
//! the Gerber import as regions.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Via, Zone};
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
/// via of GND in the middle of it.
fn board() -> Board {
    let mut b = Board {
        name: "zone-edit".into(),
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
            cutouts: Vec::new(),
        },
    );
    b.vias.insert(
        id(20),
        Via {
            net: id(2),
            at: pt(10 * MM, 10 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(MM).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    b
}

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_board(dir: &Path) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

const CONFIG: &str = "keep_unconnected_islands = true\nmin_island_area_nm2 = 0\n";

/// The operation that cuts a square of 2 mm from 5 mm in x and y.
fn cutout_op() -> String {
    format!(
        r#"{{"op":"zone_cutout","zone":"{}","polygon":[{{"x_nm":5000000,"y_nm":5000000}},{{"x_nm":7000000,"y_nm":5000000}},{{"x_nm":7000000,"y_nm":7000000}},{{"x_nm":5000000,"y_nm":7000000}}]}}"#,
        id(10)
    )
}

fn area_of_fill(path: &str) -> i64 {
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    value["zones"][0]["area_nm2"].as_i64().unwrap()
}

#[test]
fn zones_fill_tries_the_editing_operations_in_memory_and_writes_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let ops = write_text(dir.path(), "ops.json", &format!("[{}]", cutout_op()));
    let plain = dir.path().join("plain.json");
    let plain = plain.to_str().unwrap();
    let (code, _, err) = command(&["zones", "fill", "--config", &config, "--out", plain, &board]);
    assert_eq!(code, 0, "{err}");
    let edited = dir.path().join("edited.json");
    let edited = edited.to_str().unwrap();
    let (code, out, err) = command(&[
        "zones", "fill", "--config", &config, "--edit", &ops, "--out", edited, &board,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("1 operations were tried in memory"), "{err}");
    assert!(out.starts_with("filled 1 zones: "), "{out}");
    // the hole of 2 mm by 2 mm is out of the copper, to the nanometre
    assert_eq!(
        area_of_fill(plain) - area_of_fill(edited),
        4_000_000_000_000
    );
    // the board and the operations are as they were; only the two fills were written
    assert_eq!(fs::read(&board).unwrap(), board_bytes());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 5);
    // the check works on the board after the operations too
    let (code, out, err) = command(&[
        "zones", "check", "--config", &config, "--edit", &ops, &board,
    ]);
    assert_eq!(code, 0, "{err}\n{out}");
}

fn board_bytes() -> Vec<u8> {
    board().to_bytes().unwrap()
}

#[test]
fn an_operation_that_is_refused_stops_the_run_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let outside = format!(
        r#"[{{"op":"zone_cutout","zone":"{}","polygon":[{{"x_nm":30000000,"y_nm":30000000}},{{"x_nm":31000000,"y_nm":30000000}},{{"x_nm":31000000,"y_nm":31000000}}]}}]"#,
        id(10)
    );
    let ops = write_text(dir.path(), "ops.json", &outside);
    let (code, _, err) = command(&["zones", "fill", "--config", &config, "--edit", &ops, &board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BZONE-004"), "{err}");
}

#[test]
fn boardedit_keeps_the_zone_data_and_the_poured_board_in_new_files_and_the_export_pours_them() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let config = write_text(dir.path(), "zones.cfg", CONFIG);
    let ops = write_text(
        dir.path(),
        "ops.json",
        &format!(r#"[{},{{"op":"zone_fill"}}]"#, cutout_op()),
    );
    let data = dir.path().join("zones-data.json");
    let poured = dir.path().join("poured.json");
    let (data, poured) = (data.to_str().unwrap(), poured.to_str().unwrap());
    let (code, out, err) = command(&[
        "boardedit",
        "--zones-config",
        &config,
        "--zones-out",
        data,
        "--poured-out",
        poured,
        "--apply",
        &ops,
        &board,
    ]);
    assert_eq!(code, 0, "{err}\n{out}");
    assert!(out.contains("zones: "), "{out}");
    assert!(out.contains("gerber: "), "{out}");
    let text = fs::read_to_string(data).unwrap();
    assert!(text.contains("phoenix-zone-data/1"), "{text}");
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let polygons = value["fills"][id(10).to_string()]["polygons"]
        .as_array()
        .unwrap()
        .len();
    assert!(polygons > 1, "{polygons}");
    // a file that is there is never written over, and the board is not touched
    let (code, _, err) = command(&[
        "boardedit",
        "--zones-config",
        &config,
        "--zones-out",
        data,
        "--apply",
        &ops,
        &board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("exists"), "{err}");
    assert_eq!(fs::read(&board).unwrap(), board_bytes());
    // the data goes back in: an editor that starts from it keeps the fill
    let again = dir.path().join("again.json");
    let again = again.to_str().unwrap();
    let nothing = write_text(dir.path(), "none.json", "[]");
    let (code, out, err) = command(&[
        "boardedit",
        "--zones",
        data,
        "--zones-config",
        &config,
        "--zones-out",
        again,
        "--apply",
        &nothing,
        &board,
    ]);
    assert_eq!(code, 0, "{err}\n{out}");
    assert_eq!(fs::read_to_string(again).unwrap(), text);
    // the poured board goes through the export and the import: one region for each polygon
    let fab = dir.path().join("fab");
    let fab_text = fab.to_str().unwrap();
    let (code, _, err) = command(&["export", "--out", fab_text, poured]);
    assert_eq!(code, 0, "{err}");
    let copper = fs::read_dir(&fab)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let name = p
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_ascii_lowercase();
            name.contains("top") && name.ends_with(".gbr")
        })
        .expect("the copper file");
    let (code, text, err) = command(&["gerber-import", copper.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}\n{text}");
    assert!(text.contains(&format!("{polygons} regions")), "{text}");
}
