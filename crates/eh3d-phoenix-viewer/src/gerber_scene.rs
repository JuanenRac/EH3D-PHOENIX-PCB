// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene of imported Gerber and Excellon files, as data in millimetres and with no window: one
//! layer for each file (copper, solder mask, paste, silkscreen, outline, drills), every object of
//! it turned into a filled polygon that is dark or clear, the colour of each layer by its role,
//! the visibility, the order and the opacity of the layers, the snap of a click to a corner or a
//! centre, and the comparison of a layer with the same layer of a board of the project.
//!
//! The scene is read-only: the files are read by `eh3d-phoenix-gerber` and nothing is written back.
//! A line, an arc and an aperture are filled shapes (a line is a capsule, an arc a curved strip),
//! a region is its polygon, and a macro keeps the exposure of each of its primitives. The polarity
//! of an object (`LPD` dark, `LPC` clear) is kept in the order of the file, so that a clear object
//! takes away what the dark ones before it drew. A thermal or a moire primitive, a step and repeat
//! and an aperture block are not read by the import and so are not drawn; the layer says so.

use std::f64::consts::{PI, TAU};
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_gerber as gbr;
use eh3d_phoenix_gerber::{
    Aperture, ApertureShape, Content, DrillFile, GerberLayer, ImportSettings, ImportedFile,
    ImportedSet, Item, MacroShape, Role,
};
use eh3d_phoenix_rules::Lang;

use crate::codes::{ViewError, FILE_NOT_RECOGNISED, FILE_UNUSABLE, NOTHING_TO_DRAW};
use crate::load::{read_document, Document};
use crate::prim::{triangulate, Bounds, P};
use crate::settings::ViewerSettings;

const NM_PER_MM: f64 = 1_000_000.0;

fn mm(nm: i64) -> f64 {
    nm as f64 / NM_PER_MM
}

fn micro_degrees(value: i64) -> f64 {
    value as f64 / 1_000_000.0
}

fn pt(p: gbr::P) -> P {
    P::new(mm(p.x), mm(p.y))
}

/// What a layer is for, as far as the drawing is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// Copper.
    Copper,
    /// Solder mask.
    Mask,
    /// Solder paste.
    Paste,
    /// Silkscreen.
    Legend,
    /// The board outline.
    Outline,
    /// Holes.
    Drill,
    /// A file whose layer could not be told.
    Unknown,
}

/// The kind of a layer from its role.
pub fn kind_of(role: Option<Role>) -> LayerKind {
    match role {
        Some(Role::CopperTop | Role::CopperBottom | Role::CopperInner(_)) => LayerKind::Copper,
        Some(Role::MaskTop | Role::MaskBottom) => LayerKind::Mask,
        Some(Role::PasteTop | Role::PasteBottom) => LayerKind::Paste,
        Some(Role::LegendTop | Role::LegendBottom) => LayerKind::Legend,
        Some(Role::Outline) => LayerKind::Outline,
        Some(Role::DrillPlated | Role::DrillNonPlated | Role::Drill) => LayerKind::Drill,
        None => LayerKind::Unknown,
    }
}

/// The colour of a layer by its role, from the settings.
pub fn role_colour(role: Option<Role>, settings: &ViewerSettings) -> [u8; 4] {
    let name = match role {
        Some(Role::CopperTop) => "top_copper",
        Some(Role::CopperBottom) => "bottom_copper",
        Some(Role::CopperInner(n)) => match n.max(1) % 4 {
            1 => "inner_copper_1",
            2 => "inner_copper_2",
            3 => "inner_copper_3",
            _ => "inner_copper_4",
        },
        Some(Role::MaskTop | Role::MaskBottom) => "mask",
        Some(Role::PasteTop | Role::PasteBottom) => "paste",
        Some(Role::LegendTop) => "silk_top",
        Some(Role::LegendBottom) => "silk_bottom",
        Some(Role::Outline) => "outline",
        Some(Role::DrillPlated | Role::Drill) => "gerber_drill_plated",
        Some(Role::DrillNonPlated) => "gerber_drill_unplated",
        None => "documentation",
    };
    settings.colour(name)
}

/// Where a layer is drawn in the stack, the lowest first: the bottom of the board, the inner
/// layers, the top, then the mask, the paste, the silkscreen, the holes and the outline.
pub fn stack_rank(role: Option<Role>) -> u32 {
    match role {
        None => 5,
        Some(Role::CopperBottom) => 10,
        Some(Role::CopperInner(n)) => 20 + n,
        Some(Role::CopperTop) => 100,
        Some(Role::MaskBottom) => 110,
        Some(Role::MaskTop) => 111,
        Some(Role::PasteBottom) => 120,
        Some(Role::PasteTop) => 121,
        Some(Role::LegendBottom) => 130,
        Some(Role::LegendTop) => 131,
        Some(Role::DrillPlated) => 140,
        Some(Role::DrillNonPlated) => 141,
        Some(Role::Drill) => 142,
        Some(Role::Outline) => 150,
    }
}

/// A filled polygon of a layer, in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct GPoly {
    /// The corners, the last one not repeated.
    pub points: Vec<P>,
    /// True for dark polarity (it adds), false for clear (it takes away).
    pub dark: bool,
    /// The triangles that fill it (indexes into `points`); empty when it is too large to fill.
    pub tris: Vec<[u32; 3]>,
    /// The centre of a flash or a hole, or the two ends of a line, an arc or a slot.
    pub anchors: [Option<P>; 2],
    /// True for a region: its corners are places a click snaps to.
    pub region: bool,
    /// The box around the corners.
    pub bounds: Bounds,
}

fn is_convex(points: &[P]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let after = points.iter().cycle().skip(1);
    let then = points.iter().cycle().skip(2);
    let mut sign = 0.0_f64;
    for ((a, b), c) in points.iter().zip(after).zip(then) {
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() < 1e-12 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if sign != cross.signum() {
            return false;
        }
    }
    sign != 0.0
}

impl GPoly {
    /// A polygon with its triangles; one with more corners than `max_fill` is not triangulated.
    pub fn new(
        points: Vec<P>,
        dark: bool,
        anchors: [Option<P>; 2],
        region: bool,
        max_fill: usize,
    ) -> GPoly {
        let mut bounds = Bounds::EMPTY;
        for p in &points {
            bounds.include(*p);
        }
        let tris = if points.len() > max_fill {
            Vec::new()
        } else if is_convex(&points) {
            (1..points.len() as u32 - 1)
                .map(|i| [0, i, i + 1])
                .collect()
        } else {
            triangulate(&points)
        };
        GPoly {
            points,
            dark,
            tris,
            anchors,
            region,
            bounds,
        }
    }
}

/// The corners of a circle, counter-clockwise.
pub fn circle_polygon(centre: P, radius: f64, sides: usize) -> Vec<P> {
    let n = sides.max(3);
    (0..n)
        .map(|i| {
            let angle = TAU * (i as f64) / (n as f64);
            P::new(
                centre.x + radius * angle.cos(),
                centre.y + radius * angle.sin(),
            )
        })
        .collect()
}

/// The corners of a rectangle around a centre, counter-clockwise.
pub fn rect_polygon(centre: P, width: f64, height: f64) -> Vec<P> {
    let (hw, hh) = (width / 2.0, height / 2.0);
    vec![
        P::new(centre.x - hw, centre.y - hh),
        P::new(centre.x + hw, centre.y - hh),
        P::new(centre.x + hw, centre.y + hh),
        P::new(centre.x - hw, centre.y + hh),
    ]
}

