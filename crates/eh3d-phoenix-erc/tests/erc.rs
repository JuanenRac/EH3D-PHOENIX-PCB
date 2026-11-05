// SPDX-License-Identifier: GPL-3.0-or-later
//! Schematics made by hand with deliberate errors: the check must find every one of them, always,
//! and the same schematic with the same settings must always give the same report.

use std::collections::BTreeMap;

use eh3d_phoenix_erc::codes::{
    DUPLICATE_PIN_NUMBER, DUPLICATE_REFDES, EMPTY_REFDES, EMPTY_VALUE, FLOATING_WIRES,
    INPUT_WITHOUT_DRIVER, JUNCTION_ON_FLOATING_WIRES, NOT_JUDGED, PART_WITHOUT_FOOTPRINT,
    PIN_TYPE_CONFLICT, PIN_UNCONNECTED, PIN_WITHOUT_NUMBER, POWER_WITHOUT_SOURCE,
    SIMILAR_NET_NAMES, SINGLE_PIN_NET,
};
use eh3d_phoenix_erc::{check, ErcConfig, Level, Report, Rule};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_schematic::{
    Id, NetPin, Part, Pin, PinKind, Placement, Schematic, SchematicNet, Sheet, Symbol, Wire,
};

struct Builder {
    s: Schematic,
    next: u16,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            s: Schematic::default(),
            next: 0,
        }
    }

    fn id(&mut self) -> Id {
        self.next += 1;
        Id::from_parts(1_700_000_000_000, self.next, u64::from(self.next)).unwrap()
    }

    /// A symbol with pins of the given numbers and types; returns the symbol and its pins.
    fn symbol(&mut self, name: &str, pins: &[(&str, Option<PinKind>)]) -> (Id, Vec<Id>) {
        let sid = self.id();
        let mut map = BTreeMap::new();
        let mut ids = Vec::new();
        for (number, kind) in pins {
            let pid = self.id();
            map.insert(
                pid,
                Pin {
                    number: (*number).to_owned(),
                    at: None,
                },
            );
            if let Some(kind) = kind {
                self.s.pin_kinds.insert(pid, *kind);
            }
            ids.push(pid);
        }
        self.s.symbols.insert(
            sid,
            Symbol {
                name: name.to_owned(),
                pins: map,
            },
        );
        (sid, ids)
    }

    fn part(&mut self, refdes: &str, value: &str, symbol: Id, footprint: bool) -> Id {
        let pid = self.id();
        let footprint = if footprint { Some(self.id()) } else { None };
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

    fn net(&mut self, name: &str, pins: &[(Id, Id)]) {
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
    }
}

fn at(report: &Report, code: &str) -> Vec<String> {
    report
        .confirmed
        .iter()
        .filter(|f| f.code == code)
        .map(|f| f.location.clone())
        .collect()
}

fn detail(report: &Report, code: &str) -> Vec<String> {
    report
        .confirmed
        .iter()
        .filter(|f| f.code == code)
        .map(|f| f.detail.clone())
        .collect()
}

fn run(b: &Builder) -> Report {
    check(&b.s, &ErcConfig::default())
}

#[test]
fn a_clean_schematic_has_nothing_to_report() {
    let mut b = Builder::new();
    let (ic, ic_pins) = b.symbol(
        "IC",
        &[
            ("1", Some(PinKind::Output)),
            ("2", Some(PinKind::Input)),
            ("3", Some(PinKind::PowerIn)),
        ],
    );
    let (r, r_pins) = b.symbol(
        "R",
        &[("1", Some(PinKind::Passive)), ("2", Some(PinKind::Passive))],
    );
    let (reg, reg_pins) = b.symbol("REG", &[("1", Some(PinKind::PowerOut))]);
    let u1 = b.part("U1", "MCU", ic, true);
    let u2 = b.part("U2", "MCU", ic, true);
    let r1 = b.part("R1", "10k", r, true);
    let r2 = b.part("R2", "10k", r, true);
    let reg1 = b.part("REG1", "LDO", reg, true);
    b.net(
        "SIG",
        &[(u1, ic_pins[0]), (u2, ic_pins[1]), (r1, r_pins[0])],
    );
    b.net(
        "FB",
        &[
            (u2, ic_pins[0]),
            (u1, ic_pins[1]),
            (r1, r_pins[1]),
            (r2, r_pins[1]),
        ],
    );
    b.net(
        "VDD",
        &[
            (reg1, reg_pins[0]),
            (u1, ic_pins[2]),
            (u2, ic_pins[2]),
            (r2, r_pins[0]),
        ],
    );
    let report = run(&b);
    assert!(report.confirmed.is_empty(), "{:?}", report.confirmed);
    assert!(report.inconclusive.is_empty(), "{:?}", report.inconclusive);
    assert!(report.passes());
}

