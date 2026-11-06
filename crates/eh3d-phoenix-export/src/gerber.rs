//! A layer of the board as a Gerber X2 file.
//!
//! The drawing items of the layer are collected in a fixed order (zones, tracks, vias, pads, each
//! by id), the apertures are numbered from 10 in the order of their sorted shapes, and the file is
//! written by the deterministic writer of EH3D-CORE-LIBS: the same board and options always give
//! the same bytes.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_gerber::{
    Aperture, Attribute, Command, CoordinateFormat, GerberFile, Operation, Point as GPoint,
    Polarity, QuadrantMode, Units, ZeroOmission,
};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, Pad, PadRef, PadShape, Segment, Via, Zone};
use eh3d_phoenix_model::{Point, Udeg};
use eh3d_writer_gerber::{GerberOptions, GerberOutput, WriteError};

use crate::codes;
use crate::profile::{ObjectAttribute, X2Level, X2Profile};
use crate::x2::{self, ApFunction, FieldError, Fields, FileInput, ObjectSet, NO_NET};

/// The coordinate format of the files unless a profile says otherwise: 4 digits before and 6 after
/// the point, in millimetres.
pub const FORMAT: CoordinateFormat = CoordinateFormat {
    zero_omission: ZeroOmission::Leading,
    integer_digits: 4,
    decimal_digits: 6,
};

/// The smallest size, in nanometres, an aperture takes when a mask expansion or a paste shrink would
/// make it vanish, so that no pad is lost from a file.
pub const MIN_APERTURE_NM: i64 = 1_000;

/// The value of the `.GenerationSoftware` attribute: vendor, product and the version of this crate.
pub const GENERATION_SOFTWARE: [&str; 3] = ["EH3D", "PHOENIX-PCB", env!("CARGO_PKG_VERSION")];

