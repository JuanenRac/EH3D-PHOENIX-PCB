// SPDX-License-Identifier: GPL-3.0-or-later
//! The design as a program of the reference tool sees it, **read only**: the board and the
//! schematic of the model, the objects a walk goes over (elements, signals, layers, wires, vias,
//! polygons, packages, pads, parts, nets, sheets) and the members each object has. Nothing here
//! writes to the model: it holds shared references and the evaluator has no way to change them.
//!
//! A length of the program is a nanometre (the unit of the model), an angle is a real number in
//! degrees. A member the evaluator does not know for an object is reported by the caller as
//! "not written yet" and never answered with a made-up value.

use eh3d_phoenix_board::{Board, Footprint, Id, Pad, PadShape, Segment, Side, SilkItem};
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
    /// The library of the design: its packages. It is made from the footprints of the board.
    Library,
    /// A part as placed on a sheet of the schematic.
    Instance(Id),
    /// A text of the silkscreen.
    Text(Id),
    /// A hole that is not plated: an unplated pad of a footprint, placed (with the component) or
    /// not.
    Hole {
        /// The component, when the hole is a placed one.
        element: Option<Id>,
        /// The footprint.
        footprint: Id,
        /// The pad.
        pad: Id,
    },
    /// The device of a part of the schematic: its symbol and its package together (the model
    /// keeps one device for each part).
    Device(Id),
    /// The one gate of the device of a part.
    Gate(Id),
    /// A symbol of the schematic, seen through a part when it was reached through one.
    Symbol {
        /// The symbol.
        symbol: Id,
        /// The part it was reached through.
        part: Option<Id>,
    },
    /// A pin of a symbol.
    SymPin {
        /// The symbol.
        symbol: Id,
        /// The pin.
        pin: Id,
        /// The part it was reached through.
        part: Option<Id>,
    },
    /// The segment of a net of the schematic: the model keeps the pins of a net, not its
    /// drawing in pieces, so a net has one segment with all its pins.
    Segment(Id),
    /// A pin of a part that a net joins.
    PinRef {
        /// The part.
        part: Id,
        /// The pin.
        pin: Id,
    },
    /// A pad that a signal of the board joins.
    ContactRef {
        /// The component.
        element: Id,
        /// The footprint of the component.
        footprint: Id,
        /// The pad.
        pad: Id,
    },
    /// The grid of the board or the schematic.
    Grid,
    /// A circle of the silkscreen (a stroke of two half turns that close).
    Circle(Id),
    /// A rectangle of the silkscreen (a stroke of four straight pieces along the axes that
    /// close).
    Rectangle(Id),
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
            Obj::Library => "library",
            Obj::Instance(_) => "instance",
            Obj::Text(_) => "text",
            Obj::Hole { .. } => "hole",
            Obj::Device(_) => "device",
            Obj::Gate(_) => "gate",
            Obj::Symbol { .. } => "symbol",
            Obj::SymPin { .. } => "pin",
            Obj::Segment(_) => "segment",
            Obj::PinRef { .. } => "pinref",
            Obj::ContactRef { .. } => "contactref",
            Obj::Grid => "grid",
            Obj::Circle(_) => "circle",
            Obj::Rectangle(_) => "rectangle",
        }
    }
}

/// The names of the walks the evaluator knows (`X.name(V)`).
pub const WALKS: &[&str] = &[
    "elements",
    "signals",
    "layers",
    "wires",
    "vias",
    "polygons",
    "packages",
    "pads",
    "vertices",
    "parts",
    "nets",
    "sheets",
    "contacts",
    "smds",
    "holes",
    "texts",
    "instances",
    "gates",
    "pins",
    "segments",
    "pinrefs",
    "contactrefs",
    "circles",
    "rectangles",
];

/// The names of the members the evaluator reads.
pub const MEMBERS: &[&str] = &[
    "name",
    "value",
    "package",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "angle",
    "mirror",
    "diameter",
    "width",
    "height",
    "drill",
    "shape",
    "signal",
    "number",
    "layer",
    "length",
    "area",
    "xc",
    "yc",
    "radius",
    "pad",
    "smd",
    "part",
    "sheet",
    "size",
    "ratio",
    "mirrored",
    "angle1",
    "angle2",
    "arc",
    "dx",
    "dy",
    "device",
    "gate",
    "schematic",
    "symbol",
    "contact",
    "direction",
    "element",
    "pin",
    "grid",
    "unit",
    "distance",
    "altunit",
    "altdistance",
    "style",
    "dots",
    "multiple",
    "library",
    "visible",
];

