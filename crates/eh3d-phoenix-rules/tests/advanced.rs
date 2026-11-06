// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced checks on small boards made by hand. The expected values were calculated by hand
//! from the coordinates (all in nanometres), not copied from the output.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::{
    advanced::codes as dcodes, advanced_findings, check_advanced, check_advanced_with, title,
    to_sarif, AdvancedConfig, Finding, IndexMode, Lang, Report, Severity,
};
use serde_json::Value;

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
const SILK: u32 = 4;
const MM: i64 = 1_000_000;

/// An empty board of 20 mm by 20 mm with two copper layers and a top silkscreen.
fn base() -> Board {
    let mut b = Board::default();
    for (n, name, kind, side) in [
        (TOP, "Top", LayerKind::Copper, None),
        (BOT, "Bottom", LayerKind::Copper, None),
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
        pt(20 * MM, 0),
        pt(20 * MM, 20 * MM),
        pt(0, 20 * MM),
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

fn silk(b: &mut Board, n: u32, width: i64, a: (i64, i64), c: (i64, i64)) {
    b.silkscreen.insert(
        id(n),
        SilkItem::Stroke {
            layer: id(SILK),
            width: nm(width),
            path: vec![line(a, c)],
        },
    );
}

/// A pad: its number, place in the footprint, shape, drill, layers and net.
type PadSpec = (
    &'static str,
    (i64, i64),
    PadShape,
    Option<i64>,
    Vec<u32>,
    Option<Id>,
);

/// One component U1 with the given pads.
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

/// The settings with every check off, then what the test needs.
fn only(f: impl FnOnce(&mut AdvancedConfig)) -> AdvancedConfig {
    let mut cfg = AdvancedConfig::all_off();
    f(&mut cfg);
    cfg
}

fn by_rule<'a>(report: &'a Report, rule: &str) -> Vec<&'a Finding> {
    report.confirmed.iter().filter(|f| f.rule == rule).collect()
}

fn joined(a: &str, b: &str) -> String {
    let mut v = [a.to_owned(), b.to_owned()];
    v.sort();
    format!("{} and {}", v[0], v[1])
}

#[test]
fn the_annular_ring_of_vias_and_drilled_pads_is_measured_against_the_drill() {
    let mut b = base();
    let (na, nb) = (net(&mut b, 10, "A"), net(&mut b, 11, "B"));
    // ring (500000 - 300000) / 2 = 100000, below 150000
    via(&mut b, 30, na, (12 * MM, 5 * MM), 300_000, 500_000);
    // ring 200000
    via(&mut b, 31, na, (15 * MM, 5 * MM), 300_000, 700_000);
    component(
        &mut b,
        (5 * MM, 10 * MM),
        vec![
            // ring (800000 - 600000) / 2 = 100000
            (
                "1",
                (0, 0),
                round(800_000),
                Some(600_000),
                vec![TOP, BOT],
                Some(na),
            ),
            // ring (1000000 - 400000) / 2 = 300000
            (
                "2",
                (2 * MM, 0),
                round(1_000_000),
                Some(400_000),
                vec![TOP, BOT],
                Some(nb),
            ),
        ],
    );
    let cfg = only(|c| {
        c.annular_ring_level = Some(Severity::Error);
        c.annular_ring_min_nm = 150_000;
    });
    let report = check_advanced(&b, &cfg);
    assert_eq!(report.confirmed.len(), 2);
    assert!(report.confirmed.iter().all(|f| f.code == "PHOENIX-DRC-001"));
    assert!(report
        .confirmed
        .iter()
        .all(|f| f.measured_nm == Some(100_000) && f.limit_nm == Some(150_000)));
    assert!(report
        .confirmed
        .iter()
        .all(|f| f.severity == Severity::Error));
    let places: Vec<&str> = report
        .confirmed
        .iter()
        .map(|f| f.location.as_str())
        .collect();
    assert!(places.contains(&"U1 pad 1"));
    assert!(places.contains(&format!("via {}", id(30)).as_str()));
    assert!(!report.passes());

    // a warning does not make the report fail, and off finds nothing
    let warn = only(|c| {
        c.annular_ring_level = Some(Severity::Warning);
        c.annular_ring_min_nm = 150_000;
    });
    let report = check_advanced(&b, &warn);
    assert_eq!(report.confirmed.len(), 2);
    assert!(report.passes());
    assert!(check_advanced(&b, &AdvancedConfig::all_off())
        .confirmed
        .is_empty());
}