/// The format of the files with `decimals` digits after the point (1 to 6).
pub fn format_with_decimals(decimals: u32) -> CoordinateFormat {
    CoordinateFormat {
        zero_omission: ZeroOmission::Leading,
        integer_digits: 4,
        decimal_digits: decimals,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ApKey {
    Circle(i64),
    Rect(i64, i64),
    Obround(i64, i64),
}

/// An aperture: its shape and, at the levels of X2 that name it, its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Ap {
    key: ApKey,
    function: Option<ApFunction>,
}

enum Draw {
    Flash(Ap, Point),
    Line(Ap, Point, Point),
    Arc(Ap, Point, Point, (f64, f64), bool),
    Region(Vec<Point>),
    /// What follows is drawn with this polarity until the next change.
    Polarity(Polarity),
}

/// How many straight pieces make the quarter of a circle at the corner of a rounded pad.
const CORNER_STEPS: i64 = 6;

/// True when the export writes the pad as a region and not as a flash: a pad with rounded
/// corners whose angle on the board is a right angle (at any other angle the pad is flashed
/// unrotated, as every pad is, with the warning `PHOENIX-EXPORT-001`).
pub fn pad_is_region(pad: &Pad, component_rotation: Udeg) -> bool {
    pad.corner_radius_nm().is_some()
        && (pad.rotation.get() + component_rotation.get()).rem_euclid(360_000_000) % 90_000_000 == 0
}

/// The corners of a rectangle of `width` by `height` about `at` with corners of `radius`, counter
/// clockwise, the corner arcs cut into `CORNER_STEPS` pieces each. The radius is at most half of
/// the shorter side.
fn rounded_rectangle(at: Point, width: i64, height: i64, radius: i64) -> Vec<Point> {
    let radius = radius.clamp(0, width.min(height) / 2);
    let (cx, cy) = (at.x_nm.get(), at.y_nm.get());
    let (hx, hy) = (width / 2 - radius, height / 2 - radius);
    let mut out: Vec<Point> = Vec::new();
    // the centres of the four corner arcs and the angle where each starts, counter clockwise from
    // the lower right corner
    let corners = [
        (hx, -hy, -90.0_f64),
        (hx, hy, 0.0),
        (-hx, hy, 90.0),
        (-hx, -hy, 180.0),
    ];
    for (dx, dy, start) in corners {
        for step in 0..=CORNER_STEPS {
            let angle = (start + 90.0 * step as f64 / CORNER_STEPS as f64).to_radians();
            let x = cx + dx + (radius as f64 * angle.cos()).round() as i64;
            let y = cy + dy + (radius as f64 * angle.sin()).round() as i64;
            if let Ok(p) = Point::from_nm(x, y) {
                if out.last() != Some(&p) {
                    out.push(p);
                }
            }
        }
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// A drawn object with the object attributes it carries.
type Drawn = (Draw, ObjectSet);

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

fn gp(p: Point) -> GPoint {
    GPoint {
        x: mm(p.x_nm.get()),
        y: mm(p.y_nm.get()),
    }
}

/// The offsets `(I, J)` from the start of an arc to its centre, in millimetres, and whether it runs
/// clockwise. `None` when the arc has no computable centre.
pub(crate) fn arc_centre(start: Point, end: Point, sweep_udeg: i64) -> Option<((f64, f64), bool)> {
    let (sx, sy) = (mm(start.x_nm.get()), mm(start.y_nm.get()));
    let (ex, ey) = (mm(end.x_nm.get()), mm(end.y_nm.get()));
    let (cx, cy) = (ex - sx, ey - sy);
    let d = (cx * cx + cy * cy).sqrt();
    let theta = (sweep_udeg.abs() as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let half = theta / 2.0;
    let s = half.sin();
    if d == 0.0 || s.abs() < 1e-9 || !d.is_finite() {
        return None;
    }
    let r = d / (2.0 * s);
    let h = r * half.cos();
    let (nx, ny) = (-cy / d, cx / d);
    let sign = if sweep_udeg >= 0 { 1.0 } else { -1.0 };
    let (mx, my) = (sx + cx / 2.0, sy + cy / 2.0);
    let (ox, oy) = (mx + sign * nx * h, my + sign * ny * h);
    Some(((ox - sx, oy - sy), sweep_udeg < 0))
}

fn pad_key(shape: PadShape, swapped: bool) -> ApKey {
    match shape {
        PadShape::Circle { diameter } => ApKey::Circle(diameter.get()),
        PadShape::Rect { width, height } => {
            let (w, h) = (width.get(), height.get());
            if swapped {
                ApKey::Rect(h, w)
            } else {
                ApKey::Rect(w, h)
            }
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (width.get(), height.get());
            if swapped {
                ApKey::Obround(h, w)
            } else {
                ApKey::Obround(w, h)
            }
        }
    }
}

fn push_pieces(
    draws: &mut Vec<Drawn>,
    ap: Ap,
    set: &ObjectSet,
    path: &[Segment],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for s in path {
        match s {
            Segment::Line { start, end } => draws.push((Draw::Line(ap, *start, *end), set.clone())),
            Segment::Arc { start, end, sweep } => match arc_centre(*start, *end, sweep.get()) {
                Some((offset, clockwise)) => {
                    draws.push((Draw::Arc(ap, *start, *end, offset, clockwise), set.clone()))
                }
                None => diagnostics.push(codes::diagnostic(
                    codes::ARC_SKIPPED,
                    Severity::Warning,
                    "an arc of a track or of the silkscreen",
                )),
            },
        }
    }
}

/// Grows (or, when negative, shrinks) every dimension of an aperture by `delta` nanometres.
fn adjust(key: ApKey, delta: i64) -> ApKey {
    let f = |v: i64| v.saturating_add(delta).max(MIN_APERTURE_NM);
    match key {
        ApKey::Circle(d) => ApKey::Circle(f(d)),
        ApKey::Rect(w, h) => ApKey::Rect(f(w), f(h)),
        ApKey::Obround(w, h) => ApKey::Obround(f(w), f(h)),
    }
}

/// The function of the aperture of a pad.
fn pad_function(pad: &Pad, footprint_name: &str, x2: &X2Profile) -> ApFunction {
    let name = footprint_name.to_lowercase();
    if x2
        .fiducial_names
        .iter()
        .any(|word| name.contains(&word.to_lowercase()))
    {
        ApFunction::fiducial(x2.fiducial_scope)
    } else if x2
        .heatsink_pad_numbers
        .iter()
        .any(|number| number.eq_ignore_ascii_case(&pad.number))
    {
        ApFunction::HeatsinkPad
    } else if pad.drill.is_some() {
        ApFunction::ComponentPad
    } else {
        ApFunction::SmdPad
    }
}

/// The objects that clear the copper of a zone: its own cutouts and a keep-away of its clearance
/// around the tracks, vias and pads of other nets that lie on the layer. All are drawn with clear
/// polarity, between a change to clear and a change back to dark, so that what is drawn after
/// them (the copper of the board) stays. Nothing when there is nothing to clear. The aperture of
/// a keep-away has the function of the object it keeps away from.
fn zone_clearing(board: &Board, zone: &Zone, layer: Id, x2: &X2Profile) -> Vec<Drawn> {
    let named = x2.level.has_aperture_attributes();
    let ap = |key: ApKey, function: ApFunction| Ap {
        key,
        function: named.then_some(function),
    };
    let none = ObjectSet::default;
    let mut clear: Vec<Drawn> = Vec::new();
    for cutout in zone.cutouts.iter().filter(|c| c.len() >= 3) {
        clear.push((Draw::Region(cutout.clone()), none()));
    }
    let c = zone.clearance.get();
    if c > 0 {
        let grow = c.saturating_mul(2);
        let xs = zone.outline.iter().map(|p| p.x_nm.get());
        let ys = zone.outline.iter().map(|p| p.y_nm.get());
        let (x0, x1) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
        let (y0, y1) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
        // an object is left out when its box, with the keep-away, is clear of the box of the zone
        let near = |p: Point, reach: i64| {
            let reach = reach.saturating_add(c);
            p.x_nm.get().saturating_add(reach) >= x0
                && p.x_nm.get().saturating_sub(reach) <= x1
                && p.y_nm.get().saturating_add(reach) >= y0
                && p.y_nm.get().saturating_sub(reach) <= y1
        };
        for track in board
            .tracks
            .values()
            .filter(|t| t.layer == layer && t.net != zone.net)
        {
            let key = ap(
                ApKey::Circle(track.width.get().saturating_add(grow)),
                ApFunction::Conductor,
            );
            let has_arc = track.path.iter().any(|s| matches!(s, Segment::Arc { .. }));
            let reach = track.width.get() / 2;
            for piece in &track.path {
                match piece {
                    Segment::Line { start, end } => {
                        if has_arc || near(*start, reach) || near(*end, reach) {
                            clear.push((Draw::Line(key, *start, *end), none()));
                        }
                    }
                    Segment::Arc { start, end, sweep } => {
                        if let Some((offset, clockwise)) = arc_centre(*start, *end, sweep.get()) {
                            clear.push((Draw::Arc(key, *start, *end, offset, clockwise), none()));
                        }
                    }
                }
            }
        }
        for via in board
            .vias
            .values()
            .filter(|v| v.net != zone.net && via_on_layer(board, v, layer))
        {
            if near(via.at, via.diameter.get() / 2) {
                let key = ap(
                    ApKey::Circle(via.diameter.get().saturating_add(grow)),
                    ApFunction::ViaPad,
                );
                clear.push((Draw::Flash(key, via.at), none()));
            }
        }
        for (cid, component) in &board.components {
            let Some(footprint) = board.footprints.get(&component.footprint) else {
                continue;
            };
            for (pid, pad) in &footprint.pads {
                if !pad.layers.contains(&layer) || pad.is_npth() {
                    continue;
                }
                let pad_ref = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                if board.net_of(pad_ref) == Some(zone.net) {
                    continue;
                }
                let Some(at) = pad_position(board, pad_ref) else {
                    continue;
                };
                let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
                let swapped = (total / 90_000_000) % 2 == 1 && total % 90_000_000 == 0;
                let key = adjust(pad_key(pad.shape, swapped), grow);
                let reach = match key {
                    ApKey::Circle(d) => d / 2,
                    ApKey::Rect(w, h) | ApKey::Obround(w, h) => w.max(h),
                };
                if near(at, reach) {
                    let function = pad_function(pad, &footprint.name, x2);
                    clear.push((Draw::Flash(ap(key, function), at), none()));
                }
            }
        }
    }
    if clear.is_empty() {
        return clear;
    }
    let mut out: Vec<Drawn> = Vec::with_capacity(clear.len() + 2);
    out.push((Draw::Polarity(Polarity::Clear), none()));
    out.extend(clear);
    out.push((Draw::Polarity(Polarity::Dark), none()));
    out
}

fn collect(
    board: &Board,
    spec: &LayerFileSpec,
    outline_width: i64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Drawn> {
    let (layer, kind) = (spec.layer, spec.kind);
    let x2 = &spec.x2;
    let named = x2.level.has_aperture_attributes();
    let ap = |key: ApKey, function: ApFunction| Ap {
        key,
        function: named.then_some(function),
    };
    let plain = |key: ApKey| Ap {
        key,
        function: None,
    };
    let delta = match kind {
        LayerKind::SolderMask => spec.mask_expansion.saturating_mul(2),
        LayerKind::Paste => spec.paste_shrink.saturating_mul(-2),
        _ => 0,
    };
    let is_copper = kind == LayerKind::Copper;
    let net_name = |id: Id| -> String {
        board
            .nets
            .get(&id)
            .map(|n| n.name.as_str())
            .filter(|n| !n.is_empty())
            .unwrap_or(NO_NET)
            .to_owned()
    };
    let net_set = |id: Id| ObjectSet {
        net: (is_copper && x2.writes(ObjectAttribute::Net)).then(|| net_name(id)),
        ..ObjectSet::default()
    };
    let mut draws: Vec<Drawn> = Vec::new();
    if is_copper {
        let mut zones = 0usize;
        for z in board.zones.values().filter(|z| z.layer == layer) {
            if z.outline.len() >= 3 {
                draws.push((Draw::Region(z.outline.clone()), net_set(z.net)));
                zones += 1;
                // the openings of the pour and the keep-away of other copper are cleared out of
                // it, before the copper of the board is drawn on top
                draws.extend(zone_clearing(board, z, layer, x2));
            }
        }
        if zones > 0 {
            diagnostics.push(codes::diagnostic(
                codes::ZONE_NOT_POURED,
                Severity::Info,
                &format!("{zones} zone(s) on one layer"),
            ));
        }
        for t in board.tracks.values().filter(|t| t.layer == layer) {
            push_pieces(
                &mut draws,
                ap(ApKey::Circle(t.width.get()), ApFunction::Conductor),
                &net_set(t.net),
                &t.path,
                diagnostics,
            );
        }
        for v in board
            .vias
            .values()
            .filter(|v| via_on_layer(board, v, layer))
        {
            draws.push((
                Draw::Flash(
                    ap(ApKey::Circle(v.diameter.get()), ApFunction::ViaPad),
                    v.at,
                ),
                net_set(v.net),
            ));
        }
    }
    if kind == LayerKind::Silkscreen {
        let (mut no_glyph, mut undrawn) = (0usize, 0usize);
        for item in board.silkscreen.values().filter(|i| i.layer() == layer) {
            match item.drawn() {
                Ok(d) => {
                    no_glyph += d.missing_glyphs;
                    push_pieces(
                        &mut draws,
                        plain(ApKey::Circle(d.width.get())),
                        &ObjectSet::default(),
                        &d.pieces,
                        diagnostics,
                    );
                }
                Err(_) => undrawn += 1,
            }
        }
        if no_glyph > 0 {
            diagnostics.push(codes::diagnostic(
                codes::SILK_NO_GLYPH,
                Severity::Warning,
                &format!("{no_glyph} character(s) on one layer"),
            ));
        }
        if undrawn > 0 {
            diagnostics.push(codes::diagnostic(
                codes::SILK_UNDRAWN,
                Severity::Warning,
                &format!("{undrawn} item(s) on one layer"),
            ));
        }
    }
    if kind == LayerKind::Outline {
        // the profile is the outline and then every opening inside it, each a closed contour
        let line = ap(ApKey::Circle(outline_width), ApFunction::Profile);
        for ring in std::iter::once(&board.outline).chain(board.outline_cutouts.iter()) {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            for i in 0..n {
                draws.push((
                    Draw::Line(line, ring[i], ring[(i + 1) % n]),
                    ObjectSet::default(),
                ));
            }
        }
    }
    let mut rotated = 0usize;
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            if !pad.layers.contains(&layer) {
                continue;
            }
            if is_copper && pad.is_npth() {
                // a hole that is not plated has no copper
                continue;
            }
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
            if total % 90_000_000 != 0 {
                rotated += 1;
            }
            let swapped = (total / 90_000_000) % 2 == 1 && total % 90_000_000 == 0;
            let refdes = (!component.refdes.is_empty()).then(|| component.refdes.clone());
            let set = ObjectSet {
                net: (is_copper && x2.writes(ObjectAttribute::Net)).then(|| {
                    board
                        .net_of(pad_ref)
                        .map_or_else(|| NO_NET.to_owned(), net_name)
                }),
                pin: refdes
                    .clone()
                    .filter(|_| x2.writes(ObjectAttribute::Pin) && !pad.number.is_empty())
                    .map(|r| (r, pad.number.clone())),
                component: refdes.filter(|_| x2.writes(ObjectAttribute::Component)),
            };
            if let (true, PadShape::Rect { width, height }, Some(radius)) = (
                pad_is_region(pad, component.rotation),
                pad.shape,
                pad.corner_radius_nm(),
            ) {
                let (w, h) = if swapped {
                    (height.get(), width.get())
                } else {
                    (width.get(), height.get())
                };
                let grow = |v: i64| v.saturating_add(delta).max(MIN_APERTURE_NM);
                let ring = rounded_rectangle(at, grow(w), grow(h), (radius + delta / 2).max(0));
                if ring.len() >= 3 {
                    draws.push((Draw::Region(ring), set));
                    continue;
                }
            }
            let function = pad_function(pad, &footprint.name, x2);
            draws.push((
                Draw::Flash(ap(adjust(pad_key(pad.shape, swapped), delta), function), at),
                set,
            ));
        }
    }
    if rotated > 0 {
        diagnostics.push(codes::diagnostic(
            codes::PAD_ROTATION,
            Severity::Warning,
            &format!("{rotated} pad(s) on one layer"),
        ));
    }
    draws
}

fn aperture_of(key: ApKey) -> Aperture {
    match key {
        ApKey::Circle(d) => Aperture::Circle {
            diameter: mm(d),
            hole: None,
        },
        ApKey::Rect(w, h) => Aperture::Rectangle {
            x: mm(w),
            y: mm(h),
            hole: None,
        },
        ApKey::Obround(w, h) => Aperture::Obround {
            x: mm(w),
            y: mm(h),
            hole: None,
        },
    }
}

/// What a layer needs to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerFileSpec {
    /// The layer.
    pub layer: Id,
    /// Its kind.
    pub kind: LayerKind,
    /// The `.FileFunction` value, when it is known.
    pub file_function: Option<Vec<String>>,
    /// The `.FilePolarity` value (`Positive` or `Negative`), when the kind of layer has one.
    pub polarity: Option<&'static str>,
    /// The coordinate format of the file.
    pub format: CoordinateFormat,
    /// How far, per side, a solder mask opening is larger than its pad, in nanometres.
    pub mask_expansion: i64,
    /// How far, per side, a paste opening is smaller than its pad, in nanometres.
    pub paste_shrink: i64,
    /// The Gerber X2 attributes to write.
    pub x2: X2Profile,
}

/// Why a layer file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// The writer of the format refused the file.
    Write(WriteError),
    /// The writer of the drill format refused the file.
    Drill(eh3d_writer_excellon::WriteError),
    /// A text cannot go into an attribute field.
    Field(FieldError),
    /// The attribute commands could not be put into the file (a defect of this export).
    Attributes,
}

impl std::fmt::Display for LayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerError::Write(e) => write!(f, "{e}"),
            LayerError::Drill(e) => write!(f, "{e}"),
            LayerError::Field(e) => write!(f, "{e}"),
            LayerError::Attributes => f.write_str("the attribute commands do not fit the file"),
        }
    }
}

