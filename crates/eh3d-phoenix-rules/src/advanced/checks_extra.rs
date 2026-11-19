// SPDX-License-Identifier: GPL-3.0-or-later
//! Four advanced checks that read the tracks, the vias and the nets of the board and nothing
//! else: short tracks, tracks whose straight pieces are off a grid of angles, the annular ring by
//! net class and the number of vias on a net. All four are off in the settings that come with the
//! program (their levels are `off`), so a board is only judged by them when the settings ask.
//!
//! The arithmetic on distances is exact integer arithmetic on nanometres. A track that has an arc
//! is not judged for its length (the length of an arc is not an integer), and the angle check
//! looks at straight pieces only.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id, Segment};

use super::checks::{net_name, to_i64};
use super::{make, AdvancedConfig, R_RING_CLASS, R_SHORT_TRACK, R_TRACK_ANGLE, R_VIA_COUNT};
use crate::report::Report;

/// A drilled item whose ring is judged against the limit of the class of its net: a pad of a part
/// (the vias are read from the board).
pub(super) struct RingItem {
    /// What the finding is about.
    pub(super) label: String,
    /// Its net.
    pub(super) net: Option<Id>,
    /// Where it is, in nanometres.
    pub(super) at: (i64, i64),
    /// The smaller side of its pad (or its diameter) less its drill: twice the ring.
    pub(super) ring2: i128,
}

/// The length of the straight piece from `(ax, ay)` to `(bx, by)` rounded up to a whole
/// nanometre, so that the sum of these is never below the true length.
fn chord_ceil(a: (i64, i64), b: (i64, i64)) -> i128 {
    let dx = i128::from(b.0) - i128::from(a.0);
    let dy = i128::from(b.1) - i128::from(a.1);
    let d2 = dx * dx + dy * dy;
    let r = d2.isqrt();
    if r * r < d2 {
        r + 1
    } else {
        r
    }
}

/// Tracks that are no longer than `short_track_min_nm` in all (`PHOENIX-DRC-016`): a fragment
/// that is usually left over from editing. A track with an arc is not judged.
pub(super) fn short_tracks(board: &Board, cfg: &AdvancedConfig, out: &mut Report) {
    let Some(severity) = cfg.short_track_level else {
        return;
    };
    let min = i128::from(cfg.short_track_min_nm);
    if min <= 0 {
        return;
    }
    for (id, track) in &board.tracks {
        let Some(first) = track.path.first() else {
            continue;
        };
        let mut upper: i128 = 0;
        let mut has_arc = false;
        for piece in &track.path {
            match piece {
                Segment::Line { start, end } => {
                    upper += chord_ceil(
                        (start.x_nm.get(), start.y_nm.get()),
                        (end.x_nm.get(), end.y_nm.get()),
                    );
                }
                Segment::Arc { .. } => has_arc = true,
            }
        }
        if has_arc || upper >= min {
            continue;
        }
        let at = first.start();
        out.confirmed.push(
            make(&R_SHORT_TRACK, severity, format!("track {id}"))
                .at(at.x_nm.get(), at.y_nm.get())
                .values(to_i64(upper), cfg.short_track_min_nm),
        );
    }
}

/// Tracks with straight pieces that are not on the grid of angles of `track_angle_step_deg`
/// (`PHOENIX-DRC-017`): 90 degrees allows the two axes, 45 degrees the eight directions. One
/// finding for each track, with the number of pieces that are off the grid. Arcs and pieces of no
/// length are left out.
pub(super) fn track_angles(board: &Board, cfg: &AdvancedConfig, out: &mut Report) {
    let Some(severity) = cfg.track_angle_level else {
        return;
    };
    let diagonals = cfg.track_angle_step_deg == 45;
    for (id, track) in &board.tracks {
        let mut off = 0usize;
        let mut first: Option<(i64, i64)> = None;
        for piece in &track.path {
            let Segment::Line { start, end } = piece else {
                continue;
            };
            let dx = i128::from(end.x_nm.get()) - i128::from(start.x_nm.get());
            let dy = i128::from(end.y_nm.get()) - i128::from(start.y_nm.get());
            if dx == 0 && dy == 0 {
                continue;
            }
            let on_grid = dx == 0 || dy == 0 || (diagonals && dx.abs() == dy.abs());
            if !on_grid {
                off += 1;
                first.get_or_insert((start.x_nm.get(), start.y_nm.get()));
            }
        }
        if let Some((x, y)) = first {
            out.confirmed.push(
                make(&R_TRACK_ANGLE, severity, format!("track {id}"))
                    .at(x, y)
                    .why(&format!(
                        "{off} straight piece(s) off the grid of {} degrees",
                        cfg.track_angle_step_deg
                    )),
            );
        }
    }
}

