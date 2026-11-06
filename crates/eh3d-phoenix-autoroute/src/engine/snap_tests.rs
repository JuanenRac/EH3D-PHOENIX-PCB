// SPDX-License-Identifier: GPL-3.0-or-later
//! The vias that are already on the board and are not on a node of the grid: which of them the
//! search may use (`via_snap_nm`) and the short piece that joins one to its node, with numbers
//! worked out by hand.

use std::collections::HashMap;

use eh3d_phoenix_board::Via;
use eh3d_phoenix_model::Nm;
use eh3d_phoenix_route::copper_layers;

use super::{assemble, dogleg, index_vias};
use crate::search::{Found, Grid, Node, Terminal};
use crate::testboard::{id, mm, pt, Fix};

fn grid() -> Grid {
    Grid {
        ox: 0,
        oy: 0,
        pitch: 500_000,
        nx: 41,
        ny: 21,
    }
}

/// A path along y = 0 on the first layer from x = 0 to x = 1 mm, a change of layer at the node
/// (2, 0) and one more step on the second layer.
fn found() -> Found {
    let node = |ix: i64, slot: usize| Node { ix, iy: 0, slot };
    Found {
        nodes: vec![node(0, 0), node(1, 0), node(2, 0), node(2, 1), node(3, 1)],
        src: 0,
        dst: 0,
        expansions: 5,
    }
}

fn terminal(slot: usize, ix: i64, at: (i64, i64)) -> Terminal {
    Terminal {
        slot,
        ix,
        iy: 0,
        stub: vec![at],
    }
}

#[test]
fn the_way_from_a_node_to_a_via_goes_along_the_axes() {
    assert_eq!(dogleg((10, 10), (10, 15)), vec![(10, 15)]);
    assert_eq!(dogleg((10, 10), (15, 10)), vec![(15, 10)]);
    assert_eq!(dogleg((10, 10), (13, 14)), vec![(13, 10), (13, 14)]);
    assert_eq!(dogleg((10, 10), (7, 4)), vec![(7, 10), (7, 4)]);
}

#[test]
fn a_route_through_a_via_off_its_node_is_joined_to_it_on_both_layers() {
    let sources = [terminal(0, 0, (0, 0))];
    let targets = [terminal(1, 3, (1_500_000, 0))];
    let free: HashMap<(i64, i64), (usize, usize)> = HashMap::from([((2, 0), (0, 1))]);
    // the via is 0.1 mm to the east and 0.08 mm to the north of the node (1 mm, 0)
    let off: HashMap<(i64, i64), (i64, i64)> = HashMap::from([((2, 0), (1_100_000, 80_000))]);
    let slots = [0, 1];
    let route = assemble(&found(), &sources, &targets, &slots, &grid(), &free, &off);
    assert_eq!(route.snapped, 1);
    assert!(route.vias.is_empty());
    assert_eq!(route.runs.len(), 2);
    assert_eq!(route.runs[0].layer, 0);
    assert_eq!(
        route.runs[0].pts,
        vec![(0, 0), (1_100_000, 0), (1_100_000, 80_000)]
    );
    assert_eq!(route.runs[1].layer, 1);
    assert_eq!(
        route.runs[1].pts,
        vec![(1_100_000, 80_000), (1_100_000, 0), (1_500_000, 0)]
    );
}

#[test]
fn without_the_offset_the_route_changes_layer_on_the_node_as_before() {
    let sources = [terminal(0, 0, (0, 0))];
    let targets = [terminal(1, 3, (1_500_000, 0))];
    let free: HashMap<(i64, i64), (usize, usize)> = HashMap::from([((2, 0), (0, 1))]);
    let off: HashMap<(i64, i64), (i64, i64)> = HashMap::new();
    let slots = [0, 1];
    let route = assemble(&found(), &sources, &targets, &slots, &grid(), &free, &off);
    assert_eq!(route.snapped, 0);
    assert!(route.vias.is_empty());
    assert_eq!(route.runs[0].pts, vec![(0, 0), (1_000_000, 0)]);
    assert_eq!(route.runs[1].pts, vec![(1_000_000, 0), (1_500_000, 0)]);
}

fn via_at(fix: &Fix, net: eh3d_phoenix_board::Id, x: i64, y: i64) -> Via {
    Via {
        net,
        at: pt(x, y),
        drill: Nm::new(300_000).unwrap(),
        diameter: Nm::new(600_000).unwrap(),
        from_layer: fix.top,
        to_layer: fix.bottom,
    }
}

/// Three vias of one net: one on the node (12, 10), one 0.1 mm east of it and one 0.2 mm east
/// and 0.1 mm north of the node (18, 6).
fn board_with_three_vias() -> (Fix, eh3d_phoenix_board::Id) {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let vias = [
        (900, mm(6), mm(5)),
        (901, 6_100_000, mm(5)),
        (902, 9_200_000, 3_100_000),
    ];
    for (n, x, y) in vias {
        let via = via_at(&fix, sig, x, y);
        fix.board.vias.insert(id(n), via);
    }
    (fix, sig)
}

#[test]
fn only_a_via_on_a_node_is_used_when_the_distance_is_zero() {
    let (fix, sig) = board_with_three_vias();
    let copper = copper_layers(&fix.board);
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 0);
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].len(), 1);
    assert_eq!(free[0].get(&(12, 10)), Some(&(0, 1)));
    assert!(off[0].is_empty());
}

#[test]
fn a_via_near_a_node_is_used_when_it_is_within_the_distance_and_not_when_it_is_beyond() {
    let (fix, sig) = board_with_three_vias();
    let copper = copper_layers(&fix.board);
    // the second via is 100000 nm from the node (12, 10), which the first via is on: the one on
    // the node is kept, and nothing is off the node
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 100_000);
    assert_eq!(free[0].len(), 1);
    assert!(off[0].is_empty());
    // the third via is sqrt(200000^2 + 100000^2) = 223606.8 nm from the node (18, 6)
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 223_606);
    assert_eq!(free[0].len(), 1);
    assert!(off[0].is_empty());
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 223_607);
    assert_eq!(free[0].len(), 2);
    assert_eq!(free[0].get(&(18, 6)), Some(&(0, 1)));
    assert_eq!(off[0].len(), 1);
    assert_eq!(off[0].get(&(18, 6)), Some(&(9_200_000, 3_100_000)));
}

#[test]
fn of_two_vias_for_one_node_the_nearer_one_is_kept() {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    for (n, x) in [(900, 6_100_000), (901, 5_950_000)] {
        let via = via_at(&fix, sig, x, mm(5));
        fix.board.vias.insert(id(n), via);
    }
    let copper = copper_layers(&fix.board);
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 150_000);
    assert_eq!(free[0].len(), 1);
    assert_eq!(off[0].get(&(12, 10)), Some(&(5_950_000, mm(5))));
}

#[test]
fn a_via_of_another_net_or_outside_the_grid_is_not_used() {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let other = fix.net("OTHER");
    let on_other = via_at(&fix, other, mm(6), mm(5));
    fix.board.vias.insert(id(900), on_other);
    let outside = via_at(&fix, sig, -mm(3), mm(5));
    fix.board.vias.insert(id(901), outside);
    let copper = copper_layers(&fix.board);
    let (free, off) = index_vias(&fix.board, &[sig], &grid(), &copper, 5_000_000);
    assert!(free[0].is_empty());
    assert!(off[0].is_empty());
}
