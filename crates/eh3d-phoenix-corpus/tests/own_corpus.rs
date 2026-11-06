// SPDX-License-Identifier: GPL-3.0-or-later
//! The own corpus of the repository (`tests/corpus`), read with the real readers and writers.

use std::collections::BTreeSet;
use std::path::PathBuf;

use eh3d_phoenix_corpus::suite::run::run;
use eh3d_phoenix_corpus::suite::{self, Corpus, CorpusSettings, Verdict, KINDS};
use eh3d_phoenix_rules::Lang;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

fn corpus() -> Corpus {
    suite::load(&root(), &CorpusSettings::default()).unwrap()
}

#[test]
fn the_corpus_is_consistent() {
    let corpus = corpus();
    let problems: Vec<String> = suite::check(&corpus)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(corpus.entries.len() >= 90, "{}", corpus.entries.len());
}

#[test]
fn every_piece_is_as_its_expectation_says() {
    let corpus = corpus();
    let report = run(&corpus, &CorpusSettings::default(), None, false);
    assert!(!report.has_problems(), "{}", report.to_text(Lang::En));
    assert_eq!(report.results.len(), corpus.entries.len());
}

#[test]
fn the_report_is_the_same_every_time() {
    let corpus = corpus();
    let settings = CorpusSettings::default();
    let first = run(&corpus, &settings, None, false);
    let second = run(&corpus, &settings, None, false);
    assert_eq!(first, second);
    assert_eq!(first.to_json(), second.to_json());
    assert_eq!(first.to_text(Lang::Pt), second.to_text(Lang::Pt));
}

#[test]
fn every_kind_of_piece_is_there_with_files_that_are_refused_on_purpose() {
    let corpus = corpus();
    let kinds: BTreeSet<&str> = corpus
        .entries
        .iter()
        .map(|l| l.entry.kind.as_str())
        .collect();
    let all: BTreeSet<&str> = KINDS.iter().copied().collect();
    assert_eq!(kinds, all);
    let refused = corpus
        .entries
        .iter()
        .filter(|l| l.entry.expect.outcome == "refused")
        .count();
    assert!(refused >= 30, "{refused}");
    let report = run(&corpus, &CorpusSettings::default(), None, false);
    let expected_errors = report
        .results
        .iter()
        .filter(|r| r.verdict == Verdict::ExpectedError)
        .count();
    assert_eq!(expected_errors, refused);
}

#[test]
fn the_formats_of_the_suite_are_all_in_the_corpus() {
    let corpus = corpus();
    let has = |ext: &str| {
        corpus
            .data_files
            .iter()
            .any(|f| f.to_ascii_lowercase().ends_with(ext))
    };
    for ext in [
        ".brd",
        ".sch",
        ".lbr",
        ".kicad_pcb",
        ".kicad_sch",
        ".kicad_mod",
        ".kicad_sym",
        ".kicad_pro",
        ".gtl",
        ".drl",
        ".board.json",
        ".ses",
        ".cir",
    ] {
        assert!(has(ext), "no file {ext} in the corpus");
    }
}

#[test]
fn the_properties_hold_for_the_seeds_of_the_settings() {
    let corpus = corpus();
    let settings = CorpusSettings {
        property_seeds: 16,
        ..CorpusSettings::default()
    };
    let report = run(&corpus, &settings, Some("CORPUS-TRIP-001"), true);
    assert!(!report.has_problems(), "{}", report.to_text(Lang::En));
    assert_eq!(report.results.len(), 1 + suite::PROPERTIES.len());
}

#[test]
fn the_pieces_are_small_and_hold_no_text_of_a_maker() {
    let corpus = corpus();
    for file in &corpus.data_files {
        let size = std::fs::metadata(root().join(file)).unwrap().len();
        assert!(size < 64 * 1024, "{file}: {size} bytes");
    }
    for loaded in &corpus.entries {
        let text = std::fs::read_to_string(root().join(&loaded.file)).unwrap();
        for word in ["autodesk", "copyright"] {
            assert!(
                !text.to_ascii_lowercase().contains(word),
                "{}: {word}",
                loaded.file
            );
        }
    }
}
