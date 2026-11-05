// SPDX-License-Identifier: GPL-3.0-or-later
//! The transmission lines of a stack-up: the cross section is taken from the layers, the models of
//! [`crate::models`] give the impedance and the permittivity, and this module adds the losses, the
//! delay and the guided wavelength at a frequency, and the synthesis of a track width.
//!
//! Losses:
//!
//! * Conductor loss by Wheeler's incremental inductance rule (H. A. Wheeler, "Formulas for the
//!   skin effect", Proc. IRE, 1942): the surfaces of the copper are moved into the metal by half
//!   a skin depth, and the rise of the inductance of the line in air gives the resistance,
//!   `alpha_c = Rs / (eta0 Z0) * dZair/ds` in Np/m, with `Rs = sqrt(pi f mu0 / sigma)` the
//!   surface resistance, `Z0` the impedance of the line and `dZair/ds` the rate at which the
//!   impedance of the line in air grows when every conductor width and thickness is reduced and
//!   every gap enlarged by `s`. The derivative is taken numerically on the same model that gives
//!   `Z0`. The rule assumes copper thick compared with the skin depth.
//! * Roughness by the Hammerstad-Jensen factor `Kr = 1 + (2/pi) atan(1.4 (D/delta)^2)` with `D`
//!   the RMS roughness (E. Hammerstad and O. Jensen, 1980); it saturates at two. One factor is
//!   applied to the whole conductor loss, taking the mean roughness of the copper layers that
//!   carry the current.
//! * Dielectric loss `alpha_d = (k0 / 2) * sum(q_i er_i tan_i) / sqrt(eps_eff)` in Np/m, where
//!   `q_i` is the share of the field in the dielectric `i` (D. M. Pozar, "Microwave Engineering",
//!   for the microstrip case, which is the same expression for a homogeneous line).
//!
//! The layers between the track and the plane are combined as capacitors in series: the
//! permittivity of the run is `sum(h) / sum(h / er)` and its loss tangent is
//! `sum(h tan / er) / sum(h / er)`. The copper layer next to the dielectric on each side is the
//! reference plane; a plane is taken as continuous and without holes.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::error::{
    StackupError, Warning, WARN_APPROXIMATE, WARN_MASK, WARN_QUASI_STATIC, WARN_THIN,
};
use crate::material::{MaterialSample, MAX_NAME_CHARS};
use crate::models::{Section, C0, ETA0, MU0};
use crate::settings::StackupSettings;
use crate::stackup::{Layer, Stackup};

/// The widest track or the widest gap that is accepted, in nanometres (1 m).
pub const MAX_WIDTH_NM: i64 = 1_000_000_000;

/// The kinds of line this crate computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    /// A track on an outer layer with a plane under the dielectric.
    Microstrip,
    /// A track on an inner layer between two planes (centred or offset).
    Stripline,
    /// A coplanar waveguide on an outer layer, without a ground plane under the substrate.
    Coplanar,
    /// A coplanar waveguide on an outer layer with a ground plane under the substrate.
    CoplanarGrounded,
    /// An edge-coupled pair of microstrips.
    DifferentialMicrostrip,
    /// An edge-coupled pair of striplines.
    DifferentialStripline,
}

impl LineKind {
    /// The name the kind has in the files.
    pub fn as_str(self) -> &'static str {
        match self {
            LineKind::Microstrip => "microstrip",
            LineKind::Stripline => "stripline",
            LineKind::Coplanar => "coplanar",
            LineKind::CoplanarGrounded => "coplanar_grounded",
            LineKind::DifferentialMicrostrip => "differential_microstrip",
            LineKind::DifferentialStripline => "differential_stripline",
        }
    }

    /// Whether the kind needs a gap (the slot of a coplanar line, the spacing of a pair).
    pub fn needs_gap(self) -> bool {
        !matches!(self, LineKind::Microstrip | LineKind::Stripline)
    }
}

/// One line to compute: where it is, what it is, and either its width or the impedance wanted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineSpec {
    /// The name of the line in the report.
    pub name: String,
    /// The kind of line.
    pub kind: LineKind,
    /// The name of the copper layer the track is on.
    pub layer: String,
    /// The width of the track in nanometres. Exactly one of `width_nm` and `target_ohm`.
    #[serde(default)]
    pub width_nm: Option<i64>,
    /// The impedance wanted in ohm; the width is then found. For a pair it is the differential
    /// impedance.
    #[serde(default)]
    pub target_ohm: Option<f64>,
    /// The width of each slot of a coplanar line, or the spacing between the edges of a pair, in
    /// nanometres.
    #[serde(default)]
    pub gap_nm: Option<i64>,
}

impl LineSpec {
    /// Check the description of the line by itself.
    pub fn validate(&self) -> Result<(), StackupError> {
        let bad = |reason: String| StackupError::InvalidLine { reason };
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_NAME_CHARS {
            return Err(bad(format!(
                "the name must have 1 to {MAX_NAME_CHARS} characters"
            )));
        }
        match (self.width_nm, self.target_ohm) {
            (Some(_), Some(_)) => {
                return Err(bad(format!(
                    "{}: give either a width or a target impedance, not both",
                    self.name
                )))
            }
            (None, None) => {
                return Err(bad(format!(
                    "{}: give a width or a target impedance",
                    self.name
                )))
            }
            (Some(width), None) => {
                if !(1..=MAX_WIDTH_NM).contains(&width) {
                    return Err(bad(format!(
                        "{}: the width must be between 1 nm and {MAX_WIDTH_NM} nm",
                        self.name
                    )));
                }
            }
            (None, Some(target)) => {
                if !target.is_finite() || target <= 0.0 || target > 10_000.0 {
                    return Err(bad(format!(
                        "{}: the target impedance must be above zero and at most 10000 ohm",
                        self.name
                    )));
                }
            }
        }
        match (self.kind.needs_gap(), self.gap_nm) {
            (true, None) => Err(bad(format!(
                "{}: a {} needs a gap",
                self.name,
                self.kind.as_str()
            ))),
            (false, Some(_)) => Err(bad(format!(
                "{}: a {} has no gap",
                self.name,
                self.kind.as_str()
            ))),
            (true, Some(gap)) if !(1..=MAX_WIDTH_NM).contains(&gap) => Err(bad(format!(
                "{}: the gap must be between 1 nm and {MAX_WIDTH_NM} nm",
                self.name
            ))),
            _ => Ok(()),
        }
    }
}

