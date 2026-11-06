// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the own corpus, read from a plain `key = value` file (`config/corpus.cfg`).
//! A missing line keeps the default; an unknown key or a value out of range is refused with its
//! line number.

use super::codes::{SuiteError, SETTINGS_INVALID};

/// The bounds of a run of the own corpus and of the properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusSettings {
    /// The largest corpus file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The most expectation files of one corpus.
    pub max_entries: usize,
    /// The deepest nesting of folders the walk enters.
    pub max_depth: usize,
    /// How far two positions may differ in a round trip, in nanometres, when the expectation does
    /// not say.
    pub round_trip_tolerance_nm: i64,
    /// How many seeds each property is tried with.
    pub property_seeds: u32,
    /// The first seed; the others follow one by one.
    pub property_first_seed: u64,
    /// The most components of a generated board.
    pub property_max_components: u32,
    /// The most changes tried on a generated board by the properties of undo and rejection.
    pub property_max_steps: u32,
}

impl Default for CorpusSettings {
    fn default() -> Self {
        CorpusSettings {
            max_file_bytes: 8 * 1024 * 1024,
            max_entries: 2000,
            max_depth: 8,
            round_trip_tolerance_nm: 10,
            property_seeds: 48,
            property_first_seed: 1,
            property_max_components: 5,
            property_max_steps: 12,
        }
    }
}

impl CorpusSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<CorpusSettings, SuiteError> {
        let mut s = CorpusSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                SuiteError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                SuiteError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_entries" => s.max_entries = value.parse().map_err(|_| bad())?,
                "max_depth" => s.max_depth = value.parse().map_err(|_| bad())?,
                "round_trip_tolerance_nm" => {
                    s.round_trip_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "property_seeds" => s.property_seeds = value.parse().map_err(|_| bad())?,
                "property_first_seed" => {
                    s.property_first_seed = value.parse().map_err(|_| bad())?
                }
                "property_max_components" => {
                    s.property_max_components = value.parse().map_err(|_| bad())?
                }
                "property_max_steps" => s.property_max_steps = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(SuiteError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the run relies on.
    pub fn validate(&self) -> Result<(), SuiteError> {
        let bad = |what: &str| Err(SuiteError::new(SETTINGS_INVALID, what.to_owned()));
        if self.max_file_bytes == 0 || self.max_file_bytes > 256 * 1024 * 1024 {
            return bad("max_file_bytes must be from 1 to 268435456");
        }
        if self.max_entries == 0 || self.max_entries > 100_000 {
            return bad("max_entries must be from 1 to 100000");
        }
        if self.max_depth == 0 || self.max_depth > 32 {
            return bad("max_depth must be from 1 to 32");
        }
        if !(0..=1_000_000).contains(&self.round_trip_tolerance_nm) {
            return bad("round_trip_tolerance_nm must be from 0 to 1000000");
        }
        if self.property_seeds > 10_000 {
            return bad("property_seeds must be at most 10000");
        }
        if !(2..=12).contains(&self.property_max_components) {
            return bad("property_max_components must be from 2 to 12");
        }
        if !(1..=64).contains(&self.property_max_steps) {
            return bad("property_max_steps must be from 1 to 64");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_missing_line_keeps_them() {
        assert!(CorpusSettings::default().validate().is_ok());
        assert_eq!(
            CorpusSettings::parse("# nothing\n").unwrap(),
            CorpusSettings::default()
        );
        let s = CorpusSettings::parse("property_seeds = 5\nproperty_first_seed = 9\n").unwrap();
        assert_eq!((s.property_seeds, s.property_first_seed), (5, 9));
        assert_eq!(s.max_entries, CorpusSettings::default().max_entries);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for text in [
            "oops",
            "max_entries = ten",
            "colour = red",
            "max_entries = 0",
            "max_depth = 33",
            "round_trip_tolerance_nm = -1",
            "property_max_components = 1",
            "property_max_steps = 65",
            "max_file_bytes = 0",
        ] {
            let e = CorpusSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
        }
        let e = CorpusSettings::parse("\n\ncolour = red").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_file_of_the_repository_holds_the_defaults() {
        let text = include_str!("../../../../config/corpus.cfg");
        assert_eq!(
            CorpusSettings::parse(text).unwrap(),
            CorpusSettings::default()
        );
    }
}
