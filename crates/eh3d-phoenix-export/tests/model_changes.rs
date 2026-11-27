//! What the export writes for the cutouts of the outline, rounded and not plated pads, the cutouts
//! and the keep-away of a pour, and blind and buried vias. The numbers are worked out by hand in
//! the comments.

use std::collections::BTreeMap;

use eh3d_parser_gerber::{Aperture, Operation, Polarity};
use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_export::{build, verify, FabSet, FileRole, Profile};
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

/// Four copper layers (ids 1 to 4, top to bottom), a top mask (5) and an outline layer (6) on a
/// board of 20 by 10 mm.
fn base() -> Board {
    let mut b = Board {
        name: "model".into(),
        outline: vec![mm(0, 0), mm(20, 0), mm(20, 10), mm(0, 10)],
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
    b
}

fn via(b: &mut Board, n: u16, x: i64, from: u16, to: u16) {
    b.vias.insert(
        id(n),
        Via {
            net: id(7),
            at: mm(x, 2),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(from),
            to_layer: id(to),
        },
    );
}

fn file<'a>(set: &'a FabSet, path: &str) -> &'a str {
    set.files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no file {path}"))
        .text()
}

fn layer_text(set: &FabSet, layer: Id) -> &str {
    set.files
        .iter()
        .find(|f| matches!(f.role, FileRole::Layer { layer: l, .. } if l == layer))
        .unwrap_or_else(|| panic!("no file for the layer"))
        .text()
}

#[test]
fn each_pair_of_layers_that_vias_join_has_a_drill_file() {
    let mut b = base();
    // one through via, two blind vias from layer 1 to 2, one from 1 to 3, one buried from 2 to 3
    via(&mut b, 40, 2, 1, 4);
    via(&mut b, 41, 4, 1, 2);
    via(&mut b, 42, 6, 2, 1);
    via(&mut b, 43, 8, 1, 3);
    via(&mut b, 44, 10, 2, 3);
    let set = build(&b, &Profile::default()).unwrap();
    let spans: Vec<(String, usize, usize, bool)> = set
        .files
        .iter()
        .filter_map(|f| match f.role {
            FileRole::DrillSpan { from, to, buried } => Some((f.path.clone(), from, to, buried)),
            _ => None,
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            ("layers/drill_pth_blind_1-2.drl".to_owned(), 1, 2, false),
            ("layers/drill_pth_blind_1-3.drl".to_owned(), 1, 3, false),
            ("layers/drill_pth_buried_2-3.drl".to_owned(), 2, 3, true),
        ]
    );
    let holes = |path: &str| eh3d_parser_excellon::parse(file(&set, path)).drills.len();
    assert_eq!(holes("layers/drill_pth.drl"), 1);
    assert_eq!(holes("layers/drill_pth_blind_1-2.drl"), 2);
    assert_eq!(holes("layers/drill_pth_blind_1-3.drl"), 1);
    assert_eq!(holes("layers/drill_pth_buried_2-3.drl"), 1);
    assert!(
        file(&set, "layers/drill_pth_blind_1-2.drl").contains("TF.FileFunction,Plated,1,2,Blind")
    );
    assert!(
        file(&set, "layers/drill_pth_buried_2-3.drl").contains("TF.FileFunction,Plated,2,3,Buried")
    );
    assert!(file(&set, "layers/drill_pth.drl").contains("TF.FileFunction,Plated,1,4,PTH"));
    // the files are what the check of the export reads back
    assert!(verify(&b, &Profile::default(), &set).is_empty());
    // the same board gives the same files
    let again = build(&b, &Profile::default()).unwrap();
    assert_eq!(set.files, again.files);
}

#[test]
fn a_board_with_only_through_vias_has_no_file_of_a_span() {
    let mut b = base();
    via(&mut b, 40, 2, 1, 4);
    via(&mut b, 41, 4, 4, 1);
    let set = build(&b, &Profile::default()).unwrap();
    assert!(set
        .files
        .iter()
        .all(|f| !matches!(f.role, FileRole::DrillSpan { .. })));
    // a board without a stack-up takes every via for a through via
    b.copper_stack.clear();
    via(&mut b, 42, 6, 1, 2);
    let set = build(&b, &Profile::default()).unwrap();
    assert!(set
        .files
        .iter()
        .all(|f| !matches!(f.role, FileRole::DrillSpan { .. })));
}

