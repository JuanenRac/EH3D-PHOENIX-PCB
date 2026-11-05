// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader and writer of Touchstone files (`.s1p`, `.s2p`, ... `.sNp`), version 1.0 and 1.1.
//!
//! * The option line `# <unit> <parameter> <format> R <ohm>` gives the frequency unit (Hz, kHz,
//!   MHz or GHz), the kind of parameter (S, Y, Z, H or G), the format of the pairs (RI, MA or DB)
//!   and the reference resistance; the defaults are `GHz S MA R 50`.
//! * Comments start with `!` and run to the end of the line.
//! * A two-port lists its four parameters in the order 11, 21, 12, 22; every other size lists
//!   them by rows. The matrix of a point is always stored by rows here.
//! * A two-port file may end with noise data: frequency, minimum noise figure in dB, the
//!   magnitude and angle of the optimum source reflection and the equivalent noise resistance
//!   normalised to the reference resistance.
//! * The number of ports is not in a version 1 file: it comes from the extension of the name
//!   ([`ports_from_name`]).
//!
//! Not supported: the keyword syntax of version 2 (a line that starts with `[` is refused), a
//! reference resistance per port, and the conversion of H and G parameters.
//!
//! Every error names the line. The sizes of the file are bounded by the settings.

use crate::complex::{clip, Complex};
use crate::error::RfError;
use crate::matrix::CMatrix;
use crate::settings::RfSettings;

/// The unit of the frequencies of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreqUnit {
    /// Hertz.
    Hz,
    /// Kilohertz.
    KHz,
    /// Megahertz.
    MHz,
    /// Gigahertz.
    GHz,
}

impl FreqUnit {
    /// How many hertz one unit is.
    pub fn hertz(self) -> f64 {
        match self {
            FreqUnit::Hz => 1.0,
            FreqUnit::KHz => 1e3,
            FreqUnit::MHz => 1e6,
            FreqUnit::GHz => 1e9,
        }
    }

    fn name(self) -> &'static str {
        match self {
            FreqUnit::Hz => "Hz",
            FreqUnit::KHz => "kHz",
            FreqUnit::MHz => "MHz",
            FreqUnit::GHz => "GHz",
        }
    }
}

/// The kind of parameter in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    /// Scattering parameters.
    S,
    /// Admittance parameters (normalised to the reference resistance).
    Y,
    /// Impedance parameters (normalised to the reference resistance).
    Z,
    /// Hybrid h parameters (two ports).
    H,
    /// Inverse hybrid g parameters (two ports).
    G,
}

impl Parameter {
    fn letter(self) -> &'static str {
        match self {
            Parameter::S => "S",
            Parameter::Y => "Y",
            Parameter::Z => "Z",
            Parameter::H => "H",
            Parameter::G => "G",
        }
    }
}

/// How each complex value is written as a pair of numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Real and imaginary parts.
    Ri,
    /// Magnitude and angle in degrees.
    Ma,
    /// Magnitude in dB (`20 log10`) and angle in degrees.
    Db,
}

impl Format {
    fn name(self) -> &'static str {
        match self {
            Format::Ri => "RI",
            Format::Ma => "MA",
            Format::Db => "DB",
        }
    }

    fn read(self, a: f64, b: f64) -> Option<Complex> {
        match self {
            Format::Ri => Some(Complex::new(a, b)),
            Format::Ma if a >= 0.0 => Some(Complex::from_polar(a, b.to_radians())),
            Format::Ma => None,
            Format::Db => Some(Complex::from_db_deg(a, b)),
        }
    }

    fn write(self, z: Complex) -> Option<(f64, f64)> {
        match self {
            Format::Ri => Some((z.re, z.im)),
            Format::Ma => Some((z.abs(), z.arg_deg())),
            Format::Db => {
                let db = z.db20();
                if db.is_finite() {
                    Some((db, z.arg_deg()))
                } else {
                    None
                }
            }
        }
    }
}

/// One frequency point: the frequency and the matrix of the network at it.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// The frequency in Hz.
    pub freq_hz: f64,
    /// The parameters by rows, `ports * ports` of them, in the kind of the file.
    pub data: Vec<Complex>,
}

/// One point of the noise data of a two-port.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoisePoint {
    /// The frequency in Hz.
    pub freq_hz: f64,
    /// The minimum noise figure in dB.
    pub nf_min_db: f64,
    /// The source reflection coefficient that gives the minimum noise figure.
    pub gamma_opt: Complex,
    /// The equivalent noise resistance divided by the reference resistance.
    pub rn_normalized: f64,
}

