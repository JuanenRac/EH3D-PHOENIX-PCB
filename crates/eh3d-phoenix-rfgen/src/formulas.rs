// SPDX-License-Identifier: GPL-3.0-or-later
//! The published formulas of the generator, as functions on numbers in SI units (metres, hertz,
//! ohm). Each function says where its formula comes from, and each has a companion that lists the
//! places where the inputs leave the range the program checks, so that the report can warn. The
//! integer geometry of the board never goes through here: lengths come out as `f64` and are
//! converted to nanometres by [`crate::layout::nm_from_m`].
//!
//! **State: prototype.** None of these formulas has been compared with a field solver or with a
//! measurement of a board.

use std::f64::consts::PI;

use eh3d_phoenix_rf::filter::Prototype;
use eh3d_phoenix_stackup::elliptic::k_ratio;
use eh3d_phoenix_stackup::models::microstrip;

use crate::codes::{RfgenError, NOT_REALISABLE, PARAMETER_INVALID, SOLVER_FAILED};

/// The speed of light in vacuum, in m/s.
pub const C0: f64 = 299_792_458.0;
/// The permittivity of vacuum, in F/m.
pub const EPS0: f64 = 8.854_187_812_8e-12;

/// The optimum fraction of the diagonal that a mitered right-angle bend cuts off, `M = x / d`,
/// with `x` the distance from the outer corner to the cut along the diagonal and `d` the diagonal
/// of the corner square. Douville and James, "Experimental study of symmetric microstrip bends and
/// their compensation", IEEE Trans. MTT, 1978: `M = 52 + 65 exp(-1.35 W/h)` per cent. The authors
/// give it for `W/h >= 0.25` and `er <= 25`.
pub fn miter_fraction(w_over_h: f64) -> f64 {
    0.52 + 0.65 * (-1.35 * w_over_h).exp()
}

/// Where the inputs of [`miter_fraction`] leave the range of the source.
pub fn miter_range(w_over_h: f64, er: f64) -> Vec<String> {
    let mut out = Vec::new();
    if w_over_h < 0.25 {
        out.push(format!("W/h = {w_over_h:.3} is below 0.25"));
    }
    if er > 25.0 {
        out.push(format!("er = {er:.2} is above 25"));
    }
    out
}

/// The length that the fringing field at the open end of a microstrip adds, in metres. Hammerstad,
/// "Equations for microstrip circuit design", 5th European Microwave Conference, 1975:
/// `dL / h = 0.412 (e + 0.3) / (e - 0.258) * (W/h + 0.264) / (W/h + 0.8)` with `e` the effective
/// permittivity. The same expression is the extension of the edge of a patch (Balanis, Antenna
/// Theory, section 14.2).
pub fn open_end_extension_m(w_m: f64, h_m: f64, eps_eff: f64) -> f64 {
    let u = w_m / h_m;
    h_m * 0.412 * (eps_eff + 0.3) / (eps_eff - 0.258) * (u + 0.264) / (u + 0.8)
}

/// Where the inputs of [`open_end_extension_m`] leave the range this program checks (`0.1 <= W/h
/// <= 100`, `1 <= er <= 20`).
pub fn open_end_range(w_over_h: f64, er: f64) -> Vec<String> {
    let mut out = Vec::new();
    if !(0.1..=100.0).contains(&w_over_h) {
        out.push(format!("W/h = {w_over_h:.3} is outside 0.1 to 100"));
    }
    if !(1.0..=20.0).contains(&er) {
        out.push(format!("er = {er:.2} is outside 1 to 20"));
    }
    out
}

/// The even- and odd-mode impedances and effective permittivities of a symmetric pair of coupled
/// microstrips of zero thickness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coupled {
    /// The even-mode impedance in ohm.
    pub ze_ohm: f64,
    /// The odd-mode impedance in ohm.
    pub zo_ohm: f64,
    /// The even-mode effective permittivity.
    pub eps_even: f64,
    /// The odd-mode effective permittivity.
    pub eps_odd: f64,
}

fn single(u: f64, er: f64) -> Result<(f64, f64), RfgenError> {
    microstrip(u, 0.0, 1.0, er)
        .map_err(|e| RfgenError::new(PARAMETER_INVALID, format!("single microstrip: {e}")))
}

