// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced check of the strips of solder mask between two openings (`PHOENIX-DRC-022`).
//!
//! The opening of the mask over a pad is the box of the pad (its width and height, turned by the
//! angle of the pad) grown by `mask_sliver_expansion_nm` on every side; the pads that appear on a
//! layer of solder mask are the openings of that layer. [`crate::mask::mask_slivers`] gives each
//! pair of openings whose gap is more than nothing and less than `mask_sliver_min_nm`. A pad is
//! a rectangle with square corners only when it is a rectangle at a quarter turn; a round pad, an
//! oblong pad and a rectangle with round corners are taken as their box, which can only make a
//! gap look smaller than it is, so a pair with one of them is told as information in the
//! inconclusive list and never as a proven finding. A pad at an angle that is not a quarter turn
//! is not judged. The check is off in the settings that come with the program.

use eh3d_phoenix_board::{Board, LayerKind, PadShape};

use super::checks::both;
use super::{make, not_judged, AdvancedConfig, R_MASK_SLIVER};
use crate::items::PlacedPad;
use crate::mask::{mask_slivers, MaskOpening};
use crate::report::{Report, Severity};

const QUARTER_TURN_UDEG: i64 = 90_000_000;

/// The opening over a pad: its box grown by `expansion_nm`, and whether the pad is that box
/// exactly. `None` for a pad at an angle that is not a quarter turn.
pub(super) fn opening_of(pad: &PlacedPad, expansion_nm: i64) -> Option<(MaskOpening, bool)> {
    if pad.angle_udeg % QUARTER_TURN_UDEG != 0 {
        return None;
    }
    let (width, height, square) = match pad.shape {
        PadShape::Circle { diameter } => (diameter.get(), diameter.get(), false),
        PadShape::Rect { width, height } => (width.get(), height.get(), pad.roundness == 0),
        PadShape::Oblong { width, height } => (width.get(), height.get(), false),
    };
    let (width, height) = if (pad.angle_udeg / QUARTER_TURN_UDEG) % 2 == 1 {
        (height, width)
    } else {
        (width, height)
    };
    let (x, y) = (pad.at.x_nm.get(), pad.at.y_nm.get());
    let half_x = width / 2 + width % 2 + expansion_nm;
    let half_y = height / 2 + height % 2 + expansion_nm;
    Some((
        MaskOpening {
            x1_nm: x - half_x,
            y1_nm: y - half_y,
            x2_nm: x + half_x,
            y2_nm: y + half_y,
        },
        square,
    ))
}

