// SPDX-License-Identifier: GPL-3.0-or-later
//! Measures taken from the results of an analysis: the value of a signal at an instant, its
//! maximum, minimum, mean or root mean square in a window, the time it crosses a level, its rise
//! and fall times, and, on an AC sweep, the gain and phase at a frequency, the peak, the -3 dB
//! bandwidth, the quality factor, the unity-gain frequency and the phase and gain margins.
//!
//! A measure is written as one line, in a netlist after `.measure` or on the command line after
//! `--measure`:
//!
//! ```text
//! tran NAME at SIGNAL TIME
//! tran NAME max|min|avg|rms|pp SIGNAL [from=T1] [to=T2]
//! tran NAME cross SIGNAL LEVEL [rise|fall|any] [n=K] [from=T]
//! tran NAME rise|fall SIGNAL [low=0.1] [high=0.9] [from=T1] [to=T2]
//! dc   NAME ...                      (the same words; the "time" is the swept value)
//! ac   NAME gain|gaindb|phase SIGNAL FREQ
//! ac   NAME bw SIGNAL [ref=first|max]
//! ac   NAME peak|peakfreq|qband|qlow|ugf|phasemargin|gainmargin SIGNAL
//! ```
//!
//! The waveform measures read the points of the result with straight lines between them. The
//! rise time is taken from the first rising edge between the smallest and the largest value of
//! the window (the levels are fractions of that swing). The AC measures that look for a
//! crossing interpolate between the points of the sweep, in the logarithm of the frequency, so
//! the sweep needs a few tens of points per decade for four digits; the gain and phase at a
//! frequency are computed at that frequency itself.
//!
//! A measure that is well formed but finds nothing (a level that is never crossed, a bandwidth
//! that is never reached) has no value: [`None`], printed as an empty cell or `null`.

use eh3d_phoenix_rf::Complex;

use crate::codes::{SimError, MEASURE_INVALID};
use crate::units::parse_number;

/// What the measure is taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// The transient analysis, against time.
    Tran,
    /// The DC sweep, against the value of the swept source.
    Dc,
    /// The AC sweep, against frequency.
    Ac,
}

/// The direction of a crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// From below the level to the level or above.
    Rise,
    /// From above the level to the level or below.
    Fall,
    /// Either.
    Any,
}

/// The gain the bandwidth is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// The gain at the first frequency of the sweep (a low-pass response).
    First,
    /// The largest gain (a band-pass response: the width between the two -3 dB frequencies).
    Max,
}

/// A window of the horizontal axis; a missing end is the end of the data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    /// Where the window starts.
    pub from: Option<f64>,
    /// Where the window ends.
    pub to: Option<f64>,
}

/// What a measure computes.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasureKind {
    /// The value at a time (or swept value).
    At(f64),
    /// The largest value in a window.
    Max(Window),
    /// The smallest value in a window.
    Min(Window),
    /// The mean over a window.
    Avg(Window),
    /// The root mean square over a window.
    Rms(Window),
    /// The largest minus the smallest value in a window.
    PeakToPeak(Window),
    /// The time of the `n`-th crossing of a level after `from`.
    Cross {
        /// The level.
        level: f64,
        /// The direction.
        edge: Edge,
        /// Which crossing, counted from 1.
        n: usize,
        /// Where the search starts.
        from: Option<f64>,
    },
    /// The rise time between two fractions of the swing of a window.
    RiseTime {
        /// The fraction where the rise starts, when it is not the one of the settings.
        low: Option<f64>,
        /// The fraction where the rise ends, when it is not the one of the settings.
        high: Option<f64>,
        /// The window.
        window: Window,
    },
    /// The fall time between two fractions of the swing of a window.
    FallTime {
        /// The fraction where the fall starts (the high one), when it is not the one of the
        /// settings.
        low: Option<f64>,
        /// The fraction where the fall ends, when it is not the one of the settings.
        high: Option<f64>,
        /// The window.
        window: Window,
    },
    /// The magnitude at a frequency.
    Gain(f64),
    /// The magnitude in dB at a frequency.
    GainDb(f64),
    /// The phase in degrees at a frequency.
    Phase(f64),
    /// The -3 dB bandwidth.
    Bandwidth(Reference),
    /// The largest magnitude.
    Peak,
    /// The frequency of the largest magnitude.
    PeakFreq,
    /// The quality factor of a band-pass response: the peak frequency over the -3 dB bandwidth.
    QBand,
    /// The quality factor of a second-order low-pass response: the gain at the frequency where
    /// the phase is -90 degrees over the gain at the first frequency.
    QLow,
    /// The frequency where the magnitude falls through 1.
    UnityGain,
    /// The phase margin: 180 degrees plus the phase at the unity-gain frequency.
    PhaseMargin,
    /// The gain margin in dB: minus the gain where the phase falls through -180 degrees.
    GainMargin,
}

/// One measure: a name, what to measure and on which signal.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureSpec {
    /// The name, as written.
    pub name: String,
    /// The analysis it needs.
    pub domain: Domain,
    /// The signal, in lower case (`v(out)`, `i(v1)`).
    pub signal: String,
    /// What to compute.
    pub kind: MeasureKind,
    /// The line as written, normalised (single spaces, no space around `=`); it is what the
    /// writer of netlists prints after `.measure`.
    pub text: String,
}

