// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper clearance check with the spatial index against the comparison of every pair, on
//! boards made from fixed seeds, and the count of comparisons that tells linear work from
//! quadratic work without a clock.

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::{
    analyse_board, analyse_board_with, check_board, check_board_with, clearance_work, edge_work,
    IndexMode, Report, RuleValues,
};
use eh3d_phoenix_spatial::testgen::Rng;
use eh3d_phoenix_spatial::SpatialConfig;

const MM: i64 = 1_000_000;
const TOP: u32 = 1;
const BOT: u32 = 2;

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

/// An empty square board with two copper layers.
fn base(side: i64) -> Board {
    let mut b = Board::default();
    for (n, name) in [(TOP, "Top"), (BOT, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    b.copper_stack = vec![id(TOP), id(BOT)];
    b.outline = vec![pt(0, 0), pt(side, 0), pt(side, side), pt(0, side)];
    b
}

fn add_track(b: &mut Board, n: u32, net: Id, layer: u32, width: i64, path: Vec<Segment>) {
    b.tracks.insert(
        id(n),
        Track {
            net,
            layer: id(layer),
            width: nm(width),
            path,
        },
    );
}

fn add_via(b: &mut Board, n: u32, net: Id, at: (i64, i64), diameter: i64) {
    b.vias.insert(
        id(n),
        Via {
            net,
            at: pt(at.0, at.1),
            drill: nm(diameter / 2),
            diameter: nm(diameter),
            from_layer: id(TOP),
            to_layer: id(BOT),
        },
    );
}

/// A board with `tracks` tracks and `vias` vias at random, a few of them long, with an arc, with
/// the same place twice, outside the outline, at negative coordinates and at the extremes of the
/// range of the design.
fn random_board(seed: u64, tracks: u32, vias: u32, side_mm: i64) -> Board {
    let mut rng = Rng::new(seed);
    let side = side_mm * MM;
    let mut b = base(side);
    let mut nets: Vec<Id> = Vec::new();
    for k in 0..5_u32 {
        let net_id = id(10 + k);
        b.nets.insert(
            net_id,
            Net {
                name: format!("N{k}"),
            },
        );
        nets.push(net_id);
    }
    for k in 0..tracks {
        let net = nets[rng.below(5) as usize];
        let (x, y) = (
            rng.between(-side / 10, side + side / 10),
            rng.between(-side / 10, side + side / 10),
        );
        let reach = if rng.below(25) == 0 { side / 3 } else { 3 * MM };
        let (dx, dy) = (rng.between(-reach, reach), rng.between(-reach, reach));
        let mut path = vec![line((x, y), (x + dx, y + dy))];
        if rng.below(3) == 0 {
            let (ex, ey) = (rng.between(-MM, MM), rng.between(-MM, MM));
            path.push(line((x + dx, y + dy), (x + dx + ex, y + dy + ey)));
        }
        if rng.below(40) == 0 {
            path.push(Segment::Arc {
                start: pt(x + dx, y + dy),
                end: pt(x + dx + MM, y + dy),
                sweep: Udeg::ZERO,
            });
        }
        let width = 100_000 + rng.between(0, 300_000);
        let layer = if rng.below(3) == 0 { BOT } else { TOP };
        add_track(&mut b, 1000 + k, net, layer, width, path);
    }
    for k in 0..vias {
        let net = nets[rng.below(5) as usize];
        let at = (rng.between(0, side), rng.between(0, side));
        add_via(
            &mut b,
            20_000 + k,
            net,
            at,
            400_000 + rng.between(0, 400_000),
        );
    }
    // the same place twice, on two nets
    add_track(
        &mut b,
        30_000,
        nets[0],
        TOP,
        200_000,
        vec![line((MM, MM), (5 * MM, MM))],
    );
    add_track(
        &mut b,
        30_001,
        nets[1],
        TOP,
        200_000,
        vec![line((MM, MM), (5 * MM, MM))],
    );
    // from one end of the range of the design to the other, and a via at each end
    let (lo, hi) = (Nm::MIN.get(), Nm::MAX.get());
    add_track(
        &mut b,
        30_002,
        nets[2],
        TOP,
        200_000,
        vec![line((lo, lo), (hi, hi))],
    );
    add_via(&mut b, 30_003, nets[3], (lo, lo), 500_000);
    add_via(&mut b, 30_004, nets[4], (hi, hi), 500_000);
    b
}

fn rules() -> RuleValues {
    RuleValues {
        min_track_width: nm(150_000),
        min_clearance: nm(300_000),
        min_drill: nm(150_000),
        min_via_diameter: nm(450_000),
        min_edge_clearance: nm(500_000),
        min_drill_spacing: nm(300_000),
        ..RuleValues::default()
    }
}

fn same_report(a: &Report, b: &Report, what: &str) {
    assert_eq!(a, b, "{what}");
    assert_eq!(a.to_json(), b.to_json(), "{what}");
}

#[test]
fn the_grid_gives_the_report_of_comparing_every_pair() {
    for (seed, tracks, vias, side_mm) in
        [(1, 60, 20, 20), (2, 600, 150, 60), (3, 3_000, 1_000, 150)]
    {
        let board = random_board(seed, tracks, vias, side_mm);
        let grid = check_board_with(&board, &rules(), IndexMode::Grid);
        let naive = check_board_with(&board, &rules(), IndexMode::Naive);
        same_report(&grid, &naive, &format!("seed {seed}"));
        same_report(&check_board(&board, &rules()), &naive, "the default way");
        // the boards are dense, so the comparison is about something
        let clearance = grid
            .confirmed
            .iter()
            .filter(|f| f.rule == "copper_clearance")
            .count();
        assert!(clearance > 5, "seed {seed}: {clearance} clearance findings");
    }
}

#[test]
fn the_touching_copper_found_with_the_index_is_that_of_comparing_every_pair() {
    for (seed, tracks, vias, side_mm) in [(1, 60, 20, 20), (2, 600, 150, 40), (3, 1_500, 500, 60)] {
        let board = random_board(seed, tracks, vias, side_mm);
        for tolerance in [0, 1_000, 250_000, 2 * MM] {
            let grid = analyse_board_with(&board, nm(tolerance), IndexMode::Grid);
            let naive = analyse_board_with(&board, nm(tolerance), IndexMode::Naive);
            same_report(
                &grid,
                &naive,
                &format!("seed {seed}, tolerance {tolerance}"),
            );
            same_report(
                &analyse_board(&board, nm(tolerance)),
                &naive,
                "the default way",
            );
            assert!(
                grid.confirmed.len() > 5,
                "seed {seed}: the board is dense enough for the comparison to say something"
            );
        }
    }
}

/// Square boards of tracks one millimetre long, ten millimetres apart.
fn lattice_board(seed: u64, count: u32) -> Board {
    let mut rng = Rng::new(seed);
    let columns = f64::from(count).sqrt().ceil() as i64;
    let side = (columns + 1) * 10 * MM;
    let mut b = base(side);
    let nets: Vec<Id> = (0..2_u32)
        .map(|k| {
            b.nets.insert(
                id(10 + k),
                Net {
                    name: format!("N{k}"),
                },
            );
            id(10 + k)
        })
        .collect();
    for k in 0..count {
        let (col, row) = (i64::from(k) % columns, i64::from(k) / columns);
        let x = 5 * MM + col * 10 * MM + rng.between(0, MM);
        let y = 5 * MM + row * 10 * MM + rng.between(0, MM);
        add_track(
            &mut b,
            1000 + k,
            nets[(k % 2) as usize],
            TOP,
            200_000,
            vec![line((x, y), (x + MM, y))],
        );
    }
    b
}

#[test]
fn the_comparisons_grow_with_the_items_and_not_with_their_square() {
    let config = SpatialConfig::default();
    let mut previous = 0_u64;
    for count in [500_u32, 1_000, 2_000, 4_000] {
        let board = lattice_board(11, count);
        let grid = clearance_work(&board, &rules(), IndexMode::Grid, &config);
        let naive = clearance_work(&board, &rules(), IndexMode::Naive, &config);
        let n = u64::from(count);
        assert_eq!(grid.items, count as usize);
        assert_eq!(grid.refused, None);
        // each track is near itself and a few others at most
        assert!(grid.comparisons <= 4 * n, "{count}: {}", grid.comparisons);
        assert_eq!(naive.comparisons, n * n);
        assert!(grid.cells_used > 0 && grid.entries >= grid.cells_used as u64);
        // doubling the items does not much more than double the comparisons
        if previous > 0 {
            assert!(grid.comparisons <= 3 * previous, "{count}");
        }
        previous = grid.comparisons;
    }
}

#[test]
fn a_grid_the_settings_refuse_is_replaced_by_the_comparison_of_every_pair() {
    let board = random_board(5, 300, 100, 40);
    let tight = SpatialConfig {
        cell_nm: 100_000,
        max_cells: 10,
        ..SpatialConfig::default()
    };
    let work = clearance_work(&board, &rules(), IndexMode::Grid, &tight);
    assert_eq!(work.refused, Some("PHOENIX-SPATIAL-002"));
    assert_eq!(work.comparisons, (work.items as u64) * (work.items as u64));
    // the check itself does not depend on the refusal
    let grid = check_board_with(&board, &rules(), IndexMode::Grid);
    let naive = check_board_with(&board, &rules(), IndexMode::Naive);
    same_report(&grid, &naive, "refused");
}

#[test]
fn with_no_clearance_in_the_rules_there_is_no_work() {
    let board = lattice_board(3, 100);
    let off = RuleValues::default();
    let work = clearance_work(&board, &off, IndexMode::Grid, &SpatialConfig::default());
    assert_eq!((work.items, work.comparisons), (100, 0));
}

/// A board whose outline is a polygon of `edges` pieces round a centre, with `tracks` short tracks
/// at random inside and a little outside it, and the extreme track and vias of `random_board`.
fn round_board(seed: u64, tracks: u32, edges: u32) -> Board {
    let side = 80 * MM;
    let mut b = random_board(seed, tracks, 40, 80);
    let (centre, radius) = (side as f64 / 2.0, side as f64 / 2.0);
    b.outline = (0..edges)
        .map(|k| {
            let turn = std::f64::consts::TAU * f64::from(k) / f64::from(edges);
            pt(
                (centre + radius * turn.cos()).round() as i64,
                (centre + radius * turn.sin()).round() as i64,
            )
        })
        .collect();
    b
}

#[test]
fn the_edge_check_with_the_index_gives_the_report_of_comparing_every_piece() {
    for (seed, tracks, edges) in [
        (21_u64, 300_u32, 40_u32),
        (22, 1_500, 200),
        (23, 4_000, 600),
    ] {
        let board = round_board(seed, tracks, edges);
        let grid = check_board_with(&board, &rules(), IndexMode::Grid);
        let naive = check_board_with(&board, &rules(), IndexMode::Naive);
        same_report(&grid, &naive, &format!("seed {seed}"));
        let near_the_edge = grid
            .confirmed
            .iter()
            .filter(|f| f.rule == "edge_clearance")
            .count();
        assert!(near_the_edge > 5, "seed {seed}: {near_the_edge}");
        // the boards have a piece of copper that reaches the extremes of the range, which is
        // judged against the whole outline in both ways
        assert!(
            grid.inconclusive.iter().any(|f| f.rule == "not_judged"),
            "seed {seed}"
        );
    }
}

#[test]
fn the_pieces_of_the_outline_are_looked_up_and_not_compared_one_by_one() {
    let config = SpatialConfig::default();
    let board = round_board(24, 2_000, 400);
    let grid = edge_work(&board, &rules(), IndexMode::Grid, &config);
    let naive = edge_work(&board, &rules(), IndexMode::Naive, &config);
    assert_eq!(grid.edges, 400);
    assert!(grid.indexed && !naive.indexed);
    let every_pair = (grid.items as u64) * 400;
    assert_eq!(naive.comparisons, every_pair);
    // the few pieces of copper that are too far out for the index are compared with all of them;
    // the rest see a handful of pieces each
    assert!(
        grid.comparisons * 4 < every_pair,
        "{} of {every_pair}",
        grid.comparisons
    );
}

#[test]
fn an_outline_with_few_pieces_is_compared_piece_by_piece_unless_the_settings_say_otherwise() {
    let board = round_board(25, 200, 8);
    let config = SpatialConfig::default();
    let few = edge_work(&board, &rules(), IndexMode::Grid, &config);
    assert!(!few.indexed);
    assert_eq!(few.comparisons, (few.items as u64) * 8);
    let eager = SpatialConfig {
        edge_index_min_edges: 4,
        ..SpatialConfig::default()
    };
    assert!(edge_work(&board, &rules(), IndexMode::Grid, &eager).indexed);
    // the report does not depend on it
    same_report(
        &check_board_with(&board, &rules(), IndexMode::Grid),
        &check_board_with(&board, &rules(), IndexMode::Naive),
        "eight pieces",
    );
    // with no edge clearance in the rules there is no work
    let off = RuleValues::default();
    let none = edge_work(&board, &off, IndexMode::Grid, &config);
    assert_eq!(none.comparisons, 0);
}
