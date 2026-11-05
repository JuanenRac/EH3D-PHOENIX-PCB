// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene: a board or a schematic turned into drawing primitives in millimetres, grouped in
//! layers with a colour, and linked to the items of the document and to their nets. Nothing here
//! knows egui or a window.

use std::collections::{BTreeMap, HashMap};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{
    Board, Id, LayerKind, PadRef, PadShape, Segment, Side, SilkItem, StackFunction,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_schematic::{Id as SchId, NetPin, Schematic};

use crate::codes::{ViewError, TOO_MANY_PRIMITIVES};
use crate::prim::{arc_from_ends, Bounds, Prim, Shape, P};
use crate::settings::ViewerSettings;

/// What a layer of the scene is for. It decides the colour and the order of drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Documentation layers.
    Documentation,
    /// The bottom copper layer.
    BottomCopper,
    /// An inner copper layer; the number counts from the first inner layer.
    InnerCopper(u8),
    /// The top copper layer.
    TopCopper,
    /// Solder mask openings.
    Mask,
    /// Solder paste openings.
    Paste,
    /// Silkscreen of the bottom side.
    SilkBottom,
    /// Silkscreen of the top side.
    SilkTop,
    /// The reference designators of the components.
    Designator,
    /// The board outline.
    Outline,
    /// The drill holes.
    Drill,
    /// The frames of the sheets of a schematic.
    Sheet,
    /// The wires of a schematic.
    Wire,
    /// The bodies of the symbols of a schematic.
    Symbol,
    /// The pins of a schematic.
    Pin,
    /// The text of a schematic.
    Label,
}

impl Role {
    /// The place in the drawing order; a larger number is drawn later, so above.
    pub fn rank(self) -> u32 {
        match self {
            Role::Documentation => 0,
            Role::BottomCopper => 3,
            Role::InnerCopper(k) => 4 + u32::from(k),
            Role::TopCopper => 300,
            Role::Mask => 310,
            Role::Paste => 311,
            Role::SilkBottom => 320,
            Role::SilkTop => 321,
            Role::Designator => 322,
            Role::Outline => 330,
            Role::Drill => 340,
            Role::Sheet => 0,
            Role::Wire => 1,
            Role::Symbol => 2,
            Role::Pin => 3,
            Role::Label => 4,
        }
    }

    /// The name of its colour in the settings.
    pub fn colour_key(self) -> &'static str {
        match self {
            Role::Documentation => "documentation",
            Role::BottomCopper => "bottom_copper",
            Role::InnerCopper(k) => [
                "inner_copper_1",
                "inner_copper_2",
                "inner_copper_3",
                "inner_copper_4",
            ][usize::from(k) % 4],
            Role::TopCopper => "top_copper",
            Role::Mask => "mask",
            Role::Paste => "paste",
            Role::SilkBottom => "silk_bottom",
            Role::SilkTop => "silk_top",
            Role::Designator => "designator",
            Role::Outline => "outline",
            Role::Drill => "drill",
            Role::Sheet => "sheet",
            Role::Wire => "wire",
            Role::Symbol => "symbol",
            Role::Pin => "pin",
            Role::Label => "label",
        }
    }

    /// True for a layer that the viewer adds and does not take from the document, so that its
    /// name is shown in the language of the window.
    pub fn is_synthetic(self) -> bool {
        matches!(
            self,
            Role::Designator
                | Role::Outline
                | Role::Drill
                | Role::Sheet
                | Role::Wire
                | Role::Symbol
                | Role::Pin
                | Role::Label
        )
    }
}

/// A layer of the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLayer {
    /// The name: the one of the document, or an English one for a layer that the viewer adds.
    pub name: String,
    /// What it is for.
    pub role: Role,
    /// Red, green, blue and alpha.
    pub colour: [u8; 4],
    /// Whether it is drawn and can be picked.
    pub visible: bool,
}

/// What an item of the document is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A pad of a component.
    Pad,
    /// A track.
    Track,
    /// A via.
    Via,
    /// A copper zone.
    Zone,
    /// A text or a line of the silkscreen.
    Silk,
    /// The board outline.
    Outline,
    /// The reference designator of a component.
    Designator,
    /// A placed part of a schematic.
    Part,
    /// A wire of a schematic.
    Wire,
    /// A pin of a part of a schematic.
    Pin,
    /// The name of a net on a schematic.
    NetLabel,
    /// The frame of a sheet.
    Sheet,
}

