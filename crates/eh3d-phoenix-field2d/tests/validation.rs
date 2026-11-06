// SPDX-License-Identifier: GPL-3.0-or-later
//! The field solver against solutions that are known and against the closed formulas of the
//! stack-up crate. The sources of the references:
//!
//! * the coaxial line of circular conductors, `C = 2 pi eps / ln(b/a)` per unit length (any text
//!   on transmission lines, for example D. M. Pozar, "Microwave Engineering", the section on the
//!   coaxial line; the number of the equation is not verified here). The circles are drawn as
//!   steps of the mesh, so the answer is only close and gets closer as the steps get smaller;
//! * the stripline of zero thickness, exact (S. B. Cohn, "Characteristic impedance of the
//!   shielded-strip transmission line", IRE Transactions on Microwave Theory and Techniques,
//!   July 1954; the page numbers are not verified here), as implemented in
//!   `eh3d_phoenix_stackup::models::stripline_zero_thickness_exact`;
//! * the microstrip of E. Hammerstad and O. Jensen (1980), as implemented in
//!   `eh3d_phoenix_stackup::models::microstrip`;
//! * the capacitor of parallel plates and the capacitors in series are in the unit tests of
//!   `grid.rs`.

use eh3d_phoenix_field2d::codes::{MESH_TOO_LARGE, NOT_CONVERGED};
use eh3d_phoenix_field2d::grid::{Grid, Rect, Wall, Walls};
use eh3d_phoenix_field2d::mesh::uniform_lines;
use eh3d_phoenix_field2d::{
    analyze_shape, run_job, solve_line, Block, Electrode, Field2dSettings, Method, Shape, C0, EPS0,
};
use eh3d_phoenix_stackup::models::{microstrip, stripline_zero_thickness_exact};
use eh3d_phoenix_stackup::{
    analyze, CopperThickness, Layer, LineKind, LineSpec, MaterialSample, Stackup, StackupJob,
    StackupSettings,
};

fn microstrip_shape(w: f64, t: f64, h: f64, er: f64, tan_d: f64, factor: f64) -> Shape {
    let margin = factor * (h + w);
    let half = w / 2.0 + margin;
    let mut blocks = Vec::new();
    if er > 1.0 {
        blocks.push(Block {
            rect: Rect::new(-half, half, 0.0, h),
            eps_r: er,
            tan_d,
        });
    }
    Shape {
        domain: Rect::new(-half, half, 0.0, h + t + margin),
        walls: Walls::GROUNDED,
        blocks,
        electrodes: vec![Electrode {
            rect: Rect::new(-w / 2.0, w / 2.0, h, h + t),
            id: 1,
        }],
        grounds: Vec::new(),
        conductors: 1,
    }
}

fn stripline_shape(w: f64, t: f64, b: f64, er: f64, side: f64) -> Shape {
    let half = w / 2.0 + side;
    let y0 = (b - t) / 2.0;
    Shape {
        domain: Rect::new(-half, half, 0.0, b),
        walls: Walls::GROUNDED,
        blocks: vec![Block {
            rect: Rect::new(-half, half, 0.0, b),
            eps_r: er,
            tan_d: 0.0,
        }],
        electrodes: vec![Electrode {
            rect: Rect::new(-w / 2.0, w / 2.0, y0, y0 + t),
            id: 1,
        }],
        grounds: Vec::new(),
        conductors: 1,
    }
}

fn percent(a: f64, b: f64) -> f64 {
    100.0 * (a / b - 1.0)
}

/// A quarter of a coaxial line of inner radius `a` and outer radius `b`, drawn with cells of side
/// `a / n`; the walls on the axes are planes of symmetry. Returns the capacitance of the whole
/// line per metre.
fn coax_capacitance(a: f64, b: f64, n: usize) -> f64 {
    let step = a / n as f64;
    let cells = (b / step).ceil() as usize + 1;
    let lines = uniform_lines(0.0, step * cells as f64, cells);
    let walls = Walls {
        left: Wall::Symmetry,
        bottom: Wall::Symmetry,
        right: Wall::Ground,
        top: Wall::Ground,
    };
    let mut grid = Grid::new(lines.clone(), lines, walls, 1_000_000).unwrap();
    grid.paint_conductor_where(1, |x, y| x.hypot(y) < a);
    grid.paint_conductor_where(0, |x, y| x.hypot(y) > b);
    let settings = Field2dSettings {
        tolerance: 1.0e-11,
        ..Field2dSettings::default()
    };
    4.0 * grid.solve(1, &settings).unwrap().c[0][0]
}

