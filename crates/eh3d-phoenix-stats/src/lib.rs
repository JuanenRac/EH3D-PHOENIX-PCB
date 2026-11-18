// SPDX-License-Identifier: GPL-3.0-or-later
//! Statistics of a board: the copper of each layer, the drills, the counts and the density.
//!
//! * [`stats`]: the numbers of a board, the area, the counts of parts, pads, tracks, vias and
//!   zones, the copper of each layer by pads, tracks, vias and zones with its coverage, the drills
//!   by diameter and the density per square centimetre;
//! * [`settings`]: the limit and the choices of the report, read from `config/stats.cfg`;
//! * [`report`]: the text, with its titles in the eight languages, and the JSON of a report, the
//!   same every time;
//! * [`codes`]: the codes `PHOENIX-STATS-001` to `PHOENIX-STATS-007` with their titles in the
//!   eight languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. The copper
//! of a layer adds the pads, the tracks, the vias and the zones without taking away where they
//! overlap and counts a zone as its whole outline before the clearances, so it is an upper bound
//! that is never above the area of the board; the arcs of the outline are not in the area.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
mod geom;
pub mod report;
pub mod settings;
pub mod stats;

pub use codes::{title, Finding, Severity, StatsError};
pub use report::{findings_text, report_json, report_text};
pub use settings::StatsSettings;
pub use stats::{compute, BoardArea, BoardStats, Counts, Density, DrillStats, LayerStats, FORMAT};
