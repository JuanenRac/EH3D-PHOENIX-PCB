// SPDX-License-Identifier: GPL-3.0-or-later
//! The statistics of boards written by the test itself, with the numbers worked out by hand.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stats::codes::{
    Severity, LAYER_UNBALANCED, LIMIT_EXCEEDED, NO_BOARD_AREA, REGION_UNEVEN, STACK_ORDER_UNKNOWN,
    ZONE_OUTLINE_COUNTED,
};
use eh3d_phoenix_stats::{compute, report_json, report_text, StatsSettings};

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

fn layer(name: &str) -> Layer {
    Layer {
        name: name.into(),
        kind: LayerKind::Copper,
        side: None,
    }
}

fn pad(number: &str, shape: PadShape, drill: Option<i64>, plated: bool, layers: &[Id]) -> Pad {
    Pad {
        number: number.into(),
        shape,
        at: pt(0, 0),
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers: layers.to_vec(),
        roundness: 0,
        plated,
    }
}

/// A 20 x 10 mm board with a 2 x 2 mm opening, two copper layers, two parts of one footprint
/// (one on each side) with an SMD pad, a plated pad and a mounting hole, a track of 10 mm, a via
/// and a zone of the bottom layer with a cutout.
fn board() -> Board {
    let (top, bottom) = (id(1), id(2));
    let mut b = Board {
        name: "stats".into(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        outline_cutouts: vec![vec![
            pt(2 * MM, 2 * MM),
            pt(4 * MM, 2 * MM),
            pt(4 * MM, 4 * MM),
            pt(2 * MM, 4 * MM),
        ]],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    b.layers.insert(top, layer("Top"));
    b.layers.insert(bottom, layer("Bottom"));
    b.nets.insert(id(3), Net { name: "SIG".into() });
    let mut footprint = Footprint {
        name: "FP".into(),
        pads: Default::default(),
    };
    footprint.pads.insert(
        id(11),
        pad(
            "1",
            PadShape::Rect {
                width: nm(MM),
                height: nm(MM / 2),
            },
            None,
            true,
            &[top],
        ),
    );
    footprint.pads.insert(
        id(12),
        pad(
            "2",
            PadShape::Circle {
                diameter: nm(1_600_000),
            },
            Some(800_000),
            true,
            &[top, bottom],
        ),
    );
    footprint.pads.insert(
        id(13),
        pad(
            "3",
            PadShape::Circle {
                diameter: nm(3 * MM),
            },
            Some(3 * MM),
            false,
            &[top, bottom],
        ),
    );
    b.footprints.insert(id(10), footprint);
    for (n, side) in [(20_u32, Side::Top), (21, Side::Bottom)] {
        b.components.insert(
            id(n),
            Component {
                refdes: format!("U{n}"),
                value: "x".into(),
                symbol: None,
                footprint: id(10),
                at: pt(10 * MM, 5 * MM),
                rotation: Udeg::ZERO,
                side,
            },
        );
    }
    b.pad_nets = vec![PadNet {
        component: id(20),
        pad: id(11),
        net: id(3),
    }];
    b.tracks.insert(
        id(30),
        Track {
            net: id(3),
            layer: top,
            width: nm(250_000),
            path: vec![Segment::Line {
                start: pt(5 * MM, 5 * MM),
                end: pt(15 * MM, 5 * MM),
            }],
        },
    );
    b.vias.insert(
        id(31),
        Via {
            net: id(3),
            at: pt(8 * MM, 8 * MM),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: top,
            to_layer: bottom,
        },
    );
    b.zones.insert(
        id(32),
        Zone {
            net: id(3),
            layer: bottom,
            outline: vec![
                pt(0, 0),
                pt(20 * MM, 0),
                pt(20 * MM, 10 * MM),
                pt(0, 10 * MM),
            ],
            clearance: nm(200_000),
            cutouts: vec![vec![
                pt(10 * MM, 0),
                pt(15 * MM, 0),
                pt(15 * MM, 5 * MM),
                pt(10 * MM, 5 * MM),
            ]],
        },
    );
    b
}

#[test]
fn the_size_and_the_area_of_the_board_are_exact() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    assert_eq!(s.board.width_nm, 20 * MM);
    assert_eq!(s.board.height_nm, 10 * MM);
    assert_eq!(s.board.outline_um2, 200_000_000);
    assert_eq!(s.board.cutouts_um2, 4_000_000);
    assert_eq!(s.board.area_um2, 196_000_000);
}

#[test]
fn the_counts_are_the_ones_of_the_board() {
    let c = compute(&board(), &StatsSettings::default()).unwrap().counts;
    assert_eq!(
        (c.components, c.components_top, c.components_bottom),
        (2, 1, 1)
    );
    assert_eq!(c.nets, 1);
    assert_eq!(
        (c.pads, c.smd_pads, c.through_pads, c.unplated_holes),
        (6, 2, 2, 2)
    );
    assert_eq!(
        (c.tracks, c.track_segments, c.track_length_nm),
        (1, 1, 10 * MM)
    );
    assert_eq!((c.vias, c.zones, c.silkscreen_items), (1, 1, 0));
}

#[test]
fn the_drills_are_counted_by_diameter_from_the_smallest() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    let rows: Vec<(i64, usize, usize, usize, usize)> = s
        .drills
        .iter()
        .map(|d| {
            (
                d.diameter_nm,
                d.vias,
                d.plated_pads,
                d.unplated_pads,
                d.total,
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (300_000, 1, 0, 0, 1),
            (800_000, 0, 2, 0, 2),
            (3 * MM, 0, 0, 2, 2),
        ]
    );
}

#[test]
fn the_copper_of_each_layer_is_the_sum_of_its_parts() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    let [top, bottom] = s.layers.as_slice() else {
        panic!("{} layers", s.layers.len());
    };
    assert_eq!((top.layer.as_str(), top.position), ("Top", 1));
    assert_eq!((bottom.layer.as_str(), bottom.position), ("Bottom", 2));
    // two SMD pads of 0.5 mm2 and two plated pads of 1.507964 mm2
    assert_eq!(top.pads, 4);
    assert_eq!(top.pads_um2, 4_015_928);
    // 10 mm by 0.25 mm and the two round ends
    assert_eq!((top.tracks, top.track_length_nm), (1, 10 * MM));
    assert_eq!(top.tracks_um2, 2_549_087);
    // the ring between the drill of 0.3 mm and the pad of 0.6 mm, on both layers
    assert_eq!((top.vias, top.vias_um2), (1, 212_057));
    assert_eq!((bottom.vias, bottom.vias_um2), (1, 212_057));
    assert_eq!((top.zones, top.zones_um2), (0, 0));
    // the outline of 200 mm2 less the cutout of 25 mm2
    assert_eq!((bottom.zones, bottom.zones_um2), (1, 175_000_000));
    assert_eq!(bottom.pads, 2);
    assert_eq!(bottom.pads_um2, 3_015_928);
    assert_eq!(top.copper_um2, 6_777_073);
    assert_eq!(top.coverage_x100, 345);
    assert_eq!(bottom.copper_um2, 178_227_986);
    assert_eq!(bottom.coverage_x100, 9_093);
}

