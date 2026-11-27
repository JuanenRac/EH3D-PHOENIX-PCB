// SPDX-License-Identifier: GPL-3.0-or-later
//! The octagon around a piece of copper (horizontal, vertical and 45-degree edges only) and the
//! walk of a line around it, for the push that moves tracks out of the way and around obstacles.
//!
//! An octagon is held as the smallest and the largest value of `x`, `y`, `x + y` and `x - y`, so
//! every corner is a whole number of nanometres and the octagon around two others is found by
//! comparing numbers. A line walked around it keeps to horizontal, vertical and 45-degree pieces.

#![allow(clippy::needless_range_loop)] // the four axes are used by number

use std::f64::consts::SQRT_2;

/// A point in whole nanometres.
pub(crate) type P = (i64, i64);

/// A part of a line shorter than this inside an octagon does not count as going into it (it is the
/// rounding of a point on the edge).
const TOLERANCE_NM: f64 = 2.5;

/// The values of a point along the four axes of an octagon.
fn axes(p: P) -> [i64; 4] {
    [p.0, p.1, p.0 + p.1, p.0 - p.1]
}

/// The distance along a diagonal axis that is `radius` away from a point.
fn diagonal(radius: i64) -> i64 {
    (radius as f64 * SQRT_2).ceil() as i64
}

/// An octagon with edges at 0, 45 and 90 degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Oct {
    lo: [i64; 4],
    hi: [i64; 4],
}

impl Oct {
    /// The smallest octagon that holds the points.
    pub fn around(points: &[P]) -> Oct {
        let mut out = Oct {
            lo: [i64::MAX; 4],
            hi: [i64::MIN; 4],
        };
        for p in points {
            let v = axes(*p);
            for (k, value) in v.iter().enumerate() {
                out.lo[k] = out.lo[k].min(*value);
                out.hi[k] = out.hi[k].max(*value);
            }
        }
        out
    }

    /// The octagon of every point at most `radius` away.
    pub fn grown(&self, radius: i64) -> Oct {
        let mut out = *self;
        for k in 0..4 {
            let by = if k < 2 { radius } else { diagonal(radius) };
            out.lo[k] = out.lo[k].saturating_sub(by);
            out.hi[k] = out.hi[k].saturating_add(by);
        }
        out
    }

    /// The box along x and y: `[min x, min y, max x, max y]`.
    pub fn bx(&self) -> [i128; 4] {
        [self.lo[0], self.lo[1], self.hi[0], self.hi[1]].map(i128::from)
    }

    /// The smallest octagon that holds both.
    pub fn union(&self, other: &Oct) -> Oct {
        let mut out = *self;
        for k in 0..4 {
            out.lo[k] = out.lo[k].min(other.lo[k]);
            out.hi[k] = out.hi[k].max(other.hi[k]);
        }
        out
    }

    /// Whether the boxes of the two (their sides along x and y) are at most `margin` apart.
    pub fn near(&self, other: &Oct, margin: i64) -> bool {
        (0..2).all(|k| {
            self.lo[k] <= other.hi[k].saturating_add(margin)
                && other.lo[k] <= self.hi[k].saturating_add(margin)
        })
    }

    /// Whether the point is inside, away from the edge.
    pub fn holds_inside(&self, p: P) -> bool {
        let v = axes(p);
        (0..4).all(|k| v[k] > self.lo[k] + 2 && v[k] < self.hi[k] - 2)
    }

    /// The part of the piece from `a` to `b` that is inside, as the parameters of the way in and
    /// out (0 at `a`, 1 at `b`); `None` when it only touches the edge or stays outside.
    pub fn span(&self, a: P, b: P) -> Option<(f64, f64)> {
        let (va, vb) = (axes(a), axes(b));
        let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
        for k in 0..4 {
            // inside is `value < 0` on both sides of the axis
            for (from, to) in [
                (va[k] - self.hi[k], vb[k] - self.hi[k]),
                (self.lo[k] - va[k], self.lo[k] - vb[k]),
            ] {
                if from >= 0 && to >= 0 {
                    return None;
                }
                if from >= 0 {
                    t0 = t0.max(from as f64 / (from - to) as f64);
                } else if to >= 0 {
                    t1 = t1.min(from as f64 / (from - to) as f64);
                }
            }
        }
        let length = (b.0 - a.0).abs().max((b.1 - a.1).abs()) as f64;
        ((t1 - t0) * length > TOLERANCE_NM).then_some((t0, t1))
    }

