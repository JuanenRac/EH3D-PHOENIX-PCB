// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `lengths`: the copper length and the delay of the nets of a board, the groups of
//! nets that must match in length and the differential pairs.
//!
//! Exit codes: 0 done, 1 a rule of a group or a pair is proven broken, 2 the arguments or the
//! input are unusable. Nothing is written unless `--out` names a new file; an existing file is
//! never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_lengths::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_lengths::{
    analyse, report_json, report_text, LengthError, LengthFile, LengthSettings,
};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/lengths.cfg";

pub(crate) const USAGE: &str = "\
phoenix lengths [--config FILE] [--stackup FILE] [--groups FILE] [--out FILE] [--json]
                [--lang CODE] <board>

  lengths       the copper length and the delay of every net of a board (tracks and vias, the
                delay of each layer from the settings, from the stack-up or from the default),
                and, with --groups, the groups of nets that must match in length within a
                tolerance and the differential pairs with their separation, width and
                difference of length. Without --out nothing is written; with --out the JSON
                report is written to a new file (an existing file is never replaced)
  --config      the delays, tolerances and limits (default: config/lengths.cfg when it exists)
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the delay of each layer
                and the length of each via
  --groups      a JSON file (format phoenix-lengths/1) with the groups and the pairs
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    stackup: Option<String>,
    groups: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, LengthError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        stackup: None,
        groups: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| LengthError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, LengthError> {
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
            "--stackup" => options.stackup = Some(value("--stackup needs a file")?),
            "--groups" => options.groups = Some(value("--groups needs a file")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(LengthError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn unreadable(what: String) -> LengthError {
    LengthError::new(INPUT_UNREADABLE, what)
}

fn load_settings(path: Option<&str>) -> Result<LengthSettings, LengthError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(LengthSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            LengthSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &LengthError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), LengthError> {
    let failed = |e: std::io::Error| LengthError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn lengths(options: &Options, out: &mut dyn Write) -> Result<i32, LengthError> {
    let [board_path] = options.files.as_slice() else {
        return Err(LengthError::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let board = read_board_file(board_path).map_err(unreadable)?;
    let settings = load_settings(options.config.as_deref())?;
    let job = match &options.stackup {
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            Some(StackupJob::parse(&text).map_err(|e| unreadable(format!("{path}: {e}")))?)
        }
        None => None,
    };
    let file = match &options.groups {
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            Some(LengthFile::from_json(&text)?)
        }
        None => None,
    };
    let report = analyse(&board, &settings, job.as_ref(), file.as_ref())?;
    if let Some(path) = &options.out {
        write_new(path, &report_json(&report))?;
    }
    let text = if options.json {
        report_json(&report)
    } else {
        let mut text = report_text(&report, options.lang);
        if let Some(path) = &options.out {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.has_errors()))
}

pub(crate) fn lengths_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match lengths(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
