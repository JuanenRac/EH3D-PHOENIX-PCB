// SPDX-License-Identifier: GPL-3.0-or-later
//! The quality and the time of the autorouter on a set of boards that the test makes itself from
//! fixed formulas: pads on a lattice, nets of two to four pads that lie near each other, a part of
//! the pads through the board. For each board the test counts the connections routed, the vias,
//! the length of the tracks, the nodes expanded and the tracks torn up.
//!
//! The quick test routes the small board and holds it to a floor; the table of all the boards is
//! printed by the test that is left out of a normal run:
//! `cargo test --release -p eh3d-phoenix-autoroute --test quality -- --ignored --nocapture`.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use eh3d_phoenix_autoroute::{autoroute, AutorouteConfig, AutoroutePlan, Request};
use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::{RouteSettings, RuleContext};
use eh3d_phoenix_rules::RuleValues;

/// A small generator of numbers, the same on every computer.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n.max(1)
    }
}

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

struct Case {
    name: &'static str,
    width_mm: i64,
    height_mm: i64,
    pitch_nm: i64,
    nets: usize,
    seed: u64,
}

const CASES: [Case; 4] = [
    Case {
        name: "small",
        width_mm: 30,
        height_mm: 30,
        pitch_nm: 2_500_000,
        nets: 40,
        seed: 11,
    },
    Case {
        name: "medium",
        width_mm: 50,
        height_mm: 50,
        pitch_nm: 2_500_000,
        nets: 110,
        seed: 23,
    },
    Case {
        name: "dense",
        width_mm: 40,
        height_mm: 40,
        pitch_nm: 2_000_000,
        nets: 130,
        seed: 37,
    },
    Case {
        name: "large",
        width_mm: 90,
        height_mm: 70,
        pitch_nm: 2_500_000,
        nets: 320,
        seed: 41,
    },
];

/// The board of a case: one pad on some places of the lattice, each net with two to four pads
/// within a few places of each other.
fn board_of(case: &Case) -> Board {
    let (top, bottom, smd, thru, smd_pad, thru_pad) =
        (id(1), id(2), id(10), id(11), id(12), id(13));
    let mut board = Board {
        name: case.name.to_owned(),
        ..Board::default()
    };
    let (w, h) = (case.width_mm * 1_000_000, case.height_mm * 1_000_000);
    let pt = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
    board.outline = vec![pt(0, 0), pt(w, 0), pt(w, h), pt(0, h)];
    for (layer, name, side) in [(top, "Top", Side::Top), (bottom, "Bottom", Side::Bottom)] {
        board.layers.insert(
            layer,
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: Some(side),
            },
        );
    }
    board.copper_stack = vec![top, bottom];
    for (footprint, pad, layers, drill) in [
        (smd, smd_pad, vec![top], None),
        (
            thru,
            thru_pad,
            vec![top, bottom],
            Some(Nm::new(400_000).unwrap()),
        ),
    ] {
        let mut pads = BTreeMap::new();
        pads.insert(
            pad,
            Pad {
                number: "1".to_owned(),
                shape: PadShape::Circle {
                    diameter: Nm::new(900_000).unwrap(),
                },
                at: pt(0, 0),
                rotation: Udeg::ZERO,
                drill,
                layers,
                roundness: 0,
                plated: true,
            },
        );
        board.footprints.insert(
            footprint,
            Footprint {
                name: "P".to_owned(),
                pads,
            },
        );
    }
    let columns = (w - 2_000_000) / case.pitch_nm + 1;
    let rows = (h - 2_000_000) / case.pitch_nm + 1;
    let mut rng = Lcg(case.seed);
    let mut used: BTreeSet<(i64, i64)> = BTreeSet::new();
    let mut next = 1000_u32;
    let mut made = 0_usize;
    let mut tries = 0_usize;
    while made < case.nets && tries < case.nets * 50 {
        tries += 1;
        let first = (
            rng.below(columns as u64) as i64,
            rng.below(rows as u64) as i64,
        );
        if used.contains(&first) {
            continue;
        }
        let want = match rng.below(10) {
            0 => 4,
            1 | 2 => 3,
            _ => 2,
        };
        let mut places = vec![first];
        for _ in 0..40 {
            if places.len() == want {
                break;
            }
            let near = (
                first.0 + rng.below(13) as i64 - 6,
                first.1 + rng.below(13) as i64 - 6,
            );
            if near.0 < 0
                || near.1 < 0
                || near.0 >= columns
                || near.1 >= rows
                || used.contains(&near)
                || places.contains(&near)
            {
                continue;
            }
            places.push(near);
        }
        if places.len() < 2 {
            continue;
        }
        next += 1;
        let net = id(next);
        board.nets.insert(
            net,
            Net {
                name: format!("N{made}"),
            },
        );
        made += 1;
        for place in places {
            used.insert(place);
            next += 1;
            let component = id(next);
            let through = rng.below(10) < 3;
            let (footprint, pad) = if through {
                (thru, thru_pad)
            } else {
                (smd, smd_pad)
            };
            board.components.insert(
                component,
                Component {
                    refdes: format!("P{next}"),
                    value: "x".to_owned(),
                    symbol: None,
                    footprint,
                    at: pt(
                        1_000_000 + place.0 * case.pitch_nm,
                        1_000_000 + place.1 * case.pitch_nm,
                    ),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
            board.pad_nets.push(PadNet {
                component,
                pad,
                net,
            });
        }
    }
    board.pad_nets.sort_by_key(PadNet::pad_ref);
    board
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: Nm::new(200_000).unwrap(),
            min_track_width: Nm::new(150_000).unwrap(),
            min_edge_clearance: Nm::new(250_000).unwrap(),
            ..RuleValues::default()
        },
        classes: Default::default(),
    }
}

