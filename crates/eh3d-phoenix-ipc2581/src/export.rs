// SPDX-License-Identifier: GPL-3.0-or-later
//! The IPC-2581 file of a board, a subset of revision C.
//!
//! The document is built as a tree of elements (the XML tree of `eh3d-phoenix-eagle`, which
//! escapes every text and attribute) and written with two spaces of indent. The same board and
//! settings always give the same bytes: the order of everything is fixed, the numbers are the
//! integers of the model written as millimetres with up to six decimals, and the date is the
//! setting `origination`, not the clock.
//!
//! **What is written:** the layers (copper with their side from the order of the stack, silkscreen,
//! solder mask and paste as declared layers without features), the outline with its cutouts, the
//! standard shapes and the padstacks (shape, drill, plating and layers), the footprints as
//! packages, the placed parts, the nets with their pins, and for each copper layer the pads, the
//! vias, the tracks (with arcs) and the outlines of the zones, grouped by net, the lines of the
//! dimensions and their texts on a layer of the function DOCUMENT, and a basic bill of materials.
//! **What is left out** is reported (`PHOENIX-IPC2581-006`): the silkscreen items, the arcs of the
//! outline, the copper fill of the zones, the texts of the dimensions when the settings say so,
//! the other documentation layers and the stack-up.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{
    Board, Id, LayerKind, Pad, PadRef, PadShape, Segment, Side, SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_model::Point;
use serde::Serialize;

use crate::codes::{
    Finding, Ipc2581Error, Severity, LEFT_OUT, LIMIT_EXCEEDED, NO_OUTLINE, PAD_NO_LAYER,
};
use crate::settings::Ipc2581Settings;

/// The name of the format of the JSON summary.
pub const SUMMARY_FORMAT: &str = "phoenix-ipc2581/1";

/// What the file holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// The name of the format of this summary.
    pub format: &'static str,
    /// The name of the board.
    pub name: String,
    /// The layers declared.
    pub layers: usize,
    /// The copper layers among them.
    pub copper_layers: usize,
    /// The placed parts.
    pub components: usize,
    /// The footprints written as packages.
    pub packages: usize,
    /// The padstacks.
    pub padstacks: usize,
    /// The pads of the parts written on the copper layers.
    pub pads: usize,
    /// The vias.
    pub vias: usize,
    /// The tracks written.
    pub tracks: usize,
    /// The zones written, as outlines.
    pub zones: usize,
    /// The dimensions written, as lines.
    pub dimensions: usize,
    /// The texts of the dimensions written.
    pub dimension_texts: usize,
    /// The nets.
    pub nets: usize,
    /// The lines of the bill of materials.
    pub bom_lines: usize,
    /// The size of the file in bytes.
    pub bytes: usize,
}

/// The file and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipc2581 {
    /// The XML text of the file.
    pub xml: String,
    /// What it holds.
    pub summary: Summary,
    /// What was left out or could not be written.
    pub findings: Vec<Finding>,
}

// ---------------------------------------------------------------------------------------------
// the small tools

/// A length in nanometres as millimetres with up to six decimals and no trailing zeros.
pub fn mm(nm: i64) -> String {
    decimal(nm)
}

/// An angle in micro-degrees as degrees with up to six decimals and no trailing zeros.
pub fn degrees(udeg: i64) -> String {
    decimal(udeg)
}

fn decimal(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let a = value.unsigned_abs();
    let mut text = format!("{}.{:06}", a / 1_000_000, a % 1_000_000);
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    format!("{sign}{text}")
}

fn el(name: &str) -> Element {
    Element {
        name: name.to_owned(),
        ..Element::default()
    }
}

trait Build {
    fn with(self, key: &str, value: impl Into<String>) -> Self;
    fn push(self, child: Element) -> Self;
}

impl Build for Element {
    fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.attrs.push((key.to_owned(), value.into()));
        self
    }

    fn push(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }
}

fn point_attrs(name: &str, p: Point) -> Element {
    el(name)
        .with("x", mm(p.x_nm.get()))
        .with("y", mm(p.y_nm.get()))
}

fn yes(flag: bool) -> &'static str {
    if flag {
        "true"
    } else {
        "false"
    }
}

/// A closed polygon: the first point, a step to each of the others and a step back to the first.
fn polygon_steps(parent: Element, points: &[Point]) -> Element {
    let mut parent = parent;
    let Some(first) = points.first() else {
        return parent;
    };
    parent = parent.push(point_attrs("PolyBegin", *first));
    for p in &points[1..] {
        parent = parent.push(point_attrs("PolyStepSegment", *p));
    }
    parent.push(point_attrs("PolyStepSegment", *first))
}