/// An item of the document, which may have several primitives.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// What it is.
    pub kind: ItemKind,
    /// A short description (`R1 pad 1`).
    pub label: String,
    /// The index of its net, when it has one.
    pub net: Option<u32>,
}

/// Whether a scene is a board or a schematic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneKind {
    /// A board.
    Board,
    /// A schematic.
    Schematic,
}

/// The drawing of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// A board or a schematic.
    pub kind: SceneKind,
    /// The layers, in the order of drawing.
    pub layers: Vec<SceneLayer>,
    /// The names of the nets; a primitive refers to one by its index.
    pub nets: Vec<String>,
    /// The items of the document.
    pub items: Vec<Item>,
    /// The primitives.
    pub prims: Vec<Prim>,
    /// The box that holds every primitive.
    pub bounds: Bounds,
    /// How many things the document holds that could not be drawn (an arc without a centre, a
    /// text that cannot be made, a part without a place).
    pub skipped: usize,
}

/// How a primitive is painted, apart from its shape.
#[derive(Clone, Copy)]
struct Paint {
    width: f64,
    filled: bool,
    layer: u32,
    net: Option<u32>,
    item: u32,
}

struct Builder {
    prims: Vec<Prim>,
    items: Vec<Item>,
    bounds: Bounds,
    cap: usize,
    over: bool,
    skipped: usize,
}

impl Builder {
    fn new(cap: usize) -> Builder {
        Builder {
            prims: Vec::new(),
            items: Vec::new(),
            bounds: Bounds::EMPTY,
            cap,
            over: false,
            skipped: 0,
        }
    }

    fn item(&mut self, kind: ItemKind, label: impl Into<String>, net: Option<u32>) -> u32 {
        self.items.push(Item {
            kind,
            label: label.into(),
            net,
        });
        (self.items.len() - 1) as u32
    }

    fn push(&mut self, shape: Shape, paint: Paint) {
        if self.prims.len() >= self.cap {
            self.over = true;
            return;
        }
        let prim = Prim {
            shape,
            width: paint.width,
            filled: paint.filled,
            layer: paint.layer,
            net: paint.net,
            item: paint.item,
        };
        self.bounds = self.bounds.union(prim.bounds());
        self.prims.push(prim);
    }

    fn segment(&mut self, segment: &Segment, paint: Paint) {
        match segment {
            Segment::Line { start, end } => self.push(
                Shape::Line {
                    a: pt(*start),
                    b: pt(*end),
                },
                paint,
            ),
            Segment::Arc { start, end, sweep } => {
                match arc_from_ends(pt(*start), pt(*end), rad(*sweep)) {
                    Some((centre, radius, first)) => self.push(
                        Shape::Arc {
                            centre,
                            radius,
                            start: first,
                            sweep: rad(*sweep),
                        },
                        paint,
                    ),
                    None => self.skipped += 1,
                }
            }
        }
    }

    fn finish(
        self,
        kind: SceneKind,
        layers: Vec<SceneLayer>,
        nets: Vec<String>,
    ) -> Result<Scene, ViewError> {
        if self.over {
            return Err(ViewError::new(
                TOO_MANY_PRIMITIVES,
                format!("more than {} primitives", self.cap),
            ));
        }
        Ok(Scene {
            kind,
            layers,
            nets,
            items: self.items,
            prims: self.prims,
            bounds: self.bounds,
            skipped: self.skipped,
        })
    }
}

fn mm(nm: Nm) -> f64 {
    nm.get() as f64 / 1_000_000.0
}

fn pt(p: Point) -> P {
    P::new(mm(p.x_nm), mm(p.y_nm))
}

fn rad(u: Udeg) -> f64 {
    (u.get() as f64 / 1_000_000.0).to_radians()
}

fn turned(p: P, cos: f64, sin: f64) -> P {
    P::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos)
}

fn at(centre: P, local: P, cos: f64, sin: f64) -> P {
    let r = turned(local, cos, sin);
    P::new(centre.x + r.x, centre.y + r.y)
}

