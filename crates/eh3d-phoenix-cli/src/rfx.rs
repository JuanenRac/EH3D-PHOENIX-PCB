// SPDX-License-Identifier: GPL-3.0-or-later
//! The extended `phoenix rf` commands: pi and T matching networks and the equivalents with
//! lines, filters with standard values and the simulated response, S parameters of a network
//! with a mask, and the stability of an amplifier. They only read files; `--out`, `--svg` and
//! `--s2p-out` write to a file that does not exist yet and never overwrite one. The numbers are
//! estimates (ideal elements, lossless quasi-static lines), nothing was compared with
//! measurements or with another program.
//!
//! Exit codes are those of `phoenix rf`: 0 done, 1 the calculation asked for has no result (no
//! matching network, a mask that is not met), 2 the arguments or the input are unusable.

use crate::rf::{parse_sweep, Args};
use crate::rfx_text::{
    filter_json, filter_text, lines_json, lines_text, match_json, match_text, sparams_json,
    sparams_text, stability_json, stability_text, FilterReport, FilterRow, LinesReport,
    MatchReport, NetworkResult, PassWord, SParamsReport, SRow, SnapResult, StabRow, StabilityOut,
};
use eh3d_phoenix_rf::circles::stability_report;
use eh3d_phoenix_rf::eseries::Series;
use eh3d_phoenix_rf::filter::{Band, Branch, Filter};
use eh3d_phoenix_rf::filterx::{snap, snap_effect};
use eh3d_phoenix_rf::line::Line;
use eh3d_phoenix_rf::mask::{check_mask, Mask};
use eh3d_phoenix_rf::matching::{l_match, LKind};
use eh3d_phoenix_rf::matchnet::{
    bandwidth, l_network_q, l_parts, ladder_input_impedance, line_equivalents, pi_match, t_match,
    Part, PassKind, ThreeElement,
};
use eh3d_phoenix_rf::netdesc::{figures, frequencies, response, to_touchstone, NetDesc, SPoint};
use eh3d_phoenix_rf::network::{max_available_gain, max_stable_gain, stability, Mat2};
use eh3d_phoenix_rf::touchstone::{ports_from_name, Format, FreqUnit};
use eh3d_phoenix_rf::trace::{smith_svg, steps_from_parts, trace};
use eh3d_phoenix_rf::{Complex, Language, RfError, RfSettings, Touchstone};
use eh3d_phoenix_sim::filter::{ac_response, filter_spice, ResponsePoint};
use eh3d_phoenix_sim::SimSettings;
use eh3d_phoenix_stackup::{StackupJob, StackupSettings};
use std::io::Write;

/// The S matrix of a two-port at each frequency in Hz.
type Response = Vec<(f64, Mat2)>;

/// What `phoenix rf stability` works from: the name of the source, the points of the table and,
/// for one frequency, the matrix to give in detail.
type StabilityInput = (String, Response, Option<(f64, Mat2)>);

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

fn rf_error(err: &mut dyn Write, error: &RfError, language: Language, code: i32) -> i32 {
    let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
    code
}

/// Write a file that does not exist yet.
fn write_new(path: &str, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{path}: {e}"))?;
    file.write_all(bytes).map_err(|e| format!("{path}: {e}"))
}

/// Give the text or the JSON to `--out` or to the standard output.
fn emit(
    args: &Args,
    text: String,
    json: serde_json::Value,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), i32> {
    let body = if args.has("--json") {
        match serde_json::to_string_pretty(&json) {
            Ok(mut s) => {
                s.push('\n');
                s
            }
            Err(e) => return Err(fail(err, &e.to_string())),
        }
    } else {
        text
    };
    match args.get("--out") {
        Some(path) => {
            if let Err(message) = write_new(path, body.as_bytes()) {
                return Err(fail(err, &format!("cannot write the output: {message}")));
            }
        }
        None => {
            let _ = out.write_all(body.as_bytes());
        }
    }
    Ok(())
}

