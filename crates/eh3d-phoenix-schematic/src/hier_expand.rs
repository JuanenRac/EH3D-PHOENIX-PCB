//! Expanding the hierarchy: the same schematic with every instance of every sheet drawn out, as a
//! flat schematic whose parts have hierarchical references (`U1/R1`) and whose nets have
//! hierarchical names (`/U1/VOUT`), so that the netlist of the simulator and the rule check can
//! walk it like any other schematic.
//!
//! The rules, kept simple so the result is always the same:
//!
//! * a sheet that no sheet symbol shows is a top sheet and is kept as it is; a sheet shown by a
//!   symbol is copied once per symbol (and once per copy of the sheet that holds the symbol), with
//!   new ids that depend only on the schematic and on the order of the instances;
//! * the reference of a part of an instance is made from the names of the sheet symbols and the
//!   reference on the sheet, as the settings say (`U1/R1` or `R1_U1`);
//! * a net label is local to its instance (the net is named `/U1/NAME`; on a top sheet `/NAME`)
//!   unless its name is global in the settings or it is a power port, which is always global; with
//!   `local_labels = false` every label is global;
//! * a pin of a sheet symbol and the port of the same name on the sheet it shows are one
//!   connection: both get the same hidden label, so the nets on the two sides are one net;
//! * the name of a net is the shallowest name that a label or a power port gives it; a net that
//!   only a pin and a port name is called `/U1/PORT`; a net no name reaches keeps its name, behind
//!   the name of its instance;
//! * the connections that only the net list gave (pins with no wire) are kept inside the instance
//!   they are in; a connection that the net list gave between two sheets is not kept, because it
//!   is the sheet symbols that say how sheets are connected.
//!
//! A schematic without sheet symbols is not changed: its sheets are all top sheets and its labels
//! stay global, as they always were.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};

use crate::draw::{Sheet, Wire};
use crate::edit::{EditError, IdSource};
use crate::hier::{hier_diagnostic, HierConfig, Hierarchy, RefdesStyle, REFDES_REPEATED};
use crate::marks::{Junction, NetLabel, PowerPort};
use crate::model::{Id, NetPin, Schematic, SchematicNet};

/// What starts the name of the hidden label that joins a pin of a sheet symbol and its port.
const LINK_MARK: &str = "\u{1}";

/// A schematic with its hierarchy drawn out.
#[derive(Debug, Clone)]
pub struct Expanded {
    /// The flat schematic: parts with hierarchical references, nets with hierarchical names.
    pub schematic: Schematic,
    /// The instances of the sheets, with the loops and limits found while walking the tree.
    pub hierarchy: Hierarchy,
    /// For each part of the flat schematic: the instance it is in (an index into
    /// `hierarchy.instances`) and the part it was copied from.
    pub origin: BTreeMap<Id, (usize, Id)>,
    /// For each net of the flat schematic: the instances where it has a pin.
    pub net_instances: BTreeMap<Id, BTreeSet<usize>>,
    /// The reference on the sheet (`R1`) of every reference of the flat schematic (`U1/R1`), for
    /// the checks that look at the letters of a designator. Empty for a schematic that has no
    /// sheet symbol.
    pub designators: BTreeMap<String, String>,
    /// What the instance of a top sheet is called in a report: the separator of the net names.
    pub top_name: String,
    /// The problems of the expansion: loops, limits and references that repeat.
    pub problems: Vec<Diagnostic>,
}

/// A net with the instances it is in, for a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetRow {
    /// The hierarchical name.
    pub name: String,
    /// How many pins it has.
    pub pins: usize,
    /// The names of the instances where it has a pin; a top sheet is shown as the separator.
    pub instances: Vec<String>,
}

/// The reference of a part of the instance with the given path.
pub fn refdes_in(cfg: &HierConfig, path: &[String], refdes: &str) -> String {
    if path.is_empty() {
        return refdes.to_owned();
    }
    let joined = path.join(&cfg.refdes_separator);
    match cfg.refdes_style {
        RefdesStyle::Prefix => format!("{joined}{}{refdes}", cfg.refdes_separator),
        RefdesStyle::Suffix => format!("{refdes}{}{joined}", cfg.refdes_separator),
    }
}

fn scoped(cfg: &HierConfig, instance: &str, name: &str) -> String {
    if !cfg.local_labels || cfg.is_global(name) {
        name.to_owned()
    } else {
        format!("{instance}{}{name}", cfg.net_separator)
    }
}

