// SPDX-License-Identifier: GPL-3.0-or-later
//! The geometry under the reader and the writer: arcs given by their end points and a sweep (the
//! way a bulge of a polyline gives them), the straight pieces that replace an arc, and the
//! predicates on polygons. Coordinates are integer nanometres; products are worked out in 128
//! bits, so the predicates are exact. An arc goes through the sine and the cosine once and is
//! rounded to the nanometre, so that everything after it is integer.

use std::f64::consts::PI;

/// A point of the drawing plane, in nanometres.
pub type P = (i64, i64);

/// Micro-degrees in a degree.
const UDEG: f64 = 1_000_000.0;

/// A piece of a path: a straight line when the sweep is 0, else the arc from `a` to `b` that
/// turns by `sweep_udeg` micro-degrees (positive is counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seg {
    /// Where the piece starts.
    pub a: P,
    /// Where the piece ends.
    pub b: P,
    /// The sweep of an arc, in micro-degrees; 0 for a line.
    pub sweep_udeg: i64,
}

impl Seg {
    /// A straight piece.
    pub fn line(a: P, b: P) -> Seg {
        Seg {
            a,
            b,
            sweep_udeg: 0,
        }
    }

    /// The same piece walked the other way.
    pub fn reversed(&self) -> Seg {
        Seg {
            a: self.b,
            b: self.a,
            sweep_udeg: -self.sweep_udeg,
        }
    }
}

/// The bulge of a polyline vertex for a sweep: the tangent of a quarter of the sweep.
pub fn sweep_to_bulge(sweep_udeg: i64) -> f64 {
    (sweep_udeg as f64 / UDEG / 4.0).to_radians().tan()
}

/// The sweep of a bulge, in micro-degrees.
pub fn bulge_to_sweep(bulge: f64) -> i64 {
    ((4.0 * bulge.atan()).to_degrees() * UDEG).round() as i64
}

/// The length of the vector between two points, as a number.
pub fn distance(a: P, b: P) -> f64 {
    ((a.0 - b.0) as f64).hypot((a.1 - b.1) as f64)
}

/// The centre and the radius of the arc of a piece; `None` for a line or a piece whose ends
/// are the same point.
pub fn arc_of(seg: &Seg) -> Option<((f64, f64), f64)> {
    if seg.sweep_udeg == 0 {
        return None;
    }
    let chord = distance(seg.a, seg.b);
    if chord == 0.0 {
        return None;
    }
    let half = (seg.sweep_udeg as f64 / UDEG).to_radians() / 2.0;
    let tan_half = half.tan();
    if tan_half == 0.0 || !tan_half.is_finite() {
        // a sweep of 180 degrees puts the centre on the chord
        let mid = (
            (seg.a.0 + seg.b.0) as f64 / 2.0,
            (seg.a.1 + seg.b.1) as f64 / 2.0,
        );
        return Some((mid, chord / 2.0));
    }
    let radius = chord / (2.0 * half.sin().abs());
    let offset = chord / 2.0 / tan_half;
    let u = (
        (seg.b.0 - seg.a.0) as f64 / chord,
        (seg.b.1 - seg.a.1) as f64 / chord,
    );
    let mid = (
        (seg.a.0 + seg.b.0) as f64 / 2.0,
        (seg.a.1 + seg.b.1) as f64 / 2.0,
    );
    Some(((mid.0 - u.1 * offset, mid.1 + u.0 * offset), radius))
}

/// The points that replace a piece, from just after its start to its end, so that no point of
/// the arc is further than `tolerance_nm` from them. A line gives its end only.
pub fn tessellate(seg: &Seg, tolerance_nm: i64) -> Vec<P> {
    let Some((centre, radius)) = arc_of(seg) else {
        return vec![seg.b];
    };
    let tolerance = tolerance_nm.max(1) as f64;
    let step = if tolerance >= radius {
        PI / 2.0
    } else {
        2.0 * (1.0 - tolerance / radius).acos()
    };
    let sweep = (seg.sweep_udeg as f64 / UDEG).to_radians();
    let count = ((sweep.abs() / step).ceil() as usize).clamp(1, 100_000);
    let start = (seg.a.1 as f64 - centre.1).atan2(seg.a.0 as f64 - centre.0);
    let mut out = Vec::with_capacity(count);
    for k in 1..count {
        let angle = start + sweep * (k as f64) / (count as f64);
        out.push((
            (centre.0 + radius * angle.cos()).round() as i64,
            (centre.1 + radius * angle.sin()).round() as i64,
        ));
    }
    out.push(seg.b);
    out
}

