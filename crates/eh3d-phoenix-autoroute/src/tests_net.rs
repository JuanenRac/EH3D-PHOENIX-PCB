// SPDX-License-Identifier: GPL-3.0-or-later
//! The weights of the nets, the limit of expanded nodes of each net, the statistics of each net,
//! a pair that crosses several layers and a route that goes through a via of the net that is off
//! the grid half way: boards with a result worked out by hand and the same result every time.
//! What is off by default is shown to change nothing.

use eh3d_phoenix_board::{Id, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::RouteSettings;
use eh3d_phoenix_rules::{NetClass, NetClasses};

use crate::codes::{NET_LIMIT, NET_SETTING_UNUSED, PAIR_MANY_LAYERS, PAIR_UNCOUPLED, VIA_SNAPPED};
use crate::config::AutorouteConfig;
use crate::plan::AutoroutePlan;
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

/// Net A, 6 mm on y = 2, and net B, 8 mm on y = 8, both on the top layer. B is made first, so its
/// id is the lower one.
fn two_lines() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let b = fix.net("B");
    let a = fix.net("A");
    fix.pad(b, mm(2), mm(8), false);
    fix.pad(b, mm(10), mm(8), false);
    fix.pad(a, mm(2), mm(2), false);
    fix.pad(a, mm(8), mm(2), false);
    fix
}

fn one_line() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    fix
}

fn with_weights(pairs: &[(&str, i64)]) -> AutorouteConfig {
    let mut config = cfg();
    for (name, weight) in pairs {
        config.net_weight.insert((*name).to_owned(), *weight);
    }
    config
}

#[test]
fn the_nets_are_routed_shortest_first_and_the_report_gives_the_order() {
    // A has 6 mm of connection and B has 8 mm: A goes first
    let plan = run(&two_lines(), &cfg());
    assert_eq!(plan.report.net_order, ["A", "B"]);
    let stats = &plan.report.net_stats;
    assert_eq!(stats.len(), 2);
    assert_eq!((stats[0].name.as_str(), stats[0].order), ("A", 1));
    assert_eq!((stats[1].name.as_str(), stats[1].order), ("B", 2));
    for line in stats {
        assert_eq!(line.weight_percent, 100);
        assert_eq!((line.rips, line.corners, line.max_expansions), (0, 0, 0));
        assert!(line.expansions > 0, "{line:?}");
    }
    let text = plan.to_text("en");
    assert!(text.contains("order of the nets: A, B\n"), "{text}");
    assert!(
        text.contains("net-stats A: weight 100%, order 1, expanded nodes "),
        "{text}"
    );
    let json = plan.to_json();
    assert!(json.contains("\"net_order\":[\"A\",\"B\"],\"net_stats\":[{\"name\":\"A\",\"weight_percent\":100,\"order\":1,"));
    assert!(json.ends_with("\"unrouted\":[]}"));
    // the nodes of the nets add up to the nodes of the run
    let sum: u64 = stats.iter().map(|n| n.expansions).sum();
    assert_eq!(sum, plan.report.expansions);
}

#[test]
fn a_net_of_more_weight_is_routed_first_and_nothing_else_changes() {
    let fix = two_lines();
    let plain = run(&fix, &cfg());
    let heavy = run(&fix, &with_weights(&[("B", 200)]));
    assert_eq!(heavy.report.net_order, ["B", "A"]);
    let stats = &heavy.report.net_stats;
    assert_eq!(
        (
            stats[0].name.as_str(),
            stats[0].weight_percent,
            stats[0].order
        ),
        ("A", 100, 2)
    );
    assert_eq!(
        (
            stats[1].name.as_str(),
            stats[1].weight_percent,
            stats[1].order
        ),
        ("B", 200, 1)
    );
    // the two lines do not meet, so the tracks are the same
    assert_eq!(heavy.tracks, plain.tracks);
    assert_eq!(heavy.vias, plain.vias);
    assert!(heavy.to_text("en").contains("order of the nets: B, A\n"));
    assert!(heavy.to_json().contains("\"net_order\":[\"B\",\"A\"]"));
    assert_eq!(heavy.report.errors_after, 0);
    // the same result twice
    let again = run(&fix, &with_weights(&[("B", 200)]));
    assert_eq!(heavy.to_json(), again.to_json());
    assert_eq!(heavy.to_text("pt"), again.to_text("pt"));
}

