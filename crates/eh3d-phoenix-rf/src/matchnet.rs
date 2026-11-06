// SPDX-License-Identifier: GPL-3.0-or-later
//! Matching networks beyond the L network: pi and T networks with a chosen loaded quality
//! factor, the bandwidth of a network, the double stub, and the equivalents with transmission
//! lines (single stub, double stub, quarter-wave transformer) with their lengths in metres on a
//! line of the stack-up.
//!
//! Everything here is an estimate at one frequency from ideal lumped elements and lossless
//! lines (quasi-static): there is no loss, no parasitic and no tolerance, and nothing was
//! compared with a measurement or with another program. Every network carries what is needed to
//! rebuild it and give the input impedance, which is the check of the tests.

use crate::complex::Complex;
use crate::error::RfError;
use crate::filter::Position;
use crate::line::Line;
use crate::matching::{
    half_turn, quarter_wave, series_element, shunt_element, stub_match, Element, ElementKind,
    LMatch, QuarterWave, StubKind, StubMatch, Termination, Topology,
};
use crate::network::{abcd_lossless_line, abcd_series, abcd_shunt, input_impedance, Mat2};
use crate::settings::RfSettings;
use std::f64::consts::PI;

/// One element of a ladder, in series or in shunt, listed from the source side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    /// Whether the element is in series with the signal or across it.
    pub position: Position,
    /// The inductor or capacitor.
    pub element: Element,
}

impl Part {
    /// The ABCD matrix of the element at a frequency in Hz.
    pub fn abcd(&self, freq_hz: f64) -> Result<Mat2, RfError> {
        let omega = 2.0 * PI * crate::error::positive("frequency", freq_hz)?;
        let reactive = Complex::new(0.0, omega * self.element.value);
        Ok(match (self.position, self.element.kind) {
            (Position::Series, ElementKind::Inductor) => abcd_series(reactive),
            (Position::Series, ElementKind::Capacitor) => abcd_series(reactive.recip()?),
            (Position::Shunt, ElementKind::Capacitor) => abcd_shunt(reactive),
            (Position::Shunt, ElementKind::Inductor) => abcd_shunt(reactive.recip()?),
        })
    }

    /// The reactance in ohm of a series element, or the susceptance in siemens of a shunt one,
    /// at a frequency in Hz.
    pub fn reactance_or_susceptance(&self, freq_hz: f64) -> Result<f64, RfError> {
        let omega = 2.0 * PI * crate::error::positive("frequency", freq_hz)?;
        let v = self.element.value;
        let positive_kind = matches!(
            (self.position, self.element.kind),
            (Position::Series, ElementKind::Inductor) | (Position::Shunt, ElementKind::Capacitor)
        );
        if positive_kind {
            Ok(omega * v)
        } else if v == 0.0 {
            Err(RfError::Singular {
                what: "an element of value zero".to_string(),
            })
        } else {
            Ok(-1.0 / (omega * v))
        }
    }
}

/// The ABCD matrix of a ladder, source side first.
pub fn ladder_abcd(parts: &[Part], freq_hz: f64) -> Result<Mat2, RfError> {
    let mut total = Mat2::identity();
    for part in parts {
        total = total.times(&part.abcd(freq_hz)?);
    }
    Ok(total)
}

/// The impedance seen from the source side of a ladder that ends on a load.
pub fn ladder_input_impedance(
    parts: &[Part],
    load: Complex,
    freq_hz: f64,
) -> Result<Complex, RfError> {
    input_impedance(&ladder_abcd(parts, freq_hz)?, load)
}

/// The two elements of an L network as a ladder, source side first.
pub fn l_parts(m: &LMatch) -> Vec<Part> {
    let series = Part {
        position: Position::Series,
        element: m.series,
    };
    let shunt = Part {
        position: Position::Shunt,
        element: m.shunt,
    };
    match m.topology {
        Topology::SeriesAtSource => vec![series, shunt],
        Topology::SeriesAtLoad => vec![shunt, series],
    }
}

/// The quality factor of an L network, `sqrt(R_high / R_low - 1)`, where `R_high` is the
/// parallel resistance of the side that has the shunt element and `R_low` the series resistance
/// of the other side.
pub fn l_network_q(m: &LMatch, source: Complex, load: Complex) -> Result<f64, RfError> {
    let ratio = match m.topology {
        Topology::SeriesAtSource => 1.0 / load.recip()?.re / source.re,
        Topology::SeriesAtLoad => 1.0 / source.recip()?.re / load.re,
    };
    Ok((ratio - 1.0).max(0.0).sqrt())
}

