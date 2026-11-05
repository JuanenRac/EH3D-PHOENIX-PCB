// SPDX-License-Identifier: GPL-3.0-or-later
//! The files of reference cases (`phoenix-oracle/1`).
//!
//! A file holds a suite of cases. A *cited* case names a check of this repository, the inputs of
//! the check, the value that an open source publishes (or that follows exactly from a definition),
//! the unit, the tolerance and who chose it, and the source with its place in it. A *pending* case
//! has no number: it says what would be compared and why that cannot be done yet.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::codes::{OracleError, CASE_INVALID, FILE_INVALID};

/// The format name of a case file.
pub const FORMAT: &str = "phoenix-oracle/1";

/// The state of a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The value and its source are written and the case must pass.
    Cited,
    /// The case cannot be run yet; the reason is written.
    Pending,
}

impl Status {
    /// The name the status has in the files.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Cited => "cited",
            Status::Pending => "pending",
        }
    }
}

/// How far a result may be from the reference: the larger of an absolute and a relative bound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    /// The absolute bound, in the unit of the case.
    #[serde(default)]
    pub abs: Option<f64>,
    /// The relative bound, as a fraction of the reference value.
    #[serde(default)]
    pub rel: Option<f64>,
}

impl Tolerance {
    /// The largest difference that is accepted for a reference value.
    pub fn allowed(&self, expected: f64) -> f64 {
        let abs = self.abs.unwrap_or(0.0);
        let rel = self.rel.unwrap_or(0.0) * expected.abs();
        abs.max(rel)
    }

    fn usable(&self) -> bool {
        let good = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x >= 0.0);
        (self.abs.is_some() || self.rel.is_some()) && good(self.abs) && good(self.rel)
    }
}

/// One reference case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// The identity of the case, `ORACLE-<AREA>-<NNN>`, unique in the repository.
    pub id: String,
    /// What the case checks, in English.
    pub title: String,
    /// The part of the suite the case is about (`stackup`, `rf`, `sim`, `rules`, `lengths`,
    /// `zones` or the area a pending case is for).
    pub area: String,
    /// The name of the check that is run; empty for a pending case that has none yet.
    #[serde(default)]
    pub check: String,
    /// The inputs of the check.
    #[serde(default)]
    pub inputs: Map<String, Value>,
    /// The reference value; only a cited case has one.
    #[serde(default)]
    pub expected: Option<f64>,
    /// The unit of the reference value.
    #[serde(default)]
    pub unit: String,
    /// The tolerance; only a cited case has one.
    #[serde(default)]
    pub tolerance: Option<Tolerance>,
    /// Who chose the tolerance and why.
    #[serde(default)]
    pub tolerance_note: String,
    /// The source of the value, with its place in it; for a pending case the source that will be
    /// used.
    pub source: String,
    /// Cited or pending.
    pub status: Status,
    /// Why a pending case is pending.
    #[serde(default)]
    pub reason: String,
}

fn invalid(id: &str, what: &str) -> OracleError {
    OracleError::new(CASE_INVALID, format!("{id}: {what}"))
}

fn id_is_well_formed(id: &str) -> bool {
    id.starts_with("ORACLE-")
        && id.len() > 7
        && id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

impl Case {
    /// Check the case by itself: a cited case has a check, a finite value, a usable tolerance, a
    /// note about it and a source; a pending case has a reason and no number.
    pub fn validate(&self) -> Result<(), OracleError> {
        let id = self.id.as_str();
        if !id_is_well_formed(id) {
            return Err(invalid(
                id,
                "the id must be ORACLE- followed by capital letters, digits and dashes",
            ));
        }
        if self.title.trim().is_empty() || self.area.trim().is_empty() {
            return Err(invalid(id, "the title and the area must not be empty"));
        }
        if self.source.trim().is_empty() {
            return Err(invalid(id, "every case names its source"));
        }
        match self.status {
            Status::Cited => {
                if self.check.trim().is_empty() {
                    return Err(invalid(id, "a cited case names its check"));
                }
                if !self.expected.is_some_and(f64::is_finite) {
                    return Err(invalid(id, "a cited case has a finite reference value"));
                }
                if !self.tolerance.as_ref().is_some_and(Tolerance::usable) {
                    return Err(invalid(
                        id,
                        "a cited case has a tolerance (abs or rel, not negative)",
                    ));
                }
                if self.tolerance_note.trim().is_empty() {
                    return Err(invalid(id, "a cited case says who chose the tolerance"));
                }
                if !self.reason.is_empty() {
                    return Err(invalid(id, "a cited case has no reason to be pending"));
                }
            }
            Status::Pending => {
                if self.reason.trim().is_empty() {
                    return Err(invalid(id, "a pending case says why it is pending"));
                }
                if self.expected.is_some() || self.tolerance.is_some() {
                    return Err(invalid(id, "a pending case carries no number"));
                }
            }
        }
        Ok(())
    }
}

/// A file of cases: one suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    /// Must be [`FORMAT`].
    pub format: String,
    /// The name of the suite.
    pub suite: String,
    /// What the suite is about, in English.
    pub title: String,
    /// The cases, in the order they are run.
    pub cases: Vec<Case>,
}

