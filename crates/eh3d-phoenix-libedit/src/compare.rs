// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of the footprints of two libraries.
//!
//! [`compare_libraries`] matches the footprints of two libraries by name and says which one library
//! has that the other has not and, for the footprints both have, what is different in their pads:
//! a pad that only one has, a pad that moved, a pad of another size, another kind (through-hole or
//! surface-mount), another drill or another copper layer. Only the pads are compared, and the
//! drawing items that are not pads are left out. A shift or a change of size of no more than the
//! tolerance is not a difference.
//!
//! The comparison reads both libraries and changes nothing. All the arithmetic is on the integer
//! nanometres of the model, so the answer is exact and the same every time.

use std::collections::BTreeMap;

use crate::model::{Item, Library, Package};

/// What the comparison needs to know of a pad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadSummary {
    /// The name of the pad.
    pub name: String,
    /// `pad` for a through-hole pad and `smd` for a surface-mount pad.
    pub kind: &'static str,
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The width of the copper of an SMD pad, and the diameter of the copper of a through-hole
    /// pad (0 when the design rules decide).
    pub width: i64,
    /// The height of an SMD pad, and the diameter of a through-hole pad again.
    pub height: i64,
    /// The drill of a through-hole pad; 0 for an SMD pad.
    pub drill: i64,
    /// The copper layer of an SMD pad; 0 for a through-hole pad.
    pub layer: u32,
}

/// The pads of a footprint, in the order of the file.
pub fn pad_summaries(package: &Package) -> Vec<PadSummary> {
    package
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Pad(p) => Some(PadSummary {
                name: p.name.clone(),
                kind: "pad",
                x: p.x,
                y: p.y,
                width: p.diameter,
                height: p.diameter,
                drill: p.drill,
                layer: 0,
            }),
            Item::Smd(s) => Some(PadSummary {
                name: s.name.clone(),
                kind: "smd",
                x: s.x,
                y: s.y,
                width: s.dx,
                height: s.dy,
                drill: 0,
                layer: s.layer,
            }),
            _ => None,
        })
        .collect()
}

fn far(a: i64, b: i64, tolerance: i64) -> bool {
    (i128::from(a) - i128::from(b)).abs() > i128::from(tolerance)
}

/// What differs between two lists of pads, as sentences in the order of the pad names; empty when
/// the pads are the same within the tolerance. A name that is used by two pads of a list is
/// compared by its first pad and noted.
pub fn compare_pads(old: &[PadSummary], new: &[PadSummary], tolerance_nm: i64) -> Vec<String> {
    let tolerance = tolerance_nm.max(0);
    let mut out = Vec::new();
    let mut first_old: BTreeMap<&str, &PadSummary> = BTreeMap::new();
    let mut first_new: BTreeMap<&str, &PadSummary> = BTreeMap::new();
    for (list, map, side) in [(old, &mut first_old, "old"), (new, &mut first_new, "new")] {
        for pad in list {
            if map.insert(pad.name.as_str(), pad).is_some() {
                // The later pad of the same name replaced the first; put the first back.
                if let Some(first) = list.iter().find(|p| p.name == pad.name) {
                    map.insert(pad.name.as_str(), first);
                }
                out.push(format!(
                    "pad {} is named twice in the {side} footprint",
                    pad.name
                ));
            }
        }
    }
    let names: std::collections::BTreeSet<&str> =
        first_old.keys().chain(first_new.keys()).copied().collect();
    for name in names {
        match (first_old.get(name), first_new.get(name)) {
            (Some(_), None) => out.push(format!("pad {name} is only in the old footprint")),
            (None, Some(_)) => out.push(format!("pad {name} is only in the new footprint")),
            (Some(a), Some(b)) => {
                if a.kind != b.kind {
                    out.push(format!(
                        "pad {name} is {} in the old footprint and {} in the new",
                        a.kind, b.kind
                    ));
                    continue;
                }
                if far(a.x, b.x, tolerance) || far(a.y, b.y, tolerance) {
                    out.push(format!(
                        "pad {name} moved from ({}, {}) to ({}, {})",
                        a.x, a.y, b.x, b.y
                    ));
                }
                if far(a.width, b.width, tolerance) || far(a.height, b.height, tolerance) {
                    out.push(format!(
                        "pad {name} changed size from {} x {} to {} x {}",
                        a.width, a.height, b.width, b.height
                    ));
                }
                if far(a.drill, b.drill, tolerance) {
                    out.push(format!(
                        "pad {name} changed drill from {} to {}",
                        a.drill, b.drill
                    ));
                }
                if a.layer != b.layer {
                    out.push(format!(
                        "pad {name} changed layer from {} to {}",
                        a.layer, b.layer
                    ));
                }
            }
            (None, None) => {}
        }
    }
    out
}

