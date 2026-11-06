// SPDX-License-Identifier: GPL-3.0-or-later
//! From the capacitance matrices to the quantities of a line.
//!
//! With the capacitance per unit length `C` of the section as it is and `C_air` of the same
//! section with vacuum in place of every dielectric, and `c` the speed of light:
//!
//! * `Z0 = 1 / (c sqrt(C C_air))` and `eps_eff = C / C_air` (the line is quasi-TEM and the copper
//!   is not magnetic);
//! * the inductance per unit length is `L = 1 / (c^2 C_air)` (the inductance of the vacuum line,
//!   by reciprocity: the same field with the same boundaries); for two conductors the matrix
//!   is `L = C_air^-1 / c^2`, with the mutual inductance `Lm` its off-diagonal term;
//! * for a symmetric pair the coupling is `Cm = -C12`, the capacitance of the even mode is
//!   `Ce = C11 - Cm` and that of the odd mode `Co = C11 + Cm`, with `Ze`, `Zo` as above for each
//!   mode, `Zdiff = 2 Zo` and `Zcommon = Ze / 2`;
//! * the loss tangent of the field is the energy in each cell weighted by the loss tangent of the
//!   cell, over the whole energy, and the dielectric loss is `alpha_d = pi f sqrt(eps_eff) tan /
//!   c` in Np/m (the odd mode for a pair). The loss of the copper is not computed here.

use std::f64::consts::{LN_10, PI};

use eh3d_phoenix_stackup::{
    analyze, synthesize_width, LineKind, LineSpec, MaterialSample, Stackup, StackupJob,
    StackupSettings,
};
use serde::Serialize;

use crate::codes::{FieldError, DIFFERS_FROM_FORMULA, SHAPE_INVALID};
use crate::section::{section_from_stackup, Note};
use crate::settings::Field2dSettings;
use crate::shape::Shape;
use crate::C0;

/// What is particular to a pair of coupled conductors.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairResult {
    /// The impedance of the even mode, in ohm.
    pub z_even_ohm: f64,
    /// The impedance of the odd mode, in ohm.
    pub z_odd_ohm: f64,
    /// The differential impedance, twice the odd mode, in ohm.
    pub z_diff_ohm: f64,
    /// The common-mode impedance, half the even mode, in ohm.
    pub z_common_ohm: f64,
    /// The effective permittivity of the even mode.
    pub eps_even: f64,
    /// The effective permittivity of the odd mode.
    pub eps_odd: f64,
    /// The mutual capacitance `-C12`, in pF/m.
    pub cm_pf_per_m: f64,
    /// The self inductance, in nH/m.
    pub ls_nh_per_m: f64,
    /// The mutual inductance, in nH/m.
    pub lm_nh_per_m: f64,
    /// How different the two conductors came out, `|C11 - C22| / C11`.
    pub asymmetry: f64,
}

/// The result of the field solution of one section.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldResult {
    /// The cells of the mesh.
    pub cells: usize,
    /// The cells in the `x` direction.
    pub nx: usize,
    /// The cells in the `y` direction.
    pub ny: usize,
    /// The most iterations of any of the solutions.
    pub iterations: u32,
    /// The largest final residual, as a fraction of the first.
    pub residual: f64,
    /// The impedance of the line in ohm; for a pair the geometric mean of the two modes.
    pub z0_ohm: f64,
    /// The effective permittivity; for a pair the mean of the two modes.
    pub eps_eff: f64,
    /// The self capacitance in pF/m.
    pub c_pf_per_m: f64,
    /// The self capacitance without the dielectric in pF/m.
    pub c_air_pf_per_m: f64,
    /// The self inductance in nH/m.
    pub l_nh_per_m: f64,
    /// The delay in ps/mm.
    pub delay_ps_per_mm: f64,
    /// The dielectric loss in dB/cm (the odd mode for a pair).
    pub dielectric_loss_db_per_cm: f64,
    /// The quantities of a pair.
    pub pair: Option<PairResult>,
}

fn finite_positive(value: f64, what: &str) -> Result<f64, FieldError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(FieldError::new(
            SHAPE_INVALID,
            format!("{what} is not a positive number"),
        ))
    }
}

fn loss_db_per_cm(freq_ghz: f64, eps_eff: f64, tan_d: f64) -> f64 {
    let alpha_np_per_m = PI * freq_ghz * 1.0e9 * eps_eff.sqrt() / C0 * tan_d;
    alpha_np_per_m * (20.0 / LN_10) / 100.0
}

