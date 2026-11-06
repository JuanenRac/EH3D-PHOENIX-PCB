// SPDX-License-Identifier: GPL-3.0-or-later
//! The symbols of the schematic model written as KiCad 8 symbols: the `.kicad_sym` library file,
//! and the `lib_symbols` of a schematic.
//!
//! The model keeps a symbol as a name and its numbered pins, with a position when it is known and
//! an electrical type when one is given. It has no drawing, so the body is a rectangle around the
//! pins and the report says so. The y axis of a symbol points up in KiCad as in the model, so the
//! pins are written as they are.

use std::collections::BTreeMap;

use eh3d_phoenix_schematic::{Id, PinKind, Schematic, Symbol};

use crate::settings::KicadSettings;
use crate::write_codes::{
    Export, ExportFile, ExportReport, CHANGED, DANGLING, DEFAULTED, ROUND_TRIP,
};
use crate::write_pcb::{check_syntax, clean_name, file_stem};
use crate::write_settings::WriteSettings;
use crate::write_sexpr::{at3, atom, effects, int, list, num, pair, render, string, Sx};

/// Where a pin goes in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PinPlan {
    pub(crate) id: Id,
    pub(crate) number: String,
    /// The point where wires join, y up.
    pub(crate) x: i64,
    pub(crate) y: i64,
    /// The direction in which the pin goes towards the body, in whole degrees.
    pub(crate) angle: i64,
    pub(crate) kind: &'static str,
}

fn direction(angle: i64) -> (i64, i64) {
    match angle {
        0 => (1, 0),
        90 => (0, 1),
        180 => (-1, 0),
        _ => (0, -1),
    }
}

/// The place, the direction and the type of every pin of a symbol, in the order of the ids.
pub(crate) fn plan_pins(
    symbol: &Symbol,
    kinds: &BTreeMap<Id, PinKind>,
    settings: &WriteSettings,
    report: &mut ExportReport,
) -> Vec<PinPlan> {
    let mut plans = Vec::new();
    let mut loose = 0i64;
    for (pin_id, pin) in &symbol.pins {
        let (x, y, angle) = match pin.at {
            Some(p) => {
                let (x, y) = (p.x_nm.get(), p.y_nm.get());
                let angle = if x.abs() >= y.abs() {
                    if x < 0 {
                        0
                    } else {
                        180
                    }
                } else if y > 0 {
                    270
                } else {
                    90
                };
                (x, y, angle)
            }
            None => {
                report.note(
                    DEFAULTED,
                    "pins without a position",
                    "put in a column on the left of the symbol",
                );
                let column = -(settings.pin_length_nm + 2 * settings.pin_pitch_nm);
                loose += 1;
                (column, -(loose - 1) * settings.pin_pitch_nm, 0)
            }
        };
        let kind = match kinds.get(pin_id) {
            Some(PinKind::NotConnected) => "no_connect",
            Some(kind) => kind.as_str(),
            None => {
                report.note(
                    DEFAULTED,
                    "pins without an electrical type",
                    "written as unspecified",
                );
                "unspecified"
            }
        };
        plans.push(PinPlan {
            id: *pin_id,
            number: pin.number.clone(),
            x,
            y,
            angle,
            kind,
        });
    }
    plans
}

/// The corners of the body of a symbol (lower left and upper right, y up): the box that the
/// inner ends of the pins span, widened to the margin when it is thinner than that.
fn body_box(plans: &[PinPlan], settings: &WriteSettings) -> (i64, i64, i64, i64) {
    let mut xs: Vec<i64> = Vec::new();
    let mut ys: Vec<i64> = Vec::new();
    for plan in plans {
        let (dx, dy) = direction(plan.angle);
        xs.push(plan.x + dx * settings.pin_length_nm);
        ys.push(plan.y + dy * settings.pin_length_nm);
    }
    let (mut x0, mut x1) = (
        xs.iter().copied().min().unwrap_or(0),
        xs.iter().copied().max().unwrap_or(0),
    );
    let (mut y0, mut y1) = (
        ys.iter().copied().min().unwrap_or(0),
        ys.iter().copied().max().unwrap_or(0),
    );
    let margin = settings.body_margin_nm;
    if x1 - x0 < 2 * margin {
        let centre = (x0 + x1) / 2;
        x0 = centre - margin;
        x1 = centre + margin;
    }
    if y1 - y0 < 2 * margin {
        let centre = (y0 + y1) / 2;
        y0 = centre - margin;
        y1 = centre + margin;
    }
    (x0, y0, x1, y1)
}

