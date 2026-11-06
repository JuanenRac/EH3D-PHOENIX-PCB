// SPDX-License-Identifier: GPL-3.0-or-later
//! Small boards with a result that can be worked out by hand: two pads in line, an obstacle that
//! has to be gone around, a layer change with two vias, two nets that cross, a net that cannot be
//! routed, the limits that count nodes, the requests that name nets and layers, the report in
//! text and JSON, and the plan as operations of the board editor.

use eh3d_phoenix_board::Operation;
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings};
use eh3d_phoenix_route::RouteSettings;

use crate::codes::{
    BOARD_NOT_ROUTABLE, EXPANSION_LIMIT, INCOMPLETE, LAYER_UNKNOWN, NET_UNKNOWN, NO_PATH,
};
use crate::config::AutorouteConfig;
use crate::plan::AutoroutePlan;
use crate::testboard::{cfg, ctx, mm, pt, Fix};
use crate::{autoroute, Request};

fn run(fix: &Fix, config: &AutorouteConfig, request: &Request) -> AutoroutePlan {
    autoroute(
        &fix.board,
        &ctx(),
        &RouteSettings::default(),
        config,
        request,
    )
    .unwrap()
}

fn two_pads_in_line() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    fix
}

/// A wall of another net across the top layer, from 0.5 mm to 9.5 mm: nothing passes on top.
fn board_with_a_wall() -> Fix {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let wall = fix.net("W");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    let top = fix.top;
    fix.wall(wall, top, (mm(6), mm(1) / 2), (mm(6), mm(19) / 2));
    fix
}

#[test]
fn two_pads_in_line_get_one_straight_track() {
    let fix = two_pads_in_line();
    let plan = run(&fix, &cfg(), &Request::default());
    assert_eq!(plan.tracks.len(), 1);
    assert!(plan.vias.is_empty());
    let track = &plan.tracks[0];
    assert_eq!(track.layer, fix.top);
    assert_eq!(track.width_nm, 250_000);
    assert_eq!(track.points, vec![pt(mm(2), mm(5)), pt(mm(10), mm(5))]);
    assert_eq!(plan.report.length_nm, 8_000_000);
    assert_eq!(plan.report.connections_total, 1);
    assert_eq!(plan.report.connections_routed, 1);
    assert_eq!(plan.report.nets_completed, vec!["SIG".to_owned()]);
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_before, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(plan.report.unrouted.is_empty());
    plan.require_complete().unwrap();
}

#[test]
fn an_obstacle_in_the_way_is_gone_around_with_two_corners() {
    let mut fix = Fix::new(20, 10, true);
    let sig = fix.net("SIG");
    let other = fix.net("OTHER");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    fix.pad(other, mm(6), mm(5), false);
    let plan = run(&fix, &cfg(), &Request::default());
    assert_eq!(plan.tracks.len(), 1);
    assert!(plan.vias.is_empty());
    // a track keeps 0.626 mm from the centre of the pad, so it passes 1 mm above or below it:
    // one diagonal piece away from the first pad, a straight run and one diagonal piece back
    let above = vec![
        pt(mm(2), mm(5)),
        pt(mm(3), mm(4)),
        pt(mm(9), mm(4)),
        pt(mm(10), mm(5)),
    ];
    let below = vec![
        pt(mm(2), mm(5)),
        pt(mm(3), mm(6)),
        pt(mm(9), mm(6)),
        pt(mm(10), mm(5)),
    ];
    let points = &plan.tracks[0].points;
    assert!(*points == above || *points == below, "{points:?}");
    assert_eq!(plan.report.length_nm, 8_828_428);
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
}

