//! More rule checks: pads, the edge of the board, the solder mask, and zones.
//!
//! The pads are not points: a pad has a shape. The shapes are bounded by two circles about the
//! centre of the pad, the largest circle inside it (its inscribed circle) and the smallest circle
//! around it. Two pads are certainly too close when even their inscribed circles are, and certainly
//! far enough when even their circumscribed circles are; in between a round pad is exact and any
//! other pad is reported as *inconclusive* (`PHOENIX-RULE-009`), never as a failure. All the
//! arithmetic is in integer nanometres.
//!
//! Silkscreen over pads is checked on the strokes that are really printed (the lines of a stroke
//! item and of every character of a text, with the width of the printed line) against the pads that
//! show on the side of the silkscreen layer; what cannot be judged (a layer without a side, a
//! board without a copper stack and a pad without a drill, an arc of a stroke) is said once with
//! `PHOENIX-RULE-010`, never as a failure.
//!
//! The checks of a pad against the other pads, the tracks and the vias would compare every pad with
//! every object; they look the neighbours up in a grid of cells as large as the reach of the rule
//! instead (see [`Grid`]), so that the time grows with the number of objects and not with its
//! square. The grid only leaves out the objects that are certainly too far to give a finding, so the
//! findings, their order and their text are those of the pass over every pair, which stays for the
//! boards whose reach is too large for a grid and for the tests that compare the two.

use std::collections::HashMap;

use eh3d_diagnostic::Severity;
use eh3d_phoenix_model::{segments_intersect, Nm, Point};

use crate::connectivity::{near_segment_exact, pad_position, point_in_polygon};
use crate::model::{Board, Id, LayerKind, PadRef, PadShape, Segment, Side, Track, Via};
use crate::outline_arc::{arc_chords, ArcSettings};
use crate::rules::{
    make, RuleReport, RuleSet, CLEARANCE_INCONCLUSIVE, CLEARANCE_VIOLATED, EDGE_TOO_CLOSE,
    MASK_BRIDGE_THIN, NOT_CHECKABLE, PAD_SHAPE_UNSURE, SILK_OVER_MASK, SILK_OVER_PAD,
    ZONES_OVERLAP, ZONE_CLEARANCE_SMALL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Violation,
    Fine,
    Unsure,
}

struct PadInfo {
    label: String,
    at: Point,
    layers: Vec<Id>,
    net: Option<Id>,
    /// Twice the radius of the inscribed circle (a diameter) and of the circumscribed circle.
    ins2: i128,
    circ2: i128,
    /// A through-hole pad shows on both sides.
    through: bool,
}

/// Smallest `r` with `r * r >= n`.
fn isqrt_ceil(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut r = (n as f64).sqrt() as i128;
    while r * r < n {
        r += 1;
    }
    while r > 0 && (r - 1) * (r - 1) >= n {
        r -= 1;
    }
    r
}

fn diameters(shape: PadShape) -> (i128, i128) {
    match shape {
        PadShape::Circle { diameter } => (i128::from(diameter.get()), i128::from(diameter.get())),
        PadShape::Rect { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            (w.min(h), isqrt_ceil(w * w + h * h))
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            (w.min(h), w.max(h))
        }
    }
}

fn pads(board: &Board) -> Vec<PadInfo> {
    let mut out = Vec::new();
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            if pad.is_npth() {
                // a hole that is not plated has no copper to keep apart from other copper
                continue;
            }
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let (ins2, circ2) = diameters(pad.shape);
            out.push(PadInfo {
                label: format!("{} pad {}", component.refdes, pad.number),
                at,
                layers: pad.layers.clone(),
                net: board.net_of(pad_ref),
                ins2,
                circ2,
                through: pad.drill.is_some(),
            });
        }
    }
    out
}

fn share_layer(a: &[Id], b: &[Id], filter: impl Fn(&Id) -> bool) -> bool {
    a.iter().any(|l| filter(l) && b.contains(l))
}

fn is_copper(board: &Board, layer: &Id) -> bool {
    board
        .layers
        .get(layer)
        .is_some_and(|l| l.kind == LayerKind::Copper)
}

fn is_mask(board: &Board, layer: &Id) -> bool {
    board
        .layers
        .get(layer)
        .is_some_and(|l| l.kind == LayerKind::SolderMask)
}

/// Compares the squared distance (times four) of two centres with the two reach values.
fn gap_points(dist2x4: i128, t_ins: i128, t_circ: i128) -> Verdict {
    if t_ins > 0 && dist2x4 < t_ins * t_ins {
        Verdict::Violation
    } else if dist2x4 >= t_circ * t_circ {
        Verdict::Fine
    } else {
        Verdict::Unsure
    }
}

fn centre_distance2x4(a: Point, b: Point) -> i128 {
    let dx = i128::from(a.x_nm.get()) - i128::from(b.x_nm.get());
    let dy = i128::from(a.y_nm.get()) - i128::from(b.y_nm.get());
    4 * (dx * dx + dy * dy)
}

/// The pad centre against a straight piece, for the two reach values.
fn gap_segment(at: Point, s: Point, e: Point, t_ins: i128, t_circ: i128) -> Verdict {
    if t_ins > 0 && near_segment_exact(at, s, e, t_ins * t_ins, true) == Some(true) {
        return Verdict::Violation;
    }
    if near_segment_exact(at, s, e, t_circ * t_circ, true) == Some(false) {
        Verdict::Fine
    } else {
        Verdict::Unsure
    }
}

/// Two straight pieces closer than `reach` (twice the allowed centre distance)? `None` when the
/// numbers do not fit.
fn pieces_too_close(a: (Point, Point), b: (Point, Point), reach: i128) -> Option<bool> {
    if segments_intersect(a.0, a.1, b.0, b.1) {
        return Some(true);
    }
    let mut known = true;
    for (p, s, e) in [
        (a.0, b.0, b.1),
        (a.1, b.0, b.1),
        (b.0, a.0, a.1),
        (b.1, a.0, a.1),
    ] {
        match near_segment_exact(p, s, e, reach * reach, true) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => known = false,
        }
    }
    known.then_some(false)
}

/// True when the arc from `a` to `b` (sweep in micro-degrees) printed with line width `w` is
/// provably clear of the pad. An arc of at most half a turn lies inside the disc that has its
/// chord as a diameter, so it is clear when the centre of that disc is farther from the pad than
/// the sum of the disc radius, the half line width and the radius of the circle around the pad.
/// Everything is in integers, with an upper bound for the chord length (the sum of the
/// differences of the coordinates); an arc of more than half a turn is never judged clear.
fn arc_clear_of_pad(pad: &PadInfo, a: Point, b: Point, sweep: i64, w: i128) -> bool {
    if sweep.abs() > 180_000_000 {
        return false;
    }
    let (ax, ay) = (i128::from(a.x_nm.get()), i128::from(a.y_nm.get()));
    let (bx, by) = (i128::from(b.x_nm.get()), i128::from(b.y_nm.get()));
    let upper_chord = (ax - bx).abs() + (ay - by).abs();
    // twice the centre of the disc, against twice the pad centre
    let dx = 2 * i128::from(pad.at.x_nm.get()) - (ax + bx);
    let dy = 2 * i128::from(pad.at.y_nm.get()) - (ay + by);
    let reach = upper_chord + w + pad.circ2;
    dx * dx + dy * dy >= reach * reach
}

fn lines(path: &[Segment]) -> Vec<(Point, Point)> {
    path.iter()
        .filter_map(|s| match s {
            Segment::Line { start, end } => Some((*start, *end)),
            Segment::Arc { .. } => None,
        })
        .collect()
}

