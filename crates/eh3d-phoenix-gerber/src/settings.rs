// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the import. Every limit and default is read from a plain `key = value` text,
//! `config/gerber_import.cfg`, and not buried in the code. A missing line keeps the default; an
//! unknown key or a value out of range is refused with its line number.

use crate::codes::{GbrError, SETTINGS_INVALID};

/// Which zeros an Excellon file leaves out when it does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroOmission {
    /// The leading zeros are left out (`TZ`: the trailing zeros are written).
    Leading,
    /// The trailing zeros are left out (`LZ`: the leading zeros are written).
    Trailing,
}

/// The limits and defaults of one import. Lengths are in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSettings {
    /// The largest file that is read, in bytes.
    pub max_file_bytes: usize,
    /// The most files of one import.
    pub max_files: usize,
    /// The most commands (blocks) of one Gerber file or lines of one Excellon file.
    pub max_commands: usize,
    /// The most apertures of a Gerber file, or tools of an Excellon file.
    pub max_apertures: usize,
    /// The most drawn objects (flashes, lines, arcs, regions, holes) of one file.
    pub max_objects: usize,
    /// The most points of the regions of one file, arcs included.
    pub max_region_points: usize,
    /// The most primitives of one aperture macro, once its variables are evaluated.
    pub max_macro_primitives: usize,
    /// The longest expression of an aperture macro, in characters.
    pub max_expression_chars: usize,
    /// The largest distance, in nanometres, between an arc and the straight pieces that replace it
    /// inside a region.
    pub arc_chord_nm: i64,
    /// The most straight pieces that replace one arc inside a region.
    pub arc_max_segments: usize,
    /// How far the two radii of an arc may differ before the arc is reported.
    pub arc_radius_tolerance_nm: i64,
    /// The largest coordinate or size that is read (absolute value).
    pub coordinate_limit_nm: i64,
    /// How far two positions may differ and still be the same in the comparison with a board.
    pub compare_tolerance_nm: i64,
    /// The zeros an Excellon file leaves out when it does not say.
    pub excellon_omission: ZeroOmission,
    /// The digits before the point of an Excellon file in inches that does not say.
    pub excellon_inch_integer_digits: u32,
    /// The digits after the point of an Excellon file in inches that does not say.
    pub excellon_inch_decimal_digits: u32,
    /// The digits before the point of an Excellon file in millimetres that does not say.
    pub excellon_metric_integer_digits: u32,
    /// The digits after the point of an Excellon file in millimetres that does not say.
    pub excellon_metric_decimal_digits: u32,
}

impl Default for ImportSettings {
    fn default() -> Self {
        ImportSettings {
            max_file_bytes: 64 * 1024 * 1024,
            max_files: 64,
            max_commands: 5_000_000,
            max_apertures: 4096,
            max_objects: 2_000_000,
            max_region_points: 2_000_000,
            max_macro_primitives: 256,
            max_expression_chars: 256,
            arc_chord_nm: 5_000,
            arc_max_segments: 4096,
            arc_radius_tolerance_nm: 5_000,
            coordinate_limit_nm: 1 << 30,
            compare_tolerance_nm: 1_000,
            excellon_omission: ZeroOmission::Leading,
            excellon_inch_integer_digits: 2,
            excellon_inch_decimal_digits: 4,
            excellon_metric_integer_digits: 3,
            excellon_metric_decimal_digits: 3,
        }
    }
}

