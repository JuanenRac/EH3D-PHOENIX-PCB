// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `annotate`, `sync --diff` and the options of `erc` on files of the design model
//! written by the tests.
use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Component, Footprint, Net, Pad, PadNet, PadShape, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_schematic::{NetPin, Part, Pin, PinKind, Schematic, SchematicNet, Symbol};

fn id(n: u32) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A schematic with R1 and R2 of one two-pin symbol (the pins are passive, the second pin of R2 is
/// on no net), a net N that joins pin 1 of both, and a footprint for both.
fn schematic() -> Schematic {
    let mut s = Schematic::default();
    let mut pins = BTreeMap::new();
    for (pin, number) in [(2, "1"), (3, "2")] {
        pins.insert(
            id(pin),
            Pin {
                number: number.into(),
                at: None,
            },
        );
        s.pin_kinds.insert(id(pin), PinKind::Passive);
    }
    s.symbols.insert(
        id(1),
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    for (part, refdes) in [(10, "R1"), (11, "R2")] {
        s.parts.insert(
            id(part),
            Part {
                refdes: refdes.into(),
                value: "1k".into(),
                symbol: id(1),
                footprint: Some(id(20)),
                placement: None,
            },
        );
    }
    let mut net_pins = vec![
        NetPin {
            part: id(10),
            pin: id(2),
        },
        NetPin {
            part: id(11),
            pin: id(2),
        },
        NetPin {
            part: id(10),
            pin: id(3),
        },
    ];
    net_pins.sort();
    s.nets.insert(
        id(30),
        SchematicNet {
            name: "N".into(),
            pins: net_pins,
        },
    );
    s
}

/// A board with the footprint and a component for R1 only; R1 has a different value.
fn board() -> Board {
    let mut b = Board::default();
    let mut pads = BTreeMap::new();
    for (pad, number) in [(21, "1"), (22, "2")] {
        pads.insert(
            id(pad),
            Pad {
                number: number.into(),
                shape: PadShape::Circle {
                    diameter: Nm::new(1_000_000).unwrap(),
                },
                at: pt(0, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![],
                roundness: 0,
                plated: true,
            },
        );
    }
    b.footprints.insert(
        id(20),
        Footprint {
            name: "R0603".into(),
            pads,
        },
    );
    b.nets.insert(id(40), Net { name: "N".into() });
    b.components.insert(
        id(50),
        Component {
            refdes: "R1".into(),
            value: "2k".into(),
            symbol: Some(id(10)),
            footprint: id(20),
            at: pt(1_000_000, 1_000_000),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: id(50),
        pad: id(21),
        net: id(40),
    });
    b
}

struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn new() -> Files {
        Files {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_str().unwrap().to_owned()
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

/// The code of the plan that a dry run printed, read from its JSON.
fn code_of(schematic: &str, board: &str, extra: &[&str]) -> String {
    let mut args = vec!["annotate", "--json"];
    args.extend_from_slice(extra);
    args.push(schematic);
    args.push(board);
    let (_, json, _) = call(&args);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["digest"].as_str().unwrap().to_owned()
}

#[test]
fn annotate_is_a_dry_run_that_prints_the_plan_and_its_code_and_exits_1() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let before = std::fs::read(&sch).unwrap();
    let (code, out, _) = call(&["annotate", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    // R1 has another value on the board; R2 has no component and is only listed
    assert!(
        out.contains("[applicable] PHOENIX-ANNOT-002 Value differs on the board: R1 | 1k -> 2k"),
        "{out}"
    );
    assert!(
        out.contains("[ignored] PHOENIX-ANNOT-005 Part has no component on the board: R2"),
        "{out}"
    );
    assert!(out.contains("Confirmation code: "), "{out}");
    assert_eq!(std::fs::read(&sch).unwrap(), before);
    // the same files give the same text, and every language has its words
    assert_eq!(call(&["annotate", sch.as_str(), brd.as_str()]).1, out);
    let (_, pt, _) = call(&["annotate", "--lang", "pt", sch.as_str(), brd.as_str()]);
    assert!(pt.contains("O valor é diferente na placa"), "{pt}");
    assert!(pt.contains("Código de confirmação"), "{pt}");
    let (code, json, _) = call(&["annotate", "--json", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-annotate/1");
    assert_eq!(value["applicable"], 1);
    assert_eq!(value["ignored"], 1);
}

#[test]
fn annotate_applies_to_a_new_file_only_with_the_code_of_the_plan() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let out_path = f.path("s2.json");
    let digest = code_of(&sch, &brd, &[]);

    // no code, or another code: nothing is written
    let (code, _, err) = call(&[
        "annotate",
        "--apply",
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("--confirm"), "{err}");
    let (code, _, err) = call(&[
        "annotate",
        "--apply",
        "--confirm",
        "0000",
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ANNOT-011"), "{err}");
    assert!(!std::path::Path::new(&out_path).exists());

    // with the code the copy is written; the input is not touched
    let before = std::fs::read(&sch).unwrap();
    let (code, _, err) = call(&[
        "annotate",
        "--apply",
        "--confirm",
        digest.as_str(),
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("wrote"), "{err}");
    assert_eq!(std::fs::read(&sch).unwrap(), before);
    let written = Schematic::from_bytes(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(written.parts[&id(10)].value, "2k");
    assert_eq!(written.parts[&id(11)].value, "1k");
    assert_eq!(written.nets, schematic().nets);

    // a file that exists is never written again, and an input is never an output
    let (code, _, err) = call(&[
        "annotate",
        "--apply",
        "--confirm",
        digest.as_str(),
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("exists already"), "{err}");
    let (code, _, err) = call(&[
        "annotate",
        "--apply",
        "--confirm",
        digest.as_str(),
        "--out",
        sch.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("input"), "{err}");

    // the copy agrees with the board: only what the board does not have is left, and ignored
    let (code, out, _) = call(&["annotate", out_path.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("[applicable]"), "{out}");
}

#[test]
fn annotate_renumbers_by_position_and_writes_the_renamed_board_to_a_new_file() {
    let f = Files::new();
    // the component of R1 is called R7 on the board
    let mut renamed = board();
    renamed.components.get_mut(&id(50)).unwrap().refdes = "R7".into();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &renamed.to_bytes().unwrap());
    // without renumbering the board name is written into the schematic
    let (code, out, _) = call(&["annotate", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-ANNOT-001"), "{out}");
    assert!(out.contains("R1 | R1 -> R7"), "{out}");
    // with it the board is numbered by position (R2 of the schematic has no component and its
    // number is kept), so the schematic keeps R1 and the board is renamed
    let (code, out, _) = call(&["annotate", "--renumber", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-ANNOT-007"), "{out}");
    let digest = code_of(&sch, &brd, &["--renumber"]);
    let sch_out = f.path("s2.json");
    let brd_out = f.path("b2.json");
    let (code, _, err) = call(&[
        "annotate",
        "--renumber",
        "--apply",
        "--confirm",
        digest.as_str(),
        "--out",
        sch_out.as_str(),
        "--board-out",
        brd_out.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let written = Board::from_bytes(&std::fs::read(&brd_out).unwrap()).unwrap();
    assert_eq!(written.components[&id(50)].refdes, "R1");
    let kept = Schematic::from_bytes(&std::fs::read(&sch_out).unwrap()).unwrap();
    assert_eq!(kept.parts[&id(10)].refdes, "R1");
    // the original board is untouched
    assert_eq!(
        Board::from_bytes(&std::fs::read(&brd).unwrap()).unwrap(),
        renamed
    );
    // the renamed board agrees with the schematic about the references
    let (_, out, _) = call(&["annotate", "--renumber", sch_out.as_str(), brd_out.as_str()]);
    assert!(!out.contains("PHOENIX-ANNOT-007"), "{out}");
}

#[test]
fn annotate_reads_its_settings_from_a_file_and_refuses_what_it_cannot_use() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let cfg = f.write("a.cfg", b"values = false\n");
    let (code, out, _) = call(&[
        "annotate",
        "--config",
        cfg.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    // the value is left out, so only ignored items remain
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("[ignored] PHOENIX-ANNOT-009 Change left out by the settings"),
        "{out}"
    );
    let bad = f.write("bad.cfg", b"order = sideways\n");
    let (code, _, err) = call(&[
        "annotate",
        "--config",
        bad.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("line 1"), "{err}");
    for args in [
        vec!["annotate"],
        vec!["annotate", sch.as_str()],
        vec!["annotate", "--lang", "xx", sch.as_str(), brd.as_str()],
        vec!["annotate", "--nothing", sch.as_str(), brd.as_str()],
        vec!["annotate", "missing.json", brd.as_str()],
        vec!["annotate", "--out", "x.json", sch.as_str(), brd.as_str()],
        vec![
            "annotate",
            "--board-out",
            "x.brd",
            sch.as_str(),
            brd.as_str(),
        ],
    ] {
        assert_eq!(call(&args).0, 2, "{args:?}");
    }
}

#[test]
fn sync_diff_lists_the_differences_and_exits_1() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let (code, out, _) = call(&["sync", "--diff", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    // R2 has no component and R1 has another value
    assert!(out.contains("PHOENIX-ANNOT-013"), "{out}");
    assert!(out.contains("PHOENIX-ANNOT-015"), "{out}");
    assert!(out.contains("schematic: 1k | board: 2k"), "{out}");
    assert_eq!(call(&["sync", "--diff", sch.as_str(), brd.as_str()]).1, out);
    let (_, json, _) = call(&["sync", "--diff", "--json", sch.as_str(), brd.as_str()]);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-diff/1");
    assert_eq!(value["in_sync"], false);
    let (_, es, _) = call(&["sync", "--diff", "--lang", "es", sch.as_str(), brd.as_str()]);
    assert!(es.contains("Los valores difieren"), "{es}");
    // it only compares
    for extra in ["--apply", "--package", "--backward"] {
        let (code, _, err) = call(&["sync", "--diff", extra, sch.as_str(), brd.as_str()]);
        assert_eq!(code, 2, "{extra}");
        assert!(err.contains("--diff"), "{err}");
    }
}

#[test]
fn erc_takes_advanced_options_and_the_command_line_wins_over_the_file() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    // the net N of the sample joins two passive pins and nothing drives it, which is only looked
    // for when the rule is on
    let (_, out, _) = call(&["erc", sch.as_str()]);
    assert!(!out.contains("PHOENIX-ERC-017"), "{out}");
    let (_, out, _) = call(&["erc", "--advanced", sch.as_str()]);
    assert!(
        out.contains("[warning] PHOENIX-ERC-017 Net has no driver: net N"),
        "{out}"
    );
    // a severity changes one rule: the open pin is only a warning now, so the exit code is 0
    let (code, out, _) = call(&["erc", "--severity", "pin_unconnected=warning", sch.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("[warning] PHOENIX-ERC-001"), "{out}");
    let (code, out, _) = call(&[
        "erc",
        "--advanced",
        "--severity",
        "pin_unconnected=off",
        "--severity",
        "net_without_driver=error",
        sch.as_str(),
    ]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("[error] PHOENIX-ERC-017"), "{out}");
    assert!(!out.contains("PHOENIX-ERC-001"), "{out}");
    // a cell of the matrix and a power flag are accepted
    let (code, _, err) = call(&[
        "erc",
        "--matrix",
        "input.input=warning",
        "--power-flag",
        "VBUS",
        "--severity",
        "pin_unconnected=off",
        sch.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    // what is not valid is refused with the line
    for args in [
        vec!["erc", "--severity", "nothing=error", sch.as_str()],
        vec!["erc", "--severity", "pin_unconnected", sch.as_str()],
        vec!["erc", "--matrix", "input.nothing=error", sch.as_str()],
        vec!["erc", "--severity"],
        vec!["erc", "--power-flag"],
    ] {
        assert_eq!(call(&args).0, 2, "{args:?}");
    }
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let (code, _, err) = call(&["sync", "--advanced", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("erc"), "{err}");
}
