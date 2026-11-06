// SPDX-License-Identifier: GPL-3.0-or-later
//! The board model written as a KiCad 8 `.kicad_pcb` file, and the footprints of a board written
//! as `.kicad_mod` files.
//!
//! The y axis of the model points up and the one of KiCad points down, so every position gets its
//! y turned. Angles keep their sign. The copper layers take the names `F.Cu`, `In1.Cu`, ... and
//! `B.Cu` from their place in the stack. Everything the model cannot say is counted in the
//! report: filled copper of zones, the real font of the text, the stackup, design rules, and so on.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, LayerKind, Pad, PadShape, Segment, Side, SilkItem,
};
use eh3d_phoenix_model::Point;

use crate::settings::KicadSettings;
use crate::write_codes::{
    Export, ExportFile, ExportReport, CHANGED, DANGLING, DEFAULTED, LEFT_OUT, ROUND_TRIP,
};
use crate::write_settings::WriteSettings;
use crate::write_sexpr::{
    at, atom, effects, int, list, num, pair, render, string, uuid_from, uuid_node, Sx,
};

/// The names of the layers of the model in KiCad.
pub(crate) struct LayerNames {
    pub(crate) names: BTreeMap<Id, String>,
    pub(crate) copper: Vec<String>,
}

const FIXED_LAYERS: [(i64, &str, &str, &str); 18] = [
    (9, "F.Adhes", "user", "F.Adhesive"),
    (11, "B.Adhes", "user", "B.Adhesive"),
    (13, "F.Paste", "user", ""),
    (15, "B.Paste", "user", ""),
    (5, "F.SilkS", "user", "F.Silkscreen"),
    (7, "B.SilkS", "user", "B.Silkscreen"),
    (1, "F.Mask", "user", ""),
    (3, "B.Mask", "user", ""),
    (17, "Dwgs.User", "user", "User.Drawings"),
    (19, "Cmts.User", "user", "User.Comments"),
    (21, "Eco1.User", "user", "User.Eco1"),
    (23, "Eco2.User", "user", "User.Eco2"),
    (25, "Edge.Cuts", "user", ""),
    (27, "Margin", "user", ""),
    (31, "F.CrtYd", "user", "F.Courtyard"),
    (29, "B.CrtYd", "user", "B.Courtyard"),
    (35, "F.Fab", "user", ""),
    (33, "B.Fab", "user", ""),
];

/// The most copper layers a KiCad board can have.
const MAX_COPPER_LAYERS: usize = 32;

fn layer_row(number: i64, name: &str, kind: &str, alias: &str) -> Sx {
    let mut items = vec![int(number), string(name), atom(kind)];
    if !alias.is_empty() {
        items.push(string(alias));
    }
    Sx::List(items)
}

fn table_copper(names: &LayerNames) -> Vec<String> {
    if names.copper.is_empty() {
        vec!["F.Cu".to_owned(), "B.Cu".to_owned()]
    } else {
        names.copper.clone()
    }
}

fn layer_table(copper: &[String]) -> Sx {
    let mut rows = Vec::new();
    for (index, name) in copper.iter().enumerate() {
        let number = if index == 0 {
            0
        } else if index + 1 == copper.len() {
            2
        } else {
            4 + 2 * (index as i64 - 1)
        };
        rows.push(layer_row(number, name, "signal", ""));
    }
    for (number, name, kind, alias) in FIXED_LAYERS.iter() {
        rows.push(layer_row(*number, name, kind, alias));
    }
    list("layers", rows)
}

/// The side a layer of the model is for, and its names on the two sides.
fn sided(
    side: Option<Side>,
    front: &str,
    back: &str,
    what: &str,
    report: &mut ExportReport,
) -> String {
    match side {
        Some(Side::Bottom) => back.to_owned(),
        Some(Side::Top) => front.to_owned(),
        None => {
            report.note(
                DEFAULTED,
                &format!("{what} layers without a side"),
                "written on the top side",
            );
            front.to_owned()
        }
    }
}

/// Gives each layer of the model its name in KiCad.
pub(crate) fn name_layers(board: &Board, report: &mut ExportReport) -> LayerNames {
    let mut copper_ids: Vec<Id> = board
        .copper_stack
        .iter()
        .copied()
        .filter(|id| {
            board
                .layers
                .get(id)
                .is_some_and(|l| l.kind == LayerKind::Copper)
        })
        .collect();
    if board.copper_stack.is_empty() {
        copper_ids = board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == LayerKind::Copper)
            .map(|(id, _)| *id)
            .collect();
        if copper_ids.len() > 1 {
            report.note(
                DEFAULTED,
                "copper layers without an order in the stack",
                "taken in the order of their ids, the first as the top layer",
            );
        }
    }
    let total = copper_ids.len();
    let mut names: BTreeMap<Id, String> = BTreeMap::new();
    let mut copper: Vec<String> = Vec::new();
    for (index, id) in copper_ids.iter().enumerate() {
        if index >= MAX_COPPER_LAYERS {
            report.note(
                LEFT_OUT,
                "copper layers beyond the 32 of KiCad",
                "the layer and what is on it are left out",
            );
            continue;
        }
        let name = if index == 0 {
            "F.Cu".to_owned()
        } else if index + 1 == total {
            "B.Cu".to_owned()
        } else {
            format!("In{index}.Cu")
        };
        names.insert(*id, name.clone());
        copper.push(name);
    }
    let mut taken: BTreeMap<String, Id> = BTreeMap::new();
    for (id, layer) in &board.layers {
        let name = match layer.kind {
            LayerKind::Copper => continue,
            LayerKind::Silkscreen => sided(layer.side, "F.SilkS", "B.SilkS", "silkscreen", report),
            LayerKind::SolderMask => sided(layer.side, "F.Mask", "B.Mask", "solder mask", report),
            LayerKind::Paste => sided(layer.side, "F.Paste", "B.Paste", "solder paste", report),
            LayerKind::Outline => "Edge.Cuts".to_owned(),
            LayerKind::Documentation => {
                report.note(
                    CHANGED,
                    "documentation layers",
                    "written on the layer User.Drawings",
                );
                "Dwgs.User".to_owned()
            }
        };
        if let Some(other) = taken.get(&name) {
            if other != id {
                report.note(
                    CHANGED,
                    "layers of the model that share one layer of KiCad",
                    &format!("two layers are written on {name}"),
                );
            }
        } else {
            taken.insert(name.clone(), *id);
        }
        names.insert(*id, name);
    }
    LayerNames { names, copper }
}

/// The number and the name of each net in the file.
pub(crate) struct NetTable {
    pub(crate) number: BTreeMap<Id, i64>,
    pub(crate) name: BTreeMap<Id, String>,
}