/// A text file no larger than `limit` bytes.
fn read_limited(path: &str, limit: u64) -> Result<String, String> {
    let length = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if length > limit {
        return Err(format!("the file is larger than {limit} bytes"));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A two-port Touchstone file as S matrices.
fn read_two_port(path: &str, settings: &RfSettings) -> Result<(Touchstone, Response), String> {
    let Some(ports) = ports_from_name(path) else {
        return Err("the number of ports comes from the extension of the name".to_string());
    };
    if ports != 2 {
        return Err("a two-port file (.s2p) is needed".to_string());
    }
    let text = read_limited(path, settings.max_file_bytes)?;
    let file = Touchstone::parse(&text, ports, settings).map_err(|e| e.to_string())?;
    let mut data = Vec::with_capacity(file.points.len());
    for (index, point) in file.points.iter().enumerate() {
        let matrix = file.s_matrix(index).map_err(|e| e.to_string())?;
        let s = Mat2::from_cmatrix(&matrix).map_err(|e| e.to_string())?;
        data.push((point.freq_hz, s));
    }
    Ok((file, data))
}

// ---------------------------------------------------------------------------------------------
// matching

fn pass_of(kind: LKind) -> PassWord {
    match kind {
        LKind::LowPass => PassWord::Low,
        LKind::HighPass => PassWord::High,
        LKind::Mixed => PassWord::Mixed,
    }
}

fn pass_of_three(net: &ThreeElement) -> PassWord {
    match net.pass {
        PassKind::LowPass => PassWord::Low,
        PassKind::HighPass => PassWord::High,
    }
}

/// What is known of a network before it is checked.
struct Candidate {
    shape: &'static str,
    pass: PassWord,
    parts: Vec<Part>,
    q_network: f64,
    virtual_ohm: Option<f64>,
}

fn network_result(
    candidate: Candidate,
    ends: (Complex, Complex),
    freq_hz: f64,
    settings: &RfSettings,
) -> Result<NetworkResult, RfError> {
    let Candidate {
        shape,
        pass,
        parts,
        q_network,
        virtual_ohm,
    } = candidate;
    let (source, load) = ends;
    let input_impedance = ladder_input_impedance(&parts, load, freq_hz)?;
    let bw = bandwidth(&parts, source, load, freq_hz, q_network, settings)?;
    let steps = steps_from_parts(&parts, freq_hz)?;
    let chart = trace(load, &steps, source.re, settings.smith_arc_points)?;
    Ok(NetworkResult {
        shape,
        pass,
        parts,
        q_network,
        virtual_ohm,
        bandwidth: bw,
        input_impedance,
        trace: chart,
    })
}

fn pass_choice(args: &Args) -> Result<Vec<PassKind>, String> {
    match args.get("--pass") {
        None => Ok(vec![PassKind::LowPass, PassKind::HighPass]),
        Some("low") => Ok(vec![PassKind::LowPass]),
        Some("high") => Ok(vec![PassKind::HighPass]),
        Some(_) => Err("--pass is low or high".to_string()),
    }
}

fn build_networks(
    kind: &str,
    ends: (Complex, Complex),
    freq_hz: f64,
    q: f64,
    passes: &[PassKind],
    settings: &RfSettings,
) -> Result<Vec<NetworkResult>, RfError> {
    let (source, load) = ends;
    let mut networks = Vec::new();
    match kind {
        "l" => {
            for m in l_match(source, load, freq_hz, settings)? {
                let candidate = Candidate {
                    shape: "L",
                    pass: pass_of(m.kind),
                    parts: l_parts(&m),
                    q_network: l_network_q(&m, source, load)?,
                    virtual_ohm: None,
                };
                networks.push(network_result(candidate, ends, freq_hz, settings)?);
            }
        }
        "pi" | "t" => {
            for &pass in passes {
                let net = if kind == "pi" {
                    pi_match(source, load, freq_hz, q, pass, settings)?
                } else {
                    t_match(source, load, freq_hz, q, pass, settings)?
                };
                let candidate = Candidate {
                    shape: if kind == "pi" { "pi" } else { "T" },
                    pass: pass_of_three(&net),
                    parts: net.parts.to_vec(),
                    q_network: net.loaded_q,
                    virtual_ohm: Some(net.virtual_ohm),
                };
                networks.push(network_result(candidate, ends, freq_hz, settings)?);
            }
        }
        _ => {}
    }
    Ok(networks)
}

/// `phoenix rf match l|pi|t` with the extended report, and `phoenix rf match lines`.
pub(crate) fn match_command(
    kind: &str,
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let load = match args.impedance("--load") {
        Ok(Some(z)) => z,
        Ok(None) => return fail(err, "--load is needed"),
        Err(message) => return fail(err, &message),
    };
    let freq_hz = match args.required_number("--freq-ghz") {
        Ok(f) => f * 1e9,
        Err(message) => return fail(err, &message),
    };
    if kind == "lines" {
        return lines_command(args, load, freq_hz, settings, language, out, err);
    }
    let source = match args.impedance("--source") {
        Ok(z) => z.unwrap_or(Complex::real(settings.default_z0_ohm)),
        Err(message) => return fail(err, &message),
    };
    let q = match args.number("--q") {
        Ok(q) => q.unwrap_or(settings.default_loaded_q),
        Err(message) => return fail(err, &message),
    };
    let passes = match pass_choice(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &message),
    };
    let ends = (source, load);
    let networks = match build_networks(kind, ends, freq_hz, q, &passes, settings) {
        Ok(list) => list,
        Err(error) => return rf_error(err, &error, language, 1),
    };
    if networks.is_empty() {
        return fail(err, "unknown kind of network");
    }
    if let Some(path) = args.get("--svg") {
        let pick = match args.number("--pick") {
            Ok(p) => p.unwrap_or(1.0),
            Err(message) => return fail(err, &message),
        };
        let index = pick as usize;
        if pick.fract() != 0.0 || index == 0 || index > networks.len() {
            return fail(err, "--pick is the number of a network of the list");
        }
        let net = &networks[index - 1];
        let title = format!(
            "{} {} matching network",
            net.shape,
            match net.pass {
                PassWord::Low => "low-pass",
                PassWord::High => "high-pass",
                PassWord::Mixed => "mixed",
            }
        );
        if let Err(message) = write_new(path, smith_svg(&net.trace, &title).as_bytes()) {
            return fail(err, &format!("cannot write the picture: {message}"));
        }
    }
    let report = MatchReport {
        source,
        load,
        freq_hz,
        return_loss_limit_db: settings.return_loss_bandwidth_db,
        networks,
    };
    match emit(
        args,
        match_text(&report, language),
        match_json(&report),
        out,
        err,
    ) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

fn line_from_args(
    args: &Args,
    freq_hz: f64,
    settings: &RfSettings,
) -> Result<(Line, Option<String>), String> {
    if let Some(path) = args.get("--stackup") {
        let name = args
            .get("--line")
            .ok_or_else(|| "--line names the line of the stack-up file".to_string())?;
        let text = read_limited(path, 16 * 1024 * 1024)?;
        let job = StackupJob::parse(&text).map_err(|e| e.to_string())?;
        let outcomes = job.run(freq_hz / 1e9, &StackupSettings::default());
        let outcome = outcomes
            .into_iter()
            .find(|o| o.name == name)
            .ok_or_else(|| format!("the stack-up file has no line named {name}"))?;
        let result = outcome.result.map_err(|e| e.to_string())?;
        let line = Line::from_stackup(&result).map_err(|e| e.to_string())?;
        return Ok((line, Some(name.to_string())));
    }
    let z0 = args.number("--z0")?.unwrap_or(settings.default_z0_ohm);
    let eps = args.number("--eps-eff")?.unwrap_or(1.0);
    Ok((Line::new(z0, eps).map_err(|e| e.to_string())?, None))
}

fn lines_command(
    args: &Args,
    load: Complex,
    freq_hz: f64,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let (line, line_name) = match line_from_args(args, freq_hz, settings) {
        Ok(found) => found,
        Err(message) => return fail(err, &message),
    };
    let spacing = match args.number("--spacing-deg") {
        Ok(s) => s.unwrap_or(settings.default_stub_spacing_deg),
        Err(message) => return fail(err, &message),
    };
    let equivalents = match line_equivalents(load, &line, freq_hz, spacing, settings) {
        Ok(found) => found,
        Err(error) => return rf_error(err, &error, language, 1),
    };
    let report = LinesReport {
        load,
        equivalents,
        line_name,
    };
    match emit(
        args,
        lines_text(&report, language),
        lines_json(&report),
        out,
        err,
    ) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

// ---------------------------------------------------------------------------------------------
// filters

fn band_text(band: Band) -> String {
    let hz = |x: f64| eh3d_phoenix_rf::report::engineering(x, "Hz");
    match band {
        Band::LowPass { cutoff_hz } => format!("lowpass {}", hz(cutoff_hz)),
        Band::HighPass { cutoff_hz } => format!("highpass {}", hz(cutoff_hz)),
        Band::BandPass { low_hz, high_hz } => {
            format!("bandpass {} .. {}", hz(low_hz), hz(high_hz))
        }
        Band::BandStop { low_hz, high_hz } => {
            format!("bandstop {} .. {}", hz(low_hz), hz(high_hz))
        }
    }
}

fn components_of(filter: &Filter) -> Vec<(String, f64)> {
    let mut list = Vec::new();
    for (index, section) in filter.sections.iter().enumerate() {
        let n = index + 1;
        match section.branch {
            Branch::Inductor(l) => list.push((format!("L{n}"), l)),
            Branch::Capacitor(c) => list.push((format!("C{n}"), c)),
            Branch::SeriesLc {
                inductance_h,
                capacitance_f,
            }
            | Branch::ParallelLc {
                inductance_h,
                capacitance_f,
            } => {
                list.push((format!("L{n}"), inductance_h));
                list.push((format!("C{n}"), capacitance_f));
            }
        }
    }
    list
}

/// The filter of a `phoenix rf filter` command and what was asked about it.
pub(crate) struct FilterInput<'a> {
    pub ladder: &'a Filter,
    pub family: &'static str,
    pub ripple_db: Option<f64>,
    pub sweep: Option<(f64, f64, usize)>,
}

/// `phoenix rf filter` with `--snap`, `--simulate`, `--netlist`, `--json` or `--out`.
pub(crate) fn filter_extended(
    args: &Args,
    input: &FilterInput,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let FilterInput {
        ladder,
        family,
        ripple_db,
        sweep,
    } = *input;
    let snapped = match args.get("--snap") {
        None => None,
        Some(name) => {
            let named = if name == "default" {
                Series::from_number(settings.default_series)
            } else {
                Series::from_name(name)
            };
            let series = match named {
                Ok(s) => s,
                Err(error) => return rf_error(err, &error, language, 2),
            };
            let made = snap(ladder, series)
                .and_then(|s| snap_effect(ladder, &s.filter, settings).map(|effect| (s, effect)));
            match made {
                Ok(pair) => Some(pair),
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
    };
    let freqs = match sweep {
        None => Vec::new(),
        Some((start, stop, count)) => {
            match frequencies(start * 1e9, stop * 1e9, count, args.has("--log"), settings) {
                Ok(list) => list,
                Err(error) => return rf_error(err, &error, language, 1),
            }
        }
    };
    if args.has("--simulate") && freqs.is_empty() {
        return fail(err, "--simulate needs --sweep-ghz START,STOP,POINTS");
    }
    let simulated: Vec<Option<ResponsePoint>> = if args.has("--simulate") {
        match ac_response(ladder, &freqs, &SimSettings::default()) {
            Ok(points) => points.into_iter().map(Some).collect(),
            Err(error) => return fail(err, &format!("simulation: {}", error.detail)),
        }
    } else {
        vec![None; freqs.len()]
    };
    let mut rows = Vec::with_capacity(freqs.len());
    for (freq_hz, sim) in freqs.iter().zip(simulated) {
        let point = match ladder.response_at(*freq_hz) {
            Ok(p) => p,
            Err(error) => return rf_error(err, &error, language, 1),
        };
        let delay_s = match ladder.group_delay_s(*freq_hz, settings) {
            Ok(d) => d,
            Err(error) => return rf_error(err, &error, language, 1),
        };
        let snapped_gain_db = match &snapped {
            None => None,
            Some((s, _)) => match s.filter.response_at(*freq_hz) {
                Ok(p) => Some(p.s21.db20()),
                Err(error) => return rf_error(err, &error, language, 1),
            },
        };
        rows.push(FilterRow {
            freq_hz: *freq_hz,
            gain_db: point.s21.db20(),
            phase_deg: point.s21.arg_deg(),
            delay_s,
            simulated: sim,
            snapped_gain_db,
        });
    }
    let spice = if args.has("--netlist") {
        Some(match &snapped {
            Some((s, _)) => filter_spice(&s.filter),
            None => filter_spice(ladder),
        })
    } else {
        None
    };
    let report = FilterReport {
        approximation: family,
        order: ladder.prototype.order,
        band: band_text(ladder.band),
        ripple_db,
        z0_ohm: ladder.z0_ohm,
        load_ohm: ladder.load_ohm,
        g: ladder.prototype.g.clone(),
        g_load: ladder.prototype.g_load,
        components: components_of(ladder),
        snap: snapped.map(|(s, effect)| SnapResult {
            series: s.series,
            values: s.values,
            effect,
        }),
        rows,
        spice,
    };
    match emit(
        args,
        filter_text(&report, language),
        filter_json(&report),
        out,
        err,
    ) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Whether a `phoenix rf filter` command asks for something of the extended report.
pub(crate) fn wants_extended_filter(args: &Args) -> bool {
    args.get("--snap").is_some()
        || args.get("--out").is_some()
        || ["--simulate", "--netlist", "--json"]
            .iter()
            .any(|flag| args.has(flag))
}

/// Whether a `phoenix rf match l` command asks for the extended report.
pub(crate) fn wants_extended_match(args: &Args) -> bool {
    ["--svg", "--out", "--pass", "--pick"]
        .iter()
        .any(|key| args.get(key).is_some())
        || args.has("--json")
}

// ---------------------------------------------------------------------------------------------
// S parameters

fn format_of(args: &Args) -> Result<Format, String> {
    match args.get("--format") {
        None | Some("ri") => Ok(Format::Ri),
        Some("ma") => Ok(Format::Ma),
        Some("db") => Ok(Format::Db),
        Some(_) => Err("--format is ri, ma or db".to_string()),
    }
}

/// `phoenix rf sparams`.
pub(crate) fn sparams_command(
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let (name, z0_ohm, data): (String, f64, Response) = match (args.get("--net"), args.get("--s2p"))
    {
        (Some(path), None) => {
            let text = match read_limited(path, settings.max_net_bytes as u64) {
                Ok(t) => t,
                Err(message) => return fail(err, &message),
            };
            let desc = match NetDesc::parse(&text, settings) {
                Ok(d) => d,
                Err(error) => return rf_error(err, &error, language, 2),
            };
            let Some(spec) = args.get("--sweep-ghz") else {
                return fail(err, "--sweep-ghz START,STOP,POINTS is needed with --net");
            };
            let Some((start, stop, count)) = parse_sweep(spec) else {
                return fail(err, "--sweep-ghz is START,STOP,POINTS");
            };
            let freqs =
                match frequencies(start * 1e9, stop * 1e9, count, args.has("--log"), settings) {
                    Ok(f) => f,
                    Err(error) => return rf_error(err, &error, language, 1),
                };
            let points: Vec<SPoint> = match response(&desc, &freqs) {
                Ok(p) => p,
                Err(error) => return rf_error(err, &error, language, 1),
            };
            if let Some(path) = args.get("--s2p-out") {
                let format = match format_of(args) {
                    Ok(f) => f,
                    Err(message) => return fail(err, &message),
                };
                let file = to_touchstone(
                    &points,
                    desc.z0_ohm,
                    format,
                    FreqUnit::GHz,
                    "S parameters of a network of ideal elements and lossless lines",
                );
                let text = match file.to_text() {
                    Ok(t) => t,
                    Err(error) => return rf_error(err, &error, language, 1),
                };
                if let Err(message) = write_new(path, text.as_bytes()) {
                    return fail(err, &format!("cannot write the Touchstone file: {message}"));
                }
            }
            (
                path.to_string(),
                desc.z0_ohm,
                points.iter().map(|p| (p.freq_hz, p.s)).collect(),
            )
        }
        (None, Some(path)) => {
            if args.get("--s2p-out").is_some() {
                return fail(err, "--s2p-out writes the response of a --net description");
            }
            match read_two_port(path, settings) {
                Ok((file, data)) => (path.to_string(), file.z_ref_ohm, data),
                Err(message) => return fail(err, &message),
            }
        }
        _ => return fail(err, "give --net FILE or --s2p FILE (one of them)"),
    };
    let mask = match args.get("--mask") {
        None => None,
        Some(path) => {
            let text = match read_limited(path, settings.max_net_bytes as u64) {
                Ok(t) => t,
                Err(message) => return fail(err, &message),
            };
            match Mask::parse(&text, settings) {
                Ok(mask) => Some(check_mask(&mask, &data, settings)),
                Err(error) => return rf_error(err, &error, language, 2),
            }
        }
    };
    let passed = mask.as_ref().is_none_or(|m| m.passed);
    let report = SParamsReport {
        source_name: name,
        z0_ohm,
        rows: data
            .iter()
            .map(|(freq_hz, s)| SRow {
                freq_hz: *freq_hz,
                s: *s,
                figures: figures(s),
            })
            .collect(),
        mask,
    };
    match emit(
        args,
        sparams_text(&report, language),
        sparams_json(&report),
        out,
        err,
    ) {
        Ok(()) if passed => 0,
        Ok(()) => 1,
        Err(code) => code,
    }
}

// ---------------------------------------------------------------------------------------------
// stability

/// A value as `a+jb` or as `magnitude@angle` in degrees.
fn parse_s_value(text: &str) -> Result<Complex, String> {
    match text.split_once('@') {
        Some((magnitude, angle)) => {
            let m: f64 = magnitude
                .trim()
                .parse()
                .map_err(|_| format!("{text} is not magnitude@angle"))?;
            let a: f64 = angle
                .trim()
                .parse()
                .map_err(|_| format!("{text} is not magnitude@angle"))?;
            if !m.is_finite() || !a.is_finite() || m < 0.0 {
                return Err(format!("{text} is not magnitude@angle"));
            }
            Ok(Complex::from_polar(m, a.to_radians()))
        }
        None => Complex::parse(text).map_err(|e| e.to_string()),
    }
}

/// `phoenix rf stability`.
pub(crate) fn stability_command(
    args: &Args,
    settings: &RfSettings,
    language: Language,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let freq_hz = match args.number("--freq-ghz") {
        Ok(f) => f.map(|g| g * 1e9),
        Err(message) => return fail(err, &message),
    };
    let (name, rows, detail_s): StabilityInput = match args.get("--s2p") {
        Some(path) => {
            let (file, data) = match read_two_port(path, settings) {
                Ok(found) => found,
                Err(message) => return fail(err, &message),
            };
            match freq_hz {
                None => (path.to_string(), data, None),
                Some(f) => {
                    let s = match file.s_matrix_at(f).and_then(|m| Mat2::from_cmatrix(&m)) {
                        Ok(s) => s,
                        Err(error) => return rf_error(err, &error, language, 1),
                    };
                    (path.to_string(), vec![(f, s)], Some((f, s)))
                }
            }
        }
        None => {
            let mut values = [Complex::ZERO; 4];
            for (slot, key) in values.iter_mut().zip(["--s11", "--s12", "--s21", "--s22"]) {
                let Some(text) = args.get(key) else {
                    return fail(
                        err,
                        "give --s2p FILE or the four values --s11 --s12 --s21 --s22",
                    );
                };
                match parse_s_value(text) {
                    Ok(z) => *slot = z,
                    Err(message) => return fail(err, &format!("{key}: {message}")),
                }
            }
            let s = Mat2::new(values[0], values[1], values[2], values[3]);
            let f = freq_hz.unwrap_or(0.0);
            (
                "--s11 --s12 --s21 --s22".to_string(),
                vec![(f, s)],
                Some((f, s)),
            )
        }
    };
    let detail = match detail_s {
        None => None,
        Some((f, s)) => match stability_report(&s, settings.tolerance) {
            Ok(report) => Some((f, s, report)),
            Err(error) => return rf_error(err, &error, language, 1),
        },
    };
    let (msg, mag) = match &detail {
        Some((_, s, _)) => (Some(max_stable_gain(s)), max_available_gain(s)),
        None => (None, None),
    };
    let report = StabilityOut {
        source_name: name,
        rows: rows
            .iter()
            .map(|(f, s)| StabRow {
                freq_hz: *f,
                figures: stability(s),
            })
            .collect(),
        detail,
        max_stable_gain: msg,
        max_available_gain: mag,
    };
    match emit(
        args,
        stability_text(&report, language),
        stability_json(&report),
        out,
        err,
    ) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_values_are_read_as_polar_or_rectangular() {
        let z = parse_s_value("0.5@90").unwrap();
        assert!(z.re.abs() < 1e-12 && (z.im - 0.5).abs() < 1e-12);
        let w = parse_s_value("0.3+j0.4").unwrap();
        assert!((w.abs() - 0.5).abs() < 1e-12);
        for bad in ["", "x@1", "1@x", "-1@0", "inf@0", "1@nan", "j"] {
            assert!(parse_s_value(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_file_is_written_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        let path = path.to_str().unwrap();
        write_new(path, b"one").unwrap();
        assert!(write_new(path, b"two").is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "one");
    }
}
