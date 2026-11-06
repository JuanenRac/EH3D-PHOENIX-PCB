// SPDX-License-Identifier: GPL-3.0-or-later
//! Stability of an amplifier from its S parameters: the stability circles of the source and of
//! the load as data, the side of each circle that is stable, and the simultaneous conjugate match
//! with the gain it gives when the two-port is unconditionally stable.
//!
//! The factors K, delta, mu and mu' and the maximum stable and available gains are in
//! [`crate::network`]. The circles and the match follow Pozar, *Microwave Engineering*, sections
//! 11.3 and 11.4:
//!
//! * load plane: `C_L = (S22 - D S11*)* / (|S22|^2 - |D|^2)`, `r_L = |S12 S21| / ||S22|^2 - |D|^2|`;
//! * source plane: `C_S = (S11 - D S22*)* / (|S11|^2 - |D|^2)`, `r_S = |S12 S21| / ||S11|^2 - |D|^2|`;
//! * match: `G_s = (B1 -+ sqrt(B1^2 - 4|C1|^2)) / (2 C1)` with `B1 = 1 + |S11|^2 - |S22|^2 - |D|^2`
//!   and `C1 = S11 - D S22*` (the sign is minus for `B1 > 0`), and the same for the load with
//!   `B2 = 1 + |S22|^2 - |S11|^2 - |D|^2` and `C2 = S22 - D S11*`.
//!
//! `D` is the determinant of the S matrix. Everything is at one frequency and for linear
//! operation.

use crate::complex::Complex;
use crate::error::RfError;
use crate::network::{stability, transducer_gain, Mat2, Stability};

/// Which side of a stability circle is stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StableSide {
    /// The inside of the circle is stable.
    Inside,
    /// The outside of the circle is stable.
    Outside,
}

/// A stability circle in the plane of a reflection coefficient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StabilityCircle {
    /// The centre.
    pub center: Complex,
    /// The radius.
    pub radius: f64,
    /// The stable side of the circle.
    pub stable: StableSide,
    /// Whether the circle cuts the unit circle of the Smith chart, so that part of the chart is
    /// unstable.
    pub cuts_chart: bool,
}

impl StabilityCircle {
    /// `count` points of the circle, evenly spaced in angle from angle zero.
    pub fn points(&self, count: usize) -> Vec<Complex> {
        (0..count)
            .map(|k| {
                let angle = 2.0 * std::f64::consts::PI * k as f64 / count.max(1) as f64;
                Complex::new(
                    self.center.re + self.radius * angle.cos(),
                    self.center.im + self.radius * angle.sin(),
                )
            })
            .collect()
    }
}

fn circle(center: Complex, radius: f64, origin_stable: bool, tolerance: f64) -> StabilityCircle {
    let origin_inside = center.abs() < radius;
    // a coefficient of zero at the plane is stable when the other side's input reflection is
    // below one; the stable side is the one that holds the origin in that case
    let stable = if origin_inside == origin_stable {
        StableSide::Inside
    } else {
        StableSide::Outside
    };
    let distance = center.abs();
    let cuts_chart =
        distance + radius > 1.0 + tolerance && (distance - radius).abs() < 1.0 - tolerance;
    StabilityCircle {
        center,
        radius,
        stable,
        cuts_chart,
    }
}

/// The two stability circles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StabilityCircles {
    /// The circle in the plane of the source reflection coefficient.
    pub source: StabilityCircle,
    /// The circle in the plane of the load reflection coefficient.
    pub load: StabilityCircle,
}

/// The stability circles of a two-port. A two-port with a determinant that makes a circle
/// degenerate into a line (`|S11|^2 = |D|^2` or `|S22|^2 = |D|^2`) or with `S12 S21 = 0` is
/// refused.
pub fn stability_circles(s: &Mat2, tolerance: f64) -> Result<StabilityCircles, RfError> {
    let delta = s.det();
    let product = (s.m12 * s.m21).abs();
    if product == 0.0 {
        return Err(RfError::Singular {
            what: "a unilateral two-port has no stability circles".to_string(),
        });
    }
    let d2 = delta.norm_sqr();
    let load_denominator = s.m22.norm_sqr() - d2;
    let source_denominator = s.m11.norm_sqr() - d2;
    if load_denominator.abs() <= tolerance || source_denominator.abs() <= tolerance {
        return Err(RfError::Singular {
            what: "a stability circle is a straight line for these parameters".to_string(),
        });
    }
    let load_center = (s.m22 - delta * s.m11.conj())
        .conj()
        .scale(1.0 / load_denominator);
    let source_center = (s.m11 - delta * s.m22.conj())
        .conj()
        .scale(1.0 / source_denominator);
    Ok(StabilityCircles {
        source: circle(
            source_center,
            product / source_denominator.abs(),
            s.m22.abs() < 1.0,
            tolerance,
        ),
        load: circle(
            load_center,
            product / load_denominator.abs(),
            s.m11.abs() < 1.0,
            tolerance,
        ),
    })
}

