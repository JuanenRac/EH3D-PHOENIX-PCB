// SPDX-License-Identifier: GPL-3.0-or-later
//! Arcs of a board: the circle through three points (KiCad 6 and later write an arc as its start,
//! a point in the middle and its end), the arc of a centre, a start and an angle (the older
//! `gr_arc`), the signed sweep in micro-degrees that the model stores, and the points of an arc
//! cut into straight pieces for the board outline.
//!
//! The model's y axis points up, so the points are turned before the angles are taken; a positive
//! sweep is counter-clockwise. The arithmetic is in floating point and every result is rounded
//! once, to a whole nanometre or a whole micro-degree.

use std::f64::consts::TAU;

use eh3d_phoenix_model::{Point, Udeg};

use crate::conv::P;

/// An arc as a circle and a sweep, in the orientation of the model.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ArcGeom {
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    start_angle: f64,
    sweep: f64,
}

fn turned(p: P) -> (f64, f64) {
    (p.0 as f64, -(p.1 as f64))
}

/// A point of the model from coordinates in nanometres, `None` when it is outside the range.
pub(crate) fn point_from(x: f64, y: f64) -> Option<Point> {
    if !x.is_finite() || !y.is_finite() || x.abs() > 4e9 || y.abs() > 4e9 {
        return None;
    }
    Point::from_nm(x.round() as i64, y.round() as i64).ok()
}

impl ArcGeom {
    /// The arc that starts at `start`, passes through `mid` and ends at `end` (positions of the
    /// file, y down). `None` when the three points lie on a line or coincide.
    pub fn through(start: P, mid: P, end: P) -> Option<ArcGeom> {
        let (ax, ay) = turned(start);
        let (bx, by) = turned(mid);
        let (cx, cy) = turned(end);
        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        let scale = (bx - ax).hypot(by - ay) * (cx - ax).hypot(cy - ay);
        if !d.is_finite() || d.abs() <= 1e-9 * scale || scale == 0.0 {
            return None;
        }
        let a2 = ax * ax + ay * ay;
        let b2 = bx * bx + by * by;
        let c2 = cx * cx + cy * cy;
        let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
        let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
        let radius = (ax - ux).hypot(ay - uy);
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let a0 = (ay - uy).atan2(ax - ux);
        let am = (by - uy).atan2(bx - ux);
        let a1 = (cy - uy).atan2(cx - ux);
        let ccw = (a1 - a0).rem_euclid(TAU);
        let mid_offset = (am - a0).rem_euclid(TAU);
        let sweep = if mid_offset < ccw { ccw } else { ccw - TAU };
        if sweep == 0.0 {
            return None;
        }
        Some(ArcGeom {
            centre_x: ux,
            centre_y: uy,
            radius,
            start_angle: a0,
            sweep,
        })
    }

    /// The arc round `centre` that starts at `start` and turns by `sweep_udeg` micro-degrees
    /// (positive is counter-clockwise), as the older `gr_arc` says it.
    pub fn from_centre(centre: P, start: P, sweep_udeg: i64) -> Option<ArcGeom> {
        let (cx, cy) = turned(centre);
        let (sx, sy) = turned(start);
        let radius = (sx - cx).hypot(sy - cy);
        if !radius.is_finite() || radius <= 0.0 || sweep_udeg == 0 {
            return None;
        }
        Some(ArcGeom {
            centre_x: cx,
            centre_y: cy,
            radius,
            start_angle: (sy - cy).atan2(sx - cx),
            sweep: (sweep_udeg as f64 / 1e6).to_radians(),
        })
    }

    /// The point at the fraction `t` of the sweep, `0.0` the start and `1.0` the end.
    pub fn point_at(&self, t: f64) -> Option<Point> {
        let angle = self.start_angle + self.sweep * t;
        point_from(
            self.centre_x + self.radius * angle.cos(),
            self.centre_y + self.radius * angle.sin(),
        )
    }

    /// The sweep in micro-degrees, rounded.
    pub fn sweep_udeg(&self) -> Option<Udeg> {
        let value = (self.sweep.to_degrees() * 1e6).round();
        if !value.is_finite() || value.abs() > 360_000_000.0 {
            return None;
        }
        Udeg::new(value as i64).ok()
    }

