// SPDX-License-Identifier: GPL-3.0-or-later
//! The check of the board and its components against an enclosure. The enclosure is a cavity: a
//! box of an inner length, width and height, or an inner contour (a polygon of the board plane) and
//! a height, with a floor below the board (the height of the posts, `standoff`) and a lid. The
//! board, each component and each screw post are tested against it: a component box that leaves
//! the walls, goes through the lid or the floor, or is nearer to them than the least clearance;
//! a component that collides with a post (a cylinder from the floor to the underside of the
//! board); a post with no hole in the board or outside its outline. The least clearances are
//! reported whatever they are. All of it is worked out on the boxes of the components, as the
//! collisions of the assembly are; nothing is judged from the real shape of a part.
//!
//! The settings are those of a plain `key = value` text (a file of the enclosure), lengths in
//! millimetres with up to six decimals. Besides the checks, [`usable_holes`] picks the holes
//! that can be written through the STEP solid of the board (`crate::prism::board_step`).

use std::collections::BTreeMap;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef};
use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{AssemblyError, Finding, Severity, LIMIT_EXCEEDED};
use crate::enclosure_codes::{
    CLEARANCES, CONTOUR_UNUSABLE, FLOOR, HOLE_SKIPPED, LID, POST_COLLISION, POST_NO_HOLE,
    POST_OUTSIDE, SPEC_INVALID, WALL,
};
use crate::geom::{counter_clockwise, point_in_polygon, twice_area, P};
use crate::prism::Hole;
use crate::scene::{millimetres, Scene3d};
use crate::settings::AssemblySettings;
use crate::step::decimal_to_nm;

/// The most posts a file of an enclosure may name.
const MAX_POSTS: usize = 64;
/// The largest length of the file of an enclosure, in nanometres.
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The inside of the enclosure in the board plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inner {
    /// A box of this length (along x) and width (along y).
    Box {
        /// The length along x.
        length_nm: i64,
        /// The width along y.
        width_nm: i64,
    },
    /// The contour of a DXF drawing, read by the caller.
    Contour {
        /// The file, relative to the folder of the file of the enclosure.
        path: String,
        /// The layer of the drawing to take it from; every layer when left out.
        layer: Option<String>,
    },
}

/// A screw post: a cylinder from the floor up to its top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    /// The centre along x.
    pub x_nm: i64,
    /// The centre along y.
    pub y_nm: i64,
    /// The diameter of the post.
    pub diameter_nm: i64,
    /// The diameter of the hole that the screw needs in the board, when it is given.
    pub hole_nm: Option<i64>,
    /// The height of the top of the post above the underside of the board; the post reaches the
    /// underside of the board when it is left out.
    pub top_nm: Option<i64>,
}

/// The file of an enclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclosureSpec {
    /// The inside in the board plane.
    pub inner: Inner,
    /// The inner height, from the floor to the lid.
    pub height_nm: i64,
    /// The thickness of the walls (for the size of the outside; the checks use the inside).
    pub wall_nm: i64,
    /// The height of the underside of the board above the floor.
    pub standoff_nm: i64,
    /// The place of the lower left corner of a box, or the shift of a contour; a box is centred
    /// on the board and a contour is not shifted when they are left out.
    pub x_nm: Option<i64>,
    /// See `x_nm`.
    pub y_nm: Option<i64>,
    /// The least distance that the board and the components keep from the walls, the lid and the
    /// floor.
    pub min_clearance_nm: i64,
    /// How far the centre of a hole of the board may be from the centre of a post.
    pub post_hole_tolerance_nm: i64,
    /// The screw posts.
    pub posts: Vec<Post>,
}

fn invalid(line: usize, what: &str) -> AssemblyError {
    AssemblyError::new(SPEC_INVALID, format!("line {line}: {what}"))
}

/// A length in millimetres of the file as nanometres, within the range.
fn length(value: &str) -> Option<i64> {
    decimal_to_nm(value, 1_000_000).filter(|v| v.abs() < LENGTH_LIMIT_NM)
}

