// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the reader of user programs, read from the plain `key = value` text
//! `config/ulp.cfg`: every limit of size, tokens, nesting and statements, and how many problems a
//! report lists. A missing line keeps the default; an unknown key or a value out of range is
//! refused with its line number.

use crate::codes::{Diag, SETTINGS_INVALID};

/// The settings of one reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UlpSettings {
    /// The largest program, in bytes.
    pub max_source_bytes: usize,
    /// The most tokens a program may have.
    pub max_tokens: usize,
    /// The deepest nesting of blocks, parentheses and expressions.
    pub max_depth: usize,
    /// The most statements (declarations, blocks and functions included) a program may have.
    pub max_statements: usize,
    /// The longest text in quotes, in characters.
    pub max_string_chars: usize,
    /// The longest name, in characters.
    pub max_name_chars: usize,
    /// The most problems that are kept; the rest are counted and the reading goes on to the end.
    pub max_problems: usize,
}

impl Default for UlpSettings {
    fn default() -> Self {
        UlpSettings {
            max_source_bytes: 1_048_576,
            max_tokens: 200_000,
            max_depth: 100,
            max_statements: 50_000,
            max_string_chars: 65_536,
            max_name_chars: 128,
            max_problems: 50,
        }
    }
}

/// The largest nesting that the settings may ask for: the reader calls itself once for each level.
pub const DEPTH_CEILING: usize = 500;

impl UlpSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<UlpSettings, Diag> {
        let mut s = UlpSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| Diag::at(SETTINGS_INVALID, line, 1, "not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                Diag::at(
                    SETTINGS_INVALID,
                    line,
                    1,
                    format!("the value of {key} is not valid"),
                )
            };
            let number = || value.parse::<usize>().map_err(|_| bad());
            match key {
                "max_source_bytes" => s.max_source_bytes = number()?,
                "max_tokens" => s.max_tokens = number()?,
                "max_depth" => s.max_depth = number()?,
                "max_statements" => s.max_statements = number()?,
                "max_string_chars" => s.max_string_chars = number()?,
                "max_name_chars" => s.max_name_chars = number()?,
                "max_problems" => s.max_problems = number()?,
                _ => {
                    return Err(Diag::at(
                        SETTINGS_INVALID,
                        line,
                        1,
                        format!("unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the reader relies on.
    pub fn validate(&self) -> Result<(), Diag> {
        let fail = |what: &str| Err(Diag::new(SETTINGS_INVALID, what.to_string()));
        if !(1..=1 << 30).contains(&self.max_source_bytes) {
            return fail("max_source_bytes must be between 1 and 1073741824");
        }
        if !(1..=100_000_000).contains(&self.max_tokens) {
            return fail("max_tokens must be between 1 and 100000000");
        }
        if !(1..=DEPTH_CEILING).contains(&self.max_depth) {
            return fail("max_depth must be between 1 and 500");
        }
        if !(1..=10_000_000).contains(&self.max_statements) {
            return fail("max_statements must be between 1 and 10000000");
        }
        if !(1..=1 << 24).contains(&self.max_string_chars) {
            return fail("max_string_chars must be between 1 and 16777216");
        }
        if !(1..=4096).contains(&self.max_name_chars) {
            return fail("max_name_chars must be between 1 and 4096");
        }
        if !(1..=100_000).contains(&self.max_problems) {
            return fail("max_problems must be between 1 and 100000");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(UlpSettings::default().validate().is_ok());
        assert_eq!(
            UlpSettings::parse("# nothing\n\n").unwrap(),
            UlpSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/ulp.cfg");
        assert_eq!(UlpSettings::parse(text).unwrap(), UlpSettings::default());
    }

    #[test]
    fn lines_change_the_values() {
        let s = UlpSettings::parse(
            "max_source_bytes = 100\nmax_tokens = 50\nmax_depth = 8\nmax_statements = 9\nmax_string_chars = 20\nmax_name_chars = 12\nmax_problems = 3\n",
        )
        .unwrap();
        assert_eq!(
            (
                s.max_source_bytes,
                s.max_tokens,
                s.max_depth,
                s.max_statements,
                s.max_string_chars,
                s.max_name_chars,
                s.max_problems
            ),
            (100, 50, 8, 9, 20, 12, 3)
        );
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        for (text, line) in [
            ("max_tokens\n", 1),
            ("\nmax_tokens = many\n", 2),
            ("# x\n\nunknown_key = 1\n", 3),
            ("max_depth = -1\n", 1),
        ] {
            let e = UlpSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text:?}");
            assert_eq!(e.line, line, "{text:?}");
        }
    }

    #[test]
    fn a_value_out_of_range_is_refused() {
        for text in [
            "max_source_bytes = 0",
            "max_tokens = 0",
            "max_depth = 0",
            "max_depth = 501",
            "max_statements = 0",
            "max_string_chars = 0",
            "max_name_chars = 0",
            "max_name_chars = 4097",
            "max_problems = 0",
            "max_problems = 100001",
        ] {
            let e = UlpSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
        }
    }
}
