// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of two snapshots: what the new board has that the old one has not, what it has
//! lost and what it has changed, as a list of differences in a fixed order (the kinds in the
//! order of their codes, and inside a kind the names in order), so the same two boards always give
//! the same report.

use std::collections::BTreeSet;

use crate::codes::{
    Difference, COPPER_DIFF, DUPLICATE_REF, FOOTPRINT_DIFF, LAYER_DIFF, LIST_CUT, MOVE_DIFF,
    NET_DIFF, OUTLINE_DIFF, PART_DIFF, PIN_DIFF, VALUE_DIFF, VIA_DIFF, ZONE_DIFF,
};
use crate::settings::DiffSettings;
use crate::snapshot::{CopperFact, OutlineFact, PartFact, Snapshot, ZoneFact};

/// A count in the old board and in the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    /// What is counted: `parts`, `nets`, `pins`, `tracks`, `vias` or `zones`.
    pub what: &'static str,
    /// In the old board.
    pub old: usize,
    /// In the new board.
    pub new: usize,
}

/// The result of comparing two boards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReport {
    /// The name of the old board.
    pub old_name: String,
    /// The name of the new board.
    pub new_name: String,
    /// The counts of the objects of each board.
    pub counts: Vec<Count>,
    /// The differences found, at most `max_differences`, followed by a note when the list was
    /// cut.
    pub differences: Vec<Difference>,
    /// How many differences were found, listed or not (the notes are not counted).
    pub total: usize,
}

impl DiffReport {
    /// True when the boards differ in anything that was compared.
    pub fn differs(&self) -> bool {
        self.total > 0
    }
}

/// A number of nanometres as millimetres in text, exact: no exponent and no trailing zeros.
pub fn mm_text(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let abs = nm.unsigned_abs();
    let (whole, frac) = (abs / 1_000_000, abs % 1_000_000);
    if frac == 0 {
        return format!("{sign}{whole}");
    }
    let digits = format!("{frac:06}");
    format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
}

/// An area in square millimetres with three decimals, from twice the area in square nanometres.
fn area_text(double_area: i128) -> String {
    let thousandths = (double_area / 2 + 500_000_000) / 1_000_000_000;
    format!("{}.{:03}", thousandths / 1000, thousandths % 1000)
}

fn degrees_text(udeg: i64) -> String {
    let whole = udeg / 1_000_000;
    let frac = udeg % 1_000_000;
    if frac == 0 {
        return whole.to_string();
    }
    let digits = format!("{frac:06}");
    format!("{whole}.{}", digits.trim_end_matches('0'))
}

fn describe(p: &PartFact) -> String {
    format!(
        "{}, {}, {}",
        if p.value.is_empty() { "-" } else { &p.value },
        p.footprint,
        if p.bottom { "bottom" } else { "top" }
    )
}

fn only_in_one<'a>(
    old: &'a BTreeSet<String>,
    new: &'a BTreeSet<String>,
) -> impl Iterator<Item = (&'a String, bool)> {
    let removed = old.difference(new).map(|n| (n, false));
    let added = new.difference(old).map(|n| (n, true));
    let mut all: Vec<(&String, bool)> = removed.chain(added).collect();
    all.sort();
    all.into_iter()
}

fn parts(old: &Snapshot, new: &Snapshot, s: &DiffSettings, out: &mut Vec<Difference>) {
    let names: BTreeSet<&String> = old.parts.keys().chain(new.parts.keys()).collect();
    let mut value = Vec::new();
    let mut footprint = Vec::new();
    let mut moved = Vec::new();
    for name in names {
        match (old.parts.get(name), new.parts.get(name)) {
            (Some(o), None) => out.push(Difference::new(
                PART_DIFF,
                name.as_str(),
                format!("removed ({})", describe(o)),
            )),
            (None, Some(n)) => out.push(Difference::new(
                PART_DIFF,
                name.as_str(),
                format!("added ({})", describe(n)),
            )),
            (Some(o), Some(n)) => {
                if o.value != n.value {
                    value.push(Difference::new(
                        VALUE_DIFF,
                        name.as_str(),
                        format!("{} -> {}", o.value, n.value),
                    ));
                }
                if o.footprint != n.footprint {
                    footprint.push(Difference::new(
                        FOOTPRINT_DIFF,
                        name.as_str(),
                        format!("{} -> {}", o.footprint, n.footprint),
                    ));
                }
                if let Some(detail) = movement(o, n, s) {
                    moved.push(Difference::new(MOVE_DIFF, name.as_str(), detail));
                }
            }
            (None, None) => {}
        }
    }
    out.extend(value);
    out.extend(footprint);
    out.extend(moved);
}