#[test]
fn copper_and_pads_are_measured_against_the_board_edge() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    // 0.2 mm from the top edge, so the gap between the copper and the edge is 200000 - 100000
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2 * MM, 19_800_000), (8 * MM, 19_800_000))],
    );
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![line((2 * MM, 10 * MM), (8 * MM, 10 * MM))],
    );
    via(&mut b, 30, na, (10 * MM, 12 * MM), 300_000, 600_000);
    component(
        &mut b,
        (600_000, 10 * MM),
        vec![
            // the centre is 600000 from the left edge: the gap is 600000 - 500000 = 100000
            ("1", (0, 0), round(1_000_000), None, vec![TOP], Some(na)),
            (
                "2",
                (5 * MM, 0),
                round(1_000_000),
                None,
                vec![TOP],
                Some(na),
            ),
        ],
    );
    let cfg = only(|c| {
        c.copper_edge_level = Some(Severity::Error);
        c.copper_edge_min_nm = 300_000;
        c.pad_edge_level = Some(Severity::Warning);
        c.pad_edge_min_nm = 250_000;
    });
    let report = check_advanced(&b, &cfg);
    let copper = by_rule(&report, "copper_edge");
    assert_eq!(copper.len(), 1);
    assert_eq!(copper[0].location, format!("track {}", id(20)));
    assert_eq!(copper[0].measured_nm, Some(100_000));
    assert_eq!(copper[0].limit_nm, Some(300_000));
    assert_eq!(copper[0].at_nm, Some([2 * MM, 19_800_000]));
    assert_eq!(copper[0].severity, Severity::Error);
    let pads = by_rule(&report, "pad_edge");
    assert_eq!(pads.len(), 1);
    assert_eq!(pads[0].location, "U1 pad 1");
    assert_eq!(pads[0].measured_nm, Some(100_000));
    assert_eq!(pads[0].limit_nm, Some(250_000));
    assert_eq!(pads[0].severity, Severity::Warning);
    assert_eq!(report.confirmed.len(), 2);
}

#[test]
fn a_board_without_an_outline_leaves_the_edge_checks_inconclusive() {
    let mut b = base();
    b.outline.clear();
    let na = net(&mut b, 10, "A");
    track(&mut b, 20, na, 200_000, vec![line((MM, MM), (5 * MM, MM))]);
    let cfg = only(|c| {
        c.copper_edge_level = Some(Severity::Error);
        c.copper_edge_min_nm = 300_000;
    });
    let report = check_advanced(&b, &cfg);
    assert!(report.confirmed.is_empty());
    assert_eq!(report.inconclusive.len(), 1);
    assert_eq!(report.inconclusive[0].code, "PHOENIX-DRC-014");
    assert_eq!(report.inconclusive[0].location, "board outline");
    assert_eq!(report.inconclusive[0].severity, Severity::Info);
    assert!(report.passes());
}

