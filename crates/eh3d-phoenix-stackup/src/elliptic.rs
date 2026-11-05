// SPDX-License-Identifier: GPL-3.0-or-later
//! The complete elliptic integral of the first kind, by the arithmetic-geometric mean.
//!
//! `K(k) = pi / (2 * AGM(1, sqrt(1 - k^2)))` (Abramowitz and Stegun, 17.6). The coplanar waveguide
//! and the stripline of zero thickness are written with the ratio of two such integrals.

use std::f64::consts::PI;

/// The arithmetic-geometric mean of two non-negative numbers.
fn agm(first: f64, second: f64) -> f64 {
    let mut a = first;
    let mut b = second;
    for _ in 0..64 {
        if (a - b).abs() <= 1.0e-15 * a {
            break;
        }
        let next_a = (a + b) / 2.0;
        b = (a * b).sqrt();
        a = next_a;
    }
    a
}

/// The complete elliptic integral of the first kind `K(k)` for a modulus `0 <= k < 1`.
/// The result is infinite at `k = 1` and not a number for a modulus outside `0..=1`.
pub fn complete_k(k: f64) -> f64 {
    if k == 1.0 {
        return f64::INFINITY;
    }
    if !(0.0..1.0).contains(&k) {
        return f64::NAN;
    }
    PI / (2.0 * agm(1.0, (1.0 - k * k).sqrt()))
}

/// The ratio `K(k) / K(k')` of a modulus `k` and its complementary modulus `kc = sqrt(1 - k^2)`,
/// both given, so that a modulus very close to one can be passed with a complement that was
/// computed without the loss of precision of `1 - k^2`. It is `AGM(1, k) / AGM(1, kc)`.
pub fn k_over_complement(k: f64, kc: f64) -> f64 {
    agm(1.0, k) / agm(1.0, kc)
}

/// The ratio `K(k') / K(k)` where `k' = sqrt(1 - k^2)` is the complementary modulus.
pub fn k_ratio(k: f64) -> f64 {
    let complementary = (1.0 - k * k).sqrt();
    complete_k(complementary) / complete_k(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k_of_zero_is_half_pi() {
        assert!((complete_k(0.0) - PI / 2.0).abs() < 1e-14);
    }

    #[test]
    fn k_matches_published_table_values() {
        // Abramowitz and Stegun, table 17.1: K(m) with m = k^2 = 0.5 is 1.8540746773
        let k = 0.5_f64.sqrt();
        assert!((complete_k(k) - 1.854_074_677_3).abs() < 1e-9);
        // m = 0.25 gives 1.6857503548
        assert!((complete_k(0.5) - 1.685_750_354_8).abs() < 1e-9);
    }

    #[test]
    fn the_ratio_is_one_at_the_self_complementary_modulus() {
        assert!((k_ratio(0.5_f64.sqrt()) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_ratio_falls_as_the_modulus_grows() {
        let mut last = f64::INFINITY;
        for step in 1..20 {
            let k = f64::from(step) / 20.0;
            let r = k_ratio(k);
            assert!(r < last, "{k}");
            last = r;
        }
    }

    #[test]
    fn the_ratio_with_a_given_complement_agrees_with_the_plain_ratio() {
        for k in [0.1_f64, 0.5, 0.9, 0.99] {
            let kc = (1.0 - k * k).sqrt();
            assert!(
                (k_over_complement(k, kc) - 1.0 / k_ratio(k)).abs() < 1e-12,
                "{k}"
            );
        }
        // a complement far below the precision of 1 - k^2 is still a finite ratio
        let huge = k_over_complement(1.0, 1.0e-150);
        assert!(huge.is_finite() && huge > 100.0);
    }

    #[test]
    fn the_integral_is_infinite_at_one_and_not_a_number_beyond() {
        assert!(complete_k(1.0).is_infinite());
        assert!(complete_k(1.5).is_nan());
    }
}
