// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `testpoints`: the pads of the parts that are test points, the nets that no test
//! point reaches and the test points that are too small, too close to each other or too close to
//! the edge of the board.
//!
//! Exit codes: 0 done, 1 a finding of the report is an error, 2 the arguments or the input are
//! unusable. Nothing is written unless `--out` names a new file: the report goes to the output stream,
//! and an existing file is never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_testpoints::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_testpoints::{
    analyse, report_json, report_text, TestPointError, TestPointSettings,
};

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/testpoints.cfg";

pub(crate) const USAGE: &str =
    "phoenix testpoints [--config FILE] [--out FILE] [--json] [--lang CODE] <board>

  testpoints    the test points of a board: the pads of the parts whose reference starts with TP
                or whose footprint is named as a test point, the nets that have none on the side
                that is probed (and the nets whose test points are all on the other side), and
                the test points that are smaller than the probe touches, closer to each other
                than the probes reach or too near the edge, with the codes PHOENIX-TP-<NNN>.
                A dry run unless --out names a new file, which gets the JSON report (an
                existing file is never replaced); a prototype
  --config      what counts as a test point, the side probed and the limits (default:
                config/testpoints.cfg when it exists)
  --out FILE    write the JSON report to a new file
  --json        print the report as JSON, the same every time
  --lang CODE   the titles in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, TestPointError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| TestPointError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, TestPointError> {
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
            "--out" => options.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(TestPointError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<TestPointSettings, TestPointError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(TestPointSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| TestPointError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            TestPointSettings::parse(&text)
        }
    }
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), TestPointError> {
    let failed = |e: std::io::Error| TestPointError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn fail(err: &mut dyn Write, error: &TestPointError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

fn testpoints(options: &Options, out: &mut dyn Write) -> Result<i32, TestPointError> {
    let [board_path] = options.files.as_slice() else {
        return Err(TestPointError::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let board =
        read_board_file(board_path).map_err(|e| TestPointError::new(INPUT_UNREADABLE, e))?;
    let settings = load_settings(options.config.as_deref())?;
    let report = analyse(&board, &settings)?;
    if let Some(path) = &options.out {
        write_new(path, &report_json(&report))?;
    }
    let mut text = if options.json {
        report_json(&report)
    } else {
        report_text(&report, options.lang)
    };
    if let (Some(path), false) = (&options.out, options.json) {
        text.push_str(&format!("written: {path}\n"));
    }
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.has_errors()))
}

pub(crate) fn testpoints_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match testpoints(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
