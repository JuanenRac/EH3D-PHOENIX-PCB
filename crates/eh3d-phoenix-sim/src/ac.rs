// SPDX-License-Identifier: GPL-3.0-or-later
//! The AC (small-signal) analysis: the circuit is linearised at its operating point (the diodes
//! become their conductance), the equations are solved in complex numbers at each frequency, and
//! for each unknown the magnitude, the phase and the group delay are read from the result. The
//! group delay is `-dφ/dω`, taken from the phase a small fraction of the frequency below and above
//! the frequency (`delay_step_fraction`).

use std::f64::consts::PI;

use eh3d_phoenix_rf::Complex;

use crate::circuit::Circuit;
use crate::codes::{SimError, ANALYSIS_INVALID, LIMIT_EXCEEDED, NOT_FINITE};
use crate::dc::solve_dc;
use crate::linalg::solve_complex;
use crate::real::{diode_point, singular_error, volt};
use crate::settings::SimSettings;

/// How the frequencies of a sweep are spaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepKind {
    /// Evenly: `points` frequencies from the start to the stop.
    Linear,
    /// `points` frequencies per decade.
    Decade,
    /// `points` frequencies per octave.
    Octave,
}

/// A sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcRequest {
    /// The spacing.
    pub kind: SweepKind,
    /// The points: in all, or per decade, or per octave.
    pub points: usize,
    /// The first frequency in Hz.
    pub start_hz: f64,
    /// The last frequency in Hz.
    pub stop_hz: f64,
}

/// The result of an AC analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct AcResult {
    /// The labels of the unknowns.
    pub labels: Vec<String>,
    /// How many of the labels are node voltages.
    pub node_count: usize,
    /// The frequencies in Hz.
    pub freqs: Vec<f64>,
    /// For each frequency, the unknowns as complex numbers.
    pub values: Vec<Vec<Complex>>,
    /// For each frequency, the group delay of each unknown in seconds.
    pub delay_s: Vec<Vec<f64>>,
}

impl AcResult {
    /// The index of a label, compared in lower case.
    pub fn column(&self, label: &str) -> Option<usize> {
        let wanted = label.trim().to_lowercase();
        self.labels.iter().position(|l| *l == wanted)
    }

    /// One unknown over the frequencies.
    pub fn series(&self, label: &str) -> Option<Vec<Complex>> {
        let k = self.column(label)?;
        Some(self.values.iter().map(|row| row[k]).collect())
    }

    /// The group delay of one unknown over the frequencies.
    pub fn delay(&self, label: &str) -> Option<Vec<f64>> {
        let k = self.column(label)?;
        Some(self.delay_s.iter().map(|row| row[k]).collect())
    }
}

fn invalid(why: &str) -> SimError {
    SimError::new(ANALYSIS_INVALID, why.to_string())
}

/// The frequencies of a sweep.
pub fn frequencies(req: &AcRequest, s: &SimSettings) -> Result<Vec<f64>, SimError> {
    if req.points == 0 {
        return Err(invalid("a sweep needs at least one point"));
    }
    if !(req.start_hz.is_finite() && req.start_hz > 0.0) {
        return Err(invalid("the start frequency must be above zero"));
    }
    if !req.stop_hz.is_finite() || req.stop_hz < req.start_hz {
        return Err(invalid(
            "the stop frequency cannot be below the start frequency",
        ));
    }
    let too_many = |count: f64| {
        SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "{count:.0} frequencies are more than max_ac_points ({})",
                s.max_ac_points
            ),
        )
    };
    let (start, stop) = (req.start_hz, req.stop_hz);
    match req.kind {
        SweepKind::Linear => {
            if req.points > s.max_ac_points {
                return Err(too_many(req.points as f64));
            }
            if req.points == 1 {
                return Ok(vec![start]);
            }
            let last = (req.points - 1) as f64;
            let mut out: Vec<f64> = (0..req.points)
                .map(|i| start + (stop - start) * i as f64 / last)
                .collect();
            out[req.points - 1] = stop;
            Ok(out)
        }
        SweepKind::Decade | SweepKind::Octave => {
            let (base, span) = if req.kind == SweepKind::Decade {
                (10.0f64, (stop / start).log10())
            } else {
                (2.0f64, (stop / start).log2())
            };
            let per = req.points as f64;
            let count = (per * span + 1e-9).floor();
            if count + 2.0 > s.max_ac_points as f64 {
                return Err(too_many(count + 1.0));
            }
            let mut out: Vec<f64> = (0..=(count as usize))
                .map(|i| start * base.powf(i as f64 / per))
                .collect();
            if out.last().is_some_and(|last| *last < stop * (1.0 - 1e-9)) {
                out.push(stop);
            }
            Ok(out)
        }
    }
}

