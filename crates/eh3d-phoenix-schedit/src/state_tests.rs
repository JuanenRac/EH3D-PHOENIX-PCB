// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the state of the editor: the tools, the drag, the selection, copy and paste, the rule
//! check after each change and the way out to the board and the simulator.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, History};
use eh3d_phoenix_erc::{self as erc, ErcConfig};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{Id, Item, Schematic};
use eh3d_phoenix_sim::SimSettings;
use eh3d_phoenix_sync::SyncSettings;

use crate::palette::Palette;
use crate::settings::SchEditSettings;
use crate::state::{SchEdit, Tool};
use crate::testutil::{fixture, pt, resistor, Fixture};

fn edit(f: &Fixture) -> SchEdit {
    SchEdit::new(
        f.schematic.clone(),
        SchEditSettings::default(),
        ErcConfig::default(),
    )
}

fn other_id(id: Id) -> Id {
    let mut bytes = *id.as_bytes();
    bytes[15] ^= 0x01;
    Id::from_bytes(bytes).unwrap()
}

#[test]
fn dragging_a_part_moves_it_and_its_wire_follows_the_pin() {
    let f = fixture();
    let mut se = edit(&f);
    se.pointer_pressed(pt(20_000_000, 20_000_000), false);
    assert_eq!(se.selection, BTreeSet::from([Item::Part(f.r1)]));
    assert_eq!(se.drag.as_ref().unwrap().items, vec![Item::Part(f.r1)]);
    se.pointer_moved(pt(20_000_000, 25_400_000));
    assert_eq!(se.drag.as_ref().unwrap().offset(), (0, 5_080_000));
    let preview = se.drag_preview().unwrap();
    assert_eq!(preview.wires[&f.wire].path.len(), 3);
    assert_eq!(se.schematic(), &f.schematic);
    se.pointer_released(pt(20_000_000, 25_400_000));
    assert!(se.drag.is_none());
    assert!(se.dirty());
    let s = se.schematic();
    assert_eq!(
        s.wires[&f.wire].path,
        vec![
            pt(22_540_000, 25_080_000),
            pt(22_540_000, 20_000_000),
            pt(37_460_000, 20_000_000),
        ]
    );
    assert_eq!(s.nets, f.schematic.nets);
    assert!(se.undo());
    assert_eq!(se.schematic(), &f.schematic);
    assert!(se.can_redo());
    assert!(se.redo());
    assert!(!se.redo());
}

#[test]
fn a_rectangle_selects_what_it_takes_in() {
    let f = fixture();
    let mut se = edit(&f);
    se.pointer_pressed(pt(10_000_000, 10_000_000), false);
    assert!(se.selection.is_empty());
    assert!(se.marquee.is_some());
    se.pointer_moved(pt(45_000_000, 25_000_000));
    se.pointer_released(pt(45_000_000, 25_000_000));
    assert!(se.marquee.is_none());
    assert_eq!(
        se.selection,
        BTreeSet::from([Item::Part(f.r1), Item::Part(f.r2), Item::Wire(f.wire)])
    );
    // shift takes one away
    se.pointer_pressed(pt(40_000_000, 20_000_000), true);
    assert!(!se.selection.contains(&Item::Part(f.r2)));
    assert!(se.drag.is_none());
}

#[test]
fn a_wire_is_drawn_in_clicks_with_a_corner_and_goes_on_until_it_is_finished() {
    let f = fixture();
    let mut se = edit(&f);
    se.set_tool(Tool::Wire);
    se.pointer_pressed(pt(30_000_000, 40_000_000), false);
    assert_eq!(se.wire_start, Some(pt(30_480_000, 40_640_000)));
    assert_eq!(
        se.wire_preview(pt(50_800_000, 50_800_000)),
        vec![
            pt(30_480_000, 40_640_000),
            pt(50_800_000, 40_640_000),
            pt(50_800_000, 50_800_000),
        ]
    );
    se.flip_corner();
    assert_eq!(
        se.wire_preview(pt(50_800_000, 50_800_000))[1],
        pt(30_480_000, 50_800_000)
    );
    se.flip_corner();
    se.pointer_pressed(pt(50_000_000, 50_000_000), false);
    assert_eq!(se.schematic().wires.len(), 2);
    let (_, new_wire) = se
        .schematic()
        .wires
        .iter()
        .find(|(id, _)| **id != f.wire)
        .unwrap();
    assert_eq!(new_wire.path.len(), 3);
    // the wire goes on from its end; a click on that same point finishes it
    assert_eq!(se.wire_start, Some(pt(50_800_000, 50_800_000)));
    se.pointer_pressed(pt(50_800_000, 50_800_000), false);
    assert_eq!(se.wire_start, None);
    se.pointer_pressed(pt(1_000_000, 1_000_000), false);
    assert!(se.wire_start.is_some());
    se.cancel();
    assert_eq!(se.wire_start, None);
}

