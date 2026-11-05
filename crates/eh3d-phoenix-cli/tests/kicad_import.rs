// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix kicad-import`: a small KiCad project, written by hand in
//! `tests/kicad_files/`, read into the design model, written as new files, and checked against the
//! fabrication files that the export makes from the same board.
//!
//! The project: two resistors R1 and R2 of one footprint on a 30 mm by 20 mm board, joined by the
//! net `SIG`. In the file (y down) R1 is at (10, 10) and R2 at (20, 10); the model turns y, so
//! R1 is at (10 mm, -10 mm) and R2 at (20 mm, -10 mm), in nanometres 10_000_000 and -10_000_000.
//! Each footprint has a round through-hole pad 1 of 1.6 mm with a 0.8 mm drill at its origin and a
//! rectangular surface-mount pad 2 of 1 mm by 2 mm at 5 mm to the right. One track of 0.25 mm runs
//! from (10, 10) to (16, 10) on the top copper and one via of 0.6 mm with a 0.3 mm drill is at
//! (16, 12).

use std::path::Path;

use eh3d_phoenix_cli::run;
use eh3d_phoenix_export::Profile;
use eh3d_phoenix_gerber::{round_trip, ImportSettings, Role};
use eh3d_phoenix_kicad::{read_pcb, sequential_ids, KicadSettings};
use eh3d_phoenix_model::Point;

const PRO: &str = include_str!("kicad_files/demo.kicad_pro");
const SCH: &str = include_str!("kicad_files/demo.kicad_sch");
const PCB: &str = include_str!("kicad_files/demo.kicad_pcb");

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

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

fn project() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let pro = write(&dir, "demo.kicad_pro", PRO);
    write(&dir, "demo.kicad_sch", SCH);
    write(&dir, "demo.kicad_pcb", PCB);
    (dir, pro)
}

fn names(dir: &tempfile::TempDir) -> Vec<String> {
    let mut all: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    all.sort();
    all
}

