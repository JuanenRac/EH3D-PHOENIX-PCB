// SPDX-License-Identifier: GPL-3.0-or-later
//! The starter kit of `kit/`: its catalogue, the command `kit`, and every piece of it. The
//! libraries pass their check and read back, the sample boards and schematics pass the rule
//! checks and read back, they are exported with the profiles of the kit, and every user program
//! prints what its `.expected` file says. Each test gathers all that is wrong before it fails.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::ObjectId;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn kit() -> PathBuf {
    root().join("kit")
}

fn text_of(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn in_kit(relative: &str) -> String {
    text_of(&kit().join(relative))
}

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

fn catalogue() -> Vec<Value> {
    let text = fs::read_to_string(kit().join("manifest.json")).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    value["items"].as_array().unwrap().clone()
}

/// The names of the pieces of one type.
fn names_of(kind: &str) -> Vec<String> {
    catalogue()
        .iter()
        .filter(|i| i["type"] == kind)
        .map(|i| i["name"].as_str().unwrap().to_owned())
        .collect()
}

fn ids() -> impl FnMut() -> ObjectId {
    let mut n = 0u16;
    move || {
        n += 1;
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
}

fn none_wrong(wrong: &[String]) {
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn the_catalogue_has_every_kind_of_piece_and_every_piece_is_own_work() {
    let items = catalogue();
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for item in &items {
        *count.entry(item["type"].as_str().unwrap()).or_default() += 1;
        assert_eq!(item["license"], "GPL-3.0-or-later", "{item}");
        assert_eq!(item["provenance"], "own", "{item}");
        assert_eq!(item["verified"], false, "{item}");
    }
    let wanted: BTreeMap<&str, usize> = [
        ("library", 5),
        ("board", 12),
        ("schematic", 12),
        ("program", 20),
        ("dfm", 4),
        ("rules", 2),
        ("export-profile", 2),
    ]
    .into_iter()
    .collect();
    assert_eq!(count, wanted);
}

#[test]
fn list_and_info_show_the_pieces_in_the_language_asked_for() {
    let folder = text_of(&kit());
    let (code, out, err) = call(&["kit", "list", "--kit", &folder]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("library-passives") && out.contains("57 / 57"),
        "{out}"
    );
    let (code, out, _) = call(&[
        "kit", "list", "--kit", &folder, "--type", "board", "--lang", "es",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("12 / 57") && out.contains("(placa)"), "{out}");
    assert!(!out.contains("schematic-"), "{out}");
    let (code, out, _) = call(&["kit", "list", "--kit", &folder, "--json"]);
    assert_eq!(code, 0);
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["format"], "phoenix-kit-list/1");
    assert_eq!(value["items"].as_array().unwrap().len(), 57);
    let (code, out, err) = call(&[
        "kit",
        "info",
        "--kit",
        &folder,
        "--lang",
        "pt",
        "ulp-bom-csv",
    ]);
    assert_eq!(code, 0, "{err}");
    for wanted in [
        "GPL-3.0-or-later",
        "Proveniência: own",
        "Verificado: não",
        "boards/regulator_sot223.brd",
        "ulp/bom_csv.ulp",
    ] {
        assert!(out.contains(wanted), "{wanted}: {out}");
    }
}

#[test]
fn wrong_arguments_and_unknown_pieces_are_refused() {
    let folder = text_of(&kit());
    for (args, code) in [
        (vec!["kit"], None),
        (vec!["kit", "wipe"], Some("PHOENIX-KIT-007")),
        (
            vec!["kit", "list", "--type", "firmware"],
            Some("PHOENIX-KIT-007"),
        ),
        (
            vec!["kit", "copy", "library-passives"],
            Some("PHOENIX-KIT-007"),
        ),
        (
            vec!["kit", "info", "--kit", &folder, "nothing"],
            Some("PHOENIX-KIT-006"),
        ),
        (
            vec!["kit", "list", "--kit", "no-such-folder"],
            Some("PHOENIX-KIT-001"),
        ),
    ] {
        let (exit, out, err) = call(&args);
        assert_eq!((exit, out.as_str()), (2, ""), "{args:?}: {err}");
        if let Some(code) = code {
            assert!(err.contains(code), "{args:?}: {err}");
        }
    }
}

#[test]
fn copy_all_gives_the_same_bytes_and_never_writes_over_a_file() {
    let folder = text_of(&kit());
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("mine");
    let target = text_of(&out_dir);
    // this also checks the SHA-256 of every file of the catalogue
    let (code, out, err) = call(&["kit", "copy", "--kit", &folder, "--all", "--out", &target]);
    assert_eq!(code, 0, "{err}");
    let mut files = 0;
    for item in catalogue() {
        for file in item["files"].as_array().unwrap() {
            let relative = file["path"].as_str().unwrap();
            assert_eq!(
                fs::read(out_dir.join(relative)).unwrap(),
                fs::read(kit().join(relative)).unwrap(),
                "{relative}"
            );
            files += 1;
        }
    }
    assert!(out.contains(&format!("{files} files copied")), "{out}");
    let changed = out_dir.join("ulp/bom_csv.ulp");
    fs::write(&changed, "mine").unwrap();
    let (code, _, err) = call(&[
        "kit",
        "copy",
        "--kit",
        &folder,
        "--out",
        &target,
        "ulp-bom-csv",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-KIT-005"), "{err}");
    assert_eq!(fs::read_to_string(&changed).unwrap(), "mine");
}

#[test]
fn a_changed_file_or_a_path_outside_the_kit_is_not_copied() {
    let dir = tempfile::tempdir().unwrap();
    let mine = dir.path().join("kit");
    let folder = text_of(&mine);
    let (code, _, err) = call(&[
        "kit",
        "copy",
        "--kit",
        &text_of(&kit()),
        "--out",
        &folder,
        "rules-default",
    ]);
    assert_eq!(code, 0, "{err}");
    fs::copy(kit().join("manifest.json"), mine.join("manifest.json")).unwrap();
    fs::write(mine.join("rules/default.rules.json"), "{}").unwrap();
    let out_dir = dir.path().join("out");
    let target = text_of(&out_dir);
    let (code, _, err) = call(&[
        "kit",
        "copy",
        "--kit",
        &folder,
        "--out",
        &target,
        "rules-default",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-KIT-004"), "{err}");
    assert!(!out_dir.exists());
    let text = fs::read_to_string(mine.join("manifest.json"))
        .unwrap()
        .replace("rules/default.rules.json", "../outside.json");
    fs::write(mine.join("manifest.json"), text).unwrap();
    let (code, _, err) = call(&["kit", "list", "--kit", &folder]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-KIT-002"), "{err}");
}

#[test]
fn every_library_passes_its_check_and_reads_back_the_same() {
    use eh3d_phoenix_libedit::{check_library, read_library, write, LibSettings};
    let settings = LibSettings::default();
    let mut wrong = Vec::new();
    for name in names_of("library") {
        let path = in_kit(&format!("libraries/{name}.lbr"));
        let bytes = fs::read(&path).unwrap();
        let library = match read_library(&bytes, &settings.limits()) {
            Ok(library) => library,
            Err(e) => {
                wrong.push(format!("{name}: {e}"));
                continue;
            }
        };
        if library.kept_raw != 0 || library.packages.is_empty() || library.devicesets.is_empty() {
            wrong.push(format!("{name}: {} items not read", library.kept_raw));
        }
        for finding in check_library(&library, &settings).findings {
            wrong.push(format!("{name}: {finding:?}"));
        }
        if let Err(e) = write::verify_round_trip(&library) {
            wrong.push(format!("{name}: {e}"));
        }
        let (code, out, err) = call(&["libedit", &path]);
        if code != 0 {
            wrong.push(format!("{name}: libedit {code}: {out}{err}"));
        }
    }
    none_wrong(&wrong);
}

#[test]
fn every_board_and_schematic_passes_the_checks_and_reads_back() {
    use eh3d_phoenix_eagle_board::{
        read_board, read_schematic, round_trip, schematic_signature, write_schematic,
    };
    use eh3d_phoenix_erc::{check, ErcConfig};
    use eh3d_phoenix_rules::{check_board, RuleValues};
    let mut wrong = Vec::new();
    let boards = names_of("board");
    assert_eq!(boards, names_of("schematic"));
    for name in boards {
        let board_path = in_kit(&format!("boards/{name}.brd"));
        let schematic_path = in_kit(&format!("schematics/{name}.sch"));
        let board_text = fs::read_to_string(&board_path).unwrap();
        let schematic_text = fs::read_to_string(&schematic_path).unwrap();
        let mut new_id = ids();
        let read = match read_board(&board_text, &mut new_id) {
            Ok(read) => read,
            Err(e) => {
                wrong.push(format!("{name}: {}", e.message));
                continue;
            }
        };
        for d in &read.diagnostics {
            wrong.push(format!("{name}: board: {}", d.message));
        }
        let board = read.board;
        if !board.connectivity().is_fully_connected() {
            wrong.push(format!(
                "{name}: not routed: {:?}",
                board.connectivity().diagnostics
            ));
        }
        let rules = RuleValues::from_eagle_board(&board_text).unwrap().0;
        for finding in check_board(&board, &rules).confirmed {
            wrong.push(format!("{name}: rule {finding:?}"));
        }
        match round_trip(&board, &mut new_id) {
            Ok(again) if again.identical => {}
            Ok(again) => wrong.push(format!(
                "{name}: board round trip: {:?} {:?}",
                again.only_in_original, again.only_in_copy
            )),
            Err(e) => wrong.push(format!("{name}: {}", e.message)),
        }
        let footprints: BTreeMap<String, ObjectId> = board
            .footprints
            .iter()
            .map(|(id, f)| (f.name.clone(), *id))
            .collect();
        let schematic = match read_schematic(&schematic_text, &footprints, &mut new_id) {
            Ok(read) => {
                for d in &read.diagnostics {
                    if !d.message.ends_with("label element(s)") {
                        wrong.push(format!("{name}: schematic: {}", d.message));
                    }
                }
                read.schematic
            }
            Err(e) => {
                wrong.push(format!("{name}: {}", e.message));
                continue;
            }
        };
        for finding in check(&schematic, &ErcConfig::default()).confirmed {
            wrong.push(format!("{name}: erc {finding:?}"));
        }
        let written = write_schematic(&board, &schematic);
        match read_schematic(&written.text, &footprints, &mut new_id) {
            Ok(again) => {
                let before = schematic_signature(&schematic);
                let after = schematic_signature(&again.schematic);
                if before != after {
                    wrong.push(format!(
                        "{name}: schematic round trip: {before:?} {after:?}"
                    ));
                }
            }
            Err(e) => wrong.push(format!("{name}: {}", e.message)),
        }
        for args in [
            vec!["check", &board_path],
            vec!["connectivity", &schematic_path, &board_path],
            vec!["erc", &schematic_path, &board_path],
        ] {
            let (code, out, err) = call(&args);
            if code != 0 {
                wrong.push(format!("{name}: {} {code}: {out}{err}", args[0]));
            }
        }
    }
    none_wrong(&wrong);
}

#[test]
fn the_profiles_of_the_kit_check_and_export_the_sample_boards() {
    let mut wrong = Vec::new();
    let dir = tempfile::tempdir().unwrap();
    for name in names_of("board") {
        let board = in_kit(&format!("boards/{name}.brd"));
        for profile in names_of("dfm") {
            // the wide figures for hand work are not meant for the fine thin packages
            if profile == "hand_assembly" && name.contains("ssop") {
                continue;
            }
            let file = in_kit(&format!("rules/{profile}.cfg"));
            let (code, out, err) = call(&["check", "--dfm", &file, &board]);
            if code != 0 || !out.contains("PHOENIX-DFM-006") {
                wrong.push(format!("{name} {profile} {code}: {out}{err}"));
            }
        }
        let rules = in_kit("rules/default.rules.json");
        let (code, out, err) = call(&["check", "--rules", &rules, &board]);
        if code != 0 {
            wrong.push(format!("{name} default rules {code}: {out}{err}"));
        }
        for (file, profile) in [
            ("cam/gerber_basic.json", "kit-gerber-basic"),
            ("cam/assembly_files.json", "kit-assembly"),
        ] {
            let target = text_of(&dir.path().join(format!("{name}-{profile}")));
            let (code, out, err) = call(&[
                "export",
                "--profiles",
                &in_kit(file),
                "--profile",
                profile,
                "--out",
                &target,
                &board,
            ]);
            if code != 0 {
                wrong.push(format!("{name} {profile} {code}: {out}{err}"));
            }
        }
        let schematic = in_kit(&format!("schematics/{name}.sch"));
        for args in [
            vec!["kicad-export", "board", &board],
            vec!["kicad-export", "--board", &board, "schematic", &schematic],
        ] {
            let (code, out, err) = call(&args);
            if code != 0 {
                wrong.push(format!("{name} {args:?} {code}: {out}{err}"));
            }
        }
    }
    // the class of the supply nets asks for tracks of 0.3 mm, which these samples have
    for name in [
        "led_indicator",
        "power_input",
        "tht_led_pair",
        "regulator_sot223",
    ] {
        let board = in_kit(&format!("boards/{name}.brd"));
        let rules = in_kit("rules/power_classes.rules.json");
        let (code, out, err) = call(&["check", "--rules", &rules, &board]);
        if code != 0 {
            wrong.push(format!("{name} classes {code}: {out}{err}"));
        }
    }
    none_wrong(&wrong);
}

#[test]
fn every_user_program_is_covered_and_prints_its_expected_output() {
    let settings = text_of(&root().join("config/ulp.cfg"));
    let mut wrong = Vec::new();
    for item in catalogue() {
        if item["type"] != "program" {
            continue;
        }
        let name = item["name"].as_str().unwrap();
        let program = in_kit(&format!("ulp/{name}.ulp"));
        let input = item["runs_on"].as_str().unwrap();
        let option = if input.ends_with(".brd") {
            "--board"
        } else {
            "--schematic"
        };
        let (code, out, err) = call(&["ulp", "report", "--config", &settings, &program]);
        if code != 0 {
            wrong.push(format!("{name}: report {code}: {out}{err}"));
        }
        let (code, out, err) = call(&[
            "ulp",
            "run",
            "--config",
            &settings,
            option,
            &in_kit(input),
            &program,
        ]);
        let expected = fs::read_to_string(kit().join(format!("ulp/{name}.expected"))).unwrap();
        if code != 0 || out != expected {
            wrong.push(format!(
                "{name}: {code} {err}\n--- printed\n{out}--- expected\n{expected}"
            ));
        }
    }
    assert_eq!(names_of("program").len(), 20);
    none_wrong(&wrong);
}