pub(crate) fn net_table(board: &Board, report: &mut ExportReport) -> NetTable {
    let mut number = BTreeMap::new();
    let mut name = BTreeMap::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for (index, (id, net)) in board.nets.iter().enumerate() {
        let n = index as i64 + 1;
        let mut text = net.name.clone();
        if text.is_empty() {
            text = format!("Net-{n}");
            report.note(CHANGED, "nets without a name", "named Net-<number>");
        }
        if used.contains(&text) {
            text = format!("{text}_{n}");
            report.note(
                CHANGED,
                "nets with a name that another net has",
                "the number of the net was added to the name",
            );
        }
        used.insert(text.clone());
        number.insert(*id, n);
        name.insert(*id, text);
    }
    NetTable { number, name }
}

fn stroke(width: i64) -> Sx {
    list(
        "stroke",
        vec![
            list("width", vec![num(width)]),
            list("type", vec![atom("default")]),
        ],
    )
}

/// The point halfway along an arc given as end points and a signed sweep, in the orientation of
/// the model. `None` when the three values do not make an arc.
pub(crate) fn arc_mid(start: Point, end: Point, sweep_udeg: i64) -> Option<(i64, i64)> {
    let sx = start.x_nm.get() as f64;
    let sy = start.y_nm.get() as f64;
    let ex = end.x_nm.get() as f64;
    let ey = end.y_nm.get() as f64;
    let dx = ex - sx;
    let dy = ey - sy;
    let chord = (dx * dx + dy * dy).sqrt();
    if chord == 0.0 {
        return None;
    }
    let theta = sweep_udeg as f64 / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let half = theta.abs() / 2.0;
    let s = half.sin();
    if s.abs() < 1e-9 {
        return None;
    }
    let radius = chord / 2.0 / s;
    let height = radius * (1.0 - half.cos());
    let sign = if sweep_udeg > 0 { 1.0 } else { -1.0 };
    let mx = (sx + ex) / 2.0 + sign * (dy / chord) * height;
    let my = (sy + ey) / 2.0 - sign * (dx / chord) * height;
    Some((mx.round() as i64, my.round() as i64))
}

fn ky(p: Point) -> i64 {
    -p.y_nm.get()
}

fn kx(p: Point) -> i64 {
    p.x_nm.get()
}

fn xy_node(p: Point) -> Sx {
    pair("xy", kx(p), ky(p))
}

/// Everything the writers of a board need to know about it.
struct Ctx<'a> {
    settings: &'a WriteSettings,
    names: LayerNames,
    nets: NetTable,
    pad_net: BTreeMap<(Id, Id), Id>,
}

/// The name of a footprint in a library: the characters that KiCad does not allow in one are
/// replaced.
pub(crate) fn clean_name(name: &str, report: &mut ExportReport) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, ':' | '/' | '\\' | '"') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = if cleaned.trim().is_empty() {
        "footprint".to_owned()
    } else {
        cleaned
    };
    if cleaned != name {
        report.note(
            CHANGED,
            "names with characters that KiCad does not allow",
            "the characters were replaced by an underscore",
        );
    }
    cleaned
}

fn merge_pair(names: &mut Vec<String>, a: &str, b: &str, wildcard: &str) {
    let first = names.iter().position(|n| n == a || n == b);
    if names.iter().any(|n| n == a) && names.iter().any(|n| n == b) {
        names.retain(|n| n != a && n != b);
        let at_index = first.unwrap_or(0).min(names.len());
        names.insert(at_index, wildcard.to_owned());
    }
}

fn pad_layer_names(ctx: &Ctx, pad: &Pad, through: bool, report: &mut ExportReport) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for id in &pad.layers {
        match ctx.names.names.get(id) {
            Some(name) => {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            None => report.note(
                DANGLING,
                "pad layers that the board does not have",
                "the layer is left out of the pad",
            ),
        }
    }
    if names.is_empty() {
        report.note(
            DEFAULTED,
            "pads without a layer",
            "written on the usual layers of its kind",
        );
        let usual: &[&str] = if through {
            &["*.Cu", "*.Mask"]
        } else {
            &["F.Cu", "F.Paste", "F.Mask"]
        };
        return usual.iter().map(|s| (*s).to_owned()).collect();
    }
    let every_copper =
        ctx.names.copper.len() > 1 && ctx.names.copper.iter().all(|c| names.contains(c));
    if every_copper {
        let first = names
            .iter()
            .position(|n| ctx.names.copper.contains(n))
            .unwrap_or(0);
        names.retain(|n| !ctx.names.copper.contains(n));
        names.insert(first.min(names.len()), "*.Cu".to_owned());
    }
    merge_pair(&mut names, "F.Mask", "B.Mask", "*.Mask");
    names
}

fn pad_node(
    ctx: &Ctx,
    owner: Option<(&Id, &Component)>,
    pad_id: &Id,
    pad: &Pad,
    report: &mut ExportReport,
) -> Sx {
    let bottom = owner.is_some_and(|(_, c)| c.side == Side::Bottom);
    let component_angle = owner.map_or(0, |(_, c)| c.rotation.get());
    let own_angle = pad.rotation.get();
    let angle = (if bottom {
        component_angle - own_angle
    } else {
        component_angle + own_angle
    })
    .rem_euclid(360_000_000);
    let x = if bottom {
        -pad.at.x_nm.get()
    } else {
        pad.at.x_nm.get()
    };
    let y = -pad.at.y_nm.get();
    let through = pad.drill.is_some();
    let layers = pad_layer_names(ctx, pad, through, report);
    let has_copper = layers.iter().any(|n| n.ends_with(".Cu"));
    let kind = if !through {
        "smd"
    } else if has_copper && pad.plated {
        "thru_hole"
    } else {
        "np_thru_hole"
    };
    let rounded = pad.roundness > 0 && matches!(pad.shape, PadShape::Rect { .. });
    let (shape, w, h) = match pad.shape {
        PadShape::Circle { diameter } => ("circle", diameter.get(), diameter.get()),
        PadShape::Rect { width, height } => (
            if rounded { "roundrect" } else { "rect" },
            width.get(),
            height.get(),
        ),
        PadShape::Oblong { width, height } => ("oval", width.get(), height.get()),
    };
    let mut items = vec![
        string(&pad.number),
        atom(kind),
        atom(shape),
        at(x, y, angle),
        pair("size", w, h),
    ];
    if let Some(drill) = pad.drill {
        items.push(list("drill", vec![num(drill.get())]));
    }
    items.push(list("layers", layers.iter().map(|n| string(n)).collect()));
    if rounded {
        // the radius ratio of KiCad is the roundness over two thousand
        let ratio = f64::from(pad.roundness.min(1000)) / 2000.0;
        items.push(list("roundrect_rratio", vec![atom(&format!("{ratio}"))]));
    }
    if let Some((component_id, _)) = owner {
        if kind != "np_thru_hole" {
            if let Some(net_id) = ctx.pad_net.get(&(*component_id, *pad_id)) {
                if let (Some(n), Some(name)) =
                    (ctx.nets.number.get(net_id), ctx.nets.name.get(net_id))
                {
                    items.push(list("net", vec![int(*n), string(name)]));
                    report.wrote("pad nets", 1);
                } else {
                    report.note(
                        DANGLING,
                        "pads connected to a net the board does not have",
                        "the pad is written without a net",
                    );
                }
            }
        }
    }
    let mut key: Vec<u8> = Vec::new();
    if let Some((component_id, _)) = owner {
        key.extend_from_slice(component_id.as_bytes());
    }
    key.extend_from_slice(pad_id.as_bytes());
    items.push(list(
        "uuid",
        vec![string(&uuid_from(
            if owner.is_some() {
                "pad"
            } else {
                "library-pad"
            },
            &key,
        ))],
    ));
    report.wrote("pads", 1);
    list("pad", items)
}

