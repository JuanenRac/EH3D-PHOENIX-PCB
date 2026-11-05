// SPDX-License-Identifier: GPL-3.0-or-later
//! Integer geometry of the assembly: the rotation of a point, the overlap of two quadrilaterals
//! on the board plane and the position of a point against an outline. Coordinates are nanometres
//! and every product is worked out in 128 bits. A rotation by a multiple of 90 degrees is exact;
//! any other angle goes through the sine and the cosine and is rounded to the nanometre, so that
//! everything after it is integer.

/// A point of the board plane as a pair of nanometres.
pub type P = (i64, i64);

/// The floor of the square root of a number (zero for a number that is not positive).
pub fn isqrt(n: i128) -> i128 {
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

/// A point turned counter-clockwise around the origin by an angle in micro-degrees.
pub fn rotate(p: P, micro_degrees: i64) -> P {
    let a = micro_degrees.rem_euclid(360_000_000);
    match a {
        0 => p,
        90_000_000 => (-p.1, p.0),
        180_000_000 => (-p.0, -p.1),
        270_000_000 => (p.1, -p.0),
        _ => {
            let radians = (a as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
            let (s, c) = radians.sin_cos();
            let (x, y) = (p.0 as f64, p.1 as f64);
            (
                (x * c - y * s).round() as i64,
                (x * s + y * c).round() as i64,
            )
        }
    }
}

/// The four corners of a rectangle given by its least and greatest corner, counter-clockwise.
pub fn rect_corners(min: P, max: P) -> [P; 4] {
    [
        (min.0, min.1),
        (max.0, min.1),
        (max.0, max.1),
        (min.0, max.1),
    ]
}

/// Twice the signed area of a polygon (positive when it runs counter-clockwise).
pub fn twice_area(poly: &[P]) -> i128 {
    let mut sum: i128 = 0;
    for (i, a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        sum += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    sum
}

/// The polygon running counter-clockwise (reversed when it runs the other way).
pub fn counter_clockwise(poly: &[P]) -> Vec<P> {
    let mut out = poly.to_vec();
    if twice_area(&out) < 0 {
        out.reverse();
    }
    out
}

/// Whether two convex quadrilaterals overlap by more than `min_depth_nm` on every axis of
/// separation (the edges of both): they overlap when no edge separates them. Two that only touch
/// do not overlap. A quadrilateral with no area overlaps nothing.
pub fn quads_overlap(a: &[P; 4], b: &[P; 4], min_depth_nm: i64) -> bool {
    if twice_area(a) == 0 || twice_area(b) == 0 {
        return false;
    }
    for quad in [a, b] {
        for i in 0..4 {
            let (p, q) = (quad[i], quad[(i + 1) % 4]);
            let normal = (i128::from(q.1 - p.1), -i128::from(q.0 - p.0));
            let length = isqrt(normal.0 * normal.0 + normal.1 * normal.1);
            if length == 0 {
                continue;
            }
            let project = |pt: &P| i128::from(pt.0) * normal.0 + i128::from(pt.1) * normal.1;
            let (a_min, a_max) = extent(a.iter().map(project));
            let (b_min, b_max) = extent(b.iter().map(project));
            let overlap = a_max.min(b_max) - a_min.max(b_min);
            if overlap <= i128::from(min_depth_nm.max(0)) * length {
                return false;
            }
        }
    }
    true
}

fn extent(values: impl Iterator<Item = i128>) -> (i128, i128) {
    let mut lo = i128::MAX;
    let mut hi = i128::MIN;
    for v in values {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

/// Whether a point is inside a polygon or on its edge.
pub fn point_in_polygon(p: P, poly: &[P]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let cross = i128::from(b.0 - a.0) * i128::from(p.1 - a.1)
            - i128::from(b.1 - a.1) * i128::from(p.0 - a.0);
        let within_x = p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0);
        let within_y = p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1);
        if cross == 0 && within_x && within_y {
            return true;
        }
        if (a.1 > p.1) != (b.1 > p.1) {
            // the crossing of the edge with the horizontal line of the point, compared exactly
            let lhs = i128::from(p.0 - a.0) * i128::from(b.1 - a.1);
            let rhs = i128::from(b.0 - a.0) * i128::from(p.1 - a.1);
            let to_the_right = if b.1 > a.1 { lhs < rhs } else { lhs > rhs };
            if to_the_right {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    #[test]
    fn the_square_root_is_the_floor() {
        for (n, root) in [(0, 0), (1, 1), (3, 1), (4, 2), (99, 9), (100, 10), (-5, 0)] {
            assert_eq!(isqrt(n), root, "{n}");
        }
        let big: i128 = 1 << 60;
        assert_eq!(isqrt(big * big), big);
    }

    #[test]
    fn right_angles_are_exact_and_other_angles_are_rounded() {
        assert_eq!(rotate((3 * MM, MM), 0), (3 * MM, MM));
        assert_eq!(rotate((3 * MM, MM), 90_000_000), (-MM, 3 * MM));
        assert_eq!(rotate((3 * MM, MM), 180_000_000), (-3 * MM, -MM));
        assert_eq!(rotate((3 * MM, MM), 270_000_000), (MM, -3 * MM));
        assert_eq!(rotate((3 * MM, MM), -90_000_000), (MM, -3 * MM));
        assert_eq!(rotate((3 * MM, MM), 360_000_000), (3 * MM, MM));
        let (x, y) = rotate((MM, 0), 45_000_000);
        assert_eq!((x, y), (707_107, 707_107));
        let (x, y) = rotate((MM, 0), 30_000_000);
        assert_eq!((x, y), (866_025, 500_000));
    }

    #[test]
    fn the_area_tells_the_direction_and_the_polygon_is_turned_the_right_way() {
        let ccw = [(0, 0), (4 * MM, 0), (4 * MM, 3 * MM), (0, 3 * MM)];
        assert_eq!(twice_area(&ccw), 24 * i128::from(MM) * i128::from(MM));
        let mut cw = ccw;
        cw.reverse();
        assert!(twice_area(&cw) < 0);
        assert_eq!(counter_clockwise(&cw), counter_clockwise(&ccw));
        assert_eq!(counter_clockwise(&ccw), ccw.to_vec());
    }

    #[test]
    fn boxes_that_cross_overlap_and_boxes_that_touch_do_not() {
        let a = rect_corners((0, 0), (4 * MM, 4 * MM));
        let across = rect_corners((3 * MM, 3 * MM), (6 * MM, 6 * MM));
        let touching = rect_corners((4 * MM, 0), (6 * MM, 4 * MM));
        let apart = rect_corners((5 * MM, 0), (6 * MM, 4 * MM));
        let inside = rect_corners((MM, MM), (2 * MM, 2 * MM));
        assert!(quads_overlap(&a, &across, 0));
        assert!(!quads_overlap(&a, &touching, 0));
        assert!(!quads_overlap(&a, &apart, 0));
        assert!(quads_overlap(&a, &inside, 0));
        assert!(quads_overlap(&across, &a, 0));
    }

    #[test]
    fn the_overlap_must_be_deeper_than_the_tolerance() {
        let a = rect_corners((0, 0), (4 * MM, 4 * MM));
        let shallow = rect_corners((4 * MM - 1_000, 0), (6 * MM, 4 * MM));
        assert!(quads_overlap(&a, &shallow, 0));
        assert!(quads_overlap(&a, &shallow, 999));
        assert!(!quads_overlap(&a, &shallow, 1_000));
        assert!(!quads_overlap(&a, &shallow, 5_000));
    }

    #[test]
    fn a_turned_box_is_judged_by_its_corners() {
        let a = rect_corners((0, 0), (4 * MM, 4 * MM));
        let mut diamond = rect_corners((-MM, -MM), (MM, MM));
        for p in &mut diamond {
            *p = rotate(*p, 45_000_000);
            p.0 += 5 * MM;
            p.1 += 2 * MM;
        }
        // the diamond reaches 5 - 1.414 = 3.586 mm on the x axis: it enters the box
        assert!(quads_overlap(&a, &diamond, 0));
        for p in &mut diamond {
            p.0 += MM;
        }
        assert!(!quads_overlap(&a, &diamond, 0));
        let flat = [(0, 0), (MM, 0), (2 * MM, 0), (3 * MM, 0)];
        assert!(!quads_overlap(&a, &flat, 0));
    }

    #[test]
    fn a_point_is_inside_on_the_edge_or_outside() {
        let l_shape = [
            (0, 0),
            (4 * MM, 0),
            (4 * MM, MM),
            (MM, MM),
            (MM, 4 * MM),
            (0, 4 * MM),
        ];
        assert!(point_in_polygon((MM / 2, MM / 2), &l_shape));
        assert!(point_in_polygon((0, 0), &l_shape));
        assert!(point_in_polygon((2 * MM, 0), &l_shape));
        assert!(point_in_polygon((MM / 2, 3 * MM), &l_shape));
        assert!(!point_in_polygon((2 * MM, 2 * MM), &l_shape));
        assert!(!point_in_polygon((-1, 0), &l_shape));
        assert!(!point_in_polygon((5 * MM, MM / 2), &l_shape));
        assert!(!point_in_polygon((0, 0), &l_shape[..2]));
    }
}