impl EnclosureSpec {
    /// Reads the text of a file of an enclosure.
    pub fn parse(text: &str) -> Result<EnclosureSpec, AssemblyError> {
        let mut inner: Option<Inner> = None;
        let mut height: Option<i64> = None;
        let mut contour_layer: Option<String> = None;
        let mut spec = EnclosureSpec {
            inner: Inner::Box {
                length_nm: 0,
                width_nm: 0,
            },
            height_nm: 0,
            wall_nm: 2_000_000,
            standoff_nm: 3_000_000,
            x_nm: None,
            y_nm: None,
            min_clearance_nm: 500_000,
            post_hole_tolerance_nm: 300_000,
            posts: Vec::new(),
        };
        let mut box_height = false;
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| invalid(line, "not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || invalid(line, &format!("the value of {key} is not valid"));
            let positive = |v: &str| length(v).filter(|n| *n > 0).ok_or_else(bad);
            let not_negative = |v: &str| length(v).filter(|n| *n >= 0).ok_or_else(bad);
            match key {
                "box" => {
                    if inner.is_some() {
                        return Err(invalid(line, "give box or contour, once"));
                    }
                    let sizes = value.strip_prefix("box:").unwrap_or(value);
                    let parts: Vec<&str> = sizes.split(['x', 'X']).collect();
                    let [l, w, h] = parts.as_slice() else {
                        return Err(bad());
                    };
                    inner = Some(Inner::Box {
                        length_nm: positive(l)?,
                        width_nm: positive(w)?,
                    });
                    height = Some(positive(h)?);
                    box_height = true;
                }
                "contour" => {
                    if inner.is_some() {
                        return Err(invalid(line, "give box or contour, once"));
                    }
                    let unsafe_path = value.is_empty()
                        || value.starts_with('/')
                        || value.starts_with('\\')
                        || value.contains(':')
                        || value.split(['/', '\\']).any(|part| part == "..");
                    if unsafe_path {
                        return Err(invalid(
                            line,
                            "the contour is the relative name of a DXF file with no .. in it",
                        ));
                    }
                    inner = Some(Inner::Contour {
                        path: value.to_owned(),
                        layer: None,
                    });
                }
                "contour_layer" => {
                    if value.is_empty() {
                        return Err(bad());
                    }
                    contour_layer = Some(value.to_owned());
                }
                "height_mm" => {
                    if box_height {
                        return Err(invalid(line, "the height of a box is in box = LxWxH"));
                    }
                    height = Some(positive(value)?);
                }
                "wall_mm" => spec.wall_nm = not_negative(value)?,
                "standoff_mm" => spec.standoff_nm = not_negative(value)?,
                "x_mm" => spec.x_nm = Some(length(value).ok_or_else(bad)?),
                "y_mm" => spec.y_nm = Some(length(value).ok_or_else(bad)?),
                "min_clearance_mm" => spec.min_clearance_nm = not_negative(value)?,
                "post_hole_tolerance_mm" => spec.post_hole_tolerance_nm = not_negative(value)?,
                "post" => {
                    if spec.posts.len() >= MAX_POSTS {
                        return Err(invalid(line, "more posts than the limit of 64"));
                    }
                    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
                    if !(3..=5).contains(&fields.len()) {
                        return Err(invalid(
                            line,
                            "a post is x, y, diameter, and optionally the hole and the top",
                        ));
                    }
                    spec.posts.push(Post {
                        x_nm: length(fields[0]).ok_or_else(bad)?,
                        y_nm: length(fields[1]).ok_or_else(bad)?,
                        diameter_nm: positive(fields[2])?,
                        hole_nm: match fields.get(3) {
                            Some(f) if !f.is_empty() => Some(positive(f)?),
                            _ => None,
                        },
                        top_nm: match fields.get(4) {
                            Some(f) if !f.is_empty() => Some(length(f).ok_or_else(bad)?),
                            _ => None,
                        },
                    });
                }
                _ => return Err(invalid(line, &format!("unknown key {key}"))),
            }
        }
        spec.inner = match inner {
            Some(Inner::Contour { path, .. }) => Inner::Contour {
                path,
                layer: contour_layer,
            },
            Some(other) => {
                if contour_layer.is_some() {
                    return Err(AssemblyError::new(
                        SPEC_INVALID,
                        "contour_layer belongs to a contour, not to a box",
                    ));
                }
                other
            }
            None => {
                return Err(AssemblyError::new(
                    SPEC_INVALID,
                    "give the inside: box = LxWxH or contour = FILE.dxf with height_mm",
                ))
            }
        };
        spec.height_nm = height.ok_or_else(|| {
            AssemblyError::new(SPEC_INVALID, "a contour needs height_mm, the inner height")
        })?;
        Ok(spec)
    }
}

/// The cavity of the enclosure in the coordinates of the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cavity {
    /// The inside in the board plane, counter-clockwise.
    pub outline: Vec<P>,
    /// The height of the floor (below the board, so negative).
    pub floor_nm: i64,
    /// The height of the lid.
    pub lid_nm: i64,
}

impl Cavity {
    /// The cavity of `spec` for a board with the outline `board_outline`. A contour is given by
    /// the caller (read from the drawing the spec names), as the corners of its polygon.
    pub fn new(
        spec: &EnclosureSpec,
        board_outline: &[P],
        contour: Option<&[P]>,
    ) -> Result<Cavity, AssemblyError> {
        let outline = match &spec.inner {
            Inner::Box {
                length_nm,
                width_nm,
            } => {
                if board_outline.len() < 3 {
                    return Err(AssemblyError::new(
                        SPEC_INVALID,
                        "the board has no outline to centre the box on",
                    ));
                }
                let xs = board_outline.iter().map(|p| p.0);
                let ys = board_outline.iter().map(|p| p.1);
                let centre_x = (xs.clone().min().unwrap_or(0) + xs.max().unwrap_or(0)) / 2;
                let centre_y = (ys.clone().min().unwrap_or(0) + ys.max().unwrap_or(0)) / 2;
                let x = spec.x_nm.unwrap_or(centre_x - length_nm / 2);
                let y = spec.y_nm.unwrap_or(centre_y - width_nm / 2);
                vec![
                    (x, y),
                    (x + length_nm, y),
                    (x + length_nm, y + width_nm),
                    (x, y + width_nm),
                ]
            }
            Inner::Contour { .. } => {
                let contour = contour.filter(|c| c.len() >= 3).ok_or_else(|| {
                    AssemblyError::new(CONTOUR_UNUSABLE, "the contour has fewer than 3 corners")
                })?;
                let (dx, dy) = (spec.x_nm.unwrap_or(0), spec.y_nm.unwrap_or(0));
                let shifted: Vec<P> = contour.iter().map(|p| (p.0 + dx, p.1 + dy)).collect();
                if twice_area(&shifted) == 0 {
                    return Err(AssemblyError::new(
                        CONTOUR_UNUSABLE,
                        "the contour has no area",
                    ));
                }
                counter_clockwise(&shifted)
            }
        };
        let floor_nm = -spec.standoff_nm;
        Ok(Cavity {
            outline,
            floor_nm,
            lid_nm: floor_nm + spec.height_nm,
        })
    }
}

/// The distance from a point to a segment, as a number.
fn point_segment(p: P, a: P, b: P) -> f64 {
    let (px, py) = (p.0 as f64, p.1 as f64);
    let (ax, ay) = (a.0 as f64, a.1 as f64);
    let (dx, dy) = (b.0 as f64 - ax, b.1 as f64 - ay);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / length_squared).clamp(0.0, 1.0)
    };
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

/// The distance from a point to the nearest edge of a polygon.
fn distance_to_edges(p: P, polygon: &[P]) -> f64 {
    let n = polygon.len();
    let mut least = f64::MAX;
    for (i, &a) in polygon.iter().enumerate() {
        least = least.min(point_segment(p, a, polygon[(i + 1) % n]));
    }
    least
}

fn orientation(a: P, b: P, c: P) -> i32 {
    let cross = i128::from(b.0 - a.0) * i128::from(c.1 - a.1)
        - i128::from(b.1 - a.1) * i128::from(c.0 - a.0);
    match cross {
        0 => 0,
        x if x > 0 => 1,
        _ => -1,
    }
}

/// True when an edge of `a` crosses an edge of `b` at a point inside both.
fn edges_cross(a: &[P], b: &[P]) -> bool {
    for (i, &p) in a.iter().enumerate() {
        let q = a[(i + 1) % a.len()];
        for (j, &r) in b.iter().enumerate() {
            let s = b[(j + 1) % b.len()];
            if orientation(p, q, r) * orientation(p, q, s) < 0
                && orientation(r, s, p) * orientation(r, s, q) < 0
            {
                return true;
            }
        }
    }
    false
}