fn error(line: Option<usize>, why: String) -> SimError {
    match line {
        Some(number) => SimError::at(MEASURE_INVALID, number, why),
        None => SimError::new(MEASURE_INVALID, why),
    }
}

/// The text with no space around `=` and a single space between words.
fn normalise(text: &str) -> String {
    let mut joined = String::new();
    for word in text.split_whitespace() {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(word);
    }
    let mut out = String::new();
    let mut chars = joined.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ' ' {
            // a space before or after an equals sign is dropped
            let next_is_equals = chars.peek() == Some(&'=');
            let previous_is_equals = out.ends_with('=');
            if next_is_equals || previous_is_equals {
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// The words of a measure after its signal, split into the ones with `=` and the others.
struct Args {
    positional: Vec<String>,
    keyed: Vec<(String, String)>,
}

impl Args {
    fn new(words: &[&str]) -> Args {
        let mut positional = Vec::new();
        let mut keyed = Vec::new();
        for word in words {
            match word.split_once('=') {
                Some((key, value)) => keyed.push((key.to_ascii_lowercase(), value.to_string())),
                None => positional.push(word.to_string()),
            }
        }
        positional.reverse();
        Args { positional, keyed }
    }

    /// The next positional word (the words are taken in the order they were written).
    fn take_word(&mut self) -> Option<String> {
        self.positional.pop()
    }

    /// The value of a key, which is then used up.
    fn key(&mut self, name: &str) -> Option<String> {
        let at = self.keyed.iter().position(|(k, _)| k == name)?;
        Some(self.keyed.remove(at).1)
    }
}

fn number(word: &str, what: &str, line: Option<usize>, spec: &str) -> Result<f64, SimError> {
    parse_number(word)
        .ok_or_else(|| error(line, format!("{spec}: {what} '{word}' is not a number")))
}

fn optional_key(
    args: &mut Args,
    name: &str,
    line: Option<usize>,
    spec: &str,
) -> Result<Option<f64>, SimError> {
    match args.key(name) {
        Some(word) => Ok(Some(number(&word, name, line, spec)?)),
        None => Ok(None),
    }
}

/// Reads a measure from its text (without `.measure`). `line` is the line of the netlist when
/// there is one.
pub fn parse_measure(text: &str, line: Option<usize>) -> Result<MeasureSpec, SimError> {
    let normalised = normalise(text);
    let words: Vec<&str> = normalised.split_whitespace().collect();
    if words.len() < 4 {
        return Err(error(
            line,
            "a measure needs a domain (tran, dc or ac), a name, a kind and a signal".to_string(),
        ));
    }
    let domain = match words[0].to_ascii_lowercase().as_str() {
        "tran" => Domain::Tran,
        "dc" => Domain::Dc,
        "ac" => Domain::Ac,
        other => {
            return Err(error(
                line,
                format!("'{other}' is not tran, dc or ac (the analysis the measure reads)"),
            ))
        }
    };
    let name = words[1].to_string();
    let spec = format!("measure {name}");
    if !name.starts_with(|c: char| c.is_alphabetic())
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return Err(error(
            line,
            format!("'{name}' is not a name (letters, digits, _ and ., starting with a letter)"),
        ));
    }
    let kind_word = words[2].to_ascii_lowercase();
    let signal = words[3].to_ascii_lowercase();
    if !(signal.ends_with(')') && (signal.starts_with("v(") || signal.starts_with("i("))) {
        return Err(error(
            line,
            format!("{spec}: the signal '{signal}' is not v(node) or i(source)"),
        ));
    }
    let mut args = Args::new(&words[4..]);
    let window = |args: &mut Args| -> Result<Window, SimError> {
        Ok(Window {
            from: optional_key(args, "from", line, &spec)?,
            to: optional_key(args, "to", line, &spec)?,
        })
    };
    let kind = match (domain, kind_word.as_str()) {
        (Domain::Tran | Domain::Dc, "at") => {
            let word = args
                .take_word()
                .ok_or_else(|| error(line, format!("{spec}: at needs a time")))?;
            MeasureKind::At(number(&word, "the time", line, &spec)?)
        }
        (Domain::Tran | Domain::Dc, "max") => MeasureKind::Max(window(&mut args)?),
        (Domain::Tran | Domain::Dc, "min") => MeasureKind::Min(window(&mut args)?),
        (Domain::Tran | Domain::Dc, "avg") => MeasureKind::Avg(window(&mut args)?),
        (Domain::Tran | Domain::Dc, "rms") => MeasureKind::Rms(window(&mut args)?),
        (Domain::Tran | Domain::Dc, "pp") => MeasureKind::PeakToPeak(window(&mut args)?),
        (Domain::Tran | Domain::Dc, "cross") => {
            let word = args
                .take_word()
                .ok_or_else(|| error(line, format!("{spec}: cross needs a level")))?;
            let level = number(&word, "the level", line, &spec)?;
            let mut edge = Edge::Any;
            if let Some(direction) = args.take_word() {
                edge = match direction.to_ascii_lowercase().as_str() {
                    "rise" => Edge::Rise,
                    "fall" => Edge::Fall,
                    "any" => Edge::Any,
                    other => {
                        return Err(error(
                            line,
                            format!("{spec}: '{other}' is not rise, fall or any"),
                        ))
                    }
                };
            }
            let n = match optional_key(&mut args, "n", line, &spec)? {
                Some(v) if v >= 1.0 && v.fract() == 0.0 && v <= 1e6 => v as usize,
                Some(_) => {
                    return Err(error(
                        line,
                        format!("{spec}: n is not a whole number above zero"),
                    ))
                }
                None => 1,
            };
            let from = optional_key(&mut args, "from", line, &spec)?;
            MeasureKind::Cross {
                level,
                edge,
                n,
                from,
            }
        }
        (Domain::Tran | Domain::Dc, word @ ("rise" | "fall")) => {
            let low = optional_key(&mut args, "low", line, &spec)?;
            let high = optional_key(&mut args, "high", line, &spec)?;
            let window = window(&mut args)?;
            if let (Some(l), Some(h)) = (low, high) {
                if !(l > 0.0 && l < h && h < 1.0) {
                    return Err(error(
                        line,
                        format!("{spec}: low and high must satisfy 0 < low < high < 1"),
                    ));
                }
            }
            for fraction in [low, high].into_iter().flatten() {
                if !(fraction > 0.0 && fraction < 1.0) {
                    return Err(error(
                        line,
                        format!("{spec}: low and high are fractions between 0 and 1"),
                    ));
                }
            }
            if word == "rise" {
                MeasureKind::RiseTime { low, high, window }
            } else {
                MeasureKind::FallTime { low, high, window }
            }
        }
        (Domain::Ac, "gain" | "gaindb" | "phase") => {
            let word = args
                .take_word()
                .ok_or_else(|| error(line, format!("{spec}: {kind_word} needs a frequency")))?;
            let freq = number(&word, "the frequency", line, &spec)?;
            if freq <= 0.0 {
                return Err(error(
                    line,
                    format!("{spec}: the frequency must be above zero"),
                ));
            }
            match kind_word.as_str() {
                "gain" => MeasureKind::Gain(freq),
                "gaindb" => MeasureKind::GainDb(freq),
                _ => MeasureKind::Phase(freq),
            }
        }
        (Domain::Ac, "bw") => {
            let reference = match args.key("ref") {
                None => Reference::First,
                Some(word) => match word.to_ascii_lowercase().as_str() {
                    "first" => Reference::First,
                    "max" => Reference::Max,
                    other => {
                        return Err(error(
                            line,
                            format!("{spec}: ref is first or max, not '{other}'"),
                        ))
                    }
                },
            };
            MeasureKind::Bandwidth(reference)
        }
        (Domain::Ac, "peak") => MeasureKind::Peak,
        (Domain::Ac, "peakfreq") => MeasureKind::PeakFreq,
        (Domain::Ac, "qband") => MeasureKind::QBand,
        (Domain::Ac, "qlow") => MeasureKind::QLow,
        (Domain::Ac, "ugf") => MeasureKind::UnityGain,
        (Domain::Ac, "phasemargin") => MeasureKind::PhaseMargin,
        (Domain::Ac, "gainmargin") => MeasureKind::GainMargin,
        (_, other) => {
            return Err(error(
                line,
                format!("{spec}: '{other}' is not a measure of this analysis"),
            ))
        }
    };
    if let Some(extra) = args.take_word() {
        return Err(error(
            line,
            format!("{spec}: '{extra}' is not expected here"),
        ));
    }
    if let Some((key, _)) = args.keyed.first() {
        return Err(error(
            line,
            format!("{spec}: the option '{key}' is not used by {kind_word}"),
        ));
    }
    Ok(MeasureSpec {
        name,
        domain,
        signal,
        kind,
        text: normalised,
    })
}

// ---------------------------------------------------------------------------------------------
// Waveforms: time or swept value against a real signal
// ---------------------------------------------------------------------------------------------

/// The value of the signal at `at`, with a straight line between the points; `None` outside the
/// data.
fn interpolate(x: &[f64], y: &[f64], at: f64) -> Option<f64> {
    if x.is_empty() || x.len() != y.len() {
        return None;
    }
    if at < x[0] || at > x[x.len() - 1] {
        return None;
    }
    let i = x.partition_point(|v| *v < at);
    if i == 0 {
        return Some(y[0]);
    }
    let (x0, x1, y0, y1) = (x[i - 1], x[i], y[i - 1], y[i]);
    if x1 == x0 {
        return Some(y1);
    }
    Some(y0 + (y1 - y0) * (at - x0) / (x1 - x0))
}

/// The points of the window, with the ends interpolated.
fn clip(x: &[f64], y: &[f64], window: Window) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let (Some(first), Some(last)) = (x.first(), x.last()) else {
        return out;
    };
    let from = window.from.unwrap_or(*first).max(*first);
    let to = window.to.unwrap_or(*last).min(*last);
    if from > to {
        return out;
    }
    if let Some(v) = interpolate(x, y, from) {
        out.push((from, v));
    }
    for (xi, yi) in x.iter().zip(y.iter()) {
        if *xi > from && *xi < to {
            out.push((*xi, *yi));
        }
    }
    if to > from {
        if let Some(v) = interpolate(x, y, to) {
            out.push((to, v));
        }
    }
    out
}

/// The time of the `n`-th crossing of `level` at or after `from`.
fn crossing(x: &[f64], y: &[f64], level: f64, edge: Edge, n: usize, from: f64) -> Option<f64> {
    let mut seen = 0usize;
    for (xs, ys) in x.windows(2).zip(y.windows(2)) {
        if xs[1] < from {
            continue;
        }
        let (y0, y1) = (ys[0], ys[1]);
        let rising = y0 < level && y1 >= level;
        let falling = y0 > level && y1 <= level;
        let hit = match edge {
            Edge::Rise => rising,
            Edge::Fall => falling,
            Edge::Any => rising || falling,
        };
        if !hit {
            continue;
        }
        let at = if y1 == y0 {
            xs[1]
        } else {
            xs[0] + (level - y0) * (xs[1] - xs[0]) / (y1 - y0)
        };
        if at < from {
            continue;
        }
        seen += 1;
        if seen == n {
            return Some(at);
        }
    }
    None
}

/// The time an edge takes between two fractions of the swing of the window.
fn edge_time(
    x: &[f64],
    y: &[f64],
    window: Window,
    low: f64,
    high: f64,
    rising: bool,
) -> Option<f64> {
    let points = clip(x, y, window);
    if points.len() < 2 {
        return None;
    }
    let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let swing = hi - lo;
    if !(swing > 0.0 && swing.is_finite()) {
        return None;
    }
    let a = lo + low * swing;
    let b = lo + high * swing;
    if rising {
        let t_a = crossing(&xs, &ys, a, Edge::Rise, 1, xs[0])?;
        let t_b = crossing(&xs, &ys, b, Edge::Rise, 1, t_a)?;
        Some(t_b - t_a)
    } else {
        let t_b = crossing(&xs, &ys, b, Edge::Fall, 1, xs[0])?;
        let t_a = crossing(&xs, &ys, a, Edge::Fall, 1, t_b)?;
        Some(t_a - t_b)
    }
}

/// The integral of the signal over the points, with straight lines between them, and the integral
/// of its square (exact for those lines).
fn integrals(points: &[(f64, f64)]) -> (f64, f64) {
    let mut area = 0.0;
    let mut square = 0.0;
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let h = x1 - x0;
        area += h * (y0 + y1) / 2.0;
        square += h * (y0 * y0 + y0 * y1 + y1 * y1) / 3.0;
    }
    (area, square)
}

