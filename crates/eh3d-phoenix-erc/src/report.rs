// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of the check: what is proven and what could not be judged, in two lists that are
//! never mixed. The lists are sorted in a fixed order, so the same schematic and settings always
//! give the same report, as text and as JSON.

use std::fmt::Write as _;

use eh3d_phoenix_rules::codes::word;
use eh3d_phoenix_rules::{Lang, Severity};
use serde_json::{json, Value};

use crate::codes::title;

/// One finding of the check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The place of the rule in the fixed order of the checks (19 for what could not be judged).
    pub rank: u8,
    /// The stable name of the rule (`pin_unconnected`).
    pub rule: &'static str,
    /// The stable code (`PHOENIX-ERC-001`).
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What it is about, by names: `R1.2`, `net GND`, `symbol NAND`.
    pub location: String,
    /// More words, English: the pins involved, a count, why it could not be judged.
    pub detail: String,
}

impl Finding {
    fn sort_key(&self) -> (u8, &str, &str, &str) {
        (self.rank, &self.location, &self.detail, self.code)
    }

    fn to_value(&self) -> Value {
        json!({
            "rank": self.rank,
            "rule": self.rule,
            "code": self.code,
            "severity": self.severity.as_str(),
            "location": self.location,
            "detail": self.detail,
        })
    }

    fn to_line(&self, lang: Lang) -> String {
        let mut line = format!(
            "  [{}] {} {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang).unwrap_or(""),
            self.location
        );
        if !self.detail.is_empty() {
            let _ = write!(line, " | {}", self.detail);
        }
        line
    }
}

/// The result of the check: two lists, never mixed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// What is proven wrong or worth a look.
    pub confirmed: Vec<Finding>,
    /// What could not be judged.
    pub inconclusive: Vec<Finding>,
}

impl Report {
    /// True when no error is proven. Warnings, information and the inconclusive list do not count.
    pub fn passes(&self) -> bool {
        self.confirmed.iter().all(|f| f.severity != Severity::Error)
    }

    /// The number of proven findings of one severity.
    pub fn count(&self, severity: Severity) -> usize {
        self.confirmed
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    /// Puts both lists in the fixed order.
    pub(crate) fn finish(&mut self) {
        self.confirmed
            .sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        self.inconclusive
            .sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    /// The report as text, with the titles in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        for (key, list) in [
            ("confirmed", &self.confirmed),
            ("inconclusive", &self.inconclusive),
        ] {
            let _ = writeln!(out, "{}: {}", word(key, lang), list.len());
            for f in list {
                let _ = writeln!(out, "{}", f.to_line(lang));
            }
        }
        if self.confirmed.is_empty() && self.inconclusive.is_empty() {
            let _ = writeln!(out, "{}", word("nothing", lang));
        }
        out
    }

    /// The report as JSON: the same two lists, with the English detail and the code of each
    /// finding (the titles are translated by the user interface from the code).
    pub fn to_json(&self) -> String {
        let value = json!({
            "schema": "phoenix-erc/1",
            "passes": self.passes(),
            "confirmed": self.confirmed.iter().map(Finding::to_value).collect::<Vec<_>>(),
            "inconclusive": self.inconclusive.iter().map(Finding::to_value).collect::<Vec<_>>(),
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{PIN_UNCONNECTED, SINGLE_PIN_NET};

    fn finding(code: &'static str, rank: u8, severity: Severity, location: &str) -> Finding {
        Finding {
            rank,
            rule: "x",
            code,
            severity,
            location: location.to_owned(),
            detail: String::new(),
        }
    }

    #[test]
    fn findings_are_sorted_by_rule_then_place_whatever_the_order_they_were_added() {
        let a = finding(PIN_UNCONNECTED, 1, Severity::Error, "R2.1");
        let b = finding(PIN_UNCONNECTED, 1, Severity::Error, "R1.2");
        let c = finding(SINGLE_PIN_NET, 2, Severity::Warning, "net A");
        let mut one = Report {
            confirmed: vec![c.clone(), a.clone(), b.clone()],
            inconclusive: Vec::new(),
        };
        let mut two = Report {
            confirmed: vec![b, c, a],
            inconclusive: Vec::new(),
        };
        one.finish();
        two.finish();
        assert_eq!(one, two);
        assert_eq!(one.confirmed[0].location, "R1.2");
        assert_eq!(one.to_json(), two.to_json());
        assert_eq!(one.count(Severity::Error), 2);
        assert!(!one.passes());
    }

    #[test]
    fn the_text_keeps_the_lists_apart_and_translates_the_title() {
        let mut r = Report::default();
        r.confirmed
            .push(finding(PIN_UNCONNECTED, 1, Severity::Error, "R1.2"));
        let mut inconclusive = finding(crate::codes::NOT_JUDGED, 15, Severity::Info, "schematic");
        inconclusive.detail = "3 pins have no type".to_owned();
        r.inconclusive.push(inconclusive);
        let en = r.to_text(Lang::En);
        assert!(en.starts_with("Confirmed: 1\n"));
        assert!(en.contains("[error] PHOENIX-ERC-001 Pin is not connected: R1.2"));
        assert!(en.contains("Inconclusive: 1\n"));
        assert!(en.contains("| 3 pins have no type"));
        assert!(r.to_text(Lang::Pt).contains("Pino sem ligação"));
        assert!(Report::default()
            .to_text(Lang::En)
            .contains("nothing found"));
    }

    #[test]
    fn json_has_both_lists_and_valid_syntax() {
        let mut r = Report::default();
        r.confirmed
            .push(finding(SINGLE_PIN_NET, 2, Severity::Warning, "net A"));
        let value: Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(value["schema"], "phoenix-erc/1");
        assert_eq!(value["passes"], true);
        assert_eq!(value["confirmed"][0]["severity"], "warning");
        assert_eq!(value["inconclusive"].as_array().map(Vec::len), Some(0));
    }
}