/// The corners of a regular polygon, counter-clockwise, the first corner turned by `rotation_deg`
/// from the positive x axis.
pub fn regular_polygon(centre: P, diameter: f64, vertices: u32, rotation_deg: f64) -> Vec<P> {
    let n = vertices.max(3) as usize;
    (0..n)
        .map(|i| {
            let angle = rotation_deg.to_radians() + TAU * (i as f64) / (n as f64);
            P::new(
                centre.x + diameter / 2.0 * angle.cos(),
                centre.y + diameter / 2.0 * angle.sin(),
            )
        })
        .collect()
}

/// The outline of a line with round ends from `a` to `b`: a capsule, counter-clockwise. `sides` is
/// the number of sides of a full circle.
pub fn capsule(a: P, b: P, radius: f64, sides: usize) -> Vec<P> {
    let half = (sides / 2).max(2);
    let theta = if a == b {
        0.0
    } else {
        (b.y - a.y).atan2(b.x - a.x)
    };
    let mut out = Vec::with_capacity(2 * half + 2);
    for i in 0..=half {
        let angle = theta - PI / 2.0 + PI * (i as f64) / (half as f64);
        out.push(P::new(
            b.x + radius * angle.cos(),
            b.y + radius * angle.sin(),
        ));
    }
    for i in 0..=half {
        let angle = theta + PI / 2.0 + PI * (i as f64) / (half as f64);
        out.push(P::new(
            a.x + radius * angle.cos(),
            a.y + radius * angle.sin(),
        ));
    }
    out
}

/// The outline of an arc drawn with a round aperture of width `width`: the strip around the arc
/// with a half circle at each end. `start` and `sweep` are in radians (a negative sweep runs
/// clockwise) and `step` is the angle of one straight piece.
pub fn arc_stroke(
    centre: P,
    radius: f64,
    start: f64,
    sweep: f64,
    width: f64,
    step: f64,
    sides: usize,
) -> Vec<P> {
    let half = width / 2.0;
    let sign = if sweep < 0.0 { -1.0 } else { 1.0 };
    let pieces = ((sweep.abs() / step.max(1e-3)).ceil() as usize).max(1);
    let cap = (sides / 2).max(2);
    let at = |angle: f64, r: f64| P::new(centre.x + r * angle.cos(), centre.y + r * angle.sin());
    let outer = radius + half;
    let inner = (radius - half).max(0.0);
    let end = start + sweep;
    let mut points = Vec::with_capacity(2 * pieces + 2 * cap + 4);
    for i in 0..=pieces {
        points.push(at(start + sweep * (i as f64) / (pieces as f64), outer));
    }
    let tip = at(end, radius);
    for i in 1..cap {
        let angle = end + sign * PI * (i as f64) / (cap as f64);
        points.push(P::new(
            tip.x + half * angle.cos(),
            tip.y + half * angle.sin(),
        ));
    }
    for i in (0..=pieces).rev() {
        points.push(at(start + sweep * (i as f64) / (pieces as f64), inner));
    }
    let root = at(start, radius);
    for i in 1..cap {
        let angle = start + sign * (PI + PI * (i as f64) / (cap as f64));
        points.push(P::new(
            root.x + half * angle.cos(),
            root.y + half * angle.sin(),
        ));
    }
    points
}

fn turn(p: P, degrees: f64) -> P {
    let (s, c) = degrees.to_radians().sin_cos();
    P::new(p.x * c - p.y * s, p.x * s + p.y * c)
}

/// A layer of the scene: the objects of one file.
#[derive(Debug, Clone, PartialEq)]
pub struct GLayer {
    /// The name of the file.
    pub name: String,
    /// The role of the file, when it could be told.
    pub role: Option<Role>,
    /// What the layer is for.
    pub kind: LayerKind,
    /// The polygons, in the order of the file.
    pub polys: Vec<GPoly>,
    /// The box around the polygons.
    pub bounds: Bounds,
    /// Whether the layer is drawn.
    pub visible: bool,
    /// The opacity, above 0 up to 1.
    pub opacity: f64,
    /// The colour, red, green, blue and alpha.
    pub colour: [u8; 4],
    /// The objects of dark polarity of the file.
    pub dark: usize,
    /// The objects of clear polarity of the file.
    pub clear: usize,
    /// The round holes of a drill file.
    pub holes: usize,
    /// The slots of a drill file.
    pub slots: usize,
    /// Why the file could not be read, when it could not.
    pub error: Option<String>,
    /// True when the import skipped something that changes the picture.
    pub incomplete: bool,
    /// The number of findings of the import.
    pub findings: usize,
    /// True when the layer was cut at the limit of polygons.
    pub truncated: bool,
}

impl GLayer {
    /// The colour with the opacity of the layer applied to its alpha.
    pub fn draw_colour(&self) -> [u8; 4] {
        let alpha = (f64::from(self.colour[3]) * self.opacity)
            .round()
            .clamp(0.0, 255.0) as u8;
        [self.colour[0], self.colour[1], self.colour[2], alpha]
    }

    /// True when the layer has something to draw.
    pub fn is_drawable(&self) -> bool {
        self.error.is_none() && !self.polys.is_empty()
    }
}

struct Builder<'a> {
    settings: &'a ViewerSettings,
    limit: usize,
    truncated: bool,
    polys: Vec<GPoly>,
}