/// How a part moved, turned or changed side, or none when it did not (beyond the tolerance).
fn movement(old: &PartFact, new: &PartFact, s: &DiffSettings) -> Option<String> {
    let mut what = Vec::new();
    let dx = i128::from(new.x_nm) - i128::from(old.x_nm);
    let dy = i128::from(new.y_nm) - i128::from(old.y_nm);
    let tolerance = i128::from(s.move_tolerance_nm);
    if dx * dx + dy * dy > tolerance * tolerance {
        let distance = ((dx * dx + dy * dy) as f64).sqrt().round() as i64;
        what.push(format!("moved {} mm", mm_text(distance)));
    }
    if old.rotation_udeg != new.rotation_udeg {
        what.push(format!(
            "turned from {} to {} degrees",
            degrees_text(old.rotation_udeg),
            degrees_text(new.rotation_udeg)
        ));
    }
    if old.bottom != new.bottom {
        what.push(format!(
            "now on the {}",
            if new.bottom { "bottom" } else { "top" }
        ));
    }
    (!what.is_empty()).then(|| what.join(", "))
}

fn pins(old: &Snapshot, new: &Snapshot, out: &mut Vec<Difference>) {
    let keys: BTreeSet<&String> = old.pins.keys().chain(new.pins.keys()).collect();
    for key in keys {
        // only the pins of the parts that both boards have: a part that was added or removed is
        // already one difference
        let part = key.split('.').next().unwrap_or("");
        if !(old.parts.contains_key(part) && new.parts.contains_key(part)) {
            continue;
        }
        let (a, b) = (old.pins.get(key), new.pins.get(key));
        if a != b {
            out.push(Difference::new(
                PIN_DIFF,
                key.as_str(),
                format!(
                    "{} -> {}",
                    a.map_or("(no net)", String::as_str),
                    b.map_or("(no net)", String::as_str)
                ),
            ));
        }
    }
}

fn copper(old: &Snapshot, new: &Snapshot, s: &DiffSettings, out: &mut Vec<Difference>) {
    let keys: BTreeSet<&(String, String)> = old.copper.keys().chain(new.copper.keys()).collect();
    for key in keys {
        let a = old.copper.get(key).copied().unwrap_or_default();
        let b = new.copper.get(key).copied().unwrap_or_default();
        if copper_differs(&a, &b, s) {
            out.push(Difference::new(
                COPPER_DIFF,
                format!("{} on {}", key.0, key.1),
                format!(
                    "tracks {} -> {}, length {} mm -> {} mm",
                    a.tracks,
                    b.tracks,
                    mm_text(a.length_nm),
                    mm_text(b.length_nm)
                ),
            ));
        }
    }
}

fn copper_differs(a: &CopperFact, b: &CopperFact, s: &DiffSettings) -> bool {
    a.tracks != b.tracks || (a.length_nm - b.length_nm).abs() > s.length_tolerance_nm
}

fn vias(old: &Snapshot, new: &Snapshot, out: &mut Vec<Difference>) {
    let keys: BTreeSet<&(String, i64, i64)> = old.vias.keys().chain(new.vias.keys()).collect();
    for key in keys {
        let a = old.vias.get(key).copied().unwrap_or(0);
        let b = new.vias.get(key).copied().unwrap_or(0);
        if a == b {
            continue;
        }
        let (word, count) = if b > a {
            ("added", b - a)
        } else {
            ("removed", a - b)
        };
        out.push(Difference::new(
            VIA_DIFF,
            format!("{} at {}, {} mm", key.0, mm_text(key.1), mm_text(key.2)),
            if count == 1 {
                word.to_owned()
            } else {
                format!("{word} x{count}")
            },
        ));
    }
}

fn zones(old: &Snapshot, new: &Snapshot, out: &mut Vec<Difference>) {
    let keys: BTreeSet<&(String, String)> = old.zones.keys().chain(new.zones.keys()).collect();
    for key in keys {
        let subject = format!("{} on {}", key.0, key.1);
        match (old.zones.get(key), new.zones.get(key)) {
            (Some(_), None) => out.push(Difference::new(ZONE_DIFF, subject, "removed")),
            (None, Some(_)) => out.push(Difference::new(ZONE_DIFF, subject, "added")),
            (Some(a), Some(b)) if a != b => {
                out.push(Difference::new(ZONE_DIFF, subject, zone_detail(a, b)))
            }
            _ => {}
        }
    }
}