/// The result of the calculation of one line at one frequency.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LineResult {
    /// The name of the line.
    pub name: String,
    /// The kind of line.
    pub kind: LineKind,
    /// The frequency in GHz.
    pub freq_ghz: f64,
    /// The width of the track in nanometres.
    pub width_nm: i64,
    /// The gap in nanometres, for the kinds that have one.
    pub gap_nm: Option<i64>,
    /// The impedance the kind is defined by: `Z0`, or `Zdiff` for a pair, in ohm.
    pub impedance_ohm: f64,
    /// The impedance of one track alone, in ohm.
    pub z_single_ohm: f64,
    /// The effective relative permittivity.
    pub eps_eff: f64,
    /// The propagation delay in ps/mm.
    pub delay_ps_per_mm: f64,
    /// The guided wavelength in mm.
    pub guided_wavelength_mm: f64,
    /// The skin depth of the copper in micrometres.
    pub skin_depth_um: f64,
    /// The roughness factor applied to the conductor loss (1 for smooth copper, 2 at most).
    pub roughness_factor: f64,
    /// The conductor loss in dB/cm.
    pub conductor_loss_db_per_cm: f64,
    /// The dielectric loss in dB/cm.
    pub dielectric_loss_db_per_cm: f64,
    /// The sum of both losses in dB/cm.
    pub total_loss_db_per_cm: f64,
    /// What to know about the result.
    pub warnings: Vec<Warning>,
}

/// The result of a synthesis.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Synthesis {
    /// The width found, in nanometres.
    pub width_nm: i64,
    /// The steps the search took.
    pub iterations: u32,
    /// The calculation of the line with that width.
    pub result: LineResult,
}

/// What lies on one side of a copper layer, up to the next copper layer.
struct Side {
    /// The indices of the cores and prepregs, in order of distance.
    dielectrics: Vec<usize>,
    /// The index of the copper layer that ends the run, if there is one.
    reference: Option<usize>,
    /// Whether solder mask lies on this side.
    mask: bool,
}

fn walk(layers: &[Layer], from: usize, up: bool) -> Side {
    let mut side = Side {
        dielectrics: Vec::new(),
        reference: None,
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
        let Some(layer) = layers.get(index) else {
            break;
        };
        if layer.is_copper() {
            side.reference = Some(index);
            break;
        } else if layer.is_mask() {
            side.mask = true;
        } else {
            side.dielectrics.push(index);
        }
    }
    side
}

/// A run of dielectric layers combined into one.
struct Dielectric {
    h_m: f64,
    er: f64,
    tan_d: f64,
}

fn combine(
    stackup: &Stackup,
    materials: &[MaterialSample],
    indices: &[usize],
    freq_ghz: f64,
) -> Result<Dielectric, StackupError> {
    let mut h_sum = 0.0;
    let mut h_over_e = 0.0;
    let mut h_tan_over_e = 0.0;
    for &index in indices {
        let layer = stackup
            .layers
            .get(index)
            .ok_or_else(|| StackupError::InvalidLine {
                reason: "a layer of the run is missing".to_string(),
            })?;
        let name = layer.material().ok_or_else(|| StackupError::InvalidLine {
            reason: format!("the layer {} has no material", layer.name()),
        })?;
        let material = materials.iter().find(|m| m.name == name).ok_or_else(|| {
            StackupError::UnknownMaterial {
                name: name.to_string(),
            }
        })?;
        let dk = material.dk_at(freq_ghz)?;
        let df = material.df_at(freq_ghz)?;
        let h = layer.thickness_nm()? as f64 * 1.0e-9;
        h_sum += h;
        h_over_e += h / dk;
        h_tan_over_e += h * df / dk;
    }
    if h_sum <= 0.0 || h_over_e <= 0.0 {
        return Err(StackupError::InvalidLine {
            reason: "there is no dielectric between the track and the plane".to_string(),
        });
    }
    Ok(Dielectric {
        h_m: h_sum,
        er: h_sum / h_over_e,
        tan_d: h_tan_over_e / h_over_e,
    })
}

/// What the cross section needs besides the section itself.
struct Context {
    roughness_rms_nm: Vec<i64>,
    mask_open: bool,
    notes: Vec<Warning>,
}

fn roughness_nm(layer: &Layer) -> i64 {
    match layer {
        Layer::Copper {
            roughness_rms_nm, ..
        } => *roughness_rms_nm,
        _ => 0,
    }
}

fn invalid(reason: &str) -> StackupError {
    StackupError::InvalidLine {
        reason: reason.to_string(),
    }
}

