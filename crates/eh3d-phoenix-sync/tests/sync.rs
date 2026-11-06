// SPDX-License-Identifier: GPL-3.0-or-later
//! Schematics and boards made by hand that differ on purpose: the plan must find every difference,
//! the package must apply, undo and redo exactly, and applying it twice must change nothing. A
//! seeded generator checks the same properties on many other pairs.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, History, Id, Layer, LayerKind, Net, Operation, Pad, PadNet,
    PadShape, Segment, Side, Track, Via,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{
    check_consistency, NetPin, Part, Pin, Schematic, SchematicNet, Symbol,
};
use eh3d_phoenix_sync::codes::{
    FOOTPRINT_UNUSABLE, NET_NAME_TWICE, PART_TWICE, PIN_WITHOUT_PAD, REFDES_CLASH,
};
use eh3d_phoenix_sync::{plan_backward, plan_forward, ChangeKind, SchematicHistory, SyncSettings};

fn id(n: u16) -> Id {
    Id::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

/// The ids that a plan makes new objects with; they never meet the ids of the fixtures.
fn id_source() -> impl FnMut() -> Id {
    let mut n: u16 = 40_000;
    move || {
        n += 1;
        Id::from_parts(1_700_000_001_000, n, u64::from(n)).unwrap()
    }
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn footprint(name: &str, pads: &[(Id, &str)]) -> Footprint {
    let mut map = BTreeMap::new();
    for (pad_id, number) in pads {
        map.insert(
            *pad_id,
            Pad {
                number: (*number).to_owned(),
                shape: PadShape::Circle {
                    diameter: nm(1_000_000),
                },
                at: pt(0, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![],
                roundness: 0,
                plated: true,
            },
        );
    }
    Footprint {
        name: name.to_owned(),
        pads: map,
    }
}

fn component(refdes: &str, value: &str, symbol: Option<Id>, fp: Id, x_mm: i64) -> Component {
    Component {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol,
        footprint: fp,
        at: pt(x_mm * 1_000_000, 1_000_000),
        rotation: Udeg::ZERO,
        side: Side::Top,
    }
}

fn pad_net(board: &mut Board, component: Id, pad: Id, net: Id) {
    board.pad_nets.push(PadNet {
        component,
        pad,
        net,
    });
    board.pad_nets.sort_by_key(PadNet::pad_ref);
}

// ids of the fixture
const SYM: u16 = 1;
const PIN1: u16 = 2;
const PIN2: u16 = 3;
const PART_A: u16 = 10;
const PART_B: u16 = 11;
const PART_D: u16 = 12;
const PART_E: u16 = 13;
const PART_C: u16 = 14;
const FP_R: u16 = 20;
const FP_C: u16 = 21;
const R_PAD1: u16 = 22;
const R_PAD2: u16 = 23;
const C_PAD1: u16 = 24;
const C_PAD2: u16 = 25;
const CA: u16 = 30;
const CB: u16 = 31;
const CD: u16 = 32;
const CE: u16 = 33;
const CX: u16 = 34;
const CF: u16 = 35;
const GONE: u16 = 99;
const N_GND: u16 = 40;
const N_SIG_OLD: u16 = 41;
const N_OLD: u16 = 42;
const N_X: u16 = 43;

fn schematic_fixture() -> Schematic {
    let mut s = Schematic {
        name: "S".into(),
        ..Schematic::default()
    };
    let mut pins = BTreeMap::new();
    for (pin, number) in [(PIN1, "1"), (PIN2, "2")] {
        pins.insert(
            id(pin),
            Pin {
                number: number.into(),
                at: None,
            },
        );
    }
    s.symbols.insert(
        id(SYM),
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    for (part, refdes, value, fp) in [
        (PART_A, "R1", "10k", FP_R),
        (PART_B, "R2", "1k", FP_R),
        (PART_D, "R7", "2k", FP_R),
        (PART_E, "R5", "3k", FP_C),
        (PART_C, "C1", "100n", FP_C),
    ] {
        s.parts.insert(
            id(part),
            Part {
                refdes: refdes.into(),
                value: value.into(),
                symbol: id(SYM),
                footprint: Some(id(fp)),
                placement: None,
            },
        );
    }
    let np = |part: u16, pin: u16| NetPin {
        part: id(part),
        pin: id(pin),
    };
    for (net, name, mut pins) in [
        (50u16, "GND", vec![np(PART_A, PIN2), np(PART_B, PIN2)]),
        (
            51,
            "SIG",
            vec![np(PART_A, PIN1), np(PART_B, PIN1), np(PART_D, PIN1)],
        ),
        (52, "N5", vec![np(PART_E, PIN1), np(PART_C, PIN1)]),
    ] {
        pins.sort();
        s.nets.insert(
            id(net),
            SchematicNet {
                name: name.into(),
                pins,
            },
        );
    }
    s
}

fn board_fixture() -> Board {
    let mut b = Board {
        name: "B".into(),
        ..Board::default()
    };
    b.layers.insert(
        id(60),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.copper_stack = vec![id(60)];
    b.footprints.insert(
        id(FP_R),
        footprint("R0603", &[(id(R_PAD1), "1"), (id(R_PAD2), "2")]),
    );
    b.footprints.insert(
        id(FP_C),
        footprint("C0603", &[(id(C_PAD1), "1"), (id(C_PAD2), "2")]),
    );
    for (n, name) in [
        (N_GND, "GND"),
        (N_SIG_OLD, "SIG_OLD"),
        (N_OLD, "OLD"),
        (N_X, "X"),
    ] {
        b.nets.insert(id(n), Net { name: name.into() });
    }
    for (c, refdes, value, symbol, x) in [
        (CA, "R1", "10k", Some(id(PART_A)), 1),
        (CB, "R2", "4k7", Some(id(PART_B)), 2),
        (CD, "R3", "2k", Some(id(PART_D)), 3),
        (CE, "R5", "3k", Some(id(PART_E)), 4),
        (CX, "R99", "x", Some(id(GONE)), 5),
        (CF, "TP1", "tp", None, 6),
    ] {
        b.components
            .insert(id(c), component(refdes, value, symbol, id(FP_R), x));
    }
    pad_net(&mut b, id(CA), id(R_PAD1), id(N_SIG_OLD));
    pad_net(&mut b, id(CA), id(R_PAD2), id(N_GND));
    pad_net(&mut b, id(CB), id(R_PAD1), id(N_SIG_OLD));
    pad_net(&mut b, id(CD), id(R_PAD1), id(N_SIG_OLD));
    pad_net(&mut b, id(CD), id(R_PAD2), id(N_X));
    pad_net(&mut b, id(CE), id(R_PAD1), id(N_OLD));
    b.tracks.insert(
        id(61),
        Track {
            net: id(N_SIG_OLD),
            layer: id(60),
            width: nm(200_000),
            path: vec![Segment::Line {
                start: pt(0, 0),
                end: pt(5_000_000, 0),
            }],
        },
    );
    b.vias.insert(
        id(62),
        Via {
            net: id(N_SIG_OLD),
            at: pt(5_000_000, 0),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(60),
            to_layer: id(60),
        },
    );
    b
}

fn kinds(changes: &[eh3d_phoenix_sync::Change]) -> Vec<(ChangeKind, &str)> {
    changes
        .iter()
        .map(|c| (c.kind, c.location.as_str()))
        .collect()
}

#[test]
fn the_plan_finds_every_difference_in_a_fixed_order_and_changes_nothing() {
    let (s, b) = (schematic_fixture(), board_fixture());
    let (s0, b0) = (s.clone(), b.clone());
    let package = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source());
    assert_eq!((&s, &b), (&s0, &b0));
    assert_eq!(
        kinds(&package.changes),
        [
            (ChangeKind::NewPart, "C1"),
            (ChangeKind::MissingPart, "R99"),
            (ChangeKind::RefdesDiffers, "R7"),
            (ChangeKind::ValueDiffers, "R2"),
            (ChangeKind::FootprintDiffers, "R5"),
            (ChangeKind::NetAdded, "net N5"),
            (ChangeKind::NetRenamed, "net SIG"),
            (ChangeKind::PadNetChanged, "C1.1"),
            (ChangeKind::PadNetChanged, "R2.2"),
            (ChangeKind::PadNetChanged, "R5.1"),
            (ChangeKind::PadNetChanged, "R7.2"),
        ]
    );
    let c = &package.changes;
    assert_eq!((c[2].from.as_str(), c[2].to.as_str()), ("R3", "R7"));
    assert_eq!((c[3].from.as_str(), c[3].to.as_str()), ("4k7", "1k"));
    assert_eq!((c[4].from.as_str(), c[4].to.as_str()), ("R0603", "C0603"));
    assert_eq!((c[6].from.as_str(), c[6].to.as_str()), ("SIG_OLD", "SIG"));
    assert_eq!((c[10].from.as_str(), c[10].to.as_str()), ("X", "-"));
    assert!(package.skipped.is_empty(), "{:?}", package.skipped);
    // one component, four edits of fields (two of them for the footprint) and six of nets
    assert_eq!(package.operations.len(), 11);
    // the same inputs always give the same package, as value and as bytes
    let again = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source());
    assert_eq!(package, again);
    assert_eq!(package.to_json(), again.to_json());
    let value: serde_json::Value = serde_json::from_str(&package.to_json()).unwrap();
    assert_eq!(value["schema"], "phoenix-sync-package/1");
    assert_eq!(value["direction"], "forward");
    assert_eq!(value["operations"].as_array().map(Vec::len), Some(11));
}

#[test]
fn applying_the_package_makes_the_board_agree_and_touches_nothing_else() {
    let (s, b0) = (schematic_fixture(), board_fixture());
    let package = plan_forward(&s, &b0, &SyncSettings::default(), &mut id_source());
    let mut b = b0.clone();
    let mut history = History::new();
    package.apply(&mut b, &mut history).unwrap();
    assert_eq!(history.done().len(), 1);

    // the new component is made from the part, by its id, and put where the settings say
    let (new_id, new_c) = b
        .components
        .iter()
        .find(|(_, c)| c.symbol == Some(id(PART_C)))
        .map(|(i, c)| (*i, c.clone()))
        .unwrap();
    assert_eq!(new_c.refdes, "C1");
    assert_eq!(new_c.footprint, id(FP_C));
    assert_eq!((new_c.at, new_c.side), (pt(0, 0), Side::Top));
    // the fields changed, and the places stayed
    assert_eq!(b.components[&id(CD)].refdes, "R7");
    assert_eq!(b.components[&id(CB)].value, "1k");
    assert_eq!(b.components[&id(CE)].footprint, id(FP_C));
    for c in [CA, CB, CD, CE, CX, CF] {
        assert_eq!(b.components[&id(c)].at, b0.components[&id(c)].at);
        assert_eq!(b.components[&id(c)].symbol, b0.components[&id(c)].symbol);
    }
    // the net is renamed, so the track and the via still belong to it
    assert_eq!(b.nets[&id(N_SIG_OLD)].name, "SIG");
    let n5 = b
        .nets
        .iter()
        .find(|(_, n)| n.name == "N5")
        .map(|(i, _)| *i)
        .unwrap();
    let net_of = |board: &Board, component: Id, pad: u16| {
        board.net_of(eh3d_phoenix_board::PadRef {
            component,
            pad: id(pad),
        })
    };
    assert_eq!(net_of(&b, id(CB), R_PAD1), Some(id(N_SIG_OLD)));
    assert_eq!(net_of(&b, id(CB), R_PAD2), Some(id(N_GND)));
    assert_eq!(net_of(&b, id(CD), R_PAD2), None);
    assert_eq!(net_of(&b, id(CE), C_PAD1), Some(n5));
    assert_eq!(net_of(&b, new_id, C_PAD1), Some(n5));
    assert_eq!(net_of(&b, new_id, C_PAD2), None);
    assert_eq!(
        b.pad_nets.iter().filter(|p| p.component == id(CE)).count(),
        1
    );
    // nothing the package does not name is touched
    assert_eq!(b.tracks, b0.tracks);
    assert_eq!(b.vias, b0.vias);
    assert_eq!(b.zones, b0.zones);
    assert_eq!(b.silkscreen, b0.silkscreen);
    assert_eq!(b.layers, b0.layers);
    assert_eq!(b.footprints, b0.footprints);
    assert_eq!(b.outline, b0.outline);
    assert!(b.components.contains_key(&id(CX)) && b.components.contains_key(&id(CF)));
    assert!(b.nets.contains_key(&id(N_OLD)) && b.nets.contains_key(&id(N_X)));
}

#[test]
fn applying_twice_changes_nothing_and_undo_and_redo_are_exact() {
    let (s, b0) = (schematic_fixture(), board_fixture());
    let settings = SyncSettings::default();
    let package = plan_forward(&s, &b0, &settings, &mut id_source());
    let mut b = b0.clone();
    let mut history = History::new();
    package.apply(&mut b, &mut history).unwrap();
    let after = b.clone();

    // the second time there is nothing to do (the only change left is the component that is kept)
    let second = plan_forward(&s, &b, &settings, &mut id_source());
    assert!(second.is_empty(), "{:?}", second.operations);
    assert_eq!(kinds(&second.changes), [(ChangeKind::MissingPart, "R99")]);
    assert!(second.report().in_sync());
    second.apply(&mut b, &mut history).unwrap();
    assert_eq!(b, after);
    assert_eq!(history.done().len(), 1);

    // undo gives back the original, byte for byte; redo gives the same board as the first time
    assert!(history.undo(&mut b));
    assert_eq!(b, b0);
    assert_eq!(b.to_bytes().unwrap(), b0.to_bytes().unwrap());
    assert!(!history.undo(&mut b));
    assert!(history.redo(&mut b));
    assert_eq!(b, after);
    assert_eq!(b.to_bytes().unwrap(), after.to_bytes().unwrap());
    assert!(!history.redo(&mut b));
}

#[test]
fn after_the_transfer_the_schematic_and_the_board_agree() {
    let (s, b0) = (schematic_fixture(), board_fixture());
    let before = check_consistency(&s, &b0);
    assert!(!before.agrees());
    let settings = SyncSettings::parse("remove_missing_components = true\n").unwrap();
    let package = plan_forward(&s, &b0, &settings, &mut id_source());
    assert_eq!(package.operations.len(), 12);
    let mut b = b0.clone();
    let mut history = History::new();
    package.apply(&mut b, &mut history).unwrap();
    let after = check_consistency(&s, &b);
    assert!(after.agrees(), "{:?}", after.errors);
    assert!(after.warnings.is_empty(), "{:?}", after.warnings);
    assert!(!b.components.contains_key(&id(CX)));
    // nothing is left to do
    assert!(plan_forward(&s, &b, &settings, &mut id_source()).is_empty());
    // the removal is one step with the rest and is undone with it
    assert!(history.undo(&mut b));
    assert_eq!(b, b0);
}

#[test]
fn what_cannot_be_done_is_listed_apart_and_never_guessed() {
    let (mut s, b) = (schematic_fixture(), board_fixture());
    // a part with no footprint, a part whose footprint the board does not have, a part whose
    // reference designator a component already holds, and a second net with an old name
    let mut extra = |part: u16, refdes: &str, footprint: Option<Id>| {
        s.parts.insert(
            id(part),
            Part {
                refdes: refdes.into(),
                value: "x".into(),
                symbol: id(SYM),
                footprint,
                placement: None,
            },
        );
    };
    extra(70, "R8", None);
    extra(71, "R9", Some(id(77)));
    extra(72, "TP1", Some(id(FP_R)));
    s.nets.insert(
        id(53),
        SchematicNet {
            name: "GND".into(),
            pins: vec![NetPin {
                part: id(PART_D),
                pin: id(PIN2),
            }],
        },
    );
    let package = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source());
    let mut why: Vec<(&str, &str)> = package
        .skipped
        .iter()
        .map(|k| (k.code, k.location.as_str()))
        .collect();
    why.sort_unstable();
    assert_eq!(
        why,
        [
            (FOOTPRINT_UNUSABLE, "R8"),
            (FOOTPRINT_UNUSABLE, "R9"),
            (REFDES_CLASH, "TP1"),
            (NET_NAME_TWICE, "net GND"),
        ]
    );
    assert!(package
        .changes
        .iter()
        .all(|c| !["R8", "R9", "TP1"].contains(&c.location.as_str())));
    // the skipped items are in the inconclusive list of the report, apart from the changes
    let report = package.report();
    assert_eq!(report.inconclusive.len(), 4);
    assert!(report.confirmed.iter().all(|f| !f.code.is_empty()));
    let mut b2 = b.clone();
    package.apply(&mut b2, &mut History::new()).unwrap();
    assert!(b2
        .components
        .values()
        .all(|c| c.refdes != "R8" && c.refdes != "R9"));
}

#[test]
fn a_pin_without_a_pad_and_a_part_made_into_two_components_are_reported() {
    let (mut s, mut b) = (schematic_fixture(), board_fixture());
    // the symbol gets a third pin that the footprints do not have, on a net with a second pin
    let pin3 = id(4);
    s.symbols.get_mut(&id(SYM)).unwrap().pins.insert(
        pin3,
        Pin {
            number: "3".into(),
            at: None,
        },
    );
    let mut pins = vec![
        NetPin {
            part: id(PART_A),
            pin: pin3,
        },
        NetPin {
            part: id(PART_B),
            pin: pin3,
        },
    ];
    pins.sort();
    s.nets.insert(
        id(54),
        SchematicNet {
            name: "EXTRA".into(),
            pins,
        },
    );
    // a second component made from part A
    b.components.insert(
        id(36),
        component("R1B", "10k", Some(id(PART_A)), id(FP_R), 7),
    );
    let package = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source());
    let codes: Vec<&str> = package.skipped.iter().map(|k| k.code).collect();
    assert_eq!(codes.iter().filter(|c| **c == PIN_WITHOUT_PAD).count(), 2);
    assert_eq!(codes.iter().filter(|c| **c == PART_TWICE).count(), 1);
    assert!(!package.changes.iter().any(|c| c.location == "net EXTRA"));
}

#[test]
fn the_report_is_in_the_eight_languages_and_in_json() {
    let (s, b) = (schematic_fixture(), board_fixture());
    let report = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source()).report();
    assert!(!report.in_sync());
    assert_eq!(report.pending, 11);
    let titles = [
        (Lang::En, "Reference designators differ"),
        (Lang::Es, "Las referencias son distintas"),
        (Lang::De, "Bauteilbezeichnungen unterscheiden sich"),
        (Lang::Fr, "Les références diffèrent"),
        (Lang::It, "I riferimenti sono diversi"),
        (Lang::Ja, "リファレンスが異なる"),
        (Lang::Zh, "位号不同"),
        (Lang::Pt, "As referências são diferentes"),
    ];
    for (lang, title) in titles {
        assert!(report.to_text(lang).contains(title), "{lang:?}");
    }
    let value: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(value["schema"], "phoenix-sync/1");
    assert_eq!(value["pending"], 11);
    assert_eq!(value["confirmed"].as_array().map(Vec::len), Some(11));
}

