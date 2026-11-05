// SPDX-License-Identifier: GPL-3.0-or-later
//! The geometry that an import produces: points in integer nanometres, apertures, and the drawn
//! objects of a layer. It is a separate, read-only description for review; it is never merged into
//! the board and never written back.

use std::collections::BTreeMap;

use crate::codes::Finding;
use crate::settings::ZeroOmission;

/// A point in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct P {
    /// Horizontal coordinate.
    pub x: i64,
    /// Vertical coordinate.
    pub y: i64,
}

/// The units of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    /// Millimetres.
    Millimetres,
    /// Inches.
    Inches,
}

impl Units {
    /// Nanometres in one unit.
    pub fn nm(self) -> i64 {
        match self {
            Units::Millimetres => 1_000_000,
            Units::Inches => 25_400_000,
        }
    }

    /// The name printed in a summary.
    pub fn name(self) -> &'static str {
        match self {
            Units::Millimetres => "mm",
            Units::Inches => "in",
        }
    }
}

/// A primitive of an aperture macro, evaluated. Lengths are nanometres, angles micro-degrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroShape {
    /// A filled circle.
    Circle {
        /// True when it adds copper, false when it takes it away from the macro.
        exposure: bool,
        /// Diameter.
        diameter: i64,
        /// Centre.
        centre: P,
        /// Rotation about the origin of the macro.
        rotation: i64,
    },
    /// A line with square ends.
    VectorLine {
        /// True when it adds, false when it takes away.
        exposure: bool,
        /// Width.
        width: i64,
        /// One end.
        start: P,
        /// The other end.
        end: P,
        /// Rotation about the origin of the macro.
        rotation: i64,
    },
    /// A rectangle given by its centre.
    CenterLine {
        /// True when it adds, false when it takes away.
        exposure: bool,
        /// Width.
        width: i64,
        /// Height.
        height: i64,
        /// Centre.
        centre: P,
        /// Rotation about the origin of the macro.
        rotation: i64,
    },
    /// A closed polygon.
    Outline {
        /// True when it adds, false when it takes away.
        exposure: bool,
        /// The corners, the first point repeated at the end.
        points: Vec<P>,
        /// Rotation about the origin of the macro.
        rotation: i64,
    },
    /// A regular polygon.
    Polygon {
        /// True when it adds, false when it takes away.
        exposure: bool,
        /// The number of corners.
        vertices: u32,
        /// Centre.
        centre: P,
        /// Diameter of the circle through the corners.
        diameter: i64,
        /// Rotation about the origin of the macro.
        rotation: i64,
    },
}

/// The shape of an aperture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApertureShape {
    /// A circle.
    Circle {
        /// Diameter.
        diameter: i64,
    },
    /// A rectangle.
    Rect {
        /// Width.
        width: i64,
        /// Height.
        height: i64,
    },
    /// A rectangle with round ends.
    Obround {
        /// Width.
        width: i64,
        /// Height.
        height: i64,
    },
    /// A regular polygon.
    Polygon {
        /// Diameter of the circle through the corners.
        diameter: i64,
        /// The number of corners, 3 to 12.
        vertices: u32,
        /// Rotation in micro-degrees.
        rotation: i64,
    },
    /// A macro with its parameters put in.
    Macro {
        /// The name of the macro.
        name: String,
        /// Its primitives.
        shapes: Vec<MacroShape>,
    },
}

/// An aperture: a shape and, when it has one, a round hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aperture {
    /// The shape.
    pub shape: ApertureShape,
    /// The diameter of the hole, when there is one.
    pub hole: Option<i64>,
}

impl Aperture {
    /// The width that a line drawn with this aperture takes: the diameter of a circle, the smaller
    /// side of a rectangle, 0 for a macro.
    pub fn stroke_width(&self) -> i64 {
        match &self.shape {
            ApertureShape::Circle { diameter } => *diameter,
            ApertureShape::Rect { width, height } | ApertureShape::Obround { width, height } => {
                (*width).min(*height)
            }
            ApertureShape::Polygon { diameter, .. } => *diameter,
            ApertureShape::Macro { .. } => 0,
        }
    }

    /// True for the only shape that the Gerber format allows for a drawn line.
    pub fn is_circle(&self) -> bool {
        matches!(self.shape, ApertureShape::Circle { .. })
    }
}

/// One drawn object of a layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// An aperture put at a point.
    Flash {
        /// Where.
        at: P,
        /// The number of the aperture (`D10` is 10).
        aperture: u32,
        /// True for dark polarity, false for clear.
        dark: bool,
    },
    /// A straight line drawn with a round aperture.
    Line {
        /// Start.
        from: P,
        /// End.
        to: P,
        /// Width.
        width: i64,
        /// True for dark polarity, false for clear.
        dark: bool,
    },
    /// A circular arc drawn with a round aperture.
    Arc {
        /// Start.
        from: P,
        /// End.
        to: P,
        /// Centre.
        centre: P,
        /// True when it runs clockwise.
        clockwise: bool,
        /// Width.
        width: i64,
        /// True for dark polarity, false for clear.
        dark: bool,
    },
    /// A filled polygon (arcs already replaced by straight pieces); the last point joins the first.
    Region {
        /// The corners.
        points: Vec<P>,
        /// True for dark polarity, false for clear.
        dark: bool,
    },
}

