// SPDX-License-Identifier: GPL-3.0-or-later
//! The tutorials of `docs/tutorials`: the blocks marked `# test` run the commands they show
//! through `run`, on a copy of `examples/`, and their documented output and exit code are
//! compared with what the command gives. A command that a tutorial shows and that does not exist
//! in the table fails, and every translation keeps the commands of the English text.
//!
//! The format of a block, as the tutorials write it (the lines below are the inside of a block that
//! starts with the line "```console"):
//!
//! ```text
//!     # test
//!     $ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
//!     # exit 1
//!     Confirmed: 1
//!     ~ PHOENIX-CHECK-001
//! ```
//!
//! `$ phoenix ...` is a command; `# exit N` the exit code (0 when it is not given); a plain line
//! is the start of the next line of the output; a line that starts with `~ ` has to be found
//! somewhere in the output and one that starts with `~~ ` somewhere in the error stream. A path
//! that starts with `examples/` is a path of the copy and one that starts with `build/` is a path
//! of a folder that the test makes; both are written without the folder in the output.

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::commands::COMMANDS;
use eh3d_phoenix_cli::run;

const LANGUAGES: [&str; 7] = ["es", "de", "fr", "it", "ja", "zh", "pt"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tutorials_folder() -> PathBuf {
    root().join("docs").join("tutorials")
}

/// The English tutorials: the files `TUTORIAL_*.md` of the folder, by name.
fn tutorial_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(tutorials_folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("TUTORIAL_") && name.ends_with(".md"))
        .collect();
    names.sort();
    names
}

/// The lines of the blocks marked `console`, each block with the number of its first line.
fn console_blocks(text: &str) -> Vec<(usize, Vec<String>)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, Vec<String>)> = None;
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if let Some((_, lines)) = current.as_mut() {
            if line == "```" {
                blocks.extend(current.take());
            } else {
                lines.push(line.to_owned());
            }
        } else if line == "```console" {
            current = Some((number + 1, Vec::new()));
        }
    }
    blocks
}

/// The words of a command line: separated by spaces, a word between double quotes keeps them.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            ' ' if !quoted => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            other => {
                word.push(other);
                started = true;
            }
        }
    }
    if started {
        out.push(word);
    }
    out
}

/// One command of a block with what the block says about its result.
struct Step {
    arguments: Vec<String>,
    exit: i32,
    start: Vec<String>,
    found: Vec<String>,
    found_in_errors: Vec<String>,
}

fn steps(lines: &[String]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in lines {
        if let Some(command) = line.strip_prefix("$ phoenix ") {
            steps.push(Step {
                arguments: words(command),
                exit: 0,
                start: Vec::new(),
                found: Vec::new(),
                found_in_errors: Vec::new(),
            });
            continue;
        }
        let Some(step) = steps.last_mut() else {
            continue;
        };
        if let Some(code) = line.strip_prefix("# exit ") {
            step.exit = code.trim().parse().expect("a number after # exit");
        } else if let Some(text) = line.strip_prefix("~~ ") {
            step.found_in_errors.push(text.to_owned());
        } else if let Some(text) = line.strip_prefix("~ ") {
            step.found.push(text.to_owned());
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            step.start.push(line.to_owned());
        }
    }
    steps
}

