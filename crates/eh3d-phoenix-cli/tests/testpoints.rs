// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `testpoints` on a board written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side,
};
use eh3d_phoenix_cli::run;
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

fn pad(number: &str, at: Point, diameter: i64) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: Nm::new(diameter).unwrap(),
        },
        at,
        rotation: Udeg::ZERO,
        drill: None,
        layers: vec![id(1)],
        roundness: 0,
        plated: true,
    }
}

/// A 40 x 30 mm board with two resistors between GND and SIG, and one test point on GND.
fn board() -> Board {
    let mut b = Board {
        name: "tp".into(),
        outline: vec![
            pt(0, 0),
            pt(40 * MM, 0),
            pt(40 * MM, 30 * MM),
            pt(0, 30 * MM),
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
    resistor.pads.insert(id(11), pad("1", pt(-MM, 0), 800_000));
    resistor.pads.insert(id(12), pad("2", pt(MM, 0), 800_000));
    b.footprints.insert(id(10), resistor);
    let mut test = Footprint {
        name: "Pad15".into(),
        pads: Default::default(),
    };
    test.pads.insert(id(14), pad("1", pt(0, 0), 1_500_000));
    b.footprints.insert(id(13), test);
    for (n, refdes, x) in [(20_u32, "R1", 10 * MM), (21, "R2", 30 * MM)] {
        b.components.insert(
            id(n),
            Component {
                refdes: refdes.into(),
                value: "10k".into(),
                symbol: None,
                footprint: id(10),
                at: pt(x, 10 * MM),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(11),
            net: id(3),
        });
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(12),
            net: id(2),
        });
    }
    b.components.insert(
        id(30),
        Component {
            refdes: "TP1".into(),
            value: String::new(),
            symbol: None,
            footprint: id(13),
            at: pt(20 * MM, 20 * MM),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: id(30),
        pad: id(14),
        net: id(2),
    });
    b.pad_nets.sort_by_key(PadNet::pad_ref);
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
fn the_test_points_and_the_nets_without_access_are_printed_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, err) = command(&["testpoints", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("Test points of the board: tp\n"), "{out}");
    assert!(out.contains("Test points: 1 (probe side: both)\n"), "{out}");
    assert!(
        out.contains("Nets asked: 2; covered: 1; coverage: 50.00 %\n"),
        "{out}"
    );
    assert!(
        out.contains("  TP1:1 GND top 20.000000 mm 20.000000 mm, size 1.500000 mm\n"),
        "{out}"
    );
    assert!(out.contains("Nets without access:\n  SIG\n"), "{out}");
    assert!(out.contains("warning: PHOENIX-TP-006"), "{out}");
    // only the board is in the folder
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn the_severity_of_a_net_without_access_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let strict = write_text(dir.path(), "tp.cfg", "uncovered_severity = error\n");
    let (code, out, _) = command(&["testpoints", "--config", &strict, &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error: PHOENIX-TP-006"), "{out}");
    let none = write_text(dir.path(), "none.cfg", "required_per_net = 0\n");
    let (code, out, _) = command(&["testpoints", "--config", &none, &path]);
    assert_eq!(code, 0);
    assert!(out.contains("Nets without access:\n  none\n"), "{out}");
}

#[test]
fn the_settings_say_what_a_test_point_is() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let other = write_text(dir.path(), "other.cfg", "testpoint_prefixes = J\n");
    let (code, out, _) = command(&["testpoints", "--config", &other, &path]);
    assert_eq!(code, 0);
    assert!(out.contains("Test points: 0 "), "{out}");
    assert!(out.contains("PHOENIX-TP-012"), "{out}");
}

#[test]
fn the_json_report_has_the_numbers_of_the_board() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, err) = command(&["testpoints", "--json", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"format\": \"phoenix-testpoints/1\""),
        "{out}"
    );
    assert!(out.contains("\"nets_asked\": 2"), "{out}");
    assert!(out.contains("\"coverage_x100\": 5000"), "{out}");
    let (again, same, _) = command(&["testpoints", "--json", &path]);
    assert_eq!(again, 0);
    assert_eq!(same, out);
}

#[test]
fn the_titles_come_in_the_language_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    let (code, out, _) = command(&["testpoints", "--lang", "es", &path]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("Puntos de prueba de la placa: tp\n"),
        "{out}"
    );
    assert!(out.contains("Redes sin acceso:\n  SIG\n"), "{out}");
    let (_, pt_out, _) = command(&["testpoints", "--lang", "pt-PT", &path]);
    assert!(
        pt_out.starts_with("Pontos de teste da placa: tp\n"),
        "{pt_out}"
    );
}

#[test]
fn bad_arguments_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board());
    assert_eq!(command(&["testpoints"]).0, 2);
    assert_eq!(command(&["testpoints", "--bogus", &path]).0, 2);
    assert_eq!(command(&["testpoints", "--lang", "xx", &path]).0, 2);
    assert_eq!(command(&["testpoints", "--config"]).0, 2);
    assert_eq!(command(&["testpoints", &path, &path]).0, 2);
    let bad = write_text(dir.path(), "bad.cfg", "probe_side = left\n");
    let (code, _, err) = command(&["testpoints", "--config", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-TP-001"), "{err}");
    let small = write_text(dir.path(), "small.cfg", "max_items = 2\n");
    let (code, _, err) = command(&["testpoints", "--config", &small, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-TP-005"), "{err}");
    let missing = dir.path().join("none.json");
    let (code, _, err) = command(&["testpoints", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-TP-003"), "{err}");
}

#[test]
fn the_help_of_the_command_is_in_the_table() {
    let (code, out, _) = command(&["help", "testpoints", "--lang", "de"]);
    assert_eq!(code, 0);
    assert!(out.contains("phoenix testpoints"), "{out}");
    let (code, listing, _) = command(&["help", "manufacturing"]);
    assert_eq!(code, 0);
    assert!(listing.contains("testpoints"), "{listing}");
}
