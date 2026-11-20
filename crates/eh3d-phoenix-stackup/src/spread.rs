// SPDX-License-Identifier: GPL-3.0-or-later
//! The spread of the impedance of a controlled-impedance line: how far the impedance can be from
//! its nominal value when the width of the track, the height of the dielectric and its
//! permittivity are each off by a stated amount.
//!
//! The impedance of a microstrip and of a centred stripline grows with the height of the
//! dielectric and falls with the width of the track and with the permittivity, so the lowest and
//! the highest impedance are at two of the eight corners of the box of the tolerances. The
//! function evaluates the eight corners with the closed forms of [`crate::models`] and keeps the
//! lowest and the highest. A corner that leaves the range of the model (a track of no width, a
//! dielectric of no height, a permittivity below one, a stripline whose copper is as thick as the
//! plane spacing) refuses the whole calculation and names the value, so that a spread is never
//! given for a box that is only partly inside the model.
//!
//! The result is an estimate of the quasi-static line like every other result of this crate; the
//! etch of a maker is not a rectangle and the tolerances of a laminate are not independent.

use crate::error::StackupError;
use crate::models::{microstrip, stripline_symmetric};

/// The most that a tolerance may be of its value: a tolerance as large as the value itself would
/// let the width, the height or the permittivity reach zero.
pub const MAX_TOLERANCE_FRACTION: f64 = 0.5;

/// The kind of line whose spread is worked out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpreadKind {
    /// A microstrip: dielectric and a plane on one side and air on the other.
    Microstrip,
    /// A centred stripline: the same dielectric and a plane on both sides.
    Stripline,
}

/// The nominal geometry of a line, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpreadGeometry {
    /// The kind of line.
    pub kind: SpreadKind,
    /// The width of the track.
    pub width_m: f64,
    /// The thickness of the copper of the track.
    pub thickness_m: f64,
    /// The height of the dielectric of a microstrip, or the spacing of the two planes of a
    /// stripline.
    pub height_m: f64,
    /// The relative permittivity of the dielectric.
    pub er: f64,
}

/// How far each value may be from its nominal one, in the unit of the value: metres for the
/// width and the height, a plain number for the permittivity. Each is a plus or minus amount.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpreadTolerances {
    /// The width of the track, plus or minus.
    pub width_m: f64,
    /// The height of the dielectric, plus or minus.
    pub height_m: f64,
    /// The relative permittivity, plus or minus.
    pub er: f64,
}

/// The lowest and the highest impedance of the box of the tolerances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spread {
    /// The impedance of the nominal geometry, in ohms.
    pub nominal_ohm: f64,
    /// The lowest impedance of the eight corners, in ohms.
    pub min_ohm: f64,
    /// The highest impedance of the eight corners, in ohms.
    pub max_ohm: f64,
}

impl Spread {
    /// How far below the nominal impedance the lowest corner is, in per cent of the nominal
    /// value (zero or more).
    pub fn minus_percent(&self) -> f64 {
        (self.nominal_ohm - self.min_ohm).max(0.0) / self.nominal_ohm * 100.0
    }

    /// How far above the nominal impedance the highest corner is, in per cent of the nominal
    /// value (zero or more).
    pub fn plus_percent(&self) -> f64 {
        (self.max_ohm - self.nominal_ohm).max(0.0) / self.nominal_ohm * 100.0
    }

    /// True when every corner is within `percent` per cent of `target_ohm`.
    pub fn fits(&self, target_ohm: f64, percent: f64) -> bool {
        if !(target_ohm.is_finite() && target_ohm > 0.0 && percent.is_finite() && percent >= 0.0) {
            return false;
        }
        let slack = target_ohm * percent / 100.0;
        self.min_ohm >= target_ohm - slack && self.max_ohm <= target_ohm + slack
    }
}

fn bad(reason: String) -> StackupError {
    StackupError::OutsideDomain { reason }
}