    /// The points between the start and the end when the arc is cut into straight pieces, with
    /// `steps_per_quarter` pieces for a quarter turn (at least one piece for the whole arc).
    pub fn inner_points(&self, steps_per_quarter: usize) -> Vec<Point> {
        let quarter = TAU / 4.0;
        let pieces = ((self.sweep.abs() / quarter) * steps_per_quarter as f64)
            .ceil()
            .max(1.0) as usize;
        (1..pieces)
            .filter_map(|k| self.point_at(k as f64 / pieces as f64))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    #[test]
    fn a_clockwise_quarter_circle_has_a_negative_sweep() {
        // the file's y axis points down: from the top of the circle to its right side
        let centre = (30 * MM, 25 * MM);
        let start = (30 * MM, 20 * MM);
        let mid = (33_535_534, 21_464_466);
        let end = (35 * MM, 25 * MM);
        let arc = ArcGeom::through(start, mid, end).unwrap();
        let sweep = arc.sweep_udeg().unwrap().get();
        assert!((sweep + 90_000_000).abs() < 1000, "{sweep}");
        assert!((arc.radius - 5.0 * MM as f64).abs() < 10.0);
        assert!((arc.centre_x - centre.0 as f64).abs() < 10.0);
        assert!((arc.centre_y + centre.1 as f64).abs() < 10.0);
        let end_point = arc.point_at(1.0).unwrap();
        assert!((end_point.x_nm.get() - end.0).abs() < 10);
        assert!((end_point.y_nm.get() + end.1).abs() < 10);
    }

    #[test]
    fn a_counter_clockwise_half_circle_has_a_positive_sweep() {
        let arc = ArcGeom::through((10 * MM, 0), (0, -10 * MM), (-10 * MM, 0)).unwrap();
        let sweep = arc.sweep_udeg().unwrap().get();
        assert!((sweep - 180_000_000).abs() < 1000, "{sweep}");
        let other = ArcGeom::through((10 * MM, 0), (0, 10 * MM), (-10 * MM, 0)).unwrap();
        assert!((other.sweep_udeg().unwrap().get() + 180_000_000).abs() < 1000);
    }

    #[test]
    fn three_points_on_a_line_or_equal_points_are_no_arc() {
        assert!(ArcGeom::through((0, 0), (MM, 0), (2 * MM, 0)).is_none());
        assert!(ArcGeom::through((0, 0), (0, 0), (0, 0)).is_none());
        assert!(ArcGeom::through((0, 0), (MM, MM), (0, 0)).is_none());
    }

    #[test]
    fn the_older_arc_turns_counter_clockwise_for_a_positive_angle() {
        // centre at the origin, start on the right, a quarter turn: the end is at the top of the
        // model (the bottom of the file's y axis is not involved)
        let arc = ArcGeom::from_centre((0, 0), (10 * MM, 0), 90_000_000).unwrap();
        let end = arc.point_at(1.0).unwrap();
        assert_eq!((end.x_nm.get(), end.y_nm.get()), (0, 10 * MM));
        assert_eq!(arc.sweep_udeg().unwrap().get(), 90_000_000);
        assert!(ArcGeom::from_centre((0, 0), (0, 0), 90_000_000).is_none());
        assert!(ArcGeom::from_centre((0, 0), (MM, 0), 0).is_none());
    }

    #[test]
    fn the_inner_points_of_an_arc_lie_on_it() {
        let arc = ArcGeom::from_centre((0, 0), (10 * MM, 0), 180_000_000).unwrap();
        let inner = arc.inner_points(8);
        assert_eq!(inner.len(), 15);
        for p in &inner {
            let r = ((p.x_nm.get() as f64).powi(2) + (p.y_nm.get() as f64).powi(2)).sqrt();
            assert!((r - 10.0 * MM as f64).abs() < 2.0, "{r}");
        }
        assert!(arc.inner_points(1).len() == 1);
    }

    #[test]
    fn points_outside_the_range_are_refused() {
        assert!(point_from(1e12, 0.0).is_none());
        assert!(point_from(f64::NAN, 0.0).is_none());
        assert!(point_from(2e9, 0.0).is_none());
        assert!(point_from(1e6, -1e6).is_some());
    }
}
