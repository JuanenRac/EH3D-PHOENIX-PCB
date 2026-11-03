//! The contract for pairs of an EAGLE binary file and the XML that EAGLE saved from it.
//!
//! The reader of the binary files of the old EAGLE versions is not written here and is not guessed
//! at: it will be built against pairs that the owner of the files provides, each pair being one
//! binary file and the XML file that EAGLE 9 saves when it opens that same binary file. This module
//! fixes how the pairs are asked for, where they are compared and which codes say what is wrong, so
//! that the reader can be plugged in without changing anything else.
//!
//! **How a pair looks.** In one folder (subfolders are allowed) each pair is two files:
//! `NAME.brd` (or `.sch`, `.lbr`) and `NAME.brd.xml`, the binary file and the XML saved from it. The
//! folder holds a manifest named `PAIRS.sha256` with one line per file, `<sha256>  <relative path>`
//! (two spaces, forward slashes), so that a changed file is noticed. The files are copies; the
//! originals are never touched.
//!
//! **Where they are compared.** [`check_pairs`] reads the XML side with this crate, reads the binary
//! side with a [`BinaryReader`] given by the caller and compares the two [`DocumentSummary`] values
//! field by field ([`compare_summaries`]). Without a binary reader the pairs are only verified
//! (complete, hashes right, XML side readable) and the check says that nothing was compared.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eh3d_diagnostic::{Diagnostic, Severity};

use crate::codes;
use crate::corpus::sha256_file;
use crate::document::{read_bytes, DocumentSummary};
use crate::xml::Limits;

/// The name of the manifest in the folder of the pairs.
pub const MANIFEST: &str = "PAIRS.sha256";
/// The extensions of the binary side of a pair.
pub const BINARY_EXTENSIONS: &[&str] = &["brd", "sch", "lbr"];

/// One pair: the binary file and the XML saved from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// The path of the binary file relative to the folder, with forward slashes.
    pub name: String,
    /// The binary file.
    pub binary: PathBuf,
    /// The XML file.
    pub xml: PathBuf,
}

/// The pairs of a folder and what was wrong with the files around them.
#[derive(Debug, Clone, Default)]
pub struct PairSet {
    /// The complete pairs, in order of name.
    pub pairs: Vec<Pair>,
    /// Files without their other half, and files that cannot be listed.
    pub diagnostics: Vec<Diagnostic>,
}

/// What a binary reader gives for the binary side of a pair.
pub trait BinaryReader {
    /// Reads the bytes of an EAGLE binary file into the same summary the XML reader gives, or says
    /// why it cannot with a diagnostic.
    fn read(&self, bytes: &[u8]) -> Result<DocumentSummary, Diagnostic>;
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            walk(&path, out)?;
        } else if meta.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Finds the pairs in a folder.
pub fn find_pairs(dir: &Path) -> PairSet {
    let mut set = PairSet::default();
    let mut files = Vec::new();
    if let Err(e) = walk(dir, &mut files) {
        set.diagnostics.push(codes::diagnostic(
            codes::IO_FAILED,
            Severity::Error,
            &format!("the folder of the pairs: {}", e.kind()),
        ));
        return set;
    }
    let names: BTreeMap<String, PathBuf> = files
        .into_iter()
        .map(|p| (relative(dir, &p), p))
        .filter(|(name, _)| name != MANIFEST)
        .collect();
    let is_binary = |name: &str| {
        let lower = name.to_ascii_lowercase();
        BINARY_EXTENSIONS
            .iter()
            .any(|e| lower.ends_with(&format!(".{e}")))
    };
    for (name, path) in &names {
        if is_binary(name) {
            let xml_name = format!("{name}.xml");
            match names.get(&xml_name) {
                Some(xml) => set.pairs.push(Pair {
                    name: name.clone(),
                    binary: path.clone(),
                    xml: xml.clone(),
                }),
                None => set.diagnostics.push(codes::diagnostic(
                    codes::PAIR_INCOMPLETE,
                    Severity::Error,
                    &format!("{name} has no {xml_name}"),
                )),
            }
        } else if let Some(stem) = name.strip_suffix(".xml") {
            if !(is_binary(stem) && names.contains_key(stem)) {
                set.diagnostics.push(codes::diagnostic(
                    codes::PAIR_INCOMPLETE,
                    Severity::Error,
                    &format!("{name} has no binary file"),
                ));
            }
        }
    }
    set
}

