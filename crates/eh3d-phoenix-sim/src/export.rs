// SPDX-License-Identifier: GPL-3.0-or-later
//! The netlist of a schematic. The pins of each part are joined into nodes by the nets of the
//! schematic, the value of the part is read as the value of the element, and the nets whose names
//! are in the ground list of the settings (`0`, `gnd`, `agnd`, ...) become node 0. What cannot
//! be translated is listed in the report with a code and the reason; nothing is dropped
//! silently.
//!
//! The kind of an element comes from the reference designator, which must be one letter followed
//! by digits: `R C L` (two pins, the value is the number), `V I` (two pins, the first is the
//! positive one; the value is what follows the nodes in a netlist, `5`, `DC 5` or
//! `SIN(0 1 1k)`), `D` (two pins, the anode is the pin `A` or else the first; the value is the
//! name of the model), `E G` (four pins: output positive and negative, control positive and
//! negative; the value is the gain), `F H` (two pins; the value is the controlling source and
//! the gain, `V1 2`) and `K` (no pins; the value is `L1 L2 k`). Pins are taken in the order of
//! their numbers.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{NetPin, Schematic};

use crate::codes::{
    SimError, DUPLICATE_NAME, INPUT_UNREADABLE, NO_GROUND, PART_SKIPPED, PIN_UNCONNECTED,
};
use crate::netlist::{DiodeModel, Netlist};
use crate::parse::parse_element_line;
use crate::settings::SimSettings;

/// What the translation did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportReport {
    /// The reference designators that became elements.
    pub translated: Vec<String>,
    /// What could not be translated, and the pins that are on no net.
    pub issues: Vec<SimError>,
    /// The names of the nets that were taken as ground.
    pub grounds: Vec<String>,
    /// The number of nodes of the netlist, ground included.
    pub nodes: usize,
}

impl ExportReport {
    /// True when every part became an element and every pin is on a net.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// The report as text, with the messages in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!(
            "netlist: {} elements translated, {} nodes, {} issues\n",
            self.translated.len(),
            self.nodes,
            self.issues.len()
        );
        if !self.grounds.is_empty() {
            out.push_str(&format!("ground nets: {}\n", self.grounds.join(", ")));
        }
        for issue in &self.issues {
            out.push_str(&issue.message(lang));
            out.push('\n');
        }
        out
    }
}

