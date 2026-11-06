// SPDX-License-Identifier: GPL-3.0-or-later
//! The termination of the nets: the impedance, the electrical length and the delay of each net,
//! the criterion of a long line (`tr < 2 Td`), the reflections by the method of Bergeron with the
//! driver and the load of the settings, the proposals of termination (series, parallel, Thevenin
//! and RC) with the result of each one, and the slowing of the edge by the loss of the line.
//!
//! The line is the one of the main layer and width of the net; branches, vias and the pads of
//! the parts are not in the model. Everything is quasi-static and lossless in the reflections.

use serde::Serialize;

use crate::bergeron::{bounce_table, simulate, Bounce, Input, Limits, Outcome, Termination};
use crate::codes::{
    Finding, Severity, LONG_LINE, LOSS_DEGRADES, OVERSHOOT, RINGBACK, SIM_TRUNCATED,
};
use crate::env::Env;
use crate::netdata::NetData;
use crate::phys::{
    bandwidth_ghz, is_long_line, lowpass_cutoff_ghz, lowpass_rise_ps, rise_after, round_to,
};
use crate::settings::SiSettings;

/// A termination that was tried and what it gave.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Proposal {
    /// The kind: `series`, `parallel`, `thevenin` or `rc`.
    pub name: &'static str,
    /// The values of the parts.
    pub values: String,
    /// The voltage at the load.
    pub outcome: Outcome,
}

/// The slowing of the edge along a fraction of the line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Degradation {
    /// The fraction of the line.
    pub fraction: f64,
    /// The length of that fraction, in millimetres.
    pub length_mm: f64,
    /// The loss at the bandwidth of the edge over that length, in decibel.
    pub loss_db: f64,
    /// The rise time at the end of that length, in picoseconds.
    pub rise_out_ps: f64,
}

/// The result for one net.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NetReflect {
    /// The net.
    pub net: String,
    /// The layer most of it runs on.
    pub layer: String,
    /// The copper length, in millimetres.
    pub length_mm: f64,
    /// The impedance of the line, in ohm.
    pub z0_ohm: f64,
    /// The delay, in picoseconds.
    pub td_ps: f64,
    /// The rise time of the driver, in picoseconds.
    pub rise_ps: f64,
    /// The bandwidth of the edge, in GHz.
    pub bandwidth_ghz: f64,
    /// Whether `tr < 2 Td`.
    pub long_line: bool,
    /// The output resistance of the driver, in ohm.
    pub rs_ohm: f64,
    /// The resistance of the load, in ohm, if there is one.
    pub rl_ohm: Option<f64>,
    /// The capacitance of the load, in picofarad.
    pub cl_pf: f64,
    /// The loss of the whole line at the bandwidth of the edge, in decibel.
    pub loss_db: f64,
    /// The rise time at the end of the whole line, in picoseconds.
    pub rise_out_ps: f64,
    /// The rise time along the line.
    pub degradation: Vec<Degradation>,
    /// The diagram of bounces for a step that rises at once and the resistance of the load only.
    pub bounces: Vec<Bounce>,
    /// The voltage at the load as it is; only for a long line.
    pub baseline: Option<Outcome>,
    /// The terminations that were tried; only for a long line.
    pub proposals: Vec<Proposal>,
    /// The proposal with the least ringing, if it is better than the line as it is.
    pub best: Option<&'static str>,
}

fn rounded(o: &Outcome) -> Outcome {
    Outcome {
        final_level: round_to(o.final_level, 4),
        overshoot_pct: round_to(o.overshoot_pct, 2),
        ringback_pct: round_to(o.ringback_pct, 2),
        settle_ps: o.settle_ps.map(|t| round_to(t, 1)),
        step_ps: round_to(o.step_ps, 3),
        steps: o.steps,
        truncated: o.truncated,
    }
}

/// The worse of the overshoot and the ringback of an outcome.
fn worst(o: &Outcome) -> f64 {
    o.overshoot_pct.max(o.ringback_pct)
}

/// Looks at every net of the list.
pub(crate) fn analyse(
    nets: &[NetData],
    settings: &SiSettings,
    env: &mut Env,
) -> (Vec<NetReflect>, Vec<Finding>) {
    let mut out = Vec::new();
    let mut findings = Vec::new();
    for net in nets {
        out.push(one(net, settings, env, &mut findings));
    }
    (out, findings)
}

