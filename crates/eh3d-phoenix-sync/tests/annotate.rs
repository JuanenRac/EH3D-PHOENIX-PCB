// SPDX-License-Identifier: GPL-3.0-or-later
//! Schematics and boards made by hand that differ on purpose: the plan of the annotation back into
//! the schematic must name every change with its state, apply only with the confirmation code,
//! undo exactly, and the comparison of the two documents must name every difference and conflict,
//! always in the same order.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, History, Id, Net, Pad, PadNet, PadShape, Side,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{NetPin, Part, Pin, Schematic, SchematicNet, Sheet, Symbol, Wire};
use eh3d_phoenix_sync::annot_codes::{title, CLASH, DIGEST_MISMATCH, LEFT_OUT};
use eh3d_phoenix_sync::{
    compare, plan_annotation, AnnotateError, AnnotateSettings, DiffKind, ItemKind, SchematicEdit,
    SchematicHistory, Status,
};

struct Fx {
    s: Schematic,
    b: Board,
    next: u16,
}

impl Fx {
    fn new() -> Fx {
        Fx {
            s: Schematic {
                name: "S".into(),
                ..Schematic::default()
            },
            b: Board::default(),
            next: 0,
        }
    }

    fn id(&mut self) -> Id {
        self.next += 1;
        Id::from_parts(1_700_000_000_000, self.next, u64::from(self.next)).unwrap()
    }

    /// A symbol with pins of the given numbers; returns the symbol and its pins.
    fn symbol(&mut self, name: &str, numbers: &[&str]) -> (Id, Vec<Id>) {
        let sid = self.id();
        let mut pins = BTreeMap::new();
        let mut ids = Vec::new();
        for number in numbers {
            let pid = self.id();
            pins.insert(
                pid,
                Pin {
                    number: (*number).to_owned(),
                    at: None,
                },
            );
            ids.push(pid);
        }
        self.s.symbols.insert(
            sid,
            Symbol {
                name: name.to_owned(),
                pins,
            },
        );
        (sid, ids)
    }

    /// A footprint on the board with pads of the given numbers.
    fn footprint(&mut self, name: &str, numbers: &[&str]) -> (Id, Vec<Id>) {
        let fid = self.id();
        let mut pads = BTreeMap::new();
        let mut ids = Vec::new();
        for number in numbers {
            let pid = self.id();
            pads.insert(
                pid,
                Pad {
                    number: (*number).to_owned(),
                    shape: PadShape::Circle {
                        diameter: Nm::new(1_000_000).unwrap(),
                    },
                    at: Point::from_nm(0, 0).unwrap(),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![],
                },
            );
            ids.push(pid);
        }
        self.b.footprints.insert(
            fid,
            Footprint {
                name: name.to_owned(),
                pads,
            },
        );
        (fid, ids)
    }

    fn part(&mut self, symbol: Id, refdes: &str, value: &str, footprint: Option<Id>) -> Id {
        let pid = self.id();
        self.s.parts.insert(
            pid,
            Part {
                refdes: refdes.to_owned(),
                value: value.to_owned(),
                symbol,
                footprint,
                placement: None,
            },
        );
        pid
    }

    fn comp(
        &mut self,
        part: Option<Id>,
        refdes: &str,
        value: &str,
        footprint: Id,
        at_mm: (i64, i64),
        side: Side,
    ) -> Id {
        let cid = self.id();
        self.b.components.insert(
            cid,
            Component {
                refdes: refdes.to_owned(),
                value: value.to_owned(),
                symbol: part,
                footprint,
                at: Point::from_nm(at_mm.0 * 1_000_000, at_mm.1 * 1_000_000).unwrap(),
                rotation: Udeg::ZERO,
                side,
            },
        );
        cid
    }

    fn sch_net(&mut self, name: &str, pins: &[(Id, Id)]) -> Id {
        let nid = self.id();
        let mut list: Vec<NetPin> = pins
            .iter()
            .map(|(part, pin)| NetPin {
                part: *part,
                pin: *pin,
            })
            .collect();
        list.sort();
        self.s.nets.insert(
            nid,
            SchematicNet {
                name: name.to_owned(),
                pins: list,
            },
        );
        nid
    }

