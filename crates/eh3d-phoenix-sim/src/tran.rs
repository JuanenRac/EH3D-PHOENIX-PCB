// SPDX-License-Identifier: GPL-3.0-or-later
//! The transient analysis: backward Euler or the trapezoidal rule, with a fixed step or with an
//! adaptive step (the answer of one step is compared with the answer of two half steps, and the
//! step stops at the corners of the pulses and the piecewise-linear sources), from the operating
//! point or from the initial conditions (`uic`). The first step, and the first one after a
//! corner, is backward Euler, which does not need the currents of the capacitors before it.

use crate::circuit::Circuit;
use crate::codes::{SimError, ANALYSIS_INVALID, LIMIT_EXCEEDED, NO_CONVERGENCE, STEP_TOO_SMALL};
use crate::dc::solve_dc;
use crate::real::{newton, Companion, Method, Mode, State};
use crate::settings::SimSettings;

/// What to compute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TranRequest {
    /// The step in seconds (the largest step when the step is adaptive).
    pub tstep: f64,
    /// The end time in seconds.
    pub tstop: f64,
    /// The integration method.
    pub method: Method,
    /// Choose the step by the local error.
    pub adaptive: bool,
    /// Start from the initial conditions of the capacitors and the inductors instead of from the
    /// operating point.
    pub uic: bool,
}

/// The result of a transient analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct TranResult {
    /// The labels of the unknowns.
    pub labels: Vec<String>,
    /// How many of the labels are node voltages.
    pub node_count: usize,
    /// The time of each point, starting at zero.
    pub times: Vec<f64>,
    /// The unknowns at each time.
    pub values: Vec<Vec<f64>>,
    /// The steps that were kept.
    pub accepted: usize,
    /// The steps that were thrown away and made again smaller.
    pub rejected: usize,
    /// The Newton-Raphson iterations of all the steps.
    pub iterations: usize,
    /// Warnings that did not stop the run.
    pub notes: Vec<SimError>,
}

impl TranResult {
    /// The values of one unknown over time, by label (`v(out)`), compared in lower case.
    pub fn series(&self, label: &str) -> Option<Vec<f64>> {
        let wanted = label.trim().to_lowercase();
        let column = self.labels.iter().position(|l| *l == wanted)?;
        Some(self.values.iter().map(|row| row[column]).collect())
    }
}

fn invalid(why: &str) -> SimError {
    SimError::new(ANALYSIS_INVALID, why.to_string())
}

/// One step of `dt` ending at `t_new`.
fn step(
    c: &Circuit,
    s: &SimSettings,
    state: &State,
    t_new: f64,
    dt: f64,
    requested: Method,
) -> Result<(Vec<f64>, usize), SimError> {
    let mode = Mode {
        time: Some(t_new),
        tran: Some(Companion {
            dt,
            method: state.method_for(requested),
            state,
        }),
        ic: false,
        scale: 1.0,
        gmin_nodes: 0.0,
    };
    newton(c, s, &mode, &state.x)
}

/// The times where a source has a corner or a jump, sorted and without repeats. The adaptive
/// step lands on each of them.
fn breakpoints(c: &Circuit, s: &SimSettings, end: f64) -> Result<Vec<f64>, SimError> {
    let too_many = || {
        SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the sources have more than max_breakpoints ({}) corners before the stop time",
                s.max_breakpoints
            ),
        )
    };
    let mut all: Vec<f64> = Vec::new();
    let waves = c
        .vsources
        .iter()
        .map(|e| &e.source.wave)
        .chain(c.isources.iter().map(|e| &e.source.wave));
    for wave in waves {
        let found = wave
            .breakpoints(end, s.max_breakpoints)
            .ok_or_else(too_many)?;
        all.extend(found);
        if all.len() > s.max_breakpoints {
            return Err(too_many());
        }
    }
    all.sort_by(f64::total_cmp);
    let tolerance = end * 1e-12;
    all.dedup_by(|later, earlier| *later - *earlier <= tolerance);
    Ok(all)
}

