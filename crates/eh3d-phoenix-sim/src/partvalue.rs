// SPDX-License-Identifier: GPL-3.0-or-later
//! The value of a part as it is written on a schematic, read as a number.
//!
//! Besides the notation of SPICE (`4.7k`, `100n`, `2.2meg`), the notation of the marking of parts
//! (IEC 60062) is read: a letter in the place of the decimal point (`4k7` is 4.7 kilo, `1u5` is
//! 1.5 micro, `2M2` is 2.2 mega, `4R7` is 4.7 and `R47` is 0.47). A capital `M` means mega in this
//! notation, as on the marking of a resistor, and a small `m` means milli; the setting
//! `upper_m_is_mega` of the table of models turns that off, which gives the reading of SPICE where
//! `M` and `m` are both milli. Units after the number (`F`, `H`, `V`, `A`, `ohm`, the ohm sign) are
//! ignored, as in the netlist reader.

use crate::units::{format_number, parse_number};

/// The letters that can stand in the place of the decimal point.
const POINT_LETTERS: &str = "rkmgtunpf";

/// The marking `4k7`: digits, one letter and digits again. `None` when the text is not of that
/// form.
fn marking_form(text: &str, upper_m_is_mega: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let split = bytes.iter().position(|b| !b.is_ascii_digit())?;
    let letter = char::from(bytes[split]);
    if !letter.is_ascii_alphabetic() || !POINT_LETTERS.contains(letter.to_ascii_lowercase()) {
        return None;
    }
    let tail = &text[split + 1..];
    if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let head = &text[..split];
    if head.is_empty() && !letter.eq_ignore_ascii_case(&'r') {
        return None;
    }
    let head = if head.is_empty() { "0" } else { head };
    let suffix = match letter {
        'R' | 'r' => String::new(),
        'M' if upper_m_is_mega => "meg".to_owned(),
        other => other.to_ascii_lowercase().to_string(),
    };
    Some(format!("{head}.{tail}{suffix}"))
}

/// A capital `M` after the digits of a number is mega when the setting says so (`2.2M`, `5MHz`);
/// `MEG` and `MIL` are left as they are.
fn capital_m_as_mega(text: &str, upper_m_is_mega: bool) -> String {
    if !upper_m_is_mega {
        return text.to_owned();
    }
    let end = text
        .bytes()
        .position(|b| !(b.is_ascii_digit() || b == b'.' || b == b'+' || b == b'-'))
        .unwrap_or(text.len());
    let (number, rest) = text.split_at(end);
    let spice_word = rest
        .get(..3)
        .is_some_and(|s| s.eq_ignore_ascii_case("meg") || s.eq_ignore_ascii_case("mil"));
    if number.is_empty() || !rest.starts_with('M') || spice_word {
        return text.to_owned();
    }
    format!("{number}meg{}", &rest[1..])
}

/// Reads the value of a part: `4k7`, `100n`, `1u5`, `2M2`, `10kOhm`, `0.1`, `1e-9`. `None` when it
/// is not a finite number.
pub fn parse_value(text: &str, upper_m_is_mega: bool) -> Option<f64> {
    let cleaned = text
        .trim()
        .replace(['\u{b5}', '\u{3bc}'], "u")
        .replace('\u{3a9}', "")
        .replace('\u{2126}', "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned.chars().any(char::is_control) {
        return None;
    }
    if let Some(form) = marking_form(cleaned, upper_m_is_mega) {
        return parse_number(&form);
    }
    parse_number(&capital_m_as_mega(cleaned, upper_m_is_mega))
}

/// The value as the text of a netlist: a number in its shortest exact form.
pub fn value_text(text: &str, upper_m_is_mega: bool) -> Option<String> {
    parse_value(text, upper_m_is_mega).map(format_number)
}