/// A measure of a waveform (`tran` or `dc`): `x` ascending (a descending sweep is turned round
/// first), `y` the signal. `low` and `high` are the fractions of the settings for the rise and
/// fall times. `None` when the measure finds nothing.
pub fn waveform(kind: &MeasureKind, x: &[f64], y: &[f64], low: f64, high: f64) -> Option<f64> {
    let descending = x.len() > 1 && x[0] > x[x.len() - 1];
    let (x, y): (Vec<f64>, Vec<f64>) = if descending {
        (
            x.iter().rev().copied().collect(),
            y.iter().rev().copied().collect(),
        )
    } else {
        (x.to_vec(), y.to_vec())
    };
    match kind {
        MeasureKind::At(at) => interpolate(&x, &y, *at),
        MeasureKind::Max(w) => clip(&x, &y, *w)
            .iter()
            .map(|p| p.1)
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a| a.max(v)))
            }),
        MeasureKind::Min(w) => clip(&x, &y, *w)
            .iter()
            .map(|p| p.1)
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a| a.min(v)))
            }),
        MeasureKind::PeakToPeak(w) => {
            let points = clip(&x, &y, *w);
            if points.is_empty() {
                return None;
            }
            let hi = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            let lo = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            Some(hi - lo)
        }
        MeasureKind::Avg(w) | MeasureKind::Rms(w) => {
            let points = clip(&x, &y, *w);
            let span = points.last()?.0 - points.first()?.0;
            if span <= 0.0 {
                return points.first().map(|p| p.1);
            }
            let (area, square) = integrals(&points);
            if matches!(kind, MeasureKind::Avg(_)) {
                Some(area / span)
            } else {
                Some((square / span).sqrt())
            }
        }
        MeasureKind::Cross {
            level,
            edge,
            n,
            from,
        } => crossing(&x, &y, *level, *edge, *n, from.unwrap_or(f64::NEG_INFINITY)),
        MeasureKind::RiseTime {
            low: l,
            high: h,
            window,
        } => edge_time(&x, &y, *window, l.unwrap_or(low), h.unwrap_or(high), true),
        MeasureKind::FallTime {
            low: l,
            high: h,
            window,
        } => edge_time(&x, &y, *window, l.unwrap_or(low), h.unwrap_or(high), false),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// AC responses: frequency against a complex signal
