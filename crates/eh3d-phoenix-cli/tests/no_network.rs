// SPDX-License-Identifier: GPL-3.0-or-later
//! The program opens no network connection and starts no other program, except where this file
//! says so. The test reads the source of every crate of the workspace and fails on a use of the
//! network or of processes that the list below does not allow, and on an allowance that nothing
//! uses any more; it also reads `Cargo.lock` and fails on a library of networking.
//!
//! The words are built from pieces so that this file does not contain them.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What is searched for in the source.
fn words() -> Vec<String> {
    vec![
        concat!("std", "::net").to_owned(),
        concat!("Tcp", "Stream").to_owned(),
        concat!("Tcp", "Listener").to_owned(),
        concat!("Udp", "Socket").to_owned(),
        concat!("Command", "::new").to_owned(),
        concat!("std", "::process").to_owned(),
        concat!("req", "west").to_owned(),
        concat!("u", "req").to_owned(),
        concat!("hyp", "er::").to_owned(),
        concat!("cu", "rl::").to_owned(),
        concat!("lib", "curl").to_owned(),
    ]
}

/// The allowances: the file, the word and why. Nothing else may use the word.
fn allowed() -> Vec<(String, String, &'static str)> {
    let process = concat!("std", "::process").to_owned();
    let command = concat!("Command", "::new").to_owned();
    vec![
        (
            "crates/eh3d-phoenix-route/src/autorouter.rs".to_owned(),
            process.clone(),
            "the adapter of an external autorouter: off by default, needs enabled = true and --execute, starts the program directly with an empty environment and opens no connection",
        ),
        (
            "crates/eh3d-phoenix-route/src/autorouter.rs".to_owned(),
            command,
            "the same adapter: the only place that starts a program",
        ),
        (
            "crates/eh3d-phoenix-cli/src/main.rs".to_owned(),
            process.clone(),
            "ends the program with its exit code",
        ),
        (
            "crates/eh3d-phoenix-cli/src/gui_main.rs".to_owned(),
            process.clone(),
            "the exit code type of the window program",
        ),
        (
            "crates/eh3d-phoenix-project/tests/project.rs".to_owned(),
            process.clone(),
            "the number of the process in the name of a temporary folder of a test",
        ),
        (
            "crates/eh3d-phoenix-sim/src/include.rs".to_owned(),
            process.clone(),
            "the number of the process in the name of a temporary folder of a test",
        ),
        (
            "crates/eh3d-phoenix-sim/src/output.rs".to_owned(),
            process.clone(),
            "the number of the process in the name of a temporary folder of a test",
        ),
        (
            "crates/eh3d-phoenix-sim/src/schgen.rs".to_owned(),
            process,
            "the number of the process in the name of a temporary folder of a test",
        ),
    ]
}

fn files_below(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            files_below(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    let base = root().canonicalize().unwrap();
    path.canonicalize()
        .unwrap()
        .strip_prefix(&base)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn no_source_file_opens_the_network_or_starts_a_program_outside_the_list() {
    let mut files = Vec::new();
    files_below(&root().join("crates"), &mut files);
    assert!(files.len() > 400, "{} files", files.len());
    let words = words();
    let allowed = allowed();
    let mut used: Vec<bool> = vec![false; allowed.len()];
    let mut problems = Vec::new();
    for path in &files {
        let name = relative(path);
        let text = fs::read_to_string(path).unwrap();
        for word in &words {
            if !text.contains(word.as_str()) {
                continue;
            }
            match allowed
                .iter()
                .position(|(file, w, _)| *file == name && w == word)
            {
                Some(i) => used[i] = true,
                None => problems.push(format!("{name}: {word}")),
            }
        }
    }
    assert!(problems.is_empty(), "not allowed: {problems:#?}");
    let stale: Vec<String> = allowed
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|((file, word, _), _)| format!("{file}: {word}"))
        .collect();
    assert!(
        stale.is_empty(),
        "allowed and not used any more: {stale:#?}"
    );
}

#[test]
fn every_allowance_says_why() {
    for (file, word, why) in allowed() {
        assert!(why.len() > 20, "{file}: {word}");
        assert!(root().join(&file).is_file(), "{file}");
    }
}

#[test]
fn the_lock_file_holds_no_library_of_networking() {
    let lock = fs::read_to_string(root().join("Cargo.lock")).unwrap();
    let names: Vec<&str> = lock
        .lines()
        .filter_map(|l| l.strip_prefix("name = \""))
        .filter_map(|l| l.strip_suffix('"'))
        .collect();
    assert!(names.len() > 50, "{} packages", names.len());
    for banned in [
        "req\u{77}est",
        "u\u{72}eq",
        "hyp\u{65}r",
        "cu\u{72}l",
        "curl-sys",
        "tokio",
        "mio",
        "socket2",
        "h2",
        "openssl",
        "openssl-sys",
        "rustls",
        "native-tls",
        "isahc",
        "attohttpc",
        "surf",
    ] {
        // the lock of the window lists the graphics backend that eframe can use instead of glow (wgpu) and the async runtime
        // that backend brings along; the window is built on glow and neither is compiled in
        let optional_backend =
            banned == "tokio" && names.contains(&"eframe") && names.contains(&"wgpu");
        if optional_backend {
            continue;
        }
        assert!(!names.contains(&banned), "{banned} is in Cargo.lock");
    }
}

#[test]
fn the_autorouter_is_off_in_the_settings_that_ship() {
    let text = fs::read_to_string(root().join("config/autorouter.cfg")).unwrap();
    let enabled: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("enabled"))
        .collect();
    assert_eq!(enabled, ["enabled = false"]);
}
