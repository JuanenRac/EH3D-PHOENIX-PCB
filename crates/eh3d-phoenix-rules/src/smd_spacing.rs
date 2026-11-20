// SPDX-License-Identifier: GPL-3.0-or-later
//! The least distance between surface-mount parts, by the type of their packages.
//!
//! Makers of assembly publish a table of the spacing they recommend between two parts: chips of
//! each size among themselves, chips and the bodies with leads (small outline, quad flat, no-lead,
//! transistor outline) and the ball grids. The table is data in `config/dfm/smd_spacing.cfg` (the
//! sets are `[table NAME]`, the lines `CLASS-CLASS = nanometres`, and `verified = false` until the
//! figures are compared with the maker); a profile names the table it uses with
//! `smd_spacing = NAME`. A pair that the table does not list is not checked.
//!
//! This module holds the table, the classification of a footprint name into one of ten classes and
//! the measure of the gap between the boxes of two parts; [`crate::smd_board`] takes the parts
//! from a board. A box is the box of the pads of the part, which is smaller than the courtyard and
//! so errs on the side of finding less.

use std::collections::BTreeMap;

/// The type of package of a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmdClass {
    /// Chip 0201 (imperial).
    C0201,
    /// Chip 0402.
    C0402,
    /// Chip 0603.
    C0603,
    /// Chip 0805.
    C0805,
    /// Chip 1206.
    C1206,
    /// No-lead body (QFN, DFN, SON).
    Qfn,
    /// Quad flat package with leads.
    Qfp,
    /// Small outline package (SOIC, SOP, SSOP, TSSOP).
    Sop,
    /// Small outline transistor and diode packages.
    Sot,
    /// Ball grid array.
    Bga,
}

impl SmdClass {
    /// The ten classes, in the order of the table.
    pub const ALL: [SmdClass; 10] = [
        SmdClass::C0201,
        SmdClass::C0402,
        SmdClass::C0603,
        SmdClass::C0805,
        SmdClass::C1206,
        SmdClass::Qfn,
        SmdClass::Qfp,
        SmdClass::Sop,
        SmdClass::Sot,
        SmdClass::Bga,
    ];

    /// The name in the table file.
    pub fn key(self) -> &'static str {
        match self {
            SmdClass::C0201 => "0201",
            SmdClass::C0402 => "0402",
            SmdClass::C0603 => "0603",
            SmdClass::C0805 => "0805",
            SmdClass::C1206 => "1206",
            SmdClass::Qfn => "qfn",
            SmdClass::Qfp => "qfp",
            SmdClass::Sop => "sop",
            SmdClass::Sot => "sot",
            SmdClass::Bga => "bga",
        }
    }

    /// The class of a name in the table file.
    pub fn from_key(key: &str) -> Option<SmdClass> {
        SmdClass::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// A table that cannot be read: the line and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmdTableError {
    /// The line of the text, counted from 1.
    pub line: usize,
    /// What is wrong, in English.
    pub detail: String,
}

impl std::fmt::Display for SmdTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.detail)
    }
}

impl std::error::Error for SmdTableError {}

/// One table: the least gaps by pair of classes, in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmdTable {
    /// The name that a profile gives.
    pub name: String,
    /// True when the figures were compared with the maker.
    pub verified: bool,
    gaps: BTreeMap<(SmdClass, SmdClass), i64>,
}

fn ordered(a: SmdClass, b: SmdClass) -> (SmdClass, SmdClass) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

impl SmdTable {
    /// An empty table.
    pub fn new(name: &str) -> SmdTable {
        SmdTable {
            name: name.to_owned(),
            verified: false,
            gaps: BTreeMap::new(),
        }
    }

    /// Sets the gap of a pair (the same for both orders).
    pub fn set(&mut self, a: SmdClass, b: SmdClass, nm: i64) {
        self.gaps.insert(ordered(a, b), nm);
    }

    /// The least gap of a pair, when the table lists it.
    pub fn gap(&self, a: SmdClass, b: SmdClass) -> Option<i64> {
        self.gaps.get(&ordered(a, b)).copied()
    }

    /// How many pairs the table lists.
    pub fn len(&self) -> usize {
        self.gaps.len()
    }

    /// True when the table lists no pair.
    pub fn is_empty(&self) -> bool {
        self.gaps.is_empty()
    }

    /// The largest gap of the table.
    pub fn largest(&self) -> i64 {
        self.gaps.values().copied().max().unwrap_or(0)
    }
}

