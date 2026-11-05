// SPDX-License-Identifier: GPL-3.0-or-later
//! The text reports of the RF commands. The labels are in English; the errors that stop a
//! calculation have their titles in the eight languages (see [`crate::error`]).

use crate::complex::Complex;
use crate::error::RfError;
use crate::filter::{Band, Branch, Filter, FilterPoint, Position};
use crate::matching::{
    ElementKind, LKind, LMatch, QuarterWave, StubKind, StubMatch, Termination, Topology,
};
use crate::network::{
    insertion_loss_db, is_passive, is_reciprocal, max_available_gain, max_stable_gain,
    return_loss_db, stability, vswr, Mat2,
};
use crate::settings::RfSettings;
use crate::smith::SmithPoint;
use crate::touchstone::{Format, FreqUnit, Parameter, Touchstone};
use std::fmt::Write;

/// A value with an SI prefix: `1.5e-9, "H"` is `1.5000 nH`. Zero and values that are not finite
/// are written plainly.
pub fn engineering(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return format!("{value} {unit}");
    }
    if value == 0.0 {
        return format!("0 {unit}");
    }
    const PREFIXES: [(f64, &str); 9] = [
        (1e12, "T"),
        (1e9, "G"),
        (1e6, "M"),
        (1e3, "k"),
        (1.0, ""),
        (1e-3, "m"),
        (1e-6, "u"),
        (1e-9, "n"),
        (1e-12, "p"),
    ];
    let magnitude = value.abs();
    let (scale, prefix) = PREFIXES
        .iter()
        .copied()
        .find(|(scale, _)| magnitude >= *scale * 0.9995)
        .unwrap_or((1e-12, "p"));
    format!("{:.4} {prefix}{unit}", value / scale)
}

fn complex_text(z: Complex) -> String {
    if z.im < 0.0 {
        format!("{:.4} - j{:.4}", z.re, -z.im)
    } else {
        format!("{:.4} + j{:.4}", z.re, z.im)
    }
}

fn polar_text(z: Complex) -> String {
    format!("{:.4} dB  {:.2} deg", z.db20(), z.arg_deg())
}

fn db_text(db: f64) -> String {
    let db = if db == 0.0 { 0.0 } else { db };
    if db.is_infinite() {
        "infinite".to_string()
    } else {
        format!("{db:.3} dB")
    }
}

fn ratio_text(value: f64) -> String {
    if value.is_infinite() {
        "infinite".to_string()
    } else {
        format!("{value:.4}")
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "yes"
    } else {
        "no"
    }
}

