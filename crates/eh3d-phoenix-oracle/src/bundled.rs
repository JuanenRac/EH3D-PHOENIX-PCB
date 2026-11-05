// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the case files come from: the ones kept in `tests/oracle/` of this repository, built into
//! the program, or the `.json` files of a folder.

use std::fs;
use std::path::Path;

use crate::case::CaseFile;
use crate::codes::{OracleError, INPUT_UNREADABLE};

/// The largest case file that is read, in bytes.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// The case files of `tests/oracle/`, by name, in the order they are run.
pub const BUNDLED: &[(&str, &str)] = &[
    (
        "board.json",
        include_str!("../../../tests/oracle/board.json"),
    ),
    (
        "pending.json",
        include_str!("../../../tests/oracle/pending.json"),
    ),
    ("rf.json", include_str!("../../../tests/oracle/rf.json")),
    ("sim.json", include_str!("../../../tests/oracle/sim.json")),
    (
        "stackup.json",
        include_str!("../../../tests/oracle/stackup.json"),
    ),
];

/// Read the files built into the program.
pub fn bundled_files() -> Result<Vec<CaseFile>, OracleError> {
    BUNDLED
        .iter()
        .map(|(name, text)| {
            CaseFile::from_json(text).map_err(|e| OracleError::new(e.code, format!("{name}: {e}")))
        })
        .collect()
}

/// Read the `.json` files of a folder, in the order of their names. Nothing is written.
pub fn read_dir(dir: &Path) -> Result<Vec<CaseFile>, OracleError> {
    let unreadable = |what: String| OracleError::new(INPUT_UNREADABLE, what);
    let entries = fs::read_dir(dir).map_err(|e| unreadable(format!("{}: {e}", dir.display())))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| unreadable(format!("{}: {e}", dir.display())))?
            .path();
        if path.is_file() && path.extension().is_some_and(|x| x == "json") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut files = Vec::new();
    for path in paths {
        let size = fs::metadata(&path)
            .map_err(|e| unreadable(format!("{}: {e}", path.display())))?
            .len();
        if size > MAX_FILE_BYTES {
            return Err(unreadable(format!(
                "{} is {size} bytes, the limit is {MAX_FILE_BYTES}",
                path.display()
            )));
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| unreadable(format!("{}: {e}", path.display())))?;
        let file = CaseFile::from_json(&text)
            .map_err(|e| OracleError::new(e.code, format!("{}: {e}", path.display())))?;
        files.push(file);
    }
    if files.is_empty() {
        return Err(unreadable(format!(
            "{} holds no .json file of cases",
            dir.display()
        )));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_files_are_valid_and_their_ids_are_unique_across_files() {
        let files = bundled_files().unwrap();
        assert_eq!(files.len(), BUNDLED.len());
        let mut ids: Vec<&str> = files
            .iter()
            .flat_map(|f| f.cases.iter().map(|c| c.id.as_str()))
            .collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
        assert!(total > 40, "{total}");
    }

    #[test]
    fn a_missing_folder_and_an_empty_folder_are_refused() {
        let missing = read_dir(Path::new("no-such-folder-of-cases")).unwrap_err();
        assert_eq!(missing.code, INPUT_UNREADABLE);
        let here = std::env::temp_dir().join("phoenix-oracle-empty-folder-test");
        fs::create_dir_all(&here).unwrap();
        let empty = read_dir(&here).unwrap_err();
        assert_eq!(empty.code, INPUT_UNREADABLE);
        assert!(empty.detail.contains("no .json file"));
        fs::remove_dir(&here).unwrap();
    }
}