#[test]
fn backward_annotation_writes_the_board_into_the_parts_by_identity() {
    let (s0, mut b) = (schematic_fixture(), board_fixture());
    // the person renumbered and re-valued R1 on the board and changed its footprint
    {
        let c = b.components.get_mut(&id(CA)).unwrap();
        c.refdes = "R10".into();
        c.value = "22k".into();
        c.footprint = id(FP_C);
    }
    let settings = SyncSettings::default();
    let package = plan_backward(&s0, &b, &settings);
    assert_eq!(
        kinds(&package.changes),
        [
            (ChangeKind::RefdesDiffers, "R1"),
            (ChangeKind::RefdesDiffers, "R7"),
            (ChangeKind::ValueDiffers, "R1"),
            (ChangeKind::ValueDiffers, "R2"),
            (ChangeKind::FootprintDiffers, "R1"),
            (ChangeKind::FootprintDiffers, "R5"),
        ]
    );
    assert_eq!(package.edits.len(), 6);
    assert!(package.skipped.is_empty(), "{:?}", package.skipped);
    assert_eq!(package, plan_backward(&s0, &b, &settings));
    assert_eq!(
        package.to_json(),
        plan_backward(&s0, &b, &settings).to_json()
    );

    let mut s = s0.clone();
    let mut history = SchematicHistory::new();
    package.apply(&mut s, &mut history).unwrap();
    let a = &s.parts[&id(PART_A)];
    assert_eq!((a.refdes.as_str(), a.value.as_str()), ("R10", "22k"));
    assert_eq!(a.footprint, Some(id(FP_C)));
    assert_eq!(s.parts[&id(PART_B)].value, "4k7");
    assert_eq!(s.parts[&id(PART_D)].refdes, "R3");
    assert_eq!(s.parts[&id(PART_E)].footprint, Some(id(FP_R)));
    // what the edits do not name stays: the symbols, the nets and the part without a component
    assert_eq!(s.symbols, s0.symbols);
    assert_eq!(s.nets, s0.nets);
    assert_eq!(s.parts[&id(PART_C)], s0.parts[&id(PART_C)]);
    // again: nothing to do, nothing recorded
    let second = plan_backward(&s, &b, &settings);
    assert!(second.is_empty());
    second.apply(&mut s, &mut history).unwrap();
    assert_eq!(history.done().len(), 1);
    let after = s.clone();
    // undo and redo are exact, also as bytes
    assert!(history.undo(&mut s));
    assert_eq!(s, s0);
    assert_eq!(s.to_bytes().unwrap(), s0.to_bytes().unwrap());
    assert!(history.redo(&mut s));
    assert_eq!(s, after);
    assert!(!history.redo(&mut s));
    // the forward plan no longer finds a field that differs
    let forward = plan_forward(&after, &b, &settings, &mut id_source());
    assert!(forward.changes.iter().all(|c| !matches!(
        c.kind,
        ChangeKind::RefdesDiffers | ChangeKind::ValueDiffers | ChangeKind::FootprintDiffers
    )));
}