/// The model a run reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct World<'a> {
    /// The board, when one was given.
    pub board: Option<&'a Board>,
    /// The schematic, when one was given.
    pub schematic: Option<&'a Schematic>,
    /// The unit of the grid (0 micron, 1 millimetre, 2 mil, 3 inch); with a distance of 0 the
    /// grid is 1 mm.
    pub grid_unit: i64,
    /// The distance of the grid, in nanometres; 0 means the default.
    pub grid_nm: i64,
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
            "library" => Ok(self.board.map(|_| Obj::Library)),
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
            (Obj::Library, "packages") => board
                .map(|b| b.footprints.keys().map(|k| Obj::Package(*k)).collect())
                .unwrap_or_default(),
            (Obj::Package(f), "contacts" | "smds" | "holes") => board
                .and_then(|b| b.footprints.get(f))
                .map(|fp| pads_of(fp, None, *f, walk))
                .unwrap_or_default(),
            (Obj::Element(e), "contacts" | "smds" | "holes") => board
                .and_then(|b| {
                    let c = b.components.get(e)?;
                    let fp = b.footprints.get(&c.footprint)?;
                    Some(pads_of(fp, Some(*e), c.footprint, walk))
                })
                .unwrap_or_default(),
            (Obj::Board, "holes") => board
                .map(|b| {
                    let mut out = Vec::new();
                    for (e, c) in &b.components {
                        if let Some(fp) = b.footprints.get(&c.footprint) {
                            out.extend(pads_of(fp, Some(*e), c.footprint, "holes"));
                        }
                    }
                    out
                })
                .unwrap_or_default(),
            (Obj::Board, "texts") => board
                .map(|b| {
                    b.silkscreen
                        .iter()
                        .filter(|(_, s)| matches!(s, SilkItem::Text { .. }))
                        .map(|(k, _)| Obj::Text(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Schematic, "instances") => sch
                .map(|s| {
                    s.parts
                        .iter()
                        .filter(|(_, p)| p.placement.is_some())
                        .map(|(k, _)| Obj::Instance(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Sheet(sh), "instances") => sch
                .map(|s| {
                    s.parts
                        .iter()
                        .filter(|(_, p)| p.placement.as_ref().is_some_and(|pl| pl.sheet == *sh))
                        .map(|(k, _)| Obj::Instance(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Part(p), "instances") => sch
                .and_then(|s| s.parts.get(p))
                .filter(|part| part.placement.is_some())
                .map(|_| vec![Obj::Instance(*p)])
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
            (Obj::Device(p), "gates") => sch
                .filter(|s| s.parts.contains_key(p))
                .map(|_| vec![Obj::Gate(*p)])
                .unwrap_or_default(),
            (Obj::Symbol { symbol, part }, "pins") => sch
                .and_then(|s| s.symbols.get(symbol))
                .map(|sym| {
                    sym.pins
                        .keys()
                        .map(|pin| Obj::SymPin {
                            symbol: *symbol,
                            pin: *pin,
                            part: *part,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::SchNet(n), "segments") => sch
                .filter(|s| s.nets.contains_key(n))
                .map(|_| vec![Obj::Segment(*n)])
                .unwrap_or_default(),
            (Obj::Segment(n), "pinrefs") => sch
                .and_then(|s| s.nets.get(n))
                .map(|net| {
                    net.pins
                        .iter()
                        .map(|r| Obj::PinRef {
                            part: r.part,
                            pin: r.pin,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Signal(n), "contactrefs") => board
                .map(|b| {
                    b.pad_nets
                        .iter()
                        .filter(|r| r.net == *n)
                        .filter_map(|r| {
                            let c = b.components.get(&r.component)?;
                            Some(Obj::ContactRef {
                                element: r.component,
                                footprint: c.footprint,
                                pad: r.pad,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Board, "circles") => board
                .map(|b| {
                    b.silkscreen
                        .iter()
                        .filter(|(_, s)| circle_of(s).is_some())
                        .map(|(k, _)| Obj::Circle(*k))
                        .collect()
                })
                .unwrap_or_default(),
            (Obj::Board, "rectangles") => board
                .map(|b| {
                    b.silkscreen
                        .iter()
                        .filter(|(_, s)| rectangle_of(s).is_some())
                        .map(|(k, _)| Obj::Rectangle(*k))
                        .collect()
                })
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

    /// The pad of the board that a pin of a part stands for: the pad of the package of the part
    /// with the number of the pin, on the component with the same designator when there is one.
    fn contact_of(&self, part: Option<Id>, number: &str) -> Result<Value, Fault> {
        let missing = || (RUN_TYPE, "a pin without a pad of the package".to_owned());
        let p = part
            .and_then(|id| self.schematic.and_then(|s| s.parts.get(&id)))
            .ok_or_else(missing)?;
        let footprint = p.footprint.ok_or_else(missing)?;
        let pad = self
            .board
            .and_then(|b| b.footprints.get(&footprint))
            .and_then(|f| f.pads.iter().find(|(_, pad)| pad.number == number))
            .map(|(k, _)| *k)
            .ok_or_else(missing)?;
        let element = self.board.and_then(|b| {
            b.components
                .iter()
                .find(|(_, c)| c.refdes == p.refdes && c.footprint == footprint)
                .map(|(k, _)| *k)
        });
        Ok(Value::Obj(Obj::Pad {
            element,
            footprint,
            pad,
        }))
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

    /// Where a pad stands, in nanometres: with the component when it is a placed one.
    fn pad_place(&self, element: Option<Id>, p: &Pad) -> (i64, i64) {
        let (px, py) = (p.at.x_nm.get(), p.at.y_nm.get());
        match element.and_then(|e| self.board.and_then(|b| b.components.get(&e))) {
            None => (px, py),
            Some(c) => {
                let px = if c.side == Side::Bottom { -px } else { px };
                let (tx, ty) = turn(px, py, c.rotation.get());
                (c.at.x_nm.get() + tx, c.at.y_nm.get() + ty)
            }
        }
    }

    /// The value of a member of an object.
    pub fn member(&self, obj: &Obj, name: &str) -> Result<Value, Fault> {
        let unknown = || not_written(format!("the member {name} of a {}", obj.kind()));
        let b = self.board;
        let value = match obj {
            Obj::Board => match name {
                "name" => text(&b.map(|b| b.name.clone()).unwrap_or_default()),
                "grid" => Value::Obj(Obj::Grid),
                _ => return Err(unknown()),
            },
            Obj::Schematic => match name {
                "name" => text(&self.schematic.map(|s| s.name.clone()).unwrap_or_default()),
                "grid" => Value::Obj(Obj::Grid),
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
            Obj::Library => match name {
                "name" => text(&b.map(|b| b.name.clone()).unwrap_or_default()),
                _ => return Err(unknown()),
            },
            Obj::Text(id) => {
                let Some(SilkItem::Text {
                    layer,
                    at,
                    text: value,
                    height,
                    stroke,
                    rotation,
                    mirrored,
                }) = b.and_then(|b| b.silkscreen.get(id))
                else {
                    return Err((RUN_TYPE, "a text that is not in the board".to_owned()));
                };
                match name {
                    "value" | "name" => text(value),
                    "x" => nm(at.x_nm.get()),
                    "y" => nm(at.y_nm.get()),
                    "size" | "height" => nm(height.get()),
                    "angle" => Value::Real(rotation.get() as f64 / 1e6),
                    "layer" => nm(self.layer_number(layer)),
                    "ratio" => nm(if height.get() > 0 {
                        stroke.get() * 100 / height.get()
                    } else {
                        0
                    }),
                    "mirrored" | "mirror" => nm(i64::from(*mirrored)),
                    _ => return Err(unknown()),
                }
            }
            Obj::Instance(id) => {
                let part = self
                    .schematic
                    .and_then(|s| s.parts.get(id))
                    .ok_or_else(|| {
                        (
                            RUN_TYPE,
                            "an instance that is not in the schematic".to_owned(),
                        )
                    })?;
                let pl = part
                    .placement
                    .as_ref()
                    .ok_or_else(|| (RUN_TYPE, "an instance that is not placed".to_owned()))?;
                match name {
                    "name" => text(&part.refdes),
                    "value" => text(&part.value),
                    "x" => nm(pl.at.x_nm.get()),
                    "y" => nm(pl.at.y_nm.get()),
                    "angle" => Value::Real(pl.rotation.get() as f64 / 1e6),
                    "part" => Value::Obj(Obj::Part(*id)),
                    "device" => Value::Obj(Obj::Device(*id)),
                    "gate" => Value::Obj(Obj::Gate(*id)),
                    "sheet" => Value::Obj(Obj::Sheet(pl.sheet)),
                    _ => return Err(unknown()),
                }
            }
            Obj::Hole {
                element,
                footprint,
                pad,
            } => {
                let p = b
                    .and_then(|b| b.footprints.get(footprint))
                    .and_then(|f| f.pads.get(pad))
                    .ok_or_else(|| (RUN_TYPE, "a hole that is not in the board".to_owned()))?;
                let drill = p.drill.map_or(0, |d| d.get());
                match name {
                    "x" | "y" => {
                        let at = self.pad_place(*element, p);
                        nm(if name == "x" { at.0 } else { at.1 })
                    }
                    "drill" | "diameter" => nm(drill),
                    "radius" => nm(drill / 2),
                    "name" => text(&p.number),
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
                        let at = self.pad_place(*element, p);
                        nm(if name == "x" { at.0 } else { at.1 })
                    }
                    "pad" | "smd" => Value::Obj(obj.clone()),
                    "element" => match element {
                        Some(e) => Value::Obj(Obj::Element(*e)),
                        None => {
                            return Err((RUN_TYPE, "a pad of a package has no element".to_owned()))
                        }
                    },
                    "diameter" => match p.shape {
                        PadShape::Circle { diameter } => nm(diameter.get()),
                        _ => nm(0),
                    },
                    "width" => nm(w),
                    "height" => nm(h),
                    "drill" => nm(p.drill.map_or(0, |d| d.get())),
                    // the numbers of PAD_SHAPE_ROUND, PAD_SHAPE_SQUARE and PAD_SHAPE_LONG
                    "shape" => nm(match p.shape {
                        PadShape::Circle { .. } => 1,
                        PadShape::Rect { .. } => 0,
                        PadShape::Oblong { .. } => 3,
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
                    "angle1" | "angle2" => match arc_centre(s) {
                        Some((cx, cy, _)) => {
                            let p = if name == "angle1" { a } else { z };
                            let deg = (p.y_nm.get() as f64 - cy)
                                .atan2(p.x_nm.get() as f64 - cx)
                                .to_degrees();
                            Value::Real(deg.rem_euclid(360.0))
                        }
                        None => Value::Real(0.0),
                    },
                    "xc" | "yc" | "radius" => {
                        let c = arc_centre(s);
                        match (name, c) {
                            ("xc", Some(c)) => nm(c.0.round() as i64),
                            ("yc", Some(c)) => nm(c.1.round() as i64),
                            ("radius", Some(c)) => nm(c.2.round() as i64),
                            _ => nm(0),
                        }
                    }
                    "length" => {
                        let d2 = a.distance_squared(z);
                        nm(isqrt(d2))
                    }
                    "dx" => nm(z.x_nm.get() - a.x_nm.get()),
                    "dy" => nm(z.y_nm.get() - a.y_nm.get()),
                    "arc" => Value::Obj(obj.clone()),
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
            Obj::Device(id) => {
                let part = self
                    .schematic
                    .and_then(|s| s.parts.get(id))
                    .ok_or_else(|| {
                        (RUN_TYPE, "a device that is not in the schematic".to_owned())
                    })?;
                match name {
                    "name" => text(
                        &self
                            .schematic
                            .and_then(|s| s.symbols.get(&part.symbol))
                            .map(|s| s.name.clone())
                            .unwrap_or_default(),
                    ),
                    "package" => match part.footprint {
                        Some(f) => Value::Obj(Obj::Package(f)),
                        None => return Err((RUN_TYPE, "a device without a package".to_owned())),
                    },
                    _ => return Err(unknown()),
                }
            }
            Obj::Gate(id) => {
                let part = self
                    .schematic
                    .and_then(|s| s.parts.get(id))
                    .ok_or_else(|| (RUN_TYPE, "a gate that is not in the schematic".to_owned()))?;
                match name {
                    "name" => text("G$1"),
                    "symbol" => Value::Obj(Obj::Symbol {
                        symbol: part.symbol,
                        part: Some(*id),
                    }),
                    "x" => nm(part.placement.as_ref().map_or(0, |p| p.at.x_nm.get())),
                    "y" => nm(part.placement.as_ref().map_or(0, |p| p.at.y_nm.get())),
                    _ => return Err(unknown()),
                }
            }
            Obj::Symbol { symbol, .. } => match name {
                "name" => text(
                    &self
                        .schematic
                        .and_then(|s| s.symbols.get(symbol))
                        .map(|s| s.name.clone())
                        .unwrap_or_default(),
                ),
                _ => return Err(unknown()),
            },
            Obj::SymPin { symbol, pin, part } => {
                let sch = self.schematic;
                let p = sch
                    .and_then(|s| s.symbols.get(symbol))
                    .and_then(|s| s.pins.get(pin))
                    .ok_or_else(|| (RUN_TYPE, "a pin that is not in the schematic".to_owned()))?;
                match name {
                    "name" => text(&p.number),
                    "x" => nm(p.at.map_or(0, |a| a.x_nm.get())),
                    "y" => nm(p.at.map_or(0, |a| a.y_nm.get())),
                    "direction" => nm(sch
                        .and_then(|s| s.pin_kinds.get(pin))
                        .map_or(6, |k| pin_direction(*k))),
                    "contact" => self.contact_of(*part, &p.number)?,
                    _ => return Err(unknown()),
                }
            }
            Obj::Segment(_) => return Err(unknown()),
            Obj::PinRef { part, pin } => {
                let p = self
                    .schematic
                    .and_then(|s| s.parts.get(part))
                    .ok_or_else(|| (RUN_TYPE, "a pin of a part that is not there".to_owned()))?;
                match name {
                    "part" => Value::Obj(Obj::Part(*part)),
                    "pin" => Value::Obj(Obj::SymPin {
                        symbol: p.symbol,
                        pin: *pin,
                        part: Some(*part),
                    }),
                    "instance" if p.placement.is_some() => Value::Obj(Obj::Instance(*part)),
                    _ => return Err(unknown()),
                }
            }
            Obj::ContactRef {
                element,
                footprint,
                pad,
            } => match name {
                "element" => Value::Obj(Obj::Element(*element)),
                "contact" => Value::Obj(Obj::Pad {
                    element: Some(*element),
                    footprint: *footprint,
                    pad: *pad,
                }),
                _ => return Err(unknown()),
            },
            Obj::Grid => {
                let (unit, distance) = if self.grid_nm > 0 {
                    (self.grid_unit, self.grid_nm)
                } else {
                    (1, 1_000_000)
                };
                match name {
                    "unit" | "altunit" => nm(unit),
                    "distance" | "altdistance" => nm(distance),
                    "style" | "dots" => nm(0),
                    "multiple" => nm(1),
                    _ => return Err(unknown()),
                }
            }
            Obj::Circle(id) | Obj::Rectangle(id) => {
                let item = b
                    .and_then(|b| b.silkscreen.get(id))
                    .ok_or_else(|| (RUN_TYPE, "a drawing that is not in the board".to_owned()))?;
                let layer = match item {
                    SilkItem::Stroke { layer, .. } | SilkItem::Text { layer, .. } => *layer,
                };
                if let Obj::Circle(_) = obj {
                    let (cx, cy, r, w) = circle_of(item)
                        .ok_or_else(|| (RUN_TYPE, "a circle that is not one".to_owned()))?;
                    match name {
                        "x" => nm(cx),
                        "y" => nm(cy),
                        "radius" => nm(r),
                        "width" => nm(w),
                        "layer" => nm(self.layer_number(&layer)),
                        _ => return Err(unknown()),
                    }
                } else {
                    let (x1, y1, x2, y2, w) = rectangle_of(item)
                        .ok_or_else(|| (RUN_TYPE, "a rectangle that is not one".to_owned()))?;
                    match name {
                        "x1" => nm(x1),
                        "y1" => nm(y1),
                        "x2" => nm(x2),
                        "y2" => nm(y2),
                        "width" => nm(w),
                        "angle" => Value::Real(0.0),
                        "layer" => nm(self.layer_number(&layer)),
                        _ => return Err(unknown()),
                    }
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
                    "device" => Value::Obj(Obj::Device(*id)),
                    "gate" => Value::Obj(Obj::Gate(*id)),
                    "schematic" => Value::Obj(Obj::Schematic),
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

/// The place and the size of a circle that a stroke draws: two half turns that close on each
/// other. Its centre, radius and width, in nanometres.
fn circle_of(item: &SilkItem) -> Option<(i64, i64, i64, i64)> {
    let SilkItem::Stroke { width, path, .. } = item else {
        return None;
    };
    let [a @ Segment::Arc { sweep: s1, .. }, z @ Segment::Arc { sweep: s2, .. }] = path.as_slice()
    else {
        return None;
    };
    let half = 180_000_000;
    if s1.get().abs() != half || s2.get().abs() != half {
        return None;
    }
    if a.end() != z.start() || z.end() != a.start() {
        return None;
    }
    let (p, q) = (a.start(), a.end());
    let cx = (p.x_nm.get() + q.x_nm.get()) / 2;
    let cy = (p.y_nm.get() + q.y_nm.get()) / 2;
    let r = isqrt(p.distance_squared(q)) / 2;
    Some((cx, cy, r, width.get()))
}

/// The corners of a rectangle that a stroke draws: four straight pieces along the axes that close
/// on each other. Its low corner, its high corner and the width, in nanometres.
fn rectangle_of(item: &SilkItem) -> Option<(i64, i64, i64, i64, i64)> {
    let SilkItem::Stroke { width, path, .. } = item else {
        return None;
    };
    if path.len() != 4 {
        return None;
    }
    for i in 0..4 {
        let (p, q) = (path[i].start(), path[i].end());
        if !matches!(path[i], Segment::Line { .. }) || path[(i + 1) % 4].start() != q {
            return None;
        }
        if (p.x_nm == q.x_nm) == (p.y_nm == q.y_nm) {
            return None;
        }
    }
    let xs: Vec<i64> = path.iter().map(|s| s.start().x_nm.get()).collect();
    let ys: Vec<i64> = path.iter().map(|s| s.start().y_nm.get()).collect();
    let (x1, x2) = (*xs.iter().min()?, *xs.iter().max()?);
    let (y1, y2) = (*ys.iter().min()?, *ys.iter().max()?);
    if x1 == x2 || y1 == y2 {
        return None;
    }
    Some((x1, y1, x2, y2, width.get()))
}

/// The direction of a pin as the language numbers it.
fn pin_direction(kind: eh3d_phoenix_schematic::PinKind) -> i64 {
    use eh3d_phoenix_schematic::PinKind;
    match kind {
        PinKind::NotConnected => 0,
        PinKind::Input => 1,
        PinKind::Output => 2,
        PinKind::Bidirectional => 3,
        PinKind::OpenCollector => 4,
        PinKind::PowerIn => 5,
        PinKind::Passive => 6,
        PinKind::TriState => 7,
        PinKind::PowerOut => 8,
    }
}

/// The pads of a footprint a walk goes over: `contacts` all, `smds` those without a drill, `holes`
/// the drilled ones that are not plated.
fn pads_of(fp: &Footprint, element: Option<Id>, footprint: Id, walk: &str) -> Vec<Obj> {
    fp.pads
        .iter()
        .filter(|(_, p)| match walk {
            "smds" => p.drill.is_none(),
            "holes" => p.is_npth(),
            _ => true,
        })
        .map(|(k, _)| {
            if walk == "holes" {
                Obj::Hole {
                    element,
                    footprint,
                    pad: *k,
                }
            } else {
                Obj::Pad {
                    element,
                    footprint,
                    pad: *k,
                }
            }
        })
        .collect()
}

/// The centre and the radius of the arc of a piece of track, in nanometres, when it is one.
fn arc_centre(s: &Segment) -> Option<(f64, f64, f64)> {
    let Segment::Arc { start, end, sweep } = s else {
        return None;
    };
    let (x1, y1) = (start.x_nm.get() as f64, start.y_nm.get() as f64);
    let (x2, y2) = (end.x_nm.get() as f64, end.y_nm.get() as f64);
    let half = (sweep.get() as f64 / 1e6).to_radians() / 2.0;
    let chord = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    if half.sin().abs() < 1e-12 {
        return None;
    }
    if chord == 0.0 {
        // a full turn: the centre cannot be told from the end points alone
        return None;
    }
    let r = chord / (2.0 * half.sin().abs());
    let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let d = (r * r - chord * chord / 4.0).max(0.0).sqrt();
    // left normal of the chord; the centre is on the left of a counter-clockwise arc that is
    // shorter than a half turn
    let (ux, uy) = ((x2 - x1) / chord, (y2 - y1) / chord);
    let (nx, ny) = (-uy, ux);
    let sign = if (sweep.get() > 0) == (half.abs() < std::f64::consts::FRAC_PI_2) {
        1.0
    } else {
        -1.0
    };
    Some((mx + sign * nx * d, my + sign * ny * d, r))
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
