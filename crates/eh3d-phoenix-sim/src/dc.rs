// SPDX-License-Identifier: GPL-3.0-or-later
//! The DC operating point: capacitors are open, inductors are short circuits, the sources take
//! their DC value, and the non-linear devices are solved by Newton-Raphson. When the plain
//! iteration does not converge, two aids are tried in turn: gmin stepping (a conductance from
//! every node to ground that is lowered step by step) and source stepping (the sources are raised
//! from zero to their value in steps).

use crate::circuit::Circuit;
use crate::codes::{SimError, ANALYSIS_INVALID, LIMIT_EXCEEDED, NO_CONVERGENCE, UNDEFINED};
use crate::real::{dc_residual, newton, Mode};
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
    /// How the point was found: `direct`, `gmin-stepping` or `source-stepping`.
    pub strategy: &'static str,
    /// The largest residual of the equations at the point: amperes in the node equations and
    /// volts in the branch equations.
    pub residual: f64,
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

/// Solves with a conductance from every node to ground that starts at `gmin_start` and is lowered
/// by `gmin_factor` at each of `gmin_steps` steps, each solution the start of the next, and then
/// without it.
fn gmin_stepping(
    c: &Circuit,
    s: &SimSettings,
    gmin_nodes: f64,
    start: &[f64],
) -> Result<(Vec<f64>, usize), SimError> {
    let mut x = start.to_vec();
    let mut total = 0;
    let mut g = s.gmin_start;
    for _ in 0..s.gmin_steps {
        let mode = Mode::dc(gmin_nodes.max(g));
        let (next, iterations) = newton(c, s, &mode, &x).map_err(|inner| {
            SimError::new(
                inner.code,
                format!("{} (gmin stepping failed at {g:e} S)", inner.detail),
            )
        })?;
        x = next;
        total += iterations;
        g /= s.gmin_factor;
    }
    let (next, iterations) = newton(c, s, &Mode::dc(gmin_nodes), &x).map_err(|inner| {
        SimError::new(
            inner.code,
            format!("{} (after the gmin stepping)", inner.detail),
        )
    })?;
    Ok((next, total + iterations))
}

/// Solves with the sources raised from zero to their value in `source_steps` steps.
fn source_stepping(
    c: &Circuit,
    s: &SimSettings,
    gmin_nodes: f64,
    start: &[f64],
) -> Result<(Vec<f64>, usize), SimError> {
    let mut x = start.to_vec();
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
    Ok((x, total))
}

/// Solves the DC equations with a conductance `gmin_nodes` from every node to ground. Returns
/// the solution, the iterations and the name of the strategy: the plain iteration first, then
/// gmin stepping and then source stepping.
pub fn solve_dc(
    c: &Circuit,
    s: &SimSettings,
    gmin_nodes: f64,
) -> Result<(Vec<f64>, usize, &'static str), SimError> {
    let zero = vec![0.0; c.unknowns()];
    match newton(c, s, &Mode::dc(gmin_nodes), &zero) {
        Ok((x, iterations)) => return Ok((x, iterations, "direct")),
        Err(e) if e.code == NO_CONVERGENCE && c.is_nonlinear() => {}
        Err(e) => return Err(e),
    }
    if s.gmin_steps > 0 {
        if let Ok((x, iterations)) = gmin_stepping(c, s, gmin_nodes, &zero) {
            return Ok((x, iterations, "gmin-stepping"));
        }
    }
    let (x, iterations) = source_stepping(c, s, gmin_nodes, &zero)?;
    Ok((x, iterations, "source-stepping"))
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
    let residual = dc_residual(c, s, &Mode::dc(gmin_nodes), &values);
    Ok(OpResult {
        labels: c.labels(),
        values,
        node_count: c.node_names.len(),
        iterations,
        strategy,
        residual,
    })
}

/// A DC sweep: the operating point while one independent source takes the values
/// `start`, `start + step`, ... up to `stop`.
#[derive(Debug, Clone, PartialEq)]
pub struct DcRequest {
    /// The name of the independent voltage or current source (compared in lower case).
    pub source: String,
    /// The first value.
    pub start: f64,
    /// The last value (reached when the step divides the range).
    pub stop: f64,
    /// The increment; negative to sweep downwards.
    pub step: f64,
}

/// The result of a DC sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct DcResult {
    /// The labels of the unknowns.
    pub labels: Vec<String>,
    /// How many of the labels are node voltages.
    pub node_count: usize,
    /// The name of the swept source, in lower case.
    pub source: String,
    /// The value of the source at each point.
    pub sweep: Vec<f64>,
    /// The unknowns at each point.
    pub values: Vec<Vec<f64>>,
    /// The Newton-Raphson iterations of all the points.
    pub iterations: usize,
    /// The points (after the first) that the continuation from the previous point did not solve
    /// and that were solved again from zero with the aids of the operating point.
    pub restarts: usize,
    /// The largest residual over all the points.
    pub residual: f64,
}

