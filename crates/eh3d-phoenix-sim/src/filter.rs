// SPDX-License-Identifier: GPL-3.0-or-later
//! The coherence check against the RF filters. A ladder filter of `eh3d-phoenix-rf` is turned
//! into a netlist (a source with the source resistance, the series and shunt branches, the load),
//! simulated in AC, and the magnitude, the phase and the group delay of the output are compared
//! with the response that the RF crate computes from ABCD matrices. The two use unrelated
//! methods, so a difference means that one of them is wrong.

use eh3d_phoenix_rf::filter::{Band, Branch, Filter, Position};
use eh3d_phoenix_rf::{Complex, RfSettings};

use crate::ac::ac_at;
use crate::circuit::Circuit;
use crate::codes::{SimError, ANALYSIS_INVALID, COHERENCE_FAILED};
use crate::netlist::{Element, Kind, Netlist, Source, Wave};
use crate::settings::SimSettings;

fn element(name: String, nodes: [&str; 2], kind: Kind) -> Element {
    Element {
        name,
        nodes: nodes.iter().map(|n| (*n).to_string()).collect(),
        kind,
    }
}

fn add_branch(out: &mut Vec<Element>, tag: usize, a: &str, b: &str, branch: &Branch) {
    let inductor = |name: String, x: &str, y: &str, henries: f64| {
        element(
            name,
            [x, y],
            Kind::Inductor {
                value: henries,
                ic: None,
            },
        )
    };
    let capacitor = |name: String, x: &str, y: &str, farads: f64| {
        element(
            name,
            [x, y],
            Kind::Capacitor {
                value: farads,
                ic: None,
            },
        )
    };
    match *branch {
        Branch::Inductor(l) => out.push(inductor(format!("l{tag}"), a, b, l)),
        Branch::Capacitor(c) => out.push(capacitor(format!("c{tag}"), a, b, c)),
        Branch::SeriesLc {
            inductance_h,
            capacitance_f,
        } => {
            let middle = format!("m{tag}");
            out.push(inductor(format!("l{tag}"), a, &middle, inductance_h));
            out.push(capacitor(format!("c{tag}"), &middle, b, capacitance_f));
        }
        Branch::ParallelLc {
            inductance_h,
            capacitance_f,
        } => {
            out.push(inductor(format!("l{tag}"), a, b, inductance_h));
            out.push(capacitor(format!("c{tag}"), a, b, capacitance_f));
        }
    }
}

/// The netlist of a ladder filter driven by a 1 V AC source, and the label of its output.
pub fn netlist_of_filter(f: &Filter) -> (Netlist, String) {
    let mut elements = vec![
        element(
            "v1".to_string(),
            ["src", "0"],
            Kind::Voltage(Source {
                dc: None,
                ac: Some((1.0, 0.0)),
                wave: Wave::None,
            }),
        ),
        element("rs".to_string(), ["src", "n0"], Kind::Resistor(f.z0_ohm)),
    ];
    let mut current = "n0".to_string();
    let mut series = 0;
    for (i, section) in f.sections.iter().enumerate() {
        match section.position {
            Position::Series => {
                series += 1;
                let next = format!("n{series}");
                add_branch(&mut elements, i, &current, &next, &section.branch);
                current = next;
            }
            Position::Shunt => {
                add_branch(&mut elements, i, &current, "0", &section.branch);
            }
        }
    }
    elements.push(element(
        "rl".to_string(),
        [current.as_str(), "0"],
        Kind::Resistor(f.load_ohm),
    ));
    let label = format!("v({current})");
    (
        Netlist {
            title: "ladder filter".to_string(),
            elements,
            ..Netlist::default()
        },
        label,
    )
}

/// The frequencies of the check: `coherence_points` in a geometric progression from a tenth of
/// the lowest edge of the filter to ten times the highest.
pub fn check_frequencies(f: &Filter, s: &SimSettings) -> Vec<f64> {
    let (low, high) = match f.band {
        Band::LowPass { cutoff_hz } | Band::HighPass { cutoff_hz } => {
            (cutoff_hz / 10.0, cutoff_hz * 10.0)
        }
        Band::BandPass { low_hz, high_hz } | Band::BandStop { low_hz, high_hz } => {
            (low_hz / 10.0, high_hz * 10.0)
        }
    };
    let last = (s.coherence_points - 1) as f64;
    (0..s.coherence_points)
        .map(|i| low * (high / low).powf(i as f64 / last))
        .collect()
}