/// Where a shape of the board plane is against the walls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wall {
    /// Inside, this far from the nearest wall (nanometres).
    Inside(i64),
    /// Outside by this much (nanometres), or across a wall when it is 0.
    Outside(i64),
}

fn against_walls(shape: &[P], cavity: &[P]) -> Wall {
    let mut depth = 0.0f64;
    let mut outside = false;
    for &v in shape {
        if !point_in_polygon(v, cavity) {
            outside = true;
            depth = depth.max(distance_to_edges(v, cavity));
        }
    }
    if outside {
        return Wall::Outside((depth.round() as i64).max(1));
    }
    if edges_cross(shape, cavity) {
        return Wall::Outside(0);
    }
    let mut least = f64::MAX;
    for &v in shape {
        least = least.min(distance_to_edges(v, cavity));
    }
    for &c in cavity {
        least = least.min(distance_to_edges(c, shape));
    }
    Wall::Inside(least.round() as i64)
}

/// The distance from a point to a convex quadrilateral, 0 when the point is inside it.
fn distance_to_quad(p: P, quad: &[P; 4]) -> f64 {
    if point_in_polygon(p, quad) {
        0.0
    } else {
        distance_to_edges(p, quad)
    }
}

/// The least clearances found, in nanometres; negative when something is through a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Clearances {
    /// To the walls.
    pub wall_nm: Option<i64>,
    /// To the lid.
    pub lid_nm: Option<i64>,
    /// To the floor.
    pub floor_nm: Option<i64>,
    /// To the posts.
    pub post_nm: Option<i64>,
}

fn lower(slot: &mut Option<i64>, value: i64) {
    *slot = Some(slot.map_or(value, |old| old.min(value)));
}

/// The result of the check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnclosureChecks {
    /// The findings, the errors first.
    pub findings: Vec<Finding>,
    /// The least clearances.
    pub least: Clearances,
}

/// The findings of a run, with the limit of each code.
struct Sink {
    findings: Vec<Finding>,
    kept: BTreeMap<&'static str, usize>,
    omitted: BTreeMap<&'static str, usize>,
    limit: usize,
}

impl Sink {
    fn push(&mut self, finding: Finding) {
        let count = self.kept.entry(finding.code).or_insert(0);
        if *count < self.limit {
            *count += 1;
            self.findings.push(finding);
        } else {
            *self.omitted.entry(finding.code).or_insert(0) += 1;
        }
    }
}

/// The holes of the board: its vias and the drills of its pads, in nanometres.
pub fn board_holes(board: &Board) -> Vec<Hole> {
    let mut holes = Vec::new();
    for via in board.vias.values() {
        holes.push(Hole {
            at: (via.at.x_nm.get(), via.at.y_nm.get()),
            diameter_nm: via.drill.get(),
        });
    }
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let Some(drill) = pad.drill else { continue };
            let at = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            );
            if let Some(at) = at {
                holes.push(Hole {
                    at: (at.x_nm.get(), at.y_nm.get()),
                    diameter_nm: drill.get(),
                });
            }
        }
    }
    holes
}

fn mm(nm: i64) -> String {
    millimetres(nm)
}

fn mm_f(nm: f64) -> String {
    millimetres(nm.round() as i64)
}

