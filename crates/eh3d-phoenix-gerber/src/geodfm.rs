// SPDX-License-Identifier: GPL-3.0-or-later
//! Geometric checks of imported Gerber and Excellon files against the figures of a capability
//! profile of a maker: the narrowest gap between copper, the narrowest conductor, the ring of
//! copper round a plated hole in the worst case, copper at the edge, the bridge of solder mask,
//! legend over a pad, a via inside a pad and floating copper.
//!
//! The shapes are made from the files with the 2D geometry of EH3D-CORE-LIBS (`eh3d-geometry`:
//! unions, differences and offsets of regions). A gap or a conductor narrower than a limit is found
//! by the closing or the opening of the layer by half that limit; what the rounded corners of that
//! operation leave is ignored (`corner_area_factor_x1000`). The width of a finding is an estimate
//! (twice the area over the perimeter of the sliver found), good for a long narrow sliver.
//!
//! **What this does not know.** A Gerber file has no nets unless X2 says them, so the spacing is
//! the spacing between any two pieces of copper, the same net included. The mask layers are taken as
//! positive (a flash is an opening). The figures of the profiles are not verified.

use eh3d_geometry::boolean::{difference, intersection, union, union_all, BooleanOptions};
use eh3d_geometry::offset::{offset, stroke, OffsetOptions};
use eh3d_geometry::polygon::{signed_area, Containment, Pt, Region};
use eh3d_phoenix_rules::DfmProfile;

use crate::geodfm_text as text;
use crate::identify::Role;
use crate::import::{Content, ImportedSet};
use crate::model::{Aperture, ApertureShape, GerberLayer, Item, MacroShape, P};

/// The settings of the geometric checks, read from `config/gerber_dfm.cfg`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GeoSettings {
    /// How far a drawn circle or offset may lie inside the true one, in nanometres.
    pub offset_tolerance_nm: i64,
    /// The grid the corners move to in the operations, in nanometres.
    pub grid_nm: i64,
    /// The most edges of an operation.
    pub max_edges: usize,
    /// Registration error of the drill that is taken from the ring, in nanometres.
    pub registration_nm: i64,
    /// The largest drill of a hole taken as a via when the file does not say, in nanometres.
    pub via_max_drill_nm: i64,
    /// The area, in thousandths of the square of the limit, under which a find is rounding.
    pub corner_area_factor_x1000: i64,
    /// The area under which an overlap is rounding, in square nanometres.
    pub area_floor_nm2: i64,
    /// The most findings kept for one check.
    pub max_findings: usize,
    /// The most holes looked at.
    pub max_holes: usize,
    /// The most anchor points looked at.
    pub max_anchors: usize,
    /// The distance an arc is cut to straight pieces within, in nanometres.
    pub arc_chord_nm: i64,
    /// Things closer than this are one thing (touching), in nanometres.
    pub touch_nm: i64,
    /// The most pairs of things looked at in one layer for spacing.
    pub max_pair_tests: usize,
    /// The most entries of a ZIP archive.
    pub zip_max_entries: usize,
    /// The most bytes all the entries of a ZIP archive hold once unpacked.
    pub zip_max_total_bytes: u64,
    /// The most an entry of a ZIP archive may grow, as a multiple of its packed size.
    pub zip_max_ratio: u64,
}

/// The text of the settings that comes with the program.
pub const SETTINGS_TEXT: &str = include_str!("../../../config/gerber_dfm.cfg");

impl GeoSettings {
    /// The settings of the file that comes with the program.
    pub fn standard() -> GeoSettings {
        GeoSettings::parse(SETTINGS_TEXT).unwrap_or_default()
    }

    /// Read a settings text. A key that is not known, a key twice and a value that is not a whole
    /// number are refused with the number of the line.
    pub fn parse(text: &str) -> Result<GeoSettings, String> {
        let mut s = GeoSettings::default();
        let mut seen: Vec<String> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = index + 1;
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("line {at}: expected key = value"));
            };
            let (key, value) = (key.trim(), value.trim());
            if seen.iter().any(|k| k == key) {
                return Err(format!("line {at}: {key} is given twice"));
            }
            seen.push(key.to_owned());
            let number: i64 = value
                .parse()
                .map_err(|_| format!("line {at}: {key} needs a whole number"))?;
            if number < 0 {
                return Err(format!("line {at}: {key} cannot be negative"));
            }
            match key {
                "offset_tolerance_nm" => s.offset_tolerance_nm = number,
                "grid_nm" => s.grid_nm = number,
                "max_edges" => s.max_edges = number as usize,
                "registration_nm" => s.registration_nm = number,
                "via_max_drill_nm" => s.via_max_drill_nm = number,
                "corner_area_factor_x1000" => s.corner_area_factor_x1000 = number,
                "area_floor_nm2" => s.area_floor_nm2 = number,
                "max_findings" => s.max_findings = number as usize,
                "max_holes" => s.max_holes = number as usize,
                "max_anchors" => s.max_anchors = number as usize,
                "arc_chord_nm" => s.arc_chord_nm = number,
                "touch_nm" => s.touch_nm = number,
                "max_pair_tests" => s.max_pair_tests = number as usize,
                "zip_max_entries" => s.zip_max_entries = number as usize,
                "zip_max_total_bytes" => s.zip_max_total_bytes = number as u64,
                "zip_max_ratio" => s.zip_max_ratio = number as u64,
                _ => return Err(format!("line {at}: {key} is not a known key")),
            }
        }
        if s.offset_tolerance_nm == 0
            || s.grid_nm == 0
            || s.max_edges == 0
            || s.arc_chord_nm == 0
            || s.touch_nm == 0
            || s.max_pair_tests == 0
            || s.zip_max_entries == 0
            || s.zip_max_total_bytes == 0
            || s.zip_max_ratio == 0
        {
            return Err(
                "offset_tolerance_nm, grid_nm, max_edges, arc_chord_nm and the limits of the ZIP archive must be above zero"
                    .to_owned(),
            );
        }
        Ok(s)
    }
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoSeverity {
    /// The files do not pass.
    Error,
    /// Worth a look; the files still pass.
    Warning,
    /// A note.
    Info,
}

/// One finding of a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoFinding {
    /// The stable code (`PHOENIX-GDF-<NNN>`).
    pub code: &'static str,
    /// How serious it is.
    pub severity: GeoSeverity,
    /// The layer, by the name of its file.
    pub layer: String,
    /// Where, in nanometres.
    pub x_nm: i64,
    /// Where, in nanometres.
    pub y_nm: i64,
    /// What was measured (an estimate for a sliver), in nanometres; 0 when it has no meaning.
    pub measured_nm: i64,
    /// The limit of the profile, in nanometres; 0 when the check has none.
    pub limit_nm: i64,
    /// Extra words in English (a reason that a check was skipped).
    pub detail: String,
}

/// The result of the checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoReport {
    /// The name of the profile.
    pub profile: String,
    /// Whether the profile says its figures were verified.
    pub profile_verified: bool,
    /// How many files were read.
    pub files: usize,
    /// The findings, by check and then by place.
    pub findings: Vec<GeoFinding>,
    /// The findings that were over the limit of one check and were counted but not kept, by code.
    pub dropped: Vec<(&'static str, usize)>,
}