    fn brd_net(&mut self, name: &str) -> Id {
        let nid = self.id();
        self.b.nets.insert(
            nid,
            Net {
                name: name.to_owned(),
            },
        );
        nid
    }

    fn connect(&mut self, component: Id, pad: Id, net: Id) {
        self.b.pad_nets.push(PadNet {
            component,
            pad,
            net,
        });
        self.b.pad_nets.sort_by_key(PadNet::pad_ref);
    }
}

/// A resistor symbol, two footprints for it and the ids of their pins and pads.
struct Rig {
    fx: Fx,
    sym: Id,
    pins: Vec<Id>,
    fp: Id,
    fp2: Id,
    pads: Vec<Id>,
}

fn rig() -> Rig {
    let mut fx = Fx::new();
    let (sym, pins) = fx.symbol("R", &["1", "2"]);
    let (fp, pads) = fx.footprint("R0603", &["1", "2"]);
    let (fp2, _) = fx.footprint("R0805", &["1", "2"]);
    Rig {
        fx,
        sym,
        pins,
        fp,
        fp2,
        pads,
    }
}

/// A part and the component made from it, with the fields given.
fn pair(
    r: &mut Rig,
    part_ref: &str,
    part_value: &str,
    comp_ref: &str,
    comp_value: &str,
    at_mm: (i64, i64),
) -> (Id, Id) {
    let p = r.fx.part(r.sym, part_ref, part_value, Some(r.fp));
    let c =
        r.fx.comp(Some(p), comp_ref, comp_value, r.fp, at_mm, Side::Top);
    (p, c)
}

fn kinds(plan: &eh3d_phoenix_sync::AnnotatePlan) -> Vec<(ItemKind, Status)> {
    plan.items.iter().map(|i| (i.kind, i.status)).collect()
}

fn refdes_of(s: &Schematic) -> Vec<String> {
    let mut names: Vec<String> = s.parts.values().map(|p| p.refdes.clone()).collect();
    names.sort();
    names
}

#[test]
fn the_fields_of_the_board_are_planned_and_applied_only_with_the_confirmation_code() {
    let mut r = rig();
    let (a, ca) = pair(&mut r, "R1", "10k", "R5", "22k", (0, 0));
    let (b, _) = pair(&mut r, "R2", "1k", "R2", "1k", (10, 0));
    r.fx.b.components.get_mut(&ca).unwrap().footprint = r.fp2;
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert_eq!(
        kinds(&plan),
        [
            (ItemKind::RefdesFromBoard, Status::Applicable),
            (ItemKind::ValueFromBoard, Status::Applicable),
            (ItemKind::FootprintFromBoard, Status::Applicable),
        ]
    );
    assert_eq!(
        plan.items.iter().map(|i| i.number).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(plan.items.iter().all(|i| i.location == "R1"));
    assert_eq!(
        (plan.items[0].from.as_str(), plan.items[0].to.as_str()),
        ("R1", "R5")
    );
    assert_eq!(
        (plan.items[2].from.as_str(), plan.items[2].to.as_str()),
        ("R0603", "R0805")
    );
    // the plan is the same every time, and a plan changes nothing
    assert_eq!(
        plan,
        plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default())
    );

    // a confirmation that is not the code of the plan applies nothing
    let original = r.fx.s.clone();
    let mut copy = original.clone();
    let mut history = SchematicHistory::new();
    let error = plan
        .apply_to_schematic(&mut copy, &mut history, "0000")
        .unwrap_err();
    assert!(matches!(error, AnnotateError::DigestMismatch { .. }));
    assert_eq!(error.code(), DIGEST_MISMATCH);
    assert_eq!(copy, original);
    assert!(history.done().is_empty());

    // with the code (in either case) the three changes are one step
    let code = plan.digest().to_uppercase();
    assert_eq!(
        plan.apply_to_schematic(&mut copy, &mut history, &code),
        Ok(3)
    );
    assert_eq!(history.done().len(), 1);
    assert_eq!(copy.parts[&a].refdes, "R5");
    assert_eq!(copy.parts[&a].value, "22k");
    assert_eq!(copy.parts[&a].footprint, Some(r.fp2));
    assert_eq!(copy.parts[&b], original.parts[&b]);
    // the plan of the result is empty, and the step is undone and redone exactly
    let again = plan_annotation(&copy, &r.fx.b, &AnnotateSettings::default());
    assert!(again.items.is_empty());
    assert!(history.undo(&mut copy));
    assert_eq!(copy, original);
    assert!(history.redo(&mut copy));
    assert_eq!(copy.parts[&a].refdes, "R5");
}

