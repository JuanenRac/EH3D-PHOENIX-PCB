// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper of a board that a zone has to keep away from, or to join: the pads, tracks and vias
//! on each copper layer, every one as a few points and segments with a half width.
//!
//! A pad that sits at a right angle is its true shape (a rectangle by its four sides, an oblong by
//! its axis); any other angle is the smallest circle around it. An arc is a short polyline whose
//! pieces are widened by their sagitta, so "clear" is always safe and "too close" may be cautious.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, PadShape, Segment};
use eh3d_phoenix_model::Point;

use crate::geom::{bbox, isqrt, Bx, P};

/// What a piece of copper is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A pad of a placed component.
    Pad,
    /// A via.
    Via,
    /// A piece of a track.
    Track,
}

/// A piece of copper on one layer.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub kind: Kind,
    pub net: Option<Id>,
    /// Names the item in a finding (`R1 pad 2`, `track <id>`, `via <id>`).
    pub label: String,
    /// The points whose circles of radius `half` make up the copper, for the hull.
    pub centres: Vec<P>,
    /// The segments the distance to the copper is measured from.
    pub segs: Vec<(P, P)>,
    /// The half width added around the segments.
    pub half: i64,
    /// True for a piece that stands for an arc.
    pub arc: bool,
    /// Where a zone of the same net joins it.
    pub centre: P,
    /// The radius of the smallest circle around it, for the relief.
    pub radius: i64,
    /// The box of the copper.
    pub bounds: Bx,
}

/// The items of each copper layer, in a fixed order: pads, then tracks, then vias.
pub(crate) type LayerItems = BTreeMap<Id, Vec<Item>>;

/// The geometry of a piece of copper before it is placed on a layer.
#[derive(Debug, Clone)]
struct Shape {
    centres: Vec<P>,
    segs: Vec<(P, P)>,
    half: i64,
    radius: i64,
}

fn pair(p: Point) -> P {
    (p.x_nm.get(), p.y_nm.get())
}

/// Half of a non-negative length, rounded up.
pub(crate) fn half_up(v: i64) -> i64 {
    v / 2 + v % 2
}

/// The radius of the smallest circle around a rectangle with these half sides, rounded up.
fn circum(hw: i64, hh: i64) -> i64 {
    let n = i128::from(hw) * i128::from(hw) + i128::from(hh) * i128::from(hh);
    let r = isqrt(n);
    let r = if r * r < n { r + 1 } else { r };
    i64::try_from(r).unwrap_or(i64::MAX)
}

fn make(kind: Kind, net: Option<Id>, label: String, shape: Shape, centre: P) -> Item {
    let Shape {
        centres,
        segs,
        half,
        radius,
    } = shape;
    let mut points: Vec<P> = Vec::new();
    for (a, b) in &segs {
        points.push(*a);
        points.push(*b);
    }
    let (x0, y0, x1, y1) = bbox(&points);
    Item {
        kind,
        net,
        label,
        centres,
        segs,
        half,
        arc: false,
        centre,
        radius,
        bounds: (
            x0.saturating_sub(half),
            y0.saturating_sub(half),
            x1.saturating_add(half),
            y1.saturating_add(half),
        ),
    }
}

/// The geometry of a pad centred at `c` and turned by `total` micro-degrees.
fn pad_geometry(shape: PadShape, c: P, total: i64) -> Shape {
    let right_angle = total % 90_000_000 == 0;
    let swapped = total == 90_000_000 || total == 270_000_000;
    let round = |r: i64| Shape {
        centres: vec![c],
        segs: vec![(c, c)],
        half: r,
        radius: r,
    };
    match shape {
        PadShape::Circle { diameter } => round(half_up(diameter.get())),
        PadShape::Rect { width, height } => {
            let (w, h) = if swapped {
                (height.get(), width.get())
            } else {
                (width.get(), height.get())
            };
            let (hw, hh) = (half_up(w), half_up(h));
            if right_angle {
                let corners = [
                    (c.0 - hw, c.1 - hh),
                    (c.0 + hw, c.1 - hh),
                    (c.0 + hw, c.1 + hh),
                    (c.0 - hw, c.1 + hh),
                ];
                let segs = vec![
                    (corners[0], corners[1]),
                    (corners[1], corners[2]),
                    (corners[2], corners[3]),
                    (corners[3], corners[0]),
                ];
                Shape {
                    centres: corners.to_vec(),
                    segs,
                    half: 0,
                    radius: circum(hw, hh),
                }
            } else {
                round(circum(hw, hh))
            }
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = if swapped {
                (height.get(), width.get())
            } else {
                (width.get(), height.get())
            };
            let (hw, hh) = (half_up(w), half_up(h));
            if !right_angle {
                return round(hw.max(hh));
            }
            if hw >= hh {
                let d = hw - hh;
                let (a, b) = ((c.0 - d, c.1), (c.0 + d, c.1));
                Shape {
                    centres: vec![a, b],
                    segs: vec![(a, b)],
                    half: hh,
                    radius: hw,
                }
            } else {
                let d = hh - hw;
                let (a, b) = ((c.0, c.1 - d), (c.0, c.1 + d));
                Shape {
                    centres: vec![a, b],
                    segs: vec![(a, b)],
                    half: hw,
                    radius: hh,
                }
            }
        }
    }
}

