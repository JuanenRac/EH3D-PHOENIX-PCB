// SPDX-License-Identifier: GPL-3.0-or-later
//! The first step of moving the Gerber reader to the shared one: an adapter that reads a file
//! with the reader of EH3D-CORE-LIBS and gives the model of this crate ([`GerberLayer`]), so that
//! the result of the two readers can be compared on the same files before the reader of
//! [`crate::parse`] is replaced.
//!
//! What the adapter converts: the units, the apertures that are not macros, the attributes of the
//! file (`TF`), the flashes (the shift of the placement), the strokes with a round pen, the arcs and
//! the regions (the arcs of a contour as straight pieces whose side is never farther than
//! `arc_chord_nm` from the arc). What it cannot give yet makes `complete` false and is named in
//! [`Adapted::not_converted`]: a macro aperture (its primitives), a flash that is turned or
//! scaled, a stroke with a pen that is not round or is stretched, the apertures of blocks, the
//! attributes of the apertures and of the objects (`TA` and `TO`) and a negative image. The remarks of the shared reader come in
//! [`Adapted::notes`] with its stable codes.

use std::collections::BTreeMap;

use eh3d_parser_gerber::image::{self, Affine, Contour, Segment, Shape};
use eh3d_parser_gerber::{Aperture as CoreAperture, Polarity, Severity as CoreSeverity};

use crate::model::{Aperture, ApertureShape, GerberLayer, Item, Units, P};
use crate::settings::ImportSettings;

/// A remark of the shared reader: its line, whether it is an error, and its stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreNote {
    /// The line in the file.
    pub line: usize,
    /// True for a remark of severity error.
    pub error: bool,
    /// The stable code of the shared reader.
    pub code: String,
}

/// What the adapter gives.
#[derive(Debug, Clone, PartialEq)]
pub struct Adapted {
    /// The layer in the model of this crate.
    pub layer: GerberLayer,
    /// The remarks of the shared reader.
    pub notes: Vec<CoreNote>,
    /// What the adapter could not give, by name (empty when `layer.complete` holds).
    pub not_converted: Vec<&'static str>,
}

fn round_nm(value: f64, scale: f64) -> i64 {
    let v = value * scale;
    if v.is_finite() {
        v.round().clamp(-(1.0e15), 1.0e15) as i64
    } else {
        0
    }
}

fn aperture_of(a: &CoreAperture, scale: f64) -> Option<Aperture> {
    let nm = |v: f64| round_nm(v, scale);
    let (shape, hole) = match a {
        CoreAperture::Circle { diameter, hole } => (
            ApertureShape::Circle {
                diameter: nm(*diameter),
            },
            *hole,
        ),
        CoreAperture::Rectangle { x, y, hole } => (
            ApertureShape::Rect {
                width: nm(*x),
                height: nm(*y),
            },
            *hole,
        ),
        CoreAperture::Obround { x, y, hole } => (
            ApertureShape::Obround {
                width: nm(*x),
                height: nm(*y),
            },
            *hole,
        ),
        CoreAperture::Polygon {
            diameter,
            vertices,
            rotation,
            hole,
        } => (
            ApertureShape::Polygon {
                diameter: nm(*diameter),
                vertices: *vertices,
                rotation: (rotation * 1.0e6).round() as i64,
            },
            *hole,
        ),
        _ => return None,
    };
    Some(Aperture {
        shape,
        hole: hole.map(nm),
    })
}

/// True when the linear part of a transform is the identity (nothing turned, scaled or mirrored).
fn is_plain(t: &Affine) -> bool {
    (t.a - 1.0).abs() < 1e-9 && t.b.abs() < 1e-9 && t.c.abs() < 1e-9 && (t.d - 1.0).abs() < 1e-9
}

