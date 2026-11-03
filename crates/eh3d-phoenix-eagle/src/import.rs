//! The EAGLE XML reader as an importer of the format hub.
//!
//! The quality report says what the summary counted. `kept` lists what was read into the summary;
//! `dropped` lists the elements that were counted under their name and not interpreted; nothing is
//! transformed, because the summary holds counts and names and no converted values.

use eh3d_phoenix_hub::{Confidence, ImportOutcome, Importer, QualityReport};

use crate::document::{read_bytes, DocumentSummary};
use crate::xml::Limits;

/// The importer of EAGLE XML files (versions 6 to 9).
#[derive(Debug, Clone, Copy, Default)]
pub struct EagleImporter {
    /// The limits of the XML parser.
    pub limits: Limits,
}

/// The format name used in the hub.
pub const FORMAT: &str = "eagle-xml";

fn quality_of(s: &DocumentSummary) -> QualityReport {
    let mut q = QualityReport::default();
    QualityReport::add(&mut q.kept, "layers", s.layers);
    QualityReport::add(&mut q.kept, "packages", s.packages.len());
    QualityReport::add(&mut q.kept, "symbols", s.symbols.len());
    QualityReport::add(&mut q.kept, "device sets", s.device_sets.len());
    QualityReport::add(&mut q.kept, "parts", s.parts);
    QualityReport::add(&mut q.kept, "nets", s.nets);
    QualityReport::add(&mut q.kept, "routed items", s.routed_items);
    QualityReport::add(&mut q.kept, "sheets", s.sheets);
    QualityReport::add(&mut q.kept, "embedded libraries", s.embedded_libraries);
    for (name, n) in &s.uninterpreted {
        QualityReport::add(&mut q.dropped, name, *n);
    }
    q
}

impl Importer for EagleImporter {
    fn format(&self) -> &'static str {
        FORMAT
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["lbr", "brd", "sch"]
    }

    fn sniff(&self, extension: &str, head: &[u8]) -> Confidence {
        let has = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);
        if has(b"<eagle") {
            Confidence::Yes
        } else if self.extensions().contains(&extension) && has(b"<?xml") {
            Confidence::Maybe
        } else {
            Confidence::No
        }
    }

    fn import(&self, bytes: &[u8]) -> ImportOutcome {
        let reading = read_bytes(bytes, &self.limits);
        match reading.summary {
            Some(s) => ImportOutcome {
                format: FORMAT,
                kind: s.kind.name().to_owned(),
                version: s.version.clone(),
                quality: quality_of(&s),
                diagnostics: reading.diagnostics,
                usable: true,
            },
            None => ImportOutcome {
                format: FORMAT,
                kind: String::new(),
                version: String::new(),
                quality: QualityReport::default(),
                diagnostics: reading.diagnostics,
                usable: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_hub::Hub;

    const BRD: &str = "<?xml version=\"1.0\"?><eagle version=\"9.6.2\"><drawing><layers><layer number=\"1\" name=\"Top\"/></layers><board><elements><element name=\"R1\"/></elements><signals><signal name=\"A\"><wire x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\"/></signal></signals><weird/><weird/></board></drawing></eagle>";

    #[test]
    fn the_hub_finds_the_importer_by_content_even_with_another_extension() {
        let mut hub = Hub::new();
        hub.register(Box::new(EagleImporter::default())).unwrap();
        let out = hub.import("anything.xml", BRD.as_bytes()).unwrap();
        assert_eq!(out.format, FORMAT);
        assert_eq!(out.kind, "board");
        assert_eq!(out.version, "9.6.2");
        assert!(out.usable);
    }

    #[test]
    fn the_quality_report_lists_what_was_kept_and_what_was_not_interpreted() {
        let out = EagleImporter::default().import(BRD.as_bytes());
        let kept = |name: &str| {
            out.quality
                .kept
                .iter()
                .find(|i| i.category == name)
                .map(|i| i.count)
        };
        assert_eq!(kept("parts"), Some(1));
        assert_eq!(kept("nets"), Some(1));
        assert_eq!(kept("routed items"), Some(1));
        assert_eq!(kept("layers"), Some(1));
        assert_eq!(out.quality.dropped.len(), 1);
        assert_eq!(out.quality.dropped[0].category, "weird");
        assert_eq!(out.quality.dropped[0].count, 2);
        assert!(!out.quality.is_lossless());
    }

    #[test]
    fn a_file_that_is_not_eagle_is_not_usable() {
        let out = EagleImporter::default().import(b"<html/>");
        assert!(!out.usable);
        assert!(out.kind.is_empty());
        assert_eq!(
            out.diagnostics[0].code.as_deref(),
            Some("PHOENIX-EAGLE-002")
        );
        let mut hub = Hub::new();
        hub.register(Box::new(EagleImporter::default())).unwrap();
        assert!(hub.import("x.txt", b"hello").is_err());
    }
}
