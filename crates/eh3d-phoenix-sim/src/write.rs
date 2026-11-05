// SPDX-License-Identifier: GPL-3.0-or-later
//! The writer of SPICE-style netlists. The text is deterministic (parameters and models sorted by
//! name, elements in their order) and reads back, with [`crate::parse::parse`], as the same
//! netlist.

use crate::netlist::{Element, Kind, Netlist, Source, Wave};
use crate::units::format_number;

fn n(v: f64) -> String {
    format_number(v)
}

fn source_text(source: &Source) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(dc) = source.dc {
        parts.push(format!("DC {}", n(dc)));
    }
    if let Some((magnitude, phase)) = source.ac {
        parts.push(format!("AC {} {}", n(magnitude), n(phase)));
    }
    match &source.wave {
        Wave::None => {}
        Wave::Sin {
            vo,
            va,
            freq,
            td,
            theta,
            phase_deg,
        } => parts.push(format!(
            "SIN({} {} {} {} {} {})",
            n(*vo),
            n(*va),
            n(*freq),
            n(*td),
            n(*theta),
            n(*phase_deg)
        )),
        Wave::Pulse {
            v1,
            v2,
            td,
            tr,
            tf,
            pw,
            per,
        } => parts.push(format!(
            "PULSE({} {} {} {} {} {} {})",
            n(*v1),
            n(*v2),
            n(*td),
            n(*tr),
            n(*tf),
            n(*pw),
            n(*per)
        )),
        Wave::Pwl(points) => {
            let pairs: Vec<String> = points
                .iter()
                .map(|(t, v)| format!("{} {}", n(*t), n(*v)))
                .collect();
            parts.push(format!("PWL({})", pairs.join(" ")));
        }
    }
    parts.join(" ")
}

/// The line of one element.
pub fn element_line(e: &Element) -> String {
    let nodes = e.nodes.join(" ");
    let ic = |ic: &Option<f64>| match ic {
        Some(v) => format!(" ic={}", n(*v)),
        None => String::new(),
    };
    match &e.kind {
        Kind::Resistor(v) => format!("{} {nodes} {}", e.name, n(*v)),
        Kind::Capacitor { value, ic: i } => format!("{} {nodes} {}{}", e.name, n(*value), ic(i)),
        Kind::Inductor { value, ic: i } => format!("{} {nodes} {}{}", e.name, n(*value), ic(i)),
        Kind::Voltage(source) | Kind::Current(source) => {
            format!("{} {nodes} {}", e.name, source_text(source))
        }
        Kind::Vcvs(gain) => format!("{} {nodes} {}", e.name, n(*gain)),
        Kind::Vccs(gm) => format!("{} {nodes} {}", e.name, n(*gm)),
        Kind::Cccs { control, gain } => format!("{} {nodes} {control} {}", e.name, n(*gain)),
        Kind::Ccvs { control, rm } => format!("{} {nodes} {control} {}", e.name, n(*rm)),
        Kind::Mutual { l1, l2, k } => format!("{} {l1} {l2} {}", e.name, n(*k)),
        Kind::Diode { model } => format!("{} {nodes} {model}", e.name),
        Kind::Instance { subckt } => format!("{} {nodes} {subckt}", e.name),
    }
}

/// The text of a netlist.
pub fn to_spice(netlist: &Netlist) -> String {
    let mut out = String::new();
    out.push_str(&netlist.title);
    out.push('\n');
    for (name, value) in &netlist.params {
        out.push_str(&format!(".param {name}={}\n", n(*value)));
    }
    for (name, model) in &netlist.models {
        out.push_str(&format!(
            ".model {name} D(IS={} N={})\n",
            n(model.is),
            n(model.n)
        ));
    }
    for (name, sub) in &netlist.subckts {
        out.push_str(&format!(".subckt {name} {}\n", sub.ports.join(" ")));
        for e in &sub.elements {
            out.push_str(&element_line(e));
            out.push('\n');
        }
        out.push_str(&format!(".ends {name}\n"));
    }
    for e in &netlist.elements {
        out.push_str(&element_line(e));
        out.push('\n');
    }
    out.push_str(".end\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::settings::SimSettings;

    const FULL: &str = "everything\n.param rr=2k\n.model dm D(IS=2e-14 N=1.5)\n.subckt half a b\nR1 a m 1k\nR2 m b {rr}\n.ends half\nV1 in 0 DC 5 AC 1 45 SIN(0 1 1k 0 0 0)\nV2 p 0 PULSE(0 5 1u 1n 1n 5u 10u)\nV3 q 0 PWL(0 0 1m 1)\nI1 r 0 2m\nR3 in out 4.7k\nC1 out 0 10n ic=0.5\nL1 out x 1m ic=0\nL2 y 0 4m\nK1 L1 L2 0.95\nD1 out 0 dm\nE1 e 0 in 0 2\nG1 g 0 in 0 1m\nF1 f 0 V1 3\nH1 h 0 V1 100\nX1 in z half\n";

    #[test]
    fn a_netlist_survives_writing_and_reading() {
        let s = SimSettings::default();
        let first = parse(FULL, &s).unwrap();
        let text = to_spice(&first);
        let second = parse(&text, &s).unwrap();
        assert_eq!(first, second, "{text}");
        // and the text of the second is the text of the first
        assert_eq!(to_spice(&second), text);
    }

    #[test]
    fn the_text_is_the_expected_one_for_a_small_netlist() {
        let s = SimSettings::default();
        let nl = parse("rc\nV1 in 0 DC 1\nR1 in out 1k\nC1 out 0 1u\n", &s).unwrap();
        assert_eq!(
            to_spice(&nl),
            "rc\nV1 in 0 DC 1e0\nR1 in out 1e3\nC1 out 0 1e-6\n.end\n"
        );
    }
}