#[test]
fn a_weight_of_100_and_no_weight_give_the_same_plan() {
    let fix = two_lines();
    let plain = run(&fix, &cfg());
    let named = run(&fix, &with_weights(&[("A", 100), ("B", 100)]));
    assert_eq!(plain.tracks, named.tracks);
    assert_eq!(plain.report.net_order, named.report.net_order);
    assert_eq!(plain.report.net_stats, named.report.net_stats);
    assert_eq!(plain.to_json(), named.to_json());
    assert!(named.report.notes.is_empty());
}

#[test]
fn the_weight_of_a_class_applies_to_its_nets_and_the_weight_of_the_net_wins() {
    let fix = two_lines();
    let zero = Nm::new(0).unwrap();
    let class = NetClass {
        name: "Fast".to_owned(),
        nets: vec!["B".to_owned()],
        min_track_width: zero,
        min_clearance: zero,
        min_via_diameter: zero,
        min_via_drill: zero,
    };
    let mut rules = ctx();
    rules.classes = NetClasses::new(vec![class]).unwrap();
    let route = |config: &AutorouteConfig| {
        autoroute(
            &fix.board,
            &rules,
            &RouteSettings::default(),
            config,
            &Request::default(),
        )
        .unwrap()
    };
    let mut by_class = cfg();
    by_class.class_weight.insert("Fast".to_owned(), 300);
    let plan = route(&by_class);
    assert_eq!(plan.report.net_order, ["B", "A"]);
    assert_eq!(plan.report.net_stats[1].weight_percent, 300);
    assert_eq!(plan.report.net_stats[0].weight_percent, 100);
    // the weight of the net itself wins over the one of its class
    by_class.net_weight.insert("B".to_owned(), 50);
    let plan = route(&by_class);
    assert_eq!(plan.report.net_order, ["A", "B"]);
    assert_eq!(plan.report.net_stats[1].weight_percent, 50);
    assert!(plan.report.notes.is_empty(), "{:?}", plan.report.notes);
}

#[test]
fn a_net_that_reaches_its_own_limit_is_left_unrouted_with_a_note() {
    let fix = one_line();
    // the search expands a fourth node and stops: 4 nodes, then no other search
    let mut limited = cfg();
    limited.net_max_expansions.insert("SIG".to_owned(), 3);
    let plan = run(&fix, &limited);
    assert!(plan.tracks.is_empty());
    assert_eq!(plan.report.unrouted.len(), 1);
    assert_eq!(plan.report.unrouted[0].code, NET_LIMIT);
    assert!(plan.report.unrouted[0].detail.contains("limit of 3"));
    assert_eq!(plan.report.expansions, 4);
    assert_eq!(plan.report.net_stats[0].expansions, 4);
    assert_eq!(plan.report.net_stats[0].max_expansions, 3);
    assert_eq!(plan.report.notes.len(), 1);
    assert_eq!(
        plan.report.notes[0],
        "PHOENIX-AROUTE-020: net SIG: the limit of 3 expanded nodes was reached"
    );
    assert!(plan
        .to_text("pt")
        .contains("Uma rede atingiu o seu próprio limite de nós expandidos"));
    // the limit for every net does the same, and the limit of the net wins over it
    let all = AutorouteConfig {
        max_expansions_per_net: 3,
        ..cfg()
    };
    let same = run(&fix, &all);
    assert_eq!(same.report.unrouted[0].code, NET_LIMIT);
    assert_eq!(same.report.expansions, 4);
    let mut wide = all.clone();
    wide.net_max_expansions.insert("SIG".to_owned(), 1_000_000);
    let routed = run(&fix, &wide);
    assert_eq!(routed.tracks.len(), 1);
    assert!(routed.report.notes.is_empty());
    assert_eq!(routed.report.net_stats[0].max_expansions, 1_000_000);
}

#[test]
fn a_limit_that_is_never_reached_changes_nothing_but_the_number_in_the_report() {
    let fix = two_lines();
    let plain = run(&fix, &cfg());
    let roomy = AutorouteConfig {
        max_expansions_per_net: 10_000_000,
        ..cfg()
    };
    let limited = run(&fix, &roomy);
    assert_eq!(plain.tracks, limited.tracks);
    assert_eq!(plain.report.expansions, limited.report.expansions);
    assert!(limited.report.notes.is_empty());
    assert!(limited.report.unrouted.is_empty());
}

