// SPDX-License-Identifier: GPL-3.0-or-later
//! A whole KiCad project read as one design: the project file, the schematic with its sheets and
//! the board, each by its own reader, with one report and one fidelity level for the set.
//!
//! The functions here open no file: the caller gives the bytes of the files it found and the
//! names of the files it looked for and did not find. A part that cannot be read does not stop the
//! others; it is listed with its finding, and the fidelity level of the set is then `partial`.
//! Nothing is written back to KiCad.

use std::collections::BTreeSet;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::Id;
use eh3d_phoenix_hub::QualityReport;
use serde_json::{json, Value};

use crate::error::KicadError;
use crate::import::Fidelity;
use crate::pcb::{read_pcb, ReadPcb};
use crate::project::{read_project, ReadProject};
use crate::schematic::{read_schematic_with_sheets, ReadSchematic};
use crate::settings::KicadSettings;

/// The name of the document that [`DesignImport::to_value`] makes.
pub const SCHEMA: &str = "phoenix-kicad-import/1";

/// The most part names that a finding about the two sides lists.
const MAX_LISTED_PARTS: usize = 10;

/// The bytes of the files of one project.
#[derive(Debug, Clone, Default)]
pub struct DesignFiles<'a> {
    /// The project file (`.kicad_pro`).
    pub project: Option<&'a [u8]>,
    /// The root schematic (`.kicad_sch`).
    pub schematic: Option<&'a [u8]>,
    /// The board (`.kicad_pcb`).
    pub board: Option<&'a [u8]>,
    /// The names of the files that were looked for next to the project and are not there.
    pub absent: Vec<String>,
}

