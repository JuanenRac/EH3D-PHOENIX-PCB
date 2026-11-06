// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the differential pairs: the three operations of the editor and the checks, on the small
//! board of the fixture with the numbers worked out on paper.

use eh3d_phoenix_board::{Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

use crate::codes::{GEOMETRY_INVALID, OPERATION_REFUSED};
use crate::edit::BoardEdit;
use crate::ops::{EditOp, PairLayerChange};
use crate::pair_check::check_pairs;
use crate::pair_codes::{
    ASYMMETRIC, GAP_OUT, IMPEDANCE_OFF, LENGTH_MISMATCH, NET_MISSING, PAIR_OPERATION_REFUSED,
    VIAS_UNPAIRED, WIDTH_WRONG,
};
use crate::pairs::PairConfig;
use crate::settings::BoardEditSettings;
use crate::shape::{MeanderSide, Shape};
use crate::testutil::{fixture, id, strict_rules, Fixture};

const MM: i64 = 1_000_000;

/// A pair of SIG and GND with a track of 0.25 mm and a gap of 0.25 mm.
const PAIR: &str =
    "pair.p.positive = SIG\npair.p.negative = GND\npair.p.width_nm = 250000\npair.p.gap_nm = 250000\n";

fn editor(f: &Fixture, text: &str) -> BoardEdit {
    BoardEdit::new(
        f.board.clone(),
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
    .with_pairs(PairConfig::parse(text).unwrap())
}

fn p(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
    Segment::Line {
        start: p(a.0, a.1),
        end: p(b.0, b.1),
    }
}

/// A track of the ground net on the top layer of the fixture.
fn put_gnd(f: &mut Fixture, n: u16, width_nm: i64, path: Vec<Segment>) {
    f.board.tracks.insert(
        id(n),
        Track {
            net: f.gnd,
            layer: f.top,
            width: Nm::new(width_nm).unwrap(),
            path,
        },
    );
}

fn through_via(f: &mut Fixture, n: u16, signal: bool, at: Point) {
    f.board.vias.insert(
        id(n),
        Via {
            net: if signal { f.sig } else { f.gnd },
            at,
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: f.top,
            to_layer: f.bottom,
        },
    );
}

fn route_pair(f: &Fixture, points: Vec<Point>, changes: Vec<PairLayerChange>) -> EditOp {
    EditOp::RoutePair {
        pair: "p".to_owned(),
        layer: f.top,
        points,
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        gap_nm: None,
        horizontal_first: None,
        layer_changes: changes,
    }
}

#[test]
fn a_pair_is_drawn_side_by_side_with_the_width_and_the_gap_of_the_pair() {
    let f = fixture();
    let mut e = editor(&f, PAIR);
    let before = e.board().to_bytes().unwrap();
    let out = e
        .apply(&route_pair(
            &f,
            vec![f.pt(10, 20), f.pt(30, 20)],
            Vec::new(),
        ))
        .unwrap();
    assert!(out.changed && !out.drc.adds_error());
    assert_eq!(out.created.len(), 2);
    let positive = &e.board().tracks[&out.created[0]];
    let negative = &e.board().tracks[&out.created[1]];
    assert_eq!((positive.net, negative.net), (f.sig, f.gnd));
    assert_eq!(positive.width.get(), 250_000);
    assert_eq!(
        positive.path,
        vec![line((10 * MM, 20_250_000), (30 * MM, 20_250_000))]
    );
    assert_eq!(
        negative.path,
        vec![line((10 * MM, 19_750_000), (30 * MM, 19_750_000))]
    );
    assert_eq!(e.board().tracks.len(), 3);
    // one step, and undo gives back the same bytes
    assert_eq!(e.steps(), 1);
    assert!(e.undo());
    assert_eq!(e.board().to_bytes().unwrap(), before);
}

#[test]
fn a_pair_keeps_its_gap_round_a_corner_and_on_a_45_degree_piece() {
    let f = fixture();
    let mut e = editor(&f, PAIR);
    let op = EditOp::RoutePair {
        pair: "p".to_owned(),
        layer: f.top,
        points: vec![f.pt(10, 20), f.pt(20, 20), f.pt(25, 25)],
        shape: Some(Shape::Diagonal),
        width_nm: Some(200_000),
        gap_nm: Some(300_000),
        horizontal_first: None,
        layer_changes: Vec::new(),
    };
    let out = e.apply(&op).unwrap();
    let positive = &e.board().tracks[&out.created[0]];
    let negative = &e.board().tracks[&out.created[1]];
    assert_eq!(positive.width.get(), 200_000);
    assert_eq!(positive.path.len(), 2);
    assert_eq!(negative.path.len(), 2);
    // the first pieces are 250 000 nm to each side of the centre line
    assert_eq!(positive.path[0].start().y_nm.get(), 20 * MM + 250_000);
    assert_eq!(negative.path[0].start().y_nm.get(), 20 * MM - 250_000);
    // the corner pieces still meet
    assert_eq!(positive.path[0].end(), positive.path[1].start());
    assert_eq!(negative.path[0].end(), negative.path[1].start());
}

#[test]
fn a_pair_that_changes_layer_gets_a_via_for_each_net_and_the_second_one_is_moved_along() {
    let f = fixture();
    let text = "pair.p.positive = SIG\npair.p.negative = GND\npair.p.width_nm = 250000\npair.p.gap_nm = 1000000\n";
    let mut e = editor(&f, text);
    let change = PairLayerChange {
        vertex: 1,
        layer: f.bottom,
    };
    let out = e
        .apply(&route_pair(
            &f,
            vec![f.pt(10, 20), f.pt(20, 20), f.pt(30, 20)],
            vec![change],
        ))
        .unwrap();
    assert_eq!(out.created.len(), 6);
    assert_eq!(e.board().vias.len(), 2);
    let board = e.board();
    assert_eq!(
        board.tracks[&out.created[0]].path,
        vec![line((10 * MM, 20_625_000), (20 * MM, 20_625_000))]
    );
    assert_eq!(board.tracks[&out.created[1]].layer, f.bottom);
    assert_eq!(
        board.tracks[&out.created[1]].path,
        vec![line((20 * MM, 20_625_000), (30 * MM, 20_625_000))]
    );
    assert_eq!(
        board.tracks[&out.created[2]].path,
        vec![line((10 * MM, 19_375_000), (21 * MM, 19_375_000))]
    );
    let first = &board.vias[&out.created[4]];
    let second = &board.vias[&out.created[5]];
    assert_eq!((first.net, second.net), (f.sig, f.gnd));
    assert_eq!(first.at, p(20 * MM, 20_625_000));
    assert_eq!(second.at, p(21 * MM, 19_375_000));
    assert_eq!((first.from_layer, first.to_layer), (f.top, f.bottom));
}

#[test]
fn a_pair_that_cannot_be_drawn_is_refused_with_its_reason() {
    let f = fixture();
    let mut e = editor(&f, PAIR);
    let before = e.board().to_bytes().unwrap();
    let mut unknown = route_pair(&f, vec![f.pt(10, 20), f.pt(30, 20)], Vec::new());
    if let EditOp::RoutePair { pair, .. } = &mut unknown {
        *pair = "nope".to_owned();
    }
    assert_eq!(e.apply(&unknown).unwrap_err().code, PAIR_OPERATION_REFUSED);
    let mut silk = route_pair(&f, vec![f.pt(10, 20), f.pt(30, 20)], Vec::new());
    if let EditOp::RoutePair { layer, .. } = &mut silk {
        *layer = id(3);
    }
    assert_eq!(e.apply(&silk).unwrap_err().code, OPERATION_REFUSED);
    let no_vertex = route_pair(
        &f,
        vec![f.pt(10, 20), f.pt(30, 20)],
        vec![PairLayerChange {
            vertex: 5,
            layer: f.bottom,
        }],
    );
    assert_eq!(e.apply(&no_vertex).unwrap_err().code, GEOMETRY_INVALID);
    let one_point = route_pair(&f, vec![f.pt(10, 20)], Vec::new());
    assert_eq!(e.apply(&one_point).unwrap_err().code, GEOMETRY_INVALID);
    // a net that the board does not have
    let mut lost = editor(&f, "pair.q.positive = NOPE\npair.q.negative = GND\n");
    let mut op = route_pair(&f, vec![f.pt(10, 20), f.pt(30, 20)], Vec::new());
    if let EditOp::RoutePair { pair, .. } = &mut op {
        *pair = "q".to_owned();
    }
    assert_eq!(lost.apply(&op).unwrap_err().code, NET_MISSING);
    assert_eq!(e.board().to_bytes().unwrap(), before);
    assert_eq!(e.steps(), 0);
}

#[test]
fn the_shorter_net_gets_meanders_that_bend_away_from_the_other_net() {
    let mut f = fixture();
    // GND runs 13 mm above the 17 mm of SIG
    put_gnd(
        &mut f,
        41,
        250_000,
        vec![line((11_500_000, 12 * MM), (24_500_000, 12 * MM))],
    );
    let mut e = editor(&f, PAIR);
    let tune = EditOp::TunePair {
        pair: "p".to_owned(),
        side: MeanderSide::Auto,
    };
    let out = e.apply(&tune).unwrap();
    assert!(out.changed);
    let (before, after) = out.length_nm.unwrap();
    assert_eq!(before, 13_000_000);
    assert!((after - 17_000_000).abs() <= 100_000, "{after}");
    let track = &e.board().tracks[&id(41)];
    assert!(track
        .path
        .iter()
        .all(|s| s.start().y_nm.get() >= 12 * MM && s.end().y_nm.get() >= 12 * MM));
    assert!(track.path.iter().any(|s| s.start().y_nm.get() > 12 * MM));
    // a pair within its tolerance is left as it is
    let again = e.apply(&tune).unwrap();
    assert!(!again.changed);
    assert!(!again.notes.is_empty());
    assert_eq!(e.steps(), 1);
}

#[test]
fn a_piece_with_a_gap_out_of_tolerance_is_moved_to_the_gap_of_the_pair() {
    let mut f = fixture();
    // the middle piece of the track of GND is 0.9 mm above SIG, not 0.5 mm
    put_gnd(
        &mut f,
        42,
        250_000,
        vec![
            line((14 * MM, 14 * MM), (14 * MM, 10_900_000)),
            line((14 * MM, 10_900_000), (26 * MM, 10_900_000)),
            line((26 * MM, 10_900_000), (26 * MM, 14 * MM)),
        ],
    );
    let mut e = editor(&f, PAIR);
    let before = check_pairs(e.board(), e.pair_config(), None).unwrap();
    assert!(before.findings.iter().any(|x| x.code == GAP_OUT));
    let out = e
        .apply(&EditOp::FixPairGap {
            pair: "p".to_owned(),
        })
        .unwrap();
    assert!(out.changed && !out.drc.adds_error());
    assert_eq!(
        e.board().tracks[&id(42)].path,
        vec![
            line((14 * MM, 14 * MM), (14 * MM, 10_500_000)),
            line((14 * MM, 10_500_000), (26 * MM, 10_500_000)),
            line((26 * MM, 10_500_000), (26 * MM, 14 * MM)),
        ]
    );
    let after = check_pairs(e.board(), e.pair_config(), None).unwrap();
    assert!(!after.findings.iter().any(|x| x.code == GAP_OUT));
    // nothing is left to move
    let again = e
        .apply(&EditOp::FixPairGap {
            pair: "p".to_owned(),
        })
        .unwrap();
    assert!(!again.changed);
    assert_eq!(e.steps(), 1);
}

#[test]
fn a_piece_that_cannot_follow_is_refused_and_the_board_is_left_as_it_was() {
    let mut f = fixture();
    put_gnd(
        &mut f,
        42,
        250_000,
        vec![line((14 * MM, 10_900_000), (26 * MM, 10_900_000))],
    );
    let mut e = editor(&f, PAIR);
    let before = e.board().to_bytes().unwrap();
    let error = e
        .apply(&EditOp::FixPairGap {
            pair: "p".to_owned(),
        })
        .unwrap_err();
    assert_eq!(error.code, PAIR_OPERATION_REFUSED);
    assert_eq!(e.board().to_bytes().unwrap(), before);
}

#[test]
fn the_pair_operations_are_read_from_a_file_of_operations() {
    let f = fixture();
    let text = format!(
        r#"[{{"op":"route_pair","pair":"p","layer":"{}","points":[{{"x_nm":10000000,"y_nm":20000000}},{{"x_nm":30000000,"y_nm":20000000}}],"layer_changes":[{{"vertex":1,"layer":"{}"}}]}},
            {{"op":"tune_pair","pair":"p"}},{{"op":"fix_pair_gap","pair":"p"}}]"#,
        f.top, f.bottom
    );
    let ops = crate::ops_file::read_operations(&text).unwrap();
    assert_eq!(ops.len(), 3);
    assert!(matches!(&ops[0], EditOp::RoutePair { layer_changes, .. } if layer_changes.len() == 1));
    assert!(matches!(
        &ops[1],
        EditOp::TunePair {
            side: MeanderSide::Auto,
            ..
        }
    ));
    assert!(matches!(&ops[2], EditOp::FixPairGap { pair } if pair == "p"));
    let back = serde_json::to_string(&ops[0]).unwrap();
    assert_eq!(serde_json::from_str::<EditOp>(&back).unwrap(), ops[0]);
}

/// A board with GND running 1 mm above SIG over the whole 17 mm of SIG, for the checks.
fn coupled() -> Fixture {
    let mut f = fixture();
    put_gnd(
        &mut f,
        41,
        250_000,
        vec![line((11_500_000, 11 * MM), (28_500_000, 11 * MM))],
    );
    f
}

const CHECKED: &str =
    "pair.p.positive = SIG\npair.p.negative = GND\npair.p.width_nm = 250000\npair.p.gap_nm = 750000\n";

fn codes(f: &Fixture, text: &str) -> Vec<&'static str> {
    let config = PairConfig::parse(text).unwrap();
    check_pairs(&f.board, &config, None)
        .unwrap()
        .findings
        .iter()
        .map(|x| x.code)
        .collect()
}

