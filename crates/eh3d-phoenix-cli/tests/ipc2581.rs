// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ipc2581` on a board written by the test itself: the file it writes is read back
//! with the XML reader of the repository, and it never replaces a file without `--overwrite`.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_eagle::xml::{parse, Limits};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

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

fn pad(number: &str, at: Point) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Rect {
            width: Nm::new(800_000).unwrap(),
            height: Nm::new(600_000).unwrap(),
        },
        at,
        rotation: Udeg::ZERO,
        drill: None,
        layers: vec![id(1)],
        roundness: 0,
        plated: true,
    }
}

/// A 20 x 10 mm board with one layer, a resistor between two nets and a track.
fn board() -> Board {
    let mut b = Board {
        name: "demo".into(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 10 * MM),
            pt(0, 10 * MM),
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
    let mut resistor = Footprint {
        name: "R_0603".into(),
        pads: Default::default(),
    };
    resistor.pads.insert(id(11), pad("1", pt(-800_000, 0)));
    resistor.pads.insert(id(12), pad("2", pt(800_000, 0)));
    b.footprints.insert(id(10), resistor);
    b.components.insert(
        id(20),
        Component {
            refdes: "R1".into(),
            value: "10k".into(),
            symbol: None,
            footprint: id(10),
            at: pt(10 * MM, 5 * MM),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    for (pad_id, net) in [(11, 3), (12, 2)] {
        b.pad_nets.push(PadNet {
            component: id(20),
            pad: id(pad_id),
            net: id(net),
        });
    }
    b.pad_nets.sort_by_key(PadNet::pad_ref);
    b.tracks.insert(
        id(30),
        Track {
            net: id(3),
            layer: id(1),
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(9 * MM, 5 * MM),
                end: pt(3 * MM, 5 * MM),
            }],
        },
    );
    b
}

fn write_board(dir: &Path) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn without_out_the_command_is_a_dry_run_that_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, err) = command(&["ipc2581", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("IPC-2581 of demo: 1 layers (1 copper), 1 parts in 1 packages, 1 padstacks, 2 pads, 0 vias, 1 tracks, 0 zones, 2 nets, 1 bill lines, "),
        "{out}"
    );
    assert!(out.contains("info: PHOENIX-IPC2581-006"), "{out}");
    assert_eq!(names(dir.path()), vec!["board.json"]);
}

#[test]
fn the_file_is_written_to_a_new_name_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let target = dir.path().join("demo.xml");
    let target = target.to_str().unwrap();
    let (code, out, err) = command(&["ipc2581", "--out", target, &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains(&format!("written: {target}")), "{out}");
    let text = fs::read_to_string(target).unwrap();
    let root = parse(&text, &Limits::default()).expect("well formed");
    assert_eq!(root.name, "IPC-2581");
    let step = root
        .child("Ecad")
        .and_then(|e| e.child("CadData"))
        .and_then(|c| c.child("Step"))
        .unwrap();
    assert_eq!(step.attr("name"), Some("demo"));
    let profile = step.child("Profile").unwrap().child("Polygon").unwrap();
    assert_eq!(profile.children.len(), 5);
    assert_eq!(profile.children[1].attr("x"), Some("20"));
    let component = step.child("Component").unwrap();
    assert_eq!(component.attr("refDes"), Some("R1"));
    assert_eq!(component.child("Location").unwrap().attr("x"), Some("10"));
    let line = step
        .child("LayerFeature")
        .unwrap()
        .children_named("Set")
        .flat_map(|s| s.children_named("Features"))
        .find_map(|f| f.child("Polyline"))
        .unwrap();
    assert_eq!(line.children[0].attr("x"), Some("9"));
    assert_eq!(line.children[1].attr("x"), Some("3"));
    // the same board gives the same bytes
    let again = dir.path().join("again.xml");
    let (code, _, _) = command(&["ipc2581", "--out", again.to_str().unwrap(), &path]);
    assert_eq!(code, 0);
    assert_eq!(fs::read_to_string(&again).unwrap(), text);
}

