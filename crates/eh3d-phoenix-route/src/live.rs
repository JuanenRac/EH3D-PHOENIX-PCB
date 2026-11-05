// SPDX-License-Identifier: GPL-3.0-or-later
//! The live check: the findings of the rules on a board that are proven (the confirmed list of the
//! rule check, of the net class check and the copper that touches copper of another net), and the
//! difference between two boards, so that an edit can say what it adds and what it removes.

use std::collections::BTreeSet;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::{
    analyse_board, check_board, check_classes, codes, Finding, NetClasses, RuleValues, Severity,
};

/// The rules a board is edited under: the limits and the net classes.
#[derive(Debug, Clone, Default)]
pub struct RuleContext {
    /// The limits of the rule file.
    pub rules: RuleValues,
    /// The net classes of the rule file (the width of the tracks and the size of the vias of
    /// their nets).
    pub classes: NetClasses,
}

/// The proven findings of the rules on a board, in a fixed order.
pub fn findings(board: &Board, ctx: &RuleContext) -> Vec<Finding> {
    let mut report = check_board(board, &ctx.rules);
    if !ctx.classes.is_empty() {
        report.merge(check_classes(board, &ctx.classes));
    }
    let mut out = report.confirmed;
    for finding in analyse_board(board, ctx.rules.connect_tolerance).confirmed {
        if finding.code == codes::COPPER_SHORT {
            out.push(finding);
        }
    }
    out.sort_by(|a, b| {
        (a.rank, &a.location, a.at_nm, a.code).cmp(&(b.rank, &b.location, b.at_nm, b.code))
    });
    out
}

/// What changed between two lists of findings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveDiff {
    /// Findings that the second list has and the first has not.
    pub added: Vec<Finding>,
    /// Findings that the first list has and the second has not.
    pub removed: Vec<Finding>,
}

impl LiveDiff {
    /// True when something was added that is an error.
    pub fn adds_error(&self) -> bool {
        self.added.iter().any(|f| f.severity == Severity::Error)
    }
}

fn key(finding: &Finding) -> String {
    format!("{finding:?}")
}

/// The findings that appeared and the findings that went away.
pub fn diff(before: &[Finding], after: &[Finding]) -> LiveDiff {
    let old: BTreeSet<String> = before.iter().map(key).collect();
    let new: BTreeSet<String> = after.iter().map(key).collect();
    LiveDiff {
        added: after
            .iter()
            .filter(|f| !old.contains(&key(f)))
            .cloned()
            .collect(),
        removed: before
            .iter()
            .filter(|f| !new.contains(&key(f)))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Id, Layer, LayerKind, Net, Track};
    use eh3d_phoenix_board::{Segment, Side};
    use eh3d_phoenix_model::{Nm, ObjectId, Point};

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn board_with_track(width: i64) -> Board {
        let mut b = Board::default();
        b.layers.insert(
            id(1),
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: Some(Side::Top),
            },
        );
        b.nets.insert(id(2), Net { name: "A".into() });
        b.tracks.insert(
            id(3),
            Track {
                net: id(2),
                layer: id(1),
                width: Nm::new(width).unwrap(),
                path: vec![Segment::Line {
                    start: Point::from_nm(0, 0).unwrap(),
                    end: Point::from_nm(1_000_000, 0).unwrap(),
                }],
            },
        );
        b
    }

    #[test]
    fn a_narrow_track_is_a_finding_and_widening_it_removes_it() {
        let ctx = RuleContext {
            rules: RuleValues {
                min_track_width: Nm::new(200_000).unwrap(),
                ..RuleValues::default()
            },
            classes: NetClasses::default(),
        };
        let narrow = findings(&board_with_track(100_000), &ctx);
        let wide = findings(&board_with_track(300_000), &ctx);
        let is_narrow = |f: &Finding| f.code == codes::TRACK_NARROW;
        assert_eq!(narrow.iter().filter(|f| is_narrow(f)).count(), 1);
        assert_eq!(wide.iter().filter(|f| is_narrow(f)).count(), 0);
        let d = diff(&narrow, &wide);
        assert!(d.added.is_empty() && d.removed.iter().any(is_narrow));
        assert!(!d.adds_error());
        let back = diff(&wide, &narrow);
        assert!(back.removed.is_empty() && back.adds_error());
        assert_eq!(diff(&narrow, &narrow), LiveDiff::default());
    }
}