/// The even-mode and odd-mode capacitances per metre of the pair, with the width `u` and the gap
/// `g` in units of the substrate height, for a relative permittivity `er`.
fn pair_capacitances(u: f64, g: f64, er: f64) -> Result<(f64, f64), RfgenError> {
    let (z0, eps_eff) = single(u, er)?;
    let c_plate = EPS0 * er * u;
    let c_fringe = 0.5 * (eps_eff.sqrt() / (C0 * z0) - c_plate);
    let a = (-0.1 * (2.33 - 2.53 * u).exp()).exp();
    let c_fringe_pair = c_fringe / (1.0 + a / g * (8.0 * g).tanh());
    let k = g / (g + 2.0 * u);
    let c_air_gap = EPS0 * k_ratio(k);
    let c_fringe_odd = EPS0 * er / PI * (1.0 / (PI * g / 4.0).tanh()).ln()
        + 0.65 * c_fringe * (0.02 * er.sqrt() / g + 1.0 - 1.0 / (er * er));
    let even = c_plate + c_fringe + c_fringe_pair;
    let odd = c_plate + c_fringe + c_fringe_odd + c_air_gap;
    Ok((even, odd))
}

/// The even- and odd-mode impedances and effective permittivities of two coupled microstrips of
/// width `W = u h` separated by `s = g h` on a substrate of height `h` and permittivity `er`.
///
/// The capacitances are the closed forms of Hammerstad and Jensen, "Accurate models for microstrip
/// computer-aided design", IEEE MTT-S Symposium Digest, 1980, as collected by Garg and Bahl,
/// "Characteristics of coupled microstriplines", IEEE Trans. MTT, 1979, and by Gupta, Garg, Bahl
/// and Bhartia, "Microstrip Lines and Slotlines": the plate term `eps0 er W/h`, the fringing term
/// taken from the single line, the fringing of the pair `Cf / (1 + A (h/s) tanh(8 s/h))` with
/// `A = exp(-0.1 exp(2.33 - 2.53 W/h))` for the even mode, and for the odd mode the extra fringing
/// `(eps0 er / pi) ln coth(pi s / 4h) + 0.65 Cf (0.02 sqrt(er) h/s + 1 - 1/er^2)` and the air-gap
/// term `eps0 K(k') / K(k)` with `k = (s/h) / (s/h + 2 W/h)`. The impedance of a mode is
/// `1 / (c sqrt(C C_air))` with `C_air` the same capacitance for `er = 1`, and the effective
/// permittivity is `C / C_air`. The thickness of the copper is not in the model.
pub fn coupled(u: f64, g: f64, er: f64) -> Result<Coupled, RfgenError> {
    if !(u.is_finite() && u > 0.0 && g.is_finite() && g > 0.0 && er.is_finite() && er >= 1.0) {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            "the width, the gap and the permittivity of a coupled pair must be positive",
        ));
    }
    let (ce, co) = pair_capacitances(u, g, er)?;
    let (cea, coa) = pair_capacitances(u, g, 1.0)?;
    Ok(Coupled {
        ze_ohm: 1.0 / (C0 * (ce * cea).sqrt()),
        zo_ohm: 1.0 / (C0 * (co * coa).sqrt()),
        eps_even: ce / cea,
        eps_odd: co / coa,
    })
}

/// Where the inputs of [`coupled`] leave the range this program checks (`0.1 <= W/h <= 10`,
/// `0.1 <= s/h <= 3`, `1 <= er <= 18`). Beyond a gap of about three heights the odd-mode fit
/// stops following the physics.
pub fn coupled_range(u: f64, g: f64, er: f64) -> Vec<String> {
    let mut out = Vec::new();
    if !(0.1..=10.0).contains(&u) {
        out.push(format!("W/h = {u:.3} is outside 0.1 to 10"));
    }
    if !(0.1..=3.0).contains(&g) {
        out.push(format!("s/h = {g:.3} is outside 0.1 to 3"));
    }
    if !(1.0..=18.0).contains(&er) {
        out.push(format!("er = {er:.2} is outside 1 to 18"));
    }
    out
}

/// The result of the search for a coupled pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoupledSolution {
    /// The width over the height.
    pub u: f64,
    /// The gap over the height.
    pub g: f64,
    /// The steps of the final search.
    pub iterations: u32,
    /// The pair with that width and gap.
    pub pair: Coupled,
}

