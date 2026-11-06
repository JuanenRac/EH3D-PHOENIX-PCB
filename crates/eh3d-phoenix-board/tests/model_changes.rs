//! The openings inside the outline, the rounded corners and the plating of a pad, and the cutouts
//! of a zone: they are optional fields, so a board written before them reads as before and a board
//! that does not use them writes the same bytes as before.

use std::collections::BTreeMap;

use eh3d_phoenix_board::validate::BAD_OUTLINE;
use eh3d_phoenix_board::{
    Board, Footprint, History, Id, Layer, LayerKind, Operation, Pad, PadShape, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn pad(shape: PadShape, roundness: u16, plated: bool, drill: Option<i64>) -> Pad {
    Pad {
        number: "1".into(),
        shape,
        at: pt(0, 0),
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers: vec![id(1)],
        roundness,
        plated,
    }
}

/// A board of 20 by 10 mm with a copper layer, a footprint of one pad and a zone.
fn board(pad: Pad) -> Board {
    let mut b = Board {
        name: "model".into(),
        outline: vec![pt(0, 0), pt(20, 0), pt(20, 10), pt(0, 10)],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    let mut pads = BTreeMap::new();
    pads.insert(id(10), pad);
    b.footprints.insert(
        id(11),
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    b
}

fn plain_pad() -> Pad {
    pad(
        PadShape::Rect {
            width: nm(2_000_000),
            height: nm(1_000_000),
        },
        0,
        true,
        None,
    )
}

#[test]
fn a_board_that_uses_none_of_the_new_fields_writes_none_of_them() {
    let b = board(plain_pad());
    let text = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    for word in ["roundness", "plated", "outline_cutouts", "cutouts"] {
        assert!(!text.contains(word), "{word} in {text}");
    }
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), b);
}

#[test]
fn the_new_fields_are_written_when_they_are_used_and_read_back() {
    let mut b = board(pad(
        PadShape::Rect {
            width: nm(2_000_000),
            height: nm(1_000_000),
        },
        500,
        false,
        Some(300_000),
    ));
    b.outline_cutouts = vec![vec![pt(5, 3), pt(8, 3), pt(8, 6), pt(5, 6)]];
    let text = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    for word in ["roundness", "plated", "outline_cutouts"] {
        assert!(text.contains(word), "{word} missing in {text}");
    }
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), b);
    // a board with a zone that has a cutout
    let mut z = board(plain_pad());
    z.zones.insert(
        id(20),
        Zone {
            net: id(21),
            layer: id(1),
            outline: vec![pt(0, 0), pt(20, 0), pt(20, 10), pt(0, 10)],
            clearance: nm(200_000),
            cutouts: vec![vec![pt(2, 2), pt(4, 2), pt(4, 4)]],
        },
    );
    let text = String::from_utf8(z.to_bytes().unwrap()).unwrap();
    assert!(text.contains("cutouts"));
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), z);
}

#[test]
fn the_radius_of_a_rounded_pad_follows_its_shorter_side() {
    // 2 mm by 1 mm: roundness 500 is a radius of a quarter of the shorter side, 1000 a half of it
    let quarter = pad(
        PadShape::Rect {
            width: nm(2_000_000),
            height: nm(1_000_000),
        },
        500,
        true,
        None,
    );
    assert_eq!(quarter.corner_radius_nm(), Some(250_000));
    let full = Pad {
        roundness: 1000,
        ..quarter.clone()
    };
    assert_eq!(full.corner_radius_nm(), Some(500_000));
    // square corners, and shapes that are not a rectangle, have no radius
    assert_eq!(plain_pad().corner_radius_nm(), None);
    let circle = pad(
        PadShape::Circle {
            diameter: nm(1_000_000),
        },
        500,
        true,
        None,
    );
    assert_eq!(circle.corner_radius_nm(), None);
    // only a drilled pad that is not plated is a hole without copper
    assert!(!quarter.is_npth());
    assert!(pad(
        PadShape::Circle {
            diameter: nm(1_000_000)
        },
        0,
        false,
        Some(500_000)
    )
    .is_npth());
    assert!(!pad(
        PadShape::Circle {
            diameter: nm(1_000_000)
        },
        0,
        false,
        None
    )
    .is_npth());
}

#[test]
fn the_cutouts_of_the_outline_are_replaced_and_undone_byte_for_byte() {
    let mut b = board(plain_pad());
    let before = b.to_bytes().unwrap();
    let hole = vec![pt(5, 3), pt(8, 3), pt(8, 6), pt(5, 6)];
    let mut history = History::new();
    history
        .execute(
            &mut b,
            Operation::SetOutlineCutouts {
                cutouts: vec![hole.clone()],
            },
        )
        .unwrap();
    assert_eq!(b.outline_cutouts, vec![hole]);
    assert!(b.validate().is_empty(), "{:?}", b.validate());
    assert!(history.undo(&mut b));
    assert!(b.outline_cutouts.is_empty());
    assert_eq!(b.to_bytes().unwrap(), before);
    let text = serde_json::to_string(&Operation::SetOutlineCutouts { cutouts: vec![] }).unwrap();
    assert!(text.starts_with(r#"{"op":"set_outline_cutouts""#), "{text}");
}

#[test]
fn a_cutout_that_crosses_itself_or_a_pad_without_a_drill_that_is_not_plated_is_reported() {
    let mut b = board(plain_pad());
    // a bow tie
    b.outline_cutouts = vec![vec![pt(5, 3), pt(8, 6), pt(8, 3), pt(5, 6)]];
    let codes: Vec<String> = b
        .validate()
        .iter()
        .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
        .collect();
    assert_eq!(codes, vec![BAD_OUTLINE.to_owned()]);
    // a pad that is not plated has to have a hole
    let mut c = board(pad(
        PadShape::Circle {
            diameter: nm(1_000_000),
        },
        0,
        false,
        None,
    ));
    c.outline_cutouts = Vec::new();
    assert_eq!(c.validate().len(), 1);
    // a roundness above the largest value
    let d = board(pad(
        PadShape::Rect {
            width: nm(2_000_000),
            height: nm(1_000_000),
        },
        1001,
        true,
        None,
    ));
    assert_eq!(d.validate().len(), 1);
}
