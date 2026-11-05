// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `sanitize` on libraries written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;

fn footprint(name: &str, left_x: &str) -> String {
    format!(
        "<package name=\"{name}\"><description>chip</description>\
<smd name=\"1\" x=\"{left_x}\" y=\"0\" dx=\"1.2\" dy=\"1.4\" layer=\"1\"/>\
<smd name=\"2\" x=\"0.9\" y=\"0\" dx=\"1.2\" dy=\"1.4\" layer=\"1\"/>\
<wire x1=\"-2\" y1=\"1\" x2=\"2\" y2=\"1\" width=\"0.1\" layer=\"21\"/></package>"
    )
}

fn library(packages: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\"?><eagle version=\"9.6.2\"><drawing><library><packages>{}</packages></library></drawing></eagle>",
        packages.join("")
    )
}

fn folder(dir: &Path) {
    fs::write(dir.join("a.lbr"), library(&[footprint("R0805", "-0.9")])).unwrap();
    fs::write(dir.join("b.lbr"), library(&[footprint("RES", "-0.9")])).unwrap();
    fs::write(dir.join("c.lbr"), library(&[footprint("OTHER", "-3.5")])).unwrap();
    fs::write(dir.join("broken.lbr"), "<eagle").unwrap();
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

#[test]
fn the_summary_names_the_group_and_the_skipped_file() {
    let dir = tempfile::tempdir().unwrap();
    folder(dir.path());
    let (code, out, err) = command(&["sanitize", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("libraries: 4 found, 3 read, 1 skipped"),
        "{out}"
    );
    assert!(
        out.contains("P001 package score 100 exact keep a.lbr:R0805"),
        "{out}"
    );
    assert!(out.contains("PHOENIX-SAN-004"), "{out}");
}

#[test]
fn out_writes_the_page_the_json_and_the_plan_and_leaves_the_libraries_alone() {
    let dir = tempfile::tempdir().unwrap();
    folder(dir.path());
    let before: Vec<Vec<u8>> = ["a", "b", "c", "broken"]
        .iter()
        .map(|n| fs::read(dir.path().join(format!("{n}.lbr"))).unwrap())
        .collect();
    let target = tempfile::tempdir().unwrap();
    let base = target.path().join("made").join("report");
    let (code, out, err) = command(&[
        "sanitize",
        "--out",
        base.to_str().unwrap(),
        "--lang",
        "pt",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("report.html") && out.contains("report.plan.json"),
        "{out}"
    );
    let html = fs::read_to_string(target.path().join("made/report.html")).unwrap();
    assert!(html.contains("lang=\"pt-PT\"") && html.contains("Ficheiros lidos"));
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.path().join("made/report.json")).unwrap())
            .unwrap();
    assert_eq!(json["totals"]["groups"][0], 1);
    let plan: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.path().join("made/report.plan.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(plan["applied"], false);
    assert_eq!(plan["groups"][0]["keep"]["name"], "R0805");
    assert_eq!(plan["groups"][0]["replace"][0]["name"], "RES");
    let after: Vec<Vec<u8>> = ["a", "b", "c", "broken"]
        .iter()
        .map(|n| fs::read(dir.path().join(format!("{n}.lbr"))).unwrap())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn the_threshold_decides_what_is_grouped_and_json_prints_the_report() {
    let dir = tempfile::tempdir().unwrap();
    folder(dir.path());
    fs::write(
        dir.path().join("d.lbr"),
        library(&[footprint("NEAR", "-0.92")]),
    )
    .unwrap();
    let folder_arg = dir.path().to_str().unwrap();
    let (code, strict, _) = command(&["sanitize", "--threshold", "100", "--json", folder_arg]);
    assert_eq!(code, 0);
    let strict: serde_json::Value = serde_json::from_str(&strict).unwrap();
    assert_eq!(strict["groups"][0]["members"].as_array().unwrap().len(), 2);
    let (_, loose, _) = command(&["sanitize", "--threshold", "80", "--json", folder_arg]);
    let loose: serde_json::Value = serde_json::from_str(&loose).unwrap();
    assert_eq!(loose["groups"][0]["members"].as_array().unwrap().len(), 3);
    assert_eq!(loose["settings"]["threshold"], 80);
}

#[test]
fn a_settings_file_is_read_and_a_bad_one_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    folder(dir.path());
    let cfg = dir.path().join("sanitizer.cfg");
    fs::write(&cfg, "threshold = 77\n").unwrap();
    let (code, out, _) = command(&[
        "sanitize",
        "--config",
        cfg.to_str().unwrap(),
        "--json",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("\"threshold\": 77"));
    fs::write(&cfg, "colour = red\n").unwrap();
    let (code, _, err) = command(&[
        "sanitize",
        "--config",
        cfg.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-006"), "{err}");
}

#[test]
fn unusable_arguments_and_folders_give_exit_code_2_with_a_code_in_the_chosen_language() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec!["sanitize"],
        vec!["sanitize", "--threshold", "101", "x"],
        vec!["sanitize", "--threshold", "many", "x"],
        vec!["sanitize", "--lang", "xx", "x"],
        vec!["sanitize", "--nope", "x"],
        vec!["sanitize", "--out"],
        vec!["sanitize", "a", "b"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SAN-012"), "{args:?}: {err}");
    }
    let missing = dir.path().join("missing");
    let (code, _, err) = command(&["sanitize", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-001"), "{err}");
    let (code, _, err) = command(&["sanitize", "--lang", "es", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(
        err.contains("No se puede leer la carpeta de bibliotecas"),
        "{err}"
    );
}
