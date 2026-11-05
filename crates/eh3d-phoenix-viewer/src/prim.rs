// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing primitives: lines, arcs, polygons, circles and text in millimetres, with the
//! geometry that does not depend on a window (boxes, distance from a point, arcs as short lines,
//! the triangles of a polygon). Nothing here knows egui.

use std::f64::consts::{PI, TAU};

/// A point in millimetres; y points up.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct P {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl P {
    /// A point.
    pub fn new(x: f64, y: f64) -> P {
        P { x, y }
    }

    /// The distance to another point.
    pub fn dist(self, other: P) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

/// An axis-aligned box in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The corner with the smallest coordinates.
    pub min: P,
    /// The corner with the largest coordinates.
    pub max: P,
}

impl Bounds {
    /// The box that holds nothing: it grows with the first point included.
    pub const EMPTY: Bounds = Bounds {
        min: P {
            x: f64::INFINITY,
            y: f64::INFINITY,
        },
        max: P {
            x: f64::NEG_INFINITY,
            y: f64::NEG_INFINITY,
        },
    };

    /// True when the box holds nothing.
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    /// Grows the box to hold a point.
    pub fn include(&mut self, p: P) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    /// The smallest box that holds both.
    pub fn union(self, other: Bounds) -> Bounds {
        Bounds {
            min: P::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: P::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// The box grown by `d` on every side.
    pub fn expanded(self, d: f64) -> Bounds {
        Bounds {
            min: P::new(self.min.x - d, self.min.y - d),
            max: P::new(self.max.x + d, self.max.y + d),
        }
    }

    /// True when the boxes share at least a point.
    pub fn intersects(&self, other: &Bounds) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// True when the point is inside or on the border.
    pub fn contains(&self, p: P) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// The width, zero for an empty box.
    pub fn width(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.x - self.min.x
        }
    }

    /// The height, zero for an empty box.
    pub fn height(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.y - self.min.y
        }
    }

    /// The middle of the box.
    pub fn centre(&self) -> P {
        P::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    /// The area of the box.
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// The geometry of a primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A straight line.
    Line {
        /// Start.
        a: P,
        /// End.
        b: P,
    },
    /// A circular arc: the centre, the radius, the angle of its start and its signed sweep, both
    /// in radians (positive is counter-clockwise).
    Arc {
        /// The centre.
        centre: P,
        /// The radius.
        radius: f64,
        /// The angle of the start.
        start: f64,
        /// The signed sweep.
        sweep: f64,
    },
    /// A closed polygon (the last point joins the first).
    Polygon {
        /// The corners.
        points: Vec<P>,
    },
    /// A circle.
    Circle {
        /// The centre.
        centre: P,
        /// The radius.
        radius: f64,
    },
    /// A text, with the lower left corner of its first character.
    Text {
        /// Where it starts.
        at: P,
        /// The height of a capital letter.
        height: f64,
        /// The characters.
        text: String,
    },
}

/// One thing to draw: a shape, how it is drawn, the layer it is on, its net and the item of the
/// document it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Prim {
    /// The geometry.
    pub shape: Shape,
    /// The width of the stroke in millimetres; zero for a thin line of the screen.
    pub width: f64,
    /// True when the inside is painted (a polygon or a circle).
    pub filled: bool,
    /// The index of the layer, which is also the drawing order.
    pub layer: u32,
    /// The index of the net, when it has one.
    pub net: Option<u32>,
    /// The index of the item of the document that it belongs to.
    pub item: u32,
}

/// The width of a character of the text, as a fraction of its height.
const TEXT_ADVANCE: f64 = 0.6;

/// The point of an arc at an angle.
pub fn arc_point(centre: P, radius: f64, angle: f64) -> P {
    P::new(
        centre.x + radius * angle.cos(),
        centre.y + radius * angle.sin(),
    )
}