/// Vias and drilled pads whose annular ring is thinner than the `ring_min_nm` of the class of
/// their net (`PHOENIX-DRC-018`). A net that is in no class, or in a class that gives no ring, is
/// left to the check of `annular_ring_min_nm`.
pub(super) fn ring_classes(
    board: &Board,
    cfg: &AdvancedConfig,
    pads: &[RingItem],
    out: &mut Report,
) {
    let Some(severity) = cfg.ring_class_level else {
        return;
    };
    for (id, via) in &board.vias {
        let name = net_name(board, Some(via.net));
        let Some((min, class)) = cfg.ring_min_for(name) else {
            continue;
        };
        let ring2 = i128::from(via.diameter.get()) - i128::from(via.drill.get());
        if ring2 < 2 * i128::from(min) {
            out.confirmed.push(
                make(&R_RING_CLASS, severity, format!("via {id}"))
                    .at(via.at.x_nm.get(), via.at.y_nm.get())
                    .values(to_i64(ring2.div_euclid(2).max(0)), min)
                    .why(&format!("class {class}")),
            );
        }
    }
    for pad in pads {
        let Some((min, class)) = cfg.ring_min_for(net_name(board, pad.net)) else {
            continue;
        };
        if pad.ring2 < 2 * i128::from(min) {
            out.confirmed.push(
                make(&R_RING_CLASS, severity, pad.label.clone())
                    .at(pad.at.0, pad.at.1)
                    .values(to_i64(pad.ring2.div_euclid(2).max(0)), min)
                    .why(&format!("class {class}")),
            );
        }
    }
}

