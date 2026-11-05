// SPDX-License-Identifier: GPL-3.0-or-later
//! The batch inventory: walks a folder of libraries, reads every `.lbr` with the EAGLE reader and
//! the limits of the settings, and keeps what the later steps need. A file that cannot be used is
//! noted with its code and the walk goes on; nothing here panics on a damaged file.

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_eagle::codes as eagle_codes;
use eh3d_phoenix_eagle::document::{read_bytes, DocumentKind};
use eh3d_phoenix_eagle::xml;
use sha2::{Digest, Sha256};

use crate::codes::{
    SanError, DUPLICATE_NAME, FILE_TOO_LARGE, FILE_UNREADABLE, FOLDER_UNUSABLE, ITEM_SKIPPED,
    NOT_A_LIBRARY, READER_WARNINGS,
};
use crate::model::{read_library, LibraryData};
use crate::settings::SanitizerSettings;

/// A note about a library or about the run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Note {
    /// The code, `PHOENIX-SAN-<NNN>`.
    pub code: String,
    /// The library it concerns, relative to the folder, or empty for the run.
    pub library: String,
    /// What is specific to the case.
    pub detail: String,
}

impl Note {
    /// A note with a code, a library and a detail.
    pub fn new(code: &str, library: &str, detail: impl Into<String>) -> Note {
        Note {
            code: code.to_owned(),
            library: library.to_owned(),
            detail: detail.into(),
        }
    }
}

/// One library file after the read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLibrary {
    /// The path relative to the folder, with `/` between the parts.
    pub path: String,
    /// The size in bytes, zero when it could not be read.
    pub bytes: u64,
    /// The SHA-256 of the content, empty when it could not be read.
    pub sha256: String,
    /// The declared EAGLE version, empty when unknown.
    pub version: String,
    /// What the library holds; `None` when the file was left out.
    pub data: Option<LibraryData>,
    /// The notes about this file.
    pub notes: Vec<Note>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The text of the file the way the reader takes it: UTF-8, or Latin-1 when it is not UTF-8.
fn text_of(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(t) => t.to_owned(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The library files under `root`, sorted by path, and the notes about folders that could not be
/// entered. Links are never followed. A bound of the settings that is exceeded stops the run with
/// an error, because a partial inventory would read as a complete one.
pub fn find_libraries(
    root: &Path,
    s: &SanitizerSettings,
) -> Result<(Vec<PathBuf>, Vec<Note>, u64), SanError> {
    let mut found = Vec::new();
    let mut notes = Vec::new();
    let mut skipped_entries = 0u64;
    let mut files = 0u64;
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        if depth > s.limits.max_depth {
            return Err(SanError::new(
                FOLDER_UNUSABLE,
                "folder depth over the limit",
            ));
        }
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if dir == root => {
                return Err(SanError::new(FOLDER_UNUSABLE, e.to_string()));
            }
            Err(e) => {
                notes.push(Note::new(
                    FOLDER_UNUSABLE,
                    &relative(root, &dir),
                    e.to_string(),
                ));
                continue;
            }
        };
        for entry in entries {
            let Ok(entry) = entry else {
                skipped_entries += 1;
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                skipped_entries += 1;
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                pending.push((path, depth + 1));
            } else if kind.is_file() {
                files += 1;
                if files > s.limits.max_files as u64 {
                    return Err(SanError::new(
                        FOLDER_UNUSABLE,
                        "number of files over the limit",
                    ));
                }
                let is_lbr = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("lbr"));
                if is_lbr {
                    found.push(path);
                }
            } else {
                skipped_entries += 1;
            }
        }
    }
    found.sort_by_key(|p| relative(root, p));
    Ok((found, notes, skipped_entries))
}

