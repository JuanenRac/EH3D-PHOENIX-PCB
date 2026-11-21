// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ulp` on programs written by the test itself: the reader lists the problems of a
//! program with their places, the report gives the verdict and the exit code follows it, and
//! nothing is written.

use std::fs;
use std::path::Path;

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

const GOOD: &str = "#usage \"Counts the parts\"\nint n = 0;\nboard(B) {\n  B.elements(E) { n++; }\n}\noutput(\"count.txt\", \"wt\") {\n  printf(\"%d\\n\", n);\n}\n";
const BROKEN: &str = "int a = ;\nint b = 2\nstring s = \"open;\n";
const SYSTEM: &str = "system(\"dir\");\n";
const DIALOG: &str = "int ok = dlgDialog(\"Title\") { dlgLabel(\"x\"); };\n";

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn check_reads_a_good_program_without_a_problem() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let (code, out, err) = command(&["ulp", "check", &good]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("0 problems"), "{out}");
    assert!(err.is_empty());
}

#[test]
fn check_lists_the_problems_with_their_places_and_exits_with_one() {
    let dir = tempfile::tempdir().unwrap();
    let broken = write(dir.path(), "broken.ulp", BROKEN);
    let (code, out, _) = command(&["ulp", "check", &broken]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-ULP-015 [1:9]"), "{out}");
    assert!(out.contains("PHOENIX-ULP-008 [3:12]"), "{out}");
    assert!(out.contains("problems"), "{out}");
}

#[test]
fn check_in_json_has_the_places_and_is_the_same_every_time() {
    let dir = tempfile::tempdir().unwrap();
    let broken = write(dir.path(), "broken.ulp", BROKEN);
    let (code, out, _) = command(&["ulp", "check", "--json", &broken]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["format"], "phoenix-ulp-check/1");
    let first = &v["programs"][0]["problems"][0];
    assert_eq!(first["code"], "PHOENIX-ULP-015");
    assert_eq!(
        (first["line"].as_u64(), first["col"].as_u64()),
        (Some(1), Some(9))
    );
    assert_eq!(command(&["ulp", "check", "--json", &broken]).1, out);
}

#[test]
fn report_says_covered_for_a_program_the_list_covers() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let (code, out, err) = command(&["ulp", "report", &good]);
    assert_eq!(code, 0, "{err}\n{out}");
    assert!(out.contains("covered"), "{out}");
    assert!(out.contains("Nothing was run"), "{out}");
    assert!(out.contains("SHA-256"), "{out}");
}

#[test]
fn report_says_not_covered_for_a_command_of_the_system_and_partly_for_a_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let system = write(dir.path(), "system.ulp", SYSTEM);
    let dialog = write(dir.path(), "dialog.ulp", DIALOG);
    let (code, out, _) = command(&["ulp", "report", &system]);
    assert_eq!(code, 1);
    assert!(
        out.contains("not covered") && out.contains("PHOENIX-ULP-020"),
        "{out}"
    );
    let (code, out, _) = command(&["ulp", "report", &dialog]);
    assert_eq!(code, 1);
    assert!(
        out.contains("partly covered") && out.contains("PHOENIX-ULP-021"),
        "{out}"
    );
}

#[test]
fn report_in_json_holds_every_program_and_the_verdict_of_each() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let system = write(dir.path(), "system.ulp", SYSTEM);
    let (code, out, _) = command(&["ulp", "report", "--json", &good, &system]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["format"], "phoenix-ulp-reports/1");
    assert_eq!(v["programs"][0]["verdict"], "covered");
    assert_eq!(v["programs"][1]["verdict"], "not_covered");
    assert_eq!(v["programs"][1]["ran"], false);
}

#[test]
fn the_words_follow_the_language() {
    let dir = tempfile::tempdir().unwrap();
    let broken = write(dir.path(), "broken.ulp", BROKEN);
    let (_, out, _) = command(&["ulp", "check", "--lang", "pt", &broken]);
    assert!(out.contains("Falta um elemento necessário"), "{out}");
    let good = write(dir.path(), "good.ulp", GOOD);
    let (_, out, _) = command(&["ulp", "report", "--lang", "de", &good]);
    assert!(out.contains("Kompatibilitätsbericht"), "{out}");
}

#[test]
fn a_list_of_names_given_with_support_changes_the_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let strict = write(dir.path(), "strict.cfg", "function printf denied\n");
    let (code, out, _) = command(&["ulp", "report", "--support", &strict, &good]);
    assert_eq!(code, 1);
    assert!(out.contains("not covered"), "{out}");
}

#[test]
fn a_config_file_changes_the_limits() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let cfg = write(dir.path(), "ulp.cfg", "max_tokens = 5\n");
    let (code, out, _) = command(&["ulp", "check", "--config", &cfg, &good]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-ULP-005"), "{out}");
    let (code, out, _) = command(&["ulp", "report", "--config", &cfg, &good]);
    assert_eq!(code, 1);
    assert!(
        out.contains("not covered") && out.contains("PHOENIX-ULP-005"),
        "{out}"
    );
}

#[test]
fn nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let before: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    command(&["ulp", "check", &good]);
    command(&["ulp", "report", "--json", &good]);
    let after: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(before.len(), after.len());
    assert!(
        !dir.path().join("count.txt").exists(),
        "the program was run"
    );
}

#[test]
fn a_mistake_in_the_arguments_or_the_files_exits_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "good.ulp", GOOD);
    let bad_cfg = write(dir.path(), "bad.cfg", "max_depth = lots\n");
    let bad_list = write(dir.path(), "bad_list.cfg", "verb name allowed\n");
    let missing = dir.path().join("missing.ulp");
    let missing = missing.to_str().unwrap();
    let (code, _, err) = command(&["ulp"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("phoenix ulp "), "{err}");
    for args in [
        vec!["ulp", "run", good.as_str()],
        vec!["ulp", "check"],
        vec!["ulp", "check", "--bogus", good.as_str()],
        vec!["ulp", "check", "--lang", "xx", good.as_str()],
        vec!["ulp", "check", "--config"],
        vec!["ulp", "check", "--config", bad_cfg.as_str(), good.as_str()],
        vec![
            "ulp",
            "report",
            "--support",
            bad_list.as_str(),
            good.as_str(),
        ],
        vec!["ulp", "report", "--support"],
        vec!["ulp", "check", missing],
    ] {
        let (code, out, err) = command(&args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(err.contains("error: PHOENIX-ULP-"), "{args:?}: {err}");
    }
}

#[test]
fn a_program_that_is_too_large_is_refused_before_it_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), "ulp.cfg", "max_source_bytes = 20\n");
    let big = write(dir.path(), "big.ulp", GOOD);
    let (code, _, err) = command(&["ulp", "check", "--config", &cfg, &big]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ULP-004"), "{err}");
}

#[test]
fn the_command_is_in_the_help_in_every_language() {
    for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let (code, out, _) = command(&["help", "ulp", "--lang", lang]);
        assert_eq!(code, 0, "{lang}");
        assert!(out.contains("ulp"), "{lang}: {out}");
    }
}
