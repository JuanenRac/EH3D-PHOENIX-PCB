// SPDX-License-Identifier: GPL-3.0-or-later
//! Lengths, groups and pairs on small boards made by hand, with the numbers worked out on paper.

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track, Via};
use eh3d_phoenix_lengths::codes::{
    ARC_ESTIMATED, GROUP_MISMATCH, GROUP_TARGET, LAYER_NO_DELAY, NET_NOT_FOUND, NET_NO_COPPER,
    PAIR_GAP_SMALL, PAIR_NOT_COUPLED, PAIR_SKEW, PAIR_UNCOUPLED, PAIR_WIDTH, STACKUP_FALLBACK,
};
use eh3d_phoenix_lengths::{
    analyse, report_json, report_text, LengthFile, LengthReport, LengthSettings,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

const MM: i64 = 1_000_000;
const TOP: u8 = 1;
const BOTTOM: u8 = 2;
const NET_P: u8 = 3;
const NET_N: u8 = 4;
const NET_A: u8 = 5;
const NET_B: u8 = 6;

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
        name: "pairs".to_owned(),
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
    for (n, name) in [
        (NET_P, "USB_P"),
        (NET_N, "USB_N"),
        (NET_A, "D0"),
        (NET_B, "D1"),
    ] {
        board.nets.insert(
            id(n),
            Net {
                name: name.to_owned(),
            },
        );
    }
    board
}

fn line(start: (i64, i64), end: (i64, i64)) -> Segment {
    Segment::Line {
        start: pt(start.0, start.1),
        end: pt(end.0, end.1),
    }
}

fn add_track(board: &mut Board, n: u8, net: u8, width: i64, path: Vec<Segment>) {
    board.tracks.insert(
        id(n),
        Track {
            net: id(net),
            layer: id(TOP),
            width: Nm::new(width).unwrap(),
            path,
        },
    );
}

fn add_through_via(board: &mut Board, n: u8, net: u8, at: (i64, i64)) {
    board.vias.insert(
        id(n),
        Via {
            net: id(net),
            at: pt(at.0, at.1),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(TOP),
            to_layer: id(BOTTOM),
        },
    );
}

fn run(board: &Board, file: Option<&str>) -> LengthReport {
    let file = file.map(|t| LengthFile::from_json(t).unwrap());
    analyse(board, &LengthSettings::default(), None, file.as_ref()).unwrap()
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
    {"name": "x", "kind": "microstrip", "layer": "F.Cu", "width_nm": 150000}
  ]
}"#;

#[test]
fn the_length_of_a_net_is_its_tracks_and_its_vias_and_the_delay_follows() {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_P,
        150_000,
        vec![
            line((0, 0), (10 * MM, 0)),
            line((10 * MM, 0), (10 * MM, 5 * MM)),
        ],
    );
    add_through_via(&mut board, 20, NET_P, (10 * MM, 5 * MM));
    let report = run(&board, None);
    assert_eq!(report.nets.len(), 1);
    let net = &report.nets[0];
    assert_eq!(net.name, "USB_P");
    assert_eq!(net.tracks_nm, 15_000_000);
    assert_eq!(net.vias, 1);
    assert_eq!(net.vias_nm, 1_600_000);
    assert_eq!(net.total_nm, 16_600_000);
    // 15 mm at 6 ps/mm is 90 000 fs, a via of 1.6 mm at 6.9 ps/mm is 11 040 fs
    assert_eq!(net.delay_fs, 101_040);
    assert_eq!(net.layers.len(), 1);
    assert_eq!(net.layers[0].layer, "F.Cu");
    assert_eq!(net.layers[0].length_nm, 15_000_000);
    assert_eq!(net.layers[0].delay_fs, 90_000);
    assert!(report.findings.iter().any(|f| f.code == LAYER_NO_DELAY));
}

#[test]
fn the_delay_of_a_layer_can_be_set_by_name() {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_P,
        150_000,
        vec![line((0, 0), (15 * MM, 0))],
    );
    let settings = LengthSettings::parse("layer_delay_ps_per_mm.F.Cu = 5.0\n").unwrap();
    let report = analyse(&board, &settings, None, None).unwrap();
    assert_eq!(report.nets[0].delay_fs, 75_000);
    assert!(!report.findings.iter().any(|f| f.code == LAYER_NO_DELAY));
}

