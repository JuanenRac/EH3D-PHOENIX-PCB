// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix rf`: Touchstone files, impedance matching and ladder filters.
//!
//! Exit codes: 0 done, 1 the calculation asked for has no result (no matching network, a
//! frequency outside the file), 2 the arguments or the input are unusable.

use eh3d_phoenix_rf::filter::{design, prototype, Approximation, Band, Position};
use eh3d_phoenix_rf::matching::{l_match, quarter_wave, stub_match, StubKind, Termination};
use eh3d_phoenix_rf::report::{
    filter_report, l_match_report, quarter_wave_report, stub_report, touchstone_report,
};
use eh3d_phoenix_rf::touchstone::ports_from_name;
use eh3d_phoenix_rf::{Complex, Language, RfError, RfSettings, Touchstone};
use std::io::Write;

use crate::rfx;

/// The usage text of the command.
pub const RF_USAGE: &str = "\
phoenix rf touchstone <file.sNp> [--freq-ghz X] [--settings FILE] [--lang xx]

  touchstone  read a Touchstone file (the number of ports comes from the extension of the
              name) and print its header and, at a frequency, its parameters, return and
              insertion loss, VSWR, group delay, gain, stability, passivity and reciprocity

phoenix rf match l    --load Z [--source Z] --freq-ghz F [--settings FILE] [--lang xx]
phoenix rf match stub --load Z [--z0 R] --freq-ghz F [--stub shunt|series] [--end open|short]
                      [--eps-eff E] [--settings FILE] [--lang xx]
phoenix rf match qw   --load Z [--z0 R] --freq-ghz F [--eps-eff E] [--settings FILE] [--lang xx]

  match       impedance matching: L network (the components), single stub or quarter-wave
              transformer (electrical lengths, and metres with --eps-eff); Z is an impedance
              in ohm, as 50, 50+j25, 50-j25 or j25

phoenix rf filter <butterworth|chebyshev|bessel> <order> <lowpass|highpass|bandpass|bandstop>
                  (--fc-ghz F | --f1-ghz F --f2-ghz F) [--ripple-db R] [--z0 R]
                  [--first shunt|series] [--sweep-ghz START,STOP,POINTS] [--log]
                  [--snap E12|E24|E96|default] [--simulate] [--netlist] [--json] [--out FILE]
                  [--settings FILE] [--lang xx]

  filter      ladder filter from the prototype tables: g values, the inductors and capacitors,
              and optionally the insertion and return loss over a sweep; ideal components;
              --snap takes the components to a series of standard values and says how far
              the band edges move, --simulate adds the response of the AC simulator, --netlist
              adds the SPICE text; the result is an estimate

phoenix rf match l|pi|t --load Z [--source Z] --freq-ghz F [--q Q] [--pass low|high]
                  [--svg FILE] [--pick N] [--json] [--out FILE] [--settings FILE] [--lang xx]
phoenix rf match lines --load Z --freq-ghz F [--z0 R --eps-eff E | --stackup FILE --line NAME]
                  [--spacing-deg D] [--json] [--out FILE] [--settings FILE] [--lang xx]

  match pi|t  matching networks with a chosen loaded Q, the bandwidth, the path on the Smith
              chart (as data, and as a picture with --svg); match lines gives the single
              stub, the double stub and the quarter-wave transformer with lengths in metres

phoenix rf sparams (--net FILE --sweep-ghz START,STOP,POINTS [--log] | --s2p FILE)
                  [--mask FILE] [--s2p-out FILE] [--format ri|ma|db] [--json] [--out FILE]
phoenix rf stability (--s2p FILE [--freq-ghz F] | --s11 X --s12 X --s21 X --s22 X)
                  [--json] [--out FILE]

  sparams     S parameters, return and insertion loss and VSWR of a network of lumped
              elements and lines, written as text, or of a Touchstone file; a mask gives a
              pass or fail (exit code 1 on fail)
  stability   K, delta, mu, stability circles and the simultaneous conjugate match of an
              amplifier from its S parameters

  --out, --svg and --s2p-out write to a file that does not exist yet and never overwrite one.
