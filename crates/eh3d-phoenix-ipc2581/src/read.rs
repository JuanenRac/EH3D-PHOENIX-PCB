// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of IPC-2581 files (revisions A, B and C, as XML).
//!
//! The file is not trusted. It is read as a stream of events with limits on its size, its depth,
//! its elements and its items ([`ReadOptions`]); an element the reader does not know is left out
//! and reported by name with how many times it appeared; a value that cannot be read (a length,
//! a unit) is left out and reported the same way; nothing in the file can make the reader panic or
//! read beyond the text it was given. Every length is converted from the unit of the file to
//! whole nanometres with exact integer arithmetic ([`crate::units`]).
//!
//! What is read: the layers and their function, side and polarity; the specifications of the
//! header and the stack-up that refers to them (thicknesses, tolerances, material, dielectric
//! constant, loss tangent, resin content, conductivity); for each step its origin, its profile
//! and cutouts, its padstacks, holes and slots, packages, placed parts, nets with their pins and
//! physical nets; for each layer the pads, lines, arcs, polylines and regions of copper, mask and
//! legend; and the bills of materials. The dictionaries of shapes are counted and not kept.
//!
//! **State:** checked against the sample files of several makers of design tools in revisions A,
//! B and C; not against the schema of the standard, and a feature that is placed with a rotation
//! or a mirror keeps the coordinates it is written with (the number of such features is
//! reported).

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    Finding, Ipc2581Error, Severity, READ_LIMIT, READ_NOT_IPC2581, READ_NOT_XML, READ_SKIPPED,
    READ_VALUE_INVALID,
};
use crate::pull::{Event, Node, Pull, PullError, PullLimits, Tag};
use crate::read_model::{
    DictionaryInfo, Hole, IpcArc, IpcBom, IpcBomItem, IpcComponent, IpcDoc, IpcLayer, IpcLine,
    IpcNet, IpcOutline, IpcPackage, IpcPadstack, IpcPhyNet, IpcPolyline, IpcRegion, IpcSpec,
    IpcStackup, IpcStep, LayerContent, LayerFunction, Plating, PolyPoint, Side, StackLayer,
};
use crate::settings::Ipc2581Settings;
use crate::units::{to_f64, to_nm, Unit};

/// The limits and choices of one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadOptions {
    /// Largest input in bytes.
    pub max_bytes: usize,
    /// Deepest nesting of elements.
    pub max_depth: usize,
    /// Most elements.
    pub max_elements: usize,
    /// Most items (holes, parts, nets, pads, tracks, regions, lines of the bill).
    pub max_items: usize,
    /// Keep the geometry of tracks, arcs, polylines and regions.
    pub keep_geometry: bool,
    /// Most attributes on one element.
    pub max_attrs: usize,
    /// Most findings listed.
    pub max_findings: usize,
}

impl Default for ReadOptions {
    fn default() -> Self {
        ReadOptions::from(&Ipc2581Settings::default())
    }
}

impl From<&Ipc2581Settings> for ReadOptions {
    fn from(s: &Ipc2581Settings) -> ReadOptions {
        ReadOptions {
            max_bytes: s.read_max_bytes,
            max_depth: s.read_max_depth,
            max_elements: s.read_max_elements,
            max_items: s.read_max_items,
            keep_geometry: s.read_keep_geometry,
            max_attrs: s.read_max_attrs,
            max_findings: s.read_max_findings,
        }
    }
}

fn limit_error(what: &str) -> Ipc2581Error {
    Ipc2581Error::new(READ_LIMIT, what)
}

fn map_pull(e: PullError) -> Ipc2581Error {
    match e {
        PullError::Malformed { .. } => Ipc2581Error::new(READ_NOT_XML, e.to_string()),
        PullError::Limit(what) => limit_error(what),
    }
}