#[test]
fn the_coaxial_line_is_close_to_the_formula_and_closer_with_smaller_steps() {
    let a: f64 = 1.0e-3;
    let b: f64 = 2.3e-3;
    let exact = 2.0 * std::f64::consts::PI * EPS0 / (b / a).ln();
    let errors: Vec<f64> = [10, 20, 40]
        .iter()
        .map(|n| percent(coax_capacitance(a, b, *n), exact).abs())
        .collect();
    // steps drawn on a circle overstate the inner conductor, so the capacitance is a little high
    assert!(errors[0] < 7.0, "{errors:?}");
    assert!(errors[1] < 3.5, "{errors:?}");
    assert!(errors[2] < 1.6, "{errors:?}");
    assert!(errors[0] > errors[1] && errors[1] > errors[2], "{errors:?}");
}

#[test]
fn a_thin_centred_stripline_agrees_with_cohn_to_a_percent_and_a_half() {
    // the strip is 2 micrometres thick, so the exact value for zero thickness is a little high
    let (w, t, b, er) = (1.0e-3, 2.0e-6, 1.0e-3, 4.0);
    let shape = stripline_shape(w, t, b, er, 4.0 * b);
    let found = analyze_shape(&shape, &Field2dSettings::default(), 1.0).unwrap();
    let cohn = stripline_zero_thickness_exact(w, b, er).unwrap();
    let difference = percent(found.z0_ohm, cohn);
    assert!(
        difference.abs() < 1.5,
        "{} {cohn} {difference}",
        found.z0_ohm
    );
    assert!((found.eps_eff - er).abs() < 1.0e-6, "{}", found.eps_eff);
}

#[test]
fn a_wide_microstrip_in_vacuum_agrees_with_hammerstad_and_jensen() {
    let (h, w) = (0.2e-3, 2.0e-3);
    let shape = microstrip_shape(w, 0.4e-6, h, 1.0, 0.0, 8.0);
    let found = analyze_shape(&shape, &Field2dSettings::default(), 1.0).unwrap();
    let (reference, _) = microstrip(w, 0.0, h, 1.0).unwrap();
    let difference = percent(found.z0_ohm, reference);
    assert!(difference.abs() < 1.0, "{} {reference}", found.z0_ohm);
    // the parallel plates alone give eps0 w / h per metre; the fringing adds to it
    let plates = EPS0 * w / h * 1.0e12;
    let ratio = found.c_pf_per_m / plates;
    assert!(ratio > 1.0 && ratio < 1.5, "{ratio}");
}

#[test]
fn the_error_falls_as_the_mesh_is_refined() {
    let shape = microstrip_shape(0.3e-3, 0.035e-3, 0.2e-3, 4.3, 0.02, 8.0);
    let z: Vec<f64> = [1.0, 2.0, 4.0]
        .iter()
        .map(|refine| {
            let settings = Field2dSettings {
                refine: *refine,
                ..Field2dSettings::default()
            };
            analyze_shape(&shape, &settings, 1.0).unwrap().z0_ohm
        })
        .collect();
    let first = (z[1] - z[0]).abs();
    let second = (z[2] - z[1]).abs();
    assert!(first > 0.0 && second < 0.9 * first, "{z:?}");
}

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

fn two_layer() -> Stackup {
    Stackup {
        name: "two".to_string(),
        layers: vec![
            copper("top", 35.0),
            Layer::Core {
                name: "core".to_string(),
                thickness_nm: 200_000,
                material: "fr4".to_string(),
            },
            copper("bottom", 35.0),
        ],
    }
}

