// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper a new track has to keep away from: the pads, tracks and vias of other nets, per copper
//! layer, in an index of squares, and the edge of the board.
//!
//! Every obstacle is a segment with a half width (a pad or a via is a segment of one point). A pad
//! is taken as the smallest circle around it, an arc as a short polyline whose pieces are widened
//! by their sagitta, so the answer "clear" is always safe and the answer "not clear" may be
//! cautious. Distances are compared exactly (see `geom`).

use std::collections::BTreeMap;
use std::f64::consts::PI;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment};
use eh3d_phoenix_model::Point;

use crate::geom::{half_up, inside_polygon, isqrt, pair, seg_len, seg_seg_ge, P};

/// One obstacle: a segment widened by `half` on each side.
struct Item {
    a: P,
    b: P,
    half: i64,
}

/// The obstacles of one layer, indexed by squares of a fixed side.
struct LayerObs {
    items: Vec<Item>,
    buckets: BTreeMap<(i64, i64), Vec<usize>>,
    size: i64,
}

impl LayerObs {
    fn new(size: i64) -> LayerObs {
        LayerObs {
            items: Vec::new(),
            buckets: BTreeMap::new(),
            size,
        }
    }

    fn add(&mut self, a: P, b: P, half: i64) {
        let index = self.items.len();
        self.items.push(Item { a, b, half });
        let (x0, x1) = (a.0.min(b.0) - half, a.0.max(b.0) + half);
        let (y0, y1) = (a.1.min(b.1) - half, a.1.max(b.1) + half);
        for bx in x0.div_euclid(self.size)..=x1.div_euclid(self.size) {
            for by in y0.div_euclid(self.size)..=y1.div_euclid(self.size) {
                self.buckets.entry((bx, by)).or_default().push(index);
            }
        }
    }

