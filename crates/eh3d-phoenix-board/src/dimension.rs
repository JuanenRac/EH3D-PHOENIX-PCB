// SPDX-License-Identifier: GPL-3.0-or-later
//! Dimensions: the distance between two points of the board, drawn as a dimension line with
//! extension lines, two arrow heads and the measure as text, on a documentation layer.
//!
//! A dimension is an addition to the board and has no effect on a board that has none: the map is
//! not written when it is empty and a file made before it existed reads as before. The measure is
//! not stored: it follows from the two points and the kind, so it cannot disagree with them.
//!
//! * `aligned`: the length of the straight line from `from` to `to`; the dimension line runs
//!   parallel to it, `offset` away to the left of the direction from `from` to `to` (to the right
//!   when negative);
//! * `horizontal`: the distance along x; the dimension line is horizontal, at `from.y + offset`;
//! * `vertical`: the distance along y; the dimension line is vertical, at `from.x + offset`.
//!
//! The drawing is made of integer nanometre pieces: every division is rounded half away from zero,
//! so the same dimension always gives the same bytes on every system. The text is horizontal when
//! the dimension line is nearer to horizontal than to vertical and runs bottom to top otherwise,
//! centred over the dimension line, in the stroke font of the silkscreen.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::model::{Board, Id, LayerKind, Segment};
use crate::silk::SilkItem;
use crate::validate::{BAD_LAYER, BAD_SIZE, DANGLING};

/// What a dimension measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionKind {
    /// The length of the line from the first point to the second.
    Aligned,
    /// The distance along x.
    Horizontal,
    /// The distance along y.
    Vertical,
}

impl DimensionKind {
    /// The word of the file formats and the messages.
    pub fn as_str(self) -> &'static str {
        match self {
            DimensionKind::Aligned => "aligned",
            DimensionKind::Horizontal => "horizontal",
            DimensionKind::Vertical => "vertical",
        }
    }
}

/// One dimension of the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    /// The documentation layer it is drawn on.
    pub layer: Id,
    /// What it measures.
    pub kind: DimensionKind,
    /// The first point measured.
    pub from: Point,
    /// The second point measured.
    pub to: Point,
    /// How far the dimension line is from the measured points (see the module).
    pub offset: Nm,
    /// Height of the text, which is also the length of the arrow heads.
    pub text_height: Nm,
    /// Width of the lines and of the strokes of the text.
    pub stroke: Nm,
}

/// Why a dimension cannot be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionError {
    /// The two points give a measure of zero.
    Empty,
    /// A size is not positive.
    Size,
    /// A point of the drawing is outside the coordinate range.
    Range,
}

/// A place in whole nanometres, wide enough for every step of the arithmetic.
type Spot = (i128, i128);

