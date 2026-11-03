//! The common interface of the format importers (the core of the format hub).
//!
//! Every importer says which file names and contents it recognises and, for a file, returns one
//! [`ImportOutcome`]: the kind of document, a [`QualityReport`] that lists what was kept,
//! transformed and dropped (nothing is lost silently), and the diagnostics. The [`Hub`] picks the
//! importer by content first and by extension second, and refuses a file that no importer
//! recognises or that two importers claim equally, with stable codes.
//!
//! The hub does not know any format. The importers live in the crates of their formats and are
//! registered by the program that uses them.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// No importer recognises the file.
pub const UNKNOWN_FORMAT: &str = "PHOENIX-HUB-001";
/// Two importers recognise the file equally well.
pub const AMBIGUOUS_FORMAT: &str = "PHOENIX-HUB-002";
/// An importer is registered twice under the same format name.
pub const DUPLICATE_IMPORTER: &str = "PHOENIX-HUB-003";

/// The codes of this crate with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (UNKNOWN_FORMAT, "No importer recognises the file"),
    (
        AMBIGUOUS_FORMAT,
        "Several importers recognise the file equally",
    ),
    (DUPLICATE_IMPORTER, "Importer registered twice"),
];

/// How sure an importer is that a file is of its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Not this format.
    No,
    /// Could be: the name or the first bytes fit but do not prove it.
    Maybe,
    /// This format: a signature of the content was found.
    Yes,
}

/// A count of one category of data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityItem {
    /// What is counted, such as `packages` or `nets`.
    pub category: String,
    /// How many.
    pub count: usize,
}

/// What an import did with the data of the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QualityReport {
    /// Data that came through as it was.
    pub kept: Vec<QualityItem>,
    /// Data that came through changed (a unit, a name, a shape).
    pub transformed: Vec<QualityItem>,
    /// Data that did not come through.
    pub dropped: Vec<QualityItem>,
}

impl QualityReport {
    /// Adds a count to one of the lists, summing with an existing category.
    pub fn add(list: &mut Vec<QualityItem>, category: &str, count: usize) {
        if count == 0 {
            return;
        }
        match list.iter_mut().find(|i| i.category == category) {
            Some(item) => item.count += count,
            None => list.push(QualityItem {
                category: category.to_owned(),
                count,
            }),
        }
    }

    /// True when nothing was transformed or dropped.
    pub fn is_lossless(&self) -> bool {
        self.transformed.is_empty() && self.dropped.is_empty()
    }
}

/// The result of importing one file.
#[derive(Debug, Clone)]
pub struct ImportOutcome {
    /// The format name of the importer that read it.
    pub format: &'static str,
    /// The kind of document (`library`, `board`, `schematic`, ...), empty when unknown.
    pub kind: String,
    /// The version the file declares, empty when none.
    pub version: String,
    /// What was kept, transformed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
    /// True when the file could be read at all (a quality report exists and means something).
    pub usable: bool,
}

/// A reader of one file format.
pub trait Importer {
    /// The name of the format, such as `eagle-xml`. Unique in a hub.
    fn format(&self) -> &'static str;
    /// The file extensions of the format, lower case, without the dot.
    fn extensions(&self) -> &'static [&'static str];
    /// How sure the importer is about a file, given its extension and its first bytes.
    fn sniff(&self, extension: &str, head: &[u8]) -> Confidence;
    /// Reads the whole file.
    fn import(&self, bytes: &[u8]) -> ImportOutcome;
}

/// The importers a program has registered.
#[derive(Default)]
pub struct Hub {
    importers: Vec<Box<dyn Importer>>,
}

/// How many bytes of the start of a file the importers see when they sniff.
pub const SNIFF_BYTES: usize = 4096;

fn hub_diagnostic(code: &'static str, severity: Severity, detail: &str) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    let message = if detail.is_empty() {
        title.to_owned()
    } else {
        format!("{title}: {detail}")
    };
    Diagnostic::new(0, severity, message)
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(Recoverability::Unrecoverable)
}

impl Hub {
    /// An empty hub.
    pub fn new() -> Hub {
        Hub::default()
    }

    /// Registers an importer. A second importer with the same format name is refused.
    pub fn register(&mut self, importer: Box<dyn Importer>) -> Result<(), Diagnostic> {
        if self
            .importers
            .iter()
            .any(|i| i.format() == importer.format())
        {
            return Err(hub_diagnostic(
                DUPLICATE_IMPORTER,
                Severity::Error,
                importer.format(),
            ));
        }
        self.importers.push(importer);
        Ok(())
    }