/// The centre of an arc from its end points and its signed sweep in micro-degrees, in
/// nanometres as floats; `None` for an arc that has no shape.
fn arc_centre(start: Point, end: Point, sweep_udeg: i64) -> Option<(f64, f64)> {
    let (ax, ay) = (start.x_nm.get() as f64, start.y_nm.get() as f64);
    let (bx, by) = (end.x_nm.get() as f64, end.y_nm.get() as f64);
    let (dx, dy) = (bx - ax, by - ay);
    let chord = (dx * dx + dy * dy).sqrt();
    let theta = (sweep_udeg.unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
    let sin_half = (theta / 2.0).sin();
    if chord == 0.0 || !theta.is_finite() || sin_half.abs() < 1e-9 {
        return None;
    }
    let radius = chord / (2.0 * sin_half);
    let offset = radius * (theta / 2.0).cos();
    let (nx, ny) = (-dy / chord, dx / chord);
    let side = if sweep_udeg >= 0 { 1.0 } else { -1.0 };
    Some((
        (ax + bx) / 2.0 + side * nx * offset,
        (ay + by) / 2.0 + side * ny * offset,
    ))
}

// ---------------------------------------------------------------------------------------------
// shapes and padstacks

/// A standard shape of the dictionary, in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    Circle(i64),
    Rect(i64, i64),
    Oval(i64, i64),
    RoundRect(i64, i64, i64),
}

impl Shape {
    fn of(pad: &Pad) -> Shape {
        match pad.shape {
            PadShape::Circle { diameter } => Shape::Circle(diameter.get()),
            PadShape::Rect { width, height } => match pad.corner_radius_nm() {
                Some(r) => Shape::RoundRect(width.get(), height.get(), r),
                None => Shape::Rect(width.get(), height.get()),
            },
            PadShape::Oblong { width, height } => Shape::Oval(width.get(), height.get()),
        }
    }

    fn id(self) -> String {
        match self {
            Shape::Circle(d) => format!("C_{}", mm(d)),
            Shape::Rect(w, h) => format!("R_{}x{}", mm(w), mm(h)),
            Shape::Oval(w, h) => format!("O_{}x{}", mm(w), mm(h)),
            Shape::RoundRect(w, h, r) => format!("RR_{}x{}_{}", mm(w), mm(h), mm(r)),
        }
    }

    fn element(self) -> Element {
        let shape = match self {
            Shape::Circle(d) => el("Circle").with("diameter", mm(d)),
            Shape::Rect(w, h) => el("RectCenter").with("width", mm(w)).with("height", mm(h)),
            Shape::Oval(w, h) => el("Oval").with("width", mm(w)).with("height", mm(h)),
            Shape::RoundRect(w, h, r) => el("RectRound")
                .with("width", mm(w))
                .with("height", mm(h))
                .with("radius", mm(r))
                .with("upperRight", "true")
                .with("upperLeft", "true")
                .with("lowerRight", "true")
                .with("lowerLeft", "true"),
        };
        el("EntryStandard").with("id", self.id()).push(shape)
    }

    /// The half extent in x and y, in nanometres.
    fn half_box(self) -> (i64, i64) {
        match self {
            Shape::Circle(d) => (d / 2, d / 2),
            Shape::Rect(w, h) | Shape::Oval(w, h) | Shape::RoundRect(w, h, _) => (w / 2, h / 2),
        }
    }
}

/// What makes two padstacks the same.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StackKey {
    shape: Shape,
    drill_nm: Option<i64>,
    plated: bool,
    layers: Vec<String>,
}

/// The parts of one group of the bill of materials: reference, pin count and side.
type Members<'a> = Vec<(&'a str, usize, Side)>;

/// One layer of the file.
struct FileLayer {
    id: Id,
    name: String,
    function: &'static str,
    side: &'static str,
    copper: bool,
}

