// SPDX-License-Identifier: GPL-3.0-or-later
//! Properties of the plan on small boards made by a generator with a fixed seed: the result
//! passes the design rules, never creates a short, is the same every time, only adds, can be
//! undone, and stays inside the limit of expanded nodes.

use eh3d_phoenix_route::{diff, findings, RouteSettings};
use eh3d_phoenix_spatial::testgen::Rng;

use crate::config::{AutorouteConfig, NetOrder};
use crate::plan::AutoroutePlan;
use crate::testboard::{cfg, ctx, mm, Fix};
use crate::{autoroute, Request};

/// A board of 24 by 16 millimetres with three to five nets of two pads each, at places in whole
/// millimetres that no other pad has, on the top layer or through the board.
fn random_board(seed: u64) -> Fix {
    let mut rng = Rng::new(seed);
    let mut fix = Fix::new(24, 16, true);
    let nets = 3 + rng.below(3) as usize;
    let mut taken: Vec<(i64, i64)> = Vec::new();
    for n in 0..nets {
        let net = fix.net(&format!("N{n}"));
        for _ in 0..2 {
            let at = loop {
                let p = (rng.between(2, 22), rng.between(2, 14));
                if !taken.contains(&p) {
                    break p;
                }
            };
            taken.push(at);
            let through = rng.below(2) == 0;
            fix.pad(net, mm(at.0), mm(at.1), through);
        }
    }
    fix
}

fn run(fix: &Fix, config: &AutorouteConfig) -> AutoroutePlan {
    autoroute(
        &fix.board,
        &ctx(),
        &RouteSettings::default(),
        config,
        &Request::default(),
    )
    .unwrap()
}

#[test]
fn the_generator_makes_the_same_boards_for_the_same_seed() {
    for seed in 0..4 {
        let a = random_board(seed).board.to_bytes().unwrap();
        let b = random_board(seed).board.to_bytes().unwrap();
        assert_eq!(a, b);
    }
    let a = random_board(0).board.to_bytes().unwrap();
    let b = random_board(1).board.to_bytes().unwrap();
    assert_ne!(a, b);
}

#[test]
fn random_small_boards_keep_the_promises_of_the_plan() {
    let orders = [NetOrder::Shortest, NetOrder::Widest, NetOrder::Critical];
    for seed in 0..12_u64 {
        let fix = random_board(seed);
        let config = AutorouteConfig {
            order: orders[seed as usize % orders.len()],
            critical_nets: vec!["N1".to_owned()],
            ..cfg()
        };
        let plan = run(&fix, &config);
        let again = run(&fix, &config);
        // the same input gives the same bytes
        assert_eq!(
            plan.board_after.to_bytes().unwrap(),
            again.board_after.to_bytes().unwrap(),
            "seed {seed}"
        );
        assert_eq!(plan.to_json(), again.to_json(), "seed {seed}");
        // it passes the design rules and creates no short
        let before = findings(&fix.board, &ctx());
        let after = findings(&plan.board_after, &ctx());
        assert!(!diff(&before, &after).adds_error(), "seed {seed}");
        assert!(plan.report.errors_after <= plan.report.errors_before);
        // the limit of expanded nodes
        assert!(
            plan.report.expansions <= config.max_expansions_total,
            "seed {seed}"
        );
        // it only adds
        assert_eq!(
            plan.board_after.tracks.len(),
            fix.board.tracks.len() + plan.tracks.len()
        );
        assert_eq!(
            plan.board_after.vias.len(),
            fix.board.vias.len() + plan.vias.len()
        );
        // every connection is either routed or reported
        assert_eq!(
            plan.report.connections_routed + plan.report.unrouted.len(),
            plan.report.connections_total,
            "seed {seed}"
        );
        // the nets that are said to be complete are in one piece
        let connectivity = plan.board_after.connectivity();
        for name in &plan.report.nets_completed {
            let id = plan
                .board_after
                .nets
                .iter()
                .find(|(_, net)| net.name == *name)
                .map(|(id, _)| *id)
                .unwrap();
            let island_count = connectivity
                .nets
                .iter()
                .find(|n| n.net == id)
                .map_or(0, |n| n.islands.len());
            assert!(island_count <= 1, "seed {seed} net {name}");
        }
        for via in &plan.vias {
            assert_ne!(via.from_layer, via.to_layer);
        }
        // the plan is one step with an exact inverse
        let mut board = fix.board.clone();
        plan.apply(&mut board).unwrap();
        assert_eq!(board, plan.board_after, "seed {seed}");
        let inverse = {
            let mut scratch = fix.board.clone();
            scratch.apply(&plan.operation).unwrap()
        };
        board.apply(&inverse).unwrap();
        assert_eq!(board, fix.board, "seed {seed}");
    }
}

#[test]
fn a_tiny_limit_leaves_a_valid_plan_and_stops_the_search_at_the_limit() {
    for seed in 0..6_u64 {
        let fix = random_board(seed);
        let config = AutorouteConfig {
            max_expansions_total: 50,
            ..cfg()
        };
        let plan = run(&fix, &config);
        assert!(
            plan.report.expansions <= 52,
            "seed {seed}: {}",
            plan.report.expansions
        );
        let before = findings(&fix.board, &ctx());
        let after = findings(&plan.board_after, &ctx());
        assert!(!diff(&before, &after).adds_error(), "seed {seed}");
        assert_eq!(
            plan.report.connections_routed + plan.report.unrouted.len(),
            plan.report.connections_total
        );
    }
}

#[test]
fn without_diagonals_every_piece_runs_along_an_axis() {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let other = fix.net("OTHER");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    fix.pad(other, mm(6), mm(5), false);
    let config = AutorouteConfig {
        diagonal: false,
        ..cfg()
    };
    let plan = run(&fix, &config);
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(!plan.tracks.is_empty());
    for track in &plan.tracks {
        for w in track.points.windows(2) {
            let same_x = w[0].x_nm == w[1].x_nm;
            let same_y = w[0].y_nm == w[1].y_nm;
            assert!(same_x || same_y, "{w:?}");
        }
    }
}
