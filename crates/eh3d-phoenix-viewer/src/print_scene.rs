// SPDX-License-Identifier: GPL-3.0-or-later
//! What the viewer hands to the printing (`eh3d-phoenix-print`): the sheets of a schematic, the
//! assembly plan and the layers of a board, and a panel of boards, as the content of pages in
//! millimetres. A schematic is cut into its sheets and given the marks that the scene does not
//! draw (junctions, names of nets, power ports, ports of sheets and buses); an assembly plan
//! shows, for one side, the outline, the drills, the silkscreen, a silhouette of each footprint
//! (the box around its pads) with its reference designator, the mark of its first pin and the
//! fiducials, seen from above or, for the bottom side, as in a mirror. No window is needed.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef, PadShape, Side};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_print::{
    Anchor, Content, Item, Layer, LayerKind, PrintConfig, PrintText, Pt, Rgba,
};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{Id as SchId, Schematic};

use crate::load::Document;
use crate::panel_scene::{PanelLayerKind, PanelScene, PanelShape};
use crate::prim::{arc_points, Bounds, Prim, Shape, P};
use crate::scene::{Role, Scene};
use crate::settings::ViewerSettings;

/// The height of a capital letter over the size of the font.
const CAP_HEIGHT: f64 = 0.72;

fn nm_mm(n: Nm) -> f64 {
    n.get() as f64 / 1_000_000.0
}

fn point_mm(p: &Point) -> Pt {
    (nm_mm(p.x_nm), nm_mm(p.y_nm))
}

/// The kind of layer of the printing for a role of the scene.
pub fn kind_of(role: Role) -> LayerKind {
    match role {
        Role::Documentation => LayerKind::Documentation,
        Role::BottomCopper => LayerKind::BottomCopper,
        Role::InnerCopper(_) => LayerKind::InnerCopper,
        Role::TopCopper => LayerKind::TopCopper,
        Role::Mask => LayerKind::Mask,
        Role::Paste => LayerKind::Paste,
        Role::SilkBottom => LayerKind::SilkBottom,
        Role::SilkTop => LayerKind::SilkTop,
        Role::Designator => LayerKind::Designator,
        Role::Outline => LayerKind::Outline,
        Role::Drill => LayerKind::Drill,
        Role::Sheet => LayerKind::Sheet,
        Role::Wire => LayerKind::Wire,
        Role::Symbol => LayerKind::Symbol,
        Role::Pin => LayerKind::Pin,
        Role::Label => LayerKind::Label,
    }
}

/// How the points of the scene go to the page: a mirror about a vertical line (the bottom side of
/// a board) and a shift to the left (the sheets of a schematic are laid side by side).
struct Map {
    mirror_sum: Option<f64>,
    dx: f64,
}

impl Map {
    fn pt(&self, p: P) -> Pt {
        let x = match self.mirror_sum {
            Some(sum) => sum - p.x,
            None => p.x,
        };
        (x - self.dx, p.y)
    }
}

fn prim_item(prim: &Prim, colour: Rgba, map: &Map) -> Option<Item> {
    let width = prim.width;
    match &prim.shape {
        Shape::Line { a, b } => Some(Item::line(map.pt(*a), map.pt(*b), colour, width)),
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let points: Vec<Pt> = arc_points(*centre, *radius, *start, *sweep, 0.02, 8, 256)
                .into_iter()
                .map(|p| map.pt(p))
                .collect();
            Some(Item::polyline(points, colour, width))
        }
        Shape::Polygon { points } => (points.len() >= 3).then(|| {
            let mapped: Vec<Pt> = points.iter().map(|p| map.pt(*p)).collect();
            Item::polygon(mapped, colour, width, prim.filled)
        }),
        Shape::Circle { centre, radius } => Some(Item::circle(
            map.pt(*centre),
            *radius,
            colour,
            width,
            prim.filled,
        )),
        Shape::Text { at, height, text } => Some(Item::text(
            map.pt(*at),
            height / CAP_HEIGHT,
            Anchor::Start,
            false,
            text,
            colour,
        )),
    }
}

