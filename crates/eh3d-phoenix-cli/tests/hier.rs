// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `hier`, and the netlist, the simulation and the rule check of a schematic with sheet
//! symbols, on a design written by the test itself: a divider sheet (1k on top of 3k) shown twice
//! by the sheet symbols U1 and U2, both fed by a source of 10 V.
//!
//! The expected voltage of both outputs is 10 V times 3k / (1k + 3k) = 7.5 V.

use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::{EditOp, Editor, Id, PinDef, PinKind, PortDir, Schematic, SymbolDef};

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
}

fn mm(v: i64) -> Nm {
    Nm::new(v * 1_000_000).unwrap()
}

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

fn symbol(name: &str) -> SymbolDef {
    let pin = |number: &str, y: i64| PinDef {
        number: number.into(),
        at: pt(0, y),
        kind: Some(PinKind::Passive),
    };
    SymbolDef {
        name: name.into(),
        pins: vec![pin("1", 5), pin("2", -5)],
    }
}

fn apply(ed: &mut Editor, op: EditOp) -> Vec<Id> {
    ed.apply(&op)
        .unwrap_or_else(|e| panic!("{op:?}: {e}"))
        .created
}

fn place(ed: &mut Editor, sheet: Id, kind: &str, at: Point, refdes: &str, value: &str) {
    apply(
        ed,
        EditOp::PlaceSymbol {
            symbol: kind.into(),
            define: Some(symbol(kind)),
            sheet,
            at,
            rotation: None,
            mirrored: false,
            refdes: Some(refdes.into()),
            value: Some(value.into()),
        },
    );
}

fn design() -> Schematic {
    let mut ed = Editor::new(Schematic {
        name: "Two dividers".into(),
        ..Schematic::default()
    });
    let sheet = |ed: &mut Editor, name: &str| {
        apply(
            ed,
            EditOp::AddSheet {
                name: name.into(),
                width: mm(297),
                height: mm(210),
            },
        )[0]
    };
    let top = sheet(&mut ed, "Top");
    let child = sheet(&mut ed, "Divider");
    place(&mut ed, child, "R", pt(50, 80), "R1", "1k");
    place(&mut ed, child, "R", pt(50, 60), "R2", "3k");
    apply(
        &mut ed,
        EditOp::AddWire {
            sheet: child,
            path: vec![pt(50, 75), pt(50, 70), pt(50, 65)],
        },
    );
    apply(
        &mut ed,
        EditOp::AddPowerPort {
            sheet: child,
            at: pt(50, 55),
            name: "GND".into(),
        },
    );
    for (name, dir, at) in [
        ("IN", PortDir::Input, pt(50, 85)),
        ("OUT", PortDir::Output, pt(50, 70)),
    ] {
        apply(
            &mut ed,
            EditOp::AddSheetPort {
                sheet: child,
                name: name.into(),
                dir,
                at,
            },
        );
    }
    place(&mut ed, top, "V", pt(20, 100), "V1", "10");
    apply(
        &mut ed,
        EditOp::AddPowerPort {
            sheet: top,
            at: pt(20, 95),
            name: "GND".into(),
        },
    );
    for (name, y) in [("U1", 100), ("U2", 70)] {
        let block = apply(
            &mut ed,
            EditOp::AddSheetBlock {
                sheet: top,
                child,
                name: name.into(),
                at: pt(60, y),
                width: mm(20),
                height: mm(20),
            },
        )[0];
        for (pin, dir, x) in [("IN", PortDir::Input, 60), ("OUT", PortDir::Output, 80)] {
            apply(
                &mut ed,
                EditOp::AddHierPin {
                    block,
                    name: pin.into(),
                    dir,
                    at: pt(x, y + 5),
                },
            );
        }
    }
    for path in [
        vec![pt(20, 105), pt(60, 105)],
        vec![pt(40, 105), pt(40, 75), pt(60, 75)],
    ] {
        apply(&mut ed, EditOp::AddWire { sheet: top, path });
    }
    for (at, name) in [
        (pt(20, 105), "VIN"),
        (pt(80, 105), "VOUT1"),
        (pt(80, 75), "VOUT2"),
    ] {
        apply(
            &mut ed,
            EditOp::AddLabel {
                sheet: top,
                at,
                name: name.into(),
            },
        );
    }
    ed.into_schematic()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path.to_str().unwrap().to_owned()
}

/// The value after the comma on the line of a probe in the table `sim --op` prints.
fn probed(table: &str, probe: &str) -> f64 {
    let line = table
        .lines()
        .find(|l| l.starts_with(&format!("{probe},")))
        .unwrap_or_else(|| panic!("no {probe} in {table}"));
    line.split_once(',').unwrap().1.trim().parse().unwrap()
}

