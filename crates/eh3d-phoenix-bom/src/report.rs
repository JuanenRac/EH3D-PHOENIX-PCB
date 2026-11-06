// SPDX-License-Identifier: GPL-3.0-or-later
//! The bill of materials as CSV, JSON, HTML and text. The same input always gives the same bytes:
//! the order is fixed, money is written as decimals with six places worked out in whole
//! millionths, and nothing depends on the time or the machine. Every cell of the CSV and the
//! HTML that comes from a file is quoted or escaped.

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Map, Value};

use crate::bom::{Bom, BomLine};
use crate::codes::{Finding, Severity};
use crate::money::format_micro;
use crate::text::{csv_raw, csv_text, html_escape};

/// The state of a line in one word: `ok`, `no_price`, `no_footprint` or both joined by `+`.
pub fn line_status(line: &BomLine) -> String {
    let mut parts = Vec::new();
    if line.purchase.is_none() {
        parts.push("no_price");
    }
    if line.footprint.is_empty() {
        parts.push("no_footprint");
    }
    if parts.is_empty() {
        "ok".to_owned()
    } else {
        parts.join("+")
    }
}

fn breaks_text(line: &BomLine) -> String {
    line.purchase.as_ref().map_or_else(String::new, |p| {
        p.tiers
            .iter()
            .map(|(q, price)| format!("{q}:{}", format_micro(i128::from(*price))))
            .collect::<Vec<_>>()
            .join(";")
    })
}

