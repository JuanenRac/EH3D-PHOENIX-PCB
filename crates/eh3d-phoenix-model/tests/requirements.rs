// SPDX-License-Identifier: GPL-3.0-or-later
//! The register of requirements (`docs/REQUIREMENTS.md`) is checked against the code: every
//! requirement that is not `planned` names a test that exists, and nothing is `validated` on the
//! evidence of the own tests alone.

use std::fs;
use std::path::{Path, PathBuf};

const STATUSES: [&str; 4] = ["planned", "prototype", "implemented", "validated"];
const EVIDENCE: [&str; 4] = ["simulator", "bench", "field", "none"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Row {
    id: String,
    status: String,
    evidence: String,
    test: String,
}

fn rows() -> Vec<Row> {
    let text = fs::read_to_string(root().join("docs/REQUIREMENTS.md")).unwrap();
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if !line.starts_with("| PPCB-") {
            continue;
        }
        let cells: Vec<&str> = line.split(" | ").map(str::trim).collect();
        assert!(cells.len() >= 8, "a row with too few columns: {line}");
        out.push(Row {
            id: cells[0].trim_start_matches("| ").to_owned(),
            status: cells[2].to_owned(),
            evidence: cells[3].to_owned(),
            test: cells[4].trim_matches('`').to_owned(),
        });
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// True when a function with this name, marked as a test, is in a file of the crate.
fn test_exists(krate: &str, name: &str) -> bool {
    let dir = root().join("crates").join(krate);
    let mut files = Vec::new();
    rust_files(&dir, &mut files);
    let needle = format!("fn {name}(");
    let marker = ["#[", "test]"].concat();
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with(&needle) {
                let from = i.saturating_sub(3);
                if lines[from..i].iter().any(|l| l.trim() == marker) {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn every_requirement_has_a_known_status_and_evidence() {
    let rows = rows();
    assert!(rows.len() >= 31, "{} rows", rows.len());
    for r in &rows {
        assert!(
            STATUSES.contains(&r.status.as_str()),
            "{}: {}",
            r.id,
            r.status
        );
        assert!(
            EVIDENCE.contains(&r.evidence.as_str()),
            "{}: {}",
            r.id,
            r.evidence
        );
        if r.status == "planned" {
            assert_eq!(r.evidence, "none", "{}", r.id);
        } else {
            assert_ne!(r.evidence, "none", "{}", r.id);
        }
        if r.status == "validated" {
            assert!(
                r.evidence == "bench" || r.evidence == "field",
                "{} is validated on evidence that is not a bench or the field",
                r.id
            );
        }
    }
}

#[test]
fn every_requirement_that_is_not_planned_names_a_test_that_exists() {
    for r in rows() {
        if r.status == "planned" {
            continue;
        }
        let (krate, name) = r
            .test
            .split_once("::")
            .unwrap_or_else(|| panic!("{} names no test as crate::name: {:?}", r.id, r.test));
        assert!(
            root().join("crates").join(krate).is_dir(),
            "{}: no crate {krate}",
            r.id
        );
        assert!(
            test_exists(krate, name),
            "{}: no test {name} in {krate}",
            r.id
        );
    }
}

#[test]
fn the_header_counts_the_rows_that_are_planned() {
    let rows = rows();
    let planned = rows.iter().filter(|r| r.status == "planned").count();
    let text = fs::read_to_string(root().join("docs/REQUIREMENTS.md")).unwrap();
    for r in rows.iter().filter(|r| r.status == "planned") {
        assert!(
            text.lines().take(3).any(|l| l.contains(&r.id)),
            "{} is planned and the header does not say so",
            r.id
        );
    }
    assert_eq!(planned, 3, "the header says three requirements are planned");
}
