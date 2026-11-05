// SPDX-License-Identifier: GPL-3.0-or-later
//! Schematics: a `.kicad_sch` file, and the sheets it refers to, read into the schematic model.
//!
//! What is read: the symbol definitions of the file (`lib_symbols`), the placed symbols with their
//! reference, value, footprint name, position and turn, the wires, the junctions, the labels
//! (local, global and hierarchical), the power symbols, and the sub-sheets (through a loader the
//! caller gives, so that this crate opens no file itself).
//!
//! KiCad does not store the nets in a schematic, so they are computed here from the drawing: pins,
//! wire ends, junctions and labels that share a position are joined; a point that lies in the middle
//! of a horizontal or vertical wire is joined to it; labels with one name are joined within a
//! sheet, global labels and power symbols everywhere, and a hierarchical label to the pin of the
//! same name on the sheet symbol above. The name of a net is the name of a global label or power
//! symbol, otherwise that of a local label with the path of its sheet (`/NAME`, `/Sub/NAME`),
//! otherwise `Net-(R1-Pad1)` after its first pin. A net with one pin and no name is not stored.
//!
//! Not kept: the drawing of symbols and texts, graphic lines, rectangles, images, buses, no-connect
//! flags, the fields other than the reference, the value and the footprint, and the sheet
//! properties other than the name, the file and the paper size. A part that is drawn as several
//! units becomes one part whose symbol holds the pins of all its units, placed relative to the
//! first unit. A symbol that is mirrored gets a mirrored copy of its symbol. A rotation that is not
//! a right angle is rounded.

use std::collections::BTreeMap;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::Id;
use eh3d_phoenix_hub::QualityReport;
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_schematic::model::{NetPin, Part, Pin, Schematic, SchematicNet, Symbol};
use eh3d_phoenix_schematic::{Placement, Sheet, Wire};

use crate::conv::{
    at_args, check_root, decode, is_safe_relative_name, nm_text, to_model, turn_up, xy_args, Ctx, P,
};
use crate::error::KicadError;
use crate::settings::KicadSettings;
use crate::sexpr::{parse, Node};
use crate::symbols::{build_symbol, parse_symbol, resolve_extends, PinInfo, SymbolDef};

/// A schematic read into the model.
#[derive(Debug, Clone)]
pub struct ReadSchematic {
    /// The schematic, with the parts of every sheet that was read.
    pub schematic: Schematic,
    /// The footprint each part names (`Resistor_SMD:R_0603_1608Metric`), by part.
    pub footprint_names: BTreeMap<Id, String>,
    /// The files of the sheets the schematic refers to, in order.
    pub sheet_files: Vec<String>,
    /// How many sheets were read, the first one included.
    pub sheets_read: usize,
    /// The version the file declares.
    pub version: String,
    /// The program that wrote the file.
    pub generator: String,
    /// What was kept, changed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    Local,
    Global,
    Hierarchical,
}

#[derive(Debug, Clone)]
struct RawLabel {
    text: String,
    at: P,
    kind: LabelKind,
}

#[derive(Debug, Clone)]
struct RawPart {
    lib_id: String,
    unit: u32,
    at: P,
    quarters: i64,
    mirror: (bool, bool),
    reference: String,
    value: String,
    footprint: String,
    uuid: String,
    instance_reference: Option<String>,
    line: u32,
}

#[derive(Debug, Clone)]
struct RawSheetRef {
    name: String,
    file: String,
    pins: Vec<(String, P)>,
    line: u32,
}

#[derive(Debug, Default)]
struct Parsed {
    title: String,
    paper: Option<P>,
    lib: BTreeMap<String, SymbolDef>,
    parts: Vec<RawPart>,
    wires: Vec<(P, P)>,
    junctions: Vec<P>,
    labels: Vec<RawLabel>,
    sheets: Vec<RawSheetRef>,
    references: BTreeMap<String, String>,
}

/// The size of a sheet of paper in nanometres.
fn paper_size(name: &str) -> Option<P> {
    let (w, h): (i64, i64) = match name {
        "A5" => (210_000_000, 148_000_000),
        "A4" => (297_000_000, 210_000_000),
        "A3" => (420_000_000, 297_000_000),
        "A2" => (594_000_000, 420_000_000),
        "A1" => (841_000_000, 594_000_000),
        "A0" => (1_189_000_000, 841_000_000),
        "A" => (279_400_000, 215_900_000),
        "B" => (431_800_000, 279_400_000),
        "C" => (558_800_000, 431_800_000),
        "D" => (863_600_000, 558_800_000),
        "E" => (1_117_600_000, 863_600_000),
        _ => return None,
    };
    Some((w, h))
}

fn property_named(node: &Node, names: &[&str]) -> Option<String> {
    node.children("property").find_map(|p| {
        let key: String = p
            .arg_text(0)?
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase();
        if names.contains(&key.as_str()) {
            Some(p.arg_text(1).unwrap_or("").to_owned())
        } else {
            None
        }
    })
}

fn parse_label(node: &Node, kind: LabelKind, ctx: &mut Ctx) -> Option<RawLabel> {
    let Some(text) = node.arg_text(0) else {
        ctx.invalid(node, "label", "it has no text");
        return None;
    };
    let Some((at, _)) = node.child("at").and_then(at_args) else {
        ctx.invalid(node, "label position", "not two numbers");
        return None;
    };
    Some(RawLabel {
        text: text.to_owned(),
        at,
        kind,
    })
}

fn parse_part(node: &Node, ctx: &mut Ctx) -> Option<RawPart> {
    let Some(lib_id) = node.child("lib_id").and_then(|n| n.arg_text(0)) else {
        ctx.invalid(node, "symbol", "it has no lib_id");
        return None;
    };
    let Some((at, angle)) = node.child("at").and_then(at_args) else {
        ctx.invalid(node, "symbol position", "not two numbers");
        return None;
    };
    let quarter = 90_000_000;
    if angle % quarter != 0 {
        ctx.simplify(
            "symbol rotation that is not a right angle",
            "rounded to the nearest right angle",
        );
    }
    let quarters = ((angle + quarter / 2).div_euclid(quarter)).rem_euclid(4);
    let mut mirror = (false, false);
    if let Some(m) = node.child("mirror") {
        mirror.0 = m.has_word("x");
        mirror.1 = m.has_word("y");
    }
    let unit = node
        .child("unit")
        .and_then(|u| u.arg_text(0))
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(1);
    Some(RawPart {
        lib_id: lib_id.to_owned(),
        unit,
        at,
        quarters,
        mirror,
        reference: property_named(node, &["reference"]).unwrap_or_default(),
        value: property_named(node, &["value"]).unwrap_or_default(),
        footprint: property_named(node, &["footprint"]).unwrap_or_default(),
        uuid: node
            .child("uuid")
            .and_then(|u| u.arg_text(0))
            .unwrap_or("")
            .to_owned(),
        instance_reference: node
            .child("instances")
            .and_then(|i| i.child("project"))
            .and_then(|p| p.child("path"))
            .and_then(|p| p.child("reference"))
            .and_then(|r| r.arg_text(0))
            .map(str::to_owned),
        line: node.line,
    })
}

