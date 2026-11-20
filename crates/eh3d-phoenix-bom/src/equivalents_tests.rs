// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the sets of equivalent lines with a fixed seed: the sets that the function finds are
//! those of a slow search written apart, they do not depend on the order of the lines, and no
//! part number is in two sets.

use std::collections::BTreeSet;

use crate::bom::BomLine;
use crate::equivalents::equivalent_groups;

/// A generator with a fixed seed, so every run makes the same bills.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

fn line(mpn: String, alternatives: Vec<String>, qty: u64) -> BomLine {
    BomLine {
        value: "v".to_owned(),
        footprint: "f".to_owned(),
        manufacturer: String::new(),
        mpn,
        alternatives,
        references: Vec::new(),
        qty,
        needed: qty + 1,
        purchase: None,
        cost_per_board_micro: 0,
    }
}

/// Lines with part numbers taken from a small pool, so that many of them meet.
fn random_bill(rng: &mut Lcg, count: usize, pool: u64) -> Vec<BomLine> {
    (0..count)
        .map(|_| {
            let name = |rng: &mut Lcg| {
                let n = rng.next() % pool;
                if rng.next().is_multiple_of(2) {
                    format!("pn{n}")
                } else {
                    format!("PN{n}")
                }
            };
            let mpn = if rng.next().is_multiple_of(6) {
                String::new()
            } else {
                name(rng)
            };
            let alternatives = (0..rng.next() % 3).map(|_| name(rng)).collect();
            line(mpn, alternatives, 1 + rng.next() % 20)
        })
        .collect()
}

fn numbers(l: &BomLine) -> BTreeSet<String> {
    std::iter::once(&l.mpn)
        .chain(&l.alternatives)
        .filter(|n| !n.is_empty())
        .map(|n| n.to_uppercase())
        .collect()
}

/// The slow search: lines are neighbours when they share a part number; the sets are the
/// components of that graph with at least two lines.
fn slow_sets(lines: &[BomLine]) -> BTreeSet<Vec<usize>> {
    let sets: Vec<BTreeSet<String>> = lines.iter().map(numbers).collect();
    let mut seen = vec![false; lines.len()];
    let mut out = BTreeSet::new();
    for start in 0..lines.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut members = vec![start];
        let mut at = 0;
        while at < members.len() {
            let here = members[at];
            at += 1;
            for other in 0..lines.len() {
                if !seen[other] && !sets[here].is_disjoint(&sets[other]) {
                    seen[other] = true;
                    members.push(other);
                }
            }
        }
        if members.len() >= 2 {
            members.sort_unstable();
            out.insert(members);
        }
    }
    out
}

#[test]
fn the_sets_are_those_of_the_slow_search() {
    let mut rng = Lcg(0x5a_0c01);
    for _ in 0..150 {
        let count = 1 + (rng.next() % 40) as usize;
        let pool = 2 + rng.next() % 40;
        let bill = random_bill(&mut rng, count, pool);
        let found: BTreeSet<Vec<usize>> = equivalent_groups(&bill)
            .into_iter()
            .map(|g| g.lines)
            .collect();
        assert_eq!(found, slow_sets(&bill));
    }
}

#[test]
fn no_part_number_is_in_two_sets_and_the_totals_are_the_sums_of_the_lines() {
    let mut rng = Lcg(0x5a_0c02);
    for _ in 0..150 {
        let bill = random_bill(&mut rng, 30, 25);
        let groups = equivalent_groups(&bill);
        let mut all: BTreeSet<String> = BTreeSet::new();
        for g in &groups {
            assert!(g.lines.len() >= 2);
            assert!(g.lines.windows(2).all(|w| w[0] < w[1]));
            let own: BTreeSet<String> = g.part_numbers.iter().map(|n| n.to_uppercase()).collect();
            assert_eq!(own.len(), g.part_numbers.len());
            assert!(all.is_disjoint(&own));
            all.extend(own);
            assert_eq!(g.qty, g.lines.iter().map(|&i| bill[i].qty).sum::<u64>());
            assert_eq!(
                g.needed,
                g.lines.iter().map(|&i| bill[i].needed).sum::<u64>()
            );
            assert!(!g.mixed_kinds);
        }
        let firsts: Vec<usize> = groups.iter().map(|g| g.lines[0]).collect();
        assert!(firsts.windows(2).all(|w| w[0] < w[1]));
    }
}

#[test]
fn the_order_of_the_lines_does_not_change_the_part_numbers_of_the_sets() {
    let mut rng = Lcg(0x5a_0c03);
    for _ in 0..100 {
        let bill = random_bill(&mut rng, 25, 20);
        let mut shuffled = bill.clone();
        for i in (1..shuffled.len()).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            shuffled.swap(i, j);
        }
        let keys = |b: &[BomLine]| -> BTreeSet<Vec<String>> {
            equivalent_groups(b)
                .into_iter()
                .map(|g| g.part_numbers.iter().map(|n| n.to_uppercase()).collect())
                .collect()
        };
        assert_eq!(keys(&bill), keys(&shuffled));
    }
}
