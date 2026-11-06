//! Reading an EAGLE schematic file into a schematic, and the signature that proves the round trip.
//!
//! Read: the symbols (with their pins and positions) and the device sets of the embedded library,
//! the parts, the sheets with their instances (the placement of each part), and the nets with their
//! pin references and wires. A device is linked to a footprint of the board by the name of its
//! package, through the map the caller gives. Sheets get the name `Sheet N` and the size of an A4
//! sheet, because the file does not keep them. Everything else is counted by element name in a
//! `PHOENIX-EAGLE-022` note.
//!
//! A module of a hierarchical design is read as a sheet that sheet symbols show: each `moduleinst`
//! becomes a sheet symbol with a pin for each port of the module, and the module has the ports
//! as ports of its sheet. The parts and the nets inside a module are not read; they are counted.
//!
//! A `bus` becomes a bus of its sheet for each of its vectors (`D[0..7]`, or the vectors of a name
//! with several, `A[0..3],B[0..3]`, or of a named one, `BUS:A[0..3]`), anchored where its first
//! wire starts and expanded to its members. A net of the sheet that has the name of a member and
//! an end on a wire of the bus becomes an entry of that bus at that end. What the model does not
//! have is counted: the wires and labels of the bus, the names in a bus that are not vectors and
//! the buses that cannot be expanded or have no wire.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::Id;
use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::{
    expand_spec, Bus, BusEntry, HierPin, NetPin, Part, Pin, PinKind, Placement, PortDir, Schematic,
    SchematicNet, Sheet, SheetBlock, SheetPort, Symbol, Wire,
};

use crate::codes::{self, NOT_READ, UNREADABLE};
use crate::limits::EagleLimits;
use crate::reader::{nm, note, point, rotation};

/// The size of the symbol of a module whose file gives none, in nanometres (20 mm).
const MODULE_SIZE_NM: i64 = 20_000_000;

/// A port of a module: where it is on the edge of the module symbol.
struct ModulePort {
    name: String,
    dir: PortDir,
    side: String,
    coord: i64,
}

/// A module of the file: the sheet that stands for it, the size of its symbol and its ports.
struct ModuleDef {
    sheet: Id,
    size: (i64, i64),
    ports: Vec<ModulePort>,
}

/// A schematic read from a file and what was not read.
#[derive(Debug, Clone)]
pub struct ReadSchematic {
    /// The schematic. Its name is empty: the file does not carry one.
    pub schematic: Schematic,
    /// Parts of the file that were not read.
    pub diagnostics: Vec<Diagnostic>,
}

