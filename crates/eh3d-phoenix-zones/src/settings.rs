// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the filling of zones. Every radius, width, limit and default is read from a
//! plain `key = value` text, `config/zones.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{ZoneError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// How a zone joins a pad or a via of its own net.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Connection {
    /// The copper reaches the pad only through spokes, with a gap around the rest.
    Thermal,
    /// The copper runs up to the pad.
    Solid,
    /// The pad is kept apart like copper of another net.
    None,
}

impl Connection {
    /// The word used in the files.
    pub fn as_str(self) -> &'static str {
        match self {
            Connection::Thermal => "thermal",
            Connection::Solid => "solid",
            Connection::None => "none",
        }
    }

    /// Reads the word.
    pub fn from_word(word: &str) -> Option<Connection> {
        match word {
            "thermal" => Some(Connection::Thermal),
            "solid" => Some(Connection::Solid),
            "none" => Some(Connection::None),
            _ => None,
        }
    }
}

/// The radii, widths, limits and defaults of one run. Lengths are in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneSettings {
    /// The sides of the polygon that holds a circle (8 or 16).
    pub circle_sides: u32,
    /// How far from exact the cut points and the shared borders may be, in nanometres.
    pub snap_tolerance_nm: i64,
    /// The narrowest a copper island may be.
    pub min_width_nm: i64,
    /// The smallest area of a copper island, in square nanometres.
    pub min_island_area_nm2: i64,
    /// Keep the islands that touch no copper of the net.
    pub keep_unconnected_islands: bool,
    /// How a pad of the net is joined.
    pub pad_connection: Connection,
    /// How a via of the net is joined.
    pub via_connection: Connection,
    /// The width of the gap of a thermal relief.
    pub thermal_gap_nm: i64,
    /// The width of each spoke of a thermal relief.
    pub thermal_spoke_width_nm: i64,
    /// The number of spokes (2 or 4).
    pub thermal_spokes: u32,
    /// The inner radius of a relief is at least this, whatever the pad.
    pub thermal_inner_radius_nm: i64,
    /// The outer radius of a relief is at least this.
    pub thermal_outer_radius_nm: i64,
    /// The largest angle one piece of an arc spans, in whole degrees.
    pub arc_piece_degrees: u32,
    /// The most pieces of copper one zone may be cut into.
    pub max_fragments: usize,
    /// The most points of the outline of a zone.
    pub max_outline_points: usize,
    /// The radius of the rounded corners of the fill and of the thermal reliefs, in nanometres;
    /// zero leaves every corner sharp.
    pub corner_radius_nm: i64,
    /// The most an arc of a rounded corner may leave its chords, in nanometres.
    pub corner_tolerance_nm: i64,
}

