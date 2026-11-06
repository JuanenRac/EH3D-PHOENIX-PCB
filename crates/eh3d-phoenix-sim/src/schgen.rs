// SPDX-License-Identifier: GPL-3.0-or-later
//! The netlist of a simulation made from a schematic, with no netlist written by hand.
//!
//! Each part gets its model from the table of models ([`ModelTable`], `config/sim_models.cfg`):
//! a rule by value, by symbol name or by the prefix of the reference designator, or the attributes
//! of the part (`SIM.DEVICE`, `SIM.VALUE`, `SPICE_MODEL`, `SIM.FILE`, `SIM.PINS`), which win. The
//! pins of the symbol are given to the roles of the device by their names (`A` and `K`, `C B E`,
//! `D G S`) or by their order, or by the `pins=` of the rule. Values are read with the notation
//! of the markings of parts (`4k7`, `1u5`, `2M2`).
//!
//! The nets of the schematic become the nodes of the netlist with names that do not change when
//! something else is edited: a net named `GND`, `AGND`, `0` and the other names of the setting
//! `schematic_ground` is node 0; a named net is its name in lower case with anything but letters,
//! digits and underscores as an underscore; a net with no name is `n_<refdes>_<pin>` of its first
//! pin in the order of the designators and the pin numbers.
//!
//! Everything that cannot be simulated is in the report with a code `PHOENIX-NETLIST-<NNN>` and a
//! severity; nothing is dropped silently. The result is SPICE text that the reader of the crate
//! reads back, with the subcircuits of `.include` files of one folder.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_schematic::{
    EditError, HierConfig, Id, NetPin, Part, Pin, Schematic, SchematicNet,
};

use crate::circuit::Circuit;
use crate::codes::SimError;
use crate::dc::operating_point;
use crate::export::{clean, pin_key};
use crate::include::read_netlist_text;
use crate::modeltable::{is_plain_file, is_plain_name, Device, ModelTable, PartProps, Rule};
use crate::netcodes::{
    GenIssue, MODEL_UNDEFINED, NET_FLOATING, NO_GROUND_NET, NO_MODEL, PINS_MISMATCH, PIN_ON_NO_NET,
    REFDES_TWICE, SOURCE_SHORTED, VALUE_BAD,
};
use crate::netlist::Netlist;
use crate::parse::parse_element_line;
use crate::partvalue::{source_text, value_text};
use crate::settings::SimSettings;
use crate::units::format_number;

/// A node of the netlist and the net of the schematic it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeName {
    /// The node in the netlist (`out`, `0`, `n_r1_2`).
    pub node: String,
    /// The name of the net in the schematic, or the node when the net has no name.
    pub net: String,
}

/// A source whose current is an unknown of the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    /// The reference designator of the part.
    pub refdes: String,
    /// The element in the netlist, in lower case (`v1`).
    pub element: String,
}

/// A voltmeter of the schematic: the voltage from the plus node to the minus node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meter {
    /// The reference designator of the part.
    pub refdes: String,
    /// The node of the plus pin.
    pub plus: String,
    /// The node of the minus pin.
    pub minus: String,
}

/// What the translation did and what it could not do.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetlistReport {
    /// The reference designators that became elements, in the order of the netlist.
    pub translated: Vec<String>,
    /// What could not be translated or looks wrong, with a code and a severity.
    pub issues: Vec<GenIssue>,
    /// The names of the nets that were taken as ground.
    pub grounds: Vec<String>,
    /// The number of nodes of the netlist, ground included.
    pub nodes: usize,
    /// The nodes with the net each one stands for.
    pub node_names: Vec<NodeName>,
    /// The sources whose current can be read.
    pub currents: Vec<SourceRef>,
    /// The voltmeters.
    pub meters: Vec<Meter>,
    /// The files that the netlist includes (they must be in the folder of the netlist).
    pub includes: Vec<String>,
}

impl NetlistReport {
    /// True when there is nothing but information to say.
    pub fn is_clean(&self) -> bool {
        self.issues.iter().all(|i| i.severity == Severity::Info)
    }

    /// True when something stops a simulation.
    pub fn has_errors(&self) -> bool {
        self.count(Severity::Error) > 0
    }

    /// The number of issues of one severity.
    pub fn count(&self, severity: Severity) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .count()
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
            let _ = writeln!(out, "ground nets: {}", self.grounds.join(", "));
        }
        if !self.includes.is_empty() {
            let _ = writeln!(out, "includes: {}", self.includes.join(", "));
        }
        for issue in &self.issues {
            out.push_str(&issue.message(lang));
            out.push('\n');
        }
        out
    }
}

/// The text of the netlist and its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetlistText {
    /// The SPICE text.
    pub text: String,
    /// What was translated and what was not.
    pub report: NetlistReport,
}

/// The nodes of the nets.
struct Wiring {
    pin_node: BTreeMap<NetPin, String>,
    shown: BTreeMap<String, String>,
    grounds: Vec<String>,
}

/// The name of a net with no name: the first pin of the net in the order of designators and pin
/// numbers, which does not change when something else is edited.
fn stable_name(schematic: &Schematic, net: &SchematicNet) -> String {
    let first = net
        .pins
        .iter()
        .filter_map(|pin| {
            let part = schematic.parts.get(&pin.part)?;
            let number = schematic.pin_number(*pin)?;
            Some((clean(&part.refdes), pin_key(number), clean(number)))
        })
        .min();
    match first {
        Some((refdes, _, number)) => format!("n_{refdes}_{number}"),
        None => "n_empty".to_owned(),
    }
}

