//! Tests of the sheet symbols, the repeated sheets, the buses and their operations.
//!
//! The design most of them use: a divider sheet (R1 of 1k on top of R2 of 3k, the port IN on the
//! top of R1, the port OUT in the middle, GND under R2) shown twice on the top sheet by the sheet
//! symbols U1 and U2, both fed by one source V1 of 10 V. The expected voltage of both outputs is
//! 10 V times 3k / (1k + 3k) = 7.5 V, and the source gives 2 x 10 V / 4k = 5 mA.

use std::collections::BTreeSet;

use eh3d_phoenix_model::{Nm, Point};

use crate::bus::{ALIAS_INVALID, BAD_SPEC, ENTRY_DANGLING, MEMBER_UNUSED, ON_SIMPLE_NET};
use crate::edit::{EditOp, Editor, Item, PinDef, SymbolDef};
use crate::hier::{
    HierConfig, PortDir, BLOCK_NAME, DIRECTION_MISMATCH, PIN_WITHOUT_PORT, PORT_WITHOUT_PIN,
};
use crate::model::{Id, PinKind, Schematic};

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
}

fn mm(v: i64) -> Nm {
    Nm::new(v * 1_000_000).unwrap()
}

fn symbol(name: &str) -> SymbolDef {
    SymbolDef {
        name: name.into(),
        pins: vec![
            PinDef {
                number: "1".into(),
                at: pt(0, 5),
                kind: Some(PinKind::Passive),
            },
            PinDef {
                number: "2".into(),
                at: pt(0, -5),
                kind: Some(PinKind::Passive),
            },
        ],
    }
}

fn do_op(ed: &mut Editor, op: EditOp) -> Vec<Id> {
    ed.apply(&op)
        .unwrap_or_else(|e| panic!("{op:?}: {e}"))
        .created
}

fn sheet(ed: &mut Editor, name: &str) -> Id {
    do_op(
        ed,
        EditOp::AddSheet {
            name: name.into(),
            width: mm(297),
            height: mm(210),
        },
    )[0]
}

fn place(ed: &mut Editor, sheet: Id, kind: &str, at: Point, refdes: &str, value: &str) -> Id {
    let made = do_op(
        ed,
        EditOp::PlaceSymbol {
            symbol: kind.into(),
            define: Some(symbol(kind)),
            sheet,
            at,
            rotation: None,
            mirrored: false,
            refdes: Some(refdes.into()),
            value: Some(value.into()),
        },
    );
    *made.last().unwrap()
}

fn wire(ed: &mut Editor, sheet: Id, path: &[Point]) {
    do_op(
        ed,
        EditOp::AddWire {
            sheet,
            path: path.to_vec(),
        },
    );
}

fn label(ed: &mut Editor, sheet: Id, at: Point, name: &str) {
    do_op(
        ed,
        EditOp::AddLabel {
            sheet,
            at,
            name: name.into(),
        },
    );
}

fn power(ed: &mut Editor, sheet: Id, at: Point, name: &str) {
    do_op(
        ed,
        EditOp::AddPowerPort {
            sheet,
            at,
            name: name.into(),
        },
    );
}

fn block(ed: &mut Editor, sheet: Id, child: Id, name: &str, at: Point) -> Id {
    do_op(
        ed,
        EditOp::AddSheetBlock {
            sheet,
            child,
            name: name.into(),
            at,
            width: mm(20),
            height: mm(20),
        },
    )[0]
}

fn hier_pin(ed: &mut Editor, block: Id, name: &str, dir: PortDir, at: Point) -> Id {
    do_op(
        ed,
        EditOp::AddHierPin {
            block,
            name: name.into(),
            dir,
            at,
        },
    )[0]
}

fn port(ed: &mut Editor, sheet: Id, name: &str, dir: PortDir, at: Point) -> Id {
    do_op(
        ed,
        EditOp::AddSheetPort {
            sheet,
            name: name.into(),
            dir,
            at,
        },
    )[0]
}

struct Design {
    ed: Editor,
    top: Id,
    child: Id,
    u1: Id,
    u2: Id,
}