fn file_layers(board: &Board, with_dimensions: bool) -> Vec<FileLayer> {
    let mut out = Vec::new();
    let coppers: Vec<Id> = if board.copper_stack.is_empty() {
        board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == LayerKind::Copper)
            .map(|(id, _)| *id)
            .collect()
    } else {
        board
            .copper_stack
            .iter()
            .filter(|id| board.layers.contains_key(id))
            .copied()
            .collect()
    };
    for (i, id) in coppers.iter().enumerate() {
        let side = if i == 0 {
            "TOP"
        } else if i + 1 == coppers.len() {
            "BOTTOM"
        } else {
            "INTERNAL"
        };
        out.push(FileLayer {
            id: *id,
            name: board.layers[id].name.clone(),
            function: "CONDUCTOR",
            side,
            copper: true,
        });
    }
    for (id, layer) in &board.layers {
        let function = match layer.kind {
            LayerKind::Silkscreen => "SILKSCREEN",
            LayerKind::SolderMask => "SOLDERMASK",
            LayerKind::Paste => "SOLDERPASTE",
            // a documentation layer is written when a dimension is drawn on it
            LayerKind::Documentation
                if with_dimensions && board.dimensions.values().any(|d| d.layer == *id) =>
            {
                "DOCUMENT"
            }
            _ => continue,
        };
        let side = match layer.side {
            Some(Side::Top) => "TOP",
            Some(Side::Bottom) => "BOTTOM",
            None => "ALL",
        };
        out.push(FileLayer {
            id: *id,
            name: layer.name.clone(),
            function,
            side,
            copper: false,
        });
    }
    out
}

fn pad_name(refdes: &str, number: &str) -> String {
    format!("{refdes}:{number}")
}

fn side_layer(layers: &[FileLayer], side: Side) -> Option<&FileLayer> {
    let mut coppers = layers.iter().filter(|l| l.copper);
    match side {
        Side::Top => coppers.next(),
        Side::Bottom => coppers.next_back(),
    }
}

// ---------------------------------------------------------------------------------------------
// the export