#[test]
fn the_profile_has_the_outline_and_every_cutout_as_a_closed_contour() {
    let mut b = base();
    b.outline_cutouts = vec![vec![mm(5, 3), mm(8, 3), mm(8, 6), mm(5, 6)]];
    let set = build(&b, &Profile::default()).unwrap();
    let outline = eh3d_parser_gerber::parse(layer_text(&set, id(6)));
    let lines = outline
        .commands
        .iter()
        .filter(|c| matches!(c.op, Operation::Line { .. }))
        .count();
    // four lines for the outline and four for the cutout
    assert_eq!(lines, 8);
    assert!(verify(&b, &Profile::default(), &set).is_empty());
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

/// One component at (10, 5) mm with three pads: a rounded SMD pad 2 by 1 mm at x - 4 mm, a plated
/// hole of 0.8 mm in a pad of 1.6 mm at x, and a hole of 2 mm that is not plated at x + 4 mm.
fn with_pads() -> Board {
    let mut b = base();
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
    b
}

fn count(text: &str, wanted: fn(&Operation) -> bool) -> usize {
    eh3d_parser_gerber::parse(text)
        .commands
        .iter()
        .filter(|c| wanted(&c.op))
        .count()
}

#[test]
fn a_rounded_pad_is_a_region_and_a_hole_that_is_not_plated_has_no_copper() {
    let b = with_pads();
    let set = build(&b, &Profile::default()).unwrap();
    let flash: fn(&Operation) -> bool = |o| matches!(o, Operation::Flash { .. });
    let region: fn(&Operation) -> bool = |o| matches!(o, Operation::RegionStart);
    // top copper: the plated pad is flashed, the rounded one is a region, the hole has nothing
    let top = layer_text(&set, id(1));
    assert_eq!(count(top, flash), 1);
    assert_eq!(count(top, region), 1);
    // the top mask opens all three: two flashes and the region of the rounded pad
    let mask = layer_text(&set, id(5));
    assert_eq!(count(mask, flash), 2);
    assert_eq!(count(mask, region), 1);
    // the drill files: the plated hole in one, the hole that is not plated in the other
    let plated = eh3d_parser_excellon::parse(file(&set, "layers/drill_pth.drl"));
    let not_plated = eh3d_parser_excellon::parse(file(&set, "layers/drill_npth.drl"));
    assert_eq!(plated.drills.len(), 1);
    assert_eq!(not_plated.drills.len(), 1);
    assert!((plated.tools.values().next().unwrap().diameter - 0.8).abs() < 1e-9);
    assert!((not_plated.tools.values().next().unwrap().diameter - 2.0).abs() < 1e-9);
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

#[test]
fn a_pad_that_is_not_plated_is_not_plated_whatever_layers_it_is_on() {
    // the same pads, with the hole that is not plated also on the bottom copper layer
    let mut b = with_pads();
    b.footprints
        .get_mut(&id(20))
        .unwrap()
        .pads
        .get_mut(&id(13))
        .unwrap()
        .layers
        .push(id(4));
    let set = build(&b, &Profile::default()).unwrap();
    let bottom = layer_text(&set, id(4));
    assert_eq!(count(bottom, |o| matches!(o, Operation::Flash { .. })), 0);
    let not_plated = eh3d_parser_excellon::parse(file(&set, "layers/drill_npth.drl"));
    assert_eq!(not_plated.drills.len(), 1);
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

/// A pour on the top copper of net N1 with a cutout, a track and a via of net N2 inside it.
fn with_pour() -> Board {
    let mut b = base();
    b.zones.insert(
        id(30),
        Zone {
            net: id(7),
            layer: id(1),
            outline: vec![mm(1, 1), mm(19, 1), mm(19, 9), mm(1, 9)],
            clearance: nm(200_000),
            cutouts: vec![vec![mm(2, 2), mm(4, 2), mm(4, 4), mm(2, 4)]],
        },
    );
    b.tracks.insert(
        id(31),
        Track {
            net: id(8),
            layer: id(1),
            width: nm(250_000),
            path: vec![Segment::Line {
                start: mm(6, 5),
                end: mm(12, 5),
            }],
        },
    );
    b.vias.insert(
        id(32),
        Via {
            net: id(8),
            at: mm(14, 5),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(1),
            to_layer: id(4),
        },
    );
    b
}

#[test]
fn the_cutouts_and_the_keep_away_of_a_pour_are_regions_and_lines_of_clear_polarity() {
    let b = with_pour();
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(1));
    let read = eh3d_parser_gerber::parse(text);
    // the order of the objects: the pour, then the clear ones (the cutout, the track and the via
    // with their keep-away), then dark again for the track and the via themselves
    let mut order: Vec<&str> = Vec::new();
    let mut in_region = false;
    for command in &read.commands {
        match &command.op {
            Operation::RegionStart => {
                in_region = true;
                order.push("region");
            }
            Operation::RegionEnd => in_region = false,
            Operation::Polarity(Polarity::Clear) => order.push("clear"),
            Operation::Polarity(Polarity::Dark) => order.push("dark"),
            Operation::Line { .. } if !in_region => order.push("line"),
            Operation::Flash { .. } => order.push("flash"),
            _ => {}
        }
    }
    assert_eq!(
        order,
        vec!["region", "clear", "region", "line", "flash", "dark", "line", "flash"]
    );
    // the apertures: the track (0.25) and its keep-away (0.25 + 2 * 0.2 = 0.65), the via (0.6)
    // and its keep-away (0.6 + 0.4 = 1.0)
    let mut diameters: Vec<f64> = read
        .apertures
        .values()
        .filter_map(|a| match a {
            Aperture::Circle { diameter, .. } => Some(*diameter),
            _ => None,
        })
        .collect();
    diameters.sort_by(f64::total_cmp);
    let wanted = [0.25, 0.6, 0.65, 1.0];
    assert_eq!(diameters.len(), wanted.len(), "{diameters:?}");
    for (found, expected) in diameters.iter().zip(wanted) {
        assert!((found - expected).abs() < 1e-9, "{diameters:?}");
    }
    // the check of the export does not count what takes copper away
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

#[test]
fn a_pour_with_nothing_to_clear_has_no_change_of_polarity() {
    let mut b = with_pour();
    b.zones.get_mut(&id(30)).unwrap().cutouts.clear();
    b.tracks.clear();
    b.vias.clear();
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(1));
    assert!(!text.contains("LPC") && !text.contains("LPD"), "{text}");
}

#[test]
fn the_file_of_a_span_is_named_from_the_file_of_the_plated_holes() {
    {
        use eh3d_phoenix_export::fab::span_file_name;
        assert_eq!(
            span_file_name("drill_pth.drl", 1, 2, false),
            "drill_pth_blind_1-2.drl"
        );
        assert_eq!(
            span_file_name("drill.drl", 2, 3, true),
            "drill_buried_2-3.drl"
        );
        assert_eq!(span_file_name("holes", 1, 3, false), "holes_blind_1-3");
    }
}

/// The pour with a pad of its own net, round and 1 mm wide, at (16, 7) mm.
fn with_pour_and_a_pad_of_its_net() -> Board {
    let mut b = with_pour();
    b.footprints.insert(
        id(40),
        Footprint {
            name: "ROUND".into(),
            pads: BTreeMap::from([(
                id(41),
                Pad {
                    number: "1".into(),
                    shape: PadShape::Circle {
                        diameter: nm(1_000_000),
                    },
                    at: mm(0, 0),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![id(1)],
                    roundness: 0,
                    plated: true,
                },
            )]),
        },
    );
    b.components.insert(
        id(42),
        Component {
            refdes: "TP1".into(),
            value: "x".into(),
            symbol: None,
            footprint: id(40),
            at: mm(16, 7),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: id(42),
        pad: id(41),
        net: id(7),
    });
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    b
}

fn regions(text: &str) -> usize {
    count(text, |o| matches!(o, Operation::RegionStart))
}

#[test]
fn the_pad_of_the_net_of_a_pour_is_solid_unless_the_profile_asks_for_a_thermal_relief() {
    let b = with_pour_and_a_pad_of_its_net();
    let set = build(&b, &Profile::default()).unwrap();
    // the pour and its cutout
    assert_eq!(regions(layer_text(&set, id(1))), 2);
    let mut profile = Profile::default();
    profile.gerber.zone_thermal_spokes = 4;
    profile.validate().unwrap();
    let set = build(&b, &profile).unwrap();
    let text = layer_text(&set, id(1));
    // four more regions of clear polarity, the gaps between the four spokes
    assert_eq!(regions(text), 6);
    let read = eh3d_parser_gerber::parse(text);
    assert!(read.is_clean());
    // every corner of the gaps is on the ring between the pad (radius 0.5 mm) and 0.25 mm beyond
    // it, and none is inside a spoke (0.25 mm wide, along the axes)
    let mut ring = 0;
    let mut in_region = false;
    for command in &read.commands {
        match &command.op {
            Operation::RegionStart => in_region = true,
            Operation::RegionEnd => in_region = false,
            Operation::Line { to, .. } if in_region => {
                let (dx, dy) = (to.x - 16.0, to.y - 7.0);
                if dx.hypot(dy) < 1.0 {
                    ring += 1;
                    let r = dx.hypot(dy);
                    assert!((0.4999..=0.7501).contains(&r), "{r}");
                    assert!(dx.abs() >= 0.1249 || dy.abs() >= 0.1249);
                }
            }
            _ => {}
        }
    }
    // each of the four gaps has 14 corners
    assert!(ring >= 4 * 13, "{ring}");
    // the clear regions come before the copper is drawn again, and the check of the export does
    // not count what takes copper away
    assert!(verify(&b, &profile, &set).is_empty());
    // the same board and profile give the same file
    let again = build(&b, &profile).unwrap();
    assert_eq!(layer_text(&again, id(1)), text);
}

#[test]
fn a_thermal_relief_asks_for_a_valid_profile() {
    let mut profile = Profile::default();
    profile.gerber.zone_thermal_spokes = 3;
    assert!(profile.validate().is_err());
    profile.gerber.zone_thermal_spokes = 4;
    profile.gerber.zone_thermal_gap_nm = 0;
    assert!(profile.validate().is_err());
    profile.gerber.zone_thermal_gap_nm = 250_000;
    profile.gerber.zone_thermal_spoke_width_nm = 6_000_000;
    assert!(profile.validate().is_err());
}

/// The pour with its pad changed to a rectangle of 1 by 0.6 mm, the component turned by `turn`
/// micro-degrees.
fn with_a_rectangular_pad(turn: i64) -> Board {
    let mut b = with_pour_and_a_pad_of_its_net();
    b.footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(41))
        .unwrap()
        .shape = PadShape::Rect {
        width: nm(1_000_000),
        height: nm(600_000),
    };
    b.components.get_mut(&id(42)).unwrap().rotation = Udeg::new(turn).unwrap();
    b
}

fn thermal_profile(spokes: u32, non_round: bool) -> Profile {
    let mut profile = Profile::default();
    profile.gerber.zone_thermal_spokes = spokes;
    profile.gerber.zone_thermal_non_round = non_round;
    profile.validate().unwrap();
    profile
}

fn codes_of(set: &FabSet) -> Vec<String> {
    set.diagnostics
        .iter()
        .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
        .collect()
}

#[test]
fn a_rectangular_pad_stays_solid_unless_the_profile_asks_for_its_relief() {
    let b = with_a_rectangular_pad(0);
    let solid = build(&b, &Profile::default()).unwrap();
    // the pour and its cutout only
    assert_eq!(regions(layer_text(&solid, id(1))), 2);
    // four spokes without the switch for the pads that are not round: as before
    let same = build(&b, &thermal_profile(4, false)).unwrap();
    assert_eq!(layer_text(&same, id(1)), layer_text(&solid, id(1)));
    // with it: four L shaped gaps
    let profile = thermal_profile(4, true);
    let set = build(&b, &profile).unwrap();
    let text = layer_text(&set, id(1));
    assert_eq!(regions(text), 6);
    assert!(eh3d_parser_gerber::parse(text).is_clean());
    assert!(verify(&b, &profile, &set).is_empty());
    // two spokes: a gap above and a gap below
    let profile = thermal_profile(2, true);
    let set = build(&b, &profile).unwrap();
    assert_eq!(regions(layer_text(&set, id(1))), 4);
    assert!(verify(&b, &profile, &set).is_empty());
}

#[test]
fn two_spokes_also_reach_a_round_pad() {
    let b = with_pour_and_a_pad_of_its_net();
    let set = build(&b, &thermal_profile(2, false)).unwrap();
    assert_eq!(regions(layer_text(&set, id(1))), 4);
}

/// The largest distances from the pad at (16, 7) mm of the corners of the regions near it.
fn reach(text: &str) -> (f64, f64) {
    let read = eh3d_parser_gerber::parse(text);
    let (mut rx, mut ry) = (0.0_f64, 0.0_f64);
    let mut in_region = false;
    for command in &read.commands {
        match &command.op {
            Operation::RegionStart => in_region = true,
            Operation::RegionEnd => in_region = false,
            Operation::Line { to, .. } if in_region => {
                let (dx, dy) = ((to.x - 16.0).abs(), (to.y - 7.0).abs());
                if dx < 1.0 && dy < 1.0 {
                    rx = rx.max(dx);
                    ry = ry.max(dy);
                }
            }
            _ => {}
        }
    }
    (rx, ry)
}

#[test]
fn the_gaps_of_a_rectangular_pad_follow_the_turn_of_its_component() {
    // 1 by 0.6 mm and a gap of 0.25 mm: the gaps reach 0.75 mm along x and 0.55 mm along y
    let profile = thermal_profile(4, true);
    let flat = build(&with_a_rectangular_pad(0), &profile).unwrap();
    let (rx, ry) = reach(layer_text(&flat, id(1)));
    assert!(
        (rx - 0.75).abs() < 1e-6 && (ry - 0.55).abs() < 1e-6,
        "{rx} {ry}"
    );
    // a quarter turn swaps them
    let upright = build(&with_a_rectangular_pad(90_000_000), &profile).unwrap();
    let (rx, ry) = reach(layer_text(&upright, id(1)));
    assert!(
        (rx - 0.55).abs() < 1e-6 && (ry - 0.75).abs() < 1e-6,
        "{rx} {ry}"
    );
}

#[test]
fn an_island_smaller_than_the_minimum_is_left_out_of_the_file() {
    // the pour is 18 by 8 mm = 144 mm2, less a cutout of 2 by 2 mm = 140 mm2
    let b = with_pour();
    let mut profile = Profile::default();
    profile.gerber.zone_island_min_area_nm2 = 140_000_000_000_000;
    profile.validate().unwrap();
    let set = build(&b, &profile).unwrap();
    assert_eq!(regions(layer_text(&set, id(1))), 2);
    assert!(!codes_of(&set).contains(&"PHOENIX-EXPORT-021".to_owned()));
    assert!(verify(&b, &profile, &set).is_empty());
    // one square nanometre more and the whole pour goes, with the clearing that belongs to it
    profile.gerber.zone_island_min_area_nm2 = 140_000_000_000_001;
    let set = build(&b, &profile).unwrap();
    let text = layer_text(&set, id(1));
    assert_eq!(regions(text), 0);
    assert!(!text.contains("LPC"));
    assert!(codes_of(&set).contains(&"PHOENIX-EXPORT-021".to_owned()));
    assert!(verify(&b, &profile, &set).is_empty());
    // the default writes it as before
    let before = build(&b, &Profile::default()).unwrap();
    assert_eq!(regions(layer_text(&before, id(1))), 2);
}

#[test]
fn the_new_options_of_the_pour_ask_for_a_valid_profile() {
    let mut profile = Profile::default();
    profile.gerber.zone_thermal_spokes = 2;
    assert!(profile.validate().is_ok());
    profile.gerber.zone_thermal_spokes = 1;
    assert!(profile.validate().is_err());
    profile.gerber.zone_thermal_spokes = 0;
    profile.gerber.zone_island_min_area_nm2 = -1;
    assert!(profile.validate().is_err());
    profile.gerber.zone_island_min_area_nm2 = 1_000_000_000_000_001;
    assert!(profile.validate().is_err());
    profile.gerber.zone_island_min_area_nm2 = 0;
    assert!(profile.validate().is_ok());
}

/// The pour with a pad of 1 by 0.6 mm whose corners have the given roundness (a radius of
/// `shorter side * roundness / 2000`, so 500 is 0.15 mm).
fn with_a_rounded_pad(roundness: u16) -> Board {
    let mut b = with_a_rectangular_pad(0);
    b.footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(41))
        .unwrap()
        .roundness = roundness;
    b
}

