// SPDX-License-Identifier: GPL-3.0-or-later
//! The series of standard values E12, E24 and E96 of IEC 60063, and the nearest value of a
//! series to a number. The values of E12 and E24 are the published ones; the 96 values of E96
//! are the published ones too (each is `10^(k/96)` rounded to three digits, and a test checks
//! that).

use crate::error::RfError;

/// The twelve values of the series E12 for one decade.
pub const E12: [f64; 12] = [1.0, 1.2, 1.5, 1.8, 2.2, 2.7, 3.3, 3.9, 4.7, 5.6, 6.8, 8.2];

/// The twenty-four values of the series E24 for one decade.
pub const E24: [f64; 24] = [
    1.0, 1.1, 1.2, 1.3, 1.5, 1.6, 1.8, 2.0, 2.2, 2.4, 2.7, 3.0, 3.3, 3.6, 3.9, 4.3, 4.7, 5.1, 5.6,
    6.2, 6.8, 7.5, 8.2, 9.1,
];

/// The ninety-six values of the series E96 for one decade.
pub const E96: [f64; 96] = [
    1.00, 1.02, 1.05, 1.07, 1.10, 1.13, 1.15, 1.18, 1.21, 1.24, 1.27, 1.30, 1.33, 1.37, 1.40, 1.43,
    1.47, 1.50, 1.54, 1.58, 1.62, 1.65, 1.69, 1.74, 1.78, 1.82, 1.87, 1.91, 1.96, 2.00, 2.05, 2.10,
    2.15, 2.21, 2.26, 2.32, 2.37, 2.43, 2.49, 2.55, 2.61, 2.67, 2.74, 2.80, 2.87, 2.94, 3.01, 3.09,
    3.16, 3.24, 3.32, 3.40, 3.48, 3.57, 3.65, 3.74, 3.83, 3.92, 4.02, 4.12, 4.22, 4.32, 4.42, 4.53,
    4.64, 4.75, 4.87, 4.99, 5.11, 5.23, 5.36, 5.49, 5.62, 5.76, 5.90, 6.04, 6.19, 6.34, 6.49, 6.65,
    6.81, 6.98, 7.15, 7.32, 7.50, 7.68, 7.87, 8.06, 8.25, 8.45, 8.66, 8.87, 9.09, 9.31, 9.53, 9.76,
];

/// A series of standard values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Series {
    /// Twelve values per decade.
    E12,
    /// Twenty-four values per decade.
    E24,
    /// Ninety-six values per decade.
    E96,
}

impl Series {
    /// The series from its number: 12, 24 or 96.
    pub fn from_number(number: usize) -> Result<Series, RfError> {
        match number {
            12 => Ok(Series::E12),
            24 => Ok(Series::E24),
            96 => Ok(Series::E96),
            other => Err(RfError::UnknownSeries {
                name: format!("E{other}"),
            }),
        }
    }

    /// The series from a name such as `e24` or `E96` (the case is ignored).
    pub fn from_name(name: &str) -> Result<Series, RfError> {
        let unknown = || RfError::UnknownSeries {
            name: name.chars().take(20).collect(),
        };
        let digits = name.trim().strip_prefix(['e', 'E']).ok_or_else(unknown)?;
        let number: usize = digits.parse().map_err(|_| unknown())?;
        Series::from_number(number)
    }

    /// The number of values per decade.
    pub fn number(self) -> usize {
        match self {
            Series::E12 => 12,
            Series::E24 => 24,
            Series::E96 => 96,
        }
    }

