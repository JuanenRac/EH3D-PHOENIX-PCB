//! Reversible operations on a board and the undo/redo history.
//!
//! Every change goes through an [`Operation`]. Applying one returns its exact inverse, built from
//! the state before the change, so undoing is applying the inverse and the board comes back byte for
//! byte. The history is a plain list that can be written and replayed: the same board and the same
//! list always give the same result. Nothing here reads the clock or makes ids; the caller supplies
//! them.

use eh3d_phoenix_model::{Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::model::{Board, Component, Footprint, Id, LayerKind, PadNet, PadRef, Track, Via, Zone};
use crate::silk::SilkItem;

/// A change to a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Adds a net.
    AddNet {
        /// The new id.
        id: Id,
        /// The name.
        name: String,
    },
    /// Removes a net that nothing uses.
    RemoveNet {
        /// The net.
        id: Id,
    },
    /// Renames a net.
    RenameNet {
        /// The net.
        id: Id,
        /// The new name.
        name: String,
    },
    /// Adds a component.
    AddComponent {
        /// The new id.
        id: Id,
        /// The component.
        component: Component,
    },
    /// Removes a component together with the net assignments of its pads.
    RemoveComponent {
        /// The component.
        id: Id,
    },
    /// Moves and turns a component.
    MoveComponent {
        /// The component.
        id: Id,
        /// The new place.
        at: Point,
        /// The new rotation.
        rotation: Udeg,
    },
    /// Changes the reference designator, the value and the footprint of a component; its place,
    /// rotation, side and link to a part of the schematic stay. A component whose pads have nets
    /// keeps its footprint (the nets are removed first, then the footprint changes).
    SetComponentFields {
        /// The component.
        id: Id,
        /// The new reference designator.
        refdes: String,
        /// The new value.
        value: String,
        /// The new footprint, which the board must have.
        footprint: Id,
    },
    /// Gives a pad a net, or none.
    SetPadNet {
        /// The pad.
        pad: PadRef,
        /// The net, or none to disconnect.
        net: Option<Id>,
    },
    /// Adds a track.
    AddTrack {
        /// The new id.
        id: Id,
        /// The track.
        track: Track,
    },
    /// Removes a track.
    RemoveTrack {
        /// The track.
        id: Id,
    },
    /// Adds a via.
    AddVia {
        /// The new id.
        id: Id,
        /// The via.
        via: Via,
    },
    /// Removes a via.
    RemoveVia {
        /// The via.
        id: Id,
    },
    /// Adds a copper zone, a closed polygon of copper of one net on one layer.
    AddZone {
        /// The new id.
        id: Id,
        /// The zone.
        zone: Zone,
    },
    /// Removes a zone.
    RemoveZone {
        /// The zone.
        id: Id,
    },
    /// Adds a footprint to the library of the board; its pads name layers the board has.
    AddFootprint {
        /// The new id.
        id: Id,
        /// The footprint.
        footprint: Footprint,
    },
    /// Removes a footprint that no component uses.
    RemoveFootprint {
        /// The footprint.
        id: Id,
    },
    /// Gives the ordered stack of copper layers, from the top of the board to the bottom.
    SetCopperStack {
        /// The copper layers in order; empty to say that the order is not given.
        stack: Vec<Id>,
    },
    /// Adds a text or stroke to a silkscreen layer.
    AddSilk {
        /// The new id.
        id: Id,
        /// The item.
        item: SilkItem,
    },
    /// Removes a silkscreen item.
    RemoveSilk {
        /// The item.
        id: Id,
    },
    /// Replaces the board outline, a closed polygon (the last corner joins the first).
    SetOutline {
        /// The corners.
        outline: Vec<Point>,
    },
    /// Several operations that are applied and undone as one.
    Batch {
        /// The operations, in order.
        ops: Vec<Operation>,
    },
}