/// The smallest and the largest value of `W/h` and `s/h` that the search visits.
const SEARCH_LOW: f64 = 0.05;
const SEARCH_HIGH: f64 = 20.0;

fn residual(u: f64, g: f64, er: f64, ze: f64, zo: f64) -> Result<(f64, f64), RfgenError> {
    let pair = coupled(u, g, er)?;
    Ok((pair.ze_ohm - ze, pair.zo_ohm - zo))
}

/// Finds the width and the gap, in units of the substrate height, of a coupled pair with the
/// even- and odd-mode impedances `ze_ohm` and `zo_ohm` on a substrate of permittivity `er`.
///
/// A coarse grid in the logarithm of both unknowns (`0.1 <= W/h <= 10`, `0.1 <= s/h <= 3`) gives
/// the start, and a damped Newton iteration with a finite-difference Jacobian in the logarithms
/// refines it until both impedances are within `tolerance_ohm`. The result is deterministic.
pub fn solve_coupled(
    ze_ohm: f64,
    zo_ohm: f64,
    er: f64,
    tolerance_ohm: f64,
    max_iterations: u32,
) -> Result<CoupledSolution, RfgenError> {
    let fail = |what: &str| Err(RfgenError::new(SOLVER_FAILED, what.to_string()));
    if !(ze_ohm.is_finite() && zo_ohm.is_finite() && zo_ohm > 0.0 && ze_ohm > zo_ohm) {
        return fail("the even-mode impedance must be above the odd-mode impedance, above zero");
    }
    let grid: u32 = 25;
    let (lu0, lu1) = (0.1_f64.ln(), 10.0_f64.ln());
    let (lg0, lg1) = (0.1_f64.ln(), 3.0_f64.ln());
    let mut best: Option<(f64, f64, f64)> = None;
    for i in 0..grid {
        for j in 0..grid {
            let lu = lu0 + (lu1 - lu0) * f64::from(i) / f64::from(grid - 1);
            let lg = lg0 + (lg1 - lg0) * f64::from(j) / f64::from(grid - 1);
            let (da, db) = residual(lu.exp(), lg.exp(), er, ze_ohm, zo_ohm)?;
            let error = da * da + db * db;
            if best.is_none_or(|b| error < b.0) {
                best = Some((error, lu, lg));
            }
        }
    }
    let Some((_, mut lu, mut lg)) = best else {
        return fail("the grid is empty");
    };
    let (low, high) = (SEARCH_LOW.ln(), SEARCH_HIGH.ln());
    let mut r = residual(lu.exp(), lg.exp(), er, ze_ohm, zo_ohm)?;
    for step in 0..max_iterations {
        let norm = r.0.hypot(r.1);
        if norm < tolerance_ohm {
            let (u, g) = (lu.exp(), lg.exp());
            return Ok(CoupledSolution {
                u,
                g,
                iterations: step,
                pair: coupled(u, g, er)?,
            });
        }
        let h = 1.0e-6;
        let ra = residual((lu + h).exp(), lg.exp(), er, ze_ohm, zo_ohm)?;
        let rb = residual(lu.exp(), (lg + h).exp(), er, ze_ohm, zo_ohm)?;
        let (a, c) = ((ra.0 - r.0) / h, (ra.1 - r.1) / h);
        let (b, d) = ((rb.0 - r.0) / h, (rb.1 - r.1) / h);
        let det = a * d - b * c;
        if det.abs() < 1.0e-12 {
            return fail("the Jacobian is singular");
        }
        let du = (-r.0 * d + r.1 * b) / det;
        let dg = (-a * r.1 + c * r.0) / det;
        let mut t = 1.0_f64;
        let mut moved = None;
        while t > 1.0e-4 {
            let nlu = (lu + t * du).clamp(low, high);
            let nlg = (lg + t * dg).clamp(low, high);
            let nr = residual(nlu.exp(), nlg.exp(), er, ze_ohm, zo_ohm)?;
            if nr.0.hypot(nr.1) < norm {
                moved = Some((nlu, nlg, nr));
                break;
            }
            t /= 2.0;
        }
        let Some((nlu, nlg, nr)) = moved else {
            return fail("no step reduces the error");
        };
        lu = nlu;
        lg = nlg;
        r = nr;
    }
    fail("the iteration limit was reached")
}

