// SPDX-License-Identifier: GPL-3.0-or-later
//! Several measures of distance and angle at once, without a screen: the unit of the readings
//! (millimetres or mils), how many decimals they show, the snap of a click to the notable points of
//! the drawing (the centre of a pad, the end of a track, the corner of a shape), a list of the
//! measures that were made, the angle between two of them and the removal of one or of all.

use crate::prim::{arc_point, Bounds, Shape, P};
use crate::viewer::Viewer;

/// One mil is this many millimetres.
const MM_PER_MIL: f64 = 0.0254;

/// The unit of the readings of a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Millimetres.
    Mm,
    /// Thousandths of an inch.
    Mil,
}

impl Unit {
    /// The word that names the unit in the settings.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mil => "mil",
        }
    }

    /// The unit a word names, in any case.
    pub fn parse(text: &str) -> Option<Unit> {
        match text.trim().to_ascii_lowercase().as_str() {
            "mm" => Some(Unit::Mm),
            "mil" => Some(Unit::Mil),
            _ => None,
        }
    }

    /// The other unit.
    pub fn other(self) -> Unit {
        match self {
            Unit::Mm => Unit::Mil,
            Unit::Mil => Unit::Mm,
        }
    }

    /// A length in millimetres as a number of this unit.
    pub fn convert(self, mm: f64) -> f64 {
        match self {
            Unit::Mm => mm,
            Unit::Mil => mm / MM_PER_MIL,
        }
    }
}

/// A length in millimetres written in a unit with a number of decimals, as `12.345 mm`.
pub fn format_length(mm: f64, unit: Unit, decimals: usize) -> String {
    format!("{:.*} {}", decimals, unit.convert(mm), unit.name())
}

/// The reading of a measure from `a` to `b`: the distance, the difference along x and y and the
/// angle of the line from the positive x axis, counter-clockwise, in degrees.
pub fn describe(a: P, b: P, unit: Unit, decimals: usize) -> String {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    format!(
        "{} (dx {:.*}, dy {:.*}, {:.2}°)",
        format_length(dx.hypot(dy), unit, decimals),
        decimals,
        unit.convert(dx),
        decimals,
        unit.convert(dy),
        dy.atan2(dx).to_degrees()
    )
}

/// The measures that were made and are still shown.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeasureSet {
    done: Vec<(P, P)>,
    max: usize,
}

impl MeasureSet {
    /// An empty list that keeps at most `max` measures (at least one).
    pub fn new(max: usize) -> MeasureSet {
        MeasureSet {
            done: Vec::new(),
            max: max.max(1),
        }
    }

    /// Keeps a measure; the oldest goes when the limit is passed. Two equal points are not a
    /// measure and are not kept.
    pub fn push(&mut self, a: P, b: P) -> bool {
        if a == b {
            return false;
        }
        self.done.push((a, b));
        let limit = self.max.max(1);
        if self.done.len() > limit {
            let extra = self.done.len() - limit;
            self.done.drain(..extra);
        }
        true
    }

    /// Takes one measure out; the answer is false when there is none at that place.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.done.len() {
            self.done.remove(index);
            true
        } else {
            false
        }
    }

    /// Takes every measure out.
    pub fn clear(&mut self) {
        self.done.clear();
    }

    /// The measures, the oldest first.
    pub fn items(&self) -> &[(P, P)] {
        &self.done
    }

    /// How many measures there are.
    pub fn len(&self) -> usize {
        self.done.len()
    }

    /// True when there is none.
    pub fn is_empty(&self) -> bool {
        self.done.is_empty()
    }

    /// The angle between two measures, as two lines, in degrees from 0 to 180; none when a place
    /// does not exist.
    pub fn angle_between(&self, first: usize, second: usize) -> Option<f64> {
        let (a, b) = *self.done.get(first)?;
        let (c, d) = *self.done.get(second)?;
        let (u, v) = ((b.x - a.x, b.y - a.y), (d.x - c.x, d.y - c.y));
        let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
        if lu == 0.0 || lv == 0.0 {
            return None;
        }
        let cosine = ((u.0 * v.0 + u.1 * v.1) / (lu * lv)).clamp(-1.0, 1.0);
        Some(cosine.acos().to_degrees())
    }

    /// The angle between the last two measures.
    pub fn last_angle(&self) -> Option<f64> {
        let n = self.done.len();
        if n < 2 {
            return None;
        }
        self.angle_between(n - 2, n - 1)
    }
}

/// What a notable point of a shape is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// The centre of a circle, of an arc or of the box of a shape (the middle of a pad).
    Centre,
    /// The end of a line or of an arc.
    Endpoint,
    /// A corner of a polygon.
    Corner,
}