impl ImportSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<ImportSettings, GbrError> {
        let mut s = ImportSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                GbrError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                GbrError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_files" => s.max_files = value.parse().map_err(|_| bad())?,
                "max_commands" => s.max_commands = value.parse().map_err(|_| bad())?,
                "max_apertures" => s.max_apertures = value.parse().map_err(|_| bad())?,
                "max_objects" => s.max_objects = value.parse().map_err(|_| bad())?,
                "max_region_points" => s.max_region_points = value.parse().map_err(|_| bad())?,
                "max_macro_primitives" => {
                    s.max_macro_primitives = value.parse().map_err(|_| bad())?
                }
                "max_expression_chars" => {
                    s.max_expression_chars = value.parse().map_err(|_| bad())?
                }
                "arc_chord_nm" => s.arc_chord_nm = value.parse().map_err(|_| bad())?,
                "arc_max_segments" => s.arc_max_segments = value.parse().map_err(|_| bad())?,
                "arc_radius_tolerance_nm" => {
                    s.arc_radius_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "coordinate_limit_nm" => {
                    s.coordinate_limit_nm = value.parse().map_err(|_| bad())?
                }
                "compare_tolerance_nm" => {
                    s.compare_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "excellon_omission" => {
                    s.excellon_omission = match value {
                        "leading" => ZeroOmission::Leading,
                        "trailing" => ZeroOmission::Trailing,
                        _ => return Err(bad()),
                    }
                }
                "excellon_inch_integer_digits" => {
                    s.excellon_inch_integer_digits = value.parse().map_err(|_| bad())?
                }
                "excellon_inch_decimal_digits" => {
                    s.excellon_inch_decimal_digits = value.parse().map_err(|_| bad())?
                }
                "excellon_metric_integer_digits" => {
                    s.excellon_metric_integer_digits = value.parse().map_err(|_| bad())?
                }
                "excellon_metric_decimal_digits" => {
                    s.excellon_metric_decimal_digits = value.parse().map_err(|_| bad())?
                }
                _ => {
                    return Err(GbrError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the readers rely on.
    pub fn validate(&self) -> Result<(), GbrError> {
        let fail = |what: &str| Err(GbrError::new(SETTINGS_INVALID, what.to_string()));
        if !(1..=1 << 30).contains(&self.max_file_bytes) {
            return fail("max_file_bytes must be between 1 and 2^30");
        }
        if !(1..=1024).contains(&self.max_files) {
            return fail("max_files must be between 1 and 1024");
        }
        if !(1..=100_000_000).contains(&self.max_commands) {
            return fail("max_commands must be between 1 and 100000000");
        }
        if !(1..=1_000_000).contains(&self.max_apertures) {
            return fail("max_apertures must be between 1 and 1000000");
        }
        if !(1..=100_000_000).contains(&self.max_objects) {
            return fail("max_objects must be between 1 and 100000000");
        }
        if !(1..=100_000_000).contains(&self.max_region_points) {
            return fail("max_region_points must be between 1 and 100000000");
        }
        if !(1..=4096).contains(&self.max_macro_primitives) {
            return fail("max_macro_primitives must be between 1 and 4096");
        }
        if !(8..=4096).contains(&self.max_expression_chars) {
            return fail("max_expression_chars must be between 8 and 4096");
        }
        if !(1..=1_000_000).contains(&self.arc_chord_nm) {
            return fail("arc_chord_nm must be between 1 and 1000000");
        }
        if !(1..=65_536).contains(&self.arc_max_segments) {
            return fail("arc_max_segments must be between 1 and 65536");
        }
        if !(0..=1_000_000).contains(&self.arc_radius_tolerance_nm) {
            return fail("arc_radius_tolerance_nm must be between 0 and 1000000");
        }
        if !(1..=1 << 40).contains(&self.coordinate_limit_nm) {
            return fail("coordinate_limit_nm must be between 1 and 2^40");
        }
        if !(0..=1_000_000).contains(&self.compare_tolerance_nm) {
            return fail("compare_tolerance_nm must be between 0 and 1000000");
        }
        let digits = |integer: u32, decimal: u32| {
            (1..=8).contains(&integer) && (1..=8).contains(&decimal) && integer + decimal <= 15
        };
        if !digits(
            self.excellon_inch_integer_digits,
            self.excellon_inch_decimal_digits,
        ) {
            return fail("the Excellon inch digits must be 1 to 8 each and 15 at most together");
        }
        if !digits(
            self.excellon_metric_integer_digits,
            self.excellon_metric_decimal_digits,
        ) {
            return fail("the Excellon metric digits must be 1 to 8 each and 15 at most together");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            ImportSettings::parse("# nothing\n").unwrap(),
            ImportSettings::default()
        );
        let s = ImportSettings::parse(
            "max_files = 3\nexcellon_omission = trailing\narc_chord_nm = 100\n",
        )
        .unwrap();
        assert_eq!(s.max_files, 3);
        assert_eq!(s.excellon_omission, ZeroOmission::Trailing);
        assert_eq!(s.arc_chord_nm, 100);
        assert_eq!(s.max_objects, ImportSettings::default().max_objects);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("max_files = many", "max_files"),
            ("max_files = 0", "max_files"),
            ("colour = red", "unknown key"),
            ("oops", "key = value"),
            ("excellon_omission = both", "excellon_omission"),
            ("arc_chord_nm = 0", "arc_chord_nm"),
            ("max_expression_chars = 2", "max_expression_chars"),
            ("excellon_inch_integer_digits = 9", "inch digits"),
            ("excellon_metric_decimal_digits = 0", "metric digits"),
        ] {
            let e = ImportSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = ImportSettings::parse("\n\nmax_files = x").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/gerber_import.cfg");
        assert_eq!(
            ImportSettings::parse(text).unwrap(),
            ImportSettings::default()
        );
    }
}
