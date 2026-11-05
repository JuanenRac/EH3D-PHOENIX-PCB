// SPDX-License-Identifier: GPL-3.0-or-later
//! The measure of the distance between two points: the first click sets the start, the second the
//! end and the third starts again.

use crate::prim::P;

/// A measure in progress or done.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Measure {
    /// The first point.
    pub a: Option<P>,
    /// The second point.
    pub b: Option<P>,
}

impl Measure {
    /// A click at a point of the drawing.
    pub fn click(&mut self, p: P) {
        match (self.a, self.b) {
            (None, _) | (Some(_), Some(_)) => {
                self.a = Some(p);
                self.b = None;
            }
            (Some(_), None) => self.b = Some(p),
        }
    }

    /// Forgets both points.
    pub fn clear(&mut self) {
        self.a = None;
        self.b = None;
    }

    /// True when both points are set.
    pub fn is_complete(&self) -> bool {
        self.a.is_some() && self.b.is_some()
    }

    /// The difference in x and y from the first point to the second, in millimetres.
    pub fn delta(&self) -> Option<(f64, f64)> {
        let (a, b) = (self.a?, self.b?);
        Some((b.x - a.x, b.y - a.y))
    }

    /// The distance between the points in millimetres.
    pub fn distance(&self) -> Option<f64> {
        self.delta().map(|(dx, dy)| dx.hypot(dy))
    }

    /// The angle of the line from the first point to the second in degrees, from the positive x
    /// axis, counter-clockwise, in the range (-180, 180].
    pub fn angle_deg(&self) -> Option<f64> {
        self.delta().map(|(dx, dy)| dy.atan2(dx).to_degrees())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_clicks_measure_and_the_third_starts_again() {
        let mut m = Measure::default();
        assert!(m.distance().is_none());
        m.click(P::new(1.0, 1.0));
        assert!(!m.is_complete());
        assert!(m.distance().is_none());
        m.click(P::new(4.0, 5.0));
        assert!(m.is_complete());
        assert!((m.distance().unwrap() - 5.0).abs() < 1e-12);
        assert_eq!(m.delta(), Some((3.0, 4.0)));
        m.click(P::new(9.0, 9.0));
        assert_eq!(m.a, Some(P::new(9.0, 9.0)));
        assert!(m.b.is_none());
        m.clear();
        assert_eq!(m, Measure::default());
    }

    #[test]
    fn the_angle_follows_the_mathematical_direction() {
        let mut m = Measure::default();
        m.click(P::new(0.0, 0.0));
        m.click(P::new(0.0, 2.0));
        assert!((m.angle_deg().unwrap() - 90.0).abs() < 1e-9);
        m.click(P::new(0.0, 0.0));
        m.click(P::new(-1.0, 0.0));
        assert!((m.angle_deg().unwrap() - 180.0).abs() < 1e-9);
        m.click(P::new(0.0, 0.0));
        m.click(P::new(0.0, -1.0));
        assert!((m.angle_deg().unwrap() + 90.0).abs() < 1e-9);
    }
}