fn push(
    report: &mut RuleReport,
    verdict: Verdict,
    violation: (&'static str, String),
    unsure: String,
) {
    match verdict {
        Verdict::Violation => {
            report
                .confirmed
                .push(make(violation.0, Severity::Error, violation.1))
        }
        Verdict::Unsure => report
            .inconclusive
            .push(make(PAD_SHAPE_UNSURE, Severity::Info, unsure)),
        Verdict::Fine => {}
    }
}

fn worst(a: Verdict, b: Verdict) -> Verdict {
    match (a, b) {
        (Verdict::Violation, _) | (_, Verdict::Violation) => Verdict::Violation,
        (Verdict::Unsure, _) | (_, Verdict::Unsure) => Verdict::Unsure,
        _ => Verdict::Fine,
    }
}

/// The work of the clearance check of the pads on a board, counted (see
/// [`Board::pad_check_work`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadCheckWork {
    /// The pads that show copper.
    pub pads: usize,
    /// The tracks.
    pub tracks: usize,
    /// The vias.
    pub vias: usize,
    /// The pads after a pad, the tracks and the vias that the grids hand out to the pads, added
    /// up: the objects the exact test is made for.
    pub candidates: u64,
    /// The comparisons of the pass over every pair: each pad with each later pad, each track and
    /// each via.
    pub all_pairs: u64,
}

/// A piece of copper of a track: a straight piece, or a chord of an arc with how far the arc may
/// be from it.
type Piece = (Point, Point, i128);

/// A track with the pieces of its path.
type TrackPieces<'a> = (&'a Id, &'a Track, Vec<Piece>);

/// The pieces of a path that a pad is measured against: the straight pieces as they are and the
/// arcs as chords (an arc with no shape as its two end points).
fn copper_pieces(path: &[Segment]) -> Vec<Piece> {
    let mut out = Vec::new();
    for segment in path {
        match *segment {
            Segment::Line { start, end } => out.push((start, end, 0)),
            Segment::Arc { start, end, sweep } => {
                match arc_chords(start, end, sweep, &ArcSettings::current()) {
                    Some((points, doubt)) => {
                        out.extend(points.windows(2).map(|w| (w[0], w[1], i128::from(doubt))))
                    }
                    None => out.extend([(start, start, 0), (end, end, 0)]),
                }
            }
        }
    }
    out
}

/// What the clearance check of the pads looks things up in.
struct PadIndex<'a> {
    pads: Grid,
    vias: Grid,
    via_list: Vec<(&'a Id, &'a Via)>,
    track_grid: Grid,
    /// The tracks with a piece that is too long for the cells, tried for every pad.
    long: Vec<usize>,
    tracks: Vec<TrackPieces<'a>>,
}

impl PadIndex<'_> {
    /// The tracks that can be near a point, in increasing order, once each.
    fn tracks_near(&self, p: Point) -> Vec<usize> {
        let mut near = self.track_grid.at(p);
        near.extend(self.long.iter().copied());
        near.sort_unstable();
        near.dedup();
        near
    }
}

/// The largest reach, in nanometres, for which the grid is used. A larger one (a clearance of
/// hundreds of millimetres) makes every cell hold most of the board, and the exact tests could not
/// fit their numbers any more, so the pass over every pair is made.
const GRID_LIMIT_NM: i128 = 1 << 28;

/// The most cells one piece of track may be put in; a longer piece is kept apart and tried for
/// every pad.
const MAX_CELLS_PER_PIECE: i64 = 256;

/// A grid of square cells over the board, to find what is near a point without looking at
/// everything. A cell is as large as the reach of the rule (the distance beyond which two objects
/// are certainly clear of each other), so everything within the reach of a point is in the nine
/// cells around it (for points) or in its own cell (for pieces put in every cell their box, grown
/// by the reach, covers). Cells are looked up by key: the order of a hash map never shows, because
/// the indexes found are sorted before use.
struct Grid {
    cell: i64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl Grid {
    /// A grid of cells `reach` wide; `None` when the reach is not above zero or above the limit.
    fn new(reach: i128) -> Option<Grid> {
        if !(1..=GRID_LIMIT_NM).contains(&reach) {
            return None;
        }
        Some(Grid {
            cell: i64::try_from(reach).ok()?,
            cells: HashMap::new(),
        })
    }

    fn index_of(&self, v: i64) -> i64 {
        v.div_euclid(self.cell)
    }

    fn add_point(&mut self, p: Point, index: usize) {
        let key = (self.index_of(p.x_nm.get()), self.index_of(p.y_nm.get()));
        self.cells.entry(key).or_default().push(index);
    }

    /// Puts a straight piece in every cell its box, grown by `reach`, covers. False (and nothing
    /// put) when that would be more than [`MAX_CELLS_PER_PIECE`] cells.
    fn add_piece(&mut self, s: Point, e: Point, reach: i128, index: usize) -> bool {
        let reach = i64::try_from(reach).unwrap_or(i64::MAX / 4);
        let lo = |a: i64, b: i64| a.min(b).saturating_sub(reach);
        let hi = |a: i64, b: i64| a.max(b).saturating_add(reach);
        let (x0, x1) = (
            self.index_of(lo(s.x_nm.get(), e.x_nm.get())),
            self.index_of(hi(s.x_nm.get(), e.x_nm.get())),
        );
        let (y0, y1) = (
            self.index_of(lo(s.y_nm.get(), e.y_nm.get())),
            self.index_of(hi(s.y_nm.get(), e.y_nm.get())),
        );
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > MAX_CELLS_PER_PIECE {
            return false;
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                self.cells.entry((x, y)).or_default().push(index);
            }
        }
        true
    }

