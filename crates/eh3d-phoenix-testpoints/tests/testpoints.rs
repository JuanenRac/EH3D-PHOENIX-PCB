// SPDX-License-Identifier: GPL-3.0-or-later
//! The test points of a board written by the test itself, with the numbers worked out by hand.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadRef, PadShape, Side,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_testpoints::codes::{
    Severity, LIMIT_EXCEEDED, NEAR_EDGE, NET_NO_TESTPOINT, NET_WRONG_SIDE, NONE_RECOGNISED, NO_NET,
    PAD_SMALL, TOO_CLOSE,
};
use eh3d_phoenix_testpoints::{
    analyse, report_json, report_text, ProbeSide, TestPointReport, TestPointSettings,
};

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn circle(number: &str, at: Point, diameter: i64, layers: &[Id]) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: Nm::new(diameter).unwrap(),
        },
        at,
        rotation: Udeg::ZERO,
        drill: None,
        layers: layers.to_vec(),
        roundness: 0,
        plated: true,
    }
}

fn footprint(name: &str, pads: Vec<(u32, Pad)>) -> Footprint {
    Footprint {
        name: name.into(),
        pads: pads.into_iter().map(|(n, p)| (id(n), p)).collect(),
    }
}

fn place(board: &mut Board, n: u32, refdes: &str, fp: u32, x: i64, y: i64, side: Side) {
    board.components.insert(
        id(n),
        Component {
            refdes: refdes.into(),
            value: String::new(),
            symbol: None,
            footprint: id(fp),
            at: pt(x * MM / 10, y * MM / 10),
            rotation: Udeg::ZERO,
            side,
        },
    );
}

/// A 40 x 30 mm board (the positions are in tenths of a millimetre) with four resistors and the
/// nets GND, SIG1, SIG2 and SIG3, and the test points:
/// TP1 (GND, top, 1.5 mm), TP2 (SIG1, bottom, 1.5 mm), TP3 (GND, top, 0.8 mm, 0.5 mm from TP1),
/// TP4 (no net, top, 0.5 mm from the edge) and X1, a part whose footprint name holds the word
/// `TestPoint` (SIG2, top). SIG3 has no test point.
fn board() -> Board {
    let top = id(1);
    let mut b = Board {
        name: "tp".into(),
        outline: vec![
            pt(0, 0),
            pt(40 * MM, 0),
            pt(40 * MM, 30 * MM),
            pt(0, 30 * MM),
        ],
        copper_stack: vec![top, id(2)],
        ..Board::default()
    };
    b.layers.insert(
        top,
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.layers.insert(
        id(2),
        Layer {
            name: "Bottom".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    for (n, name) in [(3, "GND"), (4, "SIG1"), (5, "SIG2"), (6, "SIG3")] {
        b.nets.insert(id(n), Net { name: name.into() });
    }
    b.footprints.insert(
        id(10),
        footprint(
            "R_0603",
            vec![
                (11, circle("1", pt(-MM, 0), 800_000, &[top])),
                (12, circle("2", pt(MM, 0), 800_000, &[top])),
            ],
        ),
    );
    b.footprints.insert(
        id(13),
        footprint(
            "Pad15",
            vec![(14, circle("1", pt(0, 0), 1_500_000, &[top]))],
        ),
    );
    b.footprints.insert(
        id(15),
        footprint("Pad08", vec![(16, circle("1", pt(0, 0), 800_000, &[top]))]),
    );
    b.footprints.insert(
        id(17),
        footprint(
            "TestPoint_Pad",
            vec![(18, circle("1", pt(0, 0), 1_500_000, &[top]))],
        ),
    );
    // the resistors: pin 1 on a signal, pin 2 on GND
    for (n, refdes, x, y, signal) in [
        (20, "R1", 100, 100, 4),
        (21, "R2", 300, 100, 5),
        (22, "R3", 100, 250, 6),
        (23, "R4", 300, 250, 6),
    ] {
        place(&mut b, n, refdes, 10, x, y, Side::Top);
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(11),
            net: id(signal),
        });
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(12),
            net: id(3),
        });
    }
    place(&mut b, 30, "TP1", 13, 200, 150, Side::Top);
    place(&mut b, 31, "TP2", 13, 250, 150, Side::Bottom);
    place(&mut b, 32, "TP3", 15, 205, 150, Side::Top);
    place(&mut b, 33, "TP4", 13, 5, 150, Side::Top);
    place(&mut b, 34, "X1", 17, 350, 200, Side::Top);
    for (n, pad, net) in [(30, 14, 3), (31, 14, 4), (32, 16, 3), (34, 18, 5)] {
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(pad),
            net: id(net),
        });
    }
    b.pad_nets.sort_by_key(PadNet::pad_ref);
    b
}

