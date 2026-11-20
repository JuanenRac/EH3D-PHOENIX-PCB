// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests with a fixed seed of the comparison of two libraries of footprints: the same cases on
//! every run.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::compare::compare_libraries;
use crate::model::{Item, Library, Package, Smd};

/// A small generator that always gives the same numbers for the same seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn range(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }
}

fn random_package(rng: &mut Lcg, name: String) -> Package {
    let pads = rng.range(1, 8);
    let items = (0..pads)
        .map(|n| {
            Item::Smd(Smd {
                name: (n + 1).to_string(),
                x: rng.range(-5_000_000, 5_000_000),
                y: rng.range(-5_000_000, 5_000_000),
                dx: rng.range(200_000, 2_000_000),
                dy: rng.range(200_000, 2_000_000),
                ..Smd::default()
            })
        })
        .collect();
    Package {
        name,
        items,
        ..Package::default()
    }
}

fn random_library(rng: &mut Lcg, pool: usize) -> Library {
    let mut names = BTreeSet::new();
    for _ in 0..rng.range(0, 12) {
        names.insert(format!("FP{}", rng.range(0, pool as i64 - 1)));
    }
    Library {
        packages: names
            .into_iter()
            .map(|n| Arc::new(random_package(rng, n)))
            .collect(),
        ..Library::empty()
    }
}

/// A copy of a library with every pad moved by `shift` along x.
fn shifted(lib: &Library, shift: i64) -> Library {
    Library {
        packages: lib
            .packages
            .iter()
            .map(|p| {
                let items = p
                    .items
                    .iter()
                    .map(|i| match i {
                        Item::Smd(s) => Item::Smd(Smd {
                            x: s.x + shift,
                            ..s.clone()
                        }),
                        other => other.clone(),
                    })
                    .collect();
                Arc::new(Package {
                    items,
                    ..(**p).clone()
                })
            })
            .collect(),
        ..Library::empty()
    }
}

#[test]
fn a_library_compared_with_itself_has_no_difference() {
    let mut rng = Lcg(0x5EED_0301);
    for _ in 0..200 {
        let lib = random_library(&mut rng, 20);
        let found = compare_libraries(&lib, &lib, 0);
        assert!(found.is_same());
        assert_eq!(found.same, lib.packages.len());
    }
}

#[test]
fn what_is_added_one_way_is_removed_the_other_way() {
    let mut rng = Lcg(0x5EED_0302);
    let mut mixed = 0;
    for _ in 0..300 {
        let (a, b) = (random_library(&mut rng, 10), random_library(&mut rng, 10));
        let forward = compare_libraries(&a, &b, 100);
        let back = compare_libraries(&b, &a, 100);
        assert_eq!(forward.added, back.removed);
        assert_eq!(forward.removed, back.added);
        assert_eq!(forward.same, back.same);
        let names = |c: &crate::compare::LibraryComparison| -> Vec<String> {
            c.changed.iter().map(|f| f.name.clone()).collect()
        };
        assert_eq!(names(&forward), names(&back));
        // Every footprint of the old library is removed, changed or the same.
        assert_eq!(
            forward.removed.len() + forward.changed.len() + forward.same,
            a.packages.len()
        );
        if !forward.added.is_empty() && !forward.removed.is_empty() {
            mixed += 1;
        }
    }
    assert!(mixed > 30, "{mixed}");
}

#[test]
fn the_lists_are_in_alphabetical_order_without_repeats() {
    let mut rng = Lcg(0x5EED_0303);
    for _ in 0..200 {
        let (a, b) = (random_library(&mut rng, 15), random_library(&mut rng, 15));
        let found = compare_libraries(&a, &b, 0);
        let changed: Vec<&String> = found.changed.iter().map(|c| &c.name).collect();
        for list in [
            found.added.iter().collect::<Vec<_>>(),
            found.removed.iter().collect::<Vec<_>>(),
            changed,
        ] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "{list:?}");
        }
    }
}

#[test]
fn a_shift_within_the_tolerance_is_nothing_and_one_beyond_it_changes_every_footprint() {
    let mut rng = Lcg(0x5EED_0304);
    let mut compared = 0;
    for _ in 0..200 {
        let lib = random_library(&mut rng, 20);
        if lib.packages.is_empty() {
            continue;
        }
        let tolerance = rng.range(0, 50_000);
        assert!(compare_libraries(&lib, &shifted(&lib, tolerance), tolerance).is_same());
        let beyond = compare_libraries(&lib, &shifted(&lib, tolerance + 1), tolerance);
        assert_eq!(beyond.changed.len(), lib.packages.len());
        assert!(beyond.added.is_empty() && beyond.removed.is_empty());
        for change in &beyond.changed {
            assert!(change.differences.iter().all(|d| d.contains("moved")));
        }
        compared += 1;
    }
    assert!(compared > 150, "{compared}");
}