impl Schematic {
    /// Draws out the hierarchy. See the module for the rules.
    pub fn expand(&self, cfg: &HierConfig) -> Result<Expanded, EditError> {
        let hierarchy = self.hierarchy(cfg);
        if !self.has_hierarchy() {
            return Ok(self.unchanged(cfg, hierarchy));
        }
        let mut ids = IdSource::for_schematic(self);
        let mut out = Schematic {
            name: self.name.clone(),
            symbols: self.symbols.clone(),
            pin_kinds: self.pin_kinds.clone(),
            ..Schematic::default()
        };
        let count = hierarchy.instances.len();

        let mut new_sheet: Vec<Id> = Vec::with_capacity(count);
        for inst in &hierarchy.instances {
            let id = if inst.parent.is_none() {
                inst.sheet
            } else {
                ids.fresh()?
            };
            if let Some(src) = self.sheets.get(&inst.sheet) {
                let name = if inst.parent.is_none() {
                    src.name.clone()
                } else {
                    inst.name.clone()
                };
                out.sheets.insert(
                    id,
                    Sheet {
                        name,
                        width: src.width,
                        height: src.height,
                    },
                );
            }
            new_sheet.push(id);
        }

        let first_top = hierarchy.instances.iter().position(|i| i.parent.is_none());
        let entry_marks = self.bus_entry_marks();
        let mut part_map: Vec<BTreeMap<Id, Id>> = vec![BTreeMap::new(); count];
        let mut origin: BTreeMap<Id, (usize, Id)> = BTreeMap::new();
        let mut designators: BTreeMap<String, String> = BTreeMap::new();
        for (idx, inst) in hierarchy.instances.iter().enumerate() {
            let top = inst.parent.is_none();
            let sheet = new_sheet[idx];
            for (pid, part) in &self.parts {
                let here = match part.placement {
                    Some(place) => place.sheet == inst.sheet,
                    None => Some(idx) == first_top,
                };
                if !here {
                    continue;
                }
                let new_id = if top { *pid } else { ids.fresh()? };
                let mut copy = part.clone();
                copy.refdes = refdes_in(cfg, &inst.path, &part.refdes);
                designators.insert(copy.refdes.clone(), part.refdes.clone());
                if let Some(place) = copy.placement.as_mut() {
                    place.sheet = sheet;
                }
                out.parts.insert(new_id, copy);
                if self.mirrored.contains(pid) {
                    out.mirrored.insert(new_id);
                }
                part_map[idx].insert(*pid, new_id);
                origin.insert(new_id, (idx, *pid));
            }
            for (wid, wire) in self.wires.iter().filter(|(_, w)| w.sheet == inst.sheet) {
                let new_id = if top { *wid } else { ids.fresh()? };
                out.wires.insert(
                    new_id,
                    Wire {
                        sheet,
                        path: wire.path.clone(),
                    },
                );
            }
            for (jid, junction) in self.junctions.iter().filter(|(_, j)| j.sheet == inst.sheet) {
                let new_id = if top { *jid } else { ids.fresh()? };
                out.junctions.insert(
                    new_id,
                    Junction {
                        sheet,
                        at: junction.at,
                    },
                );
            }
            for (lid, label) in self.labels.iter().filter(|(_, l)| l.sheet == inst.sheet) {
                let new_id = if top { *lid } else { ids.fresh()? };
                out.labels.insert(
                    new_id,
                    NetLabel {
                        sheet,
                        at: label.at,
                        name: scoped(cfg, &inst.name, &label.name),
                    },
                );
            }
            for (pid, port) in self
                .power_ports
                .iter()
                .filter(|(_, p)| p.sheet == inst.sheet)
            {
                let new_id = if top { *pid } else { ids.fresh()? };
                out.power_ports.insert(
                    new_id,
                    PowerPort {
                        sheet,
                        at: port.at,
                        name: port.name.clone(),
                    },
                );
            }
            for (_, at, member) in entry_marks.iter().filter(|(s, _, _)| *s == inst.sheet) {
                out.labels.insert(
                    ids.fresh()?,
                    NetLabel {
                        sheet,
                        at: *at,
                        name: scoped(cfg, &inst.name, member),
                    },
                );
            }
        }

        // a pin of a sheet symbol and the port of the sheet it shows: the same hidden label
        for (idx, inst) in hierarchy.instances.iter().enumerate() {
            let (Some(parent), Some(block)) = (inst.parent, inst.block) else {
                continue;
            };
            let link = |name: &str| format!("{LINK_MARK}{}{}{name}", inst.name, cfg.net_separator);
            for pin in self.hier_pins.values().filter(|p| p.block == block) {
                out.labels.insert(
                    ids.fresh()?,
                    NetLabel {
                        sheet: new_sheet[parent],
                        at: pin.at,
                        name: link(&pin.name),
                    },
                );
            }
            for port in self.sheet_ports.values().filter(|p| p.sheet == inst.sheet) {
                out.labels.insert(
                    ids.fresh()?,
                    NetLabel {
                        sheet: new_sheet[idx],
                        at: port.at,
                        name: link(&port.name),
                    },
                );
            }
        }

        // the connections of the net list, inside each instance
        for map in &part_map {
            for net in self.nets.values() {
                let mut pins: Vec<NetPin> = net
                    .pins
                    .iter()
                    .filter_map(|p| {
                        map.get(&p.part).map(|new_part| NetPin {
                            part: *new_part,
                            pin: p.pin,
                        })
                    })
                    .collect();
                if pins.len() < 2 {
                    continue;
                }
                pins.sort();
                out.nets.insert(
                    ids.fresh()?,
                    SchematicNet {
                        name: net.name.clone(),
                        pins,
                    },
                );
            }
        }
        crate::nets::rebuild_nets(&mut out, &mut ids, &[])?;
        name_nets(&mut out, cfg, &hierarchy, &origin);

        let mut net_instances: BTreeMap<Id, BTreeSet<usize>> = BTreeMap::new();
        for (nid, net) in &out.nets {
            let set = net_instances.entry(*nid).or_default();
            for pin in &net.pins {
                if let Some((idx, _)) = origin.get(&pin.part) {
                    set.insert(*idx);
                }
            }
        }

        let mut problems = hierarchy.problems.clone();
        let mut seen: BTreeMap<&str, Id> = BTreeMap::new();
        for (id, part) in &out.parts {
            if let Some(first) = seen.insert(part.refdes.as_str(), *id) {
                problems.push(hier_diagnostic(
                    REFDES_REPEATED,
                    Severity::Error,
                    format!("{} is the reference of {first} and {id}", part.refdes),
                ));
            }
        }
        Ok(Expanded {
            schematic: out,
            hierarchy,
            origin,
            net_instances,
            designators,
            top_name: cfg.net_separator.clone(),
            problems,
        })
    }

