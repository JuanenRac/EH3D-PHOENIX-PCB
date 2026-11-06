// SPDX-License-Identifier: GPL-3.0-or-later
//! The annotation back into the schematic as a plan that is checked before anything is written.
//!
//! [`plan_annotation`] compares a schematic with the board it was transferred to, by the identity
//! of the parts, and lists every change the board would make to the schematic: reference
//! designators, values and footprints, the renumbering of the references by position, equivalent
//! pins and gates that the board connects the other way round, and the components and parts that
//! only one of the two documents has. Every item has a state:
//!
//! * **applicable**: the schematic can take it and it clashes with nothing;
//! * **conflict**: it clashes with another item or the schematic cannot take it (the reason says
//!   why); it is never applied;
//! * **ignored**: it is listed but not applied, because the settings leave it out or because it
//!   needs a drawing (a component with no part).
//!
//! Planning changes nothing. Applying is a second step that needs the confirmation code of the
//! plan (its digest), so a plan that was looked at is the plan that is applied, and it is applied
//! to a copy that the caller gives, as one step of the history, so it can be undone. The same
//! schematic, board and settings always give the same plan and the same digest.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use eh3d_phoenix_board::{Board, History, Id, Operation, PadRef, Side};
use eh3d_phoenix_rules::codes::word as common_word;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{NetPin, Schematic};
use serde::Serialize;
use serde_json::json;

use crate::annot_codes::{title, word, ItemKind, CLASH, DIGEST_MISMATCH, LEFT_OUT, NOT_APPLIED};
use crate::annotate_settings::{AnnotateSettings, Order};
use crate::backward::{PinMove, SchematicEdit, SchematicHistory};

/// What becomes of an item of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// It can be applied.
    Applicable,
    /// It clashes with the rest of the plan or cannot be taken.
    Conflict,
    /// It is listed and not applied.
    Ignored,
}

impl Status {
    /// The stable name of the state (`applicable`).
    pub fn name(self) -> &'static str {
        match self {
            Status::Applicable => "applicable",
            Status::Conflict => "conflict",
            Status::Ignored => "ignored",
        }
    }
}

/// One change of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanItem {
    /// The place in the plan, from 1.
    pub number: usize,
    /// What kind of change it is.
    pub kind: ItemKind,
    /// The code shown for it: the one of the kind, or the one of the conflict or of the setting
    /// that leaves it out.
    pub code: &'static str,
    /// What becomes of it.
    pub status: Status,
    /// What it is about, by names: `R1`, `U2 pins 1,2`.
    pub location: String,
    /// What the schematic holds now.
    pub from: String,
    /// What it would hold.
    pub to: String,
    /// Why it is a conflict or ignored (English); empty for an applicable item.
    pub reason: String,
    /// The edit of the schematic that makes the change, for an applicable item.
    #[serde(skip)]
    pub edit: Option<SchematicEdit>,
    /// The operation of the board that goes with it (the renumbering renames components).
    #[serde(skip)]
    pub board_edit: Option<Operation>,
}

/// Why a plan could not be applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnnotateError {
    /// The confirmation is not the digest of the plan.
    #[error("the confirmation {given:?} is not the code of this plan ({expected})")]
    DigestMismatch {
        /// The digest of the plan.
        expected: String,
        /// What was given.
        given: String,
    },
    /// An edit did not apply to the document.
    #[error("{0}")]
    Apply(String),
}

impl AnnotateError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            AnnotateError::DigestMismatch { .. } => DIGEST_MISMATCH,
            AnnotateError::Apply(_) => NOT_APPLIED,
        }
    }
}

/// The plan: the changes in a fixed order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatePlan {
    /// The items, in the fixed order (kind, place, then the texts).
    pub items: Vec<PlanItem>,
}

impl AnnotatePlan {
    /// The number of items in a state.
    pub fn count(&self, status: Status) -> usize {
        self.items.iter().filter(|i| i.status == status).count()
    }

