// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper near a track as simple items, to plan the push of neighbour tracks and to check the
//! arcs of a track against the clearance of the rules.
//!
//! The numbers here are floating point and are only used to *plan*: every piece that is kept is
//! rounded to whole nanometres and the design rules of `eh3d-phoenix-rules` judge the result
//! exactly afterwards. The planning leans to the safe side (a pad is taken as a disc of its larger
//! half side, an arc as short chords widened by their sagitta).

use std::f64::consts::PI;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment, Side};
use eh3d_phoenix_model::Point;

/// A point in nanometres, as floating point.
pub type V = (f64, f64);

/// How close to a distance limit a result may be and still count as reaching it, in nanometres.
pub const EPS: f64 = 0.5;

/// A point as floating point.
pub fn vp(p: Point) -> V {
    (p.x_nm.get() as f64, p.y_nm.get() as f64)
}

/// The nearest point of the segment `ab` to `p` and its distance.
pub fn point_segment(p: V, a: V, b: V) -> (f64, V) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    let q = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - q.0).hypot(p.1 - q.1), q)
}

fn cross(o: V, a: V, b: V) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// True when the segments `ab` and `cd` cross each other properly (a touch is not a crossing).
pub fn segments_cross(a: V, b: V, c: V, d: V) -> bool {
    let (d1, d2) = (cross(a, b, c), cross(a, b, d));
    let (d3, d4) = (cross(c, d, a), cross(c, d, b));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The distance between two segments, with the nearest point of each (the first one twice when
/// they cross, at distance zero).
pub fn segment_segment(a: V, b: V, c: V, d: V) -> (f64, V, V) {
    if segments_cross(a, b, c, d) {
        return (0.0, a, a);
    }
    let mut best = (f64::INFINITY, a, c);
    for (p, s0, s1, swap) in [
        (a, c, d, false),
        (b, c, d, false),
        (c, a, b, true),
        (d, a, b, true),
    ] {
        let (dist, q) = point_segment(p, s0, s1);
        if dist < best.0 {
            best = if swap { (dist, q, p) } else { (dist, p, q) };
        }
    }
    best
}

/// The points of the arc from `start` to `end` with a signed sweep in micro-degrees (positive is
/// counter-clockwise), cut into `steps_per_quarter` chords for every quarter turn, with the
/// sagitta of a chord. `None` when the arc has no shape.
pub fn arc_points(
    start: Point,
    end: Point,
    sweep_udeg: i64,
    steps_per_quarter: usize,
) -> Option<(Vec<V>, f64)> {
    let (a, b) = (vp(start), vp(end));
    let theta = (sweep_udeg.unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
    let chord = (b.0 - a.0).hypot(b.1 - a.1);
    let sin_half = (theta / 2.0).sin();
    if theta <= 0.0 || sin_half <= 1e-9 || chord <= 0.0 {
        return None;
    }
    let sign = if sweep_udeg < 0 { -1.0 } else { 1.0 };
    let radius = chord / (2.0 * sin_half);
    let (ux, uy) = ((b.0 - a.0) / chord, (b.1 - a.1) / chord);
    // the centre is on the left of the chord for a counter-clockwise sweep below a half turn
    let offset = sign * radius * (theta / 2.0).cos();
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let centre = (mid.0 - uy * offset, mid.1 + ux * offset);
    let first = (a.1 - centre.1).atan2(a.0 - centre.0);
    let pieces = ((theta / (PI / 2.0) * steps_per_quarter as f64 - 1e-9).ceil() as usize).max(1);
    let step = theta / pieces as f64;
    let mut points = Vec::with_capacity(pieces + 1);
    points.push(a);
    for k in 1..pieces {
        let phi = first + sign * step * k as f64;
        points.push((centre.0 + radius * phi.cos(), centre.1 + radius * phi.sin()));
    }
    points.push(b);
    let sagitta = radius * (1.0 - (step / 2.0).cos());
    Some((points, sagitta))
}

/// A piece of copper (a segment widened by `half` on each side) that other copper keeps away from.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    /// One end.
    pub a: V,
    /// The other end (the same as `a` for a pad or a via).
    pub b: V,
    /// Half the width.
    pub half: f64,
    /// The net, when it has one.
    pub net: Option<Id>,
    /// True for the edge of the board, which keeps its own clearance.
    pub edge: bool,
}

/// The pads, vias and edges of the board that a track on `layer` has to keep away from. They are
/// never moved.
pub fn fixed_items(board: &Board, layer: Id) -> Vec<Item> {
    let mut out = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let on_layer = pad.drill.is_some()
                || component.side == Side::Bottom
                || pad.layers.contains(&layer);
            if !on_layer {
                continue;
            }
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let half = match pad.shape {
                PadShape::Circle { diameter } => diameter.get() as f64 / 2.0,
                PadShape::Oblong { width, height } | PadShape::Rect { width, height } => {
                    width.get().max(height.get()) as f64 / 2.0
                }
            };
            out.push(Item {
                a: vp(at),
                b: vp(at),
                half,
                net: board.net_of(pad_ref),
                edge: false,
            });
        }
    }
    for via in board.vias.values() {
        out.push(Item {
            a: vp(via.at),
            b: vp(via.at),
            half: via.diameter.get() as f64 / 2.0,
            net: Some(via.net),
            edge: false,
        });
    }
    if board.outline.len() >= 3 {
        for (a, b) in board
            .outline
            .iter()
            .zip(board.outline.iter().cycle().skip(1))
        {
            out.push(Item {
                a: vp(*a),
                b: vp(*b),
                half: 0.0,
                net: None,
                edge: true,
            });
        }
    }
    out
}

