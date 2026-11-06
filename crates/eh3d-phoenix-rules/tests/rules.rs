// SPDX-License-Identifier: GPL-3.0-or-later
//! Boards made by hand with deliberate errors: the engine must find every one of them, always, and
//! the same board with the same rules must always give the same report.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::{
    analyse_board, check_board, check_connectivity, compare_schematic, Finding, Lang, Report,
    RuleValues, Severity,
};
use eh3d_phoenix_schematic::{NetPin, Part, Pin, Schematic, SchematicNet, Symbol};

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
fn round(d: i64) -> PadShape {
    PadShape::Circle { diameter: nm(d) }
}

const TOP: u32 = 1;
const BOT: u32 = 2;
const MASK: u32 = 3;
const SILK: u32 = 4;

/// An empty board of 20 mm by 20 mm with two copper layers, a mask layer and a top silkscreen.
fn base() -> Board {
    let mut b = Board::default();
    for (n, name, kind, side) in [
        (TOP, "Top", LayerKind::Copper, None),
        (BOT, "Bottom", LayerKind::Copper, None),
        (MASK, "Mask", LayerKind::SolderMask, None),
        (SILK, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
    ] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind,
                side,
            },
        );
    }
    b.copper_stack = vec![id(TOP), id(BOT)];
    b.outline = vec![
        pt(0, 0),
        pt(20_000_000, 0),
        pt(20_000_000, 20_000_000),
        pt(0, 20_000_000),
    ];
    b
}

fn net(b: &mut Board, n: u32, name: &str) -> Id {
    b.nets.insert(id(n), Net { name: name.into() });
    id(n)
}

fn track(b: &mut Board, n: u32, net: Id, width: i64, path: Vec<Segment>) {
    b.tracks.insert(
        id(n),
        Track {
            net,
            layer: id(TOP),
            width: nm(width),
            path,
        },
    );
}

fn via(b: &mut Board, n: u32, net: Id, at: (i64, i64), drill: i64, diameter: i64) {
    b.vias.insert(
        id(n),
        Via {
            net,
            at: pt(at.0, at.1),
            drill: nm(drill),
            diameter: nm(diameter),
            from_layer: id(TOP),
            to_layer: id(BOT),
        },
    );
}

/// One component U1 with the given pads (number, position, shape, drill, layers, net).
type PadSpec = (
    &'static str,
    (i64, i64),
    PadShape,
    Option<i64>,
    Vec<u32>,
    Option<Id>,
);

fn component(b: &mut Board, at: (i64, i64), pads: Vec<PadSpec>) {
    let (fp, comp) = (id(40), id(43));
    let mut map = BTreeMap::new();
    let mut pad_nets = Vec::new();
    for (i, (number, pos, shape, drill, layers, net)) in pads.into_iter().enumerate() {
        let pid = id(41 + i as u32);
        map.insert(
            pid,
            Pad {
                number: number.into(),
                shape,
                at: pt(pos.0, pos.1),
                rotation: Udeg::ZERO,
                drill: drill.map(nm),
                layers: layers.into_iter().map(id).collect(),
                roundness: 0,
                plated: true,
            },
        );
        if let Some(net) = net {
            pad_nets.push(PadNet {
                component: comp,
                pad: pid,
                net,
            });
        }
    }
    b.footprints.insert(
        fp,
        Footprint {
            name: "F".into(),
            pads: map,
        },
    );
    b.components.insert(
        comp,
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: fp,
            at: pt(at.0, at.1),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    pad_nets.sort_by_key(|p| p.pad_ref());
    b.pad_nets = pad_nets;
}

/// A board with one deliberate error for each of the rules that measure.
fn faulty_board() -> Board {
    let mut b = base();
    let (na, nb) = (net(&mut b, 10, "A"), net(&mut b, 11, "B"));
    // too narrow (0.10 mm)
    track(
        &mut b,
        20,
        na,
        100_000,
        vec![line((2_000_000, 2_000_000), (8_000_000, 2_000_000))],
    );
    // 0.10 mm from the first track, edge to edge
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((2_000_000, 2_250_000), (8_000_000, 2_250_000))],
    );
    // 0.10 mm from the top edge of the board
    track(
        &mut b,
        22,
        na,
        200_000,
        vec![line((2_000_000, 19_800_000), (8_000_000, 19_800_000))],
    );
    // too small in diameter, in drill and in ring
    via(&mut b, 30, na, (12_000_000, 5_000_000), 200_000, 400_000);
    // overlaps the first via, and its drill is close to the other one
    via(&mut b, 31, nb, (12_500_000, 5_000_000), 300_000, 700_000);
    // a ring of 0.10 mm
    component(
        &mut b,
        (5_000_000, 10_000_000),
        vec![
            (
                "1",
                (0, 0),
                round(800_000),
                Some(600_000),
                vec![TOP, BOT],
                Some(na),
            ),
            (
                "2",
                (0, 1_000_000),
                round(800_000),
                Some(400_000),
                vec![TOP, BOT],
                Some(nb),
            ),
        ],
    );
    b
}

