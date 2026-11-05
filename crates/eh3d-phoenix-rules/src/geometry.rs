// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact distances between pieces of copper, in integer nanometres.
//!
//! A squared distance is kept as a fraction ([`Dist2`]) so that it can be compared with a limit
//! without a square root or a rounding. Every product is checked: when the numbers do not fit in
//! 128 bits the answer is `None`, which the checks report as inconclusive and never as a result.
//! Floating point is used only to show a measured value.

use eh3d_phoenix_board::{Board, PadShape};
use eh3d_phoenix_model::{segments_intersect, Point};

/// A squared distance `num / den`, with `den > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dist2 {
    num: i128,
    den: i128,
}

impl Dist2 {
    fn zero() -> Dist2 {
        Dist2 { num: 0, den: 1 }
    }

    /// The squared distance between two points.
    pub(crate) fn of_points(a: Point, b: Point) -> Dist2 {
        Dist2 {
            num: a.distance_squared(b),
            den: 1,
        }
    }

    /// Is `4 d^2 < t^2`, that is `2 d < t`? `t` is a distance already doubled.
    pub(crate) fn doubled_below(self, t: i128) -> Option<bool> {
        let lhs = self.num.checked_mul(4)?;
        let rhs = t.checked_mul(t)?.checked_mul(self.den)?;
        Some(lhs < rhs)
    }

    /// Is `2 d <= t`?
    pub(crate) fn doubled_at_most(self, t: i128) -> Option<bool> {
        let lhs = self.num.checked_mul(4)?;
        let rhs = t.checked_mul(t)?.checked_mul(self.den)?;
        Some(lhs <= rhs)
    }

    fn less(self, other: Dist2) -> Option<bool> {
        Some(self.num.checked_mul(other.den)? < other.num.checked_mul(self.den)?)
    }

    /// The distance in nanometres, for showing only.
    pub(crate) fn nm_f64(self) -> f64 {
        (self.num as f64 / self.den as f64).sqrt()
    }
}

fn xy(p: Point) -> (i128, i128) {
    (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()))
}

/// The squared distance from a point to a segment.
pub(crate) fn point_segment(p: Point, a: Point, b: Point) -> Option<Dist2> {
    let (px, py) = xy(p);
    let (ax, ay) = xy(a);
    let (bx, by) = xy(b);
    let (abx, aby) = (bx - ax, by - ay);
    let len2 = abx * abx + aby * aby;
    let t = (px - ax) * abx + (py - ay) * aby;
    if len2 == 0 || t <= 0 {
        return Some(Dist2::of_points(p, a));
    }
    if t >= len2 {
        return Some(Dist2::of_points(p, b));
    }
    let cross = (px - ax) * aby - (py - ay) * abx;
    Some(Dist2 {
        num: cross.checked_mul(cross)?,
        den: len2,
    })
}

/// The squared distance between two segments (zero when they cross or touch).
pub(crate) fn segment_segment(a: Point, b: Point, c: Point, d: Point) -> Option<Dist2> {
    if segments_intersect(a, b, c, d) {
        return Some(Dist2::zero());
    }
    let mut best = point_segment(a, c, d)?;
    for candidate in [
        point_segment(b, c, d)?,
        point_segment(c, a, b)?,
        point_segment(d, a, b)?,
    ] {
        if candidate.less(best)? {
            best = candidate;
        }
    }
    Some(best)
}

/// The edges of a closed polygon, the last one back to the first point.
fn edges(polygon: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    (0..polygon.len()).map(move |i| (polygon[i], polygon[(i + 1) % polygon.len()]))
}

/// True when the point is inside the polygon or on its boundary.
pub(crate) fn inside_polygon(p: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let (px, py) = xy(p);
    let mut inside = false;
    for (a, b) in edges(polygon) {
        if point_segment(p, a, b).is_some_and(|d| d.num == 0) {
            return true;
        }
        let (ax, ay) = xy(a);
        let (bx, by) = xy(b);
        if (ay > py) != (by > py) {
            let lhs = (px - ax) * (by - ay);
            let rhs = (py - ay) * (bx - ax);
            let crosses = if by > ay { lhs < rhs } else { lhs > rhs };
            if crosses {
                inside = !inside;
            }
        }
    }
    inside
}

/// True when two polygons share a point.
pub(crate) fn polygons_touch(a: &[Point], b: &[Point]) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return false;
    }
    if edges(a).any(|(p, q)| edges(b).any(|(r, s)| segments_intersect(p, q, r, s))) {
        return true;
    }
    a.iter().any(|p| inside_polygon(*p, b)) || b.iter().any(|p| inside_polygon(*p, a))
}

