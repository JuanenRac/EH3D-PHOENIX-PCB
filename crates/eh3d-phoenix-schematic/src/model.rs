//! The schematic model and its own consistency checks (`PHOENIX-SCH-<NNN>`).

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::draw::{Placement, Sheet, Wire};
use crate::marks::{Junction, NetLabel, PowerPort};

/// The identity of any part of the schematic: the shared object identity of the suite.
pub type Id = eh3d_container::ObjectId;

/// A pin of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    /// The number or name printed on the symbol (`1`, `A3`); it is the number of the pad that the
    /// pin goes to.
    pub number: String,
    /// Where the pin is, relative to the origin of the symbol. Not written when it is not given;
    /// a pin without a position cannot be judged against the wires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Point>,
}

/// The electrical type of a pin, which the electrical rule check judges the connections by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinKind {
    /// Takes a signal in.
    Input,
    /// Drives a signal out.
    Output,
    /// Drives and takes a signal.
    Bidirectional,
    /// An output that can be switched off (a bus driver).
    TriState,
    /// A pin of a passive part (a resistor, a capacitor): it neither drives nor needs a driver.
    Passive,
    /// A supply pin that needs a source.
    PowerIn,
    /// A pin that supplies a net (a regulator output).
    PowerOut,
    /// An open collector or open drain output.
    OpenCollector,
    /// A pin that is meant to stay unconnected.
    NotConnected,
}

impl PinKind {
    /// Every kind, in a fixed order; the position of a kind in this list is its index in the
    /// matrix of the check.
    pub const ALL: [PinKind; 9] = [
        PinKind::Input,
        PinKind::Output,
        PinKind::Bidirectional,
        PinKind::TriState,
        PinKind::Passive,
        PinKind::PowerIn,
        PinKind::PowerOut,
        PinKind::OpenCollector,
        PinKind::NotConnected,
    ];

    /// The name used in files (`power_in`).
    pub fn as_str(self) -> &'static str {
        match self {
            PinKind::Input => "input",
            PinKind::Output => "output",
            PinKind::Bidirectional => "bidirectional",
            PinKind::TriState => "tri_state",
            PinKind::Passive => "passive",
            PinKind::PowerIn => "power_in",
            PinKind::PowerOut => "power_out",
            PinKind::OpenCollector => "open_collector",
            PinKind::NotConnected => "not_connected",
        }
    }

    /// Reads the name used in files.
    pub fn from_name(name: &str) -> Option<PinKind> {
        PinKind::ALL.into_iter().find(|k| k.as_str() == name)
    }

    /// The position of this kind in [`PinKind::ALL`].
    pub fn index(self) -> usize {
        PinKind::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }
}

/// A symbol: a named set of pins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    /// The name.
    pub name: String,
    /// The pins, by id.
    pub pins: BTreeMap<Id, Pin>,
}

/// A placed symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// The reference designator (`R1`).
    pub refdes: String,
    /// The value (`10k`).
    pub value: String,
    /// The symbol it is an instance of.
    pub symbol: Id,
    /// The footprint it is meant to use, when it is chosen. It is the id of a footprint of the
    /// board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<Id>,
    /// Where it is drawn, when it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
}

/// A pin of a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NetPin {
    /// The part.
    pub part: Id,
    /// The pin of its symbol.
    pub pin: Id,
}

/// A net: the pins that are connected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicNet {
    /// The name, an attribute: renaming never changes the identity.
    pub name: String,
    /// The pins, sorted.
    pub pins: Vec<NetPin>,
}

/// The whole schematic.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schematic {
    /// The name.
    pub name: String,
    /// The symbols, by id.
    pub symbols: BTreeMap<Id, Symbol>,
    /// The parts, by id.
    pub parts: BTreeMap<Id, Part>,
    /// The nets, by id.
    pub nets: BTreeMap<Id, SchematicNet>,
    /// The sheets of the drawing, by id. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sheets: BTreeMap<Id, Sheet>,
    /// The wires of the drawing, by id. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub wires: BTreeMap<Id, Wire>,
    /// The electrical type of the pins of the symbols, by pin id. A pin that is not listed has no
    /// type given and the checks that need one leave it out and say so. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pin_kinds: BTreeMap<Id, PinKind>,
    /// The net labels of the drawing, by id. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<Id, NetLabel>,
    /// The junction dots of the drawing, by id. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub junctions: BTreeMap<Id, Junction>,
    /// The power ports of the drawing, by id. Not written when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub power_ports: BTreeMap<Id, PowerPort>,
    /// The placed parts drawn mirrored left to right (before the rotation turns them). Not
    /// written when empty.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub mirrored: BTreeSet<Id>,
}

/// A reference to a part, symbol or pin that does not exist.
pub const DANGLING: &str = "PHOENIX-SCH-001";
/// Two parts with the same reference designator.
pub const DUPLICATE_REFDES: &str = "PHOENIX-SCH-002";
/// A pin on more than one net (or twice on one).
pub const PIN_ON_TWO_NETS: &str = "PHOENIX-SCH-003";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (DANGLING, "Reference to a part that does not exist"),
    (DUPLICATE_REFDES, "Duplicate reference designator"),
    (PIN_ON_TWO_NETS, "Pin on more than one net"),
];