/// What the checks can claim: a failure is always a failure, but a pass only counts as confirmed
/// when no check was skipped, no layer was missing and the figures of the profile were verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoVerdict {
    /// At least one finding is an error.
    Failed,
    /// No error, but a check could not be done or the profile is not verified: not a confirmation.
    Inconclusive,
    /// No error and every check was done against a verified profile.
    Confirmed,
}

impl GeoVerdict {
    /// The word used in the JSON of a report.
    pub fn word(self) -> &'static str {
        match self {
            GeoVerdict::Failed => "failed",
            GeoVerdict::Inconclusive => "inconclusive",
            GeoVerdict::Confirmed => "confirmed",
        }
    }
}

impl GeoFinding {
    /// True for a finding that says a check could not be made (skipped, a layer missing or an
    /// unverified profile) instead of reporting something measured.
    pub fn is_inconclusive(&self) -> bool {
        matches!(
            self.code,
            text::SKIPPED | text::LAYER_MISSING | text::UNVERIFIED
        )
    }
}

impl GeoReport {
    /// True when no finding is an error.
    pub fn passed(&self) -> bool {
        self.findings
            .iter()
            .all(|f| f.severity != GeoSeverity::Error)
    }

    /// The findings that are measurements, not notes about checks that could not be made.
    pub fn confirmed_findings(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| !f.is_inconclusive())
            .count()
    }

    /// The findings that say a check could not be made.
    pub fn inconclusive_findings(&self) -> usize {
        self.findings.iter().filter(|f| f.is_inconclusive()).count()
    }

    /// The verdict: failed, inconclusive or confirmed (see [`GeoVerdict`]).
    pub fn verdict(&self) -> GeoVerdict {
        if !self.passed() {
            GeoVerdict::Failed
        } else if self.inconclusive_findings() > 0 || !self.profile_verified {
            GeoVerdict::Inconclusive
        } else {
            GeoVerdict::Confirmed
        }
    }

    /// The number of findings with a code.
    pub fn count(&self, code: &str) -> usize {
        self.findings.iter().filter(|f| f.code == code).count()
    }
}

struct Ctx<'a> {
    s: &'a GeoSettings,
    bo: BooleanOptions,
    oo: OffsetOptions,
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1.0e6
}

fn nm(mm: f64) -> i64 {
    if mm.is_finite() {
        (mm * 1.0e6).round() as i64
    } else {
        0
    }
}

fn point(p: P) -> Pt {
    (mm(p.x), mm(p.y))
}

fn rotate(p: Pt, degrees: f64) -> Pt {
    let a = degrees.to_radians();
    (p.0 * a.cos() - p.1 * a.sin(), p.0 * a.sin() + p.1 * a.cos())
}

fn disc(ctx: &Ctx, c: Pt, r: f64) -> Region {
    let tol = mm(ctx.s.offset_tolerance_nm).max(1e-9);
    let sides = if r > tol {
        let step = 2.0 * (1.0 - tol / r).clamp(-1.0, 1.0).acos();
        ((std::f64::consts::TAU / step).ceil() as usize).clamp(12, 256)
    } else {
        12
    };
    Region::regular_polygon(c, r, sides)
}

fn rect(c: Pt, w: f64, h: f64) -> Region {
    Region::rectangle(c.0 - w / 2.0, c.1 - h / 2.0, c.0 + w / 2.0, c.1 + h / 2.0)
}

fn rotated_rect(c: Pt, w: f64, h: f64, degrees: f64) -> Region {
    let corners = [
        (-w / 2.0, -h / 2.0),
        (w / 2.0, -h / 2.0),
        (w / 2.0, h / 2.0),
        (-w / 2.0, h / 2.0),
    ];
    Region::from_ring(
        corners
            .iter()
            .map(|p| {
                let r = rotate(*p, degrees);
                (r.0 + c.0, r.1 + c.1)
            })
            .collect(),
    )
}

fn macro_region(ctx: &Ctx, shapes: &[MacroShape], at: Pt) -> Result<Region, String> {
    let mut acc = Region::new();
    for shape in shapes {
        let (exposure, region) = match shape {
            MacroShape::Circle {
                exposure,
                diameter,
                centre,
                rotation,
            } => {
                let c = rotate(point(*centre), *rotation as f64 / 1.0e6);
                (
                    *exposure,
                    disc(ctx, (c.0 + at.0, c.1 + at.1), mm(*diameter) / 2.0),
                )
            }
            MacroShape::CenterLine {
                exposure,
                width,
                height,
                centre,
                rotation,
            } => {
                let deg = *rotation as f64 / 1.0e6;
                let c = rotate(point(*centre), deg);
                (
                    *exposure,
                    rotated_rect((c.0 + at.0, c.1 + at.1), mm(*width), mm(*height), deg),
                )
            }
            MacroShape::VectorLine {
                exposure,
                width,
                start,
                end,
                rotation,
            } => {
                let deg = *rotation as f64 / 1.0e6;
                let (a, b) = (rotate(point(*start), deg), rotate(point(*end), deg));
                let length = (b.0 - a.0).hypot(b.1 - a.1);
                let angle = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
                let mid = ((a.0 + b.0) / 2.0 + at.0, (a.1 + b.1) / 2.0 + at.1);
                (*exposure, rotated_rect(mid, length, mm(*width), angle))
            }
            MacroShape::Outline {
                exposure,
                points,
                rotation,
            } => {
                let deg = *rotation as f64 / 1.0e6;
                let mut ring: Vec<Pt> = points
                    .iter()
                    .map(|p| {
                        let r = rotate(point(*p), deg);
                        (r.0 + at.0, r.1 + at.1)
                    })
                    .collect();
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                (*exposure, Region::from_ring(ring))
            }
            MacroShape::Polygon {
                exposure,
                vertices,
                centre,
                diameter,
                rotation,
            } => {
                let deg = *rotation as f64 / 1.0e6;
                let c = rotate(point(*centre), deg);
                let n = (*vertices).max(3) as usize;
                let r = mm(*diameter) / 2.0;
                let ring: Vec<Pt> = (0..n)
                    .map(|i| {
                        let a = deg.to_radians() + std::f64::consts::TAU * i as f64 / n as f64;
                        (c.0 + at.0 + r * a.cos(), c.1 + at.1 + r * a.sin())
                    })
                    .collect();
                (*exposure, Region::from_ring(ring))
            }
        };
        acc = if exposure {
            union(&acc, &region, &ctx.bo)
        } else {
            difference(&acc, &region, &ctx.bo)
        }
        .map_err(|e| e.to_string())?;
    }
    Ok(acc)
}