fn parse_sheet_ref(node: &Node, ctx: &mut Ctx) -> Option<RawSheetRef> {
    let file = property_named(node, &["sheetfile", "file"]).unwrap_or_default();
    let name = property_named(node, &["sheetname", "name"]).unwrap_or_default();
    if file.is_empty() {
        ctx.invalid(node, "sheet", "it names no file");
        return None;
    }
    let mut pins = Vec::new();
    for pin in node.children("pin") {
        let pin_name = pin.arg_text(0).unwrap_or("");
        match pin.child("at").and_then(at_args) {
            Some((at, _)) => pins.push((pin_name.to_owned(), at)),
            None => ctx.invalid(pin, "sheet pin position", "not two numbers"),
        }
    }
    Some(RawSheetRef {
        name: if name.is_empty() { file.clone() } else { name },
        file,
        pins,
        line: node.line,
    })
}

fn parse_sheet(root: &Node, ctx: &mut Ctx) -> Parsed {
    let mut p = Parsed::default();
    let max = ctx.settings.max_items;
    for child in root.args() {
        let Some(head) = child.head() else {
            continue;
        };
        match head {
            "version" | "generator" | "generator_version" | "uuid" | "sheet_instances"
            | "embedded_fonts" | "embedded_files" => {}
            "paper" => {
                p.paper = match child.arg_text(0) {
                    Some("User") => {
                        match (
                            child.arg_text(1).and_then(nm_text),
                            child.arg_text(2).and_then(nm_text),
                        ) {
                            (Some(w), Some(h)) => Some((w, h)),
                            _ => None,
                        }
                    }
                    Some(name) => paper_size(name),
                    None => None,
                };
            }
            "title_block" => {
                p.title = child
                    .child("title")
                    .and_then(|t| t.arg_text(0))
                    .unwrap_or("")
                    .to_owned();
            }
            "lib_symbols" => {
                let mut defs = Vec::new();
                for s in child.children("symbol") {
                    if let Some(def) = parse_symbol(s, ctx) {
                        defs.push(def);
                    }
                }
                resolve_extends(&mut defs, ctx);
                for def in defs {
                    p.lib.insert(def.name.clone(), def);
                }
            }
            "symbol" => {
                if p.parts.len() >= max {
                    ctx.ignore("symbol (over max_items)");
                } else if let Some(part) = parse_part(child, ctx) {
                    p.parts.push(part);
                }
            }
            "wire" => {
                if p.wires.len() >= max {
                    ctx.ignore("wire (over max_items)");
                    continue;
                }
                let points: Vec<P> = child
                    .child("pts")
                    .map(|pts| pts.children("xy").filter_map(xy_args).collect())
                    .unwrap_or_default();
                if points.len() < 2 {
                    ctx.invalid(child, "wire", "fewer than two usable points");
                } else {
                    for pair in points.windows(2) {
                        p.wires.push((pair[0], pair[1]));
                    }
                }
            }
            "junction" => match child.child("at").and_then(at_args) {
                Some((at, _)) => p.junctions.push(at),
                None => ctx.invalid(child, "junction", "not two numbers"),
            },
            "label" | "global_label" | "hierarchical_label" => {
                if p.labels.len() >= max {
                    ctx.ignore("label (over max_items)");
                    continue;
                }
                let kind = match head {
                    "label" => LabelKind::Local,
                    "global_label" => LabelKind::Global,
                    _ => LabelKind::Hierarchical,
                };
                if let Some(label) = parse_label(child, kind, ctx) {
                    p.labels.push(label);
                }
            }
            "sheet" => {
                if let Some(sheet) = parse_sheet_ref(child, ctx) {
                    p.sheets.push(sheet);
                }
            }
            "symbol_instances" => {
                for path in child.children("path") {
                    let full = path.arg_text(0).unwrap_or("");
                    let last = full.rsplit('/').next().unwrap_or("");
                    if let Some(r) = path.child("reference").and_then(|r| r.arg_text(0)) {
                        p.references.insert(last.to_owned(), r.to_owned());
                    }
                }
            }
            "no_connect" => ctx.lose("no-connect flags", 1),
            other => ctx.ignore(other),
        }
    }
    p
}

struct PinRecord {
    pin: NetPin,
    node: usize,
    refdes: String,
    number: String,
}

type VariantKey = (String, u32, bool, bool);
type Group<'a> = Vec<(&'a RawPart, &'a SymbolDef)>;

struct Finished {
    schematic: Schematic,
    footprint_names: BTreeMap<Id, String>,
    sheet_files: Vec<String>,
    sheets_read: usize,
}

struct State {
    schematic: Schematic,
    parent: Vec<usize>,
    node_of: BTreeMap<(usize, i64, i64), usize>,
    names: Vec<(usize, u8, String)>,
    globals: BTreeMap<String, usize>,
    locals: BTreeMap<(usize, String), usize>,
    pins: Vec<PinRecord>,
    variants: BTreeMap<VariantKey, (Id, Vec<PinInfo>)>,
    references: BTreeMap<String, String>,
    footprint_names: BTreeMap<Id, String>,
    sheet_files: Vec<String>,
    open_files: Vec<String>,
    instances: usize,
    sheets_read: usize,
    labels: usize,
}

/// The position of a pin on the sheet: the position of the part plus the offset of the pin turned
/// by the part. The file's y axis points down, the symbol's up.
fn placed(at: P, quarters: i64, local: P) -> P {
    let (rx, ry) = turn_up(local, quarters);
    (at.0 + rx, at.1 - ry)
}

impl State {
    fn node(&mut self, instance: usize, p: P) -> usize {
        if let Some(n) = self.node_of.get(&(instance, p.0, p.1)) {
            return *n;
        }
        let n = self.parent.len();
        self.parent.push(n);
        self.node_of.insert((instance, p.0, p.1), n);
        n
    }

