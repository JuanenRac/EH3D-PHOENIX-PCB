// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the statistics. Every limit and every choice is read from a plain
//! `key = value` text, `config/stats.cfg`, and not buried in the code. A missing line keeps the
//! default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{Severity, StatsError, SETTINGS_INVALID};

/// The settings of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsSettings {
    /// The most objects (parts, tracks, vias, zones, connected pads and silkscreen items) a board
    /// may hold for the report to be made.
    pub max_items: usize,
    /// Count the area of the zones as copper (the whole outline less its cutouts, before the
    /// clearances); with false the zones are counted but have no area.
    pub count_zones: bool,
    /// Compare the copper of the layers that mirror each other (the first with the last, and so
    /// on inwards).
    pub balance_check: bool,
    /// The most that the coverage of two layers that mirror each other may differ, in hundredths
    /// of a percentage point.
    pub balance_max_difference_x100: i64,
    /// How serious a pair above that limit is: information, a warning or an error.
    pub balance_severity: Severity,
}

impl Default for StatsSettings {
    fn default() -> Self {
        StatsSettings {
            max_items: 1_000_000,
            count_zones: true,
            balance_check: true,
            balance_max_difference_x100: 1500,
            balance_severity: Severity::Warning,
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
                "balance_check" => s.balance_check = parse_bool(value).ok_or_else(bad)?,
                "balance_max_difference_x100" => {
                    s.balance_max_difference_x100 = value.parse().map_err(|_| bad())?
                }
                "balance_severity" => {
                    s.balance_severity = match value {
                        "info" => Severity::Info,
                        "warning" => Severity::Warning,
                        "error" => Severity::Error,
                        _ => return Err(bad()),
                    }
                }
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
        if !(0..=10_000).contains(&self.balance_max_difference_x100) {
            return Err(StatsError::new(
                SETTINGS_INVALID,
                "balance_max_difference_x100 must be between 0 and 10000",
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
    fn the_lines_of_the_balance_change_its_limit_and_its_severity() {
        let s = StatsSettings::parse(
            "balance_check = false\nbalance_max_difference_x100 = 2000\nbalance_severity = error\n",
        )
        .unwrap();
        assert!(!s.balance_check);
        assert_eq!(s.balance_max_difference_x100, 2000);
        assert_eq!(s.balance_severity, Severity::Error);
        let info = StatsSettings::parse("balance_severity = info\n").unwrap();
        assert_eq!(info.balance_severity, Severity::Info);
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
        assert!(StatsSettings::parse("balance_check = 1\n").is_err());
        assert!(StatsSettings::parse("balance_max_difference_x100 = 10001\n").is_err());
        assert!(StatsSettings::parse("balance_max_difference_x100 = -1\n").is_err());
        assert!(StatsSettings::parse("balance_severity = loud\n").is_err());
    }
}
