// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene as a simplified model: the board as a prism of its outline and its thickness, and
//! each component as a box, written as OBJ and as STEP. The copper, the pads, the holes and the
//! real shape of the parts are not in it; [`export_note`] says so as a finding.

use crate::codes::{Finding, Severity, EXPORT_SIMPLIFIED};
use crate::geom::P;
use crate::prism::{prisms_obj, prisms_step, Prism};
use crate::scene::Scene3d;

/// The prisms of a scene: the board first (when its outline has at least three points), then the
/// components by reference.
pub fn scene_prisms(scene: &Scene3d) -> Vec<Prism> {
    let mut out = Vec::with_capacity(scene.parts.len() + 1);
    if scene.outline.len() >= 3 {
        out.push(Prism {
            name: "board".to_owned(),
            base: scene.outline.clone(),
            z_low: 0,
            z_high: scene.thickness_nm,
        });
    }
    for part in &scene.parts {
        let base: Vec<P> = part.corners.to_vec();
        out.push(Prism {
            name: part.reference.clone(),
            base,
            z_low: part.z_min,
            z_high: part.z_max,
        });
    }
    out
}

/// The scene as an OBJ file.
pub fn scene_obj(scene: &Scene3d) -> String {
    prisms_obj(&scene_prisms(scene))
}

/// The scene as a STEP file.
pub fn scene_step(scene: &Scene3d) -> String {
    prisms_step(&scene_prisms(scene))
}

/// The finding that tells that the export is a simplified model.
pub fn export_note(scene: &Scene3d) -> Finding {
    Finding::new(
        EXPORT_SIMPLIFIED,
        Severity::Info,
        None,
        format!(
            "{} components as boxes and the board as a prism of its outline; copper, pads, holes and the real shape of the parts are not in the file",
            scene.parts.len()
        ),
    )
}
