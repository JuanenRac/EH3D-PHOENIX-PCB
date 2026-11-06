// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the arcs in tracks: one arc of a given radius, the rounding of every corner, the
//! radius that has to fit, and the check of the arc against the clearance of the rules.

use eh3d_phoenix_board::{Id, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::shove_codes::{ARC_CLEARANCE, ARC_NO_CORNER, ARC_RADIUS};
use crate::testutil::{fixture, id, strict_rules, Fixture};

/// A point from micrometres.
fn um(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000, y * 1_000).unwrap()
}

fn line(a: Point, b: Point) -> Segment {
    Segment::Line { start: a, end: b }
}

fn arc(a: Point, b: Point, sweep_udeg: i64) -> Segment {
    Segment::Arc {
        start: a,
        end: b,
        sweep: Udeg::new(sweep_udeg).unwrap(),
    }
}

/// A track of the ground net with the given pieces.
fn add_track(f: &mut Fixture, n: u16, path: Vec<Segment>) -> Id {
    let track = id(n);
    f.board.tracks.insert(
        track,
        Track {
            net: f.gnd,
            layer: f.top,
            width: Nm::new(250_000).unwrap(),
            path,
        },
    );
    track
}

/// Two pieces of 10 mm with a corner to the left at (24, 14) mm.
fn corner_track(f: &mut Fixture) -> Id {
    add_track(
        f,
        42,
        vec![
            line(um(14_000, 14_000), um(24_000, 14_000)),
            line(um(24_000, 14_000), um(24_000, 24_000)),
        ],
    )
}

