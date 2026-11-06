// SPDX-License-Identifier: GPL-3.0-or-later
//! Assigning a part of the parts base to a symbol that is placed in the schematic, as a plan that
//! is read first and applied as one step that can be undone.
//!
//! The plan holds what the assignment would change in the schematic (the value of the part) and
//! what it would add to the data of the bill of materials (the maker and the part number, as the
//! line of the file of part data that `phoenix bom --parts` reads, and the footprint and the
//! simulation model the part asks for). Nothing is changed by making the plan. Applying it runs
//! one operation of the editor, so one undo gives the schematic back as it was.
//!
//! The schematic keeps the footprint of a part as the id of a footprint of the board, so the name
//! of the footprint in the plan is for the person to choose it; it is not written here.

use eh3d_phoenix_parts::codes::{PartsError, ASSIGN_REFUSED};
use eh3d_phoenix_parts::{norm_value, Part as BasePart, Status};
use eh3d_phoenix_schematic::{EditError, EditOp, Id, Schematic};

use crate::state::SchEdit;

/// What assigning one part of the base to one placed symbol would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignPlan {
    /// The placed part of the schematic.
    pub part: Id,
    /// Its reference designator.
    pub refdes: String,
    /// The id of the part of the base.
    pub base_id: String,
    /// The value it has now.
    pub old_value: String,
    /// The value the base gives it.
    pub new_value: String,
    /// The footprint the base asks for; empty when it does not say.
    pub footprint: String,
    /// The maker.
    pub manufacturer: String,
    /// The part number.
    pub mpn: String,
    /// The simulation model the base names; empty when it does not.
    pub sim_model: String,
    /// The state of the part in the base.
    pub status: Status,
}

impl AssignPlan {
    /// True when the value is the same (compared without case, blanks or the ohm sign), so the
    /// schematic does not change and only the data of the bill is added.
    pub fn keeps_value(&self) -> bool {
        norm_value(&self.old_value) == norm_value(&self.new_value)
    }

    /// The operations of the editor that apply the plan; none when the value stays.
    pub fn ops(&self) -> Vec<EditOp> {
        if self.keeps_value() {
            return Vec::new();
        }
        vec![EditOp::SetValue {
            part: self.part,
            value: self.new_value.clone(),
        }]
    }

    /// The line of the file of part data (`match,manufacturer,mpn,fitted`) for this part.
    pub fn parts_line(&self) -> String {
        format!(
            "{},{},{},yes",
            quote(&self.refdes),
            quote(&self.manufacturer),
            quote(&self.mpn)
        )
    }

    /// One line for a person: the part, what changes and what the base adds.
    pub fn describe(&self) -> String {
        let value = if self.keeps_value() {
            format!("value {} stays", self.old_value)
        } else {
            format!("value {} -> {}", self.old_value, self.new_value)
        };
        let mut text = format!("{} <- {}: {value}", self.refdes, self.base_id);
        if !self.manufacturer.is_empty() || !self.mpn.is_empty() {
            text.push_str(&format!(", {} {}", self.manufacturer, self.mpn));
        }
        if !self.footprint.is_empty() {
            text.push_str(&format!(", footprint {}", self.footprint));
        }
        if self.status != Status::Active {
            text.push_str(&format!(", {}", self.status.as_str()));
        }
        text
    }
}

