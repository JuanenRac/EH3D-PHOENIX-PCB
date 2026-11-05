// SPDX-License-Identifier: GPL-3.0-or-later
//! The KiCad readers as an importer of the format hub.
//!
//! The importer finds the kind of file from its content (the name of the root list, or a JSON
//! object for a project) and reads it with the reader of that kind. The quality report is the one
//! of the reader: `kept` counts what came through as it was, `transformed` what came through
//! changed and `dropped` what did not come through, each with its reason in the name. The last
//! diagnostic of every outcome says the fidelity level.

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_hub::{Confidence, ImportOutcome, Importer, QualityReport};

use crate::error::KicadError;
use crate::pcb::read_pcb;
use crate::project::read_project;
use crate::schematic::read_schematic_with_sheets;
use crate::settings::KicadSettings;
use crate::symbols::read_symbol_library;
use crate::{footprint::read_footprint, sequential_ids};

/// The format name used in the hub.
pub const FORMAT: &str = "kicad";

/// The kind of KiCad file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadKind {
    /// A `.kicad_sch` schematic sheet.
    Schematic,
    /// A `.kicad_pcb` board.
    Board,
    /// A `.kicad_sym` symbol library.
    SymbolLibrary,
    /// A `.kicad_mod` footprint.
    Footprint,
    /// A `.kicad_pro` project (JSON).
    Project,
}

impl KicadKind {
    /// The name of the kind, as the outcome of the hub says it.
    pub fn name(self) -> &'static str {
        match self {
            KicadKind::Schematic => "schematic",
            KicadKind::Board => "board",
            KicadKind::SymbolLibrary => "symbol library",
            KicadKind::Footprint => "footprint",
            KicadKind::Project => "project",
        }
    }
}

/// How faithfully a file came into the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// Everything that was read came through as it was.
    Complete,
    /// The file was read and some of it was changed or left out; the report says what.
    Partial,
    /// The file could not be read.
    Unusable,
}

impl Fidelity {
    /// The name of the level.
    pub fn name(self) -> &'static str {
        match self {
            Fidelity::Complete => "complete",
            Fidelity::Partial => "partial",
            Fidelity::Unusable => "unusable",
        }
    }
}

/// The fidelity level of an import from its quality report.
pub fn fidelity_of(usable: bool, quality: &QualityReport) -> Fidelity {
    if !usable {
        Fidelity::Unusable
    } else if quality.is_lossless() {
        Fidelity::Complete
    } else {
        Fidelity::Partial
    }
}