#[test]
fn a_hole_that_is_not_plated_has_no_copper() {
    let mut b = board();
    // leave only the mounting hole on the footprint
    let fp = b.footprints.get_mut(&id(10)).unwrap();
    fp.pads.remove(&id(11));
    fp.pads.remove(&id(12));
    b.tracks.clear();
    b.vias.clear();
    b.zones.clear();
    let s = compute(&b, &StatsSettings::default()).unwrap();
    assert!(s.layers.iter().all(|l| l.copper_um2 == 0 && l.pads == 0));
    assert_eq!(s.counts.unplated_holes, 2);
}

#[test]
fn the_zones_can_be_left_out_of_the_copper() {
    let settings = StatsSettings {
        count_zones: false,
        ..StatsSettings::default()
    };
    let s = compute(&board(), &settings).unwrap();
    assert_eq!(s.layers[1].zones, 1);
    assert_eq!(s.layers[1].zones_um2, 0);
    assert!(s.findings.iter().all(|f| f.code != ZONE_OUTLINE_COUNTED));
    let counted = compute(&board(), &StatsSettings::default()).unwrap();
    assert!(counted
        .findings
        .iter()
        .any(|f| f.code == ZONE_OUTLINE_COUNTED));
}

#[test]
fn the_copper_of_a_layer_is_never_above_the_area_of_the_board() {
    let mut b = board();
    // a zone that is bigger than the board itself
    let zone = b.zones.get_mut(&id(32)).unwrap();
    zone.outline = vec![
        pt(-50 * MM, -50 * MM),
        pt(50 * MM, -50 * MM),
        pt(50 * MM, 50 * MM),
        pt(-50 * MM, 50 * MM),
    ];
    zone.cutouts.clear();
    let s = compute(&b, &StatsSettings::default()).unwrap();
    assert_eq!(s.layers[1].copper_um2, s.board.area_um2);
    assert_eq!(s.layers[1].coverage_x100, 10_000);
}

