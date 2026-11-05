// SPDX-License-Identifier: GPL-3.0-or-later
//! The file of operations that `phoenix schedit` applies to a schematic: JSON, either a list of
//! operations or an object with the list under `ops`. The operations are those of
//! `eh3d-phoenix-schematic` (`{"op": "set_value", "part": "...", "value": "4k7"}`).

use eh3d_phoenix_schematic::{EditOp, Editor, Id, Schematic};
use serde::Deserialize;

use crate::codes::{SchEditError, OPERATIONS_INVALID};

/// The most operations a file may hold.
pub const MAX_OPERATIONS: usize = 100_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationsFile {
    ops: Vec<EditOp>,
}

/// Reads a file of operations.
pub fn read_operations(text: &str) -> Result<Vec<EditOp>, SchEditError> {
    let bad = |e: serde_json::Error| {
        SchEditError::new(OPERATIONS_INVALID, format!("the file cannot be read: {e}"))
    };
    let ops = if text.trim_start().starts_with('[') {
        serde_json::from_str::<Vec<EditOp>>(text).map_err(bad)?
    } else {
        serde_json::from_str::<OperationsFile>(text)
            .map_err(bad)?
            .ops
    };
    if ops.len() > MAX_OPERATIONS {
        return Err(SchEditError::new(
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
    /// The ids that the operations made (symbols, parts, wires, labels, power ports, sheets).
    pub created: Vec<Id>,
    /// Parts before and after.
    pub parts: (usize, usize),
    /// Wires before and after.
    pub wires: (usize, usize),
    /// Nets before and after.
    pub nets: (usize, usize),
}

impl ApplyReport {
    /// The report as text.
    pub fn to_text(&self) -> String {
        format!(
            "operations applied: {}\nnew items: {}\nparts: {} -> {}\nwires: {} -> {}\nnets: {} -> {}\n",
            self.applied,
            self.created.len(),
            self.parts.0,
            self.parts.1,
            self.wires.0,
            self.wires.1,
            self.nets.0,
            self.nets.1
        )
    }
}

/// Applies the operations one after the other to a copy of the schematic. All or nothing: the first
/// operation that is refused ends the run with its number (from 1), and the schematic that was given
/// is never changed.
pub fn apply_operations(
    schematic: &Schematic,
    ops: &[EditOp],
) -> Result<(Schematic, ApplyReport), SchEditError> {
    let mut editor = Editor::with_limit(schematic.clone(), ops.len().max(1));
    let mut report = ApplyReport {
        parts: (schematic.parts.len(), schematic.parts.len()),
        wires: (schematic.wires.len(), schematic.wires.len()),
        nets: (schematic.nets.len(), schematic.nets.len()),
        ..ApplyReport::default()
    };
    for (index, op) in ops.iter().enumerate() {
        match editor.apply(op) {
            Ok(applied) => {
                report.applied += 1;
                report.created.extend(applied.created);
            }
            Err(e) => {
                return Err(SchEditError::new(
                    OPERATIONS_INVALID,
                    format!("operation {} was refused: {e}", index + 1),
                ));
            }
        }
    }
    let result = editor.into_schematic();
    report.parts.1 = result.parts.len();
    report.wires.1 = result.wires.len();
    report.nets.1 = result.nets.len();
    Ok((result, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;

    #[test]
    fn both_forms_of_the_file_are_read_and_nothing_else() {
        let f = fixture();
        let op = format!(r#"{{"op":"set_value","part":"{}","value":"4k7"}}"#, f.r1);
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
        let ops = vec![
            EditOp::SetValue {
                part: f.r1,
                value: "4k7".into(),
            },
            EditOp::Delete {
                items: vec![eh3d_phoenix_schematic::Item::Wire(f.wire)],
            },
        ];
        let (result, report) = apply_operations(&f.schematic, &ops).unwrap();
        assert_eq!(report.applied, 2);
        assert_eq!(report.wires, (1, 0));
        assert_eq!(report.nets, (1, 0));
        assert_eq!(report.parts, (2, 2));
        assert_eq!(result.parts[&f.r1].value, "4k7");
        assert!(report.to_text().contains("wires: 1 -> 0"));
        // the schematic that was given is as it was
        assert_eq!(f.schematic.parts[&f.r1].value, "10k");
    }

    #[test]
    fn a_refused_operation_stops_everything_and_names_its_number() {
        let f = fixture();
        let ops = vec![
            EditOp::SetValue {
                part: f.r1,
                value: "1".into(),
            },
            EditOp::SetRefdes {
                part: f.r2,
                refdes: "R1".into(),
            },
        ];
        let error = apply_operations(&f.schematic, &ops).unwrap_err();
        assert_eq!(error.code, OPERATIONS_INVALID);
        assert!(
            error.detail.starts_with("operation 2 was refused"),
            "{error}"
        );
    }
}