fn add(a: &mut [Complex], n: usize, r: Option<usize>, col: Option<usize>, v: Complex) {
    if let (Some(r), Some(col)) = (r, col) {
        a[r * n + col] = a[r * n + col] + v;
    }
}

fn row(node: usize) -> Option<usize> {
    node.checked_sub(1)
}

fn conduct(a: &mut [Complex], n: usize, p: usize, q: usize, y: Complex) {
    let (rp, rq) = (row(p), row(q));
    add(a, n, rp, rp, y);
    add(a, n, rq, rq, y);
    add(a, n, rp, rq, -y);
    add(a, n, rq, rp, -y);
}

fn branch_kcl(a: &mut [Complex], n: usize, nn: usize, p: usize, q: usize, branch: usize) {
    add(a, n, row(p), Some(nn + branch), Complex::ONE);
    add(a, n, row(q), Some(nn + branch), -Complex::ONE);
}

fn branch_voltage(a: &mut [Complex], n: usize, nn: usize, p: usize, q: usize, branch: usize) {
    add(a, n, Some(nn + branch), row(p), Complex::ONE);
    add(a, n, Some(nn + branch), row(q), -Complex::ONE);
}

/// The matrix and the right-hand side at an angular frequency; `gd` is the small-signal
/// conductance of each diode.
fn assemble(c: &Circuit, omega: f64, gd: &[f64]) -> (Vec<Complex>, Vec<Complex>) {
    let n = c.unknowns();
    let nn = c.node_names.len();
    let mut a = vec![Complex::ZERO; n * n];
    let mut b = vec![Complex::ZERO; n];

    for e in &c.resistors {
        conduct(&mut a, n, e.a, e.b, Complex::real(1.0 / e.ohms));
    }
    for e in &c.capacitors {
        conduct(&mut a, n, e.a, e.b, Complex::new(0.0, omega * e.farads));
    }
    for e in &c.inductors {
        let r = nn + e.branch;
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        add(
            &mut a,
            n,
            Some(r),
            Some(r),
            Complex::new(0.0, -omega * e.henries),
        );
        for (j, m) in &e.partners {
            let col = nn + c.inductors[*j].branch;
            add(&mut a, n, Some(r), Some(col), Complex::new(0.0, -omega * m));
        }
    }
    for e in &c.vsources {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        b[nn + e.branch] = e.source.ac_phasor();
    }
    for e in &c.isources {
        let phasor = e.source.ac_phasor();
        if let Some(r) = row(e.a) {
            b[r] = b[r] - phasor;
        }
        if let Some(r) = row(e.b) {
            b[r] = b[r] + phasor;
        }
    }
    for e in &c.vcvs {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        add(
            &mut a,
            n,
            Some(nn + e.branch),
            row(e.cp),
            Complex::real(-e.gain),
        );
        add(
            &mut a,
            n,
            Some(nn + e.branch),
            row(e.cn),
            Complex::real(e.gain),
        );
    }
    for e in &c.vccs {
        let gm = Complex::real(e.gm);
        add(&mut a, n, row(e.a), row(e.cp), gm);
        add(&mut a, n, row(e.a), row(e.cn), -gm);
        add(&mut a, n, row(e.b), row(e.cp), -gm);
        add(&mut a, n, row(e.b), row(e.cn), gm);
    }
    for e in &c.cccs {
        let gain = Complex::real(e.gain);
        add(&mut a, n, row(e.a), Some(nn + e.control), gain);
        add(&mut a, n, row(e.b), Some(nn + e.control), -gain);
    }
    for e in &c.ccvs {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        add(
            &mut a,
            n,
            Some(nn + e.branch),
            Some(nn + e.control),
            Complex::real(-e.rm),
        );
    }
    for (k, d) in c.diodes.iter().enumerate() {
        conduct(&mut a, n, d.a, d.k, Complex::real(gd[k]));
    }
    (a, b)
}