#[test]
fn a_via_crosses_the_layers_between_its_ends() {
    let mut b = board();
    let inner = id(4);
    b.layers.insert(inner, layer("Inner"));
    b.copper_stack = vec![id(1), inner, id(2)];
    let s = compute(&b, &StatsSettings::default()).unwrap();
    let vias: Vec<usize> = s.layers.iter().map(|l| l.vias).collect();
    assert_eq!(vias, vec![1, 1, 1]);
}

#[test]
fn a_board_with_no_outline_has_no_area_and_says_so() {
    let s = compute(&Board::default(), &StatsSettings::default()).unwrap();
    assert_eq!(s.board.area_um2, 0);
    assert_eq!(s.density.components_per_cm2_x100, 0);
    assert!(s.findings.iter().any(|f| f.code == NO_BOARD_AREA));
    assert!(s.layers.is_empty() && s.drills.is_empty());
}

#[test]
fn a_board_with_too_many_objects_is_refused() {
    let settings = StatsSettings {
        max_items: 3,
        ..StatsSettings::default()
    };
    let e = compute(&board(), &settings).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
}

#[test]
fn the_density_is_per_square_centimetre_of_board() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    // 2 parts on 1.96 cm2: 1.02 per cm2
    assert_eq!(s.density.components_per_cm2_x100, 102);
    // 6 pads: 3.06
    assert_eq!(s.density.pads_per_cm2_x100, 306);
    // 2 plated pads, 2 holes and 1 via: 5 holes, 2.55
    assert_eq!(s.density.holes_per_cm2_x100, 255);
}

#[test]
fn the_text_is_in_the_language_asked_for_and_the_same_every_time() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    let en = report_text(&s, Lang::En);
    assert!(en.starts_with("Board statistics: stats\n"), "{en}");
    assert!(
        en.contains("Board: 20.000000 x 10.000000 mm; 196.000000 mm2"),
        "{en}"
    );
    assert!(en.contains("  Top: copper 6.777073 mm2"), "{en}");
    assert!(
        en.contains("0.300000 mm: 1 Vias; 0 plated pads; 0 unplated"),
        "{en}"
    );
    assert_eq!(
        en,
        report_text(
            &compute(&board(), &StatsSettings::default()).unwrap(),
            Lang::En
        )
    );
    let titles: Vec<String> = Lang::ALL
        .iter()
        .map(|l| report_text(&s, *l).lines().next().unwrap().to_owned())
        .collect();
    for (i, a) in titles.iter().enumerate() {
        for b in &titles[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert!(report_text(&s, Lang::Es).starts_with("Estadísticas de la placa: stats\n"));
    assert!(report_text(&s, Lang::Ja).starts_with("基板の統計: stats\n"));
}

#[test]
fn the_json_is_the_same_every_time() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    assert_eq!(report_json(&s), report_json(&s.clone()));
    assert!(report_json(&s).ends_with('\n'));
}

#[test]
fn the_layers_that_mirror_each_other_are_compared() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    let [pair] = s.balance.as_slice() else {
        panic!("{} pairs", s.balance.len());
    };
    assert_eq!(
        (pair.layer_a.as_str(), pair.layer_b.as_str()),
        ("Top", "Bottom")
    );
    assert_eq!((pair.coverage_a_x100, pair.coverage_b_x100), (345, 9_093));
    assert_eq!(pair.difference_x100, 8_748);
    assert_eq!(pair.limit_x100, 1_500);
    assert!(!pair.balanced);
    let finding = s
        .findings
        .iter()
        .find(|f| f.code == LAYER_UNBALANCED)
        .unwrap();
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.subject.as_deref(), Some("Top / Bottom"));
    assert!(!s.has_errors());
}