impl Default for ZoneSettings {
    fn default() -> Self {
        ZoneSettings {
            circle_sides: 16,
            snap_tolerance_nm: 2,
            min_width_nm: 150_000,
            min_island_area_nm2: 100_000_000_000,
            keep_unconnected_islands: false,
            pad_connection: Connection::Thermal,
            via_connection: Connection::Solid,
            thermal_gap_nm: 250_000,
            thermal_spoke_width_nm: 250_000,
            thermal_spokes: 4,
            thermal_inner_radius_nm: 0,
            thermal_outer_radius_nm: 0,
            arc_piece_degrees: 10,
            max_fragments: 200_000,
            max_outline_points: 10_000,
            corner_radius_nm: 0,
            corner_tolerance_nm: 1_000,
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

impl ZoneSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<ZoneSettings, ZoneError> {
        let mut s = ZoneSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                ZoneError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                ZoneError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "circle_sides" => s.circle_sides = value.parse().map_err(|_| bad())?,
                "snap_tolerance_nm" => s.snap_tolerance_nm = value.parse().map_err(|_| bad())?,
                "min_width_nm" => s.min_width_nm = value.parse().map_err(|_| bad())?,
                "min_island_area_nm2" => {
                    s.min_island_area_nm2 = value.parse().map_err(|_| bad())?
                }
                "keep_unconnected_islands" => {
                    s.keep_unconnected_islands = parse_bool(value).ok_or_else(bad)?
                }
                "pad_connection" => {
                    s.pad_connection = Connection::from_word(value).ok_or_else(bad)?
                }
                "via_connection" => {
                    s.via_connection = Connection::from_word(value).ok_or_else(bad)?
                }
                "thermal_gap_nm" => s.thermal_gap_nm = value.parse().map_err(|_| bad())?,
                "thermal_spoke_width_nm" => {
                    s.thermal_spoke_width_nm = value.parse().map_err(|_| bad())?
                }
                "thermal_spokes" => s.thermal_spokes = value.parse().map_err(|_| bad())?,
                "thermal_inner_radius_nm" => {
                    s.thermal_inner_radius_nm = value.parse().map_err(|_| bad())?
                }
                "thermal_outer_radius_nm" => {
                    s.thermal_outer_radius_nm = value.parse().map_err(|_| bad())?
                }
                "arc_piece_degrees" => s.arc_piece_degrees = value.parse().map_err(|_| bad())?,
                "max_fragments" => s.max_fragments = value.parse().map_err(|_| bad())?,
                "max_outline_points" => s.max_outline_points = value.parse().map_err(|_| bad())?,
                "corner_radius_nm" => s.corner_radius_nm = value.parse().map_err(|_| bad())?,
                "corner_tolerance_nm" => {
                    s.corner_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                _ => {
                    return Err(ZoneError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the algorithms rely on.
    pub fn validate(&self) -> Result<(), ZoneError> {
        let fail = |what: &str| Err(ZoneError::new(SETTINGS_INVALID, what.to_string()));
        let length = |v: i64| (0..LENGTH_LIMIT_NM).contains(&v);
        if self.circle_sides != 8 && self.circle_sides != 16 {
            return fail("circle_sides must be 8 or 16");
        }
        if !(0..=1_000).contains(&self.snap_tolerance_nm) {
            return fail("snap_tolerance_nm must be between 0 and 1000");
        }
        if !length(self.min_width_nm)
            || !length(self.thermal_gap_nm)
            || !length(self.thermal_inner_radius_nm)
            || !length(self.thermal_outer_radius_nm)
        {
            return fail("a width, a gap or a radius is outside 0 to 2^30 nanometres");
        }
        if !(1..LENGTH_LIMIT_NM).contains(&self.thermal_spoke_width_nm) {
            return fail("thermal_spoke_width_nm must be at least 1 and below 2^30");
        }
        if self.min_island_area_nm2 < 0 {
            return fail("min_island_area_nm2 must not be negative");
        }
        if self.thermal_spokes != 2 && self.thermal_spokes != 4 {
            return fail("thermal_spokes must be 2 or 4");
        }
        if !(1..=90).contains(&self.arc_piece_degrees) {
            return fail("arc_piece_degrees must be between 1 and 90");
        }
        if !(1..=10_000_000).contains(&self.max_fragments) {
            return fail("max_fragments must be between 1 and 10000000");
        }
        if !(3..=1_000_000).contains(&self.max_outline_points) {
            return fail("max_outline_points must be between 3 and 1000000");
        }
        if !length(self.corner_radius_nm) {
            return fail("corner_radius_nm is outside 0 to 2^30 nanometres");
        }
        if !(1..=1_000_000).contains(&self.corner_tolerance_nm) {
            return fail("corner_tolerance_nm must be between 1 and 1000000");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(ZoneSettings::default().validate().is_ok());
        assert_eq!(
            ZoneSettings::parse("# nothing\n\n").unwrap_or_default(),
            ZoneSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/zones.cfg");
        assert_eq!(
            ZoneSettings::parse(text).unwrap_or_default(),
            ZoneSettings::default()
        );
    }

    #[test]
    fn lines_change_the_values_and_the_connection_words_are_read() {
        let s = ZoneSettings::parse(
            "thermal_gap_nm = 300000\nthermal_spokes = 2\npad_connection = solid\nkeep_unconnected_islands = true\n",
        )
        .unwrap_or_default();
        assert_eq!(s.thermal_gap_nm, 300_000);
        assert_eq!(s.thermal_spokes, 2);
        assert_eq!(s.pad_connection, Connection::Solid);
        assert!(s.keep_unconnected_islands);
        assert_eq!(Connection::Thermal.as_str(), "thermal");
        assert_eq!(Connection::from_word("none"), Some(Connection::None));
        assert_eq!(Connection::from_word("star"), None);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = ZoneSettings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        let shape = ZoneSettings::parse("\nthermal_spokes = 3\n").unwrap_err();
        assert!(shape.detail.contains("thermal_spokes"), "{}", shape.detail);
        assert!(ZoneSettings::parse("circle_sides = 12\n").is_err());
        assert!(ZoneSettings::parse("min_width_nm = x\n").is_err());
        assert!(ZoneSettings::parse("min_width_nm\n").is_err());
    }
}