    /// The corners, counter-clockwise from the lower end of the right side; some may coincide.
    fn corners(&self) -> [P; 8] {
        let [x0, y0, s0, d0] = self.lo;
        let [x1, y1, s1, d1] = self.hi;
        [
            (x1, s1 - x1),
            (s1 - y1, y1),
            (d0 + y1, y1),
            (x0, x0 - d0),
            (x0, s0 - x0),
            (s0 - y0, y0),
            (d1 + y0, y0),
            (x1, x1 - d1),
        ]
    }

    /// The side that a point of the edge is on and how far along it (counter-clockwise).
    fn side_of(&self, p: P) -> (usize, i64) {
        let v = axes(p);
        // right, upper right, top, upper left, left, lower left, bottom, lower right
        let gaps = [
            v[0] - self.hi[0],
            v[2] - self.hi[2],
            v[1] - self.hi[1],
            v[3] - self.lo[3],
            v[0] - self.lo[0],
            v[2] - self.lo[2],
            v[1] - self.lo[1],
            v[3] - self.hi[3],
        ];
        let side = (0..8).min_by_key(|k| gaps[*k].abs()).unwrap_or(0);
        let corners = self.corners();
        let from = corners[(side + 7) % 8];
        let to = corners[side];
        let (dx, dy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
        (side, (p.0 - from.0) * dx + (p.1 - from.1) * dy)
    }

    /// The corners passed going counter-clockwise along the edge from `a` to `b`.
    fn between(&self, a: P, b: P) -> Vec<P> {
        let (side_a, along_a) = self.side_of(a);
        let (side_b, along_b) = self.side_of(b);
        let mut steps = (side_b + 8 - side_a) % 8;
        if steps == 0 && along_b < along_a {
            steps = 8;
        }
        let corners = self.corners();
        (0..steps).map(|k| corners[(side_a + k) % 8]).collect()
    }
}

/// The octagons around a straight piece at most `radius` away from it. A piece that is not at 0,
/// 45 or 90 degrees is cut into short parts, so that its octagons stay close to it.
pub(crate) fn around_piece(a: P, b: P, radius: i64) -> Vec<Oct> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        return vec![Oct::around(&[a, b]).grown(radius)];
    }
    let step = radius.saturating_mul(2).max(100_000);
    let parts = (dx.abs().max(dy.abs()) / step + 1).min(256);
    let at = |i: i64| {
        let part = |d: i64| (i128::from(d) * i128::from(i) / i128::from(parts)) as i64;
        (a.0 + part(dx), a.1 + part(dy))
    };
    (0..parts)
        .map(|i| Oct::around(&[at(i), at(i + 1)]).grown(radius))
        .collect()
}

/// Which way round an octagon a walk goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Turn {
    /// The shorter of the two ways, at every obstacle.
    Shortest,
    /// Counter-clockwise.
    Left,
    /// Clockwise.
    Right,
}

/// The length of a line.
pub(crate) fn length(path: &[P]) -> f64 {
    path.windows(2)
        .map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64))
        .sum()
}

/// Drops repeated points and points in the middle of a straight run (also of one that goes back
/// over itself), except the points that have to stay.
pub(crate) fn tidy(points: &[P], keep: &[P]) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(points.len());
    for &p in points {
        loop {
            if out.last() == Some(&p) {
                break;
            }
            let n = out.len();
            if n < 2 || keep.contains(&out[n - 1]) {
                out.push(p);
                break;
            }
            let (a, b) = (out[n - 2], out[n - 1]);
            let cross = i128::from(b.0 - a.0) * i128::from(p.1 - b.1)
                - i128::from(b.1 - a.1) * i128::from(p.0 - b.0);
            if cross != 0 {
                out.push(p);
                break;
            }
            out.pop();
        }
    }
    out
}

/// The point of a piece at the parameter `t`, rounded. When the piece is at 45 degrees the point
/// is rounded away from the inside (`outward` is -1 towards the start and 1 towards the end), so
/// that it stays on the piece.
fn point_at(a: P, b: P, t: f64, outward: f64) -> P {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx != 0 && dx.abs() == dy.abs() {
        let exact = t * dx.abs() as f64;
        let whole = if (exact - exact.round()).abs() < 0.25 {
            exact.round()
        } else if outward < 0.0 {
            exact.floor()
        } else {
            exact.ceil()
        };
        let k = (whole as i64).clamp(0, dx.abs());
        return (a.0 + k * dx.signum(), a.1 + k * dy.signum());
    }
    (
        a.0 + (t * dx as f64).round() as i64,
        a.1 + (t * dy as f64).round() as i64,
    )
}