#[test]
fn a_pair_that_runs_side_by_side_has_no_finding() {
    let f = coupled();
    let config = PairConfig::parse(CHECKED).unwrap();
    let report = check_pairs(&f.board, &config, None).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let m = &report.pairs[0];
    assert!(m.ok);
    assert_eq!((m.positive_nm, m.negative_nm), (17_000_000, 17_000_000));
    assert_eq!(m.skew_nm, 0);
    assert_eq!(m.coupled_positive_nm, 17_000_000);
    assert_eq!(m.min_gap_nm, Some(750_000));
    assert_eq!(m.max_gap_nm, Some(750_000));
    assert_eq!(m.uncoupled_nm, 0);
    assert!(!report.has_errors());
}

#[test]
fn a_short_and_too_near_net_breaks_the_gap_the_symmetry_and_the_lengths() {
    let mut f = coupled();
    put_gnd(
        &mut f,
        41,
        250_000,
        vec![line((11_500_000, 10_900_000), (20_500_000, 10_900_000))],
    );
    let found = codes(&f, CHECKED);
    assert_eq!(found, vec![GAP_OUT, ASYMMETRIC, LENGTH_MISMATCH]);
}

#[test]
fn vias_with_no_partner_and_a_wrong_width_are_found() {
    let mut f = coupled();
    through_via(&mut f, 50, true, p(20 * MM, 10 * MM));
    through_via(&mut f, 51, false, p(20 * MM, 25 * MM));
    assert_eq!(codes(&f, CHECKED), vec![VIAS_UNPAIRED]);
    // moved next to its partner the via of GND is paired
    through_via(&mut f, 51, false, p(20 * MM, 11 * MM));
    assert!(codes(&f, CHECKED).is_empty());
    put_gnd(
        &mut f,
        41,
        280_000,
        vec![line((11_500_000, 11 * MM), (28_500_000, 11 * MM))],
    );
    assert_eq!(codes(&f, CHECKED), vec![WIDTH_WRONG]);
}

