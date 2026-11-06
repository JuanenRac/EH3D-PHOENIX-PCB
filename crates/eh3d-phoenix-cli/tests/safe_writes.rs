// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands write only where they are told to, only to a path that is acceptable and never over
//! a file that exists. The test goes through the table of commands (`commands.rs`), so a command
//! that is added is checked without a line being added here.

use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::commands::COMMANDS;
use eh3d_phoenix_cli::{output_option_problem, run, OUTPUT_OPTIONS};

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

/// Every word that starts a command, the ones of the table and the ones `run` handles itself.
fn command_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = COMMANDS.iter().map(|c| c.name).collect();
    for extra in ["inventory", "eagle-rate"] {
        if !names.contains(&extra) {
            names.push(extra);
        }
    }
    names
}

/// The names of what is in a folder, sorted.
fn listing(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn the_table_of_commands_is_not_empty_and_every_name_is_one_word() {
    let names = command_names();
    assert!(names.len() >= 50, "{} commands", names.len());
    for name in &names {
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "{name}"
        );
    }
}

#[test]
fn a_command_without_arguments_writes_nothing_where_the_tests_run() {
    // the default of every command is a dry run: with no argument nothing is made in the folder
    // that the program runs in. `view` is left out because with the window compiled in it opens
    // the start screen.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let before = listing(&here);
    for name in command_names() {
        if name == "view" {
            continue;
        }
        let _ = call(&[name]);
    }
    assert_eq!(listing(&here), before);
}

#[test]
fn a_command_that_cannot_read_its_input_makes_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.input");
    let missing = missing.to_str().unwrap();
    for name in command_names() {
        if name == "view" {
            continue;
        }
        for option in OUTPUT_OPTIONS {
            let target = dir.path().join(format!("{name}{option}.out"));
            let target_text = target.to_str().unwrap();
            let _ = call(&[name, option, target_text, missing]);
            assert!(
                !target.exists(),
                "{name} {option} made a file for an input that is not there"
            );
        }
    }
    assert!(listing(dir.path()).is_empty());
}

#[test]
fn every_output_option_refuses_the_paths_that_are_not_acceptable() {
    let bad = [
        "",
        "NUL",
        "work/aux.txt",
        "out.json:stream",
        "\\\\server\\share\\out",
        "\\\\.\\COM1",
        "//?/C:/out",
    ];
    for name in command_names() {
        for option in OUTPUT_OPTIONS {
            for path in bad {
                let (code, _, err) = call(&[name, option, path]);
                assert_eq!(code, 2, "{name} {option} {path:?}");
                assert!(
                    err.contains("PHOENIX-SEC-001"),
                    "{name} {option} {path:?}: {err}"
                );
            }
        }
    }
}

#[test]
fn a_path_that_is_fine_is_not_refused_by_the_check() {
    let good: Vec<String> = ["out.json", "folder/out.csv", "C:\\work\\out.svg", "../up/x"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    for option in OUTPUT_OPTIONS {
        for path in &good {
            let args = vec!["sync".to_owned(), option.to_owned(), path.clone()];
            assert_eq!(output_option_problem(&args), None, "{option} {path}");
        }
    }
    // an option that is not an output option is not looked at
    let args = vec!["x".to_owned(), "--lang".to_owned(), "NUL".to_owned()];
    assert_eq!(output_option_problem(&args), None);
}

#[test]
fn the_output_options_are_the_ones_the_commands_use() {
    // the options of the table that name a file or a folder to write; a new one must be added to
    // `OUTPUT_OPTIONS` and to this list on purpose
    assert_eq!(
        OUTPUT_OPTIONS,
        ["--out", "--dir", "--into", "--svg", "--csv", "--html", "--parts-out"]
    );
}