/// Mesh a section, solve it with and without the dielectric and work out the line.
pub fn analyze_shape(
    shape: &Shape,
    settings: &Field2dSettings,
    freq_ghz: f64,
) -> Result<FieldResult, FieldError> {
    if shape.conductors != 1 && shape.conductors != 2 {
        return Err(FieldError::new(
            SHAPE_INVALID,
            "a section has one conductor or a pair of two",
        ));
    }
    let grid = shape.mesh(settings)?;
    let air = grid.without_dielectric();
    let with = grid.solve(shape.conductors, settings)?;
    let vacuum = air.solve(shape.conductors, settings)?;
    let iterations = with.iterations.max(vacuum.iterations);
    let residual = with.residual.max(vacuum.residual);
    let common = |c: f64, ca: f64| -> Result<f64, FieldError> {
        finite_positive(1.0 / (C0 * (c * ca).sqrt()), "the impedance")
    };
    if shape.conductors == 1 {
        let c = finite_positive(with.c[0][0], "the capacitance")?;
        let ca = finite_positive(vacuum.c[0][0], "the capacitance without dielectric")?;
        let eps_eff = finite_positive(c / ca, "the effective permittivity")?;
        let tan_d = grid.loss_tangent(&with, &[1.0]);
        return Ok(FieldResult {
            cells: grid.cells(),
            nx: grid.nx(),
            ny: grid.ny(),
            iterations,
            residual,
            z0_ohm: common(c, ca)?,
            eps_eff,
            c_pf_per_m: c * 1.0e12,
            c_air_pf_per_m: ca * 1.0e12,
            l_nh_per_m: 1.0 / (C0 * C0 * ca) * 1.0e9,
            delay_ps_per_mm: eps_eff.sqrt() / C0 * 1.0e9,
            dielectric_loss_db_per_cm: loss_db_per_cm(freq_ghz, eps_eff, tan_d),
            pair: None,
        });
    }
    let (c, v) = (&with.c, &vacuum.c);
    let c11 = finite_positive((c[0][0] + c[1][1]) / 2.0, "the self capacitance")?;
    let cm = -(c[0][1] + c[1][0]) / 2.0;
    let a11 = finite_positive((v[0][0] + v[1][1]) / 2.0, "the self capacitance in vacuum")?;
    let am = -(v[0][1] + v[1][0]) / 2.0;
    let (c_even, c_odd) = (c11 - cm, c11 + cm);
    let (a_even, a_odd) = (a11 - am, a11 + am);
    let z_even = common(c_even, a_even)?;
    let z_odd = common(c_odd, a_odd)?;
    let eps_even = finite_positive(c_even / a_even, "the permittivity of the even mode")?;
    let eps_odd = finite_positive(c_odd / a_odd, "the permittivity of the odd mode")?;
    let det = a11 * a11 - am * am;
    let ls = finite_positive(a11 / det / (C0 * C0), "the self inductance")?;
    let lm = am / det / (C0 * C0);
    let tan_odd = grid.loss_tangent(&with, &[1.0, -1.0]);
    let eps_mean = (eps_even + eps_odd) / 2.0;
    Ok(FieldResult {
        cells: grid.cells(),
        nx: grid.nx(),
        ny: grid.ny(),
        iterations,
        residual,
        z0_ohm: (z_even * z_odd).sqrt(),
        eps_eff: eps_mean,
        c_pf_per_m: c11 * 1.0e12,
        c_air_pf_per_m: a11 * 1.0e12,
        l_nh_per_m: ls * 1.0e9,
        delay_ps_per_mm: eps_mean.sqrt() / C0 * 1.0e9,
        dielectric_loss_db_per_cm: loss_db_per_cm(freq_ghz, eps_odd, tan_odd),
        pair: Some(PairResult {
            z_even_ohm: z_even,
            z_odd_ohm: z_odd,
            z_diff_ohm: 2.0 * z_odd,
            z_common_ohm: z_even / 2.0,
            eps_even,
            eps_odd,
            cm_pf_per_m: cm * 1.0e12,
            ls_nh_per_m: ls * 1.0e9,
            lm_nh_per_m: lm * 1.0e9,
            asymmetry: (c[0][0] - c[1][1]).abs() / c11,
        }),
    })
}

/// The field solution set against the closed formulas of the stack-up crate.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Comparison {
    /// The impedance of the closed formula in ohm (`Zdiff` for a pair).
    pub closed_impedance_ohm: f64,
    /// The impedance of the field solver in ohm (`Zdiff` for a pair).
    pub field_impedance_ohm: f64,
    /// `100 (field / closed - 1)`.
    pub impedance_difference_percent: f64,
    /// The effective permittivity of the closed formula (a single line only).
    pub closed_eps_eff: Option<f64>,
    /// The effective permittivity of the field solver (a single line only).
    pub field_eps_eff: Option<f64>,
    /// `100 (field / closed - 1)` for the permittivity.
    pub eps_difference_percent: Option<f64>,
    /// Whether the impedance differs by more than `compare_warn_percent`.
    pub exceeds_limit: bool,
}