/// The phase difference wrapped into one turn.
fn wrapped(mut d: f64) -> f64 {
    while d > PI {
        d -= 2.0 * PI;
    }
    while d <= -PI {
        d += 2.0 * PI;
    }
    d
}

/// Runs the analysis at the given frequencies.
pub fn ac_at(c: &Circuit, s: &SimSettings, freqs: &[f64]) -> Result<AcResult, SimError> {
    let n = c.unknowns();
    if freqs.len() * n > s.max_result_cells {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the result would hold more than max_result_cells ({}) values",
                s.max_result_cells
            ),
        ));
    }
    if s.floating_is_error {
        if let Some(error) = c.floating_error(false) {
            return Err(error);
        }
    }
    let gd: Vec<f64> = if c.is_nonlinear() {
        let (x, _, _) = solve_dc(c, s, s.gmin)?;
        let vt = s.thermal_voltage();
        c.diodes
            .iter()
            .map(|d| diode_point(d.is, d.n * vt, s.gmin, volt(&x, d.a) - volt(&x, d.k)).1)
            .collect()
    } else {
        Vec::new()
    };

    let solve = |freq: f64| -> Result<Vec<Complex>, SimError> {
        if !(freq.is_finite() && freq > 0.0) {
            return Err(invalid("a frequency must be above zero"));
        }
        let (mut a, mut b) = assemble(c, 2.0 * PI * freq, &gd);
        solve_complex(&mut a, &mut b, n, s.pivot_tolerance).map_err(|k| singular_error(c, k))?;
        if b.iter().any(|v| !v.is_finite()) {
            return Err(SimError::new(
                NOT_FINITE,
                format!("the solution at {freq} Hz is not a finite number"),
            ));
        }
        Ok(b)
    };

    let mut result = AcResult {
        labels: c.labels(),
        node_count: c.node_names.len(),
        freqs: freqs.to_vec(),
        values: Vec::with_capacity(freqs.len()),
        delay_s: Vec::with_capacity(freqs.len()),
    };
    for &freq in freqs {
        let x = solve(freq)?;
        let below = solve(freq * (1.0 - s.delay_step_fraction))?;
        let above = solve(freq * (1.0 + s.delay_step_fraction))?;
        let span = 2.0 * PI * freq * 2.0 * s.delay_step_fraction;
        let delays: Vec<f64> = below
            .iter()
            .zip(above.iter())
            .map(|(lo, hi)| -wrapped(hi.arg() - lo.arg()) / span)
            .collect();
        result.values.push(x);
        result.delay_s.push(delays);
    }
    Ok(result)
}

