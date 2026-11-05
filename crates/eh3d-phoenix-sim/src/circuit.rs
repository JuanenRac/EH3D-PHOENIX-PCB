// SPDX-License-Identifier: GPL-3.0-or-later
//! The circuit ready to be solved: the subcircuits expanded, the nodes numbered (node 0 is
//! ground) and the elements sorted by kind. The unknowns of the equations are the voltages of
//! the nodes other than ground, in order of first use, followed by the currents of the branches
//! (voltage sources, inductors, and the voltage-controlled and current-controlled voltage
//! sources).

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    SimError, DUPLICATE_NAME, FLOATING_NODE, LIMIT_EXCEEDED, NO_GROUND, SUBCKT_INVALID, UNDEFINED,
};
use crate::netlist::{Element, Kind, Netlist, Source};
use crate::settings::SimSettings;

/// A resistor.
#[derive(Debug, Clone, PartialEq)]
pub struct Resistor {
    /// The name.
    pub name: String,
    /// The first node.
    pub a: usize,
    /// The second node.
    pub b: usize,
    /// The resistance in ohm.
    pub ohms: f64,
}

/// A capacitor.
#[derive(Debug, Clone, PartialEq)]
pub struct Capacitor {
    /// The name.
    pub name: String,
    /// The first node.
    pub a: usize,
    /// The second node.
    pub b: usize,
    /// The capacitance in farad.
    pub farads: f64,
    /// The initial voltage.
    pub ic: Option<f64>,
}

/// An inductor.
#[derive(Debug, Clone, PartialEq)]
pub struct Inductor {
    /// The name.
    pub name: String,
    /// The first node.
    pub a: usize,
    /// The second node.
    pub b: usize,
    /// The inductance in henry.
    pub henries: f64,
    /// The initial current.
    pub ic: Option<f64>,
    /// The branch of its current.
    pub branch: usize,
    /// The inductors coupled with this one: their index and the mutual inductance in henry.
    pub partners: Vec<(usize, f64)>,
}

/// An independent voltage source.
#[derive(Debug, Clone, PartialEq)]
pub struct VSource {
    /// The name.
    pub name: String,
    /// The positive node.
    pub a: usize,
    /// The negative node.
    pub b: usize,
    /// The branch of its current.
    pub branch: usize,
    /// The value.
    pub source: Source,
}

/// An independent current source.
#[derive(Debug, Clone, PartialEq)]
pub struct ISource {
    /// The name.
    pub name: String,
    /// The node the current leaves.
    pub a: usize,
    /// The node the current enters.
    pub b: usize,
    /// The value.
    pub source: Source,
}

/// A voltage-controlled voltage source.
#[derive(Debug, Clone, PartialEq)]
pub struct Vcvs {
    /// The name.
    pub name: String,
    /// The positive output node.
    pub a: usize,
    /// The negative output node.
    pub b: usize,
    /// The positive control node.
    pub cp: usize,
    /// The negative control node.
    pub cn: usize,
    /// The gain.
    pub gain: f64,
    /// The branch of its current.
    pub branch: usize,
}

/// A voltage-controlled current source.
#[derive(Debug, Clone, PartialEq)]
pub struct Vccs {
    /// The name.
    pub name: String,
    /// The node the current leaves.
    pub a: usize,
    /// The node the current enters.
    pub b: usize,
    /// The positive control node.
    pub cp: usize,
    /// The negative control node.
    pub cn: usize,
    /// The transconductance in siemens.
    pub gm: f64,
}

/// A current-controlled current source.
#[derive(Debug, Clone, PartialEq)]
pub struct Cccs {
    /// The name.
    pub name: String,
    /// The node the current leaves.
    pub a: usize,
    /// The node the current enters.
    pub b: usize,
    /// The branch of the controlling current.
    pub control: usize,
    /// The gain.
    pub gain: f64,
}

/// A current-controlled voltage source.
#[derive(Debug, Clone, PartialEq)]
pub struct Ccvs {
    /// The name.
    pub name: String,
    /// The positive output node.
    pub a: usize,
    /// The negative output node.
    pub b: usize,
    /// The branch of the controlling current.
    pub control: usize,
    /// The transresistance in ohm.
    pub rm: f64,
    /// The branch of its own current.
    pub branch: usize,
}