/// Reads an EAGLE schematic file. The new objects get their ids from `new_id`; `footprints` maps
/// the name of a package to the id of the footprint of the board that it stands for.
/// The limits are the default ones, see [`EagleLimits`].
pub fn read_schematic(
    text: &str,
    footprints: &BTreeMap<String, Id>,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<ReadSchematic, Diagnostic> {
    read_schematic_with_limits(text, footprints, new_id, &EagleLimits::default())
}

/// Like [`read_schematic`] with the limits given.
pub fn read_schematic_with_limits(
    text: &str,
    footprints: &BTreeMap<String, Id>,
    new_id: &mut dyn FnMut() -> Id,
    limits: &EagleLimits,
) -> Result<ReadSchematic, Diagnostic> {
    let unreadable = |why: &str| codes::diagnostic(UNREADABLE, Severity::Error, why);
    let root = parse(text, &Limits::default()).map_err(|e| unreadable(&e.to_string()))?;
    if root.name != "eagle" {
        return Err(unreadable("the root element is not eagle"));
    }
    let element = root
        .child("drawing")
        .and_then(|d| d.child("schematic"))
        .ok_or_else(|| unreadable("there is no schematic in the drawing"))?;

    let mut s = Schematic::default();
    let mut notes: BTreeMap<String, usize> = BTreeMap::new();

    // symbols: name -> (id, pin number -> pin id)
    let mut symbols: BTreeMap<String, (Id, BTreeMap<String, Id>)> = BTreeMap::new();
    // device sets: name -> (symbol name, device name -> package name)
    let mut sets: BTreeMap<String, (String, BTreeMap<String, String>)> = BTreeMap::new();
    let library = element.child("libraries").and_then(|l| l.child("library"));
    if let Some(library) = library {
        if let Some(list) = library.child("symbols") {
            for symbol in list.children_named("symbol") {
                let name = symbol.attr("name").unwrap_or("").to_owned();
                let mut pins = BTreeMap::new();
                let mut numbers = BTreeMap::new();
                for pin in symbol.children_named("pin") {
                    let number = pin.attr("name").unwrap_or("").to_owned();
                    let id = new_id();
                    if let Some(kind) = pin.attr("direction").and_then(pin_kind) {
                        s.pin_kinds.insert(id, kind);
                    }
                    pins.insert(
                        id,
                        Pin {
                            number: number.clone(),
                            at: point(pin, "x", "y"),
                        },
                    );
                    numbers.insert(number, id);
                }
                let id = new_id();
                s.symbols.insert(
                    id,
                    Symbol {
                        name: name.clone(),
                        pins,
                    },
                );
                symbols.insert(name, (id, numbers));
            }
        }
        if let Some(list) = library.child("devicesets") {
            for set in list.children_named("deviceset") {
                let name = set.attr("name").unwrap_or("").to_owned();
                let symbol = set
                    .child("gates")
                    .and_then(|g| g.child("gate"))
                    .and_then(|g| g.attr("symbol"))
                    .unwrap_or("")
                    .to_owned();
                let mut devices = BTreeMap::new();
                if let Some(list) = set.child("devices") {
                    for device in list.children_named("device") {
                        devices.insert(
                            device.attr("name").unwrap_or("").to_owned(),
                            device.attr("package").unwrap_or("").to_owned(),
                        );
                    }
                }
                sets.insert(name, (symbol, devices));
            }
        }
    }

    // parts
    let mut parts: BTreeMap<String, Id> = BTreeMap::new();
    if let Some(list) = element.child("parts") {
        for part in list.children_named("part") {
            let refdes = part.attr("name").unwrap_or("").to_owned();
            let set = sets.get(part.attr("deviceset").unwrap_or(""));
            let symbol = set.and_then(|(sym, _)| symbols.get(sym));
            let Some((symbol, _)) = symbol else {
                note("part", &mut notes);
                continue;
            };
            let footprint = set
                .and_then(|(_, devices)| devices.get(part.attr("device").unwrap_or("")))
                .and_then(|package| footprints.get(package))
                .copied();
            let id = new_id();
            s.parts.insert(
                id,
                Part {
                    refdes: refdes.clone(),
                    value: part.attr("value").unwrap_or("").to_owned(),
                    symbol: *symbol,
                    footprint,
                    placement: None,
                },
            );
            parts.insert(refdes, id);
        }
    }

    // modules
    let mut modules: BTreeMap<String, ModuleDef> = BTreeMap::new();
    if let Some(list) = element.child("modules") {
        for module in list.children_named("module") {
            read_module(module, &mut s, &mut modules, new_id, &mut notes);
        }
    }

    // sheets
    let mut nets: BTreeMap<String, Id> = BTreeMap::new();
    let mut net_wires: Vec<(String, Vec<(Point, Point)>)> = Vec::new();
    let mut bus_budget = limits.max_bus_checks;
    if let Some(list) = element.child("sheets") {
        for (index, sheet) in list.children_named("sheet").enumerate() {
            let sheet_id = new_id();
            s.sheets.insert(
                sheet_id,
                Sheet {
                    name: format!("Sheet {}", index + 1),
                    width: Nm::new(297_000_000).expect("in range"),
                    height: Nm::new(210_000_000).expect("in range"),
                },
            );
            if let Some(list) = sheet.child("instances") {
                for instance in list.children_named("instance") {
                    let placed = instance
                        .attr("part")
                        .and_then(|p| parts.get(p))
                        .zip(point(instance, "x", "y"));
                    match placed {
                        Some((part, at)) => {
                            let (rot, mirrored) = rotation(instance);
                            if mirrored {
                                note("mirrored instance", &mut notes);
                            }
                            if let Some(p) = s.parts.get_mut(part) {
                                p.placement = Some(Placement {
                                    sheet: sheet_id,
                                    at,
                                    rotation: rot,
                                });
                            }
                        }
                        None => note("instance", &mut notes),
                    }
                }
            }
            if let Some(list) = sheet.child("moduleinsts") {
                for inst in list.children_named("moduleinst") {
                    add_module_symbol(inst, sheet_id, &modules, &mut s, new_id, &mut notes);
                }
            }
            net_wires.clear();
            if let Some(list) = sheet.child("nets") {
                for net in list.children_named("net") {
                    let name = net.attr("name").unwrap_or("").to_owned();
                    net_wires.push((name.clone(), wires_of(net)));
                    let net_id = *nets.entry(name.clone()).or_insert_with(|| {
                        let id = new_id();
                        s.nets.insert(
                            id,
                            SchematicNet {
                                name,
                                pins: Vec::new(),
                            },
                        );
                        id
                    });
                    for segment in net.children_named("segment") {
                        read_segment(
                            segment, sheet_id, net_id, &parts, &symbols, &mut s, new_id, &mut notes,
                        );
                    }
                }
            }
            if let Some(list) = sheet.child("busses") {
                for bus in list.children_named("bus") {
                    read_bus(
                        bus,
                        sheet_id,
                        &net_wires,
                        &mut bus_budget,
                        &mut s,
                        new_id,
                        &mut notes,
                    );
                }
            }
        }
    }
    for net in s.nets.values_mut() {
        net.pins.sort();
        net.pins.dedup();
    }
    for sheet in element
        .child("sheets")
        .map(|l| l.children_named("sheet"))
        .into_iter()
        .flatten()
    {
        let count = sheet.child("plain").map_or(0, |e| e.children.len());
        for _ in 0..count {
            note("plain", &mut notes);
        }
    }
    let diagnostics = notes
        .iter()
        .map(|(name, count)| {
            codes::diagnostic(
                NOT_READ,
                Severity::Info,
                &format!("{count} {name} element(s)"),
            )
        })
        .collect();
    Ok(ReadSchematic {
        schematic: s,
        diagnostics,
    })
}

/// The wires of the segments of a net or a bus, as pairs of end points.
fn wires_of(element: &Element) -> Vec<(Point, Point)> {
    let mut out = Vec::new();
    for segment in element.children_named("segment") {
        for wire in segment.children_named("wire") {
            if let (Some(a), Some(b)) = (point(wire, "x1", "y1"), point(wire, "x2", "y2")) {
                out.push((a, b));
            }
        }
    }
    out
}

/// Whether `p` lies on the straight piece from `a` to `b`, ends included.
fn on_piece(p: Point, a: Point, b: Point) -> bool {
    let (px, py) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
    let (ax, ay) = (i128::from(a.x_nm.get()), i128::from(a.y_nm.get()));
    let (bx, by) = (i128::from(b.x_nm.get()), i128::from(b.y_nm.get()));
    let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
    cross == 0 && px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by)
}

