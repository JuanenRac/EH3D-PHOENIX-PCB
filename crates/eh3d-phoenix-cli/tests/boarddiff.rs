// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `boarddiff` on boards written by the test itself: two revisions of a small board
//! are compared, the exit code follows the result, and nothing is written.

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

/// A 20 x 10 mm board with a resistor of the given value, and a second resistor when asked.
fn board(name: &str, value: &str, second: bool) -> Board {
    let mut b = Board {
        name: name.into(),
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
    for (n, number) in [(11, "1"), (12, "2")] {
        resistor.pads.insert(
            id(n),
            Pad {
                number: number.into(),
                shape: PadShape::Circle {
                    diameter: Nm::new(800_000).unwrap(),
                },
                at: pt(if n == 11 { -800_000 } else { 800_000 }, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![id(1)],
                roundness: 0,
                plated: true,
            },
        );
    }
    b.footprints.insert(id(10), resistor);
    let mut parts = vec![(20, "R1", value, 5 * MM)];
    if second {
        parts.push((21, "R2", "1k", 12 * MM));
    }
    for (n, refdes, value, x) in parts {
        b.components.insert(
            id(n),
            Component {
                refdes: refdes.into(),
                value: value.into(),
                symbol: None,
                footprint: id(10),
                at: pt(x, 5 * MM),
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
    b.pad_nets.sort_by_key(PadNet::pad_ref);
    b
}

fn write_board(dir: &Path, file: &str, b: &Board) -> String {
    let path = dir.join(file);
    fs::write(&path, b.to_bytes().unwrap()).unwrap();
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
fn two_boards_that_are_the_same_have_no_difference_and_the_exit_code_is_0() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let b = write_board(dir.path(), "b.json", &board("rev A", "10k", false));
    let (code, out, err) = command(&["boarddiff", &a, &b]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("Board comparison: rev A -> rev A\n"),
        "{out}"
    );
    assert!(out.ends_with("No differences\n"), "{out}");
    assert_eq!(names(dir.path()), vec!["a.json", "b.json"]);
}

#[test]
fn two_revisions_are_compared_and_the_exit_code_is_1() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let b = write_board(dir.path(), "b.json", &board("rev B", "22k", true));
    let (code, out, err) = command(&["boarddiff", &a, &b]);
    assert_eq!(code, 1, "{err}");
    assert!(out.contains("Counts, old to new: parts 1 -> 2"), "{out}");
    assert!(out.contains("Differences: 2"), "{out}");
    assert!(
        out.contains("PHOENIX-DIFF-007: Part added or removed [R2]: added (1k, R_0603, top)"),
        "{out}"
    );
    assert!(
        out.contains("PHOENIX-DIFF-008: Value of a part changed [R1]: 10k -> 22k"),
        "{out}"
    );
    // the other way round the same two differences are the other way round
    let (code, out, _) = command(&["boarddiff", &b, &a]);
    assert_eq!(code, 1);
    assert!(out.contains("[R2]: removed (1k, R_0603, top)"), "{out}");
    assert!(out.contains("[R1]: 22k -> 10k"), "{out}");
    assert_eq!(names(dir.path()), vec!["a.json", "b.json"]);
}

#[test]
fn the_json_report_and_the_language_of_the_words() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let b = write_board(dir.path(), "b.json", &board("rev B", "22k", false));
    let (code, out, _) = command(&["boarddiff", "--json", &a, &b]);
    assert_eq!(code, 1);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["format"], "phoenix-boarddiff/1");
    assert_eq!(j["total"], 1);
    assert_eq!(j["differences"][0]["code"], "PHOENIX-DIFF-008");
    assert_eq!(j["counts"]["parts"][1], 1);
    let (_, text, _) = command(&["boarddiff", "--lang", "pt", &a, &b]);
    assert!(
        text.contains("Comparação de placas: rev A -> rev B"),
        "{text}"
    );
    assert!(
        text.contains("Valor de uma peça alterado [R1]: 10k -> 22k"),
        "{text}"
    );
}

#[test]
fn the_differences_can_be_written_as_a_table_to_a_new_file_and_never_over_one() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let b = write_board(dir.path(), "b.json", &board("rev B", "22k", true));
    let table = dir.path().join("diff.csv");
    let table_text = table.to_str().unwrap();
    let (code, out, err) = command(&["boarddiff", "--csv", table_text, &a, &b]);
    assert_eq!(code, 1, "{err}");
    assert!(out.contains(&format!("written: {table_text}")), "{out}");
    assert_eq!(
        fs::read_to_string(&table).unwrap(),
        "code,subject,detail\nPHOENIX-DIFF-007,R2,\"added (1k, R_0603, top)\"\nPHOENIX-DIFF-008,R1,10k -> 22k\n"
    );
    // a file that is there is kept as it is and the command stops
    fs::write(&table, "keep me").unwrap();
    let (code, out, err) = command(&["boarddiff", "--csv", table_text, &a, &b]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("PHOENIX-DIFF-018"), "{err}");
    assert_eq!(fs::read_to_string(&table).unwrap(), "keep me");
    // with --json the text is not added to
    let other = dir.path().join("other.csv");
    let (code, out, _) = command(&[
        "boarddiff",
        "--json",
        "--csv",
        other.to_str().unwrap(),
        &a,
        &b,
    ]);
    assert_eq!(code, 1);
    assert!(out.starts_with('{') && !out.contains("written:"), "{out}");
    assert!(other.is_file());
}

#[test]
fn a_settings_file_changes_what_is_compared() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let b = write_board(dir.path(), "b.json", &board("rev B", "22k", true));
    let cfg = dir.path().join("diff.cfg");
    fs::write(&cfg, "ignore_parts = R2\n").unwrap();
    let (code, out, _) = command(&["boarddiff", "--config", cfg.to_str().unwrap(), &a, &b]);
    assert_eq!(code, 1);
    assert!(out.contains("Differences: 1"), "{out}");
    assert!(!out.contains("[R2]"), "{out}");
}

#[test]
fn a_wrong_call_a_bad_setting_or_a_missing_file_is_refused_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_board(dir.path(), "a.json", &board("rev A", "10k", false));
    let (code, _, err) = command(&["boarddiff"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("phoenix boarddiff "), "{err}");
    for args in [
        vec!["boarddiff", a.as_str()],
        vec!["boarddiff", a.as_str(), a.as_str(), a.as_str()],
        vec!["boarddiff", "--bogus", a.as_str(), a.as_str()],
        vec!["boarddiff", "--lang", "xx", a.as_str(), a.as_str()],
        vec!["boarddiff", "--config"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-DIFF-002"), "{args:?}: {err}");
    }
    let bad = dir.path().join("bad.cfg");
    fs::write(&bad, "move_tolerance_nm = -1\n").unwrap();
    let (code, _, err) = command(&["boarddiff", "--config", bad.to_str().unwrap(), &a, &a]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-DIFF-001") && err.contains("line 1"),
        "{err}"
    );
    let missing = dir.path().join("missing.json");
    let (code, _, err) = command(&["boarddiff", &a, missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-DIFF-003"), "{err}");
    assert_eq!(names(dir.path()), vec!["a.json", "bad.cfg"]);
}