// ---------------------------------------------------------------------------------------------

fn db(magnitude: f64) -> f64 {
    20.0 * magnitude.log10()
}

/// The value of an AC measure at one frequency, from the signal there; `None` for a kind that
/// needs the whole sweep.
pub fn ac_point(kind: &MeasureKind, value: Complex) -> Option<f64> {
    match kind {
        MeasureKind::Gain(_) => Some(value.abs()),
        MeasureKind::GainDb(_) => Some(db(value.abs())),
        MeasureKind::Phase(_) => Some(value.arg_deg()),
        _ => None,
    }
}

/// The frequency of an AC measure that is taken at one frequency.
pub fn ac_point_freq(kind: &MeasureKind) -> Option<f64> {
    match kind {
        MeasureKind::Gain(f) | MeasureKind::GainDb(f) | MeasureKind::Phase(f) => Some(*f),
        _ => None,
    }
}

/// The phase of a response in degrees with the jumps of 360 degrees taken out, starting from the
/// principal value of the first point.
pub fn unwrap_degrees(values: &[Complex]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(values.len());
    for value in values {
        let mut phase = value.arg_deg();
        if let Some(previous) = out.last() {
            while phase - previous > 180.0 {
                phase -= 360.0;
            }
            while phase - previous < -180.0 {
                phase += 360.0;
            }
        }
        out.push(phase);
    }
    out
}