/// Reads the tables of a text: `[table NAME]` starts one, `verified = true|false` and
/// `CLASS-CLASS = nanometres` are its lines. A class that is not one of the ten, a pair written
/// twice (in either order), a name used twice or a value out of range is refused with its line.
pub fn parse_tables(text: &str) -> Result<Vec<SmdTable>, SmdTableError> {
    let mut tables: Vec<SmdTable> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fail = |detail: String| SmdTableError { line, detail };
        if let Some(rest) = trimmed.strip_prefix("[table ") {
            let name = rest
                .strip_suffix(']')
                .map(str::trim)
                .ok_or_else(|| fail("a section is [table NAME]".to_owned()))?;
            let plain = !name.is_empty()
                && name.len() <= 32
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
            if !plain {
                return Err(fail("a name is 1 to 32 letters, digits, - or _".to_owned()));
            }
            if tables.iter().any(|t| t.name == name) {
                return Err(fail(format!("the table '{name}' appears twice")));
            }
            tables.push(SmdTable::new(name));
            continue;
        }
        let Some(table) = tables.last_mut() else {
            return Err(fail("a line is inside a [table NAME] section".to_owned()));
        };
        let (key, value) = trimmed
            .split_once('=')
            .ok_or_else(|| fail("not key = value".to_owned()))?;
        let (key, value) = (key.trim(), value.trim());
        if key == "verified" {
            table.verified = match value {
                "true" => true,
                "false" => false,
                _ => return Err(fail("verified is true or false".to_owned())),
            };
            continue;
        }
        let (a, b) = key
            .split_once('-')
            .and_then(|(a, b)| Some((SmdClass::from_key(a)?, SmdClass::from_key(b)?)))
            .ok_or_else(|| fail(format!("{key} is not a pair of classes like 0402-qfn")))?;
        let nm: i64 = value
            .parse()
            .map_err(|_| fail(format!("the value of {key} is not a number")))?;
        if !(0..=100_000_000).contains(&nm) {
            return Err(fail(format!("{key} is not between 0 and 100000000")));
        }
        if table.gap(a, b).is_some() {
            return Err(fail(format!("the pair {key} appears twice")));
        }
        table.set(a, b, nm);
    }
    Ok(tables)
}

/// The tables that come with the program.
pub const BUILTIN_TEXT: &str = include_str!("../../../config/dfm/smd_spacing.cfg");

/// A table that comes with the program, by name.
pub fn builtin_table(name: &str) -> Option<SmdTable> {
    parse_tables(BUILTIN_TEXT)
        .ok()?
        .into_iter()
        .find(|t| t.name == name)
}

/// The names of the tables that come with the program.
pub fn builtin_table_names() -> Vec<String> {
    parse_tables(BUILTIN_TEXT)
        .map(|tables| tables.into_iter().map(|t| t.name).collect())
        .unwrap_or_default()
}

/// The digits of a name in runs (`R_0603_1608Metric` gives `0603` and `1608`), with the byte where
/// each starts.
fn digit_runs(name: &str) -> Vec<(usize, &str)> {
    let bytes = name.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            runs.push((start, &name[start..i]));
        } else {
            i += 1;
        }
    }
    runs
}

/// The class of a footprint name, or `None` when the name is not one of the ten types. The
/// bodies are recognised by their family letters (BGA, QFN, QFP, SOIC, SOT...), the chips by the
/// code of their size: the metric one when the name is written like IPC-7351B (`RESC1608X55N`
/// is a 0603) and the imperial one otherwise (`R_0603_1608Metric` is a 0603).
pub fn classify_footprint(name: &str) -> Option<SmdClass> {
    let upper = name.to_ascii_uppercase();
    let has = |words: &[&str]| words.iter().any(|w| upper.contains(w));
    if has(&["BGA", "LGA", "CSP"]) {
        return Some(SmdClass::Bga);
    }
    if has(&["QFN", "DFN", "SON", "MLF", "LFCSP"]) {
        return Some(SmdClass::Qfn);
    }
    if has(&["QFP"]) {
        return Some(SmdClass::Qfp);
    }
    if has(&["SOIC", "SSOP", "TSSOP", "MSOP", "SOP"]) {
        return Some(SmdClass::Sop);
    }
    if has(&["SOT", "SC70", "SC-70", "SOD"]) {
        return Some(SmdClass::Sot);
    }
    let runs = digit_runs(&upper);
    // IPC-7351B: letters of a family, then the metric size and an X
    if let Some((start, digits)) = runs.first() {
        let letters_before =
            upper[..*start].chars().all(|c| c.is_ascii_uppercase()) && !upper[..*start].is_empty();
        let followed_by_x = upper[start + digits.len()..].starts_with('X');
        if letters_before && followed_by_x && digits.len() == 4 {
            return match *digits {
                "0603" => Some(SmdClass::C0201),
                "1005" => Some(SmdClass::C0402),
                "1608" => Some(SmdClass::C0603),
                "2012" => Some(SmdClass::C0805),
                "3216" => Some(SmdClass::C1206),
                _ => None,
            };
        }
    }
    for (_, digits) in &runs {
        match *digits {
            "0201" => return Some(SmdClass::C0201),
            "0402" => return Some(SmdClass::C0402),
            "0603" => return Some(SmdClass::C0603),
            "0805" => return Some(SmdClass::C0805),
            "1206" => return Some(SmdClass::C1206),
            _ => {}
        }
    }
    None
}