fn check_amount(name: &str, tolerance: f64, nominal: f64) -> Result<(), StackupError> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(bad(format!(
            "the tolerance of {name} is not a number of at least zero"
        )));
    }
    if tolerance > nominal * MAX_TOLERANCE_FRACTION {
        return Err(bad(format!(
            "the tolerance of {name} is more than half of its value"
        )));
    }
    Ok(())
}

fn impedance(g: &SpreadGeometry, width: f64, height: f64, er: f64) -> Result<f64, StackupError> {
    match g.kind {
        SpreadKind::Microstrip => microstrip(width, g.thickness_m, height, er).map(|(z, _)| z),
        SpreadKind::Stripline => stripline_symmetric(width, g.thickness_m, height, er),
    }
}

/// The impedance of the nominal geometry and the lowest and the highest impedance of the eight
/// corners of the tolerances.
///
/// A width, a height or a permittivity that is not a positive finite number (a permittivity of at
/// least one), a tolerance that is negative, not finite or more than half of its value, and any
/// corner that the model refuses are errors that name the value.
pub fn impedance_spread(
    geometry: &SpreadGeometry,
    tolerances: &SpreadTolerances,
) -> Result<Spread, StackupError> {
    for (name, value) in [
        ("the width", geometry.width_m),
        ("the height", geometry.height_m),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(bad(format!("{name} is not a positive number")));
        }
    }
    if !geometry.er.is_finite() || geometry.er < 1.0 {
        return Err(bad("the relative permittivity is below one".to_string()));
    }
    check_amount("the width", tolerances.width_m, geometry.width_m)?;
    check_amount("the height", tolerances.height_m, geometry.height_m)?;
    check_amount("the permittivity", tolerances.er, geometry.er)?;
    if geometry.er - tolerances.er < 1.0 {
        return Err(bad(
            "the lowest permittivity of the tolerance is below one".to_string()
        ));
    }
    let nominal = impedance(geometry, geometry.width_m, geometry.height_m, geometry.er)?;
    let mut low = nominal;
    let mut high = nominal;
    for dw in [-1.0, 1.0] {
        for dh in [-1.0, 1.0] {
            for de in [-1.0, 1.0] {
                let z = impedance(
                    geometry,
                    geometry.width_m + dw * tolerances.width_m,
                    geometry.height_m + dh * tolerances.height_m,
                    geometry.er + de * tolerances.er,
                )?;
                low = low.min(z);
                high = high.max(z);
            }
        }
    }
    Ok(Spread {
        nominal_ohm: nominal,
        min_ohm: low,
        max_ohm: high,
    })
}

