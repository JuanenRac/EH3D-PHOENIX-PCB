// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact integer geometry for the fill of zones.
//!
//! Coordinates are nanometres in 64-bit integers and every product is taken in 128 bits, so the
//! orientation of three points, the distance between segments compared with a limit and the
//! decision whether two pieces share an edge are exact. The only inexact step is the point where
//! a line cuts an edge: it is rounded to the nearest nanometre, the same way on both sides of the
//! cut, and the checks allow the tolerance of the settings for it.
//!
//! A polygon here is a list of points in counter-clockwise order; the last point joins the first.
//! The clipping works on convex polygons only: a zone is cut into convex pieces first, copper to
//! keep away from is a convex polygon too, and taking one convex polygon out of another gives
//! convex pieces again.

/// A point as a pair of nanometres.
pub(crate) type P = (i64, i64);

/// A box: smallest x, smallest y, largest x, largest y.
pub(crate) type Bx = (i64, i64, i64, i64);

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

fn big(v: i64) -> i128 {
    i128::from(v)
}

/// The sign and size of the turn `o -> a -> b`: positive for a left turn.
pub(crate) fn cross(o: P, a: P, b: P) -> i128 {
    (big(a.0) - big(o.0)) * (big(b.1) - big(o.1)) - (big(a.1) - big(o.1)) * (big(b.0) - big(o.0))
}

/// Twice the signed area: positive when the points run counter-clockwise.
pub(crate) fn area2(poly: &[P]) -> i128 {
    let n = poly.len();
    let mut sum: i128 = 0;
    for (i, &p) in poly.iter().enumerate() {
        let q = poly[(i + 1) % n];
        sum += big(p.0) * big(q.1) - big(q.0) * big(p.1);
    }
    sum
}

/// Turns a polygon counter-clockwise.
pub(crate) fn make_ccw(poly: &mut [P]) {
    if area2(poly) < 0 {
        poly.reverse();
    }
}

/// Removes repeated points, also between the last point and the first.
pub(crate) fn dedupe(poly: &mut Vec<P>) {
    poly.dedup();
    while poly.len() > 1 && poly.first() == poly.last() {
        poly.pop();
    }
}

/// Removes the points that lie exactly on the line between their neighbours.
pub(crate) fn drop_collinear(poly: &[P]) -> Vec<P> {
    let n = poly.len();
    if n < 4 {
        return poly.to_vec();
    }
    let mut out = Vec::with_capacity(n);
    for (i, &p) in poly.iter().enumerate() {
        let prev = poly[(i + n - 1) % n];
        let next = poly[(i + 1) % n];
        if cross(prev, p, next) != 0 {
            out.push(p);
        }
    }
    if out.len() < 3 {
        poly.to_vec()
    } else {
        out
    }
}

/// True when the polygon has an area and at least three points.
pub(crate) fn usable(poly: &[P]) -> bool {
    poly.len() >= 3 && area2(poly) != 0
}