#[test]
fn a_file_that_exists_is_never_replaced_without_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let target = write_text(dir.path(), "demo.xml", "keep me");
    let (code, _, err) = command(&["ipc2581", "--out", &target, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-004"), "{err}");
    assert!(err.contains("--overwrite"), "{err}");
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep me");
    assert_eq!(names(dir.path()), vec!["board.json", "demo.xml"]);
}

#[test]
fn overwrite_replaces_a_plain_file_and_leaves_no_side_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let target = write_text(dir.path(), "demo.xml", "old");
    let (code, out, err) = command(&["ipc2581", "--out", &target, "--overwrite", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written: "), "{out}");
    let text = fs::read_to_string(&target).unwrap();
    assert!(
        text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
        "{text}"
    );
    assert_eq!(names(dir.path()), vec!["board.json", "demo.xml"]);
}

#[test]
fn overwrite_never_touches_the_input_board_a_folder_or_a_side_file_in_the_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let before = fs::read(&path).unwrap();
    let (code, _, err) = command(&["ipc2581", "--out", &path, "--overwrite", &path]);
    assert_eq!(code, 2);
    assert!(err.contains("is the input board"), "{err}");
    assert_eq!(fs::read(&path).unwrap(), before);
    let folder = dir.path().join("folder");
    fs::create_dir(&folder).unwrap();
    let (code, _, err) = command(&[
        "ipc2581",
        "--out",
        folder.to_str().unwrap(),
        "--overwrite",
        &path,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("not a plain file"), "{err}");
    // a name for the side file that is already taken stops the replacement
    let target = write_text(dir.path(), "demo.xml", "old");
    write_text(dir.path(), "demo.xml.new", "in the way");
    let (code, _, err) = command(&["ipc2581", "--out", &target, "--overwrite", &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-004"), "{err}");
    assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    assert_eq!(
        fs::read_to_string(dir.path().join("demo.xml.new")).unwrap(),
        "in the way"
    );
}

#[test]
fn the_json_summary_has_the_counts() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, err) = command(&["ipc2581", "--json", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"format\": \"phoenix-ipc2581/1\""), "{out}");
    assert!(out.contains("\"pads\": 2"), "{out}");
    assert!(out.contains("\"tracks\": 1"), "{out}");
}

#[test]
fn the_settings_say_what_goes_into_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let config = write_text(
        dir.path(),
        "ipc.cfg",
        "include_bom = false\ninclude_tracks = false\norigination = 2001-02-03T04:05:06\n",
    );
    let target = dir.path().join("demo.xml");
    let target = target.to_str().unwrap();
    let (code, _, err) = command(&["ipc2581", "--config", &config, "--out", target, &path]);
    assert_eq!(code, 0, "{err}");
    let text = fs::read_to_string(target).unwrap();
    assert!(!text.contains("<Bom "), "no bill");
    assert!(!text.contains("<Polyline"), "no tracks");
    assert!(
        text.contains("origination=\"2001-02-03T04:05:06\""),
        "{text}"
    );
    let bad = write_text(dir.path(), "bad.cfg", "origination = soon\n");
    let (code, _, err) = command(&["ipc2581", "--config", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-001"), "{err}");
    let small = write_text(dir.path(), "small.cfg", "max_items = 1\n");
    let (code, _, err) = command(&["ipc2581", "--config", &small, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-005"), "{err}");
}

#[test]
fn the_messages_come_in_the_language_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, _) = command(&["ipc2581", "--lang", "es", &path]);
    assert_eq!(code, 0);
    assert!(
        out.contains("info: PHOENIX-IPC2581-006: Partes de la placa quedan fuera del archivo"),
        "{out}"
    );
}

#[test]
fn bad_arguments_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    assert_eq!(command(&["ipc2581"]).0, 2);
    assert_eq!(command(&["ipc2581", "--bogus", &path]).0, 2);
    assert_eq!(command(&["ipc2581", "--lang", "xx", &path]).0, 2);
    assert_eq!(command(&["ipc2581", "--out"]).0, 2);
    assert_eq!(command(&["ipc2581", &path, &path]).0, 2);
    let (code, _, err) = command(&["ipc2581", "--overwrite", &path]);
    assert_eq!(code, 2);
    assert!(err.contains("--overwrite needs --out"), "{err}");
    let missing = dir.path().join("none.json");
    let (code, _, err) = command(&["ipc2581", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-003"), "{err}");
    assert_eq!(names(dir.path()), vec!["board.json"]);
}

#[test]
fn the_help_of_the_command_is_in_the_table() {
    let (code, out, _) = command(&["help", "ipc2581"]);
    assert_eq!(code, 0);
    assert!(out.contains("phoenix ipc2581"), "{out}");
    assert!(out.contains("docs/IPC2581.md"), "{out}");
}
