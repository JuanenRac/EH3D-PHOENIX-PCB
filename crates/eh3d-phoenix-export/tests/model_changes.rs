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
fn a_rectangular_pad_at_another_angle_stays_solid_and_is_reported() {
    let b = with_a_rectangular_pad(45_000_000);
    let set = build(&b, &thermal_profile(4, true)).unwrap();
    assert_eq!(regions(layer_text(&set, id(1))), 2);
    assert!(codes_of(&set).contains(&"PHOENIX-EXPORT-022".to_owned()));
    // nothing is reported when the relief is not asked for
    let set = build(&b, &Profile::default()).unwrap();
    assert!(!codes_of(&set).contains(&"PHOENIX-EXPORT-022".to_owned()));
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
