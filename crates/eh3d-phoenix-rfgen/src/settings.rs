// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the generator. Every default of a structure, every limit and every rule of the
//! check of the result is read from a plain `key = value` text, `config/rfgen.cfg`, and not buried
//! in the code. A missing line keeps the default; an unknown key or a value out of range is refused
//! with its line number.

use eh3d_phoenix_model::Nm;
use eh3d_phoenix_rules::RuleValues;

use crate::codes::{RfgenError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The defaults, limits and rules of one run. Lengths are in nanometres, impedances in ohm and
/// angles in degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct RfgenSettings {
    /// The impedance of the lines when none is given.
    pub default_z0_ohm: f64,
    /// The electrical length of a line when none is given.
    pub default_line_degrees: f64,
    /// The length of the leads that join a structure to the rest of the board.
    pub lead_length_nm: i64,
    /// A mitered bend needs arms at least this many widths long.
    pub miter_min_arm_over_width: f64,
    /// The radius of a curved bend in widths of the track.
    pub curve_radius_over_width: f64,
    /// The electrical length of a width transition.
    pub taper_length_degrees: f64,
    /// The electrical length of the main line that carries a stub.
    pub stub_main_degrees: f64,
    /// The impedance of the series line that stands for a series inductance.
    pub lmatch_series_line_ohm: f64,
    /// The longest series line of an L network, in degrees.
    pub lmatch_max_series_degrees: f64,
    /// The drill of the vias.
    pub via_drill_nm: i64,
    /// The pad diameter of the vias.
    pub via_diameter_nm: i64,
    /// The impedance of the narrow sections of a stepped-impedance low-pass filter.
    pub lowpass_z_high_ohm: f64,
    /// The impedance of the wide sections of a stepped-impedance low-pass filter.
    pub lowpass_z_low_ohm: f64,
    /// The gap kept between the open ends of resonators that face each other.
    pub bandpass_end_gap_nm: i64,
    /// The fractional bandwidth of a band-pass filter when none is given, in percent.
    pub bandpass_bandwidth_percent: f64,
    /// The most sections a filter may have.
    pub max_filter_order: usize,
    /// The tolerance of the search for a coupled pair, in ohm.
    pub solver_tolerance_ohm: f64,
    /// The most steps of that search.
    pub solver_max_iterations: u32,
    /// The distance between the arms of a Wilkinson divider, in widths of an arm.
    pub wilkinson_pitch_over_width: f64,
    /// The gap between the output arms where the isolation resistor goes.
    pub wilkinson_resistor_gap_nm: i64,
    /// The gap on each side of the feed line where it enters a patch.
    pub patch_notch_gap_nm: i64,
    /// The shortening of a printed dipole against a half wavelength in the substrate.
    pub dipole_shortening: f64,
    /// The width of the strips of a printed dipole.
    pub dipole_strip_width_nm: i64,
    /// The gap at the feed point of a printed dipole.
    pub dipole_feed_gap_nm: i64,
    /// The length of the feed strips of a printed dipole.
    pub dipole_feed_length_nm: i64,
    /// The steps of the integral of the mutual conductance of a patch.
    pub patch_integral_steps: u32,
    /// The width of the signal pad of the SMA launch.
    pub sma_signal_pad_width_nm: i64,
    /// The length of the pads of the SMA launch.
    pub sma_pad_length_nm: i64,
    /// The width of the ground pads of the SMA launch.
    pub sma_ground_pad_width_nm: i64,
    /// The gap between the ground copper and the widest of the signal pad and the line.
    pub sma_ground_gap_nm: i64,
    /// The spacing of the vias of a fence as a fraction of the guided wavelength.
    pub fence_spacing_wavelengths: f64,
    /// The copper margin around each via of the fence.
    pub fence_gap_nm: i64,
    /// Add a ground pour on the reference layer under the structure.
    pub ground_pour: bool,
    /// How far the ground pour goes beyond the structure.
    pub ground_margin_nm: i64,
    /// The clearance written into the zones that hold the copper shapes.
    pub zone_clearance_nm: i64,
    /// A shape narrower than this raises a warning.
    pub min_feature_nm: i64,
    /// Refuse a result that adds an error to the rule check.
    pub rules_gate: bool,
    /// The narrowest track of the rule check.
    pub rule_min_track_width_nm: i64,
    /// The copper clearance of the rule check.
    pub rule_min_clearance_nm: i64,
    /// The smallest via diameter of the rule check.
    pub rule_min_via_diameter_nm: i64,
    /// The smallest drill of the rule check.
    pub rule_min_drill_nm: i64,
    /// The thinnest annular ring of the rule check.
    pub rule_min_annular_ring_nm: i64,
    /// The distance to the board edge of the rule check.
    pub rule_min_edge_clearance_nm: i64,
    /// The distance between drill walls of the rule check.
    pub rule_min_drill_spacing_nm: i64,
    /// The most items one structure may have.
    pub max_items: usize,
    /// How many ids are tried when a new part needs a free one.
    pub id_attempts: usize,
}