#[test]
fn a_label_names_the_net_under_it_and_an_empty_name_is_refused() {
    let f = fixture();
    let mut se = edit(&f);
    se.set_tool(Tool::Label);
    se.pending_name = String::new();
    se.pointer_pressed(pt(22_540_000, 20_000_000), false);
    assert!(se.error.is_some());
    assert!(se.schematic().labels.is_empty());
    se.pending_name = "SIG".into();
    se.pointer_pressed(pt(22_540_000, 20_000_000), false);
    assert!(se.error.is_none());
    assert_eq!(se.schematic().nets.len(), 1);
    assert_eq!(se.schematic().nets.values().next().unwrap().name, "SIG");
    assert_eq!(se.selection.len(), 1);
    se.set_tool(Tool::Bus);
    se.pending_name = "D[0..3]".into();
    se.pointer_pressed(pt(60_000_000, 10_000_000), false);
    assert_eq!(se.schematic().labels.len(), 5);
    se.set_tool(Tool::PowerPort);
    se.pending_name = "GND".into();
    se.pointer_pressed(pt(60_000_000, 80_000_000), false);
    assert_eq!(se.schematic().power_ports.len(), 1);
}

#[test]
fn the_place_tool_places_the_chosen_symbol_and_reuses_the_one_the_schematic_has() {
    let f = fixture();
    let mut se = edit(&f);
    assert!(!se.choose_symbol("Nothing"));
    assert!(se.choose_symbol("R"));
    assert_eq!(se.tool, Tool::Place);
    se.pointer_pressed(pt(60_000_000, 30_000_000), false);
    let s = se.schematic();
    assert_eq!(s.parts.len(), 3);
    assert_eq!(s.symbols.len(), 1);
    let made = s.parts.values().find(|p| p.refdes == "R3").unwrap();
    assert_eq!(made.placement.unwrap().at, pt(60_960_000, 30_480_000));
    assert_eq!(se.selection.len(), 1);
}

#[test]
fn a_symbol_with_the_name_of_another_one_is_kept_apart() {
    let f = fixture();
    let mut se = edit(&f);
    let mut other = resistor();
    other.pins[1].at = pt(5_080_000, 0);
    se.palette = Palette::new();
    se.palette.add("Library", other);
    assert!(se.choose_symbol("R"));
    se.pointer_pressed(pt(60_000_000, 30_000_000), false);
    assert_eq!(se.schematic().symbols.len(), 2);
    assert!(se
        .schematic()
        .symbols
        .values()
        .any(|symbol| symbol.name == "Library:R"));
    // the second part of it uses the symbol that was defined, it does not define it again
    se.pointer_pressed(pt(80_000_000, 30_000_000), false);
    assert_eq!(se.schematic().symbols.len(), 2);
    assert_eq!(se.schematic().parts.len(), 4);
}

#[test]
fn turn_mirror_copy_paste_cut_and_delete_work_on_the_selection() {
    let f = fixture();
    let mut se = edit(&f);
    assert!(se.delete_selection().is_err());
    assert!(se.error.is_some());
    assert!(se.rotate_selection(1).is_err());
    assert!(!se.copy_selection());
    assert!(se.paste(None).is_err());
    se.selection.insert(Item::Part(f.r1));
    se.rotate_selection(1).unwrap();
    let rotation = se.schematic().parts[&f.r1].placement.unwrap().rotation;
    assert_eq!(rotation.get(), 90_000_000);
    se.mirror_selection().unwrap();
    assert!(se.schematic().mirrored.contains(&f.r1));
    assert!(se.copy_selection());
    se.paste(Some(pt(60_000_000, 60_000_000))).unwrap();
    assert_eq!(se.schematic().parts.len(), 3);
    assert_eq!(se.selection.len(), 1);
    assert!(!se.selection.contains(&Item::Part(f.r1)));
    se.paste(None).unwrap();
    assert_eq!(se.schematic().parts.len(), 4);
    se.cut_selection().unwrap();
    assert_eq!(se.schematic().parts.len(), 3);
    assert!(se.selection.is_empty());
    se.selection.insert(Item::Part(f.r2));
    se.delete_selection().unwrap();
    assert_eq!(se.schematic().parts.len(), 2);
}

