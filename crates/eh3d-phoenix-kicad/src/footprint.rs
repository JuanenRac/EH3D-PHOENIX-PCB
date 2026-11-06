// SPDX-License-Identifier: GPL-3.0-or-later
//! Footprints: a `.kicad_mod` file, or a `footprint` placed on a board, read into the footprints of
//! the board model.
//!
//! The model holds a footprint as a name and its pads. A pad keeps its number, its shape (a circle,
//! a rectangle or an oblong; rounded rectangles, trapezoids and custom shapes are read as the
//! rectangle of their size), its place relative to the origin of the footprint, its turn
//! relative to the footprint, its drill and the layers it appears on. The silkscreen, fabrication
//! and courtyard drawing, the texts, the fields and the 3D model are not kept.
//!
//! KiCad writes the angle of a pad as an angle on the board, so the angle of the footprint is
//! taken off to get the angle relative to the footprint.

use std::collections::BTreeMap;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::{Footprint, Id, Layer, Pad, PadShape};
use eh3d_phoenix_hub::QualityReport;
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::conv::{at_args, check_root, decode, nm_text, to_model, xy_args, Ctx, P};
use crate::error::KicadError;
use crate::layers::LayerMap;
use crate::settings::KicadSettings;
use crate::sexpr::{parse, Node};

/// The net of a pad as the file names it: a number, a name or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetRef {
    pub id: Option<i64>,
    pub name: Option<String>,
}

/// A pad as the file says it (positions with y down).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PadDef {
    pub number: String,
    pub kind: String,
    pub shape: String,
    pub at: P,
    pub angle_udeg: i64,
    pub size: P,
    pub drill: Option<i64>,
    /// The roundness of a `roundrect` pad in the units of the model (see `Pad::roundness`).
    pub roundness: u16,
    pub layers: Vec<String>,
    pub net: Option<NetRef>,
}

/// A footprint as the file says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FootprintDef {
    pub name: String,
    pub bottom: bool,
    pub at: P,
    pub angle_udeg: i64,
    pub reference: String,
    pub value: String,
    pub pads: Vec<PadDef>,
    pub extras: usize,
}

pub(crate) fn net_ref(node: &Node) -> Option<NetRef> {
    let first = node.arg_text(0)?;
    Some(match node.arg_text(1) {
        Some(second) => NetRef {
            id: first.parse().ok(),
            name: Some(second.to_owned()),
        },
        None => match first.parse::<i64>() {
            Ok(id) => NetRef {
                id: Some(id),
                name: None,
            },
            Err(_) => NetRef {
                id: None,
                name: Some(first.to_owned()),
            },
        },
    })
}

fn drill_of(pad: &Node) -> Option<i64> {
    let d = pad.child("drill")?;
    let size = if d.arg_text(0) == Some("oval") {
        let w = nm_text(d.arg_text(1)?)?;
        let h = d.arg_text(2).and_then(nm_text).unwrap_or(w);
        w.max(h)
    } else {
        nm_text(d.arg_text(0)?)?
    };
    (size > 0).then_some(size)
}

/// The roundness of a `roundrect` pad from its `roundrect_rratio` (0.25 when the file does not
/// say), in the units of `Pad::roundness`: the ratio of KiCad times two thousand, at most 1000.
fn roundness_of(pad: &Node, shape: &str) -> u16 {
    if shape != "roundrect" {
        return 0;
    }
    let ratio = pad
        .child("roundrect_rratio")
        .and_then(|r| r.arg_text(0))
        .and_then(|t| t.parse::<f64>().ok())
        .filter(|r| r.is_finite())
        .unwrap_or(0.25);
    (ratio * 2000.0).round().clamp(0.0, 1000.0) as u16
}

