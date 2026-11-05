// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations of the board editor, as the file of operations of `phoenix boardedit` writes
//! them (`{"op": "route", "net": "...", "layer": "...", "points": [...]}`). Each one is turned
//! into the reversible operations of `eh3d-phoenix-board` (one batch, all or nothing) by the
//! planner of this crate, and checked by the design rules before it is kept.

use eh3d_phoenix_board::{Id, PadRef, Side};
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::shape::{MeanderSide, Shape};

/// One operation of the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    /// Places a new component of a footprint that the board has.
    Place {
        /// The id of the component; made by the editor when it is left out.
        #[serde(default)]
        id: Option<Id>,
        /// The reference designator, which no other component has.
        refdes: String,
        /// The value.
        #[serde(default)]
        value: String,
        /// The footprint.
        footprint: Id,
        /// Where the origin of the footprint goes.
        at: Point,
        /// The rotation in micro-degrees.
        #[serde(default)]
        rotation_udeg: i64,
        /// The side; the top when it is left out.
        #[serde(default)]
        side: Option<Side>,
    },
    /// Moves a component, and turns it when a rotation is given.
    Move {
        /// The component.
        id: Id,
        /// The new place.
        at: Point,
        /// The new rotation in micro-degrees; it stays when left out.
        #[serde(default)]
        rotation_udeg: Option<i64>,
        /// Whether the tracks on its pads follow; the setting `follow_tracks` when left out.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Turns a component about its origin.
    Rotate {
        /// The component.
        id: Id,
        /// The angle to add in micro-degrees.
        delta_udeg: i64,
        /// Whether the tracks on its pads follow.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Puts a component on the other side of the board (it is mirrored about the vertical axis).
    Mirror {
        /// The component.
        id: Id,
        /// Whether the tracks on its pads follow.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Locks a component: moving, turning, mirroring and removing it is refused.
    Lock {
        /// The component.
        id: Id,
    },
    /// Unlocks a component.
    Unlock {
        /// The component.
        id: Id,
    },
    /// Removes a component with the nets of its pads.
    Remove {
        /// The component.
        id: Id,
    },
    /// Draws a track through points on one layer, with horizontal and vertical pieces and, with
    /// the diagonal shape, 45-degree pieces.
    Route {
        /// The net.
        net: Id,
        /// The copper layer.
        layer: Id,
        /// The points, at least two.
        points: Vec<Point>,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
        /// The width in nanometres; the width of the net class when left out.
        #[serde(default)]
        width_nm: Option<i64>,
        /// Whether a corner goes first along x; when left out both orders are tried.
        #[serde(default)]
        horizontal_first: Option<bool>,
    },
    /// Adds a via, sized by the rules and the net class.
    Via {
        /// The net.
        net: Id,
        /// Where it goes.
        at: Point,
        /// The first layer; the top copper layer when left out.
        #[serde(default)]
        from_layer: Option<Id>,
        /// The last layer; the bottom copper layer when left out.
        #[serde(default)]
        to_layer: Option<Id>,
    },
    /// Moves the part of a track after a vertex to another layer and puts a via at the vertex.
    ChangeLayer {
        /// The track.
        track: Id,
        /// The vertex, from 1 (the end of the first piece).
        vertex: usize,
        /// The copper layer of the part that moves.
        layer: Id,
    },
    /// Deletes one piece of a track; the rest of the track stays in one or two tracks.
    DeleteSegment {
        /// The track.
        track: Id,
        /// The piece, from 0.
        index: usize,
    },
    /// Deletes a track.
    DeleteTrack {
        /// The track.
        track: Id,
    },
    /// Deletes a via.
    DeleteVia {
        /// The via.
        via: Id,
    },
    /// Deletes every track and via of a net (its zones and the nets of its pads stay).
    DeleteNet {
        /// The net.
        net: Id,
    },
    /// Draws the shortest route between two pads: a straight piece when they are in line, an L
    /// otherwise. A pad that has no net takes the net of the other.
    RouteBetween {
        /// The first pad.
        from: PadRef,
        /// The second pad.
        to: PadRef,
        /// The copper layer.
        layer: Id,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
    },
    /// Pastes a net: gives the pads the net of this name (made when the board has none) and joins
    /// them in the order given, each with the shortest route to the next.
    PasteNet {
        /// The name of the net.
        name: String,
        /// The pads, at least two.
        pads: Vec<PadRef>,
        /// The copper layer.
        layer: Id,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
    },
    /// Adds meanders to a net until its copper length is the target.
    TuneLength {
        /// The net.
        net: Id,
        /// The length asked for in nanometres.
        target_nm: i64,
        /// The track to put the meanders on; the longest straight pieces of the net when left out.
        #[serde(default)]
        track: Option<Id>,
        /// The side of the meanders.
        #[serde(default)]
        side: MeanderSide,
    },
    /// Several operations that are kept or undone as one step.
    Group {
        /// The operations, in order.
        ops: Vec<EditOp>,
    },
    /// An operation that is kept even if it adds a rule error (the report still says so).
    Forced {
        /// The operation.
        with: Box<EditOp>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_operation_is_written_and_read_back_the_same() {
        let text = r#"{"op":"forced","with":{"op":"group","ops":[{"op":"lock","id":"01000000-0000-7000-8000-000000000001"}]}}"#;
        let op: EditOp = serde_json::from_str(text).unwrap();
        let back = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&back).unwrap(), op);
        assert!(matches!(op, EditOp::Forced { .. }));
    }

    #[test]
    fn what_is_left_out_takes_its_default() {
        let text =
            r#"{"op":"tune_length","net":"01000000-0000-7000-8000-000000000001","target_nm":5}"#;
        let op: EditOp = serde_json::from_str(text).unwrap();
        match op {
            EditOp::TuneLength { track, side, .. } => {
                assert_eq!(track, None);
                assert_eq!(side, MeanderSide::Auto);
            }
            other => panic!("{other:?}"),
        }
        assert!(serde_json::from_str::<EditOp>(r#"{"op":"fly"}"#).is_err());
    }
}
