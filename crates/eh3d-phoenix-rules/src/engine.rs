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

use eh3d_phoenix_spatial::{inflate, Bx, Index, SpatialConfig, SpatialError};

use crate::advanced::IndexMode;
use crate::codes;
use crate::geometry::{outline_edges, pad_min_side, Dist2, Near, Prim};
use crate::items::{pad_prim, placed_pads, track_prim, via_prim, PlacedPad};
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
    /// Copper to the wall of a hole that is not plated.
    HoleClearance,
    /// The wall of a drill hole to the board edge.
    HoleEdge,
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
            // 14 to 16 are the checks of the net classes
            Rule::HoleClearance => (17, "hole_clearance", codes::HOLE_CLEARANCE),
            Rule::HoleEdge => (18, "hole_edge", codes::HOLE_EDGE),
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
    layers: Vec<Id>,
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
    mode: IndexMode,
    pads: Vec<PlacedPad>,
    copper: Vec<Copper>,
    report: Report,
}

/// The tracks and then the vias of a board as copper that can touch other copper.
fn copper_of(board: &Board) -> Vec<Copper> {
    let mut copper = Vec::new();
    for (id, track) in &board.tracks {
        if let Some(prim) = track_prim(track) {
            copper.push(Copper {
                rank: 0,
                id: *id,
                net: track.net,
                layers: vec![track.layer],
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
                layers: board.via_layers(via),
                prim,
                label: format!("via {id}"),
            });
        }
    }
    copper
}

/// The boxes that find the pairs of copper to compare: the box of the end points of each item,
/// grown by half of its widest reading, which is how far the item reaches.
fn grown_boxes(copper: &[Copper]) -> Vec<Bx> {
    copper
        .iter()
        .map(|c| inflate(&c.prim.bbox(), c.prim.hi / 2 + 1))
        .collect()
}

/// The way to find the neighbours of each box: the grid, or the comparison of every pair. A grid
/// that the settings refuse is replaced by the comparison of every pair, which gives the same
/// result more slowly; the refusal is returned.
fn clearance_index(
    boxes: &[Bx],
    mode: IndexMode,
    config: &SpatialConfig,
) -> (Index, Option<SpatialError>) {
    match mode {
        IndexMode::Naive => (Index::naive(boxes.len()), None),
        IndexMode::Grid => match Index::grid(boxes, config) {
            Ok(index) => (index, None),
            Err(refused) => (Index::naive(boxes.len()), Some(refused)),
        },
    }
}

/// The coordinates (in nanometres, as a distance from the origin) up to which the distances of the
/// copper to the pieces of the edge of the board cannot overflow the exact test, so that leaving
/// out the pieces that are far away cannot hide a piece that could not be judged. Beyond it the
/// whole outline is compared, as it always was.
const EDGE_INDEX_SAFE_NM: i128 = 1 << 29;

/// The index of the pieces of the outline, for a board whose outline has at least
/// `edge_index_min_edges` pieces of the settings, in the grid mode and with every coordinate
/// within the range where nothing can overflow; else `None`, and every piece is compared.
fn edge_index_of(edges: &[Prim], mode: IndexMode, config: &SpatialConfig) -> Option<Index> {
    if mode != IndexMode::Grid || edges.len() < config.edge_index_min_edges as usize {
        return None;
    }
    let boxes: Vec<Bx> = edges.iter().map(|e| inflate(&e.bbox(), 1)).collect();
    if boxes
        .iter()
        .any(|b| b.iter().any(|v| v.abs() > EDGE_INDEX_SAFE_NM))
    {
        return None;
    }
    Index::grid(&boxes, config).ok()
}

/// The pieces of the outline that an item of copper may be too near to when the edge clearance is
/// `e`, in ascending order; `None` when the item is out of the range where this is safe and every
/// piece has to be compared.
fn edge_candidates(index: &Index, item: &Prim, e: i128) -> Option<Vec<usize>> {
    let query = inflate(&item.bbox(), e + item.lo / 2 + 2);
    if query.iter().any(|v| v.abs() > EDGE_INDEX_SAFE_NM) {
        return None;
    }
    Some(index.candidates(&query))
}

/// How much work the edge clearance check does on a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeWork {
    /// The pieces of copper that are compared with the outline.
    pub items: usize,
    /// The pieces of the outline and of its openings.
    pub edges: usize,
    /// Whether the pieces of the outline were put in an index (not when there are fewer than
    /// `edge_index_min_edges`, in the comparison of every pair, or when the settings refuse it).
    pub indexed: bool,
    /// The pairs of a piece of copper and a piece of the outline that the check had to look at.
    pub comparisons: u64,
}

