// SPDX-License-Identifier: GPL-3.0-or-later
//! The 3D window of the assembled board, as data and with no window: an orbiting camera, the
//! perspective projection of a point worked out by hand, the faces of the board and of the
//! components sorted from far to near, the selection of a face under a click, the state of each
//! component (collision, too tall, outside the outline) taken from the checks of
//! `eh3d-phoenix-assembly3d`, and the colours.
//!
//! The scene is `eh3d_phoenix_assembly3d::Scene3d`: the board as a prism of the thickness of its
//! stack-up and each component as a box. Lengths are millimetres; the board plane is x and y, the
//! height is z, and the camera turns around the vertical axis. A face is drawn with the painter
//! algorithm (far faces first); the faces that look away from the camera are left out, which is
//! right for closed boxes. A face with a corner behind the camera is left out as well, so the
//! camera keeps a distance from what it looks at.

use std::path::Path;

use eh3d_phoenix_assembly3d::codes::{
    FILE_TOO_LARGE, LIMIT_EXCEEDED, MODEL_UNREADABLE, STEP_EMPTY, STEP_INVALID, STEP_UNIT,
};
use eh3d_phoenix_assembly3d::{reach_nm, read_step, AssemblyError, StepError, StepSolid};
use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::Lang;

use crate::codes::{ViewError, FILE_UNUSABLE};
use crate::prim::{point_in_polygon, triangulate, P};
use crate::scene3d::{
    build_scene, check_scene, scene_bounds, AssemblySettings, Box3, Checks, ModelTable, Scene3d,
};
use crate::settings::ViewerSettings;

/// A point or a direction in three dimensions.
pub type V3 = [f64; 3];

const NM_PER_MM: f64 = 1_000_000.0;
/// Corners nearer to the camera than this (in millimetres, along the view) are not drawn.
const NEAR_MM: f64 = 1e-3;
/// The widest the camera may go from the scene, as a multiple of its first distance.
const FAR_FACTOR: f64 = 20.0;

fn mm(nm: i64) -> f64 {
    nm as f64 / NM_PER_MM
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(a: V3) -> V3 {
    let length = dot(a, a).sqrt();
    if length > 0.0 {
        [a[0] / length, a[1] / length, a[2] / length]
    } else {
        a
    }
}

/// The camera: it looks at `target` from `distance` millimetres away, turned `yaw_deg` around the
/// vertical axis (0 is in front of the board, looking along +y) and raised `pitch_deg` above the
/// board plane (negative looks from below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The point looked at.
    pub target: V3,
    /// The turn around the vertical axis, in degrees.
    pub yaw_deg: f64,
    /// The rise above the board plane, in degrees.
    pub pitch_deg: f64,
    /// The distance from the target, in millimetres.
    pub distance: f64,
    /// The farthest the camera may go, in millimetres.
    pub max_distance: f64,
    /// The vertical field of view, in degrees.
    pub fov_deg: f64,
    /// The width of the window in pixels.
    pub width_px: f64,
    /// The height of the window in pixels.
    pub height_px: f64,
}

/// What the camera needs to project a point, worked out once for a picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projector {
    /// Where the camera is.
    pub eye: V3,
    /// To the right of the picture.
    pub right: V3,
    /// To the top of the picture.
    pub up: V3,
    /// Into the picture.
    pub forward: V3,
    /// The distance of the picture plane, in pixels.
    pub focal_px: f64,
    /// The pixel of the middle of the window, across.
    pub cx: f64,
    /// The pixel of the middle of the window, down.
    pub cy: f64,
}

impl Projector {
    /// The pixel of a point and its depth (the distance along the view), or `None` when the point
    /// is behind the camera.
    pub fn project(&self, p: V3) -> Option<(f64, f64, f64)> {
        let v = sub(p, self.eye);
        let depth = dot(v, self.forward);
        if depth <= NEAR_MM {
            return None;
        }
        let x = dot(v, self.right);
        let y = dot(v, self.up);
        Some((
            self.cx + self.focal_px * x / depth,
            self.cy - self.focal_px * y / depth,
            depth,
        ))
    }
}

impl Camera {
    /// A camera that shows a whole box, from the start angles of the settings.
    pub fn for_box(bounds: Box3, settings: &ViewerSettings) -> Camera {
        let size = bounds.size();
        let radius = (0.5 * dot(size, size).sqrt()).max(1.0);
        let half_fov = (settings.view3d_fov_deg / 2.0).to_radians();
        let distance = (radius / half_fov.sin() * settings.view3d_fit_factor)
            .max(settings.view3d_min_distance_mm);
        Camera {
            target: bounds.centre(),
            yaw_deg: settings.view3d_start_yaw_deg,
            pitch_deg: settings.view3d_start_pitch_deg,
            distance,
            max_distance: distance * FAR_FACTOR,
            fov_deg: settings.view3d_fov_deg,
            width_px: 800.0,
            height_px: 600.0,
        }
    }

    /// Tells the camera how large the window is now.
    pub fn resize(&mut self, width_px: f64, height_px: f64) {
        if width_px.is_finite() && height_px.is_finite() && width_px > 0.0 && height_px > 0.0 {
            self.width_px = width_px;
            self.height_px = height_px;
        }
    }

    /// Where the camera is.
    pub fn eye(&self) -> V3 {
        let yaw = self.yaw_deg.to_radians();
        let pitch = self.pitch_deg.to_radians();
        [
            self.target[0] + self.distance * yaw.sin() * pitch.cos(),
            self.target[1] - self.distance * yaw.cos() * pitch.cos(),
            self.target[2] + self.distance * pitch.sin(),
        ]
    }

    /// The projection of this camera.
    pub fn projector(&self) -> Projector {
        let eye = self.eye();
        let forward = normalize(sub(self.target, eye));
        let right = normalize(cross(forward, [0.0, 0.0, 1.0]));
        let up = cross(right, forward);
        Projector {
            eye,
            right,
            up,
            forward,
            focal_px: self.height_px / 2.0 / (self.fov_deg / 2.0).to_radians().tan(),
            cx: self.width_px / 2.0,
            cy: self.height_px / 2.0,
        }
    }

