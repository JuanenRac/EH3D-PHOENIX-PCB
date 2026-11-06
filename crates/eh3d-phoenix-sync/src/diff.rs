// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of a schematic and a board, in both directions at once: a list of the
//! differences (parts, values, footprints, reference designators, nets, pins on no net, nets with
//! other pins) and of the conflicts that copying one document into the other would not settle, in
//! a fixed order, as text in eight languages and as JSON.
//!
//! The parts and the components are paired by identity (`Component::symbol`), never by name. The
//! comparison changes nothing and judges nothing: it says what differs and what clashes, and the
//! plans (`plan_forward`, `plan_backward`, `plan_annotation`) say what to do about it.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use eh3d_phoenix_board::{Board, Id, PadRef};
use eh3d_phoenix_rules::codes::word as common_word;
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_schematic::{NetPin, Schematic};
use serde::Serialize;
use serde_json::json;

use crate::annot_codes::{title, word, DiffKind};

/// The most pins named in the text of one difference.
const MAX_LISTED: usize = 8;

/// One difference or conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffEntry {
    /// What kind it is.
    pub kind: DiffKind,
    /// What it is about, by names: `R1`, `R1.2`, `net GND`.
    pub location: String,
    /// What the schematic holds.
    pub schematic: String,
    /// What the board holds.
    pub board: String,
}

impl DiffEntry {
    fn new(kind: DiffKind, location: &str, schematic: &str, board: &str) -> DiffEntry {
        DiffEntry {
            kind,
            location: location.to_owned(),
            schematic: schematic.to_owned(),
            board: board.to_owned(),
        }
    }

    /// How serious it is: a conflict is an error, a difference a warning.
    pub fn severity(&self) -> Severity {
        if self.kind.is_conflict() {
            Severity::Error
        } else {
            Severity::Warning
        }
    }
}

/// The result of the comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReport {
    /// The differences and conflicts, in a fixed order.
    pub entries: Vec<DiffEntry>,
}

impl DiffReport {
    /// True when the two documents agree and nothing clashes.
    pub fn in_sync(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of differences that are not conflicts.
    pub fn differences(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.kind.is_conflict())
            .count()
    }

    /// The number of conflicts.
    pub fn conflicts(&self) -> usize {
        self.entries.iter().filter(|e| e.kind.is_conflict()).count()
    }

