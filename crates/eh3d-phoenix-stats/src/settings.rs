// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the statistics. Every limit and every choice is read from a plain
//! `key = value` text, `config/stats.cfg`, and not buried in the code. A missing line keeps the
//! default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{StatsError, SETTINGS_INVALID};

/// The settings of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsSettings {
    /// The most objects (parts, tracks, vias, zones, connected pads and silkscreen items) a board
    /// may hold for the report to be made.
    pub max_items: usize,
    /// Count the area of the zones as copper (the whole outline less its cutouts, before the
    /// clearances); with false the zones are counted but have no area.
    pub count_zones: bool,
}

impl Default for StatsSettings {
    fn default() -> Self {
        StatsSettings {
            max_items: 1_000_000,
            count_zones: true,
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

impl StatsSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<StatsSettings, StatsError> {
        let mut s = StatsSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                StatsError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                StatsError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                "count_zones" => s.count_zones = parse_bool(value).ok_or_else(bad)?,
                _ => {
                    return Err(StatsError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the calculations rely on.
    pub fn validate(&self) -> Result<(), StatsError> {
        if !(1..=100_000_000).contains(&self.max_items) {
            return Err(StatsError::new(
                SETTINGS_INVALID,
                "max_items must be between 1 and 100000000",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(StatsSettings::default().validate().is_ok());
        assert_eq!(
            StatsSettings::parse("# nothing\n\n").unwrap(),
            StatsSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/stats.cfg");
        assert_eq!(
            StatsSettings::parse(text).unwrap(),
            StatsSettings::default()
        );
    }

    #[test]
    fn lines_change_the_values() {
        let s = StatsSettings::parse("max_items = 50\ncount_zones = false\n").unwrap();
        assert_eq!(s.max_items, 50);
        assert!(!s.count_zones);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = StatsSettings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        assert!(StatsSettings::parse("\nmax_items = 0\n").is_err());
        assert!(StatsSettings::parse("max_items = x\n").is_err());
        assert!(StatsSettings::parse("max_items = -3\n").is_err());
        assert!(StatsSettings::parse("count_zones = maybe\n").is_err());
        assert!(StatsSettings::parse("count_zones\n").is_err());
    }
}