/// The simultaneous conjugate match of a two-port that is unconditionally stable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConjugateMatch {
    /// The source reflection coefficient of the match.
    pub gamma_source: Complex,
    /// The load reflection coefficient of the match.
    pub gamma_load: Complex,
    /// The transducer power gain, linear, with that source and load: the maximum available gain.
    pub gain: f64,
}

/// The simultaneous conjugate match, or `None` when the two-port is not unconditionally stable
/// (`K > 1` and `|D| < 1` are needed) or is unilateral.
pub fn conjugate_match(s: &Mat2) -> Result<Option<ConjugateMatch>, RfError> {
    let figures = stability(s);
    if !figures.unconditionally_stable || !figures.k.is_finite() {
        return Ok(None);
    }
    let delta = figures.delta;
    let d2 = delta.norm_sqr();
    let b1 = 1.0 + s.m11.norm_sqr() - s.m22.norm_sqr() - d2;
    let b2 = 1.0 + s.m22.norm_sqr() - s.m11.norm_sqr() - d2;
    let c1 = s.m11 - delta * s.m22.conj();
    let c2 = s.m22 - delta * s.m11.conj();
    let root = |b: f64, c: Complex| -> Result<Complex, RfError> {
        let radicand = b * b - 4.0 * c.norm_sqr();
        let r = radicand.max(0.0).sqrt();
        let top = if b > 0.0 { b - r } else { b + r };
        Complex::real(top).checked_div(c.scale(2.0))
    };
    let gamma_source = root(b1, c1)?;
    let gamma_load = root(b2, c2)?;
    let gain = transducer_gain(s, gamma_source, gamma_load)?;
    Ok(Some(ConjugateMatch {
        gamma_source,
        gamma_load,
        gain,
    }))
}

/// The figures of stability of a two-port with its circles, when they exist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StabilityReport {
    /// K, the determinant, mu, mu' and the verdict.
    pub figures: Stability,
    /// The circles, or `None` for a two-port that has none (unilateral or degenerate).
    pub circles: Option<StabilityCircles>,
    /// The simultaneous conjugate match, or `None` when the two-port is not unconditionally
    /// stable.
    pub conjugate: Option<ConjugateMatch>,
}