/// The size of the difference between two solutions in units of the tolerances.
fn difference(c: &Circuit, s: &SimSettings, a: &[f64], b: &[f64]) -> f64 {
    let nn = c.node_names.len();
    let mut worst = 0.0f64;
    for (k, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        let absolute = if k < nn { s.vntol } else { s.abstol };
        let scale = s.reltol * x.abs().max(y.abs()) + absolute;
        worst = worst.max((x - y).abs() / scale);
    }
    worst
}

/// The outcome of one try of the adaptive step.
struct Attempt {
    /// The state after the two half steps.
    next: State,
    /// The method of the first half step.
    used: Method,
    /// The estimated error in units of the tolerances.
    err: f64,
    /// The Newton-Raphson iterations of the three solutions.
    iterations: usize,
}

/// Makes the step of `dt` once, and again as two half steps, and compares the two answers.
fn try_step(
    c: &Circuit,
    s: &SimSettings,
    state: &State,
    t: f64,
    dt: f64,
    requested: Method,
) -> Result<Attempt, SimError> {
    let method = state.method_for(requested);
    let (full, i1) = step(c, s, state, t + dt, dt, requested)?;
    let (half1, i2) = step(c, s, state, t + dt / 2.0, dt / 2.0, requested)?;
    let mid = state.advance(c, half1, dt / 2.0, method);
    let method2 = mid.method_for(requested);
    let (half2, i3) = step(c, s, &mid, t + dt, dt / 2.0, requested)?;
    let order = if method == Method::BackwardEuler {
        1.0
    } else {
        2.0
    };
    let err = difference(c, s, &half2, &full) / (2f64.powf(order) - 1.0);
    let next = mid.advance(c, half2, dt / 2.0, method2);
    Ok(Attempt {
        next,
        used: method,
        err,
        iterations: i1 + i2 + i3,
    })
}