#[test]
fn the_text_and_the_json_of_the_plan_are_in_eight_languages() {
    let mut r = rig();
    pair(&mut r, "R1", "10k", "R5", "10k", (0, 0));
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    let en = plan.to_text(Lang::En);
    assert!(en.contains(
        "[applicable] PHOENIX-ANNOT-001 Reference designator differs on the board: R1 | R1 -> R5"
    ));
    assert!(en.contains(&plan.digest()));
    assert!(plan
        .to_text(Lang::Pt)
        .contains("A referência é diferente na placa"));
    assert!(plan.to_text(Lang::Pt).contains("aplicável"));
    for lang in Lang::ALL {
        let text = plan.to_text(lang);
        assert!(
            text.contains(title("PHOENIX-ANNOT-001", lang).unwrap()),
            "{lang:?}"
        );
    }
    let value: serde_json::Value = serde_json::from_str(&plan.to_json()).unwrap();
    assert_eq!(value["schema"], "phoenix-annotate/1");
    assert_eq!(value["digest"], plan.digest());
    assert_eq!(value["applicable"], 1);
    assert_eq!(value["items"][0]["rule"], "refdes_from_board");
    assert_eq!(value["items"][0]["status"], "applicable");
    let empty = plan_annotation(
        &Schematic::default(),
        &Board::default(),
        &AnnotateSettings::default(),
    );
    assert!(empty.to_text(Lang::En).contains("nothing found"));
}

#[test]
fn the_settings_can_leave_values_and_footprints_out_and_the_plan_says_so() {
    let mut r = rig();
    let (a, ca) = pair(&mut r, "R1", "10k", "R5", "22k", (0, 0));
    r.fx.b.components.get_mut(&ca).unwrap().footprint = r.fp2;
    let settings = AnnotateSettings::parse("values = false\nfootprints = false\n").unwrap();
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &settings);
    assert_eq!(
        kinds(&plan),
        [
            (ItemKind::RefdesFromBoard, Status::Applicable),
            (ItemKind::ValueFromBoard, Status::Ignored),
            (ItemKind::FootprintFromBoard, Status::Ignored),
        ]
    );
    assert_eq!(plan.items[1].code, LEFT_OUT);
    assert!(plan.items[1].reason.starts_with("the settings"));
    let mut copy = r.fx.s.clone();
    let applied = plan.apply_to_schematic(&mut copy, &mut SchematicHistory::new(), &plan.digest());
    assert_eq!(applied, Ok(1));
    assert_eq!(copy.parts[&a].refdes, "R5");
    assert_eq!(copy.parts[&a].value, "10k");
    assert_eq!(copy.parts[&a].footprint, Some(r.fp));
}

#[test]
fn references_that_would_clash_are_conflicts_and_are_never_applied() {
    let mut r = rig();
    pair(&mut r, "R1", "1k", "R9", "1k", (0, 0));
    pair(&mut r, "R2", "1k", "R9", "1k", (10, 0));
    pair(&mut r, "R3", "1k", "", "1k", (20, 0));
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert_eq!(plan.count(Status::Conflict), 3);
    assert!(plan.items.iter().all(|i| i.code == CLASH));
    assert!(plan.items[0].reason.contains("used twice"));
    assert!(plan.items[2].reason.contains("empty"));
    assert!(!plan.is_empty());
    let mut copy = r.fx.s.clone();
    let mut history = SchematicHistory::new();
    assert_eq!(
        plan.apply_to_schematic(&mut copy, &mut history, &plan.digest()),
        Ok(0)
    );
    assert_eq!(copy, r.fx.s);
    assert!(history.done().is_empty());
}