/// The cells of a line, as plain text: references, quantity, value, footprint, maker, part number,
/// supplier, supplier code, unit price, price break, quantity to buy, cost of the batch, cost per
/// board, price breaks and state.
fn cells(line: &BomLine) -> [String; 15] {
    let (supplier, code, unit, tier, buy, batch) = match &line.purchase {
        Some(p) => (
            p.supplier.clone(),
            p.supplier_code.clone(),
            format_micro(i128::from(p.unit_price_micro)),
            p.tier_min_qty.to_string(),
            p.buy_qty.to_string(),
            format_micro(p.cost_micro),
        ),
        None => (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
    };
    let per_board = if line.purchase.is_some() {
        format_micro(line.cost_per_board_micro)
    } else {
        String::new()
    };
    [
        line.references.join(" "),
        line.qty.to_string(),
        line.value.clone(),
        line.footprint.clone(),
        line.manufacturer.clone(),
        line.mpn.clone(),
        supplier,
        code,
        unit,
        tier,
        buy,
        batch,
        per_board,
        breaks_text(line),
        line_status(line),
    ]
}

/// The names of the columns.
pub const COLUMNS: [&str; 15] = [
    "references",
    "quantity",
    "value",
    "footprint",
    "manufacturer",
    "mpn",
    "supplier",
    "supplier_code",
    "unit_price",
    "price_break",
    "quantity_to_buy",
    "line_cost_batch",
    "line_cost_per_board",
    "price_breaks",
    "status",
];

/// The columns that hold numbers worked out by the program (not text from a file).
const NUMERIC: [usize; 6] = [1, 8, 9, 10, 11, 12];

/// The lines of the bill as CSV, with a header line and `\n` line ends.
pub fn bom_csv(bom: &Bom) -> String {
    let all: Vec<String> = COLUMNS.iter().map(|c| (*c).to_owned()).collect();
    bom_csv_with(bom, &all)
}

/// The lines of the bill as CSV with the columns named, in that order (`csv_columns` of
/// `config/costs.cfg`). A name that is not one of [`COLUMNS`] is skipped; the settings refuse it
/// before.
pub fn bom_csv_with(bom: &Bom, columns: &[String]) -> String {
    let indexes: Vec<usize> = columns
        .iter()
        .filter_map(|name| COLUMNS.iter().position(|c| *c == name.as_str()))
        .collect();
    let header: Vec<&str> = indexes.iter().map(|&i| COLUMNS[i]).collect();
    let mut out = header.join(",");
    out.push('\n');
    for line in &bom.lines {
        let all = cells(line);
        let row: Vec<String> = indexes
            .iter()
            .map(|&i| {
                if NUMERIC.contains(&i) {
                    csv_raw(&all[i])
                } else {
                    csv_text(&all[i])
                }
            })
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

fn finding_json(f: &Finding) -> Value {
    json!({
        "code": f.code,
        "severity": f.severity.as_str(),
        "subject": f.subject,
        "detail": f.detail,
    })
}

/// The whole bill as JSON (money as decimal strings with six places), ending with a line break.
pub fn bom_json(bom: &Bom) -> String {
    let lines: Vec<Value> = bom
        .lines
        .iter()
        .map(|l| {
            let mut map = Map::new();
            map.insert("references".into(), json!(l.references));
            map.insert("quantity".into(), json!(l.qty));
            map.insert("value".into(), json!(l.value));
            map.insert("footprint".into(), json!(l.footprint));
            map.insert("manufacturer".into(), json!(l.manufacturer));
            map.insert("mpn".into(), json!(l.mpn));
            map.insert("quantity_needed".into(), json!(l.needed));
            map.insert("status".into(), json!(line_status(l)));
            let offer = match &l.purchase {
                Some(p) => {
                    let breaks: Vec<Value> = p
                        .tiers
                        .iter()
                        .map(|(q, price)| {
                            json!({"from": q, "unit_price": format_micro(i128::from(*price))})
                        })
                        .collect();
                    json!({
                    "supplier": p.supplier,
                    "supplier_code": p.supplier_code,
                    "price_break": p.tier_min_qty,
                    "unit_price": format_micro(i128::from(p.unit_price_micro)),
                    "quantity_to_buy": p.buy_qty,
                    "cost_batch": format_micro(p.cost_micro),
                    "cost_per_board": format_micro(l.cost_per_board_micro),
                    "price_breaks": breaks,
                    })
                }
                None => Value::Null,
            };
            map.insert("offer".into(), offer);
            Value::Object(map)
        })
        .collect();
    let pcb = match &bom.pcb {
        Some(p) => json!({
            "layers": p.layers,
            "area_nm2": p.area_nm2.to_string(),
            "thickness_nm": p.thickness_nm,
            "holes": p.holes,
            "per_board": format_micro(p.per_board_micro),
            "setup": format_micro(p.setup_micro),
            "batch": format_micro(p.batch_micro),
            "estimate": true,
        }),
        None => Value::Null,
    };
    let findings: Vec<Value> = bom.findings.iter().map(finding_json).collect();
    let doc = json!({
        "format": "phoenix-bom/1",
        "currency": bom.currency,
        "boards": bom.boards,
        "waste_percent": format_micro(i128::from(bom.waste_percent_micro)),
        "complete": bom.complete,
        "lines": lines,
        "parts": {
            "batch": format_micro(bom.parts_batch_micro),
            "per_board": format_micro(bom.parts_per_board_micro),
            "lines_without_price": bom.unpriced_lines,
            "parts_without_price": bom.unpriced_parts,
            "lines_without_footprint": bom.no_footprint_lines,
        },
        "pcb": pcb,
        "total": {
            "batch": format_micro(bom.total_batch_micro),
            "per_board": format_micro(bom.total_per_board_micro),
        },
        "findings": findings,
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// The bill as a page of HTML with no script and no outside file; every text from a file is
/// escaped.
pub fn bom_html(bom: &Bom, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"");
    out.push_str(lang.tag());
    out.push_str("\">\n<head>\n<meta charset=\"utf-8\">\n<title>Bill of materials</title>\n");
    out.push_str("<style>body{font-family:sans-serif}table{border-collapse:collapse}");
    out.push_str("td,th{border:1px solid #888;padding:2px 6px;text-align:left}</style>\n");
    out.push_str("</head>\n<body>\n<h1>Bill of materials</h1>\n");
    out.push_str(&format!(
        "<p>{} boards, {} of waste, in {}</p>\n<table>\n<tr>",
        bom.boards,
        html_escape(&format!(
            "{}%",
            format_micro(i128::from(bom.waste_percent_micro))
        )),
        html_escape(&bom.currency)
    ));
    for name in COLUMNS {
        out.push_str(&format!("<th>{}</th>", html_escape(name)));
    }
    out.push_str("</tr>\n");
    for line in &bom.lines {
        out.push_str("<tr>");
        for cell in cells(line) {
            out.push_str(&format!("<td>{}</td>", html_escape(&cell)));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n<table>\n");
    let mut totals = vec![
        ("parts, batch", format_micro(bom.parts_batch_micro)),
        ("parts, one board", format_micro(bom.parts_per_board_micro)),
    ];
    if let Some(p) = &bom.pcb {
        totals.push(("boards, batch (estimate)", format_micro(p.batch_micro)));
        totals.push(("boards, one (estimate)", format_micro(p.per_board_micro)));
    }
    totals.push(("total, batch", format_micro(bom.total_batch_micro)));
    totals.push(("total, one board", format_micro(bom.total_per_board_micro)));
    for (name, value) in totals {
        out.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>\n",
            html_escape(name),
            html_escape(&value)
        ));
    }
    out.push_str("</table>\n");
    if !bom.complete {
        out.push_str("<p>The totals are not complete.</p>\n");
    }
    if !bom.findings.is_empty() {
        out.push_str("<ul>\n");
        for f in sorted_findings(&bom.findings) {
            out.push_str(&format!("<li>{}</li>\n", html_escape(&f.line(lang))));
        }
        out.push_str("</ul>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

/// The findings, the most serious first and otherwise in the order they were found.
pub fn sorted_findings(findings: &[Finding]) -> Vec<&Finding> {
    let mut sorted: Vec<&Finding> = findings.iter().collect();
    sorted.sort_by_key(|f| std::cmp::Reverse(f.severity));
    sorted
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

/// The bill as text: a line for each part group, the totals and the findings.
pub fn bom_text(bom: &Bom, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "bill of materials: {} lines, {} boards, waste {}%, {}\n",
        bom.lines.len(),
        bom.boards,
        format_micro(i128::from(bom.waste_percent_micro)),
        bom.currency
    ));
    for l in &bom.lines {
        let offer = match &l.purchase {
            Some(p) => format!(
                "{} {} x {} = {}",
                p.supplier,
                p.buy_qty,
                format_micro(i128::from(p.unit_price_micro)),
                format_micro(p.cost_micro)
            ),
            None => "no price".to_owned(),
        };
        out.push_str(&format!(
            "{} x{} {} {} {} {}: {}\n",
            l.references.join(" "),
            l.qty,
            l.value,
            l.footprint,
            l.manufacturer,
            l.mpn,
            offer
        ));
    }
    out.push_str(&format!(
        "parts: {} for the batch, {} for one board\n",
        format_micro(bom.parts_batch_micro),
        format_micro(bom.parts_per_board_micro)
    ));
    if let Some(p) = &bom.pcb {
        out.push_str(&format!(
            "boards (estimate): {} for the batch, {} for one, {} layers, {} holes\n",
            format_micro(p.batch_micro),
            format_micro(p.per_board_micro),
            p.layers,
            p.holes
        ));
    }
    out.push_str(&format!(
        "total: {} for the batch, {} for one board{}\n",
        format_micro(bom.total_batch_micro),
        format_micro(bom.total_per_board_micro),
        if bom.complete { "" } else { " (not complete)" }
    ));
    let (errors, warnings, infos) = counts(&bom.findings);
    out.push_str(&format!(
        "findings: {errors} errors, {warnings} warnings, {infos} information\n"
    ));
    for f in sorted_findings(&bom.findings) {
        out.push_str(&f.line(lang));
        out.push('\n');
    }
    out
}