/// A diode.
#[derive(Debug, Clone, PartialEq)]
pub struct Diode {
    /// The name.
    pub name: String,
    /// The anode.
    pub a: usize,
    /// The cathode.
    pub k: usize,
    /// The saturation current in ampere.
    pub is: f64,
    /// The emission coefficient.
    pub n: f64,
}

/// A circuit ready to be solved.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Circuit {
    /// The names of the nodes other than ground: node `i + 1` has name `i`.
    pub node_names: Vec<String>,
    /// The labels of the branch currents, `i(name)`.
    pub branch_names: Vec<String>,
    /// The resistors.
    pub resistors: Vec<Resistor>,
    /// The capacitors.
    pub capacitors: Vec<Capacitor>,
    /// The inductors.
    pub inductors: Vec<Inductor>,
    /// The independent voltage sources.
    pub vsources: Vec<VSource>,
    /// The independent current sources.
    pub isources: Vec<ISource>,
    /// The voltage-controlled voltage sources.
    pub vcvs: Vec<Vcvs>,
    /// The voltage-controlled current sources.
    pub vccs: Vec<Vccs>,
    /// The current-controlled current sources.
    pub cccs: Vec<Cccs>,
    /// The current-controlled voltage sources.
    pub ccvs: Vec<Ccvs>,
    /// The diodes.
    pub diodes: Vec<Diode>,
}

#[allow(clippy::too_many_arguments)]
fn flatten(
    netlist: &Netlist,
    settings: &SimSettings,
    elements: &[Element],
    prefix: &str,
    map: &BTreeMap<String, String>,
    depth: usize,
    out: &mut Vec<Element>,
) -> Result<(), SimError> {
    for element in elements {
        let nodes: Vec<String> = element
            .nodes
            .iter()
            .map(|n| {
                if settings.is_netlist_ground(n) {
                    "0".to_string()
                } else if let Some(mapped) = map.get(n) {
                    mapped.clone()
                } else if prefix.is_empty() {
                    n.clone()
                } else {
                    format!("{prefix}{n}")
                }
            })
            .collect();
        if let Kind::Instance { subckt } = &element.kind {
            if depth >= settings.max_subckt_depth {
                return Err(SimError::new(
                    SUBCKT_INVALID,
                    format!(
                        "{}: subcircuits are nested deeper than max_subckt_depth ({}); is {subckt} recursive?",
                        element.name, settings.max_subckt_depth
                    ),
                ));
            }
            let Some(sub) = netlist.subckts.get(subckt) else {
                return Err(SimError::new(
                    UNDEFINED,
                    format!("{}: the subcircuit {subckt} is not defined", element.name),
                ));
            };
            if sub.ports.len() != nodes.len() {
                return Err(SimError::new(
                    SUBCKT_INVALID,
                    format!("{}: wrong number of nodes for {subckt}", element.name),
                ));
            }
            let mut child: BTreeMap<String, String> = BTreeMap::new();
            for (port, node) in sub.ports.iter().zip(nodes.iter()) {
                child.insert(port.clone(), node.clone());
            }
            let child_prefix = format!("{prefix}{}.", element.name.to_lowercase());
            flatten(
                netlist,
                settings,
                &sub.elements,
                &child_prefix,
                &child,
                depth + 1,
                out,
            )?;
            continue;
        }
        let mut copy = Element {
            name: format!("{prefix}{}", element.name),
            nodes,
            kind: element.kind.clone(),
        };
        match &mut copy.kind {
            Kind::Cccs { control, .. } | Kind::Ccvs { control, .. } => {
                *control = format!("{prefix}{control}");
            }
            Kind::Mutual { l1, l2, .. } => {
                *l1 = format!("{prefix}{l1}");
                *l2 = format!("{prefix}{l2}");
            }
            _ => {}
        }
        out.push(copy);
        if out.len() > settings.max_elements {
            return Err(SimError::new(
                LIMIT_EXCEEDED,
                format!(
                    "more than max_elements ({}) elements after the subcircuits are expanded",
                    settings.max_elements
                ),
            ));
        }
    }
    Ok(())
}

