//! Points, orientation predicates, boxes and arcs on the integer grid.
//!
//! The predicates work on coordinate differences, which the range of [`Nm`] keeps below `2^31`,
//! so they use `i64` only and cannot overflow. Anything that builds a new point (an intersection,
//! an arc centre) is not here: it needs `i128` and named overflow boundaries.

use serde::{Deserialize, Serialize};

use crate::error::ModelError;
use crate::units::{Nm, Udeg};

/// A point of the design plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal coordinate in nanometres.
    pub x_nm: Nm,
    /// Vertical coordinate in nanometres.
    pub y_nm: Nm,
}

impl Point {
    /// Builds a point from raw nanometres, checking both coordinates.
    pub fn from_nm(x: i64, y: i64) -> Result<Point, ModelError> {
        Ok(Point {
            x_nm: Nm::new(x)?,
            y_nm: Nm::new(y)?,
        })
    }

    /// Squared distance. A difference is below `2^31`, its square below `2^62`, the sum below
    /// `2^63`, but the result is returned as `i128` so callers can compare it with squared
    /// distances built from wider values.
    pub fn distance_squared(self, other: Point) -> i128 {
        let dx = i128::from(self.x_nm.delta(other.x_nm));
        let dy = i128::from(self.y_nm.delta(other.y_nm));
        dx * dx + dy * dy
    }
}

/// On which side of the directed line `a -> b` a point lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Turning right, as seen with y pointing up.
    Clockwise,
    /// On the line.
    Collinear,
    /// Turning left, as seen with y pointing up.
    CounterClockwise,
}

/// The orientation of the triple `a, b, c`: the sign of `(b - a) x (c - a)`.
///
/// Each difference is below `2^31` in absolute value, each product below `2^62` and the
/// difference of two products below `2^63`, so plain `i64` arithmetic is exact.
pub fn orient(a: Point, b: Point, c: Point) -> Orientation {
    let abx = b.x_nm.delta(a.x_nm);
    let aby = b.y_nm.delta(a.y_nm);
    let acx = c.x_nm.delta(a.x_nm);
    let acy = c.y_nm.delta(a.y_nm);
    let cross = abx * acy - aby * acx;
    match cross.cmp(&0) {
        std::cmp::Ordering::Greater => Orientation::CounterClockwise,
        std::cmp::Ordering::Less => Orientation::Clockwise,
        std::cmp::Ordering::Equal => Orientation::Collinear,
    }
}

/// An axis-aligned box, corners included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    /// Corner with the smallest coordinates.
    pub min: Point,
    /// Corner with the largest coordinates.
    pub max: Point,
}

impl Rect {
    /// The smallest box that holds both points, whatever their order.
    pub fn from_corners(a: Point, b: Point) -> Rect {
        Rect {
            min: Point {
                x_nm: a.x_nm.min(b.x_nm),
                y_nm: a.y_nm.min(b.y_nm),
            },
            max: Point {
                x_nm: a.x_nm.max(b.x_nm),
                y_nm: a.y_nm.max(b.y_nm),
            },
        }
    }

    /// True when the point is inside or on the border.
    pub fn contains(&self, p: Point) -> bool {
        p.x_nm >= self.min.x_nm
            && p.x_nm <= self.max.x_nm
            && p.y_nm >= self.min.y_nm
            && p.y_nm <= self.max.y_nm
    }

    /// True when the boxes share at least a point.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.min.x_nm <= other.max.x_nm
            && other.min.x_nm <= self.max.x_nm
            && self.min.y_nm <= other.max.y_nm
            && other.min.y_nm <= self.max.y_nm
    }
}

/// True when the closed segments `a-b` and `c-d` share at least a point (touching ends and
/// overlapping collinear segments count).
pub fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 != o2 && o3 != o4 {
        return true;
    }
    // Remaining touching cases: an end point that lies on the other segment.
    let on = |p: Point, q: Point, r: Point| Rect::from_corners(p, q).contains(r);
    (o1 == Orientation::Collinear && on(a, b, c))
        || (o2 == Orientation::Collinear && on(a, b, d))
        || (o3 == Orientation::Collinear && on(c, d, a))
        || (o4 == Orientation::Collinear && on(c, d, b))
}