/// True when `angle` lies on the arc that starts at `start` and sweeps `sweep` (radians).
pub fn angle_in_sweep(angle: f64, start: f64, sweep: f64) -> bool {
    if sweep >= 0.0 {
        (angle - start).rem_euclid(TAU) <= sweep
    } else {
        (start - angle).rem_euclid(TAU) <= -sweep
    }
}

/// The centre, the radius and the start angle of the arc that goes from `start` to `end` with a
/// signed sweep in radians (positive is counter-clockwise). `None` when the ends coincide (the
/// centre of a full turn is not known) or the sweep is zero.
pub fn arc_from_ends(start: P, end: P, sweep: f64) -> Option<(P, f64, f64)> {
    let chord = start.dist(end);
    let half = (sweep / 2.0).abs();
    if chord <= 0.0 || sweep == 0.0 || !sweep.is_finite() || half.sin() <= 0.0 {
        return None;
    }
    let radius = chord / (2.0 * half.sin());
    let mid = P::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    // the unit vector to the left of the chord
    let left = P::new(-(end.y - start.y) / chord, (end.x - start.x) / chord);
    let offset = sweep.signum() * radius * (sweep / 2.0).cos();
    let centre = P::new(mid.x + left.x * offset, mid.y + left.y * offset);
    let first = (start.y - centre.y).atan2(start.x - centre.x);
    Some((centre, radius, first))
}

/// The points of an arc drawn as short lines whose distance from the true arc stays below
/// `tolerance`, with at least `min_segments` and at most `max_segments` lines.
pub fn arc_points(
    centre: P,
    radius: f64,
    start: f64,
    sweep: f64,
    tolerance: f64,
    min_segments: usize,
    max_segments: usize,
) -> Vec<P> {
    let step = if tolerance > 0.0 && radius > tolerance {
        2.0 * (1.0 - tolerance / radius).acos()
    } else {
        PI / 4.0
    };
    let least = min_segments.max(1);
    let most = max_segments.max(least);
    let n = ((sweep.abs() / step).ceil() as usize).clamp(least, most);
    (0..=n)
        .map(|i| arc_point(centre, radius, start + sweep * (i as f64) / (n as f64)))
        .collect()
}

fn segment_distance(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length_squared = dx * dx + dy * dy;
    if length_squared <= 0.0 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length_squared).clamp(0.0, 1.0);
    p.dist(P::new(a.x + t * dx, a.y + t * dy))
}

