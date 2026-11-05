// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks themselves, in a fixed order. They read the schematic and the settings and change
//! nothing. What the net list says is the truth for the connections; the wires of the drawing are
//! only looked at for loose ends.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_rules::Severity;
use eh3d_phoenix_schematic::{Id, NetPin, PinKind, Schematic, SchematicNet};

use crate::codes::{Rule, NOT_JUDGED};
use crate::config::{ErcConfig, Level};
use crate::report::{Finding, Report};

/// The place of what could not be judged in the order of the report.
const RANK_NOT_JUDGED: u8 = 15;

/// A pin of a net as the report names it.
struct PinInfo {
    refdes: String,
    number: String,
    kind: Option<PinKind>,
}

impl PinInfo {
    fn label(&self) -> String {
        let refdes = if self.refdes.trim().is_empty() {
            "?"
        } else {
            self.refdes.as_str()
        };
        format!("{refdes}.{}", self.number)
    }
}

/// A point of a sheet: the sheet and the place in nanometres.
type Node = (Id, (i64, i64));

struct Run<'a> {
    schematic: &'a Schematic,
    config: &'a ErcConfig,
    report: Report,
    cut: bool,
}

/// Checks a schematic against the settings. The report has the proven findings and, apart, what
/// could not be judged (pins without a type, pins without a position, a list that was cut).
pub fn check(schematic: &Schematic, config: &ErcConfig) -> Report {
    let mut run = Run {
        schematic,
        config,
        report: Report::default(),
        cut: false,
    };
    run.parts();
    run.symbols();
    run.nets();
    run.net_names();
    run.wires();
    run.untyped_pins();
    if run.cut {
        let detail = format!(
            "more findings than the {} that the settings allow; the rest are not listed",
            config.max_findings
        );
        run.report.inconclusive.push(Finding {
            rank: RANK_NOT_JUDGED,
            rule: "not_judged",
            code: NOT_JUDGED,
            severity: Severity::Info,
            location: "report".to_owned(),
            detail,
        });
    }
    run.report.finish();
    run.report
}