#[test]
fn references_that_trade_places_are_not_a_clash() {
    let mut r = rig();
    let (a, _) = pair(&mut r, "R1", "1k", "R2", "1k", (0, 0));
    let (b, _) = pair(&mut r, "R2", "2k", "R1", "2k", (10, 0));
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert_eq!(plan.count(Status::Applicable), 2);
    let mut copy = r.fx.s.clone();
    let done = plan.apply_to_schematic(&mut copy, &mut SchematicHistory::new(), &plan.digest());
    assert_eq!(done, Ok(2));
    assert_eq!(copy.parts[&a].refdes, "R2");
    assert_eq!(copy.parts[&b].refdes, "R1");
    assert!(
        plan_annotation(&copy, &r.fx.b, &AnnotateSettings::default())
            .items
            .is_empty()
    );
}

#[test]
fn what_only_one_document_has_is_listed_and_ignored_and_a_second_component_is_a_conflict() {
    let mut r = rig();
    let (p1, _) = pair(&mut r, "R1", "1k", "R1", "1k", (0, 0));
    // a part with no component, and a component with no part
    r.fx.part(r.sym, "R3", "1k", Some(r.fp));
    r.fx.comp(None, "X1", "x", r.fp, (50, 0), Side::Top);
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert_eq!(
        kinds(&plan),
        [
            (ItemKind::ComponentAdded, Status::Ignored),
            (ItemKind::PartRemoved, Status::Ignored),
        ]
    );
    assert_eq!(plan.items[0].location, "X1");
    assert_eq!(plan.items[1].location, "R3");
    // only ignored items: nothing to apply and nothing that clashes
    assert!(plan.is_empty());
    // a second component made from the same part clashes
    r.fx.comp(Some(p1), "R1b", "1k", r.fp, (60, 0), Side::Top);
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert_eq!(
        kinds(&plan),
        [
            (ItemKind::ComponentAdded, Status::Ignored),
            (ItemKind::PartRemoved, Status::Ignored),
            (ItemKind::Clash, Status::Conflict),
        ]
    );
    assert!(!plan.is_empty());
}

/// Four resistors on a grid: R10 and R20 on a row at the top, R30 and R40 on a row below that
/// leans by 1 mm; each part has a component of the same name.
fn grid() -> (Rig, [Id; 4], [Id; 4]) {
    let mut r = rig();
    let (p10, c10) = pair(&mut r, "R10", "1k", "R10", "1k", (10, 30));
    let (p20, c20) = pair(&mut r, "R20", "1k", "R20", "1k", (30, 30));
    let (p30, c30) = pair(&mut r, "R30", "1k", "R30", "1k", (10, 10));
    let (p40, c40) = pair(&mut r, "R40", "1k", "R40", "1k", (30, 11));
    (r, [p10, p20, p30, p40], [c10, c20, c30, c40])
}