/// A footprint of the model as a `footprint` item. `owner` is the component when it is on a
/// board and `None` when it is for a library file.
fn footprint_node(
    ctx: &Ctx,
    footprint_id: &Id,
    footprint: &Footprint,
    owner: Option<(&Id, &Component)>,
    report: &mut ExportReport,
) -> Sx {
    let settings = ctx.settings;
    let bottom = owner.is_some_and(|(_, c)| c.side == Side::Bottom);
    let name = clean_name(&footprint.name, report);
    let front = if bottom { "B.SilkS" } else { "F.SilkS" };
    let fab = if bottom { "B.Fab" } else { "F.Fab" };
    let justify: &[&str] = if bottom { &["mirror"] } else { &[] };
    let (reference, value) = match owner {
        Some((_, c)) => (c.refdes.clone(), c.value.clone()),
        None => ("REF**".to_owned(), name.clone()),
    };
    let size = settings.text_size_nm;
    let text_uuid = |role: &str| -> Sx {
        let mut key: Vec<u8> = Vec::new();
        if let Some((cid, _)) = owner {
            key.extend_from_slice(cid.as_bytes());
        }
        key.extend_from_slice(footprint_id.as_bytes());
        list("uuid", vec![string(&uuid_from(role, &key))])
    };
    let property = |label: &str, text: &str, layer: &str, y: i64, role: &str| -> Sx {
        list(
            "property",
            vec![
                string(label),
                string(text),
                at(0, y, 0),
                list("layer", vec![string(layer)]),
                text_uuid(role),
                effects(size, settings.text_thickness_nm, false, justify),
            ],
        )
    };
    let mut items = vec![
        string(&format!("{}:{}", settings.lib_nickname, name)),
        list("layer", vec![string(if bottom { "B.Cu" } else { "F.Cu" })]),
    ];
    match owner {
        Some((cid, c)) => {
            items.push(uuid_node(cid, "footprint"));
            items.push(at(
                c.at.x_nm.get(),
                -c.at.y_nm.get(),
                c.rotation.get().rem_euclid(360_000_000),
            ));
        }
        None => items.push(uuid_node(footprint_id, "library-footprint")),
    }
    items.push(property(
        "Reference",
        &reference,
        front,
        -2 * size,
        "reference",
    ));
    items.push(property("Value", &value, fab, 2 * size, "value"));
    if let Some((_, c)) = owner {
        if let Some(part) = &c.symbol {
            items.push(list(
                "path",
                vec![string(&format!(
                    "/{}",
                    crate::write_sexpr::uuid_for(part, "schematic-part")
                ))],
            ));
            report.note(
                CHANGED,
                "links to a part of the schematic",
                "written as a path on the first sheet",
            );
        }
    }
    if !footprint.pads.is_empty() {
        let attr = if footprint.pads.values().any(|p| p.drill.is_some()) {
            "through_hole"
        } else {
            "smd"
        };
        items.push(list("attr", vec![atom(attr)]));
    }
    for (pad_id, pad) in &footprint.pads {
        items.push(pad_node(ctx, owner, pad_id, pad, report));
    }
    list("footprint", items)
}

fn capped(mut nodes: Vec<Sx>, max: usize, what: &str, report: &mut ExportReport) -> Vec<Sx> {
    if nodes.len() > max {
        report.note_many(
            LEFT_OUT,
            &format!("{what} over the limit max_items"),
            "the first ones are written",
            nodes.len() - max,
        );
        nodes.truncate(max);
    }
    nodes
}

fn track_nodes(board: &Board, ctx: &Ctx, report: &mut ExportReport) -> Vec<Sx> {
    let mut out = Vec::new();
    for (track_id, track) in &board.tracks {
        let (Some(layer), Some(net)) = (
            ctx.names.names.get(&track.layer),
            ctx.nets.number.get(&track.net),
        ) else {
            report.note(
                DANGLING,
                "tracks on a layer or a net the board does not have",
                "the track is left out",
            );
            continue;
        };
        for (index, piece) in track.path.iter().enumerate() {
            let role = format!("track:{index}");
            let tail = |nodes: &mut Vec<Sx>| {
                nodes.push(list("width", vec![num(track.width.get())]));
                nodes.push(list("layer", vec![string(layer)]));
                nodes.push(list("net", vec![int(*net)]));
                nodes.push(uuid_node(track_id, &role));
            };
            let mut nodes: Vec<Sx> = Vec::new();
            let mut as_arc = false;
            if let Segment::Arc { start, end, sweep } = piece {
                if let Some((mx, my)) = arc_mid(*start, *end, sweep.get()) {
                    nodes.push(pair("start", kx(*start), ky(*start)));
                    nodes.push(pair("mid", mx, -my));
                    nodes.push(pair("end", kx(*end), ky(*end)));
                    as_arc = true;
                } else {
                    report.note(
                        CHANGED,
                        "arcs without a usable circle",
                        "written as straight pieces between their ends",
                    );
                }
            }
            if !as_arc {
                let (a, b) = (piece.start(), piece.end());
                nodes.push(pair("start", kx(a), ky(a)));
                nodes.push(pair("end", kx(b), ky(b)));
            }
            tail(&mut nodes);
            out.push(list(if as_arc { "arc" } else { "segment" }, nodes));
        }
    }
    report.wrote("track pieces", out.len());
    out
}

fn via_nodes(board: &Board, ctx: &Ctx, report: &mut ExportReport) -> Vec<Sx> {
    let mut out = Vec::new();
    for (via_id, via) in &board.vias {
        let (Some(from), Some(to), Some(net)) = (
            ctx.names.names.get(&via.from_layer),
            ctx.names.names.get(&via.to_layer),
            ctx.nets.number.get(&via.net),
        ) else {
            report.note(
                DANGLING,
                "vias on a layer or a net the board does not have",
                "the via is left out",
            );
            continue;
        };
        let outer = |n: &str| n == "F.Cu" || n == "B.Cu";
        let through = outer(from.as_str()) && outer(to.as_str()) && from != to;
        let mut items: Vec<Sx> = Vec::new();
        if !through {
            items.push(atom("blind"));
            report.note(
                CHANGED,
                "vias that do not go from the top to the bottom",
                "written as blind vias",
            );
        }
        items.push(at(via.at.x_nm.get(), -via.at.y_nm.get(), 0));
        items.push(list("size", vec![num(via.diameter.get())]));
        items.push(list("drill", vec![num(via.drill.get())]));
        items.push(list("layers", vec![string(from), string(to)]));
        items.push(list("net", vec![int(*net)]));
        items.push(uuid_node(via_id, "via"));
        out.push(list("via", items));
    }
    report.wrote("vias", out.len());
    out
}

