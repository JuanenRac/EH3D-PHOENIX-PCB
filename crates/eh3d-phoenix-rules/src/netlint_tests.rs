// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests with a fixed seed of the check of the names of the nets: the same cases on every run.

use crate::netlint::{lint_net_names, NetIssue, NetIssueKind, NetLintConfig};

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

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const POOL: [&str; 14] = [
    "GND",
    "gnd",
    "VCC",
    "+3V3",
    "N$7",
    "CLK_P",
    "CLK_N",
    "A B",
    "x;y",
    "SDA",
    "Net-(R1-Pad1)",
    "USB_DP",
    "",
    "usb_dn",
];

fn random_names(rng: &mut Lcg) -> Vec<String> {
    let count = rng.below(25) as usize;
    (0..count)
        .map(|_| POOL[rng.below(POOL.len() as u64) as usize].to_owned())
        .collect()
}

fn shuffled(rng: &mut Lcg, names: &[String]) -> Vec<String> {
    let mut out = names.to_vec();
    for i in (1..out.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        out.swap(i, j);
    }
    out
}

#[test]
fn the_order_of_the_names_does_not_change_the_issues() {
    let mut rng = Lcg(0x5EED_0001);
    let config = NetLintConfig::default();
    let mut with_issues = 0;
    for _ in 0..300 {
        let names = random_names(&mut rng);
        let first = lint_net_names(&names, &config);
        let second = lint_net_names(&shuffled(&mut rng, &names), &config);
        assert_eq!(first, second, "{names:?}");
        if !first.is_empty() {
            with_issues += 1;
        }
    }
    assert!(with_issues > 100, "{with_issues}");
}

#[test]
fn every_issue_names_a_net_of_the_list_and_the_issues_are_in_order() {
    let mut rng = Lcg(0x5EED_0002);
    let config = NetLintConfig::default();
    for _ in 0..300 {
        let names = random_names(&mut rng);
        let issues = lint_net_names(&names, &config);
        for issue in &issues {
            assert!(names.contains(&issue.net), "{issue:?}");
            assert!(!issue.detail.is_empty());
        }
        let keys: Vec<(&str, NetIssueKind)> =
            issues.iter().map(|i| (i.net.as_str(), i.kind)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(keys, sorted, "{names:?}");
    }
}

#[test]
fn names_of_a_clean_alphabet_give_no_issue() {
    let mut rng = Lcg(0x5EED_0003);
    let config = NetLintConfig::default();
    for _ in 0..200 {
        let count = rng.below(40) as usize + 1;
        let names: Vec<String> = (0..count)
            .map(|i| format!("S{}_{}", i, rng.below(1000)))
            .collect();
        let unique: std::collections::BTreeSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
        assert!(lint_net_names(&names, &config).is_empty(), "{names:?}");
    }
}

#[test]
fn listing_a_name_twice_adds_one_duplicate_for_it() {
    let mut rng = Lcg(0x5EED_0004);
    let config = NetLintConfig::default();
    for _ in 0..200 {
        let count = rng.below(30) as usize + 1;
        let mut names: Vec<String> = (0..count).map(|i| format!("S{i}")).collect();
        let pick = rng.below(count as u64) as usize;
        names.push(names[pick].clone());
        let issues: Vec<NetIssue> = lint_net_names(&names, &config);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].kind, NetIssueKind::Duplicate);
        assert_eq!(issues[0].net, names[pick]);
    }
}

#[test]
fn a_name_that_is_cut_by_the_limit_is_listed_and_one_at_the_limit_is_not() {
    let mut rng = Lcg(0x5EED_0005);
    for _ in 0..200 {
        let limit = rng.below(30) as usize + 1;
        let config = NetLintConfig {
            max_len: limit,
            ..NetLintConfig::default()
        };
        let at = "S".repeat(limit);
        let over = "S".repeat(limit + 1);
        assert!(lint_net_names(&[at], &config).is_empty());
        let found = lint_net_names(&[over], &config);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, NetIssueKind::TooLong);
    }
}