/// How far the simulation is from the RF crate.
#[derive(Debug, Clone, PartialEq)]
pub struct Coherence {
    /// The frequencies compared.
    pub points: usize,
    /// The largest difference of magnitude in dB.
    pub max_db_error: f64,
    /// The largest difference of phase in degrees.
    pub max_phase_error_deg: f64,
    /// The largest difference of group delay, as a fraction of one period.
    pub max_delay_error_periods: f64,
    /// The frequency where the difference of magnitude is the largest.
    pub worst_freq_hz: f64,
}

impl Coherence {
    /// True when every difference is within the tolerances of the settings.
    pub fn is_within(&self, s: &SimSettings) -> bool {
        self.max_db_error <= s.coherence_db_tolerance
            && self.max_phase_error_deg <= s.coherence_phase_tolerance_deg
            && self.max_delay_error_periods <= s.coherence_delay_tolerance
    }

    /// `Ok` when the differences are within the tolerances, else the error with the figures.
    pub fn verify(&self, s: &SimSettings) -> Result<(), SimError> {
        if self.is_within(s) {
            return Ok(());
        }
        Err(SimError::new(
            COHERENCE_FAILED,
            format!(
                "{} points: magnitude {:e} dB (limit {:e}), phase {:e} deg (limit {:e}), group delay {:e} periods (limit {:e}); worst at {:e} Hz",
                self.points,
                self.max_db_error,
                s.coherence_db_tolerance,
                self.max_phase_error_deg,
                s.coherence_phase_tolerance_deg,
                self.max_delay_error_periods,
                s.coherence_delay_tolerance,
                self.worst_freq_hz
            ),
        ))
    }

    /// A short text of the figures.
    pub fn to_text(&self) -> String {
        format!(
            "coherence with the RF filter over {} frequencies: magnitude {:e} dB, phase {:e} deg, group delay {:e} periods (worst magnitude at {:e} Hz)\n",
            self.points,
            self.max_db_error,
            self.max_phase_error_deg,
            self.max_delay_error_periods,
            self.worst_freq_hz
        )
    }
}

fn rf_error(e: impl std::fmt::Display) -> SimError {
    SimError::new(ANALYSIS_INVALID, e.to_string())
}

