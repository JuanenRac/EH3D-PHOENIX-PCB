// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF for boards and panels (a prototype).
//!
//! * [`write`]: the writer of DXF release 12 in ASCII, the same text every time: layers with a
//!   configurable name, an outline as a closed polyline with its arcs as bulges, holes as
//!   circles, marks as points or crosses, texts if asked, lengths in millimetres;
//! * [`read`]: the reader, with limits on the bytes, the entities, the blocks and the depth of
//!   their nesting, that understands lines, arcs, circles, points, light and old polylines with
//!   bulges, splines (cut into straight pieces within a tolerance, with a finding), inserts of
//!   blocks with a scale and a turn, layers and the unit of `$INSUNITS`, and that lists whatever
//!   it does not understand;
//! * [`outline`]: the outline of a board from loose entities: pieces joined end to end within a
//!   tolerance, closed contours, the outer contour and the cutouts found by containment, and the
//!   report of chains that are open, duplicates, contours that cross and several outer contours;
//! * [`spline`] and [`geom`]: the evaluation of splines and the integer geometry under all of it;
//! * [`model`]: the drawing the reader makes and the writer takes;
//! * [`settings`] and [`codes`]: `config/dxf.cfg` and the codes `PHOENIX-DXF-001` to `019` with
//!   their titles in the eight languages.
//!
//! **State: prototype.** It has been tried on files written by the tests and by this crate, and
//! no CAD program has opened a file it writes. An ellipse, a hatch, a dimension, a text of the
//! drawing, a mesh and an array of inserts are not understood and are listed; the cutouts of a
//! drawing are kept by the board as the cutouts of its outline.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod geom;
pub mod model;
pub mod outline;
pub mod read;
pub mod settings;
pub mod spline;
pub mod write;

pub use codes::{registry, title, DxfError, Finding, Report, Severity, TITLES};
pub use geom::{Seg, P};
pub use model::{Drawing, Entity, Shape};
pub use outline::{extract_outline, Outline, OutlineStats};
pub use read::{read_dxf, Read};
pub use settings::{DefaultUnit, DxfSettings};
pub use write::{layer_name, write_dxf};
