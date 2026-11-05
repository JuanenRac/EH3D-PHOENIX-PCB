// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the synchronisation, read from `config/sync.cfg` (which is also built into the
//! program as the default). Nothing that is tuned is written in the code of the planner.

use eh3d_phoenix_board::Side;

use crate::codes::SETTINGS_INVALID;

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1 << 20;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/sync.cfg");

/// Why a settings file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// The file is larger than [`MAX_SETTINGS_BYTES`].
    #[error("the settings file is {size} bytes, the limit is {limit}")]
    TooLarge {
        /// Its size.
        size: usize,
        /// The limit.
        limit: usize,
    },
    /// A line is not valid.
    #[error("line {line}: {message}")]
    Line {
        /// The line, from 1.
        line: usize,
        /// What is wrong.
        message: String,
    },
}

impl SettingsError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        SETTINGS_INVALID
    }
}

/// The settings of the synchronisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSettings {
    /// X of the first new component, in nanometres.
    pub new_part_x_nm: i64,
    /// Y of the new components, in nanometres.
    pub new_part_y_nm: i64,
    /// The distance along X between new components, in nanometres.
    pub new_part_step_nm: i64,
    /// The side of the board a new component is put on.
    pub new_part_side: Side,
    /// Remove a component whose part is no longer in the schematic.
    pub remove_missing_components: bool,
    /// Rename a net of the board instead of making a new one when the pads agree.
    pub rename_nets: bool,
    /// Write the footprint of the board back into a part of the schematic.
    pub backward_footprints: bool,
    /// How many times the rename of a written file is tried again.
    pub write_retries: u32,
    /// How long to wait between two tries, in milliseconds.
    pub write_retry_ms: u64,
}

impl Default for SyncSettings {
    /// The settings file of the repository (`config/sync.cfg`).
    fn default() -> Self {
        let mut settings = SyncSettings::empty();
        if settings.apply(BUILT_IN).is_err() {
            return SyncSettings::empty();
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

impl SyncSettings {
    fn empty() -> SyncSettings {
        SyncSettings {
            new_part_x_nm: 0,
            new_part_y_nm: 0,
            new_part_step_nm: 0,
            new_part_side: Side::Top,
            remove_missing_components: false,
            rename_nets: false,
            backward_footprints: false,
            write_retries: 0,
            write_retry_ms: 0,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<SyncSettings, SettingsError> {
        let mut settings = SyncSettings::default();
        settings.apply(text)?;
        Ok(settings)
    }

    fn apply(&mut self, text: &str) -> Result<(), SettingsError> {
        if text.len() > MAX_SETTINGS_BYTES {
            return Err(SettingsError::TooLarge {
                size: text.len(),
                limit: MAX_SETTINGS_BYTES,
            });
        }
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| SettingsError::Line { line, message };
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || fail(format!("the value of {key} is not valid"));
            match key {
                "new_part_x_nm" => self.new_part_x_nm = value.parse().map_err(|_| bad())?,
                "new_part_y_nm" => self.new_part_y_nm = value.parse().map_err(|_| bad())?,
                "new_part_step_nm" => self.new_part_step_nm = value.parse().map_err(|_| bad())?,
                "new_part_side" => {
                    self.new_part_side = match value {
                        "top" => Side::Top,
                        "bottom" => Side::Bottom,
                        _ => return Err(bad()),
                    }
                }
                "remove_missing_components" => {
                    self.remove_missing_components = parse_bool(value).ok_or_else(bad)?;
                }
                "rename_nets" => self.rename_nets = parse_bool(value).ok_or_else(bad)?,
                "backward_footprints" => {
                    self.backward_footprints = parse_bool(value).ok_or_else(bad)?;
                }
                "write_retries" => self.write_retries = value.parse().map_err(|_| bad())?,
                "write_retry_ms" => self.write_retry_ms = value.parse().map_err(|_| bad())?,
                _ => return Err(fail(format!("unknown key {key}"))),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = SyncSettings::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, SyncSettings::default());
        let s = SyncSettings::default();
        assert_eq!(s.new_part_step_nm, 5_000_000);
        assert_eq!(s.new_part_side, Side::Top);
        assert!(!s.remove_missing_components && s.rename_nets && s.backward_footprints);
        assert_eq!((s.write_retries, s.write_retry_ms), (5, 50));
    }

    #[test]
    fn a_line_overrides_the_default_and_the_rest_stays() {
        let s = SyncSettings::parse(
            "new_part_x_nm = 7\nnew_part_side = bottom\nremove_missing_components = true\n",
        )
        .unwrap();
        assert_eq!(s.new_part_x_nm, 7);
        assert_eq!(s.new_part_side, Side::Bottom);
        assert!(s.remove_missing_components && s.rename_nets);
        assert_eq!(
            SyncSettings::parse("# none\n").unwrap(),
            SyncSettings::default()
        );
    }

    #[test]
    fn bad_settings_name_their_line_and_never_panic() {
        for (text, line) in [
            ("nonsense", 1),
            ("\nunknown = 1", 2),
            ("new_part_x_nm = x", 1),
            ("new_part_side = left", 1),
            ("rename_nets = maybe", 1),
            ("write_retries = -1", 1),
        ] {
            match SyncSettings::parse(text) {
                Err(SettingsError::Line { line: found, .. }) => assert_eq!(found, line, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
        let big = " ".repeat(MAX_SETTINGS_BYTES + 1);
        let error = SyncSettings::parse(&big).unwrap_err();
        assert!(matches!(error, SettingsError::TooLarge { .. }));
        assert_eq!(error.code(), SETTINGS_INVALID);
    }
}