/// The line with the part inside `hull` replaced by a walk along its edge, and the walked part.
/// `None` when an end of the line is inside the octagon.
fn splice(path: &[P], hull: &Oct, left: bool) -> Option<(Vec<P>, Vec<P>)> {
    let (first, last) = (*path.first()?, *path.last()?);
    if hull.holds_inside(first) || hull.holds_inside(last) {
        return None;
    }
    let mut enter: Option<(usize, P)> = None;
    let mut leave: Option<(usize, P)> = None;
    for (i, w) in path.windows(2).enumerate() {
        if let Some((t0, t1)) = hull.span(w[0], w[1]) {
            enter.get_or_insert((i, point_at(w[0], w[1], t0, -1.0)));
            leave = Some((i, point_at(w[0], w[1], t1, 1.0)));
        }
    }
    let (Some((i, p)), Some((j, q))) = (enter, leave) else {
        return Some((path.to_vec(), Vec::new()));
    };
    let around = if left {
        hull.between(p, q)
    } else {
        let mut back = hull.between(q, p);
        back.reverse();
        back
    };
    let mut walked = Vec::with_capacity(around.len() + 2);
    walked.push(p);
    walked.extend(around);
    walked.push(q);
    let mut out: Vec<P> = path[..=i].to_vec();
    out.extend_from_slice(&walked);
    out.extend_from_slice(&path[j + 1..]);
    Some((out, walked))
}

/// The first octagon, along the line, that the line goes into.
fn first_hit(path: &[P], obstacles: &[Oct]) -> Option<usize> {
    for w in path.windows(2) {
        let area = Oct::around(&[w[0], w[1]]);
        let mut best: Option<(f64, usize)> = None;
        for (k, obstacle) in obstacles.iter().enumerate() {
            if !area.near(obstacle, 0) {
                continue;
            }
            if let Some((t0, _)) = obstacle.span(w[0], w[1]) {
                if best.is_none_or(|(t, _)| t0 < t) {
                    best = Some((t0, k));
                }
            }
        }
        if let Some((_, k)) = best {
            return Some(k);
        }
    }
    None
}

/// Walks one octagon, joined with every octagon that the walk itself runs into, the way `left`
/// says.
fn walk_cluster(path: &[P], obstacles: &[Oct], start: usize, left: bool) -> Option<Vec<P>> {
    let mut cluster = obstacles[start];
    // every turn of the loop takes in at least one more obstacle
    for _ in 0..=obstacles.len() {
        let (out, walked) = splice(path, &cluster, left)?;
        let mut grown = cluster;
        for w in walked.windows(2) {
            let area = Oct::around(&[w[0], w[1]]);
            for obstacle in obstacles {
                if area.near(obstacle, 0)
                    && cluster.union(obstacle) != cluster
                    && obstacle.span(w[0], w[1]).is_some()
                {
                    grown = grown.union(obstacle);
                }
            }
        }
        if grown == cluster {
            return Some(out);
        }
        cluster = grown;
    }
    None
}

/// The most obstacles one line is walked around, one after the other.
const MAX_ROUNDS: usize = 64;

