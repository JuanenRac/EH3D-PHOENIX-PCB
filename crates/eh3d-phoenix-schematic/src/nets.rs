//! The net list that follows the drawing: after an edit that changes wires, labels, power ports or
//! the places of pins, the nets are made again from what the drawing joins.
//!
//! The rules, kept simple so the result is always the same:
//!
//! * pins that wires join, or that carry the same label or power port name, are on one net;
//! * a connection that only the net list gave (pins that no wire joined before the edit) is kept:
//!   one pin of each piece of a net stays joined to the others;
//! * a net keeps its identity: a group of pins takes the old net that holds most of its pins (the
//!   smallest id on a tie), and a group that no old net claims gets a new id;
//! * the name of a group is the smallest label or power port name on it; without one, the group keeps
//!   the name of its old net, or gets `N$1`, `N$2`, ... that no net has;
//! * a lone pin has no net, except a pin whose old net was a single pin already.

use std::cmp::Reverse;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use crate::edit::{EditError, IdSource};
use crate::model::{Id, NetPin, Schematic, SchematicNet};

struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Dsu {
        Dsu {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[a.max(b)] = a.min(b);
        }
    }
}

fn slot(index: &mut BTreeMap<NetPin, usize>, pin: NetPin) -> usize {
    let next = index.len();
    *index.entry(pin).or_insert(next)
}

/// Makes the nets of `s` again from its drawing. `before` are the groups of pins that the drawing
/// joined before the edit (`Schematic::drawn().groups`); they tell a connection of the drawing,
/// which the drawing decides again, from one that only the net list gave.
pub(crate) fn rebuild_nets(
    s: &mut Schematic,
    ids: &mut IdSource,
    before: &[BTreeSet<NetPin>],
) -> Result<(), EditError> {
    let drawn = s.drawn();

    let mut index: BTreeMap<NetPin, usize> = BTreeMap::new();
    for group in &drawn.groups {
        for pin in group {
            slot(&mut index, *pin);
        }
    }
    for net in s.nets.values() {
        for pin in &net.pins {
            if s.pin_number(*pin).is_some() {
                slot(&mut index, *pin);
            }
        }
    }
    let mut dsu = Dsu::new(index.len());
    for group in &drawn.groups {
        let mut pins = group.iter();
        if let Some(first) = pins.next() {
            let a = index[first];
            for pin in pins {
                dsu.join(a, index[pin]);
            }
        }
    }
    let piece_of: BTreeMap<NetPin, usize> = before
        .iter()
        .enumerate()
        .flat_map(|(i, group)| group.iter().map(move |p| (*p, i)))
        .collect();
    for net in s.nets.values() {
        let mut reps: BTreeMap<(u8, usize), NetPin> = BTreeMap::new();
        for pin in &net.pins {
            let Some(&i) = index.get(pin) else {
                continue;
            };
            let key = match piece_of.get(pin) {
                Some(piece) => (0u8, *piece),
                None => (1u8, i),
            };
            reps.entry(key)
                .and_modify(|q| {
                    if pin < q {
                        *q = *pin;
                    }
                })
                .or_insert(*pin);
        }
        let mut reps = reps.values();
        if let Some(first) = reps.next() {
            for pin in reps {
                dsu.join(index[first], index[pin]);
            }
        }
    }

    let mut classes: BTreeMap<usize, BTreeSet<NetPin>> = BTreeMap::new();
    for (pin, i) in &index {
        classes.entry(dsu.find(*i)).or_default().insert(*pin);
    }
    let mut class_names: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (group, names) in drawn.groups.iter().zip(&drawn.group_names) {
        if let Some(first) = group.iter().next() {
            class_names
                .entry(dsu.find(index[first]))
                .or_default()
                .extend(names.iter().cloned());
        }
    }

    // which old net each class continues
    let old_net_of: BTreeMap<NetPin, Id> = s
        .nets
        .iter()
        .flat_map(|(id, n)| n.pins.iter().map(move |p| (*p, *id)))
        .collect();
    let mut votes: Vec<(Reverse<usize>, Id, usize)> = Vec::new();
    for (root, pins) in &classes {
        let mut count: BTreeMap<Id, usize> = BTreeMap::new();
        for pin in pins {
            if let Some(net) = old_net_of.get(pin) {
                *count.entry(*net).or_insert(0) += 1;
            }
        }
        for (net, c) in count {
            votes.push((Reverse(c), net, *root));
        }
    }
    votes.sort();
    let mut class_net: BTreeMap<usize, Id> = BTreeMap::new();
    let mut claimed: BTreeSet<Id> = BTreeSet::new();
    for (_, net, root) in votes {
        if let Entry::Vacant(slot) = class_net.entry(root) {
            if claimed.insert(net) {
                slot.insert(net);
            }
        }
    }

    let mut nets: BTreeMap<Id, SchematicNet> = BTreeMap::new();
    for (root, pins) in &classes {
        let label = class_names
            .get(root)
            .and_then(|names| names.iter().next())
            .cloned();
        let alone_before = pins.len() == 1
            && pins
                .iter()
                .next()
                .and_then(|p| old_net_of.get(p))
                .and_then(|n| s.nets.get(n))
                .is_some_and(|net| net.pins.len() == 1);
        if pins.len() < 2 && label.is_none() && !alone_before {
            continue;
        }
        let id = match class_net.get(root) {
            Some(id) => *id,
            None => ids.fresh()?,
        };
        let name = match (label, s.nets.get(&id)) {
            (Some(label), _) => label,
            (None, Some(old)) => old.name.clone(),
            (None, None) => String::new(),
        };
        nets.insert(
            id,
            SchematicNet {
                name,
                pins: pins.iter().copied().collect(),
            },
        );
    }
    let nameless: Vec<Id> = nets
        .iter()
        .filter(|(_, n)| n.name.trim().is_empty())
        .map(|(id, _)| *id)
        .collect();
    let mut counter = 0usize;
    for id in nameless {
        loop {
            counter += 1;
            let candidate = format!("N${counter}");
            if !nets.values().any(|n| n.name == candidate) {
                if let Some(net) = nets.get_mut(&id) {
                    net.name = candidate;
                }
                break;
            }
        }
    }
    s.nets = nets;
    Ok(())
}