/// Reads a `bus` of a sheet: a bus of the model for each vector in its name, and an entry for each
/// net of the sheet with the name of a member that ends on a wire of the bus.
fn read_bus(
    bus: &Element,
    sheet: Id,
    net_wires: &[(String, Vec<(Point, Point)>)],
    budget: &mut u64,
    s: &mut Schematic,
    new_id: &mut dyn FnMut() -> Id,
    notes: &mut BTreeMap<String, usize>,
) {
    let wires = wires_of(bus);
    for segment in bus.children_named("segment") {
        for item in &segment.children {
            match item.name.as_str() {
                "wire" => note("bus wire", notes),
                "label" => note("bus label", notes),
                other => note(other, notes),
            }
        }
    }
    let Some(anchor) = wires.first().map(|(a, _)| *a) else {
        note("bus without a wire", notes);
        return;
    };
    // a name with a colon names the bus itself; the members follow it
    let full = bus.attr("name").unwrap_or("");
    let list = full.split_once(':').map_or(full, |(_, rest)| rest);
    let none = BTreeMap::new();
    for text in list.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if !text.contains('[') {
            note("bus name that is not a vector", notes);
            continue;
        }
        let Ok(members) = expand_spec(text, &none) else {
            note("bus name that cannot be expanded", notes);
            continue;
        };
        let id = new_id();
        s.buses.insert(
            id,
            Bus {
                sheet,
                at: anchor,
                spec: text.to_owned(),
            },
        );
        let member_set: BTreeSet<&str> = members.iter().map(String::as_str).collect();
        for (name, ends) in net_wires {
            if !member_set.contains(name.as_str()) {
                continue;
            }
            let cost = (ends.len() as u64)
                .saturating_mul(2)
                .saturating_mul(wires.len() as u64);
            if cost > *budget {
                *budget = 0;
                note("bus entry (over max_bus_checks)", notes);
                continue;
            }
            *budget -= cost;
            let end = ends
                .iter()
                .flat_map(|(a, b)| [*a, *b])
                .find(|p| wires.iter().any(|(a, b)| on_piece(*p, *a, *b)));
            if let Some(at) = end {
                s.bus_entries.insert(
                    new_id(),
                    BusEntry {
                        bus: id,
                        at,
                        member: name.clone(),
                    },
                );
            }
        }
    }
}

