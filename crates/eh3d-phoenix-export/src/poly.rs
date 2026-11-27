// SPDX-License-Identifier: GPL-3.0-or-later
//! The polygon work of the copper files: turning the outline of a pad to its angle, and keeping
//! what one pour clears inside that pour. Clear polarity takes away everything drawn before it, so
//! a keep-away of a pour that reaches a pour drawn earlier is cut to the outline of its own pour.

use eh3d_phoenix_board::connectivity::point_in_polygon;
use eh3d_phoenix_model::{segments_intersect, Point};

type Xy = (f64, f64);

/// The number of sides of the polygon that stands for a circle when it has to be cut.
const CIRCLE_SIDES: usize = 32;

/// Pieces smaller than this, in square nanometres, are rounding dust and are not written.
const DUST_NM2: i128 = 2_000_000;

/// `(x, y)` turned counter-clockwise by `udeg` micro-degrees and moved to `at`; right angles are
/// exact, other angles are rounded to the nanometre.
pub(crate) fn place(at: Point, x: f64, y: f64, udeg: i64) -> Option<Point> {
    let (rx, ry) = match udeg.rem_euclid(360_000_000) {
        0 => (x, y),
        90_000_000 => (-y, x),
        180_000_000 => (-x, -y),
        270_000_000 => (y, -x),
        a => {
            let (s, c) = (a as f64 / 1_000_000.0).to_radians().sin_cos();
            (x * c - y * s, x * s + y * c)
        }
    };
    Point::from_nm(
        at.x_nm.get() + rx.round() as i64,
        at.y_nm.get() + ry.round() as i64,
    )
    .ok()
}

/// A ring without repeated corners.
pub(crate) fn tidy(mut ring: Vec<Point>) -> Vec<Point> {
    ring.dedup();
    while ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    ring
}

/// How much a circle of `radius` has to grow so that a polygon of `sides` chords on the grown
/// circle holds all of it.
pub(crate) fn chord_room(radius: i64, sides: usize) -> i64 {
    let half = std::f64::consts::PI / sides as f64;
    (radius as f64 * (1.0 / half.cos() - 1.0)).ceil() as i64 + 1
}

/// A convex polygon that holds the whole of the line from `a` to `b` drawn with a round pen of
/// `width` (a disc when both ends are the same).
pub(crate) fn stadium(a: Point, b: Point, width: i64) -> Vec<Point> {
    let radius = width / 2 + width % 2;
    let radius = (radius + chord_room(radius, CIRCLE_SIDES)) as f64;
    let dx = (b.x_nm.get() - a.x_nm.get()) as f64;
    let dy = (b.y_nm.get() - a.y_nm.get()) as f64;
    let along = if dx == 0.0 && dy == 0.0 {
        0.0
    } else {
        dy.atan2(dx)
    };
    let half = CIRCLE_SIDES / 2;
    let step = std::f64::consts::PI / half as f64;
    let mut ring = Vec::with_capacity(CIRCLE_SIDES + 2);
    // the half circle about b, from its right to its left, then the one about a
    for (centre, start) in [
        (b, along - std::f64::consts::FRAC_PI_2),
        (a, along + std::f64::consts::FRAC_PI_2),
    ] {
        for k in 0..=half {
            let angle = start + step * k as f64;
            if let Some(p) = place(centre, radius * angle.cos(), radius * angle.sin(), 0) {
                ring.push(p);
            }
        }
    }
    tidy(ring)
}

fn xy(p: Point) -> (i128, i128) {
    (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()))
}

