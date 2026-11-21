// SPDX-License-Identifier: GPL-3.0-or-later
//! The members, walks and named values of the evaluator for the schematic and the library (the
//! symbol, the device, the gate, the pins and their pads, the segments of a net), the circles and
//! rectangles of the silkscreen, the pieces of a wire that are arcs, the grid and the constants for
//! the shapes of pads, the units of the grid, the layers and the directions of pins. The design is
//! built here, small, and every expected line is fixed.

use std::collections::BTreeMap;

use eh3d_container::ObjectId;
use eh3d_phoenix_board::{
    Board, Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_schematic::{NetPin, Part, Pin, PinKind, Schematic, SchematicNet, Symbol};
use eh3d_phoenix_ulp::{run_source, Support, UlpSettings, World};

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn board() -> Board {
    let (top, net, fp, pad1, pad2, u1) = (id(1), id(2), id(3), id(4), id(5), id(6));
    let mut b = Board {
        name: "gaps".into(),
        ..Board::default()
    };
    b.layers.insert(
        top,
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.copper_stack = vec![top];
    b.nets.insert(net, Net { name: "SIG".into() });
    let mut pads = BTreeMap::new();
    for (pid, number, shape) in [
        (
            pad1,
            "1",
            PadShape::Circle {
                diameter: nm(600_000),
            },
        ),
        (
            pad2,
            "2",
            PadShape::Rect {
                width: nm(600_000),
                height: nm(600_000),
            },
        ),
    ] {
        pads.insert(
            pid,
            Pad {
                number: number.into(),
                shape,
                at: pt(0, 0),
                rotation: Udeg::new(0).unwrap(),
                drill: None,
                layers: vec![top],
                roundness: 0,
                plated: true,
            },
        );
    }
    b.footprints.insert(
        fp,
        Footprint {
            name: "SOT".into(),
            pads,
        },
    );
    b.components.insert(
        u1,
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: fp,
            at: pt(0, 0),
            rotation: Udeg::new(0).unwrap(),
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: u1,
        pad: pad2,
        net,
    });
    b.tracks.insert(
        id(10),
        Track {
            net,
            layer: top,
            width: nm(200_000),
            path: vec![
                Segment::Line {
                    start: pt(0, 0),
                    end: pt(3_000_000, 4_000_000),
                },
                Segment::Arc {
                    start: pt(3_000_000, 4_000_000),
                    end: pt(5_000_000, 2_000_000),
                    sweep: Udeg::new(-90_000_000).unwrap(),
                },
            ],
        },
    );
    // a circle of radius 2 mm around (10, 10) mm: two half turns
    b.silkscreen.insert(
        id(20),
        SilkItem::Stroke {
            layer: top,
            width: nm(150_000),
            path: vec![
                Segment::Arc {
                    start: pt(8_000_000, 10_000_000),
                    end: pt(12_000_000, 10_000_000),
                    sweep: Udeg::new(180_000_000).unwrap(),
                },
                Segment::Arc {
                    start: pt(12_000_000, 10_000_000),
                    end: pt(8_000_000, 10_000_000),
                    sweep: Udeg::new(180_000_000).unwrap(),
                },
            ],
        },
    );
    // a rectangle from (1, 2) to (5, 4) mm
    let corners = [
        pt(1_000_000, 2_000_000),
        pt(5_000_000, 2_000_000),
        pt(5_000_000, 4_000_000),
        pt(1_000_000, 4_000_000),
    ];
    b.silkscreen.insert(
        id(21),
        SilkItem::Stroke {
            layer: top,
            width: nm(100_000),
            path: (0..4)
                .map(|i| Segment::Line {
                    start: corners[i],
                    end: corners[(i + 1) % 4],
                })
                .collect(),
        },
    );
    // a stroke that is neither
    b.silkscreen.insert(
        id(22),
        SilkItem::Stroke {
            layer: top,
            width: nm(100_000),
            path: vec![Segment::Line {
                start: pt(0, 0),
                end: pt(1_000_000, 1_000_000),
            }],
        },
    );
    b
}

fn schematic() -> Schematic {
    let (sym, p1, p2, part) = (id(50), id(51), id(52), id(53));
    let mut s = Schematic {
        name: "gaps".into(),
        ..Schematic::default()
    };
    let mut pins = BTreeMap::new();
    pins.insert(
        p1,
        Pin {
            number: "1".into(),
            at: Some(pt(1_000_000, 2_000_000)),
        },
    );
    pins.insert(
        p2,
        Pin {
            number: "2".into(),
            at: None,
        },
    );
    s.symbols.insert(
        sym,
        Symbol {
            name: "SOT-SYM".into(),
            pins,
        },
    );
    s.pin_kinds.insert(p1, PinKind::Input);
    s.pin_kinds.insert(p2, PinKind::PowerOut);
    s.parts.insert(
        part,
        Part {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: sym,
            footprint: Some(id(3)),
            placement: None,
        },
    );
    s.nets.insert(
        id(60),
        SchematicNet {
            name: "SIG".into(),
            pins: vec![NetPin { part, pin: p1 }, NetPin { part, pin: p2 }],
        },
    );
    s
}

fn out(text: &str) -> String {
    let (b, s) = (board(), schematic());
    let world = World {
        board: Some(&b),
        schematic: Some(&s),
        ..World::default()
    };
    match run_source(text, &UlpSettings::default(), &Support::shipped(), world) {
        Ok(o) => o.output,
        Err(e) => panic!("{e}\n{text}"),
    }
}

#[test]
fn the_constants_have_the_numbers_of_the_language() {
    assert_eq!(
        out("printf(\"%d %d %d %d\\n\", PAD_SHAPE_SQUARE, PAD_SHAPE_ROUND, PAD_SHAPE_OCTAGON, PAD_SHAPE_LONG);"),
        "0 1 2 3\n"
    );
    assert_eq!(
        out("printf(\"%d %d %d %d\\n\", GRID_UNIT_MIC, GRID_UNIT_MM, GRID_UNIT_MIL, GRID_UNIT_INCH);"),
        "0 1 2 3\n"
    );
    assert_eq!(
        out("printf(\"%d %d %d\\n\", LAYER_TOP, LAYER_BOTTOM, LAYER_DIMENSION);"),
        "1 16 20\n"
    );
    assert_eq!(
        out("printf(\"%d %d\\n\", PIN_DIRECTION_IN, PIN_DIRECTION_SUP);"),
        "1 8\n"
    );
}

#[test]
fn the_shape_of_a_pad_is_the_number_of_its_constant() {
    assert_eq!(
        out("board(B) { B.packages(K) { K.pads(P) { printf(\"%s %d %d\\n\", P.name, P.shape, P.shape == PAD_SHAPE_ROUND); } } }"),
        "1 1 1\n2 0 0\n"
    );
}

#[test]
fn a_wire_has_its_run_and_its_arc() {
    assert_eq!(
        out("board(B) { B.wires(W) { printf(\"%d %d %d\\n\", W.dx, W.dy, W.arc.radius); } }"),
        "3000000 4000000 0\n2000000 -2000000 2000000\n"
    );
}

#[test]
fn the_grid_is_the_one_of_the_settings() {
    assert_eq!(
        out("board(B) { printf(\"%d %d\\n\", B.grid.unit, B.grid.distance); }"),
        "1 1000000\n"
    );
    let (b, s) = (board(), schematic());
    let world = World {
        board: Some(&b),
        schematic: Some(&s),
        grid_unit: 2,
        grid_nm: 1_270_000,
    };
    let o = run_source(
        "schematic(S) { printf(\"%d %d\\n\", S.grid.unit, S.grid.distance); }",
        &UlpSettings::default(),
        &Support::shipped(),
        world,
    )
    .unwrap();
    assert_eq!(o.output, "2 1270000\n");
}

#[test]
fn the_device_the_gate_the_symbol_and_the_pins_of_a_part_are_read() {
    let program = "schematic(S) { S.parts(P) { printf(\"%s %s %s\\n\", P.name, P.device.name, P.device.package.name); \
        P.device.gates(G) { printf(\"%s %s\\n\", G.name, G.symbol.name); \
        G.symbol.pins(N) { printf(\"%s %d %d %d\\n\", N.name, N.x, N.y, N.direction); } } } }";
    assert_eq!(
        out(program),
        "U1 SOT-SYM SOT\nG$1 SOT-SYM\n1 1000000 2000000 1\n2 0 0 8\n"
    );
}

#[test]
fn a_part_has_its_gate_and_its_schematic() {
    assert_eq!(
        out("schematic(S) { S.parts(P) { printf(\"%s %s %s\\n\", P.gate.name, P.gate.symbol.name, P.schematic.name); } }"),
        "G$1 SOT-SYM gaps\n"
    );
}

#[test]
fn the_contact_of_a_pin_is_the_pad_with_the_same_number() {
    let program = "schematic(S) { S.parts(P) { P.device.gates(G) { G.symbol.pins(N) { \
        printf(\"%s %s %s\\n\", N.name, N.contact.name, N.contact.element.name); } } } }";
    assert_eq!(out(program), "1 1 U1\n2 2 U1\n");
}

#[test]
fn a_net_has_one_segment_with_its_pin_references() {
    let program = "schematic(S) { S.nets(N) { N.segments(G) { G.pinrefs(R) { \
        printf(\"%s %s %s\\n\", N.name, R.part.name, R.pin.name); } } } }";
    assert_eq!(out(program), "SIG U1 1\nSIG U1 2\n");
}

#[test]
fn a_signal_has_the_contact_references_of_its_pads() {
    let program = "board(B) { B.signals(S) { S.contactrefs(C) { \
        printf(\"%s %s %s\\n\", S.name, C.element.name, C.contact.name); } } }";
    assert_eq!(out(program), "SIG U1 2\n");
}

#[test]
fn the_circles_and_the_rectangles_of_the_silkscreen_are_found_by_their_shape() {
    assert_eq!(
        out("board(B) { B.circles(C) { printf(\"%d %d %d %d\\n\", C.x, C.y, C.radius, C.width); } }"),
        "10000000 10000000 2000000 150000\n"
    );
    assert_eq!(
        out("board(B) { B.rectangles(R) { printf(\"%d %d %d %d %d\\n\", R.x1, R.y1, R.x2, R.y2, R.width); } }"),
        "1000000 2000000 5000000 4000000 100000\n"
    );
}

#[test]
fn what_the_report_says_about_the_new_names_is_what_the_evaluator_does() {
    use eh3d_phoenix_ulp::evaluable::evaluates;
    use eh3d_phoenix_ulp::support::UseKind;
    for name in [
        "gates",
        "pins",
        "segments",
        "pinrefs",
        "contactrefs",
        "circles",
        "rectangles",
        "dx",
        "dy",
        "arc",
        "grid",
        "unit",
        "device",
        "symbol",
        "contact",
    ] {
        assert!(evaluates(UseKind::Member, name), "{name}");
    }
    assert!(evaluates(UseKind::Constant, "PAD_SHAPE_LONG"));
    assert!(evaluates(UseKind::Constant, "LAYER_TOP"));
    assert!(!evaluates(UseKind::Constant, "LAYER_NOT_A_LAYER"));
    let list = Support::shipped();
    for name in [
        "PAD_SHAPE_LONG",
        "GRID_UNIT_MM",
        "LAYER_BOTTOM",
        "PIN_DIRECTION_IN",
    ] {
        assert!(list.lookup(UseKind::Constant, name).is_some(), "{name}");
    }
}
