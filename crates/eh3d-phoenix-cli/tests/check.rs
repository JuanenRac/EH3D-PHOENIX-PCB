// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `check` and `connectivity` on EAGLE files written by the tool itself.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_eagle_board::{write_board, write_schematic};
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_schematic::{Part, Pin, Schematic, Symbol};

fn id(n: u32) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with one track 0.10 mm wide.
fn board() -> Board {
    let mut b = Board::default();
    for (n, name) in [(1, "Top"), (2, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    b.copper_stack = vec![id(1), id(2)];
    b.outline = vec![
        pt(0, 0),
        pt(20_000_000, 0),
        pt(20_000_000, 20_000_000),
        pt(0, 20_000_000),
    ];
    b.nets.insert(id(10), Net { name: "A".into() });
    b.tracks.insert(
        id(20),
        Track {
            net: id(10),
            layer: id(1),
            width: Nm::new(100_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(2_000_000, 2_000_000),
                end: pt(8_000_000, 2_000_000),
            }],
        },
    );
    b
}

fn schematic() -> Schematic {
    let mut s = Schematic::default();
    let mut pins = BTreeMap::new();
    pins.insert(
        id(2),
        Pin {
            number: "1".into(),
            at: None,
        },
    );
    s.symbols.insert(
        id(1),
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    s.parts.insert(
        id(3),
        Part {
            refdes: "R1".into(),
            value: "1k".into(),
            symbol: id(1),
            footprint: None,
            placement: None,
        },
    );
    s
}

const RULES: &str = r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":150000}}"#;

struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn new() -> Files {
        Files {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn write(&self, name: &str, text: &str) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.to_str().unwrap().to_owned()
    }
}

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

#[test]
fn check_finds_the_narrow_track_and_exits_1() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("Confirmed: 1"), "{out}");
    assert!(out.contains("PHOENIX-CHECK-001"));
    assert!(out.contains("measured 100000 nm") && out.contains("limit 150000 nm"));
    assert!(out.contains("Inconclusive: 0"));
    // the same file twice gives the same text
    assert_eq!(
        call(&["check", "--rules", rules.as_str(), brd.as_str()]).1,
        out
    );
}

#[test]
fn check_prints_json_and_translated_text() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&["check", "--json", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 1);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["passes"], false);
    assert_eq!(json["confirmed"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["confirmed"][0]["measured_nm"], 100_000);
    let (_, es, _) = call(&[
        "check",
        "--lang",
        "es",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert!(es.contains("Pista más estrecha que el mínimo"), "{es}");
    let (_, pt, _) = call(&[
        "check",
        "--lang",
        "pt-PT",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert!(pt.contains("Pista mais estreita do que o mínimo"), "{pt}");
}

#[test]
fn check_exits_0_when_the_limit_is_met() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write(
        "r.json",
        r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":100000}}"#,
    );
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!((code, out.contains("Confirmed: 0")), (0, true), "{out}");
}

#[test]
fn check_takes_the_limits_from_the_design_rules_of_the_board() {
    let f = Files::new();
    let text = write_board(&board()).text;
    let plain = f.write("plain.brd", &text);
    let (code, _, err) = call(&["check", plain.as_str()]);
    assert_eq!(code, 2);
    assert!(
        err.contains("design rules") && err.contains("--rules"),
        "{err}"
    );
    let with_rules = text.replacen(
        "<board>",
        "<board><designrules name=\"x\"><param name=\"msWidth\" value=\"0.15mm\"/></designrules>",
        1,
    );
    assert_ne!(with_rules, text);
    let brd = f.write("rules.brd", &with_rules);
    let (code, out, _) = call(&["check", brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-CHECK-001"));
}

#[test]
fn bad_arguments_and_bad_files_exit_2() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let missing = f.dir.path().join("absent.brd");
    for list in [
        vec!["check"],
        vec!["check", "--lang", "xx", brd.as_str()],
        vec!["check", "--bogus", brd.as_str()],
        vec!["check", "--rules"],
        vec!["connectivity", brd.as_str()],
    ] {
        let (code, out, err) = call(&list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty());
    }
    let (code, _, err) = call(&[
        "check",
        "--rules",
        rules.as_str(),
        missing.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("cannot read"));
    let broken = f.write("broken.brd", "not xml");
    assert_eq!(
        call(&["check", "--rules", rules.as_str(), broken.as_str()]).0,
        2
    );
    let bad_rules = f.write("bad.json", "{}");
    assert_eq!(
        call(&["check", "--rules", bad_rules.as_str(), brd.as_str()]).0,
        2
    );
}

#[test]
fn connectivity_reads_both_files_and_prints_one_report() {
    let f = Files::new();
    let b = board();
    let brd = f.write("b.brd", &write_board(&b).text);
    let sch = f.write("s.sch", &write_schematic(&b, &schematic()).text);
    let (code, out, err) = call(&["connectivity", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out} {err}");
    assert!(
        out.contains("Confirmed: 0") && out.contains("Inconclusive: 0"),
        "{out}"
    );
    let (code, json, _) = call(&["connectivity", "--json", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-connectivity/1");
    let rules = f.write(
        "r.json",
        r#"{"format":"phoenix-rules/1","rules":{"connect_tolerance_nm":1000}}"#,
    );
    assert_eq!(
        call(&[
            "connectivity",
            "--rules",
            rules.as_str(),
            sch.as_str(),
            brd.as_str()
        ])
        .0,
        0
    );
}