fn div_round(n: i128, d: i128) -> i128 {
    // d is positive
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

fn isqrt_round(v: u128) -> u128 {
    let s = v.isqrt();
    // the nearer of s and s + 1
    if (s + 1) * (s + 1) - v <= v - s * s {
        s + 1
    } else {
        s
    }
}

fn point(x: i128, y: i128) -> Result<Point, DimensionError> {
    let x = i64::try_from(x).map_err(|_| DimensionError::Range)?;
    let y = i64::try_from(y).map_err(|_| DimensionError::Range)?;
    Point::from_nm(x, y).map_err(|_| DimensionError::Range)
}

impl Dimension {
    /// The measure in nanometres, rounded to the nearest nanometre.
    pub fn length_nm(&self) -> i64 {
        let dx = i128::from(self.to.x_nm.get()) - i128::from(self.from.x_nm.get());
        let dy = i128::from(self.to.y_nm.get()) - i128::from(self.from.y_nm.get());
        let length = match self.kind {
            DimensionKind::Horizontal => dx.unsigned_abs(),
            DimensionKind::Vertical => dy.unsigned_abs(),
            DimensionKind::Aligned => isqrt_round((dx * dx + dy * dy).unsigned_abs()),
        };
        i64::try_from(length).unwrap_or(i64::MAX)
    }

    /// The measure as text in millimetres, rounded to the micrometre, with the zeros at the end
    /// of the decimals left out (`12.5`, `3`, `0.025`).
    pub fn measure_text(&self) -> String {
        let um = (self.length_nm().unsigned_abs() + 500) / 1000;
        let (whole, frac) = (um / 1000, um % 1000);
        if frac == 0 {
            whole.to_string()
        } else {
            let frac = format!("{frac:03}");
            format!("{whole}.{}", frac.trim_end_matches('0'))
        }
    }

    /// A point of the dimension line (the end next to `from`), for the file formats that give the
    /// line by a point it passes through.
    pub fn line_point(&self) -> Result<Point, DimensionError> {
        let (a, _) = self.line_ends()?;
        point(a.0, a.1)
    }

    /// The offset that puts the dimension line of a dimension of `kind` between `from` and `to`
    /// through `through`, rounded to the nearest nanometre: the inverse of
    /// [`Dimension::line_point`] for the formats that give the line by a point. `None` when the
    /// points measure nothing or the offset is outside the range.
    pub fn offset_through(
        kind: DimensionKind,
        from: Point,
        to: Point,
        through: Point,
    ) -> Option<Nm> {
        let (ax, ay) = (i128::from(from.x_nm.get()), i128::from(from.y_nm.get()));
        let (bx, by) = (i128::from(to.x_nm.get()), i128::from(to.y_nm.get()));
        let (px, py) = (
            i128::from(through.x_nm.get()),
            i128::from(through.y_nm.get()),
        );
        let offset = match kind {
            DimensionKind::Horizontal => py - ay,
            DimensionKind::Vertical => px - ax,
            DimensionKind::Aligned => {
                let (dx, dy) = (bx - ax, by - ay);
                let len = i128::try_from(isqrt_round((dx * dx + dy * dy).unsigned_abs())).ok()?;
                if len == 0 {
                    return None;
                }
                div_round((px - ax) * -dy + (py - ay) * dx, len)
            }
        };
        Nm::new(i64::try_from(offset).ok()?).ok()
    }

    /// The two ends of the dimension line.
    fn line_ends(&self) -> Result<(Spot, Spot), DimensionError> {
        let (ax, ay) = (
            i128::from(self.from.x_nm.get()),
            i128::from(self.from.y_nm.get()),
        );
        let (bx, by) = (
            i128::from(self.to.x_nm.get()),
            i128::from(self.to.y_nm.get()),
        );
        let d = i128::from(self.offset.get());
        match self.kind {
            DimensionKind::Horizontal => Ok(((ax, ay + d), (bx, ay + d))),
            DimensionKind::Vertical => Ok(((ax + d, ay), (ax + d, by))),
            DimensionKind::Aligned => {
                let (dx, dy) = (bx - ax, by - ay);
                let len = i128::try_from(isqrt_round((dx * dx + dy * dy).unsigned_abs()))
                    .map_err(|_| DimensionError::Range)?;
                if len == 0 {
                    return Err(DimensionError::Empty);
                }
                // the left normal times the offset
                let (nx, ny) = (div_round(-dy * d, len), div_round(dx * d, len));
                Ok(((ax + nx, ay + ny), (bx + nx, by + ny)))
            }
        }
    }

    /// The drawing of the dimension as items of the silkscreen font and strokes (extension lines,
    /// dimension line, arrow heads and text), all on the layer of the dimension, in a fixed order.
    pub fn items(&self) -> Result<Vec<SilkItem>, DimensionError> {
        if self.text_height.get() <= 0 || self.stroke.get() <= 0 {
            return Err(DimensionError::Size);
        }
        if self.length_nm() == 0 {
            return Err(DimensionError::Empty);
        }
        let (a, b) = self.line_ends()?;
        let from = (
            i128::from(self.from.x_nm.get()),
            i128::from(self.from.y_nm.get()),
        );
        let to = (
            i128::from(self.to.x_nm.get()),
            i128::from(self.to.y_nm.get()),
        );
        let mut lines: Vec<(Spot, Spot)> = Vec::new();
        // the extension lines, from the measured point to the dimension line
        if from != a {
            lines.push((from, a));
        }
        if to != b {
            lines.push((to, b));
        }
        // the dimension line and the arrow heads
        let (ux, uy) = (b.0 - a.0, b.1 - a.1);
        let line_len = i128::try_from(isqrt_round((ux * ux + uy * uy).unsigned_abs()))
            .map_err(|_| DimensionError::Range)?;
        if line_len > 0 {
            lines.push((a, b));
            let arrow = i128::from(self.text_height.get());
            // the unit vector along the line and its left normal, scaled to the arrow length
            let (px, py) = (
                div_round(ux * arrow, line_len),
                div_round(uy * arrow, line_len),
            );
            let (qx, qy) = (-py, px);
            let wing = |tip: Spot, sign: i128| {
                // from the tip back along the line by the arrow, and a third of it to each side
                let back = (tip.0 - sign * px, tip.1 - sign * py);
                [
                    (tip, (back.0 + div_round(qx, 3), back.1 + div_round(qy, 3))),
                    (tip, (back.0 - div_round(qx, 3), back.1 - div_round(qy, 3))),
                ]
            };
            lines.extend(wing(a, -1));
            lines.extend(wing(b, 1));
        }
        let mut items = Vec::with_capacity(lines.len() + 1);
        for (s, e) in lines {
            items.push(SilkItem::Stroke {
                layer: self.layer,
                width: self.stroke,
                path: vec![Segment::Line {
                    start: point(s.0, s.1)?,
                    end: point(e.0, e.1)?,
                }],
            });
        }
        // the text, centred over the dimension line
        let text = self.measure_text();
        let height = i128::from(self.text_height.get());
        let units = 5 * i128::try_from(text.chars().count()).unwrap_or(0) - 1;
        let width = div_round(units * height, 6);
        let mid = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
        let gap = i128::from(self.stroke.get()) * 2;
        let horizontal = ux.abs() >= uy.abs();
        let (at, rotation) = if horizontal {
            (point(mid.0 - width / 2, mid.1 + gap)?, Udeg::ZERO)
        } else {
            (
                point(mid.0 - gap, mid.1 - width / 2)?,
                Udeg::new(90_000_000).map_err(|_| DimensionError::Range)?,
            )
        };
        items.push(SilkItem::Text {
            layer: self.layer,
            at,
            text,
            height: self.text_height,
            stroke: self.stroke,
            rotation,
            mirrored: false,
        });
        Ok(items)
    }
}

fn make(code: &'static str, detail: String) -> Diagnostic {
    let title = crate::validate::REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, Severity::Error, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Geometric)
        .with_recoverability(Recoverability::Unrecoverable)
}