    /// The indexes in the nine cells around a point, in increasing order, once each.
    fn around(&self, p: Point) -> Vec<usize> {
        let (cx, cy) = (self.index_of(p.x_nm.get()), self.index_of(p.y_nm.get()));
        let mut found = Vec::new();
        for x in cx - 1..=cx + 1 {
            for y in cy - 1..=cy + 1 {
                if let Some(list) = self.cells.get(&(x, y)) {
                    found.extend_from_slice(list);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The indexes in the cell of a point, unsorted and with repeats.
    fn at(&self, p: Point) -> Vec<usize> {
        let key = (self.index_of(p.x_nm.get()), self.index_of(p.y_nm.get()));
        self.cells.get(&key).cloned().unwrap_or_default()
    }
}

impl Board {
    /// The checks of pads, the edge of the board, the mask and zones. Called by `check_rules`.
    pub(crate) fn check_more_rules(&self, rules: &RuleSet, report: &mut RuleReport) {
        let all_pads = pads(self);
        if rules.min_clearance.get() > 0 {
            self.pad_clearances(rules.min_clearance, &all_pads, report);
        }
        if rules.min_edge_clearance.get() > 0 {
            self.edge_clearances(rules.min_edge_clearance, &all_pads, report);
        }
        if rules.min_mask_bridge.get() > 0 {
            self.mask_bridges(rules.min_mask_bridge, &all_pads, report);
        }
        if rules.min_clearance.get() > 0 {
            self.zone_rules(rules.min_clearance, report);
        }
        if rules.silkscreen_over_pads {
            self.silk_over_pads(&all_pads, report, false);
        }
        if rules.silkscreen_over_mask_openings {
            self.silk_over_pads(&all_pads, report, true);
        }
    }

    /// Silkscreen printed over pads that show on the same side. An arc is only judged to be clear
    /// of a pad (see [`arc_clear_of_pad`]); when it is not provably clear the result is
    /// inconclusive, never a failure.
    fn silk_over_pads(&self, all_pads: &[PadInfo], report: &mut RuleReport, mask: bool) {
        let code = if mask { SILK_OVER_MASK } else { SILK_OVER_PAD };
        let (mut no_side, mut no_stack) = (false, false);
        for (sid, item) in &self.silkscreen {
            let Some(layer) = self.layers.get(&item.layer()) else {
                continue;
            };
            if layer.kind != LayerKind::Silkscreen {
                continue;
            }
            let Some(side) = layer.side else {
                no_side = true;
                continue;
            };
            // an item that cannot be drawn is a problem of the board, reported by `validate`
            let Ok(drawn) = item.drawn() else {
                continue;
            };
            let outer = match side {
                Side::Top => self.copper_stack.first(),
                Side::Bottom => self.copper_stack.last(),
            };
            let pieces = lines(&drawn.pieces);
            let arcs: Vec<(Point, Point, i64)> = drawn
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Segment::Arc { start, end, sweep } => Some((*start, *end, sweep.get())),
                    Segment::Line { .. } => None,
                })
                .collect();
            let w = i128::from(drawn.width.get());
            for pad in all_pads {
                let shows = if mask {
                    // the opening is where the pad has a mask layer of the side of the silkscreen
                    pad.layers.iter().any(|l| {
                        self.layers.get(l).is_some_and(|x| {
                            x.kind == LayerKind::SolderMask && x.side == Some(side)
                        })
                    })
                } else if pad.through {
                    true
                } else if let Some(outer) = outer {
                    pad.layers.contains(outer)
                } else {
                    no_stack = true;
                    false
                };
                if !shows {
                    continue;
                }
                let mut verdict = Verdict::Fine;
                for (s, e) in &pieces {
                    verdict = worst(
                        verdict,
                        gap_segment(pad.at, *s, *e, w + pad.ins2, w + pad.circ2),
                    );
                }
                let arc_unsure = arcs
                    .iter()
                    .any(|(a, b, sweep)| !arc_clear_of_pad(pad, *a, *b, *sweep, w));
                if verdict != Verdict::Fine {
                    push(
                        report,
                        verdict,
                        (code, format!("item {sid} over {}", pad.label)),
                        format!("silkscreen item {sid} and {}", pad.label),
                    );
                }
                if arc_unsure && verdict == Verdict::Fine {
                    report.inconclusive.push(make(
                        CLEARANCE_INCONCLUSIVE,
                        Severity::Info,
                        format!("silkscreen item {sid} has an arc near {}", pad.label),
                    ));
                }
            }
        }
        for (flag, why) in [
            (no_side, "a silkscreen layer does not say which side it is on"),
            (
                no_stack,
                "the board does not give the copper stack, so the side of a pad without a drill is not known",
            ),
        ] {
            if flag {
                report.inconclusive.push(make(
                    NOT_CHECKABLE,
                    Severity::Info,
                    format!("silkscreen over pads: {why}"),
                ));
            }
        }
    }

    /// The grids of the clearance check of the pads: the pads, the vias and the pieces of track.
    /// `None` when the reach of the rule is too large for a grid.
    fn pad_index<'a>(&'a self, clearance: Nm, all_pads: &[PadInfo]) -> Option<PadIndex<'a>> {
        let c2 = 2 * i128::from(clearance.get());
        let max_circ = all_pads.iter().map(|p| p.circ2).max().unwrap_or(0);
        let max_via = self
            .vias
            .values()
            .map(|v| i128::from(v.diameter.get()))
            .max()
            .unwrap_or(0);
        let max_width = self
            .tracks
            .values()
            .map(|t| i128::from(t.width.get()))
            .max()
            .unwrap_or(0);
        // two pads, a pad and a via, a pad and a piece of track are certainly clear of each other
        // when their centres (or the centre and the piece) are this far apart, in nanometres
        let reach_pad = (c2 + 2 * max_circ) / 2 + 1;
        let reach_via = (c2 + max_via + max_circ) / 2 + 1;
        let reach_track = (c2 + max_width + max_circ) / 2 + 1;
        let mut pads = Grid::new(reach_pad)?;
        let mut via_grid = Grid::new(reach_via)?;
        let mut track_grid = Grid::new(reach_track)?;
        for (i, p) in all_pads.iter().enumerate() {
            pads.add_point(p.at, i);
        }
        let vias: Vec<_> = self.vias.iter().collect();
        for (k, (_, v)) in vias.iter().enumerate() {
            via_grid.add_point(v.at, k);
        }
        // the pieces of every track, made once and not again for every pad
        let tracks: Vec<_> = self
            .tracks
            .iter()
            .map(|(tid, t)| (tid, t, copper_pieces(&t.path)))
            .collect();
        let mut long: Vec<usize> = Vec::new();
        for (k, (_, _, pieces)) in tracks.iter().enumerate() {
            let mut in_cells = true;
            for (s, e, doubt) in pieces {
                in_cells &= track_grid.add_piece(*s, *e, reach_track + doubt, k);
            }
            if !in_cells {
                long.push(k);
            }
        }
        Some(PadIndex {
            pads,
            vias: via_grid,
            via_list: vias,
            track_grid,
            long,
            tracks,
        })
    }

    /// The clearance of the pads to each other, to the tracks and to the vias, with the
    /// neighbours looked up in grids; the pass over every pair when the reach is too large.
    fn pad_clearances(&self, clearance: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let Some(index) = self.pad_index(clearance, all_pads) else {
            self.pad_clearances_all(clearance, all_pads, report);
            return;
        };
        let c2 = 2 * i128::from(clearance.get());
        for (i, a) in all_pads.iter().enumerate() {
            for j in index.pads.around(a.at) {
                if j <= i {
                    continue;
                }
                let b = &all_pads[j];
                let verdict = gap_points(
                    centre_distance2x4(a.at, b.at),
                    c2 + a.ins2 + b.ins2,
                    c2 + a.circ2 + b.circ2,
                );
                if verdict == Verdict::Fine
                    || (a.net.is_some() && a.net == b.net)
                    || !share_layer(&a.layers, &b.layers, |l| is_copper(self, l))
                {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and {}", a.label, b.label)),
                    format!("{} and {}", a.label, b.label),
                );
            }
            for k in index.tracks_near(a.at) {
                let (tid, t, pieces) = &index.tracks[k];
                if (a.net.is_some() && a.net == Some(t.net)) || !a.layers.contains(&t.layer) {
                    continue;
                }
                let w = i128::from(t.width.get());
                let mut verdict = Verdict::Fine;
                for (s, e, doubt) in pieces {
                    let (sure, widest) =
                        (c2 + w + a.ins2 - 2 * doubt, c2 + w + a.circ2 + 2 * doubt);
                    verdict = worst(verdict, gap_segment(a.at, *s, *e, sure, widest));
                }
                if verdict == Verdict::Fine {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and track {tid}", a.label)),
                    format!("{} and track {tid}", a.label),
                );
            }
            for k in index.vias.around(a.at) {
                let (vid, v) = index.via_list[k];
                if (a.net.is_some() && a.net == Some(v.net))
                    || !self.via_layers(v).iter().any(|l| a.layers.contains(l))
                {
                    continue;
                }
                let d = i128::from(v.diameter.get());
                let verdict = gap_points(
                    centre_distance2x4(a.at, v.at),
                    c2 + d + a.ins2,
                    c2 + d + a.circ2,
                );
                if verdict == Verdict::Fine {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and via {vid}", a.label)),
                    format!("{} and via {vid}", a.label),
                );
            }
        }
    }

    /// How much work the clearance check of the pads does on this board with this clearance,
    /// counted and not timed, so it is the same on every computer: how many objects the grids hand
    /// out to the pads (the other pads after it, the tracks and the vias) against the number of
    /// comparisons of the pass over every pair. `None` when the clearance is not above zero or its
    /// reach is too large for a grid (the pass over every pair is made then).
    pub fn pad_check_work(&self, clearance: Nm) -> Option<PadCheckWork> {
        if clearance.get() <= 0 {
            return None;
        }
        let all_pads = pads(self);
        let index = self.pad_index(clearance, &all_pads)?;
        let mut candidates = 0u64;
        for (i, a) in all_pads.iter().enumerate() {
            candidates += index.pads.around(a.at).iter().filter(|j| **j > i).count() as u64;
            candidates += index.tracks_near(a.at).len() as u64;
            candidates += index.vias.around(a.at).len() as u64;
        }
        let (p, t, v) = (
            all_pads.len() as u64,
            self.tracks.len() as u64,
            self.vias.len() as u64,
        );
        Some(PadCheckWork {
            pads: all_pads.len(),
            tracks: self.tracks.len(),
            vias: self.vias.len(),
            candidates,
            all_pairs: p * p.saturating_sub(1) / 2 + p * t + p * v,
        })
    }