#[test]
fn the_balance_can_be_off_or_strict() {
    let off = StatsSettings {
        balance_check: false,
        ..StatsSettings::default()
    };
    let s = compute(&board(), &off).unwrap();
    assert!(s.balance.is_empty());
    assert!(s.findings.iter().all(|f| f.code != LAYER_UNBALANCED));
    let strict = StatsSettings {
        balance_severity: Severity::Error,
        ..StatsSettings::default()
    };
    assert!(compute(&board(), &strict).unwrap().has_errors());
    let loose = StatsSettings {
        balance_max_difference_x100: 10_000,
        ..StatsSettings::default()
    };
    let s = compute(&board(), &loose).unwrap();
    assert!(s.balance[0].balanced);
    assert!(s.findings.iter().all(|f| f.code != LAYER_UNBALANCED));
}

#[test]
fn a_board_without_the_order_of_its_layers_is_paired_by_id_and_says_so() {
    let mut b = board();
    b.copper_stack.clear();
    let s = compute(&b, &StatsSettings::default()).unwrap();
    assert_eq!(s.balance.len(), 1);
    assert!(s.layers.iter().all(|l| l.position == 0));
    assert!(s.findings.iter().any(|f| f.code == STACK_ORDER_UNKNOWN));
}

#[test]
fn the_text_lists_the_pairs_in_the_language_asked_for() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    let en = report_text(&s, Lang::En);
    assert!(en.contains("Copper balance:\n"), "{en}");
    assert!(
        en.contains("  Top / Bottom: 3.45 % against 90.93 %, difference 87.48 points\n"),
        "{en}"
    );
    assert!(en.contains("warning: PHOENIX-STATS-008"), "{en}");
    assert!(report_text(&s, Lang::De).contains("Kupferbalance:\n"));
    assert!(report_text(&s, Lang::Pt).contains("Equilíbrio do cobre:\n"));
}

/// The settings that cut the 20 x 10 mm box of the board into two cells side by side, each 10 x 10
/// mm: the opening of 2 x 2 mm is in the left one.
fn two_columns() -> StatsSettings {
    StatsSettings {
        region_columns: 2,
        ..StatsSettings::default()
    }
}

#[test]
fn without_a_grid_there_are_no_regions_and_the_json_has_no_trace_of_them() {
    let s = compute(&board(), &StatsSettings::default()).unwrap();
    assert!(s.regions.is_empty());
    assert!(!report_json(&s).contains("regions"));
    assert!(!report_text(&s, Lang::En).contains("Regions"));
    assert!(s.findings.iter().all(|f| f.code != REGION_UNEVEN));
}

#[test]
fn the_copper_of_each_layer_is_added_up_in_each_cell_of_the_grid() {
    let s = compute(&board(), &two_columns()).unwrap();
    let [top, bottom] = s.regions.as_slice() else {
        panic!("{} layers", s.regions.len());
    };
    assert_eq!((top.layer.as_str(), top.columns, top.rows), ("Top", 2, 1));
    assert_eq!(bottom.layer, "Bottom");
    let cells = |r: &eh3d_phoenix_stats::RegionStats| -> Vec<(usize, usize, i64, i64, i64)> {
        r.cells
            .iter()
            .map(|c| (c.column, c.row, c.area_um2, c.copper_um2, c.coverage_x100))
            .collect()
    };
    // left: half of the track, its first end and the via; right: the two pads of each part, the
    // other half of the track and its second end. The pads are on the edge between the cells and
    // belong to the right one.
    assert_eq!(
        cells(top),
        vec![
            (0, 0, 96_000_000, 1_486_601, 154),
            (1, 0, 100_000_000, 5_290_472, 529),
        ]
    );
    // left: the zone and the via, more than the area of the cell, so the cell is full; right: the
    // zone less its notch and the two plated pads
    assert_eq!(
        cells(bottom),
        vec![
            (0, 0, 96_000_000, 96_000_000, 10_000),
            (1, 0, 100_000_000, 78_015_928, 7801),
        ]
    );
}