/// The points of a circle, `count` of them from the angle 0 counter-clockwise, with at least
/// the tolerance of `tolerance_nm` between the circle and the polygon.
pub fn circle_points(centre: P, radius: i64, tolerance_nm: i64) -> Vec<P> {
    let radius_f = radius as f64;
    let tolerance = tolerance_nm.max(1) as f64;
    let step = if tolerance >= radius_f {
        PI / 2.0
    } else {
        2.0 * (1.0 - tolerance / radius_f).acos()
    };
    let count = (((2.0 * PI) / step).ceil() as usize).clamp(8, 100_000);
    (0..count)
        .map(|k| {
            let angle = 2.0 * PI * (k as f64) / (count as f64);
            (
                centre.0 + (radius_f * angle.cos()).round() as i64,
                centre.1 + (radius_f * angle.sin()).round() as i64,
            )
        })
        .collect()
}

/// Twice the signed area of a polygon, positive when it turns counter-clockwise.
pub fn twice_area(poly: &[P]) -> i128 {
    let mut sum: i128 = 0;
    for (i, a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        sum += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    sum
}

/// Where a point is against a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loc {
    /// Strictly inside.
    Inside,
    /// On an edge or at a corner.
    On,
    /// Strictly outside.
    Outside,
}

/// Where `p` is against the polygon `poly`.
pub fn locate(p: P, poly: &[P]) -> Loc {
    let n = poly.len();
    if n < 3 {
        return Loc::Outside;
    }
    let mut inside = false;
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % n];
        let cross = i128::from(b.0 - a.0) * i128::from(p.1 - a.1)
            - i128::from(b.1 - a.1) * i128::from(p.0 - a.0);
        let within_x = p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0);
        let within_y = p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1);
        if cross == 0 && within_x && within_y {
            return Loc::On;
        }
        if (a.1 > p.1) != (b.1 > p.1) {
            let lhs = i128::from(p.0 - a.0) * i128::from(b.1 - a.1);
            let rhs = i128::from(b.0 - a.0) * i128::from(p.1 - a.1);
            let to_the_right = if b.1 > a.1 { lhs < rhs } else { lhs > rhs };
            if to_the_right {
                inside = !inside;
            }
        }
    }
    if inside {
        Loc::Inside
    } else {
        Loc::Outside
    }
}

fn orient(a: P, b: P, c: P) -> i32 {
    let cross = i128::from(b.0 - a.0) * i128::from(c.1 - a.1)
        - i128::from(b.1 - a.1) * i128::from(c.0 - a.0);
    match cross {
        0 => 0,
        x if x > 0 => 1,
        _ => -1,
    }
}

/// True when the open segments `a-b` and `c-d` cross at one point that is inside both. Touching
/// at an end, or lying along each other, is not a crossing.
pub fn properly_cross(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    o1 * o2 < 0 && o3 * o4 < 0
}

