// SPDX-License-Identifier: GPL-3.0-or-later
//! The bulk import of libraries. It walks a folder for EAGLE libraries (`.lbr`) and KiCad files
//! (`.kicad_mod` footprints and `.kicad_sym` symbol libraries), reads each with the readers of
//! the suite, judges every footprint and symbol, finds the ones that appear under the same name
//! or with the same geometry, and makes an index, `phoenix-libindex/1`, with the hash of the
//! content of every item. Nothing is copied: the index holds names, hashes and findings, and the
//! files are only read.
//!
//! The walk is bounded (files, bytes, depth and items) by `config/import_bulk.cfg`. Links are
//! never followed and nothing whose real path is outside the folder is read. The order of
//! everything is that of the sorted paths, so the same folder gives the same index.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use eh3d_phoenix_kicad::{read_footprint, read_symbol_library, sequential_ids, KicadSettings};
use eh3d_phoenix_libedit::model::{Item, Package, Symbol};
use eh3d_phoenix_libedit::read::read_library;
use eh3d_phoenix_libedit::write::sha256_hex;
use eh3d_phoenix_libedit::LibSettings;
use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{
    english, title, IpcError, IDX_DRILL_TOO_BIG, IDX_DUP_GEOMETRY, IDX_DUP_NAME, IDX_FILE_TOO_BIG,
    IDX_FILE_UNREADABLE, IDX_FOLDER_UNUSABLE, IDX_LINK_SKIPPED, IDX_NAME_EMPTY, IDX_NO_CONTENT,
    IDX_NO_ORIGIN, IDX_PAD_DUPLICATE, IDX_PAD_NO_COPPER, IDX_PIN_DUPLICATE, IDX_PIN_NO_NAME,
    IDX_TOO_DEEP, IDX_TOO_MANY_FILES, IDX_TOO_MANY_ITEMS, IDX_WRITE_FAILED,
};
use crate::pads::{from_kicad, from_package, PadGeom};
use crate::settings::BulkSettings;

/// The format of the index.
pub const INDEX_FORMAT: &str = "phoenix-libindex/1";

/// How many names a finding lists before it says how many more there are.
const LISTED: usize = 5;

/// A finding about an item or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, `PHOENIX-LIBIDX-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case.
    pub detail: String,
}

/// A footprint or a symbol of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRecord {
    /// The path of the file, relative to the folder, with `/`.
    pub file: String,
    /// `footprint` or `symbol`.
    pub kind: &'static str,
    /// The name.
    pub name: String,
    /// The hash of the content.
    pub hash: String,
    /// The hash of the geometry of the pads, for a footprint with pads.
    pub geometry: Option<String>,
    /// How many pads or pins it has.
    pub count: usize,
    /// What was found about it.
    pub findings: Vec<Finding>,
}

/// A file that was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    /// The path, relative to the folder, with `/`.
    pub path: String,
    /// `eagle_library`, `kicad_footprint` or `kicad_symbols`.
    pub kind: &'static str,
    /// The size in bytes.
    pub bytes: u64,
    /// The SHA-256 of the file.
    pub sha256: String,
    /// How many items it gave.
    pub items: usize,
    /// Why it could not be read, when it could not.
    pub error: Option<Finding>,
}

/// A path that was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The path, relative to the folder, with `/`.
    pub path: String,
    /// The code that says why.
    pub code: &'static str,
    /// What is specific to the case.
    pub detail: String,
}

/// Items that share a name or a geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// `footprint` or `symbol`.
    pub kind: &'static str,
    /// The name, or the hash of the geometry.
    pub key: String,
    /// The members as `file::name`, sorted.
    pub members: Vec<String>,
}

/// The index of a folder of libraries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Index {
    /// The files that were read.
    pub files: Vec<FileRecord>,
    /// The footprints and symbols, in the order of the files.
    pub items: Vec<ItemRecord>,
    /// The paths that were not read.
    pub skipped: Vec<Skipped>,
    /// The names that more than one item has.
    pub same_name: Vec<Group>,
    /// The geometries that more than one name has.
    pub same_geometry: Vec<Group>,
}

