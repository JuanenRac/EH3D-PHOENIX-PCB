// SPDX-License-Identifier: GPL-3.0-or-later
//! A small board for the tests: two layers, two resistors of two pads, a net that joins them with
//! one track and a net of ground.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadRef, PadShape, Segment,
    Side, Track,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::RuleContext;
use eh3d_phoenix_rules::RuleValues;

/// An id from a small number.
pub fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

/// The board and the ids of its parts.
pub struct Fixture {
    pub board: Board,
    pub top: Id,
    pub bottom: Id,
    pub sig: Id,
    pub gnd: Id,
    pub footprint: Id,
    pub r1: Id,
    pub r2: Id,
    pub pad1: Id,
    pub pad2: Id,
    pub track: Id,
}

impl Fixture {
    /// A point from millimetres.
    pub fn pt(&self, x_mm: i64, y_mm: i64) -> Point {
        Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
    }

    /// A pad of a component.
    pub fn pad(&self, component: Id, pad: Id) -> PadRef {
        PadRef { component, pad }
    }
}

/// The rules of a board with a clearance of 0.2 mm and a track of at least 0.15 mm.
pub fn strict_rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: Nm::new(200_000).unwrap(),
            min_track_width: Nm::new(150_000).unwrap(),
            ..RuleValues::default()
        },
        classes: Default::default(),
    }
}

/// The board of the tests. R1 is at (10, 10) and R2 at (30, 10); each has pad 1 at -1.5 mm and
/// pad 2 at +1.5 mm. R1.2 and R2.1 are on `sig` and joined by a track on the top layer from (11.5,
/// 10) to (28.5, 10); R1.1 and R2.2 are on `gnd` with no copper.
pub fn fixture() -> Fixture {
    let (top, bottom, silk) = (id(1), id(2), id(3));
    let (sig, gnd) = (id(10), id(11));
    let (footprint, pad1, pad2) = (id(20), id(21), id(22));
    let (r1, r2, track) = (id(30), id(31), id(40));
    let mut board = Board {
        name: "fixture".to_owned(),
        ..Board::default()
    };
    let corner = |x: i64, y: i64| Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap();
    board.outline = vec![corner(0, 0), corner(60, 0), corner(60, 40), corner(0, 40)];
    for (layer, name, kind, side) in [
        (top, "Top", LayerKind::Copper, Some(Side::Top)),
        (bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
        (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
    ] {
        board.layers.insert(
            layer,
            Layer {
                name: name.to_owned(),
                kind,
                side,
            },
        );
    }
    board.copper_stack = vec![top, bottom];
    board.nets.insert(
        sig,
        Net {
            name: "SIG".to_owned(),
        },
    );
    board.nets.insert(
        gnd,
        Net {
            name: "GND".to_owned(),
        },
    );
    let mut pads = BTreeMap::new();
    for (pad, number, x) in [(pad1, "1", -1), (pad2, "2", 1)] {
        pads.insert(
            pad,
            Pad {
                number: number.to_owned(),
                shape: PadShape::Rect {
                    width: Nm::new(1_000_000).unwrap(),
                    height: Nm::new(1_000_000).unwrap(),
                },
                at: Point::from_nm(x * 1_500_000, 0).unwrap(),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![top],
            },
        );
    }
    board.footprints.insert(
        footprint,
        Footprint {
            name: "R".to_owned(),
            pads,
        },
    );
    for (component, refdes, x) in [(r1, "R1", 10), (r2, "R2", 30)] {
        board.components.insert(
            component,
            Component {
                refdes: refdes.to_owned(),
                value: "10k".to_owned(),
                symbol: None,
                footprint,
                at: corner(x, 10),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
    }
    board.pad_nets = vec![
        PadNet {
            component: r1,
            pad: pad1,
            net: gnd,
        },
        PadNet {
            component: r1,
            pad: pad2,
            net: sig,
        },
        PadNet {
            component: r2,
            pad: pad1,
            net: sig,
        },
        PadNet {
            component: r2,
            pad: pad2,
            net: gnd,
        },
    ];
    board.pad_nets.sort_by_key(PadNet::pad_ref);
    board.tracks.insert(
        track,
        Track {
            net: sig,
            layer: top,
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: Point::from_nm(11_500_000, 10_000_000).unwrap(),
                end: Point::from_nm(28_500_000, 10_000_000).unwrap(),
            }],
        },
    );
    Fixture {
        board,
        top,
        bottom,
        sig,
        gnd,
        footprint,
        r1,
        r2,
        pad1,
        pad2,
        track,
    }
}
