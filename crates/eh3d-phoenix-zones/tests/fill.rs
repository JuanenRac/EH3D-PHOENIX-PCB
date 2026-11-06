// SPDX-License-Identifier: GPL-3.0-or-later
//! The fill of zones on small boards made by hand, with areas worked out on paper.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side, Via, Zone,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_zones::codes::{
    CLEARANCE_VIOLATED, FILL_EMPTY, OUTSIDE_OUTLINE, PRIORITY_TIE, RELIEF_IMPOSSIBLE,
    ZONE_FILE_INVALID, ZONE_NOT_FOUND,
};
use eh3d_phoenix_zones::{
    check_fill, fill_board, fill_file_json, fill_json, has_errors, FillReport, ZoneFile, ZoneRules,
    ZoneSettings,
};

const MM: i64 = 1_000_000;

fn id(n: u8) -> Id {
    format!("01890a5d-ac96-7ab2-80e2-4536629c13{n:02x}")
        .parse()
        .unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<Point> {
    vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]
}

const LAYER: u8 = 1;
const NET_A: u8 = 2;
const NET_B: u8 = 3;

/// A board of 20 mm by 20 mm with one copper layer and two nets.
fn base_board() -> Board {
    let mut board = Board {
        name: "test".to_owned(),
        outline: square(0, 0, 20 * MM, 20 * MM),
        ..Board::default()
    };
    board.layers.insert(
        id(LAYER),
        Layer {
            name: "F.Cu".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.nets.insert(
        id(NET_A),
        Net {
            name: "GND".to_owned(),
        },
    );
    board.nets.insert(
        id(NET_B),
        Net {
            name: "SIG".to_owned(),
        },
    );
    board
}

fn add_zone(board: &mut Board, n: u8, net: u8, outline: Vec<Point>) -> Id {
    let zone = id(n);
    board.zones.insert(
        zone,
        Zone {
            net: id(net),
            layer: id(LAYER),
            outline,
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    zone
}

fn add_via(board: &mut Board, n: u8, net: u8, x: i64, y: i64) {
    board.vias.insert(
        id(n),
        Via {
            net: id(net),
            at: pt(x, y),
            drill: nm(300_000),
            diameter: nm(MM),
            from_layer: id(LAYER),
            to_layer: id(LAYER),
        },
    );
}

fn rules() -> ZoneRules {
    ZoneRules {
        min_clearance_nm: 200_000,
        min_edge_clearance_nm: 0,
        class_clearance_nm: BTreeMap::new(),
    }
}

fn settings() -> ZoneSettings {
    ZoneSettings {
        keep_unconnected_islands: true,
        min_island_area_nm2: 0,
        ..ZoneSettings::default()
    }
}

fn run(board: &Board) -> FillReport {
    fill_board(board, &rules(), &settings(), None).unwrap()
}

fn inside(polygon: &[[i64; 2]], x: i64, y: i64) -> bool {
    let n = polygon.len();
    (0..n).all(|i| {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        let cross = i128::from(b[0] - a[0]) * i128::from(y - a[1])
            - i128::from(b[1] - a[1]) * i128::from(x - a[0]);
        cross >= 0
    })
}

fn covered(report: &FillReport, x: i64, y: i64) -> bool {
    report
        .fill
        .zones
        .iter()
        .flat_map(|z| z.polygons.iter())
        .any(|p| inside(p, x, y))
}

fn covered_in(report: &FillReport, zone: Id, x: i64, y: i64) -> bool {
    report
        .fill
        .zones
        .iter()
        .filter(|z| z.zone == zone)
        .flat_map(|z| z.polygons.iter())
        .any(|p| inside(p, x, y))
}

#[test]
fn a_zone_alone_is_filled_with_its_own_outline() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let report = run(&board);
    assert_eq!(report.fill.zones.len(), 1);
    let zone = &report.fill.zones[0];
    assert_eq!(
        zone.polygons,
        vec![vec![
            [MM, MM],
            [19 * MM, MM],
            [19 * MM, 19 * MM],
            [MM, 19 * MM]
        ]]
    );
    assert_eq!(zone.area_nm2, 324_000_000_000_000);
    assert_eq!(report.stats[0].islands_kept, 1);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}

#[test]
fn a_via_of_another_net_leaves_a_hole_that_keeps_the_clearance() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_via(&mut board, 20, NET_B, 10 * MM, 10 * MM);
    let report = run(&board);
    let zone = &report.fill.zones[0];
    // the hole holds a circle of 0.7 mm radius and is a polygon around it
    let hole = 324_000_000_000_000 - zone.area_nm2;
    assert!(
        hole > 1_500_000_000_000 && hole < 1_600_000_000_000,
        "{hole}"
    );
    assert!(!covered(&report, 10 * MM, 10 * MM));
    assert!(!covered(&report, 10 * MM + 650_000, 10 * MM));
    assert!(covered(&report, 10 * MM + 800_000, 10 * MM));
    assert!(covered(&report, 2 * MM, 2 * MM));
    // measured again against the board, the fill breaks no rule
    let findings = check_fill(&board, &rules(), &settings(), &report.fill).unwrap();
    assert!(!has_errors(&findings), "{findings:?}");
}

#[test]
fn the_same_board_gives_the_same_bytes() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_via(&mut board, 20, NET_B, 10 * MM, 10 * MM);
    add_via(&mut board, 21, NET_B, 5 * MM, 12 * MM);
    let first = run(&board);
    let second = run(&board);
    assert_eq!(first, second);
    assert_eq!(fill_json(&first), fill_json(&second));
    assert_eq!(fill_file_json(&first), fill_file_json(&second));
}

#[test]
fn a_fill_that_covers_a_via_is_found_by_the_check() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_via(&mut board, 20, NET_B, 10 * MM, 10 * MM);
    let mut report = run(&board);
    report.fill.zones[0].polygons = vec![vec![
        [MM, MM],
        [19 * MM, MM],
        [19 * MM, 19 * MM],
        [MM, 19 * MM],
    ]];
    let findings = check_fill(&board, &rules(), &settings(), &report.fill).unwrap();
    assert!(
        findings.iter().any(|f| f.code == CLEARANCE_VIOLATED),
        "{findings:?}"
    );
    assert!(has_errors(&findings));
}

