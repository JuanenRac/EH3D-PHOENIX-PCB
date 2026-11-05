// SPDX-License-Identifier: GPL-3.0-or-later
//! Two-port networks: the 2 x 2 matrix, the conversions between S, Z, Y and ABCD parameters, the
//! cascade, and the figures read from them (reflection coefficient, VSWR, return loss, insertion
//! loss, group delay, gain, stability, passivity and reciprocity).
//!
//! The conversions follow the usual definitions (Pozar, *Microwave Engineering*, chapter 4): both
//! ports share one real reference impedance `z0`, and the ABCD matrix relates the voltage and
//! the current at port 1 to those at port 2 with the current of port 2 flowing out of the
//! network.

use crate::complex::Complex;
use crate::error::{positive, RfError};
use crate::matrix::CMatrix;
use std::f64::consts::PI;

/// A 2 x 2 complex matrix: the parameters of a two-port, whatever their kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat2 {
    /// Row 1, column 1.
    pub m11: Complex,
    /// Row 1, column 2.
    pub m12: Complex,
    /// Row 2, column 1.
    pub m21: Complex,
    /// Row 2, column 2.
    pub m22: Complex,
}

impl Mat2 {
    /// A matrix from its four entries.
    pub const fn new(m11: Complex, m12: Complex, m21: Complex, m22: Complex) -> Mat2 {
        Mat2 { m11, m12, m21, m22 }
    }

    /// The identity matrix (an ABCD matrix of a plain connection).
    pub const fn identity() -> Mat2 {
        Mat2::new(Complex::ONE, Complex::ZERO, Complex::ZERO, Complex::ONE)
    }

    /// The determinant.
    pub fn det(&self) -> Complex {
        self.m11 * self.m22 - self.m12 * self.m21
    }

    /// The product `self * other`.
    pub fn times(&self, other: &Mat2) -> Mat2 {
        Mat2 {
            m11: self.m11 * other.m11 + self.m12 * other.m21,
            m12: self.m11 * other.m12 + self.m12 * other.m22,
            m21: self.m21 * other.m11 + self.m22 * other.m21,
            m22: self.m21 * other.m12 + self.m22 * other.m22,
        }
    }

    /// Whether the four entries are finite.
    pub fn is_finite(&self) -> bool {
        self.m11.is_finite() && self.m12.is_finite() && self.m21.is_finite() && self.m22.is_finite()
    }

    /// The largest absolute difference between the entries of two matrices.
    pub fn max_difference(&self, other: &Mat2) -> f64 {
        [
            self.m11 - other.m11,
            self.m12 - other.m12,
            self.m21 - other.m21,
            self.m22 - other.m22,
        ]
        .iter()
        .map(|z| z.abs())
        .fold(0.0, f64::max)
    }

    /// The matrix as a general [`CMatrix`].
    pub fn to_cmatrix(self) -> Result<CMatrix, RfError> {
        CMatrix::new(2, vec![self.m11, self.m12, self.m21, self.m22])
    }

    /// A 2 x 2 [`CMatrix`] as a [`Mat2`]; any other size is refused.
    pub fn from_cmatrix(matrix: &CMatrix) -> Result<Mat2, RfError> {
        if matrix.size() != 2 {
            return Err(RfError::InvalidNetwork {
                reason: format!("a two-port needs 2 ports, not {}", matrix.size()),
            });
        }
        Ok(Mat2::new(
            matrix.get(0, 0),
            matrix.get(0, 1),
            matrix.get(1, 0),
            matrix.get(1, 1),
        ))
    }
}

