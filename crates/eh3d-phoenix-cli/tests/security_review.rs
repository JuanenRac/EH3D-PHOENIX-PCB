// SPDX-License-Identifier: GPL-3.0-or-later
//! The document of the review of security (`docs/SECURITY_REVIEW.md`) says what the settings and
//! the code say: the limits of its tables are the ones in force, the codes it names exist, and the
//! source writes files only in the ways it lists.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn document() -> String {
    fs::read_to_string(root().join("docs/SECURITY_REVIEW.md")).unwrap()
}

/// The rows of the table between two markers, split into their cells.
fn rows(document: &str, name: &str) -> Vec<Vec<String>> {
    let start = format!("<!-- {name}:start -->");
    let end = format!("<!-- {name}:end -->");
    let from = document.find(&start).unwrap() + start.len();
    let to = document.find(&end).unwrap();
    document[from..to]
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|') && !l.starts_with("|---"))
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_owned())
                .collect()
        })
        .skip(1)
        .collect()
}

#[test]
fn the_limits_of_the_document_are_the_ones_of_the_settings() {
    let table = rows(&document(), "limits");
    assert!(table.len() >= 40, "{} rows", table.len());
    for row in &table {
        let [reader, file, key, value] = row.as_slice() else {
            panic!("a row without four cells: {row:?}");
        };
        let text = fs::read_to_string(root().join(file)).unwrap();
        let wanted = format!("{key} = {value}");
        assert!(
            text.lines().any(|l| l.trim() == wanted),
            "{reader}: {file} has no line `{wanted}`"
        );
    }
}

#[test]
fn the_limits_fixed_in_the_code_are_the_ones_of_the_document() {
    let limits = eh3d_phoenix_eagle::Limits::default();
    let expected = [
        ("max_bytes", limits.max_bytes.to_string()),
        ("max_depth", limits.max_depth.to_string()),
        ("max_nodes", limits.max_nodes.to_string()),
        (
            "MAX_CONFIG_BYTES",
            eh3d_phoenix_cli::bounded::MAX_CONFIG_BYTES.to_string(),
        ),
    ];
    let table = rows(&document(), "code-limits");
    assert_eq!(table.len(), expected.len());
    for (row, (key, value)) in table.iter().zip(&expected) {
        assert_eq!(&row[1], key);
        assert_eq!(&row[2], value, "{key}");
    }
}

#[test]
fn the_document_names_the_codes_and_says_what_kind_of_review_it_is() {
    let text = document();
    for (code, _) in eh3d_phoenix_cli::sec_codes::TITLES {
        assert!(text.contains(code), "{code}");
    }
    assert!(text.contains("No external audit has been made"));
    assert!(text.contains("nothing here is a certification"));
    assert!(text.contains("## What was not covered"));
}

// ---------------------------------------------------------------------------------------------
// the ways in which the source writes files

fn files_below(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "tests")
            {
                continue;
            }
            files_below(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// The calls that write a file without creating it exclusively, and why each file may have them.
fn allowed() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "crates/eh3d-phoenix-cli/src/window_run.rs",
            "the error file of the window, in its own folder of settings",
        ),
        (
            "crates/eh3d-phoenix-viewer/src/launcher_session.rs",
            "the list of recent files, in the folder of settings of the window",
        ),
        (
            "crates/eh3d-phoenix-viewer/src/session.rs",
            "the save of a file that the window saved before, after a copy of it",
        ),
        (
            "crates/eh3d-phoenix-route/src/autorouter.rs",
            "files of the external autorouter, in a folder that was made new for the run",
        ),
    ]
}

#[test]
fn the_code_of_production_writes_only_in_the_ways_of_the_document() {
    let words = [
        concat!("fs", "::write("),
        concat!("File", "::create("),
        concat!("fs", "::copy("),
    ];
    let mut files = Vec::new();
    files_below(&root().join("crates"), &mut files);
    assert!(files.len() > 300, "{} files", files.len());
    let allowed = allowed();
    let mut used = vec![false; allowed.len()];
    let mut problems = Vec::new();
    for path in &files {
        let name = path
            .canonicalize()
            .unwrap()
            .strip_prefix(root().canonicalize().unwrap())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // the files of tests are named `tests.rs` or `*_tests.rs`
        if name.ends_with("tests.rs") {
            continue;
        }
        let text = fs::read_to_string(path).unwrap();
        // what follows the first `#[cfg(test)]` is the tests of the file
        let production = text.split("#[cfg(test)]").next().unwrap_or("");
        let uses = production.lines().any(|l| {
            let l = l.trim_start();
            !l.starts_with("//") && words.iter().any(|w| l.contains(w))
        });
        if !uses {
            continue;
        }
        match allowed.iter().position(|(file, _)| *file == name) {
            Some(i) => used[i] = true,
            None => problems.push(name),
        }
    }
    assert!(problems.is_empty(), "not allowed: {problems:#?}");
    let stale: Vec<&str> = allowed
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|((file, _), _)| *file)
        .collect();
    assert!(
        stale.is_empty(),
        "allowed and not used any more: {stale:#?}"
    );
}
