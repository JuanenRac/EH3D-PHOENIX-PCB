//! The round trip: the files that the export writes for a board are read back by this import and
//! compared with the board; a file that is changed afterwards is found.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_export::{build, FabFile, FileRole, Profile};
use eh3d_phoenix_gerber::{
    compare_files, round_trip, Content, ImportSettings, Role, Source, Units,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}
fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}
fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

/// Two copper layers in a stack, a mask, a paste layer and an outline; two components of a
/// footprint with a round through-hole pad, a rectangular SMD pad and a mounting hole; a track
/// with a line and an arc; two vias; a zone.
fn board() -> Board {
    let (top, bottom, mask, paste, outline) = (id(1), id(2), id(3), id(4), id(5));
    let net = id(6);
    let mut b = Board {
        name: "Round trip".into(),
        outline: vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 20_000_000),
            pt(0, 20_000_000),
        ],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    for (l, name, kind, side) in [
        (top, "Top copper", LayerKind::Copper, None),
        (bottom, "Bottom copper", LayerKind::Copper, None),
        (mask, "Top mask", LayerKind::SolderMask, Some(Side::Top)),
        (paste, "Top paste", LayerKind::Paste, Some(Side::Top)),
        (outline, "Outline", LayerKind::Outline, None),
    ] {
        b.layers.insert(
            l,
            Layer {
                name: name.into(),
                kind,
                side,
            },
        );
    }
    b.nets.insert(net, Net { name: "N".into() });
    let fp = id(10);
    let (round, rect, mounting) = (id(11), id(12), id(13));
    let pad = |number: &str, shape, at, drill: Option<i64>, layers: Vec<Id>| Pad {
        number: number.into(),
        shape,
        at,
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers,
    };
    let mut pads = BTreeMap::new();
    pads.insert(
        round,
        pad(
            "1",
            PadShape::Circle {
                diameter: nm(1_600_000),
            },
            pt(0, 0),
            Some(800_000),
            vec![top, bottom, mask],
        ),
    );
    pads.insert(
        rect,
        pad(
            "2",
            PadShape::Rect {
                width: nm(1_000_000),
                height: nm(2_000_000),
            },
            pt(5_000_000, 0),
            None,
            vec![top, mask, paste],
        ),
    );
    pads.insert(
        mounting,
        pad(
            "M",
            PadShape::Circle {
                diameter: nm(3_000_000),
            },
            pt(0, 4_000_000),
            Some(3_000_000),
            vec![],
        ),
    );
    b.footprints.insert(
        fp,
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    for (cid, refdes, x) in [(id(20), "R1", 5_000_000), (id(21), "R2", 20_000_000)] {
        b.components.insert(
            cid,
            Component {
                refdes: refdes.into(),
                value: "1k".into(),
                symbol: None,
                footprint: fp,
                at: pt(x, 10_000_000),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        b.pad_nets.push(PadNet {
            component: cid,
            pad: round,
            net,
        });
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    b.tracks.insert(
        id(30),
        Track {
            net,
            layer: top,
            width: nm(250_000),
            path: vec![
                Segment::Line {
                    start: pt(5_000_000, 10_000_000),
                    end: pt(10_000_000, 10_000_000),
                },
                Segment::Arc {
                    start: pt(10_000_000, 10_000_000),
                    end: pt(12_000_000, 12_000_000),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        },
    );
    for (vid, x) in [(id(40), 12_000_000), (id(41), 14_000_000)] {
        b.vias.insert(
            vid,
            Via {
                net,
                at: pt(x, 12_000_000),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
    }
    b.zones.insert(
        id(50),
        Zone {
            net,
            layer: bottom,
            outline: vec![
                pt(1_000_000, 1_000_000),
                pt(29_000_000, 1_000_000),
                pt(29_000_000, 19_000_000),
            ],
            clearance: nm(200_000),
        },
    );
    b
}

fn files(b: &Board) -> Vec<FabFile> {
    build(b, &Profile::default()).unwrap().files
}

#[test]
fn the_files_of_the_export_read_back_as_the_board() {
    let b = board();
    let trip = round_trip(&b, &Profile::default(), &ImportSettings::default()).unwrap();
    assert_eq!(trip.differences, Vec::new());
    // five layers and the two drill files
    assert_eq!(trip.files, 7);
    assert_eq!(trip.imported.failed(), 0);
    // the layers are told from the attributes of the files
    let top = trip.imported.by_role(Role::CopperTop).unwrap();
    assert_eq!(top.identified.unwrap().source, Source::Attribute);
    let gerber = top.gerber().unwrap();
    assert_eq!(gerber.units, Units::Millimetres);
    assert_eq!(gerber.lines(), 1);
    assert_eq!(gerber.arcs(), 1);
    // two vias and three pads (two round, two rectangular on top: 2 + 2 + 2 vias)
    assert_eq!(gerber.flashes(), 6);
    assert!(trip.imported.by_role(Role::CopperBottom).is_some());
    assert!(trip.imported.by_role(Role::MaskTop).is_some());
    assert!(trip.imported.by_role(Role::PasteTop).is_some());
    assert!(trip.imported.by_role(Role::Outline).is_some());
    let plated = trip.imported.by_role(Role::DrillPlated).unwrap();
    assert_eq!(plated.drill().unwrap().hits.len(), 4);
    let not_plated = trip.imported.by_role(Role::DrillNonPlated).unwrap();
    assert_eq!(not_plated.drill().unwrap().hits.len(), 2);
}

#[test]
fn the_same_board_gives_the_same_result() {
    let b = board();
    let a = round_trip(&b, &Profile::default(), &ImportSettings::default()).unwrap();
    let c = round_trip(&b, &Profile::default(), &ImportSettings::default()).unwrap();
    assert_eq!(a, c);
}

#[test]
fn a_coarser_format_still_agrees_within_the_tolerance() {
    let b = board();
    let mut profile = Profile::default();
    profile.gerber.decimal_digits = 4;
    let trip = round_trip(&b, &profile, &ImportSettings::default()).unwrap();
    assert_eq!(trip.differences, Vec::new());
}

#[test]
fn a_flash_that_is_lost_is_found() {
    let b = board();
    let mut set = files(&b);
    let mut changed = false;
    for file in &mut set {
        if matches!(
            file.role,
            FileRole::Layer {
                kind: LayerKind::Copper,
                ..
            }
        ) && !changed
        {
            let text = String::from_utf8(file.bytes.clone()).unwrap();
            if let Some(position) = text.lines().position(|l| l.ends_with("D03*")) {
                let kept: Vec<&str> = text
                    .lines()
                    .enumerate()
                    .filter(|(i, _)| *i != position)
                    .map(|(_, l)| l)
                    .collect();
                file.bytes = (kept.join("\n") + "\n").into_bytes();
                changed = true;
            }
        }
    }
    assert!(changed);
    let trip = compare_files(&b, &set, &ImportSettings::default());
    assert!(trip
        .differences
        .iter()
        .any(|d| d.what.starts_with("flashes")));
    let findings = trip.difference_findings();
    assert_eq!(findings.len(), trip.differences.len());
    assert!(findings.iter().all(|f| f.code == "PHOENIX-GBR-016"));
}

#[test]
fn a_hole_that_is_lost_and_a_file_that_is_missing_are_found() {
    let b = board();
    let mut set = files(&b);
    for file in &mut set {
        if file.role == (FileRole::Drill { plated: true }) {
            let text = String::from_utf8(file.bytes.clone()).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let after_header = lines.iter().position(|l| *l == "%").unwrap();
            let first_hole = lines
                .iter()
                .enumerate()
                .skip(after_header)
                .find(|(_, l)| l.starts_with('X'))
                .map(|(i, _)| i)
                .unwrap();
            let kept: Vec<&str> = lines
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != first_hole)
                .map(|(_, l)| *l)
                .collect();
            file.bytes = (kept.join("\n") + "\n").into_bytes();
        }
    }
    let trip = compare_files(&b, &set, &ImportSettings::default());
    assert!(trip.differences.iter().any(|d| d.what.starts_with("holes")));
    set.retain(|f| f.role != FileRole::Drill { plated: false });
    let trip = compare_files(&b, &set, &ImportSettings::default());
    assert!(trip
        .differences
        .iter()
        .any(|d| d.what.contains("no file for them")));
}

#[test]
fn a_file_that_cannot_be_read_back_is_a_difference() {
    let b = board();
    let mut set = files(&b);
    for file in &mut set {
        if matches!(
            file.role,
            FileRole::Layer {
                kind: LayerKind::Outline,
                ..
            }
        ) {
            file.bytes = b"not a gerber file".to_vec();
        }
    }
    let trip = compare_files(&b, &set, &ImportSettings::default());
    assert!(trip
        .differences
        .iter()
        .any(|d| d.what.contains("cannot be read back")));
    assert_eq!(trip.imported.failed(), 1);
    assert!(trip
        .imported
        .files
        .iter()
        .any(|f| matches!(f.content, Content::Failed(_))));
}