fn run(settings: &TestPointSettings) -> TestPointReport {
    analyse(&board(), settings).unwrap()
}

fn codes_of(report: &TestPointReport) -> Vec<&'static str> {
    report.findings.iter().map(|f| f.code).collect()
}

#[test]
fn the_test_points_are_found_by_reference_and_by_footprint_name() {
    let r = run(&TestPointSettings::default());
    let names: Vec<String> = r
        .test_points
        .iter()
        .map(|p| format!("{}:{}", p.refdes, p.pad))
        .collect();
    assert_eq!(names, vec!["TP1:1", "TP2:1", "TP3:1", "TP4:1", "X1:1"]);
    let tp2 = &r.test_points[1];
    assert_eq!(
        (tp2.side, tp2.x_nm, tp2.y_nm, tp2.size_nm),
        ("bottom", 25 * MM, 15 * MM, 1_500_000)
    );
    assert_eq!(tp2.net.as_deref(), Some("SIG1"));
    assert!(r.test_points[3].net.is_none());
}

#[test]
fn a_net_without_a_test_point_is_reported_and_the_coverage_counted() {
    let r = run(&TestPointSettings::default());
    assert_eq!(r.nets_asked, 4);
    assert_eq!(r.nets_covered, 3);
    assert_eq!(r.coverage_x100, 7_500);
    let open: Vec<&str> = r
        .nets
        .iter()
        .filter(|n| !n.covered)
        .map(|n| n.net.as_str())
        .collect();
    assert_eq!(open, vec!["SIG3"]);
    let f = r
        .findings
        .iter()
        .find(|f| f.code == NET_NO_TESTPOINT)
        .unwrap();
    assert_eq!(
        (f.severity, f.subject.as_deref()),
        (Severity::Warning, Some("SIG3"))
    );
    assert_eq!(f.detail, "0 of 1 probed");
    let names: Vec<&str> = r.nets.iter().map(|n| n.net.as_str()).collect();
    assert_eq!(names, vec!["GND", "SIG1", "SIG2", "SIG3"]);
    assert_eq!(r.nets[0].test_points, vec!["TP1:1", "TP3:1"]);
}

#[test]
fn each_test_point_is_judged_for_size_spacing_edge_and_net() {
    let r = run(&TestPointSettings::default());
    let small = r.findings.iter().find(|f| f.code == PAD_SMALL).unwrap();
    assert_eq!(small.subject.as_deref(), Some("TP3:1"));
    assert_eq!(small.detail, "0.800000 mm, the smallest is 1.000000 mm");
    let close = r.findings.iter().find(|f| f.code == TOO_CLOSE).unwrap();
    assert_eq!(close.subject.as_deref(), Some("TP1:1 / TP3:1"));
    assert_eq!(close.detail, "0.500000 mm apart, the least is 1.270000 mm");
    let edge = r.findings.iter().find(|f| f.code == NEAR_EDGE).unwrap();
    assert_eq!(edge.subject.as_deref(), Some("TP4:1"));
    assert_eq!(
        edge.detail,
        "0.500000 mm from the edge, the least is 1.000000 mm"
    );
    let no_net = r.findings.iter().find(|f| f.code == NO_NET).unwrap();
    assert_eq!(no_net.subject.as_deref(), Some("TP4:1"));
    // one of each and the net without a test point
    assert_eq!(r.findings.len(), 5, "{:?}", codes_of(&r));
}

