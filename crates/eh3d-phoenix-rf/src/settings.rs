// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the RF tools. Every limit and every tolerance is read from a settings text,
//! `config/rf.cfg`, and not buried in the calculations.

use crate::error::RfError;

/// The limits and tolerances of the RF tools.
#[derive(Clone, Debug, PartialEq)]
pub struct RfSettings {
    /// The most ports a Touchstone file may have.
    pub max_ports: usize,
    /// The most frequency points (and noise points) a Touchstone file may have.
    pub max_points: usize,
    /// The largest Touchstone file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The highest frequency accepted, in GHz.
    pub max_frequency_ghz: f64,
    /// The highest filter order accepted.
    pub max_filter_order: usize,
    /// The smallest ripple of a Chebyshev filter, in dB.
    pub min_ripple_db: f64,
    /// The largest ripple of a Chebyshev filter, in dB.
    pub max_ripple_db: f64,
    /// The smallest magnitude of an impedance given as input, in ohm.
    pub min_impedance_ohm: f64,
    /// The largest magnitude of an impedance given as input, in ohm.
    pub max_impedance_ohm: f64,
    /// The most points of a frequency sweep.
    pub max_sweep_points: usize,
    /// The step of the numerical group delay of a filter, as a fraction of the frequency.
    pub delay_step_fraction: f64,
    /// The relative tolerance below which a reactance, a susceptance or a coefficient is zero.
    pub tolerance: f64,
    /// A network whose largest singular value exceeds 1 by more than this is not passive.
    pub passivity_tolerance: f64,
    /// The reference impedance used when none is given, in ohm.
    pub default_z0_ohm: f64,
    /// The most elements of a network description.
    pub max_net_elements: usize,
    /// The largest network description or mask text that is read, in bytes.
    pub max_net_bytes: usize,
    /// The most segments of a mask.
    pub max_mask_segments: usize,
    /// The points that sample each element of a path on the Smith chart.
    pub smith_arc_points: usize,
    /// The loaded quality factor used for a pi or T network when none is given.
    pub default_loaded_q: f64,
    /// The largest loaded quality factor accepted for a pi or T network.
    pub max_loaded_q: f64,
    /// The frequencies of the scans that look for band edges and bandwidths.
    pub scan_points: usize,
    /// The return loss in dB that bounds the bandwidth of a matching network.
    pub return_loss_bandwidth_db: f64,
    /// The series of standard values used when none is given: 12, 24 or 96.
    pub default_series: usize,
    /// The spacing of the two stubs of a double-stub match when none is given, in degrees.
    pub default_stub_spacing_deg: f64,
    /// The margin in dB below which a point of a mask still counts as on the limit.
    pub mask_margin_tolerance_db: f64,
}

impl Default for RfSettings {
    fn default() -> Self {
        RfSettings {
            max_ports: 16,
            max_points: 200_000,
            max_file_bytes: 64 * 1024 * 1024,
            max_frequency_ghz: 1000.0,
            max_filter_order: 12,
            min_ripple_db: 0.001,
            max_ripple_db: 6.0,
            min_impedance_ohm: 0.01,
            max_impedance_ohm: 1_000_000.0,
            max_sweep_points: 10_000,
            delay_step_fraction: 1e-4,
            tolerance: 1e-9,
            passivity_tolerance: 1e-6,
            default_z0_ohm: 50.0,
            max_net_elements: 200,
            max_net_bytes: 65_536,
            max_mask_segments: 200,
            smith_arc_points: 24,
            default_loaded_q: 5.0,
            max_loaded_q: 100.0,
            scan_points: 400,
            return_loss_bandwidth_db: 10.0,
            default_series: 24,
            default_stub_spacing_deg: 45.0,
            mask_margin_tolerance_db: 1e-9,
        }
    }
}

fn number(value: &str, line: usize, key: &str) -> Result<f64, RfError> {
    let parsed: f64 = value.parse().map_err(|_| RfError::InvalidSettings {
        line,
        reason: format!("{key}: not a number"),
    })?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(RfError::InvalidSettings {
            line,
            reason: format!("{key}: must be a finite number above zero"),
        });
    }
    Ok(parsed)
}

fn whole(value: &str, line: usize, key: &str) -> Result<u64, RfError> {
    let parsed: u64 = value.parse().map_err(|_| RfError::InvalidSettings {
        line,
        reason: format!("{key}: not a whole number"),
    })?;
    if parsed == 0 {
        return Err(RfError::InvalidSettings {
            line,
            reason: format!("{key}: must be above zero"),
        });
    }
    Ok(parsed)
}

fn count(value: &str, line: usize, key: &str) -> Result<usize, RfError> {
    usize::try_from(whole(value, line, key)?).map_err(|_| RfError::InvalidSettings {
        line,
        reason: format!("{key}: too large"),
    })
}