const RULES_JSON: &str = r#"{"format":"phoenix-rules/1","name":"test","rules":{
    "min_track_width_nm":150000,"min_clearance_nm":200000,"min_drill_nm":300000,
    "min_via_diameter_nm":600000,"min_annular_ring_nm":150000,"min_edge_clearance_nm":300000,
    "min_drill_spacing_nm":400000}}"#;

fn rules() -> RuleValues {
    RuleValues::from_json(RULES_JSON).unwrap().1
}

fn by_rule<'a>(report: &'a Report, rule: &str) -> Vec<&'a Finding> {
    report.confirmed.iter().filter(|f| f.rule == rule).collect()
}

#[test]
fn every_deliberate_error_is_found_with_its_measured_value_and_limit() {
    let report = check_board(&faulty_board(), &rules());

    let width = by_rule(&report, "track_width");
    assert_eq!(width.len(), 1);
    assert_eq!(
        (width[0].measured_nm, width[0].limit_nm),
        (Some(100_000), Some(150_000))
    );
    assert_eq!(width[0].location, format!("track {}", id(20)));
    assert_eq!(width[0].at_nm, Some([2_000_000, 2_000_000]));

    let diameter = by_rule(&report, "via_diameter");
    assert_eq!(diameter.len(), 1);
    assert_eq!(diameter[0].measured_nm, Some(400_000));

    let drill = by_rule(&report, "drill_size");
    assert_eq!(drill.len(), 1);
    assert_eq!(drill[0].measured_nm, Some(200_000));

    let ring = by_rule(&report, "annular_ring");
    assert_eq!(ring.len(), 2);
    assert!(ring.iter().all(|f| f.measured_nm == Some(100_000)));
    assert!(ring.iter().any(|f| f.location == "U1 pad 1"));

    let clearance = by_rule(&report, "copper_clearance");
    assert_eq!(clearance.len(), 2);
    assert!(clearance.iter().any(|f| f.measured_nm == Some(100_000)));
    assert!(clearance.iter().any(|f| f.measured_nm == Some(0)));
    assert!(clearance.iter().all(|f| f.limit_nm == Some(200_000)));

    let edge = by_rule(&report, "edge_clearance");
    assert_eq!(edge.len(), 1);
    assert_eq!(
        (edge[0].measured_nm, edge[0].limit_nm),
        (Some(100_000), Some(300_000))
    );

    let spacing = by_rule(&report, "drill_spacing");
    assert_eq!(spacing.len(), 1);
    assert_eq!(spacing[0].measured_nm, Some(250_000));

    assert_eq!(report.confirmed.len(), 9);
    assert!(report.inconclusive.is_empty());
    assert!(report
        .confirmed
        .iter()
        .all(|f| f.severity == Severity::Error));
    assert!(!report.passes());
}