fn quote(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// The plan of assigning a part of the base to a placed part of the schematic.
pub fn plan_assignment(
    schematic: &Schematic,
    part: Id,
    base: &BasePart,
) -> Result<AssignPlan, PartsError> {
    let placed = schematic.parts.get(&part).ok_or_else(|| {
        PartsError::new(ASSIGN_REFUSED, "the symbol is not a part of the schematic")
    })?;
    if base.value.trim().is_empty() {
        return Err(PartsError::new(
            ASSIGN_REFUSED,
            format!("the part {} of the base has no value", base.id),
        ));
    }
    Ok(AssignPlan {
        part,
        refdes: placed.refdes.clone(),
        base_id: base.id.clone(),
        old_value: placed.value.clone(),
        new_value: base.value.trim().to_owned(),
        footprint: base.footprint.clone(),
        manufacturer: base.manufacturer.clone(),
        mpn: base.mpn.clone(),
        sim_model: base.sim_model.clone(),
        status: base.status,
    })
}

/// The plans of assigning one part of the base to every part that is selected in the editor, in
/// the order of the ids. A selection with no part is refused.
pub fn plan_selected(edit: &SchEdit, base: &BasePart) -> Result<Vec<AssignPlan>, PartsError> {
    let selected = edit.selected_parts();
    if selected.is_empty() {
        return Err(PartsError::new(ASSIGN_REFUSED, "no part is selected"));
    }
    selected
        .into_iter()
        .map(|part| plan_assignment(edit.schematic(), part, base))
        .collect()
}

/// Applies plans as one step of the editor: one undo gives the schematic back. Returns whether
/// the schematic changed (it does not when every value stays).
pub fn apply_assignments(edit: &mut SchEdit, plans: &[AssignPlan]) -> Result<bool, EditError> {
    let ops: Vec<EditOp> = plans.iter().flat_map(AssignPlan::ops).collect();
    if ops.is_empty() {
        return Ok(false);
    }
    edit.run(&EditOp::Batch { ops })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SchEditSettings;
    use crate::testutil::{fixture, Fixture};
    use eh3d_phoenix_erc::ErcConfig;

    fn edit(f: &Fixture) -> SchEdit {
        SchEdit::new(
            f.schematic.clone(),
            SchEditSettings::default(),
            ErcConfig::default(),
        )
    }

    fn base(value: &str) -> BasePart {
        BasePart {
            id: "r-22k".into(),
            value: value.into(),
            footprint: "R0603".into(),
            manufacturer: "Acme, Inc".into(),
            mpn: "AC-22K".into(),
            sim_model: "res".into(),
            ..BasePart::default()
        }
    }

    #[test]
    fn the_plan_says_what_would_change_and_changes_nothing() {
        let f = fixture();
        let plan = plan_assignment(&f.schematic, f.r1, &base("22k")).unwrap();
        assert_eq!(plan.refdes, "R1");
        assert_eq!(
            (plan.old_value.as_str(), plan.new_value.as_str()),
            ("10k", "22k")
        );
        assert!(!plan.keeps_value());
        assert_eq!(plan.ops().len(), 1);
        assert_eq!(
            plan.describe(),
            "R1 <- r-22k: value 10k -> 22k, Acme, Inc AC-22K, footprint R0603"
        );
        assert_eq!(plan.parts_line(), "R1,\"Acme, Inc\",AC-22K,yes");
        assert_eq!(f.schematic.parts[&f.r1].value, "10k");
    }

    #[test]
    fn a_value_that_is_the_same_in_another_notation_stays_as_the_person_wrote_it() {
        let f = fixture();
        let plan = plan_assignment(&f.schematic, f.r1, &base("10 k\u{3a9}")).unwrap();
        assert!(plan.keeps_value());
        assert!(plan.ops().is_empty());
        let mut se = edit(&f);
        assert!(!apply_assignments(&mut se, &[plan]).unwrap());
        assert!(!se.can_undo());
        assert_eq!(se.schematic(), &f.schematic);
    }

    #[test]
    fn applying_is_one_step_and_one_undo_gives_the_schematic_back() {
        let f = fixture();
        let mut se = edit(&f);
        let plans = vec![
            plan_assignment(se.schematic(), f.r1, &base("22k")).unwrap(),
            plan_assignment(se.schematic(), f.r2, &base("22k")).unwrap(),
        ];
        assert!(apply_assignments(&mut se, &plans).unwrap());
        assert_eq!(se.schematic().parts[&f.r1].value, "22k");
        assert_eq!(se.schematic().parts[&f.r2].value, "22k");
        assert!(se.dirty());
        assert!(se.undo());
        assert_eq!(se.schematic(), &f.schematic);
        assert!(se.can_redo());
        assert!(se.redo());
        assert_eq!(se.schematic().parts[&f.r2].value, "22k");
    }

    #[test]
    fn the_selection_is_planned_and_an_empty_one_is_refused() {
        let f = fixture();
        let mut se = edit(&f);
        let e = plan_selected(&se, &base("22k")).unwrap_err();
        assert_eq!(e.code, ASSIGN_REFUSED);
        se.pointer_pressed(crate::testutil::pt(20_000_000, 20_000_000), false);
        let plans = plan_selected(&se, &base("22k")).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].part, f.r1);
    }

    #[test]
    fn what_cannot_be_assigned_is_refused() {
        let f = fixture();
        let e = plan_assignment(&f.schematic, f.r1, &base("  ")).unwrap_err();
        assert_eq!(e.code, ASSIGN_REFUSED);
        assert!(e.detail.contains("has no value"));
        let mut gone = *f.r1.as_bytes();
        gone[15] ^= 0x7f;
        let missing = Id::from_bytes(gone).unwrap();
        let e = plan_assignment(&f.schematic, missing, &base("22k")).unwrap_err();
        assert_eq!(e.code, ASSIGN_REFUSED);
        assert!(e.detail.contains("not a part of the schematic"));
    }
}
