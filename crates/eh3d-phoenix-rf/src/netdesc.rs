// SPDX-License-Identifier: GPL-3.0-or-later
//! A network of lumped elements and lines written as a few lines of text, its S parameters over
//! a sweep (ABCD matrices in cascade, then S against a real reference impedance), and the
//! figures read from them: S11 and S21 in magnitude and phase, return loss, VSWR and insertion
//! loss. The result can be written as a Touchstone file.
//!
//! The text has one element per line, from the port 1 side to the port 2 side:
//!
//! ```text
//! # a comment (also after !)
//! reference 50
//! series L 10n
//! shunt C 2.2p
//! line 50 90deg@1G
//! line 50 12mm 3.2
//! ```
//!
//! `series` and `shunt` take `R`, `L` or `C` and a value in ohm, henry or farad. A `line` takes
//! its characteristic impedance and either an electrical length at a frequency (`90deg@1G`) or a
//! physical length (`12mm`, `1.2cm`, `0.012m`, `12000um`) with the effective permittivity.
//! Values may end in a prefix: `f p n u m k meg g t` (`m` is milli, `M` and `meg` are mega) and
//! may carry a unit after it (`10nH`, `2.2pF`, `1GHz`).
//!
//! The elements are ideal (no loss, no parasitics) and the lines lossless and without
//! dispersion: this is an estimate from lumped elements and quasi-static lines.

use crate::complex::Complex;
use crate::error::RfError;
use crate::line::Line;
use crate::network::{abcd_lossless_line, abcd_series, abcd_shunt, abcd_to_s, Mat2};
use crate::settings::RfSettings;
use crate::touchstone::{Format, FreqUnit, Parameter, Point, Touchstone};
use std::f64::consts::PI;

/// The kind of a lumped element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lumped {
    /// A resistor, in ohm.
    R,
    /// An inductor, in henry.
    L,
    /// A capacitor, in farad.
    C,
}

/// One element of the network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NetElement {
    /// A lumped element in series with the signal.
    Series(Lumped, f64),
    /// A lumped element across the signal.
    Shunt(Lumped, f64),
    /// A lossless line.
    Line {
        /// The characteristic impedance in ohm.
        z0_ohm: f64,
        /// The delay in seconds.
        delay_s: f64,
    },
}

/// A network: a reference impedance and the elements from port 1 to port 2.
#[derive(Clone, Debug, PartialEq)]
pub struct NetDesc {
    /// The reference impedance of both ports in ohm.
    pub z0_ohm: f64,
    /// The elements from port 1 to port 2.
    pub elements: Vec<NetElement>,
}

/// A value with an optional prefix and unit, such as `10n`, `2.2pF`, `1GHz` or `4.7k`.
fn si_value(text: &str) -> Result<f64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a value is missing".to_string());
    }
    let numeric = |c: char| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E');
    let mut cut = t.find(|c: char| !numeric(c)).unwrap_or(t.len());
    // a number such as `5e` is not complete: give back the characters that do not parse
    let number = loop {
        match t[..cut].parse::<f64>() {
            Ok(value) => break value,
            Err(_) if cut > 0 => cut -= 1,
            Err(_) => return Err(format!("{} is not a number", text.trim())),
        }
        while cut > 0 && !t.is_char_boundary(cut) {
            cut -= 1;
        }
    };
    if !number.is_finite() {
        return Err(format!("{} is not a finite number", text.trim()));
    }
    let rest = &t[cut..];
    let lower = rest.to_ascii_lowercase();
    let (scale, unit): (f64, &str) = if let Some(after) = lower.strip_prefix("meg") {
        (1e6, after)
    } else {
        match rest.chars().next() {
            None => (1.0, ""),
            Some('f') => (1e-15, &lower[1..]),
            Some('p' | 'P') => (1e-12, &lower[1..]),
            Some('n' | 'N') => (1e-9, &lower[1..]),
            Some('u' | 'U') => (1e-6, &lower[1..]),
            Some('µ') => (1e-6, &lower[2..]),
            Some('m') => (1e-3, &lower[1..]),
            Some('M') => (1e6, &lower[1..]),
            Some('k' | 'K') => (1e3, &lower[1..]),
            Some('g' | 'G') => (1e9, &lower[1..]),
            Some('t' | 'T') => (1e12, &lower[1..]),
            Some(_) => (1.0, lower.as_str()),
        }
    };
    if !["", "h", "f", "ohm", "hz", "s"].contains(&unit) {
        return Err(format!("{} has an unknown suffix", text.trim()));
    }
    let value = number * scale;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{} is too large", text.trim()))
    }
}

