// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the schematic editor, read from `config/schedit.cfg` (which is also built into
//! the program as the default). Nothing that is tuned is written in the code of the editor.

use crate::codes::{SchEditError, SETTINGS_INVALID};

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1 << 20;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/schedit.cfg");

/// The settings of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchEditSettings {
    /// The grid that points snap to, in nanometres.
    pub grid_nm: i64,
    /// Whether snapping is on.
    pub snap: bool,
    /// How far from something a click still picks it or joins it, in nanometres.
    pub pick_tolerance_nm: i64,
    /// The space added around the pins of a part to make the box that picks it, in nanometres.
    pub part_padding_nm: i64,
    /// How far a pasted copy lies from the copy when there is no target, in nanometres.
    pub paste_offset_nm: i64,
    /// The distance between the labels of a bus, in nanometres.
    pub bus_step_nm: i64,
    /// How many steps can be undone.
    pub history_limit: usize,
    /// Whether the electrical rule check runs after every change.
    pub live_erc: bool,
    /// Whether a wire that turns a corner goes along x first.
    pub wire_horizontal_first: bool,
    /// How many symbols a search of the palette lists.
    pub palette_results: usize,
    /// The name of the sheet that is added to a schematic that has none.
    pub new_sheet_name: String,
    /// Its width, in nanometres.
    pub new_sheet_width_nm: i64,
    /// Its height, in nanometres.
    pub new_sheet_height_nm: i64,
}

impl Default for SchEditSettings {
    /// The settings file of the repository (`config/schedit.cfg`).
    fn default() -> Self {
        let mut settings = SchEditSettings::empty();
        if settings.apply(BUILT_IN).is_err() {
            return SchEditSettings::empty();
        }
        settings
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl SchEditSettings {
    fn empty() -> SchEditSettings {
        SchEditSettings {
            grid_nm: 1,
            snap: false,
            pick_tolerance_nm: 0,
            part_padding_nm: 0,
            paste_offset_nm: 0,
            bus_step_nm: 0,
            history_limit: 1,
            live_erc: false,
            wire_horizontal_first: true,
            palette_results: 1,
            new_sheet_name: String::new(),
            new_sheet_width_nm: 1,
            new_sheet_height_nm: 1,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<SchEditSettings, SchEditError> {
        let mut settings = SchEditSettings::default();
        settings.apply(text)?;
        Ok(settings)
    }

    fn apply(&mut self, text: &str) -> Result<(), SchEditError> {
        if text.len() > MAX_SETTINGS_BYTES {
            return Err(SchEditError::new(
                SETTINGS_INVALID,
                format!(
                    "the file is {} bytes, the limit is {MAX_SETTINGS_BYTES}",
                    text.len()
                ),
            ));
        }
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| {
                SchEditError::new(SETTINGS_INVALID, format!("line {line}: {message}"))
            };
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || fail(format!("the value of {key} is not valid"));
            match key {
                "grid_nm" => self.grid_nm = value.parse().map_err(|_| bad())?,
                "snap" => self.snap = parse_bool(value).ok_or_else(bad)?,
                "pick_tolerance_nm" => self.pick_tolerance_nm = value.parse().map_err(|_| bad())?,
                "part_padding_nm" => self.part_padding_nm = value.parse().map_err(|_| bad())?,
                "paste_offset_nm" => self.paste_offset_nm = value.parse().map_err(|_| bad())?,
                "bus_step_nm" => self.bus_step_nm = value.parse().map_err(|_| bad())?,
                "history_limit" => self.history_limit = value.parse().map_err(|_| bad())?,
                "live_erc" => self.live_erc = parse_bool(value).ok_or_else(bad)?,
                "wire_horizontal_first" => {
                    self.wire_horizontal_first = parse_bool(value).ok_or_else(bad)?;
                }
                "palette_results" => self.palette_results = value.parse().map_err(|_| bad())?,
                "new_sheet_name" => {
                    if value.is_empty() || value.chars().any(char::is_control) {
                        return Err(bad());
                    }
                    self.new_sheet_name = value.to_owned();
                }
                "new_sheet_width_nm" => {
                    self.new_sheet_width_nm = value.parse().map_err(|_| bad())?;
                }
                "new_sheet_height_nm" => {
                    self.new_sheet_height_nm = value.parse().map_err(|_| bad())?;
                }
                _ => return Err(fail(format!("unknown key {key}"))),
            }
        }
        self.check()
    }

    /// Checks the ranges of the values.
    fn check(&self) -> Result<(), SchEditError> {
        let limit = 1i64 << 29;
        let ranges = [
            ("grid_nm", self.grid_nm, 1, limit),
            ("pick_tolerance_nm", self.pick_tolerance_nm, 0, limit),
            ("part_padding_nm", self.part_padding_nm, 0, limit),
            ("paste_offset_nm", self.paste_offset_nm, -limit, limit),
            ("bus_step_nm", self.bus_step_nm, 1, limit),
            ("new_sheet_width_nm", self.new_sheet_width_nm, 1, limit),
            ("new_sheet_height_nm", self.new_sheet_height_nm, 1, limit),
        ];
        for (name, value, low, high) in ranges {
            if !(low..=high).contains(&value) {
                return Err(SchEditError::new(
                    SETTINGS_INVALID,
                    format!("{name} must be between {low} and {high}"),
                ));
            }
        }
        if self.history_limit == 0 || self.palette_results == 0 {
            return Err(SchEditError::new(
                SETTINGS_INVALID,
                "history_limit and palette_results must be at least 1",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = SchEditSettings::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, SchEditSettings::default());
        let s = SchEditSettings::default();
        assert_eq!(s.grid_nm, 2_540_000);
        assert!(s.snap && s.live_erc && s.wire_horizontal_first);
        assert_eq!(s.history_limit, 1000);
        assert_eq!(s.new_sheet_name, "Sheet 1");
        assert_eq!(
            (s.new_sheet_width_nm, s.new_sheet_height_nm),
            (297_000_000, 210_000_000)
        );
    }

    #[test]
    fn a_line_overrides_the_default_and_the_rest_stays() {
        let s = SchEditSettings::parse("grid_nm = 1270000\nsnap = false\n").unwrap();
        assert_eq!(s.grid_nm, 1_270_000);
        assert!(!s.snap);
        assert!(s.live_erc);
        assert_eq!(
            SchEditSettings::parse("# nothing\n").unwrap(),
            SchEditSettings::default()
        );
    }

    #[test]
    fn bad_settings_are_refused_with_the_code_and_never_panic() {
        for text in [
            "nonsense",
            "unknown = 1",
            "grid_nm = x",
            "grid_nm = 0",
            "snap = maybe",
            "history_limit = 0",
            "palette_results = -1",
            "new_sheet_name = ",
            "bus_step_nm = 0",
        ] {
            let error = SchEditSettings::parse(text).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
        }
        let big = " ".repeat(MAX_SETTINGS_BYTES + 1);
        assert!(SchEditSettings::parse(&big).is_err());
    }
}