fn one(
    net: &NetData,
    settings: &SiSettings,
    env: &mut Env,
    findings: &mut Vec<Finding>,
) -> NetReflect {
    let subject = Some(net.name.as_str());
    let tr = settings.rise_for(&net.name);
    let bw = bandwidth_ghz(tr);
    let line = env.line(&net.layer, net.width_nm, bw);
    let length_mm = net.length_nm as f64 / 1.0e6;
    let long_line = is_long_line(tr, net.td_ps);
    let rs = settings.rs_for(&net.name);
    let rl = settings.rl_for(&net.name);
    let cl = settings.cl_for(&net.name);
    let loss_db = line.loss_db_per_cm * length_mm / 10.0;
    let rise_out = |fraction: f64| match lowpass_cutoff_ghz(bw, loss_db * fraction, 1) {
        Some(fc) => rise_after(tr, lowpass_rise_ps(fc, 1)),
        None => tr,
    };
    let degradation: Vec<Degradation> = [0.25, 0.5, 0.75, 1.0]
        .iter()
        .map(|f| Degradation {
            fraction: *f,
            length_mm: round_to(length_mm * f, 3),
            loss_db: round_to(loss_db * f, 4),
            rise_out_ps: round_to(rise_out(*f), 1),
        })
        .collect();
    let rise_out_ps = rise_out(1.0);
    let slowed = (rise_out_ps - tr) / tr * 100.0;
    if slowed >= settings.rise_degradation_warn_pct {
        findings.push(Finding::new(
            LOSS_DEGRADES,
            Severity::Warning,
            subject,
            format!(
                "the rise time grows from {tr:.0} ps to {rise_out_ps:.0} ps ({loss_db:.2} dB at {bw:.2} GHz)"
            ),
        ));
    }
    let bounces = bounce_table(line.z0, net.td_ps, rs, rl, settings.bounces);
    let mut result = NetReflect {
        net: net.name.clone(),
        layer: net.layer.clone(),
        length_mm: round_to(length_mm, 3),
        z0_ohm: round_to(line.z0, 2),
        td_ps: round_to(net.td_ps, 2),
        rise_ps: tr,
        bandwidth_ghz: round_to(bw, 4),
        long_line,
        rs_ohm: rs,
        rl_ohm: rl,
        cl_pf: cl,
        loss_db: round_to(loss_db, 4),
        rise_out_ps: round_to(rise_out_ps, 1),
        degradation,
        bounces: bounces
            .iter()
            .map(|b| Bounce {
                time_ps: round_to(b.time_ps, 2),
                at: b.at,
                incident_v: round_to(b.incident_v, 4),
                node_v: round_to(b.node_v, 4),
            })
            .collect(),
        baseline: None,
        proposals: Vec::new(),
        best: None,
    };
    if !long_line {
        return result;
    }
    findings.push(Finding::new(
        LONG_LINE,
        Severity::Info,
        subject,
        format!(
            "rise time {tr:.0} ps is under twice the delay {:.0} ps",
            net.td_ps
        ),
    ));
    let limits = Limits {
        steps_per_rise: settings.steps_per_rise,
        delays: settings.delays_simulated,
        max_steps: settings.max_steps,
        settle_band_pct: settings.settle_band_pct,
    };
    let run = |term: Termination| {
        simulate(
            &Input {
                z0: line.z0,
                td_ps: net.td_ps,
                rs,
                rl,
                cl_pf: cl,
                tr_ps: tr,
                term,
            },
            &limits,
        )
    };
    let baseline = run(Termination::None);
    let mut tried: Vec<(&'static str, String, Termination)> = Vec::new();
    if line.z0 - rs > 0.5 {
        let r = line.z0 - rs;
        tried.push((
            "series",
            format!("R={r:.1} ohm at the driver"),
            Termination::Series(r),
        ));
    }
    tried.push((
        "parallel",
        format!("R={:.1} ohm to ground at the load", line.z0),
        Termination::Parallel(line.z0),
    ));
    tried.push((
        "thevenin",
        format!(
            "R1=R2={:.1} ohm to the supply and to ground at the load",
            2.0 * line.z0
        ),
        Termination::Thevenin(2.0 * line.z0, 2.0 * line.z0),
    ));
    let c_pf = settings.rc_tau_over_td * net.td_ps / line.z0;
    tried.push((
        "rc",
        format!(
            "R={:.1} ohm and C={:.1} pF to ground at the load",
            line.z0, c_pf
        ),
        Termination::Rc(line.z0, c_pf),
    ));
    let mut best: Option<(&'static str, f64)> = None;
    let mut cut = baseline.truncated;
    for (name, values, term) in tried {
        let outcome = run(term);
        cut |= outcome.truncated;
        let score = round_to(worst(&outcome), 2);
        if outcome.final_level >= 0.5 && best.is_none_or(|(_, b)| score < b) {
            best = Some((name, score));
        }
        result.proposals.push(Proposal {
            name,
            values,
            outcome: rounded(&outcome),
        });
    }
    if cut {
        findings.push(Finding::new(
            SIM_TRUNCATED,
            Severity::Warning,
            subject,
            format!("at {} steps", settings.max_steps),
        ));
    }
    let base_worst = round_to(worst(&baseline), 2);
    if baseline.overshoot_pct >= settings.overshoot_warn_pct {
        let severity = if baseline.overshoot_pct >= settings.overshoot_error_pct {
            Severity::Error
        } else {
            Severity::Warning
        };
        findings.push(Finding::new(
            OVERSHOOT,
            severity,
            subject,
            format!(
                "{:.1} % with Rs {rs:.1} ohm, Z0 {:.1} ohm{}",
                baseline.overshoot_pct,
                line.z0,
                hint(best)
            ),
        ));
    }
    if baseline.ringback_pct >= settings.ringback_warn_pct {
        findings.push(Finding::new(
            RINGBACK,
            Severity::Warning,
            subject,
            format!(
                "{:.1} % under the final level{}",
                baseline.ringback_pct,
                hint(best)
            ),
        ));
    }
    result.baseline = Some(rounded(&baseline));
    result.best = best
        .filter(|(_, score)| *score < base_worst)
        .map(|(n, _)| n);
    result
}

fn hint(best: Option<(&'static str, f64)>) -> String {
    match best {
        Some((name, score)) => format!("; the {name} termination brings the worst to {score:.1} %"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(td_ps: f64, length_nm: i64) -> NetData {
        NetData {
            name: "CLK".to_owned(),
            length_nm,
            td_ps,
            layer: "F.Cu".to_owned(),
            width_nm: 200_000,
        }
    }

    fn run(settings: &SiSettings, data: &NetData) -> (NetReflect, Vec<Finding>) {
        let mut env = Env::new(None, settings);
        let (mut list, findings) = analyse(std::slice::from_ref(data), settings, &mut env);
        (list.remove(0), findings)
    }

    #[test]
    fn a_short_line_is_left_alone() {
        let settings = SiSettings::default();
        // 40 ps against a rise time of 500 ps
        let (r, findings) = run(&settings, &net(40.0, 6_000_000));
        assert!(!r.long_line);
        assert!(r.baseline.is_none() && r.proposals.is_empty());
        assert!(findings.iter().all(|f| f.code != LONG_LINE));
        assert_eq!(r.bounces.len(), 16);
    }

    #[test]
    fn a_long_line_with_a_weak_driver_rings_and_a_termination_is_proposed() {
        let settings = SiSettings {
            rs_ohm: 10.0,
            cl_pf: 0.0,
            ..SiSettings::default()
        };
        // 1 ns on a line of 50 ohm (the default of the settings)
        let (r, findings) = run(&settings, &net(1000.0, 170_000_000));
        assert!(r.long_line);
        let base = r.baseline.as_ref().unwrap();
        assert!(base.overshoot_pct > 30.0, "{}", base.overshoot_pct);
        assert!(findings.iter().any(|f| f.code == OVERSHOOT));
        assert!(findings.iter().any(|f| f.code == LONG_LINE));
        assert_eq!(r.proposals.len(), 4);
        assert_eq!(r.proposals[0].name, "series");
        // the series resistor that completes the source to Z0 removes the ringing
        assert_eq!(r.best, Some("series"));
        let series = &r.proposals[0].outcome;
        assert!(series.overshoot_pct < 1.0, "{}", series.overshoot_pct);
    }

    #[test]
    fn a_driver_that_matches_the_line_needs_no_series_proposal() {
        let settings = SiSettings {
            rs_ohm: 50.0,
            cl_pf: 0.0,
            ..SiSettings::default()
        };
        let (r, findings) = run(&settings, &net(1000.0, 170_000_000));
        assert!(r.proposals.iter().all(|p| p.name != "series"));
        assert!(findings.iter().all(|f| f.code != OVERSHOOT));
        assert_eq!(r.best, None);
    }

    #[test]
    fn the_rise_time_grows_along_a_lossy_line() {
        let settings = SiSettings {
            rise_time_ps: 100.0,
            ..SiSettings::default()
        };
        // 300 mm at the loss of the defaults
        let (r, findings) = run(&settings, &net(1700.0, 300_000_000));
        assert!(r.loss_db > 1.0, "{}", r.loss_db);
        assert!(r.rise_out_ps > 100.0);
        let fractions: Vec<f64> = r.degradation.iter().map(|d| d.rise_out_ps).collect();
        assert!(fractions.windows(2).all(|w| w[1] > w[0]), "{fractions:?}");
        assert!(findings.iter().any(|f| f.code == LOSS_DEGRADES));
    }

    #[test]
    fn the_results_are_the_same_every_time() {
        let settings = SiSettings::default();
        let a = run(&settings, &net(800.0, 140_000_000));
        let b = run(&settings, &net(800.0, 140_000_000));
        assert_eq!(a, b);
    }
}
