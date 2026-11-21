// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ulp`: the reader and the compatibility report of the user programs of the
//! reference tool. `phoenix ulp check` reads programs and lists what is wrong with their shape;
//! `phoenix ulp report` says what they use and what the list of allowed names covers. Nothing of a
//! program is run and nothing is written.
//!
//! Exit codes of `check`: 0 every program was read without a problem, 1 some program has a
//! problem, 2 the arguments or a file are unusable. Exit codes of `report`: 0 every program is
//! covered, 1 some program is partly or not covered, 2 as above.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_ulp::codes::{INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_ulp::{
    analyse, parse, read_text_limited, report_json, report_text, Diag, Support, UlpSettings,
    Verdict,
};
use serde_json::json;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/ulp.cfg";

/// The most programs one command reads.
const MAX_FILES: usize = 10_000;

pub(crate) const USAGE: &str =
    "phoenix ulp check|report [--config FILE] [--support FILE] [--json] [--lang CODE] <program>...

  ulp check     read the programs and list what is wrong with their shape, with the line and
                the column and the codes PHOENIX-ULP-<NNN>: texts and comments that are not
                closed, numbers that are not valid, a mark that is missing, blocks nested too
                deep; exit code 1 when a program has a problem
  ulp report    what each program uses (directives, kinds of statement, types, functions,
                members and named values) and what the list of allowed names says about each,
                with a verdict: covered, partly covered or not covered; exit code 1 unless every
                program is covered. Nothing is run: the evaluator is not written yet, so the
                verdict says what the list would let through
  --config      the limits of the reader (default: config/ulp.cfg when it exists)
  --support     the list of allowed names (default: the list that ships with the program,
                config/ulp_support.cfg)
  --json        print the result as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
  A prototype; it reads files and writes nothing.
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Check,
    Report,
}

struct Options {
    mode: Mode,
    json: bool,
    lang: Lang,
    config: Option<String>,
    support: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, Diag> {
    let bad = |what: &str| Diag::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    let mode = match iter.next() {
        Some(&"check") => Mode::Check,
        Some(&"report") => Mode::Report,
        _ => return Err(bad("the first word is `check` or `report`")),
    };
    let mut options = Options {
        mode,
        json: false,
        lang: Lang::En,
        config: None,
        support: None,
        files: Vec::new(),
    };
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, Diag> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => options.config = Some(value("--config needs a file")?),
            "--support" => options.support = Some(value("--support needs a file")?),
            other if other.starts_with("--") => {
                return Err(Diag::new(OPTION_INVALID, format!("unknown option {other}")))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    if options.files.is_empty() {
        return Err(bad("give at least one program"));
    }
    if options.files.len() > MAX_FILES {
        return Err(bad("too many programs for one command"));
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<UlpSettings, Diag> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(UlpSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| Diag::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            UlpSettings::parse(&text)
        }
    }
}

fn load_support(path: Option<&str>) -> Result<Support, Diag> {
    match path {
        None => Ok(Support::shipped()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| Diag::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            Support::parse(&text).map_err(|mut d| {
                d.detail = format!("{path}: {}", d.detail);
                d
            })
        }
    }
}

fn fail(err: &mut dyn Write, error: &Diag, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// One program read by `check`: the lines of text and the JSON, and whether it is clean.
fn check_one(
    path: &str,
    source: &str,
    settings: &UlpSettings,
    lang: Lang,
) -> (String, serde_json::Value, bool) {
    let diag_json = |d: &Diag| {
        json!({
            "code": d.code,
            "line": d.line,
            "col": d.col,
            "detail": d.detail,
        })
    };
    match parse(source, settings) {
        Ok(parsed) => {
            let p = &parsed.program;
            let clean = parsed.problems.total() == 0;
            let mut text = format!(
                "{path}: {} statements, {} functions, {} directives, {} problems\n",
                p.statement_count(),
                p.functions.len(),
                p.directives.len(),
                parsed.problems.total()
            );
            for d in &parsed.problems.list {
                text.push_str(&format!("  {}\n", d.message(lang)));
            }
            if parsed.problems.dropped > 0 {
                text.push_str(&format!(
                    "  {} (+{})\n",
                    Diag::new(eh3d_phoenix_ulp::codes::LIST_CUT, "").message(lang),
                    parsed.problems.dropped
                ));
            }
            let value = json!({
                "program": path,
                "statements": p.statement_count(),
                "functions": p.functions.len(),
                "directives": p.directives.len(),
                "problems": parsed.problems.list.iter().map(diag_json).collect::<Vec<_>>(),
                "problems_dropped": parsed.problems.dropped,
                "stopped": serde_json::Value::Null,
            });
            (text, value, clean)
        }
        Err(stopped) => {
            let text = format!("{path}: {}\n", stopped.message(lang));
            let value = json!({
                "program": path,
                "problems": [],
                "problems_dropped": 0,
                "stopped": diag_json(&stopped),
            });
            (text, value, false)
        }
    }
}

fn run(options: &Options, out: &mut dyn Write) -> Result<i32, Diag> {
    let settings = load_settings(options.config.as_deref())?;
    let support = load_support(options.support.as_deref())?;
    let mut sources = Vec::new();
    for path in &options.files {
        let source = read_text_limited(Path::new(path), settings.max_source_bytes)?;
        sources.push((path.as_str(), source));
    }
    let mut all_good = true;
    let mut text = String::new();
    let mut values = Vec::new();
    for (path, source) in &sources {
        match options.mode {
            Mode::Check => {
                let (t, v, clean) = check_one(path, source, &settings, options.lang);
                all_good &= clean;
                text.push_str(&t);
                values.push(v);
            }
            Mode::Report => {
                let report = analyse(path, source, &settings, &support);
                all_good &= report.verdict == Verdict::Covered;
                if options.json {
                    values.push(serde_json::from_str(&report_json(&report)).unwrap_or_default());
                } else {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&report_text(&report, options.lang));
                }
            }
        }
    }
    if options.json {
        let format = match options.mode {
            Mode::Check => "phoenix-ulp-check/1",
            Mode::Report => "phoenix-ulp-reports/1",
        };
        let value = json!({ "format": format, "programs": values });
        text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
    }
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(!all_good))
}

pub(crate) fn ulp_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    match run(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