impl std::error::Error for LayerError {}

impl From<WriteError> for LayerError {
    fn from(e: WriteError) -> Self {
        LayerError::Write(e)
    }
}

impl From<eh3d_writer_excellon::WriteError> for LayerError {
    fn from(e: eh3d_writer_excellon::WriteError) -> Self {
        LayerError::Drill(e)
    }
}

impl From<FieldError> for LayerError {
    fn from(e: FieldError) -> Self {
        LayerError::Field(e)
    }
}

/// Writes one layer. Returns the file and the diagnostics.
pub fn write_layer(
    board: &Board,
    spec: &LayerFileSpec,
    outline_width: i64,
) -> Result<(GerberOutput, Vec<Diagnostic>), LayerError> {
    let mut diagnostics = Vec::new();
    let draws = collect(board, spec, outline_width, &mut diagnostics);
    let mut keys: Vec<Ap> = draws
        .iter()
        .filter_map(|(d, _)| match d {
            Draw::Flash(k, _) | Draw::Line(k, _, _) | Draw::Arc(k, _, _, _, _) => Some(*k),
            Draw::Region(_) | Draw::Polarity(_) => None,
        })
        .collect();
    keys.sort();
    keys.dedup();
    let numbers: BTreeMap<Ap, u32> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, 10 + i as u32))
        .collect();
    let mut file = GerberFile {
        units: Some(Units::Millimetres),
        format: Some(spec.format),
        apertures: numbers
            .iter()
            .map(|(k, n)| (*n, aperture_of(k.key)))
            .collect(),
        macros: BTreeMap::new(),
        attributes: Vec::new(),
        commands: Vec::new(),
        ended: true,
        diagnostics: Vec::new(),
    };
    let mut fields = Fields::new(spec.x2.strict);
    let input = FileInput {
        x2: &spec.x2,
        board_name: &board.name,
        function: spec.file_function.clone(),
        polarity: spec.polarity,
    };
    for (name, values) in x2::file_attributes(&input, &mut fields)? {
        file.attributes.push(Attribute {
            kind: "TF".to_owned(),
            name: name.to_owned(),
            values,
        });
    }
    let objects = spec.x2.level == X2Level::Full;
    let mut changes: Vec<String> = Vec::new();
    let mut in_force = ObjectSet::default();
    let mut current: Option<u32> = None;
    let push = |file: &mut GerberFile, op: Operation| {
        file.commands.push(Command { line: 0, op });
    };
    for (d, set) in &draws {
        if objects {
            let text = x2::object_changes(&in_force, set, &mut fields)?;
            if !text.is_empty() {
                push(&mut file, x2::marker());
                changes.push(text);
                in_force = set.clone();
            }
        }
        if let Draw::Flash(k, _) | Draw::Line(k, _, _) | Draw::Arc(k, _, _, _, _) = d {
            let n = numbers[k];
            if current != Some(n) {
                push(&mut file, Operation::SelectAperture(n));
                current = Some(n);
            }
        }
        match d {
            Draw::Flash(_, at) => push(&mut file, Operation::Flash { at: gp(*at) }),
            Draw::Line(_, from, to) => push(
                &mut file,
                Operation::Line {
                    from: gp(*from),
                    to: gp(*to),
                },
            ),
            Draw::Arc(_, from, to, offset, clockwise) => push(
                &mut file,
                Operation::Arc {
                    from: gp(*from),
                    to: gp(*to),
                    center_offset: *offset,
                    clockwise: *clockwise,
                    quadrant: QuadrantMode::Multi,
                },
            ),
            Draw::Region(points) => {
                push(&mut file, Operation::RegionStart);
                push(&mut file, Operation::Move { to: gp(points[0]) });
                for w in points.windows(2) {
                    push(
                        &mut file,
                        Operation::Line {
                            from: gp(w[0]),
                            to: gp(w[1]),
                        },
                    );
                }
                push(
                    &mut file,
                    Operation::Line {
                        from: gp(points[points.len() - 1]),
                        to: gp(points[0]),
                    },
                );
                push(&mut file, Operation::RegionEnd);
            }
            Draw::Polarity(p) => push(&mut file, Operation::Polarity(*p)),
        }
    }
    if objects {
        let text = x2::object_changes(&in_force, &ObjectSet::default(), &mut fields)?;
        if !text.is_empty() {
            push(&mut file, x2::marker());
            changes.push(text);
        }
    }
    let mut output = eh3d_writer_gerber::write(&file, &GerberOptions::default())?;
    if objects {
        output.text = x2::replace_markers(&output.text, &changes).ok_or(LayerError::Attributes)?;
    }
    if spec.x2.level.has_aperture_attributes() {
        let functions: BTreeMap<u32, ApFunction> = numbers
            .iter()
            .filter_map(|(k, n)| k.function.map(|f| (*n, f)))
            .collect();
        output.text = x2::insert_aperture_attributes(&output.text, &functions);
    }
    if fields.escaped() > 0 {
        diagnostics.push(codes::diagnostic(
            codes::ATTRIBUTE_ESCAPED,
            Severity::Info,
            &format!("{} text(s) in one file", fields.escaped()),
        ));
    }
    Ok((output, diagnostics))
}