/// The products `Z0 J` of the impedance inverters of a coupled-line band-pass filter, `N + 1`
/// numbers for the `N` element values `g` of the low-pass prototype, the load value `g_load` and the
/// fractional bandwidth `delta` (Pozar, Microwave Engineering, section 8.7, the design equations of
/// the coupled-line band-pass filter; Hong and Lancaster, Microstrip Filters for RF/Microwave Applications, chapter 5):
/// `Z0 J1 = sqrt(pi delta / (2 g1))`, `Z0 Jn = (pi delta / 2) / sqrt(g(n-1) g(n))` and
/// `Z0 J(N+1) = sqrt(pi delta / (2 gN g(N+1)))`.
pub fn bandpass_inverters(g: &[f64], g_load: f64, delta: f64) -> Vec<f64> {
    let n = g.len();
    let mut out = Vec::with_capacity(n + 1);
    if n == 0 {
        return out;
    }
    out.push((PI * delta / (2.0 * g[0])).sqrt());
    for k in 1..n {
        out.push((PI * delta / 2.0) / (g[k - 1] * g[k]).sqrt());
    }
    out.push((PI * delta / (2.0 * g[n - 1] * g_load)).sqrt());
    out
}

/// The even- and odd-mode impedances in ohm of a coupled section with the inverter `z0_j = Z0 J`
/// (Pozar, section 8.7): `Z0e = Z0 (1 + Z0 J + (Z0 J)^2)` and
/// `Z0o = Z0 (1 - Z0 J + (Z0 J)^2)`.
pub fn even_odd_from_inverter(z0_ohm: f64, z0_j: f64) -> (f64, f64) {
    (
        z0_ohm * (1.0 + z0_j + z0_j * z0_j),
        z0_ohm * (1.0 - z0_j + z0_j * z0_j),
    )
}

/// One section of a stepped-impedance low-pass filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LowpassSection {
    /// True for a shunt capacitor (a wide, low-impedance line), false for a series inductor (a
    /// narrow, high-impedance line).
    pub capacitor: bool,
    /// The element value of the prototype.
    pub g: f64,
    /// The impedance of the line in ohm.
    pub impedance_ohm: f64,
    /// The electrical length of the line at the cutoff, in degrees.
    pub degrees: f64,
}

/// The sections of a stepped-impedance low-pass filter by the Richards transformation and the
/// Kuroda identities used as in Pozar, Microwave Engineering, section 8.6 and example 8.5: a
/// series inductor `L` becomes a line of impedance `Zh` and electrical length `beta l = L / Zh` at
/// the cutoff, a shunt capacitor `C` becomes a line of impedance `Zl` and `beta l = C Zl`, with
/// the prototype values scaled to `Z0` (`L = g Z0`, `C = g / Z0`). The lengths are exact at the
/// cutoff only and the approximation is good while the lines are short.
pub fn lowpass_sections(
    prototype: &Prototype,
    z0_ohm: f64,
    z_high_ohm: f64,
    z_low_ohm: f64,
) -> Vec<LowpassSection> {
    prototype
        .g
        .iter()
        .enumerate()
        .map(|(index, g)| {
            let capacitor = prototype.position(index) == eh3d_phoenix_rf::filter::Position::Shunt;
            let (impedance_ohm, radians) = if capacitor {
                (z_low_ohm, g * z_low_ohm / z0_ohm)
            } else {
                (z_high_ohm, g * z0_ohm / z_high_ohm)
            };
            LowpassSection {
                capacitor,
                g: *g,
                impedance_ohm,
                degrees: radians.to_degrees(),
            }
        })
        .collect()
}

/// The dimensions of a rectangular patch antenna.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// The width of the radiating edge in metres.
    pub width_m: f64,
    /// The length in metres.
    pub length_m: f64,
    /// The effective permittivity of the patch.
    pub eps_eff: f64,
    /// The extension of the length at each radiating edge in metres.
    pub delta_l_m: f64,
}