fn aperture_region(ctx: &Ctx, ap: &Aperture, at: P) -> Result<Region, String> {
    let c = point(at);
    let base = match &ap.shape {
        ApertureShape::Circle { diameter } => disc(ctx, c, mm(*diameter) / 2.0),
        ApertureShape::Rect { width, height } => rect(c, mm(*width), mm(*height)),
        ApertureShape::Obround { width, height } => {
            let (w, h) = (mm(*width), mm(*height));
            let path = if w >= h {
                [(c.0 - (w - h) / 2.0, c.1), (c.0 + (w - h) / 2.0, c.1)]
            } else {
                [(c.0, c.1 - (h - w) / 2.0), (c.0, c.1 + (h - w) / 2.0)]
            };
            stroke(&path, w.min(h), false, &ctx.oo).map_err(|e| e.to_string())?
        }
        ApertureShape::Polygon {
            diameter,
            vertices,
            rotation,
        } => {
            let n = (*vertices).max(3) as usize;
            let r = mm(*diameter) / 2.0;
            let start = (*rotation as f64 / 1.0e6).to_radians();
            Region::from_ring(
                (0..n)
                    .map(|i| {
                        let a = start + std::f64::consts::TAU * i as f64 / n as f64;
                        (c.0 + r * a.cos(), c.1 + r * a.sin())
                    })
                    .collect(),
            )
        }
        ApertureShape::Macro { shapes, .. } => macro_region(ctx, shapes, c)?,
    };
    match ap.hole {
        Some(hole) if hole > 0 => {
            let cut = disc(ctx, c, mm(hole) / 2.0);
            difference(&base, &cut, &ctx.bo).map_err(|e| e.to_string())
        }
        _ => Ok(base),
    }
}

/// The points of an arc from `from` to `to` round `centre`, cut so that no piece lies farther than
/// the chord tolerance from it.
fn arc_points(from: P, to: P, centre: P, clockwise: bool, chord_nm: i64) -> Vec<Pt> {
    let (f, t, c) = (point(from), point(to), point(centre));
    let radius = (f.0 - c.0).hypot(f.1 - c.1);
    if radius <= 0.0 || !radius.is_finite() {
        return vec![f, t];
    }
    let a0 = (f.1 - c.1).atan2(f.0 - c.0);
    let a1 = (t.1 - c.1).atan2(t.0 - c.0);
    let tau = std::f64::consts::TAU;
    let mut sweep = a1 - a0;
    if clockwise {
        while sweep >= 0.0 {
            sweep -= tau;
        }
    } else {
        while sweep <= 0.0 {
            sweep += tau;
        }
    }
    if from == to {
        sweep = if clockwise { -tau } else { tau };
    }
    let tolerance = mm(chord_nm.max(1)).min(radius);
    let step = 2.0 * (1.0 - tolerance / radius).clamp(-1.0, 1.0).acos();
    let sides = if step > 0.0 {
        (sweep.abs() / step).ceil().clamp(1.0, 4096.0) as usize
    } else {
        4096
    };
    let mut out = Vec::with_capacity(sides + 1);
    out.push(f);
    for i in 1..sides {
        let a = a0 + sweep * i as f64 / sides as f64;
        out.push((c.0 + radius * a.cos(), c.1 + radius * a.sin()));
    }
    out.push(t);
    out
}

fn flush(ctx: &Ctx, acc: &mut Region, pending: &mut Vec<Region>, dark: bool) -> Result<(), String> {
    if pending.is_empty() {
        return Ok(());
    }
    let joined = union_all(pending, &ctx.bo).map_err(|e| e.to_string())?;
    pending.clear();
    *acc = if dark {
        union(acc, &joined, &ctx.bo)
    } else {
        difference(acc, &joined, &ctx.bo)
    }
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// What a copper layer is made of, for the checks that need more than its shape.
#[derive(Default)]
struct Anchors {
    points: Vec<Pt>,
}

/// The region a Gerber layer draws, with clear polarity taken off in the order of the file, and
/// the points that hold copper in place (flashes and the ends of lines).
fn layer_region(ctx: &Ctx, layer: &GerberLayer) -> Result<(Region, Anchors), String> {
    let mut acc = Region::new();
    let mut pending: Vec<Region> = Vec::new();
    let mut pending_dark = true;
    let mut anchors = Anchors::default();
    for item in &layer.items {
        let (dark, region) = match item {
            Item::Flash { at, aperture, dark } => {
                anchors.points.push(point(*at));
                let Some(ap) = layer.apertures.get(aperture) else {
                    continue;
                };
                (*dark, aperture_region(ctx, ap, *at)?)
            }
            Item::Line {
                from,
                to,
                width,
                dark,
            } => {
                anchors.points.push(point(*from));
                anchors.points.push(point(*to));
                if *width <= 0 {
                    continue;
                }
                let path = [point(*from), point(*to)];
                (
                    *dark,
                    stroke(&path, mm(*width), false, &ctx.oo).map_err(|e| e.to_string())?,
                )
            }
            Item::Arc {
                from,
                to,
                centre,
                clockwise,
                width,
                dark,
            } => {
                anchors.points.push(point(*from));
                anchors.points.push(point(*to));
                if *width <= 0 {
                    continue;
                }
                let path = arc_points(*from, *to, *centre, *clockwise, ctx.s.arc_chord_nm);
                (
                    *dark,
                    stroke(&path, mm(*width), false, &ctx.oo).map_err(|e| e.to_string())?,
                )
            }
            Item::Region { points, dark } => {
                if points.len() < 3 {
                    continue;
                }
                (
                    *dark,
                    Region::from_ring(points.iter().map(|p| point(*p)).collect()),
                )
            }
        };
        if dark != pending_dark {
            flush(ctx, &mut acc, &mut pending, pending_dark)?;
            pending_dark = dark;
        }
        pending.push(region);
    }
    flush(ctx, &mut acc, &mut pending, pending_dark)?;
    Ok((acc, anchors))
}

/// A place a check found: the middle of it, its width (an estimate) and its area, in millimetres.
struct Spot {
    x: f64,
    y: f64,
    width: f64,
    area: f64,
}

fn perimeter(ring: &[Pt]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            (a.0 - b.0).hypot(a.1 - b.1)
        })
        .sum()
}

fn bounds_of(ring: &[Pt]) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in ring {
        b.0 = b.0.min(p.0);
        b.1 = b.1.min(p.1);
        b.2 = b.2.max(p.0);
        b.3 = b.3.max(p.1);
    }
    b
}

/// The outer rings of a region (counterclockwise) that have an area of at least `floor`.
fn spots_of(region: &Region, floor: f64) -> Vec<Spot> {
    let mut out = Vec::new();
    for ring in &region.rings {
        if ring.len() < 3 || signed_area(ring) <= 0.0 {
            continue;
        }
        let area = signed_area(ring);
        if area < floor {
            continue;
        }
        let b = bounds_of(ring);
        let p = perimeter(ring);
        out.push(Spot {
            x: (b.0 + b.2) / 2.0,
            y: (b.1 + b.3) / 2.0,
            width: if p > 0.0 { 2.0 * area / p } else { 0.0 },
            area,
        });
    }
    out.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.area.total_cmp(&b.area))
    });
    out
}

/// How a drawn thing grows: tracks and round pads exactly (a wider track, a larger disc), anything
/// else by the offset of its region.
enum Grow {
    Shape(Region),
    Disc(Pt, f64),
    Stroke(Vec<Pt>, f64),
}

/// One drawn thing of a layer with its box, kept apart so that two things can be compared without
/// offsetting a thin region (the offset of a thin track with round ends is not reliable).
struct Prim {
    grow: Grow,
    bbox: (f64, f64, f64, f64),
    /// The width of a track or an arc, in nanometres.
    width_nm: Option<i64>,
    mid: Pt,
}

