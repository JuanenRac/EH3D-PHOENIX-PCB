// SPDX-License-Identifier: GPL-3.0-or-later
//! A pseudo-random generator of our own for the properties: xorshift64, started from a seed
//! through one step of splitmix64 so that nearby seeds give unrelated sequences. The same seed
//! always gives the same sequence on every computer; nothing here is for secrets.

/// The generator.
#[derive(Debug, Clone)]
pub struct XorShift {
    state: u64,
}

impl XorShift {
    /// A generator for a seed. Any seed is fine, zero included.
    pub fn new(seed: u64) -> XorShift {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        XorShift {
            state: if z == 0 { 0x2545_F491_4F6C_DD1D } else { z },
        }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// A number from 0 to `n` - 1; 0 when `n` is 0.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// A number from `low` to `high`, both included; `low` when `high` is not above it.
    pub fn range(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        let span = (high as i128 - low as i128 + 1) as u64;
        low + self.below(span) as i64
    }

    /// True with the chance `numerator` in `denominator`.
    pub fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        self.below(denominator) < numerator
    }

    /// The index of a random item of a list of `len` items; 0 when it is empty.
    pub fn index(&mut self, len: usize) -> usize {
        self.below(len as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence_and_another_seed_another_one() {
        let mut a = XorShift::new(7);
        let mut b = XorShift::new(7);
        let mut c = XorShift::new(8);
        let (x, y, z): (Vec<u64>, Vec<u64>, Vec<u64>) = (
            (0..16).map(|_| a.next_u64()).collect(),
            (0..16).map(|_| b.next_u64()).collect(),
            (0..16).map(|_| c.next_u64()).collect(),
        );
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn the_state_never_becomes_zero_and_the_values_move() {
        for seed in [0, 1, u64::MAX] {
            let mut r = XorShift::new(seed);
            let first = r.next_u64();
            let second = r.next_u64();
            assert_ne!(r.state, 0);
            assert_ne!(first, second);
        }
    }

    #[test]
    fn the_helpers_stay_inside_their_bounds() {
        let mut r = XorShift::new(42);
        for _ in 0..500 {
            assert!(r.below(7) < 7);
            let v = r.range(-3, 3);
            assert!((-3..=3).contains(&v));
            assert!(r.index(4) < 4);
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.range(5, 5), 5);
        assert_eq!(r.range(9, 2), 9);
        assert_eq!(r.index(0), 0);
        assert!(!r.chance(0, 5));
        assert!(r.chance(5, 5));
    }

    #[test]
    fn every_value_of_a_small_range_comes_up() {
        let mut r = XorShift::new(3);
        let mut seen = [false; 4];
        for _ in 0..200 {
            seen[r.below(4) as usize] = true;
        }
        assert_eq!(seen, [true; 4]);
    }
}
