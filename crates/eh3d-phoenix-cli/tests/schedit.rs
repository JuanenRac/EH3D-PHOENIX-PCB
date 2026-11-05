// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `schedit` on files written by the tests: a dry run, a new output file, the refusals
//! and the JSON report.

use std::path::Path;

use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::{
    orthogonal_path, EditOp, Editor, Id, PinDef, PinKind, Schematic, SymbolDef,
};

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn resistor() -> SymbolDef {
    SymbolDef {
        name: "R".into(),
        pins: ["1", "2"]
            .iter()
            .enumerate()
            .map(|(i, number)| PinDef {
                number: (*number).into(),
                at: pt(if i == 0 { -2_540_000 } else { 2_540_000 }, 0),
                kind: Some(PinKind::Passive),
            })
            .collect(),
    }
}

/// Two resistors whose pins are all joined by wires, so the rule check proves no error.
fn schematic() -> (Schematic, Id, Id) {
    let mut ed = Editor::new(Schematic {
        name: "T".into(),
        ..Schematic::default()
    });
    let sheet = ed
        .apply(&EditOp::AddSheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        })
        .unwrap()
        .created[0];
    let mut parts = Vec::new();
    for x in [20, 40] {
        let made = ed
            .apply(&EditOp::PlaceSymbol {
                symbol: "R".into(),
                define: Some(resistor()),
                sheet,
                at: pt(x * 1_000_000, 20_000_000),
                rotation: None,
                mirrored: false,
                refdes: None,
                value: Some("1k".into()),
            })
            .unwrap();
        let part = *made
            .created
            .iter()
            .find(|id| ed.schematic().parts.contains_key(*id))
            .unwrap();
        parts.push(part);
    }
    for path in [
        orthogonal_path(pt(22_540_000, 20_000_000), pt(37_460_000, 20_000_000), true),
        vec![
            pt(17_460_000, 20_000_000),
            pt(17_460_000, 10_000_000),
            pt(42_540_000, 10_000_000),
            pt(42_540_000, 20_000_000),
        ],
    ] {
        ed.apply(&EditOp::AddWire { sheet, path }).unwrap();
    }
    (ed.into_schematic(), parts[0], parts[1])
}

struct Files {
    dir: tempfile::TempDir,
    schematic: String,
    r1: Id,
    r2: Id,
}

fn files() -> Files {
    let dir = tempfile::tempdir().unwrap();
    let (s, r1, r2) = schematic();
    let path = dir.path().join("in.json");
    std::fs::write(&path, s.to_bytes().unwrap()).unwrap();
    Files {
        schematic: path.to_str().unwrap().to_owned(),
        dir,
        r1,
        r2,
    }
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn without_out_nothing_is_written_and_the_report_says_what_changed() {
    let f = files();
    let before = std::fs::read(&f.schematic).unwrap();
    let ops = write(
        f.dir.path(),
        "ops.json",
        &format!(r#"[{{"op":"set_value","part":"{}","value":"4k7"}}]"#, f.r1),
    );
    let (code, out, err) = call(&["schedit", "--apply", &ops, &f.schematic]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("operations applied: 1"), "{out}");
    assert!(out.contains("dry run"), "{out}");
    assert_eq!(std::fs::read(&f.schematic).unwrap(), before);
}

#[test]
fn out_writes_a_new_file_and_never_an_existing_one_or_an_input() {
    let f = files();
    let ops = write(
        f.dir.path(),
        "ops.json",
        &format!(
            r#"{{"ops":[{{"op":"set_value","part":"{}","value":"4k7"}}]}}"#,
            f.r2
        ),
    );
    let new_file = f.dir.path().join("new.json");
    let target = new_file.to_str().unwrap();
    let (code, out, err) = call(&["schedit", "--apply", &ops, "--out", target, &f.schematic]);
    assert_eq!(code, 0, "{out}{err}");
    let written = Schematic::from_bytes(&std::fs::read(&new_file).unwrap()).unwrap();
    assert_eq!(written.parts[&f.r2].value, "4k7");
    // the same output again, the input, the operations and a format that is not written
    for refused in [
        target.to_owned(),
        f.schematic.clone(),
        ops.clone(),
        f.dir.path().join("x.txt").to_str().unwrap().to_owned(),
    ] {
        let (code, _, err) = call(&["schedit", "--apply", &ops, "--out", &refused, &f.schematic]);
        assert_eq!(code, 2, "{refused}");
        assert!(err.contains("PHOENIX-SCHEDIT-003"), "{err}");
    }
}

#[test]
fn a_refused_operation_writes_nothing_and_names_its_number() {
    let f = files();
    let ops = write(
        f.dir.path(),
        "ops.json",
        &format!(
            r#"[{{"op":"set_value","part":"{}","value":"1"}},{{"op":"set_refdes","part":"{}","refdes":"R1"}}]"#,
            f.r1, f.r2
        ),
    );
    let new_file = f.dir.path().join("never.json");
    let (code, out, err) = call(&[
        "schedit",
        "--apply",
        &ops,
        "--out",
        new_file.to_str().unwrap(),
        &f.schematic,
    ]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("operation 2 was refused"), "{err}");
    assert!(!new_file.exists());
}

#[test]
fn the_json_report_has_the_counts_and_the_rule_check() {
    let f = files();
    let ops = write(f.dir.path(), "ops.json", "[]");
    let (code, out, _) = call(&["schedit", "--json", "--apply", &ops, &f.schematic]);
    assert_eq!(code, 0, "{out}");
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(value["schema"], "phoenix-schedit/1");
    assert_eq!(value["applied"], 0);
    assert_eq!(value["parts"], serde_json::json!([2, 2]));
    assert!(value["written"].is_null());
    assert_eq!(value["erc"]["schema"], "phoenix-erc/1");
}

#[test]
fn an_eagle_file_can_be_the_output_and_bad_arguments_exit_2() {
    let f = files();
    let ops = write(f.dir.path(), "ops.json", "[]");
    let eagle = f.dir.path().join("out.sch");
    let (code, out, err) = call(&[
        "schedit",
        "--apply",
        &ops,
        "--out",
        eagle.to_str().unwrap(),
        &f.schematic,
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let text = std::fs::read_to_string(&eagle).unwrap();
    assert!(text.contains("eagle"));
    for list in [
        &["schedit"][..],
        &["schedit", "--apply", "x"],
        &["schedit", "--bogus", "a"],
        &["schedit", "--apply"],
    ] {
        let (code, out, _) = call(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
    }
    let (code, _, err) = call(&["schedit", "--apply", &ops, "/nonexistent/in.json"]);
    assert_eq!(code, 2);
    assert!(err.contains("cannot read"), "{err}");
}