fn make(code: &'static str, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, Severity::Error, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(Recoverability::Unrecoverable)
}

impl Schematic {
    /// The canonical bytes of the schematic (`design/schematic.json`).
    pub fn to_bytes(&self) -> Result<Vec<u8>, eh3d_container::CanonicalError> {
        eh3d_container::to_canonical_file_bytes(self)
    }

    /// Reads a schematic. Whether the references hold is a separate check ([`Schematic::validate`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Schematic, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// The number of a pin of a part, when both exist.
    pub fn pin_number(&self, pin: NetPin) -> Option<&str> {
        let part = self.parts.get(&pin.part)?;
        let symbol = self.symbols.get(&part.symbol)?;
        symbol.pins.get(&pin.pin).map(|p| p.number.as_str())
    }

    /// Every problem of the schematic, in a fixed order.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let mut refdes: BTreeMap<&str, &Id> = BTreeMap::new();
        for (id, part) in &self.parts {
            if !self.symbols.contains_key(&part.symbol) {
                out.push(make(
                    DANGLING,
                    format!("part {id} names a symbol that does not exist"),
                ));
            }
            if let Some(first) = refdes.insert(part.refdes.as_str(), id) {
                out.push(make(
                    DUPLICATE_REFDES,
                    format!("{} on parts {first} and {id}", part.refdes),
                ));
            }
        }
        let mut seen: BTreeSet<NetPin> = BTreeSet::new();
        for (id, net) in &self.nets {
            for pin in &net.pins {
                if self.pin_number(*pin).is_none() {
                    out.push(make(
                        DANGLING,
                        format!("net {id} lists a pin that does not exist"),
                    ));
                }
                if !seen.insert(*pin) {
                    out.push(make(
                        PIN_ON_TWO_NETS,
                        format!("net {id} holds a pin that is already on a net"),
                    ));
                }
            }
        }
        out.extend(self.drawn().problems);
        out
    }

    /// True when [`Schematic::validate`] finds no error (warnings and information do not count).
    pub fn is_consistent(&self) -> bool {
        self.validate()
            .iter()
            .all(|d| d.severity != Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{id, schematic as sample};

    fn codes(s: &Schematic) -> Vec<String> {
        s.validate()
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .collect()
    }

    #[test]
    fn a_good_schematic_is_consistent_and_round_trips_byte_for_byte() {
        let (s, _) = sample();
        assert!(s.is_consistent(), "{:?}", s.validate());
        let bytes = s.to_bytes().unwrap();
        let back = Schematic::from_bytes(&bytes).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn each_kind_of_problem_has_its_code() {
        let (s, ids) = sample();
        let mut a = s.clone();
        a.parts.get_mut(&ids[2]).unwrap().symbol = id(99);
        // the part and the pin of that part in the net both lose their symbol
        assert_eq!(codes(&a), vec![DANGLING, DANGLING]);
        let mut b = s.clone();
        b.parts.get_mut(&ids[2]).unwrap().refdes = "R1".into();
        assert_eq!(codes(&b), vec![DUPLICATE_REFDES]);
        let mut c = s.clone();
        c.nets.get_mut(&ids[5]).unwrap().pins.push(NetPin {
            part: ids[1],
            pin: id(99),
        });
        assert_eq!(codes(&c), vec![DANGLING]);
        let mut d = s.clone();
        let again = d.nets[&ids[5]].pins[0];
        d.nets.insert(
            id(50),
            SchematicNet {
                name: "M".into(),
                pins: vec![again],
            },
        );
        assert_eq!(codes(&d), vec![PIN_ON_TWO_NETS]);
    }

    #[test]
    fn pin_kinds_are_written_only_when_given_and_round_trip() {
        let (mut s, ids) = sample();
        let plain = s.to_bytes().unwrap();
        assert!(!String::from_utf8_lossy(&plain).contains("pin_kinds"));
        s.pin_kinds.insert(ids[3], PinKind::PowerIn);
        let bytes = s.to_bytes().unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("power_in"));
        assert_eq!(Schematic::from_bytes(&bytes).unwrap(), s);
        for kind in PinKind::ALL {
            assert_eq!(PinKind::from_name(kind.as_str()), Some(kind));
            assert_eq!(PinKind::ALL[kind.index()], kind);
        }
        assert_eq!(PinKind::from_name("nothing"), None);
    }

    #[test]
    fn damaged_text_is_an_error_and_never_a_panic() {
        let (s, _) = sample();
        let good = s.to_bytes().unwrap();
        for cut in (0..good.len()).step_by(23) {
            let _ = Schematic::from_bytes(&good[..cut]);
        }
        assert!(Schematic::from_bytes(b"{\"name\":1}").is_err());
        assert!(Schematic::from_bytes(b"{\"name\":\"x\",\"extra\":1}").is_err());
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