/// True when the point is inside the polygon (even-odd rule).
pub fn point_in_polygon(p: P, points: &[P]) -> bool {
    let n = points.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points[n - 1];
    for &current in points {
        let (a, b) = (current, previous);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn polygon_edge_distance(p: P, points: &[P]) -> f64 {
    let n = points.len();
    match n {
        0 => f64::INFINITY,
        1 => p.dist(points[0]),
        _ => (0..n)
            .map(|i| segment_distance(p, points[i], points[(i + 1) % n]))
            .fold(f64::INFINITY, f64::min),
    }
}

fn cross(o: P, a: P, b: P) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn in_triangle(p: P, a: P, b: P, c: P) -> bool {
    let d1 = cross(a, b, p);
    let d2 = cross(b, c, p);
    let d3 = cross(c, a, p);
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

/// The triangles (indexes into `points`) that fill a simple polygon, by ear clipping. A closing
/// point equal to the first is ignored; a polygon without area gives no triangle. The cost grows
/// with the cube of the number of points in the worst case, so callers bound it.
pub fn triangulate(points: &[P]) -> Vec<[u32; 3]> {
    let mut n = points.len();
    if n > 1 && points[0] == points[n - 1] {
        n -= 1;
    }
    if n < 3 {
        return Vec::new();
    }
    let mut area2 = 0.0;
    for (i, a) in points.iter().take(n).enumerate() {
        let b = points[(i + 1) % n];
        area2 += a.x * b.y - b.x * a.y;
    }
    if area2 == 0.0 {
        return Vec::new();
    }
    let ccw = area2 > 0.0;
    let mut ring: Vec<usize> = (0..n).collect();
    let mut out: Vec<[u32; 3]> = Vec::with_capacity(n - 2);
    while ring.len() > 3 {
        let m = ring.len();
        let mut clipped = false;
        for i in 0..m {
            let (a, b, c) = (ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]);
            let turn = cross(points[a], points[b], points[c]);
            if (ccw && turn <= 0.0) || (!ccw && turn >= 0.0) {
                continue;
            }
            let blocked = ring.iter().any(|&k| {
                k != a
                    && k != b
                    && k != c
                    && in_triangle(points[k], points[a], points[b], points[c])
            });
            if blocked {
                continue;
            }
            out.push([a as u32, b as u32, c as u32]);
            ring.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // a polygon that crosses itself or has collinear runs: cut the first corner anyway so
            // that the loop ends
            out.push([ring[m - 1] as u32, ring[0] as u32, ring[1] as u32]);
            ring.remove(0);
        }
    }
    if ring.len() == 3 {
        out.push([ring[0] as u32, ring[1] as u32, ring[2] as u32]);
    }
    out
}

impl Shape {
    /// The box of the geometry alone, without the width of the stroke.
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::EMPTY;
        match self {
            Shape::Line { a, b: end } => {
                b.include(*a);
                b.include(*end);
            }
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                b.include(arc_point(*centre, *radius, *start));
                b.include(arc_point(*centre, *radius, start + sweep));
                for k in 0..4 {
                    let angle = f64::from(k) * PI / 2.0;
                    if angle_in_sweep(angle, *start, *sweep) {
                        b.include(arc_point(*centre, *radius, angle));
                    }
                }
            }
            Shape::Polygon { points } => {
                for p in points {
                    b.include(*p);
                }
            }
            Shape::Circle { centre, radius } => {
                b.include(P::new(centre.x - radius, centre.y - radius));
                b.include(P::new(centre.x + radius, centre.y + radius));
            }
            Shape::Text { at, height, text } => {
                let width = TEXT_ADVANCE * height * (text.chars().count() as f64);
                b.include(*at);
                b.include(P::new(at.x + width, at.y + height));
            }
        }
        b
    }
}

impl Prim {
    /// The box of the primitive including half the width of its stroke.
    pub fn bounds(&self) -> Bounds {
        let b = self.shape.bounds();
        if b.is_empty() {
            b
        } else {
            b.expanded(self.width / 2.0)
        }
    }

    /// The distance from a point to what the primitive paints: zero inside a filled shape or
    /// within half the width of a stroke.
    pub fn distance(&self, p: P) -> f64 {
        let geometric = match &self.shape {
            Shape::Line { a, b } => segment_distance(p, *a, *b),
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                let angle = (p.y - centre.y).atan2(p.x - centre.x);
                if angle_in_sweep(angle, *start, *sweep) {
                    (p.dist(*centre) - radius).abs()
                } else {
                    let first = arc_point(*centre, *radius, *start);
                    let last = arc_point(*centre, *radius, start + sweep);
                    p.dist(first).min(p.dist(last))
                }
            }
            Shape::Polygon { points } => {
                if self.filled && point_in_polygon(p, points) {
                    0.0
                } else {
                    polygon_edge_distance(p, points)
                }
            }
            Shape::Circle { centre, radius } => {
                let d = p.dist(*centre);
                if self.filled {
                    (d - radius).max(0.0)
                } else {
                    (d - radius).abs()
                }
            }
            Shape::Text { .. } => {
                let b = self.shape.bounds();
                let dx = (b.min.x - p.x).max(0.0).max(p.x - b.max.x);
                let dy = (b.min.y - p.y).max(0.0).max(p.y - b.max.y);
                dx.hypot(dy)
            }
        };
        (geometric - self.width / 2.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: (f64, f64), b: (f64, f64), width: f64) -> Prim {
        Prim {
            shape: Shape::Line {
                a: P::new(a.0, a.1),
                b: P::new(b.0, b.1),
            },
            width,
            filled: false,
            layer: 0,
            net: None,
            item: 0,
        }
    }

