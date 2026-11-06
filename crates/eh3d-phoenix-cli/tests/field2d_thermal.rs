// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `field2d` and `thermal`, and the option `--field2d` of `stackup`, run through
//! `run` on a job file written for the test and on a board of the corpus.

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::run;
use serde_json::Value;

fn call(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

/// A board with the net A: a track of 0.25 mm on the layer Top and vias with a drill of 0.3 mm.
fn board() -> String {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/model/mini_pair.board.json");
    root.to_str().unwrap().to_owned()
}

const JOB: &str = r#"{
  "stackup": {"name": "two", "layers": [
    {"kind": "copper", "name": "Top", "thickness": {"um": 35.0}},
    {"kind": "core", "name": "core", "thickness_nm": 200000, "material": "fr4"},
    {"kind": "copper", "name": "Bottom", "thickness": {"um": 35.0}}]},
  "materials": [{"name": "fr4", "dk_points": [[0.1, 4.3], [50.0, 4.3]],
                 "df_points": [[0.1, 0.02], [50.0, 0.02]]}],
  "lines": [
    {"name": "a", "kind": "microstrip", "layer": "Top", "width_nm": 300000}]
}"#;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn text_of(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn the_field_solver_solves_the_lines_of_a_job_and_prints_the_comparison() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(dir.path(), "job.json", JOB);
    let (code, text, err) = call(&["field2d", text_of(&job)]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.contains("line a (microstrip)"), "{text}");
    assert!(text.contains("closed formula"), "{text}");
    let (code, json, _) = call(&["field2d", text_of(&job), "--json"]);
    assert_eq!(code, 0);
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], "phoenix-field2d/1");
    assert_eq!(value["state"], "prototype");
    let z0 = value["lines"][0]["result"]["field"]["z0_ohm"]
        .as_f64()
        .unwrap();
    assert!(z0 > 52.0 && z0 < 57.0, "{z0}");
    let difference = value["lines"][0]["result"]["comparison"]["impedance_difference_percent"]
        .as_f64()
        .unwrap();
    assert!(difference.abs() < 3.0, "{difference}");
    // the same input gives the same bytes
    let (_, again, _) = call(&["field2d", text_of(&job), "--json"]);
    assert_eq!(json, again);
}

