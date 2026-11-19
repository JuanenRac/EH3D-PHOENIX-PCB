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
//! * `vertical`: the distance along y; the dimension line is vertical, at `from.x + offset`;
//! * `radius`: `from` is the centre of a circle and `to` a point on it; the leader runs from the
//!   centre through `to` and goes on `offset` beyond it (not negative), with an arrow head on the
//!   circle, and the text `R` and the radius at its end;
//! * `diameter`: `from` and `to` are the two ends of a diameter; the line joins them with an arrow
//!   head at each end, goes on `offset` beyond `to` (not negative), and the text `D` and the
//!   diameter are at its end;
//! * `angle`: `from` is the vertex, `to` a point on the first ray and `third` a point on the
//!   second; the angle measured is the smaller one (0 to 180 degrees, not 0), drawn as an arc of
//!   radius `offset` (above zero) made of straight pieces, with an arrow head at each end and the
//!   text in degrees beside its middle.
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
    /// The radius of a circle: from its centre to a point on it.
    Radius,
    /// The diameter of a circle: between the two ends of a diameter.
    Diameter,
    /// The angle between two rays from a vertex.
    Angle,
}

impl DimensionKind {
    /// How many points the person gives to make a dimension of the kind: the two points measured
    /// and the one the line passes through, and for an angle the vertex, a point on each ray and
    /// a point of the arc.
    pub fn clicks(self) -> usize {
        match self {
            DimensionKind::Angle => 4,
            _ => 3,
        }
    }

    /// The word of the file formats and the messages.
    pub fn as_str(self) -> &'static str {
        match self {
            DimensionKind::Aligned => "aligned",
            DimensionKind::Horizontal => "horizontal",
            DimensionKind::Vertical => "vertical",
            DimensionKind::Radius => "radius",
            DimensionKind::Diameter => "diameter",
            DimensionKind::Angle => "angle",
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
    /// The third point of an angle (a point on its second ray); none for every other kind, and
    /// not written to the file then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub third: Option<Point>,
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
    /// An angle without its third point, or another kind with one.
    Points,
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

/// A length in nanometres as text in millimetres, rounded to the micrometre, with the zeros at
/// the end of the decimals left out (`12.5`, `3`, `-0.025`).
pub fn mm_text(nm: i64) -> String {
    let um = (nm.unsigned_abs() + 500) / 1000;
    let (whole, frac) = (um / 1000, um % 1000);
    let sign = if nm < 0 && um > 0 { "-" } else { "" };
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let frac = format!("{frac:03}");
        format!("{sign}{whole}.{}", frac.trim_end_matches('0'))
    }
}

/// An angle in millionths of a degree as text in degrees, rounded to the hundredth, with the
/// zeros at the end of the decimals left out (`45`, `30.25`, `0.5`).
pub fn degrees_text(udeg: i64) -> String {
    let hundredths = (udeg.unsigned_abs() + 5_000) / 10_000;
    let (whole, frac) = (hundredths / 100, hundredths % 100);
    let sign = if udeg < 0 && hundredths > 0 { "-" } else { "" };
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let frac = format!("{frac:02}");
        format!("{sign}{whole}.{}", frac.trim_end_matches('0'))
    }
}

impl Dimension {
    /// The measure in nanometres, rounded to the nearest nanometre. An angle has no length: it
    /// gives 0 (see [`Dimension::angle_udeg`]).
    pub fn length_nm(&self) -> i64 {
        let dx = i128::from(self.to.x_nm.get()) - i128::from(self.from.x_nm.get());
        let dy = i128::from(self.to.y_nm.get()) - i128::from(self.from.y_nm.get());
        let length = match self.kind {
            DimensionKind::Horizontal => dx.unsigned_abs(),
            DimensionKind::Vertical => dy.unsigned_abs(),
            DimensionKind::Aligned | DimensionKind::Radius | DimensionKind::Diameter => {
                isqrt_round((dx * dx + dy * dy).unsigned_abs())
            }
            DimensionKind::Angle => 0,
        };
        i64::try_from(length).unwrap_or(i64::MAX)
    }

