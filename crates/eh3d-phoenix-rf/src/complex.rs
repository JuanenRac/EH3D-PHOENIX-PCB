// SPDX-License-Identifier: GPL-3.0-or-later
//! A complex number of two 64-bit floats, with the operations the RF tools need. It is a type of
//! this crate and not a dependency. Division by zero gives a non-finite number through the
//! operator and a typed error through [`Complex::checked_div`]; nothing here panics.

use crate::error::RfError;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A complex number `re + j im`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    /// The real part.
    pub re: f64,
    /// The imaginary part.
    pub im: f64,
}

impl Complex {
    /// Zero.
    pub const ZERO: Complex = Complex { re: 0.0, im: 0.0 };
    /// One.
    pub const ONE: Complex = Complex { re: 1.0, im: 0.0 };
    /// The imaginary unit.
    pub const J: Complex = Complex { re: 0.0, im: 1.0 };

    /// A number from its real and imaginary parts.
    pub const fn new(re: f64, im: f64) -> Complex {
        Complex { re, im }
    }

    /// A number with no imaginary part.
    pub const fn real(re: f64) -> Complex {
        Complex { re, im: 0.0 }
    }

    /// A number from its magnitude and its angle in radians.
    pub fn from_polar(magnitude: f64, angle_rad: f64) -> Complex {
        Complex {
            re: magnitude * angle_rad.cos(),
            im: magnitude * angle_rad.sin(),
        }
    }

    /// A number from a magnitude in dB (of a voltage ratio, `20 log10`) and an angle in degrees.
    pub fn from_db_deg(magnitude_db: f64, angle_deg: f64) -> Complex {
        Complex::from_polar(10f64.powf(magnitude_db / 20.0), angle_deg.to_radians())
    }

    /// The magnitude.
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The square of the magnitude.
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// The angle in radians, in the range minus pi to pi.
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    /// The angle in degrees, in the range minus 180 to 180.
    pub fn arg_deg(self) -> f64 {
        self.arg().to_degrees()
    }

    /// The magnitude in dB (`20 log10`); minus infinity for zero.
    pub fn db20(self) -> f64 {
        20.0 * self.abs().log10()
    }

    /// The complex conjugate.
    pub fn conj(self) -> Complex {
        Complex {
            re: self.re,
            im: -self.im,
        }
    }