impl Board {
    /// The problems of the dimensions, in a fixed order (by id). Nothing when there are none.
    pub fn dimension_problems(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for (id, dimension) in &self.dimensions {
            let what = format!("dimension {id}");
            match self.layers.get(&dimension.layer) {
                None => out.push(make(
                    DANGLING,
                    format!("{what} names a layer that does not exist"),
                )),
                Some(l) if l.kind != LayerKind::Documentation => out.push(make(
                    BAD_LAYER,
                    format!("{what} is on a layer that is not a documentation layer"),
                )),
                Some(_) => {}
            }
            match dimension.items() {
                Ok(_) => {}
                Err(DimensionError::Empty) => {
                    out.push(make(BAD_SIZE, format!("{what} measures nothing")))
                }
                Err(DimensionError::Size) => out.push(make(
                    BAD_SIZE,
                    format!("text height or line width of {what}"),
                )),
                Err(DimensionError::Range) => out.push(make(
                    BAD_SIZE,
                    format!("{what} does not fit the coordinate range"),
                )),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::ObjectId;

    const MM: i64 = 1_000_000;

    fn layer() -> Id {
        ObjectId::from_parts(1_700_000_000_000, 1, 1).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn dim(kind: DimensionKind, from: Point, to: Point, offset: i64) -> Dimension {
        Dimension {
            layer: layer(),
            kind,
            from,
            to,
            offset: Nm::new(offset).unwrap(),
            text_height: Nm::new(MM).unwrap(),
            stroke: Nm::new(100_000).unwrap(),
        }
    }

    fn line(item: &SilkItem) -> (Point, Point) {
        match item {
            SilkItem::Stroke { path, .. } => match path[0] {
                Segment::Line { start, end } => (start, end),
                Segment::Arc { .. } => panic!("a line"),
            },
            SilkItem::Text { .. } => panic!("a stroke"),
        }
    }

    #[test]
    fn the_measure_is_worked_out_from_the_points_and_the_kind() {
        let aligned = dim(DimensionKind::Aligned, pt(0, 0), pt(3 * MM, 4 * MM), MM);
        assert_eq!(aligned.length_nm(), 5 * MM);
        assert_eq!(aligned.measure_text(), "5");
        let horizontal = dim(DimensionKind::Horizontal, pt(0, 0), pt(3 * MM, 4 * MM), MM);
        assert_eq!(horizontal.length_nm(), 3 * MM);
        let vertical = dim(DimensionKind::Vertical, pt(MM, 9 * MM), pt(0, 0), MM);
        assert_eq!(vertical.length_nm(), 9 * MM);
        // the order of the points does not change the measure
        let back = dim(DimensionKind::Aligned, pt(3 * MM, 4 * MM), pt(0, 0), MM);
        assert_eq!(back.length_nm(), 5 * MM);
        // a length that is not a whole number of nanometres is rounded to the nearest
        let slant = dim(DimensionKind::Aligned, pt(0, 0), pt(1, 1), 0);
        assert_eq!(slant.length_nm(), 1);
    }

    #[test]
    fn the_text_has_the_decimals_that_are_needed_up_to_the_micrometre() {
        let at = |nm: i64| dim(DimensionKind::Horizontal, pt(0, 0), pt(nm, 0), 0);
        assert_eq!(at(12_500_000).measure_text(), "12.5");
        assert_eq!(at(12_000_400).measure_text(), "12");
        assert_eq!(at(12_345_678).measure_text(), "12.346");
        assert_eq!(at(25_000).measure_text(), "0.025");
        assert_eq!(at(100_000_000).measure_text(), "100");
        assert_eq!(at(-7_000_000).measure_text(), "7");
    }

    #[test]
    fn a_horizontal_dimension_is_drawn_with_the_numbers_worked_out_by_hand() {
        let d = dim(DimensionKind::Horizontal, pt(0, 0), pt(10 * MM, 0), 2 * MM);
        let items = d.items().unwrap();
        // two extension lines, the dimension line, four lines of arrow heads and the text
        assert_eq!(items.len(), 8);
        assert_eq!(line(&items[0]), (pt(0, 0), pt(0, 2 * MM)));
        assert_eq!(line(&items[1]), (pt(10 * MM, 0), pt(10 * MM, 2 * MM)));
        assert_eq!(line(&items[2]), (pt(0, 2 * MM), pt(10 * MM, 2 * MM)));
        // an arrow head is as long as the text is high and a third of it wide on each side
        assert_eq!(line(&items[3]), (pt(0, 2 * MM), pt(MM, 2 * MM + 333_333)));
        assert_eq!(line(&items[4]), (pt(0, 2 * MM), pt(MM, 2 * MM - 333_333)));
        assert_eq!(
            line(&items[5]),
            (pt(10 * MM, 2 * MM), pt(9 * MM, 2 * MM + 333_333))
        );
        let SilkItem::Text {
            at, text, rotation, ..
        } = &items[7]
        else {
            panic!("the text");
        };
        // "10" is 9 units wide, a unit being a sixth of the height; centred on 5 mm, 0.2 mm above
        assert_eq!(text, "10");
        assert_eq!(*at, pt(5 * MM - 750_000, 2 * MM + 200_000));
        assert_eq!(rotation.get(), 0);
    }

    #[test]
    fn an_aligned_dimension_is_offset_to_the_left_of_its_direction() {
        let d = dim(DimensionKind::Aligned, pt(0, 0), pt(3 * MM, 4 * MM), MM);
        let items = d.items().unwrap();
        // the left normal of (3, 4) is (-4, 3), so the line is 0.8 mm left and 0.6 mm up
        assert_eq!(
            line(&items[2]),
            (pt(-800_000, 600_000), pt(2_200_000, 4_600_000))
        );
        let right = dim(DimensionKind::Aligned, pt(0, 0), pt(3 * MM, 4 * MM), -MM);
        assert_eq!(
            line(&right.items().unwrap()[2]),
            (pt(800_000, -600_000), pt(3_800_000, 3_400_000))
        );
    }

    #[test]
    fn a_vertical_dimension_has_its_text_turned_and_a_zero_offset_has_no_extension_lines() {
        let d = dim(DimensionKind::Vertical, pt(0, 0), pt(0, 7_500_000), -MM);
        let items = d.items().unwrap();
        assert_eq!(line(&items[2]), (pt(-MM, 0), pt(-MM, 7_500_000)));
        let SilkItem::Text { rotation, text, .. } = items.last().unwrap() else {
            panic!("the text");
        };
        assert_eq!((text.as_str(), rotation.get()), ("7.5", 90_000_000));
        let flat = dim(DimensionKind::Horizontal, pt(0, 0), pt(MM, 0), 0);
        // no extension lines: the dimension line is on the measured points
        assert_eq!(flat.items().unwrap().len(), 1 + 4 + 1);
    }

    #[test]
    fn what_cannot_be_drawn_is_refused() {
        let same = dim(DimensionKind::Aligned, pt(MM, MM), pt(MM, MM), MM);
        assert_eq!(same.items(), Err(DimensionError::Empty));
        let no_run = dim(DimensionKind::Horizontal, pt(0, 0), pt(0, MM), MM);
        assert_eq!(no_run.items(), Err(DimensionError::Empty));
        let mut thin = dim(DimensionKind::Aligned, pt(0, 0), pt(MM, 0), MM);
        thin.stroke = Nm::new(0).unwrap();
        assert_eq!(thin.items(), Err(DimensionError::Size));
        let mut flat = dim(DimensionKind::Aligned, pt(0, 0), pt(MM, 0), MM);
        flat.text_height = Nm::new(-1).unwrap();
        assert_eq!(flat.items(), Err(DimensionError::Size));
    }

    #[test]
    fn the_same_dimension_always_gives_the_same_drawing() {
        let d = dim(
            DimensionKind::Aligned,
            pt(123_456, -7_890),
            pt(9 * MM, 5 * MM),
            3 * MM,
        );
        assert_eq!(d.items().unwrap(), d.items().unwrap());
        for item in d.items().unwrap() {
            assert!(item.drawn().is_ok());
        }
    }
}
