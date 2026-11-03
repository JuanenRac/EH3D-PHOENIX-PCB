//! The board and its parts.
//!
//! Every part has a stable id (a UUID v7 made when the part is created, never by a writer) and
//! the parts refer to each other by id: a component to its footprint (and, when there is one, to
//! its schematic symbol), a pad assignment to a component, a pad and a net, a track or a zone to
//! its net and layer. All lengths are integer nanometres inside the checked range and all angles
//! integer micro-degrees. Maps are ordered by id, so the same board always writes the same bytes.

use std::collections::BTreeMap;

use eh3d_phoenix_model::{Arc, Nm, ObjectId, Point, Udeg};
use serde::{Deserialize, Serialize};

/// The identity of any part.
pub type Id = ObjectId;

/// What a layer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// A copper layer.
    Copper,
    /// Silkscreen.
    Silkscreen,
    /// Solder mask openings.
    SolderMask,
    /// Solder paste openings.
    Paste,
    /// The board outline.
    Outline,
    /// Documentation only.
    Documentation,
}

/// A layer of the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// The name shown to the user.
    pub name: String,
    /// What it is for.
    pub kind: LayerKind,
    /// The side of the board the layer is on, when it has one (a silkscreen layer: the side its
    /// Gerber file is for). Not written when it is not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
}

/// An electrical net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Net {
    /// The name, an attribute: renaming never changes the identity.
    pub name: String,
}

/// The shape of a pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum PadShape {
    /// A circle.
    Circle {
        /// Diameter.
        diameter: Nm,
    },
    /// A rectangle.
    Rect {
        /// Width.
        width: Nm,
        /// Height.
        height: Nm,
    },
    /// A rectangle with round ends.
    Oblong {
        /// Width.
        width: Nm,
        /// Height.
        height: Nm,
    },
}

/// A pad of a footprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pad {
    /// The number or name printed on the part (`1`, `A3`).
    pub number: String,
    /// Its shape.
    pub shape: PadShape,
    /// Where it is, relative to the origin of the footprint.
    pub at: Point,
    /// Rotation.
    pub rotation: Udeg,
    /// Drill diameter for a through-hole pad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<Nm>,
    /// The layers it appears on.
    pub layers: Vec<Id>,
}

/// A footprint: a named set of pads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Footprint {
    /// The name.
    pub name: String,
    /// The pads, by id.
    pub pads: BTreeMap<Id, Pad>,
}

/// Which side of the board a component is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
}

/// A placed component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    /// The reference designator (`R1`).
    pub refdes: String,
    /// The value (`10k`).
    pub value: String,
    /// The part of the schematic it was made from (the id of the part), when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<Id>,
    /// Its footprint.
    pub footprint: Id,
    /// Where the origin of the footprint is.
    pub at: Point,
    /// Rotation.
    pub rotation: Udeg,
    /// The side.
    pub side: Side,
}

/// A pad of a placed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PadRef {
    /// The component.
    pub component: Id,
    /// The pad of its footprint.
    pub pad: Id,
}

/// The net a pad belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PadNet {
    /// The component.
    pub component: Id,
    /// The pad of its footprint.
    pub pad: Id,
    /// Its net.
    pub net: Id,
}

impl PadNet {
    /// The pad this assignment is about.
    pub fn pad_ref(&self) -> PadRef {
        PadRef {
            component: self.component,
            pad: self.pad,
        }
    }
}

/// One piece of a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    /// A straight piece.
    Line {
        /// Start.
        start: Point,
        /// End.
        end: Point,
    },
    /// An arc, stored as end points and a signed sweep.
    Arc {
        /// Start.
        start: Point,
        /// End.
        end: Point,
        /// Signed sweep; positive is counter-clockwise.
        sweep: Udeg,
    },
}

impl Segment {
    /// Where the piece starts.
    pub fn start(&self) -> Point {
        match self {
            Segment::Line { start, .. } => *start,
            Segment::Arc { start, .. } => *start,
        }
    }

