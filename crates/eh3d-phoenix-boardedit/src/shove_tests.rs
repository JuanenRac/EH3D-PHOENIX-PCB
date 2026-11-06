// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the push of neighbour tracks: what is pushed, what keeps its ends, what stops a push,
//! the limits of the settings, the preview and the meanders on a 45-degree piece.

use eh3d_phoenix_board::{Id, Net, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_rules::Lang;

use crate::codes::BoardEditError;
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::shape::{MeanderSide, Shape};
use crate::shove_codes::{
    SHOVE_ADDS_ERROR, SHOVE_BLOCKED, SHOVE_CROSSING, SHOVE_NOT_PUSHABLE, SHOVE_TOO_DEEP,
    SHOVE_TOO_MANY,
};
use crate::testutil::{fixture, id, strict_rules, Fixture};

/// A point from micrometres.
fn um(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000, y * 1_000).unwrap()
}

fn editor(f: &Fixture, settings: BoardEditSettings) -> BoardEdit {
    BoardEdit::new(f.board.clone(), strict_rules(), settings).unwrap()
}

fn bytes(e: &BoardEdit) -> Vec<u8> {
    e.board().to_bytes().unwrap()
}

/// A straight route of the ground net on the top layer, y in micrometres.
fn gnd_shove(f: &Fixture, y: i64, x0: i64, x1: i64) -> EditOp {
    EditOp::RouteShove {
        net: f.gnd,
        layer: f.top,
        points: vec![um(x0, y), um(x1, y)],
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: Some(true),
    }
}

/// A third net with a straight track on the top layer from x = 14 mm to x = 26 mm at `y`
/// micrometres, 0.25 mm wide.
fn add_other(f: &mut Fixture, y: i64) -> Id {
    let (net, track) = (id(12), id(41));
    f.board.nets.insert(
        net,
        Net {
            name: "OTHER".to_owned(),
        },
    );
    f.board.tracks.insert(
        track,
        Track {
            net,
            layer: f.top,
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: um(14_000, y),
                end: um(26_000, y),
            }],
        },
    );
    track
}

fn is_line_to(segment: &Segment, y_nm: i64) -> bool {
    matches!(segment, Segment::Line { start, end }
        if start.y_nm.get() == y_nm && end.y_nm.get() == y_nm)
}