#[test]
fn the_findings_come_in_the_fixed_order_of_the_rules() {
    let report = check_board(&faulty_board(), &rules());
    let ranks: Vec<u8> = report.confirmed.iter().map(|f| f.rank).collect();
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    assert_eq!(ranks, sorted);
    assert_eq!(report.confirmed[0].rule, "track_width");
    assert_eq!(report.confirmed[8].rule, "drill_spacing");
}

#[test]
fn a_limit_exactly_met_is_not_a_failure_and_a_value_of_zero_turns_the_rule_off() {
    let b = faulty_board();
    // the narrowest track is 0.10 mm: exactly met
    let met = RuleValues {
        min_track_width: nm(100_000),
        ..RuleValues::default()
    };
    assert!(check_board(&b, &met).confirmed.is_empty());
    let off = RuleValues::default();
    let report = check_board(&b, &off);
    assert!(report.confirmed.is_empty() && report.inconclusive.is_empty());
    assert!(report.passes());
}

#[test]
fn a_clean_board_passes_every_rule() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2_000_000, 2_000_000), (8_000_000, 2_000_000))],
    );
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((2_000_000, 3_000_000), (8_000_000, 3_000_000))],
    );
    via(&mut b, 30, na, (12_000_000, 5_000_000), 300_000, 700_000);
    via(&mut b, 31, nb, (15_000_000, 5_000_000), 300_000, 700_000);
    let report = check_board(&b, &rules());
    assert!(report.confirmed.is_empty(), "{:?}", report.confirmed);
    assert!(report.inconclusive.is_empty());
}

#[test]
fn an_arc_is_inconclusive_and_never_a_failure_but_a_proven_end_point_is_a_failure() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    let arc = Segment::Arc {
        start: pt(2_000_000, 2_000_000),
        end: pt(4_000_000, 2_000_000),
        sweep: Udeg::new(90_000_000).unwrap(),
    };
    track(&mut b, 20, na, 200_000, vec![arc]);
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((2_000_000, 6_000_000), (8_000_000, 6_000_000))],
    );
    let only_clearance = RuleValues {
        min_clearance: nm(200_000),
        ..RuleValues::default()
    };
    let report = check_board(&b, &only_clearance);
    assert!(report.confirmed.is_empty());
    assert_eq!(report.inconclusive.len(), 1);
    assert_eq!(report.inconclusive[0].rule, "not_judged");
    // a straight track that touches an end point of the arc is a proven failure
    track(
        &mut b,
        22,
        nb,
        200_000,
        vec![line((4_000_000, 2_100_000), (8_000_000, 2_100_000))],
    );
    let report = check_board(&b, &only_clearance);
    assert_eq!(by_rule(&report, "copper_clearance").len(), 1);
}

#[test]
fn a_track_with_no_piece_a_damaged_reference_and_an_empty_board_do_not_panic() {
    let empty = Board::default();
    assert!(check_board(&empty, &rules()).confirmed.is_empty());
    assert!(analyse_board(&empty, nm(0)).confirmed.is_empty());
    let mut b = base();
    let na = net(&mut b, 10, "A");
    track(&mut b, 20, na, 100_000, Vec::new());
    // a layer and a net that do not exist
    b.tracks.insert(
        id(21),
        Track {
            net: id(999),
            layer: id(998),
            width: nm(100_000),
            path: vec![line((0, 0), (1_000_000, 0))],
        },
    );
    // an outline that is not a polygon
    b.outline = vec![pt(0, 0)];
    let report = check_board(&b, &rules());
    assert_eq!(by_rule(&report, "track_width").len(), 2);
    assert_eq!(report.inconclusive.len(), 1);
    let _ = analyse_board(&b, nm(0));
}