/// The shape of a pad at `centre`, turned by `angle` radians: the geometry, the width of the
/// stroke and whether it is filled.
fn pad_shape(shape: PadShape, centre: P, angle: f64) -> (Shape, f64, bool) {
    let (sin, cos) = angle.sin_cos();
    match shape {
        PadShape::Circle { diameter } => (
            Shape::Circle {
                centre,
                radius: mm(diameter) / 2.0,
            },
            0.0,
            true,
        ),
        PadShape::Rect { width, height } => {
            let (hw, hh) = (mm(width) / 2.0, mm(height) / 2.0);
            let points = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
                .iter()
                .map(|(x, y)| at(centre, P::new(*x, *y), cos, sin))
                .collect();
            (Shape::Polygon { points }, 0.0, true)
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (mm(width), mm(height));
            let (ends, stroke) = if w >= h {
                (P::new((w - h) / 2.0, 0.0), h)
            } else {
                (P::new(0.0, (h - w) / 2.0), w)
            };
            let a = at(centre, P::new(-ends.x, -ends.y), cos, sin);
            let b = at(centre, ends, cos, sin);
            (Shape::Line { a, b }, stroke, false)
        }
    }
}

fn role_of(board: &Board, id: Id, layer: &eh3d_phoenix_board::Layer) -> Option<Role> {
    let lower = layer.name.to_lowercase();
    let on_bottom = layer.side == Some(Side::Bottom) || lower.contains("bot");
    match layer.kind {
        LayerKind::Copper => Some(match board.stack_position(id) {
            Some((_, StackFunction::Top)) => Role::TopCopper,
            Some((_, StackFunction::Bottom)) => Role::BottomCopper,
            Some((n, StackFunction::Inner)) => {
                Role::InnerCopper(u8::try_from(n.saturating_sub(2)).unwrap_or(u8::MAX))
            }
            None => {
                if layer.side == Some(Side::Top) || lower.contains("top") {
                    Role::TopCopper
                } else if on_bottom {
                    Role::BottomCopper
                } else {
                    Role::InnerCopper(0)
                }
            }
        }),
        LayerKind::Silkscreen => Some(if on_bottom {
            Role::SilkBottom
        } else {
            Role::SilkTop
        }),
        LayerKind::SolderMask => Some(Role::Mask),
        LayerKind::Paste => Some(Role::Paste),
        LayerKind::Documentation => Some(Role::Documentation),
        // the outline of the board is drawn from the outline of the board, on a layer of its own
        LayerKind::Outline => None,
    }
}

fn make_layers(entries: Vec<(String, Role)>, settings: &ViewerSettings) -> Vec<SceneLayer> {
    entries
        .into_iter()
        .map(|(name, role)| SceneLayer {
            name,
            role,
            colour: settings.colour(role.colour_key()),
            visible: true,
        })
        .collect()
}

impl Scene {
    /// The index of the first layer with a role.
    pub fn layer_of_role(&self, role: Role) -> Option<u32> {
        self.layers
            .iter()
            .position(|l| l.role == role)
            .map(|i| i as u32)
    }

