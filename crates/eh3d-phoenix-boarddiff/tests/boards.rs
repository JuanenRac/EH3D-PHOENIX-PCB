// SPDX-License-Identifier: GPL-3.0-or-later
//! Two revisions of a board built by the test itself and compared: the differences are the ones
//! that were made, they do not depend on the ids of the objects, and the report is the same every
//! time.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_boarddiff::codes::{LIMIT_EXCEEDED, LIST_CUT};
use eh3d_phoenix_boarddiff::{compare_boards, report_json, report_text, DiffSettings};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::Lang;

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn pad(number: &str, at: Point, top: Id) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: nm(800_000),
        },
        at,
        rotation: Udeg::ZERO,
        drill: None,
        layers: vec![top],
        roundness: 0,
        plated: true,
    }
}

fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<Point> {
    vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]
}

/// Revision A of a 40 x 20 mm board, or revision B (`b`) with a changed value, a part moved and
/// turned, a part and a net added, a longer track, a moved via, a bigger cutout in the zone, a
/// pin on another net and a wider outline. Every id is shifted by `offset`.
fn board(offset: u32, b: bool) -> Board {
    let i = |n: u32| id(offset + n);
    let (top, bottom, gnd, sig, vcc) = (i(1), i(2), i(3), i(4), i(5));
    let width = if b { 45 * MM } else { 40 * MM };
    let track_end = if b { 22 * MM } else { 20 * MM };
    let mut board = Board {
        name: if b { "rev B" } else { "rev A" }.into(),
        outline: rectangle(0, 0, width, 20 * MM),
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    for (layer, name) in [(top, "Top"), (bottom, "Bottom")] {
        board.layers.insert(
            layer,
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    board.nets.insert(gnd, Net { name: "GND".into() });
    board.nets.insert(sig, Net { name: "SIG".into() });
    if b {
        board.nets.insert(vcc, Net { name: "VCC".into() });
    }
    let mut two_pads = |footprint: u32, name: &str, first: u32| {
        let mut f = Footprint {
            name: name.into(),
            pads: Default::default(),
        };
        f.pads.insert(i(first), pad("1", pt(-800_000, 0), top));
        f.pads.insert(i(first + 1), pad("2", pt(800_000, 0), top));
        board.footprints.insert(i(footprint), f);
    };
    two_pads(10, "R_0603", 11);
    two_pads(20, "C_0603", 21);
    let mut place = |n: u32, refdes: &str, value: &str, fp: u32, at: Point, deg: i64| {
        board.components.insert(
            i(n),
            Component {
                refdes: refdes.into(),
                value: value.into(),
                symbol: None,
                footprint: i(fp),
                at,
                rotation: Udeg::new(deg).unwrap(),
                side: Side::Top,
            },
        );
    };
    place(
        30,
        "R1",
        if b { "22k" } else { "10k" },
        10,
        pt(10 * MM, 10 * MM),
        0,
    );
    if b {
        place(31, "C1", "100n", 20, pt(22 * MM, 10 * MM), 90_000_000);
        place(32, "R2", "1k", 10, pt(30 * MM, 10 * MM), 0);
    } else {
        place(31, "C1", "100n", 20, pt(20 * MM, 10 * MM), 0);
    }
    let mut connect = |component: u32, pad_id: u32, net: Id| {
        board.pad_nets.push(PadNet {
            component: i(component),
            pad: i(pad_id),
            net,
        });
    };
    connect(30, 11, sig);
    connect(30, 12, gnd);
    connect(31, 21, sig);
    connect(31, 22, if b { vcc } else { gnd });
    board.tracks.insert(
        i(40),
        Track {
            net: sig,
            layer: top,
            width: nm(200_000),
            path: vec![Segment::Line {
                start: pt(10 * MM, 10 * MM),
                end: pt(track_end, 10 * MM),
            }],
        },
    );
    board.vias.insert(
        i(50),
        Via {
            net: gnd,
            at: if b {
                pt(16 * MM, 15 * MM)
            } else {
                pt(15 * MM, 15 * MM)
            },
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: top,
            to_layer: bottom,
        },
    );
    let cutout = if b {
        rectangle(5 * MM, 5 * MM, 9 * MM, 9 * MM)
    } else {
        rectangle(5 * MM, 5 * MM, 7 * MM, 7 * MM)
    };
    board.zones.insert(
        i(60),
        Zone {
            net: gnd,
            layer: bottom,
            outline: rectangle(MM, MM, 39 * MM, 19 * MM),
            clearance: nm(250_000),
            cutouts: vec![cutout],
        },
    );
    board
}

#[test]
fn the_differences_are_the_ones_that_were_made_in_the_order_of_their_codes() {
    let r = compare_boards(&board(0, false), &board(0, true), &DiffSettings::default()).unwrap();
    let lines: Vec<(&str, &str, &str)> = r
        .differences
        .iter()
        .map(|d| (d.code, d.subject.as_str(), d.detail.as_str()))
        .collect();
    assert_eq!(
        lines,
        vec![
            ("PHOENIX-DIFF-006", "VCC", "added"),
            ("PHOENIX-DIFF-007", "R2", "added (1k, R_0603, top)"),
            ("PHOENIX-DIFF-008", "R1", "10k -> 22k"),
            (
                "PHOENIX-DIFF-010",
                "C1",
                "moved 2 mm, turned from 0 to 90 degrees"
            ),
            ("PHOENIX-DIFF-011", "C1.2", "GND -> VCC"),
            (
                "PHOENIX-DIFF-012",
                "SIG on Top",
                "tracks 1 -> 1, length 10 mm -> 12 mm"
            ),
            ("PHOENIX-DIFF-013", "GND at 15, 15 mm", "removed"),
            ("PHOENIX-DIFF-013", "GND at 16, 15 mm", "added"),
            (
                "PHOENIX-DIFF-014",
                "GND on Bottom",
                "zones 1 -> 1, area 680.000 mm2 -> 668.000 mm2"
            ),
            (
                "PHOENIX-DIFF-015",
                "outline",
                "box 40 x 20 mm -> 45 x 20 mm, area 800.000 mm2 -> 900.000 mm2"
            ),
        ]
    );
    assert_eq!(r.total, 10);
    assert!(r.differs());
    let get = |what: &str| r.counts.iter().find(|c| c.what == what).copied().unwrap();
    assert_eq!((get("parts").old, get("parts").new), (2, 3));
    assert_eq!((get("nets").old, get("nets").new), (2, 3));
}

#[test]
fn the_ids_of_the_objects_do_not_matter() {
    let same = compare_boards(
        &board(0, true),
        &board(5000, true),
        &DiffSettings::default(),
    )
    .unwrap();
    assert!(!same.differs(), "{:?}", same.differences);
    // and the differences are the same whatever the ids are
    let shifted = compare_boards(
        &board(300, false),
        &board(2000, true),
        &DiffSettings::default(),
    )
    .unwrap();
    let plain =
        compare_boards(&board(0, false), &board(0, true), &DiffSettings::default()).unwrap();
    assert_eq!(shifted.differences, plain.differences);
}

#[test]
fn the_report_in_text_and_in_json_is_the_same_every_time() {
    let s = DiffSettings::default();
    let a = compare_boards(&board(0, false), &board(0, true), &s).unwrap();
    let b = compare_boards(&board(0, false), &board(0, true), &s).unwrap();
    assert_eq!(report_json(&a), report_json(&b));
    let text = report_text(&a, Lang::Es);
    assert!(
        text.starts_with("Comparación de placas: rev A -> rev B\n"),
        "{text}"
    );
    assert!(text.contains("Diferencias: 10"), "{text}");
    assert!(
        text.contains("Valor de una pieza cambiado [R1]: 10k -> 22k"),
        "{text}"
    );
}

#[test]
fn the_parts_and_nets_that_are_left_out_do_not_show() {
    let s = DiffSettings {
        ignore_parts: vec!["R2".into()],
        ignore_nets: vec!["VCC".into()],
        ..DiffSettings::default()
    };
    let r = compare_boards(&board(0, false), &board(0, true), &s).unwrap();
    // R2 and VCC are gone from the list; the pin of C1 that was on GND has no net now that VCC is
    // not looked at
    assert_eq!(r.total, 8, "{:?}", r.differences);
    assert!(r
        .differences
        .iter()
        .all(|d| d.subject != "R2" && d.subject != "VCC"));
    let pin = r.differences.iter().find(|d| d.subject == "C1.2").unwrap();
    assert_eq!(pin.detail, "GND -> (no net)");
}

#[test]
fn a_part_that_moved_less_than_the_tolerance_is_not_listed() {
    let s = DiffSettings {
        move_tolerance_nm: 2 * MM,
        compare_pins: false,
        ..DiffSettings::default()
    };
    let r = compare_boards(&board(0, false), &board(0, true), &s).unwrap();
    // the move of 2 mm is not more than the tolerance, but the turn of the part still counts
    let detail = &r
        .differences
        .iter()
        .find(|d| d.subject == "C1" && d.code == "PHOENIX-DIFF-010")
        .unwrap()
        .detail;
    assert_eq!(detail, "turned from 0 to 90 degrees");
}

#[test]
fn a_board_with_more_objects_than_the_limit_is_refused_with_its_code() {
    let s = DiffSettings {
        max_items: 3,
        ..DiffSettings::default()
    };
    let e = compare_boards(&board(0, false), &board(0, true), &s).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
    assert!(e.detail.contains("rev A"), "{}", e.detail);
}

#[test]
fn a_list_that_is_cut_says_so_and_still_counts_every_difference() {
    let s = DiffSettings {
        max_differences: 4,
        ..DiffSettings::default()
    };
    let r = compare_boards(&board(0, false), &board(0, true), &s).unwrap();
    assert_eq!(r.total, 10);
    assert_eq!(r.differences.len(), 5);
    let last = r.differences.last().unwrap();
    assert_eq!(last.code, LIST_CUT);
    assert_eq!(last.detail, "6 more not listed (max_differences = 4)");
}

#[test]
fn an_empty_board_compared_with_itself_has_nothing() {
    let r = compare_boards(
        &Board::default(),
        &Board::default(),
        &DiffSettings::default(),
    )
    .unwrap();
    assert!(!r.differs());
    assert!(r.counts.iter().all(|c| c.old == 0 && c.new == 0));
}
