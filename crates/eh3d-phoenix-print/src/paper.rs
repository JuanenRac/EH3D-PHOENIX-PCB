// SPDX-License-Identifier: GPL-3.0-or-later
//! The sizes of paper: A4, A3, Letter and a custom size in millimetres.

use crate::codes::{PrintError, PAPER_INVALID};

/// The smallest side of a custom paper, in millimetres.
pub const MIN_SIDE_MM: f64 = 50.0;
/// The largest side of a custom paper, in millimetres.
pub const MAX_SIDE_MM: f64 = 5000.0;

/// A size of paper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Paper {
    /// 210 by 297 millimetres.
    A4,
    /// 297 by 420 millimetres.
    A3,
    /// 215.9 by 279.4 millimetres.
    Letter,
    /// A size in millimetres, the short side first.
    Custom {
        /// The short side.
        short_mm: f64,
        /// The long side.
        long_mm: f64,
    },
}

impl Paper {
    /// Reads `A4`, `A3`, `Letter` (in any case) or `WIDTHxHEIGHT` in millimetres.
    pub fn parse(text: &str) -> Result<Paper, PrintError> {
        let lower = text.trim().to_ascii_lowercase();
        match lower.as_str() {
            "a4" => return Ok(Paper::A4),
            "a3" => return Ok(Paper::A3),
            "letter" => return Ok(Paper::Letter),
            _ => {}
        }
        let bad = || PrintError::new(PAPER_INVALID, text.trim());
        let (w, h) = lower.split_once('x').ok_or_else(bad)?;
        let w: f64 = w.trim().parse().map_err(|_| bad())?;
        let h: f64 = h.trim().parse().map_err(|_| bad())?;
        let sides = MIN_SIDE_MM..=MAX_SIDE_MM;
        if !sides.contains(&w) || !sides.contains(&h) {
            return Err(PrintError::new(
                PAPER_INVALID,
                format!(
                    "{}: each side must be from {MIN_SIDE_MM} to {MAX_SIDE_MM} mm",
                    text.trim()
                ),
            ));
        }
        Ok(Paper::Custom {
            short_mm: w.min(h),
            long_mm: w.max(h),
        })
    }

    /// The short and the long side in millimetres.
    pub fn sides(self) -> (f64, f64) {
        match self {
            Paper::A4 => (210.0, 297.0),
            Paper::A3 => (297.0, 420.0),
            Paper::Letter => (215.9, 279.4),
            Paper::Custom { short_mm, long_mm } => (short_mm, long_mm),
        }
    }

    /// The name, as it is written in the title block.
    pub fn name(self) -> String {
        match self {
            Paper::A4 => "A4".to_owned(),
            Paper::A3 => "A3".to_owned(),
            Paper::Letter => "Letter".to_owned(),
            Paper::Custom { short_mm, long_mm } => {
                format!(
                    "{}x{}",
                    crate::draw::num(short_mm),
                    crate::draw::num(long_mm)
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_sizes_and_the_custom_one_are_read() {
        assert_eq!(Paper::parse("a4").unwrap(), Paper::A4);
        assert_eq!(Paper::parse(" A3 ").unwrap(), Paper::A3);
        assert_eq!(Paper::parse("LETTER").unwrap(), Paper::Letter);
        let custom = Paper::parse("400x300").unwrap();
        assert_eq!(custom.sides(), (300.0, 400.0));
        assert_eq!(custom.name(), "300x400");
        assert_eq!(Paper::A4.sides(), (210.0, 297.0));
        assert_eq!(Paper::Letter.name(), "Letter");
    }

    #[test]
    fn a_size_that_is_not_valid_is_refused() {
        for bad in [
            "", "A5", "10x10", "300", "axb", "6000x300", "300x-4", "nanx300",
        ] {
            let error = Paper::parse(bad).unwrap_err();
            assert_eq!(error.code, PAPER_INVALID, "{bad}");
        }
    }
}
