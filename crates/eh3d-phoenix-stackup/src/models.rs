// SPDX-License-Identifier: GPL-3.0-or-later
//! The closed-form models of the transmission lines, in metres, and the sources they come from.
//!
//! All the formulas are quasi-static: they give the impedance and the effective permittivity at
//! low frequency, where the field is the static one. Each function documents its source and the
//! range for which that source publishes it; a [`Section`] gathers the checks of the range in
//! [`Section::range_warnings`]. The conductor and the dielectric are taken one at a time:
//!
//! * microstrip: E. Hammerstad and O. Jensen, "Accurate models for microstrip computer-aided
//!   design", IEEE MTT-S International Microwave Symposium Digest, 1980 (impedance and
//!   permittivity as functions of `u = W/h`, with the thickness correction of the effective width
//!   and of the permittivity given in K. C. Gupta, R. Garg, I. Bahl and P. Bhartia, "Microstrip
//!   Lines and Slotlines", 2nd edition, Artech House, 1996);
//! * stripline: S. B. Cohn, "Characteristic impedance of the shielded-strip transmission line",
//!   IRE Trans. MTT, 1954, and "Problems in strip transmission lines", IRE Trans. MTT, 1955 (the
//!   finite-thickness fringing capacitance); the narrow-strip form is the round-wire equivalent
//!   as collected in B. C. Wadell, "Transmission Line Design Handbook", Artech House, 1991;
//!   IPC-2141 gives the same family of formulas for printed boards;
//! * coplanar waveguide with and without a ground plane: R. N. Simons, "Coplanar Waveguide
//!   Circuits, Components, and Systems", Wiley, 2001 (conformal mapping), with the thickness
//!   correction of the strip and the slot of Gupta et al.;
//! * edge-coupled pair: the fits of IPC-2141 for the differential impedance of two coupled lines
//!   in terms of the single-ended impedance and the spacing.
//!
//! None of these models has been compared here with a field solver or with a measurement.

use std::f64::consts::PI;

use crate::elliptic::{k_over_complement, k_ratio};
use crate::error::{StackupError, Warning, WARN_APPROXIMATE, WARN_RANGE};

/// The speed of light in vacuum, in m/s (exact by definition of the metre).
pub const C0: f64 = 299_792_458.0;
/// The permeability of vacuum, in H/m (CODATA 2018 value).
pub const MU0: f64 = 1.256_637_062_12e-6;
/// The impedance of free space, `mu0 * c0`, in ohm.
pub const ETA0: f64 = MU0 * C0;

/// Microstrip validity of the Hammerstad-Jensen permittivity: `u = W/h` from 0.01 to 100.
pub const MICROSTRIP_U_RANGE: (f64, f64) = (0.01, 100.0);
/// Microstrip validity of the Hammerstad-Jensen model: relative permittivity from 1 to 128.
pub const MICROSTRIP_ER_RANGE: (f64, f64) = (1.0, 128.0);
/// Above this `t/h` the thickness corrections of the microstrip are not trusted (a limit chosen
/// by this implementation, not a published bound).
pub const MICROSTRIP_MAX_T_OVER_H: f64 = 0.2;
/// Stripline validity of the IPC-2141 formulas: `t/b` below 0.25.
pub const STRIPLINE_MAX_T_OVER_B: f64 = 0.25;
/// Below this `W/(b-t)` the stripline uses the narrow-strip form, above the upper limit the
/// finite-thickness Cohn form; in between the two are blended linearly so that the impedance has
/// no step. The boundary 0.35 is the one of IPC-2141 and Wadell; the width of the band is a
/// choice of this implementation.
pub const STRIPLINE_BLEND: (f64, f64) = (0.30, 0.40);
/// Above this `t / min(W, G)` the coplanar thickness correction is not trusted (a limit chosen by
/// this implementation, not a published bound).
pub const COPLANAR_MAX_T_OVER_MIN: f64 = 0.1;
/// The window of `s/h` or `s/b` in which the differential fits are used without a warning (a
/// window chosen by this implementation, not a published bound).
pub const DIFFERENTIAL_S_RANGE: (f64, f64) = (0.1, 3.0);

fn positive(value: f64, what: &str) -> Result<f64, StackupError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(StackupError::OutsideDomain {
            reason: format!("{what} is not a positive number"),
        })
    }
}

/// The effective width of a microstrip of thickness `t` (Gupta et al.): the strip is wider than
/// drawn because of the field at its edges.
fn microstrip_effective_width(w: f64, t: f64, h: f64) -> f64 {
    if t <= 0.0 {
        return w;
    }
    let extra = if w / h <= 1.0 / (2.0 * PI) {
        (t / PI) * (1.0 + (4.0 * PI * w / t).ln())
    } else {
        (t / PI) * (1.0 + (2.0 * h / t).ln())
    };
    w + extra.max(0.0)
}

/// The impedance and the effective permittivity of a microstrip of width `w`, thickness `t` on a
/// substrate of height `h` and relative permittivity `er`, all in metres.
///
/// Hammerstad and Jensen (1980), with `u = W/h`:
/// `F = 6 + (2 pi - 6) exp(-(30.666/u)^0.7528)`,
/// `Z01 = (eta0 / 2 pi) ln(F/u + sqrt(1 + (2/u)^2))`,
/// `eps_eff = (er+1)/2 + (er-1)/2 (1 + 10/u)^(-a b)` with
/// `a = 1 + ln((u^4 + (u/52)^2) / (u^4 + 0.432))/49 + ln(1 + (u/18.1)^3)/18.7` and
/// `b = 0.564 ((er - 0.9)/(er + 3))^0.053`, and `Z0 = Z01 / sqrt(eps_eff)`. The published
/// accuracy is 0.2 % for the permittivity with `0.01 <= u <= 100` and `1 <= er <= 128`, and 0.01 %
/// for `Z01` up to `u = 1` (0.03 % up to `u = 1000`). The thickness enters as an effective width
/// in `u` and as the correction `-(er-1)/4.6 (t/h)/sqrt(W/h)` of the permittivity (Gupta et al.).
pub fn microstrip(w: f64, t: f64, h: f64, er: f64) -> Result<(f64, f64), StackupError> {
    positive(w, "the width")?;
    positive(h, "the height")?;
    if !t.is_finite() || t < 0.0 {
        return Err(StackupError::OutsideDomain {
            reason: "the thickness is not a number of at least zero".to_string(),
        });
    }
    if !er.is_finite() || er < 1.0 {
        return Err(StackupError::OutsideDomain {
            reason: "the relative permittivity is below one".to_string(),
        });
    }
    let u = microstrip_effective_width(w, t, h) / h;
    let f = 6.0 + (2.0 * PI - 6.0) * (-(30.666 / u).powf(0.7528)).exp();
    let z01 = ETA0 / (2.0 * PI) * (f / u + (1.0 + (2.0 / u).powi(2)).sqrt()).ln();
    let a = 1.0
        + ((u.powi(4) + (u / 52.0).powi(2)) / (u.powi(4) + 0.432)).ln() / 49.0
        + (1.0 + (u / 18.1).powi(3)).ln() / 18.7;
    let b = 0.564 * ((er - 0.9) / (er + 3.0)).powf(0.053);
    let mut eps = (er + 1.0) / 2.0 + (er - 1.0) / 2.0 * (1.0 + 10.0 / u).powf(-a * b);
    if t > 0.0 {
        eps -= (er - 1.0) / 4.6 * (t / h) / (w / h).sqrt();
    }
    let eps = positive(eps.max(1.0), "the effective permittivity")?;
    let z0 = positive(z01 / eps.sqrt(), "the impedance")?;
    Ok((z0, eps))
}

