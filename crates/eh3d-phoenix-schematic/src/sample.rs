//! A small schematic and board for the tests of this crate.

use std::collections::BTreeMap;

use eh3d_container::ObjectId;

use crate::model::{Id, NetPin, Part, Pin, Schematic, SchematicNet, Symbol};

pub(crate) fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

/// Two parts of one two-pin symbol, joined by one net. The ids are, in order: the symbol, the two
/// parts, the two pins, the net.
pub(crate) fn schematic() -> (Schematic, [Id; 6]) {
    let (sym, p1, p2, pin1, pin2, net) = (id(1), id(2), id(3), id(4), id(5), id(6));
    let mut s = Schematic {
        name: "S".into(),
        ..Schematic::default()
    };
    let mut pins = BTreeMap::new();
    pins.insert(
        pin1,
        Pin {
            number: "1".into(),
            at: None,
        },
    );
    pins.insert(
        pin2,
        Pin {
            number: "2".into(),
            at: None,
        },
    );
    s.symbols.insert(
        sym,
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    for (pid, refdes) in [(p1, "R1"), (p2, "R2")] {
        s.parts.insert(
            pid,
            Part {
                refdes: refdes.into(),
                value: "10k".into(),
                symbol: sym,
                footprint: None,
                placement: None,
            },
        );
    }
    let mut net_pins = vec![
        NetPin {
            part: p1,
            pin: pin2,
        },
        NetPin {
            part: p2,
            pin: pin1,
        },
    ];
    net_pins.sort();
    s.nets.insert(
        net,
        SchematicNet {
            name: "N".into(),
            pins: net_pins,
        },
    );
    (s, [sym, p1, p2, pin1, pin2, net])
}

/// The sample schematic drawn on one sheet: two resistors 20 mm apart, the pins at 2.54 mm on
/// each side of the origin of the symbol, and one wire joining pin 2 of the first with pin 1 of the
/// second through a point in the middle. Returns the schematic, the ids of [`schematic`], the
/// sheet and the wire.
pub(crate) fn drawn_schematic() -> (Schematic, [Id; 6], Id, Id) {
    use crate::draw::{Placement, Sheet, Wire};
    use eh3d_phoenix_model::{Nm, Point, Udeg};

    let (mut s, ids) = schematic();
    let (sheet, wire) = (id(70), id(71));
    let pt = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
    s.sheets.insert(
        sheet,
        Sheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        },
    );
    let symbol = s.symbols.get_mut(&ids[0]).unwrap();
    symbol.pins.get_mut(&ids[3]).unwrap().at = Some(pt(-2_540_000, 0));
    symbol.pins.get_mut(&ids[4]).unwrap().at = Some(pt(2_540_000, 0));
    for (part, x) in [(ids[1], 20_000_000), (ids[2], 40_000_000)] {
        s.parts.get_mut(&part).unwrap().placement = Some(Placement {
            sheet,
            at: pt(x, 20_000_000),
            rotation: Udeg::ZERO,
        });
    }
    s.wires.insert(
        wire,
        Wire {
            sheet,
            path: vec![
                pt(22_540_000, 20_000_000),
                pt(30_000_000, 20_000_000),
                pt(37_460_000, 20_000_000),
            ],
        },
    );
    (s, ids, sheet, wire)
}