#[test]
fn only_the_probed_side_counts() {
    let settings = TestPointSettings {
        probe_side: ProbeSide::Top,
        ..TestPointSettings::default()
    };
    let r = run(&settings);
    assert_eq!(r.probe_side, "top");
    assert_eq!(r.nets_covered, 2);
    let wrong = r
        .findings
        .iter()
        .find(|f| f.code == NET_WRONG_SIDE)
        .unwrap();
    assert_eq!(wrong.subject.as_deref(), Some("SIG1"));
    assert_eq!(wrong.detail, "1 test points, none on the top side");
    assert!(!r.test_points[1].probed && r.test_points[0].probed);
    let bottom = TestPointSettings {
        probe_side: ProbeSide::Bottom,
        ..TestPointSettings::default()
    };
    let r = run(&bottom);
    assert_eq!(r.nets_covered, 1);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.code == NET_WRONG_SIDE)
            .count(),
        2
    );
}

#[test]
fn a_net_may_need_more_than_one_test_point() {
    let settings = TestPointSettings {
        required_per_net: 2,
        ..TestPointSettings::default()
    };
    let r = run(&settings);
    assert_eq!(r.nets_covered, 1);
    assert_eq!(r.coverage_x100, 2_500);
    let sig1 = r
        .findings
        .iter()
        .find(|f| f.subject.as_deref() == Some("SIG1"))
        .unwrap();
    assert_eq!(
        (sig1.code, sig1.detail.as_str()),
        (NET_NO_TESTPOINT, "1 of 2 probed")
    );
    let none = TestPointSettings {
        required_per_net: 0,
        ..TestPointSettings::default()
    };
    assert_eq!(run(&none).nets_covered, 4);
}

#[test]
fn the_limits_can_be_switched_off() {
    let settings = TestPointSettings {
        min_pad_nm: 0,
        min_spacing_nm: 0,
        min_edge_nm: 0,
        ..TestPointSettings::default()
    };
    let codes = codes_of(&run(&settings));
    assert!(codes
        .iter()
        .all(|c| ![PAD_SMALL, TOO_CLOSE, NEAR_EDGE].contains(c)));
    assert_eq!(codes.len(), 2, "{codes:?}");
}

#[test]
fn nets_that_are_ignored_or_have_one_pad_are_not_asked() {
    let settings = TestPointSettings {
        ignore_nets: vec!["sig3".to_owned()],
        ..TestPointSettings::default()
    };
    let r = run(&settings);
    assert_eq!(
        (r.nets_asked, r.nets_covered, r.coverage_x100),
        (3, 3, 10_000)
    );
    assert!(r.findings.iter().all(|f| f.code != NET_NO_TESTPOINT));
    let more = TestPointSettings {
        min_pads_for_test: 3,
        ..TestPointSettings::default()
    };
    // only GND has three pads or more
    assert_eq!(run(&more).nets_asked, 1);
}

#[test]
fn a_board_with_no_test_point_says_so_once() {
    let mut b = board();
    for n in 30..=34 {
        b.components.remove(&id(n));
    }
    b.pad_nets
        .retain(|p| !(30..=34).any(|n| id(n) == p.component));
    let r = analyse(&b, &TestPointSettings::default()).unwrap();
    assert!(r.test_points.is_empty());
    assert_eq!(codes_of(&r), vec![NONE_RECOGNISED]);
    assert_eq!(r.nets_covered, 0);
}

#[test]
fn the_severity_of_a_net_without_access_sets_the_errors() {
    assert!(!run(&TestPointSettings::default()).has_errors());
    let settings = TestPointSettings {
        uncovered_severity: Severity::Error,
        ..TestPointSettings::default()
    };
    assert!(run(&settings).has_errors());
}

#[test]
fn a_board_with_too_many_objects_is_refused() {
    let settings = TestPointSettings {
        max_items: 5,
        ..TestPointSettings::default()
    };
    let e = analyse(&board(), &settings).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
}

