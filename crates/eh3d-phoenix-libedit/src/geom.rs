// SPDX-License-Identifier: GPL-3.0-or-later
//! The small geometry that the checks and the window of the library editor need: the shape of a
//! pad, the outline of a drawing item as a polyline, distances between a polyline and a pad, and
//! the boxes of items. Lengths are nanometres; the measures of distance are floating point
//! because a result at the exact edge is never relied on.

use crate::model::{Item, Wire};

/// A box in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The smallest x.
    pub min_x: i64,
    /// The smallest y.
    pub min_y: i64,
    /// The largest x.
    pub max_x: i64,
    /// The largest y.
    pub max_y: i64,
}

impl Bounds {
    /// A box that holds one point.
    pub fn at(x: i64, y: i64) -> Bounds {
        Bounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    /// Makes the box hold a point too.
    pub fn include(&mut self, x: i64, y: i64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// True when the point is in the box or on its edge, with `margin` added to every side.
    pub fn contains(&self, x: i64, y: i64, margin: i64) -> bool {
        x >= self.min_x - margin
            && x <= self.max_x + margin
            && y >= self.min_y - margin
            && y <= self.max_y + margin
    }

    /// The width of the box.
    pub fn width(&self) -> i64 {
        self.max_x - self.min_x
    }

    /// The height of the box.
    pub fn height(&self) -> i64 {
        self.max_y - self.min_y
    }
}

/// The mirror flag and the angle in degrees of a rotation as the file writes it (`R90`, `MR45`,
/// `SR0`, empty for none). An unreadable angle counts as zero.
pub fn rotation(rot: &str) -> (bool, f64) {
    let mirror = rot.starts_with('M');
    let digits = rot.trim_start_matches(['M', 'S', 'R']);
    let angle = digits.parse::<f64>().unwrap_or(0.0);
    (mirror, angle)
}

/// The rotation as a number of quarter turns, when it is a multiple of 90 degrees.
pub fn quarter_turns(rot: &str) -> Option<u32> {
    let (_, angle) = rotation(rot);
    let turns = angle / 90.0;
    if (turns - turns.round()).abs() > 1e-9 {
        return None;
    }
    Some((turns.round() as i64).rem_euclid(4) as u32)
}

/// The copper of a pad as a simple shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A disc.
    Circle {
        /// The centre.
        cx: f64,
        /// The centre.
        cy: f64,
        /// The radius.
        r: f64,
    },
    /// A rectangle with its sides along the axes.
    Rect {
        /// The centre.
        cx: f64,
        /// The centre.
        cy: f64,
        /// Half of the width.
        hw: f64,
        /// Half of the height.
        hh: f64,
    },
}

/// The diameter of the copper of a through-hole pad that has none given: the drill with a ring
/// of a quarter of its diameter, and at least a quarter of a millimetre, on each side.
pub fn auto_diameter(drill: i64) -> i64 {
    drill + 2 * (drill / 4).max(250_000)
}

/// The shape of the copper of a pad; `None` for the other items. A pad turned by an angle that
/// is not a multiple of 90 degrees is taken as the disc that holds it.
pub fn pad_shape(item: &Item) -> Option<Shape> {
    match item {
        Item::Pad(p) => {
            let copper = if p.diameter > 0 {
                p.diameter
            } else {
                auto_diameter(p.drill)
            };
            let d = copper as f64;
            let (cx, cy) = (p.x as f64, p.y as f64);
            Some(match p.shape {
                crate::model::PadShape::Round | crate::model::PadShape::Octagon => {
                    Shape::Circle { cx, cy, r: d / 2.0 }
                }
                crate::model::PadShape::Square => Shape::Rect {
                    cx,
                    cy,
                    hw: d / 2.0,
                    hh: d / 2.0,
                },
                crate::model::PadShape::Long | crate::model::PadShape::Offset => {
                    turned(cx, cy, d, d / 2.0, &p.rot)
                }
            })
        }
        Item::Smd(p) => Some(turned(
            p.x as f64,
            p.y as f64,
            p.dx as f64 / 2.0,
            p.dy as f64 / 2.0,
            &p.rot,
        )),
        _ => None,
    }
}

/// A rectangle of half sides `hw` and `hh` turned by `rot`.
fn turned(cx: f64, cy: f64, hw: f64, hh: f64, rot: &str) -> Shape {
    match quarter_turns(rot) {
        Some(0) | Some(2) => Shape::Rect { cx, cy, hw, hh },
        Some(_) => Shape::Rect {
            cx,
            cy,
            hw: hh,
            hh: hw,
        },
        None => Shape::Circle {
            cx,
            cy,
            r: hw.hypot(hh),
        },
    }
}

/// The points of a line or an arc, in order: two for a line, and for an arc enough to follow it
/// within about ten degrees.
pub fn wire_points(w: &Wire) -> Vec<(f64, f64)> {
    let (x1, y1, x2, y2) = (w.x1 as f64, w.y1 as f64, w.x2 as f64, w.y2 as f64);
    let line = vec![(x1, y1), (x2, y2)];
    if w.curve == 0 {
        return line;
    }
    let theta = (w.curve as f64 / 1_000_000.0).to_radians();
    let chord = (x2 - x1).hypot(y2 - y1);
    let half_sin = (theta.abs() / 2.0).sin();
    if chord == 0.0 || half_sin.abs() < 1e-9 {
        return line;
    }
    let r = chord / (2.0 * half_sin);
    let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let (ux, uy) = ((x2 - x1) / chord, (y2 - y1) / chord);
    let (nx, ny) = (-uy, ux);
    let h = r * (theta.abs() / 2.0).cos() * theta.signum();
    let (cx, cy) = (mx + nx * h, my + ny * h);
    let start = (y1 - cy).atan2(x1 - cx);
    let steps = ((theta.abs().to_degrees() / 10.0).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| {
            let a = start + theta * (i as f64) / (steps as f64);
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// The distance from a point to a segment.
pub fn point_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// True when two segments cross or touch.
fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let d1 = cross(a, b, c);
    let d2 = cross(a, b, d);
    let d3 = cross(c, d, a);
    let d4 = cross(c, d, b);
    (d1 * d2 < 0.0 && d3 * d4 < 0.0)
        || (d1 == 0.0 && on_segment(a, b, c))
        || (d2 == 0.0 && on_segment(a, b, d))
        || (d3 == 0.0 && on_segment(c, d, a))
        || (d4 == 0.0 && on_segment(c, d, b))
}

fn on_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

/// The distance from a segment to the copper of a pad; zero when they touch or cross.
pub fn segment_shape(a: (f64, f64), b: (f64, f64), shape: &Shape) -> f64 {
    match *shape {
        Shape::Circle { cx, cy, r } => (point_segment((cx, cy), a, b) - r).max(0.0),
        Shape::Rect { cx, cy, hw, hh } => {
            let inside = |p: (f64, f64)| (p.0 - cx).abs() <= hw && (p.1 - cy).abs() <= hh;
            if inside(a) || inside(b) {
                return 0.0;
            }
            let corners = [
                (cx - hw, cy - hh),
                (cx + hw, cy - hh),
                (cx + hw, cy + hh),
                (cx - hw, cy + hh),
            ];
            let mut best = f64::MAX;
            for i in 0..4 {
                let (c, d) = (corners[i], corners[(i + 1) % 4]);
                if segments_cross(a, b, c, d) {
                    return 0.0;
                }
                best = best.min(point_segment(a, c, d)).min(point_segment(b, c, d));
                best = best.min(point_segment(c, a, b)).min(point_segment(d, a, b));
            }
            best
        }
    }
}

/// The distance from a polyline to the copper of a pad.
pub fn polyline_shape(points: &[(f64, f64)], shape: &Shape) -> f64 {
    match points {
        [] => f64::MAX,
        [only] => segment_shape(*only, *only, shape),
        _ => points
            .windows(2)
            .map(|w| segment_shape(w[0], w[1], shape))
            .fold(f64::MAX, f64::min),
    }
}

/// The distance from the line work of an item to the copper of a pad, with the half of the width
/// of the line taken off; `None` for items that have no line work (texts, holes, pins). A filled
/// circle counts as a disc, a circle with a width as the polyline of its ring.
pub fn stroke_distance(item: &Item, shape: &Shape) -> Option<f64> {
    match item {
        Item::Wire(w) => {
            let d = polyline_shape(&wire_points(w), shape);
            Some((d - w.width as f64 / 2.0).max(0.0))
        }
        Item::Circle(c) => {
            let (cx, cy, r) = (c.x as f64, c.y as f64, c.radius as f64);
            if c.width == 0 {
                let d = polyline_shape(&[(cx, cy)], shape);
                return Some((d - r).max(0.0));
            }
            let ring: Vec<(f64, f64)> = (0..=48)
                .map(|i| {
                    let a = std::f64::consts::TAU * f64::from(i) / 48.0;
                    (cx + r * a.cos(), cy + r * a.sin())
                })
                .collect();
            let d = polyline_shape(&ring, shape);
            Some((d - c.width as f64 / 2.0).max(0.0))
        }
        _ => None,
    }
}

/// The box of the items that have a place (pads, line work, texts as their point, holes, pins).
pub fn items_bounds(items: &[Item]) -> Option<Bounds> {
    let mut bounds: Option<Bounds> = None;
    let mut add = |x: i64, y: i64| match bounds.as_mut() {
        Some(b) => b.include(x, y),
        None => bounds = Some(Bounds::at(x, y)),
    };
    for item in items {
        match item {
            Item::Pad(_) | Item::Smd(_) => {
                if let Some(shape) = pad_shape(item) {
                    let (cx, cy, hw, hh) = match shape {
                        Shape::Circle { cx, cy, r } => (cx, cy, r, r),
                        Shape::Rect { cx, cy, hw, hh } => (cx, cy, hw, hh),
                    };
                    add((cx - hw).floor() as i64, (cy - hh).floor() as i64);
                    add((cx + hw).ceil() as i64, (cy + hh).ceil() as i64);
                }
            }
            Item::Wire(w) => {
                for (x, y) in wire_points(w) {
                    add(x.round() as i64, y.round() as i64);
                }
            }
            Item::Rect(r) => {
                add(r.x1, r.y1);
                add(r.x2, r.y2);
            }
            Item::Circle(c) => {
                add(c.x - c.radius, c.y - c.radius);
                add(c.x + c.radius, c.y + c.radius);
            }
            Item::Text(t) => add(t.x, t.y),
            Item::Hole(h) => {
                add(h.x - h.drill / 2, h.y - h.drill / 2);
                add(h.x + h.drill / 2, h.y + h.drill / 2);
            }
            Item::Pin(p) => add(p.x, p.y),
            Item::Other(_) => {}
        }
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Circle, Pad, PadShape, Smd};

    #[test]
    fn rotations_are_read_as_the_file_writes_them() {
        assert_eq!(rotation(""), (false, 0.0));
        assert_eq!(rotation("R90"), (false, 90.0));
        assert_eq!(rotation("MR270"), (true, 270.0));
        assert_eq!(rotation("SR45.5"), (false, 45.5));
        assert_eq!(quarter_turns("R90"), Some(1));
        assert_eq!(quarter_turns("R-90"), Some(3));
        assert_eq!(quarter_turns("R180"), Some(2));
        assert_eq!(quarter_turns("R45"), None);
        assert_eq!(quarter_turns(""), Some(0));
    }

    #[test]
    fn a_point_is_measured_to_the_nearest_part_of_a_segment() {
        let (a, b) = ((0.0, 0.0), (10.0, 0.0));
        assert!((point_segment((5.0, 3.0), a, b) - 3.0).abs() < 1e-9);
        assert!((point_segment((-4.0, 3.0), a, b) - 5.0).abs() < 1e-9);
        assert!((point_segment((13.0, 4.0), a, b) - 5.0).abs() < 1e-9);
        assert!((point_segment((1.0, 1.0), a, a) - std::f64::consts::SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn a_segment_is_measured_to_a_rectangle_and_a_disc() {
        let rect = Shape::Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 1.0,
            hh: 1.0,
        };
        assert!((segment_shape((3.0, 0.0), (3.0, 5.0), &rect) - 2.0).abs() < 1e-9);
        assert_eq!(segment_shape((-5.0, 0.0), (5.0, 0.0), &rect), 0.0);
        assert_eq!(segment_shape((0.5, 0.5), (0.7, 0.7), &rect), 0.0);
        assert!(
            (segment_shape((4.0, 4.0), (5.0, 4.0), &rect) - (3.0 * std::f64::consts::SQRT_2)).abs()
                < 1e-9
        );
        let disc = Shape::Circle {
            cx: 0.0,
            cy: 0.0,
            r: 1.0,
        };
        assert!((segment_shape((3.0, -5.0), (3.0, 5.0), &disc) - 2.0).abs() < 1e-9);
        assert_eq!(segment_shape((0.0, -5.0), (0.0, 5.0), &disc), 0.0);
    }

    #[test]
    fn an_arc_follows_its_circle() {
        let w = Wire {
            x1: 1_000_000,
            y1: 0,
            x2: 0,
            y2: 1_000_000,
            curve: 90_000_000,
            width: 100_000,
            layer: 21,
            ..Wire::default()
        };
        let points = wire_points(&w);
        assert!(points.len() >= 4);
        for (x, y) in &points {
            assert!((x.hypot(*y) - 1_000_000.0).abs() < 1.0, "{x} {y}");
        }
        let first = points[0];
        let last = points[points.len() - 1];
        assert!((first.0 - 1_000_000.0).abs() < 1.0 && first.1.abs() < 1.0);
        assert!(last.0.abs() < 1.0 && (last.1 - 1_000_000.0).abs() < 1.0);
        // turned the other way the arc bulges to the other side of the chord
        let reverse = Wire {
            curve: -90_000_000,
            ..w.clone()
        };
        let back = wire_points(&reverse);
        let mid = back[back.len() / 2];
        assert!(mid.0 < 0.4e6 && mid.1 < 0.4e6, "{mid:?}");
        let mid = points[points.len() / 2];
        assert!(mid.0 < 0.8e6 && mid.1 < 0.8e6 && mid.0 > 0.5e6, "{mid:?}");
    }

    #[test]
    fn pads_become_shapes_and_turn_with_their_rotation() {
        let smd = Item::Smd(Smd {
            name: "1".into(),
            dx: 2_000_000,
            dy: 1_000_000,
            rot: "R90".into(),
            ..Smd::default()
        });
        assert_eq!(
            pad_shape(&smd),
            Some(Shape::Rect {
                cx: 0.0,
                cy: 0.0,
                hw: 500_000.0,
                hh: 1_000_000.0
            })
        );
        let odd = Item::Smd(Smd {
            dx: 3_000_000,
            dy: 4_000_000,
            rot: "R45".into(),
            ..Smd::default()
        });
        assert_eq!(
            pad_shape(&odd),
            Some(Shape::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 2_500_000.0
            })
        );
        let round = Item::Pad(Pad {
            drill: 800_000,
            diameter: 1_600_000,
            ..Pad::default()
        });
        assert_eq!(
            pad_shape(&round),
            Some(Shape::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 800_000.0
            })
        );
        let automatic = Item::Pad(Pad {
            drill: 1_000_000,
            shape: PadShape::Square,
            ..Pad::default()
        });
        // the drill and a ring of a quarter of a millimetre on each side
        assert_eq!(auto_diameter(1_000_000), 1_500_000);
        assert_eq!(
            pad_shape(&automatic),
            Some(Shape::Rect {
                cx: 0.0,
                cy: 0.0,
                hw: 750_000.0,
                hh: 750_000.0
            })
        );
        assert_eq!(pad_shape(&Item::Circle(Circle::default())), None);
    }

    #[test]
    fn line_work_is_measured_without_the_half_of_its_width() {
        let pad = Shape::Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 500_000.0,
            hh: 500_000.0,
        };
        let near = Item::Wire(Wire {
            x1: -1_000_000,
            y1: 800_000,
            x2: 1_000_000,
            y2: 800_000,
            width: 200_000,
            layer: 21,
            ..Wire::default()
        });
        // 300000 from the pad to the axis of the line, less 100000
        let d = stroke_distance(&near, &pad).unwrap();
        assert!((d - 200_000.0).abs() < 1e-6, "{d}");
        let over = Item::Wire(Wire {
            y1: 400_000,
            y2: 400_000,
            ..match near {
                Item::Wire(w) => w,
                _ => unreachable!(),
            }
        });
        assert_eq!(stroke_distance(&over, &pad), Some(0.0));
        let dot = Item::Circle(Circle {
            x: 1_000_000,
            y: 0,
            radius: 200_000,
            width: 0,
            layer: 21,
            ..Circle::default()
        });
        let d = stroke_distance(&dot, &pad).unwrap();
        assert!((d - 300_000.0).abs() < 1e-6, "{d}");
        assert_eq!(stroke_distance(&Item::Text(Default::default()), &pad), None);
    }

    #[test]
    fn boxes_hold_pads_lines_and_the_rest() {
        let items = vec![
            Item::Smd(Smd {
                x: -1_000_000,
                dx: 600_000,
                dy: 400_000,
                ..Smd::default()
            }),
            Item::Wire(Wire {
                x1: 0,
                y1: 2_000_000,
                x2: 3_000_000,
                y2: 2_000_000,
                ..Wire::default()
            }),
        ];
        let b = items_bounds(&items).unwrap();
        assert_eq!((b.min_x, b.min_y), (-1_300_000, -200_000));
        assert_eq!((b.max_x, b.max_y), (3_000_000, 2_000_000));
        assert_eq!((b.width(), b.height()), (4_300_000, 2_200_000));
        assert!(b.contains(0, 0, 0));
        assert!(!b.contains(4_000_000, 0, 0));
        assert!(b.contains(3_100_000, 0, 200_000));
        assert_eq!(items_bounds(&[]), None);
    }
}