impl Builder<'_> {
    fn push(&mut self, points: Vec<P>, dark: bool, anchors: [Option<P>; 2], region: bool) {
        if points.len() < 3 {
            return;
        }
        if self.polys.len() >= self.limit {
            self.truncated = true;
            return;
        }
        self.polys.push(GPoly::new(
            points,
            dark,
            anchors,
            region,
            self.settings.max_fill_points,
        ));
    }

    fn width(&self, nm: i64) -> f64 {
        if nm > 0 {
            mm(nm)
        } else {
            self.settings.gerber_hairline_mm
        }
    }

    fn sides(&self) -> usize {
        self.settings.gerber_circle_segments
    }

    fn flash(&mut self, aperture: &Aperture, at: P, dark: bool) {
        let sides = self.sides();
        let anchors = [Some(at), None];
        match &aperture.shape {
            ApertureShape::Circle { diameter } => {
                self.push(
                    circle_polygon(at, mm(*diameter) / 2.0, sides),
                    dark,
                    anchors,
                    false,
                );
            }
            ApertureShape::Rect { width, height } => {
                self.push(
                    rect_polygon(at, mm(*width), mm(*height)),
                    dark,
                    anchors,
                    false,
                );
            }
            ApertureShape::Obround { width, height } => {
                let (w, h) = (mm(*width), mm(*height));
                let points = if w >= h {
                    let d = (w - h) / 2.0;
                    capsule(
                        P::new(at.x - d, at.y),
                        P::new(at.x + d, at.y),
                        h / 2.0,
                        sides,
                    )
                } else {
                    let d = (h - w) / 2.0;
                    capsule(
                        P::new(at.x, at.y - d),
                        P::new(at.x, at.y + d),
                        w / 2.0,
                        sides,
                    )
                };
                self.push(points, dark, anchors, false);
            }
            ApertureShape::Polygon {
                diameter,
                vertices,
                rotation,
            } => {
                self.push(
                    regular_polygon(at, mm(*diameter), *vertices, micro_degrees(*rotation)),
                    dark,
                    anchors,
                    false,
                );
            }
            ApertureShape::Macro { shapes, .. } => {
                for shape in shapes {
                    self.macro_shape(shape, at, dark);
                }
            }
        }
        if dark {
            if let Some(hole) = aperture.hole.filter(|h| *h > 0) {
                self.push(
                    circle_polygon(at, mm(hole) / 2.0, sides),
                    false,
                    [None, None],
                    false,
                );
            }
        }
    }

    fn macro_shape(&mut self, shape: &MacroShape, at: P, flash_dark: bool) {
        let sides = self.sides();
        let (exposure, points): (bool, Vec<P>) = match shape {
            MacroShape::Circle {
                exposure,
                diameter,
                centre,
                rotation,
            } => {
                let c = turn(pt(*centre), micro_degrees(*rotation));
                (*exposure, circle_polygon(c, mm(*diameter) / 2.0, sides))
            }
            MacroShape::VectorLine {
                exposure,
                width,
                start,
                end,
                rotation,
            } => {
                let (a, b) = (pt(*start), pt(*end));
                let half = mm(*width) / 2.0;
                let length = a.dist(b);
                let (ux, uy) = if length > 0.0 {
                    ((b.x - a.x) / length, (b.y - a.y) / length)
                } else {
                    (1.0, 0.0)
                };
                let (nx, ny) = (-uy * half, ux * half);
                let r = micro_degrees(*rotation);
                let corners = [
                    P::new(a.x + nx, a.y + ny),
                    P::new(b.x + nx, b.y + ny),
                    P::new(b.x - nx, b.y - ny),
                    P::new(a.x - nx, a.y - ny),
                ];
                (*exposure, corners.iter().map(|p| turn(*p, r)).collect())
            }
            MacroShape::CenterLine {
                exposure,
                width,
                height,
                centre,
                rotation,
            } => {
                let r = micro_degrees(*rotation);
                let corners = rect_polygon(pt(*centre), mm(*width), mm(*height));
                (*exposure, corners.iter().map(|p| turn(*p, r)).collect())
            }
            MacroShape::Outline {
                exposure,
                points,
                rotation,
            } => {
                let r = micro_degrees(*rotation);
                let mut corners: Vec<P> = points.iter().map(|p| turn(pt(*p), r)).collect();
                if corners.len() > 1 && corners.first() == corners.last() {
                    corners.pop();
                }
                (*exposure, corners)
            }
            MacroShape::Polygon {
                exposure,
                vertices,
                centre,
                diameter,
                rotation,
            } => {
                let r = micro_degrees(*rotation);
                let corners = regular_polygon(pt(*centre), mm(*diameter), *vertices, 0.0);
                (*exposure, corners.iter().map(|p| turn(*p, r)).collect())
            }
        };
        // a primitive that takes away from a macro flashed clear has nothing left to take
        let dark = if exposure {
            flash_dark
        } else if flash_dark {
            false
        } else {
            return;
        };
        let moved = points
            .into_iter()
            .map(|p| P::new(p.x + at.x, p.y + at.y))
            .collect();
        self.push(moved, dark, [None, None], false);
    }

    fn line(&mut self, from: P, to: P, width: i64, dark: bool) {
        let radius = self.width(width) / 2.0;
        let sides = self.sides();
        self.push(
            capsule(from, to, radius, sides),
            dark,
            [Some(from), Some(to)],
            false,
        );
    }

    fn arc(&mut self, from: P, to: P, centre: P, clockwise: bool, width: i64, dark: bool) {
        let radius = centre.dist(from);
        let start = (from.y - centre.y).atan2(from.x - centre.x);
        let end = (to.y - centre.y).atan2(to.x - centre.x);
        let ccw = (end - start).rem_euclid(TAU);
        let full = ccw < 1e-9;
        let sweep = match (clockwise, full) {
            (true, true) => -TAU,
            (true, false) => ccw - TAU,
            (false, true) => TAU,
            (false, false) => ccw,
        };
        let step = self.settings.gerber_arc_step_deg.to_radians();
        let stroke = arc_stroke(
            centre,
            radius,
            start,
            sweep,
            self.width(width),
            step,
            self.sides(),
        );
        self.push(stroke, dark, [Some(from), Some(to)], false);
    }

    fn gerber(&mut self, layer: &GerberLayer) {
        for item in &layer.items {
            match item {
                Item::Flash { at, aperture, dark } => {
                    if let Some(ap) = layer.apertures.get(aperture) {
                        self.flash(ap, pt(*at), *dark);
                    }
                }
                Item::Line {
                    from,
                    to,
                    width,
                    dark,
                } => self.line(pt(*from), pt(*to), *width, *dark),
                Item::Arc {
                    from,
                    to,
                    centre,
                    clockwise,
                    width,
                    dark,
                } => self.arc(pt(*from), pt(*to), pt(*centre), *clockwise, *width, *dark),
                Item::Region { points, dark } => {
                    let mut corners: Vec<P> = points.iter().map(|p| pt(*p)).collect();
                    if corners.len() > 1 && corners.first() == corners.last() {
                        corners.pop();
                    }
                    self.push(corners, *dark, [None, None], true);
                }
            }
        }
    }

    fn drill(&mut self, file: &DrillFile) {
        let sides = self.sides();
        for hit in &file.hits {
            let at = pt(hit.at);
            let diameter = self.width(file.diameter(hit.tool));
            self.push(
                circle_polygon(at, diameter / 2.0, sides),
                true,
                [Some(at), None],
                false,
            );
        }
        for slot in &file.slots {
            let (a, b) = (pt(slot.from), pt(slot.to));
            let diameter = self.width(file.diameter(slot.tool));
            self.push(
                capsule(a, b, diameter / 2.0, sides),
                true,
                [Some(a), Some(b)],
                false,
            );
        }
    }
}

fn layer_of(file: &ImportedFile, settings: &ViewerSettings, remaining: &mut usize) -> GLayer {
    let role = file.identified.map(|i| i.role);
    let mut layer = GLayer {
        name: file.name.clone(),
        role,
        kind: kind_of(role),
        polys: Vec::new(),
        bounds: Bounds::EMPTY,
        visible: true,
        opacity: settings.gerber_default_opacity,
        colour: role_colour(role, settings),
        dark: 0,
        clear: 0,
        holes: 0,
        slots: 0,
        error: None,
        incomplete: false,
        findings: file.findings().len(),
        truncated: false,
    };
    let mut builder = Builder {
        settings,
        limit: *remaining,
        truncated: false,
        polys: Vec::new(),
    };
    match &file.content {
        Content::Gerber(gerber) => {
            builder.gerber(gerber);
            layer.incomplete = !gerber.complete;
            layer.dark = gerber.items.iter().filter(|i| item_is_dark(i)).count();
            layer.clear = gerber.items.len() - layer.dark;
        }
        Content::Drill(drill) => {
            builder.drill(drill);
            layer.holes = drill.hits.len();
            layer.slots = drill.slots.len();
            layer.dark = layer.holes + layer.slots;
        }
        Content::Failed(error) => {
            layer.error = Some(error.message(Lang::En));
            layer.visible = false;
        }
    }
    *remaining = remaining.saturating_sub(builder.polys.len());
    layer.truncated = builder.truncated;
    for poly in &builder.polys {
        layer.bounds = layer.bounds.union(poly.bounds);
    }
    layer.polys = builder.polys;
    layer
}

fn item_is_dark(item: &Item) -> bool {
    match item {
        Item::Flash { dark, .. }
        | Item::Line { dark, .. }
        | Item::Arc { dark, .. }
        | Item::Region { dark, .. } => *dark,
    }
}

/// The imported files as layers of polygons.
#[derive(Debug, Clone, PartialEq)]
pub struct GerberScene {
    /// The layers, the first one drawn first (at the back).
    pub layers: Vec<GLayer>,
    /// True when the set was cut at the limit of polygons.
    pub truncated: bool,
}

fn closer(best: &mut Option<(f64, P)>, from: P, candidate: P, radius: f64) {
    let distance = candidate.dist(from);
    if distance <= radius && best.is_none_or(|(d, _)| distance < d) {
        *best = Some((distance, candidate));
    }
}