fn finding(code: &'static str, detail: impl Into<String>) -> Finding {
    Finding {
        code,
        detail: detail.into(),
    }
}

/// A list of names for a finding: the first few and how many more.
fn listed(names: &[String]) -> String {
    let shown: Vec<&str> = names.iter().take(LISTED).map(String::as_str).collect();
    if names.len() > LISTED {
        format!("{} and {} more", shown.join(", "), names.len() - LISTED)
    } else {
        shown.join(", ")
    }
}

/// The findings about the pads of a footprint.
fn footprint_findings(name: &str, pads: &[PadGeom], s: &BulkSettings) -> Vec<Finding> {
    let mut out = Vec::new();
    if name.trim().is_empty() {
        out.push(finding(IDX_NAME_EMPTY, "the footprint has no name"));
    }
    if pads.is_empty() {
        out.push(finding(IDX_NO_CONTENT, "the footprint has no pads"));
        return out;
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for p in pads.iter().filter(|p| !p.name.is_empty()) {
        *counts.entry(p.name.as_str()).or_insert(0) += 1;
    }
    let twice: Vec<String> = counts
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(k, _)| (*k).to_owned())
        .collect();
    if !twice.is_empty() {
        out.push(finding(
            IDX_PAD_DUPLICATE,
            format!("{} names used twice: {}", twice.len(), listed(&twice)),
        ));
    }
    let bare: Vec<String> = pads
        .iter()
        .filter(|p| !p.copper)
        .map(|p| p.name.clone())
        .collect();
    if !bare.is_empty() {
        out.push(finding(
            IDX_PAD_NO_COPPER,
            format!("{} pads: {}", bare.len(), listed(&bare)),
        ));
    }
    let wide: Vec<String> = pads
        .iter()
        .filter(|p| p.copper && p.drill > 0 && p.drill > p.w.min(p.h))
        .map(|p| p.name.clone())
        .collect();
    if !wide.is_empty() {
        out.push(finding(
            IDX_DRILL_TOO_BIG,
            format!("{} pads: {}", wide.len(), listed(&wide)),
        ));
    }
    let min_x = pads.iter().map(|p| p.x).min().unwrap_or(0);
    let max_x = pads.iter().map(|p| p.x).max().unwrap_or(0);
    let min_y = pads.iter().map(|p| p.y).min().unwrap_or(0);
    let max_y = pads.iter().map(|p| p.y).max().unwrap_or(0);
    let dx = min_x.max(-max_x).max(0) as f64;
    let dy = min_y.max(-max_y).max(0) as f64;
    let far = (dx * dx + dy * dy).sqrt();
    if far > s.origin_far_nm as f64 {
        out.push(finding(
            IDX_NO_ORIGIN,
            format!(
                "the origin is {} mm from the nearest pad",
                eh3d_phoenix_libedit::write::fmt_mm(far.round() as i64)
            ),
        ));
    }
    out
}

/// The findings about the pins of a symbol, given their names.
fn symbol_findings(name: &str, pins: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    if name.trim().is_empty() {
        out.push(finding(IDX_NAME_EMPTY, "the symbol has no name"));
    }
    if pins.is_empty() {
        out.push(finding(IDX_NO_CONTENT, "the symbol has no pins"));
        return out;
    }
    let empty = pins.iter().filter(|p| p.trim().is_empty()).count();
    if empty > 0 {
        out.push(finding(IDX_PIN_NO_NAME, format!("{empty} pins")));
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for p in pins.iter().filter(|p| !p.trim().is_empty()) {
        *counts.entry(p.as_str()).or_insert(0) += 1;
    }
    let twice: Vec<String> = counts
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(k, _)| (*k).to_owned())
        .collect();
    if !twice.is_empty() {
        out.push(finding(
            IDX_PIN_DUPLICATE,
            format!("{} names used twice: {}", twice.len(), listed(&twice)),
        ));
    }
    out
}

