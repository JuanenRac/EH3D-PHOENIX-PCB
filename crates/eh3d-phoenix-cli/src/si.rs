// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `si`: signal integrity estimates of a board (crosstalk between parallel tracks,
//! terminations and reflections, the eye diagram and the return path).
//!
//! Exit codes: 0 done, 1 a threshold of the settings is exceeded by an error finding, 2 the
//! arguments or the input are unusable. Nothing is written unless `--out` names a new file; an
//! existing file is never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_si::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_si::{analyse, report_json, report_text, Analysis, SiError, SiSettings};
use eh3d_phoenix_stackup::StackupJob;

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/si.cfg";

pub(crate) const USAGE: &str = "\
phoenix si <crosstalk|reflect|eye|check> [--stackup FILE] [--net NAME] [--config FILE]
           [--out FILE] [--json] [--lang CODE] <board>

  crosstalk     the near-end and far-end crosstalk, in percent of the step of the aggressor, of every
                pair of nets that runs side by side on a layer, from the length that runs parallel,
                the distance between centres and the line under it, with the victims of each
                aggressor ranked and the sum on each victim
  reflect       for each net: the impedance, the delay, the criterion of a long line (tr < 2 Td),
                the diagram of bounces by the method of Bergeron, the overshoot, the ringback and
                the settling with the driver and the load of the settings, proposals of a series,
                parallel, Thevenin and RC termination with the result of each, and the loss and the
                slowing of the rise time along the line
  eye           a simplified eye diagram of each net: a PRBS7 or PRBS9 sequence through the rise time
                and a low-pass filter made from the loss of the line; height, width and jitter
  check         all of them and the return path: tracks that cross the edge or a split of the
                copper next to them and via stubs; the exit code is 1 when a threshold is exceeded
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the impedance, delay, loss,
                height and permittivity of each layer (without it the defaults of the settings)
  --net         only this net (the aggressor in crosstalk, the line in reflect and eye)
  --config      thresholds, driver, load and limits (default: config/si.cfg when it exists)
  --out         write the JSON report to a new file (an existing file is never replaced)
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt

A prototype: quasi-static estimates from closed formulas, tried on boards made by the tests only
and not compared with a field solver, a circuit simulator or a measurement. A dry run: nothing is
written without --out.
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    stackup: Option<String>,
    net: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, SiError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        stackup: None,
        net: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| SiError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, SiError> {
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
            "--net" => options.net = Some(value("--net needs a net name")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(SiError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn unreadable(what: String) -> SiError {
    SiError::new(INPUT_UNREADABLE, what)
}

fn load_settings(path: Option<&str>) -> Result<SiSettings, SiError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(SiSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            SiSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &SiError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), SiError> {
    let failed = |e: std::io::Error| SiError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn run_analysis(what: Analysis, options: &Options, out: &mut dyn Write) -> Result<i32, SiError> {
    let [board_path] = options.files.as_slice() else {
        return Err(SiError::new(OPTION_INVALID, "give exactly one board file"));
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
    let report = analyse(
        &board,
        &settings,
        job.as_ref(),
        what,
        options.net.as_deref(),
    )?;
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

pub(crate) fn si_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&name, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let what = match name {
        "crosstalk" => Analysis::Crosstalk,
        "reflect" => Analysis::Reflect,
        "eye" => Analysis::Eye,
        "check" => Analysis::Check,
        _ => {
            let _ = err.write_all(USAGE.as_bytes());
            let error = SiError::new(OPTION_INVALID, format!("unknown subcommand {name}"));
            return fail(err, &error, Lang::En);
        }
    };
    let options = match parse_options(tail) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    match run_analysis(what, &options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
