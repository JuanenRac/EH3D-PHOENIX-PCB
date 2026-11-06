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
//! * [`edit`]: operations that change the drawing all or nothing (place, move, turn, mirror, delete,
//!   copy and paste, values and references, wires, labels, power ports, bus labels), the nets made
//!   again from the drawing after each one, and undo and redo (`PHOENIX-SCH-010`).
//!
//! * [`hier`] and [`hier_expand`]: sheet symbols that show other sheets (also the same sheet several
//!   times), their pins and the ports of the shown sheet, the instances with hierarchical references
//!   and net names, and the checks `PHOENIX-HIER-<NNN>`;
//! * [`bus`]: buses such as `D[0..7]` and `USB{DP DM}` as objects, with entries, aliases and the checks
//!   `PHOENIX-BUS-<NNN>`.
//!
//! Nothing here reads another program's files.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bus;
pub mod draw;
pub mod edit;
mod edit_ops;
#[cfg(test)]
mod edit_tests;
pub mod hier;
mod hier_edit;
pub mod hier_expand;
#[cfg(test)]
mod hier_tests;
pub mod link;
pub mod marks;
pub mod model;
mod nets;
#[cfg(test)]
pub(crate) mod sample;

pub use bus::{expand_spec, Bus, BusAlias, BusEntry, SpecError};
pub use draw::{Drawn, Placement, Sheet, Wire};
pub use edit::{
    orthogonal_path, Applied, Clip, ClipMark, ClipPart, EditError, EditOp, Editor, IdSource, Item,
    PinDef, SymbolDef,
};
pub use hier::{
    code_title, HierConfig, HierPin, Hierarchy, Instance, PortDir, RefdesStyle, SheetBlock,
    SheetPort,
};
pub use hier_expand::{refdes_in, Expanded, NetRow};
pub use link::{check_consistency, check_drawn_against_board, LinkReport};
pub use marks::{bus_members, Junction, NetLabel, PowerPort};
pub use model::{Id, NetPin, Part, Pin, PinKind, Schematic, SchematicNet, Symbol};
