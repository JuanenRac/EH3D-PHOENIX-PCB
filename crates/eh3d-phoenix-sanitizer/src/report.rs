// SPDX-License-Identifier: GPL-3.0-or-later
//! The report: the inventory of the libraries, the groups of duplicates and the notes, as one
//! value that is written as HTML and as JSON. The same libraries and settings always give the
//! same report: no date, no host name, no user name, no program version.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::codes::SanError;
use crate::group::{group_device_sets, group_items, Group};
use crate::scan::{scan, LoadedLibrary, Note};
use crate::settings::{MasterPick, SanitizerSettings};

/// The format tag of the JSON report.
pub const REPORT_FORMAT: &str = "phoenix-sanitize/1";

/// One library in the inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LibraryRow {
    /// The path relative to the folder.
    pub path: String,
    /// The size in bytes.
    pub bytes: u64,
    /// The SHA-256 of the content.
    pub sha256: String,
    /// The declared EAGLE version.
    pub version: String,
    /// `read` or `skipped`.
    pub status: String,
    /// The footprints.
    pub packages: usize,
    /// The symbols.
    pub symbols: usize,
    /// The device sets.
    pub devicesets: usize,
}

/// The settings that shape the result, as they were used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SettingsView {
    /// The lowest score at which two items are grouped.
    pub threshold: u32,
    /// The rounding grid of the hash, in nanometres.
    pub round_nm: i64,
    /// The rounding grid of the angles, in micro-degrees.
    pub angle_round_udeg: i64,
    /// Turns by 90 degrees count as the same shape.
    pub allow_rotation: bool,
    /// A mirror in X counts as the same shape.
    pub allow_mirror: bool,
    /// A shift counts as the same shape.
    pub ignore_origin: bool,
    /// The distance at which the position of a pad stops matching, in nanometres.
    pub tolerance_position_nm: i64,
    /// The difference of size at which a pad stops matching, in nanometres.
    pub tolerance_size_nm: i64,
    /// The distance at which a point of a drawing stops matching, in nanometres.
    pub tolerance_outline_nm: i64,
    /// How the item to keep is chosen.
    pub master_pick: String,
}

/// The counts of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Totals {
    /// The library files found.
    pub files: usize,
    /// The library files read.
    pub read: usize,
    /// The library files left out, each with a note.
    pub skipped: usize,
    /// Entries other than plain files and folders, which are never followed.
    pub skipped_entries: u64,
    /// Footprints, symbols and device sets in the libraries read.
    pub packages: usize,
    /// Symbols in the libraries read.
    pub symbols: usize,
    /// Device sets in the libraries read.
    pub devicesets: usize,
    /// Groups by kind: footprints, symbols, device sets.
    pub groups: [usize; 3],
    /// Items that a replacement would remove, by kind: the members of the groups minus one each.
    pub redundant: [usize; 3],
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The format tag.
    pub format: String,
    /// The settings used.
    pub settings: SettingsView,
    /// The counts.
    pub totals: Totals,
    /// The libraries, sorted by path.
    pub libraries: Vec<LibraryRow>,
    /// Sets of library files with the same content.
    pub identical_files: Vec<Vec<String>>,
    /// The groups of duplicates: footprints, then symbols, then device sets.
    pub groups: Vec<Group>,
    /// The notes, sorted.
    pub notes: Vec<Note>,
}

impl Report {
    /// The report as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// A short text for a terminal.
    pub fn to_text(&self) -> String {
        let t = &self.totals;
        let mut out = format!(
            "libraries: {} found, {} read, {} skipped\nfootprints: {}  symbols: {}  device sets: {}\n",
            t.files, t.read, t.skipped, t.packages, t.symbols, t.devicesets
        );
        out.push_str(&format!(
            "groups of duplicates: footprints {}, symbols {}, device sets {}\nredundant items: footprints {}, symbols {}, device sets {}\n",
            t.groups[0], t.groups[1], t.groups[2], t.redundant[0], t.redundant[1], t.redundant[2]
        ));
        out.push_str(&format!(
            "identical library files: {}\n",
            self.identical_files.len()
        ));
        out.push_str(&format!("threshold: {}\n", self.settings.threshold));
        for g in &self.groups {
            let keep = g
                .keep()
                .map(|m| format!("{}:{}", m.library, m.name))
                .unwrap_or_default();
            out.push_str(&format!(
                "{} {} score {} {} keep {} ({} members)\n",
                g.id,
                g.kind,
                g.score,
                if g.exact { "exact" } else { "close" },
                keep,
                g.members.len()
            ));
        }
        for n in &self.notes {
            out.push_str(&format!("{} {} {}\n", n.code, n.library, n.detail));
        }
        out
    }
}

