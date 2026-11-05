// SPDX-License-Identifier: GPL-3.0-or-later
//! Connectivity and design rules of a board.
//!
//! * [`rulefile`]: the values a board is checked against, read from a JSON rule file or from the
//!   `<designrules>` of an EAGLE board; no limit is written in the code of the checks;
//! * [`engine`]: the rule checks in a fixed order (track width, via diameter, drill, annular ring,
//!   copper clearance, board edge, drill spacing, solder mask, zones, silkscreen), each finding with
//!   its place, the rule, the measured value, the limit and the severity;
//! * [`connectivity`]: the nets that the copper really forms (a gap of the tolerance still counts as
//!   touching), shorts and unrouted connections, and the comparison with the schematic;
//! * [`report`]: the report as text and as JSON, with proven (`confirmed`) and unjudged
//!   (`inconclusive`) findings in two lists;
//! * [`codes`]: the stable codes `PHOENIX-CHECK-<NNN>`, `PHOENIX-RULEFILE-<NNN>` and
//!   `PHOENIX-CONN-<NNN>`, with their titles in the eight languages.
//!
//! All arithmetic is in integer nanometres and exact; when the numbers do not fit, or a shape is
//! too coarse to tell, the result is inconclusive and never a failure. Nothing here panics on a
//! damaged board. The rules and the connectivity have been tried on boards made by hand and by the
//! tests; they have not been compared with the reference tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod connectivity;
pub mod engine;
mod geometry;
mod items;
pub mod report;
pub mod rulefile;

pub use codes::{title, Lang, REGISTRY};
pub use connectivity::{analyse_board, check_connectivity, compare_schematic};
pub use engine::{check_board, Rule};
pub use report::{Finding, Report, Severity};
pub use rulefile::{RuleFileError, RuleValues};