/// The box of a polygon; a box of nothing when it has no points.
pub(crate) fn bbox(poly: &[P]) -> Bx {
    let mut b: Bx = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for &(x, y) in poly {
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
    b
}

/// True when the boxes share at least a point.
pub(crate) fn bbox_overlap(a: Bx, b: Bx) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

/// The box grown by `by` on every side.
pub(crate) fn bbox_grow(a: Bx, by: i64) -> Bx {
    (
        a.0.saturating_sub(by),
        a.1.saturating_sub(by),
        a.2.saturating_add(by),
        a.3.saturating_add(by),
    )
}

/// The convex hull in counter-clockwise order, without points on its sides.
pub(crate) fn convex_hull(points: &[P]) -> Vec<P> {
    let mut pts = points.to_vec();
    pts.sort_unstable();
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let mut lower: Vec<P> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<P> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The corners of the circumscribed polygon of a circle with 8 sides, scaled by one million:
/// the sides are tangent to the circle, so the polygon holds the whole circle.
const CORNERS_8: [(i64, i64); 8] = [
    (1_000_000, 414_214),
    (414_214, 1_000_000),
    (-414_214, 1_000_000),
    (-1_000_000, 414_214),
    (-1_000_001, -414_214),
    (-414_214, -1_000_000),
    (414_214, -1_000_000),
    (1_000_000, -414_214),
];

/// The same with 16 sides.
const CORNERS_16: [(i64, i64); 16] = [
    (1_000_000, 198_913),
    (847_760, 566_455),
    (566_455, 847_760),
    (198_913, 1_000_000),
    (-198_913, 1_000_000),
    (-566_455, 847_760),
    (-847_760, 566_455),
    (-1_000_000, 198_913),
    (-1_000_000, -198_913),
    (-847_760, -566_455),
    (-566_455, -847_760),
    (-198_913, -1_000_000),
    (198_913, -1_000_000),
    (566_455, -847_760),
    (847_760, -566_455),
    (1_000_000, -198_913),
];

/// The product rounded away from zero to a whole number of units of one million.
fn scaled_up(r: i128, unit: i64) -> i64 {
    let v = r * big(unit);
    let q = if v >= 0 {
        (v + 999_999) / 1_000_000
    } else {
        -((-v + 999_999) / 1_000_000)
    };
    i64::try_from(q).unwrap_or(i64::MAX)
}

/// The corners of a polygon that holds the circle of radius `r` around `c`: the circumscribed
/// polygon of 8 or 16 sides, one nanometre wider than the radius. A radius of zero is the point.
pub(crate) fn disc_points(c: P, r: i64, sides: u32) -> Vec<P> {
    if r <= 0 {
        return vec![c];
    }
    let table: &[(i64, i64)] = if sides <= 8 { &CORNERS_8 } else { &CORNERS_16 };
    let r = big(r) + 1;
    table
        .iter()
        .map(|&(ux, uy)| {
            (
                c.0.saturating_add(scaled_up(r, ux)),
                c.1.saturating_add(scaled_up(r, uy)),
            )
        })
        .collect()
}

/// The convex polygon that holds every circle of radius `r` around the centres.
pub(crate) fn grown_hull(centres: &[P], r: i64, sides: u32) -> Vec<P> {
    let mut pts: Vec<P> = Vec::new();
    for &c in centres {
        pts.extend(disc_points(c, r, sides));
    }
    convex_hull(&pts)
}

/// Rounded division of `n` by `d` (not zero) to the nearest whole number, halves up.
fn round_div(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    (2 * n + d).div_euclid(2 * d)
}

/// Where the edge `p -> q` crosses the line, given the signed sides of its ends (of opposite sign).
fn cut(p: P, q: P, sp: i128, sq: i128) -> P {
    let den = sp - sq;
    let x = big(p.0) + round_div((big(q.0) - big(p.0)) * sp, den);
    let y = big(p.1) + round_div((big(q.1) - big(p.1)) * sp, den);
    (
        i64::try_from(x).unwrap_or(i64::MAX),
        i64::try_from(y).unwrap_or(i64::MAX),
    )
}

/// The part of a convex polygon on one side of the line `a -> b` (the line itself included).
pub(crate) fn clip_half(poly: &[P], a: P, b: P, keep_left: bool) -> Vec<P> {
    let n = poly.len();
    let mut out: Vec<P> = Vec::with_capacity(n + 2);
    for (i, &p) in poly.iter().enumerate() {
        let q = poly[(i + 1) % n];
        let sp = cross(a, b, p);
        let sq = cross(a, b, q);
        let inside_p = if keep_left { sp >= 0 } else { sp <= 0 };
        if inside_p {
            out.push(p);
        }
        if (sp > 0 && sq < 0) || (sp < 0 && sq > 0) {
            out.push(cut(p, q, sp, sq));
        }
    }
    dedupe(&mut out);
    out
}

fn one_side_apart(a: &[P], b: &[P]) -> bool {
    let n = a.len();
    for (i, &p) in a.iter().enumerate() {
        let q = a[(i + 1) % n];
        if b.iter().all(|&v| cross(p, q, v) <= 0) {
            return true;
        }
    }
    false
}

/// True when two convex polygons share no area (touching is not sharing).
pub(crate) fn separated(a: &[P], b: &[P]) -> bool {
    one_side_apart(a, b) || one_side_apart(b, a)
}

/// Takes a convex polygon out of a convex piece. `None` when the piece is not touched, else the
/// convex pieces that remain (none at all when the piece lies inside the polygon). The pieces
/// share only their borders.
pub(crate) fn subtract_convex(piece: &[P], obstacle: &[P]) -> Option<Vec<Vec<P>>> {
    if obstacle.len() < 3 || separated(piece, obstacle) {
        return None;
    }
    let mut rest = piece.to_vec();
    let mut out: Vec<Vec<P>> = Vec::new();
    let n = obstacle.len();
    for (i, &a) in obstacle.iter().enumerate() {
        let b = obstacle[(i + 1) % n];
        let outside = clip_half(&rest, a, b, false);
        if usable(&outside) {
            out.push(outside);
        }
        rest = clip_half(&rest, a, b, true);
        if !usable(&rest) {
            break;
        }
    }
    Some(out)
}

/// True when the point is inside the convex polygon or on its border.
pub(crate) fn inside_convex(p: P, poly: &[P]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    poly.iter()
        .enumerate()
        .all(|(i, &a)| cross(a, poly[(i + 1) % n], p) >= 0)
}

/// True when the point is inside the polygon (any polygon, the last point joins the first).
pub(crate) fn inside_polygon(p: P, polygon: &[P]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let (px, py) = (big(p.0), big(p.1));
    let mut inside = false;
    for (i, &a) in polygon.iter().enumerate() {
        let b = polygon[(i + 1) % n];
        let (ax, ay) = (big(a.0), big(a.1));
        let (bx, by) = (big(b.0), big(b.1));
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

/// The mean of the points, which is inside a convex polygon.
pub(crate) fn vertex_mean(poly: &[P]) -> P {
    if poly.is_empty() {
        return (0, 0);
    }
    let n = poly.len() as i128;
    let sx: i128 = poly.iter().map(|p| big(p.0)).sum();
    let sy: i128 = poly.iter().map(|p| big(p.1)).sum();
    (
        i64::try_from(sx.div_euclid(n)).unwrap_or(0),
        i64::try_from(sy.div_euclid(n)).unwrap_or(0),
    )
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
    let dx = big(a.0) - big(b.0);
    let dy = big(a.1) - big(b.1);
    dx * dx + dy * dy
}

/// True when the distance from the point `p` to the segment `ab` is at least `r`. A product that
/// does not fit counts as too close.
pub(crate) fn point_seg_ge(p: P, a: P, b: P, r: i128) -> bool {
    if r <= 0 {
        return true;
    }
    let Some(r2) = r.checked_mul(r) else {
        return false;
    };
    let dx = big(b.0) - big(a.0);
    let dy = big(b.1) - big(a.1);
    let len2 = dx * dx + dy * dy;
    let px = big(p.0) - big(a.0);
    let py = big(p.1) - big(a.1);
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
    if r <= 0 {
        return true;
    }
    !segments_touch(a, b, c, d)
        && point_seg_ge(a, c, d, r)
        && point_seg_ge(b, c, d, r)
        && point_seg_ge(c, a, b, r)
        && point_seg_ge(d, a, b, r)
}

/// True when the segments `pq` and `rs` run along the same line (within `tol` nanometres) and
/// overlap over a stretch of positive length.
fn collinear_overlap(p: P, q: P, r: P, s: P, tol: i64) -> bool {
    let dx = big(q.0) - big(p.0);
    let dy = big(q.1) - big(p.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return false;
    }
    let t2 = big(tol) * big(tol);
    let Some(limit) = t2.checked_mul(len2) else {
        return false;
    };
    let cr = cross(p, q, r);
    let cs = cross(p, q, s);
    let (Some(c1), Some(c2)) = (cr.checked_mul(cr), cs.checked_mul(cs)) else {
        return false;
    };
    if c1 > limit || c2 > limit {
        return false;
    }
    let tr = (big(r.0) - big(p.0)) * dx + (big(r.1) - big(p.1)) * dy;
    let ts = (big(s.0) - big(p.0)) * dx + (big(s.1) - big(p.1)) * dy;
    let lo = tr.min(ts).max(0);
    let hi = tr.max(ts).min(len2);
    hi > lo
}

/// True when two pieces share a stretch of border of positive length.
pub(crate) fn share_edge(a: &[P], b: &[P], tol: i64) -> bool {
    let (na, nb) = (a.len(), b.len());
    for (i, &p) in a.iter().enumerate() {
        let q = a[(i + 1) % na];
        for (j, &r) in b.iter().enumerate() {
            let s = b[(j + 1) % nb];
            if collinear_overlap(p, q, r, s, tol) {
                return true;
            }
        }
    }
    false
}

/// True when the convex polygon is narrower than `width` somewhere: some pair of parallel lines
/// closer than `width` holds all of it, so no disc of that diameter fits. A polygon with no area
/// is narrower than any width.
pub(crate) fn narrower_than(hull: &[P], width: i64) -> bool {
    let n = hull.len();
    if n < 3 || area2(hull) == 0 {
        return true;
    }
    let w2 = big(width) * big(width);
    for (i, &a) in hull.iter().enumerate() {
        let b = hull[(i + 1) % n];
        let len2 = dist2(a, b);
        let reach = hull.iter().map(|&v| cross(a, b, v)).max().unwrap_or(0);
        if let (Some(lhs), Some(rhs)) = (reach.checked_mul(reach), w2.checked_mul(len2)) {
            if lhs < rhs {
                return true;
            }
        }
    }
    false
}

/// True when no side of the polygon turns right (it may have collinear points).
pub(crate) fn is_convex(poly: &[P]) -> bool {
    let n = poly.len();
    poly.iter()
        .enumerate()
        .all(|(i, &a)| cross(a, poly[(i + 1) % n], poly[(i + 2) % n]) >= 0)
}

/// True when a side turns back along the one before it.
fn folds_back(a: P, b: P, c: P) -> bool {
    let dot = (big(b.0) - big(a.0)) * (big(c.0) - big(b.0))
        + (big(b.1) - big(a.1)) * (big(c.1) - big(b.1));
    cross(a, b, c) == 0 && dot < 0
}

/// True when no two sides of the polygon touch except neighbours at their shared point.
pub(crate) fn is_simple(poly: &[P]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % n];
        if a == b {
            return false;
        }
        if folds_back(a, b, poly[(i + 2) % n]) {
            return false;
        }
        for (j, &c) in poly.iter().enumerate().skip(i + 1) {
            let d = poly[(j + 1) % n];
            let neighbours = j == i + 1 || (i == 0 && j + 1 == n);
            if !neighbours && segments_touch(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

fn in_triangle(p: P, a: P, b: P, c: P) -> bool {
    cross(a, b, p) >= 0 && cross(b, c, p) >= 0 && cross(c, a, p) >= 0
}

/// Cuts a simple counter-clockwise polygon into triangles by cutting off ears. `None` when no ear
/// can be found, which a simple polygon with distinct points never causes.
pub(crate) fn triangulate(poly: &[P]) -> Option<Vec<Vec<P>>> {
    let mut idx: Vec<usize> = (0..poly.len()).collect();
    let mut out: Vec<Vec<P>> = Vec::new();
    while idx.len() > 3 {
        let len = idx.len();
        let mut cut_at: Option<usize> = None;
        let mut flat_at: Option<usize> = None;
        for (k, &i1) in idx.iter().enumerate() {
            let i0 = idx[(k + len - 1) % len];
            let i2 = idx[(k + 1) % len];
            let (a, b, c) = (poly[i0], poly[i1], poly[i2]);
            let turn = cross(a, b, c);
            if turn == 0 {
                flat_at = Some(k);
                continue;
            }
            if turn < 0 {
                continue;
            }
            let blocked = idx
                .iter()
                .any(|&j| j != i0 && j != i1 && j != i2 && in_triangle(poly[j], a, b, c));
            if !blocked {
                cut_at = Some(k);
                break;
            }
        }
        match (cut_at, flat_at) {
            (Some(k), _) => {
                let len = idx.len();
                let i0 = idx[(k + len - 1) % len];
                let i1 = idx[k];
                let i2 = idx[(k + 1) % len];
                out.push(vec![poly[i0], poly[i1], poly[i2]]);
                idx.remove(k);
            }
            (None, Some(k)) => {
                idx.remove(k);
            }
            (None, None) => return None,
        }
    }
    if idx.len() == 3 {
        let tri = vec![poly[idx[0]], poly[idx[1]], poly[idx[2]]];
        if usable(&tri) {
            out.push(tri);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [P; 4] = [(0, 0), (10, 0), (10, 10), (0, 10)];

    #[test]
    fn the_square_root_is_exact_around_squares() {
        for n in [0_i128, 1, 2, 3, 4, 15, 16, 17, 999_999_999_999, 1 << 62] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "{n}");
        }
        assert_eq!(isqrt(-5), 0);
    }

    #[test]
    fn the_area_has_a_sign_and_the_hull_drops_inner_points() {
        assert_eq!(area2(&SQUARE), 200);
        let mut reversed = SQUARE;
        reversed.reverse();
        assert_eq!(area2(&reversed), -200);
        make_ccw(&mut reversed);
        assert_eq!(area2(&reversed), 200);
        let hull = convex_hull(&[(0, 0), (10, 0), (5, 5), (10, 10), (0, 10), (5, 0)]);
        assert_eq!(hull.len(), 4);
        assert_eq!(area2(&hull), 200);
    }

    #[test]
    fn taking_a_square_out_of_a_square_leaves_the_rest_in_four_pieces() {
        let hole = [(4, 4), (6, 4), (6, 6), (4, 6)];
        let pieces = subtract_convex(&SQUARE, &hole).unwrap_or_default();
        assert_eq!(pieces.len(), 4);
        let total: i128 = pieces.iter().map(|p| area2(p)).sum();
        assert_eq!(total, 192);
        for piece in &pieces {
            assert!(separated(piece, &hole));
        }
    }

    #[test]
    fn a_piece_that_is_not_touched_is_left_alone_and_one_inside_vanishes() {
        let far = [(20, 20), (30, 20), (30, 30), (20, 30)];
        assert!(subtract_convex(&SQUARE, &far).is_none());
        let beside = [(10, 0), (20, 0), (20, 10), (10, 10)];
        assert!(subtract_convex(&SQUARE, &beside).is_none());
        let big_one = [(-5, -5), (15, -5), (15, 15), (-5, 15)];
        let left = subtract_convex(&SQUARE, &big_one).unwrap_or_default();
        assert!(left.is_empty());
    }

    #[test]
    fn a_corner_cut_gives_a_whole_number_area() {
        // the square without the triangle under the line from (5,0) to (10,5)
        let wedge = [(5, 0), (20, 0), (20, 20), (5, 20)];
        let tilted = [(0, 5), (5, 0), (20, 0), (20, 20), (0, 20)];
        let pieces = subtract_convex(&SQUARE, &tilted).unwrap_or_default();
        let total: i128 = pieces.iter().map(|p| area2(p)).sum();
        // what remains of the square outside the pentagon is the corner triangle (0,0),(5,0),(0,5)
        assert_eq!(total, 25);
        assert!(separated(&wedge, &[(0, 0), (4, 0), (0, 4)]));
    }

    #[test]
    fn a_circle_is_held_by_its_polygon() {
        let c = (1_000, -2_000);
        for sides in [8_u32, 16] {
            let pts = disc_points(c, 1_000_000, sides);
            assert_eq!(pts.len() as u32, sides);
            for p in &pts {
                assert!(dist2(*p, c) >= 1_000_000_i128 * 1_000_000_i128);
            }
            assert_eq!(convex_hull(&pts).len() as u32, sides);
        }
        assert_eq!(disc_points(c, 0, 16), vec![c]);
        let hull = grown_hull(&[(0, 0), (10_000_000, 0)], 1_000_000, 16);
        assert!(inside_convex((5_000_000, 999_999), &hull));
        assert!(!inside_convex((5_000_000, 1_100_000), &hull));
    }

    #[test]
    fn touching_pieces_do_not_share_area_but_overlapping_ones_do() {
        let right = [(10, 0), (20, 0), (20, 10), (10, 10)];
        assert!(separated(&SQUARE, &right));
        let over = [(5, 5), (15, 5), (15, 15), (5, 15)];
        assert!(!separated(&SQUARE, &over));
    }

    #[test]
    fn pieces_that_share_a_stretch_of_border_are_told_from_those_that_touch_at_a_corner() {
        let right = [(10, 0), (20, 0), (20, 10), (10, 10)];
        let diagonal = [(10, 10), (20, 10), (20, 20), (10, 20)];
        assert!(share_edge(&SQUARE, &right, 2));
        assert!(!share_edge(&SQUARE, &diagonal, 2));
        let shifted = [(10, 1), (20, 1), (20, 11), (10, 11)];
        assert!(share_edge(&SQUARE, &shifted, 2));
    }

    #[test]
    fn a_narrow_polygon_is_found_and_a_wide_one_is_not() {
        assert!(narrower_than(&SQUARE, 11));
        assert!(!narrower_than(&SQUARE, 10));
        assert!(narrower_than(&[(0, 0), (100, 0)], 1));
        let strip = [(0, 0), (1000, 0), (1000, 4), (0, 4)];
        assert!(narrower_than(&strip, 5));
        assert!(!narrower_than(&strip, 4));
    }

    #[test]
    fn an_l_shape_is_cut_into_triangles_that_keep_its_area() {
        let l = [(0, 0), (2, 0), (2, 1), (1, 1), (1, 2), (0, 2)];
        assert!(is_simple(&l));
        assert!(!is_convex(&l));
        let tris = triangulate(&l).unwrap_or_default();
        assert_eq!(tris.len(), 4);
        let total: i128 = tris.iter().map(|t| area2(t)).sum();
        assert_eq!(total, 6);
        assert!(is_convex(&SQUARE));
    }

    #[test]
    fn a_bow_tie_is_not_simple() {
        assert!(!is_simple(&[(0, 0), (10, 10), (10, 0), (0, 10)]));
        assert!(is_simple(&SQUARE));
        assert!(!is_simple(&[(0, 0), (10, 0), (5, 0)]));
    }

    #[test]
    fn distances_between_segments_are_compared_without_rounding() {
        let (a, b) = ((0, 0), (10, 0));
        assert!(point_seg_ge((5, 3), a, b, 3));
        assert!(!point_seg_ge((5, 3), a, b, 4));
        assert!(seg_seg_ge((0, 0), (10, 0), (0, 4), (10, 4), 4));
        assert!(!seg_seg_ge((0, 0), (10, 0), (0, 4), (10, 4), 5));
        assert!(!seg_seg_ge((0, 0), (10, 10), (0, 10), (10, 0), 1));
    }

    #[test]
    fn a_point_is_inside_a_polygon_or_not() {
        assert!(inside_polygon((5, 5), &SQUARE));
        assert!(!inside_polygon((15, 5), &SQUARE));
        assert!(inside_convex((10, 5), &SQUARE));
        assert!(!inside_convex((11, 5), &SQUARE));
        assert_eq!(vertex_mean(&SQUARE), (5, 5));
    }

    #[test]
    fn collinear_points_and_repeats_are_dropped() {
        let with_extra = [(0, 0), (5, 0), (10, 0), (10, 10), (0, 10)];
        assert_eq!(drop_collinear(&with_extra).len(), 4);
        let mut repeated = vec![(0, 0), (0, 0), (10, 0), (10, 10), (0, 0)];
        dedupe(&mut repeated);
        assert_eq!(repeated, vec![(0, 0), (10, 0), (10, 10)]);
    }
}
