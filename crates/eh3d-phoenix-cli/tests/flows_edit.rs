// SPDX-License-Identifier: GPL-3.0-or-later
//! The flow of the editing of a board: place a component, route two tracks, add a via, draw a
//! differential pair and add a copper zone, one operation after another, each under the rules, and
//! then undo every step down to the board that was opened and redo them all. The bytes of the
//! board after each step are kept, so undo and redo must give back exactly those bytes. The
//! expectations are worked out by hand.
//!
//! The board is 60 mm by 40 mm with two resistors R1 and R2 at (10, 10) and (30, 10), the net SIG
//! joined by one track along y = 10, and the net GND on the other pads. So the board starts with
//! 1 track, 2 components, no via and no zone, and each operation adds what the table in the test
//! says.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track,
};
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp, PairConfig, Shape, ZoneProps};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::RuleContext;
use eh3d_phoenix_rules::RuleValues;

const MM: i64 = 1_000_000;

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x * MM, y * MM).unwrap()
}

/// The layers, nets and parts of the board, by the ids the operations name.
struct Ids {
    top: Id,
    bottom: Id,
    sig: Id,
    gnd: Id,
    footprint: Id,
}

fn board() -> (Board, Ids) {
    let ids = Ids {
        top: id(1),
        bottom: id(2),
        sig: id(10),
        gnd: id(11),
        footprint: id(20),
    };
    let mut board = Board {
        name: "flow".to_owned(),
        ..Board::default()
    };
    board.outline = vec![pt(0, 0), pt(60, 0), pt(60, 40), pt(0, 40)];
    for (layer, name, kind, side) in [
        (ids.top, "Top", LayerKind::Copper, Some(Side::Top)),
        (ids.bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
        (id(3), "Silk", LayerKind::Silkscreen, Some(Side::Top)),
    ] {
        board.layers.insert(
            layer,
            Layer {
                name: name.to_owned(),
                kind,
                side,
            },
        );
    }
    board.copper_stack = vec![ids.top, ids.bottom];
    for (net, name) in [(ids.sig, "SIG"), (ids.gnd, "GND")] {
        board.nets.insert(
            net,
            Net {
                name: name.to_owned(),
            },
        );
    }
    let mut pads = BTreeMap::new();
    for (pad, number, x) in [(id(21), "1", -1), (id(22), "2", 1)] {
        pads.insert(
            pad,
            Pad {
                number: number.to_owned(),
                shape: PadShape::Rect {
                    width: Nm::new(MM).unwrap(),
                    height: Nm::new(MM).unwrap(),
                },
                at: Point::from_nm(x * 1_500_000, 0).unwrap(),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![ids.top],
            },
        );
    }
    board.footprints.insert(
        ids.footprint,
        Footprint {
            name: "R".to_owned(),
            pads,
        },
    );
    for (component, refdes, x) in [(id(30), "R1", 10), (id(31), "R2", 30)] {
        board.components.insert(
            component,
            Component {
                refdes: refdes.to_owned(),
                value: "10k".to_owned(),
                symbol: None,
                footprint: ids.footprint,
                at: pt(x, 10),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
    }
    for (component, pad, net) in [
        (id(30), id(21), ids.gnd),
        (id(30), id(22), ids.sig),
        (id(31), id(21), ids.sig),
        (id(31), id(22), ids.gnd),
    ] {
        board.pad_nets.push(PadNet {
            component,
            pad,
            net,
        });
    }
    board.pad_nets.sort_by_key(PadNet::pad_ref);
    board.tracks.insert(
        id(40),
        Track {
            net: ids.sig,
            layer: ids.top,
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: Point::from_nm(11_500_000, 10_000_000).unwrap(),
                end: Point::from_nm(28_500_000, 10_000_000).unwrap(),
            }],
        },
    );
    (board, ids)
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: Nm::new(200_000).unwrap(),
            min_track_width: Nm::new(150_000).unwrap(),
            ..RuleValues::default()
        },
        classes: Default::default(),
    }
}

const PAIR: &str = "pair.p.positive = SIG\npair.p.negative = GND\npair.p.width_nm = 250000\npair.p.gap_nm = 250000\n";

fn editor(board: &Board) -> BoardEdit {
    BoardEdit::new(board.clone(), rules(), BoardEditSettings::default())
        .unwrap()
        .with_pairs(PairConfig::parse(PAIR).unwrap())
}

fn route(ids: &Ids, y: i64) -> EditOp {
    EditOp::Route {
        net: ids.gnd,
        layer: ids.top,
        points: vec![pt(5, y), pt(15, y)],
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: Some(true),
    }
}

