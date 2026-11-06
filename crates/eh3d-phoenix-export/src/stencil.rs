//! The stencil (the solder paste plate): the openings of each face, from the paste layer of the
//! board, as Gerber files and a report with a line for every opening.
//!
//! The settings come from `config/stencil.cfg` ([`StencilSettings::parse`]):
//!
//! * the reduction of the openings, by footprint family: `shrink` for all and `reduce = PATTERN
//!   VALUE` lines tried in order against the footprint name; a value is a per cent (`10%`: each
//!   side of the pad is that much shorter) or a length (`0.03mm`: each edge is that much inside the
//!   pad);
//! * the splitting of a large pad into windows: above `window_min_area_mm2` a rectangular pad is
//!   divided in a grid of panes of at most `window_max_mm` with bridges of `window_bridge_mm`;
//! * the rounding of the corners (`corner_radius_mm`, `corner_segments` pieces for each corner);
//! * the thickness of the plate and the limits of IPC-7525: the aspect ratio (the smaller side of
//!   the opening over the thickness) and the area ratio (the area of the opening over the area of
//!   its walls), both at least the values of the settings.
//!
//! A reduction that leaves an opening smaller than `min_feature_mm` takes it away (finding
//! `PHOENIX-STENCIL-010`). An opening is a flash of a circle, rectangle or obround aperture when it
//! can be, and a region when it has rounded corners or is turned by an angle that is not a right
//! angle. The files carry the Gerber X2 attributes that the export profile asks for. Lengths are
//! integer nanometres and the ratios integer millionths.

use std::collections::BTreeMap;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Component, Id, LayerKind, PadRef, PadShape, Side};

use crate::assembly::{format_micro, natural_key, FULL_TURN};
use crate::fabcodes::{
    FabError, Finding, Severity, STENCIL_AREA, STENCIL_ASPECT, STENCIL_NO_OPENINGS,
    STENCIL_NO_SIDE, STENCIL_NO_SPLIT, STENCIL_REGION, STENCIL_SETTINGS_INVALID, STENCIL_VANISHED,
    STENCIL_WINDOWS,
};
use crate::fabutil::{glob_match, parse_micro, settings_line, split_pair};
use crate::pnp::orientation;
use crate::profile::{ObjectAttribute, X2Level, X2Profile};
use crate::x2::{file_attributes, Fields, FileInput};

const HALF_TURN: i64 = 180_000_000;

/// How much smaller an opening is than its pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reduction {
    /// Each side is shorter by this part, in millionths of a per cent.
    Percent(i64),
    /// Each edge is this far inside the pad, in nanometres.
    Length(i64),
}

impl Reduction {
    fn parse(value: &str) -> Option<Reduction> {
        if let Some(number) = value.strip_suffix('%') {
            let p = parse_micro(number)?;
            return (0..100_000_000)
                .contains(&p)
                .then_some(Reduction::Percent(p));
        }
        let number = value.strip_suffix("mm")?;
        let l = parse_micro(number)?;
        (0..10_000_000).contains(&l).then_some(Reduction::Length(l))
    }

    /// A side of `side` nanometres after the reduction.
    pub fn apply(self, side: i64) -> i64 {
        match self {
            Reduction::Percent(p) => {
                let kept = i128::from(100_000_000 - p);
                (i128::from(side) * kept / 100_000_000) as i64
            }
            Reduction::Length(l) => side - 2 * l,
        }
    }
}

/// One line `reduce = PATTERN VALUE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceRule {
    /// The pattern of the footprint name.
    pub pattern: String,
    /// The reduction.
    pub reduction: Reduction,
}

/// The settings of the stencil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StencilSettings {
    /// The thickness of the plate, in nanometres.
    pub thickness_nm: i64,
    /// The least aspect ratio, in millionths.
    pub min_aspect_micro: i64,
    /// The least area ratio, in millionths.
    pub min_area_micro: i64,
    /// The reduction of a footprint that no `reduce` line matches.
    pub shrink: Reduction,
    /// The reductions by footprint family, in the order of the file.
    pub rules: Vec<ReduceRule>,
    /// The area, in millionths of a square millimetre, above which a pad is split into windows;
    /// 0 turns the splitting off.
    pub window_min_area_micro: i64,
    /// The largest side of a window, in nanometres.
    pub window_max_nm: i64,
    /// The width of the bridge between two windows, in nanometres.
    pub window_bridge_nm: i64,
    /// The radius of the corners of the openings, in nanometres (0: square corners).
    pub corner_radius_nm: i64,
    /// The pieces that draw a rounded corner.
    pub corner_segments: u32,
    /// The smallest side an opening may have, in nanometres.
    pub min_feature_nm: i64,
}

impl Default for StencilSettings {
    fn default() -> Self {
        StencilSettings {
            thickness_nm: 120_000,
            min_aspect_micro: 1_500_000,
            min_area_micro: 660_000,
            shrink: Reduction::Percent(0),
            rules: Vec::new(),
            window_min_area_micro: 6_000_000,
            window_max_nm: 2_000_000,
            window_bridge_nm: 300_000,
            corner_radius_nm: 0,
            corner_segments: 4,
            min_feature_nm: 50_000,
        }
    }
}

fn bad(detail: String) -> FabError {
    FabError::new(STENCIL_SETTINGS_INVALID, detail)
}

