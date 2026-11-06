//! The round trip: the files that the export writes for a board are read back by this import and
//! compared with the board; a file that is changed afterwards is found.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_export::profile::{X2Level, X2Profile};
use eh3d_phoenix_export::{build, FabFile, FileRole, Profile};
use eh3d_phoenix_gerber::x2check::{
    check_text, has_errors, END_COMMAND, RECOMMENDED_MISSING, REQUIRED_MISSING,
};
use eh3d_phoenix_gerber::{
    compare_files, compare_files_with, round_trip, Content, ImportSettings, Role, Source, Units,
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
        roundness: 0,
        plated: true,
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
            cutouts: Vec::new(),
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

fn with_level(level: X2Level) -> Profile {
    let mut profile = Profile::default();
    profile.gerber.x2.level = level;
    profile
}

fn texts(set: &[FabFile]) -> Vec<(String, String)> {
    set.iter()
        .filter(|f| matches!(f.role, FileRole::Layer { .. } | FileRole::Drill { .. }))
        .map(|f| (f.folder_name().to_owned(), f.text().to_owned()))
        .collect()
}

#[test]
fn every_level_of_x2_reads_back_as_the_board() {
    let b = board();
    for level in [
        X2Level::Legacy,
        X2Level::X1,
        X2Level::File,
        X2Level::Aperture,
        X2Level::Full,
    ] {
        let trip = round_trip(&b, &with_level(level), &ImportSettings::default()).unwrap();
        assert_eq!(trip.differences, Vec::new(), "{level:?}");
        assert_eq!(trip.files, 7, "{level:?}");
        assert_eq!(trip.imported.failed(), 0, "{level:?}");
    }
}

#[test]
fn the_attributes_of_apertures_and_objects_are_read_back() {
    let b = board();
    let trip = round_trip(&b, &with_level(X2Level::Full), &ImportSettings::default()).unwrap();
    let top = trip.imported.by_role(Role::CopperTop).unwrap();
    let gerber = top.gerber().unwrap();
    for number in gerber.apertures.keys() {
        assert!(
            gerber.aperture_attributes[number].contains_key(".AperFunction"),
            "D{number}"
        );
    }
    assert!(gerber.attribute_sets[0].is_empty());
    assert_eq!(gerber.item_sets.len(), gerber.items.len());
    // the first pad: the pin, the component and the net of the pad
    let pins: Vec<(String, String)> = (0..gerber.items.len())
        .filter_map(|i| {
            let set = gerber.object_attributes(i)?;
            let pin = set.get(".P")?;
            Some((pin[0].clone(), pin[1].clone()))
        })
        .collect();
    assert_eq!(pins.len(), 4);
    assert!(pins.contains(&("R1".to_owned(), "1".to_owned())));
    assert!(pins.contains(&("R2".to_owned(), "2".to_owned())));
    let with_net = (0..gerber.items.len())
        .filter(|i| {
            gerber
                .object_attributes(*i)
                .is_some_and(|s| s.contains_key(".N"))
        })
        .count();
    assert_eq!(with_net, gerber.items.len());
    // the plated drill file names the function of each tool
    let drill = trip
        .imported
        .by_role(Role::DrillPlated)
        .unwrap()
        .drill()
        .unwrap();
    assert_eq!(drill.tool_attributes.len(), drill.tools.len());
    let file_attributes = &gerber.attributes;
    assert!(file_attributes.contains_key(".ProjectId"));
    assert_eq!(file_attributes[".Part"], vec!["Single".to_owned()]);
}

#[test]
fn x1_files_have_no_attribute() {
    let b = board();
    let trip = round_trip(&b, &with_level(X2Level::X1), &ImportSettings::default()).unwrap();
    assert_eq!(trip.differences, Vec::new());
    assert_eq!(trip.imported.failed(), 0);
    for file in &trip.imported.files {
        if let Some(gerber) = file.gerber() {
            assert!(gerber.attributes.is_empty(), "{}", file.name);
            assert!(gerber.aperture_attributes.is_empty(), "{}", file.name);
            assert_eq!(gerber.attribute_sets.len(), 1, "{}", file.name);
        }
        if let Some(drill) = file.drill() {
            assert!(drill.attributes.is_empty() && drill.tool_attributes.is_empty());
        }
    }
}

#[test]
fn a_changed_attribute_is_found() {
    let b = board();
    let profile = with_level(X2Level::Full);
    let settings = ImportSettings::default();
    let set = build(&b, &profile).unwrap().files;
    let changed = |from: &str, to: &str| -> Vec<FabFile> {
        set.iter()
            .map(|f| {
                let mut f = f.clone();
                if f.folder_name().ends_with("top_copper.gbr") {
                    f.bytes = f.text().replace(from, to).into_bytes();
                }
                f
            })
            .collect()
    };
    let what = |files: &[FabFile], expect: &X2Profile| -> Vec<String> {
        compare_files_with(&b, files, &settings, expect)
            .differences
            .into_iter()
            .map(|d| d.what)
            .collect()
    };
    let x2 = &profile.gerber.x2;
    assert_eq!(what(&set, x2), Vec::<String>::new());
    // a net that is not the net of the board
    let found = what(&changed("%TO.N,N*%", "%TO.N,OTHER*%"), x2);
    assert!(found.iter().any(|w| w.contains("nets of the")), "{found:?}");
    // a pin that is not a pad of the board
    let found = what(&changed("%TO.P,R1,1*%", "%TO.P,R9,1*%"), x2);
    assert!(
        found.iter().any(|w| w.contains("pin R9,1 is not a pad")),
        "{found:?}"
    );
    // a component that is not the one of the pin
    let found = what(&changed("%TO.C,R1*%", "%TO.C,R5*%"), x2);
    assert!(found.iter().any(|w| w.contains("component")), "{found:?}");
    // an aperture without its function
    let found = what(&changed("%TA.AperFunction,Conductor*%\n", ""), x2);
    assert!(
        found.iter().any(|w| w.contains("has no function")),
        "{found:?}"
    );
    // the files are a level that the profile did not ask for
    let found = what(&set, &with_level(X2Level::X1).gerber.x2);
    assert!(found.iter().any(|w| w.contains("level is X1")), "{found:?}");
    let plain = build(&b, &with_level(X2Level::Legacy)).unwrap().files;
    let found = what(&plain, x2);
    assert!(
        found.iter().any(|w| w.contains(".ProjectId is missing")),
        "{found:?}"
    );
    assert!(found.iter().any(|w| w.contains("has no function")));
}

#[test]
fn net_names_with_escapes_still_read_back_as_the_board() {
    let mut b = board();
    b.nets.get_mut(&id(6)).unwrap().name = "A,B 100%".into();
    let trip = round_trip(&b, &with_level(X2Level::Full), &ImportSettings::default()).unwrap();
    assert_eq!(trip.differences, Vec::new());
    let set = build(&b, &with_level(X2Level::Full)).unwrap().files;
    let top = texts(&set)
        .into_iter()
        .find(|(name, _)| name.ends_with("top_copper.gbr"))
        .unwrap()
        .1;
    assert!(top.contains("%TO.N,A\\u002CB 100\\u0025*%"), "{top}");
}

#[test]
fn the_check_finds_nothing_in_the_files_of_the_full_level() {
    let b = board();
    let set = build(&b, &with_level(X2Level::Full)).unwrap().files;
    let files = texts(&set);
    assert_eq!(files.len(), 7);
    for (name, text) in &files {
        assert_eq!(check_text(text), Vec::new(), "{name}");
    }
}

#[test]
fn the_check_tells_the_levels_apart() {
    let b = board();
    // the default level has the three attributes: no error, and the part is recommended
    let legacy = build(&b, &with_level(X2Level::Legacy)).unwrap().files;
    for (name, text) in texts(&legacy) {
        let found = check_text(&text);
        assert!(!has_errors(&found), "{name}: {found:?}");
        assert!(
            found.iter().any(|f| f.code == RECOMMENDED_MISSING),
            "{name}"
        );
    }
    // plain X1 has no file function
    let x1 = build(&b, &with_level(X2Level::X1)).unwrap().files;
    for (name, text) in texts(&x1) {
        let found = check_text(&text);
        assert!(found.iter().any(|f| f.code == REQUIRED_MISSING), "{name}");
    }
    // a file cut short
    let full = build(&b, &with_level(X2Level::Full)).unwrap().files;
    let (_, top) = texts(&full)
        .into_iter()
        .find(|(name, _)| name.ends_with("top_copper.gbr"))
        .unwrap();
    let cut = top.replace("M02*\n", "");
    assert!(check_text(&cut).iter().any(|f| f.code == END_COMMAND));
}
