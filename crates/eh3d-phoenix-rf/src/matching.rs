// SPDX-License-Identifier: GPL-3.0-or-later
//! Impedance matching: the L network, the single stub and the quarter-wave transformer.
//!
//! * **L network.** Two reactive elements, one in series and one in shunt, that turn a complex
//!   load into the complex conjugate of a complex source impedance (the condition of maximum
//!   power transfer; for a real source it is simply a match to that resistance). Both
//!   topologies are tried, each with its two solutions, and every solution that exists is
//!   returned with the value of its components at the frequency asked.
//! * **Single stub.** A line of the characteristic impedance `z0` between the load and the
//!   stub, and a stub (shunt or series, open or short-circuited) that cancels the reactive part
//!   (Pozar, *Microwave Engineering*, section 5.2). Lengths are electrical lengths in degrees.
//! * **Quarter-wave transformer.** `z = sqrt(z1 z2)` for two real impedances; for a complex load
//!   a line of the characteristic impedance `z0` first brings the load to a real value.
//!
//! The source and load impedances must have a real part above zero. The lines are lossless.

use crate::complex::Complex;
use crate::error::RfError;
use crate::network::{
    abcd_lossless_line, abcd_series, abcd_shunt, input_impedance, reflection_from_impedance, Mat2,
};
use crate::settings::RfSettings;
use std::f64::consts::PI;

/// The kind of a reactive element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    /// An inductor.
    Inductor,
    /// A capacitor.
    Capacitor,
}

/// A reactive element with its value in henry or farad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Element {
    /// Whether it is an inductor or a capacitor.
    pub kind: ElementKind,
    /// The inductance in henry or the capacitance in farad.
    pub value: f64,
}

/// The position of the series element of an L network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topology {
    /// The series element is on the source side and the shunt element on the load side.
    SeriesAtSource,
    /// The shunt element is on the source side and the series element on the load side.
    SeriesAtLoad,
}

/// How the network treats frequency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LKind {
    /// Series inductor and shunt capacitor: passes low frequencies.
    LowPass,
    /// Series capacitor and shunt inductor: passes high frequencies.
    HighPass,
    /// Two inductors or two capacitors.
    Mixed,
}

/// One L network that matches a load to a source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LMatch {
    /// Which element is on the source side.
    pub topology: Topology,
    /// The reactance of the series element in ohm at the design frequency.
    pub series_reactance_ohm: f64,
    /// The susceptance of the shunt element in siemens at the design frequency.
    pub shunt_susceptance_s: f64,
    /// The series element as a component.
    pub series: Element,
    /// The shunt element as a component.
    pub shunt: Element,
    /// Low-pass, high-pass or mixed.
    pub kind: LKind,
}

impl LMatch {
    /// The ABCD matrix of the network, from the source side to the load side.
    pub fn abcd(&self) -> Mat2 {
        let series = abcd_series(Complex::new(0.0, self.series_reactance_ohm));
        let shunt = abcd_shunt(Complex::new(0.0, self.shunt_susceptance_s));
        match self.topology {
            Topology::SeriesAtSource => series.times(&shunt),
            Topology::SeriesAtLoad => shunt.times(&series),
        }
    }

    /// The impedance seen from the source side when the network ends on `load`; for a good
    /// match it is the conjugate of the source impedance.
    pub fn input_impedance(&self, load: Complex) -> Result<Complex, RfError> {
        input_impedance(&self.abcd(), load)
    }
}

pub(crate) fn series_element(reactance_ohm: f64, omega: f64) -> Element {
    if reactance_ohm >= 0.0 {
        Element {
            kind: ElementKind::Inductor,
            value: reactance_ohm / omega,
        }
    } else {
        Element {
            kind: ElementKind::Capacitor,
            value: -1.0 / (omega * reactance_ohm),
        }
    }
}

pub(crate) fn shunt_element(susceptance_s: f64, omega: f64) -> Element {
    if susceptance_s >= 0.0 {
        Element {
            kind: ElementKind::Capacitor,
            value: susceptance_s / omega,
        }
    } else {
        Element {
            kind: ElementKind::Inductor,
            value: -1.0 / (omega * susceptance_s),
        }
    }
}

