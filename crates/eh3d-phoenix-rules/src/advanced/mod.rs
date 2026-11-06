// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced design rule checks: further checks that run beside the ones of [`crate::engine`]
//! and never change them or their codes.
//!
//! Every limit, every level (off, info, warning or error) and the table of spacing by voltage are
//! read from `config/drc_advanced.cfg` ([`AdvancedConfig`]); no value is written in a check. The
//! checks, in the order they run, with their codes:
//!
//! * `PHOENIX-DRC-001` annular ring of vias and drilled pads against their drill;
//! * `PHOENIX-DRC-002` copper (tracks and vias) against the board edge;
//! * `PHOENIX-DRC-003` pads against the board edge;
//! * `PHOENIX-DRC-004` smallest drill by net class;
//! * `PHOENIX-DRC-005` silkscreen against the drill of through holes;
//! * `PHOENIX-DRC-006` necks: a narrow track whose end joins wider copper of its net;
//! * `PHOENIX-DRC-007` acid traps: pieces of tracks of one net that meet at an acute angle;
//! * `PHOENIX-DRC-008` spacing by the voltage between nets, from the table of IPC-2221 (the
//!   figures in the settings file are not verified against the standard);
//! * `PHOENIX-DRC-009` copper islands that join no pad;
//! * `PHOENIX-DRC-010` stubs: a track with one free end;
//! * `PHOENIX-DRC-011` duplicate or stacked vias;
//! * `PHOENIX-DRC-012` pads on no net;
//! * `PHOENIX-DRC-013` parts with a pad outside the outline;
//! * `PHOENIX-DRC-014` what could not be judged (information).
//!
//! The pairs of items that are close to each other are found with a uniform grid. The grid only
//! chooses which pairs are compared, so the report is the one of the search that compares every
//! pair ([`IndexMode::Naive`]); the tests check it. Arcs in tracks are measured by their end
//! points: a distance found that way is proven, a distance not found says nothing about the arc.
//! The checks have been tried on boards made by hand only and have not been compared with the
//! rules of a manufacturer or with another tool.

mod cfg;
mod checks;
mod checks_more;
pub mod codes;
mod grid;
mod sarif;

pub use cfg::{AdvancedConfig, AdvancedError, DrillClass, IsoRow, MAX_CONFIG_BYTES};
pub use sarif::to_sarif;

use eh3d_phoenix_board::Board;

use crate::report::{Finding, Report, Severity};

/// A rule of the advanced checks: its place in the order, its stable name and its code.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleInfo {
    pub(crate) rank: u8,
    pub(crate) name: &'static str,
    pub(crate) code: &'static str,
}

pub(crate) const R_RING: RuleInfo = RuleInfo {
    rank: 21,
    name: "annular_ring_min",
    code: codes::RING,
};
pub(crate) const R_COPPER_EDGE: RuleInfo = RuleInfo {
    rank: 22,
    name: "copper_edge",
    code: codes::COPPER_EDGE,
};
pub(crate) const R_PAD_EDGE: RuleInfo = RuleInfo {
    rank: 23,
    name: "pad_edge",
    code: codes::PAD_EDGE,
};
pub(crate) const R_DRILL_CLASS: RuleInfo = RuleInfo {
    rank: 24,
    name: "drill_class",
    code: codes::DRILL_CLASS,
};
pub(crate) const R_SILK_HOLE: RuleInfo = RuleInfo {
    rank: 25,
    name: "silk_hole",
    code: codes::SILK_HOLE,
};
pub(crate) const R_NECK: RuleInfo = RuleInfo {
    rank: 26,
    name: "neck",
    code: codes::NECK,
};
pub(crate) const R_ACID_TRAP: RuleInfo = RuleInfo {
    rank: 27,
    name: "acid_trap",
    code: codes::ACID_TRAP,
};
pub(crate) const R_ISOLATION: RuleInfo = RuleInfo {
    rank: 28,
    name: "isolation",
    code: codes::ISOLATION,
};
pub(crate) const R_ISLAND: RuleInfo = RuleInfo {
    rank: 29,
    name: "island",
    code: codes::ISLAND,
};
pub(crate) const R_STUB: RuleInfo = RuleInfo {
    rank: 30,
    name: "stub",
    code: codes::STUB,
};
pub(crate) const R_VIA_DUP: RuleInfo = RuleInfo {
    rank: 31,
    name: "via_duplicate",
    code: codes::VIA_DUP,
};
pub(crate) const R_PAD_NO_NET: RuleInfo = RuleInfo {
    rank: 32,
    name: "pad_no_net",
    code: codes::PAD_NO_NET,
};
pub(crate) const R_OUTSIDE: RuleInfo = RuleInfo {
    rank: 33,
    name: "outside_outline",
    code: codes::OUTSIDE,
};
pub(crate) const R_NOT_JUDGED: RuleInfo = RuleInfo {
    rank: 34,
    name: "advanced_not_judged",
    code: codes::NOT_JUDGED,
};

/// A finding of a rule, at a severity, about a place.
pub(crate) fn make(rule: &RuleInfo, severity: Severity, location: String) -> Finding {
    Finding::new(rule.rank, rule.name, rule.code, severity, location)
}

/// A finding that goes to the inconclusive list.
pub(crate) fn not_judged(location: String, why: &str) -> Finding {
    make(&R_NOT_JUDGED, Severity::Info, location).why(why)
}

/// How the pairs of close items are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexMode {
    /// A uniform grid (the cell is `grid_cell_nm` of the settings): the default.
    Grid,
    /// Every pair is compared. Slow on a large board; kept to check the grid against.
    Naive,
}

/// Runs the advanced checks on a board with the settings, using the grid.
pub fn check_advanced(board: &Board, config: &AdvancedConfig) -> Report {
    check_advanced_with(board, config, IndexMode::Grid)
}

/// Runs the advanced checks with the chosen way of finding close items. Both ways give the same
/// report.
pub fn check_advanced_with(board: &Board, config: &AdvancedConfig, mode: IndexMode) -> Report {
    let ctx = checks::build(board, config, mode);
    let mut report = Report::new("drc-advanced");
    checks::run_all(&ctx, &mut report);
    report.finish();
    report
}

/// The proven findings of the advanced checks with the built-in settings, for the board editor.
pub fn advanced_findings(board: &Board) -> Vec<Finding> {
    check_advanced(board, &AdvancedConfig::default()).confirmed
}
