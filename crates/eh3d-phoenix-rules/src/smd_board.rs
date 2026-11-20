// SPDX-License-Identifier: GPL-3.0-or-later
//! The spacing of surface-mount parts, judged on a board: the parts and the box of their pads are
//! taken from the placed pads, and [`crate::smd_spacing`] finds the pairs that are closer than the
//! table of the profile asks (`smd_spacing = NAME`, `PHOENIX-DFM-020`).
//!
//! A part counts when none of its pads has a drill and its footprint name is one of the ten
//! types of the table; the others are left out of this check. The box of a part grows each pad by
//! half of its longer side, whatever the turn of the part, so it is never smaller than the pads.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, PadShape, Side};

use crate::dfm::DfmProfile;
use crate::items::placed_pads;
use crate::report::{Finding, Severity};
use crate::smd_spacing::{builtin_table, classify_footprint, find_violations, SmdPart};

fn half_extent(shape: PadShape) -> i64 {
    let longest = match shape {
        PadShape::Circle { diameter } => diameter.get(),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            width.get().max(height.get())
        }
    };
    longest / 2 + longest % 2
}

/// The surface-mount parts of a board that the table can judge, with the box of their pads, in the
/// order of their reference designators.
pub(crate) fn parts_of(board: &Board) -> Vec<SmdPart> {
    // reference designator -> (min x, min y, max x, max y, has a drilled pad)
    let mut boxes: BTreeMap<String, (i64, i64, i64, i64, bool)> = BTreeMap::new();
    for pad in placed_pads(board) {
        let half = half_extent(pad.shape);
        let (x, y) = (pad.at.x_nm.get(), pad.at.y_nm.get());
        let entry = boxes.entry(pad.refdes.clone()).or_insert((
            i64::MAX,
            i64::MAX,
            i64::MIN,
            i64::MIN,
            false,
        ));
        entry.0 = entry.0.min(x.saturating_sub(half));
        entry.1 = entry.1.min(y.saturating_sub(half));
        entry.2 = entry.2.max(x.saturating_add(half));
        entry.3 = entry.3.max(y.saturating_add(half));
        entry.4 |= pad.drill.is_some();
    }
    let mut parts = Vec::new();
    for component in board.components.values() {
        let Some(&(min_x, min_y, max_x, max_y, through)) = boxes.get(&component.refdes) else {
            continue;
        };
        if through {
            continue;
        }
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let Some(class) = classify_footprint(&footprint.name) else {
            continue;
        };
        parts.push(SmdPart {
            refdes: component.refdes.clone(),
            class,
            bottom: matches!(component.side, Side::Bottom),
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }
    parts.sort_by(|a, b| a.refdes.cmp(&b.refdes));
    parts
}

/// The findings of `PHOENIX-DFM-020` for the table the profile names; nothing when it names none.
pub(crate) fn findings(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if profile.smd_spacing.is_empty() {
        return;
    }
    let Some(table) = builtin_table(&profile.smd_spacing) else {
        return;
    };
    for v in find_violations(&parts_of(board), &table) {
        out.push(
            Finding::new(
                59,
                "dfm_parts_close",
                crate::dfm_more::PARTS_TOO_CLOSE,
                Severity::Warning,
                format!("{} {}", v.a, v.b),
            )
            .at(v.at.0, v.at.1)
            .values(v.gap_nm, v.required_nm)
            .why(&format!(
                "{} and {} on the table {}",
                v.class_a.key(),
                v.class_b.key(),
                table.name
            )),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dfm::tests::{id, pt};
    use eh3d_phoenix_board::{Component, Footprint, Layer, LayerKind, Pad};
    use eh3d_phoenix_model::{Nm, Udeg};

    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    fn footprint(name: &str, drill: Option<i64>) -> Footprint {
        let mut pads = BTreeMap::new();
        for (n, x) in [(1u32, -400_000), (2, 400_000)] {
            pads.insert(
                id(900 + n),
                Pad {
                    number: n.to_string(),
                    shape: PadShape::Rect {
                        width: nm(600_000),
                        height: nm(500_000),
                    },
                    at: pt(x, 0),
                    rotation: Udeg::ZERO,
                    drill: drill.map(nm),
                    layers: vec![id(1)],
                    roundness: 0,
                    plated: true,
                },
            );
        }
        Footprint {
            name: name.to_owned(),
            pads,
        }
    }

    fn board_with(parts: &[(&str, &str, i64, i64, Side, Option<i64>)]) -> Board {
        let mut b = Board::default();
        b.layers.insert(
            id(1),
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack = vec![id(1)];
        for (i, (refdes, fp, x, y, side, drill)) in parts.iter().enumerate() {
            let fid = id(100 + i as u32);
            b.footprints.insert(fid, footprint(fp, *drill));
            b.components.insert(
                id(200 + i as u32),
                Component {
                    refdes: (*refdes).to_owned(),
                    value: String::new(),
                    symbol: None,
                    footprint: fid,
                    at: pt(*x, *y),
                    rotation: Udeg::ZERO,
                    side: *side,
                },
            );
        }
        b
    }

    fn profile() -> DfmProfile {
        DfmProfile::parse("verified = true\nsmd_spacing = jlcpcb\n").unwrap()
    }

    #[test]
    fn the_box_of_a_part_is_the_box_of_its_pads_grown_by_half_of_the_longer_side() {
        let board = board_with(&[(
            "R1",
            "R_0603_1608Metric",
            10_000_000,
            5_000_000,
            Side::Top,
            None,
        )]);
        let parts = parts_of(&board);
        assert_eq!(parts.len(), 1);
        // pads at x -0.4 and +0.4 mm, longer side 0.6 mm: half 0.3 mm
        assert_eq!(
            (
                parts[0].min_x,
                parts[0].max_x,
                parts[0].min_y,
                parts[0].max_y
            ),
            (9_300_000, 10_700_000, 4_700_000, 5_300_000)
        );
        assert!(!parts[0].bottom);
    }

    #[test]
    fn through_hole_parts_and_unknown_footprints_are_not_judged() {
        let board = board_with(&[
            ("J1", "QFN-16", 0, 0, Side::Top, Some(300_000)),
            ("X1", "AXIAL", 5_000_000, 0, Side::Top, None),
            ("U1", "QFN-16", 10_000_000, 0, Side::Top, None),
        ]);
        let parts = parts_of(&board);
        assert_eq!(
            parts.iter().map(|p| p.refdes.as_str()).collect::<Vec<_>>(),
            ["U1"]
        );
    }

    #[test]
    fn a_chip_next_to_a_no_lead_body_is_found_and_a_far_one_is_not() {
        // the pads of the QFN reach x = 0.7 mm and those of the chip start at 0.9 mm: 0.2 mm apart
        let close = board_with(&[
            ("U1", "QFN-16", 0, 0, Side::Top, None),
            ("C1", "C_0402_1005Metric", 1_600_000, 0, Side::Top, None),
        ]);
        let mut out = Vec::new();
        findings(&close, &profile(), &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, crate::dfm_more::PARTS_TOO_CLOSE);
        assert_eq!(out[0].location, "C1 U1");
        assert_eq!(out[0].limit_nm, Some(1_000_000));
        assert_eq!(out[0].measured_nm, Some(200_000));
        assert!(out[0].detail.contains("0402 and qfn"), "{}", out[0].detail);
        let far = board_with(&[
            ("U1", "QFN-16", 0, 0, Side::Top, None),
            ("C1", "C_0402_1005Metric", 5_000_000, 0, Side::Top, None),
        ]);
        let mut none = Vec::new();
        findings(&far, &profile(), &mut none);
        assert!(none.is_empty());
        let other_side = board_with(&[
            ("U1", "QFN-16", 0, 0, Side::Top, None),
            ("C1", "C_0402_1005Metric", 1_600_000, 0, Side::Bottom, None),
        ]);
        findings(&other_side, &profile(), &mut none);
        assert!(none.is_empty());
    }

    #[test]
    fn the_check_runs_with_the_profile_and_is_off_without_the_line() {
        let board = board_with(&[
            ("U1", "QFN-16", 0, 0, Side::Top, None),
            ("C1", "C_0402_1005Metric", 1_600_000, 0, Side::Top, None),
        ]);
        let codes = |text: &str| -> Vec<&'static str> {
            let p = DfmProfile::parse(&format!("verified = true\n{text}")).unwrap();
            crate::dfm::dfm_findings(&board, &p)
                .confirmed
                .iter()
                .map(|f| f.code)
                .collect()
        };
        assert_eq!(
            codes("smd_spacing = jlcpcb\n"),
            [crate::dfm_more::PARTS_TOO_CLOSE]
        );
        assert!(codes("").is_empty());
        let p = DfmProfile::parse("smd_spacing = nobody\n");
        assert!(p.unwrap_err().detail.contains("no table called nobody"));
    }
}
