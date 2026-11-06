// SPDX-License-Identifier: GPL-3.0-or-later
//! A report of a check as a table of CSV and as a page of HTML, grouped by category.
//!
//! The categories group the stable codes by what a designer fixes together (sizes, clearances, the
//! edge and the holes, the mask and the silkscreen, zones, connectivity, the advanced checks).
//! The CSV keeps the stable English keys in its header and in its category and severity columns,
//! so a script can read it in any language; only the title of a finding is in the language asked.
//! The HTML has no script and no outside file, every text that can come from a board file is
//! escaped, and the headings are in the language asked. The same report always gives the same
//! bytes.

use std::fmt::Write as _;

use crate::codes::{title, word, Lang};
use crate::report::{Finding, Report, Severity};

/// The categories, in the order they are listed.
pub const CATEGORIES: [&str; 8] = [
    "size",
    "clearance",
    "edge",
    "mask_silk",
    "zones",
    "connectivity",
    "advanced",
    "other",
];

/// The header line of the CSV.
pub const CSV_HEADER: &str =
    "list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail";

/// The category of a code: one of [`CATEGORIES`].
pub fn category(code: &str) -> &'static str {
    match code {
        "PHOENIX-CHECK-001" | "PHOENIX-CHECK-002" | "PHOENIX-CHECK-003" | "PHOENIX-CHECK-004"
        | "PHOENIX-CHECK-014" | "PHOENIX-CHECK-015" | "PHOENIX-DRC-001" | "PHOENIX-DRC-004"
        | "PHOENIX-DRC-006" => "size",
        "PHOENIX-CHECK-005" | "PHOENIX-DRC-008" => "clearance",
        "PHOENIX-CHECK-006" | "PHOENIX-CHECK-007" | "PHOENIX-DRC-002" | "PHOENIX-DRC-003" => "edge",
        "PHOENIX-CHECK-008" | "PHOENIX-CHECK-011" | "PHOENIX-CHECK-012" | "PHOENIX-DRC-005" => {
            "mask_silk"
        }
        "PHOENIX-CHECK-009" | "PHOENIX-CHECK-010" => "zones",
        "PHOENIX-DRC-014" | "PHOENIX-DRC-015" | "PHOENIX-CHECK-013" => "other",
        _ if code.starts_with("PHOENIX-CONN-") => "connectivity",
        _ if code.starts_with("PHOENIX-DRC-") => "advanced",
        _ => "other",
    }
}

/// The findings of each category: the proven ones and the inconclusive ones, for the categories
/// that have any, in the order of [`CATEGORIES`].
pub fn category_counts(report: &Report) -> Vec<(&'static str, usize, usize)> {
    CATEGORIES
        .iter()
        .filter_map(|&name| {
            let proven = report
                .confirmed
                .iter()
                .filter(|f| category(f.code) == name)
                .count();
            let open = report
                .inconclusive
                .iter()
                .filter(|f| category(f.code) == name)
                .count();
            (proven + open > 0).then_some((name, proven, open))
        })
        .collect()
}

