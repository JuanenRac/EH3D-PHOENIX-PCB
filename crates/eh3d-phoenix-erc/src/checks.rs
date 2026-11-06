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
const RANK_NOT_JUDGED: u8 = 19;

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
    run.net_name_typos();
    run.decoupling();
    run.orphan_labels();
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

            if pins.len() >= 2 && self.config.level(Rule::NetWithoutDriver) != Level::Off {
                let strong = self
                    .config
                    .strong_drivers
                    .iter()
                    .any(|k| !by_kind[k.index()].is_empty());
                if !strong {
                    if unknown.is_empty() {
                        self.emit(
                            Rule::NetWithoutDriver,
                            place.clone(),
                            format!("{} pins and none drives the net", pins.len()),
                        );
                    } else {
                        self.not_judged(place.clone(), format!("no driver found; {unknown_text}"));
                    }
                }
            }

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
            let flagged = self
                .config
                .power_flags
                .iter()
                .any(|f| f.eq_ignore_ascii_case(net.name.trim()));
            if !by_kind[PinKind::PowerIn.index()].is_empty()
                && !has(&self.config.power_sources)
                && !flagged
            {
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

    /// Net names one character apart that the rule of similar names does not already name.
    fn net_name_typos(&mut self) {
        if self.config.level(Rule::NetNameTypo) == Level::Off {
            return;
        }
        let s = self.schematic;
        let min_length = self.config.typo_min_length.max(1);
        let mut names: Vec<&str> = s
            .nets
            .values()
            .map(|n| n.name.trim())
            .filter(|n| n.chars().count() >= min_length && !n.contains('$'))
            .collect();
        names.sort_unstable();
        names.dedup();
        for (i, a) in names.iter().enumerate() {
            for b in names.iter().skip(i + 1) {
                if self.full() {
                    self.cut = true;
                    return;
                }
                if typo_pair(a, b) {
                    self.emit(
                        Rule::NetNameTypo,
                        format!("net {a} / net {b}"),
                        String::new(),
                    );
                }
            }
        }
    }

    /// Power inputs of chips with no capacitor to ground between their net and ground, near them.
    fn decoupling(&mut self) {
        if self.config.level(Rule::MissingDecoupling) == Level::Off
            || self.config.decoupling_prefixes.is_empty()
            || self.config.capacitor_prefixes.is_empty()
        {
            return;
        }
        let s = self.schematic;
        let config = self.config;
        let chips = &config.decoupling_prefixes;
        let capacitors = &config.capacitor_prefixes;
        let limit = i128::from(config.decoupling_max_distance_nm);
        let ground_ids: BTreeSet<Id> = s
            .nets
            .iter()
            .filter(|(_, n)| {
                config
                    .ground_nets
                    .iter()
                    .any(|g| g.eq_ignore_ascii_case(n.name.trim()))
            })
            .map(|(id, _)| *id)
            .collect();
        let mut net_of: BTreeMap<NetPin, Id> = BTreeMap::new();
        for (id, net) in &s.nets {
            for pin in &net.pins {
                net_of.insert(*pin, *id);
            }
        }
        let positions: BTreeMap<NetPin, (Id, (i64, i64))> = s
            .pin_positions()
            .into_iter()
            .map(|(pin, sheet, at)| (pin, (sheet, (at.x_nm.get(), at.y_nm.get()))))
            .collect();

        let mut findings: Vec<(String, String)> = Vec::new();
        for (net_id, net) in &s.nets {
            if ground_ids.contains(net_id) {
                continue;
            }
            // the capacitors with one pin on this net and another on a ground net
            let mut caps: Vec<NetPin> = Vec::new();
            for pin in &net.pins {
                let Some(part) = s.parts.get(&pin.part) else {
                    continue;
                };
                if !has_prefix(&part.refdes, capacitors) {
                    continue;
                }
                let Some(symbol) = s.symbols.get(&part.symbol) else {
                    continue;
                };
                let to_ground = symbol.pins.keys().filter(|k| **k != pin.pin).any(|k| {
                    let other = NetPin {
                        part: pin.part,
                        pin: *k,
                    };
                    net_of.get(&other).is_some_and(|n| ground_ids.contains(n))
                });
                if to_ground {
                    caps.push(*pin);
                }
            }
            // the power inputs of chips on this net, by chip
            let mut by_chip: BTreeMap<Id, Vec<NetPin>> = BTreeMap::new();
            for pin in &net.pins {
                let Some(part) = s.parts.get(&pin.part) else {
                    continue;
                };
                if has_prefix(&part.refdes, chips)
                    && self.kind_of(pin.pin) == Some(PinKind::PowerIn)
                {
                    by_chip.entry(pin.part).or_default().push(*pin);
                }
            }
            let place_net = net_place(net_id, net);
            for (chip, pins) in by_chip {
                let refdes = s.parts.get(&chip).map_or("?", |p| p.refdes.as_str());
                if caps.is_empty() {
                    findings.push((
                        format!("{refdes} on {place_net}"),
                        "no capacitor between this net and ground".to_owned(),
                    ));
                    continue;
                }
                // the nearest pair; a pair with a position that is not given is near by connection
                let mut nearest: Option<i128> = None;
                let mut unknown = false;
                for a in &pins {
                    for b in &caps {
                        match (positions.get(a), positions.get(b)) {
                            (Some((sa, pa)), Some((sb, pb))) if sa == sb => {
                                let dx = i128::from(pa.0) - i128::from(pb.0);
                                let dy = i128::from(pa.1) - i128::from(pb.1);
                                let d2 = dx * dx + dy * dy;
                                nearest = Some(nearest.map_or(d2, |n| n.min(d2)));
                            }
                            (Some(_), Some(_)) => {}
                            _ => unknown = true,
                        }
                    }
                }
                if unknown || limit == 0 {
                    continue;
                }
                match nearest {
                    Some(d2) if d2 <= limit * limit => {}
                    Some(d2) => findings.push((
                        format!("{refdes} on {place_net}"),
                        format!(
                            "the nearest capacitor to ground is {:.1} mm away, the limit is {:.1} mm",
                            (d2 as f64).sqrt() / 1e6,
                            limit as f64 / 1e6
                        ),
                    )),
                    None => findings.push((
                        format!("{refdes} on {place_net}"),
                        "the capacitors to ground are on other sheets".to_owned(),
                    )),
                }
            }
        }
        for (place, detail) in findings {
            self.emit(Rule::MissingDecoupling, place, detail);
        }
    }

    /// Net labels and power ports that touch no wire and no pin.
    fn orphan_labels(&mut self) {
        if self.config.level(Rule::OrphanLabel) == Level::Off {
            return;
        }
        let s = self.schematic;
        if s.labels.is_empty() && s.power_ports.is_empty() {
            return;
        }
        let pins: BTreeSet<Node> = s
            .pin_positions()
            .into_iter()
            .map(|(_, sheet, at)| (sheet, (at.x_nm.get(), at.y_nm.get())))
            .collect();
        let mut unpositioned = 0usize;
        for part in s.parts.values() {
            if part.placement.is_none() {
                continue;
            }
            if let Some(symbol) = s.symbols.get(&part.symbol) {
                unpositioned += symbol.pins.values().filter(|p| p.at.is_none()).count();
            }
        }
        let marks = s
            .labels
            .values()
            .map(|l| ("label", l.sheet, l.at, l.name.as_str()))
            .chain(
                s.power_ports
                    .values()
                    .map(|p| ("power port", p.sheet, p.at, p.name.as_str())),
            );
        let mut found: Vec<(String, String)> = Vec::new();
        for (what, sheet, at, name) in marks {
            let Some(sheet_data) = s.sheets.get(&sheet) else {
                continue;
            };
            let point = (at.x_nm.get(), at.y_nm.get());
            let on_pin = pins.contains(&(sheet, point));
            let on_wire = s.wires.values().filter(|w| w.sheet == sheet).any(|w| {
                w.path.iter().any(|p| (p.x_nm.get(), p.y_nm.get()) == point)
                    || w.path.windows(2).any(|pair| {
                        on_segment(
                            point,
                            (pair[0].x_nm.get(), pair[0].y_nm.get()),
                            (pair[1].x_nm.get(), pair[1].y_nm.get()),
                        )
                    })
            });
            if on_pin || on_wire {
                continue;
            }
            let place = format!(
                "{what} {name} on sheet {} at ({}, {}) nm",
                sheet_data.name, point.0, point.1
            );
            if unpositioned > 0 {
                self.not_judged(
                    place,
                    format!(
                        "{unpositioned} pins of placed parts have no position and could lie under it"
                    ),
                );
            } else {
                found.push((place, String::new()));
            }
        }
        for (place, detail) in found {
            self.emit(Rule::OrphanLabel, place, detail);
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

/// The reference designator's letters in upper case (`R` of `r12`).
fn refdes_letters(refdes: &str) -> String {
    refdes
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// True when the letters of a reference designator are one of the prefixes (case does not matter).
fn has_prefix(refdes: &str, prefixes: &[String]) -> bool {
    let letters = refdes_letters(refdes);
    !letters.is_empty() && prefixes.iter().any(|p| p.eq_ignore_ascii_case(&letters))
}

/// True when two names, in lower case, differ by one character: one inserted, one dropped, one
/// changed or two neighbours swapped. A change of one digit to another (`D0` and `D1`) is a bus,
/// not a mistake.
fn one_edit_apart(x: &[char], y: &[char]) -> bool {
    let (nx, ny) = (x.len(), y.len());
    if nx.abs_diff(ny) > 1 {
        return false;
    }
    let prefix = x.iter().zip(y.iter()).take_while(|(p, q)| p == q).count();
    if nx == ny {
        if prefix == nx {
            return false;
        }
        if x[prefix + 1..] == y[prefix + 1..] {
            return !(x[prefix].is_ascii_digit() && y[prefix].is_ascii_digit());
        }
        prefix + 2 <= nx
            && x[prefix] == y[prefix + 1]
            && x[prefix + 1] == y[prefix]
            && x[prefix + 2..] == y[prefix + 2..]
    } else {
        let (long, short) = if nx > ny { (x, y) } else { (y, x) };
        long[prefix + 1..] == short[prefix..]
    }
}

/// True when two net names may be a typing mistake of each other. Names that differ only by case or
/// by the number on the end are the business of the rule of similar names.
fn typo_pair(a: &str, b: &str) -> bool {
    let (la, lb) = (a.to_lowercase(), b.to_lowercase());
    if la == lb || stem_of(&la) == stem_of(&lb) {
        return false;
    }
    let x: Vec<char> = la.chars().collect();
    let y: Vec<char> = lb.chars().collect();
    one_edit_apart(&x, &y)
}

/// True when `p` lies on the piece from `a` to `b` (ends included).
fn on_segment(p: (i64, i64), a: (i64, i64), b: (i64, i64)) -> bool {
    let (px, py) = (i128::from(p.0), i128::from(p.1));
    let (ax, ay) = (i128::from(a.0), i128::from(a.1));
    let (bx, by) = (i128::from(b.0), i128::from(b.1));
    let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
    cross == 0 && px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by)
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

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn names_one_character_apart_are_typing_mistakes_unless_they_are_a_bus() {
        for (a, b, expected) in [
            ("sda", "sdl", true),
            ("vbus", "vbuss", true),
            ("vbus", "vbs", true),
            ("sdal", "sdla", true),
            ("d0", "d1", false),
            ("adc1_in", "adc2_in", false),
            ("clk", "clk", false),
            ("clk", "data", false),
            ("abc", "abcde", false),
        ] {
            assert_eq!(one_edit_apart(&chars(a), &chars(b)), expected, "{a} {b}");
            assert_eq!(one_edit_apart(&chars(b), &chars(a)), expected, "{b} {a}");
        }
        assert!(!typo_pair("GND", "gnd"));
        assert!(!typo_pair("DATA", "DATA1"));
        assert!(typo_pair("RESET", "RESEET"));
    }

    #[test]
    fn a_prefix_is_the_letters_of_the_reference_without_regard_to_case() {
        let prefixes = vec!["U".to_owned(), "ic".to_owned()];
        assert_eq!(refdes_letters(" r12"), "R");
        assert!(has_prefix("U3", &prefixes));
        assert!(has_prefix("IC1", &prefixes));
        assert!(!has_prefix("UR1", &prefixes));
        assert!(!has_prefix("12", &prefixes));
        assert!(!has_prefix("", &prefixes));
    }

    #[test]
    fn a_point_is_on_a_piece_only_between_its_ends() {
        assert!(on_segment((5, 0), (0, 0), (10, 0)));
        assert!(on_segment((0, 0), (0, 0), (10, 0)));
        assert!(on_segment((3, 3), (0, 0), (10, 10)));
        assert!(!on_segment((11, 0), (0, 0), (10, 0)));
        assert!(!on_segment((5, 1), (0, 0), (10, 0)));
    }

    #[test]
    fn the_roots_of_joined_nodes_agree() {
        let mut parent = vec![0, 0, 1, 3];
        assert_eq!(find(&mut parent, 2), 0);
        assert_eq!(find(&mut parent, 3), 3);
    }
}
