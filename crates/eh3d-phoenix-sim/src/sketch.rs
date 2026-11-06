// SPDX-License-Identifier: GPL-3.0-or-later
//! A small schematic written as a few lines of text, for the reference cases and the tests: no
//! drawing, only the symbols, the parts and the nets.
//!
//! ```text
//! name divider            the name of the schematic
//! symbol R 1 2            a symbol and the numbers of its pins
//! part V1 R 10            a part: the designator, the symbol and the value (the rest of the line)
//! part R1 R 1k
//! net IN V1.1 R1.1        a net: its name (a dash for no name) and its pins as DESIGNATOR.PIN
//! ```
//!
//! What follows a `#` is ignored. The identities are made in the order of the lines, so the same
//! text is always the same schematic.

use std::collections::BTreeMap;

use eh3d_phoenix_schematic::{Id, NetPin, Part, Pin, Schematic, SchematicNet, Symbol};

/// The time stamp of the identities that the sketch makes.
const STAMP_MS: u64 = 1_700_000_000_000;

struct Maker {
    next: u16,
}

impl Maker {
    fn fresh(&mut self) -> Result<Id, String> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| "the sketch has too many objects".to_owned())?;
        Id::from_parts(STAMP_MS, self.next, u64::from(self.next)).map_err(|e| format!("{e:?}"))
    }
}

/// A symbol of the sketch: its identity and the identity of each pin by number.
struct SketchSymbol {
    id: Id,
    pins: BTreeMap<String, Id>,
}