/// The direction of a port of a module (`in`, `out`, `io`, anything else is passive).
fn port_dir(direction: &str) -> PortDir {
    match direction {
        "in" => PortDir::Input,
        "out" => PortDir::Output,
        "io" => PortDir::Bidirectional,
        _ => PortDir::Passive,
    }
}

/// Where a port is on a module symbol whose lower left corner is `origin`: on the edge it names,
/// `coord` along it.
fn port_at(origin: Point, size: (i64, i64), side: &str, coord: i64) -> Option<Point> {
    let (x, y) = (origin.x_nm.get(), origin.y_nm.get());
    let (px, py) = match side {
        "left" => (x, y.checked_add(coord)?),
        "right" => (x.checked_add(size.0)?, y.checked_add(coord)?),
        "top" => (x.checked_add(coord)?, y.checked_add(size.1)?),
        _ => (x.checked_add(coord)?, y),
    };
    Point::from_nm(px, py).ok()
}

/// Reads a module as a sheet with the ports of the module as its ports.
fn read_module(
    module: &Element,
    s: &mut Schematic,
    modules: &mut BTreeMap<String, ModuleDef>,
    new_id: &mut dyn FnMut() -> Id,
    notes: &mut BTreeMap<String, usize>,
) {
    let name = module.attr("name").unwrap_or("").to_owned();
    if name.is_empty() || modules.contains_key(&name) {
        note("module", notes);
        return;
    }
    if module.child("parts").is_some() || module.child("sheets").is_some() {
        note("module contents", notes);
    }
    let size = point(module, "dx", "dy").map_or((MODULE_SIZE_NM, MODULE_SIZE_NM), |p| {
        (p.x_nm.get().abs().max(1), p.y_nm.get().abs().max(1))
    });
    let sheet = new_id();
    s.sheets.insert(
        sheet,
        Sheet {
            name: name.clone(),
            width: Nm::new(297_000_000).expect("in range"),
            height: Nm::new(210_000_000).expect("in range"),
        },
    );
    let mut ports = Vec::new();
    if let Some(list) = module.child("ports") {
        for port in list.children_named("port") {
            let port_name = port.attr("name").unwrap_or("").to_owned();
            if port_name.is_empty() {
                note("port", notes);
                continue;
            }
            ports.push(ModulePort {
                name: port_name,
                dir: port_dir(port.attr("direction").unwrap_or("")),
                side: port.attr("side").unwrap_or("").to_owned(),
                coord: nm(port, "coord").map_or(0, |n| n.get()),
            });
        }
    }
    let origin = Point::from_nm(0, 0).expect("in range");
    for port in &ports {
        if s.sheet_ports
            .values()
            .any(|p| p.sheet == sheet && p.name == port.name)
        {
            note("port", notes);
            continue;
        }
        if let Some(at) = port_at(origin, size, &port.side, port.coord) {
            s.sheet_ports.insert(
                new_id(),
                SheetPort {
                    sheet,
                    name: port.name.clone(),
                    dir: port.dir,
                    at,
                },
            );
        }
    }
    modules.insert(name, ModuleDef { sheet, size, ports });
}