/// A project read.
#[derive(Debug, Clone)]
pub struct DesignImport {
    /// The project file, when it was given and could be read.
    pub project: Option<ReadProject>,
    /// The schematic, when it was given and could be read.
    pub schematic: Option<ReadSchematic>,
    /// The board, when it was given and could be read.
    pub board: Option<ReadPcb>,
    /// The parts that were given and could not be read (`project`, `schematic` or `board`), each
    /// with the finding that says why.
    pub unreadable: Vec<(&'static str, Diagnostic)>,
    /// The findings about the set: files that are missing and parts that differ.
    pub diagnostics: Vec<Diagnostic>,
}

fn listing(names: &[&String]) -> String {
    let shown: Vec<&str> = names
        .iter()
        .take(MAX_LISTED_PARTS)
        .map(|n| n.as_str())
        .collect();
    let mut text = shown.join(", ");
    if names.len() > MAX_LISTED_PARTS {
        text.push_str(&format!(" and {} more", names.len() - MAX_LISTED_PARTS));
    }
    if text.is_empty() {
        text.push_str("none");
    }
    text
}

/// Reads the files of a project. A schematic asks `loader` for its sheets (never an absolute name
/// or one with `..`).
pub fn read_design(
    files: &DesignFiles<'_>,
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
    loader: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> DesignImport {
    let mut out = DesignImport {
        project: None,
        schematic: None,
        board: None,
        unreadable: Vec::new(),
        diagnostics: Vec::new(),
    };
    for name in &files.absent {
        out.diagnostics.push(
            KicadError::ProjectFile {
                what: name.clone(),
                detail: "the file is not next to the project, so it is not part of the import"
                    .to_owned(),
            }
            .diagnostic(),
        );
    }
    if let Some(bytes) = files.project {
        match read_project(bytes, settings) {
            Ok(read) => out.project = Some(read),
            Err(d) => out.unreadable.push(("project", d)),
        }
    }
    if let Some(bytes) = files.schematic {
        match read_schematic_with_sheets(bytes, settings, &mut *new_id, loader) {
            Ok(read) => out.schematic = Some(read),
            Err(d) => out.unreadable.push(("schematic", d)),
        }
    }
    if let Some(bytes) = files.board {
        match read_pcb(bytes, settings, &mut *new_id) {
            Ok(read) => out.board = Some(read),
            Err(d) => out.unreadable.push(("board", d)),
        }
    }
    if let Some(note) = out.parts_differ() {
        out.diagnostics.push(note);
    }
    out
}

impl DesignImport {
    /// The finding about parts that are on the schematic only or on the board only, when both
    /// were read and they differ. A reference that is not annotated (`R?`) is not compared.
    fn parts_differ(&self) -> Option<Diagnostic> {
        let (schematic, board) = (self.schematic.as_ref()?, self.board.as_ref()?);
        let wanted = |name: &String| !name.is_empty() && !name.contains('?');
        let on_schematic: BTreeSet<String> = schematic
            .schematic
            .parts
            .values()
            .map(|p| p.refdes.clone())
            .filter(wanted)
            .collect();
        let on_board: BTreeSet<String> = board
            .board
            .components
            .values()
            .map(|c| c.refdes.clone())
            .filter(wanted)
            .collect();
        let schematic_only: Vec<&String> = on_schematic.difference(&on_board).collect();
        let board_only: Vec<&String> = on_board.difference(&on_schematic).collect();
        if schematic_only.is_empty() && board_only.is_empty() {
            return None;
        }
        Some(
            KicadError::PartsDiffer {
                detail: format!(
                    "on the schematic only: {}; on the board only: {}",
                    listing(&schematic_only),
                    listing(&board_only)
                ),
            }
            .diagnostic(),
        )
    }

    /// Whether any part could be read.
    pub fn usable(&self) -> bool {
        self.project.is_some() || self.schematic.is_some() || self.board.is_some()
    }

    /// The fidelity level of the set: `complete` when every part was read and nothing was changed
    /// or left out, `partial` when something was, or a part is missing or unreadable, and
    /// `unusable` when no part could be read.
    pub fn fidelity(&self) -> Fidelity {
        if !self.usable() {
            return Fidelity::Unusable;
        }
        let lossless = |q: &QualityReport| q.is_lossless();
        let all_clean = self.project.as_ref().is_none_or(|r| lossless(&r.quality))
            && self.schematic.as_ref().is_none_or(|r| lossless(&r.quality))
            && self.board.as_ref().is_none_or(|r| lossless(&r.quality))
            && self.unreadable.is_empty()
            && self.diagnostics.is_empty();
        if all_clean {
            Fidelity::Complete
        } else {
            Fidelity::Partial
        }
    }

    /// Every finding with the part it comes from (`project`, `schematic`, `board` or `set`).
    pub fn findings(&self) -> Vec<(&'static str, &Diagnostic)> {
        let mut all: Vec<(&'static str, &Diagnostic)> = Vec::new();
        for (part, d) in &self.unreadable {
            all.push((*part, d));
        }
        if let Some(r) = &self.project {
            all.extend(r.diagnostics.iter().map(|d| ("project", d)));
        }
        if let Some(r) = &self.schematic {
            all.extend(r.diagnostics.iter().map(|d| ("schematic", d)));
        }
        if let Some(r) = &self.board {
            all.extend(r.diagnostics.iter().map(|d| ("board", d)));
        }
        all.extend(self.diagnostics.iter().map(|d| ("set", d)));
        all
    }

    /// The reports of the parts that were read, by part.
    pub fn reports(&self) -> Vec<(&'static str, &QualityReport)> {
        let mut all: Vec<(&'static str, &QualityReport)> = Vec::new();
        if let Some(r) = &self.project {
            all.push(("project", &r.quality));
        }
        if let Some(r) = &self.schematic {
            all.push(("schematic", &r.quality));
        }
        if let Some(r) = &self.board {
            all.push(("board", &r.quality));
        }
        all
    }

    /// The whole import as one JSON document: the models, the reports, the findings and the
    /// fidelity level.
    pub fn to_value(&self) -> Result<Value, String> {
        let model = |e: serde_json::Error| format!("cannot write the model: {e}");
        let board = match &self.board {
            Some(r) => json!({
                "version": r.version,
                "generator": r.generator,
                "model": serde_json::to_value(&r.board).map_err(model)?,
            }),
            None => Value::Null,
        };
        let schematic = match &self.schematic {
            Some(r) => json!({
                "version": r.version,
                "generator": r.generator,
                "sheets_read": r.sheets_read,
                "sheet_files": r.sheet_files,
                "footprint_names": serde_json::to_value(&r.footprint_names).map_err(model)?,
                "model": serde_json::to_value(&r.schematic).map_err(model)?,
            }),
            None => Value::Null,
        };
        let project = match &self.project {
            Some(r) => {
                let classes: Vec<Value> = r
                    .net_classes
                    .iter()
                    .map(|c| {
                        json!({
                            "name": c.name,
                            "clearance_nm": c.clearance_nm,
                            "track_width_nm": c.track_width_nm,
                            "via_diameter_nm": c.via_diameter_nm,
                            "via_drill_nm": c.via_drill_nm,
                        })
                    })
                    .collect();
                json!({
                    "meta_version": r.meta_version,
                    "net_classes": classes,
                    "design_rules_nm": r.design_rules,
                    "text_variables": r.text_variables,
                })
            }
            None => Value::Null,
        };
        let mut report = serde_json::Map::new();
        for (part, quality) in self.reports() {
            report.insert(part.to_owned(), quality_value(quality));
        }
        let findings: Vec<Value> = self
            .findings()
            .into_iter()
            .map(|(part, d)| finding_value(part, d))
            .collect();
        let unreadable: Vec<&str> = self.unreadable.iter().map(|(part, _)| *part).collect();
        Ok(json!({
            "schema": SCHEMA,
            "fidelity": self.fidelity().name(),
            "unreadable": unreadable,
            "board": board,
            "schematic": schematic,
            "project": project,
            "report": Value::Object(report),
            "findings": findings,
        }))
    }
}

fn list_value(items: &[eh3d_phoenix_hub::QualityItem]) -> Vec<Value> {
    items
        .iter()
        .map(|i| json!({ "category": i.category, "count": i.count }))
        .collect()
}

fn quality_value(q: &QualityReport) -> Value {
    json!({
        "kept": list_value(&q.kept),
        "changed": list_value(&q.transformed),
        "left_out": list_value(&q.dropped),
    })
}

fn finding_value(part: &str, d: &Diagnostic) -> Value {
    json!({
        "part": part,
        "code": d.code.as_deref(),
        "severity": format!("{:?}", d.severity).to_lowercase(),
        "line": d.line,
        "message": d.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRO: &str = r#"{"meta": {"version": 1},
      "net_settings": {"classes": [{"name": "Default", "clearance": 0.2, "track_width": 0.25,
        "via_diameter": 0.8, "via_drill": 0.4}]}}"#;

    /// Two resistors on one wire; the board holds R1 and R2.
    fn schematic() -> String {
        crate::schematic::tests::root_v6()
    }

    const BOARD: &str = r#"(kicad_pcb (version 20211014) (generator pcbnew)
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user))
  (net 0 "") (net 1 "SIG")
  (footprint "Lib:Part" (layer "F.Cu") (at 10 10)
    (fp_text reference "R1" (at 0 0) (layer "F.SilkS"))
    (fp_text value "1k" (at 0 0) (layer "F.Fab"))
    (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (net 1 "SIG")))
  (footprint "Lib:Part" (layer "F.Cu") (at 20 10)
    (fp_text reference "R9" (at 0 0) (layer "F.SilkS"))
    (fp_text value "1k" (at 0 0) (layer "F.Fab"))
    (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (net 1 "SIG")))
)"#;

    fn read(files: &DesignFiles<'_>) -> DesignImport {
        let mut ids = crate::sequential_ids().unwrap();
        let mut none = |_: &str| -> Option<Vec<u8>> { None };
        read_design(files, &KicadSettings::default(), &mut ids, &mut none)
    }

    #[test]
    fn the_three_files_are_read_together() {
        let sch = schematic();
        let files = DesignFiles {
            project: Some(PRO.as_bytes()),
            schematic: Some(sch.as_bytes()),
            board: Some(BOARD.as_bytes()),
            absent: Vec::new(),
        };
        let r = read(&files);
        assert!(r.usable());
        assert!(r.project.is_some() && r.schematic.is_some() && r.board.is_some());
        assert!(r.unreadable.is_empty());
        assert_eq!(r.board.as_ref().unwrap().board.components.len(), 2);
        assert_eq!(r.fidelity(), Fidelity::Partial);
        // the two readers gave their ids from one sequence: no id is used twice
        let board = &r.board.as_ref().unwrap().board;
        let schematic = &r.schematic.as_ref().unwrap().schematic;
        for id in schematic.parts.keys() {
            assert!(!board.components.contains_key(id));
        }
    }

    #[test]
    fn parts_on_one_side_only_are_found_and_named() {
        let sch = schematic();
        let files = DesignFiles {
            project: None,
            schematic: Some(sch.as_bytes()),
            board: Some(BOARD.as_bytes()),
            absent: Vec::new(),
        };
        let r = read(&files);
        let note = r
            .diagnostics
            .iter()
            .find(|d| d.code.as_deref() == Some("PHOENIX-KICAD-017"))
            .expect("the parts differ");
        // the schematic has R1, R2 and U1 (the power symbols are not parts); the board has R1, R9
        assert!(
            note.message
                .contains("on the schematic only: R2, U1; on the board only: R9"),
            "{}",
            note.message
        );
    }

    #[test]
    fn a_missing_file_is_a_finding_and_a_damaged_part_does_not_stop_the_others() {
        let files = DesignFiles {
            project: Some(b"{\"meta\": "),
            schematic: None,
            board: Some(BOARD.as_bytes()),
            absent: vec!["demo.kicad_sch".to_owned()],
        };
        let r = read(&files);
        assert!(r.usable());
        assert_eq!(r.unreadable.len(), 1);
        assert_eq!(r.unreadable[0].0, "project");
        assert_eq!(r.unreadable[0].1.code.as_deref(), Some("PHOENIX-KICAD-011"));
        let missing = r
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("PHOENIX-KICAD-015"))
            .count();
        assert_eq!(missing, 1);
        assert_eq!(r.fidelity(), Fidelity::Partial);
        // no schematic was read, so the parts are not compared
        assert!(r
            .diagnostics
            .iter()
            .all(|d| d.code.as_deref() != Some("PHOENIX-KICAD-017")));
    }

    #[test]
    fn nothing_readable_is_unusable() {
        let files = DesignFiles {
            project: None,
            schematic: Some(b"(kicad_sch (version 20211123)"),
            board: Some(b"(kicad_sch (version 20211123))"),
            absent: Vec::new(),
        };
        let r = read(&files);
        assert!(!r.usable());
        assert_eq!(r.fidelity(), Fidelity::Unusable);
        assert_eq!(r.unreadable.len(), 2);
        let none = read(&DesignFiles::default());
        assert_eq!(none.fidelity(), Fidelity::Unusable);
    }

    #[test]
    fn the_document_holds_the_models_the_reports_and_the_findings() {
        let sch = schematic();
        let files = DesignFiles {
            project: Some(PRO.as_bytes()),
            schematic: Some(sch.as_bytes()),
            board: Some(BOARD.as_bytes()),
            absent: Vec::new(),
        };
        let value = read(&files).to_value().unwrap();
        assert_eq!(value["schema"], SCHEMA);
        assert_eq!(value["fidelity"], "partial");
        assert_eq!(value["board"]["version"], "20211014");
        assert_eq!(value["project"]["net_classes"][0]["name"], "Default");
        assert_eq!(value["project"]["net_classes"][0]["clearance_nm"], 200_000);
        assert_eq!(
            value["project"]["net_classes"][0]["track_width_nm"],
            250_000
        );
        assert!(value["board"]["model"]["components"].is_object());
        assert!(value["schematic"]["model"]["parts"].is_object());
        assert!(value["report"]["board"]["kept"].is_array());
        let findings = value["findings"].as_array().unwrap();
        assert!(findings
            .iter()
            .any(|f| f["code"] == "PHOENIX-KICAD-017" && f["part"] == "set"));
        // the same input gives the same document
        assert_eq!(read(&files).to_value().unwrap(), value);
    }

    #[test]
    fn a_long_list_of_parts_is_cut() {
        let names: Vec<String> = (1..=13).map(|n| format!("C{n}")).collect();
        let refs: Vec<&String> = names.iter().collect();
        assert!(listing(&refs).ends_with("C10 and 3 more"));
        assert_eq!(listing(&[]), "none");
    }
}
