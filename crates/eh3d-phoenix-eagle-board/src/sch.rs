//! Writing a schematic as an EAGLE XML file (`.sch`).
//!
//! The file embeds the library of [`crate::write_library`] (packages for the footprints of the
//! board, symbols and device sets for the symbols of the schematic), one `part` for every part, and
//! one `sheet` for every sheet with the instances of the parts placed on it and the nets with their
//! pin references and wires. A wire belongs to the net of the pins it reaches, directly or through
//! other wires. A schematic without sheets is written as one sheet that holds every net.
//!
//! Not kept: the name and size of a sheet, the ids, the net list of a pin that has no part. What is
//! not kept is reported (`PHOENIX-EAGLE-020`, `PHOENIX-EAGLE-021`). No EAGLE has opened the file.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_eagle::write_document;
use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use eh3d_phoenix_schematic::{NetPin, Schematic};

use crate::codes::{self, ITEM_LEFT_OUT, ITEM_SIMPLIFIED};
use crate::decimal;
use crate::layers::{assign, name_of};
use crate::library::{symbol_names, write_library};
use crate::writer::{el, package_names, Written, LIBRARY};

const VERSION: &str = "9.6.2";

fn rot_text(micro_degrees: i64) -> String {
    format!("R{}", decimal(micro_degrees.rem_euclid(360_000_000)))
}

