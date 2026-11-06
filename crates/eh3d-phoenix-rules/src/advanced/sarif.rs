// SPDX-License-Identifier: GPL-3.0-or-later
//! The report as JSON in a layout like SARIF, `phoenix-drc/1`: the rules that appear, and one
//! result for each finding, proven or inconclusive. It is not the SARIF of the OASIS standard: it
//! keeps the names `ruleId`, `level` and `message` and adds the measured value and the limit in
//! nanometres. The same report always gives the same bytes.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::codes::{title, Lang};
use crate::report::{Finding, Report, Severity};

fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "note",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn result(finding: &Finding, kind: &str, lang: Lang) -> Value {
    let message = format!(
        "{}: {}",
        title(finding.code, lang).unwrap_or(finding.rule),
        finding.location
    );
    json!({
        "ruleId": finding.code,
        "level": level(finding.severity),
        "kind": kind,
        "message": { "text": message },
        "location": { "name": finding.location, "at_nm": finding.at_nm },
        "properties": {
            "measured_nm": finding.measured_nm,
            "limit_nm": finding.limit_nm,
            "detail": finding.detail,
        },
    })
}

/// The report as text in the layout `phoenix-drc/1`, with the titles in `lang`. The proven
/// findings have the kind `fail` and the ones that could not be judged the kind `review`.
pub fn to_sarif(report: &Report, lang: Lang) -> String {
    let mut rules: BTreeMap<&'static str, &'static str> = BTreeMap::new();
    for f in report.confirmed.iter().chain(report.inconclusive.iter()) {
        rules.entry(f.code).or_insert(f.rule);
    }
    let rule_list: Vec<Value> = rules
        .iter()
        .map(|(code, name)| {
            json!({
                "id": code,
                "name": name,
                "title": title(code, lang).unwrap_or(""),
            })
        })
        .collect();
    let mut results: Vec<Value> = Vec::new();
    for f in &report.confirmed {
        results.push(result(f, "fail", lang));
    }
    for f in &report.inconclusive {
        results.push(result(f, "review", lang));
    }
    let value = json!({
        "schema": "phoenix-drc/1",
        "language": lang.tag(),
        "passes": report.passes(),
        "rules": rule_list,
        "results": results,
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}
