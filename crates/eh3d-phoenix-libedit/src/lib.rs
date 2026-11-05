// SPDX-License-Identifier: GPL-3.0-or-later
//! The editor of EAGLE libraries (a prototype). It holds a library in memory and edits it without
//! a screen:
//!
//! * [`model`]: the library as footprints, symbols and device sets, with the drawing items as
//!   integers and whatever it does not understand kept as the XML it came from;
//! * [`read`] and [`write`]: reading an EAGLE library, writing it, proving that the file reads
//!   back the same, and saving it as a new file with a copy of the original (a file that exists
//!   is never replaced);
//! * [`ops`]: the operations (create, rename and delete symbols, footprints and devices; add,
//!   move, change and delete pads, lines, arcs, texts, pins and holes; openings of the mask and the
//!   paste; gates and the connection of pins to pads), applied all or nothing, with undo and redo;
//! * [`check`]: the checks of the library with the codes `PHOENIX-LIB-001` to `014` and a
//!   deterministic report;
//! * [`footprints`]: the generators of footprints by the method of IPC-7351B, with the values of
//!   `config/footprints.cfg`;
//! * [`plan`]: the plan to replace an item by the master candidate of the sanitizer, which is not
//!   applied without confirmation;
//! * [`settings`], [`geom`] and [`codes`]: the settings file, the small geometry and the codes.
//!
//! It has only been tried on libraries written by hand for its tests; no EAGLE has opened a file
//! written here, and the dimensions of the generators are not checked against a datasheet.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod check;
pub mod codes;
pub mod geom;
pub mod model;
pub mod ops;
pub mod read;
pub mod settings;
pub mod write;

pub use check::{check_library, CheckReport, Finding, Severity};
pub use codes::{LibError, REGISTRY};
pub use model::{Item, Kind, Library};
pub use ops::{apply_all, parse_operations, History, Operation};
pub use read::read_library;
pub use settings::LibSettings;
pub use write::{save_new, to_text, verify_round_trip, Saved};