pub(crate) fn property(
    label: &str,
    text: &str,
    x: i64,
    y: i64,
    hide: bool,
    settings: &WriteSettings,
) -> Sx {
    list(
        "property",
        vec![
            string(label),
            string(text),
            at3(x, y, 0),
            effects(settings.text_size_nm, settings.text_thickness_nm, hide, &[]),
        ],
    )
}

/// The letters a reference starts with, `U` when it has none.
pub(crate) fn reference_prefix(refdes: &str) -> String {
    let letters: String = refdes
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() {
        "U".to_owned()
    } else {
        letters
    }
}

/// A symbol of the model as a `symbol` item. `name` is the name the item takes (`Library:Name`
/// inside a schematic, the plain name in a library) and `short` the name of its parts.
pub(crate) fn symbol_node(
    name: &str,
    short: &str,
    prefix: &str,
    plans: &[PinPlan],
    settings: &WriteSettings,
) -> Sx {
    let (x0, y0, x1, y1) = body_box(plans, settings);
    let size = settings.text_size_nm;
    let body = list(
        "symbol",
        vec![
            string(&format!("{short}_0_1")),
            list(
                "rectangle",
                vec![
                    pair("start", x0, y1),
                    pair("end", x1, y0),
                    list(
                        "stroke",
                        vec![
                            list("width", vec![num(settings.symbol_line_nm)]),
                            list("type", vec![atom("default")]),
                        ],
                    ),
                    list("fill", vec![list("type", vec![atom("background")])]),
                ],
            ),
        ],
    );
    let mut pins = vec![string(&format!("{short}_1_1"))];
    for plan in plans {
        let font = effects(size, settings.text_thickness_nm, false, &[]);
        pins.push(list(
            "pin",
            vec![
                atom(plan.kind),
                atom("line"),
                at3(plan.x, plan.y, plan.angle * 1_000_000),
                list("length", vec![num(settings.pin_length_nm)]),
                list("name", vec![string("~"), font.clone()]),
                list("number", vec![string(&plan.number), font]),
            ],
        ));
    }
    list(
        "symbol",
        vec![
            string(name),
            list("pin_names", vec![list("offset", vec![int(0)])]),
            list("exclude_from_sim", vec![atom("no")]),
            list("in_bom", vec![atom("yes")]),
            list("on_board", vec![atom("yes")]),
            property("Reference", prefix, 0, y1 + 2 * size, false, settings),
            property("Value", short, 0, y0 - 2 * size, false, settings),
            property("Footprint", "", 0, 0, true, settings),
            property("Datasheet", "~", 0, 0, true, settings),
            body,
            list("symbol", pins),
        ],
    )
}

/// The short name of each symbol, unique and without the characters KiCad does not allow.
pub(crate) fn symbol_names(
    schematic: &Schematic,
    report: &mut ExportReport,
) -> BTreeMap<Id, String> {
    let mut out: BTreeMap<Id, String> = BTreeMap::new();
    let mut used: BTreeMap<String, usize> = BTreeMap::new();
    for (id, symbol) in &schematic.symbols {
        let mut name = clean_name(&symbol.name, report);
        let seen = used.entry(name.clone()).or_insert(0);
        *seen += 1;
        if *seen > 1 {
            name = format!("{name}_{seen}");
            report.note(
                CHANGED,
                "symbols with a name that another symbol has",
                "the number of the symbol was added to the name",
            );
        }
        out.insert(*id, name);
    }
    out
}

/// The reference prefix of each symbol, from the first part that uses it.
pub(crate) fn prefixes(schematic: &Schematic) -> BTreeMap<Id, String> {
    let mut out: BTreeMap<Id, String> = BTreeMap::new();
    for part in schematic.parts.values() {
        out.entry(part.symbol)
            .or_insert_with(|| reference_prefix(&part.refdes));
    }
    out
}

/// Writes the symbols of a schematic as a `.kicad_sym` library.
pub fn write_symbol_library(schematic: &Schematic, settings: &WriteSettings) -> Export {
    let mut report = ExportReport::default();
    let names = symbol_names(schematic, &mut report);
    let prefixes = prefixes(schematic);
    let mut root: Vec<Sx> = vec![
        list("version", vec![int(settings.sch_version as i64)]),
        list("generator", vec![string(&settings.generator)]),
    ];
    let mut pins_written = 0usize;
    for (id, symbol) in &schematic.symbols {
        let plans = plan_pins(symbol, &schematic.pin_kinds, settings, &mut report);
        pins_written += plans.len();
        let short = names.get(id).cloned().unwrap_or_default();
        let prefix = prefixes.get(id).cloned().unwrap_or_else(|| "U".to_owned());
        root.push(symbol_node(&short, &short, &prefix, &plans, settings));
    }
    report.wrote("symbols", schematic.symbols.len());
    report.wrote("pins", pins_written);
    if !schematic.symbols.is_empty() {
        report.note_many(
            DEFAULTED,
            "symbols drawn as a rectangle around their pins",
            "the model has no drawing of a symbol",
            schematic.symbols.len(),
        );
    }
    let text = render(&list("kicad_symbol_lib", root));
    check_symbols(&text, schematic.symbols.len(), pins_written, &mut report);
    Export {
        files: vec![ExportFile {
            name: format!("{}.kicad_sym", file_stem(&schematic.name, "symbols")),
            text,
        }],
        report,
    }
}

