// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced analyses through the public interface of the simulator, against closed-form
//! results: the noise of an RC circuit (kT/C), the poles and the figures of merit of series RLC
//! circuits, the uncertainty of a divider with tolerances and a step on a parameter. The expected
//! values come from the formulas written in each test, evaluated here, not from the code under
//! test.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use eh3d_phoenix_sim::measure::{figures, FIGURE_NAMES};
use eh3d_phoenix_sim::settings::BOLTZMANN;
use eh3d_phoenix_sim::{
    ac_analysis, noise_analysis, parse, parse_with_params, pole_zero, run_study, AcRequest,
    Circuit, McSpec, SimSettings, StudyPlan, SweepKind,
};

fn settings() -> SimSettings {
    SimSettings::default()
}

fn circuit(text: &str) -> Circuit {
    let s = settings();
    Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
}

fn sweep(per_decade: usize, start_hz: f64, stop_hz: f64) -> AcRequest {
    AcRequest {
        kind: SweepKind::Decade,
        points: per_decade,
        start_hz,
        stop_hz,
    }
}

fn figure(values: &[Option<f64>], name: &str) -> Option<f64> {
    let k = FIGURE_NAMES.iter().position(|(n, _)| *n == name).unwrap();
    values[k]
}

#[test]
fn the_total_noise_of_a_resistor_across_a_capacitor_is_root_kt_over_c() {
    let s = settings();
    let c = circuit("ktc\nR1 out 0 1k\nC1 out 0 1u\n");
    let r = noise_analysis(&c, &s, "v(out)", &sweep(100, 1e-3, 1e9)).unwrap();
    let want = (BOLTZMANN * (s.temperature_c + 273.15) / 1e-6).sqrt();
    assert!(
        (r.rms_volts / want - 1.0).abs() < 5e-4,
        "{} {want}",
        r.rms_volts
    );
    // the density at low frequency is sqrt(4 k T R)
    let density = r.voltage_density()[0];
    let flat = (4.0 * BOLTZMANN * (s.temperature_c + 273.15) * 1000.0).sqrt();
    assert!((density / flat - 1.0).abs() < 1e-9, "{density} {flat}");
}

#[test]
fn the_poles_of_a_series_rlc_circuit_have_its_natural_frequency_and_quality() {
    // R = 10 ohm, L = 1 mH, C = 1 uF across the capacitor: w0 = 1/sqrt(L C), Q = sqrt(L/C)/R
    let s = settings();
    let c = circuit("rlc\nV1 in 0 AC 1\nL1 in a 1m\nR1 a out 10\nC1 out 0 1u\n");
    let r = ac_analysis(&c, &s, &sweep(20, 10.0, 1e6)).unwrap();
    let h = r.series("v(out)").unwrap();
    let found = pole_zero(&r.freqs, &h, None, &s).unwrap();
    assert_eq!(found.order, 2);
    assert!(found.zeros.is_empty());
    let w0 = 1.0 / (1e-3f64 * 1e-6).sqrt();
    let q = (1e-3f64 / 1e-6).sqrt() / 10.0;
    for p in &found.poles {
        assert!((p.abs() / w0 - 1.0).abs() < 1e-5, "{p:?}");
        assert!(
            (p.abs() / (2.0 * p.re.abs()) / q - 1.0).abs() < 1e-5,
            "{p:?}"
        );
        assert!(p.re < 0.0);
    }
    assert!((found.dc_gain - 1.0).abs() < 1e-5);
    assert!(found.max_error < 1e-6);
}

#[test]
fn a_zero_of_a_lead_network_is_found() {
    // H = (1 + s R1 C) / (1 + s (R1 || R2) C ...) for a divider with a capacitor across R1:
    // zero at 1/(R1 C), pole at 1/((R1 || R2) C)
    let s = settings();
    let c = circuit("lead\nV1 in 0 AC 1\nR1 in out 9k\nC1 in out 100n\nR2 out 0 1k\n");
    let r = ac_analysis(&c, &s, &sweep(20, 1.0, 1e6)).unwrap();
    let h = r.series("v(out)").unwrap();
    let found = pole_zero(&r.freqs, &h, None, &s).unwrap();
    assert_eq!(found.poles.len(), 1);
    assert_eq!(found.zeros.len(), 1);
    let zero = 1.0 / (9e3 * 100e-9);
    let pole = 1.0 / ((9e3 * 1e3 / 10e3) * 100e-9);
    assert!(
        (found.zeros[0].abs() / zero - 1.0).abs() < 1e-5,
        "{:?}",
        found.zeros
    );
    assert!(
        (found.poles[0].abs() / pole - 1.0).abs() < 1e-5,
        "{:?}",
        found.poles
    );
}

