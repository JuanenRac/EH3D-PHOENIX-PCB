// SPDX-License-Identifier: GPL-3.0-or-later
//! The text and the JSON of a length report. The same board always gives the same bytes: the
//! order is fixed, the numbers are integers and nothing depends on the time or the machine.

use eh3d_phoenix_rules::Lang;

use crate::analyse::LengthReport;
use crate::codes::{Finding, Severity};

/// A length in nanometres as millimetres with six decimals, exactly.
pub fn millimetres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
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

/// The report as text: the nets, the groups, the pairs and the findings.
pub fn report_text(report: &LengthReport, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!("nets: {}\n", report.nets.len()));
    for n in &report.nets {
        out.push_str(&format!(
            "net {}: {} mm ({} mm of track, {} vias of {} mm), delay {} fs\n",
            n.name,
            millimetres(n.total_nm),
            millimetres(n.tracks_nm),
            n.vias,
            millimetres(n.vias_nm),
            n.delay_fs
        ));
    }
    for g in &report.groups {
        let state = if g.ok { "ok" } else { "broken" };
        out.push_str(&format!(
            "group {}: {} nets from {} mm to {} mm, spread {} mm, tolerance {} mm: {}\n",
            g.name,
            g.nets.len(),
            millimetres(g.min_nm),
            millimetres(g.max_nm),
            millimetres(g.spread_nm),
            millimetres(g.tolerance_nm),
            state
        ));
    }
    for p in &report.pairs {
        let state = if p.ok { "ok" } else { "broken" };
        let gap = match (p.min_gap_nm, p.max_gap_nm) {
            (Some(lo), Some(hi)) => format!("{} mm to {} mm", millimetres(lo), millimetres(hi)),
            _ => "none".to_owned(),
        };
        out.push_str(&format!(
            "pair {}: {} {} mm, {} {} mm, skew {} mm of {} mm, delay skew {} fs, coupled {} mm, uncoupled {} mm, gap {}: {}\n",
            p.name,
            p.positive,
            millimetres(p.positive_nm),
            p.negative,
            millimetres(p.negative_nm),
            millimetres(p.skew_nm),
            millimetres(p.max_skew_nm),
            p.delay_skew_fs,
            millimetres(p.coupled_nm),
            millimetres(p.uncoupled_nm),
            gap,
            state
        ));
    }
    let (errors, warnings, _) = counts(&report.findings);
    out.push_str(&format!("{errors} errors, {warnings} warnings\n"));
    out.push_str(&findings_text(&report.findings, lang));
    out
}

/// The report as JSON.
pub fn report_json(report: &LengthReport) -> String {
    let mut text = serde_json::to_string_pretty(report).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{GROUP_MISMATCH, NET_NO_COPPER};

    #[test]
    fn a_length_is_written_in_millimetres_without_rounding() {
        assert_eq!(millimetres(0), "0.000000");
        assert_eq!(millimetres(1), "0.000001");
        assert_eq!(millimetres(15_707_963), "15.707963");
        assert_eq!(millimetres(-250_000), "-0.250000");
        assert_eq!(millimetres(20_000_000), "20.000000");
    }

    #[test]
    fn the_findings_are_listed_errors_first_and_counted() {
        let list = vec![
            Finding::new(NET_NO_COPPER, Severity::Warning, Some("A"), "zero"),
            Finding::new(GROUP_MISMATCH, Severity::Error, Some("g"), "big"),
        ];
        assert_eq!(counts(&list), (1, 1, 0));
        let text = findings_text(&list, Lang::De);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("error: PHOENIX-LEN-004"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("warning: PHOENIX-LEN-013"),
            "{}",
            lines[1]
        );
        assert_eq!(findings_text(&[], Lang::En), "no findings\n");
    }
}