impl GerberScene {
    /// The scene of a set of imported files, the layers in the order of the stack of a board.
    pub fn from_set(set: &ImportedSet, settings: &ViewerSettings) -> GerberScene {
        let mut remaining = settings.gerber_max_polys;
        let mut layers: Vec<GLayer> = set
            .files
            .iter()
            .map(|file| layer_of(file, settings, &mut remaining))
            .collect();
        layers.sort_by_key(|l| stack_rank(l.role));
        let truncated = layers.iter().any(|l| l.truncated);
        GerberScene { layers, truncated }
    }

    /// The box around everything that can be drawn.
    pub fn bounds(&self) -> Bounds {
        self.layers
            .iter()
            .filter(|l| l.is_drawable())
            .fold(Bounds::EMPTY, |all, l| all.union(l.bounds))
    }

    /// The layer with a role, when there is one.
    pub fn layer_by_role(&self, role: Role) -> Option<&GLayer> {
        self.layers.iter().find(|l| l.role == Some(role))
    }

    /// Shows or hides a layer.
    pub fn set_visible(&mut self, index: usize, visible: bool) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.visible = visible && layer.error.is_none();
        }
    }

    /// Shows or hides every layer that can be drawn.
    pub fn set_all_visible(&mut self, visible: bool) {
        for layer in &mut self.layers {
            layer.visible = visible && layer.error.is_none();
        }
    }

    /// Sets the opacity of a layer, kept between 0.01 and 1.
    pub fn set_opacity(&mut self, index: usize, opacity: f64) {
        if let Some(layer) = self.layers.get_mut(index) {
            if opacity.is_finite() {
                layer.opacity = opacity.clamp(0.01, 1.0);
            }
        }
    }

    /// Draws a layer later, over the one that follows it in the list; false at the end.
    pub fn move_up(&mut self, index: usize) -> bool {
        if index + 1 < self.layers.len() {
            self.layers.swap(index, index + 1);
            true
        } else {
            false
        }
    }

    /// Draws a layer earlier, under the one before it in the list; false at the start.
    pub fn move_down(&mut self, index: usize) -> bool {
        if index > 0 && index < self.layers.len() {
            self.layers.swap(index, index - 1);
            true
        } else {
            false
        }
    }

    /// The corner or centre of a visible layer nearest to a point and within `radius`
    /// millimetres of it: the centre of a flash or a hole, the ends of a line, an arc or a slot
    /// and the corners of a region.
    pub fn snap(&self, p: P, radius: f64) -> Option<P> {
        let mut best: Option<(f64, P)> = None;
        for layer in self
            .layers
            .iter()
            .filter(|l| l.visible && l.error.is_none())
        {
            if !layer.bounds.expanded(radius).contains(p) {
                continue;
            }
            for poly in &layer.polys {
                if !poly.bounds.expanded(radius).contains(p) {
                    continue;
                }
                for anchor in poly.anchors.iter().flatten() {
                    closer(&mut best, p, *anchor, radius);
                }
                if poly.region {
                    for corner in &poly.points {
                        closer(&mut best, p, *corner, radius);
                    }
                }
            }
        }
        best.map(|(_, q)| q)
    }
}

/// The files of a folder, read for the import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRead {
    /// The name and the bytes of each file, in the order of the names.
    pub files: Vec<(String, Vec<u8>)>,
    /// The files left aside: pictures, documents and the like, files too large and the ones over
    /// the limit of files.
    pub skipped: usize,
}

const SKIPPED_EXTENSIONS: [&str; 13] = [
    "txt", "md", "pdf", "zip", "csv", "json", "png", "jpg", "jpeg", "html", "log", "rpt", "svg",
];

/// Reads the files of a folder (not the folders in it), leaving aside the ones that cannot be
/// Gerber or Excellon.
pub fn read_folder(dir: &Path, limits: &ImportSettings) -> Result<FolderRead, ViewError> {
    let unusable =
        |e: std::io::Error| ViewError::new(FILE_UNUSABLE, format!("{}: {e}", dir.display()));
    let entries = std::fs::read_dir(dir).map_err(unusable)?;
    let mut names: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            names.push((entry.file_name().to_string_lossy().into_owned(), path));
        }
    }
    names.sort();
    let mut files = Vec::new();
    let mut skipped = 0;
    for (name, path) in names {
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if SKIPPED_EXTENSIONS.contains(&extension.as_str()) || files.len() >= limits.max_files {
            skipped += 1;
            continue;
        }
        let size = std::fs::metadata(&path).map_or(0, |m| m.len());
        if size > limits.max_file_bytes as u64 {
            skipped += 1;
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => files.push((name, bytes)),
            Err(_) => skipped += 1,
        }
    }
    Ok(FolderRead { files, skipped })
}

/// Reads and draws the Gerber and Excellon files of a folder. The second value is the number of
/// files left aside.
pub fn open_folder(
    dir: &Path,
    settings: &ViewerSettings,
) -> Result<(GerberScene, usize), ViewError> {
    let limits = ImportSettings::default();
    let read = read_folder(dir, &limits)?;
    if read.files.is_empty() {
        return Err(ViewError::new(
            NOTHING_TO_DRAW,
            format!("{}: no Gerber or Excellon file", dir.display()),
        ));
    }
    let set = gbr::import_files(&read.files, &limits)
        .map_err(|e| ViewError::new(FILE_UNUSABLE, e.message(Lang::En)))?;
    Ok((GerberScene::from_set(&set, settings), read.skipped))
}

/// The board of a file (EAGLE or the JSON of the model), for the comparison.
pub fn load_board(path: &Path, settings: &ViewerSettings) -> Result<Board, ViewError> {
    match read_document(path, settings)?.document {
        Document::Board(board) => Ok(*board),
        Document::Schematic(_) => Err(ViewError::new(
            FILE_NOT_RECOGNISED,
            format!("{}: a schematic, not a board", path.display()),
        )),
    }
}

/// The layers that the export of this suite writes for a board, read back by the import and
/// drawn as a scene, to compare with a set of files.
pub fn board_scene(board: &Board, settings: &ViewerSettings) -> Result<GerberScene, ViewError> {
    let trip = gbr::round_trip(
        board,
        &eh3d_phoenix_export::Profile::default(),
        &ImportSettings::default(),
    )
    .map_err(|e| ViewError::new(FILE_UNUSABLE, e.message(Lang::En)))?;
    Ok(GerberScene::from_set(&trip.imported, settings))
}

/// A grid of cells that are covered or not, to compare two layers.
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    /// The corner of the first cell.
    pub origin: P,
    /// The side of a cell, in millimetres.
    pub cell: f64,
    /// The number of columns.
    pub width: usize,
    /// The number of rows.
    pub height: usize,
    cells: Vec<bool>,
}

impl Raster {
    /// A grid over a box with cells of about `cell` millimetres: the cell grows until the grid
    /// has no more than `max_cells` cells. A box that holds nothing gives a grid of no cells.
    pub fn new(bounds: Bounds, cell: f64, max_cells: usize) -> Raster {
        if bounds.is_empty() {
            return Raster {
                origin: P::new(0.0, 0.0),
                cell: cell.max(1e-6),
                width: 0,
                height: 0,
                cells: Vec::new(),
            };
        }
        let mut side = cell.max(1e-6);
        let size = |side: f64| {
            (
                (bounds.width() / side).ceil() as usize + 1,
                (bounds.height() / side).ceil() as usize + 1,
            )
        };
        let (mut width, mut height) = size(side);
        while width.saturating_mul(height) > max_cells.max(1) {
            side *= 1.5;
            (width, height) = size(side);
        }
        Raster {
            origin: bounds.min,
            cell: side,
            width,
            height,
            cells: vec![false; width * height],
        }
    }

