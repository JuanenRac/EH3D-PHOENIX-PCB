//! Writing the footprints of a board and the symbols of a schematic as an EAGLE library (`.lbr`).
//!
//! * a package for every footprint of the board (the same packages as in a board file);
//! * a symbol for every symbol of the schematic, its pins at the positions the symbol gives (at
//!   the origin when it gives none, which is reported);
//! * a device set for every symbol: one gate, and one device for every footprint that a part of
//!   that symbol uses, connecting the pins to the pads of the same number (a pin without a pad is
//!   reported and left unconnected). A symbol none of whose parts has a footprint gets one device
//!   without a package.
//!
//! The library is written from the format as documented; no EAGLE has opened it.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_eagle::{write_document, Element};
use eh3d_phoenix_schematic::Schematic;

use crate::codes::{self, ITEM_SIMPLIFIED};
use crate::decimal;
use crate::layers::{assign, name_of};
use crate::writer::{el, package, package_names, Written};

const VERSION: &str = "9.6.2";

/// The letters at the start of a reference designator (`R` of `R12`), `U` when there are none.
fn prefix_of(refdes: &str) -> String {
    let letters: String = refdes
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if letters.is_empty() {
        "U".to_owned()
    } else {
        letters
    }
}

/// Unique names for the symbols: the name of the symbol, with a number added when two share one.
fn symbol_names(schematic: &Schematic, diagnostics: &mut Vec<Diagnostic>) -> BTreeMap<Id, String> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (id, symbol) in &schematic.symbols {
        let mut name = symbol.name.clone();
        let mut n = 1;
        while !used.insert(name.clone()) {
            n += 1;
            name = format!("{}_{n}", symbol.name);
        }
        if name != symbol.name {
            diagnostics.push(codes::diagnostic(
                ITEM_SIMPLIFIED,
                Severity::Info,
                &format!("symbol {id} is written as {name}: its name is used twice"),
            ));
        }
        names.insert(*id, name);
    }
    names
}