#[test]
fn a_weight_or_a_limit_for_a_net_or_class_that_is_not_in_the_run_is_noted() {
    let fix = one_line();
    let mut config = cfg();
    config.net_weight.insert("NOPE".to_owned(), 150);
    config.net_max_expansions.insert("NONE".to_owned(), 5);
    config.class_weight.insert("Ghost".to_owned(), 120);
    let plan = run(&fix, &config);
    assert_eq!(plan.report.connections_routed, 1);
    assert_eq!(
        plan.report.notes,
        [
            format!("{NET_SETTING_UNUSED}: net_weight.NOPE: no connection to route"),
            format!("{NET_SETTING_UNUSED}: net_max_expansions.NONE: no connection to route"),
            format!("{NET_SETTING_UNUSED}: net_class_weight.Ghost: no connection to route"),
        ]
    );
    assert!(plan
        .to_text("en")
        .contains("note: PHOENIX-AROUTE-021: net_weight.NOPE: no connection to route\n"));
}

/// The track of a net on a layer, from the lower x to the higher one.
fn ends_on(plan: &AutoroutePlan, layer: Id) -> Vec<Point> {
    let mut points = plan
        .tracks
        .iter()
        .find(|t| t.layer == layer)
        .unwrap_or_else(|| panic!("no track on the layer"))
        .points
        .clone();
    if points[0].x_nm.get() > points[points.len() - 1].x_nm.get() {
        points.reverse();
    }
    points
}

/// Net SIG: a pad A on the top layer at (8, 5) mm, a pad B on the bottom layer at (14, 5) mm and,
/// on the island of A, a track that goes up from A, left along y = 8 and down to a via 0.1 mm east
/// of the node (2, 5). The via is the only place to change layer that costs nothing, and it is not
/// on the way from A to B: the best route goes to it and comes back.
fn board_with_a_via_half_way() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    fix.pad(sig, mm(8), mm(5), false);
    fix.pad_bottom(sig, mm(14), mm(5));
    let (top, bottom) = (fix.top, fix.bottom);
    let line = |from: (i64, i64), to: (i64, i64)| Segment::Line {
        start: pt(from.0, from.1),
        end: pt(to.0, to.1),
    };
    fix.board.tracks.insert(
        id(910),
        Track {
            net: sig,
            layer: top,
            width: Nm::new(250_000).unwrap(),
            path: vec![
                line((mm(8), mm(5)), (mm(8), mm(8))),
                line((mm(8), mm(8)), (2_100_000, mm(8))),
                line((2_100_000, mm(8)), (2_100_000, mm(5))),
            ],
        },
    );
    fix.board.vias.insert(
        id(900),
        Via {
            net: sig,
            at: pt(2_100_000, mm(5)),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: top,
            to_layer: bottom,
        },
    );
    fix
}

#[test]
fn the_best_route_goes_through_a_via_off_the_grid_half_way_and_puts_no_new_via() {
    let fix = board_with_a_via_half_way();
    // a via costs 100 mm: any route with a new via is far worse than the walk to the via that is
    // there. The walk is 12 steps of 0.5 mm west on the top layer, which prefers horizontal
    // steps, to the node (2, 5) mm, and 24 steps east on the bottom layer, where a horizontal
    // step costs half as much again: 6 + 18 = 24 mm. The via is 0.1 mm east of the node, so the
    // top track ends at x = 2.1 mm (5.9 mm long) and the bottom one starts there (11.9 mm long).
    let config = AutorouteConfig {
        via_penalty_nm: 100_000_000,
        via_snap_nm: 150_000,
        ..cfg()
    };
    let plan = run(&fix, &config);
    assert_eq!(plan.report.connections_total, 1);
    assert_eq!(plan.report.connections_routed, 1);
    assert!(plan.vias.is_empty());
    assert_eq!(plan.tracks.len(), 2);
    assert_eq!(
        ends_on(&plan, fix.top),
        vec![pt(2_100_000, mm(5)), pt(mm(8), mm(5))]
    );
    assert_eq!(
        ends_on(&plan, fix.bottom),
        vec![pt(2_100_000, mm(5)), pt(mm(14), mm(5))]
    );
    assert_eq!(plan.report.length_nm, 17_800_000);
    assert_eq!(plan.report.net_stats[0].corners, 0);
    assert_eq!(plan.report.notes.len(), 1);
    assert!(plan.report.notes[0].starts_with(VIA_SNAPPED));
    assert!(plan.report.notes[0].contains("1 existing via(s)"));
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_before, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(plan.board_after.connectivity().is_fully_connected());
    // the same result every time, and the via that was there is not put down again
    let again = run(&fix, &config);
    assert_eq!(plan.to_json(), again.to_json());
    assert_eq!(plan.tracks, again.tracks);
}

