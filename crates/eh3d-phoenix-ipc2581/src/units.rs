// SPDX-License-Identifier: GPL-3.0-or-later
//! The units of an IPC-2581 file and the exact conversion of a decimal text to nanometres.
//!
//! A length in the file is a decimal number written in the unit of its section (`MILLIMETER`,
//! `MICRON`, `INCH`, and `MIL` or `MILS` in the properties). The conversion works on the digits
//! of the text with integer arithmetic, so that `0.0254` inches is exactly 645 160 nm and a value
//! that has more digits than a nanometre is rounded half away from zero, once.

/// A unit of length of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Millimetres (`MILLIMETER`, `MM`).
    Millimeter,
    /// Micrometres (`MICRON`).
    Micron,
    /// Inches (`INCH`).
    Inch,
    /// Thousandths of an inch (`MIL`, `MILS`).
    Mil,
}

impl Unit {
    /// The unit of a name written in a file, in any case; `None` for a name that is not one.
    pub fn parse(text: &str) -> Option<Unit> {
        let t = text.trim();
        let is = |name: &str| t.eq_ignore_ascii_case(name);
        if is("MILLIMETER") || is("MILLIMETRE") || is("MM") {
            Some(Unit::Millimeter)
        } else if is("MICRON") || is("MICROMETER") || is("UM") {
            Some(Unit::Micron)
        } else if is("INCH") || is("IN") {
            Some(Unit::Inch)
        } else if is("MIL") || is("MILS") {
            Some(Unit::Mil)
        } else {
            None
        }
    }

    /// The name used in the reports.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Millimeter => "MILLIMETER",
            Unit::Micron => "MICRON",
            Unit::Inch => "INCH",
            Unit::Mil => "MIL",
        }
    }

    /// Nanometres in one unit.
    pub fn nm_per_unit(self) -> i64 {
        match self {
            Unit::Millimeter => 1_000_000,
            Unit::Micron => 1_000,
            Unit::Inch => 25_400_000,
            Unit::Mil => 25_400,
        }
    }
}

/// The most digits of a number that are looked at; a text with more is refused.
const MAX_DIGITS: usize = 30;

/// The decimal `text` in `unit` as whole nanometres, rounded half away from zero; `None` for a text
/// that is not a plain decimal number (a sign, digits, a point, an exponent), for one that
/// has more than 30 digits and for one that does not fit in 64 bits.
pub fn to_nm(text: &str, unit: Unit) -> Option<i64> {
    let (mantissa, scale) = decimal(text)?;
    scaled(mantissa, scale, i128::from(unit.nm_per_unit()))
}

