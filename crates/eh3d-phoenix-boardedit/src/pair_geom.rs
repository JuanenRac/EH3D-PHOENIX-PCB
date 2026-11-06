// SPDX-License-Identifier: GPL-3.0-or-later
//! The geometry of a pair of tracks, in integer nanometres: the two paths that run parallel on
//! both sides of a centre line made of horizontal, vertical and 45-degree pieces.

use eh3d_phoenix_model::Point;

use crate::codes::{BoardEditError, GEOMETRY_INVALID};
use crate::shape::pt;

/// The direction of a straight piece that is horizontal, vertical or at 45 degrees, as the sign
/// of each axis; `None` for a piece of no length or at any other angle.
pub fn direction(a: Point, b: Point) -> Option<(i64, i64)> {
    let dx = b.x_nm.get() - a.x_nm.get();
    let dy = b.y_nm.get() - a.y_nm.get();
    if dx == 0 && dy == 0 {
        return None;
    }
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        Some((dx.signum(), dy.signum()))
    } else {
        None
    }
}

fn geometry(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(GEOMETRY_INVALID, what)
}

/// Rounded division to the nearest whole number, halves up; `d` is not zero.
fn round_div(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    (2 * n + d).div_euclid(2 * d)
}

/// The offset of each axis that moves a piece of direction `dir` sideways by `h` nanometres
/// (to the left of the direction when `h` is above zero). A 45-degree piece moves `h` divided by
/// the square root of two on each axis, to the nearest nanometre.
fn offset_vector(dir: (i64, i64), h: i64) -> (i64, i64) {
    let magnitude = h.unsigned_abs() as i128;
    let k = if dir.0 != 0 && dir.1 != 0 {
        round_div(magnitude * 70_710_678, 100_000_000)
    } else {
        magnitude
    };
    let k = i64::try_from(k).unwrap_or(i64::MAX) * h.signum();
    (-dir.1 * k, dir.0 * k)
}