/// The layers of the scene that are shown and wanted, with the primitives that are wanted.
fn scene_layers(
    scene: &Scene,
    cfg: &PrintConfig,
    map: &Map,
    keep_layer: impl Fn(Role) -> bool,
    keep_prim: impl Fn(&Prim) -> bool,
) -> Vec<Layer> {
    let mut layers = Vec::new();
    for (index, layer) in scene.layers.iter().enumerate() {
        if !layer.visible || !keep_layer(layer.role) {
            continue;
        }
        let kind = kind_of(layer.role);
        let colour = cfg.colour(kind);
        let items: Vec<Item> = scene
            .prims
            .iter()
            .filter(|p| p.layer as usize == index && keep_prim(p))
            .filter_map(|p| prim_item(p, colour, map))
            .collect();
        if !items.is_empty() {
            layers.push(Layer { kind, items });
        }
    }
    layers
}

fn side_word(side: Side, lang: Lang) -> &'static str {
    match side {
        Side::Top => PrintText::Top.tr(lang),
        Side::Bottom => PrintText::Bottom.tr(lang),
    }
}

// ---------------------------------------------------------------------------------------------
// schematics

/// The x of a primitive, to tell on which sheet it is.
fn rep_x(shape: &Shape) -> f64 {
    match shape {
        Shape::Line { a, .. } => a.x,
        Shape::Arc { centre, .. } | Shape::Circle { centre, .. } => centre.x,
        Shape::Polygon { points } => points.first().map_or(0.0, |p| p.x),
        Shape::Text { at, .. } => at.x,
    }
}

fn sheet_of(x: f64, boxes: &[(String, Bounds)]) -> usize {
    if let Some(i) = boxes
        .iter()
        .position(|(_, b)| x >= b.min.x - 1e-6 && x <= b.max.x + 1e-6)
    {
        return i;
    }
    let distance = |b: &Bounds| (x - b.centre().x).abs();
    boxes
        .iter()
        .enumerate()
        .min_by(|a, b| distance(&a.1 .1).total_cmp(&distance(&b.1 .1)))
        .map_or(0, |(i, _)| i)
}

/// The marks of a sheet that the scene does not draw. The coordinates of the model are those of
/// the sheet, so no shift is needed.
fn sheet_extras(
    schematic: &Schematic,
    sheet: &SchId,
    settings: &ViewerSettings,
    cfg: &PrintConfig,
) -> Vec<Layer> {
    let font = settings.label_height_mm / CAP_HEIGHT;
    let mut junctions = Vec::new();
    let mut labels = Vec::new();
    let mut ports = Vec::new();
    let mut buses = Vec::new();
    let (cj, cl, cp, cb) = (
        cfg.colour(LayerKind::Junction),
        cfg.colour(LayerKind::Label),
        cfg.colour(LayerKind::Port),
        cfg.colour(LayerKind::Bus),
    );
    for junction in schematic.junctions.values().filter(|j| j.sheet == *sheet) {
        junctions.push(Item::circle(point_mm(&junction.at), 0.5, cj, 0.0, true));
    }
    for label in schematic.labels.values().filter(|l| l.sheet == *sheet) {
        let (x, y) = point_mm(&label.at);
        labels.push(Item::text(
            (x + 0.4, y + 0.4),
            font,
            Anchor::Start,
            false,
            &label.name,
            cl,
        ));
    }
    for port in schematic.power_ports.values().filter(|p| p.sheet == *sheet) {
        let (x, y) = point_mm(&port.at);
        ports.push(Item::line((x - 1.5, y), (x + 1.5, y), cp, 0.3));
        ports.push(Item::line((x, y), (x, y - 1.5), cp, 0.2));
        ports.push(Item::text(
            (x, y + 0.6),
            font,
            Anchor::Middle,
            false,
            &port.name,
            cp,
        ));
    }
    for port in schematic.sheet_ports.values().filter(|p| p.sheet == *sheet) {
        let (x, y) = point_mm(&port.at);
        ports.push(Item::polygon(
            vec![(x, y), (x - 1.5, y + 0.8), (x - 1.5, y - 0.8)],
            cp,
            0.2,
            true,
        ));
        ports.push(Item::text(
            (x + 0.8, y - 0.4),
            font,
            Anchor::Start,
            false,
            &port.name,
            cp,
        ));
    }
    for bus in schematic.buses.values().filter(|b| b.sheet == *sheet) {
        let (x, y) = point_mm(&bus.at);
        buses.push(Item::line((x, y), (x + 12.0, y), cb, 0.5));
        buses.push(Item::text(
            (x, y + 0.8),
            font,
            Anchor::Start,
            false,
            &bus.spec,
            cb,
        ));
    }
    for entry in schematic.bus_entries.values() {
        let on_sheet = schematic
            .buses
            .get(&entry.bus)
            .is_some_and(|b| b.sheet == *sheet);
        if on_sheet {
            let (x, y) = point_mm(&entry.at);
            buses.push(Item::line((x, y), (x + 1.5, y + 1.5), cb, 0.2));
            buses.push(Item::text(
                (x + 1.8, y + 1.0),
                font * 0.9,
                Anchor::Start,
                false,
                &entry.member,
                cb,
            ));
        }
    }
    let mut out = Vec::new();
    for (kind, items) in [
        (LayerKind::Junction, junctions),
        (LayerKind::Label, labels),
        (LayerKind::Port, ports),
        (LayerKind::Bus, buses),
    ] {
        if !items.is_empty() {
            out.push(Layer { kind, items });
        }
    }
    out
}

