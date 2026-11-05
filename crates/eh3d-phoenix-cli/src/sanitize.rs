// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `sanitize`: the analysis of a folder of EAGLE libraries for duplicates. It reads
//! and never changes the libraries; with `--out` it writes a report (HTML and JSON) and a plan
//! that is not applied.
//!
//! Exit codes: 0 done, 2 the arguments, the settings or the folder are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sanitizer::codes::{SanError, OPTION_INVALID, SETTINGS_INVALID};
use eh3d_phoenix_sanitizer::{sanitize, write_outputs, SanitizerSettings};

pub(crate) const USAGE: &str = "\
phoenix sanitize [--out BASE] [--threshold N] [--config FILE] [--lang xx] [--json] <folder>

  sanitize    read the .lbr libraries under <folder> and find the footprints, symbols and device
              sets that are duplicated or nearly duplicated: an inventory, a canonical geometric
              hash, a score against a threshold, groups to confirm by hand and the item that
              would be kept in each; nothing is rewritten and nothing is merged
  --out       write BASE.html (one page), BASE.json (the same report) and BASE.plan.json (the
              plan of the master library, which this program never applies)
  --threshold the lowest score, 0 to 100, at which two items are grouped (the settings give the
              default)
  --config    a settings file (config/sanitizer.cfg shows every key)
  --lang      the language of the page, one of en, es, de, fr, it, ja, zh, pt
  --json      print the report as JSON instead of the summary
";

/// The largest settings file that is read, in bytes.
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

struct Options {
    out: Option<String>,
    threshold: Option<u32>,
    config: Option<String>,
    lang: Lang,
    json: bool,
    folder: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, SanError> {
    let bad = |what: &str| SanError::new(OPTION_INVALID, what.to_owned());
    let mut options = Options {
        out: None,
        threshold: None,
        config: None,
        lang: Lang::En,
        json: false,
        folder: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--out" => {
                let value = iter.next().ok_or_else(|| bad("--out needs a path"))?;
                options.out = Some((*value).to_owned());
            }
            "--config" => {
                let value = iter.next().ok_or_else(|| bad("--config needs a file"))?;
                options.config = Some((*value).to_owned());
            }
            "--threshold" => {
                let value = iter
                    .next()
                    .ok_or_else(|| bad("--threshold needs a number"))?;
                let n: u32 = value
                    .parse()
                    .map_err(|_| bad("--threshold is not a whole number"))?;
                if n > 100 {
                    return Err(bad("--threshold must be between 0 and 100"));
                }
                options.threshold = Some(n);
            }
            "--lang" => {
                let value = iter.next().ok_or_else(|| bad("--lang needs a code"))?;
                options.lang = Lang::from_tag(value)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            folder => {
                if options.folder.is_some() {
                    return Err(bad("only one folder"));
                }
                options.folder = Some(folder.to_owned());
            }
        }
    }
    Ok(options)
}

fn read_settings(path: &str) -> Result<SanitizerSettings, SanError> {
    let invalid = |detail: String| SanError::new(SETTINGS_INVALID, detail);
    let length = std::fs::metadata(path)
        .map_err(|e| invalid(e.to_string()))?
        .len();
    if length > MAX_CONFIG_BYTES {
        return Err(invalid(format!(
            "the file is larger than {MAX_CONFIG_BYTES} bytes"
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
    SanitizerSettings::parse(&text)
}

fn run_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), (SanError, Lang)> {
    let options = parse_options(args).map_err(|e| (e, Lang::En))?;
    let lang = options.lang;
    let fail = |e: SanError| (e, lang);
    let Some(folder) = options.folder else {
        let _ = err.write_all(USAGE.as_bytes());
        return Err(fail(SanError::new(OPTION_INVALID, "a folder is needed")));
    };
    let mut settings = match &options.config {
        Some(path) => read_settings(path).map_err(fail)?,
        None => SanitizerSettings::default(),
    };
    if let Some(threshold) = options.threshold {
        settings.threshold = threshold;
    }
    let report = sanitize(Path::new(&folder), &settings).map_err(fail)?;
    if let Some(base) = &options.out {
        let paths = write_outputs(&report, Path::new(base), lang, &settings).map_err(fail)?;
        for p in &paths {
            let _ = writeln!(out, "written: {}", p.display());
        }
    }
    let text = if options.json {
        report.to_json()
    } else {
        report.to_text()
    };
    let _ = out.write_all(text.as_bytes());
    Ok(())
}

/// Runs the command with the arguments that follow its name. Returns the exit code.
pub(crate) fn sanitize_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match run_inner(args, out, err) {
        Ok(()) => 0,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}