fn spec(kind: LineKind, gap: Option<i64>) -> LineSpec {
    LineSpec {
        name: "line".to_string(),
        kind,
        layer: "top".to_string(),
        width_nm: Some(300_000),
        target_ohm: None,
        gap_nm: gap,
    }
}

fn solve(kind: LineKind, gap: Option<i64>) -> eh3d_phoenix_field2d::LineField {
    solve_line(
        &two_layer(),
        &[flat("fr4", 4.3, 0.02)],
        &spec(kind, gap),
        300_000,
        1.0,
        false,
        &Field2dSettings::default(),
        &StackupSettings::default(),
    )
    .unwrap()
}

#[test]
fn a_microstrip_of_the_stackup_is_within_a_few_percent_of_the_closed_formula() {
    let found = solve(LineKind::Microstrip, None);
    let comparison = found.comparison.clone().unwrap();
    assert!(
        comparison.impedance_difference_percent.abs() < 3.0,
        "{comparison:?}"
    );
    assert!(comparison.eps_difference_percent.unwrap().abs() < 4.0);
    assert!(!comparison.exceeds_limit);
    assert!(found.notes.is_empty());
    // the loss of the dielectric agrees with the closed formula to a few percent
    let closed = analyze(
        &two_layer(),
        &[flat("fr4", 4.3, 0.02)],
        &spec(LineKind::Microstrip, None),
        300_000,
        1.0,
        &StackupSettings::default(),
    )
    .unwrap();
    let ratio = found.field.dielectric_loss_db_per_cm / closed.dielectric_loss_db_per_cm;
    assert!(ratio > 0.85 && ratio < 1.1, "{ratio}");
}

#[test]
fn the_inductance_and_the_delay_follow_from_the_capacitance() {
    let found = solve(LineKind::Microstrip, None).field;
    let l = found.l_nh_per_m * 1.0e-9;
    let c = found.c_pf_per_m * 1.0e-12;
    let c_air = found.c_air_pf_per_m * 1.0e-12;
    assert!((l * c_air * C0 * C0 - 1.0).abs() < 1.0e-9);
    assert!((found.z0_ohm - (l / c).sqrt()).abs() < 1.0e-9 * found.z0_ohm);
    let delay = found.eps_eff.sqrt() * 1.0e9 / C0;
    assert!((found.delay_ps_per_mm - delay).abs() < 1.0e-9);
}

#[test]
fn a_pair_far_apart_is_two_lines_and_a_pair_close_together_is_coupled() {
    let single = solve(LineKind::Microstrip, None).field;
    let far = solve(LineKind::DifferentialMicrostrip, Some(2_000_000));
    let near = solve(LineKind::DifferentialMicrostrip, Some(100_000));
    let (far_pair, near_pair) = (far.field.pair.unwrap(), near.field.pair.unwrap());
    // the two conductors came out the same
    assert!(far_pair.asymmetry < 1.0e-3 && near_pair.asymmetry < 1.0e-3);
    // far apart: the differential impedance is twice that of one line
    let ratio = far_pair.z_diff_ohm / (2.0 * single.z0_ohm);
    assert!(ratio > 0.97 && ratio < 1.01, "{ratio}");
    // close: the even mode is above the odd mode, the common impedance above half the single one
    assert!(near_pair.z_even_ohm > single.z0_ohm && single.z0_ohm > near_pair.z_odd_ohm);
    assert!(near_pair.z_diff_ohm < 2.0 * single.z0_ohm);
    assert!(near_pair.z_common_ohm < near_pair.z_even_ohm);
    assert!(near_pair.cm_pf_per_m > far_pair.cm_pf_per_m);
    assert!(near_pair.lm_nh_per_m > far_pair.lm_nh_per_m);
    assert!(near_pair.lm_nh_per_m > 0.0 && near_pair.lm_nh_per_m < near_pair.ls_nh_per_m);
    // the comparison of a pair is made on the differential impedance
    let comparison = near.comparison.unwrap();
    assert_eq!(comparison.field_impedance_ohm, near_pair.z_diff_ohm);
    assert!(comparison.closed_eps_eff.is_none());
}