/// One line solved by the field solver, with the comparison.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LineField {
    /// The name of the line.
    pub name: String,
    /// The kind of line.
    pub kind: LineKind,
    /// The frequency at which the materials were taken, in GHz.
    pub freq_ghz: f64,
    /// The width of the track in nanometres.
    pub width_nm: i64,
    /// The gap in nanometres, for the kinds that have one.
    pub gap_nm: Option<i64>,
    /// Whether the width was found from a target impedance by the closed formula.
    pub synthesized: bool,
    /// The solution.
    pub field: FieldResult,
    /// The comparison with the closed formula, when the formula could be computed.
    pub comparison: Option<Comparison>,
    /// What to know about the model.
    pub notes: Vec<Note>,
}

fn percent(field: f64, closed: f64) -> f64 {
    100.0 * (field / closed - 1.0)
}

/// Solve one line of a stack-up with a width and compare it with the closed formula.
#[allow(clippy::too_many_arguments)]
pub fn solve_line(
    stackup: &Stackup,
    materials: &[MaterialSample],
    spec: &LineSpec,
    width_nm: i64,
    freq_ghz: f64,
    synthesized: bool,
    settings: &Field2dSettings,
    stackup_settings: &StackupSettings,
) -> Result<LineField, FieldError> {
    let section = section_from_stackup(stackup, materials, spec, width_nm, freq_ghz, settings)?;
    let field = analyze_shape(&section.shape, settings, freq_ghz)?;
    let mut notes = section.notes;
    let comparison = analyze(
        stackup,
        materials,
        spec,
        width_nm,
        freq_ghz,
        stackup_settings,
    )
    .ok()
    .map(|closed| {
        let (field_z, single) = match &field.pair {
            Some(pair) => (pair.z_diff_ohm, false),
            None => (field.z0_ohm, true),
        };
        let difference = percent(field_z, closed.impedance_ohm);
        Comparison {
            closed_impedance_ohm: closed.impedance_ohm,
            field_impedance_ohm: field_z,
            impedance_difference_percent: difference,
            closed_eps_eff: single.then_some(closed.eps_eff),
            field_eps_eff: single.then_some(field.eps_eff),
            eps_difference_percent: single.then(|| percent(field.eps_eff, closed.eps_eff)),
            exceeds_limit: difference.abs() > settings.compare_warn_percent,
        }
    });
    if let Some(found) = &comparison {
        if found.exceeds_limit {
            notes.push(Note::new(
                DIFFERS_FROM_FORMULA,
                format!(
                    "the field solver gives {:.3} ohm and the formula {:.3} ohm, {:+.2} %",
                    found.field_impedance_ohm,
                    found.closed_impedance_ohm,
                    found.impedance_difference_percent
                ),
            ));
        }
    }
    Ok(LineField {
        name: spec.name.clone(),
        kind: spec.kind,
        freq_ghz,
        width_nm,
        gap_nm: spec.gap_nm,
        synthesized,
        field,
        comparison,
        notes,
    })
}

/// What happened to one line of a job.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseOutcome {
    /// The name of the line.
    pub name: String,
    /// The solution, or why there is none.
    pub result: Result<LineField, FieldError>,
}

/// Solve every line of a job file of `phoenix stackup`. A line given by its width is solved with
/// that width; a line given by a target impedance first gets its width from the closed formula
/// and the field solver is run on that width, so the comparison shows how far the field is
/// from the target. A line that cannot be solved gives its error and the others go on.
pub fn run_job(
    job: &StackupJob,
    freq_ghz: f64,
    settings: &Field2dSettings,
    stackup_settings: &StackupSettings,
) -> Vec<CaseOutcome> {
    job.lines
        .iter()
        .take(settings.max_cases)
        .map(|spec| {
            let width = match (spec.width_nm, spec.target_ohm) {
                (Some(width), _) => Ok((width, false)),
                (None, Some(target)) => synthesize_width(
                    &job.stackup,
                    &job.materials,
                    spec,
                    target,
                    freq_ghz,
                    stackup_settings,
                )
                .map(|found| (found.width_nm, true))
                .map_err(|e| FieldError::new(crate::codes::LINE_UNAVAILABLE, e.to_string())),
                (None, None) => Err(FieldError::new(
                    crate::codes::LINE_UNAVAILABLE,
                    "give a width or a target impedance",
                )),
            };
            let result = width.and_then(|(width_nm, synthesized)| {
                solve_line(
                    &job.stackup,
                    &job.materials,
                    spec,
                    width_nm,
                    freq_ghz,
                    synthesized,
                    settings,
                    stackup_settings,
                )
            });
            CaseOutcome {
                name: spec.name.clone(),
                result,
            }
        })
        .collect()
}