#[test]
fn the_smallest_drill_follows_the_class_of_the_net() {
    let mut b = base();
    let pwr = net(&mut b, 10, "PWR");
    let sig = net(&mut b, 11, "SIG");
    via(&mut b, 30, pwr, (5 * MM, 5 * MM), 300_000, 700_000);
    via(&mut b, 31, sig, (8 * MM, 5 * MM), 150_000, 600_000);
    via(&mut b, 32, sig, (11 * MM, 5 * MM), 250_000, 600_000);
    let cfg = AdvancedConfig::parse(
        "drill_class_level = warning\ndrill_default_min_nm = 200000\n\
         drill_class.power.nets = PWR, VBUS\ndrill_class.power.min_nm = 400000\n",
    )
    .unwrap();
    let report = check_advanced(&b, &cfg);
    let found = by_rule(&report, "drill_class");
    assert_eq!(found.len(), 2);
    let power = found
        .iter()
        .find(|f| f.location == format!("via {}", id(30)))
        .unwrap();
    assert_eq!(
        (power.measured_nm, power.limit_nm),
        (Some(300_000), Some(400_000))
    );
    assert_eq!(power.detail, "minimum of the class power");
    let default = found
        .iter()
        .find(|f| f.location == format!("via {}", id(31)))
        .unwrap();
    assert_eq!(
        (default.measured_nm, default.limit_nm),
        (Some(150_000), Some(200_000))
    );
    assert_eq!(default.detail, "default minimum");
    assert_eq!(default.severity, Severity::Warning);
}

#[test]
fn silkscreen_is_measured_against_the_drill_of_a_through_hole() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    component(
        &mut b,
        (10 * MM, 10 * MM),
        vec![(
            "1",
            (0, 0),
            round(1_000_000),
            Some(800_000),
            vec![TOP, BOT],
            Some(na),
        )],
    );
    // the centre line is 600000 from the hole: 600000 - 75000 - 400000 = 125000 of gap
    silk(
        &mut b,
        60,
        150_000,
        (8 * MM, 10_600_000),
        (12 * MM, 10_600_000),
    );
    // 1000000 from the hole: 1000000 - 75000 - 400000 = 525000
    silk(&mut b, 61, 150_000, (8 * MM, 11 * MM), (12 * MM, 11 * MM));
    let cfg = only(|c| {
        c.silk_hole_level = Some(Severity::Warning);
        c.silk_hole_min_nm = 150_000;
    });
    let report = check_advanced(&b, &cfg);
    assert_eq!(report.confirmed.len(), 1);
    let f = &report.confirmed[0];
    assert_eq!(f.code, "PHOENIX-DRC-005");
    assert_eq!(f.location, joined("U1 pad 1", &format!("silk {}", id(60))));
    assert_eq!((f.measured_nm, f.limit_nm), (Some(125_000), Some(150_000)));
    assert_eq!(f.at_nm, Some([10 * MM, 10 * MM]));
}

#[test]
fn a_narrow_track_that_joins_wider_copper_is_a_neck() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    component(
        &mut b,
        (5 * MM, 5 * MM),
        vec![("1", (0, 0), round(1_200_000), None, vec![TOP], Some(na))],
    );
    track(
        &mut b,
        20,
        na,
        100_000,
        vec![line((5 * MM, 5 * MM), (9 * MM, 5 * MM))],
    );
    // narrow too, but it joins nothing wider
    track(
        &mut b,
        21,
        na,
        100_000,
        vec![line((12 * MM, 12 * MM), (15 * MM, 12 * MM))],
    );
    let cfg = only(|c| {
        c.neck_level = Some(Severity::Warning);
        c.neck_min_width_nm = 200_000;
    });
    let report = check_advanced(&b, &cfg);
    assert_eq!(report.confirmed.len(), 1);
    let f = &report.confirmed[0];
    assert_eq!(f.location, format!("track {}", id(20)));
    assert_eq!((f.measured_nm, f.limit_nm), (Some(100_000), Some(200_000)));
    assert_eq!(f.at_nm, Some([5 * MM, 5 * MM]));
    assert_eq!(f.detail, "joins U1 pad 1 (1200000 nm wide)");
}

