// SPDX-License-Identifier: GPL-3.0-or-later
//! The report for each net, a pair that crosses layers with one via for each track, and the
//! vias off the grid: boards with a result worked out by hand, the same result every time, and a
//! plan that never adds a rule error. What is off by default is shown to change nothing.

use eh3d_phoenix_board::{Id, Via};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::{diff, findings, RouteSettings};
use eh3d_phoenix_spatial::testgen::Rng;

use crate::codes::{PAIR_UNCOUPLED, PAIR_WITH_VIA};
use crate::config::AutorouteConfig;
use crate::plan::{AutoroutePlan, NetReport};
use crate::testboard::{cfg, ctx, id, mm, pt, Fix};
use crate::{autoroute, autoroute_with_pairs, PairSpec, Request};

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

fn run_pairs(fix: &Fix, config: &AutorouteConfig, pairs: &[PairSpec]) -> AutoroutePlan {
    autoroute_with_pairs(
        &fix.board,
        &ctx(),
        &RouteSettings::default(),
        config,
        &Request::default(),
        pairs,
    )
    .unwrap()
}

/// Four nets of two pads each, at places in whole millimetres that no other pad has, on the top
/// layer or through the board.
fn random_board(seed: u64) -> Fix {
    let mut rng = Rng::new(seed);
    let mut fix = Fix::new(24, 16, true);
    let mut taken: Vec<(i64, i64)> = Vec::new();
    for n in 0..4 {
        let net = fix.net(&format!("N{n}"));
        for through in [true, false] {
            let at = loop {
                let p = (rng.between(2, 22), rng.between(2, 14));
                if !taken.contains(&p) {
                    break p;
                }
            };
            taken.push(at);
            fix.pad(net, mm(at.0), mm(at.1), through);
        }
    }
    fix
}

fn net_line(name: &str, connections: usize, routed: usize) -> NetReport {
    NetReport {
        name: name.to_owned(),
        connections,
        routed,
        tracks: 0,
        vias: 0,
        length_nm: 0,
    }
}

#[test]
fn the_report_gives_each_net_its_connections_tracks_vias_and_length() {
    // A: 6 mm straight on y = 2; B: 8 mm straight on y = 8; both on the top layer
    let mut fix = Fix::new(20, 10, true);
    let b = fix.net("B");
    let a = fix.net("A");
    fix.pad(b, mm(2), mm(8), false);
    fix.pad(b, mm(10), mm(8), false);
    fix.pad(a, mm(2), mm(2), false);
    fix.pad(a, mm(8), mm(2), false);
    let plan = run(&fix, &cfg());
    assert_eq!(
        plan.report.nets,
        vec![
            NetReport {
                tracks: 1,
                length_nm: 6_000_000,
                ..net_line("A", 1, 1)
            },
            NetReport {
                tracks: 1,
                length_nm: 8_000_000,
                ..net_line("B", 1, 1)
            },
        ]
    );
    assert_eq!(plan.report.nets[0].remaining(), 0);
    // the lengths of the nets add up to the length of the plan
    let sum: i64 = plan.report.nets.iter().map(|n| n.length_nm).sum();
    assert_eq!(sum, plan.report.length_nm);
    assert_eq!(plan.report.length_nm, 14_000_000);
    let text = plan.to_text("en");
    assert!(
        text.contains("net A: routed 1 of 1, remaining 0, tracks 1, vias 0, length 6000000 nm\n")
    );
    assert!(
        text.contains("net B: routed 1 of 1, remaining 0, tracks 1, vias 0, length 8000000 nm\n")
    );
    let json = plan.to_json();
    assert!(json.contains(
        "\"nets\":[{\"name\":\"A\",\"connections\":1,\"routed\":1,\"remaining\":0,\"tracks\":1,\"vias\":0,\"length_nm\":6000000},{\"name\":\"B\",\"connections\":1,\"routed\":1,\"remaining\":0,\"tracks\":1,\"vias\":0,\"length_nm\":8000000}],\"pairs_coupled\":"
    ));
}