/// The hash of the geometry of the pads: sizes and places rounded to a grid, with the names left
/// out, so that the same land pattern under two names, or in two formats, has the same hash.
fn geometry_hash(pads: &[PadGeom], grid: i64) -> String {
    let round = |v: i64| -> i64 {
        if grid > 0 {
            (v as f64 / grid as f64).round() as i64
        } else {
            v
        }
    };
    let mut lines: Vec<String> = pads
        .iter()
        .map(|p| {
            format!(
                "{} {} {} {} {}",
                round(p.x),
                round(p.y),
                round(p.w),
                round(p.h),
                round(p.drill)
            )
        })
        .collect();
    lines.sort();
    sha256_hex(lines.join("\n").as_bytes())
}

/// The hash of the items of an EAGLE footprint or symbol, in their order.
fn hash_items(kind: &str, name: &str, items: &[Item]) -> String {
    let mut text = format!("{kind}\u{0}{name}\u{0}");
    for item in items {
        text.push_str(&serde_json::to_string(item).unwrap_or_default());
        text.push('\n');
    }
    sha256_hex(text.as_bytes())
}

fn footprint_record(
    file: &str,
    name: &str,
    hash: String,
    pads: &[PadGeom],
    s: &BulkSettings,
) -> ItemRecord {
    ItemRecord {
        file: file.to_owned(),
        kind: "footprint",
        name: name.to_owned(),
        hash,
        geometry: if pads.is_empty() {
            None
        } else {
            Some(geometry_hash(pads, s.geometry_round_nm))
        },
        count: pads.len(),
        findings: footprint_findings(name, pads, s),
    }
}

fn eagle_footprint(file: &str, p: &Package, s: &BulkSettings) -> ItemRecord {
    let pads = from_package(p);
    let hash = hash_items("package", &p.name, &p.items);
    footprint_record(file, &p.name, hash, &pads, s)
}

fn eagle_symbol(file: &str, sym: &Symbol) -> ItemRecord {
    let names: Vec<String> = sym.pins().map(|(_, pin)| pin.name.clone()).collect();
    ItemRecord {
        file: file.to_owned(),
        kind: "symbol",
        name: sym.name.clone(),
        hash: hash_items("symbol", &sym.name, &sym.items),
        geometry: None,
        count: names.len(),
        findings: symbol_findings(&sym.name, &names),
    }
}

/// The kinds of file that are read, by extension.
fn kind_of(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "lbr" => Some("eagle_library"),
        "kicad_mod" => Some("kicad_footprint"),
        "kicad_sym" => Some("kicad_symbols"),
        _ => None,
    }
}

/// The files of a folder that will be read, and what was left out.
struct Walk {
    files: Vec<(String, PathBuf, &'static str)>,
    skipped: Vec<Skipped>,
    too_many: bool,
}

fn walk(root: &Path, dir: &Path, rel: &str, depth: usize, s: &BulkSettings, w: &mut Walk) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            w.skipped.push(Skipped {
                path: rel.to_owned(),
                code: IDX_FILE_UNREADABLE,
                detail: e.to_string(),
            });
            return;
        }
    };
    let mut list: Vec<(String, PathBuf, std::fs::FileType)> = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        list.push((
            entry.file_name().to_string_lossy().into_owned(),
            entry.path(),
            file_type,
        ));
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path, file_type) in list {
        let shown = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if file_type.is_symlink() {
            w.skipped.push(Skipped {
                path: shown,
                code: IDX_LINK_SKIPPED,
                detail: "a link is never followed".to_owned(),
            });
            continue;
        }
        let inside = std::fs::canonicalize(&path).is_ok_and(|real| real.starts_with(root));
        if !inside {
            w.skipped.push(Skipped {
                path: shown,
                code: IDX_LINK_SKIPPED,
                detail: "the real path is outside the folder".to_owned(),
            });
            continue;
        }
        if file_type.is_dir() {
            if depth >= s.max_depth {
                w.skipped.push(Skipped {
                    path: shown,
                    code: IDX_TOO_DEEP,
                    detail: format!("deeper than {} levels", s.max_depth),
                });
            } else {
                walk(root, &path, &shown, depth + 1, s, w);
            }
        } else if file_type.is_file() {
            let Some(kind) = kind_of(&path) else {
                continue;
            };
            if w.files.len() >= s.max_files {
                if !w.too_many {
                    w.too_many = true;
                    w.skipped.push(Skipped {
                        path: shown,
                        code: IDX_TOO_MANY_FILES,
                        detail: format!("more than {} files; the rest are not read", s.max_files),
                    });
                }
                continue;
            }
            w.files.push((shown, path, kind));
        }
    }
}