impl SnapKind {
    /// Which is preferred at the same distance: the centre, then an end, then a corner.
    fn rank(self) -> u8 {
        match self {
            SnapKind::Centre => 0,
            SnapKind::Endpoint => 1,
            SnapKind::Corner => 2,
        }
    }
}

/// The notable points of a shape: the centre of a circle, the centre and the ends of an arc, the
/// ends of a line, and the corners and the middle of the box of a polygon. A text has none.
pub fn notable_points(shape: &Shape) -> Vec<(P, SnapKind)> {
    match shape {
        Shape::Line { a, b } => vec![(*a, SnapKind::Endpoint), (*b, SnapKind::Endpoint)],
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => vec![
            (*centre, SnapKind::Centre),
            (arc_point(*centre, *radius, *start), SnapKind::Endpoint),
            (
                arc_point(*centre, *radius, *start + *sweep),
                SnapKind::Endpoint,
            ),
        ],
        Shape::Circle { centre, .. } => vec![(*centre, SnapKind::Centre)],
        Shape::Polygon { points } => {
            let mut area = Bounds::EMPTY;
            let mut found: Vec<(P, SnapKind)> = Vec::with_capacity(points.len() + 1);
            for p in points {
                area.include(*p);
                found.push((*p, SnapKind::Corner));
            }
            if !area.is_empty() {
                found.push((area.centre(), SnapKind::Centre));
            }
            found
        }
        Shape::Text { .. } => Vec::new(),
    }
}

/// The notable point of the visible layers that is nearest to `p` within `radius_mm`, with what it
/// is; at the same distance a centre wins over an end and an end over a corner.
pub fn snap_point(viewer: &Viewer, p: P, radius_mm: f64) -> Option<(P, SnapKind)> {
    let area = Bounds {
        min: P::new(p.x - radius_mm, p.y - radius_mm),
        max: P::new(p.x + radius_mm, p.y + radius_mm),
    };
    let mut best: Option<(f64, P, SnapKind)> = None;
    for index in viewer.visible(area) {
        let prim = &viewer.scene.prims[index as usize];
        for (q, kind) in notable_points(&prim.shape) {
            let distance = q.dist(p);
            if distance > radius_mm {
                continue;
            }
            let better = best.is_none_or(|(d, _, k)| {
                distance < d - 1e-12 || ((distance - d).abs() <= 1e-12 && kind.rank() < k.rank())
            });
            if better {
                best = Some((distance, q, kind));
            }
        }
    }
    best.map(|(_, q, kind)| (q, kind))
}

impl Viewer {
    /// A click of the measure tool at a pixel: when the snap is on, the point goes to the notable
    /// point near it; a measure that was complete is kept in the list before the click starts a new
    /// one.
    pub fn measure_click(&mut self, sx: f64, sy: f64) {
        let mut p = self.view.to_world(sx, sy);
        if self.settings.measure_snap {
            let radius = self.settings.measure_snap_px / self.view.scale;
            if let Some((snapped, _)) = snap_point(self, p, radius) {
                p = snapped;
            }
        }
        if let (Some(a), Some(b)) = (self.measure.a, self.measure.b) {
            self.measures.push(a, b);
        }
        self.measure.click(p);
    }

    /// Takes every measure out: the one in progress and the ones that were kept.
    pub fn clear_measures(&mut self) {
        self.measure.clear();
        self.measures.clear();
    }

