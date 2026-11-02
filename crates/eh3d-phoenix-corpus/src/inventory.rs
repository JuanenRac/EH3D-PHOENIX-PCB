// SPDX-License-Identifier: GPL-3.0-or-later
//! Walks a folder of design files and counts what is there, in redacted mode.

use crate::config::InventoryConfig;
use crate::error::CorpusError;
use crate::sniff::{sniff, Family};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

/// The kind of design file, from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Extension {
    /// A schematic (`.sch`).
    Sch,
    /// A board (`.brd`).
    Brd,
    /// A library (`.lbr`).
    Lbr,
    /// Any other file; counted but never opened.
    Other,
}

impl Extension {
    fn of(path: &Path) -> Extension {
        let lower = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match lower.as_deref() {
            Some("sch") => Extension::Sch,
            Some("brd") => Extension::Brd,
            Some("lbr") => Extension::Lbr,
            _ => Extension::Other,
        }
    }
}

/// How many design files of one extension have one kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindCount {
    /// The extension.
    pub extension: Extension,
    /// The kind found in the first bytes.
    pub family: Family,
    /// How many files.
    pub count: u64,
}

/// How many `eagle` XML files declare one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionCount {
    /// The declared version (digits and dots only), or `unreadable` when it was missing or not made of digits and dots.
    pub version: String,
    /// How many files.
    pub count: u64,
}

/// The report. It holds counts, kinds, declared versions and one fingerprint, and nothing taken from a name or a file body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Inventory {
    /// Every file seen.
    pub files: u64,
    /// Files by extension.
    pub by_extension: BTreeMap<Extension, u64>,
    /// Design files (`sch`, `brd`, `lbr`) by extension and kind, sorted.
    pub kinds: Vec<KindCount>,
    /// Declared versions of the `eagle` XML files, sorted.
    pub versions: Vec<VersionCount>,
    /// Design files that could not be opened.
    pub unreadable: u64,
    /// Design files above the size limit: counted, not read whole.
    pub oversize: u64,
    /// Links and other entries that are not plain files or folders: never followed.
    pub skipped_entries: u64,
    /// SHA-256 over the sorted SHA-256 of every design file read whole; it changes when any design file changes and says nothing about names. Absent when switched off.
    pub fingerprint: Option<String>,
}

impl Inventory {
    /// The report as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// A short text form for a terminal.
    pub fn to_text(&self) -> String {
        let mut out = format!("files: {}\n", self.files);
        for (extension, count) in &self.by_extension {
            out.push_str(&format!("  {}: {count}\n", extension_name(*extension)));
        }
        for row in &self.kinds {
            out.push_str(&format!(
                "{} {}: {}\n",
                extension_name(row.extension),
                family_name(row.family),
                row.count
            ));
        }
        for row in &self.versions {
            out.push_str(&format!("version {}: {}\n", row.version, row.count));
        }
        out.push_str(&format!(
            "unreadable: {}\noversize: {}\nskipped entries: {}\n",
            self.unreadable, self.oversize, self.skipped_entries
        ));
        if let Some(print) = &self.fingerprint {
            out.push_str(&format!("fingerprint: {print}\n"));
        }
        out
    }
}

fn extension_name(e: Extension) -> &'static str {
    match e {
        Extension::Sch => "sch",
        Extension::Brd => "brd",
        Extension::Lbr => "lbr",
        Extension::Other => "other",
    }
}