    /// The expansion of a schematic that has no sheet symbol: the schematic as it is.
    fn unchanged(&self, cfg: &HierConfig, hierarchy: Hierarchy) -> Expanded {
        let sheet_instance: BTreeMap<Id, usize> = hierarchy
            .instances
            .iter()
            .enumerate()
            .map(|(i, inst)| (inst.sheet, i))
            .collect();
        let instance_of = |part: &Id| -> usize {
            self.parts
                .get(part)
                .and_then(|p| p.placement)
                .and_then(|place| sheet_instance.get(&place.sheet).copied())
                .unwrap_or(0)
        };
        let origin: BTreeMap<Id, (usize, Id)> = self
            .parts
            .keys()
            .map(|id| (*id, (instance_of(id), *id)))
            .collect();
        let net_instances: BTreeMap<Id, BTreeSet<usize>> = self
            .nets
            .iter()
            .map(|(id, net)| (*id, net.pins.iter().map(|p| instance_of(&p.part)).collect()))
            .collect();
        let problems = hierarchy.problems.clone();
        Expanded {
            schematic: self.clone(),
            hierarchy,
            origin,
            net_instances,
            designators: BTreeMap::new(),
            top_name: cfg.net_separator.clone(),
            problems,
        }
    }
}

/// Names the nets of an expanded schematic: the shallowest name that a label or a power port gives,
/// else the name of the pin and port that join it, else its own name behind the name of its
/// instance.
fn name_nets(
    s: &mut Schematic,
    cfg: &HierConfig,
    hierarchy: &Hierarchy,
    origin: &BTreeMap<Id, (usize, Id)>,
) {
    let drawn = s.drawn();
    let group_of: BTreeMap<NetPin, usize> = drawn
        .groups
        .iter()
        .enumerate()
        .flat_map(|(i, g)| g.iter().map(move |p| (*p, i)))
        .collect();
    let separator = cfg.net_separator.as_str();
    let mut renames: Vec<(Id, String)> = Vec::new();
    for (nid, net) in &s.nets {
        let mut names: BTreeSet<&String> = BTreeSet::new();
        for pin in &net.pins {
            if let Some(group) = group_of.get(pin).and_then(|g| drawn.group_names.get(*g)) {
                names.extend(group.iter());
            }
        }
        let best = names.iter().copied().min_by_key(|name| {
            (
                name.starts_with(LINK_MARK),
                name.matches(separator).count(),
                (*name).clone(),
            )
        });
        let name = match best {
            Some(name) => name.trim_start_matches(LINK_MARK).to_owned(),
            None => {
                let instance = net
                    .pins
                    .iter()
                    .find_map(|p| origin.get(&p.part))
                    .and_then(|(idx, _)| hierarchy.instances.get(*idx))
                    .map_or("", |inst| inst.name.as_str());
                format!("{instance}{separator}{}", net.name)
            }
        };
        renames.push((*nid, name));
    }
    for (nid, name) in renames {
        if let Some(net) = s.nets.get_mut(&nid) {
            net.name = name;
        }
    }
}

impl Expanded {
    /// The nets of the expanded schematic with the instances they are in, sorted by name.
    pub fn net_rows(&self) -> Vec<NetRow> {
        let mut rows: Vec<NetRow> = self
            .schematic
            .nets
            .iter()
            .map(|(id, net)| {
                let mut instances: Vec<String> = self
                    .net_instances
                    .get(id)
                    .map(|set| {
                        set.iter()
                            .filter_map(|i| self.hierarchy.instances.get(*i))
                            .map(|inst| {
                                if inst.name.is_empty() {
                                    self.top_name.clone()
                                } else {
                                    inst.name.clone()
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                instances.sort();
                instances.dedup();
                NetRow {
                    name: net.name.clone(),
                    pins: net.pins.len(),
                    instances,
                }
            })
            .collect();
        rows.sort_by(|a, b| (&a.name, a.pins).cmp(&(&b.name, b.pins)));
        rows
    }
}
