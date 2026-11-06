// SPDX-License-Identifier: GPL-3.0-or-later
//! Small boards for the tests, built by hand: a rectangle of two layers (or one), nets, round pads
//! on the top layer or through the board, and tracks that are in the way.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadRef, PadShape, Segment,
    Side, Track,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::RuleContext;
use eh3d_phoenix_rules::RuleValues;

use crate::config::AutorouteConfig;

/// An id from a small number.
pub(crate) fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

/// Millimetres in nanometres.
pub(crate) fn mm(v: i64) -> i64 {
    v * 1_000_000
}

/// A point from nanometres.
pub(crate) fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// The board of a test and the ids it needs.
pub(crate) struct Fix {
    pub board: Board,
    pub top: Id,
    pub bottom: Id,
    smd: Id,
    smd_bottom: Id,
    thru: Id,
    next: u32,
}

impl Fix {
    /// A rectangle of `width` by `height` millimetres from the origin, with two copper layers
    /// (`Top`, `Bottom`) or only the first.
    pub(crate) fn new(width: i64, height: i64, two_layers: bool) -> Fix {
        let (top, bottom, smd, thru) = (id(1), id(2), id(10), id(11));
        let smd_bottom = id(14);
        let mut board = Board {
            name: "test".to_owned(),
            ..Board::default()
        };
        board.outline = vec![
            pt(0, 0),
            pt(mm(width), 0),
            pt(mm(width), mm(height)),
            pt(0, mm(height)),
        ];
        board.layers.insert(
            top,
            Layer {
                name: "Top".to_owned(),
                kind: LayerKind::Copper,
                side: Some(Side::Top),
            },
        );
        board.copper_stack = vec![top];
        let mut thru_layers = vec![top];
        if two_layers {
            board.layers.insert(
                bottom,
                Layer {
                    name: "Bottom".to_owned(),
                    kind: LayerKind::Copper,
                    side: Some(Side::Bottom),
                },
            );
            board.copper_stack.push(bottom);
            thru_layers.push(bottom);
        }
        let round = PadShape::Circle {
            diameter: Nm::new(600_000).unwrap(),
        };
        let mut kinds = vec![
            (smd, id(12), vec![top], None),
            (thru, id(13), thru_layers, Some(Nm::new(300_000).unwrap())),
        ];
        if two_layers {
            kinds.push((smd_bottom, id(15), vec![bottom], None));
        }
        for (footprint, pad_id, layers, drill) in kinds {
            let mut pads = BTreeMap::new();
            pads.insert(
                pad_id,
                Pad {
                    number: "1".to_owned(),
                    shape: round,
                    at: pt(0, 0),
                    rotation: Udeg::ZERO,
                    drill,
                    layers,
                    roundness: 0,
                    plated: true,
                },
            );
            board.footprints.insert(
                footprint,
                Footprint {
                    name: "P".to_owned(),
                    pads,
                },
            );
        }
        Fix {
            board,
            top,
            bottom,
            smd,
            smd_bottom,
            thru,
            next: 1000,
        }
    }

    fn fresh(&mut self) -> Id {
        self.next += 1;
        id(self.next)
    }

    /// A net.
    pub(crate) fn net(&mut self, name: &str) -> Id {
        let net = self.fresh();
        self.board.nets.insert(
            net,
            Net {
                name: name.to_owned(),
            },
        );
        net
    }

    /// A round pad of 0.6 mm on a net, at a place in nanometres: on the top layer, or through the
    /// board with a drill of 0.3 mm.
    pub(crate) fn pad(&mut self, net: Id, x: i64, y: i64, through: bool) -> PadRef {
        let (footprint, pad) = if through {
            (self.thru, id(13))
        } else {
            (self.smd, id(12))
        };
        self.place(net, (x, y), footprint, pad)
    }

    /// A round pad of 0.6 mm on a net that is on the bottom layer only (a board of two layers).
    pub(crate) fn pad_bottom(&mut self, net: Id, x: i64, y: i64) -> PadRef {
        let (footprint, pad) = (self.smd_bottom, id(15));
        self.place(net, (x, y), footprint, pad)
    }

    fn place(&mut self, net: Id, at: (i64, i64), footprint: Id, pad: Id) -> PadRef {
        let component = self.fresh();
        let refdes = format!("P{}", self.next);
        self.board.components.insert(
            component,
            Component {
                refdes,
                value: "x".to_owned(),
                symbol: None,
                footprint,
                at: pt(at.0, at.1),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        let pad_ref = PadRef { component, pad };
        self.board.pad_nets.push(PadNet {
            component,
            pad,
            net,
        });
        self.board.pad_nets.sort_by_key(PadNet::pad_ref);
        pad_ref
    }

    /// A straight track of 0.25 mm that is already on the board.
    pub(crate) fn wall(&mut self, net: Id, layer: Id, from: (i64, i64), to: (i64, i64)) {
        let track = self.fresh();
        self.board.tracks.insert(
            track,
            Track {
                net,
                layer,
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(from.0, from.1),
                    end: pt(to.0, to.1),
                }],
            },
        );
    }
}

/// The rules of the tests: a clearance of 0.2 mm, tracks of at least 0.15 mm and 0.25 mm from the
/// edge of the board.
pub(crate) fn ctx() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: Nm::new(200_000).unwrap(),
            min_track_width: Nm::new(150_000).unwrap(),
            min_edge_clearance: Nm::new(250_000).unwrap(),
            ..RuleValues::default()
        },
        classes: Default::default(),
    }
}

/// The settings of the tests: a grid of 0.5 mm.
pub(crate) fn cfg() -> AutorouteConfig {
    AutorouteConfig {
        grid_nm: 500_000,
        ..AutorouteConfig::default()
    }
}
