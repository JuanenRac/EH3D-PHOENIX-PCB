// SPDX-License-Identifier: GPL-3.0-or-later
//! What a reader, a writer or a chain of them gave for one piece of the corpus, and the judgement
//! of that against the expectation.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::codes::{DIFFERENCE, REFUSAL_EXPECTED, ROUND_TRIP_LOSS, UNEXPECTED_FAILURE};
use super::entry::Entry;

/// The result of one piece.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// True when the reader refused the file (or the chain stopped).
    pub refused: bool,
    /// Every diagnostic code seen.
    pub codes: BTreeSet<String>,
    /// The measures, by name.
    pub measures: BTreeMap<String, Value>,
    /// The numbers of the solver, by label.
    pub probes: BTreeMap<String, f64>,
    /// Lines for the person who reads the report (the differences of a round trip, the reason of
    /// a refusal that has no code).
    pub notes: Vec<String>,
}

impl Outcome {
    /// Sets a measure.
    pub fn put(&mut self, key: &str, value: impl Into<Value>) {
        self.measures.insert(key.to_owned(), value.into());
    }

    /// Adds a code to the ones seen.
    pub fn code(&mut self, code: &str) {
        self.codes.insert(code.to_owned());
    }

    /// Adds the measures of a map.
    pub fn merge(&mut self, measures: BTreeMap<String, Value>) {
        self.measures.extend(measures);
    }

    /// A refusal with a code (or without).
    pub fn refusal(code: Option<&str>) -> Outcome {
        let mut o = Outcome {
            refused: true,
            ..Outcome::default()
        };
        if let Some(c) = code {
            o.code(c);
        }
        o
    }
}

/// The verdict on a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Everything is as the expectation says and the file was read.
    Ok,
    /// The result differs from the expectation.
    Difference,
    /// The file was refused and the expectation says it must be.
    ExpectedError,
    /// A reader failed where success was expected, or the piece could not run.
    Failed,
}

/// The verdict with its code (none for ok) and the lines that explain it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    /// The verdict.
    pub verdict: Verdict,
    /// The code of the verdict, `None` for ok.
    pub code: Option<&'static str>,
    /// What differs, one line each.
    pub lines: Vec<String>,
}

fn join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