    /// The format names, in the order of registration.
    pub fn formats(&self) -> Vec<&'static str> {
        self.importers.iter().map(|i| i.format()).collect()
    }

    /// The importer for a file called `name` that starts with `bytes`.
    pub fn detect(&self, name: &str, bytes: &[u8]) -> Result<&dyn Importer, Diagnostic> {
        let extension = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        let head = &bytes[..bytes.len().min(SNIFF_BYTES)];
        let mut best: Vec<(Confidence, &dyn Importer)> = Vec::new();
        for importer in &self.importers {
            let c = importer.sniff(&extension, head);
            if c == Confidence::No {
                continue;
            }
            match best.first() {
                Some((top, _)) if c < *top => {}
                Some((top, _)) if c > *top => {
                    best.clear();
                    best.push((c, importer.as_ref()));
                }
                _ => best.push((c, importer.as_ref())),
            }
        }
        match best.len() {
            0 => Err(hub_diagnostic(
                UNKNOWN_FORMAT,
                Severity::Error,
                &format!("extension `{extension}`"),
            )),
            1 => Ok(best[0].1),
            _ => {
                let names: Vec<&str> = best.iter().map(|(_, i)| i.format()).collect();
                Err(hub_diagnostic(
                    AMBIGUOUS_FORMAT,
                    Severity::Error,
                    &names.join(", "),
                ))
            }
        }
    }

    /// Detects the importer of a file and reads it.
    pub fn import(&self, name: &str, bytes: &[u8]) -> Result<ImportOutcome, Diagnostic> {
        Ok(self.detect(name, bytes)?.import(bytes))
    }
}

/// Adds up a list of outcomes into per-format counts of usable and unusable files (what a corpus
/// panel shows).
pub fn tally(outcomes: &[ImportOutcome]) -> BTreeMap<&'static str, (usize, usize)> {
    let mut map: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for o in outcomes {
        let e = map.entry(o.format).or_insert((0, 0));
        if o.usable {
            e.0 += 1;
        } else {
            e.1 += 1;
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        name: &'static str,
        ext: &'static [&'static str],
        magic: &'static [u8],
    }

    impl Importer for Fake {
        fn format(&self) -> &'static str {
            self.name
        }
        fn extensions(&self) -> &'static [&'static str] {
            self.ext
        }
        fn sniff(&self, extension: &str, head: &[u8]) -> Confidence {
            if head.windows(self.magic.len()).any(|w| w == self.magic) {
                Confidence::Yes
            } else if self.ext.contains(&extension) {
                Confidence::Maybe
            } else {
                Confidence::No
            }
        }
        fn import(&self, bytes: &[u8]) -> ImportOutcome {
            let mut quality = QualityReport::default();
            QualityReport::add(&mut quality.kept, "bytes", bytes.len());
            ImportOutcome {
                format: self.name,
                kind: "thing".into(),
                version: String::new(),
                quality,
                diagnostics: Vec::new(),
                usable: true,
            }
        }
    }

    fn hub() -> Hub {
        let mut h = Hub::new();
        h.register(Box::new(Fake {
            name: "alpha",
            ext: &["al"],
            magic: b"ALPHA",
        }))
        .unwrap();
        h.register(Box::new(Fake {
            name: "beta",
            ext: &["be", "al"],
            magic: b"BETA",
        }))
        .unwrap();
        h
    }

    #[test]
    fn the_content_beats_the_extension() {
        let h = hub();
        // the extension says alpha or beta, the content says beta
        let found = h.detect("x.al", b"..BETA..").unwrap();
        assert_eq!(found.format(), "beta");
        let out = h.import("x.be", b"BETA 123").unwrap();
        assert_eq!(out.format, "beta");
        assert!(out.usable);
        assert!(out.quality.is_lossless());
        assert_eq!(out.quality.kept[0].count, 8);
    }

    #[test]
    fn an_extension_alone_is_enough_when_only_one_importer_has_it() {
        let h = hub();
        assert_eq!(h.detect("x.be", b"nothing").unwrap().format(), "beta");
    }

    #[test]
    fn unknown_and_ambiguous_files_are_refused_with_codes() {
        let h = hub();
        let e = h.detect("x.zz", b"nothing").err().unwrap();
        assert_eq!(e.code.as_deref(), Some(UNKNOWN_FORMAT));
        // both claim the extension `al` and neither signature is present
        let e = h.detect("x.al", b"nothing").err().unwrap();
        assert_eq!(e.code.as_deref(), Some(AMBIGUOUS_FORMAT));
        assert!(e.message.contains("alpha") && e.message.contains("beta"));
    }

    #[test]
    fn a_format_cannot_be_registered_twice() {
        let mut h = hub();
        let e = h
            .register(Box::new(Fake {
                name: "alpha",
                ext: &[],
                magic: b"X",
            }))
            .unwrap_err();
        assert_eq!(e.code.as_deref(), Some(DUPLICATE_IMPORTER));
        assert_eq!(h.formats(), vec!["alpha", "beta"]);
    }

    #[test]
    fn quality_counts_add_up_and_zero_counts_are_not_listed() {
        let mut q = QualityReport::default();
        QualityReport::add(&mut q.kept, "nets", 2);
        QualityReport::add(&mut q.kept, "nets", 3);
        QualityReport::add(&mut q.dropped, "fonts", 0);
        QualityReport::add(&mut q.dropped, "texts", 1);
        assert_eq!(q.kept[0].count, 5);
        assert!(q.dropped.len() == 1);
        assert!(!q.is_lossless());
    }

    #[test]
    fn the_tally_counts_usable_and_unusable_files_by_format() {
        let h = hub();
        let mut outs = vec![h.import("a.be", b"BETA").unwrap()];
        let mut bad = h.import("a.be", b"BETA").unwrap();
        bad.usable = false;
        outs.push(bad);
        assert_eq!(tally(&outs).get("beta"), Some(&(1, 1)));
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