    fn square() -> Vec<P> {
        vec![
            P::new(0.0, 0.0),
            P::new(4.0, 0.0),
            P::new(4.0, 4.0),
            P::new(0.0, 4.0),
        ]
    }

    #[test]
    fn a_box_grows_with_points_and_unions() {
        let mut b = Bounds::EMPTY;
        assert!(b.is_empty());
        assert_eq!(b.width(), 0.0_f64);
        b.include(P::new(1.0, 2.0));
        b.include(P::new(-1.0, 5.0));
        assert_eq!(b.min, P::new(-1.0, 2.0));
        assert_eq!(b.max, P::new(1.0, 5.0));
        assert_eq!(b.centre(), P::new(0.0, 3.5));
        assert_eq!(b.area(), 6.0_f64);
        let c = Bounds {
            min: P::new(0.0, 0.0),
            max: P::new(9.0, 1.0),
        };
        assert!(b.intersects(&c));
        let u = b.union(c);
        assert_eq!(u.max.x, 9.0_f64);
        assert!(u.contains(P::new(5.0, 3.0)));
        assert!(!b.expanded(0.5).intersects(&Bounds {
            min: P::new(20.0, 20.0),
            max: P::new(21.0, 21.0)
        }));
    }

    #[test]
    fn the_distance_to_a_line_counts_half_its_width() {
        let l = line((0.0, 0.0), (10.0, 0.0), 2.0);
        assert_eq!(l.distance(P::new(5.0, 0.5)), 0.0_f64);
        assert!((l.distance(P::new(5.0, 3.0)) - 2.0).abs() < 1e-12);
        assert!((l.distance(P::new(13.0, 0.0)) - 2.0).abs() < 1e-12);
        let bounds = l.bounds();
        assert_eq!(bounds.min, P::new(-1.0, -1.0));
        assert_eq!(bounds.max, P::new(11.0, 1.0));
    }

    #[test]
    fn a_filled_polygon_is_hit_inside_and_an_open_one_only_on_its_edge() {
        let mut p = Prim {
            shape: Shape::Polygon { points: square() },
            width: 0.0,
            filled: true,
            layer: 0,
            net: None,
            item: 0,
        };
        assert_eq!(p.distance(P::new(2.0, 2.0)), 0.0_f64);
        assert!((p.distance(P::new(6.0, 2.0)) - 2.0).abs() < 1e-12);
        p.filled = false;
        assert!((p.distance(P::new(2.0, 2.0)) - 2.0).abs() < 1e-12);
        assert!(p.distance(P::new(4.0, 2.0)) < 1e-12);
    }

    #[test]
    fn a_circle_is_a_disc_or_a_ring() {
        let mut c = Prim {
            shape: Shape::Circle {
                centre: P::new(1.0, 1.0),
                radius: 2.0,
            },
            width: 0.0,
            filled: true,
            layer: 0,
            net: None,
            item: 0,
        };
        assert_eq!(c.distance(P::new(1.5, 1.0)), 0.0_f64);
        assert!((c.distance(P::new(6.0, 1.0)) - 3.0).abs() < 1e-12);
        c.filled = false;
        assert!((c.distance(P::new(1.0, 1.0)) - 2.0).abs() < 1e-12);
        assert!(c.distance(P::new(3.0, 1.0)) < 1e-12);
    }