impl RfSettings {
    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error.
    pub fn parse(text: &str) -> Result<RfSettings, RfError> {
        let mut s = RfSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| RfError::InvalidSettings {
                    line,
                    reason: "expected key = value".to_string(),
                })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "max_ports" => s.max_ports = count(value, line, key)?,
                "max_points" => s.max_points = count(value, line, key)?,
                "max_file_bytes" => s.max_file_bytes = whole(value, line, key)?,
                "max_frequency_ghz" => s.max_frequency_ghz = number(value, line, key)?,
                "max_filter_order" => s.max_filter_order = count(value, line, key)?,
                "min_ripple_db" => s.min_ripple_db = number(value, line, key)?,
                "max_ripple_db" => s.max_ripple_db = number(value, line, key)?,
                "min_impedance_ohm" => s.min_impedance_ohm = number(value, line, key)?,
                "max_impedance_ohm" => s.max_impedance_ohm = number(value, line, key)?,
                "max_sweep_points" => s.max_sweep_points = count(value, line, key)?,
                "delay_step_fraction" => s.delay_step_fraction = number(value, line, key)?,
                "tolerance" => s.tolerance = number(value, line, key)?,
                "passivity_tolerance" => s.passivity_tolerance = number(value, line, key)?,
                "default_z0_ohm" => s.default_z0_ohm = number(value, line, key)?,
                "max_net_elements" => s.max_net_elements = count(value, line, key)?,
                "max_net_bytes" => s.max_net_bytes = count(value, line, key)?,
                "max_mask_segments" => s.max_mask_segments = count(value, line, key)?,
                "smith_arc_points" => s.smith_arc_points = count(value, line, key)?,
                "default_loaded_q" => s.default_loaded_q = number(value, line, key)?,
                "max_loaded_q" => s.max_loaded_q = number(value, line, key)?,
                "scan_points" => s.scan_points = count(value, line, key)?,
                "return_loss_bandwidth_db" => {
                    s.return_loss_bandwidth_db = number(value, line, key)?
                }
                "default_series" => s.default_series = count(value, line, key)?,
                "default_stub_spacing_deg" => {
                    s.default_stub_spacing_deg = number(value, line, key)?
                }
                "mask_margin_tolerance_db" => {
                    s.mask_margin_tolerance_db = number(value, line, key)?
                }
                _ => {
                    return Err(RfError::InvalidSettings {
                        line,
                        reason: format!("unknown key {key}"),
                    })
                }
            }
        }
        s.check_order()?;
        Ok(s)
    }

    fn check_order(&self) -> Result<(), RfError> {
        let pairs = [
            (
                self.min_ripple_db,
                self.max_ripple_db,
                "min_ripple_db must be below max_ripple_db",
            ),
            (
                self.min_impedance_ohm,
                self.max_impedance_ohm,
                "min_impedance_ohm must be below max_impedance_ohm",
            ),
        ];
        for (low, high, reason) in pairs {
            if low >= high {
                return Err(RfError::InvalidSettings {
                    line: 0,
                    reason: reason.to_string(),
                });
            }
        }
        if self.delay_step_fraction >= 0.5 {
            return Err(RfError::InvalidSettings {
                line: 0,
                reason: "delay_step_fraction must be below 0.5".to_string(),
            });
        }
        if ![12, 24, 96].contains(&self.default_series) {
            return Err(RfError::InvalidSettings {
                line: 0,
                reason: "default_series must be 12, 24 or 96".to_string(),
            });
        }
        if self.scan_points < 20 || self.smith_arc_points < 2 {
            return Err(RfError::InvalidSettings {
                line: 0,
                reason: "scan_points must be at least 20 and smith_arc_points at least 2"
                    .to_string(),
            });
        }
        if self.default_loaded_q > self.max_loaded_q {
            return Err(RfError::InvalidSettings {
                line: 0,
                reason: "default_loaded_q must not be above max_loaded_q".to_string(),
            });
        }
        if self.default_stub_spacing_deg >= 180.0 {
            return Err(RfError::InvalidSettings {
                line: 0,
                reason: "default_stub_spacing_deg must be below 180".to_string(),
            });
        }
        Ok(())
    }

    /// The largest frequency accepted, in Hz.
    pub fn max_frequency_hz(&self) -> f64 {
        self.max_frequency_ghz * 1e9
    }

    /// Check a frequency in Hz: finite, above zero and not above the limit.
    pub fn check_frequency_hz(&self, field: &str, freq_hz: f64) -> Result<f64, RfError> {
        crate::error::positive(field, freq_hz)?;
        if freq_hz > self.max_frequency_hz() {
            return Err(RfError::OutsideLimit {
                field: field.to_string(),
                value: freq_hz,
                limit: self.max_frequency_hz(),
            });
        }
        Ok(freq_hz)
    }

    /// Check the magnitude of an impedance given as input against the limits.
    pub fn check_impedance_ohm(
        &self,
        field: &str,
        z: crate::complex::Complex,
    ) -> Result<(), RfError> {
        if !z.is_finite() {
            return Err(RfError::InvalidNumber {
                field: field.to_string(),
                reason: "must be finite".to_string(),
            });
        }
        let magnitude = z.abs();
        if magnitude < self.min_impedance_ohm {
            return Err(RfError::OutsideLimit {
                field: field.to_string(),
                value: magnitude,
                limit: self.min_impedance_ohm,
            });
        }
        if magnitude > self.max_impedance_ohm {
            return Err(RfError::OutsideLimit {
                field: field.to_string(),
                value: magnitude,
                limit: self.max_impedance_ohm,
            });
        }
        Ok(())
    }

    /// Check a real reference or characteristic impedance: finite, above zero and inside the
    /// limits.
    pub fn check_reference_ohm(&self, field: &str, z0_ohm: f64) -> Result<f64, RfError> {
        crate::error::positive(field, z0_ohm)?;
        self.check_impedance_ohm(field, crate::complex::Complex::real(z0_ohm))?;
        Ok(z0_ohm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complex::Complex;

    #[test]
    fn an_empty_text_keeps_the_defaults() {
        assert_eq!(RfSettings::parse("").unwrap(), RfSettings::default());
        assert_eq!(
            RfSettings::parse("# a comment\n\n").unwrap(),
            RfSettings::default()
        );
    }

    #[test]
    fn the_shipped_settings_file_parses_to_the_defaults() {
        let text = include_str!("../../../config/rf.cfg");
        assert_eq!(RfSettings::parse(text).unwrap(), RfSettings::default());
    }

    #[test]
    fn keys_change_the_values() {
        let text = "max_ports = 4\nmax_points = 10\nmax_file_bytes = 1000\n\
                    max_frequency_ghz = 40\nmax_filter_order = 5\nmax_sweep_points = 7\n\
                    tolerance = 1e-6\ndefault_z0_ohm = 75";
        let s = RfSettings::parse(text).unwrap();
        assert_eq!((s.max_ports, s.max_points, s.max_file_bytes), (4, 10, 1000));
        assert_eq!((s.max_frequency_ghz, s.max_filter_order), (40.0, 5));
        assert_eq!(
            (s.max_sweep_points, s.tolerance, s.default_z0_ohm),
            (7, 1e-6, 75.0)
        );
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, line) in [
            ("nonsense", 1),
            ("\nmax_ports = many", 2),
            ("max_ports = 0", 1),
            ("max_ports = -1", 1),
            ("max_ports = 2.5", 1),
            ("max_frequency_ghz = -1", 1),
            ("max_frequency_ghz = inf", 1),
            ("max_points = 99999999999999999999999", 1),
            ("unknown_key = 1", 1),
        ] {
            match RfSettings::parse(text) {
                Err(RfError::InvalidSettings { line: got, .. }) => assert_eq!(got, line, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn the_bounds_must_be_in_order() {
        for text in [
            "min_ripple_db = 3\nmax_ripple_db = 3",
            "min_impedance_ohm = 5\nmax_impedance_ohm = 4",
            "delay_step_fraction = 0.5",
            "default_series = 48",
            "scan_points = 10",
            "smith_arc_points = 1",
            "default_loaded_q = 500",
            "default_stub_spacing_deg = 180",
        ] {
            assert!(
                matches!(
                    RfSettings::parse(text),
                    Err(RfError::InvalidSettings { .. })
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn the_checks_use_the_limits() {
        let s = RfSettings::default();
        assert!(s.check_frequency_hz("f", 1e9).is_ok());
        assert!(matches!(
            s.check_frequency_hz("f", 1e13),
            Err(RfError::OutsideLimit { .. })
        ));
        assert!(matches!(
            s.check_frequency_hz("f", 0.0),
            Err(RfError::InvalidNumber { .. })
        ));
        assert!(s.check_impedance_ohm("z", Complex::new(50.0, 20.0)).is_ok());
        for z in [
            Complex::real(0.0),
            Complex::real(1e9),
            Complex::new(f64::NAN, 0.0),
        ] {
            assert!(s.check_impedance_ohm("z", z).is_err());
        }
        assert_eq!(s.check_reference_ohm("z0", 50.0).unwrap(), 50.0);
        assert!(s.check_reference_ohm("z0", -50.0).is_err());
    }
}