/// Builds the report of libraries that were read already.
pub fn analyse_libraries(
    libs: &[LoadedLibrary],
    mut notes: Vec<Note>,
    skipped_entries: u64,
    s: &SanitizerSettings,
) -> Report {
    let mut budget = s.max_pairs;
    let packages = group_items(libs, "package", s, &mut budget);
    let symbols = group_items(libs, "symbol", s, &mut budget);
    let sets = group_device_sets(libs, &packages.groups, &symbols.groups, s);

    let mut rows = Vec::with_capacity(libs.len());
    let mut by_sha: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let (mut n_pk, mut n_sy, mut n_ds) = (0usize, 0usize, 0usize);
    for lib in libs {
        notes.extend(lib.notes.iter().cloned());
        let (p, y, d) = match &lib.data {
            Some(data) => (
                data.packages.len(),
                data.symbols.len(),
                data.devicesets.len(),
            ),
            None => (0, 0, 0),
        };
        n_pk += p;
        n_sy += y;
        n_ds += d;
        if !lib.sha256.is_empty() {
            by_sha
                .entry(lib.sha256.as_str())
                .or_default()
                .push(lib.path.as_str());
        }
        let status = if lib.data.is_some() {
            "read"
        } else {
            "skipped"
        };
        rows.push(LibraryRow {
            path: lib.path.clone(),
            bytes: lib.bytes,
            sha256: lib.sha256.clone(),
            version: lib.version.clone(),
            status: status.to_owned(),
            packages: p,
            symbols: y,
            devicesets: d,
        });
    }
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    let mut identical_files: Vec<Vec<String>> = by_sha
        .into_values()
        .filter(|v| v.len() > 1)
        .map(|v| {
            let mut list: Vec<String> = v.into_iter().map(str::to_owned).collect();
            list.sort();
            list
        })
        .collect();
    identical_files.sort();

    notes.extend(packages.notes);
    notes.extend(symbols.notes);
    notes.extend(sets.notes);
    notes.sort_by(|a, b| (&a.code, &a.library, &a.detail).cmp(&(&b.code, &b.library, &b.detail)));
    notes.dedup();

    let groups_of = |g: &[Group]| g.len();
    let redundant_of = |g: &[Group]| g.iter().map(|x| x.members.len() - 1).sum::<usize>();
    let totals = Totals {
        files: libs.len(),
        read: libs.iter().filter(|l| l.data.is_some()).count(),
        skipped: libs.iter().filter(|l| l.data.is_none()).count(),
        skipped_entries,
        packages: n_pk,
        symbols: n_sy,
        devicesets: n_ds,
        groups: [
            groups_of(&packages.groups),
            groups_of(&symbols.groups),
            groups_of(&sets.groups),
        ],
        redundant: [
            redundant_of(&packages.groups),
            redundant_of(&symbols.groups),
            redundant_of(&sets.groups),
        ],
    };
    let mut groups = packages.groups;
    groups.extend(symbols.groups);
    groups.extend(sets.groups);
    let master_pick = match s.master_pick {
        MasterPick::Richest => "richest",
        MasterPick::First => "first",
    };
    Report {
        format: REPORT_FORMAT.to_owned(),
        settings: SettingsView {
            threshold: s.threshold,
            round_nm: s.round_nm,
            angle_round_udeg: s.angle_round_udeg,
            allow_rotation: s.allow_rotation,
            allow_mirror: s.allow_mirror,
            ignore_origin: s.ignore_origin,
            tolerance_position_nm: s.tolerance_position_nm,
            tolerance_size_nm: s.tolerance_size_nm,
            tolerance_outline_nm: s.tolerance_outline_nm,
            master_pick: master_pick.to_owned(),
        },
        totals,
        libraries: rows,
        identical_files,
        groups,
        notes,
    }
}

