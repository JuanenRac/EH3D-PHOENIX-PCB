//! The round trip of a board that uses the cutouts of the outline, rounded pads, holes that are not
//! plated, the cutouts and keep-away of a pour (clear polarity) and blind and buried vias: the files
//! of the export read back as the board, with the attributes of every level of Gerber X2.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_export::profile::{X2Level, X2Profile};
use eh3d_phoenix_export::{build, FileRole, Profile};
use eh3d_phoenix_gerber::{drill_span, round_trip, DrillSpan, ImportSettings, Role, Source};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn mm(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn pad(
    number: &str,
    shape: PadShape,
    x: i64,
    drill: Option<i64>,
    plated: bool,
    roundness: u16,
) -> Pad {
    Pad {
        number: number.into(),
        shape,
        at: mm(x, 0),
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers: vec![id(1), id(5)],
        roundness,
        plated,
    }
}

/// Four copper layers, a top mask and an outline layer; the outline has a cutout; a component with
/// a rounded pad, a plated hole and a hole that is not plated; a pour with a cutout, a track and a
/// via of another net inside it; and vias of every span.
fn board() -> Board {
    let mut b = Board {
        name: "model".into(),
        outline: vec![mm(0, 0), mm(40, 0), mm(40, 20), mm(0, 20)],
        outline_cutouts: vec![vec![mm(30, 14), mm(36, 14), mm(36, 18), mm(30, 18)]],
        ..Board::default()
    };
    for (n, name, kind, side) in [
        (1, "Top", LayerKind::Copper, None),
        (2, "In1", LayerKind::Copper, None),
        (3, "In2", LayerKind::Copper, None),
        (4, "Bottom", LayerKind::Copper, None),
        (5, "Top mask", LayerKind::SolderMask, Some(Side::Top)),
        (6, "Outline", LayerKind::Outline, None),
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
    b.copper_stack = vec![id(1), id(2), id(3), id(4)];
    b.nets.insert(id(7), Net { name: "N1".into() });
    b.nets.insert(id(8), Net { name: "N2".into() });

    let mut pads = BTreeMap::new();
    pads.insert(
        id(11),
        pad(
            "1",
            PadShape::Rect {
                width: nm(2_000_000),
                height: nm(1_000_000),
            },
            -4,
            None,
            true,
            500,
        ),
    );
    pads.insert(
        id(12),
        pad(
            "2",
            PadShape::Circle {
                diameter: nm(1_600_000),
            },
            0,
            Some(800_000),
            true,
            0,
        ),
    );
    pads.insert(
        id(13),
        pad(
            "3",
            PadShape::Circle {
                diameter: nm(3_000_000),
            },
            4,
            Some(2_000_000),
            false,
            0,
        ),
    );
    b.footprints.insert(
        id(20),
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    b.components.insert(
        id(21),
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: id(20),
            at: mm(10, 5),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    for p in [11, 12] {
        b.pad_nets.push(PadNet {
            component: id(21),
            pad: id(p),
            net: id(7),
        });
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());

    b.zones.insert(
        id(30),
        Zone {
            net: id(7),
            layer: id(1),
            outline: vec![mm(1, 8), mm(39, 8), mm(39, 13), mm(1, 13)],
            clearance: nm(200_000),
            cutouts: vec![vec![mm(2, 9), mm(4, 9), mm(4, 11), mm(2, 11)]],
        },
    );
    b.tracks.insert(
        id(31),
        Track {
            net: id(8),
            layer: id(1),
            width: nm(250_000),
            path: vec![Segment::Line {
                start: mm(6, 10),
                end: mm(12, 10),
            }],
        },
    );
    for (n, x, from, to, net) in [
        (32, 14, 1, 4, 8),
        (40, 20, 1, 4, 7),
        (41, 22, 1, 2, 7),
        (42, 24, 2, 1, 7),
        (43, 26, 1, 3, 7),
        (44, 28, 2, 3, 7),
    ] {
        b.vias.insert(
            id(n),
            Via {
                net: id(net),
                at: mm(x, 10),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: id(from),
                to_layer: id(to),
            },
        );
    }
    b
}

fn profile(level: X2Level) -> Profile {
    let mut p = Profile::default();
    p.gerber.x2 = X2Profile {
        level,
        ..X2Profile::default()
    };
    p
}

#[test]
fn the_files_read_back_as_the_board_at_every_level_of_x2() {
    let b = board();
    for level in [X2Level::X1, X2Level::Legacy, X2Level::Full] {
        let trip = round_trip(&b, &profile(level), &ImportSettings::default()).unwrap();
        assert_eq!(trip.differences, Vec::new(), "{level:?}");
        assert_eq!(trip.imported.failed(), 0);
    }
}

#[test]
fn the_import_tells_the_drill_files_of_blind_and_buried_vias() {
    let b = board();
    let trip = round_trip(&b, &profile(X2Level::Full), &ImportSettings::default()).unwrap();
    // the layers: four copper, a mask and an outline; the drills: through, not plated, and three
    // spans
    assert_eq!(trip.files, 6 + 2 + 3);
    let mut spans: Vec<DrillSpan> = trip
        .imported
        .files
        .iter()
        .filter_map(|f| f.drill_span())
        .collect();
    spans.sort_by_key(|s| (s.from, s.to, s.buried));
    assert_eq!(
        spans,
        vec![
            DrillSpan {
                from: 1,
                to: 2,
                buried: false
            },
            DrillSpan {
                from: 1,
                to: 3,
                buried: false
            },
            DrillSpan {
                from: 2,
                to: 3,
                buried: true
            },
        ]
    );
    // they are plated drill files, told from the attribute, and the file of the through holes
    // is not a span
    let plated: Vec<_> = trip
        .imported
        .files
        .iter()
        .filter(|f| f.identified.is_some_and(|i| i.role == Role::DrillPlated))
        .collect();
    assert_eq!(plated.len(), 4);
    assert!(plated
        .iter()
        .all(|f| f.identified.unwrap().source == Source::Attribute));
    assert_eq!(
        plated.iter().filter(|f| f.drill_span().is_none()).count(),
        1
    );
}

#[test]
fn the_span_of_a_drill_file_is_told_from_its_file_function() {
    let attributes = |values: &[&str]| {
        BTreeMap::from([(
            ".FileFunction".to_owned(),
            values.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>(),
        )])
    };
    assert_eq!(
        drill_span(&attributes(&["Plated", "1", "2", "Blind", "Drill"])),
        Some(DrillSpan {
            from: 1,
            to: 2,
            buried: false
        })
    );
    // the ends in the other order
    assert_eq!(
        drill_span(&attributes(&["Plated", "3", "2", "Buried"])),
        Some(DrillSpan {
            from: 2,
            to: 3,
            buried: true
        })
    );
    assert_eq!(
        drill_span(&attributes(&["Plated", "1", "4", "PTH", "Drill"])),
        None
    );
    assert_eq!(
        drill_span(&attributes(&["NonPlated", "1", "4", "NPTH"])),
        None
    );
    assert_eq!(
        drill_span(&attributes(&["Plated", "a", "2", "Blind"])),
        None
    );
    assert_eq!(drill_span(&BTreeMap::new()), None);
}

#[test]
fn a_file_of_a_span_that_changes_is_found() {
    let b = board();
    let mut set = build(&b, &profile(X2Level::Legacy)).unwrap().files;
    // take the holes out of the file of the blind vias between layers 1 and 2
    let file = set
        .iter_mut()
        .find(|f| matches!(f.role, FileRole::DrillSpan { from: 1, to: 2, .. }))
        .unwrap();
    let text = String::from_utf8(file.bytes.clone()).unwrap();
    let kept: Vec<&str> = text.lines().filter(|l| !l.starts_with('X')).collect();
    assert!(kept.len() < text.lines().count(), "{text}");
    file.bytes = kept.join("\n").into_bytes();
    let trip = eh3d_phoenix_gerber::compare_files(&b, &set, &ImportSettings::default());
    assert!(
        trip.differences.iter().any(|d| d.what.contains("holes")),
        "{:?}",
        trip.differences
    );
}
