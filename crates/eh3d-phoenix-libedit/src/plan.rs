// SPDX-License-Identifier: GPL-3.0-or-later
//! Replacing an item of a library by the master candidate that the sanitizer proposes, as a plan.
//! Reading the plan of the sanitizer for one library gives the steps that would replace its
//! footprints and symbols; nothing is changed until the owner confirms a step, and then only a
//! step that cannot move a part: the names of the pads or pins must be the same, and a step that
//! needs a turn, a mirror or a shift is left out.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use eh3d_phoenix_sanitizer::Plan;
use serde::Serialize;
use serde_json::Value;

use crate::codes::{LibError, NOT_CONFIRMED, NOT_FOUND, OPERATIONS_INVALID, VALUE_INVALID};
use crate::model::{Item, Kind, Library};

/// The format tag of the plan file of the sanitizer that is read here.
pub const SANITIZER_PLAN_FORMAT: &str = "phoenix-sanitize-plan/1";
/// The format tag of the plan written here.
pub const REPLACE_PLAN_FORMAT: &str = "phoenix-libedit-replace-plan/1";

/// One replacement of an item of the edited library by the master item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplaceStep {
    /// The id of the group in the report of the sanitizer.
    pub group: String,
    /// `package`, `symbol` or `deviceset`.
    pub kind: String,
    /// The name in the edited library.
    pub name: String,
    /// The library file of the master item, relative to the folder that was analysed.
    pub master_library: String,
    /// The name of the master item.
    pub master_name: String,
    /// The score against the master item.
    pub score: u32,
    /// True when the geometry is the same as the master item.
    pub exact: bool,
    /// True when the replacement would have to turn, mirror or shift the part.
    pub turned: bool,
    /// What the owner has to look at before confirming.
    pub review: Vec<String>,
}

/// The steps of the plan of the sanitizer that concern one library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplacePlan {
    /// The format tag.
    pub format: String,
    /// Always false: a plan is not applied by reading it.
    pub applied: bool,
    /// Always true: no step is made without the confirmation of the owner.
    pub needs_confirmation: bool,
    /// The library the steps are about.
    pub library: String,
    /// The steps.
    pub steps: Vec<ReplaceStep>,
}

impl ReplacePlan {
    /// The plan as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn bad(what: impl Into<String>) -> LibError {
    LibError::new(OPERATIONS_INVALID, what)
}

fn text_of<'a>(value: &'a Value, key: &str) -> Result<&'a str, LibError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| bad(format!("the plan has no text {key}")))
}

fn same_library(in_plan: &str, library: &str) -> bool {
    in_plan == library
        || match (
            Path::new(in_plan).file_name(),
            Path::new(library).file_name(),
        ) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
}

/// The steps of a plan file of the sanitizer (its JSON) that replace items of `library`.
pub fn replace_plan_from_json(text: &str, library: &str) -> Result<ReplacePlan, LibError> {
    let value: Value = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
    if value.get("format").and_then(Value::as_str) != Some(SANITIZER_PLAN_FORMAT) {
        return Err(bad("this is not a plan of the sanitizer"));
    }
    let groups = value
        .get("groups")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("the plan has no groups"))?;
    let mut steps = Vec::new();
    for group in groups {
        let keep = group
            .get("keep")
            .ok_or_else(|| bad("a group has no item to keep"))?;
        let master_library = text_of(keep, "library")?;
        let master_name = text_of(keep, "name")?;
        let review: Vec<String> = group
            .get("review")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let replaced = group
            .get("replace")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("a group has nothing to replace"))?;
        for item in replaced {
            if !same_library(text_of(item, "library")?, library) {
                continue;
            }
            steps.push(ReplaceStep {
                group: text_of(group, "id")?.to_owned(),
                kind: text_of(group, "kind")?.to_owned(),
                name: text_of(item, "name")?.to_owned(),
                master_library: master_library.to_owned(),
                master_name: master_name.to_owned(),
                score: item
                    .get("score")
                    .and_then(Value::as_u64)
                    .and_then(|s| u32::try_from(s).ok())
                    .unwrap_or(0),
                exact: item.get("exact").and_then(Value::as_bool).unwrap_or(false),
                turned: item.get("turned").and_then(Value::as_bool).unwrap_or(false),
                review: review.clone(),
            });
        }
    }
    steps.sort_by(|a, b| (&a.kind, &a.name, &a.group).cmp(&(&b.kind, &b.name, &b.group)));
    Ok(ReplacePlan {
        format: REPLACE_PLAN_FORMAT.to_owned(),
        applied: false,
        needs_confirmation: true,
        library: library.to_owned(),
        steps,
    })
}

