// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the board editor: the operations, the rule check after each one, undo and redo.

use eh3d_phoenix_board::{Component, PadNet, PadRef, Segment, Side};
use eh3d_phoenix_model::{Point, Udeg};

use crate::codes::{COMPONENT_LOCKED, OPERATION_REFUSED, REJECTED_BY_DRC, TUNE_IMPOSSIBLE};
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::shape::Shape;
use crate::testutil::{fixture, id, strict_rules, Fixture};

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

/// Makes the track of the fixture run through the given x values (in millimetres) at y = 10.
fn through(f: &mut Fixture, xs: &[i64]) {
    let track = f.board.tracks.get_mut(&f.track).unwrap();
    track.path = xs
        .windows(2)
        .map(|w| Segment::Line {
            start: Point::from_nm(w[0] * 1_000_000, 10_000_000).unwrap(),
            end: Point::from_nm(w[1] * 1_000_000, 10_000_000).unwrap(),
        })
        .collect();
}

/// A resistor without nets on the board of the fixture.
fn add_part(f: &mut Fixture, n: u16, refdes: &str, x: i64, y: i64) -> eh3d_phoenix_board::Id {
    let part = id(n);
    f.board.components.insert(
        part,
        Component {
            refdes: refdes.to_owned(),
            value: "1k".to_owned(),
            symbol: None,
            footprint: f.footprint,
            at: f.pt(x, y),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    part
}

fn gnd_route(f: &Fixture, points: Vec<Point>) -> EditOp {
    EditOp::Route {
        net: f.gnd,
        layer: f.top,
        points,
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: Some(true),
    }
}

#[test]
fn a_route_adds_a_track_of_the_default_width_and_undo_and_redo_give_the_same_bytes() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    let op = gnd_route(&f, vec![f.pt(5, 14), f.pt(15, 20)]);
    let out = e.apply(&op).unwrap();
    assert!(out.changed && !out.forced);
    assert_eq!(out.created.len(), 1);
    assert!(!out.drc.adds_error());
    let track = &e.board().tracks[&out.created[0]];
    assert_eq!(track.width.get(), 250_000);
    assert_eq!(track.path.len(), 2);
    assert_eq!(e.steps(), 1);
    let after = bytes(&e);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    assert!(!e.can_undo() && e.can_redo());
    assert!(e.redo());
    assert_eq!(bytes(&e), after);
    assert!(!e.redo());
}

#[test]
fn an_operation_that_adds_a_rule_error_is_refused_unless_it_is_forced() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    // a track of the ground net straight across the signal track
    let across = gnd_route(&f, vec![f.pt(20, 8), f.pt(20, 12)]);
    let error = e.apply(&across).unwrap_err();
    assert_eq!(error.code, REJECTED_BY_DRC);
    assert_eq!(bytes(&e), before);
    assert_eq!(e.steps(), 0);
    let forced = EditOp::Forced {
        with: Box::new(across),
    };
    let out = e.apply(&forced).unwrap();
    assert!(out.forced && out.drc.adds_error());
    assert!(out.drc.errors_after > out.drc.errors_before);
    assert_eq!(e.board().tracks.len(), 2);
    assert!(out
        .drc
        .to_text(eh3d_phoenix_rules::Lang::Es)
        .contains("errores de reglas"));
    // without the check an operation is not refused
    let settings = BoardEditSettings {
        reject_new_errors: false,
        ..BoardEditSettings::default()
    };
    let mut open = BoardEdit::new(f.board.clone(), strict_rules(), settings).unwrap();
    let again = gnd_route(&f, vec![f.pt(20, 8), f.pt(20, 12)]);
    assert!(open.apply(&again).is_ok());
}

