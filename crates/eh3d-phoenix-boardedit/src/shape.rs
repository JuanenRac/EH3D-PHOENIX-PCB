// SPDX-License-Identifier: GPL-3.0-or-later
//! The shapes that the operations make: the pieces of a route between two points (horizontal,
//! vertical and 45-degree pieces) and the meanders that add length to a straight piece. All in
//! integer nanometres, with no floating point.

use eh3d_phoenix_board::Segment;
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::codes::{BoardEditError, GEOMETRY_INVALID, TUNE_IMPOSSIBLE};

/// What a route between two points may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// Horizontal and vertical pieces only.
    #[default]
    Orthogonal,
    /// Horizontal, vertical and 45-degree pieces.
    Diagonal,
}

/// The side of the straight piece the meanders are drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeanderSide {
    /// The side that keeps the rules, the left side first.
    #[default]
    Auto,
    /// The left of the direction of the piece.
    Left,
    /// The right of the direction of the piece.
    Right,
}

/// A point, checked to be inside the design range.
pub fn pt(x: i64, y: i64) -> Result<Point, BoardEditError> {
    Point::from_nm(x, y).map_err(|e| BoardEditError::new(GEOMETRY_INVALID, e.to_string()))
}

/// The corners of the route from `a` to `b`, with both ends. A piece that is horizontal, vertical
/// or (with the diagonal shape) at 45 degrees is a straight run; any other pair of points gets one
/// corner, and the piece that is not diagonal goes first when `horizontal_first` is true.
pub fn legs(
    a: Point,
    b: Point,
    shape: Shape,
    horizontal_first: bool,
) -> Result<Vec<Point>, BoardEditError> {
    let (ax, ay) = (a.x_nm.get(), a.y_nm.get());
    let (bx, by) = (b.x_nm.get(), b.y_nm.get());
    let (dx, dy) = (bx - ax, by - ay);
    let straight = dx == 0 || dy == 0 || (shape == Shape::Diagonal && dx.abs() == dy.abs());
    if straight {
        return Ok(vec![a, b]);
    }
    match shape {
        Shape::Orthogonal => {
            let corner = if horizontal_first {
                pt(bx, ay)?
            } else {
                pt(ax, by)?
            };
            Ok(vec![a, corner, b])
        }
        Shape::Diagonal => {
            let d = dx.abs().min(dy.abs());
            let (sx, sy) = (dx.signum(), dy.signum());
            // the straight piece runs along the longer axis, the diagonal piece covers d on both
            let (ox, oy) = if dx.abs() > dy.abs() {
                (sx * (dx.abs() - d), 0)
            } else {
                (0, sy * (dy.abs() - d))
            };
            let corner = if horizontal_first {
                pt(ax + ox, ay + oy)?
            } else {
                pt(bx - ox, by - oy)?
            };
            Ok(vec![a, corner, b])
        }
    }
}

/// The straight pieces through a list of points; a piece of no length is left out.
pub fn lines_through(points: &[Point]) -> Vec<Segment> {
    let mut out = Vec::new();
    for pair in points.windows(2) {
        if pair[0] != pair[1] {
            out.push(Segment::Line {
                start: pair[0],
                end: pair[1],
            });
        }
    }
    out
}

/// The pieces of a route through several points, each pair joined by [`legs`].
pub fn route_pieces(
    points: &[Point],
    shape: Shape,
    horizontal_first: bool,
) -> Result<Vec<Segment>, BoardEditError> {
    let mut all: Vec<Point> = Vec::new();
    for pair in points.windows(2) {
        for p in legs(pair[0], pair[1], shape, horizontal_first)? {
            if all.last() != Some(&p) {
                all.push(p);
            }
        }
    }
    Ok(lines_through(&all))
}

/// The length of a horizontal or vertical straight piece; `None` for any other.
pub fn axis_length(a: Point, b: Point) -> Option<i64> {
    let dx = b.x_nm.get() - a.x_nm.get();
    let dy = b.y_nm.get() - a.y_nm.get();
    if (dx == 0) == (dy == 0) {
        return None;
    }
    Some(dx.abs() + dy.abs())
}

