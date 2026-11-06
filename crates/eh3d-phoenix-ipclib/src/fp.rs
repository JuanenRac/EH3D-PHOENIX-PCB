// SPDX-License-Identifier: GPL-3.0-or-later
//! A footprint of the library: pads, silkscreen, assembly outline and courtyard as integers in
//! nanometres with y up, and its two ways out: a footprint of the library editor (saved as an
//! EAGLE library) and a `.kicad_mod` file.

use std::path::Path;
use std::sync::Arc;

use eh3d_phoenix_libedit::model::{
    Circle, Item, Library, Package, Pad, PadShape as EaglePadShape, Smd, Text, Wire, LAYER_TOP,
    LAYER_T_DOCU, LAYER_T_KEEPOUT, LAYER_T_NAMES, LAYER_T_PLACE, LAYER_T_VALUES,
};
use eh3d_phoenix_libedit::write::{fmt_mm, save_new, Saved};
use serde_json::{json, Value};

use crate::codes::{IpcError, WRITE_FAILED};
use crate::family::Density;

/// A rectangle as `(x1, y1, x2, y2)` with `x1 <= x2` and `y1 <= y2`.
pub type Rect = (i64, i64, i64, i64);

/// The shape of a pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadKind {
    /// A rectangle.
    Rect,
    /// A circle.
    Round,
    /// A rectangle with round ends.
    Oval,
}

impl PadKind {
    /// The word for the shape in a report.
    pub fn word(self) -> &'static str {
        match self {
            PadKind::Rect => "rect",
            PadKind::Round => "round",
            PadKind::Oval => "oval",
        }
    }
}

/// A pad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPad {
    /// The name.
    pub name: String,
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The size along x.
    pub w: i64,
    /// The size along y.
    pub h: i64,
    /// The shape.
    pub shape: PadKind,
    /// The diameter of the hole; zero for a surface-mount pad.
    pub drill: i64,
    /// True when the solder mask is open over the pad.
    pub mask: bool,
    /// The percentage of the pad that the paste covers.
    pub paste: u32,
}

impl IpcPad {
    /// The box of the pad.
    pub fn rect(&self) -> Rect {
        let left = self.x - self.w / 2;
        let bottom = self.y - self.h / 2;
        (left, bottom, left + self.w, bottom + self.h)
    }

    /// True for a surface-mount pad.
    pub fn is_smd(&self) -> bool {
        self.drill == 0
    }
}

/// A line of the silkscreen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcLine {
    /// The start.
    pub x1: i64,
    /// The start.
    pub y1: i64,
    /// The end.
    pub x2: i64,
    /// The end.
    pub y2: i64,
    /// The width.
    pub width: i64,
}

/// A dot of the silkscreen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcDot {
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The radius.
    pub r: i64,
}

/// A footprint of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcFootprint {
    /// The name, by the naming of IPC-7351B.
    pub name: String,
    /// The family.
    pub family: String,
    /// The variant.
    pub variant: String,
    /// The prefix the name starts with.
    pub prefix: String,
    /// The density level.
    pub density: Density,
    /// The pitch of the family in nanometres; zero when the family has none to check.
    pub pitch: i64,
    /// True when the smallest distance between two pads is the pitch itself; false when it may
    /// be a multiple of it (the rows of an SOT-23 skip places).
    pub pitch_exact: bool,
    /// The pads.
    pub pads: Vec<IpcPad>,
    /// The silkscreen.
    pub silk: Vec<IpcLine>,
    /// The dots of the silkscreen.
    pub dots: Vec<IpcDot>,
    /// The assembly outline, the body of the component.
    pub body: Rect,
    /// The courtyard.
    pub court: Rect,
    /// The excess of the courtyard over the pads and the body that the density level asked for.
    pub court_excess: i64,
    /// The distance from the silkscreen to the copper that the generator kept.
    pub silk_gap: i64,
    /// What the generator says about the footprint.
    pub notes: Vec<String>,
    /// True when the dimensions of the family were verified against datasheets.
    pub verified: bool,
}