#[test]
fn without_the_distance_the_via_off_the_grid_is_left_alone_and_a_new_via_is_put_down() {
    let fix = board_with_a_via_half_way();
    let config = AutorouteConfig {
        via_penalty_nm: 100_000_000,
        ..cfg()
    };
    let plan = run(&fix, &config);
    // the route is the straight 6 mm from A to B with one new via on the way
    assert_eq!(plan.vias.len(), 1);
    assert_eq!(plan.report.length_nm, 6_000_000);
    assert!(plan.report.notes.is_empty());
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
}

/// A board of 30 by 14 millimetres with two copper layers: the pair P and N on the top layer at
/// x = 4 and at x = 26, 1 mm apart, and a wall of another net across the whole of the top layer
/// at x = 15.
fn pair_behind_a_wall() -> Fix {
    let mut fix = Fix::new(30, 14, true);
    let p = fix.net("P");
    let n = fix.net("N");
    let w = fix.net("W");
    fix.pad(p, mm(4), mm(7), false);
    fix.pad(n, mm(4), mm(6), false);
    fix.pad(p, mm(26), mm(7), false);
    fix.pad(n, mm(26), mm(6), false);
    let top = fix.top;
    fix.wall(w, top, (mm(15), mm(1) / 2), (mm(15), mm(27) / 2));
    fix
}

fn the_pair() -> PairSpec {
    PairSpec {
        name: "PAIR".to_owned(),
        positive: "P".to_owned(),
        negative: "N".to_owned(),
        gap_nm: 750_000,
    }
}

fn run_pair(fix: &Fix, config: &AutorouteConfig) -> AutoroutePlan {
    autoroute_with_pairs(
        &fix.board,
        &ctx(),
        &RouteSettings::default(),
        config,
        &Request::default(),
        &[the_pair()],
    )
    .unwrap()
}

#[test]
fn a_pair_that_goes_under_a_wall_changes_layer_twice_when_the_settings_allow_it() {
    let fix = pair_behind_a_wall();
    // with one change of layer allowed the pair cannot be coupled: both ends are on the top layer
    let one = run_pair(
        &fix,
        &AutorouteConfig {
            couple_via: true,
            ..cfg()
        },
    );
    assert!(one.report.pairs_coupled.is_empty());
    assert!(
        one.report.notes[0].starts_with(PAIR_UNCOUPLED),
        "{:?}",
        one.report.notes
    );
    // with two the pair goes down before the wall and up after it, one via for each track at each
    let many = AutorouteConfig {
        couple_via: true,
        couple_layers_max: 2,
        ..cfg()
    };
    let plan = run_pair(&fix, &many);
    assert_eq!(plan.report.pairs_coupled, vec!["PAIR".to_owned()]);
    assert_eq!(plan.report.notes.len(), 1, "{:?}", plan.report.notes);
    assert!(plan.report.notes[0].starts_with(PAIR_MANY_LAYERS));
    assert!(plan.report.notes[0].contains("2 changes of layer"));
    assert_eq!(plan.vias.len(), 4);
    assert_eq!(plan.tracks.len(), 6);
    for line in &plan.report.nets {
        assert_eq!((line.tracks, line.vias), (3, 2), "{}", line.name);
    }
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_before, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(plan.board_after.connectivity().is_fully_connected());
    assert!(plan.to_text("en").contains("coupled pair: PAIR\n"));
    // the same result every time
    let again = run_pair(&fix, &many);
    assert_eq!(plan.to_json(), again.to_json());
    assert_eq!(plan.tracks, again.tracks);
    assert_eq!(plan.vias, again.vias);
}

#[test]
fn more_changes_of_layer_change_nothing_for_a_pair_that_crosses_once() {
    let mut fix = Fix::new(30, 14, true);
    let p = fix.net("P");
    let n = fix.net("N");
    fix.pad(p, mm(4), mm(7), false);
    fix.pad(n, mm(4), mm(6), false);
    fix.pad_bottom(p, mm(26), mm(7));
    fix.pad_bottom(n, mm(26), mm(6));
    let once = AutorouteConfig {
        couple_via: true,
        ..cfg()
    };
    let more = AutorouteConfig {
        couple_layers_max: 4,
        ..once.clone()
    };
    let (a, b) = (run_pair(&fix, &once), run_pair(&fix, &more));
    assert_eq!(a.to_json(), b.to_json());
    assert_eq!(a.tracks, b.tracks);
    assert_eq!(a.vias, b.vias);
}