impl StencilSettings {
    /// Reads a settings text. A missing line keeps the default; an unknown key or a value out of
    /// range is refused with its line number.
    pub fn parse(text: &str) -> Result<StencilSettings, FabError> {
        let mut s = StencilSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let Some(line) = settings_line(raw) else {
                continue;
            };
            let (key, value) = split_pair(line, number, STENCIL_SETTINGS_INVALID)?;
            let invalid = || bad(format!("line {number}: the value of {key} is not valid"));
            let micro = || parse_micro(value).ok_or_else(invalid);
            match key {
                "thickness_mm" => s.thickness_nm = micro()?,
                "min_aspect" => s.min_aspect_micro = micro()?,
                "min_area_ratio" => s.min_area_micro = micro()?,
                "shrink" => s.shrink = Reduction::parse(value).ok_or_else(invalid)?,
                "reduce" => {
                    let mut words = value.split_whitespace();
                    let (Some(pattern), Some(amount), None) =
                        (words.next(), words.next(), words.next())
                    else {
                        return Err(invalid());
                    };
                    s.rules.push(ReduceRule {
                        pattern: pattern.to_owned(),
                        reduction: Reduction::parse(amount).ok_or_else(invalid)?,
                    });
                }
                "window_min_area_mm2" => s.window_min_area_micro = micro()?,
                "window_max_mm" => s.window_max_nm = micro()?,
                "window_bridge_mm" => s.window_bridge_nm = micro()?,
                "corner_radius_mm" => s.corner_radius_nm = micro()?,
                "corner_segments" => s.corner_segments = value.parse().map_err(|_| invalid())?,
                "min_feature_mm" => s.min_feature_nm = micro()?,
                _ => return Err(bad(format!("line {number}: unknown key {key}"))),
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges.
    pub fn validate(&self) -> Result<(), FabError> {
        let in_range = |name: &str, value: i64, low: i64, high: i64| {
            if (low..=high).contains(&value) {
                Ok(())
            } else {
                Err(bad(format!("{name} is out of range")))
            }
        };
        in_range("thickness_mm", self.thickness_nm, 10_000, 2_000_000)?;
        in_range("min_aspect", self.min_aspect_micro, 0, 100_000_000)?;
        in_range("min_area_ratio", self.min_area_micro, 0, 100_000_000)?;
        in_range(
            "window_min_area_mm2",
            self.window_min_area_micro,
            0,
            100_000_000_000,
        )?;
        in_range("window_max_mm", self.window_max_nm, 100_000, 50_000_000)?;
        in_range(
            "window_bridge_mm",
            self.window_bridge_nm,
            10_000,
            10_000_000,
        )?;
        in_range("corner_radius_mm", self.corner_radius_nm, 0, 5_000_000)?;
        in_range("min_feature_mm", self.min_feature_nm, 1_000, 5_000_000)?;
        in_range("corner_segments", i64::from(self.corner_segments), 1, 16)
    }

    /// The reduction for a footprint: the first line that matches, else `shrink`.
    pub fn reduction_for(&self, footprint: &str) -> Reduction {
        self.rules
            .iter()
            .find(|r| glob_match(&r.pattern, footprint))
            .map_or(self.shrink, |r| r.reduction)
    }
}

/// The shape of a pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A circle.
    Round,
    /// A rectangle.
    Rect,
    /// A rectangle with round ends.
    Obround,
}

impl Shape {
    fn word(self) -> &'static str {
        match self {
            Shape::Round => "round",
            Shape::Rect => "rect",
            Shape::Obround => "obround",
        }
    }
}

/// One window of an opening: its centre on the board and its sides along the axes of the pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pane {
    /// X of the centre, in nanometres.
    pub x: i64,
    /// Y of the centre, in nanometres.
    pub y: i64,
    /// The side along the axis of the pad that is its width.
    pub w: i64,
    /// The side along the other axis.
    pub h: i64,
}

/// The opening of a pad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The face.
    pub side: Side,
    /// The reference designator of the component.
    pub refdes: String,
    /// The number of the pad.
    pub pad: String,
    /// The name of the footprint.
    pub footprint: String,
    /// The shape of the pad.
    pub shape: Shape,
    /// The width of the pad, in nanometres.
    pub pad_w: i64,
    /// The height of the pad, in nanometres.
    pub pad_h: i64,
    /// The width of the opening after the reduction (all of it, before the windows).
    pub w: i64,
    /// The height of the opening after the reduction.
    pub h: i64,
    /// The direction of the pad, in micro-degrees.
    pub angle: i64,
    /// The windows; empty when the reduction took the opening away.
    pub panes: Vec<Pane>,
    /// The aspect ratio of a window, in millionths (0 when there is none).
    pub aspect_micro: i64,
    /// The area ratio of a window, in millionths (0 when there is none).
    pub area_micro: i64,
    /// True when the aspect ratio is below the limit.
    pub aspect_low: bool,
    /// True when the area ratio is below the limit.
    pub area_low: bool,
}

impl Opening {
    /// The word of the report: `ok`, `aspect`, `area`, `aspect+area` or `removed`.
    pub fn status(&self) -> &'static str {
        match (self.panes.is_empty(), self.aspect_low, self.area_low) {
            (true, _, _) => "removed",
            (false, false, false) => "ok",
            (false, true, false) => "aspect",
            (false, false, true) => "area",
            (false, true, true) => "aspect+area",
        }
    }
}