/// The frequencies where `q` goes through `level`, rising or falling, by a straight line between
/// the points in the logarithm of the frequency.
fn level_crossings(freqs: &[f64], q: &[f64], level: f64, rising: bool) -> Vec<f64> {
    let mut out = Vec::new();
    for (f, v) in freqs.windows(2).zip(q.windows(2)) {
        let hit = if rising {
            v[0] < level && v[1] >= level
        } else {
            v[0] > level && v[1] <= level
        };
        if !hit {
            continue;
        }
        let fraction = if v[1] == v[0] {
            1.0
        } else {
            (level - v[0]) / (v[1] - v[0])
        };
        let log_f = f[0].ln() + fraction * (f[1].ln() - f[0].ln());
        out.push(log_f.exp());
    }
    out
}

/// The value of `q` at a frequency, by a straight line in the logarithm of the frequency.
fn value_at(freqs: &[f64], q: &[f64], freq: f64) -> Option<f64> {
    let logs: Vec<f64> = freqs.iter().map(|f| f.ln()).collect();
    interpolate(&logs, q, freq.ln())
}

/// The largest magnitude: the frequency and the magnitude in dB, refined by the parabola through
/// the point and its two neighbours (in dB against the logarithm of the frequency).
fn peak_point(freqs: &[f64], mags_db: &[f64]) -> Option<(f64, f64)> {
    let mut best = 0usize;
    for (k, v) in mags_db.iter().enumerate() {
        if *v > mags_db[best] {
            best = k;
        }
    }
    if mags_db.is_empty() || !mags_db[best].is_finite() {
        return None;
    }
    if best == 0 || best + 1 >= mags_db.len() {
        return Some((freqs[best], mags_db[best]));
    }
    let x = [freqs[best - 1].ln(), freqs[best].ln(), freqs[best + 1].ln()];
    let y = [mags_db[best - 1], mags_db[best], mags_db[best + 1]];
    let d0 = (x[0] - x[1]) * (x[0] - x[2]);
    let d1 = (x[1] - x[0]) * (x[1] - x[2]);
    let d2 = (x[2] - x[0]) * (x[2] - x[1]);
    let a = y[0] / d0 + y[1] / d1 + y[2] / d2;
    let b = -y[0] * (x[1] + x[2]) / d0 - y[1] * (x[0] + x[2]) / d1 - y[2] * (x[0] + x[1]) / d2;
    let c = y[0] * x[1] * x[2] / d0 + y[1] * x[0] * x[2] / d1 + y[2] * x[0] * x[1] / d2;
    if !(a.is_finite() && a < 0.0) {
        return Some((freqs[best], mags_db[best]));
    }
    let vertex = -b / (2.0 * a);
    if vertex < x[0] || vertex > x[2] {
        return Some((freqs[best], mags_db[best]));
    }
    let top = a * vertex * vertex + b * vertex + c;
    Some((vertex.exp(), top))
}

/// The -3 dB bandwidth of a response.
fn bandwidth(freqs: &[f64], mags_db: &[f64], reference: Reference) -> Option<f64> {
    match reference {
        Reference::First => {
            let level = *mags_db.first()? - 20.0 * std::f64::consts::SQRT_2.log10();
            level_crossings(freqs, mags_db, level, false)
                .first()
                .copied()
        }
        Reference::Max => {
            let (peak_f, peak_db) = peak_point(freqs, mags_db)?;
            let level = peak_db - 20.0 * std::f64::consts::SQRT_2.log10();
            let upper = level_crossings(freqs, mags_db, level, false)
                .into_iter()
                .find(|f| *f > peak_f)?;
            let lower = level_crossings(freqs, mags_db, level, true)
                .into_iter()
                .rfind(|f| *f < peak_f)?;
            Some(upper - lower)
        }
    }
}