fn design() -> Design {
    let mut ed = Editor::new(Schematic {
        name: "Two dividers".into(),
        ..Schematic::default()
    });
    let top = sheet(&mut ed, "Top");
    let child = sheet(&mut ed, "Divider");
    // the divider sheet
    place(&mut ed, child, "R", pt(50, 80), "R1", "1k");
    place(&mut ed, child, "R", pt(50, 60), "R2", "3k");
    wire(&mut ed, child, &[pt(50, 75), pt(50, 70), pt(50, 65)]);
    power(&mut ed, child, pt(50, 55), "GND");
    port(&mut ed, child, "IN", PortDir::Input, pt(50, 85));
    port(&mut ed, child, "OUT", PortDir::Output, pt(50, 70));
    // the top sheet
    place(&mut ed, top, "V", pt(20, 100), "V1", "10");
    power(&mut ed, top, pt(20, 95), "GND");
    let u1 = block(&mut ed, top, child, "U1", pt(60, 100));
    let u2 = block(&mut ed, top, child, "U2", pt(60, 70));
    hier_pin(&mut ed, u1, "IN", PortDir::Input, pt(60, 105));
    hier_pin(&mut ed, u1, "OUT", PortDir::Output, pt(80, 105));
    hier_pin(&mut ed, u2, "IN", PortDir::Input, pt(60, 75));
    hier_pin(&mut ed, u2, "OUT", PortDir::Output, pt(80, 75));
    wire(&mut ed, top, &[pt(20, 105), pt(60, 105)]);
    wire(&mut ed, top, &[pt(40, 105), pt(40, 75), pt(60, 75)]);
    label(&mut ed, top, pt(20, 105), "VIN");
    label(&mut ed, top, pt(80, 105), "VOUT1");
    label(&mut ed, top, pt(80, 75), "VOUT2");
    Design {
        ed,
        top,
        child,
        u1,
        u2,
    }
}

fn names(s: &Schematic) -> Vec<String> {
    let mut out: Vec<String> = s.nets.values().map(|n| n.name.clone()).collect();
    out.sort();
    out
}

fn codes(problems: &[eh3d_diagnostic::Diagnostic]) -> Vec<String> {
    problems
        .iter()
        .filter_map(|d| d.code.as_deref().map(str::to_owned))
        .collect()
}

#[test]
fn a_sheet_shown_twice_is_two_instances_with_hierarchical_references() {
    let d = design();
    let s = d.ed.schematic();
    assert!(s.is_consistent(), "{:?}", s.validate());
    let cfg = HierConfig::default();
    let h = s.hierarchy(&cfg);
    assert_eq!(h.roots, vec![d.top]);
    let shown: Vec<&str> = h.instances.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(shown, vec!["", "/U1", "/U2"]);
    assert_eq!(h.instances[1].path, vec!["U1".to_owned()]);
    assert_eq!(h.instances[1].parent, Some(0));
    assert_eq!(h.instances[1].block, Some(d.u1));
    assert_eq!(h.instances[2].sheet, d.child);
    let x = s.expand(&cfg).unwrap();
    assert!(x.problems.is_empty(), "{:?}", x.problems);
    let mut refdes: Vec<&str> = x
        .schematic
        .parts
        .values()
        .map(|p| p.refdes.as_str())
        .collect();
    refdes.sort_unstable();
    assert_eq!(refdes, vec!["U1/R1", "U1/R2", "U2/R1", "U2/R2", "V1"]);
    // the sheet was drawn once and the parts are copied: five parts, not three
    assert_eq!(s.parts.len(), 3);
    assert_eq!(x.origin.len(), 5);
}

#[test]
fn the_nets_have_stable_hierarchical_names_and_the_sheets_they_touch() {
    let d = design();
    let x = d.ed.schematic().expand(&HierConfig::default()).unwrap();
    assert_eq!(names(&x.schematic), vec!["/VIN", "/VOUT1", "/VOUT2", "GND"]);
    let rows = x.net_rows();
    let row = |name: &str| rows.iter().find(|r| r.name == name).unwrap();
    assert_eq!(row("/VIN").pins, 3);
    assert_eq!(row("/VIN").instances, vec!["/", "/U1", "/U2"]);
    assert_eq!(row("/VOUT1").pins, 2);
    assert_eq!(row("/VOUT1").instances, vec!["/U1"]);
    assert_eq!(row("/VOUT2").instances, vec!["/U2"]);
    assert_eq!(row("GND").pins, 3);
    assert_eq!(row("GND").instances, vec!["/", "/U1", "/U2"]);
    // the nets of the two instances are not one: U1 and U2 each have their own R1 pin 2
    let vout1 = x
        .schematic
        .nets
        .values()
        .find(|n| n.name == "/VOUT1")
        .unwrap();
    let refs: BTreeSet<&str> = vout1
        .pins
        .iter()
        .map(|p| x.schematic.parts[&p.part].refdes.as_str())
        .collect();
    assert_eq!(refs, BTreeSet::from(["U1/R1", "U1/R2"]));
    assert!(x.schematic.is_consistent(), "{:?}", x.schematic.validate());
}

