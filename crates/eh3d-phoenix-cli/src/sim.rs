// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `sim`: circuit simulation with the solver of our own. It reads a SPICE-style
//! netlist, or the schematic of a project and translates it, and runs one analysis: the
//! operating point, a DC sweep, an AC sweep or a transient (named on the command line, or else
//! the first one the netlist asks for with `.op`, `.dc`, `.ac` or `.tran`). A file of models can
//! be read with it (`--model`). It can also print the netlist of a schematic, and check a ladder
//! filter of the RF tools against its simulation. Nothing is written unless `--out` is given.
//!
//! Exit codes: 0 done, 1 the simulation disagrees with the RF filter (`--check-filter`), 2 the
//! arguments, the settings or the input are unusable or the circuit cannot be solved.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rf::filter::{design, prototype, Approximation, Band, Position};
use eh3d_phoenix_rf::RfSettings;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sim::codes::{SimError, INPUT_UNREADABLE, OPTION_INVALID, SETTINGS_INVALID};
use eh3d_phoenix_sim::filter::check_frequencies;
use eh3d_phoenix_sim::output::{
    ac_csv, ac_json, dc_csv, dc_json, op_csv, op_json, select, tran_csv, tran_json, write_outputs,
};
use eh3d_phoenix_sim::units::parse_number;
use eh3d_phoenix_sim::{
    ac_analysis, compare_filter, dc_sweep, netlist_from_schematic_json, operating_point,
    read_netlist_file, to_spice, transient, AcRequest, Circuit, DcRequest, Directive, Method,
    Netlist, SimSettings, SweepKind, TranRequest,
};

pub(crate) const USAGE: &str = "\
phoenix sim [--config FILE] [--lang xx] [--json] [--out BASE] [--probe LIST] [--model FILE] <input> [<analysis>]
phoenix sim [--config FILE] [--lang xx] --check-filter FAMILY ORDER BAND F1 [F2] [--ripple-db DB] [--z0 OHM]

  sim         simulate a circuit with the solver of this program (modified nodal analysis; a
              prototype, never compared with SPICE). <input> is a SPICE-style netlist, or the
              schematic of a project (a .json file), which is translated first and reported on
  --op                    the DC operating point (Newton-Raphson for the non-linear devices; the
                          iterations, the strategy and the residual are reported in the JSON)
  --dc SRC START STOP STEP
                          sweep an independent source and solve the operating point at each value
                          (a negative STEP sweeps downwards)
  --ac KIND N F1 F2       an AC sweep; KIND is lin (N points), dec (N per decade) or oct (N per
                          octave); the magnitude, dB, phase and group delay are written
  --tran STEP STOP        a transient analysis; add --uic to start from the initial conditions,
                          --adaptive for an adaptive step (STEP is then the largest) and
                          --method be|trap (trapezoidal by default)
  --netlist               print the netlist (useful for a schematic) and stop
  (no analysis)           the first .op, .dc, .ac or .tran line of the netlist is run
  --model FILE            read a file of .model and .subckt lines before the netlist; it is only
                          read, never changed. An .include in a netlist is allowed only for a file
                          in the same folder
  --probe LIST            the unknowns to write, such as v(out),i(v1); all by default
  --out BASE              write BASE.csv and BASE.json instead of printing
  --json                  print JSON instead of CSV
  --check-filter ...      simulate a ladder filter of the RF tools and compare its magnitude,
                          phase and group delay with the RF crate (FAMILY is butterworth,
                          chebyshev or bessel; BAND is lowpass, highpass, bandpass or bandstop;
                          F1 and F2 are the edges in Hz; chebyshev needs --ripple-db)
  --config FILE           a settings file (config/sim.cfg shows every key)
  --lang                  one of en, es, de, fr, it, ja, zh, pt
  numbers take the suffixes of SPICE: 1k, 10meg, 4.7u
";

/// The largest settings file that is read, in bytes.
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

enum Analysis {
    Op,
    Dc(String, f64, f64, f64),
    Ac(SweepKind, usize, f64, f64),
    Tran(f64, f64, bool),
    Netlist,
    CheckFilter(Vec<String>),
}

struct Options {
    lang: Lang,
    config: Option<String>,
    json: bool,
    out: Option<String>,
    probes: Vec<String>,
    input: Option<String>,
    model: Option<String>,
    analysis: Option<Analysis>,
    uic: bool,
    adaptive: bool,
    method: Method,
    ripple_db: Option<f64>,
    z0: f64,
}

