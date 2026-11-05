// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the board editor, read from `config/boardedit.cfg` (which is also built into
//! the program as the default). Nothing that is tuned is written in the code of the editor.

use eh3d_phoenix_route::RouteSettings;

use crate::codes::{BoardEditError, SETTINGS_INVALID};

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1 << 20;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/boardedit.cfg");

/// The settings of the board editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEditSettings {
    /// The grid that points snap to, in nanometres.
    pub grid_nm: i64,
    /// Whether snapping is on.
    pub snap: bool,
    /// How far from a track or a via a click still picks it, in nanometres.
    pub pick_tolerance_nm: i64,
    /// The space added around the pads of a component to make the box that picks it.
    pub part_padding_nm: i64,
    /// Whether the design rule check runs after every operation.
    pub live_drc: bool,
    /// Whether an operation that adds a rule error is refused.
    pub reject_new_errors: bool,
    /// Whether the tracks on the pads of a component follow it when it moves.
    pub follow_tracks: bool,
    /// Whether tracks may use 45-degree pieces.
    pub diagonal_routes: bool,
    /// Whether a corner goes first along x.
    pub corner_horizontal_first: bool,
    /// The width of a track whose net class asks for no other, in nanometres.
    pub track_width_nm: i64,
    /// The drill of a via, in nanometres.
    pub via_drill_nm: i64,
    /// The pad diameter of a via, in nanometres.
    pub via_diameter_nm: i64,
    /// The amplitude the meanders start with, in nanometres.
    pub meander_amplitude_nm: i64,
    /// The largest amplitude of a meander, in nanometres.
    pub meander_max_amplitude_nm: i64,
    /// The run of one half period of a meander, in nanometres.
    pub meander_pitch_nm: i64,
    /// How far from the target the length of a tuned net may end, in nanometres.
    pub length_tolerance_nm: i64,
    /// How many places (the longest straight pieces) are tried before a tuning is refused.
    pub meander_candidates: usize,
}

