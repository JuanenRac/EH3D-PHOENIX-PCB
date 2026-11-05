// SPDX-License-Identifier: GPL-3.0-or-later
//! Numbers of a script: lengths with a unit and rotations.
//!
//! A length is a decimal number with an optional unit written straight after it (`0.2mm`, `8mil`,
//! `1inch`, `25micron`, `2grid`). A number without a unit is in the current unit of the script
//! (the unit of the last grid command). The value is kept as a decimal scaled by 10^6, so a length
//! is exact until it is turned into integer nanometres, which rounds half away from zero once.

use eh3d_phoenix_model::{Nm, ScaledDecimal, Udeg};

/// A unit of length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Millimetres.
    Mm,
    /// Thousandths of an inch.
    Mil,
    /// Inches.
    Inch,
    /// Micrometres.
    Micron,
    /// Steps of the current grid.
    Grid,
}

impl Unit {
    /// The unit named by a word, case-insensitively: `mm`, `mil`, `inch`, `in`, `micron`, `mic`.
    /// The grid unit has no word of this kind; it only exists as a suffix.
    pub fn from_word(word: &str) -> Option<Unit> {
        match word.to_ascii_lowercase().as_str() {
            "mm" => Some(Unit::Mm),
            "mil" => Some(Unit::Mil),
            "inch" | "in" => Some(Unit::Inch),
            "micron" | "mic" => Some(Unit::Micron),
            _ => None,
        }
    }

    /// The unit named by the suffix of a number: the words of [`Unit::from_word`] and `grid`.
    pub fn from_suffix(suffix: &str) -> Option<Unit> {
        if suffix.eq_ignore_ascii_case("grid") {
            Some(Unit::Grid)
        } else {
            Unit::from_word(suffix)
        }
    }

    /// The name used in messages.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mil => "mil",
            Unit::Inch => "inch",
            Unit::Micron => "micron",
            Unit::Grid => "grid",
        }
    }

    /// Nanometres in one unit; the grid needs the size of the current grid.
    fn nm(self, grid_nm: i64) -> i128 {
        match self {
            Unit::Mm => 1_000_000,
            Unit::Mil => 25_400,
            Unit::Inch => 25_400_000,
            Unit::Micron => 1_000,
            Unit::Grid => i128::from(grid_nm),
        }
    }
}

/// A length as written: a decimal scaled by 10^6 and the unit when one was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Len {
    /// The value times 10^6.
    pub scaled: i128,
    /// The unit that followed the number, if any.
    pub unit: Option<Unit>,
}

/// Divides and rounds half away from zero.
fn round_div(n: i128, d: i128) -> i128 {
    let half = d / 2;
    if n >= 0 {
        (n + half) / d
    } else {
        -((-n + half) / d)
    }
}

impl Len {
    /// Reads `[+-]digits[.digits]` followed by an optional unit. `None` when the text is not a
    /// number or the unit is not one of the units of a length.
    pub fn parse(text: &str) -> Option<Len> {
        let bytes = text.as_bytes();
        let mut end = 0;
        if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
            end = 1;
        }
        let digits_from = end;
        let mut dots = 0;
        while end < bytes.len() {
            match bytes[end] {
                b'0'..=b'9' => end += 1,
                b'.' if dots == 0 => {
                    dots += 1;
                    end += 1;
                }
                _ => break,
            }
        }
        if end == digits_from || &text[digits_from..end] == "." {
            return None;
        }
        let number = ScaledDecimal::parse(&text[..end]).ok()?;
        let suffix = &text[end..];
        let unit = if suffix.is_empty() {
            None
        } else {
            Some(Unit::from_suffix(suffix)?)
        };
        Some(Len {
            scaled: number.scaled,
            unit,
        })
    }

    /// True when the text is a number that [`Len::parse`] reads.
    pub fn is_number(text: &str) -> bool {
        Len::parse(text).is_some()
    }

    /// The length in nanometres, with `current` for a number without a unit and `grid_nm` for the
    /// grid unit. `None` when it does not fit the range of the model.
    pub fn to_nm(self, current: Unit, grid_nm: i64) -> Option<Nm> {
        let unit = self.unit.unwrap_or(current);
        let product = self.scaled.checked_mul(unit.nm(grid_nm))?;
        Nm::from_i128(round_div(product, 1_000_000)).ok()
    }
}