/// Reads the library back and compares the number of symbols and of pins with what was written.
fn check_symbols(text: &str, symbols: usize, pins: usize, report: &mut ExportReport) {
    if !check_syntax(text, "the symbol library", report) {
        return;
    }
    let mut ids = match crate::sequential_ids() {
        Ok(ids) => ids,
        Err(error) => {
            report.note(ROUND_TRIP, "the check could not start", &error.to_string());
            return;
        }
    };
    match crate::symbols::read_symbol_library(text.as_bytes(), &KicadSettings::default(), &mut ids)
    {
        Ok(read) => {
            let got_pins: usize = read.symbols.values().map(|s| s.pins.len()).sum();
            if read.symbols.len() != symbols {
                report.note(
                    ROUND_TRIP,
                    "symbols",
                    &format!(
                        "{symbols} were written and {} were read back",
                        read.symbols.len()
                    ),
                );
            }
            if got_pins != pins {
                report.note(
                    ROUND_TRIP,
                    "pins of symbols",
                    &format!("{pins} were written and {got_pins} were read back"),
                );
            }
        }
        Err(diagnostic) => report.note(ROUND_TRIP, "the symbol library", &diagnostic.message),
    }
}

/// A note for a pin that names a part or a symbol the model does not have.
pub(crate) fn dangling_pin(report: &mut ExportReport) {
    report.note(
        DANGLING,
        "pins of nets that name a part or a pin the schematic does not have",
        "the pin is left out",
    );
}

#[cfg(test)]
pub(crate) mod sample {
    use std::collections::BTreeMap;

    use eh3d_phoenix_model::{Nm, Point};
    use eh3d_phoenix_schematic::{
        Id, NetPin, Part, Pin, PinKind, Placement, Schematic, SchematicNet, Sheet, Symbol, Wire,
    };

