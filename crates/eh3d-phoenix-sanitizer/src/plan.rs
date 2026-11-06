// SPDX-License-Identifier: GPL-3.0-or-later
//! The master-library plan: for each group, the item that would be kept and the items that would
//! be replaced by it. The plan is a file for the owner to read and to edit. It is applied by
//! [`crate::apply`], to a copy of the libraries and only with the confirmation code of the plan;
//! the checks listed in the plan are the ones that application makes for every step.

use serde::Serialize;

use crate::report::Report;

/// The format tag of the plan file.
pub const PLAN_FORMAT: &str = "phoenix-sanitize-plan/1";

/// A check that has to pass before a plan is applied: an identifier and what it proves.
pub const PRECONDITIONS: [(&str, &str); 8] = [
    (
        "confirmed",
        "the owner confirmed the group, one by one, by its id; no group is applied because of its score alone",
    ),
    (
        "backup",
        "every file that would change was copied first and the SHA-256 of the copy equals the one of the original",
    ),
    (
        "footprint-same",
        "after the replacement every placed part keeps the position, size, shape and layer of each pad, checked against the canonical hash of the footprint",
    ),
    (
        "pad-names",
        "the pads and pins of the kept item are named the same way as those of the replaced one, and the pin-to-pad connections of every device set are the same",
    ),
    (
        "no-turn",
        "no replacement needs a turn, a mirror or a shift of the part (the members marked turned are left out)",
    ),
    (
        "layout-unchanged",
        "the positions, rotations, sides and nets of every placed part of every board and schematic that uses the library are the same before and after",
    ),
    (
        "rules-unchanged",
        "the design rule check and the connectivity check of every board give the same findings before and after",
    ),
    (
        "reread",
        "the written libraries are read back and give the hashes the plan promised",
    ),
];

/// A reference to one item of one library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemRef {
    /// The library file, relative to the folder that was analysed.
    pub library: String,
    /// The name in that library.
    pub name: String,
}

/// One item that would be replaced by the kept one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Replacement {
    /// The library file.
    pub library: String,
    /// The name.
    pub name: String,
    /// The score against the item to keep.
    pub score: u32,
    /// True when the geometry is the same as the item to keep.
    pub exact: bool,
    /// True when the replacement would have to turn, mirror or shift the part.
    pub turned: bool,
}

/// The plan of one group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanGroup {
    /// The id of the group in the report.
    pub id: String,
    /// `package`, `symbol` or `deviceset`.
    pub kind: String,
    /// The item to keep.
    pub keep: ItemRef,
    /// The items that would be replaced by it.
    pub replace: Vec<Replacement>,
    /// What the owner has to look at before confirming: `below-threshold`, `names-differ`,
    /// `turned`, `not-exact`.
    pub review: Vec<String>,
}

/// One check of the list that has to pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Precondition {
    /// The identifier.
    pub id: String,
    /// What it proves.
    pub text: String,
}

/// The plan file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// The format tag.
    pub format: String,
    /// Always false: the file of a plan is never changed; applying it makes a copy of the
    /// libraries.
    pub applied: bool,
    /// The threshold of the report the plan comes from.
    pub threshold: u32,
    /// The checks to pass before the plan is applied.
    pub preconditions: Vec<Precondition>,
    /// The groups.
    pub groups: Vec<PlanGroup>,
}