/// Simulates the filter at the frequencies and compares it with the RF crate.
pub fn compare_filter(f: &Filter, freqs: &[f64], s: &SimSettings) -> Result<Coherence, SimError> {
    let (netlist, label) = netlist_of_filter(f);
    let circuit = Circuit::compile(&netlist, s)?;
    let ac = ac_at(&circuit, s, freqs)?;
    let (Some(output), Some(delays)) = (ac.series(&label), ac.delay(&label)) else {
        return Err(SimError::new(
            ANALYSIS_INVALID,
            format!("the output {label} is not in the circuit"),
        ));
    };
    let rf = RfSettings {
        delay_step_fraction: s.delay_step_fraction,
        ..RfSettings::default()
    };
    // the transmission of the RF crate is relative to the power available from the source
    let ratio = (f.load_ohm / f.z0_ohm).sqrt() / 2.0;
    let mut result = Coherence {
        points: freqs.len(),
        max_db_error: 0.0,
        max_phase_error_deg: 0.0,
        max_delay_error_periods: 0.0,
        worst_freq_hz: freqs.first().copied().unwrap_or(0.0),
    };
    for (k, &freq) in freqs.iter().enumerate() {
        let expected: Complex = f.response_at(freq).map_err(rf_error)?.s21.scale(ratio);
        let got = output[k];
        let db_error = (got.db20() - expected.db20()).abs();
        let phase_error = (got * expected.conj()).arg_deg().abs();
        let expected_delay = f.group_delay_s(freq, &rf).map_err(rf_error)?;
        let delay_error = (delays[k] - expected_delay).abs() * freq;
        if db_error > result.max_db_error {
            result.max_db_error = db_error;
            result.worst_freq_hz = freq;
        }
        result.max_phase_error_deg = result.max_phase_error_deg.max(phase_error);
        result.max_delay_error_periods = result.max_delay_error_periods.max(delay_error);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rf::filter::{design, prototype, Approximation};

    fn built(approx: Approximation, order: usize, ripple: Option<f64>, band: Band) -> Filter {
        let rf = RfSettings::default();
        let p = prototype(approx, order, ripple, Position::Shunt, &rf).unwrap();
        design(&p, band, 50.0, &rf).unwrap()
    }

    #[test]
    fn the_netlist_of_a_third_order_low_pass_has_the_ladder_of_the_design() {
        let f = built(
            Approximation::Butterworth,
            3,
            None,
            Band::LowPass { cutoff_hz: 1e6 },
        );
        let (nl, label) = netlist_of_filter(&f);
        // source, source resistance, three branches, load
        assert_eq!(nl.elements.len(), 6);
        assert_eq!(
            label,
            format!(
                "v(n{})",
                f.sections
                    .iter()
                    .filter(|s| s.position == Position::Series)
                    .count()
            )
        );
        let c = Circuit::compile(&nl, &SimSettings::default()).unwrap();
        assert_eq!(c.inductors.len() + c.capacitors.len(), 3);
    }

    #[test]
    fn the_simulation_agrees_with_the_rf_crate_for_every_band_and_family() {
        let s = SimSettings::default();
        let cases = [
            built(
                Approximation::Butterworth,
                3,
                None,
                Band::LowPass { cutoff_hz: 1e6 },
            ),
            built(
                Approximation::Chebyshev,
                4,
                Some(0.5),
                Band::LowPass { cutoff_hz: 10e6 },
            ),
            built(
                Approximation::Bessel,
                5,
                None,
                Band::HighPass { cutoff_hz: 2e6 },
            ),
            built(
                Approximation::Butterworth,
                3,
                None,
                Band::BandPass {
                    low_hz: 9e6,
                    high_hz: 11e6,
                },
            ),
            built(
                Approximation::Chebyshev,
                3,
                Some(0.1),
                Band::BandStop {
                    low_hz: 40e6,
                    high_hz: 60e6,
                },
            ),
        ];
        for (n, f) in cases.iter().enumerate() {
            let freqs = check_frequencies(f, &s);
            assert_eq!(freqs.len(), s.coherence_points);
            let c = compare_filter(f, &freqs, &s).unwrap();
            assert!(c.is_within(&s), "case {n}: {}", c.to_text());
            c.verify(&s).unwrap();
        }
    }

    #[test]
    fn a_tolerance_that_cannot_be_met_is_reported_with_the_figures() {
        let f = built(
            Approximation::Butterworth,
            3,
            None,
            Band::LowPass { cutoff_hz: 1e6 },
        );
        let strict = SimSettings {
            coherence_db_tolerance: 1e-300,
            ..SimSettings::default()
        };
        let freqs = check_frequencies(&f, &strict);
        let c = compare_filter(&f, &freqs, &strict).unwrap();
        let e = c.verify(&strict).unwrap_err();
        assert_eq!(e.code, COHERENCE_FAILED);
        assert!(e.detail.contains("41 points"), "{}", e.detail);
    }

    #[test]
    fn the_response_has_the_textbook_corner() {
        // a third order Butterworth low pass is 3 dB down at its cutoff
        let s = SimSettings::default();
        let f = built(
            Approximation::Butterworth,
            3,
            None,
            Band::LowPass { cutoff_hz: 1e6 },
        );
        let (nl, label) = netlist_of_filter(&f);
        let c = Circuit::compile(&nl, &s).unwrap();
        let ac = ac_at(&c, &s, &[1e3, 1e6]).unwrap();
        let h = ac.series(&label).unwrap();
        // the load is the same as the source, so the gain at DC is 1/2
        assert!((h[0].abs() - 0.5).abs() < 1e-6);
        assert!((h[1].db20() - h[0].db20() + 3.010_299_956_6).abs() < 1e-4);
    }
}
