// SPDX-License-Identifier: GPL-3.0-or-later
//! The import of a set of files: what each file is (Gerber or Excellon), the layer it belongs to
//! and the geometry it holds. A file that cannot be read does not stop the others; it is kept with
//! its error. The result is read-only geometry for review: it is not merged into a board and it is
//! never written back.

use crate::codes::{
    Finding, GbrError, Severity, GERBER_SYNTAX, INPUT_TOO_LARGE, LIMIT_EXCEEDED, ROLE_UNKNOWN,
};
use crate::drill::parse_drill;
use crate::identify::{identify, sniff, Identified, Kind};
use crate::model::{DrillFile, GerberLayer};
use crate::parse::parse_gerber;
use crate::settings::ImportSettings;

/// What a file held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// A Gerber file.
    Gerber(GerberLayer),
    /// An Excellon file.
    Drill(DrillFile),
    /// A file that could not be read.
    Failed(GbrError),
}

/// A file of the import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedFile {
    /// The name of the file as it was given.
    pub name: String,
    /// The layer it belongs to, when it could be told.
    pub identified: Option<Identified>,
    /// What it held.
    pub content: Content,
}

impl ImportedFile {
    /// The findings of the file (none for a file that failed).
    pub fn findings(&self) -> &[Finding] {
        match &self.content {
            Content::Gerber(layer) => &layer.findings,
            Content::Drill(file) => &file.findings,
            Content::Failed(_) => &[],
        }
    }

    /// The Gerber layer, when the file is one.
    pub fn gerber(&self) -> Option<&GerberLayer> {
        match &self.content {
            Content::Gerber(layer) => Some(layer),
            _ => None,
        }
    }

    /// The drill file, when the file is one.
    pub fn drill(&self) -> Option<&DrillFile> {
        match &self.content {
            Content::Drill(file) => Some(file),
            _ => None,
        }
    }

    /// The layers a drill file of blind or buried vias joins, when the file says so with its
    /// `.FileFunction` (`Plated,1,2,Blind`, `Plated,2,3,Buried`).
    pub fn drill_span(&self) -> Option<crate::identify::DrillSpan> {
        self.drill()
            .and_then(|file| crate::identify::drill_span(&file.attributes))
    }
}

/// The files of one import, in the order they were given.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportedSet {
    /// The files.
    pub files: Vec<ImportedFile>,
}

impl ImportedSet {
    /// The number of files that could not be read.
    pub fn failed(&self) -> usize {
        self.files
            .iter()
            .filter(|f| matches!(f.content, Content::Failed(_)))
            .count()
    }

    /// The file that holds a layer role, when there is one.
    pub fn by_role(&self, role: crate::identify::Role) -> Option<&ImportedFile> {
        self.files
            .iter()
            .find(|f| f.identified.is_some_and(|i| i.role == role))
    }
}

fn add_finding(content: &mut Content, finding: Finding) {
    match content {
        Content::Gerber(layer) => layer.findings.push(finding),
        Content::Drill(file) => file.findings.push(finding),
        Content::Failed(_) => {}
    }
}

/// Reads one file. `name` is only used to tell the layer; nothing is read from disk here.
pub fn import_file(name: &str, bytes: &[u8], settings: &ImportSettings) -> ImportedFile {
    if bytes.len() > settings.max_file_bytes {
        return ImportedFile {
            name: name.to_owned(),
            identified: None,
            content: Content::Failed(GbrError::new(
                INPUT_TOO_LARGE,
                format!(
                    "{} bytes, the limit is {}",
                    bytes.len(),
                    settings.max_file_bytes
                ),
            )),
        };
    }
    let text = String::from_utf8_lossy(bytes);
    let by_name_is_drill = crate::identify::from_name(name).is_some_and(|r| r.is_drill());
    let kind = sniff(&text).or(by_name_is_drill.then_some(Kind::Excellon));
    let mut content = match kind {
        Some(Kind::Gerber) => match parse_gerber(&text, settings) {
            Ok(layer) => Content::Gerber(layer),
            Err(error) => Content::Failed(error),
        },
        Some(Kind::Excellon) => match parse_drill(&text, settings) {
            Ok(file) => Content::Drill(file),
            Err(error) => Content::Failed(error),
        },
        None => Content::Failed(GbrError::new(
            GERBER_SYNTAX,
            "the file is neither Gerber nor Excellon",
        )),
    };
    let attributes = match &content {
        Content::Gerber(layer) => Some(&layer.attributes),
        Content::Drill(file) => Some(&file.attributes),
        Content::Failed(_) => None,
    };
    let identified = attributes.and_then(|a| identify(name, a));
    let is_drill_content = match &content {
        Content::Drill(_) => Some(true),
        Content::Gerber(_) => Some(false),
        Content::Failed(_) => None,
    };
    match (identified, is_drill_content) {
        (None, Some(_)) => add_finding(
            &mut content,
            Finding::new(
                ROLE_UNKNOWN,
                Severity::Info,
                0,
                "neither the attributes nor the name tell the layer",
            ),
        ),
        (Some(found), Some(false)) if found.role.is_drill() => add_finding(
            &mut content,
            Finding::new(
                ROLE_UNKNOWN,
                Severity::Warning,
                0,
                "the name says a drill file and the content is Gerber",
            ),
        ),
        (Some(found), Some(true)) if !found.role.is_drill() => add_finding(
            &mut content,
            Finding::new(
                ROLE_UNKNOWN,
                Severity::Warning,
                0,
                "the name says a layer and the content is Excellon",
            ),
        ),
        _ => {}
    }
    ImportedFile {
        name: name.to_owned(),
        identified,
        content,
    }
}

