// SPDX-License-Identifier: GPL-3.0-or-later
//! Numbers of the files as integer nanometres. A value with a decimal point and a coordinate
//! written as an integer with an implied point are converted with `i128` arithmetic and rounded
//! half away from zero, so a coordinate that fits the format is exact.

use crate::settings::ZeroOmission;

/// The most digits of one number that are read; more is not a number of a CAM file.
const MAX_DIGITS: usize = 40;

/// The most digits after the point that count (more are cut off).
const MAX_FRACTION: usize = 18;

/// `mantissa / scale` units of `unit_nm` nanometres each, rounded half away from zero.
fn scaled_to_nm(mantissa: i128, scale: i128, negative: bool, unit_nm: i64) -> Option<i64> {
    let numerator = mantissa.checked_mul(i128::from(unit_nm))?;
    let rounded = numerator
        .checked_mul(2)?
        .checked_add(scale)?
        .checked_div(scale.checked_mul(2)?)?;
    let signed = if negative { -rounded } else { rounded };
    i64::try_from(signed).ok()
}

/// A number written with or without a decimal point (`-0.25`, `.5`, `12`) as nanometres, when its
/// unit is `unit_nm` nanometres. `None` for anything that is not such a number.
pub fn decimal_to_nm(text: &str, unit_nm: i64) -> Option<i64> {
    if text.is_empty() || text.len() > MAX_DIGITS + 2 {
        return None;
    }
    let (negative, body) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (whole, fraction) = match body.split_once('.') {
        Some((w, f)) => (w, f),
        None => (body, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let kept = &fraction[..fraction.len().min(MAX_FRACTION)];
    let mut mantissa: i128 = 0;
    for b in whole.bytes().chain(kept.bytes()) {
        mantissa = mantissa
            .checked_mul(10)?
            .checked_add(i128::from(b - b'0'))?;
    }
    let scale = 10i128.checked_pow(u32::try_from(kept.len()).ok()?)?;
    scaled_to_nm(mantissa, scale, negative, unit_nm)
}

/// A coordinate written as an integer with `decimals` implied decimals, with the zeros that the
/// file leaves out put back, as nanometres. `digits` holds no sign.
pub fn integer_to_nm(
    digits: &str,
    negative: bool,
    integer_digits: u32,
    decimals: u32,
    omission: ZeroOmission,
    unit_nm: i64,
) -> Option<i64> {
    if digits.is_empty() || digits.len() > MAX_DIGITS || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let total = (integer_digits + decimals) as usize;
    let padded;
    let used = if omission == ZeroOmission::Trailing && digits.len() < total {
        padded = format!("{digits:0<total$}");
        padded.as_str()
    } else {
        digits
    };
    let mut mantissa: i128 = 0;
    for b in used.bytes() {
        mantissa = mantissa
            .checked_mul(10)?
            .checked_add(i128::from(b - b'0'))?;
    }
    let scale = 10i128.checked_pow(decimals)?;
    scaled_to_nm(mantissa, scale, negative, unit_nm)
}

/// Splits a signed number text into its sign and its digits (with a point, when there is one).
pub fn split_sign(text: &str) -> (bool, &str) {
    match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;
    const IN: i64 = 25_400_000;

    #[test]
    fn decimals_are_exact_and_rounded_half_away_from_zero() {
        assert_eq!(decimal_to_nm("1.5", MM), Some(1_500_000));
        assert_eq!(decimal_to_nm("-0.25", MM), Some(-250_000));
        assert_eq!(decimal_to_nm(".5", MM), Some(500_000));
        assert_eq!(decimal_to_nm("12", MM), Some(12_000_000));
        assert_eq!(decimal_to_nm("0.001", IN), Some(25_400));
        assert_eq!(decimal_to_nm("0.0000005", MM), Some(1));
        assert_eq!(decimal_to_nm("-0.0000005", MM), Some(-1));
        assert_eq!(decimal_to_nm("0.0000004", MM), Some(0));
    }

    #[test]
    fn what_is_not_a_number_is_refused() {
        for text in ["", ".", "-", "1.2.3", "1e3", "abc", "1,5", "--1", " 1"] {
            assert_eq!(decimal_to_nm(text, MM), None, "{text:?}");
        }
        assert_eq!(decimal_to_nm(&"9".repeat(60), MM), None);
        assert_eq!(decimal_to_nm(&"9".repeat(38), MM), None);
    }

    #[test]
    fn integers_with_an_implied_point() {
        let leading = ZeroOmission::Leading;
        assert_eq!(
            integer_to_nm("1500000", false, 4, 6, leading, MM),
            Some(1_500_000)
        );
        assert_eq!(integer_to_nm("0", true, 4, 6, leading, MM), Some(0));
        assert_eq!(integer_to_nm("25", true, 2, 4, leading, IN), Some(-63_500));
        // trailing zeros left out: 15 with 3.3 is 150.000
        assert_eq!(
            integer_to_nm("15", false, 3, 3, ZeroOmission::Trailing, MM),
            Some(150_000_000)
        );
        assert_eq!(
            integer_to_nm("150000", false, 3, 3, ZeroOmission::Trailing, MM),
            Some(150_000_000)
        );
        assert_eq!(integer_to_nm("", false, 3, 3, leading, MM), None);
        assert_eq!(integer_to_nm("1.5", false, 3, 3, leading, MM), None);
    }

    #[test]
    fn the_sign_is_split_off() {
        assert_eq!(split_sign("-12"), (true, "12"));
        assert_eq!(split_sign("+12"), (false, "12"));
        assert_eq!(split_sign("12"), (false, "12"));
        assert_eq!(split_sign(""), (false, ""));
    }
}