fn grown(ctx: &Ctx, prim: &Prim, r: f64) -> Result<Region, String> {
    match &prim.grow {
        Grow::Disc(c, rad) => Ok(disc(ctx, *c, rad + r)),
        Grow::Stroke(path, w) => {
            stroke(path, w + 2.0 * r, false, &ctx.oo).map_err(|e| e.to_string())
        }
        Grow::Shape(region) => offset(region, r, &ctx.oo).map_err(|e| e.to_string()),
    }
}

fn prim_of(grow: Grow, width_nm: Option<i64>) -> Prim {
    let (bbox, mid) = match &grow {
        Grow::Disc(c, rad) => ((c.0 - rad, c.1 - rad, c.0 + rad, c.1 + rad), *c),
        Grow::Stroke(path, w) => {
            let b = bounds_of(path);
            let mid = match (path.first(), path.last()) {
                (Some(a), Some(z)) => ((a.0 + z.0) / 2.0, (a.1 + z.1) / 2.0),
                _ => (0.0, 0.0),
            };
            (
                (b.0 - w / 2.0, b.1 - w / 2.0, b.2 + w / 2.0, b.3 + w / 2.0),
                mid,
            )
        }
        Grow::Shape(region) => {
            let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for ring in &region.rings {
                let r = bounds_of(ring);
                b = (b.0.min(r.0), b.1.min(r.1), b.2.max(r.2), b.3.max(r.3));
            }
            if b.0 > b.2 {
                b = (0.0, 0.0, 0.0, 0.0);
            }
            (b, ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0))
        }
    };
    Prim {
        grow,
        bbox,
        width_nm,
        mid,
    }
}

/// The dark things of a layer, one by one, and whether the layer also has clear polarity (then the
/// things alone do not tell what copper is left).
fn layer_prims(ctx: &Ctx, layer: &GerberLayer) -> Result<(Vec<Prim>, bool), String> {
    let mut out = Vec::new();
    let mut has_clear = false;
    for item in &layer.items {
        match item {
            Item::Flash { at, aperture, dark } => {
                if !*dark {
                    has_clear = true;
                    continue;
                }
                let Some(ap) = layer.apertures.get(aperture) else {
                    continue;
                };
                let c = point(*at);
                let grow = match (&ap.shape, ap.hole) {
                    (ApertureShape::Circle { diameter }, None) => {
                        Grow::Disc(c, mm(*diameter) / 2.0)
                    }
                    (ApertureShape::Obround { width, height }, None) => {
                        let (w, h) = (mm(*width), mm(*height));
                        let path = if w >= h {
                            vec![(c.0 - (w - h) / 2.0, c.1), (c.0 + (w - h) / 2.0, c.1)]
                        } else {
                            vec![(c.0, c.1 - (h - w) / 2.0), (c.0, c.1 + (h - w) / 2.0)]
                        };
                        Grow::Stroke(path, w.min(h))
                    }
                    _ => Grow::Shape(aperture_region(ctx, ap, *at)?),
                };
                out.push(prim_of(grow, None));
            }
            Item::Line {
                from,
                to,
                width,
                dark,
            } => {
                if !*dark {
                    has_clear = true;
                } else if *width > 0 {
                    out.push(prim_of(
                        Grow::Stroke(vec![point(*from), point(*to)], mm(*width)),
                        Some(*width),
                    ));
                }
            }
            Item::Arc {
                from,
                to,
                centre,
                clockwise,
                width,
                dark,
            } => {
                if !*dark {
                    has_clear = true;
                } else if *width > 0 {
                    let path = arc_points(*from, *to, *centre, *clockwise, ctx.s.arc_chord_nm);
                    out.push(prim_of(Grow::Stroke(path, mm(*width)), Some(*width)));
                }
            }
            Item::Region { points, dark } => {
                if !*dark {
                    has_clear = true;
                } else if points.len() >= 3 {
                    let ring: Vec<Pt> = points.iter().map(|p| point(*p)).collect();
                    out.push(prim_of(Grow::Shape(Region::from_ring(ring)), None));
                }
            }
        }
    }
    Ok((out, has_clear))
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Where two separate things are closer than `limit` (millimetres). Things that touch or overlap
/// are one (and so is everything they connect), whatever net it belongs to. Each thing is grown by
/// half the limit; where two grown things overlap and the things are apart, the width of the
/// overlap is what the limit exceeds the gap by.
fn near_pairs(ctx: &Ctx, prims: &[Prim], limit: f64) -> Result<Vec<Spot>, String> {
    let half = limit / 2.0;
    let touch = mm(ctx.s.touch_nm.max(1)) / 2.0;
    let mut order: Vec<usize> = (0..prims.len()).collect();
    order.sort_by(|a, b| prims[*a].bbox.0.total_cmp(&prims[*b].bbox.0));
    let mut parent: Vec<usize> = (0..prims.len()).collect();
    let mut near: Vec<(usize, usize, Region)> = Vec::new();
    let mut tests = 0usize;
    for (k, &i) in order.iter().enumerate() {
        let a = &prims[i];
        for &j in &order[k + 1..] {
            let b = &prims[j];
            if b.bbox.0 > a.bbox.2 + limit {
                break;
            }
            if b.bbox.1 > a.bbox.3 + limit || b.bbox.3 < a.bbox.1 - limit {
                continue;
            }
            tests += 1;
            if tests > ctx.s.max_pair_tests {
                return Err(format!(
                    "more than {} pairs of things close to each other",
                    ctx.s.max_pair_tests
                ));
            }
            let (ga, gb) = (grown(ctx, a, half)?, grown(ctx, b, half)?);
            let overlap = intersection(&ga, &gb, &ctx.bo).map_err(|e| e.to_string())?;
            if overlap.is_empty() || overlap.area() <= 0.0 {
                continue;
            }
            let (ta, tb) = (grown(ctx, a, touch)?, grown(ctx, b, touch)?);
            let tight = intersection(&ta, &tb, &ctx.bo).map_err(|e| e.to_string())?;
            if !tight.is_empty() && tight.area() > 0.0 {
                let (ra, rb) = (find(&mut parent, i), find(&mut parent, j));
                parent[ra] = rb;
            } else {
                near.push((i, j, overlap));
            }
        }
    }
    let mut out = Vec::new();
    let floor = area_floor(ctx.s);
    for (i, j, overlap) in near {
        if find(&mut parent, i) == find(&mut parent, j) {
            continue;
        }
        for spot in spots_of(&overlap, floor) {
            out.push(Spot {
                x: spot.x,
                y: spot.y,
                width: (limit - spot.width).max(0.0),
                area: spot.area,
            });
        }
    }
    out.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    Ok(out)
}

/// The tracks and arcs narrower than `limit_nm`, each one a spot with its own width.
fn narrow_tracks(prims: &[Prim], limit_nm: i64) -> Vec<Spot> {
    prims
        .iter()
        .filter_map(|p| {
            let w = p.width_nm?;
            (w < limit_nm).then_some(Spot {
                x: p.mid.0,
                y: p.mid.1,
                width: mm(w),
                area: 0.0,
            })
        })
        .collect()
}

struct Collector<'a> {
    s: &'a GeoSettings,
    findings: Vec<GeoFinding>,
    dropped: Vec<(&'static str, usize)>,
}

impl Collector<'_> {
    fn push(&mut self, finding: GeoFinding) {
        let kept = self
            .findings
            .iter()
            .filter(|f| f.code == finding.code)
            .count();
        if kept >= self.s.max_findings {
            match self.dropped.iter_mut().find(|(c, _)| *c == finding.code) {
                Some(entry) => entry.1 += 1,
                None => self.dropped.push((finding.code, 1)),
            }
        } else {
            self.findings.push(finding);
        }
    }

    fn spots(
        &mut self,
        code: &'static str,
        layer: &str,
        spots: &[Spot],
        limit_nm: i64,
        measured: impl Fn(&Spot) -> f64,
    ) {
        for spot in spots {
            self.push(GeoFinding {
                code,
                severity: GeoSeverity::Error,
                layer: layer.to_owned(),
                x_nm: nm(spot.x),
                y_nm: nm(spot.y),
                measured_nm: nm(measured(spot)).max(0),
                limit_nm,
                detail: String::new(),
            });
        }
    }

    fn skipped(&mut self, layer: &str, check: &str, why: &str) {
        self.push(GeoFinding {
            code: text::SKIPPED,
            severity: GeoSeverity::Warning,
            layer: layer.to_owned(),
            x_nm: 0,
            y_nm: 0,
            measured_nm: 0,
            limit_nm: 0,
            detail: format!("{check}: {why}"),
        });
    }

    fn missing(&mut self, what: &str) {
        self.push(GeoFinding {
            code: text::LAYER_MISSING,
            severity: GeoSeverity::Warning,
            layer: what.to_owned(),
            x_nm: 0,
            y_nm: 0,
            measured_nm: 0,
            limit_nm: 0,
            detail: String::new(),
        });
    }
}