/// Reads one library file. The result always says what happened.
pub fn load_library(root: &Path, path: &Path, s: &SanitizerSettings) -> LoadedLibrary {
    let rel = relative(root, path);
    let mut out = LoadedLibrary {
        path: rel.clone(),
        bytes: 0,
        sha256: String::new(),
        version: String::new(),
        data: None,
        notes: Vec::new(),
    };
    match fs::metadata(path) {
        Ok(m) => {
            out.bytes = m.len();
            if m.len() > s.limits.max_file_bytes {
                out.notes.push(Note::new(
                    FILE_TOO_LARGE,
                    &rel,
                    format!("{} bytes", m.len()),
                ));
                return out;
            }
        }
        Err(e) => {
            out.notes
                .push(Note::new(FILE_UNREADABLE, &rel, e.to_string()));
            return out;
        }
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            out.notes
                .push(Note::new(FILE_UNREADABLE, &rel, e.to_string()));
            return out;
        }
    };
    out.bytes = bytes.len() as u64;
    out.sha256 = hex(&Sha256::digest(&bytes));
    load_bytes(&mut out, &bytes, s);
    out
}

/// Reads the content of a library file into `out`.
pub fn load_bytes(out: &mut LoadedLibrary, bytes: &[u8], s: &SanitizerSettings) {
    let limits = s.xml_limits();
    let reading = read_bytes(bytes, &limits);
    let rel = out.path.clone();
    let Some(summary) = &reading.summary else {
        let why = reading
            .diagnostics
            .first()
            .and_then(|d| d.code.as_deref().map(str::to_owned))
            .unwrap_or_default();
        out.notes.push(Note::new(NOT_A_LIBRARY, &rel, why));
        return;
    };
    out.version = summary.version.clone();
    if summary.kind != DocumentKind::Library {
        out.notes.push(Note::new(
            NOT_A_LIBRARY,
            &rel,
            format!("it is a {}", summary.kind.name()),
        ));
        return;
    }
    if !reading.is_clean() {
        let mut codes: Vec<String> = reading
            .diagnostics
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .filter(|c| c != eagle_codes::ELEMENT_UNKNOWN)
            .collect();
        codes.sort();
        codes.dedup();
        out.notes
            .push(Note::new(READER_WARNINGS, &rel, codes.join(", ")));
    }
    let root = match xml::parse(&text_of(bytes), &limits) {
        Ok(r) => r,
        Err(e) => {
            out.notes
                .push(Note::new(NOT_A_LIBRARY, &rel, e.to_string()));
            return;
        }
    };
    let Some(library) = root.child("drawing").and_then(|d| d.child("library")) else {
        out.notes
            .push(Note::new(NOT_A_LIBRARY, &rel, "there is no library"));
        return;
    };
    let data = read_library(library);
    if data.skipped > 0 {
        out.notes.push(Note::new(
            ITEM_SKIPPED,
            &rel,
            format!("{} items", data.skipped),
        ));
    }
    if data.duplicate_names > 0 {
        out.notes.push(Note::new(
            DUPLICATE_NAME,
            &rel,
            format!("{} names", data.duplicate_names),
        ));
    }
    out.data = Some(data);
}

