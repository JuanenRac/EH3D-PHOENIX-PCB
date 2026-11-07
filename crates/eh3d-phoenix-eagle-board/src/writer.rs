//! Writing a board as an EAGLE 9 XML file.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id, PadShape, Segment, Side};
use eh3d_phoenix_eagle::{write_document, Element};
use eh3d_phoenix_model::{Point, Udeg};

use crate::codes::{self, ITEM_LEFT_OUT, ITEM_SIMPLIFIED};
use crate::decimal;
use crate::layers::{assign, effective_copper, name_of};

/// The version written in the file.
const VERSION: &str = "9.6.2";
/// The name of the one library the packages go into.
pub(crate) const LIBRARY: &str = "phoenix";

/// The file and the report of what had to be simplified or left out.
#[derive(Debug, Clone)]
pub struct Written {
    /// The EAGLE XML text.
    pub text: String,
    /// What was written in a simpler form (information) or left out (warnings).
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn el(name: &str, attrs: Vec<(&str, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        text: String::new(),
        children,
    }
}

fn rot_text(rotation: Udeg, mirrored: bool) -> String {
    format!(
        "{}R{}",
        if mirrored { "M" } else { "" },
        decimal(rotation.normalised().get())
    )
}

fn wire(from: Point, to: Point, width: i64, layer: u32, curve: Option<i64>) -> Element {
    let mut attrs = vec![
        ("x1", decimal(from.x_nm.get())),
        ("y1", decimal(from.y_nm.get())),
        ("x2", decimal(to.x_nm.get())),
        ("y2", decimal(to.y_nm.get())),
        ("width", decimal(width)),
        ("layer", layer.to_string()),
    ];
    if let Some(sweep) = curve {
        attrs.push(("curve", decimal(sweep)));
    }
    el("wire", attrs, Vec::new())
}

fn segment_wire(segment: &Segment, width: i64, layer: u32) -> Element {
    match segment {
        Segment::Line { start, end } => wire(*start, *end, width, layer, None),
        Segment::Arc { start, end, sweep } => wire(*start, *end, width, layer, Some(sweep.get())),
    }
}

/// Unique package names: the name of the footprint, with a number added when two share one.
pub(crate) fn package_names(
    board: &Board,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<Id, String> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (id, fp) in &board.footprints {
        let mut name = fp.name.clone();
        let mut n = 1;
        while !used.insert(name.clone()) {
            n += 1;
            name = format!("{}_{n}", fp.name);
        }
        if name != fp.name {
            diagnostics.push(codes::diagnostic(
                ITEM_SIMPLIFIED,
                Severity::Info,
                &format!("footprint {id} is written as {name}: its name is used twice"),
            ));
        }
        names.insert(*id, name);
    }
    names
}

pub(crate) fn package(
    board: &Board,
    id: &Id,
    name: &str,
    numbers: &BTreeMap<Id, u32>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Element {
    let fp = &board.footprints[id];
    let copper = effective_copper(board);
    let mut children = Vec::new();
    for (pad_id, pad) in &fp.pads {
        let at = vec![
            ("x", decimal(pad.at.x_nm.get())),
            ("y", decimal(pad.at.y_nm.get())),
        ];
        let rot = (pad.rotation.normalised().get() != 0).then(|| rot_text(pad.rotation, false));
        let on: Vec<usize> = copper
            .iter()
            .enumerate()
            .filter(|(_, l)| pad.layers.contains(l) && numbers.contains_key(*l))
            .map(|(i, _)| i)
            .collect();
        if let (true, Some(drill)) = (pad.is_npth(), pad.drill) {
            // a hole that is not plated is a hole of the package, with no copper and no name
            let mut attrs = at.clone();
            attrs.push(("drill", decimal(drill.get())));
            children.push(el("hole", attrs, Vec::new()));
            continue;
        }
        if let Some(drill) = pad.drill {
            let (shape, diameter) = match pad.shape {
                PadShape::Circle { diameter } => ("round", diameter.get()),
                PadShape::Rect { width, height } if width == height => ("square", width.get()),
                PadShape::Rect { width, height } => {
                    diagnostics.push(codes::diagnostic(
                        ITEM_SIMPLIFIED,
                        Severity::Info,
                        &format!("pad {pad_id}: a drilled rectangle is written as a square"),
                    ));
                    ("square", width.get().min(height.get()))
                }
                PadShape::Oblong { width, height } => {
                    diagnostics.push(codes::diagnostic(
                        ITEM_SIMPLIFIED,
                        Severity::Info,
                        &format!("pad {pad_id}: a drilled oblong pad is written as a round one"),
                    ));
                    ("round", width.get().min(height.get()))
                }
            };
            if on.len() != copper.len() {
                diagnostics.push(codes::diagnostic(
                    ITEM_SIMPLIFIED,
                    Severity::Info,
                    &format!("pad {pad_id}: a drilled pad is on every copper layer"),
                ));
            }
            let mut attrs = vec![("name", pad.number.clone())];
            attrs.extend(at);
            attrs.push(("drill", decimal(drill.get())));
            attrs.push(("diameter", decimal(diameter)));
            attrs.push(("shape", shape.to_owned()));
            if let Some(r) = rot {
                attrs.push(("rot", r));
            }
            children.push(el("pad", attrs, Vec::new()));
        } else {
            let Some(first) = on.first() else {
                diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("pad {pad_id}: it is on no copper layer"),
                ));
                continue;
            };
            if on.len() > 1 {
                diagnostics.push(codes::diagnostic(
                    ITEM_SIMPLIFIED,
                    Severity::Info,
                    &format!("pad {pad_id}: a surface pad is written on one layer only"),
                ));
            }
            let layer = numbers[&copper[*first]];
            let (dx, dy, roundness) = match pad.shape {
                PadShape::Rect { width, height } => (
                    width.get(),
                    height.get(),
                    // the roundness of EAGLE is a whole percent (100 is a pad with round ends)
                    (u32::from(pad.roundness.min(1000)) + 5) / 10,
                ),
                PadShape::Circle { diameter } => (diameter.get(), diameter.get(), 100),
                PadShape::Oblong { width, height } => (width.get(), height.get(), 100),
            };
            let mut attrs = vec![("name", pad.number.clone())];
            attrs.extend(at);
            attrs.push(("dx", decimal(dx)));
            attrs.push(("dy", decimal(dy)));
            attrs.push(("layer", layer.to_string()));
            attrs.push(("roundness", roundness.to_string()));
            if let Some(r) = rot {
                attrs.push(("rot", r));
            }
            children.push(el("smd", attrs, Vec::new()));
        }
    }
    el("package", vec![("name", name.to_owned())], children)
}