fn build_section(
    stackup: &Stackup,
    materials: &[MaterialSample],
    spec: &LineSpec,
    width_nm: i64,
    freq_ghz: f64,
) -> Result<(Section, Context), StackupError> {
    let index = stackup
        .find(&spec.layer)
        .ok_or_else(|| StackupError::InvalidLine {
            reason: format!("there is no layer named {}", spec.layer),
        })?;
    let layer = stackup
        .layers
        .get(index)
        .ok_or_else(|| invalid("the layer is missing"))?;
    if !layer.is_copper() {
        return Err(StackupError::InvalidLine {
            reason: format!("the layer {} is not copper", spec.layer),
        });
    }
    let t_m = layer.thickness_nm()? as f64 * 1.0e-9;
    let w_m = width_nm as f64 * 1.0e-9;
    let gap_m = spec.gap_nm.map(|g| g as f64 * 1.0e-9);
    let up = walk(&stackup.layers, index, true);
    let down = walk(&stackup.layers, index, false);
    let mut context = Context {
        roughness_rms_nm: vec![roughness_nm(layer)],
        mask_open: false,
        notes: Vec::new(),
    };
    let reference_roughness = |side: &Side| {
        side.reference
            .and_then(|r| stackup.layers.get(r))
            .map(roughness_nm)
    };
    match spec.kind {
        LineKind::Stripline | LineKind::DifferentialStripline => {
            if up.dielectrics.is_empty()
                || down.dielectrics.is_empty()
                || up.reference.is_none()
                || down.reference.is_none()
            {
                return Err(invalid(
                    "a stripline needs a dielectric and a copper plane on both sides of the track",
                ));
            }
            let first = combine(stackup, materials, &up.dielectrics, freq_ghz)?;
            let second = combine(stackup, materials, &down.dielectrics, freq_ghz)?;
            context.roughness_rms_nm.extend(reference_roughness(&up));
            context.roughness_rms_nm.extend(reference_roughness(&down));
            let section = match (spec.kind, gap_m) {
                (LineKind::DifferentialStripline, Some(s_m)) => Section::DiffStripline {
                    w_m,
                    t_m,
                    h1_m: first.h_m,
                    h2_m: second.h_m,
                    er1: first.er,
                    er2: second.er,
                    tan_d1: first.tan_d,
                    tan_d2: second.tan_d,
                    s_m,
                },
                (LineKind::DifferentialStripline, None) => {
                    return Err(invalid("a pair needs a gap"))
                }
                _ => Section::Stripline {
                    w_m,
                    t_m,
                    h1_m: first.h_m,
                    h2_m: second.h_m,
                    er1: first.er,
                    er2: second.er,
                    tan_d1: first.tan_d,
                    tan_d2: second.tan_d,
                },
            };
            Ok((section, context))
        }
        _ => {
            let (substrate, open) = match (up.dielectrics.is_empty(), down.dielectrics.is_empty()) {
                (true, false) => (&down, &up),
                (false, true) => (&up, &down),
                _ => {
                    return Err(invalid(
                        "the track needs dielectric on one side and air on the other",
                    ))
                }
            };
            context.mask_open = open.mask;
            let d = combine(stackup, materials, &substrate.dielectrics, freq_ghz)?;
            let needs_plane = !matches!(spec.kind, LineKind::Coplanar);
            match (substrate.reference, needs_plane) {
                (None, true) => {
                    return Err(invalid("there is no copper plane under the substrate"));
                }
                (Some(_), true) => {
                    context
                        .roughness_rms_nm
                        .extend(reference_roughness(substrate));
                }
                (Some(r), false) => {
                    let name = stackup.layers.get(r).map(Layer::name).unwrap_or("");
                    context.notes.push(Warning::new(
                        WARN_APPROXIMATE,
                        format!(
                            "the copper layer {name} under the substrate is not part of this model"
                        ),
                    ));
                }
                (None, false) => {}
            }
            let section = match (spec.kind, gap_m) {
                (LineKind::Microstrip, _) => Section::Microstrip {
                    w_m,
                    t_m,
                    h_m: d.h_m,
                    er: d.er,
                    tan_d: d.tan_d,
                },
                (LineKind::DifferentialMicrostrip, Some(s_m)) => Section::DiffMicrostrip {
                    w_m,
                    t_m,
                    h_m: d.h_m,
                    er: d.er,
                    tan_d: d.tan_d,
                    s_m,
                },
                (LineKind::Coplanar | LineKind::CoplanarGrounded, Some(g_m)) => Section::Coplanar {
                    w_m,
                    g_m,
                    t_m,
                    h_m: d.h_m,
                    er: d.er,
                    tan_d: d.tan_d,
                    backed: matches!(spec.kind, LineKind::CoplanarGrounded),
                },
                _ => return Err(invalid("this kind of line needs a gap")),
            };
            Ok((section, context))
        }
    }
}

fn db_per_cm(np_per_m: f64) -> f64 {
    np_per_m * (20.0 / std::f64::consts::LN_10) / 100.0
}

