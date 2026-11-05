// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of a fill and of a check, as text and as JSON. The same board always gives the same
//! bytes: nothing here depends on the time, the order of a map or the machine.

use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::Lang;
use serde::Serialize;

use crate::codes::{Finding, Severity};
use crate::fill::FillReport;

fn net_name(board: &Board, id: eh3d_phoenix_board::Id) -> &str {
    board.nets.get(&id).map_or("?", |n| n.name.as_str())
}

fn layer_name(board: &Board, id: eh3d_phoenix_board::Id) -> &str {
    board.layers.get(&id).map_or("?", |l| l.name.as_str())
}

/// The report of a fill as text: one line per zone, then the findings.
pub fn fill_text(report: &FillReport, board: &Board, lang: Lang) -> String {
    let mut out = String::new();
    let polygons: usize = report.fill.zones.iter().map(|z| z.polygons.len()).sum();
    let area: i128 = report
        .fill
        .zones
        .iter()
        .map(|z| i128::from(z.area_nm2))
        .sum();
    out.push_str(&format!(
        "filled {} zones: {} polygons, {} square nm of copper\n",
        report.fill.zones.len(),
        polygons,
        area
    ));
    for stats in &report.stats {
        let zone = report.fill.zones.iter().find(|z| z.zone == stats.zone);
        let (net, layer, priority) = zone.map_or(("?", "?", 0), |z| {
            (
                net_name(board, z.net),
                layer_name(board, z.layer),
                z.priority,
            )
        });
        out.push_str(&format!(
            "zone {} net {} layer {} priority {}: {} polygons from {} pieces, {} islands kept, {} dropped, {} square nm\n",
            stats.zone,
            net,
            layer,
            priority,
            stats.polygons,
            stats.pieces,
            stats.islands_kept,
            stats.islands_removed,
            stats.area_nm2
        ));
    }
    out.push_str(&findings_text(&report.findings, lang));
    out
}

/// The findings as text, one line each, errors first. Nothing at all gives the line "no findings".
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

/// The numbers of the findings by severity: errors, warnings and information.
pub fn counts(findings: &[Finding]) -> (usize, usize, usize) {
    let of = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    (
        of(Severity::Error),
        of(Severity::Warning),
        of(Severity::Info),
    )
}

/// The result of a check as it is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    /// The proven breaks of a rule.
    pub errors: usize,
    /// The warnings.
    pub warnings: usize,
    /// The findings, in the order they were made.
    pub findings: Vec<Finding>,
}

impl CheckReport {
    /// The report of a list of findings.
    pub fn new(findings: Vec<Finding>) -> CheckReport {
        let (errors, warnings, _) = counts(&findings);
        CheckReport {
            errors,
            warnings,
            findings,
        }
    }

    /// The report as text: the numbers, then the findings.
    pub fn to_text(&self, lang: Lang) -> String {
        format!(
            "check: {} errors, {} warnings\n{}",
            self.errors,
            self.warnings,
            findings_text(&self.findings, lang)
        )
    }

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        json_of(self)
    }
}

/// The report of a fill as JSON.
pub fn fill_json(report: &FillReport) -> String {
    json_of(report)
}

/// The file of filled zones as JSON, ready to be written.
pub fn fill_file_json(report: &FillReport) -> String {
    json_of(&report.fill)
}

fn json_of<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{CLEARANCE_VIOLATED, ISLAND_SMALL};

    #[test]
    fn the_findings_are_listed_errors_first_and_counted() {
        let list = vec![
            Finding::new(ISLAND_SMALL, Severity::Info, None, "a"),
            Finding::new(CLEARANCE_VIOLATED, Severity::Error, None, "b"),
            Finding::new(ISLAND_SMALL, Severity::Warning, None, "c"),
        ];
        assert_eq!(counts(&list), (1, 1, 1));
        let text = findings_text(&list, Lang::En);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(
            lines[0].starts_with("error: PHOENIX-ZONE-006"),
            "{}",
            lines[0]
        );
        assert!(lines[1].starts_with("warning:"), "{}", lines[1]);
        assert!(lines[2].starts_with("info:"), "{}", lines[2]);
        assert_eq!(findings_text(&[], Lang::En), "no findings\n");
    }

    #[test]
    fn a_check_report_has_its_numbers_in_text_and_in_json() {
        let report = CheckReport::new(vec![Finding::new(
            CLEARANCE_VIOLATED,
            Severity::Error,
            None,
            "R1 pad 1 is closer than 200000 nm",
        )]);
        assert_eq!(report.errors, 1);
        assert_eq!(report.warnings, 0);
        let text = report.to_text(Lang::Es);
        assert!(text.starts_with("check: 1 errors, 0 warnings\n"), "{text}");
        assert!(text.contains("El relleno está más cerca"), "{text}");
        let json = report.to_json();
        assert!(json.contains("\"errors\": 1"), "{json}");
        assert!(json.contains("PHOENIX-ZONE-006"), "{json}");
        assert!(json.ends_with("}\n"), "{json}");
    }
}