#[test]
fn pins_on_no_net_are_found_and_a_pin_meant_to_stay_open_is_not() {
    let mut b = Builder::new();
    let (s, pins) = b.symbol(
        "S",
        &[
            ("1", Some(PinKind::Passive)),
            ("2", Some(PinKind::Passive)),
            ("3", Some(PinKind::NotConnected)),
            ("4", None),
        ],
    );
    let a1 = b.part("A1", "x", s, true);
    let b1 = b.part("B1", "x", s, true);
    b.net("N", &[(a1, pins[0]), (b1, pins[0])]);
    let report = run(&b);
    assert_eq!(
        at(&report, PIN_UNCONNECTED),
        ["A1.2", "A1.4", "B1.2", "B1.4"]
    );
    assert_eq!(report.confirmed.len(), 4);
    assert!(!report.passes());
    assert_eq!(report.count(Severity::Error), 4);
}

#[test]
fn nets_with_one_pin_or_none_are_found_and_a_lone_open_pin_is_not() {
    let mut b = Builder::new();
    let (s, pins) = b.symbol("S", &[("1", Some(PinKind::Passive))]);
    let (nc, nc_pins) = b.symbol("NC", &[("1", Some(PinKind::NotConnected))]);
    let p1 = b.part("P1", "x", s, true);
    let n1 = b.part("N1", "x", nc, true);
    b.net("ALONE", &[(p1, pins[0])]);
    b.net("EMPTY", &[]);
    b.net("OPEN", &[(n1, nc_pins[0])]);
    let report = run(&b);
    assert_eq!(at(&report, SINGLE_PIN_NET), ["net ALONE", "net EMPTY"]);
    assert_eq!(detail(&report, SINGLE_PIN_NET), ["only P1.1", "no pin"]);
    assert_eq!(report.count(Severity::Warning), 2);
    assert!(report.passes());
}

#[test]
fn pin_types_the_matrix_does_not_allow_are_found_with_the_severity_of_the_matrix() {
    let mut b = Builder::new();
    let (o, o_pins) = b.symbol("O", &[("1", Some(PinKind::Output))]);
    let (bi, bi_pins) = b.symbol("B", &[("1", Some(PinKind::Bidirectional))]);
    let (nc, nc_pins) = b.symbol("NC", &[("1", Some(PinKind::NotConnected))]);
    let (p, p_pins) = b.symbol("P", &[("1", Some(PinKind::Passive))]);
    let (i, i_pins) = b.symbol("I", &[("1", Some(PinKind::Input))]);
    let u: Vec<Id> = ["U1", "U2", "U3"]
        .iter()
        .map(|r| b.part(r, "x", o, true))
        .collect();
    let b1 = b.part("B1", "x", bi, true);
    let n1 = b.part("N1", "x", nc, true);
    let p1 = b.part("P1", "x", p, true);
    let i1 = b.part("I1", "x", i, true);
    let i2 = b.part("I2", "x", i, true);
    b.net(
        "BUS",
        &[(u[0], o_pins[0]), (u[1], o_pins[0]), (u[2], o_pins[0])],
    );
    b.net("MIX", &[(u[0], o_pins[0]), (b1, bi_pins[0])]);
    b.net("TIED", &[(n1, nc_pins[0]), (p1, p_pins[0])]);
    b.net("INS", &[(i1, i_pins[0]), (i2, i_pins[0])]);
    // the same output pin cannot be on two nets in a real design, so give each net its own pins
    b.s.nets
        .values_mut()
        .filter(|n| n.name == "MIX")
        .for_each(|n| n.pins.retain(|np| np.part != u[0]));
    let report = run(&b);
    // BUS: three outputs, three pairs; TIED: open pin with a passive; MIX has one pin left
    assert_eq!(at(&report, PIN_TYPE_CONFLICT), ["net BUS", "net TIED"]);
    assert_eq!(
        detail(&report, PIN_TYPE_CONFLICT),
        [
            "output U1.1 with output U2.1 (and 2 more pairs of these types)",
            "passive P1.1 with not_connected N1.1"
        ]
    );
    assert!(report
        .confirmed
        .iter()
        .filter(|f| f.code == PIN_TYPE_CONFLICT)
        .all(|f| f.severity == Severity::Error));
}