#[test]
fn the_expansion_is_the_same_every_time() {
    let d = design();
    let cfg = HierConfig::default();
    let a = d.ed.schematic().expand(&cfg).unwrap();
    let b = d.ed.schematic().expand(&cfg).unwrap();
    assert_eq!(a.schematic, b.schematic);
    assert_eq!(
        a.schematic.to_bytes().unwrap(),
        b.schematic.to_bytes().unwrap()
    );
    // and so is the design: the same operations on a new editor give the same bytes
    let again = design();
    assert_eq!(
        d.ed.schematic().to_bytes().unwrap(),
        again.ed.schematic().to_bytes().unwrap()
    );
}

#[test]
fn the_reference_style_and_the_labels_follow_the_settings() {
    let d = design();
    let suffix = HierConfig::parse("refdes_style = suffix\nrefdes_separator = _\n").unwrap();
    let x = d.ed.schematic().expand(&suffix).unwrap();
    let mut refdes: Vec<&str> = x
        .schematic
        .parts
        .values()
        .map(|p| p.refdes.as_str())
        .collect();
    refdes.sort_unstable();
    assert_eq!(refdes, vec!["R1_U1", "R1_U2", "R2_U1", "R2_U2", "V1"]);
    // a sheet label that is global joins what the same name joins on the other instance
    let mut ed = d.ed;
    label(&mut ed, d.child, pt(50, 72), "SENSE");
    let local = ed.schematic().expand(&HierConfig::default()).unwrap();
    let count = |x: &crate::hier_expand::Expanded| x.schematic.nets.len();
    assert_eq!(count(&local), 4);
    let flat = HierConfig::parse("local_labels = false\n").unwrap();
    let global = ed.schematic().expand(&flat).unwrap();
    // with global labels the two SENSE labels join the two outputs: three nets
    assert_eq!(count(&global), 3, "{:?}", names(&global.schematic));
}

#[test]
fn operations_are_reversible_all_or_nothing_and_deterministic() {
    let mut d = design();
    let before = d.ed.schematic().clone();
    // a sheet cannot show itself, directly or through another sheet
    for (sheet, child) in [(d.top, d.top), (d.child, d.top)] {
        let err =
            d.ed.apply(&EditOp::AddSheetBlock {
                sheet,
                child,
                name: "LOOP".into(),
                at: pt(0, 0),
                width: mm(10),
                height: mm(10),
            })
            .unwrap_err();
        assert_eq!(err.code(), "PHOENIX-SCH-010");
    }
    // a name with a slash, a name used twice and a pin used twice are refused
    for bad in ["A/B", "U1", ""] {
        assert!(d
            .ed
            .apply(&EditOp::AddSheetBlock {
                sheet: d.top,
                child: d.child,
                name: bad.into(),
                at: pt(0, 0),
                width: mm(10),
                height: mm(10),
            })
            .is_err());
    }
    assert!(d
        .ed
        .apply(&EditOp::AddHierPin {
            block: d.u1,
            name: "IN".into(),
            dir: PortDir::Input,
            at: pt(0, 0),
        })
        .is_err());
    assert_eq!(d.ed.schematic(), &before);
    // a batch that fails in its second operation leaves nothing of the first
    let failed = d.ed.apply(&EditOp::Batch {
        ops: vec![
            EditOp::RenameSheet {
                sheet: d.top,
                name: "Renamed".into(),
            },
            EditOp::RemoveSheet { sheet: d.child },
        ],
    });
    assert!(failed.is_err());
    assert_eq!(d.ed.schematic(), &before);
    // undo and redo put back what was there, to the byte
    let depth = d.ed.undo_depth();
    do_op(
        &mut d.ed,
        EditOp::RenameSheetBlock {
            block: d.u2,
            name: "U9".into(),
        },
    );
    assert_ne!(d.ed.schematic(), &before);
    assert!(d.ed.undo());
    assert_eq!(d.ed.schematic(), &before);
    assert!(d.ed.redo());
    let renamed = d.ed.schematic().clone();
    assert_eq!(renamed.sheet_blocks[&d.u2].name, "U9");
    assert!(d.ed.undo());
    assert_eq!(d.ed.undo_depth(), depth);
    assert_eq!(
        d.ed.schematic().to_bytes().unwrap(),
        before.to_bytes().unwrap()
    );
}

