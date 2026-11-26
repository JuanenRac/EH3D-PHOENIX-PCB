// SPDX-License-Identifier: GPL-3.0-or-later
//! Turning the numbers of the records into the names of the model: the symbol of a gate, the
//! footprint of a device and the connections between pins and pads.
//!
//! **Connections of version 4 and 5.** The connection records of a device hold one entry for
//! each pad of its footprint, in the order of the pads, in bytes 2 to 23. An entry is zero for
//! a pad that is not connected, else it holds the number of the gate (from 1) in its high bits
//! and the number of the pin of that gate (from 1) in its low bits. The width of an entry (one
//! or two bytes) and the number of low bits depend on how many gates the device set has and on
//! the most pins of a gate ([`entry_form`]).
//!
//! **Connections of version 3.** One group of records for each gate, with one entry of two bytes
//! for each pin, ten in a record from byte 4: the number of the pad (from 1).
//!
//! A device whose entries do not all make sense keeps no connection, and that is noted: a wrong
//! split of the bits must not give wrong connections.

use std::collections::BTreeSet;

use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_libedit::model::{Connect, Device, DeviceSet, Gate, Item, Package, Symbol};

use crate::codes::{self, BinError};
use crate::records::{u16_at, Record};
use crate::report::Report;
use crate::text::{Attribute, Technology};
use crate::walk::{RawDevice, RawSet};

/// The bytes of one entry of a connection record that are not its header.
const ENTRY_BYTES: std::ops::Range<usize> = 2..24;
/// Entries in one record of version 3.
const OLD_ENTRIES: usize = 10;

/// How many bytes an entry of a connection has and how many of its low bits are the pin, for a
/// device set with `gates` gates whose largest gate has `pins` pins (versions 4 and 5).
pub fn entry_form(gates: usize, pins: usize) -> (usize, u32) {
    let bits = |n: usize| usize::BITS - n.leading_zeros();
    let (g, p) = (bits(gates), bits(pins));
    if g.saturating_add(p) <= 8 {
        (1, if g <= 4 { p.max(4) } else { p })
    } else if g <= 4 && p >= 8 {
        (2, 12)
    } else if g <= 8 && p <= 8 {
        (2, 8)
    } else {
        (2, p)
    }
}

/// A connection as the indices of the gate and of the pin of that gate, and of the pad.
type Link = (usize, usize, usize);

/// The connections of a device of version 4 or 5; `None` when an entry makes no sense.
fn links(records: &[Record], pins: &[usize], pads: usize) -> Option<Vec<Link>> {
    let (width, shift) = entry_form(pins.len(), pins.iter().copied().max().unwrap_or(0));
    if shift >= 16 {
        return None;
    }
    let bytes: Vec<u8> = records
        .iter()
        .flat_map(|r| r[ENTRY_BYTES].iter().copied())
        .collect();
    let mut out = Vec::new();
    let mut used = BTreeSet::new();
    for (at, entry) in bytes.chunks_exact(width).enumerate() {
        let value = entry
            .iter()
            .rev()
            .fold(0usize, |v, &b| v << 8 | usize::from(b));
        if value == 0 {
            continue;
        }
        let gate = (value >> shift).checked_sub(1)?;
        let pin = (value & ((1 << shift) - 1)).checked_sub(1)?;
        if at >= pads || pin >= *pins.get(gate)? || !used.insert((gate, pin)) {
            return None;
        }
        out.push((gate, pin, at));
    }
    out.sort_unstable();
    Some(out)
}

/// The connections of a device of version 3; `None` when a record or an entry makes no sense.
fn old_links(records: &[Record], pins: &[usize], pads: usize) -> Option<Vec<Link>> {
    let mut out = Vec::new();
    let mut used = BTreeSet::new();
    let mut records = records.iter();
    for (gate, &count) in pins.iter().enumerate() {
        for group in 0..count.div_ceil(OLD_ENTRIES) {
            let r = records.next()?;
            for slot in 0..OLD_ENTRIES {
                let pin = group * OLD_ENTRIES + slot;
                let pad = usize::from(u16_at(r, 4 + 2 * slot));
                if let Some(pad) = pad.checked_sub(1) {
                    if pin >= count || pad >= pads || !used.insert(pad) {
                        return None;
                    }
                    out.push((gate, pin, pad));
                }
            }
        }
    }
    if records.next().is_some() {
        return None;
    }
    Some(out)
}