    /// The report as text, with the titles in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} | {}: {}",
            word("differences", lang),
            self.differences(),
            word("conflicts", lang),
            self.conflicts()
        );
        for e in &self.entries {
            let _ = writeln!(
                out,
                "  [{}] {} {}: {} | {}: {} | {}: {}",
                e.severity().as_str(),
                e.kind.code(),
                title(e.kind.code(), lang).unwrap_or(""),
                e.location,
                word("schematic", lang),
                e.schematic,
                word("board", lang),
                e.board
            );
        }
        if self.entries.is_empty() {
            let _ = writeln!(out, "{}", common_word("nothing", lang));
        }
        out
    }

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        let entries: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|e| {
                json!({
                    "rule": e.kind.name(),
                    "code": e.kind.code(),
                    "severity": e.severity().as_str(),
                    "location": e.location,
                    "schematic": e.schematic,
                    "board": e.board,
                })
            })
            .collect();
        let value = json!({
            "schema": "phoenix-diff/1",
            "in_sync": self.in_sync(),
            "differences": self.differences(),
            "conflicts": self.conflicts(),
            "entries": entries,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn listed(pins: &[&String]) -> String {
    if pins.is_empty() {
        return "-".to_owned();
    }
    let mut text = pins
        .iter()
        .take(MAX_LISTED)
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if pins.len() > MAX_LISTED {
        let _ = write!(text, " (+{} more)", pins.len() - MAX_LISTED);
    }
    text
}

fn footprint_name(board: &Board, footprint: Option<Id>) -> String {
    footprint
        .and_then(|f| board.footprints.get(&f))
        .map_or_else(|| "-".to_owned(), |f| f.name.clone())
}

/// Compares a schematic and a board. The same pair always gives the same report.
pub fn compare(schematic: &Schematic, board: &Board) -> DiffReport {
    let mut entries: Vec<DiffEntry> = Vec::new();

    // part -> component; the second component of a part is a conflict and is left out
    let mut component_of: BTreeMap<Id, Id> = BTreeMap::new();
    let mut second: BTreeSet<Id> = BTreeSet::new();
    for (cid, component) in &board.components {
        let Some(pid) = component.symbol else {
            continue;
        };
        let Some(part) = schematic.parts.get(&pid) else {
            continue;
        };
        match component_of.entry(pid) {
            Entry::Occupied(_) => {
                second.insert(*cid);
                entries.push(DiffEntry::new(
                    DiffKind::PartTwice,
                    &part.refdes,
                    &part.refdes,
                    &component.refdes,
                ));
            }
            Entry::Vacant(slot) => {
                slot.insert(*cid);
            }
        }
    }
    let linked: BTreeSet<Id> = component_of.values().copied().collect();

    for (pid, part) in &schematic.parts {
        if !component_of.contains_key(pid) {
            entries.push(DiffEntry::new(
                DiffKind::PartOnly,
                &part.refdes,
                &part.value,
                "-",
            ));
        }
    }
    for (cid, component) in &board.components {
        if !linked.contains(cid) && !second.contains(cid) {
            entries.push(DiffEntry::new(
                DiffKind::ComponentOnly,
                &component.refdes,
                "-",
                &component.value,
            ));
        }
    }

    for (pid, cid) in &component_of {
        let (Some(part), Some(component)) = (schematic.parts.get(pid), board.components.get(cid))
        else {
            continue;
        };
        if part.refdes != component.refdes {
            entries.push(DiffEntry::new(
                DiffKind::Refdes,
                &part.refdes,
                &part.refdes,
                &component.refdes,
            ));
        }
        if part.value != component.value {
            entries.push(DiffEntry::new(
                DiffKind::Value,
                &part.refdes,
                &part.value,
                &component.value,
            ));
        }
        if part.footprint != Some(component.footprint) {
            entries.push(DiffEntry::new(
                DiffKind::Footprint,
                &part.refdes,
                &footprint_name(board, part.footprint),
                &footprint_name(board, Some(component.footprint)),
            ));
        }
    }

    reference_conflicts(schematic, board, &component_of, &mut entries);
    net_differences(schematic, board, &component_of, &mut entries);

    entries.sort_by(|a, b| {
        (a.kind.rank(), &a.location, &a.schematic, &a.board).cmp(&(
            b.kind.rank(),
            &b.location,
            &b.schematic,
            &b.board,
        ))
    });
    DiffReport { entries }
}

/// Reference designators used twice in one document, and references that a change would give to
/// an item that keeps its own.
fn reference_conflicts(
    schematic: &Schematic,
    board: &Board,
    component_of: &BTreeMap<Id, Id>,
    entries: &mut Vec<DiffEntry>,
) {
    let mut of_part: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for (pid, part) in &schematic.parts {
        of_part.entry(part.refdes.as_str()).or_default().push(*pid);
    }
    let mut of_component: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for (cid, component) in &board.components {
        of_component
            .entry(component.refdes.as_str())
            .or_default()
            .push(*cid);
    }
    for (refdes, ids) in &of_part {
        if ids.len() > 1 {
            entries.push(DiffEntry::new(
                DiffKind::RefdesTwice,
                refdes,
                &format!("{} parts", ids.len()),
                "-",
            ));
        }
    }
    for (refdes, ids) in &of_component {
        if ids.len() > 1 {
            entries.push(DiffEntry::new(
                DiffKind::RefdesTwice,
                refdes,
                "-",
                &format!("{} components", ids.len()),
            ));
        }
    }

    // an item keeps its reference when it is not paired, or when both of its pair agree
    let keeps_part = |pid: &Id| -> bool {
        match (
            schematic.parts.get(pid),
            component_of.get(pid).and_then(|c| board.components.get(c)),
        ) {
            (Some(part), Some(component)) => part.refdes == component.refdes,
            _ => true,
        }
    };
    let part_of: BTreeMap<Id, Id> = component_of.iter().map(|(p, c)| (*c, *p)).collect();
    let keeps_component = |cid: &Id| -> bool {
        match (
            board.components.get(cid),
            part_of.get(cid).and_then(|p| schematic.parts.get(p)),
        ) {
            (Some(component), Some(part)) => part.refdes == component.refdes,
            _ => true,
        }
    };
    for (pid, cid) in component_of {
        let (Some(part), Some(component)) = (schematic.parts.get(pid), board.components.get(cid))
        else {
            continue;
        };
        if part.refdes == component.refdes {
            continue;
        }
        let holds_in_schematic = of_part
            .get(component.refdes.as_str())
            .is_some_and(|ids| ids.iter().any(|b| b != pid && keeps_part(b)));
        if holds_in_schematic {
            entries.push(DiffEntry::new(
                DiffKind::RefdesTaken,
                &part.refdes,
                &format!("{} is held by another part that keeps it", component.refdes),
                &component.refdes,
            ));
        }
        let holds_on_board = of_component
            .get(part.refdes.as_str())
            .is_some_and(|ids| ids.iter().any(|c| c != cid && keeps_component(c)));
        if holds_on_board {
            entries.push(DiffEntry::new(
                DiffKind::RefdesTaken,
                &component.refdes,
                &part.refdes,
                &format!("{} is held by another component that keeps it", part.refdes),
            ));
        }
    }
}

/// Nets that only one document has, pins on a net in one document only and nets that hold other
/// pins in each document. Only the pins of paired parts are compared.
fn net_differences(
    schematic: &Schematic,
    board: &Board,
    component_of: &BTreeMap<Id, Id>,
    entries: &mut Vec<DiffEntry>,
) {
    let mut net_of_pin: BTreeMap<NetPin, &str> = BTreeMap::new();
    for net in schematic.nets.values() {
        for pin in &net.pins {
            net_of_pin.insert(*pin, net.name.as_str());
        }
    }
    // net name -> the pins that are on it in the schematic (and on a net on the board), and on
    // the board (and on a net in the schematic)
    let mut in_schematic: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut on_board: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (pid, cid) in component_of {
        let (Some(part), Some(component)) = (schematic.parts.get(pid), board.components.get(cid))
        else {
            continue;
        };
        let (Some(symbol), Some(footprint)) = (
            schematic.symbols.get(&part.symbol),
            board.footprints.get(&component.footprint),
        ) else {
            continue;
        };
        for (pin_id, pin) in &symbol.pins {
            let Some(pad) = footprint
                .pads
                .iter()
                .find(|(_, p)| p.number == pin.number)
                .map(|(id, _)| *id)
            else {
                continue;
            };
            let label = format!("{}.{}", part.refdes, pin.number);
            let sch = net_of_pin
                .get(&NetPin {
                    part: *pid,
                    pin: *pin_id,
                })
                .copied();
            let brd = board
                .net_of(PadRef {
                    component: *cid,
                    pad,
                })
                .and_then(|n| board.nets.get(&n))
                .map(|n| n.name.as_str());
            match (sch, brd) {
                (None, None) => {}
                (Some(s), Some(b)) => {
                    in_schematic.entry(s).or_default().insert(label.clone());
                    on_board.entry(b).or_default().insert(label);
                }
                (s, b) => entries.push(DiffEntry::new(
                    DiffKind::PinOneSide,
                    &label,
                    s.unwrap_or("-"),
                    b.unwrap_or("-"),
                )),
            }
        }
    }

    let schematic_names: BTreeSet<&str> = schematic
        .nets
        .values()
        .map(|n| n.name.as_str())
        .filter(|n| !n.trim().is_empty())
        .collect();
    let board_names: BTreeSet<&str> = board
        .nets
        .values()
        .map(|n| n.name.as_str())
        .filter(|n| !n.trim().is_empty())
        .collect();
    for name in schematic_names.difference(&board_names) {
        let pins: usize = schematic
            .nets
            .values()
            .filter(|n| n.name == *name)
            .map(|n| n.pins.len())
            .sum();
        entries.push(DiffEntry::new(
            DiffKind::NetOneSide,
            &format!("net {name}"),
            &format!("{pins} pins"),
            "-",
        ));
    }
    for name in board_names.difference(&schematic_names) {
        let ids: BTreeSet<Id> = board
            .nets
            .iter()
            .filter(|(_, n)| n.name == *name)
            .map(|(id, _)| *id)
            .collect();
        let pads = board
            .pad_nets
            .iter()
            .filter(|p| ids.contains(&p.net))
            .count();
        entries.push(DiffEntry::new(
            DiffKind::NetOneSide,
            &format!("net {name}"),
            "-",
            &format!("{pads} pads"),
        ));
    }
    let none: BTreeSet<String> = BTreeSet::new();
    for name in schematic_names.intersection(&board_names) {
        let a = in_schematic.get(name).unwrap_or(&none);
        let b = on_board.get(name).unwrap_or(&none);
        if a == b {
            continue;
        }
        let only_schematic: Vec<&String> = a.difference(b).collect();
        let only_board: Vec<&String> = b.difference(a).collect();
        entries.push(DiffEntry::new(
            DiffKind::NetPins,
            &format!("net {name}"),
            &listed(&only_schematic),
            &listed(&only_board),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_list_of_pins_is_cut_and_says_how_many_more() {
        let names: Vec<String> = (1..=10).map(|n| format!("R{n}.1")).collect();
        let refs: Vec<&String> = names.iter().collect();
        let text = listed(&refs);
        assert!(text.starts_with("R1.1, R2.1"));
        assert!(text.ends_with("R8.1 (+2 more)"));
        assert_eq!(listed(&[]), "-");
        assert_eq!(listed(&refs[..2]), "R1.1, R2.1");
    }

    #[test]
    fn two_empty_documents_agree() {
        let report = compare(&Schematic::default(), &Board::default());
        assert!(report.in_sync());
        assert_eq!((report.differences(), report.conflicts()), (0, 0));
        assert!(report.to_text(Lang::En).contains("nothing found"));
    }
}
