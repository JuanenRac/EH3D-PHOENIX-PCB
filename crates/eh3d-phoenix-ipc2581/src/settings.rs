// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the IPC-2581 writer. What goes into the file and every limit are read from a
//! plain `key = value` text, `config/ipc2581.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{Ipc2581Error, SETTINGS_INVALID};

/// The settings of one export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipc2581Settings {
    /// The date and time the history record of the file carries, as `YYYY-MM-DDTHH:MM:SS`. It is
    /// a setting and not the clock so that the same board always gives the same file.
    pub origination: String,
    /// Write the bill of materials (the parts grouped by value and footprint).
    pub include_bom: bool,
    /// Write the tracks.
    pub include_tracks: bool,
    /// Write the zones, as their outlines.
    pub include_zones: bool,
    /// The most objects (parts, connected pads, tracks, vias and zones) a board may hold for the
    /// file to be made.
    pub max_items: usize,
}

impl Default for Ipc2581Settings {
    fn default() -> Self {
        Ipc2581Settings {
            origination: "1970-01-01T00:00:00".to_owned(),
            include_bom: true,
            include_tracks: true,
            include_zones: true,
            max_items: 1_000_000,
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

/// True for a text of the form `YYYY-MM-DDTHH:MM:SS` with digits where they belong.
fn is_date_time(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 19
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            _ => c.is_ascii_digit(),
        })
}

impl Ipc2581Settings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<Ipc2581Settings, Ipc2581Error> {
        let mut s = Ipc2581Settings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                Ipc2581Error::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                Ipc2581Error::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "origination" => value.clone_into(&mut s.origination),
                "include_bom" => s.include_bom = parse_bool(value).ok_or_else(bad)?,
                "include_tracks" => s.include_tracks = parse_bool(value).ok_or_else(bad)?,
                "include_zones" => s.include_zones = parse_bool(value).ok_or_else(bad)?,
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(Ipc2581Error::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the writer relies on.
    pub fn validate(&self) -> Result<(), Ipc2581Error> {
        if !is_date_time(&self.origination) {
            return Err(Ipc2581Error::new(
                SETTINGS_INVALID,
                "origination must be like 1970-01-01T00:00:00",
            ));
        }
        if !(1..=100_000_000).contains(&self.max_items) {
            return Err(Ipc2581Error::new(
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
        assert!(Ipc2581Settings::default().validate().is_ok());
        assert_eq!(
            Ipc2581Settings::parse("# nothing\n\n").unwrap(),
            Ipc2581Settings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/ipc2581.cfg");
        assert_eq!(
            Ipc2581Settings::parse(text).unwrap(),
            Ipc2581Settings::default()
        );
    }

    #[test]
    fn lines_change_the_values() {
        let s = Ipc2581Settings::parse(
            "origination = 2000-02-03T04:05:06\ninclude_bom = false\ninclude_tracks = false\ninclude_zones = false\nmax_items = 9\n",
        )
        .unwrap();
        assert_eq!(s.origination, "2000-02-03T04:05:06");
        assert!(!s.include_bom && !s.include_tracks && !s.include_zones);
        assert_eq!(s.max_items, 9);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = Ipc2581Settings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        for text in [
            "origination = yesterday\n",
            "origination = 2000-02-03 04:05:06\n",
            "origination = 2000-02-03T04:05:0x\n",
            "include_bom = 1\n",
            "max_items = 0\n",
            "max_items = x\n",
            "include_zones\n",
        ] {
            assert!(Ipc2581Settings::parse(text).is_err(), "{text}");
        }
    }
}
