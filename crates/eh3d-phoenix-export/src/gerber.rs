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
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, PadShape, Segment};
use eh3d_phoenix_model::Point;
use eh3d_writer_gerber::{GerberOptions, GerberOutput, WriteError};

use crate::codes;

/// The coordinate format of the files: 4 digits before and 6 after the point, in millimetres.
pub const FORMAT: CoordinateFormat = CoordinateFormat {
    zero_omission: ZeroOmission::Leading,
    integer_digits: 4,
    decimal_digits: 6,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ApKey {
    Circle(i64),
    Rect(i64, i64),
    Obround(i64, i64),
}

enum Draw {
    Flash(ApKey, Point),
    Line(ApKey, Point, Point),
    Arc(ApKey, Point, Point, (f64, f64), bool),
    Region(Vec<Point>),
}

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
fn arc_centre(start: Point, end: Point, sweep_udeg: i64) -> Option<((f64, f64), bool)> {
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
    draws: &mut Vec<Draw>,
    key: ApKey,
    path: &[Segment],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for s in path {
        match s {
            Segment::Line { start, end } => draws.push(Draw::Line(key, *start, *end)),
            Segment::Arc { start, end, sweep } => match arc_centre(*start, *end, sweep.get()) {
                Some((offset, clockwise)) => {
                    draws.push(Draw::Arc(key, *start, *end, offset, clockwise))
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

fn collect(
    board: &Board,
    layer: Id,
    kind: LayerKind,
    outline_width: i64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Draw> {
    let mut draws = Vec::new();
    let is_copper = kind == LayerKind::Copper;
    if is_copper {
        let mut zones = 0usize;
        for z in board.zones.values().filter(|z| z.layer == layer) {
            if z.outline.len() >= 3 {
                draws.push(Draw::Region(z.outline.clone()));
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
                ApKey::Circle(t.width.get()),
                &t.path,
                diagnostics,
            );
        }
        for v in board
            .vias
            .values()
            .filter(|v| v.from_layer == layer || v.to_layer == layer)
        {
            draws.push(Draw::Flash(ApKey::Circle(v.diameter.get()), v.at));
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
                        ApKey::Circle(d.width.get()),
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
        let key = ApKey::Circle(outline_width);
        let n = board.outline.len();
        for i in 0..n {
            draws.push(Draw::Line(
                key,
                board.outline[i],
                board.outline[(i + 1) % n],
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
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            ) else {
                continue;
            };
            let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
            if total % 90_000_000 != 0 {
                rotated += 1;
            }
            let swapped = (total / 90_000_000) % 2 == 1 && total % 90_000_000 == 0;
            draws.push(Draw::Flash(pad_key(pad.shape, swapped), at));
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
}

/// Writes one layer. Returns the file and the diagnostics.
pub fn write_layer(
    board: &Board,
    spec: &LayerFileSpec,
    outline_width: i64,
) -> Result<(GerberOutput, Vec<Diagnostic>), WriteError> {
    let mut diagnostics = Vec::new();
    let draws = collect(
        board,
        spec.layer,
        spec.kind,
        outline_width,
        &mut diagnostics,
    );
    let mut keys: Vec<ApKey> = draws
        .iter()
        .filter_map(|d| match d {
            Draw::Flash(k, _) | Draw::Line(k, _, _) | Draw::Arc(k, _, _, _, _) => Some(*k),
            Draw::Region(_) => None,
        })
        .collect();
    keys.sort();
    keys.dedup();
    let numbers: BTreeMap<ApKey, u32> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, 10 + i as u32))
        .collect();
    let mut file = GerberFile {
        units: Some(Units::Millimetres),
        format: Some(FORMAT),
        apertures: numbers.iter().map(|(k, n)| (*n, aperture_of(*k))).collect(),
        macros: BTreeMap::new(),
        attributes: Vec::new(),
        commands: Vec::new(),
        ended: true,
        diagnostics: Vec::new(),
    };
    if let Some(values) = &spec.file_function {
        file.attributes.push(Attribute {
            kind: "TF".to_owned(),
            name: ".FileFunction".to_owned(),
            values: values.clone(),
        });
    }
    let mut current: Option<u32> = None;
    let push = |file: &mut GerberFile, op: Operation| {
        file.commands.push(Command { line: 0, op });
    };
    for d in &draws {
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
    let output = eh3d_writer_gerber::write(&file, &GerberOptions::default())?;
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
