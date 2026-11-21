// SPDX-License-Identifier: GPL-3.0-or-later
use eh3d_phoenix_model::Point;

use crate::edit::BoardEdit;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, strict_rules, Fixture};

fn editor(f: &Fixture) -> BoardEdit {
    BoardEdit::new(
        f.board.clone(),
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
}

fn mm(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
}

#[test]
fn a_route_stops_at_the_last_place_that_keeps_the_clearance() {
    let f = fixture();
    let e = editor(&f);
    // the track of SIG runs along y = 10 mm and is 0.25 mm wide; the track of GND is 0.25 mm wide
    // and the clearance is 0.2 mm, so the centre line must stay at 10.45 mm or farther
    let found = e
        .advance_route(f.gnd, f.top, mm(20, 20), mm(20, 5), true)
        .unwrap();
    assert!(!found.complete);
    assert_eq!(found.points.first(), Some(&mm(20, 20)));
    let last = found.points.last().unwrap();
    assert_eq!(last.x_nm.get(), 20_000_000);
    let y = last.y_nm.get();
    assert!((10_449_000..=10_452_000).contains(&y), "{y}");
}

#[test]
fn a_clear_route_is_complete_and_the_board_is_not_changed() {
    let f = fixture();
    let e = editor(&f);
    let (revision, steps) = (e.revision(), e.steps());
    let found = e
        .advance_route(f.gnd, f.top, mm(20, 20), mm(40, 30), true)
        .unwrap();
    assert!(found.complete);
    assert_eq!(found.points.last(), Some(&mm(40, 30)));
    assert_eq!((e.revision(), e.steps()), (revision, steps));
    assert_eq!(e.board().tracks.len(), 1);
}

#[test]
fn a_route_that_is_blocked_at_the_start_gives_only_the_start() {
    let f = fixture();
    let e = editor(&f);
    let start = Point::from_nm(20_000_000, 10_300_000).unwrap();
    let found = e
        .advance_route(f.gnd, f.top, start, mm(20, 5), true)
        .unwrap();
    assert!(!found.complete);
    assert_eq!(found.points, vec![start]);
}

#[test]
fn the_advance_is_the_same_every_time_and_follows_the_corner_order() {
    let f = fixture();
    let e = editor(&f);
    let a = e
        .advance_route(f.gnd, f.top, mm(20, 20), mm(25, 5), true)
        .unwrap();
    let b = e
        .advance_route(f.gnd, f.top, mm(20, 20), mm(25, 5), true)
        .unwrap();
    assert_eq!(a, b);
    // along y first the route meets the track of SIG at once; along x first it meets it later
    let first_y = e
        .advance_route(f.gnd, f.top, mm(20, 20), mm(25, 5), false)
        .unwrap();
    assert_ne!(a.points, first_y.points);
}

#[test]
fn a_search_with_fewer_iterations_stops_earlier_or_at_the_same_place() {
    let f = fixture();
    let fine = editor(&f)
        .advance_route(f.gnd, f.top, mm(20, 20), mm(20, 5), true)
        .unwrap();
    let settings = BoardEditSettings {
        advance_iterations: 4,
        ..BoardEditSettings::default()
    };
    let coarse = BoardEdit::new(f.board.clone(), strict_rules(), settings)
        .unwrap()
        .advance_route(f.gnd, f.top, mm(20, 20), mm(20, 5), true)
        .unwrap();
    let (y_fine, y_coarse) = (
        fine.points.last().unwrap().y_nm.get(),
        coarse.points.last().unwrap().y_nm.get(),
    );
    // moving up the board the y falls; the coarse search keeps farther from the track
    assert!(y_coarse >= y_fine, "{y_coarse} {y_fine}");
}