/// Draws the sheet symbol of a `moduleinst`: a block on the sheet that shows the sheet of the
/// module, with one pin for each port of the module.
fn add_module_symbol(
    inst: &Element,
    sheet: Id,
    modules: &BTreeMap<String, ModuleDef>,
    s: &mut Schematic,
    new_id: &mut dyn FnMut() -> Id,
    notes: &mut BTreeMap<String, usize>,
) {
    let found = inst
        .attr("module")
        .and_then(|m| modules.get(m))
        .zip(point(inst, "x", "y"));
    let Some((def, at)) = found else {
        note("moduleinst", notes);
        return;
    };
    let (Ok(width), Ok(height)) = (Nm::new(def.size.0), Nm::new(def.size.1)) else {
        note("moduleinst", notes);
        return;
    };
    let base = inst.attr("name").unwrap_or("").trim().replace('/', "_");
    let base = if base.is_empty() {
        "M".to_owned()
    } else {
        base
    };
    let mut name = base.clone();
    let mut n = 1usize;
    while s
        .sheet_blocks
        .values()
        .any(|b| b.sheet == sheet && b.name == name)
    {
        n += 1;
        name = format!("{base}_{n}");
    }
    let block = new_id();
    s.sheet_blocks.insert(
        block,
        SheetBlock {
            sheet,
            child: def.sheet,
            name,
            at,
            width,
            height,
        },
    );
    for port in &def.ports {
        if let Some(pin_at) = port_at(at, def.size, &port.side, port.coord) {
            s.hier_pins.insert(
                new_id(),
                HierPin {
                    block,
                    name: port.name.clone(),
                    dir: port.dir,
                    at: pin_at,
                },
            );
        }
    }
}

/// The electrical type of a pin from the `direction` of an EAGLE pin (`in`, `out`, `io`, `pas`,
/// `pwr`, `sup`, `oc`, `hiz`, `nc`); `None` for a direction this reader does not know.
fn pin_kind(direction: &str) -> Option<PinKind> {
    match direction {
        "in" => Some(PinKind::Input),
        "out" => Some(PinKind::Output),
        "io" => Some(PinKind::Bidirectional),
        "pas" => Some(PinKind::Passive),
        "pwr" => Some(PinKind::PowerIn),
        "sup" => Some(PinKind::PowerOut),
        "oc" => Some(PinKind::OpenCollector),
        "hiz" => Some(PinKind::TriState),
        "nc" => Some(PinKind::NotConnected),
        _ => None,
    }
}

/// Ends the run of wire pieces: two or more points make a wire.
fn flush_run(run: &mut Vec<Point>, sheet: Id, s: &mut Schematic, new_id: &mut dyn FnMut() -> Id) {
    if run.len() >= 2 {
        s.wires.insert(
            new_id(),
            Wire {
                sheet,
                path: std::mem::take(run),
            },
        );
    }
    run.clear();
}

#[allow(clippy::too_many_arguments)]
fn read_segment(
    segment: &Element,
    sheet: Id,
    net: Id,
    parts: &BTreeMap<String, Id>,
    symbols: &BTreeMap<String, (Id, BTreeMap<String, Id>)>,
    s: &mut Schematic,
    new_id: &mut dyn FnMut() -> Id,
    notes: &mut BTreeMap<String, usize>,
) {
    let mut run: Vec<Point> = Vec::new();
    for item in &segment.children {
        match item.name.as_str() {
            "pinref" => {
                let found = item
                    .attr("part")
                    .and_then(|p| parts.get(p))
                    .and_then(|pid| {
                        let part = s.parts.get(pid)?;
                        let (_, numbers) = symbols.values().find(|(id, _)| *id == part.symbol)?;
                        Some(NetPin {
                            part: *pid,
                            pin: *numbers.get(item.attr("pin")?)?,
                        })
                    });
                match found {
                    Some(pin) => {
                        if let Some(n) = s.nets.get_mut(&net) {
                            n.pins.push(pin);
                        }
                    }
                    None => note("pinref", notes),
                }
            }
            "wire" => {
                let (Some(a), Some(b)) = (point(item, "x1", "y1"), point(item, "x2", "y2")) else {
                    note("wire", notes);
                    continue;
                };
                if run.last() != Some(&a) {
                    flush_run(&mut run, sheet, s, new_id);
                    run.push(a);
                }
                run.push(b);
            }
            other => note(other, notes),
        }
    }
    flush_run(&mut run, sheet, s, new_id);
}