#[test]
fn pieces_of_one_net_that_meet_at_an_acute_angle_are_an_acid_trap() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((5 * MM, 5 * MM), (10 * MM, 5 * MM))],
    );
    // leaves the same point at 45 degrees from the first
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![line((5 * MM, 5 * MM), (7 * MM, 3 * MM))],
    );
    // leaves it the other way: 180 degrees from the first and 135 from the second
    track(
        &mut b,
        22,
        na,
        200_000,
        vec![line((5 * MM, 5 * MM), (0, 5 * MM))],
    );
    // a right-angle corner inside one track
    track(
        &mut b,
        23,
        na,
        200_000,
        vec![
            line((12 * MM, 10 * MM), (15 * MM, 10 * MM)),
            line((15 * MM, 10 * MM), (15 * MM, 13 * MM)),
        ],
    );
    let cfg = only(|c| {
        c.acid_trap_level = Some(Severity::Warning);
        c.acid_trap_min_angle_deg = 90;
    });
    let report = check_advanced(&b, &cfg);
    assert_eq!(report.confirmed.len(), 1);
    let f = &report.confirmed[0];
    assert_eq!(
        f.location,
        joined(&format!("track {}", id(20)), &format!("track {}", id(21)))
    );
    assert_eq!(f.at_nm, Some([5 * MM, 5 * MM]));
    assert_eq!(f.detail, "angle 45.000 degrees, minimum 90 degrees");
    // with 100 degrees the right angle is acute too
    let stricter = only(|c| {
        c.acid_trap_level = Some(Severity::Warning);
        c.acid_trap_min_angle_deg = 100;
    });
    assert_eq!(check_advanced(&b, &stricter).confirmed.len(), 2);
}

#[test]
fn the_spacing_follows_the_voltage_between_the_nets() {
    let mut b = base();
    let (na, nb, nc) = (
        net(&mut b, 10, "A"),
        net(&mut b, 11, "B"),
        net(&mut b, 12, "C"),
    );
    // the centre lines are 500000 apart: the gap is 500000 - 200000 = 300000
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((2 * MM, 5 * MM), (8 * MM, 5 * MM))],
    );
    track(
        &mut b,
        21,
        nb,
        200_000,
        vec![line((2 * MM, 5_500_000), (8 * MM, 5_500_000))],
    );
    // C has no voltage in the settings, so it is not judged
    track(
        &mut b,
        22,
        nc,
        200_000,
        vec![line((2 * MM, 5_900_000), (8 * MM, 5_900_000))],
    );
    let on = |a: &str| {
        AdvancedConfig::parse(&format!(
            "isolation_level = error\nvoltage.A = {a}\nvoltage.B = 0\n"
        ))
        .unwrap()
    };
    // 48 V is in the row up to 50 V: column b2 asks 600000
    let report = check_advanced(&b, &on("48"));
    let found = by_rule(&report, "isolation");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].location,
        joined(&format!("track {}", id(20)), &format!("track {}", id(21)))
    );
    assert_eq!(
        (found[0].measured_nm, found[0].limit_nm),
        (Some(300_000), Some(600_000))
    );
    assert!(found[0].detail.contains("48 V"));
    assert!(found[0].detail.contains("column b2"));
    assert!(found[0].detail.contains("not verified"));
    // 12 V is in the row up to 15 V: 100000 is enough
    assert!(by_rule(&check_advanced(&b, &on("12")), "isolation").is_empty());

    // above 500 V the spacing is the figure per volt times the voltage: 5000 * 600 = 3000000
    let mut far = b.clone();
    track(
        &mut far,
        21,
        nb,
        200_000,
        vec![line((2 * MM, 7 * MM), (8 * MM, 7 * MM))],
    );
    let report = check_advanced(&far, &on("600"));
    let found = by_rule(&report, "isolation");
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].measured_nm, found[0].limit_nm),
        (Some(1_800_000), Some(3_000_000))
    );
}