/// Smallest `r` with `r * r >= n`.
fn isqrt_ceil(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut r = (n as f64).sqrt() as i128;
    while r * r < n {
        r += 1;
    }
    while r > 0 && (r - 1) * (r - 1) >= n {
        r -= 1;
    }
    r
}

/// The diameter of the largest circle inside a pad and of the smallest circle around it.
pub(crate) fn pad_diameters(shape: PadShape) -> (i128, i128) {
    match shape {
        PadShape::Circle { diameter } => (i128::from(diameter.get()), i128::from(diameter.get())),
        PadShape::Rect { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            (w.min(h), isqrt_ceil(w * w + h * h))
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            (w.min(h), w.max(h))
        }
    }
}

/// The smaller side of a pad (its diameter when it is round).
pub(crate) fn pad_min_side(shape: PadShape) -> i128 {
    pad_diameters(shape).0
}

/// A piece of copper as straight pieces with a width. `lo` is the width that is certainly copper
/// (twice the radius of the largest circle inside) and `hi` the width that certainly holds all of
/// it (twice the radius of the smallest circle around it); they are equal for a track and a
/// round pad, and differ for a pad that is not round.
#[derive(Debug, Clone)]
pub(crate) struct Prim {
    pieces: Vec<(Point, Point)>,
    pub(crate) lo: i128,
    pub(crate) hi: i128,
    pub(crate) has_arc: bool,
    bbox: [i128; 4],
}

impl Prim {
    /// `None` when there is nothing to measure.
    pub(crate) fn new(
        pieces: Vec<(Point, Point)>,
        lo: i128,
        hi: i128,
        has_arc: bool,
    ) -> Option<Prim> {
        if pieces.is_empty() {
            return None;
        }
        let mut bbox = [i128::MAX, i128::MAX, i128::MIN, i128::MIN];
        for (a, b) in &pieces {
            for p in [*a, *b] {
                let (x, y) = xy(p);
                bbox[0] = bbox[0].min(x);
                bbox[1] = bbox[1].min(y);
                bbox[2] = bbox[2].max(x);
                bbox[3] = bbox[3].max(y);
            }
        }
        Some(Prim {
            pieces,
            lo,
            hi,
            has_arc,
            bbox,
        })
    }

    /// A round or square-ish item at one point.
    pub(crate) fn at_point(p: Point, lo: i128, hi: i128) -> Option<Prim> {
        Prim::new(vec![(p, p)], lo, hi, false)
    }

    /// The first point of the first piece, to say where the item is.
    pub(crate) fn first_point(&self) -> Point {
        self.pieces[0].0
    }

    /// Could the two items be within `margin` of each other? A cheap test that never says no
    /// when the answer is yes.
    pub(crate) fn may_be_within(&self, other: &Prim, margin: i128) -> bool {
        let ea = self.hi / 2 + 1 + margin;
        let eb = other.hi / 2 + 1;
        self.bbox[0] - ea <= other.bbox[2] + eb
            && other.bbox[0] - eb <= self.bbox[2] + ea
            && self.bbox[1] - ea <= other.bbox[3] + eb
            && other.bbox[1] - eb <= self.bbox[3] + ea
    }

    /// The squared distance between the centre lines of two items.
    pub(crate) fn distance(&self, other: &Prim) -> Option<Dist2> {
        let mut best: Option<Dist2> = None;
        for (a, b) in &self.pieces {
            for (c, d) in &other.pieces {
                let candidate = segment_segment(*a, *b, *c, *d)?;
                best = match best {
                    Some(current) if !candidate.less(current)? => Some(current),
                    _ => Some(candidate),
                };
            }
        }
        best
    }
}

/// What can be said about two pieces of copper that may touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Contact {
    /// They touch, even in the least favourable reading of the shapes.
    Yes,
    /// They are apart, even in the most favourable reading of the shapes.
    No,
    /// The shapes are too coarse to tell.
    Maybe,
    /// The numbers do not fit in the exact test.
    Unknown,
}

/// Do two items touch, allowing a gap of `tolerance` (a gap of exactly the tolerance counts)?
pub(crate) fn contact(a: &Prim, b: &Prim, tolerance: i128) -> Contact {
    let Some(d) = a.distance(b) else {
        return Contact::Unknown;
    };
    match d.doubled_at_most(2 * tolerance + a.lo + b.lo) {
        None => Contact::Unknown,
        Some(true) => Contact::Yes,
        Some(false) => match d.doubled_at_most(2 * tolerance + a.hi + b.hi) {
            None => Contact::Unknown,
            Some(true) => Contact::Maybe,
            Some(false) => Contact::No,
        },
    }
}