struct Hole {
    at: Pt,
    d_nm: i64,
    plated: bool,
    via: bool,
}

struct Side {
    name: String,
    copper: Region,
    anchors: Anchors,
    prims: Vec<Prim>,
    has_clear: bool,
}

struct Layer {
    region: Region,
    prims: Vec<Prim>,
    has_clear: bool,
}

/// Read the layers of a set of files and check them against a profile.
pub fn check_set(set: &ImportedSet, profile: &DfmProfile, s: &GeoSettings) -> GeoReport {
    let ctx = Ctx {
        s,
        bo: BooleanOptions {
            grid: mm(s.grid_nm).max(1e-9),
            max_edges: s.max_edges,
        },
        oo: OffsetOptions {
            tolerance: mm(s.offset_tolerance_nm).max(1e-9),
            boolean: BooleanOptions {
                grid: mm(s.grid_nm).max(1e-9),
                max_edges: s.max_edges,
            },
        },
    };
    let mut col = Collector {
        s,
        findings: Vec::new(),
        dropped: Vec::new(),
    };
    if !profile.verified {
        col.push(GeoFinding {
            code: text::UNVERIFIED,
            severity: GeoSeverity::Info,
            layer: String::new(),
            x_nm: 0,
            y_nm: 0,
            measured_nm: 0,
            limit_nm: 0,
            detail: String::new(),
        });
    }
    let mut coppers: Vec<Side> = Vec::new();
    let mut top: Option<usize> = None;
    let mut bottom: Option<usize> = None;
    let mut masks: [Option<Layer>; 2] = [None, None];
    let mut silks: [Option<Region>; 2] = [None, None];
    let mut outline: Option<(String, Layer)> = None;
    let mut holes: Vec<Hole> = Vec::new();
    let mut files = 0usize;
    for file in &set.files {
        let Some(id) = file.identified else { continue };
        match (&file.content, id.role) {
            (Content::Gerber(layer), role) => {
                files += 1;
                let made = layer_region(&ctx, layer);
                let (region, anchors) = match made {
                    Ok(v) => v,
                    Err(why) => {
                        col.skipped(&file.name, "read", &why);
                        continue;
                    }
                };
                let (prims, has_clear) = match layer_prims(&ctx, layer) {
                    Ok(v) => v,
                    Err(why) => {
                        col.skipped(&file.name, "read", &why);
                        continue;
                    }
                };
                match role {
                    Role::CopperTop | Role::CopperBottom | Role::CopperInner(_) => {
                        if role == Role::CopperTop {
                            top = Some(coppers.len());
                        }
                        if role == Role::CopperBottom {
                            bottom = Some(coppers.len());
                        }
                        coppers.push(Side {
                            name: file.name.clone(),
                            copper: region,
                            anchors,
                            prims,
                            has_clear,
                        });
                    }
                    Role::MaskTop => {
                        masks[0] = Some(Layer {
                            region,
                            prims,
                            has_clear,
                        })
                    }
                    Role::MaskBottom => {
                        masks[1] = Some(Layer {
                            region,
                            prims,
                            has_clear,
                        })
                    }
                    Role::LegendTop => silks[0] = Some(region),
                    Role::LegendBottom => silks[1] = Some(region),
                    Role::Outline => {
                        outline = Some((
                            file.name.clone(),
                            Layer {
                                region,
                                prims,
                                has_clear,
                            },
                        ))
                    }
                    _ => {}
                }
            }
            (Content::Drill(drill), role) => {
                files += 1;
                let plated = matches!(role, Role::DrillPlated | Role::Drill);
                for hit in &drill.hits {
                    let d = drill.diameter(hit.tool);
                    let via_attr = drill
                        .tool_attributes
                        .get(&hit.tool)
                        .and_then(|a| a.get(".AperFunction"))
                        .is_some_and(|v| v.iter().any(|x| x == "ViaDrill"));
                    holes.push(Hole {
                        at: point(hit.at),
                        d_nm: d,
                        plated,
                        via: via_attr || (plated && d > 0 && d <= s.via_max_drill_nm),
                    });
                }
            }
            (Content::Failed(_), _) => {}
        }
    }
    if holes.len() > s.max_holes {
        col.skipped(
            "",
            "holes",
            &format!("{} holes, the limit is {}", holes.len(), s.max_holes),
        );
        holes.truncate(s.max_holes);
    }
    let rules = &profile.rules;
    let clearance: i64 = rules.min_clearance.into();
    let track: i64 = rules.min_track_width.into();
    let ring_min: i64 = rules.min_annular_ring.into();
    let edge_min: i64 = rules.min_edge_clearance.into();
    let bridge_min: i64 = rules.min_mask_bridge.into();
    if coppers.is_empty() {
        col.missing("copper");
    }
    // spacing and width on every copper layer
    for side in &coppers {
        if clearance > 0 {
            if side.has_clear {
                col.skipped(&side.name, "spacing", "the layer has clear polarity");
            } else {
                match near_pairs(&ctx, &side.prims, mm(clearance)) {
                    Ok(spots) => {
                        col.spots(text::SPACING, &side.name, &spots, clearance, |p| p.width)
                    }
                    Err(why) => col.skipped(&side.name, "spacing", &why),
                }
            }
        }
        if track > 0 {
            let spots = narrow_tracks(&side.prims, track);
            col.spots(text::WIDTH, &side.name, &spots, track, |p| p.width);
        }
    }
    // the ring of copper round a plated hole, on the outer layers
    if ring_min > 0 {
        let plated: Vec<&Hole> = holes.iter().filter(|h| h.plated && h.d_nm > 0).collect();
        for index in [top, bottom].into_iter().flatten() {
            ring_check(&ctx, &mut col, &coppers[index], &plated, ring_min);
        }
    }
    // copper at the edge
    if edge_min > 0 {
        match &outline {
            None => col.missing("outline"),
            Some((name, layer)) => {
                for side in &coppers {
                    edge_check(&ctx, &mut col, side, name, layer, edge_min);
                }
            }
        }
    }
    // the bridge of mask, the legend over a pad, the via in a pad
    for (n, index) in [top, bottom].into_iter().enumerate() {
        let Some(index) = index else { continue };
        let side = &coppers[index];
        if let Some(mask) = &masks[n] {
            if bridge_min > 0 {
                if mask.has_clear {
                    col.skipped(&side.name, "mask bridge", "the layer has clear polarity");
                } else {
                    match near_pairs(&ctx, &mask.prims, mm(bridge_min)) {
                        Ok(spots) => {
                            col.spots(text::MASK_BRIDGE, &side.name, &spots, bridge_min, |p| {
                                p.width
                            })
                        }
                        Err(why) => col.skipped(&side.name, "mask bridge", &why),
                    }
                }
            }
            let exposed = match intersection(&side.copper, &mask.region, &ctx.bo) {
                Ok(r) => r,
                Err(e) => {
                    col.skipped(&side.name, "exposed copper", &e.to_string());
                    continue;
                }
            };
            if rules.silkscreen_over_pads {
                if let Some(silk) = &silks[n] {
                    match intersection(silk, &exposed, &ctx.bo) {
                        Ok(over) => {
                            let spots = spots_of(&over, area_floor(s));
                            col.spots(text::SILK_ON_PAD, &side.name, &spots, 0, |p| p.width);
                        }
                        Err(e) => col.skipped(&side.name, "legend over pads", &e.to_string()),
                    }
                }
            }
            if profile.forbid_via_in_pad {
                for hole in holes.iter().filter(|h| h.via) {
                    if exposed.contains(hole.at) == Containment::Inside {
                        col.push(GeoFinding {
                            code: text::VIA_IN_PAD,
                            severity: GeoSeverity::Error,
                            layer: side.name.clone(),
                            x_nm: nm(hole.at.0),
                            y_nm: nm(hole.at.1),
                            measured_nm: hole.d_nm,
                            limit_nm: 0,
                            detail: String::new(),
                        });
                    }
                }
            }
        }
    }
    // floating copper on the outer layers
    for index in [top, bottom].into_iter().flatten() {
        floating(&mut col, &coppers[index], &holes, s);
    }
    GeoReport {
        profile: profile.name.clone(),
        profile_verified: profile.verified,
        files,
        findings: col.findings,
        dropped: col.dropped,
    }
}