/// The report of `phoenix rf touchstone`: the header of the file and, when a frequency in GHz
/// is given, the parameters and the figures at that frequency.
pub fn touchstone_report(
    file: &Touchstone,
    freq_ghz: Option<f64>,
    settings: &RfSettings,
) -> Result<String, RfError> {
    let mut out = String::new();
    let parameter = match file.parameter {
        Parameter::S => "S",
        Parameter::Y => "Y",
        Parameter::Z => "Z",
        Parameter::H => "H",
        Parameter::G => "G",
    };
    let format = match file.format {
        Format::Ri => "RI",
        Format::Ma => "MA",
        Format::Db => "DB",
    };
    let unit = match file.unit {
        FreqUnit::Hz => "Hz",
        FreqUnit::KHz => "kHz",
        FreqUnit::MHz => "MHz",
        FreqUnit::GHz => "GHz",
    };
    let _ = writeln!(out, "ports: {}", file.ports);
    let _ = writeln!(
        out,
        "parameters: {parameter} ({format}), frequencies in {unit}, reference {} ohm",
        file.z_ref_ohm
    );
    if let Some((low, high)) = file.frequency_range_hz() {
        let _ = writeln!(
            out,
            "points: {}, from {} to {}",
            file.points.len(),
            engineering(low, "Hz"),
            engineering(high, "Hz")
        );
    }
    let _ = writeln!(out, "noise points: {}", file.noise.len());
    if file.ports == 2 && file.parameter != Parameter::H && file.parameter != Parameter::G {
        let mut passive = true;
        let mut reciprocal = true;
        for index in 0..file.points.len() {
            let s = Mat2::from_cmatrix(&file.s_matrix(index)?)?;
            passive &= is_passive(&s, settings.passivity_tolerance);
            reciprocal &= is_reciprocal(&s, settings.passivity_tolerance);
        }
        let _ = writeln!(
            out,
            "passive at every point: {}, reciprocal at every point: {}",
            yes_no(passive),
            yes_no(reciprocal)
        );
    }
    let Some(ghz) = freq_ghz else {
        return Ok(out);
    };
    let freq_hz = settings.check_frequency_hz("frequency", ghz * 1e9)?;
    let s = file.s_matrix_at(freq_hz)?;
    let _ = writeln!(out, "at {}:", engineering(freq_hz, "Hz"));
    for row in 0..file.ports {
        for col in 0..file.ports {
            let _ = writeln!(
                out,
                "  S{}{}: {}  ({})",
                row + 1,
                col + 1,
                polar_text(s.get(row, col)),
                complex_text(s.get(row, col))
            );
        }
    }
    let z0 = file.z_ref_ohm;
    match file.ports {
        1 => {
            let g = s.get(0, 0);
            let point = SmithPoint::from_gamma(g, Some(freq_hz))?;
            let _ = writeln!(out, "  return loss: {}", db_text(return_loss_db(g)));
            let _ = writeln!(out, "  VSWR: {}", ratio_text(vswr(g)));
            match point.z {
                Some(z) => {
                    let _ = writeln!(
                        out,
                        "  impedance: {} ohm (normalised {})",
                        complex_text(z.scale(z0)),
                        complex_text(z)
                    );
                }
                None => {
                    let _ = writeln!(out, "  impedance: open circuit");
                }
            }
            if let Some(y) = point.y {
                let _ = writeln!(out, "  admittance (normalised): {}", complex_text(y));
            }
        }
        2 => {
            let m = Mat2::from_cmatrix(&s)?;
            let _ = writeln!(
                out,
                "  input return loss: {}, VSWR {}",
                db_text(return_loss_db(m.m11)),
                ratio_text(vswr(m.m11))
            );
            let _ = writeln!(
                out,
                "  output return loss: {}, VSWR {}",
                db_text(return_loss_db(m.m22)),
                ratio_text(vswr(m.m22))
            );
            let _ = writeln!(
                out,
                "  insertion loss: {}",
                db_text(insertion_loss_db(m.m21))
            );
            let _ = writeln!(out, "  gain: {:.3} dB", m.m21.db20());
            match file.group_delay_s(freq_hz) {
                Ok(delay) => {
                    let _ = writeln!(out, "  group delay: {}", engineering(delay, "s"));
                }
                Err(_) => {
                    let _ = writeln!(out, "  group delay: not available");
                }
            }
            let st = stability(&m);
            let _ = writeln!(
                out,
                "  stability: K {}, |delta| {:.4}, mu {}, mu' {}, unconditionally stable: {}",
                ratio_text(st.k),
                st.delta.abs(),
                ratio_text(st.mu),
                ratio_text(st.mu_prime),
                yes_no(st.unconditionally_stable)
            );
            let _ = writeln!(
                out,
                "  maximum stable gain: {:.3} dB",
                20.0 * max_stable_gain(&m).log10()
            );
            match max_available_gain(&m) {
                Some(g) => {
                    let _ = writeln!(out, "  maximum available gain: {:.3} dB", 10.0 * g.log10());
                }
                None => {
                    let _ = writeln!(out, "  maximum available gain: not defined (K <= 1)");
                }
            }
            let _ = writeln!(
                out,
                "  passive: {}, reciprocal: {}",
                yes_no(is_passive(&m, settings.passivity_tolerance)),
                yes_no(is_reciprocal(&m, settings.passivity_tolerance))
            );
        }
        _ => {}
    }
    Ok(out)
}

fn element_text(kind: ElementKind, value: f64) -> String {
    match kind {
        ElementKind::Inductor => format!("inductor {}", engineering(value, "H")),
        ElementKind::Capacitor => format!("capacitor {}", engineering(value, "F")),
    }
}