/// The largest plus or minus tolerance of the width of the track, in metres, for which every
/// corner of the box stays within `percent` per cent of `target_ohm`, with the tolerances of the
/// height and of the permittivity taken from `others` (its `width_m` is not used).
///
/// The spread grows with the tolerance of the width, so the answer is found by bisection between
/// zero and the largest tolerance that [`impedance_spread`] accepts (half of the width). It is
/// `Ok(None)` when even a width with no tolerance does not fit, that is, when the other
/// tolerances or the nominal geometry alone already leave the target; `Ok(Some(limit))` with half
/// of the width when every tolerance that can be asked fits. A target that is not a positive
/// finite number and a percentage that is not a finite number from 0 to 100 are errors that name
/// the value, as are the errors of [`impedance_spread`].
pub fn max_width_tolerance(
    geometry: &SpreadGeometry,
    others: &SpreadTolerances,
    target_ohm: f64,
    percent: f64,
) -> Result<Option<f64>, StackupError> {
    if !target_ohm.is_finite() || target_ohm <= 0.0 {
        return Err(bad(
            "the target impedance is not a positive number".to_string()
        ));
    }
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(bad(
            "the percentage of the target is not a number from 0 to 100".to_string(),
        ));
    }
    let with = |width_m: f64| SpreadTolerances { width_m, ..*others };
    let fits = |width_m: f64| -> Result<bool, StackupError> {
        Ok(impedance_spread(geometry, &with(width_m))?.fits(target_ohm, percent))
    };
    if !fits(0.0)? {
        return Ok(None);
    }
    let limit = geometry.width_m * MAX_TOLERANCE_FRACTION;
    if fits(limit)? {
        return Ok(Some(limit));
    }
    let (mut low, mut high) = (0.0, limit);
    for _ in 0..80 {
        let middle = 0.5 * (low + high);
        if fits(middle)? {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(Some(low))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = 1.0e-3;

    fn micro() -> SpreadGeometry {
        SpreadGeometry {
            kind: SpreadKind::Microstrip,
            width_m: 0.3 * MM,
            thickness_m: 35.0e-6,
            height_m: 0.2 * MM,
            er: 4.3,
        }
    }

    fn strip() -> SpreadGeometry {
        SpreadGeometry {
            kind: SpreadKind::Stripline,
            width_m: 0.1 * MM,
            thickness_m: 17.0e-6,
            height_m: 0.4 * MM,
            er: 4.2,
        }
    }

    #[test]
    fn no_tolerance_gives_the_nominal_impedance_at_every_corner() {
        for g in [micro(), strip()] {
            let s = impedance_spread(&g, &SpreadTolerances::default()).unwrap();
            assert_eq!(s.min_ohm, s.nominal_ohm);
            assert_eq!(s.max_ohm, s.nominal_ohm);
            assert_eq!(s.minus_percent(), 0.0);
            assert_eq!(s.plus_percent(), 0.0);
        }
    }

    #[test]
    fn the_extremes_are_at_the_corners_that_the_trends_say() {
        let t = SpreadTolerances {
            width_m: 0.02 * MM,
            height_m: 0.02 * MM,
            er: 0.2,
        };
        for g in [micro(), strip()] {
            let s = impedance_spread(&g, &t).unwrap();
            let z = |dw: f64, dh: f64, de: f64| {
                impedance(
                    &g,
                    g.width_m + dw * t.width_m,
                    g.height_m + dh * t.height_m,
                    g.er + de * t.er,
                )
                .unwrap()
            };
            assert_eq!(s.max_ohm, z(-1.0, 1.0, -1.0));
            assert_eq!(s.min_ohm, z(1.0, -1.0, 1.0));
            assert!(s.min_ohm < s.nominal_ohm && s.nominal_ohm < s.max_ohm);
        }
    }

    #[test]
    fn the_spread_grows_with_each_tolerance() {
        let small = SpreadTolerances {
            width_m: 0.01 * MM,
            height_m: 0.01 * MM,
            er: 0.1,
        };
        let big = SpreadTolerances {
            width_m: 0.03 * MM,
            height_m: 0.02 * MM,
            er: 0.3,
        };
        for g in [micro(), strip()] {
            let a = impedance_spread(&g, &small).unwrap();
            let b = impedance_spread(&g, &big).unwrap();
            assert!(b.min_ohm < a.min_ohm && b.max_ohm > a.max_ohm);
        }
    }

    #[test]
    fn fits_compares_the_corners_with_a_target_and_a_percentage() {
        let t = SpreadTolerances {
            width_m: 0.02 * MM,
            height_m: 0.0,
            er: 0.0,
        };
        let s = impedance_spread(&micro(), &t).unwrap();
        let wide = s.plus_percent().max(s.minus_percent()) + 1.0;
        assert!(s.fits(s.nominal_ohm, wide));
        assert!(!s.fits(s.nominal_ohm, 0.0));
        assert!(!s.fits(s.nominal_ohm + 50.0, 1.0));
        assert!(!s.fits(f64::NAN, 10.0));
        assert!(!s.fits(50.0, -1.0));
        assert!(!s.fits(0.0, 10.0));
    }

    #[test]
    fn values_that_are_not_valid_are_refused_and_named() {
        let none = SpreadTolerances::default();
        let mut g = micro();
        g.width_m = 0.0;
        let e = impedance_spread(&g, &none).unwrap_err().to_string();
        assert!(e.contains("the width"), "{e}");
        let mut g = micro();
        g.height_m = f64::NAN;
        let e = impedance_spread(&g, &none).unwrap_err().to_string();
        assert!(e.contains("the height"), "{e}");
        let mut g = micro();
        g.er = 0.9;
        let e = impedance_spread(&g, &none).unwrap_err().to_string();
        assert!(e.contains("permittivity"), "{e}");
    }

    #[test]
    fn tolerances_that_are_not_valid_are_refused_and_named() {
        for (t, word) in [
            (
                SpreadTolerances {
                    width_m: -1.0e-6,
                    ..Default::default()
                },
                "the width",
            ),
            (
                SpreadTolerances {
                    height_m: f64::INFINITY,
                    ..Default::default()
                },
                "the height",
            ),
            (
                SpreadTolerances {
                    width_m: 0.2 * MM,
                    ..Default::default()
                },
                "the width",
            ),
            (
                SpreadTolerances {
                    er: 2.5,
                    ..Default::default()
                },
                "the permittivity",
            ),
        ] {
            let e = impedance_spread(&micro(), &t).unwrap_err().to_string();
            assert!(e.contains(word), "{e}");
        }
    }

    #[test]
    fn a_permittivity_that_reaches_below_one_is_refused() {
        let mut g = micro();
        g.er = 1.2;
        let t = SpreadTolerances {
            er: 0.3,
            ..Default::default()
        };
        let e = impedance_spread(&g, &t).unwrap_err().to_string();
        assert!(e.contains("below one"), "{e}");
    }

    #[test]
    fn a_stripline_whose_copper_fills_the_spacing_is_refused() {
        let mut g = strip();
        g.thickness_m = g.height_m - 5.0e-6;
        let t = SpreadTolerances {
            height_m: 0.01 * MM,
            ..Default::default()
        };
        assert!(impedance_spread(&g, &t).is_err());
    }

    #[test]
    fn the_largest_width_tolerance_is_the_edge_of_what_fits() {
        let g = micro();
        let others = SpreadTolerances {
            width_m: 0.0,
            height_m: 0.005 * MM,
            er: 0.1,
        };
        let nominal = impedance_spread(&g, &others).unwrap().nominal_ohm;
        let found = max_width_tolerance(&g, &others, nominal, 10.0)
            .unwrap()
            .unwrap();
        let at = |w: f64| {
            impedance_spread(
                &g,
                &SpreadTolerances {
                    width_m: w,
                    ..others
                },
            )
            .unwrap()
        };
        assert!(found > 0.0 && found < g.width_m * MAX_TOLERANCE_FRACTION);
        assert!(at(found).fits(nominal, 10.0));
        assert!(!at(found * 1.01).fits(nominal, 10.0));
    }

    #[test]
    fn a_target_that_the_other_tolerances_already_leave_gives_no_tolerance() {
        let g = micro();
        let others = SpreadTolerances {
            width_m: 0.0,
            height_m: 0.05 * MM,
            er: 0.4,
        };
        let nominal = impedance_spread(&g, &others).unwrap().nominal_ohm;
        assert_eq!(max_width_tolerance(&g, &others, nominal, 1.0), Ok(None));
        assert_eq!(
            max_width_tolerance(&g, &others, nominal * 2.0, 5.0),
            Ok(None)
        );
    }

    #[test]
    fn a_wide_enough_percentage_accepts_every_tolerance_that_can_be_asked() {
        let g = strip();
        let others = SpreadTolerances::default();
        let nominal = impedance_spread(&g, &others).unwrap().nominal_ohm;
        let found = max_width_tolerance(&g, &others, nominal, 100.0).unwrap();
        assert_eq!(found, Some(g.width_m * MAX_TOLERANCE_FRACTION));
    }

    #[test]
    fn the_target_and_the_percentage_are_checked_and_named() {
        let g = micro();
        let none = SpreadTolerances::default();
        for (target, percent, word) in [
            (0.0, 10.0, "target"),
            (f64::NAN, 10.0, "target"),
            (50.0, -1.0, "percentage"),
            (50.0, 101.0, "percentage"),
            (50.0, f64::INFINITY, "percentage"),
        ] {
            let e = max_width_tolerance(&g, &none, target, percent)
                .unwrap_err()
                .to_string();
            assert!(e.contains(word), "{e}");
        }
    }
}