/// A network read from, or to be written as, a Touchstone file.
#[derive(Clone, Debug, PartialEq)]
pub struct Touchstone {
    /// The number of ports.
    pub ports: usize,
    /// The kind of parameter.
    pub parameter: Parameter,
    /// The format of the pairs when the file is written.
    pub format: Format,
    /// The unit of the frequencies when the file is written.
    pub unit: FreqUnit,
    /// The reference resistance in ohm.
    pub z_ref_ohm: f64,
    /// The frequency points by increasing frequency.
    pub points: Vec<Point>,
    /// The noise points of a two-port, by increasing frequency.
    pub noise: Vec<NoisePoint>,
    /// The comment lines at the top of the file, without the `!`.
    pub comments: Vec<String>,
}

/// The number of ports from the extension of a file name: `.s1p` is 1, `.s2p` is 2, `.s12p` is
/// 12. `None` when the name does not end like that (the case is ignored).
pub fn ports_from_name(name: &str) -> Option<usize> {
    let lower = name.to_ascii_lowercase();
    let (_, extension) = lower.rsplit_once('.')?;
    let digits = extension.strip_prefix('s')?.strip_suffix('p')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<usize>().ok().filter(|n| *n > 0)
}

fn syntax(line: usize, reason: impl Into<String>) -> RfError {
    RfError::TouchstoneSyntax {
        line,
        reason: reason.into(),
    }
}

fn data_error(line: usize, reason: impl Into<String>) -> RfError {
    RfError::TouchstoneData {
        line,
        reason: reason.into(),
    }
}

/// The position in the file of the `k`-th value of a record of `n` ports, as an index into the
/// matrix by rows (the two-port is listed 11, 21, 12, 22).
fn matrix_index(n: usize, k: usize) -> usize {
    if n == 2 {
        match k {
            1 => 2,
            2 => 1,
            other => other,
        }
    } else {
        k
    }
}

struct Options {
    unit: FreqUnit,
    parameter: Parameter,
    format: Format,
    z_ref_ohm: f64,
}

fn parse_options(content: &str, line: usize) -> Result<Options, RfError> {
    let mut options = Options {
        unit: FreqUnit::GHz,
        parameter: Parameter::S,
        format: Format::Ma,
        z_ref_ohm: 50.0,
    };
    let mut tokens = content.trim_start_matches('#').split_whitespace();
    while let Some(token) = tokens.next() {
        match token.to_ascii_uppercase().as_str() {
            "HZ" => options.unit = FreqUnit::Hz,
            "KHZ" => options.unit = FreqUnit::KHz,
            "MHZ" => options.unit = FreqUnit::MHz,
            "GHZ" => options.unit = FreqUnit::GHz,
            "S" => options.parameter = Parameter::S,
            "Y" => options.parameter = Parameter::Y,
            "Z" => options.parameter = Parameter::Z,
            "H" => options.parameter = Parameter::H,
            "G" => options.parameter = Parameter::G,
            "RI" => options.format = Format::Ri,
            "MA" => options.format = Format::Ma,
            "DB" => options.format = Format::Db,
            "R" => {
                let value = tokens
                    .next()
                    .ok_or_else(|| syntax(line, "R must be followed by the resistance"))?;
                let ohm: f64 = value
                    .parse()
                    .map_err(|_| syntax(line, format!("not a resistance: {}", clip(value))))?;
                if !ohm.is_finite() || ohm <= 0.0 {
                    return Err(data_error(
                        line,
                        "the reference resistance must be above zero",
                    ));
                }
                options.z_ref_ohm = ohm;
            }
            _ => return Err(syntax(line, format!("unknown option {}", clip(token)))),
        }
    }
    Ok(options)
}