/// Walks the folder and reads every library in it.
pub fn scan(
    root: &Path,
    s: &SanitizerSettings,
) -> Result<(Vec<LoadedLibrary>, Vec<Note>, u64), SanError> {
    let (paths, notes, skipped) = find_libraries(root, s)?;
    let libraries = paths.iter().map(|p| load_library(root, p, s)).collect();
    Ok((libraries, notes, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::{library_xml, package_xml};

    fn write(dir: &Path, rel: &str, content: &[u8]) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn a_bad_file_is_noted_and_the_good_ones_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let good = library_xml(
            &[package_xml("R", &[(-0.9, 0.0), (0.9, 0.0)], 1.2, 1.4)],
            &[],
            &[],
        );
        write(dir.path(), "a/good.lbr", good.as_bytes());
        write(dir.path(), "a/b/also.LBR", good.as_bytes());
        write(dir.path(), "broken.lbr", b"<eagle");
        write(dir.path(), "binary.lbr", &[0x10, 0x80, 0, 0]);
        write(
            dir.path(),
            "board.lbr",
            b"<eagle version=\"9.0.0\"><drawing><board/></drawing></eagle>",
        );
        write(dir.path(), "empty.lbr", b"");
        write(dir.path(), "notes.txt", b"not a library");
        let (libs, notes, skipped) = scan(dir.path(), &SanitizerSettings::default()).unwrap();
        assert!(notes.is_empty());
        assert_eq!(skipped, 0);
        let names: Vec<&str> = libs.iter().map(|l| l.path.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "a/b/also.LBR",
                "a/good.lbr",
                "binary.lbr",
                "board.lbr",
                "broken.lbr",
                "empty.lbr"
            ]
        );
        let ok: Vec<&LoadedLibrary> = libs.iter().filter(|l| l.data.is_some()).collect();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[0].sha256, ok[1].sha256);
        assert_eq!(ok[0].data.as_ref().unwrap().packages.len(), 1);
        for bad in libs.iter().filter(|l| l.data.is_none()) {
            assert_eq!(bad.notes.len(), 1, "{}", bad.path);
            assert_eq!(bad.notes[0].code, NOT_A_LIBRARY, "{}", bad.path);
        }
        let board = libs.iter().find(|l| l.path == "board.lbr").unwrap();
        assert!(board.notes[0].detail.contains("board"));
    }

    #[test]
    fn the_limits_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let good = library_xml(&[], &[], &[]);
        write(dir.path(), "a.lbr", good.as_bytes());
        write(dir.path(), "b.lbr", good.as_bytes());
        let small = SanitizerSettings {
            limits: eh3d_phoenix_corpus::InventoryConfig {
                max_file_bytes: 10,
                ..Default::default()
            },
            ..SanitizerSettings::default()
        };
        let (libs, _, _) = scan(dir.path(), &small).unwrap();
        assert!(libs.iter().all(|l| l.data.is_none()));
        assert!(libs.iter().all(|l| l.notes[0].code == FILE_TOO_LARGE));
        let few = SanitizerSettings {
            limits: eh3d_phoenix_corpus::InventoryConfig {
                max_files: 1,
                ..Default::default()
            },
            ..SanitizerSettings::default()
        };
        assert_eq!(scan(dir.path(), &few).unwrap_err().code, FOLDER_UNUSABLE);
        let shallow = SanitizerSettings {
            limits: eh3d_phoenix_corpus::InventoryConfig {
                max_depth: 0,
                ..Default::default()
            },
            ..SanitizerSettings::default()
        };
        write(dir.path(), "x/y.lbr", good.as_bytes());
        assert_eq!(
            scan(dir.path(), &shallow).unwrap_err().code,
            FOLDER_UNUSABLE
        );
        // a folder that does not exist
        let missing = dir.path().join("nope");
        assert_eq!(
            scan(&missing, &SanitizerSettings::default())
                .unwrap_err()
                .code,
            FOLDER_UNUSABLE
        );
    }

    #[test]
    fn problems_inside_a_library_are_noted_without_leaving_it_out() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"<eagle version="9.6.2"><drawing><library><packages>
<package name="A"><smd name="1" x="oops" y="0" dx="1" dy="1" layer="1"/></package>
<package name="A"/>
</packages><devicesets><deviceset name="D"><gates><gate name="G" symbol="NOPE" x="0" y="0"/></gates><devices/></deviceset></devicesets></library></drawing></eagle>"#;
        write(dir.path(), "x.lbr", text.as_bytes());
        let (libs, _, _) = scan(dir.path(), &SanitizerSettings::default()).unwrap();
        let codes: Vec<&str> = libs[0].notes.iter().map(|n| n.code.as_str()).collect();
        assert!(libs[0].data.is_some());
        assert!(codes.contains(&READER_WARNINGS));
        assert!(codes.contains(&ITEM_SKIPPED));
        assert!(codes.contains(&DUPLICATE_NAME));
        assert!(libs[0].notes[0].detail.contains("PHOENIX-EAGLE-007"));
    }
}