/// Why an operation could not be applied. The board is left as it was.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    /// The part to add already exists.
    #[error("the id {0} is already used")]
    Exists(Id),
    /// The part to change does not exist.
    #[error("the id {0} does not exist")]
    Missing(Id),
    /// Something still uses the part.
    #[error("the id {0} is still in use")]
    InUse(Id),
    /// The result would refer to a part that does not exist.
    #[error("the operation refers to a part that does not exist: {0}")]
    Dangling(Id),
    /// A layer of the copper stack is not a copper layer.
    #[error("the layer {0} is not a copper layer")]
    NotCopper(Id),
    /// The layer of a silkscreen item is not a silkscreen layer.
    #[error("the layer {0} is not a silkscreen layer")]
    NotSilkscreen(Id),
}

fn set_pad_net(board: &mut Board, pad: PadRef, net: Option<Id>) -> Result<Operation, OpError> {
    if !board.components.contains_key(&pad.component) {
        return Err(OpError::Missing(pad.component));
    }
    let found = board.pad_nets.binary_search_by(|p| p.pad_ref().cmp(&pad));
    let before = found.ok().map(|i| board.pad_nets[i].net);
    if let Some(net) = net {
        if !board.nets.contains_key(&net) {
            return Err(OpError::Dangling(net));
        }
    }
    match (found, net) {
        (Ok(i), Some(net)) => board.pad_nets[i].net = net,
        (Ok(i), None) => {
            board.pad_nets.remove(i);
        }
        (Err(i), Some(net)) => board.pad_nets.insert(
            i,
            PadNet {
                component: pad.component,
                pad: pad.pad,
                net,
            },
        ),
        (Err(_), None) => {}
    }
    Ok(Operation::SetPadNet { pad, net: before })
}