/// The report of `phoenix rf match l`.
pub fn l_match_report(source: Complex, load: Complex, freq_hz: f64, matches: &[LMatch]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "L network from source {} ohm to load {} ohm at {}",
        complex_text(source),
        complex_text(load),
        engineering(freq_hz, "Hz")
    );
    for (index, m) in matches.iter().enumerate() {
        let place = match m.topology {
            Topology::SeriesAtSource => "series element at the source, shunt element at the load",
            Topology::SeriesAtLoad => "shunt element at the source, series element at the load",
        };
        let kind = match m.kind {
            LKind::LowPass => "low-pass",
            LKind::HighPass => "high-pass",
            LKind::Mixed => "mixed",
        };
        let _ = writeln!(out, "solution {} ({kind}): {place}", index + 1);
        let _ = writeln!(
            out,
            "  series: {} (reactance {:.4} ohm)",
            element_text(m.series.kind, m.series.value),
            m.series_reactance_ohm
        );
        let _ = writeln!(
            out,
            "  shunt: {} (susceptance {:.6} S)",
            element_text(m.shunt.kind, m.shunt.value),
            m.shunt_susceptance_s
        );
        if let Ok(zin) = m.input_impedance(load) {
            let _ = writeln!(
                out,
                "  check: the source sees {} ohm (conjugate of the source: {})",
                complex_text(zin),
                complex_text(source.conj())
            );
        }
    }
    out
}

/// The report of `phoenix rf match stub`. `eps_eff` gives the physical lengths at `freq_hz`.
pub fn stub_report(
    load: Complex,
    z0_ohm: f64,
    freq_hz: f64,
    eps_eff: f64,
    matches: &[StubMatch],
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "single stub on a {z0_ohm} ohm line, load {} ohm at {}",
        complex_text(load),
        engineering(freq_hz, "Hz")
    );
    let line = crate::line::Line::new(z0_ohm, eps_eff).ok();
    for (index, m) in matches.iter().enumerate() {
        let kind = match m.kind {
            StubKind::Shunt => "shunt",
            StubKind::Series => "series",
        };
        let end = match m.termination {
            Termination::Open => "open",
            Termination::Short => "short-circuited",
        };
        let _ = writeln!(out, "solution {} ({kind} stub, {end}):", index + 1);
        let physical = |degrees: f64| -> String {
            match line.and_then(|l| l.length_m(degrees, freq_hz).ok()) {
                Some(metres) => format!(", {}", engineering(metres, "m")),
                None => String::new(),
            }
        };
        let _ = writeln!(
            out,
            "  distance from the load: {:.3} deg ({:.4} wavelengths{})",
            m.distance_deg,
            m.distance_wavelengths(),
            physical(m.distance_deg)
        );
        let _ = writeln!(
            out,
            "  stub length: {:.3} deg ({:.4} wavelengths{})",
            m.stub_deg,
            m.stub_wavelengths(),
            physical(m.stub_deg)
        );
        if let Ok(zin) = m.input_impedance(load, z0_ohm) {
            let _ = writeln!(out, "  check: the source sees {} ohm", complex_text(zin));
        }
    }
    out
}

/// The report of `phoenix rf match qw`.
pub fn quarter_wave_report(
    load: Complex,
    z0_ohm: f64,
    freq_hz: f64,
    eps_eff: f64,
    matches: &[QuarterWave],
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "quarter-wave transformer on a {z0_ohm} ohm line, load {} ohm at {}",
        complex_text(load),
        engineering(freq_hz, "Hz")
    );
    let line = crate::line::Line::new(z0_ohm, eps_eff).ok();
    for (index, m) in matches.iter().enumerate() {
        let _ = writeln!(out, "solution {}:", index + 1);
        if m.distance_deg > 0.0 {
            let physical = match line.and_then(|l| l.length_m(m.distance_deg, freq_hz).ok()) {
                Some(metres) => format!(", {}", engineering(metres, "m")),
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "  line from the load: {:.3} deg ({:.4} wavelengths{physical}), then a real impedance of {:.4} ohm",
                m.distance_deg,
                m.distance_deg / 360.0,
                m.real_impedance_ohm
            );
        } else {
            let _ = writeln!(out, "  the load is real: {:.4} ohm", m.real_impedance_ohm);
        }
        let section = line
            .and_then(|_| crate::line::Line::new(m.section_ohm, eps_eff).ok())
            .and_then(|l| l.length_m(QuarterWave::SECTION_DEG, freq_hz).ok())
            .map(|metres| format!(", {}", engineering(metres, "m")))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "  transformer: {:.4} ohm, {} deg{section}",
            m.section_ohm,
            QuarterWave::SECTION_DEG
        );
        if let Ok(g) = m.reflection(load, z0_ohm) {
            let _ = writeln!(out, "  check: reflection at the source {:.2e}", g.abs());
        }
    }
    out
}

