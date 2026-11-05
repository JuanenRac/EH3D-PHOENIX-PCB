// SPDX-License-Identifier: GPL-3.0-or-later
//! Coordinates of the Smith chart: for a point or a sweep, the reflection coefficient (the
//! position on the chart) and the normalised impedance and admittance (the values the circles
//! are labelled with), and the circles of constant resistance, reactance, conductance and
//! susceptance, so that another module can draw the chart. Nothing is drawn here.

use crate::complex::Complex;
use crate::error::{positive, RfError};
use crate::network::reflection_from_impedance;
use crate::touchstone::Touchstone;

/// One point of the chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmithPoint {
    /// The frequency in Hz when the point comes from a sweep.
    pub freq_hz: Option<f64>,
    /// The reflection coefficient: the position of the point in the unit disc.
    pub gamma: Complex,
    /// The impedance divided by the reference impedance (`r + jx`); `None` at the open circuit.
    pub z: Option<Complex>,
    /// The admittance multiplied by the reference impedance (`g + jb`); `None` at the short
    /// circuit.
    pub y: Option<Complex>,
}

impl SmithPoint {
    /// The point of a reflection coefficient.
    pub fn from_gamma(gamma: Complex, freq_hz: Option<f64>) -> Result<SmithPoint, RfError> {
        if !gamma.is_finite() {
            return Err(RfError::InvalidNumber {
                field: "reflection coefficient".to_string(),
                reason: "must be finite".to_string(),
            });
        }
        let z = (Complex::ONE + gamma)
            .checked_div(Complex::ONE - gamma)
            .ok();
        let y = (Complex::ONE - gamma)
            .checked_div(Complex::ONE + gamma)
            .ok();
        Ok(SmithPoint {
            freq_hz,
            gamma,
            z,
            y,
        })
    }

    /// The point of an impedance in ohm against a reference impedance.
    pub fn from_impedance(
        z_ohm: Complex,
        z0_ohm: f64,
        freq_hz: Option<f64>,
    ) -> Result<SmithPoint, RfError> {
        let gamma = reflection_from_impedance(z_ohm, z0_ohm)?;
        SmithPoint::from_gamma(gamma, freq_hz)
    }

    /// The Cartesian position of the point on the chart, `(re, im)` of the coefficient.
    pub fn position(&self) -> (f64, f64) {
        (self.gamma.re, self.gamma.im)
    }

    /// Whether the point is inside or on the unit circle (a passive load).
    pub fn is_passive(&self) -> bool {
        self.gamma.abs() <= 1.0
    }
}

/// The points of a sweep of reflection coefficients at frequencies in Hz.
pub fn sweep(freqs_hz: &[f64], gammas: &[Complex]) -> Result<Vec<SmithPoint>, RfError> {
    if freqs_hz.len() != gammas.len() {
        return Err(RfError::InvalidNetwork {
            reason: format!(
                "{} frequencies and {} coefficients",
                freqs_hz.len(),
                gammas.len()
            ),
        });
    }
    freqs_hz
        .iter()
        .zip(gammas)
        .map(|(f, g)| SmithPoint::from_gamma(*g, Some(*f)))
        .collect()
}

/// The points of a sweep of impedances in ohm at frequencies in Hz.
pub fn sweep_impedances(
    freqs_hz: &[f64],
    z_ohm: &[Complex],
    z0_ohm: f64,
) -> Result<Vec<SmithPoint>, RfError> {
    if freqs_hz.len() != z_ohm.len() {
        return Err(RfError::InvalidNetwork {
            reason: format!(
                "{} frequencies and {} impedances",
                freqs_hz.len(),
                z_ohm.len()
            ),
        });
    }
    freqs_hz
        .iter()
        .zip(z_ohm)
        .map(|(f, z)| SmithPoint::from_impedance(*z, z0_ohm, Some(*f)))
        .collect()
}

/// The chart of the reflection at one port of a Touchstone file: the diagonal entry of that
/// port (`S11` for port 1), at every frequency point. The file is converted to S parameters
/// first. Ports are counted from 1.
pub fn sweep_from_touchstone(file: &Touchstone, port: usize) -> Result<Vec<SmithPoint>, RfError> {
    if port == 0 || port > file.ports {
        return Err(RfError::InvalidNetwork {
            reason: format!("port {port} is not in a file of {} ports", file.ports),
        });
    }
    let mut points = Vec::with_capacity(file.points.len());
    for (index, point) in file.points.iter().enumerate() {
        let s = file.s_matrix(index)?;
        points.push(SmithPoint::from_gamma(
            s.get(port - 1, port - 1),
            Some(point.freq_hz),
        )?);
    }
    Ok(points)
}

