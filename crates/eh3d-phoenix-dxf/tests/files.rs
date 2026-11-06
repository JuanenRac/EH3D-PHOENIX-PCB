//! The reader, the chaining and the writer on the small drawings of `tests/dxf`, written by hand
//! with the results worked out by hand: a plate of four loose lines with a gap of 4 micrometres,
//! a hole and a text; a plate with four mounting holes made by inserts of one block; and a
//! stadium in inches with arcs written as bulges.

use eh3d_phoenix_dxf::codes::{
    ARCS_APPROXIMATED, CHAIN_OPEN, CUTOUT_NOT_KEPT, ENTITY_UNSUPPORTED, UNIT_UNKNOWN,
};
use eh3d_phoenix_dxf::geom::{bounds, twice_area};
use eh3d_phoenix_dxf::{extract_outline, read_dxf, write_dxf, DxfSettings, Seg, Shape};

const MM: i64 = 1_000_000;

const PLATE_LINES: &str = include_str!("../../../tests/dxf/plate_lines.dxf");
const PLATE_BLOCKS: &str = include_str!("../../../tests/dxf/plate_blocks.dxf");
const STADIUM: &str = include_str!("../../../tests/dxf/stadium_inches.dxf");

fn area_mm2(poly: &[(i64, i64)]) -> f64 {
    twice_area(poly).abs() as f64 / 2.0 / (MM as f64 * MM as f64)
}

#[test]
fn four_lines_with_a_gap_of_4_micrometres_make_the_plate_and_the_hole_is_listed() {
    let settings = DxfSettings::default();
    let read = read_dxf(PLATE_LINES, &settings).unwrap();
    assert_eq!(read.unit_mm, 1.0);
    assert_eq!(read.counts.get("LINE"), Some(&4));
    assert_eq!(read.counts.get("CIRCLE"), Some(&1));
    assert_eq!(read.counts.get("TEXT"), Some(&1));
    // no unit in the file, and a text that is not read
    let codes: Vec<&str> = read.report.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![UNIT_UNKNOWN, ENTITY_UNSUPPORTED]);
    assert_eq!(
        read.report.findings[1].detail,
        "1 left out, on the layers NOTES"
    );

    let found = extract_outline(
        &read.drawing,
        Some("edge"),
        settings.join_tolerance_nm,
        &settings,
    );
    let exterior = found.exterior.clone().unwrap();
    assert_eq!(
        exterior,
        vec![(0, 0), (50 * MM, 0), (50 * MM, 30 * MM), (0, 30 * MM)]
    );
    assert_eq!(
        twice_area(&exterior),
        3000 * i128::from(MM) * i128::from(MM)
    );
    assert_eq!(found.holes.len(), 1);
    // a circle of radius 1.6 mm: 8.042 mm2, a little less as a polygon
    let hole = area_mm2(&found.holes[0]);
    assert!(hole > 7.95 && hole < 8.05, "{hole}");
    let codes: Vec<&str> = found.report.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![CUTOUT_NOT_KEPT, ARCS_APPROXIMATED]);
    assert_eq!(
        (
            found.stats.pieces,
            found.stats.chains,
            found.stats.contours,
            found.stats.open_chains,
            found.stats.duplicates,
            found.stats.arcs_approximated
        ),
        (5, 1, 2, 0, 0, 1)
    );
}

#[test]
fn with_a_tolerance_below_the_gap_the_plate_is_lost_and_the_report_says_so() {
    let settings = DxfSettings::default();
    let read = read_dxf(PLATE_LINES, &settings).unwrap();
    let found = extract_outline(&read.drawing, Some("EDGE"), 1_000, &settings);
    // the chain of lines stays open; the only closed contour is the hole
    assert_eq!(found.stats.open_chains, 1);
    let taken = area_mm2(&found.exterior.unwrap());
    assert!(taken > 7.95 && taken < 8.05, "{taken}");
    let codes: Vec<&str> = found.report.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![CHAIN_OPEN, ARCS_APPROXIMATED]);
    assert!(found.report.findings[0]
        .detail
        .starts_with("4 pieces from (0.000, 0.000) mm to (0.004, 0.000) mm"));
}