fn area_floor(s: &GeoSettings) -> f64 {
    s.area_floor_nm2 as f64 / 1.0e12
}

fn ring_check(ctx: &Ctx, col: &mut Collector, side: &Side, holes: &[&Hole], ring_nm: i64) {
    if holes.is_empty() {
        return;
    }
    let register = ctx.s.registration_nm;
    let need = |h: &Hole| mm(h.d_nm + 2 * (ring_nm + register)) / 2.0;
    let discs: Vec<Region> = holes.iter().map(|h| disc(ctx, h.at, need(h))).collect();
    let all = match union_all(&discs, &ctx.bo) {
        Ok(r) => r,
        Err(e) => {
            col.skipped(&side.name, "annular ring", &e.to_string());
            return;
        }
    };
    let bad = match difference(&all, &side.copper, &ctx.bo) {
        Ok(r) => r,
        Err(e) => {
            col.skipped(&side.name, "annular ring", &e.to_string());
            return;
        }
    };
    for spot in spots_of(&bad, area_floor(ctx.s)) {
        // the hole whose disc holds the middle of the find, or the nearest
        let near = holes.iter().min_by(|a, b| {
            let da = (a.at.0 - spot.x).hypot(a.at.1 - spot.y);
            let db = (b.at.0 - spot.x).hypot(b.at.1 - spot.y);
            da.total_cmp(&db)
        });
        let Some(hole) = near else { continue };
        let ring = worst_ring(ctx, side, hole, need(hole));
        col.push(GeoFinding {
            code: text::RING,
            severity: GeoSeverity::Error,
            layer: side.name.clone(),
            x_nm: nm(hole.at.0),
            y_nm: nm(hole.at.1),
            measured_nm: ring.max(0),
            limit_nm: ring_nm,
            detail: String::new(),
        });
    }
}