fn route(case: &Case) -> (AutoroutePlan, u128) {
    let board = board_of(case);
    let started = Instant::now();
    let plan = autoroute(
        &board,
        &rules(),
        &RouteSettings::default(),
        &AutorouteConfig::default(),
        &Request::default(),
    )
    .unwrap();
    (plan, started.elapsed().as_millis())
}

fn line(case: &Case, plan: &AutoroutePlan, ms: u128) -> String {
    let r = &plan.report;
    format!(
        "AUTOROUTE {}: connections {}/{} ({:.1} %), vias {}, length {:.1} mm, expansions {}, ripups {}, errors {}->{}, {} ms",
        case.name,
        r.connections_routed,
        r.connections_total,
        100.0 * r.connections_routed as f64 / r.connections_total.max(1) as f64,
        r.vias_added,
        r.length_nm as f64 / 1e6,
        r.expansions,
        r.ripups,
        r.errors_before,
        r.errors_after,
        ms
    )
}

#[test]
fn the_small_board_of_the_set_is_routed_the_same_twice_and_adds_no_rule_error() {
    let case = &CASES[0];
    let (first, ms) = route(case);
    println!("{}", line(case, &first, ms));
    let (second, _) = route(case);
    assert_eq!(first.report, second.report);
    assert_eq!(first.board_after, second.board_after);
    let r = &first.report;
    assert!(
        r.errors_after <= r.errors_before,
        "{}",
        line(case, &first, ms)
    );
    assert!(
        r.connections_routed * 100 >= r.connections_total * 80,
        "{}",
        line(case, &first, ms)
    );
}

#[test]
#[ignore = "the table of the whole set takes long in a debug build"]
fn the_table_of_the_boards_of_the_set() {
    for case in &CASES {
        let (plan, ms) = route(case);
        println!("{}", line(case, &plan, ms));
    }
    let d = AutorouteConfig::default();
    let variants = [
        ("nopass", AutorouteConfig { optimize_passes: 0, ..d.clone() }),
        ("rips400", AutorouteConfig { max_ripup_iterations: 400, ..d.clone() }),
        ("rips2000x8", AutorouteConfig { max_ripup_iterations: 2000, max_rips_per_connection: 8, max_expansions_total: 60_000_000, ..d.clone() }),
        ("total60M", AutorouteConfig { max_expansions_total: 60_000_000, ..d.clone() }),
        ("rips400total60M", AutorouteConfig { max_ripup_iterations: 400, max_expansions_total: 60_000_000, ..d }),
    ];
    for (name, cfg) in &variants {
        for case in &CASES {
            let board = board_of(case);
            let started = Instant::now();
            let plan = autoroute(&board, &rules(), &RouteSettings::default(), cfg, &Request::default()).unwrap();
            println!("{name} {}", line(case, &plan, started.elapsed().as_millis()));
        }
    }
}