    /// True when the segment `ab` keeps `reach` away from the edge of every obstacle.
    fn clear(&self, a: P, b: P, reach: i64) -> bool {
        let (x0, x1) = (a.0.min(b.0) - reach, a.0.max(b.0) + reach);
        let (y0, y1) = (a.1.min(b.1) - reach, a.1.max(b.1) + reach);
        for bx in x0.div_euclid(self.size)..=x1.div_euclid(self.size) {
            for by in y0.div_euclid(self.size)..=y1.div_euclid(self.size) {
                let Some(list) = self.buckets.get(&(bx, by)) else {
                    continue;
                };
                for index in list {
                    let item = &self.items[*index];
                    let limit = i128::from(item.half) + i128::from(reach);
                    if !seg_seg_ge(a, b, item.a, item.b, limit) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// The smallest circle around a pad, as a radius rounded up.
fn pad_radius(shape: PadShape) -> i64 {
    match shape {
        PadShape::Circle { diameter } => half_up(diameter.get()),
        PadShape::Oblong { width, height } => half_up(width.get().max(height.get())),
        PadShape::Rect { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            let n = w * w + h * h;
            let r = isqrt(n);
            let r = if r * r < n { r + 1 } else { r };
            half_up(i64::try_from(r).unwrap_or(i64::MAX))
        }
    }
}

/// Adds an arc as a polyline of short chords, each widened by its sagitta. An arc whose shape
/// cannot be worked out is replaced by its chord widened by the chord itself.
fn add_arc(layer: &mut LayerObs, start: Point, end: Point, sweep_udeg: i64, half: i64) {
    let (a, b) = (pair(start), pair(end));
    let chord = seg_len(a, b);
    let theta = (sweep_udeg.unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
    let sin_half = (theta / 2.0).sin();
    if chord == 0 || theta.is_nan() || theta <= 0.0 || sin_half.is_nan() || sin_half <= 1e-9 {
        layer.add(a, b, half.saturating_add(chord.max(1)));
        return;
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
    let pieces = ((theta / (PI / 18.0)).ceil() as usize).max(1);
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
    for pair_of_points in points.windows(2) {
        layer.add(pair_of_points[0], pair_of_points[1], widened);
    }
}

/// The obstacles for a track or a via of one net.
pub(crate) struct Space {
    obs: Vec<LayerObs>,
    outline: Vec<P>,
    half_track: i64,
    via_half: i64,
    clear: i64,
    edge_clear: i64,
}

/// What a `Space` is built from.
pub(crate) struct SpaceSpec {
    /// The copper layers, in stack order.
    pub layers: Vec<Id>,
    /// The net being routed: its copper is no obstacle.
    pub net: Id,
    /// Half the width of the track.
    pub half_track: i64,
    /// Half the diameter of a via.
    pub via_half: i64,
    /// The least gap to copper of another net, margin included.
    pub clear: i64,
    /// The least gap to the edge of the board, margin included.
    pub edge_clear: i64,
    /// The side of the squares of the index.
    pub bucket: i64,
}

impl Space {
    pub(crate) fn build(board: &Board, spec: &SpaceSpec) -> Space {
        let mut index: BTreeMap<Id, usize> = BTreeMap::new();
        for (i, layer) in spec.layers.iter().enumerate() {
            index.insert(*layer, i);
        }
        let mut obs: Vec<LayerObs> = spec
            .layers
            .iter()
            .map(|_| LayerObs::new(spec.bucket))
            .collect();
        for (cid, component) in &board.components {
            let Some(footprint) = board.footprints.get(&component.footprint) else {
                continue;
            };
            for (pid, pad) in &footprint.pads {
                let pad_ref = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                if board.net_of(pad_ref) == Some(spec.net) {
                    continue;
                }
                let Some(at) = pad_position(board, pad_ref) else {
                    continue;
                };
                let radius = pad_radius(pad.shape);
                for layer in &pad.layers {
                    if let Some(i) = index.get(layer) {
                        obs[*i].add(pair(at), pair(at), radius);
                    }
                }
            }
        }
        for track in board.tracks.values() {
            if track.net == spec.net {
                continue;
            }
            let Some(i) = index.get(&track.layer) else {
                continue;
            };
            let half = half_up(track.width.get());
            for segment in &track.path {
                match segment {
                    Segment::Line { start, end } => obs[*i].add(pair(*start), pair(*end), half),
                    Segment::Arc { start, end, sweep } => {
                        add_arc(&mut obs[*i], *start, *end, sweep.get(), half)
                    }
                }
            }
        }
        for via in board.vias.values() {
            if via.net == spec.net {
                continue;
            }
            let half = half_up(via.diameter.get());
            for layer in [via.from_layer, via.to_layer] {
                if let Some(i) = index.get(&layer) {
                    obs[*i].add(pair(via.at), pair(via.at), half);
                }
            }
        }
        let outline: Vec<P> = if board.outline.len() >= 3 {
            board
                .outline_polygon(&eh3d_phoenix_board::ArcSettings::default())
                .iter()
                .map(|p| pair(*p))
                .collect()
        } else {
            Vec::new()
        };
        Space {
            obs,
            outline,
            half_track: spec.half_track,
            via_half: spec.via_half,
            clear: spec.clear,
            edge_clear: spec.edge_clear,
        }
    }

    /// The outline of the board as pairs; empty when the board has none.
    pub(crate) fn outline(&self) -> &[P] {
        &self.outline
    }

    fn edges_ok(&self, a: P, b: P, reach: i64) -> bool {
        let n = self.outline.len();
        if n < 3 {
            return true;
        }
        if !inside_polygon(a, &self.outline) || !inside_polygon(b, &self.outline) {
            return false;
        }
        let r = i128::from(reach);
        (0..n).all(|i| seg_seg_ge(a, b, self.outline[i], self.outline[(i + 1) % n], r))
    }

    /// True when a track of the width of the route may run from `a` to `b` on layer number `layer`.
    pub(crate) fn seg_clear(&self, layer: usize, a: P, b: P) -> bool {
        self.obs[layer].clear(a, b, self.half_track.saturating_add(self.clear))
            && self.edges_ok(a, b, self.half_track.saturating_add(self.edge_clear))
    }

    /// True when a via may stand at `p`, joining the layers numbered `l1` and `l2`.
    pub(crate) fn via_clear(&self, l1: usize, l2: usize, p: P) -> bool {
        let reach = self.via_half.saturating_add(self.clear);
        self.obs[l1].clear(p, p, reach)
            && self.obs[l2].clear(p, p, reach)
            && self.edges_ok(p, p, self.via_half.saturating_add(self.edge_clear))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_finds_what_a_scan_finds() {
        let mut layer = LayerObs::new(10_000);
        layer.add((0, 0), (0, 0), 100);
        layer.add((50_000, 0), (90_000, 40_000), 50);
        layer.add((-30_000, -30_000), (-30_000, 30_000), 500);
        for (a, b, reach, expected) in [
            ((1_000, 0), (5_000, 0), 800, true),
            ((1_000, 0), (5_000, 0), 901, false),
            ((0, 5_000), (20_000, 5_000), 100, true),
            ((60_000, 0), (60_000, 20_000), 100, false),
            ((-29_500, -40_000), (-29_500, 40_000), 400, false),
            ((-28_000, -40_000), (-28_000, 40_000), 400, true),
        ] {
            assert_eq!(layer.clear(a, b, reach), expected, "{a:?} {b:?} {reach}");
        }
    }

    #[test]
    fn a_pad_is_the_smallest_circle_around_it() {
        use eh3d_phoenix_model::Nm;
        let nm = |v: i64| Nm::new(v).unwrap();
        assert_eq!(pad_radius(PadShape::Circle { diameter: nm(600) }), 300);
        assert_eq!(pad_radius(PadShape::Circle { diameter: nm(601) }), 301);
        assert_eq!(
            pad_radius(PadShape::Oblong {
                width: nm(2000),
                height: nm(500)
            }),
            1000
        );
        // 600 by 800 has a diagonal of 1000
        assert_eq!(
            pad_radius(PadShape::Rect {
                width: nm(600),
                height: nm(800)
            }),
            500
        );
    }

    #[test]
    fn an_arc_is_covered_by_its_polyline() {
        // a half turn from (0,0) to (10000,0), counter-clockwise: it bulges below the chord
        let mut layer = LayerObs::new(10_000);
        let start = Point::from_nm(0, 0).unwrap();
        let end = Point::from_nm(10_000, 0).unwrap();
        add_arc(&mut layer, start, end, 180_000_000, 100);
        assert!(!layer.clear((5_000, -5_000), (5_000, -5_000), 200));
        assert!(layer.clear((5_000, -6_000), (5_000, -6_000), 200));
        assert!(layer.clear((5_000, 3_000), (5_000, 3_000), 200));
        assert!(layer.items.len() >= 2);
    }
}
