// SPDX-License-Identifier: GPL-3.0-or-later
//! Small hand-made documents for the tests of the viewer.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_schematic::{
    NetPin, Part, Pin, Placement, Schematic, SchematicNet, Sheet, Symbol, Wire,
};

/// An id from a small number.
pub fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

/// An id of the schematic from a small number.
pub fn sch_id(n: u16) -> ObjectId {
    id(n)
}

fn nm(mm: f64) -> Nm {
    Nm::new((mm * 1_000_000.0).round() as i64).unwrap()
}

/// A point from millimetres.
pub fn point(x: f64, y: f64) -> Point {
    Point {
        x_nm: nm(x),
        y_nm: nm(y),
    }
}

/// A board of 30 by 20 mm with two copper layers, one component of two pads, two tracks (one an
/// arc), a via, a zone and a silkscreen line: 15 primitives in 9 items.
pub fn board() -> Board {
    let (top, bottom, silk, mask) = (id(1), id(2), id(3), id(4));
    let (gnd, vcc) = (id(10), id(11));
    let (footprint, pad1, pad2, component) = (id(20), id(21), id(22), id(30));
    let mut board = Board {
        name: "sample".to_string(),
        ..Board::default()
    };
    board.outline = vec![
        point(0.0, 0.0),
        point(30.0, 0.0),
        point(30.0, 20.0),
        point(0.0, 20.0),
    ];
    for (layer_id, name, kind, side) in [
        (top, "Top", LayerKind::Copper, Some(Side::Top)),
        (bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
        (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
        (mask, "Mask", LayerKind::SolderMask, None),
    ] {
        board.layers.insert(
            layer_id,
            Layer {
                name: name.to_string(),
                kind,
                side,
            },
        );
    }
    board.copper_stack = vec![top, bottom];
    board.nets.insert(
        gnd,
        Net {
            name: "GND".to_string(),
        },
    );
    board.nets.insert(
        vcc,
        Net {
            name: "VCC".to_string(),
        },
    );
    let mut pads = BTreeMap::new();
    pads.insert(
        pad1,
        Pad {
            number: "1".to_string(),
            shape: PadShape::Rect {
                width: nm(1.6),
                height: nm(1.0),
            },
            at: point(-1.5, 0.0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![top, mask],
        },
    );
    pads.insert(
        pad2,
        Pad {
            number: "2".to_string(),
            shape: PadShape::Circle { diameter: nm(1.6) },
            at: point(1.5, 0.0),
            rotation: Udeg::ZERO,
            drill: Some(nm(0.8)),
            layers: vec![top, bottom, mask],
        },
    );
    board.footprints.insert(
        footprint,
        Footprint {
            name: "FP".to_string(),
            pads,
        },
    );
    board.components.insert(
        component,
        Component {
            refdes: "R1".to_string(),
            value: "10k".to_string(),
            symbol: None,
            footprint,
            at: point(10.0, 10.0),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    board.pad_nets = vec![
        PadNet {
            component,
            pad: pad1,
            net: gnd,
        },
        PadNet {
            component,
            pad: pad2,
            net: vcc,
        },
    ];
    board.tracks.insert(
        id(40),
        Track {
            net: gnd,
            layer: top,
            width: nm(0.25),
            path: vec![Segment::Line {
                start: point(5.0, 5.0),
                end: point(9.0, 5.0),
            }],
        },
    );
    board.tracks.insert(
        id(41),
        Track {
            net: vcc,
            layer: bottom,
            width: nm(0.2),
            path: vec![Segment::Arc {
                start: point(20.0, 10.0),
                end: point(22.0, 12.0),
                sweep: Udeg::new(90_000_000).unwrap(),
            }],
        },
    );
    board.vias.insert(
        id(42),
        Via {
            net: gnd,
            at: point(15.0, 15.0),
            drill: nm(0.3),
            diameter: nm(0.6),
            from_layer: top,
            to_layer: bottom,
        },
    );
    board.zones.insert(
        id(43),
        Zone {
            net: gnd,
            layer: top,
            outline: vec![
                point(2.0, 12.0),
                point(8.0, 12.0),
                point(8.0, 18.0),
                point(2.0, 18.0),
            ],
            clearance: nm(0.3),
        },
    );
    board.silkscreen.insert(
        id(44),
        SilkItem::Stroke {
            layer: silk,
            width: nm(0.15),
            path: vec![Segment::Line {
                start: point(12.0, 3.0),
                end: point(18.0, 3.0),
            }],
        },
    );
    board
}

/// A schematic of two sheets of 100 by 80 mm with two resistors placed on the first one, the net
/// N1 joining a pin of each, and three wires: two that reach those pins through a common bend
/// and one that touches nothing.
pub fn schematic() -> Schematic {
    let (s1, s2, symbol) = (id(1), id(2), id(3));
    let (r1, r2) = (id(10), id(11));
    let (pin1, pin2) = (id(20), id(21));
    let mut s = Schematic {
        name: "sample".to_string(),
        ..Schematic::default()
    };
    for (sheet_id, name) in [(s1, "Sheet 1"), (s2, "Sheet 2")] {
        s.sheets.insert(
            sheet_id,
            Sheet {
                name: name.to_string(),
                width: nm(100.0),
                height: nm(80.0),
            },
        );
    }
    let mut pins = BTreeMap::new();
    pins.insert(
        pin1,
        Pin {
            number: "1".to_string(),
            at: Some(point(-5.0, 0.0)),
        },
    );
    pins.insert(
        pin2,
        Pin {
            number: "2".to_string(),
            at: Some(point(5.0, 0.0)),
        },
    );
    s.symbols.insert(
        symbol,
        Symbol {
            name: "R".to_string(),
            pins,
        },
    );
    for (part_id, refdes, x) in [(r1, "R1", 20.0), (r2, "R2", 60.0)] {
        s.parts.insert(
            part_id,
            Part {
                refdes: refdes.to_string(),
                value: "10k".to_string(),
                symbol,
                footprint: None,
                placement: Some(Placement {
                    sheet: s1,
                    at: point(x, 40.0),
                    rotation: Udeg::ZERO,
                }),
            },
        );
    }
    s.nets.insert(
        id(30),
        SchematicNet {
            name: "N1".to_string(),
            pins: vec![
                NetPin {
                    part: r1,
                    pin: pin2,
                },
                NetPin {
                    part: r2,
                    pin: pin1,
                },
            ],
        },
    );
    for (wire_id, path) in [
        (id(40), vec![point(25.0, 40.0), point(40.0, 40.0)]),
        (id(41), vec![point(40.0, 40.0), point(55.0, 40.0)]),
        (id(42), vec![point(10.0, 10.0), point(20.0, 10.0)]),
    ] {
        s.wires.insert(wire_id, Wire { sheet: s1, path });
    }
    s
}