/// A rectangular patch by the transmission-line model (Balanis, Antenna Theory, section 14.2):
/// `W = c / (2 f) sqrt(2 / (er + 1))`, `e = (er + 1)/2 + (er - 1)/2 (1 + 12 h/W)^(-1/2)`,
/// `dL = 0.412 h (e + 0.3)(W/h + 0.264) / ((e - 0.258)(W/h + 0.8))` and
/// `L = c / (2 f sqrt(e)) - 2 dL`.
pub fn patch(freq_hz: f64, er: f64, h_m: f64) -> Result<Patch, RfgenError> {
    if !(freq_hz.is_finite() && freq_hz > 0.0 && h_m.is_finite() && h_m > 0.0 && er >= 1.0) {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            "the frequency, the height and the permittivity of a patch must be positive",
        ));
    }
    let width_m = C0 / (2.0 * freq_hz) * (2.0 / (er + 1.0)).sqrt();
    let eps_eff = (er + 1.0) / 2.0 + (er - 1.0) / 2.0 * (1.0 + 12.0 * h_m / width_m).powf(-0.5);
    let delta_l_m = open_end_extension_m(width_m, h_m, eps_eff);
    let length_m = C0 / (2.0 * freq_hz * eps_eff.sqrt()) - 2.0 * delta_l_m;
    if length_m <= 0.0 {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            "the substrate is too thick for a patch at this frequency",
        ));
    }
    Ok(Patch {
        width_m,
        length_m,
        eps_eff,
        delta_l_m,
    })
}

/// Where the inputs of [`patch`] leave the range of the source (`h / lambda0 < 0.1`, `W / h >= 1`).
pub fn patch_range(freq_hz: f64, h_m: f64, width_m: f64) -> Vec<String> {
    let mut out = Vec::new();
    let ratio = h_m * freq_hz / C0;
    if ratio >= 0.1 {
        out.push(format!("h / lambda0 = {ratio:.3} is not below 0.1"));
    }
    if width_m / h_m < 1.0 {
        out.push(format!("W/h = {:.3} is below 1", width_m / h_m));
    }
    out
}

/// The Bessel function `J0(x)` by its power series; it is used for `|x| <= 10` where the 40 terms
/// give at least eight digits.
fn bessel_j0(x: f64) -> f64 {
    let q = x * x / 4.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for m in 1..=40 {
        let m = f64::from(m);
        term *= -q / (m * m);
        sum += term;
    }
    sum
}

/// The resistance at the radiating edge of a patch in ohm, `1 / (2 (G1 + G12))` (Balanis, section
/// 14.2): `G1 = W / (120 lambda0) (1 - (k0 h)^2 / 24)` for `h << lambda0` and
/// `G12 = 1 / (120 pi^2) * integral from 0 to pi of
/// [sin(k0 W cos(t) / 2) / cos(t)]^2 J0(k0 L sin(t)) sin(t)^3 dt`, integrated here by Simpson's
/// rule with `steps` intervals (made even).
pub fn patch_edge_resistance_ohm(
    freq_hz: f64,
    h_m: f64,
    width_m: f64,
    length_m: f64,
    steps: u32,
) -> Result<f64, RfgenError> {
    let lambda0 = C0 / freq_hz;
    let k0 = 2.0 * PI / lambda0;
    let g1 = width_m / (120.0 * lambda0) * (1.0 - (k0 * h_m).powi(2) / 24.0);
    let n = (steps.max(2) + 1) & !1;
    let f = |t: f64| {
        let c = t.cos();
        let s = if c.abs() < 1.0e-9 {
            k0 * width_m / 2.0
        } else {
            (k0 * width_m / 2.0 * c).sin() / c
        };
        s * s * bessel_j0(k0 * length_m * t.sin()) * t.sin().powi(3)
    };
    let step = PI / f64::from(n);
    let mut acc = f(0.0) + f(PI);
    for i in 1..n {
        let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
        acc += weight * f(f64::from(i) * step);
    }
    let g12 = acc * step / 3.0 / (120.0 * PI * PI);
    let total = g1 + g12;
    if !(total.is_finite() && total > 0.0) {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            "the conductance of the patch is not positive",
        ));
    }
    Ok(1.0 / (2.0 * total))
}