#[test]
fn the_table_of_spacing_reads_as_the_file_says() {
    let cfg = AdvancedConfig::default();
    assert_eq!(cfg.iso_column_name(), "b2");
    assert_eq!(cfg.required_spacing_nm(12), 100_000);
    assert_eq!(cfg.required_spacing_nm(-12), 100_000);
    assert_eq!(cfg.required_spacing_nm(48), 600_000);
    assert_eq!(cfg.required_spacing_nm(120), 600_000);
    assert_eq!(cfg.required_spacing_nm(160), 1_250_000);
    assert_eq!(cfg.required_spacing_nm(300), 1_250_000);
    assert_eq!(cfg.required_spacing_nm(301), 2_500_000);
    assert_eq!(cfg.required_spacing_nm(600), 3_000_000);
    // writing rows replaces the table, and the column can change
    let own = AdvancedConfig::parse(
        "iso_column = a7\niso_row = 10 1 2 3 4 5 6 7\niso_row = 20 8 9 10 11 12 13 14\n",
    )
    .unwrap();
    assert_eq!(own.iso_rows.len(), 2);
    assert_eq!(own.required_spacing_nm(10), 7);
    assert_eq!(own.required_spacing_nm(11), 14);
    assert_eq!(own.required_spacing_nm(21), 3_050 * 21);
}

#[test]
fn copper_that_joins_no_pad_is_an_island_and_a_track_with_a_free_end_is_a_stub() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    component(
        &mut b,
        (3 * MM, 3 * MM),
        vec![("1", (0, 0), round(1_000_000), None, vec![TOP], Some(na))],
    );
    track(
        &mut b,
        20,
        na,
        200_000,
        vec![line((3 * MM, 3 * MM), (6 * MM, 3 * MM))],
    );
    // joined to the first track at one end, free at the other: a stub
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![line((6 * MM, 3 * MM), (9 * MM, 3 * MM))],
    );
    // joins nothing: an island of one track, but not a stub (both ends are free)
    track(
        &mut b,
        22,
        na,
        200_000,
        vec![line((10 * MM, 10 * MM), (12 * MM, 10 * MM))],
    );
    via(&mut b, 30, na, (15 * MM, 15 * MM), 300_000, 600_000);
    let cfg = only(|c| {
        c.island_level = Some(Severity::Warning);
        c.stub_level = Some(Severity::Warning);
    });
    let report = check_advanced(&b, &cfg);
    let islands = by_rule(&report, "island");
    assert_eq!(islands.len(), 2);
    let places: Vec<&str> = islands.iter().map(|f| f.location.as_str()).collect();
    assert!(places.contains(&format!("net A: track {}", id(22)).as_str()));
    assert!(places.contains(&format!("net A: via {}", id(30)).as_str()));
    let stubs = by_rule(&report, "stub");
    assert_eq!(stubs.len(), 1);
    assert_eq!(stubs[0].location, format!("track {}", id(21)));
    assert_eq!(stubs[0].at_nm, Some([9 * MM, 3 * MM]));
    assert_eq!(report.confirmed.len(), 3);
}

#[test]
fn vias_on_the_same_place_are_duplicates_and_the_distance_is_a_setting() {
    let mut b = base();
    let (na, nb) = (net(&mut b, 10, "A"), net(&mut b, 11, "B"));
    via(&mut b, 30, na, (5 * MM, 5 * MM), 300_000, 600_000);
    via(&mut b, 31, nb, (5 * MM, 5 * MM), 300_000, 600_000);
    via(&mut b, 32, na, (5_100_000, 5 * MM), 300_000, 600_000);
    let exact = only(|c| {
        c.via_duplicate_level = Some(Severity::Error);
        c.via_duplicate_max_distance_nm = 0;
    });
    let report = check_advanced(&b, &exact);
    assert_eq!(report.confirmed.len(), 1);
    let f = &report.confirmed[0];
    assert_eq!(
        f.location,
        joined(&format!("via {}", id(30)), &format!("via {}", id(31)))
    );
    assert_eq!(f.measured_nm, Some(0));
    assert_eq!(f.detail, "different nets");
    let near = only(|c| {
        c.via_duplicate_level = Some(Severity::Error);
        c.via_duplicate_max_distance_nm = 150_000;
    });
    let report = check_advanced(&b, &near);
    assert_eq!(report.confirmed.len(), 3);
    assert!(report
        .confirmed
        .iter()
        .any(|f| f.measured_nm == Some(100_000)));
}

