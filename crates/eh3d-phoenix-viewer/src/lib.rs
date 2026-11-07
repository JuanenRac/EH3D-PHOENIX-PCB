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
//! * [`snapshot`]: the scene as a deterministic SVG picture, made without a window;
//! * [`session`]: the document that is edited: a board with its rules, its history of edits, the
//!   findings of the rules and the ratsnest, or a schematic with its electrical rule check and the
//!   synchronisation with a board, and the saving of either under another name;
//! * [`workspace`]: the state of editing in the window: the tool, the drag, the first click of a
//!   route, the keyboard shortcuts, undo and redo, the notes of the status bar and the jump to the
//!   place of a finding;
//! * [`schematic_hier`], [`schematic_hier_text`] and [`schematic_bus`]: the tabs of the sheets of a
//!   hierarchical schematic, the way into a sheet symbol and back, the marks of sheet symbols and
//!   buses, and the tools «Bus» and «Bus entry» with their texts in the eight languages (the window is
//!   in `schematic_gui`, behind the `gui` feature);
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
//! * [`board_select`] and [`board_select_text`]: the selection of several objects in the window of the
//!   board editor (the picked set joined with the zones and silkscreen items, the keys that move, line up,
//!   select a net or everything and drop the selection, and the gesture of the rectangle) and its texts in
//!   the eight languages (the use is in `board_gui`, behind the `gui` feature);
//! * [`board_zone`] and [`board_zone_text`]: the zone tool of the board editor (the polygon drawn by
//!   clicks with snapping, its vertices that can be picked and moved, the preview of the fill, the zones
//!   in the order of their priority and the operations that fill, raise, lower and remove them) and its
//!   texts in the eight languages (the section of the panel is in `board_zone_gui`, behind the `gui`
//!   feature);
//! * [`board_shove`] and [`board_shove_text`]: the push tool of the board editor (the net that is
//!   chosen, the points of the new track, the preview of the tracks that the push would move, and
//!   the acceptance of the push as one step of the editor) and its texts in the eight languages
//!   (the section of the panel is in `board_shove_gui`, behind the `gui` feature);
//! * [`board_autoroute`] and [`board_autoroute_text`]: the autorouter in the board editor (the
//!   plan that `eh3d-phoenix-autoroute` makes for the board as it is, kept as a preview that is
//!   dropped when the board changes, and its acceptance as one step of the editor) and its texts in
//!   the eight languages (the section of the panel is in `board_autoroute_gui`, behind the `gui`
//!   feature);
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
//! * [`panel_scene`] and [`panel_text`]: the panel of boards of `eh3d-phoenix-panel` as shapes on
//!   layers that can be hidden (the copies, the frame, the tabs, the V-score lines, the fiducials
//!   and the holes), its measures and the part of its area that the boards take, the session that
//!   reads the boards and lays the panel out again when the matrix or the joint changes, and the
//!   texts of the window in the eight languages (the window is in `panel_gui`, behind the `gui`
//!   feature);
//! * [`print_scene`], [`print_export`] and [`print_text`]: the sheets of a schematic, the assembly
//!   plan and the layers of a board and a panel as pages for `eh3d-phoenix-print`, the export of the
//!   open document to SVG and PDF in a place that is new, and the texts of the button in the eight
//!   languages (the controls are in `print_gui`, behind the `gui` feature);
//! * [`search_index`]: the search of components by reference, value or footprint, nets by name and
//!   pads or pins by `U1.5`, the list of results that the keys walk through and the selection and
//!   the zoom that a result asks for, also over the components of the 3D window;
//! * [`measure_set`]: several measures of distance and angle, the unit of the readings (millimetres
//!   or mils) and the snap of a click to the pads, the ends of the tracks and the corners;
//! * [`layer_presets`]: layer presets saved with a name (which layers are shown, their order and
//!   their opacity), the view of only the active layer and the highlight of the net of the selection;
//! * [`keymap`]: the table from action to keys, read from the settings with the clashes refused,
//!   from which the list of shortcuts of the window is made;
//! * [`history_view`]: the steps that were done and the ones that were undone as a list with a
//!   description of each, and the undos or redos it takes to go to a step;
//! * [`appearance`]: the theme (dark, light or high contrast), the scale of the interface, the size
//!   of the letters and the contrast between two colours;
//! * [`prefs`]: the preferences that the window remembers between runs, read strictly and written
//!   to a side file that is renamed over the old one (the window of these parts is in `window_gui`,
//!   behind the `gui` feature);
//! * [`cfg_edit`], [`cfg_kinds`] and [`cfg_text`]: the editor of the settings files (`config/*.cfg` and
//!   the profiles of the board makers): the settings of a file with their value, default, range and
//!   comment, the change of a value without touching the rest of the text, the validation by the very
//!   readers that the commands use, the range found by offering numbers to those readers, the writing
//!   of the result in one piece and never over a file that was not ordered to be replaced, and its
//!   texts in the eight languages (the window is in `cfg_gui`, behind the `gui` feature);
//! * [`help_text`] and [`window_text`]: the help of each mode and the texts of these parts in the
//!   eight languages;
//! * [`settings`], [`codes`], [`msg`] and [`editmsg`]: the settings file, the codes
//!   `PHOENIX-VIEW-<NNN>` and the texts in the eight languages.
//!
//! The window is behind the `gui` feature (`gui`, egui on eframe) and has not been seen on a
//! screen: the benchmark measures the preparation of the data, not the drawing, and the editing
//! has been tried through the state above only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod appearance;
pub mod bench;
pub mod board_autoroute;
#[cfg(feature = "gui")]
pub mod board_autoroute_gui;
pub mod board_autoroute_text;
#[cfg(feature = "gui")]
pub mod board_gui;
pub mod board_pair;
#[cfg(feature = "gui")]
pub mod board_pair_gui;
pub mod board_pair_text;
pub mod board_select;
pub mod board_select_text;
pub mod board_shove;
#[cfg(feature = "gui")]
pub mod board_shove_gui;
pub mod board_shove_text;
pub mod board_sync;
pub mod board_text;
pub mod board_zone;
#[cfg(feature = "gui")]
pub mod board_zone_gui;
pub mod board_zone_text;
pub mod cfg_edit;
#[cfg(feature = "gui")]
pub mod cfg_gui;
pub mod cfg_kinds;
pub mod cfg_text;
pub mod codes;
#[cfg(feature = "gui")]
pub mod drc_gui;
pub mod drc_panel;
pub mod drc_text;
pub mod editmsg;
#[cfg(feature = "gui")]
pub mod gerber_gui;
pub mod gerber_scene;
pub mod gerber_text;
pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
pub mod help_text;
pub mod history_view;
pub mod keymap;
#[cfg(feature = "gui")]
pub mod launcher_gui;
pub mod launcher_recent;
pub mod launcher_session;
pub mod launcher_text;
pub mod layer_presets;
#[cfg(feature = "gui")]
pub mod library_gui;
pub mod library_session;
pub mod library_text;
pub mod load;
pub mod measure;
pub mod measure_set;
pub mod msg;
#[cfg(feature = "gui")]
pub mod panel_gui;
pub mod panel_scene;
pub mod panel_text;
pub mod prefs;
pub mod prim;
pub mod print_export;
#[cfg(feature = "gui")]
pub mod print_gui;
pub mod print_scene;
pub mod print_text;
#[cfg(test)]
pub(crate) mod sample;
pub mod scene;
pub mod scene3d;
pub mod schematic_bus;
#[cfg(feature = "gui")]
pub mod schematic_bus_gui;
#[cfg(feature = "gui")]
pub mod schematic_gui;
pub mod schematic_hier;
#[cfg(feature = "gui")]
pub mod schematic_hier_gui;
pub mod schematic_hier_text;
pub mod schematic_sync;
pub mod schematic_text;
pub mod search_index;
pub mod session;
pub mod settings;
pub mod shot;
pub mod snapshot;
pub mod view;
pub mod view3d;
#[cfg(feature = "gui")]
pub mod view3d_gui;
pub mod view3d_text;
pub mod viewer;
#[cfg(feature = "gui")]
pub mod window_gui;
pub mod window_text;
pub mod workspace;

pub use codes::{ViewError, REGISTRY};
pub use load::{open, Document};
pub use scene::{Scene, SceneKind};
pub use session::{BoardSession, SchematicSession};
pub use settings::ViewerSettings;
pub use viewer::{Tool, Viewer};
pub use workspace::Workspace;