/// What reading one file gave.
struct Read {
    items: Vec<ItemRecord>,
    error: Option<Finding>,
}

fn read_file(rel: &str, kind: &'static str, bytes: &[u8], s: &BulkSettings) -> Read {
    let unreadable = |what: String| Read {
        items: Vec::new(),
        error: Some(finding(IDX_FILE_UNREADABLE, what)),
    };
    match kind {
        "eagle_library" => match read_library(bytes, &LibSettings::default().limits()) {
            Err(e) => unreadable(e.to_string()),
            Ok(lib) => {
                let mut items: Vec<ItemRecord> = Vec::new();
                for p in &lib.packages {
                    items.push(eagle_footprint(rel, p, s));
                }
                for sym in &lib.symbols {
                    items.push(eagle_symbol(rel, sym));
                }
                Read { items, error: None }
            }
        },
        "kicad_footprint" => {
            let mut ids = match sequential_ids() {
                Ok(ids) => ids,
                Err(e) => return unreadable(e.to_string()),
            };
            match read_footprint(bytes, &KicadSettings::default(), &mut ids) {
                Err(d) => unreadable(d.message),
                Ok(read) => {
                    let pads = from_kicad(&read.footprint);
                    let mut lines: Vec<String> = pads
                        .iter()
                        .map(|p| {
                            format!(
                                "{} {} {} {} {} {} {}",
                                p.name, p.x, p.y, p.w, p.h, p.drill, p.copper
                            )
                        })
                        .collect();
                    lines.sort();
                    let text = format!(
                        "footprint\u{0}{}\u{0}{}",
                        read.footprint.name,
                        lines.join("\n")
                    );
                    let item = footprint_record(
                        rel,
                        &read.footprint.name,
                        sha256_hex(text.as_bytes()),
                        &pads,
                        s,
                    );
                    Read {
                        items: vec![item],
                        error: None,
                    }
                }
            }
        }
        _ => {
            let mut ids = match sequential_ids() {
                Ok(ids) => ids,
                Err(e) => return unreadable(e.to_string()),
            };
            match read_symbol_library(bytes, &KicadSettings::default(), &mut ids) {
                Err(d) => unreadable(d.message),
                Ok(read) => {
                    let mut items = Vec::new();
                    for sym in read.symbols.values() {
                        let numbers: Vec<String> =
                            sym.pins.values().map(|p| p.number.clone()).collect();
                        let mut lines: Vec<String> = sym
                            .pins
                            .values()
                            .map(|p| match p.at {
                                Some(at) => {
                                    format!("{} {} {}", p.number, at.x_nm.get(), at.y_nm.get())
                                }
                                None => format!("{} -", p.number),
                            })
                            .collect();
                        lines.sort();
                        let text = format!("symbol\u{0}{}\u{0}{}", sym.name, lines.join("\n"));
                        items.push(ItemRecord {
                            file: rel.to_owned(),
                            kind: "symbol",
                            name: sym.name.clone(),
                            hash: sha256_hex(text.as_bytes()),
                            geometry: None,
                            count: numbers.len(),
                            findings: symbol_findings(&sym.name, &numbers),
                        });
                    }
                    Read { items, error: None }
                }
            }
        }
    }
}

/// The members of a group as `file::name`.
fn member(item: &ItemRecord) -> String {
    format!("{}::{}", item.file, item.name)
}