#[test]
fn a_pad_on_no_net_and_a_part_outside_the_outline_are_found() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    component(
        &mut b,
        (19_500_000, 10 * MM),
        vec![
            ("1", (0, 0), round(500_000), None, vec![TOP], Some(na)),
            // 20.5 mm: outside the 20 mm board, and on no net
            ("2", (MM, 0), round(500_000), None, vec![TOP], None),
        ],
    );
    let cfg = only(|c| {
        c.pad_no_net_level = Some(Severity::Warning);
        c.outside_outline_level = Some(Severity::Error);
    });
    let report = check_advanced(&b, &cfg);
    let nets = by_rule(&report, "pad_no_net");
    assert_eq!(nets.len(), 1);
    assert_eq!(nets[0].location, "U1 pad 2");
    assert_eq!(nets[0].at_nm, Some([20_500_000, 10 * MM]));
    let outside = by_rule(&report, "outside_outline");
    assert_eq!(outside.len(), 1);
    assert_eq!(outside[0].location, "U1");
    assert_eq!(outside[0].detail, "1 of 2 pads outside");
    assert_eq!(outside[0].at_nm, Some([20_500_000, 10 * MM]));
    assert_eq!(report.confirmed.len(), 2);
}

#[test]
fn the_settings_file_is_strict() {
    let code = |text: &str| AdvancedConfig::parse(text).unwrap_err().code;
    assert_eq!(code("no_such_key = 1"), "PHOENIX-DRC-015");
    assert_eq!(code("annular_ring_level = loud"), "PHOENIX-DRC-015");
    assert_eq!(code("annular_ring_min_nm = -1"), "PHOENIX-DRC-015");
    assert_eq!(code("annular_ring_min_nm = 2000000000"), "PHOENIX-DRC-015");
    assert_eq!(code("grid_cell_nm = 0"), "PHOENIX-DRC-015");
    assert_eq!(code("acid_trap_min_angle_deg = 181"), "PHOENIX-DRC-015");
    assert_eq!(
        code("neck_level = warning\nneck_level = error"),
        "PHOENIX-DRC-015"
    );
    assert_eq!(code("not a pair"), "PHOENIX-DRC-015");
    assert_eq!(code("iso_column = z9"), "PHOENIX-DRC-015");
    assert_eq!(code("iso_row = 1 2 3"), "PHOENIX-DRC-015");
    assert_eq!(
        code("iso_row = 10 1 1 1 1 1 1 1\niso_row = 10 1 1 1 1 1 1 1"),
        "PHOENIX-DRC-015"
    );
    assert_eq!(code("voltage. = 5"), "PHOENIX-DRC-015");
    assert_eq!(code("voltage.A = twelve"), "PHOENIX-DRC-015");
    assert_eq!(
        code("drill_class.x.nets = A\ndrill_class.y.nets = A"),
        "PHOENIX-DRC-015"
    );
    assert_eq!(code("drill_class.x.min_nm = 5"), "PHOENIX-DRC-015");
    assert_eq!(code("drill_class.x.speed = 5"), "PHOENIX-DRC-015");
    assert_eq!(code("#".repeat(2 << 20).as_str()), "PHOENIX-DRC-015");
    // comments and blank lines are fine, and an empty file is the default
    assert_eq!(
        AdvancedConfig::parse("# nothing\n\n").unwrap(),
        AdvancedConfig::default()
    );
    assert_eq!(
        AdvancedConfig::parse("").unwrap(),
        AdvancedConfig::default()
    );
    let own = AdvancedConfig::parse("neck_level = off\nvoltage.GND = 0\n").unwrap();
    assert_eq!(own.neck_level, None);
    assert_eq!(own.voltages.get("GND"), Some(&0));
}