    /// The name, `E12`, `E24` or `E96`.
    pub fn name(self) -> &'static str {
        match self {
            Series::E12 => "E12",
            Series::E24 => "E24",
            Series::E96 => "E96",
        }
    }

    /// The values of one decade, from 1 up to but not including 10.
    pub fn values(self) -> &'static [f64] {
        match self {
            Series::E12 => &E12,
            Series::E24 => &E24,
            Series::E96 => &E96,
        }
    }

    /// The value of the series nearest to `value` in ratio (the nearest on a logarithmic scale),
    /// in any decade. The value must be finite and above zero, and between 1e-18 and 1e18.
    pub fn nearest(self, value: f64) -> Result<f64, RfError> {
        if !value.is_finite() || !(1e-18..=1e18).contains(&value) {
            return Err(RfError::InvalidNumber {
                field: "component value".to_string(),
                reason: "must be a finite number between 1e-18 and 1e18".to_string(),
            });
        }
        let decade = value.log10().floor();
        let scale = 10f64.powi(decade as i32);
        let mantissa = value / scale;
        let values = self.values();
        // the candidates are the values of the decade, the first of the next decade and the last
        // of the one before
        let mut best = values[0];
        let mut best_distance = f64::INFINITY;
        let before = values[values.len() - 1] / 10.0;
        for candidate in std::iter::once(before)
            .chain(values.iter().copied())
            .chain(std::iter::once(10.0))
        {
            let distance = (mantissa / candidate).ln().abs();
            if distance < best_distance {
                best_distance = distance;
                best = candidate;
            }
        }
        Ok(best * scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_series_have_their_published_sizes_and_increase() {
        for series in [Series::E12, Series::E24, Series::E96] {
            let values = series.values();
            assert_eq!(values.len(), series.number());
            assert!(values.windows(2).all(|w| w[0] < w[1]));
            assert_eq!(values[0], 1.0);
            assert!(values[values.len() - 1] < 10.0);
        }
    }

    #[test]
    fn every_e12_value_is_in_e24() {
        for v in E12 {
            assert!(E24.contains(&v), "{v}");
        }
    }

    #[test]
    fn e96_is_ten_to_the_k_over_96_rounded_to_three_digits() {
        for (k, &v) in E96.iter().enumerate() {
            let exact: f64 = 10f64.powf(k as f64 / 96.0);
            let rounded = (exact * 100.0).round() / 100.0;
            assert!((v - rounded).abs() < 1e-9, "k = {k}: {v} {rounded}");
        }
    }

    #[test]
    fn names_and_numbers_are_read() {
        assert_eq!(Series::from_name("e24").unwrap(), Series::E24);
        assert_eq!(Series::from_name(" E96 ").unwrap(), Series::E96);
        assert_eq!(Series::from_number(12).unwrap().name(), "E12");
        for bad in ["", "24", "E7", "E", "Ex", "E24x"] {
            assert!(
                matches!(Series::from_name(bad), Err(RfError::UnknownSeries { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_nearest_value_is_found_in_every_decade() {
        let near = |series: Series, value: f64| series.nearest(value).unwrap();
        let close = |a: f64, b: f64| ((a - b) / b).abs() < 1e-12;
        assert!(close(near(Series::E24, 3.18e-11), 3.3e-11));
        assert!(close(near(Series::E12, 15.9e-12), 15e-12));
        assert!(close(near(Series::E24, 79.6e-9), 82e-9));
        assert!(close(near(Series::E96, 79.6e-9), 78.7e-9));
        assert!(close(near(Series::E24, 159e-9), 160e-9));
        assert!(close(near(Series::E12, 1.0), 1.0));
    }

    #[test]
    fn the_nearest_value_wraps_between_decades() {
        // 9.8 is nearer to 10 than to 9.1 in ratio, and 0.99 is nearer to 1 than to 0.91
        assert!((Series::E24.nearest(9.8).unwrap() - 10.0).abs() < 1e-9);
        assert!((Series::E24.nearest(0.99).unwrap() - 1.0).abs() < 1e-12);
        assert!((Series::E12.nearest(8.9e3).unwrap() - 8.2e3).abs() < 1e-6);
        assert!((Series::E12.nearest(9.1e3).unwrap() - 10.0e3).abs() < 1e-6);
    }

    #[test]
    fn a_value_of_the_series_is_its_own_nearest_value() {
        for series in [Series::E12, Series::E24, Series::E96] {
            for &v in series.values() {
                for exponent in [-12, -9, -3, 0, 3, 6] {
                    let x = v * 10f64.powi(exponent);
                    let got = series.nearest(x).unwrap();
                    assert!(((got - x) / x).abs() < 1e-12, "{x} {got}");
                }
            }
        }
    }

    #[test]
    fn values_that_are_not_numbers_in_range_are_refused() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-30, 1e30] {
            assert!(
                matches!(Series::E24.nearest(bad), Err(RfError::InvalidNumber { .. })),
                "{bad}"
            );
        }
        assert!(matches!(
            Series::from_number(48),
            Err(RfError::UnknownSeries { .. })
        ));
    }
}