impl Plan {
    /// The plan as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// The plan of a report.
pub fn plan_of(report: &Report) -> Plan {
    let mut groups = Vec::with_capacity(report.groups.len());
    for g in &report.groups {
        let Some(keep) = g.keep() else { continue };
        let mut review = Vec::new();
        if g.weak {
            review.push("below-threshold".to_owned());
        }
        if !g.names_agree {
            review.push("names-differ".to_owned());
        }
        if g.members.iter().any(|m| m.turned) {
            review.push("turned".to_owned());
        }
        if !g.exact {
            review.push("not-exact".to_owned());
        }
        groups.push(PlanGroup {
            id: g.id.clone(),
            kind: g.kind.clone(),
            keep: ItemRef {
                library: keep.library.clone(),
                name: keep.name.clone(),
            },
            replace: g
                .members
                .iter()
                .filter(|m| !m.keep)
                .map(|m| Replacement {
                    library: m.library.clone(),
                    name: m.name.clone(),
                    score: m.score,
                    exact: m.exact,
                    turned: m.turned,
                })
                .collect(),
            review,
        });
    }
    Plan {
        format: PLAN_FORMAT.to_owned(),
        applied: false,
        threshold: report.settings.threshold,
        preconditions: PRECONDITIONS
            .iter()
            .map(|(id, text)| Precondition {
                id: (*id).to_owned(),
                text: (*text).to_owned(),
            })
            .collect(),
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::analyse_libraries;
    use crate::scan::{load_bytes, LoadedLibrary};
    use crate::settings::SanitizerSettings;
    use crate::testdata::{library_xml, package_xml};

    fn lib(path: &str, text: &str, s: &SanitizerSettings) -> LoadedLibrary {
        let mut l = LoadedLibrary {
            path: path.into(),
            bytes: 0,
            sha256: String::new(),
            version: String::new(),
            data: None,
            notes: Vec::new(),
        };
        load_bytes(&mut l, text.as_bytes(), s);
        l
    }

    #[test]
    fn the_plan_names_what_is_kept_and_what_is_replaced_and_is_never_applied() {
        let s = SanitizerSettings::default();
        let two = [(-0.9, 0.0), (0.9, 0.0)];
        let libs = [
            lib(
                "a.lbr",
                &library_xml(&[package_xml("A", &two, 1.2, 1.4)], &[], &[]),
                &s,
            ),
            lib(
                "b.lbr",
                &library_xml(&[package_xml("B", &two, 1.2, 1.4)], &[], &[]),
                &s,
            ),
            lib(
                "c.lbr",
                &library_xml(
                    &[package_xml("C", &[(-0.92, 0.0), (0.9, 0.0)], 1.2, 1.4)],
                    &[],
                    &[],
                ),
                &s,
            ),
        ];
        let report = analyse_libraries(&libs, Vec::new(), 0, &s);
        let plan = plan_of(&report);
        assert_eq!(plan.format, PLAN_FORMAT);
        assert!(!plan.applied);
        assert_eq!(plan.threshold, 90);
        assert_eq!(plan.groups.len(), 1);
        let g = &plan.groups[0];
        assert_eq!(g.id, "P001");
        assert_eq!(
            g.keep,
            ItemRef {
                library: "a.lbr".into(),
                name: "A".into()
            }
        );
        assert_eq!(g.replace.len(), 2);
        assert!(g
            .replace
            .iter()
            .any(|r| r.name == "B" && r.exact && r.score == 100));
        assert!(g
            .replace
            .iter()
            .any(|r| r.name == "C" && !r.exact && r.score < 100));
        // one member is close and not equal: the owner has to look at it
        assert_eq!(g.review, vec!["not-exact".to_string()]);
        let ids: Vec<&str> = plan.preconditions.iter().map(|p| p.id.as_str()).collect();
        assert!(
            ids.contains(&"backup")
                && ids.contains(&"layout-unchanged")
                && ids.contains(&"rules-unchanged")
        );
        let json = plan.to_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["applied"], false);
        assert_eq!(plan_of(&report).to_json(), json);
    }

    #[test]
    fn a_report_without_groups_gives_a_plan_without_groups() {
        let s = SanitizerSettings::default();
        let report = analyse_libraries(&[], Vec::new(), 0, &s);
        let plan = plan_of(&report);
        assert!(plan.groups.is_empty());
        assert_eq!(plan.preconditions.len(), PRECONDITIONS.len());
    }
}