#[test]
fn a_wall_on_the_top_layer_needs_a_via_down_and_a_via_up() {
    let fix = board_with_a_wall();
    let plan = run(&fix, &cfg(), &Request::default());
    assert_eq!(plan.vias.len(), 2);
    assert!(plan.tracks.iter().any(|t| t.layer == fix.bottom));
    for via in &plan.vias {
        assert_eq!((via.from_layer, via.to_layer), (fix.top, fix.bottom));
        assert_eq!((via.drill_nm, via.diameter_nm), (300_000, 600_000));
    }
    assert_eq!(plan.report.nets_completed, vec!["SIG".to_owned()]);
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_after, 0);
    // the wall is not touched
    assert_eq!(
        plan.board_after.tracks.len(),
        fix.board.tracks.len() + plan.tracks.len()
    );
    for (id, track) in &fix.board.tracks {
        assert_eq!(plan.board_after.tracks.get(id), Some(track));
    }
}

#[test]
fn restricted_to_the_top_layer_the_wall_cannot_be_passed() {
    let fix = board_with_a_wall();
    let request = Request {
        nets: Vec::new(),
        layers: vec!["Top".to_owned()],
    };
    let plan = run(&fix, &cfg(), &request);
    assert!(plan.tracks.is_empty() && plan.vias.is_empty());
    assert_eq!(plan.report.unrouted.len(), 1);
    assert_eq!(plan.report.unrouted[0].code, NO_PATH);
}

#[test]
fn two_nets_that_cross_both_get_a_route() {
    let mut fix = Fix::new(20, 10, true);
    let a = fix.net("A");
    let b = fix.net("B");
    fix.pad(a, mm(2), mm(2), true);
    fix.pad(a, mm(18), mm(8), true);
    fix.pad(b, mm(2), mm(8), true);
    fix.pad(b, mm(18), mm(2), true);
    let plan = run(&fix, &cfg(), &Request::default());
    assert_eq!(
        plan.report.nets_completed,
        vec!["A".to_owned(), "B".to_owned()]
    );
    assert_eq!(plan.report.connections_total, 2);
    assert_eq!(plan.report.connections_routed, 2);
    assert_eq!(plan.report.connections_left, 0);
    assert_eq!(plan.report.errors_before, 0);
    assert_eq!(plan.report.errors_after, 0);
    assert!(plan.board_after.connectivity().is_fully_connected());
}

#[test]
fn a_net_with_no_way_is_reported_with_its_reason_and_nothing_is_added() {
    let mut fix = Fix::new(16, 10, false);
    let sig = fix.net("SIG");
    let wall = fix.net("W");
    fix.pad(sig, mm(2), mm(5), false);
    fix.pad(sig, mm(10), mm(5), false);
    let top = fix.top;
    fix.wall(wall, top, (mm(6), 0), (mm(6), mm(10)));
    let plan = run(&fix, &cfg(), &Request::default());
    assert!(plan.tracks.is_empty() && plan.vias.is_empty());
    assert_eq!(plan.report.connections_total, 1);
    assert_eq!(plan.report.connections_routed, 0);
    assert!(plan.report.nets_completed.is_empty());
    assert_eq!(plan.report.unrouted.len(), 1);
    let missed = &plan.report.unrouted[0];
    assert_eq!(missed.net, "SIG");
    assert_eq!(missed.code, NO_PATH);
    assert_eq!(plan.operation, Operation::Batch { ops: Vec::new() });
    assert_eq!(plan.board_after, fix.board);
    assert_eq!(plan.require_complete().unwrap_err().code, INCOMPLETE);
    let text = plan.to_text("pt");
    assert!(
        text.contains("Não existe caminho na grelha de encaminhamento"),
        "{text}"
    );
    assert!(plan
        .to_text("en")
        .contains("No path exists on the routing grid"));
}

#[test]
fn the_limits_count_nodes_and_not_time() {
    let fix = two_pads_in_line();
    let tight = AutorouteConfig {
        max_expansions_per_search: 3,
        ..cfg()
    };
    let plan = run(&fix, &tight, &Request::default());
    assert!(plan.tracks.is_empty());
    assert_eq!(plan.report.unrouted.len(), 1);
    assert_eq!(plan.report.unrouted[0].code, EXPANSION_LIMIT);
    // the search and the search through other copper: each stops after the limit and one more
    assert!(plan.report.expansions <= 8, "{}", plan.report.expansions);
    let none = AutorouteConfig {
        max_expansions_total: 1,
        ..cfg()
    };
    let plan = run(&fix, &none, &Request::default());
    assert!(plan.tracks.is_empty());
    assert!(plan.report.expansions <= 4, "{}", plan.report.expansions);
}