/// A value with an optional prefix and unit (`10n`, `2.2pF`, `1GHz`). The error says why the text
/// is not a value.
pub fn parse_value(text: &str) -> Result<f64, RfError> {
    si_value(text).map_err(|reason| RfError::InvalidNumber {
        field: "value".to_string(),
        reason,
    })
}

pub(crate) fn parse_si_text(text: &str) -> Result<f64, String> {
    si_value(text)
}

fn syntax(line: usize, reason: impl Into<String>) -> RfError {
    RfError::NetSyntax {
        line,
        reason: reason.into(),
    }
}

fn lumped(word: &str) -> Option<Lumped> {
    match word.to_ascii_lowercase().as_str() {
        "r" => Some(Lumped::R),
        "l" => Some(Lumped::L),
        "c" => Some(Lumped::C),
        _ => None,
    }
}

/// The delay in seconds of the length written in a `line` element.
fn line_delay(
    length: &str,
    eps_text: Option<&str>,
    z0_ohm: f64,
    line: usize,
) -> Result<f64, RfError> {
    let lower = length.to_ascii_lowercase();
    if let Some(position) = lower.find("deg@") {
        let degrees: f64 = length[..position]
            .parse()
            .map_err(|_| syntax(line, "the electrical length is not a number"))?;
        // the frequency keeps its case: `M` is mega and `m` is milli
        let freq = si_value(&length[position + 4..]).map_err(|reason| syntax(line, reason))?;
        if !degrees.is_finite() || degrees < 0.0 || !freq.is_finite() || freq <= 0.0 {
            return Err(syntax(
                line,
                "an electrical length needs degrees of zero or more and a frequency above zero",
            ));
        }
        return Ok(degrees / 360.0 / freq);
    }
    let (digits, scale) = if let Some(d) = lower.strip_suffix("mm") {
        (d, 1e-3)
    } else if let Some(d) = lower.strip_suffix("cm") {
        (d, 1e-2)
    } else if let Some(d) = lower.strip_suffix("um") {
        (d, 1e-6)
    } else if let Some(d) = lower.strip_suffix('m') {
        (d, 1.0)
    } else {
        return Err(syntax(
            line,
            "a length is written as 90deg@1G or with a unit: mm, cm, um or m",
        ));
    };
    let value: f64 = digits
        .parse()
        .map_err(|_| syntax(line, "the length is not a number"))?;
    let eps_text = eps_text.ok_or_else(|| {
        syntax(
            line,
            "a physical length needs the effective permittivity after it",
        )
    })?;
    let eps: f64 = eps_text
        .parse()
        .map_err(|_| syntax(line, "the effective permittivity is not a number"))?;
    let carrier = Line::new(z0_ohm, eps).map_err(|e| syntax(line, e.to_string()))?;
    carrier
        .delay_s(value * scale)
        .map_err(|e| syntax(line, e.to_string()))
}