/// The largest sum of the distances along x and along y from the pad at (16, 7) mm to a corner of
/// a region near it.
fn diagonal_reach(text: &str) -> f64 {
    let read = eh3d_parser_gerber::parse(text);
    let mut far = 0.0_f64;
    let mut in_region = false;
    for command in &read.commands {
        match &command.op {
            Operation::RegionStart => in_region = true,
            Operation::RegionEnd => in_region = false,
            Operation::Line { to, .. } if in_region => {
                let (dx, dy) = ((to.x - 16.0).abs(), (to.y - 7.0).abs());
                if dx < 1.0 && dy < 1.0 {
                    far = far.max(dx + dy);
                }
            }
            _ => {}
        }
    }
    far
}

#[test]
fn the_gaps_of_a_pad_with_rounded_corners_follow_its_corners() {
    let profile = thermal_profile(4, true);
    // square corners: the gap reaches the corner (0.75, 0.55) mm of the ring, 0.75 + 0.55 = 1.3
    let sharp = build(&with_a_rounded_pad(0), &profile).unwrap();
    let far = diagonal_reach(layer_text(&sharp, id(1)));
    assert!((far - 1.3).abs() < 1e-5, "{far}");
    // roundness 500 is a radius of 0.15 mm; the ring has corners of 0.15 + 0.25 = 0.4 mm about
    // (0.35, 0.15), and its chord at 45 degrees ends at 0.35 + 0.15 + 0.4 * sqrt(2) = 1.0656854
    let round = build(&with_a_rounded_pad(500), &profile).unwrap();
    let text = layer_text(&round, id(1));
    assert!(eh3d_parser_gerber::parse(text).is_clean());
    let far = diagonal_reach(text);
    assert!((far - 1.0656854).abs() < 1e-5, "{far}");
}