/// Whether a pi or T network passes low or high frequencies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassKind {
    /// Series inductors and shunt capacitors.
    LowPass,
    /// Series capacitors and shunt inductors.
    HighPass,
}

/// The shape of a three-element network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Shunt, series, shunt.
    Pi,
    /// Series, shunt, series.
    T,
}

/// A pi or T network that gives a source impedance the load it wants (the conjugate of the
/// source impedance at the input), with the loaded quality factor chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreeElement {
    /// Pi or T.
    pub shape: Shape,
    /// Low-pass or high-pass.
    pub pass: PassKind,
    /// The loaded quality factor asked for: the factor of the higher-resistance side of a pi
    /// network and of the lower-resistance side of a T network.
    pub loaded_q: f64,
    /// The virtual resistance in ohm: the one the two L sections meet at.
    pub virtual_ohm: f64,
    /// The quality factor of the section on the source side.
    pub q_source: f64,
    /// The quality factor of the section on the load side.
    pub q_load: f64,
    /// The three elements, source side first.
    pub parts: [Part; 3],
}

fn terminal(field: &str, z: Complex, settings: &RfSettings) -> Result<(), RfError> {
    settings.check_impedance_ohm(field, z)?;
    if z.re <= 0.0 {
        return Err(RfError::InvalidNumber {
            field: field.to_string(),
            reason: "the real part must be above zero".to_string(),
        });
    }
    Ok(())
}

fn checked_q(q: f64, minimum: f64, settings: &RfSettings) -> Result<(), RfError> {
    crate::error::positive("loaded Q", q)?;
    if q > settings.max_loaded_q {
        return Err(RfError::OutsideLimit {
            field: "loaded Q".to_string(),
            value: q,
            limit: settings.max_loaded_q,
        });
    }
    if q < minimum * (1.0 - settings.tolerance) {
        return Err(RfError::NoMatch {
            reason: format!(
                "the loaded Q must be at least {minimum:.6} for these impedances (the Q of an L network)"
            ),
        });
    }
    Ok(())
}

fn sign_of(pass: PassKind) -> f64 {
    match pass {
        PassKind::LowPass => 1.0,
        PassKind::HighPass => -1.0,
    }
}

/// The pi network (shunt, series, shunt) from a complex source impedance to a complex load
/// impedance at a frequency in Hz, with the loaded quality factor `q` of the side of the higher
/// parallel resistance. The reactance of the source and of the load is taken into the shunt
/// element next to it. `q` has to be at least that of the L network.
pub fn pi_match(
    source: Complex,
    load: Complex,
    freq_hz: f64,
    q: f64,
    pass: PassKind,
    settings: &RfSettings,
) -> Result<ThreeElement, RfError> {
    terminal("source impedance", source, settings)?;
    terminal("load impedance", load, settings)?;
    let omega = 2.0 * PI * settings.check_frequency_hz("frequency", freq_hz)?;
    let (ys, yl) = (source.recip()?, load.recip()?);
    let (rp_s, rp_l) = (1.0 / ys.re, 1.0 / yl.re);
    let (high, low) = (rp_s.max(rp_l), rp_s.min(rp_l));
    checked_q(q, (high / low - 1.0).max(0.0).sqrt(), settings)?;
    let virtual_ohm = high / (1.0 + q * q);
    let q_source = (rp_s / virtual_ohm - 1.0).max(0.0).sqrt();
    let q_load = (rp_l / virtual_ohm - 1.0).max(0.0).sqrt();
    let sign = sign_of(pass);
    let b_first = sign * q_source * ys.re - ys.im;
    let x_middle = sign * virtual_ohm * (q_source + q_load);
    let b_last = sign * q_load * yl.re - yl.im;
    Ok(ThreeElement {
        shape: Shape::Pi,
        pass,
        loaded_q: q,
        virtual_ohm,
        q_source,
        q_load,
        parts: [
            Part {
                position: Position::Shunt,
                element: shunt_element(b_first, omega),
            },
            Part {
                position: Position::Series,
                element: series_element(x_middle, omega),
            },
            Part {
                position: Position::Shunt,
                element: shunt_element(b_last, omega),
            },
        ],
    })
}