fn cross(o: Point, a: Point, b: Point) -> i128 {
    let (o, a, b) = (xy(o), xy(a), xy(b));
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Twice the signed area, positive for a ring that turns counter-clockwise.
pub(crate) fn double_area(ring: &[Point]) -> i128 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (xy(ring[i]), xy(ring[(i + 1) % n]));
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

pub(crate) fn bounds(ring: &[Point]) -> (i64, i64, i64, i64) {
    let xs = ring.iter().map(|p| p.x_nm.get());
    let ys = ring.iter().map(|p| p.y_nm.get());
    (
        xs.clone().min().unwrap_or(0),
        ys.clone().min().unwrap_or(0),
        xs.max().unwrap_or(0),
        ys.max().unwrap_or(0),
    )
}

fn boxes_apart(a: &[Point], b: &[Point]) -> bool {
    let (a, b) = (bounds(a), bounds(b));
    a.2 < b.0 || b.2 < a.0 || a.3 < b.1 || b.3 < a.1
}

fn edges_cross(a: &[Point], b: &[Point]) -> bool {
    let (na, nb) = (a.len(), b.len());
    (0..na)
        .any(|i| (0..nb).any(|j| segments_intersect(a[i], a[(i + 1) % na], b[j], b[(j + 1) % nb])))
}

/// Whether the two filled polygons may share copper. It never says no when they do.
pub(crate) fn meet(a: &[Point], b: &[Point]) -> bool {
    if a.len() < 3 || b.len() < 3 || boxes_apart(a, b) {
        return false;
    }
    point_in_polygon(a[0], b) || point_in_polygon(b[0], a) || edges_cross(a, b)
}

/// Whether `inner` lies wholly inside `outer`, clear of its border.
pub(crate) fn within(inner: &[Point], outer: &[Point]) -> bool {
    inner.len() >= 3
        && outer.len() >= 3
        && inner.iter().all(|p| point_in_polygon(*p, outer))
        && !edges_cross(inner, outer)
}

fn convex(ring: &[Point]) -> bool {
    let n = ring.len();
    let turns: Vec<i128> = (0..n)
        .map(|i| cross(ring[i], ring[(i + 1) % n], ring[(i + 2) % n]))
        .collect();
    turns.iter().all(|t| *t >= 0) || turns.iter().all(|t| *t <= 0)
}

/// The polygon as convex pieces that turn counter-clockwise: itself when it is convex, otherwise
/// the triangles of ear clipping. A polygon that crosses itself gives what could be clipped.
fn convex_pieces(ring: &[Point]) -> Vec<Vec<Point>> {
    let mut ring = tidy(ring.to_vec());
    if ring.len() < 3 {
        return Vec::new();
    }
    if double_area(&ring) < 0 {
        ring.reverse();
    }
    if convex(&ring) {
        return vec![ring];
    }
    let mut out = Vec::new();
    while ring.len() > 3 {
        let n = ring.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            cross(a, b, c) > 0
                && !ring.iter().any(|&p| {
                    p != a
                        && p != b
                        && p != c
                        && cross(a, b, p) >= 0
                        && cross(b, c, p) >= 0
                        && cross(c, a, p) >= 0
                })
        });
        // a corner in the middle of a straight run is dropped when no ear is left
        let Some(i) = ear.or_else(|| {
            (0..n).find(|&i| cross(ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]) == 0)
        }) else {
            break;
        };
        let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        if cross(a, b, c) != 0 {
            out.push(vec![a, b, c]);
        }
        ring.remove(i);
    }
    if ring.len() == 3 && double_area(&ring) != 0 {
        out.push(ring);
    }
    out
}