    /// Draws a board.
    pub fn from_board(board: &Board, settings: &ViewerSettings) -> Result<Scene, ViewError> {
        // the layers: those of the board and the three the viewer adds, in the order of drawing
        let mut entries: Vec<(u32, String, Role, Option<Id>)> = Vec::new();
        for (id, layer) in &board.layers {
            if let Some(role) = role_of(board, *id, layer) {
                entries.push((role.rank(), layer.name.clone(), role, Some(*id)));
            }
        }
        for (name, role) in [
            ("Designators", Role::Designator),
            ("Outline", Role::Outline),
            ("Drills", Role::Drill),
        ] {
            entries.push((role.rank(), name.to_string(), role, None));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut layer_index: HashMap<Id, u32> = HashMap::new();
        let mut named: Vec<(String, Role)> = Vec::new();
        for (i, (_, name, role, id)) in entries.into_iter().enumerate() {
            if let Some(id) = id {
                layer_index.insert(id, i as u32);
            }
            named.push((name, role));
        }
        let layers = make_layers(named, settings);
        let outline_layer = layers
            .iter()
            .position(|l| l.role == Role::Outline)
            .unwrap_or(0) as u32;
        let drill_layer = layers
            .iter()
            .position(|l| l.role == Role::Drill)
            .unwrap_or(0) as u32;
        let designator_layer = layers
            .iter()
            .position(|l| l.role == Role::Designator)
            .unwrap_or(0) as u32;

        let net_names: Vec<String> = board.nets.values().map(|n| n.name.clone()).collect();
        let net_index: HashMap<Id, u32> = board
            .nets
            .keys()
            .enumerate()
            .map(|(i, id)| (*id, i as u32))
            .collect();
        let net_of = |id: &Id| net_index.get(id).copied();

        let mut b = Builder::new(settings.max_primitives);

        // the outline
        if board.outline.len() >= 2 {
            let item = b.item(ItemKind::Outline, "outline", None);
            b.push(
                Shape::Polygon {
                    points: board.outline.iter().map(|p| pt(*p)).collect(),
                },
                Paint {
                    width: settings.outline_width_mm,
                    filled: false,
                    layer: outline_layer,
                    net: None,
                    item,
                },
            );
        }

        // zones
        for zone in board.zones.values() {
            let (Some(&layer), true) = (layer_index.get(&zone.layer), zone.outline.len() >= 3)
            else {
                b.skipped += 1;
                continue;
            };
            let net = net_of(&zone.net);
            let item = b.item(ItemKind::Zone, "zone", net);
            b.push(
                Shape::Polygon {
                    points: zone.outline.iter().map(|p| pt(*p)).collect(),
                },
                Paint {
                    width: 0.0,
                    filled: true,
                    layer,
                    net,
                    item,
                },
            );
        }

        // tracks
        for track in board.tracks.values() {
            let Some(&layer) = layer_index.get(&track.layer) else {
                b.skipped += 1;
                continue;
            };
            let net = net_of(&track.net);
            let item = b.item(ItemKind::Track, "track", net);
            let paint = Paint {
                width: mm(track.width),
                filled: false,
                layer,
                net,
                item,
            };
            for segment in &track.path {
                b.segment(segment, paint);
            }
        }

        // vias: a disc on every copper layer from the first to the last, and the hole
        for via in board.vias.values() {
            let net = net_of(&via.net);
            let item = b.item(ItemKind::Via, "via", net);
            let ends = (
                board.copper_stack.iter().position(|l| *l == via.from_layer),
                board.copper_stack.iter().position(|l| *l == via.to_layer),
            );
            let spanned: Vec<Id> = match ends {
                (Some(a), Some(z)) => board.copper_stack[a.min(z)..=a.max(z)].to_vec(),
                _ => vec![via.from_layer, via.to_layer],
            };
            let centre = pt(via.at);
            for id in spanned {
                if let Some(&layer) = layer_index.get(&id) {
                    b.push(
                        Shape::Circle {
                            centre,
                            radius: mm(via.diameter) / 2.0,
                        },
                        Paint {
                            width: 0.0,
                            filled: true,
                            layer,
                            net,
                            item,
                        },
                    );
                }
            }
            b.push(
                Shape::Circle {
                    centre,
                    radius: mm(via.drill) / 2.0,
                },
                Paint {
                    width: 0.0,
                    filled: true,
                    layer: drill_layer,
                    net,
                    item,
                },
            );
        }

        // components: the pads, the holes and the reference designator
        for (cid, component) in &board.components {
            let Some(footprint) = board.footprints.get(&component.footprint) else {
                b.skipped += 1;
                continue;
            };
            let sign = if component.side == Side::Bottom {
                -1.0
            } else {
                1.0
            };
            for (pid, pad) in &footprint.pads {
                let pad_ref = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                let Some(centre) = pad_position(board, pad_ref) else {
                    b.skipped += 1;
                    continue;
                };
                let net = board.net_of(pad_ref).and_then(|id| net_of(&id));
                let item = b.item(
                    ItemKind::Pad,
                    format!("{} pad {}", component.refdes, pad.number),
                    net,
                );
                let angle = rad(component.rotation) + sign * rad(pad.rotation);
                let (shape, width, filled) = pad_shape(pad.shape, pt(centre), angle);
                for id in &pad.layers {
                    if let Some(&layer) = layer_index.get(id) {
                        b.push(
                            shape.clone(),
                            Paint {
                                width,
                                filled,
                                layer,
                                net,
                                item,
                            },
                        );
                    }
                }
                if let Some(drill) = pad.drill {
                    b.push(
                        Shape::Circle {
                            centre: pt(centre),
                            radius: mm(drill) / 2.0,
                        },
                        Paint {
                            width: 0.0,
                            filled: true,
                            layer: drill_layer,
                            net,
                            item,
                        },
                    );
                }
            }
            let item = b.item(
                ItemKind::Designator,
                format!("{} {}", component.refdes, component.value),
                None,
            );
            b.push(
                Shape::Text {
                    at: pt(component.at),
                    height: settings.designator_height_mm,
                    text: component.refdes.clone(),
                },
                Paint {
                    width: 0.0,
                    filled: false,
                    layer: designator_layer,
                    net: None,
                    item,
                },
            );
        }

        // the silkscreen
        for silk in board.silkscreen.values() {
            let Some(&layer) = layer_index.get(&silk.layer()) else {
                b.skipped += 1;
                continue;
            };
            let Ok(drawn) = silk.drawn() else {
                b.skipped += 1;
                continue;
            };
            let label = match silk {
                SilkItem::Text { text, .. } => text.clone(),
                SilkItem::Stroke { .. } => "silkscreen".to_string(),
            };
            let item = b.item(ItemKind::Silk, label, None);
            let paint = Paint {
                width: mm(drawn.width),
                filled: false,
                layer,
                net: None,
                item,
            };
            for piece in &drawn.pieces {
                b.segment(piece, paint);
            }
        }

        b.finish(SceneKind::Board, layers, net_names)
    }

    /// Draws a schematic. The model holds no drawing of a symbol, only its pins, so a part is
    /// drawn as the box around its pins; the sheets are laid side by side.
    pub fn from_schematic(
        schematic: &Schematic,
        settings: &ViewerSettings,
    ) -> Result<Scene, ViewError> {
        let layers = make_layers(
            vec![
                ("Sheets".to_string(), Role::Sheet),
                ("Wires".to_string(), Role::Wire),
                ("Symbols".to_string(), Role::Symbol),
                ("Pins".to_string(), Role::Pin),
                ("Labels".to_string(), Role::Label),
            ],
            settings,
        );
        const SHEET: u32 = 0;
        const WIRE: u32 = 1;
        const SYMBOL: u32 = 2;
        const PIN: u32 = 3;
        const LABEL: u32 = 4;

        let net_names: Vec<String> = schematic.nets.values().map(|n| n.name.clone()).collect();
        let mut pin_net: HashMap<NetPin, u32> = HashMap::new();
        for (i, net) in schematic.nets.values().enumerate() {
            for pin in &net.pins {
                pin_net.insert(*pin, i as u32);
            }
        }

        let mut b = Builder::new(settings.max_primitives);

        // the sheets side by side
        let mut offset: HashMap<SchId, f64> = HashMap::new();
        let mut next_x = 0.0;
        for (id, sheet) in &schematic.sheets {
            let (w, h) = (mm(sheet.width), mm(sheet.height));
            offset.insert(*id, next_x);
            let item = b.item(ItemKind::Sheet, sheet.name.clone(), None);
            b.push(
                Shape::Polygon {
                    points: vec![
                        P::new(next_x, 0.0),
                        P::new(next_x + w, 0.0),
                        P::new(next_x + w, h),
                        P::new(next_x, h),
                    ],
                },
                Paint {
                    width: settings.outline_width_mm,
                    filled: false,
                    layer: SHEET,
                    net: None,
                    item,
                },
            );
            b.push(
                Shape::Text {
                    at: P::new(next_x, h + settings.label_height_mm * 0.5),
                    height: settings.label_height_mm,
                    text: sheet.name.clone(),
                },
                Paint {
                    width: 0.0,
                    filled: false,
                    layer: SHEET,
                    net: None,
                    item,
                },
            );
            next_x += w + settings.sheet_gap_mm;
        }

        // the pins on their sheets
        let mut by_part: BTreeMap<SchId, Vec<(NetPin, P)>> = BTreeMap::new();
        let mut pin_world: HashMap<NetPin, P> = HashMap::new();
        let mut nodes = Nodes::default();
        let mut pin_nodes: Vec<(usize, u32)> = Vec::new();
        for (pin, sheet, point) in schematic.pin_positions() {
            let Some(&ox) = offset.get(&sheet) else {
                continue;
            };
            let p = P::new(ox + mm(point.x_nm), mm(point.y_nm));
            by_part.entry(pin.part).or_default().push((pin, p));
            pin_world.insert(pin, p);
            if let Some(&net) = pin_net.get(&pin) {
                let node = nodes.node(sheet, point.x_nm.get(), point.y_nm.get());
                pin_nodes.push((node, net));
            }
        }

        // the parts: a box around the pins, the pins and the labels
        for (part_id, part) in &schematic.parts {
            let Some(pins) = by_part.get(part_id) else {
                b.skipped += 1;
                continue;
            };
            let mut area = Bounds::EMPTY;
            for (_, p) in pins {
                area.include(*p);
            }
            let area = area.expanded(settings.symbol_padding_mm);
            let item = b.item(
                ItemKind::Part,
                format!("{} {}", part.refdes, part.value),
                None,
            );
            b.push(
                Shape::Polygon {
                    points: vec![
                        area.min,
                        P::new(area.max.x, area.min.y),
                        area.max,
                        P::new(area.min.x, area.max.y),
                    ],
                },
                Paint {
                    width: settings.symbol_line_width_mm,
                    filled: false,
                    layer: SYMBOL,
                    net: None,
                    item,
                },
            );
            b.push(
                Shape::Text {
                    at: P::new(area.min.x, area.max.y + settings.label_height_mm * 0.3),
                    height: settings.label_height_mm,
                    text: format!("{} {}", part.refdes, part.value),
                },
                Paint {
                    width: 0.0,
                    filled: false,
                    layer: LABEL,
                    net: None,
                    item,
                },
            );
            for (pin, p) in pins {
                let number = schematic.pin_number(*pin).unwrap_or("?");
                let net = pin_net.get(pin).copied();
                let pin_item = b.item(ItemKind::Pin, format!("{}.{}", part.refdes, number), net);
                b.push(
                    Shape::Circle {
                        centre: *p,
                        radius: settings.pin_radius_mm,
                    },
                    Paint {
                        width: 0.0,
                        filled: true,
                        layer: PIN,
                        net,
                        item: pin_item,
                    },
                );
                b.push(
                    Shape::Text {
                        at: P::new(
                            p.x + settings.pin_radius_mm * 1.5,
                            p.y + settings.pin_radius_mm * 1.5,
                        ),
                        height: settings.pin_number_height_mm,
                        text: number.to_string(),
                    },
                    Paint {
                        width: 0.0,
                        filled: false,
                        layer: LABEL,
                        net,
                        item: pin_item,
                    },
                );
            }
        }

        // the wires: points that are joined, and the net that they reach
        let mut wire_nodes: Vec<(SchId, usize)> = Vec::new();
        for (wid, wire) in &schematic.wires {
            let mut first: Option<usize> = None;
            let mut previous: Option<usize> = None;
            for point in &wire.path {
                let node = nodes.node(wire.sheet, point.x_nm.get(), point.y_nm.get());
                first.get_or_insert(node);
                if let Some(before) = previous {
                    nodes.join(before, node);
                }
                previous = Some(node);
            }
            if let Some(node) = first {
                wire_nodes.push((*wid, node));
            }
        }
        let mut net_of_root: HashMap<usize, u32> = HashMap::new();
        for (node, net) in pin_nodes {
            let root = nodes.find(node);
            net_of_root.entry(root).or_insert(net);
        }
        for (wid, node) in wire_nodes {
            let Some(wire) = schematic.wires.get(&wid) else {
                continue;
            };
            let Some(&ox) = offset.get(&wire.sheet) else {
                b.skipped += 1;
                continue;
            };
            let root = nodes.find(node);
            let net = net_of_root.get(&root).copied();
            let item = b.item(ItemKind::Wire, "wire", net);
            for pair in wire.path.windows(2) {
                let a = P::new(ox + mm(pair[0].x_nm), mm(pair[0].y_nm));
                let z = P::new(ox + mm(pair[1].x_nm), mm(pair[1].y_nm));
                b.push(
                    Shape::Line { a, b: z },
                    Paint {
                        width: settings.wire_width_mm,
                        filled: false,
                        layer: WIRE,
                        net,
                        item,
                    },
                );
            }
        }

        // the name of each net, next to its first pin that has a place
        for (i, net) in schematic.nets.values().enumerate() {
            let Some(p) = net.pins.iter().find_map(|pin| pin_world.get(pin)) else {
                continue;
            };
            let idx = i as u32;
            let item = b.item(ItemKind::NetLabel, net.name.clone(), Some(idx));
            b.push(
                Shape::Text {
                    at: P::new(
                        p.x + settings.pin_radius_mm * 2.0,
                        p.y - settings.label_height_mm - settings.pin_radius_mm,
                    ),
                    height: settings.label_height_mm,
                    text: net.name.clone(),
                },
                Paint {
                    width: 0.0,
                    filled: false,
                    layer: LABEL,
                    net: Some(idx),
                    item,
                },
            );
        }

        b.finish(SceneKind::Schematic, layers, net_names)
    }
}

/// The points of a schematic that wires and pins share, joined into groups.
#[derive(Default)]
struct Nodes {
    index: HashMap<(SchId, i64, i64), usize>,
    parent: Vec<usize>,
}

impl Nodes {
    fn node(&mut self, sheet: SchId, x: i64, y: i64) -> usize {
        let next = self.parent.len();
        let node = *self.index.entry((sheet, x, y)).or_insert(next);
        if node == next {
            self.parent.push(next);
        }
        node
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[a.max(b)] = a.min(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;

    fn count(scene: &Scene, role: Role) -> usize {
        let layer = scene.layer_of_role(role).unwrap();
        scene.prims.iter().filter(|p| p.layer == layer).count()
    }

    #[test]
    fn a_board_becomes_primitives_on_its_layers() {
        let board = sample::board();
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.kind, SceneKind::Board);
        assert_eq!(scene.prims.len(), 15);
        assert_eq!(scene.items.len(), 9);
        assert_eq!(scene.skipped, 0);
        assert_eq!(count(&scene, Role::TopCopper), 5);
        assert_eq!(count(&scene, Role::BottomCopper), 3);
        assert_eq!(count(&scene, Role::Mask), 2);
        assert_eq!(count(&scene, Role::SilkTop), 1);
        assert_eq!(count(&scene, Role::Drill), 2);
        assert_eq!(count(&scene, Role::Outline), 1);
        assert_eq!(count(&scene, Role::Designator), 1);
        assert_eq!(scene.nets, vec!["GND".to_string(), "VCC".to_string()]);
    }

    #[test]
    fn the_layers_are_in_the_order_of_drawing_and_take_their_colours_from_the_settings() {
        let board = sample::board();
        let settings = ViewerSettings::parse("colour.top_copper = #112233").unwrap();
        let scene = Scene::from_board(&board, &settings).unwrap();
        let ranks: Vec<u32> = scene.layers.iter().map(|l| l.role.rank()).collect();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        assert_eq!(ranks, sorted);
        let top = scene.layer_of_role(Role::TopCopper).unwrap() as usize;
        let bottom = scene.layer_of_role(Role::BottomCopper).unwrap() as usize;
        assert!(bottom < top);
        assert_eq!(scene.layers[top].colour, [0x11, 0x22, 0x33, 0xff]);
        assert_eq!(
            scene.layers[bottom].colour,
            settings.colour("bottom_copper")
        );
        assert!(scene.layers.iter().all(|l| l.visible));
    }

    #[test]
    fn the_nets_of_pads_tracks_vias_and_zones_are_linked() {
        let board = sample::board();
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        let nets_of = |kind: ItemKind| -> Vec<Option<u32>> {
            scene
                .items
                .iter()
                .filter(|i| i.kind == kind)
                .map(|i| i.net)
                .collect()
        };
        assert_eq!(nets_of(ItemKind::Pad), vec![Some(0), Some(1)]);
        assert_eq!(nets_of(ItemKind::Via), vec![Some(0)]);
        assert_eq!(nets_of(ItemKind::Zone), vec![Some(0)]);
        assert_eq!(nets_of(ItemKind::Silk), vec![None]);
        // every primitive carries the net of its item
        for prim in &scene.prims {
            assert_eq!(prim.net, scene.items[prim.item as usize].net);
        }
    }

    #[test]
    fn an_arc_track_is_an_arc_with_the_right_centre() {
        let board = sample::board();
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        let arc = scene
            .prims
            .iter()
            .find_map(|p| match &p.shape {
                Shape::Arc { centre, radius, .. } => Some((*centre, *radius, p.width)),
                _ => None,
            })
            .unwrap();
        // from (20, 10) to (22, 12) with a quarter turn counter-clockwise: the centre is (20, 12)
        assert!(arc.0.dist(P::new(20.0, 12.0)) < 1e-6);
        assert!((arc.1 - 2.0).abs() < 1e-6);
        assert!((arc.2 - 0.2).abs() < 1e-9);
    }

    #[test]
    fn a_pad_of_a_component_on_the_bottom_is_mirrored_and_a_turned_one_is_turned() {
        let mut board = sample::board();
        let cid = *board.components.keys().next().unwrap();
        board.components.get_mut(&cid).unwrap().side = Side::Bottom;
        board.components.get_mut(&cid).unwrap().rotation = Udeg::new(90_000_000).unwrap();
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        // the circular pad was at x = +1.5: mirrored to -1.5 and then turned 90 degrees to (0, -1.5)
        let circle = scene
            .prims
            .iter()
            .find_map(|p| match &p.shape {
                Shape::Circle { centre, radius } if (*radius - 0.8).abs() < 1e-9 => Some(*centre),
                _ => None,
            })
            .unwrap();
        assert!(circle.dist(P::new(10.0, 8.5)) < 1e-6, "{circle:?}");
    }

    #[test]
    fn an_unknown_layer_or_a_missing_footprint_is_counted_and_never_a_panic() {
        let mut board = sample::board();
        let ghost = sample::id(900);
        for track in board.tracks.values_mut() {
            track.layer = ghost;
        }
        let cid = *board.components.keys().next().unwrap();
        board.components.get_mut(&cid).unwrap().footprint = ghost;
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.skipped, 3);
        assert_eq!(count(&scene, Role::Designator), 0);
    }