fn l_kind(series: Element, shunt: Element) -> LKind {
    match (series.kind, shunt.kind) {
        (ElementKind::Inductor, ElementKind::Capacitor) => LKind::LowPass,
        (ElementKind::Capacitor, ElementKind::Inductor) => LKind::HighPass,
        _ => LKind::Mixed,
    }
}

fn passive(field: &str, z: Complex, settings: &RfSettings) -> Result<(), RfError> {
    settings.check_impedance_ohm(field, z)?;
    if z.re <= 0.0 {
        return Err(RfError::InvalidNumber {
            field: field.to_string(),
            reason: "the real part must be above zero".to_string(),
        });
    }
    Ok(())
}

fn make_l(
    topology: Topology,
    series_reactance_ohm: f64,
    shunt_susceptance_s: f64,
    omega: f64,
) -> LMatch {
    let series = series_element(series_reactance_ohm, omega);
    let shunt = shunt_element(shunt_susceptance_s, omega);
    LMatch {
        topology,
        series_reactance_ohm,
        shunt_susceptance_s,
        series,
        shunt,
        kind: l_kind(series, shunt),
    }
}

/// The square roots with which a quadratic's two solutions are formed: one when the radicand is
/// zero within the tolerance, two otherwise, none when it is clearly negative.
fn roots(radicand: f64, scale: f64, tolerance: f64) -> Vec<f64> {
    if radicand < -tolerance * scale {
        Vec::new()
    } else if radicand <= tolerance * scale {
        vec![0.0]
    } else {
        let r = radicand.sqrt();
        vec![r, -r]
    }
}

/// The L networks that give a source impedance `source` the load `load` it wants (the conjugate
/// of the source impedance at the input of the network), with their components at a frequency
/// in Hz. Returns [`RfError::NoMatch`] when neither topology has a solution.
pub fn l_match(
    source: Complex,
    load: Complex,
    freq_hz: f64,
    settings: &RfSettings,
) -> Result<Vec<LMatch>, RfError> {
    passive("source impedance", source, settings)?;
    passive("load impedance", load, settings)?;
    let omega = 2.0 * PI * settings.check_frequency_hz("frequency", freq_hz)?;
    let tol = settings.tolerance;
    let mut found = Vec::new();

    // series element on the source side, shunt element across the load
    let y_load = load.recip()?;
    let (g_l, b_l) = (y_load.re, y_load.im);
    let r_s = source.re;
    for s in roots(g_l / r_s - g_l * g_l, g_l / r_s, tol) {
        let total = s; // b_l + b_shunt
        let shunt = -b_l + s;
        let series = -source.im + total * r_s / g_l;
        found.push(make_l(Topology::SeriesAtSource, series, shunt, omega));
    }

    // shunt element on the source side, series element on the load side
    let y_source = source.recip()?;
    let (g_s, b_s) = (y_source.re, y_source.im);
    let r_l = load.re;
    for x in roots(r_l / g_s - r_l * r_l, r_l / g_s, tol) {
        let series = -load.im + x;
        let shunt = -b_s + x * g_s / r_l;
        found.push(make_l(Topology::SeriesAtLoad, series, shunt, omega));
    }

    if found.is_empty() {
        return Err(RfError::NoMatch {
            reason: "an L network needs the source resistance, or the load resistance, \
                     on the right side of the other; use two sections or another network"
                .to_string(),
        });
    }
    Ok(found)
}

/// Where the stub is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StubKind {
    /// In parallel with the line.
    Shunt,
    /// In series with the line.
    Series,
}

/// How the far end of the stub is terminated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// Open circuit.
    Open,
    /// Short circuit.
    Short,
}

/// One single-stub match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StubMatch {
    /// Shunt or series stub.
    pub kind: StubKind,
    /// Open or short-circuited stub.
    pub termination: Termination,
    /// The electrical length in degrees of the line from the load to the stub.
    pub distance_deg: f64,
    /// The electrical length in degrees of the stub.
    pub stub_deg: f64,
}

impl StubMatch {
    /// The distance from the load to the stub in wavelengths.
    pub fn distance_wavelengths(&self) -> f64 {
        self.distance_deg / 360.0
    }

    /// The length of the stub in wavelengths.
    pub fn stub_wavelengths(&self) -> f64 {
        self.stub_deg / 360.0
    }