fn wire(schematic: &Schematic, settings: &SimSettings) -> Wiring {
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for net in schematic.nets.values() {
        if !settings.is_schematic_ground(&net.name) {
            let cleaned = clean(&net.name);
            if !cleaned.is_empty() {
                taken.insert(cleaned);
            }
        }
    }
    let mut pin_node: BTreeMap<NetPin, String> = BTreeMap::new();
    let mut shown: BTreeMap<String, String> = BTreeMap::new();
    let mut grounds: BTreeSet<String> = BTreeSet::new();
    for net in schematic.nets.values() {
        let name = net.name.trim();
        let (node, label) = if settings.is_schematic_ground(name) {
            grounds.insert(name.to_owned());
            ("0".to_owned(), name.to_owned())
        } else {
            let cleaned = clean(name);
            if cleaned.is_empty() {
                let mut generated = stable_name(schematic, net);
                while taken.contains(&generated) {
                    generated.push('_');
                }
                taken.insert(generated.clone());
                (generated.clone(), generated)
            } else {
                (cleaned, name.to_owned())
            }
        };
        for pin in &net.pins {
            pin_node.insert(*pin, node.clone());
        }
        shown.entry(node).or_insert(label);
    }
    Wiring {
        pin_node,
        shown,
        grounds: grounds.into_iter().collect(),
    }
}

/// Which pin of the symbol (by its place in the list sorted by number) goes to each role of the
/// device. `None` is a role that may be left out (the bulk of a MOSFET).
fn assign_pins(
    device: Device,
    numbers: &[String],
    explicit: &[String],
) -> Result<Vec<Option<usize>>, String> {
    let lower: Vec<String> = numbers.iter().map(|n| n.to_ascii_lowercase()).collect();
    let find = |name: &str| {
        lower
            .iter()
            .position(|n| *n == name.trim().to_ascii_lowercase())
    };
    if device == Device::Subckt {
        if explicit.is_empty() {
            return Ok((0..numbers.len()).map(Some).collect());
        }
        return explicit
            .iter()
            .map(|e| {
                find(e)
                    .map(Some)
                    .ok_or_else(|| format!("the symbol has no pin {e}"))
            })
            .collect();
    }
    let roles = device.roles();
    let required = device.required();
    if !explicit.is_empty() {
        if !(required..=roles.len()).contains(&explicit.len()) {
            return Err(format!(
                "pins= lists {} pins and a {} takes {} to {}",
                explicit.len(),
                device.name(),
                required,
                roles.len()
            ));
        }
        let mut out: Vec<Option<usize>> = Vec::new();
        for e in explicit {
            let at = find(e).ok_or_else(|| format!("the symbol has no pin {e}"))?;
            if out.contains(&Some(at)) {
                return Err(format!("pins= lists the pin {e} twice"));
            }
            out.push(Some(at));
        }
        out.resize(roles.len(), None);
        return Ok(out);
    }
    let by_name: Vec<Option<usize>> = roles
        .iter()
        .map(|aliases| aliases.iter().find_map(|alias| find(alias)))
        .collect();
    let found: BTreeSet<usize> = by_name.iter().flatten().copied().collect();
    let all_required = by_name.iter().take(required).all(Option::is_some);
    if all_required && found.len() == by_name.iter().flatten().count() {
        return Ok(by_name);
    }
    if (required..=roles.len()).contains(&numbers.len()) {
        return Ok((0..roles.len())
            .map(|i| (i < numbers.len()).then_some(i))
            .collect());
    }
    Err(format!(
        "the symbol has {} pins that are not named for the roles of a {}, which takes {} to {} (give the order with pins=)",
        numbers.len(),
        device.name(),
        required,
        roles.len()
    ))
}

/// The name of an element: the letter of its kind and the designator.
fn element_name(letter: char, refdes: &str) -> String {
    let plain: String = refdes
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let starts = plain.len() > 1
        && plain
            .chars()
            .next()
            .is_some_and(|c| c.eq_ignore_ascii_case(&letter));
    if starts {
        plain
    } else {
        format!("{letter}_{plain}")
    }
}

fn title_line(name: &str) -> String {
    let flat: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    if flat.is_empty() {
        "schematic".to_owned()
    } else {
        flat.to_owned()
    }
}

struct Builder<'a> {
    schematic: &'a Schematic,
    table: &'a ModelTable,
    props: Option<&'a PartProps>,
    /// The reference on its sheet of each reference of an expanded schematic (`R1` for `U1/R1`);
    /// the rules of the table look at that one.
    local: Option<&'a BTreeMap<String, String>>,
    settings: &'a SimSettings,
    wiring: Wiring,
    report: NetlistReport,
    lines: Vec<String>,
    models: BTreeMap<String, String>,
    model_types: BTreeMap<String, &'static str>,
    includes: BTreeSet<String>,
    names: BTreeSet<String>,
    used: BTreeSet<String>,
    uses: BTreeMap<String, usize>,
    not_connected: BTreeSet<String>,
    pairs: BTreeMap<(String, String), String>,
}