/// The depth in metres of the inset that brings the input resistance of a patch from `edge_ohm`
/// to `target_ohm`: `Rin(y0) = Rin(0) cos^2(pi y0 / L)` (Balanis, section 14.2), so
/// `y0 = (L / pi) acos(sqrt(target / edge))`.
pub fn patch_inset_m(length_m: f64, edge_ohm: f64, target_ohm: f64) -> Result<f64, RfgenError> {
    if target_ohm > edge_ohm {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            format!(
                "the edge resistance {edge_ohm:.1} ohm is below the feed impedance {target_ohm:.1} ohm: an inset cannot raise it"
            ),
        ));
    }
    Ok(length_m / PI * (target_ohm / edge_ohm).sqrt().acos())
}

/// The length of each arm of a printed half-wave dipole in metres: a quarter of the wavelength in
/// a medium of effective permittivity `(er + 1) / 2` (the common approximation for a conductor on
/// the surface of a dielectric half space, a rule of thumb without a single source) times a
/// shortening factor from the settings for the end effects and the width of the strips.
pub fn dipole_arm_m(freq_hz: f64, er: f64, shortening: f64) -> f64 {
    shortening * C0 / freq_hz / ((er + 1.0) / 2.0).sqrt() / 4.0
}

/// The impedance of the arms of a Wilkinson divider, `sqrt(2) Z0`, and the isolation resistor,
/// `2 Z0` (Pozar, section 7.3).
pub fn wilkinson(z0_ohm: f64) -> (f64, f64) {
    (std::f64::consts::SQRT_2 * z0_ohm, 2.0 * z0_ohm)
}

/// The impedances of the series arms, `Z0 / sqrt(2)`, and of the shunt arms, `Z0`, of a
/// branch-line (quadrature hybrid) coupler (Pozar, section 7.4).
pub fn branch_line(z0_ohm: f64) -> (f64, f64) {
    (z0_ohm / std::f64::consts::SQRT_2, z0_ohm)
}

/// The spacing of the vias of a fence, as a fraction of the guided wavelength.
pub fn fence_spacing_m(lambda_g_m: f64, fraction: f64) -> f64 {
    lambda_g_m * fraction
}

