//! Reader of EAGLE XML files (versions 6 to 9). It only reads.
//!
//! * [`xml`]: a small XML parser with limits on size, depth and node count, so a hostile or damaged
//!   file gives an error and never a panic or a huge allocation;
//! * [`codes`]: the stable codes `PHOENIX-EAGLE-<NNN>`;
//! * the writer of the XML tree ([`write_document`]): the same tree always gives the same text and
//!   reading that text gives the tree back, which the corpus measure checks on every file;
//! * [`document`]: reads a file into a [`DocumentSummary`] (a library, a board or a schematic) and
//!   the diagnostics that tell what could not be confirmed;
//! * [`corpus`]: copies files with a SHA-256 check, proves the originals were not changed, and
//!   measures the read rate over the copies in a form that holds only codes and counts.
//!
//! This crate does not write EAGLE files from the design model and does not build that model yet: the summary is
//! what the readers of boards and schematics will be checked against.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod corpus;
pub mod document;
pub mod import;
pub mod xml;

pub use document::{read_bytes, DocumentKind, DocumentSummary, Reading};
pub use import::EagleImporter;
pub use xml::{write_document, Element, Limits, XmlError};