fn editor(f: &Fixture) -> BoardEdit {
    BoardEdit::new(
        f.board.clone(),
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
}

fn bytes(e: &BoardEdit) -> Vec<u8> {
    e.board().to_bytes().unwrap()
}

#[test]
fn an_arc_of_the_radius_asked_for_is_tangent_to_both_pieces() {
    let mut f = fixture();
    let track = corner_track(&mut f);
    let mut e = editor(&f);
    let before = bytes(&e);
    let out = e
        .apply(&EditOp::RouteArc {
            track,
            vertex: 1,
            radius_nm: 2_000_000,
        })
        .unwrap();
    assert!(out.changed && !out.drc.adds_error());
    assert_eq!(
        e.board().tracks[&track].path,
        vec![
            line(um(14_000, 14_000), um(22_000, 14_000)),
            arc(um(22_000, 14_000), um(24_000, 16_000), 90_000_000),
            line(um(24_000, 16_000), um(24_000, 24_000)),
        ]
    );
    // the copper is shorter than the two sharp pieces: 20 mm less 2 * 2 mm plus a quarter circle
    let length = e.net_length_nm(f.gnd).unwrap();
    assert!((length - 19_141_593).abs() <= 20_000, "{length}");
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn a_corner_to_the_right_gets_an_arc_with_a_negative_sweep() {
    let mut f = fixture();
    let track = add_track(
        &mut f,
        42,
        vec![
            line(um(14_000, 24_000), um(24_000, 24_000)),
            line(um(24_000, 24_000), um(24_000, 14_000)),
        ],
    );
    let mut e = editor(&f);
    e.apply(&EditOp::RouteArc {
        track,
        vertex: 1,
        radius_nm: 2_000_000,
    })
    .unwrap();
    assert_eq!(
        e.board().tracks[&track].path[1],
        arc(um(22_000, 24_000), um(24_000, 22_000), -90_000_000)
    );
}

#[test]
fn every_corner_of_a_track_is_rounded_with_the_radius_that_fits() {
    let mut f = fixture();
    let track = add_track(
        &mut f,
        42,
        vec![
            line(um(14_000, 14_000), um(24_000, 14_000)),
            line(um(24_000, 14_000), um(24_000, 24_000)),
            line(um(24_000, 24_000), um(34_000, 24_000)),
        ],
    );
    let mut e = editor(&f);
    let round = |net: Option<Id>, track: Option<Id>, radius_nm: i64| EditOp::RoundCorners {
        track,
        net,
        radius_nm,
    };
    e.apply(&round(None, Some(track), 2_000_000)).unwrap();
    assert_eq!(
        e.board().tracks[&track].path,
        vec![
            line(um(14_000, 14_000), um(22_000, 14_000)),
            arc(um(22_000, 14_000), um(24_000, 16_000), 90_000_000),
            line(um(24_000, 16_000), um(24_000, 22_000)),
            arc(um(24_000, 22_000), um(26_000, 24_000), -90_000_000),
            line(um(26_000, 24_000), um(34_000, 24_000)),
        ]
    );
    // a corner that is already round has no straight pieces on both sides: one with a bigger
    // radius than the room is cut to the room (5 mm: half of the piece between the two corners)
    assert!(e.undo());
    let out = e.apply(&round(Some(f.gnd), None, 20_000_000)).unwrap();
    assert!(out.notes.iter().any(|n| n.contains("2 corner(s) rounded")));
    let path = &e.board().tracks[&track].path;
    assert_eq!(path.len(), 4);
    assert_eq!(
        path[1],
        arc(um(19_000, 14_000), um(24_000, 19_000), 90_000_000)
    );
    assert_eq!(
        path[2],
        arc(um(24_000, 19_000), um(29_000, 24_000), -90_000_000)
    );
}

#[test]
fn a_radius_that_is_not_valid_or_does_not_fit_is_refused_with_its_code() {
    let mut f = fixture();
    let track = corner_track(&mut f);
    let mut e = editor(&f);
    let before = bytes(&e);
    let arc_at = |vertex: usize, radius_nm: i64| EditOp::RouteArc {
        track,
        vertex,
        radius_nm,
    };
    assert_eq!(
        e.apply(&arc_at(1, 100_000)).unwrap_err().code,
        ARC_RADIUS,
        "below the smallest radius"
    );
    assert_eq!(e.apply(&arc_at(1, -5)).unwrap_err().code, ARC_RADIUS);
    assert_eq!(
        e.apply(&arc_at(1, 20_000_000)).unwrap_err().code,
        ARC_NO_CORNER,
        "the pieces are 10 mm long"
    );
    assert_eq!(
        e.apply(&arc_at(0, 1_000_000)).unwrap_err().code,
        ARC_NO_CORNER
    );
    assert_eq!(
        e.apply(&arc_at(2, 1_000_000)).unwrap_err().code,
        ARC_NO_CORNER
    );
    let missing = EditOp::RoundCorners {
        track: None,
        net: None,
        radius_nm: 1_000_000,
    };
    assert!(e.apply(&missing).is_err());
    assert_eq!(bytes(&e), before);
}

#[test]
fn pieces_on_one_line_and_arcs_have_no_corner_to_round() {
    let mut f = fixture();
    let straight = add_track(
        &mut f,
        42,
        vec![
            line(um(14_000, 14_000), um(20_000, 14_000)),
            line(um(20_000, 14_000), um(30_000, 14_000)),
        ],
    );
    let curved = add_track(
        &mut f,
        43,
        vec![
            line(um(14_000, 20_000), um(22_000, 20_000)),
            arc(um(22_000, 20_000), um(24_000, 22_000), 90_000_000),
            line(um(24_000, 22_000), um(24_000, 30_000)),
        ],
    );
    let mut e = editor(&f);
    let before = bytes(&e);
    for track in [straight, curved] {
        let error = e
            .apply(&EditOp::RouteArc {
                track,
                vertex: 1,
                radius_nm: 1_000_000,
            })
            .unwrap_err();
        assert_eq!(error.code, ARC_NO_CORNER);
        let error = e
            .apply(&EditOp::RoundCorners {
                track: Some(track),
                net: None,
                radius_nm: 1_000_000,
            })
            .unwrap_err();
        assert_eq!(error.code, ARC_NO_CORNER);
    }
    assert_eq!(bytes(&e), before);
}

#[test]
fn an_arc_that_comes_too_close_to_other_copper_is_refused() {
    let mut f = fixture();
    let track = corner_track(&mut f);
    // inside the corner, 1 mm from both pieces: far enough from the sharp corner, but the arc
    // cuts the corner and passes 0.59 mm from the middle of the via
    f.board.vias.insert(
        id(50),
        Via {
            net: f.sig,
            at: um(23_000, 15_000),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: f.top,
            to_layer: f.bottom,
        },
    );
    let mut e = editor(&f);
    let before = bytes(&e);
    let op = EditOp::RouteArc {
        track,
        vertex: 1,
        radius_nm: 2_000_000,
    };
    assert_eq!(e.apply(&op).unwrap_err().code, ARC_CLEARANCE);
    assert_eq!(bytes(&e), before);
    // a smaller arc keeps away
    let small = EditOp::RouteArc {
        track,
        vertex: 1,
        radius_nm: 600_000,
    };
    e.apply(&small).unwrap();
    assert_eq!(e.board().tracks[&track].path.len(), 3);
}