fn finite_matrix(m: Mat2, what: &str) -> Result<Mat2, RfError> {
    if m.is_finite() {
        Ok(m)
    } else {
        Err(RfError::Singular {
            what: format!("{what} is not finite"),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// conversions

/// `Z = z0 (I + S) (I - S)^-1`.
pub fn s_to_z(s: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let (sm, id) = (s.to_cmatrix()?, CMatrix::identity(2));
    let z = id
        .plus(&sm)?
        .times(&id.minus(&sm)?.inverse()?)?
        .scaled(Complex::real(z0))?;
    Mat2::from_cmatrix(&z)
}

/// `S = (Z - z0 I) (Z + z0 I)^-1`.
pub fn z_to_s(z: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let zm = z.to_cmatrix()?;
    let shift = CMatrix::identity(2).scaled(Complex::real(z0))?;
    let s = zm.minus(&shift)?.times(&zm.plus(&shift)?.inverse()?)?;
    Mat2::from_cmatrix(&s)
}

/// `Y = (1 / z0) (I - S) (I + S)^-1`.
pub fn s_to_y(s: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let (sm, id) = (s.to_cmatrix()?, CMatrix::identity(2));
    let y = id
        .minus(&sm)?
        .times(&id.plus(&sm)?.inverse()?)?
        .scaled(Complex::real(1.0 / z0))?;
    Mat2::from_cmatrix(&y)
}

/// `S = (I - z0 Y) (I + z0 Y)^-1`.
pub fn y_to_s(y: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let (ym, id) = (y.to_cmatrix()?, CMatrix::identity(2));
    let zy = ym.scaled(Complex::real(z0))?;
    let s = id.minus(&zy)?.times(&id.plus(&zy)?.inverse()?)?;
    Mat2::from_cmatrix(&s)
}

/// `Y = Z^-1`.
pub fn z_to_y(z: &Mat2) -> Result<Mat2, RfError> {
    Mat2::from_cmatrix(&z.to_cmatrix()?.inverse()?)
}

/// `Z = Y^-1`.
pub fn y_to_z(y: &Mat2) -> Result<Mat2, RfError> {
    z_to_y(y)
}

/// The ABCD matrix of a two-port given by its S parameters.
pub fn s_to_abcd(s: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let one = Complex::ONE;
    let s12s21 = s.m12 * s.m21;
    let two_s21 = s.m21.scale(2.0);
    let a = ((one + s.m11) * (one - s.m22) + s12s21).checked_div(two_s21)?;
    let b = ((one + s.m11) * (one + s.m22) - s12s21)
        .scale(z0)
        .checked_div(two_s21)?;
    let c = ((one - s.m11) * (one - s.m22) - s12s21).checked_div(two_s21.scale(z0))?;
    let d = ((one - s.m11) * (one + s.m22) + s12s21).checked_div(two_s21)?;
    finite_matrix(Mat2::new(a, b, c, d), "the ABCD matrix")
}

/// The S parameters of a two-port given by its ABCD matrix.
pub fn abcd_to_s(t: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    let (a, b, c, d) = (t.m11, t.m12, t.m21, t.m22);
    let b_n = b.scale(1.0 / z0);
    let c_n = c.scale(z0);
    let den = a + b_n + c_n + d;
    let s11 = (a + b_n - c_n - d).checked_div(den)?;
    let s12 = (a * d - b * c).scale(2.0).checked_div(den)?;
    let s21 = Complex::real(2.0).checked_div(den)?;
    let s22 = (d + b_n - c_n - a).checked_div(den)?;
    finite_matrix(Mat2::new(s11, s12, s21, s22), "the S matrix")
}

/// The ABCD matrix of a two-port given by its Z parameters.
pub fn z_to_abcd(z: &Mat2) -> Result<Mat2, RfError> {
    let a = z.m11.checked_div(z.m21)?;
    let b = z.det().checked_div(z.m21)?;
    let c = Complex::ONE.checked_div(z.m21)?;
    let d = z.m22.checked_div(z.m21)?;
    finite_matrix(Mat2::new(a, b, c, d), "the ABCD matrix")
}

/// The Z parameters of a two-port given by its ABCD matrix.
pub fn abcd_to_z(t: &Mat2) -> Result<Mat2, RfError> {
    let z11 = t.m11.checked_div(t.m21)?;
    let z12 = t.det().checked_div(t.m21)?;
    let z21 = Complex::ONE.checked_div(t.m21)?;
    let z22 = t.m22.checked_div(t.m21)?;
    finite_matrix(Mat2::new(z11, z12, z21, z22), "the Z matrix")
}

/// The ABCD matrix of a two-port given by its Y parameters.
pub fn y_to_abcd(y: &Mat2) -> Result<Mat2, RfError> {
    let a = (-y.m22).checked_div(y.m21)?;
    let b = (-Complex::ONE).checked_div(y.m21)?;
    let c = (-y.det()).checked_div(y.m21)?;
    let d = (-y.m11).checked_div(y.m21)?;
    finite_matrix(Mat2::new(a, b, c, d), "the ABCD matrix")
}

/// The Y parameters of a two-port given by its ABCD matrix.
pub fn abcd_to_y(t: &Mat2) -> Result<Mat2, RfError> {
    let y11 = t.m22.checked_div(t.m12)?;
    let y12 = (t.m12 * t.m21 - t.m11 * t.m22).checked_div(t.m12)?;
    let y21 = (-Complex::ONE).checked_div(t.m12)?;
    let y22 = t.m11.checked_div(t.m12)?;
    finite_matrix(Mat2::new(y11, y12, y21, y22), "the Y matrix")
}

// ---------------------------------------------------------------------------------------------
// cascade and elements

/// The cascade of two two-ports given by their ABCD matrices: the output of `first` feeds the
/// input of `second`.
pub fn cascade(first: &Mat2, second: &Mat2) -> Mat2 {
    first.times(second)
}

/// The cascade of two two-ports given by their S parameters (through the ABCD matrices; it
/// fails when a transmission is zero).
pub fn cascade_s(first: &Mat2, second: &Mat2, z0_ohm: f64) -> Result<Mat2, RfError> {
    let total = cascade(&s_to_abcd(first, z0_ohm)?, &s_to_abcd(second, z0_ohm)?);
    abcd_to_s(&total, z0_ohm)
}

/// The ABCD matrix of an impedance in series.
pub fn abcd_series(z_ohm: Complex) -> Mat2 {
    Mat2::new(Complex::ONE, z_ohm, Complex::ZERO, Complex::ONE)
}

/// The ABCD matrix of an admittance in shunt.
pub fn abcd_shunt(y_siemens: Complex) -> Mat2 {
    Mat2::new(Complex::ONE, Complex::ZERO, y_siemens, Complex::ONE)
}

/// The ABCD matrix of a line of characteristic impedance `z0_ohm` and total propagation
/// `gamma_l` (nepers plus j radians over the length).
pub fn abcd_line(z0_ohm: f64, gamma_l: Complex) -> Result<Mat2, RfError> {
    let z0 = positive("characteristic impedance", z0_ohm)?;
    let (ch, sh) = (gamma_l.cosh(), gamma_l.sinh());
    finite_matrix(
        Mat2::new(ch, sh.scale(z0), sh.scale(1.0 / z0), ch),
        "the ABCD matrix of the line",
    )
}

/// The ABCD matrix of a lossless line of electrical length `theta_rad`.
pub fn abcd_lossless_line(z0_ohm: f64, theta_rad: f64) -> Result<Mat2, RfError> {
    abcd_line(z0_ohm, Complex::new(0.0, theta_rad))
}

/// The impedance seen at the input of a two-port (given by its ABCD matrix) that ends on a load.
pub fn input_impedance(abcd: &Mat2, load_ohm: Complex) -> Result<Complex, RfError> {
    (abcd.m11 * load_ohm + abcd.m12).checked_div(abcd.m21 * load_ohm + abcd.m22)
}

// ---------------------------------------------------------------------------------------------
// figures

/// The reflection coefficient of an impedance against a real reference: `(z - z0) / (z + z0)`.
pub fn reflection_from_impedance(z_ohm: Complex, z0_ohm: f64) -> Result<Complex, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    (z_ohm - Complex::real(z0)).checked_div(z_ohm + Complex::real(z0))
}

/// The impedance of a reflection coefficient: `z0 (1 + g) / (1 - g)`.
pub fn impedance_from_reflection(gamma: Complex, z0_ohm: f64) -> Result<Complex, RfError> {
    let z0 = positive("reference impedance", z0_ohm)?;
    (Complex::ONE + gamma)
        .scale(z0)
        .checked_div(Complex::ONE - gamma)
}

/// The voltage standing wave ratio of a reflection coefficient, `(1 + |g|) / (1 - |g|)`;
/// infinity when `|g|` is 1 or more.
pub fn vswr(gamma: Complex) -> f64 {
    let m = gamma.abs();
    if m >= 1.0 {
        f64::INFINITY
    } else {
        (1.0 + m) / (1.0 - m)
    }
}

/// The return loss in dB, `-20 log10 |g|`; infinity for a perfect match.
pub fn return_loss_db(gamma: Complex) -> f64 {
    -gamma.db20()
}

/// The mismatch loss in dB, `-10 log10 (1 - |g|^2)`; infinity when `|g|` is 1 or more.
pub fn mismatch_loss_db(gamma: Complex) -> f64 {
    let p = 1.0 - gamma.norm_sqr();
    if p <= 0.0 {
        f64::INFINITY
    } else {
        -10.0 * p.log10()
    }
}

/// The insertion loss in dB of a transmission coefficient, `-20 log10 |s21|`.
pub fn insertion_loss_db(s21: Complex) -> f64 {
    -s21.db20()
}

/// The group delay in seconds between two frequencies, from the phase of the transmission at
/// each (the phase step is taken in the range minus pi to pi, so the points must be close
/// enough for the phase to move less than half a turn between them).
pub fn group_delay_s(
    s21_low: Complex,
    s21_high: Complex,
    freq_low_hz: f64,
    freq_high_hz: f64,
) -> Result<f64, RfError> {
    if !freq_low_hz.is_finite() || !freq_high_hz.is_finite() || freq_high_hz <= freq_low_hz {
        return Err(RfError::InvalidNumber {
            field: "frequency pair".to_string(),
            reason: "the second frequency must be above the first".to_string(),
        });
    }
    let step = (s21_high * s21_low.conj()).arg();
    Ok(-step / (2.0 * PI * (freq_high_hz - freq_low_hz)))
}

/// The transducer power gain (linear) of a two-port with S parameters `s` between a source of
/// reflection coefficient `gamma_s` and a load of `gamma_l`.
pub fn transducer_gain(s: &Mat2, gamma_s: Complex, gamma_l: Complex) -> Result<f64, RfError> {
    let one = Complex::ONE;
    let numerator = s.m21.norm_sqr() * (1.0 - gamma_s.norm_sqr()) * (1.0 - gamma_l.norm_sqr());
    let loop_term =
        (one - s.m11 * gamma_s) * (one - s.m22 * gamma_l) - s.m12 * s.m21 * gamma_l * gamma_s;
    let denominator = loop_term.norm_sqr();
    if denominator == 0.0 || !denominator.is_finite() {
        return Err(RfError::Singular {
            what: "the transducer gain denominator is zero".to_string(),
        });
    }
    Ok(numerator / denominator)
}

/// The maximum stable gain `|s21| / |s12|` (linear); infinity for a unilateral two-port.
pub fn max_stable_gain(s: &Mat2) -> f64 {
    let reverse = s.m12.abs();
    if reverse == 0.0 {
        f64::INFINITY
    } else {
        s.m21.abs() / reverse
    }
}

/// The maximum available gain (linear), defined only when the two-port is unconditionally
/// stable (`K > 1`); `None` otherwise.
pub fn max_available_gain(s: &Mat2) -> Option<f64> {
    let stability = stability(s);
    if stability.k <= 1.0 {
        return None;
    }
    let ratio = max_stable_gain(s);
    if ratio.is_infinite() {
        return Some(ratio);
    }
    Some(ratio * (stability.k - (stability.k * stability.k - 1.0).sqrt()))
}

/// The stability figures of a two-port.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stability {
    /// The Rollett factor `K`; infinity when the two-port is unilateral.
    pub k: f64,
    /// The determinant of the S matrix.
    pub delta: Complex,
    /// The geometric stability factor `mu` of the load side (stable when above 1).
    pub mu: f64,
    /// The geometric stability factor `mu'` of the source side (stable when above 1).
    pub mu_prime: f64,
    /// Whether the two-port is unconditionally stable (`K > 1` and `|delta| < 1`).
    pub unconditionally_stable: bool,
}

/// The stability figures of a two-port given by its S parameters.
pub fn stability(s: &Mat2) -> Stability {
    let delta = s.det();
    let product = (s.m12 * s.m21).abs();
    let numerator = 1.0 - s.m11.norm_sqr() - s.m22.norm_sqr() + delta.norm_sqr();
    let k = if product == 0.0 {
        f64::INFINITY
    } else {
        numerator / (2.0 * product)
    };
    let ratio = |top: f64, other: Complex| {
        let bottom = other.abs() + product;
        if bottom == 0.0 {
            f64::INFINITY
        } else {
            top / bottom
        }
    };
    let mu = ratio(1.0 - s.m11.norm_sqr(), s.m22 - delta * s.m11.conj());
    let mu_prime = ratio(1.0 - s.m22.norm_sqr(), s.m11 - delta * s.m22.conj());
    Stability {
        k,
        delta,
        mu,
        mu_prime,
        unconditionally_stable: k > 1.0 && delta.abs() < 1.0,
    }
}

/// The largest singular value of a 2 x 2 matrix. A passive network has at most 1.
pub fn max_singular_value(s: &Mat2) -> f64 {
    let p = s.m11.norm_sqr() + s.m21.norm_sqr();
    let r = s.m12.norm_sqr() + s.m22.norm_sqr();
    let q = s.m11.conj() * s.m12 + s.m21.conj() * s.m22;
    let half_sum = (p + r) / 2.0;
    let half_gap = (p - r) / 2.0;
    (half_sum + (half_gap * half_gap + q.norm_sqr()).sqrt()).sqrt()
}

/// Whether a two-port with S parameters `s` is passive: its largest singular value does not
/// exceed 1 by more than `tolerance`.
pub fn is_passive(s: &Mat2, tolerance: f64) -> bool {
    max_singular_value(s) <= 1.0 + tolerance
}

/// Whether a two-port is reciprocal: `|s12 - s21|` is within `tolerance`.
pub fn is_reciprocal(s: &Mat2, tolerance: f64) -> bool {
    (s.m12 - s.m21).abs() <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    /// A small deterministic generator for the property tests.
    struct Lcg(u64);

    impl Lcg {
        fn uniform(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }

        /// A passive, reciprocal S matrix with moderate entries.
        fn reciprocal(&mut self) -> Mat2 {
            let s12 = c(0.3 * self.uniform() + 0.2, 0.3 * self.uniform() - 0.15);
            Mat2::new(
                c(0.4 * self.uniform() - 0.2, 0.4 * self.uniform() - 0.2),
                s12,
                s12,
                c(0.4 * self.uniform() - 0.2, 0.4 * self.uniform() - 0.2),
            )
        }
    }

    #[test]
    fn a_through_line_has_the_expected_parameters() {
        let through = Mat2::new(Complex::ZERO, Complex::ONE, Complex::ONE, Complex::ZERO);
        let t = s_to_abcd(&through, 50.0).unwrap();
        assert!(t.max_difference(&Mat2::identity()) < 1e-15);
        let s = abcd_to_s(&Mat2::identity(), 50.0).unwrap();
        assert!(s.max_difference(&through) < 1e-15);
    }

    #[test]
    fn a_series_resistor_has_the_book_parameters() {
        // 50 ohm in series between 50 ohm ports: s11 = 1/3, s21 = 2/3
        let s = abcd_to_s(&abcd_series(Complex::real(50.0)), 50.0).unwrap();
        assert!((s.m11.re - 1.0 / 3.0).abs() < 1e-12 && s.m11.im.abs() < 1e-12);
        assert!((s.m21.re - 2.0 / 3.0).abs() < 1e-12);
        assert!((s.m12 - s.m21).abs() < 1e-12 && (s.m22 - s.m11).abs() < 1e-12);
    }

    #[test]
    fn a_shunt_resistor_has_the_book_parameters() {
        // 50 ohm in shunt between 50 ohm ports: s11 = -1/3, s21 = 2/3
        let s = abcd_to_s(&abcd_shunt(Complex::real(1.0 / 50.0)), 50.0).unwrap();
        assert!((s.m11.re + 1.0 / 3.0).abs() < 1e-12);
        assert!((s.m21.re - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn s_z_y_abcd_conversions_round_trip() {
        let mut rng = Lcg(7);
        for _ in 0..200 {
            let s = rng.reciprocal();
            let z0 = 25.0 + 100.0 * rng.uniform();
            let via_abcd = abcd_to_s(&s_to_abcd(&s, z0).unwrap(), z0).unwrap();
            let via_z = z_to_s(&s_to_z(&s, z0).unwrap(), z0).unwrap();
            let via_y = y_to_s(&s_to_y(&s, z0).unwrap(), z0).unwrap();
            for back in [via_abcd, via_z, via_y] {
                assert!(back.max_difference(&s) < 1e-9, "{s:?} {back:?}");
            }
            let z = s_to_z(&s, z0).unwrap();
            let y = s_to_y(&s, z0).unwrap();
            assert!(z_to_y(&z).unwrap().max_difference(&y) < 1e-9 * (1.0 + y.m11.abs()));
            assert!(y_to_z(&y).unwrap().max_difference(&z) < 1e-6 * (1.0 + z.m11.abs()));
            let t = s_to_abcd(&s, z0).unwrap();
            let t_from_z = z_to_abcd(&z).unwrap();
            let t_from_y = y_to_abcd(&y).unwrap();
            let scale = 1.0 + t.m12.abs() + t.m21.abs();
            assert!(t_from_z.max_difference(&t) < 1e-6 * scale);
            assert!(t_from_y.max_difference(&t) < 1e-6 * scale);
            assert!(abcd_to_z(&t).unwrap().max_difference(&z) < 1e-6 * (1.0 + z.m11.abs()));
            assert!(abcd_to_y(&t).unwrap().max_difference(&y) < 1e-6 * (1.0 + y.m11.abs()));
        }
    }

    #[test]
    fn a_reciprocal_network_has_a_unit_abcd_determinant() {
        let mut rng = Lcg(11);
        for _ in 0..100 {
            let s = rng.reciprocal();
            let t = s_to_abcd(&s, 50.0).unwrap();
            assert!((t.det() - Complex::ONE).abs() < 1e-9);
            assert!(is_reciprocal(&s, 1e-15));
        }
    }

    #[test]
    fn the_cascade_of_s_parameters_matches_the_abcd_product() {
        let mut rng = Lcg(3);
        for _ in 0..50 {
            let (a, b) = (rng.reciprocal(), rng.reciprocal());
            let by_s = cascade_s(&a, &b, 50.0).unwrap();
            let t = cascade(&s_to_abcd(&a, 50.0).unwrap(), &s_to_abcd(&b, 50.0).unwrap());
            assert!(by_s.max_difference(&abcd_to_s(&t, 50.0).unwrap()) < 1e-12);
            // a cascade of passive networks is passive
            assert!(is_passive(&by_s, 1e-9));
        }
    }

    #[test]
    fn two_half_resistors_make_one_resistor() {
        let half = abcd_series(Complex::real(25.0));
        let whole = abcd_series(Complex::real(50.0));
        assert!(cascade(&half, &half).max_difference(&whole) < 1e-15);
        let a = abcd_shunt(c(0.0, 0.01));
        let b = abcd_shunt(c(0.0, 0.02));
        assert!(cascade(&a, &b).max_difference(&abcd_shunt(c(0.0, 0.03))) < 1e-15);
    }

    #[test]
    fn a_quarter_wave_line_inverts_the_impedance() {
        // zin = z0^2 / zl for a quarter-wave line
        let line = abcd_lossless_line(70.0, PI / 2.0).unwrap();
        let zin = input_impedance(&line, Complex::real(100.0)).unwrap();
        assert!((zin.re - 4900.0 / 100.0).abs() < 1e-9 && zin.im.abs() < 1e-9);
        // a half-wave line repeats the load
        let half = abcd_lossless_line(70.0, PI).unwrap();
        let again = input_impedance(&half, c(30.0, 40.0)).unwrap();
        assert!((again - c(30.0, 40.0)).abs() < 1e-9);
        // a lossy line is passive
        let lossy = abcd_line(50.0, c(0.1, 1.0)).unwrap();
        let s = abcd_to_s(&lossy, 50.0).unwrap();
        assert!(is_passive(&s, 1e-9) && is_reciprocal(&s, 1e-12));
        assert!(s.m11.abs() < 1e-12);
    }

    #[test]
    fn reflection_and_impedance_are_inverse() {
        let g = reflection_from_impedance(c(75.0, 20.0), 50.0).unwrap();
        let z = impedance_from_reflection(g, 50.0).unwrap();
        assert!((z - c(75.0, 20.0)).abs() < 1e-9);
        assert_eq!(
            reflection_from_impedance(Complex::real(50.0), 50.0).unwrap(),
            Complex::ZERO
        );
        // short, open and a 100 ohm load
        let short = reflection_from_impedance(Complex::ZERO, 50.0).unwrap();
        assert_eq!(short, Complex::real(-1.0));
        let g100 = reflection_from_impedance(Complex::real(100.0), 50.0).unwrap();
        assert!((g100.re - 1.0 / 3.0).abs() < 1e-12);
        assert!(impedance_from_reflection(Complex::ONE, 50.0).is_err());
        assert!(reflection_from_impedance(Complex::real(-50.0), 50.0).is_err());
        assert!(reflection_from_impedance(Complex::ONE, -1.0).is_err());
    }

    #[test]
    fn vswr_and_losses_have_book_values() {
        let g = Complex::real(1.0 / 3.0);
        assert!((vswr(g) - 2.0).abs() < 1e-12);
        assert!((return_loss_db(g) - 9.5424250943932).abs() < 1e-9);
        assert!((mismatch_loss_db(g) - 0.5115252244738).abs() < 1e-9);
        assert_eq!(vswr(Complex::ONE), f64::INFINITY);
        assert_eq!(vswr(Complex::ZERO), 1.0);
        assert_eq!(return_loss_db(Complex::ZERO), f64::INFINITY);
        assert_eq!(mismatch_loss_db(c(0.0, 1.0)), f64::INFINITY);
        assert!((insertion_loss_db(Complex::real(0.5)) - 6.0205999132796).abs() < 1e-9);
        assert_eq!(insertion_loss_db(Complex::ONE), 0.0);
    }

    #[test]
    fn the_group_delay_of_a_delay_line_is_its_delay() {
        let delay = 2.5e-9;
        let (f1, f2) = (1.0e9, 1.01e9);
        let at = |f: f64| Complex::from_polar(1.0, -2.0 * PI * f * delay);
        let tau = group_delay_s(at(f1), at(f2), f1, f2).unwrap();
        assert!((tau - delay).abs() < 1e-15);
        assert!(group_delay_s(at(f1), at(f2), f2, f1).is_err());
        assert!(group_delay_s(at(f1), at(f2), f1, f1).is_err());
        assert!(group_delay_s(at(f1), at(f2), f1, f64::NAN).is_err());
    }

    #[test]
    fn the_gain_of_a_matched_amplifier_is_s21_squared() {
        let s = Mat2::new(Complex::ZERO, Complex::ZERO, c(4.0, 0.0), Complex::ZERO);
        let gain = transducer_gain(&s, Complex::ZERO, Complex::ZERO).unwrap();
        assert!((gain - 16.0).abs() < 1e-12);
        assert_eq!(max_stable_gain(&s), f64::INFINITY);
        assert_eq!(max_available_gain(&s), Some(f64::INFINITY));
        // a feedback loop that closes on itself gives a singular denominator
        let loopy = Mat2::new(Complex::ZERO, Complex::ONE, Complex::ONE, Complex::ZERO);
        assert!(transducer_gain(&loopy, Complex::ONE, Complex::ONE).is_err());
    }

    #[test]
    fn the_stability_factors_agree() {
        let stable = Mat2::new(c(0.2, 0.1), c(0.05, 0.0), c(3.0, 0.5), c(0.3, -0.1));
        let st = stability(&stable);
        assert!(st.k > 1.0 && st.delta.abs() < 1.0 && st.unconditionally_stable);
        assert!(st.mu > 1.0 && st.mu_prime > 1.0);
        let mag = max_available_gain(&stable).unwrap();
        assert!(mag > 0.0 && mag < max_stable_gain(&stable));
        let unstable = Mat2::new(c(0.9, 0.3), c(0.2, 0.0), c(3.0, 0.0), c(0.8, -0.4));
        let un = stability(&unstable);
        assert!(!un.unconditionally_stable && un.mu < 1.0 && un.mu_prime < 1.0);
        assert!(un.k < 1.0);
        assert_eq!(max_available_gain(&unstable), None);
        // the two criteria give the same answer on a spread of amplifiers
        let mut rng = Lcg(21);
        for _ in 0..200 {
            let s = Mat2::new(
                c(rng.uniform() - 0.5, rng.uniform() - 0.5),
                c(0.3 * rng.uniform(), 0.0),
                c(4.0 * rng.uniform(), rng.uniform()),
                c(rng.uniform() - 0.5, rng.uniform() - 0.5),
            );
            let st = stability(&s);
            assert_eq!(st.unconditionally_stable, st.mu > 1.0, "{s:?}");
        }
    }

    #[test]
    fn passivity_is_judged_by_the_singular_value() {
        let matched_loss = Mat2::new(Complex::ZERO, c(0.5, 0.0), c(0.5, 0.0), Complex::ZERO);
        assert!((max_singular_value(&matched_loss) - 0.5).abs() < 1e-12);
        assert!(is_passive(&matched_loss, 1e-9));
        let gain = Mat2::new(Complex::ZERO, Complex::ZERO, c(2.0, 0.0), Complex::ZERO);
        assert!((max_singular_value(&gain) - 2.0).abs() < 1e-12);
        assert!(!is_passive(&gain, 1e-6));
        assert!(!is_reciprocal(&gain, 1e-6));
        // a lossless reflection keeps the value 1
        let mirror = Mat2::new(
            Complex::real(-1.0),
            Complex::ZERO,
            Complex::ZERO,
            Complex::ONE,
        );
        assert!((max_singular_value(&mirror) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_references_and_singular_conversions_are_errors() {
        let s = Mat2::identity();
        assert!(matches!(
            s_to_z(&s, 0.0),
            Err(RfError::InvalidNumber { .. })
        ));
        assert!(s_to_abcd(&s, f64::NAN).is_err() && abcd_to_s(&s, -1.0).is_err());
        // s21 = 0 has no ABCD matrix
        let blocked = Mat2::new(Complex::ZERO, Complex::ZERO, Complex::ZERO, Complex::ZERO);
        assert!(matches!(
            s_to_abcd(&blocked, 50.0),
            Err(RfError::Singular { .. })
        ));
        // a series impedance has no Z matrix (C = 0)
        assert!(abcd_to_z(&abcd_series(Complex::real(10.0))).is_err());
        // a shunt admittance has no Y matrix (B = 0)
        assert!(abcd_to_y(&abcd_shunt(Complex::real(0.1))).is_err());
        assert!(abcd_line(0.0, Complex::ONE).is_err());
        assert!(Mat2::from_cmatrix(&CMatrix::identity(3)).is_err());
    }
}