#[test]
fn removing_a_sheet_needs_the_symbol_to_go_first_and_takes_what_is_on_it() {
    let mut d = design();
    assert!(d.ed.apply(&EditOp::RemoveSheet { sheet: d.child }).is_err());
    do_op(&mut d.ed, EditOp::RemoveSheetBlock { block: d.u1 });
    assert!(d.ed.apply(&EditOp::RemoveSheet { sheet: d.child }).is_err());
    do_op(&mut d.ed, EditOp::RemoveSheetBlock { block: d.u2 });
    do_op(&mut d.ed, EditOp::RemoveSheet { sheet: d.child });
    let s = d.ed.schematic();
    assert_eq!(s.sheets.len(), 1);
    assert_eq!(s.parts.len(), 1);
    assert!(s.sheet_blocks.is_empty() && s.hier_pins.is_empty() && s.sheet_ports.is_empty());
    assert!(s.is_consistent(), "{:?}", s.validate());
    // and all of it comes back with undo
    assert!(d.ed.undo());
    assert_eq!(d.ed.schematic().sheets.len(), 2);
    assert_eq!(d.ed.schematic().parts.len(), 3);
}

#[test]
fn the_checks_find_pins_without_ports_ports_without_pins_and_the_rest() {
    let mut d = design();
    assert!(codes(&d.ed.schematic().validate()).is_empty());
    let extra = hier_pin(&mut d.ed, d.u1, "EN", PortDir::Input, pt(70, 100));
    let got = codes(&d.ed.schematic().validate());
    assert_eq!(got, vec![PIN_WITHOUT_PORT.to_owned()]);
    do_op(&mut d.ed, EditOp::RemoveHierPin { pin: extra });
    // the port OUT without its pin on U2
    let out =
        d.ed.schematic()
            .hier_pins
            .iter()
            .find(|(_, p)| p.block == d.u2 && p.name == "OUT")
            .map(|(id, _)| *id)
            .unwrap();
    do_op(&mut d.ed, EditOp::RemoveHierPin { pin: out });
    assert_eq!(
        codes(&d.ed.schematic().validate()),
        vec![PORT_WITHOUT_PIN.to_owned()]
    );
    // a pin that is an input against a port that is an output
    hier_pin(&mut d.ed, d.u2, "OUT", PortDir::Input, pt(80, 75));
    assert_eq!(
        codes(&d.ed.schematic().validate()),
        vec![DIRECTION_MISMATCH.to_owned()]
    );
    // a symbol that has no name
    let mut s = d.ed.schematic().clone();
    s.sheet_blocks.get_mut(&d.u1).unwrap().name = "A/B".into();
    assert!(codes(&s.validate()).contains(&BLOCK_NAME.to_owned()));
}

#[test]
fn a_sheet_that_holds_itself_is_found_in_a_file_that_was_written_by_hand() {
    let d = design();
    let mut s = d.ed.schematic().clone();
    // the divider sheet shows the top sheet, which shows the divider sheet
    let block = s.sheet_blocks.get_mut(&d.u1).unwrap();
    block.sheet = d.child;
    block.child = d.top;
    block.name = "UP".into();
    let problems = s.hierarchy(&HierConfig::default()).problems;
    assert!(
        codes(&problems).contains(&crate::hier::SHEET_LOOP.to_owned()),
        "{problems:?}"
    );
    // expanding it does not hang
    let _ = s.expand(&HierConfig::default());
}

