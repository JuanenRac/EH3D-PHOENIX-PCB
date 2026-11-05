// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `oracle` on the reference cases built into the program and on a folder of cases
//! written by the test itself.

use std::fs;

use eh3d_phoenix_cli::run;

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

const WRONG_CASE: &str = r#"{"format":"phoenix-oracle/1","suite":"mine","title":"t","cases":[
    {"id":"ORACLE-MINE-001","title":"light","area":"stackup","check":"const_c0",
     "expected":300000000.0,"unit":"m/s","tolerance":{"abs":1.0},
     "tolerance_note":"test","source":"none","status":"cited"}]}"#;

#[test]
fn the_built_in_cases_run_and_the_pending_ones_are_listed() {
    let (code, out, err) = command(&["oracle", "run"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("ok: ORACLE-STACKUP-001"), "{out}");
    assert!(out.contains("pending: ORACLE-PENDING-001"), "{out}");
    assert!(err.is_empty(), "{err}");
}

#[test]
fn the_json_report_and_the_language_of_the_words() {
    let (code, out, _) = command(&["oracle", "run", "--json", "--suite", "rf"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["format"], "phoenix-oracle-report/1");
    assert_eq!(json["counts"]["outside"], 0);
    let (_, es, _) = command(&["oracle", "run", "--suite", "sim", "--lang", "es"]);
    assert!(es.contains("correcto: ORACLE-SIM-001"), "{es}");
    let (_, pt, _) = command(&["oracle", "run", "--suite", "pending", "--lang", "pt"]);
    assert!(pt.contains("pendente: ORACLE-PENDING-001"), "{pt}");
}

#[test]
fn the_list_has_one_line_per_case() {
    let (code, out, _) = command(&["oracle", "list", "--suite", "stackup"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("ORACLE-STACKUP-001 | cited | stackup | "));
    assert_eq!(out.lines().count(), 19, "{out}");
}

#[test]
fn a_case_outside_its_tolerance_gives_exit_one_and_a_folder_can_be_chosen() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("mine.json"), WRONG_CASE).unwrap();
    let folder = dir.path().to_str().unwrap();
    let (code, out, _) = command(&["oracle", "run", "--dir", folder]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("outside tolerance: ORACLE-MINE-001"), "{out}");
    assert!(out.contains("PHOENIX-ORACLE-006"), "{out}");
}

#[test]
fn the_report_file_is_new_and_an_existing_file_is_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("report.json");
    let path = target.to_str().unwrap();
    let (code, out, _) = command(&["oracle", "run", "--suite", "rf", "--out", path]);
    assert_eq!(code, 0, "{out}");
    let written = fs::read_to_string(&target).unwrap();
    assert!(written.contains("ORACLE-RF-001"));
    let (code, _, err) = command(&["oracle", "run", "--suite", "rf", "--out", path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ORACLE-010"), "{err}");
    assert_eq!(fs::read_to_string(&target).unwrap(), written);
}

#[test]
fn bad_arguments_and_unreadable_folders_give_exit_two() {
    assert_eq!(command(&["oracle"]).0, 2);
    assert_eq!(command(&["oracle", "fly"]).0, 2);
    let (code, _, err) = command(&["oracle", "run", "--colour"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ORACLE-009"), "{err}");
    let (code, _, err) = command(&["oracle", "run", "--dir", "no-such-folder-of-cases"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ORACLE-008"), "{err}");
    let (code, _, err) = command(&["oracle", "run", "--lang", "xx"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ORACLE-009"), "{err}");
}
