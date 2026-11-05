// SPDX-License-Identifier: GPL-3.0-or-later
//! The connectivity of a board, and its comparison with the schematic.
//!
//! Two questions are answered, both with a report that keeps confirmed and inconclusive results
//! apart:
//!
//! * [`analyse_board`]: which copper really touches which. Pads, tracks, vias and zones are joined
//!   by geometry on a layer both are on, allowing a gap of the tolerance of the rule file. A group
//!   of joined copper that holds two nets is a short (`PHOENIX-CONN-004`); a net whose copper is in
//!   several groups still has connections to route (`PHOENIX-CONN-005`). A zone joins only the
//!   copper of its own net that lies inside its outline (the clearance of a zone cuts it away from
//!   other nets). Arcs are judged by their end points, which are copper: a contact found that way is
//!   proven, and an arc that may touch where its end points do not is said once, as inconclusive.
//!   Contacts that the shape of a pad is too coarse to tell are inconclusive, never a short.
//! * [`compare_schematic`]: the nets of the schematic against those of the board. Pins are matched
//!   by reference designator and pin number, so a net that was renamed is still found; names are
//!   compared afterwards. It finds nets missing from the board, pins that are not connected, a
//!   board net that joins the pins of two schematic nets, and differences of name.
//!
//! [`check_connectivity`] runs both and merges the reports.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_schematic::Schematic;

use crate::codes;
use crate::geometry::{contact, inside_polygon, polygons_touch, Contact, Prim};
use crate::items::{pad_prim, placed_pads, track_prim, via_prim};
use crate::report::{Finding, Report, Severity};

fn make(code: &'static str, rule: &'static str, severity: Severity, location: String) -> Finding {
    let rank = code
        .rsplit('-')
        .next()
        .and_then(|n| n.parse::<u8>().ok())
        .unwrap_or(0);
    Finding::new(rank, rule, code, severity, location)
}

fn net_name(board: &Board, id: Id) -> String {
    board
        .nets
        .get(&id)
        .map_or_else(|| id.to_string(), |n| n.name.clone())
}

/// A piece of copper in the graph of contacts.
struct Node {
    label: String,
    net: Option<Id>,
    layers: Vec<Id>,
    prim: Option<Prim>,
    /// Points that say whether the item lies inside a zone.
    probes: Vec<Point>,
    outline: Option<Vec<Point>>,
}

struct Sets {
    parent: Vec<usize>,
}

impl Sets {
    fn new(n: usize) -> Sets {
        Sets {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            let next = self.parent[self.parent[i]];
            self.parent[i] = next;
            i = next;
        }
        i
    }

    /// The smaller index stays the root, so the result does not depend on the order of the joins.
    fn join(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi] = lo;
        }
    }
}

fn nodes_of(board: &Board) -> Vec<Node> {
    let mut nodes = Vec::new();
    for pad in placed_pads(board) {
        let prim = pad_prim(&pad);
        nodes.push(Node {
            label: pad.label.clone(),
            net: pad.net,
            layers: pad.layers.clone(),
            prim,
            probes: vec![pad.at],
            outline: None,
        });
    }
    for (id, track) in &board.tracks {
        let mut probes = Vec::new();
        for segment in &track.path {
            probes.push(segment.start());
            probes.push(segment.end());
        }
        nodes.push(Node {
            label: format!("track {id}"),
            net: Some(track.net),
            layers: vec![track.layer],
            prim: track_prim(track),
            probes,
            outline: None,
        });
    }
    for (id, via) in &board.vias {
        nodes.push(Node {
            label: format!("via {id}"),
            net: Some(via.net),
            layers: vec![via.from_layer, via.to_layer],
            prim: via_prim(via),
            probes: vec![via.at],
            outline: None,
        });
    }
    for (id, zone) in &board.zones {
        nodes.push(Node {
            label: format!("zone {id}"),
            net: Some(zone.net),
            layers: vec![zone.layer],
            prim: None,
            probes: Vec::new(),
            outline: Some(zone.outline.clone()),
        });
    }
    nodes
}

fn shares_layer(a: &Node, b: &Node) -> bool {
    a.layers.iter().any(|l| b.layers.contains(l))
}

