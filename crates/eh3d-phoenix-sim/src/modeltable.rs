// SPDX-License-Identifier: GPL-3.0-or-later
//! The table that says which simulation model each part of a schematic gets, read from
//! `config/sim_models.cfg` (which is also the built-in table).
//!
//! The file is made of lines; blank lines and what follows a `#` are ignored:
//!
//! * `upper_m_is_mega = true` — a capital `M` in a value means mega (see `partvalue`);
//! * `refdes R = resistor` — the parts whose reference designator starts with `R` followed by a
//!   digit or a sign, not by a letter (so `R1` and `R_load`, not `RV1`); the longest prefix wins;
//! * `value 2N3904 = npn model=q2n3904` — the parts with that value, as written, in any case; a
//!   `*` at the end stands for any ending;
//! * `symbol LED* = led` — the parts whose symbol has that name, with the same ending;
//! * `attr R7 SIM.VALUE = 4k7` — an attribute of one part, which beats every rule;
//! * `.model name TYPE(params)` — a model, kept as it is and written to the netlist when a part
//!   uses it.
//!
//! After the device come options: `model=NAME`, `file=NAME.lib` (a file in the folder of the
//! netlist that holds the model or the subcircuit), `subckt=NAME`, `pins=A,K` (the pins of the
//! symbol in the order the device takes them), for a MOSFET `w=` and `l=`, and for a resistor, a
//! capacitor or an inductor `tol=5%` (its tolerance in a Monte Carlo study). A value rule beats
//! a symbol rule, which beats a prefix rule. The attributes a part can have are `SIM.DEVICE`,
//! `SIM.VALUE`, `SPICE_MODEL`, `SIM.FILE`, `SIM.PINS` and `SIM.TOL`.

use std::collections::BTreeMap;

use eh3d_phoenix_rules::Severity;

use crate::montecarlo::parse_fraction;
use crate::netcodes::{GenIssue, TABLE_INVALID};
use crate::units::{format_number, parse_number};

/// The built-in table, the same text as `config/sim_models.cfg`.
const BUILT_IN: &str = include_str!("../../../config/sim_models.cfg");

/// The attributes of a part, by reference designator in lower case and then by attribute name in
/// capitals.
pub type PartProps = BTreeMap<String, BTreeMap<String, String>>;

/// The attribute names a part can carry.
pub const ATTRIBUTE_KEYS: [&str; 6] = [
    "SIM.DEVICE",
    "SIM.VALUE",
    "SPICE_MODEL",
    "SIM.FILE",
    "SIM.PINS",
    "SIM.TOL",
];

/// The names a pin can have for a role of a device, in lower case.
type Aliases = &'static [&'static str];

const TWO_PASSIVE: &[Aliases] = &[&["1", "a", "+", "p"], &["2", "b", "-", "n"]];
const TWO_SOURCE: &[Aliases] = &[&["+", "p", "1", "plus"], &["-", "n", "2", "minus"]];
const TWO_DIODE: &[Aliases] = &[&["a", "anode", "+"], &["k", "c", "cathode", "-"]];
const BJT: &[Aliases] = &[&["c", "collector"], &["b", "base"], &["e", "emitter"]];
const MOS: &[Aliases] = &[
    &["d", "drain"],
    &["g", "gate"],
    &["s", "source"],
    &["b", "bulk", "body", "substrate"],
];
const SWITCH: &[Aliases] = &[
    &["1", "a"],
    &["2", "b"],
    &["c+", "cp", "3", "ctrl+"],
    &["c-", "cn", "4", "ctrl-"],
];
const AMP: &[Aliases] = &[
    &["+", "in+", "inp", "ip", "noninv"],
    &["-", "in-", "inn", "inv"],
    &["out", "output", "o"],
];
const NO_ROLES: &[Aliases] = &[];

