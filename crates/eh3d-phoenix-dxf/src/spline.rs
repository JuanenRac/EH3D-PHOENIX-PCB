// SPDX-License-Identifier: GPL-3.0-or-later
//! Splines of a DXF file as straight pieces. A spline is a B-spline (a NURBS when it has weights)
//! given by its degree, its knots and its control points; it is evaluated by the recurrence of
//! Cox and de Boor and cut into pieces so that the middle of no piece is further from the curve
//! than a tolerance. The lengths are the units of the drawing; the caller converts them.

/// A spline as a file gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct SplineData {
    /// The degree.
    pub degree: usize,
    /// The knots: as many as control points plus the degree plus one.
    pub knots: Vec<f64>,
    /// The control points.
    pub control: Vec<(f64, f64)>,
    /// The weights, one for each control point, or none.
    pub weights: Vec<f64>,
}

/// The largest degree that is evaluated.
const MAX_DEGREE: usize = 10;

fn basis(i: usize, degree: usize, t: f64, k: &[f64]) -> f64 {
    if degree == 0 {
        return if k[i] <= t && t < k[i + 1] { 1.0 } else { 0.0 };
    }
    let mut value = 0.0;
    let left = k[i + degree] - k[i];
    if left > 0.0 {
        value += (t - k[i]) / left * basis(i, degree - 1, t, k);
    }
    let right = k[i + degree + 1] - k[i + 1];
    if right > 0.0 {
        value += (k[i + degree + 1] - t) / right * basis(i + 1, degree - 1, t, k);
    }
    value
}

fn valid(d: &SplineData) -> bool {
    let n = d.control.len();
    d.degree >= 1
        && d.degree <= MAX_DEGREE
        && n > d.degree
        && d.knots.len() == n + d.degree + 1
        && d.knots.iter().all(|k| k.is_finite())
        && d.knots.windows(2).all(|w| w[0] <= w[1])
        && d.knots[n] > d.knots[d.degree]
        && d.control.iter().all(|c| c.0.is_finite() && c.1.is_finite())
        && (d.weights.is_empty()
            || (d.weights.len() == n && d.weights.iter().all(|w| w.is_finite() && *w > 0.0)))
}

fn eval(d: &SplineData, t: f64) -> (f64, f64) {
    let n = d.control.len();
    let p = d.degree;
    let k = &d.knots;
    let t = t.min(k[n] - (k[n] - k[p]) * 1e-12).max(k[p]);
    let mut s = p;
    while s + 1 < n && k[s + 1] <= t {
        s += 1;
    }
    let (mut x, mut y, mut w) = (0.0, 0.0, 0.0);
    for i in (s - p)..=s {
        let weight = d.weights.get(i).copied().unwrap_or(1.0);
        let b = basis(i, p, t, k) * weight;
        x += b * d.control[i].0;
        y += b * d.control[i].1;
        w += b;
    }
    if w == 0.0 {
        d.control[s]
    } else {
        (x / w, y / w)
    }
}

fn refine(
    d: &SplineData,
    from: (f64, (f64, f64)),
    to: (f64, (f64, f64)),
    tolerance: f64,
    depth: u32,
    budget: &mut usize,
    out: &mut Vec<(f64, f64)>,
) {
    let t_mid = (from.0 + to.0) / 2.0;
    let mid = eval(d, t_mid);
    let chord_mid = ((from.1 .0 + to.1 .0) / 2.0, (from.1 .1 + to.1 .1) / 2.0);
    let deviation = (mid.0 - chord_mid.0).hypot(mid.1 - chord_mid.1);
    if depth > 0 && deviation > tolerance && *budget > 0 {
        *budget -= 1;
        let middle = (t_mid, mid);
        refine(d, from, middle, tolerance, depth - 1, budget, out);
        refine(d, middle, to, tolerance, depth - 1, budget, out);
    } else {
        out.push(to.1);
    }
}

