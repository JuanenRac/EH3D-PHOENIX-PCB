// SPDX-License-Identifier: GPL-3.0-or-later
use eh3d_phoenix_board::{Id, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point};

use crate::codes::{COMPONENT_LOCKED, GEOMETRY_INVALID, OPERATION_REFUSED};
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, id, strict_rules, Fixture};
use crate::tidy::tidied_path;

fn editor(f: &Fixture) -> BoardEdit {
    BoardEdit::new(
        f.board.clone(),
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
}

fn p(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
}

fn line(a: Point, b: Point) -> Segment {
    Segment::Line { start: a, end: b }
}

fn code(e: &mut BoardEdit, op: &EditOp) -> &'static str {
    e.apply(op).unwrap_err().code
}

fn with_via(f: &mut Fixture) -> Id {
    let via = id(50);
    f.board.vias.insert(
        via,
        Via {
            net: f.sig,
            at: p(20, 20),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: f.top,
            to_layer: f.bottom,
        },
    );
    via
}

fn track_on(f: &mut Fixture, n: u16, path: Vec<Segment>) -> Id {
    let t = id(n);
    f.board.tracks.insert(
        t,
        Track {
            net: f.sig,
            layer: f.bottom,
            width: Nm::new(250_000).unwrap(),
            path,
        },
    );
    t
}

#[test]
fn the_width_of_a_track_changes_and_undoes() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&EditOp::SetTrackWidth {
        track: f.track,
        width_nm: 400_000,
    })
    .unwrap();
    assert_eq!(e.board().tracks[&f.track].width.get(), 400_000);
    assert!(e.undo());
    assert_eq!(e.board().tracks[&f.track].width.get(), 250_000);
    for (track, width_nm, expected) in [
        (f.track, 250_000, OPERATION_REFUSED),
        (f.track, 0, GEOMETRY_INVALID),
        (id(999), 300_000, OPERATION_REFUSED),
    ] {
        assert_eq!(
            code(&mut e, &EditOp::SetTrackWidth { track, width_nm }),
            expected
        );
    }
}

#[test]
fn a_via_moves_and_is_resized_as_one_step_each() {
    let mut f = fixture();
    let via = with_via(&mut f);
    let mut e = editor(&f);
    let steps = e.steps();
    e.apply(&EditOp::MoveVia { via, to: p(25, 25) }).unwrap();
    assert_eq!(e.board().vias[&via].at, p(25, 25));
    e.apply(&EditOp::ResizeVia {
        via,
        drill_nm: 350_000,
        diameter_nm: 700_000,
    })
    .unwrap();
    assert_eq!(e.board().vias[&via].diameter.get(), 700_000);
    assert_eq!(e.steps(), steps + 2);
    assert!(e.undo() && e.undo());
    assert_eq!(e.board().vias[&via].at, p(20, 20));
    assert_eq!(e.board().vias[&via].drill.get(), 300_000);
    assert_eq!(
        code(&mut e, &EditOp::MoveVia { via, to: p(20, 20) }),
        OPERATION_REFUSED
    );
    assert_eq!(
        code(
            &mut e,
            &EditOp::ResizeVia {
                via,
                drill_nm: 500_000,
                diameter_nm: 500_000
            }
        ),
        GEOMETRY_INVALID
    );
}

#[test]
fn a_track_is_cut_after_a_piece_and_joined_again() {
    let mut f = fixture();
    let t = track_on(
        &mut f,
        60,
        vec![
            line(p(5, 30), p(10, 30)),
            line(p(10, 30), p(10, 35)),
            line(p(10, 35), p(20, 35)),
        ],
    );
    let mut e = editor(&f);
    let before = e.board().tracks.len();
    let out = e.apply(&EditOp::SplitTrack { track: t, piece: 0 }).unwrap();
    assert_eq!(e.board().tracks.len(), before + 1);
    assert_eq!(e.board().tracks[&t].path.len(), 1);
    let new = *out.created.first().unwrap();
    assert_eq!(e.board().tracks[&new].path.len(), 2);
    e.apply(&EditOp::JoinTracks { a: t, b: new }).unwrap();
    assert_eq!(e.board().tracks.len(), before);
    assert_eq!(e.board().tracks[&t].path.len(), 3);
    for piece in [2, 9] {
        assert_eq!(
            code(&mut e, &EditOp::SplitTrack { track: t, piece }),
            OPERATION_REFUSED
        );
    }
}