    #[test]
    fn a_scene_over_the_cap_is_refused_with_its_code() {
        let board = sample::board();
        let settings = ViewerSettings::parse("max_primitives = 5").unwrap();
        let error = Scene::from_board(&board, &settings).unwrap_err();
        assert_eq!(error.code, TOO_MANY_PRIMITIVES);
    }

    #[test]
    fn an_empty_board_has_no_primitives_and_an_empty_box() {
        let scene = Scene::from_board(&Board::default(), &ViewerSettings::default()).unwrap();
        assert!(scene.prims.is_empty());
        assert!(scene.bounds.is_empty());
    }

    #[test]
    fn a_schematic_becomes_boxes_pins_wires_and_labels_on_sheets_side_by_side() {
        let schematic = sample::schematic();
        let scene = Scene::from_schematic(&schematic, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.kind, SceneKind::Schematic);
        assert_eq!(scene.nets, vec!["N1".to_string()]);
        let on = |role: Role| count(&scene, role);
        // two sheets: a frame and a name each
        assert_eq!(on(Role::Sheet), 4);
        // three wires of one piece each
        assert_eq!(on(Role::Wire), 3);
        // two parts, a box each
        assert_eq!(on(Role::Symbol), 2);
        // four pins
        assert_eq!(on(Role::Pin), 4);
        // a name per part, a number per pin and the name of the net
        assert_eq!(on(Role::Label), 2 + 4 + 1);
        assert_eq!(scene.skipped, 0);
        // the second sheet starts after the first one and the gap
        let frames: Vec<f64> = scene
            .prims
            .iter()
            .filter_map(|p| match &p.shape {
                Shape::Polygon { points } if p.layer == 0 => Some(points[0].x),
                _ => None,
            })
            .collect();
        assert_eq!(frames.len(), 2);
        assert!(
            (frames[1] - frames[0] - (100.0 + 20.0)).abs() < 1e-9,
            "{frames:?}"
        );
    }

