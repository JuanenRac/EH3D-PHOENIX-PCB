// SPDX-License-Identifier: GPL-3.0-or-later
//! The lines of a bill that can be bought as one: lines whose part numbers meet.
//!
//! The bill groups parts by value, footprint, maker and part number, so the same resistor given
//! by two part numbers is two lines. When the file of part data names alternatives, a part number
//! of one line may be the part number or an alternative of another; those lines are equivalent
//! for a purchase, and the quantity to buy is that of the lines together. [`equivalent_groups`]
//! finds those sets: two lines are joined when a part number of one (its own or an alternative)
//! is a part number of the other, in any case, and the sets are the lines joined directly or
//! through others.
//!
//! The function reads the lines and changes nothing. Lines with no part number and no
//! alternative are never in a group, and a set of one line is not reported. A set whose lines do
//! not all have the same value and footprint is marked so, because an alternative that has
//! another footprint is more likely a mistake in the file than a part that can be fitted.

use crate::bom::BomLine;

/// A set of lines that can be bought as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquivalentGroup {
    /// The positions of the lines in the bill, increasing; at least two.
    pub lines: Vec<usize>,
    /// The part numbers of the lines and their alternatives, without repeats in any case, in
    /// alphabetical order of their upper case form; the first spelling found is kept.
    pub part_numbers: Vec<String>,
    /// The parts of one board in all the lines.
    pub qty: u64,
    /// The parts that the batch needs, with the waste, in all the lines.
    pub needed: u64,
    /// True when the lines do not all have the same value and the same footprint.
    pub mixed_kinds: bool,
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The sets of lines whose part numbers meet, in the order of their first line.
pub fn equivalent_groups(lines: &[BomLine]) -> Vec<EquivalentGroup> {
    let mut parent: Vec<usize> = (0..lines.len()).collect();
    // The first line that holds each part number, in upper case.
    let mut owner: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let numbers = std::iter::once(&line.mpn).chain(line.alternatives.iter());
        for number in numbers.map(|n| n.trim()).filter(|n| !n.is_empty()) {
            let key = number.to_uppercase();
            match owner.get(&key) {
                Some(&other) => {
                    let (a, b) = (find(&mut parent, index), find(&mut parent, other));
                    if a != b {
                        // The smaller root wins, so that a set is named by its first line.
                        parent[a.max(b)] = a.min(b);
                    }
                }
                None => {
                    owner.insert(key, index);
                }
            }
        }
    }
    let mut sets: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for index in 0..lines.len() {
        let root = find(&mut parent, index);
        sets.entry(root).or_default().push(index);
    }
    sets.into_values()
        .filter(|members| members.len() >= 2)
        .map(|members| {
            let mut numbers: std::collections::BTreeMap<String, String> =
                std::collections::BTreeMap::new();
            for &i in &members {
                let line = &lines[i];
                for number in std::iter::once(&line.mpn).chain(line.alternatives.iter()) {
                    let number = number.trim();
                    if !number.is_empty() {
                        numbers
                            .entry(number.to_uppercase())
                            .or_insert_with(|| number.to_owned());
                    }
                }
            }
            let first = &lines[members[0]];
            EquivalentGroup {
                qty: members.iter().map(|&i| lines[i].qty).sum(),
                needed: members.iter().map(|&i| lines[i].needed).sum(),
                mixed_kinds: members.iter().any(|&i| {
                    lines[i].value != first.value || lines[i].footprint != first.footprint
                }),
                part_numbers: numbers.into_values().collect(),
                lines: members,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(value: &str, footprint: &str, mpn: &str, alternatives: &[&str], qty: u64) -> BomLine {
        BomLine {
            value: value.to_owned(),
            footprint: footprint.to_owned(),
            manufacturer: String::new(),
            mpn: mpn.to_owned(),
            alternatives: alternatives.iter().map(|a| (*a).to_owned()).collect(),
            references: Vec::new(),
            qty,
            needed: qty * 10,
            purchase: None,
            cost_per_board_micro: 0,
        }
    }

    #[test]
    fn two_lines_that_name_each_others_part_number_are_one_set() {
        let lines = [
            line("10k", "R0603", "RC0603-A", &["RC0603-B"], 3),
            line("100n", "C0603", "CC-1", &[], 2),
            line("10k", "R0603", "rc0603-b", &[], 4),
        ];
        let sets = equivalent_groups(&lines);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].lines, vec![0, 2]);
        assert_eq!((sets[0].qty, sets[0].needed), (7, 70));
        assert_eq!(sets[0].part_numbers, vec!["RC0603-A", "RC0603-B"]);
        assert!(!sets[0].mixed_kinds);
    }

    #[test]
    fn sets_are_joined_through_a_third_line() {
        let lines = [
            line("1k", "R0402", "A", &["B"], 1),
            line("1k", "R0402", "C", &["D"], 1),
            line("1k", "R0402", "B", &["C"], 1),
        ];
        let sets = equivalent_groups(&lines);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].lines, vec![0, 1, 2]);
        assert_eq!(sets[0].part_numbers, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn lines_with_no_part_number_and_lines_alone_are_not_reported() {
        let lines = [
            line("1k", "R0402", "", &[], 1),
            line("1k", "R0402", "", &[], 1),
            line("2k", "R0402", "X", &[], 1),
            line("3k", "R0402", "Y", &["Z"], 1),
        ];
        assert!(equivalent_groups(&lines).is_empty());
        assert!(equivalent_groups(&[]).is_empty());
    }

    #[test]
    fn an_alternative_with_another_footprint_marks_the_set() {
        let lines = [
            line("10k", "R0603", "A", &["B"], 1),
            line("10k", "R0805", "B", &[], 1),
        ];
        let sets = equivalent_groups(&lines);
        assert_eq!(sets.len(), 1);
        assert!(sets[0].mixed_kinds);
    }

    #[test]
    fn blanks_around_a_part_number_do_not_matter() {
        let lines = [
            line("1k", "R0402", " A ", &[], 1),
            line("1k", "R0402", "a", &[], 1),
        ];
        let sets = equivalent_groups(&lines);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].part_numbers, vec!["A"]);
    }
}