/// The work of the edge clearance check on a board, counted and not timed: the same search as
/// the check makes, with the pairs counted instead of judged. A rule file with no edge clearance
/// turns the check off, and then there is no work.
pub fn edge_work(
    board: &Board,
    rules: &RuleValues,
    mode: IndexMode,
    config: &SpatialConfig,
) -> EdgeWork {
    let copper = copper_of(board);
    let limit = rules.min_edge_clearance;
    let edges = outline_edges(board);
    let mut work = EdgeWork {
        items: copper.len(),
        edges: edges.len(),
        indexed: false,
        comparisons: 0,
    };
    if !on(limit) || edges.is_empty() {
        return work;
    }
    let e = i128::from(limit.get());
    let index = edge_index_of(&edges, mode, config);
    work.indexed = index.is_some();
    for item in &copper {
        work.comparisons += match index
            .as_ref()
            .and_then(|index| edge_candidates(index, &item.prim, e))
        {
            Some(found) => found.len() as u64,
            None => edges.len() as u64,
        };
    }
    work
}

/// How much work the copper clearance check does on a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearanceWork {
    /// The tracks and vias that are compared.
    pub items: usize,
    /// The candidates that the search handed out: the pairs the check had to look at (each pair
    /// is seen from both items).
    pub comparisons: u64,
    /// The size of a cell in nanometres (1 when every pair is compared).
    pub cell_nm: i128,
    /// The cells that hold an item.
    pub cells_used: usize,
    /// The entries of the grid.
    pub entries: u64,
    /// The code of the refusal when the settings would not let the grid be built.
    pub refused: Option<&'static str>,
}

/// The work of the copper clearance check on a board, counted and not timed: the same search as
/// the check makes, with the candidates counted instead of judged. A rule file with no clearance
/// turns the check off, and then there is no work.
pub fn clearance_work(
    board: &Board,
    rules: &RuleValues,
    mode: IndexMode,
    config: &SpatialConfig,
) -> ClearanceWork {
    let copper = copper_of(board);
    let limit = rules.min_clearance;
    if !on(limit) {
        return ClearanceWork {
            items: copper.len(),
            comparisons: 0,
            cell_nm: 1,
            cells_used: 0,
            entries: 0,
            refused: None,
        };
    }
    let c = i128::from(limit.get());
    let boxes = grown_boxes(&copper);
    let (index, refused) = clearance_index(&boxes, mode, config);
    for b in &boxes {
        let _ = index.candidates(&inflate(b, c));
    }
    ClearanceWork {
        items: copper.len(),
        comparisons: index.examined(),
        cell_nm: index.cell_nm(),
        cells_used: index.cells_used(),
        entries: index.entries(),
        refused: refused.map(|e| e.code),
    }
}

/// Checks a board against the values of a rule file.
pub fn check_board(board: &Board, rules: &RuleValues) -> Report {
    check_board_with(board, rules, IndexMode::Grid)
}

