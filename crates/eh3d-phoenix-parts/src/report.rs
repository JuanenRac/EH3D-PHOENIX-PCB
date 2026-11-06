// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of the commands: the result of a search, one part in full, and the findings of the
//! checker, as text and as JSON, the same every time.

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{Finding, Severity};
use crate::index::Found;
use crate::model::Part;
use crate::write::{part_json, price_text};

/// One line for a part: id, value, footprint, maker and number, and the state.
pub fn part_line(p: &Part) -> String {
    let maker = [p.manufacturer.as_str(), p.mpn.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<&str>>()
        .join(" ");
    format!(
        "{} | {} | {} | {} | {}",
        p.id,
        p.value,
        p.footprint,
        maker,
        p.status.as_str()
    )
}

/// The result of a search as text: a line for each part and the count.
pub fn search_text(found: &Found) -> String {
    let mut out = String::new();
    for p in &found.parts {
        out.push_str(&part_line(p));
        out.push('\n');
    }
    if found.total > found.parts.len() {
        out.push_str(&format!(
            "{} of {} parts shown\n",
            found.parts.len(),
            found.total
        ));
    } else {
        out.push_str(&format!("{} parts\n", found.total));
    }
    out
}

/// The result of a search as JSON, with a line feed at the end.
pub fn search_json(found: &Found) -> String {
    let parts: Vec<Value> = found.parts.iter().map(|p| part_json(p)).collect();
    let document = json!({
        "format": "phoenix-parts-search/1",
        "total": found.total,
        "parts": parts,
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

/// One part in full as text.
pub fn show_text(p: &Part) -> String {
    let mut out = String::new();
    let mut line = |key: &str, value: &str| {
        if !value.is_empty() {
            out.push_str(&format!("{key}: {value}\n"));
        }
    };
    line("id", &p.id);
    line("value", &p.value);
    line("tolerance", &p.tolerance);
    line("power", &p.power);
    line("voltage", &p.voltage);
    line("footprint", &p.footprint);
    line("symbol", &p.symbol);
    line("manufacturer", &p.manufacturer);
    line("mpn", &p.mpn);
    line("status", p.status.as_str());
    line("sim_model", &p.sim_model);
    line("notes", &p.notes);
    line("alternatives", &p.alternatives.join(", "));
    for (key, value) in &p.params {
        line(&format!("param {key}"), value);
    }
    for offer in &p.offers {
        let tiers: Vec<String> = offer
            .tiers
            .iter()
            .map(|t| {
                let currency = if t.currency.is_empty() {
                    String::new()
                } else {
                    format!(" {}", t.currency)
                };
                format!(
                    "{} -> {}{}",
                    t.min_qty,
                    price_text(t.unit_price_micro),
                    currency
                )
            })
            .collect();
        let code = if offer.supplier_code.is_empty() {
            String::new()
        } else {
            format!(" ({})", offer.supplier_code)
        };
        line(
            "offer",
            &format!("{}{}: {}", offer.supplier, code, tiers.join("; ")),
        );
    }
    line("origin", &p.origin);
    out
}

/// The number of findings by severity: errors, warnings and information.
pub fn counts(findings: &[Finding]) -> (usize, usize, usize) {
    let of = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    (
        of(Severity::Error),
        of(Severity::Warning),
        of(Severity::Info),
    )
}

/// The findings as text in a language, then the counts.
pub fn check_text(findings: &[Finding], parts: usize, lang: Lang) -> String {
    let mut out = String::new();
    for f in findings {
        out.push_str(&f.line(lang));
        out.push('\n');
    }
    let (errors, warnings, infos) = counts(findings);
    out.push_str(&format!(
        "{parts} parts checked: {errors} errors, {warnings} warnings, {infos} information\n"
    ));
    out
}

/// The findings as JSON, with a line feed at the end.
pub fn check_json(findings: &[Finding], parts: usize) -> String {
    let (errors, warnings, infos) = counts(findings);
    let list: Vec<Value> = findings.iter().map(Finding::to_json).collect();
    let document = json!({
        "format": "phoenix-parts-check/1",
        "parts": parts,
        "errors": errors,
        "warnings": warnings,
        "information": infos,
        "findings": list,
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{FOOTPRINT_UNKNOWN, ID_DUPLICATE};
    use crate::model::{Offer, Status, Tier};

    fn sample() -> Part {
        let mut p = Part {
            id: "r-10k".into(),
            value: "10k".into(),
            footprint: "R0603".into(),
            manufacturer: "Acme".into(),
            mpn: "AC-10K".into(),
            status: Status::NotRecommended,
            origin: "base.json: part 1".into(),
            ..Part::default()
        };
        p.params.insert("tcr".into(), "100ppm".into());
        p.alternatives = vec!["a".into(), "b".into()];
        p.offers = vec![Offer {
            supplier: "Shop A".into(),
            supplier_code: "A-1".into(),
            tiers: vec![
                Tier {
                    min_qty: 1,
                    unit_price_micro: 10_000,
                    currency: "EUR".into(),
                },
                Tier {
                    min_qty: 100,
                    unit_price_micro: 4_000,
                    currency: String::new(),
                },
            ],
        }];
        p
    }

    #[test]
    fn a_part_line_and_a_search_say_what_they_found() {
        let p = sample();
        assert_eq!(
            part_line(&p),
            "r-10k | 10k | R0603 | Acme AC-10K | not-recommended"
        );
        let found = Found {
            parts: vec![&p],
            total: 3,
        };
        let text = search_text(&found);
        assert!(text.ends_with("1 of 3 parts shown\n"));
        let all = Found {
            parts: vec![&p],
            total: 1,
        };
        assert!(search_text(&all).ends_with("1 parts\n"));
        let json: Value = serde_json::from_str(&search_json(&found)).unwrap();
        assert_eq!(json["total"], 3);
        assert_eq!(json["parts"][0]["id"], "r-10k");
    }

    #[test]
    fn one_part_is_shown_in_full() {
        let text = show_text(&sample());
        for line in [
            "id: r-10k\n",
            "status: not-recommended\n",
            "alternatives: a, b\n",
            "param tcr: 100ppm\n",
            "offer: Shop A (A-1): 1 -> 0.01 EUR; 100 -> 0.004\n",
            "origin: base.json: part 1\n",
        ] {
            assert!(text.contains(line), "{line} in {text}");
        }
        assert!(!text.contains("tolerance"));
    }

    #[test]
    fn the_findings_are_counted_and_written_in_a_language() {
        let findings = vec![
            Finding::new(
                ID_DUPLICATE,
                Severity::Error,
                "a",
                "x.json: part 2",
                "also at x.json: part 1",
            ),
            Finding::new(FOOTPRINT_UNKNOWN, Severity::Warning, "b", "", "R9"),
        ];
        assert_eq!(counts(&findings), (1, 1, 0));
        let text = check_text(&findings, 5, Lang::Es);
        assert!(
            text.contains("error: PHOENIX-PARTS-007: Identificador de pieza usado dos veces [a]")
        );
        assert!(text.ends_with("5 parts checked: 1 errors, 1 warnings, 0 information\n"));
        let json: Value = serde_json::from_str(&check_json(&findings, 5)).unwrap();
        assert_eq!(json["errors"], 1);
        assert_eq!(json["findings"][1]["code"], "PHOENIX-PARTS-008");
        assert_eq!(check_json(&findings, 5), check_json(&findings, 5));
    }
}
