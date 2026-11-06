// SPDX-License-Identifier: GPL-3.0-or-later
//! The closed formulas of the signal integrity estimates, each one quasi-static and each one with
//! its source. Every function is pure: the same numbers give the same result.
//!
//! Sources, quoted from memory of the books and not checked against a page of them: the
//! relation between rise time and bandwidth `BW = 0.35 / tr` and the criterion of a long line are
//! in H. Johnson and M. Graham, *High-Speed Digital Design: A Handbook of Black Magic* (1993) and in
//! S. Hall, G. Hall and J. McCall, *High-Speed Digital System Design* (2000); the crosstalk
//! coefficients of weakly coupled lines `Kb = (Cm/C + Lm/L) / 4` and `Kf = (Cm/C - Lm/L) / 2` are in
//! the same books and in C. Paul, *Analysis of Multiconductor Transmission Lines*. The mutual
//! inductance of two thin wires over a plane by the method of images and the ratio of the
//! capacitances taken from the effective permittivity are estimates made here, marked as such
//! where they are used.

/// The speed of light in vacuum, in metres per second.
pub const C0: f64 = 299_792_458.0;
/// The permeability of vacuum divided by 4 pi, in henry per metre.
pub const MU0_OVER_4PI: f64 = 1.0e-7;

/// The bandwidth that an edge of this 10 to 90 percent rise time needs, in GHz: `0.35 / tr`.
pub fn bandwidth_ghz(tr_ps: f64) -> f64 {
    0.35 / (tr_ps * 1.0e-3)
}

/// The delay of a line per millimetre from its effective permittivity, in picoseconds: the phase
/// velocity is `c / sqrt(eps_eff)`.
pub fn delay_ps_per_mm(eps_eff: f64) -> f64 {
    eps_eff.sqrt() / C0 * 1.0e9
}

/// Whether a line of this delay is electrically long for an edge of this rise time: `tr < 2 Td`.
pub fn is_long_line(tr_ps: f64, td_ps: f64) -> bool {
    tr_ps < 2.0 * td_ps
}

/// The reflection coefficient of a load on a line: `(ZL - Z0) / (ZL + Z0)`.
pub fn reflection(zl: f64, z0: f64) -> f64 {
    (zl - z0) / (zl + z0)
}

/// The cutoff of one of `stages` equal first-order low-pass stages whose product has the gain of
/// `loss_db` decibel at `freq_ghz`, in GHz; `None` when the loss is too small to matter.
pub fn lowpass_cutoff_ghz(freq_ghz: f64, loss_db: f64, stages: u32) -> Option<f64> {
    if !loss_db.is_finite() || loss_db < 0.001 || stages == 0 {
        return None;
    }
    let total = 10f64.powf(-loss_db / 20.0);
    let per_stage = total.powf(1.0 / f64::from(stages));
    let ratio = 1.0 / (per_stage * per_stage) - 1.0;
    if ratio <= 0.0 {
        return None;
    }
    Some(freq_ghz / ratio.sqrt())
}

/// The rise time that `stages` equal first-order stages of this cutoff add, in picoseconds: each
/// stage has a rise time of `0.35 / fc` and the stages add in quadrature.
pub fn lowpass_rise_ps(cutoff_ghz: f64, stages: u32) -> f64 {
    0.35 / cutoff_ghz * 1.0e3 * f64::from(stages).sqrt()
}

/// The rise time at the output of a stage that adds `added_ps` to an input of `tr_ps`, by the
/// root of the sum of the squares.
pub fn rise_after(tr_ps: f64, added_ps: f64) -> f64 {
    (tr_ps * tr_ps + added_ps * added_ps).sqrt()
}

/// What an estimate of the coupling of two lines gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coupling {
    /// The ratio of the mutual to the self inductance, limited to below one.
    pub lm_over_l: f64,
    /// The ratio of the mutual to the self capacitance.
    pub cm_over_c: f64,
    /// Whether the ratio of inductances was limited because the lines are too close for the model.
    pub limited: bool,
}

/// The inductive and capacitive coupling of two equal lines at a distance `d_m` between centres
/// and a height `h_m` over their reference plane, from the impedance `z0` and the effective
/// permittivity `eps_eff` of one line alone and the relative permittivity `er` of the dielectric.
///
/// The mutual inductance per metre of two thin wires over a plane is taken by the method of
/// images, `Lm = (mu0 / 4 pi) ln(1 + (2 h / d)^2)`, and the self inductance from the line, `L = Z0
/// sqrt(eps_eff) / c`. In a homogeneous medium (a stripline) `Cm/C = Lm/L` exactly; for a
/// microstrip the capacitive ratio is taken as `Lm/L * eps_eff / er`, an estimate made here that
/// has not been checked against a field solver.
pub fn coupling(
    h_m: f64,
    d_m: f64,
    z0: f64,
    eps_eff: f64,
    er: f64,
    microstrip: bool,
    limit: f64,
) -> Coupling {
    let x = 2.0 * h_m / d_m;
    let lm = MU0_OVER_4PI * (1.0 + x * x).ln();
    let l = z0 * eps_eff.sqrt() / C0;
    let raw = lm / l;
    let limited = raw > limit;
    let lm_over_l = raw.clamp(0.0, 0.95);
    let eps_ratio = if microstrip { eps_eff / er } else { 1.0 };
    Coupling {
        lm_over_l,
        cm_over_c: lm_over_l * eps_ratio,
        limited,
    }
}