/// The IPC-2581 file of a board.
pub fn export(board: &Board, settings: &Ipc2581Settings) -> Result<Ipc2581, Ipc2581Error> {
    let items = board
        .components
        .len()
        .saturating_add(board.pad_nets.len())
        .saturating_add(board.tracks.len())
        .saturating_add(board.vias.len())
        .saturating_add(board.zones.len());
    if items > settings.max_items {
        return Err(Ipc2581Error::new(
            LIMIT_EXCEEDED,
            format!("{items} objects, the limit is {}", settings.max_items),
        ));
    }
    let layers = file_layers(board, settings.include_dimensions);
    let mut findings = Vec::new();
    let layer_index: BTreeMap<Id, usize> =
        layers.iter().enumerate().map(|(i, l)| (l.id, i)).collect();
    let net_name = |id: Id| board.nets.get(&id).map(|n| n.name.clone());

    // the components in the order of their references, and the pads they carry
    let mut parts: Vec<(Id, &eh3d_phoenix_board::Component)> =
        board.components.iter().map(|(id, c)| (*id, c)).collect();
    parts.sort_by(|a, b| a.1.refdes.cmp(&b.1.refdes).then(a.0.cmp(&b.0)));

    // padstacks: every distinct pad and via
    let mut stacks: BTreeSet<StackKey> = BTreeSet::new();
    let mut shapes: BTreeSet<Shape> = BTreeSet::new();
    let layer_names = |ids: &[Id]| -> Vec<String> {
        let mut found: Vec<usize> = ids
            .iter()
            .filter_map(|i| layer_index.get(i))
            .copied()
            .collect();
        found.sort_unstable();
        found.dedup();
        found
            .into_iter()
            .filter(|i| layers[*i].copper)
            .map(|i| layers[i].name.clone())
            .collect()
    };
    let pad_key = |pad: &Pad| StackKey {
        shape: Shape::of(pad),
        drill_nm: pad.drill.map(|d| d.get()),
        plated: pad.plated,
        layers: if pad.is_npth() {
            Vec::new()
        } else {
            layer_names(&pad.layers)
        },
    };
    for footprint in board.footprints.values() {
        for pad in footprint.pads.values() {
            shapes.insert(Shape::of(pad));
            stacks.insert(pad_key(pad));
        }
    }
    let copper_ids: Vec<Id> = layers.iter().filter(|l| l.copper).map(|l| l.id).collect();
    let via_layers = |via: &Via| -> Vec<Id> {
        let from = copper_ids.iter().position(|i| *i == via.from_layer);
        let to = copper_ids.iter().position(|i| *i == via.to_layer);
        match (from, to) {
            (Some(a), Some(b)) if !board.copper_stack.is_empty() => {
                copper_ids[a.min(b)..=a.max(b)].to_vec()
            }
            (Some(a), Some(b)) if a == b => vec![copper_ids[a]],
            (Some(a), Some(b)) => vec![copper_ids[a], copper_ids[b]],
            (Some(a), None) | (None, Some(a)) => vec![copper_ids[a]],
            (None, None) => Vec::new(),
        }
    };
    let via_key = |via: &Via| StackKey {
        shape: Shape::Circle(via.diameter.get()),
        drill_nm: Some(via.drill.get()),
        plated: true,
        layers: layer_names(&via_layers(via)),
    };
    for via in board.vias.values() {
        shapes.insert(Shape::Circle(via.diameter.get()));
        stacks.insert(via_key(via));
    }
    let stack_ids: BTreeMap<StackKey, String> = stacks
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), format!("PS{}", i + 1)))
        .collect();

    // ---- Content
    let mut content = el("Content")
        .with("roleRef", "Owner")
        .push(el("FunctionMode").with("mode", "USERDEF"))
        .push(el("StepRef").with("name", board.name.clone()));
    for layer in &layers {
        content = content.push(el("LayerRef").with("name", layer.name.clone()));
    }
    if settings.include_bom {
        content = content.push(el("BomRef").with("name", "Bom"));
    }
    let mut dictionary = el("DictionaryStandard").with("units", "MILLIMETER");
    for shape in &shapes {
        dictionary = dictionary.push(shape.element());
    }
    content = content.push(dictionary);

    // ---- the header and the history
    let header = el("LogisticHeader")
        .push(
            el("Role")
                .with("id", "Owner")
                .with("roleFunction", "SENDER"),
        )
        .push(el("Enterprise").with("id", "EH3D").with("code", "NONE"));
    let history = el("HistoryRecord")
        .with("number", "1")
        .with("origination", settings.origination.clone())
        .with("software", "EH3D-PHOENIX-PCB")
        .with("lastChange", settings.origination.clone())
        .push(
            el("FileRevision")
                .with("fileRevisionId", "1")
                .with("comment", "")
                .with("label", "")
                .push(
                    el("SoftwarePackage")
                        .with("name", "phoenix")
                        .with("vendor", "EH3D")
                        .with("revision", option_env!("CARGO_PKG_VERSION").unwrap_or("0")),
                ),
        );

    // ---- the bill of materials
    let mut bom_lines = 0;
    let bom = if settings.include_bom {
        let mut groups: BTreeMap<(String, String), Members> = BTreeMap::new();
        for (_, c) in &parts {
            let fp = board.footprints.get(&c.footprint);
            let key = (
                c.value.clone(),
                fp.map_or_else(String::new, |f| f.name.clone()),
            );
            groups.entry(key).or_default().push((
                c.refdes.as_str(),
                fp.map_or(0, |f| f.pads.len()),
                c.side,
            ));
        }
        bom_lines = groups.len();
        let mut bom = el("Bom").with("name", "Bom").push(
            el("BomHeader")
                .with("assembly", board.name.clone())
                .with("revision", "1")
                .push(el("StepRef").with("name", board.name.clone())),
        );
        for ((value, footprint), members) in &groups {
            let number = match (value.is_empty(), footprint.is_empty()) {
                (true, true) => "NONE".to_owned(),
                (true, false) => footprint.clone(),
                (false, true) => value.clone(),
                (false, false) => format!("{value}_{footprint}"),
            };
            let mut item = el("BomItem")
                .with("OEMDesignNumberRef", number)
                .with("quantity", members.len().to_string())
                .with("pinCount", members[0].1.to_string())
                .with("category", "ELECTRICAL");
            for (refdes, _, side) in members {
                let layer = side_layer(&layers, *side).map_or_else(String::new, |l| l.name.clone());
                item = item.push(
                    el("RefDes")
                        .with("name", (*refdes).to_owned())
                        .with("packageRef", footprint.clone())
                        .with("populate", "true")
                        .with("layerRef", layer),
                );
            }
            bom = bom.push(item);
        }
        Some(bom)
    } else {
        None
    };

    // ---- the step
    let mut step = el("Step")
        .with("name", board.name.clone())
        .push(el("Datum").with("x", "0").with("y", "0"));
    if board.outline.len() >= 3 {
        let mut profile = el("Profile").push(polygon_steps(el("Polygon"), &board.outline));
        for cutout in &board.outline_cutouts {
            if cutout.len() >= 3 {
                profile = profile.push(polygon_steps(el("Cutout"), cutout));
            }
        }
        step = step.push(profile);
    } else {
        findings.push(Finding::new(NO_OUTLINE, Severity::Warning, None, ""));
    }
    for (key, id) in &stack_ids {
        let mut stack = el("PadStack").with("name", id.clone());
        if let Some(drill) = key.drill_nm {
            stack = stack.push(
                el("PadstackHoleDef")
                    .with("name", format!("H_{}", mm(drill)))
                    .with("diameter", mm(drill))
                    .with(
                        "platingStatus",
                        if key.plated { "PLATED" } else { "NONPLATED" },
                    )
                    .with("plusTol", "0")
                    .with("minusTol", "0")
                    .with("x", "0")
                    .with("y", "0"),
            );
        }
        for layer in &key.layers {
            stack = stack.push(
                el("PadstackPadDef")
                    .with("layerRef", layer.clone())
                    .with("padUse", "REGULAR")
                    .push(el("StandardPrimitiveRef").with("id", key.shape.id())),
            );
        }
        step = step.push(stack);
    }

    // the packages: the footprints in use, by name
    let used: BTreeSet<Id> = board.components.values().map(|c| c.footprint).collect();
    let mut packages: Vec<(&str, Id)> = used
        .iter()
        .filter_map(|id| board.footprints.get(id).map(|f| (f.name.as_str(), *id)))
        .collect();
    packages.sort();
    packages.dedup_by(|a, b| a.0 == b.0);
    for (name, id) in &packages {
        let footprint = &board.footprints[id];
        let mut pads: Vec<&Pad> = footprint.pads.values().collect();
        pads.sort_by(|a, b| a.number.cmp(&b.number));
        let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (0_i64, 0_i64, 0_i64, 0_i64);
        for pad in &pads {
            let (hx, hy) = Shape::of(pad).half_box();
            lo_x = lo_x.min(pad.at.x_nm.get() - hx);
            lo_y = lo_y.min(pad.at.y_nm.get() - hy);
            hi_x = hi_x.max(pad.at.x_nm.get() + hx);
            hi_y = hi_y.max(pad.at.y_nm.get() + hy);
        }
        let corners: Vec<Point> = [(lo_x, lo_y), (hi_x, lo_y), (hi_x, hi_y), (lo_x, hi_y)]
            .iter()
            .filter_map(|(x, y)| Point::from_nm(*x, *y).ok())
            .collect();
        let outline = polygon_steps(el("Polygon"), &corners);
        let mut package = el("Package")
            .with("name", (*name).to_owned())
            .with("type", "OTHER")
            .push(el("Outline").push(outline));
        for pad in pads {
            package = package.push(
                el("Pin")
                    .with("number", pad.number.clone())
                    .push(point_attrs("Location", pad.at))
                    .push(el("StandardPrimitiveRef").with("id", Shape::of(pad).id())),
            );
        }
        step = step.push(package);
    }

    // the components
    for (_, c) in &parts {
        let Some(footprint) = board.footprints.get(&c.footprint) else {
            continue;
        };
        let layer = side_layer(&layers, c.side).map_or_else(String::new, |l| l.name.clone());
        let through = footprint.pads.values().any(|p| p.drill.is_some());
        step = step.push(
            el("Component")
                .with("refDes", c.refdes.clone())
                .with("packageRef", footprint.name.clone())
                .with("layerRef", layer)
                .with("mountType", if through { "THMT" } else { "SMT" })
                .push(
                    el("Xform")
                        .with("rotation", degrees(c.rotation.normalised().get()))
                        .with("mirror", yes(c.side == Side::Bottom)),
                )
                .push(point_attrs("Location", c.at)),
        );
    }

    // the nets and their pins
    let mut pins: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for net in board.nets.values() {
        pins.entry(net.name.clone()).or_default();
    }
    for pad_net in &board.pad_nets {
        let (Some(component), Some(name)) = (
            board.components.get(&pad_net.component),
            net_name(pad_net.net),
        ) else {
            continue;
        };
        let Some(pad) = board
            .footprints
            .get(&component.footprint)
            .and_then(|f| f.pads.get(&pad_net.pad))
        else {
            continue;
        };
        pins.entry(name)
            .or_default()
            .push((component.refdes.clone(), pad.number.clone()));
    }
    for (name, list) in &mut pins {
        list.sort();
        let mut net = el("LogicalNet").with("name", name.clone());
        for (refdes, pin) in list.iter() {
            net = net.push(
                el("PinRef")
                    .with("componentRef", refdes.clone())
                    .with("pin", pin.clone()),
            );
        }
        step = step.push(net);
    }

    // the copper layers: sets by net
    let mut counts = Summary {
        format: SUMMARY_FORMAT,
        name: board.name.clone(),
        layers: layers.len(),
        copper_layers: layers.iter().filter(|l| l.copper).count(),
        components: board.components.len(),
        packages: packages.len(),
        padstacks: stack_ids.len(),
        pads: 0,
        vias: board.vias.len(),
        tracks: 0,
        zones: 0,
        dimensions: 0,
        dimension_texts: 0,
        nets: pins.len(),
        bom_lines,
        bytes: 0,
    };
    let mut sets: Vec<BTreeMap<String, Vec<Element>>> =
        layers.iter().map(|_| BTreeMap::new()).collect();
    let no_net = "No Net".to_owned();
    let mut unplated = 0_usize;
    for (id, c) in &parts {
        let Some(footprint) = board.footprints.get(&c.footprint) else {
            continue;
        };
        let mut pads: Vec<(&Id, &Pad)> = footprint.pads.iter().collect();
        pads.sort_by(|a, b| a.1.number.cmp(&b.1.number).then(a.0.cmp(b.0)));
        for (pad_id, pad) in pads {
            if pad.is_npth() {
                unplated += 1;
                continue;
            }
            let pad_ref = PadRef {
                component: *id,
                pad: *pad_id,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let key = stack_ids.get(&pad_key(pad)).cloned().unwrap_or_default();
            let net = board.net_of(pad_ref).and_then(net_name);
            let rotation = (pad.rotation.get() + c.rotation.get()).rem_euclid(360_000_000);
            let mut written = false;
            for layer_id in &pad.layers {
                let Some(&i) = layer_index.get(layer_id) else {
                    continue;
                };
                if !layers[i].copper {
                    continue;
                }
                written = true;
                counts.pads += 1;
                let element = el("Pad")
                    .with("padstackDefRef", key.clone())
                    .push(
                        el("Xform")
                            .with("rotation", degrees(rotation))
                            .with("mirror", yes(c.side == Side::Bottom)),
                    )
                    .push(point_attrs("Location", at))
                    .push(el("StandardPrimitiveRef").with("id", Shape::of(pad).id()))
                    .push(
                        el("PinRef")
                            .with("componentRef", c.refdes.clone())
                            .with("pin", pad.number.clone()),
                    );
                sets[i]
                    .entry(net.clone().unwrap_or_else(|| no_net.clone()))
                    .or_default()
                    .push(element);
            }
            if !written {
                findings.push(Finding::new(
                    PAD_NO_LAYER,
                    Severity::Warning,
                    Some(&pad_name(&c.refdes, &pad.number)),
                    "",
                ));
            }
        }
    }
    for via in board.vias.values() {
        let key = stack_ids.get(&via_key(via)).cloned().unwrap_or_default();
        let net = net_name(via.net).unwrap_or_else(|| no_net.clone());
        for layer_id in via_layers(via) {
            let Some(&i) = layer_index.get(&layer_id) else {
                continue;
            };
            let element = el("Pad")
                .with("padstackDefRef", key.clone())
                .push(point_attrs("Location", via.at))
                .push(
                    el("StandardPrimitiveRef").with("id", Shape::Circle(via.diameter.get()).id()),
                );
            sets[i].entry(net.clone()).or_default().push(element);
        }
    }
    if settings.include_tracks {
        for track in board.tracks.values() {
            let Some(&i) = layer_index.get(&track.layer) else {
                continue;
            };
            if !layers[i].copper || track.path.is_empty() {
                continue;
            }
            counts.tracks += 1;
            let net = net_name(track.net).unwrap_or_else(|| no_net.clone());
            sets[i]
                .entry(net)
                .or_default()
                .push(el("Features").push(track_polyline(track)));
        }
    }
    if settings.include_zones {
        for zone in board.zones.values() {
            let Some(&i) = layer_index.get(&zone.layer) else {
                continue;
            };
            if !layers[i].copper || zone.outline.len() < 3 {
                continue;
            }
            counts.zones += 1;
            let net = net_name(zone.net).unwrap_or_else(|| no_net.clone());
            sets[i]
                .entry(net)
                .or_default()
                .push(el("Features").push(zone_contour(zone)));
        }
    }
    for (layer, by_net) in layers.iter().zip(sets) {
        if !layer.copper {
            continue;
        }
        let mut feature = el("LayerFeature").with("layerRef", layer.name.clone());
        for (net, items) in by_net {
            let mut set = el("Set").with("net", net);
            for item in items {
                set = set.push(item);
            }
            feature = feature.push(set);
        }
        step = step.push(feature);
    }

    // the lines of the dimensions, on the layers of the function DOCUMENT
    let mut dimension_texts = 0_usize;
    let mut dimensions_undrawn = 0_usize;
    for layer in layers.iter().filter(|l| l.function == "DOCUMENT") {
        let mut set = el("Set");
        let mut any = false;
        for dimension in board.dimensions.values().filter(|d| d.layer == layer.id) {
            let Ok(items) = dimension.items() else {
                dimensions_undrawn += 1;
                continue;
            };
            counts.dimensions += 1;
            for item in items {
                match item {
                    SilkItem::Stroke { width, path, .. } => {
                        any = true;
                        set = set.push(el("Features").push(path_polyline(&path, width.get())));
                    }
                    SilkItem::Text {
                        at,
                        text,
                        height,
                        rotation,
                        ..
                    } if settings.include_dimension_texts => {
                        any = true;
                        counts.dimension_texts += 1;
                        let height = if settings.dimension_text_height_nm > 0 {
                            settings.dimension_text_height_nm
                        } else {
                            height.get()
                        };
                        set = set.push(text_feature(at, &text, height, rotation.get()));
                    }
                    SilkItem::Text { .. } => dimension_texts += 1,
                }
            }
        }
        if any {
            step = step.push(
                el("LayerFeature")
                    .with("layerRef", layer.name.clone())
                    .push(set),
            );
        }
    }

    // ---- the layers of the Ecad section
    let mut cad = el("CadData");
    for layer in &layers {
        cad = cad.push(
            el("Layer")
                .with("name", layer.name.clone())
                .with("layerFunction", layer.function)
                .with("side", layer.side)
                .with("polarity", "POSITIVE"),
        );
    }
    cad = cad.push(step);
    let ecad = el("Ecad")
        .with("name", "PHOENIX")
        .push(el("CadHeader").with("units", "MILLIMETER"))
        .push(cad);

    let mut root = el("IPC-2581")
        .with("revision", "C")
        .with("xmlns", "http://webstds.ipc.org/2581")
        .with("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance")
        .with(
            "xsi:schemaLocation",
            "http://webstds.ipc.org/2581 http://webstds.ipc.org/2581/IPC-2581C.xsd",
        )
        .push(content)
        .push(header)
        .push(history);
    if let Some(bom) = bom {
        root = root.push(bom);
    }
    root = root.push(ecad);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
        root.to_xml()
    );

    // ---- what was left out
    let mut left = Vec::new();
    if !board.silkscreen.is_empty() {
        left.push(format!("{} silkscreen items", board.silkscreen.len()));
    }
    if !board.outline_arcs.is_empty() {
        left.push(format!("{} arcs of the outline", board.outline_arcs.len()));
    }
    if unplated > 0 {
        left.push(format!(
            "{unplated} holes that are not plated (in the padstacks only)"
        ));
    }
    if !settings.include_zones && !board.zones.is_empty() {
        left.push("the zones".to_owned());
    }
    if !settings.include_tracks && !board.tracks.is_empty() {
        left.push("the tracks".to_owned());
    }
    if board.zones.values().any(|z| z.outline.len() >= 3) && settings.include_zones {
        left.push("the copper fill of the zones".to_owned());
    }
    if board
        .layers
        .iter()
        .any(|(id, l)| l.kind == LayerKind::Documentation && !layers.iter().any(|f| f.id == *id))
    {
        left.push("the documentation layers".to_owned());
    }
    if dimension_texts > 0 {
        left.push(format!("{dimension_texts} texts of dimensions"));
    }
    if dimensions_undrawn > 0 {
        left.push(format!(
            "{dimensions_undrawn} dimensions that cannot be drawn"
        ));
    }
    left.push("the stack-up".to_owned());
    findings.insert(
        0,
        Finding::new(LEFT_OUT, Severity::Info, None, left.join(", ")),
    );

    counts.bytes = xml.len();
    Ok(Ipc2581 {
        xml,
        summary: counts,
        findings,
    })
}

