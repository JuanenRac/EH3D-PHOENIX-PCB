// SPDX-License-Identifier: GPL-3.0-or-later
//! The poles and zeros of a linear circuit, found from its AC response.
//!
//! The response `H(j w)` of one signal is fitted by a ratio of two polynomials in `s = j w`,
//! `N(s) / D(s)` with `D(0) = 1` and the same degree `n` in both, by least squares on the relative
//! error (the Sanathanan-Koerner reweighting, a few passes; the least-squares problem is solved
//! by Householder reflections, not by the normal equations, so the conditioning is not squared).
//! The frequencies are scaled by the geometric mean of the sweep first. The orders 1, 2, ... up to
//! `max_pole_order` are tried in turn and the first whose fit is within `pole_fit_tolerance` of
//! the response at every frequency (relative error) is kept. The poles and zeros are the roots of
//! `D` and `N`, found by the Durand-Kerner iteration; a term of `N` that adds less than
//! `pole_trim` of the largest term at the top of the band is taken as zero.
//!
//! This is a fit, not an eigenvalue problem of the matrices of the circuit, and it is only as
//! good as the response it is given:
//!
//! * a pole or a zero far outside the band of the sweep has almost no effect on the response and
//!   is not found accurately; sweep a band that holds the poles you want;
//! * a pole at the origin (an ideal integrator) is not covered, because the model has `D(0) = 1`;
//! * a response that no rational function of order `max_pole_order` fits (a transmission line, a
//!   pure delay) is refused with `PHOENIX-SIM-029`; the fit never returns poles of a model whose
//!   error is above the tolerance.

use std::f64::consts::PI;

use eh3d_phoenix_rf::Complex;

use crate::codes::{SimError, FIT_FAILED};
use crate::settings::SimSettings;

/// The poles and zeros of a response.
#[derive(Debug, Clone, PartialEq)]
pub struct PoleZero {
    /// The degree of the denominator of the fitted model (the number of poles).
    pub order: usize,
    /// The poles in rad/s, by increasing magnitude.
    pub poles: Vec<Complex>,
    /// The zeros in rad/s, by increasing magnitude.
    pub zeros: Vec<Complex>,
    /// The response of the fitted model at zero frequency.
    pub dc_gain: f64,
    /// The largest relative error of the fitted model over the frequencies of the response.
    pub max_error: f64,
}

/// The magnitude of a pole or zero as a frequency in hertz.
pub fn frequency_hz(root: Complex) -> f64 {
    root.abs() / (2.0 * PI)
}

/// The quality factor `|p| / (2 |Re p|)` of a pole or zero with an imaginary part; `None` for a
/// real one and for one on the imaginary axis.
pub fn quality(root: Complex) -> Option<f64> {
    if root.im.abs() <= 1e-9 * root.abs() || root.re == 0.0 {
        return None;
    }
    Some(root.abs() / (2.0 * root.re.abs()))
}

/// The polynomial with the coefficients `c` (lowest degree first) at `s`.
fn poly_eval(c: &[f64], s: Complex) -> Complex {
    let mut acc = Complex::ZERO;
    for &coefficient in c.iter().rev() {
        acc = acc * s + Complex::real(coefficient);
    }
    acc
}

