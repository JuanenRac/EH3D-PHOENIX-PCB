//! Working on a corpus of EAGLE files without touching the originals.
//!
//! [`snapshot`] copies the files of a folder to another folder: the originals are only opened for
//! reading, each copy is checked by SHA-256 against the original, and the original is hashed again
//! after the copy to prove it did not change. The copies get neutral names (`000001.brd`), so the
//! report never carries a file or folder name. [`measure`] reads the copies and returns a
//! [`ReadRate`] that holds only counts, versions and diagnostic codes.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use eh3d_diagnostic::{Diagnostic, Severity};
use sha2::{Digest, Sha256};

use crate::codes;
use crate::document::{read_bytes, DocumentKind};
use crate::xml::{self, Limits};

/// One file that was copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRecord {
    /// Where the copy is.
    pub copy: PathBuf,
    /// The SHA-256 of the content, in lower-case hexadecimal (the same for the original and the copy).
    pub sha256: String,
    /// The size in bytes.
    pub size: u64,
}

/// The SHA-256 of a file, read in blocks.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut block = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        hasher.update(&block[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| extensions.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn walk(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            walk(&path, extensions, out)?;
        } else if meta.is_file() && has_extension(&path, extensions) {
            out.push(path);
        }
    }
    Ok(())
}

/// The canonical path a folder will have once created: the nearest existing ancestor made
/// canonical, followed by the names that do not exist yet.
fn resolve_future(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = absolute.as_path();
    while !probe.exists() {
        match (probe.file_name(), probe.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_owned());
                probe = parent;
            }
            _ => break,
        }
    }
    let mut resolved = fs::canonicalize(probe)?;
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

/// Copies every file of `source` with one of `extensions` into `target`, checked by SHA-256.
///
/// `target` must not be inside `source`. A file whose copy or original does not match its hash is
/// reported with `PHOENIX-EAGLE-011` and left out of the records.
pub fn snapshot(
    source: &Path,
    target: &Path,
    extensions: &[&str],
) -> io::Result<(Vec<CopyRecord>, Vec<Diagnostic>)> {
    let source_abs = fs::canonicalize(source)?;
    // decided before anything is created, so a refused call leaves the originals' folder untouched
    if resolve_future(target)?.starts_with(&source_abs) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the folder for the copies must be outside the folder of the originals",
        ));
    }
    fs::create_dir_all(target)?;
    let target_abs = fs::canonicalize(target)?;
    let mut files = Vec::new();
    walk(&source_abs, extensions, &mut files)?;
    let mut records = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, original) in files.iter().enumerate() {
        let ext = original
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_ascii_lowercase();
        let copy = target_abs.join(format!("{:06}.{ext}", index + 1));
        match copy_checked(original, &copy) {
            Ok(rec) => records.push(rec),
            Err(why) => diagnostics.push(codes::diagnostic(
                codes::COPY_MISMATCH,
                Severity::Error,
                &format!("file {}: {why}", index + 1),
            )),
        }
    }
    Ok((records, diagnostics))
}

fn copy_checked(original: &Path, copy: &Path) -> Result<CopyRecord, String> {
    let before = sha256_file(original).map_err(|e| format!("cannot read the original: {e}"))?;
    let size = fs::copy(original, copy).map_err(|e| format!("cannot copy: {e}"))?;
    let copied = sha256_file(copy).map_err(|e| format!("cannot read the copy: {e}"))?;
    let after =
        sha256_file(original).map_err(|e| format!("cannot read the original again: {e}"))?;
    if before != copied {
        return Err("the copy differs from the original".to_owned());
    }
    if before != after {
        return Err("the original changed while it was copied".to_owned());
    }
    Ok(CopyRecord {
        copy: copy.to_path_buf(),
        sha256: before,
        size,
    })
}

/// The measured read rate of a set of copies. It holds counts, versions and codes only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadRate {
    /// Files read.
    pub total: usize,
    /// Files with a summary and nothing but information.
    pub clean: usize,
    /// Files with a summary and at least one warning.
    pub with_warnings: usize,
    /// Files without a summary.
    pub failed: usize,
    /// Files by kind (`library`, `board`, `schematic`).
    pub by_kind: BTreeMap<String, usize>,
    /// Files by declared version.
    pub by_version: BTreeMap<String, usize>,
    /// How many files raised each code.
    pub by_code: BTreeMap<String, usize>,
    /// Files whose XML tree came back identical after being written and read again.
    pub round_trip_ok: usize,
    /// Files whose tree did not come back identical (always a defect of the writer or the reader).
    pub round_trip_failed: usize,
}

impl ReadRate {
    /// The report as text: counts, versions and codes only, never a name or a value.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "files: {}
read: {} of {} ({} per thousand)
  clean: {}
  with warnings: {}
not read: {}
round trip of the XML tree: {} identical, {} different
",
            self.total,
            self.total - self.failed,
            self.total,
            self.read_per_thousand(),
            self.clean,
            self.with_warnings,
            self.failed,
            self.round_trip_ok,
            self.round_trip_failed
        ));
        for (title, map) in [
            ("kind", &self.by_kind),
            ("version", &self.by_version),
            ("code", &self.by_code),
        ] {
            for (key, n) in map {
                out.push_str(&format!(
                    "{title} {key}: {n}
"
                ));
            }
        }
        out
    }

    /// The share of files that gave a summary, in parts per thousand (0 when there are no files).
    pub fn read_per_thousand(&self) -> u32 {
        if self.total == 0 {
            return 0;
        }
        (((self.total - self.failed) as u64 * 1000) / self.total as u64) as u32
    }
}

