// SPDX-License-Identifier: GPL-3.0-or-later
//! The requests of the server mode made from a board built by the test itself: the sizes, the
//! gaps between copper, the holes (with the places measured from the corner of the outline) and
//! the parts, written as the JSON that the plugin `pcb` of the server reads.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadShape, Segment, Side, Track,
    Via,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_server::codes::{Severity, NOTHING_TO_SEND, OPTION_INVALID, PARTIAL_REQUEST};
use eh3d_phoenix_server::request::{facts_from_board, rules_request};
use eh3d_phoenix_server::{prepare_board, Kind, ServerSettings};
use eh3d_serverclient::Json;

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
    Segment::Line {
        start: pt(a.0, a.1),
        end: pt(b.0, b.1),
    }
}

fn track(net: u32, layer: u32, width: i64, path: Vec<Segment>) -> Track {
    Track {
        net: id(net),
        layer: id(layer),
        width: nm(width),
        path,
    }
}

fn pad(number: &str, at: Point, drill: Option<i64>) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: nm(1_600_000),
        },
        at,
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers: vec![id(1)],
        roundness: 0,
        plated: true,
    }
}

/// A 50 x 30 mm board whose corner is at 1, 1 mm, with the layers Top and Bottom, the nets GND and
/// SIG, a GND track and a SIG track 0.5 mm apart on Top (so 0.325 mm of air between the copper), a
/// GND track further away, a SIG track on Bottom, a via, a resistor and a connector with a hole.
fn board() -> Board {
    let mut b = Board {
        name: "demo".into(),
        outline: vec![
            pt(MM, MM),
            pt(51 * MM, MM),
            pt(51 * MM, 31 * MM),
            pt(MM, 31 * MM),
        ],
        copper_stack: vec![id(1), id(2)],
        ..Board::default()
    };
    for (n, name) in [(1, "Top"), (2, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    b.nets.insert(id(5), Net { name: "GND".into() });
    b.nets.insert(id(6), Net { name: "SIG".into() });
    b.tracks.insert(
        id(20),
        track(
            5,
            1,
            150_000,
            vec![line((5 * MM, 5 * MM), (25 * MM, 5 * MM))],
        ),
    );
    b.tracks.insert(
        id(21),
        track(
            6,
            1,
            200_000,
            vec![line(
                (5 * MM, 5 * MM + 500_000),
                (25 * MM, 5 * MM + 500_000),
            )],
        ),
    );
    b.tracks.insert(
        id(22),
        track(
            5,
            1,
            150_000,
            vec![line((5 * MM, 20 * MM), (25 * MM, 20 * MM))],
        ),
    );
    b.tracks.insert(
        id(23),
        track(
            6,
            2,
            200_000,
            vec![line((5 * MM, 5 * MM), (25 * MM, 5 * MM))],
        ),
    );
    b.vias.insert(
        id(30),
        Via {
            net: id(5),
            at: pt(10 * MM, 10 * MM),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(1),
            to_layer: id(2),
        },
    );
    let mut resistor = Footprint {
        name: "R_0603".into(),
        pads: Default::default(),
    };
    resistor
        .pads
        .insert(id(11), pad("1", pt(-800_000, 0), None));
    resistor.pads.insert(id(12), pad("2", pt(800_000, 0), None));
    b.footprints.insert(id(10), resistor);
    let mut header = Footprint {
        name: "HDR".into(),
        pads: Default::default(),
    };
    header
        .pads
        .insert(id(14), pad("1", pt(0, 0), Some(1_000_000)));
    b.footprints.insert(id(13), header);
    for (n, refdes, value, fp, at) in [
        (40, "R1", "10k", 10, (30 * MM, 10 * MM)),
        (41, "J1", "HDR1", 13, (41 * MM, 21 * MM)),
    ] {
        b.components.insert(
            id(n),
            Component {
                refdes: refdes.into(),
                value: value.into(),
                symbol: None,
                footprint: id(fp),
                at: pt(at.0, at.1),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
    }
    b
}

fn settings() -> ServerSettings {
    ServerSettings::default()
}

#[test]
fn the_facts_of_a_board_are_its_sizes_gaps_holes_and_parts() {
    let facts = facts_from_board(&board(), &settings()).unwrap();
    assert_eq!(facts.outline_nm, Some((50 * MM, 30 * MM)));
    assert_eq!(facts.tracks.len(), 4);
    assert_eq!(facts.parts.len(), 2);
    // the only pair of different nets on the same layer that is closer than 1 mm
    assert_eq!(facts.clearances.len(), 1);
    assert_eq!(facts.clearances[0].gap_nm, 325_000);
    assert_eq!(facts.clearances[0].name, "GND/SIG Top");
    // the via and the hole of the connector, measured from the corner of the outline
    assert_eq!(facts.drills.len(), 2);
    assert_eq!(
        (facts.drills[0].x_nm, facts.drills[0].y_nm),
        (9 * MM, 9 * MM)
    );
    assert_eq!(facts.drills[0].name, "via GND");
    assert_eq!(
        (facts.drills[1].x_nm, facts.drills[1].dia_nm),
        (40 * MM, 1_000_000)
    );
    assert_eq!(facts.drills[1].name, "J1.1");
    assert!(facts.notes.is_empty(), "{:?}", facts.notes);
}

#[test]
fn the_request_of_the_rules_holds_the_millimetres_of_the_board() {
    let p = prepare_board(Kind::Rules, &board(), &settings()).unwrap();
    assert_eq!(p.items, 4 + 1 + 2);
    let j = Json::parse(&p.request).unwrap();
    let b = j.get("board").unwrap();
    assert_eq!(
        b.get("outline").unwrap().get("height_mm"),
        Some(&Json::Num(30.0))
    );
    let gaps = b.get("clearances").unwrap().as_array().unwrap();
    assert_eq!(gaps[0].get("gap_mm"), Some(&Json::Num(0.325)));
    let drills = b.get("drills").unwrap().as_array().unwrap();
    assert_eq!(drills[0].get("x"), Some(&Json::Num(9.0)));
    assert_eq!(drills[1].get("dia_mm"), Some(&Json::Num(1.0)));
    // the same board gives the same request
    assert_eq!(
        p.request,
        prepare_board(Kind::Rules, &board(), &settings())
            .unwrap()
            .request
    );
}

#[test]
fn the_other_kinds_take_the_holes_and_the_parts_and_the_inspection_takes_no_board() {
    let drill = prepare_board(Kind::Drill, &board(), &settings()).unwrap();
    assert_eq!(drill.items, 2);
    let bom = prepare_board(Kind::Bom, &board(), &settings()).unwrap();
    assert_eq!(bom.items, 2);
    let j = Json::parse(&bom.request).unwrap();
    let parts = j.get("components").unwrap().as_array().unwrap();
    let footprints: Vec<&str> = parts
        .iter()
        .filter_map(|p| p.get("footprint").and_then(Json::as_str))
        .collect();
    assert_eq!(footprints, ["R_0603", "HDR"]);
    let e = prepare_board(Kind::Inspect, &board(), &settings()).unwrap_err();
    assert_eq!(e.code, OPTION_INVALID);
}

#[test]
fn a_board_with_nothing_for_a_report_is_refused_with_its_code() {
    let empty = Board::default();
    for kind in [Kind::Rules, Kind::Drill, Kind::Bom] {
        let e = prepare_board(kind, &empty, &settings()).unwrap_err();
        assert_eq!(e.code, NOTHING_TO_SEND, "{kind:?}");
    }
}

#[test]
fn the_gaps_can_be_turned_off_or_limited_by_the_settings() {
    let none = ServerSettings {
        clearance_max: 0,
        ..settings()
    };
    let facts = facts_from_board(&board(), &none).unwrap();
    assert!(facts.clearances.is_empty() && facts.notes.is_empty());
    // a gap of 0.325 mm is not listed when the limit is below it
    let tight = ServerSettings {
        clearance_below_nm: 300_000,
        ..settings()
    };
    assert!(facts_from_board(&board(), &tight)
        .unwrap()
        .clearances
        .is_empty());
    // too many tracks for the gaps: none are listed and the note says so
    let few = ServerSettings {
        clearance_max_tracks: 3,
        ..settings()
    };
    let facts = facts_from_board(&board(), &few).unwrap();
    assert!(facts.clearances.is_empty());
    assert_eq!(facts.notes.len(), 1);
    assert_eq!(facts.notes[0].code, PARTIAL_REQUEST);
    assert!(facts.notes[0].detail.contains("clearance_max_tracks"));
}

#[test]
fn a_pair_budget_that_is_too_small_is_a_warning_and_an_arc_is_taken_as_its_chord() {
    let mut b = board();
    // two pieces in each of two tracks of different nets: four pairs of pieces
    b.tracks.insert(
        id(24),
        track(
            5,
            2,
            150_000,
            vec![
                line((5 * MM, 8 * MM), (10 * MM, 8 * MM)),
                line((10 * MM, 8 * MM), (15 * MM, 8 * MM)),
            ],
        ),
    );
    b.tracks.insert(
        id(25),
        track(
            6,
            2,
            150_000,
            vec![
                line((5 * MM, 9 * MM), (10 * MM, 9 * MM)),
                Segment::Arc {
                    start: pt(10 * MM, 9 * MM),
                    end: pt(15 * MM, 9 * MM),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        ),
    );
    let small = ServerSettings {
        clearance_pair_budget: 2,
        ..settings()
    };
    let facts = facts_from_board(&b, &small).unwrap();
    assert!(facts
        .notes
        .iter()
        .any(|n| n.severity == Severity::Warning && n.detail.contains("clearance_pair_budget")));
    let facts = facts_from_board(&b, &settings()).unwrap();
    assert!(facts
        .notes
        .iter()
        .any(|n| n.severity == Severity::Info && n.detail.contains("chord")));
    // the pair on Bottom is 1 mm apart from centre to centre, 0.85 mm of air between the copper
    assert_eq!(facts.clearances.len(), 1 + 1);
}

#[test]
fn an_outline_without_area_sends_no_outline_and_says_so() {
    let mut b = board();
    b.outline = vec![pt(0, 0), pt(MM, 0)];
    let facts = facts_from_board(&b, &settings()).unwrap();
    assert_eq!(facts.outline_nm, None);
    assert!(facts.notes.iter().any(|n| n.detail.contains("outline")));
    // the holes are then measured from the origin of the board
    assert_eq!(facts.drills[0].x_nm, 10 * MM);
    let text = rules_request(&facts, &settings()).unwrap();
    assert!(!text.contains("\"outline\""));
}

/// A generator with a fixed seed, so every run makes the same boards.
fn next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 33
}

#[test]
fn the_gaps_of_generated_horizontal_tracks_follow_an_independent_calculation() {
    let mut seed = 0x5eed_0004_u64;
    let mut found_any = 0;
    for _ in 0..25 {
        let mut b = Board {
            name: "generated".into(),
            outline: vec![
                pt(0, 0),
                pt(40 * MM, 0),
                pt(40 * MM, 20 * MM),
                pt(0, 20 * MM),
            ],
            ..Board::default()
        };
        for (n, name) in [(1, "Top"), (2, "Bottom")] {
            b.layers.insert(
                id(n),
                Layer {
                    name: name.into(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        for (n, name) in [(5, "A"), (6, "B"), (7, "C")] {
            b.nets.insert(id(n), Net { name: name.into() });
        }
        // net, layer, y, from x, to x and width of each track, in nanometres
        let mut shapes = Vec::new();
        for k in 0..10 {
            let net = next(&mut seed) % 3;
            let layer = next(&mut seed) % 2;
            let y = (next(&mut seed) % 20) as i64 * 100_000;
            let x1 = (next(&mut seed) % 30) as i64 * 100_000;
            let x2 = x1 + (1 + next(&mut seed) % 20) as i64 * 100_000;
            let width = (1 + next(&mut seed) % 4) as i64 * 50_000;
            b.tracks.insert(
                id(100 + k),
                track(
                    5 + net as u32,
                    1 + layer as u32,
                    width,
                    vec![line((x1, y), (x2, y))],
                ),
            );
            shapes.push((net, layer, y, x1, x2, width));
        }
        let below = 1_000_000;
        let s = ServerSettings {
            clearance_below_nm: below,
            clearance_max: 1_000,
            ..settings()
        };
        let facts = facts_from_board(&b, &s).unwrap();
        let found: Vec<i64> = facts.clearances.iter().map(|c| c.gap_nm).collect();
        // the same gaps from the formula for two horizontal pieces: the distance along x that
        // separates them and the distance across, less the two half widths
        let mut expected = Vec::new();
        for (i, a) in shapes.iter().enumerate() {
            for c in &shapes[i + 1..] {
                if a.0 == c.0 || a.1 != c.1 {
                    continue;
                }
                let along = (a.3.max(c.3) - a.4.min(c.4)).max(0) as f64;
                let across = (a.2 - c.2).abs() as f64;
                let distance = along.hypot(across);
                let gap = (distance - (a.5 + c.5) as f64 / 2.0).max(0.0).floor() as i64;
                if gap < below {
                    expected.push(gap);
                }
            }
        }
        expected.sort_unstable();
        assert_eq!(found, expected, "{shapes:?}");
        assert!(found.windows(2).all(|w| w[0] <= w[1]));
        found_any += found.len();
    }
    // the generator puts tracks close enough that the check was not met by empty lists
    assert!(found_any > 50, "{found_any}");
}