/// A footprint that both libraries have and whose pads differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootprintChange {
    /// The name of the footprint.
    pub name: String,
    /// What differs, as sentences.
    pub differences: Vec<String>,
}

/// What the comparison of two libraries found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryComparison {
    /// The names of the footprints only the new library has, in alphabetical order.
    pub added: Vec<String>,
    /// The names of the footprints only the old library has, in alphabetical order.
    pub removed: Vec<String>,
    /// The footprints both have whose pads differ, in alphabetical order of the name.
    pub changed: Vec<FootprintChange>,
    /// How many footprints both have whose pads are the same.
    pub same: usize,
}

impl LibraryComparison {
    /// True when the libraries have the same footprints with the same pads.
    pub fn is_same(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Compares the footprints of two libraries by name.
pub fn compare_libraries(old: &Library, new: &Library, tolerance_nm: i64) -> LibraryComparison {
    let by_name = |lib: &Library| -> BTreeMap<String, Vec<PadSummary>> {
        let mut map = BTreeMap::new();
        for package in &lib.packages {
            // The first footprint of a name is the one that counts.
            map.entry(package.name.clone())
                .or_insert_with(|| pad_summaries(package));
        }
        map
    };
    let (old_map, new_map) = (by_name(old), by_name(new));
    let mut result = LibraryComparison::default();
    for (name, old_pads) in &old_map {
        match new_map.get(name) {
            None => result.removed.push(name.clone()),
            Some(new_pads) => {
                let differences = compare_pads(old_pads, new_pads, tolerance_nm);
                if differences.is_empty() {
                    result.same += 1;
                } else {
                    result.changed.push(FootprintChange {
                        name: name.clone(),
                        differences,
                    });
                }
            }
        }
    }
    for name in new_map.keys() {
        if !old_map.contains_key(name) {
            result.added.push(name.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model::{Pad, Smd};

    fn smd(name: &str, x: i64, y: i64, dx: i64, dy: i64) -> Item {
        Item::Smd(Smd {
            name: name.to_owned(),
            x,
            y,
            dx,
            dy,
            ..Smd::default()
        })
    }

    fn package(name: &str, items: Vec<Item>) -> Arc<Package> {
        Arc::new(Package {
            name: name.to_owned(),
            items,
            ..Package::default()
        })
    }

    fn library(packages: Vec<Arc<Package>>) -> Library {
        Library {
            packages,
            ..Library::empty()
        }
    }

    fn chip(name: &str, shift: i64) -> Arc<Package> {
        package(
            name,
            vec![
                smd("1", -500_000 + shift, 0, 600_000, 700_000),
                smd("2", 500_000, 0, 600_000, 700_000),
            ],
        )
    }

    #[test]
    fn the_same_libraries_are_the_same() {
        let a = library(vec![chip("R0603", 0), chip("R0805", 0)]);
        let found = compare_libraries(&a, &a.clone(), 0);
        assert!(found.is_same());
        assert_eq!(found.same, 2);
    }

    #[test]
    fn footprints_that_only_one_library_has_are_listed_in_order() {
        let old = library(vec![chip("B", 0), chip("A", 0)]);
        let new = library(vec![chip("D", 0), chip("C", 0), chip("A", 0)]);
        let found = compare_libraries(&old, &new, 0);
        assert_eq!(found.removed, vec!["B".to_owned()]);
        assert_eq!(found.added, vec!["C".to_owned(), "D".to_owned()]);
        assert_eq!(found.same, 1);
        assert!(!found.is_same());
    }

    #[test]
    fn a_moved_pad_is_named_unless_it_moved_within_the_tolerance() {
        let old = library(vec![chip("R0603", 0)]);
        let new = library(vec![chip("R0603", 10_000)]);
        let found = compare_libraries(&old, &new, 5_000);
        assert_eq!(found.changed.len(), 1);
        assert_eq!(
            found.changed[0].differences,
            vec!["pad 1 moved from (-500000, 0) to (-490000, 0)".to_owned()]
        );
        assert!(compare_libraries(&old, &new, 10_000).is_same());
        assert!(!compare_libraries(&old, &new, 9_999).is_same());
        // A tolerance below zero is read as zero.
        assert!(!compare_libraries(&old, &new, -5).is_same());
    }

    #[test]
    fn size_drill_kind_and_layer_are_compared() {
        let through = |name: &str, drill: i64, diameter: i64| {
            Item::Pad(Pad {
                name: name.to_owned(),
                drill,
                diameter,
                ..Pad::default()
            })
        };
        let old = library(vec![package(
            "P",
            vec![
                through("1", 800_000, 1_600_000),
                smd("2", 0, 0, 1_000_000, 1_000_000),
            ],
        )]);
        let new = library(vec![package(
            "P",
            vec![
                through("1", 900_000, 1_800_000),
                Item::Smd(Smd {
                    name: "2".to_owned(),
                    dx: 1_000_000,
                    dy: 1_200_000,
                    layer: 16,
                    ..Smd::default()
                }),
            ],
        )]);
        let found = compare_libraries(&old, &new, 0);
        let all = found.changed[0].differences.join("\n");
        assert!(all.contains("pad 1 changed size from 1600000 x 1600000 to 1800000 x 1800000"));
        assert!(all.contains("pad 1 changed drill from 800000 to 900000"));
        assert!(all.contains("pad 2 changed size from 1000000 x 1000000 to 1000000 x 1200000"));
        assert!(all.contains("pad 2 changed layer from 1 to 16"));
        let swapped = library(vec![package(
            "P",
            vec![smd("1", 0, 0, 1, 1), smd("2", 0, 0, 1, 1)],
        )]);
        let kind = compare_libraries(&old, &swapped, 0);
        assert!(kind.changed[0]
            .differences
            .contains(&"pad 1 is pad in the old footprint and smd in the new".to_owned()));
    }

    #[test]
    fn pads_that_only_one_footprint_has_and_names_used_twice() {
        let old = library(vec![package(
            "P",
            vec![smd("1", 0, 0, 1, 1), smd("1", 5, 5, 1, 1)],
        )]);
        let new = library(vec![package(
            "P",
            vec![smd("1", 0, 0, 1, 1), smd("2", 0, 0, 1, 1)],
        )]);
        let found = compare_libraries(&old, &new, 0);
        let all = found.changed[0].differences.join("\n");
        assert!(
            all.contains("pad 1 is named twice in the old footprint"),
            "{all}"
        );
        assert!(all.contains("pad 2 is only in the new footprint"));
        assert!(!all.contains("moved"), "{all}");
    }

    #[test]
    fn the_far_corners_of_the_range_do_not_overflow() {
        let edge = i64::MAX;
        let a = [PadSummary {
            name: "1".to_owned(),
            kind: "smd",
            x: edge,
            y: i64::MIN,
            width: 0,
            height: 0,
            drill: 0,
            layer: 1,
        }];
        let mut b = a.clone();
        b[0].x = i64::MIN;
        assert_eq!(compare_pads(&a, &b, 0).len(), 1);
        assert!(compare_pads(&a, &a, i64::MAX).is_empty());
    }
}