/// The T network (series, shunt, series) from a complex source impedance to a complex load
/// impedance at a frequency in Hz, with the loaded quality factor `q` of the side of the lower
/// series resistance. The reactance of the source and of the load is taken into the series
/// element next to it. `q` has to be at least that of the L network.
pub fn t_match(
    source: Complex,
    load: Complex,
    freq_hz: f64,
    q: f64,
    pass: PassKind,
    settings: &RfSettings,
) -> Result<ThreeElement, RfError> {
    terminal("source impedance", source, settings)?;
    terminal("load impedance", load, settings)?;
    let omega = 2.0 * PI * settings.check_frequency_hz("frequency", freq_hz)?;
    let (r_s, r_l) = (source.re, load.re);
    let (high, low) = (r_s.max(r_l), r_s.min(r_l));
    checked_q(q, (high / low - 1.0).max(0.0).sqrt(), settings)?;
    let virtual_ohm = low * (1.0 + q * q);
    let q_source = (virtual_ohm / r_s - 1.0).max(0.0).sqrt();
    let q_load = (virtual_ohm / r_l - 1.0).max(0.0).sqrt();
    let sign = sign_of(pass);
    let x_first = sign * q_source * r_s - source.im;
    let b_middle = sign * (q_source + q_load) / virtual_ohm;
    let x_last = sign * q_load * r_l - load.im;
    Ok(ThreeElement {
        shape: Shape::T,
        pass,
        loaded_q: q,
        virtual_ohm,
        q_source,
        q_load,
        parts: [
            Part {
                position: Position::Series,
                element: series_element(x_first, omega),
            },
            Part {
                position: Position::Shunt,
                element: shunt_element(b_middle, omega),
            },
            Part {
                position: Position::Series,
                element: series_element(x_last, omega),
            },
        ],
    })
}

impl ThreeElement {
    /// The impedance seen from the source side when the network ends on `load`, at a frequency
    /// in Hz; for a good match it is the conjugate of the source impedance.
    pub fn input_impedance(&self, load: Complex, freq_hz: f64) -> Result<Complex, RfError> {
        ladder_input_impedance(&self.parts, load, freq_hz)
    }
}

/// The bandwidth of a matching network, as an estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bandwidth {
    /// The quality factor of the network.
    pub q_network: f64,
    /// The rule of thumb `f0 / Q` in Hz: only an order of magnitude.
    pub rule_of_thumb_hz: f64,
    /// The lower frequency in Hz where the return loss falls to the limit of the settings, found
    /// by a scan, for a source and a load that are real; `None` when they are not real or when
    /// the scan range ends before the edge.
    pub low_hz: Option<f64>,
    /// The upper frequency in Hz where the return loss falls to the limit.
    pub high_hz: Option<f64>,
}

/// The estimate of the bandwidth of a ladder designed for `f0_hz`. The scan keeps the values of
/// the elements fixed (they do not follow the frequency) and goes from a quarter to four times
/// `f0_hz`.
pub fn bandwidth(
    parts: &[Part],
    source: Complex,
    load: Complex,
    f0_hz: f64,
    q_network: f64,
    settings: &RfSettings,
) -> Result<Bandwidth, RfError> {
    let f0 = settings.check_frequency_hz("frequency", f0_hz)?;
    let rule = if q_network > 0.0 {
        f0 / q_network
    } else {
        f64::INFINITY
    };
    let real = source.im.abs() <= settings.tolerance * source.re.abs()
        && load.im.abs() <= settings.tolerance * load.re.abs();
    if !real {
        return Ok(Bandwidth {
            q_network,
            rule_of_thumb_hz: rule,
            low_hz: None,
            high_hz: None,
        });
    }
    let limit = 10f64.powf(-settings.return_loss_bandwidth_db / 20.0);
    let passes = |f: f64| -> bool {
        let Ok(zin) = ladder_input_impedance(parts, load, f) else {
            return false;
        };
        let Ok(gamma) = (zin - source).checked_div(zin + source) else {
            return false;
        };
        gamma.abs() <= limit
    };
    if !passes(f0) {
        return Ok(Bandwidth {
            q_network,
            rule_of_thumb_hz: rule,
            low_hz: None,
            high_hz: None,
        });
    }
    let edge = |factor: f64| -> Option<f64> {
        let steps = settings.scan_points;
        let mut previous = f0;
        for k in 1..=steps {
            let f = f0 * factor.powf(k as f64 / steps as f64);
            if !passes(f) {
                let (mut good, mut bad) = (previous, f);
                for _ in 0..60 {
                    let middle = (good * bad).sqrt();
                    if passes(middle) {
                        good = middle;
                    } else {
                        bad = middle;
                    }
                }
                return Some((good * bad).sqrt());
            }
            previous = f;
        }
        None
    };
    Ok(Bandwidth {
        q_network,
        rule_of_thumb_hz: rule,
        low_hz: edge(0.25),
        high_hz: edge(4.0),
    })
}

