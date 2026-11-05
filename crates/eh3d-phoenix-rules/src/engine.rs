// SPDX-License-Identifier: GPL-3.0-or-later
//! The rule checks, in a fixed order, against the values of a rule file.
//!
//! The order is the order of [`Rule`]: track width, via diameter, drill, annular ring, copper
//! clearance, board edge, drill spacing, then the solder mask, the zones and the silkscreen. Inside
//! one rule the findings are sorted by what they are about, so the same board and rules always give
//! the same report. A rule whose value is zero is off.
//!
//! Clearance, edge distance and drill spacing are measured with exact integer arithmetic against
//! the track and via items; the measured value shown is rounded to the nearest nanometre. The checks
//! that involve the shape of pads, the solder mask, zones and silkscreen are those of the board
//! crate (`Board::check_rules`) and are listed here with their limit but without a measured value.

use eh3d_diagnostic::{Diagnostic, Severity as DiagSeverity};
use eh3d_phoenix_board::rules as board_rules;
use eh3d_phoenix_board::rules::RuleSet;
use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::{Nm, Point};

use crate::codes;
use crate::geometry::{outline_edges, pad_min_side, Dist2, Prim};
use crate::items::{placed_pads, track_prim, via_prim, PlacedPad};
use crate::report::{Finding, Report, Severity};
use crate::rulefile::RuleValues;

/// The rules, in the order the checks run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// Narrowest track.
    TrackWidth,
    /// Smallest via diameter.
    ViaDiameter,
    /// Smallest drill.
    DrillSize,
    /// Thinnest annular ring.
    AnnularRing,
    /// Copper clearance between nets.
    CopperClearance,
    /// Copper to the board edge.
    EdgeClearance,
    /// Drill hole to drill hole.
    DrillSpacing,
    /// Solder mask bridge between openings.
    MaskBridge,
    /// Zone clearance.
    ZoneClearance,
    /// Zones of different nets that overlap.
    ZonesOverlap,
    /// Silkscreen over a pad.
    SilkOverPad,
    /// Silkscreen over a solder mask opening.
    SilkOverMask,
    /// What could not be judged.
    NotJudged,
}

impl Rule {
    /// The place in the order, the stable name and the code of the rule.
    fn meta(self) -> (u8, &'static str, &'static str) {
        match self {
            Rule::TrackWidth => (1, "track_width", codes::TRACK_NARROW),
            Rule::ViaDiameter => (2, "via_diameter", codes::VIA_SMALL),
            Rule::DrillSize => (3, "drill_size", codes::DRILL_SMALL),
            Rule::AnnularRing => (4, "annular_ring", codes::RING_THIN),
            Rule::CopperClearance => (5, "copper_clearance", codes::CLEARANCE),
            Rule::EdgeClearance => (6, "edge_clearance", codes::EDGE_CLOSE),
            Rule::DrillSpacing => (7, "drill_spacing", codes::DRILL_SPACING),
            Rule::MaskBridge => (8, "mask_bridge", codes::MASK_BRIDGE),
            Rule::ZoneClearance => (9, "zone_clearance", codes::ZONE_CLEARANCE),
            Rule::ZonesOverlap => (10, "zones_overlap", codes::ZONES_OVERLAP),
            Rule::SilkOverPad => (11, "silk_over_pad", codes::SILK_OVER_PAD),
            Rule::SilkOverMask => (12, "silk_over_mask", codes::SILK_OVER_MASK),
            Rule::NotJudged => (13, "not_judged", codes::NOT_JUDGED),
        }
    }

    /// The stable name of the rule (`copper_clearance`).
    pub fn name(self) -> &'static str {
        self.meta().1
    }

    /// The stable code of the rule (`PHOENIX-CHECK-005`).
    pub fn code(self) -> &'static str {
        self.meta().2
    }
}

fn make(rule: Rule, severity: Severity, location: String) -> Finding {
    let (rank, name, code) = rule.meta();
    Finding::new(rank, name, code, severity, location)
}

fn on(value: Nm) -> bool {
    value.get() > 0
}

/// A track or a via, as copper that can touch other copper.
struct Copper {
    /// Tracks come before vias.
    rank: u8,
    id: Id,
    net: Id,
    layers: [Id; 2],
    prim: Prim,
    label: String,
}

struct Hole {
    at: Point,
    diameter: i128,
    label: String,
}

