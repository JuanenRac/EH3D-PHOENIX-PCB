// SPDX-License-Identifier: GPL-3.0-or-later
//! Ranges of references in the cell of a bill: `R1-R5` is the five parts `R1`, `R2`, `R3`, `R4`
//! and `R5`, and `R1-5` is the same. A range is made of a prefix of letters, a number, a dash and
//! an end that is a number or the same prefix and a number. Every other text, `TP-1` or a name
//! with a dash that does not start like a part, is a reference as it is.
//!
//! A range that is badly formed, upside down or longer than the limit is an error with a code of
//! its own, never a silent guess.

use crate::codes::{BomError, RANGE_INVERTED, RANGE_MALFORMED, RANGE_TOO_LARGE};

/// The most digits of a number of a range; more would not be a part number.
const MAX_DIGITS: usize = 9;

/// Takes out the spaces around the dashes, so that `R1 - R5` is read as `R1-R5` when a space also
/// separates references.
pub fn collapse_dashes(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut pending = String::new();
    let mut after_dash = false;
    for c in cell.chars() {
        if c == '-' {
            pending.clear();
            out.push('-');
            after_dash = true;
        } else if c.is_whitespace() {
            if !after_dash {
                pending.push(c);
            }
        } else {
            out.push_str(&pending);
            pending.clear();
            after_dash = false;
            out.push(c);
        }
    }
    out.push_str(&pending);
    out
}

/// Splits `R12` into the prefix `R` and the digits `12`; `None` when it is not letters followed by
/// digits only.
fn split_name(s: &str) -> Option<(&str, &str)> {
    let first_digit = s.find(|c: char| c.is_ascii_digit())?;
    let (prefix, digits) = s.split_at(first_digit);
    let prefix_ok =
        !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_alphabetic() || c == '_');
    (prefix_ok && digits.chars().all(|c| c.is_ascii_digit())).then_some((prefix, digits))
}

fn number(digits: &str) -> Option<u64> {
    if digits.is_empty() || digits.len() > MAX_DIGITS {
        return None;
    }
    digits.parse().ok()
}

/// Expands one reference of a cell: the reference itself, or the parts of a range. `max_range` is
/// the most parts one range may name.
pub fn expand(token: &str, max_range: usize) -> Result<Vec<String>, BomError> {
    let Some((left, right)) = token.split_once('-') else {
        return Ok(vec![token.to_owned()]);
    };
    let Some((prefix, start_digits)) = split_name(left) else {
        return Ok(vec![token.to_owned()]);
    };
    let malformed = |why: &str| BomError::new(RANGE_MALFORMED, format!("{token}: {why}"));
    if right.is_empty() {
        return Err(malformed("the end of the range is missing"));
    }
    let end_digits = if right.chars().all(|c| c.is_ascii_digit()) {
        right
    } else {
        match split_name(right) {
            Some((p, d)) if p == prefix => d,
            Some(_) => return Err(malformed("the two ends have different prefixes")),
            None => return Err(malformed("the end is not a number or a reference")),
        }
    };
    let (Some(start), Some(end)) = (number(start_digits), number(end_digits)) else {
        return Err(malformed("a number is empty or has more than nine digits"));
    };
    if start > end {
        return Err(BomError::new(
            RANGE_INVERTED,
            format!("{token}: the end {end} is below the start {start}"),
        ));
    }
    let count = end - start + 1;
    if count > max_range as u64 {
        return Err(BomError::new(
            RANGE_TOO_LARGE,
            format!("{token}: {count} parts, the limit is {max_range}"),
        ));
    }
    // a start written with leading zeros (R01) keeps its width
    let width = if start_digits.len() > 1 && start_digits.starts_with('0') {
        start_digits.len()
    } else {
        0
    };
    Ok((start..=end)
        .map(|n| format!("{prefix}{n:0width$}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_is_expanded_to_the_parts_it_names() {
        assert_eq!(
            expand("R1-R5", 100).unwrap(),
            ["R1", "R2", "R3", "R4", "R5"]
        );
        // the short form with the number alone
        assert_eq!(expand("C8-11", 100).unwrap(), ["C8", "C9", "C10", "C11"]);
        // one part, and leading zeros keep their width
        assert_eq!(expand("U7-U7", 100).unwrap(), ["U7"]);
        assert_eq!(
            expand("R08-R11", 100).unwrap(),
            ["R08", "R09", "R10", "R11"]
        );
    }

    #[test]
    fn a_text_that_is_not_a_range_is_a_reference_as_it_is() {
        for text in ["R1", "TP-1", "TP-A", "12-14", "-R1", "J-", "MH_1"] {
            assert_eq!(expand(text, 10).unwrap(), [text], "{text}");
        }
    }

    #[test]
    fn a_range_badly_formed_is_refused_with_its_code() {
        for (text, needle) in [
            ("R1-", "missing"),
            ("R1-C5", "different prefixes"),
            ("R1-A", "not a number"),
            ("R1-5x", "not a number"),
            ("R1-R2-R3", "not a number"),
            ("R1-1234567890", "nine digits"),
        ] {
            let e = expand(text, 100).unwrap_err();
            assert_eq!(e.code, RANGE_MALFORMED, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn an_inverted_range_and_an_excessive_one_have_their_own_codes() {
        let e = expand("R5-R1", 100).unwrap_err();
        assert_eq!(e.code, RANGE_INVERTED);
        assert_eq!(e.detail, "R5-R1: the end 1 is below the start 5");
        let e = expand("R1-R101", 100).unwrap_err();
        assert_eq!(e.code, RANGE_TOO_LARGE);
        assert_eq!(e.detail, "R1-R101: 101 parts, the limit is 100");
        // the limit itself is allowed
        assert_eq!(expand("R1-R100", 100).unwrap().len(), 100);
        assert_eq!(expand("R1-R100", 99).unwrap_err().code, RANGE_TOO_LARGE);
    }

    #[test]
    fn the_spaces_around_a_dash_are_taken_out() {
        assert_eq!(collapse_dashes("R1 - R5, C1"), "R1-R5, C1");
        assert_eq!(collapse_dashes("R1 -R5 C1- C3"), "R1-R5 C1-C3");
        assert_eq!(collapse_dashes("R1 R2 "), "R1 R2 ");
        assert_eq!(collapse_dashes("R1 -"), "R1-");
    }
}
