// SPDX-License-Identifier: GPL-3.0-or-later
//! The result of a run: the tracks and vias to add as one reversible all-or-nothing operation of
//! the board, the same as operations of the board editor, and a report that is the same for the
//! same input.
//!
//! The plan is judged by the design rules on a copy of the board before it is handed out. If all
//! the routes together add a rule error, the nets are added one by one in order and a net whose
//! routes add an error is left out (and reported); the plan never adds an error that the board did
//! not have.

use eh3d_phoenix_board::{Board, Id, Operation, Segment, Track, Via};
use eh3d_phoenix_boardedit::ids::IdSupply;
use eh3d_phoenix_boardedit::{EditOp, Shape};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::{diff, findings, ratsnest, RouteSettings, RuleContext};
use eh3d_phoenix_rules::Severity;

use crate::codes::{title, AutorouteError, APPLY_FAILED, INCOMPLETE, REJECTED_BY_RULES};
use crate::geom::{pair, seg_len};

/// A track to add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedTrack {
    /// Its net.
    pub net: Id,
    /// Its copper layer.
    pub layer: Id,
    /// Its width in nanometres.
    pub width_nm: i64,
    /// The points, each joined to the next by a straight piece at 0, 45 or 90 degrees.
    pub points: Vec<Point>,
}

impl PlannedTrack {
    /// The length of the track in nanometres, each piece rounded up.
    pub fn length_nm(&self) -> i64 {
        self.points
            .windows(2)
            .map(|w| seg_len(pair(w[0]), pair(w[1])))
            .sum()
    }
}

/// A via to add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedVia {
    /// Its net.
    pub net: Id,
    /// Where it is.
    pub at: Point,
    /// The first layer.
    pub from_layer: Id,
    /// The last layer.
    pub to_layer: Id,
    /// The drill in nanometres.
    pub drill_nm: i64,
    /// The pad diameter in nanometres.
    pub diameter_nm: i64,
}

/// A connection that was not routed, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrouted {
    /// The name of the net.
    pub net: String,
    /// One end, as `R1.2`.
    pub from: String,
    /// The other end.
    pub to: String,
    /// The code of the reason.
    pub code: &'static str,
    /// The detail, in English.
    pub detail: String,
}

/// What a run did, in counts that do not depend on a clock.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// The nets that had connections to route.
    pub nets_total: usize,
    /// The nets whose connections are all routed in the plan, by name.
    pub nets_completed: Vec<String>,
    /// The connections to route.
    pub connections_total: usize,
    /// The connections the plan routes.
    pub connections_routed: usize,
    /// The connections it does not, with the reason.
    pub unrouted: Vec<Unrouted>,
    /// The tracks it adds.
    pub tracks_added: usize,
    /// The vias it adds.
    pub vias_added: usize,
    /// The length of the tracks it adds, in nanometres.
    pub length_nm: i64,
    /// How many times a route was torn up to make room for another.
    pub ripups: u32,
    /// How many nodes the searches expanded.
    pub expansions: u64,
    /// The errors of the design rules before.
    pub errors_before: usize,
    /// The errors of the design rules after (never more than before).
    pub errors_after: usize,
    /// The connections of the whole board that are still open after the plan.
    pub connections_left: usize,
}

/// The routes of one net, as they leave the search.
pub(crate) struct NetOutcome {
    pub name: String,
    /// The connections of the net.
    pub total: usize,
    /// The ends of the connections that were routed.
    pub routed: Vec<(String, String)>,
    pub tracks: Vec<PlannedTrack>,
    pub vias: Vec<PlannedVia>,
}

/// The counts of the search.
pub(crate) struct Totals {
    pub ripups: u32,
    pub expansions: u64,
}

/// The plan.
#[derive(Debug, Clone)]
pub struct AutoroutePlan {
    /// The tracks to add, in the order of the nets.
    pub tracks: Vec<PlannedTrack>,
    /// The vias to add.
    pub vias: Vec<PlannedVia>,
    /// The report.
    pub report: Report,
    /// The operation that adds everything, with an exact inverse: one step of the history.
    pub operation: Operation,
    /// The board with the plan applied.
    pub board_after: Board,
    diagonal: bool,
}

impl AutoroutePlan {
    /// Applies the plan to a board, all or nothing.
    pub fn apply(&self, board: &mut Board) -> Result<(), AutorouteError> {
        board
            .apply(&self.operation)
            .map(|_| ())
            .map_err(|e| AutorouteError::new(APPLY_FAILED, e.to_string()))
    }