impl Touchstone {
    /// Read a file of `ports` ports from its text.
    pub fn parse(text: &str, ports: usize, settings: &RfSettings) -> Result<Touchstone, RfError> {
        if ports == 0 || ports > settings.max_ports {
            return Err(RfError::OutsideLimit {
                field: "number of ports".to_string(),
                value: ports as f64,
                limit: settings.max_ports as f64,
            });
        }
        if text.len() as u64 > settings.max_file_bytes {
            return Err(RfError::OutsideLimit {
                field: "size of the file in bytes".to_string(),
                value: text.len() as f64,
                limit: settings.max_file_bytes as f64,
            });
        }
        let record_len = 1 + 2 * ports * ports;
        let mut parsed = Touchstone {
            ports,
            parameter: Parameter::S,
            format: Format::Ma,
            unit: FreqUnit::GHz,
            z_ref_ohm: 50.0,
            points: Vec::new(),
            noise: Vec::new(),
            comments: Vec::new(),
        };
        let mut options: Option<Options> = None;
        let mut record: Vec<f64> = Vec::new();
        let mut record_line = 0;
        let mut in_noise = false;
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let (content, comment) = match raw.split_once('!') {
                Some((before, after)) => (before.trim(), Some(after)),
                None => (raw.trim(), None),
            };
            if content.is_empty() {
                if let (Some(comment), true) = (comment, parsed.points.is_empty()) {
                    if parsed.comments.len() < 64 {
                        parsed.comments.push(comment.trim().to_string());
                    }
                }
                continue;
            }
            if content.starts_with('[') {
                return Err(syntax(line, "the keywords of version 2 are not supported"));
            }
            if content.starts_with('#') {
                if options.is_none() {
                    options = Some(parse_options(content, line)?);
                }
                continue;
            }
            let options = options.get_or_insert(Options {
                unit: FreqUnit::GHz,
                parameter: Parameter::S,
                format: Format::Ma,
                z_ref_ohm: 50.0,
            });
            let mut numbers: Vec<f64> = Vec::new();
            for token in content.split_whitespace() {
                let value: f64 = token
                    .parse()
                    .map_err(|_| syntax(line, format!("not a number: {}", clip(token))))?;
                if !value.is_finite() {
                    return Err(data_error(line, "a value is not finite"));
                }
                numbers.push(value);
            }
            let scale = options.unit.hertz();
            if in_noise || (ports == 2 && record.is_empty() && numbers.len() == 5) {
                if numbers.len() != 5 {
                    return Err(syntax(line, "a noise line has 5 values"));
                }
                if parsed.points.is_empty() {
                    return Err(data_error(line, "noise data before the network data"));
                }
                in_noise = true;
                if parsed.noise.len() >= settings.max_points {
                    return Err(RfError::OutsideLimit {
                        field: "number of noise points".to_string(),
                        value: parsed.noise.len() as f64 + 1.0,
                        limit: settings.max_points as f64,
                    });
                }
                let freq_hz = numbers[0] * scale;
                if freq_hz < 0.0 || !freq_hz.is_finite() {
                    return Err(data_error(line, "a frequency must be zero or more"));
                }
                if let Some(last) = parsed.noise.last() {
                    if freq_hz <= last.freq_hz {
                        return Err(data_error(line, "noise frequencies must increase"));
                    }
                }
                if numbers[2] < 0.0 || numbers[4] < 0.0 {
                    return Err(data_error(line, "a magnitude or a resistance is negative"));
                }
                parsed.noise.push(NoisePoint {
                    freq_hz,
                    nf_min_db: numbers[1],
                    gamma_opt: Complex::from_polar(numbers[2], numbers[3].to_radians()),
                    rn_normalized: numbers[4],
                });
                continue;
            }
            if record.is_empty() {
                record_line = line;
            }
            record.extend(numbers);
            if record.len() > record_len {
                return Err(syntax(
                    line,
                    format!("a record of {ports} ports has {record_len} values, this one has more"),
                ));
            }
            if record.len() == record_len {
                if parsed.points.len() >= settings.max_points {
                    return Err(RfError::OutsideLimit {
                        field: "number of frequency points".to_string(),
                        value: parsed.points.len() as f64 + 1.0,
                        limit: settings.max_points as f64,
                    });
                }
                let point = build_point(&record, ports, options, record_line)?;
                if let Some(last) = parsed.points.last() {
                    if point.freq_hz <= last.freq_hz {
                        return Err(data_error(record_line, "frequencies must increase"));
                    }
                }
                parsed.points.push(point);
                record.clear();
            }
        }
        if !record.is_empty() {
            return Err(data_error(
                record_line,
                format!("incomplete record: {} of {record_len} values", record.len()),
            ));
        }
        if let Some(options) = options {
            parsed.unit = options.unit;
            parsed.parameter = options.parameter;
            parsed.format = options.format;
            parsed.z_ref_ohm = options.z_ref_ohm;
        }
        if parsed.points.is_empty() {
            return Err(data_error(0, "the file has no data"));
        }
        Ok(parsed)
    }

    /// The lowest and the highest frequency of the file, in Hz.
    pub fn frequency_range_hz(&self) -> Option<(f64, f64)> {
        let first = self.points.first()?.freq_hz;
        let last = self.points.last()?.freq_hz;
        Some((first, last))
    }

    /// The S matrix of one point, converting from the kind of parameter of the file with the
    /// reference resistance of the file.
    pub fn s_matrix(&self, index: usize) -> Result<CMatrix, RfError> {
        let point = self
            .points
            .get(index)
            .ok_or_else(|| data_error(0, format!("there is no point {index}")))?;
        let raw = CMatrix::new(self.ports, point.data.clone())?;
        let z0 = self.z_ref_ohm;
        let id = CMatrix::identity(self.ports);
        match self.parameter {
            Parameter::S => Ok(raw),
            Parameter::Z => {
                // the values are normalised to the reference resistance
                let denominator = raw.plus(&id)?.inverse()?;
                raw.minus(&id)?.times(&denominator)
            }
            Parameter::Y => {
                let denominator = id.plus(&raw)?.inverse()?;
                id.minus(&raw)?.times(&denominator)
            }
            Parameter::H | Parameter::G => Err(data_error(
                0,
                format!(
                    "{} parameters are read but not converted (reference {z0} ohm)",
                    self.parameter.letter()
                ),
            )),
        }
    }

    /// The S matrices of every point.
    pub fn s_matrices(&self) -> Result<Vec<CMatrix>, RfError> {
        (0..self.points.len()).map(|i| self.s_matrix(i)).collect()
    }

    /// The S matrix at a frequency, interpolated linearly (real and imaginary parts) between the
    /// two points around it. A frequency outside the file is refused: nothing is extrapolated.
    pub fn s_matrix_at(&self, freq_hz: f64) -> Result<CMatrix, RfError> {
        let (low, high) = self
            .frequency_range_hz()
            .ok_or_else(|| data_error(0, "the file has no data"))?;
        if !(low..=high).contains(&freq_hz) {
            return Err(RfError::OutsideRange {
                freq_hz,
                low_hz: low,
                high_hz: high,
            });
        }
        let upper = self.points.partition_point(|p| p.freq_hz < freq_hz);
        let upper = upper.min(self.points.len() - 1);
        let lower = upper.saturating_sub(1);
        let (a, b) = (
            self.points.get(lower).map(|p| p.freq_hz).unwrap_or(low),
            self.points.get(upper).map(|p| p.freq_hz).unwrap_or(high),
        );
        let s_upper = self.s_matrix(upper)?;
        if b <= a || freq_hz >= b {
            return Ok(s_upper);
        }
        let s_lower = self.s_matrix(lower)?;
        let t = (freq_hz - a) / (b - a);
        let blended: Vec<Complex> = s_lower
            .data()
            .iter()
            .zip(s_upper.data())
            .map(|(x, y)| *x + (*y - *x).scale(t))
            .collect();
        CMatrix::new(self.ports, blended)
    }

    /// The group delay in seconds of the transmission from port 1 to port 2 (the entry `S21`)
    /// at a frequency, from the phase step between the two points of the file around it.
    pub fn group_delay_s(&self, freq_hz: f64) -> Result<f64, RfError> {
        if self.ports < 2 {
            return Err(RfError::InvalidNetwork {
                reason: "a group delay needs at least two ports".to_string(),
            });
        }
        let (low, high) = self
            .frequency_range_hz()
            .ok_or_else(|| data_error(0, "the file has no data"))?;
        if self.points.len() < 2 {
            return Err(RfError::InvalidNetwork {
                reason: "a group delay needs at least two frequency points".to_string(),
            });
        }
        if !(low..=high).contains(&freq_hz) {
            return Err(RfError::OutsideRange {
                freq_hz,
                low_hz: low,
                high_hz: high,
            });
        }
        let upper = self.points.partition_point(|p| p.freq_hz < freq_hz);
        let upper = upper.clamp(1, self.points.len() - 1);
        let lower = upper - 1;
        let (a, b) = (
            self.points.get(lower).map_or(low, |p| p.freq_hz),
            self.points.get(upper).map_or(high, |p| p.freq_hz),
        );
        let s_a = self.s_matrix(lower)?.get(1, 0);
        let s_b = self.s_matrix(upper)?.get(1, 0);
        crate::network::group_delay_s(s_a, s_b, a, b)
    }

    /// The text of the file. Frequencies are written in the unit of the file, the pairs in its
    /// format; the numbers are written so that reading them back gives the same values for the
    /// RI format.
    pub fn to_text(&self) -> Result<String, RfError> {
        let n = self.ports;
        if n == 0 {
            return Err(RfError::InvalidNetwork {
                reason: "a file needs at least one port".to_string(),
            });
        }
        if !self.z_ref_ohm.is_finite() || self.z_ref_ohm <= 0.0 {
            return Err(RfError::InvalidNumber {
                field: "reference resistance".to_string(),
                reason: "must be a finite number above zero".to_string(),
            });
        }
        if !self.noise.is_empty() && n != 2 {
            return Err(RfError::InvalidNetwork {
                reason: "only a two-port has noise data".to_string(),
            });
        }
        let mut out = String::new();
        for comment in &self.comments {
            out.push_str("! ");
            out.push_str(&comment.replace(['\r', '\n'], " "));
            out.push('\n');
        }
        out.push_str(&format!(
            "# {} {} {} R {}\n",
            self.unit.name(),
            self.parameter.letter(),
            self.format.name(),
            self.z_ref_ohm
        ));
        let scale = self.unit.hertz();
        for point in &self.points {
            if point.data.len() != n * n {
                return Err(RfError::InvalidNetwork {
                    reason: format!(
                        "a point needs {} values, this one has {}",
                        n * n,
                        point.data.len()
                    ),
                });
            }
            let mut pairs: Vec<String> = Vec::with_capacity(n * n);
            for k in 0..n * n {
                let z = point
                    .data
                    .get(matrix_index(n, k))
                    .copied()
                    .unwrap_or(Complex::ZERO);
                let (a, b) = self.format.write(z).ok_or_else(|| RfError::InvalidNumber {
                    field: "value".to_string(),
                    reason: "a zero cannot be written in dB".to_string(),
                })?;
                if !a.is_finite() || !b.is_finite() || !point.freq_hz.is_finite() {
                    return Err(RfError::InvalidNumber {
                        field: "value".to_string(),
                        reason: "must be finite".to_string(),
                    });
                }
                pairs.push(format!("{a} {b}"));
            }
            let freq = point.freq_hz / scale;
            if n <= 2 {
                out.push_str(&format!("{freq} {}\n", pairs.join(" ")));
            } else {
                // by rows, at most four pairs on a line, the frequency on the first line
                let mut first = true;
                for row in pairs.chunks(n) {
                    for part in row.chunks(4) {
                        if first {
                            out.push_str(&format!("{freq} {}\n", part.join(" ")));
                            first = false;
                        } else {
                            out.push_str(&format!("  {}\n", part.join(" ")));
                        }
                    }
                }
            }
        }
        for noise in &self.noise {
            let (mag, angle) = (noise.gamma_opt.abs(), noise.gamma_opt.arg_deg());
            out.push_str(&format!(
                "{} {} {} {} {}\n",
                noise.freq_hz / scale,
                noise.nf_min_db,
                mag,
                angle,
                noise.rn_normalized
            ));
        }
        Ok(out)
    }
}