fn element(name: &str, attrs: Vec<(String, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs,
        text: String::new(),
        children,
    }
}

fn pin_names(symbol: &Symbol) -> Vec<&str> {
    symbol.pins().map(|(_, p)| p.name.as_str()).collect()
}

fn pad_names(package: &Package) -> Vec<&str> {
    package
        .items
        .iter()
        .filter(|i| matches!(i, Item::Pad(_) | Item::Smd(_)))
        .filter_map(Item::name)
        .collect()
}

fn technology_element(technologies: &[Technology]) -> Element {
    let attribute = |a: &Attribute| {
        let mut attrs = vec![
            ("name".into(), a.name.clone()),
            ("value".into(), a.value.clone()),
        ];
        // the XML leaves the constant ones without the attribute
        if !a.constant {
            attrs.push(("constant".into(), "no".into()));
        }
        element("attribute", attrs, Vec::new())
    };
    let list = technologies
        .iter()
        .map(|t| {
            let attributes = t.attributes.iter().map(attribute).collect();
            element(
                "technology",
                vec![("name".into(), t.name.clone())],
                attributes,
            )
        })
        .collect();
    element("technologies", Vec::new(), list)
}

/// What is left of the work that resolving may do, so that a file whose many devices point to
/// the same huge drawing cannot ask for unbounded time or memory.
#[derive(Debug)]
pub struct Budget(pub usize);

impl Budget {
    fn spend(&mut self, units: usize) -> Result<(), BinError> {
        self.0 = self
            .0
            .checked_sub(units.saturating_add(1))
            .ok_or_else(|| BinError::new(codes::LIMIT, "too much to resolve"))?;
        Ok(())
    }
}

/// What a gate uses: the pins of its symbol.
struct GatePins<'a> {
    name: &'a str,
    pins: Vec<&'a str>,
}

fn device(
    raw: &RawDevice,
    set: &RawSet,
    gates: &[GatePins<'_>],
    packages: &[std::sync::Arc<Package>],
    budget: &mut Budget,
    report: &mut Report,
) -> Result<Device, BinError> {
    // the file writes two quotes for the variant without a name
    let name = if raw.name == "''" {
        String::new()
    } else {
        raw.name.clone()
    };
    let place = set.name.as_str();
    let package = match raw.package.checked_sub(1) {
        None => None,
        Some(at) => {
            let found = packages.get(at);
            if found.is_none() {
                report.note(codes::REFERENCE, "footprint of a device", place);
            }
            found
        }
    };
    let pins: Vec<usize> = gates.iter().map(|g| g.pins.len()).collect();
    let mut connects = Vec::new();
    let any = raw
        .connections
        .iter()
        .any(|r| r[ENTRY_BYTES].iter().any(|&b| b != 0));
    if let Some(package) = package {
        budget.spend(package.items.len() + package.name.len())?;
        let pads = pad_names(package);
        let found = if set.old_connections {
            old_links(&raw.connections, &pins, pads.len())
        } else {
            links(&raw.connections, &pins, pads.len())
        };
        match found {
            Some(list) => {
                for (gate, pin, pad) in list {
                    budget.spend(
                        gates[gate].name.len() + gates[gate].pins[pin].len() + pads[pad].len(),
                    )?;
                    report.count("connection");
                    connects.push(Connect {
                        gate: gates[gate].name.to_owned(),
                        pin: gates[gate].pins[pin].to_owned(),
                        pad: pads[pad].to_owned(),
                        extra: Vec::new(),
                    });
                }
            }
            None => report.note(
                codes::SKIPPED,
                "connections of a device (they do not fit its pins and pads)",
                place,
            ),
        }
    } else if any {
        report.note(
            codes::SKIPPED,
            "connections of a device without a footprint",
            place,
        );
    }
    report.count("device");
    for technology in &raw.technologies {
        report.count("technology");
        for attribute in &technology.attributes {
            report.count("attribute");
            budget.spend(attribute.name.len() + attribute.value.len())?;
        }
    }
    Ok(Device {
        name,
        package: package.map(|p| p.name.clone()),
        connects,
        rest: vec![technology_element(&raw.technologies)],
        extra: Vec::new(),
    })
}

/// The addlevel of a gate as the XML writes it; `None` for the default.
fn addlevel(value: u8) -> Option<&'static str> {
    match value {
        0 => Some("must"),
        1 => Some("can"),
        3 => Some("request"),
        4 => Some("always"),
        _ => None,
    }
}