/// The pages of a schematic: one for each sheet, at the size of the sheet.
pub fn schematic_contents(
    schematic: &Schematic,
    scene: &Scene,
    settings: &ViewerSettings,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Vec<Content> {
    // the frames of the sheets, side by side in the scene
    let sheet_layer = scene.layer_of_role(Role::Sheet);
    let mut boxes: Vec<(String, Bounds)> = Vec::new();
    if let Some(index) = sheet_layer {
        for prim in scene.prims.iter().filter(|p| p.layer == index) {
            if let Shape::Polygon { points } = &prim.shape {
                let mut b = Bounds::EMPTY;
                for p in points {
                    b.include(*p);
                }
                let label = scene
                    .items
                    .get(prim.item as usize)
                    .map_or(String::new(), |i| i.label.clone());
                boxes.push((label, b));
            }
        }
    }
    if boxes.is_empty() {
        let mut all = Bounds::EMPTY;
        all.include(scene.bounds.min);
        all.include(scene.bounds.max);
        boxes.push((schematic.name.clone(), all));
    }
    let count = boxes.len();
    let sheet_ids: Vec<&SchId> = schematic.sheets.keys().collect();
    let mut contents = Vec::new();
    for (k, (name, frame)) in boxes.iter().enumerate() {
        let map = Map {
            mirror_sum: None,
            dx: frame.min.x,
        };
        let mut layers = scene_layers(
            scene,
            cfg,
            &map,
            |_| true,
            |p| sheet_of(rep_x(&p.shape), &boxes) == k,
        );
        if let Some(id) = sheet_ids.get(k).filter(|_| sheet_ids.len() == count) {
            layers.extend(sheet_extras(schematic, id, settings, cfg));
        }
        let title = if name.is_empty() {
            PrintText::Schematic.tr(lang).to_owned()
        } else {
            format!("{} - {}", PrintText::Schematic.tr(lang), name)
        };
        contents.push(Content {
            name: format!("sheet-{:02}", k + 1),
            project: project.to_owned(),
            title,
            sheet_no: k + 1,
            sheet_count: count,
            layers,
            dimension_box: Some(((0.0, frame.min.y), (frame.width(), frame.max.y))),
        });
    }
    contents
}

// ---------------------------------------------------------------------------------------------
// boards

fn outline_box(board: &Board, scene: &Scene) -> Bounds {
    let mut b = Bounds::EMPTY;
    for p in &board.outline {
        let (x, y) = point_mm(p);
        b.include(P::new(x, y));
    }
    if b.is_empty() {
        b = scene.bounds;
    }
    b
}

fn mirror_for(side: Side, outline: &Bounds) -> Map {
    Map {
        mirror_sum: (side == Side::Bottom).then_some(outline.min.x + outline.max.x),
        dx: 0.0,
    }
}

fn dimension_box(outline: &Bounds, map: &Map) -> Option<(Pt, Pt)> {
    if outline.is_empty() {
        return None;
    }
    let (a, b) = (map.pt(outline.min), map.pt(outline.max));
    Some(((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1))))
}

fn pad_half(shape: &PadShape) -> f64 {
    match shape {
        PadShape::Circle { diameter } => nm_mm(*diameter) / 2.0,
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            nm_mm(*width).max(nm_mm(*height)) / 2.0
        }
    }
}

