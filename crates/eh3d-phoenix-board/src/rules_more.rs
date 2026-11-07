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

use eh3d_diagnostic::Severity;
use eh3d_phoenix_model::{segments_intersect, Nm, Point};

use crate::connectivity::{near_segment_exact, pad_position, point_in_polygon};
use crate::model::{Board, Id, LayerKind, PadRef, PadShape, Segment, Side};
use crate::outline_arc::ArcSettings;
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

    fn pad_clearances(&self, clearance: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
        let c2 = 2 * i128::from(clearance.get());
        // the straight pieces of every track, made once and not again for every pad
        let track_lines: Vec<_> = self
            .tracks
            .iter()
            .map(|(tid, t)| (tid, t, lines(&t.path)))
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
                for (s, e) in pieces {
                    verdict = worst(
                        verdict,
                        gap_segment(a.at, *s, *e, c2 + w + a.ins2, c2 + w + a.circ2),
                    );
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
                    || !(a.layers.contains(&v.from_layer) || a.layers.contains(&v.to_layer))
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
        let outline = self.outline_polygon(&ArcSettings::default());
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

    fn mask_bridges(&self, bridge: Nm, all_pads: &[PadInfo], report: &mut RuleReport) {
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