/// The corners of a contour, its arcs as straight pieces; the last point joins the first.
fn flatten_contour(contour: &Contour, scale: f64, chord_nm: i64) -> Vec<P> {
    let p = |v: (f64, f64)| P {
        x: round_nm(v.0, scale),
        y: round_nm(v.1, scale),
    };
    let mut out = vec![p(contour.start)];
    let mut at = contour.start;
    for segment in &contour.segments {
        match *segment {
            Segment::Line { to } => out.push(p(to)),
            Segment::Arc {
                to,
                center,
                clockwise,
            } => {
                let radius = (at.0 - center.0).hypot(at.1 - center.1);
                if radius <= 0.0 || !radius.is_finite() {
                    out.push(p(to));
                } else {
                    let a0 = (at.1 - center.1).atan2(at.0 - center.0);
                    let a1 = (to.1 - center.1).atan2(to.0 - center.0);
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
                    if (at.0 - to.0).abs() < 1e-12 && (at.1 - to.1).abs() < 1e-12 {
                        sweep = if clockwise { -tau } else { tau };
                    }
                    let tolerance = (chord_nm.max(1) as f64 / scale).min(radius);
                    let step = 2.0 * (1.0 - tolerance / radius).clamp(-1.0, 1.0).acos();
                    let sides = if step > 0.0 {
                        (sweep.abs() / step).ceil().clamp(1.0, 4096.0) as usize
                    } else {
                        4096
                    };
                    for i in 1..sides {
                        let a = a0 + sweep * i as f64 / sides as f64;
                        out.push(p((
                            center.0 + radius * a.cos(),
                            center.1 + radius * a.sin(),
                        )));
                    }
                    out.push(p(to));
                }
            }
        }
        at = segment.end();
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Read `text` with the shared reader and give it in the model of this crate. The units missing
/// from the file are taken as millimetres, as the shared reader does.
pub fn adapt_gerber(text: &str, settings: &ImportSettings) -> Adapted {
    let file = eh3d_parser_gerber::parse(text);
    let resolved = image::resolve(&file);
    let units = match file.effective_units() {
        eh3d_parser_gerber::Units::Millimetres => Units::Millimetres,
        eh3d_parser_gerber::Units::Inches => Units::Inches,
    };
    let scale = units.nm() as f64;
    let mut not_converted: Vec<&'static str> = Vec::new();
    let mut note = |name: &'static str| {
        if !not_converted.contains(&name) {
            not_converted.push(name);
        }
    };

    let mut apertures = BTreeMap::new();
    for (number, a) in &file.apertures {
        match aperture_of(a, scale) {
            Some(converted) => {
                apertures.insert(*number, converted);
            }
            None => note("macro-aperture"),
        }
    }
    let mut attributes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in &file.attributes {
        if a.kind == "TF" {
            attributes.insert(a.name.clone(), a.values.clone());
        } else if a.kind == "TO" {
            note("object-attributes");
        } else if a.kind == "TA" {
            note("aperture-attributes");
        }
    }

    if resolved.negative {
        note("negative-image");
    }
    let mut items = Vec::new();
    for it in &resolved.items {
        if it.group.is_some() {
            note("aperture-blocks");
            continue;
        }
        let dark = it.polarity == Polarity::Dark;
        match &it.shape {
            Shape::Flash {
                aperture,
                transform,
            } => {
                if !is_plain(transform) {
                    note("transformed-flash");
                }
                items.push(Item::Flash {
                    at: P {
                        x: round_nm(transform.e, scale),
                        y: round_nm(transform.f, scale),
                    },
                    aperture: *aperture,
                    dark,
                });
            }
            Shape::Stroke {
                aperture,
                from,
                to,
                pen,
            } => {
                let width = match file.apertures.get(aperture) {
                    Some(CoreAperture::Circle { diameter, .. }) if is_plain(pen) => {
                        round_nm(*diameter, scale)
                    }
                    _ => {
                        note("non-round-pen");
                        apertures
                            .get(aperture)
                            .map_or(0, crate::model::Aperture::stroke_width)
                    }
                };
                items.push(Item::Line {
                    from: P {
                        x: round_nm(from.0, scale),
                        y: round_nm(from.1, scale),
                    },
                    to: P {
                        x: round_nm(to.0, scale),
                        y: round_nm(to.1, scale),
                    },
                    width,
                    dark,
                });
            }
            Shape::ArcStroke {
                aperture,
                from,
                to,
                center,
                clockwise,
                pen,
            } => {
                let width = match file.apertures.get(aperture) {
                    Some(CoreAperture::Circle { diameter, .. }) if is_plain(pen) => {
                        round_nm(*diameter, scale)
                    }
                    _ => {
                        note("non-round-pen");
                        apertures
                            .get(aperture)
                            .map_or(0, crate::model::Aperture::stroke_width)
                    }
                };
                items.push(Item::Arc {
                    from: P {
                        x: round_nm(from.0, scale),
                        y: round_nm(from.1, scale),
                    },
                    to: P {
                        x: round_nm(to.0, scale),
                        y: round_nm(to.1, scale),
                    },
                    centre: P {
                        x: round_nm(center.0, scale),
                        y: round_nm(center.1, scale),
                    },
                    clockwise: *clockwise,
                    width,
                    dark,
                });
            }
            Shape::Region { contours } => {
                for contour in contours {
                    items.push(Item::Region {
                        points: flatten_contour(contour, scale, settings.arc_chord_nm),
                        dark,
                    });
                }
            }
        }
    }
    let item_sets = vec![0; items.len()];
    let mut notes: Vec<CoreNote> = file
        .diagnostics
        .iter()
        .chain(resolved.diagnostics.iter())
        .map(|d| CoreNote {
            line: d.line,
            error: d.severity == CoreSeverity::Error,
            code: d.code.to_owned(),
        })
        .collect();
    notes.sort_by(|a, b| (a.line, &a.code).cmp(&(b.line, &b.code)));
    let layer = GerberLayer {
        units,
        attributes,
        apertures,
        aperture_attributes: BTreeMap::new(),
        attribute_sets: vec![BTreeMap::new()],
        item_sets,
        items,
        findings: Vec::new(),
        ended: file.ended,
        complete: not_converted.is_empty(),
    };
    Adapted {
        layer,
        notes,
        not_converted,
    }
}

/// How a layer read by [`crate::parse::parse_gerber`] and the same file through the adapter
/// differ: the lines are empty when the two give the same drawing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Equivalence {
    /// What differs, one line each.
    pub differences: Vec<String>,
}