#[test]
fn the_text_is_in_the_language_asked_for_and_the_same_every_time() {
    let r = run(&TestPointSettings::default());
    let en = report_text(&r, Lang::En);
    assert!(en.starts_with("Test points of the board: tp\n"), "{en}");
    assert!(en.contains("Test points: 5 (probe side: both)\n"), "{en}");
    assert!(
        en.contains("Nets asked: 4; covered: 3; coverage: 75.00 %\n"),
        "{en}"
    );
    assert!(
        en.contains("  TP2:1 SIG1 bottom 25.000000 mm 15.000000 mm, size 1.500000 mm\n"),
        "{en}"
    );
    assert!(en.contains("  TP4:1 - top"), "{en}");
    assert!(en.contains("Nets without access:\n  SIG3\n"), "{en}");
    assert!(en.contains("warning: PHOENIX-TP-006"), "{en}");
    assert_eq!(
        en,
        report_text(&run(&TestPointSettings::default()), Lang::En)
    );
    assert!(report_text(&r, Lang::Es).contains("Redes sin acceso:\n  SIG3\n"));
    assert!(report_text(&r, Lang::Pt).starts_with("Pontos de teste da placa: tp\n"));
    let covered = TestPointSettings {
        required_per_net: 0,
        ..TestPointSettings::default()
    };
    assert!(report_text(&run(&covered), Lang::En).contains("Nets without access:\n  none\n"));
}

#[test]
fn the_json_is_the_same_every_time() {
    let r = run(&TestPointSettings::default());
    assert_eq!(report_json(&r), report_json(&r.clone()));
    assert!(report_json(&r).ends_with('\n'));
}

fn pin_of_r3() -> PadRef {
    PadRef {
        component: id(22),
        pad: id(11),
    }
}

#[test]
fn a_pad_the_board_marks_is_a_test_point_whatever_its_part_is_called() {
    let mut b = board();
    // pin 1 of R3 is on SIG3, the one net that no test point reaches
    assert!(b.mark_test_point(pin_of_r3()));
    let r = analyse(&b, &TestPointSettings::default()).unwrap();
    let names: Vec<String> = r
        .test_points
        .iter()
        .map(|p| format!("{}:{}", p.refdes, p.pad))
        .collect();
    assert_eq!(
        names,
        vec!["R3:1", "TP1:1", "TP2:1", "TP3:1", "TP4:1", "X1:1"]
    );
    // only the marked pad, not pin 2 of the same part
    assert_eq!(r.test_points[0].net.as_deref(), Some("SIG3"));
    assert_eq!(r.nets_covered, 4);
    assert_eq!(r.coverage_x100, 10_000);
    assert!(!codes_of(&r).contains(&NET_NO_TESTPOINT));
}

#[test]
fn marks_and_names_can_each_be_switched_off() {
    let mut b = board();
    b.mark_test_point(pin_of_r3());
    let names_only = TestPointSettings {
        use_marks: false,
        ..TestPointSettings::default()
    };
    let r = analyse(&b, &names_only).unwrap();
    assert_eq!(r.test_points.len(), 5);
    assert!(r.test_points.iter().all(|p| p.refdes != "R3"));
    let marks_only = TestPointSettings {
        use_names: false,
        ..TestPointSettings::default()
    };
    let r = analyse(&b, &marks_only).unwrap();
    assert_eq!(r.test_points.len(), 1);
    assert_eq!(r.test_points[0].refdes, "R3");
    // nothing marked and nothing named: nothing is recognised
    let neither = TestPointSettings {
        use_marks: false,
        use_names: false,
        ..TestPointSettings::default()
    };
    let r = analyse(&b, &neither).unwrap();
    assert!(r.test_points.is_empty());
    assert!(codes_of(&r).contains(&NONE_RECOGNISED));
}

#[test]
fn a_mark_that_names_a_pad_the_board_no_longer_has_is_ignored() {
    let mut b = board();
    b.test_points.push(PadRef {
        component: id(22),
        pad: id(99),
    });
    let r = analyse(&b, &TestPointSettings::default()).unwrap();
    assert_eq!(r.test_points.len(), 5);
}
