// SPDX-License-Identifier: GPL-3.0-or-later
//! The reflections of a lossless line between a driver and a load by the method of Bergeron (the
//! method of characteristics; see for instance W. Dally and J. Poulton, *Digital Systems
//! Engineering*, or any text on lattice diagrams).
//!
//! The driver is a step of 1 V that rises in `tr` through a resistance `Rs`; the line has an
//! impedance `Z0` and a delay `Td`; the load is any parallel mix of a resistance, a capacitance and
//! a series RC branch. Two things come out:
//!
//! * [`bounce_table`]: the diagram of bounces for a step that rises at once and a load that is a
//!   resistance or an open end, exact: every arrival at the load and at the source with its
//!   voltage;
//! * [`simulate`]: the voltage at the load with time steps of a fraction of the delay, with the
//!   capacitors as trapezoidal companions, from which the overshoot, the ringback and the
//!   settling are measured.
//!
//! The line is lossless and the driver has a fixed resistance: an estimate, not a simulation of a
//! buffer.

use serde::Serialize;

use crate::phys::reflection;

/// What is added at the load or the driver to terminate the line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Termination {
    /// Nothing is added.
    None,
    /// A resistance in series with the driver, in ohm.
    Series(f64),
    /// A resistance from the load to ground, in ohm.
    Parallel(f64),
    /// Two resistances, one to the supply and one to ground, in ohm; for the steps they are in
    /// parallel.
    Thevenin(f64, f64),
    /// A resistance in series with a capacitor from the load to ground, in ohm and picofarad.
    Rc(f64, f64),
}

/// The inputs of one simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    /// The impedance of the line, in ohm.
    pub z0: f64,
    /// The delay of the line, in picoseconds.
    pub td_ps: f64,
    /// The output resistance of the driver, in ohm.
    pub rs: f64,
    /// The resistance of the load to ground; `None` for an open end.
    pub rl: Option<f64>,
    /// The capacitance of the load, in picofarad.
    pub cl_pf: f64,
    /// The rise time of the step, in picoseconds (the whole ramp, from 0 to 100 percent).
    pub tr_ps: f64,
    /// The termination that is added.
    pub term: Termination,
}

/// The limits of one simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// The steps the rise time is cut into.
    pub steps_per_rise: u32,
    /// The delays of the line that are covered.
    pub delays: u32,
    /// The most steps.
    pub max_steps: usize,
    /// The band around the final level inside which the line counts as settled, in percent.
    pub settle_band_pct: f64,
}

/// What the voltage at the load did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Outcome {
    /// The final level at the load, as a fraction of the step of the driver.
    pub final_level: f64,
    /// The highest voltage above the final level, in percent of the final level.
    pub overshoot_pct: f64,
    /// The lowest voltage below the final level after it was first reached, in percent of the
    /// final level.
    pub ringback_pct: f64,
    /// The time after which the voltage stays inside the band, in picoseconds from the start of
    /// the step; `None` if it had not settled when the simulation ended.
    pub settle_ps: Option<f64>,
    /// The time step, in picoseconds.
    pub step_ps: f64,
    /// The number of steps.
    pub steps: usize,
    /// Whether the simulation was cut at the limit of steps.
    pub truncated: bool,
}

/// One arrival of a wave in the diagram of bounces.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bounce {
    /// The time of the arrival, in picoseconds after the step reaches the line.
    pub time_ps: f64,
    /// Where the wave arrives: `load` or `source`.
    pub at: &'static str,
    /// The voltage of the wave that arrives, in volt.
    pub incident_v: f64,
    /// The voltage at that end after the arrival, in volt.
    pub node_v: f64,
}

/// The bounces of a step of 1 V that rises at once through `rs` on a line of `z0` and `td_ps` into a
/// resistance `rl` (`None` for an open end): the first wave is `Z0 / (Rs + Z0)`, each arrival at the
/// load is reflected with `(RL - Z0) / (RL + Z0)` and each arrival at the source with
/// `(Rs - Z0) / (Rs + Z0)`. The list ends after `count` bounces.
pub fn bounce_table(z0: f64, td_ps: f64, rs: f64, rl: Option<f64>, count: usize) -> Vec<Bounce> {
    let gamma_s = reflection(rs, z0);
    let gamma_l = rl.map_or(1.0, |r| reflection(r, z0));
    let mut out = Vec::new();
    // the wave that goes towards the load
    let mut wave = z0 / (rs + z0);
    let mut source_v = wave;
    let mut load_v = 0.0;
    let mut time = 0.0;
    for _ in 0..count {
        time += td_ps;
        load_v += wave * (1.0 + gamma_l);
        out.push(Bounce {
            time_ps: time,
            at: "load",
            incident_v: wave,
            node_v: load_v,
        });
        let back = wave * gamma_l;
        time += td_ps;
        source_v += back * (1.0 + gamma_s);
        out.push(Bounce {
            time_ps: time,
            at: "source",
            incident_v: back,
            node_v: source_v,
        });
        wave = back * gamma_s;
    }
    out
}

