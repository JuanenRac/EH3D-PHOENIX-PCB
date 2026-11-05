//! The marks of the drawing that are not wires: net labels, junctions and power ports.
//!
//! * a **net label** names the net of the wire point or the pin it sits on; labels with the same
//!   name join their nets even when no wire runs between them;
//! * a **power port** is a label of a supply (`GND`, `VCC`) whose name is global in the same way;
//! * a **junction** is a dot at a point where three or more wire pieces or pins meet. It carries no
//!   meaning of its own: wires are joined by sharing a point, as in `draw`, and the editor puts a
//!   junction where that happens so the drawing shows it.
//!
//! None of them is written when a schematic has none, so schematics without marks keep their bytes.

use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::model::Id;

/// The longest name of a net label or a power port, in characters.
pub const MAX_NAME_CHARS: usize = 128;

/// A net label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetLabel {
    /// The sheet.
    pub sheet: Id,
    /// The point it names.
    pub at: Point,
    /// The name of the net.
    pub name: String,
}

/// A power port: the global name of a supply net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerPort {
    /// The sheet.
    pub sheet: Id,
    /// The point it sits on.
    pub at: Point,
    /// The name of the net (`GND`, `VCC`).
    pub name: String,
}

/// A junction dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Junction {
    /// The sheet.
    pub sheet: Id,
    /// Where the dot is.
    pub at: Point,
}

/// The names of the members of a simple bus such as `D[0..7]` (`D0` to `D7`). A bus is a way of
/// naming: its members are ordinary nets. The prefix may be empty or any text without brackets;
/// the range may run up or down; at most 256 members.
///
/// Returns `None` when the text is not of the form `prefix[first..last]`.
pub fn bus_members(spec: &str) -> Option<Vec<String>> {
    let spec = spec.trim();
    let open = spec.find('[')?;
    let inner = spec.get(open + 1..)?.strip_suffix(']')?;
    let prefix = &spec[..open];
    if prefix.contains(['[', ']']) || prefix.is_empty() {
        return None;
    }
    let (first, last) = inner.split_once("..")?;
    let first: i64 = first.trim().parse().ok()?;
    let last: i64 = last.trim().parse().ok()?;
    if first < 0 || last < 0 || first.abs_diff(last) >= 256 {
        return None;
    }
    let numbers: Vec<i64> = if first <= last {
        (first..=last).collect()
    } else {
        (last..=first).rev().collect()
    };
    Some(
        numbers
            .into_iter()
            .map(|n| format!("{prefix}{n}"))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bus_lists_its_members_up_or_down() {
        assert_eq!(
            bus_members("D[0..3]"),
            Some(vec!["D0".into(), "D1".into(), "D2".into(), "D3".into()])
        );
        assert_eq!(
            bus_members(" A[2..0] "),
            Some(vec!["A2".into(), "A1".into(), "A0".into()])
        );
        assert_eq!(bus_members("X[5..5]"), Some(vec!["X5".to_owned()]));
    }

    #[test]
    fn a_text_that_is_not_a_bus_is_refused() {
        for bad in [
            "D",
            "D[]",
            "D[1]",
            "D[a..b]",
            "[0..3]",
            "D[0..3",
            "D[-1..2]",
            "D[0..300]",
            "D[[0..3]",
        ] {
            assert_eq!(bus_members(bad), None, "{bad}");
        }
    }
}