/// The strips of mask that are too narrow (`PHOENIX-DRC-022`), layer of mask by layer of mask.
pub(super) fn mask_sliver_findings(
    board: &Board,
    pads: &[PlacedPad],
    cfg: &AdvancedConfig,
    out: &mut Report,
) {
    let Some(severity) = cfg.mask_sliver_level else {
        return;
    };
    let min = cfg.mask_sliver_min_nm;
    if min <= 0 {
        return;
    }
    for (layer_id, layer) in &board.layers {
        if layer.kind != LayerKind::SolderMask {
            continue;
        }
        let mut openings = Vec::new();
        let mut owners: Vec<(&PlacedPad, bool)> = Vec::new();
        for pad in pads.iter().filter(|p| p.layers.contains(layer_id)) {
            match opening_of(pad, cfg.mask_sliver_expansion_nm) {
                Some((opening, exact)) => {
                    openings.push(opening);
                    owners.push((pad, exact));
                }
                None => out.inconclusive.push(not_judged(
                    pad.label.clone(),
                    &format!(
                        "its opening on {} is under an angle that is not a quarter turn",
                        layer.name
                    ),
                )),
            }
        }
        for sliver in mask_slivers(&openings, min) {
            let (a, exact_a) = owners[sliver.first];
            let (b, exact_b) = owners[sliver.second];
            let gap = i64::try_from(sliver.gap_nm).unwrap_or(i64::MAX);
            let place = both(&a.label, &b.label);
            let (x, y) = (a.at.x_nm.get(), a.at.y_nm.get());
            if exact_a && exact_b {
                out.confirmed.push(
                    make(&R_MASK_SLIVER, severity, place)
                        .at(x, y)
                        .values(gap, min)
                        .why(&format!("mask {}", layer.name)),
                );
            } else {
                out.inconclusive.push(
                    make(&R_MASK_SLIVER, Severity::Info, place)
                        .at(x, y)
                        .values(gap, min)
                        .why(&format!(
                            "mask {}: an opening that is not a square-cornered rectangle is taken as its box",
                            layer.name
                        )),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Id, Layer};
    use eh3d_phoenix_model::{Nm, ObjectId, Point};

    const MM: i64 = 1_000_000;

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    fn config(text: &str) -> AdvancedConfig {
        AdvancedConfig::parse(text).unwrap()
    }

    fn rect(w: i64, h: i64) -> PadShape {
        PadShape::Rect {
            width: nm(w),
            height: nm(h),
        }
    }

    fn pad(label: &str, at: (i64, i64), shape: PadShape, angle_udeg: i64, layer: u32) -> PlacedPad {
        PlacedPad {
            refdes: label.to_owned(),
            label: format!("{label} pad 1"),
            pin_label: format!("{label}.1"),
            at: Point::from_nm(at.0, at.1).unwrap(),
            net: None,
            shape,
            drill: None,
            layers: vec![id(layer)],
            angle_udeg,
            roundness: 0,
            plated: true,
        }
    }

    /// Two layers of mask, 1 and 2, called Top mask and Bottom mask.
    fn board() -> Board {
        let mut board = Board::default();
        for (n, name) in [(1, "Top mask"), (2, "Bottom mask")] {
            board.layers.insert(
                id(n),
                Layer {
                    name: name.to_owned(),
                    kind: LayerKind::SolderMask,
                    side: None,
                },
            );
        }
        board
    }

    fn run(pads: &[PlacedPad], text: &str) -> Report {
        let mut out = Report::new("t");
        mask_sliver_findings(&board(), pads, &config(text), &mut out);
        out.finish();
        out
    }

    const ON: &str = "mask_sliver_level = warning\nmask_sliver_min_nm = 100000\n";

    #[test]
    fn the_check_is_off_in_the_settings_that_come_with_the_program() {
        let pads = [
            pad("A", (0, 0), rect(MM, MM), 0, 1),
            pad("B", (MM + 50_000, 0), rect(MM, MM), 0, 1),
        ];
        let mut out = Report::new("t");
        mask_sliver_findings(&board(), &pads, &AdvancedConfig::default(), &mut out);
        assert!(out.confirmed.is_empty() && out.inconclusive.is_empty());
        assert!(run(
            &pads,
            "mask_sliver_level = warning\nmask_sliver_min_nm = 0\n"
        )
        .confirmed
        .is_empty());
    }

    #[test]
    fn a_strip_narrower_than_the_limit_is_found_and_one_at_the_limit_or_touching_is_not() {
        let at = |gap: i64| {
            [
                pad("A", (0, 0), rect(MM, MM), 0, 1),
                pad("B", (MM + gap, 0), rect(MM, MM), 0, 1),
            ]
        };
        let out = run(&at(50_000), ON);
        assert_eq!(out.confirmed.len(), 1);
        let finding = &out.confirmed[0];
        assert_eq!(finding.code, "PHOENIX-DRC-022");
        assert_eq!(finding.location, "A pad 1 and B pad 1");
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(50_000), Some(100_000))
        );
        assert_eq!(finding.detail, "mask Top mask");
        assert_eq!(finding.at_nm, Some([0, 0]));
        for gap in [0, 100_000, 400_000] {
            assert!(run(&at(gap), ON).confirmed.is_empty(), "{gap}");
        }
        assert_eq!(run(&at(99_999), ON).confirmed.len(), 1);
    }

    #[test]
    fn the_expansion_of_the_mask_narrows_the_strip_on_every_side() {
        let pads = [
            pad("A", (0, 0), rect(MM, MM), 0, 1),
            pad("B", (MM + 150_000, 0), rect(MM, MM), 0, 1),
        ];
        assert!(run(&pads, ON).confirmed.is_empty());
        let out = run(&pads, &format!("{ON}mask_sliver_expansion_nm = 30000\n"));
        assert_eq!(out.confirmed.len(), 1);
        assert_eq!(out.confirmed[0].measured_nm, Some(90_000));
        // an expansion of 50 micrometres each leaves nothing
        assert!(
            run(&pads, &format!("{ON}mask_sliver_expansion_nm = 75000\n"))
                .confirmed
                .is_empty()
        );
    }

    #[test]
    fn a_quarter_turn_swaps_the_sides_and_another_angle_is_not_judged() {
        // 2 x 0.5 mm turned a quarter becomes 0.5 x 2 mm: the gap along x is 60 micrometres
        let turned = [
            pad("A", (0, 0), rect(2 * MM, MM / 2), 90_000_000, 1),
            pad("B", (MM / 2 + 60_000, 0), rect(MM / 2, 2 * MM), 0, 1),
        ];
        let out = run(&turned, ON);
        assert_eq!(out.confirmed.len(), 1);
        assert_eq!(out.confirmed[0].measured_nm, Some(60_000));
        let flat = [
            pad("A", (0, 0), rect(2 * MM, MM / 2), 0, 1),
            pad("B", (MM / 2 + 60_000, 0), rect(MM / 2, 2 * MM), 0, 1),
        ];
        assert!(run(&flat, ON).confirmed.is_empty());
        let skew = [
            pad("A", (0, 0), rect(MM, MM), 45_000_000, 1),
            pad("B", (MM + 50_000, 0), rect(MM, MM), 0, 1),
        ];
        let out = run(&skew, ON);
        assert!(out.confirmed.is_empty());
        assert_eq!(out.inconclusive.len(), 1);
        assert_eq!(out.inconclusive[0].location, "A pad 1");
        assert_eq!(out.inconclusive[0].code, "PHOENIX-DRC-014");
    }

    #[test]
    fn a_pad_that_is_not_a_square_cornered_rectangle_gives_information_only() {
        let circle = PadShape::Circle { diameter: nm(MM) };
        let pads = [
            pad("A", (0, 0), circle, 0, 1),
            pad("B", (MM + 50_000, 0), rect(MM, MM), 0, 1),
        ];
        let out = run(&pads, ON);
        assert!(out.confirmed.is_empty());
        assert_eq!(out.inconclusive.len(), 1);
        let finding = &out.inconclusive[0];
        assert_eq!(finding.code, "PHOENIX-DRC-022");
        assert_eq!(finding.severity, Severity::Info);
        assert!(
            finding.detail.contains("taken as its box"),
            "{}",
            finding.detail
        );
        let mut rounded = pad("C", (0, 0), rect(MM, MM), 0, 1);
        rounded.roundness = 500;
        let pads = [rounded, pad("D", (MM + 50_000, 0), rect(MM, MM), 0, 1)];
        assert!(run(&pads, ON).confirmed.is_empty());
        assert_eq!(run(&pads, ON).inconclusive.len(), 1);
    }

    #[test]
    fn only_pads_on_the_same_layer_of_mask_are_openings_of_one_side() {
        let pads = [
            pad("A", (0, 0), rect(MM, MM), 0, 1),
            pad("B", (MM + 50_000, 0), rect(MM, MM), 0, 2),
            pad("C", (2 * MM + 100_000, 0), rect(MM, MM), 0, 1),
            pad("D", (3 * MM + 150_000, 0), rect(MM, MM), 0, 2),
        ];
        // far apart on the same layer: no strip, and the near pairs are on different layers
        assert!(run(&pads, ON).confirmed.is_empty());
        let mut both_layers = pad("E", (MM + 40_000, 0), rect(MM, MM), 0, 1);
        both_layers.layers.push(id(2));
        let out = run(&[pads[0].clone(), both_layers], ON);
        assert_eq!(out.confirmed.len(), 1);
        assert_eq!(out.confirmed[0].detail, "mask Top mask");
        // a pad on a layer that is not mask is no opening
        let on_copper = pad("F", (MM + 40_000, 0), rect(MM, MM), 0, 9);
        assert!(run(&[pads[0].clone(), on_copper], ON).confirmed.is_empty());
    }

    /// A generator with a fixed seed, so every run makes the same boards.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn range(&mut self, low: i64, high: i64) -> i64 {
            low + (self.next() % (high - low + 1) as u64) as i64
        }
    }

    #[test]
    fn on_generated_pads_the_pairs_found_are_the_ones_of_a_search_of_every_pair() {
        let mut rng = Lcg(0x6d_0001);
        let (mut found_total, mut quiet_total) = (0, 0);
        for round in 0..60 {
            let count = rng.range(2, 14) as usize;
            let mut pads = Vec::new();
            for n in 0..count {
                let w = rng.range(200_000, 1_500_000);
                let h = rng.range(200_000, 1_500_000);
                let quarter = rng.range(0, 3) * 90_000_000;
                let at = (rng.range(0, 6_000_000), rng.range(0, 4_000_000));
                pads.push(pad(&format!("P{n:02}"), at, rect(w, h), quarter, 1));
            }
            let limit = rng.range(20_000, 300_000);
            let expansion = rng.range(0, 40_000);
            let text = format!(
                "mask_sliver_level = info\nmask_sliver_min_nm = {limit}\nmask_sliver_expansion_nm = {expansion}\n"
            );
            let out = run(&pads, &text);
            assert!(out.inconclusive.is_empty(), "round {round}");
            let mut expected = Vec::new();
            for i in 0..count {
                for j in i + 1..count {
                    let (a, _) = opening_of(&pads[i], expansion).unwrap();
                    let (b, _) = opening_of(&pads[j], expansion).unwrap();
                    let dx = (a.x1_nm.max(b.x1_nm) - a.x2_nm.min(b.x2_nm)).max(0);
                    let dy = (a.y1_nm.max(b.y1_nm) - a.y2_nm.min(b.y2_nm)).max(0);
                    let squared = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
                    if squared > 0 && squared < i128::from(limit) * i128::from(limit) {
                        expected.push(both(&pads[i].label, &pads[j].label));
                    }
                }
            }
            expected.sort();
            let mut got: Vec<String> = out.confirmed.iter().map(|f| f.location.clone()).collect();
            got.sort();
            assert_eq!(got, expected, "round {round}");
            found_total += got.len();
            quiet_total += usize::from(got.is_empty());
        }
        // both outcomes happen, so neither was met by chance
        assert!(
            found_total > 10 && quiet_total > 3,
            "{found_total} {quiet_total}"
        );
    }
}
