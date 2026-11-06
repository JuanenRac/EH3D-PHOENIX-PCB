// SPDX-License-Identifier: GPL-3.0-or-later
//! The 3D assembly of a board with the models of its components.
//!
//! * [`step`]: a reader of the least of a STEP file: the box that encloses its solid, in
//!   nanometres, exact for a plain box and honest about what it replaced for anything else;
//! * [`models`]: the table that gives a footprint its model (a STEP file or a box written in the
//!   table), its shift and its turn;
//! * [`scene`]: the assembled board as data: the board with the thickness of its stack-up and
//!   each component as a box on its side of the board (no window, no egui: the viewer of the
//!   suite draws it);
//! * [`check`]: the collisions between the boxes, the height limit and the components that leave
//!   the outline;
//! * [`export`] and [`prism`]: the scene as a simplified OBJ and STEP file, and the board alone as
//!   a STEP solid (the prism of its outline) with its holes through it;
//! * [`enclosure`] and [`enclosure_codes`]: the check of the board, its components and the screw
//!   posts against an enclosure (a box, or an inner contour and a height): clearances to the
//!   walls, the lid and the floor, collisions with the posts, holes of the board under the posts,
//!   and the codes `PHOENIX-ENCL-001` to `014`;
//! * [`report`]: the scene and its checks as text and JSON, the same every time;
//! * [`geom`]: the integer geometry under all of it;
//! * [`settings`]: the thickness, the tolerances and the limits, read from
//!   `config/assembly3d.cfg`;
//! * [`codes`]: the codes `PHOENIX-3D-001` to `PHOENIX-3D-018` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by the tests and on STEP files written
//! by the tests, and no CAD program has opened the files it writes. A component is only its
//! enclosing box, so it finds collisions between boxes and not between the real parts; a STEP file
//! that is not a plain box is replaced by its box and a finding says so; a component without a
//! model is left out and a finding says so, and no model is ever made up.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod check;
pub mod codes;
pub mod enclosure;
pub mod enclosure_codes;
pub mod export;
pub mod geom;
pub mod models;
pub mod prism;
pub mod report;
pub mod scene;
pub mod settings;
pub mod step;

pub use check::{check_scene, reach_nm, Checks, Collision};
pub use codes::{title, AssemblyError, Finding, Severity};
pub use enclosure::{
    board_holes, check_enclosure, enclosure_json, enclosure_text, usable_holes, Cavity, Clearances,
    EnclosureChecks, EnclosureSpec, Inner, Post,
};
pub use export::{export_note, scene_obj, scene_prisms, scene_step};
pub use models::{ModelEntry, ModelSource, ModelTable};
pub use prism::{board_step, Hole, Prism};
pub use report::{all_findings, scene_json, scene_text};
pub use scene::{build_scene, millimetres, Part3d, PartSource, Scene3d};
pub use settings::AssemblySettings;
pub use step::{read_step, SolidKind, StepError, StepSolid};