impl Default for BoardEditSettings {
    /// The settings file of the repository (`config/boardedit.cfg`).
    fn default() -> Self {
        let mut settings = BoardEditSettings::empty();
        if settings.apply(BUILT_IN).is_err() {
            return BoardEditSettings::empty();
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

impl BoardEditSettings {
    fn empty() -> BoardEditSettings {
        BoardEditSettings {
            grid_nm: 1,
            snap: false,
            pick_tolerance_nm: 0,
            part_padding_nm: 0,
            live_drc: false,
            reject_new_errors: false,
            follow_tracks: false,
            diagonal_routes: false,
            corner_horizontal_first: true,
            track_width_nm: 1,
            via_drill_nm: 1,
            via_diameter_nm: 1,
            meander_amplitude_nm: 1,
            meander_max_amplitude_nm: 1,
            meander_pitch_nm: 1,
            length_tolerance_nm: 0,
            meander_candidates: 1,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<BoardEditSettings, BoardEditError> {
        let mut settings = BoardEditSettings::default();
        settings.apply(text)?;
        Ok(settings)
    }

    fn apply(&mut self, text: &str) -> Result<(), BoardEditError> {
        if text.len() > MAX_SETTINGS_BYTES {
            return Err(BoardEditError::new(
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
                BoardEditError::new(SETTINGS_INVALID, format!("line {line}: {message}"))
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
                "live_drc" => self.live_drc = parse_bool(value).ok_or_else(bad)?,
                "reject_new_errors" => {
                    self.reject_new_errors = parse_bool(value).ok_or_else(bad)?
                }
                "follow_tracks" => self.follow_tracks = parse_bool(value).ok_or_else(bad)?,
                "diagonal_routes" => self.diagonal_routes = parse_bool(value).ok_or_else(bad)?,
                "corner_horizontal_first" => {
                    self.corner_horizontal_first = parse_bool(value).ok_or_else(bad)?;
                }
                "track_width_nm" => self.track_width_nm = value.parse().map_err(|_| bad())?,
                "via_drill_nm" => self.via_drill_nm = value.parse().map_err(|_| bad())?,
                "via_diameter_nm" => self.via_diameter_nm = value.parse().map_err(|_| bad())?,
                "meander_amplitude_nm" => {
                    self.meander_amplitude_nm = value.parse().map_err(|_| bad())?;
                }
                "meander_max_amplitude_nm" => {
                    self.meander_max_amplitude_nm = value.parse().map_err(|_| bad())?;
                }
                "meander_pitch_nm" => self.meander_pitch_nm = value.parse().map_err(|_| bad())?,
                "length_tolerance_nm" => {
                    self.length_tolerance_nm = value.parse().map_err(|_| bad())?;
                }
                "meander_candidates" => {
                    self.meander_candidates = value.parse().map_err(|_| bad())?;
                }
                _ => return Err(fail(format!("unknown key {key}"))),
            }
        }
        self.check()
    }

    /// Checks the ranges of the values.
    fn check(&self) -> Result<(), BoardEditError> {
        let limit = 1i64 << 29;
        let ranges = [
            ("grid_nm", self.grid_nm, 1),
            ("pick_tolerance_nm", self.pick_tolerance_nm, 0),
            ("part_padding_nm", self.part_padding_nm, 0),
            ("track_width_nm", self.track_width_nm, 1),
            ("via_drill_nm", self.via_drill_nm, 1),
            ("via_diameter_nm", self.via_diameter_nm, 1),
            ("meander_amplitude_nm", self.meander_amplitude_nm, 1),
            ("meander_max_amplitude_nm", self.meander_max_amplitude_nm, 1),
            ("meander_pitch_nm", self.meander_pitch_nm, 1),
            ("length_tolerance_nm", self.length_tolerance_nm, 0),
        ];
        for (name, value, low) in ranges {
            if !(low..=limit).contains(&value) {
                return Err(BoardEditError::new(
                    SETTINGS_INVALID,
                    format!("{name} must be between {low} and {limit}"),
                ));
            }
        }
        if self.meander_max_amplitude_nm < self.meander_amplitude_nm {
            return Err(BoardEditError::new(
                SETTINGS_INVALID,
                "meander_max_amplitude_nm must not be smaller than meander_amplitude_nm",
            ));
        }
        if self.meander_candidates == 0 || self.meander_candidates > 64 {
            return Err(BoardEditError::new(
                SETTINGS_INVALID,
                "meander_candidates must be between 1 and 64",
            ));
        }
        Ok(())
    }

    /// The settings of the router that the editor uses: the width of a track and the size of a
    /// via come from here, the rest is the default of the router.
    pub fn route_settings(&self) -> RouteSettings {
        RouteSettings {
            default_width_nm: self.track_width_nm,
            via_drill_nm: self.via_drill_nm,
            via_diameter_nm: self.via_diameter_nm,
            ..RouteSettings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = BoardEditSettings::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, BoardEditSettings::default());
        let s = BoardEditSettings::default();
        assert_eq!(s.grid_nm, 250_000);
        assert!(s.snap && s.live_drc && s.reject_new_errors && s.follow_tracks);
        assert!(s.diagonal_routes && s.corner_horizontal_first);
        assert_eq!(s.track_width_nm, 250_000);
        assert_eq!(s.meander_candidates, 3);
        let route = s.route_settings();
        assert_eq!(route.default_width_nm, 250_000);
        assert_eq!(route.via_diameter_nm, 600_000);
    }

    #[test]
    fn a_line_overrides_the_default_and_the_rest_stays() {
        let s = BoardEditSettings::parse("grid_nm = 500000\nsnap = false\n").unwrap();
        assert_eq!(s.grid_nm, 500_000);
        assert!(!s.snap);
        assert!(s.live_drc);
        assert_eq!(
            BoardEditSettings::parse("# nothing\n").unwrap(),
            BoardEditSettings::default()
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
            "track_width_nm = 0",
            "meander_candidates = 0",
            "meander_candidates = 65",
            "meander_max_amplitude_nm = 10",
            "length_tolerance_nm = -1",
        ] {
            let error = BoardEditSettings::parse(text).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
        }
        let big = " ".repeat(MAX_SETTINGS_BYTES + 1);
        assert!(BoardEditSettings::parse(&big).is_err());
    }
}
