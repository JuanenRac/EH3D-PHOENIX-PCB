// SPDX-License-Identifier: GPL-3.0-or-later
//! Running the cases and the report: the same cases give the same report, byte for byte, in text
//! and in JSON.

use std::fmt::Write as _;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

use crate::case::{Case, CaseFile, Status};
use crate::codes::{column, title, CALC_FAILED, CASE_PENDING, OUT_OF_TOLERANCE, WORDS};
use crate::compute::compute;

/// The format name of a JSON report.
pub const REPORT_FORMAT: &str = "phoenix-oracle-report/1";

/// What happened to a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The result is inside the tolerance of the reference.
    Ok,
    /// The result is outside the tolerance of the reference.
    Outside,
    /// The case cannot be run yet.
    Pending,
    /// The calculation under test, or the case itself, failed.
    Failed,
}

impl Verdict {
    fn index(self) -> usize {
        match self {
            Verdict::Ok => 0,
            Verdict::Outside => 1,
            Verdict::Pending => 2,
            Verdict::Failed => 3,
        }
    }
}

/// The result of one case.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaseResult {
    /// The identity of the case.
    pub id: String,
    /// The suite the case is in.
    pub suite: String,
    /// The area the case is about.
    pub area: String,
    /// What the case checks.
    pub title: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The code of a case that is not ok.
    pub code: Option<&'static str>,
    /// What the function under test gave.
    pub actual: Option<f64>,
    /// The reference value.
    pub expected: Option<f64>,
    /// The largest difference that was accepted.
    pub allowed: Option<f64>,
    /// The unit.
    pub unit: String,
    /// The source of the reference value.
    pub source: String,
    /// Why the case is pending or not ok, in English.
    pub detail: String,
}

/// Run one case of a suite.
pub fn run_case(suite: &str, case: &Case) -> CaseResult {
    let mut result = CaseResult {
        id: case.id.clone(),
        suite: suite.to_string(),
        area: case.area.clone(),
        title: case.title.clone(),
        verdict: Verdict::Ok,
        code: None,
        actual: None,
        expected: case.expected,
        allowed: None,
        unit: case.unit.clone(),
        source: case.source.clone(),
        detail: String::new(),
    };
    if case.status == Status::Pending {
        result.verdict = Verdict::Pending;
        result.code = Some(CASE_PENDING);
        result.detail = case.reason.clone();
        return result;
    }
    let (Some(expected), Some(tolerance)) = (case.expected, case.tolerance.as_ref()) else {
        result.verdict = Verdict::Failed;
        result.code = Some(CALC_FAILED);
        result.detail = "the case has no reference value or tolerance".to_string();
        return result;
    };
    let allowed = tolerance.allowed(expected);
    result.allowed = Some(allowed);
    match compute(case) {
        Err(error) => {
            result.verdict = Verdict::Failed;
            result.code = Some(error.code);
            result.detail = error.detail;
        }
        Ok(actual) => {
            result.actual = Some(actual);
            if (actual - expected).abs() > allowed {
                result.verdict = Verdict::Outside;
                result.code = Some(OUT_OF_TOLERANCE);
                result.detail = format!("the difference is {}", (actual - expected).abs());
            }
        }
    }
    result
}

/// The counts of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Cases inside the tolerance.
    pub ok: usize,
    /// Cases outside the tolerance.
    pub outside: usize,
    /// Cases still pending.
    pub pending: usize,
    /// Cases that failed to run.
    pub failed: usize,
}

/// The report of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// One result per case, in the order of the files and of the cases in them.
    pub results: Vec<CaseResult>,
}

#[derive(Serialize)]
struct JsonReport<'a> {
    format: &'static str,
    counts: Counts,
    cases: &'a [CaseResult],
}

impl Report {
    /// The counts per verdict.
    pub fn counts(&self) -> Counts {
        let count = |v: Verdict| self.results.iter().filter(|r| r.verdict == v).count();
        Counts {
            ok: count(Verdict::Ok),
            outside: count(Verdict::Outside),
            pending: count(Verdict::Pending),
            failed: count(Verdict::Failed),
        }
    }

    /// True when a case is outside its tolerance or failed. Pending cases do not count.
    pub fn has_problems(&self) -> bool {
        let c = self.counts();
        c.outside + c.failed > 0
    }