/// Finds the names and the geometries that appear more than once and adds the finding to the
/// items that share them.
fn find_duplicates(index: &mut Index) {
    let mut by_name: BTreeMap<(&'static str, String), Vec<usize>> = BTreeMap::new();
    let mut by_geometry: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, item) in index.items.iter().enumerate() {
        if !item.name.is_empty() {
            by_name
                .entry((item.kind, item.name.clone()))
                .or_default()
                .push(i);
        }
        if let Some(g) = &item.geometry {
            by_geometry.entry(g.clone()).or_default().push(i);
        }
    }
    let mut same_name = Vec::new();
    let mut notes: Vec<(usize, Finding)> = Vec::new();
    for ((kind, name), members) in &by_name {
        if members.len() < 2 {
            continue;
        }
        let mut names: Vec<String> = members.iter().map(|i| member(&index.items[*i])).collect();
        names.sort();
        for i in members {
            notes.push((
                *i,
                finding(
                    IDX_DUP_NAME,
                    format!("{} items have this name: {}", members.len(), listed(&names)),
                ),
            ));
        }
        same_name.push(Group {
            kind: *kind,
            key: name.clone(),
            members: names,
        });
    }
    let mut same_geometry = Vec::new();
    for (hash, members) in &by_geometry {
        let names: BTreeSet<&str> = members
            .iter()
            .map(|i| index.items[*i].name.as_str())
            .collect();
        if members.len() < 2 || names.len() < 2 {
            continue;
        }
        let mut list: Vec<String> = members.iter().map(|i| member(&index.items[*i])).collect();
        list.sort();
        for i in members {
            notes.push((
                *i,
                finding(
                    IDX_DUP_GEOMETRY,
                    format!(
                        "{} names have this geometry: {}",
                        names.len(),
                        listed(&list)
                    ),
                ),
            ));
        }
        same_geometry.push(Group {
            kind: "footprint",
            key: hash.clone(),
            members: list,
        });
    }
    for (i, f) in notes {
        index.items[i].findings.push(f);
    }
    same_name.sort_by(|a: &Group, b: &Group| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
    same_geometry.sort_by(|a: &Group, b: &Group| a.members.cmp(&b.members));
    index.same_name = same_name;
    index.same_geometry = same_geometry;
}

/// Walks a folder of libraries and makes its index.
pub fn index_folder(folder: &Path, s: &BulkSettings) -> Result<Index, IpcError> {
    let unusable =
        |what: String| IpcError::new(IDX_FOLDER_UNUSABLE, format!("{}: {what}", folder.display()));
    let meta = std::fs::metadata(folder).map_err(|e| unusable(e.to_string()))?;
    if !meta.is_dir() {
        return Err(unusable("not a folder".to_owned()));
    }
    let root = std::fs::canonicalize(folder).map_err(|e| unusable(e.to_string()))?;
    let mut w = Walk {
        files: Vec::new(),
        skipped: Vec::new(),
        too_many: false,
    };
    walk(&root, &root, "", 0, s, &mut w);
    w.files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut index = Index::default();
    let mut full = false;
    for (rel, path, kind) in w.files {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > s.max_file_bytes {
            w.skipped.push(Skipped {
                path: rel,
                code: IDX_FILE_TOO_BIG,
                detail: format!("{size} bytes, the limit is {}", s.max_file_bytes),
            });
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                w.skipped.push(Skipped {
                    path: rel,
                    code: IDX_FILE_UNREADABLE,
                    detail: e.to_string(),
                });
                continue;
            }
        };
        let mut read = read_file(&rel, kind, &bytes, s);
        let room = s.max_items.saturating_sub(index.items.len());
        if read.items.len() > room {
            read.items.truncate(room);
            if !full {
                full = true;
                w.skipped.push(Skipped {
                    path: rel.clone(),
                    code: IDX_TOO_MANY_ITEMS,
                    detail: format!("more than {} items; the rest are not indexed", s.max_items),
                });
            }
        }
        index.files.push(FileRecord {
            path: rel,
            kind,
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            items: read.items.len(),
            error: read.error,
        });
        index.items.extend(read.items);
    }
    w.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    index.skipped = w.skipped;
    find_duplicates(&mut index);
    Ok(index)
}

fn finding_json(f: &Finding) -> Value {
    json!({"code": f.code, "title": english(f.code), "detail": f.detail})
}