/// The least-squares solution of `a x = b` for `a` with `rows` rows and `cols` columns (by
/// rows), by Householder reflections after scaling every column to length one. `None` when a
/// column is zero or the matrix has no full rank.
fn least_squares(mut a: Vec<f64>, mut b: Vec<f64>, rows: usize, cols: usize) -> Option<Vec<f64>> {
    if cols == 0 || rows < cols || a.len() != rows * cols || b.len() != rows {
        return None;
    }
    let mut scale: Vec<f64> = Vec::with_capacity(cols);
    for j in 0..cols {
        let norm = (0..rows)
            .map(|i| a[i * cols + j] * a[i * cols + j])
            .sum::<f64>()
            .sqrt();
        if norm.is_nan() || norm <= 0.0 || !norm.is_finite() {
            return None;
        }
        scale.push(norm);
        for i in 0..rows {
            a[i * cols + j] /= norm;
        }
    }
    for k in 0..cols {
        let norm = (k..rows)
            .map(|i| a[i * cols + k] * a[i * cols + k])
            .sum::<f64>()
            .sqrt();
        if norm.is_nan() || norm <= 0.0 {
            return None;
        }
        let alpha = if a[k * cols + k] > 0.0 { -norm } else { norm };
        let mut v: Vec<f64> = (k..rows).map(|i| a[i * cols + k]).collect();
        v[0] -= alpha;
        let v_norm2: f64 = v.iter().map(|x| x * x).sum();
        if v_norm2 <= 0.0 {
            continue;
        }
        for j in k..cols {
            let dot: f64 = v
                .iter()
                .enumerate()
                .map(|(idx, vi)| vi * a[(k + idx) * cols + j])
                .sum();
            let factor = 2.0 * dot / v_norm2;
            for (idx, vi) in v.iter().enumerate() {
                a[(k + idx) * cols + j] -= factor * vi;
            }
        }
        let dot: f64 = v.iter().enumerate().map(|(idx, vi)| vi * b[k + idx]).sum();
        let factor = 2.0 * dot / v_norm2;
        for (idx, vi) in v.iter().enumerate() {
            b[k + idx] -= factor * vi;
        }
    }
    let mut x = vec![0.0f64; cols];
    for k in (0..cols).rev() {
        let tail: f64 = x
            .iter()
            .enumerate()
            .skip(k + 1)
            .map(|(j, xj)| a[k * cols + j] * xj)
            .sum();
        let diagonal = a[k * cols + k];
        if diagonal.abs() < 1e-12 || !diagonal.is_finite() {
            return None;
        }
        x[k] = (b[k] - tail) / diagonal;
    }
    for (xj, sj) in x.iter_mut().zip(scale.iter()) {
        *xj /= sj;
    }
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}

/// The numerator and denominator (lowest degree first, `D(0) = 1`) of the fit of degree `n` in
/// both, to the response `h` at the normalised frequencies `sigma` (`s = j sigma`).
fn fit(sigma: &[f64], h: &[Complex], n: usize, passes: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    let m = n;
    let cols = (m + 1) + n;
    let samples = sigma.len();
    if 2 * samples < cols + 2 {
        return None;
    }
    let mut den: Vec<f64> = vec![1.0];
    let mut num: Vec<f64> = Vec::new();
    for _ in 0..passes.max(1) {
        let mut a: Vec<f64> = Vec::with_capacity(2 * samples * cols);
        let mut b: Vec<f64> = Vec::with_capacity(2 * samples);
        for (&sg, &hk) in sigma.iter().zip(h.iter()) {
            let s = Complex::new(0.0, sg);
            let weight = 1.0 / (hk.abs() * poly_eval(&den, s).abs().max(1e-12));
            let mut powers: Vec<Complex> = Vec::with_capacity(n + 1);
            let mut p = Complex::ONE;
            for _ in 0..=n {
                powers.push(p);
                p = p * s;
            }
            let mut row_re: Vec<f64> = Vec::with_capacity(cols);
            let mut row_im: Vec<f64> = Vec::with_capacity(cols);
            for pk in powers.iter() {
                row_re.push(pk.re * weight);
                row_im.push(pk.im * weight);
            }
            for pk in powers.iter().skip(1) {
                let t = hk * *pk;
                row_re.push(-t.re * weight);
                row_im.push(-t.im * weight);
            }
            a.extend(row_re);
            b.push(hk.re * weight);
            a.extend(row_im);
            b.push(hk.im * weight);
        }
        let x = least_squares(a, b, 2 * samples, cols)?;
        num = x[..=m].to_vec();
        den = vec![1.0];
        den.extend_from_slice(&x[m + 1..]);
    }
    Some((num, den))
}

