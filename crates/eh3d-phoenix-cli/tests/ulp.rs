// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ulp` on programs written by the test itself and on the examples of `examples/ulp/`:
//! the reader lists the problems of a program with their places, the report gives the verdict and
//! the exit code follows it, `run` prints what the program prints and exits with 0, 2 or 1, and
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
        vec!["ulp", "walk", good.as_str()],
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

fn example(name: &str) -> String {
    format!("{}/../../examples/ulp/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn tutorial(name: &str) -> String {
    format!(
        "{}/../../examples/tutorials/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn expected(name: &str) -> String {
    fs::read_to_string(example(name))
        .unwrap()
        .replace("\r\n", "\n")
}

/// The examples: the program, the arguments that give it a design, and the file with its output.
fn examples() -> Vec<(&'static str, Vec<String>, &'static str)> {
    let board = |f: &str| vec!["--board".to_owned(), tutorial(f)];
    vec![
        ("math_table.ulp", vec![], "math_table.expected"),
        ("text_tools.ulp", vec![], "text_tools.expected"),
        ("parts_list.ulp", board("tut.brd"), "parts_list.expected"),
        ("netlist.ulp", board("tut.brd"), "netlist.expected"),
        ("board_stats.ulp", board("tut.brd"), "board_stats.expected"),
        (
            "bill_by_value.ulp",
            board("tut.brd"),
            "bill_by_value.expected",
        ),
        (
            "sheet_parts.ulp",
            vec!["--schematic".to_owned(), tutorial("tut.sch")],
            "sheet_parts.expected",
        ),
    ]
}

#[test]
fn every_example_gives_the_output_that_is_next_to_it() {
    for (program, extra, wanted) in examples() {
        let mut args = vec!["ulp".to_owned(), "run".to_owned()];
        args.extend(extra.clone());
        args.push(example(program));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (code, out, err) = command(&refs);
        assert_eq!(code, 0, "{program}: {err}");
        assert_eq!(out, expected(wanted), "{program}");
        // the same with the check: the output is the expected one and the exit code says so
        args.push("--expect".to_owned());
        args.push(example(wanted));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (code, _, err) = command(&refs);
        assert_eq!(code, 0, "{program} with --expect: {err}");
    }
}

#[test]
fn a_run_prints_the_same_bytes_every_time() {
    let board = tutorial("tut.brd");
    let program = example("netlist.ulp");
    let first = command(&["ulp", "run", "--board", &board, &program]);
    for _ in 0..3 {
        assert_eq!(command(&["ulp", "run", "--board", &board, &program]), first);
    }
}

#[test]
fn a_run_whose_output_is_not_the_expected_one_exits_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let program = example("math_table.ulp");
    let wrong = write(dir.path(), "wrong.expected", "other text\n");
    let (code, out, err) = command(&["ulp", "run", "--expect", &wrong, &program]);
    assert_eq!(code, 2, "{err}");
    assert!(out.contains("gcd(84, 36) = 12"), "{out}");
    assert!(err.contains("PHOENIX-ULP-041"), "{err}");
    // a file with the line ends of Windows is the same text
    let crlf = write(
        dir.path(),
        "crlf.expected",
        &expected("math_table.expected").replace('\n', "\r\n"),
    );
    let (code, _, err) = command(&["ulp", "run", "--expect", &crlf, &program]);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_program_that_ends_with_a_number_other_than_zero_exits_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let fails = write(
        dir.path(),
        "fails.ulp",
        "printf(\"checked\\n\");\nif (1 + 1 == 2) exit(3);\nprintf(\"not here\\n\");\n",
    );
    let (code, out, err) = command(&["ulp", "run", &fails]);
    assert_eq!(code, 2, "{err}");
    assert_eq!(out, "checked\n");
    assert!(err.contains("exit code 3"), "{err}");
    let fine = write(
        dir.path(),
        "fine.ulp",
        "printf(\"a\");\nexit(0);\nprintf(\"b\");\n",
    );
    let (code, out, _) = command(&["ulp", "run", &fine]);
    assert_eq!((code, out.as_str()), (0, "a"));
}

#[test]
fn a_program_that_cannot_run_exits_with_one_and_prints_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let divides = write(
        dir.path(),
        "div.ulp",
        "printf(\"before\\n\");\nint z = 0;\nint x = 1 / z;\n",
    );
    let broken = write(dir.path(), "broken.ulp", BROKEN);
    let missing = dir.path().join("missing.ulp");
    let missing = missing.to_str().unwrap();
    let board = tutorial("tut.brd");
    let good = write(dir.path(), "good.ulp", GOOD);
    let refused = example("refused_system.ulp");
    for (args, code_in_message) in [
        (vec!["ulp", "run", divides.as_str()], "PHOENIX-ULP-030 [3:"),
        (vec!["ulp", "run", broken.as_str()], "PHOENIX-ULP-039"),
        (
            vec!["ulp", "run", refused.as_str()],
            "PHOENIX-ULP-034 [4:1]",
        ),
        (vec!["ulp", "run", good.as_str()], "PHOENIX-ULP-036"),
        (
            vec!["ulp", "run", "--board", board.as_str(), good.as_str()],
            "PHOENIX-ULP-034",
        ),
        (vec!["ulp", "run", missing], "PHOENIX-ULP-003"),
        (
            vec!["ulp", "run", "--board", missing, good.as_str()],
            "PHOENIX-ULP-003",
        ),
        (
            vec!["ulp", "run", "--bogus", good.as_str()],
            "PHOENIX-ULP-002",
        ),
        (vec!["ulp", "run"], "PHOENIX-ULP-002"),
        (
            vec!["ulp", "run", good.as_str(), divides.as_str()],
            "PHOENIX-ULP-002",
        ),
    ] {
        let (code, out, err) = command(&args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(out.is_empty(), "{args:?} printed {out:?}");
        assert!(err.contains(code_in_message), "{args:?}: {err}");
    }
    assert!(
        !dir.path().join("count.txt").exists(),
        "the program opened a file"
    );
}

#[test]
fn the_limits_of_a_run_come_from_the_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    let program = example("math_table.ulp");
    for (line, code_in_message) in [
        ("max_steps = 20\n", "PHOENIX-ULP-025"),
        ("max_output_bytes = 30\n", "PHOENIX-ULP-028"),
        ("max_eval_depth = 3\n", "PHOENIX-ULP-027"),
    ] {
        let cfg = write(dir.path(), "ulp.cfg", line);
        let (code, out, err) = command(&["ulp", "run", "--config", &cfg, &program]);
        assert_eq!(code, 1, "{line}: {err}");
        assert!(out.is_empty(), "{line}: {out}");
        assert!(err.contains(code_in_message), "{line}: {err}");
    }
    let grows = write(
        dir.path(),
        "grows.ulp",
        "string s = \"xxxxxxxx\";\nwhile (1) s = s + s;\n",
    );
    let cfg = write(dir.path(), "small.cfg", "max_memory_bytes = 1024\n");
    let (code, out, err) = command(&["ulp", "run", "--config", &cfg, &grows]);
    assert_eq!(code, 1, "{err}");
    assert!(
        out.is_empty() && err.contains("PHOENIX-ULP-026"),
        "{out}{err}"
    );
    let cfg = write(dir.path(), "bad.cfg", "max_steps = 0\n");
    let (code, _, err) = command(&["ulp", "run", "--config", &cfg, &program]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-ULP-001"), "{err}");
}

#[test]
fn the_words_of_an_error_of_a_run_follow_the_language() {
    let dir = tempfile::tempdir().unwrap();
    let divides = write(dir.path(), "div.ulp", "int z = 0;\nint x = 1 / z;\n");
    let (code, _, err) = command(&["ulp", "run", "--lang", "pt", &divides]);
    assert_eq!(code, 1);
    assert!(err.contains("Divisão por zero"), "{err}");
    let (_, _, err) = command(&["ulp", "run", "--lang", "es", &divides]);
    assert!(err.contains("División por cero"), "{err}");
}

#[test]
fn the_report_says_which_programs_the_evaluator_runs() {
    for (program, _, _) in examples() {
        let (code, out, _) = command(&["ulp", "report", "--json", &example(program)]);
        assert_eq!(code, 0, "{program}: {out}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let p = &v["programs"][0];
        assert_eq!(p["verdict"], "covered", "{program}");
        assert_eq!(p["runnable"], true, "{program}");
        assert_eq!(p["evaluator"], "read_only");
        assert_eq!(p["sandbox"], "not_written");
        assert_eq!(p["ran"], false);
        assert_eq!(p["not_run"].as_array().unwrap().len(), 0, "{program}");
    }
    let (code, out, _) = command(&["ulp", "report", "--json", &example("refused_system.ulp")]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["programs"][0]["verdict"], "not_covered");
    assert_eq!(v["programs"][0]["runnable"], false);
    assert_eq!(v["programs"][0]["not_run"][0], "function system");
    let (_, text, _) = command(&[
        "ulp",
        "report",
        "--lang",
        "pt",
        &example("refused_system.ulp"),
    ]);
    assert!(text.contains("Executa-se no avaliador: não"), "{text}");
}

#[test]
fn a_run_writes_nothing_and_leaves_the_design_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let board = dir.path().join("tut.brd");
    fs::copy(tutorial("tut.brd"), &board).unwrap();
    let before = fs::read(&board).unwrap();
    let program = write(
        dir.path(),
        "tries.ulp",
        "board(B) { B.elements(E) { printf(\"%s\\n\", E.name); } }\noutput(\"made.txt\", \"wt\") { printf(\"x\"); }\n",
    );
    let files = |d: &Path| {
        let mut names: Vec<_> = fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let listed = files(dir.path());
    let (code, out, err) = command(&["ulp", "run", "--board", board.to_str().unwrap(), &program]);
    assert_eq!(code, 1, "{err}");
    assert!(
        out.is_empty() && err.contains("PHOENIX-ULP-034"),
        "{out}{err}"
    );
    assert_eq!(files(dir.path()), listed);
    assert_eq!(fs::read(&board).unwrap(), before);
}

#[test]
fn the_help_of_the_command_names_run() {
    let (code, out, _) = command(&["help", "ulp"]);
    assert_eq!(code, 0);
    assert!(out.contains("run"), "{out}");
    let (_, _, err) = command(&["ulp"]);
    assert!(err.contains("phoenix ulp run"), "{err}");
}
