// SPDX-License-Identifier: GPL-3.0-or-later
//! The walk over the own corpus: load the expectations, check that the corpus is consistent,
//! list the entries, run them and write a report that is the same every time.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use super::codes::{
    column, title, SuiteError, CORPUS_INCONSISTENT, FILE_TOO_LARGE, FOLDER_UNREADABLE,
    PROPERTY_BROKEN, WORDS,
};
use super::entry::Entry;
use super::outcome::{judge, Verdict};
use super::props;
use super::read::run_entry;
use super::settings::CorpusSettings;

/// The ending of an expectation file.
pub const EXPECT_SUFFIX: &str = ".expect.json";

/// An expectation with the place it was read from.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The expectation.
    pub entry: Entry,
    /// The path of the expectation file below the corpus folder, with `/`.
    pub file: String,
    /// The folder of the expectation below the corpus folder, with `/`; empty for the root.
    pub folder: String,
}

/// The corpus as it lies on disk.
#[derive(Debug, Clone)]
pub struct Corpus {
    /// The folder of the corpus.
    pub root: PathBuf,
    /// The expectations that could be read, by id.
    pub entries: Vec<Loaded>,
    /// The expectation files that could not be read, with why.
    pub problems: Vec<(String, SuiteError)>,
    /// The data files (everything that is not an expectation), below the folder, with `/`.
    pub data_files: BTreeSet<String>,
}

fn unreadable(detail: impl Into<String>) -> SuiteError {
    SuiteError::new(FOLDER_UNREADABLE, detail)
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    settings: &CorpusSettings,
    files: &mut Vec<String>,
) -> Result<(), SuiteError> {
    let read = fs::read_dir(dir).map_err(|e| unreadable(format!("{}: {e}", dir.display())))?;
    let mut items: Vec<PathBuf> = Vec::new();
    for item in read {
        let item = item.map_err(|e| unreadable(format!("{}: {e}", dir.display())))?;
        items.push(item.path());
    }
    items.sort();
    for path in items {
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| unreadable(format!("{}: {e}", path.display())))?
            .file_type();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if depth + 1 > settings.max_depth {
                return Err(unreadable(format!(
                    "{}: deeper than {} folders",
                    path.display(),
                    settings.max_depth
                )));
            }
            walk(root, &path, depth + 1, settings, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| unreadable(e.to_string()))?;
            let parts: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            files.push(parts.join("/"));
            if files.len() > settings.max_entries.saturating_mul(8) {
                return Err(unreadable(format!(
                    "more than {} files",
                    settings.max_entries.saturating_mul(8)
                )));
            }
        }
    }
    Ok(())
}

/// Reads the corpus folder. An expectation that is not valid is kept as a problem, not as an
/// error, so that one bad file does not hide the others.
pub fn load(root: &Path, settings: &CorpusSettings) -> Result<Corpus, SuiteError> {
    if !root.is_dir() {
        return Err(unreadable(format!("{} is not a folder", root.display())));
    }
    let mut files: Vec<String> = Vec::new();
    walk(root, root, 0, settings, &mut files)?;
    files.sort();
    let mut corpus = Corpus {
        root: root.to_path_buf(),
        entries: Vec::new(),
        problems: Vec::new(),
        data_files: BTreeSet::new(),
    };
    for file in files {
        let name = file.rsplit('/').next().unwrap_or_default();
        if !file.ends_with(EXPECT_SUFFIX) {
            if !name.starts_with('.') {
                corpus.data_files.insert(file);
            }
            continue;
        }
        let path = root.join(&file);
        let size = fs::metadata(&path)
            .map_err(|e| unreadable(format!("{file}: {e}")))?
            .len();
        if size > settings.max_file_bytes {
            corpus.problems.push((
                file.clone(),
                SuiteError::new(FILE_TOO_LARGE, format!("{file}: {size} bytes")),
            ));
            continue;
        }
        let text = match fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                corpus
                    .problems
                    .push((file.clone(), unreadable(format!("{file}: {e}"))));
                continue;
            }
        };
        match Entry::parse(&text) {
            Ok(entry) => {
                let folder = file
                    .rsplit_once('/')
                    .map(|(f, _)| f.to_owned())
                    .unwrap_or_default();
                corpus.entries.push(Loaded {
                    entry,
                    file,
                    folder,
                });
            }
            Err(e) => corpus.problems.push((file, e)),
        }
    }
    if corpus.entries.len() + corpus.problems.len() > settings.max_entries {
        return Err(unreadable(format!(
            "more than {} expectation files",
            settings.max_entries
        )));
    }
    corpus.entries.sort_by(|a, b| {
        (a.entry.id.as_str(), a.file.as_str()).cmp(&(b.entry.id.as_str(), b.file.as_str()))
    });
    Ok(corpus)
}

