// SPDX-License-Identifier: GPL-3.0-or-later
//! One expectation of the own corpus (`phoenix-corpus/1`): which file, which reader or writer
//! reads it, what must come out, and how the numbers were worked out by hand.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde::Deserialize;
use serde_json::Value;

use super::codes::{SuiteError, EXPECTATION_INVALID};

/// The format name of an expectation file.
pub const FORMAT: &str = "phoenix-corpus/1";

/// The kinds of entry, each one a reader, a writer or a chain of them.
pub const KINDS: &[&str] = &[
    "eagle",
    "eagle-board",
    "eagle-schematic",
    "kicad-board",
    "kicad-schematic",
    "kicad-footprint",
    "kicad-symbols",
    "kicad-project",
    "gerber",
    "board-json",
    "dsn",
    "dsn-ses",
    "spice",
    "roundtrip",
    "gerber-roundtrip",
];

/// A value read with a probe: a number and how far the result may be from it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    /// The value worked out by hand.
    pub value: f64,
    /// The largest accepted distance from it.
    pub abs: f64,
}

/// What must come out of the file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// `read` when the file is read, `refused` when the reader must refuse it.
    pub outcome: String,
    /// Every diagnostic code that must be seen, and no other (when given).
    #[serde(default)]
    pub codes: Option<Vec<String>>,
    /// Codes that must be among the ones seen.
    #[serde(default)]
    pub codes_include: Vec<String>,
    /// Measures worked out by hand: a measure that the result does not have counts as 0.
    #[serde(default)]
    pub measures: BTreeMap<String, Value>,
    /// Numbers of the solver with the distance accepted from each.
    #[serde(default)]
    pub probes: BTreeMap<String, Probe>,
}

/// An expectation file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// `phoenix-corpus/1`.
    pub format: String,
    /// `CORPUS-<AREA>-<NNN>`, unique in the corpus.
    pub id: String,
    /// What the piece shows.
    pub title: String,
    /// The reader or chain that runs (one of [`KINDS`]).
    pub kind: String,
    /// The data file, relative to the folder of the expectation.
    pub input: String,
    /// A second data file (the session of `dsn-ses`).
    #[serde(default)]
    pub session: Option<String>,
    /// How the numbers were worked out, one line each.
    pub how: Vec<String>,
    /// Limits of the reader for this piece: a number, or `@size` and `@size-1` for the size of
    /// the data file.
    #[serde(default)]
    pub limits: BTreeMap<String, Value>,
    /// The text of a rule file (`name = value` lines) for the rule check of a board.
    #[serde(default)]
    pub rules: Option<String>,
    /// True to run the electrical rule check on a schematic with the settings of the repository.
    #[serde(default)]
    pub erc: bool,
    /// The formats a round trip goes through, in order: `eagle`, `kicad` or `json`.
    #[serde(default)]
    pub via: Vec<String>,
    /// What a round trip compares.
    #[serde(default)]
    pub compare: Vec<String>,
    /// How far two positions may differ in a round trip, in nanometres.
    #[serde(default)]
    pub tolerance_nm: Option<i64>,
    /// What must come out.
    pub expect: Expect,
}

fn invalid(detail: impl Into<String>) -> SuiteError {
    SuiteError::new(EXPECTATION_INVALID, detail)
}