#[test]
fn moving_a_component_drags_the_track_ends_on_its_pads_unless_told_not_to() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    let moved = EditOp::Move {
        id: f.r1,
        at: f.pt(10, 13),
        rotation_udeg: None,
        follow: None,
    };
    e.apply(&moved).unwrap();
    let Segment::Line { start, end } = e.board().tracks[&f.track].path[0] else {
        panic!("a line");
    };
    assert_eq!(start, Point::from_nm(11_500_000, 13_000_000).unwrap());
    assert_eq!(end, Point::from_nm(28_500_000, 10_000_000).unwrap());
    assert_eq!(e.steps(), 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    let stay = EditOp::Move {
        id: f.r1,
        at: f.pt(10, 13),
        rotation_udeg: None,
        follow: Some(false),
    };
    e.apply(&stay).unwrap();
    let Segment::Line { start, .. } = e.board().tracks[&f.track].path[0] else {
        panic!("a line");
    };
    assert_eq!(start, Point::from_nm(11_500_000, 10_000_000).unwrap());
}

#[test]
fn a_locked_component_is_refused_and_unlocking_frees_it() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    assert!(!e.apply(&EditOp::Lock { id: f.r1 }).unwrap().changed);
    assert!(e.is_locked(f.r1));
    let to = f.pt(10, 12);
    let move_it = EditOp::Move {
        id: f.r1,
        at: to,
        rotation_udeg: None,
        follow: Some(false),
    };
    assert_eq!(e.apply(&move_it).unwrap_err().code, COMPONENT_LOCKED);
    for op in [
        EditOp::Rotate {
            id: f.r1,
            delta_udeg: 90_000_000,
            follow: None,
        },
        EditOp::Mirror {
            id: f.r1,
            follow: None,
        },
        EditOp::Remove { id: f.r1 },
    ] {
        assert_eq!(e.apply(&op).unwrap_err().code, COMPONENT_LOCKED);
    }
    assert_eq!(bytes(&e), before);
    e.apply(&EditOp::Unlock { id: f.r1 }).unwrap();
    assert!(e.apply(&move_it).is_ok());
    assert_eq!(e.board().components[&f.r1].at, to);
    assert_eq!(
        e.apply(&EditOp::Lock { id: id(99) }).unwrap_err().code,
        OPERATION_REFUSED
    );
}

