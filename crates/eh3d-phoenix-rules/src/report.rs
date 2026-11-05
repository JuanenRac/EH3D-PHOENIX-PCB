// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of a check: what is proven and what could not be judged, kept in two lists.
//!
//! A finding in `confirmed` is proven with exact integer arithmetic. A finding in `inconclusive`
//! is something this engine cannot judge (an arc, a pad shape too coarse to tell, numbers too
//! large for the exact test, a rule that asks for what the board does not say); it is never a
//! failure and never hidden. The lists are sorted in a fixed order, so the same board and rules
//! always give the same report, as text and as JSON.

use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::codes::{title, word, Lang};

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth knowing, changes nothing.
    Info,
    /// Probably wrong, or approximated.
    Warning,
    /// A rule that is broken.
    Error,
}

impl Severity {
    /// The word used in the text and JSON reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One finding of a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The place of the rule in the fixed order of the checks.
    pub rank: u8,
    /// The stable name of the rule (`copper_clearance`).
    pub rule: &'static str,
    /// The stable code (`PHOENIX-CHECK-005`).
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What it is about: the objects, by their identities or reference designators.
    pub location: String,
    /// Where it is on the board, in nanometres, when it has a place.
    pub at_nm: Option<[i64; 2]>,
    /// The measured value in nanometres, when the check measures one. For a distance it is
    /// rounded to the nearest nanometre.
    pub measured_nm: Option<i64>,
    /// The limit it was compared with, in nanometres, when there is one.
    pub limit_nm: Option<i64>,
    /// More words, English: why it is inconclusive, or a count.
    pub detail: String,
}

impl Finding {
    pub(crate) fn new(
        rank: u8,
        rule: &'static str,
        code: &'static str,
        severity: Severity,
        location: String,
    ) -> Finding {
        Finding {
            rank,
            rule,
            code,
            severity,
            location,
            at_nm: None,
            measured_nm: None,
            limit_nm: None,
            detail: String::new(),
        }
    }

    pub(crate) fn at(mut self, x_nm: i64, y_nm: i64) -> Finding {
        self.at_nm = Some([x_nm, y_nm]);
        self
    }

    pub(crate) fn values(mut self, measured_nm: i64, limit_nm: i64) -> Finding {
        self.measured_nm = Some(measured_nm);
        self.limit_nm = Some(limit_nm);
        self
    }

    pub(crate) fn limit(mut self, limit_nm: i64) -> Finding {
        self.limit_nm = Some(limit_nm);
        self
    }

    pub(crate) fn why(mut self, detail: &str) -> Finding {
        self.detail = detail.to_owned();
        self
    }

    fn sort_key(&self) -> (u8, String, Option<[i64; 2]>, &'static str) {
        (self.rank, self.location.clone(), self.at_nm, self.code)
    }

    fn to_value(&self) -> Value {
        json!({
            "rank": self.rank,
            "rule": self.rule,
            "code": self.code,
            "severity": self.severity.as_str(),
            "location": self.location,
            "at_nm": self.at_nm,
            "measured_nm": self.measured_nm,
            "limit_nm": self.limit_nm,
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
        if let Some([x, y]) = self.at_nm {
            let _ = write!(line, " | at ({x}, {y}) nm");
        }
        if let Some(m) = self.measured_nm {
            let _ = write!(line, " | {} {m} nm", word("measured", lang));
        }
        if let Some(l) = self.limit_nm {
            let _ = write!(line, " | {} {l} nm", word("limit", lang));
        }
        if !self.detail.is_empty() {
            let _ = write!(line, " | {}", self.detail);
        }
        line
    }
}

/// The result of a check: two lists, never mixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// What the report is about (`check` or `connectivity`).
    pub kind: &'static str,
    /// Rules that are broken, proven.
    pub confirmed: Vec<Finding>,
    /// What could not be judged.
    pub inconclusive: Vec<Finding>,
}

impl Report {
    pub(crate) fn new(kind: &'static str) -> Report {
        Report {
            kind,
            confirmed: Vec::new(),
            inconclusive: Vec::new(),
        }
    }

    /// True when nothing is proven wrong. It says nothing about the inconclusive list.
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

    /// Puts both lists in the fixed order. Called once, when a check is done.
    pub(crate) fn finish(&mut self) {
        self.confirmed.sort_by_key(Finding::sort_key);
        self.inconclusive.sort_by_key(Finding::sort_key);
    }

    /// Adds the findings of another report of the same kind and puts everything in order.
    pub fn merge(&mut self, other: Report) {
        self.confirmed.extend(other.confirmed);
        self.inconclusive.extend(other.inconclusive);
        self.finish();
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
            "schema": format!("phoenix-{}/1", self.kind),
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
    use crate::codes::CLEARANCE;

    fn sample(location: &str, x: i64) -> Finding {
        Finding::new(
            5,
            "copper_clearance",
            CLEARANCE,
            Severity::Error,
            location.to_owned(),
        )
        .at(x, 0)
        .values(100, 200)
    }

    #[test]
    fn findings_are_sorted_by_rule_then_place_whatever_the_order_they_were_added() {
        let mut a = Report::new("check");
        a.confirmed.push(sample("b", 1));
        a.confirmed.push(sample("a", 2));
        a.confirmed.push(sample("a", 1));
        let mut b = Report::new("check");
        b.confirmed.push(sample("a", 1));
        b.confirmed.push(sample("b", 1));
        b.confirmed.push(sample("a", 2));
        a.finish();
        b.finish();
        assert_eq!(a, b);
        assert_eq!(a.confirmed[0].location, "a");
        assert_eq!(a.confirmed[0].at_nm, Some([1, 0]));
        assert_eq!(a.to_json(), b.to_json());
    }

    #[test]
    fn the_text_keeps_the_two_lists_apart_and_translates_the_title() {
        let mut r = Report::new("check");
        r.confirmed.push(sample("track x", 7));
        r.inconclusive.push(
            Finding::new(
                13,
                "not_judged",
                crate::codes::NOT_JUDGED,
                Severity::Info,
                "track y".into(),
            )
            .why("arc"),
        );
        let en = r.to_text(Lang::En);
        assert!(en.starts_with("Confirmed: 1\n"));
        assert!(en.contains(
            "Copper clearance violated: track x | at (7, 0) nm | measured 100 nm | limit 200 nm"
        ));
        assert!(en.contains("Inconclusive: 1\n"));
        assert!(en.contains("| arc"));
        let pt = r.to_text(Lang::Pt);
        assert!(pt.contains("Distância do cobre não respeitada"));
        assert!(!r.passes());
        assert_eq!(r.count(Severity::Error), 1);
    }

    #[test]
    fn json_has_both_lists_and_valid_syntax() {
        let mut r = Report::new("connectivity");
        r.confirmed.push(sample("x", 0));
        let value: Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(value["schema"], "phoenix-connectivity/1");
        assert_eq!(value["passes"], false);
        assert_eq!(value["confirmed"][0]["measured_nm"], 100);
        assert_eq!(value["confirmed"][0]["limit_nm"], 200);
        assert_eq!(value["inconclusive"].as_array().map(Vec::len), Some(0));
        assert!(Report::new("check")
            .to_text(Lang::En)
            .contains("nothing found"));
    }

    #[test]
    fn warnings_and_information_do_not_make_a_report_fail() {
        let mut r = Report::new("check");
        r.confirmed.push(Finding::new(
            1,
            "x",
            CLEARANCE,
            Severity::Warning,
            "w".into(),
        ));
        assert!(r.passes());
    }
}