fn renumbered(r: &Rig, settings: &str) -> Vec<(String, String)> {
    let settings = AnnotateSettings::parse(settings).unwrap();
    plan_annotation(&r.fx.s, &r.fx.b, &settings)
        .items
        .iter()
        .map(|i| (i.location.clone(), i.to.clone()))
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

#[test]
fn the_renumbering_follows_the_position_row_by_row_or_column_by_column() {
    let (r, parts, comps) = grid();
    // the Y axis grows upwards, so the row at y = 30 is the first, and the two lower components
    // are one row because they are within 2 mm
    assert_eq!(
        renumbered(&r, "renumber = true\n"),
        pairs(&[("R10", "R1"), ("R20", "R2"), ("R30", "R3"), ("R40", "R4")])
    );
    assert_eq!(
        renumbered(&r, "renumber = true\norder = left_to_right\n"),
        pairs(&[("R10", "R1"), ("R20", "R3"), ("R30", "R2"), ("R40", "R4")])
    );
    // with a tolerance of nothing the leaning row is two rows
    assert_eq!(
        renumbered(&r, "renumber = true\nrow_tolerance_nm = 0\n"),
        pairs(&[("R10", "R1"), ("R20", "R2"), ("R30", "R4"), ("R40", "R3")])
    );
    // a Y axis that grows downwards turns the order round
    assert_eq!(
        renumbered(&r, "renumber = true\ny_axis_up = false\n"),
        pairs(&[("R10", "R3"), ("R20", "R4"), ("R30", "R1"), ("R40", "R2")])
    );

    // the plan renames the parts of the schematic and the components of the board, as one step each
    let settings = AnnotateSettings::parse("renumber = true\n").unwrap();
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &settings);
    assert!(plan
        .items
        .iter()
        .all(|i| i.kind == ItemKind::Renumbered && i.status == Status::Applicable));
    assert!(plan
        .items
        .iter()
        .all(|i| i.edit.is_some() && i.board_edit.is_some()));
    let mut schematic = r.fx.s.clone();
    let mut board = r.fx.b.clone();
    let mut history = History::new();
    let code = plan.digest();
    assert_eq!(
        plan.apply_to_schematic(&mut schematic, &mut SchematicHistory::new(), &code),
        Ok(4)
    );
    assert_eq!(
        plan.apply_to_board(&mut board, &mut history, "wrong"),
        Err(AnnotateError::DigestMismatch {
            expected: code.clone(),
            given: "wrong".to_owned()
        })
    );
    assert_eq!(plan.apply_to_board(&mut board, &mut history, &code), Ok(4));
    for (part, comp) in parts.iter().zip(comps) {
        assert_eq!(schematic.parts[part].refdes, board.components[&comp].refdes);
    }
    assert_eq!(board.components[&comps[2]].refdes, "R3");
    // the result is numbered already
    assert!(plan_annotation(&schematic, &board, &settings)
        .items
        .is_empty());
    assert!(history.undo(&mut board));
    assert_eq!(board, r.fx.b);
}

#[test]
fn the_renumbering_keeps_locked_references_skips_taken_numbers_and_follows_the_settings() {
    let (mut r, _, comps) = grid();
    assert_eq!(
        renumbered(&r, "renumber = true\nlocked = r20\n"),
        pairs(&[("R10", "R1"), ("R30", "R2"), ("R40", "R3")])
    );
    assert_eq!(
        renumbered(&r, "renumber = true\nstart.R = 100\nstep.R = 5\n"),
        pairs(&[
            ("R10", "R100"),
            ("R20", "R105"),
            ("R30", "R110"),
            ("R40", "R115")
        ])
    );
    // a prefix that is not named is left alone
    assert!(renumbered(&r, "renumber = true\nprefixes = C\n").is_empty());
    // the sides are numbered apart
    r.fx.b.components.get_mut(&comps[3]).unwrap().side = Side::Bottom;
    assert_eq!(
        renumbered(&r, "renumber = true\nby_side = true\n"),
        pairs(&[("R10", "R1"), ("R20", "R2"), ("R30", "R3"), ("R40", "R101")])
    );
    // a number that a part with no component holds is never given again
    r.fx.part(r.sym, "R2", "1k", Some(r.fp));
    let list = renumbered(&r, "renumber = true\n");
    let taken: Vec<&str> = list
        .iter()
        .filter(|(location, _)| location != "R2")
        .map(|(_, to)| to.as_str())
        .collect();
    assert_eq!(taken, ["R1", "R3", "R4", "R5"]);
}

/// A connector of two interchangeable pins whose nets the board connects the other way round.
fn connector() -> (Fx, Id, Vec<Id>, [Id; 2]) {
    let mut fx = Fx::new();
    let (sym, pins) = fx.symbol("CONN2", &["1", "2"]);
    let (fp, pads) = fx.footprint("HDR2", &["1", "2"]);
    let j1 = fx.part(sym, "J1", "conn", Some(fp));
    let cj = fx.comp(Some(j1), "J1", "conn", fp, (0, 0), Side::Top);
    let a = fx.sch_net("A", &[(j1, pins[0])]);
    let b = fx.sch_net("B", &[(j1, pins[1])]);
    let (na, nb) = (fx.brd_net("A"), fx.brd_net("B"));
    fx.connect(cj, pads[0], nb);
    fx.connect(cj, pads[1], na);
    (fx, j1, pins, [a, b])
}