/// The path that runs at a distance `h` from the path through `points`, to the left of its
/// direction when `h` is above zero and to the right when it is below. Every piece of `points`
/// must be horizontal, vertical or at 45 degrees; a corner is the meeting of the two offset lines.
/// A path whose pieces are too short for the offset (they would vanish or turn round) is refused.
pub fn offset_path(points: &[Point], h: i64) -> Result<Vec<Point>, BoardEditError> {
    if points.len() < 2 {
        return Err(geometry("a path needs at least two points"));
    }
    let mut dirs: Vec<(i64, i64)> = Vec::with_capacity(points.len() - 1);
    for pair in points.windows(2) {
        dirs.push(
            direction(pair[0], pair[1])
                .ok_or_else(|| geometry("a piece is not horizontal, vertical or at 45 degrees"))?,
        );
    }
    let at = |p: Point, o: (i64, i64)| pt(p.x_nm.get() + o.0, p.y_nm.get() + o.1);
    let offsets: Vec<(i64, i64)> = dirs.iter().map(|d| offset_vector(*d, h)).collect();
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    out.push(at(points[0], offsets[0])?);
    for (i, window) in points.windows(3).enumerate() {
        let corner = window[1];
        let (d1, d2) = (dirs[i], dirs[i + 1]);
        if d1 == d2 {
            out.push(at(corner, offsets[i + 1])?);
            continue;
        }
        if d1.0 == -d2.0 && d1.1 == -d2.1 {
            return Err(geometry("the path turns back on itself"));
        }
        let a1 = at(corner, offsets[i])?;
        let a2 = at(corner, offsets[i + 1])?;
        let cross = i128::from(d1.0) * i128::from(d2.1) - i128::from(d1.1) * i128::from(d2.0);
        let wx = i128::from(a2.x_nm.get()) - i128::from(a1.x_nm.get());
        let wy = i128::from(a2.y_nm.get()) - i128::from(a1.y_nm.get());
        let numerator = wx * i128::from(d2.1) - wy * i128::from(d2.0);
        let x = i128::from(a1.x_nm.get()) + round_div(numerator * i128::from(d1.0), cross);
        let y = i128::from(a1.y_nm.get()) + round_div(numerator * i128::from(d1.1), cross);
        let (x, y) = (
            i64::try_from(x).map_err(|_| geometry("a corner is out of range"))?,
            i64::try_from(y).map_err(|_| geometry("a corner is out of range"))?,
        );
        out.push(pt(x, y)?);
    }
    out.push(at(points[points.len() - 1], offsets[offsets.len() - 1])?);
    for (i, pair) in out.windows(2).enumerate() {
        if direction(pair[0], pair[1]) != Some(dirs[i]) {
            return Err(geometry(
                "a piece is too short for the width and the gap of the pair",
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn the_direction_is_the_sign_of_each_axis_or_nothing() {
        assert_eq!(direction(p(0, 0), p(5, 0)), Some((1, 0)));
        assert_eq!(direction(p(0, 0), p(0, -5)), Some((0, -1)));
        assert_eq!(direction(p(0, 0), p(-3, 3)), Some((-1, 1)));
        assert_eq!(direction(p(0, 0), p(4, 3)), None);
        assert_eq!(direction(p(2, 2), p(2, 2)), None);
    }

    #[test]
    fn a_straight_path_is_moved_to_the_left_and_to_the_right() {
        let line = [p(0, 0), p(10_000_000, 0)];
        let left = offset_path(&line, 175_000).unwrap();
        assert_eq!(left, vec![p(0, 175_000), p(10_000_000, 175_000)]);
        let right = offset_path(&line, -175_000).unwrap();
        assert_eq!(right, vec![p(0, -175_000), p(10_000_000, -175_000)]);
        let down = offset_path(&[p(0, 0), p(0, -10_000_000)], 100_000).unwrap();
        assert_eq!(down, vec![p(100_000, 0), p(100_000, -10_000_000)]);
    }

    #[test]
    fn an_orthogonal_corner_is_the_meeting_of_the_two_offset_lines() {
        // east then north: the left side is the inside of the turn
        let path = [p(0, 0), p(10_000_000, 0), p(10_000_000, 10_000_000)];
        let left = offset_path(&path, 100_000).unwrap();
        assert_eq!(
            left,
            vec![
                p(0, 100_000),
                p(9_900_000, 100_000),
                p(9_900_000, 10_000_000)
            ]
        );
        let right = offset_path(&path, -100_000).unwrap();
        assert_eq!(
            right,
            vec![
                p(0, -100_000),
                p(10_100_000, -100_000),
                p(10_100_000, 10_000_000)
            ]
        );
    }

    #[test]
    fn a_45_degree_piece_keeps_its_angle_and_the_distance_to_one_nanometre() {
        let path = [p(0, 0), p(10_000_000, 0), p(15_000_000, 5_000_000)];
        for h in [150_000, -150_000, 175_001] {
            let out = offset_path(&path, h).unwrap();
            assert_eq!(out.len(), 3);
            assert_eq!(direction(out[0], out[1]), Some((1, 0)));
            assert_eq!(direction(out[1], out[2]), Some((1, 1)));
            // the distance from the offset corner to the centre line of the diagonal piece
            let d = (out[2].x_nm.get() - out[2].y_nm.get() - 10_000_000) as f64 / 2f64.sqrt();
            assert!((d.abs() - h.abs() as f64).abs() < 1.0, "{h}: {d}");
        }
    }

    #[test]
    fn a_path_too_short_for_the_offset_or_not_octilinear_is_refused() {
        // an inside corner whose short piece would turn round
        let short = [p(0, 0), p(10_000_000, 0), p(10_000_000, 50_000)];
        assert!(offset_path(&short, 100_000).is_err());
        assert!(offset_path(&[p(0, 0), p(4, 3)], 10).is_err());
        assert!(offset_path(&[p(0, 0)], 10).is_err());
        let back = [p(0, 0), p(10, 0), p(0, 0)];
        assert!(offset_path(&back, 1).is_err());
    }
}
