// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the operations on a group of objects: the offsets are worked out by hand, and every
//! operation is undone to the same bytes.

use eh3d_phoenix_board::{Id, Segment, SilkItem, Via, Zone};
use eh3d_phoenix_model::{Nm, Point, Udeg};

use crate::codes::{ALIGN_TOO_FEW, COMPONENT_LOCKED, ITEM_MISSING, SELECTION_EMPTY};
use crate::edit::BoardEdit;
use crate::group::{align_moves, snap_to_grid, AlignMode, ItemMove};
use crate::ops::EditOp;
use crate::ops_file::{apply_operations, read_operations};
use crate::selection::{Item, RectMode, SelectAction, SelectStep};
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, id, strict_rules, Fixture};

/// An editor with the checks off, so that the tests are about the group operations only.
fn editor(board: eh3d_phoenix_board::Board) -> BoardEdit {
    let settings = BoardEditSettings {
        live_drc: false,
        reject_new_errors: false,
        ..BoardEditSettings::default()
    };
    BoardEdit::new(board, strict_rules(), settings).unwrap()
}

fn bytes(e: &BoardEdit) -> Vec<u8> {
    e.board().to_bytes().unwrap()
}

fn mm(v: i64) -> i64 {
    v * 1_000_000
}

/// The fixture with three vias of `sig` at x = 0, 2 and 10 mm (y = 20), each 0.6 mm wide, a zone
/// of `gnd` from (40, 20) to (50, 30) with a cutout from (42, 22) to (44, 24), and a text and a
/// stroke on the silkscreen layer.
fn busy() -> (Fixture, [Id; 3], Id, Id, Id) {
    let mut f = fixture();
    let vias = [id(50), id(51), id(52)];
    for (via, x) in vias.iter().zip([0, 2, 10]) {
        f.board.vias.insert(
            *via,
            Via {
                net: f.sig,
                at: f.pt(x, 20),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: f.top,
                to_layer: f.bottom,
            },
        );
    }
    let zone = id(60);
    f.board.zones.insert(
        zone,
        Zone {
            net: f.gnd,
            layer: f.top,
            outline: vec![f.pt(40, 20), f.pt(50, 20), f.pt(50, 30), f.pt(40, 30)],
            clearance: Nm::new(200_000).unwrap(),
            cutouts: Vec::new(),
        },
    );
    let silk_layer = id(3);
    let text = id(61);
    f.board.silkscreen.insert(
        text,
        SilkItem::Text {
            layer: silk_layer,
            at: f.pt(5, 30),
            text: "A".to_owned(),
            height: Nm::new(1_000_000).unwrap(),
            stroke: Nm::new(100_000).unwrap(),
            rotation: Udeg::ZERO,
            mirrored: false,
        },
    );
    let stroke = id(62);
    f.board.silkscreen.insert(
        stroke,
        SilkItem::Stroke {
            layer: silk_layer,
            width: Nm::new(200_000).unwrap(),
            path: vec![Segment::Line {
                start: f.pt(20, 30),
                end: f.pt(25, 30),
            }],
        },
    );
    (f, vias, zone, text, stroke)
}

fn select(items: Vec<Item>) -> EditOp {
    EditOp::Select {
        steps: vec![SelectStep::Add { items }],
    }
}