#[test]
fn moving_items_to_another_sheet_makes_the_nets_again() {
    let mut ed = Editor::new(Schematic::default());
    let a = sheet(&mut ed, "A");
    let b = sheet(&mut ed, "B");
    let r1 = place(&mut ed, a, "R", pt(30, 50), "R1", "1k");
    let r2 = place(&mut ed, a, "R", pt(50, 50), "R2", "1k");
    wire(&mut ed, a, &[pt(30, 45), pt(50, 45)]);
    assert_eq!(ed.schematic().nets.len(), 1);
    let wire_id = *ed.schematic().wires.keys().next().unwrap();
    do_op(
        &mut ed,
        EditOp::MoveToSheet {
            items: vec![Item::Part(r1), Item::Part(r2), Item::Wire(wire_id)],
            sheet: b,
            dx: 10_000_000,
            dy: 0,
        },
    );
    let s = ed.schematic();
    assert_eq!(s.parts[&r1].placement.unwrap().sheet, b);
    assert_eq!(s.parts[&r2].placement.unwrap().at, pt(60, 50));
    assert_eq!(s.wires[&wire_id].sheet, b);
    assert_eq!(s.wires[&wire_id].path[0], pt(40, 45));
    assert_eq!(s.nets.len(), 1);
    assert_eq!(s.nets.values().next().unwrap().pins.len(), 2);
    assert!(s.is_consistent(), "{:?}", s.validate());
    // only the part moves: the wire is left, so the connection is gone
    assert!(ed.undo());
    do_op(
        &mut ed,
        EditOp::MoveToSheet {
            items: vec![Item::Part(r1)],
            sheet: b,
            dx: 0,
            dy: 0,
        },
    );
    assert!(ed.schematic().nets.is_empty());
    // a sheet that does not exist is refused and nothing changes
    let before = ed.schematic().clone();
    assert!(ed
        .apply(&EditOp::MoveToSheet {
            items: vec![Item::Part(r1)],
            sheet: Id::from_parts(1_800_000_000_000, 1, 99).unwrap(),
            dx: 0,
            dy: 0,
        })
        .is_err());
    assert_eq!(ed.schematic(), &before);
}

fn bus_design() -> (Editor, Id, Id) {
    let mut ed = Editor::new(Schematic::default());
    let a = sheet(&mut ed, "A");
    let r1 = place(&mut ed, a, "R", pt(30, 50), "R1", "1k");
    let r2 = place(&mut ed, a, "R", pt(60, 50), "R2", "1k");
    let _ = (r1, r2);
    let bus = do_op(
        &mut ed,
        EditOp::PlaceBus {
            sheet: a,
            at: pt(45, 20),
            spec: "D[0..3]".into(),
        },
    )[0];
    (ed, a, bus)
}

#[test]
fn a_bus_entry_names_the_net_of_a_pin_like_a_label() {
    let (mut ed, _, bus) = bus_design();
    // pin 1 of R1 is at (30, 55) and pin 1 of R2 at (60, 55): the same member joins them
    do_op(
        &mut ed,
        EditOp::AddBusEntry {
            bus,
            member: "D1".into(),
            at: pt(30, 55),
        },
    );
    // a name on one pin is a net of one pin, as a label is
    assert_eq!(ed.schematic().nets.len(), 1);
    assert_eq!(ed.schematic().nets.values().next().unwrap().pins.len(), 1);
    do_op(
        &mut ed,
        EditOp::AddBusEntry {
            bus,
            member: "D1".into(),
            at: pt(60, 55),
        },
    );
    let s = ed.schematic();
    assert_eq!(s.nets.len(), 1);
    let net = s.nets.values().next().unwrap();
    assert_eq!(net.name, "D1");
    assert_eq!(net.pins.len(), 2);
    // a member that the bus does not have is refused
    assert!(ed
        .apply(&EditOp::AddBusEntry {
            bus,
            member: "D9".into(),
            at: pt(0, 0),
        })
        .is_err());
    // undo takes the second entry and the join away
    assert!(ed.undo());
    assert_eq!(ed.schematic().nets.len(), 1);
    assert_eq!(ed.schematic().nets.values().next().unwrap().pins.len(), 1);
}

