// SPDX-License-Identifier: GPL-3.0-or-later
//! The export to Specctra DSN and the import of a Specctra session on boards made by hand: the
//! text is the same every time, the new copper is checked against the rules before it is
//! accepted, a name the board does not have refuses the whole session, and what is already on the
//! board is not added twice.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::codes::{
    DSN_EXPORT_FAILED, NOT_CONNECTED, RULES_REFUSE, SES_MISMATCH, SES_UNREADABLE,
};
use eh3d_phoenix_route::{export_dsn, import_ses, RouteSettings, RuleContext};
use eh3d_phoenix_rules::{NetClass, NetClasses, RuleValues};

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A 20 mm board with two copper layers; net A has the pads U1.1 at (2, 2) mm and U1.2 at (6, 6)
/// mm and, when `obstacle` is true, net B has the pad U1.3 at (4, 4) mm between them.
fn board(obstacle: bool) -> Board {
    let mut b = Board::default();
    for (n, name) in [(1_u32, "Top"), (2, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack.push(id(n));
    }
    b.outline = vec![
        pt(0, 0),
        pt(20_000_000, 0),
        pt(20_000_000, 20_000_000),
        pt(0, 20_000_000),
    ];
    b.nets.insert(id(10), Net { name: "A".into() });
    b.nets.insert(id(11), Net { name: "B".into() });
    let mut places = vec![(2_000_000, 2_000_000, 10), (6_000_000, 6_000_000, 10)];
    if obstacle {
        places.push((4_000_000, 4_000_000, 11));
    }
    let mut pads = BTreeMap::new();
    let mut pad_nets = Vec::new();
    for (i, (x, y, net)) in places.iter().enumerate() {
        let pid = id(100 + i as u32);
        pads.insert(
            pid,
            Pad {
                number: (i + 1).to_string(),
                shape: PadShape::Circle {
                    diameter: nm(600_000),
                },
                at: pt(*x, *y),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![id(1)],
                roundness: 0,
                plated: true,
            },
        );
        pad_nets.push(PadNet {
            component: id(43),
            pad: pid,
            net: id(*net),
        });
    }
    b.footprints.insert(
        id(40),
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    b.components.insert(
        id(43),
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: id(40),
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    pad_nets.sort_by_key(|n| n.pad_ref());
    b.pad_nets = pad_nets;
    b
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_track_width: nm(150_000),
            min_clearance: nm(200_000),
            ..RuleValues::default()
        },
        classes: NetClasses::default(),
    }
}

fn ses(body: &str) -> String {
    format!(
        "(session s\n  (base_design d)\n  (routes\n    (resolution um 1000)\n    (parser (host_cad x))\n    (library_out)\n    (network_out\n{body}\n    )\n  )\n)\n"
    )
}

/// The session of a route from U1.1 to U1.2 by two vias and a piece on the bottom layer.
const ROUTE_WITH_VIAS: &str = "
(net A
  (wire (path Top 250000 2000000 2000000 4000000 2000000) (type route))
  (via Via_600000_300000_nm 4000000 2000000)
  (wire (path Bottom 250000 4000000 2000000 4000000 6000000) (type route))
  (via Via_600000_300000_nm 4000000 6000000)
  (wire (path Top 250000 4000000 6000000 6000000 6000000) (type route))
)";

fn ids() -> impl FnMut() -> Id {
    let mut n = 5000_u32;
    move || {
        n += 1;
        id(n)
    }
}

#[test]
fn the_cutouts_of_a_zone_are_keep_out_areas_of_its_layer_and_the_file_says_so() {
    let mut b = board(false);
    b.zones.insert(
        id(60),
        eh3d_phoenix_board::Zone {
            net: id(10),
            layer: id(2),
            outline: vec![
                pt(1_000_000, 1_000_000),
                pt(15_000_000, 1_000_000),
                pt(15_000_000, 15_000_000),
                pt(1_000_000, 15_000_000),
            ],
            clearance: nm(200_000),
            cutouts: vec![
                vec![
                    pt(8_000_000, 8_000_000),
                    pt(10_000_000, 8_000_000),
                    pt(10_000_000, 10_000_000),
                ],
                vec![pt(3_000_000, 3_000_000), pt(4_000_000, 3_000_000)],
            ],
        },
    );
    let dsn = export_dsn(&b, &rules(), &RouteSettings::default()).unwrap();
    assert_eq!(dsn.zone_cutouts, 1);
    // on the layer of the zone, closed back to its first point
    assert!(
        dsn.text.contains(
            "(keepout \"zone_cutout1\" (polygon Bottom 0 8000000 8000000 10000000 8000000 10000000 10000000 8000000 8000000))"
        ),
        "{}",
        dsn.text
    );
    assert_eq!(dsn.text.matches("zone_cutout").count(), 1);
    assert_eq!(dsn.text.matches('(').count(), dsn.text.matches(')').count());
    // nothing is dropped without a word: the keep-out is a simplification and the short ring is left out
    assert_eq!(dsn.notes.len(), 2);
    assert!(dsn.notes[0].contains("keep-out areas"));
    assert!(dsn.notes[1].contains("fewer than three points"));
    // a board without cutouts has no notes and no keep-out of a zone
    let plain = export_dsn(&board(false), &rules(), &RouteSettings::default()).unwrap();
    assert!(plain.notes.is_empty());
    assert_eq!(plain.zone_cutouts, 0);
}

#[test]
fn the_export_is_the_same_every_time_and_describes_the_board() {
    let b = board(false);
    let first = export_dsn(&b, &rules(), &RouteSettings::default()).unwrap();
    let second = export_dsn(&b, &rules(), &RouteSettings::default()).unwrap();
    assert_eq!(first, second);
    let text = &first.text;
    assert!(text.starts_with("(pcb \"board\"\n"), "{text}");
    assert_eq!(text.matches('(').count(), text.matches(')').count());
    for line in [
        "(resolution um 1000)",
        "(unit um)",
        "(layer Top",
        "(layer Bottom",
        "(boundary (path pcb 0 0 0 20000000 0 20000000 20000000 0 20000000 0 0))",
        "(via Via_600000_300000_nm)",
        "(rule (width 250000) (clearance 201000))",
        "(place U1 0 0 front 0)",
        "(pin Pad_1 1 2000000 2000000)",
        "(pin Pad_1 2 6000000 6000000)",
        "(shape (circle Top 600000 0 0))",
        "(pins U1-1 U1-2)",
        "(class eh3d_default A",
        "(circuit (use_via Via_600000_300000_nm))",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
    assert_eq!(
        (
            first.layers,
            first.components,
            first.pads,
            first.nets,
            first.padstacks,
            first.tracks,
            first.vias
        ),
        (2, 1, 2, 1, 2, 0, 0)
    );
}

#[test]
fn the_net_classes_give_the_width_the_clearance_and_the_via_of_their_nets() {
    let mut ctx = rules();
    ctx.classes = NetClasses::new(vec![NetClass {
        name: "power".into(),
        nets: vec!["B".into()],
        min_track_width: nm(400_000),
        min_clearance: nm(300_000),
        min_via_diameter: nm(900_000),
        min_via_drill: nm(400_000),
    }])
    .unwrap();
    let dsn = export_dsn(&board(true), &ctx, &RouteSettings::default()).unwrap();
    for line in [
        "(class power B",
        "(rule (width 400000) (clearance 301000))",
        "(circuit (use_via Via_900000_400000_nm))",
        "(via Via_900000_400000_nm)",
        "(class eh3d_default A",
    ] {
        assert!(dsn.text.contains(line), "{line}\n{}", dsn.text);
    }
}

#[test]
fn the_copper_that_exists_is_written_as_fixed_wiring() {
    let mut b = board(false);
    b.tracks.insert(
        id(200),
        Track {
            net: id(10),
            layer: id(1),
            width: nm(250_000),
            path: vec![Segment::Line {
                start: pt(2_000_000, 2_000_000),
                end: pt(2_000_000, 6_000_000),
            }],
        },
    );
    b.vias.insert(
        id(201),
        Via {
            net: id(10),
            at: pt(2_000_000, 6_000_000),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(1),
            to_layer: id(2),
        },
    );
    let dsn = export_dsn(&b, &rules(), &RouteSettings::default()).unwrap();
    assert!(dsn
        .text
        .contains("(wire (path Top 250000 2000000 2000000 2000000 6000000) (net A) (type fix))"));
    assert!(dsn
        .text
        .contains("(via Via_600000_300000_nm 2000000 6000000 (net A) (type fix))"));
    assert_eq!((dsn.tracks, dsn.vias), (1, 1));
}

#[test]
fn a_board_the_file_cannot_hold_is_refused_with_a_code() {
    let settings = RouteSettings::default();
    let mut no_outline = board(false);
    no_outline.outline.clear();
    let mut arc = board(false);
    arc.tracks.insert(
        id(200),
        Track {
            net: id(10),
            layer: id(1),
            width: nm(250_000),
            path: vec![Segment::Arc {
                start: pt(2_000_000, 2_000_000),
                end: pt(4_000_000, 4_000_000),
                sweep: Udeg::new(90_000_000).unwrap(),
            }],
        },
    );
    let mut same_names = board(true);
    same_names.nets.get_mut(&id(11)).unwrap().name = "A".into();
    let mut quote = board(false);
    quote.nets.get_mut(&id(10)).unwrap().name = "A\"B".into();
    let mut no_copper = board(false);
    no_copper.copper_stack.clear();
    no_copper.layers.clear();
    for (name, bad) in [
        ("no outline", no_outline),
        ("arc", arc),
        ("same net names", same_names),
        ("quote in a name", quote),
        ("no copper", no_copper),
    ] {
        let e = export_dsn(&bad, &rules(), &settings).unwrap_err();
        assert_eq!(e.code, DSN_EXPORT_FAILED, "{name}");
    }
}

#[test]
fn the_wire_of_a_session_is_added_and_closes_the_connection() {
    let b = board(false);
    let text = ses("(net A (wire (path Top 250000 2000000 2000000 6000000 6000000) (type route)))");
    let plan = import_ses(&b, &rules(), &RouteSettings::default(), &text, &mut ids()).unwrap();
    assert_eq!((plan.tracks.len(), plan.vias.len()), (1, 0));
    assert_eq!((plan.unrouted_before, plan.unrouted_after), (1, 0));
    assert_eq!(plan.already_there, 0);
    assert_eq!(plan.length_nm, 5_656_854);
    assert!(!plan.diff.adds_error());
    assert_eq!(plan.nets, vec![("A".to_owned(), 1, 0, 5_656_854)]);
    assert!(plan.require_complete().is_ok());
    // The board that was read is not changed; a copy gets the copper.
    assert!(b.tracks.is_empty());
    let mut routed = b.clone();
    plan.apply(&mut routed).unwrap();
    assert_eq!(routed.tracks.len(), 1);
    let track = routed.tracks.values().next().unwrap();
    assert_eq!(
        (track.net, track.layer, track.width),
        (id(10), id(1), nm(250_000))
    );
    let report = plan.to_text();
    assert!(report.starts_with("session routes: 1 tracks, 0 vias, 5656854 nm of copper\n"));
    assert!(report.contains("connections to route: 1 before, 0 after"));
    assert!(plan.to_json().starts_with("{\"tracks\":1,\"vias\":0,"));
}

#[test]
fn the_units_of_the_session_are_converted() {
    let b = board(false);
    let text = "(session s (routes (resolution mm 1000) (network_out \
        (net A (wire (path Top 250 2000 2000 6000 2000))))))";
    let plan = import_ses(&b, &rules(), &RouteSettings::default(), text, &mut ids()).unwrap();
    let (_, track) = &plan.tracks[0];
    assert_eq!(track.width, nm(250_000));
    assert_eq!(track.path[0].start(), pt(2_000_000, 2_000_000));
    assert_eq!(track.path[0].end(), pt(6_000_000, 2_000_000));
}

#[test]
fn a_session_that_breaks_the_rules_is_refused_whole() {
    let b = board(true);
    let text = ses(
        "(net A (wire (path Top 250000 2000000 2000000 2000000 3000000)) \
         (wire (path Top 250000 2000000 2000000 6000000 6000000)))",
    );
    let e = import_ses(&b, &rules(), &RouteSettings::default(), &text, &mut ids()).unwrap_err();
    assert_eq!(e.code, RULES_REFUSE);
    assert!(e.detail.contains("PHOENIX-"), "{}", e.detail);
}

#[test]
fn a_name_the_board_does_not_have_refuses_the_whole_session() {
    let b = board(false);
    let good = "(wire (path Top 250000 2000000 2000000 2000000 3000000))";
    for (what, body) in [
        (
            "net",
            "(net Z (wire (path Top 250000 1000000 1000000 2000000 1000000)))".to_owned(),
        ),
        (
            "layer",
            format!("(net A {good} (wire (path Inner 250000 1000000 1000000 2000000 1000000)))"),
        ),
        (
            "via",
            format!("(net A {good} (via Via_1_2_um 3000000 3000000))"),
        ),
        (
            "shape",
            format!("(net A {good} (wire (qarc Top 250000 1 1 2 2 3 3)))"),
        ),
        (
            "no length",
            format!("(net A {good} (wire (path Top 250000 1000000 1000000 1000000 1000000)))"),
        ),
        (
            "no width",
            format!("(net A {good} (wire (path Top 0 1000000 1000000 2000000 1000000)))"),
        ),
    ] {
        let e = import_ses(
            &b,
            &rules(),
            &RouteSettings::default(),
            &ses(&body),
            &mut ids(),
        )
        .unwrap_err();
        assert_eq!(e.code, SES_MISMATCH, "{what}");
    }
    for (what, text) in [
        ("not a session", "(pcb x)".to_owned()),
        (
            "odd coordinates",
            ses("(net A (wire (path Top 250000 1 2 3)))"),
        ),
        (
            "not a number",
            ses("(net A (wire (path Top 250000 1 2 x 4)))"),
        ),
        ("cut text", "(session s (routes".to_owned()),
    ] {
        let e = import_ses(&b, &rules(), &RouteSettings::default(), &text, &mut ids()).unwrap_err();
        assert_eq!(e.code, SES_UNREADABLE, "{what}");
    }
}

#[test]
fn vias_and_a_second_layer_are_added_and_a_repeat_adds_nothing() {
    let b = board(false);
    let settings = RouteSettings::default();
    let plan = import_ses(&b, &rules(), &settings, &ses(ROUTE_WITH_VIAS), &mut ids()).unwrap();
    assert_eq!((plan.tracks.len(), plan.vias.len()), (3, 2));
    assert_eq!((plan.unrouted_before, plan.unrouted_after), (1, 0));
    assert_eq!(plan.length_nm, 8_000_000);
    assert_eq!(plan.nets, vec![("A".to_owned(), 3, 2, 8_000_000)]);
    for (_, via) in &plan.vias {
        assert_eq!((via.from_layer, via.to_layer), (id(1), id(2)));
        assert_eq!((via.drill, via.diameter), (nm(300_000), nm(600_000)));
    }
    let mut routed = b.clone();
    plan.apply(&mut routed).unwrap();
    assert_eq!((routed.tracks.len(), routed.vias.len()), (3, 2));

    // The same session again on the routed board, with one wire written the other way round.
    let again = ses(&ROUTE_WITH_VIAS.replace(
        "(path Top 250000 4000000 6000000 6000000 6000000)",
        "(path Top 250000 6000000 6000000 4000000 6000000)",
    ));
    let second = import_ses(&routed, &rules(), &settings, &again, &mut ids()).unwrap();
    assert_eq!((second.tracks.len(), second.vias.len()), (0, 0));
    assert_eq!(second.already_there, 5);
    assert_eq!((second.unrouted_before, second.unrouted_after), (0, 0));
}

#[test]
fn the_same_session_gives_the_same_plan() {
    let b = board(false);
    let settings = RouteSettings::default();
    let one = import_ses(&b, &rules(), &settings, &ses(ROUTE_WITH_VIAS), &mut ids()).unwrap();
    let two = import_ses(&b, &rules(), &settings, &ses(ROUTE_WITH_VIAS), &mut ids()).unwrap();
    assert_eq!(one, two);
    assert_eq!(one.to_text(), two.to_text());
    assert_eq!(one.to_json(), two.to_json());
}

#[test]
fn a_plan_that_leaves_connections_unrouted_can_be_refused() {
    let b = board(true);
    let text = ses("(net A (wire (path Top 250000 2000000 2000000 2000000 6000000)))");
    let plan = import_ses(&b, &rules(), &RouteSettings::default(), &text, &mut ids()).unwrap();
    assert_eq!((plan.unrouted_before, plan.unrouted_after), (1, 1));
    assert_eq!(plan.require_complete().unwrap_err().code, NOT_CONNECTED);
}