/// What a part is for the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Device {
    /// `R`.
    Resistor,
    /// `C`.
    Capacitor,
    /// `L`.
    Inductor,
    /// `V`: an independent voltage source.
    VSource,
    /// `I`: an independent current source.
    ISource,
    /// `D` with a diode model.
    Diode,
    /// `D` with an LED model.
    Led,
    /// `D` with a Zener model.
    Zener,
    /// `Q` NPN.
    Npn,
    /// `Q` PNP.
    Pnp,
    /// `M` N-channel.
    Nmos,
    /// `M` P-channel.
    Pmos,
    /// `S`: a switch controlled by a voltage.
    Switch,
    /// `U` with an op-amp model.
    OpAmp,
    /// `U` with a comparator model.
    Comparator,
    /// `X`: an instance of a subcircuit from a file.
    Subckt,
    /// A voltage source of zero volts that lets the current of a branch be read.
    Ammeter,
    /// Reads the voltage between two nets and adds nothing to the circuit.
    Voltmeter,
    /// A part that is not part of the circuit (a test point, a mounting hole).
    Skip,
}

impl Device {
    /// Every device, in a fixed order.
    pub const ALL: [Device; 19] = [
        Device::Resistor,
        Device::Capacitor,
        Device::Inductor,
        Device::VSource,
        Device::ISource,
        Device::Diode,
        Device::Led,
        Device::Zener,
        Device::Npn,
        Device::Pnp,
        Device::Nmos,
        Device::Pmos,
        Device::Switch,
        Device::OpAmp,
        Device::Comparator,
        Device::Subckt,
        Device::Ammeter,
        Device::Voltmeter,
        Device::Skip,
    ];

    /// The name used in the table and in the attribute `SIM.DEVICE`.
    pub fn name(self) -> &'static str {
        match self {
            Device::Resistor => "resistor",
            Device::Capacitor => "capacitor",
            Device::Inductor => "inductor",
            Device::VSource => "vsource",
            Device::ISource => "isource",
            Device::Diode => "diode",
            Device::Led => "led",
            Device::Zener => "zener",
            Device::Npn => "npn",
            Device::Pnp => "pnp",
            Device::Nmos => "nmos",
            Device::Pmos => "pmos",
            Device::Switch => "switch",
            Device::OpAmp => "opamp",
            Device::Comparator => "comparator",
            Device::Subckt => "subckt",
            Device::Ammeter => "ammeter",
            Device::Voltmeter => "voltmeter",
            Device::Skip => "skip",
        }
    }

    /// Reads a name: the full name or the short one (`r`, `c`, `l`, `v`, `i`, `d`, `sw`, `x`,
    /// `u`), in any case.
    pub fn from_name(text: &str) -> Option<Device> {
        let lower = text.trim().to_ascii_lowercase();
        if let Some(found) = Device::ALL.into_iter().find(|d| d.name() == lower) {
            return Some(found);
        }
        match lower.as_str() {
            "r" => Some(Device::Resistor),
            "c" => Some(Device::Capacitor),
            "l" => Some(Device::Inductor),
            "v" => Some(Device::VSource),
            "i" => Some(Device::ISource),
            "d" => Some(Device::Diode),
            "sw" => Some(Device::Switch),
            "x" => Some(Device::Subckt),
            "u" => Some(Device::OpAmp),
            "comp" => Some(Device::Comparator),
            _ => None,
        }
    }

    /// The letter of the element in a netlist; a space for a device that has none.
    pub fn letter(self) -> char {
        match self {
            Device::Resistor => 'R',
            Device::Capacitor => 'C',
            Device::Inductor => 'L',
            Device::VSource | Device::Ammeter => 'V',
            Device::ISource => 'I',
            Device::Diode | Device::Led | Device::Zener => 'D',
            Device::Npn | Device::Pnp => 'Q',
            Device::Nmos | Device::Pmos => 'M',
            Device::Switch => 'S',
            Device::OpAmp | Device::Comparator => 'U',
            Device::Subckt => 'X',
            Device::Voltmeter | Device::Skip => ' ',
        }
    }

    /// The type word of the model this device needs (`NPN`), when it needs one.
    pub fn model_type(self) -> Option<&'static str> {
        match self {
            Device::Diode => Some("D"),
            Device::Led => Some("LED"),
            Device::Zener => Some("ZENER"),
            Device::Npn => Some("NPN"),
            Device::Pnp => Some("PNP"),
            Device::Nmos => Some("NMOS"),
            Device::Pmos => Some("PMOS"),
            Device::Switch => Some("SW"),
            Device::OpAmp => Some("OPAMP"),
            Device::Comparator => Some("COMP"),
            _ => None,
        }
    }

    /// The names a pin can have for each role of the device, in the order the netlist takes the
    /// nodes.
    pub(crate) fn roles(self) -> &'static [Aliases] {
        match self {
            Device::Resistor | Device::Capacitor | Device::Inductor => TWO_PASSIVE,
            Device::VSource | Device::ISource | Device::Ammeter | Device::Voltmeter => TWO_SOURCE,
            Device::Diode | Device::Led | Device::Zener => TWO_DIODE,
            Device::Npn | Device::Pnp => BJT,
            Device::Nmos | Device::Pmos => MOS,
            Device::Switch => SWITCH,
            Device::OpAmp | Device::Comparator => AMP,
            Device::Subckt | Device::Skip => NO_ROLES,
        }
    }

    /// How many of the roles must be found (the bulk of a MOSFET may be left out).
    pub(crate) fn required(self) -> usize {
        match self {
            Device::Nmos | Device::Pmos => 3,
            other => other.roles().len(),
        }
    }
}