fn family_name(f: Family) -> &'static str {
    match f {
        Family::EagleXml => "eagle-xml",
        Family::OtherXml => "other-xml",
        Family::NotXml => "not-xml",
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Count the files under `root`. Links are never followed; a bound of the configuration that is
/// exceeded stops the run with an error, because a partial count would read as a complete one.
pub fn inventory(root: &Path, config: &InventoryConfig) -> Result<Inventory, CorpusError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| CorpusError::Io { path, source }
    };
    let mut files = 0u64;
    let mut by_extension: BTreeMap<Extension, u64> = BTreeMap::new();
    let mut kinds: BTreeMap<(Extension, Family), u64> = BTreeMap::new();
    let mut versions: BTreeMap<String, u64> = BTreeMap::new();
    let (mut unreadable, mut oversize, mut skipped) = (0u64, 0u64, 0u64);
    let mut digests: Vec<[u8; 32]> = Vec::new();

    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        if depth > config.max_depth {
            return Err(CorpusError::LimitExceeded {
                what: "folder depth",
            });
        }
        for item in fs::read_dir(&dir).map_err(io(&dir))? {
            let item = item.map_err(io(&dir))?;
            let kind = item.file_type().map_err(io(&item.path()))?;
            let path = item.path();
            if kind.is_dir() {
                pending.push((path, depth + 1));
                continue;
            }
            if !kind.is_file() {
                skipped += 1;
                continue;
            }
            files += 1;
            if files > config.max_files as u64 {
                return Err(CorpusError::LimitExceeded {
                    what: "number of files",
                });
            }
            let extension = Extension::of(&path);
            *by_extension.entry(extension).or_insert(0) += 1;
            if extension == Extension::Other {
                continue;
            }
            let size = item.metadata().map_err(io(&path))?.len();
            if size > config.max_file_bytes {
                oversize += 1;
                continue;
            }
            let Ok(mut file) = fs::File::open(&path) else {
                unreadable += 1;
                continue;
            };
            let mut hasher = Sha256::new();
            let mut head = Vec::with_capacity(config.sniff_bytes);
            let mut block = vec![0u8; 64 * 1024];
            let mut failed = false;
            loop {
                match file.read(&mut block) {
                    Ok(0) => break,
                    Ok(n) => {
                        if head.len() < config.sniff_bytes {
                            let take = (config.sniff_bytes - head.len()).min(n);
                            head.extend_from_slice(&block[..take]);
                        }
                        hasher.update(&block[..n]);
                    }
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            if failed {
                unreadable += 1;
                continue;
            }
            digests.push(hasher.finalize().into());
            let sniffed = sniff(&head);
            *kinds.entry((extension, sniffed.family)).or_insert(0) += 1;
            if sniffed.family == Family::EagleXml {
                *versions
                    .entry(sniffed.version.unwrap_or_else(|| "unreadable".to_string()))
                    .or_insert(0) += 1;
            }
        }
    }

    let fingerprint = config.fingerprint.then(|| {
        digests.sort_unstable();
        let mut all = Sha256::new();
        for digest in &digests {
            all.update(digest);
        }
        hex(&all.finalize())
    });
    Ok(Inventory {
        files,
        by_extension,
        kinds: kinds
            .into_iter()
            .map(|((extension, family), count)| KindCount {
                extension,
                family,
                count,
            })
            .collect(),
        versions: versions
            .into_iter()
            .map(|(version, count)| VersionCount { version, count })
            .collect(),
        unreadable,
        oversize,
        skipped_entries: skipped,
        fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eagle(version: &str, secret: &str) -> String {
        format!("<?xml version=\"1.0\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n<eagle version=\"{version}\"><drawing><note>{secret}</note></drawing></eagle>")
    }

    fn corpus() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let client = dir.path().join("ClientAcme").join("Revision3");
        fs::create_dir_all(&client).unwrap();
        fs::write(
            client.join("SecretBoard.brd"),
            eagle("9.6.2", "ProprietaryNetName"),
        )
        .unwrap();
        fs::write(
            client.join("SecretBoard.SCH"),
            eagle("9.6.2", "ProprietaryPartValue"),
        )
        .unwrap();
        fs::write(dir.path().join("old.brd"), [0x10u8, 0x80, 0, 0, 1, 2, 3]).unwrap();
        fs::write(dir.path().join("lib6.lbr"), eagle("6.5.0", "x")).unwrap();
        fs::write(dir.path().join("kicad.sch"), "(kicad_sch (version 1))").unwrap();
        fs::write(dir.path().join("notes.txt"), "do not leak").unwrap();
        dir
    }

    #[test]
    fn counts_files_kinds_and_versions() {
        let dir = corpus();
        let report = inventory(dir.path(), &InventoryConfig::default()).unwrap();
        assert_eq!(report.files, 6);
        assert_eq!(report.by_extension[&Extension::Brd], 2);
        assert_eq!(report.by_extension[&Extension::Sch], 2);
        assert_eq!(report.by_extension[&Extension::Lbr], 1);
        assert_eq!(report.by_extension[&Extension::Other], 1);
        let count = |e, f| {
            report
                .kinds
                .iter()
                .find(|k| k.extension == e && k.family == f)
                .map_or(0, |k| k.count)
        };
        assert_eq!(count(Extension::Brd, Family::EagleXml), 1);
        assert_eq!(count(Extension::Brd, Family::NotXml), 1);
        assert_eq!(count(Extension::Sch, Family::EagleXml), 1);
        assert_eq!(count(Extension::Sch, Family::NotXml), 1);
        let versions: Vec<_> = report
            .versions
            .iter()
            .map(|v| (v.version.as_str(), v.count))
            .collect();
        assert_eq!(versions, vec![("6.5.0", 1), ("9.6.2", 2)]);
        assert_eq!(
            (report.unreadable, report.oversize, report.skipped_entries),
            (0, 0, 0)
        );
    }

    #[test]
    fn the_report_contains_nothing_taken_from_the_corpus() {
        let dir = corpus();
        let report = inventory(dir.path(), &InventoryConfig::default()).unwrap();
        let shown = format!("{}{}{:?}", report.to_json(), report.to_text(), report);
        for needle in [
            "ClientAcme",
            "Revision3",
            "SecretBoard",
            "ProprietaryNetName",
            "ProprietaryPartValue",
            "do not leak",
            "notes",
            "kicad_sch",
            "old",
            "lib6",
        ] {
            assert!(!shown.contains(needle), "{needle:?} leaked");
        }
        let root_name = dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(!shown.contains(&root_name), "the path of the corpus leaked");
    }

    #[test]
    fn the_fingerprint_follows_the_content_and_not_the_names_or_the_order() {
        let (a, b) = (corpus(), corpus());
        let config = InventoryConfig::default();
        let first = inventory(a.path(), &config).unwrap().fingerprint.unwrap();
        assert_eq!(
            first,
            inventory(b.path(), &config).unwrap().fingerprint.unwrap()
        );
        // renaming a design file changes nothing
        fs::rename(b.path().join("lib6.lbr"), b.path().join("renamed.lbr")).unwrap();
        assert_eq!(
            first,
            inventory(b.path(), &config).unwrap().fingerprint.unwrap()
        );
        // changing one byte of a design file changes it
        let target = b.path().join("renamed.lbr");
        let mut text = fs::read_to_string(&target).unwrap();
        text.push(' ');
        fs::write(&target, text).unwrap();
        assert_ne!(
            first,
            inventory(b.path(), &config).unwrap().fingerprint.unwrap()
        );
        // a file that is not a design file does not count
        fs::write(b.path().join("more.txt"), "x").unwrap();
        let off = InventoryConfig {
            fingerprint: false,
            ..InventoryConfig::default()
        };
        assert_eq!(inventory(b.path(), &off).unwrap().fingerprint, None);
    }

    #[test]
    fn bounds_stop_the_run_instead_of_giving_a_partial_count() {
        let dir = corpus();
        let few = InventoryConfig {
            max_files: 3,
            ..InventoryConfig::default()
        };
        assert!(matches!(
            inventory(dir.path(), &few),
            Err(CorpusError::LimitExceeded {
                what: "number of files"
            })
        ));
        let shallow = InventoryConfig {
            max_depth: 1,
            ..InventoryConfig::default()
        };
        assert!(matches!(
            inventory(dir.path(), &shallow),
            Err(CorpusError::LimitExceeded {
                what: "folder depth"
            })
        ));
    }

    #[test]
    fn a_large_design_file_is_counted_as_oversize_and_not_read() {
        let dir = corpus();
        let small = InventoryConfig {
            max_file_bytes: 60,
            ..InventoryConfig::default()
        };
        let report = inventory(dir.path(), &small).unwrap();
        assert!(report.oversize >= 2);
        assert_eq!(
            report.kinds.iter().map(|k| k.count).sum::<u64>() + report.oversize,
            5
        );
    }

    #[test]
    fn a_missing_folder_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            inventory(&dir.path().join("absent"), &InventoryConfig::default()),
            Err(CorpusError::Io { .. })
        ));
    }

    #[test]
    fn an_empty_corpus_is_reported_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let report = inventory(dir.path(), &InventoryConfig::default()).unwrap();
        assert_eq!(report.files, 0);
        assert!(report.kinds.is_empty() && report.versions.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_skipped_and_counted_never_followed() {
        let dir = corpus();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("elsewhere.brd"), eagle("1.0", "x")).unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        let report = inventory(dir.path(), &InventoryConfig::default()).unwrap();
        assert_eq!(report.files, 6);
        assert_eq!(report.skipped_entries, 1);
    }
}