#[test]
fn the_figures_of_a_second_order_butterworth_low_pass() {
    // Q = sqrt(L/C)/R = 1/sqrt(2) with R = 44.72 ohm, f0 = 1/(2 pi sqrt(L C)) = 5033 Hz; a maximally
    // flat response has its -3 dB point at f0
    let s = settings();
    let c = circuit(
        "butterworth\nV1 in 0 AC 1\nL1 in a 1m\nR1 a out 44.721359549995796\nC1 out 0 1u\n",
    );
    let r = ac_analysis(&c, &s, &sweep(100, 10.0, 1e6)).unwrap();
    let values = figures(&r.freqs, &r.series("v(out)").unwrap());
    let f0 = 1.0 / (2.0 * PI * (1e-3f64 * 1e-6).sqrt());
    let q = figure(&values, "q_lowpass").unwrap();
    assert!((q * 2f64.sqrt() - 1.0).abs() < 1e-3, "{q}");
    let fc = figure(&values, "cutoff_3db").unwrap();
    assert!((fc / f0 - 1.0).abs() < 1e-3, "{fc} {f0}");
    assert!((figure(&values, "dc_gain").unwrap() - 1.0).abs() < 1e-6);
    // no peak above the gain at DC
    assert!(figure(&values, "peak_gain").unwrap() <= 1.0 + 1e-6);
}

#[test]
fn the_bandwidth_and_the_quality_of_a_series_rlc_band_pass() {
    // across the resistor: B = R/(2 pi L), Q = sqrt(L/C)/R
    let s = settings();
    let c = circuit("bandpass\nV1 in 0 AC 1\nL1 in a 1m\nC1 a out 1u\nR1 out 0 10\n");
    let r = ac_analysis(&c, &s, &sweep(100, 10.0, 1e6)).unwrap();
    let values = figures(&r.freqs, &r.series("v(out)").unwrap());
    let q = figure(&values, "q_bandpass").unwrap();
    assert!(
        (q / ((1e-3f64 / 1e-6).sqrt() / 10.0) - 1.0).abs() < 1e-3,
        "{q}"
    );
    let peak = figure(&values, "peak_gain").unwrap();
    assert!((peak - 1.0).abs() < 1e-3, "{peak}");
    let f0 = figure(&values, "peak_freq").unwrap();
    assert!((f0 / 5032.921210448703 - 1.0).abs() < 1e-3, "{f0}");
}

#[test]
fn a_step_on_a_parameter_and_monte_carlo_work_through_one_plan() {
    let s = settings();
    let text = "div\n.param rl=1k\nV1 a 0 DC 10\nR1 a b 1k tol=2% dist=uniform\nR2 b 0 {rl} tol=2% dist=uniform\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n.step param rl list 1k 3k\n";
    let mut plan = StudyPlan::from_netlist(&parse(text, &s).unwrap());
    plan.mc = Some(McSpec {
        runs: 2000,
        seed: 11,
    });
    let build = |overrides: &BTreeMap<String, f64>| parse_with_params(text, &s, overrides);
    let table = run_study(&plan, &build, &s).unwrap();
    assert_eq!(table.rows.len(), 4000);
    assert_eq!(table.summaries.len(), 2);
    // Vout = 10 rl/(1k + rl): 5 V and 7.5 V; the deviation follows from the derivatives with
    // respect to R1 (-10 rl/(R1 + rl)^2) and rl (10 R1/(R1 + rl)^2), each with a deviation
    // of 2 % of the value over sqrt(3)
    for (summary, (rl, mean)) in table.summaries.iter().zip([(1e3, 5.0), (3e3, 7.5)]) {
        let m = summary.per_measure[0].as_ref().unwrap();
        let r1 = 1e3;
        let d1 = -10.0 * rl / ((r1 + rl) * (r1 + rl));
        let d2 = 10.0 * r1 / ((r1 + rl) * (r1 + rl));
        let sigma = ((d1 * 0.02 * r1).powi(2) + (d2 * 0.02 * rl).powi(2)).sqrt() / 3f64.sqrt();
        assert!(
            (m.mean - mean).abs() < 6.0 * sigma / 2000f64.sqrt() + 1e-4,
            "{}",
            m.mean
        );
        assert!((m.std / sigma - 1.0).abs() < 0.08, "{} {sigma}", m.std);
    }
}