/// The largest relative error of the model `num / den` against the response, or infinity when
/// the model is not a finite number somewhere.
fn max_relative_error(sigma: &[f64], h: &[Complex], num: &[f64], den: &[f64]) -> f64 {
    let mut worst = 0.0f64;
    for (&sg, &hk) in sigma.iter().zip(h.iter()) {
        let s = Complex::new(0.0, sg);
        let model = poly_eval(num, s) / poly_eval(den, s);
        let error = (model - hk).abs() / hk.abs();
        if !error.is_finite() {
            return f64::INFINITY;
        }
        worst = worst.max(error);
    }
    worst
}

/// The roots of the polynomial with the coefficients `c` (lowest degree first) by the
/// Durand-Kerner iteration; `None` when it does not settle.
fn roots(c: &[f64], iterations: usize, tolerance: f64) -> Option<Vec<Complex>> {
    let mut c: Vec<f64> = c.to_vec();
    while c.last() == Some(&0.0) {
        c.pop();
    }
    if c.is_empty() {
        return None;
    }
    let degree = c.len() - 1;
    if degree == 0 {
        return Some(Vec::new());
    }
    let lead = c[degree];
    let monic: Vec<f64> = c.iter().map(|v| v / lead).collect();
    let geometric = monic[0].abs().powf(1.0 / degree as f64);
    let radius = if geometric.is_finite() {
        geometric.clamp(0.05, 20.0)
    } else {
        1.0
    };
    let mut z: Vec<Complex> = (0..degree)
        .map(|k| Complex::from_polar(radius, 2.0 * PI * k as f64 / degree as f64 + 0.4))
        .collect();
    let mut change = f64::INFINITY;
    for _ in 0..iterations {
        let old = z.clone();
        change = 0.0;
        for (i, zi) in z.iter_mut().enumerate() {
            let mut denominator = Complex::ONE;
            for (j, zj) in old.iter().enumerate() {
                if j != i {
                    denominator = denominator * (old[i] - *zj);
                }
            }
            let delta = poly_eval(&monic, old[i]) / denominator;
            if !delta.is_finite() {
                return None;
            }
            *zi = old[i] - delta;
            change = change.max(delta.abs() / (1.0 + old[i].abs()));
        }
        if change < tolerance {
            break;
        }
    }
    if change > 1e-6 {
        return None;
    }
    for root in z.iter_mut() {
        if root.im.abs() < 1e-10 * root.abs() {
            root.im = 0.0;
        }
    }
    Some(z)
}

fn sorted(mut list: Vec<Complex>) -> Vec<Complex> {
    list.sort_by(|a, b| a.abs().total_cmp(&b.abs()).then(b.im.total_cmp(&a.im)));
    list
}