#[test]
fn an_arc_of_the_outline_is_written_as_an_arc_and_the_other_edges_as_lines() {
    use eh3d_phoenix_board::OutlineArc;
    let mut b = base();
    // the edge from (20, 0) to (20, 10) turned counter-clockwise by 180 degrees is a half circle
    // of radius 5 mm about (20, 5) that bulges to the right
    b.outline_arcs = vec![OutlineArc {
        edge: 1,
        sweep: Udeg::new(180_000_000).unwrap(),
    }];
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(6));
    let read = eh3d_parser_gerber::parse(text);
    assert!(read.is_clean());
    let lines = read
        .commands
        .iter()
        .filter(|c| matches!(c.op, Operation::Line { .. }))
        .count();
    assert_eq!(lines, 3);
    let arcs: Vec<_> = read
        .commands
        .iter()
        .filter_map(|c| match &c.op {
            Operation::Arc {
                from,
                to,
                center_offset,
                clockwise,
                ..
            } => Some((from.x, from.y, to.x, to.y, *center_offset, *clockwise)),
            _ => None,
        })
        .collect();
    assert_eq!(arcs.len(), 1);
    let (from_x, from_y, to_x, to_y, offset, clockwise) = arcs[0];
    assert!((from_x - 20.0).abs() < 1e-9 && from_y.abs() < 1e-9);
    assert!((to_x - 20.0).abs() < 1e-9 && (to_y - 10.0).abs() < 1e-9);
    // the centre is (20, 5), which is (0, 5) from the start
    assert!(
        offset.0.abs() < 1e-9 && (offset.1 - 5.0).abs() < 1e-9,
        "{offset:?}"
    );
    assert!(!clockwise);
    // the check of the export counts three lines and one arc
    assert!(verify(&b, &Profile::default(), &set).is_empty());
    // an arc that cannot be used is written as the straight edge
    b.outline_arcs[0].sweep = Udeg::ZERO;
    let set = build(&b, &Profile::default()).unwrap();
    let read = eh3d_parser_gerber::parse(layer_text(&set, id(6)));
    assert!(read
        .commands
        .iter()
        .all(|c| !matches!(c.op, Operation::Arc { .. })));
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

