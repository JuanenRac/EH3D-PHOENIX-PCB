// SPDX-License-Identifier: GPL-3.0-or-later
//! Symbols: the definitions of a `.kicad_sym` library or of the `lib_symbols` of a schematic, read
//! into the symbols of the schematic model.
//!
//! What the model holds of a symbol is its name and its pins (number and position). The drawing
//! of the symbol (rectangles, lines, texts), the names of the pins, their electrical type and the
//! fields other than the reference and the value are not kept and are counted as dropped. A symbol
//! that extends another one (KiCad 7 and later) gets the pins of the symbol it extends.

use std::collections::BTreeMap;

use eh3d_phoenix_board::Id;
use eh3d_phoenix_hub::QualityReport;
use eh3d_phoenix_model::Point;
use eh3d_phoenix_schematic::model::{Pin, Symbol};

use crate::conv::{at_args, check_root, decode, to_model_up, Ctx, P};
use crate::error::KicadError;
use crate::settings::KicadSettings;
use crate::sexpr::{parse, Node};
use eh3d_diagnostic::Diagnostic;

/// A pin of a definition, in the coordinates of the symbol (y up).
#[derive(Debug, Clone)]
pub(crate) struct PinDef {
    pub number: String,
    pub name: String,
    pub electrical: String,
    pub at: P,
    pub unit: u32,
    pub style: u32,
    pub hidden: bool,
}

/// A symbol definition.
#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolDef {
    pub name: String,
    pub power: bool,
    pub extends: Option<String>,
    pub pins: Vec<PinDef>,
    pub graphics: usize,
    pub line: u32,
}

const GRAPHICS: [&str; 7] = [
    "rectangle",
    "polyline",
    "circle",
    "arc",
    "text",
    "bezier",
    "text_box",
];

/// The unit and the style of a sub-symbol named like `R_1_1`.
fn unit_and_style(sub_name: &str) -> Option<(u32, u32)> {
    let mut parts = sub_name.rsplitn(3, '_');
    let style = parts.next()?.parse::<u32>().ok()?;
    let unit = parts.next()?.parse::<u32>().ok()?;
    parts.next()?;
    Some((unit, style))
}

fn parse_pin(node: &Node, unit: u32, style: u32, ctx: &mut Ctx) -> Option<PinDef> {
    let Some(at_node) = node.child("at") else {
        ctx.invalid(node, "pin", "it has no position");
        return None;
    };
    let Some((at, _)) = at_args(at_node) else {
        ctx.invalid(at_node, "pin position", "not two numbers");
        return None;
    };
    let hidden = node.has_word("hide")
        || node
            .child("hide")
            .is_some_and(|h| h.arg_text(0) != Some("no"));
    Some(PinDef {
        number: node
            .child("number")
            .and_then(|n| n.arg_text(0))
            .unwrap_or("")
            .to_owned(),
        name: node
            .child("name")
            .and_then(|n| n.arg_text(0))
            .unwrap_or("")
            .to_owned(),
        electrical: node.arg_text(0).unwrap_or("").to_owned(),
        at,
        unit,
        style,
        hidden,
    })
}