#[test]
fn equivalent_pins_that_the_board_swaps_are_planned_as_a_swap_that_undoes_exactly() {
    let (fx, j1, pins, [a, b]) = connector();
    // without the list of equivalent pins the swap is only a difference
    let plain = plan_annotation(&fx.s, &fx.b, &AnnotateSettings::default());
    assert!(plain.items.is_empty());
    let settings = AnnotateSettings::parse("swap.CONN2 = 1 | 2\n").unwrap();
    let plan = plan_annotation(&fx.s, &fx.b, &settings);
    assert_eq!(kinds(&plan), [(ItemKind::PinsSwapped, Status::Applicable)]);
    let item = &plan.items[0];
    assert_eq!(item.location, "J1 pins 1,2");
    assert_eq!(item.from, "1=A | 2=B");
    assert_eq!(item.to, "1=B | 2=A");
    let mut copy = fx.s.clone();
    let mut history = SchematicHistory::new();
    assert_eq!(
        plan.apply_to_schematic(&mut copy, &mut history, &plan.digest()),
        Ok(1)
    );
    let pin = |n: usize| NetPin {
        part: j1,
        pin: pins[n],
    };
    assert_eq!(copy.nets[&a].pins, [pin(1)]);
    assert_eq!(copy.nets[&b].pins, [pin(0)]);
    assert!(plan_annotation(&copy, &fx.b, &settings).items.is_empty());
    assert!(history.undo(&mut copy));
    assert_eq!(copy, fx.s);
    assert!(history.redo(&mut copy));
    assert_eq!(copy.nets[&a].pins, [pin(1)]);
}

#[test]
fn a_swap_the_net_list_cannot_take_is_a_conflict() {
    let settings = AnnotateSettings::parse("swap.CONN2 = 1 | 2\n").unwrap();
    // wires in the drawing would not follow a change of the net list
    let (mut fx, _, _, _) = connector();
    let sheet = fx.id();
    fx.s.sheets.insert(
        sheet,
        Sheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        },
    );
    let wire = fx.id();
    fx.s.wires.insert(
        wire,
        Wire {
            sheet,
            path: vec![
                Point::from_nm(0, 0).unwrap(),
                Point::from_nm(1_000_000, 0).unwrap(),
            ],
        },
    );
    let plan = plan_annotation(&fx.s, &fx.b, &settings);
    assert_eq!(kinds(&plan), [(ItemKind::PinsSwapped, Status::Conflict)]);
    assert_eq!(plan.items[0].code, CLASH);
    assert!(plan.items[0].reason.contains("wires"));
    assert_eq!(plan.items[0].edit, None);

    // a net name that two nets of the schematic share is not one net
    let (mut fx, _, _, _) = connector();
    fx.sch_net("A", &[]);
    let plan = plan_annotation(&fx.s, &fx.b, &settings);
    assert_eq!(kinds(&plan), [(ItemKind::PinsSwapped, Status::Conflict)]);
    assert!(plan.items[0].reason.contains("not one net"));
}

/// A part of two gates whose inputs and output the board connects as the other gate.
fn quad(swapped: bool) -> (Fx, Id, Vec<Id>) {
    let mut fx = Fx::new();
    let (sym, pins) = fx.symbol("NAND", &["1", "2", "3", "4", "5", "6"]);
    let (fp, pads) = fx.footprint("SO6", &["1", "2", "3", "4", "5", "6"]);
    let u1 = fx.part(sym, "U1", "nand", Some(fp));
    let cu = fx.comp(Some(u1), "U1", "nand", fp, (0, 0), Side::Top);
    let names = ["A", "B", "C", "D", "E", "F"];
    let board_nets: Vec<Id> = names.iter().map(|n| fx.brd_net(n)).collect();
    for (i, name) in names.iter().enumerate() {
        fx.sch_net(name, &[(u1, pins[i])]);
    }
    // the board connects the pads of the first gate to D, E, F and those of the second to A, B, C
    let order = if swapped {
        [3, 4, 5, 0, 1, 2]
    } else {
        [0, 1, 2, 3, 4, 5]
    };
    for (pad, net) in order.into_iter().enumerate() {
        fx.connect(cu, pads[pad], board_nets[net]);
    }
    (fx, u1, pins)
}