/// An AC measure that needs the whole sweep: `freqs` ascending, `values` the signal. `None` when
/// the measure finds nothing, and for a kind that is taken at one frequency.
pub fn ac_sweep(kind: &MeasureKind, freqs: &[f64], values: &[Complex]) -> Option<f64> {
    if freqs.len() < 2 || freqs.len() != values.len() {
        return None;
    }
    let mags_db: Vec<f64> = values.iter().map(|v| db(v.abs())).collect();
    match kind {
        MeasureKind::Bandwidth(reference) => bandwidth(freqs, &mags_db, *reference),
        MeasureKind::Peak => peak_point(freqs, &mags_db).map(|p| 10f64.powf(p.1 / 20.0)),
        MeasureKind::PeakFreq => peak_point(freqs, &mags_db).map(|p| p.0),
        MeasureKind::QBand => {
            let (peak_f, _) = peak_point(freqs, &mags_db)?;
            let width = bandwidth(freqs, &mags_db, Reference::Max)?;
            if width > 0.0 {
                Some(peak_f / width)
            } else {
                None
            }
        }
        MeasureKind::QLow => {
            let phase = unwrap_degrees(values);
            let f90 = *level_crossings(freqs, &phase, -90.0, false).first()?;
            let at_f90 = value_at(freqs, &mags_db, f90)?;
            Some(10f64.powf((at_f90 - mags_db[0]) / 20.0))
        }
        MeasureKind::UnityGain => level_crossings(freqs, &mags_db, 0.0, false)
            .first()
            .copied(),
        MeasureKind::PhaseMargin => {
            let unity = *level_crossings(freqs, &mags_db, 0.0, false).first()?;
            let phase = unwrap_degrees(values);
            Some(180.0 + value_at(freqs, &phase, unity)?)
        }
        MeasureKind::GainMargin => {
            let phase = unwrap_degrees(values);
            let f180 = *level_crossings(freqs, &phase, -180.0, false).first()?;
            Some(-value_at(freqs, &mags_db, f180)?)
        }
        _ => None,
    }
}

/// The figures of merit of a response, with the unit of each.
pub const FIGURE_NAMES: [(&str, &str); 9] = [
    ("dc_gain", ""),
    ("peak_gain", ""),
    ("peak_freq", "Hz"),
    ("cutoff_3db", "Hz"),
    ("q_lowpass", ""),
    ("q_bandpass", ""),
    ("unity_gain_freq", "Hz"),
    ("phase_margin", "deg"),
    ("gain_margin", "dB"),
];