#[test]
fn a_net_that_cannot_be_routed_shows_the_connections_that_remain() {
    let mut fix = Fix::new(16, 10, false);
    let sig = fix.net("SIG");
    let wall = fix.net("W");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    fix.pad(sig, mm(12), mm(5), false);
    let top = fix.top;
    fix.wall(wall, top, (mm(6), 0), (mm(6), mm(10)));
    let plan = run(&fix, &cfg());
    assert_eq!(plan.report.nets.len(), 1);
    let line = &plan.report.nets[0];
    assert_eq!(line.name, "SIG");
    assert_eq!(line.connections, 2);
    // the two pads on the right of the wall are joined, the one on the left cannot be
    assert_eq!(line.routed, 1);
    assert_eq!(line.remaining(), 1);
    assert_eq!((line.tracks, line.vias), (1, 0));
    assert_eq!(line.length_nm, 2_000_000);
    assert!(plan.report.nets_completed.is_empty());
    assert_eq!(plan.report.connections_left, 1);
    assert!(plan
        .to_text("en")
        .contains("net SIG: routed 1 of 2, remaining 1, tracks 1, vias 0, length 2000000 nm\n"));
}

#[test]
fn the_report_of_each_net_is_the_same_every_time() {
    let fix = random_board(5);
    let first = run(&fix, &cfg());
    let second = run(&fix, &cfg());
    assert_eq!(first.report.nets, second.report.nets);
    assert_eq!(first.to_json(), second.to_json());
    assert_eq!(first.to_text("pt"), second.to_text("pt"));
    // the nets are in the order of their names
    let names: Vec<&str> = first.report.nets.iter().map(|n| n.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    // what the nets say adds up to what the plan says
    let tracks: usize = first.report.nets.iter().map(|n| n.tracks).sum();
    let vias: usize = first.report.nets.iter().map(|n| n.vias).sum();
    assert_eq!(tracks, first.report.tracks_added);
    assert_eq!(vias, first.report.vias_added);
    let routed: usize = first.report.nets.iter().map(|n| n.routed).sum();
    assert_eq!(routed, first.report.connections_routed);
}

/// A board of 30 by 14 millimetres, two layers, with the pair P and N: the pads on the top layer
/// at x = 4 and the pads on the bottom layer at x = 26, 1 mm apart.
fn crossing_pair_board() -> Fix {
    let mut fix = Fix::new(30, 14, true);
    let p = fix.net("P");
    let n = fix.net("N");
    fix.pad(p, mm(4), mm(7), false);
    fix.pad(n, mm(4), mm(6), false);
    fix.pad_bottom(p, mm(26), mm(7));
    fix.pad_bottom(n, mm(26), mm(6));
    fix
}

fn pair_with_gap(gap_nm: i64) -> PairSpec {
    PairSpec {
        name: "PAIR".to_owned(),
        positive: "P".to_owned(),
        negative: "N".to_owned(),
        gap_nm,
    }
}

fn via_config() -> AutorouteConfig {
    AutorouteConfig {
        couple_via: true,
        ..cfg()
    }
}

fn net_id(fix: &Fix, name: &str) -> Id {
    *fix.board
        .nets
        .iter()
        .find(|(_, n)| n.name == name)
        .map(|(id, _)| id)
        .unwrap()
}

/// The points of the track of a net on a layer, from the lower x to the higher one.
fn on_layer(plan: &AutoroutePlan, fix: &Fix, net: &str, layer: Id) -> Vec<Point> {
    let wanted = net_id(fix, net);
    let mut points = plan
        .tracks
        .iter()
        .find(|t| t.net == wanted && t.layer == layer)
        .unwrap_or_else(|| panic!("no track of {net}"))
        .points
        .clone();
    if points[0].x_nm.get() > points[points.len() - 1].x_nm.get() {
        points.reverse();
    }
    points
}

#[test]
fn a_pair_that_changes_layer_is_two_ordinary_nets_unless_the_via_is_asked_for() {
    let fix = crossing_pair_board();
    let plan = run_pairs(&fix, &cfg(), &[pair_with_gap(750_000)]);
    assert!(plan.report.pairs_coupled.is_empty());
    assert_eq!(plan.report.notes.len(), 1);
    let note = &plan.report.notes[0];
    assert!(note.starts_with(PAIR_UNCOUPLED), "{note}");
    assert!(note.contains("no layer in common"), "{note}");
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
}

#[test]
fn a_pair_crosses_layers_with_one_via_for_each_track_side_by_side() {
    let fix = crossing_pair_board();
    let plan = run_pairs(&fix, &via_config(), &[pair_with_gap(750_000)]);
    assert_eq!(plan.report.pairs_coupled, vec!["PAIR".to_owned()]);
    assert_eq!(plan.report.notes.len(), 1, "{:?}", plan.report.notes);
    assert!(plan.report.notes[0].starts_with(PAIR_WITH_VIA));
    assert!(plan.report.notes[0].contains("pair PAIR"));
    // the gap of 0.75 mm and the width of 0.25 mm put the centre lines 1 mm apart, which is the
    // distance of the pads: the fat track runs along y = 6.5 mm from x = 5.5 mm (the first node
    // that keeps away from the four pads) to x = 24.5 mm, where the pair changes layer, and
    // each track is a straight line from its pad to its via
    assert_eq!(plan.tracks.len(), 4);
    assert_eq!(plan.vias.len(), 2);
    let (top, bottom) = (fix.top, fix.bottom);
    assert_eq!(
        on_layer(&plan, &fix, "P", top),
        vec![pt(mm(4), mm(7)), pt(24_500_000, mm(7))]
    );
    assert_eq!(
        on_layer(&plan, &fix, "P", bottom),
        vec![pt(24_500_000, mm(7)), pt(mm(26), mm(7))]
    );
    assert_eq!(
        on_layer(&plan, &fix, "N", top),
        vec![pt(mm(4), mm(6)), pt(24_500_000, mm(6))]
    );
    assert_eq!(
        on_layer(&plan, &fix, "N", bottom),
        vec![pt(24_500_000, mm(6)), pt(mm(26), mm(6))]
    );
    // one via for each net, at the end of the track on top, 1 mm from the other
    let mut spots: Vec<(Id, i64, i64)> = plan
        .vias
        .iter()
        .map(|v| (v.net, v.at.x_nm.get(), v.at.y_nm.get()))
        .collect();
    spots.sort_by_key(|s| s.2);
    assert_eq!(spots[0], (net_id(&fix, "N"), 24_500_000, mm(6)));
    assert_eq!(spots[1], (net_id(&fix, "P"), 24_500_000, mm(7)));
    for via in &plan.vias {
        assert_eq!((via.from_layer, via.to_layer), (top, bottom));
        assert_eq!((via.drill_nm, via.diameter_nm), (300_000, 600_000));
    }
    // each track is 20.5 mm on top and 1.5 mm below
    let lengths: Vec<i64> = plan.tracks.iter().map(|t| t.length_nm()).collect();
    assert_eq!(lengths.iter().sum::<i64>(), 44_000_000);
    assert_eq!(plan.report.length_nm, 44_000_000);
    for line in &plan.report.nets {
        assert_eq!((line.tracks, line.vias), (2, 1));
        assert_eq!(line.length_nm, 22_000_000);
    }
    // the rules find nothing new and the board is connected
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_before, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(plan.board_after.connectivity().is_fully_connected());
    assert!(plan.to_text("en").contains("coupled pair: PAIR\n"));
    assert!(plan
        .to_text("en")
        .contains("note: PHOENIX-AROUTE-018: pair PAIR: one via for each track\n"));
}

#[test]
fn the_pair_across_layers_is_the_same_every_time_and_one_reversible_step() {
    let fix = crossing_pair_board();
    let pairs = [pair_with_gap(750_000)];
    let first = run_pairs(&fix, &via_config(), &pairs);
    let second = run_pairs(&fix, &via_config(), &pairs);
    assert_eq!(first.to_json(), second.to_json());
    assert_eq!(first.to_text("de"), second.to_text("de"));
    assert_eq!(first.tracks, second.tracks);
    assert_eq!(first.vias, second.vias);
    let before = findings(&fix.board, &ctx());
    let after = findings(&first.board_after, &ctx());
    assert!(!diff(&before, &after).adds_error());
    let mut board = fix.board.clone();
    first.apply(&mut board).unwrap();
    assert_eq!(board, first.board_after);
}

#[test]
fn vias_that_would_touch_each_other_keep_the_pair_from_crossing() {
    // a gap of 0.25 mm puts the centre lines 0.5 mm apart: two vias of 0.6 mm would overlap
    let fix = crossing_pair_board();
    let plan = run_pairs(&fix, &via_config(), &[pair_with_gap(250_000)]);
    assert!(plan.report.pairs_coupled.is_empty());
    assert_eq!(plan.report.notes.len(), 1);
    let note = &plan.report.notes[0];
    assert!(note.starts_with(PAIR_UNCOUPLED), "{note}");
    assert!(note.contains("too close to each other"), "{note}");
    // both nets are routed all the same, and the rules find nothing new
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
}

#[test]
fn the_via_for_pairs_changes_nothing_for_a_pair_that_is_coupled_on_one_layer() {
    let mut fix = Fix::new(30, 14, false);
    let p = fix.net("P");
    let n = fix.net("N");
    fix.pad(p, mm(4), mm(7), false);
    fix.pad(n, mm(4), mm(6), false);
    fix.pad(p, mm(26), mm(7), false);
    fix.pad(n, mm(26), mm(6), false);
    let pairs = [pair_with_gap(250_000)];
    let off = run_pairs(&fix, &cfg(), &pairs);
    let on = run_pairs(&fix, &via_config(), &pairs);
    assert_eq!(off.report.pairs_coupled, vec!["PAIR".to_owned()]);
    assert_eq!(off.to_json(), on.to_json());
    assert_eq!(off.tracks, on.tracks);
    assert!(on.vias.is_empty());
}

#[test]
fn the_new_settings_change_nothing_on_boards_with_no_pair_and_no_via() {
    let fix = random_board(11);
    let wide = AutorouteConfig {
        couple_via: true,
        via_snap_nm: 400_000,
        ..cfg()
    };
    let plain = run(&fix, &cfg());
    let changed = run(&fix, &wide);
    assert_eq!(plain.to_json(), changed.to_json());
    assert_eq!(plain.tracks, changed.tracks);
    assert_eq!(plain.vias, changed.vias);
}

/// The board of the existing test of the vias, with the via of the net 0.1 mm off its node.
fn board_with_a_via_off_the_grid() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let wall = fix.net("W");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(14), mm(5), false);
    let (top, bottom) = (fix.top, fix.bottom);
    fix.wall(wall, top, (mm(10), mm(1) / 2), (mm(10), mm(19) / 2));
    fix.wall(sig, top, (mm(2), mm(5)), (6_100_000, mm(5)));
    fix.board.vias.insert(
        id(900),
        Via {
            net: sig,
            at: pt(6_100_000, mm(5)),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: top,
            to_layer: bottom,
        },
    );
    fix
}

#[test]
fn a_via_off_the_grid_leaves_no_rule_error_and_the_same_result_every_time() {
    let fix = board_with_a_via_off_the_grid();
    for snap in [0, 99_999, 100_000, 150_000, 400_000] {
        let config = AutorouteConfig {
            via_snap_nm: snap,
            ..cfg()
        };
        let first = run(&fix, &config);
        let second = run(&fix, &config);
        assert_eq!(first.to_json(), second.to_json(), "snap {snap}");
        assert_eq!(first.tracks, second.tracks, "snap {snap}");
        assert_eq!(first.report.connections_left, 0, "snap {snap}");
        assert!(first.report.errors_after <= first.report.errors_before);
        assert_eq!(first.report.errors_after, 0, "snap {snap}");
        assert!(first.board_after.connectivity().is_fully_connected());
        // the via that was there is never put down again
        assert!(first
            .vias
            .iter()
            .all(|v| (v.at.x_nm.get(), v.at.y_nm.get()) != (6_100_000, mm(5))));
    }
}