#[test]
fn the_delay_and_the_via_come_from_the_stackup_when_one_is_given() {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_P,
        150_000,
        vec![line((0, 0), (10 * MM, 0))],
    );
    add_through_via(&mut board, 20, NET_N, (0, 0));
    let job = StackupJob::parse(TWO_LAYER_JOB).unwrap();
    let report = analyse(&board, &LengthSettings::default(), Some(&job), None).unwrap();
    let p = report.nets.iter().find(|n| n.name == "USB_P").unwrap();
    // a microstrip of 0.15 mm on 1.6 mm of laminate is near 5.7 ps/mm, not the 6 of the default
    assert!(p.delay_fs > 50_000 && p.delay_fs < 65_000, "{}", p.delay_fs);
    assert_ne!(p.delay_fs, 60_000);
    // the via runs from the middle of the top copper to the middle of the bottom copper
    let n = report.nets.iter().find(|n| n.name == "USB_N").unwrap();
    assert_eq!(n.vias_nm, 17_399 + 1_600_000 + 17_500);
    assert_eq!(n.delay_fs, 11_281);
    assert!(!report.findings.iter().any(|f| f.code == STACKUP_FALLBACK));
}

#[test]
fn a_layer_the_stackup_does_not_name_falls_back_to_the_default_with_a_warning() {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_P,
        150_000,
        vec![line((0, 0), (10 * MM, 0))],
    );
    let text = TWO_LAYER_JOB
        .replace("F.Cu", "top")
        .replace("B.Cu", "bottom");
    let job = StackupJob::parse(&text).unwrap();
    let report = analyse(&board, &LengthSettings::default(), Some(&job), None).unwrap();
    assert_eq!(report.nets[0].delay_fs, 60_000);
    assert!(report.findings.iter().any(|f| f.code == STACKUP_FALLBACK));
}

#[test]
fn an_arc_is_measured_and_reported_as_an_estimate() {
    let mut board = base_board();
    let arc = Segment::Arc {
        start: pt(-5 * MM, 0),
        end: pt(5 * MM, 0),
        sweep: Udeg::new(180_000_000).unwrap(),
    };
    add_track(&mut board, 10, NET_P, 150_000, vec![arc]);
    let report = run(&board, None);
    assert_eq!(report.nets[0].tracks_nm, 15_707_963);
    assert_eq!(report.nets[0].arcs, 1);
    assert!(report.findings.iter().any(|f| f.code == ARC_ESTIMATED));
}

fn data_board(second: i64) -> Board {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_A,
        150_000,
        vec![line((0, 0), (10 * MM, 0))],
    );
    add_track(
        &mut board,
        11,
        NET_B,
        150_000,
        vec![line((0, 2 * MM), (second, 2 * MM))],
    );
    board
}

const GROUP: &str = r#"{"format":"phoenix-lengths/1","groups":[{"name":"data","nets":["D0","D1"],"tolerance_nm":500000}]}"#;

#[test]
fn a_group_within_its_tolerance_is_ok_and_one_beyond_it_is_an_error() {
    let ok = run(&data_board(10_400_000), Some(GROUP));
    assert_eq!(ok.groups.len(), 1);
    let g = &ok.groups[0];
    assert_eq!(
        (g.min_nm, g.max_nm, g.spread_nm),
        (10_000_000, 10_400_000, 400_000)
    );
    assert!(g.ok);
    assert!(!ok.has_errors());
    let broken = run(&data_board(10_600_000), Some(GROUP));
    assert!(!broken.groups[0].ok);
    assert!(broken.findings.iter().any(|f| f.code == GROUP_MISMATCH));
    assert!(broken.has_errors());
}

#[test]
fn a_group_with_a_target_judges_each_net_against_it() {
    let text = r#"{"format":"phoenix-lengths/1","groups":[{"name":"data","nets":["D0","D1"],"tolerance_nm":300000,"target_nm":10000000}]}"#;
    let report = run(&data_board(10_400_000), Some(text));
    let targets: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.code == GROUP_TARGET)
        .collect();
    assert_eq!(targets.len(), 1);
    assert!(targets[0].detail.contains("D1"), "{}", targets[0].detail);
    assert!(!report.groups[0].ok);
}

#[test]
fn a_net_the_board_does_not_have_leaves_the_group_out() {
    let text = r#"{"format":"phoenix-lengths/1","groups":[{"name":"data","nets":["D0","D9"]}]}"#;
    let report = run(&data_board(10 * MM), Some(text));
    assert!(report.groups.is_empty());
    assert!(report.findings.iter().any(|f| f.code == NET_NOT_FOUND));
}