/// The box that holds two boxes.
pub fn union(a: Rect, b: Rect) -> Rect {
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

/// A box grown by `d` on every side.
pub fn grow(r: Rect, d: i64) -> Rect {
    (r.0 - d, r.1 - d, r.2 + d, r.3 + d)
}

/// The box that holds all the pads.
pub fn pads_bounds(pads: &[IpcPad]) -> Option<Rect> {
    let mut iter = pads.iter().map(IpcPad::rect);
    let first = iter.next()?;
    Some(iter.fold(first, union))
}

impl IpcFootprint {
    /// The courtyard that the density level asks for: the box of the pads and the body, grown.
    pub fn required_courtyard(&self) -> Option<Rect> {
        let pads = pads_bounds(&self.pads)?;
        Some(grow(union(pads, self.body), self.court_excess))
    }

    /// The footprint as a footprint of the library editor.
    pub fn to_package(&self) -> Package {
        let mut items: Vec<Item> = Vec::new();
        for p in &self.pads {
            if p.is_smd() {
                items.push(Item::Smd(Smd {
                    name: p.name.clone(),
                    x: p.x,
                    y: p.y,
                    dx: p.w,
                    dy: p.h,
                    layer: LAYER_TOP,
                    stop: p.mask,
                    cream: p.paste > 0,
                    ..Smd::default()
                }));
            } else {
                let shape = match p.shape {
                    PadKind::Rect => EaglePadShape::Square,
                    PadKind::Round => EaglePadShape::Round,
                    PadKind::Oval => EaglePadShape::Long,
                };
                items.push(Item::Pad(Pad {
                    name: p.name.clone(),
                    x: p.x,
                    y: p.y,
                    drill: p.drill,
                    diameter: p.w.min(p.h),
                    shape,
                    stop: p.mask,
                    ..Pad::default()
                }));
            }
        }
        for l in &self.silk {
            items.push(wire(LAYER_T_PLACE, l.x1, l.y1, l.x2, l.y2, l.width));
        }
        for d in &self.dots {
            items.push(Item::Circle(Circle {
                layer: LAYER_T_PLACE,
                x: d.x,
                y: d.y,
                radius: d.r,
                width: 0,
                extra: Vec::new(),
            }));
        }
        items.extend(box_wires(LAYER_T_DOCU, self.body, 100_000));
        items.extend(box_wires(LAYER_T_KEEPOUT, self.court, 50_000));
        items.push(Item::Text(Text {
            layer: LAYER_T_NAMES,
            x: self.court.0,
            y: self.court.3 + 300_000,
            size: 1_000_000,
            content: ">NAME".to_owned(),
            ..Text::default()
        }));
        items.push(Item::Text(Text {
            layer: LAYER_T_VALUES,
            x: self.court.0,
            y: self.court.1 - 300_000 - 1_000_000,
            size: 1_000_000,
            content: ">VALUE".to_owned(),
            ..Text::default()
        }));
        Package {
            name: self.name.clone(),
            description: Vec::new(),
            items,
            extra: Vec::new(),
        }
    }

    /// The text of a `.kicad_mod` file with the footprint.
    pub fn to_kicad_mod(&self) -> String {
        let mut out = String::new();
        let through = self.pads.iter().any(|p| !p.is_smd());
        out.push_str(&format!(
            "(footprint \"{}\" (version 20240108) (generator \"phoenix\") (generator_version \"0\") (layer \"F.Cu\")\n",
            self.name
        ));
        out.push_str(&format!(
            "  (descr \"{} {}, density {}, IPC-7351B, dimensions not verified against a datasheet\")\n",
            self.family,
            self.variant,
            self.density.key()
        ));
        out.push_str(&format!(
            "  (attr {})\n",
            if through { "through_hole" } else { "smd" }
        ));
        let label_x = (self.court.0 + self.court.2) / 2;
        out.push_str(&format!(
            "  (property \"Reference\" \"REF**\" (at {} {} 0) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.15))))\n",
            fmt_mm(label_x),
            fmt_mm(-(self.court.3 + 1_000_000))
        ));
        out.push_str(&format!(
            "  (property \"Value\" \"{}\" (at {} {} 0) (layer \"F.Fab\") (effects (font (size 1 1) (thickness 0.15))))\n",
            self.name,
            fmt_mm(label_x),
            fmt_mm(-(self.court.1 - 1_000_000))
        ));
        for l in &self.silk {
            out.push_str(&kicad_line(l.x1, l.y1, l.x2, l.y2, l.width, "F.SilkS"));
        }
        for d in &self.dots {
            out.push_str(&format!(
                "  (fp_circle (center {} {}) (end {} {}) (stroke (width 0.15) (type solid)) (fill solid) (layer \"F.SilkS\"))\n",
                fmt_mm(d.x),
                fmt_mm(-d.y),
                fmt_mm(d.x + d.r),
                fmt_mm(-d.y)
            ));
        }
        out.push_str(&kicad_rect(self.body, 100_000, "F.Fab"));
        out.push_str(&kicad_rect(self.court, 50_000, "F.CrtYd"));
        for p in &self.pads {
            out.push_str(&kicad_pad(p));
        }
        out.push_str(")\n");
        out
    }

    /// The footprint as data, with every length in nanometres.
    pub fn to_json(&self) -> Value {
        let pads: Vec<Value> = self
            .pads
            .iter()
            .map(|p| {
                json!({
                    "name": p.name,
                    "x": p.x,
                    "y": p.y,
                    "w": p.w,
                    "h": p.h,
                    "shape": p.shape.word(),
                    "drill": p.drill,
                    "mask": p.mask,
                    "paste": p.paste,
                })
            })
            .collect();
        let silk: Vec<Value> = self
            .silk
            .iter()
            .map(|l| json!([l.x1, l.y1, l.x2, l.y2, l.width]))
            .collect();
        let dots: Vec<Value> = self.dots.iter().map(|d| json!([d.x, d.y, d.r])).collect();
        json!({
            "name": self.name,
            "family": self.family,
            "variant": self.variant,
            "density": self.density.key(),
            "verified": self.verified,
            "unit": "nm",
            "pitch": self.pitch,
            "pitch_exact": self.pitch_exact,
            "pads": pads,
            "silk": silk,
            "dots": dots,
            "body": [self.body.0, self.body.1, self.body.2, self.body.3],
            "courtyard": [self.court.0, self.court.1, self.court.2, self.court.3],
            "notes": self.notes,
        })
    }
}

fn wire(layer: u32, x1: i64, y1: i64, x2: i64, y2: i64, width: i64) -> Item {
    Item::Wire(Wire {
        layer,
        x1,
        y1,
        x2,
        y2,
        width,
        ..Wire::default()
    })
}

fn box_wires(layer: u32, r: Rect, width: i64) -> Vec<Item> {
    vec![
        wire(layer, r.0, r.1, r.2, r.1, width),
        wire(layer, r.2, r.1, r.2, r.3, width),
        wire(layer, r.2, r.3, r.0, r.3, width),
        wire(layer, r.0, r.3, r.0, r.1, width),
    ]
}

fn kicad_line(x1: i64, y1: i64, x2: i64, y2: i64, width: i64, layer: &str) -> String {
    format!(
        "  (fp_line (start {} {}) (end {} {}) (stroke (width {}) (type solid)) (layer \"{layer}\"))\n",
        fmt_mm(x1),
        fmt_mm(-y1),
        fmt_mm(x2),
        fmt_mm(-y2),
        fmt_mm(width)
    )
}

fn kicad_rect(r: Rect, width: i64, layer: &str) -> String {
    format!(
        "  (fp_rect (start {} {}) (end {} {}) (stroke (width {}) (type solid)) (fill none) (layer \"{layer}\"))\n",
        fmt_mm(r.0),
        fmt_mm(-r.3),
        fmt_mm(r.2),
        fmt_mm(-r.1),
        fmt_mm(width)
    )
}

fn kicad_pad(p: &IpcPad) -> String {
    let shape = p.shape.word();
    let at = format!("(at {} {})", fmt_mm(p.x), fmt_mm(-p.y));
    let size = format!("(size {} {})", fmt_mm(p.w), fmt_mm(p.h));
    if p.is_smd() {
        let mut layers = String::from("\"F.Cu\"");
        if p.paste > 0 {
            layers.push_str(" \"F.Paste\"");
        }
        if p.mask {
            layers.push_str(" \"F.Mask\"");
        }
        let margin = if p.paste > 0 && p.paste < 100 {
            let ratio = (f64::from(p.paste) / 100.0).sqrt() - 1.0;
            format!(" (solder_paste_margin_ratio {ratio:.4})")
        } else {
            String::new()
        };
        format!(
            "  (pad \"{}\" smd {shape} {at} {size} (layers {layers}){margin})\n",
            p.name
        )
    } else {
        let layers = if p.mask {
            "\"*.Cu\" \"*.Mask\""
        } else {
            "\"*.Cu\""
        };
        format!(
            "  (pad \"{}\" thru_hole {shape} {at} {size} (drill {}) (layers {layers}))\n",
            p.name,
            fmt_mm(p.drill)
        )
    }
}

/// A library of the editor with these footprints.
pub fn to_library(footprints: &[IpcFootprint]) -> Library {
    let mut lib = Library::empty();
    for fp in footprints {
        lib.packages.push(Arc::new(fp.to_package()));
    }
    lib
}

/// Saves the footprints as a new EAGLE library, which is read back and compared; a file that
/// exists is never replaced.
pub fn save_lbr(footprints: &[IpcFootprint], out: &Path) -> Result<Saved, IpcError> {
    let lib = to_library(footprints);
    save_new(&lib, None, out).map_err(|e| IpcError::new(WRITE_FAILED, e.to_string()))
}

/// Saves one footprint as a new `.kicad_mod` file; a file that exists is never replaced.
pub fn save_kicad_mod(fp: &IpcFootprint, out: &Path) -> Result<usize, IpcError> {
    use std::io::Write;
    let fail = |e: std::io::Error| IpcError::new(WRITE_FAILED, format!("{}: {e}", out.display()));
    let text = fp.to_kicad_mod();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)?;
    file.sync_all().map_err(fail)?;
    Ok(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> IpcFootprint {
        let pad = |name: &str, x: i64| IpcPad {
            name: name.to_owned(),
            x,
            y: 0,
            w: 800_000,
            h: 900_000,
            shape: PadKind::Rect,
            drill: 0,
            mask: true,
            paste: 100,
        };
        IpcFootprint {
            name: "RESC1608X55N".to_owned(),
            family: "chip".to_owned(),
            variant: "0603".to_owned(),
            prefix: "RESC".to_owned(),
            density: Density::B,
            pitch: 0,
            pitch_exact: false,
            pads: vec![pad("1", -800_000), pad("2", 800_000)],
            silk: vec![IpcLine {
                x1: -1_200_000,
                y1: 700_000,
                x2: 1_200_000,
                y2: 700_000,
                width: 150_000,
            }],
            dots: Vec::new(),
            body: (-800_000, -400_000, 800_000, 400_000),
            court: (-1_650_000, -700_000, 1_650_000, 700_000),
            court_excess: 250_000,
            silk_gap: 200_000,
            notes: Vec::new(),
            verified: false,
        }
    }

    #[test]
    fn a_pad_has_its_box_and_the_boxes_are_joined() {
        let fp = sample();
        assert_eq!(fp.pads[0].rect(), (-1_200_000, -450_000, -400_000, 450_000));
        let all = pads_bounds(&fp.pads).unwrap();
        assert_eq!(all, (-1_200_000, -450_000, 1_200_000, 450_000));
        assert_eq!(
            fp.required_courtyard().unwrap(),
            (-1_450_000, -700_000, 1_450_000, 700_000)
        );
        assert_eq!(grow((0, 0, 1, 1), 2), (-2, -2, 3, 3));
        assert!(pads_bounds(&[]).is_none());
    }

    #[test]
    fn the_package_has_the_pads_the_silkscreen_the_outlines_and_the_texts() {
        let package = sample().to_package();
        assert_eq!(package.name, "RESC1608X55N");
        let smd = package
            .items
            .iter()
            .filter(|i| matches!(i, Item::Smd(_)))
            .count();
        assert_eq!(smd, 2);
        let wires = package
            .items
            .iter()
            .filter(|i| matches!(i, Item::Wire(_)))
            .count();
        assert_eq!(wires, 1 + 4 + 4);
        let texts = package
            .items
            .iter()
            .filter(|i| matches!(i, Item::Text(_)))
            .count();
        assert_eq!(texts, 2);
    }

    #[test]
    fn the_kicad_file_flips_y_and_names_the_layers() {
        let text = sample().to_kicad_mod();
        assert!(text.starts_with("(footprint \"RESC1608X55N\""));
        assert!(text.contains("(attr smd)"));
        assert!(text.contains("(pad \"1\" smd rect (at -0.8 0) (size 0.8 0.9)"));
        assert!(text.contains("(fp_line (start -1.2 -0.7) (end 1.2 -0.7)"));
        assert!(text.contains("\"F.CrtYd\""));
        assert!(text.trim_end().ends_with(')'));
    }

    #[test]
    fn a_through_hole_pad_is_written_with_its_drill_and_a_partial_paste_with_its_margin() {
        let mut fp = sample();
        fp.pads[0].drill = 800_000;
        fp.pads[0].shape = PadKind::Round;
        fp.pads[1].paste = 60;
        let text = fp.to_kicad_mod();
        assert!(text.contains("thru_hole round (at -0.8 0) (size 0.8 0.9) (drill 0.8)"));
        assert!(text.contains("(solder_paste_margin_ratio -0.2254)"));
        assert!(text.contains("(attr through_hole)"));
    }

    #[test]
    fn the_json_holds_every_pad_in_nanometres() {
        let value = sample().to_json();
        assert_eq!(value["unit"], "nm");
        assert_eq!(value["pads"][1]["x"], 800_000);
        assert_eq!(value["pads"].as_array().unwrap().len(), 2);
        assert_eq!(value["density"], "B");
    }

    #[test]
    fn a_library_file_is_written_once_and_never_over_a_file_that_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("one.lbr");
        let saved = save_lbr(&[sample()], &path).unwrap();
        assert_eq!(saved.path, path);
        assert!(path.exists());
        let e = save_lbr(&[sample()], &path).unwrap_err();
        assert_eq!(e.code, WRITE_FAILED);
        let mod_path = dir.path().join("one.kicad_mod");
        let size = save_kicad_mod(&sample(), &mod_path).unwrap();
        assert!(size > 100);
        assert_eq!(
            save_kicad_mod(&sample(), &mod_path).unwrap_err().code,
            WRITE_FAILED
        );
    }
}
