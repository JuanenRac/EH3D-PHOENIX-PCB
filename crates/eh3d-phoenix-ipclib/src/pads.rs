// SPDX-License-Identifier: GPL-3.0-or-later
//! The pads of a footprint in one shape, whatever file they came from: the footprints of an EAGLE
//! library, of a KiCad file and of this library are all read as a list of pads with a name, a
//! centre, a size and a hole, with y up and every length in nanometres.

use eh3d_phoenix_board::{Footprint, PadShape as KicadShape};
use eh3d_phoenix_libedit::geom::{auto_diameter, quarter_turns};
use eh3d_phoenix_libedit::model::{Item, Package, PadShape};

use crate::fp::IpcFootprint;

/// A pad as a box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadGeom {
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
    /// The diameter of the hole; zero for a surface-mount pad.
    pub drill: i64,
    /// False when the file says that the pad has no copper.
    pub copper: bool,
}

/// The pads of a footprint of an EAGLE library.
pub fn from_package(package: &Package) -> Vec<PadGeom> {
    let mut out = Vec::new();
    for item in &package.items {
        match item {
            Item::Smd(s) => {
                let turned = matches!(quarter_turns(&s.rot), Some(1) | Some(3));
                let (w, h) = if turned { (s.dy, s.dx) } else { (s.dx, s.dy) };
                out.push(PadGeom {
                    name: s.name.clone(),
                    x: s.x,
                    y: s.y,
                    w,
                    h,
                    drill: 0,
                    copper: s.dx > 0 && s.dy > 0 && (s.layer == 1 || s.layer == 16),
                });
            }
            Item::Pad(p) => {
                let dia = if p.diameter > 0 {
                    p.diameter
                } else {
                    auto_diameter(p.drill)
                };
                let long = matches!(p.shape, PadShape::Long | PadShape::Offset);
                let turned = matches!(quarter_turns(&p.rot), Some(1) | Some(3));
                let (w, h) = match (long, turned) {
                    (true, false) => (2 * dia, dia),
                    (true, true) => (dia, 2 * dia),
                    (false, _) => (dia, dia),
                };
                out.push(PadGeom {
                    name: p.name.clone(),
                    x: p.x,
                    y: p.y,
                    w,
                    h,
                    drill: p.drill,
                    copper: dia > 0,
                });
            }
            _ => {}
        }
    }
    out
}

/// The pads of a footprint read from a KiCad file.
pub fn from_kicad(footprint: &Footprint) -> Vec<PadGeom> {
    footprint
        .pads
        .values()
        .map(|pad| {
            let (w, h) = match pad.shape {
                KicadShape::Circle { diameter } => (diameter.get(), diameter.get()),
                KicadShape::Rect { width, height } | KicadShape::Oblong { width, height } => {
                    (width.get(), height.get())
                }
            };
            let quarters = (pad.rotation.normalised().get() + 45_000_000) / 90_000_000;
            let (w, h) = if quarters % 2 == 1 { (h, w) } else { (w, h) };
            PadGeom {
                name: pad.number.clone(),
                x: pad.at.x_nm.get(),
                y: pad.at.y_nm.get(),
                w,
                h,
                drill: pad.drill.map_or(0, |d| d.get()),
                copper: !pad.layers.is_empty() && w > 0 && h > 0,
            }
        })
        .collect()
}

/// The pads of a footprint of this library.
pub fn from_ipc(fp: &IpcFootprint) -> Vec<PadGeom> {
    fp.pads
        .iter()
        .map(|p| PadGeom {
            name: p.name.clone(),
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
            drill: p.drill,
            copper: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_libedit::model::{Pad, Smd};

    #[test]
    fn the_pads_of_an_eagle_footprint_are_read_with_their_turns() {
        let package = Package {
            name: "X".to_owned(),
            description: Vec::new(),
            items: vec![
                Item::Smd(Smd {
                    name: "1".to_owned(),
                    dx: 800_000,
                    dy: 900_000,
                    rot: "R90".to_owned(),
                    ..Smd::default()
                }),
                Item::Smd(Smd {
                    name: "2".to_owned(),
                    dx: 0,
                    dy: 900_000,
                    ..Smd::default()
                }),
                Item::Pad(Pad {
                    name: "3".to_owned(),
                    drill: 800_000,
                    diameter: 0,
                    shape: PadShape::Long,
                    ..Pad::default()
                }),
            ],
            extra: Vec::new(),
        };
        let pads = from_package(&package);
        assert_eq!(pads.len(), 3);
        assert_eq!(
            (pads[0].w, pads[0].h, pads[0].copper),
            (900_000, 800_000, true)
        );
        assert!(!pads[1].copper);
        let dia = auto_diameter(800_000);
        assert_eq!(
            (pads[2].w, pads[2].h, pads[2].drill),
            (2 * dia, dia, 800_000)
        );
    }

    #[test]
    fn the_pads_of_this_library_keep_their_boxes() {
        let set = crate::family::FamilySet::builtin().unwrap();
        let fp = crate::generate::generate(
            &set,
            "chip",
            "0603",
            crate::family::Density::B,
            &crate::generate::Params::new(),
        )
        .unwrap();
        let pads = from_ipc(&fp);
        assert_eq!(pads.len(), 2);
        assert_eq!(pads[0].w, fp.pads[0].w);
        assert!(pads.iter().all(|p| p.copper && p.drill == 0));
    }
}
