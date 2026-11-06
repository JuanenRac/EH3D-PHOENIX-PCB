// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the annotation back into the schematic, read from `config/annotate.cfg` (which
//! is also built into the program as the default): what is taken from the board, the rules of the
//! renumbering by position and the pins and gates that are equivalent. Nothing that is tuned is
//! written in the code of the planner.

use std::collections::{BTreeMap, BTreeSet};

use crate::annot_codes::SETTINGS_INVALID;

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1 << 20;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/annotate.cfg");

/// The pin numbers of one gate (or of one pin) of a group of equivalent ones.
pub type Slot = Vec<String>;

/// Equivalent slots: when the board connects them the other way round the plan proposes a swap.
pub type SwapGroup = Vec<Slot>;

/// Why a settings file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// The file is larger than [`MAX_SETTINGS_BYTES`].
    #[error("the settings file is {size} bytes, the limit is {limit}")]
    TooLarge {
        /// Its size.
        size: usize,
        /// The limit.
        limit: usize,
    },
    /// A line is not valid.
    #[error("line {line}: {message}")]
    Line {
        /// The line, from 1.
        line: usize,
        /// What is wrong.
        message: String,
    },
}

impl SettingsError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        SETTINGS_INVALID
    }
}

/// How the components are put in order to be numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Column by column, from left to right; inside a column from the top down.
    LeftToRight,
    /// Row by row, from the top down; inside a row from left to right.
    TopToBottom,
}

/// The settings of the annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotateSettings {
    /// Write the values of the board into the parts.
    pub values: bool,
    /// Write the footprints of the board into the parts.
    pub footprints: bool,
    /// Renumber the components by their position.
    pub renumber: bool,
    /// The order of the numbers.
    pub order: Order,
    /// Components whose coordinate across the order lies within this distance of the first of a
    /// row (or column) are in it, in nanometres.
    pub row_tolerance_nm: i64,
    /// The Y axis of the board grows upwards.
    pub y_axis_up: bool,
    /// The first number.
    pub start: u32,
    /// The step between two numbers (at least 1).
    pub step: u32,
    /// The first number of a prefix, by prefix in upper case.
    pub start_of: BTreeMap<String, u32>,
    /// The step of a prefix, by prefix in upper case.
    pub step_of: BTreeMap<String, u32>,
    /// The prefixes that are renumbered, in upper case; empty means all.
    pub prefixes: Vec<String>,
    /// The references that keep their name, in upper case.
    pub locked: BTreeSet<String>,
    /// Number the two sides apart.
    pub by_side: bool,
    /// Added to the first number of the bottom side when the sides are numbered apart.
    pub bottom_offset: u32,
    /// The equivalent pins and gates, by name of symbol.
    pub swaps: BTreeMap<String, Vec<SwapGroup>>,
}