/// The model of one kind of part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// What the part is.
    pub device: Device,
    /// The name of the model (`model=`), when it is given.
    pub model: Option<String>,
    /// The file in the folder of the netlist that is included (`file=`).
    pub file: Option<String>,
    /// The name of the subcircuit (`subckt=`).
    pub subckt: Option<String>,
    /// The pins of the symbol in the order the device takes them (`pins=`).
    pub pins: Vec<String>,
    /// The channel width of a MOSFET as a number in a netlist (`w=`).
    pub width: Option<String>,
    /// The channel length of a MOSFET as a number in a netlist (`l=`).
    pub length: Option<String>,
    /// The tolerance of a resistor, a capacitor or an inductor as a fraction in a netlist
    /// (`tol=5%` is `5e-2`).
    pub tolerance: Option<String>,
}

impl Rule {
    /// A rule for a device with no option.
    pub fn new(device: Device) -> Rule {
        Rule {
            device,
            model: None,
            file: None,
            subckt: None,
            pins: Vec::new(),
            width: None,
            length: None,
            tolerance: None,
        }
    }
}

/// A name of a model, a subcircuit or a pin that can be written in a netlist as it is.
pub(crate) fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.$+-".contains(c))
}

/// A file name with no folder, no drive and no `..`, as an `.include` allows.
pub(crate) fn is_plain_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && !name.contains("..")
        && !name.contains(['/', '\\', ':', '\0'])
        && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// The table of models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTable {
    /// A capital `M` in a value means mega.
    pub upper_m_is_mega: bool,
    by_refdes: Vec<(String, Rule)>,
    by_value: Vec<(String, Rule)>,
    by_symbol: Vec<(String, Rule)>,
    models: BTreeMap<String, String>,
    attrs: PartProps,
}

fn matches_pattern(pattern: &str, text: &str, as_prefix: bool) -> bool {
    if as_prefix {
        return text
            .strip_prefix(pattern)
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()));
    }
    match pattern.strip_suffix('*') {
        Some(stem) => text.starts_with(stem),
        None => text == pattern,
    }
}

fn best<'a>(rules: &'a [(String, Rule)], text: &str, as_prefix: bool) -> Option<&'a Rule> {
    if text.is_empty() {
        return None;
    }
    rules
        .iter()
        .filter(|(pattern, _)| matches_pattern(pattern, text, as_prefix))
        .max_by_key(|(pattern, _)| (!pattern.ends_with('*'), pattern.len()))
        .map(|(_, rule)| rule)
}

