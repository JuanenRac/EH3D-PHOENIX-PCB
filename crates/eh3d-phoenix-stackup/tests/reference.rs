// SPDX-License-Identifier: GPL-3.0-or-later
//! Reference cases for the stack-up calculations: widths that are quoted for common boards,
//! cross-checks between independent published formulas, properties that must always hold, and
//! the behaviour on the edges of the input.
//!
//! The material values written here are the ones commonly quoted for the cases (a relative
//! permittivity of 4.5 for FR-4, the design value of 3.66 at 10 GHz for RO4350B); they are
//! inputs of the cases, not data of a sheet of this repository.

use eh3d_phoenix_stackup::models;
use eh3d_phoenix_stackup::{
    analyze, synthesize_width, CopperThickness, Layer, LineKind, LineSpec, MaterialSample, Stackup,
    StackupError, StackupSettings,
};

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

fn core_layer(name: &str, thickness_nm: i64, material: &str) -> Layer {
    Layer::Core {
        name: name.to_string(),
        thickness_nm,
        material: material.to_string(),
    }
}

fn spec(kind: LineKind, layer: &str, gap_nm: Option<i64>, target: Option<f64>) -> LineSpec {
    LineSpec {
        name: "line".to_string(),
        kind,
        layer: layer.to_string(),
        width_nm: if target.is_some() {
            None
        } else {
            Some(500_000)
        },
        target_ohm: target,
        gap_nm,
    }
}

fn two_layer(thickness_nm: i64, material: &str, copper_um: f64) -> Stackup {
    Stackup {
        name: "two-layer".to_string(),
        layers: vec![
            copper("top", copper_um),
            core_layer("core", thickness_nm, material),
            copper("bottom", copper_um),
        ],
    }
}

fn four_layer() -> Stackup {
    Stackup {
        name: "four-layer".to_string(),
        layers: vec![
            copper("top", 35.0),
            core_layer("prepreg-1", 200_000, "fr4"),
            copper("inner-1", 17.5),
            core_layer("core", 800_000, "fr4"),
            copper("inner-2", 17.5),
            core_layer("prepreg-2", 200_000, "fr4"),
            copper("bottom", 35.0),
        ],
    }
}

fn width_for(stackup: &Stackup, materials: &[MaterialSample], spec: &LineSpec, freq: f64) -> f64 {
    let target = spec.target_ohm.unwrap_or(50.0);
    synthesize_width(
        stackup,
        materials,
        spec,
        target,
        freq,
        &StackupSettings::default(),
    )
    .unwrap()
    .width_nm as f64
        * 1.0e-6
}

#[test]
fn fifty_ohm_microstrip_on_1_6_mm_of_fr4_is_about_3_mm_wide() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let stack = two_layer(1_600_000, "fr4", 35.0);
    let s = spec(LineKind::Microstrip, "top", None, Some(50.0));
    let width_mm = width_for(&stack, &materials, &s, 1.0);
    assert!((2.8..=3.2).contains(&width_mm), "{width_mm} mm");
}

#[test]
fn fifty_ohm_microstrip_on_0_508_mm_of_ro4350b_is_about_1_1_mm_wide() {
    let ro4350b = MaterialSample {
        name: "ro4350b".to_string(),
        dk_points: vec![(10.0, 3.66)],
        df_points: vec![(10.0, 0.0037)],
        thickness_nm: Some(508_000),
    };
    let stack = two_layer(508_000, "ro4350b", 35.0);
    let s = spec(LineKind::Microstrip, "top", None, Some(50.0));
    let width_mm = width_for(&stack, &[ro4350b], &s, 10.0);
    assert!((1.0..=1.2).contains(&width_mm), "{width_mm} mm");
}

#[test]
fn the_half_ounce_and_the_one_ounce_copper_change_the_width_a_little() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let s = spec(LineKind::Microstrip, "top", None, Some(50.0));
    let thin = width_for(&two_layer(1_600_000, "fr4", 17.5), &materials, &s, 1.0);
    let thick = width_for(&two_layer(1_600_000, "fr4", 70.0), &materials, &s, 1.0);
    // a thicker track needs to be a little narrower for the same impedance
    assert!(thick < thin);
    assert!(thin - thick < 0.3, "{thin} {thick}");
}

