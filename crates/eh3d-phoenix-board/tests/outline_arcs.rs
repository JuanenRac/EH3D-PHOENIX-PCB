//! The arcs in the outline of a board: a field that is optional, so a board without arcs is
//! written as before, the polygon the arcs are cut into, and the checks of the arcs.

use eh3d_phoenix_board::validate::{BAD_ARC, BAD_OUTLINE};
use eh3d_phoenix_board::{ArcSettings, Board, OutlineArc};
use eh3d_phoenix_model::{Point, Udeg};

const MM: i64 = 1_000_000;

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x * MM, y * MM).unwrap()
}

fn deg(d: i64) -> Udeg {
    Udeg::new(d * 1_000_000).unwrap()
}

/// A board of 10 by 2 mm whose first edge, from (0, 0) to (10, 0), is an arc of the sweep.
fn strip(sweep: Udeg) -> Board {
    Board {
        name: "strip".to_owned(),
        outline: vec![pt(0, 0), pt(10, 0), pt(10, 2), pt(0, 2)],
        outline_arcs: vec![OutlineArc { edge: 0, sweep }],
        ..Board::default()
    }
}

fn count(board: &Board, code: &str) -> usize {
    board
        .validate()
        .iter()
        .filter(|d| d.code.as_deref() == Some(code))
        .count()
}

/// Twice the area of a polygon in square nanometres, without its sign.
fn double_area(points: &[Point]) -> i128 {
    let n = points.len();
    let mut sum: i128 = 0;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        sum += i128::from(a.x_nm.get()) * i128::from(b.y_nm.get())
            - i128::from(b.x_nm.get()) * i128::from(a.y_nm.get());
    }
    sum.abs()
}

#[test]
fn a_board_without_arcs_writes_none_and_a_board_with_arcs_reads_back() {
    let mut board = strip(deg(180));
    let text = String::from_utf8(board.to_bytes().unwrap()).unwrap();
    assert!(text.contains("outline_arcs"), "{text}");
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), board);
    board.outline_arcs.clear();
    let plain = String::from_utf8(board.to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("outline_arcs"), "{plain}");
    assert_eq!(Board::from_bytes(plain.as_bytes()).unwrap(), board);
}

#[test]
fn a_half_circle_on_one_edge_adds_the_area_of_its_chords() {
    // the edge from (0, 0) to (10, 0) turned counter-clockwise by 180 degrees bulges outwards, to
    // y = -5 mm: a half circle of radius 5 mm about (5, 0). With a tolerance of 1 micrometre a
    // chord may span 2 * acos(1 - 1000 / 5e6) = 0.0400007 rad, so 79 chords, 78 new points. The
    // chords are inscribed, so the area is 20 mm2 + 79 * (1/2) * 5^2 * sin(pi / 79) mm2
    // = 59.259559 mm2, not the 59.269908 mm2 of the true half circle
    let board = strip(deg(180));
    let flat = board.outline_polygon(&ArcSettings::default());
    assert_eq!(flat.len(), 4 + 78);
    assert_eq!(flat[0], pt(0, 0));
    assert_eq!(flat[79], pt(10, 0));
    let twice = double_area(&flat);
    let sampled = 2 * 59_259_558_658_214_i128;
    assert!((twice - sampled).abs() < 1_000_000_000, "{twice}");
    let exact = 2 * 59_269_908_169_872_i128;
    assert!((twice - exact).abs() < 40_000_000_000, "{twice}");
    // nothing is wrong with it
    assert_eq!(count(&board, BAD_OUTLINE), 0);
    assert_eq!(count(&board, BAD_ARC), 0);
}

#[test]
fn an_arc_that_bulges_into_the_board_across_another_edge_is_found() {
    // clockwise, the arc rises to y = 5 mm, past the edge at y = 2 mm
    let board = strip(deg(-180));
    assert_eq!(count(&board, BAD_OUTLINE), 1);
    assert_eq!(count(&board, BAD_ARC), 0);
    // the same arc as a short one stays inside: 2 * 5 * sin(...) ... a sweep of -20 degrees
    // from (0, 0) to (10, 0) has a radius of 10 / (2 * sin 10 degrees) = 28.79 mm and a rise of
    // R * (1 - cos 10 degrees) = 0.437 mm, below the edge at 2 mm
    assert_eq!(count(&strip(deg(-20)), BAD_OUTLINE), 0);
}

#[test]
fn the_arcs_that_cannot_be_used_are_errors_and_leave_the_straight_edge() {
    let mut board = strip(deg(90));
    board.outline_arcs = vec![
        OutlineArc {
            edge: 7,
            sweep: deg(90),
        },
        OutlineArc {
            edge: 1,
            sweep: Udeg::ZERO,
        },
        OutlineArc {
            edge: 2,
            sweep: deg(30),
        },
        OutlineArc {
            edge: 2,
            sweep: deg(30),
        },
    ];
    // an edge that does not exist, a zero sweep, and the second arc on edge 2
    assert_eq!(count(&board, BAD_ARC), 3);
    let flat = board.outline_polygon(&ArcSettings::default());
    // the arc of edge 2 is used once; the others leave their edges straight
    assert!(flat.len() > 4);
    assert_eq!(flat[0], pt(0, 0));
    assert_eq!(flat[1], pt(10, 0));
    assert_eq!(flat[2], pt(10, 2));
}