/// The assembly plan of one side of a board.
fn assembly_content(
    board: &Board,
    scene: &Scene,
    side: Side,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Content {
    let outline = outline_box(board, scene);
    let map = mirror_for(side, &outline);
    let silk = if side == Side::Top {
        Role::SilkTop
    } else {
        Role::SilkBottom
    };
    let mut layers = scene_layers(
        scene,
        cfg,
        &map,
        |role| matches!(role, Role::Outline | Role::Drill) || role == silk,
        |_| true,
    );
    let (mut boxes, mut names, mut firsts, mut fiducials) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (c_asm, c_des, c_pin1, c_fid) = (
        cfg.colour(LayerKind::Assembly),
        cfg.colour(LayerKind::Designator),
        cfg.colour(LayerKind::Pin1),
        cfg.colour(LayerKind::Fiducial),
    );
    for (cid, component) in board.components.iter().filter(|(_, c)| c.side == side) {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let mut area = Bounds::EMPTY;
        let mut first: Option<P> = None;
        for (pid, pad) in &footprint.pads {
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            ) else {
                continue;
            };
            let (x, y) = point_mm(&at);
            let half = pad_half(&pad.shape);
            area.include(P::new(x - half, y - half));
            area.include(P::new(x + half, y + half));
            if pad.number == "1" {
                first = Some(P::new(x, y));
            }
        }
        if area.is_empty() {
            continue;
        }
        let centre = map.pt(area.centre());
        let is_fiducial = component.refdes.to_ascii_uppercase().starts_with("FID")
            || footprint.name.to_ascii_lowercase().contains("fiducial");
        if is_fiducial {
            let r = 0.5;
            fiducials.push(Item::circle(centre, r, c_fid, 0.12, false));
            fiducials.push(Item::line(
                (centre.0 - r * 1.6, centre.1),
                (centre.0 + r * 1.6, centre.1),
                c_fid,
                0.1,
            ));
            fiducials.push(Item::line(
                (centre.0, centre.1 - r * 1.6),
                (centre.0, centre.1 + r * 1.6),
                c_fid,
                0.1,
            ));
            continue;
        }
        let body = area.expanded(0.25);
        let corners = [
            P::new(body.min.x, body.min.y),
            P::new(body.max.x, body.min.y),
            P::new(body.max.x, body.max.y),
            P::new(body.min.x, body.max.y),
        ];
        boxes.push(Item::polygon(
            corners.iter().map(|c| map.pt(*c)).collect(),
            c_asm,
            0.12,
            false,
        ));
        let size = (body.width().min(body.height()) * 0.55).clamp(0.8, 2.0);
        names.push(Item::text(
            (centre.0, centre.1 - size * 0.35),
            size,
            Anchor::Middle,
            false,
            &component.refdes,
            c_des,
        ));
        if let (Some(p), true) = (first, footprint.pads.len() >= 2) {
            let at = map.pt(p);
            firsts.push(Item::circle(at, 0.55, c_pin1, 0.12, false));
            firsts.push(Item::circle(at, 0.2, c_pin1, 0.0, true));
        }
    }
    for (kind, items) in [
        (LayerKind::Assembly, boxes),
        (LayerKind::Fiducial, fiducials),
        (LayerKind::Designator, names),
        (LayerKind::Pin1, firsts),
    ] {
        if !items.is_empty() {
            layers.push(Layer { kind, items });
        }
    }
    let word = if side == Side::Top { "top" } else { "bottom" };
    Content {
        name: format!("assembly-{word}"),
        project: project.to_owned(),
        title: format!(
            "{} - {}",
            PrintText::Assembly.tr(lang),
            side_word(side, lang)
        ),
        sheet_no: 1,
        sheet_count: 1,
        layers,
        dimension_box: dimension_box(&outline, &map),
    }
}

