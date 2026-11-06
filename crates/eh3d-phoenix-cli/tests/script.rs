// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `script run`, `script report` and `cam-import` on files written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_export::ProfileSet;
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_schematic::Schematic;

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

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

/// A board with two copper layers, a silkscreen layer and the net GND.
fn board() -> Board {
    let mut b = Board::default();
    for (n, name, kind) in [
        (1_u32, "Top", LayerKind::Copper),
        (2, "Bottom", LayerKind::Copper),
        (3, "tPlace", LayerKind::Silkscreen),
    ] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind,
                side: (n == 3).then_some(Side::Top),
            },
        );
    }
    b.nets.insert(id(10), Net { name: "GND".into() });
    b
}

fn write_board(dir: &Path) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_script(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

const WIRE: &str = "LAYER Top\nWIRE 'GND' 0.3 (1 1) (5 1)\n";

#[test]
fn a_run_is_a_dry_run_by_default_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let before = fs::read(&board_path).unwrap();
    let (code, out, err) = command(&["script", "run", &script, &board_path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("result: carried out"), "{out}");
    assert!(out.contains("operations: 1"), "{out}");
    assert!(out.contains("dry run: nothing was written"), "{out}");
    assert!(err.contains("dry run"), "{err}");
    assert_eq!(fs::read(&board_path).unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn out_writes_the_new_board_once_and_never_replaces_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let before = fs::read(&board_path).unwrap();
    let target = dir.path().join("result.json");
    let target_text = target.to_str().unwrap();
    let (code, out, err) = command(&["script", "run", "--out", target_text, &script, &board_path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let result = Board::from_bytes(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(result.tracks.len(), 1);
    assert_eq!(fs::read(&board_path).unwrap(), before);

    let written = fs::read(&target).unwrap();
    let (code, _, err) = command(&["script", "run", "--out", target_text, &script, &board_path]);
    assert_eq!(code, 2);
    assert!(err.contains("never replaced"), "{err}");
    assert_eq!(fs::read(&target).unwrap(), written);

    let (code, _, err) = command(&["script", "run", "--out", &board_path, &script, &board_path]);
    assert_eq!(code, 2, "{err}");
    assert_eq!(fs::read(&board_path).unwrap(), before);
}

#[test]
fn dry_run_wins_over_out() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let target = dir.path().join("never.json");
    let (code, out, _) = command(&[
        "script",
        "run",
        "--dry-run",
        "--out",
        target.to_str().unwrap(),
        &script,
        &board_path,
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("dry run"), "{out}");
    assert!(!target.exists());
}

#[test]
fn a_script_that_cannot_run_exits_1_with_the_place_of_the_problem() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "bad.scr", "GRID mm\nFROBNICATE\n");
    let target = dir.path().join("never.json");
    let (code, out, _) = command(&[
        "script",
        "run",
        "--out",
        target.to_str().unwrap(),
        &script,
        &board_path,
    ]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-006"), "{out}");
    assert!(out.contains("[2:1]"), "{out}");
    assert!(out.contains("stopped, nothing changed"), "{out}");
    assert!(!target.exists());

    let (code, out, _) = command(&["script", "run", "--lang", "es", &script, &board_path]);
    assert_eq!(code, 1);
    assert!(out.contains("Orden desconocida"), "{out}");
}

#[test]
fn an_error_in_the_middle_changes_nothing_even_with_out() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(
        dir.path(),
        "mid.scr",
        "LAYER Top\nWIRE 'GND' (1 1) (5 1)\nMOVE R9 (1 1)\n",
    );
    let target = dir.path().join("never.json");
    let (code, out, _) = command(&[
        "script",
        "run",
        "--out",
        target.to_str().unwrap(),
        &script,
        &board_path,
    ]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-009"), "{out}");
    assert!(!target.exists());
}

#[test]
fn json_output_has_the_counts_and_the_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let (code, out, err) = command(&["script", "run", "--json", &script, &board_path]);
    assert_eq!(code, 0, "{err}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["schema"], "phoenix-script-run/1");
    assert_eq!(json["ok"], true);
    assert_eq!(json["operations"], 1);
    assert_eq!(json["executed"], 2);
    let bad = write_script(dir.path(), "bad.scr", "FROBNICATE");
    let (code, out, _) = command(&["script", "run", "--json", &bad, &board_path]);
    assert_eq!(code, 1);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["diagnostics"][0]["code"], "PHOENIX-SCRIPT-006");
    assert_eq!(json["diagnostics"][0]["line"], 1);
}

#[test]
fn a_schematic_takes_erc_and_is_never_written() {
    let dir = tempfile::tempdir().unwrap();
    let schematic = dir.path().join("sch.json");
    fs::write(
        &schematic,
        serde_json::to_vec(&Schematic::default()).unwrap(),
    )
    .unwrap();
    let script = write_script(dir.path(), "erc.scr", "GRID mm 0.5\nERC\n");
    let (code, out, err) = command(&["script", "run", &script, schematic.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("result: carried out"), "{out}");
    let target = dir.path().join("never.json");
    let (code, _, err) = command(&[
        "script",
        "run",
        "--out",
        target.to_str().unwrap(),
        &script,
        schematic.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("a schematic is not changed"), "{err}");
    assert!(!target.exists());
    let wire = write_script(dir.path(), "wire.scr", WIRE);
    let (code, out, _) = command(&["script", "run", &wire, schematic.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-016"), "{out}");
}

#[test]
fn unusable_arguments_and_inputs_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let missing = dir.path().join("none").to_str().unwrap().to_owned();
    for args in [
        vec!["script"],
        vec!["script", "run"],
        vec!["script", "run", script.as_str()],
        vec![
            "script",
            "run",
            "--nope",
            script.as_str(),
            board_path.as_str(),
        ],
        vec![
            "script",
            "run",
            "--lang",
            "xx",
            script.as_str(),
            board_path.as_str(),
        ],
        vec!["script", "run", "--out"],
        vec!["script", "run", missing.as_str(), board_path.as_str()],
        vec!["script", "run", script.as_str(), missing.as_str()],
        vec!["script", "report"],
        vec!["script", "report", missing.as_str()],
        vec!["script", "other"],
        vec!["cam-import"],
        vec!["cam-import", missing.as_str()],
    ] {
        let (code, out, err) = command(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}: {out}");
        assert!(!err.is_empty(), "{args:?}");
    }
    let text = write_script(dir.path(), "notes.txt", "not a board");
    let (code, _, err) = command(&["script", "run", script.as_str(), text.as_str()]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn a_script_larger_than_the_limit_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let config = write_script(dir.path(), "script.cfg", "max_script_bytes = 5\n");
    let (code, _, err) = command(&["script", "run", "--config", &config, &script, &board_path]);
    assert_eq!(code, 2);
    assert!(err.contains("larger than 5 bytes"), "{err}");
    let bad = write_script(dir.path(), "bad.cfg", "nope = 1\n");
    let (code, _, err) = command(&["script", "run", "--config", &bad, &script, &board_path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SCRIPT-001"), "{err}");
}

#[test]
fn write_in_a_script_needs_the_permission_and_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "w.scr", "WRITE 'out/copy.json'\n");
    let (code, out, _) = command(&["script", "run", &script, &board_path]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-013"), "{out}");

    let config = write_script(
        dir.path(),
        "script.cfg",
        "allow_write = true\nallowed_dirs = out\n",
    );
    let (code, out, err) = command(&["script", "run", "--config", &config, &script, &board_path]);
    assert_eq!(code, 0, "{err}");
    assert!(!dir.path().join("out").exists(), "{out}");

    let target = dir.path().join("result.json");
    let (code, out, err) = command(&[
        "script",
        "run",
        "--config",
        &config,
        "--out",
        target.to_str().unwrap(),
        &script,
        &board_path,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("copy.json"), "{out}");
    assert_eq!(
        fs::read(dir.path().join("out").join("copy.json")).unwrap(),
        board().to_bytes().unwrap()
    );
}

fn scripts_folder(dir: &Path) {
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("a.scr"), "GRID mm 0.5\nLAYER Top\nDRC\n").unwrap();
    fs::write(dir.join("sub").join("b.scr"), "MOVE (1 1) (2 2)\nROUTE\n").unwrap();
    fs::write(dir.join("notes.txt"), "GRID mm").unwrap();
}

#[test]
fn the_report_lists_every_script_with_its_totals_and_never_a_full_path() {
    let dir = tempfile::tempdir().unwrap();
    scripts_folder(dir.path());
    let before = fs::read(dir.path().join("a.scr")).unwrap();
    let folder = dir.path().to_str().unwrap();
    let (code, out, err) = command(&["script", "report", folder]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("a.scr") && out.contains("sub/b.scr"), "{out}");
    assert!(!out.contains("notes.txt"), "{out}");
    assert!(out.contains("PHOENIX-SCRIPT-028"), "{out}");
    assert!(out.contains("PHOENIX-SCRIPT-027"), "{out}");
    assert!(!out.contains(folder), "{out}");
    assert_eq!(fs::read(dir.path().join("a.scr")).unwrap(), before);

    let (code, out, _) = command(&["script", "report", "--json", folder]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["totals"]["scripts"], 2);
    assert_eq!(json["totals"]["commands"], 5);
    assert_eq!(json["totals"]["equivalent"], 2);
    assert_eq!(json["totals"]["adapted"], 1);
    assert_eq!(json["totals"]["pending"], 2);

    let (code, out, _) = command(&["script", "report", "--lang", "de", folder]);
    assert_eq!(code, 0);
    assert!(out.contains("Kompatibilitätsbericht der Skripte"), "{out}");

    let single = dir.path().join("a.scr");
    let (code, out, _) = command(&["script", "report", single.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.contains("a.scr") && !out.contains("sub/b.scr"), "{out}");
}

#[test]
fn the_html_report_is_written_once_to_a_new_file() {
    let dir = tempfile::tempdir().unwrap();
    scripts_folder(dir.path());
    let page_dir = tempfile::tempdir().unwrap();
    let page = page_dir.path().join("report.html");
    let folder = dir.path().to_str().unwrap();
    let (code, out, err) = command(&[
        "script",
        "report",
        "--html",
        page.to_str().unwrap(),
        "--lang",
        "ja",
        folder,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let html = fs::read_to_string(&page).unwrap();
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("スクリプト互換性レポート"));
    assert!(html.contains("sub/b.scr"));
    assert!(!html.contains(folder));
    let (code, _, err) = command(&["script", "report", "--html", page.to_str().unwrap(), folder]);
    assert_eq!(code, 2, "{err}");
    assert_eq!(fs::read_to_string(&page).unwrap(), html);
}

const JOB: &str = "\
[CAM Processor Job]
Description=Two layer boards

[Section]
Name=%N.cmp
Device=GERBER_RS274X
Layers=1 17 18 20

[Section]
Name=%N.drd
Device=EXCELLON
Layers=44 45

[Section]
Name=%N.plt
Device=HPGL
Layers=1
";

#[test]
fn cam_import_reports_what_was_translated_and_writes_the_profile_once() {
    let dir = tempfile::tempdir().unwrap();
    let job = write_script(dir.path(), "two layers.cam", JOB);
    let (code, out, err) = command(&["cam-import", &job]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("files: 3"), "{out}");
    assert!(out.contains("PHOENIX-SCRIPT-024"), "{out}");
    assert!(out.contains("PHOENIX-SCRIPT-026"), "{out}");

    let profile = dir.path().join("profile.json");
    let (code, out, err) = command(&["cam-import", "--out", profile.to_str().unwrap(), &job]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let text = fs::read_to_string(&profile).unwrap();
    let set = ProfileSet::from_json(&text).unwrap();
    let found = set.get("two-layers").unwrap();
    assert_eq!(found.gerber.extension, "cmp");
    assert_eq!(found.drill.plated_file, "board.drd");
    let (code, _, _) = command(&["cam-import", "--out", profile.to_str().unwrap(), &job]);
    assert_eq!(code, 2);
    assert_eq!(fs::read_to_string(&profile).unwrap(), text);

    let (code, out, _) = command(&["cam-import", "--json", "--lang", "pt", &job]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["schema"], "phoenix-cam-import/1");
    assert_eq!(json["pending"], 1);

    let empty = write_script(dir.path(), "empty.cam", "[Job]\n");
    let (code, _, err) = command(&["cam-import", &empty]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SCRIPT-023"), "{err}");
}

const SECOND: &str = "phoenix-script/2 board\nlet t = route(\"GND\", \"Top\", 0.2mm, [0mm, 0mm, 5mm, 0mm])\nprint(t.length)\n";

#[test]
fn a_script_of_the_second_version_is_a_dry_run_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.phs", SECOND);
    let before = fs::read(&board_path).unwrap();
    let (code, out, err) = command(&["script", "run", &script, "--board", &board_path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("result: carried out"), "{out}");
    assert!(out.contains("\n5000000\n"), "{out}");
    assert!(out.contains("history steps: 1"), "{out}");
    assert!(out.contains("dry run: nothing was written"), "{out}");
    assert!(err.contains("dry run"), "{err}");
    assert_eq!(fs::read(&board_path).unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn out_writes_the_result_of_a_second_version_script_once() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.phs", SECOND);
    let target = dir.path().join("result.json");
    let target_text = target.to_str().unwrap();
    let (code, out, err) = command(&[
        "script",
        "run",
        "--json",
        "--out",
        target_text,
        &script,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 0, "{err}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["schema"], "phoenix-script-run/2");
    assert_eq!(json["ok"], true);
    assert_eq!(json["written"][0], target_text);
    let result = Board::from_bytes(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(result.tracks.len(), 1);
    let written = fs::read(&target).unwrap();
    let (code, _, err) = command(&[
        "script",
        "run",
        "--out",
        target_text,
        &script,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("never replaced"), "{err}");
    assert_eq!(fs::read(target.as_path()).unwrap(), written);
}

#[test]
fn a_second_version_script_that_fails_exits_1_with_line_and_column() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let syntax = write_script(dir.path(), "a.phs", "phoenix-script/2\nlet = 1\n");
    let (code, out, _) = command(&["script", "run", &syntax, "--board", &board_path]);
    assert_eq!(code, 1);
    assert!(
        out.contains("PHOENIX-SCRIPT-043") && out.contains("[2:5]"),
        "{out}"
    );
    let runtime = write_script(
        dir.path(),
        "b.phs",
        "phoenix-script/2\nprint(1)\nprint(zzz)\n",
    );
    let (code, out, _) = command(&[
        "script",
        "run",
        "--lang",
        "pt",
        &runtime,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 1);
    assert!(
        out.contains("A variável não está definida [3:7]: zzz"),
        "{out}"
    );
    assert!(out.contains("result: stopped, nothing changed"), "{out}");
    let target = write_script(dir.path(), "c.phs", "phoenix-script/2 library\nprint(1)\n");
    let (code, out, _) = command(&["script", "run", &target, "--board", &board_path]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-058"), "{out}");
}

#[test]
fn the_data_folder_is_the_only_place_a_script_can_write_and_only_when_applied() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let config = write_script(dir.path(), "script.cfg", "data_dir = data\n");
    let script = write_script(
        dir.path(),
        "w.phs",
        "phoenix-script/2\nwrite_text(\"a.txt\", \"hi\")\n",
    );
    let file = dir.path().join("data").join("a.txt");
    let (code, out, _) = command(&[
        "script",
        "run",
        "--config",
        &config,
        &script,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("file: a.txt (2 bytes)"), "{out}");
    assert!(!file.exists());
    let target = dir.path().join("out.json");
    let (code, _, err) = command(&[
        "script",
        "run",
        "--config",
        &config,
        "--out",
        target.to_str().unwrap(),
        &script,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(fs::read_to_string(file.as_path()).unwrap(), "hi");
    let escape = write_script(
        dir.path(),
        "e.phs",
        "phoenix-script/2\nprint(read_text(\"../script.cfg\"))\n",
    );
    let (code, out, _) = command(&[
        "script",
        "run",
        "--config",
        &config,
        &escape,
        "--board",
        &board_path,
    ]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-SCRIPT-056"), "{out}");
}

#[test]
fn check_list_and_help_work_without_a_document() {
    let dir = tempfile::tempdir().unwrap();
    let good = write_script(dir.path(), "g.phs", SECOND);
    let (code, out, _) = command(&["script", "check", &good]);
    assert_eq!(code, 0);
    assert!(out.starts_with("check: ok\n"), "{out}");
    assert!(out.contains("functions: print x1, route x1"), "{out}");
    let bad = write_script(
        dir.path(),
        "b.phs",
        "phoenix-script/2\nprint(zzz)\nnothing(1)\n",
    );
    let (code, out, _) = command(&["script", "check", "--json", &bad]);
    assert_eq!(code, 1);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["schema"], "phoenix-script-check/2");
    assert_eq!(json["errors"].as_array().unwrap().len(), 2);
    let (code, out, _) = command(&["script", "list"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("led_matrix") && out.contains("panelize_matrix"),
        "{out}"
    );
    let (code, out, _) = command(&["script", "list", "--show", "led_matrix"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("phoenix-script/2 board\n"), "{out}");
    let (code, _, err) = command(&["script", "list", "--show", "nothing"]);
    assert_eq!(code, 2);
    assert!(err.contains("no example"), "{err}");
    let (code, out, _) = command(&["script", "list", "--json"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(json["examples"].as_array().unwrap().len() >= 7);
    let (code, out, _) = command(&["script", "help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("phoenix-script/2 - the language"), "{out}");
    assert!(out.contains("move_to(target, x, y)"), "{out}");
}

#[test]
fn the_first_version_still_runs_with_the_document_given_by_a_flag() {
    let dir = tempfile::tempdir().unwrap();
    let board_path = write_board(dir.path());
    let script = write_script(dir.path(), "s.scr", WIRE);
    let (code, out, err) = command(&["script", "run", &script, "--board", &board_path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("operations: 1"), "{out}");
    let (code, _, _) = command(&["script", "run", &script]);
    assert_eq!(code, 2);
}
