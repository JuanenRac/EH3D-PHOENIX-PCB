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
//! * [`advanced`]: further checks with a settings file of their own (annular ring, distance to the
//!   board edge, acid traps, silkscreen over holes, isolation by voltage, copper islands, stubs,
//!   duplicate vias, pads on no net and parts outside the outline), with the codes
//!   `PHOENIX-DRC-<NNN>`;
//! * [`classes`]: net classes of a rule file of the second format, with limits of their own for
//!   the width of the tracks and the size of the vias of their nets;
//! * [`dfm_more`]: further lines of the capability profile (drill sizes, aspect ratio of the holes,
//!   vias in pads, the silkscreen, the copper layers, the proportion of the board, blind vias and
//!   surface-mount pads, the ring of drilled pads and the distance from a hole to the edge, the width of the cutouts and the unplated holes) with the
//!   codes `PHOENIX-DFM-007` to `PHOENIX-DFM-019`;
//! * [`dfm`]: a capability profile of a board maker read from a text file, its limits for the rule
//!   check and the checks of the layers, the size of the board and the holes (`PHOENIX-DFM-<NNN>`);
//! * [`dfm_compare`]: the comparison of one board against the profiles of several makers, as text
//!   in eight languages and as JSON;
//! * [`mask`]: the strips of solder mask between two openings that are narrower than a maker can
//!   print;
//! * [`netlint`]: the spelling of the names of the nets (spaces, odd characters, length, case,
//!   names a program made up and halves of pairs with no other half);
//! * [`notes`]: the notes of the consistency checks of a board that a report carries as information
//!   (a polygon over `max_polygon_points`, code `PHOENIX-BOARD-017`);
//! * [`report`]: the report as text and as JSON, with proven (`confirmed`) and unjudged
//!   (`inconclusive`) findings in two lists;
//! * [`report_export`]: the report as CSV and as a page of HTML, grouped by category;
//! * [`codes`]: the stable codes `PHOENIX-CHECK-<NNN>`, `PHOENIX-RULEFILE-<NNN>` and
//!   `PHOENIX-CONN-<NNN>`, with their titles in the eight languages.
//!
//! All arithmetic is in integer nanometres and exact; when the numbers do not fit, or a shape is
//! too coarse to tell, the result is inconclusive and never a failure. Nothing here panics on a
//! damaged board. The rules and the connectivity have been tried on boards made by hand and by the
//! tests; they have not been compared with the reference tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod advanced;
pub mod classes;
pub mod codes;
pub mod connectivity;
pub mod dfm;
pub mod dfm_compare;
pub mod dfm_more;
pub mod engine;
mod geometry;
mod items;
pub mod mask;
#[cfg(test)]
mod mask_tests;
pub mod netlint;
#[cfg(test)]
mod netlint_tests;
pub mod notes;
pub mod report;
pub mod report_export;
pub mod rulefile;

pub use advanced::{
    advanced_findings, check_advanced, check_advanced_with, to_sarif, AdvancedConfig,
    AdvancedError, IndexMode,
};
pub use classes::{check_classes, NetClass, NetClasses};
pub use codes::{title, Lang, REGISTRY};
pub use connectivity::{analyse_board, check_connectivity, compare_schematic};
pub use dfm::{builtin_names, builtin_profile, dfm_findings, DfmError, DfmProfile, BUILTIN};
pub use dfm_compare::{compare_profiles, DfmComparison, ProfileRun};
pub use engine::{
    check_board, check_board_with, clearance_work, edge_work, ClearanceWork, EdgeWork, Rule,
};
pub use mask::{mask_slivers, MaskOpening, MaskSliver};
pub use netlint::{lint_net_names, NetIssue, NetIssueKind, NetLintConfig};
pub use notes::board_notes;
pub use report::{Finding, Report, Severity};
pub use report_export::{category, category_counts, report_csv, report_html};
pub use rulefile::{parse_length, RuleFile, RuleFileError, RuleValues};
