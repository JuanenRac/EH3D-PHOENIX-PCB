// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only import of Gerber RS-274X and Excellon files, for review.
//!
//! * [`parse`]: the Gerber reader (apertures `C`, `R`, `O`, `P` and macros, polarity, regions,
//!   lines, flashes and arcs, the file attributes);
//! * [`core_adapter`]: the first step of the move to the shared reader of EH3D-CORE-LIBS, an
//!   adapter to the model of this crate with the comparison of both readings (the reader of
//!   [`parse`] is not replaced yet);
//! * [`macros`]: the aperture macros, with their expressions and basic primitives;
//! * [`drill`]: the Excellon reader (tools, units, zeros, holes, slots and routed paths);
//! * [`identify`]: the layer a file belongs to, from its X2 attributes or its name (EAGLE,
//!   Protel and KiCad);
//! * [`odb`]: a basic reader of the matrix file of an ODB++ job (the steps and the layers), strict
//!   and with an ordered refusal of whatever it does not read;
//! * [`import`]: a set of files read together, each kept with its error when it cannot be read;
//! * [`compare`]: the round trip, the files that the export writes for a board read back and
//!   compared with that board;
//! * [`report`]: the summary as text and as JSON;
//! * [`settings`]: the limits and defaults, read from `config/gerber_import.cfg`;
//! * [`x2check`]: a basic check of the structure of Gerber X2 and Excellon files, with the stable
//!   codes `PHOENIX-GX2-<NNN>` (the command `phoenix gerber-check`);
//! * [`codes`]: the stable codes `PHOENIX-GBR-<NNN>` with their titles in the eight languages.
//!
//! The geometry is kept apart from the board model: nothing is merged into a board and nothing is
//! written. Coordinates are integer nanometres; a coordinate that fits the declared format is
//! exact. Not read: step and repeat, aperture blocks and transforms, thermal and moire macro
//! primitives, incremental coordinates and arcs in drill routes.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod compare;
pub mod core_adapter;
pub mod drill;
pub mod identify;
pub mod import;
pub mod macros;
pub mod model;
pub mod odb;
#[cfg(test)]
mod odb_tests;
pub mod parse;
pub mod report;
pub mod settings;
pub mod units;
pub mod x2check;

pub use codes::{title, Finding, GbrError, Severity, TITLES};
pub use core_adapter::{adapt_gerber, compare_layers, Adapted, CoreNote, Equivalence};
pub use compare::{compare_files, compare_files_with, round_trip, Difference, RoundTrip};
pub use drill::parse_drill;
pub use identify::{drill_span, identify, DrillSpan, Identified, Role, Source};
pub use import::{import_file, import_files, Content, ImportedFile, ImportedSet};
pub use model::{
    Aperture, ApertureShape, DrillFile, GerberLayer, Hit, Item, MacroShape, Slot, Units, P,
};
pub use odb::{parse_matrix, OdbContext, OdbLayer, OdbLayerKind, OdbLimits, OdbMatrix};
pub use parse::parse_gerber;
pub use settings::{ImportSettings, ZeroOmission};
