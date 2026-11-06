// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the KiCad readers. Every limit is read from a settings text,
//! `config/kicad.cfg`, and not buried in the readers.

use crate::error::KicadError;

/// The limits of the KiCad readers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KicadSettings {
    /// The largest file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The deepest nesting of lists in an s-expression file.
    pub max_depth: usize,
    /// The most nodes (atoms, strings and lists) an s-expression file may have.
    pub max_nodes: usize,
    /// The longest atom or quoted string, in bytes.
    pub max_string_bytes: usize,
    /// The most items of one kind (tracks, wires, pads, ...) read from a file.
    pub max_items: usize,
    /// The most sheets read through a hierarchy.
    pub max_sheets: usize,
    /// The deepest nesting of sheets.
    pub max_sheet_depth: usize,
    /// The most diagnostics kept for a file.
    pub max_diagnostics: usize,
    /// The oldest file version that the tests cover.
    pub min_tested_version: u64,
    /// The newest file version that the tests cover.
    pub max_tested_version: u64,
    /// The largest project file that is read, in bytes.
    pub max_project_bytes: u64,
    /// The straight pieces that replace a quarter turn of an arc of the board outline.
    pub outline_arc_steps_per_quarter: usize,
    /// The most point-in-polygon tests (corners of a rule area times edges of a zone) that the
    /// assignment of the rule areas that keep the pour out to the zones may make.
    pub max_keepout_checks: u64,
    /// The most closed rings of the board outline (the outline and its openings) that are kept
    /// and tested for lying inside the largest one.
    pub max_outline_rings: usize,
}

impl Default for KicadSettings {
    fn default() -> Self {
        KicadSettings {
            max_file_bytes: 128 * 1024 * 1024,
            max_depth: 64,
            max_nodes: 5_000_000,
            max_string_bytes: 1024 * 1024,
            max_items: 2_000_000,
            max_sheets: 256,
            max_sheet_depth: 8,
            max_diagnostics: 1000,
            min_tested_version: 20_211_014,
            max_tested_version: 20_251_231,
            max_project_bytes: 4 * 1024 * 1024,
            outline_arc_steps_per_quarter: 8,
            max_keepout_checks: 100_000_000,
            max_outline_rings: 10_000,
        }
    }
}

/// The most straight pieces per quarter turn that the settings may ask for.
const MAX_ARC_STEPS: usize = 4096;

fn whole(value: &str, line: usize, key: &str) -> Result<u64, KicadError> {
    let parsed: u64 = value.parse().map_err(|_| KicadError::InvalidSettings {
        line,
        reason: format!("{key}: not a whole number"),
    })?;
    if parsed == 0 {
        return Err(KicadError::InvalidSettings {
            line,
            reason: format!("{key}: must be above zero"),
        });
    }
    Ok(parsed)
}

fn count(value: &str, line: usize, key: &str) -> Result<usize, KicadError> {
    usize::try_from(whole(value, line, key)?).map_err(|_| KicadError::InvalidSettings {
        line,
        reason: format!("{key}: too large"),
    })
}

impl KicadSettings {
    /// Parses a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error.
    pub fn parse(text: &str) -> Result<KicadSettings, KicadError> {
        let mut s = KicadSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) =
                trimmed
                    .split_once('=')
                    .ok_or_else(|| KicadError::InvalidSettings {
                        line,
                        reason: "expected key = value".to_string(),
                    })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "max_file_bytes" => s.max_file_bytes = whole(value, line, key)?,
                "max_depth" => s.max_depth = count(value, line, key)?,
                "max_nodes" => s.max_nodes = count(value, line, key)?,
                "max_string_bytes" => s.max_string_bytes = count(value, line, key)?,
                "max_items" => s.max_items = count(value, line, key)?,
                "max_sheets" => s.max_sheets = count(value, line, key)?,
                "max_sheet_depth" => s.max_sheet_depth = count(value, line, key)?,
                "max_diagnostics" => s.max_diagnostics = count(value, line, key)?,
                "min_tested_version" => s.min_tested_version = whole(value, line, key)?,
                "max_tested_version" => s.max_tested_version = whole(value, line, key)?,
                "max_project_bytes" => s.max_project_bytes = whole(value, line, key)?,
                "outline_arc_steps_per_quarter" => {
                    s.outline_arc_steps_per_quarter = count(value, line, key)?;
                    if s.outline_arc_steps_per_quarter > MAX_ARC_STEPS {
                        return Err(KicadError::InvalidSettings {
                            line,
                            reason: format!("{key}: more than {MAX_ARC_STEPS}"),
                        });
                    }
                }
                "max_keepout_checks" => s.max_keepout_checks = whole(value, line, key)?,
                "max_outline_rings" => s.max_outline_rings = count(value, line, key)?,
                _ => {
                    return Err(KicadError::InvalidSettings {
                        line,
                        reason: format!("unknown key {key}"),
                    })
                }
            }
        }
        if s.min_tested_version > s.max_tested_version {
            return Err(KicadError::InvalidSettings {
                line: 0,
                reason: "min_tested_version is above max_tested_version".to_string(),
            });
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_settings_text_is_the_default() {
        let text = include_str!("../../../config/kicad.cfg");
        assert_eq!(KicadSettings::parse(text), Ok(KicadSettings::default()));
    }

    #[test]
    fn a_key_overrides_its_default_and_the_rest_stay() {
        let s = KicadSettings::parse("# note\n\nmax_depth = 8\nmax_items=5\n").unwrap();
        assert_eq!(s.max_depth, 8);
        assert_eq!(s.max_items, 5);
        assert_eq!(s.max_nodes, KicadSettings::default().max_nodes);
    }

    #[test]
    fn bad_lines_are_errors_with_their_line() {
        for (text, line) in [
            ("max_depth", 1),
            ("\nmax_depth = x", 2),
            ("max_depth = 0", 1),
            ("\n\nwhat = 3", 3),
            ("max_file_bytes = -4", 1),
            ("outline_arc_steps_per_quarter = 5000", 1),
            ("max_keepout_checks = 0", 1),
            ("max_outline_rings = x", 1),
        ] {
            match KicadSettings::parse(text) {
                Err(KicadError::InvalidSettings { line: l, .. }) => assert_eq!(l, line, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
        assert!(KicadSettings::parse("min_tested_version = 5\nmax_tested_version = 4").is_err());
    }
}