    /// The same clearances by comparing every pad with every other pad, track and via.
    fn pad_clearances_all(&self, clearance: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let c2 = 2 * i128::from(clearance.get());
        // the pieces of every track, made once and not again for every pad
        let track_lines: Vec<_> = self
            .tracks
            .iter()
            .map(|(tid, t)| (tid, t, copper_pieces(&t.path)))
            .collect();
        for (i, a) in all_pads.iter().enumerate() {
            for b in &all_pads[i + 1..] {
                // the cheap test of the distance first; only a pair that is not far enough
                // apart gets the net and layer tests and a message
                let verdict = gap_points(
                    centre_distance2x4(a.at, b.at),
                    c2 + a.ins2 + b.ins2,
                    c2 + a.circ2 + b.circ2,
                );
                if verdict == Verdict::Fine
                    || (a.net.is_some() && a.net == b.net)
                    || !share_layer(&a.layers, &b.layers, |l| is_copper(self, l))
                {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and {}", a.label, b.label)),
                    format!("{} and {}", a.label, b.label),
                );
            }
            for (tid, t, pieces) in &track_lines {
                if (a.net.is_some() && a.net == Some(t.net)) || !a.layers.contains(&t.layer) {
                    continue;
                }
                let w = i128::from(t.width.get());
                let mut verdict = Verdict::Fine;
                for (s, e, doubt) in pieces {
                    let (sure, widest) =
                        (c2 + w + a.ins2 - 2 * doubt, c2 + w + a.circ2 + 2 * doubt);
                    verdict = worst(verdict, gap_segment(a.at, *s, *e, sure, widest));
                }
                if verdict == Verdict::Fine {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and track {tid}", a.label)),
                    format!("{} and track {tid}", a.label),
                );
            }
            for (vid, v) in &self.vias {
                if (a.net.is_some() && a.net == Some(v.net))
                    || !self.via_layers(v).iter().any(|l| a.layers.contains(l))
                {
                    continue;
                }
                let d = i128::from(v.diameter.get());
                let verdict = gap_points(
                    centre_distance2x4(a.at, v.at),
                    c2 + d + a.ins2,
                    c2 + d + a.circ2,
                );
                if verdict == Verdict::Fine {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (CLEARANCE_VIOLATED, format!("{} and via {vid}", a.label)),
                    format!("{} and via {vid}", a.label),
                );
            }
        }
    }

    /// The pieces of the board edge: the outline and the interior cutouts, each closed. A ring of
    /// fewer than three points is not an edge.
    fn edge_pieces(&self) -> Vec<(Point, Point)> {
        let mut edges: Vec<(Point, Point)> = Vec::new();
        // the arcs of the outline are measured as the chords they are cut into
        let outline = self.outline_polygon(&ArcSettings::current());
        for ring in std::iter::once(&outline).chain(self.outline_cutouts.iter()) {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            edges.extend((0..n).map(|i| (ring[i], ring[(i + 1) % n])));
        }
        edges
    }