    /// Where the piece ends.
    pub fn end(&self) -> Point {
        match self {
            Segment::Line { end, .. } => *end,
            Segment::Arc { end, .. } => *end,
        }
    }

    /// The arc of an arc piece, checked to have a shape.
    pub fn to_arc(&self) -> Option<Result<Arc, eh3d_phoenix_model::ModelError>> {
        match self {
            Segment::Arc { start, end, sweep } => Some(Arc::new(*start, *end, *sweep)),
            Segment::Line { .. } => None,
        }
    }
}

/// A connected run of copper of one width on one layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    /// Its net.
    pub net: Id,
    /// Its layer.
    pub layer: Id,
    /// Copper width.
    pub width: Nm,
    /// The pieces, each starting where the one before ends.
    pub path: Vec<Segment>,
}

/// A via.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Via {
    /// Its net.
    pub net: Id,
    /// Where it is.
    pub at: Point,
    /// Drill diameter.
    pub drill: Nm,
    /// Pad diameter.
    pub diameter: Nm,
    /// First layer.
    pub from_layer: Id,
    /// Last layer.
    pub to_layer: Id,
}

/// A copper pour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    /// Its net.
    pub net: Id,
    /// Its layer.
    pub layer: Id,
    /// The outline, a closed polygon (the last point joins the first).
    pub outline: Vec<Point>,
    /// Clearance to other copper.
    pub clearance: Nm,
}

/// The whole board.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// The name.
    pub name: String,
    /// The board outline, a closed polygon.
    pub outline: Vec<Point>,
    /// The layers, by id.
    pub layers: BTreeMap<Id, Layer>,
    /// The copper layers from the top of the board to the bottom. Empty when the order is not
    /// given; a board without it is read as before. The first is the top layer, the last the
    /// bottom layer and the ones between are inner layers (the function of a layer follows from
    /// its place).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copper_stack: Vec<Id>,
    /// The nets, by id.
    pub nets: BTreeMap<Id, Net>,
    /// The footprints, by id.
    pub footprints: BTreeMap<Id, Footprint>,
    /// The placed components, by id.
    pub components: BTreeMap<Id, Component>,
    /// The net of each connected pad, sorted by pad.
    pub pad_nets: Vec<PadNet>,
    /// The tracks, by id.
    pub tracks: BTreeMap<Id, Track>,
    /// The vias, by id.
    pub vias: BTreeMap<Id, Via>,
    /// The zones, by id.
    pub zones: BTreeMap<Id, Zone>,
    /// The text and strokes printed on the silkscreen layers, by id. Not written when empty; a
    /// board made before it existed reads as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub silkscreen: BTreeMap<Id, crate::silk::SilkItem>,
}

/// The place of a copper layer in the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackFunction {
    /// The first layer of the stack.
    Top,
    /// An inner layer.
    Inner,
    /// The last layer of the stack.
    Bottom,
}

impl Board {
    /// The place of copper layer `layer` in the stack: its number from 1 (the top) and its
    /// function. `None` when the stack does not hold it. A stack of one layer is its top layer.
    pub fn stack_position(&self, layer: Id) -> Option<(usize, StackFunction)> {
        let index = self.copper_stack.iter().position(|l| *l == layer)?;
        let function = if index == 0 {
            StackFunction::Top
        } else if index + 1 == self.copper_stack.len() {
            StackFunction::Bottom
        } else {
            StackFunction::Inner
        };
        Some((index + 1, function))
    }

    /// The canonical bytes of the board (`design/board.json`).
    pub fn to_bytes(&self) -> Result<Vec<u8>, eh3d_container::CanonicalError> {
        eh3d_container::to_canonical_file_bytes(self)
    }

    /// Reads a board. Values outside the checked ranges, malformed ids and unknown fields are
    /// errors; whether the references hold is a separate check ([`Board::validate`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Board, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// The net of a pad, if it has one.
    pub fn net_of(&self, pad: PadRef) -> Option<Id> {
        self.pad_nets
            .binary_search_by(|p| p.pad_ref().cmp(&pad))
            .ok()
            .map(|i| self.pad_nets[i].net)
    }
}