#[test]
fn a_reference_that_the_board_uses_twice_is_not_written_back() {
    let (s, mut b) = (schematic_fixture(), board_fixture());
    b.components.get_mut(&id(CB)).unwrap().refdes = "R1".into();
    let package = plan_backward(&s, &b, &SyncSettings::default());
    assert!(package
        .skipped
        .iter()
        .any(|k| k.code == REFDES_CLASH && k.location == "R2"));
    assert!(!package
        .changes
        .iter()
        .any(|c| c.kind == ChangeKind::RefdesDiffers && c.location == "R2"));
    // the footprint can be left alone by the settings
    let settings = SyncSettings::parse("backward_footprints = false\n").unwrap();
    let package = plan_backward(&s, &b, &settings);
    assert!(!package
        .changes
        .iter()
        .any(|c| c.kind == ChangeKind::FootprintDiffers));
}

#[test]
fn a_package_for_another_schematic_is_refused_and_changes_nothing() {
    let (s, b) = (schematic_fixture(), board_fixture());
    let mut package = plan_backward(&s, &b, &SyncSettings::default());
    assert!(!package.is_empty());
    let mut empty = Schematic::default();
    let before = empty.clone();
    let mut history = SchematicHistory::new();
    let error = package.apply(&mut empty, &mut history).unwrap_err();
    assert_eq!(error.code(), eh3d_phoenix_sync::codes::NOT_APPLIED);
    assert_eq!(empty, before);
    assert!(history.done().is_empty());
    assert!(!history.undo(&mut empty));
    package.edits.clear();
    assert!(package.is_empty());
}