    /// The reading of every measure in the unit of the settings: the ones that were kept and then
    /// the one that is complete now.
    pub fn readings(&self) -> Vec<String> {
        let (unit, decimals) = (self.settings.measure_unit, self.settings.measure_decimals);
        let mut lines: Vec<String> = self
            .measures
            .items()
            .iter()
            .map(|(a, b)| describe(*a, *b, unit, decimals))
            .collect();
        if let (Some(a), Some(b)) = (self.measure.a, self.measure.b) {
            lines.push(describe(a, b, unit, decimals));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use crate::scene::ItemKind;
    use crate::settings::ViewerSettings;

    fn square(x: f64, y: f64, half: f64) -> Shape {
        Shape::Polygon {
            points: vec![
                P::new(x - half, y - half),
                P::new(x + half, y - half),
                P::new(x + half, y + half),
                P::new(x - half, y + half),
            ],
        }
    }

    fn viewer() -> Viewer {
        let scene = sample::scene(
            &["GND"],
            &[
                (
                    ItemKind::Track,
                    "track",
                    Some(0),
                    Shape::Line {
                        a: P::new(0.0, 0.0),
                        b: P::new(10.0, 0.0),
                    },
                ),
                (
                    ItemKind::Via,
                    "via",
                    Some(0),
                    Shape::Circle {
                        centre: P::new(5.0, 5.0),
                        radius: 0.4,
                    },
                ),
                (ItemKind::Pad, "R1 pad 1", None, square(10.0, 10.0, 0.5)),
            ],
        );
        Viewer::new(scene, ViewerSettings::default())
    }

    #[test]
    fn the_units_convert_and_are_read_in_any_case() {
        assert_eq!(Unit::parse("MM"), Some(Unit::Mm));
        assert_eq!(Unit::parse(" mil "), Some(Unit::Mil));
        assert_eq!(Unit::parse("inch"), None);
        assert_eq!(Unit::Mm.other(), Unit::Mil);
        assert_eq!(Unit::Mil.other(), Unit::Mm);
        assert!((Unit::Mil.convert(2.54) - 100.0).abs() < 1e-9);
        assert_eq!(Unit::Mm.convert(2.54), 2.54);
        assert_eq!(format_length(2.54, Unit::Mil, 1), "100.0 mil");
        assert_eq!(format_length(1.5, Unit::Mm, 3), "1.500 mm");
        assert_eq!(format_length(1.5, Unit::Mm, 0), "2 mm");
    }

    #[test]
    fn a_reading_has_the_distance_the_differences_and_the_angle() {
        let text = describe(P::new(1.0, 1.0), P::new(4.0, 5.0), Unit::Mm, 3);
        assert_eq!(text, "5.000 mm (dx 3.000, dy 4.000, 53.13°)");
        let mils = describe(P::new(0.0, 0.0), P::new(2.54, 0.0), Unit::Mil, 1);
        assert_eq!(mils, "100.0 mil (dx 100.0, dy 0.0, 0.00°)");
    }

    #[test]
    fn several_measures_are_kept_and_one_can_be_taken_out() {
        let mut set = MeasureSet::new(3);
        assert!(set.is_empty());
        assert!(set.push(P::new(0.0, 0.0), P::new(1.0, 0.0)));
        assert!(set.push(P::new(0.0, 0.0), P::new(0.0, 2.0)));
        assert!(!set.push(P::new(5.0, 5.0), P::new(5.0, 5.0)));
        assert_eq!(set.len(), 2);
        assert!(set.push(P::new(1.0, 1.0), P::new(2.0, 2.0)));
        assert!(set.push(P::new(2.0, 2.0), P::new(3.0, 3.0)));
        assert_eq!(set.len(), 3);
        assert_eq!(set.items()[0], (P::new(0.0, 0.0), P::new(0.0, 2.0)));
        assert!(!set.remove(3));
        assert!(set.remove(0));
        assert_eq!(set.len(), 2);
        set.clear();
        assert!(set.is_empty());
        // a limit of zero still keeps one
        let mut one = MeasureSet::new(0);
        one.push(P::new(0.0, 0.0), P::new(1.0, 0.0));
        one.push(P::new(0.0, 0.0), P::new(2.0, 0.0));
        assert_eq!(one.len(), 1);
        assert_eq!(one.items()[0].1, P::new(2.0, 0.0));
    }

    #[test]
    fn the_angle_between_two_measures_is_between_zero_and_a_half_turn() {
        let mut set = MeasureSet::new(10);
        set.push(P::new(0.0, 0.0), P::new(2.0, 0.0));
        set.push(P::new(5.0, 5.0), P::new(5.0, 8.0));
        set.push(P::new(0.0, 0.0), P::new(-3.0, 0.0));
        assert!((set.angle_between(0, 1).unwrap() - 90.0).abs() < 1e-9);
        assert!((set.angle_between(0, 2).unwrap() - 180.0).abs() < 1e-9);
        assert!(set.angle_between(0, 0).unwrap().abs() < 1e-6);
        assert!((set.last_angle().unwrap() - 90.0).abs() < 1e-9);
        assert!(set.angle_between(0, 3).is_none());
        assert!(MeasureSet::new(2).last_angle().is_none());
    }

    #[test]
    fn the_notable_points_of_each_shape() {
        let line = notable_points(&Shape::Line {
            a: P::new(0.0, 0.0),
            b: P::new(1.0, 1.0),
        });
        assert_eq!(line.len(), 2);
        assert!(line.iter().all(|(_, k)| *k == SnapKind::Endpoint));
        let circle = notable_points(&Shape::Circle {
            centre: P::new(2.0, 3.0),
            radius: 1.0,
        });
        assert_eq!(circle, vec![(P::new(2.0, 3.0), SnapKind::Centre)]);
        let polygon = notable_points(&square(0.0, 0.0, 1.0));
        assert_eq!(polygon.len(), 5);
        assert_eq!(polygon[4], (P::new(0.0, 0.0), SnapKind::Centre));
        let arc = notable_points(&Shape::Arc {
            centre: P::new(0.0, 0.0),
            radius: 2.0,
            start: 0.0,
            sweep: std::f64::consts::FRAC_PI_2,
        });
        assert_eq!(arc.len(), 3);
        assert!((arc[1].0.x - 2.0).abs() < 1e-9 && arc[1].0.y.abs() < 1e-9);
        assert!(arc[2].0.x.abs() < 1e-9 && (arc[2].0.y - 2.0).abs() < 1e-9);
        let text = notable_points(&Shape::Text {
            at: P::new(0.0, 0.0),
            height: 1.0,
            text: "R1".to_string(),
        });
        assert!(text.is_empty());
    }

    #[test]
    fn a_point_goes_to_the_nearest_notable_point_within_the_radius() {
        let v = viewer();
        // near the end of the track
        let (p, kind) = snap_point(&v, P::new(10.2, 0.1), 0.5).unwrap();
        assert_eq!((p, kind), (P::new(10.0, 0.0), SnapKind::Endpoint));
        // near the centre of the via
        let (p, kind) = snap_point(&v, P::new(5.1, 4.9), 0.5).unwrap();
        assert_eq!((p, kind), (P::new(5.0, 5.0), SnapKind::Centre));
        // near a corner of the pad, and near its middle
        let (p, kind) = snap_point(&v, P::new(10.6, 10.6), 0.5).unwrap();
        assert_eq!((p, kind), (P::new(10.5, 10.5), SnapKind::Corner));
        let (p, kind) = snap_point(&v, P::new(10.1, 10.0), 0.5).unwrap();
        assert_eq!((p, kind), (P::new(10.0, 10.0), SnapKind::Centre));
        // too far from everything
        assert!(snap_point(&v, P::new(7.0, 7.0), 0.5).is_none());
    }

    #[test]
    fn a_hidden_layer_is_not_snapped_to() {
        let mut v = viewer();
        v.set_layer_visible(0, false);
        assert!(snap_point(&v, P::new(10.2, 0.1), 0.5).is_none());
    }

    #[test]
    fn the_click_of_the_measure_tool_snaps_keeps_and_starts_again() {
        let mut v = viewer();
        let pixel = |v: &Viewer, x: f64, y: f64| v.view.to_screen(P::new(x, y));
        let (sx, sy) = pixel(&v, 10.05, 0.04);
        v.measure_click(sx, sy);
        assert_eq!(v.measure.a, Some(P::new(10.0, 0.0)));
        let (sx, sy) = pixel(&v, 5.03, 5.02);
        v.measure_click(sx, sy);
        assert_eq!(v.measure.b, Some(P::new(5.0, 5.0)));
        assert!(v.measures.is_empty());
        assert_eq!(v.readings().len(), 1);
        // the third click keeps the complete measure and starts another
        let (sx, sy) = pixel(&v, 10.0, 10.0);
        v.measure_click(sx, sy);
        assert_eq!(v.measures.len(), 1);
        assert_eq!(v.measure.a, Some(P::new(10.0, 10.0)));
        assert!(v.measure.b.is_none());
        assert_eq!(v.readings().len(), 1);
        v.clear_measures();
        assert!(v.measures.is_empty() && v.measure.a.is_none());
        assert!(v.readings().is_empty());
    }

    #[test]
    fn with_the_snap_off_the_click_is_where_the_pointer_is() {
        let mut v = viewer();
        v.settings.measure_snap = false;
        let (sx, sy) = v.view.to_screen(P::new(10.05, 0.04));
        v.measure_click(sx, sy);
        let a = v.measure.a.unwrap();
        assert!((a.x - 10.05).abs() < 1e-6 && (a.y - 0.04).abs() < 1e-6);
    }

    #[test]
    fn the_readings_follow_the_unit_of_the_settings() {
        let mut v = viewer();
        v.settings.measure_unit = Unit::Mil;
        v.settings.measure_decimals = 1;
        v.settings.measure_snap = false;
        let (sx, sy) = v.view.to_screen(P::new(0.0, 0.0));
        v.measure_click(sx, sy);
        let (sx, sy) = v.view.to_screen(P::new(2.54, 0.0));
        v.measure_click(sx, sy);
        let readings = v.readings();
        assert_eq!(readings.len(), 1);
        assert!(readings[0].starts_with("100.0 mil"), "{}", readings[0]);
    }
}
