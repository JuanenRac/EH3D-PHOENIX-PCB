// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `corpus`: run the own corpus of small hand-written pieces against the real readers
//! and writers, list it, or check that it is consistent.
//!
//! Exit codes: 0 every piece is as its expectation says (a refusal that was expected counts as
//! right), 1 a piece differs or failed, or the corpus is not consistent, 2 the arguments or the
//! folder are unusable. The command starts no other program and writes nothing unless `--out`
//! names a new file; an existing file is never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_corpus::suite::codes::{OPTION_INVALID, OUTPUT_FAILED, SETTINGS_INVALID};
use eh3d_phoenix_corpus::suite::{self, CorpusSettings, SuiteError};
use eh3d_phoenix_rules::Lang;

pub(crate) const USAGE: &str = "\
phoenix corpus run   [--dir FOLDER] [--filter TEXT] [--properties] [--settings FILE] [--limits FILE] [--out FILE] [--json] [--lang CODE]
phoenix corpus list  [--dir FOLDER] [--filter TEXT] [--settings FILE]
phoenix corpus check [--dir FOLDER] [--settings FILE] [--lang CODE]

  run           read every piece of the own corpus (boards, schematics and libraries of EAGLE and
                KiCad, Gerber and Excellon files, the board as JSON, the files of the autorouter,
                SPICE netlists) with the real readers and writers and compare with the expectation
                of the piece; the round trips between formats are pieces too. With --properties
                it also tries the properties (undo, rejected changes, idempotent writing, the
                rule check after a round trip) on boards made from seeds. Each piece is ok, a
                difference, an expected error (a file that must be refused and is) or failed
  list          one line per piece: id, kind, outcome and title
  check         the corpus without running it: expectations that are not valid, ids used twice,
                data files that are missing or that no expectation names
  --dir         the folder of the corpus (default tests/corpus)
  --filter      only the pieces whose id or kind contains the text
  --settings    a settings file (config/corpus.cfg) with the limits and the number of seeds
  --limits      the limits of the EAGLE readers and of the checks of a board (config/eagle_board.cfg
                and config/board.cfg) and the cut of the arcs (config/outline_arcs.cfg), as for
                every command; without it the defaults are used
  --out         write the JSON report to a new file (an existing file is never replaced)
  --lang CODE   the words and messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    properties: bool,
    lang: Lang,
    dir: String,
    filter: Option<String>,
    settings: Option<String>,
    out: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, SuiteError> {
    let mut options = Options {
        json: false,
        properties: false,
        lang: Lang::En,
        dir: "tests/corpus".to_owned(),
        filter: None,
        settings: None,
        out: None,
    };
    let bad = |what: &str| SuiteError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, SuiteError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--properties" => options.properties = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--dir" => options.dir = value("--dir needs a folder")?,
            "--filter" => options.filter = Some(value("--filter needs a text")?),
            "--settings" => options.settings = Some(value("--settings needs a file")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other => {
                return Err(SuiteError::new(
                    OPTION_INVALID,
                    format!("unknown option or argument {other}"),
                ))
            }
        }
    }
    Ok(options)
}

fn settings_of(options: &Options) -> Result<CorpusSettings, SuiteError> {
    let settings = match &options.settings {
        None => CorpusSettings::default(),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| SuiteError::new(SETTINGS_INVALID, format!("{path}: {e}")))?;
            CorpusSettings::parse(&text)?
        }
    };
    // the limits of `--limits` (the defaults without it) go to the readers and the checks
    let limits = crate::limits::active();
    Ok(CorpusSettings {
        board_limits: limits.board,
        eagle_limits: limits.eagle,
        ..settings
    })
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), SuiteError> {
    let failed = |e: std::io::Error| SuiteError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn run(options: &Options, out: &mut dyn Write) -> Result<i32, SuiteError> {
    let settings = settings_of(options)?;
    let corpus = suite::load(Path::new(&options.dir), &settings)?;
    let report = suite::run::run(
        &corpus,
        &settings,
        options.filter.as_deref(),
        options.properties,
    );
    if let Some(path) = &options.out {
        write_new(path, &report.to_json())?;
    }
    let text = if options.json {
        report.to_json()
    } else {
        let mut text = report.to_text(options.lang);
        if let Some(path) = &options.out {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.has_problems()))
}

fn list(options: &Options, out: &mut dyn Write) -> Result<i32, SuiteError> {
    let settings = settings_of(options)?;
    let corpus = suite::load(Path::new(&options.dir), &settings)?;
    let _ = out.write_all(suite::list(&corpus, options.filter.as_deref()).as_bytes());
    Ok(0)
}

fn check(options: &Options, out: &mut dyn Write) -> Result<i32, SuiteError> {
    let settings = settings_of(options)?;
    let corpus = suite::load(Path::new(&options.dir), &settings)?;
    let problems = suite::check(&corpus);
    let mut text = String::new();
    for p in &problems {
        text.push_str(&format!("{}\n", p.message(options.lang)));
    }
    text.push_str(&format!(
        "corpus: {} pieces, {} data files, {} problems\n",
        corpus.entries.len() + corpus.problems.len(),
        corpus.data_files.len(),
        problems.len()
    ));
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(!problems.is_empty()))
}

pub(crate) fn corpus_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&action, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if !matches!(action, "run" | "list" | "check") {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(tail) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Lang::En));
            return 2;
        }
    };
    let result = match action {
        "run" => run(&options, out),
        "list" => list(&options, out),
        _ => check(&options, out),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}