#[test]
fn mirroring_changes_the_side_and_keeps_the_nets_of_the_pads() {
    let mut f = fixture();
    let r3 = add_part(&mut f, 60, "R3", 40, 30);
    f.board.pad_nets.push(PadNet {
        component: r3,
        pad: f.pad1,
        net: f.gnd,
    });
    f.board.pad_nets.sort_by_key(PadNet::pad_ref);
    let mut e = editor(&f);
    let before = bytes(&e);
    e.apply(&EditOp::Mirror {
        id: r3,
        follow: None,
    })
    .unwrap();
    assert_eq!(e.board().components[&r3].side, Side::Bottom);
    assert_eq!(e.board().pad_nets.len(), 5);
    assert_eq!(e.board().net_of(f.pad(r3, f.pad1)), Some(f.gnd));
    // a pad that was at -1.5 mm is now at +1.5 mm
    let at = eh3d_phoenix_board::connectivity::pad_position(e.board(), f.pad(r3, f.pad1)).unwrap();
    assert_eq!(at, Point::from_nm(41_500_000, 30_000_000).unwrap());
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn rotating_turns_by_the_angle_and_keeps_it_inside_one_turn() {
    let mut f = fixture();
    let r3 = add_part(&mut f, 60, "R3", 40, 30);
    let mut e = editor(&f);
    for _ in 0..5 {
        e.apply(&EditOp::Rotate {
            id: r3,
            delta_udeg: 90_000_000,
            follow: None,
        })
        .unwrap();
    }
    assert_eq!(e.board().components[&r3].rotation.get(), 90_000_000);
}

#[test]
fn placing_makes_a_component_and_refuses_a_second_reference() {
    let f = fixture();
    let mut e = editor(&f);
    let place = |refdes: &str| EditOp::Place {
        id: None,
        refdes: refdes.to_owned(),
        value: "2k".to_owned(),
        footprint: f.footprint,
        at: f.pt(40, 30),
        rotation_udeg: 0,
        side: Some(Side::Bottom),
    };
    let out = e.apply(&place("R7")).unwrap();
    assert_eq!(e.board().components[&out.created[0]].refdes, "R7");
    assert_eq!(e.board().components[&out.created[0]].side, Side::Bottom);
    assert_eq!(e.apply(&place("R7")).unwrap_err().code, OPERATION_REFUSED);
    assert_eq!(e.apply(&place("")).unwrap_err().code, OPERATION_REFUSED);
}

#[test]
fn tuning_adds_meanders_until_the_net_is_the_length_asked_for() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    assert_eq!(e.net_length_nm(f.sig), Some(17_000_000));
    let tune = |target_nm: i64| EditOp::TuneLength {
        net: f.sig,
        target_nm,
        track: None,
        side: crate::shape::MeanderSide::Auto,
    };
    let out = e.apply(&tune(20_000_000)).unwrap();
    assert!(out.changed);
    assert_eq!(out.length_nm, Some((17_000_000, 20_000_000)));
    assert_eq!(e.net_length_nm(f.sig), Some(20_000_000));
    assert!(e.board().tracks[&f.track].path.len() > 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    // within the tolerance nothing is done, and a net that is already too long is refused
    let near = e.apply(&tune(17_050_000)).unwrap();
    assert!(!near.changed);
    assert_eq!(e.steps(), 0);
    assert_eq!(
        e.apply(&tune(10_000_000)).unwrap_err().code,
        TUNE_IMPOSSIBLE
    );
    // a net with no copper cannot be measured
    let none = EditOp::TuneLength {
        net: f.gnd,
        target_nm: 5_000_000,
        track: None,
        side: crate::shape::MeanderSide::Left,
    };
    assert_eq!(e.apply(&none).unwrap_err().code, TUNE_IMPOSSIBLE);
}

#[test]
fn pasting_a_net_makes_it_gives_it_to_the_pads_and_joins_them() {
    let mut f = fixture();
    let r3 = add_part(&mut f, 60, "R3", 40, 30);
    let r4 = add_part(&mut f, 61, "R4", 50, 35);
    let mut e = editor(&f);
    let before = bytes(&e);
    let paste = EditOp::PasteNet {
        name: "NEW".to_owned(),
        pads: vec![f.pad(r3, f.pad2), f.pad(r4, f.pad1)],
        layer: f.top,
        shape: None,
    };
    let out = e.apply(&paste).unwrap();
    assert_eq!(out.created.len(), 2);
    let net = e
        .board()
        .nets
        .iter()
        .find(|(_, n)| n.name == "NEW")
        .map(|(i, _)| *i)
        .unwrap();
    assert_eq!(e.board().net_of(f.pad(r3, f.pad2)), Some(net));
    assert_eq!(e.board().net_of(f.pad(r4, f.pad1)), Some(net));
    assert_eq!(e.board().tracks.len(), 2);
    // 41.5 -> 43.5 straight, then 45 degrees to (48.5, 35)
    assert_eq!(e.board().tracks[&out.created[1]].path.len(), 2);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    // a pad that is on another net is refused
    let taken = EditOp::PasteNet {
        name: "OTHER".to_owned(),
        pads: vec![f.pad(f.r1, f.pad1), f.pad(r4, f.pad1)],
        layer: f.top,
        shape: None,
    };
    assert_eq!(e.apply(&taken).unwrap_err().code, OPERATION_REFUSED);
    assert_eq!(bytes(&e), before);
}

#[test]
fn the_shortest_route_between_two_pads_is_straight_when_they_are_in_line() {
    let mut f = fixture();
    let r3 = add_part(&mut f, 60, "R3", 40, 30);
    let r4 = add_part(&mut f, 61, "R4", 50, 30);
    f.board.pad_nets.push(PadNet {
        component: r3,
        pad: f.pad2,
        net: f.gnd,
    });
    f.board.pad_nets.sort_by_key(PadNet::pad_ref);
    let mut e = editor(&f);
    let join = EditOp::RouteBetween {
        from: f.pad(r3, f.pad2),
        to: f.pad(r4, f.pad1),
        layer: f.top,
        shape: None,
    };
    let out = e.apply(&join).unwrap();
    assert_eq!(e.board().tracks[&out.created[0]].path.len(), 1);
    assert_eq!(e.board().net_of(f.pad(r4, f.pad1)), Some(f.gnd));
    assert_eq!(e.steps(), 1);
    // two pads with no net have nothing to join, and a pad is not joined with itself
    let none = EditOp::RouteBetween {
        from: f.pad(r4, f.pad2),
        to: f.pad(r3, f.pad1),
        layer: f.top,
        shape: None,
    };
    assert_eq!(e.apply(&none).unwrap_err().code, OPERATION_REFUSED);
    let same = EditOp::RouteBetween {
        from: f.pad(r4, f.pad1),
        to: f.pad(r4, f.pad1),
        layer: f.top,
        shape: None,
    };
    assert_eq!(e.apply(&same).unwrap_err().code, OPERATION_REFUSED);
}

#[test]
fn a_via_is_added_with_the_size_of_the_rules() {
    let f = fixture();
    let mut e = editor(&f);
    let out = e
        .apply(&EditOp::Via {
            net: f.gnd,
            at: f.pt(20, 20),
            from_layer: None,
            to_layer: None,
        })
        .unwrap();
    let via = &e.board().vias[&out.created[0]];
    assert_eq!((via.drill.get(), via.diameter.get()), (300_000, 600_000));
    assert_eq!((via.from_layer, via.to_layer), (f.top, f.bottom));
    let one_layer = EditOp::Via {
        net: f.gnd,
        at: f.pt(20, 22),
        from_layer: Some(f.top),
        to_layer: Some(f.top),
    };
    assert_eq!(e.apply(&one_layer).unwrap_err().code, OPERATION_REFUSED);
}

#[test]
fn changing_layer_splits_the_track_and_puts_a_via_at_the_vertex() {
    let mut f = fixture();
    through(&mut f, &[11, 20, 28]);
    let mut e = editor(&f);
    let before = bytes(&e);
    let out = e
        .apply(&EditOp::ChangeLayer {
            track: f.track,
            vertex: 1,
            layer: f.bottom,
        })
        .unwrap();
    assert_eq!(e.board().tracks.len(), 2);
    assert_eq!(e.board().vias.len(), 1);
    assert_eq!(e.board().tracks[&f.track].path.len(), 1);
    assert_eq!(e.board().tracks[&out.created[0]].layer, f.bottom);
    assert_eq!(e.board().vias[&out.created[1]].at, f.pt(20, 10));
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    for vertex in [0, 2] {
        let bad = EditOp::ChangeLayer {
            track: f.track,
            vertex,
            layer: f.bottom,
        };
        assert!(e.apply(&bad).is_err(), "{vertex}");
    }
}

#[test]
fn deleting_a_stretch_leaves_the_rest_in_one_or_two_tracks() {
    let mut f = fixture();
    through(&mut f, &[11, 15, 20, 28]);
    let mut e = editor(&f);
    let before = bytes(&e);
    let out = e
        .apply(&EditOp::DeleteSegment {
            track: f.track,
            index: 1,
        })
        .unwrap();
    assert_eq!(e.board().tracks.len(), 2);
    assert_eq!(out.created.len(), 1);
    assert_eq!(e.board().tracks[&f.track].path.len(), 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    e.apply(&EditOp::DeleteSegment {
        track: f.track,
        index: 0,
    })
    .unwrap();
    assert_eq!(e.board().tracks.len(), 1);
    assert_eq!(e.board().tracks[&f.track].path.len(), 2);
    let missing = EditOp::DeleteSegment {
        track: f.track,
        index: 9,
    };
    assert!(e.apply(&missing).is_err());
}

#[test]
fn deleting_a_net_removes_its_tracks_and_vias_only() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&EditOp::Via {
        net: f.sig,
        at: f.pt(20, 20),
        from_layer: None,
        to_layer: None,
    })
    .unwrap();
    e.apply(&EditOp::DeleteNet { net: f.sig }).unwrap();
    assert!(e.board().tracks.is_empty() && e.board().vias.is_empty());
    assert_eq!(e.board().net_of(f.pad(f.r1, f.pad2)), Some(f.sig));
    assert_eq!(
        e.apply(&EditOp::DeleteNet { net: f.gnd }).unwrap_err().code,
        OPERATION_REFUSED
    );
    assert!(e.undo());
    assert_eq!(e.board().tracks.len(), 1);
}