/// An arc stored as start point, end point and signed sweep. The centre and the radius are
/// derived by whoever needs them; an exact centre is generally not an integer number of
/// nanometres, so it is never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Arc {
    /// Where the arc starts.
    pub start: Point,
    /// Where the arc ends.
    pub end: Point,
    /// Signed sweep in micro-degrees: positive is counter-clockwise.
    pub sweep_udeg: Udeg,
}

impl Arc {
    /// Checks that the arc has a shape: the sweep is not zero, and the end points differ unless
    /// the sweep is a full turn (a circle).
    pub fn new(start: Point, end: Point, sweep: Udeg) -> Result<Arc, ModelError> {
        if sweep == Udeg::ZERO {
            return Err(ModelError::DegenerateArc {
                reason: "the sweep is zero",
            });
        }
        let full = sweep.get().unsigned_abs() == Udeg::FULL_TURN.get().unsigned_abs();
        if start == end && !full {
            return Err(ModelError::DegenerateArc {
                reason: "the end points coincide and the sweep is not a full turn",
            });
        }
        if start != end && full {
            return Err(ModelError::DegenerateArc {
                reason: "a full turn needs the end points to coincide",
            });
        }
        Ok(Arc {
            start,
            end,
            sweep_udeg: sweep,
        })
    }

    /// Squared length of the chord.
    pub fn chord_squared(&self) -> i128 {
        self.start.distance_squared(self.end)
    }
}

