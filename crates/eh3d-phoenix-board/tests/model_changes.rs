//! The openings inside the outline, the rounded corners and the plating of a pad, and the cutouts
//! of a zone: they are optional fields, so a board written before them reads as before and a board
//! that does not use them writes the same bytes as before.

use std::collections::BTreeMap;

use eh3d_phoenix_board::validate::{BAD_LAYER, BAD_OUTLINE, BAD_SIZE, DANGLING, PAD_NET_CONFLICT};
use eh3d_phoenix_board::{
    Board, Component, Dimension, DimensionKind, Footprint, History, Id, Layer, LayerKind, OpError,
    Operation, Pad, PadRef, PadShape, Side, Zone,
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

/// The board of the fixture with one component, J1, of its footprint.
fn with_component() -> Board {
    let mut b = board(plain_pad());
    b.components.insert(
        id(30),
        Component {
            refdes: "J1".into(),
            value: String::new(),
            symbol: None,
            footprint: id(11),
            at: pt(5, 5),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b
}

fn mark() -> PadRef {
    PadRef {
        component: id(30),
        pad: id(10),
    }
}

#[test]
fn the_test_point_marks_are_written_only_when_there_are_some_and_read_back() {
    let mut b = with_component();
    let plain = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("test_points"), "{plain}");
    assert!(!b.is_test_point(mark()));
    assert!(b.mark_test_point(mark()));
    assert!(!b.mark_test_point(mark()));
    assert!(b.is_test_point(mark()));
    assert!(b.validate().is_empty(), "{:?}", b.validate());
    let text = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    assert!(text.contains("test_points"), "{text}");
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), b);
    assert!(b.unmark_test_point(mark()));
    assert!(!b.unmark_test_point(mark()));
    assert_eq!(b.to_bytes().unwrap(), plain.as_bytes());
}

#[test]
fn the_marks_stay_sorted_by_pad() {
    let mut b = with_component();
    let second = PadRef {
        component: id(30),
        pad: id(9),
    };
    b.footprints
        .get_mut(&id(11))
        .unwrap()
        .pads
        .insert(id(9), plain_pad());
    assert!(b.mark_test_point(mark()));
    assert!(b.mark_test_point(second));
    assert_eq!(b.test_points, vec![second, mark()]);
    assert!(b.validate().is_empty(), "{:?}", b.validate());
}

