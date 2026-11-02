//! Lengths in nanometres and angles in micro-degrees, both integers with a checked range.
//!
//! The limits are named constants. Coordinates stay strictly inside `+-2^30` nm (about 1.07 m):
//! then every difference of two coordinates is below `2^31`, every product of two differences is
//! below `2^62` and a 2x2 cross product is below `2^63`, so the orientation predicates of
//! [`crate::geometry`] are plain `i64` arithmetic that cannot overflow.

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// Coordinates are strictly inside `+-COORD_LIMIT_NM` nanometres.
pub const COORD_LIMIT_NM: i64 = 1 << 30;
/// Angles (rotations and sweeps) are inside `+-ANGLE_LIMIT_UDEG` micro-degrees: one turn.
pub const ANGLE_LIMIT_UDEG: i64 = 360_000_000;
/// Nanometres in a millimetre.
pub const NM_PER_MM: i64 = 1_000_000;
/// Nanometres in a mil (a thousandth of an inch), exactly.
pub const NM_PER_MIL: i64 = 25_400;
/// Nanometres in an inch, exactly.
pub const NM_PER_INCH: i64 = 25_400_000;
/// Longest decimal text accepted by [`ScaledDecimal::parse`].
const MAX_DECIMAL_CHARS: usize = 64;

/// A length or coordinate in nanometres, always inside the design range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Nm(i64);

impl Nm {
    /// Zero.
    pub const ZERO: Nm = Nm(0);
    /// The largest value of the range.
    pub const MAX: Nm = Nm(COORD_LIMIT_NM - 1);
    /// The smallest value of the range.
    pub const MIN: Nm = Nm(-(COORD_LIMIT_NM - 1));

    /// Checks the range and wraps the value.
    pub fn new(value: i64) -> Result<Nm, ModelError> {
        if value > -COORD_LIMIT_NM && value < COORD_LIMIT_NM {
            Ok(Nm(value))
        } else {
            Err(ModelError::CoordinateOutOfRange {
                value: i128::from(value),
                limit: COORD_LIMIT_NM,
            })
        }
    }

    /// Like [`Nm::new`] from a wider integer, as produced by constructions done in `i128`.
    pub fn from_i128(value: i128) -> Result<Nm, ModelError> {
        match i64::try_from(value) {
            Ok(v) => Nm::new(v),
            Err(_) => Err(ModelError::CoordinateOutOfRange {
                value,
                limit: COORD_LIMIT_NM,
            }),
        }
    }

    /// The value in nanometres.
    pub fn get(self) -> i64 {
        self.0
    }

    /// A whole number of mils (1 mil = 25 400 nm exactly).
    pub fn from_mil(mils: i64) -> Result<Nm, ModelError> {
        Nm::new(mils.checked_mul(NM_PER_MIL).ok_or(ModelError::Overflow {
            operation: "mil to nm",
        })?)
    }

    /// A whole number of inches (1 inch = 25 400 000 nm exactly).
    pub fn from_inch(inches: i64) -> Result<Nm, ModelError> {
        Nm::new(
            inches
                .checked_mul(NM_PER_INCH)
                .ok_or(ModelError::Overflow {
                    operation: "inch to nm",
                })?,
        )
    }

    /// A whole number of millimetres.
    pub fn from_mm(mm: i64) -> Result<Nm, ModelError> {
        Nm::new(mm.checked_mul(NM_PER_MM).ok_or(ModelError::Overflow {
            operation: "mm to nm",
        })?)
    }

    /// Reads a decimal text in millimetres. The second value is false when the text had more
    /// than six decimals and was rounded half to even (the caller reports it and keeps the text).
    pub fn parse_mm(text: &str) -> Result<(Nm, bool), ModelError> {
        let d = ScaledDecimal::parse(text)?;
        Ok((Nm::from_i128(d.scaled)?, d.exact))
    }

    /// The value in millimetres. Exact for the digits: six decimals at most.
    pub fn to_mm(self) -> f64 {
        self.0 as f64 / NM_PER_MM as f64
    }

    /// Sum, checked against the range.
    pub fn checked_add(self, other: Nm) -> Result<Nm, ModelError> {
        Nm::new(self.0 + other.0)
    }

    /// Difference, checked against the range.
    pub fn checked_sub(self, other: Nm) -> Result<Nm, ModelError> {
        Nm::new(self.0 - other.0)
    }

    /// `self - other` as a plain integer. The range guarantees `|result| < 2^31`.
    pub fn delta(self, other: Nm) -> i64 {
        self.0 - other.0
    }
}

impl TryFrom<i64> for Nm {
    type Error = ModelError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Nm::new(value)
    }
}

impl From<Nm> for i64 {
    fn from(value: Nm) -> i64 {
        value.0
    }
}

/// An angle in micro-degrees, inside one turn either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Udeg(i64);