/// One double-stub match: two shunt stubs of the same termination, the first at the load, the
/// second a distance `spacing_deg` towards the source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleStub {
    /// Open or short-circuited stubs.
    pub termination: Termination,
    /// The electrical length in degrees between the two stubs.
    pub spacing_deg: f64,
    /// The electrical length in degrees of the stub at the load.
    pub first_deg: f64,
    /// The electrical length in degrees of the stub at the source side.
    pub second_deg: f64,
}

fn stub_admittance(deg: f64, termination: Termination, z0_ohm: f64) -> Complex {
    let (sin, cos) = deg.to_radians().sin_cos();
    let normalised = match termination {
        Termination::Open => Complex::new(0.0, sin / cos),
        Termination::Short => Complex::new(0.0, -cos / sin),
    };
    normalised.scale(1.0 / z0_ohm)
}

/// The length in degrees of a stub whose normalised susceptance has to be `s`.
fn stub_length_deg(s: f64, termination: Termination) -> f64 {
    match termination {
        Termination::Open => half_turn(s.atan()).to_degrees(),
        Termination::Short => 1.0f64.atan2(-s).to_degrees(),
    }
}

impl DoubleStub {
    /// The impedance seen from the source side when the stubs, the line between them and the load
    /// are put together on a line of characteristic impedance `z0_ohm`; for a good match it is
    /// `z0_ohm`.
    pub fn input_impedance(&self, load: Complex, z0_ohm: f64) -> Result<Complex, RfError> {
        let first = abcd_shunt(stub_admittance(self.first_deg, self.termination, z0_ohm));
        let second = abcd_shunt(stub_admittance(self.second_deg, self.termination, z0_ohm));
        let line = abcd_lossless_line(z0_ohm, self.spacing_deg.to_radians())?;
        input_impedance(&second.times(&line).times(&first), load)
    }
}

/// The double-stub matches of a load on a line of characteristic impedance `z0_ohm`, with the
/// stubs `spacing_deg` apart (strictly between 0 and 180 degrees). There are two solutions, or
/// one, or none: a load whose normalised conductance is above `1 / sin^2(spacing)` is in the
/// forbidden region of the double stub and has no match (change the spacing).
pub fn double_stub_match(
    load: Complex,
    z0_ohm: f64,
    spacing_deg: f64,
    termination: Termination,
    settings: &RfSettings,
) -> Result<Vec<DoubleStub>, RfError> {
    settings.check_reference_ohm("characteristic impedance", z0_ohm)?;
    terminal("load impedance", load, settings)?;
    if !spacing_deg.is_finite() || spacing_deg <= 0.0 || spacing_deg >= 180.0 {
        return Err(RfError::InvalidNumber {
            field: "stub spacing".to_string(),
            reason: "must be above 0 and below 180 degrees".to_string(),
        });
    }
    let theta = spacing_deg.to_radians();
    if theta.sin().abs() < 1e-6 {
        return Err(RfError::InvalidNumber {
            field: "stub spacing".to_string(),
            reason: "is too close to 0 or 180 degrees".to_string(),
        });
    }
    let t = theta.tan();
    let y = load.scale(1.0 / z0_ohm).recip()?;
    let (g, b) = (y.re, y.im);
    let scale = g * (1.0 + t * t);
    let radicand = scale - g * g * t * t;
    if radicand < -settings.tolerance * scale {
        return Err(RfError::NoMatch {
            reason: format!(
                "the load is in the forbidden region of a double stub spaced {spacing_deg} degrees \
                 (its normalised conductance {g:.6} is above {:.6}); change the spacing",
                1.0 / theta.sin().powi(2)
            ),
        });
    }
    let root = radicand.max(0.0).sqrt();
    let roots: Vec<f64> = if root <= settings.tolerance * scale.sqrt() {
        vec![0.0]
    } else {
        vec![root, -root]
    };
    let mut found = Vec::new();
    for r in roots {
        let big_b = (1.0 - r) / t;
        let after_first = Complex::new(g, big_b);
        let moved = (after_first + Complex::new(0.0, t))
            .checked_div(Complex::ONE + Complex::new(0.0, t) * after_first)?;
        found.push(DoubleStub {
            termination,
            spacing_deg,
            first_deg: stub_length_deg(big_b - b, termination),
            second_deg: stub_length_deg(-moved.im, termination),
        });
    }
    found.sort_by(|a, c| a.first_deg.total_cmp(&c.first_deg));
    Ok(found)
}

