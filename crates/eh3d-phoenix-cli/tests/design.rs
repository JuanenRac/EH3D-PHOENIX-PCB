// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `erc` and `sync` on files of the design model written by the tests.

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

#[test]
fn erc_finds_the_open_pin_and_exits_1() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let (code, out, _) = call(&["erc", sch.as_str()]);
    assert_eq!(code, 1, "{out}");
    // pin 2 of R2 is on no net; both parts have a footprint and a value
    assert!(out.contains("Confirmed: 1"), "{out}");
    assert!(
        out.contains("PHOENIX-ERC-001 Pin is not connected: R2.2"),
        "{out}"
    );
    assert!(out.contains("Inconclusive: 0"));
    // the same file twice gives the same text, and every language has its title
    assert_eq!(call(&["erc", sch.as_str()]).1, out);
    let (_, es, _) = call(&["erc", "--lang", "es", sch.as_str()]);
    assert!(es.contains("Pin sin conectar"), "{es}");
    let (_, pt, _) = call(&["erc", "--lang", "pt", sch.as_str()]);
    assert!(pt.contains("Pino sem ligação"), "{pt}");
    let (code, json, _) = call(&["erc", "--json", sch.as_str()]);
    assert_eq!(code, 1);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-erc/1");
    assert_eq!(value["confirmed"][0]["code"], "PHOENIX-ERC-001");
}