/// Runs a sweep.
pub fn ac_analysis(c: &Circuit, s: &SimSettings, req: &AcRequest) -> Result<AcResult, SimError> {
    let freqs = frequencies(req, s)?;
    ac_at(c, s, &freqs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn circuit(text: &str) -> Circuit {
        let s = SimSettings::default();
        Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
    }

    fn at(text: &str, freqs: &[f64]) -> AcResult {
        ac_at(&circuit(text), &SimSettings::default(), freqs).unwrap()
    }

    fn sweep(kind: SweepKind, points: usize, start_hz: f64, stop_hz: f64) -> Vec<f64> {
        let req = AcRequest {
            kind,
            points,
            start_hz,
            stop_hz,
        };
        frequencies(&req, &SimSettings::default()).unwrap()
    }

    #[test]
    fn sweeps_are_spaced_as_asked() {
        let lin = sweep(SweepKind::Linear, 5, 100.0, 500.0);
        assert_eq!(lin, [100.0, 200.0, 300.0, 400.0, 500.0]);
        assert_eq!(sweep(SweepKind::Linear, 1, 7.0, 9.0), [7.0]);
        let dec = sweep(SweepKind::Decade, 10, 1.0, 1000.0);
        assert_eq!(dec.len(), 31);
        assert_eq!(dec[0], 1.0);
        assert!((dec[10] - 10.0).abs() < 1e-9 && (dec[30] - 1000.0).abs() < 1e-6);
        let oct = sweep(SweepKind::Octave, 3, 100.0, 800.0);
        assert_eq!(oct.len(), 10);
        assert!((oct[3] - 200.0).abs() < 1e-9 && (oct[9] - 800.0).abs() < 1e-9);
        // a stop that is not on the grid is added at the end
        let odd = sweep(SweepKind::Decade, 1, 1.0, 50.0);
        assert_eq!(odd.len(), 3);
        assert_eq!(odd[2], 50.0);
    }

    #[test]
    fn bad_sweeps_are_refused() {
        let s = SimSettings::default();
        for (kind, points, start_hz, stop_hz) in [
            (SweepKind::Linear, 0, 1.0, 2.0),
            (SweepKind::Linear, 5, 0.0, 2.0),
            (SweepKind::Decade, 5, 10.0, 1.0),
            (SweepKind::Octave, 5, 1.0, f64::NAN),
        ] {
            let req = AcRequest {
                kind,
                points,
                start_hz,
                stop_hz,
            };
            assert_eq!(frequencies(&req, &s).unwrap_err().code, ANALYSIS_INVALID);
        }
        let small = SimSettings::parse("max_ac_points = 10\n").unwrap();
        let req = AcRequest {
            kind: SweepKind::Decade,
            points: 10,
            start_hz: 1.0,
            stop_hz: 1e6,
        };
        assert_eq!(frequencies(&req, &small).unwrap_err().code, LIMIT_EXCEEDED);
    }

    #[test]
    fn the_rc_low_pass_is_3_db_down_at_one_over_two_pi_rc() {
        let text = "rc\nV1 in 0 AC 1\nR1 in out 1k\nC1 out 0 1u\n";
        let fc = 1.0 / (2.0 * PI * 1e3 * 1e-6);
        let r = at(text, &[fc]);
        let h = r.series("v(out)").unwrap()[0];
        assert!((h.abs() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        assert!((h.db20() - 20.0 * std::f64::consts::FRAC_1_SQRT_2.log10()).abs() < 1e-9);
        assert!((h.arg_deg() + 45.0).abs() < 1e-9);
        // the group delay of a first-order low pass is tau / (1 + (w tau)^2), half of tau here
        let delay = r.delay("v(out)").unwrap()[0];
        assert!((delay - 0.5e-3).abs() < 1e-9, "{delay}");
    }

    #[test]
    fn the_low_pass_rolls_off_at_20_db_per_decade() {
        let text = "rc\nV1 in 0 AC 1\nR1 in out 1k\nC1 out 0 1u\n";
        let r = at(text, &[1.0, 1e5, 1e6]);
        let h = r.series("v(out)").unwrap();
        assert!((h[0].abs() - 1.0).abs() < 1e-4);
        assert!((h[1].db20() - h[2].db20() - 20.0).abs() < 1e-3);
        let phase = h[2].arg_deg();
        assert!(phase < -89.0 && phase > -90.0);
    }

    #[test]
    fn the_series_rlc_resonates_with_unit_gain_and_the_quality_factor_of_the_book() {
        // across R: a band-pass with f0 = 1 / (2 pi sqrt(LC)) and Q = sqrt(L/C) / R
        let text = "rlc\nV1 in 0 AC 1\nL1 in a 1m\nC1 a out 1u\nR1 out 0 10\n";
        let (l, cap, rr) = (1e-3f64, 1e-6f64, 10.0f64);
        let f0 = 1.0 / (2.0 * PI * (l * cap).sqrt());
        let q = (l / cap).sqrt() / rr;
        assert!((q - 10f64.sqrt()).abs() < 1e-12);
        let half = 1.0 / (2.0 * q);
        let ratio_hi = (1.0 + half * half).sqrt() + half;
        let ratio_lo = (1.0 + half * half).sqrt() - half;
        let r = at(text, &[f0, f0 * ratio_hi, f0 * ratio_lo]);
        let h = r.series("v(out)").unwrap();
        assert!((h[0].abs() - 1.0).abs() < 1e-9, "{}", h[0].abs());
        assert!(h[0].arg_deg().abs() < 1e-6);
        let half_power = std::f64::consts::FRAC_1_SQRT_2;
        assert!((h[1].abs() - half_power).abs() < 1e-9);
        assert!((h[2].abs() - half_power).abs() < 1e-9);
        // Q = f0 / bandwidth
        let measured = f0 / (f0 * ratio_hi - f0 * ratio_lo);
        assert!((measured - q).abs() < 1e-12);
        // the phase is positive below resonance and negative above
        assert!(h[2].arg_deg() > 0.0 && h[1].arg_deg() < 0.0);
        // at resonance the current of the source is 1 / R, entering the source
        let i = r.series("i(v1)").unwrap()[0];
        assert!((i.abs() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn two_inductors_coupled_by_k_near_one_make_a_transformer() {
        let text = "xf\nV1 p 0 AC 1\nL1 p 0 100m\nL2 s 0 400m\nK1 L1 L2 0.9999\nRL s 0 10k\n";
        let r = at(text, &[1000.0]);
        let h = r.series("v(s)").unwrap()[0];
        // a turns ratio of sqrt(L2 / L1) = 2, times k, with the output turned by half a turn
        assert!((h.abs() - 2.0 * 0.9999).abs() < 5e-4, "{}", h.abs());
        assert!((h.arg_deg().abs() - 180.0).abs() < 0.01, "{}", h.arg_deg());
    }

    #[test]
    fn a_diode_is_linearised_at_its_operating_point() {
        // 5 V bias through 1k into the diode, a 1 mV small signal on top
        let text = "d\n.model dm D(IS=1e-14 N=1)\nV1 in 0 DC 5 AC 1m\nR1 in d 1k\nD1 d 0 dm\n";
        let c = circuit(text);
        let s = SimSettings::default();
        let r = ac_at(&c, &s, &[1000.0]).unwrap();
        let h = r.series("v(d)").unwrap()[0];
        // rd = vt / id with id about 4.3 mA, so the divider gives rd / (rd + 1k)
        let vt = s.thermal_voltage();
        let id = 4.307e-3;
        let rd = vt / id;
        let want = 1e-3 * rd / (rd + 1000.0);
        assert!((h.abs() - want).abs() / want < 5e-3, "{} {}", h.abs(), want);
    }

    #[test]
    fn a_source_without_an_ac_part_gives_zero_and_a_floating_node_is_refused() {
        let r = at("t\nV1 in 0 DC 5\nR1 in 0 1k\n", &[1000.0]);
        assert_eq!(r.series("v(in)").unwrap()[0], Complex::ZERO);
        let c = circuit("t\nV1 a 0 AC 1\nR1 b c 1k\nR2 c 0 1k\nI1 0 d 1\nR3 d 0 1\n");
        // b and c are tied to ground by R2; every node is reached, so this is accepted
        assert!(ac_at(&c, &SimSettings::default(), &[10.0]).is_ok());
        let f = circuit("t\nV1 a 0 AC 1\nR1 b c 1k\nR2 d 0 1k\nI1 0 b 1\n");
        let e = ac_at(&f, &SimSettings::default(), &[10.0]).unwrap_err();
        assert_eq!(e.code, crate::codes::FLOATING_NODE);
    }

    #[test]
    fn a_frequency_of_zero_is_refused() {
        let c = circuit("t\nV1 a 0 AC 1\nR1 a 0 1k\n");
        let e = ac_at(&c, &SimSettings::default(), &[0.0]).unwrap_err();
        assert_eq!(e.code, ANALYSIS_INVALID);
    }
}