/// Everything about the stability of a two-port at one frequency.
pub fn stability_report(s: &Mat2, tolerance: f64) -> Result<StabilityReport, RfError> {
    Ok(StabilityReport {
        figures: stability(s),
        circles: stability_circles(s, tolerance).ok(),
        conjugate: conjugate_match(s)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{max_available_gain, max_stable_gain};

    fn polar(mag: f64, deg: f64) -> Complex {
        Complex::from_polar(mag, deg.to_radians())
    }

    /// A conditionally stable transistor with S parameters of the size found in textbooks.
    fn pozar_transistor() -> Mat2 {
        Mat2::new(
            polar(0.65, -95.0),
            polar(0.035, 40.0),
            polar(5.0, 115.0),
            polar(0.8, -35.0),
        )
    }

    /// A two-port that is unconditionally stable: well below the oscillation limit.
    fn stable_two_port() -> Mat2 {
        Mat2::new(
            polar(0.3, -40.0),
            polar(0.02, 60.0),
            polar(3.0, 80.0),
            polar(0.4, -30.0),
        )
    }

    #[test]
    fn the_circles_of_a_conditionally_stable_transistor_match_the_formulas() {
        let s = pozar_transistor();
        let delta = s.det();
        let d2 = delta.norm_sqr();
        let circles = stability_circles(&s, 1e-12).unwrap();
        // load circle
        let denominator = s.m22.norm_sqr() - d2;
        let centre = (s.m22 - delta * s.m11.conj())
            .conj()
            .scale(1.0 / denominator);
        let radius = (s.m12 * s.m21).abs() / denominator.abs();
        assert!((circles.load.center - centre).abs() < 1e-12);
        assert!((circles.load.radius - radius).abs() < 1e-12);
        // the transistor is conditionally stable (K below 1) and a circle cuts the chart
        assert!(stability(&s).k < 1.0);
        assert!(circles.load.cuts_chart || circles.source.cuts_chart);
    }

    #[test]
    fn the_origin_decides_the_stable_side() {
        let s = pozar_transistor();
        let circles = stability_circles(&s, 1e-12).unwrap();
        // |S11| < 1 and |S22| < 1: the origin is stable in both planes
        for (circle, label) in [(circles.load, "load"), (circles.source, "source")] {
            let origin_inside = circle.center.abs() < circle.radius;
            match circle.stable {
                StableSide::Inside => assert!(origin_inside, "{label}"),
                StableSide::Outside => assert!(!origin_inside, "{label}"),
            }
        }
    }

    #[test]
    fn a_reflection_with_magnitude_above_one_flips_the_stable_side() {
        // |S11| = 1.2: the load plane origin is unstable
        let s = Mat2::new(
            polar(1.2, -30.0),
            polar(0.1, 40.0),
            polar(2.0, 100.0),
            polar(0.5, -20.0),
        );
        let circles = stability_circles(&s, 1e-12).unwrap();
        let origin_inside = circles.load.center.abs() < circles.load.radius;
        match circles.load.stable {
            StableSide::Inside => assert!(!origin_inside),
            StableSide::Outside => assert!(origin_inside),
        }
    }

    #[test]
    fn points_on_the_circle_make_the_input_reflection_one() {
        // on the load stability circle |Gamma_in| = 1
        let s = pozar_transistor();
        let circles = stability_circles(&s, 1e-12).unwrap();
        for g in circles.load.points(16) {
            let gamma_in = s.m11 + s.m12 * s.m21 * g / (Complex::ONE - s.m22 * g);
            assert!((gamma_in.abs() - 1.0).abs() < 1e-9, "{}", gamma_in.abs());
        }
        for g in circles.source.points(16) {
            let gamma_out = s.m22 + s.m12 * s.m21 * g / (Complex::ONE - s.m11 * g);
            assert!((gamma_out.abs() - 1.0).abs() < 1e-9, "{}", gamma_out.abs());
        }
    }

    #[test]
    fn a_conditionally_stable_transistor_has_no_conjugate_match() {
        assert_eq!(conjugate_match(&pozar_transistor()).unwrap(), None);
        let report = stability_report(&pozar_transistor(), 1e-12).unwrap();
        assert!(!report.figures.unconditionally_stable);
        assert!(report.circles.is_some());
        assert!(report.conjugate.is_none());
    }

    #[test]
    fn the_conjugate_match_gives_the_maximum_available_gain() {
        let s = stable_two_port();
        assert!(stability(&s).unconditionally_stable);
        let m = conjugate_match(&s).unwrap().unwrap();
        let mag = max_available_gain(&s).unwrap();
        assert!((m.gain - mag).abs() / mag < 1e-9, "{} {}", m.gain, mag);
        assert!(m.gamma_source.abs() < 1.0 && m.gamma_load.abs() < 1.0);
        // the match makes the input reflection the conjugate of the source's
        let gamma_in = s.m11 + s.m12 * s.m21 * m.gamma_load / (Complex::ONE - s.m22 * m.gamma_load);
        assert!((gamma_in - m.gamma_source.conj()).abs() < 1e-9);
        // and the maximum available gain is below the maximum stable gain
        assert!(mag <= max_stable_gain(&s));
    }

    #[test]
    fn a_unilateral_two_port_has_no_circles() {
        let s = Mat2::new(
            polar(0.5, 0.0),
            Complex::ZERO,
            polar(2.0, 0.0),
            polar(0.5, 0.0),
        );
        assert!(matches!(
            stability_circles(&s, 1e-12),
            Err(RfError::Singular { .. })
        ));
        assert_eq!(conjugate_match(&s).unwrap(), None);
        let report = stability_report(&s, 1e-12).unwrap();
        assert!(report.circles.is_none());
    }
}