/// Writes `board` as an EAGLE XML file.
pub fn write_board(board: &Board) -> Written {
    let (numbers, mut diagnostics) = assign(board);
    let mut used: BTreeSet<u32> = numbers.values().copied().collect();
    if board.outline.len() >= 3 {
        used.insert(20);
    }

    let names = package_names(board, &mut diagnostics);
    let packages: Vec<Element> = names
        .iter()
        .map(|(id, name)| package(board, id, name, &numbers, &mut diagnostics))
        .collect();
    let library = el(
        "library",
        vec![("name", LIBRARY.to_owned())],
        vec![el("packages", Vec::new(), packages)],
    );

    let mut elements = Vec::new();
    for (id, c) in &board.components {
        let Some(package) = names.get(&c.footprint) else {
            diagnostics.push(codes::diagnostic(
                ITEM_LEFT_OUT,
                Severity::Warning,
                &format!("component {id} has no footprint"),
            ));
            continue;
        };
        elements.push(el(
            "element",
            vec![
                ("name", c.refdes.clone()),
                ("library", LIBRARY.to_owned()),
                ("package", package.clone()),
                ("value", c.value.clone()),
                ("x", decimal(c.at.x_nm.get())),
                ("y", decimal(c.at.y_nm.get())),
                ("rot", rot_text(c.rotation, c.side == Side::Bottom)),
            ],
            Vec::new(),
        ));
    }

    let mut signals = Vec::new();
    // the cutouts of zones: EAGLE has no hole in a pour, only restrict areas on the layers
    // 41 (top) and 42 (bottom) that every pour of that layer keeps out of
    let mut restricts: Vec<(u32, Vec<Point>)> = Vec::new();
    for (nid, net) in &board.nets {
        let mut children = Vec::new();
        for pn in board.pad_nets.iter().filter(|p| p.net == *nid) {
            let contact = board.components.get(&pn.component).and_then(|c| {
                let number = &board
                    .footprints
                    .get(&c.footprint)?
                    .pads
                    .get(&pn.pad)?
                    .number;
                Some((c.refdes.clone(), number.clone()))
            });
            match contact {
                Some((element, pad)) => children.push(el(
                    "contactref",
                    vec![("element", element), ("pad", pad)],
                    Vec::new(),
                )),
                None => diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("a pad of net {} that does not exist", net.name),
                )),
            }
        }
        for (tid, t) in board.tracks.iter().filter(|(_, t)| t.net == *nid) {
            let Some(layer) = numbers.get(&t.layer) else {
                diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("track {tid}: its layer has no EAGLE number"),
                ));
                continue;
            };
            for s in &t.path {
                children.push(segment_wire(s, t.width.get(), *layer));
            }
        }
        for (vid, v) in board.vias.iter().filter(|(_, v)| v.net == *nid) {
            let (Some(a), Some(b)) = (numbers.get(&v.from_layer), numbers.get(&v.to_layer)) else {
                diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("via {vid}: a layer has no EAGLE number"),
                ));
                continue;
            };
            children.push(el(
                "via",
                vec![
                    ("x", decimal(v.at.x_nm.get())),
                    ("y", decimal(v.at.y_nm.get())),
                    ("extent", format!("{}-{}", a.min(b), a.max(b))),
                    ("drill", decimal(v.drill.get())),
                    ("diameter", decimal(v.diameter.get())),
                ],
                Vec::new(),
            ));
        }
        for (zid, z) in board.zones.iter().filter(|(_, z)| z.net == *nid) {
            let Some(layer) = numbers.get(&z.layer) else {
                diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("zone {zid}: its layer has no EAGLE number"),
                ));
                continue;
            };
            for cutout in z.cutouts.iter().filter(|c| c.len() >= 3) {
                let restrict = match *layer {
                    1 => 41,
                    16 => 42,
                    _ => {
                        diagnostics.push(codes::diagnostic(
                            ITEM_LEFT_OUT,
                            Severity::Warning,
                            &format!(
                                "zone {zid}: a cutout on an inner layer is not written, EAGLE has restrict areas for the top and the bottom layer only"
                            ),
                        ));
                        continue;
                    }
                };
                diagnostics.push(codes::diagnostic(
                    ITEM_SIMPLIFIED,
                    Severity::Info,
                    &format!(
                        "zone {zid}: a cutout is written as a restrict area, which every pour of the layer keeps out of"
                    ),
                ));
                restricts.push((restrict, cutout.clone()));
            }
            if z.cutouts.iter().any(|c| c.len() < 3) {
                diagnostics.push(codes::diagnostic(
                    ITEM_LEFT_OUT,
                    Severity::Warning,
                    &format!("zone {zid}: a cutout with fewer than three points is not written"),
                ));
            }
            let vertices = z
                .outline
                .iter()
                .map(|p| {
                    el(
                        "vertex",
                        vec![("x", decimal(p.x_nm.get())), ("y", decimal(p.y_nm.get()))],
                        Vec::new(),
                    )
                })
                .collect();
            children.push(el(
                "polygon",
                vec![
                    ("width", "0".to_owned()),
                    ("layer", layer.to_string()),
                    ("isolate", decimal(z.clearance.get())),
                ],
                vertices,
            ));
        }
        signals.push(el(
            "signal",
            vec![("name", net.name.clone()), ("class", "0".to_owned())],
            children,
        ));
    }

    let mut plain = Vec::new();
    // the outline and then each opening inside it, each a closed ring of wires on layer 20
    // the arcs of the outline are written as the chords they are cut into
    let outline = board.outline_polygon(&eh3d_phoenix_board::ArcSettings::current());
    for ring in std::iter::once(&outline).chain(board.outline_cutouts.iter()) {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            plain.push(wire(ring[i], ring[(i + 1) % n], 0, 20, None));
        }
    }
    for (restrict, ring) in &restricts {
        used.insert(*restrict);
        let vertices = ring
            .iter()
            .map(|p| {
                el(
                    "vertex",
                    vec![("x", decimal(p.x_nm.get())), ("y", decimal(p.y_nm.get()))],
                    Vec::new(),
                )
            })
            .collect();
        plain.push(el(
            "polygon",
            vec![("width", "0".to_owned()), ("layer", restrict.to_string())],
            vertices,
        ));
    }
    for (sid, item) in &board.silkscreen {
        let Some(layer) = numbers.get(&item.layer()) else {
            diagnostics.push(codes::diagnostic(
                ITEM_LEFT_OUT,
                Severity::Warning,
                &format!("silkscreen item {sid}: its layer has no EAGLE number"),
            ));
            continue;
        };
        match item.drawn() {
            Ok(d) => {
                if matches!(item, eh3d_phoenix_board::SilkItem::Text { .. }) {
                    diagnostics.push(codes::diagnostic(
                        ITEM_SIMPLIFIED,
                        Severity::Info,
                        &format!(
                            "silkscreen text {sid} is written as the strokes that are printed"
                        ),
                    ));
                }
                for s in &d.pieces {
                    plain.push(segment_wire(s, d.width.get(), *layer));
                }
            }
            Err(_) => diagnostics.push(codes::diagnostic(
                ITEM_LEFT_OUT,
                Severity::Warning,
                &format!("silkscreen item {sid} cannot be drawn"),
            )),
        }
    }

    let layers: Vec<Element> = used
        .iter()
        .map(|n| {
            el(
                "layer",
                vec![
                    ("number", n.to_string()),
                    ("name", name_of(*n)),
                    ("color", "1".to_owned()),
                    ("fill", "1".to_owned()),
                    ("visible", "yes".to_owned()),
                    ("active", "yes".to_owned()),
                ],
                Vec::new(),
            )
        })
        .collect();
    let board_element = el(
        "board",
        Vec::new(),
        vec![
            el("plain", Vec::new(), plain),
            el("libraries", Vec::new(), vec![library]),
            el("attributes", Vec::new(), Vec::new()),
            el("variantdefs", Vec::new(), Vec::new()),
            el(
                "classes",
                Vec::new(),
                vec![el(
                    "class",
                    vec![
                        ("number", "0".to_owned()),
                        ("name", "default".to_owned()),
                        ("width", "0".to_owned()),
                        ("drill", "0".to_owned()),
                    ],
                    Vec::new(),
                )],
            ),
            el("elements", Vec::new(), elements),
            el("signals", Vec::new(), signals),
        ],
    );
    let root = el(
        "eagle",
        vec![("version", VERSION.to_owned())],
        vec![el(
            "drawing",
            Vec::new(),
            vec![
                el("settings", Vec::new(), Vec::new()),
                el("layers", Vec::new(), layers),
                board_element,
            ],
        )],
    );
    Written {
        text: write_document(&root),
        diagnostics,
    }
}