/// The ring of copper that is left round a hole when the drill is off by the registration error:
/// the largest circle round the hole that is all copper, less the drill radius and the error.
fn worst_ring(ctx: &Ctx, side: &Side, hole: &Hole, reach: f64) -> i64 {
    let window = rect(hole.at, reach * 3.0, reach * 3.0);
    let Ok(local) = intersection(&side.copper, &window, &ctx.bo) else {
        return 0;
    };
    let radius_drill = mm(hole.d_nm) / 2.0;
    let (mut lo, mut hi) = (0.0_f64, reach);
    for _ in 0..14 {
        let mid = (lo + hi) / 2.0;
        let probe = disc(ctx, hole.at, mid);
        let outside = difference(&probe, &local, &ctx.bo)
            .map(|r| r.area())
            .unwrap_or(1.0);
        if outside > area_floor(ctx.s) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    nm(lo - radius_drill) - ctx.s.registration_nm
}

fn edge_check(ctx: &Ctx, col: &mut Collector, side: &Side, name: &str, layer: &Layer, limit: i64) {
    let mut pieces = Vec::new();
    for prim in &layer.prims {
        match grown(ctx, prim, mm(limit)) {
            Ok(r) => pieces.push(r),
            Err(why) => {
                col.skipped(name, "copper at the edge", &why);
                return;
            }
        }
    }
    let keepout = match union_all(&pieces, &ctx.bo) {
        Ok(r) => r,
        Err(e) => {
            col.skipped(name, "copper at the edge", &e.to_string());
            return;
        }
    };
    match intersection(&side.copper, &keepout, &ctx.bo) {
        Ok(over) => {
            let limit_mm = mm(limit);
            let spots = spots_of(&over, area_floor(ctx.s));
            col.spots(text::EDGE, &side.name, &spots, limit, |p| {
                (limit_mm - p.width).max(0.0)
            });
        }
        Err(e) => col.skipped(&side.name, "copper at the edge", &e.to_string()),
    }
}

fn floating(col: &mut Collector, side: &Side, holes: &[Hole], s: &GeoSettings) {
    let mut points: Vec<Pt> = side.anchors.points.clone();
    points.extend(holes.iter().map(|h| h.at));
    if points.len() > s.max_anchors {
        col.skipped(
            &side.name,
            "floating copper",
            &format!(
                "{} anchor points, the limit is {}",
                points.len(),
                s.max_anchors
            ),
        );
        return;
    }
    for ring in &side.copper.rings {
        if ring.len() < 3 || signed_area(ring) <= 0.0 {
            continue;
        }
        let b = bounds_of(ring);
        let held = points.iter().any(|p| {
            p.0 >= b.0
                && p.0 <= b.2
                && p.1 >= b.1
                && p.1 <= b.3
                && eh3d_geometry::polygon::ring_contains(ring, *p) != Containment::Outside
        });
        if !held {
            col.push(GeoFinding {
                code: text::FLOATING,
                severity: GeoSeverity::Warning,
                layer: side.name.clone(),
                x_nm: nm((b.0 + b.2) / 2.0),
                y_nm: nm((b.1 + b.3) / 2.0),
                measured_nm: 0,
                limit_nm: 0,
                detail: format!("{:.3} mm2", signed_area(ring)),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identify::{Identified, Source};
    use crate::import::ImportedFile;
    use crate::parse::parse_gerber;
    use crate::settings::ImportSettings;

    const HEAD: &str = "%FSLAX46Y46*%\n%MOMM*%\n";

    fn gerber(body: &str) -> GerberLayer {
        parse_gerber(&format!("{HEAD}{body}M02*\n"), &ImportSettings::default()).unwrap()
    }

    fn file(name: &str, role: Role, body: &str) -> ImportedFile {
        ImportedFile {
            name: name.to_owned(),
            identified: Some(Identified {
                role,
                source: Source::FileName,
            }),
            content: Content::Gerber(gerber(body)),
        }
    }

    fn profile(text: &str) -> DfmProfile {
        DfmProfile::parse(text).unwrap()
    }

    /// A rectangle track of a given width along X, as a region of the file.
    fn track(x0: i64, y0: i64, x1: i64, width_um: i64) -> String {
        format!(
            "%ADD10C,{:.6}*%\nD10*\nX{}Y{}D02*\nX{}Y{}D01*\n",
            width_um as f64 / 1000.0,
            x0,
            y0,
            x1,
            y0
        )
    }

    fn run(files: Vec<ImportedFile>, profile_text: &str) -> GeoReport {
        check_set(
            &ImportedSet { files },
            &profile(profile_text),
            &GeoSettings::standard(),
        )
    }

    #[test]
    fn the_standard_settings_are_read_and_a_bad_text_is_refused() {
        let s = GeoSettings::standard();
        assert_eq!(s.grid_nm, 1);
        assert_eq!(s.max_findings, 100);
        assert!(GeoSettings::parse("grid_nm = x").is_err());
        assert!(GeoSettings::parse("nothing = 1")
            .unwrap_err()
            .contains("not a known key"));
        assert!(GeoSettings::parse("grid_nm = 1\ngrid_nm = 2")
            .unwrap_err()
            .contains("twice"));
        assert!(GeoSettings::parse("offset_tolerance_nm = 0").is_err());
    }

    #[test]
    fn two_tracks_closer_than_the_limit_are_found_and_farther_ones_are_not() {
        let body = format!(
            "{}{}",
            track(1_000_000, 1_000_000, 9_000_000, 200),
            "X1000000Y1300000D02*\nX9000000Y1300000D01*\n"
        );
        // the tracks are 0.2 mm wide with 0.1 mm between them (centres 0.3 mm apart)
        let close = run(
            vec![file("top.gtl", Role::CopperTop, &body)],
            "min_clearance_nm = 150000\nverified = true\n",
        );
        assert_eq!(close.count(text::SPACING), 1, "{:?}", close.findings);
        let f = &close.findings[0];
        assert!(
            (f.measured_nm - 100_000).abs() < 25_000,
            "{}",
            f.measured_nm
        );
        assert!(!close.passed());
        let ok = run(
            vec![file("top.gtl", Role::CopperTop, &body)],
            "min_clearance_nm = 90000\nverified = true\n",
        );
        assert_eq!(ok.count(text::SPACING), 0, "{:?}", ok.findings);
        assert!(ok.passed());
    }

    #[test]
    fn a_track_narrower_than_the_limit_is_found() {
        let body = track(1_000_000, 1_000_000, 9_000_000, 100);
        let r = run(
            vec![file("top.gtl", Role::CopperTop, &body)],
            "min_track_width_nm = 150000\nverified = true\n",
        );
        assert_eq!(r.count(text::WIDTH), 1, "{:?}", r.findings);
        assert!((r.findings[0].measured_nm - 100_000).abs() < 25_000);
        let ok = run(
            vec![file("top.gtl", Role::CopperTop, &body)],
            "min_track_width_nm = 90000\nverified = true\n",
        );
        assert_eq!(ok.count(text::WIDTH), 0, "{:?}", ok.findings);
    }

    #[test]
    fn the_unverified_profile_is_said_and_does_not_fail_the_files() {
        let r = run(
            vec![file(
                "top.gtl",
                Role::CopperTop,
                &track(1_000_000, 1_000_000, 9_000_000, 300),
            )],
            "min_track_width_nm = 100000\n",
        );
        assert_eq!(r.count(text::UNVERIFIED), 1);
        assert!(r.passed());
        assert!(!r.profile_verified);
    }

    #[test]
    fn copper_at_the_edge_is_found() {
        let outline = "%ADD10C,0.100000*%\nD10*\nX0Y0D02*\nX20000000Y0D01*\nX20000000Y20000000D01*\nX0Y20000000D01*\nX0Y0D01*\n";
        let copper = track(5_000_000, 100_000, 15_000_000, 300);
        let r = run(
            vec![
                file("edge.gko", Role::Outline, outline),
                file("top.gtl", Role::CopperTop, &copper),
            ],
            "min_edge_clearance_nm = 300000\nverified = true\n",
        );
        assert_eq!(r.count(text::EDGE), 1, "{:?}", r.findings);
        let far = track(5_000_000, 5_000_000, 15_000_000, 300);
        let ok = run(
            vec![
                file("edge.gko", Role::Outline, outline),
                file("top.gtl", Role::CopperTop, &far),
            ],
            "min_edge_clearance_nm = 300000\nverified = true\n",
        );
        assert_eq!(ok.count(text::EDGE), 0, "{:?}", ok.findings);
        // no outline: said, not guessed
        let none = run(
            vec![file("top.gtl", Role::CopperTop, &copper)],
            "min_edge_clearance_nm = 300000\nverified = true\n",
        );
        assert_eq!(none.count(text::LAYER_MISSING), 1);
    }

    #[test]
    fn a_bridge_of_mask_narrower_than_the_limit_is_found() {
        let copper =
            "%ADD10R,0.600000X0.600000*%\nD10*\nX5000000Y5000000D03*\nX5700000Y5000000D03*\n";
        // openings 0.7 mm wide, 0.7 mm apart: 0.1 mm of mask between them
        let mask =
            "%ADD10R,0.600000X0.600000*%\nD10*\nX5000000Y5000000D03*\nX5700000Y5000000D03*\n";
        let r = run(
            vec![
                file("top.gtl", Role::CopperTop, copper),
                file("top.gts", Role::MaskTop, mask),
            ],
            "min_mask_bridge_nm = 150000\nverified = true\n",
        );
        assert_eq!(r.count(text::MASK_BRIDGE), 1, "{:?}", r.findings);
        let ok = run(
            vec![
                file("top.gtl", Role::CopperTop, copper),
                file("top.gts", Role::MaskTop, mask),
            ],
            "min_mask_bridge_nm = 90000\nverified = true\n",
        );
        assert_eq!(ok.count(text::MASK_BRIDGE), 0, "{:?}", ok.findings);
    }

    #[test]
    fn legend_over_an_exposed_pad_is_found() {
        let pad = "%ADD10R,1.000000X1.000000*%\nD10*\nX5000000Y5000000D03*\n";
        let silk = "%ADD11C,0.150000*%\nD11*\nX4000000Y5000000D02*\nX6000000Y5000000D01*\n";
        let r = run(
            vec![
                file("top.gtl", Role::CopperTop, pad),
                file("top.gts", Role::MaskTop, pad),
                file("top.gto", Role::LegendTop, silk),
            ],
            "silkscreen_over_pads = true\nverified = true\n",
        );
        assert_eq!(r.count(text::SILK_ON_PAD), 1, "{:?}", r.findings);
        let off = run(
            vec![
                file("top.gtl", Role::CopperTop, pad),
                file("top.gts", Role::MaskTop, pad),
                file("top.gto", Role::LegendTop, silk),
            ],
            "verified = true\n",
        );
        assert_eq!(off.count(text::SILK_ON_PAD), 0);
    }

    #[test]
    fn floating_copper_is_a_warning_and_a_track_holds_a_pour_in_place() {
        let pour = "G36*\nX5000000Y5000000D02*\nX8000000Y5000000D01*\nX8000000Y8000000D01*\nX5000000Y8000000D01*\nX5000000Y5000000D01*\nG37*\n";
        let r = run(
            vec![file("top.gtl", Role::CopperTop, pour)],
            "verified = true\n",
        );
        assert_eq!(r.count(text::FLOATING), 1, "{:?}", r.findings);
        assert!(r.passed());
        let held =
            format!("{pour}%ADD10C,0.200000*%\nD10*\nX6000000Y6000000D02*\nX7000000Y7000000D01*\n");
        let ok = run(
            vec![file("top.gtl", Role::CopperTop, &held)],
            "verified = true\n",
        );
        assert_eq!(ok.count(text::FLOATING), 0, "{:?}", ok.findings);
    }

    #[test]
    fn the_report_is_text_json_and_csv_in_the_eight_languages() {
        let r = run(
            vec![file(
                "top.gtl",
                Role::CopperTop,
                &track(1_000_000, 1_000_000, 9_000_000, 100),
            )],
            "min_track_width_nm = 150000\nverified = true\n",
        );
        for lang in eh3d_phoenix_rules::Lang::ALL {
            let t = text::report_text(&r, lang);
            assert!(t.contains("PHOENIX-GDF-002"), "{t}");
        }
        let en = text::report_text(&r, eh3d_phoenix_rules::Lang::En);
        assert!(en.starts_with("FAIL"));
        let pt = text::report_text(&r, eh3d_phoenix_rules::Lang::Pt);
        assert!(pt.starts_with("REPROVADO"));
        let json = text::report_json(&r);
        assert_eq!(json["passed"], false);
        assert_eq!(json["findings"][0]["code"], "PHOENIX-GDF-002");
        assert_eq!(
            json["findings"][0]["title"]["pt"],
            "Condutor mais estreito do que o perfil permite"
        );
        let csv = text::report_csv(&r, eh3d_phoenix_rules::Lang::En);
        assert!(csv.starts_with("code,severity,layer,"));
        assert_eq!(csv.lines().count(), 2);
    }

    #[test]
    fn a_plated_hole_with_too_thin_a_ring_is_found_on_the_outer_layers() {
        use crate::model::{DrillFile, Hit, Units};
        use crate::settings::ZeroOmission;
        use std::collections::BTreeMap;
        let pad = "%ADD10C,1.000000*%\nD10*\nX5000000Y5000000D03*\n";
        let drill = DrillFile {
            units: Units::Millimetres,
            tools: BTreeMap::from([(1, 800_000)]),
            hits: vec![Hit {
                tool: 1,
                at: P {
                    x: 5_000_000,
                    y: 5_000_000,
                },
            }],
            slots: Vec::new(),
            attributes: BTreeMap::new(),
            tool_attributes: BTreeMap::new(),
            findings: Vec::new(),
            ended: true,
            omission: ZeroOmission::Leading,
        };
        let drill_file = ImportedFile {
            name: "plated.drl".to_owned(),
            identified: Some(Identified {
                role: Role::DrillPlated,
                source: Source::FileName,
            }),
            content: Content::Drill(drill),
        };
        // pad 1.0, drill 0.8: a ring of 0.1 mm, less the registration error in the worst case
        let r = run(
            vec![file("top.gtl", Role::CopperTop, pad), drill_file.clone()],
            "min_annular_ring_nm = 100000\nverified = true\n",
        );
        assert_eq!(r.count(text::RING), 1, "{:?}", r.findings);
        assert!(r.findings[0].measured_nm < 100_000);
        let ok = run(
            vec![file("top.gtl", Role::CopperTop, pad), drill_file],
            "min_annular_ring_nm = 20000\nverified = true\n",
        );
        assert_eq!(ok.count(text::RING), 0, "{:?}", ok.findings);
    }

    #[test]
    fn tracks_that_cross_are_one_thing_and_a_layer_with_clear_polarity_is_inconclusive() {
        // two tracks that cross, then a pad that touches one: one thing, no spacing finding
        let body = "%ADD10C,0.200000*%
D10*
X1000000Y1000000D02*
X9000000Y1000000D01*
X5000000Y500000D02*
X5000000Y1500000D01*
X1000000Y1100000D03*
";
        let r = run(
            vec![file("top.gtl", Role::CopperTop, body)],
            "min_clearance_nm = 150000
verified = true
",
        );
        assert_eq!(r.count(text::SPACING), 0, "{:?}", r.findings);
        let clear = format!(
            "{body}%LPC*%
X3000000Y1000000D03*
"
        );
        let r = run(
            vec![file("top.gtl", Role::CopperTop, &clear)],
            "min_clearance_nm = 150000
verified = true
",
        );
        assert_eq!(r.count(text::SKIPPED), 1, "{:?}", r.findings);
        assert!(r.passed());
    }

    #[test]
    fn findings_over_the_limit_of_a_check_are_counted_and_not_kept() {
        let mut body = String::from(
            "%ADD10C,0.050000*%
D10*
",
        );
        for k in 0..5 {
            body.push_str(&format!(
                "X1000000Y{}D02*
X2000000Y{}D01*
",
                1_000_000 + k * 1_000_000,
                1_000_000 + k * 1_000_000
            ));
        }
        let files = vec![file("top.gtl", Role::CopperTop, &body)];
        let mut s = GeoSettings::standard();
        s.max_findings = 2;
        let r = check_set(
            &ImportedSet { files },
            &profile(
                "min_track_width_nm = 100000
verified = true
",
            ),
            &s,
        );
        assert_eq!(r.count(text::WIDTH), 2);
        assert_eq!(r.dropped, vec![(text::WIDTH, 3)]);
    }
}
