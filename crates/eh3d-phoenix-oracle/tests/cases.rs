// SPDX-License-Identifier: GPL-3.0-or-later
//! The reference cases kept in `tests/oracle/`: every cited case must pass, every pending case
//! must say why, and the files read from the folder must be the ones built into the program.

use std::path::PathBuf;

use eh3d_phoenix_oracle::{bundled_files, read_dir, run_files, Status, Verdict, BUNDLED, CHECKS};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/oracle")
}

#[test]
fn every_cited_case_passes_within_its_tolerance() {
    let files = bundled_files().unwrap();
    let report = run_files(&files, None);
    let bad: Vec<String> = report
        .results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Outside | Verdict::Failed))
        .map(|r| {
            format!(
                "{}: {:?} actual {:?} expected {:?} allowed {:?} {}",
                r.id, r.verdict, r.actual, r.expected, r.allowed, r.detail
            )
        })
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    let counts = report.counts();
    assert!(counts.ok >= 40, "{counts:?}");
    assert!(counts.pending >= 10, "{counts:?}");
}

#[test]
fn the_folder_and_the_built_in_files_are_the_same_cases() {
    let from_folder = read_dir(&folder()).unwrap();
    let built_in = bundled_files().unwrap();
    assert_eq!(from_folder.len(), BUNDLED.len());
    assert_eq!(from_folder, built_in);
}

#[test]
fn a_pending_case_has_a_reason_and_no_number_and_a_cited_one_has_a_source() {
    for file in bundled_files().unwrap() {
        for case in &file.cases {
            assert!(!case.source.trim().is_empty(), "{}", case.id);
            match case.status {
                Status::Pending => {
                    assert!(!case.reason.trim().is_empty(), "{}", case.id);
                    assert!(case.expected.is_none(), "{}", case.id);
                }
                Status::Cited => {
                    assert!(CHECKS.contains(&case.check.as_str()), "{}", case.id);
                    assert!(case.tolerance.is_some(), "{}", case.id);
                }
            }
        }
    }
}

#[test]
fn the_report_of_the_built_in_cases_is_the_same_every_time() {
    let files = bundled_files().unwrap();
    let first = run_files(&files, None).to_json();
    let second = run_files(&files, None).to_json();
    assert_eq!(first, second);
    assert!(first.contains("ORACLE-STACKUP-001"));
}

#[test]
fn a_wrong_tolerance_would_be_caught() {
    // the same case with a reference value that is not the published one must fall outside
    let mut files = bundled_files().unwrap();
    let file = files
        .iter_mut()
        .find(|f| f.suite == "stackup")
        .expect("the stack-up suite");
    for case in &mut file.cases {
        if case.id == "ORACLE-STACKUP-001" {
            case.expected = Some(300_000_000.0);
        }
    }
    let report = run_files(&files, Some("stackup"));
    let first = &report.results[0];
    assert_eq!(first.id, "ORACLE-STACKUP-001");
    assert_eq!(first.verdict, Verdict::Outside);
    assert!(report.has_problems());
}