/// The value of an independent source: a number (`5`, `3V3`) or the words of a netlist that follow
/// the nodes (`DC 5`, `SIN(0 1 1k)`, `PULSE(0 5 0 1n 1n 1m 2m)`, `PWL(0 0 1m 1)`), which are passed
/// on as they are.
pub fn source_text(text: &str, upper_m_is_mega: bool) -> Option<String> {
    let trimmed = text.trim();
    let first = trimmed.chars().next()?;
    if trimmed.chars().any(char::is_control) {
        return None;
    }
    if first.is_ascii_alphabetic() && marking_form(trimmed, upper_m_is_mega).is_none() {
        return Some(trimmed.to_owned());
    }
    value_text(trimmed, upper_m_is_mega)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: Option<f64>, want: f64) {
        let got = got.unwrap_or(f64::NAN);
        assert!(
            (got - want).abs() <= 1e-12 * want.abs(),
            "got {got}, want {want}"
        );
    }

    #[test]
    fn the_marking_with_a_letter_for_the_point_is_read() {
        close(parse_value("4k7", true), 4700.0);
        close(parse_value("1u5", true), 1.5e-6);
        close(parse_value("2M2", true), 2.2e6);
        close(parse_value("4R7", true), 4.7);
        close(parse_value("R47", true), 0.47);
        close(parse_value("1k5", true), 1500.0);
        close(parse_value("3n3", true), 3.3e-9);
        close(parse_value("2p2", true), 2.2e-12);
    }

    #[test]
    fn a_capital_m_is_mega_unless_the_setting_says_it_is_milli() {
        close(parse_value("2M2", true), 2.2e6);
        close(parse_value("2M2", false), 2.2e-3);
        close(parse_value("2.2M", true), 2.2e6);
        close(parse_value("2.2M", false), 2.2e-3);
        close(parse_value("10m", true), 0.01);
        close(parse_value("10MEG", true), 1e7);
        close(parse_value("10meg", true), 1e7);
        close(parse_value("5MHz", true), 5e6);
    }

    #[test]
    fn the_notation_of_spice_and_the_units_are_read() {
        close(parse_value("10k", true), 1e4);
        close(parse_value("100n", true), 1e-7);
        close(parse_value("10kOhm", true), 1e4);
        close(parse_value("4.7\u{b5}F", true), 4.7e-6);
        close(parse_value("100 \u{3a9}", true), 100.0);
        close(parse_value("1e-9", true), 1e-9);
        close(parse_value("0.1", true), 0.1);
        close(parse_value("5V", true), 5.0);
        close(parse_value("  330  ", true), 330.0);
    }

    #[test]
    fn what_is_not_a_number_is_refused() {
        for bad in ["", "  ", "abc", "k7", "4k7x", "--1", "4k\n7"] {
            assert_eq!(parse_value(bad, true), None, "{bad:?}");
        }
    }

    #[test]
    fn a_source_takes_a_number_or_the_words_of_a_netlist() {
        assert_eq!(source_text("DC 5", true).as_deref(), Some("DC 5"));
        assert_eq!(
            source_text("SIN(0 1 1k)", true).as_deref(),
            Some("SIN(0 1 1k)")
        );
        assert_eq!(
            source_text("PULSE(0 5 0 1n 1n 1m 2m)", true).as_deref(),
            Some("PULSE(0 5 0 1n 1n 1m 2m)")
        );
        close(source_text("5V", true).and_then(|t| parse_number(&t)), 5.0);
        close(source_text("12", true).and_then(|t| parse_number(&t)), 12.0);
        assert_eq!(source_text("", true), None);
        assert_eq!(source_text("DC 5\nR9 a b 1", true), None);
    }

    #[test]
    fn the_text_of_a_value_reads_back_as_the_same_number() {
        for text in ["4k7", "100n", "1u5", "2M2", "0.1", "330"] {
            let written = value_text(text, true).unwrap();
            close(parse_number(&written), parse_value(text, true).unwrap());
        }
    }
}
