// SPDX-License-Identifier: GPL-3.0-or-later
//! The dielectric constant and the loss tangent of a material at a frequency.
//!
//! A [`MaterialSample`] holds the points a data sheet publishes, as pairs of a frequency in GHz
//! and a value. The value at a frequency is interpolated linearly in the logarithm of the
//! frequency between the two published points that enclose it, and it is never extrapolated: a
//! frequency below the lowest or above the highest published point is an error (the same rule as
//! the materials database of the suite, so the two agree).

use serde::{Deserialize, Serialize};

use crate::error::StackupError;

/// The most points of one list that are accepted.
pub const MAX_POINTS: usize = 256;
/// The lowest dielectric constant that is accepted (vacuum).
pub const DK_MIN: f64 = 1.0;
/// The highest dielectric constant that is accepted.
pub const DK_MAX: f64 = 100.0;
/// The highest loss tangent that is accepted.
pub const DF_MAX: f64 = 1.0;
/// The longest name that is accepted, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// The published values of one dielectric material.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialSample {
    /// The name layers refer to.
    pub name: String,
    /// Pairs of a frequency in GHz and a dielectric constant, by increasing frequency.
    pub dk_points: Vec<(f64, f64)>,
    /// Pairs of a frequency in GHz and a loss tangent, by increasing frequency.
    pub df_points: Vec<(f64, f64)>,
    /// The nominal thickness of the material in nanometres, when the sheet gives one. It is for
    /// information; the thickness of a layer is the one written on the layer.
    #[serde(default)]
    pub thickness_nm: Option<i64>,
}

fn check_points(
    name: &str,
    points: &[(f64, f64)],
    what: &str,
    low: f64,
    high: f64,
) -> Result<(), StackupError> {
    let bad = |reason: String| StackupError::InvalidMaterial {
        name: name.to_string(),
        reason,
    };
    if points.is_empty() {
        return Err(bad(format!("{what} has no points")));
    }
    if points.len() > MAX_POINTS {
        return Err(bad(format!("{what} has more than {MAX_POINTS} points")));
    }
    let mut last_freq = 0.0_f64;
    for (index, (freq, value)) in points.iter().enumerate() {
        if !freq.is_finite() || *freq <= 0.0 {
            return Err(bad(format!(
                "{what} point {index}: the frequency is not above zero"
            )));
        }
        if !value.is_finite() || !(low..=high).contains(value) {
            return Err(bad(format!(
                "{what} point {index}: the value {value} is outside {low} to {high}"
            )));
        }
        if index > 0 && *freq <= last_freq {
            return Err(bad(format!(
                "{what} point {index}: the frequencies must increase"
            )));
        }
        last_freq = *freq;
    }
    Ok(())
}

fn interpolate(
    name: &str,
    what: &str,
    points: &[(f64, f64)],
    freq_ghz: f64,
) -> Result<f64, StackupError> {
    if !freq_ghz.is_finite() || freq_ghz <= 0.0 {
        return Err(StackupError::InvalidNumber {
            field: "frequency".to_string(),
            reason: format!("{freq_ghz} GHz is not a frequency"),
        });
    }
    let (first, last) = match (points.first(), points.last()) {
        (Some(a), Some(b)) => (*a, *b),
        _ => {
            return Err(StackupError::InvalidMaterial {
                name: name.to_string(),
                reason: format!("{what} has no points"),
            })
        }
    };
    if !(first.0..=last.0).contains(&freq_ghz) {
        return Err(StackupError::OutsidePublished {
            what: format!("{name} {what}"),
            freq_ghz,
            low_ghz: first.0,
            high_ghz: last.0,
        });
    }
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (a.0..=b.0).contains(&freq_ghz) {
            if freq_ghz == a.0 {
                return Ok(a.1);
            }
            let t = (freq_ghz.ln() - a.0.ln()) / (b.0.ln() - a.0.ln());
            return Ok(a.1 + t * (b.1 - a.1));
        }
    }
    // one single point: the frequency range check above made the frequency equal to it
    Ok(first.1)
}