/// Reads one `(symbol "name" ...)` definition.
pub(crate) fn parse_symbol(node: &Node, ctx: &mut Ctx) -> Option<SymbolDef> {
    let Some(name) = node.arg_text(0) else {
        ctx.invalid(node, "symbol", "it has no name");
        return None;
    };
    let mut def = SymbolDef {
        name: name.to_owned(),
        line: node.line,
        ..SymbolDef::default()
    };
    let mut pins_seen = 0usize;
    for child in node.args() {
        match child.head() {
            Some("symbol") => {
                let sub_name = child.arg_text(0).unwrap_or("");
                let (unit, style) = unit_and_style(sub_name).unwrap_or_else(|| {
                    ctx.invalid(child, "sub-symbol", "its name does not end in _unit_style");
                    (0, 0)
                });
                for item in child.args() {
                    match item.head() {
                        Some("pin") => {
                            pins_seen += 1;
                            if pins_seen > ctx.settings.max_items {
                                break;
                            }
                            if let Some(pin) = parse_pin(item, unit, style, ctx) {
                                def.pins.push(pin);
                            }
                        }
                        Some(g) if GRAPHICS.contains(&g) => def.graphics += 1,
                        _ => {}
                    }
                }
            }
            Some("pin") => {
                pins_seen += 1;
                if pins_seen > ctx.settings.max_items {
                    break;
                }
                if let Some(pin) = parse_pin(child, 0, 0, ctx) {
                    def.pins.push(pin);
                }
            }
            Some("power") => def.power = true,
            Some("extends") => def.extends = child.arg_text(0).map(str::to_owned),
            Some(g) if GRAPHICS.contains(&g) => def.graphics += 1,
            _ => {}
        }
    }
    Some(def)
}

/// Gives every definition that extends another the pins of the one it extends.
pub(crate) fn resolve_extends(defs: &mut [SymbolDef], ctx: &mut Ctx) {
    let snapshot: BTreeMap<String, (Vec<PinDef>, bool)> = defs
        .iter()
        .map(|d| (d.name.clone(), (d.pins.clone(), d.power)))
        .collect();
    for def in defs.iter_mut() {
        let Some(parent) = def.extends.clone() else {
            continue;
        };
        match snapshot.get(&parent) {
            Some((pins, power)) => {
                if def.pins.is_empty() {
                    def.pins = pins.clone();
                }
                def.power = def.power || *power;
            }
            None => ctx.push(KicadError::MissingReference {
                what: format!(
                    "symbol `{}` extends `{parent}`, which the file does not hold",
                    def.name
                ),
                line: def.line as usize,
            }),
        }
    }
}

/// A pin of a built symbol: where it is (in the orientation of the file, y up, mirror applied) and
/// the net a hidden power pin joins by its name.
#[derive(Debug, Clone)]
pub(crate) struct PinInfo {
    pub id: Id,
    pub at: P,
    pub number: String,
    pub power_net: Option<String>,
}

/// Builds the model symbol of a definition. With `unit` only the pins common to all units and
/// those of that unit are used (the first drawing style only); without it every unit's pins are.
/// `mirror_x` turns y over and `mirror_y` turns x over.
pub(crate) fn build_symbol(
    def: &SymbolDef,
    name: &str,
    unit: Option<u32>,
    mirror: (bool, bool),
    ctx: &mut Ctx,
) -> (Symbol, Vec<PinInfo>) {
    let mut pins = BTreeMap::new();
    let mut infos = Vec::new();
    for p in &def.pins {
        let wanted_unit = match unit {
            None => true,
            Some(u) => p.unit == 0 || p.unit == u,
        };
        if p.style > 1 || !wanted_unit {
            continue;
        }
        let at = (
            if mirror.1 { -p.at.0 } else { p.at.0 },
            if mirror.0 { -p.at.1 } else { p.at.1 },
        );
        let id = ctx.id();
        let point: Option<Point> = to_model_up(at);
        if point.is_none() {
            ctx.push(KicadError::InvalidValue {
                line: def.line as usize,
                column: 0,
                what: format!("pin {} of `{}`", p.number, def.name),
                reason: "the position is outside the range of the model".to_owned(),
            });
        }
        pins.insert(
            id,
            Pin {
                number: p.number.clone(),
                at: point,
            },
        );
        infos.push(PinInfo {
            id,
            at,
            number: p.number.clone(),
            power_net: if p.hidden && p.electrical == "power_in" {
                Some(p.name.clone())
            } else {
                None
            },
        });
    }
    (
        Symbol {
            name: name.to_owned(),
            pins,
        },
        infos,
    )
}