impl Run<'_> {
    fn full(&self) -> bool {
        self.report.confirmed.len() >= self.config.max_findings
    }

    /// Adds a proven finding of a rule at its level; nothing when the rule is off.
    fn add(&mut self, rule: Rule, level: Level, location: String, detail: String) {
        let Some(severity) = level.severity() else {
            return;
        };
        if self.full() {
            self.cut = true;
            return;
        }
        self.report.confirmed.push(Finding {
            rank: rule.rank(),
            rule: rule.name(),
            code: rule.code(),
            severity,
            location,
            detail,
        });
    }

    fn emit(&mut self, rule: Rule, location: String, detail: String) {
        let level = self.config.level(rule);
        self.add(rule, level, location, detail);
    }

    fn not_judged(&mut self, location: String, detail: String) {
        self.report.inconclusive.push(Finding {
            rank: RANK_NOT_JUDGED,
            rule: "not_judged",
            code: NOT_JUDGED,
            severity: Severity::Info,
            location,
            detail,
        });
    }

    fn kind_of(&self, pin: Id) -> Option<PinKind> {
        self.schematic.pin_kinds.get(&pin).copied()
    }

    fn pin_info(&self, np: NetPin) -> Option<PinInfo> {
        let part = self.schematic.parts.get(&np.part)?;
        let symbol = self.schematic.symbols.get(&part.symbol)?;
        let pin = symbol.pins.get(&np.pin)?;
        Some(PinInfo {
            refdes: part.refdes.clone(),
            number: pin.number.clone(),
            kind: self.kind_of(np.pin),
        })
    }

    /// Unconnected pins, duplicate and empty references, empty values, missing footprints.
    fn parts(&mut self) {
        let s = self.schematic;
        let connected: BTreeSet<NetPin> = s
            .nets
            .values()
            .flat_map(|n| n.pins.iter().copied())
            .collect();
        let mut by_refdes: BTreeMap<&str, usize> = BTreeMap::new();
        for (part_id, part) in &s.parts {
            let refdes = part.refdes.trim();
            if refdes.is_empty() {
                self.emit(Rule::EmptyRefdes, format!("part {part_id}"), String::new());
            } else {
                *by_refdes.entry(refdes).or_default() += 1;
            }
            let name = if refdes.is_empty() { "?" } else { refdes };
            if part.value.trim().is_empty() {
                self.emit(Rule::EmptyValue, name.to_owned(), String::new());
            }
            if part.footprint.is_none() {
                self.emit(Rule::PartWithoutFootprint, name.to_owned(), String::new());
            }
            let Some(symbol) = s.symbols.get(&part.symbol) else {
                continue;
            };
            for (pin_id, pin) in &symbol.pins {
                let np = NetPin {
                    part: *part_id,
                    pin: *pin_id,
                };
                if connected.contains(&np) || self.kind_of(*pin_id) == Some(PinKind::NotConnected) {
                    continue;
                }
                self.emit(
                    Rule::PinUnconnected,
                    format!("{name}.{}", pin.number),
                    String::new(),
                );
            }
        }
        for (refdes, count) in by_refdes {
            if count > 1 {
                self.emit(
                    Rule::DuplicateRefdes,
                    refdes.to_owned(),
                    format!("{count} parts"),
                );
            }
        }
    }

    /// Pins without a number and pin numbers used twice.
    fn symbols(&mut self) {
        let s = self.schematic;
        for symbol in s.symbols.values() {
            let mut numbers: BTreeMap<&str, usize> = BTreeMap::new();
            let mut blank = 0usize;
            for pin in symbol.pins.values() {
                let number = pin.number.trim();
                if number.is_empty() {
                    blank += 1;
                } else {
                    *numbers.entry(number).or_default() += 1;
                }
            }
            let place = format!("symbol {}", symbol.name);
            if blank > 0 {
                self.emit(
                    Rule::PinWithoutNumber,
                    place.clone(),
                    format!("{blank} pins"),
                );
            }
            for (number, count) in numbers {
                if count > 1 {
                    self.emit(
                        Rule::DuplicatePinNumber,
                        place.clone(),
                        format!("number {number} on {count} pins"),
                    );
                }
            }
        }
    }

    /// What each net holds: one pin, conflicting types, no driver, no source.
    fn nets(&mut self) {
        let s = self.schematic;
        for (net_id, net) in &s.nets {
            let place = net_place(net_id, net);
            let mut pins: Vec<PinInfo> =
                net.pins.iter().filter_map(|p| self.pin_info(*p)).collect();
            pins.sort_by(|a, b| {
                (a.refdes.as_str(), a.number.as_str()).cmp(&(b.refdes.as_str(), b.number.as_str()))
            });

            if pins.len() <= 1 && pins.iter().all(|p| p.kind != Some(PinKind::NotConnected)) {
                let detail = pins
                    .first()
                    .map_or_else(|| "no pin".to_owned(), |p| format!("only {}", p.label()));
                self.emit(Rule::SinglePinNet, place.clone(), detail);
            }

            // the pins of each kind, in the fixed order of the pins
            let mut by_kind: Vec<Vec<&PinInfo>> = vec![Vec::new(); PinKind::ALL.len()];
            let mut unknown: Vec<&PinInfo> = Vec::new();
            for p in &pins {
                match p.kind {
                    Some(kind) => by_kind[kind.index()].push(p),
                    None => unknown.push(p),
                }
            }

            if self.config.level(Rule::PinTypeConflict) != Level::Off {
                for (i, ka) in PinKind::ALL.into_iter().enumerate() {
                    for kb in PinKind::ALL.into_iter().skip(i) {
                        let level = self.config.pair_level(ka, kb);
                        let (na, nb) = (&by_kind[ka.index()], &by_kind[kb.index()]);
                        let (first, second, pairs) = if ka == kb {
                            if na.len() < 2 {
                                continue;
                            }
                            (na[0], na[1], na.len() * (na.len() - 1) / 2)
                        } else {
                            if na.is_empty() || nb.is_empty() {
                                continue;
                            }
                            (na[0], nb[0], na.len() * nb.len())
                        };
                        let mut detail = format!(
                            "{} {} with {} {}",
                            ka.as_str(),
                            first.label(),
                            kb.as_str(),
                            second.label()
                        );
                        if pairs > 1 {
                            detail =
                                format!("{detail} (and {} more pairs of these types)", pairs - 1);
                        }
                        self.add(Rule::PinTypeConflict, level, place.clone(), detail);
                    }
                }
            }

            let has = |kinds: &[PinKind]| kinds.iter().any(|k| !by_kind[k.index()].is_empty());
            let listed = |p: &[&PinInfo]| -> String {
                p.iter()
                    .take(3)
                    .map(|p| p.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let unknown_text = if unknown.is_empty() {
                String::new()
            } else {
                format!(
                    "{} pins without a type could be the missing part: {}",
                    unknown.len(),
                    listed(&unknown)
                )
            };

            if !by_kind[PinKind::Input.index()].is_empty() && !has(&self.config.drivers) {
                let inputs = listed(&by_kind[PinKind::Input.index()]);
                if unknown.is_empty() {
                    self.emit(
                        Rule::InputWithoutDriver,
                        place.clone(),
                        format!("inputs {inputs}"),
                    );
                } else if self.config.level(Rule::InputWithoutDriver) != Level::Off {
                    self.not_judged(place.clone(), format!("inputs {inputs}; {unknown_text}"));
                }
            }
            if !by_kind[PinKind::PowerIn.index()].is_empty() && !has(&self.config.power_sources) {
                let inputs = listed(&by_kind[PinKind::PowerIn.index()]);
                if unknown.is_empty() {
                    self.emit(
                        Rule::PowerWithoutSource,
                        place.clone(),
                        format!("power inputs {inputs}"),
                    );
                } else if self.config.level(Rule::PowerWithoutSource) != Level::Off {
                    self.not_judged(
                        place.clone(),
                        format!("power inputs {inputs}; {unknown_text}"),
                    );
                }
            }
        }
    }

    /// Net names that differ only by case or by a number on the end.
    fn net_names(&mut self) {
        let s = self.schematic;
        let min_stem = self.config.similar_name_min_stem;
        let mut by_stem: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for net in s.nets.values() {
            let name = net.name.trim();
            if name.is_empty() {
                continue;
            }
            let stem = stem_of(name);
            if stem.chars().count() < min_stem.max(1) {
                continue;
            }
            by_stem.entry(stem).or_default().insert(name);
        }
        for (stem, names) in by_stem {
            let names: Vec<&str> = names.into_iter().collect();
            for (i, a) in names.iter().enumerate() {
                for b in names.iter().skip(i + 1) {
                    if self.full() {
                        self.cut = true;
                        return;
                    }
                    let (la, lb) = (a.to_lowercase(), b.to_lowercase());
                    if la == lb || la == stem || lb == stem {
                        self.emit(
                            Rule::SimilarNetNames,
                            format!("net {a} / net {b}"),
                            String::new(),
                        );
                    }
                }
            }
        }
    }

    /// Wires that join no pin, and junctions on them.
    fn wires(&mut self) {
        let s = self.schematic;
        if s.wires.is_empty() {
            return;
        }
        // pins of placed parts whose position is not given may sit on any wire
        let mut unpositioned = 0usize;
        for part in s.parts.values() {
            if part.placement.is_none() {
                continue;
            }
            if let Some(symbol) = s.symbols.get(&part.symbol) {
                unpositioned += symbol.pins.values().filter(|p| p.at.is_none()).count();
            }
        }

        let mut index: BTreeMap<Node, usize> = BTreeMap::new();
        let mut parent: Vec<usize> = Vec::new();
        let mut touches: Vec<usize> = Vec::new();
        let mut node = |sheet: Id, x: i64, y: i64, parent: &mut Vec<usize>| -> usize {
            *index.entry((sheet, (x, y))).or_insert_with(|| {
                parent.push(parent.len());
                parent.len() - 1
            })
        };
        let mut pieces: Vec<(usize, usize)> = Vec::new();
        for wire in s.wires.values() {
            let mut previous: Option<usize> = None;
            for p in &wire.path {
                let n = node(wire.sheet, p.x_nm.get(), p.y_nm.get(), &mut parent);
                if let Some(before) = previous {
                    if before != n {
                        pieces.push((before, n));
                    }
                }
                previous = Some(n);
            }
        }
        touches.resize(parent.len(), 0);
        for (a, b) in &pieces {
            touches[*a] += 1;
            touches[*b] += 1;
            let (ra, rb) = (find(&mut parent, *a), find(&mut parent, *b));
            if ra != rb {
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
        let mut with_pin: BTreeSet<usize> = BTreeSet::new();
        for (_, sheet, at) in s.pin_positions() {
            if let Some(n) = index.get(&(sheet, (at.x_nm.get(), at.y_nm.get()))) {
                with_pin.insert(find(&mut parent, *n));
            }
        }

        // the nodes of each group of wires, with their point
        let mut groups: BTreeMap<usize, Vec<(Node, usize)>> = BTreeMap::new();
        for (key, n) in &index {
            let root = find(&mut parent, *n);
            groups.entry(root).or_default().push((*key, *n));
        }
        for (root, nodes) in groups {
            if with_pin.contains(&root) {
                continue;
            }
            let place_of = |sheet: Id, (x, y): (i64, i64)| -> String {
                let name = s.sheets.get(&sheet).map_or("?", |sh| sh.name.as_str());
                format!("sheet {name} at ({x}, {y}) nm")
            };
            let Some(((sheet, point), _)) = nodes.iter().min_by_key(|((sheet, p), _)| (*p, *sheet))
            else {
                continue;
            };
            let first = place_of(*sheet, *point);
            let count = nodes.len();
            if unpositioned > 0 {
                self.not_judged(
                    first.clone(),
                    format!(
                        "wires of {count} points join no pin, but {unpositioned} pins of placed parts have no position"
                    ),
                );
            } else {
                self.emit(Rule::FloatingWires, first, format!("{count} points"));
            }
            for ((sheet, point), n) in &nodes {
                if touches[*n] >= 3 {
                    let place = place_of(*sheet, *point);
                    if unpositioned > 0 {
                        self.not_judged(
                            place,
                            format!("junction on wires that may join a pin without a position ({unpositioned} pins)"),
                        );
                    } else {
                        self.emit(
                            Rule::JunctionOnFloatingWires,
                            place,
                            format!("{} wire pieces meet", touches[*n]),
                        );
                    }
                }
            }
        }
    }

    /// Says once how many pins on nets have no type, so a schematic without types is not mistaken
    /// for a clean one.
    fn untyped_pins(&mut self) {
        let s = self.schematic;
        let count = s
            .nets
            .values()
            .flat_map(|n| n.pins.iter())
            .filter(|np| self.pin_info(**np).is_some() && self.kind_of(np.pin).is_none())
            .count();
        if count > 0 {
            self.not_judged(
                "schematic".to_owned(),
                format!("{count} pins on nets have no electrical type; the pin type checks leave them out"),
            );
        }
    }
}

/// How a net is named in the report: by its name, or by its id when it has none.
fn net_place(id: &Id, net: &SchematicNet) -> String {
    if net.name.trim().is_empty() {
        format!("net {id}")
    } else {
        format!("net {}", net.name)
    }
}

/// The name in lower case without the digits, dashes and underscores on its end.
fn stem_of(name: &str) -> String {
    name.to_lowercase()
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '_' || c == '-')
        .to_owned()
}

/// The root of a group of nodes, shortening the path on the way.
fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stem_of_a_name_drops_the_number_and_the_case() {
        assert_eq!(stem_of("GND1"), "gnd");
        assert_eq!(stem_of("Vcc"), "vcc");
        assert_eq!(stem_of("D_12"), "d");
        assert_eq!(stem_of("+3V3"), "+3v");
        assert_eq!(stem_of("123"), "");
    }

    #[test]
    fn the_roots_of_joined_nodes_agree() {
        let mut parent = vec![0, 0, 1, 3];
        assert_eq!(find(&mut parent, 2), 0);
        assert_eq!(find(&mut parent, 3), 3);
    }
}