/// Runs a transient analysis.
pub fn transient(c: &Circuit, s: &SimSettings, req: &TranRequest) -> Result<TranResult, SimError> {
    if !(req.tstep.is_finite() && req.tstep > 0.0) {
        return Err(invalid("the time step must be above zero"));
    }
    if !(req.tstop.is_finite() && req.tstop > 0.0) {
        return Err(invalid("the stop time must be above zero"));
    }
    if req.tstep > req.tstop {
        return Err(invalid("the time step cannot be larger than the stop time"));
    }
    let n = c.unknowns();
    let ratio = req.tstop / req.tstep;
    if ratio > s.max_tran_steps as f64 {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "{ratio:.0} steps are more than max_tran_steps ({})",
                s.max_tran_steps
            ),
        ));
    }
    if (ratio + 1.0) * n as f64 > s.max_result_cells as f64 {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the result would hold more than max_result_cells ({}) values",
                s.max_result_cells
            ),
        ));
    }
    if s.floating_is_error {
        if let Some(error) = c.floating_error(false) {
            return Err(error);
        }
    }

    let x0 = if req.uic {
        let mode = Mode {
            time: Some(0.0),
            tran: None,
            ic: true,
            scale: 1.0,
            gmin_nodes: s.gmin,
        };
        newton(c, s, &mode, &vec![0.0; n])?.0
    } else {
        solve_dc(c, s, s.gmin)?.0
    };
    let mut state = State::from_solution(c, x0);
    let mut result = TranResult {
        labels: c.labels(),
        node_count: c.node_names.len(),
        times: vec![0.0],
        values: vec![state.x.clone()],
        accepted: 0,
        rejected: 0,
        iterations: 0,
        notes: Vec::new(),
    };

    if !req.adaptive {
        let steps = ((ratio - 1e-9).ceil().max(1.0)) as usize;
        for k in 1..=steps {
            let t_prev = (k - 1) as f64 * req.tstep;
            let t_new = if k == steps {
                req.tstop
            } else {
                k as f64 * req.tstep
            };
            let dt = t_new - t_prev;
            let method = state.method_for(req.method);
            let (x, iterations) = step(c, s, &state, t_new, dt, req.method)
                .map_err(|e| SimError::new(e.code, format!("at t = {t_new:e} s: {}", e.detail)))?;
            state = state.advance(c, x, dt, method);
            result.times.push(t_new);
            result.values.push(state.x.clone());
            result.accepted += 1;
            result.iterations += iterations;
        }
        return Ok(result);
    }

    let dt_max = req.tstep;
    let dt_min = dt_max * s.min_step_fraction;
    let restart = (dt_max * s.initial_step_fraction).max(dt_min);
    let mut dt = restart;
    let end = req.tstop;
    let corners = breakpoints(c, s, end)?;
    let mut next_corner = 0usize;
    let mut t = 0.0;
    let mut warned = false;
    while t < end - end * 1e-12 {
        while next_corner < corners.len() && corners[next_corner] <= t + end * 1e-12 {
            next_corner += 1;
        }
        let target = match corners.get(next_corner) {
            Some(corner) if *corner < end => *corner,
            _ => end,
        };
        let remaining = target - t;
        if remaining - dt < dt_min {
            dt = remaining;
        }
        let last = dt >= remaining;
        let on_corner = last && target < end;
        let mut landed = false;
        let attempt = try_step(c, s, &state, t, dt, req.method);
        let at_minimum = dt <= dt_min * (1.0 + 1e-9);
        match attempt {
            Ok(Attempt {
                next,
                used,
                err,
                iterations,
            }) => {
                result.iterations += iterations;
                if err <= 1.0 || at_minimum {
                    if err > 1.0 && !warned {
                        warned = true;
                        result.notes.push(SimError::new(
                            STEP_TOO_SMALL,
                            format!(
                                "at t = {t:e} s the step is at its minimum ({dt_min:e} s) and the estimated error is above the tolerance"
                            ),
                        ));
                    }
                    t = if last { target } else { t + dt };
                    state = next;
                    if on_corner {
                        // start again with a small backward Euler step after a corner
                        state.fresh = true;
                        landed = true;
                    }
                    result.times.push(t);
                    result.values.push(state.x.clone());
                    result.accepted += 1;
                    if result.accepted > s.max_tran_steps {
                        return Err(SimError::new(
                            LIMIT_EXCEEDED,
                            format!("more than max_tran_steps ({}) steps", s.max_tran_steps),
                        ));
                    }
                    if result.values.len() * n > s.max_result_cells {
                        return Err(SimError::new(
                            LIMIT_EXCEEDED,
                            format!(
                                "the result holds more than max_result_cells ({}) values",
                                s.max_result_cells
                            ),
                        ));
                    }
                } else {
                    result.rejected += 1;
                }
                let order = if used == Method::BackwardEuler {
                    1.0
                } else {
                    2.0
                };
                let factor = (s.step_safety * err.max(1e-10).powf(-1.0 / (order + 1.0)))
                    .clamp(s.step_min_shrink, s.step_max_growth);
                dt = (dt * factor).clamp(dt_min, dt_max);
                if landed {
                    dt = restart;
                }
            }
            Err(e) if e.code == NO_CONVERGENCE && !at_minimum => {
                result.rejected += 1;
                dt = (dt * s.step_min_shrink).max(dt_min);
            }
            Err(e) => {
                return Err(SimError::new(
                    e.code,
                    format!("at t = {t:e} s: {}", e.detail),
                ));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn circuit(text: &str) -> Circuit {
        let s = SimSettings::default();
        Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
    }

    fn run(text: &str, req: TranRequest) -> TranResult {
        transient(&circuit(text), &SimSettings::default(), &req).unwrap()
    }

    fn request(tstep: f64, tstop: f64, method: Method) -> TranRequest {
        TranRequest {
            tstep,
            tstop,
            method,
            adaptive: false,
            uic: false,
        }
    }

    const RC: &str = "rc\nV1 in 0 PULSE(0 1 0 0 0 10 20)\nR1 in out 1k\nC1 out 0 1u\n";

    #[test]
    fn the_rc_circuit_reaches_63_percent_after_one_time_constant() {
        let want = 1.0 - (-1.0f64).exp();
        for (method, tolerance) in [(Method::BackwardEuler, 5e-4), (Method::Trapezoidal, 1e-5)] {
            let r = run(RC, request(1e-6, 1e-3, method));
            let v = r.series("v(out)").unwrap();
            let last = *v.last().unwrap();
            assert!((last - want).abs() < tolerance, "{method:?}: {last} {want}");
            assert_eq!(r.times.len(), 1001);
            assert_eq!(*r.times.last().unwrap(), 1e-3);
            assert_eq!(v[0], 0.0);
            assert_eq!(r.accepted, 1000);
        }
    }

    #[test]
    fn the_adaptive_step_gives_the_same_curve_with_fewer_points() {
        let want = 1.0 - (-1.0f64).exp();
        let mut req = request(1e-5, 1e-3, Method::Trapezoidal);
        req.adaptive = true;
        let r = run(RC, req);
        let v = r.series("v(out)").unwrap();
        assert!((*v.last().unwrap() - want).abs() < 3e-3);
        assert_eq!(*r.times.last().unwrap(), 1e-3);
        assert!(r.times.windows(2).all(|w| w[1] > w[0]));
        assert!(r.times.len() < 1001, "{}", r.times.len());
        assert_eq!(r.times.len(), r.accepted + 1);
        // against the exact curve at every point
        for (t, x) in r.times.iter().zip(v.iter()) {
            let exact = 1.0 - (-t / 1e-3).exp();
            assert!((x - exact).abs() < 5e-3, "t = {t}: {x} {exact}");
        }
    }

    #[test]
    fn the_adaptive_step_lands_on_every_corner_of_a_pulse() {
        let text = "rc\nV1 in 0 PULSE(0 1 1m 1u 1u 2m 5m)\nR1 in out 1k\nC1 out 0 1u\n";
        let mut req = request(1e-4, 8e-3, Method::Trapezoidal);
        req.adaptive = true;
        let r = run(text, req);
        let corners = [
            1e-3,
            1e-3 + 1e-6,
            1e-3 + 1e-6 + 2e-3,
            1e-3 + 2e-6 + 2e-3,
            6e-3,
            6e-3 + 1e-6,
        ];
        for corner in corners {
            assert!(
                r.times.iter().any(|t| (t - corner).abs() < 1e-14),
                "no point at {corner}"
            );
        }
        assert!(r.times.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(*r.times.last().unwrap(), 8e-3);
        // just before the fall the output has risen for 2 ms and a half microsecond of ramp
        let v = r.series("v(out)").unwrap();
        let index = r
            .times
            .iter()
            .position(|t| (t - (1e-3 + 1e-6 + 2e-3)).abs() < 1e-14)
            .unwrap();
        let exact = 1.0 - (-2.0f64).exp();
        assert!((v[index] - exact).abs() < 5e-3, "{} {exact}", v[index]);
        // nothing happens before the first corner
        let before = r.times.iter().position(|t| *t >= 1e-3).unwrap();
        assert!(v[before].abs() < 1e-9, "{}", v[before]);
    }

    #[test]
    fn a_pulse_that_is_too_fast_for_the_breakpoint_limit_is_refused() {
        let text = "rc\nV1 in 0 PULSE(0 1 0 1n 1n 1u 2u)\nR1 in out 1k\nC1 out 0 1n\n";
        let s = SimSettings::parse("max_breakpoints = 100\n").unwrap();
        let mut req = request(1e-6, 1e-3, Method::Trapezoidal);
        req.adaptive = true;
        let e = transient(&circuit(text), &s, &req).unwrap_err();
        assert_eq!(e.code, LIMIT_EXCEEDED);
        assert!(e.detail.contains("max_breakpoints"), "{}", e.detail);
        // the fixed step does not look for corners
        req.adaptive = false;
        assert!(transient(&circuit(text), &s, &req).is_ok());
    }

    #[test]
    fn a_capacitor_with_an_initial_voltage_discharges_with_the_same_time_constant() {
        let mut req = request(1e-6, 1e-3, Method::Trapezoidal);
        req.uic = true;
        let r = run("t\nR1 out 0 1k\nC1 out 0 1u ic=1\n", req);
        let v = r.series("v(out)").unwrap();
        assert!((v[0] - 1.0).abs() < 1e-6, "{}", v[0]);
        let want = (-1.0f64).exp();
        assert!((*v.last().unwrap() - want).abs() < 1e-5);
    }

    #[test]
    fn the_rl_current_rises_with_its_time_constant() {
        let text = "rl\nV1 in 0 PULSE(0 1 0 0 0 10 20)\nR1 in a 1k\nL1 a 0 1\n";
        let r = run(text, request(1e-6, 1e-3, Method::Trapezoidal));
        let i = r.series("i(l1)").unwrap();
        let want = (1.0 - (-1.0f64).exp()) / 1000.0;
        assert!((*i.last().unwrap() - want).abs() < 1e-8);
    }

    #[test]
    fn the_half_wave_rectifier_peaks_a_diode_drop_below_the_source() {
        let text =
            "rect\n.model dm D(IS=1e-14 N=1)\nV1 in 0 SIN(0 5 1k)\nD1 in out dm\nR1 out 0 1k\n";
        let s = SimSettings::default();
        let r = run(text, request(1e-6, 2e-3, Method::Trapezoidal));
        let v = r.series("v(out)").unwrap();
        let peak = v.iter().copied().fold(f64::MIN, f64::max);
        let lowest = v.iter().copied().fold(f64::MAX, f64::min);
        // the diode voltage at the peak current: vd = vt ln(i / is), i = (5 - vd) / 1k
        let vt = s.thermal_voltage();
        let mut vd = 0.6f64;
        for _ in 0..100 {
            vd = vt * ((5.0 - vd) / 1000.0 / 1e-14 + 1.0).ln();
        }
        assert!((peak - (5.0 - vd)).abs() < 2e-3, "{peak} {}", 5.0 - vd);
        assert!(peak > 4.2 && peak < 4.4);
        assert!(lowest > -1e-7, "{lowest}");
        // in the negative half cycle nothing flows
        let index = r
            .times
            .iter()
            .position(|t| (*t - 0.75e-3).abs() < 1e-12)
            .unwrap();
        assert!(v[index].abs() < 1e-7);
    }

    #[test]
    fn a_request_that_is_not_valid_is_refused() {
        let c = circuit(RC);
        let s = SimSettings::default();
        for (tstep, tstop) in [
            (0.0, 1.0),
            (1.0, 0.0),
            (2.0, 1.0),
            (f64::NAN, 1.0),
            (1e-6, f64::INFINITY),
        ] {
            let e = transient(&c, &s, &request(tstep, tstop, Method::BackwardEuler)).unwrap_err();
            assert_eq!(e.code, ANALYSIS_INVALID, "{tstep} {tstop}");
        }
        let small = SimSettings::parse("max_tran_steps = 100\n").unwrap();
        let e = transient(&c, &small, &request(1e-6, 1e-3, Method::BackwardEuler)).unwrap_err();
        assert_eq!(e.code, LIMIT_EXCEEDED);
        let cells = SimSettings::parse("max_result_cells = 100\n").unwrap();
        let e = transient(&c, &cells, &request(1e-6, 1e-3, Method::BackwardEuler)).unwrap_err();
        assert_eq!(e.code, LIMIT_EXCEEDED);
    }

    #[test]
    fn two_runs_give_the_same_numbers() {
        let a = run(RC, request(1e-5, 1e-3, Method::Trapezoidal));
        let b = run(RC, request(1e-5, 1e-3, Method::Trapezoidal));
        assert_eq!(a, b);
    }
}
