// SPDX-License-Identifier: GPL-3.0-or-later
//! The approximate distance between two footprints or two symbols, as a score from 0 to 100.
//!
//! The pads (or pins) are matched by position and size, the outline and the silkscreen (or the
//! body of a symbol) are compared as sampled points. Each part gives a share from 0 to 1 and the
//! score is their weighted mean. The score is only a proposal for the owner to confirm; nothing
//! is merged because of it.

use std::collections::BTreeMap;

use crate::hash::Transform;
use crate::model::Prim;
use crate::settings::SanitizerSettings;

/// What is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A footprint: pads, outline and silkscreen.
    Footprint,
    /// A symbol: pins and body.
    Symbol,
}

/// A pad, a pin or a hole: a point that has to be matched by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    /// The position.
    pub x: i64,
    /// The position.
    pub y: i64,
    /// 0 through-hole pad, 1 surface-mount pad, 2 pin, 3 hole.
    pub kind: u8,
    /// The width.
    pub w: i64,
    /// The height.
    pub h: i64,
    /// The drill.
    pub drill: i64,
}

/// The features of an item that the comparison uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Features {
    /// Pads, pins and holes.
    pub anchors: Vec<Anchor>,
    /// The sampled points of each class of drawing: outline and silkscreen of a footprint, or the
    /// body of a symbol.
    pub classes: Vec<Vec<(i64, i64)>>,
}

type Pt = (f64, f64);

/// A piece of drawing to be sampled.
enum Seg {
    Line(Pt, Pt),
    Arc {
        centre: Pt,
        radius: f64,
        start: f64,
        sweep: f64,
    },
    Circle(Pt, f64),
}

impl Seg {
    fn length(&self) -> f64 {
        match self {
            Seg::Line(a, b) => (b.0 - a.0).hypot(b.1 - a.1),
            Seg::Arc { radius, sweep, .. } => radius * sweep.abs(),
            Seg::Circle(_, r) => std::f64::consts::TAU * r,
        }
    }

    /// The point at a fraction `t` of the way along the piece.
    fn at(&self, t: f64) -> Pt {
        match self {
            Seg::Line(a, b) => (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
            Seg::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                let a = start + sweep * t;
                (centre.0 + radius * a.cos(), centre.1 + radius * a.sin())
            }
            Seg::Circle(c, r) => {
                let a = std::f64::consts::TAU * t;
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            }
        }
    }
}

fn to_f(x: i64, y: i64) -> Pt {
    (x as f64, y as f64)
}

/// A line or an arc between two points; `curve` is the angle of the arc in micro-degrees.
fn line_or_arc(p: Pt, q: Pt, curve: i64) -> Seg {
    let theta = curve as f64 / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let half_chord = (q.0 - p.0).hypot(q.1 - p.1) / 2.0;
    if theta.abs() < 1e-9 || half_chord == 0.0 {
        return Seg::Line(p, q);
    }
    let tan = (theta / 2.0).tan();
    let sin = (theta / 2.0).sin();
    if tan.abs() < 1e-9 || sin.abs() < 1e-9 {
        return Seg::Line(p, q);
    }
    let mid = ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    let (dx, dy) = (
        (q.0 - p.0) / (2.0 * half_chord),
        (q.1 - p.1) / (2.0 * half_chord),
    );
    let left = (-dy, dx);
    let d = half_chord / tan;
    let centre = (mid.0 + left.0 * d, mid.1 + left.1 * d);
    let radius = (p.0 - centre.0).hypot(p.1 - centre.1);
    let start = (p.1 - centre.1).atan2(p.0 - centre.0);
    Seg::Arc {
        centre,
        radius,
        start,
        sweep: theta,
    }
}

