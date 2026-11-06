//! The operations that add and remove copper zones and footprints: each has an exact inverse,
//! a refused operation leaves the board as it was, and a batch is all or nothing.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, OpError, Operation, Pad, PadShape,
    Side, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with one copper layer (id 1) and one net (id 2).
fn board() -> Board {
    let mut b = Board {
        name: "zones".into(),
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
    b.nets.insert(id(2), Net { name: "RF".into() });
    b
}

fn square(layer: Id, net: Id) -> Zone {
    Zone {
        net,
        layer,
        outline: vec![
            pt(0, 0),
            pt(1_000_000, 0),
            pt(1_000_000, 1_000_000),
            pt(0, 1_000_000),
        ],
        clearance: Nm::new(200_000).unwrap(),
        cutout,
        cutouts: Vec::new(),
        s: Vec::new(),
    }
}

fn footprint(layer: Id) -> Footprint {
    let mut pads = BTreeMap::new();
    pads.insert(
        id(30),
        Pad {
            number: "1".into(),
            shape: PadShape::Rect {
                width: Nm::new(1_000_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![layer],
            roundness: 0,
            plated: true,
        },
    );
    Footprint {
        name: "F".into(),
        pads,
    }
}

#[test]
fn a_zone_is_added_and_its_inverse_removes_it() {
    let mut b = board();
    let before = b.clone();
    let inverse = b
        .apply(&Operation::AddZone {
            id: id(10),
            zone: square(id(1), id(2)),
        })
        .unwrap();
    assert_eq!(b.zones.len(), 1);
    assert!(b.is_consistent(), "{:?}", b.validate());
    let again = b.apply(&inverse).unwrap();
    assert_eq!(b, before);
    b.apply(&again).unwrap();
    assert_eq!(b.zones.len(), 1);
}

#[test]
fn a_zone_needs_its_net_its_layer_and_a_free_id() {
    let mut b = board();
    let zone = square(id(1), id(2));
    assert_eq!(
        b.apply(&Operation::AddZone {
            id: id(10),
            zone: square(id(1), id(99)),
        }),
        Err(OpError::Dangling(id(99)))
    );
    assert_eq!(
        b.apply(&Operation::AddZone {
            id: id(10),
            zone: square(id(98), id(2)),
        }),
        Err(OpError::Dangling(id(98)))
    );
    b.apply(&Operation::AddZone {
        id: id(10),
        zone: zone.clone(),
    })
    .unwrap();
    assert_eq!(
        b.apply(&Operation::AddZone {
            id: id(10),
            zone: zone.clone(),
        }),
        Err(OpError::Exists(id(10)))
    );
    assert_eq!(
        b.apply(&Operation::RemoveZone { id: id(11) }),
        Err(OpError::Missing(id(11)))
    );
}

#[test]
fn a_footprint_is_added_and_removed_unless_a_component_uses_it() {
    let mut b = board();
    let before = b.clone();
    let inverse = b
        .apply(&Operation::AddFootprint {
            id: id(20),
            footprint: footprint(id(1)),
        })
        .unwrap();
    assert!(b.footprints.contains_key(&id(20)));
    b.apply(&Operation::AddComponent {
        id: id(21),
        component: Component {
            refdes: "J1".into(),
            value: "SMA".into(),
            symbol: None,
            footprint: id(20),
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    })
    .unwrap();
    assert_eq!(b.apply(&inverse), Err(OpError::InUse(id(20))));
    b.apply(&Operation::RemoveComponent { id: id(21) }).unwrap();
    b.apply(&inverse).unwrap();
    assert_eq!(b, before);
}

#[test]
fn a_footprint_with_a_pad_on_a_missing_layer_is_refused() {
    let mut b = board();
    assert_eq!(
        b.apply(&Operation::AddFootprint {
            id: id(20),
            footprint: footprint(id(77)),
        }),
        Err(OpError::Dangling(id(77)))
    );
    assert!(b.footprints.is_empty());
}

#[test]
fn a_batch_with_a_zone_is_all_or_nothing() {
    let mut b = board();
    let before = b.clone();
    let batch = Operation::Batch {
        ops: vec![
            Operation::AddZone {
                id: id(10),
                zone: square(id(1), id(2)),
            },
            Operation::RemoveZone { id: id(11) },
        ],
    };
    assert!(b.apply(&batch).is_err());
    assert_eq!(b, before);
}