fn point(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

#[test]
fn a_project_is_read_and_nothing_is_written_without_out() {
    let (dir, pro) = project();
    let (code, text, err) = call(&["kicad-import", &pro]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    for want in [
        "format: kicad",
        "read: project, schematic, board",
        "usable: yes",
        "fidelity: partial",
        "board kept:",
        "components: 2",
        "tracks: 1",
        "vias: 1",
        "schematic kept:",
        "parts: 2",
        "nets: 1",
        "dry run: nothing was written",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
    // the setup of the board is left out and said so; nothing else is missing or differs
    assert!(
        text.contains("setup (stack-up, rules and plot settings)"),
        "{text}"
    );
    assert!(!text.contains("PHOENIX-KICAD-015"), "{text}");
    assert!(!text.contains("PHOENIX-KICAD-017"), "{text}");
    assert_eq!(
        names(&dir),
        vec!["demo.kicad_pcb", "demo.kicad_pro", "demo.kicad_sch"]
    );
}

#[test]
fn the_json_document_holds_the_models_and_the_values_calculated_by_hand() {
    let (_dir, pro) = project();
    let (code, text, err) = call(&["kicad-import", "--json", &pro]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(value["schema"], "phoenix-kicad-import/1");
    assert_eq!(value["fidelity"], "partial");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["written"].as_array().unwrap().len(), 0);
    assert_eq!(value["board"]["version"], "20221018");
    let board = &value["board"]["model"];
    assert_eq!(board["components"].as_object().unwrap().len(), 2);
    assert_eq!(board["footprints"].as_object().unwrap().len(), 1);
    assert_eq!(board["tracks"].as_object().unwrap().len(), 1);
    assert_eq!(board["vias"].as_object().unwrap().len(), 1);
    assert_eq!(board["nets"].as_object().unwrap().len(), 1);
    // the two through-hole pads are on SIG, the surface-mount pads on no net
    assert_eq!(board["pad_nets"].as_array().unwrap().len(), 2);
    assert_eq!(board["outline"].as_array().unwrap().len(), 4);
    let schematic = &value["schematic"]["model"];
    assert_eq!(schematic["parts"].as_object().unwrap().len(), 2);
    let nets = schematic["nets"].as_object().unwrap();
    assert_eq!(nets.len(), 1);
    let net = nets.values().next().unwrap();
    assert_eq!(net["name"], "/SIG");
    assert_eq!(net["pins"].as_array().unwrap().len(), 2);
    let class = &value["project"]["net_classes"][0];
    assert_eq!(class["name"], "Default");
    assert_eq!(class["clearance_nm"], 200_000);
    assert_eq!(class["track_width_nm"], 250_000);
    assert_eq!(class["via_drill_nm"], 300_000);
    assert_eq!(
        value["project"]["design_rules_nm"]["min_track_width"],
        150_000
    );
    assert_eq!(value["project"]["text_variables"], 1);
    // the same input gives the same document
    let (_, again, _) = call(&["kicad-import", "--json", &pro]);
    assert_eq!(again, text);
}

#[test]
fn the_board_has_the_positions_and_sizes_of_the_file_turned_into_the_model() {
    let mut ids = sequential_ids().unwrap();
    let read = read_pcb(PCB.as_bytes(), &KicadSettings::default(), &mut ids).unwrap();
    let board = &read.board;
    let mut parts: Vec<(&str, Point)> = board
        .components
        .values()
        .map(|c| (c.refdes.as_str(), c.at))
        .collect();
    parts.sort_by(|a, b| a.0.cmp(b.0));
    assert_eq!(
        parts,
        vec![
            ("R1", point(10_000_000, -10_000_000)),
            ("R2", point(20_000_000, -10_000_000)),
        ]
    );
    let footprint = board.footprints.values().next().unwrap();
    assert_eq!(footprint.pads.len(), 2);
    let round = footprint.pads.values().find(|p| p.number == "1").unwrap();
    assert_eq!(round.drill.map(|d| d.get()), Some(800_000));
    let rect = footprint.pads.values().find(|p| p.number == "2").unwrap();
    assert_eq!(rect.at, point(5_000_000, 0));
    assert_eq!(rect.drill, None);
    let track = board.tracks.values().next().unwrap();
    assert_eq!(track.width.get(), 250_000);
    assert_eq!(track.path.len(), 1);
    assert_eq!(track.path[0].start(), point(10_000_000, -10_000_000));
    assert_eq!(track.path[0].end(), point(16_000_000, -10_000_000));
    let via = board.vias.values().next().unwrap();
    assert_eq!(via.at, point(16_000_000, -12_000_000));
    assert_eq!((via.drill.get(), via.diameter.get()), (300_000, 600_000));
}

#[test]
fn the_export_of_the_imported_board_reads_back_as_the_same_board() {
    let mut ids = sequential_ids().unwrap();
    let read = read_pcb(PCB.as_bytes(), &KicadSettings::default(), &mut ids).unwrap();
    let trip = round_trip(&read.board, &Profile::default(), &ImportSettings::default()).unwrap();
    assert_eq!(trip.differences, Vec::new());
    assert_eq!(trip.imported.failed(), 0);
    // on the top copper: the two round pads, the two rectangular pads and the via are flashes and
    // the track is one line
    let top = trip.imported.by_role(Role::CopperTop).unwrap();
    let gerber = top.gerber().unwrap();
    assert_eq!(gerber.flashes(), 5);
    assert_eq!(gerber.lines(), 1);
    // the holes of the plated drill file: two through-hole pads and the via
    let plated = trip.imported.by_role(Role::DrillPlated).unwrap();
    assert_eq!(plated.drill().unwrap().hits.len(), 3);
}

#[test]
fn out_writes_the_models_as_new_files_that_the_other_commands_read() {
    let (dir, pro) = project();
    let out = dir.path().join("model.json");
    let out = out.to_str().unwrap();
    let (code, text, err) = call(&["kicad-import", "--out", out, &pro]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("written:"), "{text}");
    assert!(!text.contains("dry run"), "{text}");
    let board_file = dir.path().join("model.board.json");
    let schematic_file = dir.path().join("model.schematic.json");
    assert!(board_file.is_file() && schematic_file.is_file());
    assert!(!dir.path().join("model.json").exists());
    let board = eh3d_phoenix_board::Board::from_bytes(&std::fs::read(&board_file).unwrap());
    assert_eq!(board.unwrap().components.len(), 2);
    let schematic =
        eh3d_phoenix_schematic::Schematic::from_bytes(&std::fs::read(&schematic_file).unwrap());
    assert_eq!(schematic.unwrap().parts.len(), 2);
    // the written board is a board for the commands that take one
    let board_path = board_file.to_str().unwrap();
    let (code, report, err) = call(&["ratsnest", board_path]);
    assert_ne!(code, 2, "{err}");
    assert!(!report.is_empty());
}

#[test]
fn the_commands_that_take_a_board_take_a_kicad_board_too() {
    let (dir, _pro) = project();
    let board = dir.path().join("demo.kicad_pcb");
    let board = board.to_str().unwrap();
    let rules = write(
        &dir,
        "rules.json",
        r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":100000}}"#,
    );
    let (code, report, err) = call(&["check", "--rules", &rules, board]);
    assert_ne!(code, 2, "{err}");
    assert!(!report.is_empty());
    // a KiCad board gives no design rules, so the limits must be named
    let (code, _, err) = call(&["check", board]);
    assert_eq!(code, 2);
    assert!(err.contains("--rules"), "{err}");
    let (code, _, err) = call(&["ratsnest", board]);
    assert_ne!(code, 2, "{err}");
}

#[test]
fn a_single_file_writes_its_model_and_never_over_an_existing_file() {
    let (dir, _pro) = project();
    let board = dir.path().join("demo.kicad_pcb");
    let board = board.to_str().unwrap();
    let out = dir.path().join("only.json");
    let out = out.to_str().unwrap();
    let (code, text, err) = call(&["kicad-import", "--out", out, board]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("read: board"), "{text}");
    assert!(Path::new(out).is_file());
    let first = std::fs::read(out).unwrap();
    // a second run is refused: the file exists
    let (code, text, err) = call(&["kicad-import", "--out", out, board]);
    assert_eq!(code, 2);
    assert!(text.is_empty());
    assert!(err.contains("exists"), "{err}");
    assert!(
        err.contains("The output of the import is not allowed"),
        "{err}"
    );
    assert_eq!(std::fs::read(out).unwrap(), first);
    // the title follows the language
    let (_, _, german) = call(&["kicad-import", "--lang", "de", "--out", out, board]);
    assert!(
        german.contains("Die Ausgabedatei des Imports ist nicht erlaubt"),
        "{german}"
    );
    let (_, _, portuguese) = call(&["kicad-import", "--lang", "pt", "--out", out, board]);
    assert!(
        portuguese.contains("O ficheiro de saída da importação não é permitido"),
        "{portuguese}"
    );
}

#[test]
fn an_output_that_is_not_json_or_is_an_input_is_refused_before_anything_is_read() {
    let (dir, _pro) = project();
    let board = dir.path().join("demo.kicad_pcb");
    let board = board.to_str().unwrap();
    let (code, _, err) = call(&["kicad-import", "--out", board, board]);
    assert_eq!(code, 2);
    assert!(err.contains("only a .json file"), "{err}");
    let settings = write(&dir, "limits.json", "# nothing\n");
    let (code, _, err) = call(&[
        "kicad-import",
        "--settings",
        &settings,
        "--out",
        &settings,
        board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("is an input file"), "{err}");
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), "# nothing\n");
    assert_eq!(
        names(&dir),
        vec![
            "demo.kicad_pcb",
            "demo.kicad_pro",
            "demo.kicad_sch",
            "limits.json"
        ]
    );
}

#[test]
fn a_missing_sibling_and_parts_that_differ_are_findings() {
    let dir = tempfile::tempdir().unwrap();
    let pro = write(&dir, "demo.kicad_pro", PRO);
    write(&dir, "demo.kicad_pcb", PCB);
    let (code, text, _) = call(&["kicad-import", &pro]);
    assert_eq!(code, 0);
    assert!(text.contains("read: project, board"), "{text}");
    assert!(
        text.contains("PHOENIX-KICAD-015") && text.contains("demo.kicad_sch"),
        "{text}"
    );
    assert!(text.contains("fidelity: partial"), "{text}");
    // a board that holds R9 instead of R2 does not list the parts of the schematic
    let other = dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("demo.kicad_pro"), PRO).unwrap();
    std::fs::write(other.join("demo.kicad_sch"), SCH).unwrap();
    std::fs::write(
        other.join("demo.kicad_pcb"),
        PCB.replace("\"R2\"", "\"R9\""),
    )
    .unwrap();
    let (code, text, _) = call(&["kicad-import", other.to_str().unwrap()]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("PHOENIX-KICAD-017"), "{text}");
    assert!(
        text.contains("on the schematic only: R2; on the board only: R9"),
        "{text}"
    );
}

#[test]
fn a_damaged_file_does_not_stop_the_others_and_nothing_readable_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let pro = write(&dir, "demo.kicad_pro", "{\"meta\": ");
    write(&dir, "demo.kicad_sch", SCH);
    write(&dir, "demo.kicad_pcb", PCB);
    let (code, text, _) = call(&["kicad-import", &pro]);
    assert_eq!(code, 0);
    assert!(text.contains("not readable: project"), "{text}");
    assert!(text.contains("PHOENIX-KICAD-011"), "{text}");
    assert!(text.contains("read: schematic, board"), "{text}");
    // the last parenthesis is missing: one list is still open at the end
    let cut = write(&dir, "cut.kicad_pcb", &PCB[..PCB.rfind(')').unwrap()]);
    let (code, text, err) = call(&["kicad-import", &cut]);
    assert_eq!((code, err.is_empty()), (1, true), "{err}");
    assert!(text.contains("usable: no") && text.contains("fidelity: unusable"));
    assert!(text.contains("PHOENIX-KICAD-004"), "{text}");
}

#[test]
fn usage_errors_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent.kicad_pcb");
    let missing = missing.to_str().unwrap().to_string();
    let other = write(&dir, "notes.txt", "hello");
    let empty = tempfile::tempdir().unwrap();
    let empty_path = empty.path().to_str().unwrap().to_string();
    for list in [
        vec!["kicad-import"],
        vec!["kicad-import", "--nope", "x"],
        vec!["kicad-import", "--lang"],
        vec!["kicad-import", "--lang", "xx", "x"],
        vec!["kicad-import", "--out"],
        vec!["kicad-import", "--settings"],
        vec!["kicad-import", "a.kicad_pcb", "b.kicad_pcb"],
        vec!["kicad-import", missing.as_str()],
        vec!["kicad-import", other.as_str()],
        vec!["kicad-import", empty_path.as_str()],
    ] {
        let (code, out, err) = call(&list);
        assert_eq!(code, 2, "{list:?}");
        assert!(out.is_empty(), "{list:?}");
        assert!(err.contains("error:"), "{list:?}: {err}");
    }
}