#[test]
fn a_join_turns_a_track_round_when_the_ends_need_it_and_refuses_what_does_not_meet() {
    let mut f = fixture();
    let a = track_on(&mut f, 61, vec![line(p(5, 30), p(10, 30))]);
    let b = track_on(&mut f, 62, vec![line(p(15, 30), p(10, 30))]);
    let far = track_on(&mut f, 63, vec![line(p(40, 5), p(45, 5))]);
    let mut e = editor(&f);
    assert_eq!(
        code(&mut e, &EditOp::JoinTracks { a, b: far }),
        OPERATION_REFUSED
    );
    assert_eq!(
        code(&mut e, &EditOp::JoinTracks { a, b: a }),
        OPERATION_REFUSED
    );
    e.apply(&EditOp::JoinTracks { a, b }).unwrap();
    let path = &e.board().tracks[&a].path;
    assert_eq!((path[0].start(), path[1].end()), (p(5, 30), p(15, 30)));
    assert_eq!(path[0].end(), path[1].start());
}

#[test]
fn tidy_drops_pieces_of_no_length_and_merges_straight_pieces() {
    let path = vec![
        line(p(0, 0), p(5, 0)),
        line(p(5, 0), p(5, 0)),
        line(p(5, 0), p(9, 0)),
        line(p(9, 0), p(9, 4)),
        line(p(9, 4), p(9, 2)),
    ];
    let tidy = tidied_path(&path);
    assert_eq!(
        tidy,
        vec![
            line(p(0, 0), p(9, 0)),
            line(p(9, 0), p(9, 4)),
            line(p(9, 4), p(9, 2))
        ]
    );
    assert_eq!(tidied_path(&tidy), tidy);
    let mut f = fixture();
    let t = track_on(&mut f, 64, path);
    let mut e = editor(&f);
    e.apply(&EditOp::TidyTracks { net: Some(f.sig) }).unwrap();
    assert_eq!(e.board().tracks[&t].path.len(), 3);
    assert_eq!(
        code(&mut e, &EditOp::TidyTracks { net: None }),
        OPERATION_REFUSED
    );
    assert_eq!(
        code(&mut e, &EditOp::TidyTracks { net: Some(id(999)) }),
        OPERATION_REFUSED
    );
}

#[test]
fn tidy_keeps_the_ends_of_random_tracks_and_is_idempotent() {
    let mut seed: u64 = 0x5eed_0001;
    let mut next = |m: i64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) as i64) % m
    };
    for _ in 0..200 {
        let mut at = p(next(20), next(20));
        let start = at;
        let mut path = Vec::new();
        for _ in 0..(1 + next(8)) {
            let (dx, dy) = match next(4) {
                0 => (next(5), 0),
                1 => (0, next(5)),
                2 => (-next(5), 0),
                _ => (0, 0),
            };
            let to = p(
                at.x_nm.get() / 1_000_000 + dx,
                at.y_nm.get() / 1_000_000 + dy,
            );
            path.push(line(at, to));
            at = to;
        }
        let tidy = tidied_path(&path);
        assert_eq!(tidied_path(&tidy), tidy);
        if let (Some(first), Some(last)) = (tidy.first(), tidy.last()) {
            assert_eq!(first.start(), start);
            assert_eq!(last.end(), at);
        } else {
            assert_eq!(start, at);
        }
        assert!(tidy.iter().all(|s| s.start() != s.end()));
    }
}

#[test]
fn value_designator_and_net_name_change_and_refuse_a_clash() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&EditOp::SetValue {
        component: f.r1,
        value: "4k7".to_owned(),
    })
    .unwrap();
    assert_eq!(e.board().components[&f.r1].value, "4k7");
    e.apply(&EditOp::SetRefdes {
        component: f.r1,
        refdes: "R10".to_owned(),
    })
    .unwrap();
    assert_eq!(e.board().components[&f.r1].refdes, "R10");
    e.apply(&EditOp::RenameNet {
        net: f.sig,
        name: "DATA".to_owned(),
    })
    .unwrap();
    assert_eq!(e.board().nets[&f.sig].name, "DATA");
    for op in [
        EditOp::SetRefdes {
            component: f.r1,
            refdes: "R2".to_owned(),
        },
        EditOp::SetRefdes {
            component: f.r1,
            refdes: " ".to_owned(),
        },
        EditOp::RenameNet {
            net: f.sig,
            name: "GND".to_owned(),
        },
        EditOp::SetValue {
            component: f.r1,
            value: "4k7".to_owned(),
        },
    ] {
        assert_eq!(code(&mut e, &op), OPERATION_REFUSED);
    }
    e.apply(&EditOp::Lock { id: f.r2 }).unwrap();
    assert_eq!(
        code(
            &mut e,
            &EditOp::SetValue {
                component: f.r2,
                value: "1".to_owned()
            }
        ),
        COMPONENT_LOCKED
    );
}