impl ModelTable {
    /// A table with no rule and no model.
    pub fn empty() -> ModelTable {
        ModelTable {
            upper_m_is_mega: true,
            by_refdes: Vec::new(),
            by_value: Vec::new(),
            by_symbol: Vec::new(),
            models: BTreeMap::new(),
            attrs: BTreeMap::new(),
        }
    }

    /// The table of `config/sim_models.cfg`.
    pub fn built_in() -> ModelTable {
        ModelTable::parse(BUILT_IN).unwrap_or_else(|_| ModelTable::empty())
    }

    /// The number of rules of the three kinds.
    pub fn rule_count(&self) -> usize {
        self.by_refdes.len() + self.by_value.len() + self.by_symbol.len()
    }

    /// The rule for a part: by value, else by symbol name, else by the prefix of the reference
    /// designator.
    pub fn find(&self, refdes: &str, value: &str, symbol: &str) -> Option<&Rule> {
        let value = value.trim().to_ascii_lowercase();
        let symbol = symbol.trim().to_ascii_lowercase();
        let refdes = refdes.trim().to_ascii_lowercase();
        best(&self.by_value, &value, false)
            .or_else(|| best(&self.by_symbol, &symbol, false))
            .or_else(|| best(&self.by_refdes, &refdes, true))
    }

    /// The `.model` line of a model, by its name in lower case.
    pub fn model_line(&self, name: &str) -> Option<&str> {
        self.models
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// The attributes the table gives to a part, by its reference designator.
    pub fn attributes(&self, refdes: &str) -> Option<&BTreeMap<String, String>> {
        self.attrs.get(&refdes.trim().to_ascii_lowercase())
    }

    /// Reads a table.
    pub fn parse(text: &str) -> Result<ModelTable, GenIssue> {
        let mut table = ModelTable::empty();
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let bad = |why: String| {
                GenIssue::new(
                    TABLE_INVALID,
                    Severity::Error,
                    format!("line {number}: {why}"),
                )
            };
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('.') {
                let mut words = line.split_whitespace();
                let directive = words.next().unwrap_or("").to_ascii_lowercase();
                if directive != ".model" {
                    return Err(bad(format!(
                        "only .model lines are allowed, not {directive}"
                    )));
                }
                let name = words
                    .next()
                    .filter(|n| is_plain_name(n))
                    .ok_or_else(|| bad("a .model line needs a name".to_owned()))?;
                if words.next().is_none() {
                    return Err(bad("a .model line needs a type".to_owned()));
                }
                if table
                    .models
                    .insert(name.to_ascii_lowercase(), line.to_owned())
                    .is_some()
                {
                    return Err(bad(format!("the model {name} is defined twice")));
                }
                continue;
            }
            let (left, right) = line
                .split_once('=')
                .ok_or_else(|| bad("not key = value".to_owned()))?;
            let words: Vec<&str> = left.split_whitespace().collect();
            let right = right.trim();
            match (words.len(), words.first().copied()) {
                (1, Some("upper_m_is_mega")) => {
                    table.upper_m_is_mega = match right {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad("upper_m_is_mega is true or false".to_owned())),
                    };
                }
                (2, Some(kind @ ("refdes" | "value" | "symbol"))) => {
                    let rule = match parse_rule(right) {
                        Ok(rule) => rule,
                        Err(why) => return Err(bad(why)),
                    };
                    let pattern = words[1].to_ascii_lowercase();
                    let list = match kind {
                        "refdes" => &mut table.by_refdes,
                        "value" => &mut table.by_value,
                        _ => &mut table.by_symbol,
                    };
                    if list.iter().any(|(p, _)| *p == pattern) {
                        return Err(bad(format!("{kind} {} appears twice", words[1])));
                    }
                    list.push((pattern, rule));
                }
                (3, Some("attr")) => {
                    let key = words[2].to_ascii_uppercase();
                    if !ATTRIBUTE_KEYS.contains(&key.as_str()) {
                        return Err(bad(format!("{} is not an attribute of a part", words[2])));
                    }
                    if right.is_empty() {
                        return Err(bad(format!("{} needs a value", words[2])));
                    }
                    table
                        .attrs
                        .entry(words[1].to_ascii_lowercase())
                        .or_default()
                        .insert(key, right.to_owned());
                }
                _ => return Err(bad(format!("cannot read the line start '{}'", left.trim()))),
            }
        }
        Ok(table)
    }
}

