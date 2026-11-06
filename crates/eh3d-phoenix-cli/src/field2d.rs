// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `field2d`: the impedance, effective permittivity, inductance and coupling of the
//! cross section of a line by a two-dimensional field solver, set beside the closed formulas of
//! `stackup`.
//!
//! Exit codes: 0 done, 1 a line could not be solved, 2 the arguments or the input are unusable.
//! Nothing is written unless `--out` names a new file; an existing file is never replaced.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_field2d::codes::{
    FieldError, INPUT_UNREADABLE, LINE_UNAVAILABLE, NET_NOT_FOUND, OPTION_INVALID, OUTPUT_FAILED,
};
use eh3d_phoenix_field2d::{
    report_json, report_text, report_value, run_job, solve_line, CaseOutcome, Field2dSettings,
};
use eh3d_phoenix_stackup::{Language, LineKind, LineSpec, Stackup, StackupJob, StackupSettings};
use serde_json::Value;

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/field2d.cfg";

pub(crate) const USAGE: &str = "\
phoenix field2d <config|board> [--stackup FILE] [--net NAME] [--settings FILE]
                [--freq-ghz GHZ] [--refine N] [--out FILE] [--json] [--lang CODE]

  config        a job file of `phoenix stackup` (a stack-up, its materials and the lines): every line
                is solved by the field solver and set beside the closed formula
  board         a board, with --stackup FILE: the layer and the width most of the copper of each net
                (or of --net NAME) runs on give the line to solve
  --stackup     the job file that gives the stack-up and the materials of a board
  --net         only this net of a board
  --settings    the solver, its limits and the fineness of the mesh (default: config/field2d.cfg
                when it exists)
  --freq-ghz    the frequency at which the materials are taken (default: the settings)
  --refine N    divide every step of the mesh by N (a convergence study)
  --out         write the JSON report to a new file (an existing file is never replaced)
  --lang CODE   the titles of the codes in en, es, de, fr, it, ja, zh or pt

Per line: the impedance, effective permittivity, inductance, capacitance, delay and dielectric loss,
and for a pair the differential and common impedance, the even and odd modes and the mutual
capacitance and inductance, with the difference in percent from the closed formula. A prototype: a
rectangular mesh and a grounded box in place of the open region, checked against exact solutions
and the closed formulas and never against another field solver or a measurement. A dry run:
nothing is written without --out.
";

struct Options {
    json: bool,
    lang: Language,
    settings: Option<String>,
    stackup: Option<String>,
    net: Option<String>,
    out: Option<String>,
    freq_ghz: Option<f64>,
    refine: Option<f64>,
    files: Vec<String>,
}

fn bad(what: impl Into<String>) -> FieldError {
    FieldError::new(OPTION_INVALID, what)
}

fn number(text: &str, what: &str) -> Result<f64, FieldError> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad(format!("{what} needs a number")))
}

