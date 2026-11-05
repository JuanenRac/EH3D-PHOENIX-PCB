// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene of the assembled board as data: the board as a prism of the thickness of its
//! stack-up, and each component as a box, in nanometres, with the place it stands at.
//!
//! The board lies between height 0 (its bottom) and its thickness (its top). A component on the
//! top side stands on the top, from the height its model starts at, and one on the bottom side
//! hangs below height 0 with its model turned upside down. A component on the bottom side is
//! mirrored along x before it is turned by its rotation (so the rotation is the one the board
//! file stores). The box of a component is the box of its model, moved by the table, turned by
//! the footprint and moved to the place of the component; nothing is added to it.
//!
//! The scene has no screen and no window: the viewer of the suite draws it, and
//! [`crate::export`] writes it.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Side};

use crate::codes::{
    AssemblyError, Finding, Severity, LIMIT_EXCEEDED, MODEL_MISSING, STEP_REPLACED,
    THICKNESS_DEFAULT,
};
use crate::geom::{rect_corners, rotate, P};
use crate::models::{ModelEntry, ModelSource, ModelTable};
use crate::settings::AssemblySettings;
use crate::step::{SolidKind, StepSolid};

/// Where the box of a component comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    /// A box written in the table of models.
    Table,
    /// A STEP file that is a plain box.
    StepBox,
    /// A STEP file replaced by its enclosing box.
    StepEnclosing(SolidKind),
}

/// A component of the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part3d {
    /// The reference.
    pub reference: String,
    /// The name of the footprint.
    pub footprint: String,
    /// Whether it is on the bottom side.
    pub bottom: bool,
    /// The four corners of the box on the board plane, in nanometres.
    pub corners: [P; 4],
    /// The lowest height of the box, in nanometres.
    pub z_min: i64,
    /// The highest height of the box, in nanometres.
    pub z_max: i64,
    /// Where the box comes from.
    pub source: PartSource,
}

impl Part3d {
    /// The box that encloses the corners on the board plane: least corner and greatest corner.
    pub fn plane_bounds(&self) -> (P, P) {
        let xs = self.corners.iter().map(|c| c.0);
        let ys = self.corners.iter().map(|c| c.1);
        (
            (xs.clone().min().unwrap_or(0), ys.clone().min().unwrap_or(0)),
            (xs.max().unwrap_or(0), ys.max().unwrap_or(0)),
        )
    }
}

/// The assembled board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene3d {
    /// The name of the board.
    pub name: String,
    /// The outline of the board on the plane, in nanometres.
    pub outline: Vec<P>,
    /// The thickness of the board, in nanometres.
    pub thickness_nm: i64,
    /// The components, by reference.
    pub parts: Vec<Part3d>,
    /// What the building of the scene found.
    pub findings: Vec<Finding>,
}

/// The millimetres of a length in nanometres, with six decimals, exactly.
pub fn millimetres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

/// The box of a model, least and greatest corner in nanometres, before it is placed.
fn model_box(
    entry: &ModelEntry,
    cache: &mut BTreeMap<String, Option<StepSolid>>,
    findings: &mut Vec<Finding>,
    resolve: &mut dyn FnMut(&str) -> Result<StepSolid, AssemblyError>,
) -> Option<([i64; 3], [i64; 3], PartSource)> {
    match &entry.source {
        ModelSource::Box {
            length_nm,
            width_nm,
            height_nm,
        } => {
            let min = [-(length_nm / 2), -(width_nm / 2), 0];
            let max = [
                length_nm - length_nm / 2,
                width_nm - width_nm / 2,
                *height_nm,
            ];
            Some((min, max, PartSource::Table))
        }
        ModelSource::File(name) => {
            if !cache.contains_key(name) {
                let solid = match resolve(name) {
                    Ok(solid) => {
                        if solid.kind != SolidKind::Box {
                            let mut detail = format!("{:?}", solid.kind);
                            if !solid.unknown.is_empty() {
                                detail.push_str(&format!(" ({})", solid.unknown.join(", ")));
                            }
                            findings.push(Finding::new(
                                STEP_REPLACED,
                                Severity::Warning,
                                Some(name),
                                detail,
                            ));
                        }
                        Some(solid)
                    }
                    Err(error) => {
                        findings.push(Finding::new(
                            error.code,
                            Severity::Warning,
                            Some(name),
                            error.detail,
                        ));
                        None
                    }
                };
                cache.insert(name.clone(), solid);
            }
            let solid = cache.get(name)?.as_ref()?;
            let source = if solid.kind == SolidKind::Box {
                PartSource::StepBox
            } else {
                PartSource::StepEnclosing(solid.kind)
            };
            Some((solid.min, solid.max, source))
        }
    }
}