/// The operations of the flow, each with its name and what the board holds after it: the number of
/// tracks, components, vias and zones.
fn flow(ids: &Ids) -> Vec<(&'static str, EditOp, [usize; 4])> {
    vec![
        ("route", route(ids, 14), [2, 2, 0, 0]),
        (
            "place",
            EditOp::Place {
                id: None,
                refdes: "R7".to_owned(),
                value: "2k".to_owned(),
                footprint: ids.footprint,
                at: pt(50, 5),
                rotation_udeg: 0,
                side: Some(Side::Top),
            },
            [2, 3, 0, 0],
        ),
        (
            "via",
            EditOp::Via {
                net: ids.gnd,
                at: pt(45, 35),
                from_layer: None,
                to_layer: None,
            },
            [2, 3, 1, 0],
        ),
        (
            "pair",
            EditOp::RoutePair {
                pair: "p".to_owned(),
                layer: ids.top,
                points: vec![pt(10, 20), pt(30, 20)],
                shape: Some(Shape::Orthogonal),
                width_nm: None,
                gap_nm: None,
                horizontal_first: None,
                layer_changes: Vec::new(),
            },
            [4, 3, 1, 0],
        ),
        (
            "zone",
            EditOp::ZoneAdd {
                id: None,
                net: ids.gnd,
                layer: ids.top,
                outline: vec![pt(2, 22), pt(58, 22), pt(58, 38), pt(2, 38)],
                clearance_nm: None,
                props: ZoneProps::default(),
            },
            [4, 3, 1, 1],
        ),
    ]
}

fn counts(board: &Board) -> [usize; 4] {
    [
        board.tracks.len(),
        board.components.len(),
        board.vias.len(),
        board.zones.len(),
    ]
}

#[test]
fn the_editing_of_a_board_is_undone_to_the_same_bytes_and_redone_to_the_same_bytes() {
    let (original, ids) = board();
    let mut edit = editor(&original);
    assert_eq!(
        counts(edit.board()),
        [1, 2, 0, 0],
        "stage 0 (the board that was opened)"
    );
    let mut states = vec![edit.board().to_bytes().unwrap()];
    let steps = flow(&ids);

    // each operation: it is accepted without a new rule error, it changes the board, and the
    // board holds what the table says
    for (name, op, expected) in &steps {
        let accepted = match edit.apply(op) {
            Ok(outcome) => outcome.changed && !outcome.drc.adds_error(),
            Err(error) => panic!("stage {name}: the operation was refused: {error}"),
        };
        let bytes = edit.board().to_bytes().unwrap();
        let changed = states.last() != Some(&bytes);
        assert!(
            accepted && changed && counts(edit.board()) == *expected,
            "stage {name}: accepted {accepted}, changed {changed}, counts {:?}, expected {expected:?}",
            counts(edit.board())
        );
        states.push(bytes);
    }
    assert_eq!(
        edit.steps(),
        steps.len(),
        "stage steps: one step for each operation"
    );

    // undo, one step at a time, down to the board that was opened
    for k in (0..steps.len()).rev() {
        let undone = edit.undo();
        assert!(
            undone && edit.board().to_bytes().unwrap() == states[k],
            "stage undo {} ({}): the bytes are not those of step {k}",
            k + 1,
            steps[k].0
        );
    }
    assert!(
        !edit.can_undo() && edit.can_redo() && edit.board() == &original,
        "stage undo (all): the board that was opened is not back"
    );

    // redo, one step at a time, up to the last
    for (k, (name, _, _)) in steps.iter().enumerate() {
        let redone = edit.redo();
        assert!(
            redone && edit.board().to_bytes().unwrap() == states[k + 1],
            "stage redo {} ({name}): the bytes are not those of step {}",
            k + 1,
            k + 1
        );
    }
    assert!(
        !edit.can_redo(),
        "stage redo (all): there is a step left to redo"
    );
}

#[test]
fn the_same_operations_give_the_same_board_and_the_board_file_reads_back() {
    let (original, ids) = board();
    let run = || {
        let mut edit = editor(&original);
        for (name, op, _) in flow(&ids) {
            if let Err(error) = edit.apply(&op) {
                panic!("stage {name}: the operation was refused: {error}");
            }
        }
        edit.into_board()
    };
    let (first, second) = (run(), run());
    assert_eq!(
        first.to_bytes().unwrap(),
        second.to_bytes().unwrap(),
        "stage determinism: two runs give other bytes"
    );
    let reread = Board::from_bytes(&first.to_bytes().unwrap()).unwrap();
    assert_eq!(reread, first, "stage file: the board read back differs");
    assert_ne!(first, original, "stage change: nothing was edited");
}