fn data_path(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

/// The inconsistencies of the corpus, without running anything: expectations that could not be
/// read, ids used twice, data files that an expectation names and are not there, and data files
/// that no expectation names.
pub fn check(corpus: &Corpus) -> Vec<SuiteError> {
    let mut out: Vec<SuiteError> = Vec::new();
    for (file, e) in &corpus.problems {
        out.push(SuiteError::new(e.code, format!("{file}: {}", e.detail)));
    }
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for loaded in &corpus.entries {
        let e = &loaded.entry;
        if let Some(first) = seen.insert(e.id.as_str(), loaded.file.as_str()) {
            out.push(SuiteError::new(
                CORPUS_INCONSISTENT,
                format!("{} is the id of {first} and of {}", e.id, loaded.file),
            ));
        }
        let mut names = vec![e.input.as_str()];
        names.extend(e.session.as_deref());
        for name in names {
            let path = data_path(&loaded.folder, name);
            if !corpus.data_files.contains(&path) {
                out.push(SuiteError::new(
                    CORPUS_INCONSISTENT,
                    format!("{}: the data file {path} is not there", e.id),
                ));
            }
            used.insert(path);
        }
    }
    for file in &corpus.data_files {
        if !used.contains(file) {
            out.push(SuiteError::new(
                CORPUS_INCONSISTENT,
                format!("{file}: no expectation names this file"),
            ));
        }
    }
    out
}

/// One line per entry: id, kind, outcome and title.
pub fn list(corpus: &Corpus, filter: Option<&str>) -> String {
    let mut out = String::new();
    for loaded in &corpus.entries {
        let e = &loaded.entry;
        if filter.is_some_and(|f| !e.id.contains(f) && !e.kind.contains(f)) {
            continue;
        }
        out.push_str(&format!(
            "{} | {} | {} | {}\n",
            e.id, e.kind, e.expect.outcome, e.title
        ));
    }
    out
}

/// The result of one entry or one property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryResult {
    /// The id of the entry, or `PROPERTY-<NAME>`.
    pub id: String,
    /// The kind of the entry, or `property`.
    pub kind: String,
    /// What the piece shows.
    pub title: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The code of the verdict, none for ok.
    pub code: Option<&'static str>,
    /// What differs or why it failed.
    pub details: Vec<String>,
}

/// The report of a run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// One result per entry, in the order of the ids, then the properties.
    pub results: Vec<EntryResult>,
}

fn verdict_index(v: Verdict) -> usize {
    match v {
        Verdict::Ok => 0,
        Verdict::Difference => 1,
        Verdict::ExpectedError => 2,
        Verdict::Failed => 3,
    }
}

fn verdict_name(v: Verdict) -> &'static str {
    ["ok", "difference", "expected_error", "failed"][verdict_index(v)]
}

impl Report {
    /// How many results have each verdict: ok, difference, expected error, failed.
    pub fn counts(&self) -> [usize; 4] {
        let mut n = [0usize; 4];
        for r in &self.results {
            n[verdict_index(r.verdict)] += 1;
        }
        n
    }

    /// True when a result is a difference or a failure.
    pub fn has_problems(&self) -> bool {
        let n = self.counts();
        n[1] + n[3] > 0
    }

