//! Manufacturing export of a board.
//!
//! * [`gerber`]: one layer as a Gerber X2 file, through the deterministic writer of EH3D-CORE-LIBS;
//! * [`excellon`]: the holes as one Excellon file, through the writer of EH3D-CORE-LIBS;
//! * [`package`]: an `.eh3dproj` 1.0.0 package (EH3D-COMMON) with those files, written without
//!   the clock and without randomness, so the same board and options give the same bytes;
//! * [`codes`]: the stable codes `PHOENIX-EXPORT-<NNN>` for what the files cannot say.
//!
//! Not done: copper pours (a zone is written as its outline), clearances, plating information,
//! slots, the Gerber attributes of the pads and nets, and any check against a reference tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod excellon;
pub mod gerber;
pub mod package;

pub use package::{export_package, ExportError, ExportOptions, Exported};