fn bad(why: impl Into<String>) -> SimError {
    SimError::new(OPTION_INVALID, why)
}

fn number(text: &str, what: &str) -> Result<f64, SimError> {
    parse_number(text).ok_or_else(|| bad(format!("{what} is not a number: {text}")))
}

fn parse_options(args: &[&str]) -> Result<Options, SimError> {
    let mut options = Options {
        lang: Lang::En,
        config: None,
        json: false,
        out: None,
        probes: Vec::new(),
        input: None,
        model: None,
        analysis: None,
        uic: false,
        adaptive: false,
        method: Method::Trapezoidal,
        ripple_db: None,
        z0: 50.0,
    };
    let mut iter = args.iter();
    let take = |what: &str, iter: &mut std::slice::Iter<'_, &str>| -> Result<String, SimError> {
        iter.next()
            .map(|s| (*s).to_string())
            .ok_or_else(|| bad(format!("{what} needs a value")))
    };
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--uic" => options.uic = true,
            "--adaptive" => options.adaptive = true,
            "--op" => options.analysis = Some(Analysis::Op),
            "--netlist" => options.analysis = Some(Analysis::Netlist),
            "--config" => options.config = Some(take("--config", &mut iter)?),
            "--model" => options.model = Some(take("--model", &mut iter)?),
            "--dc" => {
                let source = take("--dc", &mut iter)?;
                let start = number(&take("--dc", &mut iter)?, "the start value")?;
                let stop = number(&take("--dc", &mut iter)?, "the stop value")?;
                let step = number(&take("--dc", &mut iter)?, "the step")?;
                options.analysis = Some(Analysis::Dc(source, start, stop, step));
            }
            "--out" => options.out = Some(take("--out", &mut iter)?),
            "--lang" => {
                let value = take("--lang", &mut iter)?;
                options.lang = Lang::from_tag(&value)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            "--probe" => {
                let value = take("--probe", &mut iter)?;
                options.probes = value
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
            }
            "--method" => {
                options.method = match take("--method", &mut iter)?.as_str() {
                    "be" => Method::BackwardEuler,
                    "trap" => Method::Trapezoidal,
                    _ => return Err(bad("--method is be or trap")),
                };
            }
            "--ac" => {
                let kind = match take("--ac", &mut iter)?.as_str() {
                    "lin" => SweepKind::Linear,
                    "dec" => SweepKind::Decade,
                    "oct" => SweepKind::Octave,
                    _ => return Err(bad("--ac needs lin, dec or oct first")),
                };
                let points = number(&take("--ac", &mut iter)?, "the number of points")?;
                if points < 1.0 || points.fract() != 0.0 || points > 1e9 {
                    return Err(bad("the number of points is not a whole number above zero"));
                }
                let start = number(&take("--ac", &mut iter)?, "the start frequency")?;
                let stop = number(&take("--ac", &mut iter)?, "the stop frequency")?;
                options.analysis = Some(Analysis::Ac(kind, points as usize, start, stop));
            }
            "--tran" => {
                let step = number(&take("--tran", &mut iter)?, "the time step")?;
                let stop = number(&take("--tran", &mut iter)?, "the stop time")?;
                options.analysis = Some(Analysis::Tran(step, stop, false));
            }
            "--check-filter" => {
                // FAMILY ORDER BAND F1 and, for the two-edge bands, F2
                let mut words = Vec::new();
                for _ in 0..4 {
                    words.push(take("--check-filter", &mut iter)?);
                }
                if words[2] == "bandpass" || words[2] == "bandstop" {
                    words.push(take("--check-filter", &mut iter)?);
                }
                options.analysis = Some(Analysis::CheckFilter(words));
            }
            "--ripple-db" => {
                options.ripple_db = Some(number(&take("--ripple-db", &mut iter)?, "the ripple")?)
            }
            "--z0" => options.z0 = number(&take("--z0", &mut iter)?, "the source impedance")?,
            other if other.starts_with("--") => {
                return Err(bad(format!("unknown option {other}")));
            }
            input => {
                if options.input.is_some() {
                    return Err(bad("only one input"));
                }
                options.input = Some(input.to_string());
            }
        }
    }
    Ok(options)
}