fn zone_nodes(board: &Board, ctx: &Ctx, report: &mut ExportReport) -> Vec<Sx> {
    let mut out = Vec::new();
    let mut poured = 0usize;
    for (zone_id, zone) in &board.zones {
        let (Some(layer), Some(net), Some(net_name)) = (
            ctx.names.names.get(&zone.layer),
            ctx.nets.number.get(&zone.net),
            ctx.nets.name.get(&zone.net),
        ) else {
            report.note(
                DANGLING,
                "zones on a layer or a net the board does not have",
                "the zone is left out",
            );
            continue;
        };
        let mut points = zone.outline.clone();
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if points.len() < 3 {
            report.note(
                LEFT_OUT,
                "zones with fewer than three points",
                "the zone is left out",
            );
            continue;
        }
        let settings = ctx.settings;
        // KiCad has no hole in the outline of a zone as such: each cutout is a rule area on the
        // same layer that keeps the pour out and allows tracks, vias and pads
        for (index, cutout) in zone.cutouts.iter().enumerate() {
            let ring = closed_ring(cutout);
            if ring.len() < 3 {
                report.note(
                    LEFT_OUT,
                    "cutouts of zones with fewer than three points",
                    "the cutout is left out",
                );
                continue;
            }
            out.push(list(
                "zone",
                vec![
                    list("net", vec![int(0)]),
                    list("net_name", vec![string("")]),
                    list("layer", vec![string(layer)]),
                    uuid_node(zone_id, &format!("zone cutout {index}")),
                    list("hatch", vec![atom("edge"), num(settings.zone_hatch_nm)]),
                    list("connect_pads", vec![list("clearance", vec![num(0)])]),
                    list("min_thickness", vec![num(settings.zone_min_thickness_nm)]),
                    list(
                        "keepout",
                        vec![
                            list("tracks", vec![atom("allowed")]),
                            list("vias", vec![atom("allowed")]),
                            list("pads", vec![atom("allowed")]),
                            list("copperpour", vec![atom("not_allowed")]),
                            list("footprints", vec![atom("allowed")]),
                        ],
                    ),
                    list(
                        "polygon",
                        vec![list("pts", ring.iter().map(|p| xy_node(*p)).collect())],
                    ),
                ],
            ));
            report.wrote("zone cutouts", 1);
            report.note(
                CHANGED,
                "cutouts of zones written as rule areas that keep the pour out",
                "tracks, vias and pads are allowed in them, and KiCad reads them as keep-out areas",
            );
        }
        poured += 1;
        out.push(list(
            "zone",
            vec![
                list("net", vec![int(*net)]),
                list("net_name", vec![string(net_name)]),
                list("layer", vec![string(layer)]),
                uuid_node(zone_id, "zone"),
                list("hatch", vec![atom("edge"), num(settings.zone_hatch_nm)]),
                list(
                    "connect_pads",
                    vec![list("clearance", vec![num(zone.clearance.get())])],
                ),
                list("min_thickness", vec![num(settings.zone_min_thickness_nm)]),
                list("fill", vec![atom("yes")]),
                list(
                    "polygon",
                    vec![list("pts", points.iter().map(|p| xy_node(*p)).collect())],
                ),
            ],
        ));
    }
    report.wrote("zones", poured);
    if poured > 0 {
        report.note_many(
            DEFAULTED,
            "zones written without their filled copper",
            "the copper is filled again in KiCad",
            poured,
        );
    }
    out
}

/// The lines of one closed ring on `Edge.Cuts`. `ring` is the number of the ring (0 for the
/// outline, 1 and up for the cutouts) and goes into the identity of the lines.
fn ring_lines(board: &Board, ctx: &Ctx, ring: usize, points: &[Point]) -> Vec<Sx> {
    let mut out = Vec::new();
    for (index, a) in points.iter().enumerate() {
        let b = &points[(index + 1) % points.len()];
        if a == b {
            continue;
        }
        let mut key = format!("{}:", board.name).into_bytes();
        if ring > 0 {
            key.extend_from_slice(b"cutout:");
            key.extend_from_slice(&(ring as u64).to_be_bytes());
        }
        key.extend_from_slice(&(index as u64).to_be_bytes());
        out.push(list(
            "gr_line",
            vec![
                pair("start", kx(*a), ky(*a)),
                pair("end", kx(*b), ky(*b)),
                stroke(ctx.settings.edge_width_nm),
                list("layer", vec![string("Edge.Cuts")]),
                list("uuid", vec![string(&uuid_from("outline", &key))]),
            ],
        ));
    }
    out
}

fn closed_ring(points: &[Point]) -> Vec<Point> {
    let mut ring = points.to_vec();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    ring
}

fn outline_nodes(board: &Board, ctx: &Ctx, report: &mut ExportReport) -> Vec<Sx> {
    let mut out = Vec::new();
    let points = closed_ring(&board.outline);
    if points.is_empty() {
        return out;
    }
    if points.len() < 3 {
        report.note(
            LEFT_OUT,
            "outline with fewer than three points",
            "the outline is left out",
        );
        return out;
    }
    out.extend(ring_lines(board, ctx, 0, &points));
    report.wrote("outline lines", out.len());
    // the openings inside the outline are closed rings on the same layer
    let mut cutouts = 0usize;
    for (index, cutout) in board.outline_cutouts.iter().enumerate() {
        let ring = closed_ring(cutout);
        if ring.len() < 3 {
            report.note(
                LEFT_OUT,
                "cutouts with fewer than three points",
                "the cutout is left out",
            );
            continue;
        }
        out.extend(ring_lines(board, ctx, index + 1, &ring));
        cutouts += 1;
    }
    if cutouts > 0 {
        report.wrote("outline cutouts", cutouts);
    }
    out
}