    /// The impedance seen from the source side when the line, the stub and the load are put
    /// together with the characteristic impedance `z0_ohm`; for a good match it is `z0_ohm`.
    pub fn input_impedance(&self, load: Complex, z0_ohm: f64) -> Result<Complex, RfError> {
        let line = abcd_lossless_line(z0_ohm, self.distance_deg.to_radians())?;
        let theta = self.stub_deg.to_radians();
        let (sin, cos) = theta.sin_cos();
        let open = self.termination == Termination::Open;
        let stub = match self.kind {
            StubKind::Shunt => {
                // an open stub has the admittance j tan, a short one -j cot, in units of 1/z0
                let y = if open {
                    Complex::new(0.0, sin / cos)
                } else {
                    Complex::new(0.0, -cos / sin)
                };
                abcd_shunt(y.scale(1.0 / z0_ohm))
            }
            StubKind::Series => {
                // an open stub has the impedance -j cot, a short one j tan, in units of z0
                let z = if open {
                    Complex::new(0.0, -cos / sin)
                } else {
                    Complex::new(0.0, sin / cos)
                };
                abcd_series(z.scale(z0_ohm))
            }
        };
        input_impedance(&stub.times(&line), load)
    }
}

/// An angle brought into the range zero (included) to 180 degrees (excluded), in radians.
pub(crate) fn half_turn(theta: f64) -> f64 {
    let wrapped = theta.rem_euclid(PI);
    if wrapped >= PI {
        0.0
    } else {
        wrapped
    }
}

/// The single-stub matches of a load on a line of characteristic impedance `z0_ohm`, for a
/// kind of stub and a termination. Every solution that exists is returned, the nearest to the
/// load first.
pub fn stub_match(
    load: Complex,
    z0_ohm: f64,
    kind: StubKind,
    termination: Termination,
    settings: &RfSettings,
) -> Result<Vec<StubMatch>, RfError> {
    settings.check_reference_ohm("characteristic impedance", z0_ohm)?;
    passive("load impedance", load, settings)?;
    let tol = settings.tolerance;
    let z_norm = load.scale(1.0 / z0_ohm);
    // `w` is the normalised admittance for a shunt stub and the normalised impedance for a
    // series one; the two cases are the same calculation on `w`
    let w = match kind {
        StubKind::Shunt => z_norm.recip()?,
        StubKind::Series => z_norm,
    };
    let (g, b) = (w.re, w.im);
    if (g - 1.0).abs() <= tol && b.abs() <= tol {
        return Ok(vec![StubMatch {
            kind,
            termination,
            distance_deg: 0.0,
            stub_deg: 0.0,
        }]);
    }
    // Re[(w cos + j sin) / (cos + j w sin)] = 1 gives, with t = tan(theta),
    // (g - g^2 - b^2) t^2 + 2 b t + (g - 1) = 0
    let a = g - g * g - b * b;
    let mut thetas: Vec<f64> = Vec::new();
    if a.abs() <= tol {
        thetas.push(PI / 2.0);
        if b.abs() > tol {
            thetas.push(half_turn(((1.0 - g) / (2.0 * b)).atan()));
        }
    } else {
        let disc = b * b - a * (g - 1.0);
        if disc < -tol {
            return Err(RfError::NoMatch {
                reason: "the load has no point of unit conductance along the line".to_string(),
            });
        }
        let root = disc.max(0.0).sqrt();
        thetas.push(half_turn(((-b + root) / a).atan()));
        if root > tol {
            thetas.push(half_turn(((-b - root) / a).atan()));
        }
    }
    thetas.sort_by(f64::total_cmp);
    let mut found = Vec::new();
    for theta in thetas {
        let (sin, cos) = theta.sin_cos();
        let moved = (w.scale(cos) + Complex::new(0.0, sin))
            .checked_div(Complex::real(cos) + Complex::new(0.0, sin) * w)?;
        let p = moved.im;
        let tan_type = (kind == StubKind::Shunt) == (termination == Termination::Open);
        let stub = if tan_type {
            half_turn((-p).atan())
        } else {
            1.0f64.atan2(p)
        };
        found.push(StubMatch {
            kind,
            termination,
            distance_deg: theta.to_degrees(),
            stub_deg: stub.to_degrees(),
        });
    }
    Ok(found)
}