#[test]
fn a_net_without_copper_has_length_zero_and_a_warning() {
    let text = r#"{"format":"phoenix-lengths/1","groups":[{"name":"g","nets":["D0","USB_P"],"tolerance_nm":100}]}"#;
    let report = run(&data_board(10 * MM), Some(text));
    assert_eq!(report.groups[0].min_nm, 0);
    assert!(report.findings.iter().any(|f| f.code == NET_NO_COPPER));
    assert!(report.findings.iter().any(|f| f.code == GROUP_MISMATCH));
}

fn pair_board(gap_centre: i64, negative_width: i64, negative_end: i64) -> Board {
    let mut board = base_board();
    add_track(
        &mut board,
        10,
        NET_P,
        150_000,
        vec![line((0, 0), (10 * MM, 0))],
    );
    add_track(
        &mut board,
        11,
        NET_N,
        negative_width,
        vec![line((0, gap_centre), (negative_end, gap_centre))],
    );
    board
}

const PAIR: &str = r#"{"format":"phoenix-lengths/1","pairs":[{"name":"usb","positive":"USB_P","negative":"USB_N","max_skew_nm":200000,"gap_nm":200000,"width_nm":150000,"max_uncoupled_nm":1000000}]}"#;

#[test]
fn a_pair_that_follows_its_rules_is_ok() {
    // centre lines 350 000 nm apart and tracks 150 000 nm wide leave 200 000 nm between the edges
    let report = run(&pair_board(350_000, 150_000, 10 * MM), Some(PAIR));
    assert_eq!(report.pairs.len(), 1);
    let p = &report.pairs[0];
    assert_eq!(
        (p.positive_nm, p.negative_nm, p.skew_nm),
        (10_000_000, 10_000_000, 0)
    );
    assert_eq!(p.coupled_nm, 10_000_000);
    assert_eq!(p.uncoupled_nm, 0);
    assert_eq!((p.min_gap_nm, p.max_gap_nm), (Some(200_000), Some(200_000)));
    assert_eq!(p.delay_skew_fs, 0);
    assert!(p.ok);
    assert!(!report.has_errors(), "{:?}", report.findings);
}

#[test]
fn a_pair_that_is_too_close_too_unequal_or_the_wrong_width_is_an_error() {
    let close = run(&pair_board(250_000, 150_000, 10 * MM), Some(PAIR));
    assert!(close.findings.iter().any(|f| f.code == PAIR_GAP_SMALL));
    assert_eq!(close.pairs[0].min_gap_nm, Some(100_000));
    let skewed = run(&pair_board(350_000, 150_000, 10_300_000), Some(PAIR));
    assert!(skewed.findings.iter().any(|f| f.code == PAIR_SKEW));
    assert_eq!(skewed.pairs[0].skew_nm, 300_000);
    assert_eq!(skewed.pairs[0].delay_skew_fs, 1_800);
    let wide = run(&pair_board(350_000, 200_000, 10 * MM), Some(PAIR));
    assert!(wide.findings.iter().any(|f| f.code == PAIR_WIDTH));
    assert!(!wide.pairs[0].ok);
}

#[test]
fn a_pair_that_runs_apart_has_no_coupled_stretch_and_is_uncoupled() {
    let report = run(&pair_board(5 * MM, 150_000, 10 * MM), Some(PAIR));
    assert!(report.findings.iter().any(|f| f.code == PAIR_NOT_COUPLED));
    assert!(report.findings.iter().any(|f| f.code == PAIR_UNCOUPLED));
    assert_eq!(report.pairs[0].coupled_nm, 0);
    assert_eq!(report.pairs[0].uncoupled_nm, 10_000_000);
    assert_eq!(report.pairs[0].min_gap_nm, None);
}

#[test]
fn the_same_board_gives_the_same_bytes() {
    let board = pair_board(350_000, 150_000, 10 * MM);
    let first = run(&board, Some(PAIR));
    let second = run(&board, Some(PAIR));
    assert_eq!(first, second);
    assert_eq!(report_json(&first), report_json(&second));
    assert_eq!(
        report_text(&first, Lang::Es),
        report_text(&second, Lang::Es)
    );
    let text = report_text(&first, Lang::En);
    assert!(text.contains("net USB_N: 10.000000 mm"), "{text}");
    assert!(text.contains("pair usb:"), "{text}");
}