/// A point turned by `angle` micro-degrees about the origin; right angles are exact.
fn rotate_point(x: i64, y: i64, angle: i64) -> (i64, i64) {
    match angle.rem_euclid(FULL_TURN) {
        0 => (x, y),
        90_000_000 => (-y, x),
        180_000_000 => (-x, -y),
        270_000_000 => (y, -x),
        a => {
            let rad = (a as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
            let (s, c) = rad.sin_cos();
            let (fx, fy) = (x as f64, y as f64);
            (
                (fx * c - fy * s).round() as i64,
                (fx * s + fy * c).round() as i64,
            )
        }
    }
}

fn is_right_angle(angle: i64) -> bool {
    angle.rem_euclid(90_000_000) == 0
}

/// How many windows of at most `max` with bridges of `bridge` make a side of `side`.
fn windows_along(side: i64, max: i64, bridge: i64) -> i64 {
    let count = ((side + bridge) as u64).div_ceil((max + bridge) as u64);
    count.max(1) as i64
}

fn ratios(shape: Shape, w: i64, h: i64, thickness: i64) -> (i64, i64) {
    let t = i128::from(thickness);
    let (aspect, area) = match shape {
        Shape::Round => (
            i128::from(w) * 1_000_000 / t,
            i128::from(w) * 1_000_000 / (4 * t),
        ),
        _ => (
            i128::from(w.min(h)) * 1_000_000 / t,
            i128::from(w) * i128::from(h) * 1_000_000 / (2 * (i128::from(w) + i128::from(h)) * t),
        ),
    };
    (aspect as i64, area as i64)
}

/// The openings of every face. `findings` gets what was found.
pub fn openings(
    board: &Board,
    settings: &StencilSettings,
    findings: &mut Vec<Finding>,
) -> Vec<Opening> {
    let mut layers: Vec<(Id, Side)> = Vec::new();
    for (id, layer) in &board.layers {
        if layer.kind != LayerKind::Paste {
            continue;
        }
        match layer.side {
            Some(side) => layers.push((*id, side)),
            None => findings.push(Finding::new(
                STENCIL_NO_SIDE,
                Severity::Warning,
                layer.name.clone(),
                "its pads are left out",
            )),
        }
    }
    let mut parts: Vec<(&Id, &Component)> = board.components.iter().collect();
    parts.sort_by_cached_key(|(id, c)| (natural_key(&c.refdes), **id));
    let mut found = Vec::new();
    for (layer, side) in layers {
        for (cid, c) in &parts {
            let Some(footprint) = board.footprints.get(&c.footprint) else {
                continue;
            };
            let mut pads: Vec<_> = footprint.pads.iter().collect();
            pads.sort_by_cached_key(|(id, p)| (natural_key(&p.number), **id));
            for (pid, pad) in pads {
                if !pad.layers.contains(&layer) {
                    continue;
                }
                let Some(at) = pad_position(
                    board,
                    PadRef {
                        component: **cid,
                        pad: *pid,
                    },
                ) else {
                    continue;
                };
                let angle = orientation(c.rotation.get(), pad.rotation.get(), c.side);
                let (shape, pad_w, pad_h) = match pad.shape {
                    PadShape::Circle { diameter } => (Shape::Round, diameter.get(), diameter.get()),
                    PadShape::Rect { width, height } => (Shape::Rect, width.get(), height.get()),
                    PadShape::Oblong { width, height } => {
                        (Shape::Obround, width.get(), height.get())
                    }
                };
                let reduction = settings.reduction_for(&footprint.name);
                let (w, h) = (reduction.apply(pad_w), reduction.apply(pad_h));
                let subject = format!("{}.{}", c.refdes, pad.number);
                let mut opening = Opening {
                    side,
                    refdes: c.refdes.clone(),
                    pad: pad.number.clone(),
                    footprint: footprint.name.clone(),
                    shape,
                    pad_w,
                    pad_h,
                    w,
                    h,
                    angle,
                    panes: Vec::new(),
                    aspect_micro: 0,
                    area_micro: 0,
                    aspect_low: false,
                    area_low: false,
                };
                if w < settings.min_feature_nm || h < settings.min_feature_nm {
                    findings.push(Finding::new(
                        STENCIL_VANISHED,
                        Severity::Warning,
                        subject,
                        "",
                    ));
                    found.push(opening);
                    continue;
                }
                let (mut nx, mut ny) = (1, 1);
                let area_micro_mm2 = (i128::from(w) * i128::from(h) / 1_000_000) as i64;
                if shape != Shape::Round
                    && settings.window_min_area_micro > 0
                    && area_micro_mm2 > settings.window_min_area_micro
                {
                    let (gx, gy) = (
                        windows_along(w, settings.window_max_nm, settings.window_bridge_nm),
                        windows_along(h, settings.window_max_nm, settings.window_bridge_nm),
                    );
                    let bridge = settings.window_bridge_nm;
                    let (pw, ph) = ((w - (gx - 1) * bridge) / gx, (h - (gy - 1) * bridge) / gy);
                    if gx * gy > 1 {
                        if pw < settings.min_feature_nm || ph < settings.min_feature_nm {
                            findings.push(Finding::new(
                                STENCIL_NO_SPLIT,
                                Severity::Info,
                                subject.clone(),
                                "",
                            ));
                        } else {
                            nx = gx;
                            ny = gy;
                            findings.push(Finding::new(
                                STENCIL_WINDOWS,
                                Severity::Info,
                                subject.clone(),
                                format!("{nx} x {ny} windows"),
                            ));
                        }
                    }
                }
                let bridge = settings.window_bridge_nm;
                let (pw, ph) = ((w - (nx - 1) * bridge) / nx, (h - (ny - 1) * bridge) / ny);
                let (total_w, total_h) = (nx * pw + (nx - 1) * bridge, ny * ph + (ny - 1) * bridge);
                for j in 0..ny {
                    for i in 0..nx {
                        let ox = -total_w / 2 + pw / 2 + i * (pw + bridge);
                        let oy = -total_h / 2 + ph / 2 + j * (ph + bridge);
                        let (rx, ry) = rotate_point(ox, oy, angle);
                        opening.panes.push(Pane {
                            x: at.x_nm.get() + rx,
                            y: at.y_nm.get() + ry,
                            w: pw,
                            h: ph,
                        });
                    }
                }
                let (aspect, area) = ratios(shape, pw, ph, settings.thickness_nm);
                opening.aspect_micro = aspect;
                opening.area_micro = area;
                opening.aspect_low = aspect < settings.min_aspect_micro;
                opening.area_low = area < settings.min_area_micro;
                if opening.aspect_low {
                    findings.push(Finding::new(
                        STENCIL_ASPECT,
                        Severity::Warning,
                        subject.clone(),
                        format!(
                            "{} < {}",
                            format_micro(aspect, 3),
                            format_micro(settings.min_aspect_micro, 3)
                        ),
                    ));
                }
                if opening.area_low {
                    findings.push(Finding::new(
                        STENCIL_AREA,
                        Severity::Warning,
                        subject,
                        format!(
                            "{} < {}",
                            format_micro(area, 3),
                            format_micro(settings.min_area_micro, 3)
                        ),
                    ));
                }
                found.push(opening);
            }
        }
    }
    found
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Aperture {
    Circle(i64),
    Rect(i64, i64),
    Obround(i64, i64),
}

enum Mark {
    Flash(Aperture, i64, i64),
    Region(Vec<(i64, i64)>),
}

/// The points of a rectangle with rounded corners, counter-clockwise, about its centre and then
/// turned by `angle`.
fn rounded_rect(w: i64, h: i64, radius: i64, segments: u32, angle: i64) -> Vec<(i64, i64)> {
    let (hw, hh) = (w / 2, h / 2);
    let mut local: Vec<(i64, i64)> = Vec::new();
    if radius <= 0 {
        local.extend([(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]);
    } else {
        let corners = [
            (hw - radius, -(hh - radius), -90.0),
            (hw - radius, hh - radius, 0.0),
            (-(hw - radius), hh - radius, 90.0),
            (-(hw - radius), -(hh - radius), 180.0),
        ];
        for (cx, cy, start) in corners {
            for k in 0..=segments {
                let degrees = start + 90.0 * f64::from(k) / f64::from(segments);
                let rad = degrees * std::f64::consts::PI / 180.0;
                local.push((
                    cx + (radius as f64 * rad.cos()).round() as i64,
                    cy + (radius as f64 * rad.sin()).round() as i64,
                ));
            }
        }
    }
    let mut points: Vec<(i64, i64)> = local
        .into_iter()
        .map(|(x, y)| rotate_point(x, y, angle))
        .collect();
    points.dedup();
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn marks_of(
    opening: &Opening,
    settings: &StencilSettings,
    findings: &mut Vec<Finding>,
) -> Vec<Mark> {
    let mut marks = Vec::new();
    let swap = opening.angle.rem_euclid(HALF_TURN) == 90_000_000;
    let right = is_right_angle(opening.angle);
    let single = opening.panes.len() == 1;
    let mut region_for_angle = false;
    for pane in &opening.panes {
        let (x, y) = (pane.x, pane.y);
        let sides = if swap {
            (pane.h, pane.w)
        } else {
            (pane.w, pane.h)
        };
        if opening.shape == Shape::Round && single {
            marks.push(Mark::Flash(Aperture::Circle(pane.w), x, y));
            continue;
        }
        if opening.shape == Shape::Obround && single {
            if right {
                marks.push(Mark::Flash(Aperture::Obround(sides.0, sides.1), x, y));
            } else {
                region_for_angle = true;
                let r = pane.w.min(pane.h) / 2;
                let polygon = rounded_rect(
                    pane.w,
                    pane.h,
                    r,
                    settings.corner_segments.max(4),
                    opening.angle,
                );
                marks.push(Mark::Region(
                    polygon
                        .into_iter()
                        .map(|(px, py)| (x + px, y + py))
                        .collect(),
                ));
            }
            continue;
        }
        let radius = settings.corner_radius_nm.min(pane.w.min(pane.h) / 2);
        if radius == 0 && right {
            marks.push(Mark::Flash(Aperture::Rect(sides.0, sides.1), x, y));
        } else {
            region_for_angle |= !right;
            let polygon = rounded_rect(
                pane.w,
                pane.h,
                radius,
                settings.corner_segments,
                opening.angle,
            );
            marks.push(Mark::Region(
                polygon
                    .into_iter()
                    .map(|(px, py)| (x + px, y + py))
                    .collect(),
            ));
        }
    }
    if region_for_angle {
        findings.push(Finding::new(
            STENCIL_REGION,
            Severity::Info,
            format!("{}.{}", opening.refdes, opening.pad),
            "",
        ));
    }
    marks
}

fn aperture_text(number: u32, ap: Aperture) -> String {
    let mm = |nm: i64| format_micro(nm, 6);
    match ap {
        Aperture::Circle(d) => format!("%ADD{number}C,{}*%\n", mm(d)),
        Aperture::Rect(w, h) => format!("%ADD{number}R,{}X{}*%\n", mm(w), mm(h)),
        Aperture::Obround(w, h) => format!("%ADD{number}O,{}X{}*%\n", mm(w), mm(h)),
    }
}

/// The Gerber file of one face. `fields` counts the texts of attributes that needed an escape.
fn gerber_of(
    board_name: &str,
    side: Side,
    openings: &[&Opening],
    settings: &StencilSettings,
    x2: &X2Profile,
    findings: &mut Vec<Finding>,
) -> Result<String, FabError> {
    let mut fields = Fields::new(x2.strict);
    let field_error = |e: crate::x2::FieldError| bad(e.to_string());
    let mut tagged: Vec<(&Opening, Vec<Mark>)> = Vec::new();
    for &o in openings {
        tagged.push((o, marks_of(o, settings, findings)));
    }
    let mut apertures: Vec<Aperture> = tagged
        .iter()
        .flat_map(|(_, marks)| marks.iter())
        .filter_map(|m| match m {
            Mark::Flash(a, _, _) => Some(*a),
            Mark::Region(_) => None,
        })
        .collect();
    apertures.sort();
    apertures.dedup();
    let numbers: BTreeMap<Aperture, u32> = apertures
        .iter()
        .enumerate()
        .map(|(i, a)| (*a, 10 + i as u32))
        .collect();
    let mut text = String::new();
    if x2.level != X2Level::X1 {
        let function = vec![
            "Paste".to_owned(),
            match side {
                Side::Top => "Top",
                Side::Bottom => "Bot",
            }
            .to_owned(),
        ];
        let input = FileInput {
            x2,
            board_name,
            function: Some(function),
            polarity: Some("Positive"),
        };
        for (name, values) in file_attributes(&input, &mut fields).map_err(field_error)? {
            text.push_str(&format!("%TF{name}"));
            for v in values {
                text.push(',');
                text.push_str(&v);
            }
            text.push_str("*%\n");
        }
    }
    text.push_str("%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\n");
    for (ap, number) in &numbers {
        text.push_str(&aperture_text(*number, *ap));
    }
    text.push_str("G01*\n");
    let component = x2.writes(ObjectAttribute::Component);
    let pin = x2.writes(ObjectAttribute::Pin);
    let mut current: Option<u32> = None;
    let mut attributed = false;
    for (o, marks) in &tagged {
        if component && !o.refdes.is_empty() {
            text.push_str(&format!(
                "%TO.C,{}*%\n",
                fields.text(&o.refdes).map_err(field_error)?
            ));
            attributed = true;
        }
        if pin && !o.refdes.is_empty() && !o.pad.is_empty() {
            text.push_str(&format!(
                "%TO.P,{},{}*%\n",
                fields.text(&o.refdes).map_err(field_error)?,
                fields.text(&o.pad).map_err(field_error)?
            ));
            attributed = true;
        }
        for mark in marks {
            match mark {
                Mark::Flash(ap, x, y) => {
                    let number = numbers[ap];
                    if current != Some(number) {
                        text.push_str(&format!("D{number}*\n"));
                        current = Some(number);
                    }
                    text.push_str(&format!("X{x}Y{y}D03*\n"));
                }
                Mark::Region(points) => {
                    if points.len() < 3 {
                        continue;
                    }
                    text.push_str("G36*\n");
                    text.push_str(&format!("X{}Y{}D02*\n", points[0].0, points[0].1));
                    for (x, y) in points.iter().skip(1) {
                        text.push_str(&format!("X{x}Y{y}D01*\n"));
                    }
                    text.push_str(&format!("X{}Y{}D01*\n", points[0].0, points[0].1));
                    text.push_str("G37*\n");
                }
            }
        }
    }
    if attributed {
        text.push_str("%TD*%\n");
    }
    text.push_str("M02*\n");
    Ok(text)
}

/// The report of the openings, a CSV with a line for each.
pub fn report_csv(openings: &[Opening]) -> String {
    let mut text = String::from(
        "face,reference,pad,footprint,shape,pad_width_mm,pad_height_mm,aperture_width_mm,aperture_height_mm,windows,aspect_ratio,area_ratio,status\n",
    );
    for o in openings {
        let (aw, ah, windows) = if o.panes.is_empty() {
            (0, 0, 0)
        } else {
            (o.panes[0].w, o.panes[0].h, o.panes.len())
        };
        let fields = [
            match o.side {
                Side::Top => "top".to_owned(),
                Side::Bottom => "bottom".to_owned(),
            },
            o.refdes.clone(),
            o.pad.clone(),
            o.footprint.clone(),
            o.shape.word().to_owned(),
            format_micro(o.pad_w, 4),
            format_micro(o.pad_h, 4),
            format_micro(aw, 4),
            format_micro(ah, 4),
            windows.to_string(),
            if o.panes.is_empty() {
                String::new()
            } else {
                format_micro(o.aspect_micro, 3)
            },
            if o.panes.is_empty() {
                String::new()
            } else {
                format_micro(o.area_micro, 3)
            },
            o.status().to_owned(),
        ];
        text.push_str(&crate::assembly::csv_line(&fields, ','));
    }
    text
}

/// One file of the stencil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StencilFile {
    /// The name of the file.
    pub name: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// The files of the stencil and what was found.
#[derive(Debug, Clone)]
pub struct Stencil {
    /// The Gerber file of each face that has openings, then the report.
    pub files: Vec<StencilFile>,
    /// Every opening, face by face.
    pub openings: Vec<Opening>,
    /// What was found.
    pub findings: Vec<Finding>,
}

/// Makes the files of the stencil: `stencil_top.gbr`, `stencil_bottom.gbr` (each only when its
/// face has openings) and `stencil_report.csv`.
pub fn build_stencil(
    board: &Board,
    settings: &StencilSettings,
    x2: &X2Profile,
) -> Result<Stencil, FabError> {
    settings.validate()?;
    let mut findings = Vec::new();
    let all = openings(board, settings, &mut findings);
    let mut files = Vec::new();
    for (side, name) in [
        (Side::Top, "stencil_top.gbr"),
        (Side::Bottom, "stencil_bottom.gbr"),
    ] {
        let kept: Vec<&Opening> = all
            .iter()
            .filter(|o| o.side == side && !o.panes.is_empty())
            .collect();
        if kept.is_empty() {
            continue;
        }
        let text = gerber_of(&board.name, side, &kept, settings, x2, &mut findings)?;
        files.push(StencilFile {
            name: name.to_owned(),
            bytes: text.into_bytes(),
        });
    }
    if all.is_empty() {
        findings.push(Finding::new(STENCIL_NO_OPENINGS, Severity::Warning, "", ""));
    } else {
        files.push(StencilFile {
            name: "stencil_report.csv".to_owned(),
            bytes: report_csv(&all).into_bytes(),
        });
    }
    Ok(Stencil {
        files,
        openings: all,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Footprint, Layer, Pad};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

    const MM: i64 = 1_000_000;

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn rect_pad(number: &str, at: (i64, i64), w: i64, h: i64) -> Pad {
        Pad {
            number: number.to_owned(),
            shape: PadShape::Rect {
                width: Nm::new(w).unwrap(),
                height: Nm::new(h).unwrap(),
            },
            at: pt(at.0, at.1),
            rotation: Udeg::new(0).unwrap(),
            drill: None,
            layers: vec![id(2)],
            roundness: 0,
            roundness: 0,
            plated: true,
            plated: true,
        }
    }

    /// A board with a paste layer for each face and: R1 (two pads of 0.8 by 1.0 mm), U1 (a pad of
    /// 3 by 3 mm and a round pad of 0.6 mm) on the top face, and C1 on the bottom face.
    fn board() -> Board {
        let mut b = Board {
            name: "stencil".to_owned(),
            ..Board::default()
        };
        b.layers.insert(
            id(2),
            Layer {
                name: "F.Paste".to_owned(),
                kind: LayerKind::Paste,
                side: Some(Side::Top),
            },
        );
        b.layers.insert(
            id(3),
            Layer {
                name: "B.Paste".to_owned(),
                kind: LayerKind::Paste,
                side: Some(Side::Bottom),
            },
        );
        let mut r = Footprint {
            name: "R0603".to_owned(),
            pads: Default::default(),
        };
        r.pads
            .insert(id(100), rect_pad("1", (-800_000, 0), 800_000, 1_000_000));
        r.pads
            .insert(id(101), rect_pad("2", (800_000, 0), 800_000, 1_000_000));
        let mut under = r.clone();
        for pad in under.pads.values_mut() {
            pad.layers = vec![id(3)];
        }
        b.footprints.insert(id(10), r);
        b.footprints.insert(id(12), under);
        let mut u = Footprint {
            name: "QFN16".to_owned(),
            pads: Default::default(),
        };
        u.pads
            .insert(id(110), rect_pad("17", (0, 0), 3 * MM, 3 * MM));
        let mut round = rect_pad("1", (4 * MM, 0), 0, 0);
        round.shape = PadShape::Circle {
            diameter: Nm::new(600_000).unwrap(),
        };
        u.pads.insert(id(111), round);
        b.footprints.insert(id(11), u);
        let place = |name: &str, fp: u32, at: (i64, i64), side: Side| Component {
            refdes: name.to_owned(),
            value: "x".to_owned(),
            symbol: None,
            footprint: id(fp),
            at: pt(at.0, at.1),
            rotation: Udeg::new(0).unwrap(),
            side,
        };
        b.components
            .insert(id(20), place("R1", 10, (10 * MM, 10 * MM), Side::Top));
        b.components
            .insert(id(21), place("U1", 11, (20 * MM, 10 * MM), Side::Top));
        b.components
            .insert(id(22), place("C1", 12, (10 * MM, 20 * MM), Side::Bottom));
        b
    }

    #[test]
    fn the_settings_are_read_and_the_wrong_ones_refused() {
        let s = StencilSettings::parse(
            "thickness_mm = 0.1\nmin_aspect = 1.2\nshrink = 5%\nreduce = QFN* 0.04mm\n\
             reduce = R0* 10%\nwindow_min_area_mm2 = 4\nwindow_max_mm = 1.5\ncorner_radius_mm = 0.05\n\
             corner_segments = 6\n",
        )
        .unwrap();
        assert_eq!(s.thickness_nm, 100_000);
        assert_eq!(s.min_aspect_micro, 1_200_000);
        assert_eq!(s.shrink, Reduction::Percent(5_000_000));
        assert_eq!(s.reduction_for("QFN16"), Reduction::Length(40_000));
        assert_eq!(s.reduction_for("r0603"), Reduction::Percent(10_000_000));
        assert_eq!(s.reduction_for("SOIC8"), Reduction::Percent(5_000_000));
        assert_eq!(s.corner_segments, 6);
        for text in [
            "nope = 1\n",
            "shrink = 5\n",
            "shrink = 120%\n",
            "reduce = A\n",
            "reduce = A 5\n",
            "thickness_mm = 0\n",
            "thickness_mm = x\n",
            "corner_segments = 0\n",
            "window_bridge_mm = 0\n",
            "thickness_mm\n",
        ] {
            let e = StencilSettings::parse(text).unwrap_err();
            assert_eq!(e.code, STENCIL_SETTINGS_INVALID, "{text}");
        }
        assert_eq!(
            StencilSettings::parse("# nothing\n").unwrap(),
            StencilSettings::default()
        );
    }

    #[test]
    fn the_settings_that_come_with_the_program_are_the_defaults() {
        let shipped = include_str!("../../../config/stencil.cfg");
        assert_eq!(
            StencilSettings::parse(shipped).unwrap(),
            StencilSettings::default()
        );
    }

    #[test]
    fn a_reduction_by_a_part_or_by_a_length_is_applied_to_each_side() {
        assert_eq!(Reduction::Percent(10_000_000).apply(1_000_000), 900_000);
        assert_eq!(Reduction::Length(50_000).apply(1_000_000), 900_000);
        assert_eq!(Reduction::Percent(0).apply(777_777), 777_777);
    }

    #[test]
    fn the_ratios_follow_ipc_7525() {
        // a rectangle of 0.5 by 1.0 mm in a plate of 0.12 mm: aspect 0.5 / 0.12, area ratio
        // (0.5 * 1.0) / (2 * (0.5 + 1.0) * 0.12)
        let (aspect, area) = ratios(Shape::Rect, 500_000, 1_000_000, 120_000);
        assert_eq!(aspect, 4_166_666);
        assert_eq!(area, 1_388_888);
        // a circle of 0.6 mm: aspect 0.6 / 0.12 and area ratio 0.6 / (4 * 0.12)
        let (aspect, area) = ratios(Shape::Round, 600_000, 600_000, 120_000);
        assert_eq!(aspect, 5_000_000);
        assert_eq!(area, 1_250_000);
    }

    #[test]
    fn windows_along_a_side_make_a_grid_with_bridges() {
        // 3 mm with windows of at most 2 mm and bridges of 0.3 mm need 2 windows
        assert_eq!(windows_along(3 * MM, 2 * MM, 300_000), 2);
        assert_eq!(windows_along(2 * MM, 2 * MM, 300_000), 1);
        assert_eq!(windows_along(4_300_000, 2 * MM, 300_000), 2);
        assert_eq!(windows_along(4_301_000, 2 * MM, 300_000), 3);
    }

    #[test]
    fn the_big_pad_is_split_and_the_others_are_not() {
        let b = board();
        let mut findings = Vec::new();
        let all = openings(&b, &StencilSettings::default(), &mut findings);
        // R1 has two pads, U1 two, C1 two
        assert_eq!(all.len(), 6);
        let big = all.iter().find(|o| o.pad == "17").unwrap();
        // 3 by 3 mm is 9 mm2, above the 6 mm2 of the defaults: 2 by 2 windows of 1.35 mm
        assert_eq!(big.panes.len(), 4);
        assert_eq!(big.panes[0].w, 1_350_000);
        assert_eq!(big.panes[0].h, 1_350_000);
        // the centres are 1.65 mm apart (a window and a bridge) about the pad at (20, 10) mm
        assert_eq!(big.panes[0].x, 20 * MM - 825_000);
        assert_eq!(big.panes[1].x, 20 * MM + 825_000);
        assert_eq!(big.panes[0].y, 10 * MM - 825_000);
        assert_eq!(big.panes[2].y, 10 * MM + 825_000);
        assert!(findings
            .iter()
            .any(|f| f.code == STENCIL_WINDOWS && f.subject == "U1.17"));
        let small = all.iter().find(|o| o.refdes == "R1").unwrap();
        assert_eq!(small.panes.len(), 1);
        assert_eq!(small.status(), "ok");
    }

    #[test]
    fn a_pad_whose_ratios_are_low_is_reported() {
        let b = board();
        let settings = StencilSettings::parse("thickness_mm = 0.6\n").unwrap();
        let mut findings = Vec::new();
        let all = openings(&b, &settings, &mut findings);
        let r1 = all
            .iter()
            .find(|o| o.refdes == "R1" && o.pad == "1")
            .unwrap();
        // 0.8 mm over 0.6 mm is 1.333, below 1.5; the area ratio is 0.8 * 1.0 / (2 * 1.8 * 0.6)
        assert!(r1.aspect_low);
        assert!(r1.area_low);
        assert_eq!(r1.status(), "aspect+area");
        assert!(findings.iter().any(|f| f.code == STENCIL_ASPECT));
        assert!(findings.iter().any(|f| f.code == STENCIL_AREA));
    }

    #[test]
    fn a_reduction_that_leaves_nothing_takes_the_opening_away() {
        let b = board();
        let settings = StencilSettings::parse("reduce = R0603 0.4mm\n").unwrap();
        let mut findings = Vec::new();
        let all = openings(&b, &settings, &mut findings);
        // 0.8 mm minus 0.8 mm leaves nothing
        let gone = all.iter().find(|o| o.refdes == "R1").unwrap();
        assert!(gone.panes.is_empty());
        assert_eq!(gone.status(), "removed");
        assert_eq!(
            findings
                .iter()
                .filter(|f| f.code == STENCIL_VANISHED)
                .count(),
            4
        );
    }

    #[test]
    fn the_files_of_both_faces_and_the_report_are_made_and_are_the_same_every_time() {
        let b = board();
        let settings = StencilSettings::default();
        let x2 = X2Profile::default();
        let one = build_stencil(&b, &settings, &x2).unwrap();
        let two = build_stencil(&b, &settings, &x2).unwrap();
        let names: Vec<&str> = one.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "stencil_top.gbr",
                "stencil_bottom.gbr",
                "stencil_report.csv"
            ]
        );
        assert_eq!(one.files, two.files);
        let top = String::from_utf8(one.files[0].bytes.clone()).unwrap();
        assert!(top.contains("%TF.FileFunction,Paste,Top*%\n"));
        assert!(top.contains("%TF.FilePolarity,Positive*%\n"));
        assert!(top.contains("%FSLAX46Y46*%\n%MOMM*%\n"));
        assert!(top.contains("%ADD10C,0.600000*%\n"));
        assert!(top.contains("R,0.800000X1.000000*%\n"));
        assert!(top.contains("X9200000Y10000000D03*\n"));
        assert!(top.ends_with("M02*\n"));
        let bottom = String::from_utf8(one.files[1].bytes.clone()).unwrap();
        assert!(bottom.contains("%TF.FileFunction,Paste,Bot*%\n"));
        let report = String::from_utf8(one.files[2].bytes.clone()).unwrap();
        assert!(report.starts_with("face,reference,pad,footprint,shape,"));
        assert_eq!(report.lines().count(), 7);
        assert!(report.contains("top,R1,1,R0603,rect,0.8000,1.0000,0.8000,1.0000,1,"));
        assert!(report.contains("top,U1,17,QFN16,rect,3.0000,3.0000,1.3500,1.3500,4,"));
    }

    #[test]
    fn the_object_attributes_follow_the_profile() {
        let b = board();
        let x2 = X2Profile {
            level: X2Level::Full,
            ..X2Profile::default()
        };
        let set = build_stencil(&b, &StencilSettings::default(), &x2).unwrap();
        let top = String::from_utf8(set.files[0].bytes.clone()).unwrap();
        assert!(top.contains("%TO.C,R1*%\n"));
        assert!(top.contains("%TO.P,R1,1*%\n"));
        assert!(top.contains("%TD*%\nM02*\n"));
        let x2 = X2Profile {
            level: X2Level::X1,
            ..X2Profile::default()
        };
        let set = build_stencil(&b, &StencilSettings::default(), &x2).unwrap();
        let top = String::from_utf8(set.files[0].bytes.clone()).unwrap();
        assert!(!top.contains("%TF"));
        assert!(!top.contains("%TO"));
    }

    #[test]
    fn rounded_corners_and_turned_pads_are_written_as_regions() {
        let b = board();
        let settings =
            StencilSettings::parse("corner_radius_mm = 0.1\ncorner_segments = 2\n").unwrap();
        let set = build_stencil(&b, &settings, &X2Profile::default()).unwrap();
        let top = String::from_utf8(set.files[0].bytes.clone()).unwrap();
        assert!(top.contains("G36*\n"));
        assert!(top.contains("G37*\n"));
        // a rectangle with round corners of two pieces has 4 * 3 points
        let region_points = top
            .split("G36*\n")
            .nth(1)
            .unwrap()
            .split("G37*\n")
            .next()
            .unwrap()
            .lines()
            .count();
        assert_eq!(region_points, 13);
        // a pad turned by 45 degrees has a region of four corners
        let points = rounded_rect(2 * MM, MM, 0, 4, 45_000_000);
        assert_eq!(points.len(), 4);
        assert!(!is_right_angle(45_000_000));
        let right = rounded_rect(2 * MM, MM, 0, 4, 90_000_000);
        assert_eq!(right[0], (MM / 2, -MM));
    }

    #[test]
    fn a_board_without_openings_gives_a_finding_and_no_file() {
        let mut b = board();
        b.layers.clear();
        let set = build_stencil(&b, &StencilSettings::default(), &X2Profile::default()).unwrap();
        assert!(set.files.is_empty());
        assert_eq!(set.findings[0].code, STENCIL_NO_OPENINGS);
        let mut b = board();
        b.layers.get_mut(&id(2)).unwrap().side = None;
        let set = build_stencil(&b, &StencilSettings::default(), &X2Profile::default()).unwrap();
        assert!(set.findings.iter().any(|f| f.code == STENCIL_NO_SIDE));
        assert_eq!(set.files.len(), 2);
    }
}
