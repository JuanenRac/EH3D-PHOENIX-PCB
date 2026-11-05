// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the calculations. Every tuned value (a tolerance, a limit, a physical constant
//! that depends on the copper used) is read from a settings text, `config/stackup.cfg`, and not
//! buried in the formulas.

use crate::error::StackupError;

/// The tuned values of the stack-up calculations.
#[derive(Clone, Debug, PartialEq)]
pub struct StackupSettings {
    /// Conductivity of the copper, in S/m. The default is the value of the International
    /// Annealed Copper Standard, 5.8e7 S/m (100 % IACS); electrodeposited foil is a little lower.
    pub copper_conductivity_s_per_m: f64,
    /// A conductor thinner than this many skin depths raises a warning, because the skin-effect
    /// resistance assumes a conductor thick compared with the skin depth.
    pub thin_conductor_skin_depths: f64,
    /// The height of the substrate (or the spacing of the planes) above this fraction of the
    /// guided wavelength raises a warning, because the quasi-static formulas ignore dispersion.
    pub quasi_static_height_fraction: f64,
    /// The highest frequency that is accepted, in GHz.
    pub max_frequency_ghz: f64,
    /// The distance to the target impedance, in ohm, at which the synthesis stops.
    pub synth_tolerance_ohm: f64,
    /// The most steps the synthesis takes.
    pub synth_max_iterations: u32,
    /// The narrowest track the synthesis tries, in nanometres.
    pub synth_min_width_nm: i64,
    /// The widest track the synthesis tries, in nanometres.
    pub synth_max_width_nm: i64,
}

impl Default for StackupSettings {
    fn default() -> Self {
        StackupSettings {
            copper_conductivity_s_per_m: 5.8e7,
            thin_conductor_skin_depths: 3.0,
            quasi_static_height_fraction: 0.05,
            max_frequency_ghz: 1000.0,
            synth_tolerance_ohm: 0.01,
            synth_max_iterations: 100,
            synth_min_width_nm: 50_000,
            synth_max_width_nm: 20_000_000,
        }
    }
}

fn number(value: &str, line: usize, key: &str) -> Result<f64, StackupError> {
    let bad = |reason: &str| StackupError::InvalidSettings {
        line,
        reason: format!("{key}: {reason}"),
    };
    let parsed: f64 = value.parse().map_err(|_| bad("not a number"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(bad("must be a finite number above zero"));
    }
    Ok(parsed)
}

fn integer(value: &str, line: usize, key: &str) -> Result<i64, StackupError> {
    let bad = |reason: &str| StackupError::InvalidSettings {
        line,
        reason: format!("{key}: {reason}"),
    };
    let parsed: i64 = value.parse().map_err(|_| bad("not a whole number"))?;
    if parsed <= 0 {
        return Err(bad("must be above zero"));
    }
    Ok(parsed)
}

impl StackupSettings {
    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error.
    pub fn parse(text: &str) -> Result<StackupSettings, StackupError> {
        let mut settings = StackupSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) =
                trimmed
                    .split_once('=')
                    .ok_or_else(|| StackupError::InvalidSettings {
                        line,
                        reason: "expected key = value".to_string(),
                    })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "copper_conductivity_s_per_m" => {
                    settings.copper_conductivity_s_per_m = number(value, line, key)?
                }
                "thin_conductor_skin_depths" => {
                    settings.thin_conductor_skin_depths = number(value, line, key)?
                }
                "quasi_static_height_fraction" => {
                    settings.quasi_static_height_fraction = number(value, line, key)?
                }
                "max_frequency_ghz" => settings.max_frequency_ghz = number(value, line, key)?,
                "synth_tolerance_ohm" => settings.synth_tolerance_ohm = number(value, line, key)?,
                "synth_max_iterations" => {
                    let steps = integer(value, line, key)?;
                    settings.synth_max_iterations =
                        u32::try_from(steps).map_err(|_| StackupError::InvalidSettings {
                            line,
                            reason: format!("{key}: too large"),
                        })?;
                }
                "synth_min_width_nm" => settings.synth_min_width_nm = integer(value, line, key)?,
                "synth_max_width_nm" => settings.synth_max_width_nm = integer(value, line, key)?,
                _ => {
                    return Err(StackupError::InvalidSettings {
                        line,
                        reason: format!("unknown key {key}"),
                    })
                }
            }
        }
        if settings.synth_min_width_nm >= settings.synth_max_width_nm {
            return Err(StackupError::InvalidSettings {
                line: 0,
                reason: "synth_min_width_nm must be below synth_max_width_nm".to_string(),
            });
        }
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_text_keeps_the_defaults() {
        assert_eq!(
            StackupSettings::parse("").unwrap(),
            StackupSettings::default()
        );
        assert_eq!(
            StackupSettings::parse("# only a comment\n\n").unwrap(),
            StackupSettings::default()
        );
    }

    #[test]
    fn the_shipped_settings_file_parses_to_the_defaults() {
        let text = include_str!("../../../config/stackup.cfg");
        assert_eq!(
            StackupSettings::parse(text).unwrap(),
            StackupSettings::default()
        );
    }

    #[test]
    fn keys_change_the_values() {
        let text = "copper_conductivity_s_per_m = 5.0e7\nsynth_tolerance_ohm = 0.5\n\
                    synth_max_iterations = 7\nsynth_min_width_nm = 5000\nsynth_max_width_nm = 9000";
        let s = StackupSettings::parse(text).unwrap();
        assert_eq!(s.copper_conductivity_s_per_m, 5.0e7);
        assert_eq!(s.synth_tolerance_ohm, 0.5);
        assert_eq!(s.synth_max_iterations, 7);
        assert_eq!((s.synth_min_width_nm, s.synth_max_width_nm), (5000, 9000));
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, line) in [
            ("nonsense", 1),
            ("\nmax_frequency_ghz = fast", 2),
            ("max_frequency_ghz = -1", 1),
            ("max_frequency_ghz = inf", 1),
            ("synth_max_iterations = 2.5", 1),
            ("synth_min_width_nm = 0", 1),
            ("unknown_key = 1", 1),
            ("synth_max_iterations = 99999999999", 1),
        ] {
            match StackupSettings::parse(text) {
                Err(StackupError::InvalidSettings { line: got, .. }) => {
                    assert_eq!(got, line, "{text}")
                }
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn the_width_bounds_must_be_in_order() {
        let text = "synth_min_width_nm = 9000\nsynth_max_width_nm = 9000";
        assert!(matches!(
            StackupSettings::parse(text),
            Err(StackupError::InvalidSettings { .. })
        ));
    }
}