/// One quarter-wave match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuarterWave {
    /// The electrical length in degrees of the line of impedance `z0` between the load and the
    /// transformer (zero when the load is real).
    pub distance_deg: f64,
    /// The real impedance in ohm at the end of that line, where the transformer is connected.
    pub real_impedance_ohm: f64,
    /// The characteristic impedance in ohm of the quarter-wave section.
    pub section_ohm: f64,
}

impl QuarterWave {
    /// The electrical length of the transformer section: a quarter of a turn.
    pub const SECTION_DEG: f64 = 90.0;

    /// The reflection coefficient seen from the source side, against `z0_ohm`, when the line, the
    /// section and the load are put together; it is zero for a good match.
    pub fn reflection(&self, load: Complex, z0_ohm: f64) -> Result<Complex, RfError> {
        let line = abcd_lossless_line(z0_ohm, self.distance_deg.to_radians())?;
        let section = abcd_lossless_line(self.section_ohm, PI / 2.0)?;
        let zin = input_impedance(&section.times(&line), load)?;
        reflection_from_impedance(zin, z0_ohm)
    }
}

/// The characteristic impedance of the quarter-wave transformer between two real impedances,
/// `sqrt(z1 z2)`.
pub fn quarter_wave_impedance(
    z1_ohm: f64,
    z2_ohm: f64,
    settings: &RfSettings,
) -> Result<f64, RfError> {
    settings.check_reference_ohm("first impedance", z1_ohm)?;
    settings.check_reference_ohm("second impedance", z2_ohm)?;
    Ok((z1_ohm * z2_ohm).sqrt())
}