#[test]
fn the_matrix_comes_from_the_settings_and_can_be_changed() {
    // two outputs are an error; an output with a bidirectional pin is a warning
    let mut both = Builder::new();
    let (o2, o2_pins) = both.symbol("O", &[("1", Some(PinKind::Output))]);
    let (bi2, bi2_pins) = both.symbol("B", &[("1", Some(PinKind::Bidirectional))]);
    let (x1, x2, y1) = (
        both.part("X1", "x", o2, true),
        both.part("X2", "x", o2, true),
        both.part("Y1", "x", bi2, true),
    );
    both.net("A", &[(x1, o2_pins[0]), (x2, o2_pins[0])]);
    let (z1, z2) = (
        both.part("Z1", "x", o2, true),
        both.part("Z2", "x", bi2, true),
    );
    both.net("B", &[(z1, o2_pins[0]), (z2, bi2_pins[0])]);
    both.net("C", &[(y1, bi2_pins[0])]);
    let default = check(&both.s, &ErcConfig::default());
    let severities: Vec<(String, Severity)> = default
        .confirmed
        .iter()
        .filter(|f| f.code == PIN_TYPE_CONFLICT)
        .map(|f| (f.location.clone(), f.severity))
        .collect();
    assert_eq!(
        severities,
        [
            ("net A".to_owned(), Severity::Error),
            ("net B".to_owned(), Severity::Warning)
        ]
    );
    // a line of the settings turns the pair into a warning, and another turns the other off
    let softer =
        ErcConfig::parse("matrix.output.output = warning\nmatrix.output.bidirectional = ok\n")
            .unwrap();
    let report = check(&both.s, &softer);
    let severities: Vec<(String, Severity)> = report
        .confirmed
        .iter()
        .filter(|f| f.code == PIN_TYPE_CONFLICT)
        .map(|f| (f.location.clone(), f.severity))
        .collect();
    assert_eq!(severities, [("net A".to_owned(), Severity::Warning)]);
    // the switch of the rule turns the whole matrix off
    let mut off = ErcConfig::default();
    off.set_level(Rule::PinTypeConflict, Level::Off);
    assert!(at(&check(&both.s, &off), PIN_TYPE_CONFLICT).is_empty());
    // and the matrix can be written in code as well as in a file
    let mut strict = ErcConfig::default();
    strict.set_pair_level(PinKind::Output, PinKind::Bidirectional, Level::Error);
    let strict_report = check(&both.s, &strict);
    assert!(!strict_report.passes());
}