#[test]
fn marking_a_pad_is_an_operation_with_an_exact_inverse() {
    let mut b = with_component();
    let before = b.to_bytes().unwrap();
    let mut history = History::new();
    history
        .execute(
            &mut b,
            Operation::SetTestPoint {
                pad: mark(),
                marked: true,
            },
        )
        .unwrap();
    assert!(b.is_test_point(mark()));
    // the same mark again changes nothing and undoes to the state before it
    let marked_bytes = b.to_bytes().unwrap();
    history
        .execute(
            &mut b,
            Operation::SetTestPoint {
                pad: mark(),
                marked: true,
            },
        )
        .unwrap();
    assert_eq!(b.to_bytes().unwrap(), marked_bytes);
    assert!(history.undo(&mut b));
    assert_eq!(b.to_bytes().unwrap(), marked_bytes);
    assert!(history.undo(&mut b));
    assert_eq!(b.to_bytes().unwrap(), before);
    let text = serde_json::to_string(&Operation::SetTestPoint {
        pad: mark(),
        marked: false,
    })
    .unwrap();
    assert!(text.starts_with(r#"{"op":"set_test_point""#), "{text}");
}

#[test]
fn a_mark_on_a_pad_that_does_not_exist_is_refused() {
    let mut b = with_component();
    let no_pad = PadRef {
        component: id(30),
        pad: id(99),
    };
    let no_component = PadRef {
        component: id(98),
        pad: id(10),
    };
    for (pad, expected) in [(no_pad, id(99)), (no_component, id(98))] {
        let err = b
            .apply(&Operation::SetTestPoint { pad, marked: true })
            .unwrap_err();
        assert_eq!(err, OpError::Missing(expected));
    }
    assert!(b.test_points.is_empty());
}

#[test]
fn removing_a_component_takes_its_marks_and_undo_gives_them_back() {
    let mut b = with_component();
    b.mark_test_point(mark());
    let before = b.to_bytes().unwrap();
    let mut history = History::new();
    history
        .execute(&mut b, Operation::RemoveComponent { id: id(30) })
        .unwrap();
    assert!(b.test_points.is_empty());
    assert!(b.validate().is_empty(), "{:?}", b.validate());
    assert!(history.undo(&mut b));
    assert_eq!(b.to_bytes().unwrap(), before);
}

#[test]
fn a_mark_that_repeats_or_names_a_missing_pad_is_reported() {
    let mut b = with_component();
    b.test_points = vec![mark(), mark()];
    let codes: Vec<String> = b
        .validate()
        .iter()
        .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
        .collect();
    assert_eq!(codes, vec![PAD_NET_CONFLICT.to_owned()]);
    b.test_points = vec![PadRef {
        component: id(30),
        pad: id(99),
    }];
    let codes: Vec<String> = b
        .validate()
        .iter()
        .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
        .collect();
    assert_eq!(codes, vec![DANGLING.to_owned()]);
}

/// The board of the fixture with a documentation layer, id 2, and a dimension of 10 mm on it.
fn with_dimension() -> (Board, Dimension) {
    let mut b = board(plain_pad());
    b.layers.insert(
        id(2),
        Layer {
            name: "Dimensions".into(),
            kind: LayerKind::Documentation,
            side: None,
        },
    );
    let d = Dimension {
        layer: id(2),
        kind: DimensionKind::Horizontal,
        from: pt(2, 2),
        to: pt(12, 2),
        offset: nm(2_000_000),
        text_height: nm(1_000_000),
        stroke: nm(100_000),
    };
    (b, d)
}

fn problems(b: &Board) -> Vec<String> {
    b.validate()
        .iter()
        .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
        .collect()
}

#[test]
fn the_dimensions_are_written_only_when_there_are_some_and_read_back() {
    let (mut b, d) = with_dimension();
    let plain = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("dimensions"), "{plain}");
    b.dimensions.insert(id(60), d);
    assert!(b.validate().is_empty(), "{:?}", b.validate());
    let text = String::from_utf8(b.to_bytes().unwrap()).unwrap();
    assert!(text.contains("\"dimensions\""), "{text}");
    assert!(text.contains("\"kind\":\"horizontal\""), "{text}");
    assert_eq!(Board::from_bytes(text.as_bytes()).unwrap(), b);
    // writing what was read gives the same bytes
    let again = Board::from_bytes(text.as_bytes())
        .unwrap()
        .to_bytes()
        .unwrap();
    assert_eq!(again, text.as_bytes());
}

#[test]
fn a_dimension_is_added_and_removed_with_an_exact_inverse() {
    let (mut b, d) = with_dimension();
    let before = b.to_bytes().unwrap();
    let mut history = History::new();
    history
        .execute(
            &mut b,
            Operation::AddDimension {
                id: id(60),
                dimension: d.clone(),
            },
        )
        .unwrap();
    assert_eq!(b.dimensions[&id(60)], d);
    let added = b.to_bytes().unwrap();
    history
        .execute(&mut b, Operation::RemoveDimension { id: id(60) })
        .unwrap();
    assert!(b.dimensions.is_empty());
    assert_eq!(b.to_bytes().unwrap(), before);
    assert!(history.undo(&mut b));
    assert_eq!(b.to_bytes().unwrap(), added);
    assert!(history.undo(&mut b));
    assert_eq!(b.to_bytes().unwrap(), before);
    let text = serde_json::to_string(&Operation::RemoveDimension { id: id(60) }).unwrap();
    assert!(text.starts_with(r#"{"op":"remove_dimension""#), "{text}");
}

#[test]
fn a_dimension_is_refused_on_a_missing_layer_a_layer_that_is_not_documentation_or_twice() {
    let (mut b, d) = with_dimension();
    let add = |id_n: u16, dimension: Dimension| Operation::AddDimension {
        id: id(id_n),
        dimension,
    };
    let mut missing = d.clone();
    missing.layer = id(99);
    assert_eq!(
        b.apply(&add(60, missing)).unwrap_err(),
        OpError::Dangling(id(99))
    );
    let mut copper = d.clone();
    copper.layer = id(1);
    assert_eq!(
        b.apply(&add(60, copper)).unwrap_err(),
        OpError::NotDocumentation(id(1))
    );
    b.apply(&add(60, d.clone())).unwrap();
    assert_eq!(b.apply(&add(60, d)).unwrap_err(), OpError::Exists(id(60)));
    assert_eq!(
        b.apply(&Operation::RemoveDimension { id: id(61) })
            .unwrap_err(),
        OpError::Missing(id(61))
    );
    assert_eq!(b.dimensions.len(), 1);
}

#[test]
fn a_dimension_that_measures_nothing_or_sits_on_a_wrong_layer_is_reported() {
    let (mut b, d) = with_dimension();
    b.dimensions.insert(id(60), d.clone());
    assert!(problems(&b).is_empty());
    // the two points are the same
    let mut same = d.clone();
    same.to = same.from;
    b.dimensions.insert(id(60), same);
    assert_eq!(problems(&b), vec![BAD_SIZE.to_owned()]);
    // a size that is not above zero
    let mut thin = d.clone();
    thin.stroke = nm(0);
    b.dimensions.insert(id(60), thin);
    assert_eq!(problems(&b), vec![BAD_SIZE.to_owned()]);
    // a copper layer, and a layer that does not exist
    let mut copper = d.clone();
    copper.layer = id(1);
    b.dimensions.insert(id(60), copper);
    assert_eq!(problems(&b), vec![BAD_LAYER.to_owned()]);
    let mut missing = d;
    missing.layer = id(99);
    b.dimensions.insert(id(60), missing);
    assert_eq!(problems(&b), vec![DANGLING.to_owned()]);
}