#[test]
fn the_request_names_nets_and_layers_and_a_wrong_name_is_refused() {
    let mut fix = two_pads_in_line();
    let second = fix.net("SECOND");
    fix.pad(second, mm(2), mm(8), false);
    fix.pad(second, mm(10), mm(8), false);
    let only = Request {
        nets: vec!["SIG".to_owned()],
        layers: Vec::new(),
    };
    let plan = run(&fix, &cfg(), &only);
    assert_eq!(plan.report.connections_total, 1);
    assert_eq!(plan.report.nets_completed, vec!["SIG".to_owned()]);
    assert!(plan.tracks.iter().all(|t| t.points[0].y_nm.get() == mm(5)));
    let both = run(&fix, &cfg(), &Request::default());
    assert_eq!(both.report.connections_total, 2);
    let (rules, route, config) = (ctx(), RouteSettings::default(), cfg());
    let unknown_net = Request {
        nets: vec!["NOPE".to_owned()],
        layers: Vec::new(),
    };
    let e = autoroute(&fix.board, &rules, &route, &config, &unknown_net).unwrap_err();
    assert_eq!(e.code, NET_UNKNOWN);
    let unknown_layer = Request {
        nets: Vec::new(),
        layers: vec!["Inner".to_owned()],
    };
    let e = autoroute(&fix.board, &rules, &route, &config, &unknown_layer).unwrap_err();
    assert_eq!(e.code, LAYER_UNKNOWN);
    let mut bare = fix.board.clone();
    bare.outline.clear();
    let e = autoroute(&bare, &rules, &route, &config, &Request::default()).unwrap_err();
    assert_eq!(e.code, BOARD_NOT_ROUTABLE);
}

#[test]
fn the_report_is_text_and_json_and_the_same_every_time() {
    let fix = two_pads_in_line();
    let first = run(&fix, &cfg(), &Request::default());
    let second = run(&fix, &cfg(), &Request::default());
    assert_eq!(first.to_text("en"), second.to_text("en"));
    assert_eq!(first.to_json(), second.to_json());
    assert_eq!(first.report, second.report);
    let text = first.to_text("en");
    assert!(text.starts_with("autoroute: 1 of 1 connections routed, 1 of 1 nets complete\n"));
    assert!(text.contains("tracks added: 1, vias added: 0, length: 8000000 nm\n"));
    assert!(text.contains("complete: SIG\n"));
    let json = first.to_json();
    assert!(json.starts_with("{\"nets_total\":1,\"nets_completed\":[\"SIG\"],"));
    assert!(json.ends_with("\"unrouted\":[]}"));
}

#[test]
fn the_plan_is_one_reversible_step_and_the_same_as_a_group_of_editor_operations() {
    let fix = board_with_a_wall();
    let settings = BoardEditSettings::default();
    let route = settings.route_settings();
    let plan = autoroute(&fix.board, &ctx(), &route, &cfg(), &Request::default()).unwrap();
    assert_eq!(plan.vias.len(), 2);
    // the operation of the board has an exact inverse
    let mut board = fix.board.clone();
    plan.apply(&mut board).unwrap();
    assert_eq!(board, plan.board_after);
    assert!(board.connectivity().is_fully_connected());
    // the group of the editor is one step, checked by the rules, and undoes cleanly
    let mut edit = BoardEdit::new(fix.board.clone(), ctx(), settings).unwrap();
    let outcome = edit.apply(&plan.edit_ops()).unwrap();
    assert!(!outcome.drc.adds_error());
    assert_eq!(edit.steps(), 1);
    assert!(edit.ratsnest().is_empty());
    assert!(edit.undo());
    assert_eq!(edit.board(), &fix.board);
}