    /// The pixel of a point and its depth, or `None` when it is behind the camera.
    pub fn project(&self, p: V3) -> Option<(f64, f64, f64)> {
        self.projector().project(p)
    }

    /// Turns the camera around the scene for a drag of `dx` and `dy` pixels: a drag to the right
    /// turns the scene to the right, a drag down tilts it towards the viewer from above.
    pub fn orbit(&mut self, dx_px: f64, dy_px: f64, settings: &ViewerSettings) {
        if !dx_px.is_finite() || !dy_px.is_finite() {
            return;
        }
        let rate = settings.view3d_rotate_deg_per_px;
        let yaw = (self.yaw_deg - dx_px * rate + 180.0).rem_euclid(360.0) - 180.0;
        self.yaw_deg = yaw;
        let limit = settings.view3d_max_pitch_deg;
        self.pitch_deg = (self.pitch_deg + dy_px * rate).clamp(-limit, limit);
    }

    /// Moves the camera nearer (a positive wheel) or farther, inside its limits.
    pub fn zoom(&mut self, scroll: f64, settings: &ViewerSettings) {
        if !scroll.is_finite() {
            return;
        }
        let factor = (-scroll * settings.view3d_zoom_wheel_rate).exp();
        let farthest = self.max_distance.max(settings.view3d_min_distance_mm);
        self.distance = (self.distance * factor).clamp(settings.view3d_min_distance_mm, farthest);
    }

    /// Moves the point looked at for a drag of `dx` and `dy` pixels, so that the scene follows the
    /// pointer (at the speed of the settings, 1 being exactly).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64, settings: &ViewerSettings) {
        if !dx_px.is_finite() || !dy_px.is_finite() {
            return;
        }
        let p = self.projector();
        let per_px = self.distance / p.focal_px * settings.view3d_pan_speed;
        let step = |i: usize| (p.up[i] * dy_px - p.right[i] * dx_px) * per_px;
        self.target = [
            self.target[0] + step(0),
            self.target[1] + step(1),
            self.target[2] + step(2),
        ];
    }
}

/// Which part of the assembly a face belongs to and which way it looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceKind {
    /// The top of the board.
    BoardTop,
    /// The bottom of the board.
    BoardBottom,
    /// A side of the board.
    BoardSide,
    /// The top of a component.
    PartTop,
    /// The bottom of a component.
    PartBottom,
    /// A side of a component.
    PartSide,
}

/// A flat face of the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// The corners, in millimetres.
    pub points: Vec<V3>,
    /// The triangles that fill it (indexes into `points`).
    pub tris: Vec<[usize; 3]>,
    /// The direction it looks, away from the solid.
    pub normal: V3,
    /// The middle of the corners.
    pub centre: V3,
    /// What it is.
    pub kind: FaceKind,
    /// The place of its component in the scene, or `None` for the board.
    pub part: Option<usize>,
    /// True for a face of a component on the bottom side.
    pub bottom: bool,
}

/// The faces of the whole assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct Model3d {
    /// The faces: the board first, then each component.
    pub faces: Vec<Face>,
    /// The box that holds them all; `None` when there is nothing.
    pub bounds: Option<Box3>,
    /// The height of the middle of the board, which tells whether the camera is above or below.
    pub board_mid_z: f64,
}

fn signed_area2(points: &[(f64, f64)]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum()
}

fn counter_clockwise(mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    if signed_area2(&points) < 0.0 {
        points.reverse();
    }
    points
}

fn centre_of(points: &[V3]) -> V3 {
    let n = points.len().max(1) as f64;
    let mut sum = [0.0; 3];
    for p in points {
        for (axis, value) in sum.iter_mut().enumerate() {
            *value += p[axis];
        }
    }
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn flat_face(
    plane: &[(f64, f64)],
    z: f64,
    up: bool,
    kind: FaceKind,
    part: Option<usize>,
    bottom: bool,
) -> Face {
    let points: Vec<V3> = plane.iter().map(|p| [p.0, p.1, z]).collect();
    let tris = if plane.len() <= 512 {
        let flat: Vec<P> = plane.iter().map(|p| P::new(p.0, p.1)).collect();
        triangulate(&flat)
            .into_iter()
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect()
    } else {
        (1..plane.len().saturating_sub(1))
            .map(|i| [0, i, i + 1])
            .collect()
    };
    let centre = centre_of(&points);
    Face {
        points,
        tris,
        normal: [0.0, 0.0, if up { 1.0 } else { -1.0 }],
        centre,
        kind,
        part,
        bottom,
    }
}

fn side_faces(
    plane: &[(f64, f64)],
    z_low: f64,
    z_high: f64,
    kind: FaceKind,
    part: Option<usize>,
    bottom: bool,
    out: &mut Vec<Face>,
) {
    for (a, b) in plane.iter().zip(plane.iter().cycle().skip(1)) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dy);
        if length < 1e-12 {
            continue;
        }
        let points = vec![
            [a.0, a.1, z_low],
            [b.0, b.1, z_low],
            [b.0, b.1, z_high],
            [a.0, a.1, z_high],
        ];
        let centre = centre_of(&points);
        out.push(Face {
            points,
            tris: vec![[0, 1, 2], [0, 2, 3]],
            normal: [dy / length, -dx / length, 0.0],
            centre,
            kind,
            part,
            bottom,
        });
    }
}

