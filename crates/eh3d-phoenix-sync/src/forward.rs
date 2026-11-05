// SPDX-License-Identifier: GPL-3.0-or-later
//! Forward annotation: the transfer of the schematic to the board.
//!
//! The parts are linked to the components by identity (`Component::symbol` holds the id of the part),
//! never by name, so a renamed reference designator is a change and not a new part. The plan is a
//! list of operations of the board that are applied as one step of its undo/redo history; the
//! board is never changed by planning (a dry run), and what the plan does not name is not touched:
//! no track, via, zone, silkscreen item or position is ever changed or removed by it.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Component, History, Id, OpError, Operation, PadRef};
use eh3d_phoenix_model::{Point, Udeg};
use eh3d_phoenix_schematic::{NetPin, Schematic};
use serde_json::json;

use crate::codes::{
    ChangeKind, FOOTPRINT_UNUSABLE, NET_NAME_TWICE, NOT_APPLIED, PART_TWICE, PIN_WITHOUT_PAD,
    REFDES_CLASH,
};
use crate::report::{Change, Direction, Report, Skipped};
use crate::settings::SyncSettings;

/// The changes that make a board agree with a schematic, as a package: the differences, what could
/// not be done and the operation that applies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardPackage {
    /// The differences that the operations remove, in a fixed order.
    pub changes: Vec<Change>,
    /// What could not be judged or done.
    pub skipped: Vec<Skipped>,
    /// The operations, in the order they are applied.
    pub operations: Vec<Operation>,
}

impl ForwardPackage {
    /// True when the board already agrees with the schematic (a component that is only listed
    /// because its part is gone does not count).
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// The operations as one batch: one step of the history, applied all or not at all.
    pub fn operation(&self) -> Operation {
        Operation::Batch {
            ops: self.operations.clone(),
        }
    }

    /// Applies the package to the board as one step of `history`, so it can be undone and redone.
    /// Nothing happens, and nothing is recorded, when the package is empty.
    pub fn apply(&self, board: &mut Board, history: &mut History) -> Result<(), OpError> {
        if self.is_empty() {
            return Ok(());
        }
        history.execute(board, self.operation())
    }

    /// The report: the changes and what was left out.
    pub fn report(&self) -> Report {
        Report::build(
            Direction::Forward,
            self.operations.len(),
            &self.changes,
            &self.skipped,
        )
    }

