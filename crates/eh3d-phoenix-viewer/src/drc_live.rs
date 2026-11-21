// SPDX-License-Identifier: GPL-3.0-or-later
//! What the rule check says after each edit, for the line under the board editor: how many rule
//! errors there were and are, how many findings the edit added and removed, where the first new
//! error is (so that the window can look at it), and whether the edit made things better, worse
//! or left them as they were. It reads the report that the editor already makes after every
//! operation (`DrcReport`); it checks nothing itself.

use eh3d_phoenix_boardedit::edit::DrcReport;
use eh3d_phoenix_rules::{Lang, Severity};

use crate::codes::column;

/// How an edit changed the rule errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// There are fewer errors than before.
    Better,
    /// The same number of errors, or none.
    Same,
    /// There are more errors than before.
    Worse,
}

/// The summary of one report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDrc {
    /// The rule errors before the edit.
    pub before: usize,
    /// The rule errors after it.
    pub after: usize,
    /// The findings the edit added (errors and warnings).
    pub added: usize,
    /// The findings the edit removed.
    pub removed: usize,
    /// The place of the first error that the edit added, in nanometres.
    pub first_new_error_at_nm: Option<[i64; 2]>,
    /// Whether the edit made things better, worse or left them.
    pub tone: Tone,
}

/// The summary of a report.
pub fn summarise(report: &DrcReport) -> LiveDrc {
    let tone = match report.errors_after.cmp(&report.errors_before) {
        std::cmp::Ordering::Less => Tone::Better,
        std::cmp::Ordering::Equal => Tone::Same,
        std::cmp::Ordering::Greater => Tone::Worse,
    };
    LiveDrc {
        before: report.errors_before,
        after: report.errors_after,
        added: report.added.len(),
        removed: report.removed.len(),
        first_new_error_at_nm: report
            .added
            .iter()
            .find(|f| f.severity == Severity::Error)
            .and_then(|f| f.at_nm),
        tone,
    }
}

/// The words of the line.
#[derive(Debug, Clone, Copy)]
enum Word {
    Errors,
    New,
    Fixed,
    Clean,
}

const WORDS: [(Word, [&str; 8]); 4] = [
    (
        Word::Errors,
        [
            "Rule errors",
            "Errores de reglas",
            "Regelfehler",
            "Erreurs de règles",
            "Errori di regole",
            "規則エラー",
            "规则错误",
            "Erros de regras",
        ],
    ),
    (
        Word::New,
        [
            "new", "nuevos", "neu", "nouveaux", "nuovi", "新規", "新增", "novos",
        ],
    ),
    (
        Word::Fixed,
        [
            "fixed",
            "resueltos",
            "behoben",
            "résolus",
            "risolti",
            "解消",
            "已消除",
            "resolvidos",
        ],
    ),
    (
        Word::Clean,
        [
            "no rule errors",
            "sin errores de reglas",
            "keine Regelfehler",
            "aucune erreur de règle",
            "nessun errore di regole",
            "規則エラーなし",
            "没有规则错误",
            "sem erros de regras",
        ],
    ),
];

fn word(w: Word, lang: Lang) -> &'static str {
    let at = WORDS
        .iter()
        .position(|(x, _)| std::mem::discriminant(x) == std::mem::discriminant(&w))
        .unwrap_or(0);
    WORDS[at].1[column(lang)]
}

impl LiveDrc {
    /// The line for the status of the editor in a language: `Rule errors 0 -> 2 (+3 new, -1
    /// fixed)`, or that there are none.
    pub fn line(&self, lang: Lang) -> String {
        if self.after == 0 && self.before == 0 && self.added == 0 && self.removed == 0 {
            return word(Word::Clean, lang).to_owned();
        }
        format!(
            "{} {} -> {} (+{} {}, -{} {})",
            word(Word::Errors, lang),
            self.before,
            self.after,
            self.added,
            word(Word::New, lang),
            self.removed,
            word(Word::Fixed, lang),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rules::Finding;

    fn finding(severity: Severity, at: Option<[i64; 2]>) -> Finding {
        Finding {
            rank: 0,
            rule: "copper_clearance",
            code: "PHOENIX-CHECK-005",
            severity,
            location: "a".to_owned(),
            at_nm: at,
            measured_nm: None,
            limit_nm: None,
            detail: String::new(),
        }
    }

    fn report(before: usize, after: usize, added: Vec<Finding>, removed: usize) -> DrcReport {
        DrcReport {
            errors_before: before,
            errors_after: after,
            added,
            removed: (0..removed)
                .map(|_| finding(Severity::Error, None))
                .collect(),
        }
    }

    #[test]
    fn the_tone_follows_the_count_of_errors() {
        assert_eq!(summarise(&report(1, 3, vec![], 0)).tone, Tone::Worse);
        assert_eq!(summarise(&report(3, 1, vec![], 2)).tone, Tone::Better);
        assert_eq!(summarise(&report(2, 2, vec![], 0)).tone, Tone::Same);
    }

    #[test]
    fn the_first_new_error_is_where_to_look() {
        let added = vec![
            finding(Severity::Warning, Some([1, 1])),
            finding(Severity::Error, Some([5, 6])),
            finding(Severity::Error, Some([9, 9])),
        ];
        let s = summarise(&report(0, 2, added, 0));
        assert_eq!(s.first_new_error_at_nm, Some([5, 6]));
        assert_eq!((s.added, s.removed), (3, 0));
        assert_eq!(
            summarise(&report(0, 0, vec![], 0)).first_new_error_at_nm,
            None
        );
    }

    #[test]
    fn the_line_says_the_same_in_the_eight_languages() {
        let s = summarise(&report(0, 2, vec![finding(Severity::Error, None); 3], 1));
        assert_eq!(s.line(Lang::En), "Rule errors 0 -> 2 (+3 new, -1 fixed)");
        assert_eq!(
            s.line(Lang::Pt),
            "Erros de regras 0 -> 2 (+3 novos, -1 resolvidos)"
        );
        for lang in Lang::ALL {
            assert!(s.line(lang).contains("0 -> 2"), "{lang:?}");
        }
        let clean = summarise(&report(0, 0, vec![], 0));
        for lang in Lang::ALL {
            assert!(!clean.line(lang).contains("->"), "{lang:?}");
        }
    }

    #[test]
    fn every_word_exists_in_the_eight_languages() {
        for (_, row) in WORDS.iter() {
            for text in row {
                assert!(!text.is_empty());
            }
        }
        for lang in [Lang::Ja, Lang::Zh] {
            assert!(!word(Word::Errors, lang).is_ascii());
        }
    }
}