/// A description of a schematic without ids: sorted lines for the symbols, parts, nets and wire
/// pieces. Sheets appear by their order. The names and sizes of sheets and the grouping of wire
/// pieces into wires do not appear, because the file does not keep them.
pub fn schematic_signature(s: &Schematic) -> Vec<String> {
    let pt = |p: Point| format!("{},{}", p.x_nm.get(), p.y_nm.get());
    let sheet_index = |id: &Id| {
        s.sheets
            .keys()
            .position(|k| k == id)
            .map_or(-1, |i| i as i64)
    };
    let mut lines = Vec::new();
    for symbol in s.symbols.values() {
        for pin in symbol.pins.values() {
            lines.push(format!(
                "symbol {} pin {} at {}",
                symbol.name,
                pin.number,
                pin.at.map_or("none".to_owned(), pt)
            ));
        }
    }
    for part in s.parts.values() {
        let symbol = s.symbols.get(&part.symbol).map_or("?", |x| x.name.as_str());
        let placed = part.placement.map_or("unplaced".to_owned(), |p| {
            format!(
                "sheet {} at {} rot {}",
                sheet_index(&p.sheet),
                pt(p.at),
                p.rotation.normalised().get()
            )
        });
        lines.push(format!(
            "part {} value {} symbol {} footprint {} {}",
            part.refdes,
            part.value,
            symbol,
            part.footprint.map_or("none".to_owned(), |f| f.to_string()),
            placed
        ));
    }
    for net in s.nets.values() {
        let mut pins: Vec<String> = net
            .pins
            .iter()
            .filter_map(|p| {
                let part = s.parts.get(&p.part)?;
                Some(format!("{}.{}", part.refdes, s.pin_number(*p)?))
            })
            .collect();
        pins.sort();
        lines.push(format!("net {} pins {}", net.name, pins.join(",")));
    }
    for wire in s.wires.values() {
        for w in wire.path.windows(2) {
            lines.push(format!(
                "wire sheet {} {} {}",
                sheet_index(&wire.sheet),
                pt(w[0]),
                pt(w[1])
            ));
        }
    }
    lines.sort();
    lines
}

#[cfg(test)]
mod direction_tests {
    use super::*;
    use eh3d_phoenix_model::ObjectId;