/// The final level at the load for a driver of resistance `rs_total` into a load of conductance
/// `g_load` (the capacitors do not conduct): a divider.
fn final_level(rs_total: f64, g_load: f64) -> f64 {
    1.0 / (1.0 + rs_total * g_load)
}

/// The linear parts of the load, as conductance and companion data.
struct Load {
    /// The conductance to ground, in siemens.
    g: f64,
    /// The capacitance to ground, in picofarad.
    c_pf: f64,
    /// The series RC branch: resistance in ohm and capacitance in picofarad.
    rc: Option<(f64, f64)>,
}

fn load_of(input: &Input) -> (Load, f64) {
    let mut g = input.rl.map_or(0.0, |r| 1.0 / r);
    let mut rs_total = input.rs;
    let mut rc = None;
    match input.term {
        Termination::None => {}
        Termination::Series(r) => rs_total += r,
        Termination::Parallel(r) => g += 1.0 / r,
        Termination::Thevenin(r1, r2) => g += 1.0 / r1 + 1.0 / r2,
        Termination::Rc(r, c) => rc = Some((r, c)),
    }
    (
        Load {
            g,
            c_pf: input.cl_pf,
            rc,
        },
        rs_total,
    )
}

/// Simulates the step response at the load. The line is cut into `m` steps of delay with `m` at
/// least 4 and enough to resolve the rise time; the run covers `delays` delays and at least three
/// rise times.
pub fn simulate(input: &Input, limits: &Limits) -> Outcome {
    let z0 = input.z0;
    let td = input.td_ps.max(1.0e-3);
    let tr = input.tr_ps.max(1.0e-3);
    let (load, rs_total) = load_of(input);
    let rho = rs_total / z0;
    let m = ((td * f64::from(limits.steps_per_rise) / tr).ceil() as usize).clamp(4, 2000);
    let dt = td / m as f64;
    let span = (f64::from(limits.delays) * td).max(3.0 * tr + 10.0 * td);
    let wanted = (span / dt).ceil() as usize + 1;
    let truncated = wanted > limits.max_steps;
    let n_steps = wanted.min(limits.max_steps).max(m + 2);
    let gc_load = 2.0 * load.c_pf / dt;
    let (geq, gct) = match load.rc {
        Some((r, c)) => {
            let g = 2.0 * c / dt;
            (1.0 / (r + 1.0 / g), g)
        }
        None => (0.0, 1.0),
    };
    let denominator = 1.0 / z0 + load.g + gc_load + geq;
    // the wave that leaves the driver, the wave that comes back from the load and the voltage at
    // the load, one value for each step
    let mut a1: Vec<f64> = Vec::with_capacity(n_steps);
    let mut b2: Vec<f64> = Vec::with_capacity(n_steps);
    let mut v2: Vec<f64> = Vec::with_capacity(n_steps);
    let mut hist_load = 0.0;
    let mut hist_rc = 0.0;
    for n in 0..n_steps {
        let back = if n >= m { b2[n - m] } else { 0.0 };
        let source = (n as f64 * dt / tr).min(1.0);
        a1.push((source - (1.0 - rho) * back) / (1.0 + rho));
        let incident = if n >= m { a1[n - m] } else { 0.0 };
        let numerator = 2.0 * incident / z0 + hist_load + geq * hist_rc / gct;
        let v = numerator / denominator;
        v2.push(v);
        b2.push(v - incident);
        hist_load = 2.0 * gc_load * v - hist_load;
        if load.rc.is_some() {
            let current = geq * (v - hist_rc / gct);
            let capacitor_v = (current + hist_rc) / gct;
            hist_rc = 2.0 * gct * capacitor_v - hist_rc;
        }
    }
    measure(&v2, final_level(rs_total, load.g), dt, limits, truncated)
}