fn parse_pad(node: &Node, ctx: &mut Ctx) -> Option<PadDef> {
    let number = node.arg_text(0).unwrap_or("").to_owned();
    let kind = node.arg_text(1).unwrap_or("").to_owned();
    let shape = node.arg_text(2).unwrap_or("").to_owned();
    let shape_name = shape.clone();
    let Some(at_node) = node.child("at") else {
        ctx.invalid(node, "pad", "it has no position");
        return None;
    };
    let Some((at, angle_udeg)) = at_args(at_node) else {
        ctx.invalid(at_node, "pad position", "not two numbers");
        return None;
    };
    let drill = drill_of(node);
    if node
        .child("drill")
        .is_some_and(|d| d.arg_text(0) == Some("oval"))
    {
        ctx.simplify("oval drill", "read as a round hole of the larger size");
    }
    let Some(size_node) = node.child("size") else {
        ctx.invalid(node, "pad", "it has no size");
        return None;
    };
    let size = match xy_args(size_node) {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        Some(_) if kind == "np_thru_hole" && drill.is_some() => {
            let d = drill.unwrap_or(0);
            (d, d)
        }
        _ => {
            ctx.invalid(size_node, "pad size", "not two numbers above zero");
            return None;
        }
    };
    let mut layers = Vec::new();
    if let Some(list) = node.child("layers") {
        for l in list.args() {
            if let Some(name) = l.text() {
                layers.push(name.to_owned());
            }
        }
    }
    Some(PadDef {
        number,
        kind,
        shape,
        at,
        angle_udeg,
        size,
        drill,
        roundness: roundness_of(node, &shape_name),
        layers,
        net: node.child("net").and_then(net_ref),
    })
}

/// Reads a `footprint` (or the `module` of older files).
pub(crate) fn parse_footprint(node: &Node, ctx: &mut Ctx) -> Option<FootprintDef> {
    let Some(name) = node.arg_text(0) else {
        ctx.invalid(node, "footprint", "it has no name");
        return None;
    };
    let mut def = FootprintDef {
        name: name.to_owned(),
        bottom: node.child("layer").and_then(|l| l.arg_text(0)) == Some("B.Cu"),
        at: (0, 0),
        angle_udeg: 0,
        reference: String::new(),
        value: String::new(),
        pads: Vec::new(),
        extras: 0,
    };
    if let Some(at_node) = node.child("at") {
        match at_args(at_node) {
            Some((at, angle)) => {
                def.at = at;
                def.angle_udeg = angle;
            }
            None => ctx.invalid(at_node, "footprint position", "not two numbers"),
        }
    }
    let mut pads_seen = 0usize;
    for child in node.args() {
        match child.head() {
            Some("pad") => {
                pads_seen += 1;
                if pads_seen > ctx.settings.max_items {
                    ctx.ignore("pad (over max_items)");
                    continue;
                }
                if let Some(pad) = parse_pad(child, ctx) {
                    def.pads.push(pad);
                }
            }
            Some("property") => match child.arg_text(0) {
                Some("Reference") => def.reference = child.arg_text(1).unwrap_or("").to_owned(),
                Some("Value") => def.value = child.arg_text(1).unwrap_or("").to_owned(),
                _ => def.extras += 1,
            },
            Some("fp_text") => match child.arg_text(0) {
                Some("reference") => def.reference = child.arg_text(1).unwrap_or("").to_owned(),
                Some("value") => def.value = child.arg_text(1).unwrap_or("").to_owned(),
                _ => def.extras += 1,
            },
            Some(
                "fp_line" | "fp_rect" | "fp_circle" | "fp_arc" | "fp_poly" | "fp_curve"
                | "fp_text_box" | "model" | "dimension" | "zone" | "group" | "image",
            ) => def.extras += 1,
            _ => {}
        }
    }
    Some(def)
}

/// A footprint built for the model with the ids of its pads in the order of the pads of the file.
pub(crate) struct Built {
    pub footprint: Footprint,
    pub pad_ids: Vec<Id>,
}

fn shape_of(pad: &PadDef, ctx: &mut Ctx) -> PadShape {
    let width = Nm::new(pad.size.0).unwrap_or(Nm::ZERO);
    let height = Nm::new(pad.size.1).unwrap_or(Nm::ZERO);
    match pad.shape.as_str() {
        "circle" => PadShape::Circle { diameter: width },
        "rect" | "roundrect" => PadShape::Rect { width, height },
        "oval" if pad.size.0 == pad.size.1 => PadShape::Circle { diameter: width },
        "oval" => PadShape::Oblong { width, height },
        other => {
            ctx.simplify(
                &format!("pad shape `{other}`"),
                "read as the rectangle of its size",
            );
            ctx.transform("pads with a simpler shape", 1);
            PadShape::Rect { width, height }
        }
    }
}