#[test]
fn a_fill_outside_the_zone_is_found_by_the_check() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let mut report = run(&board);
    report.fill.zones[0].polygons = vec![vec![
        [MM, MM],
        [25 * MM, MM],
        [25 * MM, 19 * MM],
        [MM, 19 * MM],
    ]];
    let findings = check_fill(&board, &rules(), &settings(), &report.fill).unwrap();
    assert!(
        findings.iter().any(|f| f.code == OUTSIDE_OUTLINE),
        "{findings:?}"
    );
}

#[test]
fn a_fill_for_another_board_is_refused() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let mut report = run(&board);
    report.fill.board = "other".to_owned();
    assert!(check_fill(&board, &rules(), &settings(), &report.fill).is_err());
}

#[test]
fn an_l_shaped_zone_keeps_its_whole_area() {
    let mut board = base_board();
    let l = vec![
        pt(MM, MM),
        pt(3 * MM, MM),
        pt(3 * MM, 2 * MM),
        pt(2 * MM, 2 * MM),
        pt(2 * MM, 3 * MM),
        pt(MM, 3 * MM),
    ];
    add_zone(&mut board, 10, NET_A, l);
    let report = run(&board);
    let zone = &report.fill.zones[0];
    assert_eq!(zone.area_nm2, 3_000_000_000_000);
    assert!(zone.polygons.len() > 1);
    assert_eq!(report.stats[0].islands_kept, 1);
}