impl NetDesc {
    /// Read a network description. Every error names the line.
    pub fn parse(text: &str, settings: &RfSettings) -> Result<NetDesc, RfError> {
        if text.len() > settings.max_net_bytes {
            return Err(RfError::OutsideLimit {
                field: "size of the network description".to_string(),
                value: text.len() as f64,
                limit: settings.max_net_bytes as f64,
            });
        }
        let mut z0_ohm = settings.default_z0_ohm;
        let mut reference_seen = false;
        let mut elements: Vec<NetElement> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let content = raw.split(['#', '!']).next().unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            let words: Vec<&str> = content.split_whitespace().collect();
            let keyword = words[0].to_ascii_lowercase();
            match keyword.as_str() {
                "reference" => {
                    if reference_seen || !elements.is_empty() || words.len() != 2 {
                        return Err(syntax(
                            line,
                            "`reference Z0` is written once, before the elements",
                        ));
                    }
                    let value = si_value(words[1]).map_err(|r| syntax(line, r))?;
                    settings
                        .check_reference_ohm("reference impedance", value)
                        .map_err(|e| syntax(line, e.to_string()))?;
                    z0_ohm = value;
                    reference_seen = true;
                }
                "series" | "shunt" => {
                    let [_, kind, value] = words.as_slice() else {
                        return Err(syntax(line, format!("`{keyword} R|L|C value` expected")));
                    };
                    let kind = lumped(kind)
                        .ok_or_else(|| syntax(line, "the kind is R, L or C".to_string()))?;
                    let value = si_value(value).map_err(|r| syntax(line, r))?;
                    if value <= 0.0 {
                        return Err(syntax(line, "the value must be above zero"));
                    }
                    elements.push(if keyword == "series" {
                        NetElement::Series(kind, value)
                    } else {
                        NetElement::Shunt(kind, value)
                    });
                }
                "line" => {
                    if !(3..=4).contains(&words.len()) {
                        return Err(syntax(line, "`line Z0 length [eps_eff]` expected"));
                    }
                    let z0 = si_value(words[1]).map_err(|r| syntax(line, r))?;
                    settings
                        .check_reference_ohm("line impedance", z0)
                        .map_err(|e| syntax(line, e.to_string()))?;
                    let delay_s = line_delay(words[2], words.get(3).copied(), z0, line)?;
                    elements.push(NetElement::Line {
                        z0_ohm: z0,
                        delay_s,
                    });
                }
                other => {
                    let shown: String = other.chars().take(20).collect();
                    return Err(syntax(
                        line,
                        format!("unknown word {shown}: use reference, series, shunt or line"),
                    ));
                }
            }
            if elements.len() > settings.max_net_elements {
                return Err(RfError::OutsideLimit {
                    field: "elements of the network".to_string(),
                    value: elements.len() as f64,
                    limit: settings.max_net_elements as f64,
                });
            }
        }
        if elements.is_empty() {
            return Err(syntax(0, "the network has no elements"));
        }
        Ok(NetDesc { z0_ohm, elements })
    }

    /// The ABCD matrix of the network at a frequency in Hz.
    pub fn abcd_at(&self, freq_hz: f64) -> Result<Mat2, RfError> {
        let omega = 2.0 * PI * crate::error::positive("frequency", freq_hz)?;
        let mut total = Mat2::identity();
        for element in &self.elements {
            let matrix = match *element {
                NetElement::Series(kind, v) => abcd_series(match kind {
                    Lumped::R => Complex::real(v),
                    Lumped::L => Complex::new(0.0, omega * v),
                    Lumped::C => Complex::new(0.0, omega * v).recip()?,
                }),
                NetElement::Shunt(kind, v) => abcd_shunt(match kind {
                    Lumped::R => Complex::real(1.0 / v),
                    Lumped::L => Complex::new(0.0, omega * v).recip()?,
                    Lumped::C => Complex::new(0.0, omega * v),
                }),
                NetElement::Line { z0_ohm, delay_s } => {
                    abcd_lossless_line(z0_ohm, omega * delay_s)?
                }
            };
            total = total.times(&matrix);
        }
        Ok(total)
    }

    /// The S parameters at a frequency in Hz, against the reference impedance of the network.
    pub fn s_at(&self, freq_hz: f64) -> Result<Mat2, RfError> {
        abcd_to_s(&self.abcd_at(freq_hz)?, self.z0_ohm)
    }
}