    /// The number multiplied by a real factor.
    pub fn scale(self, factor: f64) -> Complex {
        Complex {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    /// Whether both parts are finite.
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// The exponential.
    pub fn exp(self) -> Complex {
        Complex::from_polar(self.re.exp(), self.im)
    }

    /// The principal square root.
    pub fn sqrt(self) -> Complex {
        let r = self.abs();
        let re = ((r + self.re) / 2.0).max(0.0).sqrt();
        let im = ((r - self.re) / 2.0).max(0.0).sqrt().copysign(self.im);
        Complex { re, im }
    }

    /// The hyperbolic cosine.
    pub fn cosh(self) -> Complex {
        Complex {
            re: self.re.cosh() * self.im.cos(),
            im: self.re.sinh() * self.im.sin(),
        }
    }

    /// The hyperbolic sine.
    pub fn sinh(self) -> Complex {
        Complex {
            re: self.re.sinh() * self.im.cos(),
            im: self.re.cosh() * self.im.sin(),
        }
    }

    /// The quotient, or [`RfError::Singular`] when the divisor is zero or the result is not
    /// finite.
    pub fn checked_div(self, divisor: Complex) -> Result<Complex, RfError> {
        let d = divisor.norm_sqr();
        if d == 0.0 || !d.is_finite() {
            return Err(RfError::Singular {
                what: "division by zero".to_string(),
            });
        }
        let q = Complex {
            re: (self.re * divisor.re + self.im * divisor.im) / d,
            im: (self.im * divisor.re - self.re * divisor.im) / d,
        };
        if q.is_finite() {
            Ok(q)
        } else {
            Err(RfError::Singular {
                what: "the quotient is not finite".to_string(),
            })
        }
    }

    /// The inverse `1 / z`, or [`RfError::Singular`] for zero.
    pub fn recip(self) -> Result<Complex, RfError> {
        Complex::ONE.checked_div(self)
    }

    /// Read a number written as `a`, `a+jb`, `a-jb`, `jb` or `-jb` (no spaces; `i` may stand for
    /// `j`). The parts must be finite.
    pub fn parse(text: &str) -> Result<Complex, RfError> {
        let bad = |reason: &str| RfError::InvalidNumber {
            field: format!("complex number {}", clip(text)),
            reason: reason.to_string(),
        };
        let t = text.trim();
        if t.is_empty() {
            return Err(bad("empty"));
        }
        let bytes = t.as_bytes();
        let split = (1..bytes.len()).rev().find(|&i| {
            matches!(bytes.get(i), Some(b'+') | Some(b'-'))
                && !matches!(bytes.get(i - 1), Some(b'e') | Some(b'E'))
        });
        let imaginary = |part: &str| -> Result<f64, RfError> {
            let (sign, rest) = match part.strip_prefix('-') {
                Some(rest) => (-1.0, rest),
                None => (1.0, part.strip_prefix('+').unwrap_or(part)),
            };
            let digits = rest
                .strip_prefix('j')
                .or_else(|| rest.strip_prefix('i'))
                .ok_or_else(|| bad("the imaginary part must start with j"))?;
            Ok(sign * finite(digits).ok_or_else(|| bad("not a finite number"))?)
        };
        let starts_imaginary = |part: &str| {
            let rest = part.trim_start_matches(['+', '-']);
            rest.starts_with('j') || rest.starts_with('i')
        };
        match split {
            Some(i) => {
                let (re_text, im_text) = t.split_at(i);
                let re = finite(re_text).ok_or_else(|| bad("not a finite number"))?;
                Ok(Complex::new(re, imaginary(im_text)?))
            }
            None if starts_imaginary(t) => Ok(Complex::new(0.0, imaginary(t)?)),
            None => Ok(Complex::real(
                finite(t).ok_or_else(|| bad("not a finite number"))?,
            )),
        }
    }
}

fn finite(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// At most 40 characters of a text, to put in a message.
pub(crate) fn clip(text: &str) -> String {
    text.chars().take(40).collect()
}

impl From<f64> for Complex {
    fn from(re: f64) -> Complex {
        Complex::real(re)
    }
}

impl Add for Complex {
    type Output = Complex;
    fn add(self, other: Complex) -> Complex {
        Complex {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl Sub for Complex {
    type Output = Complex;
    fn sub(self, other: Complex) -> Complex {
        Complex {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
}

impl Mul for Complex {
    type Output = Complex;
    fn mul(self, other: Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

impl Div for Complex {
    type Output = Complex;
    fn div(self, other: Complex) -> Complex {
        let d = other.norm_sqr();
        Complex {
            re: (self.re * other.re + self.im * other.im) / d,
            im: (self.im * other.re - self.re * other.im) / d,
        }
    }
}

impl Neg for Complex {
    type Output = Complex;
    fn neg(self) -> Complex {
        Complex {
            re: -self.re,
            im: -self.im,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Complex, b: Complex) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn arithmetic_matches_the_definitions() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert_eq!(a + b, Complex::new(4.0, 2.0));
        assert_eq!(a - b, Complex::new(2.0, 6.0));
        assert_eq!(a * b, Complex::new(11.0, -2.0));
        assert!(close((a * b) / b, a));
        assert_eq!(-a, Complex::new(-3.0, -4.0));
        assert_eq!(a.conj(), Complex::new(3.0, -4.0));
        assert_eq!(a.abs(), 5.0);
        assert_eq!(a.norm_sqr(), 25.0);
        assert_eq!(Complex::J * Complex::J, Complex::real(-1.0));
        assert_eq!(Complex::from(2.0), Complex::real(2.0));
        assert_eq!(a.scale(2.0), Complex::new(6.0, 8.0));
    }

    #[test]
    fn polar_forms_round_trip() {
        let z = Complex::from_polar(2.0, 0.75);
        assert!((z.abs() - 2.0).abs() < 1e-12 && (z.arg() - 0.75).abs() < 1e-12);
        let w = Complex::from_db_deg(-6.0, 90.0);
        assert!((w.abs() - 10f64.powf(-0.3)).abs() < 1e-12);
        assert!((w.arg_deg() - 90.0).abs() < 1e-9);
        assert!((w.db20() + 6.0).abs() < 1e-9);
        assert_eq!(Complex::ZERO.db20(), f64::NEG_INFINITY);
    }

    #[test]
    fn square_root_and_exponential() {
        for z in [
            Complex::new(-4.0, 0.0),
            Complex::new(3.0, 4.0),
            Complex::new(-3.0, -4.0),
            Complex::new(0.0, 2.0),
        ] {
            let r = z.sqrt();
            assert!(close(r * r, z), "{z:?}");
        }
        assert!(close(
            Complex::new(0.0, 3.0).exp().conj(),
            Complex::new(0.0, -3.0).exp()
        ));
        assert!(close(
            Complex::new(0.0, std::f64::consts::PI).exp(),
            Complex::real(-1.0)
        ));
    }

    #[test]
    fn hyperbolic_functions_satisfy_the_identity() {
        let z = Complex::new(0.3, 1.1);
        let c = z.cosh();
        let s = z.sinh();
        assert!(close(c * c - s * s, Complex::ONE));
        assert!(close(
            Complex::new(0.0, 1.0).cosh(),
            Complex::real(1.0f64.cos())
        ));
    }

    #[test]
    fn division_by_zero_is_a_typed_error() {
        assert!(matches!(
            Complex::ONE.checked_div(Complex::ZERO),
            Err(RfError::Singular { .. })
        ));
        assert!(matches!(
            Complex::ZERO.recip(),
            Err(RfError::Singular { .. })
        ));
        assert!(!(Complex::ONE / Complex::ZERO).is_finite());
        assert_eq!(
            Complex::new(0.0, 1.0).recip().unwrap(),
            Complex::new(0.0, -1.0)
        );
    }

    #[test]
    fn numbers_are_parsed_from_text() {
        for (text, want) in [
            ("50", Complex::new(50.0, 0.0)),
            ("-12.5", Complex::new(-12.5, 0.0)),
            ("50+j25", Complex::new(50.0, 25.0)),
            ("50-j25", Complex::new(50.0, -25.0)),
            ("j10", Complex::new(0.0, 10.0)),
            ("-j10", Complex::new(0.0, -10.0)),
            ("1e-3+j2e-3", Complex::new(1e-3, 2e-3)),
            ("1e-3-j2e-3", Complex::new(1e-3, -2e-3)),
            ("1e2", Complex::new(100.0, 0.0)),
            (" 3+i4 ", Complex::new(3.0, 4.0)),
        ] {
            assert_eq!(Complex::parse(text).unwrap(), want, "{text}");
        }
    }

    #[test]
    fn bad_numbers_are_refused() {
        for text in [
            "", " ", "abc", "50+25", "50+j", "50+jx", "inf", "nan", "1+jinf", "j",
        ] {
            assert!(
                matches!(Complex::parse(text), Err(RfError::InvalidNumber { .. })),
                "{text}"
            );
        }
    }
}