#[test]
fn a_stripline_agrees_with_cohn_and_with_ipc_2141() {
    let er = 4.5;
    let b = 1.0e-3;
    let t = 20.0e-6;
    // w / (b - t) = 0.2, inside the range of the IPC-2141 formula
    let w = 0.2 * (b - t);
    let ours = models::stripline_symmetric(w, t, b, er).unwrap();
    let ipc = 60.0 / er.sqrt() * (4.0 * b / (0.67 * std::f64::consts::PI * (0.8 * w + t))).ln();
    assert!((ours - ipc).abs() < ipc * 0.03, "{ours} {ipc}");
    // against the exact value for a strip of zero thickness
    let exact = models::stripline_zero_thickness_exact(w, b, er).unwrap();
    let thin = models::stripline_symmetric(w, 0.0, b, er).unwrap();
    assert!((thin - exact).abs() < exact * 0.01, "{thin} {exact}");
    // the thickness lowers the impedance
    assert!(ours < thin);
}

#[test]
fn fifty_ohm_stripline_in_a_four_layer_board_is_a_few_tenths_of_a_millimetre() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let s = spec(LineKind::Stripline, "inner-1", None, Some(50.0));
    let width_mm = width_for(&four_layer(), &materials, &s, 1.0);
    assert!((0.1..=0.6).contains(&width_mm), "{width_mm} mm");
}

#[test]
fn impedance_falls_with_the_width_for_every_kind_of_line() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let cases = [
        (LineKind::Microstrip, "top", None),
        (LineKind::Stripline, "inner-1", None),
        (LineKind::Coplanar, "top", Some(300_000)),
        (LineKind::CoplanarGrounded, "top", Some(300_000)),
        (LineKind::DifferentialMicrostrip, "top", Some(200_000)),
        (LineKind::DifferentialStripline, "inner-1", Some(200_000)),
    ];
    for (kind, layer, gap) in cases {
        let s = spec(kind, layer, gap, None);
        let mut last = f64::INFINITY;
        for width_um in [120, 160, 220, 300, 420, 600, 850, 1200] {
            let r = analyze(
                &four_layer(),
                &materials,
                &s,
                width_um * 1000,
                1.0,
                &StackupSettings::default(),
            )
            .unwrap();
            assert!(r.impedance_ohm < last, "{kind:?} {width_um} um");
            last = r.impedance_ohm;
        }
    }
}

#[test]
fn the_synthesis_inverts_the_direct_calculation_for_every_kind_of_line() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let settings = StackupSettings::default();
    let cases = [
        (LineKind::Microstrip, "top", None),
        (LineKind::Stripline, "inner-1", None),
        (LineKind::Coplanar, "top", Some(300_000)),
        (LineKind::CoplanarGrounded, "top", Some(300_000)),
        (LineKind::DifferentialMicrostrip, "top", Some(200_000)),
        (LineKind::DifferentialStripline, "inner-1", Some(200_000)),
    ];
    for (kind, layer, gap) in cases {
        for width_um in [150_i64, 300, 600] {
            let direct = spec(kind, layer, gap, None);
            let z = analyze(
                &four_layer(),
                &materials,
                &direct,
                width_um * 1000,
                1.0,
                &settings,
            )
            .unwrap()
            .impedance_ohm;
            let wanted = spec(kind, layer, gap, Some(z));
            let found =
                synthesize_width(&four_layer(), &materials, &wanted, z, 1.0, &settings).unwrap();
            assert!(
                (found.width_nm - width_um * 1000).abs() < 30_000,
                "{kind:?} {width_um} um: {} nm",
                found.width_nm
            );
            assert!((found.result.impedance_ohm - z).abs() <= settings.synth_tolerance_ohm);
        }
    }
}