#[test]
fn moving_a_group_changes_every_kind_of_object_and_undo_gives_the_same_bytes() {
    let (f, vias, _zone, text, stroke) = busy();
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    let items = vec![
        Item::Component(f.r1),
        Item::Track(f.track),
        Item::Via(vias[0]),
        Item::Silk(text),
        Item::Silk(stroke),
    ];
    e.apply(&select(items)).unwrap();
    assert_eq!(e.selection().len(), 5);
    // 1.5 mm and -1 mm are multiples of the grid of 0.25 mm: they stay as they are
    let out = e
        .apply(&EditOp::MoveSelection {
            dx_nm: mm(3) / 2,
            dy_nm: -mm(1),
            snap: None,
        })
        .unwrap();
    assert!(out.changed);
    let b = e.board();
    // R1 was at (10, 10)
    assert_eq!(
        b.components[&f.r1].at,
        f.pt(10, 10).shifted(1_500_000, -mm(1))
    );
    // the track ran from (11.5, 10) to (28.5, 10)
    let Segment::Line { start, end } = b.tracks[&f.track].path[0] else {
        panic!("a line");
    };
    assert_eq!(
        (start.x_nm.get(), start.y_nm.get()),
        (13_000_000, 9_000_000)
    );
    assert_eq!((end.x_nm.get(), end.y_nm.get()), (30_000_000, 9_000_000));
    assert_eq!(b.vias[&vias[0]].at, f.pt(0, 20).shifted(1_500_000, -mm(1)));
    let SilkItem::Text { at, .. } = &b.silkscreen[&text] else {
        panic!("a text");
    };
    assert_eq!((at.x_nm.get(), at.y_nm.get()), (6_500_000, 29_000_000));
    let SilkItem::Stroke { path, .. } = &b.silkscreen[&stroke] else {
        panic!("a stroke");
    };
    assert_eq!(path[0].start(), f.pt(20, 30).shifted(1_500_000, -mm(1)));
    // what was not selected stays
    assert_eq!(b.components[&f.r2].at, f.pt(30, 10));
    assert_eq!(b.vias[&vias[1]].at, f.pt(2, 20));
    // the pads keep their nets
    assert_eq!(b.pad_nets, f.board.pad_nets);
    // one step: undo gives the same bytes, redo the same result
    let after = bytes(&e);
    assert_ne!(after, before);
    assert_eq!(e.steps(), 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    assert!(e.redo());
    assert_eq!(bytes(&e), after);
}

trait Shifted {
    fn shifted(self, dx: i64, dy: i64) -> Point;
}

impl Shifted for Point {
    fn shifted(self, dx: i64, dy: i64) -> Point {
        Point::from_nm(self.x_nm.get() + dx, self.y_nm.get() + dy).unwrap()
    }
}

#[test]
fn the_offset_is_rounded_to_the_grid_unless_snapping_is_off() {
    // grid of 0.25 mm = 250 000 nm, halves upwards
    assert_eq!(snap_to_grid(1_130_000, 250_000), 1_250_000);
    assert_eq!(snap_to_grid(1_120_000, 250_000), 1_000_000);
    assert_eq!(snap_to_grid(-130_000, 250_000), -250_000);
    assert_eq!(snap_to_grid(-120_000, 250_000), 0);
    assert_eq!(snap_to_grid(125_000, 250_000), 250_000);
    assert_eq!(snap_to_grid(7, 0), 7);
    let f = fixture();
    let mut e = editor(f.board.clone());
    e.apply(&select(vec![Item::Component(f.r1)])).unwrap();
    e.apply(&EditOp::MoveSelection {
        dx_nm: 1_130_000,
        dy_nm: -130_000,
        snap: None,
    })
    .unwrap();
    // (10, 10) + (1.25, -0.25)
    assert_eq!(
        e.board().components[&f.r1].at,
        f.pt(10, 10).shifted(1_250_000, -250_000)
    );
    // with snapping off for the operation the offset is used as it is
    e.apply(&EditOp::MoveSelection {
        dx_nm: 3,
        dy_nm: 4,
        snap: Some(false),
    })
    .unwrap();
    assert_eq!(
        e.board().components[&f.r1].at,
        f.pt(10, 10).shifted(1_250_003, -249_996)
    );
    // with the setting off the offset is used as it is too
    let mut free = editor(f.board.clone());
    free.settings.snap = false;
    free.apply(&select(vec![Item::Component(f.r2)])).unwrap();
    free.apply(&EditOp::MoveSelection {
        dx_nm: 1,
        dy_nm: 0,
        snap: None,
    })
    .unwrap();
    assert_eq!(
        free.board().components[&f.r2].at,
        f.pt(30, 10).shifted(1, 0)
    );
}

#[test]
fn a_zone_moves_with_its_cutouts_and_loses_its_fill_and_undo_brings_everything_back() {
    let (f, _vias, zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    e.apply(&EditOp::ZoneCutout {
        zone,
        polygon: vec![f.pt(42, 22), f.pt(44, 22), f.pt(44, 24), f.pt(42, 24)],
    })
    .unwrap();
    let before = bytes(&e);
    let data_before = e.zone_data().clone();
    assert_eq!(data_before.props_of(zone).cutouts.len(), 1);
    e.apply(&select(vec![Item::Zone(zone), Item::Component(f.r2)]))
        .unwrap();
    e.apply(&EditOp::MoveSelection {
        dx_nm: -mm(2),
        dy_nm: mm(5),
        snap: None,
    })
    .unwrap();
    let moved = &e.board().zones[&zone];
    assert_eq!(
        moved.outline,
        vec![f.pt(38, 25), f.pt(48, 25), f.pt(48, 35), f.pt(38, 35)]
    );
    assert_eq!(
        e.zone_data().props_of(zone).cutouts,
        vec![vec![f.pt(40, 27), f.pt(42, 27), f.pt(42, 29), f.pt(40, 29)]]
    );
    assert_eq!(e.board().components[&f.r2].at, f.pt(28, 15));
    // the cutout and the move are one step each from the point of view of the person: the move is
    // one step, so one undo brings back the board and the data of the zone
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    assert_eq!(*e.zone_data(), data_before);
}

#[test]
fn deleting_a_group_removes_each_kind_and_undo_gives_the_same_bytes() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    e.apply(&EditOp::Select {
        steps: vec![
            SelectStep::All,
            SelectStep::Remove {
                items: vec![Item::Component(f.r2), Item::Via(vias[2])],
            },
        ],
    })
    .unwrap();
    // 2 components, 1 track, 3 vias, 1 zone and 2 silkscreen items, less 2
    assert_eq!(e.selection().len(), 7);
    e.apply(&EditOp::DeleteSelection).unwrap();
    let b = e.board();
    assert!(b.components.contains_key(&f.r2));
    assert!(!b.components.contains_key(&f.r1));
    assert!(b.tracks.is_empty());
    assert_eq!(b.vias.len(), 1);
    assert!(b.vias.contains_key(&vias[2]));
    assert!(b.zones.is_empty());
    assert!(b.silkscreen.is_empty());
    // the nets of the pads of the deleted component went with it
    assert!(b.pad_nets.iter().all(|p| p.component == f.r2));
    assert_eq!(b.pad_nets.len(), 2);
    // what the board lost is not selected any more
    assert!(e.selection().is_empty());
    assert_eq!(e.steps(), 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn a_locked_component_or_a_missing_object_refuses_the_whole_operation() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    e.apply(&EditOp::Lock { id: f.r1 }).unwrap();
    e.apply(&select(vec![Item::Via(vias[0]), Item::Component(f.r1)]))
        .unwrap();
    for op in [
        EditOp::MoveSelection {
            dx_nm: mm(1),
            dy_nm: 0,
            snap: None,
        },
        EditOp::DeleteSelection,
    ] {
        let error = e.apply(&op).unwrap_err();
        assert_eq!(error.code, COMPONENT_LOCKED);
        assert_eq!(bytes(&e), before);
        assert_eq!(e.steps(), 0);
    }
    let gone = EditOp::DeleteItems {
        items: vec![Item::Via(vias[0]), Item::Track(id(99))],
    };
    assert_eq!(e.apply(&gone).unwrap_err().code, ITEM_MISSING);
    let twice = EditOp::DeleteItems {
        items: vec![Item::Via(vias[0]), Item::Via(vias[0])],
    };
    assert_eq!(e.apply(&twice).unwrap_err().code, ITEM_MISSING);
    assert_eq!(bytes(&e), before);
}

#[test]
fn an_empty_selection_is_an_error_and_so_are_too_few_objects() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    for op in [
        EditOp::MoveSelection {
            dx_nm: 1,
            dy_nm: 1,
            snap: None,
        },
        EditOp::DeleteSelection,
        EditOp::AlignSelection {
            mode: AlignMode::Left,
        },
    ] {
        assert_eq!(e.apply(&op).unwrap_err().code, SELECTION_EMPTY);
    }
    e.apply(&select(vec![Item::Via(vias[0]), Item::Via(vias[1])]))
        .unwrap();
    let error = e
        .apply(&EditOp::AlignSelection {
            mode: AlignMode::DistributeX,
        })
        .unwrap_err();
    assert_eq!(error.code, ALIGN_TOO_FEW);
    e.apply(&EditOp::Select {
        steps: vec![
            SelectStep::Clear,
            SelectStep::Add {
                items: vec![Item::Via(vias[0])],
            },
        ],
    })
    .unwrap();
    let error = e
        .apply(&EditOp::AlignSelection {
            mode: AlignMode::Top,
        })
        .unwrap_err();
    assert_eq!(error.code, ALIGN_TOO_FEW);
}

#[test]
fn the_edges_and_the_centres_line_up_by_hand_worked_offsets() {
    // R1 (extent x 8..12, y 9.5..10.5), the via at (0, 20) (x -0.3..0.3, y 19.7..20.3) and the
    // via at (10, 20) (x 9.7..10.3, y 19.7..20.3): the box of the group is x -0.3..12, y 9.5..20.3
    let (f, vias, _zone, _text, _stroke) = busy();
    let items = [
        Item::Component(f.r1),
        Item::Via(vias[0]),
        Item::Via(vias[2]),
    ];
    let offsets = |mode| -> Vec<(i64, i64)> {
        align_moves(&f.board, &items, mode)
            .unwrap()
            .iter()
            .map(|m| (m.dx_nm, m.dy_nm))
            .collect()
    };
    assert_eq!(
        offsets(AlignMode::Left),
        vec![(-8_300_000, 0), (0, 0), (-10_000_000, 0)]
    );
    assert_eq!(
        offsets(AlignMode::Right),
        vec![
            (0, 0),
            (12_000_000 - 300_000, 0),
            (12_000_000 - 10_300_000, 0)
        ]
    );
    assert_eq!(
        offsets(AlignMode::Top),
        vec![(0, 20_300_000 - 10_500_000), (0, 0), (0, 0)]
    );
    assert_eq!(
        offsets(AlignMode::Bottom),
        vec![
            (0, 0),
            (0, 9_500_000 - 19_700_000),
            (0, 9_500_000 - 19_700_000)
        ]
    );
    // centre of the group x: (-0.3 + 12) / 2 = 5.85; of R1: 10; of the via at 0: 0; of the other: 10
    assert_eq!(
        offsets(AlignMode::CenterX),
        vec![(-4_150_000, 0), (5_850_000, 0), (-4_150_000, 0)]
    );
    // centre of the group y: (9.5 + 20.3) / 2 = 14.9; of R1: 10; of the vias: 20
    assert_eq!(
        offsets(AlignMode::CenterY),
        vec![(0, 4_900_000), (0, -5_100_000), (0, -5_100_000)]
    );
}

#[test]
fn distributing_gives_equal_gaps_and_leaves_the_ends_where_they_are() {
    // vias of 0.6 mm at x = 0, 2 and 10: extents 0..0.6 wide around the centres, so
    // -0.3..0.3, 1.7..2.3 and 9.7..10.3. Span 10.6 minus three widths of 0.6 is 8.8 free, so the
    // gap is 4.4 and the middle one goes to start at 0.3 + 4.4 = 4.7 (centre 5.0): it moves by 3
    let (f, vias, _zone, _text, _stroke) = busy();
    let moves = align_moves(
        &f.board,
        &[Item::Via(vias[2]), Item::Via(vias[0]), Item::Via(vias[1])],
        AlignMode::DistributeX,
    )
    .unwrap();
    let by = |v: Id| {
        moves
            .iter()
            .find(|m| m.item == Item::Via(v))
            .map(|m| (m.dx_nm, m.dy_nm))
            .unwrap()
    };
    assert_eq!(by(vias[0]), (0, 0));
    assert_eq!(by(vias[1]), (3_000_000, 0));
    assert_eq!(by(vias[2]), (0, 0));
    // applied through the editor, undone to the same bytes
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    e.apply(&select(vec![
        Item::Via(vias[0]),
        Item::Via(vias[1]),
        Item::Via(vias[2]),
    ]))
    .unwrap();
    e.apply(&EditOp::AlignSelection {
        mode: AlignMode::DistributeX,
    })
    .unwrap();
    assert_eq!(e.board().vias[&vias[1]].at, f.pt(5, 20));
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn aligning_through_the_editor_is_one_step() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    e.apply(&select(vec![
        Item::Via(vias[0]),
        Item::Via(vias[1]),
        Item::Component(f.r1),
    ]))
    .unwrap();
    e.apply(&EditOp::AlignSelection {
        mode: AlignMode::Right,
    })
    .unwrap();
    // the group ends at x = 12 (R1); the vias end at 0.3 and 2.3 so they go 11.7 and 9.7 right
    assert_eq!(
        e.board().vias[&vias[0]].at,
        f.pt(0, 20).shifted(11_700_000, 0)
    );
    assert_eq!(
        e.board().vias[&vias[1]].at,
        f.pt(2, 20).shifted(9_700_000, 0)
    );
    assert_eq!(e.board().components[&f.r1].at, f.pt(10, 10));
    assert_eq!(e.steps(), 1);
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
}

#[test]
fn explicit_moves_go_each_by_its_own_offset() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let before = bytes(&e);
    e.apply(&EditOp::MoveItems {
        moves: vec![
            ItemMove {
                item: Item::Via(vias[0]),
                dx_nm: 1,
                dy_nm: 2,
            },
            ItemMove {
                item: Item::Via(vias[1]),
                dx_nm: -3,
                dy_nm: 0,
            },
            ItemMove {
                item: Item::Via(vias[2]),
                dx_nm: 0,
                dy_nm: 0,
            },
        ],
    })
    .unwrap();
    assert_eq!(e.board().vias[&vias[0]].at, f.pt(0, 20).shifted(1, 2));
    assert_eq!(e.board().vias[&vias[1]].at, f.pt(2, 20).shifted(-3, 0));
    assert_eq!(e.board().vias[&vias[2]].at, f.pt(10, 20));
    assert!(e.undo());
    assert_eq!(bytes(&e), before);
    // an offset that leaves the range of the design is refused and changes nothing
    let far = EditOp::MoveItems {
        moves: vec![ItemMove {
            item: Item::Via(vias[0]),
            dx_nm: i64::MAX,
            dy_nm: 0,
        }],
    };
    assert!(e.apply(&far).is_err());
    assert_eq!(bytes(&e), before);
}