impl Model3d {
    /// The faces of a scene. Heights are multiplied by `height_scale` (1 is true to scale).
    pub fn from_scene(scene: &Scene3d, height_scale: f64) -> Model3d {
        let scale = if height_scale.is_finite() && height_scale > 0.0 {
            height_scale
        } else {
            1.0
        };
        let mut faces = Vec::new();
        let top = mm(scene.thickness_nm) * scale;
        if scene.outline.len() >= 3 {
            let plane =
                counter_clockwise(scene.outline.iter().map(|p| (mm(p.0), mm(p.1))).collect());
            faces.push(flat_face(
                &plane,
                top,
                true,
                FaceKind::BoardTop,
                None,
                false,
            ));
            faces.push(flat_face(
                &plane,
                0.0,
                false,
                FaceKind::BoardBottom,
                None,
                false,
            ));
            side_faces(
                &plane,
                0.0,
                top,
                FaceKind::BoardSide,
                None,
                false,
                &mut faces,
            );
        }
        for (index, part) in scene.parts.iter().enumerate() {
            let plane =
                counter_clockwise(part.corners.iter().map(|c| (mm(c.0), mm(c.1))).collect());
            let (low, high) = (mm(part.z_min) * scale, mm(part.z_max) * scale);
            let owner = Some(index);
            faces.push(flat_face(
                &plane,
                high,
                true,
                FaceKind::PartTop,
                owner,
                part.bottom,
            ));
            faces.push(flat_face(
                &plane,
                low,
                false,
                FaceKind::PartBottom,
                owner,
                part.bottom,
            ));
            side_faces(
                &plane,
                low,
                high,
                FaceKind::PartSide,
                owner,
                part.bottom,
                &mut faces,
            );
        }
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];
        let mut any = false;
        for face in &faces {
            for p in &face.points {
                any = true;
                for (axis, value) in p.iter().enumerate() {
                    min[axis] = min[axis].min(*value);
                    max[axis] = max[axis].max(*value);
                }
            }
        }
        Model3d {
            faces,
            bounds: any.then_some(Box3 { min, max }),
            board_mid_z: top / 2.0,
        }
    }
}

/// A face as it lies on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedFace {
    /// The place of the face in the model.
    pub face: usize,
    /// The corners as pixels.
    pub points: Vec<(f64, f64)>,
    /// The depth of its middle along the view.
    pub depth: f64,
    /// The light on it, from 0.45 (edge on) to 1 (facing the camera).
    pub shade: f64,
}

/// What the person clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// The board.
    Board,
    /// The component at this place of the scene.
    Part(usize),
}

impl Camera {
    /// The faces that the camera can see, in the order to draw them: the ones behind first. A
    /// component that hangs below the board is drawn before the board when the camera is above it
    /// and after it when the camera is below, and the other way round for the ones on top.
    pub fn frame(&self, model: &Model3d) -> Vec<ProjectedFace> {
        let projector = self.projector();
        let above = projector.eye[2] > model.board_mid_z;
        let mut visible: Vec<(u8, ProjectedFace)> = Vec::new();
        for (index, face) in model.faces.iter().enumerate() {
            let to_eye = sub(projector.eye, face.centre);
            let facing = dot(face.normal, normalize(to_eye));
            if facing <= 0.0 {
                continue;
            }
            let Some((_, _, depth)) = projector.project(face.centre) else {
                continue;
            };
            let corners: Option<Vec<(f64, f64)>> = face
                .points
                .iter()
                .map(|p| projector.project(*p).map(|(x, y, _)| (x, y)))
                .collect();
            let Some(points) = corners else {
                continue;
            };
            let rank = match (face.part, face.bottom) {
                (None, _) => 1,
                (Some(_), true) => {
                    if above {
                        0
                    } else {
                        2
                    }
                }
                (Some(_), false) => {
                    if above {
                        2
                    } else {
                        0
                    }
                }
            };
            visible.push((
                rank,
                ProjectedFace {
                    face: index,
                    points,
                    depth,
                    shade: 0.45 + 0.55 * facing,
                },
            ));
        }
        visible.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.depth.total_cmp(&a.1.depth)));
        visible.into_iter().map(|(_, f)| f).collect()
    }
}

/// What lies under a pixel: the face drawn last that holds it. The answer is the place of the face
/// in the model and what it belongs to.
pub fn pick(
    frame: &[ProjectedFace],
    model: &Model3d,
    sx: f64,
    sy: f64,
) -> Option<(usize, Selection)> {
    let at = P::new(sx, sy);
    frame.iter().rev().find_map(|f| {
        let polygon: Vec<P> = f.points.iter().map(|p| P::new(p.0, p.1)).collect();
        if !point_in_polygon(at, &polygon) {
            return None;
        }
        let selection = match model.faces[f.face].part {
            Some(index) => Selection::Part(index),
            None => Selection::Board,
        };
        Some((f.face, selection))
    })
}

/// What the checks found about one component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PartState {
    /// Its box overlaps the box of another component.
    pub collides: bool,
    /// It stands higher (or hangs lower) than the limit.
    pub too_tall: bool,
    /// Its box is not inside the outline of the board.
    pub outside: bool,
}

/// The state of every component of a scene, by place.
pub fn part_states(scene: &Scene3d, checks: &Checks) -> Vec<PartState> {
    let mut states = vec![PartState::default(); scene.parts.len()];
    for collision in &checks.collisions {
        for index in [collision.first, collision.second] {
            if let Some(state) = states.get_mut(index) {
                state.collides = true;
            }
        }
    }
    for index in &checks.too_tall {
        if let Some(state) = states.get_mut(*index) {
            state.too_tall = true;
        }
    }
    for index in &checks.outside {
        if let Some(state) = states.get_mut(*index) {
            state.outside = true;
        }
    }
    states
}

/// What the window tells about a selected component.
#[derive(Debug, Clone, PartialEq)]
pub struct PartInfo {
    /// The reference.
    pub reference: String,
    /// The name of the footprint.
    pub footprint: String,
    /// True when it is on the bottom side.
    pub bottom: bool,
    /// The height of its box, in millimetres.
    pub height_mm: f64,
    /// How far it stands above the board (or hangs below it), in millimetres.
    pub reach_mm: f64,
    /// The lowest height of its box, in millimetres.
    pub z_min_mm: f64,
    /// The highest height of its box, in millimetres.
    pub z_max_mm: f64,
    /// The references of the components whose boxes overlap its box.
    pub collides_with: Vec<String>,
    /// True when it is higher than the limit.
    pub too_tall: bool,
    /// True when it is not inside the outline.
    pub outside: bool,
}

