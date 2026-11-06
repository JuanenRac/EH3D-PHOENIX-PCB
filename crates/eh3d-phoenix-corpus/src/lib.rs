// SPDX-License-Identifier: GPL-3.0-or-later
//! Inventory of a private corpus of design files, in redacted mode.
//!
//! The inventory counts files by kind and by the version their header declares, and gives one
//! fingerprint of the content. Names, paths, values and anything else taken from the files never
//! appear in the report, so it can leave the machine.
//!
//! It does not read designs: it looks at the first bytes of a file to tell an XML design file
//! from anything else, and at the root element to read the declared version.
//!
//! The module [`suite`] is the other half of the crate: the own corpus of small hand-written
//! pieces in every format that the suite reads or writes, with expectations, round trips between
//! formats and properties on generated boards (`phoenix corpus`).
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod inventory;
pub mod sniff;
pub mod suite;

pub use config::InventoryConfig;
pub use error::CorpusError;
pub use inventory::{inventory, Inventory};