    fn edge_clearances(&self, clearance: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let c2 = 2 * i128::from(clearance.get());
        let edges = self.edge_pieces();
        if edges.is_empty() {
            return;
        }
        for (tid, t) in &self.tracks {
            let reach = c2 + i128::from(t.width.get());
            let mut broken = false;
            let mut unknown = false;
            for piece in lines(&t.path) {
                for edge in &edges {
                    match pieces_too_close(piece, *edge, reach) {
                        Some(true) => broken = true,
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
            }
            if broken {
                report.confirmed.push(make(
                    EDGE_TOO_CLOSE,
                    Severity::Error,
                    format!("track {tid}"),
                ));
            } else if unknown {
                report.inconclusive.push(make(
                    CLEARANCE_INCONCLUSIVE,
                    Severity::Info,
                    format!("track {tid} and the edge: numbers too large for the exact test"),
                ));
            }
        }
        for (vid, v) in &self.vias {
            let reach = c2 + i128::from(v.diameter.get());
            let broken = edges
                .iter()
                .any(|(p, q)| near_segment_exact(v.at, *p, *q, reach * reach, true) == Some(true));
            if broken {
                report
                    .confirmed
                    .push(make(EDGE_TOO_CLOSE, Severity::Error, format!("via {vid}")));
            }
        }
        for a in all_pads {
            let mut verdict = Verdict::Fine;
            for (p, q) in &edges {
                verdict = worst(
                    verdict,
                    gap_segment(a.at, *p, *q, c2 + a.ins2, c2 + a.circ2),
                );
            }
            push(
                report,
                verdict,
                (EDGE_TOO_CLOSE, a.label.clone()),
                format!("{} and the edge", a.label),
            );
        }
    }

    /// The mask bridge between pads, with the neighbours looked up in a grid; the pass over every
    /// pair when the reach is too large.
    fn mask_bridges(&self, bridge: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let b2 = 2 * i128::from(bridge.get());
        let max_circ = all_pads.iter().map(|p| p.circ2).max().unwrap_or(0);
        let Some(mut grid) = Grid::new((b2 + 2 * max_circ) / 2 + 1) else {
            self.mask_bridges_all(bridge, all_pads, report);
            return;
        };
        for (i, p) in all_pads.iter().enumerate() {
            grid.add_point(p.at, i);
        }
        for (i, a) in all_pads.iter().enumerate() {
            for j in grid.around(a.at) {
                if j <= i {
                    continue;
                }
                let b = &all_pads[j];
                let verdict = gap_points(
                    centre_distance2x4(a.at, b.at),
                    b2 + a.ins2 + b.ins2,
                    b2 + a.circ2 + b.circ2,
                );
                if verdict == Verdict::Fine
                    || !share_layer(&a.layers, &b.layers, |l| is_mask(self, l))
                {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (MASK_BRIDGE_THIN, format!("{} and {}", a.label, b.label)),
                    format!("mask between {} and {}", a.label, b.label),
                );
            }
        }
    }

    /// The same bridges by comparing every pad with every other pad.
    fn mask_bridges_all(&self, bridge: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let b2 = 2 * i128::from(bridge.get());
        for (i, a) in all_pads.iter().enumerate() {
            for b in &all_pads[i + 1..] {
                let verdict = gap_points(
                    centre_distance2x4(a.at, b.at),
                    b2 + a.ins2 + b.ins2,
                    b2 + a.circ2 + b.circ2,
                );
                if verdict == Verdict::Fine
                    || !share_layer(&a.layers, &b.layers, |l| is_mask(self, l))
                {
                    continue;
                }
                push(
                    report,
                    verdict,
                    (MASK_BRIDGE_THIN, format!("{} and {}", a.label, b.label)),
                    format!("mask between {} and {}", a.label, b.label),
                );
            }
        }
    }

    fn zone_rules(&self, min_clearance: Nm, report: &mut RuleReport) {
        for (zid, z) in &self.zones {
            if z.clearance < min_clearance {
                report.confirmed.push(make(
                    ZONE_CLEARANCE_SMALL,
                    Severity::Error,
                    format!(
                        "zone {zid}: {} nm, minimum {} nm",
                        z.clearance.get(),
                        min_clearance.get()
                    ),
                ));
            }
        }
        let zones: Vec<_> = self.zones.iter().collect();
        for (i, (ida, a)) in zones.iter().enumerate() {
            for (idb, b) in &zones[i + 1..] {
                if a.net == b.net
                    || a.layer != b.layer
                    || a.outline.len() < 3
                    || b.outline.len() < 3
                {
                    continue;
                }
                if polygons_overlap(&a.outline, &b.outline) {
                    report.confirmed.push(make(
                        ZONES_OVERLAP,
                        Severity::Error,
                        format!("zones {ida} and {idb}"),
                    ));
                }
            }
        }
    }
}

fn polygons_overlap(a: &[Point], b: &[Point]) -> bool {
    let edges = |p: &[Point]| -> Vec<(Point, Point)> {
        (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect()
    };
    let (ea, eb) = (edges(a), edges(b));
    ea.iter()
        .any(|x| eb.iter().any(|y| segments_intersect(x.0, x.1, y.0, y.1)))
        || a.iter().any(|p| point_in_polygon(*p, b))
        || b.iter().any(|p| point_in_polygon(*p, a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Footprint, Layer, Net, Pad, PadNet, Side, Track, Zone};
    use eh3d_phoenix_model::{ObjectId, Udeg};
    use std::collections::BTreeMap;

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }
    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }
    fn rules(text: &str) -> RuleSet {
        let (r, d) = RuleSet::parse(text);
        assert!(d.is_empty(), "{d:?}");
        r
    }

    /// A board with a copper layer and a mask layer and one component whose footprint has the
    /// given pads (position and shape); every pad has its own net. Returns the board and the
    /// ids of the layers.
    fn board_with_pads(specs: &[(i64, i64, PadShape)]) -> (Board, Id, Id) {
        let (top, mask, fp, comp) = (id(1), id(2), id(3), id(4));
        let mut b = Board::default();
        for (l, name, kind) in [
            (top, "Top", LayerKind::Copper),
            (mask, "Mask", LayerKind::SolderMask),
        ] {
            b.layers.insert(
                l,
                Layer {
                    name: name.into(),
                    kind,
                    side: None,
                },
            );
        }
        let mut pads = BTreeMap::new();
        let mut pad_nets = Vec::new();
        for (i, (x, y, shape)) in specs.iter().enumerate() {
            let pid = id(100 + i as u16);
            let net = id(200 + i as u16);
            b.nets.insert(
                net,
                Net {
                    name: format!("N{i}"),
                },
            );
            pads.insert(
                pid,
                Pad {
                    number: (i + 1).to_string(),
                    shape: *shape,
                    at: pt(*x, *y),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![top, mask],
                    roundness: 0,
                    plated: true,
                },
            );
            pad_nets.push(PadNet {
                component: comp,
                pad: pid,
                net,
            });
        }
        b.footprints.insert(
            fp,
            Footprint {
                name: "F".into(),
                pads,
            },
        );
        b.components.insert(
            comp,
            Component {
                refdes: "U1".into(),
                value: "x".into(),
                symbol: None,
                footprint: fp,
                at: pt(0, 0),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        pad_nets.sort_by_key(|p| p.pad_ref());
        b.pad_nets = pad_nets;
        (b, top, mask)
    }

    fn round(d: i64) -> PadShape {
        PadShape::Circle { diameter: nm(d) }
    }

    fn counts(r: &RuleReport) -> (usize, usize) {
        (r.confirmed.len(), r.inconclusive.len())
    }

    #[test]
    fn round_pads_of_different_nets_keep_their_clearance_exactly() {
        // two 1 mm pads, centres 1.2 mm apart: 0.2 mm between the edges
        let (b, ..) =
            board_with_pads(&[(0, 0, round(1_000_000)), (1_200_000, 0, round(1_000_000))]);
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.2\n"))),
            (0, 0)
        );
        let r = b.check_rules(&rules("min_clearance = 0.21\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(CLEARANCE_VIOLATED));
    }

    /// The next number of a generator that is always the same, from 0 to 999999.
    fn next(seed: &mut u64) -> i64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((*seed >> 33) % 1_000_000) as i64
    }

    /// A board of 40 by 25 mm with `count` pads of three shapes at places that the generator
    /// chooses, and tracks (some of them long and slanted), and vias among them.
    fn crowded_board(count: usize, seed: &mut u64) -> Board {
        use crate::model::Via;
        let mut specs = Vec::new();
        for i in 0..count {
            let (x, y) = (next(seed) * 40, next(seed) * 25);
            let shape = match i % 3 {
                0 => round(800_000),
                1 => PadShape::Rect {
                    width: nm(1_200_000),
                    height: nm(700_000),
                },
                _ => PadShape::Oblong {
                    width: nm(1_500_000),
                    height: nm(600_000),
                },
            };
            specs.push((x, y, shape));
        }
        let (mut b, top, mask) = board_with_pads(&specs);
        for k in 0..(count / 2) {
            let (x, y) = (next(seed) * 40, next(seed) * 25);
            let (dx, dy) = if k % 17 == 0 {
                // a long slanted piece, which does not fit the cells of the grid
                (35_000_000 - x / 2, 20_000_000 - y / 2)
            } else {
                (
                    next(seed) % 6_000_000 - 3_000_000,
                    next(seed) % 6_000_000 - 3_000_000,
                )
            };
            let end = ((x + dx).clamp(0, 40_000_000), (y + dy).clamp(0, 25_000_000));
            b.tracks.insert(
                id(1000 + k as u16),
                Track {
                    net: id(200 + (k % 7) as u16),
                    layer: top,
                    width: nm(150_000 + 50_000 * (k % 3) as i64),
                    path: vec![Segment::Line {
                        start: pt(x, y),
                        end: pt(end.0, end.1),
                    }],
                },
            );
        }
        for k in 0..(count / 5) {
            b.vias.insert(
                id(3000 + k as u16),
                Via {
                    net: id(200 + (k % 5) as u16),
                    at: pt(next(seed) * 40, next(seed) * 25),
                    drill: nm(300_000),
                    diameter: nm(600_000),
                    from_layer: top,
                    to_layer: mask,
                },
            );
        }
        b
    }

    #[test]
    fn the_grid_gives_the_findings_of_the_pass_over_every_pair() {
        let mut seed = 11;
        let b = crowded_board(300, &mut seed);
        let all_pads = pads(&b);
        for text in [
            "min_clearance = 0.1\nmin_mask_bridge = 0.1\n",
            "min_clearance = 0.35\nmin_mask_bridge = 0.3\n",
            "min_clearance = 1.5\nmin_mask_bridge = 1\n",
        ] {
            let r = rules(text);
            let mut indexed = RuleReport::default();
            b.pad_clearances(r.min_clearance, &all_pads, &mut indexed);
            b.mask_bridges(r.min_mask_bridge, &all_pads, &mut indexed);
            let mut every = RuleReport::default();
            b.pad_clearances_all(r.min_clearance, &all_pads, &mut every);
            b.mask_bridges_all(r.min_mask_bridge, &all_pads, &mut every);
            // the same findings in the same order, and enough of them to mean something
            assert_eq!(indexed, every, "{text}");
            assert!(
                indexed.confirmed.len() > 20,
                "{text}: {}",
                indexed.confirmed.len()
            );
        }
        // the inconclusive ones (the pads that are not round) are in the comparison too
        let r = rules("min_clearance = 0.35\n");
        let mut indexed = RuleReport::default();
        b.pad_clearances(r.min_clearance, &all_pads, &mut indexed);
        assert!(!indexed.inconclusive.is_empty());
    }

    #[test]
    fn a_reach_too_large_for_the_grid_falls_back_to_the_pass_over_every_pair() {
        assert!(Grid::new(0).is_none());
        assert!(Grid::new(GRID_LIMIT_NM + 1).is_none());
        assert!(Grid::new(GRID_LIMIT_NM).is_some());
        let mut seed = 3;
        let b = crowded_board(40, &mut seed);
        let r = RuleSet {
            min_clearance: nm(1 << 29),
            min_mask_bridge: nm(1 << 29),
            ..RuleSet::default()
        };
        let all_pads = pads(&b);
        let mut grid_way = RuleReport::default();
        b.pad_clearances(r.min_clearance, &all_pads, &mut grid_way);
        b.mask_bridges(r.min_mask_bridge, &all_pads, &mut grid_way);
        let mut every = RuleReport::default();
        b.pad_clearances_all(r.min_clearance, &all_pads, &mut every);
        b.mask_bridges_all(r.min_mask_bridge, &all_pads, &mut every);
        assert_eq!(grid_way, every);
        assert!(!every.confirmed.is_empty());
    }

    #[test]
    fn the_cells_of_the_grid_find_the_neighbours_across_zero_and_leave_the_far_ones() {
        let mut grid = Grid::new(10).unwrap();
        // 10 nm cells: -1 is in the cell -1, 9 in 0, 19 in 1, 35 in 3
        for (index, x) in [-1, 9, 19, 35].into_iter().enumerate() {
            grid.add_point(pt(x, 0), index);
        }
        assert_eq!(grid.around(pt(0, 0)), vec![0, 1, 2]);
        assert_eq!(grid.around(pt(20, 5)), vec![2, 3]);
        assert_eq!(grid.around(pt(30, 5)), vec![3]);
        assert!(grid.around(pt(0, 100)).is_empty());
        // a piece is in every cell its box grown by the reach covers
        let mut pieces = Grid::new(10).unwrap();
        assert!(pieces.add_piece(pt(0, 0), pt(25, 0), 10, 7));
        assert_eq!(pieces.at(pt(-9, 5)), vec![7]);
        assert_eq!(pieces.at(pt(34, -9)), vec![7]);
        assert!(pieces.at(pt(45, 0)).is_empty());
        // a long slanted one is refused and left to the caller
        assert!(!pieces.add_piece(pt(0, 0), pt(1000, 1000), 10, 8));
        assert!(pieces.at(pt(500, 500)).is_empty());
    }

    #[test]
    fn a_row_of_pads_gives_one_finding_for_each_close_neighbour_and_none_for_the_far_ones() {
        // 40 pads of 1 mm in a row, centres 1.2 mm apart: 0.2 mm between the edges of
        // neighbours (39 pairs) and 1.4 mm between the next ones; one more pad far away
        let mut specs: Vec<(i64, i64, PadShape)> = (0..40)
            .map(|k| (k * 1_200_000, 0, round(1_000_000)))
            .collect();
        specs.push((500_000_000, 500_000_000, round(1_000_000)));
        let (b, ..) = board_with_pads(&specs);
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.2\n"))),
            (0, 0)
        );
        let clearance = b.check_rules(&rules("min_clearance = 0.25\n"));
        assert_eq!(counts(&clearance), (39, 0));
        assert!(clearance
            .confirmed
            .iter()
            .all(|d| d.code.as_deref() == Some(CLEARANCE_VIOLATED)));
        // the mask bridge is the same distance test on the mask layer
        let both = b.check_rules(&rules("min_clearance = 0.25\nmin_mask_bridge = 0.25\n"));
        assert_eq!(counts(&both), (78, 0));
        let bridges = both
            .confirmed
            .iter()
            .filter(|d| d.code.as_deref() == Some(MASK_BRIDGE_THIN))
            .count();
        assert_eq!(bridges, 39);
        // 1.4 mm between the edges of the pads that are two places apart
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 1.41\n"))),
            (39 + 38, 0)
        );
    }

    #[test]
    fn a_rectangular_pad_in_the_grey_zone_is_inconclusive_not_a_failure() {
        // a 1 x 3 mm pad: its inscribed circle is 1 mm, the circle around it about 3.16 mm
        let (b, ..) = board_with_pads(&[
            (
                0,
                0,
                PadShape::Rect {
                    width: nm(1_000_000),
                    height: nm(3_000_000),
                },
            ),
            (2_000_000, 0, round(1_000_000)),
        ]);
        // centres 2 mm apart: certainly too close only if the inscribed circles are
        let r = b.check_rules(&rules("min_clearance = 0.4\n"));
        assert_eq!(counts(&r), (0, 1));
        assert_eq!(r.inconclusive[0].code.as_deref(), Some(PAD_SHAPE_UNSURE));
        // very far apart is fine; touching inscribed circles are certain
        let (b2, ..) = board_with_pads(&[
            (
                0,
                0,
                PadShape::Rect {
                    width: nm(1_000_000),
                    height: nm(3_000_000),
                },
            ),
            (10_000_000, 0, round(1_000_000)),
        ]);
        assert_eq!(
            counts(&b2.check_rules(&rules("min_clearance = 0.4\n"))),
            (0, 0)
        );
        let (b3, ..) = board_with_pads(&[
            (
                0,
                0,
                PadShape::Rect {
                    width: nm(1_000_000),
                    height: nm(3_000_000),
                },
            ),
            (1_000_000, 0, round(1_000_000)),
        ]);
        assert_eq!(
            counts(&b3.check_rules(&rules("min_clearance = 0.4\n"))).0,
            1
        );
    }

    #[test]
    fn a_pad_and_a_track_of_another_net_keep_their_clearance() {
        let (mut b, top, _) = board_with_pads(&[(0, 0, round(1_000_000))]);
        let other = id(300);
        b.nets.insert(other, Net { name: "T".into() });
        // a track 0.2 mm wide running at y = 0.7 mm: edge of the pad at 0.5, edge of the track at 0.6
        b.tracks.insert(
            id(301),
            Track {
                net: other,
                layer: top,
                width: nm(200_000),
                path: vec![Segment::Line {
                    start: pt(-5_000_000, 700_000),
                    end: pt(5_000_000, 700_000),
                }],
            },
        );
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.1\n"))),
            (0, 0)
        );
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.11\n"))),
            (1, 0)
        );
    }

