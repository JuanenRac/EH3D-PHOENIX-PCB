//! The electrical and board model of the design tool.
//!
//! * [`model`]: layers, nets, footprints and pads, placed components, tracks (lines and arcs),
//!   vias and zones, linked by stable ids and stored in integer nanometres;
//! * [`validate`]: the consistency checks, with diagnostic codes `PHOENIX-BOARD-<NNN>`;
//! * [`connectivity`]: the islands of each net (which copper touches which), with exact integer
//!   arithmetic;
//! * [`rules`]: sizes and clearances against a rule set read from a text file, with confirmed and
//!   inconclusive results kept apart;
//! * [`outline_arc`]: arcs in the outline of the board, kept apart from its corners, and the polygon
//!   of straight pieces that they are cut into;
//! * [`silk`]: text and strokes printed on the silkscreen layers, with a built-in stroke font;
//! * [`stack`]: the checks of the ordered stack of copper layers;
//! * [`ops`]: reversible operations with an exact inverse, and an undo/redo history that can be
//!   saved and replayed.
//!
//! The board is written as canonical JSON (`design/board.json` of a `.phoenixproj`); reading it back
//! gives the same board and writing that gives the same bytes.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod connectivity;
pub mod model;
pub mod ops;
pub mod outline_arc;
pub mod rules;
mod rules_more;
pub mod silk;
pub mod stack;
pub mod validate;

pub use model::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadRef, PadShape, Segment,
    Side, StackFunction, Track, Via, Zone,
};
pub use ops::{History, OpError, Operation, Step};
pub use outline_arc::{ArcSettings, OutlineArc};
pub use silk::{DrawError, Drawn, SilkItem};