/// A pseudo-random board of tracks, vias, silkscreen and a component, the same for a seed.
fn random_board(seed: u64) -> Board {
    let mut state = seed;
    let mut rand = move |n: i64| -> i64 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) % n as u64) as i64
    };
    let mut b = base();
    let nets = [
        net(&mut b, 10, "A"),
        net(&mut b, 11, "B"),
        net(&mut b, 12, "C"),
    ];
    for k in 0..120u32 {
        let (x0, y0) = (MM + rand(18 * MM), MM + rand(18 * MM));
        let (x1, y1) = (MM + rand(18 * MM), MM + rand(18 * MM));
        let layer = if rand(2) == 0 { TOP } else { BOT };
        let net = nets[rand(3) as usize];
        b.tracks.insert(
            id(100 + k),
            Track {
                net,
                layer: id(layer),
                width: nm(100_000 + rand(200_000)),
                path: vec![line((x0, y0), (x1, y0)), line((x1, y0), (x1, y1))],
            },
        );
    }
    for k in 0..40u32 {
        let drill = 150_000 + rand(150_000);
        via(
            &mut b,
            300 + k,
            nets[rand(3) as usize],
            (MM + rand(18 * MM), MM + rand(18 * MM)),
            drill,
            drill + 100_000 + rand(300_000),
        );
    }
    for k in 0..10u32 {
        silk(
            &mut b,
            400 + k,
            100_000 + rand(100_000),
            (MM + rand(18 * MM), MM + rand(18 * MM)),
            (MM + rand(18 * MM), MM + rand(18 * MM)),
        );
    }
    let mut pads: Vec<PadSpec> = Vec::new();
    for (i, number) in ["1", "2", "3", "4"].into_iter().enumerate() {
        let drill = if i % 2 == 0 { Some(300_000) } else { None };
        let net = if i == 3 {
            None
        } else {
            Some(nets[rand(3) as usize])
        };
        pads.push((
            number,
            (rand(16 * MM) - 8 * MM, rand(16 * MM) - 8 * MM),
            round(800_000 + rand(400_000)),
            drill,
            vec![TOP, BOT],
            net,
        ));
    }
    component(&mut b, (10 * MM, 10 * MM), pads);
    b
}

#[test]
fn the_grid_gives_the_same_report_as_comparing_every_pair() {
    for seed in [1u64, 7, 99] {
        let board = random_board(seed);
        for cell in [1_000i64, 800_000, 3_000_000, 200 * MM] {
            let cfg = AdvancedConfig::parse(&format!(
                "grid_cell_nm = {cell}\nvoltage.A = 48\nvoltage.B = 0\nvoltage.C = 300\n\
                 silk_hole_min_nm = 400000\nneck_min_width_nm = 250000\n\
                 acid_trap_min_angle_deg = 100\n"
            ))
            .unwrap();
            let grid = check_advanced_with(&board, &cfg, IndexMode::Grid);
            let naive = check_advanced_with(&board, &cfg, IndexMode::Naive);
            assert_eq!(grid, naive, "seed {seed}, cell {cell}");
            assert_eq!(grid.to_json(), naive.to_json());
            assert_eq!(to_sarif(&grid, Lang::En), to_sarif(&naive, Lang::En));
            // the board is dense, so the comparison is about something
            assert!(
                grid.confirmed.len() > 20,
                "seed {seed}: {}",
                grid.confirmed.len()
            );
            for rule in [
                "isolation",
                "stub",
                "acid_trap",
                "island",
                "silk_hole",
                "neck",
            ] {
                assert!(
                    !by_rule(&grid, rule).is_empty(),
                    "seed {seed}, cell {cell}: no {rule} finding"
                );
            }
        }
    }
}