fn csv_cell(text: &str) -> String {
    let guarded = if text.starts_with(['=', '+', '-', '@']) {
        format!("'{text}")
    } else {
        text.to_owned()
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

fn number(value: Option<i64>) -> String {
    value.map_or_else(String::new, |v| v.to_string())
}

fn csv_row(list: &str, f: &Finding, lang: Lang) -> String {
    let (x, y) = f.at_nm.map_or((String::new(), String::new()), |[x, y]| {
        (x.to_string(), y.to_string())
    });
    format!(
        "{list},{},{},{},{},{},{x},{y},{},{},{}\n",
        category(f.code),
        f.severity.as_str(),
        f.code,
        csv_cell(title(f.code, lang).unwrap_or("")),
        csv_cell(&f.location),
        number(f.measured_nm),
        number(f.limit_nm),
        csv_cell(&f.detail),
    )
}

/// The report as CSV: the header, then the proven findings and then the inconclusive ones, each
/// list in the order of the report, with `\n` line ends. The title is in `lang`.
pub fn report_csv(report: &Report, lang: Lang) -> String {
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    for f in &report.confirmed {
        out.push_str(&csv_row("confirmed", f, lang));
    }
    for f in &report.inconclusive {
        out.push_str(&csv_row("inconclusive", f, lang));
    }
    out
}

fn severity_word(severity: Severity, lang: Lang) -> &'static str {
    match severity {
        Severity::Info => word("sev_info", lang),
        Severity::Warning => word("sev_warning", lang),
        Severity::Error => word("sev_error", lang),
    }
}

fn html_table(findings: &[&Finding], lang: Lang, out: &mut String) {
    out.push_str("<table>\n<tr>");
    for key in [
        "col_severity",
        "col_code",
        "col_title",
        "col_location",
        "col_position",
        "measured",
        "limit",
        "col_detail",
    ] {
        let _ = write!(out, "<th>{}</th>", html_escape(word(key, lang)));
    }
    out.push_str("</tr>\n");
    for f in findings {
        let place = f
            .at_nm
            .map_or_else(String::new, |[x, y]| format!("({x}, {y}) nm"));
        let measured = f
            .measured_nm
            .map_or_else(String::new, |v| format!("{v} nm"));
        let limit = f.limit_nm.map_or_else(String::new, |v| format!("{v} nm"));
        let _ = writeln!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(severity_word(f.severity, lang)),
            html_escape(f.code),
            html_escape(title(f.code, lang).unwrap_or("")),
            html_escape(&f.location),
            html_escape(&place),
            html_escape(&measured),
            html_escape(&limit),
            html_escape(&f.detail),
        );
    }
    out.push_str("</table>\n");
}