/// What a serpentine needs.
#[derive(Debug, Clone, Copy)]
pub struct Tuning {
    /// The length to add, in nanometres.
    pub extra_nm: i64,
    /// The amplitude to start with.
    pub amplitude_nm: i64,
    /// The largest amplitude.
    pub max_amplitude_nm: i64,
    /// The run of one half period.
    pub pitch_nm: i64,
}

/// A serpentine that replaces the horizontal or vertical piece from `start` to `end` and adds
/// `tuning.extra_nm` of length (to within a nanometre): the pieces and the length they add. The
/// bumps are square, centred on the piece, with a straight lead at both ends; `side` is 1 for the
/// left of the direction of the piece and -1 for the right.
pub fn serpentine(
    start: Point,
    end: Point,
    tuning: Tuning,
    side: i64,
) -> Result<(Vec<Segment>, i64), BoardEditError> {
    let fail = |why: &str| BoardEditError::new(TUNE_IMPOSSIBLE, why.to_owned());
    let len =
        axis_length(start, end).ok_or_else(|| fail("the piece is not horizontal or vertical"))?;
    let Tuning {
        extra_nm,
        amplitude_nm,
        max_amplitude_nm,
        pitch_nm,
    } = tuning;
    if extra_nm < 2 {
        return Err(fail("the length to add is less than 2 nm"));
    }
    let usable = len - 2 * pitch_nm;
    let bumps_fit = if usable > 0 {
        usable / (2 * pitch_nm)
    } else {
        0
    };
    if bumps_fit < 1 {
        return Err(fail("the piece is too short for a meander"));
    }
    let per_bump = 2 * amplitude_nm;
    let bumps_wanted = extra_nm / per_bump + i64::from(extra_nm % per_bump != 0);
    let bumps = bumps_wanted.min(bumps_fit).min(extra_nm / 2).max(1);
    let base = extra_nm / (2 * bumps);
    let remainder = (extra_nm - 2 * base * bumps) / 2;
    let largest = base + i64::from(remainder > 0);
    if largest > max_amplitude_nm {
        return Err(fail("the piece is too short for the length to add"));
    }
    let (sx, sy) = (start.x_nm.get(), start.y_nm.get());
    let dx = end.x_nm.get() - sx;
    let dy = end.y_nm.get() - sy;
    let (ux, uy) = (dx.signum(), dy.signum());
    let (nx, ny) = (-uy * side, ux * side);
    let at =
        |along: i64, offset: i64| pt(sx + ux * along + nx * offset, sy + uy * along + ny * offset);
    let lead = (len - 2 * pitch_nm * bumps) / 2;
    let mut points = vec![at(0, 0)?];
    let mut along = lead;
    if lead > 0 {
        points.push(at(along, 0)?);
    }
    for i in 0..bumps {
        let amplitude = base + i64::from(i < remainder);
        points.push(at(along, amplitude)?);
        along += pitch_nm;
        points.push(at(along, amplitude)?);
        points.push(at(along, 0)?);
        along += pitch_nm;
        points.push(at(along, 0)?);
    }
    points.push(at(len, 0)?);
    let added = 2 * (base * bumps + remainder);
    Ok((lines_through(&points), added))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn total(path: &[Segment]) -> i64 {
        path.iter()
            .map(|s| match s {
                Segment::Line { start, end } => {
                    let dx = (end.x_nm.get() - start.x_nm.get()) as f64;
                    let dy = (end.y_nm.get() - start.y_nm.get()) as f64;
                    dx.hypot(dy).round() as i64
                }
                Segment::Arc { .. } => 0,
            })
            .sum()
    }

    #[test]
    fn aligned_points_make_one_piece_and_others_one_corner() {
        let a = p(0, 0);
        assert_eq!(
            legs(a, p(5, 0), Shape::Orthogonal, true).unwrap(),
            vec![a, p(5, 0)]
        );
        assert_eq!(
            legs(a, p(4, 3), Shape::Orthogonal, true).unwrap(),
            vec![a, p(4, 0), p(4, 3)]
        );
        assert_eq!(
            legs(a, p(4, 3), Shape::Orthogonal, false).unwrap(),
            vec![a, p(0, 3), p(4, 3)]
        );
        // a diagonal pair is one piece only with the diagonal shape
        assert_eq!(legs(a, p(3, 3), Shape::Diagonal, true).unwrap().len(), 2);
        assert_eq!(legs(a, p(3, 3), Shape::Orthogonal, true).unwrap().len(), 3);
    }

    #[test]
    fn a_diagonal_route_has_one_straight_and_one_45_degree_piece() {
        let a = p(0, 0);
        let b = p(10, 4);
        assert_eq!(
            legs(a, b, Shape::Diagonal, true).unwrap(),
            vec![a, p(6, 0), b]
        );
        assert_eq!(
            legs(a, b, Shape::Diagonal, false).unwrap(),
            vec![a, p(4, 4), b]
        );
        assert_eq!(
            legs(a, p(-3, -9), Shape::Diagonal, true).unwrap(),
            vec![a, p(0, -6), p(-3, -9)]
        );
    }

    #[test]
    fn a_route_through_points_joins_the_pieces_and_drops_the_empty_ones() {
        let path = route_pieces(
            &[p(0, 0), p(4, 0), p(4, 0), p(4, 5)],
            Shape::Orthogonal,
            true,
        )
        .unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].end(), path[1].start());
        assert!(route_pieces(&[p(1, 1), p(1, 1)], Shape::Orthogonal, true)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_serpentine_adds_the_length_asked_for_and_keeps_both_ends() {
        let start = p(0, 0);
        let end = p(20_000_000, 0);
        let tuning = Tuning {
            extra_nm: 5_000_000,
            amplitude_nm: 1_000_000,
            max_amplitude_nm: 3_000_000,
            pitch_nm: 400_000,
        };
        let (path, added) = serpentine(start, end, tuning, 1).unwrap();
        assert!((added - 5_000_000).abs() <= 1);
        assert_eq!(path.first().unwrap().start(), start);
        assert_eq!(path.last().unwrap().end(), end);
        for pair in path.windows(2) {
            assert_eq!(pair[0].end(), pair[1].start());
        }
        assert_eq!(total(&path), 20_000_000 + added);
        // the other side mirrors it
        let (other, _) = serpentine(start, end, tuning, -1).unwrap();
        assert_eq!(total(&other), total(&path));
        assert!(other.iter().any(|s| s.start().y_nm.get() < 0));
        assert!(path.iter().all(|s| s.start().y_nm.get() >= 0));
    }

    #[test]
    fn a_vertical_piece_is_tuned_too_and_a_hopeless_one_is_refused() {
        let tuning = Tuning {
            extra_nm: 2_000_000,
            amplitude_nm: 1_000_000,
            max_amplitude_nm: 3_000_000,
            pitch_nm: 400_000,
        };
        let (path, added) = serpentine(p(0, 0), p(0, -10_000_000), tuning, 1).unwrap();
        assert!((added - 2_000_000).abs() <= 1);
        assert_eq!(path.last().unwrap().end(), p(0, -10_000_000));
        // too short, not axis aligned, not enough length to add
        for (a, b, extra) in [
            (p(0, 0), p(500_000, 0), 2_000_000),
            (p(0, 0), p(5_000_000, 5_000_000), 2_000_000),
            (p(0, 0), p(10_000_000, 0), 1),
        ] {
            let error = serpentine(
                a,
                b,
                Tuning {
                    extra_nm: extra,
                    ..tuning
                },
                1,
            )
            .unwrap_err();
            assert_eq!(error.code, TUNE_IMPOSSIBLE);
        }
        // a piece that fits only two bumps cannot add much
        let error = serpentine(
            p(0, 0),
            p(2_000_000, 0),
            Tuning {
                extra_nm: 40_000_000,
                ..tuning
            },
            1,
        )
        .unwrap_err();
        assert_eq!(error.code, TUNE_IMPOSSIBLE);
    }
}
