// SPDX-License-Identifier: GPL-3.0-or-later
//! Numbers with the unit suffixes of SPICE: `f p n u m k meg g t` and `mil`. As in SPICE the
//! suffix is not case sensitive (`M` is milli, `MEG` is mega, `F` is femto), and letters after the
//! suffix are a unit that is ignored (`10kOhm`, `1uF`, `5V`).

/// The scale of the letters that follow the digits of a number.
fn suffix_scale(letters: &str) -> f64 {
    let lower = letters.to_lowercase();
    if lower.starts_with("meg") {
        return 1e6;
    }
    if lower.starts_with("mil") {
        return 25.4e-6;
    }
    match lower.chars().next() {
        Some('t') => 1e12,
        Some('g') => 1e9,
        Some('k') => 1e3,
        Some('m') => 1e-3,
        Some('u') | Some('µ') => 1e-6,
        Some('n') => 1e-9,
        Some('p') => 1e-12,
        Some('f') => 1e-15,
        _ => 1.0,
    }
}

/// Parses a number with an optional exponent, an optional suffix and an optional unit. The result
/// is finite or the text is refused.
pub fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let mut digits = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
        digits += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut look = end + 1;
        if look < bytes.len() && (bytes[look] == b'+' || bytes[look] == b'-') {
            look += 1;
        }
        if look < bytes.len() && bytes[look].is_ascii_digit() {
            while look < bytes.len() && bytes[look].is_ascii_digit() {
                look += 1;
            }
            end = look;
        }
    }
    let mantissa: f64 = text[..end].parse().ok()?;
    let rest = &text[end..];
    if !rest.chars().all(char::is_alphabetic) {
        return None;
    }
    let value = mantissa * suffix_scale(rest);
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

/// The text of a number as the writer of netlists prints it: the shortest form that reads back
/// as the same number.
pub fn format_number(value: f64) -> String {
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * b.abs().max(1e-30)
    }

    #[test]
    fn suffixes_scale_the_number() {
        for (text, want) in [
            ("1", 1.0),
            ("2.5", 2.5),
            (".5", 0.5),
            ("5.", 5.0),
            ("-3", -3.0),
            ("10k", 1e4),
            ("4.7K", 4.7e3),
            ("1meg", 1e6),
            ("1MEG", 1e6),
            ("2g", 2e9),
            ("3t", 3e12),
            ("100m", 0.1),
            ("33u", 33e-6),
            ("10n", 1e-8),
            ("22p", 22e-12),
            ("5f", 5e-15),
            ("1mil", 25.4e-6),
            ("1e3", 1e3),
            ("1.5e-9", 1.5e-9),
            ("2e3k", 2e6),
        ] {
            let got = parse_number(text).unwrap_or_else(|| panic!("{text}"));
            assert!(close(got, want), "{text}: {got} {want}");
        }
    }

    #[test]
    fn letters_after_the_suffix_are_a_unit() {
        assert!(close(parse_number("10kOhm").unwrap(), 1e4));
        assert!(close(parse_number("1uF").unwrap(), 1e-6));
        assert!(close(parse_number("5V").unwrap(), 5.0));
        assert!(close(parse_number("3mA").unwrap(), 3e-3));
        assert!(close(parse_number("10Hz").unwrap(), 10.0));
        // as in SPICE, a bare F is femto
        assert!(close(parse_number("1F").unwrap(), 1e-15));
    }

    #[test]
    fn text_that_is_not_a_number_is_refused() {
        for text in [
            "", "k", "abc", "1.2.3", "--1", "1k2", "1e999", "1,5", "nan", "inf", "5%",
        ] {
            assert!(parse_number(text).is_none(), "{text}");
        }
    }

    #[test]
    fn formatted_numbers_read_back_exactly() {
        for v in [0.0, 1.0, -2.5, 1e-15, 4.7e3, 0.1 + 0.2, 1.0 / 3.0, 6.02e23] {
            assert_eq!(parse_number(&format_number(v)).unwrap(), v);
        }
        assert_eq!(format_number(-0.0), "0e0");
    }
}