#[test]
fn values_and_references_are_set_on_the_selected_parts() {
    let f = fixture();
    let mut se = edit(&f);
    assert!(se.set_value("1k").is_err());
    se.selection.insert(Item::Part(f.r1));
    se.selection.insert(Item::Part(f.r2));
    se.set_value("1k").unwrap();
    assert_eq!(se.schematic().parts[&f.r1].value, "1k");
    assert_eq!(se.schematic().parts[&f.r2].value, "1k");
    assert!(se.set_refdes("R9").is_err());
    se.selection.remove(&Item::Part(f.r2));
    assert!(se.set_refdes("R2").is_err());
    se.set_refdes("R9").unwrap();
    assert_eq!(se.schematic().parts[&f.r1].refdes, "R9");
}

#[test]
fn the_rule_check_follows_the_changes_when_the_settings_say_so() {
    let f = fixture();
    let mut se = edit(&f);
    se.selection.insert(Item::Part(f.r1));
    se.set_value("").unwrap();
    assert_eq!(se.erc, erc::check(se.schematic(), &ErcConfig::default()));
    let settings = SchEditSettings::parse("live_erc = false").unwrap();
    let mut quiet = SchEdit::new(f.schematic.clone(), settings, ErcConfig::default());
    let first = quiet.erc.clone();
    quiet.selection.insert(Item::Part(f.r1));
    quiet.set_value("").unwrap();
    assert_eq!(quiet.erc, first);
    quiet.run_erc();
    assert_eq!(quiet.erc, se.erc);
}

#[test]
fn the_transfer_is_a_preview_first_and_the_net_list_is_for_the_simulator() {
    let f = fixture();
    let mut se = edit(&f);
    assert!(se.transfer_text(Lang::En).is_none());
    se.plan_transfer(&Board::default(), &SyncSettings::default());
    assert!(se.transfer_text(Lang::En).is_some());
    assert!(!se.transfer_has_changes());
    let mut board = Board::default();
    let mut history = History::new();
    se.apply_transfer(&mut board, &mut history).unwrap();
    assert!(se.preview.is_none());
    assert_eq!(
        se.netlist(&SimSettings::default()).report.translated.len(),
        2
    );
    // a change of the schematic throws a stale package away
    se.plan_transfer(&Board::default(), &SyncSettings::default());
    se.selection.insert(Item::Part(f.r1));
    se.set_value("2k").unwrap();
    assert!(se.preview.is_none());
}

#[test]
fn a_schematic_without_a_sheet_gets_one_when_something_is_drawn() {
    let mut se = SchEdit::new(
        Schematic::default(),
        SchEditSettings::default(),
        ErcConfig::default(),
    );
    assert!(se.sheet().is_none());
    let sheet = se.ensure_sheet().unwrap();
    assert_eq!(se.ensure_sheet().unwrap(), sheet);
    assert_eq!(se.schematic().sheets.len(), 1);
    assert_eq!(se.schematic().sheets[&sheet].name, "Sheet 1");
    assert!(!se.set_sheet(other_id(sheet)));
    assert!(se.set_sheet(sheet));
    se.set_tool(Tool::Wire);
    se.pointer_pressed(pt(0, 0), false);
    assert!(se.wire_start.is_some());
    let schematic = se.into_schematic();
    assert_eq!(schematic.sheets.len(), 1);
}

#[test]
fn the_revision_grows_with_every_change_an_undo_and_a_redo_but_not_with_a_refusal() {
    let f = fixture();
    let mut se = edit(&f);
    assert_eq!(se.revision(), 0);
    se.selection.insert(Item::Part(f.r1));
    se.set_value("1").unwrap();
    assert_eq!(se.revision(), 1);
    assert!(se.undo());
    assert_eq!(se.revision(), 2);
    assert!(se.redo());
    assert_eq!(se.revision(), 3);
    assert!(se.set_refdes("R2").is_err());
    assert_eq!(se.revision(), 3);
    assert!(se.dirty());
    se.mark_saved();
    assert!(!se.dirty());
}