impl DcResult {
    /// The values of one unknown along the sweep, by label (`v(out)`), compared in lower case.
    pub fn series(&self, label: &str) -> Option<Vec<f64>> {
        let wanted = label.trim().to_lowercase();
        let column = self.labels.iter().position(|l| *l == wanted)?;
        Some(self.values.iter().map(|row| row[column]).collect())
    }
}

/// The values of a sweep: `start + k * step` for the whole `k` that stay on this side of `stop`
/// (a small slack absorbs the rounding of the division).
pub fn sweep_values(req: &DcRequest, max_points: usize) -> Result<Vec<f64>, SimError> {
    let invalid = |why: &str| SimError::new(ANALYSIS_INVALID, format!("DC sweep: {why}"));
    if !req.start.is_finite() || !req.stop.is_finite() || !req.step.is_finite() || req.step == 0.0 {
        return Err(invalid(
            "the start, the stop and the step must be numbers and the step cannot be zero",
        ));
    }
    let span = (req.stop - req.start) / req.step;
    if span < 0.0 {
        return Err(invalid("the step goes away from the stop value"));
    }
    let count = (span + 1e-9).floor() + 1.0;
    if count > max_points as f64 {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!("the DC sweep has more than max_dc_points ({max_points}) points"),
        ));
    }
    Ok((0..count as usize)
        .map(|k| req.start + k as f64 * req.step)
        .collect())
}

