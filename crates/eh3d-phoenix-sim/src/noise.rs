// SPDX-License-Identifier: GPL-3.0-or-later
//! The noise analysis at the output: the thermal noise of the resistors, the shot noise of the
//! junctions and the thermal noise of the channel of a MOSFET. Each noise source is a current
//! source in parallel with the device, with a flat spectral density; the circuit is linearised at
//! its operating point as in the AC analysis, a unit current is injected where each source is, and
//! the voltage it makes at the output, squared, times the density of the source, is its
//! contribution. The contributions are added as powers, since the sources are not correlated.
//!
//! There is no flicker noise, no noise of the op-amp macromodel and no noise referred to the input.

use crate::ac::{assemble, frequencies, AcRequest};
use crate::circuit::Circuit;
use crate::codes::{SimError, ANALYSIS_INVALID, LIMIT_EXCEEDED, NOT_FINITE, UNDEFINED};
use crate::dc::solve_dc;
use crate::linalg::solve_complex;
use crate::real::{
    bjt_point, breakdown_point, diode_point, mos_point, singular_error, small_signal, volt,
};
use crate::settings::{SimSettings, BOLTZMANN, CHARGE};
use eh3d_phoenix_rf::Complex;

use std::f64::consts::PI;

/// The contribution of one noise source to the output noise.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseContribution {
    /// The source: the name of a resistor, or the name of a device and the kind of noise.
    pub source: String,
    /// The power spectral density at the output at each frequency, in V^2/Hz.
    pub density: Vec<f64>,
}

/// The result of a noise analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseResult {
    /// The output, as asked (`v(out)`), in lower case.
    pub output: String,
    /// The frequencies in Hz.
    pub freqs: Vec<f64>,
    /// The contribution of each source.
    pub contributions: Vec<NoiseContribution>,
    /// The total power spectral density at each frequency, in V^2/Hz.
    pub total: Vec<f64>,
    /// The root mean square of the noise voltage over the whole sweep, in volt (the density
    /// integrated by the trapezoidal rule over the frequencies of the sweep).
    pub rms_volts: f64,
}

impl NoiseResult {
    /// The voltage density at each frequency, in V/sqrt(Hz).
    pub fn voltage_density(&self) -> Vec<f64> {
        self.total.iter().map(|p| p.sqrt()).collect()
    }
}

/// A noise current source between two nodes with a flat density in A^2/Hz.
struct Generator {
    name: String,
    from: usize,
    to: usize,
    psd: f64,
}

/// The noise sources of the circuit at the operating point `x`.
fn generators(c: &Circuit, s: &SimSettings, x: &[f64]) -> Vec<Generator> {
    let kt = BOLTZMANN * (s.temperature_c + 273.15);
    let vt = s.thermal_voltage();
    let mut out: Vec<Generator> = Vec::new();
    for r in c.resistors.iter().filter(|r| r.noisy) {
        out.push(Generator {
            name: r.name.clone(),
            from: r.a,
            to: r.b,
            psd: 4.0 * kt / r.ohms,
        });
    }
    for d in &c.diodes {
        let v = volt(x, d.a) - volt(x, d.k);
        let (mut id, _) = diode_point(d.is, d.n * vt, 0.0, v);
        if let Some(bv) = d.bv {
            id += breakdown_point(bv, d.ibv, d.nbv * vt, v).0;
        }
        out.push(Generator {
            name: format!("{} shot", d.name),
            from: d.a,
            to: d.k,
            psd: 2.0 * CHARGE * id.abs(),
        });
    }
    for q in &c.bjts {
        let p = q.polarity;
        let vbe = p * (volt(x, q.b) - volt(x, q.e));
        let vbc = p * (volt(x, q.b) - volt(x, q.c));
        let pt = bjt_point(q, vt, 0.0, vbe, vbc);
        out.push(Generator {
            name: format!("{} collector shot", q.name),
            from: q.c,
            to: q.e,
            psd: 2.0 * CHARGE * pt.ic.abs(),
        });
        out.push(Generator {
            name: format!("{} base shot", q.name),
            from: q.b,
            to: q.e,
            psd: 2.0 * CHARGE * pt.ib.abs(),
        });
    }
    for m in &c.mosfets {
        let p = m.polarity;
        let swapped = p * (volt(x, m.d) - volt(x, m.s)) < 0.0;
        let (dn, sn) = if swapped { (m.s, m.d) } else { (m.d, m.s) };
        let vgs = p * (volt(x, m.g) - volt(x, sn));
        let vds = p * (volt(x, dn) - volt(x, sn));
        let (_, gm, gds) = mos_point(m.beta, m.vto, m.lambda, vgs, vds);
        // 2/3 of the transconductance in saturation, the conductance of the channel in the triode
        let g = if vds >= vgs - m.vto {
            2.0 / 3.0 * gm
        } else {
            gds
        };
        out.push(Generator {
            name: format!("{} channel", m.name),
            from: dn,
            to: sn,
            psd: 4.0 * kt * g,
        });
    }
    out
}