";

pub(crate) struct Args<'a> {
    pub(crate) positional: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>,
    flags: Vec<&'a str>,
}

impl<'a> Args<'a> {
    fn split(args: &[&'a str], valued: &[&str], flags: &[&str]) -> Option<Args<'a>> {
        let mut parsed = Args {
            positional: Vec::new(),
            options: Vec::new(),
            flags: Vec::new(),
        };
        let mut iter = args.iter().copied();
        while let Some(arg) = iter.next() {
            if !arg.starts_with("--") {
                parsed.positional.push(arg);
            } else if valued.contains(&arg) {
                parsed.options.push((arg, iter.next()?));
            } else if flags.contains(&arg) {
                parsed.flags.push(arg);
            } else {
                return None;
            }
        }
        Some(parsed)
    }

    pub(crate) fn get(&self, key: &str) -> Option<&'a str> {
        self.options
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    }

    pub(crate) fn has(&self, flag: &str) -> bool {
        self.flags.contains(&flag)
    }

    /// A number option: `Ok(None)` when absent, `Err` when it is not a finite number.
    pub(crate) fn number(&self, key: &str) -> Result<Option<f64>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(text) => match text.parse::<f64>() {
                Ok(value) if value.is_finite() => Ok(Some(value)),
                _ => Err(format!("{key} is not a number")),
            },
        }
    }

    pub(crate) fn required_number(&self, key: &str) -> Result<f64, String> {
        self.number(key)?.ok_or_else(|| format!("{key} is needed"))
    }

    /// An impedance option: `Ok(None)` when absent.
    pub(crate) fn impedance(&self, key: &str) -> Result<Option<Complex>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(text) => Complex::parse(text)
                .map(Some)
                .map_err(|e| format!("{key}: {e}")),
        }
    }
}

const VALUED: [&str; 33] = [
    "--freq-ghz",
    "--settings",
    "--lang",
    "--load",
    "--source",
    "--z0",
    "--stub",
    "--end",
    "--eps-eff",
    "--fc-ghz",
    "--f1-ghz",
    "--f2-ghz",
    "--ripple-db",
    "--first",
    "--sweep-ghz",
    "--q",
    "--pass",
    "--pick",
    "--svg",
    "--out",
    "--snap",
    "--net",
    "--s2p",
    "--s2p-out",
    "--mask",
    "--format",
    "--stackup",
    "--line",
    "--spacing-deg",
    "--s11",
    "--s12",
    "--s21",
    "--s22",
];
const FLAGS: [&str; 4] = ["--log", "--json", "--simulate", "--netlist"];

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