#[test]
fn the_report_is_the_same_whatever_the_order_the_checks_found_things() {
    let board = random_board(5);
    let cfg = AdvancedConfig::default();
    let a = check_advanced(&board, &cfg);
    let b = check_advanced(&board.clone(), &cfg);
    assert_eq!(a, b);
    assert_eq!(a.to_text(Lang::En), b.to_text(Lang::En));
    let mut sorted = a.confirmed.clone();
    sorted.sort_by_key(|f| (f.rank, f.location.clone(), f.at_nm, f.code));
    assert_eq!(sorted, a.confirmed);
    assert_eq!(advanced_findings(&board), a.confirmed);
}

#[test]
fn the_text_is_in_eight_languages_and_the_sarif_layout_is_stable() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    via(&mut b, 30, na, (12 * MM, 5 * MM), 300_000, 500_000);
    let cfg = only(|c| {
        c.annular_ring_level = Some(Severity::Error);
        c.annular_ring_min_nm = 150_000;
    });
    let report = check_advanced(&b, &cfg);
    for lang in Lang::ALL {
        let text = report.to_text(lang);
        let wanted = title(dcodes::RING, lang).unwrap();
        assert!(text.contains(wanted), "{}: {text}", lang.tag());
    }
    assert!(report.to_text(Lang::Pt).contains("Anel anular mais fino"));
    let sarif = to_sarif(&report, Lang::Es);
    assert_eq!(sarif, to_sarif(&report, Lang::Es));
    let value: Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(value["schema"], "phoenix-drc/1");
    assert_eq!(value["language"], "es");
    assert_eq!(value["passes"], false);
    assert_eq!(value["rules"][0]["id"], "PHOENIX-DRC-001");
    assert_eq!(value["rules"][0]["name"], "annular_ring_min");
    assert_eq!(value["results"][0]["ruleId"], "PHOENIX-DRC-001");
    assert_eq!(value["results"][0]["level"], "error");
    assert_eq!(value["results"][0]["kind"], "fail");
    assert_eq!(value["results"][0]["properties"]["measured_nm"], 100_000);
    assert_eq!(value["results"][0]["properties"]["limit_nm"], 150_000);
    assert_eq!(value["results"][0]["location"]["at_nm"][0], 12 * MM);
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["schema"], "phoenix-drc-advanced/1");
}

#[test]
fn the_edge_of_a_cutout_is_an_edge_of_the_board() {
    let mut b = base();
    let na = net(&mut b, 10, "A");
    // a track 0.2 mm wide from x = 2 mm to x = 8 mm, so its end reaches x = 8.1 mm
    track(
        &mut b,
        21,
        na,
        200_000,
        vec![line((2 * MM, 10 * MM), (8 * MM, 10 * MM))],
    );
    let cfg = only(|c| {
        c.copper_edge_level = Some(Severity::Error);
        c.copper_edge_min_nm = 500_000;
        c.outside_outline_level = Some(Severity::Error);
    });
    assert!(by_rule(&check_advanced(&b, &cfg), "copper_edge").is_empty());
    // an opening from x = 8.5 mm to 12 mm: its edge is 8.5 - 8.1 = 0.4 mm from the copper
    b.outline_cutouts = vec![vec![
        pt(8_500_000, 8 * MM),
        pt(12 * MM, 8 * MM),
        pt(12 * MM, 12 * MM),
        pt(8_500_000, 12 * MM),
    ]];
    let report = check_advanced(&b, &cfg);
    let copper = by_rule(&report, "copper_edge");
    assert_eq!(copper.len(), 1);
    assert_eq!(copper[0].measured_nm, Some(400_000));
    assert_eq!(copper[0].limit_nm, Some(500_000));
    // a part with a pad in the opening is outside the board
    component(
        &mut b,
        (10 * MM, 10 * MM),
        vec![("1", (0, 0), round(500_000), None, vec![TOP], Some(na))],
    );
    let report = check_advanced(&b, &cfg);
    let outside = by_rule(&report, "outside_outline");
    assert_eq!(outside.len(), 1);
    assert_eq!(outside[0].detail, "1 of 1 pads outside");
}
