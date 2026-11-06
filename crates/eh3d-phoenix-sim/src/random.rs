// SPDX-License-Identifier: GPL-3.0-or-later
//! A pseudo-random generator of our own for the Monte Carlo studies. It is SplitMix64 (a 64-bit
//! state that grows by an odd constant and is scrambled by two multiplications), so a seed always
//! gives the same numbers on every machine; nothing reads the clock or the operating system. The
//! uniform numbers have 53 random bits, and the normal numbers come from the Box-Muller
//! transform with the functions of `std`.

use std::f64::consts::PI;

/// The scrambling step of SplitMix64.
fn mix(value: u64) -> u64 {
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The generator: a state and the second number of a Box-Muller pair that was not used yet.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
    spare: Option<f64>,
}

impl Rng {
    /// A generator that starts from a seed.
    pub fn new(seed: u64) -> Rng {
        Rng {
            state: seed,
            spare: None,
        }
    }

    /// The generator of run number `index` of a study with a seed: every run has a stream of its
    /// own, so run 7 is the same whether or not the runs before it were made.
    pub fn stream(seed: u64, index: u64) -> Rng {
        Rng::new(mix(seed.wrapping_add(mix(index.wrapping_add(1)))))
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// A number in `[0, 1)` with 53 random bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }

    /// A number in `[-1, 1)`.
    pub fn symmetric(&mut self) -> f64 {
        2.0 * self.uniform() - 1.0
    }

    /// A number of the standard normal distribution (mean 0, deviation 1), by the Box-Muller
    /// transform; each draw of two uniform numbers gives two normal numbers, and the second is
    /// kept for the next call.
    pub fn normal(&mut self) -> f64 {
        if let Some(value) = self.spare.take() {
            return value;
        }
        let u1 = 1.0 - self.uniform();
        let u2 = self.uniform();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * PI * u2;
        self.spare = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_gives_its_published_first_number_for_the_seed_zero() {
        assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn the_same_seed_gives_the_same_numbers_and_another_seed_does_not() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let mut c = Rng::new(43);
        let first: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let third: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn the_streams_of_two_runs_differ_and_each_repeats() {
        let a = Rng::stream(7, 0).next_u64();
        let b = Rng::stream(7, 1).next_u64();
        assert_ne!(a, b);
        assert_eq!(a, Rng::stream(7, 0).next_u64());
    }

    #[test]
    fn uniform_numbers_stay_in_range_with_the_mean_and_deviation_of_the_distribution() {
        let mut rng = Rng::new(2024);
        let n = 20_000;
        let values: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
        assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
        let mean = values.iter().sum::<f64>() / n as f64;
        let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64;
        // mean 1/2 and variance 1/12; the standard error of the mean is 0.002
        assert!((mean - 0.5).abs() < 0.012, "{mean}");
        assert!((var - 1.0 / 12.0).abs() < 0.004, "{var}");
    }

    #[test]
    fn normal_numbers_have_mean_zero_and_deviation_one() {
        let mut rng = Rng::new(99);
        let n = 20_000;
        let values: Vec<f64> = (0..n).map(|_| rng.normal()).collect();
        let mean = values.iter().sum::<f64>() / n as f64;
        let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64;
        // the standard error of the mean is 0.007 and that of the deviation 0.005
        assert!(mean.abs() < 0.05, "{mean}");
        assert!((var.sqrt() - 1.0).abs() < 0.04, "{}", var.sqrt());
        assert!(values.iter().all(|v| v.is_finite()));
    }
}