/// Writes the tree of a UTF-8 file and reads it again: `Some(true)` when the tree is identical,
/// `None` when the file is not UTF-8 or not XML (the read already reports that).
fn round_trip(bytes: &[u8], limits: &Limits) -> Option<bool> {
    let text = std::str::from_utf8(bytes).ok()?;
    let tree = xml::parse(text, limits).ok()?;
    let written = xml::write_document(&tree);
    Some(xml::parse(&written, limits).ok()? == tree)
}

/// Reads every copy and returns the rate. Reading never writes anywhere.
pub fn measure(records: &[CopyRecord], limits: &Limits) -> ReadRate {
    let mut rate = ReadRate::default();
    for rec in records {
        rate.total += 1;
        let bytes = match fs::read(&rec.copy) {
            Ok(b) => b,
            Err(_) => {
                rate.failed += 1;
                *rate.by_code.entry(codes::IO_FAILED.to_owned()).or_insert(0) += 1;
                continue;
            }
        };
        let reading = read_bytes(&bytes, limits);
        let mut seen = std::collections::BTreeSet::new();
        for d in &reading.diagnostics {
            if let Some(code) = d.code.as_deref() {
                seen.insert(code.to_owned());
            }
        }
        for code in seen {
            *rate.by_code.entry(code).or_insert(0) += 1;
        }
        match &reading.summary {
            None => rate.failed += 1,
            Some(s) => {
                let kind: DocumentKind = s.kind;
                *rate.by_kind.entry(kind.name().to_owned()).or_insert(0) += 1;
                *rate.by_version.entry(s.version.clone()).or_insert(0) += 1;
                match round_trip(&bytes, limits) {
                    Some(true) => rate.round_trip_ok += 1,
                    Some(false) => rate.round_trip_failed += 1,
                    None => {}
                }
                if reading.is_clean() {
                    rate.clean += 1;
                } else {
                    rate.with_warnings += 1;
                }
            }
        }
    }
    rate
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB: &str = "<eagle version=\"6.5.0\"><drawing><library><packages><package name=\"A\"><pad name=\"1\"/></package></packages></library></drawing></eagle>";
    const BRD: &str =
        "<eagle version=\"9.6.2\"><drawing><board><elements/><signals/></board></drawing></eagle>";

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("originals");
        let dst = dir.path().join("copies");
        fs::create_dir_all(src.join("deep/er")).unwrap();
        fs::write(src.join("a.lbr"), LIB).unwrap();
        fs::write(src.join("deep/b.BRD"), BRD).unwrap();
        fs::write(src.join("deep/er/c.sch"), "not xml at all").unwrap();
        fs::write(src.join("deep/notes.txt"), "ignored").unwrap();
        (dir, src, dst)
    }

    #[test]
    fn copies_are_checked_and_originals_stay_the_same() {
        let (_dir, src, dst) = setup();
        let before: Vec<String> = ["a.lbr", "deep/b.BRD", "deep/er/c.sch", "deep/notes.txt"]
            .iter()
            .map(|f| sha256_file(&src.join(f)).unwrap())
            .collect();
        let (records, diags) = snapshot(&src, &dst, &["lbr", "brd", "sch"]).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(records.len(), 3);
        for rec in &records {
            assert_eq!(sha256_file(&rec.copy).unwrap(), rec.sha256);
            let name = rec.copy.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with("00000"), "neutral names: {name}");
        }
        let after: Vec<String> = ["a.lbr", "deep/b.BRD", "deep/er/c.sch", "deep/notes.txt"]
            .iter()
            .map(|f| sha256_file(&src.join(f)).unwrap())
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn the_copies_cannot_go_inside_the_originals() {
        let (_dir, src, _dst) = setup();
        let inside = src.join("copies");
        assert!(snapshot(&src, &inside, &["lbr"]).is_err());
    }

    #[test]
    fn the_rate_holds_only_counts_and_codes() {
        let (_dir, src, dst) = setup();
        let (records, _) = snapshot(&src, &dst, &["lbr", "brd", "sch"]).unwrap();
        let rate = measure(&records, &Limits::default());
        assert_eq!(rate.total, 3);
        assert_eq!(rate.failed, 1);
        assert_eq!(rate.clean, 2);
        assert_eq!((rate.round_trip_ok, rate.round_trip_failed), (2, 0));
        assert_eq!(rate.read_per_thousand(), 666);
        assert_eq!(rate.by_kind.get("library"), Some(&1));
        assert_eq!(rate.by_kind.get("board"), Some(&1));
        assert_eq!(rate.by_version.get("9.6.2"), Some(&1));
        assert_eq!(rate.by_code.get(codes::NOT_XML), Some(&1));
        let text = format!("{rate:?}");
        assert!(!text.contains("originals") && !text.contains("deep"));
    }

    #[test]
    fn an_empty_set_has_a_zero_rate() {
        assert_eq!(ReadRate::default().read_per_thousand(), 0);
    }
}