impl Builder<'_> {
    fn issue(&mut self, code: &'static str, severity: Severity, detail: String) {
        self.report
            .issues
            .push(GenIssue::new(code, severity, detail));
    }

    fn bad(&mut self, refdes: &str, why: &str) {
        self.issue(VALUE_BAD, Severity::Error, format!("{refdes}: {why}"));
    }

    /// The value of a resistor, a capacitor or an inductor as the text of a netlist.
    fn number_value(&mut self, refdes: &str, value: &str) -> Option<String> {
        if value.trim().is_empty() {
            self.bad(refdes, "the part has no value");
            return None;
        }
        let text = value_text(value, self.table.upper_m_is_mega);
        if text.is_none() {
            self.bad(
                refdes,
                &format!("the value {} is not a number", value.trim()),
            );
        }
        text
    }

    fn note_type(&mut self, refdes: &str, name: &str, type_word: &'static str) -> bool {
        match self.model_types.get(name) {
            Some(known) if *known != type_word => {
                let detail =
                    format!("{refdes}: the model {name} is used as {known} and as {type_word}");
                self.issue(MODEL_UNDEFINED, Severity::Error, detail);
                false
            }
            _ => {
                self.model_types.insert(name.to_owned(), type_word);
                true
            }
        }
    }

    /// The name of the model of a part, and the `.model` line or the include that defines it.
    fn model_for(&mut self, refdes: &str, rule: &Rule, value: &str) -> Option<String> {
        let table = self.table;
        let device = rule.device;
        let type_word = device.model_type()?;
        let from_value = clean(value);
        let generic = format!("{}_generic", device.name());
        let (name, explicit) = match &rule.model {
            Some(model) => (model.to_ascii_lowercase(), true),
            None if from_value.is_empty() => (generic.clone(), false),
            None => (from_value, false),
        };
        if let Some(file) = &rule.file {
            self.includes.insert(file.clone());
            return Some(name);
        }
        if let Some(line) = table.model_line(&name) {
            if !self.note_type(refdes, &name, type_word) {
                return None;
            }
            self.models
                .entry(name.clone())
                .or_insert_with(|| line.to_owned());
            return Some(name);
        }
        if type_word == "ZENER" {
            let detail = format!(
                "{refdes}: a Zener diode needs a model with its breakdown voltage (a .model line with BV in the table, or file=)"
            );
            self.issue(MODEL_UNDEFINED, Severity::Error, detail);
            return None;
        }
        if explicit {
            let detail = format!(
                "{refdes}: the model {name} is named and no .model line or file= defines it"
            );
            self.issue(MODEL_UNDEFINED, Severity::Error, detail);
            return None;
        }
        if !self.note_type(refdes, &name, type_word) {
            return None;
        }
        if name != generic {
            let detail = format!(
                "{refdes}: the table has no model named {name}; the generic {type_word} model of the settings is used"
            );
            self.issue(NO_MODEL, Severity::Info, detail);
        }
        self.models
            .entry(name.clone())
            .or_insert_with(|| format!(".model {name} {type_word}"));
        Some(name)
    }

    /// The line of the element, or `None` after the reason was reported.
    fn line_for(
        &mut self,
        refdes: &str,
        name: &str,
        rule: &Rule,
        roles: &[Option<String>],
        value: &str,
    ) -> Option<String> {
        let node = |i: usize| -> String { roles.get(i).cloned().flatten().unwrap_or_default() };
        match rule.device {
            Device::Resistor | Device::Capacitor | Device::Inductor => {
                let number = self.number_value(refdes, value)?;
                Some(format!("{name} {} {} {number}", node(0), node(1)))
            }
            Device::VSource | Device::ISource => {
                let Some(text) = source_text(value, self.table.upper_m_is_mega) else {
                    self.bad(
                        refdes,
                        &format!("the value '{}' is not the value of a source", value.trim()),
                    );
                    return None;
                };
                Some(format!("{name} {} {} {text}", node(0), node(1)))
            }
            Device::Ammeter => Some(format!("{name} {} {} DC 0", node(0), node(1))),
            Device::Diode | Device::Led | Device::Zener => {
                let model = self.model_for(refdes, rule, value)?;
                Some(format!("{name} {} {} {model}", node(0), node(1)))
            }
            Device::Npn | Device::Pnp => {
                let model = self.model_for(refdes, rule, value)?;
                Some(format!(
                    "{name} {} {} {} {model}",
                    node(0),
                    node(1),
                    node(2)
                ))
            }
            Device::Nmos | Device::Pmos => {
                let model = self.model_for(refdes, rule, value)?;
                let bulk = if roles.get(3).is_some_and(Option::is_some) {
                    node(3)
                } else {
                    node(2)
                };
                let mut line = format!("{name} {} {} {} {bulk} {model}", node(0), node(1), node(2));
                if let Some(width) = &rule.width {
                    line.push_str(" W=");
                    line.push_str(width);
                }
                if let Some(length) = &rule.length {
                    line.push_str(" L=");
                    line.push_str(length);
                }
                Some(line)
            }
            Device::Switch => {
                let model = self.model_for(refdes, rule, value)?;
                Some(format!(
                    "{name} {} {} {} {} {model}",
                    node(0),
                    node(1),
                    node(2),
                    node(3)
                ))
            }
            Device::OpAmp | Device::Comparator => {
                let model = self.model_for(refdes, rule, value)?;
                Some(format!(
                    "{name} {} {} {} {model}",
                    node(0),
                    node(1),
                    node(2)
                ))
            }
            Device::Subckt => {
                let Some(file) = rule.file.clone() else {
                    self.bad(refdes, "a subcircuit needs a file (file= or SIM.FILE)");
                    return None;
                };
                let subckt = match &rule.subckt {
                    Some(s) => s.clone(),
                    None if is_plain_name(value.trim()) => value.trim().to_owned(),
                    None => {
                        self.bad(
                            refdes,
                            "the name of the subcircuit is missing (subckt= or SPICE_MODEL)",
                        );
                        return None;
                    }
                };
                self.includes.insert(file);
                let nodes: Vec<String> = roles.iter().flatten().cloned().collect();
                Some(format!("{name} {} {subckt}", nodes.join(" ")))
            }
            Device::Voltmeter | Device::Skip => None,
        }
    }

    fn part(&mut self, id: Id, part: &Part) {
        let schematic = self.schematic;
        let table = self.table;
        let refdes = part.refdes.trim().to_owned();
        let Some(symbol) = schematic.symbols.get(&part.symbol) else {
            self.issue(
                PINS_MISMATCH,
                Severity::Error,
                format!("{refdes}: its symbol does not exist"),
            );
            return;
        };

        let mut attrs: BTreeMap<String, String> =
            table.attributes(&refdes).cloned().unwrap_or_default();
        if let Some(extra) = self.props.and_then(|p| p.get(&refdes.to_ascii_lowercase())) {
            for (key, value) in extra {
                attrs.insert(key.to_ascii_uppercase(), value.clone());
            }
        }

        let device_attr = match attrs.get("SIM.DEVICE") {
            Some(word) => match Device::from_name(word) {
                Some(device) => Some(device),
                None => {
                    self.bad(&refdes, &format!("SIM.DEVICE {word} is not a device"));
                    return;
                }
            },
            None => None,
        };
        let rule_refdes = self
            .local
            .and_then(|m| m.get(&refdes))
            .map_or(refdes.as_str(), String::as_str);
        let found = table.find(rule_refdes, &part.value, &symbol.name);
        let mut rule = match (device_attr, found) {
            (Some(device), Some(r)) if r.device == device => r.clone(),
            (Some(device), _) => Rule::new(device),
            (None, Some(r)) => r.clone(),
            (None, None) => {
                self.issue(
                    NO_MODEL,
                    Severity::Warning,
                    format!(
                        "{refdes}: no rule of the table matches its designator, its value '{}' or its symbol '{}'",
                        part.value.trim(),
                        symbol.name.trim()
                    ),
                );
                return;
            }
        };
        if let Some(model) = attrs.get("SPICE_MODEL") {
            if !is_plain_name(model) {
                self.bad(&refdes, &format!("SPICE_MODEL {model} is not a plain name"));
                return;
            }
            if rule.device == Device::Subckt {
                rule.subckt = Some(model.clone());
            } else {
                rule.model = Some(model.clone());
            }
        }
        if let Some(file) = attrs.get("SIM.FILE") {
            if !is_plain_file(file) {
                self.bad(
                    &refdes,
                    &format!("SIM.FILE {file} is not a file in the folder of the netlist"),
                );
                return;
            }
            rule.file = Some(file.clone());
        }
        if let Some(pins) = attrs.get("SIM.PINS") {
            rule.pins = pins
                .split(',')
                .map(|p| p.trim().to_owned())
                .filter(|p| !p.is_empty())
                .collect();
        }
        if rule.device == Device::Skip {
            return;
        }
        let value = attrs
            .get("SIM.VALUE")
            .cloned()
            .unwrap_or_else(|| part.value.trim().to_owned());

        let mut pins: Vec<(&Id, &Pin)> = symbol.pins.iter().collect();
        pins.sort_by_key(|(_, pin)| pin_key(&pin.number));
        let numbers: Vec<String> = pins
            .iter()
            .map(|(_, p)| p.number.trim().to_owned())
            .collect();
        let picked = match assign_pins(rule.device, &numbers, &rule.pins) {
            Ok(picked) => picked,
            Err(why) => {
                self.issue(PINS_MISMATCH, Severity::Error, format!("{refdes}: {why}"));
                return;
            }
        };
        let used_places: BTreeSet<usize> = picked.iter().flatten().copied().collect();
        let mut nodes: Vec<String> = Vec::new();
        for (place, (pin_id, pin)) in pins.iter().enumerate() {
            let key = NetPin {
                part: id,
                pin: **pin_id,
            };
            if let Some(node) = self.wiring.pin_node.get(&key) {
                nodes.push(node.clone());
                continue;
            }
            let node = format!("nc_{}_{}", clean(&refdes), clean(&pin.number));
            if used_places.contains(&place) {
                self.not_connected.insert(node.clone());
                self.report.issues.push(GenIssue::new(
                    PIN_ON_NO_NET,
                    Severity::Warning,
                    format!("{refdes} pin {}", pin.number.trim()),
                ));
            }
            nodes.push(node);
        }
        let roles: Vec<Option<String>> = picked
            .iter()
            .map(|place| (*place).map(|i| nodes[i].clone()))
            .collect();

        if rule.device == Device::Voltmeter {
            let plus = roles.first().cloned().flatten().unwrap_or_default();
            let minus = roles.get(1).cloned().flatten().unwrap_or_default();
            self.report.meters.push(Meter {
                refdes: refdes.clone(),
                plus,
                minus,
            });
            self.report.translated.push(refdes);
            return;
        }

        let name = element_name(rule.device.letter(), &refdes);
        let key = name.to_ascii_lowercase();
        if self.names.contains(&key) {
            self.issue(
                REFDES_TWICE,
                Severity::Error,
                format!("{refdes} gives the element {name}, which another part has already"),
            );
            return;
        }
        let Some(line) = self.line_for(&refdes, &name, &rule, &roles, &value) else {
            return;
        };
        let element = match parse_element_line(&line, 0, &BTreeMap::new(), self.settings) {
            Ok(element) => element,
            Err(e) => {
                self.bad(&refdes, &e.detail);
                return;
            }
        };

        let is_voltage = matches!(rule.device, Device::VSource | Device::Ammeter);
        if is_voltage {
            let (a, b) = (element.nodes[0].clone(), element.nodes[1].clone());
            if a == b {
                let shown = self.wiring.shown.get(&a).cloned().unwrap_or(a);
                self.issue(
                    SOURCE_SHORTED,
                    Severity::Error,
                    format!("{refdes}: both terminals are on the net {shown}"),
                );
                return;
            }
            let pair = if a < b { (a, b) } else { (b, a) };
            if let Some(other) = self.pairs.insert(pair, refdes.clone()) {
                self.issue(
                    SOURCE_SHORTED,
                    Severity::Error,
                    format!("{refdes} and {other} are voltage sources in parallel"),
                );
            }
            self.report.currents.push(SourceRef {
                refdes: refdes.clone(),
                element: key.clone(),
            });
        }
        for node in &element.nodes {
            self.used.insert(node.clone());
            *self.uses.entry(node.clone()).or_insert(0) += 1;
        }
        self.names.insert(key);
        self.lines.push(line);
        self.report.translated.push(refdes);
    }

    fn finish(mut self) -> NetlistText {
        if !self.used.contains("0") {
            let detail = format!(
                "no translated element is joined to a ground net (nets named {})",
                self.settings.schematic_ground.join(", ")
            );
            self.issue(NO_GROUND_NET, Severity::Error, detail);
        }
        let floating: Vec<String> = self
            .uses
            .iter()
            .filter(|(node, count)| {
                **count == 1 && node.as_str() != "0" && !self.not_connected.contains(*node)
            })
            .map(|(node, _)| node.clone())
            .collect();
        for node in floating {
            let shown = self.wiring.shown.get(&node).cloned().unwrap_or(node);
            self.issue(
                NET_FLOATING,
                Severity::Warning,
                format!("the net {shown} is joined to one pin only"),
            );
        }
        self.report.node_names = self
            .used
            .iter()
            .map(|node| NodeName {
                node: node.clone(),
                net: self
                    .wiring
                    .shown
                    .get(node)
                    .cloned()
                    .unwrap_or_else(|| node.clone()),
            })
            .collect();
        self.report.nodes = self.used.len();
        self.report.grounds = std::mem::take(&mut self.wiring.grounds);
        self.report.includes = self.includes.iter().cloned().collect();

        let mut text = title_line(&self.schematic.name);
        text.push('\n');
        for file in &self.includes {
            let _ = writeln!(text, ".include {file}");
        }
        for line in self.models.values() {
            text.push_str(line);
            text.push('\n');
        }
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str(".end\n");
        NetlistText {
            text,
            report: self.report,
        }
    }
}