    /// The plan as one group of operations of the board editor, which the editor keeps or undoes
    /// as one step and judges by the design rules.
    pub fn edit_ops(&self) -> EditOp {
        let shape = if self.diagonal {
            Shape::Diagonal
        } else {
            Shape::Orthogonal
        };
        let mut ops: Vec<EditOp> = Vec::new();
        for track in &self.tracks {
            ops.push(EditOp::Route {
                net: track.net,
                layer: track.layer,
                points: track.points.clone(),
                shape: Some(shape),
                width_nm: Some(track.width_nm),
                horizontal_first: None,
            });
        }
        for via in &self.vias {
            ops.push(EditOp::Via {
                net: via.net,
                at: via.at,
                from_layer: Some(via.from_layer),
                to_layer: Some(via.to_layer),
            });
        }
        EditOp::Group { ops }
    }

    /// Refuses a plan that leaves a connection of the nets it was asked for unrouted.
    pub fn require_complete(&self) -> Result<(), AutorouteError> {
        if self.report.unrouted.is_empty() {
            return Ok(());
        }
        Err(AutorouteError::new(
            INCOMPLETE,
            format!(
                "{} of {} connections are unrouted",
                self.report.unrouted.len(),
                self.report.connections_total
            ),
        ))
    }

    /// The report as text, with the titles of the reasons in the language with this tag.
    pub fn to_text(&self, tag: &str) -> String {
        let r = &self.report;
        let mut out = String::new();
        out.push_str(&format!(
            "autoroute: {} of {} connections routed, {} of {} nets complete\n",
            r.connections_routed,
            r.connections_total,
            r.nets_completed.len(),
            r.nets_total
        ));
        out.push_str(&format!(
            "tracks added: {}, vias added: {}, length: {} nm\n",
            r.tracks_added, r.vias_added, r.length_nm
        ));
        out.push_str(&format!(
            "rip-ups: {}, expanded nodes: {}\n",
            r.ripups, r.expansions
        ));
        out.push_str(&format!(
            "rule errors before: {}, after: {}, connections still open: {}\n",
            r.errors_before, r.errors_after, r.connections_left
        ));
        for name in &r.nets_completed {
            out.push_str(&format!("complete: {name}\n"));
        }
        for u in &r.unrouted {
            out.push_str(&format!(
                "unrouted: {}: {} -> {}: {}: {}: {}\n",
                u.net,
                u.from,
                u.to,
                u.code,
                title(u.code, tag),
                u.detail
            ));
        }
        out
    }

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        let r = &self.report;
        let names: Vec<String> = r
            .nets_completed
            .iter()
            .map(|n| format!("\"{}\"", json_text(n)))
            .collect();
        let missed: Vec<String> = r
            .unrouted
            .iter()
            .map(|u| {
                format!(
                    "{{\"net\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"code\":\"{}\",\"detail\":\"{}\"}}",
                    json_text(&u.net),
                    json_text(&u.from),
                    json_text(&u.to),
                    u.code,
                    json_text(&u.detail)
                )
            })
            .collect();
        format!(
            "{{\"nets_total\":{},\"nets_completed\":[{}],\"connections_total\":{},\"connections_routed\":{},\"tracks_added\":{},\"vias_added\":{},\"length_nm\":{},\"ripups\":{},\"expansions\":{},\"errors_before\":{},\"errors_after\":{},\"connections_left\":{},\"unrouted\":[{}]}}",
            r.nets_total,
            names.join(","),
            r.connections_total,
            r.connections_routed,
            r.tracks_added,
            r.vias_added,
            r.length_nm,
            r.ripups,
            r.expansions,
            r.errors_before,
            r.errors_after,
            r.connections_left,
            missed.join(",")
        )
    }
}

fn json_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn apply_failed(what: impl ToString) -> AutorouteError {
    AutorouteError::new(APPLY_FAILED, what.to_string())
}

