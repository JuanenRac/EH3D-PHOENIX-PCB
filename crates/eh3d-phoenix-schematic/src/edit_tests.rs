//! Tests of the editor: operations, the nets made again, undo and redo, and the refusals.

use eh3d_phoenix_model::Point;

use crate::edit::{
    orthogonal_path, Clip, EditError, EditOp, Editor, Item, PinDef, SymbolDef, REFUSED, REGISTRY,
};
use crate::model::{Id, PinKind, Schematic};
use crate::sample::{drawn_schematic, id};

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn cap() -> SymbolDef {
    SymbolDef {
        name: "Cap".into(),
        pins: vec![
            PinDef {
                number: "1".into(),
                at: pt(-2_540_000, 0),
                kind: Some(PinKind::Passive),
            },
            PinDef {
                number: "2".into(),
                at: pt(2_540_000, 0),
                kind: None,
            },
        ],
    }
}

fn place(sheet: Id, x_mm: i64, y_mm: i64) -> EditOp {
    EditOp::PlaceSymbol {
        symbol: "Cap".into(),
        define: Some(cap()),
        sheet,
        at: pt(x_mm * 1_000_000, y_mm * 1_000_000),
        rotation: None,
        mirrored: false,
        refdes: None,
        value: None,
    }
}

fn editor() -> (Editor, Schematic, [Id; 6], Id, Id) {
    let (s, ids, sheet, wire) = drawn_schematic();
    (Editor::new(s.clone()), s, ids, sheet, wire)
}

fn refdes_of(s: &Schematic) -> Vec<String> {
    s.parts.values().map(|p| p.refdes.clone()).collect()
}

#[test]
fn placing_a_symbol_defines_it_once_names_the_part_and_undoes() {
    let (mut ed, original, _, sheet, _) = editor();
    let first = ed.apply(&place(sheet, 60, 20)).unwrap();
    assert_eq!(first.created.len(), 2);
    assert!(refdes_of(ed.schematic()).contains(&"C1".to_owned()));
    assert_eq!(ed.schematic().symbols.len(), 2);
    assert_eq!(ed.schematic().pin_kinds.len(), 1);
    ed.apply(&place(sheet, 70, 20)).unwrap();
    assert!(refdes_of(ed.schematic()).contains(&"C2".to_owned()));
    assert_eq!(ed.schematic().symbols.len(), 2);
    assert!(ed.schematic().is_consistent());
    for created in &first.created {
        assert!(!original.parts.contains_key(created));
        assert!(!original.symbols.contains_key(created));
    }
    assert_eq!(ed.undo_depth(), 2);
    assert!(ed.undo());
    assert!(ed.undo());
    assert!(!ed.undo());
    assert_eq!(ed.schematic(), &original);
    assert!(ed.redo());
    assert!(ed.redo());
    assert!(!ed.redo());
    assert_eq!(ed.schematic().parts.len(), 4);
}

#[test]
fn the_same_operations_give_the_same_bytes() {
    let (mut a, _, ids, sheet, _) = editor();
    let (mut b, ..) = editor();
    let ops = vec![
        place(sheet, 60, 20),
        EditOp::AddWire {
            sheet,
            path: vec![pt(26_000_000, 20_000_000), pt(26_000_000, 30_000_000)],
        },
        EditOp::MoveItems {
            items: vec![Item::Part(ids[2])],
            dx: 0,
            dy: 5_000_000,
            detach: false,
        },
    ];
    for op in &ops {
        a.apply(op).unwrap();
        b.apply(op).unwrap();
    }
    assert_eq!(
        a.schematic().to_bytes().unwrap(),
        b.schematic().to_bytes().unwrap()
    );
}