/// Checks the board, its components and the posts against the cavity.
pub fn check_enclosure(
    scene: &Scene3d,
    holes: &[Hole],
    cavity: &Cavity,
    spec: &EnclosureSpec,
    settings: &AssemblySettings,
) -> EnclosureChecks {
    let minimum = spec.min_clearance_nm;
    let mut sink = Sink {
        findings: Vec::new(),
        kept: BTreeMap::new(),
        omitted: BTreeMap::new(),
        limit: settings.max_findings,
    };
    let mut least = Clearances::default();

    let walls =
        |sink: &mut Sink, least: &mut Clearances, subject: &str, shape: &[P]| match against_walls(
            shape,
            &cavity.outline,
        ) {
            Wall::Inside(clear) => {
                lower(&mut least.wall_nm, clear);
                if clear < minimum {
                    sink.push(Finding::new(
                        WALL,
                        Severity::Warning,
                        Some(subject),
                        format!(
                            "{} mm from a wall, the least is {} mm",
                            mm(clear),
                            mm(minimum)
                        ),
                    ));
                }
            }
            Wall::Outside(depth) => {
                lower(&mut least.wall_nm, -depth);
                let detail = if depth == 0 {
                    "it crosses a wall".to_owned()
                } else {
                    format!("it is outside the walls by {} mm", mm(depth))
                };
                sink.push(Finding::new(WALL, Severity::Error, Some(subject), detail));
            }
        };
    let vertical = |sink: &mut Sink,
                    slot: &mut Option<i64>,
                    code: &'static str,
                    subject: &str,
                    clear: i64,
                    what: &str| {
        lower(slot, clear);
        if clear < 0 {
            sink.push(Finding::new(
                code,
                Severity::Error,
                Some(subject),
                format!("it is through the {what} by {} mm", mm(-clear)),
            ));
        } else if clear < minimum {
            sink.push(Finding::new(
                code,
                Severity::Warning,
                Some(subject),
                format!(
                    "{} mm from the {what}, the least is {} mm",
                    mm(clear),
                    mm(minimum)
                ),
            ));
        }
    };

    // the board
    walls(&mut sink, &mut least, "board", &scene.outline);
    vertical(
        &mut sink,
        &mut least.lid_nm,
        LID,
        "board",
        cavity.lid_nm - scene.thickness_nm,
        "lid",
    );
    vertical(
        &mut sink,
        &mut least.floor_nm,
        FLOOR,
        "board",
        -cavity.floor_nm,
        "floor",
    );
    // the components
    for part in &scene.parts {
        walls(&mut sink, &mut least, &part.reference, &part.corners);
        if part.bottom {
            vertical(
                &mut sink,
                &mut least.floor_nm,
                FLOOR,
                &part.reference,
                part.z_min - cavity.floor_nm,
                "floor",
            );
        } else {
            vertical(
                &mut sink,
                &mut least.lid_nm,
                LID,
                &part.reference,
                cavity.lid_nm - part.z_max,
                "lid",
            );
        }
    }

    // the posts
    for (n, post) in spec.posts.iter().enumerate() {
        let name = format!("post {}", n + 1);
        let centre = (post.x_nm, post.y_nm);
        let radius = post.diameter_nm / 2;
        let top = post.top_nm.unwrap_or(0);
        for part in &scene.parts {
            let height_overlap = part.z_max.min(top) - part.z_min.max(cavity.floor_nm);
            if height_overlap <= settings.min_overlap_nm {
                continue;
            }
            let distance = distance_to_quad(centre, &part.corners);
            let depth = radius as f64 - distance;
            lower(
                &mut least.post_nm,
                (distance - radius as f64).round() as i64,
            );
            if depth > settings.min_overlap_nm as f64 {
                sink.push(Finding::new(
                    POST_COLLISION,
                    Severity::Error,
                    Some(&format!("{} and {name}", part.reference)),
                    format!(
                        "the box of the component enters the post by {} mm",
                        mm_f(depth)
                    ),
                ));
            }
        }
        if !point_in_polygon(centre, &scene.outline) {
            sink.push(Finding::new(
                POST_OUTSIDE,
                Severity::Warning,
                Some(&name),
                "the centre of the post is not under the board",
            ));
        }
        let tolerance = spec.post_hole_tolerance_nm as f64;
        let near = holes
            .iter()
            .filter(|h| {
                (h.at.0 as f64 - centre.0 as f64).hypot(h.at.1 as f64 - centre.1 as f64)
                    <= tolerance
            })
            .max_by_key(|h| h.diameter_nm);
        match (near, post.hole_nm) {
            (None, _) => sink.push(Finding::new(
                POST_NO_HOLE,
                Severity::Warning,
                Some(&name),
                format!(
                    "no hole of the board has its centre within {} mm of the post",
                    mm(spec.post_hole_tolerance_nm)
                ),
            )),
            (Some(hole), Some(needed)) if hole.diameter_nm < needed => sink.push(Finding::new(
                POST_NO_HOLE,
                Severity::Warning,
                Some(&name),
                format!(
                    "the hole is {} mm, the screw needs {} mm",
                    mm(hole.diameter_nm),
                    mm(needed)
                ),
            )),
            _ => {}
        }
    }

    // the least clearances, always
    let mut parts: Vec<String> = Vec::new();
    for (label, value) in [
        ("walls", least.wall_nm),
        ("lid", least.lid_nm),
        ("floor", least.floor_nm),
        ("posts", least.post_nm),
    ] {
        if let Some(v) = value {
            parts.push(format!("{label} {} mm", mm(v)));
        }
    }
    let omitted: Vec<(&'static str, usize)> = sink.omitted.iter().map(|(c, n)| (*c, *n)).collect();
    let mut findings = std::mem::take(&mut sink.findings);
    for (code, count) in omitted {
        findings.push(Finding::new(
            LIMIT_EXCEEDED,
            Severity::Info,
            Some(code),
            format!("{count} more findings of this code were counted and not listed"),
        ));
    }
    findings.push(Finding::new(
        CLEARANCES,
        Severity::Info,
        None,
        parts.join(", "),
    ));
    findings.sort_by_key(|f| std::cmp::Reverse(f.severity));
    EnclosureChecks { findings, least }
}

/// The holes that can be written through the STEP solid of the board, and a finding for each that
/// cannot: one whose centre is outside the outline or whose circle touches the edge, and one that
/// overlaps a hole before it. They come back in the order of their centres.
pub fn usable_holes(outline: &[P], holes: &[Hole]) -> (Vec<Hole>, Vec<Finding>) {
    let mut sorted: Vec<Hole> = holes
        .iter()
        .copied()
        .filter(|h| h.diameter_nm >= 2)
        .collect();
    sorted.sort_by_key(|h| (h.at.0, h.at.1, h.diameter_nm));
    let mut findings = Vec::new();
    let mut kept: Vec<Hole> = Vec::new();
    let widest = sorted.iter().map(|h| h.diameter_nm / 2).max().unwrap_or(0);
    for hole in sorted {
        let radius = hole.diameter_nm / 2;
        let place = format!("hole at ({}, {}) mm", mm(hole.at.0), mm(hole.at.1));
        if !point_in_polygon(hole.at, outline) {
            findings.push(Finding::new(
                HOLE_SKIPPED,
                Severity::Warning,
                Some(&place),
                "its centre is outside the outline",
            ));
            continue;
        }
        if distance_to_edges(hole.at, outline) <= radius as f64 {
            findings.push(Finding::new(
                HOLE_SKIPPED,
                Severity::Warning,
                Some(&place),
                "it touches or crosses an edge of the outline",
            ));
            continue;
        }
        // the holes come in order of x: only those within the widest reach in x can touch
        let overlaps = kept
            .iter()
            .rev()
            .take_while(|other| hole.at.0 - other.at.0 <= radius + widest)
            .any(|other| {
                (hole.at.0 as f64 - other.at.0 as f64).hypot(hole.at.1 as f64 - other.at.1 as f64)
                    <= (radius + other.diameter_nm / 2) as f64
            });
        if overlaps {
            findings.push(Finding::new(
                HOLE_SKIPPED,
                Severity::Warning,
                Some(&place),
                "it overlaps a hole before it",
            ));
            continue;
        }
        kept.push(hole);
    }
    (kept, findings)
}

fn finding_value(f: &Finding) -> Value {
    json!({
        "code": f.code,
        "severity": f.severity.as_str(),
        "subject": f.subject,
        "detail": f.detail,
    })
}

fn clearance_value(v: Option<i64>) -> Value {
    json!(v)
}

/// The check as JSON, the same every time.
pub fn enclosure_json(
    scene: &Scene3d,
    cavity: &Cavity,
    spec: &EnclosureSpec,
    checks: &EnclosureChecks,
    extra: &[Finding],
) -> String {
    let findings: Vec<Value> = checks
        .findings
        .iter()
        .chain(extra.iter())
        .map(finding_value)
        .collect();
    let posts: Vec<Value> = spec
        .posts
        .iter()
        .map(|p| {
            json!({
                "x_nm": p.x_nm,
                "y_nm": p.y_nm,
                "diameter_nm": p.diameter_nm,
                "hole_nm": p.hole_nm,
                "top_nm": p.top_nm,
            })
        })
        .collect();
    let doc = json!({
        "format": "phoenix-enclosure/1",
        "board": {"name": scene.name, "thickness_nm": scene.thickness_nm, "components": scene.parts.len()},
        "cavity": {
            "outline_nm": cavity.outline.iter().map(|p| json!([p.0, p.1])).collect::<Vec<Value>>(),
            "floor_nm": cavity.floor_nm,
            "lid_nm": cavity.lid_nm,
            "wall_nm": spec.wall_nm,
            "standoff_nm": spec.standoff_nm,
            "min_clearance_nm": spec.min_clearance_nm,
        },
        "posts": posts,
        "least_clearance_nm": {
            "wall": clearance_value(checks.least.wall_nm),
            "lid": clearance_value(checks.least.lid_nm),
            "floor": clearance_value(checks.least.floor_nm),
            "post": clearance_value(checks.least.post_nm),
        },
        "findings": findings,
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// The check as text, with the titles of the codes in `lang`.
pub fn enclosure_text(
    scene: &Scene3d,
    cavity: &Cavity,
    spec: &EnclosureSpec,
    checks: &EnclosureChecks,
    extra: &[Finding],
    lang: Lang,
) -> String {
    let xs = cavity.outline.iter().map(|p| p.0);
    let ys = cavity.outline.iter().map(|p| p.1);
    let mut out = format!(
        "board {}: {} mm thick, {} components with a box\n",
        scene.name,
        millimetres(scene.thickness_nm),
        scene.parts.len()
    );
    out.push_str(&format!(
        "cavity: x {} to {} mm, y {} to {} mm, floor {} mm, lid {} mm ({} corners); walls {} mm, standoff {} mm, least clearance {} mm\n",
        mm(xs.clone().min().unwrap_or(0)),
        mm(xs.max().unwrap_or(0)),
        mm(ys.clone().min().unwrap_or(0)),
        mm(ys.max().unwrap_or(0)),
        mm(cavity.floor_nm),
        mm(cavity.lid_nm),
        cavity.outline.len(),
        mm(spec.wall_nm),
        mm(spec.standoff_nm),
        mm(spec.min_clearance_nm)
    ));
    let all: Vec<&Finding> = checks.findings.iter().chain(extra.iter()).collect();
    let count = |s: Severity| all.iter().filter(|f| f.severity == s).count();
    out.push_str(&format!(
        "findings: {} errors, {} warnings, {} information\n",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    ));
    for f in all {
        out.push_str(&f.line(lang));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Part3d, PartSource};

    const MM: i64 = 1_000_000;

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> [P; 4] {
        [
            (x0 * MM, y0 * MM),
            (x1 * MM, y0 * MM),
            (x1 * MM, y1 * MM),
            (x0 * MM, y1 * MM),
        ]
    }

    fn part(reference: &str, corners: [P; 4], z_min: i64, z_max: i64, bottom: bool) -> Part3d {
        Part3d {
            reference: reference.to_owned(),
            footprint: "F".to_owned(),
            bottom,
            corners,
            z_min,
            z_max,
            source: PartSource::Table,
        }
    }

    /// A board 40 by 30 mm, 1.6 mm thick, with a part of 10 by 10 by 4 mm in the middle.
    fn scene() -> Scene3d {
        Scene3d {
            name: "demo".to_owned(),
            outline: rect(0, 0, 40, 30).to_vec(),
            thickness_nm: 1_600_000,
            parts: vec![part(
                "U1",
                rect(15, 10, 25, 20),
                1_600_000,
                5_600_000,
                false,
            )],
            findings: Vec::new(),
        }
    }

    fn spec(text: &str) -> EnclosureSpec {
        EnclosureSpec::parse(text).unwrap()
    }

    fn check(scene: &Scene3d, holes: &[Hole], spec: &EnclosureSpec) -> EnclosureChecks {
        let cavity = Cavity::new(spec, &scene.outline, None).unwrap();
        check_enclosure(scene, holes, &cavity, spec, &AssemblySettings::default())
    }

    fn codes(checks: &EnclosureChecks) -> Vec<(&'static str, Severity)> {
        checks
            .findings
            .iter()
            .map(|f| (f.code, f.severity))
            .collect()
    }

    #[test]
    fn a_box_is_read_with_its_walls_posts_and_defaults() {
        let s = spec(
            "# a case\nbox = box:50x40x20\nwall_mm = 2.5\nstandoff_mm = 4\nmin_clearance_mm = 1\npost = 5, 5, 6, 3.2\npost = 35,25,6,3.2,0.5\nx_mm = -3\n",
        );
        assert_eq!(
            s.inner,
            Inner::Box {
                length_nm: 50 * MM,
                width_nm: 40 * MM
            }
        );
        assert_eq!(s.height_nm, 20 * MM);
        assert_eq!(
            (s.wall_nm, s.standoff_nm, s.min_clearance_nm),
            (2_500_000, 4 * MM, MM)
        );
        assert_eq!((s.x_nm, s.y_nm), (Some(-3 * MM), None));
        assert_eq!(s.posts.len(), 2);
        assert_eq!(
            s.posts[1],
            Post {
                x_nm: 35 * MM,
                y_nm: 25 * MM,
                diameter_nm: 6 * MM,
                hole_nm: Some(3_200_000),
                top_nm: Some(500_000)
            }
        );
        assert_eq!(s.posts[0].top_nm, None);
        let d = spec("box = 10x10x5");
        assert_eq!(
            (d.wall_nm, d.standoff_nm, d.min_clearance_nm),
            (2 * MM, 3 * MM, 500_000)
        );
        assert_eq!(d.post_hole_tolerance_nm, 300_000);
    }

    #[test]
    fn the_example_file_that_is_shipped_is_read() {
        let s = spec(include_str!("../../../config/enclosure.cfg"));
        assert_eq!(
            s.inner,
            Inner::Box {
                length_nm: 60 * MM,
                width_nm: 45 * MM
            }
        );
        assert_eq!(
            (s.height_nm, s.wall_nm, s.standoff_nm),
            (25 * MM, 2 * MM, 4 * MM)
        );
        assert_eq!(s.posts.len(), 4);
        assert_eq!(s.posts[3].x_nm, 56_500_000);
        assert_eq!(s.posts[3].hole_nm, Some(3_200_000));
    }

    #[test]
    fn a_contour_needs_a_height_and_names_a_safe_file() {
        let s = spec("contour = case/inner.dxf\ncontour_layer = INNER\nheight_mm = 18.5\n");
        assert_eq!(
            s.inner,
            Inner::Contour {
                path: "case/inner.dxf".to_owned(),
                layer: Some("INNER".to_owned())
            }
        );
        assert_eq!(s.height_nm, 18_500_000);
    }

    #[test]
    fn what_is_wrong_in_the_file_is_refused_with_its_line() {
        for (text, needle) in [
            ("", "give the inside"),
            ("box = 10x10\n", "line 1: the value of box"),
            ("box = 10x0x5\n", "line 1: the value of box"),
            (
                "box = 1x1x1\nbox = 2x2x2\n",
                "line 2: give box or contour, once",
            ),
            (
                "box = 1x1x1\ncontour = a.dxf\n",
                "line 2: give box or contour, once",
            ),
            ("contour = a.dxf\n", "needs height_mm"),
            ("contour = ../a.dxf\nheight_mm = 5\n", "relative name"),
            ("contour = /a.dxf\nheight_mm = 5\n", "relative name"),
            ("contour = C:a.dxf\nheight_mm = 5\n", "relative name"),
            (
                "box = 1x1x1\nheight_mm = 5\n",
                "line 2: the height of a box",
            ),
            ("box = 1x1x1\ncontour_layer = A\n", "belongs to a contour"),
            (
                "box = 1x1x1\nwall_mm = -1\n",
                "line 2: the value of wall_mm",
            ),
            ("box = 1x1x1\npost = 1,2\n", "line 2: a post is"),
            ("box = 1x1x1\npost = 1,2,0\n", "line 2: the value of post"),
            ("box = 1x1x1\nfoo = 1\n", "line 2: unknown key foo"),
            ("box 1x1x1\n", "line 1: not key = value"),
        ] {
            let e = EnclosureSpec::parse(text).unwrap_err();
            assert_eq!(e.code, SPEC_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let many: String = std::iter::once("box = 1x1x1\n".to_owned())
            .chain((0..65).map(|_| "post = 0,0,1\n".to_owned()))
            .collect();
        assert!(EnclosureSpec::parse(&many)
            .unwrap_err()
            .detail
            .contains("limit of 64"));
    }

    #[test]
    fn a_box_is_centred_on_the_board_and_the_floor_and_the_lid_follow_the_standoff() {
        let s = spec("box = 50x40x20\nstandoff_mm = 3\n");
        let c = Cavity::new(&s, &scene().outline, None).unwrap();
        // the board is 40 by 30 about (20, 15): the box 50 by 40 goes from (-5, -5) to (45, 35)
        assert_eq!(c.outline, rect(-5, -5, 45, 35).to_vec());
        assert_eq!((c.floor_nm, c.lid_nm), (-3 * MM, 17 * MM));
        let placed = spec("box = 50x40x20\nx_mm = 1\ny_mm = 2\n");
        let c = Cavity::new(&placed, &scene().outline, None).unwrap();
        assert_eq!(c.outline[0], (MM, 2 * MM));
        assert_eq!(c.outline[2], (51 * MM, 42 * MM));
        let e = Cavity::new(&s, &[], None).unwrap_err();
        assert_eq!(e.code, SPEC_INVALID);
    }

    #[test]
    fn a_contour_is_shifted_and_turned_counter_clockwise_and_must_have_area() {
        let s = spec("contour = a.dxf\nheight_mm = 10\nx_mm = 1\n");
        let clockwise = [(0, 0), (0, 10 * MM), (10 * MM, 10 * MM), (10 * MM, 0)];
        let c = Cavity::new(&s, &scene().outline, Some(&clockwise[..])).unwrap();
        assert!(twice_area(&c.outline) > 0);
        assert!(c.outline.contains(&(11 * MM, 10 * MM)));
        for bad in [
            None,
            Some(&clockwise[..2]),
            Some(&[(0, 0), (MM, 0), (2 * MM, 0)][..]),
        ] {
            let e = Cavity::new(&s, &scene().outline, bad).unwrap_err();
            assert_eq!(e.code, CONTOUR_UNUSABLE);
        }
    }

    #[test]
    fn a_board_that_fits_gives_only_its_least_clearances() {
        // the box is 50 by 40 about the board: 5 mm to every wall; the lid is 17 mm up, the part
        // reaches 5.6 mm; the floor is 3 mm under the board
        let s = spec("box = 50x40x20\nstandoff_mm = 3\n");
        let checks = check(&scene(), &[], &s);
        assert_eq!(codes(&checks), vec![(CLEARANCES, Severity::Info)]);
        assert_eq!(checks.least.wall_nm, Some(5 * MM));
        assert_eq!(checks.least.lid_nm, Some(17 * MM - 5_600_000));
        assert_eq!(checks.least.floor_nm, Some(3 * MM));
        assert_eq!(checks.least.post_nm, None);
        assert_eq!(
            checks.findings[0].detail,
            "walls 5.000000 mm, lid 11.400000 mm, floor 3.000000 mm"
        );
    }

    #[test]
    fn a_board_too_near_a_wall_is_a_warning_and_one_outside_is_an_error() {
        // 41 by 31 inside: 0.5 mm to each side of the board: exactly the least, no finding
        let s = spec("box = 41x31x20\n");
        assert_eq!(codes(&check(&scene(), &[], &s)).len(), 1);
        // 40.6 by 30.6: 0.3 mm to each wall, the board is nearer than 0.5 mm
        let near = spec("box = 40.6x30.6x20\n");
        let checks = check(&scene(), &[], &near);
        assert_eq!(
            codes(&checks),
            vec![(WALL, Severity::Warning), (CLEARANCES, Severity::Info)]
        );
        assert_eq!(checks.findings[0].subject.as_deref(), Some("board"));
        assert_eq!(
            checks.findings[0].detail,
            "0.300000 mm from a wall, the least is 0.500000 mm"
        );
        assert_eq!(checks.least.wall_nm, Some(300_000));
        // 38 by 30: the board is 1 mm outside on each side along x
        let tight = spec("box = 38x30x20\n");
        let checks = check(&scene(), &[], &tight);
        let wall = checks.findings.iter().find(|f| f.code == WALL).unwrap();
        assert_eq!(wall.severity, Severity::Error);
        assert_eq!(wall.detail, "it is outside the walls by 1.000000 mm");
        assert_eq!(checks.least.wall_nm, Some(-MM));
        // the errors come first
        assert_eq!(checks.findings[0].severity, Severity::Error);
    }

    #[test]
    fn a_component_through_the_lid_or_the_floor_is_an_error_and_one_near_it_a_warning() {
        let mut s = scene();
        s.parts
            .push(part("U2", rect(2, 2, 6, 6), -2_000_000, 0, true));
        // the lid is at 5.6 + 0.3 = 5.9 mm: floor -3, height 8.9
        let spec_close = spec("box = 50x40x8.9\nstandoff_mm = 3\n");
        let checks = check(&s, &[], &spec_close);
        let lid = checks.findings.iter().find(|f| f.code == LID).unwrap();
        assert_eq!(lid.severity, Severity::Warning);
        assert_eq!(lid.subject.as_deref(), Some("U1"));
        assert_eq!(
            lid.detail,
            "0.300000 mm from the lid, the least is 0.500000 mm"
        );
        // the floor: U2 hangs 2 mm under the board, the floor is 3 mm under it
        assert_eq!(checks.least.floor_nm, Some(MM));
        // a lid at 5 mm and a floor at -1.5 mm: U1 is 0.6 mm through the lid, U2 0.5 mm through the floor
        let through = spec("box = 50x40x6.5\nstandoff_mm = 1.5\n");
        let checks = check(&s, &[], &through);
        let lid = checks.findings.iter().find(|f| f.code == LID).unwrap();
        assert_eq!(lid.severity, Severity::Error);
        assert_eq!(lid.detail, "it is through the lid by 0.600000 mm");
        let floor: Vec<&Finding> = checks.findings.iter().filter(|f| f.code == FLOOR).collect();
        assert_eq!(floor.len(), 1);
        assert_eq!(floor[0].subject.as_deref(), Some("U2"));
        assert_eq!(floor[0].detail, "it is through the floor by 0.500000 mm");
        // the board itself is 1.5 mm above the floor: no finding for it
        assert!(floor.iter().all(|f| f.subject.as_deref() != Some("board")));
    }

    #[test]
    fn a_component_across_a_wall_of_a_concave_cavity_is_found() {
        // an L: 30 by 30 with the corner x > 15, y > 15 cut away
        let l = [
            (0, 0),
            (30 * MM, 0),
            (30 * MM, 15 * MM),
            (15 * MM, 15 * MM),
            (15 * MM, 30 * MM),
            (0, 30 * MM),
        ];
        let s = spec("contour = a.dxf\nheight_mm = 20\nx_mm = 0\ny_mm = 0\n");
        let cavity = Cavity::new(&s, &[], Some(&l[..])).unwrap();
        // a part with its four corners inside the L, 12..28 by 12..14 is inside; 10..20 by 10..20
        // has a corner at (20, 20), which is outside
        let mut scene = scene();
        scene.outline = rect(0, 0, 10, 10).to_vec();
        scene.parts = vec![
            part("A", rect(12, 5, 28, 10), 1_600_000, 3_000_000, false),
            part("B", rect(10, 10, 20, 20), 1_600_000, 3_000_000, false),
        ];
        let checks = check_enclosure(&scene, &[], &cavity, &s, &AssemblySettings::default());
        let walls: Vec<&Finding> = checks.findings.iter().filter(|f| f.code == WALL).collect();
        assert!(walls.iter().any(|f| f.subject.as_deref() == Some("B")
            && f.severity == Severity::Error
            && f.detail == "it is outside the walls by 5.000000 mm"));
        assert!(walls
            .iter()
            .all(|f| f.subject.as_deref() != Some("A") || f.severity == Severity::Warning));
        // a box whose corners are all inside the L but whose edge crosses the notch
        let across = [
            (14 * MM, 14 * MM),
            (29 * MM, 14 * MM),
            (29 * MM, 14_500_000),
            (14 * MM, 14_500_000),
        ];
        assert_eq!(
            against_walls(&across, &cavity.outline),
            Wall::Inside(500_000)
        );
        let crossing = [
            (10 * MM, 10 * MM),
            (20 * MM, 10 * MM),
            (20 * MM, 20 * MM),
            (10 * MM, 20 * MM),
        ];
        assert!(matches!(against_walls(&crossing, &cavity.outline), Wall::Outside(d) if d > 0));
        // a thin quadrilateral with its four corners inside the L (two in each arm) whose long
        // side cuts the notch: x + y = 30.5 meets the wall x = 15 at y = 15.5
        let sliver = [
            (16_500_000, 14 * MM),
            (17 * MM, 13 * MM),
            (13 * MM, 17 * MM),
            (14 * MM, 16_500_000),
        ];
        assert!(sliver.iter().all(|&c| point_in_polygon(c, &cavity.outline)));
        assert_eq!(against_walls(&sliver, &cavity.outline), Wall::Outside(0));
    }

    #[test]
    fn a_post_is_hit_by_a_part_on_the_underside_and_not_by_one_on_top() {
        let mut s = scene();
        // a part under the board at 8..12 by 8..12, 2 mm tall
        s.parts
            .push(part("B1", rect(8, 8, 12, 12), -2_000_000, 0, true));
        // a post of 6 mm at (10, 10): the part is right over its centre
        let sp = spec("box = 50x40x20\nstandoff_mm = 3\npost = 10,10,6,3\n");
        let checks = check(&s, &[], &sp);
        let hit = checks
            .findings
            .iter()
            .find(|f| f.code == POST_COLLISION)
            .unwrap();
        assert_eq!(hit.severity, Severity::Error);
        assert_eq!(hit.subject.as_deref(), Some("B1 and post 1"));
        assert_eq!(
            hit.detail,
            "the box of the component enters the post by 3.000000 mm"
        );
        // the part on top (U1) is not in the way of the post: it stands above the board
        assert!(checks
            .findings
            .iter()
            .all(|f| f.subject.as_deref() != Some("U1 and post 1")));
        // a post of 6 mm at (18, 10): the part ends 6 mm away, the post reaches 3 mm: 3 mm free
        let aside = spec("box = 50x40x20\nstandoff_mm = 3\npost = 18,10,6,3\n");
        let checks = check(&s, &[], &aside);
        assert!(checks.findings.iter().all(|f| f.code != POST_COLLISION));
        assert_eq!(checks.least.post_nm, Some(3 * MM));
    }

    #[test]
    fn a_post_needs_a_hole_of_the_board_under_it_and_a_place_under_the_board() {
        let sp = spec("box = 50x40x20\npost = 5,5,6,3.2\npost = 35,25,6,3.2\npost = 60,5,6\n");
        let holes = [
            Hole {
                at: (5 * MM, 5 * MM + 100_000),
                diameter_nm: 3_200_000,
            },
            Hole {
                at: (35 * MM, 25 * MM),
                diameter_nm: 2_500_000,
            },
        ];
        let checks = check(&scene(), &holes, &sp);
        let by_subject = |subject: &str, code: &str| {
            checks
                .findings
                .iter()
                .find(|f| f.code == code && f.subject.as_deref() == Some(subject))
        };
        // post 1: a hole of 3.2 mm 0.1 mm from the centre: fine
        assert!(by_subject("post 1", POST_NO_HOLE).is_none());
        // post 2: the hole is 2.5 mm, the screw needs 3.2 mm
        assert_eq!(
            by_subject("post 2", POST_NO_HOLE).unwrap().detail,
            "the hole is 2.500000 mm, the screw needs 3.200000 mm"
        );
        // post 3: outside the board and no hole
        assert!(by_subject("post 3", POST_OUTSIDE).is_some());
        assert!(by_subject("post 3", POST_NO_HOLE)
            .unwrap()
            .detail
            .starts_with("no hole of the board has its centre within 0.300000 mm"));
    }

    #[test]
    fn the_findings_of_a_code_are_limited_and_the_rest_counted() {
        let mut s = scene();
        s.parts = (0..5)
            .map(|i| {
                part(
                    &format!("P{i}"),
                    rect(60, 0, 62, 2),
                    1_600_000,
                    2_000_000,
                    false,
                )
            })
            .collect();
        let sp = spec("box = 50x40x20\n");
        let cavity = Cavity::new(&sp, &s.outline, None).unwrap();
        let limited = AssemblySettings {
            max_findings: 2,
            ..AssemblySettings::default()
        };
        let checks = check_enclosure(&s, &[], &cavity, &sp, &limited);
        assert_eq!(checks.findings.iter().filter(|f| f.code == WALL).count(), 2);
        let more = checks
            .findings
            .iter()
            .find(|f| f.code == LIMIT_EXCEEDED)
            .unwrap();
        assert_eq!(more.subject.as_deref(), Some(WALL));
        assert!(
            more.detail.starts_with("3 more findings"),
            "{}",
            more.detail
        );
    }

    #[test]
    fn the_holes_of_the_solid_are_the_ones_inside_the_outline_clear_of_the_edges_and_of_each_other()
    {
        let outline = rect(0, 0, 20, 20);
        let holes = [
            Hole {
                at: (5 * MM, 5 * MM),
                diameter_nm: 2 * MM,
            },
            // the same place again: overlaps the one before it
            Hole {
                at: (5 * MM, 5 * MM),
                diameter_nm: 3 * MM,
            },
            // touches the edge x = 0
            Hole {
                at: (MM, 10 * MM),
                diameter_nm: 2 * MM,
            },
            // outside
            Hole {
                at: (30 * MM, 10 * MM),
                diameter_nm: MM,
            },
            // clear of the edge by 0.5 mm
            Hole {
                at: (1_500_000, 15 * MM),
                diameter_nm: 2 * MM,
            },
            // no size
            Hole {
                at: (10 * MM, 10 * MM),
                diameter_nm: 0,
            },
            // next to the first one without touching it
            Hole {
                at: (8 * MM, 5 * MM),
                diameter_nm: MM,
            },
        ];
        let (kept, findings) = usable_holes(&outline, &holes);
        let at: Vec<P> = kept.iter().map(|h| h.at).collect();
        assert_eq!(
            at,
            vec![(1_500_000, 15 * MM), (5 * MM, 5 * MM), (8 * MM, 5 * MM)]
        );
        assert_eq!(findings.len(), 3);
        let details: Vec<&str> = findings.iter().map(|f| f.detail.as_str()).collect();
        assert!(details.contains(&"its centre is outside the outline"));
        assert!(details.contains(&"it touches or crosses an edge of the outline"));
        assert!(details.contains(&"it overlaps a hole before it"));
        assert!(findings.iter().all(|f| f.code == HOLE_SKIPPED));
    }

    #[test]
    fn the_report_names_the_cavity_and_every_finding_in_text_and_in_json() {
        let sp = spec("box = 38x30x20\npost = 5,5,6\n");
        let cavity = Cavity::new(&sp, &scene().outline, None).unwrap();
        let checks = check_enclosure(&scene(), &[], &cavity, &sp, &AssemblySettings::default());
        let text = enclosure_text(&scene(), &cavity, &sp, &checks, &[], Lang::En);
        assert!(text.starts_with("board demo: 1.600000 mm thick, 1 components with a box\n"));
        assert!(text.contains("cavity: x 1.000000 to 39.000000 mm, y 0.000000 to 30.000000 mm, floor -3.000000 mm, lid 17.000000 mm (4 corners)"));
        assert!(text.contains("error: PHOENIX-ENCL-005: Board or component is outside the walls or too close to them [board]: it is outside the walls by 1.000000 mm"));
        let pt = enclosure_text(&scene(), &cavity, &sp, &checks, &[], Lang::Pt);
        assert!(
            pt.contains("A placa ou um componente está fora das paredes ou demasiado perto delas")
        );
        let json: Value =
            serde_json::from_str(&enclosure_json(&scene(), &cavity, &sp, &checks, &[])).unwrap();
        assert_eq!(json["format"], "phoenix-enclosure/1");
        assert_eq!(json["cavity"]["lid_nm"], 17_000_000);
        assert_eq!(json["least_clearance_nm"]["wall"], -1_000_000);
        assert_eq!(json["posts"][0]["diameter_nm"], 6_000_000);
        assert_eq!(json["findings"][0]["code"], "PHOENIX-ENCL-005");
        // the same input gives the same bytes
        assert_eq!(
            enclosure_json(&scene(), &cavity, &sp, &checks, &[]),
            enclosure_json(&scene(), &cavity, &sp, &checks, &[])
        );
    }
}