impl Circuit {
    /// Expands the subcircuits, numbers the nodes and sorts the elements by kind.
    pub fn compile(netlist: &Netlist, settings: &SimSettings) -> Result<Circuit, SimError> {
        let mut flat: Vec<Element> = Vec::new();
        flatten(
            netlist,
            settings,
            &netlist.elements,
            "",
            &BTreeMap::new(),
            0,
            &mut flat,
        )?;

        let mut names: BTreeSet<String> = BTreeSet::new();
        for element in &flat {
            if !names.insert(element.name.to_lowercase()) {
                return Err(SimError::new(
                    DUPLICATE_NAME,
                    format!("the element {} is defined twice", element.name),
                ));
            }
        }

        let mut circuit = Circuit::default();
        let mut node_index: BTreeMap<String, usize> = BTreeMap::new();
        node_index.insert("0".to_string(), 0);
        let mut touches_ground = false;
        for element in &flat {
            for node in &element.nodes {
                if node == "0" {
                    touches_ground = true;
                } else if !node_index.contains_key(node) {
                    circuit.node_names.push(node.clone());
                    node_index.insert(node.clone(), circuit.node_names.len());
                }
            }
        }
        if !touches_ground {
            return Err(SimError::new(
                NO_GROUND,
                "no element is connected to node 0 (or gnd)".to_string(),
            ));
        }

        // the branches, in the order of the elements
        let mut branch_of: BTreeMap<String, usize> = BTreeMap::new();
        for element in &flat {
            let owns_branch = matches!(
                element.kind,
                Kind::Voltage(_) | Kind::Inductor { .. } | Kind::Vcvs(_) | Kind::Ccvs { .. }
            );
            if owns_branch {
                let lower = element.name.to_lowercase();
                branch_of.insert(lower.clone(), circuit.branch_names.len());
                circuit.branch_names.push(format!("i({lower})"));
            }
        }
        let unknowns = circuit.node_names.len() + circuit.branch_names.len();
        if unknowns > settings.max_unknowns {
            return Err(SimError::new(
                LIMIT_EXCEEDED,
                format!(
                    "{unknowns} unknowns are more than max_unknowns ({})",
                    settings.max_unknowns
                ),
            ));
        }

        let index = |name: &str| node_index[name];
        let mut inductor_index: BTreeMap<String, usize> = BTreeMap::new();
        for element in &flat {
            let lower = element.name.to_lowercase();
            let n = &element.nodes;
            let name = element.name.clone();
            match &element.kind {
                Kind::Resistor(ohms) => circuit.resistors.push(Resistor {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    ohms: *ohms,
                }),
                Kind::Capacitor { value, ic } => circuit.capacitors.push(Capacitor {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    farads: *value,
                    ic: *ic,
                }),
                Kind::Inductor { value, ic } => {
                    inductor_index.insert(lower.clone(), circuit.inductors.len());
                    circuit.inductors.push(Inductor {
                        name,
                        a: index(&n[0]),
                        b: index(&n[1]),
                        henries: *value,
                        ic: *ic,
                        branch: branch_of[&lower],
                        partners: Vec::new(),
                    });
                }
                Kind::Voltage(source) => circuit.vsources.push(VSource {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    branch: branch_of[&lower],
                    source: source.clone(),
                }),
                Kind::Current(source) => circuit.isources.push(ISource {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    source: source.clone(),
                }),
                Kind::Vcvs(gain) => circuit.vcvs.push(Vcvs {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    cp: index(&n[2]),
                    cn: index(&n[3]),
                    gain: *gain,
                    branch: branch_of[&lower],
                }),
                Kind::Vccs(gm) => circuit.vccs.push(Vccs {
                    name,
                    a: index(&n[0]),
                    b: index(&n[1]),
                    cp: index(&n[2]),
                    cn: index(&n[3]),
                    gm: *gm,
                }),
                Kind::Cccs { control, gain } => {
                    let Some(branch) = control_branch(&flat, &branch_of, control) else {
                        return Err(SimError::new(
                            UNDEFINED,
                            format!("{name}: {control} is not a voltage source"),
                        ));
                    };
                    circuit.cccs.push(Cccs {
                        name,
                        a: index(&n[0]),
                        b: index(&n[1]),
                        control: branch,
                        gain: *gain,
                    });
                }
                Kind::Ccvs { control, rm } => {
                    let Some(branch) = control_branch(&flat, &branch_of, control) else {
                        return Err(SimError::new(
                            UNDEFINED,
                            format!("{name}: {control} is not a voltage source"),
                        ));
                    };
                    circuit.ccvs.push(Ccvs {
                        name,
                        a: index(&n[0]),
                        b: index(&n[1]),
                        control: branch,
                        rm: *rm,
                        branch: branch_of[&lower],
                    });
                }
                Kind::Diode { model } => {
                    let Some(m) = netlist.models.get(model) else {
                        return Err(SimError::new(
                            UNDEFINED,
                            format!("{name}: the model {model} is not defined"),
                        ));
                    };
                    circuit.diodes.push(Diode {
                        name,
                        a: index(&n[0]),
                        k: index(&n[1]),
                        is: m.is,
                        n: m.n,
                    });
                }
                Kind::Mutual { .. } | Kind::Instance { .. } => {}
            }
        }
        for element in &flat {
            if let Kind::Mutual { l1, l2, k } = &element.kind {
                let find = |l: &String| inductor_index.get(&l.to_lowercase()).copied();
                let (Some(i), Some(j)) = (find(l1), find(l2)) else {
                    return Err(SimError::new(
                        UNDEFINED,
                        format!("{}: {l1} and {l2} must be inductors", element.name),
                    ));
                };
                let m = k * (circuit.inductors[i].henries * circuit.inductors[j].henries).sqrt();
                circuit.inductors[i].partners.push((j, m));
                circuit.inductors[j].partners.push((i, m));
            }
        }
        Ok(circuit)
    }