/// Which copper of the board touches which, from the geometry alone. `tolerance` is the gap that
/// still counts as touching.
pub fn analyse_board(board: &Board, tolerance: Nm) -> Report {
    let mut report = Report::new("connectivity");
    let nodes = nodes_of(board);
    let tol = i128::from(tolerance.get());
    let mut sets = Sets::new(nodes.len());
    let mut shorts: Vec<(usize, usize)> = Vec::new();
    let mut uncertain: BTreeSet<Id> = BTreeSet::new();

    for (i, a) in nodes.iter().enumerate() {
        let Some(pa) = &a.prim else {
            continue;
        };
        for (offset, b) in nodes[i + 1..].iter().enumerate() {
            let j = i + 1 + offset;
            let Some(pb) = &b.prim else {
                continue;
            };
            if !shares_layer(a, b) || !pa.may_be_within(pb, tol) {
                continue;
            }
            match contact(pa, pb, tol) {
                Contact::No => {}
                Contact::Yes => {
                    sets.join(i, j);
                    if let (Some(x), Some(y)) = (a.net, b.net) {
                        if x != y {
                            shorts.push((i, j));
                        }
                    }
                }
                Contact::Maybe | Contact::Unknown => match (a.net, b.net) {
                    (Some(x), Some(y)) if x != y => {
                        let at = pa.first_point();
                        report.inconclusive.push(
                            make(
                                codes::CONN_NOT_JUDGED,
                                "not_judged",
                                Severity::Info,
                                format!("{} and {}", a.label, b.label),
                            )
                            .at(at.x_nm.get(), at.y_nm.get())
                            .why("the shape of a pad or the size of the numbers does not tell whether they touch"),
                        );
                    }
                    (Some(x), Some(_)) => {
                        uncertain.insert(x);
                    }
                    _ => {}
                },
            }
        }
    }

    // zones join the copper of their own net that lies inside them
    for (zi, z) in nodes.iter().enumerate() {
        let (Some(outline), Some(net)) = (&z.outline, z.net) else {
            continue;
        };
        for (ni, n) in nodes.iter().enumerate() {
            if ni == zi || n.net != Some(net) || !shares_layer(z, n) {
                continue;
            }
            let touches = match &n.outline {
                Some(other) => polygons_touch(outline, other),
                None => n.probes.iter().any(|p| inside_polygon(*p, outline)),
            };
            if touches {
                sets.join(zi, ni);
            }
        }
    }

    // tracks with an arc: judged by their end points
    for node in &nodes {
        if node.prim.as_ref().is_some_and(|p| p.has_arc) {
            if let Some(net) = node.net {
                uncertain.insert(net);
            }
            report.inconclusive.push(
                make(
                    codes::CONN_NOT_JUDGED,
                    "not_judged",
                    Severity::Info,
                    node.label.clone(),
                )
                .why("the track has an arc: its contacts are judged by its end points only"),
            );
        }
    }

    // shorts: direct contacts of two nets, then groups that hold two nets without one
    let mut short_roots: BTreeSet<usize> = BTreeSet::new();
    for (i, j) in shorts {
        short_roots.insert(sets.find(i));
        let (a, b) = (&nodes[i], &nodes[j]);
        let names = [a.net, b.net]
            .iter()
            .flatten()
            .map(|n| net_name(board, *n))
            .collect::<Vec<_>>()
            .join(" and ");
        let mut finding = make(
            codes::COPPER_SHORT,
            "copper_short",
            Severity::Error,
            format!("{} and {}", a.label, b.label),
        )
        .why(&format!("nets {names}"));
        if let Some(p) = &a.prim {
            let at = p.first_point();
            finding = finding.at(at.x_nm.get(), at.y_nm.get());
        }
        report.confirmed.push(finding);
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..nodes.len() {
        groups.entry(sets.find(i)).or_default().push(i);
    }
    for (root, members) in &groups {
        if short_roots.contains(root) {
            continue;
        }
        let nets: BTreeSet<Id> = members.iter().filter_map(|i| nodes[*i].net).collect();
        if nets.len() > 1 {
            let names = nets
                .iter()
                .map(|n| net_name(board, *n))
                .collect::<Vec<_>>()
                .join(" and ");
            report.confirmed.push(
                make(
                    codes::COPPER_SHORT,
                    "copper_short",
                    Severity::Error,
                    format!("nets {names}"),
                )
                .why(&format!("{} items of copper are joined", members.len())),
            );
        }
    }

    // opens: a net whose copper is in more than one group
    let mut roots_of_net: BTreeMap<Id, BTreeSet<usize>> = BTreeMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if let Some(net) = node.net {
            roots_of_net.entry(net).or_default().insert(sets.find(i));
        }
    }
    for (net, roots) in &roots_of_net {
        if roots.len() < 2 {
            continue;
        }
        let finding = make(
            codes::NET_OPEN,
            "net_open",
            Severity::Error,
            format!("net {}", net_name(board, *net)),
        )
        .why(&format!(
            "{} separate groups of copper, {} connections still to route",
            roots.len(),
            roots.len() - 1
        ));
        if uncertain.contains(net) {
            report.inconclusive.push(
                make(
                    codes::NET_OPEN,
                    "net_open",
                    Severity::Info,
                    finding.location.clone(),
                )
                .why(&format!(
                    "{}; an arc or a pad shape may join them",
                    finding.detail
                )),
            );
        } else {
            report.confirmed.push(finding);
        }
    }
    report.finish();
    report
}

