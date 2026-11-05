//! The schematic of the design and its consistency with the board.
//!
//! * [`model`]: symbols with numbered pins, parts (a placed symbol with a reference designator, a
//!   value and, when it is known, the footprint it uses) and nets that list the pins on them, all
//!   linked by stable ids; written as canonical JSON (`design/schematic.json` of a `.phoenixproj`);
//! * the checks of the schematic itself, with the codes `PHOENIX-SCH-<NNN>`;
//! * [`link`]: the consistency between a schematic and a board, with the codes
//!   `PHOENIX-LINK-<NNN>`. A component of the board is linked to the part it was made from by
//!   `Component::symbol`, which holds the id of the part; a pin is linked to a pad by the number.
//!
//! * [`draw`]: sheets, placed parts and wires, what the wires join and how that compares with the net
//!   list (`PHOENIX-SCH-004` to `PHOENIX-SCH-009`).
//!
//! Nothing here reads another program's files.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod draw;
pub mod link;
pub mod marks;
pub mod model;
#[cfg(test)]
pub(crate) mod sample;

pub use draw::{Drawn, Placement, Sheet, Wire};
pub use link::{check_consistency, check_drawn_against_board, LinkReport};
pub use marks::{bus_members, Junction, NetLabel, PowerPort};
pub use model::{Id, NetPin, Part, Pin, PinKind, Schematic, SchematicNet, Symbol};