/// A text as a feature: where it starts (its lower left corner), its turn when it has one, and the
/// characters with their height.
fn text_feature(at: Point, text: &str, height_nm: i64, rotation_udeg: i64) -> Element {
    let mut features = el("Features");
    let turn = rotation_udeg.rem_euclid(360_000_000);
    if turn != 0 {
        features = features.push(el("Xform").with("rotation", degrees(turn)));
    }
    features.push(point_attrs("Location", at)).push(
        el("UserSpecial").push(
            el("Text")
                .with("text", text)
                .with("textHeight", mm(height_nm)),
        ),
    )
}

fn track_polyline(track: &Track) -> Element {
    path_polyline(&track.path, track.width.get())
}

/// A run of lines and arcs as a polyline with round ends and the width `width_nm`.
fn path_polyline(path: &[Segment], width_nm: i64) -> Element {
    let mut line = el("Polyline");
    let mut first = true;
    for segment in path {
        if first {
            line = line.push(point_attrs("PolyBegin", segment.start()));
            first = false;
        }
        match *segment {
            Segment::Line { end, .. } => {
                line = line.push(point_attrs("PolyStepSegment", end));
            }
            Segment::Arc { start, end, sweep } => match arc_centre(start, end, sweep.get()) {
                Some((cx, cy)) => {
                    line = line.push(
                        point_attrs("PolyStepCurve", end)
                            .with("centerX", mm(cx.round() as i64))
                            .with("centerY", mm(cy.round() as i64))
                            .with("clockwise", yes(sweep.get() < 0)),
                    );
                }
                None => {
                    line = line.push(point_attrs("PolyStepSegment", end));
                }
            },
        }
    }
    line.push(
        el("LineDesc")
            .with("lineEnd", "ROUND")
            .with("lineWidth", mm(width_nm)),
    )
}