/// A part: its reference designator, class, side and the box of its pads in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmdPart {
    /// The reference designator.
    pub refdes: String,
    /// The class.
    pub class: SmdClass,
    /// True for a part on the bottom side.
    pub bottom: bool,
    /// The box: smallest x.
    pub min_x: i64,
    /// The box: smallest y.
    pub min_y: i64,
    /// The box: largest x.
    pub max_x: i64,
    /// The box: largest y.
    pub max_y: i64,
}

/// Two parts closer than the table asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmdViolation {
    /// The first reference designator (the smaller in text order).
    pub a: String,
    /// The second reference designator.
    pub b: String,
    /// The class of the first part.
    pub class_a: SmdClass,
    /// The class of the second part.
    pub class_b: SmdClass,
    /// The gap between the boxes, in nanometres, rounded down.
    pub gap_nm: i64,
    /// The least gap the table asks.
    pub required_nm: i64,
    /// The middle of the segment between the two boxes.
    pub at: (i64, i64),
}

/// The gap between two boxes, rounded down to a nanometre; 0 when they touch or overlap.
pub fn box_gap(a: &SmdPart, b: &SmdPart) -> i64 {
    let dx = i128::from(a.min_x.max(b.min_x)) - i128::from(a.max_x.min(b.max_x));
    let dy = i128::from(a.min_y.max(b.min_y)) - i128::from(a.max_y.min(b.max_y));
    let (dx, dy) = (dx.max(0), dy.max(0));
    let squared = (dx * dx + dy * dy) as u128;
    i64::try_from(squared.isqrt()).unwrap_or(i64::MAX)
}

fn middle(a: &SmdPart, b: &SmdPart) -> (i64, i64) {
    let mid = |lo_a: i64, hi_a: i64, lo_b: i64, hi_b: i64| -> i64 {
        let low = lo_a.max(lo_b);
        let high = hi_a.min(hi_b);
        if low <= high {
            ((i128::from(low) + i128::from(high)) / 2) as i64
        } else if hi_a < lo_b {
            ((i128::from(hi_a) + i128::from(lo_b)) / 2) as i64
        } else {
            ((i128::from(hi_b) + i128::from(lo_a)) / 2) as i64
        }
    };
    (
        mid(a.min_x, a.max_x, b.min_x, b.max_x),
        mid(a.min_y, a.max_y, b.min_y, b.max_y),
    )
}