#[test]
fn inputs_without_a_driver_and_supplies_without_a_source_are_found() {
    let mut b = Builder::new();
    let (inp, inp_pins) = b.symbol("I", &[("1", Some(PinKind::Input))]);
    let (pw, pw_pins) = b.symbol("P", &[("1", Some(PinKind::PowerIn))]);
    let (r, r_pins) = b.symbol("R", &[("1", Some(PinKind::Passive))]);
    let (src, src_pins) = b.symbol("S", &[("1", Some(PinKind::PowerOut))]);
    let (u, u_pins) = b.symbol("U", &[("1", None)]);
    let i1 = b.part("I1", "x", inp, true);
    let i2 = b.part("I2", "x", inp, true);
    let i3 = b.part("I3", "x", inp, true);
    let p1 = b.part("P1", "x", pw, true);
    let p2 = b.part("P2", "x", pw, true);
    let p3 = b.part("P3", "x", pw, true);
    let r1 = b.part("R1", "x", r, true);
    let s1 = b.part("S1", "x", src, true);
    let x1 = b.part("X1", "x", u, true);
    // two inputs and nothing else
    b.net("NODRV", &[(i1, inp_pins[0]), (i2, inp_pins[0])]);
    // an input with a passive pin on the net: a passive pin counts as a driver by default
    b.net("PASSIVE", &[(i3, inp_pins[0]), (r1, r_pins[0])]);
    // two power inputs with no source
    b.net("VX", &[(p1, pw_pins[0]), (p2, pw_pins[0])]);
    // a power input with a source
    b.net("VY", &[(p3, pw_pins[0]), (s1, src_pins[0])]);
    // an untyped pin could be the driver: this cannot be judged
    let (inp2, inp2_pins) = b.symbol("I2", &[("1", Some(PinKind::Input))]);
    let i4 = b.part("I4", "x", inp2, true);
    b.net("MAYBE", &[(i4, inp2_pins[0]), (x1, u_pins[0])]);
    let report = run(&b);
    assert_eq!(at(&report, INPUT_WITHOUT_DRIVER), ["net NODRV"]);
    assert_eq!(detail(&report, INPUT_WITHOUT_DRIVER), ["inputs I1.1, I2.1"]);
    assert_eq!(at(&report, POWER_WITHOUT_SOURCE), ["net VX"]);
    assert_eq!(
        detail(&report, POWER_WITHOUT_SOURCE),
        ["power inputs P1.1, P2.1"]
    );
    let judged: Vec<&str> = report
        .inconclusive
        .iter()
        .map(|f| f.location.as_str())
        .collect();
    assert_eq!(judged, ["net MAYBE", "schematic"]);
    assert!(report.inconclusive.iter().all(|f| f.code == NOT_JUDGED));
    assert!(report.inconclusive[1].detail.starts_with("1 pins on nets"));
    // a different list of drivers in the settings changes the answer
    let strict = ErcConfig::parse("drivers = output\n").unwrap();
    let report = check(&b.s, &strict);
    assert_eq!(
        at(&report, INPUT_WITHOUT_DRIVER),
        ["net NODRV", "net PASSIVE"]
    );
}

#[test]
fn net_names_that_look_alike_are_found_and_the_nets_of_a_bus_are_not() {
    let mut b = Builder::new();
    for name in [
        "GND", "GND1", "VCC", "Vcc", "D0", "D1", "DATA", "DATA0", "DATA1", "+5V", "+5V1", "CLK",
        "SDA",
    ] {
        b.net(name, &[]);
    }
    let report = run(&b);
    assert_eq!(
        at(&report, SIMILAR_NET_NAMES),
        [
            "net +5V / net +5V1",
            "net DATA / net DATA0",
            "net DATA / net DATA1",
            "net GND / net GND1",
            "net VCC / net Vcc",
        ]
    );
    // a longer minimum stem leaves the short names alone
    let config = ErcConfig::parse("similar_name_min_stem = 4\n").unwrap();
    let report = check(&b.s, &config);
    assert_eq!(
        at(&report, SIMILAR_NET_NAMES),
        ["net DATA / net DATA0", "net DATA / net DATA1"]
    );
}

#[test]
fn references_values_and_footprints_are_checked() {
    let mut b = Builder::new();
    let (s, _) = b.symbol("S", &[("1", Some(PinKind::Passive))]);
    b.part("R1", "", s, false);
    b.part("R1", "2k", s, true);
    b.part("R2", "  ", s, true);
    b.part(" ", "1k", s, true);
    let report = run(&b);
    assert_eq!(at(&report, DUPLICATE_REFDES), ["R1"]);
    assert_eq!(detail(&report, DUPLICATE_REFDES), ["2 parts"]);
    assert_eq!(at(&report, EMPTY_VALUE), ["R1", "R2"]);
    assert_eq!(at(&report, PART_WITHOUT_FOOTPRINT), ["R1"]);
    let empty = at(&report, EMPTY_REFDES);
    assert_eq!(empty.len(), 1);
    assert!(empty[0].starts_with("part "));
    // an error makes the report fail; a rule can be turned off or softened in the settings
    assert!(!report.passes());
    let config = ErcConfig::parse(
        "severity.duplicate_refdes = off\nseverity.empty_refdes = warning\nseverity.pin_unconnected = off\n",
    )
    .unwrap();
    let report = check(&b.s, &config);
    assert!(at(&report, DUPLICATE_REFDES).is_empty());
    assert!(report.passes());
    assert_eq!(report.count(Severity::Warning), 4);
}