impl Default for AnnotateSettings {
    /// The settings file of the repository (`config/annotate.cfg`).
    fn default() -> Self {
        let mut settings = AnnotateSettings::empty();
        if settings.apply(BUILT_IN).is_err() {
            return AnnotateSettings::empty();
        }
        settings
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_swap(value: &str) -> Option<Vec<SwapGroup>> {
    let mut groups: Vec<SwapGroup> = Vec::new();
    for text in value.split(';') {
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let mut group: SwapGroup = Vec::new();
        for slot_text in text.split('|') {
            let slot: Slot = slot_text
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect();
            if slot.is_empty() {
                return None;
            }
            group.push(slot);
        }
        let width = group[0].len();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let distinct = group.iter().flatten().all(|p| seen.insert(p.as_str()));
        if group.len() < 2 || group.iter().any(|s| s.len() != width) || !distinct {
            return None;
        }
        groups.push(group);
    }
    Some(groups)
}

impl AnnotateSettings {
    fn empty() -> AnnotateSettings {
        AnnotateSettings {
            values: false,
            footprints: false,
            renumber: false,
            order: Order::TopToBottom,
            row_tolerance_nm: 0,
            y_axis_up: true,
            start: 1,
            step: 1,
            start_of: BTreeMap::new(),
            step_of: BTreeMap::new(),
            prefixes: Vec::new(),
            locked: BTreeSet::new(),
            by_side: false,
            bottom_offset: 0,
            swaps: BTreeMap::new(),
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<AnnotateSettings, SettingsError> {
        let mut settings = AnnotateSettings::default();
        settings.apply(text)?;
        Ok(settings)
    }

    /// The first number of a prefix.
    pub fn start_for(&self, prefix: &str) -> u32 {
        self.start_of.get(prefix).copied().unwrap_or(self.start)
    }

    /// The step of a prefix.
    pub fn step_for(&self, prefix: &str) -> u32 {
        self.step_of.get(prefix).copied().unwrap_or(self.step)
    }

    /// True when the components of a prefix (in upper case) are renumbered.
    pub fn renumbers(&self, prefix: &str) -> bool {
        !prefix.is_empty()
            && (self.prefixes.is_empty() || self.prefixes.iter().any(|p| p == prefix))
    }

    fn apply(&mut self, text: &str) -> Result<(), SettingsError> {
        if text.len() > MAX_SETTINGS_BYTES {
            return Err(SettingsError::TooLarge {
                size: text.len(),
                limit: MAX_SETTINGS_BYTES,
            });
        }
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| SettingsError::Line { line, message };
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || fail(format!("the value of {key} is not valid"));
            if let Some(prefix) = key.strip_prefix("start.") {
                let n: u32 = value.parse().map_err(|_| bad())?;
                self.start_of.insert(prefix.trim().to_ascii_uppercase(), n);
            } else if let Some(prefix) = key.strip_prefix("step.") {
                let n: u32 = value.parse().map_err(|_| bad())?;
                if n == 0 {
                    return Err(bad());
                }
                self.step_of.insert(prefix.trim().to_ascii_uppercase(), n);
            } else if let Some(symbol) = key.strip_prefix("swap.") {
                let groups = parse_swap(value).ok_or_else(bad)?;
                let symbol = symbol.trim().to_owned();
                if groups.is_empty() {
                    self.swaps.remove(&symbol);
                } else {
                    self.swaps.insert(symbol, groups);
                }
            } else {
                match key {
                    "values" => self.values = parse_bool(value).ok_or_else(bad)?,
                    "footprints" => self.footprints = parse_bool(value).ok_or_else(bad)?,
                    "renumber" => self.renumber = parse_bool(value).ok_or_else(bad)?,
                    "y_axis_up" => self.y_axis_up = parse_bool(value).ok_or_else(bad)?,
                    "by_side" => self.by_side = parse_bool(value).ok_or_else(bad)?,
                    "order" => {
                        self.order = match value {
                            "left_to_right" => Order::LeftToRight,
                            "top_to_bottom" => Order::TopToBottom,
                            _ => return Err(bad()),
                        }
                    }
                    "row_tolerance_nm" => {
                        let n: i64 = value.parse().map_err(|_| bad())?;
                        if n < 0 {
                            return Err(bad());
                        }
                        self.row_tolerance_nm = n;
                    }
                    "start" => self.start = value.parse().map_err(|_| bad())?,
                    "step" => {
                        let n: u32 = value.parse().map_err(|_| bad())?;
                        if n == 0 {
                            return Err(bad());
                        }
                        self.step = n;
                    }
                    "bottom_offset" => self.bottom_offset = value.parse().map_err(|_| bad())?,
                    "prefixes" => {
                        let list: Vec<String> = value
                            .split(',')
                            .map(|p| p.trim().to_ascii_uppercase())
                            .filter(|p| !p.is_empty())
                            .collect();
                        if list
                            .iter()
                            .any(|p| !p.chars().all(|c| c.is_ascii_alphabetic()))
                        {
                            return Err(bad());
                        }
                        self.prefixes = list;
                    }
                    "locked" => {
                        self.locked = value
                            .split(',')
                            .map(|p| p.trim().to_ascii_uppercase())
                            .filter(|p| !p.is_empty())
                            .collect();
                    }
                    _ => return Err(fail(format!("unknown key {key}"))),
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = AnnotateSettings::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, AnnotateSettings::default());
        let s = AnnotateSettings::default();
        assert!(s.values && s.footprints && !s.renumber && s.y_axis_up && !s.by_side);
        assert_eq!(s.order, Order::TopToBottom);
        assert_eq!((s.start, s.step, s.bottom_offset), (1, 1, 100));
        assert_eq!(s.row_tolerance_nm, 2_000_000);
        assert!(s.renumbers("R") && s.renumbers("U") && !s.renumbers("J") && !s.renumbers(""));
        assert!(s.locked.is_empty() && s.swaps.is_empty());
    }

    #[test]
    fn a_line_overrides_the_default_and_the_rest_stays() {
        let s = AnnotateSettings::parse(
            "renumber = true\norder = left_to_right\nstart.c = 10\nstep.C = 5\nlocked = u1, J2\nprefixes =\n",
        )
        .unwrap();
        assert!(s.renumber && s.values);
        assert_eq!(s.order, Order::LeftToRight);
        assert_eq!((s.start_for("C"), s.step_for("C")), (10, 5));
        assert_eq!((s.start_for("R"), s.step_for("R")), (1, 1));
        assert!(s.locked.contains("U1") && s.locked.contains("J2"));
        // no prefix named means every prefix
        assert!(s.renumbers("J"));
        assert_eq!(
            AnnotateSettings::parse("# none\n").unwrap(),
            AnnotateSettings::default()
        );
    }

    #[test]
    fn swaps_are_groups_of_slots_of_the_same_width() {
        let s =
            AnnotateSettings::parse("swap.CONN2 = 1 | 2\nswap.74HC00 = 1,2,3 | 4,5,6 | 9,10,8\n")
                .unwrap();
        assert_eq!(
            s.swaps["CONN2"],
            vec![vec![vec!["1".to_owned()], vec!["2".to_owned()]]]
        );
        assert_eq!(s.swaps["74HC00"][0].len(), 3);
        assert_eq!(s.swaps["74HC00"][0][2], ["9", "10", "8"]);
        // an empty value removes the entry
        let mut again = s.clone();
        again.apply("swap.CONN2 =\n").unwrap();
        assert!(!again.swaps.contains_key("CONN2"));
        assert!(again.swaps.contains_key("74HC00"));
    }

    #[test]
    fn bad_settings_name_their_line_and_never_panic() {
        for (text, line) in [
            ("nonsense", 1),
            ("\nunknown = 1", 2),
            ("values = maybe", 1),
            ("order = sideways", 1),
            ("step = 0", 1),
            ("step.R = 0", 1),
            ("start = -1", 1),
            ("row_tolerance_nm = -5", 1),
            ("prefixes = R, 1", 1),
            ("swap.X = 1", 1),
            ("swap.X = 1,2 | 3", 1),
            ("swap.X = 1 | 1", 1),
            ("swap.X = 1 | ", 1),
        ] {
            match AnnotateSettings::parse(text) {
                Err(SettingsError::Line { line: found, .. }) => assert_eq!(found, line, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
        let big = " ".repeat(MAX_SETTINGS_BYTES + 1);
        let error = AnnotateSettings::parse(&big).unwrap_err();
        assert!(matches!(error, SettingsError::TooLarge { .. }));
        assert_eq!(error.code(), SETTINGS_INVALID);
    }
}
