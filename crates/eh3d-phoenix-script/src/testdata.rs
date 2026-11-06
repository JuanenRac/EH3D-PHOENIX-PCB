// SPDX-License-Identifier: GPL-3.0-or-later
//! A small board made by hand for the tests: two copper layers, a silkscreen layer, an outline
//! layer, two resistors and a track.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Operation, Pad, PadRef, PadShape,
    Segment, Side, Track,
};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, Point, StepClock, Udeg};

/// The board and the ids of its parts.
pub struct Fixture {
    pub board: Board,
    pub top: Id,
    pub bottom: Id,
    pub silk: Id,
    pub outline: Id,
    pub r1: Id,
    pub r2: Id,
    pub gnd: Id,
    pub vcc: Id,
    pub track: Id,
}

pub fn mm(value: i64) -> Nm {
    Nm::new(value * 1_000_000).unwrap()
}

pub fn at(x_mm: i64, y_mm: i64) -> Point {
    Point {
        x_nm: mm(x_mm),
        y_nm: mm(y_mm),
    }
}

pub fn fixture() -> Fixture {
    let mut ids = IdGenerator::new(
        StepClock::new(1_600_000_000_000, 1),
        DeterministicRandom::new(7),
    );
    let mut next = || ids.generate().unwrap();
    let (top, bottom, silk, outline) = (next(), next(), next(), next());
    let (footprint, pad1, pad2) = (next(), next(), next());
    let (r1, r2, gnd, vcc, track) = (next(), next(), next(), next(), next());

    let layers = BTreeMap::from([
        (
            top,
            Layer {
                name: "Top".to_owned(),
                kind: LayerKind::Copper,
                side: Some(Side::Top),
            },
        ),
        (
            bottom,
            Layer {
                name: "Bottom".to_owned(),
                kind: LayerKind::Copper,
                side: Some(Side::Bottom),
            },
        ),
        (
            silk,
            Layer {
                name: "tPlace".to_owned(),
                kind: LayerKind::Silkscreen,
                side: Some(Side::Top),
            },
        ),
        (
            outline,
            Layer {
                name: "Dimension".to_owned(),
                kind: LayerKind::Outline,
                side: None,
            },
        ),
    ]);
    let pad = |number: &str, x_mm: i64| Pad {
        number: number.to_owned(),
        shape: PadShape::Rect {
            width: Nm::new(800_000).unwrap(),
            height: Nm::new(800_000).unwrap(),
        },
        at: Point {
            x_nm: Nm::new(x_mm * 100_000).unwrap(),
            y_nm: Nm::ZERO,
        },
        rotation: Udeg::ZERO,
        drill: None,
        layers: vec![top],
        roundness: 0,
        plated: true,
    };
    let footprints = BTreeMap::from([(
        footprint,
        Footprint {
            name: "R0603".to_owned(),
            pads: BTreeMap::from([(pad1, pad("1", -8)), (pad2, pad("2", 8))]),
        },
    )]);
    let component = |refdes: &str, value: &str, x_mm: i64, rotation: i64| Component {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol: None,
        footprint,
        at: at(x_mm, 10),
        rotation: Udeg::new(rotation).unwrap(),
        side: Side::Top,
    };
    let components = BTreeMap::from([
        (r1, component("R1", "10k", 10, 0)),
        (r2, component("R2", "1k", 20, 90_000_000)),
    ]);
    let nets = BTreeMap::from([
        (
            gnd,
            Net {
                name: "GND".to_owned(),
            },
        ),
        (
            vcc,
            Net {
                name: "VCC".to_owned(),
            },
        ),
    ]);
    let mut board = Board {
        name: "demo".to_owned(),
        outline: vec![at(0, 0), at(40, 0), at(40, 30), at(0, 30)],
        layers,
        footprints,
        components,
        nets,
        ..Board::default()
    };
    for (component, pad, net) in [(r1, pad1, gnd), (r1, pad2, vcc), (r2, pad1, gnd)] {
        board
            .apply(&Operation::SetPadNet {
                pad: PadRef { component, pad },
                net: Some(net),
            })
            .unwrap();
    }
    board
        .apply(&Operation::SetCopperStack {
            stack: vec![top, bottom],
        })
        .unwrap();
    board
        .apply(&Operation::AddTrack {
            id: track,
            track: Track {
                net: gnd,
                layer: top,
                width: Nm::new(254_000).unwrap(),
                path: vec![Segment::Line {
                    start: at(9, 10),
                    end: at(19, 10),
                }],
            },
        })
        .unwrap();
    Fixture {
        board,
        top,
        bottom,
        silk,
        outline,
        r1,
        r2,
        gnd,
        vcc,
        track,
    }
}
