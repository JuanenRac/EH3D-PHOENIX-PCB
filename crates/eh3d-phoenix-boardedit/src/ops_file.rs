// SPDX-License-Identifier: GPL-3.0-or-later
//! The file of operations that `phoenix boardedit` applies to a board: JSON, either a list of
//! operations or an object with the list under `ops`. The operations are those of [`EditOp`]
//! (`{"op": "route", "net": "...", "layer": "...", "points": [...]}`).

use eh3d_phoenix_board::Id;
use eh3d_phoenix_rules::Lang;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::codes::{BoardEditError, OPERATIONS_INVALID};
use crate::edit::{BoardEdit, DrcReport};
use crate::ops::EditOp;

/// The most operations a file may hold.
pub const MAX_OPERATIONS: usize = 100_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationsFile {
    ops: Vec<EditOp>,
}

/// Reads a file of operations.
pub fn read_operations(text: &str) -> Result<Vec<EditOp>, BoardEditError> {
    let bad = |e: serde_json::Error| {
        BoardEditError::new(OPERATIONS_INVALID, format!("the file cannot be read: {e}"))
    };
    let ops = if text.trim_start().starts_with('[') {
        serde_json::from_str::<Vec<EditOp>>(text).map_err(bad)?
    } else {
        serde_json::from_str::<OperationsFile>(text)
            .map_err(bad)?
            .ops
    };
    if ops.len() > MAX_OPERATIONS {
        return Err(BoardEditError::new(
            OPERATIONS_INVALID,
            format!("more than {MAX_OPERATIONS} operations"),
        ));
    }
    Ok(ops)
}

/// What a run of operations did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyReport {
    /// How many operations were applied.
    pub applied: usize,
    /// How many of them were forced past a rule error.
    pub forced: usize,
    /// The ids of the parts the operations made.
    pub created: Vec<Id>,
    /// Things the operations said.
    pub notes: Vec<String>,
    /// Components before and after.
    pub components: (usize, usize),
    /// Tracks before and after.
    pub tracks: (usize, usize),
    /// Vias before and after.
    pub vias: (usize, usize),
    /// Nets before and after.
    pub nets: (usize, usize),
    /// What the design rules say changed, over the whole run.
    pub drc: DrcReport,
}

impl ApplyReport {
    /// The report as text, with the titles of the rules in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!(
            "operations applied: {} ({} forced)\nnew items: {}\ncomponents: {} -> {}\ntracks: {} -> {}\nvias: {} -> {}\nnets: {} -> {}\n",
            self.applied,
            self.forced,
            self.created.len(),
            self.components.0,
            self.components.1,
            self.tracks.0,
            self.tracks.1,
            self.vias.0,
            self.vias.1,
            self.nets.0,
            self.nets.1
        );
        for note in &self.notes {
            out.push_str(&format!("note: {note}\n"));
        }
        out.push_str(&self.drc.to_text(lang));
        out
    }

    /// The report as a JSON value.
    pub fn to_value(&self) -> Value {
        json!({
            "applied": self.applied,
            "forced": self.forced,
            "created": self.created.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "notes": self.notes,
            "components": [self.components.0, self.components.1],
            "tracks": [self.tracks.0, self.tracks.1],
            "vias": [self.vias.0, self.vias.1],
            "nets": [self.nets.0, self.nets.1],
            "drc": self.drc.to_value(),
        })
    }
}

/// The numbers of parts of an editor: components, tracks, vias and nets.
fn counts(edit: &BoardEdit) -> (usize, usize, usize, usize) {
    let b = edit.board();
    (
        b.components.len(),
        b.tracks.len(),
        b.vias.len(),
        b.nets.len(),
    )
}

/// Applies the operations one after the other. All or nothing: the first operation that is
/// refused ends the run with its number (from 1) and the editor is left as it was.
pub fn apply_operations(
    edit: &mut BoardEdit,
    ops: &[EditOp],
) -> Result<ApplyReport, BoardEditError> {
    let before = counts(edit);
    let findings_before = edit.findings().to_vec();
    let outcomes = edit.apply_all(ops)?;
    let after = counts(edit);
    let mut report = ApplyReport {
        applied: outcomes.len(),
        components: (before.0, after.0),
        tracks: (before.1, after.1),
        vias: (before.2, after.2),
        nets: (before.3, after.3),
        drc: DrcReport::new(&findings_before, edit.findings()),
        ..ApplyReport::default()
    };
    for outcome in outcomes {
        report.forced += usize::from(outcome.forced);
        report.created.extend(outcome.created);
        report.notes.extend(outcome.notes);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::OPERATION_REFUSED;
    use crate::settings::BoardEditSettings;
    use crate::testutil::{fixture, strict_rules};

    #[test]
    fn both_forms_of_the_file_are_read_and_nothing_else() {
        let f = fixture();
        let op = format!(r#"{{"op":"delete_net","net":"{}"}}"#, f.sig);
        let list = format!("[{op}]");
        let object = format!(r#"{{"ops":[{op}]}}"#);
        assert_eq!(read_operations(&list).unwrap().len(), 1);
        assert_eq!(
            read_operations(&object).unwrap(),
            read_operations(&list).unwrap()
        );
        for bad in [
            "",
            "{}",
            "[1]",
            r#"{"ops":[],"extra":1}"#,
            r#"[{"op":"fly"}]"#,
        ] {
            assert_eq!(
                read_operations(bad).unwrap_err().code,
                OPERATIONS_INVALID,
                "{bad}"
            );
        }
    }

    #[test]
    fn operations_are_applied_in_order_and_counted() {
        let f = fixture();
        let mut edit = BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap();
        let ops = vec![
            EditOp::Via {
                net: f.sig,
                at: f.pt(20, 20),
                from_layer: None,
                to_layer: None,
            },
            EditOp::DeleteTrack { track: f.track },
        ];
        let report = apply_operations(&mut edit, &ops).unwrap();
        assert_eq!(report.applied, 2);
        assert_eq!(report.created.len(), 1);
        assert_eq!(report.tracks, (1, 0));
        assert_eq!(report.vias, (0, 1));
        assert_eq!(report.components, (2, 2));
        assert!(report.to_text(Lang::En).contains("tracks: 1 -> 0"));
        assert_eq!(report.to_value()["vias"], json!([0, 1]));
        // the board that was given is as it was
        assert_eq!(f.board.tracks.len(), 1);
    }

    #[test]
    fn a_refused_operation_stops_everything_and_names_its_number() {
        let f = fixture();
        let mut edit = BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap();
        let ops = vec![
            EditOp::DeleteTrack { track: f.track },
            EditOp::DeleteTrack { track: f.track },
        ];
        let error = apply_operations(&mut edit, &ops).unwrap_err();
        assert_eq!(error.code, OPERATION_REFUSED);
        assert!(
            error.detail.starts_with("operation 2 was refused"),
            "{error}"
        );
        assert_eq!(edit.board().tracks.len(), 1);
    }
}