impl Udeg {
    /// Zero.
    pub const ZERO: Udeg = Udeg(0);
    /// A full turn.
    pub const FULL_TURN: Udeg = Udeg(ANGLE_LIMIT_UDEG);

    /// Checks the range and wraps the value.
    pub fn new(value: i64) -> Result<Udeg, ModelError> {
        if value.unsigned_abs() <= ANGLE_LIMIT_UDEG as u64 {
            Ok(Udeg(value))
        } else {
            Err(ModelError::AngleOutOfRange {
                value: i128::from(value),
                limit: ANGLE_LIMIT_UDEG,
            })
        }
    }

    /// The value in micro-degrees.
    pub fn get(self) -> i64 {
        self.0
    }

    /// The same direction in `[0, 360)` degrees.
    pub fn normalised(self) -> Udeg {
        Udeg(self.0.rem_euclid(ANGLE_LIMIT_UDEG))
    }

    /// Reads a decimal text in degrees (for example `45.5`). The second value is false when the
    /// text was rounded half to even.
    pub fn parse_deg(text: &str) -> Result<(Udeg, bool), ModelError> {
        let d = ScaledDecimal::parse(text)?;
        let v = i64::try_from(d.scaled).map_err(|_| ModelError::AngleOutOfRange {
            value: d.scaled,
            limit: ANGLE_LIMIT_UDEG,
        })?;
        Ok((Udeg::new(v)?, d.exact))
    }
}

impl TryFrom<i64> for Udeg {
    type Error = ModelError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Udeg::new(value)
    }
}

impl From<Udeg> for i64 {
    fn from(value: Udeg) -> i64 {
        value.0
    }
}

/// A decimal text multiplied by 10^6 and rounded half to even (millimetres to nanometres, degrees
/// to micro-degrees).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledDecimal {
    /// The value times 10^6.
    pub scaled: i128,
    /// False when digits beyond the sixth decimal were rounded away.
    pub exact: bool,
}