fn parse_options(args: &[&str]) -> Result<Options, FieldError> {
    let mut options = Options {
        json: false,
        lang: Language::En,
        settings: None,
        stackup: None,
        net: None,
        out: None,
        freq_ghz: None,
        refine: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, FieldError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang =
                    Language::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--settings" => options.settings = Some(value("--settings needs a file")?),
            "--stackup" => options.stackup = Some(value("--stackup needs a file")?),
            "--net" => options.net = Some(value("--net needs a net name")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            "--freq-ghz" => {
                let text = value("--freq-ghz needs a number")?;
                options.freq_ghz = Some(number(&text, "--freq-ghz")?);
            }
            "--refine" => {
                let text = value("--refine needs a number")?;
                options.refine = Some(number(&text, "--refine")?);
            }
            other if other.starts_with("--") => {
                return Err(bad(format!("unknown option {other}")));
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn unreadable(what: String) -> FieldError {
    FieldError::new(INPUT_UNREADABLE, what)
}

fn load_settings(options: &Options) -> Result<Field2dSettings, FieldError> {
    let path = options.settings.as_deref().or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    let mut settings = match path {
        None => Field2dSettings::default(),
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            Field2dSettings::parse(&text)?
        }
    };
    if let Some(refine) = options.refine {
        settings.refine = refine;
    }
    if let Some(freq) = options.freq_ghz {
        settings.freq_ghz = freq;
    }
    settings.validate()?;
    Ok(settings)
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), FieldError> {
    let failed = |e: std::io::Error| FieldError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

/// A net and the layer and width most of its copper has.
struct NetRun {
    net: String,
    layer: String,
    width_nm: i64,
}

fn chord(start: (i64, i64), end: (i64, i64)) -> f64 {
    let dx = (start.0 - end.0) as f64;
    let dy = (start.1 - end.1) as f64;
    dx.hypot(dy)
}

fn net_runs(board: &Board, only: Option<&str>) -> Result<Vec<NetRun>, FieldError> {
    let mut lengths: BTreeMap<(Id, Id, i64), f64> = BTreeMap::new();
    for track in board.tracks.values() {
        let mut sum = 0.0;
        for segment in &track.path {
            let (a, b) = (segment.start(), segment.end());
            sum += chord((a.x_nm.get(), a.y_nm.get()), (b.x_nm.get(), b.y_nm.get()));
        }
        *lengths
            .entry((track.net, track.layer, track.width.get()))
            .or_insert(0.0) += sum;
    }
    let mut mostly: BTreeMap<Id, (Id, i64, f64)> = BTreeMap::new();
    for ((net, layer, width), length) in &lengths {
        let entry = mostly.entry(*net).or_insert((*layer, *width, *length));
        if *length > entry.2 {
            *entry = (*layer, *width, *length);
        }
    }
    let mut runs: Vec<NetRun> = Vec::new();
    for (net, (layer, width, _)) in &mostly {
        let name = board
            .nets
            .get(net)
            .map_or_else(|| net.to_string(), |n| n.name.clone());
        if only.is_some_and(|wanted| wanted != name) {
            continue;
        }
        let layer_name = board
            .layers
            .get(layer)
            .map_or_else(|| layer.to_string(), |l| l.name.clone());
        runs.push(NetRun {
            net: name,
            layer: layer_name,
            width_nm: *width,
        });
    }
    runs.sort_by(|a, b| a.net.cmp(&b.net));
    if let Some(wanted) = only {
        if runs.is_empty() {
            return Err(FieldError::new(
                NET_NOT_FOUND,
                format!("the net {wanted} has no track on the board"),
            ));
        }
    }
    Ok(runs)
}

/// A line on a layer is a stripline between two copper layers and a microstrip on an outer one.
fn kind_for(stackup: &Stackup, layer: &str) -> Option<LineKind> {
    let index = stackup.find(layer)?;
    let above = stackup.layers.iter().take(index).any(|l| l.is_copper());
    let below = stackup.layers.iter().skip(index + 1).any(|l| l.is_copper());
    Some(if above && below {
        LineKind::Stripline
    } else {
        LineKind::Microstrip
    })
}

fn board_outcomes(
    board: &Board,
    job: &StackupJob,
    options: &Options,
    freq_ghz: f64,
    settings: &Field2dSettings,
) -> Result<Vec<CaseOutcome>, FieldError> {
    let runs = net_runs(board, options.net.as_deref())?;
    let stackup_settings = StackupSettings::default();
    let mut outcomes = Vec::new();
    for run in runs.iter().take(settings.max_cases) {
        let result = match kind_for(&job.stackup, &run.layer) {
            None => Err(FieldError::new(
                LINE_UNAVAILABLE,
                format!("the stack-up has no layer named {}", run.layer),
            )),
            Some(kind) => {
                let spec = LineSpec {
                    name: run.net.clone(),
                    kind,
                    layer: run.layer.clone(),
                    width_nm: Some(run.width_nm),
                    target_ohm: None,
                    gap_nm: None,
                };
                solve_line(
                    &job.stackup,
                    &job.materials,
                    &spec,
                    run.width_nm,
                    freq_ghz,
                    false,
                    settings,
                    &stackup_settings,
                )
            }
        };
        outcomes.push(CaseOutcome {
            name: run.net.clone(),
            result,
        });
    }
    Ok(outcomes)
}

fn run_field(options: &Options, out: &mut dyn Write) -> Result<i32, FieldError> {
    let [path] = options.files.as_slice() else {
        return Err(bad("give exactly one file"));
    };
    let settings = load_settings(options)?;
    let freq_ghz = settings.freq_ghz;
    let text = read_text(path).map_err(unreadable)?;
    let is_job = serde_json::from_str::<Value>(&text)
        .ok()
        .is_some_and(|v| v.get("stackup").is_some() && v.get("lines").is_some());
    let (name, outcomes) = if is_job {
        if options.stackup.is_some() || options.net.is_some() {
            return Err(bad(
                "--stackup and --net are for a board, not for a job file",
            ));
        }
        let job = StackupJob::parse(&text).map_err(|e| unreadable(format!("{path}: {e}")))?;
        let outcomes = run_job(&job, freq_ghz, &settings, &StackupSettings::default());
        (job.stackup.name.clone(), outcomes)
    } else {
        let Some(stackup_path) = &options.stackup else {
            return Err(bad("a board needs --stackup FILE"));
        };
        let job_text = read_text(stackup_path).map_err(unreadable)?;
        let job =
            StackupJob::parse(&job_text).map_err(|e| unreadable(format!("{stackup_path}: {e}")))?;
        let board = read_board_file(path).map_err(unreadable)?;
        let outcomes = board_outcomes(&board, &job, options, freq_ghz, &settings)?;
        (job.stackup.name.clone(), outcomes)
    };
    let json = report_json(&name, freq_ghz, &outcomes, &settings);
    if let Some(target) = &options.out {
        write_new(target, &json)?;
    }
    let shown = if options.json {
        json
    } else {
        let mut shown = report_text(&name, freq_ghz, &outcomes, &settings, options.lang);
        if let Some(target) = &options.out {
            shown.push_str(&format!("written: {target}\n"));
        }
        shown
    };
    let _ = out.write_all(shown.as_bytes());
    Ok(i32::from(outcomes.iter().any(|o| o.result.is_err())))
}

/// The field solver run on the lines of a job of `phoenix stackup`, as a text to append to its
/// report and as a JSON value to add to it (`phoenix stackup --field2d`). The settings are those of
/// `config/field2d.cfg` when it exists, else the defaults.
pub(crate) fn appendix(
    job: &StackupJob,
    freq_ghz: f64,
    stackup_settings: &StackupSettings,
    language: Language,
) -> Result<(String, Value), FieldError> {
    let mut settings = match Path::new(DEFAULT_SETTINGS).is_file() {
        true => Field2dSettings::parse(&read_text(DEFAULT_SETTINGS).map_err(unreadable)?)?,
        false => Field2dSettings::default(),
    };
    settings.freq_ghz = freq_ghz;
    let outcomes = run_job(job, freq_ghz, &settings, stackup_settings);
    let name = job.stackup.name.as_str();
    let text = report_text(name, freq_ghz, &outcomes, &settings, language);
    let value = report_value(name, freq_ghz, &outcomes, &settings);
    Ok((text, value))
}

pub(crate) fn field2d_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Language::En));
            return 2;
        }
    };
    match run_field(&options, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}