fn usage(err: &mut dyn Write) -> i32 {
    let _ = err.write_all(RF_USAGE.as_bytes());
    2
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

fn rf_error(err: &mut dyn Write, error: &RfError, language: Language, code: i32) -> i32 {
    let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
    code
}

fn read_settings(path: Option<&str>) -> Result<RfSettings, String> {
    let Some(path) = path else {
        return Ok(RfSettings::default());
    };
    let length = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if length > MAX_SETTINGS_BYTES {
        return Err(format!(
            "the file is larger than {MAX_SETTINGS_BYTES} bytes"
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    RfSettings::parse(&text).map_err(|e| e.to_string())
}

/// Run `phoenix rf` with the arguments after the word `rf`.
pub fn rf(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&command, tail)) = args.split_first() else {
        return usage(err);
    };
    let Some(parsed) = Args::split(tail, &VALUED, &FLAGS) else {
        return usage(err);
    };
    let language = match parsed.get("--lang") {
        None => Language::En,
        Some(tag) => match Language::from_tag(tag) {
            Some(found) => found,
            None => return usage(err),
        },
    };
    let settings = match read_settings(parsed.get("--settings")) {
        Ok(settings) => settings,
        Err(message) => return fail(err, &format!("cannot use the settings: {message}")),
    };
    match command {
        "touchstone" => touchstone(&parsed, &settings, language, out, err),
        "match" => matching(&parsed, &settings, language, out, err),
        "filter" => filter(&parsed, &settings, language, out, err),
        "sparams" => rfx::sparams_command(&parsed, &settings, language, out, err),
        "stability" => rfx::stability_command(&parsed, &settings, language, out, err),
        _ => usage(err),
    }
}

fn touchstone(
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let [path] = args.positional.as_slice() else {
        return usage(err);
    };
    let freq = match args.number("--freq-ghz") {
        Ok(freq) => freq,
        Err(message) => return fail(err, &message),
    };
    let Some(ports) = ports_from_name(path) else {
        return fail(
            err,
            "the number of ports comes from the extension of the name (.s1p, .s2p, ...)",
        );
    };
    let length = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(e) => return fail(err, &format!("cannot read the file: {e}")),
    };
    if length > settings.max_file_bytes {
        return fail(
            err,
            &format!("the file is larger than {} bytes", settings.max_file_bytes),
        );
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return fail(err, &format!("cannot read the file: {e}")),
    };
    let text = String::from_utf8_lossy(&bytes);
    let file = match Touchstone::parse(&text, ports, settings) {
        Ok(file) => file,
        Err(error) => return rf_error(err, &error, language, 2),
    };
    match touchstone_report(&file, freq, settings) {
        Ok(report) => {
            let _ = out.write_all(report.as_bytes());
            0
        }
        Err(error) => rf_error(err, &error, language, 1),
    }
}

fn matching(
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let [kind] = args.positional.as_slice() else {
        return usage(err);
    };
    if matches!(*kind, "pi" | "t" | "lines") || (*kind == "l" && rfx::wants_extended_match(args)) {
        return rfx::match_command(kind, args, settings, language, out, err);
    }
    let (load, freq_hz, eps_eff) = match match_inputs(args) {
        Ok(found) => found,
        Err(message) => return fail(err, &message),
    };
    let z0 = match args.number("--z0") {
        Ok(value) => value.unwrap_or(settings.default_z0_ohm),
        Err(message) => return fail(err, &message),
    };
    let report = match *kind {
        "l" => {
            let source = match args.impedance("--source") {
                Ok(value) => value.unwrap_or(Complex::real(settings.default_z0_ohm)),
                Err(message) => return fail(err, &message),
            };
            match l_match(source, load, freq_hz, settings) {
                Ok(found) => l_match_report(source, load, freq_hz, &found),
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
        "stub" => {
            let stub = match args.get("--stub") {
                None | Some("shunt") => StubKind::Shunt,
                Some("series") => StubKind::Series,
                Some(_) => return usage(err),
            };
            let end = match args.get("--end") {
                None | Some("short") => Termination::Short,
                Some("open") => Termination::Open,
                Some(_) => return usage(err),
            };
            if let Err(error) = settings.check_frequency_hz("frequency", freq_hz) {
                return rf_error(err, &error, language, 1);
            }
            match stub_match(load, z0, stub, end, settings) {
                Ok(found) => stub_report(load, z0, freq_hz, eps_eff, &found),
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
        "qw" => {
            if let Err(error) = settings.check_frequency_hz("frequency", freq_hz) {
                return rf_error(err, &error, language, 1);
            }
            match quarter_wave(load, z0, settings) {
                Ok(found) => quarter_wave_report(load, z0, freq_hz, eps_eff, &found),
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
        _ => return usage(err),
    };
    let _ = out.write_all(report.as_bytes());
    0
}

fn filter(
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let [family, order, band] = args.positional.as_slice() else {
        return usage(err);
    };
    let approximation = match *family {
        "butterworth" => Approximation::Butterworth,
        "chebyshev" => Approximation::Chebyshev,
        "bessel" => Approximation::Bessel,
        "legendre" | "elliptic" => {
            return fail(
                err,
                "this approximation is not implemented: it needs a published table that is not in the tool yet",
            )
        }
        _ => return usage(err),
    };
    let family_name: &'static str = match approximation {
        Approximation::Butterworth => "Butterworth",
        Approximation::Chebyshev => "Chebyshev",
        Approximation::Bessel => "Bessel",
    };
    let Ok(order) = order.parse::<usize>() else {
        return fail(err, "the order is not a whole number");
    };
    let first = match args.get("--first") {
        None | Some("shunt") => Position::Shunt,
        Some("series") => Position::Series,
        Some(_) => return usage(err),
    };
    let FilterNumbers {
        ripple,
        z0,
        fc,
        f1,
        f2,
    } = match filter_numbers(args, settings) {
        Ok(found) => found,
        Err(message) => return fail(err, &message),
    };
    let edges = match (*band, fc, f1, f2) {
        ("lowpass", Some(fc), _, _) => Band::LowPass {
            cutoff_hz: fc * 1e9,
        },
        ("highpass", Some(fc), _, _) => Band::HighPass {
            cutoff_hz: fc * 1e9,
        },
        ("bandpass", _, Some(a), Some(b)) => Band::BandPass {
            low_hz: a * 1e9,
            high_hz: b * 1e9,
        },
        ("bandstop", _, Some(a), Some(b)) => Band::BandStop {
            low_hz: a * 1e9,
            high_hz: b * 1e9,
        },
        ("lowpass" | "highpass" | "bandpass" | "bandstop", _, _, _) => {
            return fail(
                err,
                "the band edges are missing: --fc-ghz or --f1-ghz and --f2-ghz",
            )
        }
        _ => return usage(err),
    };
    let sweep = match args.get("--sweep-ghz") {
        None => None,
        Some(text) => match parse_sweep(text) {
            Some(found) => Some(found),
            None => return fail(err, "--sweep-ghz is START,STOP,POINTS"),
        },
    };
    let designed = prototype(approximation, order, ripple, first, settings)
        .and_then(|p| design(&p, edges, z0, settings));
    let ladder = match designed {
        Ok(ladder) => ladder,
        Err(error) => return rf_error(err, &error, language, 1),
    };
    if rfx::wants_extended_filter(args) {
        let input = rfx::FilterInput {
            ladder: &ladder,
            family: family_name,
            ripple_db: if approximation == Approximation::Chebyshev {
                ripple
            } else {
                None
            },
            sweep,
        };
        return rfx::filter_extended(args, &input, settings, language, out, err);
    }
    let points = match sweep {
        None => None,
        Some((start, stop, count)) => {
            match ladder.sweep(start * 1e9, stop * 1e9, count, args.has("--log"), settings) {
                Ok(points) => Some(points),
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
    };
    let _ = out.write_all(filter_report(&ladder, points.as_deref()).as_bytes());
    0
}

/// The load, the frequency in Hz and the effective permittivity of `phoenix rf match`.
fn match_inputs(args: &Args) -> Result<(Complex, f64, f64), String> {
    let load = args
        .impedance("--load")?
        .ok_or_else(|| "--load is needed".to_string())?;
    let freq_hz = args.required_number("--freq-ghz")? * 1e9;
    let eps_eff = args.number("--eps-eff")?.unwrap_or(1.0);
    Ok((load, freq_hz, eps_eff))
}

/// The numbers of `phoenix rf filter`.
struct FilterNumbers {
    ripple: Option<f64>,
    z0: f64,
    fc: Option<f64>,
    f1: Option<f64>,
    f2: Option<f64>,
}

fn filter_numbers(args: &Args, settings: &RfSettings) -> Result<FilterNumbers, String> {
    Ok(FilterNumbers {
        ripple: args.number("--ripple-db")?,
        z0: args.number("--z0")?.unwrap_or(settings.default_z0_ohm),
        fc: args.number("--fc-ghz")?,
        f1: args.number("--f1-ghz")?,
        f2: args.number("--f2-ghz")?,
    })
}

pub(crate) fn parse_sweep(text: &str) -> Option<(f64, f64, usize)> {
    let mut parts = text.split(',');
    let start = parts.next()?.trim().parse::<f64>().ok()?;
    let stop = parts.next()?.trim().parse::<f64>().ok()?;
    let count = parts.next()?.trim().parse::<usize>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((start, stop, count))
}