/// Reads the text of a sketch as a schematic.
pub fn schematic_from_sketch(text: &str) -> Result<Schematic, String> {
    let mut maker = Maker { next: 0 };
    let mut schematic = Schematic::default();
    let mut symbols: BTreeMap<String, SketchSymbol> = BTreeMap::new();
    let mut parts: BTreeMap<String, (Id, String)> = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let fail = |why: &str| format!("line {number}: {why}");
        match words[0] {
            "name" => schematic.name = words[1..].join(" "),
            "symbol" => {
                let [_, name, pin_numbers @ ..] = words.as_slice() else {
                    return Err(fail("a symbol needs a name and its pins"));
                };
                if pin_numbers.is_empty() {
                    return Err(fail("a symbol needs at least one pin"));
                }
                if symbols.contains_key(*name) {
                    return Err(fail(&format!("the symbol {name} appears twice")));
                }
                let id = maker.fresh()?;
                let mut pins = BTreeMap::new();
                let mut by_number = BTreeMap::new();
                for pin_number in pin_numbers {
                    let pin_id = maker.fresh()?;
                    if by_number.insert((*pin_number).to_owned(), pin_id).is_some() {
                        return Err(fail(&format!("the pin {pin_number} appears twice")));
                    }
                    pins.insert(
                        pin_id,
                        Pin {
                            number: (*pin_number).to_owned(),
                            at: None,
                        },
                    );
                }
                schematic.symbols.insert(
                    id,
                    Symbol {
                        name: (*name).to_owned(),
                        pins,
                    },
                );
                symbols.insert(
                    (*name).to_owned(),
                    SketchSymbol {
                        id,
                        pins: by_number,
                    },
                );
            }
            "part" => {
                let [_, refdes, symbol, value @ ..] = words.as_slice() else {
                    return Err(fail("a part needs a designator and a symbol"));
                };
                let Some(found) = symbols.get(*symbol) else {
                    return Err(fail(&format!("the symbol {symbol} is not defined")));
                };
                if parts.contains_key(*refdes) {
                    return Err(fail(&format!("the designator {refdes} appears twice")));
                }
                let id = maker.fresh()?;
                schematic.parts.insert(
                    id,
                    Part {
                        refdes: (*refdes).to_owned(),
                        value: value.join(" "),
                        symbol: found.id,
                        footprint: None,
                        placement: None,
                    },
                );
                parts.insert((*refdes).to_owned(), (id, (*symbol).to_owned()));
            }
            "net" => {
                let [_, name, pin_refs @ ..] = words.as_slice() else {
                    return Err(fail("a net needs a name and its pins"));
                };
                if pin_refs.is_empty() {
                    return Err(fail("a net needs at least one pin"));
                }
                let mut pins: Vec<NetPin> = Vec::new();
                for reference in pin_refs {
                    let Some((refdes, pin_number)) = reference.split_once('.') else {
                        return Err(fail(&format!("{reference} is not DESIGNATOR.PIN")));
                    };
                    let Some((part_id, symbol_name)) = parts.get(refdes) else {
                        return Err(fail(&format!("the part {refdes} is not defined")));
                    };
                    let pin = symbols
                        .get(symbol_name)
                        .and_then(|s| s.pins.get(pin_number))
                        .ok_or_else(|| {
                            fail(&format!("the part {refdes} has no pin {pin_number}"))
                        })?;
                    pins.push(NetPin {
                        part: *part_id,
                        pin: *pin,
                    });
                }
                pins.sort();
                let id = maker.fresh()?;
                schematic.nets.insert(
                    id,
                    SchematicNet {
                        name: if *name == "-" {
                            String::new()
                        } else {
                            (*name).to_owned()
                        },
                        pins,
                    },
                );
            }
            other => return Err(fail(&format!("{other} is not name, symbol, part or net"))),
        }
    }
    if !schematic.is_consistent() {
        return Err("the sketch is not a consistent schematic (a pin is on two nets)".to_owned());
    }
    Ok(schematic)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIVIDER: &str = "\
# a source across two resistors
name divider
symbol R 1 2
part V1 R 10
part R1 R 1k
part R2 R 3k
net IN V1.1 R1.1
net OUT R1.2 R2.1   # the middle
net GND V1.2 R2.2
";

    #[test]
    fn a_sketch_is_a_schematic_and_the_same_text_is_the_same_schematic() {
        let a = schematic_from_sketch(DIVIDER).unwrap();
        let b = schematic_from_sketch(DIVIDER).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.name, "divider");
        assert_eq!(a.symbols.len(), 1);
        assert_eq!(a.parts.len(), 3);
        assert_eq!(a.nets.len(), 3);
        assert!(a.is_consistent());
        let values: Vec<&str> = a.parts.values().map(|p| p.value.as_str()).collect();
        assert_eq!(values, vec!["10", "1k", "3k"]);
        let names: Vec<&str> = a.nets.values().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["IN", "OUT", "GND"]);
    }

    #[test]
    fn a_value_keeps_its_words_and_a_dash_is_a_net_with_no_name() {
        let s = schematic_from_sketch(
            "symbol S 1 2\npart V1 S DC 0 AC 1\npart R1 S\nnet - V1.1 R1.1\n",
        )
        .unwrap();
        let values: Vec<&str> = s.parts.values().map(|p| p.value.as_str()).collect();
        assert_eq!(values, vec!["DC 0 AC 1", ""]);
        assert_eq!(s.nets.values().next().unwrap().name, "");
    }

    #[test]
    fn a_line_that_is_wrong_is_refused_with_its_number() {
        let cases = [
            ("wire a b\n", "line 1"),
            ("symbol\n", "line 1"),
            ("symbol R\n", "line 1"),
            ("symbol R 1 1\n", "line 1"),
            ("symbol R 1 2\nsymbol R 1 2\n", "line 2"),
            ("part R1 R 1k\n", "line 1"),
            ("symbol R 1 2\npart R1 R 1k\npart R1 R 2k\n", "line 3"),
            ("symbol R 1 2\npart R1 R 1k\nnet A R1\n", "line 3"),
            ("symbol R 1 2\npart R1 R 1k\nnet A R2.1\n", "line 3"),
            ("symbol R 1 2\npart R1 R 1k\nnet A R1.3\n", "line 3"),
            ("symbol R 1 2\npart R1 R 1k\nnet A\n", "line 3"),
        ];
        for (text, line) in cases {
            let error = schematic_from_sketch(text).unwrap_err();
            assert!(error.contains(line), "{text:?}: {error}");
        }
        // a pin on two nets is not a schematic
        let twice = "symbol R 1 2\npart R1 R 1k\nnet A R1.1\nnet B R1.1\n";
        assert!(schematic_from_sketch(twice)
            .unwrap_err()
            .contains("two nets"));
    }
}