/// Reads the manifest: relative path to SHA-256. Lines that do not have the form are reported.
pub fn read_manifest(text: &str) -> (BTreeMap<String, String>, Vec<Diagnostic>) {
    let mut map = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once("  ") {
            Some((hash, path))
                if hash.len() == 64
                    && hash.bytes().all(|b| b.is_ascii_hexdigit())
                    && !path.is_empty() =>
            {
                map.insert(path.to_owned(), hash.to_ascii_lowercase());
            }
            _ => diagnostics.push(codes::diagnostic(
                codes::PAIR_HASH,
                Severity::Error,
                &format!("manifest line {} is not `<sha256>  <path>`", index + 1),
            )),
        }
    }
    (map, diagnostics)
}

/// Compares two summaries of the same file, field by field, one diagnostic per difference.
pub fn compare_summaries(
    from_xml: &DocumentSummary,
    from_binary: &DocumentSummary,
    name: &str,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut differ = |what: &str, a: String, b: String| {
        if a != b {
            out.push(codes::diagnostic(
                codes::PAIR_DIFFERS,
                Severity::Error,
                &format!("{name}: {what}: {a} in the XML, {b} in the binary file"),
            ));
        }
    };
    differ(
        "kind",
        from_xml.kind.name().to_owned(),
        from_binary.kind.name().to_owned(),
    );
    differ(
        "layers",
        from_xml.layers.to_string(),
        from_binary.layers.to_string(),
    );
    differ(
        "parts",
        from_xml.parts.to_string(),
        from_binary.parts.to_string(),
    );
    differ(
        "nets",
        from_xml.nets.to_string(),
        from_binary.nets.to_string(),
    );
    differ(
        "routed items",
        from_xml.routed_items.to_string(),
        from_binary.routed_items.to_string(),
    );
    differ(
        "sheets",
        from_xml.sheets.to_string(),
        from_binary.sheets.to_string(),
    );
    differ(
        "embedded libraries",
        from_xml.embedded_libraries.to_string(),
        from_binary.embedded_libraries.to_string(),
    );
    differ(
        "packages",
        format!("{:?}", from_xml.packages),
        format!("{:?}", from_binary.packages),
    );
    differ(
        "symbols",
        format!("{:?}", from_xml.symbols),
        format!("{:?}", from_binary.symbols),
    );
    differ(
        "device sets",
        format!("{:?}", from_xml.device_sets),
        format!("{:?}", from_binary.device_sets),
    );
    out
}

/// The outcome of checking a folder of pairs.
#[derive(Debug, Clone, Default)]
pub struct PairReport {
    /// How many complete pairs there are.
    pub pairs: usize,
    /// How many were compared by a binary reader and agreed.
    pub agreed: usize,
    /// How many were compared and differ or could not be read.
    pub failed: usize,
    /// True when no binary reader was given, so nothing was compared.
    pub not_compared: bool,
    /// Every diagnostic, in order.
    pub diagnostics: Vec<Diagnostic>,
}

impl PairReport {
    /// True when every pair was compared and agreed and nothing else is wrong.
    pub fn all_agree(&self) -> bool {
        !self.not_compared
            && self.pairs > 0
            && self.agreed == self.pairs
            && self
                .diagnostics
                .iter()
                .all(|d| d.severity != Severity::Error)
    }
}

