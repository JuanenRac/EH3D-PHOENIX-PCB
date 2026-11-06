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
    QuadrantMode, Units, ZeroOmission,
};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, Pad, PadRef, PadShape, Segment};
use eh3d_phoenix_model::Point;
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
            .filter(|v| v.from_layer == layer || v.to_layer == layer)
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
    if kind == LayerKind::Outline && board.outline.len() >= 3 {
        let line = ap(ApKey::Circle(outline_width), ApFunction::Profile);
        let n = board.outline.len();
        for i in 0..n {
            draws.push((
                Draw::Line(line, board.outline[i], board.outline[(i + 1) % n]),
                ObjectSet::default(),
            ));
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
            Draw::Region(_) => None,
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
}