/// The box around a polygon: the least and the greatest corner.
pub fn bounds(poly: &[P]) -> (P, P) {
    let mut lo = (i64::MAX, i64::MAX);
    let mut hi = (i64::MIN, i64::MIN);
    for p in poly {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    (lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    #[test]
    fn a_quarter_circle_has_the_bulge_of_the_tangent_of_an_eighth_of_a_turn() {
        // 90 degrees: tan(22.5 degrees) = 0.41421356...
        let bulge = sweep_to_bulge(90_000_000);
        assert!((bulge - 0.414_213_562).abs() < 1e-8, "{bulge}");
        assert_eq!(bulge_to_sweep(bulge), 90_000_000);
        assert_eq!(bulge_to_sweep(1.0), 180_000_000);
        assert_eq!(bulge_to_sweep(-1.0), -180_000_000);
        assert_eq!(bulge_to_sweep(0.0), 0);
    }

    #[test]
    fn the_centre_of_a_counter_clockwise_quarter_arc_is_at_the_corner() {
        // from (10, 0) to (0, 10) counter-clockwise about the origin
        let seg = Seg {
            a: (10 * MM, 0),
            b: (0, 10 * MM),
            sweep_udeg: 90_000_000,
        };
        let (centre, radius) = arc_of(&seg).unwrap();
        assert!(centre.0.abs() < 1e-3 && centre.1.abs() < 1e-3, "{centre:?}");
        assert!((radius - 10.0 * MM as f64).abs() < 1e-3, "{radius}");
        // the same points walked clockwise turn about (10, 10)
        let seg = Seg {
            a: (10 * MM, 0),
            b: (0, 10 * MM),
            sweep_udeg: -90_000_000,
        };
        let (centre, _) = arc_of(&seg).unwrap();
        assert!((centre.0 - 10.0 * MM as f64).abs() < 1e-3, "{centre:?}");
        assert!((centre.1 - 10.0 * MM as f64).abs() < 1e-3, "{centre:?}");
    }

    #[test]
    fn a_half_turn_has_its_centre_in_the_middle_of_the_chord() {
        let seg = Seg {
            a: (0, 0),
            b: (10 * MM, 0),
            sweep_udeg: 180_000_000,
        };
        let (centre, radius) = arc_of(&seg).unwrap();
        assert!((centre.0 - 5.0 * MM as f64).abs() < 1e-3);
        assert!(centre.1.abs() < 1e-3);
        assert!((radius - 5.0 * MM as f64).abs() < 1e-3);
        assert!(arc_of(&Seg::line((0, 0), (MM, 0))).is_none());
    }

    #[test]
    fn the_pieces_of_an_arc_end_on_the_arc_and_at_its_end() {
        let seg = Seg {
            a: (10 * MM, 0),
            b: (0, 10 * MM),
            sweep_udeg: 90_000_000,
        };
        let points = tessellate(&seg, 5_000);
        assert_eq!(*points.last().unwrap(), seg.b);
        assert!(points.len() >= 8, "{}", points.len());
        for p in &points {
            let r = distance(*p, (0, 0));
            assert!((r - 10.0 * MM as f64).abs() < 1.0, "{p:?} {r}");
        }
        assert_eq!(
            tessellate(&Seg::line((0, 0), (MM, MM)), 5_000),
            vec![(MM, MM)]
        );
    }

    #[test]
    fn a_circle_is_a_polygon_inside_the_tolerance() {
        let points = circle_points((0, 0), 5 * MM, 5_000);
        assert!(points.len() >= 8);
        assert_eq!(points[0], (5 * MM, 0));
        for p in &points {
            assert!((distance(*p, (0, 0)) - 5.0 * MM as f64).abs() < 1.0);
        }
        // a tolerance as large as the radius gives the least polygon
        assert_eq!(circle_points((0, 0), MM, MM).len(), 8);
    }

    #[test]
    fn the_area_tells_the_direction_and_the_point_is_inside_on_or_outside() {
        let square = [(0, 0), (4 * MM, 0), (4 * MM, 4 * MM), (0, 4 * MM)];
        assert_eq!(twice_area(&square), 32 * i128::from(MM) * i128::from(MM));
        let mut turned = square;
        turned.reverse();
        assert!(twice_area(&turned) < 0);
        assert_eq!(locate((MM, MM), &square), Loc::Inside);
        assert_eq!(locate((4 * MM, MM), &square), Loc::On);
        assert_eq!(locate((0, 0), &square), Loc::On);
        assert_eq!(locate((5 * MM, MM), &square), Loc::Outside);
        assert_eq!(locate((MM, MM), &square[..2]), Loc::Outside);
    }

    #[test]
    fn two_segments_cross_only_when_they_meet_inside_both() {
        let p = |x: i64, y: i64| (x * MM, y * MM);
        assert!(properly_cross(p(0, 0), p(4, 4), p(0, 4), p(4, 0)));
        // a T: the end of one lies on the other
        assert!(!properly_cross(p(0, 0), p(4, 0), p(2, 0), p(2, 3)));
        // apart
        assert!(!properly_cross(p(0, 0), p(1, 0), p(2, 0), p(3, 0)));
        // along each other
        assert!(!properly_cross(p(0, 0), p(4, 0), p(1, 0), p(3, 0)));
    }

    #[test]
    fn the_box_around_a_polygon_holds_its_corners() {
        assert_eq!(bounds(&[(1, 5), (-3, 2), (4, -7)]), ((-3, -7), (4, 5)));
    }

    #[test]
    fn a_piece_walked_the_other_way_turns_the_other_way() {
        let seg = Seg {
            a: (1, 2),
            b: (3, 4),
            sweep_udeg: 45_000_000,
        };
        let back = seg.reversed();
        assert_eq!(
            (back.a, back.b, back.sweep_udeg),
            ((3, 4), (1, 2), -45_000_000)
        );
    }
}