struct Run<'a> {
    board: &'a Board,
    rules: &'a RuleValues,
    pads: Vec<PlacedPad>,
    copper: Vec<Copper>,
    report: Report,
}

/// Checks a board against the values of a rule file.
pub fn check_board(board: &Board, rules: &RuleValues) -> Report {
    let mut copper = Vec::new();
    for (id, track) in &board.tracks {
        if let Some(prim) = track_prim(track) {
            copper.push(Copper {
                rank: 0,
                id: *id,
                net: track.net,
                layers: [track.layer, track.layer],
                prim,
                label: format!("track {id}"),
            });
        }
    }
    for (id, via) in &board.vias {
        if let Some(prim) = via_prim(via) {
            copper.push(Copper {
                rank: 1,
                id: *id,
                net: via.net,
                layers: [via.from_layer, via.to_layer],
                prim,
                label: format!("via {id}"),
            });
        }
    }
    let mut run = Run {
        board,
        rules,
        pads: placed_pads(board),
        copper,
        report: Report::new("check"),
    };
    run.track_widths();
    run.via_sizes();
    run.pad_holes();
    run.clearances();
    run.edges();
    run.drill_spacing();
    run.from_board_crate();
    run.report.finish();
    run.report
}

/// The distance between the edges of two items, from the distance between their centre lines.
fn gap_nm(distance: Dist2, lo_a: i128, lo_b: i128) -> i64 {
    let gap = distance.nm_f64() - (lo_a + lo_b) as f64 / 2.0;
    gap.round().max(0.0) as i64
}

