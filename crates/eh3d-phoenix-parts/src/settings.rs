// SPDX-License-Identifier: GPL-3.0-or-later
//! The limits of the parts base, read from a plain `key = value` text, `config/parts.cfg`. A
//! missing line keeps the default; an unknown key or a value out of range is refused with its
//! line number.

use crate::codes::{PartsError, SETTINGS_INVALID};

/// The limits of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartsSettings {
    /// The most files read from a folder.
    pub max_files: usize,
    /// The largest file, in bytes.
    pub max_file_bytes: u64,
    /// The most parts in the whole base.
    pub max_parts: usize,
    /// The most offers (suppliers) of one part.
    pub max_offers: usize,
    /// The most price breaks of one offer.
    pub max_tiers: usize,
    /// The most results a search prints.
    pub max_results: usize,
}

impl Default for PartsSettings {
    fn default() -> Self {
        PartsSettings {
            max_files: 200,
            max_file_bytes: 16 * 1024 * 1024,
            max_parts: 200_000,
            max_offers: 50,
            max_tiers: 50,
            max_results: 1000,
        }
    }
}

impl PartsSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<PartsSettings, PartsError> {
        let mut s = PartsSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                PartsError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                PartsError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            let count = || -> Result<usize, PartsError> {
                value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n >= 1)
                    .ok_or_else(bad)
            };
            match key {
                "max_files" => s.max_files = count()?,
                "max_file_bytes" => {
                    s.max_file_bytes = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or_else(bad)?;
                }
                "max_parts" => s.max_parts = count()?,
                "max_offers_per_part" => s.max_offers = count()?,
                "max_tiers_per_offer" => s.max_tiers = count()?,
                "max_results" => s.max_results = count()?,
                _ => {
                    return Err(PartsError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_line_keeps_the_default_and_a_line_changes_one_value() {
        let s =
            PartsSettings::parse("# limits\n\nmax_results = 5\nmax_tiers_per_offer = 7\n").unwrap();
        assert_eq!(s.max_results, 5);
        assert_eq!(s.max_tiers, 7);
        assert_eq!(s.max_parts, PartsSettings::default().max_parts);
        assert_eq!(PartsSettings::parse("").unwrap(), PartsSettings::default());
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("max_parts\n", "line 1: not key = value"),
            ("\nnope = 1\n", "line 2: unknown key nope"),
            ("max_files = 0\n", "line 1: the value of max_files"),
            ("max_file_bytes = x\n", "the value of max_file_bytes"),
            ("max_results = -3\n", "the value of max_results"),
        ] {
            let e = PartsSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn the_shipped_file_is_the_default() {
        let text = include_str!("../../../config/parts.cfg");
        assert_eq!(
            PartsSettings::parse(text).unwrap(),
            PartsSettings::default()
        );
    }
}
