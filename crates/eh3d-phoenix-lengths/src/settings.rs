// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the lengths. Every delay, tolerance and limit is read from a plain
//! `key = value` text, `config/lengths.cfg`, and not buried in the code. A missing line keeps the
//! default; an unknown key or a value out of range is refused with its line number. The delay of
//! a single layer is a line `layer_delay_ps_per_mm.<layer name> = <value>`.

use std::collections::BTreeMap;

use crate::codes::{LengthError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The delays, tolerances and limits of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthSettings {
    /// The delay of a track on a layer that has none of its own and none from the stack-up, in
    /// picoseconds per millimetre.
    pub default_delay_ps_per_mm: f64,
    /// The delay of a via barrel, in picoseconds per millimetre.
    pub via_delay_ps_per_mm: f64,
    /// The length of a via when the stack-up does not give it, in nanometres.
    pub default_via_length_nm: i64,
    /// The delay of single layers, by the name of the layer, in picoseconds per millimetre.
    pub layer_delay_ps_per_mm: BTreeMap<String, f64>,
    /// Take the delay of a layer from the stack-up when one is given.
    pub use_stackup: bool,
    /// The frequency the delay of the stack-up is worked out at, in GHz.
    pub stackup_frequency_ghz: f64,
    /// The tolerance of a group that gives none, in nanometres.
    pub default_group_tolerance_nm: i64,
    /// The widest gap at which two tracks of a pair still count as coupled, in nanometres.
    pub pair_coupling_range_nm: i64,
    /// The tolerance of the separation of a pair that gives none, in nanometres.
    pub pair_gap_tolerance_nm: i64,
    /// The tolerance of the width of a pair that gives none, in nanometres.
    pub pair_width_tolerance_nm: i64,
    /// The most nets, groups and pairs a report handles.
    pub max_items: usize,
}

impl Default for LengthSettings {
    fn default() -> Self {
        LengthSettings {
            default_delay_ps_per_mm: 6.0,
            via_delay_ps_per_mm: 6.9,
            default_via_length_nm: 1_600_000,
            layer_delay_ps_per_mm: BTreeMap::new(),
            use_stackup: true,
            stackup_frequency_ghz: 1.0,
            default_group_tolerance_nm: 500_000,
            pair_coupling_range_nm: 1_000_000,
            pair_gap_tolerance_nm: 50_000,
            pair_width_tolerance_nm: 10_000,
            max_items: 100_000,
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

impl LengthSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<LengthSettings, LengthError> {
        let mut s = LengthSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                LengthError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                LengthError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            if let Some(name) = key.strip_prefix("layer_delay_ps_per_mm.") {
                let delay: f64 = value.parse().map_err(|_| bad())?;
                if name.is_empty() || !delay.is_finite() || delay <= 0.0 || delay > 100.0 {
                    return Err(bad());
                }
                s.layer_delay_ps_per_mm.insert(name.to_owned(), delay);
                continue;
            }
            match key {
                "default_delay_ps_per_mm" => {
                    s.default_delay_ps_per_mm = value.parse().map_err(|_| bad())?
                }
                "via_delay_ps_per_mm" => {
                    s.via_delay_ps_per_mm = value.parse().map_err(|_| bad())?
                }
                "default_via_length_nm" => {
                    s.default_via_length_nm = value.parse().map_err(|_| bad())?
                }
                "use_stackup" => s.use_stackup = parse_bool(value).ok_or_else(bad)?,
                "stackup_frequency_ghz" => {
                    s.stackup_frequency_ghz = value.parse().map_err(|_| bad())?
                }
                "default_group_tolerance_nm" => {
                    s.default_group_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "pair_coupling_range_nm" => {
                    s.pair_coupling_range_nm = value.parse().map_err(|_| bad())?
                }
                "pair_gap_tolerance_nm" => {
                    s.pair_gap_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "pair_width_tolerance_nm" => {
                    s.pair_width_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(LengthError::new(
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
    pub fn validate(&self) -> Result<(), LengthError> {
        let fail = |what: &str| Err(LengthError::new(SETTINGS_INVALID, what.to_string()));
        let delay = |v: f64| v.is_finite() && v > 0.0 && v <= 100.0;
        if !delay(self.default_delay_ps_per_mm) || !delay(self.via_delay_ps_per_mm) {
            return fail("a delay must be above 0 and at most 100 ps/mm");
        }
        if self.layer_delay_ps_per_mm.values().any(|v| !delay(*v)) {
            return fail("the delay of a layer must be above 0 and at most 100 ps/mm");
        }
        if !self.stackup_frequency_ghz.is_finite()
            || self.stackup_frequency_ghz <= 0.0
            || self.stackup_frequency_ghz > 1000.0
        {
            return fail("stackup_frequency_ghz must be above 0 and at most 1000");
        }
        for (name, v) in [
            ("default_via_length_nm", self.default_via_length_nm),
            (
                "default_group_tolerance_nm",
                self.default_group_tolerance_nm,
            ),
            ("pair_coupling_range_nm", self.pair_coupling_range_nm),
            ("pair_gap_tolerance_nm", self.pair_gap_tolerance_nm),
            ("pair_width_tolerance_nm", self.pair_width_tolerance_nm),
        ] {
            if !(0..LENGTH_LIMIT_NM).contains(&v) {
                return fail(&format!("{name} must be between 0 and 2^30 nanometres"));
            }
        }
        if !(1..=10_000_000).contains(&self.max_items) {
            return fail("max_items must be between 1 and 10000000");
        }
        Ok(())
    }

    /// The delay of a layer in femtoseconds per millimetre, when the settings give one.
    pub fn layer_delay_fs_per_mm(&self, layer: &str) -> Option<i64> {
        self.layer_delay_ps_per_mm.get(layer).map(|v| to_fs(*v))
    }
}

/// Picoseconds per millimetre as whole femtoseconds per millimetre.
pub(crate) fn to_fs(ps_per_mm: f64) -> i64 {
    (ps_per_mm * 1000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(LengthSettings::default().validate().is_ok());
        assert_eq!(
            LengthSettings::parse("# nothing\n\n").unwrap(),
            LengthSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/lengths.cfg");
        assert_eq!(
            LengthSettings::parse(text).unwrap(),
            LengthSettings::default()
        );
    }

    #[test]
    fn lines_change_the_values_and_a_layer_has_a_delay_of_its_own() {
        let s = LengthSettings::parse(
            "default_delay_ps_per_mm = 5.5\nlayer_delay_ps_per_mm.F.Cu = 5.9\nuse_stackup = false\n",
        )
        .unwrap();
        assert_eq!(to_fs(s.default_delay_ps_per_mm), 5_500);
        assert!(!s.use_stackup);
        assert_eq!(s.layer_delay_fs_per_mm("F.Cu"), Some(5_900));
        assert_eq!(s.layer_delay_fs_per_mm("In1.Cu"), None);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = LengthSettings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        assert!(LengthSettings::parse("\ndefault_delay_ps_per_mm = 0\n").is_err());
        assert!(LengthSettings::parse("default_delay_ps_per_mm = x\n").is_err());
        assert!(LengthSettings::parse("layer_delay_ps_per_mm. = 5\n").is_err());
        assert!(LengthSettings::parse("pair_gap_tolerance_nm = -1\n").is_err());
        assert!(LengthSettings::parse("use_stackup = maybe\n").is_err());
        assert!(LengthSettings::parse("use_stackup\n").is_err());
    }
}