impl Run<'_> {
    fn track_widths(&mut self) {
        let limit = self.rules.min_track_width;
        if !on(limit) {
            return;
        }
        for (id, track) in &self.board.tracks {
            if track.width < limit {
                let mut finding = make(Rule::TrackWidth, Severity::Error, format!("track {id}"))
                    .values(track.width.get(), limit.get());
                if let Some(segment) = track.path.first() {
                    let p = segment.start();
                    finding = finding.at(p.x_nm.get(), p.y_nm.get());
                }
                self.report.confirmed.push(finding);
            }
        }
    }

    fn via_sizes(&mut self) {
        let r = self.rules;
        for (id, via) in &self.board.vias {
            let (x, y) = (via.at.x_nm.get(), via.at.y_nm.get());
            if on(r.min_via_diameter) && via.diameter < r.min_via_diameter {
                self.report.confirmed.push(
                    make(Rule::ViaDiameter, Severity::Error, format!("via {id}"))
                        .at(x, y)
                        .values(via.diameter.get(), r.min_via_diameter.get()),
                );
            }
            if on(r.min_drill) && via.drill < r.min_drill {
                self.report.confirmed.push(
                    make(Rule::DrillSize, Severity::Error, format!("via {id}"))
                        .at(x, y)
                        .values(via.drill.get(), r.min_drill.get()),
                );
            }
            if on(r.min_annular_ring) {
                let ring2 = via.diameter.get() - via.drill.get();
                if ring2 < 2 * r.min_annular_ring.get() {
                    self.report.confirmed.push(
                        make(Rule::AnnularRing, Severity::Error, format!("via {id}"))
                            .at(x, y)
                            .values(ring2.div_euclid(2).max(0), r.min_annular_ring.get()),
                    );
                }
            }
        }
    }

    fn pad_holes(&mut self) {
        let r = self.rules;
        for pad in &self.pads {
            let Some(drill) = pad.drill else {
                continue;
            };
            let (x, y) = (pad.at.x_nm.get(), pad.at.y_nm.get());
            if on(r.min_drill) && drill < r.min_drill {
                self.report.confirmed.push(
                    make(Rule::DrillSize, Severity::Error, pad.label.clone())
                        .at(x, y)
                        .values(drill.get(), r.min_drill.get()),
                );
            }
            if on(r.min_annular_ring) {
                let ring2 = pad_min_side(pad.shape) - i128::from(drill.get());
                if ring2 < 2 * i128::from(r.min_annular_ring.get()) {
                    self.report.confirmed.push(
                        make(Rule::AnnularRing, Severity::Error, pad.label.clone())
                            .at(x, y)
                            .values(
                                i64::try_from(ring2.div_euclid(2).max(0)).unwrap_or(0),
                                r.min_annular_ring.get(),
                            ),
                    );
                }
            }
        }
    }

    fn clearances(&mut self) {
        let limit = self.rules.min_clearance;
        if !on(limit) {
            return;
        }
        let c = i128::from(limit.get());
        for item in &self.copper {
            if item.prim.has_arc {
                self.report.inconclusive.push(
                    make(Rule::NotJudged, Severity::Info, item.label.clone()).why(
                        "the track has an arc: its clearance is judged by its end points only",
                    ),
                );
            }
        }
        for (i, a) in self.copper.iter().enumerate() {
            for b in &self.copper[i + 1..] {
                if a.net == b.net || !a.layers.iter().any(|l| b.layers.contains(l)) {
                    continue;
                }
                if !a.prim.may_be_within(&b.prim, c) {
                    continue;
                }
                let (first, second) = if (a.rank, a.id) <= (b.rank, b.id) {
                    (a, b)
                } else {
                    (b, a)
                };
                let location = format!("{} and {}", first.label, second.label);
                let at = first.prim.first_point();
                let verdict = a
                    .prim
                    .distance(&b.prim)
                    .map(|d| (d, d.doubled_below(2 * c + a.prim.lo + b.prim.lo)));
                match verdict {
                    Some((d, Some(true))) => self.report.confirmed.push(
                        make(Rule::CopperClearance, Severity::Error, location)
                            .at(at.x_nm.get(), at.y_nm.get())
                            .values(gap_nm(d, a.prim.lo, b.prim.lo), limit.get()),
                    ),
                    Some((_, Some(false))) => {}
                    _ => self.report.inconclusive.push(
                        make(Rule::NotJudged, Severity::Info, location)
                            .at(at.x_nm.get(), at.y_nm.get())
                            .limit(limit.get())
                            .why("numbers too large for the exact test"),
                    ),
                }
            }
        }
    }

    fn edges(&mut self) {
        let limit = self.rules.min_edge_clearance;
        if !on(limit) {
            return;
        }
        let edges = outline_edges(self.board);
        if edges.is_empty() {
            self.report.inconclusive.push(
                make(Rule::NotJudged, Severity::Info, "board outline".to_owned())
                    .limit(limit.get())
                    .why("the board has no closed outline, so the distance to its edge is not judged"),
            );
            return;
        }
        let e = i128::from(limit.get());
        for item in &self.copper {
            let at = item.prim.first_point();
            let mut broken = false;
            let mut unknown = false;
            let mut nearest = f64::MAX;
            for edge in &edges {
                let Some(d) = item.prim.distance(edge) else {
                    unknown = true;
                    continue;
                };
                nearest = nearest.min(d.nm_f64());
                match d.doubled_below(2 * e + item.prim.lo) {
                    Some(true) => broken = true,
                    Some(false) => {}
                    None => unknown = true,
                }
            }
            if broken {
                let gap = (nearest - item.prim.lo as f64 / 2.0).round().max(0.0) as i64;
                self.report.confirmed.push(
                    make(Rule::EdgeClearance, Severity::Error, item.label.clone())
                        .at(at.x_nm.get(), at.y_nm.get())
                        .values(gap, limit.get()),
                );
            } else if unknown {
                self.report.inconclusive.push(
                    make(Rule::NotJudged, Severity::Info, item.label.clone())
                        .at(at.x_nm.get(), at.y_nm.get())
                        .limit(limit.get())
                        .why("numbers too large for the exact test"),
                );
            }
        }
    }

    fn drill_spacing(&mut self) {
        let limit = self.rules.min_drill_spacing;
        if !on(limit) {
            return;
        }
        let mut holes: Vec<Hole> = Vec::new();
        for (id, via) in &self.board.vias {
            holes.push(Hole {
                at: via.at,
                diameter: i128::from(via.drill.get()),
                label: format!("via {id}"),
            });
        }
        for pad in &self.pads {
            if let Some(drill) = pad.drill {
                holes.push(Hole {
                    at: pad.at,
                    diameter: i128::from(drill.get()),
                    label: pad.label.clone(),
                });
            }
        }
        holes.sort_by_key(|h| (h.at.x_nm.get(), h.at.y_nm.get(), h.label.clone()));
        let l = i128::from(limit.get());
        let widest = holes.iter().map(|h| h.diameter).max().unwrap_or(0);
        let reach = l + widest;
        for (i, a) in holes.iter().enumerate() {
            for b in &holes[i + 1..] {
                // sorted by x: once the x distance alone is too large, every later hole is too far
                if i128::from(b.at.x_nm.get()) - i128::from(a.at.x_nm.get()) >= reach {
                    break;
                }
                let d = Dist2::of_points(a.at, b.at);
                if d.doubled_below(2 * l + a.diameter + b.diameter) == Some(true) {
                    let (first, second) = if a.label <= b.label { (a, b) } else { (b, a) };
                    self.report.confirmed.push(
                        make(
                            Rule::DrillSpacing,
                            Severity::Error,
                            format!("{} and {}", first.label, second.label),
                        )
                        .at(first.at.x_nm.get(), first.at.y_nm.get())
                        .values(gap_nm(d, a.diameter, b.diameter), limit.get()),
                    );
                }
            }
        }
    }

    /// The checks that the board crate already has for pads, the solder mask, zones and
    /// silkscreen. The clearance and edge findings it makes for tracks and vias are left out
    /// because this engine measures them itself.
    fn from_board_crate(&mut self) {
        let r = self.rules;
        let wanted = on(r.min_clearance)
            || on(r.min_edge_clearance)
            || on(r.min_mask_bridge)
            || r.silkscreen_over_pads
            || r.silkscreen_over_mask_openings;
        if !wanted {
            return;
        }
        let set = RuleSet {
            min_track_width: Nm::ZERO,
            min_clearance: r.min_clearance,
            min_drill: Nm::ZERO,
            min_via_diameter: Nm::ZERO,
            min_annular_ring: Nm::ZERO,
            min_edge_clearance: r.min_edge_clearance,
            min_mask_bridge: r.min_mask_bridge,
            silkscreen_over_pads: r.silkscreen_over_pads,
            silkscreen_over_mask_openings: r.silkscreen_over_mask_openings,
        };
        let result = self.board.check_rules(&set);
        for d in &result.confirmed {
            if let Some(finding) = self.confirmed_from(d) {
                self.report.confirmed.push(finding);
            }
        }
        for d in &result.inconclusive {
            if let Some(finding) = Self::inconclusive_from(d) {
                self.report.inconclusive.push(finding);
            }
        }
    }

    /// The part of a board crate message after its title.
    fn detail_of(d: &Diagnostic) -> &str {
        d.message
            .split_once(": ")
            .map_or(d.message.as_str(), |(_, rest)| rest)
    }

    /// True for the findings about a track or a via alone, which this engine makes itself.
    fn is_own(d: &Diagnostic) -> bool {
        matches!(
            d.code.as_deref(),
            Some(
                board_rules::CLEARANCE_VIOLATED
                    | board_rules::CLEARANCE_INCONCLUSIVE
                    | board_rules::EDGE_TOO_CLOSE
            )
        ) && !Self::detail_of(d).contains(" pad ")
    }

    fn confirmed_from(&self, d: &Diagnostic) -> Option<Finding> {
        if Self::is_own(d) {
            return None;
        }
        let r = self.rules;
        let (rule, limit) = match d.code.as_deref()? {
            board_rules::CLEARANCE_VIOLATED => (Rule::CopperClearance, Some(r.min_clearance)),
            board_rules::EDGE_TOO_CLOSE => (Rule::EdgeClearance, Some(r.min_edge_clearance)),
            board_rules::MASK_BRIDGE_THIN => (Rule::MaskBridge, Some(r.min_mask_bridge)),
            board_rules::ZONE_CLEARANCE_SMALL => (Rule::ZoneClearance, Some(r.min_clearance)),
            board_rules::ZONES_OVERLAP => (Rule::ZonesOverlap, None),
            board_rules::SILK_OVER_PAD => (Rule::SilkOverPad, None),
            board_rules::SILK_OVER_MASK => (Rule::SilkOverMask, None),
            _ => return None,
        };
        let mut finding = make(rule, severity_of(d), Self::detail_of(d).to_owned());
        if let Some(limit) = limit {
            finding = finding.limit(limit.get());
        }
        Some(finding)
    }

    fn inconclusive_from(d: &Diagnostic) -> Option<Finding> {
        if Self::is_own(d) {
            return None;
        }
        let title = d.message.split_once(": ").map_or("", |(t, _)| t);
        Some(
            make(
                Rule::NotJudged,
                severity_of(d),
                Self::detail_of(d).to_owned(),
            )
            .why(title),
        )
    }
}

fn severity_of(d: &Diagnostic) -> Severity {
    match d.severity {
        DiagSeverity::Error => Severity::Error,
        DiagSeverity::Warning => Severity::Warning,
        DiagSeverity::Info => Severity::Info,
    }
}