    #[test]
    fn a_pad_and_an_arc_of_another_net_keep_their_clearance() {
        let (mut b, top, _) = board_with_pads(&[(0, 0, round(1_000_000))]);
        let other = id(300);
        b.nets.insert(other, Net { name: "T".into() });
        // a quarter turn of radius 1.414 mm that goes down to y = 0.7 mm above the pad: the edge
        // of the pad is at 0.5 mm, the edge of the track at 0.6 mm; both ends are at y = 1.114 mm
        b.tracks.insert(
            id(301),
            Track {
                net: other,
                layer: top,
                width: nm(200_000),
                path: vec![Segment::Arc {
                    start: pt(-1_000_000, 1_114_214),
                    end: pt(1_000_000, 1_114_214),
                    sweep: Udeg::new(90_000_000).unwrap(),
                }],
            },
        );
        // (the pass over every pair of tracks still says that it cannot judge the arc)
        let r = b.check_rules(&rules("min_clearance = 0.09\n"));
        assert!(r.confirmed.is_empty(), "{:?}", r.confirmed);
        assert!(r
            .inconclusive
            .iter()
            .all(|d| d.code.as_deref() == Some(CLEARANCE_INCONCLUSIVE)));
        let r = b.check_rules(&rules("min_clearance = 0.12\n"));
        assert_eq!(r.confirmed.len(), 1, "{:?}", r.confirmed);
        assert_eq!(r.confirmed[0].code.as_deref(), Some(CLEARANCE_VIOLATED));
    }