#[test]
fn gates_that_the_board_swaps_are_one_swap_of_all_their_pins() {
    let settings = AnnotateSettings::parse("swap.NAND = 1,2,3 | 4,5,6\n").unwrap();
    let (fx, u1, pins) = quad(true);
    let plan = plan_annotation(&fx.s, &fx.b, &settings);
    assert_eq!(kinds(&plan), [(ItemKind::PinsSwapped, Status::Applicable)]);
    assert_eq!(plan.items[0].location, "U1 pins 1,2,3,4,5,6");
    assert_eq!(plan.items[0].from, "1,2,3=A,B,C | 4,5,6=D,E,F");
    assert_eq!(plan.items[0].to, "1,2,3=D,E,F | 4,5,6=A,B,C");
    match &plan.items[0].edit {
        Some(SchematicEdit::SetPinNets { moves }) => assert_eq!(moves.len(), 6),
        other => panic!("{other:?}"),
    }
    let mut copy = fx.s.clone();
    plan.apply_to_schematic(&mut copy, &mut SchematicHistory::new(), &plan.digest())
        .unwrap();
    let pin_net: BTreeMap<NetPin, String> = copy
        .nets
        .values()
        .flat_map(|n| n.pins.iter().map(|p| (*p, n.name.clone())))
        .collect();
    let net_of = |i: usize| {
        pin_net[&NetPin {
            part: u1,
            pin: pins[i],
        }]
            .as_str()
    };
    assert_eq!(
        [
            net_of(0),
            net_of(1),
            net_of(2),
            net_of(3),
            net_of(4),
            net_of(5)
        ],
        ["D", "E", "F", "A", "B", "C"]
    );
    // gates that already agree are not a swap, and nets that are different are not either
    let (agree, _, _) = quad(false);
    assert!(plan_annotation(&agree.s, &agree.b, &settings)
        .items
        .is_empty());
    let (mut other, _, _) = quad(true);
    other.b.pad_nets.pop();
    assert!(plan_annotation(&other.s, &other.b, &settings)
        .items
        .is_empty());
}