/// The pieces of one segment of a track as items: one for a line, several short ones for an arc.
pub fn segment_items(segment: &Segment, half: f64, net: Id, steps_per_quarter: usize) -> Vec<Item> {
    match *segment {
        Segment::Line { start, end } => vec![Item {
            a: vp(start),
            b: vp(end),
            half,
            net: Some(net),
            edge: false,
        }],
        Segment::Arc { start, end, sweep } => {
            match arc_points(start, end, sweep.get(), steps_per_quarter) {
                Some((points, sagitta)) => points
                    .windows(2)
                    .map(|w| Item {
                        a: w[0],
                        b: w[1],
                        half: half + sagitta + 1.0,
                        net: Some(net),
                        edge: false,
                    })
                    .collect(),
                None => {
                    let (a, b) = (vp(start), vp(end));
                    vec![Item {
                        a,
                        b,
                        half: half + (b.0 - a.0).hypot(b.1 - a.1),
                        net: Some(net),
                        edge: false,
                    }]
                }
            }
        }
    }
}

/// The tracks on `layer` as items.
pub fn track_items(board: &Board, layer: Id, steps_per_quarter: usize) -> Vec<Item> {
    let mut out = Vec::new();
    for track in board.tracks.values().filter(|t| t.layer == layer) {
        let half = track.width.get() as f64 / 2.0;
        for segment in &track.path {
            out.extend(segment_items(segment, half, track.net, steps_per_quarter));
        }
    }
    out
}

/// The gap that copper of `half` width on `net` must keep from an item, so that the distance
/// between their centre lines is at least `half + item.half + gap`: the clearance, or the clearance
/// to the edge of the board. `None` when the item is of the same net.
pub fn gap_to(item: &Item, net: Id, clearance: f64, edge_clearance: f64) -> Option<f64> {
    if item.net == Some(net) {
        return None;
    }
    Some(if item.edge { edge_clearance } else { clearance })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_distance_between_segments_is_that_of_the_nearest_points() {
        let (d, p, q) = segment_segment((0.0, 0.0), (10.0, 0.0), (3.0, 4.0), (8.0, 4.0));
        assert!((d - 4.0).abs() < 1e-9);
        assert!(p.1.abs() < 1e-9 && (q.1 - 4.0).abs() < 1e-9);
        // an end that is nearer than the sides
        let (d, _, _) = segment_segment((0.0, 0.0), (10.0, 0.0), (13.0, 4.0), (20.0, 4.0));
        assert!((d - 5.0).abs() < 1e-9);
    }

    #[test]
    fn crossing_segments_are_at_distance_zero_and_a_touch_is_not_a_crossing() {
        assert!(segments_cross(
            (0.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (10.0, 0.0)
        ));
        assert!(!segments_cross(
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0)
        ));
        let (d, _, _) = segment_segment((0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0));
        assert_eq!(d, 0.0);
        let (d, _, _) = segment_segment((0.0, 0.0), (10.0, 0.0), (10.0, 0.0), (10.0, 5.0));
        assert!(d < 1e-9);
    }

    #[test]
    fn a_quarter_arc_is_cut_into_the_chords_asked_for_and_stays_on_its_circle() {
        let start = Point::from_nm(10_000_000, 0).unwrap();
        let end = Point::from_nm(0, 10_000_000).unwrap();
        let (points, sagitta) = arc_points(start, end, 90_000_000, 8).unwrap();
        assert_eq!(points.len(), 9);
        for p in &points {
            assert!((p.0.hypot(p.1) - 10_000_000.0).abs() < 2.0, "{p:?}");
        }
        assert!(sagitta > 0.0 && sagitta < 20_000.0);
        // the other way round: clockwise from the top to the right
        let (cw, _) = arc_points(end, start, -90_000_000, 8).unwrap();
        for p in &cw {
            assert!((p.0.hypot(p.1) - 10_000_000.0).abs() < 2.0, "{p:?}");
        }
        assert!(arc_points(start, end, 0, 8).is_none());
    }
}
