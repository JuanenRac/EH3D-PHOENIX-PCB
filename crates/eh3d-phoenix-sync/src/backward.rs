// SPDX-License-Identifier: GPL-3.0-or-later
//! Backward annotation: the reference designators, values and footprints of the board are written
//! back into the parts of the schematic that they were made from.
//!
//! The parts are found by identity (`Component::symbol` holds the id of the part), never by name.
//! Every edit returns its exact inverse, built from the state before the change, so undoing is
//! applying the inverse; the history can be written and replayed. Nothing that is not named by an
//! edit is touched.

use std::collections::BTreeMap;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_schematic::{Id, Schematic};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::codes::{ChangeKind, FOOTPRINT_UNUSABLE, PART_TWICE, REFDES_CLASH};
use crate::report::{Change, Direction, Report, Skipped};
use crate::settings::SyncSettings;

/// A change to a schematic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SchematicEdit {
    /// Gives a part another reference designator.
    SetRefdes {
        /// The part.
        part: Id,
        /// The new reference designator.
        refdes: String,
    },
    /// Gives a part another value.
    SetValue {
        /// The part.
        part: Id,
        /// The new value.
        value: String,
    },
    /// Gives a part another footprint, or none.
    SetFootprint {
        /// The part.
        part: Id,
        /// The new footprint.
        footprint: Option<Id>,
    },
    /// Several edits that are applied and undone as one.
    Batch {
        /// The edits, in order.
        edits: Vec<SchematicEdit>,
    },
}

/// Why an edit could not be applied. The schematic is left as it was.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    /// The part to change does not exist.
    #[error("the part {0} does not exist")]
    MissingPart(Id),
}

impl SyncError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        crate::codes::NOT_APPLIED
    }
}

impl SchematicEdit {
    /// Applies the edit and returns the edit that undoes it. On an error the schematic is
    /// unchanged.
    pub fn apply(&self, schematic: &mut Schematic) -> Result<SchematicEdit, SyncError> {
        match self {
            SchematicEdit::SetRefdes { part, refdes } => {
                let p = schematic
                    .parts
                    .get_mut(part)
                    .ok_or(SyncError::MissingPart(*part))?;
                Ok(SchematicEdit::SetRefdes {
                    part: *part,
                    refdes: std::mem::replace(&mut p.refdes, refdes.clone()),
                })
            }
            SchematicEdit::SetValue { part, value } => {
                let p = schematic
                    .parts
                    .get_mut(part)
                    .ok_or(SyncError::MissingPart(*part))?;
                Ok(SchematicEdit::SetValue {
                    part: *part,
                    value: std::mem::replace(&mut p.value, value.clone()),
                })
            }
            SchematicEdit::SetFootprint { part, footprint } => {
                let p = schematic
                    .parts
                    .get_mut(part)
                    .ok_or(SyncError::MissingPart(*part))?;
                Ok(SchematicEdit::SetFootprint {
                    part: *part,
                    footprint: std::mem::replace(&mut p.footprint, *footprint),
                })
            }
            SchematicEdit::Batch { edits } => {
                // All or nothing: on a failure everything applied so far is undone.
                let mut inverses: Vec<SchematicEdit> = Vec::with_capacity(edits.len());
                for edit in edits {
                    match edit.apply(schematic) {
                        Ok(inverse) => inverses.push(inverse),
                        Err(error) => {
                            for undo in inverses.iter().rev() {
                                let _ = undo.apply(schematic);
                            }
                            return Err(error);
                        }
                    }
                }
                inverses.reverse();
                Ok(SchematicEdit::Batch { edits: inverses })
            }
        }
    }
}

/// One step of the history of a schematic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicStep {
    /// What was done.
    pub edit: SchematicEdit,
    /// What undoes it.
    pub inverse: SchematicEdit,
}

/// The undo/redo history of a schematic. It can be written and replayed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SchematicHistory {
    done: Vec<SchematicStep>,
    undone: Vec<SchematicStep>,
}

impl SchematicHistory {
    /// An empty history.
    pub fn new() -> SchematicHistory {
        SchematicHistory::default()
    }

    /// Applies `edit` to the schematic and records it. A new change clears what could be redone.
    pub fn execute(
        &mut self,
        schematic: &mut Schematic,
        edit: SchematicEdit,
    ) -> Result<(), SyncError> {
        let inverse = edit.apply(schematic)?;
        self.done.push(SchematicStep { edit, inverse });
        self.undone.clear();
        Ok(())
    }

    /// Undoes the last change; false when there is none.
    pub fn undo(&mut self, schematic: &mut Schematic) -> bool {
        let Some(step) = self.done.pop() else {
            return false;
        };
        if step.inverse.apply(schematic).is_err() {
            self.done.push(step);
            return false;
        }
        self.undone.push(step);
        true
    }

    /// Redoes the last undone change; false when there is none.
    pub fn redo(&mut self, schematic: &mut Schematic) -> bool {
        let Some(step) = self.undone.pop() else {
            return false;
        };
        if step.edit.apply(schematic).is_err() {
            self.undone.push(step);
            return false;
        }
        self.done.push(step);
        true
    }

    /// The changes that are done, oldest first.
    pub fn done(&self) -> &[SchematicStep] {
        &self.done
    }
}