fn branch_text(branch: Branch) -> String {
    match branch {
        Branch::Inductor(l) => format!("inductor {}", engineering(l, "H")),
        Branch::Capacitor(c) => format!("capacitor {}", engineering(c, "F")),
        Branch::SeriesLc {
            inductance_h,
            capacitance_f,
        } => format!(
            "inductor {} in series with capacitor {}",
            engineering(inductance_h, "H"),
            engineering(capacitance_f, "F")
        ),
        Branch::ParallelLc {
            inductance_h,
            capacitance_f,
        } => format!(
            "inductor {} in parallel with capacitor {}",
            engineering(inductance_h, "H"),
            engineering(capacitance_f, "F")
        ),
    }
}

/// The report of `phoenix rf filter`: the prototype values, the components and, when a sweep is
/// given, the insertion and return loss at its points.
pub fn filter_report(filter: &Filter, sweep: Option<&[FilterPoint]>) -> String {
    let mut out = String::new();
    let p = &filter.prototype;
    let family = match p.approximation {
        crate::filter::Approximation::Butterworth => "Butterworth".to_string(),
        crate::filter::Approximation::Chebyshev => {
            format!("Chebyshev ({} dB ripple)", p.ripple_db.unwrap_or(0.0))
        }
        crate::filter::Approximation::Bessel => "Bessel".to_string(),
    };
    let band = match filter.band {
        Band::LowPass { cutoff_hz } => format!("low-pass, cutoff {}", engineering(cutoff_hz, "Hz")),
        Band::HighPass { cutoff_hz } => {
            format!("high-pass, cutoff {}", engineering(cutoff_hz, "Hz"))
        }
        Band::BandPass { low_hz, high_hz } => format!(
            "band-pass, {} to {}",
            engineering(low_hz, "Hz"),
            engineering(high_hz, "Hz")
        ),
        Band::BandStop { low_hz, high_hz } => format!(
            "band-stop, {} to {}",
            engineering(low_hz, "Hz"),
            engineering(high_hz, "Hz")
        ),
    };
    let _ = writeln!(out, "{family} filter of order {}, {band}", p.order);
    let _ = writeln!(
        out,
        "source {} ohm, load {} ohm",
        filter.z0_ohm,
        engineering(filter.load_ohm, "ohm")
    );
    let values: Vec<String> = p.g.iter().map(|g| format!("{g:.4}")).collect();
    let _ = writeln!(
        out,
        "prototype g: {} (load {:.4})",
        values.join(" "),
        p.g_load
    );
    for (index, section) in filter.sections.iter().enumerate() {
        let place = match section.position {
            Position::Series => "series",
            Position::Shunt => "shunt",
        };
        let _ = writeln!(
            out,
            "  {} {place}: {}",
            index + 1,
            branch_text(section.branch)
        );
    }
    if let Some(points) = sweep {
        let _ = writeln!(out, "frequency, insertion loss, return loss");
        for point in points {
            let _ = writeln!(
                out,
                "  {}  {}  {}",
                engineering(point.freq_hz, "Hz"),
                db_text(point.insertion_loss_db()),
                db_text(point.return_loss_db())
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{design, prototype, Approximation};
    use crate::matching::{l_match, quarter_wave, stub_match};

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    #[test]
    fn values_are_written_with_a_prefix() {
        assert_eq!(engineering(1.5e-9, "H"), "1.5000 nH");
        assert_eq!(engineering(15.9e-12, "F"), "15.9000 pF");
        assert_eq!(engineering(2.4e9, "Hz"), "2.4000 GHz");
        assert_eq!(engineering(-50.0, "ohm"), "-50.0000 ohm");
        assert_eq!(engineering(0.0, "Hz"), "0 Hz");
        assert_eq!(engineering(999.99e6, "Hz"), "1.0000 GHz");
        assert!(engineering(f64::INFINITY, "Hz").contains("inf"));
        assert_eq!(engineering(3e-15, "F"), "0.0030 pF");
    }

    #[test]
    fn the_touchstone_report_has_the_header_and_the_figures() {
        let text = "# MHz S RI R 50
100 0.1 0 0.8 0 0.8 0 0.1 0
200 0.2 0 0.7 0 0.7 0 0.2 0
";
        let file = Touchstone::parse(text, 2, &settings()).unwrap();
        let header = touchstone_report(&file, None, &settings()).unwrap();
        assert!(header.contains("ports: 2") && header.contains("points: 2"));
        assert!(header.contains("passive at every point: yes, reciprocal at every point: yes"));
        let at = touchstone_report(&file, Some(0.15), &settings()).unwrap();
        assert!(at.contains("S21:") && at.contains("input return loss: 16.47"));
        assert!(at.contains("insertion loss:") && at.contains("group delay:"));
        assert!(at.contains("unconditionally stable: yes"));
        assert!(touchstone_report(&file, Some(1.0), &settings()).is_err());
        assert!(touchstone_report(&file, Some(0.0), &settings()).is_err());
    }

    #[test]
    fn the_one_port_report_shows_the_impedance() {
        let file =
            Touchstone::parse("# GHz S RI R 50\n1 0.3333333333 0\n", 1, &settings()).unwrap();
        let at = touchstone_report(&file, Some(1.0), &settings()).unwrap();
        assert!(at.contains("VSWR: 2.0000") && at.contains("impedance: 100.0000"));
        let open = Touchstone::parse("# GHz S RI R 50\n1 1 0\n", 1, &settings()).unwrap();
        let at = touchstone_report(&open, Some(1.0), &settings()).unwrap();
        assert!(at.contains("open circuit") && at.contains("VSWR: infinite"));
    }

    #[test]
    fn the_three_port_report_lists_the_matrix() {
        let mut text = String::from("# GHz S RI R 50\n1");
        for k in 0..9 {
            text.push_str(&format!(" {} 0", 0.1 * f64::from(k)));
        }
        text.push('\n');
        let file = Touchstone::parse(&text, 3, &settings()).unwrap();
        let at = touchstone_report(&file, Some(1.0), &settings()).unwrap();
        assert!(at.contains("S11:") && at.contains("S33:"));
    }

    #[test]
    fn the_matching_reports_have_the_components_and_the_check() {
        let s = settings();
        let found = l_match(Complex::real(50.0), Complex::real(100.0), 100e6, &s).unwrap();
        let text = l_match_report(Complex::real(50.0), Complex::real(100.0), 100e6, &found);
        assert!(text.contains("low-pass") && text.contains("79.5775 nH"));
        assert!(text.contains("capacitor 15.9155 pF") && text.contains("check:"));
        let stubs = stub_match(
            Complex::real(100.0),
            50.0,
            StubKind::Shunt,
            Termination::Short,
            &s,
        )
        .unwrap();
        let text = stub_report(Complex::real(100.0), 50.0, 1e9, 1.0, &stubs);
        assert!(text.contains("solution 2") && text.contains("wavelengths, "));
        assert!(text.contains("check: the source sees 50.0000"));
        let quarter = quarter_wave(Complex::new(30.0, 40.0), 50.0, &s).unwrap();
        let text = quarter_wave_report(Complex::new(30.0, 40.0), 50.0, 1e9, 2.0, &quarter);
        assert!(text.contains("transformer:") && text.contains("then a real impedance"));
        let real = quarter_wave(Complex::real(200.0), 50.0, &s).unwrap();
        let text = quarter_wave_report(Complex::real(200.0), 50.0, 1e9, 1.0, &real);
        assert!(text.contains("the load is real") && text.contains("100.0000 ohm"));
    }

    #[test]
    fn the_filter_report_lists_the_sections_and_the_sweep() {
        let s = settings();
        let p = prototype(Approximation::Butterworth, 3, None, Position::Shunt, &s).unwrap();
        let f = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &s).unwrap();
        let sweep = f.sweep(10e6, 200e6, 3, false, &s).unwrap();
        let text = filter_report(&f, Some(&sweep));
        assert!(text.contains("Butterworth filter of order 3, low-pass, cutoff 100.0000 MHz"));
        assert!(text.contains("prototype g: 1.0000 2.0000 1.0000"));
        assert!(text.contains("1 shunt: capacitor 31.8310 pF"));
        assert!(text.contains("2 series: inductor 159.1549 nH"));
        assert!(text.contains("frequency, insertion loss, return loss"));
        let bp = design(
            &p,
            Band::BandPass {
                low_hz: 90e6,
                high_hz: 110e6,
            },
            50.0,
            &s,
        )
        .unwrap();
        assert!(filter_report(&bp, None).contains("in parallel with capacitor"));
        let cheb = prototype(Approximation::Chebyshev, 2, Some(0.5), Position::Shunt, &s).unwrap();
        let g = design(
            &cheb,
            Band::BandStop {
                low_hz: 90e6,
                high_hz: 110e6,
            },
            50.0,
            &s,
        )
        .unwrap();
        let text = filter_report(&g, None);
        assert!(
            text.contains("Chebyshev (0.5 dB ripple)") && text.contains("in series with capacitor")
        );
    }
}
