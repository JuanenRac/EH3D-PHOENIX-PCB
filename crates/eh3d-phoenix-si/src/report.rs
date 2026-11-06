// SPDX-License-Identifier: GPL-3.0-or-later
//! The text and the JSON of a report. The same board and settings always give the same bytes: the
//! order is fixed, the numbers are rounded to a fixed count of decimals and nothing depends on the
//! time or the machine.

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

use crate::codes::{Finding, Severity};
use crate::crosstalk::CrosstalkReport;
use crate::eyenet::NetEye;
use crate::reflect::NetReflect;
use crate::returnpath::ReturnReport;

/// What every report says about itself.
pub const STATE_NOTE: &str = "prototype: quasi-static estimates from closed formulas, not compared with a field solver, a circuit simulator or a measurement";

/// The report of one command.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SiReport {
    /// The command: `crosstalk`, `reflect`, `eye` or `check`.
    pub command: &'static str,
    /// The state of the estimates.
    pub state: &'static str,
    /// The crosstalk between parallel tracks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crosstalk: Option<CrosstalkReport>,
    /// The terminations and reflections of the nets.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nets: Vec<NetReflect>,
    /// The eye diagrams of the nets.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eyes: Vec<NetEye>,
    /// The return path of the tracks and the stubs of the vias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns: Option<ReturnReport>,
    /// What the analysis found, against the thresholds of the settings.
    pub findings: Vec<Finding>,
}

impl SiReport {
    /// Whether any finding is an error.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// The numbers of the findings by severity: errors, warnings and information.
pub fn counts(findings: &[Finding]) -> (usize, usize, usize) {
    let of = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    (
        of(Severity::Error),
        of(Severity::Warning),
        of(Severity::Info),
    )
}

/// The findings as text, one line each, errors first.
pub fn findings_text(findings: &[Finding], lang: Lang) -> String {
    if findings.is_empty() {
        return "no findings\n".to_owned();
    }
    let mut sorted: Vec<&Finding> = findings.iter().collect();
    sorted.sort_by_key(|f| std::cmp::Reverse(f.severity));
    let mut out = String::new();
    for finding in sorted {
        out.push_str(&finding.line(lang));
        out.push('\n');
    }
    out
}

fn crosstalk_text(c: &CrosstalkReport, out: &mut String) {
    out.push_str(&format!(
        "crosstalk: {} coupled pairs of nets\n",
        c.rows.len() / 2
    ));
    for r in &c.rows {
        out.push_str(&format!(
            "  {} -> {} on {}: {:.3} mm parallel, {:.1} um between centres, gap {:.1} um, Lm/L {:.4}, Cm/C {:.4}, tr {} ps: NEXT {:.3} %, FEXT {:.3} %{}\n",
            r.aggressor,
            r.victim,
            r.layer,
            r.parallel_mm,
            r.distance_um,
            r.gap_um,
            r.lm_over_l,
            r.cm_over_c,
            r.rise_ps,
            r.next_pct,
            r.fext_pct,
            if r.strong { " (coupling too strong for the estimate)" } else { "" }
        ));
    }
    for v in &c.victims {
        out.push_str(&format!(
            "  victim {}: {} aggressors, NEXT sum {:.3} %, FEXT sum {:.3} %\n",
            v.victim, v.aggressors, v.next_sum_pct, v.fext_sum_pct
        ));
    }
    if c.arcs_skipped > 0 {
        out.push_str(&format!("  {} arcs were not looked at\n", c.arcs_skipped));
    }
}

fn reflect_text(n: &NetReflect, out: &mut String) {
    out.push_str(&format!(
        "net {} ({}): {:.3} mm, Z0 {:.2} ohm, Td {:.2} ps, tr {} ps (bandwidth {:.4} GHz), {}\n",
        n.net,
        n.layer,
        n.length_mm,
        n.z0_ohm,
        n.td_ps,
        n.rise_ps,
        n.bandwidth_ghz,
        if n.long_line {
            "electrically long (tr < 2 Td)"
        } else {
            "electrically short"
        }
    ));
    let load = match n.rl_ohm {
        Some(r) => format!("{r} ohm"),
        None => "open".to_owned(),
    };
    out.push_str(&format!(
        "  driver {} ohm, load {} and {} pF; loss {:.4} dB at the bandwidth, rise time at the end {:.1} ps\n",
        n.rs_ohm, load, n.cl_pf, n.loss_db, n.rise_out_ps
    ));
    for d in &n.degradation {
        out.push_str(&format!(
            "  after {:.3} mm ({:.0} %): loss {:.4} dB, rise time {:.1} ps\n",
            d.length_mm,
            d.fraction * 100.0,
            d.loss_db,
            d.rise_out_ps
        ));
    }
    for b in &n.bounces {
        out.push_str(&format!(
            "  bounce at {:.2} ps at the {}: wave {:.4} V, node {:.4} V\n",
            b.time_ps, b.at, b.incident_v, b.node_v
        ));
    }
    let settle = |t: Option<f64>| match t {
        Some(t) => format!("{t:.1} ps"),
        None => "not settled".to_owned(),
    };
    if let Some(o) = &n.baseline {
        out.push_str(&format!(
            "  as it is: final level {:.4}, overshoot {:.2} %, ringback {:.2} %, settled at {}\n",
            o.final_level,
            o.overshoot_pct,
            o.ringback_pct,
            settle(o.settle_ps)
        ));
    }
    for p in &n.proposals {
        out.push_str(&format!(
            "  {} ({}): final level {:.4}, overshoot {:.2} %, ringback {:.2} %, settled at {}\n",
            p.name,
            p.values,
            p.outcome.final_level,
            p.outcome.overshoot_pct,
            p.outcome.ringback_pct,
            settle(p.outcome.settle_ps)
        ));
    }
    if let Some(best) = n.best {
        out.push_str(&format!("  least ringing: {best}\n"));
    }
}

fn eye_text(e: &NetEye, out: &mut String) {
    let cutoff = match e.cutoff_ghz {
        Some(c) => format!("{c} GHz"),
        None => "none".to_owned(),
    };
    out.push_str(&format!(
        "eye {}: {} Mbit/s (bit {} ps), tr {} ps, loss {:.4} dB at {} GHz, cutoff {} (order {}), PRBS{} of {} bits: height {:.1} %, width {:.1} %, jitter {:.2} ps\n",
        e.net,
        e.bit_rate_mbps,
        e.ui_ps,
        e.rise_ps,
        e.loss_db,
        e.nyquist_ghz,
        cutoff,
        e.order,
        e.prbs,
        e.bits,
        e.height * 100.0,
        e.width * 100.0,
        e.jitter_pp_ps
    ));
}

fn returns_text(r: &ReturnReport, out: &mut String) {
    out.push_str(&format!(
        "return path: {} tracks looked at over copper, {} broken, {} stubs\n",
        r.tracks_checked,
        r.crossings.len(),
        r.stubs.len()
    ));
    for c in &r.crossings {
        out.push_str(&format!(
            "  {} on {} over {}: {} changes{} near ({:.3}, {:.3}) mm\n",
            c.net,
            c.layer,
            c.reference,
            c.changes,
            if c.over_nothing {
                ", no copper under it"
            } else {
                ""
            },
            c.x_mm,
            c.y_mm
        ));
    }
    for s in &r.stubs {
        out.push_str(&format!(
            "  stub {} at ({:.3}, {:.3}) mm, {}: {:.3} mm, {:.2} ps, quarter wave at {:.2} GHz\n",
            s.net, s.x_mm, s.y_mm, s.side, s.length_mm, s.delay_ps, s.resonance_ghz
        ));
    }
}

/// The report as text.
pub fn report_text(report: &SiReport, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "signal integrity {}: {}\n",
        report.command, report.state
    ));
    if let Some(c) = &report.crosstalk {
        crosstalk_text(c, &mut out);
    }
    for n in &report.nets {
        reflect_text(n, &mut out);
    }
    for e in &report.eyes {
        eye_text(e, &mut out);
    }
    if let Some(r) = &report.returns {
        returns_text(r, &mut out);
    }
    let (errors, warnings, _) = counts(&report.findings);
    out.push_str(&format!("{errors} errors, {warnings} warnings\n"));
    out.push_str(&findings_text(&report.findings, lang));
    out
}

