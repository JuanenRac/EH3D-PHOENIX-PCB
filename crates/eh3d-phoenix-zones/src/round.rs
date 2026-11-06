// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded corners, with integer arithmetic only.
//!
//! A corner of a counter-clockwise polygon is replaced by an arc of a circle of the radius asked
//! for that touches both sides (a fillet). The arc is a chain of chords whose sagitta is at most
//! the tolerance of the settings, found by halving the arc until the chord is short enough, with
//! the middle point of each half taken on the circle by an integer square root. Lengths are
//! carried with ten bits of fraction (a scale of 1024) so the products stay inside 128 bits.
//!
//! Two uses:
//!
//! * [`round_exposed`] rounds the corners of the pieces of a fill that are convex corners of the
//!   fill as a whole: a vertex that no other piece has, that lies on no side of another piece and
//!   whose corner holds no vertex of another piece. The pieces of a fill touch along their sides,
//!   so a vertex that is shared or lies on a side is inside the copper, not on its edge. Rounding
//!   only takes copper away from the piece itself, so a rounded fill never comes closer to any
//!   copper than the fill it was made from, apart from the rounding of the arc to the nanometre;
//! * [`round_all`] rounds every corner of a convex polygon, which is used for the blocks of a
//!   thermal relief: the block is the gap, so rounding it gives the spokes a fillet at their root.

use std::collections::BTreeMap;

use crate::geom::{cross, isqrt, P};

/// The scale of the lengths that carry a fraction, a power of two.
const SCALE: i128 = 1024;
/// The deepest the halving of an arc goes; it makes at most 4096 chords for a corner.
const MAX_DEPTH: u32 = 12;
/// The most cells of the index that one box may cover before the slow path is taken.
const MAX_CELLS: i64 = 4096;

/// A corner replaced by an arc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fillet {
    /// The point where the arc leaves the side that comes before the corner.
    pub first: P,
    /// The point where the arc joins the side that follows the corner.
    pub last: P,
    /// All the points of the arc in order, the first and the last included.
    pub points: Vec<P>,
}

fn diff(a: i64, b: i64) -> i128 {
    i128::from(a) - i128::from(b)
}

