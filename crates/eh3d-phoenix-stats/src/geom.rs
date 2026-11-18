// SPDX-License-Identifier: GPL-3.0-or-later
//! The areas and lengths of the statistics. Coordinates are nanometres inside the range of the
//! model, so every product is exact in 128 bits; the areas of the round shapes use one floating
//! point calculation each, rounded to a whole square nanometre, so the same board always gives
//! the same numbers.

use std::f64::consts::PI;

use eh3d_phoenix_board::{Pad, PadShape, Segment};
use eh3d_phoenix_model::Point;

/// The area of a polygon in square nanometres (the shoelace formula; a polygon that crosses
/// itself gives the sum of its signed pieces). Fewer than three points enclose nothing.
pub(crate) fn polygon_area_nm2(points: &[Point]) -> i128 {
    if points.len() < 3 {
        return 0;
    }
    let mut twice: i128 = 0;
    for (i, a) in points.iter().enumerate() {
        let b = &points[(i + 1) % points.len()];
        twice += i128::from(a.x_nm.get()) * i128::from(b.y_nm.get())
            - i128::from(b.x_nm.get()) * i128::from(a.y_nm.get());
    }
    twice.abs() / 2
}

/// The area of a circle of a diameter in square nanometres, as a float.
pub(crate) fn circle_nm2(diameter_nm: i64) -> f64 {
    let r = diameter_nm as f64 / 2.0;
    PI * r * r
}

/// A float area as whole square nanometres, never below zero.
pub(crate) fn whole(area: f64) -> i128 {
    if area.is_finite() && area > 0.0 {
        area.round() as i128
    } else {
        0
    }
}

/// The copper of a pad in square nanometres: its shape less the hole when it is plated. A hole
/// that is not plated has no copper at all and is not asked for here.
pub(crate) fn pad_copper_nm2(pad: &Pad) -> i128 {
    let shape = match pad.shape {
        PadShape::Circle { diameter } => circle_nm2(diameter.get()),
        PadShape::Rect { width, height } => {
            let full = width.get() as f64 * height.get() as f64;
            match pad.corner_radius_nm() {
                Some(r) => full - (4.0 - PI) * (r as f64) * (r as f64),
                None => full,
            }
        }
        PadShape::Oblong { width, height } => {
            let full = width.get() as f64 * height.get() as f64;
            let r = width.get().min(height.get()) as f64 / 2.0;
            full - (4.0 - PI) * r * r
        }
    };
    let hole = pad.drill.map_or(0.0, |d| circle_nm2(d.get()));
    whole(shape - hole)
}

/// The length of a piece of track in nanometres: a straight piece to the nearest nanometre, an
/// arc from its chord and its sweep.
pub(crate) fn segment_length_nm(segment: Segment) -> i64 {
    match segment {
        Segment::Line { start, end } => chord_nm(start, end),
        Segment::Arc { start, end, sweep } => {
            let chord = chord_nm(start, end);
            let theta = (sweep.get().unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
            let sin_half = (theta / 2.0).sin();
            if chord == 0 || !theta.is_finite() || theta <= 1e-9 || sin_half <= 1e-6 {
                return chord;
            }
            let radius = chord as f64 / (2.0 * sin_half);
            (radius * theta).round() as i64
        }
    }
}

fn chord_nm(a: Point, b: Point) -> i64 {
    let dx = i128::from(b.x_nm.get()) - i128::from(a.x_nm.get());
    let dy = i128::from(b.y_nm.get()) - i128::from(a.y_nm.get());
    ((dx * dx + dy * dy) as f64).sqrt().round() as i64
}

/// Square nanometres as whole square micrometres, saturating.
pub(crate) fn to_um2(nm2: i128) -> i64 {
    i64::try_from(nm2.max(0) / 1_000_000).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::{Nm, Udeg};

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn pad(shape: PadShape, drill: Option<i64>) -> Pad {
        Pad {
            number: "1".into(),
            shape,
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: drill.map(|d| Nm::new(d).unwrap()),
            layers: Vec::new(),
            roundness: 0,
            plated: true,
        }
    }

    #[test]
    fn a_polygon_has_its_exact_area_whatever_the_direction() {
        let square = [pt(0, 0), pt(4000, 0), pt(4000, 3000), pt(0, 3000)];
        assert_eq!(polygon_area_nm2(&square), 12_000_000);
        let reversed = [pt(0, 3000), pt(4000, 3000), pt(4000, 0), pt(0, 0)];
        assert_eq!(polygon_area_nm2(&reversed), 12_000_000);
        assert_eq!(polygon_area_nm2(&square[..2]), 0);
        assert_eq!(polygon_area_nm2(&[]), 0);
        let triangle = [pt(0, 0), pt(10, 0), pt(0, 10)];
        assert_eq!(polygon_area_nm2(&triangle), 50);
    }

    #[test]
    fn the_copper_of_a_pad_follows_its_shape_and_its_hole() {
        let rect = pad(
            PadShape::Rect {
                width: Nm::new(2_000_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            None,
        );
        assert_eq!(pad_copper_nm2(&rect), 2_000_000_000_000);
        let round = pad(
            PadShape::Circle {
                diameter: Nm::new(1_000_000).unwrap(),
            },
            None,
        );
        // pi * 0.5^2 mm2 = 0.785398 mm2
        assert_eq!(to_um2(pad_copper_nm2(&round)), 785_398);
        let drilled = pad(
            PadShape::Circle {
                diameter: Nm::new(1_000_000).unwrap(),
            },
            Some(600_000),
        );
        // pi * (0.5^2 - 0.3^2) = 0.502654 mm2
        assert_eq!(to_um2(pad_copper_nm2(&drilled)), 502_654);
        let oblong = pad(
            PadShape::Oblong {
                width: Nm::new(2_000_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            None,
        );
        // 2 mm2 less the four corners of a 0.5 mm radius: 2 - (4 - pi) * 0.25 = 1.785398
        assert_eq!(to_um2(pad_copper_nm2(&oblong)), 1_785_398);
    }

    #[test]
    fn rounded_corners_take_copper_off_a_rectangle() {
        let mut rect = pad(
            PadShape::Rect {
                width: Nm::new(1_000_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            None,
        );
        rect.roundness = 1000;
        // fully round ends: a circle of 1 mm
        assert_eq!(to_um2(pad_copper_nm2(&rect)), 785_398);
    }

    #[test]
    fn a_piece_of_track_has_a_length() {
        let line = Segment::Line {
            start: pt(0, 0),
            end: pt(3_000_000, 4_000_000),
        };
        assert_eq!(segment_length_nm(line), 5_000_000);
        // a half circle of diameter 2 mm: pi mm
        let arc = Segment::Arc {
            start: pt(0, 0),
            end: pt(2_000_000, 0),
            sweep: Udeg::new(180_000_000).unwrap(),
        };
        assert_eq!(segment_length_nm(arc), 3_141_593);
        let flat = Segment::Arc {
            start: pt(0, 0),
            end: pt(2_000_000, 0),
            sweep: Udeg::ZERO,
        };
        assert_eq!(segment_length_nm(flat), 2_000_000);
    }

    #[test]
    fn the_conversions_saturate_and_never_go_below_zero() {
        assert_eq!(whole(-3.0), 0);
        assert_eq!(whole(f64::NAN), 0);
        assert_eq!(whole(2.5), 3);
        assert_eq!(to_um2(-5), 0);
        assert_eq!(to_um2(i128::MAX), i64::MAX);
    }
}