#[test]
fn the_selection_by_a_net_and_by_a_rectangle_then_a_move_carries_the_copper_along() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let out = e
        .apply(&EditOp::Select {
            steps: vec![
                SelectStep::Net {
                    net: f.sig,
                    action: SelectAction::Add,
                },
                SelectStep::Rect {
                    from: f.pt(-1, 19),
                    to: f.pt(3, 21),
                    mode: RectMode::Contained,
                    action: SelectAction::Remove,
                },
            ],
        })
        .unwrap();
    // sig: R1, R2, the track and three vias; the rectangle takes two vias away
    assert_eq!(out.notes, vec!["selected objects: 4".to_owned()]);
    assert!(e.selection().contains(Item::Via(vias[2])));
    assert!(!e.selection().contains(Item::Via(vias[0])));
    e.apply(&EditOp::MoveSelection {
        dx_nm: 0,
        dy_nm: mm(5),
        snap: None,
    })
    .unwrap();
    // R1 moved with the track that was on its pad: its end is still on the pad
    assert_eq!(e.board().components[&f.r1].at, f.pt(10, 15));
    let Segment::Line { start, .. } = e.board().tracks[&f.track].path[0] else {
        panic!("a line");
    };
    assert_eq!(start, f.pt(11, 10).shifted(500_000, mm(5)));
}