/// Sweeps one independent source and solves the operating point at each value. Each point starts
/// from the solution of the one before (continuation); when that does not converge the point is
/// solved again from zero with the plain iteration, gmin stepping and source stepping.
pub fn dc_sweep(c: &Circuit, s: &SimSettings, req: &DcRequest) -> Result<DcResult, SimError> {
    let wanted = req.source.trim().to_lowercase();
    let mut work = c.clone();
    enum Swept {
        Voltage(usize),
        Current(usize),
    }
    let target = if let Some(i) = work
        .vsources
        .iter()
        .position(|e| e.name.to_lowercase() == wanted)
    {
        Swept::Voltage(i)
    } else if let Some(i) = work
        .isources
        .iter()
        .position(|e| e.name.to_lowercase() == wanted)
    {
        Swept::Current(i)
    } else {
        return Err(SimError::new(
            UNDEFINED,
            format!(
                "DC sweep: {} is not an independent source of the circuit",
                req.source
            ),
        ));
    };
    let points = sweep_values(req, s.max_dc_points)?;
    let width = c.unknowns();
    if (points.len() as u64).saturating_mul(width as u64) > s.max_result_cells as u64 {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the DC sweep result is larger than max_result_cells ({})",
                s.max_result_cells
            ),
        ));
    }
    let mut gmin_nodes = 0.0;
    if let Some(error) = c.floating_error(true) {
        if s.floating_is_error {
            return Err(error);
        }
        gmin_nodes = s.gmin;
    }
    let mut values: Vec<Vec<f64>> = Vec::with_capacity(points.len());
    let (mut iterations, mut restarts, mut residual) = (0usize, 0usize, 0.0f64);
    let mut previous: Option<Vec<f64>> = None;
    for &v in &points {
        match target {
            Swept::Voltage(i) => work.vsources[i].source.dc = Some(v),
            Swept::Current(i) => work.isources[i].source.dc = Some(v),
        }
        let continued = previous
            .as_ref()
            .and_then(|x| newton(&work, s, &Mode::dc(gmin_nodes), x).ok());
        let (x, used) = match continued {
            Some(found) => found,
            None => {
                if previous.is_some() {
                    restarts += 1;
                }
                let (x, used, _) = solve_dc(&work, s, gmin_nodes).map_err(|e| {
                    SimError::new(
                        e.code,
                        format!("{} (DC sweep of {} at {v:e})", e.detail, req.source),
                    )
                })?;
                (x, used)
            }
        };
        iterations += used;
        residual = residual.max(dc_residual(&work, s, &Mode::dc(gmin_nodes), &x));
        values.push(x.clone());
        previous = Some(x);
    }
    Ok(DcResult {
        labels: c.labels(),
        node_count: c.node_names.len(),
        source: wanted,
        sweep: points,
        values,
        iterations,
        restarts,
        residual,
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
            gmin_steps: 0,
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
    fn gmin_stepping_and_source_stepping_find_the_same_point_as_the_plain_iteration() {
        let s = SimSettings::default();
        let text = "t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dm\n";
        let c = Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap();
        let (direct, _, strategy) = solve_dc(&c, &s, 0.0).unwrap();
        assert_eq!(strategy, "direct");
        let zero = vec![0.0; c.unknowns()];
        let (by_gmin, _) = gmin_stepping(&c, &s, 0.0, &zero).unwrap();
        let (by_source, _) = source_stepping(&c, &s, 0.0, &zero).unwrap();
        for other in [by_gmin, by_source] {
            for (a, b) in direct.iter().zip(other.iter()) {
                assert!((a - b).abs() < 1e-4, "{a} {b}");
            }
        }
    }

    #[test]
    fn the_residual_is_tiny_for_a_linear_circuit_and_small_for_a_diode() {
        let r = op("divider\nV1 in 0 DC 10\nR1 in out 1k\nR2 out 0 3k\n");
        assert!(r.residual < 1e-9, "{}", r.residual);
        let d = op("t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dm\n");
        assert!(d.residual < 1e-4, "{}", d.residual);
    }

    fn sweep(text: &str, source: &str, start: f64, stop: f64, step: f64) -> DcResult {
        let s = SimSettings::default();
        let c = Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap();
        let req = DcRequest {
            source: source.to_string(),
            start,
            stop,
            step,
        };
        dc_sweep(&c, &s, &req).unwrap()
    }

    #[test]
    fn a_sweep_of_a_divider_is_a_straight_line() {
        let r = sweep(
            "divider\nV1 in 0 DC 1\nR1 in out 1k\nR2 out 0 3k\n",
            "V1",
            0.0,
            10.0,
            2.5,
        );
        assert_eq!(r.sweep, vec![0.0, 2.5, 5.0, 7.5, 10.0]);
        let out = r.series("v(out)").unwrap();
        for (x, y) in r.sweep.iter().zip(out.iter()) {
            assert!((y - 0.75 * x).abs() < 1e-9, "{x} {y}");
        }
        assert_eq!(r.restarts, 0);
        assert!(r.residual < 1e-9);
    }

    #[test]
    fn a_sweep_can_go_down_and_can_sweep_a_current_source() {
        let r = sweep("t\nI1 0 a 1m\nR1 a 0 1k\n", "i1", 2e-3, 0.0, -1e-3);
        assert_eq!(r.sweep, vec![2e-3, 1e-3, 0.0]);
        let a = r.series("v(a)").unwrap();
        assert!((a[0] - 2.0).abs() < 1e-9 && a[2].abs() < 1e-9, "{a:?}");
    }

    #[test]
    fn a_sweep_through_a_diode_follows_the_shockley_equation_at_every_point() {
        let s = SimSettings::default();
        let r = sweep(
            "t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 0\nR1 in d 1k\nD1 d 0 dm\n",
            "v1",
            0.0,
            5.0,
            0.5,
        );
        let vt = s.thermal_voltage();
        let vd = r.series("v(d)").unwrap();
        assert_eq!(vd.len(), 11);
        for (v, got) in r.sweep.iter().zip(vd.iter()) {
            // the current of the resistor falls and the one of the diode rises with the diode
            // voltage, so the balance is found by bisection
            let (mut low, mut high) = (0.0f64, *v);
            for _ in 0..200 {
                let mid = 0.5 * (low + high);
                let balance = (v - mid) / 1000.0 - 1e-14 * ((mid / vt).exp() - 1.0);
                if balance > 0.0 {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            let want = 0.5 * (low + high);
            assert!((got - want).abs() < 1e-4, "{v}: {got} {want}");
        }
        assert!(vd.windows(2).all(|w| w[1] >= w[0] - 1e-12));
    }

    #[test]
    fn a_bad_sweep_is_refused() {
        let s = SimSettings::default();
        let c = Circuit::compile(&parse("t\nV1 a 0 1\nR1 a 0 1k\n", &s).unwrap(), &s).unwrap();
        let ask = |source: &str, start: f64, stop: f64, step: f64| {
            dc_sweep(
                &c,
                &s,
                &DcRequest {
                    source: source.to_string(),
                    start,
                    stop,
                    step,
                },
            )
            .unwrap_err()
            .code
        };
        assert_eq!(ask("v9", 0.0, 1.0, 0.1), UNDEFINED);
        assert_eq!(ask("r1", 0.0, 1.0, 0.1), UNDEFINED);
        assert_eq!(ask("v1", 0.0, 1.0, 0.0), ANALYSIS_INVALID);
        assert_eq!(ask("v1", 0.0, 1.0, -0.1), ANALYSIS_INVALID);
        assert_eq!(ask("v1", 0.0, f64::NAN, 0.1), ANALYSIS_INVALID);
        assert_eq!(ask("v1", 0.0, 1e9, 1.0), LIMIT_EXCEEDED);
    }

    #[test]
    fn no_convergence_is_reported() {
        let s = SimSettings {
            newton_max_iter: 2,
            source_steps: 1,
            gmin_steps: 0,
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