/// The curve as points in order, no piece of them further than `tolerance` from it at its
/// middle, at most `max_points` of them. `None` when the data is not a spline that can be
/// evaluated (a degree of 0 or above 10, too few control points, knots that are not as many as
/// they must be or that decrease, a weight that is not above 0).
pub fn sample(d: &SplineData, tolerance: f64, max_points: usize) -> Option<Vec<(f64, f64)>> {
    if !valid(d) || max_points < 2 {
        return None;
    }
    let n = d.control.len();
    let (t0, t1) = (d.knots[d.degree], d.knots[n]);
    let initial = (n * 4).clamp(8, 4096).min(max_points - 1);
    let start = eval(d, t0);
    let mut out = vec![start];
    let mut budget = max_points.saturating_sub(initial + 1);
    let mut previous = (t0, start);
    for i in 1..=initial {
        let t = t0 + (t1 - t0) * (i as f64) / (initial as f64);
        let here = (t, eval(d, t));
        refine(d, previous, here, tolerance, 10, &mut budget, &mut out);
        previous = here;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bezier() -> SplineData {
        SplineData {
            degree: 2,
            knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            control: vec![(0.0, 0.0), (5.0, 10.0), (10.0, 0.0)],
            weights: Vec::new(),
        }
    }

    #[test]
    fn a_quadratic_bezier_passes_through_its_ends_and_the_middle_of_its_arch() {
        let points = sample(&bezier(), 0.01, 1000).unwrap();
        let first = points[0];
        let last = *points.last().unwrap();
        assert!(first.0.abs() < 1e-9 && first.1.abs() < 1e-9, "{first:?}");
        assert!(
            (last.0 - 10.0).abs() < 1e-6 && last.1.abs() < 1e-6,
            "{last:?}"
        );
        // at t = 0.5 the point is (1/4) P0 + (1/2) P1 + (1/4) P2 = (5, 5)
        assert!(
            points
                .iter()
                .any(|p| (p.0 - 5.0).abs() < 1e-9 && (p.1 - 5.0).abs() < 1e-9),
            "{points:?}"
        );
        assert!(points.iter().all(|p| p.1 <= 5.0 + 1e-9));
        assert!(points.len() > 12);
    }

    #[test]
    fn a_finer_tolerance_gives_more_points_and_the_budget_limits_them() {
        let coarse = sample(&bezier(), 1.0, 1000).unwrap().len();
        let fine = sample(&bezier(), 0.0001, 1000).unwrap().len();
        assert!(fine > coarse, "{fine} {coarse}");
        let limited = sample(&bezier(), 0.0000001, 20).unwrap().len();
        assert!(limited <= 40, "{limited}");
    }

    #[test]
    fn a_linear_spline_is_its_polyline() {
        let d = SplineData {
            degree: 1,
            knots: vec![0.0, 0.0, 1.0, 2.0, 2.0],
            control: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            weights: Vec::new(),
        };
        let points = sample(&d, 0.001, 1000).unwrap();
        assert!(points
            .iter()
            .any(|p| (p.0 - 10.0).abs() < 1e-9 && p.1.abs() < 1e-9));
        assert!(points.iter().all(|p| (-1e-9..=10.0 + 1e-9).contains(&p.0)));
    }

    #[test]
    fn weights_pull_the_curve_to_a_control_point() {
        // a rational quadratic with the weight sqrt(2)/2 on the middle point is an exact quarter
        // circle of radius 10 about the origin
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let d = SplineData {
            degree: 2,
            knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            control: vec![(10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            weights: vec![1.0, w, 1.0],
        };
        for p in sample(&d, 0.001, 1000).unwrap() {
            let r = p.0.hypot(p.1);
            assert!((r - 10.0).abs() < 1e-6, "{p:?} {r}");
        }
    }

    #[test]
    fn what_cannot_be_evaluated_is_refused() {
        let mut d = bezier();
        d.knots.pop();
        assert!(sample(&d, 0.01, 100).is_none());
        let mut d = bezier();
        d.degree = 0;
        assert!(sample(&d, 0.01, 100).is_none());
        let mut d = bezier();
        d.control.pop();
        assert!(sample(&d, 0.01, 100).is_none());
        let mut d = bezier();
        d.knots = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!(sample(&d, 0.01, 100).is_none());
        let mut d = bezier();
        d.weights = vec![1.0, 0.0, 1.0];
        assert!(sample(&d, 0.01, 100).is_none());
        assert!(sample(&bezier(), 0.01, 1).is_none());
    }
}