    #[test]
    fn a_module_and_its_instances_become_a_sheet_and_sheet_symbols() {
        let text = r#"<eagle version="9.6.2"><drawing><schematic>
            <modules>
              <module name="FILTER" dx="20" dy="10">
                <ports>
                  <port name="IN" side="left" coord="5" direction="in"/>
                  <port name="OUT" side="right" coord="5" direction="out"/>
                </ports>
                <parts/>
                <sheets><sheet/></sheets>
              </module>
            </modules>
            <sheets><sheet>
              <moduleinsts>
                <moduleinst name="F1" module="FILTER" x="30" y="40"/>
                <moduleinst name="F2" module="FILTER" x="70" y="40"/>
                <moduleinst name="F3" module="MISSING" x="0" y="0"/>
              </moduleinsts>
            </sheet></sheets></schematic></drawing></eagle>"#;
        let mut n: u16 = 0;
        let mut new_id = || {
            n += 1;
            ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
        };
        let read = read_schematic(text, &BTreeMap::new(), &mut new_id).unwrap();
        let s = read.schematic;
        // the module is a sheet next to the one sheet of the file
        assert_eq!(s.sheets.len(), 2);
        let module = s.sheets.iter().find(|(_, x)| x.name == "FILTER").unwrap().0;
        assert_eq!(s.sheet_ports.len(), 2);
        assert!(s.sheet_ports.values().all(|p| p.sheet == *module));
        // two symbols show it, with a pin for each port at the edge of the symbol
        assert_eq!(s.sheet_blocks.len(), 2);
        let mut names: Vec<&str> = s.sheet_blocks.values().map(|b| b.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["F1", "F2"]);
        assert!(s.sheet_blocks.values().all(|b| b.child == *module));
        assert_eq!(s.hier_pins.len(), 4);
        let (f1, block) = s.sheet_blocks.iter().find(|(_, b)| b.name == "F1").unwrap();
        let out = s
            .hier_pins
            .values()
            .find(|p| p.block == *f1 && p.name == "OUT")
            .unwrap();
        assert_eq!(
            (out.at.x_nm.get(), out.at.y_nm.get()),
            (50_000_000, 45_000_000)
        );
        assert_eq!(out.dir, PortDir::Output);
        assert_eq!(block.width.get(), 20_000_000);
        // the instance of a module the file does not define, and what is inside a module, are counted
        let counted: Vec<&str> = read
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        assert!(
            counted.iter().any(|m| m.contains("moduleinst")),
            "{counted:?}"
        );
        assert!(
            counted.iter().any(|m| m.contains("module contents")),
            "{counted:?}"
        );
        // the checks of the hierarchy find nothing wrong with what was read
        let problems = s.hier_problems(&eh3d_phoenix_schematic::HierConfig::default());
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_direction_of_an_eagle_pin_becomes_its_electrical_type() {
        let text = r#"<eagle version="9.6.2"><drawing><schematic><libraries><library name="l"><symbols>
            <symbol name="S">
              <pin name="1" x="0" y="0" direction="in"/>
              <pin name="2" x="0" y="0" direction="pwr"/>
              <pin name="3" direction="nc"/>
              <pin name="4" direction="sup"/>
              <pin name="5" direction="weird"/>
              <pin name="6"/>
              <pin name="7" direction="oc"/>
              <pin name="8" direction="hiz"/>
              <pin name="9" direction="io"/>
              <pin name="10" direction="out"/>
              <pin name="11" direction="pas"/>
            </symbol></symbols></library></libraries></schematic></drawing></eagle>"#;
        let mut n: u16 = 0;
        let mut new_id = || {
            n += 1;
            ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
        };
        let read = read_schematic(text, &BTreeMap::new(), &mut new_id).unwrap();
        let s = read.schematic;
        let symbol = s.symbols.values().next().unwrap();
        let mut kinds: Vec<(String, PinKind)> = symbol
            .pins
            .iter()
            .filter_map(|(id, pin)| Some((pin.number.clone(), *s.pin_kinds.get(id)?)))
            .collect();
        kinds.sort();
        assert_eq!(
            kinds,
            [
                ("1".to_owned(), PinKind::Input),
                ("10".to_owned(), PinKind::Output),
                ("11".to_owned(), PinKind::Passive),
                ("2".to_owned(), PinKind::PowerIn),
                ("3".to_owned(), PinKind::NotConnected),
                ("4".to_owned(), PinKind::PowerOut),
                ("7".to_owned(), PinKind::OpenCollector),
                ("8".to_owned(), PinKind::TriState),
                ("9".to_owned(), PinKind::Bidirectional),
            ]
        );
        // a pin with an unknown direction or none has no type, and nothing else is invented
        assert_eq!(symbol.pins.len(), 11);
        assert_eq!(s.pin_kinds.len(), 9);
    }

    const BUS_FILE: &str = r#"<eagle version="9.6.2"><drawing><schematic><sheets><sheet>
        <busses>
          <bus name="D[0..3],CLK"><segment>
            <wire x1="0" y1="0" x2="0" y2="20" width="0.762" layer="92"/>
            <label x="0" y="21" size="1.778" layer="95"/>
          </segment></bus>
          <bus name="X:A[0..1]"><segment>
            <wire x1="50" y1="0" x2="50" y2="10" width="0.762" layer="92"/>
          </segment></bus>
          <bus name="[0..3]"><segment>
            <wire x1="80" y1="0" x2="80" y2="1" width="0.762" layer="92"/>
          </segment></bus>
          <bus name="EMPTY[0..1]"><segment/></bus>
        </busses>
        <nets>
          <net name="D1" class="0"><segment>
            <wire x1="0" y1="5" x2="10" y2="5" width="0.1524" layer="91"/>
          </segment></net>
          <net name="D2" class="0"><segment>
            <wire x1="10" y1="10" x2="0" y2="10" width="0.1524" layer="91"/>
          </segment></net>
          <net name="D7" class="0"><segment>
            <wire x1="0" y1="15" x2="10" y2="15" width="0.1524" layer="91"/>
          </segment></net>
          <net name="D3" class="0"><segment>
            <wire x1="3" y1="5" x2="10" y2="6" width="0.1524" layer="91"/>
          </segment></net>
          <net name="A0" class="0"><segment>
            <wire x1="50" y1="2" x2="60" y2="2" width="0.1524" layer="91"/>
          </segment></net>
        </nets>
        </sheet></sheets></schematic></drawing></eagle>"#;

    fn read_buses() -> ReadSchematic {
        let mut n: u16 = 0;
        let mut new_id = || {
            n += 1;
            ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
        };
        read_schematic(BUS_FILE, &BTreeMap::new(), &mut new_id).unwrap()
    }

    #[test]
    fn the_vectors_of_a_bus_become_buses_with_their_members_and_the_nets_that_end_on_them_entries()
    {
        let read = read_buses();
        let s = &read.schematic;
        let mut specs: Vec<&str> = s.buses.values().map(|b| b.spec.as_str()).collect();
        specs.sort_unstable();
        assert_eq!(specs, ["A[0..1]", "D[0..3]"]);
        let d = *s.buses.iter().find(|(_, b)| b.spec == "D[0..3]").unwrap().0;
        assert_eq!(s.bus_member_names(d).unwrap(), ["D0", "D1", "D2", "D3"]);
        // the anchor is where the first wire starts: (0, 0) and (50, 0) in millimetres
        let anchor = s.buses[&d].at;
        assert_eq!((anchor.x_nm.get(), anchor.y_nm.get()), (0, 0));
        // D1 ends on the bus at its first end, D2 at its second end; D3 and D7 do not touch it
        // (D7 is not a member in any case) and A0 ends on the second bus
        let mut entries: Vec<(String, i64, i64)> = s
            .bus_entries
            .values()
            .map(|e| (e.member.clone(), e.at.x_nm.get(), e.at.y_nm.get()))
            .collect();
        entries.sort();
        assert_eq!(
            entries,
            [
                ("A0".to_owned(), 50_000_000, 2_000_000),
                ("D1".to_owned(), 0, 5_000_000),
                ("D2".to_owned(), 0, 10_000_000),
            ]
        );
        assert!(s.bus_entries.values().all(|e| s.buses.contains_key(&e.bus)));
        // the nets stay what they were
        assert_eq!(s.nets.len(), 5);
    }

    fn entries_with(limits: &EagleLimits) -> (Vec<String>, Vec<String>) {
        let mut n: u16 = 0;
        let mut new_id = || {
            n += 1;
            ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
        };
        let read =
            read_schematic_with_limits(BUS_FILE, &BTreeMap::new(), &mut new_id, limits).unwrap();
        let mut members: Vec<String> = read
            .schematic
            .bus_entries
            .values()
            .map(|e| e.member.clone())
            .collect();
        members.sort();
        let messages = read.diagnostics.into_iter().map(|d| d.message).collect();
        (members, messages)
    }

    #[test]
    fn bus_entries_over_the_check_budget_are_counted_and_not_read() {
        // the bus D[0..3] has one wire; D1, D2 and D3 are members and each has one wire with two
        // ends, so each costs 2 * 1 * 1 = 2 tests: 6 in all; A0 on the second bus costs 2 more
        let limits = |max_bus_checks| EagleLimits {
            max_bus_checks,
            ..EagleLimits::default()
        };
        let (members, _) = entries_with(&limits(8));
        assert_eq!(members, ["A0", "D1", "D2"]);
        let (members, messages) = entries_with(&limits(7));
        assert_eq!(members, ["D1", "D2"]);
        assert!(messages
            .iter()
            .any(|m| m.contains("1 bus entry (over max_bus_checks) element(s)")));
        // 3 tests: D1 is read (2), D2, D3 and A0 are left over
        let (members, messages) = entries_with(&limits(3));
        assert_eq!(members, ["D1"]);
        assert!(messages
            .iter()
            .any(|m| m.contains("3 bus entry (over max_bus_checks) element(s)")));
    }

    #[test]
    fn what_a_bus_cannot_say_is_counted_and_nothing_is_dropped_silently() {
        let read = read_buses();
        let messages: Vec<&str> = read
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        for expected in [
            "3 bus wire element(s)",
            "1 bus label element(s)",
            "1 bus name that is not a vector element(s)",
            "1 bus name that cannot be expanded element(s)",
            "1 bus without a wire element(s)",
        ] {
            assert!(
                messages.iter().any(|m| m.contains(expected)),
                "{expected}: {messages:?}"
            );
        }
        // the buses of the sheet are not counted as left out any more
        assert!(!messages.iter().any(|m| m.contains("busses")));
    }
}