/// Checks a board with the chosen way of finding the pieces of copper that are close. Both ways
/// give the same report, finding for finding and in the same order.
pub fn check_board_with(board: &Board, rules: &RuleValues, mode: IndexMode) -> Report {
    let copper = copper_of(board);
    let mut run = Run {
        board,
        rules,
        mode,
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
    run.hole_clearances();
    run.hole_edges();
    run.add_board_crate_findings();
    run.report.finish();
    run.report
}

/// Why a distance could not be judged.
pub(crate) fn why_unjudged(near: Near) -> &'static str {
    match near {
        Near::Maybe => "too close to the limit near an arc or a shape that is read in two ways",
        _ => "numbers too large for the exact test",
    }
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
                        "the track has an arc with no shape: its clearance is judged by its end points only",
                    ),
                );
            }
        }
        let boxes = grown_boxes(&self.copper);
        let (index, _) = clearance_index(&boxes, self.mode, &SpatialConfig::default());
        for (i, a) in self.copper.iter().enumerate() {
            for j in index.candidates(&inflate(&boxes[i], c)) {
                if j <= i {
                    continue;
                }
                let b = &self.copper[j];
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
                let near = a.prim.within(&b.prim, c, false);
                match (near, a.prim.distance(&b.prim)) {
                    (Near::Yes, Some(d)) => self.report.confirmed.push(
                        make(Rule::CopperClearance, Severity::Error, location)
                            .at(at.x_nm.get(), at.y_nm.get())
                            .values(gap_nm(d, a.prim.lo, b.prim.lo), limit.get()),
                    ),
                    (Near::No, _) => {}
                    (near, _) => self.report.inconclusive.push(
                        make(Rule::NotJudged, Severity::Info, location)
                            .at(at.x_nm.get(), at.y_nm.get())
                            .limit(limit.get())
                            .why(why_unjudged(near)),
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
        let edge_index = edge_index_of(&edges, self.mode, &SpatialConfig::default());
        let every_edge: Vec<usize> = (0..edges.len()).collect();
        for item in &self.copper {
            let at = item.prim.first_point();
            let mut broken = false;
            let mut unknown = None;
            let mut nearest = f64::MAX;
            let near;
            let to_compare: &[usize] = match edge_index
                .as_ref()
                .and_then(|index| edge_candidates(index, &item.prim, e))
            {
                Some(found) => {
                    near = found;
                    &near
                }
                None => &every_edge,
            };
            for edge in to_compare.iter().map(|i| &edges[*i]) {
                let Some(d) = item.prim.distance(edge) else {
                    unknown = Some(Near::Unknown);
                    continue;
                };
                nearest = nearest.min(d.nm_f64());
                match item.prim.within(edge, e, false) {
                    Near::Yes => broken = true,
                    Near::No => {}
                    other => unknown = Some(other),
                }
            }
            if broken {
                let gap = (nearest - item.prim.lo as f64 / 2.0).round().max(0.0) as i64;
                self.report.confirmed.push(
                    make(Rule::EdgeClearance, Severity::Error, item.label.clone())
                        .at(at.x_nm.get(), at.y_nm.get())
                        .values(gap, limit.get()),
                );
            } else if let Some(near) = unknown {
                self.report.inconclusive.push(
                    make(Rule::NotJudged, Severity::Info, item.label.clone())
                        .at(at.x_nm.get(), at.y_nm.get())
                        .limit(limit.get())
                        .why(why_unjudged(near)),
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

    /// Tracks, vias and pads against the wall of every hole that is not plated: it has no copper,
    /// so no other clearance sees it, and copper near it is cut when the hole is drilled.
    fn hole_clearances(&mut self) {
        let limit = self.rules.min_hole_clearance;
        if !on(limit) {
            return;
        }
        let reach = i128::from(limit.get());
        let pads: Vec<(String, Prim)> = self
            .pads
            .iter()
            .filter(|p| p.drill.is_none() || p.plated)
            .filter_map(|p| pad_prim(p).map(|prim| (p.label.clone(), prim)))
            .collect();
        for hole in self.pads.iter().filter(|p| !p.plated) {
            let Some(drill) = hole.drill else {
                continue;
            };
            let width = i128::from(drill.get());
            let Some(wall) = Prim::at_point(hole.at, width, width) else {
                continue;
            };
            let copper = self
                .copper
                .iter()
                .map(|c| (&c.label, &c.prim))
                .chain(pads.iter().map(|(label, prim)| (label, prim)));
            for (label, prim) in copper {
                if !prim.may_be_within(&wall, reach) {
                    continue;
                }
                let location = format!("{label} and the hole of {}", hole.label);
                let (x, y) = (hole.at.x_nm.get(), hole.at.y_nm.get());
                match (prim.within(&wall, reach, false), prim.distance(&wall)) {
                    (Near::Yes, Some(d)) => self.report.confirmed.push(
                        make(Rule::HoleClearance, Severity::Error, location)
                            .at(x, y)
                            .values(gap_nm(d, prim.lo, width), limit.get()),
                    ),
                    (Near::No, _) => {}
                    (near, _) => self.report.inconclusive.push(
                        make(Rule::NotJudged, Severity::Info, location)
                            .at(x, y)
                            .limit(limit.get())
                            .why(why_unjudged(near)),
                    ),
                }
            }
        }
    }

    /// The wall of every drill hole (of a via or a pad, plated or not) against the edge of the
    /// board and of its openings.
    fn hole_edges(&mut self) {
        let limit = self.rules.min_hole_edge;
        if !on(limit) {
            return;
        }
        let edges = outline_edges(self.board);
        if edges.is_empty() {
            self.report.inconclusive.push(
                make(Rule::NotJudged, Severity::Info, "board outline".to_owned())
                    .limit(limit.get())
                    .why("the board has no closed outline, so the holes are not judged against it"),
            );
            return;
        }
        let reach = i128::from(limit.get());
        let mut holes: Vec<(String, Point, i128)> = self
            .board
            .vias
            .iter()
            .map(|(id, via)| (format!("via {id}"), via.at, i128::from(via.drill.get())))
            .collect();
        holes.extend(self.pads.iter().filter_map(|p| {
            p.drill
                .map(|drill| (p.label.clone(), p.at, i128::from(drill.get())))
        }));
        for (label, at, drill) in holes {
            let Some(wall) = Prim::at_point(at, drill, drill) else {
                continue;
            };
            let mut broken: Option<i64> = None;
            let mut unknown = false;
            for edge in edges.iter().filter(|e| wall.may_be_within(e, reach)) {
                match (wall.within(edge, reach, false), wall.distance(edge)) {
                    (Near::Yes, Some(d)) => {
                        let gap = gap_nm(d, drill, 0);
                        broken = Some(broken.map_or(gap, |found| found.min(gap)));
                    }
                    (Near::No, _) => {}
                    _ => unknown = true,
                }
            }
            let (x, y) = (at.x_nm.get(), at.y_nm.get());
            if let Some(gap) = broken {
                self.report.confirmed.push(
                    make(Rule::HoleEdge, Severity::Error, label)
                        .at(x, y)
                        .values(gap, limit.get()),
                );
            } else if unknown {
                self.report.inconclusive.push(
                    make(Rule::NotJudged, Severity::Info, label)
                        .at(x, y)
                        .limit(limit.get())
                        .why("numbers too large for the exact test"),
                );
            }
        }
    }

    /// The checks that the board crate already has for pads, the solder mask, zones and
    /// silkscreen. The clearance and edge findings it makes for tracks and vias are left out
    /// because this engine measures them itself.
    fn add_board_crate_findings(&mut self) {
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
        // the clearances of tracks and vias are measured above with the index, and the findings of
        // the pass over every pair would be dropped below, so that pass is not made
        let result = self.board.check_rules_with(&set, false);
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
