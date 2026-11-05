// SPDX-License-Identifier: GPL-3.0-or-later
//! The generator on a whole board: values of the textbooks, and the properties that every
//! structure must have (the rules pass, the board stays consistent, undoing gives the original
//! back, the same request gives the same bytes).

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};
use eh3d_phoenix_rfgen::codes::{KIND_UNKNOWN, LAYER_NOT_FOUND, NOT_REALISABLE, PARAMETER_INVALID};
use eh3d_phoenix_rfgen::{
    generate, new_board, parse_stackup_file, Environment, Generated, Kind, Params, Placement,
    Request, RfgenError, RfgenSettings,
};
use eh3d_phoenix_stackup::line::{analyze, LineKind, LineSpec};
use eh3d_phoenix_stackup::StackupSettings;

const MATERIALS: &str = r#""materials": [
  {"name": "fr4", "dk_points": [[0.001, 4.4], [200.0, 4.4]], "df_points": [[0.001, 0.02], [200.0, 0.02]]},
  {"name": "prepreg", "dk_points": [[0.001, 4.2], [200.0, 4.2]], "df_points": [[0.001, 0.02], [200.0, 0.02]]}
]"#;

fn two_layer_text() -> String {
    format!(
        r#"{{"stackup": {{"name": "two-layer", "layers": [
          {{"kind": "copper", "name": "top", "thickness": {{"um": 35.0}}}},
          {{"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"}},
          {{"kind": "copper", "name": "bottom", "thickness": {{"um": 35.0}}}}]}},
          {MATERIALS}}}"#
    )
}

fn four_layer_text() -> String {
    format!(
        r#"{{"stackup": {{"name": "four-layer", "layers": [
          {{"kind": "copper", "name": "top", "thickness": {{"um": 35.0}}}},
          {{"kind": "prepreg", "name": "pp1", "thickness_nm": 200000, "material": "prepreg"}},
          {{"kind": "copper", "name": "gnd", "thickness": {{"um": 35.0}}}},
          {{"kind": "core", "name": "core", "thickness_nm": 1000000, "material": "fr4"}},
          {{"kind": "copper", "name": "power", "thickness": {{"um": 35.0}}}},
          {{"kind": "prepreg", "name": "pp2", "thickness_nm": 200000, "material": "prepreg"}},
          {{"kind": "copper", "name": "bottom", "thickness": {{"um": 35.0}}}}]}},
          {MATERIALS}}}"#
    )
}

fn env_from(text: &str, freq_ghz: f64, settings: RfgenSettings) -> Environment {
    let (stackup, materials) = parse_stackup_file(text).unwrap();
    Environment::new(
        &stackup,
        &materials,
        None,
        freq_ghz,
        settings,
        StackupSettings::default(),
    )
    .unwrap()
}

fn env(freq_ghz: f64) -> Environment {
    env_from(&two_layer_text(), freq_ghz, RfgenSettings::default())
}

fn source(seed: u64) -> impl FnMut() -> Id {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(seed),
    );
    move || generator.generate().unwrap()
}

fn make(env: &Environment, kind: Kind, params: &Params) -> Result<(Board, Generated), RfgenError> {
    let mut next = source(5);
    let board = new_board(env, "book", &mut next)?;
    let generated = generate(env, &board, &Request::new(kind, params.clone()), &mut next)?;
    Ok((board, generated))
}

fn value(generated: &Generated, name: &str) -> f64 {
    generated
        .report
        .value(name)
        .unwrap_or_else(|| panic!("no dimension {name}"))
}

/// Every type with parameters that suit a two-layer board of 1.6 mm FR-4, and the frequency.
fn cases() -> Vec<(Kind, Params, f64)> {
    vec![
        (Kind::Line, Params::new(), 1.0),
        (Kind::Bend, Params::new(), 1.0),
        (Kind::Bend, Params::new().with("turn", "right"), 2.0),
        (Kind::Curve, Params::new(), 1.0),
        (Kind::Curve, Params::new().with("turn", "right"), 1.0),
        (Kind::Taper, Params::new().with("z0_b", "100"), 1.0),
        (Kind::StubOpen, Params::new(), 1.0),
        (Kind::StubShort, Params::new(), 1.0),
        (Kind::StubMatch, Params::new().with("load_re", "100"), 1.0),
        (
            Kind::StubMatch,
            Params::new()
                .with("load_re", "30")
                .with("load_im", "20")
                .with("termination", "short"),
            1.0,
        ),
        (Kind::QuarterWave, Params::new().with("z2", "150"), 1.0),
        (Kind::Lowpass, Params::new(), 2.0),
        (
            Kind::Lowpass,
            Params::new().with("family", "chebyshev").with("order", "3"),
            1.5,
        ),
        (Kind::Bandpass, Params::new(), 2.4),
        (Kind::Bandpass, Params::new().with("order", "2"), 2.4),
        (Kind::Wilkinson, Params::new(), 1.0),
        (Kind::BranchLine, Params::new(), 1.0),
        (Kind::Patch, Params::new(), 2.4),
        (Kind::Dipole, Params::new(), 2.4),
        (Kind::Sma, Params::new(), 2.4),
    ]
}