impl Board {
    /// Applies `op` and returns the operation that undoes it. On an error the board is unchanged.
    pub fn apply(&mut self, op: &Operation) -> Result<Operation, OpError> {
        match op {
            Operation::AddNet { id, name } => {
                if self.nets.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                self.nets
                    .insert(*id, crate::model::Net { name: name.clone() });
                Ok(Operation::RemoveNet { id: *id })
            }
            Operation::RemoveNet { id } => {
                let net = self.nets.get(id).ok_or(OpError::Missing(*id))?.clone();
                let used = self.pad_nets.iter().any(|p| p.net == *id)
                    || self.tracks.values().any(|t| t.net == *id)
                    || self.vias.values().any(|v| v.net == *id)
                    || self.zones.values().any(|z| z.net == *id);
                if used {
                    return Err(OpError::InUse(*id));
                }
                self.nets.remove(id);
                Ok(Operation::AddNet {
                    id: *id,
                    name: net.name,
                })
            }
            Operation::RenameNet { id, name } => {
                let net = self.nets.get_mut(id).ok_or(OpError::Missing(*id))?;
                let old = std::mem::replace(&mut net.name, name.clone());
                Ok(Operation::RenameNet { id: *id, name: old })
            }
            Operation::AddComponent { id, component } => {
                if self.components.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                if !self.footprints.contains_key(&component.footprint) {
                    return Err(OpError::Dangling(component.footprint));
                }
                self.components.insert(*id, component.clone());
                Ok(Operation::RemoveComponent { id: *id })
            }
            Operation::RemoveComponent { id } => {
                let component = self
                    .components
                    .get(id)
                    .ok_or(OpError::Missing(*id))?
                    .clone();
                let mut undo: Vec<Operation> = vec![Operation::AddComponent { id: *id, component }];
                for p in self.pad_nets.iter().filter(|p| p.component == *id) {
                    undo.push(Operation::SetPadNet {
                        pad: p.pad_ref(),
                        net: Some(p.net),
                    });
                }
                self.pad_nets.retain(|p| p.component != *id);
                self.components.remove(id);
                Ok(Operation::Batch { ops: undo })
            }
            Operation::MoveComponent { id, at, rotation } => {
                let c = self.components.get_mut(id).ok_or(OpError::Missing(*id))?;
                let before = Operation::MoveComponent {
                    id: *id,
                    at: c.at,
                    rotation: c.rotation,
                };
                c.at = *at;
                c.rotation = *rotation;
                Ok(before)
            }
            Operation::SetComponentFields {
                id,
                refdes,
                value,
                footprint,
            } => {
                if !self.components.contains_key(id) {
                    return Err(OpError::Missing(*id));
                }
                if !self.footprints.contains_key(footprint) {
                    return Err(OpError::Dangling(*footprint));
                }
                let has_nets = self.pad_nets.iter().any(|p| p.component == *id);
                let c = self.components.get_mut(id).ok_or(OpError::Missing(*id))?;
                if c.footprint != *footprint && has_nets {
                    return Err(OpError::InUse(*id));
                }
                Ok(Operation::SetComponentFields {
                    id: *id,
                    refdes: std::mem::replace(&mut c.refdes, refdes.clone()),
                    value: std::mem::replace(&mut c.value, value.clone()),
                    footprint: std::mem::replace(&mut c.footprint, *footprint),
                })
            }
            Operation::SetPadNet { pad, net } => set_pad_net(self, *pad, *net),
            Operation::AddTrack { id, track } => {
                if self.tracks.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                if !self.nets.contains_key(&track.net) {
                    return Err(OpError::Dangling(track.net));
                }
                if !self.layers.contains_key(&track.layer) {
                    return Err(OpError::Dangling(track.layer));
                }
                self.tracks.insert(*id, track.clone());
                Ok(Operation::RemoveTrack { id: *id })
            }
            Operation::RemoveTrack { id } => {
                let track = self.tracks.remove(id).ok_or(OpError::Missing(*id))?;
                Ok(Operation::AddTrack { id: *id, track })
            }
            Operation::AddVia { id, via } => {
                if self.vias.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                if !self.nets.contains_key(&via.net) {
                    return Err(OpError::Dangling(via.net));
                }
                for layer in [via.from_layer, via.to_layer] {
                    if !self.layers.contains_key(&layer) {
                        return Err(OpError::Dangling(layer));
                    }
                }
                self.vias.insert(*id, via.clone());
                Ok(Operation::RemoveVia { id: *id })
            }
            Operation::RemoveVia { id } => {
                let via = self.vias.remove(id).ok_or(OpError::Missing(*id))?;
                Ok(Operation::AddVia { id: *id, via })
            }
            Operation::AddZone { id, zone } => {
                if self.zones.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                if !self.nets.contains_key(&zone.net) {
                    return Err(OpError::Dangling(zone.net));
                }
                if !self.layers.contains_key(&zone.layer) {
                    return Err(OpError::Dangling(zone.layer));
                }
                self.zones.insert(*id, zone.clone());
                Ok(Operation::RemoveZone { id: *id })
            }
            Operation::RemoveZone { id } => {
                let zone = self.zones.remove(id).ok_or(OpError::Missing(*id))?;
                Ok(Operation::AddZone { id: *id, zone })
            }
            Operation::AddFootprint { id, footprint } => {
                if self.footprints.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                for pad in footprint.pads.values() {
                    if let Some(layer) = pad.layers.iter().find(|l| !self.layers.contains_key(*l)) {
                        return Err(OpError::Dangling(*layer));
                    }
                }
                self.footprints.insert(*id, footprint.clone());
                Ok(Operation::RemoveFootprint { id: *id })
            }
            Operation::RemoveFootprint { id } => {
                if !self.footprints.contains_key(id) {
                    return Err(OpError::Missing(*id));
                }
                if self.components.values().any(|c| c.footprint == *id) {
                    return Err(OpError::InUse(*id));
                }
                let footprint = self.footprints.remove(id).ok_or(OpError::Missing(*id))?;
                Ok(Operation::AddFootprint { id: *id, footprint })
            }
            Operation::SetCopperStack { stack } => {
                for (i, layer) in stack.iter().enumerate() {
                    match self.layers.get(layer) {
                        None => return Err(OpError::Dangling(*layer)),
                        Some(l) if l.kind != crate::model::LayerKind::Copper => {
                            return Err(OpError::NotCopper(*layer));
                        }
                        Some(_) => {}
                    }
                    if stack[..i].contains(layer) {
                        return Err(OpError::Exists(*layer));
                    }
                }
                let before = std::mem::replace(&mut self.copper_stack, stack.clone());
                Ok(Operation::SetCopperStack { stack: before })
            }
            Operation::AddSilk { id, item } => {
                if self.silkscreen.contains_key(id) {
                    return Err(OpError::Exists(*id));
                }
                match self.layers.get(&item.layer()) {
                    None => return Err(OpError::Dangling(item.layer())),
                    Some(l) if l.kind != LayerKind::Silkscreen => {
                        return Err(OpError::NotSilkscreen(item.layer()));
                    }
                    Some(_) => {}
                }
                self.silkscreen.insert(*id, item.clone());
                Ok(Operation::RemoveSilk { id: *id })
            }
            Operation::RemoveSilk { id } => {
                let item = self.silkscreen.remove(id).ok_or(OpError::Missing(*id))?;
                Ok(Operation::AddSilk { id: *id, item })
            }
            Operation::SetOutline { outline } => {
                // the inverse of a change from a board without an outline sets that back, so the
                // number of corners is not checked here; the planner of the editor checks it
                let before = std::mem::replace(&mut self.outline, outline.clone());
                Ok(Operation::SetOutline { outline: before })
            }
            Operation::Batch { ops } => {
                // All or nothing: on a failure everything applied so far is undone.
                let mut inverses: Vec<Operation> = Vec::with_capacity(ops.len());
                for op in ops {
                    match self.apply(op) {
                        Ok(inverse) => inverses.push(inverse),
                        Err(e) => {
                            for undo in inverses.iter().rev() {
                                self.apply(undo)
                                    .expect("the inverse of an applied operation applies");
                            }
                            return Err(e);
                        }
                    }
                }
                inverses.reverse();
                Ok(Operation::Batch { ops: inverses })
            }
        }
    }
}

