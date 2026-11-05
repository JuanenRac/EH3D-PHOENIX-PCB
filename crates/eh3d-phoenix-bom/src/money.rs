// SPDX-License-Identifier: GPL-3.0-or-later
//! Money and percentages as whole millionths, so that no price passes through a floating-point
//! number: `0.0125` is `12500`. Every product is worked out in 128 bits and rounded half up.

/// One unit of money (or one per cent) in millionths.
pub const MICRO: i64 = 1_000_000;

/// Reads a decimal number with no sign and at most six decimals (more only when the rest are
/// zeros) as millionths. `None` for anything else.
pub fn parse_micro(text: &str) -> Option<i64> {
    let text = text.trim();
    let (whole, fraction) = match text.split_once('.') {
        Some((w, f)) => (w, f),
        None => (text, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if whole.len() > 12 || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fraction = fraction.trim_end_matches('0');
    if fraction.len() > 6 {
        return None;
    }
    let whole_value: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut fraction_value: i64 = 0;
    for b in fraction.bytes() {
        fraction_value = fraction_value * 10 + i64::from(b - b'0');
    }
    for _ in fraction.len()..6 {
        fraction_value *= 10;
    }
    whole_value.checked_mul(MICRO)?.checked_add(fraction_value)
}

/// Millionths as a decimal with six decimals, exactly.
pub fn format_micro(value: i128) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let a = value.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

/// `a * b / d` rounded half up, for a positive `d`.
pub fn mul_div(a: i128, b: i128, d: i128) -> i128 {
    let p = a * b;
    (p * 2 + d) / (d * 2)
}

/// `a / d` rounded up, for a positive `d` and a number that is not negative.
pub fn div_ceil(a: i128, d: i128) -> i128 {
    (a + d - 1) / d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_are_read_exactly() {
        assert_eq!(parse_micro("0.0125"), Some(12_500));
        assert_eq!(parse_micro("12"), Some(12_000_000));
        assert_eq!(parse_micro(" 3.5 "), Some(3_500_000));
        assert_eq!(parse_micro(".5"), Some(500_000));
        assert_eq!(parse_micro("1.2500000"), Some(1_250_000));
        assert_eq!(parse_micro("0.000001"), Some(1));
    }

    #[test]
    fn numbers_that_are_not_decimals_are_refused() {
        for bad in [
            "",
            ".",
            "-1",
            "+1",
            "1e3",
            "1,5",
            "0.0000011",
            "abc",
            "1.2.3",
        ] {
            assert_eq!(parse_micro(bad), None, "{bad}");
        }
        assert_eq!(parse_micro("9999999999999"), None);
    }

    #[test]
    fn millionths_are_written_with_six_decimals() {
        assert_eq!(format_micro(12_500), "0.012500");
        assert_eq!(format_micro(0), "0.000000");
        assert_eq!(format_micro(-1_500_000), "-1.500000");
        assert_eq!(format_micro(123_456_789), "123.456789");
    }

    #[test]
    fn products_are_rounded_half_up_and_divisions_up() {
        assert_eq!(mul_div(10, 1, 4), 3);
        assert_eq!(mul_div(10, 1, 3), 3);
        assert_eq!(mul_div(7, 1, 3), 2);
        assert_eq!(mul_div(0, 5, 3), 0);
        assert_eq!(div_ceil(10, 4), 3);
        assert_eq!(div_ceil(8, 4), 2);
        assert_eq!(div_ceil(0, 4), 0);
    }
}