#[test]
fn symbol_pins_without_a_number_or_with_a_repeated_one_are_found() {
    let mut b = Builder::new();
    b.symbol(
        "BAD",
        &[
            ("", Some(PinKind::Passive)),
            ("1", Some(PinKind::Passive)),
            ("1", Some(PinKind::Passive)),
            ("2", Some(PinKind::Passive)),
        ],
    );
    b.symbol("GOOD", &[("1", None), ("2", None)]);
    let report = run(&b);
    assert_eq!(at(&report, PIN_WITHOUT_NUMBER), ["symbol BAD"]);
    assert_eq!(detail(&report, PIN_WITHOUT_NUMBER), ["1 pins"]);
    assert_eq!(at(&report, DUPLICATE_PIN_NUMBER), ["symbol BAD"]);
    assert_eq!(
        detail(&report, DUPLICATE_PIN_NUMBER),
        ["number 1 on 2 pins"]
    );
    assert_eq!(report.confirmed.len(), 2);
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A sheet, a part with one pin at its origin placed at (50, 0) mm and the wires of the tests of
/// the loose wires; lengths in millimetres.
fn drawn() -> (Builder, Id) {
    let mut b = Builder::new();
    let sheet = b.id();
    b.s.sheets.insert(
        sheet,
        Sheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        },
    );
    let (s, pins) = b.symbol("S", &[("1", Some(PinKind::Passive))]);
    b.s.symbols
        .get_mut(&s)
        .unwrap()
        .pins
        .get_mut(&pins[0])
        .unwrap()
        .at = Some(pt(0, 0));
    let p = b.part("P1", "x", s, true);
    b.s.parts.get_mut(&p).unwrap().placement = Some(Placement {
        sheet,
        at: pt(50_000_000, 0),
        rotation: Udeg::ZERO,
    });
    let mm = |x: i64, y: i64| pt(x * 1_000_000, y * 1_000_000);
    let paths: Vec<Vec<Point>> = vec![
        // a loose corner
        vec![mm(0, 0), mm(10, 0), mm(10, 10)],
        // three pieces that meet at (30, 0)
        vec![mm(20, 0), mm(30, 0)],
        vec![mm(30, 0), mm(40, 0)],
        vec![mm(30, 0), mm(30, 10)],
        // a wire that starts at the pin of P1
        vec![mm(50, 0), mm(60, 0)],
    ];
    for path in paths {
        let id = b.id();
        b.s.wires.insert(id, Wire { sheet, path });
    }
    (b, sheet)
}

#[test]
fn wires_and_junctions_that_join_no_pin_are_found() {
    let (b, _) = drawn();
    let report = run(&b);
    assert_eq!(
        at(&report, FLOATING_WIRES),
        [
            "sheet Sheet 1 at (0, 0) nm",
            "sheet Sheet 1 at (20000000, 0) nm"
        ]
    );
    assert_eq!(detail(&report, FLOATING_WIRES), ["3 points", "4 points"]);
    assert_eq!(
        at(&report, JUNCTION_ON_FLOATING_WIRES),
        ["sheet Sheet 1 at (30000000, 0) nm"]
    );
    assert_eq!(
        detail(&report, JUNCTION_ON_FLOATING_WIRES),
        ["3 wire pieces meet"]
    );
    // the wire that starts on the pin of P1 is not loose, and it does not make the pin connected
    assert_eq!(at(&report, PIN_UNCONNECTED), ["P1.1"]);
}

#[test]
fn a_pin_without_a_position_makes_the_loose_wires_inconclusive() {
    let (mut b, sheet) = drawn();
    let (s, _) = b.symbol("T", &[("1", None)]);
    let q = b.part("Q1", "x", s, true);
    b.s.parts.get_mut(&q).unwrap().placement = Some(Placement {
        sheet,
        at: pt(0, 100_000_000),
        rotation: Udeg::ZERO,
    });
    let report = run(&b);
    assert!(at(&report, FLOATING_WIRES).is_empty());
    assert!(at(&report, JUNCTION_ON_FLOATING_WIRES).is_empty());
    let judged: Vec<&str> = report
        .inconclusive
        .iter()
        .map(|f| f.location.as_str())
        .collect();
    assert_eq!(
        judged,
        [
            "sheet Sheet 1 at (0, 0) nm",
            "sheet Sheet 1 at (20000000, 0) nm",
            "sheet Sheet 1 at (30000000, 0) nm"
        ]
    );
    assert!(report
        .inconclusive
        .iter()
        .all(|f| f.code == NOT_JUDGED && f.detail.contains("1 pins")));
}