#[test]
fn pad_rules_come_from_the_board_crate_with_their_limit_and_no_measure() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    // two pads of 0.5 mm with their openings 0.1 mm apart
    component(
        &mut b,
        (0, 0),
        vec![
            (
                "1",
                (10_000_000, 10_000_000),
                round(500_000),
                None,
                vec![TOP, MASK],
                Some(na),
            ),
            (
                "2",
                (10_600_000, 10_000_000),
                round(500_000),
                None,
                vec![TOP, MASK],
                Some(nb),
            ),
        ],
    );
    let only_mask = RuleValues {
        min_mask_bridge: nm(150_000),
        ..RuleValues::default()
    };
    let report = check_board(&b, &only_mask);
    let bridge = by_rule(&report, "mask_bridge");
    assert_eq!(bridge.len(), 1);
    assert_eq!(bridge[0].limit_nm, Some(150_000));
    assert_eq!(bridge[0].measured_nm, None);
    assert!(bridge[0].location.contains("U1 pad 1") && bridge[0].location.contains("U1 pad 2"));
    // the same pads are also too close for a clearance of 0.2 mm
    let clearance = RuleValues {
        min_clearance: nm(200_000),
        ..RuleValues::default()
    };
    assert_eq!(
        by_rule(&check_board(&b, &clearance), "copper_clearance").len(),
        1
    );
    // far enough apart: nothing
    b.footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(42))
        .unwrap()
        .at = pt(11_000_000, 10_000_000);
    assert!(check_board(&b, &only_mask).confirmed.is_empty());
}

#[test]
fn silkscreen_over_a_pad_and_zone_rules_are_reported_by_their_own_rule() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    component(
        &mut b,
        (0, 0),
        vec![(
            "1",
            (10_000_000, 10_000_000),
            round(1_000_000),
            None,
            vec![TOP],
            Some(na),
        )],
    );
    b.silkscreen.insert(
        id(60),
        SilkItem::Stroke {
            layer: id(SILK),
            width: nm(150_000),
            path: vec![line((8_000_000, 10_000_000), (12_000_000, 10_000_000))],
        },
    );
    let square = |x: i64, y: i64, size: i64| {
        vec![
            pt(x, y),
            pt(x + size, y),
            pt(x + size, y + size),
            pt(x, y + size),
        ]
    };
    b.zones.insert(
        id(61),
        Zone {
            net: na,
            layer: id(TOP),
            outline: square(1_000_000, 1_000_000, 4_000_000),
            clearance: nm(100_000),
            cutouts: Vec::new(),
        },
    );
    b.zones.insert(
        id(62),
        Zone {
            net: nb,
            layer: id(TOP),
            outline: square(3_000_000, 3_000_000, 4_000_000),
            clearance: nm(300_000),
            cutouts: Vec::new(),
        },
    );
    let wanted = RuleValues {
        min_clearance: nm(200_000),
        silkscreen_over_pads: true,
        ..RuleValues::default()
    };
    let report = check_board(&b, &wanted);
    assert_eq!(by_rule(&report, "silk_over_pad").len(), 1);
    assert_eq!(by_rule(&report, "zones_overlap").len(), 1);
    let small = by_rule(&report, "zone_clearance");
    assert_eq!(small.len(), 1);
    assert_eq!(small[0].limit_nm, Some(200_000));
    assert_eq!(report.confirmed.len(), 3);
}

#[test]
fn connectivity_finds_the_short_and_the_unrouted_nets_of_the_faulty_board() {
    let report = analyse_board(&faulty_board(), nm(0));
    let shorts: Vec<_> = report
        .confirmed
        .iter()
        .filter(|f| f.rule == "copper_short")
        .collect();
    assert_eq!(shorts.len(), 1);
    assert!(shorts[0].location.contains("via"));
    let opens: Vec<_> = report
        .confirmed
        .iter()
        .filter(|f| f.rule == "net_open")
        .collect();
    assert_eq!(opens.len(), 2);
    assert!(opens
        .iter()
        .any(|f| f.detail.starts_with("4 separate groups")));
    assert!(opens
        .iter()
        .any(|f| f.detail.starts_with("3 separate groups")));
    assert!(!report.passes());
}