fn silk_nodes(board: &Board, ctx: &Ctx, report: &mut ExportReport) -> Vec<Sx> {
    let mut out = Vec::new();
    let mut texts = 0usize;
    for (item_id, item) in &board.silkscreen {
        match item {
            SilkItem::Text {
                layer,
                at: place,
                text,
                height,
                stroke: thickness,
                rotation,
                mirrored,
            } => {
                let Some(name) = ctx.names.names.get(layer) else {
                    report.note(
                        DANGLING,
                        "silkscreen text on a layer the board does not have",
                        "the text is left out",
                    );
                    continue;
                };
                let mut justify = vec!["left", "bottom"];
                if *mirrored {
                    justify.push("mirror");
                }
                out.push(list(
                    "gr_text",
                    vec![
                        string(text),
                        at(
                            kx(*place),
                            ky(*place),
                            rotation.get().rem_euclid(360_000_000),
                        ),
                        list("layer", vec![string(name)]),
                        uuid_node(item_id, "silk-text"),
                        effects(height.get(), thickness.get(), false, &justify),
                    ],
                ));
                texts += 1;
            }
            SilkItem::Stroke { layer, width, path } => {
                let Some(name) = ctx.names.names.get(layer) else {
                    report.note(
                        DANGLING,
                        "silkscreen strokes on a layer the board does not have",
                        "the stroke is left out",
                    );
                    continue;
                };
                for (index, piece) in path.iter().enumerate() {
                    let role = format!("silk-stroke:{index}");
                    let mut nodes: Vec<Sx> = Vec::new();
                    let mut head = "gr_line";
                    if let Segment::Arc { start, end, sweep } = piece {
                        if let Some((mx, my)) = arc_mid(*start, *end, sweep.get()) {
                            nodes.push(pair("start", kx(*start), ky(*start)));
                            nodes.push(pair("mid", mx, -my));
                            nodes.push(pair("end", kx(*end), ky(*end)));
                            head = "gr_arc";
                        }
                    }
                    if head == "gr_line" {
                        let (a, b) = (piece.start(), piece.end());
                        nodes.push(pair("start", kx(a), ky(a)));
                        nodes.push(pair("end", kx(b), ky(b)));
                    }
                    nodes.push(stroke(width.get()));
                    nodes.push(list("layer", vec![string(name)]));
                    nodes.push(uuid_node(item_id, &role));
                    out.push(list(head, nodes));
                }
            }
        }
    }
    if texts > 0 {
        report.wrote("silkscreen texts", texts);
        report.note_many(
            CHANGED,
            "silkscreen text written as text of KiCad",
            "the stroke font of the model is not the font of KiCad",
            texts,
        );
    }
    report.wrote("silkscreen items", out.len() - texts);
    out
}

fn context<'a>(board: &Board, settings: &'a WriteSettings, report: &mut ExportReport) -> Ctx<'a> {
    let names = name_layers(board, report);
    let nets = net_table(board, report);
    let mut pad_net = BTreeMap::new();
    for pn in &board.pad_nets {
        pad_net.insert((pn.component, pn.pad), pn.net);
    }
    Ctx {
        settings,
        names,
        nets,
        pad_net,
    }
}

fn note_unwritten_parts(board: &Board, report: &mut ExportReport) {
    report.note(
        LEFT_OUT,
        "design rules, net classes and the stackup",
        "the board model keeps them in other documents of the project",
    );
    if !board.components.is_empty() {
        report.note(
            LEFT_OUT,
            "drawings of footprints (silkscreen, courtyard, 3D model)",
            "the footprint of the model holds pads only",
        );
    }
}

/// Writes a board as a `.kicad_pcb` file.
pub fn write_pcb(board: &Board, settings: &WriteSettings) -> Export {
    let mut report = ExportReport::default();
    let ctx = context(board, settings, &mut report);
    let mut root: Vec<Sx> = vec![
        list("version", vec![int(settings.pcb_version as i64)]),
        list("generator", vec![string(&settings.generator)]),
        list("paper", vec![string(&settings.paper)]),
        layer_table(&table_copper(&ctx.names)),
    ];
    let mut net_rows = vec![list("net", vec![int(0), string("")])];
    for (id, number) in &ctx.nets.number {
        let name = ctx.nets.name.get(id).cloned().unwrap_or_default();
        net_rows.push(list("net", vec![int(*number), string(&name)]));
    }
    report.wrote("nets", ctx.nets.number.len());
    root.extend(net_rows);

    let mut footprints: Vec<Sx> = Vec::new();
    let mut bottom = 0usize;
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            report.note(
                DANGLING,
                "components with a footprint the board does not have",
                "the component is left out",
            );
            continue;
        };
        if component.side == Side::Bottom {
            bottom += 1;
        }
        footprints.push(footprint_node(
            &ctx,
            &component.footprint,
            footprint,
            Some((component_id, component)),
            &mut report,
        ));
    }
    if bottom > 0 {
        report.note_many(
            CHANGED,
            "footprints on the bottom side",
            "written as KiCad flips them (x of the pads mirrored); not compared with KiCad",
            bottom,
        );
    }
    report.wrote("footprints", footprints.len());
    let max = settings.max_items;
    root.extend(capped(footprints, max, "footprints", &mut report));
    root.extend(capped(
        track_nodes(board, &ctx, &mut report),
        max,
        "track pieces",
        &mut report,
    ));
    root.extend(capped(
        via_nodes(board, &ctx, &mut report),
        max,
        "vias",
        &mut report,
    ));
    root.extend(capped(
        zone_nodes(board, &ctx, &mut report),
        max,
        "zones",
        &mut report,
    ));
    root.extend(outline_nodes(board, &ctx, &mut report));
    root.extend(capped(
        silk_nodes(board, &ctx, &mut report),
        max,
        "silkscreen items",
        &mut report,
    ));
    note_unwritten_parts(board, &mut report);

    let text = render(&list("kicad_pcb", root));
    check_pcb(&text, &mut report);
    Export {
        files: vec![ExportFile {
            name: format!("{}.kicad_pcb", file_stem(&board.name, "board")),
            text,
        }],
        report,
    }
}

/// A name that is safe as the stem of a file name.
pub fn file_stem(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(['.', ' ']).to_owned();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else if eh3d_phoenix_model::is_reserved_device_name(&trimmed) {
        // a file called CON.kicad_pcb is the console of Windows, not a file
        format!("_{trimmed}")
    } else {
        trimmed
    }
}

/// Writes the footprints of a board as `.kicad_mod` files, one for each footprint.
pub fn write_footprints(board: &Board, settings: &WriteSettings) -> Export {
    let mut report = ExportReport::default();
    let ctx = context(board, settings, &mut report);
    let mut files: Vec<ExportFile> = Vec::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for (footprint_id, footprint) in &board.footprints {
        let mut node = footprint_node(&ctx, footprint_id, footprint, None, &mut report);
        if let Sx::List(items) = &mut node {
            let extra = vec![
                list("version", vec![int(settings.pcb_version as i64)]),
                list("generator", vec![string(&settings.generator)]),
            ];
            for (offset, item) in extra.into_iter().enumerate() {
                items.insert(2 + offset, item);
            }
        }
        let mut stem = file_stem(&footprint.name, "footprint");
        if used.contains(&stem) {
            report.note(
                CHANGED,
                "footprints with a name that another footprint has",
                "the number of the footprint was added to the file name",
            );
            stem = format!("{stem}_{}", files.len() + 1);
        }
        used.insert(stem.clone());
        let text = render(&node);
        check_syntax(&text, "a footprint file", &mut report);
        files.push(ExportFile {
            name: format!("{stem}.kicad_mod"),
            text,
        });
    }
    report.wrote("footprint files", files.len());
    Export { files, report }
}

/// Parses the text with the reader of the crate; a text that does not parse is a note.
pub(crate) fn check_syntax(text: &str, what: &str, report: &mut ExportReport) -> bool {
    match crate::sexpr::parse(text, &KicadSettings::default()) {
        Ok(_) => true,
        Err(error) => {
            report.note(ROUND_TRIP, what, &error.to_string());
            false
        }
    }
}

fn expect(report: &mut ExportReport, what: &str, wrote: usize, read: usize) {
    if wrote != read {
        report.note(
            ROUND_TRIP,
            what,
            &format!("{wrote} were written and {read} were read back"),
        );
    }
}