/// The information on the component at a place of the scene.
pub fn part_info(scene: &Scene3d, checks: &Checks, index: usize) -> Option<PartInfo> {
    let part = scene.parts.get(index)?;
    let mut collides_with: Vec<String> = checks
        .collisions
        .iter()
        .filter_map(|c| {
            let other = if c.first == index {
                c.second
            } else if c.second == index {
                c.first
            } else {
                return None;
            };
            scene.parts.get(other).map(|p| p.reference.clone())
        })
        .collect();
    collides_with.sort();
    Some(PartInfo {
        reference: part.reference.clone(),
        footprint: part.footprint.clone(),
        bottom: part.bottom,
        height_mm: mm(part.z_max - part.z_min),
        reach_mm: mm(reach_nm(part, scene.thickness_nm)),
        z_min_mm: mm(part.z_min),
        z_max_mm: mm(part.z_max),
        collides_with,
        too_tall: checks.too_tall.contains(&index),
        outside: checks.outside.contains(&index),
    })
}

/// The totals of the assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// The thickness of the board, in millimetres.
    pub board_thickness_mm: f64,
    /// The number of components.
    pub parts: usize,
    /// The pairs of components that collide.
    pub collisions: usize,
    /// The components higher than the limit.
    pub too_tall: usize,
    /// The components outside the outline.
    pub outside: usize,
    /// The component that stands highest above the board (or hangs lowest) and how far, when
    /// there is one that reaches out at all.
    pub tallest: Option<(String, f64)>,
}

/// The totals of a scene and its checks.
pub fn summary(scene: &Scene3d, checks: &Checks) -> Summary {
    let tallest = scene
        .parts
        .iter()
        .map(|p| (p, reach_nm(p, scene.thickness_nm)))
        .filter(|(_, reach)| *reach > 0)
        .max_by_key(|(_, reach)| *reach)
        .map(|(p, reach)| (p.reference.clone(), mm(reach)));
    Summary {
        board_thickness_mm: mm(scene.thickness_nm),
        parts: scene.parts.len(),
        collisions: checks.collisions.len(),
        too_tall: checks.too_tall.len(),
        outside: checks.outside.len(),
        tallest,
    }
}

/// A brightness between 0.88 and 1.12 that depends only on the reference, so that neighbouring
/// components are told apart and keep their tint.
pub fn tint(reference: &str) -> f64 {
    let mut hash: u32 = 2_166_136_261;
    for byte in reference.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    0.88 + 0.24 * f64::from(hash % 1000) / 999.0
}

/// A colour with its red, green and blue multiplied by a factor.
pub fn shade_colour(colour: [u8; 4], factor: f64) -> [u8; 4] {
    let channel = |c: u8| (f64::from(c) * factor).round().clamp(0.0, 255.0) as u8;
    [
        channel(colour[0]),
        channel(colour[1]),
        channel(colour[2]),
        colour[3],
    ]
}

/// The colour of a face: the board has its own, a component has the colour of its side (a little
/// lighter or darker by its reference), a component with a collision, one that is too tall and
/// one that is outside the outline have the colours of those marks (in that order of priority),
/// and what is selected has the colour of the selection. `shade` is the light on the face.
pub fn face_colour(
    settings: &ViewerSettings,
    face: &Face,
    reference: Option<&str>,
    state: PartState,
    selected: bool,
    shade: f64,
) -> [u8; 4] {
    let base = if selected {
        settings.colour("view3d_selected")
    } else if face.part.is_none() {
        settings.colour("view3d_board")
    } else if state.collides {
        settings.colour("view3d_collision")
    } else if state.too_tall {
        settings.colour("view3d_too_tall")
    } else if state.outside {
        settings.colour("view3d_outside")
    } else {
        let side = if face.bottom {
            "view3d_bottom_part"
        } else {
            "view3d_top_part"
        };
        shade_colour(settings.colour(side), tint(reference.unwrap_or("")))
    };
    shade_colour(base, shade)
}

/// The assembly ready to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembly {
    /// The scene built by `eh3d-phoenix-assembly3d`.
    pub scene: Scene3d,
    /// Its checks.
    pub checks: Checks,
    /// The state of each component.
    pub states: Vec<PartState>,
    /// The faces.
    pub model: Model3d,
}

impl Assembly {
    /// The assembly of a scene.
    pub fn from_scene(
        scene: Scene3d,
        asm: &AssemblySettings,
        settings: &ViewerSettings,
    ) -> Assembly {
        let checks = check_scene(&scene, asm);
        let states = part_states(&scene, &checks);
        let model = Model3d::from_scene(&scene, settings.view3d_height_scale);
        Assembly {
            scene,
            checks,
            states,
            model,
        }
    }

    /// A camera that shows all of it.
    pub fn camera(&self, settings: &ViewerSettings) -> Camera {
        let bounds = self
            .model
            .bounds
            .or_else(|| scene_bounds(&self.scene))
            .unwrap_or(Box3 {
                min: [0.0, 0.0, 0.0],
                max: [10.0, 10.0, 1.0],
            });
        Camera::for_box(bounds, settings)
    }
}

fn assembly_error(error: &AssemblyError) -> ViewError {
    ViewError::new(FILE_UNUSABLE, error.message(Lang::En))
}

/// Reads the STEP file that a table names, next to the table.
pub fn read_model(
    folder: &Path,
    name: &str,
    asm: &AssemblySettings,
) -> Result<StepSolid, AssemblyError> {
    let unreadable =
        |e: std::io::Error| AssemblyError::new(MODEL_UNREADABLE, format!("{name}: {e}"));
    let path = folder.join(name);
    let size = std::fs::metadata(&path).map_err(unreadable)?.len();
    if size > asm.max_file_bytes {
        return Err(AssemblyError::new(FILE_TOO_LARGE, name));
    }
    let bytes = std::fs::read(&path).map_err(unreadable)?;
    let text = String::from_utf8_lossy(&bytes);
    read_step(&text, asm.max_step_entities).map_err(|e| match e {
        StepError::NotStep(d) | StepError::Syntax(d) => AssemblyError::new(STEP_INVALID, d),
        StepError::Unit(d) => AssemblyError::new(STEP_UNIT, d),
        StepError::NoGeometry => AssemblyError::new(STEP_EMPTY, ""),
        StepError::TooMany(n) => {
            AssemblyError::new(LIMIT_EXCEEDED, format!("more than {n} entities"))
        }
    })
}