/// Walks the folder, reads every library and builds the report.
pub fn sanitize(root: &Path, s: &SanitizerSettings) -> Result<Report, SanError> {
    s.validate()?;
    let (libs, notes, skipped_entries) = scan(root, s)?;
    Ok(analyse_libraries(&libs, notes, skipped_entries, s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{FILE_TOO_LARGE, NOT_A_LIBRARY};
    use crate::testdata::{deviceset_xml, library_xml, package_xml, symbol_xml};
    use std::fs;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn corpus(dir: &Path) {
        let pins = [("1", 0.0, 0.0), ("2", 5.08, 0.0)];
        let conn = [("1", "1"), ("2", "2")];
        let two = [(-0.9, 0.0), (0.9, 0.0)];
        write(
            dir,
            "one/passives.lbr",
            &library_xml(
                &[
                    package_xml("R0805", &two, 1.2, 1.4),
                    package_xml("C0603", &[(-0.75, 0.0), (0.75, 0.0)], 0.8, 0.95),
                ],
                &[symbol_xml("R", &pins, 1.0)],
                &[deviceset_xml("R", "R", "R", "R0805", &conn)],
            ),
        );
        write(
            dir,
            "two/old.lbr",
            &library_xml(
                &[package_xml("RES0805", &two, 1.2, 1.4)],
                &[symbol_xml("RES", &pins, 1.0)],
                &[deviceset_xml("RES", "R", "RES", "RES0805", &conn)],
            ),
        );
        write(
            dir,
            "two/copy.lbr",
            &fs::read_to_string(dir.join("two/old.lbr")).unwrap(),
        );
        write(dir, "two/broken.lbr", "<eagle");
    }

    #[test]
    fn a_folder_gives_an_inventory_groups_and_notes() {
        let dir = tempfile::tempdir().unwrap();
        corpus(dir.path());
        let r = sanitize(dir.path(), &SanitizerSettings::default()).unwrap();
        assert_eq!(r.format, REPORT_FORMAT);
        assert_eq!((r.totals.files, r.totals.read, r.totals.skipped), (4, 3, 1));
        assert_eq!(
            (r.totals.packages, r.totals.symbols, r.totals.devicesets),
            (4, 3, 3)
        );
        // the broken file is noted and the walk went on
        assert!(r
            .notes
            .iter()
            .any(|n| n.code == NOT_A_LIBRARY && n.library == "two/broken.lbr"));
        // the two identical library files
        assert_eq!(
            r.identical_files,
            vec![vec!["two/copy.lbr".to_string(), "two/old.lbr".to_string()]]
        );
        // R0805 in three copies, the symbol in three, the device set in three
        assert_eq!(r.totals.groups, [1, 1, 1]);
        assert_eq!(r.totals.redundant, [2, 2, 2]);
        let kinds: Vec<&str> = r.groups.iter().map(|g| g.kind.as_str()).collect();
        assert_eq!(kinds, vec!["package", "symbol", "deviceset"]);
        let ids: Vec<&str> = r.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, vec!["P001", "S001", "D001"]);
        assert!(r.groups.iter().all(|g| g.exact));
        // the libraries are sorted by path
        let paths: Vec<&str> = r.libraries.iter().map(|l| l.path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn the_same_folder_gives_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        corpus(dir.path());
        let s = SanitizerSettings::default();
        let a = sanitize(dir.path(), &s).unwrap();
        let b = sanitize(dir.path(), &s).unwrap();
        assert_eq!(a.to_json(), b.to_json());
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.to_json().ends_with("}\n"));
        let value: serde_json::Value = serde_json::from_str(&a.to_json()).unwrap();
        assert_eq!(value["format"], REPORT_FORMAT);
        assert_eq!(value["settings"]["threshold"], 90);
    }

    #[test]
    fn nothing_in_the_folder_is_changed() {
        let dir = tempfile::tempdir().unwrap();
        corpus(dir.path());
        let snapshot = |root: &Path| -> Vec<(String, Vec<u8>)> {
            let mut out = Vec::new();
            let mut stack = vec![root.to_path_buf()];
            while let Some(d) = stack.pop() {
                for e in fs::read_dir(d).unwrap() {
                    let p = e.unwrap().path();
                    if p.is_dir() {
                        stack.push(p);
                    } else {
                        out.push((p.to_string_lossy().into_owned(), fs::read(&p).unwrap()));
                    }
                }
            }
            out.sort();
            out
        };
        let before = snapshot(dir.path());
        sanitize(dir.path(), &SanitizerSettings::default()).unwrap();
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn an_oversize_file_is_left_out_with_its_code() {
        let dir = tempfile::tempdir().unwrap();
        corpus(dir.path());
        let s = SanitizerSettings {
            limits: eh3d_phoenix_corpus::InventoryConfig {
                max_file_bytes: 300,
                ..Default::default()
            },
            ..SanitizerSettings::default()
        };
        let r = sanitize(dir.path(), &s).unwrap();
        assert!(r.notes.iter().any(|n| n.code == FILE_TOO_LARGE));
        assert_eq!(r.totals.read, 0);
    }

    #[test]
    fn invalid_settings_are_refused_before_the_walk() {
        let dir = tempfile::tempdir().unwrap();
        let s = SanitizerSettings {
            threshold: 101,
            ..SanitizerSettings::default()
        };
        assert!(sanitize(dir.path(), &s).is_err());
    }

    #[test]
    fn the_text_summary_names_the_groups() {
        let dir = tempfile::tempdir().unwrap();
        corpus(dir.path());
        let text = sanitize(dir.path(), &SanitizerSettings::default())
            .unwrap()
            .to_text();
        assert!(text.contains("P001 package score 100 exact keep"));
        assert!(text.contains("groups of duplicates: footprints 1, symbols 1, device sets 1"));
    }
}