/// Reads the text back with the board reader of the crate and compares the counts with what was
/// written; a difference is a note of the report.
pub(crate) fn check_pcb(text: &str, report: &mut ExportReport) {
    if !check_syntax(text, "the board file", report) {
        return;
    }
    let mut ids = match crate::sequential_ids() {
        Ok(ids) => ids,
        Err(error) => {
            report.note(ROUND_TRIP, "the check could not start", &error.to_string());
            return;
        }
    };
    let read = match crate::pcb::read_pcb(text.as_bytes(), &KicadSettings::default(), &mut ids) {
        Ok(read) => read,
        Err(diagnostic) => {
            report.note(ROUND_TRIP, "the board file", &diagnostic.message);
            return;
        }
    };
    let board = &read.board;
    let pieces: usize = board.tracks.values().map(|t| t.path.len()).sum();
    let counts = [
        (
            "footprints",
            report.written_count("footprints"),
            board.components.len(),
        ),
        (
            "pad nets",
            report.written_count("pad nets"),
            board.pad_nets.len(),
        ),
        ("track pieces", report.written_count("track pieces"), pieces),
        ("vias", report.written_count("vias"), board.vias.len()),
        ("zones", report.written_count("zones"), board.zones.len()),
        ("nets", report.written_count("nets"), board.nets.len()),
    ];
    for (what, wrote, got) in counts {
        expect(report, what, wrote, got);
    }
}

#[cfg(test)]
pub(crate) mod sample {
    use super::*;
    use eh3d_phoenix_board::{Layer, Net, PadNet, Track, Via, Zone};
    use eh3d_phoenix_model::{Nm, ObjectId, Udeg};