/// Reads the table of models from a file.
pub fn load_table(path: &Path, asm: &AssemblySettings) -> Result<ModelTable, ViewError> {
    let unusable =
        |e: std::io::Error| ViewError::new(FILE_UNUSABLE, format!("{}: {e}", path.display()));
    let size = std::fs::metadata(path).map_err(unusable)?.len();
    if size > asm.max_file_bytes {
        return Err(ViewError::new(
            FILE_UNUSABLE,
            format!(
                "{}: larger than {} bytes",
                path.display(),
                asm.max_file_bytes
            ),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(unusable)?;
    ModelTable::parse(&text, asm.max_components).map_err(|e| assembly_error(&e))
}

/// Builds the assembly of a board with a table of models. `resolve` reads the STEP files that the
/// table names. A component whose footprint has no model is left out and a finding says so.
pub fn assemble(
    board: &Board,
    thickness_nm: Option<i64>,
    table: &ModelTable,
    resolve: &mut dyn FnMut(&str) -> Result<StepSolid, AssemblyError>,
    asm: &AssemblySettings,
    settings: &ViewerSettings,
) -> Result<Assembly, ViewError> {
    let scene =
        build_scene(board, thickness_nm, table, asm, resolve).map_err(|e| assembly_error(&e))?;
    Ok(Assembly::from_scene(scene, asm, settings))
}

/// Builds the assembly of a board with the table in a file, or with no table (the board alone).
pub fn assemble_with_table_file(
    board: &Board,
    table_path: Option<&Path>,
    asm: &AssemblySettings,
    settings: &ViewerSettings,
) -> Result<Assembly, ViewError> {
    let (table, folder) = match table_path {
        Some(path) => (
            load_table(path, asm)?,
            path.parent().map(Path::to_path_buf).unwrap_or_default(),
        ),
        None => (ModelTable::default(), std::path::PathBuf::new()),
    };
    let mut resolve = |name: &str| read_model(&folder, name, asm);
    assemble(board, None, &table, &mut resolve, asm, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use eh3d_phoenix_assembly3d::{Part3d, PartSource};

    const MM: i64 = 1_000_000;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn settings() -> ViewerSettings {
        ViewerSettings::default()
    }

    fn part(reference: &str, x: (i64, i64), y: (i64, i64), z: (i64, i64), bottom: bool) -> Part3d {
        Part3d {
            reference: reference.to_owned(),
            footprint: "FP".to_owned(),
            bottom,
            corners: [
                (x.0 * MM, y.0 * MM),
                (x.1 * MM, y.0 * MM),
                (x.1 * MM, y.1 * MM),
                (x.0 * MM, y.1 * MM),
            ],
            z_min: z.0,
            z_max: z.1,
            source: PartSource::Table,
        }
    }

    fn scene(parts: Vec<Part3d>) -> Scene3d {
        Scene3d {
            name: "b".to_owned(),
            outline: vec![(0, 0), (10 * MM, 0), (10 * MM, 8 * MM), (0, 8 * MM)],
            thickness_nm: 1_600_000,
            parts,
            findings: Vec::new(),
        }
    }

    fn one_part() -> Scene3d {
        scene(vec![part(
            "U1",
            (2, 4),
            (2, 3),
            (1_600_000, 6_600_000),
            false,
        )])
    }

    fn front_camera() -> Camera {
        Camera {
            target: [0.0, 0.0, 0.0],
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            distance: 100.0,
            max_distance: 2000.0,
            fov_deg: 40.0,
            width_px: 800.0,
            height_px: 600.0,
        }
    }

    #[test]
    fn a_point_is_projected_by_hand_from_the_camera() {
        let camera = front_camera();
        let focal = 300.0 / (20.0_f64).to_radians().tan();
        let (x, y, depth) = camera.project([0.0, 0.0, 0.0]).unwrap();
        assert!(near(x, 400.0) && near(y, 300.0) && near(depth, 100.0));
        // 10 mm to the right of the target: to the right of the middle
        let (x, y, _) = camera.project([10.0, 0.0, 0.0]).unwrap();
        assert!(near(x, 400.0 + focal * 10.0 / 100.0) && near(y, 300.0));
        // 10 mm above it: higher on the screen, so a smaller row
        let (x, y, _) = camera.project([0.0, 0.0, 10.0]).unwrap();
        assert!(near(x, 400.0) && near(y, 300.0 - focal * 10.0 / 100.0));
        // a point farther away is nearer to the middle
        let (x, _, depth) = camera.project([10.0, 100.0, 0.0]).unwrap();
        assert!(near(depth, 200.0) && near(x, 400.0 + focal * 10.0 / 200.0));
        // a point behind the camera has no pixel
        assert!(camera.project([0.0, -150.0, 0.0]).is_none());
        assert!(camera.project([0.0, -100.0, 0.0]).is_none());
    }

    #[test]
    fn the_eye_turns_with_the_angles() {
        let mut camera = front_camera();
        assert!(camera
            .eye()
            .iter()
            .zip([0.0, -100.0, 0.0])
            .all(|(a, b)| near(*a, b)));
        camera.pitch_deg = 89.999999;
        assert!(camera.eye()[2] > 99.99);
        camera.pitch_deg = 0.0;
        camera.yaw_deg = 90.0;
        let eye = camera.eye();
        assert!(near(eye[0], 100.0) && eye[1].abs() < 1e-9);
        // from the side, the board's +y runs to the right of the picture
        let (x, _, _) = camera.project([0.0, 10.0, 0.0]).unwrap();
        assert!(x > 400.0);
    }

    #[test]
    fn the_drag_orbits_the_wheel_zooms_and_the_pan_follows_the_pointer() {
        let s = settings();
        let mut camera = front_camera();
        camera.orbit(100.0, 50.0, &s);
        assert!(near(camera.yaw_deg, -40.0) && near(camera.pitch_deg, 20.0));
        camera.orbit(-1000.0, 0.0, &s);
        assert!(near(camera.yaw_deg, 0.0 - 40.0 + 400.0 - 360.0));
        camera.orbit(0.0, 10_000.0, &s);
        assert!(near(camera.pitch_deg, 89.0));
        camera.orbit(0.0, -100_000.0, &s);
        assert!(near(camera.pitch_deg, -89.0));
        let before = camera;
        camera.orbit(f64::NAN, 1.0, &s);
        assert_eq!(camera, before);

        let mut camera = front_camera();
        camera.zoom(100.0, &s);
        assert!(near(
            camera.distance,
            100.0 * (-100.0 * s.view3d_zoom_wheel_rate).exp()
        ));
        camera.zoom(1.0e9, &s);
        assert!(near(camera.distance, s.view3d_min_distance_mm));
        camera.zoom(-1.0e9, &s);
        assert!(near(camera.distance, 2000.0));
        camera.zoom(f64::NAN, &s);
        assert!(near(camera.distance, 2000.0));

        // the point under the target follows the pointer by the pixels of the drag
        let mut camera = front_camera();
        let mark = [3.0, 0.0, 2.0];
        let (x0, y0, _) = camera.project(mark).unwrap();
        camera.pan(30.0, -20.0, &s);
        let (x1, y1, _) = camera.project(mark).unwrap();
        assert!(near(x1 - x0, 30.0), "{}", x1 - x0);
        assert!(near(y1 - y0, -20.0), "{}", y1 - y0);
        camera.resize(0.0, 10.0);
        assert!(near(camera.width_px, 800.0));
        camera.resize(1000.0, 500.0);
        assert!(near(camera.width_px, 1000.0) && near(camera.height_px, 500.0));
    }

    #[test]
    fn a_camera_for_a_box_looks_at_its_middle_from_far_enough() {
        let s = settings();
        let bounds = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [100.0, 80.0, 10.0],
        };
        let camera = Camera::for_box(bounds, &s);
        assert_eq!(camera.target, [50.0, 40.0, 5.0]);
        assert!(near(camera.yaw_deg, s.view3d_start_yaw_deg));
        assert!(near(camera.pitch_deg, s.view3d_start_pitch_deg));
        // every corner of the box is in front of the camera and inside the window
        for x in [0.0, 100.0] {
            for y in [0.0, 80.0] {
                for z in [0.0, 10.0] {
                    let (px, py, _) = camera.project([x, y, z]).unwrap();
                    assert!(
                        (0.0..=800.0).contains(&px) && (0.0..=600.0).contains(&py),
                        "{px} {py}"
                    );
                }
            }
        }
        let tiny = Camera::for_box(
            Box3 {
                min: [0.0; 3],
                max: [0.1, 0.1, 0.1],
            },
            &s,
        );
        assert!(tiny.distance >= s.view3d_min_distance_mm);
    }

    #[test]
    fn the_model_has_the_faces_of_the_board_and_of_each_component() {
        let model = Model3d::from_scene(&one_part(), 1.0);
        // the board: top, bottom and four sides; the component: the same
        assert_eq!(model.faces.len(), 12);
        let count = |kind: FaceKind| model.faces.iter().filter(|f| f.kind == kind).count();
        assert_eq!(count(FaceKind::BoardTop), 1);
        assert_eq!(count(FaceKind::BoardBottom), 1);
        assert_eq!(count(FaceKind::BoardSide), 4);
        assert_eq!(count(FaceKind::PartTop), 1);
        assert_eq!(count(FaceKind::PartSide), 4);
        let b = model.bounds.unwrap();
        assert_eq!(b.min, [0.0, 0.0, 0.0]);
        assert_eq!(b.max, [10.0, 8.0, 6.6]);
        assert!(near(model.board_mid_z, 0.8));
        // a scale of 2 doubles every height and nothing else
        let tall = Model3d::from_scene(&one_part(), 2.0).bounds.unwrap();
        assert_eq!(tall.max, [10.0, 8.0, 13.2]);
        let nonsense = Model3d::from_scene(&one_part(), f64::NAN).bounds.unwrap();
        assert_eq!(nonsense.max, b.max);
        // the faces belong to their component
        assert!(model.faces.iter().filter(|f| f.part == Some(0)).count() == 6);
        // no outline and no component is no model
        let mut empty = one_part();
        empty.outline.clear();
        empty.parts.clear();
        let nothing = Model3d::from_scene(&empty, 1.0);
        assert!(nothing.faces.is_empty() && nothing.bounds.is_none());
    }

    #[test]
    fn every_normal_looks_away_from_the_solid_whatever_the_order_of_the_corners() {
        // the outline and the component given clockwise
        let mut s = one_part();
        s.outline.reverse();
        s.parts[0].corners.reverse();
        let model = Model3d::from_scene(&s, 1.0);
        for face in &model.faces {
            let owner = match face.part {
                Some(_) => [3.0, 2.5, 4.1],
                None => [5.0, 4.0, 0.8],
            };
            let outward = sub(face.centre, owner);
            assert!(
                dot(face.normal, outward) > 0.0,
                "{:?} {:?}",
                face.kind,
                face.normal
            );
            assert!(near(dot(face.normal, face.normal), 1.0));
        }
    }

    #[test]
    fn a_concave_top_is_triangulated_into_the_right_number_of_triangles() {
        let mut s = one_part();
        s.outline = vec![
            (0, 0),
            (10 * MM, 0),
            (10 * MM, 8 * MM),
            (5 * MM, 3 * MM),
            (0, 8 * MM),
        ];
        let model = Model3d::from_scene(&s, 1.0);
        let top = model
            .faces
            .iter()
            .find(|f| f.kind == FaceKind::BoardTop)
            .unwrap();
        assert_eq!(top.tris.len(), 3);
        assert!(top.tris.iter().all(|t| t.iter().all(|i| *i < 5)));
    }

    fn looking_down() -> Camera {
        let mut camera = Camera::for_box(
            Model3d::from_scene(&one_part(), 1.0).bounds.unwrap(),
            &settings(),
        );
        camera.yaw_deg = 0.0;
        camera.pitch_deg = 89.0;
        camera
    }

    #[test]
    fn from_above_the_tops_are_seen_and_the_bottoms_are_not() {
        let model = Model3d::from_scene(&one_part(), 1.0);
        let frame = looking_down().frame(&model);
        let kinds: Vec<FaceKind> = frame.iter().map(|f| model.faces[f.face].kind).collect();
        assert!(kinds.contains(&FaceKind::BoardTop) && kinds.contains(&FaceKind::PartTop));
        assert!(!kinds.contains(&FaceKind::BoardBottom) && !kinds.contains(&FaceKind::PartBottom));
        // the component is drawn after the board it stands on
        let at = |kind: FaceKind| kinds.iter().position(|k| *k == kind).unwrap();
        assert!(at(FaceKind::BoardTop) < at(FaceKind::PartTop));
        // the top looks straight at the camera: nearly full light
        let top = frame
            .iter()
            .find(|f| model.faces[f.face].kind == FaceKind::BoardTop)
            .unwrap();
        assert!(top.shade > 0.99);
        // from below the bottoms are seen and the tops are not
        let mut below = looking_down();
        below.pitch_deg = -89.0;
        let frame = below.frame(&model);
        let kinds: Vec<FaceKind> = frame.iter().map(|f| model.faces[f.face].kind).collect();
        assert!(kinds.contains(&FaceKind::BoardBottom));
        assert!(!kinds.contains(&FaceKind::BoardTop) && !kinds.contains(&FaceKind::PartTop));
    }

    #[test]
    fn faces_are_drawn_from_far_to_near_and_components_hanging_under_the_board_go_first() {
        let s = scene(vec![
            part("U1", (2, 4), (2, 3), (1_600_000, 6_600_000), false),
            part("U2", (6, 8), (2, 3), (-2_000_000, 0), true),
        ]);
        let model = Model3d::from_scene(&s, 1.0);
        let rank = |f: &ProjectedFace| match (model.faces[f.face].part, model.faces[f.face].bottom)
        {
            (None, _) => 1,
            (Some(_), true) => 0,
            (Some(_), false) => 2,
        };
        let above = Camera::for_box(model.bounds.unwrap(), &settings());
        let ranks: Vec<i32> = above.frame(&model).iter().map(&rank).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{ranks:?}");
        assert!(ranks.contains(&0) && ranks.contains(&1) && ranks.contains(&2));
        // from below the order is the other way round
        let mut below = above;
        below.pitch_deg = -35.0;
        let ranks: Vec<i32> = below.frame(&model).iter().map(|f| 2 - rank(f)).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{ranks:?}");
        // inside a group the faces are sorted by depth, the far ones first
        let frame = above.frame(&model);
        let board: Vec<f64> = frame
            .iter()
            .filter(|f| model.faces[f.face].part.is_none())
            .map(|f| f.depth)
            .collect();
        assert!(board.windows(2).all(|w| w[0] >= w[1]), "{board:?}");
    }

    #[test]
    fn a_click_picks_the_component_the_board_or_nothing() {
        let model = Model3d::from_scene(&one_part(), 1.0);
        let camera = looking_down();
        let frame = camera.frame(&model);
        // the middle of the top of the component
        let top = model
            .faces
            .iter()
            .find(|f| f.kind == FaceKind::PartTop)
            .unwrap();
        let (x, y, _) = camera.project(top.centre).unwrap();
        let (face, selection) = pick(&frame, &model, x, y).unwrap();
        assert_eq!(selection, Selection::Part(0));
        assert_eq!(model.faces[face].kind, FaceKind::PartTop);
        // a place on the board away from the component
        let (x, y, _) = camera.project([8.5, 6.0, 1.6]).unwrap();
        let (_, selection) = pick(&frame, &model, x, y).unwrap();
        assert_eq!(selection, Selection::Board);
        // the corner of the window shows nothing
        assert!(pick(&frame, &model, 1.0, 1.0).is_none());
        assert!(pick(&[], &model, 400.0, 300.0).is_none());
    }

    fn checked_scene() -> Scene3d {
        scene(vec![
            // two components whose boxes overlap
            part("C1", (1, 4), (1, 4), (1_600_000, 3_600_000), false),
            part("C2", (3, 6), (2, 5), (1_600_000, 3_600_000), false),
            // one far too tall
            part("T1", (7, 9), (5, 7), (1_600_000, 41_600_000), false),
            // one outside the outline
            part("O1", (11, 13), (1, 2), (1_600_000, 2_600_000), false),
        ])
    }

    #[test]
    fn the_checks_become_the_state_the_information_and_the_totals() {
        let s = checked_scene();
        let checks = check_scene(&s, &AssemblySettings::default());
        let states = part_states(&s, &checks);
        assert_eq!(
            states,
            vec![
                PartState {
                    collides: true,
                    ..PartState::default()
                },
                PartState {
                    collides: true,
                    ..PartState::default()
                },
                PartState {
                    too_tall: true,
                    ..PartState::default()
                },
                PartState {
                    outside: true,
                    ..PartState::default()
                },
            ]
        );
        let c1 = part_info(&s, &checks, 0).unwrap();
        assert_eq!(c1.reference, "C1");
        assert_eq!(c1.footprint, "FP");
        assert_eq!(c1.collides_with, vec!["C2".to_owned()]);
        assert!(near(c1.height_mm, 2.0) && near(c1.reach_mm, 2.0));
        assert!(near(c1.z_min_mm, 1.6) && near(c1.z_max_mm, 3.6));
        assert!(!c1.too_tall && !c1.outside && !c1.bottom);
        let t1 = part_info(&s, &checks, 2).unwrap();
        assert!(t1.too_tall && t1.collides_with.is_empty());
        assert!(near(t1.reach_mm, 40.0));
        assert!(part_info(&s, &checks, 3).unwrap().outside);
        assert!(part_info(&s, &checks, 9).is_none());
        let total = summary(&s, &checks);
        assert_eq!(total.parts, 4);
        assert_eq!(total.collisions, 1);
        assert_eq!((total.too_tall, total.outside), (1, 1));
        assert!(near(total.board_thickness_mm, 1.6));
        let (reference, reach) = total.tallest.unwrap();
        assert_eq!(reference, "T1");
        assert!(near(reach, 40.0));
        // a board without components has no tallest one
        let bare = scene(Vec::new());
        assert!(summary(&bare, &Checks::default()).tallest.is_none());
        // a component on the bottom reaches out below the board
        let low = scene(vec![part("B1", (2, 4), (2, 3), (-3_000_000, 0), true)]);
        let info = part_info(&low, &Checks::default(), 0).unwrap();
        assert!(info.bottom && near(info.reach_mm, 3.0));
    }

    #[test]
    fn the_colours_follow_the_marks_the_selection_and_the_light() {
        let s = settings();
        let model = Model3d::from_scene(&checked_scene(), 1.0);
        let face_of = |part: Option<usize>| {
            model
                .faces
                .iter()
                .find(|f| {
                    f.part == part && matches!(f.kind, FaceKind::PartTop | FaceKind::BoardTop)
                })
                .unwrap()
        };
        let plain = PartState::default();
        let board = face_colour(&s, face_of(None), None, plain, false, 1.0);
        assert_eq!(board, s.colour("view3d_board"));
        let part_face = face_of(Some(0));
        let normal = face_colour(&s, part_face, Some("C1"), plain, false, 1.0);
        let expected = shade_colour(s.colour("view3d_top_part"), tint("C1"));
        assert_eq!(normal, expected);
        for (state, name) in [
            (
                PartState {
                    collides: true,
                    too_tall: true,
                    outside: true,
                },
                "view3d_collision",
            ),
            (
                PartState {
                    too_tall: true,
                    outside: true,
                    ..PartState::default()
                },
                "view3d_too_tall",
            ),
            (
                PartState {
                    outside: true,
                    ..PartState::default()
                },
                "view3d_outside",
            ),
        ] {
            assert_eq!(
                face_colour(&s, part_face, Some("C1"), state, false, 1.0),
                s.colour(name)
            );
        }
        let state = PartState {
            collides: true,
            ..PartState::default()
        };
        assert_eq!(
            face_colour(&s, part_face, Some("C1"), state, true, 1.0),
            s.colour("view3d_selected")
        );
        // the light only changes red, green and blue
        let dark = face_colour(&s, face_of(None), None, plain, false, 0.5);
        assert_eq!(dark[3], s.colour("view3d_board")[3]);
        assert!(dark[1] < board[1]);
        assert_eq!(shade_colour([200, 100, 50, 7], 2.0), [255, 200, 100, 7]);
        assert_eq!(shade_colour([200, 100, 50, 7], 0.0), [0, 0, 0, 7]);
    }

    #[test]
    fn the_tint_depends_only_on_the_reference_and_stays_in_its_range() {
        assert_eq!(tint("R1").to_bits(), tint("R1").to_bits());
        let mut seen = std::collections::BTreeSet::new();
        for n in 0..50 {
            let t = tint(&format!("C{n}"));
            assert!((0.88..=1.12).contains(&t), "{t}");
            seen.insert(t.to_bits());
        }
        assert!(seen.len() > 10);
        assert!(tint("") >= 0.88);
    }

    #[test]
    fn a_board_and_a_table_give_an_assembly_with_its_checks_and_camera() {
        let board = sample::board();
        let table = ModelTable::parse("footprint,model\nFP,box:4x2x1\n", 100).unwrap();
        let mut resolve = |_: &str| -> Result<StepSolid, AssemblyError> {
            Err(AssemblyError::new(MODEL_UNREADABLE, "none"))
        };
        let s = settings();
        let asm = AssemblySettings::default();
        let built = assemble(&board, None, &table, &mut resolve, &asm, &s).unwrap();
        assert_eq!(built.scene.parts.len(), 1);
        assert_eq!(built.scene.parts[0].reference, "R1");
        assert_eq!(built.states, vec![PartState::default()]);
        assert!(built.checks.collisions.is_empty());
        let b = built.model.bounds.unwrap();
        assert!(b.max[2] > 1.6);
        let camera = built.camera(&s);
        assert!(camera
            .project(built.model.bounds.unwrap().centre())
            .is_some());
        // a limit lower than the component marks it
        let strict = AssemblySettings {
            max_component_height_nm: 500_000,
            ..AssemblySettings::default()
        };
        let marked = assemble(&board, None, &table, &mut resolve, &strict, &s).unwrap();
        assert!(marked.states[0].too_tall);
        // without a model the board is alone and a finding says so
        let alone = assemble(&board, None, &ModelTable::default(), &mut resolve, &asm, &s).unwrap();
        assert!(alone.scene.parts.is_empty() && !alone.scene.findings.is_empty());
        assert!(alone.camera(&s).distance > 0.0);
    }

    #[test]
    fn a_table_file_is_read_next_to_its_models_and_a_missing_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.csv");
        std::fs::write(&path, "footprint,model\nFP,box:4x2x1\n").unwrap();
        let s = settings();
        let asm = AssemblySettings::default();
        let built =
            assemble_with_table_file(&sample::board(), Some(path.as_path()), &asm, &s).unwrap();
        assert_eq!(built.scene.parts.len(), 1);
        let alone = assemble_with_table_file(&sample::board(), None, &asm, &s).unwrap();
        assert!(alone.scene.parts.is_empty());
        let missing = dir.path().join("nowhere.csv");
        assert_eq!(
            assemble_with_table_file(&sample::board(), Some(missing.as_path()), &asm, &s)
                .unwrap_err()
                .code,
            FILE_UNUSABLE
        );
        std::fs::write(&path, "nonsense").unwrap();
        assert_eq!(load_table(&path, &asm).unwrap_err().code, FILE_UNUSABLE);
        assert_eq!(
            read_model(dir.path(), "none.step", &asm).unwrap_err().code,
            MODEL_UNREADABLE
        );
    }
}