impl Equivalence {
    /// True when nothing differs.
    pub fn is_equal(&self) -> bool {
        self.differences.is_empty()
    }
}

fn kind_counts(layer: &GerberLayer) -> (usize, usize, usize, usize) {
    (
        layer.flashes(),
        layer.lines(),
        layer.arcs(),
        layer.regions(),
    )
}

/// Compare the old reading with the adapted one: the units, the apertures, the counts of every
/// kind of object and the objects themselves in order, each point within `tolerance_nm`. The
/// regions are compared by the number of corners and the corners within the tolerance.
pub fn compare_layers(old: &GerberLayer, new: &GerberLayer, tolerance_nm: i64) -> Equivalence {
    let mut d = Vec::new();
    if old.units != new.units {
        d.push(format!(
            "units: {} against {}",
            old.units.name(),
            new.units.name()
        ));
    }
    if old.apertures != new.apertures {
        d.push(format!(
            "apertures: {} against {}",
            old.apertures.len(),
            new.apertures.len()
        ));
    }
    if kind_counts(old) != kind_counts(new) {
        d.push(format!(
            "counts (flashes, lines, arcs, regions): {:?} against {:?}",
            kind_counts(old),
            kind_counts(new)
        ));
    }
    let near = |a: P, b: P| (a.x - b.x).abs() <= tolerance_nm && (a.y - b.y).abs() <= tolerance_nm;
    for (i, (a, b)) in old.items.iter().zip(&new.items).enumerate() {
        let same = match (a, b) {
            (
                Item::Flash {
                    at: a0,
                    aperture: n0,
                    dark: k0,
                },
                Item::Flash {
                    at: a1,
                    aperture: n1,
                    dark: k1,
                },
            ) => near(*a0, *a1) && n0 == n1 && k0 == k1,
            (
                Item::Line {
                    from: f0,
                    to: t0,
                    width: w0,
                    dark: k0,
                },
                Item::Line {
                    from: f1,
                    to: t1,
                    width: w1,
                    dark: k1,
                },
            ) => near(*f0, *f1) && near(*t0, *t1) && (w0 - w1).abs() <= tolerance_nm && k0 == k1,
            (
                Item::Arc {
                    from: f0,
                    to: t0,
                    centre: c0,
                    clockwise: r0,
                    width: w0,
                    dark: k0,
                },
                Item::Arc {
                    from: f1,
                    to: t1,
                    centre: c1,
                    clockwise: r1,
                    width: w1,
                    dark: k1,
                },
            ) => {
                near(*f0, *f1)
                    && near(*t0, *t1)
                    && near(*c0, *c1)
                    && r0 == r1
                    && (w0 - w1).abs() <= tolerance_nm
                    && k0 == k1
            }
            (
                Item::Region {
                    points: p0,
                    dark: k0,
                },
                Item::Region {
                    points: p1,
                    dark: k1,
                },
            ) => {
                k0 == k1
                    && (p0.len() == p1.len() && p0.iter().zip(p1).all(|(x, y)| near(*x, *y))
                        || region_box(p0, tolerance_nm) == region_box(p1, tolerance_nm))
            }
            _ => false,
        };
        if !same {
            d.push(format!("object {i}: {a:?} against {b:?}"));
            if d.len() > 20 {
                break;
            }
        }
    }
    Equivalence { differences: d }
}