fn group_json(g: &Group) -> Value {
    json!({"kind": g.kind, "key": g.key, "members": g.members})
}

impl Index {
    /// The number of items with at least one finding.
    pub fn flagged(&self) -> usize {
        self.items.iter().filter(|i| !i.findings.is_empty()).count()
    }

    /// The number of footprints and of symbols.
    pub fn counts(&self) -> (usize, usize) {
        let footprints = self.items.iter().filter(|i| i.kind == "footprint").count();
        (footprints, self.items.len() - footprints)
    }

    /// The index as data: `phoenix-libindex/1`.
    pub fn to_json(&self) -> Value {
        let (footprints, symbols) = self.counts();
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({
                    "path": f.path,
                    "kind": f.kind,
                    "bytes": f.bytes,
                    "sha256": f.sha256,
                    "items": f.items,
                    "error": f.error.as_ref().map(finding_json),
                })
            })
            .collect();
        let items: Vec<Value> = self
            .items
            .iter()
            .map(|i| {
                json!({
                    "file": i.file,
                    "kind": i.kind,
                    "name": i.name,
                    "hash": i.hash,
                    "geometry": i.geometry,
                    "count": i.count,
                    "findings": i.findings.iter().map(finding_json).collect::<Vec<Value>>(),
                })
            })
            .collect();
        let skipped: Vec<Value> = self
            .skipped
            .iter()
            .map(|s| {
                json!({"path": s.path, "code": s.code, "title": english(s.code), "detail": s.detail})
            })
            .collect();
        json!({
            "format": INDEX_FORMAT,
            "summary": {
                "files": self.files.len(),
                "footprints": footprints,
                "symbols": symbols,
                "flagged": self.flagged(),
                "skipped": self.skipped.len(),
                "same_name": self.same_name.len(),
                "same_geometry": self.same_geometry.len(),
            },
            "files": files,
            "items": items,
            "duplicates": {
                "same_name": self.same_name.iter().map(group_json).collect::<Vec<Value>>(),
                "same_geometry": self.same_geometry.iter().map(group_json).collect::<Vec<Value>>(),
            },
            "skipped": skipped,
        })
    }

    /// The quality report as text in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let (footprints, symbols) = self.counts();
        let mut out = format!(
            "{} files, {} footprints, {} symbols; {} with findings, {} names and {} geometries repeated, {} paths skipped\n",
            self.files.len(),
            footprints,
            symbols,
            self.flagged(),
            self.same_name.len(),
            self.same_geometry.len(),
            self.skipped.len()
        );
        for f in &self.files {
            if let Some(e) = &f.error {
                out.push_str(&format!(
                    "  {}: {}: {}: {}\n",
                    e.code,
                    title(e.code, lang),
                    f.path,
                    e.detail
                ));
            }
        }
        for s in &self.skipped {
            out.push_str(&format!(
                "  {}: {}: {}: {}\n",
                s.code,
                title(s.code, lang),
                s.path,
                s.detail
            ));
        }
        for i in self.items.iter().filter(|i| !i.findings.is_empty()) {
            out.push_str(&format!("{} {} ({})\n", i.kind, i.name, i.file));
            for f in &i.findings {
                out.push_str(&format!(
                    "  {}: {}: {}\n",
                    f.code,
                    title(f.code, lang),
                    f.detail
                ));
            }
        }
        out
    }
}

