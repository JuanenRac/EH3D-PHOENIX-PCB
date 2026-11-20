// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the comparison of two boards, read from the plain `key = value` text
//! `config/boarddiff.cfg`: how far a part may move or a net may grow before it counts as a
//! difference, which kinds of object are compared, the parts and nets that are left out, and the
//! limits. A missing line keeps the default; an unknown key or a value out of range is refused
//! with its line number. A list is words separated by commas.

use crate::codes::{DiffError, SETTINGS_INVALID};

/// The settings of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSettings {
    /// A part that moved by no more than this, in nanometres, has not moved.
    pub move_tolerance_nm: i64,
    /// The copper of a net on a layer whose length changed by no more than this, in nanometres,
    /// and whose number of tracks is the same, has not changed.
    pub length_tolerance_nm: i64,
    /// Whether the pins of the parts are compared by net.
    pub compare_pins: bool,
    /// Whether the tracks are compared (by net and layer).
    pub compare_tracks: bool,
    /// Whether the vias are compared.
    pub compare_vias: bool,
    /// Whether the zones are compared.
    pub compare_zones: bool,
    /// Whether the outline and its cutouts are compared.
    pub compare_outline: bool,
    /// References of parts that are left out, any case; a word ending in `*` matches every
    /// reference that starts with the rest.
    pub ignore_parts: Vec<String>,
    /// Names of nets that are left out, with the same rule.
    pub ignore_nets: Vec<String>,
    /// The most objects (parts, pins, tracks, vias and zones) a board may hold for the
    /// comparison to run.
    pub max_items: usize,
    /// The most differences a report lists; the rest are counted.
    pub max_differences: usize,
}

impl Default for DiffSettings {
    fn default() -> Self {
        DiffSettings {
            move_tolerance_nm: 1_000,
            length_tolerance_nm: 1_000,
            compare_pins: true,
            compare_tracks: true,
            compare_vias: true,
            compare_zones: true,
            compare_outline: true,
            ignore_parts: Vec::new(),
            ignore_nets: Vec::new(),
            max_items: 1_000_000,
            max_differences: 1_000,
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

fn matches_any(patterns: &[String], name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    patterns.iter().any(|pattern| {
        let pattern = pattern.to_ascii_uppercase();
        match pattern.strip_suffix('*') {
            Some(start) => name.starts_with(start),
            None => name == pattern,
        }
    })
}

impl DiffSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<DiffSettings, DiffError> {
        let mut s = DiffSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                DiffError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                DiffError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "move_tolerance_nm" => s.move_tolerance_nm = value.parse().map_err(|_| bad())?,
                "length_tolerance_nm" => {
                    s.length_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "compare_pins" => s.compare_pins = parse_bool(value).ok_or_else(bad)?,
                "compare_tracks" => s.compare_tracks = parse_bool(value).ok_or_else(bad)?,
                "compare_vias" => s.compare_vias = parse_bool(value).ok_or_else(bad)?,
                "compare_zones" => s.compare_zones = parse_bool(value).ok_or_else(bad)?,
                "compare_outline" => s.compare_outline = parse_bool(value).ok_or_else(bad)?,
                "ignore_parts" => s.ignore_parts = parse_list(value),
                "ignore_nets" => s.ignore_nets = parse_list(value),
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                "max_differences" => s.max_differences = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(DiffError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the comparison relies on.
    pub fn validate(&self) -> Result<(), DiffError> {
        let fail = |what: &str| Err(DiffError::new(SETTINGS_INVALID, what.to_string()));
        for (name, v) in [
            ("move_tolerance_nm", self.move_tolerance_nm),
            ("length_tolerance_nm", self.length_tolerance_nm),
        ] {
            if !(0..(1 << 30)).contains(&v) {
                return fail(&format!("{name} must be between 0 and 2^30 nanometres"));
            }
        }
        if !(1..=100_000_000).contains(&self.max_items) {
            return fail("max_items must be between 1 and 100000000");
        }
        if !(1..=1_000_000).contains(&self.max_differences) {
            return fail("max_differences must be between 1 and 1000000");
        }
        Ok(())
    }

    /// True when the reference of a part is one the settings leave out.
    pub fn ignores_part(&self, refdes: &str) -> bool {
        matches_any(&self.ignore_parts, refdes)
    }

    /// True when the name of a net is one the settings leave out.
    pub fn ignores_net(&self, net: &str) -> bool {
        matches_any(&self.ignore_nets, net)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(DiffSettings::default().validate().is_ok());
        assert_eq!(
            DiffSettings::parse("# nothing\n\n").unwrap(),
            DiffSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/boarddiff.cfg");
        assert_eq!(DiffSettings::parse(text).unwrap(), DiffSettings::default());
    }

    #[test]
    fn lines_change_the_values_and_lists_are_split_at_commas() {
        let s = DiffSettings::parse(
            "move_tolerance_nm = 5000\nlength_tolerance_nm = 0\ncompare_pins = false\ncompare_tracks = false\ncompare_vias = false\ncompare_zones = false\ncompare_outline = false\nignore_parts = TP*, FID1\nignore_nets = N$*\nmax_items = 50\nmax_differences = 7\n",
        )
        .unwrap();
        assert_eq!((s.move_tolerance_nm, s.length_tolerance_nm), (5000, 0));
        assert!(!s.compare_pins && !s.compare_tracks && !s.compare_vias);
        assert!(!s.compare_zones && !s.compare_outline);
        assert_eq!(s.ignore_parts, vec!["TP*".to_owned(), "FID1".to_owned()]);
        assert_eq!((s.max_items, s.max_differences), (50, 7));
        assert!(s.ignores_part("tp12") && s.ignores_part("fid1") && !s.ignores_part("FID2"));
        assert!(s.ignores_net("N$12") && !s.ignores_net("GND"));
        let empty = DiffSettings::parse("ignore_parts =\n").unwrap();
        assert!(empty.ignore_parts.is_empty());
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = DiffSettings::parse("\na = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 2"), "{}", unknown.detail);
        for text in [
            "move_tolerance_nm = -1\n",
            "move_tolerance_nm = 1073741824\n",
            "move_tolerance_nm = x\n",
            "length_tolerance_nm = -1\n",
            "compare_pins = yes\n",
            "compare_outline = 1\n",
            "max_items = 0\n",
            "max_items = 100000001\n",
            "max_differences = 0\n",
            "max_differences = 1000001\n",
            "no equals sign\n",
        ] {
            assert!(DiffSettings::parse(text).is_err(), "{text}");
        }
    }
}