/// The line from the same first point to the same last point that does not go into any of the
/// octagons; `None` when there is no such walk (an end is inside an octagon, or too many
/// obstacles). The points in `keep` stay where they are.
pub(crate) fn walk(path: &[P], obstacles: &[Oct], turn: Turn, keep: &[P]) -> Option<Vec<P>> {
    let mut path = path.to_vec();
    for _ in 0..MAX_ROUNDS {
        let Some(k) = first_hit(&path, obstacles) else {
            return Some(tidy(&path, keep));
        };
        let next = match turn {
            Turn::Left => walk_cluster(&path, obstacles, k, true),
            Turn::Right => walk_cluster(&path, obstacles, k, false),
            Turn::Shortest => {
                let left = walk_cluster(&path, obstacles, k, true);
                let right = walk_cluster(&path, obstacles, k, false);
                match (left, right) {
                    (Some(l), Some(r)) => Some(if length(&l) <= length(&r) { l } else { r }),
                    (l, r) => l.or(r),
                }
            }
        }?;
        path = tidy(&next, keep);
        if path.len() < 2 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn octilinear(path: &[P]) -> bool {
        path.windows(2).all(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            dx == 0 || dy == 0 || dx.abs() == dy.abs()
        })
    }

    fn clear(path: &[P], obstacles: &[Oct]) -> bool {
        path.windows(2)
            .all(|w| obstacles.iter().all(|o| o.span(w[0], w[1]).is_none()))
    }

    #[test]
    fn the_octagon_of_a_point_holds_the_circle_of_its_radius() {
        let o = Oct::around(&[(0, 0)]).grown(1_000_000);
        for k in 0..360 {
            let a = f64::from(k).to_radians();
            let p = ((a.cos() * 999_000.0) as i64, (a.sin() * 999_000.0) as i64);
            assert!(o.holds_inside(p), "{k}");
        }
        assert!(!o.holds_inside((1_000_000, 0)));
        assert!(!o.holds_inside((800_000, 800_000)));
    }

    #[test]
    fn a_line_walks_around_an_octagon_both_ways_with_octilinear_pieces() {
        let o = Oct::around(&[(5_000_000, 100_000)]).grown(1_000_000);
        let line = [(0, 0), (10_000_000, 0)];
        let left = walk(&line, &[o], Turn::Left, &[]).unwrap();
        let right = walk(&line, &[o], Turn::Right, &[]).unwrap();
        let short = walk(&line, &[o], Turn::Shortest, &[]).unwrap();
        for path in [&left, &right, &short] {
            assert_eq!((path[0], path[path.len() - 1]), (line[0], line[1]));
            assert!(octilinear(path), "{path:?}");
            assert!(clear(path, &[o]), "{path:?}");
        }
        // the obstacle is above the line: the way below is the shorter one
        assert_eq!(short, left);
        assert!(left.iter().all(|p| p.1 <= 0), "{left:?}");
        assert!(right.iter().any(|p| p.1 > 1_000_000), "{right:?}");
    }

    #[test]
    fn octagons_that_overlap_are_walked_as_one_and_an_end_inside_one_is_refused() {
        let a = Oct::around(&[(4_000_000, 0)]).grown(1_000_000);
        let b = Oct::around(&[(5_000_000, -1_200_000)]).grown(1_000_000);
        let line = [(0, 0), (10_000_000, 0)];
        let path = walk(&line, &[a, b], Turn::Left, &[]).unwrap();
        assert!(clear(&path, &[a, b]), "{path:?}");
        assert!(octilinear(&path), "{path:?}");
        assert!(walk(&[(4_000_000, 0), (9_000_000, 0)], &[a], Turn::Shortest, &[]).is_none());
        // a line that only runs along an edge does not go into the octagon
        let edge = [(0, 1_000_000), (10_000_000, 1_000_000)];
        assert_eq!(
            walk(&edge, &[a], Turn::Shortest, &[]).unwrap(),
            edge.to_vec()
        );
    }

    #[test]
    fn walks_around_many_octagons_never_end_inside_one() {
        let mut seed = 11_u64;
        let mut next = |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((seed >> 33) % n) as i64
        };
        let mut done = 0;
        for _ in 0..400 {
            let obstacles: Vec<Oct> = (0..6)
                .map(|_| {
                    let at = (2_000_000 + next(16_000_000), next(6_000_000) - 3_000_000);
                    Oct::around(&[at]).grown(200_000 + next(800_000))
                })
                .collect();
            let line = [
                (0, 0),
                (8_000_000, 0),
                (12_000_000, 4_000_000),
                (20_000_000, 4_000_000),
            ];
            for turn in [Turn::Shortest, Turn::Left, Turn::Right] {
                if let Some(path) = walk(&line, &obstacles, turn, &[]) {
                    done += 1;
                    assert_eq!((path[0], path[path.len() - 1]), (line[0], line[3]));
                    assert!(clear(&path, &obstacles), "{path:?} {obstacles:?}");
                }
            }
        }
        assert!(done > 600, "{done}");
    }

    #[test]
    fn a_piece_at_another_angle_is_cut_into_short_octagons() {
        assert_eq!(
            around_piece((0, 0), (5_000_000, 5_000_000), 100_000).len(),
            1
        );
        let parts = around_piece((0, 0), (10_000_000, 1_000_000), 100_000);
        assert!(parts.len() > 10);
        assert!(parts.iter().any(|o| o.holds_inside((5_000_000, 500_000))));
        assert!(parts
            .iter()
            .all(|o| !o.holds_inside((5_000_000, 1_200_000))));
    }

    #[test]
    fn tidying_keeps_the_points_that_have_to_stay() {
        let line = [(0, 0), (5, 0), (5, 0), (9, 0), (9, 4)];
        assert_eq!(tidy(&line, &[]), vec![(0, 0), (9, 0), (9, 4)]);
        assert_eq!(tidy(&line, &[(5, 0)]), vec![(0, 0), (5, 0), (9, 0), (9, 4)]);
    }
}