    /// The package as JSON: the changes, what was left out and the operations.
    pub fn to_json(&self) -> String {
        let value = json!({
            "schema": "phoenix-sync-package/1",
            "direction": Direction::Forward.as_str(),
            "changes": self.changes,
            "skipped": self.skipped,
            "operations": self.operations,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// The pad of a component by the number of a pin.
fn pad_of(board: &Board, component: Id, number: &str) -> Option<PadRef> {
    let footprint = board
        .footprints
        .get(&board.components.get(&component)?.footprint)?;
    let (pad, _) = footprint.pads.iter().find(|(_, p)| p.number == number)?;
    Some(PadRef {
        component,
        pad: *pad,
    })
}

fn net_name(board: &Board, net: Option<Id>) -> String {
    net.and_then(|n| board.nets.get(&n))
        .map_or_else(|| "-".to_owned(), |n| n.name.clone())
}

fn footprint_name(board: &Board, footprint: Id) -> String {
    board
        .footprints
        .get(&footprint)
        .map_or_else(|| "-".to_owned(), |f| f.name.clone())
}

struct Planner<'a> {
    schematic: &'a Schematic,
    settings: &'a SyncSettings,
    /// The board as it will be once the operations planned so far are applied.
    scratch: Board,
    operations: Vec<Operation>,
    changes: Vec<Change>,
    skipped: Vec<Skipped>,
    /// part -> component, for the parts that have one (planned ones included)
    component_of: BTreeMap<Id, Id>,
}

impl Planner<'_> {
    /// Applies an operation to the scratch board and keeps it when it applies.
    fn push(&mut self, op: Operation, what: &str) -> bool {
        match self.scratch.apply(&op) {
            Ok(_) => {
                self.operations.push(op);
                true
            }
            Err(error) => {
                self.skipped
                    .push(Skipped::new(NOT_APPLIED, what, &error.to_string()));
                false
            }
        }
    }

    fn new_parts(&mut self, new_id: &mut dyn FnMut() -> Id) {
        let s = self.schematic;
        let mut parts: Vec<(&String, Id)> = s
            .parts
            .iter()
            .filter(|(id, _)| !self.component_of.contains_key(*id))
            .map(|(id, p)| (&p.refdes, *id))
            .collect();
        parts.sort();
        let mut placed: i64 = 0;
        for (_, part_id) in parts {
            let part = &s.parts[&part_id];
            let name = part.refdes.as_str();
            let Some(footprint) = part.footprint else {
                self.skipped.push(Skipped::new(
                    FOOTPRINT_UNUSABLE,
                    name,
                    "the part has no footprint chosen, so no component is made",
                ));
                continue;
            };
            if !self.scratch.footprints.contains_key(&footprint) {
                self.skipped.push(Skipped::new(
                    FOOTPRINT_UNUSABLE,
                    name,
                    "the footprint of the part is not in the board",
                ));
                continue;
            }
            if name.trim().is_empty() || self.scratch.components.values().any(|c| c.refdes == name)
            {
                self.skipped.push(Skipped::new(
                    REFDES_CLASH,
                    name,
                    "a component of the board already has this reference designator",
                ));
                continue;
            }
            let x = placed
                .checked_mul(self.settings.new_part_step_nm)
                .and_then(|d| d.checked_add(self.settings.new_part_x_nm));
            let at = x.and_then(|x| Point::from_nm(x, self.settings.new_part_y_nm).ok());
            let Some(at) = at else {
                self.skipped.push(Skipped::new(
                    NOT_APPLIED,
                    name,
                    "the place of the new component is outside the design range",
                ));
                continue;
            };
            let id = new_id();
            let component = Component {
                refdes: part.refdes.clone(),
                value: part.value.clone(),
                symbol: Some(part_id),
                footprint,
                at,
                rotation: Udeg::ZERO,
                side: self.settings.new_part_side,
            };
            let footprint_text = footprint_name(&self.scratch, footprint);
            if self.push(Operation::AddComponent { id, component }, name) {
                self.component_of.insert(part_id, id);
                placed += 1;
                self.changes
                    .push(Change::new(ChangeKind::NewPart, name, "-", &footprint_text));
            }
        }
    }

    fn fields(&mut self) {
        let s = self.schematic;
        // the reference designators the components would have, to refuse a clash
        let mut wanted: BTreeMap<Id, String> = self
            .scratch
            .components
            .iter()
            .map(|(id, c)| (*id, c.refdes.clone()))
            .collect();
        for (part_id, cid) in &self.component_of {
            if let Some(part) = s.parts.get(part_id) {
                wanted.insert(*cid, part.refdes.clone());
            }
        }
        let mut uses: BTreeMap<String, usize> = BTreeMap::new();
        for refdes in wanted.into_values() {
            *uses.entry(refdes).or_default() += 1;
        }
        let pairs: Vec<(Id, Id)> = self.component_of.iter().map(|(p, c)| (*p, *c)).collect();
        for (part_id, cid) in pairs {
            let (Some(part), Some(component)) = (
                s.parts.get(&part_id),
                self.scratch.components.get(&cid).cloned(),
            ) else {
                continue;
            };
            let mut refdes = component.refdes.clone();
            if part.refdes != component.refdes {
                if part.refdes.trim().is_empty()
                    || uses.get(part.refdes.as_str()).copied().unwrap_or(0) > 1
                {
                    self.skipped.push(Skipped::new(
                        REFDES_CLASH,
                        &component.refdes,
                        &format!("the schematic calls it {:?}", part.refdes),
                    ));
                } else {
                    self.changes.push(Change::new(
                        ChangeKind::RefdesDiffers,
                        &part.refdes,
                        &component.refdes,
                        &part.refdes,
                    ));
                    refdes = part.refdes.clone();
                }
            }
            let mut value = component.value.clone();
            if part.value != component.value {
                self.changes.push(Change::new(
                    ChangeKind::ValueDiffers,
                    &part.refdes,
                    &component.value,
                    &part.value,
                ));
                value = part.value.clone();
            }
            let mut footprint = component.footprint;
            let mut footprint_changes = false;
            if let Some(wanted_footprint) = part.footprint {
                if wanted_footprint != component.footprint {
                    if self.scratch.footprints.contains_key(&wanted_footprint) {
                        self.changes.push(Change::new(
                            ChangeKind::FootprintDiffers,
                            &part.refdes,
                            &footprint_name(&self.scratch, component.footprint),
                            &footprint_name(&self.scratch, wanted_footprint),
                        ));
                        footprint = wanted_footprint;
                        footprint_changes = true;
                    } else {
                        self.skipped.push(Skipped::new(
                            FOOTPRINT_UNUSABLE,
                            &part.refdes,
                            "the footprint of the part is not in the board",
                        ));
                    }
                }
            }
            if refdes == component.refdes && value == component.value && !footprint_changes {
                continue;
            }
            let place = refdes.clone();
            if footprint_changes {
                // the pads of the old footprint lose their nets first; the nets are given again
                // to the pads of the new footprint by the pins
                let pads: Vec<PadRef> = self
                    .scratch
                    .pad_nets
                    .iter()
                    .filter(|p| p.component == cid)
                    .map(|p| p.pad_ref())
                    .collect();
                for pad in pads {
                    self.push(Operation::SetPadNet { pad, net: None }, &place);
                }
            }
            self.push(
                Operation::SetComponentFields {
                    id: cid,
                    refdes,
                    value,
                    footprint,
                },
                &place,
            );
        }
    }

    fn nets(&mut self, new_id: &mut dyn FnMut() -> Id) {
        let s = self.schematic;
        let mut order: Vec<(&String, Id)> = s.nets.iter().map(|(id, n)| (&n.name, *id)).collect();
        order.sort();
        let schematic_names: BTreeSet<&str> = s.nets.values().map(|n| n.name.as_str()).collect();
        let mut claimed: BTreeSet<Id> = BTreeSet::new();
        let mut names_done: BTreeSet<&str> = BTreeSet::new();
        for (name, net_id) in order {
            let net = &s.nets[&net_id];
            // the pads of the pins of this net
            let mut pads: Vec<(PadRef, String)> = Vec::new();
            for pin in &net.pins {
                let (Some(cid), Some(number), Some(part)) = (
                    self.component_of.get(&pin.part).copied(),
                    s.pin_number(*pin),
                    s.parts.get(&pin.part),
                ) else {
                    continue; // the part has no component: already reported
                };
                match pad_of(&self.scratch, cid, number) {
                    Some(pad) => pads.push((pad, format!("{}.{number}", part.refdes))),
                    None => self.skipped.push(Skipped::new(
                        PIN_WITHOUT_PAD,
                        &format!("{}.{number}", part.refdes),
                        "the footprint of the component has no pad of this number",
                    )),
                }
            }
            if pads.is_empty() {
                continue;
            }
            if !names_done.insert(name.as_str()) {
                self.skipped.push(Skipped::new(
                    NET_NAME_TWICE,
                    &format!("net {name}"),
                    "a second net of the schematic with this name is left alone",
                ));
                continue;
            }
            let place = format!("net {name}");
            // the net of the board that stands for this one
            let same_name = self
                .scratch
                .nets
                .iter()
                .find(|(id, n)| n.name == *name && !claimed.contains(*id))
                .map(|(id, _)| *id);
            let target = match same_name {
                Some(id) => Some(id),
                None => {
                    let current: BTreeSet<Option<Id>> = pads
                        .iter()
                        .map(|(pad, _)| self.scratch.net_of(*pad))
                        .collect();
                    let single = match current.iter().next() {
                        Some(Some(only)) if current.len() == 1 => Some(*only),
                        _ => None,
                    };
                    let rename = single.filter(|id| {
                        self.settings.rename_nets
                            && !claimed.contains(id)
                            && self
                                .scratch
                                .nets
                                .get(id)
                                .is_some_and(|n| !schematic_names.contains(n.name.as_str()))
                    });
                    match rename {
                        Some(id) => {
                            let old = net_name(&self.scratch, Some(id));
                            if self.push(
                                Operation::RenameNet {
                                    id,
                                    name: name.clone(),
                                },
                                &place,
                            ) {
                                self.changes.push(Change::new(
                                    ChangeKind::NetRenamed,
                                    &place,
                                    &old,
                                    name,
                                ));
                                Some(id)
                            } else {
                                None
                            }
                        }
                        None => {
                            let id = new_id();
                            if self.push(
                                Operation::AddNet {
                                    id,
                                    name: name.clone(),
                                },
                                &place,
                            ) {
                                self.changes.push(Change::new(
                                    ChangeKind::NetAdded,
                                    &place,
                                    "-",
                                    name,
                                ));
                                Some(id)
                            } else {
                                None
                            }
                        }
                    }
                }
            };
            let Some(target) = target else {
                continue;
            };
            claimed.insert(target);
            for (pad, label) in pads {
                let current = self.scratch.net_of(pad);
                if current != Some(target) {
                    let from = net_name(&self.scratch, current);
                    if self.push(
                        Operation::SetPadNet {
                            pad,
                            net: Some(target),
                        },
                        &label,
                    ) {
                        self.changes.push(Change::new(
                            ChangeKind::PadNetChanged,
                            &label,
                            &from,
                            name,
                        ));
                    }
                }
            }
        }

        // pads whose pin is on no net of the schematic lose the net they have
        let in_net: BTreeSet<NetPin> = s
            .nets
            .values()
            .flat_map(|n| n.pins.iter().copied())
            .collect();
        let pairs: Vec<(Id, Id)> = self.component_of.iter().map(|(p, c)| (*p, *c)).collect();
        for (part_id, cid) in pairs {
            let Some(part) = s.parts.get(&part_id) else {
                continue;
            };
            let Some(symbol) = s.symbols.get(&part.symbol) else {
                continue;
            };
            for (pin_id, pin) in &symbol.pins {
                let np = NetPin {
                    part: part_id,
                    pin: *pin_id,
                };
                if in_net.contains(&np) {
                    continue;
                }
                let Some(pad) = pad_of(&self.scratch, cid, &pin.number) else {
                    continue;
                };
                let current = self.scratch.net_of(pad);
                if current.is_none() {
                    continue;
                }
                let label = format!("{}.{}", part.refdes, pin.number);
                let from = net_name(&self.scratch, current);
                if self.push(Operation::SetPadNet { pad, net: None }, &label) {
                    self.changes
                        .push(Change::new(ChangeKind::PadNetChanged, &label, &from, "-"));
                }
            }
        }
    }

    fn missing(&mut self, board: &Board) {
        for (cid, component) in &board.components {
            let Some(part) = component.symbol else {
                continue;
            };
            if self.schematic.parts.contains_key(&part) {
                continue;
            }
            let to = if self.settings.remove_missing_components {
                "removed"
            } else {
                "kept"
            };
            self.changes.push(Change::new(
                ChangeKind::MissingPart,
                &component.refdes,
                "component",
                to,
            ));
            if self.settings.remove_missing_components {
                self.push(Operation::RemoveComponent { id: *cid }, &component.refdes);
            }
        }
    }
}

/// Plans the transfer of the schematic to the board. The plan is a dry run: neither document is
/// changed. `new_id` supplies the ids of the objects that the plan creates, so the same inputs and
/// the same supply of ids always give the same package.
pub fn plan_forward(
    schematic: &Schematic,
    board: &Board,
    settings: &SyncSettings,
    new_id: &mut dyn FnMut() -> Id,
) -> ForwardPackage {
    let mut component_of: BTreeMap<Id, Id> = BTreeMap::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    for (cid, component) in &board.components {
        let Some(part_id) = component.symbol else {
            continue;
        };
        if !schematic.parts.contains_key(&part_id) {
            continue;
        }
        if component_of.contains_key(&part_id) {
            skipped.push(Skipped::new(
                PART_TWICE,
                &component.refdes,
                "a second component of the same part is left alone",
            ));
        } else {
            component_of.insert(part_id, *cid);
        }
    }
    let mut planner = Planner {
        schematic,
        settings,
        scratch: board.clone(),
        operations: Vec::new(),
        changes: Vec::new(),
        skipped,
        component_of,
    };
    planner.new_parts(new_id);
    planner.fields();
    planner.nets(new_id);
    planner.missing(board);
    let mut changes = planner.changes;
    changes.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    ForwardPackage {
        changes,
        skipped: planner.skipped,
        operations: planner.operations,
    }
}