    fn find(&mut self, start: usize) -> usize {
        let mut root = start;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut x = start;
        while self.parent[x] != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (low, high) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[high] = low;
        }
    }

    fn join_global(&mut self, name: &str, node: usize) {
        let existing = self.globals.get(name).copied();
        match existing {
            Some(other) => self.union(node, other),
            None => {
                self.globals.insert(name.to_owned(), node);
            }
        }
    }

    fn join_local(&mut self, instance: usize, name: &str, node: usize) {
        let existing = self.locals.get(&(instance, name.to_owned())).copied();
        match existing {
            Some(other) => self.union(node, other),
            None => {
                self.locals.insert((instance, name.to_owned()), node);
            }
        }
    }

    fn reference_of(&self, part: &RawPart) -> String {
        let own = part.reference.trim();
        if own.is_empty() || own.ends_with('?') {
            if let Some(r) = &part.instance_reference {
                return r.clone();
            }
            if let Some(r) = self.references.get(&part.uuid) {
                return r.clone();
            }
        }
        own.to_owned()
    }

    /// Joins every point that lies in the middle of a horizontal or vertical wire to that wire.
    fn join_wires(&mut self, instance: usize, wires: &[(P, P, usize)], ctx: &mut Ctx) {
        let by_x: BTreeMap<(i64, i64), usize> = self
            .node_of
            .range((instance, i64::MIN, i64::MIN)..=(instance, i64::MAX, i64::MAX))
            .map(|(k, n)| ((k.1, k.2), *n))
            .collect();
        let by_y: BTreeMap<(i64, i64), usize> =
            by_x.iter().map(|(k, n)| ((k.1, k.0), *n)).collect();
        for (a, b, node) in wires {
            let found: Vec<usize> = if a.0 == b.0 {
                by_x.range((a.0, a.1.min(b.1))..=(a.0, a.1.max(b.1)))
                    .map(|(_, n)| *n)
                    .collect()
            } else if a.1 == b.1 {
                by_y.range((a.1, a.0.min(b.0))..=(a.1, a.0.max(b.0)))
                    .map(|(_, n)| *n)
                    .collect()
            } else {
                ctx.simplify(
                    "wire that is neither horizontal nor vertical",
                    "points in the middle of it are not joined to it",
                );
                Vec::new()
            };
            for n in found {
                self.union(*node, n);
            }
        }
    }

    fn add_single(
        &mut self,
        instance: usize,
        sheet: Id,
        part: &RawPart,
        def: &SymbolDef,
        refdes: &str,
        ctx: &mut Ctx,
    ) {
        let key: VariantKey = (part.lib_id.clone(), part.unit, part.mirror.0, part.mirror.1);
        if !self.variants.contains_key(&key) {
            let (symbol, infos) =
                build_symbol(def, &part.lib_id, Some(part.unit), part.mirror, ctx);
            let id = ctx.id();
            self.schematic.symbols.insert(id, symbol);
            self.variants.insert(key.clone(), (id, infos));
        }
        let Some((symbol_id, infos)) = self.variants.get(&key).cloned() else {
            return;
        };
        let part_id = ctx.id();
        let placement = match to_model(part.at) {
            Some(at) => Udeg::new(part.quarters * 90_000_000)
                .ok()
                .map(|rotation| Placement {
                    sheet,
                    at,
                    rotation,
                }),
            None => {
                ctx.push(KicadError::InvalidValue {
                    line: part.line as usize,
                    column: 0,
                    what: format!("position of {refdes}"),
                    reason: "outside the range of the model".to_owned(),
                });
                None
            }
        };
        self.schematic.parts.insert(
            part_id,
            Part {
                refdes: refdes.to_owned(),
                value: part.value.clone(),
                symbol: symbol_id,
                footprint: None,
                placement,
            },
        );
        self.note_footprint(part_id, part);
        for info in infos {
            let node = self.node(instance, placed(part.at, part.quarters, info.at));
            self.pins.push(PinRecord {
                pin: NetPin {
                    part: part_id,
                    pin: info.id,
                },
                node,
                refdes: refdes.to_owned(),
                number: info.number.clone(),
            });
            if let Some(name) = &info.power_net {
                self.names.push((node, 0, name.clone()));
                self.join_global(name, node);
            }
        }
    }

    fn note_footprint(&mut self, part_id: Id, part: &RawPart) {
        if !part.footprint.is_empty() {
            self.footprint_names.insert(part_id, part.footprint.clone());
        }
    }

    /// One part drawn as several units: a symbol with the pins of every unit, placed relative to the
    /// first unit.
    fn add_merged(
        &mut self,
        instance: usize,
        sheet: Id,
        parts: &[(&RawPart, &SymbolDef)],
        refdes: &str,
        ctx: &mut Ctx,
    ) {
        let Some((first, first_def)) = parts.first() else {
            return;
        };
        let mut pins = BTreeMap::new();
        let mut seen: Vec<(String, P)> = Vec::new();
        let part_id = ctx.id();
        let mut records: Vec<PinRecord> = Vec::new();
        let origin = first.at;
        for (part, def) in parts {
            let (_, infos) = build_symbol(def, &part.lib_id, Some(part.unit), part.mirror, ctx);
            for info in infos {
                let abs = placed(part.at, part.quarters, info.at);
                if seen.contains(&(info.number.clone(), abs)) {
                    continue;
                }
                seen.push((info.number.clone(), abs));
                let offset = (abs.0 - origin.0, -(abs.1 - origin.1));
                let pin_id = ctx.id();
                pins.insert(
                    pin_id,
                    Pin {
                        number: info.number.clone(),
                        at: Point::from_nm(offset.0, offset.1).ok(),
                    },
                );
                let node = self.node(instance, abs);
                records.push(PinRecord {
                    pin: NetPin {
                        part: part_id,
                        pin: pin_id,
                    },
                    node,
                    refdes: refdes.to_owned(),
                    number: info.number.clone(),
                });
                if let Some(name) = &info.power_net {
                    self.names.push((node, 0, name.clone()));
                    self.join_global(name, node);
                }
            }
        }
        let symbol_id = ctx.id();
        self.schematic.symbols.insert(
            symbol_id,
            Symbol {
                name: format!("{} (units merged)", first_def.name),
                pins,
            },
        );
        self.pins.extend(records);
        let placement = to_model(origin).map(|at| Placement {
            sheet,
            at,
            rotation: Udeg::ZERO,
        });
        self.schematic.parts.insert(
            part_id,
            Part {
                refdes: refdes.to_owned(),
                value: first.value.clone(),
                symbol: symbol_id,
                footprint: None,
                placement,
            },
        );
        self.note_footprint(part_id, first);
        ctx.simplify(
            "part drawn as several units",
            "one part whose symbol holds the pins of all its units",
        );
    }

    fn add_power(&mut self, instance: usize, part: &RawPart, def: &SymbolDef, ctx: &mut Ctx) {
        for pin in &def.pins {
            if pin.style > 1 || (pin.unit != 0 && pin.unit != part.unit) {
                continue;
            }
            let local = (
                if part.mirror.1 { -pin.at.0 } else { pin.at.0 },
                if part.mirror.0 { -pin.at.1 } else { pin.at.1 },
            );
            let node = self.node(instance, placed(part.at, part.quarters, local));
            let name = if part.value.is_empty() {
                pin.name.clone()
            } else {
                part.value.clone()
            };
            self.names.push((node, 0, name.clone()));
            self.join_global(&name, node);
        }
        ctx.transform("labels and power symbols (turned into net names)", 1);
    }
}