impl ScaledDecimal {
    /// Parses `[+-]digits[.digits]`. No exponent, no spaces, at least one digit.
    pub fn parse(text: &str) -> Result<ScaledDecimal, ModelError> {
        let bad = |reason| ModelError::InvalidDecimal { reason };
        if text.len() > MAX_DECIMAL_CHARS {
            return Err(bad("too long"));
        }
        let (negative, body) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(bad("no digits"));
        }
        if !int_part.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(bad("unexpected character"));
        }
        let mut value: i128 = 0;
        for b in int_part.bytes() {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(i128::from(b - b'0')))
                .ok_or(ModelError::Overflow {
                    operation: "decimal integer part",
                })?;
        }
        let frac = frac_part.as_bytes();
        for i in 0..6 {
            let digit = frac.get(i).map_or(0, |b| i128::from(b - b'0'));
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
                .ok_or(ModelError::Overflow {
                    operation: "decimal scaling",
                })?;
        }
        let rest = frac.get(6..).unwrap_or(&[]);
        let exact = rest.iter().all(|&b| b == b'0');
        if !exact {
            // Half to even on the digits that follow the sixth decimal.
            let first = rest[0] - b'0';
            let tail_nonzero = rest[1..].iter().any(|&b| b != b'0');
            let round_up = match first.cmp(&5) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => tail_nonzero || value % 2 == 1,
            };
            if round_up {
                value += 1;
            }
        }
        Ok(ScaledDecimal {
            scaled: if negative { -value } else { value },
            exact,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_is_strictly_inside_two_to_the_30() {
        assert!(Nm::new(COORD_LIMIT_NM - 1).is_ok());
        assert!(Nm::new(-(COORD_LIMIT_NM - 1)).is_ok());
        assert_eq!(
            Nm::new(COORD_LIMIT_NM),
            Err(ModelError::CoordinateOutOfRange {
                value: i128::from(COORD_LIMIT_NM),
                limit: COORD_LIMIT_NM
            })
        );
        assert!(Nm::new(-COORD_LIMIT_NM).is_err());
        assert!(Nm::new(i64::MAX).is_err());
        assert!(Nm::new(i64::MIN).is_err());
        assert!(Nm::from_i128(i128::MAX).is_err());
        assert_eq!(Nm::MAX.get(), 1_073_741_823);
    }

    #[test]
    fn arithmetic_is_checked_against_the_range() {
        assert!(Nm::MAX.checked_add(Nm::new(1).unwrap()).is_err());
        assert!(Nm::MIN.checked_sub(Nm::new(1).unwrap()).is_err());
        assert_eq!(Nm::MAX.checked_sub(Nm::MAX).unwrap(), Nm::ZERO);
        assert_eq!(Nm::MAX.delta(Nm::MIN), 2 * (COORD_LIMIT_NM - 1));
        assert!(Nm::MAX.delta(Nm::MIN) < 1 << 31);
    }

    #[test]
    fn exact_unit_conversions() {
        assert_eq!(Nm::from_mil(1).unwrap().get(), 25_400);
        assert_eq!(Nm::from_inch(1).unwrap().get(), 25_400_000);
        assert_eq!(Nm::from_inch(1).unwrap(), Nm::from_mil(1000).unwrap());
        assert_eq!(Nm::from_mm(3).unwrap().get(), 3_000_000);
        assert!(
            Nm::from_inch(100).is_err(),
            "2.54 m is outside the board range"
        );
        assert!(Nm::from_mil(i64::MAX).is_err());
    }

    #[test]
    fn decimals_are_scaled_exactly_when_they_can_be() {
        assert_eq!(
            Nm::parse_mm("12.4").unwrap(),
            (Nm::new(12_400_000).unwrap(), true)
        );
        assert_eq!(
            Nm::parse_mm("-0.000001").unwrap(),
            (Nm::new(-1).unwrap(), true)
        );
        assert_eq!(
            Nm::parse_mm("+7").unwrap(),
            (Nm::new(7_000_000).unwrap(), true)
        );
        assert_eq!(Nm::parse_mm(".5").unwrap().0.get(), 500_000);
        assert_eq!(Nm::parse_mm("5.").unwrap().0.get(), 5_000_000);
        assert_eq!(
            Nm::parse_mm("1.2500000").unwrap(),
            (Nm::new(1_250_000).unwrap(), true)
        );
    }

    #[test]
    fn inexact_decimals_round_half_to_even_and_say_so() {
        // 0.0000005 mm is exactly half a nanometre: ties go to the even neighbour.
        assert_eq!(
            Nm::parse_mm("0.0000005").unwrap(),
            (Nm::new(0).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("0.0000015").unwrap(),
            (Nm::new(2).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("0.0000025").unwrap(),
            (Nm::new(2).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("0.00000251").unwrap(),
            (Nm::new(3).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("0.0000005000001").unwrap(),
            (Nm::new(1).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("-0.0000015").unwrap(),
            (Nm::new(-2).unwrap(), false)
        );
        assert_eq!(
            Nm::parse_mm("0.0000004").unwrap(),
            (Nm::new(0).unwrap(), false)
        );
    }

    #[test]
    fn bad_decimal_text_is_refused() {
        for text in [
            "", "-", ".", "1e3", " 1", "1 ", "1,5", "--1", "0x10", "1.2.3", "１",
        ] {
            assert!(
                matches!(
                    ScaledDecimal::parse(text),
                    Err(ModelError::InvalidDecimal { .. })
                ),
                "{text:?}"
            );
        }
        let long = "9".repeat(100);
        assert!(ScaledDecimal::parse(&long).is_err());
        // Fits the text limit but not the range.
        assert!(Nm::parse_mm("99999999999999999999999999").is_err());
        assert!(Nm::parse_mm("1074").is_err());
        assert!(Nm::parse_mm("1073.741823").is_ok());
        assert!(Nm::parse_mm("1073.741824").is_err());
    }

    #[test]
    fn angles_stay_inside_one_turn() {
        assert!(Udeg::new(360_000_000).is_ok());
        assert!(Udeg::new(-360_000_000).is_ok());
        assert!(Udeg::new(360_000_001).is_err());
        assert!(Udeg::new(i64::MIN).is_err());
        assert_eq!(
            Udeg::new(-90_000_000).unwrap().normalised().get(),
            270_000_000
        );
        assert_eq!(Udeg::FULL_TURN.normalised(), Udeg::ZERO);
        assert_eq!(
            Udeg::new(45_500_000).unwrap().normalised().get(),
            45_500_000
        );
    }

    #[test]
    fn angle_decimals_convert_exactly() {
        assert_eq!(
            Udeg::parse_deg("45.5").unwrap(),
            (Udeg::new(45_500_000).unwrap(), true)
        );
        assert_eq!(Udeg::parse_deg("-0.000001").unwrap().0.get(), -1);
        assert_eq!(
            Udeg::parse_deg("90.0000004").unwrap(),
            (Udeg::new(90_000_000).unwrap(), false)
        );
        assert!(Udeg::parse_deg("360.000001").is_err());
        assert!(Udeg::parse_deg("999999999999999999999").is_err());
    }

    #[test]
    fn serde_checks_the_range_on_input() {
        assert_eq!(serde_json::to_string(&Nm::new(-5).unwrap()).unwrap(), "-5");
        assert_eq!(serde_json::from_str::<Nm>("42").unwrap().get(), 42);
        assert!(serde_json::from_str::<Nm>("1073741824").is_err());
        assert!(serde_json::from_str::<Nm>("1.5").is_err());
        assert!(serde_json::from_str::<Udeg>("360000001").is_err());
        assert_eq!(
            serde_json::from_str::<Udeg>("-360000000").unwrap().get(),
            -360_000_000
        );
    }
}