#[test]
fn a_neighbour_track_is_pushed_aside_and_keeps_its_ends() {
    let f = fixture();
    let mut e = editor(&f, BoardEditSettings::default());
    let before = bytes(&e);
    let out = e.apply(&gnd_shove(&f, 10_300, 14_000, 26_000)).unwrap();
    assert!(out.changed && !out.drc.adds_error());
    assert_eq!(out.created.len(), 1);
    assert!(
        out.notes.iter().any(|n| n.contains("1 track(s) moved")),
        "{:?}",
        out.notes
    );
    assert_eq!(e.board().tracks.len(), 2);
    let sig = &e.board().tracks[&f.track];
    assert_eq!(sig.path.first().unwrap().start(), um(11_500, 10_000));
    assert_eq!(sig.path.last().unwrap().end(), um(28_500, 10_000));
    // a corner down, the pushed piece, a corner up
    assert_eq!(sig.path.len(), 3);
    assert!(sig.path.windows(2).all(|w| w[0].end() == w[1].start()));
    // the gap grew from 0.3 mm to the clearance and the widths: 0.2 + 0.25 + 1 micrometre
    assert!(is_line_to(&sig.path[1], 10_000_000 - 151_001));
    // undo gives back the same bytes
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn a_route_with_nothing_in_the_way_is_a_plain_route() {
    let f = fixture();
    let mut e = editor(&f, BoardEditSettings::default());
    let out = e.apply(&gnd_shove(&f, 12_000, 14_000, 26_000)).unwrap();
    assert!(out.changed);
    assert!(out
        .notes
        .iter()
        .any(|n| n.contains("nothing was in the way")));
    assert_eq!(e.board().tracks.len(), 2);
    assert_eq!(e.board().tracks[&f.track].path.len(), 1);
}

#[test]
fn a_push_goes_on_to_the_next_track_within_the_limits_of_the_settings() {
    let mut f = fixture();
    let other = add_other(&mut f, 9_500);
    let mut e = editor(&f, BoardEditSettings::default());
    let out = e.apply(&gnd_shove(&f, 10_300, 14_000, 26_000)).unwrap();
    assert!(out.notes.iter().any(|n| n.contains("2 track(s) moved")));
    assert!(!out.drc.adds_error());
    // the ends of this track are joined to nothing: it moves as a whole
    let moved = &e.board().tracks[&other];
    assert_eq!(moved.path.len(), 1);
    assert!(is_line_to(&moved.path[0], 9_500_000 - 102_002));
    assert_eq!(moved.path[0].start().x_nm.get(), 14_000_000);

    // one level deep is not enough, and nothing changes
    let shallow = BoardEditSettings {
        shove_max_depth: 1,
        ..BoardEditSettings::default()
    };
    let mut e = editor(&f, shallow);
    let before = bytes(&e);
    let error = e.apply(&gnd_shove(&f, 10_300, 14_000, 26_000)).unwrap_err();
    assert_eq!(error.code, SHOVE_TOO_DEEP);
    assert_eq!(bytes(&e), before);

    // one track is not enough either
    let few = BoardEditSettings {
        shove_max_moved: 1,
        ..BoardEditSettings::default()
    };
    let mut e = editor(&f, few);
    let error = e.apply(&gnd_shove(&f, 10_300, 14_000, 26_000)).unwrap_err();
    assert_eq!(error.code, SHOVE_TOO_MANY);
    assert_eq!(bytes(&e), before);
}

#[test]
fn fixed_copper_stops_a_push_and_a_forced_operation_does_not_get_past_it() {
    let mut f = fixture();
    f.board.vias.insert(
        id(50),
        Via {
            net: f.gnd,
            at: um(20_000, 9_300),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: f.top,
            to_layer: f.bottom,
        },
    );
    let mut e = editor(&f, BoardEditSettings::default());
    let before = bytes(&e);
    let error = e.apply(&gnd_shove(&f, 10_300, 14_000, 26_000)).unwrap_err();
    assert_eq!(error.code, SHOVE_BLOCKED);
    let forced = EditOp::Forced {
        with: Box::new(gnd_shove(&f, 10_300, 14_000, 26_000)),
    };
    let error = e.apply(&forced).unwrap_err();
    assert_eq!(error.code, SHOVE_BLOCKED);
    assert_eq!(bytes(&e), before);
    assert!(!e.can_undo());
}

#[test]
fn a_route_across_another_net_and_a_track_that_cannot_be_pushed_are_refused() {
    let f = fixture();
    let mut e = editor(&f, BoardEditSettings::default());
    let before = bytes(&e);
    let across = EditOp::RouteShove {
        net: f.gnd,
        layer: f.top,
        points: vec![um(20_000, 5_000), um(20_000, 15_000)],
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: None,
    };
    let error = e.apply(&across).unwrap_err();
    assert_eq!(error.code, SHOVE_CROSSING);
    assert_eq!(bytes(&e), before);

    // a track that is neither horizontal, vertical nor at 45 degrees
    let mut f = fixture();
    let other = add_other(&mut f, 8_900);
    if let Some(Segment::Line { end, .. }) = f.board.tracks.get_mut(&other).unwrap().path.get_mut(0)
    {
        *end = um(26_000, 8_500);
    }
    let mut e = editor(&f, BoardEditSettings::default());
    let before = bytes(&e);
    let error = e.apply(&gnd_shove(&f, 9_000, 14_000, 26_000)).unwrap_err();
    assert_eq!(error.code, SHOVE_NOT_PUSHABLE);
    assert_eq!(bytes(&e), before);
}

#[test]
fn a_push_never_leaves_a_rule_error_even_when_errors_are_not_rejected() {
    let f = fixture();
    // the route runs into the pad of R1.2, which is on another net
    let settings = BoardEditSettings {
        reject_new_errors: false,
        ..BoardEditSettings::default()
    };
    let mut e = editor(&f, settings);
    let before = bytes(&e);
    let error = e.apply(&gnd_shove(&f, 9_400, 10_000, 14_000)).unwrap_err();
    assert_eq!(error.code, SHOVE_ADDS_ERROR);
    assert_eq!(bytes(&e), before);
}

#[test]
fn the_preview_shows_the_moves_and_changes_nothing() {
    let f = fixture();
    let e = editor(&f, BoardEditSettings::default());
    let before = bytes(&e);
    let preview = e
        .preview_shove(&gnd_shove(&f, 10_300, 14_000, 26_000))
        .unwrap();
    assert_eq!(preview.route.len(), 1);
    assert_eq!(preview.moves.len(), 1);
    assert_eq!(preview.moves[0].track, f.track);
    assert_eq!(preview.moves[0].before.len(), 1);
    assert_eq!(preview.moves[0].after.len(), 3);
    assert_eq!(bytes(&e), before);
    assert_eq!(e.steps(), 0);
    let error = e
        .preview_shove(&EditOp::DeleteTrack { track: f.track })
        .unwrap_err();
    assert_eq!(error.code, crate::codes::OPERATION_REFUSED);
}

#[test]
fn the_codes_of_the_push_are_titled_in_every_language() {
    let error = BoardEditError::new(SHOVE_BLOCKED, "x");
    assert!(error.message(Lang::En).contains("blocked"));
    assert!(error.message(Lang::Pt).contains("bloqueado"));
    assert!(error.message(Lang::Ja).starts_with("PHOENIX-SHOVE-001 "));
}

#[test]
fn meanders_are_put_on_a_piece_at_45_degrees() {
    let mut f = fixture();
    let track = id(42);
    f.board.tracks.insert(
        track,
        Track {
            net: f.gnd,
            layer: f.top,
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: um(14_000, 14_000),
                end: um(34_000, 34_000),
            }],
        },
    );
    let mut e = editor(&f, BoardEditSettings::default());
    let current = e.net_length_nm(f.gnd).unwrap();
    let target = current + 6_000_000;
    let out = e
        .apply(&EditOp::TuneLength {
            net: f.gnd,
            target_nm: target,
            track: None,
            side: MeanderSide::Auto,
        })
        .unwrap();
    let (was, now) = out.length_nm.unwrap();
    assert_eq!(was, current);
    assert!((now - target).abs() <= 100_000, "{now} {target}");
    assert!(!out.drc.adds_error());
    let tuned = &e.board().tracks[&track];
    assert!(tuned.path.len() > 3);
    assert_eq!(tuned.path.first().unwrap().start(), um(14_000, 14_000));
    assert_eq!(tuned.path.last().unwrap().end(), um(34_000, 34_000));
    for segment in &tuned.path {
        let dx = segment.end().x_nm.get() - segment.start().x_nm.get();
        let dy = segment.end().y_nm.get() - segment.start().y_nm.get();
        assert_eq!(dx.abs(), dy.abs(), "{segment:?}");
    }
}