    #[test]
    fn an_arc_from_its_ends_has_the_right_centre_and_radius() {
        // a quarter turn counter-clockwise from (1, 0) to (0, 1) about the origin
        let (centre, radius, start) =
            arc_from_ends(P::new(1.0, 0.0), P::new(0.0, 1.0), PI / 2.0).unwrap();
        assert!(centre.dist(P::new(0.0, 0.0)) < 1e-12);
        assert!((radius - 1.0).abs() < 1e-12);
        assert!(start.abs() < 1e-12);
        // the same ends clockwise: the centre is on the other side of the chord
        let (centre, radius, _) =
            arc_from_ends(P::new(1.0, 0.0), P::new(0.0, 1.0), -PI / 2.0).unwrap();
        assert!(centre.dist(P::new(1.0, 1.0)) < 1e-12);
        assert!((radius - 1.0).abs() < 1e-12);
        // a half turn: the centre is the middle of the chord
        let (centre, radius, _) = arc_from_ends(P::new(0.0, 0.0), P::new(2.0, 0.0), PI).unwrap();
        assert!(centre.dist(P::new(1.0, 0.0)) < 1e-12);
        assert!((radius - 1.0).abs() < 1e-12);
        // a three-quarter turn: the centre is on the far side
        let (centre, _, _) = arc_from_ends(P::new(1.0, 0.0), P::new(0.0, 1.0), 1.5 * PI).unwrap();
        assert!(centre.dist(P::new(1.0, 1.0)) < 1e-12);
        assert!(arc_from_ends(P::new(1.0, 0.0), P::new(1.0, 0.0), TAU).is_none());
        assert!(arc_from_ends(P::new(1.0, 0.0), P::new(0.0, 1.0), 0.0).is_none());
    }

    #[test]
    fn the_box_of_an_arc_holds_its_extremes() {
        let arc = Shape::Arc {
            centre: P::new(0.0, 0.0),
            radius: 1.0,
            start: -PI / 4.0,
            sweep: PI / 2.0,
        };
        let b = arc.bounds();
        // it passes through angle zero, so it reaches x = 1
        assert!((b.max.x - 1.0).abs() < 1e-12);
        assert!(b.min.x > 0.7);
        assert!(b.max.y < 0.75);
    }

    #[test]
    fn the_distance_to_an_arc_uses_the_ring_inside_its_sweep_and_the_ends_outside() {
        let arc = Prim {
            shape: Shape::Arc {
                centre: P::new(0.0, 0.0),
                radius: 10.0,
                start: 0.0,
                sweep: PI / 2.0,
            },
            width: 0.0,
            filled: false,
            layer: 0,
            net: None,
            item: 0,
        };
        assert!((arc.distance(P::new(5.0, 5.0)) - (50.0_f64.sqrt() - 10.0).abs()).abs() < 1e-9);
        // outside the sweep the nearest end counts: (0, 10) is the nearer one
        assert!((arc.distance(P::new(-3.0, 0.0)) - 109.0_f64.sqrt()).abs() < 1e-9);
        let clockwise = Prim {
            shape: Shape::Arc {
                centre: P::new(0.0, 0.0),
                radius: 10.0,
                start: 0.0,
                sweep: -PI / 2.0,
            },
            ..arc
        };
        assert!(clockwise.distance(P::new(7.0, -7.0)) < 0.2);
        assert!(clockwise.distance(P::new(7.0, 7.0)) > 5.0);
    }

