// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `rfgen` on a stack-up written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::Board;
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

const STACKUP: &str = r#"{
  "stackup": {"name": "two-layer", "layers": [
    {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
    {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
    {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
  "materials": [
    {"name": "fr4", "dk_points": [[0.001, 4.4], [200.0, 4.4]], "df_points": [[0.001, 0.02], [200.0, 0.02]]}
  ]
}"#;

fn stackup_file(dir: &Path) -> String {
    let path = dir.join("stackup.json");
    fs::write(&path, STACKUP).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn the_list_names_every_type_with_its_parameters() {
    let (code, out, _) = command(&["rfgen", "--list"]);
    assert_eq!(code, 0);
    for name in [
        "line",
        "bend",
        "curve",
        "taper",
        "stub-open",
        "stub-short",
        "stub-match",
        "quarter-wave",
        "l-match",
        "lowpass",
        "bandpass",
        "wilkinson",
        "branch-line",
        "patch",
        "dipole",
        "sma",
    ] {
        assert!(out.contains(name), "{name}");
    }
    assert!(out.contains("--bandwidth_percent"));
    let (code, json, _) = command(&["rfgen", "--list", "--json"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["kinds"].as_array().unwrap().len(), 16);
}

#[test]
fn a_line_is_reported_and_nothing_is_written_without_out() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let (code, out, err) = command(&["rfgen", "line", "--stackup", &stackup, "--freq-ghz", "1"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("structure: line"), "{out}");
    assert!(out.contains("line width ="), "{out}");
    assert!(err.contains("dry run"), "{err}");
    assert!(out.contains("Prototype"), "{out}");
    let (code, out, _) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--json",
    ]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["state"], "prototype");
    assert_eq!(value["rules"]["passes"], true);
}

#[test]
fn the_board_is_written_to_a_new_json_file_and_never_over_an_existing_one() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let target = dir.path().join("board.json");
    let target = target.to_str().unwrap();
    let args = [
        "rfgen",
        "bend",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "2",
        "--turn",
        "right",
        "--out",
        target,
    ];
    let (code, _, err) = command(&args);
    assert_eq!(code, 0, "{err}");
    let board = Board::from_bytes(&fs::read(target).unwrap()).unwrap();
    assert!(board.is_consistent(), "{:?}", board.validate());
    assert_eq!(board.zones.len(), 1);
    assert_eq!(board.layers.len(), 2);
    let before = fs::read(target).unwrap();
    let (code, _, err) = command(&args);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-014"), "{err}");
    assert_eq!(fs::read(target).unwrap(), before);
}

#[test]
fn a_second_structure_is_added_to_the_board_of_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let first = dir.path().join("first.json");
    let second = dir.path().join("second.json");
    let (first, second) = (first.to_str().unwrap(), second.to_str().unwrap());
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--out",
        first,
    ]);
    assert_eq!(code, 0, "{err}");
    let (code, _, err) = command(&[
        "rfgen",
        "stub-open",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--board",
        first,
        "--y-mm",
        "20",
        "--rotation",
        "90",
        "--out",
        second,
    ]);
    assert_eq!(code, 0, "{err}");
    let board = Board::from_bytes(&fs::read(second).unwrap()).unwrap();
    assert_eq!(board.zones.len(), 3);
    assert_eq!(board.nets.len(), 1);
    // the input is untouched
    let original = Board::from_bytes(&fs::read(first).unwrap()).unwrap();
    assert_eq!(original.zones.len(), 1);
}

#[test]
fn the_board_can_be_written_as_an_eagle_file() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let target = dir.path().join("board.brd");
    let target = target.to_str().unwrap();
    let (code, _, err) = command(&[
        "rfgen",
        "stub-short",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--out",
        target,
    ]);
    assert_eq!(code, 0, "{err}");
    let text = fs::read_to_string(target).unwrap();
    assert!(text.starts_with("<?xml"), "{}", &text[..40.min(text.len())]);
    assert!(text.contains("<eagle"));
}

#[test]
fn the_parameters_of_the_structure_are_options_and_a_wrong_one_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let (code, out, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--length-mm",
        "12.5",
        "--z0",
        "75",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("length = 12.500000 mm"), "{out}");
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--speed",
        "3",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-002"), "{err}");
    // the messages follow the language
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--speed",
        "3",
        "--lang",
        "es",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("Falta un parámetro"), "{err}");
}

#[test]
fn bad_arguments_are_refused_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let stackup = stackup_file(dir.path());
    let (code, _, err) = command(&["rfgen"]);
    assert_eq!(code, 2);
    assert!(err.contains("phoenix rfgen"), "{err}");
    let (code, _, err) = command(&[
        "rfgen",
        "amplifier",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-003"), "{err}");
    let (code, _, _) = command(&["rfgen", "line", "--stackup", &stackup]);
    assert_eq!(code, 2);
    let (code, _, err) = command(&["rfgen", "line", "--stackup", &stackup, "--freq-ghz", "fast"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-013"), "{err}");
    let target = dir.path().join("board.txt");
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--out",
        target.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-013"), "{err}");
    assert!(!target.exists());
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        "/no/such/file.json",
        "--freq-ghz",
        "1",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-015"), "{err}");
    let (code, _, err) = command(&[
        "rfgen",
        "line",
        "--stackup",
        &stackup,
        "--freq-ghz",
        "1",
        "--rotation",
        "45",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RFGEN-002"), "{err}");
}