/// The impedance of a centred stripline of zero thickness, exact (Cohn 1954):
/// `Z0 = (eta0 / (4 sqrt(er))) K(k) / K(k')` with `k = sech(pi W / 2b)`. It is used as a
/// reference for the closed forms below.
pub fn stripline_zero_thickness_exact(w: f64, b: f64, er: f64) -> Result<f64, StackupError> {
    positive(w, "the width")?;
    positive(b, "the plane spacing")?;
    positive(er, "the relative permittivity")?;
    let k = 1.0 / (PI * w / (2.0 * b)).cosh();
    positive(ETA0 / (4.0 * er.sqrt()) / k_ratio(k), "the impedance")
}

/// Cohn's finite-thickness form for a wide strip, `W/(b-t)` of about 0.35 and more:
/// `Z0 = (eta0/4) / (sqrt(er) (W/(b-t) + Cf))` with the fringing term
/// `Cf = (1/pi) (2 ln(1/(1 - t/b) + 1) - (t/b) ln(1/(1 - t/b)^2 - 1))` (for `t = 0`,
/// `Cf = 2 ln 2 / pi`). It is `94.15 / (sqrt(er) (W/(b-t) + Cf))` in the usual way of writing it.
fn stripline_wide(w: f64, t: f64, b: f64, er: f64) -> f64 {
    let cf = if t > 0.0 {
        let r = t / b;
        (2.0 * (1.0 / (1.0 - r) + 1.0).ln() - r * (1.0 / ((1.0 - r) * (1.0 - r)) - 1.0).ln()) / PI
    } else {
        2.0 * 2.0_f64.ln() / PI
    };
    ETA0 / 4.0 / er.sqrt() / (w / (b - t) + cf)
}

/// The form for a narrow strip, `W/(b-t)` of about 0.35 and less: the strip is replaced by a
/// round wire of diameter `d` centred between the planes, `Z0 = (eta0 / (2 pi sqrt(er)))
/// ln(4b / (pi d))`, with `d = (W/2) (1 + (t/(pi W)) (1 + ln(4 pi W/t) + 0.51 pi (t/W)^2))` (for
/// `t = 0`, `d = W/2`). Wadell collects it from Cohn; IPC-2141 has a similar one.
fn stripline_narrow(w: f64, t: f64, b: f64, er: f64) -> f64 {
    let d = if t > 0.0 {
        w / 2.0
            * (1.0 + t / (PI * w) * (1.0 + (4.0 * PI * w / t).ln() + 0.51 * PI * (t / w).powi(2)))
    } else {
        w / 2.0
    };
    ETA0 / (2.0 * PI) / er.sqrt() * (4.0 * b / (PI * d)).ln()
}

/// The impedance of a centred stripline of width `w` and thickness `t` between planes `b` apart
/// in a dielectric of relative permittivity `er`, in metres.
///
/// Below `W/(b-t) = 0.30` the narrow-strip form is used, above 0.40 the finite-thickness Cohn
/// form, and in between the two are blended linearly (see [`STRIPLINE_BLEND`]). Cohn's wide form
/// is within about 1 % of the exact zero-thickness value at the boundary and so is the round-wire
/// form; the blend is a choice of this implementation that keeps the result free of steps.
pub fn stripline_symmetric(w: f64, t: f64, b: f64, er: f64) -> Result<f64, StackupError> {
    positive(w, "the width")?;
    positive(b, "the plane spacing")?;
    positive(er, "the relative permittivity")?;
    if !t.is_finite() || !(0.0..b).contains(&t) {
        return Err(StackupError::OutsideDomain {
            reason: "the thickness must be at least zero and below the plane spacing".to_string(),
        });
    }
    let x = w / (b - t);
    let (low, high) = STRIPLINE_BLEND;
    let z = if x >= high {
        stripline_wide(w, t, b, er)
    } else if x <= low {
        stripline_narrow(w, t, b, er)
    } else {
        let weight = (x - low) / (high - low);
        (1.0 - weight) * stripline_narrow(w, t, b, er) + weight * stripline_wide(w, t, b, er)
    };
    positive(z, "the impedance")
}

/// The impedance, the effective permittivity and `sum(q_i er_i tan_i)` of a stripline whose
/// strip is a distance `h1` from one plane and `h2` from the other, with a dielectric of
/// permittivity `er1` and loss tangent `tan1` on the first side and `er2`, `tan2` on the second.
///
/// The strip is taken as two half structures, each a centred stripline of plane spacing
/// `2 h_i + t` with the permittivity of its side, whose capacitances add. With the same
/// distances and the same dielectric the result is exactly the centred stripline. For unequal
/// distances this is an approximation: it is exact for the parallel-plate part of the field and
/// not for the fringing, and its error has not been measured here.
pub fn stripline_offset(
    w: f64,
    t: f64,
    sides: [(f64, f64, f64); 2],
) -> Result<(f64, f64, f64), StackupError> {
    let mut air_sum = 0.0;
    let mut eps_sum = 0.0;
    let mut tan_sum = 0.0;
    for (h, er, tan_d) in sides {
        positive(h, "the distance to the plane")?;
        let z_air = stripline_symmetric(w, t, 2.0 * h + t, 1.0)?;
        let weight = 1.0 / (2.0 * z_air);
        air_sum += weight;
        eps_sum += weight * er;
        tan_sum += weight * er * tan_d;
    }
    let z0 = positive(1.0 / (air_sum * eps_sum).sqrt(), "the impedance")?;
    Ok((z0, eps_sum / air_sum, tan_sum / air_sum))
}