#[test]
fn the_checks_of_the_buses_have_their_codes() {
    let (mut ed, a, bus) = bus_design();
    let c = |ed: &Editor| codes(&ed.schematic().bus_problems());
    // four members and no entry
    assert_eq!(c(&ed), vec![MEMBER_UNUSED.to_owned()]);
    for x in [30, 60] {
        do_op(
            &mut ed,
            EditOp::AddBusEntry {
                bus,
                member: "D0".into(),
                at: pt(x, 55),
            },
        );
    }
    assert_eq!(c(&ed), vec![MEMBER_UNUSED.to_owned()]);
    // an alias, a bus that uses it and a bus that is on a pin
    do_op(
        &mut ed,
        EditOp::SetBusAlias {
            name: "USB".into(),
            members: vec!["DP".into(), "DM".into()],
        },
    );
    let usb = do_op(
        &mut ed,
        EditOp::PlaceBus {
            sheet: a,
            at: pt(45, 30),
            spec: "USB".into(),
        },
    )[0];
    let mut got = c(&ed);
    got.sort();
    assert_eq!(
        got,
        vec![MEMBER_UNUSED.to_owned(), MEMBER_UNUSED.to_owned()]
    );
    let members = ed.schematic().bus_member_names(usb).unwrap();
    assert_eq!(members, vec!["USB.DP", "USB.DM"]);
    // removing the alias leaves the bus with a name that means nothing
    do_op(&mut ed, EditOp::RemoveBusAlias { name: "USB".into() });
    assert!(c(&ed).contains(&BAD_SPEC.to_owned()));
    // a bus whose anchor is a pin
    do_op(
        &mut ed,
        EditOp::PlaceBus {
            sheet: a,
            at: pt(30, 55),
            spec: "X[0..1]".into(),
        },
    );
    assert!(c(&ed).contains(&ON_SIMPLE_NET.to_owned()));
    // a file written by hand with an entry for a member that is gone
    let mut s = ed.schematic().clone();
    let entry = *s.bus_entries.keys().next().unwrap();
    s.bus_entries.get_mut(&entry).unwrap().member = "D8".into();
    assert!(codes(&s.bus_problems()).contains(&ENTRY_DANGLING.to_owned()));
    // an alias with a repeated item and one with a bad name
    do_op(
        &mut ed,
        EditOp::SetBusAlias {
            name: "TWO".into(),
            members: vec!["A".into(), "B".into()],
        },
    );
    let mut t = ed.schematic().clone();
    t.bus_aliases.values_mut().next().unwrap().members = vec!["A".into(), "A".into()];
    assert!(codes(&t.bus_problems()).contains(&ALIAS_INVALID.to_owned()));
    // the operations refuse a name that is not a bus
    assert!(ed
        .apply(&EditOp::PlaceBus {
            sheet: a,
            at: pt(0, 0),
            spec: "D[0..300]".into(),
        })
        .is_err());
    assert!(ed
        .apply(&EditOp::SetBusAlias {
            name: "BAD NAME".into(),
            members: vec!["A".into()],
        })
        .is_err());
}

#[test]
fn buses_and_sheet_symbols_are_written_only_when_there_are_some_and_round_trip() {
    let (s, _) = crate::sample::schematic();
    let plain = String::from_utf8(s.to_bytes().unwrap()).unwrap();
    for word in [
        "sheet_blocks",
        "hier_pins",
        "sheet_ports",
        "buses",
        "bus_entries",
        "bus_aliases",
    ] {
        assert!(!plain.contains(word), "{word}");
    }
    let d = design();
    let bytes = d.ed.schematic().to_bytes().unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("sheet_blocks") && text.contains("hier_pins"));
    let back = Schematic::from_bytes(&bytes).unwrap();
    assert_eq!(&back, d.ed.schematic());
    assert_eq!(back.to_bytes().unwrap(), bytes);
    let (ed, ..) = bus_design();
    let bytes = ed.schematic().to_bytes().unwrap();
    assert_eq!(
        Schematic::from_bytes(&bytes).unwrap().to_bytes().unwrap(),
        bytes
    );
}

#[test]
fn a_schematic_without_sheet_symbols_is_expanded_into_itself() {
    let (s, ..) = crate::sample::drawn_schematic();
    let x = s.expand(&HierConfig::default()).unwrap();
    assert_eq!(x.schematic, s);
    assert_eq!(x.origin.len(), s.parts.len());
}