/// The strokes outside regions and the flashes that take copper away (clear polarity).
fn cleared(text: &str) -> (usize, usize) {
    let (mut strokes, mut flashes, mut clear, mut region) = (0, 0, false, false);
    for command in &eh3d_parser_gerber::parse(text).commands {
        match &command.op {
            Operation::Polarity(p) => clear = matches!(p, Polarity::Clear),
            Operation::RegionStart => region = true,
            Operation::RegionEnd => region = false,
            Operation::Line { .. } | Operation::Arc { .. } if clear && !region => strokes += 1,
            Operation::Flash { .. } if clear => flashes += 1,
            _ => {}
        }
    }
    (strokes, flashes)
}

#[test]
fn a_track_that_crosses_a_pour_with_both_ends_outside_it_is_kept_apart() {
    let mut b = with_pour();
    // from one edge of the board to the other, right through the pour of another net
    b.tracks.get_mut(&id(31)).unwrap().path = vec![Segment::Line {
        start: mm(0, 5),
        end: mm(20, 5),
    }];
    let set = build(&b, &Profile::default()).unwrap();
    // the keep-away of the track and of the via: without the first, the track and the pour are
    // one piece of copper in the file
    assert_eq!(cleared(layer_text(&set, id(1))), (1, 1));
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

#[test]
fn what_is_away_from_the_outline_of_a_pour_takes_nothing_from_the_layer() {
    let mut b = with_pour();
    // an L shaped pour: the via at (14, 7) is inside its box and 3 mm from its copper
    let zone = b.zones.get_mut(&id(30)).unwrap();
    zone.cutouts.clear();
    zone.outline = vec![mm(1, 1), mm(19, 1), mm(19, 4), mm(5, 4), mm(5, 9), mm(1, 9)];
    b.tracks.clear();
    b.vias.get_mut(&id(32)).unwrap().at = mm(14, 7);
    // and an arc of another net far from it
    b.tracks.insert(
        id(33),
        Track {
            net: id(8),
            layer: id(1),
            width: nm(250_000),
            path: vec![Segment::Arc {
                start: mm(12, 6),
                end: mm(14, 8),
                sweep: Udeg::new(90_000_000).unwrap(),
            }],
        },
    );
    let set = build(&b, &Profile::default()).unwrap();
    assert_eq!(cleared(layer_text(&set, id(1))), (0, 0));
    // the same arc through the pour is cleared
    b.tracks.get_mut(&id(33)).unwrap().path = vec![Segment::Arc {
        start: mm(8, 2),
        end: mm(10, 4),
        sweep: Udeg::new(90_000_000).unwrap(),
    }];
    let set = build(&b, &Profile::default()).unwrap();
    assert_eq!(cleared(layer_text(&set, id(1))), (1, 0));
}

#[test]
fn a_pour_keeps_away_from_a_hole_that_is_not_plated() {
    let mut b = with_pour_and_a_pad_of_its_net();
    // the pad of the net of the pour becomes a mounting hole of 1 mm on no layer at all
    let pad = b
        .footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(41))
        .unwrap();
    pad.drill = Some(nm(1_000_000));
    pad.plated = false;
    pad.layers.clear();
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(1));
    // the via of the other net and the hole
    assert_eq!(cleared(text).1, 2);
    // the hole of 1 mm and 0.2 mm all round it
    let read = eh3d_parser_gerber::parse(text);
    assert!(read
        .apertures
        .values()
        .any(|a| matches!(a, Aperture::Circle { diameter, .. } if (*diameter - 1.4).abs() < 1e-9)));
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

