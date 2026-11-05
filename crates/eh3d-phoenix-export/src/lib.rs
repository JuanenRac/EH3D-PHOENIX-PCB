//! Manufacturing export of a board.
//!
//! * [`gerber`]: one layer as a Gerber X2 file (copper, solder mask, paste, legend, profile), through
//!   the deterministic writer of EH3D-CORE-LIBS, with the attributes `.GenerationSoftware`,
//!   `.FileFunction` and `.FilePolarity`;
//! * [`excellon`]: the holes as Excellon files, plated and not plated apart, with slots (`G85`);
//! * [`assembly`]: the placement (pick and place) file and the bill of materials, as CSV;
//! * [`profile`]: the export profiles, read from a JSON file (`config/export_profiles.json`);
//! * [`fab`]: the whole set of files of a board, its manifest of SHA-256 sums and the folder it is
//!   written to;
//! * [`package`]: an `.eh3dproj` 1.0.0 package (EH3D-COMMON) with those files, written without
//!   the clock and without randomness, so the same board and options give the same bytes;
//! * [`verify`]: the check that reads the files back and compares them with the model;
//! * [`codes`] and [`messages`]: the stable codes `PHOENIX-EXPORT-<NNN>` for what the files cannot
//!   say, with their titles in the eight languages.
//!
//! Not done: copper pours (a zone is written as its outline), clearances, blind and buried vias,
//! the Gerber attributes of the pads and nets, polygon pads (the board model has none) and any check
//! against a reference tool or a manufacturer. All of it has been tried on boards made by hand only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assembly;
pub mod codes;
pub mod excellon;
pub mod fab;
pub mod gerber;
pub mod messages;
pub mod package;
pub mod profile;
pub mod verify;

pub use fab::{build, FabFile, FabSet, FileRole};
pub use messages::Lang;
pub use package::{
    export_package, export_package_reproducible, export_package_with, package_of, ExportError,
    ExportOptions, Exported,
};
pub use profile::{Profile, ProfileSet};
pub use verify::verify;