#[test]
fn the_fifty_ohm_line_on_fr4_is_about_three_millimetres_wide_and_has_fifty_ohm() {
    let e = env(1.0);
    let (_, g) = make(&e, Kind::Line, &Params::new()).unwrap();
    let width = value(&g, "line width");
    assert!((2.8..=3.2).contains(&width), "{width}");
    // the impedance of the width that is drawn, computed again by the stack-up crate
    let (stackup, materials) = parse_stackup_file(&two_layer_text()).unwrap();
    let spec = LineSpec {
        name: "check".to_string(),
        kind: LineKind::Microstrip,
        layer: "top".to_string(),
        width_nm: None,
        target_ohm: None,
        gap_nm: None,
    };
    let width_nm = (width * 1.0e6).round() as i64;
    let result = analyze(
        &stackup,
        &materials,
        &spec,
        width_nm,
        1.0,
        &StackupSettings::default(),
    )
    .unwrap();
    assert!(
        (result.impedance_ohm - 50.0).abs() < 0.05,
        "{}",
        result.impedance_ohm
    );
}

#[test]
fn a_quarter_wave_at_one_gigahertz_is_the_guided_wavelength_over_four() {
    let e = env(1.0);
    let (_, g) = make(&e, Kind::Line, &Params::new()).unwrap();
    let lambda = value(&g, "line guided wavelength");
    let length = value(&g, "length");
    assert!((length - lambda / 4.0).abs() < 1.0e-5, "{length} {lambda}");
    // about 41 mm on 1.6 mm of FR-4
    assert!((39.0..43.0).contains(&length), "{length}");
    assert!((value(&g, "electrical length") - 90.0).abs() < 1.0e-3);
}

#[test]
fn the_wilkinson_arms_are_a_quarter_wave_of_the_square_root_of_two_times_the_impedance() {
    let e = env(1.0);
    let (_, g) = make(&e, Kind::Wilkinson, &Params::new()).unwrap();
    assert!((value(&g, "arm impedance wanted") - 50.0 * 2.0_f64.sqrt()).abs() < 1.0e-9);
    assert!((value(&g, "arm impedance") - 70.71).abs() < 0.1);
    assert!((value(&g, "isolation resistor") - 100.0).abs() < 1.0e-9);
    let (stackup, materials) = parse_stackup_file(&two_layer_text()).unwrap();
    let spec = LineSpec {
        name: "arm".to_string(),
        kind: LineKind::Microstrip,
        layer: "top".to_string(),
        width_nm: None,
        target_ohm: None,
        gap_nm: None,
    };
    let width_nm = (value(&g, "arm width") * 1.0e6).round() as i64;
    let arm = analyze(
        &stackup,
        &materials,
        &spec,
        width_nm,
        1.0,
        &StackupSettings::default(),
    )
    .unwrap();
    let quarter = arm.guided_wavelength_mm / 4.0;
    assert!(
        (value(&g, "arm quarter wave") - quarter).abs() < 1.0e-4,
        "{quarter}"
    );
}

#[test]
fn the_patch_at_two_point_four_gigahertz_on_fr4_is_about_thirty_eight_by_twenty_nine_millimetres() {
    let e = env(2.4);
    let (_, g) = make(&e, Kind::Patch, &Params::new()).unwrap();
    assert!((value(&g, "patch width") - 38.0).abs() < 0.5);
    assert!((value(&g, "patch length") - 29.4).abs() < 0.5);
    assert!((value(&g, "patch eps_eff") - 4.09).abs() < 0.01);
    // 50 ohm feed at an inset of about nine millimetres
    assert!((value(&g, "inset depth") - 9.2).abs() < 0.3);
}