impl CaseFile {
    /// Read a file of cases from JSON text and check it: the format, every case and the identities
    /// used once.
    pub fn from_json(text: &str) -> Result<CaseFile, OracleError> {
        let file: CaseFile = serde_json::from_str(text)
            .map_err(|e| OracleError::new(FILE_INVALID, format!("not a file of cases: {e}")))?;
        if file.format != FORMAT {
            return Err(OracleError::new(
                FILE_INVALID,
                format!("the format must be {FORMAT}"),
            ));
        }
        if file.suite.trim().is_empty() || file.title.trim().is_empty() {
            return Err(OracleError::new(
                FILE_INVALID,
                "the suite and the title must not be empty",
            ));
        }
        let mut seen = BTreeSet::new();
        for case in &file.cases {
            case.validate()?;
            if !seen.insert(case.id.as_str()) {
                return Err(invalid(&case.id, "the id is used twice in the file"));
            }
        }
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cited() -> &'static str {
        r#"{"format":"phoenix-oracle/1","suite":"s","title":"t","cases":[
            {"id":"ORACLE-X-001","title":"c","area":"stackup","check":"const_c0",
             "expected":299792458.0,"unit":"m/s","tolerance":{"abs":0.0},
             "tolerance_note":"exact","source":"a book","status":"cited"}]}"#
    }

    #[test]
    fn a_cited_case_is_read() {
        let file = CaseFile::from_json(cited()).unwrap();
        assert_eq!(file.cases.len(), 1);
        assert_eq!(file.cases[0].status, Status::Cited);
        assert_eq!(file.cases[0].expected, Some(299_792_458.0));
        assert_eq!(Status::Pending.as_str(), "pending");
    }

    #[test]
    fn the_tolerance_is_the_larger_of_the_two_bounds() {
        let t = Tolerance {
            abs: Some(0.5),
            rel: Some(0.01),
        };
        assert!((t.allowed(10.0) - 0.5).abs() < 1e-12);
        assert!((t.allowed(100.0) - 1.0).abs() < 1e-12);
        assert!((t.allowed(-100.0) - 1.0).abs() < 1e-12);
        let none = Tolerance {
            abs: None,
            rel: None,
        };
        assert!(!none.usable());
        let negative = Tolerance {
            abs: Some(-1.0),
            rel: None,
        };
        assert!(!negative.usable());
    }

    #[test]
    fn a_pending_case_needs_a_reason_and_no_number() {
        let ok = cited()
            .replace("\"check\":\"const_c0\",", "")
            .replace("\"expected\":299792458.0,", "")
            .replace("\"tolerance\":{\"abs\":0.0},", "")
            .replace(
                "\"status\":\"cited\"",
                "\"status\":\"pending\",\"reason\":\"why\"",
            );
        assert!(CaseFile::from_json(&ok).is_ok());
        let no_reason = ok.replace(",\"reason\":\"why\"", "");
        assert!(CaseFile::from_json(&no_reason).is_err());
        let with_number = ok.replace("\"unit\"", "\"expected\":1.0,\"unit\"");
        assert!(CaseFile::from_json(&with_number).is_err());
    }

    #[test]
    fn a_cited_case_without_what_it_needs_is_refused() {
        for (from, to) in [
            ("\"expected\":299792458.0,", ""),
            ("\"tolerance\":{\"abs\":0.0},", ""),
            ("\"tolerance_note\":\"exact\",", ""),
            ("\"source\":\"a book\"", "\"source\":\" \""),
            ("\"check\":\"const_c0\",", ""),
            ("ORACLE-X-001", "oracle-1"),
        ] {
            let text = cited().replace(from, to);
            let error = CaseFile::from_json(&text).unwrap_err();
            assert_eq!(error.code, CASE_INVALID, "{from}");
        }
    }

    #[test]
    fn a_wrong_format_an_unknown_field_and_a_repeated_id_are_refused() {
        let wrong = cited().replace("phoenix-oracle/1", "phoenix-oracle/2");
        assert_eq!(CaseFile::from_json(&wrong).unwrap_err().code, FILE_INVALID);
        let extra = cited().replace("\"unit\"", "\"colour\":1,\"unit\"");
        assert_eq!(CaseFile::from_json(&extra).unwrap_err().code, FILE_INVALID);
        assert_eq!(
            CaseFile::from_json("not json").unwrap_err().code,
            FILE_INVALID
        );
        let one = CaseFile::from_json(cited()).unwrap();
        let mut twice = one.clone();
        twice.cases.push(one.cases[0].clone());
        let text = serde_json::to_string(&twice).unwrap();
        assert_eq!(CaseFile::from_json(&text).unwrap_err().code, CASE_INVALID);
    }
}