/// The pieces of the outline of the board as items of no width.
pub(crate) fn outline_edges(board: &Board) -> Vec<Prim> {
    let n = board.outline.len();
    if n < 3 {
        return Vec::new();
    }
    (0..n)
        .filter_map(|i| {
            Prim::new(
                vec![(board.outline[i], board.outline[(i + 1) % n])],
                0,
                0,
                false,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn point_to_segment_is_exact() {
        // the point is 3 above the middle of a segment of length 10
        let d = point_segment(pt(5, 3), pt(0, 0), pt(10, 0)).unwrap();
        assert_eq!(d.doubled_below(6), Some(false));
        assert_eq!(d.doubled_at_most(6), Some(true));
        assert_eq!(d.doubled_below(7), Some(true));
        // beyond the end the distance is to the end point
        let e = point_segment(pt(13, 4), pt(0, 0), pt(10, 0)).unwrap();
        assert_eq!(e.doubled_at_most(10), Some(true));
        assert_eq!(e.doubled_below(10), Some(false));
        // a segment of no length is a point
        let z = point_segment(pt(3, 4), pt(0, 0), pt(0, 0)).unwrap();
        assert_eq!(z.doubled_at_most(10), Some(true));
        assert_eq!(z.doubled_below(10), Some(false));
    }

    #[test]
    fn crossing_segments_are_at_distance_zero() {
        let d = segment_segment(pt(0, 0), pt(10, 10), pt(0, 10), pt(10, 0)).unwrap();
        assert_eq!(d.doubled_at_most(0), Some(true));
        let far = segment_segment(pt(0, 0), pt(10, 0), pt(0, 5), pt(10, 5)).unwrap();
        assert_eq!(far.doubled_below(10), Some(false));
        assert_eq!(far.doubled_below(11), Some(true));
    }

    #[test]
    fn the_polygon_test_includes_the_boundary() {
        let square = [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];
        assert!(inside_polygon(pt(5, 5), &square));
        assert!(inside_polygon(pt(0, 5), &square));
        assert!(inside_polygon(pt(10, 10), &square));
        assert!(!inside_polygon(pt(11, 5), &square));
        assert!(!inside_polygon(pt(5, -1), &square));
        assert!(!inside_polygon(pt(5, 5), &square[..2]));
    }

    #[test]
    fn polygons_touch_when_they_cross_or_contain_each_other() {
        let a = [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];
        let inner = [pt(2, 2), pt(4, 2), pt(4, 4)];
        let crossing = [pt(8, 8), pt(20, 8), pt(20, 20)];
        let apart = [pt(30, 30), pt(40, 30), pt(40, 40)];
        assert!(polygons_touch(&a, &inner));
        assert!(polygons_touch(&a, &crossing));
        assert!(!polygons_touch(&a, &apart));
    }

    #[test]
    fn contact_uses_the_two_readings_of_a_pad() {
        // a pad whose inscribed circle is 1000 wide and whose outer circle is 3000 wide
        let pad = Prim::at_point(pt(0, 0), 1_000, 3_000).unwrap();
        let round = |x| Prim::at_point(pt(x, 0), 1_000, 1_000).unwrap();
        // centres 1000 apart: the inscribed circles touch
        assert_eq!(contact(&pad, &round(1_000), 0), Contact::Yes);
        // centres 3000 apart: apart even for the outer circle (limit 2000)
        assert_eq!(contact(&pad, &round(3_000), 0), Contact::No);
        assert_eq!(contact(&pad, &round(1_800), 0), Contact::Maybe);
        // a tolerance makes a small gap count
        assert_eq!(contact(&round(0), &round(1_050), 100), Contact::Yes);
        assert_eq!(contact(&round(0), &round(1_050), 0), Contact::No);
    }

    #[test]
    fn huge_numbers_are_unknown_and_never_a_wrong_answer() {
        let big = Prim::at_point(pt(0, 0), i128::MAX / 4, i128::MAX / 4).unwrap();
        let other = Prim::at_point(pt(1_000, 0), 10, 10).unwrap();
        assert_eq!(contact(&big, &other, 0), Contact::Unknown);
    }

    #[test]
    fn pad_diameters_bound_the_shape() {
        use eh3d_phoenix_model::Nm;
        let nm = |v| Nm::new(v).unwrap();
        assert_eq!(
            pad_diameters(PadShape::Circle { diameter: nm(500) }),
            (500, 500)
        );
        assert_eq!(
            pad_diameters(PadShape::Rect {
                width: nm(300),
                height: nm(400)
            }),
            (300, 500)
        );
        assert_eq!(
            pad_diameters(PadShape::Oblong {
                width: nm(300),
                height: nm(400)
            }),
            (300, 400)
        );
    }
}