#[test]
fn hier_lists_the_instances_the_nets_and_no_problem() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "two.json", &design().to_bytes().unwrap());
    let (code, out, err) = command(&["hier", &file]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("3 instances"), "{out}");
    assert!(out.contains("/U1  Divider (symbol U1)"), "{out}");
    assert!(out.contains("/U2  Divider (symbol U2)"), "{out}");
    assert!(out.contains("/VOUT1  2 pins  /U1"), "{out}");
    assert!(out.contains("problems: none"), "{out}");
    // a read-only command: the folder holds only the input
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    // and two runs give the same text
    let (_, again, _) = command(&["hier", &file]);
    assert_eq!(out, again);
}

#[test]
fn hier_in_json_has_the_tree_the_nets_and_the_titles_in_the_language_asked() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "two.json", &design().to_bytes().unwrap());
    let (code, out, err) = command(&["hier", "--json", &file]);
    assert_eq!(code, 0, "{err}");
    let report: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(report["sheets"], 2);
    assert_eq!(report["sheet_symbols"], 2);
    assert_eq!(report["parts"], 5);
    let names: Vec<&str> = report["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["", "/U1", "/U2"]);
    let mut nets: Vec<&str> = report["nets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap())
        .collect();
    nets.sort_unstable();
    assert_eq!(nets, vec!["/VIN", "/VOUT1", "/VOUT2", "GND"]);
    assert!(report["problems"].as_array().unwrap().is_empty());

    // a pin without its port: exit 1, the code and the title in Portuguese
    let mut broken = design();
    let pin = *broken.hier_pins.keys().next().unwrap();
    broken.hier_pins.get_mut(&pin).unwrap().name = "EN".into();
    let bad = write(dir.path(), "bad.json", &broken.to_bytes().unwrap());
    let (code, out, _) = command(&["hier", "--json", "--lang", "pt", &bad]);
    assert_eq!(code, 1, "{out}");
    let report: serde_json::Value = serde_json::from_str(&out).unwrap();
    let problems = report["problems"].as_array().unwrap();
    assert!(problems.iter().any(|p| p["code"] == "PHOENIX-HIER-004"
        && p["title"] == "Um pino do símbolo de folha não tem porta na folha que mostra"));
    assert!(problems.iter().any(|p| p["code"] == "PHOENIX-HIER-005"));
}

#[test]
fn hier_takes_its_settings_from_a_file_and_refuses_a_bad_one() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "two.json", &design().to_bytes().unwrap());
    let config = write(dir.path(), "hier.cfg", b"net_separator = .\n");
    let (code, out, err) = command(&["hier", "--config", &config, &file]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains(".U1.VOUT1") || out.contains("VOUT1"), "{out}");
    assert!(out.contains(".U1  Divider"), "{out}");
    let bad = write(dir.path(), "bad.cfg", b"net_separator = a b\n");
    let (code, _, err) = command(&["hier", "--config", &bad, &file]);
    assert_eq!(code, 2);
    assert!(err.contains("net_separator"), "{err}");
    let (code, _, err) = command(&["hier"]);
    assert_eq!(code, 2);
    assert!(err.contains("phoenix hier"), "{err}");
    let (code, _, err) = command(&["hier", "--colour", &file]);
    assert_eq!(code, 2);
    assert!(err.contains("unknown option"), "{err}");
}

#[test]
fn the_simulation_of_a_sheet_shown_twice_gives_the_analytic_result() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "two.json", &design().to_bytes().unwrap());
    let (code, netlist, err) = command(&["netlist", "--no-erc", &file]);
    assert_eq!(code, 0, "{err}");
    // five elements: the source and the two resistors of each instance
    assert!(err.contains("5 elements translated"), "{err}");
    for element in ["R_U1_R1", "R_U1_R2", "R_U2_R1", "R_U2_R2"] {
        assert!(
            netlist.lines().any(|l| l.starts_with(element)),
            "{element} in {netlist}"
        );
    }
    // the output of each divider is 10 V * 3k / (1k + 3k) = 7.5 V
    for probe in ["v(_vout1)", "v(_vout2)"] {
        let (code, table, err) = command(&[
            "sim",
            "--schematic",
            &file,
            "--no-erc",
            "--op",
            "--probe",
            probe,
        ]);
        assert_eq!(code, 0, "{err}");
        let volts = probed(&table, probe);
        assert!((volts - 7.5).abs() < 1e-9, "{probe} = {volts}");
    }
}

#[test]
fn the_rule_check_names_the_parts_of_every_instance() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "two.json", &design().to_bytes().unwrap());
    let (_, out, err) = command(&["erc", "--json", &file]);
    assert!(err.is_empty(), "{err}");
    // no footprint is given: the warning names the parts of both instances
    for refdes in ["U1/R1", "U1/R2", "U2/R1", "U2/R2", "V1"] {
        assert!(out.contains(refdes), "{refdes} in {out}");
    }
}