fn read_settings(path: &str) -> Result<SimSettings, SimError> {
    let invalid = |detail: String| SimError::new(SETTINGS_INVALID, detail);
    let length = std::fs::metadata(path)
        .map_err(|e| invalid(e.to_string()))?
        .len();
    if length > MAX_CONFIG_BYTES {
        return Err(invalid(format!(
            "the file is larger than {MAX_CONFIG_BYTES} bytes"
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
    SimSettings::parse(&text)
}

/// Reads the input as a netlist, translating a schematic when it is a `.json` file. The report
/// of a translation is written to `err`.
fn load(
    path: &str,
    model: Option<&str>,
    settings: &SimSettings,
    lang: Lang,
    err: &mut dyn Write,
) -> Result<Netlist, SimError> {
    let unreadable = |why: String| SimError::new(INPUT_UNREADABLE, format!("{path}: {why}"));
    let length = std::fs::metadata(path)
        .map_err(|e| unreadable(e.to_string()))?
        .len();
    if length > settings.max_file_bytes {
        return Err(unreadable(format!(
            "the file is larger than max_file_bytes ({})",
            settings.max_file_bytes
        )));
    }
    let is_json = Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if is_json {
        if model.is_some() {
            return Err(bad(
                "--model applies to a netlist file; a schematic has no place for a file of models",
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| unreadable(e.to_string()))?;
        let (netlist, report) = netlist_from_schematic_json(&bytes, settings)?;
        let _ = err.write_all(report.to_text(lang).as_bytes());
        Ok(netlist)
    } else {
        read_netlist_file(Path::new(path), model.map(Path::new), settings)
    }
}

/// The analysis a directive of the netlist asks for.
fn from_directive(d: &Directive) -> Analysis {
    match d {
        Directive::Op => Analysis::Op,
        Directive::Dc {
            source,
            start,
            stop,
            step,
        } => Analysis::Dc(source.clone(), *start, *stop, *step),
        Directive::Tran { tstep, tstop, uic } => Analysis::Tran(*tstep, *tstop, *uic),
        Directive::Ac {
            kind,
            points,
            start_hz,
            stop_hz,
        } => Analysis::Ac(*kind, *points, *start_hz, *stop_hz),
    }
}

fn emit(
    options: &Options,
    settings: &SimSettings,
    csv: String,
    json: String,
    out: &mut dyn Write,
) -> Result<(), SimError> {
    if let Some(base) = &options.out {
        for path in write_outputs(Path::new(base), &csv, &json, settings)? {
            let _ = writeln!(out, "written: {}", path.display());
        }
    } else if options.json {
        let _ = out.write_all(json.as_bytes());
    } else {
        let _ = out.write_all(csv.as_bytes());
    }
    Ok(())
}

fn check_filter(
    words: &[String],
    options: &Options,
    settings: &SimSettings,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, SimError> {
    let rf = RfSettings::default();
    let approximation = match words[0].as_str() {
        "butterworth" => Approximation::Butterworth,
        "chebyshev" => Approximation::Chebyshev,
        "bessel" => Approximation::Bessel,
        other => return Err(bad(format!("unknown family {other}"))),
    };
    let order = number(&words[1], "the order")?;
    if order < 1.0 || order.fract() != 0.0 || order > 1e6 {
        return Err(bad("the order is not a whole number above zero"));
    }
    let f1 = number(&words[3], "the first edge")?;
    let band = match words[2].as_str() {
        "lowpass" => Band::LowPass { cutoff_hz: f1 },
        "highpass" => Band::HighPass { cutoff_hz: f1 },
        "bandpass" | "bandstop" => {
            let f2 = number(
                words
                    .get(4)
                    .ok_or_else(|| bad("this band needs two edges"))?,
                "the second edge",
            )?;
            if words[2] == "bandpass" {
                Band::BandPass {
                    low_hz: f1,
                    high_hz: f2,
                }
            } else {
                Band::BandStop {
                    low_hz: f1,
                    high_hz: f2,
                }
            }
        }
        other => return Err(bad(format!("unknown band {other}"))),
    };
    if approximation == Approximation::Chebyshev && options.ripple_db.is_none() {
        return Err(bad("a chebyshev filter needs --ripple-db"));
    }
    let ripple = if approximation == Approximation::Chebyshev {
        options.ripple_db
    } else {
        None
    };
    let rf_error = |e: eh3d_phoenix_rf::RfError| bad(e.to_string());
    let proto =
        prototype(approximation, order as usize, ripple, Position::Shunt, &rf).map_err(rf_error)?;
    let filter = design(&proto, band, options.z0, &rf).map_err(rf_error)?;
    let freqs = check_frequencies(&filter, settings);
    let coherence = compare_filter(&filter, &freqs, settings)?;
    let _ = out.write_all(coherence.to_text().as_bytes());
    match coherence.verify(settings) {
        Ok(()) => Ok(0),
        Err(e) => {
            let _ = writeln!(err, "error: {}", e.message(options.lang));
            Ok(1)
        }
    }
}

fn run_inner(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> Result<i32, SimError> {
    let options = parse_options(args)?;
    let settings = match &options.config {
        Some(path) => read_settings(path)?,
        None => SimSettings::default(),
    };
    if options.analysis.is_none() && options.input.is_none() {
        let _ = err.write_all(USAGE.as_bytes());
        return Err(bad(
            "an analysis is needed: --op, --dc, --ac, --tran, --netlist or --check-filter",
        ));
    }
    if let Some(Analysis::CheckFilter(words)) = &options.analysis {
        if options.input.is_some() {
            return Err(bad("--check-filter takes no input file"));
        }
        return check_filter(words, &options, &settings, out, err);
    }
    let Some(input) = &options.input else {
        let _ = err.write_all(USAGE.as_bytes());
        return Err(bad("an input file is needed"));
    };
    let netlist = load(
        input,
        options.model.as_deref(),
        &settings,
        options.lang,
        err,
    )?;
    let from_netlist;
    let analysis = match &options.analysis {
        Some(analysis) => analysis,
        None => {
            let Some(first) = netlist.analyses.first() else {
                let _ = err.write_all(USAGE.as_bytes());
                return Err(bad(
                    "an analysis is needed: --op, --dc, --ac, --tran, --netlist or --check-filter (the netlist asks for none)",
                ));
            };
            if netlist.analyses.len() > 1 {
                let _ = writeln!(
                    err,
                    "note: the netlist asks for {} analyses; the first one is run (name another with --op, --dc, --ac or --tran)",
                    netlist.analyses.len()
                );
            }
            from_netlist = from_directive(first);
            &from_netlist
        }
    };
    if let Analysis::Netlist = analysis {
        let _ = out.write_all(to_spice(&netlist).as_bytes());
        return Ok(0);
    }
    let circuit = Circuit::compile(&netlist, &settings)?;
    let labels = circuit.labels();
    let selected = select(&labels, &options.probes)?;
    match analysis {
        Analysis::Op => {
            let result = operating_point(&circuit, &settings)?;
            emit(
                &options,
                &settings,
                op_csv(&result, &selected),
                op_json(&result, &selected),
                out,
            )?;
        }
        Analysis::Dc(source, start, stop, step) => {
            let request = DcRequest {
                source: source.clone(),
                start: *start,
                stop: *stop,
                step: *step,
            };
            let result = dc_sweep(&circuit, &settings, &request)?;
            emit(
                &options,
                &settings,
                dc_csv(&result, &selected),
                dc_json(&result, &selected),
                out,
            )?;
        }
        Analysis::Ac(kind, points, start, stop) => {
            let request = AcRequest {
                kind: *kind,
                points: *points,
                start_hz: *start,
                stop_hz: *stop,
            };
            let result = ac_analysis(&circuit, &settings, &request)?;
            emit(
                &options,
                &settings,
                ac_csv(&result, &selected),
                ac_json(&result, &selected),
                out,
            )?;
        }
        Analysis::Tran(step, stop, uic) => {
            let request = TranRequest {
                tstep: *step,
                tstop: *stop,
                method: options.method,
                adaptive: options.adaptive,
                uic: options.uic || *uic,
            };
            let result = transient(&circuit, &settings, &request)?;
            for note in &result.notes {
                let _ = writeln!(err, "warning: {}", note.message(options.lang));
            }
            emit(
                &options,
                &settings,
                tran_csv(&result, &selected),
                tran_json(&result, &selected),
                out,
            )?;
        }
        Analysis::Netlist | Analysis::CheckFilter(_) => {}
    }
    Ok(0)
}

/// Runs the command with the arguments that follow its name. Returns the exit code.
pub(crate) fn sim_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match run_inner(args, out, err) {
        Ok(code) => code,
        Err(e) => {
            // the language is the one asked for, or English when the arguments were not read
            let lang = args
                .iter()
                .position(|a| *a == "--lang")
                .and_then(|i| args.get(i + 1))
                .and_then(|tag| Lang::from_tag(tag))
                .unwrap_or(Lang::En);
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}