/// A Gerber file read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GerberLayer {
    /// The units of the file.
    pub units: Units,
    /// The attributes of the file (`.FileFunction` and the like), each with its values.
    pub attributes: BTreeMap<String, Vec<String>>,
    /// The apertures by number.
    pub apertures: BTreeMap<u32, Aperture>,
    /// The drawn objects in file order.
    pub items: Vec<Item>,
    /// What was found while reading.
    pub findings: Vec<Finding>,
    /// True when the file ends with its end command.
    pub ended: bool,
    /// False when something that changes the picture was skipped.
    pub complete: bool,
}

/// A hole of an Excellon file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// The tool number.
    pub tool: u32,
    /// Where.
    pub at: P,
}

/// A slot of an Excellon file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The tool number.
    pub tool: u32,
    /// One end.
    pub from: P,
    /// The other end.
    pub to: P,
}

/// An Excellon file read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillFile {
    /// The units of the file.
    pub units: Units,
    /// The diameter of each tool, by number, in nanometres.
    pub tools: BTreeMap<u32, i64>,
    /// The round holes.
    pub hits: Vec<Hit>,
    /// The slots.
    pub slots: Vec<Slot>,
    /// The attributes found in comments (`.FileFunction`), each with its values.
    pub attributes: BTreeMap<String, Vec<String>>,
    /// What was found while reading.
    pub findings: Vec<Finding>,
    /// True when the file ends with its end command.
    pub ended: bool,
    /// Which zeros the file leaves out, as read or assumed.
    pub omission: ZeroOmission,
}

impl DrillFile {
    /// The diameter of the tool of a hole, 0 when the tool is not defined.
    pub fn diameter(&self, tool: u32) -> i64 {
        self.tools.get(&tool).copied().unwrap_or(0)
    }
}

/// The corners of the box around some points.
pub fn bounds_of(points: impl Iterator<Item = P>) -> Option<(P, P)> {
    let mut found: Option<(P, P)> = None;
    for p in points {
        found = Some(match found {
            None => (p, p),
            Some((lo, hi)) => (
                P {
                    x: lo.x.min(p.x),
                    y: lo.y.min(p.y),
                },
                P {
                    x: hi.x.max(p.x),
                    y: hi.y.max(p.y),
                },
            ),
        });
    }
    found
}

impl GerberLayer {
    /// The corners of the box around the centre lines of the objects (the width of a line and the
    /// size of an aperture are not added).
    pub fn centre_bounds(&self) -> Option<(P, P)> {
        let mut points = Vec::new();
        for item in &self.items {
            match item {
                Item::Flash { at, .. } => points.push(*at),
                Item::Line { from, to, .. } | Item::Arc { from, to, .. } => {
                    points.push(*from);
                    points.push(*to);
                }
                Item::Region {
                    points: corners, ..
                } => points.extend(corners.iter().copied()),
            }
        }
        bounds_of(points.into_iter())
    }

    /// The number of flashes.
    pub fn flashes(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, Item::Flash { .. }))
            .count()
    }

    /// The number of straight lines.
    pub fn lines(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, Item::Line { .. }))
            .count()
    }

    /// The number of arcs.
    pub fn arcs(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, Item::Arc { .. }))
            .count()
    }

    /// The number of regions.
    pub fn regions(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, Item::Region { .. }))
            .count()
    }
}

impl DrillFile {
    /// The corners of the box around the holes and the ends of the slots.
    pub fn centre_bounds(&self) -> Option<(P, P)> {
        let mut points: Vec<P> = self.hits.iter().map(|h| h.at).collect();
        for s in &self.slots {
            points.push(s.from);
            points.push(s.to);
        }
        bounds_of(points.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_and_stroke_widths() {
        assert_eq!(Units::Millimetres.nm(), 1_000_000);
        assert_eq!(Units::Inches.nm(), 25_400_000);
        let rect = Aperture {
            shape: ApertureShape::Rect {
                width: 300,
                height: 200,
            },
            hole: None,
        };
        assert_eq!(rect.stroke_width(), 200);
        assert!(!rect.is_circle());
        let circle = Aperture {
            shape: ApertureShape::Circle { diameter: 90 },
            hole: Some(10),
        };
        assert_eq!((circle.stroke_width(), circle.is_circle()), (90, true));
    }

    #[test]
    fn bounds_of_points() {
        assert_eq!(bounds_of(std::iter::empty()), None);
        let points = [P { x: 3, y: -1 }, P { x: -2, y: 5 }];
        let (lo, hi) = bounds_of(points.into_iter()).unwrap();
        assert_eq!((lo, hi), (P { x: -2, y: -1 }, P { x: 3, y: 5 }));
    }
}