#[test]
fn a_higher_permittivity_lowers_the_impedance_and_raises_the_delay() {
    let s = spec(LineKind::Microstrip, "top", None, None);
    let settings = StackupSettings::default();
    let mut last_z = f64::INFINITY;
    let mut last_delay = 0.0;
    for dk in [2.2, 3.0, 4.0, 6.0, 10.0] {
        let materials = [flat("fr4", dk, 0.01)];
        let r = analyze(
            &two_layer(800_000, "fr4", 35.0),
            &materials,
            &s,
            1_000_000,
            1.0,
            &settings,
        )
        .unwrap();
        assert!(r.impedance_ohm < last_z && r.delay_ps_per_mm > last_delay);
        last_z = r.impedance_ohm;
        last_delay = r.delay_ps_per_mm;
    }
}

#[test]
fn the_losses_grow_with_the_frequency_and_the_loss_tangent() {
    let s = spec(LineKind::Microstrip, "top", None, None);
    let settings = StackupSettings::default();
    let stack = two_layer(800_000, "fr4", 35.0);
    let mut last_total = 0.0;
    for freq in [0.5, 1.0, 2.0, 5.0, 10.0] {
        let r = analyze(
            &stack,
            &[flat("fr4", 4.2, 0.02)],
            &s,
            1_500_000,
            freq,
            &settings,
        )
        .unwrap();
        assert!(r.total_loss_db_per_cm > last_total, "{freq} GHz");
        last_total = r.total_loss_db_per_cm;
    }
    let low = analyze(
        &stack,
        &[flat("fr4", 4.2, 0.004)],
        &s,
        1_500_000,
        5.0,
        &settings,
    )
    .unwrap();
    let high = analyze(
        &stack,
        &[flat("fr4", 4.2, 0.02)],
        &s,
        1_500_000,
        5.0,
        &settings,
    )
    .unwrap();
    assert!(high.dielectric_loss_db_per_cm > low.dielectric_loss_db_per_cm);
    assert_eq!(high.conductor_loss_db_per_cm, low.conductor_loss_db_per_cm);
}

#[test]
fn nothing_panics_and_nothing_is_not_a_number_on_the_edges_of_the_input() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let settings = StackupSettings::default();
    let kinds = [
        (LineKind::Microstrip, "top", None),
        (LineKind::Stripline, "inner-1", None),
        (LineKind::Coplanar, "top", Some(100_000)),
        (LineKind::CoplanarGrounded, "top", Some(1)),
        (LineKind::DifferentialMicrostrip, "top", Some(1)),
        (
            LineKind::DifferentialStripline,
            "inner-1",
            Some(1_000_000_000),
        ),
    ];
    for (kind, layer, gap) in kinds {
        let s = spec(kind, layer, gap, None);
        for width in [1_i64, 1_000, 100_000, 5_000_000, 1_000_000_000] {
            for freq in [0.1, 1.0, 50.0] {
                match analyze(&four_layer(), &materials, &s, width, freq, &settings) {
                    Ok(r) => {
                        assert!(r.impedance_ohm.is_finite() && r.impedance_ohm > 0.0);
                        assert!(r.eps_eff.is_finite() && r.eps_eff >= 1.0);
                        assert!(r.total_loss_db_per_cm.is_finite());
                        assert!(r.total_loss_db_per_cm >= 0.0);
                    }
                    Err(e) => {
                        // a typed error with a registered code
                        assert!(e.code().starts_with("PHOENIX-STACK-"), "{e}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_frequency_outside_the_published_points_is_an_error_and_not_a_guess() {
    let materials = [flat("fr4", 4.5, 0.02)];
    let s = spec(LineKind::Microstrip, "top", None, None);
    for freq in [0.05, 60.0] {
        let e = analyze(
            &two_layer(1_600_000, "fr4", 35.0),
            &materials,
            &s,
            3_000_000,
            freq,
            &StackupSettings::default(),
        )
        .unwrap_err();
        assert!(matches!(e, StackupError::OutsidePublished { .. }), "{freq}");
    }
}

#[test]
fn a_single_stackup_serialises_and_reads_back_the_same() {
    let stack = four_layer();
    let text = serde_json::to_string(&stack).unwrap();
    let back: Stackup = serde_json::from_str(&text).unwrap();
    assert_eq!(back, stack);
}