/// The report as a page of HTML: a summary with the number of findings of each category, then a
/// table for each category with findings, the proven ones before the inconclusive ones.
pub fn report_html(report: &Report, lang: Lang) -> String {
    let heading = html_escape(word("report_title", lang));
    let mut out = String::new();
    let _ = write!(
        out,
        "<!DOCTYPE html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{heading}</title>\n",
        lang.tag()
    );
    out.push_str(
        "<style>body{font-family:sans-serif}table{border-collapse:collapse;margin-bottom:1em}",
    );
    out.push_str("td,th{border:1px solid #888;padding:2px 6px;text-align:left}</style>\n");
    let _ = write!(out, "</head>\n<body>\n<h1>{heading}</h1>\n");
    let _ = writeln!(out, "<h2>{}</h2>", html_escape(word("summary", lang)));
    if report.confirmed.is_empty() && report.inconclusive.is_empty() {
        let _ = writeln!(out, "<p>{}</p>", html_escape(word("nothing", lang)));
    } else {
        out.push_str("<table>\n<tr>");
        let _ = write!(
            out,
            "<th>{}</th><th>{}</th><th>{}</th>",
            html_escape(word("col_category", lang)),
            html_escape(word("confirmed", lang)),
            html_escape(word("inconclusive", lang))
        );
        out.push_str("</tr>\n");
        for (name, proven, open) in category_counts(report) {
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td>{proven}</td><td>{open}</td></tr>",
                html_escape(word(&format!("cat_{name}"), lang))
            );
        }
        out.push_str("</table>\n");
    }
    for (name, _, _) in category_counts(report) {
        let _ = writeln!(
            out,
            "<h2>{}</h2>",
            html_escape(word(&format!("cat_{name}"), lang))
        );
        for (key, list) in [
            ("confirmed", &report.confirmed),
            ("inconclusive", &report.inconclusive),
        ] {
            let rows: Vec<&Finding> = list.iter().filter(|f| category(f.code) == name).collect();
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "<h3>{}: {}</h3>",
                html_escape(word(key, lang)),
                rows.len()
            );
            html_table(&rows, lang, &mut out);
        }
    }
    out.push_str("</body>\n</html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{CLEARANCE, EDGE_CLOSE, NET_OPEN, NOT_JUDGED, TRACK_NARROW};

    fn finding(code: &'static str, location: &str, severity: Severity) -> Finding {
        Finding::new(5, "x", code, severity, location.to_owned())
    }

    #[test]
    fn a_code_belongs_to_the_category_a_designer_fixes_it_in() {
        assert_eq!(category(TRACK_NARROW), "size");
        assert_eq!(category(CLEARANCE), "clearance");
        assert_eq!(category(EDGE_CLOSE), "edge");
        assert_eq!(category("PHOENIX-CHECK-011"), "mask_silk");
        assert_eq!(category("PHOENIX-CHECK-010"), "zones");
        assert_eq!(category(NET_OPEN), "connectivity");
        assert_eq!(category("PHOENIX-DRC-009"), "advanced");
        assert_eq!(category("PHOENIX-DRC-001"), "size");
        assert_eq!(category(NOT_JUDGED), "other");
        assert_eq!(category("PHOENIX-NOPE-001"), "other");
    }

    #[test]
    fn the_counts_follow_the_fixed_order_of_the_categories() {
        let mut r = Report::new("check");
        r.confirmed.push(finding(NET_OPEN, "n", Severity::Error));
        r.confirmed.push(finding(CLEARANCE, "a", Severity::Error));
        r.confirmed.push(finding(CLEARANCE, "b", Severity::Warning));
        r.inconclusive
            .push(finding(TRACK_NARROW, "c", Severity::Info));
        assert_eq!(
            category_counts(&r),
            vec![("size", 0, 1), ("clearance", 2, 0), ("connectivity", 1, 0)]
        );
    }

    #[test]
    fn the_csv_has_one_row_per_finding_with_stable_keys_and_a_translated_title() {
        let mut r = Report::new("check");
        r.confirmed.push(
            finding(CLEARANCE, "track x", Severity::Error)
                .at(7, 0)
                .values(100, 200),
        );
        let en = report_csv(&r, Lang::En);
        let mut lines = en.lines();
        assert_eq!(lines.next(), Some(CSV_HEADER));
        assert_eq!(
            lines.next(),
            Some(
                "confirmed,clearance,error,PHOENIX-CHECK-005,Copper clearance violated,track x,7,0,100,200,"
            )
        );
        assert_eq!(lines.next(), None);
        let pt = report_csv(&r, Lang::Pt);
        assert!(pt.contains(",Distância do cobre não respeitada,"));
        assert!(pt.contains("confirmed,clearance,error,"));
    }

    #[test]
    fn a_cell_that_a_spreadsheet_would_run_is_guarded_and_a_comma_is_quoted() {
        let mut r = Report::new("check");
        r.inconclusive
            .push(finding(NOT_JUDGED, "=SUM(A1)", Severity::Info).why("a, \"b\""));
        let csv = report_csv(&r, Lang::En);
        let row = csv.lines().nth(1).unwrap_or("");
        assert!(row.starts_with("inconclusive,other,info,PHOENIX-CHECK-013,"));
        assert!(row.contains(",'=SUM(A1),,,,,\"a, \"\"b\"\"\""));
    }

    #[test]
    fn the_html_escapes_the_board_text_and_names_the_categories_in_the_language() {
        let mut r = Report::new("check");
        r.confirmed
            .push(finding(CLEARANCE, "<b>R1</b> & \"C2\"", Severity::Error).at(1, 2));
        let es = report_html(&r, Lang::Es);
        assert!(es.contains("<html lang=\"es\">"));
        assert!(es.contains("&lt;b&gt;R1&lt;/b&gt; &amp; &quot;C2&quot;"));
        assert!(!es.contains("<b>"));
        assert!(!es.contains("<script"));
        assert!(es.contains("<h2>Distancias</h2>"));
        assert!(es.contains("<td>(1, 2) nm</td>"));
        assert!(es.contains("<h3>Confirmados: 1</h3>"));
        assert_eq!(es, report_html(&r, Lang::Es));
        let empty = report_html(&Report::new("check"), Lang::En);
        assert!(empty.contains("<p>nothing found</p>"));
        assert!(!empty.contains("<h3>"));
    }
}