/// The frequencies of a sweep: `points` of them from `start_hz` to `stop_hz`, evenly or in a
/// geometric progression. The ends and the number are checked against the settings.
pub fn frequencies(
    start_hz: f64,
    stop_hz: f64,
    points: usize,
    logarithmic: bool,
    settings: &RfSettings,
) -> Result<Vec<f64>, RfError> {
    settings.check_frequency_hz("start frequency", start_hz)?;
    settings.check_frequency_hz("stop frequency", stop_hz)?;
    if stop_hz <= start_hz {
        return Err(RfError::InvalidNumber {
            field: "stop frequency".to_string(),
            reason: "must be above the start frequency".to_string(),
        });
    }
    if points < 2 {
        return Err(RfError::InvalidNumber {
            field: "number of points".to_string(),
            reason: "must be at least 2".to_string(),
        });
    }
    if points > settings.max_sweep_points {
        return Err(RfError::OutsideLimit {
            field: "number of points".to_string(),
            value: points as f64,
            limit: settings.max_sweep_points as f64,
        });
    }
    let last = (points - 1) as f64;
    Ok((0..points)
        .map(|i| {
            let t = i as f64 / last;
            if logarithmic {
                start_hz * (stop_hz / start_hz).powf(t)
            } else {
                start_hz + (stop_hz - start_hz) * t
            }
        })
        .collect())
}

/// The S matrix of the network at a frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SPoint {
    /// The frequency in Hz.
    pub freq_hz: f64,
    /// The S parameters.
    pub s: Mat2,
}

/// The S parameters of a network at each frequency.
pub fn response(desc: &NetDesc, freqs: &[f64]) -> Result<Vec<SPoint>, RfError> {
    freqs
        .iter()
        .map(|&freq_hz| {
            Ok(SPoint {
                freq_hz,
                s: desc.s_at(freq_hz)?,
            })
        })
        .collect()
}

/// The figures read from the S parameters at one frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figures {
    /// The magnitude of S11 in dB.
    pub s11_db: f64,
    /// The phase of S11 in degrees.
    pub s11_deg: f64,
    /// The magnitude of S21 in dB.
    pub s21_db: f64,
    /// The phase of S21 in degrees.
    pub s21_deg: f64,
    /// The return loss at port 1 in dB (infinite for a perfect match).
    pub return_loss_db: f64,
    /// The voltage standing wave ratio at port 1 (infinite when `|S11|` is 1 or more).
    pub vswr: f64,
    /// The insertion loss in dB, `-20 log10 |S21|`.
    pub insertion_loss_db: f64,
}

/// The figures of an S matrix.
pub fn figures(s: &Mat2) -> Figures {
    Figures {
        s11_db: s.m11.db20(),
        s11_deg: s.m11.arg_deg(),
        s21_db: s.m21.db20(),
        s21_deg: s.m21.arg_deg(),
        return_loss_db: crate::network::return_loss_db(s.m11),
        vswr: crate::network::vswr(s.m11),
        insertion_loss_db: crate::network::insertion_loss_db(s.m21),
    }
}

