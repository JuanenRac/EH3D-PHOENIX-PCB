// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact integer geometry for the search: distances between segments compared with a limit without
//! any square root, and a point in a polygon. Coordinates are nanometres inside the range of the
//! model (below `2^30`), so a difference is below `2^31` and every product below is exact in 128
//! bits; a product that still does not fit counts as too close, never as clear.

use eh3d_phoenix_model::Point;

/// A point as a pair of nanometres.
pub(crate) type P = (i64, i64);

/// The pair of a point of the model.
pub(crate) fn pair(p: Point) -> P {
    (p.x_nm.get(), p.y_nm.get())
}

/// The floor of the square root of a number (zero for a number that is not positive).
pub(crate) fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as i128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// The square root of a number rounded up.
pub(crate) fn isqrt_ceil(n: i128) -> i128 {
    let r = isqrt(n);
    if r * r < n {
        r + 1
    } else {
        r
    }
}

/// The length of a segment, rounded up to the next nanometre.
pub(crate) fn seg_len(a: P, b: P) -> i64 {
    let dx = i128::from(b.0) - i128::from(a.0);
    let dy = i128::from(b.1) - i128::from(a.1);
    i64::try_from(isqrt_ceil(dx * dx + dy * dy)).unwrap_or(i64::MAX)
}

/// Half of a non-negative length, rounded up.
pub(crate) fn half_up(v: i64) -> i64 {
    v / 2 + v % 2
}

fn cross(o: P, a: P, b: P) -> i128 {
    (i128::from(a.0) - i128::from(o.0)) * (i128::from(b.1) - i128::from(o.1))
        - (i128::from(a.1) - i128::from(o.1)) * (i128::from(b.0) - i128::from(o.0))
}

/// The cross product of the vectors `ab` and `bc`: zero when the three points are on one line.
pub(crate) fn turn(a: P, b: P, c: P) -> i128 {
    cross(a, b, c)
}

fn in_box(a: P, b: P, p: P) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

/// True when the segments `ab` and `cd` share at least one point.
pub(crate) fn segments_touch(a: P, b: P, c: P, d: P) -> bool {
    let d1 = cross(a, b, c);
    let d2 = cross(a, b, d);
    let d3 = cross(c, d, a);
    let d4 = cross(c, d, b);
    if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)) {
        return true;
    }
    (d1 == 0 && in_box(a, b, c))
        || (d2 == 0 && in_box(a, b, d))
        || (d3 == 0 && in_box(c, d, a))
        || (d4 == 0 && in_box(c, d, b))
}

fn dist2(a: P, b: P) -> i128 {
    let dx = i128::from(a.0) - i128::from(b.0);
    let dy = i128::from(a.1) - i128::from(b.1);
    dx * dx + dy * dy
}

/// True when the distance from the point `p` to the segment `ab` is at least `r`.
pub(crate) fn point_seg_ge(p: P, a: P, b: P, r: i128) -> bool {
    let Some(r2) = r.checked_mul(r) else {
        return false;
    };
    let dx = i128::from(b.0) - i128::from(a.0);
    let dy = i128::from(b.1) - i128::from(a.1);
    let len2 = dx * dx + dy * dy;
    let px = i128::from(p.0) - i128::from(a.0);
    let py = i128::from(p.1) - i128::from(a.1);
    let t = px * dx + py * dy;
    if len2 == 0 || t <= 0 {
        return dist2(p, a) >= r2;
    }
    if t >= len2 {
        return dist2(p, b) >= r2;
    }
    let c = dx * py - dy * px;
    match (c.checked_mul(c), r2.checked_mul(len2)) {
        (Some(lhs), Some(rhs)) => lhs >= rhs,
        _ => false,
    }
}

/// True when the distance between the segments `ab` and `cd` is at least `r`.
pub(crate) fn seg_seg_ge(a: P, b: P, c: P, d: P, r: i128) -> bool {
    !segments_touch(a, b, c, d)
        && point_seg_ge(a, c, d, r)
        && point_seg_ge(b, c, d, r)
        && point_seg_ge(c, a, b, r)
        && point_seg_ge(d, a, b, r)
}

/// True when the point is inside the polygon (the last point joins the first).
pub(crate) fn inside_polygon(p: P, polygon: &[P]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let (px, py) = (i128::from(p.0), i128::from(p.1));
    let mut inside = false;
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        let (ax, ay) = (i128::from(a.0), i128::from(a.1));
        let (bx, by) = (i128::from(b.0), i128::from(b.1));
        if (ay > py) != (by > py) {
            let lhs = (px - ax) * (by - ay);
            let rhs = (bx - ax) * (py - ay);
            let right_of_point = if by > ay { lhs < rhs } else { lhs > rhs };
            if right_of_point {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_square_root_is_exact_around_squares() {
        for n in [0_i128, 1, 2, 3, 4, 15, 16, 17, 999_999_999_999, 1 << 62] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "{n}");
        }
        assert_eq!(isqrt(-5), 0);
        assert_eq!(isqrt_ceil(17), 5);
        assert_eq!(isqrt_ceil(16), 4);
        assert_eq!(seg_len((0, 0), (3, 4)), 5);
        assert_eq!(seg_len((0, 0), (1, 1)), 2);
        assert_eq!(seg_len((0, 0), (1_000_000, 1_000_000)), 1_414_214);
        assert_eq!(half_up(5), 3);
        assert_eq!(half_up(6), 3);
    }

    #[test]
    fn a_point_is_compared_with_a_segment_without_rounding() {
        let (a, b) = ((0, 0), (10, 0));
        assert!(point_seg_ge((5, 3), a, b, 3));
        assert!(!point_seg_ge((5, 3), a, b, 4));
        assert!(point_seg_ge((13, 4), a, b, 5));
        assert!(!point_seg_ge((13, 4), a, b, 6));
        assert!(point_seg_ge((3, 4), (0, 0), (0, 0), 5));
        assert!(!point_seg_ge((3, 4), (0, 0), (0, 0), 6));
    }

    #[test]
    fn segments_that_cross_or_touch_are_at_distance_zero() {
        assert!(segments_touch((0, 0), (10, 10), (0, 10), (10, 0)));
        assert!(segments_touch((0, 0), (10, 0), (10, 0), (20, 5)));
        assert!(!segments_touch((0, 0), (10, 0), (0, 1), (10, 1)));
        assert!(!seg_seg_ge((0, 0), (10, 10), (0, 10), (10, 0), 1));
        assert!(seg_seg_ge((0, 0), (10, 0), (0, 4), (10, 4), 4));
        assert!(!seg_seg_ge((0, 0), (10, 0), (0, 4), (10, 4), 5));
    }

    #[test]
    fn a_polygon_has_an_inside_and_a_turn_tells_a_line_from_a_corner() {
        let square = [(0, 0), (10, 0), (10, 10), (0, 10)];
        assert!(inside_polygon((5, 5), &square));
        assert!(!inside_polygon((15, 5), &square));
        assert!(!inside_polygon((5, 5), &square[..2]));
        assert_eq!(turn((0, 0), (1, 1), (2, 2)), 0);
        assert_ne!(turn((0, 0), (1, 1), (2, 1)), 0);
    }
}