    #[test]
    fn an_arc_in_the_outline_is_the_edge_that_the_copper_keeps_away_from() {
        // a square of 10 mm whose bottom edge is a half circle of radius 5 mm about (5, 0) that
        // bulges into the board, up to (5, 5) mm; a pad of 1 mm at (5, 5.9) mm is 0.4 mm above
        // the arc (0.401 mm from the chords the arc is cut into) and 3.6 mm or more from the
        // other edges
        let (mut b, ..) = board_with_pads(&[(5_000_000, 5_900_000, round(1_000_000))]);
        b.outline = vec![
            pt(0, 0),
            pt(10_000_000, 0),
            pt(10_000_000, 10_000_000),
            pt(0, 10_000_000),
        ];
        // without the arc the nearest edge is the left or the right one, 4.5 mm away
        assert_eq!(
            counts(&b.check_rules(&rules("min_edge_clearance = 0.5\n"))),
            (0, 0)
        );
        b.outline_arcs = vec![crate::outline_arc::OutlineArc {
            edge: 0,
            sweep: eh3d_phoenix_model::Udeg::new(-180_000_000).unwrap(),
        }];
        assert_eq!(
            counts(&b.check_rules(&rules("min_edge_clearance = 0.4\n"))),
            (0, 0)
        );
        let r = b.check_rules(&rules("min_edge_clearance = 0.5\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(EDGE_TOO_CLOSE));
    }

    #[test]
    fn copper_too_near_the_edge_of_the_board_is_found() {
        let (mut b, top, _) = board_with_pads(&[(5_000_000, 5_000_000, round(1_000_000))]);
        b.outline = vec![
            pt(0, 0),
            pt(10_000_000, 0),
            pt(10_000_000, 10_000_000),
            pt(0, 10_000_000),
        ];
        let net = id(300);
        b.nets.insert(net, Net { name: "T".into() });
        b.tracks.insert(
            id(301),
            Track {
                net,
                layer: top,
                width: nm(200_000),
                path: vec![Segment::Line {
                    start: pt(1_000_000, 300_000),
                    end: pt(9_000_000, 300_000),
                }],
            },
        );
        // the track edge is at 0.2 mm from the board edge
        assert_eq!(
            counts(&b.check_rules(&rules("min_edge_clearance = 0.2\n"))),
            (0, 0)
        );
        let r = b.check_rules(&rules("min_edge_clearance = 0.25\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(EDGE_TOO_CLOSE));
        // a pad 0.4 mm from the edge (centre 0.9 mm, radius 0.5 mm)
        let (mut c, ..) = board_with_pads(&[(900_000, 5_000_000, round(1_000_000))]);
        c.outline = b.outline.clone();
        assert_eq!(
            counts(&c.check_rules(&rules("min_edge_clearance = 0.4\n"))),
            (0, 0)
        );
        assert_eq!(
            counts(&c.check_rules(&rules("min_edge_clearance = 0.5\n"))),
            (1, 0)
        );
    }

    #[test]
    fn the_solder_mask_keeps_a_bridge_between_pads() {
        // two 1 mm pads, centres 1.15 mm apart: 0.15 mm of mask between the openings
        let (b, ..) =
            board_with_pads(&[(0, 0, round(1_000_000)), (1_150_000, 0, round(1_000_000))]);
        assert_eq!(
            counts(&b.check_rules(&rules("min_mask_bridge = 0.1\n"))),
            (0, 0)
        );
        let r = b.check_rules(&rules("min_mask_bridge = 0.2\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(MASK_BRIDGE_THIN));
    }

    #[test]
    fn zones_keep_the_clearance_and_do_not_overlap_other_nets() {
        let (mut b, top, _) = board_with_pads(&[]);
        let (n1, n2) = (id(300), id(301));
        b.nets.insert(n1, Net { name: "A".into() });
        b.nets.insert(n2, Net { name: "B".into() });
        let square =
            |x: i64, y: i64, s: i64| vec![pt(x, y), pt(x + s, y), pt(x + s, y + s), pt(x, y + s)];
        b.zones.insert(
            id(310),
            Zone {
                net: n1,
                layer: top,
                outline: square(0, 0, 10_000_000),
                clearance: nm(150_000),
                cutouts: Vec::new(),
            },
        );
        b.zones.insert(
            id(311),
            Zone {
                net: n2,
                layer: top,
                outline: square(20_000_000, 0, 5_000_000),
                clearance: nm(300_000),
                cutouts: Vec::new(),
            },
        );
        let r = b.check_rules(&rules("min_clearance = 0.2\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(ZONE_CLEARANCE_SMALL));
        // move the second zone over the first
        b.zones.get_mut(&id(311)).unwrap().outline = square(5_000_000, 5_000_000, 10_000_000);
        let r = b.check_rules(&rules("min_clearance = 0.1\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(ZONES_OVERLAP));
        // one zone entirely inside the other counts too, and the same net may overlap itself
        b.zones.get_mut(&id(311)).unwrap().outline = square(2_000_000, 2_000_000, 2_000_000);
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.1\n"))),
            (1, 0)
        );
        b.zones.get_mut(&id(311)).unwrap().net = n1;
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.1\n"))),
            (0, 0)
        );
    }

    /// A board with one round pad (1 mm, through hole when `drill`) at the origin of the top
    /// layer and a top silkscreen layer, with the copper stack given when `stack`.
    fn silk_board(drill: bool, stack: bool) -> (Board, Id) {
        let (mut b, top, _mask) = board_with_pads(&[(0, 0, round(1_000_000))]);
        let silk = id(30);
        b.layers.insert(
            silk,
            Layer {
                name: "Top silk".into(),
                kind: LayerKind::Silkscreen,
                side: Some(Side::Top),
            },
        );
        if drill {
            for f in b.footprints.values_mut() {
                for p in f.pads.values_mut() {
                    p.drill = Some(nm(400_000));
                }
            }
        }
        if stack {
            b.copper_stack = vec![top];
        }
        (b, silk)
    }

    fn silk_line(layer: Id, y: i64) -> crate::silk::SilkItem {
        crate::silk::SilkItem::Stroke {
            layer,
            width: nm(150_000),
            path: vec![Segment::Line {
                start: pt(-2_000_000, y),
                end: pt(2_000_000, y),
            }],
        }
    }

    #[test]
    fn silkscreen_over_a_pad_is_a_confirmed_finding() {
        let (mut b, silk) = silk_board(false, true);
        b.silkscreen.insert(id(40), silk_line(silk, 0));
        let on = rules("silkscreen_over_pads = on\n");
        let r = b.check_rules(&on);
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(SILK_OVER_PAD));
        // the rule off, or the stroke clear of the pad (pad radius 0.5 mm + half the line)
        assert_eq!(
            counts(&b.check_rules(&rules("silkscreen_over_pads = off\n"))),
            (0, 0)
        );
        b.silkscreen.insert(id(40), silk_line(silk, 700_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
        // closer than the copper edge plus half the line is a finding, at it is not
        b.silkscreen.insert(id(40), silk_line(silk, 574_999));
        assert_eq!(counts(&b.check_rules(&on)), (1, 0));
        b.silkscreen.insert(id(40), silk_line(silk, 575_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
    }

    #[test]
    fn a_text_over_a_pad_is_found_through_its_characters() {
        let (mut b, silk) = silk_board(false, true);
        let text = |x: i64| crate::silk::SilkItem::Text {
            layer: silk,
            at: pt(x, -600_000),
            text: "I".into(),
            height: nm(1_200_000),
            stroke: nm(100_000),
            rotation: Udeg::ZERO,
            mirrored: false,
        };
        let on = rules("silkscreen_over_pads = on\n");
        // the stem of the I passes through the pad
        b.silkscreen.insert(id(41), text(-400_000));
        assert_eq!(counts(&b.check_rules(&on)), (1, 0));
        // far to the side
        b.silkscreen.insert(id(41), text(5_000_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
    }

    #[test]
    fn what_cannot_be_judged_is_said_and_never_a_failure() {
        let on = rules("silkscreen_over_pads = on\n");
        // no copper stack and a pad without a drill: its side is not known
        let (mut b, silk) = silk_board(false, false);
        b.silkscreen.insert(id(40), silk_line(silk, 0));
        let r = b.check_rules(&on);
        assert_eq!(counts(&r), (0, 1));
        assert_eq!(r.inconclusive[0].code.as_deref(), Some(NOT_CHECKABLE));
        // a through-hole pad shows on both sides, so no stack is needed
        let (mut b, silk) = silk_board(true, false);
        b.silkscreen.insert(id(40), silk_line(silk, 0));
        assert_eq!(counts(&b.check_rules(&on)), (1, 0));
        // a silkscreen layer that does not say its side
        b.layers.get_mut(&silk).unwrap().side = None;
        let r = b.check_rules(&on);
        assert_eq!(counts(&r), (0, 1));
        assert_eq!(r.inconclusive[0].code.as_deref(), Some(NOT_CHECKABLE));
    }

    #[test]
    fn the_other_side_of_the_board_is_not_checked() {
        let (mut b, silk) = silk_board(false, true);
        b.layers.get_mut(&silk).unwrap().side = Some(Side::Bottom);
        b.layers.insert(
            id(31),
            Layer {
                name: "Bottom".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack = vec![id(1), id(31)];
        b.silkscreen.insert(id(40), silk_line(silk, 0));
        // the pad is on the top layer only; the silkscreen is for the bottom
        assert_eq!(
            counts(&b.check_rules(&rules("silkscreen_over_pads = on\n"))),
            (0, 0)
        );
        let (_, d) = RuleSet::parse("silkscreen_over_pads = maybe\n");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn silkscreen_over_a_mask_opening_is_its_own_rule() {
        let on = rules("silkscreen_over_mask_openings = on\n");
        let (mut b, silk) = silk_board(false, true);
        b.silkscreen.insert(id(40), silk_line(silk, 0));
        // the pad has no mask layer of the side of the silkscreen yet: nothing is open
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
        // the mask layer of the top side, which the pad lists, makes the opening
        b.layers.get_mut(&id(2)).unwrap().side = Some(Side::Top);
        let r = b.check_rules(&on);
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(SILK_OVER_MASK));
        // the other rule stays off, and a stroke clear of the pad is silent
        assert_eq!(
            counts(&b.check_rules(&rules("silkscreen_over_pads = off\n"))),
            (0, 0)
        );
        b.silkscreen.insert(id(40), silk_line(silk, 700_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
        let (_, d) = RuleSet::parse("silkscreen_over_mask_openings = maybe\n");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn an_arc_is_clear_when_provably_so_and_inconclusive_when_near() {
        let on = rules("silkscreen_over_pads = on\n");
        let arc = |x: i64, y: i64, sweep: i64| crate::silk::SilkItem::Stroke {
            layer: id(30),
            width: nm(150_000),
            path: vec![Segment::Arc {
                start: pt(x, y),
                end: pt(x + 1_000_000, y),
                sweep: Udeg::new(sweep).unwrap(),
            }],
        };
        let (mut b, _) = silk_board(false, true);
        // far from the pad: provably clear
        b.silkscreen
            .insert(id(40), arc(10_000_000, 10_000_000, 90_000_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 0));
        // near the pad: not provably clear, so inconclusive and never a failure
        b.silkscreen
            .insert(id(40), arc(-500_000, 600_000, 90_000_000));
        let r = b.check_rules(&on);
        assert_eq!(counts(&r), (0, 1));
        assert_eq!(
            r.inconclusive[0].code.as_deref(),
            Some(CLEARANCE_INCONCLUSIVE)
        );
        // more than half a turn is never judged clear
        b.silkscreen
            .insert(id(40), arc(10_000_000, 10_000_000, 270_000_000));
        assert_eq!(counts(&b.check_rules(&on)), (0, 1));
    }

    #[test]
    fn the_rule_with_nothing_printed_has_nothing_to_report() {
        let (b, ..) = board_with_pads(&[(0, 0, round(1_000_000))]);
        assert_eq!(
            counts(&b.check_rules(&rules("silkscreen_over_pads = on\n"))),
            (0, 0)
        );
    }

    #[test]
    fn no_rule_means_nothing_is_checked() {
        let (b, ..) = board_with_pads(&[(0, 0, round(1_000_000)), (100, 0, round(1_000_000))]);
        assert_eq!(counts(&b.check_rules(&RuleSet::default())), (0, 0));
    }

    #[test]
    fn the_integer_square_root_rounds_up() {
        assert_eq!(isqrt_ceil(0), 0);
        assert_eq!(isqrt_ceil(1), 1);
        assert_eq!(isqrt_ceil(2), 2);
        assert_eq!(isqrt_ceil(16), 4);
        assert_eq!(isqrt_ceil(17), 5);
        let n: i128 = 10_000_000_000_000_000_000;
        let r = isqrt_ceil(n);
        assert!(r * r >= n && (r - 1) * (r - 1) < n);
    }

    #[test]
    fn copper_too_near_a_cutout_of_the_outline_is_found() {
        // a pad 1 mm wide centred at (5, 5) mm in a board of 10 by 10 mm
        let (mut b, ..) = board_with_pads(&[(5_000_000, 5_000_000, round(1_000_000))]);
        b.outline = vec![
            pt(0, 0),
            pt(10_000_000, 0),
            pt(10_000_000, 10_000_000),
            pt(0, 10_000_000),
        ];
        assert_eq!(
            counts(&b.check_rules(&rules("min_edge_clearance = 0.51\n"))),
            (0, 0)
        );
        // a cutout from x = 6 mm to x = 8 mm: its edge is 0.5 mm from the edge of the pad
        b.outline_cutouts = vec![vec![
            pt(6_000_000, 4_000_000),
            pt(8_000_000, 4_000_000),
            pt(8_000_000, 6_000_000),
            pt(6_000_000, 6_000_000),
        ]];
        assert_eq!(
            counts(&b.check_rules(&rules("min_edge_clearance = 0.5\n"))),
            (0, 0)
        );
        let r = b.check_rules(&rules("min_edge_clearance = 0.51\n"));
        assert_eq!(counts(&r), (1, 0));
        assert_eq!(r.confirmed[0].code.as_deref(), Some(EDGE_TOO_CLOSE));
    }

    #[test]
    fn a_hole_that_is_not_plated_has_no_copper_and_no_ring() {
        // two 1 mm pads 1.2 mm apart break a clearance of 0.21 mm
        let (mut b, ..) =
            board_with_pads(&[(0, 0, round(1_000_000)), (1_200_000, 0, round(1_000_000))]);
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.21\n"))),
            (1, 0)
        );
        // the second one is a mounting hole of 0.8 mm: no copper, so nothing to keep apart
        for pad in b.footprints.values_mut().flat_map(|f| f.pads.values_mut()) {
            if pad.number == "2" {
                pad.drill = Some(nm(800_000));
                pad.plated = false;
                assert!(pad.is_npth());
            }
        }
        assert_eq!(
            counts(&b.check_rules(&rules("min_clearance = 0.21\n"))),
            (0, 0)
        );
        // a ring of 0.1 mm around a 0.8 mm drill in a 1 mm pad is too thin for 0.15 mm when the
        // hole is plated and is not looked at when it is not
        for pad in b.footprints.values_mut().flat_map(|f| f.pads.values_mut()) {
            if pad.number == "1" {
                pad.drill = Some(nm(800_000));
            }
        }
        let ring = rules("min_annular_ring = 0.15\n");
        assert_eq!(counts(&b.check_rules(&ring)), (1, 0));
        for pad in b.footprints.values_mut().flat_map(|f| f.pads.values_mut()) {
            pad.plated = false;
        }
        assert_eq!(counts(&b.check_rules(&ring)), (0, 0));
    }
}
