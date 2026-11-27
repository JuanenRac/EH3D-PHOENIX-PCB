// SPDX-License-Identifier: GPL-3.0-or-later
//! Writer of IPC-2581 files, write only.
//!
//! * [`export`]: the file of a board, a subset of revision C: the layers, the outline and its
//!   cutouts, the standard shapes and padstacks, the packages and the placed parts, the nets, the
//!   pads, vias, tracks and zone outlines of each copper layer, and a basic bill of materials,
//!   as an XML document that is the same every time, with a summary of what it holds;
//! * [`d356`]: the netlist of a board as an IPC-D-356A test file (one line of 80 columns for each
//!   pad of a part), a prototype whose columns have not been compared with the standard;
//! * [`pull`], [`units`], [`read`], [`read_model`], [`stack_summary`] and [`read_report`]: the
//!   reader of IPC-2581 files (revisions A, B and C, as XML): a bounded pull parser, the exact
//!   conversion of every length to nanometres, the layers, the stack-up with its materials, the
//!   outline, holes, parts, nets and copper of every step, the bills of materials, what was left
//!   out by name, and a report in the eight languages;
//! * [`settings`]: what goes into the file and the limits, read from `config/ipc2581.cfg`;
//! * [`codes`]: the codes `PHOENIX-IPC2581-001` to `PHOENIX-IPC2581-008` with their titles in the
//!   eight languages.
//!
//! **State: prototype.** The file the writer makes has been read back only by the XML reader of
//! this repository, never checked against the schema of the standard and never opened in a viewer
//! or by a maker. The reader has been run on sample files of several tools in all three revisions.
//! What it leaves out is reported with the file: the silkscreen items, the copper fill of the
//! zones, the holes that are not plated (they are in the padstacks only)
//! and the stack-up.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod d356;
pub mod export;
pub mod pull;
pub mod read;
pub mod read_model;
pub mod read_report;
#[cfg(test)]
mod read_tests;
pub mod settings;
pub mod stack_summary;
pub mod units;
pub mod xform;
pub mod zipread;

pub use codes::{title, Finding, Ipc2581Error, Severity};
pub use d356::{d356, D356};
pub use export::{degrees, export, mm, Ipc2581, Summary, SUMMARY_FORMAT};
pub use read::{read_bytes, read_str, ReadOptions};
pub use read_model::{Counts, IpcDoc};
pub use read_report::report_text;
pub use settings::Ipc2581Settings;
pub use stack_summary::{stack_summary, StackKind, StackRow, StackSummary};
pub use units::Unit;
pub use xform::Xf;
pub use zipread::{read_archive, ZipLimits};