    /// The number of unknowns: node voltages, then branch currents.
    pub fn unknowns(&self) -> usize {
        self.node_names.len() + self.branch_names.len()
    }

    /// The row of a branch in the equations.
    pub fn branch_row(&self, branch: usize) -> usize {
        self.node_names.len() + branch
    }

    /// The labels of the unknowns: `v(node)` for each node, then `i(element)` for each branch.
    pub fn labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self.node_names.iter().map(|n| format!("v({n})")).collect();
        labels.extend(self.branch_names.iter().cloned());
        labels
    }

    /// The index of the unknown with a label (`v(out)`, `i(v1)`), compared in lower case.
    pub fn unknown_index(&self, label: &str) -> Option<usize> {
        let wanted = label.trim().to_lowercase();
        self.labels().iter().position(|l| *l == wanted)
    }

    /// The nodes that have no path to ground. With `dc` the capacitors do not conduct. A voltage
    /// source, an inductor, a resistor, a diode and the output of a voltage-controlled or
    /// current-controlled voltage source join their two nodes; a current source does not.
    pub fn floating_nodes(&self, dc: bool) -> Vec<String> {
        let count = self.node_names.len() + 1;
        let mut parent: Vec<usize> = (0..count).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let mut edges: Vec<(usize, usize)> = Vec::new();
        edges.extend(self.resistors.iter().map(|e| (e.a, e.b)));
        edges.extend(self.inductors.iter().map(|e| (e.a, e.b)));
        edges.extend(self.vsources.iter().map(|e| (e.a, e.b)));
        edges.extend(self.vcvs.iter().map(|e| (e.a, e.b)));
        edges.extend(self.ccvs.iter().map(|e| (e.a, e.b)));
        edges.extend(self.diodes.iter().map(|e| (e.a, e.k)));
        if !dc {
            edges.extend(self.capacitors.iter().map(|e| (e.a, e.b)));
        }
        for (a, b) in edges {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
        let ground = find(&mut parent, 0);
        let mut out = Vec::new();
        for node in 1..count {
            if find(&mut parent, node) != ground {
                out.push(self.node_names[node - 1].clone());
            }
        }
        out
    }

    /// The error for floating nodes, or `None` when every node has a path to ground.
    pub fn floating_error(&self, dc: bool) -> Option<SimError> {
        let floating = self.floating_nodes(dc);
        if floating.is_empty() {
            return None;
        }
        let shown: Vec<&str> = floating.iter().take(8).map(String::as_str).collect();
        let more = floating.len() - shown.len();
        let mut detail = format!("no path to ground: {}", shown.join(", "));
        if more > 0 {
            detail.push_str(&format!(" and {more} more"));
        }
        Some(SimError::new(FLOATING_NODE, detail))
    }

    /// True when the circuit has diodes, so the equations are not linear.
    pub fn is_nonlinear(&self) -> bool {
        !self.diodes.is_empty()
    }
}