/// The crosstalk of a coupled stretch for an aggressor edge of rise time `tr_ps`, in percent of
/// the step of the aggressor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crosstalk {
    /// The near-end (backward) crosstalk, always positive.
    pub next_pct: f64,
    /// The far-end (forward) crosstalk; negative when the inductive coupling wins.
    pub fext_pct: f64,
}

/// The near-end and far-end crosstalk of a stretch of delay `td_ps` for an edge of rise time
/// `tr_ps`: `Kb = (Cm/C + Lm/L) / 4` saturating once the stretch is longer than half the rise
/// length (`2 Td >= tr`), and `Kf = (Cm/C - Lm/L) / 2` times `Td / tr`.
pub fn crosstalk(c: &Coupling, td_ps: f64, tr_ps: f64) -> Crosstalk {
    let kb = 0.25 * (c.cm_over_c + c.lm_over_l);
    let kf = 0.5 * (c.cm_over_c - c.lm_over_l);
    let fill = (2.0 * td_ps / tr_ps).min(1.0);
    Crosstalk {
        next_pct: 100.0 * kb * fill,
        fext_pct: 100.0 * kf * td_ps / tr_ps,
    }
}

/// The skin resistance of copper at a frequency, in ohm per square: `sqrt(pi f mu0 / sigma)` with
/// the conductivity of annealed copper.
pub fn copper_surface_resistance(freq_hz: f64) -> f64 {
    let sigma = 5.8e7;
    let mu0 = 4.0e-7 * std::f64::consts::PI;
    (std::f64::consts::PI * freq_hz * mu0 / sigma).sqrt()
}

/// The loss of a line when the stack-up gives none, in decibel per centimetre: the dielectric part
/// `alpha_d = pi f sqrt(eps_eff) tan_d / c` and the conductor part `alpha_c = Rs / (Z0 w)`, both in
/// neper per metre (D. Pozar, *Microwave Engineering*, microstrip line); an estimate.
pub fn fallback_loss_db_per_cm(
    freq_ghz: f64,
    eps_eff: f64,
    tan_d: f64,
    z0: f64,
    width_m: f64,
) -> f64 {
    let f = freq_ghz * 1.0e9;
    let alpha_d = std::f64::consts::PI * f * eps_eff.sqrt() * tan_d / C0;
    let alpha_c = copper_surface_resistance(f) / (z0 * width_m.max(1.0e-6));
    // 8.685889638 dB per neper, per metre to per centimetre
    (alpha_d + alpha_c) * 8.685_889_638 / 100.0
}

/// The decibel of an amplitude ratio.
pub fn db_of_ratio(ratio: f64) -> f64 {
    20.0 * ratio.log10()
}