#[test]
fn the_book_band_pass_filter_has_four_sections_with_the_published_impedances() {
    let e = env(2.4);
    let (_, g) = make(&e, Kind::Bandpass, &Params::new()).unwrap();
    assert!((value(&g, "section 1 even impedance") - 70.61).abs() < 0.05);
    assert!((value(&g, "section 1 odd impedance") - 39.24).abs() < 0.05);
    assert!((value(&g, "section 2 even impedance") - 56.64).abs() < 0.05);
    assert!((value(&g, "section 2 odd impedance") - 44.77).abs() < 0.05);
    assert!((value(&g, "section 4 even impedance") - 70.61).abs() < 0.05);
    assert_eq!(g.report.counts.nets_added, 5);
}

#[test]
fn the_book_low_pass_filter_has_six_sections_of_the_published_lengths() {
    let e = env(2.5);
    let (_, g) = make(&e, Kind::Lowpass, &Params::new().with("order", "6")).unwrap();
    let want = [11.86, 33.76, 44.27, 46.12, 32.41, 12.36];
    for (k, degrees) in want.iter().enumerate() {
        let got = value(&g, &format!("section {} electrical length", k + 1));
        assert!((got - degrees).abs() < 0.05, "{k}: {got}");
    }
}

#[test]
fn the_via_fence_is_a_twentieth_of_the_wavelength_apart_and_the_distance_is_configurable() {
    let e = env(2.4);
    let (_, g) = make(&e, Kind::Sma, &Params::new()).unwrap();
    let lambda = value(&g, "line guided wavelength");
    assert!((value(&g, "via spacing") - lambda / 20.0).abs() < 1.0e-5);
    let settings = RfgenSettings {
        fence_spacing_wavelengths: 0.1,
        ..RfgenSettings::default()
    };
    let e = env_from(&two_layer_text(), 2.4, settings);
    let (_, wide) = make(&e, Kind::Sma, &Params::new()).unwrap();
    assert!((value(&wide, "via spacing") - lambda / 10.0).abs() < 1.0e-5);
    assert!(value(&wide, "vias per side") < value(&g, "vias per side"));
}

#[test]
fn every_structure_passes_the_rules_leaves_a_consistent_board_and_undoes_exactly() {
    for (kind, params, freq) in cases() {
        let e = env(freq);
        let (board, generated) = make(&e, kind, &params)
            .unwrap_or_else(|error| panic!("{} {params:?}: {error}", kind.name()));
        let name = kind.name();
        let rules = generated.report.rules.as_ref().unwrap();
        assert!(rules.passes, "{name}: {:?}", rules.findings);
        assert_eq!(rules.errors_after, 0, "{name}");
        assert!(
            generated.board.is_consistent(),
            "{name}: {:?}",
            generated.board.validate()
        );
        assert!(!generated.report.ports.is_empty(), "{name}");
        assert!(!generated.report.formulas.is_empty(), "{name}");
        // the board was not touched, the operation puts the structure on, its inverse takes it off
        let mut copy = board.clone();
        let inverse = generated.apply(&mut copy).unwrap();
        assert_eq!(copy, generated.board, "{name}");
        assert_ne!(copy, board, "{name}");
        copy.apply(&inverse).unwrap();
        assert_eq!(copy, board, "{name}: undo");
        assert_eq!(
            copy.to_bytes().unwrap(),
            board.to_bytes().unwrap(),
            "{name}: undo bytes"
        );
    }
}

#[test]
fn the_same_request_gives_the_same_board_report_and_bytes() {
    for (kind, params, freq) in cases() {
        let e = env(freq);
        let (_, first) = make(&e, kind, &params).unwrap();
        let (_, second) = make(&e, kind, &params).unwrap();
        assert_eq!(
            first.board.to_bytes().unwrap(),
            second.board.to_bytes().unwrap(),
            "{}",
            kind.name()
        );
        assert_eq!(first.report.to_json(), second.report.to_json());
        assert_eq!(
            first.report.to_text(eh3d_phoenix_rfgen::Lang::En),
            second.report.to_text(eh3d_phoenix_rfgen::Lang::En)
        );
    }
}

#[test]
fn the_structures_that_fit_a_thin_four_layer_board_pass_the_rules_too() {
    let e = env_from(&four_layer_text(), 2.4, RfgenSettings::default());
    for (kind, params) in [
        (Kind::Line, Params::new()),
        (Kind::Bend, Params::new()),
        (Kind::Curve, Params::new()),
        (Kind::Taper, Params::new().with("z0_b", "80")),
        (Kind::StubOpen, Params::new()),
        (Kind::StubShort, Params::new()),
        (Kind::QuarterWave, Params::new().with("z2", "100")),
        (Kind::Wilkinson, Params::new()),
        (Kind::BranchLine, Params::new()),
    ] {
        let (board, generated) = make(&e, kind, &params)
            .unwrap_or_else(|error| panic!("{} {params:?}: {error}", kind.name()));
        let rules = generated.report.rules.as_ref().unwrap();
        assert!(rules.passes, "{}: {:?}", kind.name(), rules.findings);
        // the layers are those of the stack-up, the plane is the second copper layer
        assert_eq!(board.layers.len(), 4);
        assert_eq!(e.substrate.plane_layer, "gnd");
    }
}