#[test]
fn the_joining_tolerance_decides_what_touches() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2_000_000, 2_000_000), (8_000_000, 2_000_000))],
    );
    // 0.10 mm of air between the two copper edges
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((2_000_000, 2_300_000), (8_000_000, 2_300_000))],
    );
    let shorts = |tol: i64| {
        analyse_board(&b, nm(tol))
            .confirmed
            .iter()
            .filter(|f| f.rule == "copper_short")
            .count()
    };
    assert_eq!(shorts(0), 0);
    assert_eq!(shorts(99_999), 0);
    assert_eq!(shorts(100_000), 1);
    assert_eq!(shorts(500_000), 1);
}

#[test]
fn a_pad_without_a_net_that_joins_two_nets_is_a_short_between_them() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    component(
        &mut b,
        (0, 0),
        vec![(
            "1",
            (5_000_000, 5_000_000),
            round(1_000_000),
            None,
            vec![TOP],
            None,
        )],
    );
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((3_000_000, 5_000_000), (4_600_000, 5_000_000))],
    );
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((5_400_000, 5_000_000), (7_000_000, 5_000_000))],
    );
    let report = analyse_board(&b, nm(0));
    let shorts: Vec<_> = report
        .confirmed
        .iter()
        .filter(|f| f.rule == "copper_short")
        .collect();
    assert_eq!(shorts.len(), 1);
    assert_eq!(shorts[0].location, "nets A and B");
    assert!(shorts[0].detail.starts_with("3 items"));
}

#[test]
fn a_zone_joins_the_copper_of_its_own_net_and_not_that_of_other_nets() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    let nb = net(&mut b, 11, "B");
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2_000_000, 2_000_000), (4_000_000, 2_000_000))],
    );
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![line((6_000_000, 2_000_000), (8_000_000, 2_000_000))],
    );
    track(
        &mut b,
        22,
        nb,
        200_000,
        vec![line((2_000_000, 2_500_000), (4_000_000, 2_500_000))],
    );
    let opens = |b: &Board| {
        analyse_board(b, nm(0))
            .confirmed
            .iter()
            .filter(|f| f.rule == "net_open")
            .map(|f| f.location.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(opens(&b), vec!["net A".to_owned()]);
    let outline = vec![
        pt(1_000_000, 1_000_000),
        pt(9_000_000, 1_000_000),
        pt(9_000_000, 3_000_000),
        pt(1_000_000, 3_000_000),
    ];
    b.zones.insert(
        id(60),
        Zone {
            net: nb,
            layer: id(TOP),
            outline: outline.clone(),
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    // a pour of net B does not join the tracks of net A
    assert_eq!(opens(&b), vec!["net A".to_owned()]);
    b.zones.insert(
        id(61),
        Zone {
            net: na,
            layer: id(TOP),
            outline,
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    assert!(opens(&b).is_empty());
}

#[test]
fn an_arc_leaves_the_open_net_inconclusive() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2_000_000, 2_000_000), (4_000_000, 2_000_000))],
    );
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![Segment::Arc {
            start: pt(8_000_000, 2_000_000),
            end: pt(9_000_000, 2_000_000),
            sweep: Udeg::new(90_000_000).unwrap(),
        }],
    );
    let report = analyse_board(&b, nm(0));
    assert!(report.confirmed.is_empty(), "{:?}", report.confirmed);
    assert_eq!(report.inconclusive.len(), 2);
    assert!(report.inconclusive.iter().any(|f| f.rule == "net_open"));
}

// ---- the schematic against the board ----