/// A circle on the chart, for drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// The horizontal position of the centre.
    pub center_x: f64,
    /// The vertical position of the centre.
    pub center_y: f64,
    /// The radius.
    pub radius: f64,
}

/// The circle of constant normalised resistance `r` (zero or more): the centre is
/// `(r / (1 + r), 0)` and the radius `1 / (1 + r)`.
pub fn resistance_circle(r: f64) -> Result<Circle, RfError> {
    if !r.is_finite() || r < 0.0 {
        return Err(RfError::InvalidNumber {
            field: "normalised resistance".to_string(),
            reason: "must be a finite number, zero or more".to_string(),
        });
    }
    Ok(Circle {
        center_x: r / (1.0 + r),
        center_y: 0.0,
        radius: 1.0 / (1.0 + r),
    })
}

/// The circle of constant normalised reactance `x` (not zero; the arc inside the unit disc is
/// the part to draw): the centre is `(1, 1 / x)` and the radius `1 / |x|`.
pub fn reactance_circle(x: f64) -> Result<Circle, RfError> {
    if !x.is_finite() || x == 0.0 {
        return Err(RfError::InvalidNumber {
            field: "normalised reactance".to_string(),
            reason: "must be a finite number other than zero (zero is the real axis)".to_string(),
        });
    }
    Ok(Circle {
        center_x: 1.0,
        center_y: 1.0 / x,
        radius: 1.0 / x.abs(),
    })
}

/// The circle of constant normalised conductance `g` (zero or more): the centre is
/// `(-g / (1 + g), 0)` and the radius `1 / (1 + g)`.
pub fn conductance_circle(g: f64) -> Result<Circle, RfError> {
    positive_or_zero("normalised conductance", g)?;
    Ok(Circle {
        center_x: -g / (1.0 + g),
        center_y: 0.0,
        radius: 1.0 / (1.0 + g),
    })
}

/// The circle of constant normalised susceptance `b` (not zero): the centre is `(-1, -1 / b)` and
/// the radius `1 / |b|`.
pub fn susceptance_circle(b: f64) -> Result<Circle, RfError> {
    positive("susceptance magnitude", b.abs())?;
    Ok(Circle {
        center_x: -1.0,
        center_y: -1.0 / b,
        radius: 1.0 / b.abs(),
    })
}

