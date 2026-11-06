//! The operation that replaces the outline: its inverse gives back the outline that was there,
//! also when there was none, and it is written and read back as it is.

use eh3d_phoenix_board::{Board, History, Operation};
use eh3d_phoenix_model::Point;

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
}

#[test]
fn the_outline_is_replaced_and_undone_byte_for_byte() {
    let mut board = Board {
        name: "outline".into(),
        ..Board::default()
    };
    let empty = board.to_bytes().unwrap();
    let square = vec![pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];
    let mut history = History::new();
    history
        .execute(
            &mut board,
            Operation::SetOutline {
                outline: square.clone(),
            },
        )
        .unwrap();
    assert_eq!(board.outline, square);
    let triangle = vec![pt(0, 0), pt(20, 0), pt(0, 20)];
    history
        .execute(
            &mut board,
            Operation::SetOutline {
                outline: triangle.clone(),
            },
        )
        .unwrap();
    assert_eq!(board.outline, triangle);
    assert!(history.undo(&mut board));
    assert_eq!(board.outline, square);
    assert!(history.undo(&mut board));
    assert!(board.outline.is_empty());
    assert_eq!(board.to_bytes().unwrap(), empty);
    assert!(history.redo(&mut board));
    assert_eq!(board.outline, square);
}

#[test]
fn the_operation_is_written_and_read_back_the_same() {
    let op = Operation::SetOutline {
        outline: vec![pt(0, 0), pt(1, 0), pt(1, 1)],
    };
    let text = serde_json::to_string(&op).unwrap();
    assert!(
        text.starts_with(r#"{"op":"set_outline","outline":["#),
        "{text}"
    );
    assert_eq!(serde_json::from_str::<Operation>(&text).unwrap(), op);
}
