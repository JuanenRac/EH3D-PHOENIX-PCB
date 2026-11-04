//! Reading an EAGLE XML board file into a board.
//!
//! Only what the model holds is read: layers, the board outline, packages with pads, elements,
//! signals with contacts, wires, vias and polygons, and the wires drawn on silkscreen layers.
//! Everything else is counted by element name in a `PHOENIX-EAGLE-022` note and not interpreted.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::codes::{self, ITEM_SIMPLIFIED, NOT_READ, UNREADABLE};
use crate::layers::meaning;
use crate::writer::LIBRARY;
use crate::{parse_nm, parse_udeg};

/// A board read from a file and what was not read.
#[derive(Debug, Clone)]
pub struct ReadBoard {
    /// The board. Its name is empty: the file does not carry one.
    pub board: Board,
    /// Parts not read and shapes read in a simpler form.
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn nm(e: &Element, key: &str) -> Option<Nm> {
    parse_nm(e.attr(key)?)
}

pub(crate) fn point(e: &Element, kx: &str, ky: &str) -> Option<Point> {
    Some(Point {
        x_nm: nm(e, kx)?,
        y_nm: nm(e, ky)?,
    })
}

/// The rotation of an element or pad (`R90`, `MR45.5`) and whether it is mirrored.
pub(crate) fn rotation(e: &Element) -> (Udeg, bool) {
    let Some(text) = e.attr("rot") else {
        return (Udeg::ZERO, false);
    };
    let mut rest = text.trim();
    let mut mirrored = false;
    if let Some(r) = rest.strip_prefix('S') {
        rest = r;
    }
    if let Some(r) = rest.strip_prefix('M') {
        mirrored = true;
        rest = r;
    }
    let rest = rest.strip_prefix('R').unwrap_or(rest);
    (parse_udeg(rest).unwrap_or(Udeg::ZERO), mirrored)
}

/// Counts an element that is not read.
pub(crate) fn note(name: &str, notes: &mut BTreeMap<String, usize>) {
    *notes.entry(name.to_owned()).or_insert(0) += 1;
}

struct Wire {
    layer: u32,
    width: Nm,
    segment: Segment,
}

fn wire_of(e: &Element) -> Option<Wire> {
    let start = point(e, "x1", "y1")?;
    let end = point(e, "x2", "y2")?;
    let segment = match e.attr("curve").and_then(parse_udeg) {
        Some(sweep) if sweep.get() != 0 => Segment::Arc { start, end, sweep },
        _ => Segment::Line { start, end },
    };
    Some(Wire {
        layer: e.attr("layer")?.trim().parse().ok()?,
        width: nm(e, "width")?,
        segment,
    })
}

/// Joins wires that follow each other on one layer with one width, end to start, into runs.
fn runs(wires: Vec<Wire>) -> Vec<(u32, Nm, Vec<Segment>)> {
    let mut out: Vec<(u32, Nm, Vec<Segment>)> = Vec::new();
    for w in wires {
        if let Some((layer, width, path)) = out.last_mut() {
            if *layer == w.layer
                && *width == w.width
                && path.last().map(Segment::end) == Some(w.segment.start())
            {
                path.push(w.segment);
                continue;
            }
        }
        out.push((w.layer, w.width, vec![w.segment]));
    }
    out
}

/// Reads an EAGLE XML board file. The new objects get their ids from `new_id`.
pub fn read_board(text: &str, new_id: &mut dyn FnMut() -> Id) -> Result<ReadBoard, Diagnostic> {
    let unreadable = |why: &str| codes::diagnostic(UNREADABLE, Severity::Error, why);
    let root = parse(text, &Limits::default()).map_err(|e| unreadable(&e.to_string()))?;
    if root.name != "eagle" {
        return Err(unreadable("the root element is not eagle"));
    }
    let board_element = root
        .child("drawing")
        .and_then(|d| d.child("board"))
        .ok_or_else(|| unreadable("there is no board in the drawing"))?;
    let drawing = root.child("drawing").expect("checked above");

    let mut board = Board::default();
    let mut notes: BTreeMap<String, usize> = BTreeMap::new();
    let mut diagnostics = Vec::new();

    // layers
    let mut layer_ids: BTreeMap<u32, Id> = BTreeMap::new();
    if let Some(layers) = drawing.child("layers") {
        for l in layers.children_named("layer") {
            let Some(number) = l.attr("number").and_then(|n| n.trim().parse::<u32>().ok()) else {
                note("layer", &mut notes);
                continue;
            };
            let Some((kind, side)) = meaning(number) else {
                note("layer", &mut notes);
                continue;
            };
            let id = new_id();
            board.layers.insert(
                id,
                Layer {
                    name: l.attr("name").unwrap_or("").to_owned(),
                    kind,
                    side,
                },
            );
            layer_ids.insert(number, id);
        }
    }
    board.copper_stack = layer_ids
        .iter()
        .filter(|(n, _)| (1..=16).contains(*n))
        .map(|(_, id)| *id)
        .collect();
    let copper_ids = board.copper_stack.clone();

    // packages
    let mut footprints: BTreeMap<(String, String), Id> = BTreeMap::new();
    if let Some(libraries) = board_element.child("libraries") {
        for library in libraries.children_named("library") {
            let lib_name = library.attr("name").unwrap_or("").to_owned();
            let Some(packages) = library.child("packages") else {
                continue;
            };
            for package in packages.children_named("package") {
                let mut pads = BTreeMap::new();
                for item in &package.children {
                    let number = item.attr("name").unwrap_or("").to_owned();
                    let (rot, _) = rotation(item);
                    let Some(at) = point(item, "x", "y") else {
                        note(&item.name, &mut notes);
                        continue;
                    };
                    match item.name.as_str() {
                        "pad" => {
                            let (Some(drill), Some(d)) = (nm(item, "drill"), nm(item, "diameter"))
                            else {
                                note("pad", &mut notes);
                                continue;
                            };
                            let shape = match item.attr("shape").unwrap_or("round") {
                                "square" => PadShape::Rect {
                                    width: d,
                                    height: d,
                                },
                                other => {
                                    if other != "round" {
                                        diagnostics.push(codes::diagnostic(
                                            ITEM_SIMPLIFIED,
                                            Severity::Info,
                                            &format!(
                                                "pad {number}: the shape {other} is read as round"
                                            ),
                                        ));
                                    }
                                    PadShape::Circle { diameter: d }
                                }
                            };
                            pads.insert(
                                new_id(),
                                Pad {
                                    number,
                                    shape,
                                    at,
                                    rotation: rot,
                                    drill: Some(drill),
                                    layers: copper_ids.clone(),
                                },
                            );
                        }
                        "smd" => {
                            let (Some(dx), Some(dy)) = (nm(item, "dx"), nm(item, "dy")) else {
                                note("smd", &mut notes);
                                continue;
                            };
                            let roundness = item.attr("roundness").unwrap_or("0").trim();
                            let shape = match roundness {
                                "0" => PadShape::Rect {
                                    width: dx,
                                    height: dy,
                                },
                                "100" if dx == dy => PadShape::Circle { diameter: dx },
                                "100" => PadShape::Oblong {
                                    width: dx,
                                    height: dy,
                                },
                                other => {
                                    diagnostics.push(codes::diagnostic(
                                        ITEM_SIMPLIFIED,
                                        Severity::Info,
                                        &format!("pad {number}: the roundness {other} is read as a rectangle"),
                                    ));
                                    PadShape::Rect {
                                        width: dx,
                                        height: dy,
                                    }
                                }
                            };
                            let layers = item
                                .attr("layer")
                                .and_then(|n| n.trim().parse::<u32>().ok())
                                .and_then(|n| layer_ids.get(&n))
                                .map(|id| vec![*id])
                                .unwrap_or_default();
                            pads.insert(
                                new_id(),
                                Pad {
                                    number,
                                    shape,
                                    at,
                                    rotation: rot,
                                    drill: None,
                                    layers,
                                },
                            );
                        }
                        other => note(other, &mut notes),
                    }
                }
                let id = new_id();
                let name = package.attr("name").unwrap_or("").to_owned();
                board.footprints.insert(
                    id,
                    Footprint {
                        name: name.clone(),
                        pads,
                    },
                );
                footprints.entry((lib_name.clone(), name)).or_insert(id);
            }
        }
    }

    // elements
    let mut components: BTreeMap<String, Id> = BTreeMap::new();
    if let Some(elements) = board_element.child("elements") {
        for e in elements.children_named("element") {
            let key = (
                e.attr("library").unwrap_or(LIBRARY).to_owned(),
                e.attr("package").unwrap_or("").to_owned(),
            );
            let (Some(footprint), Some(at), Some(refdes)) =
                (footprints.get(&key), point(e, "x", "y"), e.attr("name"))
            else {
                note("element", &mut notes);
                continue;
            };
            let (rot, mirrored) = rotation(e);
            let id = new_id();
            board.components.insert(
                id,
                Component {
                    refdes: refdes.to_owned(),
                    value: e.attr("value").unwrap_or("").to_owned(),
                    symbol: None,
                    footprint: *footprint,
                    at,
                    rotation: rot,
                    side: if mirrored { Side::Bottom } else { Side::Top },
                },
            );
            components.insert(refdes.to_owned(), id);
        }
    }

    // signals
    if let Some(signals) = board_element.child("signals") {
        for signal in signals.children_named("signal") {
            let net = new_id();
            board.nets.insert(
                net,
                Net {
                    name: signal.attr("name").unwrap_or("").to_owned(),
                },
            );
            let mut wires = Vec::new();
            for item in &signal.children {
                match item.name.as_str() {
                    "contactref" => {
                        let found =
                            components
                                .get(item.attr("element").unwrap_or(""))
                                .and_then(|cid| {
                                    let c = &board.components[cid];
                                    let wanted = item.attr("pad")?;
                                    let (pid, _) = board.footprints[&c.footprint]
                                        .pads
                                        .iter()
                                        .find(|(_, p)| p.number == wanted)?;
                                    Some((*cid, *pid))
                                });
                        match found {
                            Some((component, pad)) => board.pad_nets.push(PadNet {
                                component,
                                pad,
                                net,
                            }),
                            None => note("contactref", &mut notes),
                        }
                    }
                    "wire" => match wire_of(item) {
                        Some(w) => wires.push(w),
                        None => note("wire", &mut notes),
                    },
                    "via" => {
                        let ends = item.attr("extent").and_then(|x| {
                            let (a, b) = x.split_once('-')?;
                            Some((
                                *layer_ids.get(&a.trim().parse::<u32>().ok()?)?,
                                *layer_ids.get(&b.trim().parse::<u32>().ok()?)?,
                            ))
                        });
                        match (
                            point(item, "x", "y"),
                            ends,
                            nm(item, "drill"),
                            nm(item, "diameter"),
                        ) {
                            (
                                Some(at),
                                Some((from_layer, to_layer)),
                                Some(drill),
                                Some(diameter),
                            ) => {
                                board.vias.insert(
                                    new_id(),
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
                            _ => note("via", &mut notes),
                        }
                    }
                    "polygon" => {
                        let layer = item
                            .attr("layer")
                            .and_then(|n| n.trim().parse::<u32>().ok())
                            .and_then(|n| layer_ids.get(&n));
                        let outline: Option<Vec<Point>> = item
                            .children_named("vertex")
                            .map(|v| point(v, "x", "y"))
                            .collect();
                        match (layer, outline) {
                            (Some(layer), Some(outline)) => {
                                board.zones.insert(
                                    new_id(),
                                    Zone {
                                        net,
                                        layer: *layer,
                                        outline,
                                        clearance: nm(item, "isolate").unwrap_or(Nm::ZERO),
                                    },
                                );
                            }
                            _ => note("polygon", &mut notes),
                        }
                    }
                    other => note(other, &mut notes),
                }
            }
            for (layer, width, path) in runs(wires) {
                match layer_ids.get(&layer) {
                    Some(layer) => {
                        board.tracks.insert(
                            new_id(),
                            Track {
                                net,
                                layer: *layer,
                                width,
                                path,
                            },
                        );
                    }
                    None => note("wire", &mut notes),
                }
            }
        }
    }
    board.pad_nets.sort_by_key(|p| p.pad_ref());

    // the plain drawing: the outline and the silkscreen
    if let Some(plain) = board_element.child("plain") {
        let mut outline_wires = Vec::new();
        let mut silk_wires = Vec::new();
        for item in &plain.children {
            if item.name != "wire" {
                note(&item.name, &mut notes);
                continue;
            }
            match wire_of(item) {
                Some(w) if w.layer == 20 => outline_wires.push(w),
                Some(w) if meaning(w.layer).is_some_and(|(k, _)| k == LayerKind::Silkscreen) => {
                    silk_wires.push(w)
                }
                _ => note("wire", &mut notes),
            }
        }
        let ring = !outline_wires.is_empty()
            && outline_wires.iter().enumerate().all(|(i, w)| {
                matches!(w.segment, Segment::Line { .. })
                    && w.segment.end()
                        == outline_wires[(i + 1) % outline_wires.len()].segment.start()
            });
        if ring {
            board.outline = outline_wires.iter().map(|w| w.segment.start()).collect();
        } else if !outline_wires.is_empty() {
            for _ in &outline_wires {
                note("wire", &mut notes);
            }
        }
        for (layer, width, path) in runs(silk_wires) {
            if let Some(layer) = layer_ids.get(&layer) {
                board.silkscreen.insert(
                    new_id(),
                    SilkItem::Stroke {
                        layer: *layer,
                        width,
                        path,
                    },
                );
            }
        }
    }

    for (name, count) in &notes {
        diagnostics.push(codes::diagnostic(
            NOT_READ,
            Severity::Info,
            &format!("{count} {name} element(s)"),
        ));
    }
    Ok(ReadBoard { board, diagnostics })
}