/// Builds the model footprint. The angle of each pad is taken relative to the footprint.
pub(crate) fn build_footprint(def: &FootprintDef, layers: &mut LayerMap, ctx: &mut Ctx) -> Built {
    let origin = Point {
        x_nm: Nm::ZERO,
        y_nm: Nm::ZERO,
    };
    let mut pads = BTreeMap::new();
    let mut pad_ids = Vec::new();
    for pad in &def.pads {
        let mut layer_ids: Vec<Id> = Vec::new();
        for name in &pad.layers {
            for id in layers.expand(name, ctx) {
                if !layer_ids.contains(&id) {
                    layer_ids.push(id);
                }
            }
        }
        let relative = (pad.angle_udeg - def.angle_udeg) % 360_000_000;
        let id = ctx.id();
        let shape = shape_of(pad, ctx);
        pads.insert(
            id,
            Pad {
                number: pad.number.clone(),
                shape,
                at: to_model(pad.at).unwrap_or(origin),
                rotation: Udeg::new(relative).unwrap_or(Udeg::ZERO),
                drill: pad.drill.and_then(|d| Nm::new(d).ok()),
                layers: layer_ids,
                roundness: pad.roundness,
                plated: pad.kind != "np_thru_hole",
            },
        );
        pad_ids.push(id);
    }
    Built {
        footprint: Footprint {
            name: def.name.clone(),
            pads,
        },
        pad_ids,
    }
}

