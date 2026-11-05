// SPDX-License-Identifier: GPL-3.0-or-later
//! The library sanitizer: it reads a folder of old EAGLE libraries (`.lbr`) and reports the
//! footprints, symbols and device sets that are duplicated or nearly duplicated. It is analysis
//! only: no library is rewritten, nothing is merged, and the plan it writes is never applied.
//!
//! * [`scan`]: the batch inventory, with the limits of the corpus crate; a file that cannot be
//!   used is noted and the walk goes on;
//! * [`hash`]: the canonical geometric hash of a footprint or a symbol, which does not change with
//!   the order of the items, the names or small roundings, and, when the settings allow it, with
//!   turns by 90 degrees, a mirror or a shift;
//! * [`fuzzy`]: the approximate distance between two items, as a score with a threshold;
//! * [`group`]: the groups of duplicates, the item to keep and the replacements;
//! * [`report`], [`html`] and [`plan`]: the report as JSON and as one HTML page, and the plan
//!   of the master library that is written and not applied;
//! * [`settings`]: every limit, tolerance and weight, read from `config/sanitizer.cfg`.
//!
//! The codes are `PHOENIX-SAN-<NNN>`, with their titles in the eight languages of the suite.
//! The report names libraries and items, so it is for the owner of the libraries and is not meant
//! to be published.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod fuzzy;
pub mod group;
pub mod hash;
pub mod html;
pub mod model;
pub mod msg;
pub mod output;
pub mod plan;
pub mod report;
pub mod scan;
pub mod settings;

pub use codes::SanError;
pub use group::{Group, Member};
pub use html::html_report;
pub use output::{output_paths, write_outputs};
pub use plan::{plan_of, Plan};
pub use report::{analyse_libraries, sanitize, Report};
pub use settings::SanitizerSettings;

#[cfg(test)]
mod testdata;