fn rect_segs(x1: i64, y1: i64, x2: i64, y2: i64, rot: i64) -> Vec<Seg> {
    let c = ((x1 + x2) as f64 / 2.0, (y1 + y2) as f64 / 2.0);
    let (hw, hh) = ((x2 - x1).abs() as f64 / 2.0, (y2 - y1).abs() as f64 / 2.0);
    let a = rot as f64 / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let (s, k) = (a.sin(), a.cos());
    let corner = |sx: f64, sy: f64| {
        let (px, py) = (sx * hw, sy * hh);
        (c.0 + px * k - py * s, c.1 + px * s + py * k)
    };
    let pts = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    (0..4)
        .map(|i| Seg::Line(pts[i], pts[(i + 1) % 4]))
        .collect()
}

fn class_of(prim: &Prim, mode: Mode, s: &SanitizerSettings) -> Option<usize> {
    let layer = match prim {
        Prim::Wire { layer, .. }
        | Prim::Rect { layer, .. }
        | Prim::Circle { layer, .. }
        | Prim::Poly { layer, .. } => *layer,
        _ => return None,
    };
    match mode {
        Mode::Symbol => Some(0),
        Mode::Footprint => {
            if s.outline_layers.contains(&layer) {
                Some(0)
            } else if s.silk_layers.contains(&layer) {
                Some(1)
            } else {
                None
            }
        }
    }
}

fn segs_of(prim: &Prim) -> Vec<Seg> {
    match prim {
        Prim::Wire {
            x1,
            y1,
            x2,
            y2,
            curve,
            ..
        } => vec![line_or_arc(to_f(*x1, *y1), to_f(*x2, *y2), *curve)],
        Prim::Rect {
            x1,
            y1,
            x2,
            y2,
            rot,
            ..
        } => rect_segs(*x1, *y1, *x2, *y2, *rot),
        Prim::Circle { x, y, r, .. } => vec![Seg::Circle(to_f(*x, *y), (*r).abs() as f64)],
        Prim::Poly { verts, .. } => {
            let mut out = Vec::new();
            for (i, v) in verts.iter().enumerate() {
                let next = &verts[(i + 1) % verts.len()];
                out.push(line_or_arc(to_f(v.0, v.1), to_f(next.0, next.1), v.2));
            }
            out
        }
        _ => Vec::new(),
    }
}