/// The steps of a plan of the sanitizer that replace items of `library`.
pub fn replace_plan(plan: &Plan, library: &str) -> Result<ReplacePlan, LibError> {
    replace_plan_from_json(&plan.to_json(), library)
}

fn pad_names(items: &[Item]) -> BTreeSet<String> {
    items
        .iter()
        .filter(|i| i.is_pad())
        .filter_map(|i| i.name().map(str::to_owned))
        .collect()
}

fn pin_names(items: &[Item]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Pin(p) => Some(p.name.clone()),
            _ => None,
        })
        .collect()
}

/// Replaces the item of the step in `lib` by the master item of `master`, keeping the name it
/// has in `lib`. Nothing is done unless `confirmed` is true, and a step that turns the part, a
/// device set, or an item whose pads or pins are not named the same way is refused. The items
/// that use the replaced one keep working because the name stays.
pub fn apply_step(
    lib: &mut Library,
    step: &ReplaceStep,
    master: &Library,
    confirmed: bool,
) -> Result<(), LibError> {
    if !confirmed {
        return Err(LibError::new(
            NOT_CONFIRMED,
            format!("{} {} by {}", step.kind, step.name, step.master_name),
        ));
    }
    if step.turned {
        return Err(LibError::new(
            VALUE_INVALID,
            "the replacement needs a turn, a mirror or a shift of the part",
        ));
    }
    let missing = |what: &str, name: &str| LibError::new(NOT_FOUND, format!("{what} {name}"));
    match step.kind.as_str() {
        "package" => {
            let i = lib
                .package_index(&step.name)
                .ok_or_else(|| missing("footprint", &step.name))?;
            let m = master
                .package(&step.master_name)
                .ok_or_else(|| missing("master footprint", &step.master_name))?;
            if pad_names(&lib.packages[i].items) != pad_names(&m.items) {
                return Err(LibError::new(
                    VALUE_INVALID,
                    "the pads of the master are not named the same way",
                ));
            }
            let mut replacement = m.clone();
            replacement.name = step.name.clone();
            lib.packages[i] = Arc::new(replacement);
        }
        "symbol" => {
            let i = lib
                .symbol_index(&step.name)
                .ok_or_else(|| missing("symbol", &step.name))?;
            let m = master
                .symbol(&step.master_name)
                .ok_or_else(|| missing("master symbol", &step.master_name))?;
            if pin_names(&lib.symbols[i].items) != pin_names(&m.items) {
                return Err(LibError::new(
                    VALUE_INVALID,
                    "the pins of the master are not named the same way",
                ));
            }
            let mut replacement = m.clone();
            replacement.name = step.name.clone();
            lib.symbols[i] = Arc::new(replacement);
        }
        other => {
            return Err(LibError::new(
                VALUE_INVALID,
                format!("{other}: only footprints and symbols are replaced"),
            ))
        }
    }
    Ok(())
}