/// A decimal number as an integer mantissa and the number of digits after the point: the value is
/// the mantissa divided by 10 to the scale (the scale is negative for a large exponent).
fn decimal(text: &str) -> Option<(i128, i32)> {
    let b = text.trim().as_bytes();
    let mut i = 0;
    let negative = match b.first() {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let mut mantissa: i128 = 0;
    let mut digits = 0usize;
    let mut fraction = 0i32;
    let mut seen_point = false;
    let mut any = false;
    while i < b.len() {
        match b[i] {
            c @ b'0'..=b'9' => {
                any = true;
                digits += 1;
                if digits > MAX_DIGITS {
                    return None;
                }
                mantissa = mantissa * 10 + i128::from(c - b'0');
                if seen_point {
                    fraction += 1;
                }
            }
            b'.' if !seen_point => seen_point = true,
            b'e' | b'E' => break,
            _ => return None,
        }
        i += 1;
    }
    if !any {
        return None;
    }
    let mut exponent = 0i32;
    if i < b.len() {
        // b[i] is the e of an exponent
        i += 1;
        let negative_exp = match b.get(i) {
            Some(b'-') => {
                i += 1;
                true
            }
            Some(b'+') => {
                i += 1;
                false
            }
            _ => false,
        };
        let start = i;
        while i < b.len() {
            if !b[i].is_ascii_digit() || i - start >= 4 {
                return None;
            }
            exponent = exponent * 10 + i32::from(b[i] - b'0');
            i += 1;
        }
        if start == i {
            return None;
        }
        if negative_exp {
            exponent = -exponent;
        }
    }
    let scale = fraction - exponent;
    Some((if negative { -mantissa } else { mantissa }, scale))
}

fn scaled(mantissa: i128, scale: i32, factor: i128) -> Option<i64> {
    if mantissa == 0 {
        return Some(0);
    }
    let mut value = mantissa.checked_mul(factor)?;
    if scale <= 0 {
        for _ in 0..scale.unsigned_abs().min(60) {
            value = value.checked_mul(10)?;
        }
        return i64::try_from(value).ok();
    }
    if scale > 60 {
        return Some(0);
    }
    let mut divisor: i128 = 1;
    for _ in 0..scale {
        divisor = divisor.checked_mul(10)?;
    }
    let quotient = value / divisor;
    let remainder = value % divisor;
    let rounded = if remainder.abs() * 2 >= divisor {
        quotient + value.signum()
    } else {
        quotient
    };
    i64::try_from(rounded).ok()
}

/// A plain real number of a file as it is written (a dielectric constant, an angle), finite or
/// `None`.
pub fn to_f64(text: &str) -> Option<f64> {
    let v: f64 = text.trim().parse().ok()?;
    v.is_finite().then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_units_are_read_in_any_case() {
        assert_eq!(Unit::parse("MILLIMETER"), Some(Unit::Millimeter));
        assert_eq!(Unit::parse(" inch "), Some(Unit::Inch));
        assert_eq!(Unit::parse("Micron"), Some(Unit::Micron));
        assert_eq!(Unit::parse("MILS"), Some(Unit::Mil));
        assert_eq!(Unit::parse("mm"), Some(Unit::Millimeter));
        assert_eq!(Unit::parse("furlong"), None);
        assert_eq!(Unit::parse(""), None);
    }

    #[test]
    fn the_conversion_is_exact_for_each_unit() {
        assert_eq!(to_nm("1", Unit::Millimeter), Some(1_000_000));
        assert_eq!(to_nm("0.0254", Unit::Inch), Some(645_160));
        assert_eq!(to_nm("0.063200", Unit::Inch), Some(1_605_280));
        assert_eq!(to_nm("35", Unit::Micron), Some(35_000));
        assert_eq!(to_nm("1.37", Unit::Mil), Some(34_798));
        assert_eq!(to_nm("-0.5", Unit::Millimeter), Some(-500_000));
        assert_eq!(to_nm("+2.", Unit::Millimeter), Some(2_000_000));
        assert_eq!(to_nm(".25", Unit::Millimeter), Some(250_000));
        assert_eq!(to_nm("1e-3", Unit::Millimeter), Some(1_000));
        assert_eq!(to_nm("2.5E2", Unit::Micron), Some(250_000));
    }

    #[test]
    fn a_value_finer_than_a_nanometre_is_rounded_half_away_from_zero() {
        assert_eq!(to_nm("0.0000005", Unit::Millimeter), Some(1));
        assert_eq!(to_nm("-0.0000005", Unit::Millimeter), Some(-1));
        assert_eq!(to_nm("0.00000049", Unit::Millimeter), Some(0));
        assert_eq!(to_nm("0.0005", Unit::Micron), Some(1));
    }

    #[test]
    fn a_text_that_is_not_a_number_is_refused() {
        for text in [
            "", " ", "-", ".", "e5", "1e", "1e+", "1,5", "0x10", "1 2", "nan", "inf", "--1",
            "1.2.3", "1e99999",
        ] {
            assert_eq!(to_nm(text, Unit::Millimeter), None, "{text}");
        }
        let long = "1".repeat(31);
        assert_eq!(to_nm(&long, Unit::Millimeter), None);
    }

    #[test]
    fn a_value_that_does_not_fit_in_64_bits_is_refused_and_not_wrapped() {
        assert_eq!(to_nm("9223372036854775807", Unit::Micron), None);
        assert_eq!(to_nm("1e30", Unit::Inch), None);
        assert_eq!(to_nm("1e-70", Unit::Inch), Some(0));
        assert_eq!(
            to_nm("9223372036.854775", Unit::Millimeter),
            Some(9_223_372_036_854_775)
        );
    }

    #[test]
    fn a_number_written_from_nanometres_reads_back_to_the_same_nanometres() {
        let mut state: u64 = 0x0001_2581;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 11
        };
        for _ in 0..4000 {
            let unit =
                [Unit::Millimeter, Unit::Micron, Unit::Inch, Unit::Mil][(next() % 4) as usize];
            let nm = (next() % 2_000_000_000_000) as i64 - 1_000_000_000_000;
            // the decimal of nm / nm_per_unit with twenty digits after the point, which is far
            // finer than half a nanometre in every unit
            let per = unit.nm_per_unit();
            let (whole, rest) = (nm.abs() / per, nm.abs() % per);
            let fraction = (rest as u128 * 10u128.pow(20)) / per as u128;
            let text = format!("{}{whole}.{fraction:020}", if nm < 0 { "-" } else { "" });
            assert_eq!(to_nm(&text, unit), Some(nm), "{text} {unit:?}");
        }
    }

    #[test]
    fn real_numbers_must_be_finite() {
        assert_eq!(to_f64(" 4.2000 "), Some(4.2));
        assert_eq!(to_f64("NaN"), None);
        assert_eq!(to_f64("inf"), None);
        assert_eq!(to_f64("x"), None);
    }

    #[test]
    fn units_name_themselves_and_have_the_known_sizes() {
        assert_eq!(Unit::Inch.nm_per_unit(), 25_400_000);
        assert_eq!(Unit::Mil.nm_per_unit() * 1000, Unit::Inch.nm_per_unit());
        assert_eq!(Unit::Millimeter.name(), "MILLIMETER");
    }
}