/// The report as JSON.
pub fn report_json(report: &SiReport) -> String {
    let mut text = serde_json::to_string_pretty(report).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{NEXT_HIGH, PLANE_SPLIT};

    fn empty(command: &'static str) -> SiReport {
        SiReport {
            command,
            state: STATE_NOTE,
            crosstalk: None,
            nets: Vec::new(),
            eyes: Vec::new(),
            returns: None,
            findings: Vec::new(),
        }
    }

    #[test]
    fn the_findings_are_listed_errors_first_and_counted() {
        let list = vec![
            Finding::new(PLANE_SPLIT, Severity::Warning, Some("A"), "split"),
            Finding::new(NEXT_HIGH, Severity::Error, Some("A/B"), "big"),
        ];
        assert_eq!(counts(&list), (1, 1, 0));
        let text = findings_text(&list, Lang::De);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("error: PHOENIX-SI-003"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("warning: PHOENIX-SI-012"),
            "{}",
            lines[1]
        );
        assert_eq!(findings_text(&[], Lang::En), "no findings\n");
    }

    #[test]
    fn an_empty_report_says_what_it_is_and_has_no_sections() {
        let report = empty("check");
        let text = report_text(&report, Lang::En);
        assert!(
            text.starts_with("signal integrity check: prototype"),
            "{text}"
        );
        assert!(text.contains("0 errors, 0 warnings"));
        let json = report_json(&report);
        assert!(json.contains("\"command\": \"check\""));
        assert!(!json.contains("crosstalk"));
        assert!(!json.contains("\"nets\""));
        assert!(!report.has_errors());
    }

    #[test]
    fn an_error_finding_makes_the_report_fail() {
        let mut report = empty("crosstalk");
        report
            .findings
            .push(Finding::new(NEXT_HIGH, Severity::Error, None, ""));
        assert!(report.has_errors());
    }
}
