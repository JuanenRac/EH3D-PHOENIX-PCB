// SPDX-License-Identifier: GPL-3.0-or-later
//! The schematic editor without a screen.
//!
//! The window of the viewer draws this state and passes on what the person does; everything that
//! can be decided without a screen is decided here, in code that is tested:
//!
//! * [`settings`]: the grid, the snapping, the tolerances and the other tuned values, read from
//!   `config/schedit.cfg` (which is also the built-in default);
//! * [`hit`]: what lies under a point, what a rectangle takes in, and where a point snaps to;
//! * [`palette`]: the symbols that can be placed, from the symbols of the schematic and from the
//!   libraries that were loaded, with a search;
//! * [`state`]: the [`SchEdit`] state: the tool, the selection, the drag with the wires that follow
//!   the pins, the wire in progress, copy and paste, undo and redo, and the electrical rule check
//!   after each change;
//! * [`transfer`]: the transfer of the schematic to a board with a preview of the package, and the
//!   export of the net list to the simulator;
//! * [`ops_file`]: the file of operations that `phoenix schedit` applies;
//! * [`codes`]: the codes `PHOENIX-SCHEDIT-<NNN>`.
//!
//! The editing itself (the operations, the nets made again from the drawing, undo and redo) is in
//! `eh3d-phoenix-schematic`. Nothing here has been seen on a screen.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod hit;
pub mod ops_file;
pub mod palette;
pub mod settings;
pub mod state;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod testutil;
pub mod transfer;

pub use codes::{SchEditError, REGISTRY};
pub use ops_file::{apply_operations, read_operations, ApplyReport};
pub use palette::{Palette, PaletteEntry};
pub use settings::SchEditSettings;
pub use state::{Drag, SchEdit, Tool};
pub use transfer::{export_netlist, NetlistExport};
