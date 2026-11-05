// SPDX-License-Identifier: GPL-3.0-or-later
//! A lossless transmission line described by its impedance and its effective permittivity: the
//! link between the board calculator (`eh3d-phoenix-stackup`) and the RF tools. A line taken from
//! a stack-up gives the electrical length of a track, and the physical length of a stub or a
//! quarter-wave section found by the matching tools.

use crate::error::{positive, RfError};
use crate::network::{abcd_lossless_line, Mat2};
use eh3d_phoenix_stackup::LineResult;

/// The speed of light in vacuum, in m/s.
pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;

/// A line with a characteristic impedance and an effective relative permittivity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// The characteristic impedance in ohm (for a coupled pair, the differential impedance).
    pub z0_ohm: f64,
    /// The effective relative permittivity.
    pub eps_eff: f64,
}

impl Line {
    /// A line from its impedance and its effective permittivity (at least 1).
    pub fn new(z0_ohm: f64, eps_eff: f64) -> Result<Line, RfError> {
        positive("characteristic impedance", z0_ohm)?;
        positive("effective permittivity", eps_eff)?;
        if eps_eff < 1.0 {
            return Err(RfError::InvalidNumber {
                field: "effective permittivity".to_string(),
                reason: "must be at least 1".to_string(),
            });
        }
        Ok(Line { z0_ohm, eps_eff })
    }

    /// The line of a result of the stack-up calculator: its impedance and effective permittivity
    /// at the frequency it was computed for.
    pub fn from_stackup(result: &LineResult) -> Result<Line, RfError> {
        Line::new(result.impedance_ohm, result.eps_eff)
    }

    /// The guided wavelength in metres at a frequency in Hz.
    pub fn wavelength_m(&self, freq_hz: f64) -> Result<f64, RfError> {
        let f = positive("frequency", freq_hz)?;
        Ok(SPEED_OF_LIGHT_M_PER_S / (f * self.eps_eff.sqrt()))
    }

    /// The electrical length in degrees of a physical length in metres.
    pub fn electrical_degrees(&self, length_m: f64, freq_hz: f64) -> Result<f64, RfError> {
        if !length_m.is_finite() || length_m < 0.0 {
            return Err(RfError::InvalidNumber {
                field: "length".to_string(),
                reason: "must be a finite number, zero or more".to_string(),
            });
        }
        Ok(360.0 * length_m / self.wavelength_m(freq_hz)?)
    }

    /// The physical length in metres of an electrical length in degrees.
    pub fn length_m(&self, degrees: f64, freq_hz: f64) -> Result<f64, RfError> {
        if !degrees.is_finite() || degrees < 0.0 {
            return Err(RfError::InvalidNumber {
                field: "electrical length".to_string(),
                reason: "must be a finite number, zero or more".to_string(),
            });
        }
        Ok(degrees / 360.0 * self.wavelength_m(freq_hz)?)
    }

    /// The ABCD matrix of a length of this line in metres at a frequency in Hz.
    pub fn abcd(&self, length_m: f64, freq_hz: f64) -> Result<Mat2, RfError> {
        let degrees = self.electrical_degrees(length_m, freq_hz)?;
        abcd_lossless_line(self.z0_ohm, degrees.to_radians())
    }

    /// The delay in seconds of a length of this line in metres.
    pub fn delay_s(&self, length_m: f64) -> Result<f64, RfError> {
        if !length_m.is_finite() || length_m < 0.0 {
            return Err(RfError::InvalidNumber {
                field: "length".to_string(),
                reason: "must be a finite number, zero or more".to_string(),
            });
        }
        Ok(length_m * self.eps_eff.sqrt() / SPEED_OF_LIGHT_M_PER_S)
    }
}