#[test]
fn a_batch_of_the_board_is_all_or_nothing_when_the_board_is_not_the_one_planned_for() {
    let (s, b) = (schematic_fixture(), board_fixture());
    let package = plan_forward(&s, &b, &SyncSettings::default(), &mut id_source());
    let mut other = Board::default();
    let before = other.clone();
    assert!(other.apply(&package.operation()).is_err());
    assert_eq!(other, before);
    assert!(matches!(package.operation(), Operation::Batch { .. }));
}

// ---- generated pairs -------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

/// A schematic and a board that differ in a random way: some parts have no component, some fields
/// and footprints differ, nets are renamed or mixed up, and there are components without a part.
fn random_world(seed: u64) -> (Schematic, Board) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut counter: u16 = 100;
    let mut new = || {
        counter += 1;
        id(counter)
    };
    let mut s = Schematic::default();
    let mut b = Board::default();
    let layer = new();
    b.layers.insert(
        layer,
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    let sym = new();
    let pin_ids = [new(), new(), new()];
    let mut pins = BTreeMap::new();
    for (k, pin) in pin_ids.iter().enumerate() {
        pins.insert(
            *pin,
            Pin {
                number: (k + 1).to_string(),
                at: None,
            },
        );
    }
    s.symbols.insert(
        sym,
        Symbol {
            name: "S".into(),
            pins,
        },
    );
    // two footprints with three pads each
    let mut footprints: Vec<(Id, [Id; 3])> = Vec::new();
    for name in ["F1", "F2"] {
        let pad_ids = [new(), new(), new()];
        let fid = new();
        b.footprints.insert(
            fid,
            footprint(
                name,
                &[(pad_ids[0], "1"), (pad_ids[1], "2"), (pad_ids[2], "3")],
            ),
        );
        footprints.push((fid, pad_ids));
    }
    let nets_count = 1 + r.below(4) as usize;
    let board_net_ids: Vec<Id> = (0..nets_count).map(|_| new()).collect();
    for (j, net_id) in board_net_ids.iter().enumerate() {
        let name = if r.below(3) == 0 {
            format!("M{j}")
        } else {
            format!("N{j}")
        };
        b.nets.insert(*net_id, Net { name });
    }
    let values = ["1k", "2k", "10k"];
    let parts_count = 2 + r.below(6) as usize;
    let mut net_pins: Vec<Vec<NetPin>> = vec![Vec::new(); nets_count];
    let mut assignment: Vec<[Option<usize>; 3]> = Vec::new();
    let mut part_ids = Vec::new();
    for i in 0..parts_count {
        let part_id = new();
        part_ids.push(part_id);
        let footprint_choice = match r.below(5) {
            0 => None,
            1 | 2 => Some(footprints[0].0),
            _ => Some(footprints[1].0),
        };
        s.parts.insert(
            part_id,
            Part {
                refdes: format!("R{i}"),
                value: values[r.below(3) as usize].to_owned(),
                symbol: sym,
                footprint: footprint_choice,
                placement: None,
            },
        );
        let mut row = [None; 3];
        for (k, slot) in row.iter_mut().enumerate() {
            let choice = r.below(nets_count as u64 + 1) as usize;
            if choice < nets_count {
                *slot = Some(choice);
                net_pins[choice].push(NetPin {
                    part: part_id,
                    pin: pin_ids[k],
                });
            }
        }
        assignment.push(row);
    }
    for (j, mut pins) in net_pins.into_iter().enumerate() {
        pins.sort();
        s.nets.insert(
            new(),
            SchematicNet {
                name: format!("N{j}"),
                pins,
            },
        );
    }
    for (i, part_id) in part_ids.iter().enumerate() {
        if r.below(5) == 0 {
            continue; // no component for this part
        }
        let refdes = if r.below(5) == 0 {
            format!("X{i}")
        } else {
            format!("R{i}")
        };
        let value = if r.below(3) == 0 {
            "9k".to_owned()
        } else {
            s.parts[part_id].value.clone()
        };
        let (fid, pad_ids) = footprints[r.below(2) as usize];
        let cid = new();
        b.components.insert(
            cid,
            component(&refdes, &value, Some(*part_id), fid, i as i64),
        );
        for (k, pad) in pad_ids.iter().enumerate() {
            let wanted = assignment[i][k].map(|j| board_net_ids[j]);
            let net = match r.below(10) {
                0..=5 => wanted,
                6 | 7 => Some(board_net_ids[r.below(nets_count as u64) as usize]),
                _ => None,
            };
            if let Some(net) = net {
                pad_net(&mut b, cid, *pad, net);
            }
        }
    }
    if r.below(3) == 0 {
        b.components.insert(
            new(),
            component("ORPH", "o", Some(id(60_000)), footprints[0].0, 20),
        );
    }
    b.components
        .insert(new(), component("TP", "t", None, footprints[0].0, 21));
    b.tracks.insert(
        new(),
        Track {
            net: board_net_ids[0],
            layer,
            width: nm(200_000),
            path: vec![Segment::Line {
                start: pt(0, 0),
                end: pt(3_000_000, 0),
            }],
        },
    );
    (s, b)
}