/// A node or model name: letters and digits in lower case, anything else as an underscore.
fn clean(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// The sort key of a pin number: whole numbers first, in numeric order, then the others.
fn pin_key(number: &str) -> (u8, u64, String) {
    match number.trim().parse::<u64>() {
        Ok(v) => (0, v, String::new()),
        Err(_) => (1, 0, number.trim().to_lowercase()),
    }
}

fn skipped(refdes: &str, why: &str) -> SimError {
    SimError::new(PART_SKIPPED, format!("{refdes}: {why}"))
}

/// Reads a schematic from the canonical JSON of a project (`design/schematic.json`) and
/// translates it.
pub fn netlist_from_schematic_json(
    bytes: &[u8],
    settings: &SimSettings,
) -> Result<(Netlist, ExportReport), SimError> {
    let schematic = Schematic::from_bytes(bytes).map_err(|e| {
        SimError::new(
            INPUT_UNREADABLE,
            format!("the file is not a schematic of the project format: {e}"),
        )
    })?;
    Ok(netlist_from_schematic(&schematic, settings))
}

/// Translates a schematic into a netlist.
pub fn netlist_from_schematic(
    schematic: &Schematic,
    settings: &SimSettings,
) -> (Netlist, ExportReport) {
    let mut netlist = Netlist {
        title: schematic.name.clone(),
        ..Netlist::default()
    };
    let mut report = ExportReport::default();

    // the node of each pin that is on a net
    let mut pin_nodes: BTreeMap<NetPin, String> = BTreeMap::new();
    let mut grounds: BTreeSet<String> = BTreeSet::new();
    for (index, net) in schematic.nets.values().enumerate() {
        let node = if settings.is_schematic_ground(&net.name) {
            grounds.insert(net.name.trim().to_string());
            "0".to_string()
        } else {
            let cleaned = clean(&net.name);
            if cleaned.is_empty() {
                format!("net_{}", index + 1)
            } else {
                cleaned
            }
        };
        for pin in &net.pins {
            pin_nodes.insert(*pin, node.clone());
        }
    }
    report.grounds = grounds.into_iter().collect();

    let mut parts: Vec<_> = schematic.parts.iter().collect();
    parts.sort_by_key(|(_, part)| part.refdes.to_lowercase());

    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut used_nodes: BTreeSet<String> = BTreeSet::new();
    for (part_id, part) in parts {
        let refdes = part.refdes.trim();
        let Some(symbol) = schematic.symbols.get(&part.symbol) else {
            report
                .issues
                .push(skipped(refdes, "its symbol does not exist"));
            continue;
        };
        let letter = refdes
            .chars()
            .next()
            .map_or(' ', |c| c.to_ascii_uppercase());
        let well_formed = refdes.chars().nth(1).is_some_and(|c| c.is_ascii_digit());
        if !well_formed || !"RCLVIDEGFHK".contains(letter) {
            report.issues.push(skipped(
                refdes,
                "this kind of part is not simulated (a reference designator is one letter R C L V I D E G F H or K followed by digits)",
            ));
            continue;
        }

        let mut pins: Vec<_> = symbol.pins.iter().collect();
        pins.sort_by_key(|(_, pin)| pin_key(&pin.number));
        let mut nodes: Vec<(String, String)> = Vec::new();
        for (pin_id, pin) in &pins {
            let key = NetPin {
                part: *part_id,
                pin: **pin_id,
            };
            let node = match pin_nodes.get(&key) {
                Some(node) => node.clone(),
                None => {
                    report.issues.push(SimError::new(
                        PIN_UNCONNECTED,
                        format!("{refdes} pin {}", pin.number),
                    ));
                    format!("nc_{}_{}", clean(refdes), clean(&pin.number))
                }
            };
            nodes.push((pin.number.trim().to_lowercase(), node));
        }

        let value = part.value.trim();
        let expected = match letter {
            'R' | 'C' | 'L' | 'V' | 'I' | 'D' | 'F' | 'H' => 2,
            'E' | 'G' => 4,
            _ => 0,
        };
        if letter != 'K' && nodes.len() != expected {
            report.issues.push(skipped(
                refdes,
                &format!(
                    "a part of this kind needs {expected} pins and its symbol has {}",
                    nodes.len()
                ),
            ));
            continue;
        }
        if value.is_empty() && letter != 'D' {
            report.issues.push(skipped(refdes, "the part has no value"));
            continue;
        }

        let mut chosen: Vec<String> = nodes.iter().map(|(_, node)| node.clone()).collect();
        let tail = match letter {
            'D' => {
                // the anode is the pin A when the symbol names its pins A and K
                let anode_first = nodes.iter().position(|(number, _)| number == "a");
                let cathode = nodes.iter().position(|(number, _)| number == "k");
                if let (Some(a), Some(k)) = (anode_first, cathode) {
                    chosen = vec![nodes[a].1.clone(), nodes[k].1.clone()];
                }
                let model = if value.is_empty() {
                    "d".to_string()
                } else {
                    clean(value)
                };
                netlist.models.entry(model.clone()).or_insert(DiodeModel {
                    is: settings.diode_is,
                    n: settings.diode_n,
                });
                model
            }
            _ => value.to_string(),
        };
        let line = if letter == 'K' {
            format!("{refdes} {tail}")
        } else {
            format!("{refdes} {} {tail}", chosen.join(" "))
        };

        if !names.insert(refdes.to_lowercase()) {
            report.issues.push(SimError::new(
                DUPLICATE_NAME,
                format!("the reference designator {refdes} is used by more than one part"),
            ));
            continue;
        }
        match parse_element_line(&line, 0, &netlist.params, settings) {
            Ok(element) => {
                used_nodes.extend(element.nodes.iter().cloned());
                netlist.elements.push(element);
                report.translated.push(refdes.to_string());
            }
            Err(e) => {
                names.remove(&refdes.to_lowercase());
                report.issues.push(SimError { line: None, ..e });
            }
        }
    }

    if !used_nodes.contains("0") {
        report.issues.push(SimError::new(
            NO_GROUND,
            "no translated element is connected to a ground net".to_string(),
        ));
    }
    report.nodes = used_nodes.len();
    (netlist, report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::Circuit;
    use crate::codes::VALUE_INVALID;
    use crate::dc::operating_point;
    use eh3d_container::ObjectId;
    use eh3d_phoenix_schematic::{Id, Part, Pin, SchematicNet, Symbol};

    struct Builder {
        s: Schematic,
        next: u16,
        pins: BTreeMap<(Id, String), Id>,
    }

    impl Builder {
        fn new() -> Builder {
            Builder {
                s: Schematic {
                    name: "test".into(),
                    ..Schematic::default()
                },
                next: 0,
                pins: BTreeMap::new(),
            }
        }

        fn id(&mut self) -> Id {
            self.next += 1;
            ObjectId::from_parts(1_700_000_000_000, self.next, u64::from(self.next)).unwrap()
        }

        fn symbol(&mut self, name: &str, numbers: &[&str]) -> Id {
            let symbol = self.id();
            let mut pins = BTreeMap::new();
            for number in numbers {
                let pin = self.id();
                pins.insert(
                    pin,
                    Pin {
                        number: (*number).to_string(),
                        at: None,
                    },
                );
                self.pins.insert((symbol, (*number).to_string()), pin);
            }
            self.s.symbols.insert(
                symbol,
                Symbol {
                    name: name.into(),
                    pins,
                },
            );
            symbol
        }

        fn part(&mut self, symbol: Id, refdes: &str, value: &str) -> Id {
            let part = self.id();
            self.s.parts.insert(
                part,
                Part {
                    refdes: refdes.into(),
                    value: value.into(),
                    symbol,
                    footprint: None,
                    placement: None,
                },
            );
            part
        }

        fn net(&mut self, name: &str, members: &[(Id, Id, &str)]) {
            let mut pins: Vec<NetPin> = members
                .iter()
                .map(|(part, symbol, number)| NetPin {
                    part: *part,
                    pin: self.pins[&(*symbol, (*number).to_string())],
                })
                .collect();
            pins.sort();
            let id = self.id();
            self.s.nets.insert(
                id,
                SchematicNet {
                    name: name.into(),
                    pins,
                },
            );
        }
    }

    /// A 10 V source across a divider of 1k and 3k, and the id of the two-pin symbol.
    fn divider() -> (Builder, Id) {
        let mut b = Builder::new();
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "10");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "3k");
        b.net("IN", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("OUT", &[(r1, two, "2"), (r2, two, "1")]);
        b.net("GND", &[(v1, two, "2"), (r2, two, "2")]);
        (b, two)
    }

    #[test]
    fn the_divider_of_a_schematic_is_simulated() {
        let (b, _) = divider();
        let s = SimSettings::default();
        let (nl, report) = netlist_from_schematic(&b.s, &s);
        assert!(report.is_clean(), "{:?}", report.issues);
        assert_eq!(report.translated, vec!["R1", "R2", "V1"]);
        assert_eq!(report.grounds, vec!["GND"]);
        assert_eq!(report.nodes, 3);
        let c = Circuit::compile(&nl, &s).unwrap();
        let op = operating_point(&c, &s).unwrap();
        assert!((op.get("v(out)").unwrap() - 7.5).abs() < 1e-12);
        assert!((op.get("v(in)").unwrap() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn what_cannot_be_translated_is_reported_with_a_code() {
        let (mut b, two) = divider();
        let chip = b.symbol("chip", &["1", "2", "3"]);
        b.part(chip, "U1", "LM358");
        b.part(two, "R9", "abc");
        b.part(two, "C3", "");
        b.part(chip, "R7", "1k");
        b.part(two, "Rload", "1k");
        let s = SimSettings::default();
        let (nl, report) = netlist_from_schematic(&b.s, &s);
        let codes: Vec<&str> = report.issues.iter().map(|e| e.code).collect();
        // U1 (kind), R9 (value), C3 (no value), R7 (three pins), Rload (designator), plus the
        // unconnected pins of the parts that are on no net
        assert_eq!(codes.iter().filter(|c| **c == PART_SKIPPED).count(), 4);
        assert_eq!(codes.iter().filter(|c| **c == VALUE_INVALID).count(), 1);
        assert!(codes.contains(&PIN_UNCONNECTED));
        assert_eq!(nl.elements.len(), 3);
        assert!(report.translated.contains(&"R1".to_string()));
        assert!(!report.translated.contains(&"R9".to_string()));
        assert!(report.to_text(Lang::Es).contains("PHOENIX-SIM-014"));
        assert!(report
            .to_text(Lang::Pt)
            .contains("Não foi possível traduzir"));
    }

    #[test]
    fn a_schematic_without_ground_says_so() {
        let mut b = Builder::new();
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "1k");
        b.net("A", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("B", &[(v1, two, "2"), (r1, two, "2")]);
        let (nl, report) = netlist_from_schematic(&b.s, &SimSettings::default());
        assert!(report.issues.iter().any(|e| e.code == NO_GROUND));
        assert_eq!(
            Circuit::compile(&nl, &SimSettings::default())
                .unwrap_err()
                .code,
            NO_GROUND
        );
    }

    #[test]
    fn diodes_use_the_pins_a_and_k_and_the_value_as_the_model() {
        let mut b = Builder::new();
        let two = b.symbol("two-pin", &["1", "2"]);
        // the pins of the diode are named K and A, as many libraries do
        let diode = b.symbol("diode", &["K", "A"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "1k");
        let d1 = b.part(diode, "D1", "1N4148");
        b.net("IN", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("D", &[(r1, two, "2"), (d1, diode, "A")]);
        b.net("GND", &[(v1, two, "2"), (d1, diode, "K")]);
        let s = SimSettings::default();
        let (nl, report) = netlist_from_schematic(&b.s, &s);
        assert!(report.is_clean(), "{:?}", report.issues);
        assert!(nl.models.contains_key("1n4148"));
        let c = Circuit::compile(&nl, &s).unwrap();
        let op = operating_point(&c, &s).unwrap();
        let vd = op.get("v(d)").unwrap();
        assert!(vd > 0.6 && vd < 0.8, "{vd}");
    }

    #[test]
    fn nets_that_share_a_name_are_one_node_and_odd_names_are_cleaned() {
        let mut b = Builder::new();
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "1k");
        b.net("+5 V", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("+5 V", &[(r2, two, "1")]);
        b.net("AGND", &[(v1, two, "2"), (r1, two, "2"), (r2, two, "2")]);
        let (nl, report) = netlist_from_schematic(&b.s, &SimSettings::default());
        assert!(report.is_clean(), "{:?}", report.issues);
        assert_eq!(report.grounds, vec!["AGND"]);
        let r2_line = nl.elements.iter().find(|e| e.name == "R2").unwrap();
        assert_eq!(r2_line.nodes, vec!["_5_v".to_string(), "0".to_string()]);
    }
}