/// The layers of a board one over the other, seen from one side.
fn layers_content(
    board: &Board,
    scene: &Scene,
    side: Side,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Content {
    let outline = outline_box(board, scene);
    let map = mirror_for(side, &outline);
    let layers = scene_layers(scene, cfg, &map, |_| true, |_| true);
    Content {
        name: "layers".to_owned(),
        project: project.to_owned(),
        title: format!("{} - {}", PrintText::Layers.tr(lang), side_word(side, lang)),
        sheet_no: 1,
        sheet_count: 1,
        layers,
        dimension_box: dimension_box(&outline, &map),
    }
}

/// The pages of a board: the assembly plan of each side that the settings ask for and the layers
/// seen from the top, or from the bottom when only the bottom side is asked for.
pub fn board_contents(
    board: &Board,
    scene: &Scene,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Vec<Content> {
    use eh3d_phoenix_print::config::SideChoice;
    let sides: Vec<Side> = match cfg.side {
        SideChoice::Top => vec![Side::Top],
        SideChoice::Bottom => vec![Side::Bottom],
        SideChoice::Both => vec![Side::Top, Side::Bottom],
    };
    let mut contents: Vec<Content> = sides
        .iter()
        .map(|side| assembly_content(board, scene, *side, cfg, lang, project))
        .collect();
    contents.push(layers_content(board, scene, sides[0], cfg, lang, project));
    let count = contents.len();
    for (i, content) in contents.iter_mut().enumerate() {
        content.sheet_no = i + 1;
        content.sheet_count = count;
    }
    contents
}

// ---------------------------------------------------------------------------------------------
// panels

fn panel_kind(kind: PanelLayerKind) -> LayerKind {
    match kind {
        PanelLayerKind::Frame => LayerKind::PanelFrame,
        PanelLayerKind::Boards => LayerKind::PanelBoards,
        PanelLayerKind::Outline => LayerKind::PanelOutline,
        PanelLayerKind::Tabs => LayerKind::PanelTabs,
        PanelLayerKind::VScore => LayerKind::PanelVScore,
        PanelLayerKind::MouseBites => LayerKind::PanelMouseBites,
        PanelLayerKind::Fiducials => LayerKind::PanelFiducials,
        PanelLayerKind::Tooling => LayerKind::PanelTooling,
        PanelLayerKind::Holes => LayerKind::PanelHoles,
    }
}

/// The page of a panel of boards.
pub fn panel_contents(
    scene: &PanelScene,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Vec<Content> {
    let mut layers = Vec::new();
    for layer in scene.layers.iter().filter(|l| l.visible) {
        let kind = panel_kind(layer.kind);
        let colour = cfg.colour(kind);
        let mut items: Vec<Item> = Vec::new();
        for shape in &layer.shapes {
            items.push(match shape {
                PanelShape::Rect { min, max, filled } => Item::rect(
                    (min.x, min.y),
                    (max.x, max.y),
                    colour,
                    if kind == LayerKind::PanelBoards {
                        0.15
                    } else {
                        0.1
                    },
                    *filled,
                ),
                PanelShape::Line { from, to } => {
                    let width = if kind == LayerKind::PanelVScore {
                        0.25
                    } else {
                        0.15
                    };
                    Item::line((from.x, from.y), (to.x, to.y), colour, width)
                }
                PanelShape::Circle {
                    centre,
                    radius,
                    filled,
                } => Item::circle((centre.x, centre.y), *radius, colour, 0.1, *filled),
                PanelShape::Label { at, text } => {
                    Item::text((at.x, at.y), 2.5, Anchor::Start, false, text, colour)
                }
            });
        }
        if kind == LayerKind::PanelBoards {
            for (i, copy) in scene.copies.iter().enumerate() {
                let (w, h) = (copy.max.x - copy.min.x, copy.max.y - copy.min.y);
                let size = (w.min(h) * 0.2).clamp(1.5, 6.0);
                items.push(Item::text(
                    (
                        (copy.min.x + copy.max.x) / 2.0,
                        (copy.min.y + copy.max.y) / 2.0 - size * 0.35,
                    ),
                    size,
                    Anchor::Middle,
                    false,
                    &(i + 1).to_string(),
                    colour,
                ));
            }
        }
        if !items.is_empty() {
            layers.push(Layer { kind, items });
        }
    }
    vec![Content {
        name: "panel".to_owned(),
        project: project.to_owned(),
        title: format!(
            "{} - {}: {}",
            PrintText::Panel.tr(lang),
            PrintText::Copies.tr(lang),
            scene.copies.len()
        ),
        sheet_no: 1,
        sheet_count: 1,
        layers,
        dimension_box: Some(((0.0, 0.0), (scene.width, scene.height))),
    }]
}

/// The pages of a document that was read.
pub fn document_contents(
    document: &Document,
    scene: &Scene,
    settings: &ViewerSettings,
    cfg: &PrintConfig,
    lang: Lang,
    project: &str,
) -> Vec<Content> {
    match document {
        Document::Board(board) => board_contents(board, scene, cfg, lang, project),
        Document::Schematic(schematic) => {
            schematic_contents(schematic, scene, settings, cfg, lang, project)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use eh3d_phoenix_print::{render, Format};

    fn scene_of_board() -> (Board, Scene) {
        let board = sample::board();
        let scene = Scene::from_board(&board, &ViewerSettings::default()).unwrap();
        (board, scene)
    }

    #[test]
    fn a_board_gives_an_assembly_plan_for_each_side_and_its_layers() {
        let (board, scene) = scene_of_board();
        let cfg = PrintConfig::default();
        let contents = board_contents(&board, &scene, &cfg, Lang::En, "demo");
        let names: Vec<&str> = contents.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["assembly-top", "assembly-bottom", "layers"]);
        assert_eq!(contents[0].sheet_count, 3);
        assert_eq!(contents[2].sheet_no, 3);
        assert_eq!(contents[0].title, "Assembly plan - Top");
        let top = &contents[0];
        let has = |kind: LayerKind| top.layers.iter().any(|l| l.kind == kind);
        assert!(has(LayerKind::Outline));
        assert!(has(LayerKind::Assembly));
        assert!(has(LayerKind::Designator));
        assert!(top.dimension_box.is_some());
    }

    #[test]
    fn the_designators_of_the_board_are_on_the_plan_of_their_side() {
        let (board, scene) = scene_of_board();
        let cfg = PrintConfig::default();
        let contents = board_contents(&board, &scene, &cfg, Lang::En, "demo");
        let texts = |c: &Content| -> Vec<String> {
            c.layers
                .iter()
                .filter(|l| l.kind == LayerKind::Designator)
                .flat_map(|l| l.items.iter())
                .filter_map(|i| match &i.shape {
                    eh3d_phoenix_print::Shape::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let on_top = board
            .components
            .values()
            .filter(|c| c.side == Side::Top)
            .count();
        let on_bottom = board
            .components
            .values()
            .filter(|c| c.side == Side::Bottom)
            .count();
        assert_eq!(texts(&contents[0]).len(), on_top);
        assert_eq!(texts(&contents[1]).len(), on_bottom);
    }

    #[test]
    fn the_bottom_side_is_seen_in_a_mirror() {
        let (board, scene) = scene_of_board();
        let outline = outline_box(&board, &scene);
        let top = mirror_for(Side::Top, &outline);
        let bottom = mirror_for(Side::Bottom, &outline);
        let p = P::new(outline.min.x, 1.0);
        assert_eq!(top.pt(p), (outline.min.x, 1.0));
        assert_eq!(bottom.pt(p), (outline.max.x, 1.0));
    }

    #[test]
    fn the_pages_of_a_board_are_rendered_to_svg_and_pdf() {
        let (board, scene) = scene_of_board();
        let cfg = PrintConfig::default();
        let contents = board_contents(&board, &scene, &cfg, Lang::De, "demo");
        let svg = render("demo", &contents, Format::Svg, &cfg, Lang::De).unwrap();
        assert_eq!(svg.files.len(), 3);
        let pdf = render("demo", &contents, Format::Pdf, &cfg, Lang::De).unwrap();
        assert_eq!(pdf.files.len(), 1);
        assert_eq!(pdf.files[0].pages, 3);
    }

    #[test]
    fn a_schematic_gives_a_page_for_each_sheet_with_its_marks() {
        let schematic = sample::schematic();
        let settings = ViewerSettings::default();
        let scene = Scene::from_schematic(&schematic, &settings).unwrap();
        let cfg = PrintConfig::default();
        let contents = schematic_contents(&schematic, &scene, &settings, &cfg, Lang::Es, "demo");
        assert_eq!(contents.len(), schematic.sheets.len().max(1));
        assert_eq!(contents[0].name, "sheet-01");
        assert!(contents[0].title.starts_with("Esquema"));
        let items: usize = contents[0].layers.iter().map(|l| l.items.len()).sum();
        assert!(items > 0);
        let svg = render("demo", &contents, Format::Svg, &cfg, Lang::Es).unwrap();
        assert_eq!(svg.files.len(), contents.len());
    }

    #[test]
    fn the_sheet_of_a_point_is_told_by_its_x() {
        let mut a = Bounds::EMPTY;
        a.include(P::new(0.0, 0.0));
        a.include(P::new(10.0, 5.0));
        let mut b = Bounds::EMPTY;
        b.include(P::new(20.0, 0.0));
        b.include(P::new(30.0, 5.0));
        let boxes = vec![("a".to_owned(), a), ("b".to_owned(), b)];
        assert_eq!(sheet_of(5.0, &boxes), 0);
        assert_eq!(sheet_of(25.0, &boxes), 1);
        assert_eq!(sheet_of(11.0, &boxes), 0);
        assert_eq!(sheet_of(19.0, &boxes), 1);
        assert_eq!(sheet_of(-50.0, &boxes), 0);
    }
}
