// SPDX-License-Identifier: GPL-3.0-or-later
//! Test points of a board and the nets they reach.
//!
//! * [`analyse`]: the pads of the parts that are test points, the nets that have none on the side
//!   that is probed, and the test points that are too small, too close to each other or too close
//!   to the edge;
//! * [`settings`]: what counts as a test point, the side that is probed and every limit, read from
//!   `config/testpoints.cfg`;
//! * [`report`]: the text, with its titles in the eight languages, and the JSON of a report, the
//!   same every time;
//! * [`codes`]: the codes `PHOENIX-TP-001` to `PHOENIX-TP-014` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. A test
//! point is recognised by the reference or the footprint name of its part; a via or a track that
//! the probe could touch is not a test point, and the access to the pad (the solder mask, a part
//! on the other side that stands in the way) is not judged.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod analyse;
pub mod codes;
pub mod report;
pub mod settings;

pub use analyse::{analyse, NetCoverage, TestPoint, TestPointReport, FORMAT};
pub use codes::{title, Finding, Severity, TestPointError};
pub use report::{findings_text, report_json, report_text};
pub use settings::{ProbeSide, TestPointSettings};