/// A name from the file made safe to print: control characters become `?` and a long text is cut.
fn clean(text: &str) -> String {
    text.chars()
        .take(80)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// Reads a file given as bytes: a UTF-8 byte order mark is dropped, and bytes that are not UTF-8
/// are read as Latin-1 (and the finding says so).
pub fn read_bytes(bytes: &[u8], options: &ReadOptions) -> Result<IpcDoc, Ipc2581Error> {
    if bytes.len() > options.max_bytes {
        return Err(limit_error("the input is larger than the byte limit"));
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => read_str(text, options),
        Err(_) => {
            let text: String = bytes.iter().map(|b| char::from(*b)).collect();
            let mut doc = read_str(&text, options)?;
            doc.findings.insert(
                0,
                Finding::new(
                    READ_VALUE_INVALID,
                    Severity::Warning,
                    Some("encoding"),
                    "the bytes are not UTF-8 and were read as Latin-1",
                ),
            );
            Ok(doc)
        }
    }
}

/// Reads a file given as text.
pub fn read_str(text: &str, options: &ReadOptions) -> Result<IpcDoc, Ipc2581Error> {
    let mut reader = Reader {
        options: *options,
        doc: IpcDoc::default(),
        units: Unit::Millimeter,
        units_seen: false,
        bad: BTreeMap::new(),
        notes: BTreeMap::new(),
        line_widths: BTreeMap::new(),
        step: None,
        content: 0,
        items: 0,
        pending_specs: Vec::new(),
    };
    reader.run(text)?;
    Ok(reader.finish())
}

struct Reader {
    options: ReadOptions,
    doc: IpcDoc,
    units: Unit,
    units_seen: bool,
    bad: BTreeMap<String, usize>,
    notes: BTreeMap<String, usize>,
    line_widths: BTreeMap<String, i64>,
    step: Option<IpcStep>,
    content: usize,
    items: usize,
    pending_specs: Vec<(usize, usize, String)>,
}

/// Elements that only carry information the reader does not use and that are left out quietly.
const QUIET: &[&str] = &[
    "LogisticHeader",
    "Avl",
    "StepRef",
    "LayerRef",
    "BomRef",
    "AvlRef",
    "LayerDescRef",
    "StepListRef",
    "ChangeRec",
    "ColorRef",
    "TextAttribute",
    "LineDescRef",
    "Location",
    "Xform",
    "UserPrimitiveRef",
    "StandardPrimitiveRef",
    "Fiducial",
    "SpecRef",
];

impl Reader {
    fn run(&mut self, text: &str) -> Result<(), Ipc2581Error> {
        let limits = PullLimits {
            max_bytes: self.options.max_bytes,
            max_depth: self.options.max_depth,
            max_elements: self.options.max_elements,
            max_attrs: self.options.max_attrs,
        };
        let mut pull = Pull::new(text, limits).map_err(map_pull)?;
        let mut path: Vec<&str> = Vec::new();
        loop {
            match pull.next_event().map_err(map_pull)? {
                None => break,
                Some(Event::End(name)) => {
                    path.pop();
                    if name == "Step" {
                        if let Some(step) = self.step.take() {
                            self.doc.steps.push(step);
                        }
                    }
                }
                Some(Event::Start(tag)) => self.start(&mut pull, &mut path, tag)?,
            }
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), Ipc2581Error> {
        self.items += 1;
        if self.items > self.options.max_items {
            return Err(limit_error("the file holds more items than the limit"));
        }
        Ok(())
    }

    fn bad(&mut self, key: &str) {
        *self.bad.entry(key.to_owned()).or_insert(0) += 1;
    }

    fn skip_named(&mut self, name: &str, count: usize) {
        if !QUIET.contains(&name) {
            *self.doc.skipped.entry(clean(name)).or_insert(0) += count;
        }
    }

    fn note(&mut self, text: &str) {
        *self.notes.entry(text.to_owned()).or_insert(0) += 1;
    }

    /// A length attribute in the unit of the header; `None` when it is absent, and a bad value is
    /// counted.
    fn len(&mut self, value: Option<&str>, key: &str) -> Option<i64> {
        let text = value?;
        match to_nm(text, self.units) {
            Some(nm) => Some(nm),
            None => {
                self.bad(key);
                None
            }
        }
    }

    fn coord(&mut self, node: &Node<'_>, x: &str, y: &str, key: &str) -> Option<(i64, i64)> {
        let px = self.len(node.attr(x), key)?;
        let py = self.len(node.attr(y), key)?;
        Some((px, py))
    }

    fn start<'a>(
        &mut self,
        pull: &mut Pull<'a>,
        path: &mut Vec<&'a str>,
        tag: Tag<'a>,
    ) -> Result<(), Ipc2581Error> {
        let parent = path.last().copied();
        let name = tag.name;
        match (parent, name) {
            (None, _) => {
                if name != "IPC-2581" {
                    return Err(Ipc2581Error::new(
                        READ_NOT_IPC2581,
                        format!("the root element is {}", clean(name)),
                    ));
                }
                tag.attr("revision")
                    .unwrap_or("")
                    .trim()
                    .clone_into(&mut self.doc.revision);
                path.push(name);
            }
            (Some("IPC-2581"), "Content" | "Ecad") => path.push(name),
            (Some("IPC-2581"), "HistoryRecord") => {
                if let Some(software) = tag.attr("software") {
                    self.doc.software = clean(software);
                }
                pull.skip_subtree().map_err(map_pull)?;
            }
            (Some("IPC-2581"), "Bom") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_bom(&node)?;
            }
            (Some("Content"), "FunctionMode") => {
                if let Some(mode) = tag.attr("mode") {
                    self.doc.modes.push(clean(mode));
                }
                pull.skip_subtree().map_err(map_pull)?;
            }
            (Some("Content"), "DictionaryLineDesc") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_line_dictionary(&node);
            }
            (Some("Content"), n) if n.starts_with("Dictionary") => {
                let unit = tag.attr("units").and_then(Unit::parse);
                let elements = pull.skip_subtree().map_err(map_pull)?;
                self.doc.dictionaries.push(DictionaryInfo {
                    name: clean(n),
                    unit,
                    elements,
                });
            }
            (Some("Ecad"), "CadHeader") => {
                match tag.attr("units").and_then(Unit::parse) {
                    Some(unit) => {
                        self.units = unit;
                        self.units_seen = true;
                        self.doc.units = Some(unit);
                    }
                    None => self.bad("CadHeader@units"),
                }
                path.push(name);
            }
            (Some("CadHeader"), "Spec") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_spec(&node);
            }
            (Some("Ecad"), "CadData") | (Some("CadData"), "LayerDesc" | "StepList") => {
                path.push(name);
            }
            (Some("CadData" | "LayerDesc"), "Layer") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_layer(&node);
            }
            (Some("CadData" | "LayerDesc"), "Stackup") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_stackup(&node);
            }
            (Some("CadData" | "StepList"), "Step") => {
                self.step = Some(IpcStep {
                    name: clean(tag.attr("name").unwrap_or("")),
                    ..IpcStep::default()
                });
                self.content = 0;
                path.push(name);
            }
            (Some("Step"), "Datum") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                let datum = self.coord(&node, "x", "y", "Datum@x,y");
                if let Some(step) = self.step.as_mut() {
                    step.datum_nm = datum;
                }
            }
            (Some("Step"), "Profile") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                let outline = self.read_outline(&node);
                if let Some(step) = self.step.as_mut() {
                    step.profile = outline;
                }
            }
            (Some("Step"), "PadStackDef") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_padstack_def(&node)?;
            }
            (Some("Step"), "PadStack") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_padstack_old(&node)?;
            }
            (Some("Step"), "Package") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_package(&node);
            }
            (Some("Step"), "Component") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_component(&node)?;
            }
            (Some("Step"), "LogicalNet") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_net(&node)?;
            }
            (Some("Step"), "PhyNetGroup") => path.push(name),
            (Some("PhyNetGroup"), "PhyNet") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.tick()?;
                let net = IpcPhyNet {
                    name: clean(node.attr("name").unwrap_or("")),
                    points: node.children_named("PhyNetPoint").count(),
                };
                if let Some(step) = self.step.as_mut() {
                    step.phy_nets.push(net);
                }
            }
            (Some("Step"), "StackupZone" | "BendArea") => {
                pull.skip_subtree().map_err(map_pull)?;
                if let Some(step) = self.step.as_mut() {
                    step.zones += 1;
                }
            }
            (Some("Step"), "LayerFeature") => {
                let layer = match tag.attr("layerRef") {
                    Some(l) => clean(l),
                    None => {
                        self.bad("LayerFeature@layerRef");
                        String::new()
                    }
                };
                if let Some(step) = self.step.as_mut() {
                    step.contents.push(LayerContent {
                        layer,
                        ..LayerContent::default()
                    });
                    self.content = step.contents.len() - 1;
                }
                path.push(name);
            }
            (Some("LayerFeature"), "Set") => {
                let node = pull.subtree(tag).map_err(map_pull)?;
                self.read_set(&node)?;
            }
            _ => {
                let count = pull.skip_subtree().map_err(map_pull)?;
                self.skip_named(name, count);
            }
        }
        Ok(())
    }

    fn read_line_dictionary(&mut self, node: &Node<'_>) {
        let unit = node
            .attr("units")
            .and_then(Unit::parse)
            .unwrap_or(self.units);
        let mut entries = 0usize;
        for entry in node.children_named("EntryLineDesc") {
            entries += 1;
            let id = match entry.attr("id") {
                Some(id) => id.to_owned(),
                None => continue,
            };
            let width = entry
                .child("LineDesc")
                .and_then(|d| d.attr("lineWidth"))
                .and_then(|w| to_nm(w, unit));
            match width {
                Some(w) => {
                    self.line_widths.insert(id, w);
                }
                None => self.bad("EntryLineDesc@lineWidth"),
            }
        }
        self.doc.dictionaries.push(DictionaryInfo {
            name: "DictionaryLineDesc".to_owned(),
            unit: Some(unit),
            elements: entries,
        });
    }

    fn read_spec(&mut self, node: &Node<'_>) {
        let name = match node.attr("name") {
            Some(n) => clean(n),
            None => {
                self.bad("Spec@name");
                return;
            }
        };
        let mut spec = IpcSpec {
            name,
            ..IpcSpec::default()
        };
        for part in &node.children {
            let kind = part.attr("type").unwrap_or("");
            let value_text = part.child("Property").and_then(|p| p.attr("value"));
            let value = value_text.and_then(to_f64);
            if value_text.is_some() && value.is_none() {
                self.bad("Property@value");
            }
            match (part.name(), kind) {
                ("General", "MATERIAL") => {
                    spec.material = part
                        .child("Property")
                        .and_then(|p| p.attr("text"))
                        .map(clean);
                }
                ("Dielectric", "DIELECTRIC_CONSTANT") => spec.dk = value,
                ("Dielectric", "LOSS_TANGENT") => spec.df = value,
                ("Dielectric", "RESIN_CONTENT") => spec.resin_percent = value,
                ("Conductor", "CONDUCTIVITY") => {
                    let unit = part
                        .child("Property")
                        .and_then(|p| p.attr("unit"))
                        .unwrap_or("")
                        .to_ascii_uppercase();
                    spec.conductivity_s_per_m = match (value, unit.as_str()) {
                        (Some(v), "MHO/CM" | "S/CM") => Some(v * 100.0),
                        (Some(v), "MHO/M" | "S/M") => Some(v),
                        (Some(_), _) => {
                            self.bad("Conductor@unit");
                            None
                        }
                        (None, _) => None,
                    };
                }
                ("Impedance", _) => {
                    if let Some(v) = part.attr("value").and_then(to_f64) {
                        spec.impedance_ohm = Some(v);
                    }
                }
                _ => {}
            }
        }
        self.doc.specs.push(spec);
    }

    fn read_layer(&mut self, node: &Node<'_>) {
        let Some(name) = node.attr("name") else {
            self.bad("Layer@name");
            return;
        };
        let span = node
            .child("Span")
            .and_then(|s| Some((clean(s.attr("fromLayer")?), clean(s.attr("toLayer")?))));
        self.doc.layers.push(IpcLayer {
            name: clean(name),
            function: LayerFunction::parse(node.attr("layerFunction").unwrap_or("")),
            side: Side::parse(node.attr("side").unwrap_or("")),
            negative: node
                .attr("polarity")
                .is_some_and(|p| p.trim().eq_ignore_ascii_case("NEGATIVE")),
            span,
            spec_refs: node
                .children_named("SpecRef")
                .filter_map(|s| s.attr("id").map(clean))
                .collect(),
        });
    }

    fn read_stackup(&mut self, node: &Node<'_>) {
        let overall = self.len(node.attr("overallThickness"), "Stackup@overallThickness");
        let tol_plus = self.len(node.attr("tolPlus"), "Stackup@tolPlus");
        let tol_minus = self.len(node.attr("tolMinus"), "Stackup@tolMinus");
        let mut stackup = IpcStackup {
            name: clean(node.attr("name").unwrap_or("")),
            overall_thickness_nm: overall,
            tol_plus_nm: tol_plus,
            tol_minus_nm: tol_minus,
            where_measured: clean(node.attr("whereMeasured").unwrap_or("")),
            layers: Vec::new(),
        };
        let stack_index = self.doc.stackups.len();
        let mut groups: BTreeSet<String> = BTreeSet::new();
        self.collect_stack(node, "", &mut stackup.layers, &mut groups, stack_index);
        // a layer that names a group is the group itself, not a layer of the board
        let layer_names: BTreeSet<&str> = self.doc.layers.iter().map(|l| l.name.as_str()).collect();
        let before = stackup.layers.len();
        let dropped: BTreeSet<usize> = stackup
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                groups.contains(&l.layer_ref) && !layer_names.contains(l.layer_ref.as_str())
            })
            .map(|(i, _)| i)
            .collect();
        if !dropped.is_empty() {
            let mut index = 0usize;
            stackup.layers.retain(|_| {
                let keep = !dropped.contains(&index);
                index += 1;
                keep
            });
            // the pending references point at positions; renumber the ones of this stack-up
            let mut new_position: BTreeMap<usize, usize> = BTreeMap::new();
            let mut next = 0usize;
            for old in 0..before {
                if !dropped.contains(&old) {
                    new_position.insert(old, next);
                    next += 1;
                }
            }
            let mut kept = Vec::new();
            for (s, l, id) in self.pending_specs.drain(..) {
                if s != stack_index {
                    kept.push((s, l, id));
                } else if let Some(p) = new_position.get(&l) {
                    kept.push((s, *p, id));
                }
            }
            self.pending_specs = kept;
        }
        self.doc.stackups.push(stackup);
    }

    fn collect_stack(
        &mut self,
        node: &Node<'_>,
        group: &str,
        out: &mut Vec<StackLayer>,
        groups: &mut BTreeSet<String>,
        stack_index: usize,
    ) {
        for child in &node.children {
            match child.name() {
                "StackupGroup" => {
                    let name = clean(child.attr("name").unwrap_or(""));
                    groups.insert(name.clone());
                    self.collect_stack(child, &name, out, groups, stack_index);
                }
                "StackupLayer" => {
                    let layer_ref = clean(child.attr("layerOrGroupRef").unwrap_or(""));
                    let thickness_nm = self.len(child.attr("thickness"), "StackupLayer@thickness");
                    let tol_plus_nm = self.len(child.attr("tolPlus"), "StackupLayer@tolPlus");
                    let tol_minus_nm = self.len(child.attr("tolMinus"), "StackupLayer@tolMinus");
                    let sequence = child.attr("sequence").and_then(|s| s.trim().parse().ok());
                    let position = out.len();
                    for spec_ref in child.children_named("SpecRef") {
                        if let Some(id) = spec_ref.attr("id") {
                            self.pending_specs.push((stack_index, position, clean(id)));
                        }
                    }
                    out.push(StackLayer {
                        layer_ref,
                        thickness_nm,
                        tol_plus_nm,
                        tol_minus_nm,
                        sequence,
                        group: group.to_owned(),
                        material_type: child.attr("materialType").map(clean),
                        spec: None,
                    });
                }
                _ => {}
            }
        }
    }

    fn poly_points(&mut self, poly: &Node<'_>, offset: (i64, i64)) -> Vec<PolyPoint> {
        let mut points = Vec::new();
        for step in &poly.children {
            let curve = match step.name() {
                "PolyBegin" | "PolyStepSegment" => false,
                "PolyStepCurve" => true,
                _ => continue,
            };
            let Some((x, y)) = self.coord(step, "x", "y", "Poly@x,y") else {
                continue;
            };
            let center_nm = if curve {
                self.coord(step, "centerX", "centerY", "PolyStepCurve@centerX,centerY")
                    .map(|(cx, cy)| (cx.saturating_add(offset.0), cy.saturating_add(offset.1)))
            } else {
                None
            };
            let clockwise = step
                .attr("clockwise")
                .is_some_and(|c| c.trim().eq_ignore_ascii_case("true"));
            points.push(PolyPoint {
                x_nm: x.saturating_add(offset.0),
                y_nm: y.saturating_add(offset.1),
                center_nm,
                clockwise,
            });
        }
        points
    }

    fn read_outline(&mut self, node: &Node<'_>) -> Option<IpcOutline> {
        let polygon = node.child("Polygon")?;
        let outer = self.poly_points(polygon, (0, 0));
        let mut cutouts = Vec::new();
        for cut in node.children_named("Cutout") {
            cutouts.push(self.poly_points(cut, (0, 0)));
        }
        Some(IpcOutline { outer, cutouts })
    }

    fn hole_of(&mut self, node: &Node<'_>, layer: &str, offset: (i64, i64)) -> Option<Hole> {
        let diameter = self.len(node.attr("diameter"), "Hole@diameter");
        let Some(diameter_nm) = diameter else {
            if node.attr("diameter").is_none() {
                self.bad("Hole@diameter");
            }
            return None;
        };
        let x = self.len(node.attr("x"), "Hole@x").unwrap_or(0);
        let y = self.len(node.attr("y"), "Hole@y").unwrap_or(0);
        Some(Hole {
            name: clean(node.attr("name").unwrap_or("")),
            diameter_nm,
            plating: Plating::parse(node.attr("platingStatus").unwrap_or("")),
            plus_tol_nm: self.len(node.attr("plusTol"), "Hole@plusTol").unwrap_or(0),
            minus_tol_nm: self
                .len(node.attr("minusTol"), "Hole@minusTol")
                .unwrap_or(0),
            x_nm: x.saturating_add(offset.0),
            y_nm: y.saturating_add(offset.1),
            layer: layer.to_owned(),
        })
    }

    fn read_padstack_def(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        let mut def = IpcPadstack {
            name: clean(node.attr("name").unwrap_or("")),
            holes: Vec::new(),
            pads: Vec::new(),
        };
        for child in &node.children {
            match child.name() {
                "PadstackHoleDef" => {
                    self.tick()?;
                    if let Some(h) = self.hole_of(child, "", (0, 0)) {
                        def.holes.push(h);
                    }
                }
                "PadstackPadDef" => def.pads.push((
                    clean(child.attr("layerRef").unwrap_or("")),
                    clean(child.attr("padUse").unwrap_or("")),
                )),
                _ => {}
            }
        }
        if let Some(step) = self.step.as_mut() {
            step.padstacks.push(def);
        }
        Ok(())
    }

    /// The older form of a padstack: a hole for each `LayerHole`, a count for each `LayerPad`.
    fn read_padstack_old(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        let pads = node.children_named("LayerPad").count();
        let mut holes = Vec::new();
        for child in node.children_named("LayerHole") {
            self.tick()?;
            if let Some(h) = self.hole_of(child, "", (0, 0)) {
                holes.push(h);
            }
        }
        if let Some(step) = self.step.as_mut() {
            step.holes.append(&mut holes);
            let at = match step.contents.iter().position(|c| c.layer.is_empty()) {
                Some(at) => at,
                None => {
                    step.contents.push(LayerContent::default());
                    step.contents.len() - 1
                }
            };
            step.contents[at].pads = step.contents[at].pads.saturating_add(pads);
        }
        for _ in 0..pads {
            self.tick()?;
        }
        Ok(())
    }

    fn read_package(&mut self, node: &Node<'_>) {
        let height = self.len(node.attr("height"), "Package@height");
        let package = IpcPackage {
            name: clean(node.attr("name").unwrap_or("")),
            kind: clean(node.attr("type").unwrap_or("")),
            height_nm: height,
            pins: node.children_named("Pin").count(),
        };
        if let Some(step) = self.step.as_mut() {
            step.packages.push(package);
        }
    }

    fn read_component(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        self.tick()?;
        let (x, y) = node
            .child("Location")
            .and_then(|l| self.coord(l, "x", "y", "Component@Location"))
            .unwrap_or((0, 0));
        let (rotation_deg, mirror) = match node.child("Xform") {
            Some(xf) => {
                let rotation = match xf.attr("rotation") {
                    Some(r) => to_f64(r).unwrap_or_else(|| {
                        self.bad("Xform@rotation");
                        0.0
                    }),
                    None => 0.0,
                };
                let mirror = xf
                    .attr("mirror")
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("true"));
                (rotation, mirror)
            }
            None => (0.0, false),
        };
        let component = IpcComponent {
            ref_des: clean(node.attr("refDes").unwrap_or("")),
            package: clean(node.attr("packageRef").unwrap_or("")),
            part: clean(node.attr("part").unwrap_or("")),
            layer: clean(node.attr("layerRef").unwrap_or("")),
            mount: clean(node.attr("mountType").unwrap_or("")),
            x_nm: x,
            y_nm: y,
            rotation_deg,
            mirror,
        };
        if let Some(step) = self.step.as_mut() {
            step.components.push(component);
        }
        Ok(())
    }

    fn read_net(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        self.tick()?;
        let mut pins = Vec::new();
        for pin in node
            .children_named("PinRef")
            .chain(node.children_named("LogicalNetPin"))
        {
            let component = pin.attr("componentRef").unwrap_or("");
            let number = pin.attr("pin").unwrap_or("");
            pins.push((clean(component), clean(number)));
        }
        let net = IpcNet {
            name: clean(node.attr("name").unwrap_or("")),
            class: clean(node.attr("netClass").unwrap_or("")),
            pins,
        };
        if let Some(step) = self.step.as_mut() {
            step.nets.push(net);
        }
        Ok(())
    }

    fn read_bom(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        let mut bom = IpcBom {
            name: clean(node.attr("name").unwrap_or("")),
            items: Vec::new(),
        };
        for item in node.children_named("BomItem") {
            self.tick()?;
            let number = |reader: &mut Reader, key: &str, value: Option<&str>| -> Option<u64> {
                let text = value?;
                match text.trim().parse::<u64>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        reader.bad(key);
                        None
                    }
                }
            };
            let quantity = number(self, "BomItem@quantity", item.attr("quantity"));
            let pin_count = number(self, "BomItem@pinCount", item.attr("pinCount"));
            bom.items.push(IpcBomItem {
                part: clean(item.attr("OEMDesignNumberRef").unwrap_or("")),
                quantity,
                pin_count,
                category: clean(item.attr("category").unwrap_or("")),
                ref_des: item
                    .children_named("RefDes")
                    .map(|r| clean(r.attr("name").unwrap_or("")))
                    .collect(),
            });
        }
        self.doc.boms.push(bom);
        Ok(())
    }

    fn width_of(&mut self, node: &Node<'_>) -> i64 {
        if let Some(desc) = node.child("LineDesc") {
            return self
                .len(desc.attr("lineWidth"), "LineDesc@lineWidth")
                .unwrap_or(0);
        }
        if let Some(id) = node.child("LineDescRef").and_then(|r| r.attr("id")) {
            return match self.line_widths.get(id) {
                Some(w) => *w,
                None => {
                    self.bad("LineDescRef@id");
                    0
                }
            };
        }
        0
    }

    fn read_set(&mut self, node: &Node<'_>) -> Result<(), Ipc2581Error> {
        let net = clean(node.attr("net").unwrap_or(""));
        let layer = self
            .step
            .as_ref()
            .and_then(|s| s.contents.get(self.content))
            .map(|c| c.layer.clone())
            .unwrap_or_default();
        for child in &node.children {
            match child.name() {
                "Pad" => {
                    self.tick()?;
                    self.content_mut(|c| c.pads += 1);
                }
                "Hole" => {
                    self.tick()?;
                    if let Some(h) = self.hole_of(child, &layer, (0, 0)) {
                        if let Some(step) = self.step.as_mut() {
                            step.holes.push(h);
                        }
                        self.content_mut(|c| c.holes += 1);
                    }
                }
                "SlotCavity" => {
                    self.tick()?;
                    self.content_mut(|c| c.slots += 1);
                }
                "Features" => {
                    let offset = child
                        .child("Location")
                        .and_then(|l| self.coord(l, "x", "y", "Features@Location"))
                        .unwrap_or((0, 0));
                    if let Some(xf) = child.child("Xform") {
                        let turned = xf
                            .attr("rotation")
                            .and_then(to_f64)
                            .is_some_and(|r| r != 0.0);
                        let mirrored = xf
                            .attr("mirror")
                            .is_some_and(|m| m.trim().eq_ignore_ascii_case("true"));
                        if turned || mirrored {
                            self.note("features placed with a rotation or a mirror");
                        }
                    }
                    self.geometry(child, &net, offset)?;
                }
                other => self.skip_named(other, 1),
            }
        }
        Ok(())
    }

    fn content_mut(&mut self, f: impl FnOnce(&mut LayerContent)) {
        let index = self.content;
        if let Some(c) = self.step.as_mut().and_then(|s| s.contents.get_mut(index)) {
            f(c);
        }
    }

    fn geometry(
        &mut self,
        parent: &Node<'_>,
        net: &str,
        offset: (i64, i64),
    ) -> Result<(), Ipc2581Error> {
        let keep = self.options.keep_geometry;
        for child in &parent.children {
            match child.name() {
                "Line" => {
                    self.tick()?;
                    let from = self.coord(child, "startX", "startY", "Line@start");
                    let to = self.coord(child, "endX", "endY", "Line@end");
                    let width = self.width_of(child);
                    self.content_mut(|c| c.lines += 1);
                    if let (true, Some(f), Some(t)) = (keep, from, to) {
                        let item = IpcLine {
                            net: net.to_owned(),
                            from_nm: shift(f, offset),
                            to_nm: shift(t, offset),
                            width_nm: width,
                        };
                        self.content_mut(|c| c.line_items.push(item));
                    }
                }
                "Arc" => {
                    self.tick()?;
                    let from = self.coord(child, "startX", "startY", "Arc@start");
                    let to = self.coord(child, "endX", "endY", "Arc@end");
                    let center = self.coord(child, "centerX", "centerY", "Arc@center");
                    let width = self.width_of(child);
                    let clockwise = child
                        .attr("clockwise")
                        .is_some_and(|c| c.trim().eq_ignore_ascii_case("true"));
                    self.content_mut(|c| c.arcs += 1);
                    if let (true, Some(f), Some(t), Some(m)) = (keep, from, to, center) {
                        let item = IpcArc {
                            net: net.to_owned(),
                            from_nm: shift(f, offset),
                            to_nm: shift(t, offset),
                            center_nm: shift(m, offset),
                            clockwise,
                            width_nm: width,
                        };
                        self.content_mut(|c| c.arc_items.push(item));
                    }
                }
                "Polyline" => {
                    self.tick()?;
                    let width = self.width_of(child);
                    self.content_mut(|c| c.polylines += 1);
                    if keep {
                        let points = self.poly_points(child, offset);
                        let item = IpcPolyline {
                            net: net.to_owned(),
                            width_nm: width,
                            points,
                        };
                        self.content_mut(|c| c.polyline_items.push(item));
                    }
                }
                "Contour" => {
                    self.tick()?;
                    self.content_mut(|c| c.regions += 1);
                    if keep {
                        let outer = child
                            .child("Polygon")
                            .map(|p| self.poly_points(p, offset))
                            .unwrap_or_default();
                        let mut cutouts = Vec::new();
                        for cut in child.children_named("Cutout") {
                            cutouts.push(self.poly_points(cut, offset));
                        }
                        let item = IpcRegion {
                            net: net.to_owned(),
                            outer,
                            cutouts,
                        };
                        self.content_mut(|c| c.region_items.push(item));
                    }
                }
                "UserSpecial" => self.geometry(child, net, offset)?,
                other => self.skip_named(other, 1),
            }
        }
        Ok(())
    }

    fn finish(mut self) -> IpcDoc {
        // the specifications the layers of the stack-up refer to
        let pending = std::mem::take(&mut self.pending_specs);
        for (s, l, id) in pending {
            let spec = self.doc.specs.iter().find(|p| p.name == id).cloned();
            let target = self
                .doc
                .stackups
                .get_mut(s)
                .and_then(|st| st.layers.get_mut(l));
            if let (Some(layer), Some(spec)) = (target, spec) {
                layer.spec = Some(match layer.spec.take() {
                    None => spec,
                    Some(mut have) => {
                        have.material = have.material.or(spec.material);
                        have.dk = have.dk.or(spec.dk);
                        have.df = have.df.or(spec.df);
                        have.resin_percent = have.resin_percent.or(spec.resin_percent);
                        have.conductivity_s_per_m =
                            have.conductivity_s_per_m.or(spec.conductivity_s_per_m);
                        have.impedance_ohm = have.impedance_ohm.or(spec.impedance_ohm);
                        have
                    }
                });
            }
        }
        let mut findings: Vec<Finding> = Vec::new();
        if !self.units_seen {
            findings.push(Finding::new(
                READ_VALUE_INVALID,
                Severity::Warning,
                Some("CadHeader@units"),
                "the unit of the lengths is missing or unknown; millimetres are assumed",
            ));
        }
        for (name, count) in &self.doc.skipped {
            findings.push(Finding::new(
                READ_SKIPPED,
                Severity::Info,
                Some(name),
                format!("{count} times"),
            ));
        }
        for (key, count) in &self.bad {
            findings.push(Finding::new(
                READ_VALUE_INVALID,
                Severity::Warning,
                Some(key),
                format!("{count} times"),
            ));
        }
        for (text, count) in &self.notes {
            findings.push(Finding::new(
                READ_VALUE_INVALID,
                Severity::Info,
                None,
                format!("{text}: {count}"),
            ));
        }
        if findings.len() > self.options.max_findings {
            let more = findings.len() - self.options.max_findings;
            findings.truncate(self.options.max_findings);
            findings.push(Finding::new(
                READ_SKIPPED,
                Severity::Info,
                None,
                format!("{more} more findings are not listed"),
            ));
        }
        self.doc.findings = findings;
        self.doc
    }
}

fn shift(point: (i64, i64), offset: (i64, i64)) -> (i64, i64) {
    (
        point.0.saturating_add(offset.0),
        point.1.saturating_add(offset.1),
    )
}
