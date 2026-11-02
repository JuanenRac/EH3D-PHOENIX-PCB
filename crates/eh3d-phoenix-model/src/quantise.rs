//! Quantisation of exchange values before they are written.
//!
//! Lengths in millimetres are quantised to 1 nm (six decimals) and angles in degrees to 1
//! micro-degree (six decimals), with ties going to the even neighbour. After that the canonical
//! JSON writer gives one text form per value (`12.4`, `180`, `0.000001`, never an exponent in
//! this range). The design domain does not use this: it stores integers.

use crate::error::CanonicalError;
use crate::units::Nm;

/// Quantisation step as a power of ten: six decimals.
pub const SCALE: f64 = 1_000_000.0;
/// A value whose scaled form is further than this from an integer lost information
/// (floating-point noise of an exact decimal is far below it for board-sized values).
pub const LOSSY_TOLERANCE: f64 = 1e-4;
/// Values with a larger scaled magnitude are not exact in a double.
const MAX_SCALED: f64 = 9_007_199_254_740_992.0;

/// A quantised value and whether quantising changed it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantised {
    /// The value rounded to six decimals.
    pub value: f64,
    /// True when digits beyond the sixth decimal were present (the caller reports it).
    pub lossy: bool,
}

/// Rounds a millimetre or degree value to six decimals, ties to even.
pub fn quantise6(value: f64) -> Result<Quantised, CanonicalError> {
    if !value.is_finite() {
        return Err(CanonicalError::NotFinite);
    }
    let scaled = value * SCALE;
    if scaled.abs() >= MAX_SCALED {
        return Err(CanonicalError::QuantiseRange(value.to_string()));
    }
    let rounded = scaled.round_ties_even();
    // Rounding can produce negative zero; the canonical writer prints it as 0 either way.
    Ok(Quantised {
        value: rounded / SCALE,
        lossy: (scaled - rounded).abs() > LOSSY_TOLERANCE,
    })
}

/// The exact millimetre value of a length, ready for a canonical writer.
pub fn nm_to_mm(length: Nm) -> f64 {
    length.get() as f64 / SCALE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::es_number_string;

    #[test]
    fn exact_decimals_are_not_lossy_and_print_short() {
        for (v, text) in [
            (12.4, "12.4"),
            (180.0, "180"),
            (0.000001, "0.000001"),
            (-3.25, "-3.25"),
            (0.1 + 0.2, "0.3"),
        ] {
            let q = quantise6(v).unwrap();
            assert_eq!(es_number_string(q.value).unwrap(), text, "{v}");
            assert!(!q.lossy, "{v}");
        }
    }

    #[test]
    fn extra_digits_are_rounded_and_flagged() {
        let q = quantise6(1.0000004).unwrap();
        assert_eq!(es_number_string(q.value).unwrap(), "1");
        assert!(q.lossy);
        let q = quantise6(1.0000006).unwrap();
        assert_eq!(es_number_string(q.value).unwrap(), "1.000001");
        assert!(q.lossy);
        assert_eq!(
            es_number_string(quantise6(-0.0000004).unwrap().value).unwrap(),
            "0"
        );
    }

    #[test]
    fn non_finite_and_huge_values_are_errors() {
        assert_eq!(quantise6(f64::NAN), Err(CanonicalError::NotFinite));
        assert_eq!(quantise6(f64::NEG_INFINITY), Err(CanonicalError::NotFinite));
        assert!(matches!(
            quantise6(1e12),
            Err(CanonicalError::QuantiseRange(_))
        ));
        assert!(quantise6(1e9).is_ok());
    }

    #[test]
    fn quantising_is_idempotent() {
        let mut x = 0.123_456_789_f64;
        for _ in 0..2000 {
            x = (x * 7919.0 + 0.123_456_7).fract() * 2000.0 - 1000.0;
            let once = quantise6(x).unwrap().value;
            let twice = quantise6(once).unwrap();
            assert_eq!(once, twice.value);
            assert!(!twice.lossy);
        }
    }

    #[test]
    fn every_nanometre_prints_with_at_most_six_decimals() {
        // The division of an integer by 10^6 is correctly rounded, so its shortest form is the
        // decimal with six places at most. Walk a spread of the whole range.
        let mut n: i64 = -(crate::units::COORD_LIMIT_NM - 1);
        while n < crate::units::COORD_LIMIT_NM {
            let mm = nm_to_mm(Nm::new(n).unwrap());
            let text = es_number_string(mm).unwrap();
            assert!(!text.contains('e'), "{text}");
            let decimals = text.split_once('.').map_or(0, |(_, f)| f.len());
            assert!(decimals <= 6, "{text}");
            // Reading the text back and rescaling returns the same integer.
            let (back, exact) = Nm::parse_mm(&text).unwrap();
            assert!(exact);
            assert_eq!(back.get(), n, "{text}");
            n += 1_234_567;
        }
    }
}