#[test]
fn a_pair_whose_net_is_not_on_the_board_is_a_finding_and_the_rest_are_still_checked() {
    let f = coupled();
    let text = format!("{CHECKED}pair.q.positive = NOPE\npair.q.negative = GND\n");
    let config = PairConfig::parse(&text).unwrap();
    let report = check_pairs(&f.board, &config, None).unwrap();
    assert_eq!(report.pairs.len(), 1);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].code, NET_MISSING);
    assert_eq!(report.findings[0].pair, "q");
    assert!(report.has_errors());
}

#[test]
fn the_report_is_the_same_every_time_and_has_the_titles_in_the_language_asked_for() {
    let mut f = coupled();
    put_gnd(
        &mut f,
        41,
        250_000,
        vec![line((11_500_000, 10_900_000), (20_500_000, 10_900_000))],
    );
    let config = PairConfig::parse(CHECKED).unwrap();
    let a = check_pairs(&f.board, &config, None).unwrap();
    let b = check_pairs(&f.board, &config, None).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.to_json(), b.to_json());
    assert_eq!(a.to_text(Lang::En), b.to_text(Lang::En));
    assert!(a
        .to_text(Lang::Es)
        .contains("La separación del par está fuera de tolerancia"));
    assert!(a.to_text(Lang::Pt).contains("A separação do par"));
    let value = a.to_value();
    assert_eq!(value["schema"], "phoenix-pairs/1");
    assert_eq!(value["errors"], 3);
    assert_eq!(value["pairs"][0]["name"], "p");
    let empty = check_pairs(&f.board, &PairConfig::default(), None).unwrap();
    assert!(empty.pairs.is_empty() && !empty.has_errors());
    assert!(empty.to_text(Lang::En).contains("no pairs are defined"));
}