/// The operations that add the routes of one net.
fn net_operations(
    outcome: &NetOutcome,
    ids: &mut IdSupply,
    board: &Board,
) -> Result<Vec<Operation>, AutorouteError> {
    let mut ops: Vec<Operation> = Vec::new();
    for track in &outcome.tracks {
        let path: Vec<Segment> = track
            .points
            .windows(2)
            .map(|w| Segment::Line {
                start: w[0],
                end: w[1],
            })
            .collect();
        ops.push(Operation::AddTrack {
            id: ids.next_for(board),
            track: Track {
                net: track.net,
                layer: track.layer,
                width: Nm::new(track.width_nm).map_err(apply_failed)?,
                path,
            },
        });
    }
    for via in &outcome.vias {
        ops.push(Operation::AddVia {
            id: ids.next_for(board),
            via: Via {
                net: via.net,
                at: via.at,
                drill: Nm::new(via.drill_nm).map_err(apply_failed)?,
                diameter: Nm::new(via.diameter_nm).map_err(apply_failed)?,
                from_layer: via.from_layer,
                to_layer: via.to_layer,
            },
        });
    }
    Ok(ops)
}

fn count_errors(found: &[eh3d_phoenix_rules::Finding]) -> usize {
    found
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count()
}

/// Judges the routes by the rules and builds the plan.
pub(crate) fn finish(
    board: &Board,
    ctx: &RuleContext,
    route: &RouteSettings,
    diagonal: bool,
    outcomes: Vec<NetOutcome>,
    mut unrouted: Vec<Unrouted>,
    totals: &Totals,
) -> Result<AutoroutePlan, AutorouteError> {
    let mut ids = IdSupply::new().map_err(apply_failed)?;
    let before = findings(board, ctx);
    let mut per_net: Vec<Vec<Operation>> = Vec::with_capacity(outcomes.len());
    for outcome in &outcomes {
        per_net.push(net_operations(outcome, &mut ids, board)?);
    }
    let all: Vec<Operation> = per_net.iter().flatten().cloned().collect();
    let mut kept = vec![true; outcomes.len()];
    let mut board_after = board.clone();
    let whole = board_after.apply(&Operation::Batch { ops: all }).is_ok()
        && !diff(&before, &findings(&board_after, ctx)).adds_error();
    if !whole {
        board_after = board.clone();
        for (k, ops) in per_net.iter().enumerate() {
            if ops.is_empty() {
                continue;
            }
            let mut trial = board_after.clone();
            let why = match trial.apply(&Operation::Batch { ops: ops.clone() }) {
                Err(e) => Some(e.to_string()),
                Ok(_) => {
                    let d = diff(&before, &findings(&trial, ctx));
                    d.added
                        .iter()
                        .find(|f| f.severity == Severity::Error)
                        .map(|f| format!("{} at {}", f.code, f.location))
                }
            };
            match why {
                None => board_after = trial,
                Some(detail) => {
                    kept[k] = false;
                    for (from, to) in &outcomes[k].routed {
                        unrouted.push(Unrouted {
                            net: outcomes[k].name.clone(),
                            from: from.clone(),
                            to: to.clone(),
                            code: REJECTED_BY_RULES,
                            detail: detail.clone(),
                        });
                    }
                }
            }
        }
    }
    let mut tracks: Vec<PlannedTrack> = Vec::new();
    let mut vias: Vec<PlannedVia> = Vec::new();
    let mut ops: Vec<Operation> = Vec::new();
    let mut completed: Vec<String> = Vec::new();
    let mut routed = 0_usize;
    for (k, outcome) in outcomes.iter().enumerate() {
        if !kept[k] {
            continue;
        }
        tracks.extend(outcome.tracks.iter().cloned());
        vias.extend(outcome.vias.iter().cloned());
        ops.extend(per_net[k].iter().cloned());
        routed += outcome.routed.len();
        if outcome.routed.len() == outcome.total {
            completed.push(outcome.name.clone());
        }
    }
    completed.sort();
    unrouted.sort_by(|a, b| {
        a.net
            .cmp(&b.net)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
    let after = findings(&board_after, ctx);
    let left = ratsnest(&board_after, route).map_or(0, |lines| lines.len());
    let report = Report {
        nets_total: outcomes.len(),
        nets_completed: completed,
        connections_total: outcomes.iter().map(|o| o.total).sum(),
        connections_routed: routed,
        unrouted,
        tracks_added: tracks.len(),
        vias_added: vias.len(),
        length_nm: tracks.iter().map(PlannedTrack::length_nm).sum(),
        ripups: totals.ripups,
        expansions: totals.expansions,
        errors_before: count_errors(&before),
        errors_after: count_errors(&after),
        connections_left: left,
    };
    Ok(AutoroutePlan {
        tracks,
        vias,
        report,
        operation: Operation::Batch { ops },
        board_after,
        diagonal,
    })
}
