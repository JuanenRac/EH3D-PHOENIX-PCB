// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the 3D assembly. Every thickness, tolerance and limit is read from a plain
//! `key = value` text, `config/assembly3d.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{AssemblyError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The thickness, tolerances and limits of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblySettings {
    /// The thickness of the board when no stack-up gives it, in nanometres.
    pub default_board_thickness_nm: i64,
    /// The tallest a component may stand above (or hang below) the board, in nanometres; 0 for no
    /// limit.
    pub max_component_height_nm: i64,
    /// How deep two boxes must overlap, in every direction, to count as a collision, in
    /// nanometres.
    pub min_overlap_nm: i64,
    /// Whether a component whose box is not inside the outline of the board is reported.
    pub check_outline: bool,
    /// The most components a run handles.
    pub max_components: usize,
    /// The most findings of one kind a run lists.
    pub max_findings: usize,
    /// The largest input file, in bytes.
    pub max_file_bytes: u64,
    /// The most entities of a STEP file.
    pub max_step_entities: usize,
}

impl Default for AssemblySettings {
    fn default() -> Self {
        AssemblySettings {
            default_board_thickness_nm: 1_600_000,
            max_component_height_nm: 25_000_000,
            min_overlap_nm: 1_000,
            check_outline: true,
            max_components: 50_000,
            max_findings: 1_000,
            max_file_bytes: 16 * 1024 * 1024,
            max_step_entities: 2_000_000,
        }
    }
}

impl AssemblySettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<AssemblySettings, AssemblyError> {
        let mut s = AssemblySettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                AssemblyError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                AssemblyError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "default_board_thickness_nm" => {
                    s.default_board_thickness_nm = value.parse().map_err(|_| bad())?
                }
                "max_component_height_nm" => {
                    s.max_component_height_nm = value.parse().map_err(|_| bad())?
                }
                "min_overlap_nm" => s.min_overlap_nm = value.parse().map_err(|_| bad())?,
                "check_outline" => {
                    s.check_outline = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad()),
                    }
                }
                "max_components" => s.max_components = value.parse().map_err(|_| bad())?,
                "max_findings" => s.max_findings = value.parse().map_err(|_| bad())?,
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_step_entities" => s.max_step_entities = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(AssemblyError::new(
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
    pub fn validate(&self) -> Result<(), AssemblyError> {
        let fail = |what: &str| Err(AssemblyError::new(SETTINGS_INVALID, what.to_string()));
        if self.default_board_thickness_nm <= 0 || self.default_board_thickness_nm > LENGTH_LIMIT_NM
        {
            return fail("default_board_thickness_nm must be above 0 and within the range");
        }
        if self.max_component_height_nm < 0 || self.max_component_height_nm > LENGTH_LIMIT_NM {
            return fail("max_component_height_nm must be from 0 and within the range");
        }
        if self.min_overlap_nm < 0 || self.min_overlap_nm > LENGTH_LIMIT_NM {
            return fail("min_overlap_nm must be from 0 and within the range");
        }
        if self.max_components == 0 || self.max_findings == 0 {
            return fail("max_components and max_findings must be above 0");
        }
        if self.max_file_bytes == 0 || self.max_step_entities == 0 {
            return fail("max_file_bytes and max_step_entities must be above 0");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_are_valid_and_the_shipped_file_parses_to_them() {
        let d = AssemblySettings::default();
        assert!(d.validate().is_ok());
        let shipped = include_str!("../../../config/assembly3d.cfg");
        assert_eq!(AssemblySettings::parse(shipped).unwrap(), d);
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let s = AssemblySettings::parse(
            "# note\nmax_component_height_nm = 0\ncheck_outline = false\nmin_overlap_nm = 5\n",
        )
        .unwrap();
        assert_eq!(s.max_component_height_nm, 0);
        assert!(!s.check_outline);
        assert_eq!(s.min_overlap_nm, 5);
        assert_eq!(s.max_components, AssemblySettings::default().max_components);
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("max_findings\n", "line 1: not key = value"),
            ("\nfoo = 1\n", "line 2: unknown key foo"),
            ("min_overlap_nm = -1\n", "min_overlap_nm must be"),
            ("check_outline = maybe\n", "the value of check_outline"),
            (
                "default_board_thickness_nm = 0\n",
                "default_board_thickness_nm",
            ),
            ("max_components = 0\n", "max_components and max_findings"),
            ("max_file_bytes = abc\n", "the value of max_file_bytes"),
        ] {
            let e = AssemblySettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }
}