/// Sutherland-Hodgman: the part of the convex `subject` inside the convex, counter-clockwise
/// `clip`.
fn clip_convex(subject: &[Point], clip: &[Point]) -> Vec<Point> {
    let mut ring: Vec<Xy> = subject
        .iter()
        .map(|p| (p.x_nm.get() as f64, p.y_nm.get() as f64))
        .collect();
    let n = clip.len();
    for i in 0..n {
        let (a, b) = (clip[i], clip[(i + 1) % n]);
        let (ax, ay) = (a.x_nm.get() as f64, a.y_nm.get() as f64);
        let (dx, dy) = (b.x_nm.get() as f64 - ax, b.y_nm.get() as f64 - ay);
        let side = |p: &Xy| dx * (p.1 - ay) - dy * (p.0 - ax);
        let mut next: Vec<Xy> = Vec::with_capacity(ring.len() + 1);
        for k in 0..ring.len() {
            let (p, q) = (ring[k], ring[(k + 1) % ring.len()]);
            let (sp, sq) = (side(&p), side(&q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
                let t = sp / (sp - sq);
                next.push((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t));
            }
        }
        ring = next;
        if ring.len() < 3 {
            return Vec::new();
        }
    }
    tidy(
        ring.into_iter()
            .filter_map(|(x, y)| Point::from_nm(x.round() as i64, y.round() as i64).ok())
            .collect(),
    )
}

/// The parts of `shape` inside `outline`, as convex polygons.
pub(crate) fn intersection(shape: &[Point], outline: &[Point]) -> Vec<Vec<Point>> {
    let outline = convex_pieces(outline);
    let mut out = Vec::new();
    for piece in convex_pieces(shape) {
        for room in &outline {
            if boxes_apart(&piece, room) {
                continue;
            }
            let part = clip_convex(&piece, room);
            if part.len() >= 3 && double_area(&part).abs() > DUST_NM2 {
                out.push(part);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    const MM: i64 = 1_000_000;

    fn square(x: i64, y: i64, side: i64) -> Vec<Point> {
        vec![
            pt(x, y),
            pt(x + side, y),
            pt(x + side, y + side),
            pt(x, y + side),
        ]
    }

    fn total(pieces: &[Vec<Point>]) -> i128 {
        pieces.iter().map(|p| double_area(p).abs()).sum::<i128>() / 2
    }

    #[test]
    fn the_intersection_of_two_squares_is_their_common_part() {
        let out = intersection(&square(0, 0, 4 * MM), &square(3 * MM, 2 * MM, 4 * MM));
        assert_eq!(total(&out), 2 * i128::from(MM) * i128::from(MM));
        assert!(intersection(&square(0, 0, MM), &square(5 * MM, 0, MM)).is_empty());
    }

    #[test]
    fn a_concave_outline_is_cut_into_triangles_that_cover_it_once() {
        // an L: a square of 4 mm without its upper right quarter, 12 mm2, given clockwise
        let l = [
            pt(0, 0),
            pt(0, 4 * MM),
            pt(2 * MM, 4 * MM),
            pt(2 * MM, 2 * MM),
            pt(4 * MM, 2 * MM),
            pt(4 * MM, 0),
        ];
        let square_mm2 = i128::from(MM) * i128::from(MM);
        let all = intersection(&square(-MM, -MM, 6 * MM), &l);
        assert_eq!(total(&all), 12 * square_mm2);
        // the middle square of 2 mm has one quarter in the missing corner
        let middle = intersection(&square(MM, MM, 2 * MM), &l);
        assert_eq!(total(&middle), 3 * square_mm2);
    }

    #[test]
    fn a_stadium_holds_the_round_pen_and_little_more() {
        let ring = stadium(pt(0, 0), pt(10 * MM, 0), 2 * MM);
        for p in &ring {
            let x = p.x_nm.get().clamp(0, 10 * MM);
            let d = ((p.x_nm.get() - x) as f64).hypot(p.y_nm.get() as f64);
            assert!(d >= MM as f64 - 1.0 && d < 1.006 * MM as f64, "{d}");
        }
        let disc = stadium(pt(0, 0), pt(0, 0), 2 * MM);
        let area = double_area(&disc) as f64 / 2.0e12;
        assert!(area > std::f64::consts::PI && area < 3.19, "{area}");
    }

    #[test]
    fn within_and_meet_tell_inside_across_and_apart() {
        let big = square(0, 0, 10 * MM);
        assert!(within(&square(MM, MM, MM), &big));
        assert!(!within(&square(9 * MM, MM, 2 * MM), &big));
        assert!(meet(&square(9 * MM, MM, 2 * MM), &big));
        assert!(meet(&big, &square(MM, MM, MM)));
        assert!(!meet(&square(20 * MM, 0, MM), &big));
    }

    #[test]
    fn a_point_is_turned_about_the_origin_and_moved() {
        assert_eq!(place(pt(10, 20), 5.0, 0.0, 90_000_000), Some(pt(10, 25)));
        assert_eq!(
            place(pt(0, 0), 1_000_000.0, 0.0, 45_000_000),
            Some(pt(707_107, 707_107))
        );
    }
}