    pub(crate) fn sid(n: u16) -> Id {
        eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    pub(crate) fn spt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// Two resistors of one two-pin symbol on one sheet, a wire between them, a label, a junction
    /// and a power port, and a third resistor that has no place in the drawing.
    pub(crate) fn sample_schematic() -> Schematic {
        let mut s = Schematic {
            name: "Demo sheet".into(),
            ..Schematic::default()
        };
        let (sym, pin1, pin2) = (sid(1), sid(2), sid(3));
        let mut pins = BTreeMap::new();
        pins.insert(
            pin1,
            Pin {
                number: "1".into(),
                at: Some(spt(-5_080_000, 0)),
            },
        );
        pins.insert(
            pin2,
            Pin {
                number: "2".into(),
                at: Some(spt(5_080_000, 0)),
            },
        );
        s.symbols.insert(
            sym,
            Symbol {
                name: "R".into(),
                pins,
            },
        );
        s.pin_kinds.insert(pin1, PinKind::Passive);
        s.pin_kinds.insert(pin2, PinKind::Passive);
        let sheet = sid(10);
        s.sheets.insert(
            sheet,
            Sheet {
                name: "Main".into(),
                width: Nm::new(297_000_000).unwrap(),
                height: Nm::new(210_000_000).unwrap(),
            },
        );
        let (p1, p2, p3) = (sid(20), sid(21), sid(22));
        for (pid, refdes, x) in [(p1, "R1", 50_800_000), (p2, "R2", 101_600_000)] {
            s.parts.insert(
                pid,
                Part {
                    refdes: refdes.into(),
                    value: "10k".into(),
                    symbol: sym,
                    footprint: None,
                    placement: Some(Placement {
                        sheet,
                        at: spt(x, 100_000_000),
                        rotation: eh3d_phoenix_model::Udeg::ZERO,
                    }),
                },
            );
        }
        s.parts.insert(
            p3,
            Part {
                refdes: "R3".into(),
                value: "1k".into(),
                symbol: sym,
                footprint: None,
                placement: None,
            },
        );
        s.wires.insert(
            sid(30),
            Wire {
                sheet,
                path: vec![spt(55_880_000, 100_000_000), spt(96_520_000, 100_000_000)],
            },
        );
        s.labels.insert(
            sid(31),
            eh3d_phoenix_schematic::NetLabel {
                sheet,
                at: spt(76_200_000, 100_000_000),
                name: "SIG".into(),
            },
        );
        s.junctions.insert(
            sid(32),
            eh3d_phoenix_schematic::Junction {
                sheet,
                at: spt(76_200_000, 100_000_000),
            },
        );
        s.power_ports.insert(
            sid(33),
            eh3d_phoenix_schematic::PowerPort {
                sheet,
                at: spt(45_720_000, 100_000_000),
                name: "GND".into(),
            },
        );
        s.nets.insert(
            sid(40),
            SchematicNet {
                name: "SIG".into(),
                pins: vec![
                    NetPin {
                        part: p1,
                        pin: pin2,
                    },
                    NetPin {
                        part: p2,
                        pin: pin1,
                    },
                    NetPin {
                        part: p3,
                        pin: pin1,
                    },
                ],
            },
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::sample::sample_schematic;
    use super::*;
    use crate::write_sexpr::testkit::{arg, child, children, head, parse};

    #[test]
    fn a_symbol_library_has_the_symbols_pins_and_a_body() {
        let s = sample_schematic();
        let out = write_symbol_library(&s, &WriteSettings::default());
        assert_eq!(out.files.len(), 1);
        assert_eq!(out.files[0].name, "Demo sheet.kicad_sym");
        let tree = parse(&out.files[0].text).unwrap();
        assert_eq!(head(&tree), Some("kicad_symbol_lib"));
        let symbols = children(&tree, "symbol");
        assert_eq!(symbols.len(), 1);
        assert_eq!(arg(symbols[0], 0), Some("R"));
        let parts = children(symbols[0], "symbol");
        assert_eq!(parts.len(), 2);
        assert_eq!(arg(parts[0], 0), Some("R_0_1"));
        assert!(child(parts[0], "rectangle").is_some());
        assert_eq!(arg(parts[1], 0), Some("R_1_1"));
        let pins = children(parts[1], "pin");
        assert_eq!(pins.len(), 2);
        assert_eq!(arg(pins[0], 0), Some("passive"));
        // the pin on the left points to the right (angle 0), the other one to the left
        let angle = |pin: &crate::write_sexpr::Sx| -> Option<String> {
            let at = child(pin, "at")?;
            arg(at, 2).map(str::to_owned)
        };
        assert_eq!(angle(pins[0]).as_deref(), Some("0"));
        assert_eq!(angle(pins[1]).as_deref(), Some("180"));
        assert!(
            !out.report.has_problem(),
            "{}",
            out.report.text(crate::error::Language::En)
        );
        assert_eq!(out.report.written_count("pins"), 2);
    }

    #[test]
    fn pins_without_a_position_or_a_type_are_counted() {
        let mut s = sample_schematic();
        for symbol in s.symbols.values_mut() {
            for pin in symbol.pins.values_mut() {
                pin.at = None;
            }
        }
        s.pin_kinds.clear();
        let out = write_symbol_library(&s, &WriteSettings::default());
        assert_eq!(out.report.count_of(DEFAULTED), 2 + 2 + 1);
        assert!(!out.report.has_problem());
    }

    #[test]
    fn the_library_reads_back_with_the_reader_of_the_crate() {
        let s = sample_schematic();
        let out = write_symbol_library(&s, &WriteSettings::default());
        let mut ids = crate::sequential_ids().unwrap();
        let read = crate::symbols::read_symbol_library(
            out.files[0].text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
        )
        .unwrap();
        assert_eq!(read.symbols.len(), 1);
        let symbol = read.symbols.values().next().unwrap();
        assert_eq!(symbol.name, "R");
        let numbers: Vec<&str> = symbol.pins.values().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers.len(), 2);
        assert!(numbers.contains(&"1") && numbers.contains(&"2"));
    }

    #[test]
    fn two_symbols_with_one_name_get_two_names() {
        let mut s = sample_schematic();
        let mut other = s.symbols.values().next().unwrap().clone();
        other.name = "R".into();
        s.symbols.insert(crate::write_lib::sample::sid(50), other);
        let out = write_symbol_library(&s, &WriteSettings::default());
        let tree = parse(&out.files[0].text).unwrap();
        let names: Vec<&str> = children(&tree, "symbol")
            .iter()
            .filter_map(|n| arg(n, 0))
            .collect();
        assert_eq!(names, vec!["R", "R_2"]);
        assert!(out.report.count_of(CHANGED) >= 1);
    }

    #[test]
    fn reference_prefixes_follow_the_first_part() {
        assert_eq!(reference_prefix("R12"), "R");
        assert_eq!(reference_prefix("IC3"), "IC");
        assert_eq!(reference_prefix("12"), "U");
        assert_eq!(reference_prefix(""), "U");
    }
}
