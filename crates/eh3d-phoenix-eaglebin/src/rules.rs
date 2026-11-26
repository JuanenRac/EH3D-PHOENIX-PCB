// SPDX-License-Identifier: GPL-3.0-or-later
//! The design rules and the net classes that a board keeps after its long strings.
//!
//! ```text
//! 10 04 00 20        design rules: u32 size, name, description, layer setup (each ends with
//!   78 56 34 12      a zero), a mark, then 19 integers, 8 real numbers and 14 integers
//! 25 04 00 20        a net class: u32 size, name (zero), u32 number,
//!   21 43 65 87      a mark, width, drill and one (version 4) or eight (version 5) clearances,
//!   ef cd ab 89      a mark
//! ```
//!
//! Lengths are in tenths of a micrometre. Only the values listed here are interpreted.

use std::collections::BTreeSet;

use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_libedit::write::fmt_mm;

use crate::codes;
use crate::report::Report;

const RULES: [u8; 4] = [0x10, 0x04, 0x00, 0x20];
const RULES_VALUES: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
const CLASS: [u8; 4] = [0x25, 0x04, 0x00, 0x20];
const CLASS_OPEN: [u8; 4] = [0x21, 0x43, 0x65, 0x87];
const CLASS_CLOSE: [u8; 4] = [0xEF, 0xCD, 0xAB, 0x89];
/// The numbers of the classes go from 0 to this.
const LAST_CLASS: u32 = 255;
/// The longest length of a rule that is believed: ten metres.
const LONGEST: i32 = 100_000_000;

const CLEARANCES: [&str; 10] = [
    "mdWireWire",
    "mdWirePad",
    "mdWireVia",
    "mdPadPad",
    "mdPadVia",
    "mdViaVia",
    "mdSmdPad",
    "mdSmdVia",
    "mdSmdSmd",
    "mdViaViaSameLayer",
];
const RATIOS: [&str; 8] = [
    "msBlindViaRatio",
    "rvPadTop",
    "rvPadInner",
    "rvPadBottom",
    "rvViaOuter",
    "rvViaInner",
    "rvMicroViaOuter",
    "rvMicroViaInner",
];
const RINGS: [&str; 14] = [
    "rlMinPadTop",
    "rlMaxPadTop",
    "rlMinPadInner",
    "rlMaxPadInner",
    "rlMinPadBottom",
    "rlMaxPadBottom",
    "rlMinViaOuter",
    "rlMaxViaOuter",
    "rlMinViaInner",
    "rlMaxViaInner",
    "rlMinMicroViaOuter",
    "rlMaxMicroViaOuter",
    "rlMinMicroViaInner",
    "rlMaxMicroViaInner",
];

/// The net classes and the design rules, as elements of a board file.
#[derive(Debug, Clone, Default)]
pub struct Design {
    /// The `class` elements; at least the default one.
    pub classes: Vec<Element>,
    /// The `designrules` element, when the file has rules.
    pub rules: Option<Element>,
}

fn element(name: &str, attrs: Vec<(&str, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        text: String::new(),
        children,
    }
}

fn find(bytes: &[u8], mark: [u8; 4], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(4)
        .position(|w| w == mark)
        .map(|at| at + from)
}