fn positive_or_zero(field: &str, value: f64) -> Result<(), RfError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(RfError::InvalidNumber {
            field: field.to_string(),
            reason: "must be a finite number, zero or more".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::RfSettings;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    #[test]
    fn the_centre_of_the_chart_is_the_reference_impedance() {
        let p = SmithPoint::from_impedance(Complex::real(50.0), 50.0, None).unwrap();
        assert_eq!(p.position(), (0.0, 0.0));
        assert!((p.z.unwrap() - Complex::ONE).abs() < 1e-15);
        assert!((p.y.unwrap() - Complex::ONE).abs() < 1e-15);
        assert!(p.is_passive() && p.freq_hz.is_none());
    }

    #[test]
    fn a_complex_impedance_has_matching_z_and_y() {
        let p = SmithPoint::from_impedance(c(25.0, 25.0), 50.0, Some(1e9)).unwrap();
        let z = p.z.unwrap();
        assert!((z - c(0.5, 0.5)).abs() < 1e-12);
        let y = p.y.unwrap();
        assert!((z * y - Complex::ONE).abs() < 1e-12);
        assert_eq!(p.freq_hz, Some(1e9));
    }

    #[test]
    fn open_and_short_have_no_finite_value_on_one_side() {
        let open = SmithPoint::from_gamma(Complex::ONE, None).unwrap();
        assert!(open.z.is_none() && open.y.unwrap().abs() < 1e-15);
        let short = SmithPoint::from_gamma(Complex::real(-1.0), None).unwrap();
        assert!(short.y.is_none() && short.z.unwrap().abs() < 1e-15);
        assert!(SmithPoint::from_gamma(c(f64::NAN, 0.0), None).is_err());
        let active = SmithPoint::from_gamma(c(1.2, 0.0), None).unwrap();
        assert!(!active.is_passive());
    }

    #[test]
    fn sweeps_keep_their_frequencies() {
        let f = [1e9, 2e9, 3e9];
        let g = [c(0.1, 0.0), c(0.0, 0.2), c(-0.3, 0.1)];
        let points = sweep(&f, &g).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[1].freq_hz, Some(2e9));
        assert_eq!(points[2].gamma, g[2]);
        assert!(sweep(&f, &g[..2]).is_err());
        let z = [c(50.0, 0.0), c(100.0, 0.0), c(50.0, 50.0)];
        let by_z = sweep_impedances(&f, &z, 50.0).unwrap();
        assert!(by_z[0].gamma.abs() < 1e-15);
        assert!((by_z[1].gamma.re - 1.0 / 3.0).abs() < 1e-12);
        assert!(sweep_impedances(&f, &z[..1], 50.0).is_err());
        assert!(sweep_impedances(&f, &z, 0.0).is_err());
    }

    #[test]
    fn a_touchstone_file_gives_one_point_per_frequency() {
        let text = "# GHz S RI\n1 0.1 0.2 0.9 0 0.9 0 0.3 0.4\n2 0.5 0 0.8 0 0.8 0 0.6 -0.1\n";
        let file = Touchstone::parse(text, 2, &RfSettings::default()).unwrap();
        let first = sweep_from_touchstone(&file, 1).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].gamma, c(0.1, 0.2));
        assert_eq!(first[1].freq_hz, Some(2e9));
        let second = sweep_from_touchstone(&file, 2).unwrap();
        assert_eq!(second[1].gamma, c(0.6, -0.1));
        assert!(sweep_from_touchstone(&file, 0).is_err());
        assert!(sweep_from_touchstone(&file, 3).is_err());
    }

    /// A point is on the circle of its resistance, its reactance, its conductance and its
    /// susceptance.
    #[test]
    fn the_circles_pass_through_the_points_they_describe() {
        let on = |circle: &Circle, g: Complex| {
            ((g.re - circle.center_x).hypot(g.im - circle.center_y) - circle.radius).abs() < 1e-12
        };
        for z in [
            c(0.5, 0.5),
            c(2.0, -1.0),
            c(0.2, 3.0),
            c(1.0, 0.0),
            c(5.0, -0.5),
        ] {
            let p = SmithPoint::from_impedance(z.scale(50.0), 50.0, None).unwrap();
            let zn = p.z.unwrap();
            let yn = p.y.unwrap();
            assert!(on(&resistance_circle(zn.re).unwrap(), p.gamma), "{z:?}");
            assert!(on(&conductance_circle(yn.re).unwrap(), p.gamma), "{z:?}");
            if zn.im.abs() > 1e-9 {
                assert!(on(&reactance_circle(zn.im).unwrap(), p.gamma), "{z:?}");
            }
            if yn.im.abs() > 1e-9 {
                assert!(on(&susceptance_circle(yn.im).unwrap(), p.gamma), "{z:?}");
            }
        }
    }

    #[test]
    fn the_circles_have_their_book_values() {
        let unit = resistance_circle(0.0).unwrap();
        assert_eq!((unit.center_x, unit.center_y, unit.radius), (0.0, 0.0, 1.0));
        let one = resistance_circle(1.0).unwrap();
        assert_eq!((one.center_x, one.radius), (0.5, 0.5));
        let x1 = reactance_circle(1.0).unwrap();
        assert_eq!((x1.center_x, x1.center_y, x1.radius), (1.0, 1.0, 1.0));
        let xm = reactance_circle(-2.0).unwrap();
        assert_eq!((xm.center_y, xm.radius), (-0.5, 0.5));
        let g1 = conductance_circle(1.0).unwrap();
        assert_eq!((g1.center_x, g1.radius), (-0.5, 0.5));
        let b1 = susceptance_circle(1.0).unwrap();
        assert_eq!((b1.center_x, b1.center_y, b1.radius), (-1.0, -1.0, 1.0));
    }

    #[test]
    fn circles_refuse_bad_values() {
        assert!(resistance_circle(-1.0).is_err() && resistance_circle(f64::NAN).is_err());
        assert!(reactance_circle(0.0).is_err() && reactance_circle(f64::INFINITY).is_err());
        assert!(conductance_circle(-0.1).is_err());
        assert!(susceptance_circle(0.0).is_err() && susceptance_circle(f64::NAN).is_err());
    }
}