#[test]
fn the_cells_have_the_area_of_the_board_and_hold_the_copper_of_the_layer() {
    let settings = StatsSettings {
        region_columns: 2,
        region_rows: 2,
        ..StatsSettings::default()
    };
    let s = compute(&board(), &settings).unwrap();
    for region in &s.regions {
        assert_eq!(region.cells.len(), 4);
        let rows: Vec<(usize, usize)> = region.cells.iter().map(|c| (c.column, c.row)).collect();
        assert_eq!(rows, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
        // the area of the board, with the opening of 2 x 2 mm in the lower left cell
        let areas: Vec<i64> = region.cells.iter().map(|c| c.area_um2).collect();
        assert_eq!(areas, vec![46_000_000, 50_000_000, 50_000_000, 50_000_000]);
    }
    // nothing of the top layer is cut off by the area of a cell, so the cells hold all of it but
    // the fractions that each cell rounds off
    let top = &s.layers[0];
    let held: i64 = s.regions[0].cells.iter().map(|c| c.copper_um2).sum();
    assert!(
        held <= top.copper_um2 && top.copper_um2 - held < 4,
        "{held} {}",
        top.copper_um2
    );
}

#[test]
fn a_layer_with_copper_spread_unevenly_is_reported_when_a_limit_is_set() {
    let limit = StatsSettings {
        region_max_difference_x100: 1000,
        ..two_columns()
    };
    let s = compute(&board(), &limit).unwrap();
    let found: Vec<_> = s
        .findings
        .iter()
        .filter(|f| f.code == REGION_UNEVEN)
        .collect();
    assert_eq!(found.len(), 1, "{:?}", s.findings);
    assert_eq!(found[0].subject.as_deref(), Some("Bottom"));
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(
        found[0].detail,
        "coverage from 78.01 % to 100.00 %, difference 21.99 points, limit 10.00"
    );
    // the severity is the one of the balance
    let strict = StatsSettings {
        balance_severity: Severity::Error,
        ..limit.clone()
    };
    let s = compute(&board(), &strict).unwrap();
    let uneven = s.findings.iter().find(|f| f.code == REGION_UNEVEN).unwrap();
    assert_eq!(uneven.severity, Severity::Error);
    // a limit of 0 leaves the regions unjudged, and a wide limit passes
    let off = compute(&board(), &two_columns()).unwrap();
    assert!(off.findings.iter().all(|f| f.code != REGION_UNEVEN));
    let wide = StatsSettings {
        region_max_difference_x100: 5000,
        ..two_columns()
    };
    let s = compute(&board(), &wide).unwrap();
    assert!(s.findings.iter().all(|f| f.code != REGION_UNEVEN));
}

#[test]
fn the_text_and_the_json_carry_the_regions() {
    let s = compute(&board(), &two_columns()).unwrap();
    let en = report_text(&s, Lang::En);
    assert!(
        en.contains(
            "Regions: 2 x 1:\n  Top:\n    1.54 %  5.29 %\n  Bottom:\n    100.00 %  78.01 %\n"
        ),
        "{en}"
    );
    assert!(report_text(&s, Lang::De).contains("Bereiche: 2 x 1:\n"));
    assert!(report_text(&s, Lang::Pt).contains("Regiões: 2 x 1:\n"));
    let json = report_json(&s);
    assert!(json.contains("\"regions\""), "{json}");
    assert!(json.contains("\"coverage_x100\": 7801"), "{json}");
    // the same board gives the same bytes
    assert_eq!(
        json,
        report_json(&compute(&board(), &two_columns()).unwrap())
    );
}

#[test]
fn the_rows_are_written_with_the_greatest_y_first() {
    let settings = StatsSettings {
        region_rows: 2,
        ..StatsSettings::default()
    };
    let s = compute(&board(), &settings).unwrap();
    let en = report_text(&s, Lang::En);
    let top_layer: Vec<&str> = en
        .lines()
        .skip_while(|l| !l.starts_with("Regions:"))
        .skip(2)
        .take(2)
        .collect();
    let region = &s.regions[0];
    let upper = format!("    {} %", hundredths_of(region.cells[1].coverage_x100));
    let lower = format!("    {} %", hundredths_of(region.cells[0].coverage_x100));
    assert_eq!(top_layer, vec![upper.as_str(), lower.as_str()], "{en}");
}

fn hundredths_of(x100: i64) -> String {
    format!("{}.{:02}", x100 / 100, x100 % 100)
}

#[test]
fn a_board_with_no_area_has_no_regions() {
    let mut b = board();
    b.outline.clear();
    let s = compute(&b, &two_columns()).unwrap();
    assert!(s.regions.is_empty());
    assert!(s.findings.iter().any(|f| f.code == NO_BOARD_AREA));
}