fn int(bytes: &[u8], at: usize) -> Option<i32> {
    let b = bytes.get(at..at + 4)?;
    Some(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn real(bytes: &[u8], at: usize) -> Option<f64> {
    let b = bytes.get(at..at + 8)?;
    let mut eight = [0u8; 8];
    eight.copy_from_slice(b);
    Some(f64::from_le_bytes(eight))
}

fn length(units: i32) -> Option<String> {
    (0..=LONGEST)
        .contains(&units)
        .then(|| format!("{}mm", fmt_mm(i64::from(units) * 100)))
}

fn text_of(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| char::from(b))
        .filter(|c| *c >= ' ')
        .collect()
}

fn param(name: &str, value: String) -> Element {
    element(
        "param",
        vec![("name", name.to_owned()), ("value", value)],
        Vec::new(),
    )
}

fn design_rules(tail: &[u8]) -> Option<Element> {
    let start = find(tail, RULES, 0)?;
    let mark = find(tail, RULES_VALUES, start + 8)?;
    let head = tail.get(start + 8..mark)?;
    let parts: Vec<&[u8]> = head.split(|&b| b == 0).filter(|p| !p.is_empty()).collect();
    let name = parts.first().map(|p| text_of(p)).unwrap_or_default();
    let at = mark + 4;
    let doubles = at + 19 * 4;
    let rings = doubles + 8 * 8;
    // the whole block has to be there
    int(tail, rings + 13 * 4)?;
    let mut params = Vec::new();
    if parts.len() > 1 {
        let setup = parts.last().map(|p| text_of(p)).unwrap_or_default();
        if setup.len() < 200 && setup.contains('*') {
            params.push(param("layerSetup", setup));
        }
    }
    let mut add = |name: &str, slot: usize| {
        if let Some(value) = int(tail, at + slot * 4).and_then(length) {
            params.push(param(name, value));
        }
    };
    for (slot, name) in CLEARANCES.iter().enumerate() {
        add(name, slot);
    }
    add("mdCopperDimension", 11);
    add("mdDrill", 13);
    add("msWidth", 16);
    add("msDrill", 17);
    add("msMicroVia", 18);
    if let Some(layers) = int(tail, at + 10 * 4).filter(|n| (0..=16).contains(n)) {
        params.push(param("mnLayersViaInSmd", layers.to_string()));
    }
    for (slot, name) in RATIOS.iter().enumerate() {
        if let Some(value) = real(tail, doubles + slot * 8).filter(|v| (0.0..=100.0).contains(v)) {
            params.push(param(name, value.to_string()));
        }
    }
    for (slot, name) in RINGS.iter().enumerate() {
        if let Some(value) = int(tail, rings + slot * 4).and_then(length) {
            params.push(param(name, value));
        }
    }
    Some(element("designrules", vec![("name", name)], params))
}

fn class_at(tail: &[u8], at: usize) -> Option<(u32, Element)> {
    let size = usize::try_from(int(tail, at + 4)?).ok()?;
    let content = tail.get(at + 8..(at + 8).checked_add(size)?)?;
    let zero = content.iter().position(|&b| b == 0)?;
    let name = text_of(&content[..zero]);
    let number = u32::try_from(int(content, zero + 1)?).ok()?;
    let open = zero + 5;
    if content.get(open..open + 4)? != CLASS_OPEN || name.is_empty() || number > LAST_CLASS {
        return None;
    }
    let close = find(content, CLASS_CLOSE, open + 4)?;
    let values: Vec<i32> = content[open + 4..close]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| i32::from_le_bytes(*b))
        .collect();
    let width = fmt_mm(i64::from(*values.first()?) * 100);
    let drill = fmt_mm(i64::from(*values.get(1)?) * 100);
    // version 4 holds the clearance of the class to itself, version 5 to each of the classes
    let own = values.len() == 3;
    let clearances = values[2..]
        .iter()
        .enumerate()
        .filter(|(_, &v)| v != 0)
        .map(|(k, &v)| {
            let other = if own {
                number.to_string()
            } else {
                k.to_string()
            };
            element(
                "clearance",
                vec![("class", other), ("value", fmt_mm(i64::from(v) * 100))],
                Vec::new(),
            )
        })
        .collect();
    let class = element(
        "class",
        vec![
            ("number", number.to_string()),
            ("name", name),
            ("width", width),
            ("drill", drill),
        ],
        clearances,
    );
    Some((number, class))
}

/// Reads what follows the long strings of a board.
pub fn read(tail: &[u8], report: &mut Report) -> Design {
    let mut design = Design {
        classes: Vec::new(),
        rules: design_rules(tail),
    };
    let mut seen = BTreeSet::new();
    let mut at = 0;
    while let Some(found) = find(tail, CLASS, at) {
        if let Some((number, class)) = class_at(tail, found) {
            if seen.insert(number) {
                design.classes.push(class);
                report.count("class");
            }
        }
        at = found + 4;
    }
    if design.classes.is_empty() {
        design.classes.push(element(
            "class",
            vec![
                ("number", "0".to_owned()),
                ("name", "default".to_owned()),
                ("width", "0".to_owned()),
                ("drill", "0".to_owned()),
            ],
            Vec::new(),
        ));
    }
    if design.rules.is_some() {
        report.count("design rules");
        report.note(
            codes::NOT_READ,
            "design rules other than distances, sizes and rings, and the autorouter",
            "",
        );
    } else {
        // the boards of version 3 hold none in this form: information, not a loss
        report.note(codes::NOT_READ, "no design rules found in the board", "");
    }
    design
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damaged_blocks_give_no_rules_and_the_default_class() {
        for tail in [
            Vec::new(),
            RULES.to_vec(),
            [RULES.to_vec(), RULES_VALUES.to_vec()].concat(),
            [CLASS.to_vec(), vec![0xFF; 12]].concat(),
        ] {
            let mut report = Report::default();
            let design = read(&tail, &mut report);
            assert!(design.rules.is_none());
            assert_eq!(design.classes.len(), 1);
            assert_eq!(report.noted(codes::NOT_READ), 1);
            assert!(!report.has_losses());
        }
    }
}