    /// An empty grid with the same cells.
    pub fn like(&self) -> Raster {
        Raster {
            cells: vec![false; self.cells.len()],
            ..self.clone()
        }
    }

    /// Whether a cell is covered; a cell outside the grid is not.
    pub fn get(&self, column: usize, row: usize) -> bool {
        column < self.width && row < self.height && self.cells[row * self.width + column]
    }

    /// The number of covered cells.
    pub fn covered(&self) -> usize {
        self.cells.iter().filter(|c| **c).count()
    }

    /// Sets the cells whose centre is inside a polygon (even-odd rule) to `value`, with the
    /// polygon moved by `shift`.
    pub fn fill(&mut self, polygon: &[P], shift: P, value: bool) {
        if polygon.len() < 3 || self.width == 0 || self.height == 0 {
            return;
        }
        let points: Vec<P> = polygon
            .iter()
            .map(|p| P::new(p.x + shift.x, p.y + shift.y))
            .collect();
        let (mut low, mut high) = (f64::MAX, f64::MIN);
        for p in &points {
            low = low.min(p.y);
            high = high.max(p.y);
        }
        let first = ((low - self.origin.y) / self.cell - 0.5).ceil().max(0.0) as usize;
        let last = ((high - self.origin.y) / self.cell - 0.5).floor();
        if last < 0.0 {
            return;
        }
        let last = (last as usize).min(self.height - 1);
        let mut crossings: Vec<f64> = Vec::new();
        for row in first..=last {
            let y = self.origin.y + (row as f64 + 0.5) * self.cell;
            crossings.clear();
            for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
                if (a.y <= y) != (b.y <= y) {
                    crossings.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.as_chunks::<2>().0 {
                let from = ((pair[0] - self.origin.x) / self.cell - 0.5).ceil();
                let to = ((pair[1] - self.origin.x) / self.cell - 0.5).floor();
                if to < 0.0 || from > (self.width - 1) as f64 {
                    continue;
                }
                let from = from.max(0.0) as usize;
                let to = (to as usize).min(self.width - 1);
                if from > to {
                    continue;
                }
                let base = row * self.width;
                self.cells[base + from..=base + to].fill(value);
            }
        }
    }

    /// Draws the polygons of a layer in the order of the file: a dark one covers cells, a clear
    /// one uncovers them. The layer is moved by `shift`.
    pub fn paint(&mut self, layer: &GLayer, shift: P) {
        for poly in &layer.polys {
            self.fill(&poly.points, shift, poly.dark);
        }
    }
}

/// What a run of cells differs by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// Covered in the files and not in the board.
    OnlyFiles,
    /// Covered in the board and not in the files.
    OnlyBoard,
}

/// A row of neighbouring cells that differ in the same way, as a box in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffRun {
    /// The least corner.
    pub min: P,
    /// The greatest corner.
    pub max: P,
    /// How the cells differ.
    pub kind: DiffKind,
}

/// The comparison of a layer of the files with the layer of the board that has the same role.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerDiff {
    /// The role of both layers.
    pub role: Role,
    /// The name of the file.
    pub name: String,
    /// The side of a cell, in millimetres.
    pub cell_mm: f64,
    /// The cells covered in both.
    pub same_cells: usize,
    /// The cells covered only in the files.
    pub only_files_cells: usize,
    /// The cells covered only in the board.
    pub only_board_cells: usize,
    /// The cells that differ, as runs.
    pub runs: Vec<DiffRun>,
}

impl LayerDiff {
    /// True when no cell differs.
    pub fn identical(&self) -> bool {
        self.only_files_cells == 0 && self.only_board_cells == 0
    }

    /// The area that only the files cover, in square millimetres.
    pub fn only_files_mm2(&self) -> f64 {
        self.only_files_cells as f64 * self.cell_mm * self.cell_mm
    }

    /// The area that only the board covers, in square millimetres.
    pub fn only_board_mm2(&self) -> f64 {
        self.only_board_cells as f64 * self.cell_mm * self.cell_mm
    }
}

/// The comparison of a set of files with a board.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// The shift that was added to the board to bring it over the files, in millimetres.
    pub shift: P,
    /// One entry for each layer of the files that has a layer of the board with its role.
    pub layers: Vec<LayerDiff>,
    /// The names of the files with a role that no layer of the board has.
    pub unmatched: Vec<String>,
}

impl Comparison {
    /// The comparison of the layer of a file, when it was made.
    pub fn of(&self, name: &str) -> Option<&LayerDiff> {
        self.layers.iter().find(|d| d.name == name)
    }

    /// True when every compared layer is identical.
    pub fn identical(&self) -> bool {
        self.layers.iter().all(LayerDiff::identical)
    }
}

/// The shift that puts the outline of the board over the outline of the files: the difference of
/// their least corners. It is zero when either has no outline.
pub fn align_shift(files: &GerberScene, board: &GerberScene) -> P {
    let outline = |scene: &GerberScene| {
        scene
            .layer_by_role(Role::Outline)
            .map(|l| l.bounds)
            .filter(|b| !b.is_empty())
    };
    match (outline(files), outline(board)) {
        (Some(f), Some(b)) => P::new(f.min.x - b.min.x, f.min.y - b.min.y),
        _ => P::new(0.0, 0.0),
    }
}

/// Compares two layers cell by cell; the board layer is moved by `shift`.
pub fn compare_layer(
    role: Role,
    files: &GLayer,
    board: &GLayer,
    shift: P,
    cell: f64,
    max_cells: usize,
) -> LayerDiff {
    let mut bounds = files.bounds;
    if !board.bounds.is_empty() {
        bounds = bounds.union(Bounds {
            min: P::new(board.bounds.min.x + shift.x, board.bounds.min.y + shift.y),
            max: P::new(board.bounds.max.x + shift.x, board.bounds.max.y + shift.y),
        });
    }
    let mut a = Raster::new(bounds.expanded(cell), cell, max_cells);
    let mut b = a.like();
    a.paint(files, P::new(0.0, 0.0));
    b.paint(board, shift);
    let mut diff = LayerDiff {
        role,
        name: files.name.clone(),
        cell_mm: a.cell,
        same_cells: 0,
        only_files_cells: 0,
        only_board_cells: 0,
        runs: Vec::new(),
    };
    for row in 0..a.height {
        let mut open: Option<(DiffKind, usize)> = None;
        for column in 0..=a.width {
            let state = if column < a.width {
                match (a.get(column, row), b.get(column, row)) {
                    (true, true) => {
                        diff.same_cells += 1;
                        None
                    }
                    (true, false) => {
                        diff.only_files_cells += 1;
                        Some(DiffKind::OnlyFiles)
                    }
                    (false, true) => {
                        diff.only_board_cells += 1;
                        Some(DiffKind::OnlyBoard)
                    }
                    (false, false) => None,
                }
            } else {
                None
            };
            match (open, state) {
                (Some((kind, _)), Some(now)) if kind == now => {}
                (Some((kind, from)), _) => {
                    diff.runs.push(run(&a, from, column, row, kind));
                    open = state.map(|k| (k, column));
                }
                (None, Some(now)) => open = Some((now, column)),
                (None, None) => {}
            }
        }
    }
    diff
}

fn run(raster: &Raster, from: usize, to: usize, row: usize, kind: DiffKind) -> DiffRun {
    DiffRun {
        min: P::new(
            raster.origin.x + from as f64 * raster.cell,
            raster.origin.y + row as f64 * raster.cell,
        ),
        max: P::new(
            raster.origin.x + to as f64 * raster.cell,
            raster.origin.y + (row + 1) as f64 * raster.cell,
        ),
        kind,
    }
}