#[test]
fn the_zone_of_the_higher_priority_is_filled_first_and_the_other_keeps_away() {
    let mut board = base_board();
    let low = add_zone(&mut board, 10, NET_A, square(MM, MM, 11 * MM, 11 * MM));
    let high = add_zone(
        &mut board,
        11,
        NET_B,
        square(6 * MM, 6 * MM, 16 * MM, 16 * MM),
    );
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{high}\",\"priority\":1}}]}}"
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let report = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap();
    let area = |zone: Id| {
        report
            .fill
            .zones
            .iter()
            .find(|z| z.zone == zone)
            .map(|z| z.area_nm2)
            .unwrap()
    };
    assert_eq!(area(high), 100_000_000_000_000);
    // the low zone loses the square of 5 mm and a strip of 0.2 mm around it, with a rounded corner
    let low_area = area(low);
    assert!(
        low_area > 72_900_000_000_000 && low_area < 73_000_000_000_000,
        "{low_area}"
    );
    assert!(!covered_in(&report, low, 9 * MM, 9 * MM));
    assert!(!covered_in(&report, low, 5_900_000, 9 * MM));
    assert!(covered_in(&report, low, 3 * MM, 3 * MM));
    assert!(covered_in(&report, high, 9 * MM, 9 * MM));
    let findings = check_fill(&board, &rules(), &settings(), &report.fill).unwrap();
    assert!(!has_errors(&findings), "{findings:?}");
}

#[test]
fn two_zones_of_different_nets_with_the_same_priority_that_overlap_are_a_warning() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 11 * MM, 11 * MM));
    add_zone(
        &mut board,
        11,
        NET_B,
        square(6 * MM, 6 * MM, 16 * MM, 16 * MM),
    );
    let report = run(&board);
    assert!(
        report.findings.iter().any(|f| f.code == PRIORITY_TIE),
        "{:?}",
        report.findings
    );
}

#[test]
fn a_zone_named_in_the_file_but_not_on_the_board_stops_the_fill() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let ghost = id(99);
    let text = format!("{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{ghost}\"}}]}}");
    let file = ZoneFile::from_json(&text).unwrap();
    let error = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap_err();
    assert_eq!(error.code, ZONE_NOT_FOUND);
}

#[test]
fn a_zone_that_is_swallowed_by_copper_of_another_net_is_empty() {
    let mut board = base_board();
    add_zone(
        &mut board,
        10,
        NET_A,
        square(9_600_000, 9_600_000, 10_400_000, 10_400_000),
    );
    add_via(&mut board, 20, NET_B, 10 * MM, 10 * MM);
    let report = run(&board);
    assert!(report.fill.zones[0].polygons.is_empty());
    assert!(report.findings.iter().any(|f| f.code == FILL_EMPTY));
}

/// A footprint with one round pad of 1 mm, a component of the net on it and the net of the pad.
fn add_pad_of_a(board: &mut Board, x: i64, y: i64) {
    let footprint = id(30);
    let pad = id(31);
    let component = id(32);
    let mut pads = BTreeMap::new();
    pads.insert(
        pad,
        Pad {
            number: "1".to_owned(),
            shape: PadShape::Circle { diameter: nm(MM) },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![id(LAYER)],
            roundness: 0,
            plated: true,
        },
    );
    board.footprints.insert(
        footprint,
        Footprint {
            name: "round".to_owned(),
            pads,
        },
    );
    board.components.insert(
        component,
        Component {
            refdes: "C1".to_owned(),
            value: "1u".to_owned(),
            symbol: None,
            footprint,
            at: pt(x, y),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    board.pad_nets.push(PadNet {
        component,
        pad,
        net: id(NET_A),
    });
}

#[test]
fn a_pad_of_the_net_gets_a_thermal_relief_with_four_spokes() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_pad_of_a(&mut board, 10 * MM, 10 * MM);
    // the zone is joined to the pad, so the default of dropping loose islands applies
    let strict = ZoneSettings::default();
    let report = fill_board(&board, &rules(), &strict, None).unwrap();
    assert!(report.fill.zones[0].polygons.len() > 1);
    // along an axis the copper runs up to the pad: that is a spoke
    assert!(covered(&report, 10 * MM + 600_000, 10 * MM));
    assert!(covered(&report, 10 * MM, 10 * MM - 600_000));
    // on the diagonal, inside the ring, there is a gap
    assert!(!covered(&report, 10 * MM + 400_000, 10 * MM + 400_000));
    assert!(!covered(&report, 10 * MM - 400_000, 10 * MM + 400_000));
    // outside the ring the copper is whole again
    assert!(covered(&report, 10 * MM + 900_000, 10 * MM + 900_000));
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}

#[test]
fn a_solid_connection_leaves_no_gap_around_the_pad() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_pad_of_a(&mut board, 10 * MM, 10 * MM);
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{}\",\"connection\":\"solid\"}}]}}",
        id(10)
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let report = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap();
    assert_eq!(report.fill.zones[0].polygons.len(), 1);
    assert!(covered(&report, 10 * MM + 400_000, 10 * MM + 400_000));
}