/// The length in metres of an electrical length in degrees at the guided wavelength.
pub fn degrees_to_m(degrees: f64, lambda_g_m: f64) -> f64 {
    degrees / 360.0 * lambda_g_m
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rf::filter::{prototype, Approximation, Position};
    use eh3d_phoenix_rf::RfSettings;

    #[test]
    fn the_miter_of_a_line_as_wide_as_the_substrate_is_high() {
        // 52 + 65 exp(-1.35) = 68.85 per cent
        assert!((miter_fraction(1.0) - 0.688_506).abs() < 1.0e-5);
        // a very wide line tends to 52 per cent
        assert!((miter_fraction(50.0) - 0.52).abs() < 1.0e-6);
        assert!(miter_range(1.0, 4.5).is_empty());
        assert_eq!(miter_range(0.1, 30.0).len(), 2);
    }

    #[test]
    fn the_open_end_extension_of_a_line_as_wide_as_the_substrate_is_high() {
        // h = 1: 0.412 * 3.6/3.042 * 1.264/1.8
        let extension = open_end_extension_m(1.0, 1.0, 3.3);
        assert!((extension - 0.342_386).abs() < 1.0e-4, "{extension}");
        assert!(open_end_range(1.0, 4.5).is_empty());
        assert_eq!(open_end_range(0.01, 30.0).len(), 2);
    }

    #[test]
    fn the_coupled_pair_tends_to_the_single_line_when_the_gap_is_large() {
        let (z0, _) = single(1.0, 9.6).unwrap();
        let near = coupled(1.0, 3.0, 9.6).unwrap();
        assert!(near.ze_ohm > z0 && near.zo_ohm < z0 + 1.0);
        assert!((near.ze_ohm - z0).abs() < 5.0, "{near:?}");
        assert!((near.zo_ohm - z0).abs() < 2.0, "{near:?}");
        // the even mode is always the higher impedance in the range of the model
        for g in [0.1, 0.3, 1.0, 2.0] {
            let pair = coupled(1.0, g, 4.5).unwrap();
            assert!(pair.ze_ohm > pair.zo_ohm, "{g}: {pair:?}");
            assert!(pair.eps_even > pair.eps_odd, "{g}: {pair:?}");
        }
        // a tighter gap lowers the odd mode and raises the even mode
        let loose = coupled(1.0, 1.0, 4.5).unwrap();
        let tight = coupled(1.0, 0.2, 4.5).unwrap();
        assert!(tight.zo_ohm < loose.zo_ohm && tight.ze_ohm > loose.ze_ohm);
    }

    #[test]
    fn the_coupled_pair_on_alumina_matches_the_charts() {
        // W/h = 1, s/h = 1, er = 9.6: about 58 and 43 ohm in the published charts
        let pair = coupled(1.0, 1.0, 9.6).unwrap();
        assert!((pair.ze_ohm - 57.9).abs() < 1.0, "{pair:?}");
        assert!((pair.zo_ohm - 43.2).abs() < 1.0, "{pair:?}");
    }

    #[test]
    fn a_bad_coupled_input_is_refused() {
        assert!(coupled(0.0, 1.0, 4.5).is_err());
        assert!(coupled(1.0, -1.0, 4.5).is_err());
        assert!(coupled(1.0, 1.0, 0.5).is_err());
        assert!(coupled(f64::NAN, 1.0, 4.5).is_err());
        assert_eq!(coupled_range(1.0, 1.0, 4.5), Vec::<String>::new());
        assert_eq!(coupled_range(0.05, 5.0, 20.0).len(), 3);
    }

    #[test]
    fn the_search_finds_the_width_and_gap_of_a_pair_and_gives_the_impedances_back() {
        for (ze, zo, er) in [(70.61, 39.24, 4.5), (56.64, 44.77, 4.5), (60.0, 40.0, 10.0)] {
            let solution = solve_coupled(ze, zo, er, 0.001, 100).unwrap();
            assert!((solution.pair.ze_ohm - ze).abs() < 0.002, "{solution:?}");
            assert!((solution.pair.zo_ohm - zo).abs() < 0.002, "{solution:?}");
            assert!(solution.u > 0.05 && solution.u < 20.0);
            assert!(solution.g > 0.05 && solution.g < 20.0);
        }
    }

    #[test]
    fn the_search_refuses_an_impossible_pair() {
        // the odd mode above the even mode
        assert!(solve_coupled(40.0, 60.0, 4.5, 0.001, 100).is_err());
        assert!(solve_coupled(60.0, -1.0, 4.5, 0.001, 100).is_err());
        // one step is not enough
        let e = solve_coupled(70.61, 39.24, 4.5, 1.0e-12, 1).unwrap_err();
        assert_eq!(e.code, SOLVER_FAILED);
    }

    #[test]
    fn the_inverters_of_the_book_filter_of_three_resonators() {
        // Pozar example: 0.5 dB ripple, N = 3, 10 per cent bandwidth, Z0 = 50 ohm
        let g = [1.5963, 1.0967, 1.5963];
        let j = bandpass_inverters(&g, 1.0, 0.1);
        assert_eq!(j.len(), 4);
        assert!((j[0] - 0.3137).abs() < 1.0e-4, "{j:?}");
        assert!((j[1] - 0.1187).abs() < 1.0e-4, "{j:?}");
        assert!((j[2] - 0.1187).abs() < 1.0e-4, "{j:?}");
        assert!((j[3] - 0.3137).abs() < 1.0e-4, "{j:?}");
        let (ze1, zo1) = even_odd_from_inverter(50.0, j[0]);
        assert!((ze1 - 70.61).abs() < 0.01 && (zo1 - 39.24).abs() < 0.01);
        let (ze2, zo2) = even_odd_from_inverter(50.0, j[1]);
        assert!((ze2 - 56.64).abs() < 0.01 && (zo2 - 44.77).abs() < 0.01);
    }

    #[test]
    fn the_inverters_of_a_single_resonator_are_two() {
        let j = bandpass_inverters(&[1.0], 1.0, 0.1);
        assert_eq!(j.len(), 2);
        assert!(bandpass_inverters(&[], 1.0, 0.1).is_empty());
    }

    #[test]
    fn the_stepped_impedance_filter_of_the_book_has_the_published_lengths() {
        // Pozar example 8.5: maximally flat, N = 6, Zl = 20 ohm, Zh = 120 ohm, Z0 = 50 ohm
        let settings = RfSettings::default();
        let p = prototype(
            Approximation::Butterworth,
            6,
            None,
            Position::Shunt,
            &settings,
        )
        .unwrap();
        let sections = lowpass_sections(&p, 50.0, 120.0, 20.0);
        assert_eq!(sections.len(), 6);
        assert!(sections[0].capacitor && !sections[1].capacitor && sections[2].capacitor);
        assert!((sections[0].degrees - 11.86).abs() < 0.01, "{sections:?}");
        assert!((sections[1].degrees - 33.76).abs() < 0.01, "{sections:?}");
        assert!((sections[2].degrees - 44.27).abs() < 0.01, "{sections:?}");
        assert_eq!(sections[0].impedance_ohm, 20.0);
        assert_eq!(sections[1].impedance_ohm, 120.0);
    }

    #[test]
    fn the_patch_at_two_point_four_gigahertz_on_fr4() {
        // er = 4.4, h = 1.6 mm: about 38 mm by 29.4 mm
        let p = patch(2.4e9, 4.4, 1.6e-3).unwrap();
        assert!((p.width_m - 0.038_01).abs() < 5.0e-5, "{p:?}");
        assert!((p.length_m - 0.029_42).abs() < 5.0e-5, "{p:?}");
        assert!((p.eps_eff - 4.0857).abs() < 1.0e-3, "{p:?}");
        assert!((p.delta_l_m - 0.000_739).abs() < 2.0e-6, "{p:?}");
        assert!(patch_range(2.4e9, 1.6e-3, p.width_m).is_empty());
        assert_eq!(patch_range(60.0e9, 1.6e-3, 1.0e-3).len(), 2);
    }

    #[test]
    fn a_patch_on_a_substrate_that_is_far_too_thick_is_refused() {
        assert!(patch(2.4e9, 4.4, 0.0).is_err());
        assert!(patch(-1.0, 4.4, 1.0e-3).is_err());
        assert!(patch(2.4e9, 4.4, 0.1).is_err());
    }

    #[test]
    fn the_edge_resistance_and_the_inset_of_the_patch() {
        let p = patch(2.4e9, 4.4, 1.6e-3).unwrap();
        let r = patch_edge_resistance_ohm(2.4e9, 1.6e-3, p.width_m, p.length_m, 400).unwrap();
        assert!((r - 160.2).abs() < 1.0, "{r}");
        let y0 = patch_inset_m(p.length_m, r, 50.0).unwrap();
        assert!((y0 - 0.009_16).abs() < 1.0e-4, "{y0}");
        // the resistance at the inset follows the cosine law
        let back = r * (PI * y0 / p.length_m).cos().powi(2);
        assert!((back - 50.0).abs() < 1.0e-6);
        // an inset cannot raise the resistance
        assert!(patch_inset_m(0.03, 40.0, 50.0).is_err());
        // a target equal to the edge resistance needs no inset
        assert!(patch_inset_m(0.03, 50.0, 50.0).unwrap().abs() < 1.0e-12);
    }

    #[test]
    fn the_bessel_series_has_the_tabulated_values() {
        assert!((bessel_j0(0.0) - 1.0).abs() < 1.0e-15);
        // Abramowitz and Stegun, table 9.1
        assert!((bessel_j0(1.0) - 0.765_197_686_6).abs() < 1.0e-9);
        assert!((bessel_j0(2.404_825_557_7)).abs() < 1.0e-8);
    }

    #[test]
    fn the_small_formulas_of_the_dividers_and_the_dipole() {
        let (arm, resistor) = wilkinson(50.0);
        assert!((arm - 70.710_678).abs() < 1.0e-5 && resistor == 100.0);
        let (series, shunt) = branch_line(50.0);
        assert!((series - 35.355_339).abs() < 1.0e-5 && shunt == 50.0);
        assert!((degrees_to_m(90.0, 0.2) - 0.05).abs() < 1.0e-15);
        assert!((fence_spacing_m(0.1, 0.05) - 0.005).abs() < 1.0e-15);
        // in air, with no shortening, a quarter wave at 1 GHz is 74.948 mm
        let arm = dipole_arm_m(1.0e9, 1.0, 1.0);
        assert!((arm - 0.074_948).abs() < 1.0e-6, "{arm}");
    }
}