#[test]
fn a_moved_part_keeps_its_wire_and_the_wire_stays_orthogonal() {
    let (mut ed, original, ids, _, wire) = editor();
    ed.apply(&EditOp::MoveItems {
        items: vec![Item::Part(ids[2])],
        dx: 0,
        dy: 5_000_000,
        detach: false,
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(
        s.wires[&wire].path,
        vec![
            pt(22_540_000, 20_000_000),
            pt(30_000_000, 20_000_000),
            pt(37_460_000, 20_000_000),
            pt(37_460_000, 25_000_000),
        ]
    );
    assert_eq!(s.nets, original.nets);
    assert!(s.drawn().problems.is_empty(), "{:?}", s.drawn().problems);
    assert!(ed.undo());
    assert_eq!(ed.schematic(), &original);
}

#[test]
fn a_part_moved_without_its_wire_loses_the_connection_the_drawing_gave() {
    let (mut ed, _, ids, _, wire) = editor();
    ed.apply(&EditOp::MoveItems {
        items: vec![Item::Part(ids[2])],
        dx: 0,
        dy: 5_000_000,
        detach: true,
    })
    .unwrap();
    assert_eq!(ed.schematic().wires[&wire].path.len(), 3);
    assert!(ed.schematic().nets.is_empty());
}

#[test]
fn a_turned_part_takes_its_wire_along_with_a_bend() {
    let (mut ed, original, ids, _, wire) = editor();
    ed.apply(&EditOp::RotateParts {
        parts: vec![ids[1]],
        quarter_turns: 1,
    })
    .unwrap();
    let s = ed.schematic();
    let rotation = s.parts[&ids[1]].placement.unwrap().rotation;
    assert_eq!(rotation.get(), 90_000_000);
    assert_eq!(
        s.wires[&wire].path,
        vec![
            pt(20_000_000, 22_540_000),
            pt(20_000_000, 20_000_000),
            pt(30_000_000, 20_000_000),
            pt(37_460_000, 20_000_000),
        ]
    );
    assert_eq!(s.nets, original.nets);
    // four quarter turns are no turn at all
    for _ in 0..3 {
        ed.apply(&EditOp::RotateParts {
            parts: vec![ids[1]],
            quarter_turns: 1,
        })
        .unwrap();
    }
    let rotation = ed.schematic().parts[&ids[1]].placement.unwrap().rotation;
    assert_eq!(rotation.get(), 0);
}

#[test]
fn a_mirrored_part_has_its_pins_on_the_other_side_and_undoes() {
    let (mut ed, original, _, sheet, _) = editor();
    let made = ed.apply(&place(sheet, 60, 40)).unwrap();
    let part = made
        .created
        .iter()
        .copied()
        .find(|c| ed.schematic().parts.contains_key(c))
        .unwrap();
    let x_of_pin_one = |s: &Schematic| {
        s.pin_positions()
            .into_iter()
            .filter(|(p, _, _)| p.part == part && s.pin_number(*p) == Some("1"))
            .map(|(_, _, at)| at.x_nm.get())
            .next()
    };
    assert_eq!(x_of_pin_one(ed.schematic()), Some(57_460_000));
    ed.apply(&EditOp::MirrorParts { parts: vec![part] })
        .unwrap();
    assert!(ed.schematic().mirrored.contains(&part));
    assert_eq!(x_of_pin_one(ed.schematic()), Some(62_540_000));
    assert!(ed.undo());
    assert!(!ed.schematic().mirrored.contains(&part));
    assert!(ed.undo());
    assert_eq!(ed.schematic(), &original);
}

#[test]
fn a_wire_that_ends_in_the_middle_of_another_makes_a_t_with_a_junction() {
    let (mut ed, original, _, sheet, wire) = editor();
    ed.apply(&EditOp::AddWire {
        sheet,
        path: vec![pt(26_000_000, 20_000_000), pt(26_000_000, 30_000_000)],
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(
        s.wires[&wire].path,
        vec![
            pt(22_540_000, 20_000_000),
            pt(26_000_000, 20_000_000),
            pt(30_000_000, 20_000_000),
            pt(37_460_000, 20_000_000),
        ]
    );
    assert_eq!(s.junctions.len(), 1);
    assert_eq!(
        s.junctions.values().next().unwrap().at,
        pt(26_000_000, 20_000_000)
    );
    assert!(s.drawn().problems.is_empty(), "{:?}", s.drawn().problems);
    assert_eq!(s.nets, original.nets);
    assert!(ed.undo());
    assert_eq!(ed.schematic(), &original);
}

#[test]
fn labels_with_one_name_make_one_net_and_keep_its_identity() {
    let (mut ed, _, _, sheet, wire) = editor();
    ed.apply(&EditOp::Delete {
        items: vec![Item::Wire(wire)],
    })
    .unwrap();
    assert!(ed.schematic().nets.is_empty());
    ed.apply(&EditOp::AddLabel {
        sheet,
        at: pt(22_540_000, 20_000_000),
        name: " SIG ".into(),
    })
    .unwrap();
    assert_eq!(ed.schematic().nets.len(), 1);
    let first = *ed.schematic().nets.keys().next().unwrap();
    ed.apply(&EditOp::AddLabel {
        sheet,
        at: pt(37_460_000, 20_000_000),
        name: "SIG".into(),
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(s.nets.len(), 1);
    let net = &s.nets[&first];
    assert_eq!(net.name, "SIG");
    assert_eq!(net.pins.len(), 2);
}

#[test]
fn a_power_port_joins_pins_by_its_name_and_a_rename_follows() {
    let (mut ed, _, _, sheet, wire) = editor();
    ed.apply(&EditOp::Delete {
        items: vec![Item::Wire(wire)],
    })
    .unwrap();
    let mut ports = Vec::new();
    for x in [22_540_000, 37_460_000] {
        let made = ed
            .apply(&EditOp::AddPowerPort {
                sheet,
                at: pt(x, 20_000_000),
                name: "VCC".into(),
            })
            .unwrap();
        ports.extend(made.created);
    }
    assert_eq!(ed.schematic().nets.len(), 1);
    assert_eq!(ed.schematic().nets.values().next().unwrap().name, "VCC");
    ed.apply(&EditOp::RenameMark {
        id: ports[0],
        name: "VDD".into(),
    })
    .unwrap();
    // the two ports no longer share a name, so the pins are on two nets of their own
    let names: Vec<&str> = ed
        .schematic()
        .nets
        .values()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"VDD") && names.contains(&"VCC"));
}

#[test]
fn deleting_a_part_removes_its_pins_from_the_nets() {
    let (mut ed, _, ids, ..) = editor();
    ed.apply(&EditOp::Delete {
        items: vec![Item::Part(ids[2])],
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(s.parts.len(), 1);
    assert!(s.nets.is_empty());
    assert!(s.is_consistent(), "{:?}", s.validate());
}

#[test]
fn values_and_references_are_set_and_a_repeated_reference_is_refused() {
    let (mut ed, original, ids, ..) = editor();
    ed.apply(&EditOp::SetValue {
        part: ids[2],
        value: "4k7".into(),
    })
    .unwrap();
    assert_eq!(ed.schematic().parts[&ids[2]].value, "4k7");
    let error = ed
        .apply(&EditOp::SetRefdes {
            part: ids[2],
            refdes: "R1".into(),
        })
        .unwrap_err();
    assert_eq!(error, EditError::DuplicateRefdes("R1".into()));
    assert_eq!(ed.undo_depth(), 1);
    ed.apply(&EditOp::SetRefdes {
        part: ids[2],
        refdes: "R9".into(),
    })
    .unwrap();
    assert!(refdes_of(ed.schematic()).contains(&"R9".to_owned()));
    assert!(ed.undo());
    assert!(ed.undo());
    assert_eq!(ed.schematic(), &original);
}

#[test]
fn copy_and_paste_make_new_parts_with_new_references() {
    let (mut ed, _, ids, sheet, wire) = editor();
    let clip = Clip::copy(
        ed.schematic(),
        &[Item::Part(ids[1]), Item::Wire(wire), Item::Junction(id(90))],
    );
    assert_eq!(clip.parts.len(), 1);
    assert_eq!(clip.wires.len(), 1);
    assert_eq!(clip.corner(), Some(pt(20_000_000, 20_000_000)));
    let text = serde_json::to_string(&clip).unwrap();
    let back: Clip = serde_json::from_str(&text).unwrap();
    assert_eq!(back, clip);
    let paste = EditOp::Paste {
        clip: clip.clone(),
        sheet,
        dx: 0,
        dy: 20_000_000,
    };
    let made = ed.apply(&paste).unwrap();
    assert_eq!(made.created.len(), 2);
    ed.apply(&EditOp::Paste {
        clip,
        sheet,
        dx: 0,
        dy: 40_000_000,
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(s.parts.len(), 4);
    assert_eq!(s.wires.len(), 3);
    assert_eq!(s.nets.len(), 1);
    let mut names = refdes_of(s);
    names.sort();
    assert_eq!(names, vec!["R1", "R2", "R3", "R4"]);
    assert!(Clip::default().is_empty());
    assert_eq!(Clip::default().corner(), None);
}

#[test]
fn the_labels_of_a_bus_are_one_per_member() {
    let (mut ed, _, _, sheet, _) = editor();
    ed.apply(&EditOp::AddBus {
        sheet,
        at: pt(10_000_000, 10_000_000),
        spec: "D[0..3]".into(),
        dx: 0,
        dy: 0,
    })
    .unwrap();
    let s = ed.schematic();
    assert_eq!(s.labels.len(), 4);
    let d1 = s.labels.values().find(|l| l.name == "D1").unwrap();
    assert_eq!(d1.at, pt(10_000_000, 12_540_000));
    let error = ed
        .apply(&EditOp::AddBus {
            sheet,
            at: pt(0, 0),
            spec: "D".into(),
            dx: 0,
            dy: 0,
        })
        .unwrap_err();
    assert_eq!(error, EditError::BadBus("D".into()));
}

#[test]
fn a_refused_operation_changes_nothing_not_even_inside_a_batch() {
    let (mut ed, original, ids, sheet, _) = editor();
    let batch = EditOp::Batch {
        ops: vec![
            EditOp::SetValue {
                part: ids[1],
                value: "1k".into(),
            },
            place(sheet, 60, 20),
            EditOp::SetRefdes {
                part: ids[2],
                refdes: "R1".into(),
            },
        ],
    };
    assert!(ed.apply(&batch).is_err());
    assert_eq!(ed.schematic(), &original);
    assert!(!ed.can_undo());
    // the supply of ids is as it was too: the same part gets the same id afterwards
    let after_error = ed.apply(&place(sheet, 60, 20)).unwrap();
    let (mut fresh, ..) = editor();
    let clean = fresh.apply(&place(sheet, 60, 20)).unwrap();
    assert_eq!(after_error, clean);
    // a good batch is one step
    let good = EditOp::Batch {
        ops: vec![
            EditOp::SetValue {
                part: ids[1],
                value: "1k".into(),
            },
            place(sheet, 80, 20),
        ],
    };
    let depth = ed.undo_depth();
    ed.apply(&good).unwrap();
    assert_eq!(ed.undo_depth(), depth + 1);
}

#[test]
fn bad_input_is_refused_with_the_right_error() {
    let (mut ed, _, ids, sheet, _) = editor();
    let missing = id(99);
    assert_eq!(
        ed.apply(&place(missing, 0, 0)).unwrap_err(),
        EditError::NoSuchSheet(missing)
    );
    assert!(matches!(
        ed.apply(&EditOp::PlaceSymbol {
            symbol: "Nothing".into(),
            define: None,
            sheet,
            at: pt(0, 0),
            rotation: None,
            mirrored: false,
            refdes: None,
            value: None,
        }),
        Err(EditError::NoSuchSymbol(_))
    ));
    assert!(matches!(
        ed.apply(&EditOp::AddWire {
            sheet,
            path: vec![pt(0, 0), pt(5, 5)],
        }),
        Err(EditError::BadWire(_))
    ));
    assert!(matches!(
        ed.apply(&EditOp::AddWire {
            sheet,
            path: vec![pt(0, 0)],
        }),
        Err(EditError::BadWire(_))
    ));
    assert!(matches!(
        ed.apply(&EditOp::AddLabel {
            sheet,
            at: pt(0, 0),
            name: "  ".into(),
        }),
        Err(EditError::BadText(_))
    ));
    assert!(matches!(
        ed.apply(&EditOp::MoveItems {
            items: vec![Item::Part(ids[1])],
            dx: 1 << 40,
            dy: 0,
            detach: false,
        }),
        Err(EditError::OutOfRange)
    ));
    assert_eq!(
        ed.apply(&EditOp::Delete {
            items: vec![Item::Wire(missing)],
        })
        .unwrap_err(),
        EditError::NoSuchItem(missing)
    );
    assert!(!ed.can_undo());
    assert!(EditError::OutOfRange.to_string().starts_with(REFUSED));
}

#[test]
fn a_new_operation_after_an_undo_drops_the_steps_to_redo() {
    let (mut ed, _, ids, ..) = editor();
    ed.apply(&EditOp::SetValue {
        part: ids[1],
        value: "1".into(),
    })
    .unwrap();
    assert!(ed.undo());
    assert!(ed.can_redo());
    ed.apply(&EditOp::SetValue {
        part: ids[1],
        value: "2".into(),
    })
    .unwrap();
    assert!(!ed.can_redo());
    let mut limited = Editor::with_limit(drawn_schematic().0, 2);
    for n in 0..5 {
        limited
            .apply(&EditOp::SetValue {
                part: ids[1],
                value: format!("{n}"),
            })
            .unwrap();
    }
    assert_eq!(limited.undo_depth(), 2);
}

#[test]
fn an_orthogonal_path_has_a_corner_only_when_it_needs_one() {
    let a = pt(0, 0);
    let b = pt(10, 5);
    assert_eq!(orthogonal_path(a, b, true), vec![a, pt(10, 0), b]);
    assert_eq!(orthogonal_path(a, b, false), vec![a, pt(0, 5), b]);
    assert_eq!(orthogonal_path(a, pt(10, 0), true), vec![a, pt(10, 0)]);
    assert!(orthogonal_path(a, a, true).is_empty());
}

#[test]
fn operations_are_read_from_json_and_unknown_fields_are_refused() {
    let (_, _, ids, ..) = editor();
    let text = format!(r#"{{"op":"set_value","part":"{}","value":"1k"}}"#, ids[2]);
    let op: EditOp = serde_json::from_str(&text).unwrap();
    assert_eq!(
        op,
        EditOp::SetValue {
            part: ids[2],
            value: "1k".into()
        }
    );
    let extra = format!(
        r#"{{"op":"set_value","part":"{}","value":"1k","extra":1}}"#,
        ids[2]
    );
    assert!(serde_json::from_str::<EditOp>(&extra).is_err());
    let batch = EditOp::Batch {
        ops: vec![
            op,
            EditOp::MirrorParts {
                parts: vec![ids[1]],
            },
        ],
    };
    let back: EditOp = serde_json::from_str(&serde_json::to_string(&batch).unwrap()).unwrap();
    assert_eq!(back, batch);
    let items = format!(r#"{{"op":"delete","items":[{{"part":"{}"}}]}}"#, ids[1]);
    let delete: EditOp = serde_json::from_str(&items).unwrap();
    assert_eq!(
        delete,
        EditOp::Delete {
            items: vec![Item::Part(ids[1])]
        }
    );
}

#[test]
fn the_documented_table_has_every_code_of_this_module() {
    let doc = include_str!("../../../docs/ERROR_CODES.md");
    for (code, title) in REGISTRY {
        let row = doc
            .lines()
            .find(|l| l.starts_with(&format!("| {code} |")))
            .unwrap_or_else(|| panic!("{code} is not documented"));
        assert!(row.contains(title), "{code}: {row}");
    }
}