/// The electrical length in degrees of a delay in seconds at a frequency in Hz.
pub fn degrees_of_delay(delay_s: f64, freq_hz: f64) -> Result<f64, RfError> {
    if !delay_s.is_finite() || delay_s < 0.0 {
        return Err(RfError::InvalidNumber {
            field: "delay".to_string(),
            reason: "must be a finite number, zero or more".to_string(),
        });
    }
    Ok(360.0 * freq_hz * delay_s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complex::Complex;
    use crate::network::input_impedance;

    #[test]
    fn a_quarter_wave_in_air_at_one_gigahertz() {
        let line = Line::new(50.0, 1.0).unwrap();
        let lambda = line.wavelength_m(1e9).unwrap();
        assert!((lambda - 0.299792458).abs() < 1e-12);
        assert!((line.length_m(90.0, 1e9).unwrap() - lambda / 4.0).abs() < 1e-15);
        assert!((line.electrical_degrees(lambda / 4.0, 1e9).unwrap() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn the_permittivity_shortens_the_wavelength() {
        let slow = Line::new(50.0, 4.0).unwrap();
        let fast = Line::new(50.0, 1.0).unwrap();
        let ratio = fast.wavelength_m(2e9).unwrap() / slow.wavelength_m(2e9).unwrap();
        assert!((ratio - 2.0).abs() < 1e-12);
        let delay = slow.delay_s(0.1).unwrap();
        assert!(
            (degrees_of_delay(delay, 1e9).unwrap() - slow.electrical_degrees(0.1, 1e9).unwrap())
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn the_matrix_of_a_quarter_wave_inverts_the_load() {
        let line = Line::new(100.0, 2.5).unwrap();
        let length = line.length_m(90.0, 3e9).unwrap();
        let t = line.abcd(length, 3e9).unwrap();
        let zin = input_impedance(&t, Complex::real(400.0)).unwrap();
        assert!((zin.re - 25.0).abs() < 1e-9 && zin.im.abs() < 1e-9);
    }

    #[test]
    fn a_line_is_taken_from_a_stackup_result() {
        use eh3d_phoenix_stackup::{
            analyze, CopperThickness, Layer, LineKind, LineSpec, MaterialSample, Stackup,
            StackupSettings,
        };
        let stackup = Stackup {
            name: "two-layer".to_string(),
            layers: vec![
                Layer::Copper {
                    name: "top".to_string(),
                    thickness: CopperThickness::Um(35.0),
                    roughness_rms_nm: 0,
                },
                Layer::Core {
                    name: "core".to_string(),
                    thickness_nm: 1_600_000,
                    material: "fr4-test".to_string(),
                },
                Layer::Copper {
                    name: "bottom".to_string(),
                    thickness: CopperThickness::Um(35.0),
                    roughness_rms_nm: 0,
                },
            ],
        };
        let material = MaterialSample {
            name: "fr4-test".to_string(),
            dk_points: vec![(0.1, 4.4), (50.0, 4.4)],
            df_points: vec![(0.1, 0.02), (50.0, 0.02)],
            thickness_nm: None,
        };
        let spec = LineSpec {
            name: "line".to_string(),
            kind: LineKind::Microstrip,
            layer: "top".to_string(),
            width_nm: Some(3_000_000),
            target_ohm: None,
            gap_nm: None,
        };
        let result = analyze(
            &stackup,
            &[material],
            &spec,
            3_000_000,
            1.0,
            &StackupSettings::default(),
        )
        .unwrap();
        let line = Line::from_stackup(&result).unwrap();
        assert!(line.z0_ohm > 10.0 && line.z0_ohm < 200.0);
        assert!(line.eps_eff > 1.0 && line.eps_eff < 4.4);
    }

    #[test]
    fn bad_values_are_refused() {
        assert!(Line::new(0.0, 1.0).is_err());
        assert!(Line::new(50.0, 0.5).is_err());
        assert!(Line::new(50.0, f64::NAN).is_err());
        let line = Line::new(50.0, 1.0).unwrap();
        assert!(line.wavelength_m(0.0).is_err());
        assert!(line.electrical_degrees(-1.0, 1e9).is_err());
        assert!(line.length_m(f64::NAN, 1e9).is_err());
        assert!(line.delay_s(-1.0).is_err());
        assert!(degrees_of_delay(-1.0, 1e9).is_err());
    }
}