    #[test]
    fn an_arc_is_cut_into_lines_that_stay_within_the_tolerance() {
        let points = arc_points(P::new(0.0, 0.0), 10.0, 0.0, PI, 0.01, 6, 256);
        assert!(points.len() > 7);
        for pair in points.windows(2) {
            let mid = P::new((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
            assert!((10.0 - mid.dist(P::new(0.0, 0.0))).abs() <= 0.0101);
        }
        let first = points[0];
        let last = points[points.len() - 1];
        assert!(first.dist(P::new(10.0, 0.0)) < 1e-9);
        assert!(last.dist(P::new(-10.0, 0.0)) < 1e-9);
        // the limits hold whatever the tolerance
        assert_eq!(
            arc_points(P::new(0.0, 0.0), 10.0, 0.0, 0.1, 0.01, 6, 256).len(),
            7
        );
        assert_eq!(
            arc_points(P::new(0.0, 0.0), 10.0, 0.0, TAU, 1e-9, 6, 64).len(),
            65
        );
        assert_eq!(
            arc_points(P::new(0.0, 0.0), 0.001, 0.0, PI, 0.5, 6, 64).len(),
            7
        );
    }

    #[test]
    fn point_in_polygon_follows_the_even_odd_rule() {
        let s = square();
        assert!(point_in_polygon(P::new(1.0, 1.0), &s));
        assert!(!point_in_polygon(P::new(5.0, 1.0), &s));
        assert!(!point_in_polygon(P::new(1.0, 1.0), &s[..2]));
        // an L shape: the notch is outside
        let l = vec![
            P::new(0.0, 0.0),
            P::new(4.0, 0.0),
            P::new(4.0, 2.0),
            P::new(2.0, 2.0),
            P::new(2.0, 4.0),
            P::new(0.0, 4.0),
        ];
        assert!(point_in_polygon(P::new(1.0, 3.0), &l));
        assert!(!point_in_polygon(P::new(3.0, 3.0), &l));
    }

    fn triangle_area(points: &[P], t: [u32; 3]) -> f64 {
        cross(
            points[t[0] as usize],
            points[t[1] as usize],
            points[t[2] as usize],
        )
        .abs()
            / 2.0
    }

    #[test]
    fn the_triangles_of_a_polygon_cover_exactly_its_area() {
        let s = square();
        let t = triangulate(&s);
        assert_eq!(t.len(), 2);
        let total: f64 = t.iter().map(|&t| triangle_area(&s, t)).sum();
        assert!((total - 16.0).abs() < 1e-9);
        // clockwise gives the same area; a closing point equal to the first is ignored
        let mut reversed: Vec<P> = s.iter().rev().copied().collect();
        reversed.push(reversed[0]);
        let t = triangulate(&reversed);
        let total: f64 = t.iter().map(|&t| triangle_area(&reversed, t)).sum();
        assert!((total - 16.0).abs() < 1e-9);
        // the L shape has area 12 and four triangles
        let l = vec![
            P::new(0.0, 0.0),
            P::new(4.0, 0.0),
            P::new(4.0, 2.0),
            P::new(2.0, 2.0),
            P::new(2.0, 4.0),
            P::new(0.0, 4.0),
        ];
        let t = triangulate(&l);
        assert_eq!(t.len(), 4);
        let total: f64 = t.iter().map(|&t| triangle_area(&l, t)).sum();
        assert!((total - 12.0).abs() < 1e-9);
    }

    #[test]
    fn a_polygon_without_area_gives_no_triangle_and_a_bad_one_ends() {
        assert!(triangulate(&[]).is_empty());
        assert!(triangulate(&[P::new(0.0, 0.0), P::new(1.0, 1.0)]).is_empty());
        let flat = vec![P::new(0.0, 0.0), P::new(1.0, 0.0), P::new(2.0, 0.0)];
        assert!(triangulate(&flat).is_empty());
        // a bow tie crosses itself: the answer is not exact but the call returns
        let bow = vec![
            P::new(0.0, 0.0),
            P::new(2.0, 2.0),
            P::new(2.0, 0.0),
            P::new(0.0, 2.0),
            P::new(1.0, 3.0),
        ];
        assert!(!triangulate(&bow).is_empty());
    }

    #[test]
    fn a_text_has_a_box_from_its_length() {
        let t = Prim {
            shape: Shape::Text {
                at: P::new(1.0, 1.0),
                height: 2.0,
                text: "R12".to_string(),
            },
            width: 0.0,
            filled: false,
            layer: 0,
            net: None,
            item: 0,
        };
        let b = t.bounds();
        assert!((b.max.x - (1.0 + 0.6 * 2.0 * 3.0)).abs() < 1e-12);
        assert_eq!(t.distance(P::new(2.0, 2.0)), 0.0_f64);
        assert!((t.distance(P::new(1.0, 5.0)) - 2.0).abs() < 1e-12);
    }
}
