// SPDX-License-Identifier: GPL-3.0-or-later
//! The small questions that the board model answers: where a copper layer sits in the stack,
//! what is wrong with a stack, the net of a pad, the end points and the shape of a piece of track
//! and the width of a silkscreen line, each on a board made here.

use eh3d_phoenix_board::stack::{STACK_ENTRY_INVALID, STACK_INCOMPLETE};
use eh3d_phoenix_board::{
    Board, Id, Layer, LayerKind, Net, PadNet, PadRef, Segment, SilkItem, StackFunction,
};
use eh3d_phoenix_model::{ModelError, Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn p(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn layer(name: &str, kind: LayerKind) -> Layer {
    Layer {
        name: name.to_string(),
        kind,
        side: None,
    }
}

/// Three copper layers and one silkscreen layer, with the ids 1 to 4.
fn board_with_layers() -> Board {
    let mut board = Board::default();
    board.layers.insert(id(1), layer("Top", LayerKind::Copper));
    board
        .layers
        .insert(id(2), layer("Inner", LayerKind::Copper));
    board
        .layers
        .insert(id(3), layer("Bottom", LayerKind::Copper));
    board
        .layers
        .insert(id(4), layer("Silk", LayerKind::Silkscreen));
    board
}

#[test]
fn the_function_of_a_copper_layer_follows_from_its_place_in_the_stack() {
    let mut board = board_with_layers();
    board.copper_stack = vec![id(1), id(2), id(3)];
    assert_eq!(board.stack_position(id(1)), Some((1, StackFunction::Top)));
    assert_eq!(board.stack_position(id(2)), Some((2, StackFunction::Inner)));
    assert_eq!(
        board.stack_position(id(3)),
        Some((3, StackFunction::Bottom))
    );
    // a layer that is not in the stack has no place
    assert_eq!(board.stack_position(id(4)), None);
    assert_eq!(board.stack_position(id(99)), None);
}

#[test]
fn a_stack_of_one_layer_is_its_top_layer_and_of_two_has_no_inner_layer() {
    let mut board = board_with_layers();
    board.copper_stack = vec![id(1)];
    assert_eq!(board.stack_position(id(1)), Some((1, StackFunction::Top)));
    board.copper_stack = vec![id(3), id(1)];
    assert_eq!(board.stack_position(id(3)), Some((1, StackFunction::Top)));
    assert_eq!(
        board.stack_position(id(1)),
        Some((2, StackFunction::Bottom))
    );
    // no stack given: nothing has a place
    board.copper_stack.clear();
    assert_eq!(board.stack_position(id(1)), None);
}

#[test]
fn a_sound_stack_and_an_empty_one_have_no_problem() {
    let mut board = board_with_layers();
    assert!(board.stack_problems().is_empty());
    // the order is free, only the set of layers counts
    board.copper_stack = vec![id(3), id(1), id(2)];
    assert!(board.stack_problems().is_empty());
}

#[test]
fn every_fault_of_a_stack_is_named_with_its_code_in_a_fixed_order() {
    let mut board = board_with_layers();
    // the silkscreen layer is not copper, layer 1 is listed twice, layer 9 does not exist and
    // layers 2 and 3 are left out
    board.copper_stack = vec![id(1), id(4), id(1), id(9)];
    let problems = board.stack_problems();
    let found: Vec<(String, bool, bool, bool, bool)> = problems
        .iter()
        .map(|d| {
            (
                d.code.as_deref().unwrap_or("").to_string(),
                d.message.contains("is not a copper layer"),
                d.message.contains("is listed twice"),
                d.message.contains("does not exist"),
                d.message.contains("is not in the stack"),
            )
        })
        .collect();
    assert_eq!(
        found,
        vec![
            (STACK_ENTRY_INVALID.to_string(), true, false, false, false),
            (STACK_ENTRY_INVALID.to_string(), false, true, false, false),
            (STACK_ENTRY_INVALID.to_string(), false, false, true, false),
            (STACK_INCOMPLETE.to_string(), false, false, false, true),
            (STACK_INCOMPLETE.to_string(), false, false, false, true),
        ]
    );
}

#[test]
fn the_net_of_a_pad_is_found_among_the_sorted_assignments() {
    let mut board = Board::default();
    board.nets.insert(id(50), Net { name: "A".into() });
    board.nets.insert(id(51), Net { name: "B".into() });
    for (component, pad, net) in [(10, 1, 50), (10, 2, 51), (11, 1, 51), (12, 3, 50)] {
        board.pad_nets.push(PadNet {
            component: id(component),
            pad: id(pad + 100),
            net: id(net),
        });
    }
    board.pad_nets.sort_by_key(|p| p.pad_ref());
    let net_of = |component: u16, pad: u16| {
        board.net_of(PadRef {
            component: id(component),
            pad: id(pad + 100),
        })
    };
    assert_eq!(net_of(10, 1), Some(id(50)));
    assert_eq!(net_of(10, 2), Some(id(51)));
    assert_eq!(net_of(11, 1), Some(id(51)));
    assert_eq!(net_of(12, 3), Some(id(50)));
    // a pad with no net, a pad of another component and a component that is not there
    assert_eq!(net_of(11, 2), None);
    assert_eq!(net_of(12, 1), None);
    assert_eq!(net_of(99, 1), None);
    assert_eq!(
        Board::default().net_of(PadRef {
            component: id(1),
            pad: id(2)
        }),
        None
    );
}

#[test]
fn a_piece_of_track_knows_its_ends_and_only_an_arc_has_an_arc() {
    let line = Segment::Line {
        start: p(0, 0),
        end: p(3_000_000, 4_000_000),
    };
    assert_eq!(line.start(), p(0, 0));
    assert_eq!(line.end(), p(3_000_000, 4_000_000));
    assert!(line.to_arc().is_none());
    let sweep = Udeg::new(90_000_000).unwrap();
    let arc = Segment::Arc {
        start: p(0, 0),
        end: p(1_000_000, 1_000_000),
        sweep,
    };
    assert_eq!(arc.start(), p(0, 0));
    assert_eq!(arc.end(), p(1_000_000, 1_000_000));
    let made = arc.to_arc().unwrap().unwrap();
    assert_eq!(made.start, p(0, 0));
    assert_eq!(made.end, p(1_000_000, 1_000_000));
    assert_eq!(made.sweep_udeg, sweep);
    // the chord is the diagonal of a square of 1 mm: 2 * (10^6)^2
    assert_eq!(made.chord_squared(), 2_000_000_000_000);
}

#[test]
fn an_arc_without_a_shape_is_found_when_it_is_asked_for() {
    let none = Segment::Arc {
        start: p(0, 0),
        end: p(1, 1),
        sweep: Udeg::ZERO,
    };
    assert!(matches!(
        none.to_arc(),
        Some(Err(ModelError::DegenerateArc { .. }))
    ));
    // the end points coincide but the arc is not a full turn
    let same = Segment::Arc {
        start: p(5, 5),
        end: p(5, 5),
        sweep: Udeg::new(90_000_000).unwrap(),
    };
    assert!(matches!(
        same.to_arc(),
        Some(Err(ModelError::DegenerateArc { .. }))
    ));
    // a full turn needs the end points to coincide, and a circle has them there
    let circle = Segment::Arc {
        start: p(5, 5),
        end: p(5, 5),
        sweep: Udeg::FULL_TURN,
    };
    assert!(circle.to_arc().unwrap().is_ok());
    let open = Segment::Arc {
        start: p(5, 5),
        end: p(6, 5),
        sweep: Udeg::FULL_TURN,
    };
    assert!(open.to_arc().unwrap().is_err());
}

#[test]
fn the_width_of_a_printed_line_is_the_width_of_its_stroke() {
    let stroke = SilkItem::Stroke {
        layer: id(4),
        width: Nm::new(150_000).unwrap(),
        path: vec![Segment::Line {
            start: p(0, 0),
            end: p(1_000_000, 0),
        }],
    };
    assert_eq!(stroke.line_width().get(), 150_000);
}

#[test]
fn bytes_that_are_not_a_board_are_refused() {
    let mut board = board_with_layers();
    board.copper_stack = vec![id(1), id(2), id(3)];
    let bytes = board.to_bytes().unwrap();
    assert_eq!(Board::from_bytes(&bytes).unwrap(), board);
    // an unknown field is an error, not something to skip
    let text = String::from_utf8(bytes.clone()).unwrap();
    let extra = text.replacen('{', "{\"bogus\":1,", 1);
    assert!(Board::from_bytes(extra.as_bytes()).is_err());
    // half of the bytes, and nothing
    assert!(Board::from_bytes(&bytes[..bytes.len() / 2]).is_err());
    assert!(Board::from_bytes(b"").is_err());
}