/// The impedance and the effective permittivity of a coplanar waveguide: a strip of width `w`
/// with a slot of width `g` on each side and then ground, thickness `t`, on a substrate of height
/// `h` and permittivity `er` with air above. With `backed` the substrate has a ground plane on
/// its other face (grounded coplanar waveguide), without it the substrate is open to the air.
///
/// Simons (2001), with `a = W/2`, `b = a + G`, `k = a/b` and `K'(k) = K(sqrt(1 - k^2))`:
/// without the plane `k1 = sinh(pi a/2h)/sinh(pi b/2h)`,
/// `eps_eff = 1 + (er-1)/2 (K'(k)/K(k)) (K(k1)/K'(k1))`, `Z0 = (eta0/4) / sqrt(eps_eff) K'(k)/K(k)`;
/// with the plane `k3 = tanh(pi a/2h)/tanh(pi b/2h)`,
/// `eps_eff = (1 + er (K'(k)/K(k)) (K(k3)/K'(k3))) / (1 + (K'(k)/K(k)) (K(k3)/K'(k3)))`,
/// `Z0 = (eta0/2) / (sqrt(eps_eff) (K(k)/K'(k) + K(k3)/K'(k3)))`.
/// The formulas are for a strip of zero thickness; the thickness widens the strip and narrows
/// the slots by `delta = (1.25 t/pi)(1 + ln(4 pi W/t))` (Gupta et al.). The permittivity is not
/// corrected for the thickness.
pub fn coplanar(
    w: f64,
    g: f64,
    t: f64,
    h: f64,
    er: f64,
    backed: bool,
) -> Result<(f64, f64), StackupError> {
    positive(w, "the strip width")?;
    positive(g, "the slot width")?;
    positive(h, "the substrate height")?;
    positive(er, "the relative permittivity")?;
    if !t.is_finite() || t < 0.0 {
        return Err(StackupError::OutsideDomain {
            reason: "the thickness is not a number of at least zero".to_string(),
        });
    }
    let delta = if t > 0.0 {
        ((1.25 * t / PI) * (1.0 + (4.0 * PI * w / t).ln())).max(0.0)
    } else {
        0.0
    };
    if delta >= g {
        return Err(StackupError::OutsideDomain {
            reason: "the copper thickness closes the slot".to_string(),
        });
    }
    let a = (w + delta) / 2.0;
    let b = a + (g - delta);
    let k = a / b;
    let q = k_ratio(k);
    let x = PI * a / (2.0 * h);
    let y = PI * b / (2.0 * h);
    let (z0, eps) = if backed {
        // k3 = tanh(x) / tanh(y); its complement is found without the loss of precision of
        // 1 - k3^2 when both arguments are large and the two tangents are close to one
        let k3 = (x.tanh() / y.tanh()).min(1.0);
        let k3c_raw = if k3 < 0.999_999 {
            (1.0 - k3 * k3).sqrt()
        } else {
            let sech_x = 1.0 / x.cosh().powi(2);
            let sech_y = 1.0 / y.cosh().powi(2);
            (sech_x - sech_y).max(0.0).sqrt() / y.tanh()
        };
        let k3c = k3c_raw.max(1.0e-150);
        let r3 = k_over_complement(k3, k3c);
        let eps = (1.0 + er * q * r3) / (1.0 + q * r3);
        (ETA0 / 2.0 / eps.sqrt() / (1.0 / q + r3), eps)
    } else {
        // k1 = sinh(x) / sinh(y), written with exponentials so that it does not overflow
        let k1 = (x - y).exp() * (-(-2.0 * x).exp_m1()) / (-(-2.0 * y).exp_m1());
        let k1c = (1.0 - k1 * k1).max(0.0).sqrt().max(1.0e-150);
        let eps = 1.0 + (er - 1.0) / 2.0 * q * k_over_complement(k1, k1c);
        (ETA0 / 4.0 / eps.sqrt() * q, eps)
    };
    let eps = positive(eps, "the effective permittivity")?;
    Ok((positive(z0, "the impedance")?, eps))
}

/// The differential impedance of an edge-coupled pair of microstrips, IPC-2141:
/// `Zdiff = 2 Z0 (1 - 0.48 exp(-0.96 s/h))` where `Z0` is the impedance of one track alone, `s`
/// the spacing between the edges and `h` the height of the substrate.
pub fn differential_microstrip(z_single: f64, s: f64, h: f64) -> f64 {
    2.0 * z_single * (1.0 - 0.48 * (-0.96 * s / h).exp())
}

/// The differential impedance of an edge-coupled pair of striplines, IPC-2141:
/// `Zdiff = 2 Z0 (1 - 0.347 exp(-2.9 s/b))` where `b` is the spacing of the planes.
pub fn differential_stripline(z_single: f64, s: f64, b: f64) -> f64 {
    2.0 * z_single * (1.0 - 0.347 * (-2.9 * s / b).exp())
}

