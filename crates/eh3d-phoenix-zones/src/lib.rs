// SPDX-License-Identifier: GPL-3.0-or-later
//! The filling of copper zones of a board.
//!
//! * [`fill`]: the fill itself. The outline of a zone is cut into convex pieces; the copper of
//!   other nets on the layer, the edge of the board, the zones of a higher priority and the pads
//!   of the net that are kept apart are taken out with exact integer polygon clipping; thermal
//!   reliefs leave spokes to the pads of the net; islands that are too small, too narrow or
//!   joined to nothing are dropped. The order of everything is fixed, so the same board gives the
//!   same bytes;
//! * [`check`]: the measuring of a fill against the board and the rules, independent of the way it
//!   was made, with proven breaks (clearance, outline, island size and width) apart from warnings;
//! * [`rules`]: the clearances taken from the rule file and the optional file with the priority and
//!   the limits of single zones;
//! * [`settings`]: every radius, width and limit, read from `config/zones.cfg`;
//! * [`report`]: the text and JSON of a fill and of a check;
//! * [`codes`]: the codes `PHOENIX-ZONE-001` to `PHOENIX-ZONE-019` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. Pads
//! that are not at a right angle are the smallest circle around them, circles are polygons of 8 or
//! 16 sides that hold the circle (so gaps are a little wider than the rule, never narrower), the
//! cut points are rounded to the nanometre, and the narrow-island test looks at the whole island,
//! not at its necks. A fill is a set of convex polygons that touch along their sides; it is not
//! merged into one outline per island.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod check;
pub mod codes;
pub mod fill;
mod geom;
mod items;
pub mod report;
pub mod rules;
pub mod settings;

pub use check::{check_fill, has_errors};
pub use codes::{title, Finding, Severity, ZoneError};
pub use fill::{fill_board, FillFile, FillReport, FilledZone, ZoneStats, FILL_FORMAT};
pub use report::{fill_file_json, fill_json, fill_text, findings_text, CheckReport};
pub use rules::{ZoneEntry, ZoneFile, ZoneRules, ZONES_FORMAT};
pub use settings::{Connection, ZoneSettings};