/// The pieces of an arc as segments with the half width that covers the arc between them.
fn arc_pieces(a: P, b: P, sweep_udeg: i64, half: i64, piece_degrees: u32) -> Vec<(P, P, i64)> {
    let chord = {
        let dx = i128::from(b.0) - i128::from(a.0);
        let dy = i128::from(b.1) - i128::from(a.1);
        let n = dx * dx + dy * dy;
        let r = isqrt(n);
        i64::try_from(if r * r < n { r + 1 } else { r }).unwrap_or(i64::MAX)
    };
    let theta = (sweep_udeg.unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
    let sin_half = (theta / 2.0).sin();
    if chord == 0 || theta.is_nan() || theta <= 0.0 || sin_half.is_nan() || sin_half <= 1e-9 {
        return vec![(a, b, half.saturating_add(chord.max(1)))];
    }
    let radius = (chord as f64) / (2.0 * sin_half);
    let sign = if sweep_udeg < 0 { -1.0 } else { 1.0 };
    let (dx, dy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
    let (ux, uy) = (dx / chord as f64, dy / chord as f64);
    // the centre is to the left of the chord for a counter-clockwise sweep below a half turn
    let offset = sign * radius * (theta / 2.0).cos();
    let mid = (
        (a.0 as f64 + b.0 as f64) / 2.0,
        (a.1 as f64 + b.1 as f64) / 2.0,
    );
    let centre = (mid.0 - uy * offset, mid.1 + ux * offset);
    let first = (a.1 as f64 - centre.1).atan2(a.0 as f64 - centre.0);
    let piece_angle = f64::from(piece_degrees.max(1)) * PI / 180.0;
    let pieces = ((theta / piece_angle).ceil() as usize).clamp(1, 100_000);
    let step = theta / pieces as f64;
    let sagitta = (radius * (1.0 - (step / 2.0).cos())).ceil() as i64;
    let mut points: Vec<P> = Vec::with_capacity(pieces + 1);
    points.push(a);
    for k in 1..pieces {
        let phi = first + sign * step * k as f64;
        points.push((
            (centre.0 + radius * phi.cos()).round() as i64,
            (centre.1 + radius * phi.sin()).round() as i64,
        ));
    }
    points.push(b);
    let widened = half.saturating_add(sagitta).saturating_add(2);
    points
        .windows(2)
        .map(|w| (w[0], w[1], widened))
        .collect::<Vec<_>>()
}

/// The copper layers of the board.
pub(crate) fn copper_layers(board: &Board) -> BTreeSet<Id> {
    board
        .layers
        .iter()
        .filter(|(_, layer)| layer.kind == LayerKind::Copper)
        .map(|(id, _)| *id)
        .collect()
}

/// The layers a via joins: all copper layers between its ends when the board gives the stack,
/// else the two ends.
fn via_layers(board: &Board, from: Id, to: Id) -> Vec<Id> {
    let from_at = board.copper_stack.iter().position(|l| *l == from);
    let to_at = board.copper_stack.iter().position(|l| *l == to);
    match (from_at, to_at) {
        (Some(a), Some(b)) => board.copper_stack[a.min(b)..=a.max(b)].to_vec(),
        _ => {
            let mut ends = vec![from, to];
            ends.dedup();
            ends
        }
    }
}

/// Every pad, track and via of the board, by copper layer.
pub(crate) fn collect(board: &Board, piece_degrees: u32) -> LayerItems {
    let copper = copper_layers(board);
    let mut out: LayerItems = copper.iter().map(|id| (*id, Vec::new())).collect();
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let at = pair(at);
            let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
            let shape = pad_geometry(pad.shape, at, total);
            let label = format!("{} pad {}", component.refdes, pad.number);
            for layer in &pad.layers {
                if let Some(list) = out.get_mut(layer) {
                    list.push(make(
                        Kind::Pad,
                        board.net_of(pad_ref),
                        label.clone(),
                        shape.clone(),
                        at,
                    ));
                }
            }
        }
    }
    for (tid, track) in &board.tracks {
        let Some(list) = out.get_mut(&track.layer) else {
            continue;
        };
        let half = half_up(track.width.get());
        for segment in &track.path {
            match segment {
                Segment::Line { start, end } => {
                    let (a, b) = (pair(*start), pair(*end));
                    let shape = Shape {
                        centres: vec![a, b],
                        segs: vec![(a, b)],
                        half,
                        radius: 0,
                    };
                    list.push(make(
                        Kind::Track,
                        Some(track.net),
                        format!("track {tid}"),
                        shape,
                        a,
                    ));
                }
                Segment::Arc { start, end, sweep } => {
                    let (a, b) = (pair(*start), pair(*end));
                    for (p, q, widened) in arc_pieces(a, b, sweep.get(), half, piece_degrees) {
                        let shape = Shape {
                            centres: vec![p, q],
                            segs: vec![(p, q)],
                            half: widened,
                            radius: 0,
                        };
                        let mut item = make(
                            Kind::Track,
                            Some(track.net),
                            format!("track {tid}"),
                            shape,
                            p,
                        );
                        item.arc = true;
                        list.push(item);
                    }
                }
            }
        }
    }
    for (vid, via) in &board.vias {
        let at = pair(via.at);
        let half = half_up(via.diameter.get());
        for layer in via_layers(board, via.from_layer, via.to_layer) {
            if let Some(list) = out.get_mut(&layer) {
                let shape = Shape {
                    centres: vec![at],
                    segs: vec![(at, at)],
                    half,
                    radius: half,
                };
                list.push(make(
                    Kind::Via,
                    Some(via.net),
                    format!("via {vid}"),
                    shape,
                    at,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::Nm;

    #[test]
    fn halves_are_rounded_up_and_the_circle_around_a_rectangle_holds_its_corner() {
        assert_eq!(half_up(5), 3);
        assert_eq!(half_up(6), 3);
        assert_eq!(circum(3, 4), 5);
        assert_eq!(circum(1, 1), 2);
    }

    #[test]
    fn a_pad_at_a_right_angle_keeps_its_shape_and_another_angle_a_circle() {
        let c = (1_000, 2_000);
        let rect = PadShape::Rect {
            width: Nm::new(400).unwrap_or(Nm::ZERO),
            height: Nm::new(200).unwrap_or(Nm::ZERO),
        };
        let flat = pad_geometry(rect, c, 0);
        assert_eq!(flat.centres.len(), 4);
        assert_eq!(flat.segs.len(), 4);
        assert_eq!(flat.half, 0);
        assert_eq!(flat.radius, circum(200, 100));
        // turned a quarter, the long side is vertical
        let turned = pad_geometry(rect, c, 90_000_000);
        assert_eq!(turned.centres[0], (c.0 - 100, c.1 - 200));
        // turned 30 degrees: the smallest circle around it
        let tilted = pad_geometry(rect, c, 30_000_000);
        assert_eq!(tilted.centres, vec![c]);
        assert_eq!(tilted.segs, vec![(c, c)]);
        assert_eq!(tilted.half, tilted.radius);
        assert_eq!(tilted.radius, circum(200, 100));
    }

    #[test]
    fn an_oblong_is_its_axis_with_the_round_ends_as_the_half_width() {
        let oblong = PadShape::Oblong {
            width: Nm::new(1_000).unwrap_or(Nm::ZERO),
            height: Nm::new(400).unwrap_or(Nm::ZERO),
        };
        let lying = pad_geometry(oblong, (0, 0), 0);
        assert_eq!(lying.centres, vec![(-300, 0), (300, 0)]);
        assert_eq!(lying.segs, vec![((-300, 0), (300, 0))]);
        assert_eq!(lying.half, 200);
        assert_eq!(lying.radius, 500);
        let standing = pad_geometry(oblong, (0, 0), 270_000_000);
        assert_eq!(standing.centres, vec![(0, -300), (0, 300)]);
        assert_eq!(standing.half, 200);
    }

    #[test]
    fn an_arc_is_covered_by_pieces_widened_by_their_sagitta() {
        // a quarter circle of radius about 10 mm in pieces of at most 12 degrees: eight of them
        let pieces = arc_pieces((10_000_000, 0), (0, 10_000_000), 90_000_000, 100_000, 12);
        assert_eq!(pieces.len(), 8);
        for (_, _, widened) in &pieces {
            assert!(*widened > 100_000);
        }
        assert_eq!(pieces[0].0, (10_000_000, 0));
        assert_eq!(pieces[7].1, (0, 10_000_000));
        // an arc without a sweep is its chord widened by the chord
        let flat = arc_pieces((0, 0), (1_000, 0), 0, 100, 10);
        assert_eq!(flat, vec![((0, 0), (1_000, 0), 1_100)]);
    }
}
