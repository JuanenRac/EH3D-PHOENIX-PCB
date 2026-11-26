// SPDX-License-Identifier: GPL-3.0-or-later
//! The numbers of a library file that the model cannot hold to the last digit.
//!
//! The model counts lengths in whole nanometres and angles in whole micro-degrees. Library files
//! often carry more decimals than that (the internal unit of the program that wrote them is not a
//! whole number of nanometres) and some carry an exponent. Such a number is read as the nearest
//! whole millionth, half to even, and the text it came as is kept beside the item, in its `extra`
//! list, under a name that no XML attribute can have. The writer puts that text back for as long as
//! the value of the item is still the one that was read, so that a file that is read and saved says
//! exactly what it said; a value that was edited is written as the new value.

use eh3d_phoenix_model::Nm;

/// What starts the name of a kept text: a blank, which no name of an XML attribute can hold.
pub(crate) const MARK: char = ' ';

/// The largest power of ten, either way, that an exponent is worked out for.
const MAX_EXPONENT: i32 = 12;

/// A number with an exponent written as a plain decimal; the text itself when it has none; `None`
/// when it is not a number or the exponent is out of reach.
fn plain(text: &str) -> Option<std::borrow::Cow<'_, str>> {
    let Some((mantissa, exponent)) = text.split_once(['e', 'E']) else {
        return Some(text.into());
    };
    let exponent: i32 = exponent.parse().ok()?;
    if !(-MAX_EXPONENT..=MAX_EXPONENT).contains(&exponent) || mantissa.len() > 40 {
        return None;
    }
    let (sign, body) = match mantissa.as_bytes().first() {
        Some(b'-') => ("-", &mantissa[1..]),
        Some(b'+') => ("", &mantissa[1..]),
        _ => ("", mantissa),
    };
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = format!("{whole}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // where the point stands among the digits after the shift
    let point = i32::try_from(whole.len()).ok()? + exponent;
    let out = if point <= 0 {
        format!(
            "{sign}0.{}{digits}",
            "0".repeat(point.unsigned_abs() as usize)
        )
    } else {
        let point = point as usize;
        if point >= digits.len() {
            format!("{sign}{digits}{}", "0".repeat(point - digits.len()))
        } else {
            format!("{sign}{}.{}", &digits[..point], &digits[point..])
        }
    };
    Some(out.into())
}

/// A number of a file in millionths, and whether the millionths say it exactly and in the plain
/// form (false: its text has to be kept to be written back). `None` when it is not a number.
pub(crate) fn millionths(text: &str) -> Option<(i64, bool)> {
    let trimmed = text.trim();
    let plain = plain(trimmed)?;
    let (value, exact) = Nm::parse_mm(&plain).ok()?;
    Some((value.get(), exact && plain == trimmed))
}

/// The entry of an `extra` list that keeps the text of the attribute `name`.
pub(crate) fn kept(name: &str, text: &str) -> (String, String) {
    (format!("{MARK}{name}"), text.to_owned())
}

/// Puts the kept texts back into the attributes that are about to be written: an attribute whose
/// value is still the one the text was read as takes the text; one that the writer left out
/// because it holds its default of zero comes back when the text was read as zero. The kept
/// entries themselves are never written.
pub(crate) fn restore(attrs: &mut Vec<(String, String)>) {
    if !attrs.iter().any(|(name, _)| name.starts_with(MARK)) {
        return;
    }
    let (marks, mut out): (Vec<_>, Vec<_>) = attrs
        .drain(..)
        .partition(|(name, _)| name.starts_with(MARK));
    for (mark, text) in marks {
        let name = mark.trim_start_matches(MARK);
        let Some((was, _)) = millionths(&text) else {
            continue;
        };
        match out.iter_mut().find(|(n, _)| n == name) {
            Some((_, value)) => {
                if millionths(value).map(|(v, _)| v) == Some(was) {
                    *value = text;
                }
            }
            None => {
                if was == 0 {
                    out.push((name.to_owned(), text));
                }
            }
        }
    }
    *attrs = out;
}

/// How many numbers of an `extra` list are kept as text.
pub(crate) fn kept_in(extra: &[(String, String)]) -> usize {
    extra.iter().filter(|(n, _)| n.starts_with(MARK)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(name: &str, value: &str) -> (String, String) {
        (name.to_owned(), value.to_owned())
    }

    #[test]
    fn a_number_is_read_to_the_nearest_millionth_and_says_when_that_is_not_the_whole_of_it() {
        assert_eq!(millionths("1.27"), Some((1_270_000, true)));
        assert_eq!(millionths(" -0.05 "), Some((-50_000, true)));
        assert_eq!(millionths("0.000003125"), Some((3, false)));
        assert_eq!(millionths("-1.269996875"), Some((-1_269_997, false)));
        // half to even
        assert_eq!(millionths("0.0000025"), Some((2, false)));
        assert_eq!(millionths("0.0000035"), Some((4, false)));
        assert_eq!(millionths("0.1000000"), Some((100_000, true)));
        for bad in ["", "abc", "1.2.3", "1e", "e3", "1e99", "--1", "1,5"] {
            assert_eq!(millionths(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_number_with_an_exponent_is_worked_out_without_floating_point() {
        assert_eq!(millionths("1e-3"), Some((1_000, false)));
        assert_eq!(millionths("1.5E2"), Some((150_000_000, false)));
        assert_eq!(millionths("-2.5e-1"), Some((-250_000, false)));
        assert_eq!(millionths("12345e-4"), Some((1_234_500, false)));
        assert_eq!(millionths("1e-12"), Some((0, false)));
        assert_eq!(millionths("0.00012e+1"), Some((1_200, false)));
    }

    #[test]
    fn the_kept_text_goes_back_while_the_value_is_the_one_that_was_read() {
        let mut attrs = vec![
            attr("x", "-1.269997"),
            attr("y", "2"),
            kept("x", "-1.269996875"),
            kept("y", "0.000003125"),
        ];
        restore(&mut attrs);
        // x was not edited and takes its text; y was edited and keeps the new value
        assert_eq!(attrs, [attr("x", "-1.269996875"), attr("y", "2")]);
    }

    #[test]
    fn a_default_of_zero_that_was_written_with_a_text_comes_back() {
        let mut attrs = vec![attr("x", "1"), kept("curve", "1e-12")];
        restore(&mut attrs);
        assert_eq!(attrs, [attr("x", "1"), attr("curve", "1e-12")]);
        let mut attrs = vec![attr("x", "1"), kept("diameter", "0.5000001")];
        restore(&mut attrs);
        assert_eq!(attrs, [attr("x", "1")], "left out for another reason");
        assert_eq!(kept_in(&[kept("x", "1"), attr("align", "center")]), 1);
    }

    #[test]
    fn any_millionth_with_extra_digits_comes_back_as_it_was() {
        // a fixed seed: the same texts on every run
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for _ in 0..2_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let whole = (state % 200) as i64 - 100;
            let fraction = (state >> 8) % 1_000_000_000;
            let text = format!("{whole}.{fraction:09}");
            let (value, exact) = millionths(&text).unwrap();
            let mut attrs = vec![attr("x", &crate::write::fmt_mm(value))];
            if !exact {
                attrs.push(kept("x", &text));
            }
            restore(&mut attrs);
            assert_eq!(millionths(&attrs[0].1).unwrap().0, value, "{text}");
            if !exact {
                assert_eq!(attrs[0].1, text);
            }
        }
    }
}