fn zone_detail(a: &ZoneFact, b: &ZoneFact) -> String {
    format!(
        "zones {} -> {}, area {} mm2 -> {} mm2",
        a.count,
        b.count,
        area_text(a.double_area),
        area_text(b.double_area)
    )
}

fn outline(old: &OutlineFact, new: &OutlineFact, out: &mut Vec<Difference>) {
    if old == new {
        return;
    }
    let mut what = Vec::new();
    if old.corners != new.corners {
        what.push(format!("corners {} -> {}", old.corners, new.corners));
    }
    if old.cutouts != new.cutouts {
        what.push(format!("cutouts {} -> {}", old.cutouts, new.cutouts));
    }
    if (old.width_nm, old.height_nm) != (new.width_nm, new.height_nm) {
        what.push(format!(
            "box {} x {} mm -> {} x {} mm",
            mm_text(old.width_nm),
            mm_text(old.height_nm),
            mm_text(new.width_nm),
            mm_text(new.height_nm)
        ));
    }
    if old.double_area != new.double_area {
        what.push(format!(
            "area {} mm2 -> {} mm2",
            area_text(old.double_area),
            area_text(new.double_area)
        ));
    }
    out.push(Difference::new(OUTLINE_DIFF, "outline", what.join(", ")));
}

/// Compares two boards. The kinds of object that the settings do not compare are left out.
pub fn compare(old: &Snapshot, new: &Snapshot, s: &DiffSettings) -> DiffReport {
    let mut found = Vec::new();
    for (name, added) in only_in_one(&old.layers, &new.layers) {
        found.push(Difference::new(
            LAYER_DIFF,
            name.as_str(),
            if added { "added" } else { "removed" },
        ));
    }
    for (name, added) in only_in_one(&old.nets, &new.nets) {
        found.push(Difference::new(
            NET_DIFF,
            name.as_str(),
            if added { "added" } else { "removed" },
        ));
    }
    parts(old, new, s, &mut found);
    if s.compare_pins {
        pins(old, new, &mut found);
    }
    if s.compare_tracks {
        copper(old, new, s, &mut found);
    }
    if s.compare_vias {
        vias(old, new, &mut found);
    }
    if s.compare_zones {
        zones(old, new, &mut found);
    }
    if s.compare_outline {
        outline(&old.outline, &new.outline, &mut found);
    }
    for (board, name) in [(old, "the old board"), (new, "the new board")] {
        for reference in &board.duplicates {
            found.push(Difference::new(DUPLICATE_REF, reference.as_str(), name));
        }
    }
    let total = found.len();
    if total > s.max_differences {
        found.truncate(s.max_differences);
        found.push(Difference::new(
            LIST_CUT,
            "",
            format!(
                "{} more not listed (max_differences = {})",
                total - s.max_differences,
                s.max_differences
            ),
        ));
    }
    let count_tracks = |snap: &Snapshot| snap.copper.values().map(|c| c.tracks).sum::<usize>();
    let count_vias = |snap: &Snapshot| snap.vias.values().sum::<usize>();
    let count_zones = |snap: &Snapshot| snap.zones.values().map(|z| z.count).sum::<usize>();
    DiffReport {
        old_name: old.name.clone(),
        new_name: new.name.clone(),
        counts: vec![
            Count {
                what: "parts",
                old: old.parts.len(),
                new: new.parts.len(),
            },
            Count {
                what: "nets",
                old: old.nets.len(),
                new: new.nets.len(),
            },
            Count {
                what: "pins",
                old: old.pins.len(),
                new: new.pins.len(),
            },
            Count {
                what: "tracks",
                old: count_tracks(old),
                new: count_tracks(new),
            },
            Count {
                what: "vias",
                old: count_vias(old),
                new: count_vias(new),
            },
            Count {
                what: "zones",
                old: count_zones(old),
                new: count_zones(new),
            },
        ],
        differences: found,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::title;
    use eh3d_phoenix_rules::Lang;

    fn part(refdes: &str, value: &str, footprint: &str, at: (i64, i64)) -> PartFact {
        PartFact {
            refdes: refdes.into(),
            value: value.into(),
            footprint: footprint.into(),
            x_nm: at.0,
            y_nm: at.1,
            rotation_udeg: 0,
            bottom: false,
        }
    }

    fn base() -> Snapshot {
        let mut s = Snapshot {
            name: "base".into(),
            ..Snapshot::default()
        };
        s.layers.extend(["Top".to_owned(), "Bottom".to_owned()]);
        s.nets.extend(["GND".to_owned(), "SIG".to_owned()]);
        for p in [
            part("R1", "10k", "R_0603", (1_000_000, 2_000_000)),
            part("C1", "100n", "C_0603", (5_000_000, 2_000_000)),
        ] {
            s.parts.insert(p.refdes.clone(), p);
        }
        s.pins.insert("R1.1".into(), "SIG".into());
        s.pins.insert("R1.2".into(), "GND".into());
        s.pins.insert("C1.1".into(), "SIG".into());
        s.copper.insert(
            ("SIG".into(), "Top".into()),
            CopperFact {
                tracks: 2,
                length_nm: 12_000_000,
            },
        );
        s.vias.insert(("GND".into(), 3_000_000, 3_000_000), 1);
        s.zones.insert(
            ("GND".into(), "Bottom".into()),
            ZoneFact {
                count: 1,
                double_area: 200_000_000_000_000,
            },
        );
        s.outline = OutlineFact {
            corners: 4,
            cutouts: 0,
            width_nm: 20_000_000,
            height_nm: 10_000_000,
            double_area: 400_000_000_000_000,
        };
        s
    }

    fn codes(report: &DiffReport) -> Vec<&'static str> {
        report.differences.iter().map(|d| d.code).collect()
    }

    #[test]
    fn a_board_compared_with_itself_has_no_difference() {
        let r = compare(&base(), &base(), &DiffSettings::default());
        assert!(!r.differs() && r.differences.is_empty());
        assert_eq!(
            r.counts[0],
            Count {
                what: "parts",
                old: 2,
                new: 2
            }
        );
        assert_eq!(
            r.counts[3],
            Count {
                what: "tracks",
                old: 2,
                new: 2
            }
        );
    }

    #[test]
    fn layers_nets_and_parts_that_one_board_has_are_listed_added_or_removed() {
        let mut new = base();
        new.layers.insert("Inner1".into());
        new.nets.remove("SIG");
        new.nets.insert("VCC".into());
        new.parts.remove("C1");
        let p = part("U1", "", "SOIC8", (0, 0));
        new.parts.insert("U1".into(), p);
        let r = compare(&base(), &new, &DiffSettings::default());
        let lines: Vec<(&str, &str, &str)> = r
            .differences
            .iter()
            .map(|d| (d.code, d.subject.as_str(), d.detail.as_str()))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("PHOENIX-DIFF-005", "Inner1", "added"),
                ("PHOENIX-DIFF-006", "SIG", "removed"),
                ("PHOENIX-DIFF-006", "VCC", "added"),
                ("PHOENIX-DIFF-007", "C1", "removed (100n, C_0603, top)"),
                ("PHOENIX-DIFF-007", "U1", "added (-, SOIC8, top)"),
            ]
        );
        assert_eq!(r.total, 5);
    }

    #[test]
    fn a_value_a_footprint_a_move_a_turn_and_a_side_are_each_a_difference() {
        let mut new = base();
        {
            let r1 = new.parts.get_mut("R1").unwrap();
            r1.value = "22k".into();
            r1.footprint = "R_0805".into();
            r1.x_nm += 2_500_000;
            r1.rotation_udeg = 90_000_000;
            r1.bottom = true;
        }
        let r = compare(&base(), &new, &DiffSettings::default());
        assert_eq!(
            codes(&r),
            ["PHOENIX-DIFF-008", "PHOENIX-DIFF-009", "PHOENIX-DIFF-010"]
        );
        assert_eq!(r.differences[0].detail, "10k -> 22k");
        assert_eq!(r.differences[1].detail, "R_0603 -> R_0805");
        assert_eq!(
            r.differences[2].detail,
            "moved 2.5 mm, turned from 0 to 90 degrees, now on the bottom"
        );
    }

    #[test]
    fn a_part_that_moved_less_than_the_tolerance_has_not_moved() {
        let mut new = base();
        new.parts.get_mut("C1").unwrap().y_nm += 1_000;
        let mut s = DiffSettings::default();
        assert!(!compare(&base(), &new, &s).differs());
        new.parts.get_mut("C1").unwrap().y_nm += 1;
        assert!(compare(&base(), &new, &s).differs());
        s.move_tolerance_nm = 5_000;
        assert!(!compare(&base(), &new, &s).differs());
        // a turn is never inside a tolerance
        new.parts.get_mut("C1").unwrap().rotation_udeg = 1;
        assert!(compare(&base(), &new, &s).differs());
    }

    #[test]
    fn a_pin_on_another_net_is_listed_for_the_parts_that_both_boards_have_only() {
        let mut new = base();
        new.pins.insert("R1.2".into(), "SIG".into());
        new.pins.remove("C1.1");
        new.pins.insert("U9.1".into(), "GND".into());
        let r = compare(&base(), &new, &DiffSettings::default());
        let lines: Vec<(&str, &str)> = r
            .differences
            .iter()
            .map(|d| (d.subject.as_str(), d.detail.as_str()))
            .collect();
        // U9 is in neither board's parts, so its pin is not a difference of its own
        assert_eq!(
            lines,
            vec![("C1.1", "SIG -> (no net)"), ("R1.2", "GND -> SIG")]
        );
        assert!(codes(&r).iter().all(|c| *c == "PHOENIX-DIFF-011"));
    }

    #[test]
    fn copper_changes_beyond_the_length_tolerance_or_in_the_number_of_tracks_are_listed() {
        let longer = |extra: i64| {
            let mut new = base();
            new.copper
                .get_mut(&("SIG".to_owned(), "Top".to_owned()))
                .unwrap()
                .length_nm += extra;
            new
        };
        assert!(!compare(&base(), &longer(1_000), &DiffSettings::default()).differs());
        let r = compare(&base(), &longer(1_001), &DiffSettings::default());
        assert_eq!(r.differences[0].subject, "SIG on Top");
        assert_eq!(
            r.differences[0].detail,
            "tracks 2 -> 2, length 12 mm -> 12.001001 mm"
        );
        // one more track of no length is still a difference
        let mut more = base();
        more.copper
            .get_mut(&("SIG".to_owned(), "Top".to_owned()))
            .unwrap()
            .tracks = 3;
        assert!(compare(&base(), &more, &DiffSettings::default()).differs());
        // a net that gets copper on a layer it had none on
        let mut other = base();
        other.copper.insert(
            ("GND".into(), "Bottom".into()),
            CopperFact {
                tracks: 1,
                length_nm: 5_000_000,
            },
        );
        let r = compare(&base(), &other, &DiffSettings::default());
        assert_eq!(
            r.differences[0].detail,
            "tracks 0 -> 1, length 0 mm -> 5 mm"
        );
    }

    #[test]
    fn vias_zones_and_the_outline_are_compared() {
        let mut new = base();
        new.vias.insert(("GND".into(), 3_000_000, 3_000_000), 3);
        new.vias.insert(("SIG".into(), 9_000_000, 1_500_000), 1);
        new.zones
            .get_mut(&("GND".to_owned(), "Bottom".to_owned()))
            .unwrap()
            .double_area = 300_000_000_000_000;
        new.zones.insert(
            ("SIG".into(), "Top".into()),
            ZoneFact {
                count: 1,
                double_area: 2_000_000_000_000,
            },
        );
        new.outline.cutouts = 1;
        new.outline.double_area -= 2_000_000_000_000;
        let r = compare(&base(), &new, &DiffSettings::default());
        let lines: Vec<(&str, &str, &str)> = r
            .differences
            .iter()
            .map(|d| (d.code, d.subject.as_str(), d.detail.as_str()))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("PHOENIX-DIFF-013", "GND at 3, 3 mm", "added x2"),
                ("PHOENIX-DIFF-013", "SIG at 9, 1.5 mm", "added"),
                (
                    "PHOENIX-DIFF-014",
                    "GND on Bottom",
                    "zones 1 -> 1, area 100.000 mm2 -> 150.000 mm2"
                ),
                ("PHOENIX-DIFF-014", "SIG on Top", "added"),
                (
                    "PHOENIX-DIFF-015",
                    "outline",
                    "cutouts 0 -> 1, area 200.000 mm2 -> 199.000 mm2"
                ),
            ]
        );
        // a via that is gone
        let mut gone = base();
        gone.vias.clear();
        let r = compare(&base(), &gone, &DiffSettings::default());
        assert_eq!(r.differences[0].detail, "removed");
    }

    #[test]
    fn an_outline_that_is_the_same_gives_no_difference_and_each_change_is_named() {
        let mut found = Vec::new();
        outline(&base().outline, &base().outline, &mut found);
        assert!(found.is_empty());
        let mut new = base();
        new.outline.corners = 6;
        new.outline.width_nm += 1_500_000;
        outline(&base().outline, &new.outline, &mut found);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].detail,
            "corners 4 -> 6, box 20 x 10 mm -> 21.5 x 10 mm"
        );
    }

    #[test]
    fn the_kinds_the_settings_do_not_compare_are_left_out() {
        let mut new = base();
        new.pins.insert("R1.2".into(), "SIG".into());
        new.copper.clear();
        new.vias.clear();
        new.zones.clear();
        new.outline.width_nm += 1;
        let all = compare(&base(), &new, &DiffSettings::default());
        assert_eq!(all.total, 5);
        let none = DiffSettings {
            compare_pins: false,
            compare_tracks: false,
            compare_vias: false,
            compare_zones: false,
            compare_outline: false,
            ..DiffSettings::default()
        };
        assert!(!compare(&base(), &new, &none).differs());
    }

    #[test]
    fn a_reference_that_two_parts_have_is_noted_for_the_board_that_has_it() {
        let mut old = base();
        old.duplicates.insert("R1".into());
        let mut new = base();
        new.duplicates.insert("C1".into());
        let r = compare(&old, &new, &DiffSettings::default());
        let lines: Vec<(&str, &str, &str)> = r
            .differences
            .iter()
            .map(|d| (d.code, d.subject.as_str(), d.detail.as_str()))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("PHOENIX-DIFF-016", "R1", "the old board"),
                ("PHOENIX-DIFF-016", "C1", "the new board"),
            ]
        );
    }

    #[test]
    fn the_list_is_cut_at_the_limit_and_the_total_still_counts_everything() {
        let mut new = base();
        for i in 0..10 {
            new.nets.insert(format!("N{i}"));
        }
        let s = DiffSettings {
            max_differences: 3,
            ..DiffSettings::default()
        };
        let r = compare(&base(), &new, &s);
        assert_eq!(r.total, 10);
        assert_eq!(r.differences.len(), 4);
        let last = r.differences.last().unwrap();
        assert_eq!(last.code, LIST_CUT);
        assert_eq!(last.detail, "7 more not listed (max_differences = 3)");
        assert_eq!(
            title(LIST_CUT, Lang::Es),
            "La lista de diferencias se cortó"
        );
    }

    #[test]
    fn the_same_two_boards_always_give_the_same_report() {
        let mut new = base();
        new.parts.get_mut("R1").unwrap().value = "1k".into();
        new.nets.insert("X".into());
        new.vias.insert(("X".into(), 1, 2), 1);
        let a = compare(&base(), &new, &DiffSettings::default());
        let b = compare(&base(), &new, &DiffSettings::default());
        assert_eq!(a, b);
    }

    #[test]
    fn the_words_for_numbers_are_exact() {
        assert_eq!(mm_text(0), "0");
        assert_eq!(mm_text(-1_500_000), "-1.5");
        assert_eq!(mm_text(1), "0.000001");
        assert_eq!(area_text(0), "0.000");
        assert_eq!(area_text(2_000_000_000_000), "1.000");
        assert_eq!(area_text(2_000_000_000 + 1_000_000_000), "0.002");
        assert_eq!(degrees_text(90_000_000), "90");
        assert_eq!(degrees_text(22_500_000), "22.5");
        assert_eq!(degrees_text(1), "0.000001");
    }

    #[test]
    fn counts_of_objects_follow_the_boards() {
        let mut new = base();
        new.vias.insert(("GND".into(), 1, 1), 4);
        new.parts.remove("C1");
        let r = compare(&base(), &new, &DiffSettings::default());
        let get = |what: &str| r.counts.iter().find(|c| c.what == what).copied().unwrap();
        assert_eq!((get("vias").old, get("vias").new), (1, 5));
        assert_eq!((get("parts").old, get("parts").new), (2, 1));
        assert_eq!(r.counts.len(), 6);
    }
}