/// Every pair of parts on the same side that is closer than the table asks, in the order of the
/// reference designators. Parts are swept along x so that a large board is not squared.
pub fn find_violations(parts: &[SmdPart], table: &SmdTable) -> Vec<SmdViolation> {
    let reach = table.largest();
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|i| (parts[*i].min_x, *i));
    let mut out = Vec::new();
    for (pos, &i) in order.iter().enumerate() {
        let a = &parts[i];
        for &j in &order[pos + 1..] {
            let b = &parts[j];
            if i128::from(b.min_x) - i128::from(a.max_x) >= i128::from(reach) {
                // the parts after this one start no further left, so none is within reach of a
                break;
            }
            if a.bottom != b.bottom {
                continue;
            }
            let Some(required) = table.gap(a.class, b.class) else {
                continue;
            };
            let gap = box_gap(a, b);
            if gap < required {
                let (first, second) = if a.refdes <= b.refdes { (a, b) } else { (b, a) };
                out.push(SmdViolation {
                    a: first.refdes.clone(),
                    b: second.refdes.clone(),
                    class_a: first.class,
                    class_b: second.class,
                    gap_nm: gap,
                    required_nm: required,
                    at: middle(a, b),
                });
            }
        }
    }
    out.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> SmdTable {
        builtin_table("jlcpcb").unwrap()
    }

    fn part(refdes: &str, class: SmdClass, x: i64, y: i64, w: i64, h: i64) -> SmdPart {
        SmdPart {
            refdes: refdes.to_owned(),
            class,
            bottom: false,
            min_x: x,
            min_y: y,
            max_x: x + w,
            max_y: y + h,
        }
    }

    #[test]
    fn the_table_of_the_notes_is_complete_symmetric_and_marked_not_verified() {
        let t = table();
        assert!(!t.verified);
        assert_eq!(t.len(), 55);
        for a in SmdClass::ALL {
            for b in SmdClass::ALL {
                assert!(t.gap(a, b).is_some(), "{a:?} {b:?}");
                assert_eq!(t.gap(a, b), t.gap(b, a));
            }
        }
        use SmdClass::*;
        for (a, b, nm) in [
            (C0201, C0201, 150_000),
            (C0201, C0402, 150_000),
            (C0402, C0402, 150_000),
            (C0402, C0603, 180_000),
            (C0805, C0805, 180_000),
            (C0805, C1206, 250_000),
            (C1206, C1206, 350_000),
            (C0603, Qfn, 1_000_000),
            (C1206, Qfp, 500_000),
            (C0402, Sop, 400_000),
            (C0201, Sot, 200_000),
            (C0805, Bga, 1_000_000),
            (Qfn, Qfn, 1_000_000),
            (Qfn, Bga, 1_500_000),
            (Qfp, Qfp, 1_250_000),
            (Qfp, Sot, 1_000_000),
            (Sop, Sop, 500_000),
            (Sop, Sot, 400_000),
            (Sot, Sot, 400_000),
            (Sot, Bga, 1_000_000),
            (Bga, Bga, 2_000_000),
        ] {
            assert_eq!(t.gap(a, b), Some(nm), "{a:?} {b:?}");
        }
        assert_eq!(t.largest(), 2_000_000);
        assert_eq!(builtin_table_names(), ["jlcpcb"]);
        assert!(builtin_table("nobody").is_none());
    }

    #[test]
    fn footprint_names_are_classified_by_family_and_by_size() {
        use SmdClass::*;
        for (name, want) in [
            ("RESC1608X55N", Some(C0603)),
            ("CAPC1005X55N", Some(C0402)),
            ("CAPC0603X33N", Some(C0201)),
            ("RESC2012X70N", Some(C0805)),
            ("INDC3216X60N", Some(C1206)),
            ("R_0603_1608Metric", Some(C0603)),
            ("C_0402_1005Metric", Some(C0402)),
            ("C_0201_0603Metric", Some(C0201)),
            ("R_0805", Some(C0805)),
            ("LED_1206", Some(C1206)),
            ("QFN50P350X350X100-19N", Some(Qfn)),
            ("DFN-8", Some(Qfn)),
            ("QFP80P1000X1000X160-48N", Some(Qfp)),
            ("LQFP-48", Some(Qfp)),
            ("SOIC127P600X175-8N", Some(Sop)),
            ("TSSOP-16", Some(Sop)),
            ("SOT23-3", Some(Sot)),
            ("SOD-123", Some(Sot)),
            ("BGA-256", Some(Bga)),
            ("LGA-16", Some(Bga)),
            ("AXIAL", None),
            ("R_1210", None),
            ("", None),
        ] {
            assert_eq!(classify_footprint(name), want, "{name}");
        }
    }

    #[test]
    fn the_gap_between_boxes_is_measured_in_integers_and_zero_when_they_touch() {
        use SmdClass::C0402;
        let a = part("A", C0402, 0, 0, 1_000_000, 500_000);
        let right = part("B", C0402, 1_300_000, 0, 1_000_000, 500_000);
        assert_eq!(box_gap(&a, &right), 300_000);
        let diagonal = part("C", C0402, 1_300_000, 900_000, 1_000_000, 500_000);
        // dx 300000 and dy 400000 make 500000 exactly
        assert_eq!(box_gap(&a, &diagonal), 500_000);
        let touching = part("D", C0402, 1_000_000, 0, 1_000_000, 500_000);
        assert_eq!(box_gap(&a, &touching), 0);
        let over = part("E", C0402, 500_000, 100_000, 100_000, 100_000);
        assert_eq!(box_gap(&a, &over), 0);
        // rounded down
        let off = part("F", C0402, 1_000_001, 500_001, 10, 10);
        assert_eq!(box_gap(&a, &off), 1);
    }

    #[test]
    fn parts_closer_than_the_table_asks_are_found_with_the_gap_and_the_limit() {
        use SmdClass::*;
        let parts = vec![
            part("U1", Qfn, 0, 0, 5_000_000, 5_000_000),
            part("C1", C0402, 5_500_000, 0, 1_000_000, 500_000),
            part("C2", C0402, 7_000_000, 0, 1_000_000, 500_000),
            part("C3", C0603, 7_000_000, 4_000_000, 1_600_000, 800_000),
        ];
        let found = find_violations(&parts, &table());
        // C1 is 0.5 mm from the QFN (1.0 asked); C2 is 2.0 mm from it, C3 is 2.0 mm away; C1-C2
        // are 0.5 mm apart (0.15 asked); C2-C3 are 3.5 mm apart
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].a.as_str(), found[0].b.as_str()), ("C1", "U1"));
        assert_eq!(
            (found[0].gap_nm, found[0].required_nm),
            (500_000, 1_000_000)
        );
        assert_eq!((found[0].class_a, found[0].class_b), (C0402, Qfn));
        assert_eq!(found[0].at, (5_250_000, 250_000));
    }

    #[test]
    fn parts_on_the_other_side_and_pairs_the_table_does_not_list_are_left_alone() {
        use SmdClass::*;
        let mut parts = vec![
            part("U1", Qfn, 0, 0, 5_000_000, 5_000_000),
            part("C1", C0402, 5_100_000, 0, 1_000_000, 500_000),
        ];
        parts[1].bottom = true;
        assert!(find_violations(&parts, &table()).is_empty());
        let mut sparse = SmdTable::new("sparse");
        sparse.set(Qfn, C0603, 1_000_000);
        parts[1].bottom = false;
        assert!(find_violations(&parts, &sparse).is_empty());
        assert!(find_violations(&[], &table()).is_empty());
    }

    #[test]
    fn the_sweep_finds_the_same_pairs_as_looking_at_all_of_them() {
        let mut state = 0x00c0_ffee_u64;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % n) as i64
        };
        let classes = SmdClass::ALL;
        let t = table();
        for round in 0..30 {
            let n = 5 + next(40) as usize;
            let parts: Vec<SmdPart> = (0..n)
                .map(|i| {
                    let mut p = part(
                        &format!("P{i:03}"),
                        classes[next(10) as usize],
                        next(60_000_000),
                        next(30_000_000),
                        500_000 + next(8_000_000),
                        500_000 + next(8_000_000),
                    );
                    p.bottom = next(4) == 0;
                    p
                })
                .collect();
            let fast = find_violations(&parts, &t);
            let mut slow = Vec::new();
            for i in 0..parts.len() {
                for j in i + 1..parts.len() {
                    let (a, b) = (&parts[i], &parts[j]);
                    if a.bottom != b.bottom {
                        continue;
                    }
                    let required = t.gap(a.class, b.class).unwrap();
                    if box_gap(a, b) < required {
                        slow.push((a.refdes.clone(), b.refdes.clone()));
                    }
                }
            }
            slow.sort();
            let fast: Vec<(String, String)> = fast.into_iter().map(|v| (v.a, v.b)).collect();
            assert_eq!(fast, slow, "round {round}");
        }
    }

    #[test]
    fn a_bad_table_is_refused_with_its_line() {
        for (text, needle) in [
            ("verified = true\n", "inside a [table NAME]"),
            ("[table a]\n0402-xyz = 5\n", "not a pair of classes"),
            ("[table a]\n0402qfn = 5\n", "not a pair of classes"),
            ("[table a]\n0402-qfn = x\n", "not a number"),
            ("[table a]\n0402-qfn = -1\n", "not between"),
            ("[table a]\n0402-qfn = 5\nqfn-0402 = 6\n", "appears twice"),
            ("[table a]\n[table a]\n", "appears twice"),
            ("[table a b]\n", "1 to 32"),
            ("[table a\n", "[table NAME]"),
            ("[table a]\nverified = maybe\n", "true or false"),
            ("[table a]\nno equals\n", "not key = value"),
        ] {
            let e = parse_tables(text).unwrap_err();
            assert!(e.detail.contains(needle), "{text:?}: {e}");
            assert!(e.line >= 1);
        }
        let ok =
            parse_tables("# c\n[table a]\nverified = true\n0402-qfn = 5\n[table b]\n").unwrap();
        assert_eq!(ok.len(), 2);
        assert!(ok[0].verified && ok[0].gap(SmdClass::Qfn, SmdClass::C0402) == Some(5));
        assert!(ok[1].is_empty());
    }

    #[test]
    fn names_never_panic_with_a_fixed_seed() {
        let alphabet = b"RCQFNBGAOSIT0123456789_-XM ";
        let mut state = 0x1357_9bdfu64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        for _ in 0..4000 {
            let len = next() % 20;
            let name: String = (0..len)
                .map(|_| char::from(alphabet[next() % alphabet.len()]))
                .collect();
            let _ = classify_footprint(&name);
        }
    }
}