    /// The report as text in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let col = column(lang);
        let mut out = String::new();
        for r in &self.results {
            let word = WORDS[verdict_index(r.verdict)][col];
            out.push_str(&format!("{word}: {} ({}) {}", r.id, r.kind, r.title));
            if let Some(code) = r.code {
                out.push_str(&format!(" [{code}: {}]", title(code, lang)));
            }
            out.push('\n');
            for line in &r.details {
                out.push_str(&format!("  {line}\n"));
            }
        }
        let n = self.counts();
        out.push_str(&format!(
            "corpus: {} | {} {} | {} {} | {} {} | {} {}\n",
            self.results.len(),
            WORDS[0][col],
            n[0],
            WORDS[2][col],
            n[2],
            WORDS[1][col],
            n[1],
            WORDS[3][col],
            n[3]
        ));
        out
    }

    /// The report as JSON (`phoenix-corpus-report/1`).
    pub fn to_json(&self) -> String {
        let n = self.counts();
        let results: Vec<Value> = self
            .results
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "kind": r.kind,
                    "title": r.title,
                    "verdict": verdict_name(r.verdict),
                    "code": r.code,
                    "details": r.details,
                })
            })
            .collect();
        let value = json!({
            "format": "phoenix-corpus-report/1",
            "counts": {
                "ok": n[0],
                "difference": n[1],
                "expected_error": n[2],
                "failed": n[3],
            },
            "results": results,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn entry_result(loaded: &Loaded, corpus: &Corpus, settings: &CorpusSettings) -> EntryResult {
    let e = &loaded.entry;
    let folder = corpus.root.join(&loaded.folder);
    let (verdict, code, details) = match run_entry(e, &folder, settings) {
        Ok(outcome) => {
            let j = judge(e, &outcome);
            (j.verdict, j.code, j.lines)
        }
        Err(error) => (Verdict::Failed, Some(error.code), vec![error.detail]),
    };
    EntryResult {
        id: e.id.clone(),
        kind: e.kind.clone(),
        title: e.title.clone(),
        verdict,
        code,
        details,
    }
}

/// Runs the entries (all, or the ones whose id or kind contains `filter`) and, when
/// `properties` is true, the properties on generated boards. An expectation file that could not
/// be read is a failed result.
pub fn run(
    corpus: &Corpus,
    settings: &CorpusSettings,
    filter: Option<&str>,
    properties: bool,
) -> Report {
    let mut report = Report::default();
    for (file, e) in &corpus.problems {
        report.results.push(EntryResult {
            id: file.clone(),
            kind: "expectation".to_owned(),
            title: "an expectation file that could not be read".to_owned(),
            verdict: Verdict::Failed,
            code: Some(e.code),
            details: vec![e.detail.clone()],
        });
    }
    for loaded in &corpus.entries {
        let e = &loaded.entry;
        if filter.is_some_and(|f| !e.id.contains(f) && !e.kind.contains(f)) {
            continue;
        }
        report.results.push(entry_result(loaded, corpus, settings));
    }
    if properties {
        for p in props::run_all(settings, None) {
            let (verdict, code, details) = match &p.failure {
                None => (
                    Verdict::Ok,
                    None,
                    vec![format!(
                        "{} seeds from {}",
                        p.seeds, settings.property_first_seed
                    )],
                ),
                Some((seed, why)) => (
                    Verdict::Failed,
                    Some(PROPERTY_BROKEN),
                    vec![format!("seed {seed}: {why}")],
                ),
            };
            report.results.push(EntryResult {
                id: format!("PROPERTY-{}", p.name.to_ascii_uppercase()),
                kind: "property".to_owned(),
                title: format!("property {} on generated boards", p.name),
                verdict,
                code,
                details,
            });
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_BRD: &str = r#"<?xml version="1.0"?><eagle version="9.6.2"><drawing><layers><layer number="1" name="Top" color="1" fill="1" visible="yes" active="yes"/></layers><board/></drawing></eagle>"#;
    const CUT_BRD: &str = r#"<?xml version="1.0"?><eagle version="9.6.2"><drawing>"#;

    fn expect(id: &str, input: &str, outcome: &str, extra: &str) -> String {
        format!(
            r#"{{"format":"phoenix-corpus/1","id":"{id}","title":"piece {id}","kind":"eagle",
            "input":"{input}","how":["by hand"],"expect":{{"outcome":"{outcome}"{extra}}}}}"#
        )
    }

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn sample() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        write(p, "e/good.brd", GOOD_BRD);
        write(
            p,
            "e/good.brd.expect.json",
            &expect(
                "CORPUS-T-001",
                "good.brd",
                "read",
                r#","measures":{"layers":1,"parts":0,"kind":"board"}"#,
            ),
        );
        write(p, "e/cut.brd", CUT_BRD);
        write(
            p,
            "e/cut.brd.expect.json",
            &expect(
                "CORPUS-T-002",
                "cut.brd",
                "refused",
                r#","codes":["PHOENIX-EAGLE-001"]"#,
            ),
        );
        dir
    }

    #[test]
    fn the_corpus_is_loaded_listed_and_found_consistent() {
        let dir = sample();
        let settings = CorpusSettings::default();
        let corpus = load(dir.path(), &settings).unwrap();
        assert_eq!(corpus.entries.len(), 2);
        assert!(corpus.problems.is_empty());
        assert_eq!(check(&corpus), Vec::new());
        let text = list(&corpus, None);
        assert_eq!(
            text,
            "CORPUS-T-001 | eagle | read | piece CORPUS-T-001\nCORPUS-T-002 | eagle | refused | piece CORPUS-T-002\n"
        );
        assert_eq!(list(&corpus, Some("002")).lines().count(), 1);
        assert_eq!(list(&corpus, Some("nothing")), "");
    }

    #[test]
    fn what_is_not_consistent_is_found_without_running() {
        let dir = sample();
        let p = dir.path();
        write(p, "e/orphan.brd", GOOD_BRD);
        write(p, ".gitattributes", "* -text\n");
        write(
            p,
            "e/missing.brd.expect.json",
            &expect("CORPUS-T-003", "missing.brd", "read", ""),
        );
        write(
            p,
            "e/twice.brd.expect.json",
            &expect("CORPUS-T-001", "good.brd", "read", ""),
        );
        write(p, "e/broken.brd.expect.json", "{ not json");
        let corpus = load(p, &CorpusSettings::default()).unwrap();
        let problems = check(&corpus);
        let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
        let has = |needle: &str| text.iter().any(|l| l.contains(needle));
        assert!(
            has("e/orphan.brd: no expectation names this file"),
            "{text:?}"
        );
        assert!(has("the data file e/missing.brd is not there"), "{text:?}");
        assert!(has("CORPUS-T-001 is the id of"), "{text:?}");
        assert!(has("e/broken.brd.expect.json"), "{text:?}");
        assert!(!has(".gitattributes"), "{text:?}");
        assert!(problems
            .iter()
            .all(|e| e.code.starts_with("PHOENIX-CORPUS-")));
    }

    #[test]
    fn the_entries_run_and_the_report_says_ok_and_expected_error() {
        let dir = sample();
        let settings = CorpusSettings::default();
        let corpus = load(dir.path(), &settings).unwrap();
        let report = run(&corpus, &settings, None, false);
        assert_eq!(
            report.counts(),
            [1, 0, 1, 0],
            "{}",
            report.to_text(Lang::En)
        );
        assert!(!report.has_problems());
        let text = report.to_text(Lang::En);
        assert!(
            text.starts_with("ok: CORPUS-T-001 (eagle) piece CORPUS-T-001\n"),
            "{text}"
        );
        assert!(
            text.contains("expected error: CORPUS-T-002 (eagle)"),
            "{text}"
        );
        assert!(text.contains("[PHOENIX-CORPUS-006: "), "{text}");
        assert!(
            text.ends_with("corpus: 2 | ok 1 | expected error 1 | difference 0 | failed 0\n"),
            "{text}"
        );
        let pt = report.to_text(Lang::Pt);
        assert!(pt.contains("erro esperado: CORPUS-T-002"), "{pt}");
        assert_eq!(report.to_text(Lang::En), report.to_text(Lang::En));
    }

    #[test]
    fn a_difference_and_a_failure_are_reported_with_their_codes() {
        let dir = sample();
        let p = dir.path();
        write(
            p,
            "e/wrong.brd.expect.json",
            &expect(
                "CORPUS-T-003",
                "good.brd",
                "read",
                r#","measures":{"layers":7}"#,
            ),
        );
        write(
            p,
            "e/refused.brd.expect.json",
            &expect("CORPUS-T-004", "cut.brd", "read", ""),
        );
        write(
            p,
            "e/gone.brd.expect.json",
            &expect("CORPUS-T-005", "gone.brd", "read", ""),
        );
        let settings = CorpusSettings::default();
        let corpus = load(p, &settings).unwrap();
        let report = run(&corpus, &settings, None, false);
        assert_eq!(
            report.counts(),
            [1, 1, 1, 2],
            "{}",
            report.to_text(Lang::En)
        );
        assert!(report.has_problems());
        let by_id = |id: &str| report.results.iter().find(|r| r.id == id).unwrap();
        assert_eq!(by_id("CORPUS-T-003").code, Some("PHOENIX-CORPUS-004"));
        assert_eq!(
            by_id("CORPUS-T-003").details,
            vec!["layers: expected 7, got 1".to_owned()]
        );
        assert_eq!(by_id("CORPUS-T-004").code, Some("PHOENIX-CORPUS-005"));
        assert_eq!(by_id("CORPUS-T-005").code, Some("PHOENIX-CORPUS-003"));
    }

    #[test]
    fn the_json_report_has_the_counts_and_a_result_for_each_entry() {
        let dir = sample();
        let settings = CorpusSettings::default();
        let corpus = load(dir.path(), &settings).unwrap();
        let report = run(&corpus, &settings, Some("good"), false);
        assert!(report.results.is_empty());
        let report = run(&corpus, &settings, Some("eagle"), false);
        let json: Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(json["format"], "phoenix-corpus-report/1");
        assert_eq!(json["counts"]["ok"], 1);
        assert_eq!(json["counts"]["expected_error"], 1);
        assert_eq!(json["results"][0]["verdict"], "ok");
        assert_eq!(json["results"][0]["code"], Value::Null);
        assert_eq!(json["results"][1]["code"], "PHOENIX-CORPUS-006");
    }

    #[test]
    fn the_properties_are_part_of_the_report_when_asked() {
        let dir = sample();
        let settings = CorpusSettings {
            property_seeds: 2,
            ..CorpusSettings::default()
        };
        let corpus = load(dir.path(), &settings).unwrap();
        let report = run(&corpus, &settings, Some("CORPUS-T-001"), true);
        assert_eq!(report.results.len(), 1 + props::PROPERTIES.len());
        let last = report.results.last().unwrap();
        assert_eq!(last.kind, "property");
        assert_eq!(last.verdict, Verdict::Ok);
        assert_eq!(last.details, vec!["2 seeds from 1".to_owned()]);
    }

    #[test]
    fn a_folder_that_is_not_there_or_too_deep_or_too_big_is_refused() {
        let settings = CorpusSettings::default();
        let missing = std::env::temp_dir().join("phoenix-corpus-no-such-folder");
        assert_eq!(
            load(&missing, &settings).unwrap_err().code,
            FOLDER_UNREADABLE
        );
        let dir = sample();
        let shallow = CorpusSettings {
            max_depth: 1,
            ..CorpusSettings::default()
        };
        write(dir.path(), "a/b/c.brd", GOOD_BRD);
        assert_eq!(
            load(dir.path(), &shallow).unwrap_err().code,
            FOLDER_UNREADABLE
        );
        let few = CorpusSettings {
            max_entries: 1,
            ..CorpusSettings::default()
        };
        assert_eq!(load(dir.path(), &few).unwrap_err().code, FOLDER_UNREADABLE);
        let small = CorpusSettings {
            max_file_bytes: 10,
            ..CorpusSettings::default()
        };
        let corpus = load(dir.path(), &small).unwrap();
        assert_eq!(corpus.problems.len(), 2);
        assert!(corpus
            .problems
            .iter()
            .all(|(_, e)| e.code == FILE_TOO_LARGE));
    }
}
