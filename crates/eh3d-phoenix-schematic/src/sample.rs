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
    pins.insert(pin1, Pin { number: "1".into() });
    pins.insert(pin2, Pin { number: "2".into() });
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