/// The box of a region rounded to the tolerance, to compare regions whose arcs were cut in a
/// different number of pieces.
fn region_box(points: &[P], tolerance_nm: i64) -> Option<[i64; 4]> {
    let t = tolerance_nm.max(1);
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    if points.is_empty() {
        return None;
    }
    // arcs cut differently move the box by at most the chord error of either side
    Some([x0 / (4 * t), y0 / (4 * t), x1 / (4 * t), y1 / (4 * t)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_gerber;

    const BOARD: &str = "%FSLAX46Y46*%\n%MOMM*%\n%TF.FileFunction,Copper,L1,Top*%\n%ADD10C,0.200000*%\n%ADD11R,1.500000X1.000000*%\n%ADD12O,2.000000X1.000000*%\nG01*\nD10*\nX1000000Y1000000D02*\nX5000000Y1000000D01*\nX5000000Y3000000D01*\nD11*\nX2000000Y2000000D03*\nD12*\nX4000000Y4000000D03*\nG36*\nX6000000Y6000000D02*\nX8000000Y6000000D01*\nX8000000Y8000000D01*\nX6000000Y8000000D01*\nX6000000Y6000000D01*\nG37*\nM02*\n";

    fn settings() -> ImportSettings {
        ImportSettings::default()
    }

    #[test]
    fn a_board_of_flashes_lines_and_a_region_gives_the_same_drawing_through_both_readers() {
        let old = parse_gerber(BOARD, &settings()).unwrap();
        let new = adapt_gerber(BOARD, &settings());
        assert!(new.layer.complete, "{:?}", new.not_converted);
        let eq = compare_layers(&old, &new.layer, 1);
        assert!(eq.is_equal(), "{:?}", eq.differences);
        assert_eq!(new.layer.attributes, old.attributes);
        assert_eq!(kind_counts(&new.layer), (2, 2, 0, 1));
    }

    #[test]
    fn an_arc_and_a_dark_and_a_clear_polarity_agree() {
        let text = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.150000*%\nG01*\nD10*\nX1000000Y1000000D02*\nG75*\nG03*\nX3000000Y3000000I2000000J0D01*\nG01*\n%LPC*%\nX5000000Y5000000D02*\nX6000000Y5000000D01*\n%LPD*%\nM02*\n";
        let old = parse_gerber(text, &settings()).unwrap();
        let new = adapt_gerber(text, &settings());
        let eq = compare_layers(&old, &new.layer, 1);
        assert!(eq.is_equal(), "{:?}", eq.differences);
        assert_eq!(kind_counts(&new.layer), (0, 1, 1, 0));
        assert!(matches!(new.layer.items[1], Item::Line { dark: false, .. }));
    }

    #[test]
    fn inches_are_converted_at_the_edge_to_nanometres() {
        let text = "%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.0100*%\nD10*\nX10000Y10000D03*\nM02*\n";
        let old = parse_gerber(text, &settings()).unwrap();
        let new = adapt_gerber(text, &settings());
        assert_eq!(new.layer.units, Units::Inches);
        let eq = compare_layers(&old, &new.layer, 1);
        assert!(eq.is_equal(), "{:?}", eq.differences);
    }

    #[test]
    fn what_the_adapter_cannot_give_is_named_and_the_layer_is_incomplete() {
        let text = "%FSLAX46Y46*%\n%MOMM*%\n%AMTHERM*7,0,0,1.0,0.5,0.1,45*%\n%ADD10THERM*%\nD10*\nX1000000Y1000000D03*\nM02*\n";
        let new = adapt_gerber(text, &settings());
        assert!(!new.layer.complete);
        assert!(new.not_converted.contains(&"macro-aperture"));
        // the flash is still put in place
        assert_eq!(kind_counts(&new.layer).0, 1);
    }

    #[test]
    fn a_damaged_file_gives_notes_and_never_panics() {
        let new = adapt_gerber(
            "%FSLAX46Y46*%\nD10*\nX1Y1D03*\n%ADD10C,*%\nG2*\n",
            &settings(),
        );
        assert!(!new.notes.is_empty());
        let none = adapt_gerber("", &settings());
        assert!(none.layer.items.is_empty());
    }
}