/// The changes that make a schematic agree with a board, as a package: the differences, what could
/// not be done and the edits that apply it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackwardPackage {
    /// The differences that the edits remove, in a fixed order.
    pub changes: Vec<Change>,
    /// What could not be judged or done.
    pub skipped: Vec<Skipped>,
    /// The edits, in a fixed order.
    pub edits: Vec<SchematicEdit>,
}

impl BackwardPackage {
    /// True when the schematic already agrees with the board.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// The edits as one batch: one step of the history.
    pub fn edit(&self) -> SchematicEdit {
        SchematicEdit::Batch {
            edits: self.edits.clone(),
        }
    }

    /// Applies the package to the schematic as one step of `history`, so it can be undone and
    /// redone. Nothing happens, and nothing is recorded, when the package is empty.
    pub fn apply(
        &self,
        schematic: &mut Schematic,
        history: &mut SchematicHistory,
    ) -> Result<(), SyncError> {
        if self.is_empty() {
            return Ok(());
        }
        history.execute(schematic, self.edit())
    }

    /// The report: the changes and what was left out.
    pub fn report(&self) -> Report {
        Report::build(
            Direction::Backward,
            self.edits.len(),
            &self.changes,
            &self.skipped,
        )
    }

    /// The package as JSON: the changes, what was left out and the edits.
    pub fn to_json(&self) -> String {
        let value = json!({
            "schema": "phoenix-sync-package/1",
            "direction": Direction::Backward.as_str(),
            "changes": self.changes,
            "skipped": self.skipped,
            "edits": self.edits,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// Plans the writing back of the board into the schematic: reference designators, values and
/// footprints of the parts that have a component. The same schematic and board always give the
/// same package. Nothing is changed.
pub fn plan_backward(
    schematic: &Schematic,
    board: &Board,
    settings: &SyncSettings,
) -> BackwardPackage {
    let mut changes: Vec<Change> = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();

    // part -> its component; two components of one part are reported and the first is used
    let mut component_of: BTreeMap<Id, Id> = BTreeMap::new();
    for (cid, component) in &board.components {
        let Some(part_id) = component.symbol else {
            continue;
        };
        if !schematic.parts.contains_key(&part_id) {
            continue;
        }
        if component_of.contains_key(&part_id) {
            skipped.push(Skipped::new(
                PART_TWICE,
                &component.refdes,
                "a second component of the same part is left alone",
            ));
        } else {
            component_of.insert(part_id, *cid);
        }
    }

    // the reference designators the parts would have, to refuse a clash
    let mut wanted: BTreeMap<Id, String> = BTreeMap::new();
    for (part_id, part) in &schematic.parts {
        let refdes = match component_of
            .get(part_id)
            .and_then(|c| board.components.get(c))
        {
            Some(component) => component.refdes.clone(),
            None => part.refdes.clone(),
        };
        wanted.insert(*part_id, refdes);
    }
    let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
    for refdes in wanted.values() {
        *uses.entry(refdes.as_str()).or_default() += 1;
    }

    let mut edits: Vec<SchematicEdit> = Vec::new();
    for (part_id, cid) in &component_of {
        let (Some(part), Some(component)) =
            (schematic.parts.get(part_id), board.components.get(cid))
        else {
            continue;
        };
        if component.refdes != part.refdes {
            let clash = component.refdes.trim().is_empty()
                || uses.get(component.refdes.as_str()).copied().unwrap_or(0) > 1;
            if clash {
                skipped.push(Skipped::new(
                    REFDES_CLASH,
                    &part.refdes,
                    &format!("the board calls it {:?}", component.refdes),
                ));
            } else {
                changes.push(Change::new(
                    ChangeKind::RefdesDiffers,
                    &part.refdes,
                    &part.refdes,
                    &component.refdes,
                ));
                edits.push(SchematicEdit::SetRefdes {
                    part: *part_id,
                    refdes: component.refdes.clone(),
                });
            }
        }
        if component.value != part.value {
            changes.push(Change::new(
                ChangeKind::ValueDiffers,
                &part.refdes,
                &part.value,
                &component.value,
            ));
            edits.push(SchematicEdit::SetValue {
                part: *part_id,
                value: component.value.clone(),
            });
        }
        if settings.backward_footprints && part.footprint != Some(component.footprint) {
            if board.footprints.contains_key(&component.footprint) {
                let name_of = |id: Option<Id>| -> String {
                    id.and_then(|f| board.footprints.get(&f))
                        .map_or_else(|| "-".to_owned(), |f| f.name.clone())
                };
                changes.push(Change::new(
                    ChangeKind::FootprintDiffers,
                    &part.refdes,
                    &name_of(part.footprint),
                    &name_of(Some(component.footprint)),
                ));
                edits.push(SchematicEdit::SetFootprint {
                    part: *part_id,
                    footprint: Some(component.footprint),
                });
            } else {
                skipped.push(Skipped::new(
                    FOOTPRINT_UNUSABLE,
                    &part.refdes,
                    "the footprint of the component is not in the board",
                ));
            }
        }
    }
    changes.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    BackwardPackage {
        changes,
        skipped,
        edits,
    }
}