#[test]
fn a_failed_selection_step_changes_nothing_and_names_the_step() {
    let (f, _vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    e.apply(&select(vec![Item::Component(f.r1)])).unwrap();
    let error = e
        .apply(&EditOp::Select {
            steps: vec![
                SelectStep::All,
                SelectStep::Add {
                    items: vec![Item::Zone(id(99))],
                },
            ],
        })
        .unwrap_err();
    assert_eq!(error.code, ITEM_MISSING);
    assert!(
        error.detail.starts_with("step 2 of the selection"),
        "{error}"
    );
    assert_eq!(e.selection().items(), &[Item::Component(f.r1)][..]);
}

#[test]
fn undo_and_a_removal_drop_the_objects_the_board_lost_from_the_selection() {
    let (f, _vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let out = e
        .apply(&EditOp::Route {
            net: f.gnd,
            layer: f.top,
            points: vec![f.pt(5, 14), f.pt(15, 14)],
            shape: None,
            width_nm: None,
            horizontal_first: None,
        })
        .unwrap();
    let made = out.created[0];
    e.apply(&select(vec![Item::Track(made)])).unwrap();
    assert!(e.undo());
    assert!(e.selection().is_empty());
}

#[test]
fn the_file_of_operations_selects_moves_and_deletes_and_the_report_counts() {
    let (f, vias, _zone, _text, _stroke) = busy();
    let mut e = editor(f.board.clone());
    let text = format!(
        r#"[
  {{"op":"select","steps":[{{"select":"net","net":"{sig}"}},{{"select":"remove","items":[{{"kind":"component","id":"{r2}"}}]}}]}},
  {{"op":"move_selection","dx_nm":1000000,"dy_nm":0,"snap":false}},
  {{"op":"select","steps":[{{"select":"clear"}},{{"select":"add","items":[{{"kind":"via","id":"{v0}"}},{{"kind":"via","id":"{v1}"}}]}}]}},
  {{"op":"align_selection","mode":"left"}},
  {{"op":"delete_selection"}}
]"#,
        sig = f.sig,
        r2 = f.r2,
        v0 = vias[0],
        v1 = vias[1]
    );
    let ops = read_operations(&text).unwrap();
    assert_eq!(ops.len(), 5);
    let report = apply_operations(&mut e, &ops).unwrap();
    assert_eq!(report.applied, 5);
    // sig has R1, R2, the track and three vias; R2 is taken away; the 5 remaining move by 1 mm;
    // the two vias are aligned to the left (the via at x = 1 and the one at 3: the second goes 2
    // left) and deleted
    assert_eq!(report.vias, (3, 1));
    assert_eq!(report.components, (2, 2));
    assert_eq!(report.tracks, (1, 1));
    assert_eq!(e.board().vias.len(), 1);
    assert!(e.board().vias.contains_key(&vias[2]));
    assert_eq!(e.board().components[&f.r1].at, f.pt(11, 10));
    assert!(report.notes.iter().any(|n| n == "selected objects: 5"));
    // a bad file is named
    assert!(read_operations(r#"[{"op":"align_selection","mode":"diagonal"}]"#).is_err());
    assert!(read_operations(r#"[{"op":"select","steps":[{"select":"fly"}]}]"#).is_err());
}