/// Compares every layer of the files that has a role with the layer of the board with the same
/// role. With `align` the board is moved so that the two outlines meet.
pub fn compare_scenes(
    files: &GerberScene,
    board: &GerberScene,
    settings: &ViewerSettings,
    align: bool,
) -> Comparison {
    let shift = if align {
        align_shift(files, board)
    } else {
        P::new(0.0, 0.0)
    };
    let mut layers = Vec::new();
    let mut unmatched = Vec::new();
    for layer in files.layers.iter().filter(|l| l.error.is_none()) {
        let Some(role) = layer.role else {
            continue;
        };
        match board.layer_by_role(role).filter(|b| b.error.is_none()) {
            Some(other) => layers.push(compare_layer(
                role,
                layer,
                other,
                shift,
                settings.gerber_compare_cell_mm,
                settings.gerber_compare_max_cells,
            )),
            None => unmatched.push(layer.name.clone()),
        }
    }
    Comparison {
        shift,
        layers,
        unmatched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "%FSLAX46Y46*%\n%MOMM*%\n";

    fn settings() -> ViewerSettings {
        ViewerSettings::default()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn set_of(files: &[(&str, String)]) -> ImportedSet {
        let inputs: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|(name, text)| ((*name).to_owned(), text.clone().into_bytes()))
            .collect();
        gbr::import_files(&inputs, &ImportSettings::default()).unwrap()
    }

    fn scene_of(files: &[(&str, String)]) -> GerberScene {
        GerberScene::from_set(&set_of(files), &settings())
    }

    fn square(side: f64) -> String {
        // a dark region of side x side millimetres from the origin
        format!(
            "{HEAD}G36*\nX0Y0D02*\nG01*\nX{side:.3}Y0D01*\nX{side:.3}Y{side:.3}D01*\nX0Y{side:.3}D01*\nX0Y0D01*\nG37*\nM02*\n"
        )
    }

    #[test]
    fn the_shapes_are_counter_clockwise_polygons_of_the_right_size() {
        let c = circle_polygon(P::new(1.0, 2.0), 0.5, 16);
        assert_eq!(c.len(), 16);
        assert!(c.iter().all(|p| near(p.dist(P::new(1.0, 2.0)), 0.5)));
        let r = rect_polygon(P::new(0.0, 0.0), 2.0, 1.0);
        assert_eq!(r[0], P::new(-1.0, -0.5));
        assert_eq!(r[2], P::new(1.0, 0.5));
        let k = capsule(P::new(0.0, 0.0), P::new(10.0, 0.0), 0.5, 16);
        let mut b = Bounds::EMPTY;
        for p in &k {
            b.include(*p);
        }
        assert!(near(b.min.x, -0.5) && near(b.max.x, 10.5));
        assert!(near(b.min.y, -0.5) && near(b.max.y, 0.5));
        let hex = regular_polygon(P::new(0.0, 0.0), 2.0, 6, 0.0);
        assert_eq!(hex.len(), 6);
        assert!(near(hex[0].x, 1.0) && near(hex[0].y, 0.0));
        // the zero-length capsule is a circle of the same radius
        let dot = capsule(P::new(3.0, 3.0), P::new(3.0, 3.0), 1.0, 16);
        assert!(dot.iter().all(|p| near(p.dist(P::new(3.0, 3.0)), 1.0)));
    }

    #[test]
    fn convex_polygons_get_a_fan_and_a_notched_one_is_triangulated() {
        let fan = GPoly::new(
            circle_polygon(P::new(0.0, 0.0), 1.0, 8),
            true,
            [None; 2],
            false,
            100,
        );
        assert_eq!(fan.tris.len(), 6);
        let notch = vec![
            P::new(0.0, 0.0),
            P::new(4.0, 0.0),
            P::new(4.0, 4.0),
            P::new(2.0, 1.0),
            P::new(0.0, 4.0),
        ];
        assert!(!is_convex(&notch));
        let poly = GPoly::new(notch, true, [None; 2], true, 100);
        assert_eq!(poly.tris.len(), 3);
        // too many corners to fill: outline only
        let big = GPoly::new(
            circle_polygon(P::new(0.0, 0.0), 1.0, 8),
            true,
            [None; 2],
            false,
            4,
        );
        assert!(big.tris.is_empty());
    }

    #[test]
    fn an_arc_stroke_follows_the_arc_with_the_width_of_the_aperture() {
        // a quarter of a circle of radius 2 round the origin, width 0.5, drawn counter-clockwise
        let strip = arc_stroke(P::new(0.0, 0.0), 2.0, 0.0, PI / 2.0, 0.5, 0.1, 16);
        let mut b = Bounds::EMPTY;
        for p in &strip {
            b.include(*p);
            let d = p.dist(P::new(0.0, 0.0));
            assert!(d <= 2.25 + 1e-9, "{d}");
        }
        // the cap at the start reaches down to y = -0.25 and the one at the end to x = -0.25
        assert!(near(b.max.x, 2.25) && near(b.max.y, 2.25));
        assert!(near(b.min.y, -0.25) || b.min.y > -0.26);
        assert!(b.min.x < 0.0);
        // clockwise gives the strip the other way round
        let back = arc_stroke(P::new(0.0, 0.0), 2.0, PI / 2.0, -PI / 2.0, 0.5, 0.1, 16);
        assert!(back.iter().all(|p| p.dist(P::new(0.0, 0.0)) <= 2.25 + 1e-9));
        assert!(back.iter().any(|p| p.dist(P::new(0.0, 0.0)) < 1.8));
    }

    #[test]
    fn a_gerber_file_becomes_polygons_with_their_polarity_in_the_order_of_the_file() {
        let text = format!(
            "%TF.FileFunction,Copper,L1,Top*%\n{HEAD}%ADD10C,1*%\n%ADD11R,2X1*%\nD10*\nX0Y0D02*\nG01*\nX10.0Y0D01*\nD11*\nX5.0Y5.0D03*\n%LPC*%\nD10*\nX5.0Y5.0D03*\n%LPD*%\nG36*\nX0Y8.0D02*\nG01*\nX2.0Y8.0D01*\nX2.0Y10.0D01*\nX0Y8.0D01*\nG37*\nM02*\n"
        );
        let scene = scene_of(&[("top.gbr", text)]);
        let layer = &scene.layers[0];
        assert_eq!(layer.role, Some(Role::CopperTop));
        assert_eq!(layer.kind, LayerKind::Copper);
        assert_eq!(layer.polys.len(), 4);
        let polarity: Vec<bool> = layer.polys.iter().map(|p| p.dark).collect();
        assert_eq!(polarity, vec![true, true, false, true]);
        assert_eq!((layer.dark, layer.clear), (3, 1));
        assert!(layer.polys[3].region);
        assert_eq!(layer.polys[3].points.len(), 3);
        // the line: from (0,0) to (10,0), width 1
        assert!(near(layer.polys[0].bounds.min.x, -0.5));
        assert!(near(layer.polys[0].bounds.max.x, 10.5));
        // the rectangle flash is 2 by 1 at (5,5)
        assert!(near(layer.polys[1].bounds.width(), 2.0));
        assert!(near(layer.polys[1].bounds.height(), 1.0));
        assert_eq!(layer.polys[1].anchors[0], Some(P::new(5.0, 5.0)));
        assert_eq!(layer.colour, settings().colour("top_copper"));
        assert!(!layer.incomplete && layer.error.is_none());
        assert!(near(scene.bounds().max.y, 10.0));
    }

    #[test]
    fn an_arc_item_is_drawn_between_its_ends_round_its_centre() {
        let text = format!(
            "{HEAD}%ADD10C,0.2*%\nD10*\nX2000000Y1000000D02*\nG75*\nG03*\nX1000000Y2000000I-1000000J0D01*\nM02*\n"
        );
        let scene = scene_of(&[("arc.gbr", text)]);
        let layer = &scene.layers[0];
        assert_eq!(layer.polys.len(), 1);
        let poly = &layer.polys[0];
        assert_eq!(
            poly.anchors,
            [Some(P::new(2.0, 1.0)), Some(P::new(1.0, 2.0))]
        );
        // every corner is within half the width of the circle of radius 1 round (1,1)
        assert!(poly
            .points
            .iter()
            .all(|p| (p.dist(P::new(1.0, 1.0)) - 1.0).abs() <= 0.1 + 1e-9));
        // counter-clockwise from (2,1) to (1,2): it passes through the upper right quarter
        assert!(poly.points.iter().any(|p| p.x > 1.5 && p.y > 1.5));
        assert!(!poly.points.iter().any(|p| p.x < 0.8 && p.y < 0.8));
    }

    #[test]
    fn a_macro_keeps_the_exposure_of_each_primitive_and_an_aperture_hole_is_clear() {
        let text = format!(
            "{HEAD}%AMBOX*\n0 a box*\n21,1,$1,$2,0,0,0*\n1,0,$3,0,0*%\n%ADD10BOX,2X1X0.4*%\n%ADD11C,1.0X0.4*%\nD10*\nX0Y0D03*\nD11*\nX5.0Y0D03*\nM02*\n"
        );
        let scene = scene_of(&[("m.gbr", text)]);
        let polys = &scene.layers[0].polys;
        assert_eq!(polys.len(), 4);
        assert_eq!(
            polys.iter().map(|p| p.dark).collect::<Vec<_>>(),
            vec![true, false, true, false]
        );
        assert!(near(polys[0].bounds.width(), 2.0));
        assert!(near(polys[1].bounds.width(), 0.4));
        assert!(near(polys[3].bounds.width(), 0.4));
        assert_eq!(polys[3].anchors, [None, None]);
    }

    #[test]
    fn drill_files_give_holes_and_slots_and_a_file_that_fails_keeps_its_error() {
        let drill = "M48\nMETRIC\nT1C0.8\nT2C0.6\n%\nT1\nX1.0Y1.0\nX2.0Y2.0\nT2\nX3.0Y1.0G85X5.0Y1.0\nM30\n".to_owned();
        let scene = scene_of(&[
            ("a-PTH.drl", drill),
            ("junk.gbr", "hello".to_owned()),
            ("a-Edge_Cuts.gbr", square(20.0)),
        ]);
        assert_eq!(scene.layers.len(), 3);
        // the stack: the holes, then the outline; the file that failed is first (no role)
        let by_name = |n: &str| scene.layers.iter().find(|l| l.name == n).unwrap();
        let holes = by_name("a-PTH.drl");
        assert_eq!(holes.kind, LayerKind::Drill);
        assert_eq!((holes.holes, holes.slots), (2, 1));
        assert_eq!(holes.polys.len(), 3);
        assert_eq!(holes.colour, settings().colour("gerber_drill_plated"));
        assert_eq!(holes.polys[0].anchors[0], Some(P::new(1.0, 1.0)));
        let junk = by_name("junk.gbr");
        assert!(junk.error.is_some() && !junk.visible && !junk.is_drawable());
        let order: Vec<&str> = scene.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(order, vec!["junk.gbr", "a-PTH.drl", "a-Edge_Cuts.gbr"]);
        assert_eq!(by_name("a-Edge_Cuts.gbr").role, Some(Role::Outline));
    }

    #[test]
    fn the_stack_order_puts_the_overlays_over_the_copper() {
        let files = [
            ("x-F_Silkscreen.gbr", square(1.0)),
            ("x-F_Mask.gbr", square(1.0)),
            ("x-F_Cu.gbr", square(1.0)),
            ("x-In1_Cu.gbr", square(1.0)),
            ("x-B_Cu.gbr", square(1.0)),
        ];
        let scene = scene_of(&files);
        let order: Vec<Option<Role>> = scene.layers.iter().map(|l| l.role).collect();
        assert_eq!(
            order,
            vec![
                Some(Role::CopperBottom),
                Some(Role::CopperInner(1)),
                Some(Role::CopperTop),
                Some(Role::MaskTop),
                Some(Role::LegendTop)
            ]
        );
        assert_eq!(scene.layers[1].colour, settings().colour("inner_copper_1"));
    }

    #[test]
    fn visibility_order_and_opacity_are_kept_by_the_scene() {
        let mut scene = scene_of(&[("a-F_Cu.gbr", square(2.0)), ("a-B_Cu.gbr", square(2.0))]);
        assert_eq!(scene.layers[0].role, Some(Role::CopperBottom));
        assert!(scene.layers.iter().all(|l| l.visible));
        scene.set_visible(0, false);
        assert!(!scene.layers[0].visible);
        scene.set_all_visible(true);
        assert!(scene.layers[0].visible);
        scene.set_opacity(1, 0.5);
        assert_eq!(scene.layers[1].opacity, 0.5_f64);
        assert_eq!(scene.layers[1].draw_colour()[3], 128);
        scene.set_opacity(1, 7.0);
        assert_eq!(scene.layers[1].opacity, 1.0_f64);
        scene.set_opacity(1, f64::NAN);
        assert_eq!(scene.layers[1].opacity, 1.0_f64);
        scene.set_opacity(1, 0.0);
        assert_eq!(scene.layers[1].opacity, 0.01_f64);
        assert!(scene.move_up(0));
        assert_eq!(scene.layers[1].role, Some(Role::CopperBottom));
        assert!(!scene.move_up(1));
        assert!(scene.move_down(1));
        assert_eq!(scene.layers[0].role, Some(Role::CopperBottom));
        assert!(!scene.move_down(0));
        assert!(!scene.move_up(9) && !scene.move_down(9));
    }

    #[test]
    fn a_click_snaps_to_the_nearest_centre_end_or_corner_of_a_visible_layer() {
        let text = format!(
            "%TF.FileFunction,Copper,L1,Top*%\n{HEAD}%ADD10C,1*%\nD10*\nX0Y0D02*\nG01*\nX10.0Y0D01*\nX20.0Y20.0D03*\nG36*\nX30.0Y30.0D02*\nG01*\nX34.0Y30.0D01*\nX34.0Y34.0D01*\nX30.0Y30.0D01*\nG37*\nM02*\n"
        );
        let mut scene = scene_of(&[("top.gbr", text)]);
        assert_eq!(scene.snap(P::new(10.2, 0.1), 0.5), Some(P::new(10.0, 0.0)));
        assert_eq!(
            scene.snap(P::new(20.0, 20.3), 0.5),
            Some(P::new(20.0, 20.0))
        );
        assert_eq!(
            scene.snap(P::new(33.9, 30.2), 0.5),
            Some(P::new(34.0, 30.0))
        );
        assert_eq!(scene.snap(P::new(15.0, 15.0), 0.5), None);
        scene.set_visible(0, false);
        assert_eq!(scene.snap(P::new(10.2, 0.1), 0.5), None);
    }

    #[test]
    fn the_polygon_limit_cuts_the_set_and_says_so() {
        let mut limited = settings();
        limited.gerber_max_polys = 1;
        let text =
            format!("{HEAD}%ADD10C,1*%\nD10*\nX1.0Y1.0D03*\nX2.0Y2.0D03*\nX3.0Y3.0D03*\nM02*\n");
        let set = set_of(&[("a.gbr", text)]);
        let scene = GerberScene::from_set(&set, &limited);
        assert!(scene.truncated && scene.layers[0].truncated);
        assert_eq!(scene.layers[0].polys.len(), 1);
        assert!(!GerberScene::from_set(&set, &settings()).truncated);
    }

    #[test]
    fn the_raster_covers_the_cells_inside_and_a_clear_polygon_uncovers_them() {
        let bounds = Bounds {
            min: P::new(0.0, 0.0),
            max: P::new(10.0, 10.0),
        };
        let mut raster = Raster::new(bounds, 1.0, 1000);
        assert_eq!((raster.width, raster.height), (11, 11));
        raster.fill(
            &rect_polygon(P::new(5.0, 5.0), 4.0, 4.0),
            P::new(0.0, 0.0),
            true,
        );
        // cells whose centre lies in [3, 7)
        assert_eq!(raster.covered(), 16);
        assert!(raster.get(3, 3) && raster.get(6, 6));
        assert!(!raster.get(2, 3) && !raster.get(7, 3));
        raster.fill(
            &rect_polygon(P::new(5.0, 5.0), 2.0, 2.0),
            P::new(0.0, 0.0),
            false,
        );
        assert_eq!(raster.covered(), 12);
        // a polygon moved off the grid does nothing and the grid never panics
        raster.fill(
            &rect_polygon(P::new(5.0, 5.0), 2.0, 2.0),
            P::new(100.0, -100.0),
            true,
        );
        assert_eq!(raster.covered(), 12);
        // the cell grows to keep the grid within its limit
        let coarse = Raster::new(bounds, 0.01, 400);
        assert!(coarse.width * coarse.height <= 400);
        assert!(coarse.cell > 0.01);
        let none = Raster::new(Bounds::EMPTY, 1.0, 10);
        assert_eq!(none.covered(), 0);
        assert!(!none.get(0, 0));
    }

    #[test]
    fn identical_layers_agree_and_a_missing_pad_is_a_difference_with_a_run() {
        let a = "%TF.FileFunction,Copper,L1,Top*%\n%FSLAX46Y46*%\n%MOMM*%\n%ADD10R,2X2*%\nD10*\nX5.0Y5.0D03*\nX15.0Y5.0D03*\nM02*\n";
        let b = "%TF.FileFunction,Copper,L1,Top*%\n%FSLAX46Y46*%\n%MOMM*%\n%ADD10R,2X2*%\nD10*\nX5.0Y5.0D03*\nM02*\n";
        let files = scene_of(&[("f.gbr", a.to_owned())]);
        let same = scene_of(&[("b.gbr", a.to_owned())]);
        let fewer = scene_of(&[("b.gbr", b.to_owned())]);
        let mut s = settings();
        s.gerber_compare_cell_mm = 0.25;
        let equal = compare_scenes(&files, &same, &s, true);
        assert!(equal.identical());
        assert_eq!(equal.layers.len(), 1);
        assert!(equal.layers[0].same_cells > 0 && equal.layers[0].runs.is_empty());
        assert!(equal.unmatched.is_empty());
        let different = compare_scenes(&files, &fewer, &s, true);
        let diff = different.of("f.gbr").unwrap();
        assert!(!diff.identical() && !different.identical());
        assert_eq!(diff.only_board_cells, 0);
        assert!(diff.only_files_cells > 0);
        // the pad that is missing from the board is 2 by 2 millimetres
        assert!(
            (diff.only_files_mm2() - 4.0).abs() < 0.8,
            "{}",
            diff.only_files_mm2()
        );
        assert!(diff.runs.iter().all(|r| r.kind == DiffKind::OnlyFiles));
        assert!(diff.runs.iter().all(|r| r.min.x >= 13.9 && r.max.x <= 16.1));
        assert!(diff.only_board_mm2() == 0.0);
    }

    #[test]
    fn a_layer_without_a_partner_is_listed_and_the_outlines_align_the_board() {
        let files = scene_of(&[
            ("a-F_Cu.gbr", square(4.0)),
            ("a-B_Cu.gbr", square(4.0)),
            ("a-Edge_Cuts.gbr", square(20.0)),
        ]);
        // the board is the same shape moved by (+100, +50)
        let moved = |side: f64| {
            format!(
                "{HEAD}G36*\nX100.0Y50.0D02*\nG01*\nX{x:.3}Y50.0D01*\nX{x:.3}Y{y:.3}D01*\nX100.0Y{y:.3}D01*\nX100.0Y50.0D01*\nG37*\nM02*\n",
                x = 100.0 + side,
                y = 50.0 + side
            )
        };
        let board = scene_of(&[("b-F_Cu.gbr", moved(4.0)), ("b-Edge_Cuts.gbr", moved(20.0))]);
        let s = settings();
        let aligned = compare_scenes(&files, &board, &s, true);
        assert!(near(aligned.shift.x, -100.0) && near(aligned.shift.y, -50.0));
        assert!(aligned.identical(), "{aligned:?}");
        assert_eq!(aligned.unmatched, vec!["a-B_Cu.gbr".to_owned()]);
        assert_eq!(aligned.layers.len(), 2);
        let unaligned = compare_scenes(&files, &board, &s, false);
        assert_eq!(unaligned.shift, P::new(0.0, 0.0));
        assert!(!unaligned.identical());
        assert_eq!(
            align_shift(
                &board,
                &GerberScene {
                    layers: Vec::new(),
                    truncated: false
                }
            ),
            P::new(0.0, 0.0)
        );
    }

    #[test]
    fn a_folder_is_read_without_its_documents_and_a_folder_without_files_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b-B_Cu.gbr"), square(2.0)).unwrap();
        std::fs::write(dir.path().join("a-F_Cu.gbr"), square(2.0)).unwrap();
        std::fs::write(dir.path().join("README.txt"), "notes").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let read = read_folder(dir.path(), &ImportSettings::default()).unwrap();
        let names: Vec<&str> = read.files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["a-F_Cu.gbr", "b-B_Cu.gbr"]);
        assert_eq!(read.skipped, 1);
        let (scene, skipped) = open_folder(dir.path(), &settings()).unwrap();
        assert_eq!((scene.layers.len(), skipped), (2, 1));
        let empty = tempfile::tempdir().unwrap();
        let error = open_folder(empty.path(), &settings()).unwrap_err();
        assert_eq!(error.code, NOTHING_TO_DRAW);
        let missing = dir.path().join("nowhere");
        assert_eq!(
            open_folder(&missing, &settings()).unwrap_err().code,
            FILE_UNUSABLE
        );
        let few = ImportSettings {
            max_files: 1,
            ..ImportSettings::default()
        };
        let limited = read_folder(dir.path(), &few).unwrap();
        assert_eq!((limited.files.len(), limited.skipped), (1, 2));
    }

    #[test]
    fn the_role_colours_and_kinds_follow_the_settings() {
        let s = settings();
        assert_eq!(role_colour(Some(Role::MaskTop), &s), s.colour("mask"));
        assert_eq!(role_colour(Some(Role::Outline), &s), s.colour("outline"));
        assert_eq!(role_colour(None, &s), s.colour("documentation"));
        assert_eq!(
            role_colour(Some(Role::DrillNonPlated), &s),
            s.colour("gerber_drill_unplated")
        );
        assert_eq!(
            role_colour(Some(Role::CopperInner(5)), &s),
            s.colour("inner_copper_1")
        );
        assert_eq!(kind_of(Some(Role::PasteBottom)), LayerKind::Paste);
        assert_eq!(kind_of(None), LayerKind::Unknown);
        assert!(stack_rank(Some(Role::CopperBottom)) < stack_rank(Some(Role::CopperTop)));
        assert!(stack_rank(Some(Role::CopperTop)) < stack_rank(Some(Role::LegendTop)));
    }
}