/// True when the via has a pad on the copper layer: the layer is one of its two ends, or lies
/// between them in the stack-up of the board (a through via has a pad on every inner layer and a
/// blind or buried via on the layers it spans). A board without a stack-up, or a via whose ends
/// are not in it, gets a pad on its two ends only.
pub fn via_on_layer(board: &Board, via: &Via, layer: Id) -> bool {
    if via.from_layer == layer || via.to_layer == layer {
        return true;
    }
    let position = |id: Id| board.copper_stack.iter().position(|l| *l == id);
    match (
        position(via.from_layer),
        position(via.to_layer),
        position(layer),
    ) {
        (Some(a), Some(b), Some(l)) => l > a.min(b) && l < a.max(b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::Point;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn a_quarter_circle_has_its_centre_where_it_should() {
        // from (1,0) to (0,1) counter-clockwise by 90 degrees about the origin (in mm)
        let ((i, j), cw) = arc_centre(pt(1_000_000, 0), pt(0, 1_000_000), 90_000_000).unwrap();
        assert!(!cw);
        assert!((i + 1.0).abs() < 1e-9 && j.abs() < 1e-9, "{i} {j}");
        // the same points clockwise: the centre is on the other side of the chord
        let ((i, j), cw) = arc_centre(pt(1_000_000, 0), pt(0, 1_000_000), -90_000_000).unwrap();
        assert!(cw);
        assert!((i - 0.0).abs() < 1e-9 && (j - 1.0).abs() < 1e-9, "{i} {j}");
        // a half circle has its centre on the chord
        let ((i, j), _) = arc_centre(pt(0, 0), pt(2_000_000, 0), 180_000_000).unwrap();
        assert!((i - 1.0).abs() < 1e-9 && j.abs() < 1e-9);
        // no sweep and no length give no centre
        assert!(arc_centre(pt(0, 0), pt(1, 0), 0).is_none());
        assert!(arc_centre(pt(5, 5), pt(5, 5), 90_000_000).is_none());
    }

    #[test]
    fn a_via_has_a_pad_on_every_layer_it_spans_and_on_no_other() {
        use eh3d_phoenix_model::{Nm, ObjectId};
        let layer = |n: u16| ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap();
        // four copper layers, top to bottom: 1 (top), 2, 3, 4 (bottom)
        let mut board = Board {
            copper_stack: (1..=4).map(layer).collect(),
            ..Board::default()
        };
        let via = |from: u16, to: u16| Via {
            net: layer(10),
            at: pt(0, 0),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: layer(from),
            to_layer: layer(to),
        };
        let spans = |board: &Board, v: &Via| -> Vec<bool> {
            (1..=4).map(|n| via_on_layer(board, v, layer(n))).collect()
        };
        // a through via: a pad on all four layers
        assert_eq!(spans(&board, &via(1, 4)), [true, true, true, true]);
        // the ends in the other order give the same answer
        assert_eq!(spans(&board, &via(4, 1)), [true, true, true, true]);
        // a blind via from the top to the second inner layer: 1, 2 and 3 but not 4
        assert_eq!(spans(&board, &via(1, 3)), [true, true, true, false]);
        // a buried via between the two inner layers: 2 and 3 only
        assert_eq!(spans(&board, &via(2, 3)), [false, true, true, false]);
        // a via on a layer that the stack-up does not list keeps the old rule: its two ends
        assert!(via_on_layer(&board, &via(1, 9), layer(9)));
        assert!(!via_on_layer(&board, &via(1, 9), layer(2)));
        // a board without a stack-up: the two ends only
        board.copper_stack.clear();
        assert_eq!(spans(&board, &via(1, 4)), [true, false, false, true]);
    }

    #[test]
    fn apertures_are_keyed_in_a_fixed_order() {
        let mut keys = vec![
            ApKey::Obround(1, 2),
            ApKey::Circle(9),
            ApKey::Rect(3, 3),
            ApKey::Circle(2),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                ApKey::Circle(2),
                ApKey::Circle(9),
                ApKey::Rect(3, 3),
                ApKey::Obround(1, 2)
            ]
        );
    }

    #[test]
    fn a_rounded_rectangle_has_its_corners_cut_into_pieces() {
        // 2 by 1 mm with the largest radius, 0.5 mm: both ends are half circles. Each quarter has
        // 7 points; the point where two quarters meet at the end of a half circle is the same and
        // is kept once, so 7 + 6 + 7 + 6 = 26
        let ring = rounded_rectangle(pt(0, 0), 2_000_000, 1_000_000, 500_000);
        assert_eq!(ring.len(), 26);
        assert!(ring.contains(&pt(1_000_000, 0)));
        assert!(ring.contains(&pt(-1_000_000, 0)));
        assert!(ring.contains(&pt(500_000, -500_000)));
        assert!(ring.contains(&pt(-500_000, 500_000)));
        let xs: Vec<i64> = ring.iter().map(|p| p.x_nm.get()).collect();
        let ys: Vec<i64> = ring.iter().map(|p| p.y_nm.get()).collect();
        assert_eq!(
            (xs.iter().min(), xs.iter().max()),
            (Some(&-1_000_000), Some(&1_000_000))
        );
        assert_eq!(
            (ys.iter().min(), ys.iter().max()),
            (Some(&-500_000), Some(&500_000))
        );
        // a radius larger than half the shorter side is cut to it
        assert_eq!(
            ring,
            rounded_rectangle(pt(0, 0), 2_000_000, 1_000_000, 9_000_000)
        );
        // the box moves with the centre
        let moved = rounded_rectangle(pt(10_000_000, 5_000_000), 2_000_000, 1_000_000, 200_000);
        assert_eq!(moved.iter().map(|p| p.x_nm.get()).min(), Some(9_000_000));
        assert_eq!(moved.iter().map(|p| p.y_nm.get()).max(), Some(5_500_000));
    }

    #[test]
    fn only_a_rounded_rectangle_at_a_right_angle_is_a_region() {
        let pad = Pad {
            number: "1".into(),
            shape: PadShape::Rect {
                width: eh3d_phoenix_model::Nm::new(2_000_000).unwrap(),
                height: eh3d_phoenix_model::Nm::new(1_000_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: Vec::new(),
            roundness: 500,
            plated: true,
        };
        assert!(pad_is_region(&pad, Udeg::ZERO));
        assert!(pad_is_region(&pad, Udeg::new(90_000_000).unwrap()));
        assert!(!pad_is_region(&pad, Udeg::new(45_000_000).unwrap()));
        let square = Pad {
            roundness: 0,
            ..pad.clone()
        };
        assert!(!pad_is_region(&square, Udeg::ZERO));
    }
}