/// Samples the pieces at a fixed step, never more than `max_samples` points in all.
fn sample(segs: &[Seg], s: &SanitizerSettings) -> Vec<(i64, i64)> {
    let total: f64 = segs.iter().map(Seg::length).sum();
    let step = (s.sample_step_nm as f64)
        .max(total / s.max_samples as f64)
        .max(1.0);
    let mut out = Vec::new();
    for seg in segs {
        let is_circle = matches!(seg, Seg::Circle(..));
        let n = ((seg.length() / step).ceil() as usize).clamp(1, s.max_samples);
        let points = if is_circle { n.max(8) } else { n + 1 };
        for i in 0..points {
            let t = if is_circle {
                i as f64 / points as f64
            } else {
                i as f64 / n as f64
            };
            let p = seg.at(t);
            out.push((p.0.round() as i64, p.1.round() as i64));
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The features of the drawing items of an item, as they are drawn (no rounding, no transform).
pub fn features(prims: &[Prim], mode: Mode, s: &SanitizerSettings) -> Features {
    let class_count = match mode {
        Mode::Footprint => 2,
        Mode::Symbol => 1,
    };
    let mut anchors = Vec::new();
    let mut segs: Vec<Vec<Seg>> = (0..class_count).map(|_| Vec::new()).collect();
    for prim in prims {
        match prim {
            Prim::Pad {
                thru,
                x,
                y,
                w,
                h,
                drill,
                rot,
                ..
            } => {
                let quarter = rot.rem_euclid(180_000_000) == 90_000_000;
                let (w, h) = if quarter { (*h, *w) } else { (*w, *h) };
                anchors.push(Anchor {
                    x: *x,
                    y: *y,
                    kind: if *thru { 0 } else { 1 },
                    w,
                    h,
                    drill: *drill,
                });
            }
            Prim::Pin { x, y, .. } => anchors.push(Anchor {
                x: *x,
                y: *y,
                kind: 2,
                w: 0,
                h: 0,
                drill: 0,
            }),
            Prim::Hole { x, y, drill } => anchors.push(Anchor {
                x: *x,
                y: *y,
                kind: 3,
                w: *drill,
                h: *drill,
                drill: *drill,
            }),
            other => {
                if let Some(class) = class_of(other, mode, s) {
                    segs[class].extend(segs_of(other));
                }
            }
        }
    }
    anchors.sort_by_key(|a| (a.kind, a.x, a.y, a.w, a.h, a.drill));
    Features {
        anchors,
        classes: segs.iter().map(|c| sample(c, s)).collect(),
    }
}

impl Features {
    /// The same features seen under a transform.
    pub fn turned(&self, t: Transform) -> Features {
        if t.is_identity() {
            return self.clone();
        }
        let odd = t.quarter % 2 == 1;
        let mut anchors: Vec<Anchor> = self
            .anchors
            .iter()
            .map(|a| {
                let (x, y) = t.point(a.x, a.y);
                let (w, h) = if odd { (a.h, a.w) } else { (a.w, a.h) };
                Anchor { x, y, w, h, ..*a }
            })
            .collect();
        anchors.sort_by_key(|a| (a.kind, a.x, a.y, a.w, a.h, a.drill));
        Features {
            anchors,
            classes: self
                .classes
                .iter()
                .map(|c| c.iter().map(|p| t.point(p.0, p.1)).collect())
                .collect(),
        }
    }

    /// The size of the box that holds the anchors.
    pub fn extent(&self) -> (i64, i64) {
        if self.anchors.is_empty() {
            return (0, 0);
        }
        let xs = self.anchors.iter().map(|a| a.x);
        let ys = self.anchors.iter().map(|a| a.y);
        let w = xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0);
        let h = ys.clone().max().unwrap_or(0) - ys.min().unwrap_or(0);
        (w, h)
    }
}

fn dist2(a: (i64, i64), b: (i64, i64)) -> i128 {
    let dx = i128::from(a.0) - i128::from(b.0);
    let dy = i128::from(a.1) - i128::from(b.1);
    dx * dx + dy * dy
}

type Grid<T> = BTreeMap<(i64, i64), Vec<T>>;

fn cell(p: (i64, i64), size: i64) -> (i64, i64) {
    (p.0.div_euclid(size), p.1.div_euclid(size))
}

/// The share of the points of `a` that have a point of `b` within `tol`.
fn cover(a: &[(i64, i64)], b: &[(i64, i64)], tol: i64) -> f64 {
    if a.is_empty() {
        return 1.0;
    }
    let mut grid: Grid<(i64, i64)> = BTreeMap::new();
    for p in b {
        grid.entry(cell(*p, tol)).or_default().push(*p);
    }
    let limit = i128::from(tol) * i128::from(tol);
    let mut hits = 0usize;
    for p in a {
        let (cx, cy) = cell(*p, tol);
        let near = (cx - 1..=cx + 1).any(|ix| {
            (cy - 1..=cy + 1).any(|iy| {
                grid.get(&(ix, iy))
                    .is_some_and(|v| v.iter().any(|q| dist2(*p, *q) <= limit))
            })
        });
        if near {
            hits += 1;
        }
    }
    hits as f64 / a.len() as f64
}

/// How well two sets of points cover each other, 0 to 1; `None` when both are empty.
fn class_share(a: &[(i64, i64)], b: &[(i64, i64)], tol: i64) -> Option<f64> {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => None,
        (false, false) => Some(cover(a, b, tol).min(cover(b, a, tol))),
        _ => Some(0.0),
    }
}

/// The best share of the anchors of `a` that find a partner in `b`: the sum of the scores of the
/// pairs, divided by the number of anchors.
fn anchor_share(a: &[Anchor], b: &[Anchor], s: &SanitizerSettings) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    if a.is_empty() {
        return 1.0;
    }
    let tol = s.tolerance_position_nm;
    let mut grid: Grid<usize> = BTreeMap::new();
    for (j, q) in b.iter().enumerate() {
        grid.entry(cell((q.x, q.y), tol)).or_default().push(j);
    }
    let limit = i128::from(tol) * i128::from(tol);
    let mut candidates: Vec<(i128, usize, usize)> = Vec::new();
    for (i, p) in a.iter().enumerate() {
        let (cx, cy) = cell((p.x, p.y), tol);
        for ix in cx - 1..=cx + 1 {
            for iy in cy - 1..=cy + 1 {
                if let Some(list) = grid.get(&(ix, iy)) {
                    for &j in list {
                        let d = dist2((p.x, p.y), (b[j].x, b[j].y));
                        if b[j].kind == p.kind && d <= limit {
                            candidates.push((d, i, j));
                        }
                    }
                }
            }
        }
    }
    candidates.sort_unstable();
    let mut used_a = vec![false; a.len()];
    let mut used_b = vec![false; b.len()];
    let mut total = 0.0;
    for (d, i, j) in candidates {
        if used_a[i] || used_b[j] {
            continue;
        }
        used_a[i] = true;
        used_b[j] = true;
        let pos = 1.0 - (d as f64).sqrt() / tol as f64;
        let diff = (a[i].w - b[j].w)
            .abs()
            .max((a[i].h - b[j].h).abs())
            .max((a[i].drill - b[j].drill).abs());
        let size = 1.0 - diff as f64 / s.tolerance_size_nm as f64;
        total += pos.clamp(0.0, 1.0) * size.clamp(0.0, 1.0);
    }
    total / a.len() as f64
}

