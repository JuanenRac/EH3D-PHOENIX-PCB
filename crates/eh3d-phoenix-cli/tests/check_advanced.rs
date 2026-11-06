// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `check` with the advanced checks, on an EAGLE file written by the tool itself.

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

/// Rules that the track of the board keeps (0.10 mm is above 0.05 mm).
const RULES: &str = r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":50000}}"#;

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
fn without_the_option_the_advanced_checks_do_not_run() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("PHOENIX-DRC-"), "{out}");
}

#[test]
fn the_advanced_option_adds_findings_whose_level_is_a_warning_by_default() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&[
        "check",
        "--advanced",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    // a track that joins no pad is a copper island, a warning: the board does not fail
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("[warning] PHOENIX-DRC-009 Copper island joined to no pad"),
        "{out}"
    );
    // the same input gives the same text, and the text can be in Portuguese
    assert_eq!(
        call(&[
            "check",
            "--advanced",
            "--rules",
            rules.as_str(),
            brd.as_str()
        ])
        .1,
        out
    );
    let (_, pt, _) = call(&[
        "check",
        "--advanced",
        "--lang",
        "pt",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert!(pt.contains("Ilha de cobre sem nenhum pad"), "{pt}");
}

#[test]
fn a_settings_file_changes_the_level_and_implies_the_option() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let strict = f.write(
        "d.cfg",
        "island_level = error
",
    );
    let (code, out, _) = call(&[
        "check",
        "--drc-config",
        strict.as_str(),
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("[error] PHOENIX-DRC-009"), "{out}");
    let quiet = f.write(
        "q.cfg",
        "island_level = off
",
    );
    let (code, out, _) = call(&[
        "check",
        "--drc-config",
        quiet.as_str(),
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("PHOENIX-DRC-009"), "{out}");
}

#[test]
fn a_settings_file_that_is_not_valid_is_refused_with_its_code() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let bad = f.write(
        "bad.cfg",
        "no_such_key = 1
",
    );
    let (code, out, err) = call(&[
        "check",
        "--drc-config",
        bad.as_str(),
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(
        err.contains("PHOENIX-DRC-015") && err.contains("line 1"),
        "{err}"
    );
    let (code, _, err) = call(&["check", "--drc-config"]);
    assert_eq!(code, 2);
    assert!(err.contains("needs a file"), "{err}");
}

#[test]
fn the_sarif_layout_lists_the_rules_and_the_results() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&[
        "check",
        "--advanced",
        "--sarif",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{out}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["schema"], "phoenix-drc/1");
    assert_eq!(value["passes"], true);
    assert_eq!(value["rules"][0]["id"], "PHOENIX-DRC-009");
    assert_eq!(value["results"][0]["ruleId"], "PHOENIX-DRC-009");
    assert_eq!(value["results"][0]["level"], "warning");
    assert_eq!(value["results"][0]["kind"], "fail");
}

#[test]
fn the_advanced_options_are_not_for_the_connectivity_command() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let sch = f.write("s.sch", &write_schematic(&schematic()).text);
    let (code, _, err) = call(&["connectivity", "--advanced", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("belong to the command check"), "{err}");
}
