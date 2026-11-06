// SPDX-License-Identifier: GPL-3.0-or-later
//! The ratsnest with the spatial index against the comparison of every pair of points, on boards
//! made from fixed seeds, and the count of distances that shows the index doing less work.

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track};
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_route::{ratsnest_with, RouteSettings};
use eh3d_phoenix_rules::IndexMode;
use eh3d_phoenix_spatial::testgen::Rng;

const MM: i64 = 1_000_000;
const TOP: u32 = 1;

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

/// An empty board with one copper layer and `nets` nets.
fn base(nets: u32) -> Board {
    let mut b = Board::default();
    b.layers.insert(
        id(TOP),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.copper_stack = vec![id(TOP)];
    b.outline = vec![
        pt(0, 0),
        pt(300 * MM, 0),
        pt(300 * MM, 300 * MM),
        pt(0, 300 * MM),
    ];
    for k in 0..nets {
        b.nets.insert(
            id(10 + k),
            Net {
                name: format!("N{k}"),
            },
        );
    }
    b
}

fn add_track(b: &mut Board, n: u32, net: u32, path: Vec<Segment>) {
    b.tracks.insert(
        id(n),
        Track {
            net: id(10 + net),
            layer: id(TOP),
            width: nm(200_000),
            path,
        },
    );
}

fn both(board: &Board) -> (Vec<eh3d_phoenix_route::RatLine>, u64, u64) {
    let settings = RouteSettings::default();
    let (grid, grid_work) = ratsnest_with(board, &settings, IndexMode::Grid).unwrap();
    let (naive, naive_work) = ratsnest_with(board, &settings, IndexMode::Naive).unwrap();
    assert_eq!(grid, naive);
    assert_eq!(grid_work.anchors, naive_work.anchors);
    (grid, grid_work.comparisons, naive_work.comparisons)
}

#[test]
fn islands_with_many_anchors_give_the_lines_of_comparing_every_pair() {
    for seed in [1_u64, 2, 3] {
        let mut rng = Rng::new(seed);
        let mut b = base(4);
        let mut n = 1000;
        for net in 0..4_u32 {
            for _ in 0..6 {
                // a walk of 10 to 80 steps, so that some islands are small and some are large
                let steps = 10 + rng.below(70) as i64;
                let mut at = (rng.between(0, 290 * MM), rng.between(0, 290 * MM));
                let mut path = Vec::new();
                for _ in 0..steps {
                    let next = (
                        (at.0 + rng.between(-2 * MM, 2 * MM)).clamp(0, 300 * MM),
                        (at.1 + rng.between(-2 * MM, 2 * MM)).clamp(0, 300 * MM),
                    );
                    path.push(line(at, next));
                    at = next;
                }
                add_track(&mut b, n, net, path);
                n += 1;
            }
        }
        let (lines, _, _) = both(&b);
        assert!(lines.len() >= 10, "seed {seed}: {} lines", lines.len());
    }
}

/// A track of `count` pieces along a horizontal line, `step` nanometres each.
fn row(y: i64, count: i64, step: i64) -> Vec<Segment> {
    (0..count)
        .map(|i| line((i * step, y), ((i + 1) * step, y)))
        .collect()
}

#[test]
fn rows_of_points_at_the_same_distance_keep_the_first_pair() {
    let mut b = base(1);
    add_track(&mut b, 1000, 0, row(10 * MM, 3_000, 10_000));
    add_track(&mut b, 1001, 0, row(11 * MM, 3_000, 10_000));
    add_track(&mut b, 1002, 0, row(15 * MM, 3_000, 10_000));
    let (lines, indexed, every_pair) = both(&b);
    assert_eq!(lines.len(), 2);
    // many pairs are exactly 1 mm apart: the first one in the order of the points is kept
    let first = lines.iter().find(|l| l.length_nm == MM).unwrap();
    assert_eq!((first.from_at.x_nm.get(), first.to_at.x_nm.get()), (0, 0));
    // the index measures far fewer distances than the 9 million of every pair of two rows
    assert!(every_pair >= 18_000_000, "{every_pair}");
    assert!(indexed * 2 < every_pair, "{indexed} against {every_pair}");
}

#[test]
fn small_islands_are_compared_point_by_point() {
    let mut b = base(1);
    for k in 0..5_i64 {
        add_track(
            &mut b,
            1000 + k as u32,
            0,
            vec![line((k * 20 * MM, 0), (k * 20 * MM, 5 * MM))],
        );
    }
    let (lines, indexed, every_pair) = both(&b);
    assert_eq!(lines.len(), 4);
    assert_eq!(indexed, every_pair);
}