impl Default for RfgenSettings {
    fn default() -> Self {
        RfgenSettings {
            default_z0_ohm: 50.0,
            default_line_degrees: 90.0,
            lead_length_nm: 3_000_000,
            miter_min_arm_over_width: 2.0,
            curve_radius_over_width: 3.0,
            taper_length_degrees: 20.0,
            stub_main_degrees: 90.0,
            lmatch_series_line_ohm: 100.0,
            lmatch_max_series_degrees: 45.0,
            via_drill_nm: 300_000,
            via_diameter_nm: 600_000,
            lowpass_z_high_ohm: 120.0,
            lowpass_z_low_ohm: 20.0,
            bandpass_end_gap_nm: 300_000,
            bandpass_bandwidth_percent: 10.0,
            max_filter_order: 9,
            solver_tolerance_ohm: 0.001,
            solver_max_iterations: 100,
            wilkinson_pitch_over_width: 3.0,
            wilkinson_resistor_gap_nm: 1_500_000,
            patch_notch_gap_nm: 500_000,
            dipole_shortening: 0.95,
            dipole_strip_width_nm: 1_500_000,
            dipole_feed_gap_nm: 1_000_000,
            dipole_feed_length_nm: 8_000_000,
            patch_integral_steps: 400,
            sma_signal_pad_width_nm: 1_500_000,
            sma_pad_length_nm: 2_500_000,
            sma_ground_pad_width_nm: 2_000_000,
            sma_ground_gap_nm: 400_000,
            fence_spacing_wavelengths: 0.05,
            fence_gap_nm: 400_000,
            ground_pour: false,
            ground_margin_nm: 5_000_000,
            zone_clearance_nm: 200_000,
            min_feature_nm: 100_000,
            rules_gate: true,
            rule_min_track_width_nm: 100_000,
            rule_min_clearance_nm: 100_000,
            rule_min_via_diameter_nm: 400_000,
            rule_min_drill_nm: 200_000,
            rule_min_annular_ring_nm: 100_000,
            rule_min_edge_clearance_nm: 0,
            rule_min_drill_spacing_nm: 250_000,
            max_items: 100_000,
            id_attempts: 100_000,
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

impl RfgenSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<RfgenSettings, RfgenError> {
        let mut s = RfgenSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                RfgenError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                RfgenError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            let real = || value.parse::<f64>().map_err(|_| bad());
            let int = || value.parse::<i64>().map_err(|_| bad());
            match key {
                "default_z0_ohm" => s.default_z0_ohm = real()?,
                "default_line_degrees" => s.default_line_degrees = real()?,
                "lead_length_nm" => s.lead_length_nm = int()?,
                "miter_min_arm_over_width" => s.miter_min_arm_over_width = real()?,
                "curve_radius_over_width" => s.curve_radius_over_width = real()?,
                "taper_length_degrees" => s.taper_length_degrees = real()?,
                "stub_main_degrees" => s.stub_main_degrees = real()?,
                "lmatch_series_line_ohm" => s.lmatch_series_line_ohm = real()?,
                "lmatch_max_series_degrees" => s.lmatch_max_series_degrees = real()?,
                "via_drill_nm" => s.via_drill_nm = int()?,
                "via_diameter_nm" => s.via_diameter_nm = int()?,
                "lowpass_z_high_ohm" => s.lowpass_z_high_ohm = real()?,
                "lowpass_z_low_ohm" => s.lowpass_z_low_ohm = real()?,
                "bandpass_end_gap_nm" => s.bandpass_end_gap_nm = int()?,
                "bandpass_bandwidth_percent" => s.bandpass_bandwidth_percent = real()?,
                "max_filter_order" => {
                    s.max_filter_order = usize::try_from(int()?).map_err(|_| bad())?
                }
                "solver_tolerance_ohm" => s.solver_tolerance_ohm = real()?,
                "solver_max_iterations" => {
                    s.solver_max_iterations = u32::try_from(int()?).map_err(|_| bad())?
                }
                "wilkinson_pitch_over_width" => s.wilkinson_pitch_over_width = real()?,
                "wilkinson_resistor_gap_nm" => s.wilkinson_resistor_gap_nm = int()?,
                "patch_notch_gap_nm" => s.patch_notch_gap_nm = int()?,
                "dipole_shortening" => s.dipole_shortening = real()?,
                "dipole_strip_width_nm" => s.dipole_strip_width_nm = int()?,
                "dipole_feed_gap_nm" => s.dipole_feed_gap_nm = int()?,
                "dipole_feed_length_nm" => s.dipole_feed_length_nm = int()?,
                "patch_integral_steps" => {
                    s.patch_integral_steps = u32::try_from(int()?).map_err(|_| bad())?
                }
                "sma_signal_pad_width_nm" => s.sma_signal_pad_width_nm = int()?,
                "sma_pad_length_nm" => s.sma_pad_length_nm = int()?,
                "sma_ground_pad_width_nm" => s.sma_ground_pad_width_nm = int()?,
                "sma_ground_gap_nm" => s.sma_ground_gap_nm = int()?,
                "fence_spacing_wavelengths" => s.fence_spacing_wavelengths = real()?,
                "fence_gap_nm" => s.fence_gap_nm = int()?,
                "ground_pour" => s.ground_pour = parse_bool(value).ok_or_else(bad)?,
                "ground_margin_nm" => s.ground_margin_nm = int()?,
                "zone_clearance_nm" => s.zone_clearance_nm = int()?,
                "min_feature_nm" => s.min_feature_nm = int()?,
                "rules_gate" => s.rules_gate = parse_bool(value).ok_or_else(bad)?,
                "rule_min_track_width_nm" => s.rule_min_track_width_nm = int()?,
                "rule_min_clearance_nm" => s.rule_min_clearance_nm = int()?,
                "rule_min_via_diameter_nm" => s.rule_min_via_diameter_nm = int()?,
                "rule_min_drill_nm" => s.rule_min_drill_nm = int()?,
                "rule_min_annular_ring_nm" => s.rule_min_annular_ring_nm = int()?,
                "rule_min_edge_clearance_nm" => s.rule_min_edge_clearance_nm = int()?,
                "rule_min_drill_spacing_nm" => s.rule_min_drill_spacing_nm = int()?,
                "max_items" => s.max_items = usize::try_from(int()?).map_err(|_| bad())?,
                "id_attempts" => s.id_attempts = usize::try_from(int()?).map_err(|_| bad())?,
                _ => {
                    return Err(RfgenError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the generators rely on.
    pub fn validate(&self) -> Result<(), RfgenError> {
        let fail = |what: &str| Err(RfgenError::new(SETTINGS_INVALID, what.to_string()));
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let length = |v: i64| (1..LENGTH_LIMIT_NM).contains(&v);
        let length_or_zero = |v: i64| (0..LENGTH_LIMIT_NM).contains(&v);
        if !positive(self.default_z0_ohm) || self.default_z0_ohm > 1000.0 {
            return fail("default_z0_ohm must be above zero and at most 1000");
        }
        if !positive(self.default_line_degrees) || self.default_line_degrees > 3600.0 {
            return fail("default_line_degrees must be above zero and at most 3600");
        }
        if !length(self.lead_length_nm) {
            return fail("lead_length_nm must be at least 1 and below 2^30");
        }
        if !positive(self.miter_min_arm_over_width) || self.miter_min_arm_over_width > 100.0 {
            return fail("miter_min_arm_over_width must be above zero and at most 100");
        }
        if !positive(self.curve_radius_over_width) || self.curve_radius_over_width > 1000.0 {
            return fail("curve_radius_over_width must be above zero and at most 1000");
        }
        if !positive(self.taper_length_degrees) || self.taper_length_degrees > 720.0 {
            return fail("taper_length_degrees must be above zero and at most 720");
        }
        if !positive(self.stub_main_degrees) || self.stub_main_degrees > 720.0 {
            return fail("stub_main_degrees must be above zero and at most 720");
        }
        if !positive(self.lmatch_series_line_ohm) || self.lmatch_series_line_ohm > 1000.0 {
            return fail("lmatch_series_line_ohm must be above zero and at most 1000");
        }
        if !positive(self.lmatch_max_series_degrees) || self.lmatch_max_series_degrees > 90.0 {
            return fail("lmatch_max_series_degrees must be above zero and at most 90");
        }
        if !length(self.via_drill_nm)
            || self.via_diameter_nm <= self.via_drill_nm
            || self.via_diameter_nm >= LENGTH_LIMIT_NM
        {
            return fail("via_drill_nm must be at least 1 and below via_diameter_nm");
        }
        if !positive(self.lowpass_z_high_ohm)
            || !positive(self.lowpass_z_low_ohm)
            || self.lowpass_z_high_ohm <= self.lowpass_z_low_ohm
            || self.lowpass_z_high_ohm > 1000.0
        {
            return fail(
                "lowpass_z_high_ohm must be above lowpass_z_low_ohm, which must be above zero",
            );
        }
        if !length_or_zero(self.bandpass_end_gap_nm) {
            return fail("bandpass_end_gap_nm must be between 0 and 2^30");
        }
        if !positive(self.bandpass_bandwidth_percent) || self.bandpass_bandwidth_percent >= 100.0 {
            return fail("bandpass_bandwidth_percent must be above zero and below 100");
        }
        if !(1..=64).contains(&self.max_filter_order) {
            return fail("max_filter_order must be between 1 and 64");
        }
        if !positive(self.solver_tolerance_ohm) || self.solver_max_iterations < 1 {
            return fail(
                "solver_tolerance_ohm must be above zero and solver_max_iterations at least 1",
            );
        }
        if !positive(self.wilkinson_pitch_over_width) || self.wilkinson_pitch_over_width > 100.0 {
            return fail("wilkinson_pitch_over_width must be above zero and at most 100");
        }
        if !length(self.wilkinson_resistor_gap_nm) || !length(self.patch_notch_gap_nm) {
            return fail("wilkinson_resistor_gap_nm and patch_notch_gap_nm must be at least 1");
        }
        if !positive(self.dipole_shortening) || self.dipole_shortening > 1.0 {
            return fail("dipole_shortening must be above zero and at most 1");
        }
        if !length(self.dipole_strip_width_nm)
            || !length(self.dipole_feed_gap_nm)
            || !length(self.dipole_feed_length_nm)
        {
            return fail("the dipole lengths must be at least 1 and below 2^30");
        }
        if !(10..=100_000).contains(&self.patch_integral_steps) {
            return fail("patch_integral_steps must be between 10 and 100000");
        }
        if !length(self.sma_signal_pad_width_nm)
            || !length(self.sma_pad_length_nm)
            || !length(self.sma_ground_pad_width_nm)
            || !length(self.sma_ground_gap_nm)
        {
            return fail("the SMA lengths must be at least 1 and below 2^30");
        }
        if !positive(self.fence_spacing_wavelengths) || self.fence_spacing_wavelengths > 0.5 {
            return fail("fence_spacing_wavelengths must be above zero and at most 0.5");
        }
        if !length(self.fence_gap_nm) {
            return fail("fence_gap_nm must be at least 1 and below 2^30");
        }
        if !length_or_zero(self.ground_margin_nm) {
            return fail("ground_margin_nm must be between 0 and 2^30");
        }
        if !length_or_zero(self.zone_clearance_nm) || !length_or_zero(self.min_feature_nm) {
            return fail("zone_clearance_nm and min_feature_nm must be between 0 and 2^30");
        }
        for value in [
            self.rule_min_track_width_nm,
            self.rule_min_clearance_nm,
            self.rule_min_via_diameter_nm,
            self.rule_min_drill_nm,
            self.rule_min_annular_ring_nm,
            self.rule_min_edge_clearance_nm,
            self.rule_min_drill_spacing_nm,
        ] {
            if !length_or_zero(value) {
                return fail("the rule limits must be between 0 and 2^30");
            }
        }
        if self.max_items < 1 || self.id_attempts < 1 {
            return fail("max_items and id_attempts must be at least 1");
        }
        Ok(())
    }

    /// The values of the rule check of the result.
    pub fn rule_values(&self) -> Result<RuleValues, RfgenError> {
        let nm = |v: i64| {
            Nm::new(v).map_err(|e| RfgenError::new(SETTINGS_INVALID, format!("a rule limit: {e}")))
        };
        Ok(RuleValues {
            min_track_width: nm(self.rule_min_track_width_nm)?,
            min_clearance: nm(self.rule_min_clearance_nm)?,
            min_drill: nm(self.rule_min_drill_nm)?,
            min_via_diameter: nm(self.rule_min_via_diameter_nm)?,
            min_annular_ring: nm(self.rule_min_annular_ring_nm)?,
            min_edge_clearance: nm(self.rule_min_edge_clearance_nm)?,
            min_drill_spacing: nm(self.rule_min_drill_spacing_nm)?,
            ..RuleValues::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            RfgenSettings::parse("# nothing\n").unwrap(),
            RfgenSettings::default()
        );
        let s = RfgenSettings::parse("lead_length_nm = 5000000\nground_pour = true\n").unwrap();
        assert_eq!(s.lead_length_nm, 5_000_000);
        assert!(s.ground_pour);
        assert_eq!(s.via_drill_nm, RfgenSettings::default().via_drill_nm);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("lead_length_nm = wide", "lead_length_nm"),
            ("lead_length_nm = 0", "lead_length_nm"),
            ("colour = red", "unknown key"),
            ("oops", "key = value"),
            ("ground_pour = 1", "ground_pour"),
            ("via_diameter_nm = 100", "via_drill_nm"),
            ("lowpass_z_high_ohm = 10", "lowpass_z_high_ohm"),
            ("dipole_shortening = 1.5", "dipole_shortening"),
            ("max_filter_order = -3", "max_filter_order"),
            ("fence_spacing_wavelengths = 0", "fence_spacing_wavelengths"),
        ] {
            let e = RfgenSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = RfgenSettings::parse("\n\nlead_length_nm = x").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/rfgen.cfg");
        assert_eq!(
            RfgenSettings::parse(text).unwrap(),
            RfgenSettings::default()
        );
    }

    #[test]
    fn the_rule_values_follow_the_settings() {
        let rules = RfgenSettings::default().rule_values().unwrap();
        assert_eq!(rules.min_clearance.get(), 100_000);
        assert_eq!(rules.min_edge_clearance.get(), 0);
        assert_eq!(rules.min_drill_spacing.get(), 250_000);
    }
}
