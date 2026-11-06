// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `corpus` on the own corpus of the repository and on folders written by the test.

use std::fs;
use std::path::PathBuf;

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

fn corpus_dir() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .to_str()
        .unwrap()
        .to_owned()
}

const GOOD: &str = r#"<?xml version="1.0"?><eagle version="9.6.2"><drawing><layers><layer number="1" name="Top" color="1" fill="1" visible="yes" active="yes"/></layers><board/></drawing></eagle>"#;

const EXPECT: &str = r#"{"format":"phoenix-corpus/1","id":"CORPUS-CLI-001","title":"one layer",
    "kind":"eagle","input":"a.brd","how":["one layer line"],
    "expect":{"outcome":"read","measures":{"layers":WANTED}}}"#;

fn folder(wanted: u32) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.brd"), GOOD).unwrap();
    fs::write(
        dir.path().join("a.brd.expect.json"),
        EXPECT.replace("WANTED", &wanted.to_string()),
    )
    .unwrap();
    dir
}

#[test]
fn the_corpus_of_the_repository_runs_and_every_piece_is_as_expected() {
    let (code, out, err) = command(&["corpus", "run", "--dir", &corpus_dir()]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("ok: CORPUS-EAGLE-001 (eagle)"), "{out}");
    assert!(
        out.contains("expected error: CORPUS-EAGLE-006 (eagle)"),
        "{out}"
    );
    assert!(out.contains("difference 0 | failed 0"), "{out}");
    // 112 expectations: the 71 that must be read are ok and the 41 that must be refused are
    // expected errors (docs/CORPUS.md gives the same numbers)
    assert!(
        out.contains("corpus: 112 | ok 71 | expected error 41 | difference 0 | failed 0"),
        "{out}"
    );
    assert!(err.is_empty(), "{err}");
}

#[test]
fn the_list_has_as_many_lines_as_the_corpus_has_expectations() {
    let (code, all, _) = command(&["corpus", "list", "--dir", &corpus_dir()]);
    assert_eq!(code, 0);
    assert_eq!(all.lines().count(), 112, "{all}");
    assert_eq!(
        all.lines().filter(|l| l.contains(" | refused | ")).count(),
        41
    );
    assert_eq!(all.lines().filter(|l| l.contains(" | read | ")).count(), 71);
}

#[test]
fn the_list_has_one_line_per_piece_and_a_filter_keeps_some() {
    let (code, all, _) = command(&["corpus", "list", "--dir", &corpus_dir()]);
    assert_eq!(code, 0);
    assert!(
        all.starts_with("CORPUS-EAGLE-001 | eagle | read | "),
        "{all}"
    );
    let (_, trips, _) = command(&["corpus", "list", "--dir", &corpus_dir(), "--filter", "TRIP"]);
    assert_eq!(trips.lines().count(), 8, "{trips}");
    assert!(trips.lines().all(|l| l.contains(" | roundtrip | ")));
}

#[test]
fn the_check_finds_no_problem_in_the_corpus_and_finds_them_in_a_bad_folder() {
    let (code, out, _) = command(&["corpus", "check", "--dir", &corpus_dir()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.ends_with("0 problems\n"), "{out}");
    let dir = folder(1);
    fs::write(dir.path().join("orphan.brd"), GOOD).unwrap();
    let path = dir.path().to_str().unwrap();
    let (code, out, _) = command(&["corpus", "check", "--dir", path, "--lang", "es"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("PHOENIX-CORPUS-003: El corpus no es coherente: orphan.brd"),
        "{out}"
    );
}

#[test]
fn a_difference_gives_exit_one_and_the_words_follow_the_language() {
    let dir = folder(9);
    let path = dir.path().to_str().unwrap();
    let (code, out, _) = command(&["corpus", "run", "--dir", path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("difference: CORPUS-CLI-001"), "{out}");
    assert!(out.contains("layers: expected 9, got 1"), "{out}");
    let (_, pt, _) = command(&["corpus", "run", "--dir", path, "--lang", "pt"]);
    assert!(pt.contains("diferença: CORPUS-CLI-001"), "{pt}");
    let good = folder(1);
    let (code, out, _) = command(&["corpus", "run", "--dir", good.path().to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_json_report_is_written_to_a_new_file_and_never_over_one() {
    let dir = folder(1);
    let path = dir.path().to_str().unwrap();
    let out_file = dir.path().join("report.json");
    let out_path = out_file.to_str().unwrap();
    let (code, _, _) = command(&["corpus", "run", "--dir", path, "--out", out_path]);
    assert_eq!(code, 0);
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_file).unwrap()).unwrap();
    assert_eq!(json["format"], "phoenix-corpus-report/1");
    assert_eq!(json["counts"]["ok"], 1);
    let (code, _, err) = command(&["corpus", "run", "--dir", path, "--out", out_path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-CORPUS-010"), "{err}");
    let (_, json_text, _) = command(&["corpus", "run", "--dir", path, "--json"]);
    assert!(json_text.starts_with("{\n"), "{json_text}");
}

#[test]
fn the_properties_are_tried_with_a_settings_file() {
    let dir = folder(1);
    let path = dir.path().to_str().unwrap();
    let settings = dir.path().join("corpus.cfg");
    fs::write(&settings, "property_seeds = 3\nproperty_first_seed = 5\n").unwrap();
    let (code, out, err) = command(&[
        "corpus",
        "run",
        "--dir",
        path,
        "--properties",
        "--settings",
        settings.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("ok: PROPERTY-UNDO (property)"), "{out}");
    assert!(out.contains("3 seeds from 5"), "{out}");
}

#[test]
fn bad_arguments_and_folders_give_exit_two() {
    let (code, _, err) = command(&["corpus"]);
    assert_eq!(code, 2);
    assert!(err.contains("phoenix corpus run"), "{err}");
    let (code, _, err) = command(&["corpus", "run", "--colour", "red"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-CORPUS-009"), "{err}");
    let (code, _, err) = command(&["corpus", "run", "--dir", "no-such-folder-here"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-CORPUS-001"), "{err}");
    let (code, _, err) = command(&["corpus", "run", "--settings", "no-such-file.cfg"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-CORPUS-011"), "{err}");
    let (code, _, err) = command(&["corpus", "run", "--lang", "xx"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-CORPUS-009"), "{err}");
}