#[test]
fn a_coplanar_line_with_far_ground_copper_tends_to_the_microstrip() {
    let microstrip_found = solve(LineKind::Microstrip, None).field;
    let coplanar = solve(LineKind::CoplanarGrounded, Some(4_000_000)).field;
    let difference = percent(coplanar.z0_ohm, microstrip_found.z0_ohm);
    assert!(difference.abs() < 2.0, "{difference}");
    // closer ground copper lowers the impedance
    let close = solve(LineKind::CoplanarGrounded, Some(150_000)).field;
    assert!(close.z0_ohm < microstrip_found.z0_ohm - 3.0);
}

#[test]
fn the_same_input_gives_the_same_numbers() {
    let shape = microstrip_shape(0.3e-3, 0.035e-3, 0.2e-3, 4.3, 0.02, 8.0);
    let settings = Field2dSettings::default();
    let a = analyze_shape(&shape, &settings, 1.0).unwrap();
    let b = analyze_shape(&shape, &settings, 1.0).unwrap();
    assert_eq!(a, b);
}

#[test]
fn successive_over_relaxation_gives_the_same_line_as_conjugate_gradients() {
    let shape = microstrip_shape(0.3e-3, 0.035e-3, 0.2e-3, 4.3, 0.02, 8.0);
    let cg = analyze_shape(&shape, &Field2dSettings::default(), 1.0).unwrap();
    let sor_settings = Field2dSettings {
        method: Method::Sor,
        tolerance: 1.0e-9,
        max_iterations: 200_000,
        ..Field2dSettings::default()
    };
    let sor = analyze_shape(&shape, &sor_settings, 1.0).unwrap();
    assert!(((sor.z0_ohm - cg.z0_ohm) / cg.z0_ohm).abs() < 1.0e-5);
}

#[test]
fn the_limits_of_cells_and_of_iterations_refuse_and_give_no_number() {
    let shape = microstrip_shape(0.3e-3, 0.035e-3, 0.2e-3, 4.3, 0.02, 8.0);
    let few_cells = Field2dSettings {
        max_cells: 500,
        ..Field2dSettings::default()
    };
    assert_eq!(
        analyze_shape(&shape, &few_cells, 1.0).unwrap_err().code,
        MESH_TOO_LARGE
    );
    let few_iterations = Field2dSettings {
        max_iterations: 3,
        ..Field2dSettings::default()
    };
    assert_eq!(
        analyze_shape(&shape, &few_iterations, 1.0)
            .unwrap_err()
            .code,
        NOT_CONVERGED
    );
}

const JOB: &str = r#"{
  "stackup": {"name": "two", "layers": [
    {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
    {"kind": "core", "name": "core", "thickness_nm": 200000, "material": "fr4"},
    {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
  "materials": [{"name": "fr4", "dk_points": [[0.1, 4.3], [50.0, 4.3]],
                 "df_points": [[0.1, 0.02], [50.0, 0.02]]}],
  "lines": [
    {"name": "a", "kind": "microstrip", "layer": "top", "width_nm": 300000},
    {"name": "b", "kind": "microstrip", "layer": "top", "target_ohm": 50.0},
    {"name": "c", "kind": "stripline", "layer": "top", "width_nm": 300000}]
}"#;

#[test]
fn a_job_solves_each_line_and_a_line_that_cannot_be_taken_gives_its_error() {
    let job = StackupJob::parse(JOB).unwrap();
    let outcomes = run_job(
        &job,
        1.0,
        &Field2dSettings::default(),
        &StackupSettings::default(),
    );
    assert_eq!(outcomes.len(), 3);
    let a = outcomes[0].result.as_ref().unwrap();
    assert!(!a.synthesized);
    let b = outcomes[1].result.as_ref().unwrap();
    assert!(b.synthesized);
    // the width found by the closed formula for 50 ohm gives about 50 ohm in the field solver
    assert!((b.field.z0_ohm - 50.0).abs() < 3.0, "{}", b.field.z0_ohm);
    let c = outcomes[2].result.as_ref().unwrap_err();
    assert_eq!(c.code, "PHOENIX-FIELD-005");
}