    /// The angle of an angle dimension in millionths of a degree, from 0 to 180 degrees, rounded
    /// to the nearest; `None` for another kind, without its third point or when a ray has no
    /// length. The arithmetic of the angle is the one place with floating point numbers; the
    /// result is rounded to a millionth of a degree.
    pub fn angle_udeg(&self) -> Option<i64> {
        if self.kind != DimensionKind::Angle {
            return None;
        }
        let third = self.third?;
        let (v, a, b) = (self.from, self.to, third);
        let (ax, ay) = (
            i128::from(a.x_nm.get()) - i128::from(v.x_nm.get()),
            i128::from(a.y_nm.get()) - i128::from(v.y_nm.get()),
        );
        let (bx, by) = (
            i128::from(b.x_nm.get()) - i128::from(v.x_nm.get()),
            i128::from(b.y_nm.get()) - i128::from(v.y_nm.get()),
        );
        if (ax == 0 && ay == 0) || (bx == 0 && by == 0) {
            return None;
        }
        let cross = (ax * by - ay * bx).unsigned_abs();
        let dot = ax * bx + ay * by;
        // the products fit in 124 bits; the angle is the same when both are scaled down
        let shift = (128 - cross.leading_zeros()).max(128 - dot.unsigned_abs().leading_zeros());
        let shift = shift.saturating_sub(52);
        let (c, d) = (
            (cross >> shift) as f64,
            if dot < 0 {
                -((dot.unsigned_abs() >> shift) as f64)
            } else {
                (dot.unsigned_abs() >> shift) as f64
            },
        );
        let degrees = c.atan2(d).to_degrees();
        Some((degrees * 1_000_000.0).round() as i64)
    }

