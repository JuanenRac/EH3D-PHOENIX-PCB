// SPDX-License-Identifier: GPL-3.0-or-later
//! The diameter of a pad or a via that the file leaves to the design rules.
//!
//! A board writes no diameter (or zero) for the pads and vias whose copper follows the rules of
//! the restring: a share of the drill, kept between a smallest and a largest width, on each side
//! of the drill.

use eh3d_phoenix_eagle::xml::Element;
use eh3d_phoenix_model::Nm;

use crate::parse_nm;

/// One mil, in nanometres.
const MIL: f64 = 25_400.0;

/// The ring around a drill.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Restring {
    share: f64,
    smallest: i64,
    largest: i64,
}

/// A length of the design rules: `10mil`, `0.254mm`.
fn rule_length(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Some(mils) = text.strip_suffix("mil") {
        let value: f64 = mils.trim().parse().ok()?;
        return (0.0..1e6)
            .contains(&value)
            .then(|| (value * MIL).round() as i64);
    }
    parse_nm(text.strip_suffix("mm")?)
        .map(Nm::get)
        .filter(|v| *v >= 0)
}

impl Restring {
    /// The ring of the rules of a board with the given names, or the usual one: a quarter of the
    /// drill between `smallest_mil` and 20 mil.
    pub(crate) fn of(board: &Element, names: [&str; 3], smallest_mil: f64) -> Restring {
        let value = |wanted: &str| {
            board
                .child("designrules")?
                .children_named("param")
                .find(|p| p.attr("name") == Some(wanted))?
                .attr("value")
        };
        let [share, smallest, largest] = names;
        Restring {
            share: value(share)
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|v| (0.0..=10.0).contains(v))
                .unwrap_or(0.25),
            smallest: value(smallest)
                .and_then(rule_length)
                .unwrap_or((smallest_mil * MIL) as i64),
            largest: value(largest)
                .and_then(rule_length)
                .unwrap_or((20.0 * MIL) as i64),
        }
    }

    /// The diameter of the file when it gives one, else the one the rules give for the drill.
    pub(crate) fn diameter(self, given: Option<Nm>, drill: Nm) -> Option<Nm> {
        if let Some(given) = given.filter(|d| d.get() > 0) {
            return Some(given);
        }
        let ring = ((drill.get() as f64 * self.share).round() as i64)
            .clamp(self.smallest, self.largest.max(self.smallest));
        Nm::new(drill.get().checked_add(ring.checked_mul(2)?)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_eagle::xml::{parse, Limits};

    fn mm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    #[test]
    fn the_ring_follows_the_rules_and_its_limits() {
        let board = parse(
            r#"<board><designrules name="d"><param name="rv" value="0.25"/>
<param name="lo" value="10mil"/><param name="hi" value="0.508mm"/></designrules></board>"#,
            &Limits::default(),
        )
        .unwrap();
        let ring = Restring::of(&board, ["rv", "lo", "hi"], 8.0);
        // a small drill takes the smallest ring, a large one the largest
        assert_eq!(ring.diameter(None, mm(600_000)), Some(mm(1_108_000)));
        assert_eq!(ring.diameter(None, mm(1_600_000)), Some(mm(2_400_000)));
        assert_eq!(
            ring.diameter(Some(Nm::ZERO), mm(4_000_000)),
            Some(mm(5_016_000))
        );
        assert_eq!(
            ring.diameter(Some(mm(900_000)), mm(600_000)),
            Some(mm(900_000))
        );
        let usual = Restring::of(
            &parse("<board/>", &Limits::default()).unwrap(),
            ["rv", "lo", "hi"],
            8.0,
        );
        assert_eq!(usual.diameter(None, mm(400_000)), Some(mm(806_400)));
    }
}