/// Makes the netlist of a schematic. `props` are attributes of parts that the caller holds
/// (reference designator in lower case, then the attribute name in capitals); they win over the
/// `attr` lines of the table, which win over its rules.
pub fn generate_netlist(
    schematic: &Schematic,
    table: &ModelTable,
    props: Option<&PartProps>,
    settings: &SimSettings,
) -> NetlistText {
    generate_with(schematic, table, props, settings, None)
}

/// Makes the netlist of a schematic that may have sheet symbols: the sheets are drawn out first,
/// so a sheet shown twice gives its parts twice, with references such as `U1/R1` and nets such as
/// `/U1/VOUT`; the rules of the table look at the reference on the sheet (`R1`). A schematic
/// without sheet symbols gives the same netlist as [`generate_netlist`]. The error is a refusal to
/// expand (the supply of ids ran out).
pub fn generate_netlist_hier(
    schematic: &Schematic,
    hier: &HierConfig,
    table: &ModelTable,
    props: Option<&PartProps>,
    settings: &SimSettings,
) -> Result<NetlistText, EditError> {
    if !schematic.has_hierarchy() {
        return Ok(generate_netlist(schematic, table, props, settings));
    }
    let expanded = schematic.expand(hier)?;
    Ok(generate_with(
        &expanded.schematic,
        table,
        props,
        settings,
        Some(&expanded.designators),
    ))
}