/// The kind of a step as the kind of the model, when it is one that can be replaced.
pub fn step_kind(step: &ReplaceStep) -> Option<Kind> {
    match step.kind.as_str() {
        "package" => Some(Kind::Package),
        "symbol" => Some(Kind::Symbol),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{apply_all, Operation, Target};
    use crate::read::tests::sample;

    const PLAN: &str = r#"{
  "format": "phoenix-sanitize-plan/1",
  "applied": false,
  "threshold": 90,
  "preconditions": [],
  "groups": [
    {"id": "P001", "kind": "package",
     "keep": {"library": "master.lbr", "name": "R0603_M"},
     "replace": [
       {"library": "old/mine.lbr", "name": "R0603", "score": 100, "exact": true, "turned": false},
       {"library": "old/other.lbr", "name": "RX", "score": 95, "exact": false, "turned": false}],
     "review": ["names-differ"]},
    {"id": "S001", "kind": "symbol",
     "keep": {"library": "master.lbr", "name": "R_M"},
     "replace": [{"library": "mine.lbr", "name": "R", "score": 91, "exact": false, "turned": true}],
     "review": []}
  ]
}"#;

    fn done(result: Result<Library, (usize, LibError)>) -> Library {
        match result {
            Ok(lib) => lib,
            Err((i, e)) => panic!("{i}: {e}"),
        }
    }

    /// The sample with its footprint renamed as the master and a master symbol.
    fn master() -> Library {
        done(apply_all(
            &sample(),
            &[
                Operation::RenamePackage {
                    from: "R0603".into(),
                    to: "R0603_M".into(),
                },
                Operation::CreateSymbol { name: "R_M".into() },
            ],
        ))
    }

    #[test]
    fn the_steps_of_one_library_are_taken_from_the_plan_of_the_sanitizer() {
        let plan = replace_plan_from_json(PLAN, "mine.lbr").unwrap();
        assert!(!plan.applied && plan.needs_confirmation);
        assert_eq!(plan.steps.len(), 2);
        let first = &plan.steps[0];
        assert_eq!(
            (
                first.kind.as_str(),
                first.name.as_str(),
                first.master_name.as_str()
            ),
            ("package", "R0603", "R0603_M")
        );
        assert!(first.exact && !first.turned);
        assert_eq!(first.review, vec!["names-differ".to_owned()]);
        assert!(plan.steps[1].turned);
        let other = replace_plan_from_json(PLAN, "other.lbr").unwrap();
        assert_eq!(other.steps.len(), 1);
        assert_eq!(other.steps[0].name, "RX");
        assert!(replace_plan_from_json(PLAN, "nothing.lbr")
            .unwrap()
            .steps
            .is_empty());
        let json = plan.to_json();
        assert!(
            json.contains("\"applied\": false") && json.contains("\"needs_confirmation\": true")
        );
    }

    #[test]
    fn a_file_that_is_not_a_plan_is_refused() {
        for bad in ["{}", "[]", "nonsense", r#"{"format":"x"}"#] {
            assert_eq!(
                replace_plan_from_json(bad, "a.lbr").unwrap_err().code,
                OPERATIONS_INVALID,
                "{bad}"
            );
        }
        let no_groups = r#"{"format":"phoenix-sanitize-plan/1"}"#;
        assert!(replace_plan_from_json(no_groups, "a.lbr").is_err());
    }

    #[test]
    fn nothing_is_replaced_without_the_confirmation() {
        let plan = replace_plan_from_json(PLAN, "mine.lbr").unwrap();
        let mut lib = sample();
        let before = lib.clone();
        let e = apply_step(&mut lib, &plan.steps[0], &master(), false).unwrap_err();
        assert_eq!(e.code, NOT_CONFIRMED);
        assert_eq!(lib, before);
    }

    #[test]
    fn a_confirmed_step_replaces_the_item_and_keeps_its_name() {
        let plan = replace_plan_from_json(PLAN, "mine.lbr").unwrap();
        let mut master = master();
        // give the master footprint something the old one does not have
        master = done(apply_all(
            &master,
            &[Operation::AddItem {
                target: Target::package("R0603_M"),
                item: Item::Hole(crate::model::Hole {
                    drill: 500_000,
                    ..crate::model::Hole::default()
                }),
                at: None,
            }],
        ));
        let mut lib = sample();
        apply_step(&mut lib, &plan.steps[0], &master, true).unwrap();
        let p = lib.package("R0603").unwrap();
        assert_eq!(p.name, "R0603");
        assert!(p.items.iter().any(|i| matches!(i, Item::Hole(_))));
        assert_eq!(
            lib.deviceset("R").unwrap().devices[0].package.as_deref(),
            Some("R0603")
        );
    }

    #[test]
    fn a_step_that_turns_the_part_or_renames_its_pads_is_refused() {
        let plan = replace_plan_from_json(PLAN, "mine.lbr").unwrap();
        let mut lib = sample();
        let e = apply_step(&mut lib, &plan.steps[1], &master(), true).unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
        // a master whose pads have other names
        let renamed = done(apply_all(
            &master(),
            &[Operation::ReplaceItem {
                target: Target::package("R0603_M"),
                index: 0,
                item: Item::Smd(crate::model::Smd {
                    name: "A".into(),
                    dx: 800_000,
                    dy: 900_000,
                    ..crate::model::Smd::default()
                }),
            }],
        ));
        let e = apply_step(&mut lib, &plan.steps[0], &renamed, true).unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
        assert!(e.detail.contains("pads"));
        let missing = ReplaceStep {
            name: "GONE".into(),
            ..plan.steps[0].clone()
        };
        let e = apply_step(&mut lib, &missing, &master(), true).unwrap_err();
        assert_eq!(e.code, NOT_FOUND);
        let set = ReplaceStep {
            kind: "deviceset".into(),
            ..plan.steps[0].clone()
        };
        assert_eq!(
            apply_step(&mut lib, &set, &master(), true)
                .unwrap_err()
                .code,
            VALUE_INVALID
        );
        assert_eq!(step_kind(&plan.steps[0]), Some(Kind::Package));
        assert_eq!(step_kind(&set), None);
    }
}