/// The corners of every region drawn with clear polarity, as (x, y) in millimetres.
fn clear_region_points(text: &str) -> Vec<(f64, f64)> {
    let (mut out, mut clear, mut region) = (Vec::new(), false, false);
    for command in &eh3d_parser_gerber::parse(text).commands {
        match &command.op {
            Operation::Polarity(p) => clear = matches!(p, Polarity::Clear),
            Operation::RegionStart => region = true,
            Operation::RegionEnd => region = false,
            Operation::Line { to, .. } if clear && region => out.push((to.x, to.y)),
            _ => {}
        }
    }
    out
}

/// The leftmost corner of each region drawn with dark polarity, in the order of the file.
fn dark_region_lefts(text: &str) -> Vec<f64> {
    let (mut out, mut clear, mut region) = (Vec::new(), false, false);
    let mut left = f64::MAX;
    for command in &eh3d_parser_gerber::parse(text).commands {
        match &command.op {
            Operation::Polarity(p) => clear = matches!(p, Polarity::Clear),
            Operation::RegionStart => {
                region = true;
                left = f64::MAX;
            }
            Operation::RegionEnd => {
                region = false;
                if !clear {
                    out.push(left);
                }
            }
            Operation::Line { to, .. } if region => left = left.min(to.x),
            _ => {}
        }
    }
    out
}

/// Two pours side by side, of net N1 from x = 1 to 10 mm (drawn first) and of net N2 from 10.4 to
/// 19 mm, both with a clearance of 0.2 mm, and a round pad of 1 mm of net N1 at (9.8, 5).
fn two_pours_and_a_pad_between_them() -> Board {
    let mut b = with_pour_and_a_pad_of_its_net();
    b.tracks.clear();
    b.vias.clear();
    let first = b.zones.get_mut(&id(30)).unwrap();
    first.cutouts.clear();
    first.outline = vec![mm(1, 1), mm(10, 1), mm(10, 9), mm(1, 9)];
    let mut second = first.clone();
    second.net = id(8);
    second.outline = vec![
        Point::from_nm(10_400_000, 1_000_000).unwrap(),
        mm(19, 1),
        mm(19, 9),
        Point::from_nm(10_400_000, 9_000_000).unwrap(),
    ];
    b.zones.insert(id(34), second);
    b.components.get_mut(&id(42)).unwrap().at = Point::from_nm(9_800_000, 5_000_000).unwrap();
    b
}

#[test]
fn the_keep_away_of_one_pour_does_not_cut_the_pour_next_to_it() {
    let b = two_pours_and_a_pad_between_them();
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(1));
    assert!(eh3d_parser_gerber::parse(text).is_clean());
    // the second pour keeps 0.2 mm from the pad, a disc of radius 0.7 mm that reaches back to
    // x = 9.1 mm: written whole it would cut the first pour off its own pad
    assert_eq!(cleared(text), (0, 0));
    let points = clear_region_points(text);
    assert!(!points.is_empty());
    for (x, y) in &points {
        assert!(*x >= 10.4 - 2e-6 && *x <= 10.5 + 5e-3, "{x}");
        assert!((y - 5.0).abs() < 0.71, "{y}");
    }
    assert!(verify(&b, &Profile::default(), &set).is_empty());
    // with the first pour alone nothing is cut to shape
    let mut alone = b.clone();
    alone.zones.remove(&id(30));
    let set = build(&alone, &Profile::default()).unwrap();
    assert_eq!(cleared(layer_text(&set, id(1))), (0, 1));
}

#[test]
fn a_thermal_gap_stays_inside_its_pour() {
    let mut b = two_pours_and_a_pad_between_them();
    // the pour of the other net comes first by its id, so the gaps of the pad of the first pour,
    // which reach x = 10.55 mm, are drawn after it
    let second = b.zones.remove(&id(34)).unwrap();
    b.zones.insert(id(29), second);
    let mut profile = Profile::default();
    profile.gerber.zone_thermal_spokes = 4;
    let set = build(&b, &profile).unwrap();
    let right: Vec<f64> = clear_region_points(layer_text(&set, id(1)))
        .into_iter()
        .map(|p| p.0)
        .filter(|x| *x > 10.0 + 2e-6)
        .collect();
    assert!(right.is_empty(), "{right:?}");
}