/// A single-stub match with its lengths in metres on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StubOnLine {
    /// The match.
    pub stub: StubMatch,
    /// The distance from the load to the stub in metres.
    pub distance_m: f64,
    /// The length of the stub in metres.
    pub stub_m: f64,
}

/// A double-stub match with its lengths in metres on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleStubOnLine {
    /// The match.
    pub stubs: DoubleStub,
    /// The distance between the stubs in metres.
    pub spacing_m: f64,
    /// The length of the stub at the load in metres.
    pub first_m: f64,
    /// The length of the other stub in metres.
    pub second_m: f64,
}

/// A quarter-wave match with its lengths in metres on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuarterWaveOnLine {
    /// The match.
    pub transformer: QuarterWave,
    /// The distance from the load to the transformer in metres.
    pub distance_m: f64,
    /// The length of the quarter-wave section in metres.
    pub section_m: f64,
}

/// What the line equivalents of a match are on one line.
#[derive(Clone, Debug, PartialEq)]
pub struct LineEquivalents {
    /// The line (impedance and effective permittivity).
    pub line: Line,
    /// The frequency in Hz.
    pub freq_hz: f64,
    /// The single-stub matches, shunt stubs, short-circuited ones first and then open ones.
    pub single: Vec<StubOnLine>,
    /// The double-stub matches, short-circuited ones first and then open ones.
    pub double: Vec<DoubleStubOnLine>,
    /// The quarter-wave matches.
    pub quarter_wave: Vec<QuarterWaveOnLine>,
    /// The kinds that have no solution for this load, each with the reason.
    pub unavailable: Vec<(String, String)>,
}