#[test]
fn a_group_is_one_step_and_all_or_nothing() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    let group = EditOp::Group {
        ops: vec![
            gnd_route(&f, vec![f.pt(5, 14), f.pt(15, 14)]),
            gnd_route(&f, vec![f.pt(5, 16), f.pt(15, 16)]),
        ],
    };
    let out = e.apply(&group).unwrap();
    assert_eq!(out.created.len(), 2);
    assert_eq!(e.steps(), 1);
    assert_eq!(e.board().tracks.len(), 3);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    assert!(e.redo());
    assert_eq!(e.board().tracks.len(), 3);
    assert!(e.undo());
    let broken = EditOp::Group {
        ops: vec![
            gnd_route(&f, vec![f.pt(5, 14), f.pt(15, 14)]),
            EditOp::DeleteTrack { track: id(99) },
        ],
    };
    let error = e.apply(&broken).unwrap_err();
    assert!(
        error.detail.starts_with("operation 2 of the group"),
        "{error}"
    );
    assert_eq!(bytes(&e), before);
    assert_eq!(e.steps(), 0);
}

#[test]
fn a_run_of_operations_is_all_or_nothing_and_names_the_one_that_failed() {
    let f = fixture();
    let mut e = editor(&f);
    let before = bytes(&e);
    let ops = vec![
        EditOp::Lock { id: f.r1 },
        gnd_route(&f, vec![f.pt(5, 14), f.pt(15, 14)]),
        EditOp::Remove { id: id(99) },
    ];
    let error = e.apply_all(&ops).unwrap_err();
    assert_eq!(error.code, OPERATION_REFUSED);
    assert!(
        error.detail.starts_with("operation 3 was refused"),
        "{error}"
    );
    assert_eq!(bytes(&e), before);
    assert!(!e.is_locked(f.r1));
    assert_eq!(e.apply_all(&ops[..2]).unwrap().len(), 2);
}

#[test]
fn the_same_operations_always_give_the_same_board() {
    let f = fixture();
    let run = || {
        let mut e = editor(&f);
        e.apply(&gnd_route(&f, vec![f.pt(5, 14), f.pt(15, 20)]))
            .unwrap();
        e.apply(&EditOp::Via {
            net: f.gnd,
            at: f.pt(20, 20),
            from_layer: None,
            to_layer: None,
        })
        .unwrap();
        bytes(&e)
    };
    assert_eq!(run(), run());
}

#[test]
fn the_ratsnest_lists_the_connections_that_have_no_copper() {
    let f = fixture();
    let e = editor(&f);
    let lines = e.ratsnest();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].net, f.gnd);
}

#[test]
fn a_pad_reference_is_the_component_and_the_pad() {
    let f = fixture();
    assert_eq!(
        f.pad(f.r2, f.pad2),
        PadRef {
            component: f.r2,
            pad: f.pad2
        }
    );
}