/// Writes the index as a new file; a file that exists is never replaced.
pub fn write_index(index: &Index, out: &Path) -> Result<usize, IpcError> {
    use std::io::Write;
    let fail =
        |e: std::io::Error| IpcError::new(IDX_WRITE_FAILED, format!("{}: {e}", out.display()));
    let mut text = serde_json::to_string_pretty(&index.to_json())
        .map_err(|e| IpcError::new(IDX_WRITE_FAILED, e.to_string()))?;
    text.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)?;
    file.sync_all().map_err(fail)?;
    Ok(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::{Density, FamilySet};
    use crate::fp::{save_kicad_mod, save_lbr, IpcFootprint, PadKind};
    use crate::generate::{generate, Params};

    fn chip(name: &str) -> IpcFootprint {
        let set = FamilySet::builtin().unwrap();
        let mut fp = generate(&set, "chip", "0603", Density::B, &Params::new()).unwrap();
        fp.name = name.to_owned();
        fp
    }

    const SYMBOLS: &str = r#"(kicad_symbol_lib (version 20211014) (generator kicad_symbol_editor)
  (symbol "R" (pin_numbers hide) (in_bom yes) (on_board yes)
    (symbol "R_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "~") (number "1"))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "~") (number "2"))))
  (symbol "TWICE" (in_bom yes) (on_board yes)
    (symbol "TWICE_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "A") (number "1"))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "B") (number "1"))))
  (symbol "EMPTY" (in_bom yes) (on_board yes)))"#;

    /// A folder with libraries of both programs, some of them with faults.
    fn folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        save_lbr(&[chip("RES_A"), chip("RES_B")], &root.join("one.lbr")).unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        std::fs::create_dir(root.join("sub").join("parts.pretty")).unwrap();
        save_lbr(&[chip("RES_A")], &root.join("sub").join("two.lbr")).unwrap();
        save_kicad_mod(
            &chip("RES_K"),
            &root
                .join("sub")
                .join("parts.pretty")
                .join("RES_K.kicad_mod"),
        )
        .unwrap();
        std::fs::write(root.join("sub").join("symbols.kicad_sym"), SYMBOLS).unwrap();
        std::fs::write(root.join("broken.lbr"), "<eagle>").unwrap();
        std::fs::write(root.join("notes.txt"), "not a library").unwrap();
        dir
    }

    fn find<'a>(index: &'a Index, file: &str, name: &str) -> &'a ItemRecord {
        index
            .items
            .iter()
            .find(|i| i.file == file && i.name == name)
            .unwrap()
    }

    #[test]
    fn the_folder_is_walked_in_order_and_each_file_is_read() {
        let dir = folder();
        let index = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let paths: Vec<&str> = index.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "broken.lbr",
                "one.lbr",
                "sub/parts.pretty/RES_K.kicad_mod",
                "sub/symbols.kicad_sym",
                "sub/two.lbr"
            ]
        );
        assert_eq!(index.counts(), (4, 3));
        let broken = &index.files[0];
        assert_eq!(broken.error.as_ref().unwrap().code, IDX_FILE_UNREADABLE);
        assert_eq!(broken.items, 0);
        assert!(index.files.iter().all(|f| f.sha256.len() == 64));
    }

    #[test]
    fn the_same_name_in_two_libraries_and_the_same_geometry_under_two_names_are_found() {
        let dir = folder();
        let index = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let names: Vec<&str> = index.same_name.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(names, ["RES_A"]);
        assert_eq!(
            index.same_name[0].members,
            ["one.lbr::RES_A", "sub/two.lbr::RES_A"]
        );
        // RES_A, RES_B and RES_K are the same land pattern, in two formats
        assert_eq!(index.same_geometry.len(), 1);
        assert_eq!(index.same_geometry[0].members.len(), 4);
        let a = find(&index, "one.lbr", "RES_A");
        let codes: Vec<&str> = a.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, [IDX_DUP_NAME, IDX_DUP_GEOMETRY]);
        let k = find(&index, "sub/parts.pretty/RES_K.kicad_mod", "RES_K");
        assert_eq!(k.geometry, a.geometry);
        assert_ne!(k.hash, a.hash);
        assert_eq!(
            find(&index, "one.lbr", "RES_A").hash,
            find(&index, "sub/two.lbr", "RES_A").hash
        );
    }

    #[test]
    fn the_symbols_are_judged_by_their_pins() {
        let dir = folder();
        let index = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let file = "sub/symbols.kicad_sym";
        assert!(find(&index, file, "R").findings.is_empty());
        let twice = find(&index, file, "TWICE");
        assert_eq!(twice.findings[0].code, IDX_PIN_DUPLICATE);
        let empty = find(&index, file, "EMPTY");
        assert_eq!(empty.findings[0].code, IDX_NO_CONTENT);
        assert_eq!(find(&index, file, "R").count, 2);
    }

    #[test]
    fn faults_of_a_footprint_are_reported_with_their_codes() {
        let dir = tempfile::tempdir().unwrap();
        let mut bad = chip("BAD");
        bad.pads[1].name = "1".to_owned();
        let mut hole = chip("HOLE");
        hole.pads[0].drill = 2_000_000;
        hole.pads[0].w = 1_000_000;
        hole.pads[0].h = 1_000_000;
        hole.pads[0].shape = PadKind::Round;
        let mut far = chip("FAR");
        for p in &mut far.pads {
            p.x += 50_000_000;
        }
        save_lbr(&[bad, hole, far], &dir.path().join("bad.lbr")).unwrap();
        let index = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let codes = |name: &str| -> Vec<&'static str> {
            find(&index, "bad.lbr", name)
                .findings
                .iter()
                .map(|f| f.code)
                .collect()
        };
        assert_eq!(codes("BAD"), [IDX_PAD_DUPLICATE]);
        assert_eq!(codes("HOLE"), [IDX_DRILL_TOO_BIG]);
        assert_eq!(codes("FAR"), [IDX_NO_ORIGIN]);
        let text = index.to_text(Lang::Pt);
        assert!(
            text.contains("PHOENIX-LIBIDX-001: Pads com o mesmo nome"),
            "{text}"
        );
    }

    #[test]
    fn the_index_is_the_same_every_time_and_has_the_format_and_no_path_of_the_machine() {
        let dir = folder();
        let a = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let b = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let (ja, jb) = (a.to_json().to_string(), b.to_json().to_string());
        assert_eq!(ja, jb);
        assert_eq!(a.to_json()["format"], "phoenix-libindex/1");
        let root = dir.path().to_string_lossy().replace('\\', "/");
        assert!(!ja.replace("\\\\", "/").contains(&root));
        assert_eq!(a.to_json()["summary"]["footprints"], 4);
    }

    #[test]
    fn the_limits_stop_the_walk_and_say_so() {
        let dir = folder();
        let few = BulkSettings {
            max_files: 2,
            ..BulkSettings::default()
        };
        let index = index_folder(dir.path(), &few).unwrap();
        assert_eq!(index.files.len(), 2);
        assert!(index.skipped.iter().any(|s| s.code == IDX_TOO_MANY_FILES));
        let shallow = BulkSettings {
            max_depth: 0,
            ..BulkSettings::default()
        };
        let index = index_folder(dir.path(), &shallow).unwrap();
        assert_eq!(index.files.len(), 2);
        assert!(index.skipped.iter().any(|s| s.code == IDX_TOO_DEEP));
        let small = BulkSettings {
            max_file_bytes: 1024,
            ..BulkSettings::default()
        };
        let index = index_folder(dir.path(), &small).unwrap();
        assert!(index.skipped.iter().any(|s| s.code == IDX_FILE_TOO_BIG));
        let items = BulkSettings {
            max_items: 3,
            ..BulkSettings::default()
        };
        let index = index_folder(dir.path(), &items).unwrap();
        assert_eq!(index.items.len(), 3);
        assert!(index.skipped.iter().any(|s| s.code == IDX_TOO_MANY_ITEMS));
    }

    #[test]
    fn a_missing_folder_or_a_file_is_refused_and_the_index_is_written_once() {
        let dir = folder();
        let e = index_folder(&dir.path().join("nothing"), &BulkSettings::default()).unwrap_err();
        assert_eq!(e.code, IDX_FOLDER_UNUSABLE);
        let e = index_folder(&dir.path().join("one.lbr"), &BulkSettings::default()).unwrap_err();
        assert_eq!(e.code, IDX_FOLDER_UNUSABLE);
        let index = index_folder(dir.path(), &BulkSettings::default()).unwrap();
        let out = dir.path().join("index.json");
        let size = write_index(&index, &out).unwrap();
        assert!(size > 500);
        assert_eq!(
            write_index(&index, &out).unwrap_err().code,
            IDX_WRITE_FAILED
        );
        let text = std::fs::read_to_string(&out).unwrap();
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back, index.to_json());
    }
}