impl MaterialSample {
    /// Check the name and the points. Returns the first problem found.
    pub fn validate(&self) -> Result<(), StackupError> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_NAME_CHARS {
            return Err(StackupError::InvalidMaterial {
                name: self.name.chars().take(MAX_NAME_CHARS).collect(),
                reason: format!("the name must have 1 to {MAX_NAME_CHARS} characters"),
            });
        }
        check_points(&self.name, &self.dk_points, "Dk", DK_MIN, DK_MAX)?;
        check_points(&self.name, &self.df_points, "Df", 0.0, DF_MAX)?;
        if let Some(thickness) = self.thickness_nm {
            if thickness <= 0 {
                return Err(StackupError::InvalidMaterial {
                    name: self.name.clone(),
                    reason: "the thickness must be above zero".to_string(),
                });
            }
        }
        Ok(())
    }

    /// The dielectric constant at a frequency in GHz, between the published points.
    pub fn dk_at(&self, freq_ghz: f64) -> Result<f64, StackupError> {
        interpolate(&self.name, "Dk", &self.dk_points, freq_ghz)
    }

    /// The loss tangent at a frequency in GHz, between the published points.
    pub fn df_at(&self, freq_ghz: f64) -> Result<f64, StackupError> {
        interpolate(&self.name, "Df", &self.df_points, freq_ghz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invented values for the tests; they are not the data of any sheet.
    fn sample() -> MaterialSample {
        MaterialSample {
            name: "test-laminate".to_string(),
            dk_points: vec![(1.0, 4.0), (10.0, 3.0)],
            df_points: vec![(1.0, 0.02), (10.0, 0.01)],
            thickness_nm: None,
        }
    }

    #[test]
    fn a_published_frequency_gives_the_published_value() {
        let m = sample();
        assert_eq!(m.dk_at(1.0).unwrap(), 4.0);
        assert_eq!(m.dk_at(10.0).unwrap(), 3.0);
        assert_eq!(m.df_at(10.0).unwrap(), 0.01);
    }

    #[test]
    fn the_interpolation_is_linear_in_the_logarithm_of_the_frequency() {
        let m = sample();
        // sqrt(10) GHz is the middle of the decade
        let mid = 10.0_f64.sqrt();
        assert!((m.dk_at(mid).unwrap() - 3.5).abs() < 1e-12);
        assert!((m.df_at(mid).unwrap() - 0.015).abs() < 1e-12);
        // monotone between the points
        let (a, b, c) = (
            m.dk_at(2.0).unwrap(),
            m.dk_at(4.0).unwrap(),
            m.dk_at(8.0).unwrap(),
        );
        assert!(a > b && b > c);
    }

    #[test]
    fn it_never_extrapolates() {
        let m = sample();
        for freq in [0.5, 0.999, 10.001, 100.0] {
            match m.dk_at(freq) {
                Err(StackupError::OutsidePublished {
                    low_ghz, high_ghz, ..
                }) => assert_eq!((low_ghz, high_ghz), (1.0, 10.0)),
                other => panic!("{freq}: {other:?}"),
            }
        }
    }

    #[test]
    fn one_point_serves_only_its_own_frequency() {
        let m = MaterialSample {
            name: "single".to_string(),
            dk_points: vec![(2.0, 3.5)],
            df_points: vec![(2.0, 0.004)],
            thickness_nm: Some(508_000),
        };
        assert!(m.validate().is_ok());
        assert_eq!(m.dk_at(2.0).unwrap(), 3.5);
        assert!(m.dk_at(2.1).is_err());
        assert!(m.df_at(1.9).is_err());
    }

    #[test]
    fn a_frequency_that_is_not_a_number_is_refused() {
        let m = sample();
        for freq in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                m.dk_at(freq),
                Err(StackupError::InvalidNumber { .. })
            ));
        }
    }

    #[test]
    fn bad_materials_are_refused() {
        let mut empty_name = sample();
        empty_name.name = "  ".to_string();
        let mut no_points = sample();
        no_points.dk_points.clear();
        let mut unsorted = sample();
        unsorted.dk_points = vec![(10.0, 3.0), (1.0, 4.0)];
        let mut repeated = sample();
        repeated.df_points = vec![(1.0, 0.02), (1.0, 0.01)];
        let mut dk_low = sample();
        dk_low.dk_points = vec![(1.0, 0.5)];
        let mut dk_nan = sample();
        dk_nan.dk_points = vec![(1.0, f64::NAN)];
        let mut df_high = sample();
        df_high.df_points = vec![(1.0, 2.0)];
        let mut zero_freq = sample();
        zero_freq.dk_points = vec![(0.0, 3.0)];
        let mut thickness = sample();
        thickness.thickness_nm = Some(0);
        let mut many = sample();
        many.dk_points = (1..=MAX_POINTS + 1).map(|i| (i as f64, 3.0)).collect();
        for m in [
            empty_name, no_points, unsorted, repeated, dk_low, dk_nan, df_high, zero_freq,
            thickness, many,
        ] {
            assert!(
                matches!(m.validate(), Err(StackupError::InvalidMaterial { .. })),
                "{m:?}"
            );
        }
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn json_round_trip_and_unknown_fields() {
        let m = sample();
        let text = serde_json::to_string(&m).unwrap();
        let back: MaterialSample = serde_json::from_str(&text).unwrap();
        assert_eq!(back, m);
        let extra = r#"{"name":"x","dk_points":[[1,3]],"df_points":[[1,0.01]],"colour":"red"}"#;
        assert!(serde_json::from_str::<MaterialSample>(extra).is_err());
        let no_thickness = r#"{"name":"x","dk_points":[[1,3]],"df_points":[[1,0.01]]}"#;
        let parsed: MaterialSample = serde_json::from_str(no_thickness).unwrap();
        assert_eq!(parsed.thickness_nm, None);
    }
}