#[test]
fn the_field_solver_takes_the_line_of_a_net_from_a_board_and_a_stackup() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(dir.path(), "job.json", JOB);
    let (code, text, err) = call(&[
        "field2d",
        &board(),
        "--stackup",
        text_of(&job),
        "--net",
        "A",
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.contains("line A (microstrip)"), "{text}");
    assert!(text.contains("width: 0.250 mm"), "{text}");
    let (code, _, err) = call(&[
        "field2d",
        &board(),
        "--stackup",
        text_of(&job),
        "--net",
        "NOPE",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-FIELD-010"), "{err}");
}

#[test]
fn stackup_can_add_the_field_solver_to_its_report() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(dir.path(), "job.json", JOB);
    let (code, text, err) = call(&["stackup", text_of(&job), "--freq-ghz", "1", "--field2d"]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(
        text.contains("the field solver (phoenix field2d) on the same lines"),
        "{text}"
    );
    assert!(text.contains("closed formula"), "{text}");
    let (code, json, _) = call(&[
        "stackup",
        text_of(&job),
        "--freq-ghz",
        "1",
        "--field2d",
        "--json",
    ]);
    assert_eq!(code, 0);
    let value: Value = serde_json::from_str(&json).unwrap();
    assert!(value.get("field2d").is_some(), "{json}");
    // without the option the report is the one it was
    let (_, plain, _) = call(&["stackup", text_of(&job), "--freq-ghz", "1"]);
    assert!(!plain.contains("field solver"));
}

#[test]
fn the_field_solver_refuses_what_it_cannot_use_and_never_replaces_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(dir.path(), "job.json", JOB);
    assert_eq!(call(&["field2d"]).0, 2);
    assert_eq!(call(&["field2d", &board()]).0, 2);
    assert_eq!(call(&["field2d", text_of(&job), "--net", "A"]).0, 2);
    assert_eq!(call(&["field2d", text_of(&job), "--refine", "fine"]).0, 2);
    assert_eq!(call(&["field2d", text_of(&job), "--lang", "xx"]).0, 2);
    assert_eq!(call(&["field2d", "no-such-file.json"]).0, 2);
    let existing = write(dir.path(), "out.json", "kept");
    let (code, _, err) = call(&["field2d", text_of(&job), "--out", text_of(&existing)]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-FIELD-007"), "{err}");
    assert_eq!(fs::read_to_string(&existing).unwrap(), "kept");
    let target = dir.path().join("new.json");
    let (code, text, _) = call(&["field2d", text_of(&job), "--out", text_of(&target)]);
    assert_eq!(code, 0);
    assert!(text.contains("written:"), "{text}");
    let written: Value = serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(written["format"], "phoenix-field2d/1");
}

#[test]
fn a_current_that_heats_a_track_too_much_gives_a_warning_and_the_exit_code_1() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(dir.path(), "job.json", JOB);
    let low = write(dir.path(), "low.cfg", "# currents\nA = 0.5\n");
    let high = write(dir.path(), "high.cfg", "A = 3\n");
    let (code, text, err) = call(&[
        "thermal",
        "--currents",
        text_of(&low),
        "--stackup",
        text_of(&job),
        &board(),
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.contains("net A 0.5 A"), "{text}");
    assert!(
        text.contains("track on Top 0.250 mm by 35.0 um, external"),
        "{text}"
    );
    assert!(text.contains("PHOENIX-HEAT-012"), "{text}");
    assert!(!text.contains("above the limit"), "{text}");
    let (code, text, _) = call(&[
        "thermal",
        "--currents",
        text_of(&high),
        "--stackup",
        text_of(&job),
        &board(),
    ]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("[above the limit]"), "{text}");
    assert!(text.contains("PHOENIX-HEAT-004"), "{text}");
    // the vias of the net are looked at too
    assert!(text.contains("via(s) of 0.300 mm drill"), "{text}");
}

#[test]
fn without_a_stackup_the_defaults_are_used_and_the_report_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let low = write(dir.path(), "low.cfg", "A = 0.5\nNOPE = 1\n");
    let (code, text, _) = call(&["thermal", "--currents", text_of(&low), &board()]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("PHOENIX-HEAT-008"), "{text}");
    assert!(text.contains("PHOENIX-HEAT-007"), "{text}");
    assert!(text.contains("internal"), "{text}");
    let (code, json, _) = call(&["thermal", "--currents", text_of(&low), "--json", &board()]);
    assert_eq!(code, 0);
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], "phoenix-thermal/1");
    let rise = value["nets"][0]["tracks"][0]["heating"]["ipc2221_rise_k"]
        .as_f64()
        .unwrap();
    assert!(rise > 0.0 && rise < 20.0, "{rise}");
}

#[test]
fn the_thermal_command_refuses_what_it_cannot_use_and_never_replaces_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let ok = write(dir.path(), "ok.cfg", "A = 1\n");
    let twice = write(dir.path(), "twice.cfg", "A = 1\nA = 2\n");
    assert_eq!(call(&["thermal"]).0, 2);
    assert_eq!(call(&["thermal", &board()]).0, 2);
    assert_eq!(
        call(&["thermal", "--currents", text_of(&twice), &board()]).0,
        2
    );
    assert_eq!(
        call(&["thermal", "--currents", "no-such.cfg", &board()]).0,
        2
    );
    assert_eq!(
        call(&[
            "thermal",
            "--currents",
            text_of(&ok),
            "--net",
            "B",
            &board()
        ])
        .0,
        2
    );
    let existing = write(dir.path(), "out.json", "kept");
    let (code, _, err) = call(&[
        "thermal",
        "--currents",
        text_of(&ok),
        "--out",
        text_of(&existing),
        &board(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-HEAT-010"), "{err}");
    assert_eq!(fs::read_to_string(&existing).unwrap(), "kept");
}