/// The weights of the parts: anchors first, then the classes of drawing.
fn weights(mode: Mode, s: &SanitizerSettings) -> Vec<f64> {
    match mode {
        Mode::Footprint => vec![
            f64::from(s.weight_pads),
            f64::from(s.weight_outline),
            f64::from(s.weight_silk),
        ],
        Mode::Symbol => vec![
            f64::from(s.weight_symbol_pins),
            f64::from(s.weight_symbol_body),
        ],
    }
}

/// The largest score that two items can still reach when the boxes of their anchors differ by
/// more than two tolerances: at least one anchor is then left without a partner.
fn bound_when_boxes_differ(a: &Features, b: &Features, mode: Mode, s: &SanitizerSettings) -> f64 {
    let (ea, eb) = (a.extent(), b.extent());
    let slack = 2 * s.tolerance_position_nm;
    let differ = (ea.0 - eb.0).abs() > slack || (ea.1 - eb.1).abs() > slack;
    if !differ || a.anchors.is_empty() {
        return 1.0;
    }
    let w = weights(mode, s);
    let total: f64 = w.iter().sum();
    if total == 0.0 {
        return 1.0;
    }
    let n = a.anchors.len() as f64;
    (total - w[0] / n) / total
}

/// The score of `a` against `b` as they are, 0 to 100.
pub fn score_as_is(a: &Features, b: &Features, mode: Mode, s: &SanitizerSettings) -> u32 {
    let w = weights(mode, s);
    let mut sum = 0.0;
    let mut total = 0.0;
    if !a.anchors.is_empty() || !b.anchors.is_empty() {
        sum += w[0] * anchor_share(&a.anchors, &b.anchors, s);
        total += w[0];
    }
    for (i, wc) in w.iter().enumerate().skip(1) {
        let (ca, cb) = (&a.classes[i - 1], &b.classes[i - 1]);
        if let Some(share) = class_share(ca, cb, s.tolerance_outline_nm) {
            sum += wc * share;
            total += wc;
        }
    }
    if total == 0.0 {
        return 100;
    }
    ((100.0 * sum / total).round() as u32).min(100)
}