const JOB: &str = r#"{
  "stackup": {
    "name": "two-layer",
    "layers": [
      {"kind": "copper", "name": "Top", "thickness": {"oz": 1.0}},
      {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
      {"kind": "copper", "name": "Bottom", "thickness": {"um": 35.0}}
    ]
  },
  "materials": [
    {"name": "fr4", "dk_points": [[1.0, 4.5], [10.0, 4.3]], "df_points": [[1.0, 0.02], [10.0, 0.022]]}
  ],
  "lines": [
    {"name": "x", "kind": "microstrip", "layer": "Top", "width_nm": 150000}
  ]
}"#;

#[test]
fn the_impedance_is_only_judged_for_a_pair_with_a_target_and_a_stackup() {
    let f = coupled();
    let text = format!("{CHECKED}pair.p.target_diff_ohm = 1\n");
    let config = PairConfig::parse(&text).unwrap();
    let job = StackupJob::parse(JOB).unwrap();
    let with = check_pairs(&f.board, &config, Some(&job)).unwrap();
    assert!(with.findings.iter().any(|x| x.code == IMPEDANCE_OFF));
    assert!(!with.has_errors());
    let without = check_pairs(&f.board, &config, None).unwrap();
    assert!(without.findings.is_empty());
    assert_eq!(without.pairs[0].impedance_mohm, None);
}
