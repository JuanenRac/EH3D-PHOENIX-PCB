// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the library editor. Every limit, tolerance and grid is read from a plain
//! `key = value` text, `config/libedit.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{LibError, SETTINGS_INVALID};

/// The settings of the editor, of the checks and of the window of libraries.
#[derive(Debug, Clone, PartialEq)]
pub struct LibSettings {
    /// How many steps of undo are kept.
    pub history_limit: usize,
    /// The largest library file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The most nested elements of one library file.
    pub xml_max_depth: usize,
    /// The most elements of one library file.
    pub xml_max_nodes: usize,
    /// How far outside the box of the pads the origin of a footprint may be before it is
    /// reported, in nanometres.
    pub origin_tolerance_nm: i64,
    /// The clearance between silkscreen and a pad below which it is reported, in nanometres.
    pub silk_clearance_nm: i64,
    /// The layers of silkscreen that are checked against the pads.
    pub silk_layers: Vec<u32>,
    /// The layers on which a surface-mount pad has copper.
    pub copper_layers: Vec<u32>,
    /// True when a footprint without the texts `>NAME` and `>VALUE` is reported.
    pub require_name_texts: bool,
    /// The fine grid of the editor of footprints, in nanometres.
    pub grid_nm: i64,
    /// The grid of the editor of symbols, in nanometres (50 mil).
    pub symbol_grid_nm: i64,
    /// True when the cursor snaps to the grid.
    pub snap: bool,
    /// The distance in screen points at which a click picks an item.
    pub pick_px: u32,
    /// The expansion of the solder mask openings the window makes, in nanometres.
    pub mask_expansion_nm: i64,
    /// The expansion of the solder paste openings the window makes, in nanometres.
    pub paste_expansion_nm: i64,
}

impl Default for LibSettings {
    fn default() -> Self {
        LibSettings {
            history_limit: 200,
            max_file_bytes: 256 * 1024 * 1024,
            xml_max_depth: 256,
            xml_max_nodes: 20_000_000,
            origin_tolerance_nm: 100_000,
            silk_clearance_nm: 0,
            silk_layers: vec![21, 22],
            copper_layers: vec![1, 16],
            require_name_texts: true,
            grid_nm: 50_000,
            symbol_grid_nm: 1_270_000,
            snap: true,
            pick_px: 8,
            mask_expansion_nm: 50_000,
            paste_expansion_nm: 0,
        }
    }
}

fn parse_list(value: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(part.parse().ok()?);
    }
    Some(out)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl LibSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<LibSettings, LibError> {
        let mut s = LibSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                LibError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                LibError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "history_limit" => s.history_limit = value.parse().map_err(|_| bad())?,
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "xml_max_depth" => s.xml_max_depth = value.parse().map_err(|_| bad())?,
                "xml_max_nodes" => s.xml_max_nodes = value.parse().map_err(|_| bad())?,
                "origin_tolerance_nm" => {
                    s.origin_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "silk_clearance_nm" => s.silk_clearance_nm = value.parse().map_err(|_| bad())?,
                "silk_layers" => s.silk_layers = parse_list(value).ok_or_else(bad)?,
                "copper_layers" => s.copper_layers = parse_list(value).ok_or_else(bad)?,
                "require_name_texts" => s.require_name_texts = parse_bool(value).ok_or_else(bad)?,
                "grid_nm" => s.grid_nm = value.parse().map_err(|_| bad())?,
                "symbol_grid_nm" => s.symbol_grid_nm = value.parse().map_err(|_| bad())?,
                "snap" => s.snap = parse_bool(value).ok_or_else(bad)?,
                "pick_px" => s.pick_px = value.parse().map_err(|_| bad())?,
                "mask_expansion_nm" => s.mask_expansion_nm = value.parse().map_err(|_| bad())?,
                "paste_expansion_nm" => s.paste_expansion_nm = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(LibError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges.
    pub fn validate(&self) -> Result<(), LibError> {
        let bad = |what: &str| Err(LibError::new(SETTINGS_INVALID, what.to_owned()));
        if self.history_limit == 0 || self.history_limit > 100_000 {
            return bad("history_limit must be between 1 and 100000");
        }
        if self.xml_max_depth == 0 || self.xml_max_nodes == 0 || self.max_file_bytes == 0 {
            return bad("the limits of the file must not be zero");
        }
        if !(1..=100_000_000).contains(&self.grid_nm)
            || !(1..=100_000_000).contains(&self.symbol_grid_nm)
        {
            return bad("the grids must be between 1 nm and 100 mm");
        }
        if self.origin_tolerance_nm < 0 || self.silk_clearance_nm < 0 {
            return bad("the tolerances must not be negative");
        }
        if self.pick_px == 0 || self.pick_px > 100 {
            return bad("pick_px must be between 1 and 100");
        }
        if self.mask_expansion_nm.abs() > 5_000_000 || self.paste_expansion_nm.abs() > 5_000_000 {
            return bad("the expansions must be within 5 mm");
        }
        Ok(())
    }

    /// The limits of the XML parser.
    pub fn limits(&self) -> eh3d_phoenix_eagle::Limits {
        eh3d_phoenix_eagle::Limits {
            max_bytes: usize::try_from(self.max_file_bytes).unwrap_or(usize::MAX),
            max_depth: self.xml_max_depth,
            max_nodes: self.xml_max_nodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_are_valid() {
        assert_eq!(LibSettings::default().validate(), Ok(()));
    }

    #[test]
    fn a_settings_text_changes_only_the_lines_it_has() {
        let s = LibSettings::parse(
            "# grids\ngrid_nm = 25000\nsnap = false\nsilk_layers = 21, 22, 51\n\n",
        )
        .unwrap();
        assert_eq!(s.grid_nm, 25_000);
        assert!(!s.snap);
        assert_eq!(s.silk_layers, vec![21, 22, 51]);
        assert_eq!(s.history_limit, 200);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let e = LibSettings::parse("snap = true\nfoo = 1\n").unwrap_err();
        assert_eq!(e.code, SETTINGS_INVALID);
        assert!(e.detail.contains("line 2") && e.detail.contains("foo"));
        let e = LibSettings::parse("grid_nm = x\n").unwrap_err();
        assert!(e.detail.contains("line 1"));
        assert!(LibSettings::parse("snap\n").is_err());
        assert!(LibSettings::parse("grid_nm = 0\n").is_err());
        assert!(LibSettings::parse("history_limit = 0\n").is_err());
        assert!(LibSettings::parse("pick_px = 500\n").is_err());
    }

    #[test]
    fn the_file_in_the_repository_has_the_defaults() {
        let text = include_str!("../../../config/libedit.cfg");
        assert_eq!(LibSettings::parse(text).unwrap(), LibSettings::default());
    }

    #[test]
    fn the_limits_follow_the_settings() {
        let s = LibSettings {
            xml_max_depth: 7,
            ..LibSettings::default()
        };
        assert_eq!(s.limits().max_depth, 7);
        assert_eq!(s.limits().max_bytes, 256 * 1024 * 1024);
    }
}
