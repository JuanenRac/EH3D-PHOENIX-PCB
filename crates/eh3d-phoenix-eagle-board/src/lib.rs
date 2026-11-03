//! A board of the design model as an EAGLE XML file, and back.
//!
//! * [`write_board`] writes a [`Board`] as an EAGLE 9 board file: layers, the board outline, one
//!   package per footprint, one element per component, one signal per net with its contacts, wires,
//!   vias and polygons, and the silkscreen as wires on the silkscreen layers;
//! * [`read_board`] reads such a file into a board, giving the new objects ids from the caller;
//! * [`write_library`] writes the footprints of a board and the symbols of a schematic as a library;
//! * [`signature`] describes a board without its ids, so that a board and the board read back from
//!   its file can be compared ([`round_trip`]).
//!
//! What the EAGLE format cannot say about the model is not lost silently: every item that is
//! written in a simpler form (`PHOENIX-EAGLE-020`) or left out (`PHOENIX-EAGLE-021`) is reported.
//! Ids, the grouping of wires into tracks, silkscreen texts (written as the strokes that are
//! printed), the names of layers and the stack order when it was not given are not kept; the
//! signature is built so that these differences do not count and everything else does.
//!
//! The mapping is written from the format as documented and has not been checked against the
//! reference tool: no EAGLE has opened a file written here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod codes;
mod layers;
mod library;
mod reader;
mod signature;
mod writer;

pub use codes::{diagnostic, ITEM_LEFT_OUT, ITEM_SIMPLIFIED, NOT_READ, REGISTRY, UNREADABLE};
pub use library::write_library;
pub use reader::{read_board, ReadBoard};
pub use signature::{round_trip, signature, RoundTrip};
pub use writer::{write_board, Written};

use eh3d_phoenix_model::{Nm, Udeg};

/// A scaled integer (nanometres or micro-degrees) as a decimal with at most six decimals,
/// without trailing zeros; the exact text of the value.
pub(crate) fn decimal(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    let (int, frac) = (abs / 1_000_000, abs % 1_000_000);
    if frac == 0 {
        format!("{sign}{int}")
    } else {
        let text = format!("{frac:06}");
        format!("{sign}{int}.{}", text.trim_end_matches('0'))
    }
}

pub(crate) fn parse_nm(text: &str) -> Option<Nm> {
    Nm::parse_mm(text.trim()).ok().map(|(v, _)| v)
}

pub(crate) fn parse_udeg(text: &str) -> Option<Udeg> {
    Udeg::parse_deg(text.trim()).ok().map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_are_exact_and_short() {
        assert_eq!(decimal(0), "0");
        assert_eq!(decimal(1_000_000), "1");
        assert_eq!(decimal(1_500_000), "1.5");
        assert_eq!(decimal(-250_000), "-0.25");
        assert_eq!(decimal(1), "0.000001");
        assert_eq!(decimal(-1_000_001), "-1.000001");
        for v in [0, 1, -1, 999_999, 1_234_567, -7_000_000, 1 << 29] {
            assert_eq!(parse_nm(&decimal(v)).unwrap().get(), v);
        }
        assert!(parse_nm("abc").is_none());
    }
}
