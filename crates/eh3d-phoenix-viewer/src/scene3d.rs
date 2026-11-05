// SPDX-License-Identifier: GPL-3.0-or-later
//! The 3D scene of the assembled board, as data for the viewer and with no window: the board as
//! a prism of the thickness of its stack-up and each component as a box, built by
//! `eh3d-phoenix-assembly3d` from the models of a table. This module hands the scene to the
//! viewer in millimetres: the box that holds everything (to place the camera) and the outline of
//! the board and of each component on the board plane, in the same points the 2D view uses.

pub use eh3d_phoenix_assembly3d::{
    build_scene, check_scene, AssemblySettings, Checks, ModelTable, Part3d, PartSource, Scene3d,
};

use crate::prim::P;

const NM_PER_MM: f64 = 1_000_000.0;

/// A box in three dimensions, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box3 {
    /// The least corner.
    pub min: [f64; 3],
    /// The greatest corner.
    pub max: [f64; 3],
}

impl Box3 {
    /// The length, the width and the height.
    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// The middle of the box.
    pub fn centre(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }
}

fn mm(nm: i64) -> f64 {
    nm as f64 / NM_PER_MM
}

/// A point of the board plane in nanometres as a point of the view.
fn plane_point(p: (i64, i64)) -> P {
    P::new(mm(p.0), mm(p.1))
}

/// The outline of the board on the plane, in millimetres.
pub fn board_outline(scene: &Scene3d) -> Vec<P> {
    scene.outline.iter().copied().map(plane_point).collect()
}

/// The outline of a component on the plane (its four corners), in millimetres.
pub fn part_outline(part: &Part3d) -> Vec<P> {
    part.corners.iter().copied().map(plane_point).collect()
}

/// The box that holds the board and every component, in millimetres; `None` when the scene has
/// neither an outline nor a component.
pub fn scene_bounds(scene: &Scene3d) -> Option<Box3> {
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    let mut any = false;
    let mut include = |x: i64, y: i64, z: i64| {
        any = true;
        for (axis, value) in [x, y, z].into_iter().enumerate() {
            let v = mm(value);
            min[axis] = min[axis].min(v);
            max[axis] = max[axis].max(v);
        }
    };
    for p in &scene.outline {
        include(p.0, p.1, 0);
        include(p.0, p.1, scene.thickness_nm);
    }
    for part in &scene.parts {
        for c in &part.corners {
            include(c.0, c.1, part.z_min);
            include(c.0, c.1, part.z_max);
        }
    }
    any.then_some(Box3 { min, max })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    fn scene() -> Scene3d {
        Scene3d {
            name: "b".to_owned(),
            outline: vec![(0, 0), (10 * MM, 0), (10 * MM, 8 * MM), (0, 8 * MM)],
            thickness_nm: 1_600_000,
            parts: vec![Part3d {
                reference: "U1".to_owned(),
                footprint: "F".to_owned(),
                bottom: false,
                corners: [
                    (2 * MM, 2 * MM),
                    (4 * MM, 2 * MM),
                    (4 * MM, 3 * MM),
                    (2 * MM, 3 * MM),
                ],
                z_min: 1_600_000,
                z_max: 6_600_000,
                source: PartSource::Table,
            }],
            findings: Vec::new(),
        }
    }

    #[test]
    fn the_bounds_hold_the_board_and_the_tallest_component() {
        let b = scene_bounds(&scene()).unwrap();
        assert_eq!(b.min, [0.0, 0.0, 0.0]);
        assert_eq!(b.max, [10.0, 8.0, 6.6]);
        assert_eq!(b.size(), [10.0, 8.0, 6.6]);
        assert_eq!(b.centre(), [5.0, 4.0, 3.3]);
    }

    #[test]
    fn a_component_that_hangs_below_the_board_lowers_the_box() {
        let mut s = scene();
        s.parts[0].bottom = true;
        s.parts[0].z_min = -2_000_000;
        s.parts[0].z_max = 0;
        let b = scene_bounds(&s).unwrap();
        assert_eq!(b.min[2], -2.0);
        assert_eq!(b.max[2], 1.6);
    }

    #[test]
    fn an_empty_scene_has_no_bounds_and_the_outlines_are_in_millimetres() {
        let mut empty = scene();
        empty.outline.clear();
        empty.parts.clear();
        assert!(scene_bounds(&empty).is_none());
        let s = scene();
        assert_eq!(board_outline(&s)[2], P::new(10.0, 8.0));
        assert_eq!(part_outline(&s.parts[0])[0], P::new(2.0, 2.0));
        assert_eq!(part_outline(&s.parts[0]).len(), 4);
    }
}