/// Runs the noise analysis at the output `output` (a node voltage such as `v(out)`) over the
/// frequencies of a sweep.
pub fn noise_analysis(
    c: &Circuit,
    s: &SimSettings,
    output: &str,
    sweep: &AcRequest,
) -> Result<NoiseResult, SimError> {
    let wanted = output.trim().to_lowercase();
    let Some(index) = c.unknown_index(&wanted) else {
        return Err(SimError::new(
            UNDEFINED,
            format!("there is no {output} in this circuit"),
        ));
    };
    if index >= c.node_names.len() {
        return Err(SimError::new(
            ANALYSIS_INVALID,
            "the noise is measured at the voltage of a node".to_string(),
        ));
    }
    let freqs = frequencies(sweep, s)?;
    if s.floating_is_error {
        if let Some(error) = c.floating_error(false) {
            return Err(error);
        }
    }
    let n = c.unknowns();
    let x_op = if c.is_nonlinear() {
        solve_dc(c, s, s.gmin)?.0
    } else {
        vec![0.0; n]
    };
    let ss = small_signal(c, s, &x_op);
    let gens = generators(c, s, &x_op);
    if freqs.len() * (gens.len() + 1) > s.max_result_cells {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the result would hold more than max_result_cells ({}) values",
                s.max_result_cells
            ),
        ));
    }

    let mut contributions: Vec<NoiseContribution> = gens
        .iter()
        .map(|g| NoiseContribution {
            source: g.name.clone(),
            density: Vec::with_capacity(freqs.len()),
        })
        .collect();
    let mut total = vec![0.0; freqs.len()];
    for (k, &freq) in freqs.iter().enumerate() {
        let (matrix, _) = assemble(c, 2.0 * PI * freq, &ss);
        for (g, contribution) in gens.iter().zip(contributions.iter_mut()) {
            let mut a = matrix.clone();
            let mut b = vec![Complex::ZERO; n];
            if let Some(r) = g.from.checked_sub(1) {
                b[r] = b[r] - Complex::ONE;
            }
            if let Some(r) = g.to.checked_sub(1) {
                b[r] = b[r] + Complex::ONE;
            }
            solve_complex(&mut a, &mut b, n, s.pivot_tolerance)
                .map_err(|column| singular_error(c, column))?;
            let v = b[index];
            if !v.is_finite() {
                return Err(SimError::new(
                    NOT_FINITE,
                    format!("the noise at {freq} Hz is not a finite number"),
                ));
            }
            let density = v.norm_sqr() * g.psd;
            contribution.density.push(density);
            total[k] += density;
        }
    }
    let power: f64 = freqs
        .windows(2)
        .zip(total.windows(2))
        .map(|(f, p)| (p[0] + p[1]) / 2.0 * (f[1] - f[0]))
        .sum();
    Ok(NoiseResult {
        output: wanted,
        freqs,
        contributions,
        total,
        rms_volts: power.sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ac::SweepKind;
    use crate::parse::parse;

    fn circuit(text: &str) -> Circuit {
        let s = SimSettings::default();
        Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
    }

    fn sweep(points: usize, start_hz: f64, stop_hz: f64) -> AcRequest {
        AcRequest {
            kind: SweepKind::Decade,
            points,
            start_hz,
            stop_hz,
        }
    }

    fn kt() -> f64 {
        BOLTZMANN * (SimSettings::default().temperature_c + 273.15)
    }

    #[test]
    fn a_resistor_makes_four_k_t_r_of_noise_at_every_frequency() {
        let c = circuit("n\nR1 out 0 1k\n");
        let r =
            noise_analysis(&c, &SimSettings::default(), "V(OUT)", &sweep(2, 10.0, 1e6)).unwrap();
        let want = 4.0 * kt() * 1000.0;
        assert_eq!(r.output, "v(out)");
        assert_eq!(r.contributions.len(), 1);
        assert_eq!(r.contributions[0].source, "R1");
        for p in &r.total {
            assert!((p - want).abs() < 1e-9 * want, "{p} {want}");
        }
        // 4.07 nV per root hertz
        let density = r.voltage_density();
        assert!((density[0] - 4.0714e-9).abs() < 1e-12, "{}", density[0]);
    }

    #[test]
    fn a_resistor_and_a_capacitor_hold_a_noise_of_the_square_root_of_kt_over_c() {
        let c = circuit("n\nR1 out 0 1k\nC1 out 0 1p\n");
        let r = noise_analysis(
            &c,
            &SimSettings::default(),
            "v(out)",
            &sweep(100, 1.0, 1e12),
        )
        .unwrap();
        let want = (kt() / 1e-12).sqrt();
        assert!(
            (r.rms_volts - want).abs() < 2e-3 * want,
            "{} {want}",
            r.rms_volts
        );
        // the density is flat below the corner and falls above it
        let density = r.voltage_density();
        assert!(density[0] > 100.0 * density[density.len() - 1]);
    }

    #[test]
    fn the_shot_noise_of_a_forward_diode_is_two_q_i_over_its_conductance_squared() {
        let c = circuit("n\n.model dm D(IS=1e-14 N=1)\nI1 0 d 1m\nD1 d 0 dm\n");
        let s = SimSettings::default();
        let r = noise_analysis(&c, &s, "v(d)", &sweep(1, 100.0, 1000.0)).unwrap();
        let vt = s.thermal_voltage();
        let id = 1e-3f64;
        let gd = id / vt;
        let want = 2.0 * CHARGE * id / (gd * gd);
        assert_eq!(r.contributions[0].source, "D1 shot");
        assert!(
            (r.total[0] - want).abs() < 1e-3 * want,
            "{} {want}",
            r.total[0]
        );
    }

    #[test]
    fn the_total_is_the_sum_of_the_contributions_and_two_runs_agree() {
        let text = "n\n.model qm NPN(IS=1e-14 BF=100 RB=100)\nV1 vcc 0 10\nR1 vcc b 2.2meg\nR2 vcc c 4.7k\nQ1 c b 0 qm\n";
        let c = circuit(text);
        let s = SimSettings::default();
        let a = noise_analysis(&c, &s, "v(c)", &sweep(3, 10.0, 1e5)).unwrap();
        let b = noise_analysis(&c, &s, "v(c)", &sweep(3, 10.0, 1e5)).unwrap();
        assert_eq!(a, b);
        let names: Vec<&str> = a.contributions.iter().map(|k| k.source.as_str()).collect();
        assert_eq!(
            names,
            ["R1", "R2", "Q1.rb", "Q1 collector shot", "Q1 base shot"]
        );
        for (k, total) in a.total.iter().enumerate() {
            let sum: f64 = a.contributions.iter().map(|x| x.density[k]).sum();
            assert!((sum - total).abs() <= 1e-12 * total);
            assert!(*total > 0.0);
        }
    }

    #[test]
    fn an_output_that_is_not_a_node_is_refused() {
        let c = circuit("n\nV1 a 0 1\nR1 a 0 1k\n");
        let s = SimSettings::default();
        let e = noise_analysis(&c, &s, "v(zz)", &sweep(1, 1.0, 10.0)).unwrap_err();
        assert_eq!(e.code, UNDEFINED);
        let e = noise_analysis(&c, &s, "i(v1)", &sweep(1, 1.0, 10.0)).unwrap_err();
        assert_eq!(e.code, ANALYSIS_INVALID);
    }
}