/// A rotation as written: `R90`, `MR90`, `R-45` or `=R90` (absolute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rot {
    /// The angle in micro-degrees.
    pub angle: Udeg,
    /// Written with an M: mirrored.
    pub mirror: bool,
    /// Written with an equals sign: the angle replaces the current one.
    pub absolute: bool,
}

impl Rot {
    /// Reads a rotation word, `None` when the word is not one.
    pub fn parse(word: &str) -> Option<Rot> {
        let mut rest = word;
        let mut absolute = false;
        if let Some(r) = rest.strip_prefix('=') {
            absolute = true;
            rest = r;
        }
        let mut mirror = false;
        if let Some(r) = rest.strip_prefix(['M', 'm']) {
            mirror = true;
            rest = r;
        }
        let digits = rest.strip_prefix(['R', 'r'])?;
        if digits.is_empty() {
            return None;
        }
        let (angle, _exact) = Udeg::parse_deg(digits).ok()?;
        Some(Rot {
            angle,
            mirror,
            absolute,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nm(text: &str, current: Unit) -> Option<i64> {
        Len::parse(text)
            .and_then(|l| l.to_nm(current, 1_270_000))
            .map(Nm::get)
    }

    #[test]
    fn every_unit_converts_exactly() {
        assert_eq!(nm("1mm", Unit::Mil), Some(1_000_000));
        assert_eq!(nm("8mil", Unit::Mm), Some(203_200));
        assert_eq!(nm("1inch", Unit::Mm), Some(25_400_000));
        assert_eq!(nm("25micron", Unit::Mm), Some(25_000));
        assert_eq!(nm("2grid", Unit::Mm), Some(2_540_000));
        assert_eq!(nm("0.5", Unit::Mm), Some(500_000));
        assert_eq!(nm("100", Unit::Mil), Some(2_540_000));
        assert_eq!(nm("-0.25MM", Unit::Inch), Some(-250_000));
        assert_eq!(nm("+3", Unit::Micron), Some(3_000));
    }

    #[test]
    fn a_number_that_does_not_divide_is_rounded_half_away_from_zero_once() {
        // 0.0005 micron is 0.5 nm
        assert_eq!(nm("0.0005micron", Unit::Mm), Some(1));
        assert_eq!(nm("-0.0005micron", Unit::Mm), Some(-1));
        assert_eq!(nm("0.0004micron", Unit::Mm), Some(0));
        assert_eq!(nm("0.0015micron", Unit::Mm), Some(2));
        // 0.000001 inch is 25.4 nm
        assert_eq!(nm("0.000001inch", Unit::Mm), Some(25));
    }

    #[test]
    fn text_that_is_not_a_length_is_refused() {
        for bad in ["", "-", ".", "mm", "1.2.3", "5furlong", "1e3", "--1", "abc"] {
            assert!(Len::parse(bad).is_none(), "{bad}");
        }
        assert!(Len::is_number("12"));
        assert!(!Len::is_number("GND"));
    }

    #[test]
    fn a_length_outside_the_range_does_not_fit() {
        assert_eq!(nm("2000mm", Unit::Mm), None);
        assert_eq!(nm("99999999999999999999999999999999inch", Unit::Mm), None);
    }

    #[test]
    fn rotations_are_read_with_their_flags() {
        let r = Rot::parse("R90").unwrap();
        assert_eq!(r.angle.get(), 90_000_000);
        assert!(!r.mirror && !r.absolute);
        let m = Rot::parse("MR180").unwrap();
        assert!(m.mirror && !m.absolute);
        let a = Rot::parse("=R-45").unwrap();
        assert_eq!(a.angle.get(), -45_000_000);
        assert!(a.absolute);
        assert!(Rot::parse("90").is_none());
        assert!(Rot::parse("R").is_none());
        assert!(Rot::parse("RX").is_none());
        assert!(Rot::parse("GND").is_none());
    }

    #[test]
    fn units_are_read_by_word_and_by_suffix() {
        assert_eq!(Unit::from_word("MM"), Some(Unit::Mm));
        assert_eq!(Unit::from_word("in"), Some(Unit::Inch));
        assert_eq!(Unit::from_word("grid"), None);
        assert_eq!(Unit::from_suffix("Grid"), Some(Unit::Grid));
        assert_eq!(Unit::Micron.name(), "micron");
    }
}