fn schematic(parts: &[&str], nets: &[(&str, &[&str])]) -> Schematic {
    let mut s = Schematic::default();
    let mut pins = BTreeMap::new();
    pins.insert(
        id(2),
        Pin {
            number: "1".into(),
            at: None,
        },
    );
    pins.insert(
        id(3),
        Pin {
            number: "2".into(),
            at: None,
        },
    );
    s.symbols.insert(
        id(1),
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    for (i, refdes) in parts.iter().enumerate() {
        s.parts.insert(
            id(100 + i as u32),
            Part {
                refdes: (*refdes).into(),
                value: "1k".into(),
                symbol: id(1),
                footprint: None,
                placement: None,
            },
        );
    }
    for (k, (name, labels)) in nets.iter().enumerate() {
        let mut list = Vec::new();
        for label in labels.iter() {
            let (refdes, number) = label.split_once('.').unwrap();
            let part = parts.iter().position(|p| *p == refdes).unwrap();
            list.push(NetPin {
                part: id(100 + part as u32),
                pin: if number == "1" { id(2) } else { id(3) },
            });
        }
        list.sort();
        s.nets.insert(
            id(200 + k as u32),
            SchematicNet {
                name: (*name).into(),
                pins: list,
            },
        );
    }
    s
}

fn board_for(parts: &[&str], assign: &[(&str, &str)]) -> Board {
    let mut b = Board::default();
    b.layers.insert(
        id(TOP),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    let mut pads = BTreeMap::new();
    for (pid, number, x) in [(id(41), "1", -1_000_000), (id(42), "2", 1_000_000)] {
        pads.insert(
            pid,
            Pad {
                number: number.into(),
                shape: round(1_000_000),
                at: pt(x, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![id(TOP)],
                roundness: 0,
                plated: true,
            },
        );
    }
    b.footprints.insert(
        id(40),
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    for (i, refdes) in parts.iter().enumerate() {
        b.components.insert(
            id(50 + i as u32),
            Component {
                refdes: (*refdes).into(),
                value: "1k".into(),
                symbol: None,
                footprint: id(40),
                at: pt(10_000_000 * i as i64, 0),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
    }
    let mut ids: BTreeMap<String, Id> = BTreeMap::new();
    for (_, name) in assign {
        let next = id(300 + ids.len() as u32);
        ids.entry((*name).to_owned()).or_insert(next);
    }
    for (name, nid) in &ids {
        b.nets.insert(*nid, Net { name: name.clone() });
    }
    for (label, name) in assign {
        let (refdes, number) = label.split_once('.').unwrap();
        let part = parts.iter().position(|p| *p == refdes).unwrap();
        b.pad_nets.push(PadNet {
            component: id(50 + part as u32),
            pad: if number == "1" { id(41) } else { id(42) },
            net: ids[*name],
        });
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    b
}

const PAIR_NETS: &[(&str, &[&str])] = &[("VCC", &["R1.1", "R2.1"]), ("GND", &["R1.2", "R2.2"])];

fn rules_of(report: &Report) -> Vec<&'static str> {
    report.confirmed.iter().map(|f| f.rule).collect()
}

#[test]
fn a_board_that_agrees_with_its_schematic_has_nothing_to_report() {
    let s = schematic(&["R1", "R2"], PAIR_NETS);
    let b = board_for(
        &["R1", "R2"],
        &[
            ("R1.1", "VCC"),
            ("R2.1", "VCC"),
            ("R1.2", "GND"),
            ("R2.2", "GND"),
        ],
    );
    let report = compare_schematic(&s, &b);
    assert!(report.confirmed.is_empty(), "{:?}", report.confirmed);
    assert!(report.inconclusive.is_empty());
}

#[test]
fn a_renamed_net_is_found_by_its_pins_and_is_only_a_warning() {
    let s = schematic(&["R1", "R2"], PAIR_NETS);
    let b = board_for(
        &["R1", "R2"],
        &[
            ("R1.1", "+5V"),
            ("R2.1", "+5V"),
            ("R1.2", "GND"),
            ("R2.2", "GND"),
        ],
    );
    let report = compare_schematic(&s, &b);
    assert_eq!(rules_of(&report), vec!["name_differs"]);
    assert_eq!(report.confirmed[0].severity, Severity::Warning);
    assert!(report.passes());
}

#[test]
fn a_missing_net_and_an_unconnected_pin_are_confirmed_errors() {
    let nets: &[(&str, &[&str])] = &[
        ("VCC", &["R1.1", "R2.1"]),
        ("GND", &["R1.2", "R2.2"]),
        ("SIG", &["R3.1", "R3.2"]),
    ];
    let s = schematic(&["R1", "R2", "R3"], nets);
    let b = board_for(
        &["R1", "R2", "R3"],
        &[("R1.1", "VCC"), ("R2.1", "VCC"), ("R1.2", "GND")],
    );
    let report = compare_schematic(&s, &b);
    let mut found = rules_of(&report);
    found.sort_unstable();
    assert_eq!(
        found,
        vec!["name_same_pins_differ", "net_missing", "pin_unconnected"]
    );
    let pin = report
        .confirmed
        .iter()
        .find(|f| f.rule == "pin_unconnected")
        .unwrap();
    assert_eq!(pin.location, "pin R2.2 of net GND");
    assert!(!report.passes());
}

#[test]
fn a_board_net_that_joins_two_schematic_nets_is_a_short_in_the_netlist() {
    let s = schematic(&["R1", "R2"], PAIR_NETS);
    let b = board_for(
        &["R1", "R2"],
        &[
            ("R1.1", "VCC"),
            ("R2.1", "VCC"),
            ("R1.2", "VCC"),
            ("R2.2", "GND"),
        ],
    );
    let report = compare_schematic(&s, &b);
    let joined = report
        .confirmed
        .iter()
        .find(|f| f.rule == "nets_joined")
        .expect("the join is found");
    assert!(joined.detail.contains("GND") && joined.detail.contains("VCC"));
    assert!(!report.passes());
}

#[test]
fn a_pin_with_no_pad_and_a_pad_with_no_pin_are_found() {
    let s = schematic(
        &["R1", "R2", "R9"],
        &[("VCC", &["R1.1", "R9.1"]), ("GND", &["R1.2"])],
    );
    let b = board_for(
        &["R1", "R2"],
        &[("R1.1", "VCC"), ("R1.2", "GND"), ("R2.2", "GND")],
    );
    let report = compare_schematic(&s, &b);
    assert!(report
        .confirmed
        .iter()
        .any(|f| f.rule == "pin_unconnected" && f.location == "pin R9.1 of net VCC"));
    assert!(report
        .confirmed
        .iter()
        .any(|f| f.rule == "pad_only_on_board" && f.location.contains("R2.2")));
}

#[test]
fn the_combined_check_merges_both_reports_in_order() {
    let s = schematic(&["R1", "R2"], PAIR_NETS);
    let b = board_for(
        &["R1", "R2"],
        &[
            ("R1.1", "+5V"),
            ("R2.1", "+5V"),
            ("R1.2", "GND"),
            ("R2.2", "GND"),
        ],
    );
    let report = check_connectivity(&s, &b, nm(0));
    // the four pads are not routed: two nets are open, and the rename is a warning
    let rules = rules_of(&report);
    assert_eq!(rules.iter().filter(|r| **r == "net_open").count(), 2);
    assert!(rules.contains(&"name_differs"));
    let ranks: Vec<u8> = report.confirmed.iter().map(|f| f.rank).collect();
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    assert_eq!(ranks, sorted);
}

// ---- determinism ----

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
}

/// A board of tracks and vias in a field of 6 mm, dense enough to have contacts and clearances.
fn random_board(seed: u64) -> Board {
    let mut g = Lcg(seed);
    let mut b = base();
    let nets: Vec<Id> = (0..3)
        .map(|i| net(&mut b, 10 + i, &format!("N{i}")))
        .collect();
    let mut counter = 100;
    for _ in 0..14 {
        let (x, y) = (g.range(0, 120) * 50_000, g.range(0, 120) * 50_000);
        let (dx, dy) = (g.range(-30, 30) * 50_000, g.range(-30, 30) * 50_000);
        let net = nets[(g.next() % 3) as usize];
        let width = g.range(2, 8) * 50_000;
        let end = ((x + dx).clamp(0, 6_000_000), (y + dy).clamp(0, 6_000_000));
        track(&mut b, counter, net, width, vec![line((x, y), end)]);
        counter += 1;
    }
    for _ in 0..8 {
        let (x, y) = (g.range(0, 120) * 50_000, g.range(0, 120) * 50_000);
        let net = nets[(g.next() % 3) as usize];
        let drill = g.range(2, 6) * 50_000;
        via(
            &mut b,
            counter,
            net,
            (x, y),
            drill,
            drill + g.range(2, 6) * 50_000,
        );
        counter += 1;
    }
    b
}

fn full_rules() -> RuleValues {
    RuleValues {
        min_track_width: nm(150_000),
        min_clearance: nm(200_000),
        min_drill: nm(200_000),
        min_via_diameter: nm(500_000),
        min_annular_ring: nm(100_000),
        min_edge_clearance: nm(300_000),
        min_drill_spacing: nm(250_000),
        connect_tolerance: nm(10_000),
        ..RuleValues::default()
    }
}

#[test]
fn the_same_board_and_rules_always_give_the_same_report() {
    for seed in 0..150 {
        let b = random_board(seed);
        let r = full_rules();
        let first = check_board(&b, &r);
        let second = check_board(&b.clone(), &r);
        assert_eq!(first, second, "seed {seed}");
        assert_eq!(first.to_json(), second.to_json());
        assert_eq!(first.to_text(Lang::En), second.to_text(Lang::En));
        // a board that is saved and read back gives the same report
        let reread = Board::from_bytes(&b.to_bytes().unwrap()).unwrap();
        assert_eq!(check_board(&reread, &r), first, "seed {seed}");
        let conn = analyse_board(&b, r.connect_tolerance);
        assert_eq!(
            conn,
            analyse_board(&reread, r.connect_tolerance),
            "seed {seed}"
        );
    }
}

#[test]
fn a_larger_clearance_never_finds_fewer_clearance_failures() {
    for seed in 0..100 {
        let b = random_board(seed);
        let mut last = 0;
        for limit in [0, 50_000, 100_000, 200_000, 400_000, 800_000] {
            let rules = RuleValues {
                min_clearance: nm(limit),
                ..RuleValues::default()
            };
            let found = by_rule(&check_board(&b, &rules), "copper_clearance").len();
            assert!(
                found >= last,
                "seed {seed}, limit {limit}: {found} < {last}"
            );
            last = found;
        }
    }
}

#[test]
fn a_larger_tolerance_never_finds_fewer_contacts() {
    for seed in 0..100 {
        let b = random_board(seed);
        let mut last = 0;
        for tol in [0, 25_000, 100_000, 400_000] {
            let report = analyse_board(&b, nm(tol));
            let shorts = report
                .confirmed
                .iter()
                .filter(|f| f.rule == "copper_short")
                .count();
            // every item has a net, so every short is a direct contact of two items
            assert!(shorts >= last, "seed {seed}, tolerance {tol}");
            last = shorts;
        }
    }
}

#[test]
fn the_json_report_has_the_two_lists_and_the_text_is_translated() {
    let report = check_board(&faulty_board(), &rules());
    let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["schema"], "phoenix-check/1");
    assert_eq!(json["passes"], false);
    assert_eq!(json["confirmed"].as_array().map(Vec::len), Some(9));
    assert_eq!(json["inconclusive"].as_array().map(Vec::len), Some(0));
    let first = &json["confirmed"][0];
    for key in [
        "code",
        "rule",
        "severity",
        "location",
        "at_nm",
        "measured_nm",
        "limit_nm",
    ] {
        assert!(first.get(key).is_some(), "{key}");
    }
    assert!(report
        .to_text(Lang::Pt)
        .contains("Pista mais estreita do que o mínimo"));
    assert!(report.to_text(Lang::Ja).contains("配線が最小幅より細い"));
}
