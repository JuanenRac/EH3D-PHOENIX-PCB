// SPDX-License-Identifier: GPL-3.0-or-later
//! A prototype viewer and editor of boards and schematics.
//!
//! The logic has no screen and is tested without one:
//!
//! * [`prim`]: the drawing primitives (lines, arcs, polygons, circles, text) in millimetres and
//!   the geometry that goes with them (boxes, distance from a point, arcs as short lines, the
//!   triangles of a polygon);
//! * [`scene`]: a board or a schematic turned into primitives on coloured layers, each linked to
//!   its item of the document and to its net;
//! * [`view`], [`measure`] and [`viewer`]: the zoom and the pan, the measure between two points,
//!   the visibility and the colour of the layers, the selection by position with a tolerance and
//!   the highlight of a net;
//! * [`grid`]: the spatial index that makes the selection and the listing of what a window shows
//!   cheap;
//! * [`load`]: reading an EAGLE board or schematic or the JSON of the model;
//! * [`bench`]: three synthetic boards and the time of preparing and picking on them;
//! * [`session`]: the document that is edited: a board with its rules, its history of edits, the
//!   findings of the rules and the ratsnest, or a schematic with its electrical rule check and the
//!   synchronisation with a board, and the saving of either under another name;
//! * [`workspace`]: the state of editing in the window: the tool, the drag, the first click of a
//!   route, the keyboard shortcuts, undo and redo, the notes of the status bar and the jump to the
//!   place of a finding;
//! * [`schematic_sync`] and [`schematic_text`]: how the schematic editor of `eh3d-phoenix-schedit`
//!   hands its schematic to the workspace, and its texts in the eight languages (the window of
//!   the editor is in `schematic_gui`, behind the `gui` feature);
//! * [`library_session`] and [`library_text`]: the library mode: an EAGLE library edited with the
//!   operations of `eh3d-phoenix-libedit` (footprints, symbols and devices, the tools, the grid and
//!   the snap, the selection and the drag, the measure, the checks, the plan to replace by a master
//!   and the saving as a new file) and its texts in the eight languages;
//! * [`settings`], [`codes`], [`msg`] and [`editmsg`]: the settings file, the codes
//!   `PHOENIX-VIEW-<NNN>` and the texts in the eight languages.
//!
//! The window is behind the `gui` feature (`gui`, egui on eframe) and has not been seen on a
//! screen: the benchmark measures the preparation of the data, not the drawing, and the editing
//! has been tried through the state above only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bench;
pub mod codes;
pub mod editmsg;
pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
pub mod library_session;
pub mod library_text;
pub mod load;
pub mod measure;
pub mod msg;
pub mod prim;
#[cfg(test)]
pub(crate) mod sample;
pub mod scene;
#[cfg(feature = "gui")]
pub mod schematic_gui;
pub mod schematic_sync;
pub mod schematic_text;
pub mod session;
pub mod settings;
pub mod view;
pub mod viewer;
pub mod workspace;

pub use codes::{ViewError, REGISTRY};
pub use load::{open, Document};
pub use scene::{Scene, SceneKind};
pub use session::{BoardSession, SchematicSession};
pub use settings::ViewerSettings;
pub use viewer::{Tool, Viewer};
pub use workspace::Workspace;