/// A schematic with several kinds of error, built in the order given.
fn messy(reverse: bool) -> Schematic {
    let mut b = Builder::new();
    let (o, o_pins) = b.symbol(
        "O",
        &[("1", Some(PinKind::Output)), ("2", Some(PinKind::Input))],
    );
    let specs = [
        ("R1", "1k"),
        ("R1", "2k"),
        ("U1", ""),
        ("U2", "x"),
        ("U3", "y"),
    ];
    let mut order: Vec<usize> = (0..specs.len()).collect();
    if reverse {
        order.reverse();
    }
    let mut ids = vec![None; specs.len()];
    for i in order {
        // parts 0, 2 and 4 have a footprint, parts 1 and 3 have none
        ids[i] = Some(b.part(specs[i].0, specs[i].1, o, [true, false][i & 1]));
    }
    let ids: Vec<Id> = ids.into_iter().flatten().collect();
    let mut nets: Vec<(&str, Vec<(Id, Id)>)> = vec![
        (
            "BUS",
            vec![
                (ids[2], o_pins[0]),
                (ids[3], o_pins[0]),
                (ids[4], o_pins[0]),
            ],
        ),
        ("GND", vec![(ids[0], o_pins[1])]),
        ("GND1", vec![(ids[1], o_pins[1])]),
    ];
    if reverse {
        nets.reverse();
    }
    for (name, pins) in nets {
        b.net(name, &pins);
    }
    b.s
}

#[test]
fn the_same_schematic_always_gives_the_same_report_whatever_the_order_of_its_objects() {
    let config = ErcConfig::default();
    let one = check(&messy(false), &config);
    let two = check(&messy(true), &config);
    assert!(!one.confirmed.is_empty());
    assert_eq!(one, two);
    assert_eq!(one.to_json(), two.to_json());
    for lang in Lang::ALL {
        assert_eq!(one.to_text(lang), two.to_text(lang));
    }
    // a second run of the same schematic is the same too
    assert_eq!(check(&messy(false), &config), one);
    // the errors it holds were all found
    assert_eq!(at(&one, DUPLICATE_REFDES), ["R1"]);
    assert_eq!(at(&one, PIN_TYPE_CONFLICT), ["net BUS"]);
    assert_eq!(at(&one, SIMILAR_NET_NAMES), ["net GND / net GND1"]);
    assert_eq!(at(&one, EMPTY_VALUE), ["U1"]);
    // parts 1 and 3 (odd places) have no footprint: R1 (the second one) and U2
    assert_eq!(at(&one, PART_WITHOUT_FOOTPRINT), ["R1", "U2"]);
}

#[test]
fn the_report_is_in_the_eight_languages_and_in_json() {
    let (b, _) = drawn();
    let report = run(&b);
    let titles = [
        (Lang::En, "Wires join no pin"),
        (Lang::Es, "Cables que no unen ningún pin"),
        (Lang::De, "Leitungen ohne Pinverbindung"),
        (Lang::Fr, "Fils qui ne relient aucune broche"),
        (Lang::It, "Fili che non collegano alcun pin"),
        (Lang::Ja, "どのピンにもつながらない配線"),
        (Lang::Zh, "导线未连接任何引脚"),
        (Lang::Pt, "Fios que não ligam nenhum pino"),
    ];
    for (lang, title) in titles {
        assert!(report.to_text(lang).contains(title), "{lang:?}");
    }
    let value: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(value["schema"], "phoenix-erc/1");
    assert_eq!(value["passes"], false);
    assert!(value["confirmed"].as_array().is_some_and(|a| !a.is_empty()));
    assert_eq!(value["inconclusive"].as_array().map(Vec::len), Some(0));
}

#[test]
fn the_list_of_findings_is_bounded_and_says_when_it_was_cut() {
    let mut b = Builder::new();
    let (s, _) = b.symbol(
        "S",
        &[
            ("1", Some(PinKind::Passive)),
            ("2", Some(PinKind::Passive)),
            ("3", Some(PinKind::Passive)),
        ],
    );
    b.part("R1", "1k", s, true);
    let config = ErcConfig::parse("max_findings = 2\n").unwrap();
    let report = check(&b.s, &config);
    assert_eq!(report.confirmed.len(), 2);
    assert_eq!(report.inconclusive.len(), 1);
    assert_eq!(report.inconclusive[0].location, "report");
    assert!(report.inconclusive[0].detail.contains("2"));
}
