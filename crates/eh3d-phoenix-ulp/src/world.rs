// SPDX-License-Identifier: GPL-3.0-or-later
//! The design as a program of the reference tool sees it, **read only**: the board and the
//! schematic of the model, the objects a walk goes over (elements, signals, layers, wires, vias,
//! polygons, packages, pads, parts, nets, sheets) and the members each object has. Nothing here
//! writes to the model: it holds shared references and the evaluator has no way to change them.
//!
//! A length of the program is a nanometre (the unit of the model), an angle is a real number in
//! degrees. A member the evaluator does not know for an object is reported by the caller as
//! "not written yet" and never answered with a made-up value.

use eh3d_phoenix_board::{Board, Id, PadShape, Segment, Side};
use eh3d_phoenix_schematic::Schematic;

use crate::codes::{RUN_NOT_WRITTEN, RUN_TYPE};
use crate::value::Value;

/// A fault of the world: a code and the detail.
pub type Fault = (&'static str, String);

/// A handle on an object of the design.
#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    /// The board.
    Board,
    /// The schematic.
    Schematic,
    /// A placed component of the board.
    Element(Id),
    /// A footprint of the board.
    Package(Id),
    /// A pad of a footprint, placed (with the component) or not.
    Pad {
        /// The component, when the pad is a placed one.
        element: Option<Id>,
        /// The footprint.
        footprint: Id,
        /// The pad.
        pad: Id,
    },
    /// A net of the board.
    Signal(Id),
    /// A layer of the board.
    Layer(Id),
    /// One piece of a track.
    Wire {
        /// The track.
        track: Id,
        /// The place of the piece in the track.
        segment: usize,
    },
    /// A via.
    Via(Id),
    /// A zone.
    Polygon(Id),
    /// A corner of the outline of a zone.
    Vertex {
        /// The zone.
        zone: Id,
        /// The place of the corner.
        index: usize,
    },
    /// A placed part of the schematic.
    Part(Id),
    /// A net of the schematic.
    SchNet(Id),
    /// A sheet of the schematic.
    Sheet(Id),
}

impl Obj {
    /// The name of the kind, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Obj::Board => "board",
            Obj::Schematic => "schematic",
            Obj::Element(_) => "element",
            Obj::Package(_) => "package",
            Obj::Pad { .. } => "pad",
            Obj::Signal(_) => "signal",
            Obj::Layer(_) => "layer",
            Obj::Wire { .. } => "wire",
            Obj::Via(_) => "via",
            Obj::Polygon(_) => "polygon",
            Obj::Vertex { .. } => "vertex",
            Obj::Part(_) => "part",
            Obj::SchNet(_) => "net",
            Obj::Sheet(_) => "sheet",
        }
    }
}

/// The names of the walks the evaluator knows (`X.name(V)`).
pub const WALKS: &[&str] = &[
    "elements", "signals", "layers", "wires", "vias", "polygons", "packages", "pads", "vertices",
    "parts", "nets", "sheets",
];

/// The names of the members the evaluator reads.
pub const MEMBERS: &[&str] = &[
    "name", "value", "package", "x", "y", "x1", "y1", "x2", "y2", "angle", "mirror", "diameter",
    "width", "height", "drill", "shape", "signal", "number", "layer", "length", "area",
];

/// The model a run reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct World<'a> {
    /// The board, when one was given.
    pub board: Option<&'a Board>,
    /// The schematic, when one was given.
    pub schematic: Option<&'a Schematic>,
}

fn nm(v: i64) -> Value {
    Value::Int(v)
}

fn text(s: &str) -> Value {
    Value::Str(s.to_owned())
}

fn not_written(what: String) -> Fault {
    (RUN_NOT_WRITTEN, what)
}

/// The place of `(x, y)` after a turn of `udeg` micro-degrees, in nanometres. A turn of a
/// multiple of a quarter is exact; any other is computed with the floating point sine and cosine
/// of the machine and rounded to the nanometre.
fn turn(x: i64, y: i64, udeg: i64) -> (i64, i64) {
    let quarter = 90_000_000;
    if udeg % quarter == 0 {
        return match udeg.rem_euclid(4 * quarter) / quarter {
            0 => (x, y),
            1 => (-y, x),
            2 => (-x, -y),
            _ => (y, -x),
        };
    }
    let (s, c) = (udeg as f64 / 1e6).to_radians().sin_cos();
    let (fx, fy) = (x as f64, y as f64);
    (
        (fx * c - fy * s).round() as i64,
        (fx * s + fy * c).round() as i64,
    )
}

