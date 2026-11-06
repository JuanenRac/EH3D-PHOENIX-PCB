// SPDX-License-Identifier: GPL-3.0-or-later
//! The signal integrity estimates on small boards made by hand, with the numbers worked out on
//! paper from the formulas of the crate.

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track, Via, Zone};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_si::codes::{
    COUPLING_STRONG, FEXT_HIGH, LONG_LINE, NET_NOT_FOUND, NEXT_HIGH, PLANE_SPLIT, STACKUP_FALLBACK,
    STUB_LONG,
};
use eh3d_phoenix_si::{
    analyse, report_json, report_text, Analysis, Severity, SiReport, SiSettings,
};
use eh3d_phoenix_stackup::StackupJob;

const MM: i64 = 1_000_000;
const TOP: u8 = 1;
const BOTTOM: u8 = 2;
const NET_A: u8 = 3;
const NET_B: u8 = 4;
const GROUND: u8 = 5;

fn id(n: u8) -> Id {
    format!("01890a5d-ac96-7ab2-80e2-4536629c13{n:02x}")
        .parse()
        .unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn base_board() -> Board {
    let mut board = Board {
        name: "si".to_owned(),
        ..Board::default()
    };
    for (n, name) in [(TOP, "F.Cu"), (BOTTOM, "B.Cu")] {
        board.layers.insert(
            id(n),
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    board.copper_stack = vec![id(TOP), id(BOTTOM)];
    for (n, name) in [(NET_A, "A"), (NET_B, "B"), (GROUND, "GND")] {
        board.nets.insert(
            id(n),
            Net {
                name: name.to_owned(),
            },
        );
    }
    board
}

fn add_track(board: &mut Board, n: u8, net: u8, layer: u8, from: (i64, i64), to: (i64, i64)) {
    board.tracks.insert(
        id(n),
        Track {
            net: id(net),
            layer: id(layer),
            width: Nm::new(200_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(from.0, from.1),
                end: pt(to.0, to.1),
            }],
        },
    );
}

/// Two tracks of 20 mm, 0.4 mm apart between centres.
fn parallel_board(second_y: i64) -> Board {
    let mut board = base_board();
    add_track(&mut board, 10, NET_A, TOP, (0, 0), (20 * MM, 0));
    add_track(
        &mut board,
        11,
        NET_B,
        TOP,
        (0, second_y),
        (20 * MM, second_y),
    );
    board
}

fn run(board: &Board, settings: &SiSettings, what: Analysis, net: Option<&str>) -> SiReport {
    analyse(board, settings, None, what, net).unwrap()
}

#[test]
fn two_parallel_tracks_couple_by_the_numbers_of_the_formulas() {
    let report = run(
        &parallel_board(400_000),
        &SiSettings::default(),
        Analysis::Crosstalk,
        None,
    );
    let c = report.crosstalk.as_ref().unwrap();
    assert_eq!(c.rows.len(), 2);
    let a = &c.rows[0];
    assert_eq!((a.aggressor.as_str(), a.victim.as_str()), ("A", "B"));
    assert_eq!(a.layer, "F.Cu");
    assert!((a.parallel_mm - 20.0).abs() < 1e-9);
    assert!((a.distance_um - 400.0).abs() < 1e-6);
    assert!((a.gap_um - 200.0).abs() < 1e-6);
    // Lm/L = 1e-7 ln 2 / (50 sqrt 3 / c) = 0.2399; Cm/C = 0.2399 * 3 / 4.3 = 0.1674
    assert!((a.lm_over_l - 0.2399).abs() < 1e-3, "{}", a.lm_over_l);
    assert!((a.cm_over_c - 0.1674).abs() < 1e-3, "{}", a.cm_over_c);
    // Td = 20 mm * 5.7774 ps/mm = 115.5 ps; NEXT = 100 * 0.1018 * (2 Td / tr) = 4.71 %
    assert!((a.next_pct - 4.707).abs() < 0.01, "{}", a.next_pct);
    // FEXT = 100 * 0.5 * (0.1674 - 0.2399) * Td / tr = -0.838 %
    assert!((a.fext_pct + 0.838).abs() < 0.01, "{}", a.fext_pct);
    assert!(!a.strong);
    // under both limits: nothing is found but the note on the stack-up
    assert!(report
        .findings
        .iter()
        .all(|f| f.severity == Severity::Info && f.code == STACKUP_FALLBACK));
    assert_eq!(c.victims.len(), 2);
    assert!((c.victims[0].next_sum_pct - 4.707).abs() < 0.01);
}

#[test]
fn a_faster_edge_crosses_the_limits() {
    let mut settings = SiSettings::default();
    settings.net_rise_ps.insert("A".to_owned(), 300.0);
    settings.net_rise_ps.insert("B".to_owned(), 200.0);
    let report = run(
        &parallel_board(400_000),
        &settings,
        Analysis::Crosstalk,
        None,
    );
    // A at 300 ps: 2 Td / tr = 0.77, NEXT 7.8 % is a warning; B at 200 ps saturates at 10.18 %
    let next: Vec<(&str, Severity)> = report
        .findings
        .iter()
        .filter(|f| f.code == NEXT_HIGH)
        .map(|f| (f.subject.as_deref().unwrap(), f.severity))
        .collect();
    assert_eq!(
        next,
        vec![("A/B", Severity::Warning), ("B/A", Severity::Error)]
    );
    assert!(report.has_errors());
    assert!(report.findings.iter().all(|f| f.code != FEXT_HIGH));
}

#[test]
fn tracks_too_far_apart_or_on_another_layer_do_not_couple() {
    let far = run(
        &parallel_board(3 * MM),
        &SiSettings::default(),
        Analysis::Crosstalk,
        None,
    );
    assert!(far.crosstalk.as_ref().unwrap().rows.is_empty());
    let mut board = parallel_board(400_000);
    add_track(
        &mut board,
        12,
        NET_B,
        BOTTOM,
        (0, 400_000),
        (20 * MM, 400_000),
    );
    board.tracks.remove(&id(11));
    let other = run(&board, &SiSettings::default(), Analysis::Crosstalk, None);
    assert!(other.crosstalk.as_ref().unwrap().rows.is_empty());
}

#[test]
fn a_net_can_be_chosen_and_an_unknown_one_is_refused() {
    let board = parallel_board(400_000);
    let one = run(
        &board,
        &SiSettings::default(),
        Analysis::Crosstalk,
        Some("B"),
    );
    let c = one.crosstalk.as_ref().unwrap();
    assert_eq!(c.rows.len(), 1);
    assert_eq!(c.rows[0].aggressor, "B");
    assert_eq!(c.victims.len(), 1);
    assert_eq!(c.victims[0].victim, "B");
    let error = analyse(
        &board,
        &SiSettings::default(),
        None,
        Analysis::Crosstalk,
        Some("MISSING"),
    )
    .unwrap_err();
    assert_eq!(error.code, NET_NOT_FOUND);
}

const TWO_LAYER_JOB: &str = r#"{
  "stackup": {
    "name": "two-layer",
    "layers": [
      {"kind": "copper", "name": "F.Cu", "thickness": {"oz": 1.0}},
      {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
      {"kind": "copper", "name": "B.Cu", "thickness": {"um": 35.0}}
    ]
  },
  "materials": [
    {"name": "fr4", "dk_points": [[1.0, 4.5], [10.0, 4.3]], "df_points": [[1.0, 0.02], [10.0, 0.022]]}
  ],
  "lines": [
    {"name": "x", "kind": "microstrip", "layer": "F.Cu", "width_nm": 200000}
  ]
}"#;

#[test]
fn a_thick_board_couples_more_than_the_model_trusts() {
    let job = StackupJob::parse(TWO_LAYER_JOB).unwrap();
    let report = analyse(
        &parallel_board(400_000),
        &SiSettings::default(),
        Some(&job),
        Analysis::Crosstalk,
        None,
    )
    .unwrap();
    let rows = &report.crosstalk.as_ref().unwrap().rows;
    // 1.6 mm over the plane against 0.4 mm between centres: the tracks are far from the plane
    assert!(rows[0].strong);
    assert!(rows[0].lm_over_l > 0.3 * 0.95, "{}", rows[0].lm_over_l);
    assert!(report.findings.iter().any(|f| f.code == COUPLING_STRONG));
    // the stack-up was used: there is no note about the defaults
    assert!(report.findings.iter().all(|f| f.code != STACKUP_FALLBACK));
}

fn long_line_board() -> Board {
    let mut board = base_board();
    add_track(&mut board, 10, NET_A, TOP, (0, 0), (120 * MM, 0));
    board
}

#[test]
fn a_long_net_is_simulated_and_terminations_are_proposed() {
    let report = run(
        &long_line_board(),
        &SiSettings::default(),
        Analysis::Reflect,
        None,
    );
    assert_eq!(report.nets.len(), 1);
    let n = &report.nets[0];
    assert_eq!(n.net, "A");
    assert!((n.length_mm - 120.0).abs() < 1e-9);
    // 120 mm * 5.7774 ps/mm
    assert!((n.td_ps - 693.28).abs() < 0.05, "{}", n.td_ps);
    assert!(n.long_line);
    assert!((n.bandwidth_ghz - 0.7).abs() < 1e-9);
    assert_eq!(n.proposals.len(), 4);
    assert_eq!(n.proposals[0].name, "series");
    // 25 ohm driver on 50 ohm: the series part is 25 ohm
    assert!(
        n.proposals[0].values.contains("R=25.0"),
        "{}",
        n.proposals[0].values
    );
    assert!(n.baseline.is_some());
    assert_eq!(n.bounces.len(), 16);
    assert!(report.findings.iter().any(|f| f.code == LONG_LINE));
    assert!(report.crosstalk.is_none() && report.eyes.is_empty());
}

#[test]
fn a_short_net_needs_no_simulation() {
    let mut board = base_board();
    add_track(&mut board, 10, NET_A, TOP, (0, 0), (10 * MM, 0));
    let report = run(&board, &SiSettings::default(), Analysis::Reflect, None);
    assert!(!report.nets[0].long_line);
    assert!(report.nets[0].baseline.is_none());
    assert!(report.nets[0].proposals.is_empty());
}

#[test]
fn the_eye_of_a_net_is_measured_and_the_same_every_time() {
    let board = long_line_board();
    let first = run(&board, &SiSettings::default(), Analysis::Eye, None);
    let second = run(&board, &SiSettings::default(), Analysis::Eye, None);
    assert_eq!(first, second);
    assert_eq!(first.eyes.len(), 1);
    let e = &first.eyes[0];
    assert_eq!(e.bits, 127);
    assert!(e.height > 0.9 && e.height <= 1.0, "{}", e.height);
    assert!(e.width > 0.9 && e.width <= 1.0, "{}", e.width);
}

#[test]
fn a_track_that_leaves_the_plane_under_it_is_found() {
    let mut board = base_board();
    // a ground zone on the bottom layer from x = 0 to 20 mm
    board.zones.insert(
        id(30),
        Zone {
            net: id(GROUND),
            layer: id(BOTTOM),
            outline: vec![
                pt(0, -5 * MM),
                pt(20 * MM, -5 * MM),
                pt(20 * MM, 5 * MM),
                pt(0, 5 * MM),
            ],
            clearance: Nm::new(200_000).unwrap(),
            cutouts: Vec::new(),
        },
    );
    // A leaves the zone, B stays over it
    add_track(&mut board, 10, NET_A, TOP, (5 * MM, 0), (35 * MM, 0));
    add_track(
        &mut board,
        11,
        NET_B,
        TOP,
        (2 * MM, 2 * MM),
        (10 * MM, 2 * MM),
    );
    let report = run(&board, &SiSettings::default(), Analysis::Check, None);
    let returns = report.returns.as_ref().unwrap();
    assert_eq!(returns.tracks_checked, 2);
    assert_eq!(returns.crossings.len(), 1);
    let c = &returns.crossings[0];
    assert_eq!((c.net.as_str(), c.layer.as_str()), ("A", "F.Cu"));
    assert_eq!(c.reference, "B.Cu");
    assert_eq!(c.changes, 1);
    assert!(!c.over_nothing);
    // the samples are 0.5 mm apart and the one at 20 mm, on the edge of the zone, is outside it
    assert!((c.x_mm - 20.0).abs() < 1e-9, "{}", c.x_mm);
    let splits: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.code == PLANE_SPLIT)
        .collect();
    assert_eq!(splits.len(), 1);
    assert_eq!(splits[0].subject.as_deref(), Some("A"));
    // with a net chosen only its tracks are reported
    let only = run(&board, &SiSettings::default(), Analysis::Check, Some("B"));
    assert!(only.returns.as_ref().unwrap().crossings.is_empty());
}

#[test]
fn a_via_that_goes_on_past_the_last_layer_of_the_signal_leaves_a_stub() {
    let mut board = base_board();
    add_track(&mut board, 10, NET_A, TOP, (0, 0), (10 * MM, 0));
    board.vias.insert(
        id(40),
        Via {
            net: id(NET_A),
            at: pt(10 * MM, 0),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(TOP),
            to_layer: id(BOTTOM),
        },
    );
    let report = run(&board, &SiSettings::default(), Analysis::Check, None);
    let stubs = &report.returns.as_ref().unwrap().stubs;
    assert_eq!(stubs.len(), 1);
    assert_eq!(stubs[0].side, "bottom");
    // the whole barrel of the default length
    assert!((stubs[0].length_mm - 1.6).abs() < 1e-9);
    assert!(report.findings.iter().any(|f| f.code == STUB_LONG));
    // a track on the bottom layer too uses the whole via
    add_track(
        &mut board,
        11,
        NET_A,
        BOTTOM,
        (10 * MM, 0),
        (10 * MM, 8 * MM),
    );
    let both = run(&board, &SiSettings::default(), Analysis::Check, None);
    assert!(both.returns.as_ref().unwrap().stubs.is_empty());
}

#[test]
fn the_text_and_the_json_name_every_part_of_the_report() {
    let mut board = parallel_board(400_000);
    add_track(&mut board, 12, NET_A, TOP, (0, -5 * MM), (60 * MM, -5 * MM));
    let report = run(&board, &SiSettings::default(), Analysis::Check, None);
    let text = report_text(&report, Lang::Es);
    for part in [
        "signal integrity check",
        "crosstalk: ",
        "net A (F.Cu)",
        "eye A: ",
        "return path: ",
        " errors, ",
    ] {
        assert!(text.contains(part), "{part} is missing in\n{text}");
    }
    let json = report_json(&report);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    for key in [
        "command",
        "state",
        "crosstalk",
        "nets",
        "eyes",
        "returns",
        "findings",
    ] {
        assert!(value.get(key).is_some(), "{key}");
    }
    assert_eq!(report_json(&report), json);
}

#[test]
fn bad_settings_are_refused() {
    let settings = SiSettings {
        prbs: 8,
        ..SiSettings::default()
    };
    let error = analyse(&base_board(), &settings, None, Analysis::Eye, None).unwrap_err();
    assert_eq!(error.code, "PHOENIX-SI-001");
}