fn build_point(
    record: &[f64],
    ports: usize,
    options: &Options,
    line: usize,
) -> Result<Point, RfError> {
    let freq_hz = record.first().copied().unwrap_or(f64::NAN) * options.unit.hertz();
    if !freq_hz.is_finite() || freq_hz < 0.0 {
        return Err(data_error(line, "a frequency must be zero or more"));
    }
    let mut data = vec![Complex::ZERO; ports * ports];
    for k in 0..ports * ports {
        let a = record.get(1 + 2 * k).copied().unwrap_or(f64::NAN);
        let b = record.get(2 + 2 * k).copied().unwrap_or(f64::NAN);
        let value = options
            .format
            .read(a, b)
            .filter(|z| z.is_finite())
            .ok_or_else(|| data_error(line, "a magnitude is negative or a value overflows"))?;
        if let Some(slot) = data.get_mut(matrix_index(ports, k)) {
            *slot = value;
        }
    }
    Ok(Point { freq_hz, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    const TWO_PORT: &str = "! a two-port\n! second line\n# MHz S RI R 50\n\
        100 0.1 -0.2 0.9 0.1 0.9 0.1 0.2 0.0\n\
        200 0.2 -0.3 0.8 0.2 0.8 0.2 0.3 0.1\n";

    #[test]
    fn a_two_port_is_read_with_its_order_and_unit() {
        let t = Touchstone::parse(TWO_PORT, 2, &settings()).unwrap();
        assert_eq!(
            (t.ports, t.parameter, t.format, t.unit),
            (2, Parameter::S, Format::Ri, FreqUnit::MHz)
        );
        assert_eq!(t.z_ref_ohm, 50.0);
        assert_eq!(t.comments, vec!["a two-port", "second line"]);
        assert_eq!(t.points.len(), 2);
        let p = &t.points[0];
        assert_eq!(p.freq_hz, 100e6);
        // the file lists S11 S21 S12 S22
        assert_eq!(p.data[0], Complex::new(0.1, -0.2));
        assert_eq!(p.data[2], Complex::new(0.9, 0.1));
        assert_eq!(p.data[1], Complex::new(0.9, 0.1));
        assert_eq!(p.data[3], Complex::new(0.2, 0.0));
        assert_eq!(t.frequency_range_hz(), Some((100e6, 200e6)));
    }

    #[test]
    fn a_one_port_in_magnitude_and_angle() {
        let text = "# GHz S MA R 75\n1.0 0.5 90\n2.0 0.25 -90\n";
        let t = Touchstone::parse(text, 1, &settings()).unwrap();
        assert_eq!(t.z_ref_ohm, 75.0);
        let z = t.points[0].data[0];
        assert!(z.re.abs() < 1e-12 && (z.im - 0.5).abs() < 1e-12);
        assert!((t.points[1].data[0].im + 0.25).abs() < 1e-12);
    }

    #[test]
    fn the_db_format_converts_the_magnitude() {
        let text = "# Hz S DB\n1000 -6.0206 0\n";
        let t = Touchstone::parse(text, 1, &settings()).unwrap();
        assert!((t.points[0].data[0].re - 0.5).abs() < 1e-4);
        assert_eq!(t.points[0].freq_hz, 1000.0);
    }

    #[test]
    fn defaults_apply_without_an_option_line() {
        let t = Touchstone::parse("1 0.5 0\n", 1, &settings()).unwrap();
        assert_eq!(
            (t.unit, t.format, t.parameter),
            (FreqUnit::GHz, Format::Ma, Parameter::S)
        );
        assert_eq!(t.points[0].freq_hz, 1e9);
        // a second option line is ignored
        let t = Touchstone::parse("# GHz S RI\n# MHz S MA\n1 0.5 0\n", 1, &settings()).unwrap();
        assert_eq!(t.format, Format::Ri);
        // the options are case-insensitive and may be in any order
        let t = Touchstone::parse("# r 25 ri s khz\n1 0.5 0\n", 1, &settings()).unwrap();
        assert_eq!(
            (t.unit, t.format, t.z_ref_ohm),
            (FreqUnit::KHz, Format::Ri, 25.0)
        );
    }

    #[test]
    fn comments_may_follow_the_data() {
        let text = "# GHz S RI R 50 ! options\n1 0.5 0 ! first point\n";
        let t = Touchstone::parse(text, 1, &settings()).unwrap();
        assert_eq!(t.points.len(), 1);
    }

    #[test]
    fn a_three_port_record_runs_over_several_lines() {
        let mut text = String::from("# GHz S RI R 50\n");
        for (f, base) in [(1.0, 0.0), (2.0, 1.0)] {
            text.push_str(&format!("{f}"));
            for k in 0..9 {
                let v = base + f64::from(k) * 0.01;
                text.push_str(&format!(" {v} {}", -v));
                if k % 3 == 1 {
                    text.push_str("\n   ");
                }
            }
            text.push('\n');
        }
        let t = Touchstone::parse(&text, 3, &settings()).unwrap();
        assert_eq!(t.points.len(), 2);
        // by rows: entry (1, 2) is the sixth value
        assert!((t.points[0].data[5].re - 0.05).abs() < 1e-12);
        assert_eq!(t.points[1].data.len(), 9);
    }

    #[test]
    fn noise_data_follows_the_network_data_of_a_two_port() {
        let text = "# GHz S RI R 50\n1 0.1 0 0.9 0 0.9 0 0.1 0\n2 0.2 0 0.8 0 0.8 0 0.2 0\n\
                    1 0.5 0.4 30 0.2\n2 0.7 0.5 45 0.3\n";
        let t = Touchstone::parse(text, 2, &settings()).unwrap();
        assert_eq!((t.points.len(), t.noise.len()), (2, 2));
        assert_eq!(t.noise[0].freq_hz, 1e9);
        assert_eq!(t.noise[1].nf_min_db, 0.7);
        assert!((t.noise[1].gamma_opt.abs() - 0.5).abs() < 1e-12);
        assert!((t.noise[1].gamma_opt.arg_deg() - 45.0).abs() < 1e-9);
        assert_eq!(t.noise[0].rn_normalized, 0.2);
    }

    #[test]
    fn extensions_give_the_number_of_ports() {
        assert_eq!(ports_from_name("amp.s2p"), Some(2));
        assert_eq!(ports_from_name("C:\\x\\filter.S1P"), Some(1));
        assert_eq!(ports_from_name("big.s12p"), Some(12));
        for name in [
            "amp.s0p", "amp.sp", "amp.snp", "amp.txt", "s2p", "amp.s2", "amp.s-2p",
        ] {
            assert_eq!(ports_from_name(name), None, "{name}");
        }
    }

    #[test]
    fn errors_say_the_line() {
        let cases = [
            ("# GHz S RI\n1 0.5 x\n", 2, "syntax"),
            ("# GHz Q RI\n1 0.5 0\n", 1, "syntax"),
            ("# GHz S RI R\n1 0.5 0\n", 1, "syntax"),
            ("# GHz S RI R abc\n1 0.5 0\n", 1, "syntax"),
            ("# GHz S RI R -5\n1 0.5 0\n", 1, "data"),
            ("[Version] 2.0\n1 0.5 0\n", 1, "syntax"),
            ("# GHz S RI\n1 0.5 0 7\n", 2, "syntax"),
            ("# GHz S RI\n1 0.5\n", 2, "data"),
            ("# GHz S RI\n2 0.5 0\n1 0.5 0\n", 3, "data"),
            ("# GHz S RI\n1 0.5 0\n1 0.4 0\n", 3, "data"),
            ("# GHz S RI\n-1 0.5 0\n", 2, "data"),
            ("# GHz S RI\n1 inf 0\n", 2, "data"),
            ("# GHz S MA\n1 -0.5 0\n", 2, "data"),
            ("# GHz S RI\n", 0, "data"),
        ];
        for (text, line, kind) in cases {
            match Touchstone::parse(text, 1, &settings()) {
                Err(RfError::TouchstoneSyntax { line: got, .. }) if kind == "syntax" => {
                    assert_eq!(got, line, "{text}")
                }
                Err(RfError::TouchstoneData { line: got, .. }) if kind == "data" => {
                    assert_eq!(got, line, "{text}")
                }
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn noise_errors_are_refused() {
        let early = "# GHz S RI\n1 0.5 0.4 30 0.2\n";
        assert!(Touchstone::parse(early, 2, &settings()).is_err());
        let bad = "# GHz S RI\n1 0.1 0 0.9 0 0.9 0 0.1 0\n1 0.5 0.4 30 0.2\n1 0.6 0.4 30 0.2\n";
        assert!(matches!(
            Touchstone::parse(bad, 2, &settings()),
            Err(RfError::TouchstoneData { line: 4, .. })
        ));
        let short = "# GHz S RI\n1 0.1 0 0.9 0 0.9 0 0.1 0\n1 0.5 0.4 30\n";
        assert!(matches!(
            Touchstone::parse(short, 2, &settings()),
            Err(RfError::TouchstoneData { line: 3, .. })
        ));
        let shorter = "# GHz S RI\n1 0.1 0 0.9 0 0.9 0 0.1 0\n1 0.5 0.4 30 0.2\n2 0.5 0.4\n";
        assert!(matches!(
            Touchstone::parse(shorter, 2, &settings()),
            Err(RfError::TouchstoneSyntax { line: 4, .. })
        ));
    }

    #[test]
    fn the_limits_of_the_settings_apply() {
        let text = "# GHz S RI\n1 0.5 0\n2 0.5 0\n3 0.5 0\n";
        let few = RfSettings {
            max_points: 2,
            ..RfSettings::default()
        };
        assert!(matches!(
            Touchstone::parse(text, 1, &few),
            Err(RfError::OutsideLimit { .. })
        ));
        let small = RfSettings {
            max_file_bytes: 10,
            ..RfSettings::default()
        };
        assert!(matches!(
            Touchstone::parse(text, 1, &small),
            Err(RfError::OutsideLimit { .. })
        ));
        assert!(Touchstone::parse(text, 0, &settings()).is_err());
        assert!(Touchstone::parse(text, 17, &settings()).is_err());
    }

    #[test]
    fn writing_and_reading_back_gives_the_same_values() {
        let original = Touchstone::parse(TWO_PORT, 2, &settings()).unwrap();
        let text = original.to_text().unwrap();
        let again = Touchstone::parse(&text, 2, &settings()).unwrap();
        assert_eq!(again.parameter, original.parameter);
        assert_eq!((again.format, again.unit), (original.format, original.unit));
        assert_eq!(again.comments, original.comments);
        assert_eq!(again.points.len(), 2);
        for (a, b) in again.points.iter().zip(&original.points) {
            assert!((a.freq_hz - b.freq_hz).abs() < 1e-3);
            for (x, y) in a.data.iter().zip(&b.data) {
                assert!((*x - *y).abs() < 1e-15);
            }
        }
    }

    #[test]
    fn every_format_and_size_round_trips() {
        for ports in 1..=4usize {
            for format in [Format::Ri, Format::Ma, Format::Db] {
                for unit in [FreqUnit::Hz, FreqUnit::KHz, FreqUnit::MHz, FreqUnit::GHz] {
                    let mut original = Touchstone {
                        ports,
                        parameter: Parameter::S,
                        format,
                        unit,
                        z_ref_ohm: 75.0,
                        points: Vec::new(),
                        noise: Vec::new(),
                        comments: vec![],
                    };
                    for p in 0..3u32 {
                        let data = (0..ports * ports)
                            .map(|k| {
                                Complex::new(
                                    0.1 + f64::from(p) * 0.05 + k as f64 * 0.01,
                                    0.3 - k as f64 * 0.02,
                                )
                            })
                            .collect();
                        original.points.push(Point {
                            freq_hz: 1.0e6 * f64::from(p + 1),
                            data,
                        });
                    }
                    let text = original.to_text().unwrap();
                    let again = Touchstone::parse(&text, ports, &settings()).unwrap();
                    assert_eq!(again.points.len(), 3, "{ports} {format:?} {unit:?}");
                    assert_eq!(again.z_ref_ohm, 75.0);
                    for (a, b) in again.points.iter().zip(&original.points) {
                        assert!((a.freq_hz - b.freq_hz).abs() < 1e-6 * b.freq_hz);
                        for (x, y) in a.data.iter().zip(&b.data) {
                            assert!((*x - *y).abs() < 1e-9, "{ports} {format:?} {x:?} {y:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn noise_data_round_trips() {
        let text = "# GHz S RI R 50\n1 0.1 0 0.9 0 0.9 0 0.1 0\n2 0.2 0 0.8 0 0.8 0 0.2 0\n\
                    1 0.5 0.4 30 0.2\n";
        let t = Touchstone::parse(text, 2, &settings()).unwrap();
        let again = Touchstone::parse(&t.to_text().unwrap(), 2, &settings()).unwrap();
        assert_eq!(again.noise.len(), 1);
        assert!((again.noise[0].gamma_opt - t.noise[0].gamma_opt).abs() < 1e-12);
    }

    #[test]
    fn writing_refuses_what_cannot_be_written() {
        let mut t = Touchstone::parse("# GHz S RI\n1 0.5 0\n", 1, &settings()).unwrap();
        t.points[0].data.push(Complex::ONE);
        assert!(t.to_text().is_err());
        let mut t = Touchstone::parse("# GHz S RI\n1 0.5 0\n", 1, &settings()).unwrap();
        t.format = Format::Db;
        t.points[0].data[0] = Complex::ZERO;
        assert!(t.to_text().is_err());
        let mut t = Touchstone::parse("# GHz S RI\n1 0.5 0\n", 1, &settings()).unwrap();
        t.points[0].data[0] = Complex::new(f64::NAN, 0.0);
        assert!(t.to_text().is_err());
        let mut t = Touchstone::parse("# GHz S RI\n1 0.5 0\n", 1, &settings()).unwrap();
        t.z_ref_ohm = 0.0;
        assert!(t.to_text().is_err());
        let mut t = Touchstone::parse("# GHz S RI\n1 0.5 0\n", 1, &settings()).unwrap();
        t.noise.push(NoisePoint {
            freq_hz: 1.0,
            nf_min_db: 1.0,
            gamma_opt: Complex::ZERO,
            rn_normalized: 0.1,
        });
        assert!(t.to_text().is_err());
    }

    #[test]
    fn z_and_y_files_convert_to_s() {
        // a normalised series impedance of 1 (50 ohm on 50 ohm) between two ports has no Z
        // matrix; use a tee: z11 = z22 = 2, z12 = z21 = 1 (normalised)
        let text = "# GHz Z RI R 50\n1 2 0 1 0 1 0 2 0\n";
        let t = Touchstone::parse(text, 2, &settings()).unwrap();
        let s = t.s_matrix(0).unwrap();
        // S = (Z - I)(Z + I)^-1 with Z = [[2,1],[1,2]]: S11 = 1/3 * ... check against the formula
        let z = crate::network::Mat2::new(
            Complex::real(100.0),
            Complex::real(50.0),
            Complex::real(50.0),
            Complex::real(100.0),
        );
        let want = crate::network::z_to_s(&z, 50.0).unwrap();
        assert!(s.max_difference(&want.to_cmatrix().unwrap()) < 1e-12);
        let y = Touchstone::parse("# GHz Y RI R 50\n1 1 0 -0.5 0 -0.5 0 1 0\n", 2, &settings())
            .unwrap();
        let sy = y.s_matrix(0).unwrap();
        assert!(sy.get(0, 0).is_finite());
        let h = Touchstone::parse("# GHz H RI\n1 1 0 0 0 0 0 1 0\n", 2, &settings()).unwrap();
        assert!(matches!(h.s_matrix(0), Err(RfError::TouchstoneData { .. })));
        assert!(t.s_matrix(5).is_err());
    }

    #[test]
    fn interpolation_is_linear_and_never_extrapolates() {
        let text = "# GHz S RI\n1 0.0 0\n3 1.0 0\n";
        let t = Touchstone::parse(text, 1, &settings()).unwrap();
        assert!((t.s_matrix_at(2e9).unwrap().get(0, 0).re - 0.5).abs() < 1e-12);
        assert!((t.s_matrix_at(1e9).unwrap().get(0, 0).re).abs() < 1e-12);
        assert!((t.s_matrix_at(3e9).unwrap().get(0, 0).re - 1.0).abs() < 1e-12);
        assert!(matches!(
            t.s_matrix_at(0.5e9),
            Err(RfError::OutsideRange { .. })
        ));
        assert!(matches!(
            t.s_matrix_at(3.5e9),
            Err(RfError::OutsideRange { .. })
        ));
        assert!(t.s_matrix_at(f64::NAN).is_err());
        let one = Touchstone::parse("# GHz S RI\n1 0.4 0\n", 1, &settings()).unwrap();
        assert!((one.s_matrix_at(1e9).unwrap().get(0, 0).re - 0.4).abs() < 1e-12);
    }

    #[test]
    fn the_group_delay_of_a_delay_line_file() {
        let delay = 1.0e-9;
        let mut text = String::from("# MHz S RI\n");
        for k in 0..5 {
            let f = 100.0 + 10.0 * f64::from(k);
            let phase = -2.0 * std::f64::consts::PI * f * 1e6 * delay;
            text.push_str(&format!(
                "{f} 0 0 {} {} {} {} 0 0\n",
                phase.cos(),
                phase.sin(),
                phase.cos(),
                phase.sin()
            ));
        }
        let t = Touchstone::parse(&text, 2, &settings()).unwrap();
        let tau = t.group_delay_s(125e6).unwrap();
        assert!((tau - delay).abs() < 1e-12);
        assert!((t.group_delay_s(100e6).unwrap() - delay).abs() < 1e-12);
        assert!((t.group_delay_s(140e6).unwrap() - delay).abs() < 1e-12);
        assert!(t.group_delay_s(50e6).is_err());
        let one = Touchstone::parse("# GHz S RI\n1 0.4 0\n2 0.4 0\n", 1, &settings()).unwrap();
        assert!(one.group_delay_s(1.5e9).is_err());
    }
}