/// The single-stub, double-stub and quarter-wave matches of a load on a line, with their
/// lengths in metres at a frequency in Hz. A kind with no solution is listed in `unavailable`
/// and does not stop the others.
pub fn line_equivalents(
    load: Complex,
    line: &Line,
    freq_hz: f64,
    spacing_deg: f64,
    settings: &RfSettings,
) -> Result<LineEquivalents, RfError> {
    settings.check_frequency_hz("frequency", freq_hz)?;
    let mut result = LineEquivalents {
        line: *line,
        freq_hz,
        single: Vec::new(),
        double: Vec::new(),
        quarter_wave: Vec::new(),
        unavailable: Vec::new(),
    };
    for (name, end) in [("short", Termination::Short), ("open", Termination::Open)] {
        match stub_match(load, line.z0_ohm, StubKind::Shunt, end, settings) {
            Ok(list) => {
                for stub in list {
                    result.single.push(StubOnLine {
                        stub,
                        distance_m: line.length_m(stub.distance_deg, freq_hz)?,
                        stub_m: line.length_m(stub.stub_deg, freq_hz)?,
                    });
                }
            }
            Err(error @ RfError::NoMatch { .. }) => result
                .unavailable
                .push((format!("single stub, {name}"), error.to_string())),
            Err(other) => return Err(other),
        }
        match double_stub_match(load, line.z0_ohm, spacing_deg, end, settings) {
            Ok(list) => {
                for stubs in list {
                    result.double.push(DoubleStubOnLine {
                        stubs,
                        spacing_m: line.length_m(stubs.spacing_deg, freq_hz)?,
                        first_m: line.length_m(stubs.first_deg, freq_hz)?,
                        second_m: line.length_m(stubs.second_deg, freq_hz)?,
                    });
                }
            }
            Err(error @ RfError::NoMatch { .. }) => result
                .unavailable
                .push((format!("double stub, {name}"), error.to_string())),
            Err(other) => return Err(other),
        }
    }
    match quarter_wave(load, line.z0_ohm, settings) {
        Ok(list) => {
            for transformer in list {
                result.quarter_wave.push(QuarterWaveOnLine {
                    transformer,
                    distance_m: line.length_m(transformer.distance_deg, freq_hz)?,
                    section_m: line.length_m(QuarterWave::SECTION_DEG, freq_hz)?,
                });
            }
        }
        Err(error @ RfError::NoMatch { .. }) => result
            .unavailable
            .push(("quarter-wave".to_string(), error.to_string())),
        Err(other) => return Err(other),
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::l_match;

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    fn close(a: Complex, b: Complex, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance * (1.0 + b.abs())
    }

    #[test]
    fn a_pi_network_between_real_resistances_has_the_textbook_values() {
        // 50 ohm to 100 ohm, loaded Q 5 at 100 MHz: the virtual resistance is 100 / 26
        let net = pi_match(
            Complex::real(50.0),
            Complex::real(100.0),
            1e8,
            5.0,
            PassKind::LowPass,
            &settings(),
        )
        .unwrap();
        assert!((net.virtual_ohm - 100.0 / 26.0).abs() < 1e-12);
        assert!((net.q_source - 12.0f64.sqrt()).abs() < 1e-12);
        assert!((net.q_load - 5.0).abs() < 1e-12);
        let omega = 2.0 * PI * 1e8;
        // shunt C = Q / (w R) on each side, series L = (Q1 + Q2) Rv / w
        assert!((net.parts[0].element.value - 12.0f64.sqrt() / (omega * 50.0)).abs() < 1e-18);
        assert!((net.parts[2].element.value - 5.0 / (omega * 100.0)).abs() < 1e-18);
        assert!(
            (net.parts[1].element.value - (12.0f64.sqrt() + 5.0) * 100.0 / 26.0 / omega).abs()
                < 1e-15
        );
        assert_eq!(net.parts[0].element.kind, ElementKind::Capacitor);
        assert_eq!(net.parts[1].element.kind, ElementKind::Inductor);
        let zin = net.input_impedance(Complex::real(100.0), 1e8).unwrap();
        assert!(close(zin, Complex::real(50.0), 1e-9), "{zin:?}");
    }

    #[test]
    fn a_t_network_between_real_resistances_has_the_textbook_values() {
        // 50 ohm to 100 ohm, loaded Q 4 at 100 MHz: the virtual resistance is 50 * 17
        let net = t_match(
            Complex::real(50.0),
            Complex::real(100.0),
            1e8,
            4.0,
            PassKind::LowPass,
            &settings(),
        )
        .unwrap();
        assert!((net.virtual_ohm - 850.0).abs() < 1e-9);
        assert!((net.q_source - 4.0).abs() < 1e-12);
        assert!((net.q_load - (8.5f64 - 1.0).sqrt()).abs() < 1e-12);
        let zin = net.input_impedance(Complex::real(100.0), 1e8).unwrap();
        assert!(close(zin, Complex::real(50.0), 1e-9), "{zin:?}");
    }

    #[test]
    fn pi_and_t_networks_match_complex_terminations_in_both_passes() {
        let cases = [
            (Complex::new(50.0, 10.0), Complex::new(120.0, -30.0)),
            (Complex::new(75.0, -20.0), Complex::new(20.0, 15.0)),
            (Complex::real(10.0), Complex::new(200.0, 80.0)),
        ];
        for (source, load) in cases {
            for pass in [PassKind::LowPass, PassKind::HighPass] {
                for q in [15.0, 40.0] {
                    let pi = pi_match(source, load, 2.4e9, q, pass, &settings()).unwrap();
                    let t = t_match(source, load, 2.4e9, q, pass, &settings()).unwrap();
                    for net in [pi, t] {
                        let zin = net.input_impedance(load, 2.4e9).unwrap();
                        assert!(
                            close(zin, source.conj(), 1e-9),
                            "{:?} {pass:?} q {q}: {zin:?}",
                            net.shape
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_loaded_q_below_the_l_network_is_refused() {
        // 50 to 100 ohm needs a Q of at least 1
        for result in [
            pi_match(
                Complex::real(50.0),
                Complex::real(100.0),
                1e8,
                0.5,
                PassKind::LowPass,
                &settings(),
            ),
            t_match(
                Complex::real(50.0),
                Complex::real(100.0),
                1e8,
                0.5,
                PassKind::LowPass,
                &settings(),
            ),
        ] {
            assert!(matches!(result, Err(RfError::NoMatch { .. })));
        }
        // a Q equal to the minimum is the L network
        let edge = pi_match(
            Complex::real(50.0),
            Complex::real(100.0),
            1e8,
            1.0,
            PassKind::LowPass,
            &settings(),
        )
        .unwrap();
        assert!(edge.q_load.abs() < 1e-6 || edge.q_source.abs() < 1e-6);
    }

    #[test]
    fn bad_inputs_of_pi_and_t_are_refused() {
        let s = settings();
        let f = |source: Complex, load: Complex, freq: f64, q: f64| {
            pi_match(source, load, freq, q, PassKind::LowPass, &s)
        };
        let one = Complex::real(50.0);
        assert!(f(one, one, 1e8, 0.0).is_err());
        assert!(f(one, one, 1e8, f64::NAN).is_err());
        assert!(matches!(
            f(one, one, 1e8, 1e6),
            Err(RfError::OutsideLimit { .. })
        ));
        assert!(f(Complex::new(-5.0, 0.0), one, 1e8, 5.0).is_err());
        assert!(f(one, Complex::new(0.0, 10.0), 1e8, 5.0).is_err());
        assert!(f(one, one, 0.0, 5.0).is_err());
        assert!(f(one, one, 1e15, 5.0).is_err());
    }

    #[test]
    fn the_l_network_q_is_the_square_root_of_the_ratio_minus_one() {
        let s = settings();
        let found = l_match(Complex::real(50.0), Complex::real(100.0), 1e8, &s).unwrap();
        for m in &found {
            let q = l_network_q(m, Complex::real(50.0), Complex::real(100.0)).unwrap();
            assert!((q - 1.0).abs() < 1e-9, "{q}");
            let parts = l_parts(m);
            assert_eq!(parts.len(), 2);
            let zin = ladder_input_impedance(&parts, Complex::real(100.0), 1e8).unwrap();
            assert!(close(zin, Complex::real(50.0), 1e-9), "{zin:?}");
        }
    }

    #[test]
    fn the_bandwidth_narrows_when_the_q_rises() {
        let s = settings();
        let source = Complex::real(50.0);
        let load = Complex::real(100.0);
        let narrow = pi_match(source, load, 1e8, 20.0, PassKind::LowPass, &s).unwrap();
        let wide = pi_match(source, load, 1e8, 2.0, PassKind::LowPass, &s).unwrap();
        let width = |net: &ThreeElement| {
            let b = bandwidth(&net.parts, source, load, 1e8, net.loaded_q, &s).unwrap();
            (b.high_hz.unwrap() - b.low_hz.unwrap(), b)
        };
        let (narrow_width, narrow_b) = width(&narrow);
        let (wide_width, wide_b) = width(&wide);
        assert!(narrow_width < wide_width, "{narrow_width} {wide_width}");
        assert!(narrow_b.low_hz.unwrap() < 1e8 && narrow_b.high_hz.unwrap() > 1e8);
        assert!((narrow_b.rule_of_thumb_hz - 5e6).abs() < 1.0);
        assert!((wide_b.rule_of_thumb_hz - 5e7).abs() < 1.0);
    }

    #[test]
    fn the_bandwidth_of_a_complex_termination_has_only_the_rule_of_thumb() {
        let s = settings();
        let source = Complex::new(50.0, 10.0);
        let load = Complex::real(100.0);
        let net = pi_match(source, load, 1e8, 5.0, PassKind::LowPass, &s).unwrap();
        let b = bandwidth(&net.parts, source, load, 1e8, 5.0, &s).unwrap();
        assert_eq!((b.low_hz, b.high_hz), (None, None));
        assert!(b.rule_of_thumb_hz > 0.0);
    }

    #[test]
    fn the_double_stub_gives_the_input_impedance_of_the_line() {
        let s = settings();
        let loads = [
            Complex::new(60.0, -40.0),
            Complex::new(25.0, 30.0),
            Complex::real(100.0),
            Complex::new(40.0, 0.0),
        ];
        for load in loads {
            for end in [Termination::Short, Termination::Open] {
                for spacing in [45.0, 135.0, 60.0] {
                    let found = double_stub_match(load, 50.0, spacing, end, &s).unwrap();
                    assert!(!found.is_empty());
                    for m in found {
                        let zin = m.input_impedance(load, 50.0).unwrap();
                        assert!(
                            close(zin, Complex::real(50.0), 1e-8),
                            "{load:?} {end:?} {spacing}: {zin:?}"
                        );
                        assert!((0.0..=180.0).contains(&m.first_deg));
                        assert!((0.0..=180.0).contains(&m.second_deg));
                    }
                }
            }
        }
    }

    #[test]
    fn a_load_in_the_forbidden_region_of_the_double_stub_has_no_match() {
        // a normalised conductance of 3 is above 1 / sin^2(45 degrees) = 2
        let load = Complex::real(50.0 / 3.0);
        let result = double_stub_match(load, 50.0, 45.0, Termination::Short, &settings());
        assert!(matches!(result, Err(RfError::NoMatch { .. })));
        // a smaller spacing reaches it (the limit is 4 at 30 degrees); 135 degrees has the same
        // limit as 45 degrees and 60 degrees has a lower one
        assert!(double_stub_match(load, 50.0, 30.0, Termination::Short, &settings()).is_ok());
        for spacing in [135.0, 60.0] {
            assert!(
                double_stub_match(load, 50.0, spacing, Termination::Short, &settings()).is_err()
            );
        }
    }

    #[test]
    fn bad_spacings_of_the_double_stub_are_refused() {
        for bad in [0.0, -10.0, 180.0, 200.0, f64::NAN, 1e-9] {
            assert!(
                double_stub_match(
                    Complex::real(100.0),
                    50.0,
                    bad,
                    Termination::Open,
                    &settings()
                )
                .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_matched_load_needs_stubs_of_length_zero_or_a_half_wave() {
        let found = double_stub_match(
            Complex::real(50.0),
            50.0,
            45.0,
            Termination::Open,
            &settings(),
        )
        .unwrap();
        assert!(!found.is_empty());
        let first = found[0];
        assert!(first.first_deg.abs() < 1e-6 || (first.first_deg - 180.0).abs() < 1e-6);
    }

    #[test]
    fn the_line_equivalents_have_lengths_in_metres() {
        let s = settings();
        let line = Line::new(50.0, 3.0).unwrap();
        let load = Complex::new(30.0, 40.0);
        let eq = line_equivalents(load, &line, 2.4e9, 45.0, &s).unwrap();
        assert!(!eq.single.is_empty() && !eq.double.is_empty() && !eq.quarter_wave.is_empty());
        let wavelength = line.wavelength_m(2.4e9).unwrap();
        for item in &eq.single {
            assert!((item.stub_m - item.stub.stub_deg / 360.0 * wavelength).abs() < 1e-12);
        }
        for item in &eq.quarter_wave {
            assert!((item.section_m - wavelength / 4.0).abs() < 1e-12);
        }
        for item in &eq.double {
            assert!((item.spacing_m - wavelength / 8.0).abs() < 1e-12);
        }
        assert!(eq.unavailable.is_empty());
    }

    #[test]
    fn a_kind_with_no_solution_is_listed_and_the_others_go_on() {
        let s = settings();
        let line = Line::new(50.0, 1.0).unwrap();
        let load = Complex::real(50.0 / 3.0);
        let eq = line_equivalents(load, &line, 1e9, 45.0, &s).unwrap();
        assert!(eq
            .unavailable
            .iter()
            .any(|(kind, _)| kind.starts_with("double stub")));
        assert!(!eq.single.is_empty());
    }

    #[test]
    fn the_susceptance_of_a_part_agrees_with_its_matrix() {
        let part = Part {
            position: Position::Shunt,
            element: Element {
                kind: ElementKind::Inductor,
                value: 10e-9,
            },
        };
        let b = part.reactance_or_susceptance(1e8).unwrap();
        assert!((b + 1.0 / (2.0 * PI * 1e8 * 10e-9)).abs() < 1e-9);
        let m = part.abcd(1e8).unwrap();
        assert!((m.m21.im - b).abs() < 1e-9 && m.m21.re.abs() < 1e-12);
    }
}