/// Finds the poles and zeros of the response `h` at the frequencies `freqs` (hertz, above zero).
/// With `order` the fit has that many poles; without it the orders 1 to `max_pole_order` are tried
/// and the lowest that fits is kept.
pub fn pole_zero(
    freqs: &[f64],
    h: &[Complex],
    order: Option<usize>,
    s: &SimSettings,
) -> Result<PoleZero, SimError> {
    let fail = |why: String| SimError::new(FIT_FAILED, why);
    if freqs.len() != h.len() || freqs.len() < 6 {
        return Err(fail(
            "the fit needs a response at six frequencies or more".to_string(),
        ));
    }
    if freqs.iter().any(|f| !(f.is_finite() && *f > 0.0)) {
        return Err(fail("the frequencies must be above zero".to_string()));
    }
    if h.iter().any(|v| !v.is_finite() || v.abs() == 0.0) {
        return Err(fail(
            "the response is zero or not a finite number at some frequency".to_string(),
        ));
    }
    let low = freqs.iter().copied().fold(f64::INFINITY, f64::min);
    let high = freqs.iter().copied().fold(0.0, f64::max);
    if high <= low {
        return Err(fail("the frequencies must cover a band".to_string()));
    }
    let w0 = 2.0 * PI * (low * high).sqrt();
    let sigma: Vec<f64> = freqs.iter().map(|f| 2.0 * PI * f / w0).collect();
    let sigma_max = sigma.iter().copied().fold(0.0f64, f64::max);
    let orders: Vec<usize> = match order {
        Some(n) if (1..=s.max_pole_order).contains(&n) => vec![n],
        Some(n) => {
            return Err(fail(format!(
                "the order {n} is not between 1 and max_pole_order ({})",
                s.max_pole_order
            )))
        }
        None => (1..=s.max_pole_order).collect(),
    };
    let mut best = f64::INFINITY;
    for n in orders {
        let Some((num, den)) = fit(&sigma, h, n, s.pole_fit_iterations) else {
            continue;
        };
        let error = max_relative_error(&sigma, h, &num, &den);
        best = best.min(error);
        if error > s.pole_fit_tolerance {
            continue;
        }
        // a term of the numerator counts for what it adds at the top of the band
        let weight = |k: usize, c: f64| c.abs() * sigma_max.powi(k as i32);
        let biggest = num
            .iter()
            .enumerate()
            .fold(0.0f64, |a, (k, c)| a.max(weight(k, *c)));
        let mut kept = num.clone();
        while kept.len() > 1 && weight(kept.len() - 1, kept[kept.len() - 1]) < s.pole_trim * biggest
        {
            kept.pop();
        }
        let found = (
            roots(&den, s.pole_root_iterations, s.pole_root_tolerance),
            roots(&kept, s.pole_root_iterations, s.pole_root_tolerance),
        );
        let (Some(poles), Some(zeros)) = found else {
            continue;
        };
        return Ok(PoleZero {
            order: n,
            poles: sorted(poles.into_iter().map(|p| p.scale(w0)).collect()),
            zeros: sorted(zeros.into_iter().map(|z| z.scale(w0)).collect()),
            dc_gain: num[0],
            max_error: error,
        });
    }
    Err(fail(format!(
        "no rational model with up to {} poles fits the response within pole_fit_tolerance ({}); the best relative error was {best:e}",
        s.max_pole_order, s.pole_fit_tolerance
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(per_decade: usize, decades: usize, start: f64) -> Vec<f64> {
        (0..=(per_decade * decades))
            .map(|k| start * 10f64.powf(k as f64 / per_decade as f64))
            .collect()
    }

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs()
    }

    #[test]
    fn least_squares_solves_an_overdetermined_system() {
        // y = 2 + 3 x at x = 0..4
        let rows = 5;
        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..rows {
            let x = i as f64;
            a.extend([1.0, x]);
            b.push(2.0 + 3.0 * x);
        }
        let solution = least_squares(a, b, rows, 2).unwrap();
        assert!(close(solution[0], 2.0, 1e-12), "{solution:?}");
        assert!(close(solution[1], 3.0, 1e-12), "{solution:?}");
        // a column of zeros has no solution
        assert!(least_squares(vec![1.0, 0.0, 1.0, 0.0], vec![1.0, 1.0], 2, 2).is_none());
    }

    #[test]
    fn roots_of_a_polynomial_are_found() {
        // (s + 1)(s + 2)(s^2 + 2 s + 5) = s^4 + 5 s^3 + 13 s^2 + 19 s + 10 has the roots -1, -2
        // and -1 +- 2j
        let c = [10.0, 19.0, 13.0, 5.0, 1.0];
        let found = sorted(roots(&c, 500, 1e-13).unwrap());
        assert_eq!(found.len(), 4);
        let want = [(-1.0, 0.0), (-2.0, 0.0), (-1.0, 2.0), (-1.0, -2.0)];
        for (re, im) in want {
            assert!(
                found
                    .iter()
                    .any(|r| (r.re - re).abs() < 1e-9 && (r.im - im).abs() < 1e-9),
                "{re} {im} in {found:?}"
            );
        }
        assert!(roots(&[0.0, 0.0], 10, 1e-12).is_none());
        assert_eq!(roots(&[3.0], 10, 1e-12).unwrap().len(), 0);
    }

    #[test]
    fn the_pole_of_a_first_order_low_pass_is_found() {
        let fc = 159.154_943_091_895_35;
        let freqs = band(20, 4, 10.0);
        let h: Vec<Complex> = freqs
            .iter()
            .map(|f| Complex::ONE / Complex::new(1.0, f / fc))
            .collect();
        let r = pole_zero(&freqs, &h, None, &SimSettings::default()).unwrap();
        assert_eq!(r.order, 1);
        assert_eq!(r.poles.len(), 1);
        assert!(r.zeros.is_empty());
        assert!(close(r.poles[0].re, -2.0 * PI * fc, 1e-6), "{:?}", r.poles);
        assert!(r.poles[0].im.abs() < 1e-6 * r.poles[0].re.abs());
        assert!(close(frequency_hz(r.poles[0]), fc, 1e-6));
        assert!(close(r.dc_gain, 1.0, 1e-6));
        assert!(r.max_error < 1e-6);
        assert_eq!(quality(r.poles[0]), None);
    }

    #[test]
    fn the_poles_of_a_second_order_low_pass_have_its_frequency_and_quality() {
        let (f0, q) = (1000.0, 3.0);
        let freqs = band(20, 4, 10.0);
        let h: Vec<Complex> = freqs
            .iter()
            .map(|f| {
                let x = f / f0;
                Complex::ONE / Complex::new(1.0 - x * x, x / q)
            })
            .collect();
        let r = pole_zero(&freqs, &h, None, &SimSettings::default()).unwrap();
        assert_eq!(r.order, 2);
        assert_eq!(r.poles.len(), 2);
        for p in &r.poles {
            assert!(close(frequency_hz(*p), f0, 1e-6), "{p:?}");
            assert!(close(quality(*p).unwrap(), q, 1e-6), "{p:?}");
            assert!(p.re < 0.0);
        }
        assert!(r.poles[0].im * r.poles[1].im < 0.0);
    }

    #[test]
    fn a_zero_is_found_as_well() {
        let (fz, fp) = (500.0, 5000.0);
        let freqs = band(20, 4, 10.0);
        let h: Vec<Complex> = freqs
            .iter()
            .map(|f| Complex::new(1.0, f / fz) / Complex::new(1.0, f / fp))
            .collect();
        let r = pole_zero(&freqs, &h, Some(1), &SimSettings::default()).unwrap();
        assert_eq!(r.poles.len(), 1);
        assert_eq!(r.zeros.len(), 1);
        assert!(close(frequency_hz(r.poles[0]), fp, 1e-6));
        assert!(close(frequency_hz(r.zeros[0]), fz, 1e-6));
        assert!(r.zeros[0].re < 0.0);
    }

    #[test]
    fn a_response_that_no_low_order_model_fits_is_refused() {
        // a pure delay of 1 ms between 100 Hz and 10 kHz
        let freqs = band(20, 2, 100.0);
        let h: Vec<Complex> = freqs
            .iter()
            .map(|f| Complex::from_polar(1.0, -2.0 * PI * f * 1e-3))
            .collect();
        let s = SimSettings {
            max_pole_order: 3,
            ..SimSettings::default()
        };
        let e = pole_zero(&freqs, &h, None, &s).unwrap_err();
        assert_eq!(e.code, FIT_FAILED);
        // a forced order outside the settings is refused too
        let e = pole_zero(&freqs, &h, Some(9), &s).unwrap_err();
        assert_eq!(e.code, FIT_FAILED);
    }

    #[test]
    fn a_response_with_too_few_points_or_a_zero_is_refused() {
        let s = SimSettings::default();
        let few = [1.0, 2.0, 3.0];
        let h = [Complex::ONE; 3];
        assert_eq!(pole_zero(&few, &h, None, &s).unwrap_err().code, FIT_FAILED);
        let freqs = band(2, 3, 1.0);
        let mut h: Vec<Complex> = freqs.iter().map(|_| Complex::ONE).collect();
        h[2] = Complex::ZERO;
        assert_eq!(
            pole_zero(&freqs, &h, None, &s).unwrap_err().code,
            FIT_FAILED
        );
    }
}