/// True for an identifier such as `CORPUS-EAGLE-001`.
pub fn id_is_valid(id: &str) -> bool {
    id.starts_with("CORPUS-")
        && id.len() > 7
        && id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

/// True for a relative path with no `..`, no root and no drive.
pub fn path_is_inside(path: &str) -> bool {
    let p = Path::new(path);
    !path.is_empty()
        && !path.contains('\\')
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

impl Entry {
    /// Reads and checks an expectation file.
    pub fn parse(text: &str) -> Result<Entry, SuiteError> {
        let entry: Entry =
            serde_json::from_str(text).map_err(|e| invalid(format!("not a valid file: {e}")))?;
        entry.check()?;
        Ok(entry)
    }

    /// The checks that need no data file.
    pub fn check(&self) -> Result<(), SuiteError> {
        if self.format != FORMAT {
            return Err(invalid(format!("the format is {:?}", self.format)));
        }
        if !id_is_valid(&self.id) {
            return Err(invalid(format!(
                "the id {:?} is not CORPUS-<AREA>-<NNN>",
                self.id
            )));
        }
        if self.title.trim().is_empty() {
            return Err(invalid(format!("{}: the title is empty", self.id)));
        }
        if !KINDS.contains(&self.kind.as_str()) {
            return Err(invalid(format!(
                "{}: unknown kind {:?}",
                self.id, self.kind
            )));
        }
        if self.how.is_empty() || self.how.iter().any(|l| l.trim().is_empty()) {
            return Err(invalid(format!(
                "{}: the note of how the numbers were worked out is missing",
                self.id
            )));
        }
        if !path_is_inside(&self.input) {
            return Err(invalid(format!(
                "{}: the input path is not inside",
                self.id
            )));
        }
        if let Some(session) = &self.session {
            if !path_is_inside(session) {
                return Err(invalid(format!(
                    "{}: the session path is not inside",
                    self.id
                )));
            }
        }
        if (self.kind == "dsn-ses") != self.session.is_some() {
            return Err(invalid(format!(
                "{}: only the kind dsn-ses has a session file, and it needs one",
                self.id
            )));
        }
        if self.expect.outcome != "read" && self.expect.outcome != "refused" {
            return Err(invalid(format!(
                "{}: the outcome is {:?}, not read or refused",
                self.id, self.expect.outcome
            )));
        }
        if self.kind == "roundtrip" {
            if self.via.is_empty() {
                return Err(invalid(format!("{}: a round trip needs via", self.id)));
            }
            if let Some(bad) = self
                .via
                .iter()
                .find(|v| !matches!(v.as_str(), "eagle" | "kicad" | "json"))
            {
                return Err(invalid(format!(
                    "{}: unknown format {bad:?} in via",
                    self.id
                )));
            }
        } else if !self.via.is_empty() || !self.compare.is_empty() {
            return Err(invalid(format!(
                "{}: via and compare belong to round trips",
                self.id
            )));
        }
        if self.erc && !matches!(self.kind.as_str(), "eagle-schematic" | "kicad-schematic") {
            return Err(invalid(format!(
                "{}: erc belongs to the kinds eagle-schematic and kicad-schematic",
                self.id
            )));
        }
        if self.rules.is_some() && self.kind != "board-json" {
            return Err(invalid(format!(
                "{}: rules belong to the kind board-json",
                self.id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "format": "phoenix-corpus/1",
        "id": "CORPUS-TEST-001",
        "title": "a piece",
        "kind": "eagle",
        "input": "a.brd",
        "how": ["counted by hand"],
        "expect": { "outcome": "read", "measures": { "parts": 2 } }
    }"#;

    #[test]
    fn a_good_expectation_is_read() {
        let e = Entry::parse(GOOD).unwrap();
        assert_eq!(e.id, "CORPUS-TEST-001");
        assert_eq!(e.expect.measures["parts"], Value::from(2));
        assert!(e.limits.is_empty() && e.via.is_empty() && e.session.is_none());
    }

    #[test]
    fn what_is_not_valid_is_refused_with_the_code() {
        let cases = [
            GOOD.replace("phoenix-corpus/1", "phoenix-corpus/2"),
            GOOD.replace("CORPUS-TEST-001", "TEST-001"),
            GOOD.replace("CORPUS-TEST-001", "CORPUS-test-001"),
            GOOD.replace("a piece", "  "),
            GOOD.replace("\"eagle\"", "\"nothing\""),
            GOOD.replace("[\"counted by hand\"]", "[]"),
            GOOD.replace("[\"counted by hand\"]", "[\" \"]"),
            GOOD.replace("a.brd", "../a.brd"),
            GOOD.replace("a.brd", "/a.brd"),
            GOOD.replace("a.brd", ""),
            GOOD.replace("a.brd", "x\\\\a.brd"),
            GOOD.replace("\"read\"", "\"maybe\""),
            GOOD.replace("\"eagle\"", "\"dsn-ses\""),
            GOOD.replace("\"eagle\"", "\"roundtrip\""),
            GOOD.replace("\"outcome\"", "\"colour\": 1, \"outcome\""),
            GOOD.replace("\"input\"", "\"via\": [\"kicad\"], \"input\""),
            GOOD.replace("\"input\"", "\"rules\": \"min_clearance = 1\", \"input\""),
            "not json".to_owned(),
        ];
        for text in cases {
            let e = Entry::parse(&text).unwrap_err();
            assert_eq!(e.code, EXPECTATION_INVALID, "{text}");
        }
    }

    #[test]
    fn a_round_trip_and_a_session_need_what_they_use() {
        let trip = GOOD
            .replace("\"eagle\"", "\"roundtrip\"")
            .replace("\"input\"", "\"via\": [\"kicad\"], \"input\"");
        assert!(Entry::parse(&trip).is_ok());
        let wrong = trip.replace("\"kicad\"", "\"gerber\"");
        assert!(Entry::parse(&wrong).is_err());
        let ses = GOOD
            .replace("\"eagle\"", "\"dsn-ses\"")
            .replace("\"input\"", "\"session\": \"s.ses\", \"input\"");
        assert!(Entry::parse(&ses).is_ok());
        let rules = GOOD
            .replace("\"eagle\"", "\"board-json\"")
            .replace("\"input\"", "\"rules\": \"min_clearance = 1\", \"input\"");
        assert!(Entry::parse(&rules).is_ok());
        let erc = GOOD
            .replace("\"eagle\"", "\"eagle-schematic\"")
            .replace("\"input\"", "\"erc\": true, \"input\"");
        assert!(Entry::parse(&erc).unwrap().erc);
        let wrong = GOOD.replace("\"input\"", "\"erc\": true, \"input\"");
        assert!(Entry::parse(&wrong).is_err());
    }

    #[test]
    fn the_paths_and_the_ids_are_checked() {
        assert!(id_is_valid("CORPUS-KICAD-012"));
        assert!(!id_is_valid("CORPUS-"));
        assert!(!id_is_valid("OTHER-KICAD-012"));
        assert!(path_is_inside("a/b.brd"));
        assert!(!path_is_inside("a/../b.brd"));
        assert!(!path_is_inside("./a/../../b"));
        assert!(!path_is_inside(""));
    }
}