/// Nets with more vias than `via_count_max` (`PHOENIX-DRC-019`), one finding for each net, the
/// place of the first of its vias.
pub(super) fn via_counts(board: &Board, cfg: &AdvancedConfig, out: &mut Report) {
    let Some(severity) = cfg.via_count_level else {
        return;
    };
    let max = cfg.via_count_max;
    if max <= 0 {
        return;
    }
    let mut counts: BTreeMap<Id, (i64, (i64, i64))> = BTreeMap::new();
    for via in board.vias.values() {
        let entry = counts
            .entry(via.net)
            .or_insert((0, (via.at.x_nm.get(), via.at.y_nm.get())));
        entry.0 += 1;
    }
    for (net, (count, at)) in counts {
        if count <= max {
            continue;
        }
        let label = match net_name(board, Some(net)) {
            Some(name) => format!("net {name}"),
            None => format!("net {net}"),
        };
        out.confirmed.push(
            make(&R_VIA_COUNT, severity, label)
                .at(at.0, at.1)
                .why(&format!("{count} vias, at most {max}")),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Severity;
    use eh3d_phoenix_board::{Net, Track, Via};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment::Line {
            start: pt(a.0, a.1),
            end: pt(b.0, b.1),
        }
    }

    fn board_with_net(name: &str) -> Board {
        let mut board = Board::default();
        board.nets.insert(id(100), Net { name: name.into() });
        board
    }

    fn add_track(board: &mut Board, n: u32, path: Vec<Segment>) {
        board.tracks.insert(
            id(n),
            Track {
                net: id(100),
                layer: id(1),
                width: nm(200_000),
                path,
            },
        );
    }

    fn add_via_on(board: &mut Board, n: u32, net: Id, at: (i64, i64), drill: i64, diameter: i64) {
        board.vias.insert(
            id(n),
            Via {
                net,
                at: pt(at.0, at.1),
                drill: nm(drill),
                diameter: nm(diameter),
                from_layer: id(1),
                to_layer: id(2),
            },
        );
    }

    fn add_via(board: &mut Board, n: u32, at: (i64, i64), drill: i64, diameter: i64) {
        add_via_on(board, n, id(100), at, drill, diameter);
    }

    fn config(text: &str) -> AdvancedConfig {
        AdvancedConfig::parse(text).unwrap()
    }

    /// A small linear congruential generator, so the cases are the same on every run.
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
    fn the_four_checks_are_off_in_the_settings_that_come_with_the_program() {
        let mut board = board_with_net("A");
        add_track(&mut board, 1, vec![line((0, 0), (1, 1))]);
        add_via(&mut board, 2, (0, 0), 300_000, 301_000);
        let cfg = AdvancedConfig::default();
        let mut out = Report::new("t");
        short_tracks(&board, &cfg, &mut out);
        track_angles(&board, &cfg, &mut out);
        ring_classes(&board, &cfg, &[], &mut out);
        via_counts(&board, &cfg, &mut out);
        out.finish();
        assert!(out.confirmed.is_empty());
    }

    #[test]
    fn a_track_that_is_short_in_all_is_found_and_one_at_the_limit_is_not() {
        let cfg = config("short_track_level = warning\nshort_track_min_nm = 200000\n");
        let mut board = board_with_net("A");
        add_track(&mut board, 1, vec![line((0, 0), (100_000, 0))]);
        add_track(&mut board, 2, vec![line((0, 0), (200_000, 0))]);
        // two pieces of 80 and 90 micrometres: 170 in all, short
        add_track(
            &mut board,
            3,
            vec![
                line((0, 0), (80_000, 0)),
                line((80_000, 0), (80_000, 90_000)),
            ],
        );
        // an arc is not judged
        add_track(
            &mut board,
            4,
            vec![Segment::Arc {
                start: pt(0, 0),
                end: pt(10, 0),
                sweep: Udeg::ZERO,
            }],
        );
        // no piece at all
        add_track(&mut board, 5, Vec::new());
        let mut out = Report::new("t");
        short_tracks(&board, &cfg, &mut out);
        let places: Vec<&str> = out.confirmed.iter().map(|f| f.location.as_str()).collect();
        assert_eq!(places.len(), 2, "{places:?}");
        assert!(places.contains(&format!("track {}", id(1)).as_str()));
        assert!(places.contains(&format!("track {}", id(3)).as_str()));
        let first = out
            .confirmed
            .iter()
            .find(|f| f.location == format!("track {}", id(1)))
            .unwrap();
        assert_eq!(first.measured_nm, Some(100_000));
        assert_eq!(first.limit_nm, Some(200_000));
        assert_eq!(first.severity, Severity::Warning);
        assert_eq!(first.code, "PHOENIX-DRC-016");
    }

    #[test]
    fn a_length_with_a_fraction_is_rounded_up_so_a_long_enough_track_is_never_flagged() {
        // the length is the square root of 2 million: 1414.2 nm, which counts as 1415
        let cfg = config("short_track_level = info\nshort_track_min_nm = 1415\n");
        let mut board = board_with_net("A");
        add_track(&mut board, 1, vec![line((0, 0), (1000, 1000))]);
        let mut out = Report::new("t");
        short_tracks(&board, &cfg, &mut out);
        assert!(out.confirmed.is_empty());
        let cfg = config("short_track_level = info\nshort_track_min_nm = 1416\n");
        short_tracks(&board, &cfg, &mut out);
        assert_eq!(out.confirmed.len(), 1);
    }

    #[test]
    fn pieces_off_the_grid_of_angles_are_found_for_each_track_with_their_count() {
        let cfg = config("track_angle_level = warning\ntrack_angle_step_deg = 45\n");
        let mut board = board_with_net("A");
        // along the axes and on the diagonal: fine
        add_track(
            &mut board,
            1,
            vec![
                line((0, 0), (1_000_000, 0)),
                line((1_000_000, 0), (2_000_000, 1_000_000)),
                line((2_000_000, 1_000_000), (2_000_000, 0)),
                line((2_000_000, 0), (1_000_000, 1_000_000)),
            ],
        );
        // two pieces off the grid
        add_track(
            &mut board,
            2,
            vec![
                line((0, 0), (2_000_000, 1_000_000)),
                line((2_000_000, 1_000_000), (2_000_000, 3_000_000)),
                line((2_000_000, 3_000_000), (2_000_001, 4_000_000)),
            ],
        );
        // a piece of no length is not an angle
        add_track(&mut board, 3, vec![line((5, 5), (5, 5))]);
        let mut out = Report::new("t");
        track_angles(&board, &cfg, &mut out);
        assert_eq!(out.confirmed.len(), 1);
        let finding = &out.confirmed[0];
        assert_eq!(finding.code, "PHOENIX-DRC-017");
        assert_eq!(finding.location, format!("track {}", id(2)));
        assert_eq!(finding.at_nm, Some([0, 0]));
        assert_eq!(
            finding.detail,
            "2 straight piece(s) off the grid of 45 degrees"
        );
    }

    #[test]
    fn a_grid_of_90_degrees_does_not_allow_the_diagonal() {
        let cfg = config("track_angle_level = error\ntrack_angle_step_deg = 90\n");
        let mut board = board_with_net("A");
        add_track(&mut board, 1, vec![line((0, 0), (1_000_000, 1_000_000))]);
        add_track(&mut board, 2, vec![line((0, 0), (0, 1_000_000))]);
        let mut out = Report::new("t");
        track_angles(&board, &cfg, &mut out);
        assert_eq!(out.confirmed.len(), 1);
        assert_eq!(out.confirmed[0].location, format!("track {}", id(1)));
        assert_eq!(out.confirmed[0].severity, Severity::Error);
    }

    #[test]
    fn the_ring_of_a_class_is_judged_for_its_vias_and_its_pads_only() {
        let cfg = config(
            "ring_class_level = error\ndrill_class.power.nets = A\ndrill_class.power.ring_min_nm = 150000\n",
        );
        let mut board = board_with_net("A");
        board.nets.insert(id(101), Net { name: "B".into() });
        // a ring of 100 um on net A (in the class) and on net B (in none)
        add_via(&mut board, 1, (0, 0), 300_000, 500_000);
        add_via_on(&mut board, 2, id(101), (5, 5), 300_000, 500_000);
        // a via with a ring of exactly 150 um is fine
        add_via(&mut board, 3, (9, 9), 300_000, 600_000);
        let pads = [
            RingItem {
                label: "R1.1".into(),
                net: Some(id(100)),
                at: (7, 7),
                ring2: 200_000,
            },
            RingItem {
                label: "R1.2".into(),
                net: Some(id(101)),
                at: (8, 8),
                ring2: 0,
            },
            RingItem {
                label: "R1.3".into(),
                net: None,
                at: (8, 8),
                ring2: 0,
            },
        ];
        let mut out = Report::new("t");
        ring_classes(&board, &cfg, &pads, &mut out);
        let places: Vec<&str> = out.confirmed.iter().map(|f| f.location.as_str()).collect();
        assert_eq!(places.len(), 2, "{places:?}");
        assert!(places.contains(&format!("via {}", id(1)).as_str()));
        assert!(places.contains(&"R1.1"));
        for finding in &out.confirmed {
            assert_eq!(finding.code, "PHOENIX-DRC-018");
            assert_eq!(finding.limit_nm, Some(150_000));
            assert_eq!(finding.detail, "class power");
        }
        let pad = out.confirmed.iter().find(|f| f.location == "R1.1").unwrap();
        assert_eq!(pad.measured_nm, Some(100_000));
    }

    #[test]
    fn a_class_with_no_ring_leaves_its_nets_to_the_ordinary_ring_check() {
        let cfg = config(
            "ring_class_level = error\ndrill_class.power.nets = A\ndrill_class.power.min_nm = 300000\n",
        );
        let mut board = board_with_net("A");
        add_via(&mut board, 1, (0, 0), 300_000, 300_000);
        let mut out = Report::new("t");
        ring_classes(&board, &cfg, &[], &mut out);
        assert!(out.confirmed.is_empty());
    }

    #[test]
    fn a_net_with_more_vias_than_the_limit_is_found_once() {
        let cfg = config("via_count_level = warning\nvia_count_max = 2\n");
        let mut board = board_with_net("GND");
        board.nets.insert(id(101), Net { name: "SIG".into() });
        for n in 1..=3 {
            add_via(&mut board, n, (i64::from(n), 0), 300_000, 600_000);
        }
        add_via_on(&mut board, 9, id(101), (0, 0), 300_000, 600_000);
        let mut out = Report::new("t");
        via_counts(&board, &cfg, &mut out);
        assert_eq!(out.confirmed.len(), 1);
        let finding = &out.confirmed[0];
        assert_eq!(finding.code, "PHOENIX-DRC-019");
        assert_eq!(finding.location, "net GND");
        assert_eq!(finding.detail, "3 vias, at most 2");
        assert_eq!(finding.at_nm, Some([1, 0]));
    }

    #[test]
    fn short_tracks_match_the_sum_of_the_lengths_on_generated_tracks() {
        let mut rng = Lcg(0x00c0_ffee);
        let cfg = config("short_track_level = info\nshort_track_min_nm = 5000\n");
        let mut board = board_with_net("A");
        let mut expected = Vec::new();
        for n in 1..=60u32 {
            let mut path = Vec::new();
            let mut at = (0i64, 0i64);
            let mut total = 0i64;
            for _ in 0..rng.range(1, 4) {
                let step = rng.range(0, 3000);
                let to = if rng.next().is_multiple_of(2) {
                    (at.0 + step, at.1)
                } else {
                    (at.0, at.1 - step)
                };
                total += step;
                path.push(line(at, to));
                at = to;
            }
            if total < 5000 {
                expected.push(format!("track {}", id(n)));
            }
            add_track(&mut board, n, path);
        }
        let mut out = Report::new("t");
        short_tracks(&board, &cfg, &mut out);
        let mut found: Vec<String> = out.confirmed.iter().map(|f| f.location.clone()).collect();
        found.sort();
        expected.sort();
        assert_eq!(found, expected);
        assert!(!expected.is_empty() && expected.len() < 60);
    }

    #[test]
    fn the_grid_of_angles_agrees_with_the_angle_in_degrees_on_generated_pieces() {
        let mut rng = Lcg(0x0bad_cafe);
        for step in [45, 90] {
            let cfg = config(&format!(
                "track_angle_level = info\ntrack_angle_step_deg = {step}\n"
            ));
            let mut board = board_with_net("A");
            let mut expected = Vec::new();
            for n in 1..=80u32 {
                let (dx, dy) = (rng.range(-4, 4), rng.range(-4, 4));
                add_track(&mut board, n, vec![line((10, 10), (10 + dx, 10 + dy))]);
                if dx == 0 && dy == 0 {
                    continue;
                }
                let angle = (dy as f64).atan2(dx as f64).to_degrees();
                let k = (angle / f64::from(step)).round();
                if (angle - k * f64::from(step)).abs() >= 1e-9 {
                    expected.push(format!("track {}", id(n)));
                }
            }
            let mut out = Report::new("t");
            track_angles(&board, &cfg, &mut out);
            let mut found: Vec<String> = out.confirmed.iter().map(|f| f.location.clone()).collect();
            found.sort();
            expected.sort();
            assert_eq!(found, expected, "step {step}");
        }
    }
}
