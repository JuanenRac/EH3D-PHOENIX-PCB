// SPDX-License-Identifier: GPL-3.0-or-later
//! What a synchronisation finds and does, as a report: the changes in one list and what could not
//! be judged or done in another, never mixed, in a fixed order, as text in eight languages and as
//! JSON.

use std::fmt::Write as _;

use eh3d_phoenix_rules::codes::word;
use eh3d_phoenix_rules::{Lang, Severity};
use serde::Serialize;
use serde_json::{json, Value};

use crate::codes::{title, ChangeKind};

/// The place of what could not be judged or done in the order of the report.
const RANK_SKIPPED: u8 = 9;

/// Which document is changed to agree with the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The schematic is right and the board is changed (transfer to the board).
    Forward,
    /// The board is right and the schematic is changed (backward annotation).
    Backward,
}

impl Direction {
    /// The word used in the JSON report.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Forward => "forward",
            Direction::Backward => "backward",
        }
    }
}

/// One difference between the two documents that a synchronisation removes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    /// What kind of difference it is.
    pub kind: ChangeKind,
    /// What it is about, by names: `R1`, `R1.2`, `net GND`.
    pub location: String,
    /// What the document that is changed holds now.
    pub from: String,
    /// What it will hold.
    pub to: String,
}

impl Change {
    pub(crate) fn new(kind: ChangeKind, location: &str, from: &str, to: &str) -> Change {
        Change {
            kind,
            location: location.to_owned(),
            from: from.to_owned(),
            to: to.to_owned(),
        }
    }

    pub(crate) fn sort_key(&self) -> (u8, &str, &str, &str) {
        (self.kind.rank(), &self.location, &self.from, &self.to)
    }
}

/// Something that could not be judged or done, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The stable code of the reason.
    pub code: &'static str,
    /// What it is about.
    pub location: String,
    /// More words, English.
    pub detail: String,
}

impl Skipped {
    pub(crate) fn new(code: &'static str, location: &str, detail: &str) -> Skipped {
        Skipped {
            code,
            location: location.to_owned(),
            detail: detail.to_owned(),
        }
    }
}

/// One line of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The place in the fixed order of the report.
    pub rank: u8,
    /// The stable name (`refdes_differs`).
    pub rule: &'static str,
    /// The stable code (`PHOENIX-SYNC-003`).
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What it is about.
    pub location: String,
    /// More words, English.
    pub detail: String,
}

impl Finding {
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

/// The report of a synchronisation: the changes that are (or would be) made, and apart what could
/// not be judged or done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Which document is changed.
    pub direction: Direction,
    /// How many operations the package holds; zero means the documents agree.
    pub pending: usize,
    /// The changes, in a fixed order.
    pub confirmed: Vec<Finding>,
    /// What could not be judged or done, in a fixed order.
    pub inconclusive: Vec<Finding>,
}

impl Report {
    pub(crate) fn build(
        direction: Direction,
        pending: usize,
        changes: &[Change],
        skipped: &[Skipped],
    ) -> Report {
        let mut sorted: Vec<&Change> = changes.iter().collect();
        sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        let confirmed = sorted
            .into_iter()
            .map(|c| Finding {
                rank: c.kind.rank(),
                rule: c.kind.name(),
                code: c.kind.code(),
                severity: if c.kind == ChangeKind::MissingPart {
                    Severity::Info
                } else {
                    Severity::Warning
                },
                location: c.location.clone(),
                detail: format!("{} -> {}", c.from, c.to),
            })
            .collect();
        let mut inconclusive: Vec<Finding> = skipped
            .iter()
            .map(|s| Finding {
                rank: RANK_SKIPPED,
                rule: "skipped",
                code: s.code,
                severity: Severity::Info,
                location: s.location.clone(),
                detail: s.detail.clone(),
            })
            .collect();
        inconclusive.sort_by(|a, b| {
            (a.code, &a.location, &a.detail).cmp(&(b.code, &b.location, &b.detail))
        });
        Report {
            direction,
            pending,
            confirmed,
            inconclusive,
        }
    }

    /// True when nothing is left to do: the two documents agree.
    pub fn in_sync(&self) -> bool {
        self.pending == 0
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

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        let value = json!({
            "schema": "phoenix-sync/1",
            "direction": self.direction.as_str(),
            "pending": self.pending,
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
    use crate::codes::FOOTPRINT_UNUSABLE;

    #[test]
    fn the_changes_are_sorted_whatever_the_order_they_were_found() {
        let a = Change::new(ChangeKind::ValueDiffers, "R2", "1k", "2k");
        let b = Change::new(ChangeKind::ValueDiffers, "R1", "1k", "2k");
        let c = Change::new(ChangeKind::NewPart, "C1", "-", "C0603");
        let one = Report::build(
            Direction::Forward,
            3,
            &[a.clone(), b.clone(), c.clone()],
            &[],
        );
        let two = Report::build(Direction::Forward, 3, &[c, b, a], &[]);
        assert_eq!(one, two);
        assert_eq!(one.to_json(), two.to_json());
        assert_eq!(one.confirmed[0].location, "C1");
        assert_eq!(one.confirmed[1].location, "R1");
        assert_eq!(one.confirmed[1].detail, "1k -> 2k");
        assert!(!one.in_sync());
    }

    #[test]
    fn the_text_keeps_the_lists_apart_and_translates_the_title() {
        let change = Change::new(ChangeKind::RefdesDiffers, "R1", "R1", "R7");
        let skipped = Skipped::new(FOOTPRINT_UNUSABLE, "C9", "no footprint");
        let r = Report::build(Direction::Backward, 1, &[change], &[skipped]);
        let en = r.to_text(Lang::En);
        assert!(en.starts_with("Confirmed: 1\n"));
        assert!(
            en.contains("[warning] PHOENIX-SYNC-003 Reference designators differ: R1 | R1 -> R7")
        );
        assert!(en.contains("Inconclusive: 1\n"));
        assert!(en.contains("| no footprint"));
        assert!(r
            .to_text(Lang::Pt)
            .contains("As referências são diferentes"));
        let empty = Report::build(Direction::Forward, 0, &[], &[]);
        assert!(empty.in_sync());
        assert!(empty.to_text(Lang::En).contains("nothing found"));
    }

    #[test]
    fn json_has_both_lists_the_direction_and_the_count() {
        let change = Change::new(ChangeKind::NetAdded, "net GND", "-", "GND");
        let r = Report::build(Direction::Forward, 2, &[change], &[]);
        let value: Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(value["schema"], "phoenix-sync/1");
        assert_eq!(value["direction"], "forward");
        assert_eq!(value["pending"], 2);
        assert_eq!(value["confirmed"][0]["code"], "PHOENIX-SYNC-006");
        assert_eq!(value["inconclusive"].as_array().map(Vec::len), Some(0));
    }
}