/// Reads what follows the `=` of a rule: the device and its options.
fn parse_rule(text: &str) -> Result<Rule, String> {
    let mut words = text.split_whitespace();
    let device_word = words.next().ok_or("a rule needs a device")?;
    let device = Device::from_name(device_word)
        .ok_or_else(|| format!("{device_word} is not a device (resistor, capacitor, inductor, vsource, isource, diode, led, zener, npn, pnp, nmos, pmos, switch, opamp, comparator, subckt, ammeter, voltmeter, skip)"))?;
    let mut rule = Rule::new(device);
    for word in words {
        let (key, value) = word
            .split_once('=')
            .ok_or_else(|| format!("{word} is not option=value"))?;
        match key.to_ascii_lowercase().as_str() {
            "model" if is_plain_name(value) => rule.model = Some(value.to_owned()),
            "subckt" if is_plain_name(value) => rule.subckt = Some(value.to_owned()),
            "file" if is_plain_file(value) => rule.file = Some(value.to_owned()),
            "pins" => {
                rule.pins = value
                    .split(',')
                    .map(|p| p.trim().to_owned())
                    .filter(|p| !p.is_empty())
                    .collect();
                if rule.pins.is_empty() {
                    return Err("pins= needs at least one pin".to_owned());
                }
            }
            "w" | "l" if matches!(device, Device::Nmos | Device::Pmos) => {
                let number = parse_number(value)
                    .filter(|n| *n > 0.0)
                    .ok_or_else(|| format!("{word}: not a number above zero"))?;
                if key.eq_ignore_ascii_case("w") {
                    rule.width = Some(format_number(number));
                } else {
                    rule.length = Some(format_number(number));
                }
            }
            "tol" if matches!(
                device,
                Device::Resistor | Device::Capacitor | Device::Inductor
            ) =>
            {
                let fraction = parse_fraction(value).ok_or_else(|| {
                    format!("{word}: not a tolerance (such as 5% or 0.05, below 100%)")
                })?;
                rule.tolerance = Some(format_number(fraction));
            }
            "model" | "subckt" => return Err(format!("{word}: not a plain name")),
            "file" => {
                return Err(format!(
                    "{word}: only a file in the folder of the netlist is allowed (no folder, no '..', no drive)"
                ))
            }
            other => return Err(format!("{other} is not an option of this device")),
        }
    }
    if device == Device::Subckt {
        if rule.file.is_none() {
            return Err("a subckt rule needs file=".to_owned());
        }
        if rule.model.is_some() {
            return Err("a subckt rule names its subcircuit with subckt=, not model=".to_owned());
        }
    }
    Ok(rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# a table
upper_m_is_mega = true
refdes R = resistor
refdes RV = skip
refdes LED = led
refdes L = inductor
refdes Q = npn
refdes M = nmos w=10u l=1u
value BC557* = pnp model=bc557
value 74HC04 = subckt file=hc04.lib subckt=HC04 pins=1,2,7,14
symbol Zener* = zener
attr R7 SIM.VALUE = 4k7
.model bc557 PNP(BF=200)
";

    #[test]
    fn a_table_is_read_and_the_most_specific_rule_wins() {
        let t = ModelTable::parse(SAMPLE).unwrap();
        assert_eq!(t.rule_count(), 9);
        assert_eq!(t.find("R1", "10k", "R").unwrap().device, Device::Resistor);
        // RV is longer than R, and R is not followed by a letter in RV1
        assert_eq!(t.find("RV1", "10k", "POT").unwrap().device, Device::Skip);
        assert_eq!(t.find("LED3", "red", "LED").unwrap().device, Device::Led);
        assert_eq!(t.find("L2", "10u", "L").unwrap().device, Device::Inductor);
        // a value rule beats the prefix, in any case, with an ending
        let pnp = t.find("Q4", "bc557b", "Q").unwrap();
        assert_eq!(pnp.device, Device::Pnp);
        assert_eq!(pnp.model.as_deref(), Some("bc557"));
        assert_eq!(t.find("Q5", "2N3904", "Q").unwrap().device, Device::Npn);
        // a symbol rule beats the prefix
        assert_eq!(
            t.find("D1", "5V1", "Zener-SOD").unwrap().device,
            Device::Zener
        );
        assert!(t.find("D1", "1N4148", "D").is_none());
        assert!(t.find("", "", "").is_none());
        let hc = t.find("U3", "74hc04", "U").unwrap();
        assert_eq!(hc.device, Device::Subckt);
        assert_eq!(hc.subckt.as_deref(), Some("HC04"));
        assert_eq!(hc.file.as_deref(), Some("hc04.lib"));
        assert_eq!(hc.pins, vec!["1", "2", "7", "14"]);
        let m = t.find("M1", "x", "M").unwrap();
        assert_eq!(m.device, Device::Nmos);
        assert!(m.width.is_some() && m.length.is_some());
        assert_eq!(t.attributes("r7").unwrap()["SIM.VALUE"], "4k7");
        assert!(t.attributes("r8").is_none());
        assert!(t.model_line("BC557").unwrap().starts_with(".model bc557"));
        assert!(t.model_line("nothing").is_none());
    }

    #[test]
    fn a_line_that_is_wrong_is_refused_with_its_number() {
        let cases = [
            ("refdes R = nothing\n", "line 1"),
            ("\nrefdes R = resistor\nrefdes R = capacitor\n", "line 3"),
            ("refdes R resistor\n", "line 1"),
            ("value X = subckt\n", "line 1"),
            ("value X = subckt file=a/b.lib subckt=A\n", "line 1"),
            ("value X = subckt file=a.lib model=A\n", "line 1"),
            ("refdes M = nmos w=0\n", "line 1"),
            ("refdes R = resistor w=1u\n", "line 1"),
            ("attr R1 COLOUR = red\n", "line 1"),
            ("upper_m_is_mega = maybe\n", "line 1"),
            (".include other.lib\n", "line 1"),
            (".model a D\n.model A D\n", "line 2"),
            (".model\n", "line 1"),
            ("colour = red\n", "line 1"),
        ];
        for (text, line) in cases {
            let error = ModelTable::parse(text).unwrap_err();
            assert_eq!(error.code, TABLE_INVALID, "{text:?}");
            assert!(error.detail.contains(line), "{text:?}: {}", error.detail);
        }
    }

    #[test]
    fn the_built_in_table_is_the_file_of_the_repository_and_reads() {
        let table = ModelTable::parse(BUILT_IN).unwrap();
        assert_eq!(table, ModelTable::built_in());
        assert!(table.rule_count() >= 10);
        assert_eq!(
            table.find("R12", "4k7", "R").unwrap().device,
            Device::Resistor
        );
    }

    #[test]
    fn device_names_have_short_forms_and_every_device_is_known_by_its_name() {
        for device in Device::ALL {
            assert_eq!(Device::from_name(device.name()), Some(device));
        }
        assert_eq!(Device::from_name("R"), Some(Device::Resistor));
        assert_eq!(Device::from_name(" SW "), Some(Device::Switch));
        assert_eq!(Device::from_name("flux"), None);
        assert_eq!(Device::Nmos.required(), 3);
        assert_eq!(Device::Nmos.roles().len(), 4);
        assert_eq!(Device::Npn.model_type(), Some("NPN"));
        assert_eq!(Device::Resistor.model_type(), None);
    }

    #[test]
    fn names_and_files_are_checked() {
        assert!(is_plain_name("bc557_b"));
        assert!(!is_plain_name("a b"));
        assert!(!is_plain_name(""));
        assert!(is_plain_file("hc04.lib"));
        for bad in ["", "..", "a/b", "a\\b", "c:x", "a..b", "a b"] {
            assert!(!is_plain_file(bad), "{bad:?}");
        }
    }
}