/// Writes `schematic` as an EAGLE schematic file; `board` supplies the footprints of the library.
pub fn write_schematic(board: &Board, schematic: &Schematic) -> Written {
    let library = write_library(board, schematic);
    let mut diagnostics = library.diagnostics;
    let mut library_element = parse(&library.text, &Limits::default())
        .ok()
        .and_then(|root| {
            root.child("drawing")
                .and_then(|d| d.child("library"))
                .cloned()
        })
        .unwrap_or_default();
    library_element
        .attrs
        .push(("name".to_owned(), LIBRARY.to_owned()));

    let mut ignored = Vec::new();
    let package_of = package_names(board, &mut ignored);
    let symbol_of = symbol_names(schematic, &mut ignored);

    // parts
    let mut parts = Vec::new();
    for part in schematic.parts.values() {
        let Some(deviceset) = symbol_of.get(&part.symbol) else {
            diagnostics.push(codes::diagnostic(
                ITEM_LEFT_OUT,
                Severity::Warning,
                &format!("part {} has no symbol", part.refdes),
            ));
            continue;
        };
        let device = part
            .footprint
            .and_then(|f| package_of.get(&f))
            .cloned()
            .unwrap_or_default();
        parts.push(el(
            "part",
            vec![
                ("name", part.refdes.clone()),
                ("library", LIBRARY.to_owned()),
                ("deviceset", deviceset.clone()),
                ("device", device),
                ("value", part.value.clone()),
            ],
            Vec::new(),
        ));
    }

    // the sheets: the model's, or one that holds everything
    let sheet_ids: Vec<Option<Id>> = if schematic.sheets.is_empty() {
        vec![None]
    } else {
        schematic.sheets.keys().map(|s| Some(*s)).collect()
    };
    if !schematic.sheets.is_empty() {
        diagnostics.push(codes::diagnostic(
            ITEM_SIMPLIFIED,
            Severity::Info,
            "the names and sizes of the sheets are not written",
        ));
    }
    let on_sheet = |placement_sheet: Option<Id>, sheet: Option<Id>| -> bool {
        match sheet {
            None => true,
            Some(s) => placement_sheet == Some(s),
        }
    };

    // the net of each pin and of each point of a wire
    let net_of_pin: BTreeMap<NetPin, Id> = schematic
        .nets
        .iter()
        .flat_map(|(nid, n)| n.pins.iter().map(move |p| (*p, *nid)))
        .collect();
    let positions = schematic.pin_positions();
    type Key = (Id, i64, i64);
    let key =
        |sheet: Id, p: eh3d_phoenix_model::Point| -> Key { (sheet, p.x_nm.get(), p.y_nm.get()) };
    let mut point_net: BTreeMap<Key, Id> = BTreeMap::new();
    for (pin, sheet, at) in &positions {
        if let Some(net) = net_of_pin.get(pin) {
            point_net.insert(key(*sheet, *at), *net);
        }
    }
    let mut wire_net: BTreeMap<Id, Id> = BTreeMap::new();
    loop {
        let mut changed = false;
        for (wid, wire) in &schematic.wires {
            if wire_net.contains_key(wid) {
                continue;
            }
            let found = wire
                .path
                .iter()
                .find_map(|p| point_net.get(&key(wire.sheet, *p)).copied());
            if let Some(net) = found {
                wire_net.insert(*wid, net);
                for p in &wire.path {
                    point_net.insert(key(wire.sheet, *p), net);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (wid, _) in schematic
        .wires
        .iter()
        .filter(|(wid, _)| !wire_net.contains_key(wid))
    {
        diagnostics.push(codes::diagnostic(
            ITEM_LEFT_OUT,
            Severity::Warning,
            &format!("wire {wid} reaches no pin of a net"),
        ));
    }

    let mut sheets = Vec::new();
    for sheet in &sheet_ids {
        let mut instances = Vec::new();
        // the parts of this sheet: those placed on it, or every part when there are no sheets
        let placed_here: BTreeSet<Id> = schematic
            .parts
            .iter()
            .filter(|(_, part)| on_sheet(part.placement.map(|p| p.sheet), *sheet))
            .map(|(pid, _)| *pid)
            .collect();
        for part in schematic.parts.values() {
            if let Some(place) = part.placement {
                if on_sheet(Some(place.sheet), *sheet) {
                    instances.push(el(
                        "instance",
                        vec![
                            ("part", part.refdes.clone()),
                            ("gate", "G$1".to_owned()),
                            ("x", decimal(place.at.x_nm.get())),
                            ("y", decimal(place.at.y_nm.get())),
                            ("rot", rot_text(place.rotation.get())),
                        ],
                        Vec::new(),
                    ));
                }
            }
        }
        let mut nets = Vec::new();
        for (nid, net) in &schematic.nets {
            let mut children = Vec::new();
            for pin in &net.pins {
                let Some(part) = schematic.parts.get(&pin.part) else {
                    continue;
                };
                if !placed_here.contains(&pin.part) {
                    continue;
                }
                let Some(number) = schematic.pin_number(*pin) else {
                    continue;
                };
                children.push(el(
                    "pinref",
                    vec![
                        ("part", part.refdes.clone()),
                        ("gate", "G$1".to_owned()),
                        ("pin", number.to_owned()),
                    ],
                    Vec::new(),
                ));
            }
            for (wid, wire) in &schematic.wires {
                if wire_net.get(wid) != Some(nid) || !on_sheet(Some(wire.sheet), *sheet) {
                    continue;
                }
                for w in wire.path.windows(2) {
                    children.push(el(
                        "wire",
                        vec![
                            ("x1", decimal(w[0].x_nm.get())),
                            ("y1", decimal(w[0].y_nm.get())),
                            ("x2", decimal(w[1].x_nm.get())),
                            ("y2", decimal(w[1].y_nm.get())),
                            ("width", "0.1524".to_owned()),
                            ("layer", "91".to_owned()),
                        ],
                        Vec::new(),
                    ));
                }
            }
            if children.is_empty() {
                continue;
            }
            nets.push(el(
                "net",
                vec![("name", net.name.clone()), ("class", "0".to_owned())],
                vec![el("segment", Vec::new(), children)],
            ));
        }
        sheets.push(el(
            "sheet",
            Vec::new(),
            vec![
                el("plain", Vec::new(), Vec::new()),
                el("instances", Vec::new(), instances),
                el("busses", Vec::new(), Vec::new()),
                el("nets", Vec::new(), nets),
            ],
        ));
    }

    let (numbers, _) = assign(board);
    let mut used: BTreeSet<u32> = numbers.values().copied().collect();
    used.extend([91, 93, 94, 95, 96]);
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
    let schematic_element = el(
        "schematic",
        Vec::new(),
        vec![
            el("libraries", Vec::new(), vec![library_element]),
            el("attributes", Vec::new(), Vec::new()),
            el("variantdefs", Vec::new(), Vec::new()),
            el(
                "classes",
                Vec::new(),
                vec![el(
                    "class",
                    vec![
                        ("number", "0".to_owned()),
                        ("name", "default".to_owned()),
                        ("width", "0".to_owned()),
                        ("drill", "0".to_owned()),
                    ],
                    Vec::new(),
                )],
            ),
            el("parts", Vec::new(), parts),
            el("sheets", Vec::new(), sheets),
        ],
    );
    let root = el(
        "eagle",
        vec![("version", VERSION.to_owned())],
        vec![el(
            "drawing",
            Vec::new(),
            vec![
                el("settings", Vec::new(), Vec::new()),
                el("layers", Vec::new(), layers),
                schematic_element,
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
    use eh3d_phoenix_eagle::{read_bytes, DocumentKind};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
    use eh3d_phoenix_schematic::{Part, Pin, Placement, SchematicNet, Sheet, Symbol, Wire};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// Two resistors of one footprint on one sheet, joined by a wire through a middle point.
    fn fixture() -> (Board, Schematic) {
        let (top, fp) = (id(1), id(2));
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
        for (pid, number) in [(id(3), "1"), (id(4), "2")] {
            pads.insert(
                pid,
                Pad {
                    number: number.into(),
                    shape: PadShape::Circle {
                        diameter: Nm::new(800_000).unwrap(),
                    },
                    at: pt(0, 0),
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
        let (sym, pin1, pin2, sheet) = (id(10), id(11), id(12), id(13));
        let mut s = Schematic::default();
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
                at: Some(pt(2_540_000, 0)),
            },
        );
        s.symbols.insert(
            sym,
            Symbol {
                name: "R".into(),
                pins,
            },
        );
        s.sheets.insert(
            sheet,
            Sheet {
                name: "S1".into(),
                width: Nm::new(297_000_000).unwrap(),
                height: Nm::new(210_000_000).unwrap(),
            },
        );
        for (pid, refdes, x) in [(id(20), "R1", 20_000_000), (id(21), "R2", 40_000_000)] {
            s.parts.insert(
                pid,
                Part {
                    refdes: refdes.into(),
                    value: "10k".into(),
                    symbol: sym,
                    footprint: Some(fp),
                    placement: Some(Placement {
                        sheet,
                        at: pt(x, 20_000_000),
                        rotation: Udeg::ZERO,
                    }),
                },
            );
        }
        let mut net_pins = vec![
            NetPin {
                part: id(20),
                pin: pin2,
            },
            NetPin {
                part: id(21),
                pin: pin1,
            },
        ];
        net_pins.sort();
        s.nets.insert(
            id(30),
            SchematicNet {
                name: "N1".into(),
                pins: net_pins,
            },
        );
        s.wires.insert(
            id(31),
            Wire {
                sheet,
                path: vec![
                    pt(22_540_000, 20_000_000),
                    pt(30_000_000, 20_000_000),
                    pt(37_460_000, 20_000_000),
                ],
            },
        );
        (board, s)
    }

    #[test]
    fn the_schematic_reads_back_with_its_parts_sheets_and_nets() {
        let (board, s) = fixture();
        let written = write_schematic(&board, &s);
        let reading = read_bytes(written.text.as_bytes(), &Limits::default());
        let summary = reading.summary.expect("an EAGLE document");
        assert_eq!(summary.kind, DocumentKind::Schematic);
        assert_eq!(summary.parts, 2);
        assert_eq!(summary.sheets, 1);
        assert_eq!(summary.nets, 1);
        assert_eq!(summary.routed_items, 1);
        assert_eq!(summary.embedded_libraries, 1);
        assert!(reading
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error));
        // the wire is two pieces in the net, and both pins are referenced
        assert_eq!(written.text.matches("<wire ").count(), 2);
        assert_eq!(written.text.matches("<pinref ").count(), 2);
        assert_eq!(written.text.matches("<instance ").count(), 2);
    }

    #[test]
    fn the_same_schematic_gives_the_same_text() {
        let (board, s) = fixture();
        assert_eq!(
            write_schematic(&board, &s).text,
            write_schematic(&board, &s).text
        );
    }

    #[test]
    fn a_wire_that_reaches_no_net_is_left_out_with_a_warning() {
        let (board, mut s) = fixture();
        let sheet = id(13);
        s.wires.insert(
            id(32),
            Wire {
                sheet,
                path: vec![pt(1_000_000, 1_000_000), pt(2_000_000, 1_000_000)],
            },
        );
        let written = write_schematic(&board, &s);
        assert_eq!(written.text.matches("<wire ").count(), 2);
        assert!(written.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some(crate::ITEM_LEFT_OUT) && d.severity == Severity::Warning
        }));
    }

    #[test]
    fn a_schematic_without_sheets_is_written_as_one_sheet() {
        let (board, mut s) = fixture();
        s.sheets.clear();
        s.wires.clear();
        for part in s.parts.values_mut() {
            part.placement = None;
        }
        let written = write_schematic(&board, &s);
        let summary = read_bytes(written.text.as_bytes(), &Limits::default())
            .summary
            .unwrap();
        assert_eq!((summary.sheets, summary.parts), (1, 2));
        assert_eq!(written.text.matches("<pinref ").count(), 2);
        assert_eq!(written.text.matches("<instance ").count(), 0);
    }
}
