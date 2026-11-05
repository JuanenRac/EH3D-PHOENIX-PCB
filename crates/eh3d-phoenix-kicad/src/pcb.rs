// SPDX-License-Identifier: GPL-3.0-or-later
//! Boards: a `.kicad_pcb` file read into the board model.
//!
//! What is read: the layer table (and from it the order of the copper layers), the nets, the
//! footprints with their pads and the net of each pad, the tracks (straight pieces and arcs, joined
//! into runs of one net, layer and width), the vias, the zones (outline and clearance), the
//! outline drawn on `Edge.Cuts`, and the lines and texts drawn on the silkscreen layers. All
//! lengths become integer nanometres and the y axis is turned to point up.
//!
//! Not kept: the setup (stack-up, rules, plot settings), the filled copper of zones (it is
//! computed again by the tool), keep-out zones, zones and tracks without a net, graphics on layers
//! other than the outline and the silkscreen, dimensions, groups, images and the 3D models.

use std::collections::BTreeMap;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::{
    Board, Component, Id, LayerKind, Net, PadNet, Segment, Side, SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_hub::QualityReport;
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::arcs::ArcGeom;
use crate::conv::{
    at_args, check_root, decode, length_arg, stroke_width, to_model, udeg_text, xy_args, Ctx, P,
};
use crate::error::KicadError;
use crate::footprint::{build_footprint, net_ref, parse_footprint, FootprintDef, NetRef};
use crate::layers::{meaning, LayerMap};
use crate::settings::KicadSettings;
use crate::sexpr::{parse, Node};

/// A board read into the model.
#[derive(Debug, Clone)]
pub struct ReadPcb {
    /// The board.
    pub board: Board,
    /// The version the file declares.
    pub version: String,
    /// The program that wrote the file.
    pub generator: String,
    /// What was kept, changed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
struct NetTable {
    by_id: BTreeMap<i64, Id>,
    by_name: BTreeMap<String, Id>,
}

impl NetTable {
    fn create(&mut self, board: &mut Board, ctx: &mut Ctx, name: &str) -> Id {
        if let Some(id) = self.by_name.get(name) {
            return *id;
        }
        let id = ctx.id();
        board.nets.insert(
            id,
            Net {
                name: name.to_owned(),
            },
        );
        self.by_name.insert(name.to_owned(), id);
        id
    }

    fn declare(&mut self, board: &mut Board, ctx: &mut Ctx, number: i64, name: &str) {
        if number == 0 || name.is_empty() {
            return;
        }
        let id = self.create(board, ctx, name);
        self.by_id.insert(number, id);
    }

    fn resolve(
        &mut self,
        board: &mut Board,
        ctx: &mut Ctx,
        node: &Node,
        net: &NetRef,
    ) -> Option<Id> {
        if let Some(number) = net.id {
            if let Some(id) = self.by_id.get(&number) {
                return Some(*id);
            }
            if number == 0 {
                return None;
            }
        }
        match &net.name {
            Some(name) if !name.is_empty() => {
                let id = self.create(board, ctx, name);
                if let Some(number) = net.id {
                    self.by_id.insert(number, id);
                }
                Some(id)
            }
            Some(_) => None,
            None => {
                if let Some(number) = net.id {
                    ctx.missing(node, &format!("net {number} is not in the net table"));
                }
                None
            }
        }
    }
}

/// A drawn shape in the orientation of the model.
enum Shape {
    Line(Point, Point),
    Arc(ArcGeom, Point, Point),
    Ring(Vec<Point>),
}

struct Pending {
    net: Id,
    layer: Id,
    width: Nm,
    path: Vec<Segment>,
}

struct Reader {
    board: Board,
    layers: LayerMap,
    nets: NetTable,
    footprints: BTreeMap<String, (Id, Vec<Id>)>,
    pending: Option<Pending>,
    edge: Vec<Shape>,
    pads: usize,
    footprint_extras: usize,
}

fn model_points(points: &[P]) -> Option<Vec<Point>> {
    points.iter().map(|p| to_model(*p)).collect()
}

fn xy_list(node: &Node) -> Vec<P> {
    node.child("pts")
        .map(|pts| pts.children("xy").filter_map(xy_args).collect())
        .unwrap_or_default()
}

fn area2(ring: &[Point]) -> i128 {
    let mut sum: i128 = 0;
    for (i, a) in ring.iter().enumerate() {
        let b = &ring[(i + 1) % ring.len()];
        sum += i128::from(a.x_nm.get()) * i128::from(b.y_nm.get())
            - i128::from(b.x_nm.get()) * i128::from(a.y_nm.get());
    }
    sum.abs()
}

/// The signature of a footprint without its nets and without the angle of the footprint, so that
/// placed copies of one footprint share one footprint of the model.
fn signature(def: &FootprintDef) -> String {
    let mut text = format!("{}|{}|", def.name, def.bottom);
    for pad in &def.pads {
        let relative = (pad.angle_udeg - def.angle_udeg) % 360_000_000;
        text.push_str(&format!(
            "{}/{}/{}/{:?}/{}/{:?}/{:?}/{:?};",
            pad.number, pad.kind, pad.shape, pad.at, relative, pad.size, pad.drill, pad.layers
        ));
    }
    text
}

impl Reader {
    fn flush(&mut self, ctx: &mut Ctx) {
        if let Some(p) = self.pending.take() {
            let id = ctx.id();
            self.board.tracks.insert(
                id,
                Track {
                    net: p.net,
                    layer: p.layer,
                    width: p.width,
                    path: p.path,
                },
            );
        }
    }

    fn push_segment(&mut self, net: Id, layer: Id, width: Nm, segment: Segment, ctx: &mut Ctx) {
        if let Some(p) = &mut self.pending {
            if p.net == net
                && p.layer == layer
                && p.width == width
                && p.path.last().map(Segment::end) == Some(segment.start())
            {
                p.path.push(segment);
                return;
            }
        }
        self.flush(ctx);
        self.pending = Some(Pending {
            net,
            layer,
            width,
            path: vec![segment],
        });
    }

    fn layer_of(&mut self, node: &Node, ctx: &mut Ctx) -> Option<Id> {
        match node.child("layer").and_then(|l| l.arg_text(0)) {
            Some(name) => Some(self.layers.intern(name, ctx)),
            None => {
                ctx.invalid(node, "item", "it names no layer");
                None
            }
        }
    }

    fn track_piece(&mut self, node: &Node, arc: bool, ctx: &mut Ctx) {
        let start = node.child("start").and_then(xy_args);
        let end = node.child("end").and_then(xy_args);
        let width = length_arg(node.child("width")).and_then(|w| Nm::new(w).ok());
        let (Some(start), Some(end), Some(width)) = (start, end, width) else {
            ctx.invalid(
                node,
                "track",
                "start, end or width is missing or not a number",
            );
            return;
        };
        let Some(layer) = self.layer_of(node, ctx) else {
            return;
        };
        let net = match node.child("net").and_then(net_ref) {
            Some(r) => self.nets.resolve(&mut self.board, ctx, node, &r),
            None => None,
        };
        let Some(net) = net else {
            ctx.lose("tracks without a net", 1);
            return;
        };
        let (Some(a), Some(b)) = (to_model(start), to_model(end)) else {
            ctx.invalid(node, "track", "a point is outside the range of the model");
            return;
        };
        let mut segment = Segment::Line { start: a, end: b };
        if arc {
            let geometry = node
                .child("mid")
                .and_then(xy_args)
                .and_then(|mid| ArcGeom::through(start, mid, end));
            match geometry.and_then(|g| g.sweep_udeg()) {
                Some(sweep) if a != b => {
                    segment = Segment::Arc {
                        start: a,
                        end: b,
                        sweep,
                    };
                }
                _ => ctx.simplify(
                    "track arc without a usable circle",
                    "read as a straight piece between its ends",
                ),
            }
        }
        self.push_segment(net, layer, width, segment, ctx);
    }

    fn via(&mut self, node: &Node, ctx: &mut Ctx) {
        let at = node.child("at").and_then(xy_args).and_then(to_model);
        let size = length_arg(node.child("size")).and_then(|v| Nm::new(v).ok());
        let drill = length_arg(node.child("drill")).and_then(|v| Nm::new(v).ok());
        let (Some(at), Some(diameter), Some(drill)) = (at, size, drill) else {
            ctx.invalid(
                node,
                "via",
                "position, size or drill is missing or not a number",
            );
            return;
        };
        let mut ends: Vec<Id> = Vec::new();
        if let Some(list) = node.child("layers") {
            for l in list.args() {
                if let Some(name) = l.text() {
                    ends.extend(self.layers.expand(name, ctx));
                }
            }
        }
        let (Some(from_layer), Some(to_layer)) = (ends.first().copied(), ends.last().copied())
        else {
            ctx.invalid(node, "via", "it names no layers");
            return;
        };
        let net = match node.child("net").and_then(net_ref) {
            Some(r) => self.nets.resolve(&mut self.board, ctx, node, &r),
            None => None,
        };
        let Some(net) = net else {
            ctx.lose("vias without a net", 1);
            return;
        };
        let id = ctx.id();
        self.board.vias.insert(
            id,
            Via {
                net,
                at,
                drill,
                diameter,
                from_layer,
                to_layer,
            },
        );
    }

    fn zone(&mut self, node: &Node, ctx: &mut Ctx) {
        if node.child("keepout").is_some() {
            ctx.lose("keep-out zones", 1);
            return;
        }
        ctx.lose(
            "filled copper of zones (computed again by the tool)",
            node.children("filled_polygon").count(),
        );
        let net = match node.child("net").and_then(net_ref) {
            Some(mut r) => {
                if r.name.is_none() {
                    r.name = node
                        .child("net_name")
                        .and_then(|n| n.arg_text(0))
                        .map(str::to_owned);
                }
                self.nets.resolve(&mut self.board, ctx, node, &r)
            }
            None => None,
        };
        let Some(net) = net else {
            ctx.lose("zones without a net", 1);
            return;
        };
        let outline: Option<Vec<Point>> = node
            .child("polygon")
            .map(xy_list)
            .and_then(|points| model_points(&points));
        let Some(outline) = outline.filter(|o| o.len() >= 3) else {
            ctx.invalid(
                node,
                "zone",
                "it has no outline of three or more usable points",
            );
            return;
        };
        let clearance = length_arg(
            node.child("connect_pads")
                .and_then(|c| c.child("clearance")),
        )
        .or_else(|| length_arg(node.child("clearance")))
        .and_then(|c| Nm::new(c).ok());
        let clearance = match clearance {
            Some(c) => c,
            None => {
                ctx.simplify("zone without a clearance", "the clearance is read as zero");
                Nm::ZERO
            }
        };
        let mut layer_ids: Vec<Id> = Vec::new();
        if let Some(l) = node.child("layer").and_then(|l| l.arg_text(0)) {
            layer_ids.extend(self.layers.expand(l, ctx));
        }
        if let Some(list) = node.child("layers") {
            for l in list.args() {
                if let Some(name) = l.text() {
                    layer_ids.extend(self.layers.expand(name, ctx));
                }
            }
        }
        if layer_ids.is_empty() {
            ctx.invalid(node, "zone", "it names no layer");
            return;
        }
        for layer in layer_ids {
            let id = ctx.id();
            self.board.zones.insert(
                id,
                Zone {
                    net,
                    layer,
                    outline: outline.clone(),
                    clearance,
                },
            );
        }
    }

    fn footprint(&mut self, node: &Node, ctx: &mut Ctx) {
        let Some(def) = parse_footprint(node, ctx) else {
            return;
        };
        let key = signature(&def);
        if !self.footprints.contains_key(&key) {
            let built = build_footprint(&def, &mut self.layers, ctx);
            let id = ctx.id();
            self.pads += built.pad_ids.len();
            self.board.footprints.insert(id, built.footprint);
            self.footprints.insert(key.clone(), (id, built.pad_ids));
        }
        self.footprint_extras += def.extras;
        let Some((footprint, pad_ids)) = self.footprints.get(&key).cloned() else {
            return;
        };
        let Some(at) = to_model(def.at) else {
            ctx.invalid(node, "footprint position", "outside the range of the model");
            return;
        };
        let component = ctx.id();
        self.board.components.insert(
            component,
            Component {
                refdes: def.reference.clone(),
                value: def.value.clone(),
                symbol: None,
                footprint,
                at,
                rotation: Udeg::new(def.angle_udeg).unwrap_or(Udeg::ZERO),
                side: if def.bottom { Side::Bottom } else { Side::Top },
            },
        );
        for (pad, pad_id) in def.pads.iter().zip(pad_ids.iter()) {
            let Some(net) = &pad.net else {
                continue;
            };
            if let Some(net_id) = self.nets.resolve(&mut self.board, ctx, node, net) {
                self.board.pad_nets.push(PadNet {
                    component,
                    pad: *pad_id,
                    net: net_id,
                });
            }
        }
    }

    /// The shape of a `gr_*` item, in the orientation of the model.
    fn shape(&self, node: &Node, head: &str, ctx: &mut Ctx) -> Option<Shape> {
        let steps = ctx.settings.outline_arc_steps_per_quarter;
        let found = match head {
            "gr_line" => {
                let a = node.child("start").and_then(xy_args).and_then(to_model);
                let b = node.child("end").and_then(xy_args).and_then(to_model);
                a.zip(b).map(|(a, b)| Shape::Line(a, b))
            }
            "gr_rect" => {
                let a = node.child("start").and_then(xy_args);
                let b = node.child("end").and_then(xy_args);
                a.zip(b).and_then(|(a, b)| {
                    model_points(&[(a.0, a.1), (b.0, a.1), (b.0, b.1), (a.0, b.1)]).map(Shape::Ring)
                })
            }
            "gr_circle" => {
                let c = node.child("center").and_then(xy_args);
                let e = node.child("end").and_then(xy_args);
                c.zip(e).and_then(|(c, e)| {
                    let arc = ArcGeom::from_centre(c, e, 360_000_000)?;
                    let n = (steps * 4).max(4);
                    let points: Vec<Point> = (0..n)
                        .filter_map(|k| arc.point_at(k as f64 / n as f64))
                        .collect();
                    (points.len() == n).then_some(Shape::Ring(points))
                })
            }
            "gr_poly" => model_points(&xy_list(node))
                .filter(|p| p.len() >= 3)
                .map(Shape::Ring),
            "gr_arc" => {
                let start = node.child("start").and_then(xy_args);
                let end = node.child("end").and_then(xy_args);
                match node.child("mid").and_then(xy_args) {
                    Some(mid) => start.zip(end).and_then(|(s, e)| {
                        let g = ArcGeom::through(s, mid, e)?;
                        Some(Shape::Arc(g, to_model(s)?, to_model(e)?))
                    }),
                    None => {
                        let angle = node
                            .child("angle")
                            .and_then(|a| a.arg_text(0))
                            .and_then(udeg_text);
                        // the older form: the start is the centre, the end the start of the arc
                        start.zip(end).zip(angle).and_then(|((c, s), a)| {
                            let g = ArcGeom::from_centre(c, s, a)?;
                            Some(Shape::Arc(g, g.point_at(0.0)?, g.point_at(1.0)?))
                        })
                    }
                }
            }
            _ => None,
        };
        if found.is_none() {
            ctx.invalid(node, head, "its geometry is missing or not usable");
        }
        found
    }

    fn silk(&mut self, node: &Node, head: &str, layer_name: &str, ctx: &mut Ctx) {
        let layer = self.layers.intern(layer_name, ctx);
        if head == "gr_text" {
            let text = node.arg_text(0).unwrap_or("");
            let at = node.child("at").and_then(at_args);
            let size = node
                .child("effects")
                .and_then(|e| e.child("font"))
                .and_then(|f| f.child("size"))
                .and_then(xy_args);
            let (Some((place, angle)), Some((height, _))) = (at, size) else {
                ctx.invalid(node, "text", "position or size is missing or not a number");
                return;
            };
            let thickness = node
                .child("effects")
                .and_then(|e| e.child("font"))
                .and_then(|f| length_arg(f.child("thickness")))
                .filter(|t| *t > 0)
                .unwrap_or(height / 8);
            let (Some(point), Ok(height), Ok(stroke), Ok(rotation)) = (
                to_model(place),
                Nm::new(height),
                Nm::new(thickness),
                Udeg::new(angle),
            ) else {
                ctx.invalid(node, "text", "a value is outside the range of the model");
                return;
            };
            if height.get() <= 0 || stroke.get() <= 0 {
                ctx.invalid(node, "text", "the size is not above zero");
                return;
            }
            let id = ctx.id();
            self.board.silkscreen.insert(
                id,
                SilkItem::Text {
                    layer,
                    at: point,
                    text: text.to_owned(),
                    height,
                    stroke,
                    rotation,
                    mirrored: layer_name.starts_with("B."),
                },
            );
            return;
        }
        let Some(shape) = self.shape(node, head, ctx) else {
            return;
        };
        let Some(width) = stroke_width(node)
            .and_then(|w| Nm::new(w).ok())
            .filter(|w| w.get() > 0)
        else {
            ctx.invalid(node, head, "its line width is missing or not above zero");
            return;
        };
        let path: Vec<Segment> = match shape {
            Shape::Line(start, end) => vec![Segment::Line { start, end }],
            Shape::Arc(geometry, start, end) => match geometry.sweep_udeg() {
                Some(sweep) => vec![Segment::Arc { start, end, sweep }],
                None => vec![Segment::Line { start, end }],
            },
            Shape::Ring(points) => (0..points.len())
                .map(|i| Segment::Line {
                    start: points[i],
                    end: points[(i + 1) % points.len()],
                })
                .collect(),
        };
        let id = ctx.id();
        self.board
            .silkscreen
            .insert(id, SilkItem::Stroke { layer, width, path });
    }

    fn graphic(&mut self, node: &Node, head: &str, ctx: &mut Ctx) {
        let Some(layer_name) = node.child("layer").and_then(|l| l.arg_text(0)) else {
            ctx.invalid(node, head, "it names no layer");
            return;
        };
        if layer_name == "Edge.Cuts" {
            if self.edge.len() < ctx.settings.max_items {
                if let Some(shape) = self.shape(node, head, ctx) {
                    self.edge.push(shape);
                }
            }
            return;
        }
        if meaning(layer_name).0 == LayerKind::Silkscreen {
            self.silk(node, head, layer_name, ctx);
            return;
        }
        ctx.lose(
            "graphics and texts on other layers (fabrication, courtyard, documentation, copper)",
            1,
        );
    }

    /// The outline from the pieces drawn on `Edge.Cuts`: the largest closed ring, from a
    /// rectangle, a circle, a polygon or lines and arcs that join end to end. What else was drawn
    /// there (cut-outs, pieces that do not close) is counted as dropped.
    fn outline(&mut self, ctx: &mut Ctx) {
        let steps = ctx.settings.outline_arc_steps_per_quarter;
        let pieces = std::mem::take(&mut self.edge);
        let mut rings: Vec<Vec<Point>> = Vec::new();
        let mut segments: Vec<(Point, Point)> = Vec::new();
        for shape in pieces {
            match shape {
                Shape::Ring(points) => rings.push(points),
                Shape::Line(a, b) => segments.push((a, b)),
                Shape::Arc(geometry, a, b) => {
                    let mut previous = a;
                    for p in geometry.inner_points(steps) {
                        segments.push((previous, p));
                        previous = p;
                    }
                    segments.push((previous, b));
                    ctx.transform("arcs of the outline (cut into straight pieces)", 1);
                }
            }
        }
        let total = rings.len() + segments.len();
        let chained = chain(&segments);
        let mut unused = 0usize;
        let mut candidates: Vec<Vec<Point>> = rings;
        match chained {
            Some((ring, used)) => {
                if used < segments.len() {
                    unused += 1;
                }
                candidates.push(ring);
            }
            None => {
                if !segments.is_empty() {
                    unused += 1;
                }
            }
        }
        let mut best: Option<Vec<Point>> = None;
        for ring in candidates {
            let larger = match &best {
                Some(current) => area2(&ring) > area2(current),
                None => true,
            };
            if larger {
                if best.is_some() {
                    unused += 1;
                }
                best = Some(ring);
            } else {
                unused += 1;
            }
        }
        match best {
            Some(ring) => {
                ctx.lose(
                    "outline pieces not used (cut-outs and pieces that do not close)",
                    unused,
                );
                ctx.keep("outline points", ring.len());
                self.board.outline = ring;
            }
            None => {
                if total > 0 {
                    ctx.lose("outline pieces that do not close", total);
                }
            }
        }
    }
}

/// The first closed ring that the segments make when joined end to end, as its points and the
/// number of segments it used.
fn chain(segments: &[(Point, Point)]) -> Option<(Vec<Point>, usize)> {
    if segments.is_empty() {
        return None;
    }
    let key = |p: &Point| (p.x_nm.get(), p.y_nm.get());
    let mut touching: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, (a, b)) in segments.iter().enumerate() {
        touching.entry(key(a)).or_default().push(i);
        touching.entry(key(b)).or_default().push(i);
    }
    let mut used = vec![false; segments.len()];
    used[0] = true;
    let first = segments[0].0;
    let mut ring = vec![first];
    let mut current = segments[0].1;
    let mut count = 1usize;
    loop {
        if current == first {
            return if ring.len() >= 3 {
                Some((ring, count))
            } else {
                None
            };
        }
        ring.push(current);
        let next = touching
            .get(&key(&current))
            .and_then(|list| list.iter().copied().find(|j| !used[*j]));
        let j = next?;
        used[j] = true;
        count += 1;
        let (a, b) = segments[j];
        current = if a == current { b } else { a };
    }
}

/// Reads a `.kicad_pcb` file. Ids for the new objects come from `new_id`.
pub fn read_pcb(
    bytes: &[u8],
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<ReadPcb, Diagnostic> {
    let fail = |e: KicadError| e.diagnostic();
    let (text, encoding) = decode(bytes, settings).map_err(fail)?;
    let root = parse(&text, settings).map_err(fail)?;
    let mut ctx = Ctx::new(settings, new_id);
    if let Some(note) = encoding {
        ctx.push(note);
    }
    let (version, generator) =
        check_root(&root, &["kicad_pcb"], "a board (kicad_pcb)", &mut ctx).map_err(fail)?;
    let mut r = Reader {
        board: Board::default(),
        layers: LayerMap::default(),
        nets: NetTable::default(),
        footprints: BTreeMap::new(),
        pending: None,
        edge: Vec::new(),
        pads: 0,
        footprint_extras: 0,
    };
    if let Some(table) = root.child("layers") {
        for item in table.args() {
            if let Some(name) = item.arg_text(0) {
                r.layers.intern(name, &mut ctx);
            }
        }
    }
    for net in root.children("net") {
        match (
            net.arg_text(0).and_then(|n| n.parse::<i64>().ok()),
            net.arg_text(1),
        ) {
            (Some(number), Some(name)) => r.nets.declare(&mut r.board, &mut ctx, number, name),
            _ => ctx.invalid(net, "net", "it is not a number and a name"),
        }
    }
    r.board.name = root
        .child("title_block")
        .and_then(|t| t.child("title"))
        .and_then(|t| t.arg_text(0))
        .unwrap_or("")
        .to_owned();

    let max = settings.max_items;
    let mut seen = 0usize;
    for child in root.args() {
        let Some(head) = child.head() else {
            continue;
        };
        let counts = matches!(
            head,
            "segment"
                | "arc"
                | "via"
                | "zone"
                | "footprint"
                | "module"
                | "gr_line"
                | "gr_arc"
                | "gr_circle"
                | "gr_rect"
                | "gr_poly"
                | "gr_text"
        );
        if counts {
            seen += 1;
            if seen > max {
                ctx.ignore("item (over max_items)");
                continue;
            }
        }
        if !matches!(head, "segment" | "arc") {
            r.flush(&mut ctx);
        }
        match head {
            "segment" => r.track_piece(child, false, &mut ctx),
            "arc" => r.track_piece(child, true, &mut ctx),
            "via" => r.via(child, &mut ctx),
            "zone" => r.zone(child, &mut ctx),
            "footprint" | "module" => r.footprint(child, &mut ctx),
            "gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly" => {
                r.graphic(child, head, &mut ctx)
            }
            "gr_text" => match child.child("layer").and_then(|l| l.arg_text(0)) {
                Some(layer) if meaning(layer).0 == LayerKind::Silkscreen => {
                    r.silk(child, head, layer, &mut ctx)
                }
                Some(_) => ctx.lose(
                    "graphics and texts on other layers (fabrication, courtyard, documentation, copper)",
                    1,
                ),
                None => ctx.invalid(child, head, "it names no layer"),
            },
            "setup" => ctx.lose("setup (stack-up, rules and plot settings)", 1),
            "version" | "generator" | "generator_version" | "general" | "paper" | "page"
            | "title_block" | "layers" | "net" | "embedded_fonts" | "embedded_files" => {}
            other => ctx.ignore(other),
        }
    }
    r.flush(&mut ctx);
    r.outline(&mut ctx);
    r.board.pad_nets.sort_by_key(|p| p.pad_ref());
    r.board.copper_stack = r.layers.copper_stack();
    r.board.layers = std::mem::take(&mut r.layers.layers);
    ctx.lose(
        "footprint graphics, texts and 3D models",
        r.footprint_extras,
    );
    ctx.keep("layers", r.board.layers.len());
    ctx.keep("nets", r.board.nets.len());
    ctx.keep("footprints", r.board.footprints.len());
    ctx.keep("components", r.board.components.len());
    ctx.keep("pads", r.pads);
    ctx.keep("tracks", r.board.tracks.len());
    ctx.keep("vias", r.board.vias.len());
    ctx.keep("zones", r.board.zones.len());
    ctx.keep("silkscreen items", r.board.silkscreen.len());
    let (diagnostics, quality) = ctx.finish();
    Ok(ReadPcb {
        board: r.board,
        version,
        generator,
        quality,
        diagnostics,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A KiCad 6 board: two resistors (one on the bottom side), a run of track with an arc, a via,
    /// a zone, an outline of four lines, silkscreen and things that are not read.
    pub(crate) const PCB_V6: &str = r#"(kicad_pcb (version 20211014) (generator pcbnew)
  (general (thickness 1.6))
  (paper "A4")
  (title_block (title "Demo board"))
  (layers
    (0 "F.Cu" signal)
    (31 "B.Cu" signal)
    (36 "B.SilkS" user "B.Silkscreen")
    (37 "F.SilkS" user "F.Silkscreen")
    (38 "B.Mask" user "B.Mask")
    (39 "F.Mask" user "F.Mask")
    (44 "Edge.Cuts" user)
    (47 "F.CrtYd" user "F.Courtyard")
  )
  (setup (pad_to_mask_clearance 0))
  (net 0 "")
  (net 1 "GND")
  (net 2 "SIG")
  (footprint "Resistor_SMD:R_0603_1608Metric" (layer "F.Cu") (tstamp 11111111-1111-1111-1111-111111111111) (at 20 10 90)
    (fp_text reference "R1" (at 0 -1.43 90) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_text value "10k" (at 0 1.43 90) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (fp_line (start -0.8 -0.4) (end 0.8 -0.4) (layer "F.Fab") (width 0.1))
    (pad "1" smd roundrect (at -0.825 0 90) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (net 2 "SIG"))
    (pad "2" smd roundrect (at 0.825 0 90) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "GND")))
  (footprint "Resistor_SMD:R_0603_1608Metric" (layer "B.Cu") (tstamp 22222222-2222-2222-2222-222222222222) (at 30 10)
    (fp_text reference "R2" (at 0 -1.43) (layer "B.SilkS") (effects (font (size 1 1) (thickness 0.15)) (justify mirror)))
    (fp_text value "4k7" (at 0 1.43) (layer "B.Fab") (effects (font (size 1 1) (thickness 0.15)) (justify mirror)))
    (pad "1" smd roundrect (at -0.825 0) (size 0.8 0.95) (layers "B.Cu" "B.Paste" "B.Mask") (net 2 "SIG"))
    (pad "2" smd roundrect (at 0.825 0) (size 0.8 0.95) (layers "B.Cu" "B.Paste" "B.Mask") (net 1 "GND")))
  (gr_line (start 0 0) (end 50 0) (layer "Edge.Cuts") (width 0.1) (tstamp a1))
  (gr_line (start 50 0) (end 50 30) (layer "Edge.Cuts") (width 0.1) (tstamp a2))
  (gr_line (start 50 30) (end 0 30) (layer "Edge.Cuts") (width 0.1) (tstamp a3))
  (gr_line (start 0 30) (end 0 0) (layer "Edge.Cuts") (width 0.1) (tstamp a4))
  (segment (start 20 10) (end 30 10) (width 0.25) (layer "F.Cu") (net 2) (tstamp b1))
  (segment (start 30 10) (end 30 20) (width 0.25) (layer "F.Cu") (net 2) (tstamp b2))
  (arc (start 30 20) (mid 33.535534 21.464466) (end 35 25) (width 0.25) (layer "F.Cu") (net 2) (tstamp b3))
  (segment (start 5 5) (end 6 5) (width 0.25) (layer "F.Cu") (net 0) (tstamp b4))
  (via (at 35 25) (size 0.8) (drill 0.4) (layers "F.Cu" "B.Cu") (net 2) (tstamp c1))
  (zone (net 1) (net_name "GND") (layer "B.Cu") (tstamp d1) (hatch edge 0.508)
    (connect_pads (clearance 0.508)) (min_thickness 0.254) (filled_areas_thickness no)
    (fill yes (thermal_gap 0.508) (thermal_bridge_width 0.508))
    (polygon (pts (xy 1 1) (xy 49 1) (xy 49 29) (xy 1 29)))
    (filled_polygon (layer "B.Cu") (pts (xy 1 1) (xy 49 1) (xy 49 29))))
  (gr_text "HELLO" (at 10 25 180) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))) (tstamp e1))
  (gr_line (start 5 5) (end 10 5) (layer "F.SilkS") (width 0.12) (tstamp e2))
  (gr_line (start 5 6) (end 10 6) (layer "F.Fab") (width 0.12) (tstamp e3))
  (target plus (at 1 1) (size 1) (width 0.1) (layer "Edge.Cuts") (tstamp f1))
)"#;

    fn read(text: &str) -> ReadPcb {
        let mut ids = crate::sequential_ids().unwrap();
        read_pcb(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap()
    }

    fn count(list: &[eh3d_phoenix_hub::QualityItem], name: &str) -> Option<usize> {
        list.iter().find(|i| i.category == name).map(|i| i.count)
    }

    #[test]
    fn a_kicad_6_board_is_read_into_the_model() {
        let r = read(PCB_V6);
        let b = &r.board;
        assert_eq!(b.name, "Demo board");
        assert_eq!(r.version, "20211014");
        assert_eq!(r.generator, "pcbnew");
        // the eight of the table and the paste layers that the pads name
        assert_eq!(b.layers.len(), 10);
        let stack: Vec<&str> = b
            .copper_stack
            .iter()
            .map(|id| b.layers[id].name.as_str())
            .collect();
        assert_eq!(stack, vec!["F.Cu", "B.Cu"]);
        // the net 0 has no name and is not a net
        let mut nets: Vec<&str> = b.nets.values().map(|n| n.name.as_str()).collect();
        nets.sort_unstable();
        assert_eq!(nets, vec!["GND", "SIG"]);
        // the bottom resistor is another footprint of the model (its pads are on the other side)
        assert_eq!(b.footprints.len(), 2);
        assert_eq!(b.components.len(), 2);
        let r1 = b.components.values().find(|c| c.refdes == "R1").unwrap();
        assert_eq!(r1.value, "10k");
        assert_eq!(r1.side, Side::Top);
        assert_eq!(r1.rotation.get(), 90_000_000);
        assert_eq!(
            (r1.at.x_nm.get(), r1.at.y_nm.get()),
            (20_000_000, -10_000_000)
        );
        let r2 = b.components.values().find(|c| c.refdes == "R2").unwrap();
        assert_eq!(r2.side, Side::Bottom);
        assert_eq!(b.pad_nets.len(), 4);
        // the pads of R1 are not turned relative to R1
        let pad = b.footprints[&r1.footprint].pads.values().next().unwrap();
        assert_eq!(pad.rotation.get(), 0);
        assert!(b
            .pad_nets
            .windows(2)
            .all(|w| w[0].pad_ref() <= w[1].pad_ref()));
    }

    #[test]
    fn tracks_join_into_runs_and_the_arc_has_its_sweep() {
        let b = read(PCB_V6).board;
        assert_eq!(b.tracks.len(), 1);
        let track = b.tracks.values().next().unwrap();
        assert_eq!(track.width.get(), 250_000);
        assert_eq!(track.path.len(), 3);
        assert!(matches!(track.path[0], Segment::Line { .. }));
        match track.path[2] {
            Segment::Arc { sweep, start, end } => {
                assert!((sweep.get() + 90_000_000).abs() < 1000, "{}", sweep.get());
                assert_eq!(
                    (start.x_nm.get(), start.y_nm.get()),
                    (30_000_000, -20_000_000)
                );
                assert_eq!((end.x_nm.get(), end.y_nm.get()), (35_000_000, -25_000_000));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(b.nets[&track.net].name, "SIG");
        assert_eq!(b.vias.len(), 1);
        let via = b.vias.values().next().unwrap();
        assert_eq!((via.drill.get(), via.diameter.get()), (400_000, 800_000));
        assert_ne!(via.from_layer, via.to_layer);
    }

    #[test]
    fn the_zone_the_outline_and_the_silkscreen_are_read() {
        let b = read(PCB_V6).board;
        assert_eq!(b.zones.len(), 1);
        let zone = b.zones.values().next().unwrap();
        assert_eq!(zone.clearance.get(), 508_000);
        assert_eq!(zone.outline.len(), 4);
        assert_eq!(b.layers[&zone.layer].name, "B.Cu");
        assert_eq!(
            b.outline
                .iter()
                .map(|p| (p.x_nm.get(), p.y_nm.get()))
                .collect::<Vec<_>>(),
            vec![
                (0, 0),
                (50_000_000, 0),
                (50_000_000, -30_000_000),
                (0, -30_000_000)
            ]
        );
        assert_eq!(b.silkscreen.len(), 2);
        let text = b
            .silkscreen
            .values()
            .find_map(|s| match s {
                SilkItem::Text {
                    text,
                    rotation,
                    height,
                    stroke,
                    mirrored,
                    ..
                } => Some((
                    text.clone(),
                    rotation.get(),
                    height.get(),
                    stroke.get(),
                    *mirrored,
                )),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            text,
            ("HELLO".to_owned(), 180_000_000, 1_000_000, 150_000, false)
        );
    }

    #[test]
    fn the_board_read_is_consistent_for_the_model() {
        let b = read(PCB_V6).board;
        let problems: Vec<String> = b
            .validate()
            .iter()
            .filter(|d| d.severity == eh3d_diagnostic::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_report_says_what_was_kept_and_what_was_not() {
        let r = read(PCB_V6);
        let q = &r.quality;
        assert_eq!(count(&q.kept, "components"), Some(2));
        assert_eq!(count(&q.kept, "tracks"), Some(1));
        assert_eq!(count(&q.kept, "nets"), Some(2));
        assert_eq!(count(&q.kept, "outline points"), Some(4));
        assert_eq!(count(&q.kept, "pads"), Some(4));
        assert_eq!(count(&q.dropped, "tracks without a net"), Some(1));
        assert_eq!(
            count(
                &q.dropped,
                "filled copper of zones (computed again by the tool)"
            ),
            Some(1)
        );
        assert_eq!(
            count(&q.dropped, "setup (stack-up, rules and plot settings)"),
            Some(1)
        );
        assert_eq!(count(&q.dropped, "`target` elements"), Some(1));
        assert_eq!(
            count(
                &q.dropped,
                "graphics and texts on other layers (fabrication, courtyard, documentation, copper)"
            ),
            Some(1)
        );
        assert_eq!(count(&q.transformed, "pads with a simpler shape"), Some(4));
        // R1 has one line and one text kept as fields: only the line is dropped
        assert_eq!(
            count(&q.dropped, "footprint graphics, texts and 3D models"),
            Some(1)
        );
        assert!(!q.is_lossless());
    }

    #[test]
    fn kicad_7_forms_are_read_the_same_way() {
        let v7 = r#"(kicad_pcb (version 20221018) (generator pcbnew)
  (general (thickness 1.6))
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user) (37 "F.SilkS" user "F.Silkscreen"))
  (net 0 "") (net 1 "GND")
  (gr_rect (start 0 0) (end 40 20) (layer "Edge.Cuts") (stroke (width 0.1) (type default)) (fill none) (tstamp "r"))
  (gr_circle (center 20 10) (end 22 10) (layer "Edge.Cuts") (stroke (width 0.1) (type default)) (fill none) (tstamp "c"))
  (gr_line (start 1 1) (end 2 1) (layer "F.SilkS") (stroke (width 0.12) (type solid)) (tstamp "s"))
  (gr_arc (start 5 5) (mid 6 4) (end 7 5) (layer "F.SilkS") (stroke (width 0.12) (type solid)) (tstamp "a"))
  (segment (start 1 1) (end 5 1) (width 0.2) (layer "F.Cu") (net 1) (tstamp "t"))
  (via blind (at 5 1) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "v"))
  (zone (net 1) (net_name "GND") (layers "F.Cu" "B.Cu") (tstamp "z") (hatch edge 0.5) (connect_pads (clearance 0.3))
    (polygon (pts (xy 0 0) (xy 10 0) (xy 10 10))))
)"#;
        let r = read(v7);
        let b = &r.board;
        // the largest ring is the rectangle; the circle is a cut-out that is not kept
        assert_eq!(b.outline.len(), 4);
        assert_eq!(
            count(
                &r.quality.dropped,
                "outline pieces not used (cut-outs and pieces that do not close)"
            ),
            Some(1)
        );
        assert_eq!(b.silkscreen.len(), 2);
        assert_eq!(b.tracks.len(), 1);
        assert_eq!(b.vias.len(), 1);
        // one zone for each layer it names
        assert_eq!(b.zones.len(), 2);
        assert!(r
            .diagnostics
            .iter()
            .all(|d| d.code.as_deref() != Some("PHOENIX-KICAD-009")));
    }

    #[test]
    fn kicad_8_fields_and_a_net_by_name_are_read() {
        let v8 = r#"(kicad_pcb (version 20240108) (generator "pcbnew") (generator_version "8.0")
  (general (thickness 1.6) (legacy_teardrops no))
  (layers (0 "F.Cu" signal) (2 "B.Cu" signal) (25 "Edge.Cuts" user))
  (net 0 "") (net 1 "VCC")
  (footprint "C_0805" (layer "F.Cu") (uuid "u") (at 10 10)
    (property "Reference" "C1" (at 0 0 0) (layer "F.SilkS") (uuid "p1") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "100n" (at 0 0 0) (layer "F.Fab") (uuid "p2") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" smd rect (at -1 0) (size 1 1.25) (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "VCC") (pinfunction "A") (uuid "pa"))
    (pad "2" smd rect (at 1 0) (size 1 1.25) (layers "F.Cu" "F.Paste" "F.Mask") (net "NEW") (uuid "pb")))
  (segment (start 9 10) (end 5 10) (width 0.2) (layer "F.Cu") (net 1) (uuid "s"))
)"#;
        let r = read(v8);
        let b = &r.board;
        assert_eq!(b.components.values().next().unwrap().refdes, "C1");
        // the net named only by a pad is created
        let mut nets: Vec<&str> = b.nets.values().map(|n| n.name.as_str()).collect();
        nets.sort_unstable();
        assert_eq!(nets, vec!["NEW", "VCC"]);
        assert_eq!(b.pad_nets.len(), 2);
        assert_eq!(b.tracks.len(), 1);
        assert_eq!(b.copper_stack.len(), 2);
    }

    #[test]
    fn references_to_things_the_file_does_not_have_are_reported() {
        let text = r#"(kicad_pcb (version 20211014) (generator pcbnew)
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal))
  (net 0 "") (net 1 "A")
  (segment (start 0 0) (end 1 0) (width 0.2) (layer "F.Cu") (net 9))
  (via (at 1 1) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1))
  (via (at 2 2) (size 0.6) (layers "F.Cu" "B.Cu") (net 1))
  (zone (net 1) (layer "F.Cu") (polygon (pts (xy 0 0) (xy 1 0))))
  (segment (start 0 0) (end 1 0) (layer "F.Cu") (net 1))
  (gr_line (start 0 0) (end 1) (layer "Edge.Cuts") (width 0.1))
  (footprint "X" (layer "F.Cu") (at 1 1)
    (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (net 7 "")))
)"#;
        let r = read(text);
        let codes: Vec<(&str, u32)> = r
            .diagnostics
            .iter()
            .filter_map(|d| d.code.as_deref().map(|c| (c, d.line)))
            .collect();
        // the net 9 of the track is not in the table
        assert!(codes.contains(&("PHOENIX-KICAD-008", 4)));
        // a via without a drill, a zone without an outline, a track without a width, a line
        // without an end
        for line in [6u32, 7, 8, 9] {
            assert!(
                codes.contains(&("PHOENIX-KICAD-005", line)),
                "{line}: {codes:?}"
            );
        }
        assert_eq!(r.board.vias.len(), 1);
        assert!(r.board.zones.is_empty());
        assert!(r.board.tracks.is_empty());
        assert!(r.board.pad_nets.is_empty());
    }

    #[test]
    fn damaged_boards_are_refused_and_no_cut_panics() {
        let mut ids = crate::sequential_ids().unwrap();
        let s = KicadSettings::default();
        let e = read_pcb(PCB_V6[..PCB_V6.len() / 2].as_bytes(), &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-004"));
        let e = read_pcb(b"(kicad_sch (version 20211123))", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-003"));
        let e = read_pcb(b"", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-001"));
        let e = read_pcb(&[0xff, 0xfe, 0x00], &s, &mut ids).unwrap_err();
        assert!(e.code.is_some());
        for cut in (0..PCB_V6.len()).step_by(101) {
            if PCB_V6.is_char_boundary(cut) {
                let _ = read_pcb(PCB_V6[..cut].as_bytes(), &s, &mut ids);
            }
        }
        let tight = KicadSettings {
            max_items: 3,
            ..KicadSettings::default()
        };
        let r = read_pcb(PCB_V6.as_bytes(), &tight, &mut ids).unwrap();
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.message.contains("over max_items")));
    }

    #[test]
    fn an_open_outline_is_not_kept_and_is_reported() {
        let text = r#"(kicad_pcb (version 20211014) (generator pcbnew)
  (layers (0 "F.Cu" signal) (44 "Edge.Cuts" user))
  (gr_line (start 0 0) (end 10 0) (layer "Edge.Cuts") (width 0.1))
  (gr_line (start 10 0) (end 10 10) (layer "Edge.Cuts") (width 0.1))
)"#;
        let r = read(text);
        assert!(r.board.outline.is_empty());
        assert_eq!(
            count(&r.quality.dropped, "outline pieces that do not close"),
            Some(2)
        );
    }

    #[test]
    fn an_outline_drawn_with_arcs_is_cut_into_straight_pieces() {
        // a half disc: a diameter and a half circle through the top
        let text = r#"(kicad_pcb (version 20221018) (generator pcbnew)
  (layers (0 "F.Cu" signal) (44 "Edge.Cuts" user))
  (gr_line (start -10 0) (end 10 0) (layer "Edge.Cuts") (width 0.1))
  (gr_arc (start 10 0) (mid 0 -10) (end -10 0) (layer "Edge.Cuts") (width 0.1))
)"#;
        let r = read(text);
        // 16 pieces for the half circle and the diameter
        assert_eq!(r.board.outline.len(), 17);
        assert_eq!(
            count(
                &r.quality.transformed,
                "arcs of the outline (cut into straight pieces)"
            ),
            Some(1)
        );
    }
}