#[test]
fn a_pad_that_is_not_joined_is_kept_apart_like_other_copper() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_pad_of_a(&mut board, 10 * MM, 10 * MM);
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{}\",\"connection\":\"none\"}}]}}",
        id(10)
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let report = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap();
    assert!(!covered(&report, 10 * MM + 600_000, 10 * MM));
    assert!(covered(&report, 10 * MM + 800_000, 10 * MM));
}

#[test]
fn an_island_with_no_copper_of_the_net_is_dropped_unless_asked_to_keep() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let strict = ZoneSettings {
        min_island_area_nm2: 0,
        ..ZoneSettings::default()
    };
    let report = fill_board(&board, &rules(), &strict, None).unwrap();
    assert!(report.fill.zones[0].polygons.is_empty());
    assert_eq!(report.stats[0].islands_removed, 1);
    assert!(report.findings.iter().any(|f| f.code == FILL_EMPTY));
}

#[test]
fn a_cutout_of_the_zone_file_is_kept_away_from() {
    let mut board = base_board();
    let zone = add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{zone}\",\"cutouts\":[[[8000000,8000000],[12000000,8000000],[12000000,12000000],[8000000,12000000]]]}}]}}"
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let report = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap();
    assert!(!covered(&report, 10 * MM, 10 * MM));
    assert!(covered(&report, 6 * MM, 10 * MM));
    assert!(covered(&report, 14 * MM, 10 * MM));
    // 18 mm by 18 mm less the hole of 4 mm by 4 mm
    assert_eq!(report.fill.zones[0].area_nm2, 308_000_000_000_000);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let findings = check_fill(&board, &rules(), &settings(), &report.fill).unwrap();
    assert!(!has_errors(&findings), "{findings:?}");
}

#[test]
fn a_cutout_that_is_not_a_polygon_stops_the_fill_of_that_zone() {
    let mut board = base_board();
    let zone = add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{zone}\",\"cutouts\":[[[8000000,8000000],[12000000,8000000],[16000000,8000000]]]}}]}}"
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let report = fill_board(&board, &rules(), &settings(), Some(&file)).unwrap();
    assert!(report.fill.zones.is_empty());
    assert!(has_errors(&report.findings));
    let two = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{zone}\",\"cutouts\":[[[1,1],[2,2]]]}}]}}"
    );
    assert_eq!(
        ZoneFile::from_json(&two).unwrap_err().code,
        ZONE_FILE_INVALID
    );
    let spokes = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{zone}\",\"thermal_spokes\":3}}]}}"
    );
    assert_eq!(
        ZoneFile::from_json(&spokes).unwrap_err().code,
        ZONE_FILE_INVALID
    );
}

#[test]
fn a_zone_of_the_file_has_its_own_thermal_relief() {
    let mut board = base_board();
    add_zone(&mut board, 10, NET_A, square(MM, MM, 19 * MM, 19 * MM));
    add_pad_of_a(&mut board, 10 * MM, 10 * MM);
    let text = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{}\",\"thermal_spokes\":2}}]}}",
        id(10)
    );
    let file = ZoneFile::from_json(&text).unwrap();
    let strict = ZoneSettings::default();
    let report = fill_board(&board, &rules(), &strict, Some(&file)).unwrap();
    // two spokes along x: the copper runs up to the pad along x and not along y
    assert!(covered(&report, 10 * MM + 600_000, 10 * MM));
    assert!(!covered(&report, 10 * MM, 10 * MM - 600_000));
    // spokes wider than the ring: the relief cannot be made and the pad is joined solid
    let wide = format!(
        "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{}\",\"thermal_spoke_width_nm\":10000000}}]}}",
        id(10)
    );
    let file = ZoneFile::from_json(&wide).unwrap();
    let report = fill_board(&board, &rules(), &strict, Some(&file)).unwrap();
    assert!(report.findings.iter().any(|f| f.code == RELIEF_IMPOSSIBLE));
    assert!(covered(&report, 10 * MM + 400_000, 10 * MM + 400_000));
}