fn sheet_of(parsed: &Parsed, name: &str) -> Sheet {
    let (w, h) = parsed.paper.unwrap_or((297_000_000, 210_000_000));
    Sheet {
        name: name.to_owned(),
        width: Nm::new(w).unwrap_or(Nm::MAX),
        height: Nm::new(h).unwrap_or(Nm::MAX),
    }
}

fn qualified(path: &[String], text: &str) -> String {
    if path.is_empty() {
        format!("/{text}")
    } else {
        format!("/{}/{text}", path.join("/"))
    }
}

fn instantiate(
    st: &mut State,
    ctx: &mut Ctx,
    loader: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    parsed: &Parsed,
    path: &[String],
    parent_pins: &BTreeMap<String, usize>,
    depth: usize,
) {
    let instance = st.instances;
    st.instances += 1;
    st.sheets_read += 1;
    st.references.extend(
        parsed
            .references
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    let sheet_name = path.last().cloned().unwrap_or_else(|| {
        if parsed.title.is_empty() {
            "root".to_owned()
        } else {
            parsed.title.clone()
        }
    });
    let sheet_id = ctx.id();
    st.schematic
        .sheets
        .insert(sheet_id, sheet_of(parsed, &sheet_name));

    // wires
    let mut wire_nodes: Vec<(P, P, usize)> = Vec::new();
    for (a, b) in &parsed.wires {
        let na = st.node(instance, *a);
        let nb = st.node(instance, *b);
        st.union(na, nb);
        wire_nodes.push((*a, *b, na));
        if a == b {
            // a wire of no length joins nothing more than its point
            ctx.lose("wires of no length", 1);
        } else if let (Some(pa), Some(pb)) = (to_model(*a), to_model(*b)) {
            let id = ctx.id();
            st.schematic.wires.insert(
                id,
                Wire {
                    sheet: sheet_id,
                    path: vec![pa, pb],
                },
            );
        } else {
            ctx.push(KicadError::InvalidValue {
                line: 0,
                column: 0,
                what: "wire".to_owned(),
                reason: "a point is outside the range of the model".to_owned(),
            });
        }
    }
    for j in &parsed.junctions {
        st.node(instance, *j);
    }

    // labels
    for label in &parsed.labels {
        let node = st.node(instance, label.at);
        st.labels += 1;
        match label.kind {
            LabelKind::Global => {
                st.names.push((node, 0, label.text.clone()));
                st.join_global(&label.text, node);
            }
            LabelKind::Local => {
                st.names.push((node, 1, qualified(path, &label.text)));
                st.join_local(instance, &label.text, node);
            }
            LabelKind::Hierarchical => {
                st.names.push((node, 1, qualified(path, &label.text)));
                st.join_local(instance, &label.text, node);
                if let Some(up) = parent_pins.get(&label.text) {
                    st.union(node, *up);
                }
            }
        }
    }
    ctx.transform(
        "labels and power symbols (turned into net names)",
        parsed.labels.len(),
    );

    // sheet symbols
    let mut children: Vec<(&RawSheetRef, BTreeMap<String, usize>)> = Vec::new();
    for sheet in &parsed.sheets {
        let mut pins = BTreeMap::new();
        for (name, at) in &sheet.pins {
            let node = st.node(instance, *at);
            pins.insert(name.clone(), node);
        }
        children.push((sheet, pins));
    }

    // parts, in the order of the file, with the units of one reference together
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    for (index, part) in parsed.parts.iter().enumerate() {
        let Some(def) = parsed.lib.get(&part.lib_id) else {
            ctx.push(KicadError::MissingReference {
                what: format!("symbol `{}` is not in lib_symbols", part.lib_id),
                line: part.line as usize,
            });
            continue;
        };
        let refdes = st.reference_of(part);
        if def.power || refdes.starts_with('#') {
            st.add_power(instance, part, def, ctx);
            continue;
        }
        let key = if refdes.is_empty() || refdes.ends_with('?') {
            format!("?{index}:{refdes}")
        } else {
            refdes
        };
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push((part, def));
    }
    for key in &order {
        let Some(members) = groups.get(key) else {
            continue;
        };
        let refdes = key
            .split_once(':')
            .filter(|_| key.starts_with('?'))
            .map_or(key.as_str(), |(_, r)| r);
        if members.len() == 1 {
            let (part, def) = members[0];
            st.add_single(instance, sheet_id, part, def, refdes, ctx);
        } else {
            st.add_merged(instance, sheet_id, members, refdes, ctx);
        }
    }

    let wire_nodes_ref: &[(P, P, usize)] = &wire_nodes;
    st.join_wires(instance, wire_nodes_ref, ctx);

    // sub-sheets
    for (sheet, pins) in children {
        st.sheet_files.push(sheet.file.clone());
        read_child(st, ctx, loader, sheet, pins, path, depth);
    }
}

fn read_child(
    st: &mut State,
    ctx: &mut Ctx,
    loader: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    sheet: &RawSheetRef,
    pins: BTreeMap<String, usize>,
    path: &[String],
    depth: usize,
) {
    let line = sheet.line as usize;
    if depth + 1 > ctx.settings.max_sheet_depth {
        ctx.push(KicadError::Limit {
            what: "the nesting of sheets".to_owned(),
            value: depth as u64 + 1,
            limit: ctx.settings.max_sheet_depth as u64,
            line,
            column: 0,
        });
        return;
    }
    if st.sheets_read >= ctx.settings.max_sheets {
        ctx.push(KicadError::Limit {
            what: "the number of sheets".to_owned(),
            value: st.sheets_read as u64 + 1,
            limit: ctx.settings.max_sheets as u64,
            line,
            column: 0,
        });
        return;
    }
    if !is_safe_relative_name(&sheet.file) {
        ctx.push(KicadError::MissingReference {
            what: "a sheet file whose name leaves the folder of the schematic is not read"
                .to_owned(),
            line,
        });
        return;
    }
    if st.open_files.contains(&sheet.file) {
        ctx.push(KicadError::MissingReference {
            what: format!("sheet `{}` contains itself", sheet.file),
            line,
        });
        return;
    }
    let Some(bytes) = loader(&sheet.file) else {
        ctx.push(KicadError::MissingReference {
            what: format!("sheet file `{}` could not be read", sheet.file),
            line,
        });
        ctx.lose("sheets that could not be read", 1);
        return;
    };
    let (text, encoding) = match decode(&bytes, ctx.settings) {
        Ok(pair) => pair,
        Err(e) => {
            ctx.push(e);
            return;
        }
    };
    if let Some(note) = encoding {
        ctx.push(note);
    }
    let settings = ctx.settings;
    let root = match parse(&text, settings) {
        Ok(root) => root,
        Err(e) => {
            ctx.push(e);
            return;
        }
    };
    if let Err(e) = check_root(&root, &["kicad_sch"], "a schematic (kicad_sch)", ctx) {
        ctx.push(e);
        return;
    }
    let child = parse_sheet(&root, ctx);
    let mut child_path = path.to_vec();
    child_path.push(sheet.name.clone());
    st.open_files.push(sheet.file.clone());
    instantiate(st, ctx, loader, &child, &child_path, &pins, depth + 1);
    st.open_files.pop();
}

fn finish(mut st: State, ctx: &mut Ctx) -> Finished {
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..st.pins.len() {
        let node = st.pins[index].node;
        let root = st.find(node);
        by_root.entry(root).or_default().push(index);
    }
    let names = std::mem::take(&mut st.names);
    let mut best: BTreeMap<usize, (u8, String)> = BTreeMap::new();
    for (node, priority, text) in names {
        let root = st.find(node);
        let candidate = (priority, text);
        let replace = match best.get(&root) {
            Some(current) => candidate < *current,
            None => true,
        };
        if replace {
            best.insert(root, candidate);
        }
    }
    let mut nets = 0usize;
    for (root, indices) in &by_root {
        let name = best.get(root).map(|(_, text)| text.clone());
        if indices.len() < 2 && name.is_none() {
            continue;
        }
        let mut members: Vec<(&str, &str, NetPin)> = indices
            .iter()
            .map(|i| {
                let r = &st.pins[*i];
                (r.refdes.as_str(), r.number.as_str(), r.pin)
            })
            .collect();
        members.sort();
        let name = name.unwrap_or_else(|| match members.first() {
            Some((refdes, number, _)) => format!("Net-({refdes}-Pad{number})"),
            None => String::new(),
        });
        let mut pins: Vec<NetPin> = members.iter().map(|(_, _, pin)| *pin).collect();
        pins.sort();
        pins.dedup();
        let id = ctx.id();
        st.schematic.nets.insert(id, SchematicNet { name, pins });
        nets += 1;
    }
    ctx.keep("nets", nets);
    ctx.keep("parts", st.schematic.parts.len());
    ctx.keep("symbols", st.schematic.symbols.len());
    ctx.keep("wires", st.schematic.wires.len());
    ctx.keep("sheets", st.sheets_read);
    ctx.keep("footprint names", st.footprint_names.len());
    Finished {
        schematic: st.schematic,
        footprint_names: st.footprint_names,
        sheet_files: st.sheet_files,
        sheets_read: st.sheets_read,
    }
}

/// Reads a `.kicad_sch` file. The sheets it refers to are not read (use
/// [`read_schematic_with_sheets`] for that). Ids for the new objects come from `new_id`.
pub fn read_schematic(
    bytes: &[u8],
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<ReadSchematic, Diagnostic> {
    let mut none = |_: &str| -> Option<Vec<u8>> { None };
    read_schematic_with_sheets(bytes, settings, new_id, &mut none)
}

/// Reads a `.kicad_sch` file and, through `loader`, the sheets it refers to. The loader gets the
/// name of a sheet file as the schematic gives it (never an absolute name or one with `..`) and
/// returns its bytes, or `None` when it cannot.
pub fn read_schematic_with_sheets(
    bytes: &[u8],
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
    loader: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<ReadSchematic, Diagnostic> {
    let fail = |e: KicadError| e.diagnostic();
    let (text, encoding) = decode(bytes, settings).map_err(fail)?;
    let root = parse(&text, settings).map_err(fail)?;
    let mut ctx = Ctx::new(settings, new_id);
    if let Some(note) = encoding {
        ctx.push(note);
    }
    let (version, generator) =
        check_root(&root, &["kicad_sch"], "a schematic (kicad_sch)", &mut ctx).map_err(fail)?;
    let parsed = parse_sheet(&root, &mut ctx);
    let mut st = State {
        schematic: Schematic {
            name: parsed.title.clone(),
            ..Schematic::default()
        },
        parent: Vec::new(),
        node_of: BTreeMap::new(),
        names: Vec::new(),
        globals: BTreeMap::new(),
        locals: BTreeMap::new(),
        pins: Vec::new(),
        variants: BTreeMap::new(),
        references: BTreeMap::new(),
        footprint_names: BTreeMap::new(),
        sheet_files: Vec::new(),
        open_files: Vec::new(),
        instances: 0,
        sheets_read: 0,
        labels: 0,
    };
    instantiate(&mut st, &mut ctx, loader, &parsed, &[], &BTreeMap::new(), 0);
    let labels = st.labels;
    let done = finish(st, &mut ctx);
    ctx.keep("labels", labels);
    ctx.push(KicadError::Derived {
        detail: "the nets were computed from wires, junctions, labels and power symbols; the file stores no net list"
            .to_owned(),
    });
    let (diagnostics, quality) = ctx.finish();
    Ok(ReadSchematic {
        schematic: done.schematic,
        footprint_names: done.footprint_names,
        sheet_files: done.sheet_files,
        sheets_read: done.sheets_read,
        version,
        generator,
        quality,
        diagnostics,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const LIBS: &str = r##"(lib_symbols
    (symbol "Device:R" (pin_numbers hide) (pin_names (offset 0)) (in_bom yes) (on_board yes)
      (property "Reference" "R" (id 0) (at 2.032 0 90) (effects (font (size 1.27 1.27))))
      (property "Value" "R" (id 1) (at 0 0 90) (effects (font (size 1.27 1.27))))
      (symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
      (symbol "R_1_1"
        (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
        (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
    (symbol "power:GND" (power) (pin_names (offset 0)) (in_bom no) (on_board yes)
      (property "Reference" "#PWR" (id 0) (at 0 -6.35 0) (effects (font (size 1.27 1.27)) hide))
      (property "Value" "GND" (id 1) (at 0 -3.81 0) (effects (font (size 1.27 1.27))))
      (symbol "GND_0_1" (polyline (pts (xy 0 0) (xy 0 -1.27) (xy 1.27 -1.27) (xy 0 -2.54) (xy -1.27 -1.27) (xy 0 -1.27)) (stroke (width 0) (type default)) (fill (type none))))
      (symbol "GND_1_1" (pin power_in line (at 0 0 270) (length 0) hide (name "GND" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))
    (symbol "Device:Dual" (in_bom yes) (on_board yes)
      (property "Reference" "U" (id 0) (at 0 0 0) (effects (font (size 1.27 1.27))))
      (symbol "Dual_1_1" (pin input line (at -2.54 0 0) (length 2.54) (name "A" (effects (font (size 1 1)))) (number "1" (effects (font (size 1 1))))))
      (symbol "Dual_2_1" (pin input line (at -2.54 0 0) (length 2.54) (name "B" (effects (font (size 1 1)))) (number "2" (effects (font (size 1 1)))))))
  )"##;

    fn placed_symbol(lib: &str, reference: &str, value: &str, at: &str, extra: &str) -> String {
        format!(
            r#"(symbol (lib_id "{lib}") (at {at}) {extra} (in_bom yes) (on_board yes) (uuid "u-{reference}")
    (property "Reference" "{reference}" (id 0) (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Value" "{value}" (id 1) (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Resistor_SMD:R_0603_1608Metric" (id 2) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))
    (pin "1" (uuid "p1")) (pin "2" (uuid "p2")))"#
        )
    }

    /// A KiCad 6 sheet: two resistors joined by a label on a wire, two power symbols, a dual unit
    /// part, a junction, a no-connect flag, a bus and a sheet symbol with one pin.
    pub(crate) fn root_v6() -> String {
        format!(
            r#"(kicad_sch (version 20211123) (generator eeschema)
  (uuid "00000000-0000-0000-0000-000000000001")
  (paper "A4")
  (title_block (title "Demo"))
  {LIBS}
  (junction (at 120 46.19) (diameter 0) (color 0 0 0 0) (uuid "j1"))
  (no_connect (at 10 10) (uuid "nc1"))
  (wire (pts (xy 100 46.19) (xy 120 46.19)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "w1"))
  (wire (pts (xy 120 46.19) (xy 150 46.19)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "w2"))
  (wire (pts (xy 100 53.81) (xy 100 60)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "w3"))
  (wire (pts (xy 120 53.81) (xy 120 60)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "w4"))
  (wire (pts (xy 57.46 30) (xy 57.46 40)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "w5"))
  (bus (pts (xy 0 0) (xy 5 0)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "b1"))
  (label "SIG" (at 110 46.19 0) (fields_autoplaced) (effects (font (size 1.27 1.27)) (justify left bottom)) (uuid "l1"))
  (label "X" (at 57.46 40 0) (effects (font (size 1.27 1.27))) (uuid "l2"))
  {r1}
  {r2}
  {g1}
  {g2}
  {u1a}
  {u1b}
  (sheet (at 150 40) (size 20 10) (fields_autoplaced) (stroke (width 0.1524) (type solid)) (fill (color 0 0 0 0)) (uuid "s1")
    (property "Sheet name" "Sub" (id 0) (at 150 39 0) (effects (font (size 1.27 1.27)) (justify left bottom)))
    (property "Sheet file" "sub.kicad_sch" (id 1) (at 150 51 0) (effects (font (size 1.27 1.27)) (justify left top)))
    (pin "IN" input (at 150 46.19 180) (effects (font (size 1.27 1.27)) (justify left)) (uuid "sp1")))
  (sheet_instances (path "/" (page "1")))
  (symbol_instances (path "/u-R1" (reference "R1") (unit 1) (value "10k") (footprint ""))))"#,
            r1 = placed_symbol("Device:R", "R1", "10k", "100 50 0", "(unit 1)"),
            r2 = placed_symbol("Device:R", "R2", "4k7", "120 50 0", "(unit 1)"),
            g1 = placed_symbol("power:GND", "#PWR01", "GND", "100 60 0", "(unit 1)"),
            g2 = placed_symbol("power:GND", "#PWR02", "GND", "120 60 0", "(unit 1)"),
            u1a = placed_symbol("Device:Dual", "U1", "LM", "30 30 0", "(unit 1)"),
            u1b = placed_symbol("Device:Dual", "U1", "LM", "60 30 0", "(unit 2)"),
        )
    }

    /// The sheet the root refers to: one resistor under a hierarchical label.
    pub(crate) fn child_v6() -> String {
        format!(
            r#"(kicad_sch (version 20211123) (generator eeschema) (uuid "c") (paper "A4")
  {LIBS}
  (hierarchical_label "IN" (shape input) (at 50 46.19 0) (effects (font (size 1.27 1.27))) (uuid "h1"))
  (wire (pts (xy 50 53.81) (xy 50 60)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid "cw"))
  {r3}
  {g3})"#,
            r3 = placed_symbol("Device:R", "R3", "1k", "50 50 0", "(unit 1)"),
            g3 = placed_symbol("power:GND", "#PWR03", "GND", "50 60 0", "(unit 1)"),
        )
    }

    fn read(text: &str) -> ReadSchematic {
        let mut ids = crate::sequential_ids().unwrap();
        read_schematic(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap()
    }

    fn read_with_child(text: &str, child: Option<String>) -> ReadSchematic {
        let mut ids = crate::sequential_ids().unwrap();
        let mut loader = |name: &str| -> Option<Vec<u8>> {
            if name == "sub.kicad_sch" {
                child.as_ref().map(|c| c.clone().into_bytes())
            } else {
                None
            }
        };
        read_schematic_with_sheets(
            text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
            &mut loader,
        )
        .unwrap()
    }

    fn net_of<'a>(s: &'a Schematic, name: &str) -> Vec<(&'a str, &'a str)> {
        let net = s.nets.values().find(|n| n.name == name).unwrap();
        let mut members: Vec<(&str, &str)> = net
            .pins
            .iter()
            .map(|p| (s.parts[&p.part].refdes.as_str(), s.pin_number(*p).unwrap()))
            .collect();
        members.sort();
        members
    }

    #[test]
    fn one_sheet_gives_parts_wires_and_the_nets_computed_from_the_drawing() {
        let r = read(&root_v6());
        let s = &r.schematic;
        assert_eq!(s.name, "Demo");
        assert_eq!(r.version, "20211123");
        assert_eq!(r.generator, "eeschema");
        // R1, R2 and one part for the two units of U1; the power symbols are not parts
        let mut refdes: Vec<&str> = s.parts.values().map(|p| p.refdes.as_str()).collect();
        refdes.sort_unstable();
        assert_eq!(refdes, vec!["R1", "R2", "U1"]);
        assert_eq!(s.wires.len(), 5);
        assert_eq!(s.sheets.len(), 1);
        // the label is on the wire, so the top pins and the label are one net
        assert_eq!(net_of(s, "/SIG"), vec![("R1", "1"), ("R2", "1")]);
        // the two power symbols join the bottom pins by name
        assert_eq!(net_of(s, "GND"), vec![("R1", "2"), ("R2", "2")]);
        // a label with a single pin is kept because it has a name
        assert_eq!(net_of(s, "/X"), vec![("U1", "2")]);
        assert_eq!(r.sheet_files, vec!["sub.kicad_sch".to_owned()]);
        assert_eq!(r.sheets_read, 1);
    }

    #[test]
    fn the_schematic_read_is_consistent_for_the_model() {
        let r = read(&root_v6());
        let problems: Vec<String> = r
            .schematic
            .validate()
            .iter()
            .filter(|d| d.severity == eh3d_diagnostic::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn placed_parts_and_pins_are_in_the_orientation_of_the_model() {
        let r = read(&root_v6());
        let s = &r.schematic;
        let r1 = s.parts.values().find(|p| p.refdes == "R1").unwrap();
        let placement = r1.placement.unwrap();
        // y is turned: the part at y = 50 in the file is at y = -50
        assert_eq!(placement.at.x_nm.get(), 100_000_000);
        assert_eq!(placement.at.y_nm.get(), -50_000_000);
        assert_eq!(placement.rotation.get(), 0);
        let pins: Vec<(&str, i64)> = s.symbols[&r1.symbol]
            .pins
            .values()
            .map(|p| (p.number.as_str(), p.at.unwrap().y_nm.get()))
            .collect();
        assert!(pins.contains(&("1", 3_810_000)));
        assert!(pins.contains(&("2", -3_810_000)));
        // the units of U1 are one part with both pins, the second placed relative to the first
        let u1 = s.parts.values().find(|p| p.refdes == "U1").unwrap();
        let mut offsets: Vec<(String, i64, i64)> = s.symbols[&u1.symbol]
            .pins
            .values()
            .map(|p| {
                let at = p.at.unwrap();
                (p.number.clone(), at.x_nm.get(), at.y_nm.get())
            })
            .collect();
        offsets.sort();
        assert_eq!(
            offsets,
            vec![
                ("1".to_owned(), -2_540_000, 0),
                ("2".to_owned(), 27_460_000, 0)
            ]
        );
        assert_eq!(s.symbols[&u1.symbol].name, "Device:Dual (units merged)");
    }

    #[test]
    fn the_footprint_names_and_the_quality_report_are_kept() {
        let r = read(&root_v6());
        assert_eq!(r.footprint_names.len(), 3);
        let q = |list: &[eh3d_phoenix_hub::QualityItem], name: &str| {
            list.iter().find(|i| i.category == name).map(|i| i.count)
        };
        assert_eq!(q(&r.quality.kept, "parts"), Some(3));
        assert_eq!(q(&r.quality.kept, "nets"), Some(3));
        assert_eq!(q(&r.quality.dropped, "no-connect flags"), Some(1));
        assert_eq!(q(&r.quality.dropped, "`bus` elements"), Some(1));
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-013")));
        // the sheet could not be read without a loader: it is only listed
        assert!(r.sheet_files.contains(&"sub.kicad_sch".to_owned()));
    }

    #[test]
    fn a_hierarchical_label_joins_the_net_of_the_sheet_pin_above() {
        let r = read_with_child(&root_v6(), Some(child_v6()));
        let s = &r.schematic;
        assert_eq!(r.sheets_read, 2);
        assert_eq!(s.sheets.len(), 2);
        assert_eq!(
            net_of(s, "/SIG"),
            vec![("R1", "1"), ("R2", "1"), ("R3", "1")]
        );
        // the power symbol of the child is global
        assert_eq!(
            net_of(s, "GND"),
            vec![("R1", "2"), ("R2", "2"), ("R3", "2")]
        );
        assert!(s.sheets.values().any(|sheet| sheet.name == "Sub"));
    }

    #[test]
    fn a_missing_or_damaged_sheet_is_reported_and_the_rest_is_read() {
        let r = read_with_child(&root_v6(), None);
        assert_eq!(r.sheets_read, 1);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-008")));
        let r = read_with_child(
            &root_v6(),
            Some("(kicad_sch (version 20211123) (wire".to_owned()),
        );
        assert_eq!(r.sheets_read, 1);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-004")));
        let r = read_with_child(
            &root_v6(),
            Some("(kicad_pcb (version 20211014))".to_owned()),
        );
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-003")));
        assert_eq!(net_of(&r.schematic, "/SIG").len(), 2);
    }

    #[test]
    fn a_sheet_that_leaves_the_folder_or_contains_itself_is_not_followed() {
        let text = root_v6().replace("sub.kicad_sch", "../outside.kicad_sch");
        let mut ids = crate::sequential_ids().unwrap();
        let mut asked: Vec<String> = Vec::new();
        let mut loader = |name: &str| -> Option<Vec<u8>> {
            asked.push(name.to_owned());
            None
        };
        let r = read_schematic_with_sheets(
            text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
            &mut loader,
        )
        .unwrap();
        assert!(asked.is_empty());
        assert_eq!(r.sheets_read, 1);
        // a sheet that names itself stops at the first level
        let looping = child_v6().replace(
            "(hierarchical_label",
            r#"(sheet (at 0 0) (size 1 1) (property "Sheet name" "Again" (id 0) (at 0 0 0)) (property "Sheet file" "sub.kicad_sch" (id 1) (at 0 0 0)))
  (hierarchical_label"#,
        );
        let r = read_with_child(&root_v6(), Some(looping));
        assert_eq!(r.sheets_read, 2);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.message.contains("contains itself")));
    }

    #[test]
    fn the_depth_and_the_number_of_sheets_follow_the_settings() {
        let mut ids = crate::sequential_ids().unwrap();
        let settings = KicadSettings {
            max_sheet_depth: 1,
            ..KicadSettings::default()
        };
        let looping = child_v6().replace(
            "(hierarchical_label",
            r#"(sheet (at 0 0) (size 1 1) (property "Sheet name" "Deeper" (id 0) (at 0 0 0)) (property "Sheet file" "deeper.kicad_sch" (id 1) (at 0 0 0)))
  (hierarchical_label"#,
        );
        let mut loader = |name: &str| -> Option<Vec<u8>> {
            match name {
                "sub.kicad_sch" => Some(looping.clone().into_bytes()),
                "deeper.kicad_sch" => Some(child_v6().into_bytes()),
                _ => None,
            }
        };
        let r = read_schematic_with_sheets(root_v6().as_bytes(), &settings, &mut ids, &mut loader)
            .unwrap();
        assert_eq!(r.sheets_read, 2);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-002")));
        let one = KicadSettings {
            max_sheets: 1,
            ..KicadSettings::default()
        };
        let mut loader = |_: &str| -> Option<Vec<u8>> { Some(child_v6().into_bytes()) };
        let r =
            read_schematic_with_sheets(root_v6().as_bytes(), &one, &mut ids, &mut loader).unwrap();
        assert_eq!(r.sheets_read, 1);
    }

    #[test]
    fn a_mirrored_symbol_gets_a_mirrored_copy_and_a_turned_one_moves_its_pins() {
        let text = format!(
            r#"(kicad_sch (version 20230121) (generator eeschema) (paper "A4")
  {LIBS}
  (wire (pts (xy 100 53.81) (xy 90 53.81)) (stroke (width 0) (type default)) (uuid "w"))
  (label "A" (at 90 53.81 0) (effects (font (size 1.27 1.27))) (uuid "l"))
  (wire (pts (xy 150 46.19) (xy 140 46.19)) (stroke (width 0) (type default)) (uuid "w2"))
  (label "B" (at 140 46.19 0) (effects (font (size 1.27 1.27))) (uuid "l2"))
  {a}
  {b}
  {c})"#,
            a = placed_symbol("Device:R", "R1", "1k", "100 50 0", "(mirror x)"),
            b = placed_symbol("Device:R", "R2", "1k", "100 50 0", ""),
            c = placed_symbol("Device:R", "R3", "1k", "146.19 46.19 90", ""),
        );
        // R1 is mirrored about the x axis, so its pin 1 is at the bottom, at (100, 53.81), where the
        // wire joins it to the label A. R3 is turned by 90 degrees, so its two pins are on the
        // horizontal wire of the label B
        let r = read(&text);
        let s = &r.schematic;
        let symbols: Vec<&str> = s.symbols.values().map(|x| x.name.as_str()).collect();
        assert_eq!(symbols.iter().filter(|n| **n == "Device:R").count(), 2);
        let a = net_of(s, "/A");
        assert_eq!(a, vec![("R1", "1"), ("R2", "2")]);
        assert_eq!(net_of(s, "/B"), vec![("R3", "1"), ("R3", "2")]);
        let rot = s.parts.values().find(|p| p.refdes == "R3").unwrap();
        assert_eq!(rot.placement.unwrap().rotation.get(), 90_000_000);
    }

    #[test]
    fn rotations_that_are_not_right_angles_are_rounded_and_reported() {
        let text = format!(
            r#"(kicad_sch (version 20230121) (generator eeschema) (paper "A4")
  {LIBS}
  {a})"#,
            a = placed_symbol("Device:R", "R1", "1k", "10 10 80", ""),
        );
        let r = read(&text);
        let p = r.schematic.parts.values().next().unwrap();
        assert_eq!(p.placement.unwrap().rotation.get(), 90_000_000);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-007")));
    }

    #[test]
    fn kicad_8_instances_give_the_reference_of_an_unannotated_property() {
        let text = format!(
            r#"(kicad_sch (version 20231120) (generator "eeschema") (generator_version "8.0") (paper "A4")
  {LIBS}
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp no) (uuid "x")
    (property "Reference" "R?" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Value" "2k2" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (instances (project "demo" (path "/00000000" (reference "R9") (unit 1)))))
  (sheet_instances (path "/" (page "1"))))"#
        );
        let r = read(&text);
        assert_eq!(r.schematic.parts.values().next().unwrap().refdes, "R9");
        assert!(r
            .diagnostics
            .iter()
            .all(|d| d.code.as_deref() != Some("PHOENIX-KICAD-009")));
    }

    #[test]
    fn a_symbol_that_is_not_in_the_library_is_reported() {
        let text = r#"(kicad_sch (version 20211123) (generator eeschema) (paper "A4")
  (lib_symbols)
  (symbol (lib_id "Nope:X") (at 1 1 0) (unit 1) (uuid "x") (property "Reference" "X1" (id 0) (at 0 0 0))))"#;
        let r = read(text);
        assert!(r.schematic.parts.is_empty());
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-008") && d.line == 3));
    }

    #[test]
    fn damaged_schematics_are_refused_or_read_with_notes_and_never_panic() {
        let mut ids = crate::sequential_ids().unwrap();
        let s = KicadSettings::default();
        let text = root_v6();
        let e = read_schematic(text[..text.len() / 2].as_bytes(), &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-004"));
        let e = read_schematic(b"(kicad_pcb (version 20211014))", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-003"));
        let bad_numbers = r#"(kicad_sch (version 20211123) (generator eeschema)
  (wire (pts (xy a 1) (xy 2 2)))
  (wire (pts (xy 1 1)))
  (junction (at x y))
  (label "L")
  (label (at 1 1 0))
  (sheet (property "Sheet name" "S")))"#;
        let r = read_schematic(bad_numbers.as_bytes(), &s, &mut ids).unwrap();
        let bad = r
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("PHOENIX-KICAD-005"))
            .count();
        assert_eq!(bad, 6);
        // every cut of a valid file is an error or a schematic, never a panic
        for cut in (0..text.len()).step_by(97) {
            if text.is_char_boundary(cut) {
                let _ = read_schematic(text[..cut].as_bytes(), &s, &mut ids);
            }
        }
    }

    #[test]
    fn an_unconnected_pin_alone_has_no_net_and_unnamed_nets_are_named_after_a_pin() {
        let text = format!(
            r#"(kicad_sch (version 20230121) (generator eeschema) (paper "A4")
  {LIBS}
  (wire (pts (xy 100 46.19) (xy 120 46.19)) (stroke (width 0) (type default)) (uuid "w"))
  {a}
  {b})"#,
            a = placed_symbol("Device:R", "R1", "1k", "100 50 0", ""),
            b = placed_symbol("Device:R", "R2", "1k", "120 50 0", ""),
        );
        let r = read(&text);
        let s = &r.schematic;
        assert_eq!(s.nets.len(), 1);
        assert_eq!(s.nets.values().next().unwrap().name, "Net-(R1-Pad1)");
    }
}