/// A footprint file read into the model.
#[derive(Debug, Clone)]
pub struct ReadFootprint {
    /// The id of the footprint.
    pub id: Id,
    /// The footprint.
    pub footprint: Footprint,
    /// The layers its pads appear on, by id.
    pub layers: BTreeMap<Id, Layer>,
    /// The reference the file gives (`REF**` in a library).
    pub reference: String,
    /// The value the file gives.
    pub value: String,
    /// The version the file declares.
    pub version: String,
    /// The program that wrote the file.
    pub generator: String,
    /// What was kept, changed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Reads a `.kicad_mod` file. Ids for the new objects come from `new_id`.
pub fn read_footprint(
    bytes: &[u8],
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<ReadFootprint, Diagnostic> {
    let fail = |e: KicadError| e.diagnostic();
    let (text, encoding) = decode(bytes, settings).map_err(fail)?;
    let root = parse(&text, settings).map_err(fail)?;
    let mut ctx = Ctx::new(settings, new_id);
    if let Some(note) = encoding {
        ctx.push(note);
    }
    let (version, generator) = check_root(
        &root,
        &["footprint", "module"],
        "a footprint (footprint or module)",
        &mut ctx,
    )
    .map_err(fail)?;
    let Some(def) = parse_footprint(&root, &mut ctx) else {
        let (mut diagnostics, _) = ctx.finish();
        return Err(diagnostics.pop().unwrap_or_else(|| {
            KicadError::NotKicad {
                expected: "a footprint with a name".to_owned(),
                found: "a footprint without a name".to_owned(),
            }
            .diagnostic()
        }));
    };
    let mut layers = LayerMap::default();
    let built = build_footprint(&def, &mut layers, &mut ctx);
    ctx.keep("footprints", 1);
    ctx.keep("pads", built.pad_ids.len());
    ctx.lose("footprint graphics, texts and 3D models", def.extras);
    let id = ctx.id();
    let (diagnostics, quality) = ctx.finish();
    Ok(ReadFootprint {
        id,
        footprint: built.footprint,
        layers: layers.layers,
        reference: def.reference,
        value: def.value,
        version,
        generator,
        quality,
        diagnostics,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A KiCad 6 footprint: a resistor of two surface pads and a through-hole pad of every shape.
    pub(crate) const MOD_V6: &str = r#"(footprint "R_0603_1608Metric" (version 20211014) (generator pcbnew)
  (layer "F.Cu")
  (descr "Resistor SMD 0603")
  (tags "resistor")
  (attr smd)
  (fp_text reference "REF**" (at 0 -1.43) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
  (fp_text value "R_0603" (at 0 1.43) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (fp_line (start -0.8 -0.4) (end 0.8 -0.4) (layer "F.Fab") (width 0.1))
  (fp_circle (center 0 0) (end 1 0) (layer "F.CrtYd") (width 0.05))
  (pad "1" smd roundrect (at -0.825 0) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
  (pad "2" smd roundrect (at 0.825 0) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
  (pad "3" thru_hole circle (at 0 2.5) (size 1.6 1.6) (drill 0.8) (layers *.Cu *.Mask))
  (pad "4" thru_hole oval (at 0 4 90) (size 1.6 2.4) (drill oval 0.8 1.6) (layers "*.Cu" "*.Mask"))
  (pad "" np_thru_hole circle (at 3 0) (size 1.2 1.2) (drill 1.2) (layers "*.Cu" "*.Mask"))
  (model "${KICAD6_3DMODEL_DIR}/R.wrl" (offset (xyz 0 0 0))))"#;

    fn read(text: &str) -> ReadFootprint {
        let mut ids = crate::sequential_ids().unwrap();
        read_footprint(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap()
    }

    #[test]
    fn a_kicad_6_footprint_gives_pads_with_shapes_drills_and_layers() {
        let fp = read(MOD_V6);
        assert_eq!(fp.footprint.name, "R_0603_1608Metric");
        assert_eq!(fp.reference, "REF**");
        assert_eq!(fp.value, "R_0603");
        assert_eq!(fp.footprint.pads.len(), 5);
        let pad = |number: &str| {
            fp.footprint
                .pads
                .values()
                .find(|p| p.number == number)
                .unwrap()
        };
        let one = pad("1");
        assert_eq!(one.at.x_nm.get(), -825_000);
        assert_eq!(one.at.y_nm.get(), 0);
        assert_eq!(one.drill, None);
        assert_eq!(one.layers.len(), 3);
        assert!(matches!(one.shape, PadShape::Rect { .. }));
        // a ratio of 0.25 is a roundness of 500 (a radius of a quarter of the shorter side)
        assert_eq!(one.roundness, 500);
        assert!(one.plated);
        assert_eq!(one.corner_radius_nm(), Some(200_000));
        let hole = pad("");
        assert!(!hole.plated && hole.is_npth());
        assert_eq!(hole.roundness, 0);
        let three = pad("3");
        assert_eq!(three.roundness, 0);
        assert!(three.plated);
        assert_eq!(three.drill.map(|d| d.get()), Some(800_000));
        assert!(matches!(three.shape, PadShape::Circle { .. }));
        // the two outer copper layers and both masks
        assert_eq!(three.layers.len(), 4);
        // the y axis is turned
        assert_eq!(three.at.y_nm.get(), -2_500_000);
        let four = pad("4");
        assert!(matches!(four.shape, PadShape::Oblong { .. }));
        // the oval hole is read as round, of the larger size
        assert_eq!(four.drill.map(|d| d.get()), Some(1_600_000));
        assert_eq!(four.rotation.get(), 90_000_000);
    }

    #[test]
    fn the_report_names_what_was_simplified_and_what_was_dropped() {
        let fp = read(MOD_V6);
        let find = |list: &[eh3d_phoenix_hub::QualityItem], name: &str| {
            list.iter().find(|i| i.category == name).map(|i| i.count)
        };
        assert_eq!(find(&fp.quality.kept, "pads"), Some(5));
        // the rounded pads and the pad that is not plated are in the model
        assert_eq!(
            find(&fp.quality.transformed, "pads with a simpler shape"),
            None
        );
        assert_eq!(
            find(
                &fp.quality.transformed,
                "non-plated pads (the plating is not in the model)"
            ),
            None
        );
        // one line, one circle, one 3D model (the reference and the value are kept as fields)
        assert_eq!(
            find(
                &fp.quality.dropped,
                "footprint graphics, texts and 3D models"
            ),
            Some(3)
        );
        assert!(!fp
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-007")
                && d.message.contains("roundrect")));
    }

    #[test]
    fn kicad_8_fields_and_the_old_module_name_are_read() {
        let v8 = r#"(footprint "C_0805" (version 20240108) (generator "pcbnew") (generator_version "8.0") (layer "F.Cu")
  (property "Reference" "C1" (at 0 0 0) (layer "F.SilkS") (effects (font (size 1 1))))
  (property "Value" "100n" (at 0 0 0) (layer "F.Fab") (effects (font (size 1 1))))
  (property "Datasheet" "" (at 0 0 0) (layer "F.Fab") (hide yes))
  (pad "1" smd rect (at -1 0) (size 1 1.25) (layers "F.Cu" "F.Paste" "F.Mask")))"#;
        let fp = read(v8);
        assert_eq!((fp.reference.as_str(), fp.value.as_str()), ("C1", "100n"));
        assert_eq!(fp.footprint.pads.len(), 1);
        let old = r#"(module Old (layer F.Cu) (tedit 5A02FE80)
  (fp_text reference REF** (at 0 0) (layer F.SilkS))
  (pad 1 thru_hole rect (at 0 0) (size 2 2) (drill 1) (layers *.Cu *.Mask)))"#;
        let fp = read(old);
        assert_eq!(fp.version, "");
        assert_eq!(fp.reference, "REF**");
        assert_eq!(fp.footprint.pads.values().next().unwrap().number, "1");
        // a file without a version is a note, not an error
        assert!(fp
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-009")));
    }

    #[test]
    fn the_angle_of_a_pad_is_relative_to_the_footprint() {
        let text = r#"(footprint "X" (version 20211014) (at 10 10 90)
  (pad "1" smd rect (at 0 0 90) (size 1 1) (layers "F.Cu"))
  (pad "2" smd rect (at 0 0 135) (size 1 1) (layers "F.Cu"))
  (pad "3" smd rect (at 0 0 45) (size 1 1) (layers "F.Cu")))"#;
        let fp = read(text);
        let angle = |n: &str| {
            fp.footprint
                .pads
                .values()
                .find(|p| p.number == n)
                .unwrap()
                .rotation
                .get()
        };
        assert_eq!(angle("1"), 0);
        assert_eq!(angle("2"), 45_000_000);
        assert_eq!(angle("3"), -45_000_000);
    }

    #[test]
    fn bad_pads_are_reported_and_skipped_without_stopping() {
        let text = r#"(footprint "X" (version 20211014)
  (pad "1" smd rect (size 1 1) (layers "F.Cu"))
  (pad "2" smd rect (at 0 0) (layers "F.Cu"))
  (pad "3" smd rect (at 0 0) (size -1 1) (layers "F.Cu"))
  (pad "4" smd rect (at a b) (size 1 1) (layers "F.Cu"))
  (pad "5" smd rect (at 0 0) (size 1 1) (layers "F.Cu")))"#;
        let fp = read(text);
        assert_eq!(fp.footprint.pads.len(), 1);
        let bad = fp
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("PHOENIX-KICAD-005"))
            .count();
        assert_eq!(bad, 4);
        // the diagnostics point at the line of the pad
        let lines: Vec<u32> = fp.diagnostics.iter().map(|d| d.line).collect();
        assert!(lines.contains(&2) && lines.contains(&5));
    }

    #[test]
    fn a_footprint_without_a_name_or_of_another_kind_is_refused() {
        let mut ids = crate::sequential_ids().unwrap();
        let s = KicadSettings::default();
        let e = read_footprint(b"(footprint)", &s, &mut ids).unwrap_err();
        assert!(e.code.is_some());
        let e = read_footprint(b"(kicad_sch (version 1))", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-003"));
        let e = read_footprint(b"(footprint \"A\" (pad \"1\"", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-004"));
        let e = read_footprint(&[0x28, 0xff, 0xfe], &s, &mut ids).unwrap_err();
        assert!(e.code.is_some());
    }

    #[test]
    fn a_file_that_is_not_utf8_is_read_with_a_note() {
        let bytes = b"(footprint \"A\xe9B\" (version 20211014) (pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\")))".to_vec();
        let mut ids = crate::sequential_ids().unwrap();
        let fp = read_footprint(&bytes, &KicadSettings::default(), &mut ids).unwrap();
        assert!(fp
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-012")));
        assert!(fp.footprint.name.starts_with('A'));
    }

    #[test]
    fn nets_are_read_in_the_three_forms() {
        let s = KicadSettings::default();
        let root = parse("(a (net 3 \"GND\") (net \"VCC\") (net 7) (net))", &s).unwrap();
        let nets: Vec<Option<NetRef>> = root.children("net").map(net_ref).collect();
        assert_eq!(
            nets[0],
            Some(NetRef {
                id: Some(3),
                name: Some("GND".into())
            })
        );
        assert_eq!(
            nets[1],
            Some(NetRef {
                id: None,
                name: Some("VCC".into())
            })
        );
        assert_eq!(
            nets[2],
            Some(NetRef {
                id: Some(7),
                name: None
            })
        );
        assert_eq!(nets[3], None);
    }
}
