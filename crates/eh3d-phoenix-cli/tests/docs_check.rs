// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks of the documentation: every relative link of the documents in `docs/` and of the
//! READMEs points to a file or a folder that exists, and every command `phoenix X` that a
//! document shows in code is a command of the table (or `help`).

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::commands::COMMANDS;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Markdown files below `docs/` and the ones in the root of the repository.
fn documents() -> Vec<PathBuf> {
    fn walk(folder: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&root().join("docs"), &mut found);
    for entry in fs::read_dir(root()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// The targets of the links `[text](target)` of a line, without a title.
fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("](") {
        let after = &rest[at + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let inside = &after[..end];
        targets.push(inside.split(' ').next().unwrap_or(""));
        rest = &after[end..];
    }
    targets
}

#[test]
fn every_relative_link_of_the_documents_points_to_something_that_exists() {
    let mut checked = 0;
    let mut problems = Vec::new();
    for path in documents() {
        let text = fs::read_to_string(&path).unwrap();
        for (number, line) in text.lines().enumerate() {
            for target in link_targets(line) {
                let target = target.split('#').next().unwrap_or("");
                if target.is_empty()
                    || target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with("mailto:")
                {
                    continue;
                }
                checked += 1;
                let base = path.parent().unwrap();
                if !base.join(target).exists() {
                    problems.push(format!("{}:{}: {target}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(checked > 100, "{checked} links checked");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The word that follows `phoenix ` in a piece of code, if it is meant as a command.
fn command_word(code: &str) -> Option<&str> {
    let rest = code.strip_prefix("phoenix ")?;
    let word = rest.split_whitespace().next()?;
    let command_like = word
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (command_like && !word.starts_with('-')).then_some(word)
}

/// Every command word that a document shows: in a block of code (the line starts with `phoenix` or
/// `$ phoenix`) and in the code spans of a line.
fn shown_commands(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut in_block = false;
    for (number, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if in_block {
            let code = line.trim_start().trim_start_matches("$ ");
            if let Some(word) = command_word(code) {
                found.push((number + 1, word.to_owned()));
            }
        } else {
            // the odd pieces of a line cut at the backticks are the code spans
            for span in line.split('`').skip(1).step_by(2) {
                if let Some(word) = command_word(span) {
                    found.push((number + 1, word.to_owned()));
                }
            }
        }
    }
    found
}

#[test]
fn every_command_that_a_document_shows_is_in_the_table() {
    let known: Vec<&str> = COMMANDS
        .iter()
        .map(|c| c.name)
        .chain(std::iter::once("help"))
        .collect();
    let mut shown = 0;
    let mut problems = Vec::new();
    for path in documents() {
        let text = fs::read_to_string(&path).unwrap();
        for (number, word) in shown_commands(&text) {
            shown += 1;
            if !known.contains(&word.as_str()) {
                problems.push(format!("{}:{number}: phoenix {word}", path.display()));
            }
        }
    }
    assert!(shown > 500, "{shown} commands shown");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_command_of_the_table_has_a_document_and_is_named_in_the_documents() {
    let all: String = documents()
        .iter()
        .filter(|path| path.parent() == Some(root().join("docs").as_path()))
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    let mut problems = Vec::new();
    for c in COMMANDS {
        if !root().join(c.doc).is_file() {
            problems.push(format!("{}: {} does not exist", c.name, c.doc));
        }
        if !all.contains(&format!("phoenix {}", c.name)) {
            problems.push(format!("{}: no document names the command", c.name));
        }
    }
    assert!(
        problems.is_empty(),
        "{}",
        problems.join(
            "
"
        )
    );
}

#[test]
fn the_links_and_the_words_are_found_where_they_should() {
    assert_eq!(
        link_targets("see [a](x.md) and [b](docs/y.md#z \"t\") and [c](https://e.org)"),
        vec!["x.md", "docs/y.md#z", "https://e.org"]
    );
    assert_eq!(command_word("phoenix sim --op"), Some("sim"));
    assert_eq!(command_word("phoenix --help"), None);
    assert_eq!(command_word("phoenix [--config FILE] inventory"), None);
    assert_eq!(command_word("phoenix <netlist>"), None);
    assert_eq!(command_word("cargo run"), None);
    let text = "Run `phoenix kicad-read x` or\n```text\n$ phoenix panel a\nphoenix zzz\n```\n";
    let words: Vec<String> = shown_commands(text).into_iter().map(|(_, w)| w).collect();
    assert_eq!(words, vec!["kicad-read", "panel", "zzz"]);
}