/// Compares the nets of a schematic with those of a board. Pins and pads are matched by reference
/// designator and number; the names of the nets are compared afterwards.
pub fn compare_schematic(schematic: &Schematic, board: &Board) -> Report {
    let mut report = Report::new("connectivity");

    // the schematic: the pins of each net as `REFDES.PIN`
    let mut sch_labels: BTreeMap<Id, BTreeSet<String>> = BTreeMap::new();
    let mut sch_of_label: BTreeMap<String, Vec<Id>> = BTreeMap::new();
    for (id, net) in &schematic.nets {
        let labels = sch_labels.entry(*id).or_default();
        for pin in &net.pins {
            let (Some(part), Some(number)) =
                (schematic.parts.get(&pin.part), schematic.pin_number(*pin))
            else {
                continue;
            };
            let label = format!("{}.{number}", part.refdes);
            labels.insert(label.clone());
            sch_of_label.entry(label).or_default().push(*id);
        }
    }
    let refdes_of_parts: BTreeSet<&str> = schematic
        .parts
        .values()
        .map(|p| p.refdes.as_str())
        .collect();

    // the board: the pad of each pin and its net
    let mut pad_labels: BTreeSet<String> = BTreeSet::new();
    let mut net_of_label: BTreeMap<String, Id> = BTreeMap::new();
    let mut board_labels: BTreeMap<Id, BTreeSet<String>> = BTreeMap::new();
    let mut refdes_of_label: BTreeMap<String, String> = BTreeMap::new();
    for pad in placed_pads(board) {
        pad_labels.insert(pad.pin_label.clone());
        refdes_of_label.insert(pad.pin_label.clone(), pad.refdes.clone());
        if let Some(net) = pad.net {
            net_of_label.entry(pad.pin_label.clone()).or_insert(net);
            board_labels
                .entry(net)
                .or_default()
                .insert(pad.pin_label.clone());
        }
    }
    let mut board_by_name: BTreeMap<String, Id> = BTreeMap::new();
    for (id, net) in &board.nets {
        board_by_name.entry(net.name.clone()).or_insert(*id);
    }

    // which schematic nets each board net holds
    let mut board_to_sch: BTreeMap<Id, BTreeSet<Id>> = BTreeMap::new();
    for (net, labels) in &board_labels {
        let held = board_to_sch.entry(*net).or_default();
        for label in labels {
            if let Some(list) = sch_of_label.get(label) {
                held.extend(list.iter().copied());
            }
        }
    }

    for (sid, labels) in &sch_labels {
        let name = schematic
            .nets
            .get(sid)
            .map_or_else(|| sid.to_string(), |n| n.name.clone());
        let mut on_board: Vec<&String> = Vec::new();
        for label in labels {
            if pad_labels.contains(label) {
                on_board.push(label);
            } else {
                report.confirmed.push(
                    make(
                        codes::PIN_UNCONNECTED,
                        "pin_unconnected",
                        Severity::Error,
                        format!("pin {label} of net {name}"),
                    )
                    .why("the board has no such pad"),
                );
            }
        }
        if labels.len() >= 2 {
            let mut used: BTreeMap<Id, usize> = BTreeMap::new();
            for label in &on_board {
                if let Some(net) = net_of_label.get(*label) {
                    *used.entry(*net).or_insert(0) += 1;
                }
            }
            let mut dominant: Option<(Id, usize)> = None;
            for (net, count) in &used {
                if dominant.is_none_or(|(_, best)| *count > best) {
                    dominant = Some((*net, *count));
                }
            }
            match dominant {
                None => report.confirmed.push(
                    make(
                        codes::NET_MISSING,
                        "net_missing",
                        Severity::Error,
                        format!("net {name}"),
                    )
                    .why(&format!(
                        "{} pins and none of them is on a net of the board",
                        labels.len()
                    )),
                ),
                Some((main, _)) => {
                    for label in &on_board {
                        match net_of_label.get(*label) {
                            None => report.confirmed.push(
                                make(
                                    codes::PIN_UNCONNECTED,
                                    "pin_unconnected",
                                    Severity::Error,
                                    format!("pin {label} of net {name}"),
                                )
                                .why("its pad is on no net of the board"),
                            ),
                            Some(other) if *other != main => report.confirmed.push(
                                make(
                                    codes::PIN_UNCONNECTED,
                                    "pin_unconnected",
                                    Severity::Error,
                                    format!("pin {label} of net {name}"),
                                )
                                .why(&format!(
                                    "its pad is on board net {} and not on {}",
                                    net_name(board, *other),
                                    net_name(board, main)
                                )),
                            ),
                            Some(_) => {}
                        }
                    }
                    let board_name = net_name(board, main);
                    let alone = board_to_sch
                        .get(&main)
                        .is_some_and(|held| held.len() == 1 && held.contains(sid));
                    if alone && board_name != name {
                        report.confirmed.push(
                            make(
                                codes::NAME_DIFFERS,
                                "name_differs",
                                Severity::Warning,
                                format!("net {name} and board net {board_name}"),
                            )
                            .why("the same pins have another name on the board"),
                        );
                    }
                }
            }
        }
        if let Some(bnet) = board_by_name.get(&name) {
            let empty = BTreeSet::new();
            let theirs = board_labels.get(bnet).unwrap_or(&empty);
            let only_sch = labels.difference(theirs).count();
            let only_board = theirs.difference(labels).count();
            if only_sch + only_board > 0 {
                report.confirmed.push(
                    make(
                        codes::NAME_SAME_PINS_DIFFER,
                        "name_same_pins_differ",
                        Severity::Warning,
                        format!("net {name}"),
                    )
                    .why(&format!(
                        "{only_sch} pins only in the schematic, {only_board} pads only on the board"
                    )),
                );
            }
        }
    }

    for (bnet, held) in &board_to_sch {
        let bname = net_name(board, *bnet);
        if held.len() > 1 {
            let names = held
                .iter()
                .map(|s| {
                    schematic
                        .nets
                        .get(s)
                        .map_or_else(|| s.to_string(), |n| n.name.clone())
                })
                .collect::<Vec<_>>()
                .join(", ");
            report.confirmed.push(
                make(
                    codes::NETS_JOINED,
                    "nets_joined",
                    Severity::Error,
                    format!("board net {bname}"),
                )
                .why(&format!("it holds pins of the schematic nets {names}")),
            );
        }
        if let Some(labels) = board_labels.get(bnet) {
            for label in labels {
                let part_known = refdes_of_label
                    .get(label)
                    .is_some_and(|r| refdes_of_parts.contains(r.as_str()));
                if part_known && !sch_of_label.contains_key(label) {
                    report.confirmed.push(
                        make(
                            codes::PAD_ONLY_ON_BOARD,
                            "pad_only_on_board",
                            Severity::Warning,
                            format!("pad {label} on board net {bname}"),
                        )
                        .why("the pin is on no net of the schematic"),
                    );
                }
            }
        }
    }

    report.finish();
    report
}

/// Both checks in one report: the copper of the board, and the board against the schematic.
pub fn check_connectivity(schematic: &Schematic, board: &Board, tolerance: Nm) -> Report {
    let mut report = analyse_board(board, tolerance);
    report.merge(compare_schematic(schematic, board));
    report
}