/// Builds the scene. `thickness_nm` is the thickness of the stack-up when there is one;
/// `resolve` reads a STEP file by the name written in the table.
pub fn build_scene(
    board: &Board,
    thickness_nm: Option<i64>,
    table: &ModelTable,
    settings: &AssemblySettings,
    resolve: &mut dyn FnMut(&str) -> Result<StepSolid, AssemblyError>,
) -> Result<Scene3d, AssemblyError> {
    if board.components.len() > settings.max_components {
        return Err(AssemblyError::new(
            LIMIT_EXCEEDED,
            format!(
                "{} components, the limit is {}",
                board.components.len(),
                settings.max_components
            ),
        ));
    }
    let mut findings = Vec::new();
    let thickness = match thickness_nm.filter(|t| *t > 0) {
        Some(t) => t,
        None => {
            findings.push(Finding::new(
                THICKNESS_DEFAULT,
                Severity::Info,
                None,
                format!("{} mm", millimetres(settings.default_board_thickness_nm)),
            ));
            settings.default_board_thickness_nm
        }
    };
    let mut cache: BTreeMap<String, Option<StepSolid>> = BTreeMap::new();
    let mut without_model: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut parts = Vec::new();
    for component in board.components.values() {
        let footprint = board
            .footprints
            .get(&component.footprint)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let Some(entry) = table.entries.get(&footprint) else {
            without_model
                .entry(footprint)
                .or_default()
                .push(component.refdes.clone());
            continue;
        };
        let Some((min, max, source)) = model_box(entry, &mut cache, &mut findings, &mut *resolve)
        else {
            continue;
        };
        // the box of the model turned by the table and moved by it, on the footprint
        let turn = i64::from(entry.quarter_turns) * 90_000_000;
        let c0 = rotate((min[0], min[1]), turn);
        let c1 = rotate((max[0], max[1]), turn);
        let mut lo = (c0.0.min(c1.0) + entry.dx_nm, c0.1.min(c1.1) + entry.dy_nm);
        let mut hi = (c0.0.max(c1.0) + entry.dx_nm, c0.1.max(c1.1) + entry.dy_nm);
        let bottom = component.side == Side::Bottom;
        if bottom {
            let (a, b) = (-hi.0, -lo.0);
            lo.0 = a;
            hi.0 = b;
        }
        let at = (component.at.x_nm.get(), component.at.y_nm.get());
        let angle = component.rotation.get();
        let mut corners = rect_corners(lo, hi);
        for corner in &mut corners {
            let turned = rotate(*corner, angle);
            *corner = (turned.0 + at.0, turned.1 + at.1);
        }
        let (z_low, z_high) = (min[2] + entry.dz_nm, max[2] + entry.dz_nm);
        let (z_min, z_max) = if bottom {
            (-z_high, -z_low)
        } else {
            (thickness + z_low, thickness + z_high)
        };
        parts.push(Part3d {
            reference: component.refdes.clone(),
            footprint,
            bottom,
            corners,
            z_min,
            z_max,
            source,
        });
    }
    for (footprint, mut references) in without_model {
        references.sort();
        let shown: Vec<String> = references.iter().take(10).cloned().collect();
        let more = references.len() - shown.len();
        let mut detail = shown.join(", ");
        if more > 0 {
            detail.push_str(&format!(" and {more} more"));
        }
        let subject = if footprint.is_empty() {
            "(no footprint)".to_owned()
        } else {
            footprint
        };
        findings.push(Finding::new(
            MODEL_MISSING,
            Severity::Warning,
            Some(&subject),
            detail,
        ));
    }
    parts.sort_by_key(|p| (p.reference.clone(), p.footprint.clone()));
    let outline = board
        .outline
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect();
    Ok(Scene3d {
        name: board.name.clone(),
        outline,
        thickness_nm: thickness,
        parts,
        findings,
    })
}