/// The kind of file from its first bytes: the name of the root list, or an opening brace.
pub fn detect_kind(head: &[u8]) -> Option<KicadKind> {
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(head);
    let first = head.iter().position(|b| !b.is_ascii_whitespace())?;
    let rest = &head[first..];
    match rest.first()? {
        b'{' => Some(KicadKind::Project),
        b'(' => {
            let after = &rest[1..];
            let start = after.iter().position(|b| !b.is_ascii_whitespace())?;
            let word: Vec<u8> = after[start..]
                .iter()
                .copied()
                .take_while(|b| !b.is_ascii_whitespace() && *b != b'(' && *b != b')')
                .collect();
            match word.as_slice() {
                b"kicad_sch" => Some(KicadKind::Schematic),
                b"kicad_pcb" => Some(KicadKind::Board),
                b"kicad_symbol_lib" => Some(KicadKind::SymbolLibrary),
                b"footprint" | b"module" => Some(KicadKind::Footprint),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The importer of KiCad files (versions 6 to 9).
#[derive(Debug, Clone, Default)]
pub struct KicadImporter {
    /// The limits of the readers.
    pub settings: KicadSettings,
}

fn unusable(kind: &str, diagnostic: Diagnostic) -> ImportOutcome {
    let note = KicadError::Fidelity {
        level: Fidelity::Unusable.name().to_owned(),
        detail: "the file could not be read".to_owned(),
    }
    .diagnostic();
    ImportOutcome {
        format: FORMAT,
        kind: kind.to_owned(),
        version: String::new(),
        quality: QualityReport::default(),
        diagnostics: vec![diagnostic, note],
        usable: false,
    }
}

fn usable(
    kind: KicadKind,
    version: String,
    quality: QualityReport,
    mut diagnostics: Vec<Diagnostic>,
) -> ImportOutcome {
    let level = fidelity_of(true, &quality);
    let detail = format!(
        "{} kind(s) of data kept, {} changed, {} left out",
        quality.kept.len(),
        quality.transformed.len(),
        quality.dropped.len()
    );
    diagnostics.push(
        KicadError::Fidelity {
            level: level.name().to_owned(),
            detail,
        }
        .diagnostic(),
    );
    ImportOutcome {
        format: FORMAT,
        kind: kind.name().to_owned(),
        version,
        quality,
        diagnostics,
        usable: true,
    }
}

impl KicadImporter {
    /// Reads a file. A schematic asks `loader` for the files of its sheets (never an absolute
    /// name or one with `..`); the other kinds do not use it.
    pub fn import_with_sheets(
        &self,
        bytes: &[u8],
        loader: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    ) -> ImportOutcome {
        let head = &bytes[..bytes.len().min(4096)];
        let Some(kind) = detect_kind(head) else {
            return unusable(
                "",
                KicadError::NotKicad {
                    expected: "a schematic, board, symbol library, footprint or project".to_owned(),
                    found: "a file that starts with something else".to_owned(),
                }
                .diagnostic(),
            );
        };
        let mut ids = match sequential_ids() {
            Ok(ids) => ids,
            Err(e) => return unusable(kind.name(), e.diagnostic()),
        };
        let s = &self.settings;
        match kind {
            KicadKind::Schematic => match read_schematic_with_sheets(bytes, s, &mut ids, loader) {
                Ok(r) => usable(kind, r.version, r.quality, r.diagnostics),
                Err(d) => unusable(kind.name(), d),
            },
            KicadKind::Board => match read_pcb(bytes, s, &mut ids) {
                Ok(r) => usable(kind, r.version, r.quality, r.diagnostics),
                Err(d) => unusable(kind.name(), d),
            },
            KicadKind::SymbolLibrary => match read_symbol_library(bytes, s, &mut ids) {
                Ok(r) => usable(kind, r.version, r.quality, r.diagnostics),
                Err(d) => unusable(kind.name(), d),
            },
            KicadKind::Footprint => match read_footprint(bytes, s, &mut ids) {
                Ok(r) => usable(kind, r.version, r.quality, r.diagnostics),
                Err(d) => unusable(kind.name(), d),
            },
            KicadKind::Project => match read_project(bytes, s) {
                Ok(r) => usable(
                    kind,
                    r.meta_version.map(|v| v.to_string()).unwrap_or_default(),
                    r.quality,
                    r.diagnostics,
                ),
                Err(d) => unusable(kind.name(), d),
            },
        }
    }
}

impl Importer for KicadImporter {
    fn format(&self) -> &'static str {
        FORMAT
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[
            "kicad_sch",
            "kicad_pcb",
            "kicad_sym",
            "kicad_mod",
            "kicad_pro",
        ]
    }

    fn sniff(&self, extension: &str, head: &[u8]) -> Confidence {
        let named = self.extensions().contains(&extension);
        match detect_kind(head) {
            Some(KicadKind::Project) => {
                let has = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);
                if extension == "kicad_pro" && (has(b"\"meta\"") || has(b"\"net_settings\"")) {
                    Confidence::Yes
                } else if extension == "kicad_pro" {
                    Confidence::Maybe
                } else {
                    Confidence::No
                }
            }
            Some(_) => Confidence::Yes,
            None if named => Confidence::Maybe,
            None => Confidence::No,
        }
    }

    fn import(&self, bytes: &[u8]) -> ImportOutcome {
        let mut none = |_: &str| -> Option<Vec<u8>> { None };
        self.import_with_sheets(bytes, &mut none)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_hub::Hub;

    fn import(text: &str) -> ImportOutcome {
        KicadImporter::default().import(text.as_bytes())
    }

    fn fidelity(out: &ImportOutcome) -> String {
        out.diagnostics
            .iter()
            .rev()
            .find(|d| d.code.as_deref() == Some("PHOENIX-KICAD-014"))
            .map(|d| d.message.clone())
            .unwrap_or_default()
    }

    #[test]
    fn the_kind_is_found_from_the_content() {
        for (text, kind) in [
            ("(kicad_sch (version 1))", Some(KicadKind::Schematic)),
            ("\n  ( kicad_pcb (version 1))", Some(KicadKind::Board)),
            ("(kicad_symbol_lib)", Some(KicadKind::SymbolLibrary)),
            ("(footprint \"A\")", Some(KicadKind::Footprint)),
            ("(module A)", Some(KicadKind::Footprint)),
            ("{\"meta\":1}", Some(KicadKind::Project)),
            ("\u{feff}(kicad_pcb", Some(KicadKind::Board)),
            ("(something_else)", None),
            ("hello", None),
            ("", None),
            ("   ", None),
            ("(", None),
        ] {
            assert_eq!(detect_kind(text.as_bytes()), kind, "{text:?}");
        }
    }

    #[test]
    fn every_kind_of_file_is_imported_with_a_report_and_a_fidelity_note() {
        let sch = import(&crate::schematic::tests::root_v6());
        assert!(sch.usable);
        assert_eq!((sch.format, sch.kind.as_str()), (FORMAT, "schematic"));
        assert_eq!(sch.version, "20211123");
        assert!(sch.quality.kept.iter().any(|i| i.category == "nets"));
        assert!(fidelity(&sch).contains("partial"));

        let pcb = import(crate::pcb::tests::PCB_V6);
        assert_eq!(pcb.kind, "board");
        assert!(pcb
            .quality
            .dropped
            .iter()
            .any(|i| i.category.contains("target")));
        assert!(fidelity(&pcb).contains("partial"));

        let sym = import(crate::symbols::tests::LIB_V6);
        assert_eq!(sym.kind, "symbol library");
        assert!(sym.usable);

        let fp = import(crate::footprint::tests::MOD_V6);
        assert_eq!(fp.kind, "footprint");
        assert_eq!(fp.version, "20211014");

        let pro = import(crate::project::tests::PRO_V6);
        assert_eq!((pro.kind.as_str(), pro.version.as_str()), ("project", "1"));
        assert!(pro.usable);
    }

    #[test]
    fn a_file_with_nothing_left_out_is_complete() {
        let out = import("(kicad_symbol_lib (version 20211014) (symbol \"A\"))");
        assert!(out.usable);
        assert!(out.quality.is_lossless());
        assert!(fidelity(&out).contains("complete"));
        assert_eq!(fidelity_of(true, &out.quality), Fidelity::Complete);
        assert_eq!(fidelity_of(false, &out.quality), Fidelity::Unusable);
    }

    #[test]
    fn damaged_and_foreign_files_are_not_usable_and_say_why() {
        let cut = import("(kicad_pcb (version 20211014) (segment");
        assert!(!cut.usable);
        assert_eq!(cut.kind, "board");
        assert_eq!(
            cut.diagnostics[0].code.as_deref(),
            Some("PHOENIX-KICAD-004")
        );
        assert!(fidelity(&cut).contains("unusable"));
        let foreign = import("<?xml version=\"1.0\"?><eagle/>");
        assert!(!foreign.usable);
        assert_eq!(
            foreign.diagnostics[0].code.as_deref(),
            Some("PHOENIX-KICAD-003")
        );
        let wrong = import("(kicad_sch (version 20211123)))");
        assert!(!wrong.usable);
        assert_eq!(
            wrong.diagnostics[0].code.as_deref(),
            Some("PHOENIX-KICAD-001")
        );
        let json = import("{\"meta\": ");
        assert!(!json.usable);
        assert_eq!(
            json.diagnostics[0].code.as_deref(),
            Some("PHOENIX-KICAD-011")
        );
    }

    #[test]
    fn the_hub_finds_the_importer_by_content_and_by_extension() {
        let mut hub = Hub::new();
        hub.register(Box::new(KicadImporter::default())).unwrap();
        let by_content = hub
            .import("anything.txt", crate::pcb::tests::PCB_V6.as_bytes())
            .unwrap();
        assert_eq!(by_content.kind, "board");
        // an extension alone is enough; the reader then says what is wrong with the content
        let by_name = hub.import("x.kicad_sch", b"nothing useful").unwrap();
        assert!(!by_name.usable);
        // a JSON file is a KiCad project only by its extension and its sections
        assert!(hub.detect("other.json", b"{\"meta\": {}}").is_err());
        assert!(hub
            .detect("a.kicad_pro", b"{\"meta\": {\"version\": 1}}")
            .is_ok());
        assert!(hub.detect("x.brd", b"hello").is_err());
        assert_eq!(hub.formats(), vec![FORMAT]);
    }

    #[test]
    fn a_schematic_asks_the_loader_for_its_sheets() {
        let mut asked: Vec<String> = Vec::new();
        let child = crate::schematic::tests::child_v6();
        let mut loader = |name: &str| -> Option<Vec<u8>> {
            asked.push(name.to_owned());
            Some(child.clone().into_bytes())
        };
        let out = KicadImporter::default()
            .import_with_sheets(crate::schematic::tests::root_v6().as_bytes(), &mut loader);
        assert!(out.usable);
        assert_eq!(asked, vec!["sub.kicad_sch".to_owned()]);
        assert_eq!(
            out.quality
                .kept
                .iter()
                .find(|i| i.category == "sheets")
                .map(|i| i.count),
            Some(2)
        );
    }
}