/// A number rounded to a count of decimals, so that the reports are the same on every machine.
pub fn round_to(value: f64, decimals: i32) -> f64 {
    let scale = 10f64.powi(decimals);
    let rounded = (value * scale).round() / scale;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bandwidth_of_an_edge_is_035_over_the_rise_time() {
        assert!((bandwidth_ghz(350.0) - 1.0).abs() < 1e-12);
        assert!((bandwidth_ghz(1000.0) - 0.35).abs() < 1e-12);
    }

    #[test]
    fn the_delay_per_millimetre_follows_the_speed_of_light_in_the_medium() {
        // eps_eff = 4 halves the speed: 2 / c = 6.6713 ps/mm
        assert!((delay_ps_per_mm(4.0) - 6.671_281_903_963_041).abs() < 1e-12);
        assert!((delay_ps_per_mm(1.0) - 3.335_640_951_981_52).abs() < 1e-12);
    }

    #[test]
    fn a_line_is_long_when_the_rise_time_is_under_twice_the_delay() {
        assert!(is_long_line(500.0, 300.0));
        assert!(!is_long_line(500.0, 250.0));
        assert!(!is_long_line(500.0, 100.0));
    }

    #[test]
    fn the_reflection_of_a_load_follows_its_definition() {
        assert!((reflection(100.0, 50.0) - 1.0 / 3.0).abs() < 1e-12);
        assert!((reflection(0.0, 50.0) + 1.0).abs() < 1e-12);
        assert!(reflection(50.0, 50.0).abs() < 1e-12);
    }

    #[test]
    fn a_lowpass_that_loses_3_db_at_its_cutoff_has_that_cutoff() {
        // 10 log10(2) = 3.0103 dB is the loss of one stage at its cutoff
        let loss = 10.0 * 2f64.log10();
        let fc = lowpass_cutoff_ghz(2.0, loss, 1).unwrap();
        assert!((fc - 2.0).abs() < 1e-9, "{fc}");
        // two stages share the loss: each one loses half of it, so the cutoff is higher
        let fc2 = lowpass_cutoff_ghz(2.0, loss, 2).unwrap();
        assert!(fc2 > fc);
        assert_eq!(lowpass_cutoff_ghz(2.0, 0.0, 1), None);
        assert_eq!(lowpass_cutoff_ghz(2.0, 3.0, 0), None);
    }

    #[test]
    fn the_rise_time_of_a_single_pole_is_035_over_its_cutoff() {
        // fc = 1 GHz gives 350 ps
        assert!((lowpass_rise_ps(1.0, 1) - 350.0).abs() < 1e-9);
        assert!((lowpass_rise_ps(1.0, 2) - 350.0 * 2f64.sqrt()).abs() < 1e-9);
        // 300 ps and 400 ps add to 500 ps
        assert!((rise_after(300.0, 400.0) - 500.0).abs() < 1e-9);
    }

    #[test]
    fn the_inductive_coupling_of_two_wires_follows_the_images() {
        // h = 0.2 mm, d = 0.4 mm gives ln 2 / 1e7; Z0 = 50 ohm, eps_eff = 3
        let c = coupling(0.2e-3, 0.4e-3, 50.0, 3.0, 4.3, true, 0.3);
        let lm = 1.0e-7 * 2f64.ln();
        let l = 50.0 * 3f64.sqrt() / C0;
        assert!((c.lm_over_l - lm / l).abs() < 1e-9);
        assert!((c.cm_over_c - c.lm_over_l * 3.0 / 4.3).abs() < 1e-12);
        assert!(!c.limited);
        // a stripline is homogeneous: the two ratios are the same
        let s = coupling(0.2e-3, 0.4e-3, 50.0, 4.3, 4.3, false, 0.3);
        assert!((s.cm_over_c - s.lm_over_l).abs() < 1e-12);
        // wires on top of each other are limited
        let close = coupling(0.2e-3, 0.01e-3, 50.0, 3.0, 4.3, true, 0.3);
        assert!(close.limited);
        assert!(close.lm_over_l <= 0.95);
    }

    #[test]
    fn the_crosstalk_of_a_homogeneous_medium_has_no_far_end_part() {
        let s = coupling(0.2e-3, 0.4e-3, 50.0, 4.3, 4.3, false, 0.3);
        let x = crosstalk(&s, 500.0, 500.0);
        assert!(x.fext_pct.abs() < 1e-9);
        // a long stretch saturates at Kb = Lm/L / 2 for Cm/C = Lm/L
        assert!((x.next_pct - 100.0 * 0.5 * s.lm_over_l).abs() < 1e-9);
        // a short stretch gives less
        let short = crosstalk(&s, 50.0, 500.0);
        assert!((short.next_pct - 0.2 * x.next_pct).abs() < 1e-9);
    }

    #[test]
    fn the_far_end_crosstalk_of_a_microstrip_is_negative_and_grows_with_the_length() {
        let c = coupling(0.2e-3, 0.4e-3, 50.0, 3.0, 4.3, true, 0.3);
        let a = crosstalk(&c, 100.0, 500.0);
        let b = crosstalk(&c, 200.0, 500.0);
        assert!(a.fext_pct < 0.0);
        assert!((b.fext_pct - 2.0 * a.fext_pct).abs() < 1e-9);
    }

    #[test]
    fn the_fallback_loss_grows_with_the_frequency() {
        let a = fallback_loss_db_per_cm(1.0, 3.0, 0.02, 50.0, 0.2e-3);
        let b = fallback_loss_db_per_cm(4.0, 3.0, 0.02, 50.0, 0.2e-3);
        assert!(a > 0.0 && b > 2.0 * a);
        assert!(copper_surface_resistance(1.0e9) > copper_surface_resistance(1.0e8));
    }

    #[test]
    fn rounding_keeps_a_fixed_number_of_decimals_and_no_negative_zero() {
        assert_eq!(round_to(1.23456, 2), 1.23);
        assert_eq!(round_to(-0.0001, 2).to_bits(), 0.0f64.to_bits());
        assert!((db_of_ratio(0.5) + 6.0206).abs() < 1e-3);
    }
}