impl<'de> Deserialize<'de> for Arc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            start: Point,
            end: Point,
            sweep_udeg: Udeg,
        }
        let raw = Raw::deserialize(deserializer)?;
        Arc::new(raw.start, raw.end, raw.sweep_udeg).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn orientation_of_simple_triples() {
        assert_eq!(
            orient(p(0, 0), p(10, 0), p(0, 10)),
            Orientation::CounterClockwise
        );
        assert_eq!(orient(p(0, 0), p(10, 0), p(0, -10)), Orientation::Clockwise);
        assert_eq!(
            orient(p(0, 0), p(10, 10), p(-5, -5)),
            Orientation::Collinear
        );
        assert_eq!(orient(p(3, 3), p(3, 3), p(9, 1)), Orientation::Collinear);
    }

    #[test]
    fn orientation_at_the_extremes_does_not_overflow() {
        let m = Nm::MAX.get();
        // Largest possible differences on both axes.
        let a = p(-m, -m);
        let b = p(m, -m);
        let c = p(-m, m);
        assert_eq!(orient(a, b, c), Orientation::CounterClockwise);
        assert_eq!(orient(a, c, b), Orientation::Clockwise);
        assert_eq!(orient(a, p(m, m), p(0, 0)), Orientation::Collinear);
        // One cross term at its maximum, the other zero.
        assert_eq!(
            orient(p(-m, m), p(m, -m), p(m, m)),
            Orientation::CounterClockwise
        );
        assert_eq!(
            orient(p(-m, m), p(m, -m), p(-m, -m)),
            Orientation::Clockwise
        );
    }

    #[test]
    fn one_nanometre_off_the_line_is_not_collinear() {
        let m = Nm::MAX.get();
        assert_eq!(
            orient(p(-m, -m), p(m, m), p(0, 1)),
            Orientation::CounterClockwise
        );
        assert_eq!(orient(p(-m, -m), p(m, m), p(1, 0)), Orientation::Clockwise);
    }

    #[test]
    fn points_outside_the_range_cannot_be_built() {
        assert!(Point::from_nm(1 << 30, 0).is_err());
        assert!(Point::from_nm(0, -(1 << 30)).is_err());
    }

    #[test]
    fn segment_intersection_cases() {
        // Crossing.
        assert!(segments_intersect(p(0, 0), p(10, 10), p(0, 10), p(10, 0)));
        // Parallel apart.
        assert!(!segments_intersect(p(0, 0), p(10, 0), p(0, 1), p(10, 1)));
        // Touching at an end.
        assert!(segments_intersect(p(0, 0), p(10, 0), p(10, 0), p(10, 10)));
        // End of one on the interior of the other (T shape).
        assert!(segments_intersect(p(0, 0), p(10, 0), p(5, 0), p(5, 7)));
        // Collinear overlapping and collinear apart.
        assert!(segments_intersect(p(0, 0), p(10, 0), p(5, 0), p(15, 0)));
        assert!(!segments_intersect(p(0, 0), p(10, 0), p(11, 0), p(15, 0)));
        // Lines would cross, segments do not reach.
        assert!(!segments_intersect(p(0, 0), p(4, 4), p(0, 10), p(10, 0)));
        // Degenerate segment (a point) on and off a segment.
        assert!(segments_intersect(p(5, 5), p(5, 5), p(0, 0), p(10, 10)));
        assert!(!segments_intersect(p(5, 6), p(5, 6), p(0, 0), p(10, 10)));
        // Symmetric in its arguments.
        assert_eq!(
            segments_intersect(p(0, 0), p(10, 10), p(0, 10), p(10, 0)),
            segments_intersect(p(10, 0), p(0, 10), p(10, 10), p(0, 0))
        );
    }

    #[test]
    fn segment_intersection_at_the_extremes() {
        let m = Nm::MAX.get();
        assert!(segments_intersect(p(-m, -m), p(m, m), p(-m, m), p(m, -m)));
        assert!(segments_intersect(p(-m, -m), p(0, 0), p(-m, m), p(m, -m)));
        assert!(!segments_intersect(
            p(-m, -m),
            p(-1, -1),
            p(-m, m),
            p(m, -m)
        ));
        assert!(!segments_intersect(
            p(-m, -m),
            p(-m, m),
            p(-m + 1, -m),
            p(-m + 1, m)
        ));
    }

    #[test]
    fn rect_contains_and_intersects() {
        let r = Rect::from_corners(p(10, 10), p(0, 0));
        assert_eq!(r.min, p(0, 0));
        assert_eq!(r.max, p(10, 10));
        assert!(r.contains(p(0, 10)));
        assert!(!r.contains(p(11, 5)));
        assert!(r.intersects(&Rect::from_corners(p(10, 10), p(20, 20))));
        assert!(!r.intersects(&Rect::from_corners(p(11, 11), p(20, 20))));
    }

    #[test]
    fn distance_squared_is_exact_at_the_extremes() {
        let m = i128::from(Nm::MAX.get());
        assert_eq!(
            p(-(m as i64), -(m as i64)).distance_squared(p(m as i64, m as i64)),
            2 * (2 * m) * (2 * m)
        );
        assert_eq!(p(1, 1).distance_squared(p(1, 1)), 0);
    }

    #[test]
    fn arcs_must_have_a_shape() {
        let s = Udeg::new(90_000_000).unwrap();
        assert!(Arc::new(p(0, 0), p(10, 10), s).is_ok());
        assert!(Arc::new(p(0, 0), p(10, 10), Udeg::new(-90_000_000).unwrap()).is_ok());
        assert!(matches!(
            Arc::new(p(0, 0), p(10, 10), Udeg::ZERO),
            Err(ModelError::DegenerateArc { .. })
        ));
        assert!(Arc::new(p(5, 5), p(5, 5), s).is_err());
        assert!(Arc::new(p(5, 5), p(5, 5), Udeg::FULL_TURN).is_ok());
        assert!(Arc::new(p(5, 5), p(5, 5), Udeg::new(-360_000_000).unwrap()).is_ok());
        assert!(Arc::new(p(0, 0), p(1, 0), Udeg::FULL_TURN).is_err());
        assert_eq!(Arc::new(p(0, 0), p(3, 4), s).unwrap().chord_squared(), 25);
    }

    #[test]
    fn json_form_names_the_units_and_checks_arcs_on_input() {
        let arc = Arc::new(p(1, 2), p(3, 4), Udeg::new(180_000_000).unwrap()).unwrap();
        let text = serde_json::to_string(&arc).unwrap();
        assert_eq!(
            text,
            r#"{"start":{"x_nm":1,"y_nm":2},"end":{"x_nm":3,"y_nm":4},"sweep_udeg":180000000}"#
        );
        assert_eq!(serde_json::from_str::<Arc>(&text).unwrap(), arc);
        let bad = r#"{"start":{"x_nm":1,"y_nm":2},"end":{"x_nm":3,"y_nm":4},"sweep_udeg":0}"#;
        assert!(serde_json::from_str::<Arc>(bad).is_err());
        let out =
            r#"{"start":{"x_nm":1073741824,"y_nm":2},"end":{"x_nm":3,"y_nm":4},"sweep_udeg":5}"#;
        assert!(serde_json::from_str::<Arc>(out).is_err());
    }
}