/// The comparison: values, nets that only one document has and pins on a net in one only.
#[test]
fn the_comparison_lists_every_difference_in_a_fixed_order() {
    let mut r = rig();
    let (p1, c1) = pair(&mut r, "R1", "10k", "R1", "22k", (0, 0));
    let (p2, c2) = pair(&mut r, "R2", "1k", "R2", "1k", (10, 0));
    let pins = r.pins.clone();
    let pads = r.pads.clone();
    r.fx.sch_net("N1", &[(p1, pins[0]), (p2, pins[0])]);
    r.fx.sch_net("N2", &[(p1, pins[1])]);
    r.fx.sch_net("GND", &[(p2, pins[1])]);
    let (n1, gnd2) = (r.fx.brd_net("N1"), r.fx.brd_net("GND2"));
    r.fx.connect(c1, pads[0], n1);
    r.fx.connect(c2, pads[0], n1);
    r.fx.connect(c2, pads[1], gnd2);
    let report = compare(&r.fx.s, &r.fx.b);
    let listed: Vec<(DiffKind, &str, &str, &str)> = report
        .entries
        .iter()
        .map(|e| {
            (
                e.kind,
                e.location.as_str(),
                e.schematic.as_str(),
                e.board.as_str(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (DiffKind::Value, "R1", "10k", "22k"),
            (DiffKind::NetOneSide, "net GND", "1 pins", "-"),
            (DiffKind::NetOneSide, "net GND2", "-", "1 pads"),
            (DiffKind::NetOneSide, "net N2", "1 pins", "-"),
            (DiffKind::PinOneSide, "R1.2", "N2", "-"),
        ]
    );
    assert!(!report.in_sync());
    assert_eq!((report.differences(), report.conflicts()), (5, 0));
    // the same pair always gives the same report, as text and as JSON
    assert_eq!(report, compare(&r.fx.s, &r.fx.b));
    assert_eq!(report.to_json(), compare(&r.fx.s, &r.fx.b).to_json());
    for lang in Lang::ALL {
        let text = report.to_text(lang);
        assert!(text.contains("PHOENIX-ANNOT-015"), "{lang:?}");
        assert!(
            text.contains(title("PHOENIX-ANNOT-015", lang).unwrap()),
            "{lang:?}"
        );
        assert!(text.contains("R1.2"), "{lang:?}");
    }
    let value: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(value["schema"], "phoenix-diff/1");
    assert_eq!(value["in_sync"], false);
    assert_eq!(value["entries"][0]["rule"], "value");
    assert_eq!(value["entries"][0]["severity"], "warning");
}

#[test]
fn a_net_that_holds_other_pins_in_each_document_is_one_difference_per_net() {
    let mut r = rig();
    let (p1, c1) = pair(&mut r, "R1", "1k", "R1", "1k", (0, 0));
    let (p2, c2) = pair(&mut r, "R2", "1k", "R2", "1k", (10, 0));
    let pins = r.pins.clone();
    let pads = r.pads.clone();
    r.fx.sch_net("A", &[(p1, pins[0])]);
    r.fx.sch_net("B", &[(p2, pins[0])]);
    let (_, nb) = (r.fx.brd_net("A"), r.fx.brd_net("B"));
    r.fx.connect(c1, pads[0], nb);
    r.fx.connect(c2, pads[0], nb);
    let report = compare(&r.fx.s, &r.fx.b);
    let listed: Vec<(DiffKind, &str, &str, &str)> = report
        .entries
        .iter()
        .map(|e| {
            (
                e.kind,
                e.location.as_str(),
                e.schematic.as_str(),
                e.board.as_str(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (DiffKind::NetPins, "net A", "R1.1", "-"),
            (DiffKind::NetPins, "net B", "-", "R1.1"),
        ]
    );
}

#[test]
fn the_comparison_names_the_conflicts_that_copying_would_not_settle() {
    // one reference on two parts and on two components
    let mut r = rig();
    pair(&mut r, "R1", "1k", "R1", "1k", (0, 0));
    pair(&mut r, "R1", "1k", "R1", "1k", (10, 0));
    let report = compare(&r.fx.s, &r.fx.b);
    assert_eq!(
        report.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [DiffKind::RefdesTwice, DiffKind::RefdesTwice]
    );
    assert_eq!(report.conflicts(), 2);
    assert_eq!(report.entries[0].severity().as_str(), "error");

    // a reference that another item of the other document keeps
    let mut r = rig();
    pair(&mut r, "R1", "1k", "R2", "1k", (0, 0));
    r.fx.part(r.sym, "R2", "1k", Some(r.fp));
    r.fx.comp(None, "R1", "1k", r.fp, (50, 0), Side::Top);
    let report = compare(&r.fx.s, &r.fx.b);
    let kinds: Vec<DiffKind> = report.entries.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            DiffKind::PartOnly,
            DiffKind::ComponentOnly,
            DiffKind::Refdes,
            DiffKind::RefdesTaken,
            DiffKind::RefdesTaken,
        ]
    );
    assert_eq!(report.conflicts(), 2);

    // two components of one part
    let mut r = rig();
    let (p, _) = pair(&mut r, "R1", "1k", "R1", "1k", (0, 0));
    r.fx.comp(Some(p), "R1b", "1k", r.fp, (10, 0), Side::Top);
    let report = compare(&r.fx.s, &r.fx.b);
    assert_eq!(
        report.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [DiffKind::PartTwice]
    );
}

#[test]
fn two_documents_that_agree_have_an_empty_comparison_and_an_empty_plan() {
    let mut r = rig();
    let (p1, c1) = pair(&mut r, "R1", "1k", "R1", "1k", (0, 0));
    let (p2, c2) = pair(&mut r, "R2", "2k", "R2", "2k", (10, 0));
    let pins = r.pins.clone();
    let pads = r.pads.clone();
    r.fx.sch_net("N", &[(p1, pins[0]), (p2, pins[0])]);
    let n = r.fx.brd_net("N");
    r.fx.connect(c1, pads[0], n);
    r.fx.connect(c2, pads[0], n);
    assert!(compare(&r.fx.s, &r.fx.b).in_sync());
    let plan = plan_annotation(&r.fx.s, &r.fx.b, &AnnotateSettings::default());
    assert!(plan.items.is_empty() && plan.is_empty());
    assert_eq!(refdes_of(&r.fx.s), ["R1", "R2"]);
}