    pub(crate) fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    pub(crate) fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    pub(crate) fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    /// A two-layer board: two resistors, a net between them, a track with an arc, a via, a zone,
    /// a rectangular outline and a text on the silkscreen.
    pub(crate) fn sample_board() -> Board {
        let mut b = Board {
            name: "Demo board".into(),
            ..Board::default()
        };
        let (top, bottom, silk, mask) = (id(1), id(2), id(3), id(4));
        for (lid, name, kind, side) in [
            (top, "Top", LayerKind::Copper, None),
            (bottom, "Bottom", LayerKind::Copper, None),
            (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
            (mask, "Mask", LayerKind::SolderMask, Some(Side::Top)),
        ] {
            b.layers.insert(
                lid,
                Layer {
                    name: name.into(),
                    kind,
                    side,
                },
            );
        }
        b.copper_stack = vec![top, bottom];
        let (gnd, sig) = (id(10), id(11));
        b.nets.insert(gnd, Net { name: "GND".into() });
        b.nets.insert(sig, Net { name: "SIG".into() });
        let (fp, pad1, pad2) = (id(20), id(21), id(22));
        let mut pads = BTreeMap::new();
        for (pid, number, x) in [(pad1, "1", -1_000_000), (pad2, "2", 1_000_000)] {
            pads.insert(
                pid,
                Pad {
                    number: number.into(),
                    shape: PadShape::Rect {
                        width: nm(1_200_000),
                        height: nm(1_000_000),
                    },
                    at: pt(x, 0),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![top, mask],
                    roundness: 0,
                    plated: true,
                },
            );
        }
        b.footprints.insert(
            fp,
            Footprint {
                name: "R_0603".into(),
                pads,
            },
        );
        let (c1, c2) = (id(30), id(31));
        for (cid, refdes, x) in [(c1, "R1", 10_000_000), (c2, "R2", 20_000_000)] {
            b.components.insert(
                cid,
                Component {
                    refdes: refdes.into(),
                    value: "10k".into(),
                    symbol: None,
                    footprint: fp,
                    at: pt(x, 5_000_000),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
        }
        b.pad_nets = vec![
            PadNet {
                component: c1,
                pad: pad2,
                net: sig,
            },
            PadNet {
                component: c2,
                pad: pad1,
                net: sig,
            },
            PadNet {
                component: c2,
                pad: pad2,
                net: gnd,
            },
        ];
        b.pad_nets.sort_by_key(|p| p.pad_ref());
        b.tracks.insert(
            id(40),
            Track {
                net: sig,
                layer: top,
                width: nm(250_000),
                path: vec![
                    Segment::Line {
                        start: pt(11_000_000, 5_000_000),
                        end: pt(15_000_000, 5_000_000),
                    },
                    Segment::Arc {
                        start: pt(15_000_000, 5_000_000),
                        end: pt(17_000_000, 3_000_000),
                        sweep: Udeg::new(-90_000_000).unwrap(),
                    },
                ],
            },
        );
        b.vias.insert(
            id(41),
            Via {
                net: gnd,
                at: pt(21_000_000, 8_000_000),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
        b.zones.insert(
            id(42),
            Zone {
                net: gnd,
                layer: bottom,
                outline: vec![
                    pt(0, 0),
                    pt(30_000_000, 0),
                    pt(30_000_000, 20_000_000),
                    pt(0, 20_000_000),
                ],
                clearance: nm(300_000),
                cutouts: Vec::new(),
            },
        );
        b.outline = vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 20_000_000),
            pt(0, 20_000_000),
        ];
        b.silkscreen.insert(
            id(43),
            SilkItem::Text {
                layer: silk,
                at: pt(2_000_000, 2_000_000),
                text: "REV A".into(),
                height: nm(1_000_000),
                stroke: nm(150_000),
                rotation: Udeg::ZERO,
                mirrored: false,
            },
        );
        b
    }
}

#[cfg(test)]
mod tests {
    use super::sample::{id, nm, pt, sample_board};
    use super::*;
    use crate::write_sexpr::testkit::{arg, child, children, head, items, parse};
    use eh3d_phoenix_board::Net;

    #[test]
    fn the_midpoint_of_an_arc_lies_on_the_circle_on_the_right_side() {
        // a counter-clockwise half turn from (0,0) to (2,0) passes below, through (1,-1)
        let m = arc_mid(pt(0, 0), pt(2_000_000, 0), 180_000_000).unwrap();
        assert_eq!(m, (1_000_000, -1_000_000));
        // clockwise passes above
        let m = arc_mid(pt(0, 0), pt(2_000_000, 0), -180_000_000).unwrap();
        assert_eq!(m, (1_000_000, 1_000_000));
        assert_eq!(arc_mid(pt(0, 0), pt(0, 0), 90_000_000), None);
        assert_eq!(arc_mid(pt(0, 0), pt(1_000_000, 0), 0), None);
    }

    #[test]
    fn a_board_is_written_with_its_parts_and_is_well_formed() {
        let board = sample_board();
        let out = write_pcb(&board, &WriteSettings::default());
        assert_eq!(out.files.len(), 1);
        assert_eq!(out.files[0].name, "Demo board.kicad_pcb");
        let tree = parse(&out.files[0].text).unwrap();
        assert_eq!(head(&tree), Some("kicad_pcb"));
        assert_eq!(arg(child(&tree, "version").unwrap(), 0), Some("20240108"));
        assert_eq!(children(&tree, "footprint").len(), 2);
        assert_eq!(children(&tree, "segment").len(), 1);
        assert_eq!(children(&tree, "arc").len(), 1);
        assert_eq!(children(&tree, "via").len(), 1);
        assert_eq!(children(&tree, "zone").len(), 1);
        assert_eq!(children(&tree, "gr_line").len(), 4);
        assert_eq!(children(&tree, "gr_text").len(), 1);
        // three nets: the empty one, GND and SIG
        assert_eq!(children(&tree, "net").len(), 3);
        let layers = child(&tree, "layers").unwrap();
        let names: Vec<&str> = items(layers)
            .iter()
            .skip(1)
            .filter_map(|row| arg_of(row, 1))
            .collect();
        assert!(names.contains(&"F.Cu") && names.contains(&"B.Cu") && names.contains(&"Edge.Cuts"));
    }

    fn arg_of(row: &crate::write_sexpr::Sx, index: usize) -> Option<&str> {
        items(row)
            .get(index)
            .and_then(crate::write_sexpr::testkit::text)
    }

    #[test]
    fn y_is_turned_and_lengths_are_in_millimetres() {
        let board = sample_board();
        let text = write_pcb(&board, &WriteSettings::default())
            .files
            .remove(0)
            .text;
        let tree = parse(&text).unwrap();
        let first = children(&tree, "footprint")[0];
        let at_node = child(first, "at").unwrap();
        assert_eq!(arg(at_node, 0), Some("10"));
        assert_eq!(arg(at_node, 1), Some("-5"));
        let pad = child(first, "pad").unwrap();
        assert_eq!(arg(pad, 0), Some("1"));
        assert_eq!(arg(pad, 1), Some("smd"));
        assert_eq!(arg(pad, 2), Some("rect"));
        let size = child(pad, "size").unwrap();
        assert_eq!(arg(size, 0), Some("1.2"));
        assert_eq!(arg(size, 1), Some("1"));
        let via = children(&tree, "via")[0];
        assert_eq!(arg(child(via, "at").unwrap(), 1), Some("-8"));
        assert_eq!(arg(child(via, "size").unwrap(), 0), Some("0.6"));
    }

    #[test]
    fn the_output_is_the_same_every_time_and_ids_derive_from_the_model() {
        let board = sample_board();
        let a = write_pcb(&board, &WriteSettings::default());
        let b = write_pcb(&board.clone(), &WriteSettings::default());
        assert_eq!(a.files, b.files);
        let tree = parse(&a.files[0].text).unwrap();
        let mut uuids: Vec<String> = Vec::new();
        collect_uuids(&tree, &mut uuids);
        assert!(!uuids.is_empty());
        let unique: BTreeSet<&String> = uuids.iter().collect();
        assert_eq!(unique.len(), uuids.len(), "a uuid is used twice");
    }

    fn collect_uuids(node: &crate::write_sexpr::Sx, out: &mut Vec<String>) {
        if head(node) == Some("uuid") {
            if let Some(text) = arg(node, 0) {
                out.push(text.to_owned());
            }
        }
        for item in items(node) {
            collect_uuids(item, out);
        }
    }

    #[test]
    fn the_report_says_what_the_model_cannot_say() {
        let board = sample_board();
        let out = write_pcb(&board, &WriteSettings::default());
        let report = &out.report;
        assert_eq!(report.written_count("footprints"), 2);
        assert_eq!(report.written_count("pads"), 4);
        assert_eq!(report.written_count("pad nets"), 3);
        assert_eq!(report.written_count("track pieces"), 2);
        assert_eq!(report.count_of(DEFAULTED), 1);
        assert!(report.count_of(LEFT_OUT) >= 2);
        assert!(
            !report.has_problem(),
            "{}",
            report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn the_board_reads_back_with_the_reader_of_the_crate() {
        let board = sample_board();
        let out = write_pcb(&board, &WriteSettings::default());
        let mut ids = crate::sequential_ids().unwrap();
        let read = crate::pcb::read_pcb(
            out.files[0].text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
        )
        .unwrap();
        let back = &read.board;
        assert_eq!(back.components.len(), 2);
        assert_eq!(back.vias.len(), 1);
        assert_eq!(back.zones.len(), 1);
        assert_eq!(back.nets.len(), 2);
        assert_eq!(back.pad_nets.len(), 3);
        let via = back.vias.values().next().unwrap();
        assert_eq!(via.at, pt(21_000_000, 8_000_000));
        assert_eq!(via.diameter, nm(600_000));
        let refs: BTreeSet<&str> = back
            .components
            .values()
            .map(|c| c.refdes.as_str())
            .collect();
        assert_eq!(refs, BTreeSet::from(["R1", "R2"]));
        let track = back.tracks.values().next().unwrap();
        assert_eq!(track.path.len(), 2);
        assert_eq!(track.path[0].start(), pt(11_000_000, 5_000_000));
        assert_eq!(track.path[1].end(), pt(17_000_000, 3_000_000));
    }

    #[test]
    fn what_cannot_be_written_is_counted_and_never_dropped_in_silence() {
        let mut board = sample_board();
        // a track on a layer that does not exist, a net without a name, a component without a
        // footprint, a layer without a side and a via between inner layers
        board.tracks.values_mut().next().unwrap().layer = id(99);
        board.nets.insert(
            id(12),
            Net {
                name: String::new(),
            },
        );
        board.components.get_mut(&id(31)).unwrap().footprint = id(98);
        board.layers.get_mut(&id(3)).unwrap().side = None;
        let out = write_pcb(&board, &WriteSettings::default());
        let report = &out.report;
        assert_eq!(report.count_of(DANGLING), 2);
        assert!(report.count_of(CHANGED) >= 1);
        assert!(report.count_of(DEFAULTED) >= 2);
        assert_eq!(report.written_count("footprints"), 1);
        assert!(
            !report.has_problem(),
            "{}",
            report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn the_footprints_of_a_board_are_written_one_file_each() {
        let mut board = sample_board();
        let mut second = board.footprints.values().next().unwrap().clone();
        second.name = "R:0603/odd".into();
        board.footprints.insert(id(50), second);
        let out = write_footprints(&board, &WriteSettings::default());
        assert_eq!(out.files.len(), 2);
        let names: Vec<&str> = out.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["R_0603.kicad_mod", "R_0603_odd.kicad_mod"]);
        for file in &out.files {
            let tree = parse(&file.text).unwrap();
            assert_eq!(head(&tree), Some("footprint"));
            assert!(child(&tree, "version").is_some());
            assert_eq!(children(&tree, "pad").len(), 2);
        }
        assert!(out.report.count_of(CHANGED) >= 1);
        assert!(!out.report.has_problem());
        let mut ids = crate::sequential_ids().unwrap();
        let read = crate::footprint::read_footprint(
            out.files[0].text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
        )
        .unwrap();
        assert_eq!(read.footprint.pads.len(), 2);
        assert!(read.footprint.name.ends_with("R_0603"));
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem("a/b:c", "x"), "a_b_c");
        assert_eq!(file_stem("..", "x"), "x");
        assert_eq!(file_stem("", "board"), "board");
        assert_eq!(file_stem("My board 2", "x"), "My board 2");
    }

    #[test]
    fn a_name_of_a_windows_device_does_not_become_a_file_name() {
        assert_eq!(file_stem("CON", "x"), "_CON");
        assert_eq!(file_stem("nul.v2", "x"), "_nul.v2");
        assert_eq!(file_stem("Aux", "x"), "_Aux");
        assert_eq!(file_stem("console", "x"), "console");
    }

    #[test]
    fn a_through_hole_pad_with_every_copper_layer_uses_the_wildcards() {
        let mut board = sample_board();
        let fp = board.footprints.get_mut(&id(20)).unwrap();
        let pad = fp.pads.get_mut(&id(21)).unwrap();
        pad.drill = Some(nm(800_000));
        pad.shape = PadShape::Circle {
            diameter: nm(1_600_000),
        };
        pad.layers = vec![id(1), id(2), id(4)];
        let text = write_pcb(&board, &WriteSettings::default())
            .files
            .remove(0)
            .text;
        let tree = parse(&text).unwrap();
        let first = children(&tree, "footprint")[0];
        let pad = child(first, "pad").unwrap();
        assert_eq!(arg(pad, 1), Some("thru_hole"));
        assert_eq!(arg(pad, 2), Some("circle"));
        let layers = child(pad, "layers").unwrap();
        assert_eq!(arg(layers, 0), Some("*.Cu"));
        assert_eq!(arg(child(first, "attr").unwrap(), 0), Some("through_hole"));
    }

    #[test]
    fn cutouts_and_rounded_and_unplated_pads_are_written_and_read_back() {
        let mut board = sample_board();
        // the sample outline is a rectangle; an opening inside it, 5 by 3 mm
        let xs: Vec<i64> = board.outline.iter().map(|p| p.x_nm.get()).collect();
        let ys: Vec<i64> = board.outline.iter().map(|p| p.y_nm.get()).collect();
        let (x0, y0) = (
            xs.iter().min().copied().unwrap(),
            ys.iter().min().copied().unwrap(),
        );
        board.outline_cutouts = vec![vec![
            pt(x0 + 2_000_000, y0 + 2_000_000),
            pt(x0 + 7_000_000, y0 + 2_000_000),
            pt(x0 + 7_000_000, y0 + 5_000_000),
            pt(x0 + 2_000_000, y0 + 5_000_000),
        ]];
        // the first pad is rounded by a quarter of its shorter side, the second one is a drilled
        // hole that is not plated
        for footprint in board.footprints.values_mut() {
            for pad in footprint.pads.values_mut() {
                if pad.number == "1" {
                    pad.roundness = 500;
                } else {
                    pad.drill = Some(nm(800_000));
                    pad.plated = false;
                }
            }
        }
        let out = write_pcb(&board, &WriteSettings::default());
        let tree = parse(&out.files[0].text).unwrap();
        // four lines of the outline and four of the opening
        assert_eq!(children(&tree, "gr_line").len(), 8);
        assert_eq!(out.report.written_count("outline cutouts"), 1);
        let text = &out.files[0].text;
        assert!(text.contains("roundrect"), "{text}");
        assert!(text.contains("roundrect_rratio"), "{text}");
        assert!(text.contains("np_thru_hole"), "{text}");
        // read back: the opening, the roundness of the first pad and the hole
        let mut ids = crate::sequential_ids().unwrap();
        let read =
            crate::pcb::read_pcb(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap();
        assert_eq!(read.board.outline_cutouts.len(), 1);
        let mut found: Vec<(i64, i64)> = read.board.outline_cutouts[0]
            .iter()
            .map(|p| (p.x_nm.get() - x0, p.y_nm.get() - y0))
            .collect();
        found.sort_unstable();
        assert_eq!(
            found,
            vec![
                (2_000_000, 2_000_000),
                (2_000_000, 5_000_000),
                (7_000_000, 2_000_000),
                (7_000_000, 5_000_000)
            ]
        );
        let pads: Vec<_> = read
            .board
            .footprints
            .values()
            .flat_map(|f| f.pads.values())
            .collect();
        assert!(pads.iter().any(|p| p.roundness == 500 && p.plated));
        assert!(pads.iter().any(|p| p.is_npth()));
    }

    #[test]
    fn the_cutouts_of_a_zone_are_rule_areas_that_keep_the_pour_out_and_are_read_back() {
        let mut board = sample_board();
        let square = vec![
            pt(5_000_000, 5_000_000),
            pt(10_000_000, 5_000_000),
            pt(10_000_000, 10_000_000),
            pt(5_000_000, 10_000_000),
        ];
        let short = vec![pt(1_000_000, 1_000_000), pt(2_000_000, 1_000_000)];
        board.zones.values_mut().next().unwrap().cutouts = vec![square.clone(), short];
        let out = write_pcb(&board, &WriteSettings::default());
        let text = &out.files[0].text;
        let tree = parse(text).unwrap();
        // the pour and one rule area; the cutout of two points is reported and not written
        assert_eq!(children(&tree, "zone").len(), 2);
        assert_eq!(out.report.written_count("zones"), 1);
        assert_eq!(out.report.written_count("zone cutouts"), 1);
        assert!(text.contains("(copperpour not_allowed)"), "{text}");
        let notes = out.report.notes();
        assert!(notes
            .iter()
            .any(|n| n.code == LEFT_OUT && n.category.contains("fewer than three points")));
        assert!(notes.iter().any(|n| n.code == CHANGED
            && n.category == "cutouts of zones written as rule areas that keep the pour out"));
        // the same board gives the same bytes
        assert_eq!(
            write_pcb(&board, &WriteSettings::default()).files[0].text,
            *text
        );
        // read back: one zone with the cutout, at the same points
        let mut ids = crate::sequential_ids().unwrap();
        let read =
            crate::pcb::read_pcb(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap();
        assert_eq!(read.board.zones.len(), 1);
        let zone = read.board.zones.values().next().unwrap();
        assert_eq!(zone.cutouts.len(), 1);
        let mut found: Vec<(i64, i64)> = zone.cutouts[0]
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        let mut wanted: Vec<(i64, i64)> = square
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        found.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(found, wanted);
        let kept = read
            .quality
            .kept
            .iter()
            .find(|i| i.category == "zone cutouts");
        assert_eq!(kept.map(|i| i.count), Some(1));
        // a rule area that also keeps tracks out is not a cutout and is counted as lost
        let strict = text.replace("(tracks allowed)", "(tracks not_allowed)");
        let mut ids = crate::sequential_ids().unwrap();
        let read =
            crate::pcb::read_pcb(strict.as_bytes(), &KicadSettings::default(), &mut ids).unwrap();
        assert!(read.board.zones.values().all(|z| z.cutouts.is_empty()));
        let lost = read
            .quality
            .dropped
            .iter()
            .find(|i| i.category == "keep-out zones");
        assert_eq!(lost.map(|i| i.count), Some(1));
    }
}