/// Compute a line of a stack-up with a given width at a frequency in GHz.
///
/// The stack-up and the materials are checked first. The result carries warnings for everything
/// that left the range of a formula or that is only an approximation.
pub fn analyze(
    stackup: &Stackup,
    materials: &[MaterialSample],
    spec: &LineSpec,
    width_nm: i64,
    freq_ghz: f64,
    settings: &StackupSettings,
) -> Result<LineResult, StackupError> {
    if !freq_ghz.is_finite() || freq_ghz <= 0.0 || freq_ghz > settings.max_frequency_ghz {
        return Err(StackupError::InvalidNumber {
            field: "frequency".to_string(),
            reason: format!(
                "{freq_ghz} GHz is not between zero and {} GHz",
                settings.max_frequency_ghz
            ),
        });
    }
    if !(1..=MAX_WIDTH_NM).contains(&width_nm) {
        return Err(StackupError::InvalidNumber {
            field: "width".to_string(),
            reason: format!("{width_nm} nm is not between 1 nm and {MAX_WIDTH_NM} nm"),
        });
    }
    stackup.validate_with(materials)?;
    let shape = LineSpec {
        width_nm: Some(width_nm),
        target_ohm: None,
        ..spec.clone()
    };
    shape.validate()?;
    let (section, context) = build_section(stackup, materials, &shape, width_nm, freq_ghz)?;
    let solution = section.eval()?;
    let mut warnings = section.range_warnings();
    warnings.extend(context.notes);

    let f_hz = freq_ghz * 1.0e9;
    let sigma = settings.copper_conductivity_s_per_m;
    let skin = 1.0 / (PI * f_hz * MU0 * sigma).sqrt();
    let rs = 1.0 / (sigma * skin);
    let mean_roughness_m = if context.roughness_rms_nm.is_empty() {
        0.0
    } else {
        context.roughness_rms_nm.iter().sum::<i64>() as f64 / context.roughness_rms_nm.len() as f64
            * 1.0e-9
    };
    let roughness_factor = 1.0 + (2.0 / PI) * (1.4 * (mean_roughness_m / skin).powi(2)).atan();

    let base = section.base();
    let air = base.air();
    let step = 1.0e-4 * base.smallest_m();
    let z_air = air.eval()?.impedance_ohm;
    let z_receded = air.receded(step).eval()?.impedance_ohm;
    let alpha_c =
        roughness_factor * rs * (z_receded - z_air) / step / (ETA0 * solution.z_single_ohm);
    let k0 = 2.0 * PI * f_hz / C0;
    let alpha_d = 0.5 * k0 * solution.eps_tan / solution.eps_eff.sqrt();
    let conductor = db_per_cm(alpha_c.max(0.0));
    let dielectric = db_per_cm(alpha_d.max(0.0));

    let sqrt_eps = solution.eps_eff.sqrt();
    let guided_m = C0 / (f_hz * sqrt_eps);
    if section.thickness_m() < settings.thin_conductor_skin_depths * skin {
        warnings.push(Warning::new(
            WARN_THIN,
            format!(
                "the copper is {:.2} um thick and the skin depth is {:.2} um at {} GHz",
                section.thickness_m() * 1.0e6,
                skin * 1.0e6,
                freq_ghz
            ),
        ));
    }
    if section.vertical_m() > settings.quasi_static_height_fraction * guided_m {
        warnings.push(Warning::new(
            WARN_QUASI_STATIC,
            format!(
                "the height {:.3} mm is more than {} of the guided wavelength {:.3} mm at {} GHz",
                section.vertical_m() * 1.0e3,
                settings.quasi_static_height_fraction,
                guided_m * 1.0e3,
                freq_ghz
            ),
        ));
    }
    if context.mask_open {
        warnings.push(Warning::new(
            WARN_MASK,
            format!(
                "the solder mask over {} changes the impedance and is left out of the model",
                shape.name
            ),
        ));
    }

    Ok(LineResult {
        name: shape.name,
        kind: shape.kind,
        freq_ghz,
        width_nm,
        gap_nm: shape.gap_nm,
        impedance_ohm: solution.impedance_ohm,
        z_single_ohm: solution.z_single_ohm,
        eps_eff: solution.eps_eff,
        delay_ps_per_mm: sqrt_eps / C0 * 1.0e9,
        guided_wavelength_mm: guided_m * 1.0e3,
        skin_depth_um: skin * 1.0e6,
        roughness_factor,
        conductor_loss_db_per_cm: conductor,
        dielectric_loss_db_per_cm: dielectric,
        total_loss_db_per_cm: conductor + dielectric,
        warnings,
    })
}

