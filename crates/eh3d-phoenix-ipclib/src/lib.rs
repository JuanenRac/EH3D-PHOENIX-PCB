// SPDX-License-Identifier: GPL-3.0-or-later
//! The own library of footprints (a prototype) and the bulk import of libraries:
//!
//! * [`family`]: the families of footprints, with the dimensions of their bodies and leads, the
//!   tolerances and the fillet goals of the density levels A, B and C of IPC-7351B, read from
//!   data files (`config/ipc_families/*.json`, one family per file). Each family says whether it
//!   has been verified against datasheets; the ones that come with the program have not;
//! * [`generate`]: the generator, which makes a footprint with the canonical name of IPC-7351B
//!   (`RESC1608X55N`, `SOIC127P600X175-8N`) from a family, a variant, a density level and
//!   parameters;
//! * [`fp`]: the footprint, its output as a footprint of the library editor (saved as an EAGLE
//!   library) and as a `.kicad_mod` file, written only to new files;
//! * [`check`]: the checker of the library with the codes `PHOENIX-IPC-001` to `010`;
//! * [`compare`] and [`pads`]: the comparison of a footprint of the owner with the generated one,
//!   as a report that replaces nothing;
//! * [`bulk`]: the walk through a folder of EAGLE and KiCad libraries, the quality report of every
//!   footprint and symbol and the index `phoenix-libindex/1` with the hash of each item;
//! * [`settings`] and [`codes`]: the settings files and the codes, with their titles in eight
//!   languages.
//!
//! The dimensions of the families have not been checked against the text of the standard or
//! against any datasheet, and no EAGLE or KiCad has opened a file written here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bulk;
pub mod check;
pub mod codes;
pub mod compare;
pub mod family;
pub mod fp;
pub mod generate;
pub mod pads;
pub mod settings;

pub use bulk::{index_folder, write_index, Index, INDEX_FORMAT};
pub use check::{check_footprint, check_library, CheckReport};
pub use codes::{IpcError, TITLES};
pub use compare::{compare_pads, load_footprint, CompareReport};
pub use family::{Density, Family, FamilySet};
pub use fp::{save_kicad_mod, save_lbr, IpcFootprint};
pub use generate::{generate, library, Params};
pub use settings::{BulkSettings, IpcSettings};