#[test]
fn erc_settings_come_from_a_file() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let cfg = f.write("erc.cfg", b"severity.pin_unconnected = warning\n");
    let (code, out, _) = call(&["erc", "--config", cfg.as_str(), sch.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("[warning] PHOENIX-ERC-001"));
    let off = f.write("off.cfg", b"severity.pin_unconnected = off\n");
    let (code, out, _) = call(&["erc", "--config", off.as_str(), sch.as_str()]);
    assert_eq!(code, 0);
    assert!(out.contains("nothing found"), "{out}");
    let bad = f.write("bad.cfg", b"severity.nothing = error\n");
    let (code, _, err) = call(&["erc", "--config", bad.as_str(), sch.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("line 1"), "{err}");
}

#[test]
fn erc_refuses_what_it_cannot_use() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    for args in [
        vec!["erc"],
        vec!["erc", "--lang", "xx", sch.as_str()],
        vec!["erc", "--nothing", sch.as_str()],
        vec!["erc", "missing.json"],
    ] {
        let (code, _, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(!err.is_empty());
    }
    let broken = f.write("broken.json", b"{not json");
    assert_eq!(call(&["erc", broken.as_str()]).0, 2);
}

#[test]
fn sync_is_a_dry_run_that_exits_1_while_there_is_something_to_do() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let before = std::fs::read(&brd).unwrap();
    let (code, out, _) = call(&["sync", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    // R2 is new, R1 has another value, the net N gets the pads of R2 and the second pad of R1
    assert!(out.contains("PHOENIX-SYNC-001"), "{out}");
    assert!(out.contains("PHOENIX-SYNC-004"), "{out}");
    assert!(out.contains("PHOENIX-SYNC-008"), "{out}");
    assert!(out.contains("2k -> 1k"), "{out}");
    // a dry run writes nothing
    assert_eq!(std::fs::read(&brd).unwrap(), before);
    let (code, json, _) = call(&["sync", "--json", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-sync/1");
    assert_eq!(value["direction"], "forward");
    let (_, package, _) = call(&["sync", "--package", sch.as_str(), brd.as_str()]);
    let value: serde_json::Value = serde_json::from_str(&package).unwrap();
    assert_eq!(value["schema"], "phoenix-sync-package/1");
    assert!(value["operations"]
        .as_array()
        .is_some_and(|a| !a.is_empty()));
    let (_, es, _) = call(&["sync", "--lang", "es", sch.as_str(), brd.as_str()]);
    assert!(es.contains("Los valores son distintos"), "{es}");
}

#[test]
fn sync_applies_to_a_new_file_and_a_second_run_finds_nothing() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let out_path = f.path("b2.json");
    let (code, _, err) = call(&[
        "sync",
        "--apply",
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("wrote"));
    let applied = Board::from_bytes(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(applied.components.len(), 2);
    assert!(applied.components.values().any(|c| c.refdes == "R2"));
    assert!(applied.components.values().all(|c| c.value == "1k"));
    // the original file is untouched and the temporary file is gone
    assert_eq!(
        Board::from_bytes(&std::fs::read(&brd).unwrap()).unwrap(),
        board()
    );
    assert!(!std::path::Path::new(&format!("{out_path}.tmp")).exists());
    // the result agrees with the schematic: nothing is left to do
    let (code, out, _) = call(&["sync", sch.as_str(), out_path.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("Confirmed: 0"), "{out}");
}

#[test]
fn sync_backward_writes_the_board_into_the_schematic() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let (code, out, _) = call(&["sync", "--backward", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("PHOENIX-SYNC-004") && out.contains("1k -> 2k"),
        "{out}"
    );
    let out_path = f.path("s2.json");
    let (code, _, err) = call(&[
        "sync",
        "--backward",
        "--apply",
        "--out",
        out_path.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let written = Schematic::from_bytes(&std::fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(written.parts[&id(10)].value, "2k");
    assert_eq!(written.parts[&id(11)].value, "1k");
    assert_eq!(written.nets, schematic().nets);
    let (code, out, _) = call(&["sync", "--backward", out_path.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn sync_never_overwrites_an_input_and_refuses_what_it_cannot_use() {
    let f = Files::new();
    let sch = f.write("s.json", &schematic().to_bytes().unwrap());
    let brd = f.write("b.json", &board().to_bytes().unwrap());
    let (code, _, err) = call(&["sync", "--apply", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("--out"), "{err}");
    let (code, _, err) = call(&[
        "sync",
        "--apply",
        "--out",
        brd.as_str(),
        sch.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("input"), "{err}");
    for args in [
        vec!["sync"],
        vec!["sync", sch.as_str()],
        vec!["sync", "--lang", "xx", sch.as_str(), brd.as_str()],
        vec!["sync", "missing.json", brd.as_str()],
    ] {
        assert_eq!(call(&args).0, 2, "{args:?}");
    }
    let cfg = f.write("bad.cfg", b"new_part_side = left\n");
    let (code, _, err) = call(&["sync", "--config", cfg.as_str(), sch.as_str(), brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("line 1"), "{err}");
}

#[test]
fn check_reads_net_classes_from_a_rule_file_of_the_second_format() {
    use eh3d_phoenix_board::{Layer, LayerKind, Segment, Track};
    use eh3d_phoenix_eagle_board::write_board;
    let f = Files::new();
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
    b.nets.insert(id(10), Net { name: "VCC".into() });
    b.nets.insert(id(11), Net { name: "SIG".into() });
    for (t, net, y) in [(20, id(10), 2_000_000), (21, id(11), 6_000_000)] {
        b.tracks.insert(
            id(t),
            Track {
                net,
                layer: id(1),
                width: Nm::new(200_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(2_000_000, y),
                    end: pt(8_000_000, y),
                }],
            },
        );
    }
    let brd = f.write("b.brd", write_board(&b).text.as_bytes());
    let rules = f.write(
        "r.json",
        br#"{"format":"phoenix-rules/2","rules":{},
            "classes":[{"name":"power","nets":["VCC"],"min_track_width_nm":300000}]}"#,
    );
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-CHECK-014"), "{out}");
    assert!(out.contains("class power"), "{out}");
    assert!(out.contains("Confirmed: 1"), "{out}");
    // a file of the first format with the same rules in it finds nothing
    let plain = f.write("p.json", br#"{"format":"phoenix-rules/1","rules":{}}"#);
    let (code, out, _) = call(&["check", "--rules", plain.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out}");
}