/// The best score of `a` against `b` under the transforms that the settings allow, and the
/// transform of `b` that gave it. A pair that cannot reach `floor` under a transform is not
/// worked out under it.
pub fn best_score(
    a: &Features,
    b: &Features,
    mode: Mode,
    s: &SanitizerSettings,
    floor: u32,
) -> (u32, Transform) {
    let mut best = (0, Transform::IDENTITY);
    for t in Transform::allowed(s.allow_rotation, s.allow_mirror) {
        let turned = b.turned(t);
        if bound_when_boxes_differ(a, &turned, mode, s) * 100.0 + 0.5 < f64::from(floor) {
            continue;
        }
        let score = score_as_is(a, &turned, mode, s);
        if score > best.0 {
            best = (score, t);
            if score == 100 {
                break;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smd(x: i64, y: i64, w: i64, h: i64) -> Prim {
        Prim::Pad {
            name: "1".into(),
            thru: false,
            x,
            y,
            w,
            h,
            drill: 0,
            shape: "smd0".into(),
            layer: 1,
            rot: 0,
        }
    }

    fn line(layer: u32, x1: i64, y1: i64, x2: i64, y2: i64) -> Prim {
        Prim::Wire {
            layer,
            x1,
            y1,
            x2,
            y2,
            width: 100_000,
            curve: 0,
        }
    }

    fn chip(dx: i64, pad_w: i64) -> Vec<Prim> {
        vec![
            smd(-900_000 + dx, 0, pad_w, 1_400_000),
            smd(900_000, 0, pad_w, 1_400_000),
            line(21, -1_500_000, 800_000, 1_500_000, 800_000),
            line(21, -1_500_000, -800_000, 1_500_000, -800_000),
            line(39, -2_000_000, -1_200_000, 2_000_000, -1_200_000),
            line(39, 2_000_000, -1_200_000, 2_000_000, 1_200_000),
            line(39, 2_000_000, 1_200_000, -2_000_000, 1_200_000),
            line(39, -2_000_000, 1_200_000, -2_000_000, -1_200_000),
        ]
    }

    fn feat(prims: &[Prim], s: &SanitizerSettings) -> Features {
        features(prims, Mode::Footprint, s)
    }

    #[test]
    fn identical_items_score_100_and_a_small_shift_scores_a_little_less() {
        let s = SanitizerSettings::default();
        let a = feat(&chip(0, 1_200_000), &s);
        assert_eq!(score_as_is(&a, &a, Mode::Footprint, &s), 100);
        let near = feat(&chip(20_000, 1_200_000), &s);
        let score = score_as_is(&a, &near, Mode::Footprint, &s);
        assert!((90..100).contains(&score), "{score}");
        let wider = feat(&chip(0, 1_210_000), &s);
        let score2 = score_as_is(&a, &wider, Mode::Footprint, &s);
        assert!((90..100).contains(&score2), "{score2}");
    }

    #[test]
    fn items_that_differ_a_lot_score_low() {
        let s = SanitizerSettings::default();
        let a = feat(&chip(0, 1_200_000), &s);
        let mut other = chip(0, 1_200_000);
        other[0] = smd(-3_000_000, 0, 1_200_000, 1_400_000);
        let far = feat(&other, &s);
        let score = score_as_is(&a, &far, Mode::Footprint, &s);
        assert!(score <= 75, "{score}");
        // a different number of pads is never a match
        let three = feat(&[smd(0, 0, 1, 1), smd(1, 0, 1, 1), smd(2, 0, 1, 1)], &s);
        let two = feat(&[smd(0, 0, 1, 1), smd(1, 0, 1, 1)], &s);
        assert_eq!(anchor_share(&three.anchors, &two.anchors, &s), 0.0);
    }

    #[test]
    fn a_missing_outline_costs_its_weight() {
        let s = SanitizerSettings::default();
        let a = feat(&chip(0, 1_200_000), &s);
        let bare = feat(&chip(0, 1_200_000)[..2], &s);
        let score = score_as_is(&a, &bare, Mode::Footprint, &s);
        assert_eq!(score, 50);
    }

    #[test]
    fn a_turned_copy_scores_100_only_when_the_settings_allow_the_turn() {
        let s = SanitizerSettings::default();
        let a = feat(&chip(0, 1_200_000), &s);
        let turned = a.turned(Transform {
            quarter: 1,
            mirror: false,
        });
        let (plain, _) = best_score(&a, &turned, Mode::Footprint, &s, 0);
        assert!(plain < 100);
        let free = SanitizerSettings {
            allow_rotation: true,
            ..SanitizerSettings::default()
        };
        let (score, t) = best_score(&a, &turned, Mode::Footprint, &free, 0);
        assert_eq!(score, 100);
        assert!(!t.is_identity());
    }

    #[test]
    fn the_bound_skips_pairs_that_cannot_reach_the_floor() {
        let s = SanitizerSettings::default();
        let a = feat(
            &[
                smd(0, 0, 1_000_000, 1_000_000),
                smd(1_000_000, 0, 1_000_000, 1_000_000),
            ],
            &s,
        );
        let b = feat(
            &[
                smd(0, 0, 1_000_000, 1_000_000),
                smd(5_000_000, 0, 1_000_000, 1_000_000),
            ],
            &s,
        );
        // the boxes differ by far more than two tolerances: with two pads the score is at most 75
        let (score, _) = best_score(&a, &b, Mode::Footprint, &s, 90);
        assert_eq!(score, 0);
        let (full, _) = best_score(&a, &b, Mode::Footprint, &s, 0);
        assert!(full > 0 && full <= 75, "{full}");
    }

    #[test]
    fn arcs_circles_and_rectangles_are_sampled() {
        let s = SanitizerSettings::default();
        let arc = Prim::Wire {
            layer: 21,
            x1: 0,
            y1: 0,
            x2: 2_000_000,
            y2: 0,
            width: 0,
            curve: 180_000_000,
        };
        let circle = Prim::Circle {
            layer: 21,
            x: 0,
            y: 0,
            r: 1_000_000,
            width: 0,
        };
        let rect = Prim::Rect {
            layer: 21,
            x1: 0,
            y1: 0,
            x2: 2_000_000,
            y2: 1_000_000,
            rot: 0,
        };
        for p in [arc, circle, rect] {
            let f = feat(&[p], &s);
            assert!(f.classes[1].len() >= 8, "{}", f.classes[1].len());
        }
        // a half circle through the chord: the middle point is one radius from the chord
        let half = Prim::Wire {
            layer: 21,
            x1: 0,
            y1: 0,
            x2: 2_000_000,
            y2: 0,
            width: 0,
            curve: 180_000_000,
        };
        let f = feat(&[half], &s);
        let ys: Vec<i64> = f.classes[1].iter().map(|p| p.1.abs()).collect();
        assert!(ys.iter().any(|y| (*y - 1_000_000).abs() < 10_000));
    }

    #[test]
    fn symbols_compare_pins_and_body() {
        let s = SanitizerSettings::default();
        let pin = |x: i64| Prim::Pin {
            name: "P".into(),
            x,
            y: 0,
            rot: 0,
            length: "short".into(),
            direction: "io".into(),
            function: "none".into(),
            visible: "both".into(),
            swaplevel: "0".into(),
        };
        let body = line(94, 0, -1_000_000, 0, 1_000_000);
        let a = features(&[pin(0), pin(5_000_000), body], Mode::Symbol, &s);
        assert_eq!(score_as_is(&a, &a, Mode::Symbol, &s), 100);
        let moved = features(
            &[
                pin(0),
                pin(5_000_000),
                line(94, 0, -1_000_000, 0, 2_000_000),
            ],
            Mode::Symbol,
            &s,
        );
        let score = score_as_is(&a, &moved, Mode::Symbol, &s);
        assert!((50..100).contains(&score), "{score}");
    }

    #[test]
    fn two_empty_items_are_the_same() {
        let s = SanitizerSettings::default();
        let e = feat(&[], &s);
        assert_eq!(score_as_is(&e, &e, Mode::Footprint, &s), 100);
    }
}