fn generate_with(
    schematic: &Schematic,
    table: &ModelTable,
    props: Option<&PartProps>,
    settings: &SimSettings,
    local: Option<&BTreeMap<String, String>>,
) -> NetlistText {
    let mut builder = Builder {
        schematic,
        table,
        props,
        local,
        settings,
        wiring: wire(schematic, settings),
        report: NetlistReport::default(),
        lines: Vec::new(),
        models: BTreeMap::new(),
        model_types: BTreeMap::new(),
        includes: BTreeSet::new(),
        names: BTreeSet::new(),
        used: BTreeSet::new(),
        uses: BTreeMap::new(),
        not_connected: BTreeSet::new(),
        pairs: BTreeMap::new(),
    };
    let mut parts: Vec<(&Id, &Part)> = schematic.parts.iter().collect();
    parts.sort_by_key(|(id, part)| (part.refdes.to_ascii_lowercase(), **id));
    for (id, part) in parts {
        builder.part(*id, part);
    }
    builder.finish()
}

/// Reads the generated text as a netlist; the `.include` files are looked for in `folder`.
pub fn read_generated(
    generated: &NetlistText,
    folder: Option<&Path>,
    settings: &SimSettings,
) -> Result<Netlist, SimError> {
    read_netlist_text(&generated.text, folder, settings)
}

/// The voltage of a net.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeVoltage {
    /// The name of the net in the schematic.
    pub net: String,
    /// The node in the netlist.
    pub node: String,
    /// The voltage in volts.
    pub volts: f64,
}

/// The current through a source.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceCurrent {
    /// The reference designator of the source.
    pub refdes: String,
    /// The current in amperes, from the plus pin through the source to the minus pin (negative
    /// when the source delivers power).
    pub amps: f64,
}

/// The reading of a voltmeter.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterReading {
    /// The reference designator of the voltmeter.
    pub refdes: String,
    /// The voltage in volts.
    pub volts: f64,
}

/// The operating point of a schematic, net by net: data for a window or a table, no drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct OpTable {
    /// The voltage of each net, ground first.
    pub voltages: Vec<NodeVoltage>,
    /// The current of each voltage source and ammeter.
    pub currents: Vec<SourceCurrent>,
    /// The voltmeters.
    pub meters: Vec<MeterReading>,
    /// The iterations the solver used.
    pub iterations: usize,
    /// How the point was found.
    pub strategy: &'static str,
}

impl OpTable {
    /// The table as text, one line for each net, source and meter.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for v in &self.voltages {
            let _ = writeln!(out, "v({}) = {} V", v.net, format_number(v.volts));
        }
        for c in &self.currents {
            let _ = writeln!(out, "i({}) = {} A", c.refdes, format_number(c.amps));
        }
        for m in &self.meters {
            let _ = writeln!(out, "{} = {} V", m.refdes, format_number(m.volts));
        }
        out
    }
}