    /// The unit of the measure: `mm` or `deg`.
    pub fn unit(&self) -> &'static str {
        if self.kind == DimensionKind::Angle {
            "deg"
        } else {
            "mm"
        }
    }

    /// The measure as a number: in millimetres, rounded to the micrometre, with the zeros at the
    /// end of the decimals left out (`12.5`, `3`, `0.025`); for an angle in degrees, rounded to
    /// the hundredth (`45`, `30.25`).
    pub fn measure_text(&self) -> String {
        match self.kind {
            DimensionKind::Angle => degrees_text(self.angle_udeg().unwrap_or(0)),
            _ => mm_text(self.length_nm().abs()),
        }
    }

    /// The measure with its unit for a message: `12.5 mm`, or `45\u{b0}` for an angle.
    pub fn measure_with_unit(&self) -> String {
        match self.kind {
            DimensionKind::Angle => format!("{}\u{b0}", self.measure_text()),
            _ => format!("{} mm", self.measure_text()),
        }
    }

    /// The text of the drawing: the measure with the mark of its kind in front for a radius
    /// (`R5`) and a diameter (`D10`), and the unit after it for an angle (`45 DEG`).
    pub fn label_text(&self) -> String {
        let measure = self.measure_text();
        match self.kind {
            DimensionKind::Radius => format!("R{measure}"),
            DimensionKind::Diameter => format!("D{measure}"),
            DimensionKind::Angle => format!("{measure} DEG"),
            _ => measure,
        }
    }

    /// True when the dimension measures nothing: a length of zero, or an angle that is missing,
    /// has a ray without length or is zero.
    pub fn measures_nothing(&self) -> bool {
        match self.kind {
            DimensionKind::Angle => self.angle_udeg().is_none_or(|a| a == 0),
            _ => self.length_nm() == 0,
        }
    }

    /// The distance between two points, rounded to the nearest nanometre; `None` when it is
    /// outside the range.
    pub fn distance(a: Point, b: Point) -> Option<Nm> {
        let dx = i128::from(b.x_nm.get()) - i128::from(a.x_nm.get());
        let dy = i128::from(b.y_nm.get()) - i128::from(a.y_nm.get());
        let length = isqrt_round((dx * dx + dy * dy).unsigned_abs());
        Nm::new(i64::try_from(length).ok()?).ok()
    }

    /// The shortest of the two rays of an angle, rounded to the nearest nanometre: the radius of
    /// its arc for the file formats that do not carry one. `None` when a ray has no length.
    pub fn shortest_ray(vertex: Point, a: Point, b: Point) -> Option<Nm> {
        let len = |p: Point| {
            let dx = i128::from(p.x_nm.get()) - i128::from(vertex.x_nm.get());
            let dy = i128::from(p.y_nm.get()) - i128::from(vertex.y_nm.get());
            isqrt_round((dx * dx + dy * dy).unsigned_abs())
        };
        let shortest = len(a).min(len(b));
        if shortest == 0 {
            return None;
        }
        Nm::new(i64::try_from(shortest).ok()?).ok()
    }

    /// A point of the dimension line (the end next to `from`), for the file formats that give the
    /// line by a point it passes through.
    pub fn line_point(&self) -> Result<Point, DimensionError> {
        let (a, _) = self.line_ends()?;
        point(a.0, a.1)
    }

    /// The far end of the line of a radius or a diameter (beyond `to` by the offset), where the
    /// text is: the point through which the formats that give a leader by its end pass. For the
    /// other kinds it is the end of the dimension line next to `to`; an angle has none.
    pub fn leader_end(&self) -> Result<Point, DimensionError> {
        let (_, b) = self.line_ends()?;
        point(b.0, b.1)
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
            DimensionKind::Radius | DimensionKind::Diameter => {
                let (dx, dy) = (bx - ax, by - ay);
                let len = i128::try_from(isqrt_round((dx * dx + dy * dy).unsigned_abs())).ok()?;
                if len == 0 {
                    return None;
                }
                // how far `through` is beyond `to` along the line, not below zero
                div_round((px - bx) * dx + (py - by) * dy, len).max(0)
            }
            DimensionKind::Angle => return None,
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
            DimensionKind::Angle => Err(DimensionError::Points),
            DimensionKind::Radius | DimensionKind::Diameter => {
                let (dx, dy) = (bx - ax, by - ay);
                let len = i128::try_from(isqrt_round((dx * dx + dy * dy).unsigned_abs()))
                    .map_err(|_| DimensionError::Range)?;
                if len == 0 {
                    return Err(DimensionError::Empty);
                }
                // the line goes on beyond `to` by the offset, which is not below zero
                let d = d.max(0);
                Ok((
                    (ax, ay),
                    (bx + div_round(dx * d, len), by + div_round(dy * d, len)),
                ))
            }
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
        if (self.kind == DimensionKind::Angle) != self.third.is_some() {
            return Err(DimensionError::Points);
        }
        if self.measures_nothing() {
            return Err(DimensionError::Empty);
        }
        if self.kind == DimensionKind::Angle {
            return self.angle_items();
        }
        let radial = matches!(self.kind, DimensionKind::Radius | DimensionKind::Diameter);
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
        // the extension lines, from the measured point to the dimension line (a leader has none)
        if !radial {
            if from != a {
                lines.push((from, a));
            }
            if to != b {
                lines.push((to, b));
            }
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
            match self.kind {
                // the arrow head of a radius is on the circle, pointing outwards
                DimensionKind::Radius => lines.extend(wing(to, 1)),
                DimensionKind::Diameter => {
                    lines.extend(wing(from, -1));
                    lines.extend(wing(to, 1));
                }
                _ => {
                    lines.extend(wing(a, -1));
                    lines.extend(wing(b, 1));
                }
            }
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
        let text = self.label_text();
        let height = i128::from(self.text_height.get());
        let units = 5 * i128::try_from(text.chars().count()).unwrap_or(0) - 1;
        let width = div_round(units * height, 6);
        let mid = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
        let gap = i128::from(self.stroke.get()) * 2;
        let horizontal = ux.abs() >= uy.abs();
        let (at, rotation) = if radial {
            // beside the end of the leader, horizontal, on the side the leader points to
            let x = if ux >= 0 {
                b.0 + gap
            } else {
                b.0 - gap - width
            };
            (point(x, b.1 + gap)?, Udeg::ZERO)
        } else if horizontal {
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

    /// The drawing of an angle: the extension lines from the two measured points to the ends of
    /// the arc, the arc as sixteen straight pieces (the angle halved four times with whole-number
    /// arithmetic, so the pieces do not depend on the system), the two arrow heads along the arc
    /// and the text outside its middle, in a fixed order.
    fn angle_items(&self) -> Result<Vec<SilkItem>, DimensionError> {
        let third = self.third.ok_or(DimensionError::Points)?;
        let radius = i128::from(self.offset.get());
        if radius <= 0 {
            return Err(DimensionError::Size);
        }
        let coords = |p: Point| (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
        let v = coords(self.from);
        let tips = [coords(self.to), coords(third)];
        let unit = |p: Spot| -> Result<Spot, DimensionError> {
            let d = (p.0 - v.0, p.1 - v.1);
            let len = i128::try_from(isqrt_round((d.0 * d.0 + d.1 * d.1).unsigned_abs()))
                .map_err(|_| DimensionError::Range)?;
            if len == 0 {
                return Err(DimensionError::Empty);
            }
            Ok((div_round(d.0 * UNIT, len), div_round(d.1 * UNIT, len)))
        };
        let (u1, u2) = (unit(tips[0])?, unit(tips[1])?);
        let mut dirs = vec![u1];
        halve(u1, u2, ARC_HALVINGS, &mut dirs);
        let on_arc = |u: Spot| {
            (
                v.0 + div_round(radius * u.0, UNIT),
                v.1 + div_round(radius * u.1, UNIT),
            )
        };
        let arc: Vec<Spot> = dirs.iter().map(|u| on_arc(*u)).collect();
        let mut lines: Vec<(Spot, Spot)> = Vec::new();
        for (tip, end) in tips.iter().zip([arc[0], arc[arc.len() - 1]]) {
            if *tip != end {
                lines.push((*tip, end));
            }
        }
        for pair in arc.windows(2) {
            lines.push((pair[0], pair[1]));
        }
        // the arrow heads: along the arc, pointing away from it
        let arrow = i128::from(self.text_height.get());
        let wing = |tip: Spot, inside: Spot| {
            // the first piece of the arc, turned round, is the tangent pointing away from it
            let (tx, ty) = (tip.0 - inside.0, tip.1 - inside.1);
            let len = isqrt_round((tx * tx + ty * ty).unsigned_abs());
            let len = i128::try_from(len).unwrap_or(1).max(1);
            let (px, py) = (div_round(tx * arrow, len), div_round(ty * arrow, len));
            let (qx, qy) = (-py, px);
            let back = (tip.0 - px, tip.1 - py);
            [
                (tip, (back.0 + div_round(qx, 3), back.1 + div_round(qy, 3))),
                (tip, (back.0 - div_round(qx, 3), back.1 - div_round(qy, 3))),
            ]
        };
        let last = arc.len() - 1;
        lines.extend(wing(arc[0], arc[1]));
        lines.extend(wing(arc[last], arc[last - 1]));
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
        // the text, outside the middle of the arc
        let text = self.label_text();
        let height = i128::from(self.text_height.get());
        let units = 5 * i128::try_from(text.chars().count()).unwrap_or(0) - 1;
        let width = div_round(units * height, 6);
        let middle = dirs[dirs.len() / 2];
        let reach = radius + height;
        let centre = (
            v.0 + div_round(reach * middle.0, UNIT),
            v.1 + div_round(reach * middle.1, UNIT),
        );
        items.push(SilkItem::Text {
            layer: self.layer,
            at: point(centre.0 - width / 2, centre.1 - height / 2)?,
            text,
            height: self.text_height,
            stroke: self.stroke,
            rotation: Udeg::ZERO,
            mirrored: false,
        });
        Ok(items)
    }
}

/// The length of a unit vector of the arc of an angle: 2^30.
const UNIT: i128 = 1 << 30;
/// How many times the arc of an angle is halved: sixteen straight pieces.
const ARC_HALVINGS: u32 = 4;

/// Adds to `out` the directions between the unit vectors `a` and `b` (the angle halved `depth`
/// times) and `b` itself, in order. Opposite vectors are halved towards the left of `a`.
fn halve(a: Spot, b: Spot, depth: u32, out: &mut Vec<Spot>) {
    if depth == 0 {
        out.push(b);
        return;
    }
    let sum = (a.0 + b.0, a.1 + b.1);
    let len =
        i128::try_from(isqrt_round((sum.0 * sum.0 + sum.1 * sum.1).unsigned_abs())).unwrap_or(0);
    let middle = if len == 0 {
        (-a.1, a.0)
    } else {
        (div_round(sum.0 * UNIT, len), div_round(sum.1 * UNIT, len))
    };
    halve(a, middle, depth - 1, out);
    halve(middle, b, depth - 1, out);
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
                    format!("text height, line width or radius of the arc of {what}"),
                )),
                Err(DimensionError::Points) => out.push(make(
                    BAD_SIZE,
                    format!("{what} has a third point without being an angle, or is an angle without it"),
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
            third: None,
            offset: Nm::new(offset).unwrap(),
            text_height: Nm::new(MM).unwrap(),
            stroke: Nm::new(100_000).unwrap(),
        }
    }

    fn angle(vertex: Point, to: Point, third: Point, radius: i64) -> Dimension {
        Dimension {
            third: Some(third),
            ..dim(DimensionKind::Angle, vertex, to, radius)
        }
    }

    fn text_of(item: &SilkItem) -> (String, Point) {
        match item {
            SilkItem::Text { text, at, .. } => (text.clone(), *at),
            SilkItem::Stroke { .. } => panic!("a text"),
        }
    }

    #[test]
    fn the_measure_with_its_unit_is_in_millimetres_or_degrees() {
        let radius = dim(DimensionKind::Radius, pt(0, 0), pt(5 * MM, 0), 0);
        assert_eq!(radius.measure_with_unit(), "5 mm");
        let right = angle(pt(0, 0), pt(MM, 0), pt(0, MM), MM);
        assert_eq!(right.measure_with_unit(), "90\u{b0}");
    }

    #[test]
    fn a_radius_is_a_leader_from_the_centre_with_an_arrow_head_on_the_circle() {
        let d = dim(DimensionKind::Radius, pt(0, 0), pt(5 * MM, 0), 2 * MM);
        assert_eq!((d.length_nm(), d.measure_text().as_str()), (5 * MM, "5"));
        assert_eq!(d.label_text(), "R5");
        assert_eq!(d.unit(), "mm");
        let items = d.items().unwrap();
        // the leader, the two lines of the arrow head and the text: no extension lines
        assert_eq!(items.len(), 4);
        assert_eq!(line(&items[0]), (pt(0, 0), pt(7 * MM, 0)));
        assert_eq!(line(&items[1]), (pt(5 * MM, 0), pt(4 * MM, 333_333)));
        assert_eq!(line(&items[2]), (pt(5 * MM, 0), pt(4 * MM, -333_333)));
        // "R5" is 9 units wide: 1.5 mm; the text starts 0.2 mm after the end of the leader
        assert_eq!(
            text_of(&items[3]),
            ("R5".to_owned(), pt(7 * MM + 200_000, 200_000))
        );
        // a leader that points to the left puts the text on its left
        let left = dim(DimensionKind::Radius, pt(0, 0), pt(-5 * MM, 0), MM);
        let items = left.items().unwrap();
        assert_eq!(line(&items[0]), (pt(0, 0), pt(-6 * MM, 0)));
        assert_eq!(
            text_of(&items[3]).1,
            pt(-6 * MM - 200_000 - 1_500_000, 200_000)
        );
        // a negative offset is taken as zero: the leader ends on the circle
        let short = dim(DimensionKind::Radius, pt(0, 0), pt(5 * MM, 0), -MM);
        assert_eq!(line(&short.items().unwrap()[0]), (pt(0, 0), pt(5 * MM, 0)));
    }

    #[test]
    fn a_diameter_has_an_arrow_head_at_each_end() {
        let d = dim(DimensionKind::Diameter, pt(-5 * MM, 0), pt(5 * MM, 0), 0);
        assert_eq!(d.length_nm(), 10 * MM);
        assert_eq!(d.label_text(), "D10");
        let items = d.items().unwrap();
        assert_eq!(items.len(), 6);
        assert_eq!(line(&items[0]), (pt(-5 * MM, 0), pt(5 * MM, 0)));
        assert_eq!(line(&items[1]), (pt(-5 * MM, 0), pt(-4 * MM, 333_333)));
        assert_eq!(line(&items[2]), (pt(-5 * MM, 0), pt(-4 * MM, -333_333)));
        assert_eq!(line(&items[3]), (pt(5 * MM, 0), pt(4 * MM, 333_333)));
        assert_eq!(
            text_of(&items[5]),
            ("D10".to_owned(), pt(5 * MM + 200_000, 200_000))
        );
        // the leader goes on beyond the second end by the offset
        let long = dim(DimensionKind::Diameter, pt(0, 0), pt(0, 4 * MM), 3 * MM);
        assert_eq!(line(&long.items().unwrap()[0]), (pt(0, 0), pt(0, 7 * MM)));
    }

    #[test]
    fn the_offset_of_a_leader_is_found_from_a_point_of_it() {
        let radius = |through: Point| {
            Dimension::offset_through(DimensionKind::Radius, pt(0, 0), pt(5 * MM, 0), through)
        };
        assert_eq!(radius(pt(8 * MM, 3 * MM)).unwrap().get(), 3 * MM);
        // a point inside the circle gives no offset
        assert_eq!(radius(pt(2 * MM, 0)).unwrap().get(), 0);
        let d = dim(DimensionKind::Diameter, pt(0, 0), pt(0, 4 * MM), 3 * MM);
        assert_eq!(d.line_point().unwrap(), pt(0, 0));
        assert_eq!(d.leader_end().unwrap(), pt(0, 7 * MM));
        assert_eq!(
            Dimension::offset_through(
                DimensionKind::Diameter,
                d.from,
                d.to,
                d.leader_end().unwrap()
            )
            .unwrap()
            .get(),
            3 * MM
        );
        assert!(
            Dimension::offset_through(DimensionKind::Angle, pt(0, 0), pt(MM, 0), pt(0, MM))
                .is_none()
        );
    }

    #[test]
    fn an_angle_is_measured_between_its_rays_and_the_smaller_angle_is_taken() {
        let right = angle(pt(0, 0), pt(10 * MM, 0), pt(0, 10 * MM), 5 * MM);
        assert_eq!(right.angle_udeg(), Some(90_000_000));
        assert_eq!(right.measure_text(), "90");
        assert_eq!(right.label_text(), "90 DEG");
        assert_eq!((right.unit(), right.length_nm()), ("deg", 0));
        // the order of the rays does not change it
        let back = angle(pt(0, 0), pt(0, 10 * MM), pt(10 * MM, 0), 5 * MM);
        assert_eq!(back.angle_udeg(), Some(90_000_000));
        let half = angle(pt(0, 0), pt(10 * MM, 0), pt(10 * MM, 10 * MM), 5 * MM);
        assert_eq!(half.angle_udeg(), Some(45_000_000));
        let reflex = angle(pt(0, 0), pt(10 * MM, 0), pt(-10 * MM, -10 * MM), 5 * MM);
        assert_eq!(reflex.angle_udeg(), Some(135_000_000));
        let straight = angle(pt(0, 0), pt(10 * MM, 0), pt(-3 * MM, 0), 5 * MM);
        assert_eq!(straight.angle_udeg(), Some(180_000_000));
        // far from the vertex the same angle comes out, with the products of big coordinates
        let far = angle(
            pt(-400_000_000, 0),
            pt(400_000_000, 0),
            pt(400_000_000, 800_000_000),
            5 * MM,
        );
        assert_eq!(far.angle_udeg(), Some(45_000_000));
        // not an angle, no third point, or a ray without length
        assert_eq!(
            dim(DimensionKind::Aligned, pt(0, 0), pt(MM, 0), 0).angle_udeg(),
            None
        );
        assert_eq!(
            dim(DimensionKind::Angle, pt(0, 0), pt(MM, 0), MM).angle_udeg(),
            None
        );
        assert_eq!(angle(pt(0, 0), pt(0, 0), pt(MM, 0), MM).angle_udeg(), None);
    }

    #[test]
    fn an_angle_is_written_in_degrees_with_the_decimals_that_are_needed_up_to_the_hundredth() {
        assert_eq!(degrees_text(0), "0");
        assert_eq!(degrees_text(45_000_000), "45");
        assert_eq!(degrees_text(30_250_000), "30.25");
        assert_eq!(degrees_text(30_254_000), "30.25");
        assert_eq!(degrees_text(500_000), "0.5");
        assert_eq!(degrees_text(179_996_000), "180");
        assert_eq!(degrees_text(-1_500_000), "-1.5");
        assert_eq!(degrees_text(-1_000), "0");
    }

    #[test]
    fn an_angle_is_drawn_with_an_arc_of_sixteen_pieces_and_two_arrow_heads() {
        let d = angle(pt(0, 0), pt(10 * MM, 0), pt(0, 10 * MM), 5 * MM);
        let items = d.items().unwrap();
        // two extension lines, sixteen pieces of arc, two arrow heads of two lines, the text
        assert_eq!(items.len(), 2 + 16 + 4 + 1);
        assert_eq!(line(&items[0]), (pt(10 * MM, 0), pt(5 * MM, 0)));
        assert_eq!(line(&items[1]), (pt(0, 10 * MM), pt(0, 5 * MM)));
        // the arc runs from the first ray to the second
        assert_eq!(line(&items[2]).0, pt(5 * MM, 0));
        assert_eq!(line(&items[17]).1, pt(0, 5 * MM));
        // every point of the arc is at the radius, within the rounding of the halving
        for item in &items[2..18] {
            let (a, b) = line(item);
            for p in [a, b] {
                let (x, y) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
                let r = (x * x + y * y).isqrt();
                assert!((r - i128::from(5 * MM)).abs() <= 2, "{r}");
            }
        }
        // the middle of the arc is on the bisector
        let (_, mid) = line(&items[9]);
        assert!((mid.x_nm.get() - 3_535_534).abs() <= 2 && (mid.y_nm.get() - 3_535_534).abs() <= 2);
        // the arrow heads point away from the arc along its ends: the first is at the first ray
        let (tip, wing) = line(&items[18]);
        assert_eq!(tip, pt(5 * MM, 0));
        // its wings are back along the arc, about one text height up the first ray's normal
        assert!((wing.y_nm.get() - MM).abs() <= 100_000);
        assert!((wing.x_nm.get() - 5 * MM).abs() <= 333_340);
        let (text, _) = text_of(&items[22]);
        assert_eq!(text, "90 DEG");
        // the same angle always gives the same drawing, and it can all be printed
        assert_eq!(items, d.items().unwrap());
        for item in &items {
            assert!(item.drawn().is_ok());
        }
    }

    #[test]
    fn a_straight_angle_is_drawn_through_the_left_of_its_first_ray() {
        let d = angle(pt(0, 0), pt(10 * MM, 0), pt(-10 * MM, 0), 5 * MM);
        let items = d.items().unwrap();
        let (_, top) = line(&items[2 + 7]);
        assert!(top.x_nm.get().abs() <= 2 && (top.y_nm.get() - 5 * MM).abs() <= 2);
        assert_eq!(line(&items[17]).1, pt(-5 * MM, 0));
    }

    #[test]
    fn what_cannot_be_drawn_as_a_radius_or_an_angle_is_refused() {
        let same = dim(DimensionKind::Radius, pt(MM, MM), pt(MM, MM), MM);
        assert_eq!(same.items(), Err(DimensionError::Empty));
        let flat = angle(pt(0, 0), pt(MM, 0), pt(2 * MM, 0), MM);
        assert_eq!(flat.items(), Err(DimensionError::Empty));
        let no_ray = angle(pt(0, 0), pt(0, 0), pt(2 * MM, 0), MM);
        assert_eq!(no_ray.items(), Err(DimensionError::Empty));
        let no_radius = angle(pt(0, 0), pt(MM, 0), pt(0, MM), 0);
        assert_eq!(no_radius.items(), Err(DimensionError::Size));
        let missing = dim(DimensionKind::Angle, pt(0, 0), pt(MM, 0), MM);
        assert_eq!(missing.items(), Err(DimensionError::Points));
        let extra = Dimension {
            third: Some(pt(0, MM)),
            ..dim(DimensionKind::Aligned, pt(0, 0), pt(MM, 0), MM)
        };
        assert_eq!(extra.items(), Err(DimensionError::Points));
        assert!(missing.measures_nothing() && same.measures_nothing());
        assert!(!dim(DimensionKind::Diameter, pt(0, 0), pt(MM, 0), 0).measures_nothing());
    }

    #[test]
    fn the_distance_and_the_clicks_of_a_dimension_are_told() {
        assert_eq!(
            Dimension::distance(pt(0, 0), pt(3 * MM, 4 * MM))
                .unwrap()
                .get(),
            5 * MM
        );
        assert_eq!(
            Dimension::distance(pt(MM, MM), pt(MM, MM)).unwrap().get(),
            0
        );
        assert_eq!(DimensionKind::Aligned.clicks(), 3);
        assert_eq!(DimensionKind::Radius.clicks(), 3);
        assert_eq!(DimensionKind::Angle.clicks(), 4);
    }

    #[test]
    fn the_shortest_ray_is_the_radius_for_the_formats_that_have_none() {
        assert_eq!(
            Dimension::shortest_ray(pt(0, 0), pt(3 * MM, 4 * MM), pt(0, 9 * MM))
                .unwrap()
                .get(),
            5 * MM
        );
        assert!(Dimension::shortest_ray(pt(0, 0), pt(0, 0), pt(MM, 0)).is_none());
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
    fn a_length_is_written_in_millimetres_with_its_sign() {
        assert_eq!(mm_text(0), "0");
        assert_eq!(mm_text(1_500_000), "1.5");
        assert_eq!(mm_text(-1_500_000), "-1.5");
        assert_eq!(mm_text(-400), "0");
        assert_eq!(mm_text(-25_000), "-0.025");
        assert_eq!(mm_text(999_600), "1");
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