/// One step of the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// What was done.
    pub op: Operation,
    /// What undoes it.
    pub inverse: Operation,
}

/// The undo/redo history of a board. It can be saved and replayed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct History {
    done: Vec<Step>,
    undone: Vec<Step>,
}

impl History {
    /// An empty history.
    pub fn new() -> History {
        History::default()
    }

    /// Applies `op` to the board and records it. A new change clears what could be redone.
    pub fn execute(&mut self, board: &mut Board, op: Operation) -> Result<(), OpError> {
        let inverse = board.apply(&op)?;
        self.done.push(Step { op, inverse });
        self.undone.clear();
        Ok(())
    }

    /// Undoes the last change; false when there is none.
    pub fn undo(&mut self, board: &mut Board) -> bool {
        let Some(step) = self.done.pop() else {
            return false;
        };
        board
            .apply(&step.inverse)
            .expect("the inverse of an applied operation applies");
        self.undone.push(step);
        true
    }

    /// Redoes the last undone change; false when there is none.
    pub fn redo(&mut self, board: &mut Board) -> bool {
        let Some(step) = self.undone.pop() else {
            return false;
        };
        board
            .apply(&step.op)
            .expect("an undone operation applies again");
        self.done.push(step);
        true
    }

    /// The changes that are done, oldest first.
    pub fn done(&self) -> &[Step] {
        &self.done
    }

    /// The changes that were undone and can be redone, the next one to redo last.
    pub fn undone(&self) -> &[Step] {
        &self.undone
    }

    /// Applies the recorded operations, in order, to another board. Used to replay a history on
    /// the board it started from.
    pub fn replay(&self, board: &mut Board) -> Result<(), OpError> {
        for step in &self.done {
            board.apply(&step.op)?;
        }
        Ok(())
    }

    /// The canonical bytes of the history.
    pub fn to_bytes(&self) -> Result<Vec<u8>, eh3d_container::CanonicalError> {
        eh3d_container::to_canonical_file_bytes(self)
    }

    /// Reads a history.
    pub fn from_bytes(bytes: &[u8]) -> Result<History, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