    #[test]
    fn the_wires_take_the_net_of_the_pins_they_reach_even_through_other_wires() {
        let schematic = sample::schematic();
        let scene = Scene::from_schematic(&schematic, &ViewerSettings::default()).unwrap();
        let wire_nets: Vec<Option<u32>> = scene
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Wire)
            .map(|i| i.net)
            .collect();
        // the first wire joins a pin of net N1 to a bend; the second starts at that bend and
        // reaches another pin of N1; the third touches no pin
        assert_eq!(wire_nets, vec![Some(0), Some(0), None]);
        let pins: Vec<Option<u32>> = scene
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Pin)
            .map(|i| i.net)
            .collect();
        assert_eq!(pins.iter().filter(|n| **n == Some(0)).count(), 2);
    }

    #[test]
    fn a_part_without_a_place_is_counted() {
        let mut schematic = sample::schematic();
        for part in schematic.parts.values_mut() {
            part.placement = None;
        }
        let scene = Scene::from_schematic(&schematic, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.skipped, 2);
        assert_eq!(count(&scene, Role::Symbol), 0);
    }

    #[test]
    fn the_union_of_points_groups_what_is_joined() {
        let mut nodes = Nodes::default();
        let sheet = sample::sch_id(1);
        let a = nodes.node(sheet, 0, 0);
        let b = nodes.node(sheet, 1, 0);
        let c = nodes.node(sheet, 2, 0);
        assert_eq!(nodes.node(sheet, 0, 0), a);
        nodes.join(a, b);
        assert_eq!(nodes.find(a), nodes.find(b));
        assert_ne!(nodes.find(a), nodes.find(c));
        nodes.join(c, b);
        assert_eq!(nodes.find(a), nodes.find(c));
        let other = nodes.node(sample::sch_id(2), 0, 0);
        assert_ne!(nodes.find(a), nodes.find(other));
    }
}
