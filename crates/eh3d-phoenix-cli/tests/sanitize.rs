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

fn apply_folder(dir: &Path) -> String {
    let src = dir.join("libs");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("a.lbr"), library(&[footprint("R0805", "-0.9")])).unwrap();
    fs::write(src.join("b.lbr"), library(&[footprint("RES", "-0.9")])).unwrap();
    fs::write(src.join("d.lbr"), library(&[footprint("CLOSE", "-0.905")])).unwrap();
    src.to_str().unwrap().to_owned()
}

#[test]
fn apply_is_a_dry_run_until_it_has_the_code_and_writes_a_copy_in_a_new_folder_only() {
    let dir = tempfile::tempdir().unwrap();
    let src = apply_folder(dir.path());
    let (code, out, err) = command(&["sanitize", "--apply", &src]);
    assert_eq!(code, 0, "{err}");
    let line = out
        .lines()
        .find(|l| l.starts_with("confirmation code: "))
        .unwrap();
    let confirm = line.trim_start_matches("confirmation code: ").to_owned();
    assert!(
        out.contains("applied P001 package b.lbr:RES <- a.lbr:R0805"),
        "{out}"
    );
    assert!(
        out.contains("left out P001 package d.lbr:CLOSE (not-exact)"),
        "{out}"
    );
    assert!(out.contains("dry run: nothing was written"), "{out}");
    // the same plan gives the same code
    let (_, again, _) = command(&["sanitize", "--apply", &src]);
    assert!(again.contains(&format!("confirmation code: {confirm}\n")));
    let (_, json, _) = command(&["sanitize", "--apply", "--json", &src]);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["confirmation_code"], confirm.as_str());
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["skipped"][0]["reason"], "not-exact");

    // a wrong code writes nothing
    let target = dir.path().join("copy");
    let target_s = target.to_str().unwrap();
    let (code, _, err) = command(&[
        "sanitize",
        "--apply",
        "--confirm",
        "0000-0000-0000",
        "--out",
        target_s,
        &src,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-014"), "{err}");
    assert!(!target.exists());

    // the right code writes the copy; the close footprint waits for permission
    let (code, out, err) = command(&[
        "sanitize",
        "--apply",
        "--confirm",
        &confirm,
        "--out",
        target_s,
        &src,
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(err.contains("note: wrote 3 libraries to"), "{err}");
    for name in ["a.lbr", "b.lbr", "d.lbr"] {
        assert!(target.join(name).is_file(), "{name}");
    }
    assert!(fs::read_to_string(target.join("d.lbr"))
        .unwrap()
        .contains("x=\"-0.905\""));
    // a folder that exists is never written over
    let (code, _, err) = command(&[
        "sanitize",
        "--apply",
        "--confirm",
        &confirm,
        "--out",
        target_s,
        &src,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-008"), "{err}");

    // with the permission the close footprint is drawn as the master, under its own name
    let near = dir.path().join("near");
    let (code, out, err) = command(&[
        "sanitize",
        "--apply",
        "--allow-near",
        "--confirm",
        &confirm,
        "--out",
        near.to_str().unwrap(),
        &src,
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let drawn = fs::read_to_string(near.join("d.lbr")).unwrap();
    assert!(
        drawn.contains("<package name=\"CLOSE\">") && drawn.contains("x=\"-0.9\""),
        "{drawn}"
    );
    assert!(!drawn.contains("-0.905"));

    // the libraries that were read are the ones they were
    let original = fs::read_to_string(Path::new(&src).join("d.lbr")).unwrap();
    assert!(original.contains("x=\"-0.905\""));
    assert_eq!(names_of(Path::new(&src)), vec!["a.lbr", "b.lbr", "d.lbr"]);
}

fn names_of(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn apply_refuses_what_it_cannot_use_and_a_group_can_be_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let src = apply_folder(dir.path());
    let out_dir = dir.path().join("x");
    let out_s = out_dir.to_str().unwrap();
    for args in [
        vec!["sanitize", "--confirm", "A", &src],
        vec!["sanitize", "--group", "P001", &src],
        vec!["sanitize", "--allow-near", &src],
        vec!["sanitize", "--apply", "--out", out_s, &src],
        vec!["sanitize", "--apply", "--confirm", "A", &src],
        vec!["sanitize", "--apply", "--group"],
        vec!["sanitize", "--apply", "--confirm"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SAN-012"), "{args:?}: {err}");
    }
    assert!(!out_dir.exists());
    // a group that the plan does not have
    let (code, _, err) = command(&["sanitize", "--apply", "--group", "Z999", &src]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-015"), "{err}");
    // the group that it has: the others are none
    let (code, out, _) = command(&["sanitize", "--apply", "--group", "P001", &src]);
    assert_eq!(code, 0);
    assert!(out.contains("plan steps: 1 applied, 1 left out"), "{out}");
    // the copy cannot go inside the folder that was read
    let (_, dry, _) = command(&["sanitize", "--apply", &src]);
    let confirm = dry
        .lines()
        .find_map(|l| l.strip_prefix("confirmation code: "))
        .unwrap()
        .to_owned();
    let inside = Path::new(&src).join("copy");
    let (code, _, err) = command(&[
        "sanitize",
        "--apply",
        "--confirm",
        &confirm,
        "--out",
        inside.to_str().unwrap(),
        &src,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SAN-008"), "{err}");
    assert!(!inside.exists());
}