#[test]
fn a_structure_can_be_put_anywhere_and_turned_by_a_whole_quarter() {
    let e = env(1.0);
    let mut next = source(9);
    let board = new_board(&e, "book", &mut next).unwrap();
    let mut placed = Request::new(Kind::Line, Params::new().with("length_mm", "20"));
    placed.placement = Placement::new(10_000_000, -5_000_000, 90).unwrap();
    let generated = generate(&e, &board, &placed, &mut next).unwrap();
    let ports = &generated.report.ports;
    assert_eq!((ports[0].x_nm, ports[0].y_nm), (10_000_000, -5_000_000));
    // 20 mm along the turned x axis is 20 mm along y
    assert_eq!((ports[1].x_nm, ports[1].y_nm), (10_000_000, 15_000_000));
    assert_eq!(ports[0].angle_deg, 270);
    assert_eq!(ports[1].angle_deg, 90);
    assert!(Placement::new(0, 0, 30).is_err());
    let zone = generated.board.zones.values().next().unwrap();
    assert!(zone.outline.iter().all(|p| p.y_nm.get() >= -5_000_000));
}

#[test]
fn two_structures_on_one_board_and_the_second_one_keeps_the_nets_it_finds() {
    let e = env(1.0);
    let mut next = source(3);
    let board = new_board(&e, "book", &mut next).unwrap();
    let first = generate(
        &e,
        &board,
        &Request::new(Kind::Line, Params::new()),
        &mut next,
    )
    .unwrap();
    let mut second_request = Request::new(Kind::Bend, Params::new());
    second_request.placement = Placement::new(0, 30_000_000, 0).unwrap();
    let second = generate(&e, &first.board, &second_request, &mut next).unwrap();
    assert_eq!(second.board.zones.len(), 2);
    assert_eq!(second.board.nets.len(), 1);
    assert_eq!(second.report.counts.nets_added, 0);
    assert!(second.board.is_consistent());
    // a request with another net name adds a net
    let mut third_request = Request::new(Kind::Line, Params::new());
    third_request.nets.signal = "LO".to_string();
    third_request.placement = Placement::new(0, -30_000_000, 0).unwrap();
    let third = generate(&e, &second.board, &third_request, &mut next).unwrap();
    assert_eq!(third.board.nets.len(), 2);
}

#[test]
fn the_errors_have_their_codes() {
    let e = env(1.0);
    let bad = |kind: Kind, params: Params| make(&e, kind, &params).unwrap_err().code;
    assert_eq!(
        bad(Kind::Line, Params::new().with("speed", "3")),
        PARAMETER_INVALID
    );
    assert_eq!(bad(Kind::Taper, Params::new()), PARAMETER_INVALID);
    assert_eq!(
        bad(Kind::QuarterWave, Params::new().with("z2", "50")),
        NOT_REALISABLE
    );
    assert_eq!(
        bad(Kind::Bend, Params::new().with("turn", "up")),
        PARAMETER_INVALID
    );
    assert_eq!(Kind::from_name("amplifier").unwrap_err().code, KIND_UNKNOWN);
    // a board whose layers are not those of the stack-up
    let mut next = source(2);
    let mut board = new_board(&e, "book", &mut next).unwrap();
    board.layers.clear();
    let error = generate(
        &e,
        &board,
        &Request::new(Kind::Line, Params::new()),
        &mut next,
    )
    .unwrap_err();
    assert_eq!(error.code, LAYER_NOT_FOUND);
}

#[test]
fn the_report_is_available_as_text_in_every_language_and_as_json() {
    let e = env(1.0);
    let (_, g) = make(&e, Kind::StubShort, &Params::new()).unwrap();
    for lang in eh3d_phoenix_rfgen::Lang::ALL {
        let text = g.report.to_text(lang);
        assert!(text.contains("stub-short"), "{lang:?}");
        assert!(
            text.contains("Hammerstad") || text.contains("Pozar"),
            "{lang:?}"
        );
    }
    let json = g.report.to_json();
    assert_eq!(json["state"], "prototype");
    assert_eq!(json["kind"], "stub-short");
    assert_eq!(json["rules"]["passes"], true);
    assert!(json["formulas"].as_array().unwrap().len() >= 2);
}