#[test]
fn a_pour_inside_the_cutout_of_another_is_drawn_after_it() {
    let mut b = with_pour();
    b.tracks.clear();
    b.vias.clear();
    // the large pour has a hole from 6 to 12 mm and a small pour of another net lies in it, with
    // the smaller id
    b.zones.get_mut(&id(30)).unwrap().cutouts =
        vec![vec![mm(6, 2), mm(12, 2), mm(12, 8), mm(6, 8)]];
    b.zones.insert(
        id(25),
        Zone {
            net: id(8),
            layer: id(1),
            outline: vec![mm(7, 3), mm(11, 3), mm(11, 7), mm(7, 7)],
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    let set = build(&b, &Profile::default()).unwrap();
    // the large pour first, so that its hole takes nothing from the small one
    assert_eq!(dark_region_lefts(layer_text(&set, id(1))), vec![1.0, 7.0]);
    assert!(verify(&b, &Profile::default(), &set).is_empty());
}

#[test]
fn the_pours_stay_from_the_board_edge_when_the_profile_asks_for_it() {
    let b = with_pour();
    let plain = build(&b, &Profile::default()).unwrap();
    let mut profile = Profile::default();
    profile.gerber.zone_edge_clearance_nm = 300_000;
    profile.validate().unwrap();
    let set = build(&b, &profile).unwrap();
    let text = layer_text(&set, id(1));
    // the four edges of the board, drawn clear with a pen of 0.6 mm before the tracks
    let (strokes, flashes) = cleared(layer_text(&plain, id(1)));
    assert_eq!(cleared(text), (strokes + 4, flashes));
    let read = eh3d_parser_gerber::parse(text);
    assert!(read.is_clean());
    assert!(read
        .apertures
        .values()
        .any(|a| matches!(a, Aperture::Circle { diameter, .. } if (*diameter - 0.6).abs() < 1e-9)));
    assert!(verify(&b, &profile, &set).is_empty());
    profile.gerber.zone_edge_clearance_nm = -1;
    assert!(profile.validate().is_err());
}

/// A square pad of 1 mm of net N2 at (16, 7), turned by 45 degrees, in the pour of net N1.
fn with_a_turned_pad() -> Board {
    let mut b = with_a_rectangular_pad(45_000_000);
    b.footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(41))
        .unwrap()
        .shape = PadShape::Rect {
        width: nm(1_000_000),
        height: nm(1_000_000),
    };
    b.pad_nets[0].net = id(8);
    b.tracks.clear();
    b.vias.clear();
    b.zones.get_mut(&id(30)).unwrap().cutouts.clear();
    b
}

#[test]
fn a_turned_pad_is_written_turned_with_its_keep_away_and_its_function() {
    let b = with_a_turned_pad();
    let mut profile = Profile::default();
    profile.gerber.x2.level = eh3d_phoenix_export::profile::X2Level::Aperture;
    let set = build(&b, &profile).unwrap();
    let text = layer_text(&set, id(1));
    let read = eh3d_parser_gerber::parse(text);
    assert!(read.is_clean(), "{:?}", read.diagnostics);
    assert_eq!(count(text, |o| matches!(o, Operation::Flash { .. })), 0);
    // the keep-away: a square of 1.4 mm turned, its corners 0.9899 mm from the centre
    let reach = clear_region_points(text)
        .iter()
        .map(|(x, y)| (x - 16.0).abs().max((y - 7.0).abs()))
        .fold(0.0_f64, f64::max);
    assert!((reach - 0.989_949).abs() < 2e-6, "{reach}");
    // the pad itself has its corners 0.7071 mm from the centre along the axes
    let right = read.commands.iter().any(|c| {
        matches!(&c.op, Operation::Line { to, .. }
            if (to.x - 16.707_107).abs() < 2e-6 && (to.y - 7.0).abs() < 2e-6)
    });
    assert!(right, "{text}");
    // the region of the pad carries the function of a surface mount pad
    assert!(
        text.contains("%TA.AperFunction,SMDPad,CuDef*%\nG36*"),
        "{text}"
    );
    assert!(verify(&b, &profile, &set).is_empty());
}

#[test]
fn a_turned_pad_of_the_net_of_a_pour_gets_its_thermal_relief_turned() {
    let b = with_a_rectangular_pad(45_000_000);
    let set = build(&b, &thermal_profile(4, true)).unwrap();
    // the pour, its cutout, four gaps and the pad itself
    assert_eq!(regions(layer_text(&set, id(1))), 7);
    assert!(codes_of(&set).iter().all(|c| c != "PHOENIX-EXPORT-022"));
    // the rectangle 1 by 0.6 grown by 0.25 has its corners at (0.75, 0.55), which turned by 45
    // degrees are 0.9192 mm from the pad along an axis
    let (rx, ry) = reach(layer_text(&set, id(1)));
    assert!(
        (rx - 0.919_239).abs() < 2e-6 && (ry - 0.919_239).abs() < 2e-6,
        "{rx} {ry}"
    );
}

#[test]
fn a_bottom_pad_takes_its_own_turn_off_the_turn_of_its_part() {
    let mut b = with_a_rectangular_pad(45_000_000);
    b.footprints
        .get_mut(&id(40))
        .unwrap()
        .pads
        .get_mut(&id(41))
        .unwrap()
        .rotation = Udeg::new(45_000_000).unwrap();
    b.components.get_mut(&id(42)).unwrap().side = Side::Bottom;
    b.zones.clear();
    b.tracks.clear();
    b.vias.clear();
    let set = build(&b, &Profile::default()).unwrap();
    let text = layer_text(&set, id(1));
    // 45 - 45 = 0 degrees: a plain rectangle of 1 by 0.6 mm, flashed
    assert_eq!(count(text, |o| matches!(o, Operation::Flash { .. })), 1);
    assert!(eh3d_parser_gerber::parse(text)
        .apertures
        .values()
        .any(|a| matches!(a, Aperture::Rectangle { x, y, .. }
            if (*x - 1.0).abs() < 1e-9 && (*y - 0.6).abs() < 1e-9)));
}

/// What the check of the export says about the files of `written` against the board `model`.
fn places_wrong(model: &Board, written: &Board, profile: &Profile) -> Vec<String> {
    let set = build(written, profile).unwrap();
    verify(model, profile, &set)
        .iter()
        .map(|d| d.message.to_string())
        .collect()
}

#[test]
fn the_check_finds_a_hole_that_is_not_where_the_board_has_it() {
    let mut b = base();
    via(&mut b, 40, 2, 1, 4);
    via(&mut b, 41, 4, 1, 2);
    let profile = Profile::default();
    assert!(places_wrong(&b, &b, &profile).is_empty());
    // the same number of holes, one of them a micrometre away
    let mut moved = b.clone();
    moved.vias.get_mut(&id(40)).unwrap().at = Point::from_nm(2_001_000, 2_000_000).unwrap();
    let found = places_wrong(&b, &moved, &profile);
    assert!(
        found
            .iter()
            .any(|m| m.contains("drill_pth.drl: 1 holes are not where")),
        "{found:?}"
    );
    // another diameter in the file of the blind vias
    let mut wide = b.clone();
    wide.vias.get_mut(&id(41)).unwrap().drill = nm(350_000);
    let found = places_wrong(&b, &wide, &profile);
    assert!(
        found
            .iter()
            .any(|m| m.contains("drill_pth_blind_1-2.drl: 1 holes are not where")),
        "{found:?}"
    );
    // the fixed style keeps three decimals, and half of the last one is no difference
    let mut fixed = Profile::default();
    fixed.drill.style = eh3d_phoenix_export::profile::DrillStyle::Fixed;
    let mut fine = b.clone();
    fine.vias.get_mut(&id(40)).unwrap().at = Point::from_nm(2_000_400, 2_000_000).unwrap();
    let set = build(&fine, &fixed).unwrap();
    assert!(verify(&fine, &fixed, &set)
        .iter()
        .all(|d| !d.message.contains("holes are not where")));
}

#[test]
fn the_check_finds_a_pad_that_is_not_where_the_board_has_it() {
    let b = with_pads();
    let profile = Profile::default();
    assert!(places_wrong(&b, &b, &profile).is_empty());
    let mut moved = b.clone();
    moved
        .footprints
        .get_mut(&id(20))
        .unwrap()
        .pads
        .get_mut(&id(12))
        .unwrap()
        .at = Point::from_nm(10_000, 0).unwrap();
    let found = places_wrong(&b, &moved, &profile);
    assert!(
        found.iter().any(|m| m.contains("1 flashes are not where")),
        "{found:?}"
    );
}

#[test]
fn a_hole_that_is_not_plated_takes_no_paste() {
    let mut b = with_pads();
    b.layers.insert(
        id(9),
        Layer {
            name: "Top paste".into(),
            kind: LayerKind::Paste,
            side: Some(Side::Top),
        },
    );
    for footprint in b.footprints.values_mut() {
        for pad in footprint.pads.values_mut() {
            pad.layers.push(id(9));
        }
    }
    let set = build(&b, &Profile::default()).unwrap();
    // the rounded pad (a region) and the plated pad; nothing for the hole of 2 mm
    let paste = layer_text(&set, id(9));
    assert_eq!(count(paste, |o| matches!(o, Operation::Flash { .. })), 1);
    assert_eq!(regions(paste), 1);
    assert!(verify(&b, &Profile::default(), &set).is_empty());
    // nor is there an opening for it in the stencil
    let mut findings = Vec::new();
    let settings = eh3d_phoenix_export::stencil::StencilSettings::default();
    let openings = eh3d_phoenix_export::stencil::openings(&b, &settings, &mut findings);
    let mut pads: Vec<&str> = openings.iter().map(|o| o.pad.as_str()).collect();
    pads.sort_unstable();
    assert_eq!(pads, ["1", "2"]);
}

#[test]
fn the_placement_origin_is_the_corner_of_the_box_with_its_arcs() {
    use eh3d_phoenix_board::OutlineArc;
    use eh3d_phoenix_export::profile::Origin;
    let mut b = with_pads();
    // the left side bulges out by 5 mm
    b.outline_arcs = vec![OutlineArc {
        edge: 3,
        sweep: Udeg::new(180_000_000).unwrap(),
    }];
    let mut profile = Profile::default();
    profile.placement.origin = Origin::OutlineMin;
    let set = build(&b, &profile).unwrap();
    let rows = eh3d_phoenix_export::assembly::parse_csv(file(&set, "layers/placement.csv"), ',');
    // U1 at (10, 5) mm is 15 mm right of the left end of the board
    assert_eq!(&rows[1][3..5], ["15.0000", "5.0000"]);
}
