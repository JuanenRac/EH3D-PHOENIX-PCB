// SPDX-License-Identifier: GPL-3.0-or-later
//! A small drawn schematic for the tests of this crate, made with the editor itself.

use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::{
    orthogonal_path, EditOp, Editor, Id, PinDef, PinKind, Schematic, SymbolDef,
};

/// Two resistors 20 mm apart on one sheet, their inner pins joined by one wire.
pub(crate) struct Fixture {
    pub(crate) schematic: Schematic,
    pub(crate) sheet: Id,
    pub(crate) r1: Id,
    pub(crate) r2: Id,
    pub(crate) wire: Id,
}

pub(crate) fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A two-pin symbol with the pins 2.54 mm each side of the origin.
pub(crate) fn resistor() -> SymbolDef {
    SymbolDef {
        name: "R".into(),
        pins: vec![
            PinDef {
                number: "1".into(),
                at: pt(-2_540_000, 0),
                kind: Some(PinKind::Passive),
            },
            PinDef {
                number: "2".into(),
                at: pt(2_540_000, 0),
                kind: Some(PinKind::Passive),
            },
        ],
    }
}

pub(crate) fn place_op(sheet: Id, x_mm: i64, y_mm: i64) -> EditOp {
    EditOp::PlaceSymbol {
        symbol: "R".into(),
        define: Some(resistor()),
        sheet,
        at: pt(x_mm * 1_000_000, y_mm * 1_000_000),
        rotation: None,
        mirrored: false,
        refdes: None,
        value: Some("10k".into()),
    }
}

fn new_part(ed: &mut Editor, op: &EditOp) -> Id {
    let made = ed.apply(op).unwrap();
    *made
        .created
        .iter()
        .find(|id| ed.schematic().parts.contains_key(*id))
        .unwrap()
}

pub(crate) fn fixture() -> Fixture {
    let mut ed = Editor::new(Schematic {
        name: "T".into(),
        ..Schematic::default()
    });
    let sheet = ed
        .apply(&EditOp::AddSheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        })
        .unwrap()
        .created[0];
    let r1 = new_part(&mut ed, &place_op(sheet, 20, 20));
    let r2 = new_part(&mut ed, &place_op(sheet, 40, 20));
    let path = orthogonal_path(pt(22_540_000, 20_000_000), pt(37_460_000, 20_000_000), true);
    let wire = ed.apply(&EditOp::AddWire { sheet, path }).unwrap().created[0];
    Fixture {
        schematic: ed.into_schematic(),
        sheet,
        r1,
        r2,
        wire,
    }
}

#[test]
fn the_fixture_is_what_the_tests_expect() {
    let f = fixture();
    assert_eq!(f.schematic.parts.len(), 2);
    assert_eq!(f.schematic.parts[&f.r1].refdes, "R1");
    assert_eq!(f.schematic.parts[&f.r2].refdes, "R2");
    assert_eq!(f.schematic.wires[&f.wire].path.len(), 2);
    assert_eq!(f.schematic.nets.len(), 1);
    assert_eq!(f.schematic.nets.values().next().unwrap().pins.len(), 2);
    assert!(f.schematic.is_consistent());
}
