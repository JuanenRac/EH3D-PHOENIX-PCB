// SPDX-License-Identifier: GPL-3.0-or-later
//! The cross section of a line of a stack-up as a [`Shape`]: the layers between the track and the
//! reference planes, each with its own permittivity and loss tangent at the frequency (the
//! closed formulas combine them into one), the track with its thickness, the second track of a
//! pair, the grounded copper at the sides of a coplanar line, and the walls of the box.
//!
//! The open side of a microstrip or a coplanar line is air up to a wall that is
//! `box_margin_factor` times the height and the width of the conductors away, and the side walls
//! are as far; the walls are at zero potential, which is an approximation of the open region.

use eh3d_phoenix_stackup::line::MAX_WIDTH_NM;
use eh3d_phoenix_stackup::{LineKind, LineSpec, MaterialSample, Stackup, StackupError};
use serde::Serialize;

use crate::codes::{FieldError, APPROXIMATE, LINE_UNAVAILABLE, MASK_LEFT_OUT};
use crate::grid::{Rect, Walls};
use crate::settings::Field2dSettings;
use crate::shape::{Block, Electrode, Shape};

/// Something to know about a result: a code and a detail in English.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Note {
    /// The code, `PHOENIX-FIELD-<NNN>`.
    pub code: &'static str,
    /// The detail.
    pub detail: String,
}

impl Note {
    /// A note with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> Note {
        Note {
            code,
            detail: detail.into(),
        }
    }
}

/// A line of a stack-up ready to be meshed.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The name of the line.
    pub name: String,
    /// The kind of line.
    pub kind: LineKind,
    /// The rectangles of the cross section.
    pub shape: Shape,
    /// The width of the track, in metres.
    pub width_m: f64,
    /// The gap of the line, in metres, for the kinds that have one.
    pub gap_m: Option<f64>,
    /// The frequency at which the materials were taken, in GHz.
    pub freq_ghz: f64,
    /// What to know about the model.
    pub notes: Vec<Note>,
}

struct Slab {
    h_m: f64,
    eps_r: f64,
    tan_d: f64,
}

struct Run {
    /// The dielectric layers, the one nearest the track first.
    slabs: Vec<Slab>,
    /// Whether a copper layer ends the run.
    reference: bool,
    /// Whether solder mask lies on this side.
    mask: bool,
}

fn unavailable(error: StackupError) -> FieldError {
    FieldError::new(LINE_UNAVAILABLE, error.to_string())
}

fn refused(what: impl Into<String>) -> FieldError {
    FieldError::new(LINE_UNAVAILABLE, what)
}

fn walk(
    stackup: &Stackup,
    materials: &[MaterialSample],
    from: usize,
    up: bool,
    freq_ghz: f64,
) -> Result<Run, FieldError> {
    let mut run = Run {
        slabs: Vec::new(),
        reference: false,
        mask: false,
    };
    let mut index = from;
    loop {
        index = if up {
            match index.checked_sub(1) {
                Some(next) => next,
                None => break,
            }
        } else {
            index + 1
        };
        let Some(layer) = stackup.layers.get(index) else {
            break;
        };
        if layer.is_copper() {
            run.reference = true;
            break;
        }
        if layer.is_mask() {
            run.mask = true;
            continue;
        }
        let name = layer
            .material()
            .ok_or_else(|| refused(format!("the layer {} has no material", layer.name())))?;
        let material = materials
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| refused(format!("unknown material {name}")))?;
        run.slabs.push(Slab {
            h_m: layer.thickness_nm().map_err(unavailable)? as f64 * 1.0e-9,
            eps_r: material.dk_at(freq_ghz).map_err(unavailable)?,
            tan_d: material.df_at(freq_ghz).map_err(unavailable)?,
        });
    }
    Ok(run)
}

fn total(slabs: &[Slab]) -> f64 {
    let mut sum = 0.0;
    for slab in slabs {
        sum += slab.h_m;
    }
    sum
}

/// Add the layers of a run to the blocks, from `y` upwards, in the order given, and return the
/// `y` above the last.
fn stack<'a, I: Iterator<Item = &'a Slab>>(
    slabs: I,
    mut y: f64,
    half: f64,
    blocks: &mut Vec<Block>,
) -> f64 {
    for slab in slabs {
        blocks.push(Block {
            rect: Rect::new(-half, half, y, y + slab.h_m),
            eps_r: slab.eps_r,
            tan_d: slab.tan_d,
        });
        y += slab.h_m;
    }
    y
}