#[test]
fn generated_pairs_keep_the_properties_of_the_package() {
    let settings = SyncSettings::default();
    for seed in 0..300u64 {
        let (s, b0) = random_world(seed);
        let package = plan_forward(&s, &b0, &settings, &mut id_source());
        let again = plan_forward(&s, &b0, &settings, &mut id_source());
        assert_eq!(package, again, "seed {seed}");
        assert_eq!(package.to_json(), again.to_json(), "seed {seed}");

        let mut b = b0.clone();
        let mut history = History::new();
        package.apply(&mut b, &mut history).unwrap();
        let after = b.clone();
        // nothing the package does not name is touched
        assert_eq!(b.tracks, b0.tracks, "seed {seed}");
        assert_eq!(b.footprints, b0.footprints, "seed {seed}");
        assert_eq!(b.layers, b0.layers, "seed {seed}");
        // every net assignment points at what exists, and the list stays sorted
        for p in &b.pad_nets {
            assert!(b.components.contains_key(&p.component), "seed {seed}");
            assert!(b.nets.contains_key(&p.net), "seed {seed}");
            let fp = &b.footprints[&b.components[&p.component].footprint];
            assert!(fp.pads.contains_key(&p.pad), "seed {seed}");
        }
        assert!(
            b.pad_nets
                .windows(2)
                .all(|w| w[0].pad_ref() < w[1].pad_ref()),
            "seed {seed}"
        );
        // applying it again finds nothing to do
        let second = plan_forward(&s, &b, &settings, &mut id_source());
        assert!(second.is_empty(), "seed {seed}: {:?}", second.operations);
        second.apply(&mut b, &mut history).unwrap();
        assert_eq!(b, after, "seed {seed}");
        assert_eq!(history.done().len(), usize::from(!package.is_empty()));
        // undo gives back the original and redo the result
        if !package.is_empty() {
            assert!(history.undo(&mut b), "seed {seed}");
            assert_eq!(b.to_bytes().unwrap(), b0.to_bytes().unwrap(), "seed {seed}");
            assert!(history.redo(&mut b), "seed {seed}");
            assert_eq!(b, after, "seed {seed}");
        }
    }
}

#[test]
fn generated_pairs_keep_the_properties_of_the_backward_package() {
    let settings = SyncSettings::default();
    for seed in 0..300u64 {
        let (s0, b) = random_world(seed);
        let package = plan_backward(&s0, &b, &settings);
        assert_eq!(package, plan_backward(&s0, &b, &settings), "seed {seed}");
        let mut s = s0.clone();
        let mut history = SchematicHistory::new();
        package.apply(&mut s, &mut history).unwrap();
        let after = s.clone();
        assert_eq!(s.nets, s0.nets, "seed {seed}");
        assert_eq!(s.symbols, s0.symbols, "seed {seed}");
        let second = plan_backward(&s, &b, &settings);
        assert!(second.is_empty(), "seed {seed}: {:?}", second.edits);
        second.apply(&mut s, &mut history).unwrap();
        assert_eq!(s, after, "seed {seed}");
        if !package.is_empty() {
            assert!(history.undo(&mut s), "seed {seed}");
            assert_eq!(s, s0, "seed {seed}");
            assert!(history.redo(&mut s), "seed {seed}");
            assert_eq!(s, after, "seed {seed}");
        }
    }
}