/// Writes the footprints of `board` and the symbols of `schematic` as an EAGLE library.
pub fn write_library(board: &Board, schematic: &Schematic) -> Written {
    let (numbers, mut diagnostics) = assign(board);
    let package_of = package_names(board, &mut diagnostics);
    let symbol_of = symbol_names(schematic, &mut diagnostics);

    let packages: Vec<Element> = package_of
        .iter()
        .map(|(id, name)| package(board, id, name, &numbers, &mut diagnostics))
        .collect();

    let mut symbols = Vec::new();
    for (id, symbol) in &schematic.symbols {
        let mut pins = Vec::new();
        for pin in symbol.pins.values() {
            let at = pin.at.map(|p| (p.x_nm.get(), p.y_nm.get()));
            if at.is_none() {
                diagnostics.push(codes::diagnostic(
                    ITEM_SIMPLIFIED,
                    Severity::Info,
                    &format!(
                        "pin {} of {} has no position and is written at the origin",
                        pin.number, symbol.name
                    ),
                ));
            }
            let (x, y) = at.unwrap_or((0, 0));
            pins.push(el(
                "pin",
                vec![
                    ("name", pin.number.clone()),
                    ("x", decimal(x)),
                    ("y", decimal(y)),
                    ("length", "short".to_owned()),
                    ("direction", "pas".to_owned()),
                ],
                Vec::new(),
            ));
        }
        symbols.push(el("symbol", vec![("name", symbol_of[id].clone())], pins));
    }

    let mut sets = Vec::new();
    for (id, symbol) in &schematic.symbols {
        let parts: Vec<_> = schematic
            .parts
            .values()
            .filter(|p| p.symbol == *id)
            .collect();
        let prefix = parts
            .first()
            .map_or_else(|| "U".to_owned(), |p| prefix_of(&p.refdes));
        let footprints: BTreeSet<Id> = parts.iter().filter_map(|p| p.footprint).collect();
        let mut devices = Vec::new();
        for footprint in &footprints {
            let (Some(package_name), Some(fp)) =
                (package_of.get(footprint), board.footprints.get(footprint))
            else {
                diagnostics.push(codes::diagnostic(
                    ITEM_SIMPLIFIED,
                    Severity::Warning,
                    &format!(
                        "a part of {} names a footprint that the board does not have",
                        symbol.name
                    ),
                ));
                continue;
            };
            let mut connects = Vec::new();
            for pin in symbol.pins.values() {
                if fp.pads.values().any(|pad| pad.number == pin.number) {
                    connects.push(el(
                        "connect",
                        vec![
                            ("gate", "G$1".to_owned()),
                            ("pin", pin.number.clone()),
                            ("pad", pin.number.clone()),
                        ],
                        Vec::new(),
                    ));
                } else {
                    diagnostics.push(codes::diagnostic(
                        ITEM_SIMPLIFIED,
                        Severity::Info,
                        &format!(
                            "pin {} of {} has no pad in {} and is not connected",
                            pin.number, symbol.name, package_name
                        ),
                    ));
                }
            }
            devices.push(el(
                "device",
                vec![
                    ("name", package_name.clone()),
                    ("package", package_name.clone()),
                ],
                vec![el("connects", Vec::new(), connects)],
            ));
        }
        if devices.is_empty() {
            devices.push(el("device", vec![("name", String::new())], Vec::new()));
        }
        sets.push(el(
            "deviceset",
            vec![("name", symbol_of[id].clone()), ("prefix", prefix)],
            vec![
                el(
                    "gates",
                    Vec::new(),
                    vec![el(
                        "gate",
                        vec![
                            ("name", "G$1".to_owned()),
                            ("symbol", symbol_of[id].clone()),
                            ("x", "0".to_owned()),
                            ("y", "0".to_owned()),
                        ],
                        Vec::new(),
                    )],
                ),
                el("devices", Vec::new(), devices),
            ],
        ));
    }

    let mut used: BTreeSet<u32> = numbers.values().copied().collect();
    used.extend([94, 95, 96]);
    let layers: Vec<Element> = used
        .iter()
        .map(|n| {
            el(
                "layer",
                vec![
                    ("number", n.to_string()),
                    ("name", name_of(*n)),
                    ("color", "1".to_owned()),
                    ("fill", "1".to_owned()),
                    ("visible", "yes".to_owned()),
                    ("active", "yes".to_owned()),
                ],
                Vec::new(),
            )
        })
        .collect();
    let root = el(
        "eagle",
        vec![("version", VERSION.to_owned())],
        vec![el(
            "drawing",
            Vec::new(),
            vec![
                el("settings", Vec::new(), Vec::new()),
                el("layers", Vec::new(), layers),
                el(
                    "library",
                    Vec::new(),
                    vec![
                        el("packages", Vec::new(), packages),
                        el("symbols", Vec::new(), symbols),
                        el("devicesets", Vec::new(), sets),
                    ],
                ),
            ],
        )],
    );
    Written {
        text: write_document(&root),
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Footprint, Layer, LayerKind, Pad, PadShape};
    use eh3d_phoenix_eagle::{read_bytes, DocumentKind, Limits};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
    use eh3d_phoenix_schematic::{Part, Pin, Symbol};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }
    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// A board with one footprint of two surface pads and a schematic with a two-pin symbol used
    /// by two parts, one of which uses the footprint.
    fn fixture() -> (Board, Schematic) {
        let (top, fp, p1, p2) = (id(1), id(2), id(3), id(4));
        let mut board = Board::default();
        board.layers.insert(
            top,
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        let mut pads = BTreeMap::new();
        for (pid, number, x) in [(p1, "1", -1_000_000), (p2, "2", 1_000_000)] {
            pads.insert(
                pid,
                Pad {
                    number: number.into(),
                    shape: PadShape::Rect {
                        width: nm(900_000),
                        height: nm(1_300_000),
                    },
                    at: pt(x, 0),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![top],
                },
            );
        }
        board.footprints.insert(
            fp,
            Footprint {
                name: "R0805".into(),
                pads,
            },
        );
        let (sym, pin1, pin2) = (id(10), id(11), id(12));
        let mut schematic = Schematic::default();
        let mut pins = BTreeMap::new();
        pins.insert(
            pin1,
            Pin {
                number: "1".into(),
                at: Some(pt(-2_540_000, 0)),
            },
        );
        pins.insert(
            pin2,
            Pin {
                number: "2".into(),
                at: None,
            },
        );
        schematic.symbols.insert(
            sym,
            Symbol {
                name: "R".into(),
                pins,
            },
        );
        for (pid, refdes, footprint) in [(id(20), "R1", Some(fp)), (id(21), "R2", None)] {
            schematic.parts.insert(
                pid,
                Part {
                    refdes: refdes.into(),
                    value: "10k".into(),
                    symbol: sym,
                    footprint,
                    placement: None,
                },
            );
        }
        (board, schematic)
    }

    #[test]
    fn the_library_reads_back_with_its_packages_symbols_and_device_sets() {
        let (board, schematic) = fixture();
        let written = write_library(&board, &schematic);
        let reading = read_bytes(written.text.as_bytes(), &Limits::default());
        assert!(reading.is_clean(), "{:?}", reading.diagnostics);
        let summary = reading.summary.unwrap();
        assert_eq!(summary.kind, DocumentKind::Library);
        assert_eq!(summary.packages.len(), 1);
        assert_eq!(summary.packages[0].name, "R0805");
        assert_eq!((summary.packages[0].pads, summary.packages[0].smds), (0, 2));
        assert_eq!(summary.symbols.len(), 1);
        assert_eq!(
            (summary.symbols[0].name.as_str(), summary.symbols[0].pins),
            ("R", 2)
        );
        assert_eq!(summary.device_sets.len(), 1);
        let set = &summary.device_sets[0];
        assert_eq!((set.name.as_str(), set.prefix.as_str()), ("R", "R"));
        assert_eq!((set.gates, set.devices, set.connects), (1, 1, 2));
    }

    #[test]
    fn the_same_inputs_give_the_same_text_and_a_missing_position_is_reported() {
        let (board, schematic) = fixture();
        let a = write_library(&board, &schematic);
        assert_eq!(a.text, write_library(&board, &schematic).text);
        assert!(a
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(crate::ITEM_SIMPLIFIED)
                && d.message.contains("no position")));
        assert!(a.text.contains("x=\"-2.54\""));
    }

    #[test]
    fn a_symbol_without_a_footprint_gets_a_device_without_a_package() {
        let (board, mut schematic) = fixture();
        for part in schematic.parts.values_mut() {
            part.footprint = None;
        }
        let written = write_library(&board, &schematic);
        let summary = read_bytes(written.text.as_bytes(), &Limits::default())
            .summary
            .unwrap();
        let set = &summary.device_sets[0];
        assert_eq!((set.devices, set.connects), (1, 0));
    }

    #[test]
    fn a_pin_without_a_pad_is_reported_and_not_connected() {
        let (mut board, schematic) = fixture();
        for pad in board.footprints.get_mut(&id(2)).unwrap().pads.values_mut() {
            if pad.number == "2" {
                pad.number = "9".into();
            }
        }
        let written = write_library(&board, &schematic);
        let summary = read_bytes(written.text.as_bytes(), &Limits::default())
            .summary
            .unwrap();
        assert_eq!(summary.device_sets[0].connects, 1);
        assert!(written
            .diagnostics
            .iter()
            .any(|d| d.message.contains("has no pad")));
    }

    #[test]
    fn prefixes_come_from_the_designators() {
        assert_eq!(prefix_of("R12"), "R");
        assert_eq!(prefix_of("IC3"), "IC");
        assert_eq!(prefix_of("12"), "U");
    }
}
