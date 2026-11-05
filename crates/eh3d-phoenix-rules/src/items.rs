// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper of a board as items that can be measured: tracks, vias and the pads of the placed
//! components.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};

use crate::geometry::{pad_diameters, Prim};

/// A pad of a footprint, placed on the board by its component.
#[derive(Debug, Clone)]
pub(crate) struct PlacedPad {
    /// The reference designator of the component.
    pub refdes: String,
    /// The reference designator of the component and the number of the pad (`R1 pad 2`).
    pub label: String,
    /// The same as a pin of the schematic would be named (`R1.2`).
    pub pin_label: String,
    /// Where the centre of the pad is on the board.
    pub at: Point,
    /// The net it is assigned to, when it has one.
    pub net: Option<Id>,
    /// Its shape.
    pub shape: PadShape,
    /// Its drill, when it is a through-hole pad.
    pub drill: Option<Nm>,
    /// The layers it appears on.
    pub layers: Vec<Id>,
}

/// Every pad of every component that has a footprint, in a fixed order.
pub(crate) fn placed_pads(board: &Board) -> Vec<PlacedPad> {
    let mut out = Vec::new();
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            out.push(PlacedPad {
                refdes: component.refdes.clone(),
                label: format!("{} pad {}", component.refdes, pad.number),
                pin_label: format!("{}.{}", component.refdes, pad.number),
                at,
                net: board.net_of(pad_ref),
                shape: pad.shape,
                drill: pad.drill,
                layers: pad.layers.clone(),
            });
        }
    }
    out
}

/// A track as an item. Arc pieces are measured by their end points, which are really copper: a
/// distance found that way is proven, a distance not found says nothing about the arc.
pub(crate) fn track_prim(track: &Track) -> Option<Prim> {
    let mut pieces = Vec::new();
    let mut has_arc = false;
    for segment in &track.path {
        match segment {
            Segment::Line { start, end } => pieces.push((*start, *end)),
            Segment::Arc { start, end, .. } => {
                has_arc = true;
                pieces.push((*start, *start));
                pieces.push((*end, *end));
            }
        }
    }
    let width = i128::from(track.width.get());
    Prim::new(pieces, width, width, has_arc)
}

/// A via as an item: a round pad at one point.
pub(crate) fn via_prim(via: &Via) -> Option<Prim> {
    let d = i128::from(via.diameter.get());
    Prim::at_point(via.at, d, d)
}

/// A pad as an item: its two readings are the largest circle inside it and the smallest around it.
pub(crate) fn pad_prim(pad: &PlacedPad) -> Option<Prim> {
    let (lo, hi) = pad_diameters(pad.shape);
    Prim::at_point(pad.at, lo, hi)
}