impl World<'_> {
    /// The object a walk opened by `board(B)` or `schematic(S)` binds.
    pub fn root(&self, name: &str) -> Result<Option<Obj>, Fault> {
        match name {
            "board" => Ok(self.board.map(|_| Obj::Board)),
            "schematic" => Ok(self.schematic.map(|_| Obj::Schematic)),
            _ => Err(not_written(format!("function {name}"))),
        }
    }

    /// The objects a walk goes over, in the order of the model (by identity).
    pub fn walk(&self, obj: &Obj, walk: &str) -> Result<Vec<Obj>, Fault> {
        let board = self.board;
        let sch = self.schematic;
        let list = match (obj, walk) {
            (Obj::Board, "elements") => board
                .map(|b| b.components.keys().map(|k| Obj::Element(*k)).collect())
                .unwrap_or_default(),
            (Obj::Board, "signals") => board
                .map(|b| b.nets.keys().map(|k| Obj::Signal(*k)).collect())
                .unwrap_or_default(),
            (Obj::Board, "layers") => board
                .map(|b| b.layers.keys().map(|k| Obj::Layer(*k)).collect())
                .unwrap_or_default(),
            (Obj::Board, "wires") => board.map(|b| wires_of(b, None)).unwrap_or_default(),
            (Obj::Signal(n), "wires") => board.map(|b| wires_of(b, Some(*n))).unwrap_or_default(),
            (Obj::Board, "vias") => board
                .map(|b| b.vias.keys().map(|k| Obj::Via(*k)).collect())
                .unwrap_or_default(),
            (Obj::Signal(n), "vias") => board
                .map(|b| {
                    b.vias
                        .iter()
                        .filter(|(_, v)| v.net == *n)
                        .map(|(k, _)| Obj::Via(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Board, "polygons") => board
                .map(|b| b.zones.keys().map(|k| Obj::Polygon(*k)).collect())
                .unwrap_or_default(),
            (Obj::Signal(n), "polygons") => board
                .map(|b| {
                    b.zones
                        .iter()
                        .filter(|(_, z)| z.net == *n)
                        .map(|(k, _)| Obj::Polygon(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Board, "packages") => board
                .map(|b| b.footprints.keys().map(|k| Obj::Package(*k)).collect())
                .unwrap_or_default(),
            (Obj::Package(f), "pads") => board
                .and_then(|b| b.footprints.get(f))
                .map(|fp| {
                    fp.pads
                        .keys()
                        .map(|p| Obj::Pad {
                            element: None,
                            footprint: *f,
                            pad: *p,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Element(e), "pads") => board
                .and_then(|b| {
                    let c = b.components.get(e)?;
                    let fp = b.footprints.get(&c.footprint)?;
                    Some(
                        fp.pads
                            .keys()
                            .map(|p| Obj::Pad {
                                element: Some(*e),
                                footprint: c.footprint,
                                pad: *p,
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            (Obj::Polygon(z), "vertices") => board
                .and_then(|b| b.zones.get(z))
                .map(|zone| {
                    (0..zone.outline.len())
                        .map(|index| Obj::Vertex { zone: *z, index })
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Schematic, "parts") => sch
                .map(|s| s.parts.keys().map(|k| Obj::Part(*k)).collect())
                .unwrap_or_default(),
            (Obj::Schematic, "nets") => sch
                .map(|s| s.nets.keys().map(|k| Obj::SchNet(*k)).collect())
                .unwrap_or_default(),
            (Obj::Schematic, "sheets") => sch
                .map(|s| s.sheets.keys().map(|k| Obj::Sheet(*k)).collect())
                .unwrap_or_default(),
            _ => {
                return Err(not_written(format!(
                    "the walk {walk} over a {}",
                    obj.kind()
                )))
            }
        };
        Ok(list)
    }

    fn layer_number(&self, layer: &Id) -> i64 {
        self.board
            .and_then(|b| b.layers.keys().position(|k| k == layer))
            .map_or(0, |i| i as i64 + 1)
    }

    fn net_name(&self, net: &Id) -> String {
        self.board
            .and_then(|b| b.nets.get(net))
            .map(|n| n.name.clone())
            .unwrap_or_default()
    }

    /// The value of a member of an object.
    pub fn member(&self, obj: &Obj, name: &str) -> Result<Value, Fault> {
        let unknown = || not_written(format!("the member {name} of a {}", obj.kind()));
        let b = self.board;
        let value = match obj {
            Obj::Board => match name {
                "name" => text(&b.map(|b| b.name.clone()).unwrap_or_default()),
                _ => return Err(unknown()),
            },
            Obj::Schematic => match name {
                "name" => text(&self.schematic.map(|s| s.name.clone()).unwrap_or_default()),
                _ => return Err(unknown()),
            },
            Obj::Element(id) => {
                let c = b
                    .and_then(|b| b.components.get(id))
                    .ok_or_else(|| (RUN_TYPE, "an element that is not in the board".to_owned()))?;
                match name {
                    "name" => text(&c.refdes),
                    "value" => text(&c.value),
                    "package" => Value::Obj(Obj::Package(c.footprint)),
                    "x" => nm(c.at.x_nm.get()),
                    "y" => nm(c.at.y_nm.get()),
                    "angle" => Value::Real(c.rotation.get() as f64 / 1e6),
                    "mirror" => Value::Int(i64::from(c.side == Side::Bottom)),
                    _ => return Err(unknown()),
                }
            }
            Obj::Package(id) => match name {
                "name" => text(
                    &b.and_then(|b| b.footprints.get(id))
                        .map(|f| f.name.clone())
                        .unwrap_or_default(),
                ),
                _ => return Err(unknown()),
            },
            Obj::Pad {
                element,
                footprint,
                pad,
            } => {
                let p = b
                    .and_then(|b| b.footprints.get(footprint))
                    .and_then(|f| f.pads.get(pad))
                    .ok_or_else(|| (RUN_TYPE, "a pad that is not in the board".to_owned()))?;
                let (w, h) = match p.shape {
                    PadShape::Circle { diameter } => (diameter.get(), diameter.get()),
                    PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                        (width.get(), height.get())
                    }
                };
                match name {
                    "name" => text(&p.number),
                    "x" | "y" => {
                        let (px, py) = (p.at.x_nm.get(), p.at.y_nm.get());
                        match element.and_then(|e| b.and_then(|b| b.components.get(&e))) {
                            None => nm(if name == "x" { px } else { py }),
                            Some(c) => {
                                let px = if c.side == Side::Bottom { -px } else { px };
                                let (tx, ty) = turn(px, py, c.rotation.get());
                                if name == "x" {
                                    nm(c.at.x_nm.get() + tx)
                                } else {
                                    nm(c.at.y_nm.get() + ty)
                                }
                            }
                        }
                    }
                    "diameter" => match p.shape {
                        PadShape::Circle { diameter } => nm(diameter.get()),
                        _ => nm(0),
                    },
                    "width" => nm(w),
                    "height" => nm(h),
                    "drill" => nm(p.drill.map_or(0, |d| d.get())),
                    "shape" => text(match p.shape {
                        PadShape::Circle { .. } => "circle",
                        PadShape::Rect { .. } => "rect",
                        PadShape::Oblong { .. } => "oblong",
                    }),
                    "signal" => {
                        let net = element.and_then(|e| {
                            let key = eh3d_phoenix_board::PadRef {
                                component: e,
                                pad: *pad,
                            };
                            b.and_then(|b| {
                                b.pad_nets
                                    .binary_search_by(|n| n.pad_ref().cmp(&key))
                                    .ok()
                                    .map(|i| b.pad_nets[i].net)
                            })
                        });
                        text(&net.map(|n| self.net_name(&n)).unwrap_or_default())
                    }
                    _ => return Err(unknown()),
                }
            }
            Obj::Signal(id) => match name {
                "name" => text(&self.net_name(id)),
                _ => return Err(unknown()),
            },
            Obj::Layer(id) => match name {
                "name" => text(
                    &b.and_then(|b| b.layers.get(id))
                        .map(|l| l.name.clone())
                        .unwrap_or_default(),
                ),
                "number" => nm(self.layer_number(id)),
                _ => return Err(unknown()),
            },
            Obj::Wire { track, segment } => {
                let t = b
                    .and_then(|b| b.tracks.get(track))
                    .ok_or_else(|| (RUN_TYPE, "a wire that is not in the board".to_owned()))?;
                let s: &Segment = t
                    .path
                    .get(*segment)
                    .ok_or_else(|| (RUN_TYPE, "a wire that is not in the board".to_owned()))?;
                let (a, z) = (s.start(), s.end());
                match name {
                    "x1" => nm(a.x_nm.get()),
                    "y1" => nm(a.y_nm.get()),
                    "x2" => nm(z.x_nm.get()),
                    "y2" => nm(z.y_nm.get()),
                    "width" => nm(t.width.get()),
                    "layer" => nm(self.layer_number(&t.layer)),
                    "signal" => text(&self.net_name(&t.net)),
                    "length" => {
                        let d2 = a.distance_squared(z);
                        nm(isqrt(d2))
                    }
                    _ => return Err(unknown()),
                }
            }
            Obj::Via(id) => {
                let v = b
                    .and_then(|b| b.vias.get(id))
                    .ok_or_else(|| (RUN_TYPE, "a via that is not in the board".to_owned()))?;
                match name {
                    "x" => nm(v.at.x_nm.get()),
                    "y" => nm(v.at.y_nm.get()),
                    "drill" => nm(v.drill.get()),
                    "diameter" => nm(v.diameter.get()),
                    "signal" => text(&self.net_name(&v.net)),
                    _ => return Err(unknown()),
                }
            }
            Obj::Polygon(id) => {
                let z = b
                    .and_then(|b| b.zones.get(id))
                    .ok_or_else(|| (RUN_TYPE, "a polygon that is not in the board".to_owned()))?;
                match name {
                    "layer" => nm(self.layer_number(&z.layer)),
                    "signal" => text(&self.net_name(&z.net)),
                    "area" => {
                        let mut twice: i128 = 0;
                        let n = z.outline.len();
                        for i in 0..n {
                            let (p, q) = (z.outline[i], z.outline[(i + 1) % n]);
                            twice += i128::from(p.x_nm.get()) * i128::from(q.y_nm.get())
                                - i128::from(q.x_nm.get()) * i128::from(p.y_nm.get());
                        }
                        Value::Real(twice.abs() as f64 / 2.0)
                    }
                    _ => return Err(unknown()),
                }
            }
            Obj::Vertex { zone, index } => {
                let p = b
                    .and_then(|b| b.zones.get(zone))
                    .and_then(|z| z.outline.get(*index))
                    .ok_or_else(|| (RUN_TYPE, "a vertex that is not in the board".to_owned()))?;
                match name {
                    "x" => nm(p.x_nm.get()),
                    "y" => nm(p.y_nm.get()),
                    _ => return Err(unknown()),
                }
            }
            Obj::Part(id) => {
                let p = self
                    .schematic
                    .and_then(|s| s.parts.get(id))
                    .ok_or_else(|| (RUN_TYPE, "a part that is not in the schematic".to_owned()))?;
                match name {
                    "name" => text(&p.refdes),
                    "value" => text(&p.value),
                    _ => return Err(unknown()),
                }
            }
            Obj::SchNet(id) => match name {
                "name" => text(
                    &self
                        .schematic
                        .and_then(|s| s.nets.get(id))
                        .map(|n| n.name.clone())
                        .unwrap_or_default(),
                ),
                _ => return Err(unknown()),
            },
            Obj::Sheet(id) => {
                let s = self.schematic;
                match name {
                    "name" => text(
                        &s.and_then(|s| s.sheets.get(id))
                            .map(|x| x.name.clone())
                            .unwrap_or_default(),
                    ),
                    "number" => nm(s
                        .and_then(|s| s.sheets.keys().position(|k| k == id))
                        .map_or(0, |i| i as i64 + 1)),
                    "width" => nm(s
                        .and_then(|s| s.sheets.get(id))
                        .map_or(0, |x| x.width.get())),
                    "height" => nm(s
                        .and_then(|s| s.sheets.get(id))
                        .map_or(0, |x| x.height.get())),
                    _ => return Err(unknown()),
                }
            }
        };
        Ok(value)
    }
}

fn wires_of(b: &Board, net: Option<Id>) -> Vec<Obj> {
    let mut out = Vec::new();
    for (id, t) in &b.tracks {
        if net.is_some_and(|n| n != t.net) {
            continue;
        }
        for segment in 0..t.path.len() {
            out.push(Obj::Wire {
                track: *id,
                segment,
            });
        }
    }
    out
}

/// The integer square root of a non-negative number, rounded down.
fn isqrt(n: i128) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as i128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    i64::try_from(x).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_by_a_quarter_is_exact() {
        assert_eq!(turn(10, 0, 0), (10, 0));
        assert_eq!(turn(10, 0, 90_000_000), (0, 10));
        assert_eq!(turn(10, 0, 180_000_000), (-10, 0));
        assert_eq!(turn(10, 0, -90_000_000), (0, -10));
        assert_eq!(turn(10, 5, 270_000_000), (5, -10));
        assert_eq!(turn(1000, 0, 45_000_000), (707, 707));
    }

    #[test]
    fn the_integer_square_root_is_exact() {
        for n in [0i128, 1, 2, 3, 4, 15, 16, 17, 1_000_000, 999_999_999_999] {
            let r = i128::from(isqrt(n));
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "{n}");
        }
    }
}