fn copy_folder(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_folder(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn plain(text: &str, folder: &str) -> String {
    text.replace('\\', "/").replace(&format!("{folder}/"), "")
}

/// Runs the steps of a block in the folder and says what differs from the documentation.
fn check_step(step: &Step, folder: &Path, where_: &str) -> Vec<String> {
    let base = folder.to_string_lossy().replace('\\', "/");
    let arguments: Vec<String> = step
        .arguments
        .iter()
        .map(|word| {
            if word.starts_with("examples/") || word.starts_with("build/") {
                format!("{base}/{word}")
            } else {
                word.clone()
            }
        })
        .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&arguments, &mut out, &mut err);
    let out = plain(&String::from_utf8_lossy(&out), &base);
    let err = plain(&String::from_utf8_lossy(&err), &base);
    let mut problems = Vec::new();
    let command = step.arguments.join(" ");
    if code != step.exit {
        problems.push(format!(
            "{where_}: phoenix {command}: exit {code}, documented {}\n{out}{err}",
            step.exit
        ));
    }
    let lines: Vec<&str> = out.lines().collect();
    for (i, want) in step.start.iter().enumerate() {
        match lines.get(i) {
            Some(line) if line.starts_with(want.as_str()) => {}
            other => problems.push(format!(
                "{where_}: phoenix {command}: line {} is {other:?}, documented {want:?}",
                i + 1
            )),
        }
    }
    for want in &step.found {
        if !out.contains(want.as_str()) {
            problems.push(format!(
                "{where_}: phoenix {command}: {want:?} is not in the output\n{out}"
            ));
        }
    }
    for want in &step.found_in_errors {
        if !err.contains(want.as_str()) {
            problems.push(format!(
                "{where_}: phoenix {command}: {want:?} is not in the error stream\n{err}"
            ));
        }
    }
    problems
}

#[test]
fn the_commands_of_the_tutorials_give_the_documented_output() {
    let names = tutorial_names();
    assert!(names.len() >= 9, "{names:?}");
    let mut problems = Vec::new();
    for name in &names {
        let text = fs::read_to_string(tutorials_folder().join(name)).unwrap();
        let folder = tempfile::tempdir().unwrap();
        copy_folder(&root().join("examples"), &folder.path().join("examples"));
        fs::create_dir(folder.path().join("build")).unwrap();
        let mut run_blocks = 0;
        for (number, lines) in console_blocks(&text) {
            if lines.first().map(String::as_str) != Some("# test") {
                continue;
            }
            run_blocks += 1;
            for step in steps(&lines) {
                problems.extend(check_step(
                    &step,
                    folder.path(),
                    &format!("{name}:{number}"),
                ));
            }
        }
        assert!(run_blocks >= 2, "{name} has {run_blocks} blocks that run");
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_translations_show_the_same_commands_as_the_english_tutorial() {
    let mut problems = Vec::new();
    for name in tutorial_names() {
        let english = fs::read_to_string(tutorials_folder().join(&name)).unwrap();
        let wanted: Vec<Vec<String>> = console_blocks(&english)
            .into_iter()
            .map(|(_, lines)| lines)
            .collect();
        for language in LANGUAGES {
            let path = tutorials_folder().join(language).join(&name);
            let Ok(text) = fs::read_to_string(&path) else {
                problems.push(format!("{language}/{name} is missing"));
                continue;
            };
            let got: Vec<Vec<String>> = console_blocks(&text)
                .into_iter()
                .map(|(_, lines)| lines)
                .collect();
            if got != wanted {
                problems.push(format!("{language}/{name}: the console blocks differ"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_command_that_a_tutorial_shows_is_in_the_table() {
    let known: Vec<&str> = COMMANDS
        .iter()
        .map(|c| c.name)
        .chain(std::iter::once("help"))
        .collect();
    let mut shown = 0;
    let mut problems = Vec::new();
    let mut files = vec![tutorials_folder()];
    files.extend(LANGUAGES.iter().map(|l| tutorials_folder().join(l)));
    for folder in files {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for (_, lines) in console_blocks(&text) {
                for line in lines {
                    if let Some(command) = line.strip_prefix("$ phoenix ") {
                        shown += 1;
                        let first = words(command).into_iter().next().unwrap_or_default();
                        if !known.contains(&first.as_str()) {
                            problems.push(format!("{}: phoenix {first}", path.display()));
                        }
                    }
                }
            }
        }
    }
    assert!(shown >= 50, "{shown} commands shown");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_files_the_tutorials_name_exist_in_the_examples() {
    let mut problems = Vec::new();
    for name in tutorial_names() {
        let text = fs::read_to_string(tutorials_folder().join(&name)).unwrap();
        for (_, lines) in console_blocks(&text) {
            for line in lines.iter().filter(|l| l.starts_with("$ phoenix ")) {
                for word in words(line) {
                    if word.starts_with("examples/") && !root().join(&word).exists() {
                        problems.push(format!("{name}: {word}"));
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
