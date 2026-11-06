// SPDX-License-Identifier: GPL-3.0-or-later
//! Writing the output files: a temporary file next to the target, then a rename, so a reader never
//! sees half a file and an old file is never deleted first. On Windows another program may hold
//! the target for a moment, so the rename is tried a bounded number of times.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use eh3d_phoenix_rules::Lang;

use crate::codes::{SanError, OUTPUT_FAILED};
use crate::html::html_report;
use crate::plan::plan_of;
use crate::report::Report;
use crate::settings::SanitizerSettings;

/// Writes `bytes` to `target` atomically.
pub fn write_atomic(target: &Path, bytes: &[u8], s: &SanitizerSettings) -> Result<(), SanError> {
    let fail = |e: std::io::Error| SanError::new(OUTPUT_FAILED, e.to_string());
    let mut temp_name = target
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    temp_name.push(".part");
    let temp = target.with_file_name(temp_name);
    {
        // made exclusively: a link or a file left at the name of the side file is not written to
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(fail)?;
        file.write_all(bytes).map_err(fail)?;
        file.sync_all().map_err(fail)?;
    }
    let mut tries = 0u32;
    loop {
        match fs::rename(&temp, target) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tries += 1;
                if tries > s.write_retries {
                    let _ = fs::remove_file(&temp);
                    return Err(fail(e));
                }
                thread::sleep(Duration::from_millis(s.write_retry_ms));
            }
        }
    }
}

/// The three output paths of a base path: `<base>.html`, `<base>.json` and `<base>.plan.json`.
pub fn output_paths(base: &Path) -> [PathBuf; 3] {
    let with = |suffix: &str| {
        let mut name = base.as_os_str().to_os_string();
        name.push(suffix);
        PathBuf::from(name)
    };
    [with(".html"), with(".json"), with(".plan.json")]
}

/// Writes the HTML report, the JSON report and the plan next to each other. Returns the paths.
pub fn write_outputs(
    report: &Report,
    base: &Path,
    lang: Lang,
    s: &SanitizerSettings,
) -> Result<[PathBuf; 3], SanError> {
    let paths = output_paths(base);
    if let Some(parent) = base.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| SanError::new(OUTPUT_FAILED, e.to_string()))?;
    }
    write_atomic(&paths[0], html_report(report, lang).as_bytes(), s)?;
    write_atomic(&paths[1], report.to_json().as_bytes(), s)?;
    write_atomic(&paths[2], plan_of(report).to_json().as_bytes(), s)?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::analyse_libraries;

    #[test]
    fn a_file_is_written_whole_and_replaces_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.txt");
        let s = SanitizerSettings::default();
        write_atomic(&target, b"first", &s).unwrap();
        write_atomic(&target, b"second", &s).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
        assert!(!dir.path().join("out.txt.part").exists());
    }

    #[test]
    fn a_side_file_that_is_already_there_is_not_written_over() {
        let dir = tempfile::tempdir().unwrap();
        let side = dir.path().join("out.txt.part");
        fs::write(&side, b"not mine").unwrap();
        let target = dir.path().join("out.txt");
        let e = write_atomic(&target, b"x", &SanitizerSettings::default()).unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        assert_eq!(fs::read(&side).unwrap(), b"not mine");
        assert!(!target.exists());
    }

    #[test]
    fn a_target_that_cannot_be_made_gives_the_code() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("missing-folder").join("out.txt");
        let e = write_atomic(&target, b"x", &SanitizerSettings::default()).unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        // a folder where the file should go: the rename fails after the bounded tries
        let blocked = dir.path().join("blocked");
        fs::create_dir(&blocked).unwrap();
        let quick = SanitizerSettings {
            write_retries: 1,
            write_retry_ms: 1,
            ..SanitizerSettings::default()
        };
        let e = write_atomic(&blocked, b"x", &quick).unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        assert!(!dir.path().join("blocked.part").exists());
    }

    #[test]
    fn the_three_files_have_fixed_suffixes() {
        let paths = output_paths(Path::new("out/report"));
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec!["report.html", "report.json", "report.plan.json"]
        );
    }

    #[test]
    fn the_outputs_are_written_and_the_plan_says_it_was_not_applied() {
        let dir = tempfile::tempdir().unwrap();
        let s = SanitizerSettings::default();
        let report = analyse_libraries(&[], Vec::new(), 0, &s);
        let paths = write_outputs(&report, &dir.path().join("r"), Lang::En, &s).unwrap();
        let html = fs::read_to_string(&paths[0]).unwrap();
        assert!(html.starts_with("<!DOCTYPE html>"));
        let plan = fs::read_to_string(&paths[2]).unwrap();
        assert!(plan.contains("\"applied\": false"));
        assert_eq!(fs::read_to_string(&paths[1]).unwrap(), report.to_json());
    }
}
