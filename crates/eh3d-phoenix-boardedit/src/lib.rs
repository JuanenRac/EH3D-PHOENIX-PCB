// SPDX-License-Identifier: GPL-3.0-or-later
//! The board editor without a screen (prototype).
//!
//! The window of the viewer draws this state and passes on what the person does; everything that
//! can be decided without a screen is decided here, in code that is tested:
//!
//! * [`settings`]: the grid, the snapping, the width of a track, the meanders and the other tuned
//!   values, read from `config/boardedit.cfg` (which is also the built-in default);
//! * [`ops`]: the operations ([`EditOp`]): place, move, turn, mirror, lock and remove components;
//!   route orthogonal and 45-degree tracks; add vias; change layer; delete a stretch, a track, a
//!   via or a net; paste a net with the shortest route between two pads; tune a net to a length
//!   with meanders; group several into one step; force one past a rule error;
//! * [`edit`]: the [`BoardEdit`] editor: each operation is planned into the reversible operations
//!   of `eh3d-phoenix-board`, tried on a copy, checked by the design rules (`eh3d-phoenix-rules`)
//!   and kept only if the board accepts it and it adds no rule error (unless forced); undo and
//!   redo give back the same bytes;
//! * [`hit`] and [`tool`]: what lies under a point, the rectangle, and the [`EditState`] of the
//!   tools (select, place, route, via, delete, tune), the drag with the tracks that follow, the
//!   active layer, the lit net and the ratsnest;
//! * [`shape`]: the pieces of a route and the meanders, in integer nanometres;
//! * [`ops_file`]: the file of operations that `phoenix boardedit` applies;
//! * [`pairs`], [`pair_check`] and [`pair_codes`]: the differential pairs: their definition (two
//!   nets, the width, the gap, an optional target for the differential impedance, the tolerance of
//!   the lengths and the longest uncoupled stretch) read from `config/diffpairs.cfg` and from a JSON
//!   file, the operations `route_pair`, `tune_pair` and `fix_pair_gap` (planned in this crate like the
//!   rest, with the design rules checked before and after), the checks of the gap, the symmetry, the
//!   uncoupled stretch, the lengths, the vias, the width and the impedance, and the codes
//!   `PHOENIX-PAIR-<NNN>`;
//! * [`shove`] and `arc`: the push of neighbour tracks when a track is routed (`route_shove`:
//!   tracks of other nets that the clearance would invade are moved in parallel, within a depth and
//!   a number of tracks of the settings, all or nothing) and the arcs in tracks (`route_arc`,
//!   `round_corners`, the arc checked against the clearance of the rules), with the codes
//!   `PHOENIX-SHOVE-<NNN>` and `PHOENIX-ARC-<NNN>` in [`shove_codes`];
//! * [`zone`], [`zone_clip`] and [`zone_codes`]: the copper zones in the editor: the properties of a
//!   zone that the board model does not hold (priority, minimum width and island, connection and
//!   thermal relief, cutouts) and the fills of `eh3d-phoenix-zones` as data of the editor that go
//!   through undo and redo, the operations `zone_add`, `zone_edit`, `zone_remove`, `zone_fill`,
//!   `zone_priority` and `zone_cutout`, the report of zones with no fill or joined to nothing, the
//!   copy and paste of a net or a group with an offset, a turn and a mirror (`paste_group`), the
//!   board with its zones poured for the writers of files and what each format cannot say, with the
//!   codes `PHOENIX-BZONE-<NNN>`;
//! * [`codes`]: the codes `PHOENIX-BEDIT-<NNN>`.
//!
//! **State: prototype.** It has been tried through the tests only, on boards made by hand; the
//! window of the board mode has not been seen on a screen or used with a mouse. A route is made of
//! the pieces it is given (there is no autorouter here: `phoenix autoroute` is a separate
//! command); a meander adds length to a straight piece (horizontal, vertical or at 45 degrees)
//! only; a push moves only straight pieces that are horizontal, vertical or at 45 degrees; the locks
//! live in the editor and are not saved in the board.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
mod advanced_tests;
mod arc;
#[cfg(test)]
mod arc_tests;
pub mod codes;
mod copper;
pub mod edit;
#[cfg(test)]
mod edit_tests;
pub mod hit;
pub mod ids;
pub mod ops;
pub mod ops_file;
pub mod pair_check;
pub mod pair_codes;
mod pair_geom;
mod pair_plan;
#[cfg(test)]
mod pair_tests;
pub mod pairs;
pub mod plan;
pub mod settings;
pub mod shape;
pub mod shove;
pub mod shove_codes;
#[cfg(test)]
mod shove_tests;
#[cfg(test)]
mod testutil;
pub mod tool;
#[cfg(test)]
mod tool_tests;
pub mod zone;
pub mod zone_clip;
pub mod zone_codes;
#[cfg(test)]
mod zone_tests;

pub use codes::{registry, BoardEditError, TITLES};
pub use edit::{BoardEdit, DrcReport, Outcome};
pub use hit::Picked;
pub use ops::{EditOp, PairLayerChange};
pub use ops_file::{apply_operations, read_operations, ApplyReport};
pub use pair_check::{check_pairs, PairFinding, PairMeasure, PairReport};
pub use pairs::{PairConfig, PairDef, PairSettings, ResolvedPair};
pub use settings::BoardEditSettings;
pub use shape::{MeanderSide, Shape};
pub use shove::{ShoveMove, ShovePreview};
pub use tool::{Drag, EditState, Tool};
pub use zone::{
    export_issues, poured_board, validate_props, zone_issues, Target, ZoneChanges, ZoneData,
    ZoneIssue, ZoneProps,
};
pub use zone_clip::{ClipTrack, ClipVia, ClipZone, Clipboard, CLIP_FORMAT};