/// The share of the field that is in the dielectric of a microstrip, `(eps_eff - 1)/(er - 1)`.
fn filling(eps_eff: f64, er: f64) -> f64 {
    if er > 1.0 + 1.0e-9 {
        ((eps_eff - 1.0) / (er - 1.0)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The result of the quasi-static model of one cross section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    /// The impedance the line is defined by: `Z0` of one track, or `Zdiff` of a pair, in ohm.
    pub impedance_ohm: f64,
    /// The impedance of one track alone, in ohm (the same as `impedance_ohm` for one track).
    pub z_single_ohm: f64,
    /// The effective relative permittivity.
    pub eps_eff: f64,
    /// `sum(q_i er_i tan_i)` over the dielectrics, where `q_i` is the share of the field of the
    /// dielectric `i`. The dielectric loss is `(k0 / 2) * eps_tan / sqrt(eps_eff)` in Np/m.
    pub eps_tan: f64,
}

/// The cross section of a line, in metres, with the dielectric constant and loss tangent of each
/// side already taken at the frequency.
#[derive(Clone, Debug, PartialEq)]
pub enum Section {
    /// A microstrip: air above, the substrate below.
    Microstrip {
        /// Width of the track.
        w_m: f64,
        /// Thickness of the track.
        t_m: f64,
        /// Height of the substrate.
        h_m: f64,
        /// Relative permittivity of the substrate.
        er: f64,
        /// Loss tangent of the substrate.
        tan_d: f64,
    },
    /// A stripline between two planes, centred or offset.
    Stripline {
        /// Width of the track.
        w_m: f64,
        /// Thickness of the track.
        t_m: f64,
        /// Distance from the track to the first plane.
        h1_m: f64,
        /// Distance from the track to the second plane.
        h2_m: f64,
        /// Relative permittivity on the first side.
        er1: f64,
        /// Relative permittivity on the second side.
        er2: f64,
        /// Loss tangent on the first side.
        tan_d1: f64,
        /// Loss tangent on the second side.
        tan_d2: f64,
    },
    /// A coplanar waveguide, with or without a ground plane under the substrate.
    Coplanar {
        /// Width of the strip.
        w_m: f64,
        /// Width of each slot.
        g_m: f64,
        /// Thickness of the copper.
        t_m: f64,
        /// Height of the substrate.
        h_m: f64,
        /// Relative permittivity of the substrate.
        er: f64,
        /// Loss tangent of the substrate.
        tan_d: f64,
        /// Whether a ground plane lies under the substrate.
        backed: bool,
    },
    /// An edge-coupled pair of microstrips.
    DiffMicrostrip {
        /// Width of each track.
        w_m: f64,
        /// Thickness of the tracks.
        t_m: f64,
        /// Height of the substrate.
        h_m: f64,
        /// Relative permittivity of the substrate.
        er: f64,
        /// Loss tangent of the substrate.
        tan_d: f64,
        /// Spacing between the edges of the two tracks.
        s_m: f64,
    },
    /// An edge-coupled pair of striplines.
    DiffStripline {
        /// Width of each track.
        w_m: f64,
        /// Thickness of the tracks.
        t_m: f64,
        /// Distance from the tracks to the first plane.
        h1_m: f64,
        /// Distance from the tracks to the second plane.
        h2_m: f64,
        /// Relative permittivity on the first side.
        er1: f64,
        /// Relative permittivity on the second side.
        er2: f64,
        /// Loss tangent on the first side.
        tan_d1: f64,
        /// Loss tangent on the second side.
        tan_d2: f64,
        /// Spacing between the edges of the two tracks.
        s_m: f64,
    },
}

impl Section {
    /// Evaluate the quasi-static model.
    pub fn eval(&self) -> Result<Solution, StackupError> {
        match self {
            Section::Microstrip {
                w_m,
                t_m,
                h_m,
                er,
                tan_d,
            } => {
                let (z0, eps) = microstrip(*w_m, *t_m, *h_m, *er)?;
                Ok(Solution {
                    impedance_ohm: z0,
                    z_single_ohm: z0,
                    eps_eff: eps,
                    eps_tan: filling(eps, *er) * er * tan_d,
                })
            }
            Section::Stripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                er1,
                er2,
                tan_d1,
                tan_d2,
            } => {
                let (z0, eps, eps_tan) =
                    stripline_offset(*w_m, *t_m, [(*h1_m, *er1, *tan_d1), (*h2_m, *er2, *tan_d2)])?;
                Ok(Solution {
                    impedance_ohm: z0,
                    z_single_ohm: z0,
                    eps_eff: eps,
                    eps_tan,
                })
            }
            Section::Coplanar {
                w_m,
                g_m,
                t_m,
                h_m,
                er,
                tan_d,
                backed,
            } => {
                let (z0, eps) = coplanar(*w_m, *g_m, *t_m, *h_m, *er, *backed)?;
                Ok(Solution {
                    impedance_ohm: z0,
                    z_single_ohm: z0,
                    eps_eff: eps,
                    eps_tan: filling(eps, *er) * er * tan_d,
                })
            }
            Section::DiffMicrostrip { h_m, s_m, .. } => {
                let single = self.base().eval()?;
                let zdiff = differential_microstrip(single.z_single_ohm, *s_m, *h_m);
                Ok(Solution {
                    impedance_ohm: positive(zdiff, "the differential impedance")?,
                    ..single
                })
            }
            Section::DiffStripline {
                h1_m,
                h2_m,
                s_m,
                t_m,
                ..
            } => {
                let single = self.base().eval()?;
                let b = h1_m + h2_m + t_m;
                let zdiff = differential_stripline(single.z_single_ohm, *s_m, b);
                Ok(Solution {
                    impedance_ohm: positive(zdiff, "the differential impedance")?,
                    ..single
                })
            }
        }
    }

    /// The section of one track alone (the same section for a single line).
    pub fn base(&self) -> Section {
        match self {
            Section::DiffMicrostrip {
                w_m,
                t_m,
                h_m,
                er,
                tan_d,
                ..
            } => Section::Microstrip {
                w_m: *w_m,
                t_m: *t_m,
                h_m: *h_m,
                er: *er,
                tan_d: *tan_d,
            },
            Section::DiffStripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                er1,
                er2,
                tan_d1,
                tan_d2,
                ..
            } => Section::Stripline {
                w_m: *w_m,
                t_m: *t_m,
                h1_m: *h1_m,
                h2_m: *h2_m,
                er1: *er1,
                er2: *er2,
                tan_d1: *tan_d1,
                tan_d2: *tan_d2,
            },
            other => other.clone(),
        }
    }

    /// The same section with air in place of every dielectric (permittivity one, no loss). Its
    /// impedance is the one of the line when the inductance alone is considered.
    pub fn air(&self) -> Section {
        match self.clone() {
            Section::Microstrip { w_m, t_m, h_m, .. } => Section::Microstrip {
                w_m,
                t_m,
                h_m,
                er: 1.0,
                tan_d: 0.0,
            },
            Section::Stripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                ..
            } => Section::Stripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                er1: 1.0,
                er2: 1.0,
                tan_d1: 0.0,
                tan_d2: 0.0,
            },
            Section::Coplanar {
                w_m,
                g_m,
                t_m,
                h_m,
                backed,
                ..
            } => Section::Coplanar {
                w_m,
                g_m,
                t_m,
                h_m,
                er: 1.0,
                tan_d: 0.0,
                backed,
            },
            Section::DiffMicrostrip {
                w_m, t_m, h_m, s_m, ..
            } => Section::DiffMicrostrip {
                w_m,
                t_m,
                h_m,
                er: 1.0,
                tan_d: 0.0,
                s_m,
            },
            Section::DiffStripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                s_m,
                ..
            } => Section::DiffStripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                er1: 1.0,
                er2: 1.0,
                tan_d1: 0.0,
                tan_d2: 0.0,
                s_m,
            },
        }
    }

    /// The section with the surfaces of the conductors moved into the metal by `s` metres per
    /// gap or per width (Wheeler's incremental inductance rule): every width and thickness of
    /// copper is `s` smaller and every gap between conductors is `s` larger. Pairs are receded
    /// as one track alone.
    pub fn receded(&self, s: f64) -> Section {
        match self.base() {
            Section::Microstrip {
                w_m,
                t_m,
                h_m,
                er,
                tan_d,
            } => Section::Microstrip {
                w_m: w_m - s,
                t_m: t_m - s,
                h_m: h_m + s,
                er,
                tan_d,
            },
            Section::Stripline {
                w_m,
                t_m,
                h1_m,
                h2_m,
                er1,
                er2,
                tan_d1,
                tan_d2,
            } => Section::Stripline {
                w_m: w_m - s,
                t_m: t_m - s,
                h1_m: h1_m + s,
                h2_m: h2_m + s,
                er1,
                er2,
                tan_d1,
                tan_d2,
            },
            Section::Coplanar {
                w_m,
                g_m,
                t_m,
                h_m,
                er,
                tan_d,
                backed,
            } => Section::Coplanar {
                w_m: w_m - s,
                g_m: g_m + s,
                t_m: t_m - s,
                h_m: if backed { h_m + s } else { h_m },
                er,
                tan_d,
                backed,
            },
            other => other,
        }
    }

    /// The section with another track width.
    pub fn with_width(&self, width_m: f64) -> Section {
        let mut section = self.clone();
        match &mut section {
            Section::Microstrip { w_m, .. }
            | Section::Stripline { w_m, .. }
            | Section::Coplanar { w_m, .. }
            | Section::DiffMicrostrip { w_m, .. }
            | Section::DiffStripline { w_m, .. } => *w_m = width_m,
        }
        section
    }

    /// The width of the track (the strip of a coplanar waveguide).
    pub fn width_m(&self) -> f64 {
        match self {
            Section::Microstrip { w_m, .. }
            | Section::Stripline { w_m, .. }
            | Section::Coplanar { w_m, .. }
            | Section::DiffMicrostrip { w_m, .. }
            | Section::DiffStripline { w_m, .. } => *w_m,
        }
    }

    /// The thickness of the copper.
    pub fn thickness_m(&self) -> f64 {
        match self {
            Section::Microstrip { t_m, .. }
            | Section::Stripline { t_m, .. }
            | Section::Coplanar { t_m, .. }
            | Section::DiffMicrostrip { t_m, .. }
            | Section::DiffStripline { t_m, .. } => *t_m,
        }
    }

    /// The height of the substrate, or the spacing of the planes of a stripline.
    pub fn vertical_m(&self) -> f64 {
        match self {
            Section::Microstrip { h_m, .. }
            | Section::Coplanar { h_m, .. }
            | Section::DiffMicrostrip { h_m, .. } => *h_m,
            Section::Stripline {
                t_m, h1_m, h2_m, ..
            }
            | Section::DiffStripline {
                t_m, h1_m, h2_m, ..
            } => h1_m + h2_m + t_m,
        }
    }

    /// The smallest dimension of the section, used to choose the step of Wheeler's rule.
    pub fn smallest_m(&self) -> f64 {
        let (w, t) = (self.width_m(), self.thickness_m());
        let mut smallest = w.min(t);
        match self.base() {
            Section::Microstrip { h_m, .. } | Section::Coplanar { h_m, .. } => {
                smallest = smallest.min(h_m);
            }
            Section::Stripline { h1_m, h2_m, .. } => {
                smallest = smallest.min(h1_m).min(h2_m);
            }
            _ => {}
        }
        if let Section::Coplanar { g_m, .. } = self {
            smallest = smallest.min(*g_m);
        }
        smallest
    }

    /// The warnings for a section that is outside the range for which its formulas are
    /// published, or that is only an approximation.
    pub fn range_warnings(&self) -> Vec<Warning> {
        let mut out = Vec::new();
        let outside = |what: &str, value: f64, low: f64, high: f64| {
            Warning::new(
                WARN_RANGE,
                format!("{what} is {value:.4}, outside {low} to {high}"),
            )
        };
        match self {
            Section::Microstrip {
                w_m, t_m, h_m, er, ..
            }
            | Section::DiffMicrostrip {
                w_m, t_m, h_m, er, ..
            } => {
                let u = w_m / h_m;
                let (low, high) = MICROSTRIP_U_RANGE;
                if !(low..=high).contains(&u) {
                    out.push(outside("microstrip W/h", u, low, high));
                }
                let (low, high) = MICROSTRIP_ER_RANGE;
                if !(low..=high).contains(er) {
                    out.push(outside("microstrip permittivity", *er, low, high));
                }
                if t_m / h_m > MICROSTRIP_MAX_T_OVER_H {
                    out.push(outside(
                        "microstrip t/h",
                        t_m / h_m,
                        0.0,
                        MICROSTRIP_MAX_T_OVER_H,
                    ));
                }
            }
            Section::Stripline { t_m, .. } | Section::DiffStripline { t_m, .. } => {
                let b = self.vertical_m();
                if t_m / b > STRIPLINE_MAX_T_OVER_B {
                    out.push(outside(
                        "stripline t/b",
                        t_m / b,
                        0.0,
                        STRIPLINE_MAX_T_OVER_B,
                    ));
                }
            }
            Section::Coplanar { w_m, g_m, t_m, .. } => {
                let ratio = t_m / w_m.min(*g_m);
                if ratio > COPLANAR_MAX_T_OVER_MIN {
                    out.push(outside(
                        "coplanar t/min(W,G)",
                        ratio,
                        0.0,
                        COPLANAR_MAX_T_OVER_MIN,
                    ));
                }
            }
        }
        match self {
            Section::Stripline { h1_m, h2_m, .. } | Section::DiffStripline { h1_m, h2_m, .. }
                if (h1_m - h2_m).abs() > 1.0e-9 * (h1_m + h2_m) =>
            {
                out.push(Warning::new(
                    WARN_APPROXIMATE,
                    "the track is not centred: the offset stripline is the sum of two half structures",
                ));
            }
            _ => {}
        }
        match self {
            Section::DiffMicrostrip { h_m, s_m, .. } => {
                let ratio = s_m / h_m;
                let (low, high) = DIFFERENTIAL_S_RANGE;
                if !(low..=high).contains(&ratio) {
                    out.push(outside("pair spacing s/h", ratio, low, high));
                }
            }
            Section::DiffStripline { s_m, .. } => {
                let ratio = s_m / self.vertical_m();
                let (low, high) = DIFFERENTIAL_S_RANGE;
                if !(low..=high).contains(&ratio) {
                    out.push(outside("pair spacing s/b", ratio, low, high));
                }
            }
            _ => {}
        }
        if matches!(
            self,
            Section::DiffMicrostrip { .. } | Section::DiffStripline { .. }
        ) {
            out.push(Warning::new(
                WARN_APPROXIMATE,
                "the pair uses the IPC-2141 fit; the permittivity and the losses are those of one track alone",
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = 1.0e-3;

    fn close(a: f64, b: f64, percent: f64) -> bool {
        (a - b).abs() <= b.abs() * percent / 100.0
    }

    #[test]
    fn microstrip_in_air_at_w_equals_h_is_the_hammerstad_value() {
        // Z01 for u = 1: (eta0/2pi) ln(F + sqrt(5)) with F = 6 (the exponential is 1e-6 small)
        let expected = ETA0 / (2.0 * PI) * (6.0 + 5.0_f64.sqrt()).ln();
        let (z, eps) = microstrip(1.0 * MM, 0.0, 1.0 * MM, 1.0).unwrap();
        assert!(close(z, expected, 0.01), "{z} {expected}");
        assert!((eps - 1.0).abs() < 1e-12);
    }

    #[test]
    fn microstrip_effective_permittivity_lies_between_air_and_the_substrate() {
        for u in [0.05, 0.2, 1.0, 3.0, 10.0, 50.0] {
            for er in [2.2, 3.66, 4.5, 9.8] {
                let (_, eps) = microstrip(u * MM, 0.0, MM, er).unwrap();
                assert!(eps > 1.0 && eps < er, "{u} {er} {eps}");
            }
        }
    }

    #[test]
    fn a_wide_microstrip_approaches_the_parallel_plate_limit() {
        // for u = 100 the parallel-plate impedance is eta0 / (u sqrt(er)) = 3.77 / sqrt(er)
        let er = 4.0;
        let (z, eps) = microstrip(100.0 * MM, 0.0, MM, er).unwrap();
        let plate = ETA0 / (100.0 * er.sqrt());
        assert!(close(z, plate, 6.0), "{z} {plate}");
        assert!(eps > 0.9 * er);
    }

    #[test]
    fn microstrip_impedance_falls_as_the_track_widens() {
        let mut last = f64::INFINITY;
        for step in 1..60 {
            let w = f64::from(step) * 0.1 * MM;
            let (z, _) = microstrip(w, 35.0e-6, 1.6 * MM, 4.5).unwrap();
            assert!(z < last, "w = {w}");
            last = z;
        }
    }

    #[test]
    fn microstrip_impedance_rises_with_the_substrate_height() {
        let mut last = 0.0;
        for step in 1..20 {
            let h = f64::from(step) * 0.1 * MM;
            let (z, _) = microstrip(0.5 * MM, 35.0e-6, h, 4.0).unwrap();
            assert!(z > last, "h = {h}");
            last = z;
        }
    }

    #[test]
    fn a_thicker_track_has_a_lower_impedance() {
        let (thin, _) = microstrip(0.3 * MM, 9.0e-6, 0.2 * MM, 4.0).unwrap();
        let (thick, _) = microstrip(0.3 * MM, 70.0e-6, 0.2 * MM, 4.0).unwrap();
        assert!(thick < thin);
    }

    #[test]
    fn microstrip_refuses_what_has_no_meaning() {
        for (w, t, h, er) in [
            (0.0, 0.0, MM, 4.0),
            (-MM, 0.0, MM, 4.0),
            (MM, -1.0e-6, MM, 4.0),
            (MM, 0.0, 0.0, 4.0),
            (MM, 0.0, MM, 0.5),
            (MM, 0.0, MM, f64::NAN),
            (f64::INFINITY, 0.0, MM, 4.0),
        ] {
            assert!(
                matches!(
                    microstrip(w, t, h, er),
                    Err(StackupError::OutsideDomain { .. })
                ),
                "{w} {t} {h} {er}"
            );
        }
    }

    #[test]
    fn the_stripline_forms_agree_with_the_exact_zero_thickness_value() {
        // Cohn's exact value for a strip of zero thickness against the closed forms of this crate
        let b = 1.0 * MM;
        for x in [0.1, 0.2, 0.3, 0.4, 0.5, 0.8, 1.2, 2.0] {
            let exact = stripline_zero_thickness_exact(x * b, b, 1.0).unwrap();
            let closed = stripline_symmetric(x * b, 0.0, b, 1.0).unwrap();
            assert!(close(closed, exact, 3.0), "x = {x}: {closed} {exact}");
        }
    }

    #[test]
    fn the_exact_stripline_value_is_the_published_one_at_w_over_b_one_half() {
        // k = sech(pi/4); Z0 = 30 pi K(k)/K(k') for er = 1 is 100.5 ohm (to 0.1 ohm)
        let z = stripline_zero_thickness_exact(0.5 * MM, MM, 1.0).unwrap();
        assert!((z - 100.5).abs() < 0.5, "{z}");
    }

    #[test]
    fn stripline_scales_with_the_square_root_of_the_permittivity() {
        let air = stripline_symmetric(0.3 * MM, 35.0e-6, 1.0 * MM, 1.0).unwrap();
        let fr4 = stripline_symmetric(0.3 * MM, 35.0e-6, 1.0 * MM, 4.0).unwrap();
        assert!(close(fr4, air / 2.0, 1e-9));
    }

    #[test]
    fn stripline_impedance_falls_as_the_track_widens_with_no_step_at_the_blend() {
        let mut last = f64::INFINITY;
        for step in 20..400 {
            let w = f64::from(step) * 0.005 * MM;
            let z = stripline_symmetric(w, 35.0e-6, 1.0 * MM, 4.2).unwrap();
            assert!(z < last, "w = {w}");
            // no step: neighbouring widths differ by far less than a few ohm
            assert!(last.is_infinite() || last - z < 3.0, "w = {w}");
            last = z;
        }
    }

    #[test]
    fn a_stripline_track_that_fills_the_gap_is_refused() {
        assert!(stripline_symmetric(MM, 1.0 * MM, 1.0 * MM, 4.0).is_err());
        assert!(stripline_symmetric(MM, 2.0 * MM, 1.0 * MM, 4.0).is_err());
        assert!(stripline_symmetric(0.0, 0.0, 1.0 * MM, 4.0).is_err());
        assert!(stripline_symmetric(MM, 0.0, 1.0 * MM, 0.0).is_err());
    }

    #[test]
    fn a_centred_offset_stripline_is_the_centred_stripline() {
        let centred = stripline_symmetric(0.3 * MM, 35.0e-6, 1.0 * MM, 4.2).unwrap();
        let h = (1.0 * MM - 35.0e-6) / 2.0;
        let (z, eps, tan) =
            stripline_offset(0.3 * MM, 35.0e-6, [(h, 4.2, 0.02), (h, 4.2, 0.02)]).unwrap();
        assert!(close(z, centred, 1e-9));
        assert!(close(eps, 4.2, 1e-9));
        assert!(close(tan, 4.2 * 0.02, 1e-9));
    }

    #[test]
    fn moving_the_track_towards_a_plane_lowers_the_impedance() {
        let b = 1.0 * MM;
        let t = 35.0e-6;
        let centred_h = (b - t) / 2.0;
        let (centred, _, _) =
            stripline_offset(0.2 * MM, t, [(centred_h, 4.0, 0.0), (centred_h, 4.0, 0.0)]).unwrap();
        let mut last = centred;
        for shift in [0.05, 0.1, 0.2, 0.3] {
            let h1 = centred_h - shift * MM;
            let h2 = centred_h + shift * MM;
            let (z, _, _) =
                stripline_offset(0.2 * MM, t, [(h1, 4.0, 0.0), (h2, 4.0, 0.0)]).unwrap();
            assert!(z < last, "shift {shift}");
            last = z;
        }
    }

    #[test]
    fn two_different_dielectrics_give_a_permittivity_between_them() {
        let h = 0.3 * MM;
        let (_, eps, tan) =
            stripline_offset(0.2 * MM, 35.0e-6, [(h, 3.0, 0.002), (h, 4.5, 0.02)]).unwrap();
        assert!(eps > 3.0 && eps < 4.5);
        assert!(tan > 3.0 * 0.002 && tan < 4.5 * 0.02);
    }

    #[test]
    fn coplanar_without_a_plane_on_a_thick_substrate_is_the_published_limit() {
        // k = a/b = 1/sqrt(2) makes K'(k)/K(k) = 1, so Z0 = (eta0/4) / sqrt((er+1)/2); with the
        // substrate 1 m thick it is the infinitely thick substrate of Simons
        let a = 0.5 * MM;
        let g = (2.0_f64.sqrt() - 1.0) * a;
        let (z, eps) = coplanar(2.0 * a, g, 0.0, 1.0, 1.0, false).unwrap();
        assert!(close(eps, 1.0, 1e-6), "{eps}");
        assert!(close(z, ETA0 / 4.0, 1e-4), "{z}");
        let (z4, eps4) = coplanar(2.0 * a, g, 0.0, 1.0, 4.0, false).unwrap();
        assert!(close(eps4, 2.5, 1e-3), "{eps4}");
        assert!(close(z4, ETA0 / 4.0 / 2.5_f64.sqrt(), 1e-3), "{z4}");
    }

    #[test]
    fn a_ground_plane_lowers_the_coplanar_impedance() {
        let open = coplanar(0.3 * MM, 0.15 * MM, 35.0e-6, 1.6 * MM, 4.5, false).unwrap();
        let backed = coplanar(0.3 * MM, 0.15 * MM, 35.0e-6, 1.6 * MM, 4.5, true).unwrap();
        assert!(backed.0 < open.0);
        assert!(backed.1 > open.1);
        assert!(backed.1 < 4.5 && open.1 > 1.0);
    }

    #[test]
    fn a_grounded_coplanar_waveguide_on_a_thick_substrate_becomes_the_open_one() {
        let open = coplanar(0.3 * MM, 0.15 * MM, 0.0, 50.0 * MM, 4.5, false).unwrap();
        let backed = coplanar(0.3 * MM, 0.15 * MM, 0.0, 50.0 * MM, 4.5, true).unwrap();
        assert!(close(backed.0, open.0, 0.5), "{} {}", backed.0, open.0);
        assert!(close(backed.1, open.1, 0.5), "{} {}", backed.1, open.1);
    }

    #[test]
    fn coplanar_impedance_falls_as_the_strip_widens_and_rises_as_the_slot_widens() {
        let mut last = f64::INFINITY;
        for step in 1..30 {
            let w = f64::from(step) * 0.05 * MM;
            let (z, _) = coplanar(w, 0.2 * MM, 35.0e-6, 1.6 * MM, 4.5, true).unwrap();
            assert!(z < last, "w = {w}");
            last = z;
        }
        let mut last = 0.0;
        for step in 4..30 {
            let g = f64::from(step) * 0.05 * MM;
            let (z, _) = coplanar(0.3 * MM, g, 35.0e-6, 1.6 * MM, 4.5, true).unwrap();
            assert!(z > last, "g = {g}");
            last = z;
        }
    }

    #[test]
    fn a_very_wide_coplanar_strip_on_a_thin_substrate_stays_finite() {
        // the tangents of the large arguments are both one: the complement is found another way
        for h in [1.0e-6, 1.0e-4, 2.0e-4, 5.0e-3] {
            for backed in [true, false] {
                let (z, eps) = coplanar(20.0 * MM, 0.3 * MM, 35.0e-6, h, 4.5, backed).unwrap();
                assert!(z.is_finite() && z > 0.0, "{h} {backed}: {z}");
                assert!((1.0..=4.5).contains(&eps), "{h} {backed}: {eps}");
            }
        }
    }

    #[test]
    fn a_coplanar_slot_closed_by_the_copper_is_refused() {
        // 1 mm of copper in a 50 micrometre slot
        assert!(matches!(
            coplanar(0.3 * MM, 50.0e-6, 1.0 * MM, 1.6 * MM, 4.5, true),
            Err(StackupError::OutsideDomain { .. })
        ));
        for (w, g, h, er) in [
            (0.0, 0.1 * MM, MM, 4.0),
            (MM, 0.0, MM, 4.0),
            (MM, MM, 0.0, 4.0),
            (MM, MM, MM, 0.0),
        ] {
            assert!(coplanar(w, g, 0.0, h, er, true).is_err());
        }
    }

    #[test]
    fn the_differential_fits_tend_to_twice_the_single_impedance_when_far_apart() {
        assert!(close(differential_microstrip(50.0, 1.0, 0.1), 100.0, 0.01));
        assert!(close(differential_stripline(50.0, 1.0, 0.1), 100.0, 0.01));
        // closer tracks couple and the differential impedance falls
        let far = differential_microstrip(50.0, 2.0 * MM, MM);
        let near = differential_microstrip(50.0, 0.1 * MM, MM);
        assert!(near < far && far < 100.0);
        let far = differential_stripline(50.0, 2.0 * MM, MM);
        let near = differential_stripline(50.0, 0.1 * MM, MM);
        assert!(near < far && far < 100.0);
    }

    #[test]
    fn the_differential_fit_at_a_known_spacing() {
        // IPC-2141: 2 * 50 * (1 - 0.48 exp(-0.96)) = 100 * (1 - 0.48 * 0.38289) = 81.62
        let z = differential_microstrip(50.0, MM, MM);
        assert!((z - 81.62).abs() < 0.01, "{z}");
        // 2 * 50 * (1 - 0.347 exp(-2.9)) = 100 * (1 - 0.347 * 0.055023) = 98.09
        let z = differential_stripline(50.0, MM, MM);
        assert!((z - 98.09).abs() < 0.01, "{z}");
    }

    fn microstrip_section() -> Section {
        Section::Microstrip {
            w_m: 3.0 * MM,
            t_m: 35.0e-6,
            h_m: 1.6 * MM,
            er: 4.5,
            tan_d: 0.02,
        }
    }

    #[test]
    fn the_section_evaluates_like_the_function() {
        let s = microstrip_section().eval().unwrap();
        let (z, eps) = microstrip(3.0 * MM, 35.0e-6, 1.6 * MM, 4.5).unwrap();
        assert_eq!((s.impedance_ohm, s.eps_eff), (z, eps));
        assert!(s.eps_tan > 0.0 && s.eps_tan < 4.5 * 0.02);
        assert_eq!(s.impedance_ohm, s.z_single_ohm);
    }

    #[test]
    fn the_air_section_has_no_dielectric() {
        let air = microstrip_section().air().eval().unwrap();
        assert!(close(air.eps_eff, 1.0, 1e-9));
        assert_eq!(air.eps_tan, 0.0);
        let strip = Section::Stripline {
            w_m: 0.2 * MM,
            t_m: 35.0e-6,
            h1_m: 0.3 * MM,
            h2_m: 0.4 * MM,
            er1: 4.0,
            er2: 3.0,
            tan_d1: 0.02,
            tan_d2: 0.01,
        };
        let air = strip.air().eval().unwrap();
        assert!(close(air.eps_eff, 1.0, 1e-9));
        assert!(air.impedance_ohm > strip.eval().unwrap().impedance_ohm);
    }

    #[test]
    fn a_pair_has_the_single_track_underneath() {
        let pair = Section::DiffMicrostrip {
            w_m: 0.2 * MM,
            t_m: 35.0e-6,
            h_m: 0.2 * MM,
            er: 4.0,
            tan_d: 0.02,
            s_m: 0.2 * MM,
        };
        let solved = pair.eval().unwrap();
        let single = pair.base().eval().unwrap();
        assert_eq!(solved.z_single_ohm, single.impedance_ohm);
        assert!(solved.impedance_ohm < 2.0 * single.impedance_ohm);
        assert_eq!(solved.eps_eff, single.eps_eff);
        let strip_pair = Section::DiffStripline {
            w_m: 0.15 * MM,
            t_m: 35.0e-6,
            h1_m: 0.2 * MM,
            h2_m: 0.2 * MM,
            er1: 4.0,
            er2: 4.0,
            tan_d1: 0.02,
            tan_d2: 0.02,
            s_m: 0.15 * MM,
        };
        let solved = strip_pair.eval().unwrap();
        assert!(solved.impedance_ohm < 2.0 * solved.z_single_ohm);
    }

    #[test]
    fn receding_the_surfaces_raises_the_air_impedance() {
        for section in [
            microstrip_section(),
            Section::Stripline {
                w_m: 0.2 * MM,
                t_m: 35.0e-6,
                h1_m: 0.3 * MM,
                h2_m: 0.3 * MM,
                er1: 4.0,
                er2: 4.0,
                tan_d1: 0.0,
                tan_d2: 0.0,
            },
            Section::Coplanar {
                w_m: 0.3 * MM,
                g_m: 0.15 * MM,
                t_m: 35.0e-6,
                h_m: 1.6 * MM,
                er: 4.5,
                tan_d: 0.0,
                backed: true,
            },
            Section::Coplanar {
                w_m: 0.3 * MM,
                g_m: 0.15 * MM,
                t_m: 35.0e-6,
                h_m: 1.6 * MM,
                er: 4.5,
                tan_d: 0.0,
                backed: false,
            },
        ] {
            let air = section.air();
            let before = air.eval().unwrap().impedance_ohm;
            let after = air.receded(1.0e-6).eval().unwrap().impedance_ohm;
            assert!(after > before, "{section:?}");
        }
    }

    #[test]
    fn width_accessors_and_replacement() {
        let s = microstrip_section();
        assert_eq!(s.width_m(), 3.0 * MM);
        assert_eq!(s.thickness_m(), 35.0e-6);
        assert_eq!(s.vertical_m(), 1.6 * MM);
        assert_eq!(s.with_width(1.0 * MM).width_m(), 1.0 * MM);
        assert!(s.smallest_m() <= 35.0e-6);
        let strip = Section::Stripline {
            w_m: 0.2 * MM,
            t_m: 35.0e-6,
            h1_m: 0.3 * MM,
            h2_m: 0.4 * MM,
            er1: 4.0,
            er2: 4.0,
            tan_d1: 0.0,
            tan_d2: 0.0,
        };
        assert!((strip.vertical_m() - (0.7 * MM + 35.0e-6)).abs() < 1e-12);
    }

    #[test]
    fn warnings_name_the_range_that_is_left() {
        let narrow = Section::Microstrip {
            w_m: 0.001 * MM,
            t_m: 1.0e-7,
            h_m: 1.0 * MM,
            er: 4.0,
            tan_d: 0.0,
        };
        let warnings = narrow.range_warnings();
        assert!(warnings
            .iter()
            .any(|w| w.code == WARN_RANGE && w.detail.contains("W/h")));
        assert!(microstrip_section().range_warnings().is_empty());
        let thick = Section::Stripline {
            w_m: 0.3 * MM,
            t_m: 0.4 * MM,
            h1_m: 0.5 * MM,
            h2_m: 0.5 * MM,
            er1: 4.0,
            er2: 4.0,
            tan_d1: 0.0,
            tan_d2: 0.0,
        };
        assert!(thick
            .range_warnings()
            .iter()
            .any(|w| w.detail.contains("t/b")));
        let offset = Section::Stripline {
            w_m: 0.2 * MM,
            t_m: 35.0e-6,
            h1_m: 0.2 * MM,
            h2_m: 0.4 * MM,
            er1: 4.0,
            er2: 4.0,
            tan_d1: 0.0,
            tan_d2: 0.0,
        };
        assert!(offset
            .range_warnings()
            .iter()
            .any(|w| w.code == WARN_APPROXIMATE));
        let pair = Section::DiffMicrostrip {
            w_m: 0.2 * MM,
            t_m: 35.0e-6,
            h_m: 0.2 * MM,
            er: 4.0,
            tan_d: 0.0,
            s_m: 5.0 * MM,
        };
        let warnings = pair.range_warnings();
        assert!(warnings.iter().any(|w| w.detail.contains("s/h")));
        assert!(warnings.iter().any(|w| w.code == WARN_APPROXIMATE));
        let thick_copper = Section::Coplanar {
            w_m: 0.3 * MM,
            g_m: 0.1 * MM,
            t_m: 35.0e-6,
            h_m: 1.6 * MM,
            er: 4.5,
            tan_d: 0.0,
            backed: true,
        };
        assert!(thick_copper
            .range_warnings()
            .iter()
            .any(|w| w.detail.contains("coplanar")));
    }
}