/// Solves the operating point of a generated netlist and reads the result net by net. The
/// `.include` files are looked for in `folder`.
pub fn operating_table(
    generated: &NetlistText,
    folder: Option<&Path>,
    settings: &SimSettings,
) -> Result<OpTable, SimError> {
    let netlist = read_generated(generated, folder, settings)?;
    let circuit = Circuit::compile(&netlist, settings)?;
    let op = operating_point(&circuit, settings)?;
    let volts_of = |node: &str| -> Option<f64> {
        if node == "0" {
            Some(0.0)
        } else {
            op.get(&format!("v({node})"))
        }
    };
    let mut voltages: Vec<NodeVoltage> = Vec::new();
    for name in &generated.report.node_names {
        if let Some(volts) = volts_of(&name.node) {
            voltages.push(NodeVoltage {
                net: name.net.clone(),
                node: name.node.clone(),
                volts,
            });
        }
    }
    voltages.sort_by(|a, b| {
        (a.node != "0", a.net.to_ascii_lowercase())
            .cmp(&(b.node != "0", b.net.to_ascii_lowercase()))
    });
    let currents: Vec<SourceCurrent> = generated
        .report
        .currents
        .iter()
        .filter_map(|s| {
            op.get(&format!("i({})", s.element))
                .map(|amps| SourceCurrent {
                    refdes: s.refdes.clone(),
                    amps,
                })
        })
        .collect();
    let meters: Vec<MeterReading> = generated
        .report
        .meters
        .iter()
        .filter_map(|m| {
            let (plus, minus) = (volts_of(&m.plus)?, volts_of(&m.minus)?);
            Some(MeterReading {
                refdes: m.refdes.clone(),
                volts: plus - minus,
            })
        })
        .collect();
    Ok(OpTable {
        voltages,
        currents,
        meters,
        iterations: op.iterations,
        strategy: op.strategy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_container::ObjectId;
    use eh3d_phoenix_schematic::Symbol;
    use std::fs;
    use std::path::PathBuf;

    /// A schematic built from short descriptions, with ids that start at `base`.
    struct Builder2 {
        s: Schematic,
        next: u16,
        pins: BTreeMap<(Id, String), Id>,
    }

    impl Builder2 {
        fn new(base: u16) -> Builder2 {
            Builder2 {
                s: Schematic {
                    name: "test".into(),
                    ..Schematic::default()
                },
                next: base,
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

    /// A source of `volts` across two resistors; the middle net is called `middle`.
    fn divider(base: u16, volts: &str, r1: &str, r2: &str, middle: &str) -> Schematic {
        let mut b = Builder2::new(base);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", volts);
        let a = b.part(two, "R1", r1);
        let c = b.part(two, "R2", r2);
        b.net("IN", &[(v1, two, "1"), (a, two, "1")]);
        b.net(middle, &[(a, two, "2"), (c, two, "1")]);
        b.net("GND", &[(v1, two, "2"), (c, two, "2")]);
        b.s
    }

    fn make(s: &Schematic, table: &str) -> NetlistText {
        let table = ModelTable::parse(table).unwrap();
        generate_netlist(s, &table, None, &SimSettings::default())
    }

    fn built_in(s: &Schematic) -> NetlistText {
        generate_netlist(s, &ModelTable::built_in(), None, &SimSettings::default())
    }

    fn volts(t: &OpTable, net: &str) -> f64 {
        t.voltages
            .iter()
            .find(|v| v.net == net)
            .unwrap_or_else(|| panic!("no net {net} in {t:?}"))
            .volts
    }

    fn table_of(g: &NetlistText) -> OpTable {
        operating_table(g, None, &SimSettings::default()).unwrap()
    }

    #[test]
    fn a_divider_is_simulated_from_the_schematic_with_the_built_in_table() {
        let g = built_in(&divider(0, "10", "1k", "3k", "OUT"));
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        assert_eq!(g.report.translated, vec!["R1", "R2", "V1"]);
        assert_eq!(g.report.grounds, vec!["GND"]);
        assert_eq!(g.report.nodes, 3);
        assert!(g.text.starts_with("test\n"));
        assert!(g.text.ends_with(".end\n"));
        let t = table_of(&g);
        assert!((volts(&t, "OUT") - 7.5).abs() < 1e-9);
        assert!((volts(&t, "IN") - 10.0).abs() < 1e-9);
        assert_eq!(volts(&t, "GND"), 0.0);
        assert_eq!(t.voltages[0].net, "GND");
        assert_eq!(t.currents.len(), 1);
        assert_eq!(t.currents[0].refdes, "V1");
        assert!(
            (t.currents[0].amps + 2.5e-3).abs() < 1e-9,
            "{:?}",
            t.currents
        );
        assert!(t.to_text().contains("v(OUT) = "));
    }

    #[test]
    fn values_are_read_with_the_notation_of_the_markings() {
        let g = built_in(&divider(0, "10", "2k2", "6k8", "OUT"));
        let t = table_of(&g);
        assert!((volts(&t, "OUT") - 10.0 * 6800.0 / 9000.0).abs() < 1e-9);
        let bad = built_in(&divider(0, "10", "abc", "3k", "OUT"));
        assert!(bad.report.has_errors());
        assert!(bad
            .report
            .issues
            .iter()
            .any(|i| i.code == VALUE_BAD && i.detail.contains("R1")));
        assert!(!bad.report.translated.contains(&"R1".to_string()));
        let none = built_in(&divider(0, "10", "", "3k", "OUT"));
        assert!(none
            .report
            .issues
            .iter()
            .any(|i| i.code == VALUE_BAD && i.detail.contains("no value")));
    }

    #[test]
    fn a_net_with_no_name_gets_a_name_that_does_not_change_with_other_edits() {
        let a = built_in(&divider(0, "10", "1k", "3k", ""));
        let b = built_in(&divider(500, "10", "1k", "3k", ""));
        let node = |g: &NetlistText| {
            g.report
                .node_names
                .iter()
                .find(|n| n.node.starts_with("n_"))
                .map(|n| n.node.clone())
        };
        assert_eq!(node(&a).as_deref(), Some("n_r1_2"));
        assert_eq!(node(&a), node(&b));
        assert_eq!(a.text, b.text);
        // the same net with a name of its own
        assert!(built_in(&divider(0, "10", "1k", "3k", "+5 V"))
            .text
            .contains(" _5_v "));
    }

    #[test]
    fn what_cannot_be_simulated_is_reported_with_a_code_and_a_severity() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let chip = b.symbol("chip", &["1", "2", "3", "4", "5"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "1k");
        let _j1 = b.part(chip, "J1", "header");
        let u1 = b.part(chip, "U1", "LM358");
        b.part(two, "R1", "2k");
        b.net("IN", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("MID", &[(r1, two, "2"), (r2, two, "1")]);
        b.net("GND", &[(v1, two, "2")]);
        b.net("LONELY", &[(u1, chip, "1")]);
        let mut table = String::from("refdes R = resistor\nrefdes V = vsource\nrefdes U = opamp\n");
        table.push_str(".model generic OPAMP\n");
        let g = make(&b.s, &table);
        let codes: Vec<&str> = g.report.issues.iter().map(|i| i.code).collect();
        // J1 has no rule
        assert!(codes.contains(&NO_MODEL), "{codes:?}");
        // the second R1 gives the same element
        assert!(codes.contains(&REFDES_TWICE), "{codes:?}");
        // U1 has five pins that are not named for an op-amp
        assert!(codes.contains(&PINS_MISMATCH), "{codes:?}");
        // R2.2 is on no net
        assert!(codes.contains(&PIN_ON_NO_NET), "{codes:?}");
        assert!(g.report.has_errors());
        assert!(!g.report.is_clean());
        assert!(g.report.to_text(Lang::Es).contains("PHOENIX-NETLIST-002"));
        assert!(g
            .report
            .to_text(Lang::Pt)
            .contains("A peça não tem modelo de simulação"));
    }

    #[test]
    fn a_circuit_without_ground_a_single_connection_and_a_shorted_source_are_found() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let v2 = b.part(two, "V2", "3");
        let r1 = b.part(two, "R1", "1k");
        b.net("A", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("B", &[(v1, two, "2"), (r1, two, "2")]);
        b.net("C", &[(v2, two, "1"), (v2, two, "2")]);
        let g = built_in(&b.s);
        let codes: Vec<&str> = g.report.issues.iter().map(|i| i.code).collect();
        assert!(codes.contains(&NO_GROUND_NET), "{codes:?}");
        assert!(codes.contains(&SOURCE_SHORTED), "{codes:?}");
        assert!(!g.report.translated.contains(&"V2".to_string()));

        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let v2 = b.part(two, "V2", "5");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "1k");
        b.net("IN", &[(v1, two, "1"), (v2, two, "1"), (r1, two, "1")]);
        b.net("GND", &[(v1, two, "2"), (v2, two, "2"), (r1, two, "2")]);
        b.net("DANGLING", &[(r2, two, "1")]);
        let g = built_in(&b.s);
        let parallel = g
            .report
            .issues
            .iter()
            .filter(|i| i.code == SOURCE_SHORTED)
            .count();
        assert_eq!(parallel, 1);
        assert!(g
            .report
            .issues
            .iter()
            .any(|i| i.code == NET_FLOATING && i.detail.contains("DANGLING")));
    }

    #[test]
    fn attributes_win_over_the_rules_and_the_table_can_carry_them() {
        let s = divider(0, "10", "1k", "3k", "OUT");
        // the table changes the value of R1; the caller changes the device of R2
        let table = ModelTable::parse(
            "refdes R = resistor\nrefdes V = vsource\nattr R1 SIM.VALUE = 1k\nattr R2 SIM.VALUE = 9k\n",
        )
        .unwrap();
        let mut props = PartProps::new();
        props
            .entry("r2".to_string())
            .or_default()
            .insert("sim.value".to_string(), "2k".to_string());
        let g = generate_netlist(&s, &table, Some(&props), &SimSettings::default());
        let t = table_of(&g);
        assert!((volts(&t, "OUT") - 10.0 * 2000.0 / 3000.0).abs() < 1e-9);
        // a device that is not one is refused
        let table = ModelTable::parse("refdes R = resistor\nattr R1 SIM.DEVICE = flux\n").unwrap();
        let g = generate_netlist(&s, &table, None, &SimSettings::default());
        assert!(g.report.issues.iter().any(|i| i.code == VALUE_BAD));
        // a capacitor instead of a resistor
        let table = ModelTable::parse(
            "refdes R = resistor\nrefdes V = vsource\nattr R2 SIM.DEVICE = capacitor\n",
        )
        .unwrap();
        let g = generate_netlist(&s, &table, None, &SimSettings::default());
        assert!(g.text.contains("C_R2 "), "{}", g.text);
    }

    #[test]
    fn a_led_gets_the_generic_model_and_says_so() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let led = b.symbol("led", &["K", "A"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "330");
        let d1 = b.part(led, "D1", "RED");
        b.net("IN", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("D", &[(r1, two, "2"), (d1, led, "A")]);
        b.net("GND", &[(v1, two, "2"), (d1, led, "K")]);
        let g = make(
            &b.s,
            "refdes R = resistor\nrefdes V = vsource\nrefdes D = led\n",
        );
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        assert!(g.report.issues.iter().any(|i| i.severity == Severity::Info));
        assert!(g.text.contains(".model red LED"), "{}", g.text);
        let t = table_of(&g);
        assert!((volts(&t, "D") - 1.7853305611484638).abs() < 1e-4);
    }

    #[test]
    fn an_nmos_inverter_and_an_inverting_amplifier_are_simulated() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let nmos = b.symbol("nmos", &["D", "G", "S"]);
        let vdd = b.part(two, "V1", "5");
        let vin = b.part(two, "V2", "2.5");
        let rd = b.part(two, "R1", "1k");
        let m1 = b.part(nmos, "M1", "NM");
        b.net("VDD", &[(vdd, two, "1"), (rd, two, "1")]);
        b.net("IN", &[(vin, two, "1"), (m1, nmos, "G")]);
        b.net("OUT", &[(rd, two, "2"), (m1, nmos, "D")]);
        b.net("GND", &[(vdd, two, "2"), (vin, two, "2"), (m1, nmos, "S")]);
        let g = make(
            &b.s,
            "refdes R = resistor\nrefdes V = vsource\nrefdes M = nmos model=nm\n.model nm NMOS(VTO=1 KP=2e-5 W=1e-3 L=1e-5)\n",
        );
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        assert!((volts(&table_of(&g), "OUT") - 2.75).abs() < 1e-4);

        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let amp = b.symbol("opamp", &["+", "-", "OUT"]);
        let vin = b.part(two, "V1", "1");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "2k");
        let u1 = b.part(amp, "U1", "OA");
        b.net("IN", &[(vin, two, "1"), (r1, two, "1")]);
        b.net("INN", &[(r1, two, "2"), (r2, two, "1"), (u1, amp, "-")]);
        b.net("OUT", &[(r2, two, "2"), (u1, amp, "OUT")]);
        b.net("GND", &[(vin, two, "2"), (u1, amp, "+")]);
        let g = make(
            &b.s,
            "refdes R = resistor\nrefdes V = vsource\nrefdes U = opamp model=oa\n.model oa OPAMP(GAIN=1e6 GBW=0 ROUT=1)\n",
        );
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        assert!((volts(&table_of(&g), "OUT") + 2.0).abs() < 2e-5);
    }

    #[test]
    fn a_model_that_is_named_and_not_defined_is_an_error_and_a_zener_needs_one() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let r1 = b.part(two, "R1", "1k");
        let d1 = b.part(two, "D1", "1N4148");
        let z1 = b.part(two, "DZ1", "5V1");
        b.net("IN", &[(v1, two, "1"), (r1, two, "1")]);
        b.net("A", &[(r1, two, "2"), (d1, two, "1"), (z1, two, "1")]);
        b.net("GND", &[(v1, two, "2"), (d1, two, "2"), (z1, two, "2")]);
        let g = make(
            &b.s,
            "refdes R = resistor\nrefdes V = vsource\nrefdes D = diode model=missing\nrefdes DZ = zener\n",
        );
        let undefined = g
            .report
            .issues
            .iter()
            .filter(|i| i.code == MODEL_UNDEFINED)
            .count();
        assert_eq!(undefined, 2, "{:?}", g.report.issues);
    }

    #[test]
    fn a_subcircuit_comes_from_a_file_in_the_folder() {
        let dir = std::env::temp_dir().join(format!("phoenix-sim-schgen-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let lib: PathBuf = dir.join("half.lib");
        fs::write(
            &lib,
            "* a resistor of 1k\n.subckt half a b\nR1 a b 1k\n.ends half\n",
        )
        .unwrap();
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "5");
        let x1 = b.part(two, "X1", "HALF");
        b.net("IN", &[(v1, two, "1"), (x1, two, "1")]);
        b.net("GND", &[(v1, two, "2"), (x1, two, "2")]);
        let g = make(
            &b.s,
            "refdes V = vsource\nvalue HALF = subckt file=half.lib subckt=half\n",
        );
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        assert_eq!(g.report.includes, vec!["half.lib"]);
        assert!(g.text.contains(".include half.lib"));
        let t = operating_table(&g, Some(&dir), &SimSettings::default()).unwrap();
        assert!((t.currents[0].amps + 5e-3).abs() < 1e-9, "{:?}", t.currents);
        // without a folder an include is refused
        assert!(operating_table(&g, None, &SimSettings::default()).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_ammeter_and_a_voltmeter_read_a_branch_and_a_pair_of_nets() {
        let mut b = Builder2::new(0);
        let two = b.symbol("two-pin", &["1", "2"]);
        let v1 = b.part(two, "V1", "10");
        let am = b.part(two, "AM1", "");
        let r1 = b.part(two, "R1", "1k");
        let r2 = b.part(two, "R2", "1k");
        let vm = b.part(two, "VM1", "");
        b.net("IN", &[(v1, two, "1"), (am, two, "1")]);
        b.net("A", &[(am, two, "2"), (r1, two, "1"), (vm, two, "1")]);
        b.net("B", &[(r1, two, "2"), (r2, two, "1"), (vm, two, "2")]);
        b.net("GND", &[(v1, two, "2"), (r2, two, "2")]);
        let g = built_in(&b.s);
        assert!(g.report.is_clean(), "{:?}", g.report.issues);
        let t = table_of(&g);
        let amps = t.currents.iter().find(|c| c.refdes == "AM1").unwrap().amps;
        // the current enters the plus pin of the ammeter, so it is positive
        assert!((amps - 5e-3).abs() < 1e-9, "{amps}");
        assert!((t.meters[0].volts - 5.0).abs() < 1e-9, "{:?}", t.meters);
    }

    #[test]
    fn pins_go_to_the_roles_by_name_by_order_or_by_the_list_of_the_rule() {
        let n = |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_string()).collect() };
        // by name: the pins come in any order
        let got = assign_pins(Device::Npn, &n(&["B", "C", "E"]), &[]).unwrap();
        assert_eq!(got, vec![Some(1), Some(0), Some(2)]);
        // by order
        let got = assign_pins(Device::Npn, &n(&["1", "2", "3"]), &[]).unwrap();
        assert_eq!(got, vec![Some(0), Some(1), Some(2)]);
        // a diode with K and A
        let got = assign_pins(Device::Diode, &n(&["A", "K"]), &[]).unwrap();
        assert_eq!(got, vec![Some(0), Some(1)]);
        // the list of the rule
        let got = assign_pins(Device::Diode, &n(&["1", "2"]), &n(&["2", "1"])).unwrap();
        assert_eq!(got, vec![Some(1), Some(0)]);
        // a MOSFET without a bulk pin
        let got = assign_pins(Device::Nmos, &n(&["D", "G", "S"]), &[]).unwrap();
        assert_eq!(got, vec![Some(0), Some(1), Some(2), None]);
        // a subcircuit takes all the pins in order, or in the order listed
        assert_eq!(
            assign_pins(Device::Subckt, &n(&["1", "2", "3"]), &[]).unwrap(),
            vec![Some(0), Some(1), Some(2)]
        );
        assert_eq!(
            assign_pins(Device::Subckt, &n(&["1", "2", "3"]), &n(&["3", "1"])).unwrap(),
            vec![Some(2), Some(0)]
        );
        // what does not fit
        assert!(assign_pins(Device::OpAmp, &n(&["1", "2"]), &[]).is_err());
        assert!(assign_pins(Device::OpAmp, &n(&["1", "2", "3", "4", "5"]), &[]).is_err());
        assert!(assign_pins(Device::Npn, &n(&["1", "2", "3"]), &n(&["1", "1", "2"])).is_err());
        assert!(assign_pins(Device::Npn, &n(&["1", "2", "3"]), &n(&["1", "9", "2"])).is_err());
        assert!(assign_pins(Device::Npn, &n(&["1", "2", "3"]), &n(&["1"])).is_err());
    }

    #[test]
    fn element_names_keep_the_letter_of_the_kind() {
        assert_eq!(element_name('R', "R1"), "R1");
        assert_eq!(element_name('R', "r7"), "r7");
        assert_eq!(element_name('D', "LED1"), "D_LED1");
        assert_eq!(element_name('V', "Vcc 1"), "Vcc_1");
        assert_eq!(element_name('R', "R"), "R_R");
    }
}
