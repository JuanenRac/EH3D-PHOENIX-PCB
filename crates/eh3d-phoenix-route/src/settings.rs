// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the router and of the editing logic. Every pitch, penalty, limit and default
//! is read from a plain `key = value` text, `config/route.cfg`, and not buried in the code. A
//! missing line keeps the default; an unknown key or a value out of range is refused with its line
//! number.

use crate::codes::{RouteError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The pitches, penalties, limits and defaults of one run. Lengths are in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSettings {
    /// The pitch of the coarse mesh of the search, anchored at the start of the track.
    pub grid_nm: i64,
    /// How far beyond the box of the two ends the search may go.
    pub search_margin_nm: i64,
    /// The most mesh points (times copper layers) of one search; the pitch is doubled until the
    /// area fits.
    pub max_cells: usize,
    /// The most states taken out of the queue before the search gives up.
    pub max_expansions: usize,
    /// The cost of one corner, as nanometres of track.
    pub corner_penalty_nm: i64,
    /// The cost of one via, as nanometres of track.
    pub via_penalty_nm: i64,
    /// Allow turns sharper than 90 degrees.
    pub allow_acute: bool,
    /// How many mesh pitches around the end the search tries to finish with one or two pieces.
    pub final_reach_cells: i64,
    /// The width of a track whose net has no class that gives one.
    pub default_width_nm: i64,
    /// The drill of a via made by the router.
    pub via_drill_nm: i64,
    /// The pad diameter of a via made by the router.
    pub via_diameter_nm: i64,
    /// A margin added to every clearance.
    pub clearance_margin_nm: i64,
    /// The side of the squares of the index of obstacles.
    pub bucket_nm: i64,
    /// The shortest track that the mode that stops at obstacles still makes.
    pub stop_min_length_nm: i64,
    /// Refuse an edit that would add a rule error.
    pub strict_edits: bool,
    /// Moving a vertex may not turn a 0, 45 or 90 degree piece into another angle.
    pub keep_angles: bool,
    /// The most pairs of points the ratsnest compares.
    pub ratsnest_max_pairs: u64,
    /// How many ids are tried when a new part needs a free one.
    pub id_attempts: usize,
}

impl Default for RouteSettings {
    fn default() -> Self {
        RouteSettings {
            grid_nm: 250_000,
            search_margin_nm: 10_000_000,
            max_cells: 200_000,
            max_expansions: 3_000_000,
            corner_penalty_nm: 500_000,
            via_penalty_nm: 5_000_000,
            allow_acute: false,
            final_reach_cells: 4,
            default_width_nm: 250_000,
            via_drill_nm: 300_000,
            via_diameter_nm: 600_000,
            clearance_margin_nm: 1_000,
            bucket_nm: 2_000_000,
            stop_min_length_nm: 100_000,
            strict_edits: true,
            keep_angles: true,
            ratsnest_max_pairs: 50_000_000,
            id_attempts: 64,
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

impl RouteSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<RouteSettings, RouteError> {
        let mut s = RouteSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                RouteError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                RouteError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "grid_nm" => s.grid_nm = value.parse().map_err(|_| bad())?,
                "search_margin_nm" => s.search_margin_nm = value.parse().map_err(|_| bad())?,
                "max_cells" => s.max_cells = value.parse().map_err(|_| bad())?,
                "max_expansions" => s.max_expansions = value.parse().map_err(|_| bad())?,
                "corner_penalty_nm" => s.corner_penalty_nm = value.parse().map_err(|_| bad())?,
                "via_penalty_nm" => s.via_penalty_nm = value.parse().map_err(|_| bad())?,
                "allow_acute" => s.allow_acute = parse_bool(value).ok_or_else(bad)?,
                "final_reach_cells" => s.final_reach_cells = value.parse().map_err(|_| bad())?,
                "default_width_nm" => s.default_width_nm = value.parse().map_err(|_| bad())?,
                "via_drill_nm" => s.via_drill_nm = value.parse().map_err(|_| bad())?,
                "via_diameter_nm" => s.via_diameter_nm = value.parse().map_err(|_| bad())?,
                "clearance_margin_nm" => {
                    s.clearance_margin_nm = value.parse().map_err(|_| bad())?
                }
                "bucket_nm" => s.bucket_nm = value.parse().map_err(|_| bad())?,
                "stop_min_length_nm" => s.stop_min_length_nm = value.parse().map_err(|_| bad())?,
                "strict_edits" => s.strict_edits = parse_bool(value).ok_or_else(bad)?,
                "keep_angles" => s.keep_angles = parse_bool(value).ok_or_else(bad)?,
                "ratsnest_max_pairs" => s.ratsnest_max_pairs = value.parse().map_err(|_| bad())?,
                "id_attempts" => s.id_attempts = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(RouteError::new(
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
    pub fn validate(&self) -> Result<(), RouteError> {
        let fail = |what: &str| Err(RouteError::new(SETTINGS_INVALID, what.to_string()));
        let length = |v: i64| (0..LENGTH_LIMIT_NM).contains(&v);
        if !(1..LENGTH_LIMIT_NM).contains(&self.grid_nm) {
            return fail("grid_nm must be at least 1 and below 2^30");
        }
        if !length(self.search_margin_nm) || !length(self.clearance_margin_nm) {
            return fail("search_margin_nm and clearance_margin_nm must be between 0 and 2^30");
        }
        if !(1..=1_000_000).contains(&self.max_cells) {
            return fail("max_cells must be between 1 and 1000000");
        }
        if self.max_expansions < 1 {
            return fail("max_expansions must be at least 1");
        }
        let penalty = |v: i64| (0..=1_000_000_000).contains(&v);
        if !penalty(self.corner_penalty_nm) || !penalty(self.via_penalty_nm) {
            return fail("corner_penalty_nm and via_penalty_nm must be between 0 and 10^9");
        }
        if !(1..=64).contains(&self.final_reach_cells) {
            return fail("final_reach_cells must be between 1 and 64");
        }
        if !(1..LENGTH_LIMIT_NM).contains(&self.default_width_nm) {
            return fail("default_width_nm must be at least 1 and below 2^30");
        }
        if self.via_drill_nm < 1 || self.via_diameter_nm <= self.via_drill_nm {
            return fail("via_drill_nm must be at least 1 and below via_diameter_nm");
        }
        if self.via_diameter_nm >= LENGTH_LIMIT_NM {
            return fail("via_diameter_nm must be below 2^30");
        }
        if !(10_000..LENGTH_LIMIT_NM).contains(&self.bucket_nm) {
            return fail("bucket_nm must be between 10000 and 2^30");
        }
        if !length(self.stop_min_length_nm) {
            return fail("stop_min_length_nm must be between 0 and 2^30");
        }
        if self.ratsnest_max_pairs < 1 {
            return fail("ratsnest_max_pairs must be at least 1");
        }
        if self.id_attempts < 1 {
            return fail("id_attempts must be at least 1");
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
            RouteSettings::parse("# nothing\n").unwrap(),
            RouteSettings::default()
        );
        let s = RouteSettings::parse("grid_nm = 100000\nallow_acute = true\nid_attempts = 3\n")
            .unwrap();
        assert_eq!(s.grid_nm, 100_000);
        assert!(s.allow_acute);
        assert_eq!(s.id_attempts, 3);
        assert_eq!(s.via_penalty_nm, RouteSettings::default().via_penalty_nm);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("grid_nm = wide", "grid_nm"),
            ("grid_nm = 0", "grid_nm"),
            ("colour = red", "unknown key"),
            ("oops", "key = value"),
            ("allow_acute = 1", "allow_acute"),
            ("max_cells = 0", "max_cells"),
            ("final_reach_cells = 99", "final_reach_cells"),
            ("via_diameter_nm = 100", "via_drill_nm"),
            ("bucket_nm = 5", "bucket_nm"),
        ] {
            let e = RouteSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = RouteSettings::parse("\n\ngrid_nm = x").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/route.cfg");
        assert_eq!(
            RouteSettings::parse(text).unwrap(),
            RouteSettings::default()
        );
    }
}
