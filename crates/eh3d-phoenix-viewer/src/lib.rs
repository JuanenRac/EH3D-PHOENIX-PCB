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
//! * [`board_sync`] and [`board_text`]: how the board editor of `eh3d-phoenix-boardedit` hands its
//!   board to the workspace, and its texts in the eight languages (the window of the editor is in
//!   `board_gui`, behind the `gui` feature);
//! * [`board_pair`] and [`board_pair_text`]: the pair tool of the board editor (the pair that is
//!   chosen, the points of its centre line, the drawing of the pair, the matching of its lengths, the
//!   correction of its gap and the report of the checks of the pairs) and its texts in the eight
//!   languages (the section of the panel is in `board_pair_gui`, behind the `gui` feature);
//! * [`board_shove`] and [`board_shove_text`]: the push tool of the board editor (the net that is
//!   chosen, the points of the new track, the preview of the tracks that the push would move, and
//!   the acceptance of the push as one step of the editor) and its texts in the eight languages
//!   (the section of the panel is in `board_shove_gui`, behind the `gui` feature);
//! * [`library_session`] and [`library_text`]: the library mode: an EAGLE library edited with the
//!   operations of `eh3d-phoenix-libedit` (footprints, symbols and devices, the tools, the grid and
//!   the snap, the selection and the drag, the measure, the checks, the plan to replace by a master
//!   and the saving as a new file) and its texts in the eight languages (the window is in
//!   `library_gui`, behind the `gui` feature);
//! * [`launcher_session`], [`launcher_recent`] and [`launcher_text`]: the start screen of the window
//!   (what a typed path, a dropped file or a recent file asks for, the list of recent files kept in a
//!   file of the settings folder of the user, and its texts in the eight languages; the drawing is in
//!   `launcher_gui`, behind the `gui` feature);
//! * [`scene3d`]: the 3D scene of the assembled board (the board with the thickness of its
//!   stack-up and each component as a box, from `eh3d-phoenix-assembly3d`) as data in millimetres:
//!   the box that holds it all and the outlines on the board plane, with no window;
//! * [`gerber_scene`] and [`gerber_text`]: the Gerber and Excellon files of a folder as layers of
//!   filled shapes (one layer for each file, coloured by role, with the polarity kept), the
//!   visibility, the order and the opacity of the layers, the snap of the measure and the
//!   comparison of a layer with the same layer of a board of the project, and its texts in the
//!   eight languages (the window is in `gerber_gui`, behind the `gui` feature);
//! * [`view3d`] and [`view3d_text`]: the camera that orbits the assembled board, the perspective
//!   projection worked out by hand, the faces sorted from far to near, the selection under a
//!   click and the marks of the collisions and of the heights, and the texts of the 3D window in
//!   the eight languages (the window is in `view3d_gui`, behind the `gui` feature);
//! * [`settings`], [`codes`], [`msg`] and [`editmsg`]: the settings file, the codes
//!   `PHOENIX-VIEW-<NNN>` and the texts in the eight languages.
//!
//! The window is behind the `gui` feature (`gui`, egui on eframe) and has not been seen on a
//! screen: the benchmark measures the preparation of the data, not the drawing, and the editing
//! has been tried through the state above only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bench;
#[cfg(feature = "gui")]
pub mod board_gui;
pub mod board_pair;
#[cfg(feature = "gui")]
pub mod board_pair_gui;
pub mod board_pair_text;
pub mod board_shove;
#[cfg(feature = "gui")]
pub mod board_shove_gui;
pub mod board_shove_text;
pub mod board_sync;
pub mod board_text;
pub mod codes;
pub mod editmsg;
#[cfg(feature = "gui")]
pub mod gerber_gui;
pub mod gerber_scene;
pub mod gerber_text;
pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub mod launcher_gui;
pub mod launcher_recent;
pub mod launcher_session;
pub mod launcher_text;
#[cfg(feature = "gui")]
pub mod library_gui;
pub mod library_session;
pub mod library_text;
pub mod load;
pub mod measure;
pub mod msg;
pub mod prim;
#[cfg(test)]
pub(crate) mod sample;
pub mod scene;
pub mod scene3d;
#[cfg(feature = "gui")]
pub mod schematic_gui;
pub mod schematic_sync;
pub mod schematic_text;
pub mod session;
pub mod settings;
pub mod view;
pub mod view3d;
#[cfg(feature = "gui")]
pub mod view3d_gui;
pub mod view3d_text;
pub mod viewer;
pub mod workspace;

pub use codes::{ViewError, REGISTRY};
pub use load::{open, Document};
pub use scene::{Scene, SceneKind};
pub use session::{BoardSession, SchematicSession};
pub use settings::ViewerSettings;
pub use viewer::{Tool, Viewer};
pub use workspace::Workspace;