/// Checks the pairs of a folder: completeness, the hashes of the manifest, the XML side, and, when
/// a binary reader is given, the agreement of the two sides.
pub fn check_pairs(dir: &Path, reader: Option<&dyn BinaryReader>, limits: &Limits) -> PairReport {
    let set = find_pairs(dir);
    let mut report = PairReport {
        pairs: set.pairs.len(),
        not_compared: reader.is_none(),
        diagnostics: set.diagnostics,
        ..PairReport::default()
    };
    let manifest = match fs::read_to_string(dir.join(MANIFEST)) {
        Ok(text) => {
            let (map, mut bad) = read_manifest(&text);
            report.diagnostics.append(&mut bad);
            Some(map)
        }
        Err(_) => {
            report.diagnostics.push(codes::diagnostic(
                codes::PAIR_HASH,
                Severity::Error,
                &format!("the folder has no {MANIFEST}"),
            ));
            None
        }
    };
    for pair in &set.pairs {
        let mut ok = true;
        let xml_name = format!("{}.xml", pair.name);
        for (name, path) in [(&pair.name, &pair.binary), (&xml_name, &pair.xml)] {
            let Some(manifest) = &manifest else { break };
            let expected = manifest.get(name.as_str());
            let actual = sha256_file(path).ok();
            if expected.is_none() || expected != actual.as_ref() {
                ok = false;
                report.diagnostics.push(codes::diagnostic(
                    codes::PAIR_HASH,
                    Severity::Error,
                    &format!(
                        "{name}: {}",
                        if expected.is_none() {
                            "not in the manifest"
                        } else {
                            "the hash differs from the manifest"
                        }
                    ),
                ));
            }
        }
        let (Ok(binary), Ok(xml_bytes)) = (fs::read(&pair.binary), fs::read(&pair.xml)) else {
            report.diagnostics.push(codes::diagnostic(
                codes::IO_FAILED,
                Severity::Error,
                &pair.name,
            ));
            report.failed += 1;
            continue;
        };
        if binary.first() != Some(&0x10) {
            ok = false;
            report.diagnostics.push(codes::diagnostic(
                codes::PAIR_SIDE,
                Severity::Error,
                &format!("{} does not start like a binary file", pair.name),
            ));
        }
        let reading = read_bytes(&xml_bytes, limits);
        let Some(from_xml) = reading.summary else {
            report.diagnostics.push(codes::diagnostic(
                codes::PAIR_SIDE,
                Severity::Error,
                &format!("{xml_name} is not an EAGLE XML file"),
            ));
            report.failed += 1;
            continue;
        };
        let Some(reader) = reader else {
            continue;
        };
        match reader.read(&binary) {
            Ok(from_binary) => {
                let differences = compare_summaries(&from_xml, &from_binary, &pair.name);
                if differences.is_empty() && ok {
                    report.agreed += 1;
                } else {
                    report.failed += 1;
                }
                report.diagnostics.extend(differences);
            }
            Err(d) => {
                report.failed += 1;
                report.diagnostics.push(d);
            }
        }
    }
    if report.not_compared {
        report.diagnostics.push(codes::diagnostic(
            codes::PAIR_NOT_COMPARED,
            Severity::Info,
            &format!(
                "{} pair(s) verified, no binary reader to compare with",
                report.pairs
            ),
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const XML: &str = "<?xml version=\"1.0\"?><eagle version=\"9.6.2\"><drawing><layers>\
<layer number=\"1\" name=\"Top\"/></layers><board><elements/><signals/></board></drawing></eagle>";

    fn hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// A folder with one good pair and its manifest.
    fn folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let binary = [0x10u8, 0x80, 1, 2, 3];
        fs::write(dir.path().join("a.brd"), binary).unwrap();
        fs::write(dir.path().join("a.brd.xml"), XML).unwrap();
        fs::write(
            dir.path().join(MANIFEST),
            format!(
                "{}  a.brd\n{}  a.brd.xml\n",
                hex(&binary),
                hex(XML.as_bytes())
            ),
        )
        .unwrap();
        dir
    }

    struct Same;
    impl BinaryReader for Same {
        fn read(&self, _: &[u8]) -> Result<DocumentSummary, Diagnostic> {
            Ok(read_bytes(XML.as_bytes(), &Limits::default())
                .summary
                .unwrap())
        }
    }
    struct OneMoreLayer;
    impl BinaryReader for OneMoreLayer {
        fn read(&self, _: &[u8]) -> Result<DocumentSummary, Diagnostic> {
            let mut s = read_bytes(XML.as_bytes(), &Limits::default())
                .summary
                .unwrap();
            s.layers += 1;
            Ok(s)
        }
    }

    fn codes_of(r: &PairReport) -> Vec<String> {
        r.diagnostics
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .collect()
    }

    #[test]
    fn a_good_pair_is_found_and_agrees_with_a_reader_that_reads_the_same() {
        let dir = folder();
        let set = find_pairs(dir.path());
        assert_eq!(set.pairs.len(), 1);
        assert_eq!(set.pairs[0].name, "a.brd");
        assert!(set.diagnostics.is_empty());
        let r = check_pairs(dir.path(), Some(&Same), &Limits::default());
        assert!(r.all_agree(), "{:?}", codes_of(&r));
        assert_eq!((r.pairs, r.agreed, r.failed), (1, 1, 0));
    }

    #[test]
    fn without_a_binary_reader_the_pairs_are_verified_and_nothing_is_compared() {
        let dir = folder();
        let r = check_pairs(dir.path(), None, &Limits::default());
        assert!(r.not_compared && !r.all_agree());
        assert_eq!(codes_of(&r), vec![codes::PAIR_NOT_COMPARED]);
        assert_eq!(r.diagnostics[0].severity, Severity::Info);
    }

    #[test]
    fn a_difference_between_the_two_sides_is_named() {
        let dir = folder();
        let r = check_pairs(dir.path(), Some(&OneMoreLayer), &Limits::default());
        assert_eq!(codes_of(&r), vec![codes::PAIR_DIFFERS]);
        assert!(r.diagnostics[0].message.contains("layers"));
        assert_eq!((r.agreed, r.failed), (0, 1));
    }

    #[test]
    fn a_missing_half_a_changed_file_and_a_missing_manifest_are_reported() {
        let dir = folder();
        fs::write(dir.path().join("b.sch"), [0x10u8, 0x80]).unwrap();
        fs::write(dir.path().join("c.lbr.xml"), XML).unwrap();
        let set = find_pairs(dir.path());
        assert_eq!(set.pairs.len(), 1);
        assert_eq!(set.diagnostics.len(), 2);
        assert!(set
            .diagnostics
            .iter()
            .all(|d| d.code.as_deref() == Some(codes::PAIR_INCOMPLETE)));
        // a file changed after the manifest was made
        fs::write(dir.path().join("a.brd"), [0x10u8, 0x80, 9]).unwrap();
        let r = check_pairs(dir.path(), Some(&Same), &Limits::default());
        assert!(codes_of(&r).contains(&codes::PAIR_HASH.to_owned()));
        assert!(!r.all_agree());
        // no manifest at all
        fs::remove_file(dir.path().join(MANIFEST)).unwrap();
        let r = check_pairs(dir.path(), Some(&Same), &Limits::default());
        assert!(codes_of(&r).contains(&codes::PAIR_HASH.to_owned()));
    }

    #[test]
    fn the_sides_must_be_what_they_say() {
        let dir = folder();
        fs::write(dir.path().join("a.brd"), XML).unwrap();
        let r = check_pairs(dir.path(), Some(&Same), &Limits::default());
        assert!(codes_of(&r).contains(&codes::PAIR_SIDE.to_owned()));
        let dir = folder();
        fs::write(dir.path().join("a.brd.xml"), "not xml").unwrap();
        let r = check_pairs(dir.path(), Some(&Same), &Limits::default());
        assert!(codes_of(&r).contains(&codes::PAIR_SIDE.to_owned()));
        assert_eq!(r.failed, 1);
    }

    #[test]
    fn the_manifest_lines_are_read_strictly() {
        let good = format!("{}  x/y.brd\n# a comment\n\n", "ab".repeat(32));
        let (map, bad) = read_manifest(&good);
        assert!(bad.is_empty());
        assert_eq!(map["x/y.brd"], "ab".repeat(32));
        let (map, bad) = read_manifest("nothex  a.brd\nshort a.brd\n");
        assert!(map.is_empty());
        assert_eq!(bad.len(), 2);
    }
}
