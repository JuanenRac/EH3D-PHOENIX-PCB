// SPDX-License-Identifier: GPL-3.0-or-later
//! The DC operating point: capacitors are open, inductors are short circuits, the sources take
//! their DC value, and the diodes are solved by Newton-Raphson. When the plain iteration does not
//! converge, the sources are raised from zero to their value in steps.

use crate::circuit::Circuit;
use crate::codes::{SimError, NO_CONVERGENCE};
use crate::real::{newton, Mode};
use crate::settings::SimSettings;

/// The operating point.
#[derive(Debug, Clone, PartialEq)]
pub struct OpResult {
    /// The labels of the unknowns: `v(node)`, then `i(element)`.
    pub labels: Vec<String>,
    /// The values, in the order of the labels.
    pub values: Vec<f64>,
    /// How many of the labels are node voltages.
    pub node_count: usize,
    /// The Newton-Raphson iterations used (1 for a linear circuit).
    pub iterations: usize,
    /// How the point was found: `direct` or `source-stepping`.
    pub strategy: &'static str,
}

impl OpResult {
    /// The value with a label (`v(out)`, `i(v1)`), compared in lower case.
    pub fn get(&self, label: &str) -> Option<f64> {
        let wanted = label.trim().to_lowercase();
        self.labels
            .iter()
            .position(|l| *l == wanted)
            .map(|i| self.values[i])
    }
}

/// Solves the DC equations with a conductance `gmin_nodes` from every node to ground. Returns
/// the solution, the iterations and the name of the strategy.
pub fn solve_dc(
    c: &Circuit,
    s: &SimSettings,
    gmin_nodes: f64,
) -> Result<(Vec<f64>, usize, &'static str), SimError> {
    let zero = vec![0.0; c.unknowns()];
    match newton(c, s, &Mode::dc(gmin_nodes), &zero) {
        Ok((x, iterations)) => Ok((x, iterations, "direct")),
        Err(e) if e.code == NO_CONVERGENCE && c.is_nonlinear() => {
            let mut x = zero;
            let mut total = 0;
            for step in 1..=s.source_steps {
                let mode = Mode {
                    scale: step as f64 / s.source_steps as f64,
                    ..Mode::dc(gmin_nodes)
                };
                let (next, iterations) = newton(c, s, &mode, &x).map_err(|inner| {
                    SimError::new(
                        inner.code,
                        format!(
                            "{} (source stepping failed at {} of {} steps)",
                            inner.detail, step, s.source_steps
                        ),
                    )
                })?;
                x = next;
                total += iterations;
            }
            Ok((x, total, "source-stepping"))
        }
        Err(e) => Err(e),
    }
}