/// Compares an outcome with the expectation of its entry. A round trip that differs gets the
/// code of a loss, any other difference the code of a difference.
pub fn judge(entry: &Entry, outcome: &Outcome) -> Judgement {
    let want = &entry.expect;
    let want_refused = want.outcome == "refused";
    let mut lines: Vec<String> = Vec::new();
    if want_refused != outcome.refused {
        lines.push(format!(
            "outcome: expected {}, got {}",
            want.outcome,
            if outcome.refused { "refused" } else { "read" }
        ));
    }
    if let Some(codes) = &want.codes {
        let set: BTreeSet<String> = codes.iter().cloned().collect();
        if set != outcome.codes {
            lines.push(format!(
                "codes: expected [{}], got [{}]",
                join(&set),
                join(&outcome.codes)
            ));
        }
    }
    for code in &want.codes_include {
        if !outcome.codes.contains(code) {
            lines.push(format!(
                "codes: {code} is expected and was not seen (seen: [{}])",
                join(&outcome.codes)
            ));
        }
    }
    for (key, expected) in &want.measures {
        let got = outcome
            .measures
            .get(key)
            .cloned()
            .unwrap_or_else(|| Value::from(0));
        if got != *expected {
            lines.push(format!("{key}: expected {expected}, got {got}"));
        }
    }
    for (label, probe) in &want.probes {
        match outcome.probes.get(label) {
            Some(v) if (v - probe.value).abs() <= probe.abs => {}
            Some(v) => lines.push(format!(
                "{label}: expected {} within {}, got {v}",
                probe.value, probe.abs
            )),
            None => lines.push(format!("{label}: the solver gave no value")),
        }
    }
    let mut shown = lines.clone();
    shown.extend(outcome.notes.iter().cloned());
    if !want_refused && outcome.refused {
        return Judgement {
            verdict: Verdict::Failed,
            code: Some(UNEXPECTED_FAILURE),
            lines: shown,
        };
    }
    if !lines.is_empty() {
        let code = if entry.kind == "roundtrip" || entry.kind == "gerber-roundtrip" {
            ROUND_TRIP_LOSS
        } else {
            DIFFERENCE
        };
        return Judgement {
            verdict: Verdict::Difference,
            code: Some(code),
            lines: shown,
        };
    }
    if want_refused {
        return Judgement {
            verdict: Verdict::ExpectedError,
            code: Some(REFUSAL_EXPECTED),
            lines: outcome.notes.clone(),
        };
    }
    Judgement {
        verdict: Verdict::Ok,
        code: None,
        lines: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(expect: &str, kind: &str) -> Entry {
        let via = if kind == "roundtrip" {
            r#""via":["json"],"#
        } else {
            ""
        };
        Entry::parse(&format!(
            r#"{{"format":"phoenix-corpus/1","id":"CORPUS-T-001","title":"t","kind":"{kind}",
            "input":"a.json",{via}"how":["by hand"],"expect":{expect}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn a_file_that_reads_as_expected_is_ok() {
        let e = entry(
            r#"{"outcome":"read","codes":["C-1"],"measures":{"parts":2}}"#,
            "eagle",
        );
        let mut o = Outcome::default();
        o.code("C-1");
        o.put("parts", 2usize);
        let j = judge(&e, &o);
        assert_eq!(j.verdict, Verdict::Ok);
        assert_eq!(j.code, None);
        assert!(j.lines.is_empty());
    }

    #[test]
    fn a_measure_that_the_result_does_not_have_counts_as_zero() {
        let e = entry(r#"{"outcome":"read","measures":{"vias":0}}"#, "eagle");
        assert_eq!(judge(&e, &Outcome::default()).verdict, Verdict::Ok);
        let e = entry(r#"{"outcome":"read","measures":{"vias":1}}"#, "eagle");
        let j = judge(&e, &Outcome::default());
        assert_eq!(j.verdict, Verdict::Difference);
        assert_eq!(j.code, Some(DIFFERENCE));
        assert_eq!(j.lines, vec!["vias: expected 1, got 0".to_owned()]);
    }

    #[test]
    fn codes_are_compared_as_a_set_or_as_a_subset() {
        let e = entry(r#"{"outcome":"read","codes":["A","B"]}"#, "eagle");
        let mut o = Outcome::default();
        o.code("A");
        assert_eq!(judge(&e, &o).verdict, Verdict::Difference);
        o.code("B");
        assert_eq!(judge(&e, &o).verdict, Verdict::Ok);
        o.code("C");
        assert_eq!(judge(&e, &o).verdict, Verdict::Difference);
        let e = entry(r#"{"outcome":"read","codes_include":["A"]}"#, "eagle");
        assert_eq!(judge(&e, &o).verdict, Verdict::Ok);
        assert_eq!(judge(&e, &Outcome::default()).verdict, Verdict::Difference);
    }

    #[test]
    fn a_refusal_that_was_expected_is_an_expected_error() {
        let e = entry(
            r#"{"outcome":"refused","codes":["PHOENIX-X-001"]}"#,
            "eagle",
        );
        let j = judge(&e, &Outcome::refusal(Some("PHOENIX-X-001")));
        assert_eq!(j.verdict, Verdict::ExpectedError);
        assert_eq!(j.code, Some(REFUSAL_EXPECTED));
        let wrong = judge(&e, &Outcome::refusal(Some("PHOENIX-X-002")));
        assert_eq!(wrong.verdict, Verdict::Difference);
        let read = judge(&e, &Outcome::default());
        assert_eq!(read.verdict, Verdict::Difference);
    }

    #[test]
    fn a_refusal_where_success_was_expected_is_a_failure() {
        let e = entry(r#"{"outcome":"read"}"#, "eagle");
        let j = judge(&e, &Outcome::refusal(None));
        assert_eq!(j.verdict, Verdict::Failed);
        assert_eq!(j.code, Some(UNEXPECTED_FAILURE));
    }

    #[test]
    fn the_solver_numbers_are_judged_with_their_distance() {
        let e = entry(
            r#"{"outcome":"read","probes":{"v(out)":{"value":7.5,"abs":0.001}}}"#,
            "spice",
        );
        let mut o = Outcome::default();
        o.probes.insert("v(out)".to_owned(), 7.5004);
        assert_eq!(judge(&e, &o).verdict, Verdict::Ok);
        o.probes.insert("v(out)".to_owned(), 7.6);
        assert_eq!(judge(&e, &o).verdict, Verdict::Difference);
        assert_eq!(judge(&e, &Outcome::default()).verdict, Verdict::Difference);
    }

    #[test]
    fn a_round_trip_that_loses_something_gets_the_code_of_a_loss() {
        let e = entry(
            r#"{"outcome":"read","measures":{"differences":0}}"#,
            "roundtrip",
        );
        let mut o = Outcome::default();
        o.put("differences", 1usize);
        o.notes.push("components: 2 before, 1 after".to_owned());
        let j = judge(&e, &o);
        assert_eq!(j.verdict, Verdict::Difference);
        assert_eq!(j.code, Some(ROUND_TRIP_LOSS));
        assert!(j.lines.iter().any(|l| l.starts_with("components:")));
    }
}