/// The cross section of a line of a stack-up with a width, at a frequency in GHz.
pub fn section_from_stackup(
    stackup: &Stackup,
    materials: &[MaterialSample],
    spec: &LineSpec,
    width_nm: i64,
    freq_ghz: f64,
    settings: &Field2dSettings,
) -> Result<Section, FieldError> {
    if !freq_ghz.is_finite() || freq_ghz <= 0.0 {
        return Err(refused("the frequency must be a number above zero"));
    }
    if !(1..=MAX_WIDTH_NM).contains(&width_nm) {
        return Err(refused(format!("the width {width_nm} nm is out of range")));
    }
    stackup.validate_with(materials).map_err(unavailable)?;
    let sized = LineSpec {
        width_nm: Some(width_nm),
        target_ohm: None,
        ..spec.clone()
    };
    sized.validate().map_err(unavailable)?;
    let index = stackup
        .find(&spec.layer)
        .ok_or_else(|| refused(format!("there is no layer named {}", spec.layer)))?;
    let layer = stackup
        .layers
        .get(index)
        .ok_or_else(|| refused("the layer is missing"))?;
    if !layer.is_copper() {
        return Err(refused(format!("the layer {} is not copper", spec.layer)));
    }
    let t_m = layer.thickness_nm().map_err(unavailable)? as f64 * 1.0e-9;
    let w_m = width_nm as f64 * 1.0e-9;
    let gap_m = spec.gap_nm.map(|g| g as f64 * 1.0e-9);
    let up = walk(stackup, materials, index, true, freq_ghz)?;
    let down = walk(stackup, materials, index, false, freq_ghz)?;
    let factor = settings.box_margin_factor;
    let mut notes: Vec<Note> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut grounds: Vec<Rect> = Vec::new();
    let mut electrodes: Vec<Electrode> = Vec::new();
    let pair = matches!(
        spec.kind,
        LineKind::DifferentialMicrostrip | LineKind::DifferentialStripline
    );
    let gap = gap_m.unwrap_or(0.0);
    let span = match spec.kind {
        LineKind::Coplanar | LineKind::CoplanarGrounded => w_m + 2.0 * gap,
        _ if pair => 2.0 * w_m + gap,
        _ => w_m,
    };
    let domain: Rect;
    let y_track: f64;
    match spec.kind {
        LineKind::Stripline | LineKind::DifferentialStripline => {
            if up.slabs.is_empty() || down.slabs.is_empty() || !up.reference || !down.reference {
                return Err(refused(
                    "a stripline needs a dielectric and a copper plane on both sides of the track",
                ));
            }
            let height = total(&up.slabs) + t_m + total(&down.slabs);
            let half = span / 2.0 + factor * (height + span);
            let y_below = stack(down.slabs.iter().rev(), 0.0, half, &mut blocks);
            y_track = y_below;
            let y_top = stack(up.slabs.iter(), y_below + t_m, half, &mut blocks);
            domain = Rect::new(-half, half, 0.0, y_top);
        }
        _ => {
            let (substrate, open) = match (up.slabs.is_empty(), down.slabs.is_empty()) {
                (true, false) => (&down, &up),
                (false, true) => (&up, &down),
                _ => {
                    return Err(refused(
                        "the track needs dielectric on one side and air on the other",
                    ))
                }
            };
            let backed = !matches!(spec.kind, LineKind::Coplanar);
            if backed && !substrate.reference {
                return Err(refused("there is no copper plane under the substrate"));
            }
            let height = total(&substrate.slabs);
            let margin = factor * (height + span);
            let half = span / 2.0 + margin;
            let base = if backed { 0.0 } else { margin };
            if !backed {
                notes.push(Note::new(
                    APPROXIMATE,
                    format!(
                        "no copper plane is part of the model under the substrate: air down to a wall at zero potential {:.3} mm below it",
                        margin * 1.0e3
                    ),
                ));
            }
            if open.mask {
                notes.push(Note::new(
                    MASK_LEFT_OUT,
                    format!(
                        "the solder mask over {} changes the impedance and is left out of the model",
                        spec.name
                    ),
                ));
            }
            let y_substrate = stack(substrate.slabs.iter().rev(), base, half, &mut blocks);
            y_track = y_substrate;
            domain = Rect::new(-half, half, 0.0, y_substrate + t_m + margin);
        }
    }
    let band = (y_track, y_track + t_m);
    let half = domain.x1;
    match (spec.kind, gap_m) {
        (LineKind::DifferentialMicrostrip | LineKind::DifferentialStripline, Some(s)) => {
            electrodes.push(Electrode {
                rect: Rect::new(-s / 2.0 - w_m, -s / 2.0, band.0, band.1),
                id: 1,
            });
            electrodes.push(Electrode {
                rect: Rect::new(s / 2.0, s / 2.0 + w_m, band.0, band.1),
                id: 2,
            });
        }
        (LineKind::Coplanar | LineKind::CoplanarGrounded, Some(g)) => {
            electrodes.push(Electrode {
                rect: Rect::new(-w_m / 2.0, w_m / 2.0, band.0, band.1),
                id: 1,
            });
            grounds.push(Rect::new(-half, -w_m / 2.0 - g, band.0, band.1));
            grounds.push(Rect::new(w_m / 2.0 + g, half, band.0, band.1));
        }
        (_, None) if spec.kind.needs_gap() => return Err(refused("this kind of line needs a gap")),
        _ => electrodes.push(Electrode {
            rect: Rect::new(-w_m / 2.0, w_m / 2.0, band.0, band.1),
            id: 1,
        }),
    }
    let shape = Shape {
        domain,
        walls: Walls::GROUNDED,
        blocks,
        electrodes,
        grounds,
        conductors: if pair { 2 } else { 1 },
    };
    shape.validate()?;
    Ok(Section {
        name: spec.name.clone(),
        kind: spec.kind,
        shape,
        width_m: w_m,
        gap_m,
        freq_ghz,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_stackup::{CopperThickness, Layer};

    fn flat(name: &str, dk: f64, df: f64) -> MaterialSample {
        MaterialSample {
            name: name.to_string(),
            dk_points: vec![(0.1, dk), (50.0, dk)],
            df_points: vec![(0.1, df), (50.0, df)],
            thickness_nm: None,
        }
    }

    fn copper(name: &str, um: f64) -> Layer {
        Layer::Copper {
            name: name.to_string(),
            thickness: CopperThickness::Um(um),
            roughness_rms_nm: 0,
        }
    }

    fn core(name: &str, thickness_nm: i64, material: &str) -> Layer {
        Layer::Core {
            name: name.to_string(),
            thickness_nm,
            material: material.to_string(),
        }
    }

    fn two_layer() -> Stackup {
        Stackup {
            name: "two".to_string(),
            layers: vec![
                copper("top", 35.0),
                core("core", 200_000, "fr4"),
                copper("bottom", 35.0),
            ],
        }
    }

    fn four_layer() -> Stackup {
        Stackup {
            name: "four".to_string(),
            layers: vec![
                copper("top", 35.0),
                core("prepreg-1", 200_000, "fr4"),
                copper("inner-1", 17.5),
                core("core", 800_000, "fr4"),
                copper("inner-2", 17.5),
                core("prepreg-2", 300_000, "fr4"),
                copper("bottom", 35.0),
            ],
        }
    }

    fn spec(kind: LineKind, layer: &str, gap: Option<i64>) -> LineSpec {
        LineSpec {
            name: "line".to_string(),
            kind,
            layer: layer.to_string(),
            width_nm: Some(300_000),
            target_ohm: None,
            gap_nm: gap,
        }
    }

    fn settings() -> Field2dSettings {
        Field2dSettings::default()
    }

    #[test]
    fn a_microstrip_has_the_substrate_under_the_track_and_air_over_it() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        let spec = spec(LineKind::Microstrip, "top", None);
        let section =
            section_from_stackup(&two_layer(), &materials, &spec, 300_000, 1.0, &settings())
                .unwrap();
        let shape = &section.shape;
        assert_eq!(shape.blocks.len(), 1);
        assert!((shape.blocks[0].rect.y1 - 0.2e-3).abs() < 1.0e-12);
        assert_eq!(shape.blocks[0].eps_r, 4.3);
        assert_eq!(shape.electrodes.len(), 1);
        let e = shape.electrodes[0].rect;
        assert!((e.y0 - 0.2e-3).abs() < 1.0e-12);
        assert!((e.y1 - 0.235e-3).abs() < 1.0e-12);
        assert!((e.x1 - e.x0 - 0.3e-3).abs() < 1.0e-12);
        // the box is 8 times (height + width) away
        let margin = 8.0 * (0.2e-3 + 0.3e-3);
        assert!((shape.domain.y1 - (0.235e-3 + margin)).abs() < 1.0e-9);
        assert!(section.notes.is_empty());
        assert_eq!(shape.conductors, 1);
    }

    #[test]
    fn a_microstrip_on_the_bottom_layer_is_the_same_section_upside_down() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        let top = section_from_stackup(
            &two_layer(),
            &materials,
            &spec(LineKind::Microstrip, "top", None),
            300_000,
            1.0,
            &settings(),
        )
        .unwrap();
        let bottom = section_from_stackup(
            &two_layer(),
            &materials,
            &spec(LineKind::Microstrip, "bottom", None),
            300_000,
            1.0,
            &settings(),
        )
        .unwrap();
        assert_eq!(top.shape, bottom.shape);
    }

    #[test]
    fn a_stripline_has_a_dielectric_on_each_side_and_the_track_between() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        let spec = spec(LineKind::Stripline, "inner-1", None);
        let section =
            section_from_stackup(&four_layer(), &materials, &spec, 300_000, 1.0, &settings())
                .unwrap();
        let shape = &section.shape;
        assert_eq!(shape.blocks.len(), 2);
        // 0.2 mm above the track, 0.8 mm below it, the track 17.5 um thick
        let e = shape.electrodes[0].rect;
        assert!((e.y0 - 0.8e-3).abs() < 1.0e-12, "{}", e.y0);
        assert!((e.y1 - 0.8175e-3).abs() < 1.0e-12);
        assert!((shape.domain.y1 - 1.0175e-3).abs() < 1.0e-12);
        assert!((shape.domain.y0).abs() < 1.0e-15);
    }

    #[test]
    fn a_pair_has_two_electrodes_a_gap_apart() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        let spec = spec(LineKind::DifferentialMicrostrip, "top", Some(200_000));
        let section =
            section_from_stackup(&two_layer(), &materials, &spec, 300_000, 1.0, &settings())
                .unwrap();
        let shape = &section.shape;
        assert_eq!(shape.conductors, 2);
        let (a, b) = (shape.electrodes[0].rect, shape.electrodes[1].rect);
        assert!((b.x0 - a.x1 - 0.2e-3).abs() < 1.0e-12);
        assert!((a.x0 + b.x1).abs() < 1.0e-12);
        assert_eq!(shape.electrodes[0].id, 1);
        assert_eq!(shape.electrodes[1].id, 2);
    }

    #[test]
    fn a_coplanar_line_has_grounded_copper_on_both_sides() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        let spec = spec(LineKind::CoplanarGrounded, "top", Some(150_000));
        let section =
            section_from_stackup(&two_layer(), &materials, &spec, 300_000, 1.0, &settings())
                .unwrap();
        let shape = &section.shape;
        assert_eq!(shape.grounds.len(), 2);
        assert!((shape.grounds[0].x1 + 0.3e-3).abs() < 1.0e-12);
        assert!((shape.grounds[1].x0 - 0.3e-3).abs() < 1.0e-12);
        assert!(section.notes.is_empty());
        let unbacked = section_from_stackup(
            &two_layer(),
            &materials,
            &self::spec(LineKind::Coplanar, "top", Some(150_000)),
            300_000,
            1.0,
            &settings(),
        )
        .unwrap();
        assert_eq!(unbacked.notes.len(), 1);
        assert_eq!(unbacked.notes[0].code, APPROXIMATE);
    }

    #[test]
    fn a_line_that_cannot_be_taken_from_the_stackup_is_refused() {
        let materials = vec![flat("fr4", 4.3, 0.02)];
        // a stripline on an outer layer, a layer that does not exist, a material that is missing,
        // a gap that is not given, a width out of range
        let cases: Vec<(LineSpec, i64)> = vec![
            (spec(LineKind::Stripline, "top", None), 300_000),
            (spec(LineKind::Microstrip, "nowhere", None), 300_000),
            (spec(LineKind::Microstrip, "core", None), 300_000),
            (spec(LineKind::DifferentialMicrostrip, "top", None), 300_000),
            (spec(LineKind::Microstrip, "top", None), 0),
        ];
        for (line, width) in cases {
            let error =
                section_from_stackup(&two_layer(), &materials, &line, width, 1.0, &settings())
                    .unwrap_err();
            assert_eq!(error.code, LINE_UNAVAILABLE, "{line:?}");
        }
        let error = section_from_stackup(
            &two_layer(),
            &[],
            &spec(LineKind::Microstrip, "top", None),
            300_000,
            1.0,
            &settings(),
        )
        .unwrap_err();
        assert_eq!(error.code, LINE_UNAVAILABLE);
    }

    #[test]
    fn the_solder_mask_is_noted_and_left_out() {
        let materials = vec![flat("fr4", 4.3, 0.02), flat("mask", 3.5, 0.03)];
        let mut stackup = two_layer();
        stackup.layers.insert(
            0,
            Layer::SolderMask {
                name: "mask".to_string(),
                thickness_nm: 20_000,
                material: "mask".to_string(),
            },
        );
        let section = section_from_stackup(
            &stackup,
            &materials,
            &spec(LineKind::Microstrip, "top", None),
            300_000,
            1.0,
            &settings(),
        )
        .unwrap();
        assert_eq!(section.notes.len(), 1);
        assert_eq!(section.notes[0].code, MASK_LEFT_OUT);
    }
}