/// Reads a set of files given as a name and the bytes of each.
pub fn import_files(
    inputs: &[(String, Vec<u8>)],
    settings: &ImportSettings,
) -> Result<ImportedSet, GbrError> {
    if inputs.len() > settings.max_files {
        return Err(GbrError::new(
            LIMIT_EXCEEDED,
            format!(
                "{} files, the limit is {}",
                inputs.len(),
                settings.max_files
            ),
        ));
    }
    Ok(ImportedSet {
        files: inputs
            .iter()
            .map(|(name, bytes)| import_file(name, bytes, settings))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identify::{Role, Source};

    const COPPER: &str = "%TF.FileFunction,Copper,L1,Top*%\n%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.2*%\nD10*\nX0Y0D03*\nM02*\n";
    const DRILL: &str = "M48\nMETRIC,TZ\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n";

    fn settings() -> ImportSettings {
        ImportSettings::default()
    }

    #[test]
    fn each_file_is_read_by_its_kind_and_its_role_is_told() {
        let set = import_files(
            &[
                ("a.gbr".to_owned(), COPPER.as_bytes().to_vec()),
                ("a.drl".to_owned(), DRILL.as_bytes().to_vec()),
                (
                    "a-F_Mask.gbr".to_owned(),
                    b"%FSLAX46Y46*%\n%MOMM*%\nM02*\n".to_vec(),
                ),
            ],
            &settings(),
        )
        .unwrap();
        assert_eq!(set.files.len(), 3);
        assert_eq!(set.failed(), 0);
        let copper = &set.files[0];
        assert_eq!(copper.identified.unwrap().role, Role::CopperTop);
        assert_eq!(copper.identified.unwrap().source, Source::Attribute);
        assert_eq!(copper.gerber().unwrap().flashes(), 1);
        assert!(copper.drill().is_none());
        let drill = &set.files[1];
        assert_eq!(drill.identified.unwrap().role, Role::Drill);
        assert_eq!(drill.drill().unwrap().hits.len(), 1);
        assert_eq!(set.files[2].identified.unwrap().source, Source::FileName);
        assert_eq!(set.by_role(Role::MaskTop).unwrap().name, "a-F_Mask.gbr");
        assert!(set.by_role(Role::Outline).is_none());
    }

    #[test]
    fn a_file_that_fails_does_not_stop_the_others() {
        let set = import_files(
            &[
                ("junk.gbr".to_owned(), b"hello".to_vec()),
                (
                    "broken.gbr".to_owned(),
                    b"%FSLAX46Y46*%\nX0Y0D03*\n".to_vec(),
                ),
                ("a.gbr".to_owned(), COPPER.as_bytes().to_vec()),
            ],
            &settings(),
        )
        .unwrap();
        assert_eq!(set.failed(), 2);
        match &set.files[0].content {
            Content::Failed(e) => assert_eq!(e.code, GERBER_SYNTAX),
            other => panic!("{other:?}"),
        }
        assert!(set.files[0].findings().is_empty());
        assert!(set.files[2].gerber().is_some());
    }

    #[test]
    fn a_layer_that_cannot_be_told_and_a_wrong_name_are_findings() {
        let unknown = import_file(
            "mystery.bin",
            b"%FSLAX46Y46*%\n%MOMM*%\nM02*\n",
            &settings(),
        );
        assert!(unknown.identified.is_none());
        assert!(unknown
            .findings()
            .iter()
            .any(|f| f.code == ROLE_UNKNOWN && f.severity == Severity::Info));
        let bare = b"%FSLAX46Y46*%\n%MOMM*%\nM02*\n";
        let wrong = import_file("a.drl", bare, &settings());
        assert!(wrong
            .findings()
            .iter()
            .any(|f| f.code == ROLE_UNKNOWN && f.severity == Severity::Warning));
        let other = import_file("a.gtl", DRILL.as_bytes(), &settings());
        assert!(other
            .findings()
            .iter()
            .any(|f| f.code == ROLE_UNKNOWN && f.severity == Severity::Warning));
    }

    #[test]
    fn the_limits_on_files_and_bytes_are_enforced() {
        let few = ImportSettings {
            max_files: 1,
            ..settings()
        };
        let two = [
            ("a.gbr".to_owned(), COPPER.as_bytes().to_vec()),
            ("b.gbr".to_owned(), COPPER.as_bytes().to_vec()),
        ];
        assert_eq!(import_files(&two, &few).unwrap_err().code, LIMIT_EXCEEDED);
        let small = ImportSettings {
            max_file_bytes: 20,
            ..settings()
        };
        let file = import_file("a.gbr", COPPER.as_bytes(), &small);
        match file.content {
            Content::Failed(e) => assert_eq!(e.code, INPUT_TOO_LARGE),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_that_is_not_utf8_is_still_read() {
        let mut bytes = b"G04 caf\xe9 *\n".to_vec();
        bytes.extend_from_slice(COPPER.as_bytes());
        let file = import_file("a.gtl", &bytes, &settings());
        assert_eq!(file.gerber().unwrap().flashes(), 1);
    }
}