    /// True when nothing can be applied and nothing clashes.
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|i| i.status == Status::Ignored)
    }

    /// The confirmation code of the plan: a fingerprint of every item, in order, with its state.
    /// It is not a signature (it guards against applying another plan than the one that was read,
    /// not against a person who writes both).
    pub fn digest(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |text: &str| {
            for byte in text.bytes().chain(std::iter::once(0x1f)) {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for item in &self.items {
            feed(item.kind.name());
            feed(item.code);
            feed(item.status.name());
            feed(&item.location);
            feed(&item.from);
            feed(&item.to);
            feed(&item.reason);
        }
        format!("{hash:016x}")
    }

    fn applicable(&self) -> impl Iterator<Item = &PlanItem> {
        self.items.iter().filter(|i| i.status == Status::Applicable)
    }

    fn check(&self, confirmation: &str) -> Result<(), AnnotateError> {
        let expected = self.digest();
        if confirmation.trim().eq_ignore_ascii_case(&expected) {
            Ok(())
        } else {
            Err(AnnotateError::DigestMismatch {
                expected,
                given: confirmation.to_owned(),
            })
        }
    }

    /// The edits of the applicable items, in order.
    pub fn edits(&self) -> Vec<SchematicEdit> {
        self.applicable().filter_map(|i| i.edit.clone()).collect()
    }

    /// The operations of the board that go with the applicable items (the renumbering), in order.
    pub fn board_operations(&self) -> Vec<Operation> {
        self.applicable()
            .filter_map(|i| i.board_edit.clone())
            .collect()
    }

    /// Applies the applicable items to `schematic` as one step of `history`, if `confirmation` is
    /// the digest of the plan; returns how many edits were made. The caller gives a copy: nothing
    /// is written anywhere. Nothing is recorded when there is nothing to apply.
    pub fn apply_to_schematic(
        &self,
        schematic: &mut Schematic,
        history: &mut SchematicHistory,
        confirmation: &str,
    ) -> Result<usize, AnnotateError> {
        self.check(confirmation)?;
        let edits = self.edits();
        if edits.is_empty() {
            return Ok(0);
        }
        let count = edits.len();
        history
            .execute(schematic, SchematicEdit::Batch { edits })
            .map_err(|e| AnnotateError::Apply(e.to_string()))?;
        Ok(count)
    }

    /// Applies the renamings of the board that go with the plan as one step of `history`, if
    /// `confirmation` is the digest of the plan; returns how many operations were made.
    pub fn apply_to_board(
        &self,
        board: &mut Board,
        history: &mut History,
        confirmation: &str,
    ) -> Result<usize, AnnotateError> {
        self.check(confirmation)?;
        let ops = self.board_operations();
        if ops.is_empty() {
            return Ok(0);
        }
        let count = ops.len();
        history
            .execute(board, Operation::Batch { ops })
            .map_err(|e| AnnotateError::Apply(e.to_string()))?;
        Ok(count)
    }

    /// The plan as text, with the titles in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} | {} {} | {} {} | {} {}",
            word("plan", lang),
            self.items.len(),
            self.count(Status::Applicable),
            word("applicable", lang),
            self.count(Status::Conflict),
            word("conflict", lang),
            self.count(Status::Ignored),
            word("ignored", lang)
        );
        let _ = writeln!(out, "{}: {}", word("confirm", lang), self.digest());
        for item in &self.items {
            let _ = write!(
                out,
                "  {:>3} [{}] {} {}: {} | {} -> {}",
                item.number,
                word(item.status.name(), lang),
                item.code,
                title(item.code, lang).unwrap_or(""),
                item.location,
                item.from,
                item.to
            );
            if !item.reason.is_empty() {
                let _ = write!(out, " -- {}", item.reason);
            }
            out.push('\n');
        }
        if self.items.is_empty() {
            let _ = writeln!(out, "{}", common_word("nothing", lang));
        }
        out
    }

    /// The plan as JSON.
    pub fn to_json(&self) -> String {
        let items: Vec<serde_json::Value> = self
            .items
            .iter()
            .map(|i| {
                json!({
                    "n": i.number,
                    "rule": i.kind.name(),
                    "code": i.code,
                    "status": i.status.name(),
                    "location": i.location,
                    "from": i.from,
                    "to": i.to,
                    "reason": i.reason,
                })
            })
            .collect();
        let value = json!({
            "schema": "phoenix-annotate/1",
            "digest": self.digest(),
            "applicable": self.count(Status::Applicable),
            "conflicts": self.count(Status::Conflict),
            "ignored": self.count(Status::Ignored),
            "items": items,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn draft(
    kind: ItemKind,
    status: Status,
    location: &str,
    from: &str,
    to: &str,
    reason: &str,
) -> PlanItem {
    let code = match status {
        Status::Conflict => CLASH,
        _ if reason.starts_with("the settings") => LEFT_OUT,
        _ => kind.code(),
    };
    PlanItem {
        number: 0,
        kind,
        code,
        status,
        location: location.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
        reason: reason.to_owned(),
        edit: None,
        board_edit: None,
    }
}

fn footprint_name(board: &Board, footprint: Option<Id>) -> String {
    footprint
        .and_then(|f| board.footprints.get(&f))
        .map_or_else(|| "-".to_owned(), |f| f.name.clone())
}

/// The letters a reference designator starts with, in upper case.
fn prefix_of(refdes: &str) -> String {
    refdes
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// A component to be numbered: its place on the board.
#[derive(Debug, Clone)]
struct Candidate {
    id: Id,
    at: (i64, i64),
}

/// The components of one group in the order of their numbers.
fn in_order(mut group: Vec<Candidate>, settings: &AnnotateSettings) -> Vec<Candidate> {
    // the first coordinate runs along the order, the second inside a row (or column); "top first"
    // is the smaller number when the Y axis grows downwards
    let key = |c: &Candidate| -> (i64, i64) {
        let (x, y) = c.at;
        let top_first = if settings.y_axis_up { -y } else { y };
        match settings.order {
            Order::TopToBottom => (top_first, x),
            Order::LeftToRight => (x, top_first),
        }
    };
    group.sort_by(|a, b| (key(a), a.id).cmp(&(key(b), b.id)));
    let mut out: Vec<Candidate> = Vec::with_capacity(group.len());
    let mut i = 0;
    while i < group.len() {
        let first = key(&group[i]).0;
        let mut j = i;
        while j < group.len() && key(&group[j]).0 - first <= settings.row_tolerance_nm {
            j += 1;
        }
        let mut band: Vec<Candidate> = group[i..j].to_vec();
        band.sort_by(|a, b| (key(a).1, a.id).cmp(&(key(b).1, b.id)));
        out.extend(band);
        i = j;
    }
    out
}

/// The new reference designators of the components that are numbered by position: the linked
/// components of the prefixes the settings name, minus the locked ones. Every other reference of
/// either document is reserved, so a number is never given twice.
fn renumbering(
    schematic: &Schematic,
    board: &Board,
    settings: &AnnotateSettings,
    component_of: &BTreeMap<Id, Id>,
) -> BTreeMap<Id, String> {
    let linked: BTreeSet<Id> = component_of.values().copied().collect();
    let mut reserved: BTreeSet<String> = BTreeSet::new();
    let mut groups: BTreeMap<(String, bool), Vec<Candidate>> = BTreeMap::new();
    for (cid, component) in &board.components {
        let prefix = prefix_of(&component.refdes);
        let upper = component.refdes.trim().to_ascii_uppercase();
        if linked.contains(cid) && settings.renumbers(&prefix) && !settings.locked.contains(&upper)
        {
            let bottom = settings.by_side && component.side == Side::Bottom;
            groups.entry((prefix, bottom)).or_default().push(Candidate {
                id: *cid,
                at: (component.at.x_nm.get(), component.at.y_nm.get()),
            });
        } else {
            reserved.insert(upper);
        }
    }
    for (pid, part) in &schematic.parts {
        if !component_of.contains_key(pid) {
            reserved.insert(part.refdes.trim().to_ascii_uppercase());
        }
    }
    let mut out: BTreeMap<Id, String> = BTreeMap::new();
    let mut used = reserved;
    for ((prefix, bottom), group) in groups {
        let offset = if bottom {
            u64::from(settings.bottom_offset)
        } else {
            0
        };
        let mut number = u64::from(settings.start_for(&prefix)) + offset;
        let step = u64::from(settings.step_for(&prefix));
        for candidate in in_order(group, settings) {
            let name = loop {
                let name = format!("{prefix}{number}");
                number += step;
                if !used.contains(&name) {
                    break name;
                }
            };
            used.insert(name.clone());
            out.insert(candidate.id, name);
        }
    }
    out
}

/// A pin of a slot with the net it is on in each document.
struct Leg {
    pin: NetPin,
    schematic: Option<String>,
    board: Option<String>,
}

fn names_text(names: &[Option<String>]) -> String {
    names
        .iter()
        .map(|n| n.as_deref().unwrap_or("-"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The swaps of equivalent pins and gates that the board makes.
fn plan_swaps(
    schematic: &Schematic,
    board: &Board,
    settings: &AnnotateSettings,
    component_of: &BTreeMap<Id, Id>,
    items: &mut Vec<PlanItem>,
) {
    if settings.swaps.is_empty() {
        return;
    }
    let mut by_name: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    let mut net_of: BTreeMap<NetPin, Id> = BTreeMap::new();
    for (id, net) in &schematic.nets {
        by_name.entry(net.name.as_str()).or_default().push(*id);
        for pin in &net.pins {
            net_of.insert(*pin, *id);
        }
    }
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
        let Some(groups) = settings.swaps.get(&symbol.name) else {
            continue;
        };
        for group in groups {
            let mut slots: Vec<Vec<Leg>> = Vec::new();
            let mut complete = true;
            'slots: for slot in group {
                let mut row: Vec<Leg> = Vec::new();
                for number in slot {
                    let pin = symbol
                        .pins
                        .iter()
                        .find(|(_, p)| &p.number == number)
                        .map(|(id, _)| *id);
                    let pad = footprint
                        .pads
                        .iter()
                        .find(|(_, p)| &p.number == number)
                        .map(|(id, _)| *id);
                    let (Some(pin), Some(pad)) = (pin, pad) else {
                        complete = false;
                        break 'slots;
                    };
                    let pin = NetPin { part: *pid, pin };
                    row.push(Leg {
                        pin,
                        schematic: net_of
                            .get(&pin)
                            .and_then(|n| schematic.nets.get(n))
                            .map(|n| n.name.clone()),
                        board: board
                            .net_of(PadRef {
                                component: *cid,
                                pad,
                            })
                            .and_then(|n| board.nets.get(&n))
                            .map(|n| n.name.clone()),
                    });
                }
                slots.push(row);
            }
            if !complete {
                continue;
            }
            let from_nets: Vec<Vec<Option<String>>> = slots
                .iter()
                .map(|r| r.iter().map(|l| l.schematic.clone()).collect())
                .collect();
            let to_nets: Vec<Vec<Option<String>>> = slots
                .iter()
                .map(|r| r.iter().map(|l| l.board.clone()).collect())
                .collect();
            if from_nets == to_nets {
                continue;
            }
            let (mut a, mut b) = (from_nets.clone(), to_nets.clone());
            a.sort();
            b.sort();
            if a != b {
                // not a swap: the nets are different, which the comparison reports
                continue;
            }
            let numbers: Vec<&str> = group.iter().flatten().map(String::as_str).collect();
            let location = format!("{} pins {}", part.refdes, numbers.join(","));
            let text = |nets: &[Vec<Option<String>>]| -> String {
                group
                    .iter()
                    .zip(nets)
                    .map(|(slot, names)| format!("{}={}", slot.join(","), names_text(names)))
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            let (from, to) = (text(from_nets.as_slice()), text(to_nets.as_slice()));
            if !schematic.wires.is_empty() {
                items.push(draft(
                    ItemKind::PinsSwapped,
                    Status::Conflict,
                    &location,
                    &from,
                    &to,
                    "the drawing has wires that a change of the net list would not follow: swap the wires by hand",
                ));
                continue;
            }
            let mut moves: Vec<PinMove> = Vec::new();
            let mut problem: Option<String> = None;
            for leg in slots.iter().flatten() {
                if leg.schematic == leg.board {
                    continue;
                }
                let net = match &leg.board {
                    None => None,
                    Some(name) => match by_name.get(name.as_str()).map(Vec::as_slice) {
                        Some([only]) => Some(*only),
                        _ => {
                            problem = Some(format!(
                                "the net {name} is not one net of the schematic (none, or two with one name)"
                            ));
                            break;
                        }
                    },
                };
                moves.push(PinMove { pin: leg.pin, net });
            }
            match problem {
                Some(reason) => items.push(draft(
                    ItemKind::PinsSwapped,
                    Status::Conflict,
                    &location,
                    &from,
                    &to,
                    &reason,
                )),
                None => {
                    let mut item = draft(
                        ItemKind::PinsSwapped,
                        Status::Applicable,
                        &location,
                        &from,
                        &to,
                        "",
                    );
                    item.edit = Some(SchematicEdit::SetPinNets { moves });
                    items.push(item);
                }
            }
        }
    }
}

/// Plans the writing back of the board into the schematic. The same schematic, board and settings
/// always give the same plan. Nothing is changed.
pub fn plan_annotation(
    schematic: &Schematic,
    board: &Board,
    settings: &AnnotateSettings,
) -> AnnotatePlan {
    let mut items: Vec<PlanItem> = Vec::new();

    // part -> component; a second component of a part is a clash and is left alone
    let mut component_of: BTreeMap<Id, Id> = BTreeMap::new();
    let mut second: BTreeSet<Id> = BTreeSet::new();
    for (cid, component) in &board.components {
        let Some(pid) = component.symbol else {
            continue;
        };
        if !schematic.parts.contains_key(&pid) {
            continue;
        }
        match component_of.entry(pid) {
            Entry::Occupied(_) => {
                second.insert(*cid);
                items.push(draft(
                    ItemKind::Clash,
                    Status::Conflict,
                    &component.refdes,
                    "-",
                    "-",
                    "a second component of the same part is left alone",
                ));
            }
            Entry::Vacant(slot) => {
                slot.insert(*cid);
            }
        }
    }

    let renamed: BTreeMap<Id, String> = if settings.renumber {
        renumbering(schematic, board, settings, &component_of)
    } else {
        BTreeMap::new()
    };

    // the reference designators each document would have once the plan is applied
    let mut target: BTreeMap<Id, String> = BTreeMap::new();
    for (pid, cid) in &component_of {
        if let Some(component) = board.components.get(cid) {
            let name = renamed
                .get(cid)
                .cloned()
                .unwrap_or_else(|| component.refdes.clone());
            target.insert(*pid, name);
        }
    }
    let mut schematic_uses: BTreeMap<&str, usize> = BTreeMap::new();
    for (pid, part) in &schematic.parts {
        let name = target.get(pid).map_or(part.refdes.as_str(), String::as_str);
        *schematic_uses.entry(name).or_default() += 1;
    }
    let linked: BTreeSet<Id> = component_of.values().copied().collect();
    let mut board_uses: BTreeMap<&str, usize> = BTreeMap::new();
    for (cid, component) in &board.components {
        let name = renamed
            .get(cid)
            .map_or(component.refdes.as_str(), String::as_str);
        *board_uses.entry(name).or_default() += 1;
    }

    for (pid, cid) in &component_of {
        let (Some(part), Some(component), Some(wanted)) = (
            schematic.parts.get(pid),
            board.components.get(cid),
            target.get(pid),
        ) else {
            continue;
        };
        let renumbered = renamed.contains_key(cid) && *wanted != component.refdes;
        if *wanted != part.refdes || renumbered {
            let kind = if renumbered {
                ItemKind::Renumbered
            } else {
                ItemKind::RefdesFromBoard
            };
            let clash = wanted.trim().is_empty()
                || schematic_uses.get(wanted.as_str()).copied().unwrap_or(0) > 1
                || board_uses.get(wanted.as_str()).copied().unwrap_or(0) > 1;
            if clash {
                let reason = if wanted.trim().is_empty() {
                    "the reference would be empty".to_owned()
                } else {
                    format!("the reference {wanted:?} would be used twice")
                };
                items.push(draft(
                    kind,
                    Status::Conflict,
                    &part.refdes,
                    &part.refdes,
                    wanted,
                    &reason,
                ));
            } else {
                let (location, from, reason) = if *wanted == part.refdes {
                    (
                        part.refdes.as_str(),
                        component.refdes.as_str(),
                        "the schematic already has this reference; only the board is renamed",
                    )
                } else {
                    (part.refdes.as_str(), part.refdes.as_str(), "")
                };
                let mut item = draft(kind, Status::Applicable, location, from, wanted, reason);
                if *wanted != part.refdes {
                    item.edit = Some(SchematicEdit::SetRefdes {
                        part: *pid,
                        refdes: wanted.clone(),
                    });
                }
                if renumbered {
                    item.board_edit = Some(Operation::SetComponentFields {
                        id: *cid,
                        refdes: wanted.clone(),
                        value: component.value.clone(),
                        footprint: component.footprint,
                    });
                }
                items.push(item);
            }
        }
        if component.value != part.value {
            if settings.values {
                let mut item = draft(
                    ItemKind::ValueFromBoard,
                    Status::Applicable,
                    &part.refdes,
                    &part.value,
                    &component.value,
                    "",
                );
                item.edit = Some(SchematicEdit::SetValue {
                    part: *pid,
                    value: component.value.clone(),
                });
                items.push(item);
            } else {
                items.push(draft(
                    ItemKind::ValueFromBoard,
                    Status::Ignored,
                    &part.refdes,
                    &part.value,
                    &component.value,
                    "the settings leave the values out",
                ));
            }
        }
        if part.footprint != Some(component.footprint) {
            let from = footprint_name(board, part.footprint);
            let to = footprint_name(board, Some(component.footprint));
            if !settings.footprints {
                items.push(draft(
                    ItemKind::FootprintFromBoard,
                    Status::Ignored,
                    &part.refdes,
                    &from,
                    &to,
                    "the settings leave the footprints out",
                ));
            } else if board.footprints.contains_key(&component.footprint) {
                let mut item = draft(
                    ItemKind::FootprintFromBoard,
                    Status::Applicable,
                    &part.refdes,
                    &from,
                    &to,
                    "",
                );
                item.edit = Some(SchematicEdit::SetFootprint {
                    part: *pid,
                    footprint: Some(component.footprint),
                });
                items.push(item);
            } else {
                items.push(draft(
                    ItemKind::FootprintFromBoard,
                    Status::Conflict,
                    &part.refdes,
                    &from,
                    &to,
                    "the footprint of the component is not in the board",
                ));
            }
        }
    }

    plan_swaps(schematic, board, settings, &component_of, &mut items);

    for (cid, component) in &board.components {
        if linked.contains(cid) || second.contains(cid) {
            continue;
        }
        items.push(draft(
            ItemKind::ComponentAdded,
            Status::Ignored,
            &component.refdes,
            "-",
            &footprint_name(board, Some(component.footprint)),
            "a part needs a symbol: draw the part in the schematic and link it",
        ));
    }
    for (pid, part) in &schematic.parts {
        if component_of.contains_key(pid) {
            continue;
        }
        items.push(draft(
            ItemKind::PartRemoved,
            Status::Ignored,
            &part.refdes,
            &part.value,
            "-",
            "the board has no component for this part; delete the part by hand if it was dropped",
        ));
    }

    items.sort_by(|a, b| {
        (a.kind.rank(), &a.location, &a.from, &a.to).cmp(&(
            b.kind.rank(),
            &b.location,
            &b.from,
            &b.to,
        ))
    });
    for (index, item) in items.iter_mut().enumerate() {
        item.number = index + 1;
    }
    AnnotatePlan { items }
}