/// The quarter-wave matches of a load on a line of characteristic impedance `z0_ohm`: a real
/// load needs the transformer alone; a complex one needs first a length of line that brings it
/// to a real impedance, which happens at two places a quarter wave apart.
pub fn quarter_wave(
    load: Complex,
    z0_ohm: f64,
    settings: &RfSettings,
) -> Result<Vec<QuarterWave>, RfError> {
    settings.check_reference_ohm("characteristic impedance", z0_ohm)?;
    passive("load impedance", load, settings)?;
    let tol = settings.tolerance;
    let z = load.scale(1.0 / z0_ohm);
    if z.im.abs() <= tol * z.abs() {
        return Ok(vec![QuarterWave {
            distance_deg: 0.0,
            real_impedance_ohm: load.re,
            section_ohm: (z0_ohm * load.re).sqrt(),
        }]);
    }
    // Im[(z cos + j sin) / (cos + j z sin)] = 0 gives x t^2 + (|z|^2 - 1) t - x = 0
    let m = z.norm_sqr() - 1.0;
    let root = (m * m + 4.0 * z.im * z.im).sqrt();
    let mut thetas = vec![
        half_turn(((-m + root) / (2.0 * z.im)).atan()),
        half_turn(((-m - root) / (2.0 * z.im)).atan()),
    ];
    thetas.sort_by(f64::total_cmp);
    let mut found = Vec::new();
    for theta in thetas {
        let (sin, cos) = theta.sin_cos();
        let moved = (z.scale(cos) + Complex::new(0.0, sin))
            .checked_div(Complex::real(cos) + Complex::new(0.0, sin) * z)?;
        let real = moved.re * z0_ohm;
        if real <= 0.0 || !real.is_finite() {
            return Err(RfError::NoMatch {
                reason: "the line brings the load to no positive real impedance".to_string(),
            });
        }
        found.push(QuarterWave {
            distance_deg: theta.to_degrees(),
            real_impedance_ohm: real,
            section_ohm: (z0_ohm * real).sqrt(),
        });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    #[test]
    fn the_book_l_network_from_50_to_100_ohm() {
        // source 50 ohm, load 100 ohm, 100 MHz: Q = 1, X = 50 ohm, B = 0.01 S
        let found = l_match(
            Complex::real(50.0),
            Complex::real(100.0),
            100e6,
            &settings(),
        )
        .unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|m| m.topology == Topology::SeriesAtSource));
        let omega = 2.0 * PI * 100e6;
        let low = found.iter().find(|m| m.kind == LKind::LowPass).unwrap();
        assert_eq!(low.series.kind, ElementKind::Inductor);
        assert_eq!(low.shunt.kind, ElementKind::Capacitor);
        assert!((low.series_reactance_ohm - 50.0).abs() < 1e-9);
        assert!((low.shunt_susceptance_s - 0.01).abs() < 1e-12);
        assert!((low.series.value - 50.0 / omega).abs() < 1e-18);
        assert!((low.shunt.value - 0.01 / omega).abs() < 1e-21);
        // 79.6 nH and 15.9 pF
        assert!((low.series.value * 1e9 - 79.577).abs() < 1e-2);
        assert!((low.shunt.value * 1e12 - 15.915).abs() < 1e-2);
        let high = found.iter().find(|m| m.kind == LKind::HighPass).unwrap();
        assert_eq!(high.series.kind, ElementKind::Capacitor);
        assert_eq!(high.shunt.kind, ElementKind::Inductor);
        assert!((high.series.value - 1.0 / (omega * 50.0)).abs() < 1e-18);
        assert!((high.shunt.value - 100.0 / omega).abs() < 1e-15);
    }

    #[test]
    fn the_book_l_network_from_100_to_50_ohm_puts_the_shunt_on_the_source() {
        let found = l_match(
            Complex::real(100.0),
            Complex::real(50.0),
            100e6,
            &settings(),
        )
        .unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|m| m.topology == Topology::SeriesAtLoad));
        for m in &found {
            assert!((m.series_reactance_ohm.abs() - 50.0).abs() < 1e-9);
            assert!((m.shunt_susceptance_s.abs() - 0.01).abs() < 1e-12);
        }
    }

    #[test]
    fn every_l_network_turns_the_load_into_the_conjugate_of_the_source() {
        let cases = [
            (c(50.0, 0.0), c(100.0, 0.0)),
            (c(50.0, 0.0), c(10.0, 0.0)),
            (c(50.0, 0.0), c(20.0, 35.0)),
            (c(50.0, 0.0), c(200.0, -80.0)),
            (c(25.0, 10.0), c(180.0, -60.0)),
            (c(75.0, -20.0), c(12.0, 8.0)),
            (c(300.0, 100.0), c(50.0, 0.0)),
        ];
        for (source, load) in cases {
            let found = l_match(source, load, 433e6, &settings()).unwrap();
            assert!(!found.is_empty());
            for m in found {
                let zin = m.input_impedance(load).unwrap();
                assert!(
                    (zin - source.conj()).abs() < 1e-6 * source.abs(),
                    "{source:?} {load:?} {m:?} {zin:?}"
                );
            }
        }
    }

    #[test]
    fn equal_resistances_need_no_network() {
        let found = l_match(Complex::real(50.0), Complex::real(50.0), 1e9, &settings()).unwrap();
        for m in found {
            assert!(m.series_reactance_ohm.abs() < 1e-6 && m.shunt_susceptance_s.abs() < 1e-9);
        }
    }

    #[test]
    fn l_match_errors() {
        let s = settings();
        assert!(matches!(
            l_match(Complex::real(50.0), Complex::real(-50.0), 1e9, &s),
            Err(RfError::InvalidNumber { .. })
        ));
        assert!(l_match(Complex::real(50.0), Complex::real(100.0), 0.0, &s).is_err());
        assert!(l_match(Complex::real(50.0), Complex::real(100.0), 1e13, &s).is_err());
        assert!(l_match(c(f64::NAN, 0.0), Complex::real(100.0), 1e9, &s).is_err());
        assert!(l_match(Complex::real(0.0), Complex::real(100.0), 1e9, &s).is_err());
    }

    #[test]
    fn single_stub_matches_leave_the_line_matched() {
        let z0 = 50.0;
        let loads = [
            c(100.0, 0.0),
            c(25.0, 0.0),
            c(60.0, 40.0),
            c(30.0, -45.0),
            c(10.0, 5.0),
            c(200.0, 150.0),
            c(50.0, 30.0),
        ];
        for load in loads {
            for kind in [StubKind::Shunt, StubKind::Series] {
                for termination in [Termination::Open, Termination::Short] {
                    let found = stub_match(load, z0, kind, termination, &settings()).unwrap();
                    assert!(!found.is_empty());
                    for m in found {
                        assert!(m.distance_deg >= 0.0 && m.distance_deg < 180.0);
                        assert!(m.stub_deg >= 0.0 && m.stub_deg <= 180.0);
                        let zin = m.input_impedance(load, z0).unwrap();
                        assert!(
                            (zin - Complex::real(z0)).abs() < 1e-6 * z0,
                            "{load:?} {m:?} {zin:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_book_shunt_stub_for_a_real_load() {
        // ZL = 100 ohm on 50 ohm: tan(theta) = +-sqrt(zl) / ... = +-sqrt(2) (Pozar, section 5.2)
        let found = stub_match(
            Complex::real(100.0),
            50.0,
            StubKind::Shunt,
            Termination::Short,
            &settings(),
        )
        .unwrap();
        assert_eq!(found.len(), 2);
        let want = 2.0f64.sqrt().atan().to_degrees();
        assert!(
            (found[0].distance_deg - want).abs() < 1e-9,
            "{}",
            found[0].distance_deg
        );
        assert!((found[1].distance_deg - (180.0 - want)).abs() < 1e-9);
        // the load already matched needs nothing
        let matched = stub_match(
            Complex::real(50.0),
            50.0,
            StubKind::Shunt,
            Termination::Open,
            &settings(),
        )
        .unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!((matched[0].distance_deg, matched[0].stub_deg), (0.0, 0.0));
        assert!((matched[0].distance_wavelengths()).abs() < 1e-15);
    }

    #[test]
    fn stub_lengths_are_given_in_wavelengths_too() {
        let m = StubMatch {
            kind: StubKind::Shunt,
            termination: Termination::Open,
            distance_deg: 90.0,
            stub_deg: 45.0,
        };
        assert_eq!(m.distance_wavelengths(), 0.25);
        assert_eq!(m.stub_wavelengths(), 0.125);
    }

    #[test]
    fn stub_errors() {
        let s = settings();
        assert!(stub_match(c(0.0, 50.0), 50.0, StubKind::Shunt, Termination::Open, &s).is_err());
        assert!(stub_match(
            Complex::real(100.0),
            -50.0,
            StubKind::Shunt,
            Termination::Open,
            &s
        )
        .is_err());
        assert!(stub_match(
            c(f64::INFINITY, 0.0),
            50.0,
            StubKind::Series,
            Termination::Open,
            &s
        )
        .is_err());
    }

    #[test]
    fn the_quarter_wave_transformer_has_the_geometric_mean_impedance() {
        let s = settings();
        assert!((quarter_wave_impedance(50.0, 200.0, &s).unwrap() - 100.0).abs() < 1e-12);
        assert!((quarter_wave_impedance(50.0, 100.0, &s).unwrap() - 5000.0f64.sqrt()).abs() < 1e-9);
        assert!(quarter_wave_impedance(0.0, 100.0, &s).is_err());
        assert!(quarter_wave_impedance(50.0, f64::NAN, &s).is_err());
        let found = quarter_wave(Complex::real(200.0), 50.0, &s).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].distance_deg, 0.0);
        assert!((found[0].section_ohm - 100.0).abs() < 1e-12);
        assert_eq!(QuarterWave::SECTION_DEG, 90.0);
        assert!(
            found[0]
                .reflection(Complex::real(200.0), 50.0)
                .unwrap()
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn a_complex_load_is_brought_to_a_real_value_first() {
        let s = settings();
        for load in [c(30.0, 40.0), c(120.0, -60.0), c(10.0, 25.0), c(75.0, 1.0)] {
            let found = quarter_wave(load, 50.0, &s).unwrap();
            assert_eq!(found.len(), 2);
            // the two places are a quarter wave apart
            assert!((found[1].distance_deg - found[0].distance_deg - 90.0).abs() < 1e-6);
            for m in found {
                assert!(m.real_impedance_ohm > 0.0);
                assert!((m.section_ohm - (50.0 * m.real_impedance_ohm).sqrt()).abs() < 1e-9);
                let gamma = m.reflection(load, 50.0).unwrap();
                assert!(gamma.abs() < 1e-6, "{load:?} {m:?} {gamma:?}");
            }
        }
    }

    #[test]
    fn quarter_wave_errors() {
        let s = settings();
        assert!(quarter_wave(Complex::real(-5.0), 50.0, &s).is_err());
        assert!(quarter_wave(Complex::real(50.0), 0.0, &s).is_err());
    }
}