#[test]
fn the_four_inserts_of_one_block_are_four_holes_of_1_5_mm_at_their_places() {
    let settings = DxfSettings::default();
    let read = read_dxf(PLATE_BLOCKS, &settings).unwrap();
    assert!(
        read.report.findings.is_empty(),
        "{:?}",
        read.report.findings
    );
    assert_eq!(read.counts.get("INSERT"), Some(&4));
    assert_eq!(read.counts.get("CIRCLE"), Some(&1));
    assert_eq!(read.counts.get("LWPOLYLINE"), Some(&1));
    assert_eq!(read.drawing.entities.len(), 5);
    let circles: Vec<_> = read
        .drawing
        .entities
        .iter()
        .filter_map(|e| match &e.shape {
            Shape::Circle { centre, radius } => Some((e.layer.as_str(), *centre, *radius)),
            _ => None,
        })
        .collect();
    assert_eq!(
        circles,
        vec![
            ("EDGE", (5 * MM, 5 * MM), 1_500_000),
            ("EDGE", (45 * MM, 5 * MM), 1_500_000),
            ("EDGE", (5 * MM, 25 * MM), 1_500_000),
            ("EDGE", (45 * MM, 25 * MM), 1_500_000),
        ]
    );
    let found = extract_outline(&read.drawing, None, settings.join_tolerance_nm, &settings);
    assert_eq!(
        twice_area(&found.exterior.unwrap()),
        3000 * i128::from(MM) * i128::from(MM)
    );
    assert_eq!(found.holes.len(), 4);
    for hole in &found.holes {
        // the polygon of a circle: all corners on it and the middle of the corners at its centre
        let n = hole.len() as i64;
        let centre = (
            hole.iter().map(|p| p.0).sum::<i64>() / n,
            hole.iter().map(|p| p.1).sum::<i64>() / n,
        );
        assert!(
            [(5, 5), (45, 5), (5, 25), (45, 25)]
                .iter()
                .any(|(x, y)| (centre.0 - x * MM).abs() < 100 && (centre.1 - y * MM).abs() < 100),
            "{centre:?}"
        );
        for p in hole {
            let r = ((p.0 - centre.0) as f64).hypot((p.1 - centre.1) as f64);
            assert!((r - 1_500_000.0).abs() < 200.0, "{r}");
        }
    }
    let codes: Vec<&str> = found.report.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![CUTOUT_NOT_KEPT, ARCS_APPROXIMATED]);
    assert!(found.report.findings[0].detail.starts_with("4 cutouts"));
}

#[test]
fn a_stadium_in_inches_keeps_its_arcs_as_pieces_with_a_sweep_and_has_the_area_worked_out() {
    let settings = DxfSettings::default();
    let read = read_dxf(STADIUM, &settings).unwrap();
    assert_eq!(read.unit_mm, 25.4);
    assert!(read.report.findings.is_empty());
    let Shape::Path { segs, closed } = &read.drawing.entities[0].shape else {
        panic!("not a path");
    };
    assert!(*closed);
    assert_eq!(
        segs,
        &vec![
            Seg::line((0, 0), (50_800_000, 0)),
            Seg {
                a: (50_800_000, 0),
                b: (50_800_000, 25_400_000),
                sweep_udeg: 180_000_000
            },
            Seg::line((50_800_000, 25_400_000), (0, 25_400_000)),
            Seg {
                a: (0, 25_400_000),
                b: (0, 0),
                sweep_udeg: 180_000_000
            },
        ]
    );
    let found = extract_outline(&read.drawing, None, settings.join_tolerance_nm, &settings);
    let exterior = found.exterior.unwrap();
    // 2 by 1 inch and two half circles of radius half an inch: 2.7854 square inches
    let area = area_mm2(&exterior);
    assert!((area - 1797.03).abs() < 1.0, "{area}");
    let (low, high) = bounds(&exterior);
    assert!((low.0 + 12_700_000).abs() <= 5_000, "{low:?}");
    assert!((high.0 - 63_500_000).abs() <= 5_000, "{high:?}");
    assert_eq!((low.1, high.1), (0, 25_400_000));
    assert_eq!(found.stats.arcs_approximated, 2);
    assert!(found.holes.is_empty());
    let codes: Vec<&str> = found.report.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![ARCS_APPROXIMATED]);
}

#[test]
fn what_is_read_and_written_and_read_again_is_the_same_drawing() {
    let settings = DxfSettings::default();
    for text in [PLATE_LINES, PLATE_BLOCKS, STADIUM] {
        let first = read_dxf(text, &settings).unwrap();
        let written = write_dxf(&first.drawing);
        let second = read_dxf(&written, &settings).unwrap();
        assert_eq!(first.drawing, second.drawing);
        // a drawing the suite wrote has its unit, so there is no finding about it
        assert!(
            second.report.findings.is_empty(),
            "{:?}",
            second.report.findings
        );
        assert_eq!(written, write_dxf(&second.drawing));
    }
}