/// The response as a Touchstone two-port file in the given format and unit. The values are
/// written in the order the format of the file requires.
pub fn to_touchstone(
    points: &[SPoint],
    z0_ohm: f64,
    format: Format,
    unit: FreqUnit,
    comment: &str,
) -> Touchstone {
    Touchstone {
        ports: 2,
        parameter: Parameter::S,
        format,
        unit,
        z_ref_ohm: z0_ohm,
        points: points
            .iter()
            .map(|p| Point {
                freq_hz: p.freq_hz,
                data: vec![p.s.m11, p.s.m12, p.s.m21, p.s.m22],
            })
            .collect(),
        noise: Vec::new(),
        comments: vec![comment.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{is_passive, is_reciprocal};

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    #[test]
    fn values_with_prefixes_and_units_are_read() {
        let value = |t: &str| si_value(t).unwrap();
        let close = |a: f64, b: f64| ((a - b) / b).abs() < 1e-12;
        assert!(close(value("10n"), 10e-9));
        assert!(close(value("10nH"), 10e-9));
        assert!(close(value("2.2pF"), 2.2e-12));
        assert!(close(value("4.7k"), 4.7e3));
        assert!(close(value("4.7kohm"), 4.7e3));
        assert!(close(value("1G"), 1e9));
        assert!(close(value("2.4GHz"), 2.4e9));
        assert!(close(value("100MHz"), 1e8));
        assert!(close(value("100meg"), 1e8));
        assert!(close(value("5m"), 5e-3));
        assert!(close(value("1e-9"), 1e-9));
        assert!(close(value("1.5e3"), 1.5e3));
        assert!(close(value("33"), 33.0));
        assert!(close(value("3µH"), 3e-6));
        assert!(close(value("-2"), -2.0));
        assert!(close(value("2H"), 2.0));
    }

    #[test]
    fn values_that_are_not_values_are_refused() {
        for bad in [
            "", "  ", "x", "5e", "1q", "10nHz2", "inf", "nan", "1e999", "--1", "5 n",
        ] {
            assert!(si_value(bad).is_err(), "{bad:?}");
        }
        assert!(parse_value("nope").is_err());
        assert!(matches!(
            parse_value("1q"),
            Err(RfError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn the_frequency_of_an_electrical_length_keeps_its_case() {
        let net = NetDesc::parse("line 50 90deg@100MHz", &settings()).unwrap();
        match net.elements[0] {
            NetElement::Line { delay_s, .. } => assert!((delay_s - 2.5e-9).abs() < 1e-21),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_description_is_read_with_comments_and_all_the_elements() {
        let text = "# a low-pass\nreference 75\nseries L 10n ! first\nshunt C 2.2p\n\
                    line 75 90deg@1G\nline 50 12mm 3.2\nshunt R 1k\nseries r 5\nshunt l 3n\nseries c 1p\n";
        let net = NetDesc::parse(text, &settings()).unwrap();
        assert_eq!(net.z0_ohm, 75.0);
        assert_eq!(net.elements.len(), 8);
        match net.elements[0] {
            NetElement::Series(Lumped::L, value) => assert!((value - 10e-9).abs() < 1e-20),
            other => panic!("{other:?}"),
        }
        match net.elements[2] {
            NetElement::Line { z0_ohm, delay_s } => {
                assert_eq!(z0_ohm, 75.0);
                // 90 degrees at 1 GHz is a quarter of a nanosecond
                assert!((delay_s - 0.25e-9).abs() < 1e-21);
            }
            other => panic!("{other:?}"),
        }
        match net.elements[3] {
            NetElement::Line { delay_s, .. } => {
                let expected = 12e-3 * 3.2f64.sqrt() / 299_792_458.0;
                assert!((delay_s - expected).abs() < 1e-18);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn errors_name_the_line() {
        let cases = [
            ("series L", 1),
            ("series X 1n", 1),
            ("series L 0", 1),
            ("series L -1n", 1),
            ("\n\nshunt C two", 3),
            ("series L 1n\nwhat 3", 2),
            ("series L 1n\nreference 50", 2),
            ("reference 50\nreference 60\nseries L 1n", 2),
            ("line 50", 1),
            ("line 50 12mm", 1),
            ("line 50 12mm 0.5", 1),
            ("line 50 12 3.2", 1),
            ("line 50 deg@1G", 1),
            ("line 50 90deg@0", 1),
            ("line 0 90deg@1G", 1),
            ("line 50 90deg@1G 3 4", 1),
        ];
        for (text, line) in cases {
            match NetDesc::parse(text, &settings()) {
                Err(RfError::NetSyntax { line: got, .. }) => assert_eq!(got, line, "{text:?}"),
                other => panic!("{text:?}: {other:?}"),
            }
        }
        assert!(matches!(
            NetDesc::parse("# nothing\n", &settings()),
            Err(RfError::NetSyntax { line: 0, .. })
        ));
    }

    #[test]
    fn limits_of_the_description_come_from_the_settings() {
        let few = RfSettings {
            max_net_elements: 2,
            ..settings()
        };
        assert!(matches!(
            NetDesc::parse("series L 1n\nseries L 1n\nseries L 1n", &few),
            Err(RfError::OutsideLimit { .. })
        ));
        let small = RfSettings {
            max_net_bytes: 10,
            ..settings()
        };
        assert!(matches!(
            NetDesc::parse("series L 1n\nshunt C 1p\n", &small),
            Err(RfError::OutsideLimit { .. })
        ));
    }

    #[test]
    fn a_through_connection_and_a_series_resistor_have_the_textbook_s_parameters() {
        // a series resistor R between ports of z0: S11 = R / (R + 2 z0), S21 = 2 z0 / (R + 2 z0)
        let net = NetDesc::parse("series R 50", &settings()).unwrap();
        let s = net.s_at(1e9).unwrap();
        assert!((s.m11.re - 1.0 / 3.0).abs() < 1e-12 && s.m11.im.abs() < 1e-12);
        assert!((s.m21.re - 2.0 / 3.0).abs() < 1e-12 && s.m21.im.abs() < 1e-12);
        let f = figures(&s);
        assert!((f.vswr - 2.0).abs() < 1e-12);
        assert!((f.insertion_loss_db - 3.521825181113625).abs() < 1e-9);
        assert!((f.return_loss_db - 9.542425094393248).abs() < 1e-9);
    }

    #[test]
    fn a_quarter_wave_line_of_the_same_impedance_is_a_through_with_90_degrees() {
        let net = NetDesc::parse("line 50 90deg@1G", &settings()).unwrap();
        let s = net.s_at(1e9).unwrap();
        assert!(s.m11.abs() < 1e-12);
        assert!((s.m21.abs() - 1.0).abs() < 1e-12);
        assert!((s.m21.arg_deg() + 90.0).abs() < 1e-9);
        // at twice the frequency the same line is 180 degrees
        let s2 = net.s_at(2e9).unwrap();
        assert!((s2.m21.arg_deg().abs() - 180.0).abs() < 1e-9);
    }

    #[test]
    fn a_lossless_low_pass_conserves_power_and_is_reciprocal() {
        let text = "shunt C 31.83p\nseries L 159.15n\nshunt C 31.83p\n";
        let net = NetDesc::parse(text, &settings()).unwrap();
        for f in [1e6, 1e7, 1e8, 3e8, 1e9] {
            let s = net.s_at(f).unwrap();
            assert!(is_passive(&s, 1e-9));
            assert!(is_reciprocal(&s, 1e-12));
            let power = s.m11.norm_sqr() + s.m21.norm_sqr();
            assert!((power - 1.0).abs() < 1e-9, "{f}: {power}");
        }
        // 3 dB down near 100 MHz (a third-order Butterworth filter)
        let at_cutoff = figures(&net.s_at(1e8).unwrap());
        assert!((at_cutoff.insertion_loss_db - 3.0103).abs() < 0.01);
    }

    #[test]
    fn the_sweep_has_the_points_asked_for_and_checks_its_arguments() {
        let s = settings();
        let linear = frequencies(1e6, 1e9, 5, false, &s).unwrap();
        assert_eq!(linear.len(), 5);
        assert_eq!(linear[0], 1e6);
        assert!((linear[4] - 1e9).abs() < 1e-3);
        let log = frequencies(1e6, 1e9, 4, true, &s).unwrap();
        assert!((log[1] - 1e7).abs() < 1e-3 && (log[2] - 1e8).abs() < 1e-2);
        assert!(frequencies(1e9, 1e6, 5, false, &s).is_err());
        assert!(frequencies(1e6, 1e9, 1, false, &s).is_err());
        assert!(frequencies(0.0, 1e9, 5, false, &s).is_err());
        assert!(matches!(
            frequencies(1e6, 1e9, 1_000_000, false, &s),
            Err(RfError::OutsideLimit { .. })
        ));
    }

    #[test]
    fn the_touchstone_file_of_a_response_reads_back() {
        let net = NetDesc::parse("series L 10n\nshunt C 2p", &settings()).unwrap();
        let freqs = frequencies(1e8, 1e9, 4, false, &settings()).unwrap();
        let points = response(&net, &freqs).unwrap();
        let file = to_touchstone(&points, net.z0_ohm, Format::Ri, FreqUnit::Hz, "test");
        let text = file.to_text().unwrap();
        let again = Touchstone::parse(&text, 2, &settings()).unwrap();
        assert_eq!(again.points.len(), 4);
        for (a, p) in again.points.iter().zip(&points) {
            assert!((a.freq_hz - p.freq_hz).abs() < 1e-6);
            let m = again.s_matrix(0).unwrap();
            assert_eq!(m.size(), 2);
            assert!((a.data[2] - p.s.m21).abs() < 1e-12);
            assert!((a.data[0] - p.s.m11).abs() < 1e-12);
        }
        // the same bytes every time
        assert_eq!(text, file.to_text().unwrap());
    }
}
