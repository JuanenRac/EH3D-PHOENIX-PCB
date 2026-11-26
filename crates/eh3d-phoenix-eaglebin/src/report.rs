// SPDX-License-Identifier: GPL-3.0-or-later
//! The conversion report: how many objects of each kind were read and, for everything that was
//! not read as it is, a note with its code. Nothing is dropped without being counted here.
//!
//! Notes are grouped by code and subject; the report of the same file is always the same text.

use std::collections::BTreeMap;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

use crate::codes;

/// One group of notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    /// The code, `PHOENIX-EAGLEBIN-<NNN>`.
    pub code: &'static str,
    /// What it is about (a kind of object or of value), in English.
    pub subject: String,
    /// How many times.
    pub count: usize,
    /// Where it happened first (the name of a footprint, symbol or device set), when known.
    pub first: String,
}

/// The report of one conversion.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Report {
    /// The version that wrote the file, as `major.minor`.
    pub version: String,
    /// How many objects of each kind were converted.
    pub read: BTreeMap<&'static str, usize>,
    /// What was not converted as it is.
    pub notes: Vec<Note>,
}

impl Report {
    /// Counts one converted object of a kind.
    pub fn count(&mut self, kind: &'static str) {
        *self.read.entry(kind).or_insert(0) += 1;
    }

    /// Adds one note.
    pub fn note(&mut self, code: &'static str, subject: &str, place: &str) {
        self.note_many(code, subject, place, 1);
    }

    /// Adds `count` notes of the same kind.
    pub fn note_many(&mut self, code: &'static str, subject: &str, place: &str, count: usize) {
        if count == 0 {
            return;
        }
        match self
            .notes
            .binary_search_by(|n| (n.code, n.subject.as_str()).cmp(&(code, subject)))
        {
            Ok(at) => self.notes[at].count += count,
            Err(at) => self.notes.insert(
                at,
                Note {
                    code,
                    subject: subject.to_owned(),
                    count,
                    first: place.to_owned(),
                },
            ),
        }
    }

    /// How many converted objects of a kind.
    pub fn read_of(&self, kind: &str) -> usize {
        self.read.get(kind).copied().unwrap_or(0)
    }

    /// How many notes of a code.
    pub fn noted(&self, code: &str) -> usize {
        self.notes
            .iter()
            .filter(|n| n.code == code)
            .map(|n| n.count)
            .sum()
    }

    /// How many objects or values are missing or different, as [`Report::has_losses`] counts
    /// them.
    pub fn losses(&self) -> usize {
        self.notes
            .iter()
            .filter(|n| n.code != codes::ENCODING && n.code != codes::NOT_READ)
            .map(|n| n.count)
            .sum()
    }

    /// True when something of the file is missing or different in the converted library. The
    /// notes about text encoding and about data that has no meaning for a library do not count.
    pub fn has_losses(&self) -> bool {
        self.notes
            .iter()
            .any(|n| n.code != codes::ENCODING && n.code != codes::NOT_READ)
    }

    /// The report as text, with the titles of the codes in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!("EAGLE binary {}\n", self.version);
        for (kind, count) in &self.read {
            out.push_str(&format!("  {kind}: {count}\n"));
        }
        for n in &self.notes {
            out.push_str(&format!(
                "{} {}: {} x {}",
                n.code,
                codes::title(n.code, lang),
                n.count,
                n.subject
            ));
            if !n.first.is_empty() {
                out.push_str(&format!(" ({})", n.first));
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_grouped_and_kept_in_order_and_the_first_place_stays() {
        let mut r = Report::default();
        r.note(codes::VALUE, "pad shape", "B");
        r.note(codes::SKIPPED, "frame", "A");
        r.note(codes::VALUE, "pad shape", "C");
        r.note_many(codes::VALUE, "pin length", "D", 0);
        assert_eq!(r.notes.len(), 2);
        assert_eq!(r.notes[0].code, codes::SKIPPED);
        assert_eq!((r.notes[1].count, r.notes[1].first.as_str()), (2, "B"));
        assert_eq!(r.noted(codes::VALUE), 2);
        assert!(r.has_losses());
    }

    #[test]
    fn encoding_and_unread_data_are_not_losses() {
        let mut r = Report::default();
        r.note(codes::ENCODING, "name", "x");
        r.note(codes::NOT_READ, "grid", "");
        r.count("pad");
        r.count("pad");
        assert!(!r.has_losses());
        assert_eq!(r.read_of("pad"), 2);
        assert_eq!(r.read_of("smd"), 0);
        let text = r.to_text(Lang::Es);
        assert!(text.contains("pad: 2"));
        assert!(text.contains("Texto leído como Latin-1: 1 x name (x)"));
        assert!(text.contains("grid\n"));
    }
}