/// The quotient of two integers rounded to the nearest, halves away from zero; `d` is above zero.
fn div_round(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

fn clamp_nm(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

/// The points strictly between `a` and `b` on the arc of centre `c` and radius `rho`, found by
/// halving until the sagitta of the chord is at most `tolerance`.
fn arc_between(c: P, rho: i128, a: P, b: P, tolerance: i128, depth: u32, out: &mut Vec<P>) {
    let chord2 = diff(a.0, b.0) * diff(a.0, b.0) + diff(a.1, b.1) * diff(a.1, b.1);
    // sagitta = rho - sqrt(rho^2 - chord^2 / 4) <= tolerance
    //   <=>  chord^2 <= 4 (2 rho tolerance - tolerance^2)
    if depth >= MAX_DEPTH || chord2 <= 4 * (2 * rho * tolerance - tolerance * tolerance) {
        return;
    }
    let sx = diff(a.0, c.0) + diff(b.0, c.0);
    let sy = diff(a.1, c.1) + diff(b.1, c.1);
    let length = isqrt((sx * sx + sy * sy) << 20);
    if length == 0 {
        return;
    }
    let mid = (
        clamp_nm(i128::from(c.0) + div_round(rho * sx * SCALE, length)),
        clamp_nm(i128::from(c.1) + div_round(rho * sy * SCALE, length)),
    );
    arc_between(c, rho, a, mid, tolerance, depth + 1, out);
    out.push(mid);
    arc_between(c, rho, mid, b, tolerance, depth + 1, out);
}

/// The fillet of radius `radius` at the corner `v` between `prev` and `next` of a
/// counter-clockwise polygon, or `None` when the corner is not convex, the sides are degenerate
/// or the radius is too small to make an arc. The arc takes at most half of each side: a radius that
/// would need more is made smaller.
pub(crate) fn fillet(prev: P, v: P, next: P, radius: i64, tolerance: i64) -> Option<Fillet> {
    if radius < 1 {
        return None;
    }
    let (px, py) = (diff(prev.0, v.0), diff(prev.1, v.1));
    let (nx, ny) = (diff(next.0, v.0), diff(next.1, v.1));
    // positive for a convex corner of a counter-clockwise polygon
    let cr = -(px * ny - py * nx);
    if cr <= 0 {
        return None;
    }
    let dot = px * nx + py * ny;
    let lp = isqrt((px * px + py * py) << 20);
    let ln = isqrt((nx * nx + ny * ny) << 20);
    if lp == 0 || ln == 0 {
        return None;
    }
    // t = r (|p||n| + p.n) / |p x n|, the distance from the corner to the points of tangency
    let num = lp * ln + dot * SCALE * SCALE;
    if num <= 0 {
        return None;
    }
    let mut t = div_round(i128::from(radius) * num, cr * SCALE * SCALE);
    let t_max = lp.min(ln) / (2 * SCALE);
    let mut r_eff = i128::from(radius);
    if t > t_max {
        t = t_max;
        r_eff = div_round(t * cr * SCALE * SCALE, num);
    }
    if t < 1 || r_eff < 1 {
        return None;
    }
    let tangent = |x: i128, y: i128, l: i128| {
        (
            clamp_nm(i128::from(v.0) + div_round(x * t * SCALE, l)),
            clamp_nm(i128::from(v.1) + div_round(y * t * SCALE, l)),
        )
    };
    let first = tangent(px, py, lp);
    let last = tangent(nx, ny, ln);
    // the centre is on the bisector, r / sin(angle) from the corner
    let centre = (
        clamp_nm(i128::from(v.0) + div_round(r_eff * (px * ln + nx * lp), cr * SCALE)),
        clamp_nm(i128::from(v.1) + div_round(r_eff * (py * ln + ny * lp), cr * SCALE)),
    );
    let rho = isqrt(diff(first.0, centre.0).pow(2) + diff(first.1, centre.1).pow(2)).max(1);
    let mut inner: Vec<P> = Vec::new();
    arc_between(
        centre,
        rho,
        first,
        last,
        i128::from(tolerance.max(1)),
        0,
        &mut inner,
    );
    let mut points: Vec<P> = Vec::with_capacity(inner.len() + 2);
    points.push(first);
    for p in inner {
        if points.last() != Some(&p) {
            points.push(p);
        }
    }
    if points.last() != Some(&last) {
        points.push(last);
    }
    Some(Fillet {
        first,
        last,
        points,
    })
}

/// Every convex corner of a counter-clockwise polygon rounded, each by at most half of the sides
/// next to it.
pub(crate) fn round_all(poly: &[P], radius: i64, tolerance: i64) -> Vec<P> {
    let n = poly.len();
    if n < 3 || radius < 1 {
        return poly.to_vec();
    }
    let mut out: Vec<P> = Vec::with_capacity(n);
    for (i, &v) in poly.iter().enumerate() {
        match fillet(
            poly[(i + n - 1) % n],
            v,
            poly[(i + 1) % n],
            radius,
            tolerance,
        ) {
            Some(f) => out.extend(f.points),
            None => out.push(v),
        }
    }
    out
}

/// A uniform grid over points and over the boxes of sides, to find what is near a point.
struct Index {
    cell: i64,
    points: BTreeMap<(i64, i64), Vec<(P, usize)>>,
    sides: BTreeMap<(i64, i64), Vec<(P, P, usize)>>,
    long_sides: Vec<(P, P, usize)>,
    all_points: Vec<(P, usize)>,
}

fn cell_of(cell: i64, p: P) -> (i64, i64) {
    (p.0.div_euclid(cell), p.1.div_euclid(cell))
}

impl Index {
    fn new(pieces: &[Vec<P>]) -> Index {
        let mut min = (i64::MAX, i64::MAX);
        let mut max = (i64::MIN, i64::MIN);
        let mut count = 0_i64;
        for piece in pieces {
            for &(x, y) in piece {
                min = (min.0.min(x), min.1.min(y));
                max = (max.0.max(x), max.1.max(y));
                count += 1;
            }
        }
        let extent = if count == 0 {
            1
        } else {
            (max.0 - min.0).max(max.1 - min.1).max(1)
        };
        let per_side = clamp_nm(isqrt(i128::from(count.max(1)))).max(1);
        let cell = (extent / per_side).max(1);
        let mut index = Index {
            cell,
            points: BTreeMap::new(),
            sides: BTreeMap::new(),
            long_sides: Vec::new(),
            all_points: Vec::new(),
        };
        for (k, piece) in pieces.iter().enumerate() {
            let n = piece.len();
            for (i, &p) in piece.iter().enumerate() {
                index
                    .points
                    .entry(cell_of(cell, p))
                    .or_default()
                    .push((p, k));
                index.all_points.push((p, k));
                let q = piece[(i + 1) % n];
                let (a, b) = (cell_of(cell, p), cell_of(cell, q));
                let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
                let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
                if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > MAX_CELLS {
                    index.long_sides.push((p, q, k));
                } else {
                    for cx in x0..=x1 {
                        for cy in y0..=y1 {
                            index.sides.entry((cx, cy)).or_default().push((p, q, k));
                        }
                    }
                }
            }
        }
        index
    }

    /// Whether a point lies in the inside of a side of a piece other than `own` (the ends of a side
    /// do not count: a shared vertex is found by the count of vertices).
    fn on_other_side(&self, p: P, own: usize) -> bool {
        let near = self.sides.get(&cell_of(self.cell, p));
        near.into_iter()
            .flatten()
            .chain(self.long_sides.iter())
            .any(|&(a, b, k)| k != own && p != a && p != b && on_segment(p, a, b))
    }

    /// Whether a vertex of another piece than `own` lies in the closed triangle `a, b, c`
    /// (counter-clockwise).
    fn vertex_in_triangle(&self, a: P, b: P, c: P, own: usize) -> bool {
        let (x0, x1) = (a.0.min(b.0).min(c.0), a.0.max(b.0).max(c.0));
        let (y0, y1) = (a.1.min(b.1).min(c.1), a.1.max(b.1).max(c.1));
        let low = cell_of(self.cell, (x0, y0));
        let high = cell_of(self.cell, (x1, y1));
        let inside = |p: P| cross(a, b, p) >= 0 && cross(b, c, p) >= 0 && cross(c, a, p) >= 0;
        if (high.0 - low.0 + 1).saturating_mul(high.1 - low.1 + 1) > MAX_CELLS {
            return self.all_points.iter().any(|&(p, k)| k != own && inside(p));
        }
        for cx in low.0..=high.0 {
            for cy in low.1..=high.1 {
                if let Some(list) = self.points.get(&(cx, cy)) {
                    if list.iter().any(|&(p, k)| k != own && inside(p)) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn on_segment(p: P, a: P, b: P) -> bool {
    cross(a, b, p) == 0
        && p.0 >= a.0.min(b.0)
        && p.0 <= a.0.max(b.0)
        && p.1 >= a.1.min(b.1)
        && p.1 <= a.1.max(b.1)
}

/// The pieces of a fill with their exposed convex corners rounded. A piece keeps a corner as it is
/// when another piece has the same vertex, when the vertex lies on a side of another piece or when
/// a vertex of another piece lies in the corner that the arc would cut off. Each piece must be
/// counter-clockwise without collinear points.
pub(crate) fn round_exposed(pieces: &[Vec<P>], radius: i64, tolerance: i64) -> Vec<Vec<P>> {
    if radius < 1 {
        return pieces.to_vec();
    }
    let mut seen: BTreeMap<P, u32> = BTreeMap::new();
    for piece in pieces {
        for &p in piece {
            *seen.entry(p).or_insert(0) += 1;
        }
    }
    let index = Index::new(pieces);
    let mut out: Vec<Vec<P>> = Vec::with_capacity(pieces.len());
    for (k, piece) in pieces.iter().enumerate() {
        let n = piece.len();
        if n < 3 {
            out.push(piece.clone());
            continue;
        }
        let mut rounded: Vec<P> = Vec::with_capacity(n);
        for (i, &v) in piece.iter().enumerate() {
            let exposed = seen.get(&v).copied().unwrap_or(0) == 1 && !index.on_other_side(v, k);
            let cut = if exposed {
                fillet(
                    piece[(i + n - 1) % n],
                    v,
                    piece[(i + 1) % n],
                    radius,
                    tolerance,
                )
            } else {
                None
            };
            match cut {
                Some(f) if !index.vertex_in_triangle(f.first, v, f.last, k) => {
                    rounded.extend(f.points)
                }
                _ => rounded.push(v),
            }
        }
        out.push(rounded);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::area2;

    const MM: i64 = 1_000_000;

    fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<P> {
        vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    }

    /// The area taken from a right-angle corner by a circle of radius `r`: `r^2 (1 - pi / 4)`.
    fn corner_area(r: i64) -> f64 {
        let r = r as f64;
        r * r * (1.0 - std::f64::consts::FRAC_PI_4)
    }

    fn area(poly: &[P]) -> f64 {
        area2(poly) as f64 / 2.0
    }

    #[test]
    fn a_right_angle_corner_is_cut_at_the_radius_and_follows_the_circle() {
        // the corner at the origin of a counter-clockwise square: the side before it comes from
        // (0, 10 mm), the side after it goes to (10 mm, 0)
        let sq = square(0, 0, 10 * MM, 10 * MM);
        let f = fillet(sq[3], sq[0], sq[1], MM, 1000).unwrap();
        assert_eq!(f.first, (0, MM));
        assert_eq!(f.last, (MM, 0));
        // a chord of a sagitta of at most 1 micrometre on a quarter circle of 1 mm: 32 chords
        assert_eq!(f.points.len(), 33);
        // the middle of the arc is on the diagonal at r (1 - 1 / sqrt 2) = 292893.2 nm
        assert_eq!(f.points[16], (292_893, 292_893));
        // every point is on the circle of centre (r, r) to a nanometre
        for &(x, y) in &f.points {
            let d = ((x - MM) as f64).hypot((y - MM) as f64);
            assert!((d - MM as f64).abs() <= 1.5, "{x} {y} {d}");
        }
    }

    #[test]
    fn the_tolerance_sets_the_number_of_chords_and_a_larger_one_gives_fewer() {
        let sq = square(0, 0, 10 * MM, 10 * MM);
        let fine = fillet(sq[3], sq[0], sq[1], MM, 100).unwrap().points.len();
        let coarse = fillet(sq[3], sq[0], sq[1], MM, 100_000)
            .unwrap()
            .points
            .len();
        assert!(fine > 33 && coarse < 33, "{fine} {coarse}");
        assert_eq!(coarse, 3);
        // a tolerance as large as the radius leaves the straight cut across the corner
        let chamfer = fillet(sq[3], sq[0], sq[1], MM, MM).unwrap();
        assert_eq!(chamfer.points, vec![(0, MM), (MM, 0)]);
    }

    #[test]
    fn a_square_loses_four_corners_of_the_circle_and_a_little_more() {
        let sq = square(0, 0, 10 * MM, 10 * MM);
        let rounded = round_all(&sq, MM, 1000);
        let removed = area(&sq) - area(&rounded);
        let exact = 4.0 * corner_area(MM);
        // the chords lie inside the arc, so a little more copper is taken than the circle takes
        assert!(
            removed > exact && removed < exact * 1.006,
            "{removed} {exact}"
        );
        assert_eq!(rounded.len(), 4 * 33);
    }

    #[test]
    fn the_radius_is_made_smaller_when_the_sides_are_short() {
        let sq = square(0, 0, 10 * MM, 10 * MM);
        // 8 mm cannot fit in a side of 10 mm that two corners share: 5 mm is the most
        let f = fillet(sq[3], sq[0], sq[1], 8 * MM, 1000).unwrap();
        assert_eq!(f.first, (0, 5 * MM));
        assert_eq!(f.last, (5 * MM, 0));
        let small = square(0, 0, 100_000, 100_000);
        let g = fillet(small[3], small[0], small[1], MM, 1000).unwrap();
        assert_eq!(g.first, (0, 50_000));
        assert_eq!(g.last, (50_000, 0));
        assert_eq!(g.points.len(), 5);
        assert_eq!(g.points[2], (14_645, 14_645));
    }

    #[test]
    fn an_acute_and_an_obtuse_corner_are_cut_where_the_geometry_says() {
        // the triangle (0,0), (10 mm, 0), (10 mm, 4.142136 mm) has 22.5 degrees at the origin:
        // t = r cot(11.25 degrees) = 5.0273 r
        let tri = vec![(0, 0), (10 * MM, 0), (10 * MM, 4_142_136)];
        let f = fillet(tri[2], tri[0], tri[1], 500_000, 1000).unwrap();
        assert_eq!(f.last, (2_513_670, 0));
        assert_eq!(f.first, (2_322_328, 961_940));
        // the pentagon has 135 degrees at (10 mm, 0): t = r cot(67.5 degrees) = 0.4142 r
        let pent = vec![
            (0, 0),
            (10 * MM, 0),
            (15 * MM, 5 * MM),
            (10 * MM, 10 * MM),
            (0, 10 * MM),
        ];
        let g = fillet(pent[0], pent[1], pent[2], 700_000, 1000).unwrap();
        assert_eq!(g.first, (9_710_051, 0));
        assert_eq!(g.last, (10_205_025, 205_025));
    }

    #[test]
    fn a_corner_that_is_flat_or_turns_the_wrong_way_is_not_rounded() {
        // a straight line, a reflex corner of a counter-clockwise polygon, a radius of zero
        assert!(fillet((0, 0), (MM, 0), (2 * MM, 0), MM, 1000).is_none());
        let l_shape = vec![
            (0, 0),
            (10 * MM, 0),
            (10 * MM, 5 * MM),
            (5 * MM, 5 * MM),
            (5 * MM, 10 * MM),
            (0, 10 * MM),
        ];
        assert!(fillet(l_shape[2], l_shape[3], l_shape[4], MM, 1000).is_none());
        assert!(fillet(l_shape[5], l_shape[0], l_shape[1], 0, 1000).is_none());
        let rounded = round_all(&l_shape, MM, 1000);
        // five convex corners are rounded, the reflex one is kept
        assert_eq!(rounded.len(), 5 * 33 + 1);
        assert!(rounded.contains(&(5 * MM, 5 * MM)));
    }

    #[test]
    fn a_single_piece_is_rounded_at_all_its_corners_and_the_rounded_piece_stays_inside() {
        let sq = square(0, 0, 10 * MM, 10 * MM);
        let out = round_exposed(std::slice::from_ref(&sq), MM, 1000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 4 * 33);
        for &(x, y) in &out[0] {
            assert!((0..=10 * MM).contains(&x) && (0..=10 * MM).contains(&y));
        }
    }

    #[test]
    fn two_pieces_that_share_a_side_keep_their_shared_corners() {
        // 20 mm by 10 mm made of two squares; the two corners on the shared side stay sharp
        let a = square(0, 0, 10 * MM, 10 * MM);
        let b = square(10 * MM, 0, 20 * MM, 10 * MM);
        let out = round_exposed(&[a.clone(), b.clone()], MM, 1000);
        assert!(out[0].contains(&(10 * MM, 0)) && out[0].contains(&(10 * MM, 10 * MM)));
        assert!(out[1].contains(&(10 * MM, 0)) && out[1].contains(&(10 * MM, 10 * MM)));
        // the two outer corners of each piece are rounded: 2 * 33 + 2 points
        assert_eq!(out[0].len(), 2 * 33 + 2);
        assert_eq!(out[1].len(), 2 * 33 + 2);
        let removed = area(&a) + area(&b) - area(&out[0]) - area(&out[1]);
        let exact = 4.0 * corner_area(MM);
        assert!(
            removed > exact && removed < exact * 1.006,
            "{removed} {exact}"
        );
    }

    #[test]
    fn a_vertex_on_the_side_of_another_piece_and_a_corner_that_holds_one_are_left_alone() {
        // the square A has a piece B on part of its right side; B's vertex (10, 5) is inside A's side
        let a = square(0, 0, 10 * MM, 10 * MM);
        let b = square(10 * MM, 0, 20 * MM, 5 * MM);
        let out = round_exposed(&[a.clone(), b.clone()], MM, 1000);
        // A's corner (10, 10) is free and is rounded (its tangent point is 1 mm down the side)
        assert!(!out[0].contains(&(10 * MM, 10 * MM)));
        assert!(out[0].contains(&(10 * MM, 9 * MM)));
        // A's corner (10, 0) is shared with B and B's corner (10, 5) is on A's side: both stay
        assert!(out[0].contains(&(10 * MM, 0)));
        assert!(out[1].contains(&(10 * MM, 0)) && out[1].contains(&(10 * MM, 5 * MM)));
        // with a radius of 6 mm the arc of A's corner (10, 10) would reach down to B's vertex
        // (10, 5), so the corner is kept
        let big = round_exposed(&[a, b], 6 * MM, 1000);
        assert!(big[0].contains(&(10 * MM, 10 * MM)));
    }

    #[test]
    fn rounding_is_the_same_every_time_and_a_radius_of_zero_changes_nothing() {
        let a = square(0, 0, 10 * MM, 10 * MM);
        let b = square(10 * MM, 0, 20 * MM, 10 * MM);
        let pieces = [a, b];
        assert_eq!(
            round_exposed(&pieces, MM, 1000),
            round_exposed(&pieces, MM, 1000)
        );
        assert_eq!(round_exposed(&pieces, 0, 1000), pieces.to_vec());
        assert_eq!(round_all(&pieces[0], 0, 1000), pieces[0]);
    }

    #[test]
    fn the_rounding_of_a_division_goes_to_the_nearest_with_halves_away_from_zero() {
        assert_eq!(div_round(7, 2), 4);
        assert_eq!(div_round(-7, 2), -4);
        assert_eq!(div_round(5, 3), 2);
        assert_eq!(div_round(-5, 3), -2);
        assert_eq!(div_round(0, 5), 0);
    }
}