/// Find the width of the track that gives an impedance, by bisection on the direct calculation.
///
/// The impedance falls as the track widens, so the width is searched in whole nanometres between
/// `synth_min_width_nm` and `synth_max_width_nm` of the settings until the impedance is within
/// `synth_tolerance_ohm` of the target. A target that the two limits do not enclose is
/// `TargetUnreachable`; a search that runs out of steps or of nanometres before reaching the
/// tolerance is `NotConverged`. For a pair the target is the differential impedance at the gap
/// of the line.
pub fn synthesize_width(
    stackup: &Stackup,
    materials: &[MaterialSample],
    spec: &LineSpec,
    target_ohm: f64,
    freq_ghz: f64,
    settings: &StackupSettings,
) -> Result<Synthesis, StackupError> {
    if !target_ohm.is_finite() || target_ohm <= 0.0 {
        return Err(StackupError::InvalidNumber {
            field: "target impedance".to_string(),
            reason: "must be a finite number above zero".to_string(),
        });
    }
    let impedance_at = |width_nm: i64| -> Result<f64, StackupError> {
        analyze(stackup, materials, spec, width_nm, freq_ghz, settings).map(|r| r.impedance_ohm)
    };
    let tolerance = settings.synth_tolerance_ohm;
    let mut low = settings.synth_min_width_nm;
    let mut high = settings.synth_max_width_nm;
    let z_low = impedance_at(low)?;
    let z_high = impedance_at(high)?;
    if !(z_high..=z_low).contains(&target_ohm) {
        return Err(StackupError::TargetUnreachable {
            target_ohm,
            low_ohm: z_high,
            high_ohm: z_low,
        });
    }
    let mut best = (low, (z_low - target_ohm).abs());
    if (z_high - target_ohm).abs() < best.1 {
        best = (high, (z_high - target_ohm).abs());
    }
    let mut iterations: u32 = 0;
    while best.1 > tolerance && high - low > 1 {
        if iterations >= settings.synth_max_iterations {
            return Err(StackupError::NotConverged {
                iterations,
                error_ohm: best.1,
            });
        }
        iterations += 1;
        let middle = low + (high - low) / 2;
        let z = impedance_at(middle)?;
        let error = (z - target_ohm).abs();
        if error < best.1 {
            best = (middle, error);
        }
        if z > target_ohm {
            low = middle;
        } else {
            high = middle;
        }
    }
    if best.1 > tolerance {
        return Err(StackupError::NotConverged {
            iterations,
            error_ohm: best.1,
        });
    }
    let result = analyze(stackup, materials, spec, best.0, freq_ghz, settings)?;
    Ok(Synthesis {
        width_nm: best.0,
        iterations,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stackup::{copper, core_layer, CopperThickness};

    /// Invented values for the tests; they are not the data of any sheet.
    fn flat(name: &str, dk: f64, df: f64) -> MaterialSample {
        MaterialSample {
            name: name.to_string(),
            dk_points: vec![(0.001, dk), (200.0, dk)],
            df_points: vec![(0.001, df), (200.0, df)],
            thickness_nm: None,
        }
    }

    fn materials() -> Vec<MaterialSample> {
        vec![flat("fr4-test", 4.5, 0.02), flat("mask-test", 3.3, 0.03)]
    }

    fn two_layer() -> Stackup {
        Stackup {
            name: "two-layer".to_string(),
            layers: vec![
                copper("top", 35.0),
                core_layer("core", 1_600_000),
                copper("bottom", 35.0),
            ],
        }
    }

    fn four_layer() -> Stackup {
        Stackup {
            name: "four-layer".to_string(),
            layers: vec![
                copper("top", 35.0),
                core_layer("prepreg-1", 200_000),
                copper("inner-1", 17.5),
                core_layer("core", 800_000),
                copper("inner-2", 17.5),
                core_layer("prepreg-2", 200_000),
                copper("bottom", 35.0),
            ],
        }
    }

    fn spec(kind: LineKind, layer: &str, gap: Option<i64>) -> LineSpec {
        LineSpec {
            name: "line".to_string(),
            kind,
            layer: layer.to_string(),
            width_nm: Some(1_000_000),
            target_ohm: None,
            gap_nm: gap,
        }
    }

    fn run(stackup: &Stackup, spec: &LineSpec, width_nm: i64, freq_ghz: f64) -> LineResult {
        analyze(
            stackup,
            &materials(),
            spec,
            width_nm,
            freq_ghz,
            &StackupSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn a_microstrip_of_the_board_is_the_model_of_the_section() {
        let s = spec(LineKind::Microstrip, "top", None);
        let r = run(&two_layer(), &s, 3_000_000, 1.0);
        let (z, eps) = crate::models::microstrip(3.0e-3, 35.0e-6, 1.6e-3, 4.5).unwrap();
        assert!((r.impedance_ohm - z).abs() < z * 1e-9);
        assert!((r.eps_eff - eps).abs() < eps * 1e-9);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.width_nm, 3_000_000);
    }

    #[test]
    fn a_microstrip_works_from_the_bottom_layer_too() {
        let s = spec(LineKind::Microstrip, "bottom", None);
        let r = run(&two_layer(), &s, 3_000_000, 1.0);
        let top = run(
            &two_layer(),
            &spec(LineKind::Microstrip, "top", None),
            3_000_000,
            1.0,
        );
        assert_eq!(r.impedance_ohm, top.impedance_ohm);
    }

    #[test]
    fn impedance_falls_as_the_track_widens() {
        let s = spec(LineKind::Microstrip, "top", None);
        let mut last = f64::INFINITY;
        for width_um in [100, 200, 400, 800, 1600, 3200, 6400] {
            let r = run(&two_layer(), &s, width_um * 1000, 1.0);
            assert!(r.impedance_ohm < last, "{width_um}");
            last = r.impedance_ohm;
        }
    }

    #[test]
    fn delay_and_wavelength_follow_the_effective_permittivity() {
        let s = spec(LineKind::Microstrip, "top", None);
        let r = run(&two_layer(), &s, 3_000_000, 2.0);
        let expected_delay = r.eps_eff.sqrt() * 1.0e9 / C0;
        assert!((r.delay_ps_per_mm - expected_delay).abs() < 1e-9);
        // wavelength times frequency times sqrt(eps) is the speed of light
        let c = r.guided_wavelength_mm * 1.0e-3 * 2.0e9 * r.eps_eff.sqrt();
        assert!((c - C0).abs() < 1.0);
        // about 5.7 ps/mm for an effective permittivity of 3.4
        assert!(r.delay_ps_per_mm > 5.5 && r.delay_ps_per_mm < 6.5);
    }

    #[test]
    fn the_skin_depth_of_copper_at_one_gigahertz() {
        let s = spec(LineKind::Microstrip, "top", None);
        let r = run(&two_layer(), &s, 3_000_000, 1.0);
        // 2.09 um for 5.8e7 S/m
        assert!((r.skin_depth_um - 2.09).abs() < 0.01, "{}", r.skin_depth_um);
    }

    #[test]
    fn conductor_loss_goes_with_the_square_root_of_the_frequency() {
        let s = spec(LineKind::Microstrip, "top", None);
        let a = run(&two_layer(), &s, 3_000_000, 1.0);
        let c = run(&two_layer(), &s, 3_000_000, 4.0);
        let ratio = c.conductor_loss_db_per_cm / a.conductor_loss_db_per_cm;
        assert!((ratio - 2.0).abs() < 1e-6, "{ratio}");
    }

    #[test]
    fn dielectric_loss_goes_with_the_frequency() {
        let s = spec(LineKind::Microstrip, "top", None);
        let a = run(&two_layer(), &s, 3_000_000, 1.0);
        let c = run(&two_layer(), &s, 3_000_000, 4.0);
        let ratio = c.dielectric_loss_db_per_cm / a.dielectric_loss_db_per_cm;
        assert!((ratio - 4.0).abs() < 1e-6, "{ratio}");
        // a lossless material has no dielectric loss
        let lossless = vec![flat("fr4-test", 4.5, 0.0), flat("mask-test", 3.3, 0.0)];
        let r = analyze(
            &two_layer(),
            &lossless,
            &s,
            3_000_000,
            1.0,
            &StackupSettings::default(),
        )
        .unwrap();
        assert_eq!(r.dielectric_loss_db_per_cm, 0.0);
        assert!(r.conductor_loss_db_per_cm > 0.0);
    }

    #[test]
    fn the_dielectric_loss_matches_the_homogeneous_formula_for_a_stripline() {
        // in a homogeneous dielectric alpha_d = pi f sqrt(er) tan / c
        let s = spec(LineKind::Stripline, "inner-1", None);
        let stack = Stackup {
            name: "strip".to_string(),
            layers: vec![
                copper("p1", 35.0),
                core_layer("a", 300_000),
                copper("sig", 35.0),
                core_layer("b", 300_000),
                copper("p2", 35.0),
            ],
        };
        let s = LineSpec {
            layer: "sig".to_string(),
            ..s
        };
        let r = run(&stack, &s, 150_000, 2.0);
        let alpha = PI * 2.0e9 * 4.5_f64.sqrt() * 0.02 / C0;
        let expected = alpha * 20.0 / std::f64::consts::LN_10 / 100.0;
        assert!(
            (r.dielectric_loss_db_per_cm - expected).abs() < expected * 1e-9,
            "{} {}",
            r.dielectric_loss_db_per_cm,
            expected
        );
    }

    #[test]
    fn roughness_raises_the_conductor_loss_by_at_most_two() {
        let smooth = two_layer();
        let mut rough = two_layer();
        for layer in &mut rough.layers {
            if let Layer::Copper {
                roughness_rms_nm, ..
            } = layer
            {
                *roughness_rms_nm = 2_000;
            }
        }
        let s = spec(LineKind::Microstrip, "top", None);
        let a = run(&smooth, &s, 3_000_000, 10.0);
        let b = run(&rough, &s, 3_000_000, 10.0);
        assert_eq!(a.roughness_factor, 1.0);
        assert!(b.roughness_factor > 1.5 && b.roughness_factor <= 2.0);
        let ratio = b.conductor_loss_db_per_cm / a.conductor_loss_db_per_cm;
        assert!((ratio - b.roughness_factor).abs() < 1e-9);
        assert_eq!(a.dielectric_loss_db_per_cm, b.dielectric_loss_db_per_cm);
    }

    #[test]
    fn a_centred_stripline_matches_the_stripline_model() {
        let stack = Stackup {
            name: "strip".to_string(),
            layers: vec![
                copper("p1", 35.0),
                core_layer("a", 300_000),
                copper("sig", 35.0),
                core_layer("b", 300_000),
                copper("p2", 35.0),
            ],
        };
        let s = LineSpec {
            layer: "sig".to_string(),
            ..spec(LineKind::Stripline, "sig", None)
        };
        let r = run(&stack, &s, 150_000, 1.0);
        let b = 0.3e-3 + 0.3e-3 + 35.0e-6;
        let z = crate::models::stripline_symmetric(150.0e-6, 35.0e-6, b, 4.5).unwrap();
        assert!((r.impedance_ohm - z).abs() < z * 1e-9);
        assert!((r.eps_eff - 4.5).abs() < 1e-9);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn an_offset_stripline_warns_that_it_is_an_approximation() {
        let s = spec(LineKind::Stripline, "inner-1", None);
        let r = run(&four_layer(), &s, 100_000, 1.0);
        assert!(r.warnings.iter().any(|w| w.code == WARN_APPROXIMATE));
        // inner-1 has 0.2 mm of prepreg on one side and 0.8 mm of core on the other
        let centred = Stackup {
            name: "centred".to_string(),
            layers: vec![
                copper("top", 35.0),
                core_layer("a", 500_000),
                copper("inner-1", 17.5),
                core_layer("b", 500_000),
                copper("bottom", 35.0),
            ],
        };
        let c = run(&centred, &s, 100_000, 1.0);
        assert!(c.warnings.is_empty());
        assert!(r.impedance_ohm < c.impedance_ohm);
    }

    #[test]
    fn coplanar_lines_use_the_substrate_and_the_plane_as_asked() {
        let open = spec(LineKind::Coplanar, "top", Some(200_000));
        let grounded = spec(LineKind::CoplanarGrounded, "top", Some(200_000));
        let a = run(
            &two_layer(),
            &LineSpec {
                width_nm: Some(400_000),
                ..open
            },
            400_000,
            1.0,
        );
        let b = run(
            &two_layer(),
            &LineSpec {
                width_nm: Some(400_000),
                ..grounded
            },
            400_000,
            1.0,
        );
        assert!(b.impedance_ohm < a.impedance_ohm);
        assert!(b.eps_eff > a.eps_eff);
        assert!(a.warnings.iter().any(|w| w.code == WARN_APPROXIMATE));
        assert!(!b.warnings.iter().any(|w| w.code == WARN_APPROXIMATE));
        assert!(a.conductor_loss_db_per_cm > 0.0 && b.conductor_loss_db_per_cm > 0.0);
    }

    #[test]
    fn a_grounded_coplanar_line_needs_a_plane() {
        let one_sided = Stackup {
            name: "no-plane".to_string(),
            layers: vec![copper("top", 35.0), core_layer("core", 1_600_000)],
        };
        let grounded = spec(LineKind::CoplanarGrounded, "top", Some(200_000));
        let err = analyze(
            &one_sided,
            &materials(),
            &grounded,
            400_000,
            1.0,
            &StackupSettings::default(),
        )
        .unwrap_err();
        assert!(matches!(err, StackupError::InvalidLine { .. }));
        // the open one works without a plane
        let open = spec(LineKind::Coplanar, "top", Some(200_000));
        let ok = run(&one_sided, &open, 400_000, 1.0);
        assert!(ok.warnings.iter().all(|w| w.code != WARN_APPROXIMATE));
    }

    #[test]
    fn a_pair_is_below_twice_the_single_track_and_warns() {
        let single = run(
            &two_layer(),
            &spec(LineKind::Microstrip, "top", None),
            300_000,
            1.0,
        );
        let pair_spec = spec(LineKind::DifferentialMicrostrip, "top", Some(300_000));
        let pair = run(&two_layer(), &pair_spec, 300_000, 1.0);
        assert_eq!(pair.z_single_ohm, single.impedance_ohm);
        assert!(pair.impedance_ohm < 2.0 * single.impedance_ohm);
        assert_eq!(pair.eps_eff, single.eps_eff);
        assert!(pair.warnings.iter().any(|w| w.code == WARN_APPROXIMATE));
        let strip_pair = LineSpec {
            kind: LineKind::DifferentialStripline,
            layer: "inner-1".to_string(),
            gap_nm: Some(150_000),
            ..pair_spec
        };
        let r = run(&four_layer(), &strip_pair, 100_000, 1.0);
        assert!(r.impedance_ohm < 2.0 * r.z_single_ohm);
        assert_eq!(r.gap_nm, Some(150_000));
    }

    #[test]
    fn a_wider_gap_brings_the_pair_up_towards_twice_the_single_track() {
        let mut last = 0.0;
        for gap_um in [50, 100, 200, 400, 800, 1600] {
            let s = spec(LineKind::DifferentialMicrostrip, "top", Some(gap_um * 1000));
            let r = run(&two_layer(), &s, 300_000, 1.0);
            assert!(r.impedance_ohm > last, "{gap_um}");
            assert!(r.impedance_ohm < 2.0 * r.z_single_ohm);
            last = r.impedance_ohm;
        }
    }

    #[test]
    fn warnings_for_thin_copper_the_quasi_static_limit_and_the_mask() {
        let s = spec(LineKind::Microstrip, "top", None);
        // 1 MHz: skin depth 66 um, thicker than the 35 um of copper
        let low = run(&two_layer(), &s, 3_000_000, 0.001);
        assert!(low.warnings.iter().any(|w| w.code == WARN_THIN));
        // 100 GHz on 1.6 mm of laminate: the substrate is a large part of the wavelength
        let high = run(&two_layer(), &s, 3_000_000, 100.0);
        assert!(high.warnings.iter().any(|w| w.code == WARN_QUASI_STATIC));
        // solder mask over the track
        let mut masked = two_layer();
        masked.layers.insert(
            0,
            Layer::SolderMask {
                name: "mask".to_string(),
                thickness_nm: 20_000,
                material: "mask-test".to_string(),
            },
        );
        let m = run(&masked, &s, 3_000_000, 1.0);
        assert!(m.warnings.iter().any(|w| w.code == WARN_MASK));
        let plain = run(&two_layer(), &s, 3_000_000, 1.0);
        assert_eq!(m.impedance_ohm, plain.impedance_ohm);
    }

    #[test]
    fn a_track_outside_the_range_of_the_formula_is_computed_with_a_warning() {
        let s = spec(LineKind::Microstrip, "top", None);
        let r = run(&two_layer(), &s, 5_000, 1.0);
        assert!(r
            .warnings
            .iter()
            .any(|w| w.code == crate::error::WARN_RANGE));
        assert!(r.impedance_ohm > 100.0);
    }

    #[test]
    fn bad_lines_are_refused() {
        let settings = StackupSettings::default();
        let stack = two_layer();
        let mats = materials();
        let call = |spec: &LineSpec, width: i64, freq: f64| {
            analyze(&stack, &mats, spec, width, freq, &settings)
        };
        let micro = spec(LineKind::Microstrip, "top", None);
        // layers that do not exist or are not copper
        let ghost = LineSpec {
            layer: "ghost".to_string(),
            ..micro.clone()
        };
        let dielectric = LineSpec {
            layer: "core".to_string(),
            ..micro.clone()
        };
        assert!(matches!(
            call(&ghost, 1_000_000, 1.0),
            Err(StackupError::InvalidLine { .. })
        ));
        assert!(matches!(
            call(&dielectric, 1_000_000, 1.0),
            Err(StackupError::InvalidLine { .. })
        ));
        // a stripline on an outer layer
        let outer_strip = spec(LineKind::Stripline, "top", None);
        assert!(matches!(
            call(&outer_strip, 1_000_000, 1.0),
            Err(StackupError::InvalidLine { .. })
        ));
        // a microstrip on an inner layer
        let inner = LineSpec {
            layer: "inner-1".to_string(),
            ..micro.clone()
        };
        assert!(matches!(
            analyze(&four_layer(), &mats, &inner, 1_000_000, 1.0, &settings),
            Err(StackupError::InvalidLine { .. })
        ));
        // gaps
        let no_gap = spec(LineKind::Coplanar, "top", None);
        let extra_gap = spec(LineKind::Microstrip, "top", Some(100_000));
        let zero_gap = spec(LineKind::Coplanar, "top", Some(0));
        for bad in [&no_gap, &extra_gap, &zero_gap] {
            assert!(matches!(
                call(bad, 400_000, 1.0),
                Err(StackupError::InvalidLine { .. })
            ));
        }
        // numbers
        for width in [0, -5, MAX_WIDTH_NM + 1] {
            assert!(matches!(
                call(&micro, width, 1.0),
                Err(StackupError::InvalidNumber { .. })
            ));
        }
        for freq in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.0e6] {
            assert!(matches!(
                call(&micro, 1_000_000, freq),
                Err(StackupError::InvalidNumber { .. })
            ));
        }
        // a frequency outside the points of the material
        let narrow = vec![
            MaterialSample {
                name: "fr4-test".to_string(),
                dk_points: vec![(1.0, 4.5), (2.0, 4.4)],
                df_points: vec![(1.0, 0.02), (2.0, 0.02)],
                thickness_nm: None,
            },
            flat("mask-test", 3.3, 0.03),
        ];
        assert!(matches!(
            analyze(&stack, &narrow, &micro, 1_000_000, 5.0, &settings),
            Err(StackupError::OutsidePublished { .. })
        ));
        // a missing material
        assert!(matches!(
            analyze(&stack, &[], &micro, 1_000_000, 1.0, &settings),
            Err(StackupError::UnknownMaterial { .. })
        ));
    }

    #[test]
    fn the_description_of_a_line_is_checked() {
        let good = spec(LineKind::Microstrip, "top", None);
        assert!(good.validate().is_ok());
        let both = LineSpec {
            target_ohm: Some(50.0),
            ..good.clone()
        };
        let neither = LineSpec {
            width_nm: None,
            ..good.clone()
        };
        let zero_target = LineSpec {
            width_nm: None,
            target_ohm: Some(0.0),
            ..good.clone()
        };
        let huge_target = LineSpec {
            width_nm: None,
            target_ohm: Some(1.0e6),
            ..good.clone()
        };
        let nan_target = LineSpec {
            width_nm: None,
            target_ohm: Some(f64::NAN),
            ..good.clone()
        };
        let no_name = LineSpec {
            name: " ".to_string(),
            ..good.clone()
        };
        let wide = LineSpec {
            width_nm: Some(MAX_WIDTH_NM + 1),
            ..good.clone()
        };
        for bad in [
            both,
            neither,
            zero_target,
            huge_target,
            nan_target,
            no_name,
            wide,
        ] {
            assert!(
                matches!(bad.validate(), Err(StackupError::InvalidLine { .. })),
                "{bad:?}"
            );
        }
        let target = LineSpec {
            width_nm: None,
            target_ohm: Some(50.0),
            ..good
        };
        assert!(target.validate().is_ok());
    }

    fn target_spec(kind: LineKind, layer: &str, gap: Option<i64>) -> LineSpec {
        LineSpec {
            width_nm: None,
            target_ohm: Some(50.0),
            ..spec(kind, layer, gap)
        }
    }

    #[test]
    fn synthesis_finds_the_width_for_fifty_ohm() {
        let settings = StackupSettings::default();
        let s = target_spec(LineKind::Microstrip, "top", None);
        let found = synthesize_width(&two_layer(), &materials(), &s, 50.0, 1.0, &settings).unwrap();
        assert!((found.result.impedance_ohm - 50.0).abs() <= settings.synth_tolerance_ohm);
        assert_eq!(found.result.width_nm, found.width_nm);
        // about 3 mm on 1.6 mm of a laminate of permittivity 4.5
        assert!(
            found.width_nm > 2_800_000 && found.width_nm < 3_200_000,
            "{}",
            found.width_nm
        );
        assert!(found.iterations > 0 && found.iterations <= settings.synth_max_iterations);
    }

    #[test]
    fn synthesis_is_the_inverse_of_the_direct_calculation() {
        let settings = StackupSettings::default();
        let direct = spec(LineKind::Microstrip, "top", None);
        let target = target_spec(LineKind::Microstrip, "top", None);
        for width_um in [150, 300, 600, 1200, 2400, 4800] {
            let z = run(&two_layer(), &direct, width_um * 1000, 1.0).impedance_ohm;
            let found =
                synthesize_width(&two_layer(), &materials(), &target, z, 1.0, &settings).unwrap();
            let error_nm = (found.width_nm - width_um * 1000).abs();
            assert!(error_nm < 20_000, "{width_um} um: {error_nm} nm");
        }
    }

    #[test]
    fn synthesis_works_for_a_stripline_a_coplanar_line_and_a_pair() {
        let settings = StackupSettings::default();
        let cases = [
            (
                target_spec(LineKind::Stripline, "inner-1", None),
                four_layer(),
            ),
            (
                target_spec(LineKind::CoplanarGrounded, "top", Some(200_000)),
                two_layer(),
            ),
            (
                target_spec(LineKind::DifferentialMicrostrip, "top", Some(200_000)),
                two_layer(),
            ),
        ];
        for (spec, stack) in cases {
            let target = if spec.kind == LineKind::DifferentialMicrostrip {
                90.0
            } else {
                50.0
            };
            let found = synthesize_width(&stack, &materials(), &spec, target, 1.0, &settings);
            let found = found.unwrap_or_else(|e| panic!("{:?}: {e}", spec.kind));
            assert!(
                (found.result.impedance_ohm - target).abs() <= settings.synth_tolerance_ohm,
                "{:?}",
                spec.kind
            );
        }
    }

    #[test]
    fn synthesis_tolerance_and_steps_are_settings() {
        let s = target_spec(LineKind::Microstrip, "top", None);
        let loose = StackupSettings {
            synth_tolerance_ohm: 2.0,
            ..StackupSettings::default()
        };
        let tight = StackupSettings::default();
        let a = synthesize_width(&two_layer(), &materials(), &s, 50.0, 1.0, &loose).unwrap();
        let b = synthesize_width(&two_layer(), &materials(), &s, 50.0, 1.0, &tight).unwrap();
        assert!(a.iterations < b.iterations);
        // too few steps
        let few = StackupSettings {
            synth_max_iterations: 2,
            ..StackupSettings::default()
        };
        assert!(matches!(
            synthesize_width(&two_layer(), &materials(), &s, 50.0, 1.0, &few),
            Err(StackupError::NotConverged { .. })
        ));
        // a tolerance finer than a nanometre can give
        let absurd = StackupSettings {
            synth_tolerance_ohm: 1.0e-12,
            ..StackupSettings::default()
        };
        assert!(matches!(
            synthesize_width(&two_layer(), &materials(), &s, 50.0, 1.0, &absurd),
            Err(StackupError::NotConverged { .. })
        ));
    }

    #[test]
    fn a_target_the_widths_cannot_reach_is_reported_with_the_limits() {
        let s = target_spec(LineKind::Microstrip, "top", None);
        let settings = StackupSettings::default();
        for target in [0.5, 5000.0] {
            match synthesize_width(&two_layer(), &materials(), &s, target, 1.0, &settings) {
                Err(StackupError::TargetUnreachable {
                    low_ohm, high_ohm, ..
                }) => {
                    assert!(low_ohm < high_ohm);
                    assert!(target < low_ohm || target > high_ohm);
                }
                other => panic!("{target}: {other:?}"),
            }
        }
        for target in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                synthesize_width(&two_layer(), &materials(), &s, target, 1.0, &settings),
                Err(StackupError::InvalidNumber { .. })
            ));
        }
    }

    #[test]
    fn copper_in_ounces_gives_the_same_line_as_the_equal_micrometres() {
        let mut ounces = two_layer();
        ounces.layers[0] = Layer::Copper {
            name: "top".to_string(),
            thickness: CopperThickness::Oz(1.0),
            roughness_rms_nm: 0,
        };
        let mut um = two_layer();
        um.layers[0] = Layer::Copper {
            name: "top".to_string(),
            thickness: CopperThickness::Um(34.798),
            roughness_rms_nm: 0,
        };
        let s = spec(LineKind::Microstrip, "top", None);
        let a = run(&ounces, &s, 1_000_000, 1.0);
        let b = run(&um, &s, 1_000_000, 1.0);
        assert!((a.impedance_ohm - b.impedance_ohm).abs() < 1e-9);
    }
}
