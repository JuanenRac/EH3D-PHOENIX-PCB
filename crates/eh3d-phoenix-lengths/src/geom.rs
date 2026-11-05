// SPDX-License-Identifier: GPL-3.0-or-later
//! Integer geometry for the lengths: the length of a straight piece rounded to the nanometre with
//! an exact square root, and the distance between two segments.
//!
//! Coordinates are nanometres inside the range of the model, so every product is exact in 128
//! bits. The distance between two segments is returned as a floating-point number: the square
//! root is correctly rounded by the machine, so the same input gives the same bytes everywhere.

/// A point as a pair of nanometres.
pub(crate) type P = (i64, i64);

fn big(v: i64) -> i128 {
    i128::from(v)
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

fn dist2(a: P, b: P) -> i128 {
    let dx = big(a.0) - big(b.0);
    let dy = big(a.1) - big(b.1);
    dx * dx + dy * dy
}

/// The length of a segment to the nearest nanometre, halves up.
pub(crate) fn seg_len(a: P, b: P) -> i64 {
    let n = dist2(a, b);
    let r = isqrt(n);
    let r = if n - r * r > r { r + 1 } else { r };
    i64::try_from(r).unwrap_or(i64::MAX)
}

fn cross(o: P, a: P, b: P) -> i128 {
    (big(a.0) - big(o.0)) * (big(b.1) - big(o.1)) - (big(a.1) - big(o.1)) * (big(b.0) - big(o.0))
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

fn point_seg_dist(p: P, a: P, b: P) -> f64 {
    let dx = big(b.0) - big(a.0);
    let dy = big(b.1) - big(a.1);
    let len2 = dx * dx + dy * dy;
    let px = big(p.0) - big(a.0);
    let py = big(p.1) - big(a.1);
    let t = px * dx + py * dy;
    if len2 == 0 || t <= 0 {
        return (dist2(p, a) as f64).sqrt();
    }
    if t >= len2 {
        return (dist2(p, b) as f64).sqrt();
    }
    let c = (dx * py - dy * px).abs() as f64;
    c / (len2 as f64).sqrt()
}

/// The distance between the segments `ab` and `cd` in nanometres; zero when they touch.
pub(crate) fn seg_dist(a: P, b: P, c: P, d: P) -> f64 {
    if segments_touch(a, b, c, d) {
        return 0.0;
    }
    point_seg_dist(a, c, d)
        .min(point_seg_dist(b, c, d))
        .min(point_seg_dist(c, a, b))
        .min(point_seg_dist(d, a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_length_of_a_piece_is_rounded_to_the_nearest_nanometre() {
        assert_eq!(seg_len((0, 0), (3, 4)), 5);
        // the diagonal of a unit square is 1.414...
        assert_eq!(seg_len((0, 0), (1, 1)), 1);
        // 2.236...
        assert_eq!(seg_len((0, 0), (1, 2)), 2);
        // 7.071... and 7.28...
        assert_eq!(seg_len((0, 0), (5, 5)), 7);
        assert_eq!(seg_len((0, 0), (2, 7)), 7);
        // 3.605... goes up
        assert_eq!(seg_len((0, 0), (2, 3)), 4);
        assert_eq!(seg_len((7, 7), (7, 7)), 0);
        assert_eq!(seg_len((-3, -4), (0, 0)), 5);
    }

    #[test]
    fn the_square_root_is_exact_around_squares() {
        for n in [0_i128, 1, 2, 3, 4, 15, 16, 17, 999_999_999_999, 1 << 62] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "{n}");
        }
        assert_eq!(isqrt(-5), 0);
    }

    #[test]
    fn the_distance_between_segments_is_zero_when_they_touch() {
        assert_eq!(seg_dist((0, 0), (10, 10), (0, 10), (10, 0)), 0.0);
        assert_eq!(seg_dist((0, 0), (10, 0), (10, 0), (20, 5)), 0.0);
        assert_eq!(seg_dist((0, 0), (10, 0), (0, 4), (10, 4)), 4.0);
        // beyond the end, the distance is to the end point: 3 and 4 give 5
        assert_eq!(seg_dist((0, 0), (10, 0), (13, 4), (20, 4)), 5.0);
        // a point against a segment
        assert_eq!(seg_dist((5, 3), (5, 3), (0, 0), (10, 0)), 3.0);
    }
}