fn zone_contour(zone: &Zone) -> Element {
    let mut contour = el("Contour").push(polygon_steps(el("Polygon"), &zone.outline));
    for cutout in &zone.cutouts {
        if cutout.len() >= 3 {
            contour = contour.push(polygon_steps(el("Cutout"), cutout));
        }
    }
    contour
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn lengths_and_angles_are_written_without_trailing_zeros() {
        assert_eq!(mm(0), "0");
        assert_eq!(mm(1_500_000), "1.5");
        assert_eq!(mm(20_000_000), "20");
        assert_eq!(mm(-250_000), "-0.25");
        assert_eq!(mm(1), "0.000001");
        assert_eq!(mm(-1_000_001), "-1.000001");
        assert_eq!(degrees(90_000_000), "90");
        assert_eq!(degrees(22_500_000), "22.5");
    }

    #[test]
    fn the_centre_of_an_arc_is_on_the_side_of_its_turn() {
        // a quarter turn counter-clockwise from (1, 0) to (0, 1) about the origin
        let (cx, cy) = arc_centre(pt(1000, 0), pt(0, 1000), 90_000_000).unwrap();
        assert!(cx.abs() < 1e-6 && cy.abs() < 1e-6, "{cx} {cy}");
        // the same end points turned the other way, clockwise: the centre is at (1, 1)
        let (cx, cy) = arc_centre(pt(1000, 0), pt(0, 1000), -90_000_000).unwrap();
        assert!(
            (cx - 1000.0).abs() < 1e-6 && (cy - 1000.0).abs() < 1e-6,
            "{cx} {cy}"
        );
        // a half turn has its centre in the middle of the chord
        let (cx, cy) = arc_centre(pt(0, 0), pt(2000, 0), 180_000_000).unwrap();
        assert!((cx - 1000.0).abs() < 1e-6 && cy.abs() < 1e-6, "{cx} {cy}");
        assert!(arc_centre(pt(5, 5), pt(5, 5), 90_000_000).is_none());
        assert!(arc_centre(pt(0, 0), pt(5, 5), 0).is_none());
    }

    #[test]
    fn the_identifier_of_a_shape_holds_its_size() {
        assert_eq!(Shape::Circle(1_600_000).id(), "C_1.6");
        assert_eq!(Shape::Rect(800_000, 600_000).id(), "R_0.8x0.6");
        assert_eq!(Shape::Oval(2_000_000, 1_000_000).id(), "O_2x1");
        assert_eq!(
            Shape::RoundRect(1_000_000, 1_000_000, 250_000).id(),
            "RR_1x1_0.25"
        );
        assert_eq!(Shape::Rect(800_000, 600_000).half_box(), (400_000, 300_000));
    }

    #[test]
    fn a_polygon_is_closed_by_a_step_back_to_its_first_corner() {
        let square = [pt(0, 0), pt(1_000_000, 0), pt(1_000_000, 1_000_000)];
        let polygon = polygon_steps(el("Polygon"), &square);
        let names: Vec<&str> = polygon.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "PolyBegin",
                "PolyStepSegment",
                "PolyStepSegment",
                "PolyStepSegment"
            ]
        );
        assert_eq!(polygon.children[3].attr("x"), Some("0"));
        assert!(polygon_steps(el("Polygon"), &[]).children.is_empty());
    }
}