    /// The report as text, with the verdict words and the titles of the codes in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let col = column(lang);
        let mut out = format!("oracle report ({} cases)\n", self.results.len());
        for r in &self.results {
            let word = WORDS[r.verdict.index()][col];
            let _ = write!(out, "{word}: {} {}", r.id, r.title);
            if let (Some(actual), Some(expected), Some(allowed)) = (r.actual, r.expected, r.allowed)
            {
                let _ = write!(
                    out,
                    " | {actual} against {expected} {} (allowed {allowed})",
                    r.unit
                );
            }
            if let Some(code) = r.code {
                let _ = write!(out, " | {code}: {}", title(code, lang));
            }
            if !r.detail.is_empty() {
                let _ = write!(out, " | {}", r.detail);
            }
            out.push('\n');
        }
        let c = self.counts();
        let _ = writeln!(
            out,
            "{}: {}, {}: {}, {}: {}, {}: {}",
            WORDS[0][col],
            c.ok,
            WORDS[1][col],
            c.outside,
            WORDS[2][col],
            c.pending,
            WORDS[3][col],
            c.failed
        );
        out
    }

    /// The report as JSON, the same every time.
    pub fn to_json(&self) -> String {
        let report = JsonReport {
            format: REPORT_FORMAT,
            counts: self.counts(),
            cases: &self.results,
        };
        let mut text = serde_json::to_string_pretty(&report).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// Run every case of the files, in order. `suite` limits the run to the files of one suite.
pub fn run_files(files: &[CaseFile], suite: Option<&str>) -> Report {
    let mut results = Vec::new();
    for file in files {
        if suite.is_some_and(|wanted| wanted != file.suite) {
            continue;
        }
        for case in &file.cases {
            results.push(run_case(&file.suite, case));
        }
    }
    Report { results }
}

/// The cases as a list, one per line: the id, the status, the area and the title.
pub fn list_cases(files: &[CaseFile], suite: Option<&str>) -> String {
    let mut out = String::new();
    for file in files {
        if suite.is_some_and(|wanted| wanted != file.suite) {
            continue;
        }
        for case in &file.cases {
            let _ = writeln!(
                out,
                "{} | {} | {} | {}",
                case.id,
                case.status.as_str(),
                case.area,
                case.title
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{"format":"phoenix-oracle/1","suite":"unit","title":"t","cases":[
        {"id":"ORACLE-U-001","title":"c0","area":"stackup","check":"const_c0",
         "expected":299792458.0,"unit":"m/s","tolerance":{"abs":0.0},
         "tolerance_note":"exact","source":"the definition of the metre","status":"cited"},
        {"id":"ORACLE-U-002","title":"c0 wrong","area":"stackup","check":"const_c0",
         "expected":300000000.0,"unit":"m/s","tolerance":{"rel":0.0001},
         "tolerance_note":"test","source":"none","status":"cited"},
        {"id":"ORACLE-U-003","title":"no such","area":"stackup","check":"nothing",
         "expected":1.0,"unit":"","tolerance":{"abs":1.0},
         "tolerance_note":"test","source":"none","status":"cited"},
        {"id":"ORACLE-U-004","title":"later","area":"rules","source":"a standard",
         "status":"pending","reason":"no function yet"}]}"#;

    fn report() -> Report {
        let file = CaseFile::from_json(FILE).unwrap();
        run_files(&[file], None)
    }

    #[test]
    fn the_four_verdicts_are_told_apart() {
        let r = report();
        let verdicts: Vec<Verdict> = r.results.iter().map(|x| x.verdict).collect();
        assert_eq!(
            verdicts,
            [
                Verdict::Ok,
                Verdict::Outside,
                Verdict::Failed,
                Verdict::Pending
            ]
        );
        let c = r.counts();
        assert_eq!((c.ok, c.outside, c.failed, c.pending), (1, 1, 1, 1));
        assert!(r.has_problems());
        assert_eq!(r.results[1].code, Some(OUT_OF_TOLERANCE));
        assert_eq!(r.results[3].detail, "no function yet");
    }

    #[test]
    fn a_report_with_only_pending_cases_has_no_problem() {
        let file = CaseFile::from_json(FILE).unwrap();
        let pending: Vec<_> = file
            .cases
            .iter()
            .filter(|c| c.status == Status::Pending)
            .cloned()
            .collect();
        let only = CaseFile {
            cases: pending,
            ..file
        };
        assert!(!run_files(&[only], None).has_problems());
    }

    #[test]
    fn the_report_is_the_same_every_time_in_text_and_in_json() {
        let (a, b) = (report(), report());
        assert_eq!(a.to_json(), b.to_json());
        for lang in Lang::ALL {
            assert_eq!(a.to_text(lang), b.to_text(lang));
        }
        let json: serde_json::Value = serde_json::from_str(&a.to_json()).unwrap();
        assert_eq!(json["format"], REPORT_FORMAT);
        assert_eq!(json["counts"]["ok"], 1);
        assert_eq!(json["cases"][1]["verdict"], "outside");
    }

    #[test]
    fn the_text_uses_the_words_and_the_titles_of_the_language() {
        let r = report();
        let es = r.to_text(Lang::Es);
        assert!(es.contains("correcto: ORACLE-U-001"), "{es}");
        assert!(es.contains("fuera de tolerancia: ORACLE-U-002"), "{es}");
        assert!(es.contains("pendiente: ORACLE-U-004"), "{es}");
        assert!(es.contains("PHOENIX-ORACLE-003"), "{es}");
        let pt = r.to_text(Lang::Pt);
        assert!(pt.contains("fora da tolerância"), "{pt}");
        assert!(pt.contains("O caso está pendente"), "{pt}");
    }

    #[test]
    fn a_suite_can_be_chosen_and_the_cases_listed() {
        let file = CaseFile::from_json(FILE).unwrap();
        let files = [file];
        assert_eq!(run_files(&files, Some("other")).results.len(), 0);
        assert_eq!(run_files(&files, Some("unit")).results.len(), 4);
        let list = list_cases(&files, None);
        assert_eq!(list.lines().count(), 4);
        assert!(list.starts_with("ORACLE-U-001 | cited | stackup | c0"));
        assert!(list.contains("ORACLE-U-004 | pending | rules | later"));
        assert!(list_cases(&files, Some("other")).is_empty());
    }
}