/// A symbol library read into the model.
#[derive(Debug, Clone)]
pub struct ReadSymbols {
    /// The symbols, one per definition, with the pins of every unit.
    pub symbols: BTreeMap<Id, Symbol>,
    /// The version the file declares.
    pub version: String,
    /// The program that wrote the file.
    pub generator: String,
    /// What was kept, changed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Reads a `.kicad_sym` library. Ids for the new objects come from `new_id`.
pub fn read_symbol_library(
    bytes: &[u8],
    settings: &KicadSettings,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<ReadSymbols, Diagnostic> {
    let fail = |e: KicadError| e.diagnostic();
    let (text, encoding) = decode(bytes, settings).map_err(fail)?;
    let root = parse(&text, settings).map_err(fail)?;
    let mut ctx = Ctx::new(settings, new_id);
    if let Some(note) = encoding {
        ctx.push(note);
    }
    let (version, generator) = check_root(
        &root,
        &["kicad_symbol_lib"],
        "a symbol library (kicad_symbol_lib)",
        &mut ctx,
    )
    .map_err(fail)?;
    let mut defs: Vec<SymbolDef> = Vec::new();
    for child in root.args() {
        match child.head() {
            Some("symbol") => {
                if defs.len() >= settings.max_items {
                    ctx.ignore("symbol (over max_items)");
                    continue;
                }
                if let Some(def) = parse_symbol(child, &mut ctx) {
                    defs.push(def);
                }
            }
            Some("version") | Some("generator") | Some("generator_version") => {}
            Some(other) => ctx.ignore(other),
            None => {}
        }
    }
    resolve_extends(&mut defs, &mut ctx);
    let mut symbols = BTreeMap::new();
    let mut pin_count = 0usize;
    for def in &defs {
        let (symbol, _) = build_symbol(def, &def.name, None, (false, false), &mut ctx);
        pin_count += symbol.pins.len();
        ctx.lose("symbol graphics", def.graphics);
        let id = ctx.id();
        symbols.insert(id, symbol);
    }
    ctx.keep("symbols", symbols.len());
    ctx.keep("pins", pin_count);
    let (diagnostics, quality) = ctx.finish();
    Ok(ReadSymbols {
        symbols,
        version,
        generator,
        quality,
        diagnostics,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A KiCad 6 library with a resistor of two pins and an op amp of two units.
    pub(crate) const LIB_V6: &str = r#"(kicad_symbol_lib (version 20211014) (generator kicad_symbol_editor)
  (symbol "R" (pin_numbers hide) (pin_names (offset 0)) (in_bom yes) (on_board yes)
    (property "Reference" "R" (id 0) (at 2.032 0 90) (effects (font (size 1.27 1.27))))
    (property "Value" "R" (id 1) (at 0 0 90) (effects (font (size 1.27 1.27))))
    (symbol "R_0_1"
      (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
    (symbol "R_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
  (symbol "OPA2" (in_bom yes) (on_board yes)
    (property "Reference" "U" (id 0) (at 0 0 0))
    (symbol "OPA2_1_1"
      (polyline (pts (xy -2 2) (xy 2 0) (xy -2 -2) (xy -2 2)) (stroke (width 0.2) (type default)) (fill (type none)))
      (pin input line (at -5.08 1.27 0) (length 2.54) (name "+" (effects (font (size 1 1)))) (number "3" (effects (font (size 1 1)))))
      (pin output line (at 5.08 0 180) (length 2.54) (name "OUT" (effects (font (size 1 1)))) (number "1" (effects (font (size 1 1))))))
    (symbol "OPA2_2_1"
      (pin input line (at -5.08 1.27 0) (length 2.54) (name "+" (effects (font (size 1 1)))) (number "5" (effects (font (size 1 1)))))
      (pin output line (at 5.08 0 180) (length 2.54) (name "OUT" (effects (font (size 1 1)))) (number "7" (effects (font (size 1 1))))))
    (symbol "OPA2_0_1"
      (pin power_in line (at 0 5 270) hide (length 2) (name "V+") (number "8")))
    (symbol "OPA2_1_2"
      (pin input line (at -5.08 1.27 0) (length 2.54) (name "A" (effects (font (size 1 1)))) (number "30" (effects (font (size 1 1)))))))
  (some_future_thing 1))"#;

    fn read(text: &str) -> ReadSymbols {
        let mut ids = crate::sequential_ids().unwrap();
        read_symbol_library(text.as_bytes(), &KicadSettings::default(), &mut ids).unwrap()
    }

    #[test]
    fn a_kicad_6_library_is_read_into_symbols_with_pins() {
        let lib = read(LIB_V6);
        assert_eq!(lib.version, "20211014");
        assert_eq!(lib.generator, "kicad_symbol_editor");
        assert_eq!(lib.symbols.len(), 2);
        let r = lib.symbols.values().find(|s| s.name == "R").unwrap();
        assert_eq!(r.pins.len(), 2);
        let mut positions: Vec<(String, i64, i64)> = r
            .pins
            .values()
            .map(|p| {
                let at = p.at.unwrap();
                (p.number.clone(), at.x_nm.get(), at.y_nm.get())
            })
            .collect();
        positions.sort();
        // the library's y axis already points up, so it is kept
        assert_eq!(
            positions,
            vec![
                ("1".to_owned(), 0, 3_810_000),
                ("2".to_owned(), 0, -3_810_000)
            ]
        );
    }

    #[test]
    fn units_are_merged_and_the_second_drawing_style_is_left_out() {
        let lib = read(LIB_V6);
        let op = lib.symbols.values().find(|s| s.name == "OPA2").unwrap();
        let mut numbers: Vec<&str> = op.pins.values().map(|p| p.number.as_str()).collect();
        numbers.sort_unstable();
        // units 1 and 2 and the common power pin; pin 30 belongs to style 2
        assert_eq!(numbers, vec!["1", "3", "5", "7", "8"]);
    }

    #[test]
    fn the_quality_report_says_what_was_dropped() {
        let lib = read(LIB_V6);
        let kept = |name: &str| {
            lib.quality
                .kept
                .iter()
                .find(|i| i.category == name)
                .map(|i| i.count)
        };
        assert_eq!(kept("symbols"), Some(2));
        assert_eq!(kept("pins"), Some(7));
        let dropped = |name: &str| {
            lib.quality
                .dropped
                .iter()
                .find(|i| i.category == name)
                .map(|i| i.count)
        };
        assert_eq!(dropped("symbol graphics"), Some(2));
        assert_eq!(dropped("`some_future_thing` elements"), Some(1));
        assert!(lib
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-006")));
    }

    #[test]
    fn kicad_7_and_8_forms_are_read_the_same_way() {
        let v8 = r#"(kicad_symbol_lib (version 20231120) (generator "kicad_symbol_editor") (generator_version "8.0")
  (symbol "Base" (exclude_from_sim no) (in_bom yes) (on_board yes)
    (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (symbol "Base_1_1"
      (pin power_in line (at 0 5.08 270) (length 2.54) (hide yes) (name "VCC" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
      (pin passive line (at 0 -5.08 90) (length 2.54) (name "X" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
  (symbol "Derived" (extends "Base")
    (property "Reference" "U" (at 0 0 0)))
  (symbol "Orphan" (extends "Nothing")))"#;
        let lib = read(v8);
        assert_eq!(lib.symbols.len(), 3);
        let derived = lib.symbols.values().find(|s| s.name == "Derived").unwrap();
        assert_eq!(derived.pins.len(), 2);
        let orphan = lib.symbols.values().find(|s| s.name == "Orphan").unwrap();
        assert!(orphan.pins.is_empty());
        assert!(lib
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-008")));
        assert!(lib
            .diagnostics
            .iter()
            .all(|d| d.code.as_deref() != Some("PHOENIX-KICAD-009")));
    }

    #[test]
    fn a_version_outside_the_tested_range_is_a_warning_and_the_file_is_still_read() {
        let old = "(kicad_symbol_lib (version 20171130) (symbol \"A\"))";
        let lib = read(old);
        assert_eq!(lib.symbols.len(), 1);
        assert!(lib
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-009")));
        let future = "(kicad_symbol_lib (version 29990101) (symbol \"A\"))";
        assert!(read(future)
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-009")));
    }

    #[test]
    fn a_pin_without_a_usable_position_is_reported_and_skipped() {
        let text = "(kicad_symbol_lib (version 20211014) (symbol \"A\" (symbol \"A_1_1\" (pin passive line (length 1) (name \"x\") (number \"1\")) (pin passive line (at x 1 0) (number \"2\")) (pin passive line (at 1 2 0) (number \"3\")))))";
        let lib = read(text);
        let a = lib.symbols.values().next().unwrap();
        assert_eq!(a.pins.len(), 1);
        let bad = lib
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("PHOENIX-KICAD-005"))
            .count();
        assert_eq!(bad, 2);
    }

    #[test]
    fn files_of_another_kind_and_damaged_files_are_refused_with_a_code() {
        let mut ids = crate::sequential_ids().unwrap();
        let s = KicadSettings::default();
        let e = read_symbol_library(b"(kicad_pcb (version 1))", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-003"));
        let e = read_symbol_library(b"(kicad_symbol_lib (symbol", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-004"));
        let e = read_symbol_library(b"(kicad_symbol_lib))", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-001"));
        let e = read_symbol_library(b"", &s, &mut ids).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-001"));
    }

    #[test]
    fn mirroring_turns_the_pin_positions() {
        let settings = KicadSettings::default();
        let mut ids = crate::sequential_ids().unwrap();
        let mut ctx = Ctx::new(&settings, &mut ids);
        let def = SymbolDef {
            name: "M".into(),
            pins: vec![PinDef {
                number: "1".into(),
                name: "a".into(),
                electrical: "passive".into(),
                at: (2_000_000, 3_000_000),
                unit: 1,
                style: 1,
                hidden: false,
            }],
            ..SymbolDef::default()
        };
        let (_, plain) = build_symbol(&def, "M", Some(1), (false, false), &mut ctx);
        let (_, mx) = build_symbol(&def, "M", Some(1), (true, false), &mut ctx);
        let (_, my) = build_symbol(&def, "M", Some(1), (false, true), &mut ctx);
        let (_, other_unit) = build_symbol(&def, "M", Some(2), (false, false), &mut ctx);
        assert_eq!(plain[0].at, (2_000_000, 3_000_000));
        assert_eq!(mx[0].at, (2_000_000, -3_000_000));
        assert_eq!(my[0].at, (-2_000_000, 3_000_000));
        assert!(other_unit.is_empty());
    }

    #[test]
    fn hidden_power_input_pins_name_the_net_they_join() {
        let settings = KicadSettings::default();
        let mut ids = crate::sequential_ids().unwrap();
        let root = parse(LIB_V6, &settings).unwrap();
        let mut ctx = Ctx::new(&settings, &mut ids);
        let op = root
            .children("symbol")
            .find(|n| n.arg_text(0) == Some("OPA2"))
            .unwrap();
        let def = parse_symbol(op, &mut ctx).unwrap();
        let (_, infos) = build_symbol(&def, "OPA2", Some(1), (false, false), &mut ctx);
        let power: Vec<&PinInfo> = infos.iter().filter(|i| i.power_net.is_some()).collect();
        assert_eq!(power.len(), 1);
        assert_eq!(power[0].power_net.as_deref(), Some("V+"));
        assert_eq!(power[0].number, "8");
    }
}