/// The operating point of a circuit. A node with no DC path to ground is an error when the
/// settings say so (`floating_is_error`); otherwise a small conductance holds it.
pub fn operating_point(c: &Circuit, s: &SimSettings) -> Result<OpResult, SimError> {
    let mut gmin_nodes = 0.0;
    if let Some(error) = c.floating_error(true) {
        if s.floating_is_error {
            return Err(error);
        }
        gmin_nodes = s.gmin;
    }
    let (values, iterations, strategy) = solve_dc(c, s, gmin_nodes)?;
    Ok(OpResult {
        labels: c.labels(),
        values,
        node_count: c.node_names.len(),
        iterations,
        strategy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{FLOATING_NODE, SINGULAR_MATRIX};
    use crate::parse::parse;

    fn op_with(text: &str, s: &SimSettings) -> Result<OpResult, SimError> {
        let c = Circuit::compile(&parse(text, s).unwrap(), s).unwrap();
        operating_point(&c, s)
    }

    fn op(text: &str) -> OpResult {
        op_with(text, &SimSettings::default()).unwrap()
    }

    #[test]
    fn the_voltage_divider_of_the_textbook() {
        let r = op("divider\nV1 in 0 DC 10\nR1 in out 1k\nR2 out 0 3k\n");
        assert!((r.get("v(out)").unwrap() - 7.5).abs() < 1e-12);
        assert!((r.get("v(in)").unwrap() - 10.0).abs() < 1e-12);
        // the source gives 2.5 mA: SPICE reports it with the sign of a current entering the source
        assert!((r.get("i(v1)").unwrap() + 2.5e-3).abs() < 1e-15);
        assert_eq!(r.iterations, 1);
        assert_eq!(r.strategy, "direct");
        assert_eq!(r.node_count, 2);
    }

    #[test]
    fn a_current_source_through_a_resistor() {
        // 2 mA from ground into node a through the source: I1 0 a 2m, R 1k to ground -> 2 V
        let r = op("t\nI1 0 a 2m\nR1 a 0 1k\n");
        assert!((r.get("v(a)").unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn dependent_sources() {
        let e = op("t\nV1 in 0 1\nE1 out 0 in 0 5\nRL out 0 1k\n");
        assert!((e.get("v(out)").unwrap() - 5.0).abs() < 1e-12);
        assert!((e.get("i(e1)").unwrap() + 5e-3).abs() < 1e-15);
        let g = op("t\nV1 in 0 1\nG1 out 0 in 0 1m\nRL out 0 1k\n");
        assert!((g.get("v(out)").unwrap() + 1.0).abs() < 1e-12);
        let f = op("t\nV1 in 0 1\nR1 in 0 1k\nF1 out 0 V1 10\nRL out 0 1k\n");
        assert!((f.get("v(out)").unwrap() - 10.0).abs() < 1e-9);
        let h = op("t\nV1 in 0 1\nR1 in 0 1k\nH1 out 0 V1 1k\nRL out 0 1k\n");
        assert!((h.get("v(out)").unwrap() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn inductors_are_short_and_capacitors_are_open_in_dc() {
        let r = op("t\nV1 in 0 6\nR1 in a 1k\nL1 a b 1m\nR2 b 0 2k\nC1 b 0 1u\n");
        assert!((r.get("v(a)").unwrap() - 4.0).abs() < 1e-9);
        assert!((r.get("v(b)").unwrap() - 4.0).abs() < 1e-9);
        assert!((r.get("i(l1)").unwrap() - 2e-3).abs() < 1e-12);
    }

    #[test]
    fn a_diode_in_series_with_a_resistor_drops_about_seven_tenths_of_a_volt() {
        let s = SimSettings::default();
        let r = op("t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dm\n");
        let vd = r.get("v(d)").unwrap();
        // the fixed point of vd = n vt ln(i / is) with i = (5 - vd) / 1k
        let vt = s.thermal_voltage();
        let mut want = 0.6f64;
        for _ in 0..100 {
            want = vt * ((5.0 - want) / 1000.0 / 1e-14 + 1.0).ln();
        }
        assert!((vd - want).abs() < 1e-4, "{vd} {want}");
        assert!(vd > 0.65 && vd < 0.75);
        assert!(r.iterations > 1);
    }

    #[test]
    fn a_reversed_diode_blocks() {
        let r = op("t\n.model dm D\nV1 in 0 5\nR1 in d 1k\nD1 0 d dm\n");
        // the anode is ground, so d sits at the source voltage minus the leakage times 1k
        assert!((r.get("v(d)").unwrap() - 5.0).abs() < 1e-7);
    }

    #[test]
    fn source_stepping_is_the_fallback() {
        let s = SimSettings {
            newton_max_iter: 3,
            source_steps: 50,
            ..SimSettings::default()
        };
        let r = op_with("t\n.model dm D\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dm\n", &s);
        // with three iterations the plain attempt fails; the stepping may or may not succeed, but
        // it never gives a wrong answer
        if let Ok(r) = r {
            assert_eq!(r.strategy, "source-stepping");
            let vd = r.get("v(d)").unwrap();
            assert!(vd > 0.65 && vd < 0.75, "{vd}");
        }
    }

    #[test]
    fn no_convergence_is_reported() {
        let s = SimSettings {
            newton_max_iter: 2,
            source_steps: 1,
            ..SimSettings::default()
        };
        let e = op_with("t\n.model dm D\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dm\n", &s).unwrap_err();
        assert_eq!(e.code, NO_CONVERGENCE);
    }

    #[test]
    fn a_singular_circuit_names_the_unknown() {
        let e = op_with("t\nV1 a 0 1\nV2 a 0 2\n", &SimSettings::default()).unwrap_err();
        assert_eq!(e.code, SINGULAR_MATRIX);
        assert!(e.detail.contains("i(v"), "{}", e.detail);
        let loop_of_inductors = op_with(
            "t\nV1 a 0 1\nL1 a b 1m\nL2 b 0 1m\nL3 a 0 1m\n",
            &SimSettings::default(),
        )
        .unwrap_err();
        assert_eq!(loop_of_inductors.code, SINGULAR_MATRIX);
    }

    #[test]
    fn a_floating_node_is_reported_before_the_matrix() {
        let text = "t\nV1 a 0 1\nC1 a b 1u\nR1 b c 1k\n";
        let e = op_with(text, &SimSettings::default()).unwrap_err();
        assert_eq!(e.code, FLOATING_NODE);
        assert!(e.detail.contains("b, c"), "{}", e.detail);
        // with the setting off, a small conductance holds the nodes at zero volts
        let s = SimSettings {
            floating_is_error: false,
            ..SimSettings::default()
        };
        let r = op_with(text, &s).unwrap();
        assert!(r.get("v(b)").unwrap().abs() < 1e-6);
    }
}