#[test]
fn merging_two_nets_moves_pads_tracks_and_vias_in_one_step() {
    let mut f = fixture();
    let via = with_via(&mut f);
    let mut e = editor(&f);
    let steps = e.steps();
    e.apply(&EditOp::MergeNets {
        from: f.sig,
        into: f.gnd,
    })
    .unwrap();
    assert!(!e.board().nets.contains_key(&f.sig));
    assert_eq!(e.board().tracks[&f.track].net, f.gnd);
    assert_eq!(e.board().vias[&via].net, f.gnd);
    assert_eq!(e.board().net_of(f.pad(f.r1, f.pad2)), Some(f.gnd));
    assert_eq!(e.steps(), steps + 1);
    assert!(e.undo());
    assert!(e.board().nets.contains_key(&f.sig));
    assert_eq!(e.board().tracks[&f.track].net, f.sig);
    assert_eq!(
        code(
            &mut e,
            &EditOp::MergeNets {
                from: f.sig,
                into: f.sig
            }
        ),
        OPERATION_REFUSED
    );
}

#[test]
fn components_snap_to_the_nearest_point_of_the_grid() {
    let mut f = fixture();
    f.board.components.get_mut(&f.r1).unwrap().at = Point::from_nm(10_260_000, 9_740_000).unwrap();
    let mut e = editor(&f);
    e.apply(&EditOp::SnapToGrid {
        components: vec![f.r1, f.r2],
        grid_nm: 500_000,
    })
    .unwrap();
    assert_eq!(
        e.board().components[&f.r1].at,
        Point::from_nm(10_500_000, 9_500_000).unwrap()
    );
    assert_eq!(e.board().components[&f.r2].at, p(30, 10));
    assert_eq!(
        code(
            &mut e,
            &EditOp::SnapToGrid {
                components: vec![f.r1],
                grid_nm: 500_000
            }
        ),
        OPERATION_REFUSED
    );
    assert_eq!(
        code(
            &mut e,
            &EditOp::SnapToGrid {
                components: vec![f.r1],
                grid_nm: 0
            }
        ),
        GEOMETRY_INVALID
    );
}

#[test]
fn the_new_operations_are_written_and_read_in_the_file_of_operations() {
    let f = fixture();
    let ops = vec![
        EditOp::SetTrackWidth {
            track: f.track,
            width_nm: 300_000,
        },
        EditOp::MoveVia {
            via: id(50),
            to: p(1, 2),
        },
        EditOp::ResizeVia {
            via: id(50),
            drill_nm: 300_000,
            diameter_nm: 600_000,
        },
        EditOp::SplitTrack {
            track: f.track,
            piece: 0,
        },
        EditOp::SplitTrackAt {
            track: f.track,
            at: p(20, 10),
        },
        EditOp::JoinTracks {
            a: f.track,
            b: id(61),
        },
        EditOp::TidyTracks { net: None },
        EditOp::TidyTracks { net: Some(f.sig) },
        EditOp::SetValue {
            component: f.r1,
            value: "1k".to_owned(),
        },
        EditOp::SetRefdes {
            component: f.r1,
            refdes: "R9".to_owned(),
        },
        EditOp::RenameNet {
            net: f.sig,
            name: "DATA".to_owned(),
        },
        EditOp::MergeNets {
            from: f.sig,
            into: f.gnd,
        },
        EditOp::SnapToGrid {
            components: vec![f.r1, f.r2],
            grid_nm: 500_000,
        },
    ];
    for op in ops {
        let text = serde_json::to_string(&op).unwrap();
        let back: EditOp = serde_json::from_str(&text).unwrap();
        assert_eq!(back, op, "{text}");
    }
    let from_text: EditOp = serde_json::from_str(r#"{"op": "tidy_tracks"}"#).unwrap();
    assert_eq!(from_text, EditOp::TidyTracks { net: None });
    assert!(serde_json::from_str::<EditOp>(r#"{"op": "set_track_width", "track": "x"}"#).is_err());
}