fn measure(v: &[f64], level: f64, dt: f64, limits: &Limits, truncated: bool) -> Outcome {
    let peak = v.iter().copied().fold(f64::MIN, f64::max);
    let overshoot_pct = ((peak - level) / level * 100.0).max(0.0);
    let mut ringback_pct = 0.0;
    if let Some(first) = v.iter().position(|x| *x >= level) {
        let low = v[first..].iter().copied().fold(f64::MAX, f64::min);
        ringback_pct = ((level - low) / level * 100.0).max(0.0);
    }
    let band = level * limits.settle_band_pct / 100.0;
    let last_out = v.iter().rposition(|x| (x - level).abs() > band);
    let settle_ps = match last_out {
        None => Some(0.0),
        Some(i) if i + 1 < v.len() => Some((i + 1) as f64 * dt),
        Some(_) => None,
    };
    Outcome {
        final_level: level,
        overshoot_pct,
        ringback_pct,
        settle_ps,
        step_ps: dt,
        steps: v.len(),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        steps_per_rise: 20,
        delays: 40,
        max_steps: 400_000,
        settle_band_pct: 5.0,
    };

    fn input(rs: f64, rl: Option<f64>, cl_pf: f64, term: Termination) -> Input {
        Input {
            z0: 50.0,
            td_ps: 1000.0,
            rs,
            rl,
            cl_pf,
            tr_ps: 50.0,
            term,
        }
    }

    #[test]
    fn the_bounces_of_an_open_line_with_a_weak_driver_follow_the_lattice() {
        // Rs = 50/3 ohm: the first wave is 0.75 V, the source reflects -0.5, the open end +1
        let table = bounce_table(50.0, 1000.0, 50.0 / 3.0, None, 2);
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].at, "load");
        assert!((table[0].time_ps - 1000.0).abs() < 1e-9);
        assert!((table[0].incident_v - 0.75).abs() < 1e-12);
        assert!((table[0].node_v - 1.5).abs() < 1e-12);
        assert!((table[1].time_ps - 2000.0).abs() < 1e-9);
        // 0.75 comes back, and at the source it adds (1 + gamma_s) = 0.5 of itself
        assert!((table[1].node_v - (0.75 + 0.375)).abs() < 1e-12);
        // the next wave is 0.75 * -0.5 = -0.375 and the load sees 1.5 - 0.75
        assert!((table[2].incident_v + 0.375).abs() < 1e-12);
        assert!((table[2].node_v - 0.75).abs() < 1e-12);
    }

    #[test]
    fn the_bounces_into_a_matched_load_stop_after_the_first_wave() {
        let table = bounce_table(50.0, 100.0, 25.0, Some(50.0), 3);
        assert!((table[0].node_v - 50.0 / 75.0).abs() < 1e-12);
        assert!(table[1].incident_v.abs() < 1e-12);
        assert!(table[2].incident_v.abs() < 1e-12);
    }

    #[test]
    fn a_weak_driver_into_an_open_line_overshoots_by_half() {
        let out = simulate(&input(50.0 / 3.0, None, 0.0, Termination::None), &LIMITS);
        assert!((out.final_level - 1.0).abs() < 1e-12);
        assert!(
            (out.overshoot_pct - 50.0).abs() < 0.5,
            "{}",
            out.overshoot_pct
        );
        // after the peak the voltage falls to 0.75 and so under the final level
        assert!(
            (out.ringback_pct - 25.0).abs() < 0.5,
            "{}",
            out.ringback_pct
        );
        assert!(!out.truncated);
    }

    #[test]
    fn a_matched_source_gives_no_overshoot_and_settles_after_one_delay() {
        let out = simulate(&input(50.0, None, 0.0, Termination::None), &LIMITS);
        assert!(out.overshoot_pct < 0.1, "{}", out.overshoot_pct);
        assert!(out.ringback_pct < 0.1);
        let settle = out.settle_ps.unwrap();
        // the load reaches 95 percent at a delay plus 95 percent of the rise
        assert!(settle > 1000.0 && settle < 1100.0, "{settle}");
    }

    #[test]
    fn a_series_resistor_that_matches_the_source_removes_the_ringing() {
        let bare = simulate(&input(10.0, None, 0.0, Termination::None), &LIMITS);
        let fixed = simulate(&input(10.0, None, 0.0, Termination::Series(40.0)), &LIMITS);
        assert!(bare.overshoot_pct > 30.0);
        assert!(fixed.overshoot_pct < 0.5, "{}", fixed.overshoot_pct);
    }

    #[test]
    fn a_parallel_resistor_sets_the_final_level_by_the_divider() {
        let out = simulate(
            &input(25.0, None, 0.0, Termination::Parallel(50.0)),
            &LIMITS,
        );
        // 50 / (25 + 50)
        assert!((out.final_level - 2.0 / 3.0).abs() < 1e-12);
        assert!(out.overshoot_pct < 0.5);
        let thevenin = simulate(
            &input(25.0, None, 0.0, Termination::Thevenin(100.0, 100.0)),
            &LIMITS,
        );
        assert!((thevenin.final_level - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_capacitive_load_slows_the_edge_and_an_rc_branch_changes_nothing_at_dc() {
        let open = simulate(&input(50.0, None, 0.0, Termination::None), &LIMITS);
        let loaded = simulate(&input(50.0, None, 10.0, Termination::None), &LIMITS);
        assert!(loaded.settle_ps.unwrap() > open.settle_ps.unwrap());
        let rc = simulate(
            &input(50.0, None, 0.0, Termination::Rc(50.0, 100.0)),
            &LIMITS,
        );
        assert!((rc.final_level - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_run_is_cut_at_the_limit_of_steps() {
        let tight = Limits {
            max_steps: 1000,
            ..LIMITS
        };
        let out = simulate(&input(25.0, None, 0.0, Termination::None), &tight);
        assert!(out.truncated);
        assert_eq!(out.steps, 1000);
    }

    #[test]
    fn the_same_inputs_give_the_same_outcome() {
        let a = simulate(&input(20.0, Some(200.0), 3.0, Termination::None), &LIMITS);
        let b = simulate(&input(20.0, Some(200.0), 3.0, Termination::None), &LIMITS);
        assert_eq!(a, b);
    }
}