/// A device set of the model.
pub fn device_set(
    raw: &RawSet,
    symbols: &[std::sync::Arc<Symbol>],
    packages: &[std::sync::Arc<Package>],
    budget: &mut Budget,
    report: &mut Report,
) -> Result<DeviceSet, BinError> {
    let place = raw.name.as_str();
    let mut gates = Vec::new();
    let mut pins = Vec::new();
    for g in &raw.gates {
        let symbol = g.symbol.checked_sub(1).and_then(|at| symbols.get(at));
        if symbol.is_none() {
            report.note(codes::REFERENCE, "symbol of a gate", place);
        }
        let mut extra = Vec::new();
        if let Some(level) = addlevel(g.addlevel) {
            extra.push(("addlevel".to_owned(), level.to_owned()));
        } else if g.addlevel != 2 {
            report.note(codes::VALUE, "unknown addlevel of a gate", place);
        }
        if g.swaplevel != 0 {
            extra.push(("swaplevel".to_owned(), g.swaplevel.to_string()));
        }
        if let Some(s) = symbol {
            budget.spend(s.items.len() + s.name.len())?;
        }
        report.count("gate");
        gates.push(Gate {
            name: g.name.clone(),
            symbol: symbol.map(|s| s.name.clone()).unwrap_or_default(),
            x: i64::from(g.x) * 100,
            y: i64::from(g.y) * 100,
            extra,
        });
        pins.push(GatePins {
            name: &g.name,
            pins: symbol.map(|s| pin_names(s)).unwrap_or_default(),
        });
    }
    let mut devices = Vec::new();
    for d in &raw.devices {
        devices.push(device(d, raw, &pins, packages, budget, report)?);
    }
    let description = if raw.description.is_empty() {
        Vec::new()
    } else {
        report.count("description");
        vec![Element {
            name: "description".into(),
            attrs: Vec::new(),
            text: raw.description.clone(),
            children: Vec::new(),
        }]
    };
    report.count("device set");
    Ok(DeviceSet {
        name: raw.name.clone(),
        prefix: raw.prefix.clone(),
        uservalue: raw.uservalue,
        description,
        gates,
        devices,
        rest: Vec::new(),
        extra: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_width_of_an_entry_follows_the_gates_and_pins() {
        // (gates, most pins) -> (bytes, bits of the pin)
        for (shape, form) in [
            ((1, 2), (1, 4)),
            ((11, 10), (1, 4)),
            ((1, 24), (1, 5)),
            ((1, 52), (1, 6)),
            ((1, 100), (1, 7)),
            ((20, 1), (1, 1)),
            ((1, 200), (2, 12)),
            ((3, 100), (2, 8)),
            ((11, 65), (2, 8)),
            ((40, 200), (2, 8)),
        ] {
            assert_eq!(entry_form(shape.0, shape.1), form, "{shape:?}");
        }
    }

    fn record(entries: &[u8]) -> Record {
        let mut r: Record = [0; 24];
        r[0] = 0x3C;
        r[2..2 + entries.len()].copy_from_slice(entries);
        r
    }

    #[test]
    fn entries_name_the_gate_and_the_pin_of_each_pad() {
        let list = links(&[record(&[0x21, 0, 0x11, 0x12])], &[2, 1], 4).unwrap();
        assert_eq!(list, [(0, 0, 2), (0, 1, 3), (1, 0, 0)]);
    }

    #[test]
    fn entries_that_make_no_sense_give_nothing() {
        // a gate that does not exist, a pin zero, a pad that does not exist, a pin used twice
        for bad in [&[0x31][..], &[0x10], &[0, 0, 0x11], &[0x11, 0x11]] {
            assert_eq!(links(&[record(bad)], &[2, 1], 2), None, "{bad:?}");
        }
    }

    #[test]
    fn version_3_lists_the_pad_of_each_pin_gate_by_gate() {
        let first = record(&[0, 0, 2, 0, 0, 0]);
        let second = record(&[0, 0, 1, 0]);
        let list = old_links(&[first, second], &[2, 1], 2).unwrap();
        assert_eq!(list, [(0, 0, 1), (1, 0, 0)]);
        assert_eq!(old_links(&[first], &[2, 1], 2), None);
        assert_eq!(old_links(&[first, second, second], &[2, 1], 2), None);
        assert_eq!(old_links(&[first, second], &[2, 1], 1), None);
    }
}
