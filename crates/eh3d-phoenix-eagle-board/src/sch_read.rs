//! Reading an EAGLE schematic file into a schematic, and the signature that proves the round trip.
//!
//! Read: the symbols (with their pins and positions) and the device sets of the embedded library,
//! the parts, the sheets with their instances (the placement of each part), and the nets with their
//! pin references and wires. A device is linked to a footprint of the board by the name of its
//! package, through the map the caller gives. Sheets get the name `Sheet N` and the size of an A4
//! sheet, because the file does not keep them. Everything else is counted by element name in a
//! `PHOENIX-EAGLE-022` note.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::Id;
use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::{
    NetPin, Part, Pin, Placement, Schematic, SchematicNet, Sheet, Symbol, Wire,
};

use crate::codes::{self, NOT_READ, UNREADABLE};
use crate::reader::{nm, note, point, rotation};

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
pub fn read_schematic(
    text: &str,
    footprints: &BTreeMap<String, Id>,
    new_id: &mut dyn FnMut() -> Id,
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

    // sheets
    let mut nets: BTreeMap<String, Id> = BTreeMap::new();
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
            if let Some(list) = sheet.child("nets") {
                for net in list.children_named("net") {
                    let name = net.attr("name").unwrap_or("").to_owned();
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
        }
    }
    for net in s.nets.values_mut() {
        net.pins.sort();
        net.pins.dedup();
    }
    for other in ["plain", "busses"] {
        for sheet in element
            .child("sheets")
            .map(|l| l.children_named("sheet"))
            .into_iter()
            .flatten()
        {
            let count = sheet.child(other).map_or(0, |e| e.children.len());
            for _ in 0..count {
                note(other, &mut notes);
            }
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