/// The branch of the voltage source named by a controlled source.
fn control_branch(
    flat: &[Element],
    branch_of: &BTreeMap<String, usize>,
    control: &str,
) -> Option<usize> {
    let lower = control.to_lowercase();
    let is_voltage = flat
        .iter()
        .any(|e| e.name.to_lowercase() == lower && matches!(e.kind, Kind::Voltage(_)));
    if is_voltage {
        branch_of.get(&lower).copied()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn compile(text: &str) -> Result<Circuit, SimError> {
        let s = SimSettings::default();
        Circuit::compile(&parse(text, &s).unwrap(), &s)
    }

    #[test]
    fn nodes_and_branches_are_numbered_in_order_of_use() {
        let c = compile("t\nV1 in 0 1\nR1 in mid 1k\nL1 mid out 1m\nR2 out 0 1k\n").unwrap();
        assert_eq!(c.node_names, vec!["in", "mid", "out"]);
        assert_eq!(c.branch_names, vec!["i(v1)", "i(l1)"]);
        assert_eq!(c.unknowns(), 5);
        assert_eq!(
            c.labels(),
            vec!["v(in)", "v(mid)", "v(out)", "i(v1)", "i(l1)"]
        );
        assert_eq!(c.unknown_index("V(OUT)"), Some(2));
        assert_eq!(c.unknown_index("i(l1)"), Some(4));
        assert_eq!(c.unknown_index("v(zz)"), None);
        assert_eq!(c.vsources[0].branch, 0);
        assert_eq!(c.inductors[0].branch, 1);
        assert_eq!(c.branch_row(1), 4);
    }

    #[test]
    fn ground_has_two_names_and_a_circuit_needs_it() {
        let c = compile("t\nV1 a GND 1\nR1 a 0 1\n").unwrap();
        assert_eq!(c.node_names, vec!["a"]);
        let e = compile("t\nV1 a b 1\nR1 a b 1\n").unwrap_err();
        assert_eq!(e.code, NO_GROUND);
        let e = compile("t\n").unwrap_err();
        assert_eq!(e.code, NO_GROUND);
    }

    #[test]
    fn subcircuits_are_expanded_with_prefixed_names() {
        let c = compile(
            "t\n.subckt half a b\nR1 a m 1k\nR2 m b 1k\n.ends\nV1 in 0 1\nX1 in out half\nX2 out 0 half\n",
        )
        .unwrap();
        assert_eq!(c.node_names, vec!["in", "x1.m", "out", "x2.m"]);
        let names: Vec<&str> = c.resistors.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["X1.R1", "X1.R2", "X2.R1", "X2.R2"]);
    }

    #[test]
    fn a_recursive_subcircuit_is_stopped() {
        let e = compile("t\n.subckt s a\nX1 a s\n.ends\nX2 n s\nV1 n 0 1\n").unwrap_err();
        assert_eq!(e.code, SUBCKT_INVALID);
    }

    #[test]
    fn coupled_inductors_know_their_partner_and_the_mutual_inductance() {
        let c = compile("t\nL1 a 0 1\nL2 b 0 4\nK1 L1 L2 0.5\nV1 a 0 1\nR1 b 0 1\n").unwrap();
        assert_eq!(c.inductors[0].partners.len(), 1);
        assert_eq!(c.inductors[0].partners[0].0, 1);
        assert!((c.inductors[0].partners[0].1 - 1.0).abs() < 1e-12);
        assert_eq!(
            c.inductors[1].partners[0],
            (0, c.inductors[0].partners[0].1)
        );
    }

    #[test]
    fn nodes_without_a_path_to_ground_are_found() {
        let c = compile("t\nV1 a 0 1\nC1 a b 1u\nR1 b c 1k\n").unwrap();
        assert_eq!(c.floating_nodes(true), vec!["b", "c"]);
        assert!(c.floating_nodes(false).is_empty());
        let e = c.floating_error(true).unwrap();
        assert_eq!(e.code, FLOATING_NODE);
        assert!(e.detail.contains("b, c"));
        // a current source does not give a path
        let d = compile("t\nI1 a 0 1\nR1 b c 1\nR2 c 0 1\n").unwrap();
        assert_eq!(d.floating_nodes(true), vec!["a"]);
    }

    #[test]
    fn limits_are_enforced() {
        let s = SimSettings::parse("max_unknowns = 2\n").unwrap();
        let nl = parse("t\nV1 a 0 1\nR1 a b 1\nR2 b 0 1\n", &s).unwrap();
        assert_eq!(Circuit::compile(&nl, &s).unwrap_err().code, LIMIT_EXCEEDED);
    }
}