/// The figures of merit of a response, in the order of [`FIGURE_NAMES`]: the gain at the first
/// frequency, the peak gain and its frequency, the -3 dB cutoff of a low-pass response, the
/// quality factors of a second-order low-pass and of a band-pass response, the unity-gain
/// frequency and the phase and gain margins. A figure that does not exist for the response is
/// `None`.
pub fn figures(freqs: &[f64], values: &[Complex]) -> Vec<Option<f64>> {
    let dc_gain = values.first().map(|v| v.abs());
    vec![
        dc_gain,
        ac_sweep(&MeasureKind::Peak, freqs, values),
        ac_sweep(&MeasureKind::PeakFreq, freqs, values),
        ac_sweep(&MeasureKind::Bandwidth(Reference::First), freqs, values),
        ac_sweep(&MeasureKind::QLow, freqs, values),
        ac_sweep(&MeasureKind::QBand, freqs, values),
        ac_sweep(&MeasureKind::UnityGain, freqs, values),
        ac_sweep(&MeasureKind::PhaseMargin, freqs, values),
        ac_sweep(&MeasureKind::GainMargin, freqs, values),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> MeasureSpec {
        parse_measure(text, None).unwrap()
    }

    fn refused(text: &str) -> SimError {
        parse_measure(text, Some(7)).unwrap_err()
    }

    #[test]
    fn measures_are_read_with_their_options() {
        let m = parsed("tran m1 max v(out) from = 1m to=2m");
        assert_eq!(m.name, "m1");
        assert_eq!(m.domain, Domain::Tran);
        assert_eq!(m.signal, "v(out)");
        assert_eq!(
            m.kind,
            MeasureKind::Max(Window {
                from: Some(1e-3),
                to: Some(2e-3)
            })
        );
        assert_eq!(m.text, "tran m1 max v(out) from=1m to=2m");
        assert_eq!(
            parsed("tran c cross V(OUT) 0.5 fall n=2").kind,
            MeasureKind::Cross {
                level: 0.5,
                edge: Edge::Fall,
                n: 2,
                from: None
            }
        );
        assert_eq!(
            parsed("ac b bw v(out) ref=max").kind,
            MeasureKind::Bandwidth(Reference::Max)
        );
        assert_eq!(
            parsed("ac g gain v(out) 1k").kind,
            MeasureKind::Gain(1000.0)
        );
        assert_eq!(parsed("dc a at v(out) 2.5").kind, MeasureKind::At(2.5));
        // writing the text again and reading it gives the same measure
        assert_eq!(parse_measure(&m.text, None).unwrap(), m);
    }

    #[test]
    fn a_measure_that_is_not_valid_is_refused_with_its_line() {
        for text in [
            "tran m1",
            "xyz m1 max v(out)",
            "tran 1m max v(out)",
            "tran m1 max out",
            "tran m1 at v(out)",
            "tran m1 at v(out) x",
            "tran m1 cross v(out) 1 sideways",
            "tran m1 cross v(out) 1 rise n=0",
            "tran m1 max v(out) when=3",
            "tran m1 max v(out) extra",
            "tran m1 rise v(out) low=0.9 high=0.1",
            "tran m1 gain v(out) 1k",
            "ac m1 max v(out)",
            "ac m1 gain v(out)",
            "ac m1 gain v(out) -1",
            "ac m1 bw v(out) ref=third",
        ] {
            let e = refused(text);
            assert_eq!(e.code, MEASURE_INVALID, "{text}");
            assert_eq!(e.line, Some(7), "{text}");
        }
    }

    fn ramp() -> (Vec<f64>, Vec<f64>) {
        // 0 to 1 over the first second, then 1 to 0 over the next one
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 0.0];
        (x, y)
    }

    #[test]
    fn values_extremes_mean_and_rms_of_a_waveform() {
        let (x, y) = ramp();
        let all = Window {
            from: None,
            to: None,
        };
        assert_eq!(waveform(&MeasureKind::At(0.5), &x, &y, 0.1, 0.9), Some(0.5));
        assert_eq!(waveform(&MeasureKind::At(3.0), &x, &y, 0.1, 0.9), None);
        assert_eq!(
            waveform(&MeasureKind::Max(all), &x, &y, 0.1, 0.9),
            Some(1.0)
        );
        assert_eq!(
            waveform(&MeasureKind::Min(all), &x, &y, 0.1, 0.9),
            Some(0.0)
        );
        assert_eq!(
            waveform(&MeasureKind::PeakToPeak(all), &x, &y, 0.1, 0.9),
            Some(1.0)
        );
        // a triangle: mean 1/2, and the mean of the square is 1/3
        let avg = waveform(&MeasureKind::Avg(all), &x, &y, 0.1, 0.9).unwrap();
        assert!((avg - 0.5).abs() < 1e-12, "{avg}");
        let rms = waveform(&MeasureKind::Rms(all), &x, &y, 0.1, 0.9).unwrap();
        assert!((rms - (1.0f64 / 3.0).sqrt()).abs() < 1e-12, "{rms}");
        // a window that cuts the ramp: from 0.5 to 1.5 the mean is 0.75
        let cut = Window {
            from: Some(0.5),
            to: Some(1.5),
        };
        let avg = waveform(&MeasureKind::Avg(cut), &x, &y, 0.1, 0.9).unwrap();
        assert!((avg - 0.75).abs() < 1e-12, "{avg}");
        assert_eq!(
            waveform(&MeasureKind::Max(cut), &x, &y, 0.1, 0.9),
            Some(1.0)
        );
    }

    #[test]
    fn crossings_are_found_by_direction_and_count() {
        let (x, y) = ramp();
        let cross = |edge: Edge, n: usize, from: Option<f64>| {
            waveform(
                &MeasureKind::Cross {
                    level: 0.5,
                    edge,
                    n,
                    from,
                },
                &x,
                &y,
                0.1,
                0.9,
            )
        };
        assert_eq!(cross(Edge::Rise, 1, None), Some(0.5));
        assert_eq!(cross(Edge::Fall, 1, None), Some(1.5));
        assert_eq!(cross(Edge::Any, 2, None), Some(1.5));
        assert_eq!(cross(Edge::Rise, 2, None), None);
        assert_eq!(cross(Edge::Any, 1, Some(1.0)), Some(1.5));
    }

    #[test]
    fn rise_and_fall_times_use_fractions_of_the_swing() {
        // a linear rise from 1 s to 2 s, between 0 and 10: the 10 % to 90 % time is 0.8 s
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 0.0, 10.0, 10.0, 0.0];
        let all = Window {
            from: None,
            to: None,
        };
        let rise = waveform(
            &MeasureKind::RiseTime {
                low: None,
                high: None,
                window: all,
            },
            &x,
            &y,
            0.1,
            0.9,
        )
        .unwrap();
        assert!((rise - 0.8).abs() < 1e-12, "{rise}");
        let fall = waveform(
            &MeasureKind::FallTime {
                low: Some(0.2),
                high: None,
                window: all,
            },
            &x,
            &y,
            0.1,
            0.9,
        )
        .unwrap();
        // the fall from 3 s to 4 s: from 90 % to 20 % takes 0.7 s
        assert!((fall - 0.7).abs() < 1e-12, "{fall}");
    }

    #[test]
    fn a_descending_sweep_is_read_in_ascending_order() {
        let x = vec![2.0, 1.0, 0.0];
        let y = vec![4.0, 1.0, 0.0];
        assert_eq!(waveform(&MeasureKind::At(1.0), &x, &y, 0.1, 0.9), Some(1.0));
    }

    fn lowpass(fc: f64, points_per_decade: usize) -> (Vec<f64>, Vec<Complex>) {
        let count = 4 * points_per_decade;
        let freqs: Vec<f64> = (0..=count)
            .map(|k| fc / 100.0 * 10f64.powf(k as f64 / points_per_decade as f64))
            .collect();
        let values = freqs
            .iter()
            .map(|f| Complex::ONE / Complex::new(1.0, f / fc))
            .collect();
        (freqs, values)
    }

    #[test]
    fn the_cutoff_of_a_first_order_low_pass_is_found() {
        let (freqs, values) = lowpass(1000.0, 100);
        let fc = ac_sweep(&MeasureKind::Bandwidth(Reference::First), &freqs, &values).unwrap();
        assert!((fc / 1000.0 - 1.0).abs() < 1e-3, "{fc}");
        // no peak above the first point: the peak is at the start of the sweep
        let peak = ac_sweep(&MeasureKind::Peak, &freqs, &values).unwrap();
        assert!((peak - 1.0).abs() < 1e-3, "{peak}");
    }

    #[test]
    fn the_quality_factor_of_a_second_order_low_pass_is_found_from_its_phase() {
        let fc = 1000.0;
        let q = 2.0;
        let freqs: Vec<f64> = (0..=400)
            .map(|k| 10.0 * 10f64.powf(k as f64 / 100.0))
            .collect();
        let values: Vec<Complex> = freqs
            .iter()
            .map(|f| {
                let x = f / fc;
                Complex::ONE / Complex::new(1.0 - x * x, x / q)
            })
            .collect();
        let found = ac_sweep(&MeasureKind::QLow, &freqs, &values).unwrap();
        assert!((found / q - 1.0).abs() < 3e-3, "{found}");
        // the peak of a second-order low-pass is Q / sqrt(1 - 1/(4 Q^2))
        let peak = ac_sweep(&MeasureKind::Peak, &freqs, &values).unwrap();
        let want = q / (1.0 - 1.0 / (4.0 * q * q)).sqrt();
        assert!((peak / want - 1.0).abs() < 1e-3, "{peak} {want}");
    }

    #[test]
    fn the_margins_of_a_three_pole_loop_are_found() {
        // L(s) = 4 / (1 + s/w0)^3: the phase is -180 degrees at tan(60 deg) f0 and the gain
        // there is 4/8, so the gain margin is 6.02 dB
        let f0 = 1000.0;
        let freqs: Vec<f64> = (0..=300)
            .map(|k| 100.0 * 10f64.powf(k as f64 / 100.0))
            .collect();
        let values: Vec<Complex> = freqs
            .iter()
            .map(|f| {
                let one = Complex::new(1.0, f / f0);
                Complex::real(4.0) / (one * one * one)
            })
            .collect();
        let gm = ac_sweep(&MeasureKind::GainMargin, &freqs, &values).unwrap();
        assert!((gm - 20.0 * 2.0f64.log10()).abs() < 5e-3, "{gm}");
        let pm = ac_sweep(&MeasureKind::PhaseMargin, &freqs, &values).unwrap();
        let x = (4.0f64.powf(2.0 / 3.0) - 1.0).sqrt();
        let want = 180.0 - 3.0 * x.atan().to_degrees();
        assert!((pm - want).abs() < 0.05, "{pm} {want}");
    }

    #[test]
    fn a_band_pass_response_has_a_bandwidth_between_its_two_edges() {
        // H = 1 / (1 + j Q (x - 1/x)) with x = f/f0 has the -3 dB width f0/Q
        let (f0, q) = (5000.0, 4.0);
        let freqs: Vec<f64> = (0..=300)
            .map(|k| 500.0 * 10f64.powf(k as f64 / 100.0))
            .collect();
        let values: Vec<Complex> = freqs
            .iter()
            .map(|f| {
                let x = f / f0;
                Complex::ONE / Complex::new(1.0, q * (x - 1.0 / x))
            })
            .collect();
        let width = ac_sweep(&MeasureKind::Bandwidth(Reference::Max), &freqs, &values).unwrap();
        assert!((width / (f0 / q) - 1.0).abs() < 2e-3, "{width}");
        let found_q = ac_sweep(&MeasureKind::QBand, &freqs, &values).unwrap();
        assert!((found_q / q - 1.0).abs() < 3e-3, "{found_q}");
        let peak_f = ac_sweep(&MeasureKind::PeakFreq, &freqs, &values).unwrap();
        assert!((peak_f / f0 - 1.0).abs() < 1e-3, "{peak_f}");
    }

    #[test]
    fn the_gain_and_phase_at_a_frequency_come_from_the_value_there() {
        let v = Complex::new(1.0, -1.0);
        let at = MeasureKind::Gain(10.0);
        assert!((ac_point(&at, v).unwrap() - 2f64.sqrt()).abs() < 1e-12);
        assert_eq!(ac_point_freq(&at), Some(10.0));
        assert!((ac_point(&MeasureKind::Phase(1.0), v).unwrap() + 45.0).abs() < 1e-12);
        assert!((ac_point(&MeasureKind::GainDb(1.0), v).unwrap() - 3.0103).abs() < 1e-4);
        assert_eq!(ac_point(&MeasureKind::Peak, v), None);
        assert_eq!(ac_point_freq(&MeasureKind::Peak), None);
    }

    #[test]
    fn a_response_with_fewer_than_two_points_has_no_sweep_measure() {
        assert_eq!(ac_sweep(&MeasureKind::Peak, &[1.0], &[Complex::ONE]), None);
        assert_eq!(figures(&[1.0], &[Complex::ONE])[1], None);
    }
}
