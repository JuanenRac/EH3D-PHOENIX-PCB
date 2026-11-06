// SPDX-License-Identifier: GPL-3.0-or-later
//! A seeded generator of boxes for the tests that compare the index with the search of every pair.
//! The same seed gives the same boxes on every machine: the generator is SplitMix64 and uses
//! integers only.

use crate::grid::Bx;

/// The SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator started from a seed.
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to but not including `n` (0 when `n` is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        self.next_u64() % n
    }

    /// A number from `lo` to `hi`, both included (`lo` when `hi` is below it).
    pub fn between(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (i128::from(hi) - i128::from(lo) + 1) as u128;
        let r = u128::from(self.next_u64()) % span;
        (i128::from(lo) + r as i128) as i64
    }
}

/// `count` boxes scattered over `[-extent, extent]` on both axes, with sides up to `max_side`.
/// About one in sixteen is a long thin one (a segment that crosses many cells) and about one in
/// thirty-two repeats the box before it, so duplicates are in every set.
pub fn scattered_boxes(seed: u64, count: usize, extent: i64, max_side: i64) -> Vec<Bx> {
    let mut rng = Rng::new(seed);
    let mut out: Vec<Bx> = Vec::with_capacity(count);
    for _ in 0..count {
        let repeat = rng.below(32) == 0;
        if let Some(last) = out.last().copied().filter(|_| repeat) {
            out.push(last);
            continue;
        }
        let x = i128::from(rng.between(-extent, extent));
        let y = i128::from(rng.between(-extent, extent));
        let (w, h) = if rng.below(16) == 0 {
            (
                i128::from(rng.between(0, extent)),
                i128::from(rng.between(0, max_side / 8)),
            )
        } else {
            (
                i128::from(rng.between(0, max_side)),
                i128::from(rng.between(0, max_side)),
            )
        };
        out.push([x, y, x + w, y + h]);
    }
    out
}

/// `count` boxes whose edges lie exactly on multiples of `cell`, so that they touch the border of
/// a cell of a grid of that size, some of them with no width or no height.
pub fn aligned_boxes(seed: u64, count: usize, cell: i64, across: i64) -> Vec<Bx> {
    let mut rng = Rng::new(seed);
    let cell = i128::from(cell.max(1));
    let mut out: Vec<Bx> = Vec::with_capacity(count);
    for _ in 0..count {
        let x = i128::from(rng.between(-across, across)) * cell;
        let y = i128::from(rng.between(-across, across)) * cell;
        let w = i128::from(rng.between(0, 3)) * cell;
        let h = i128::from(rng.between(0, 3)) * cell;
        out.push([x, y, x + w, y + h]);
    }
    out
}

/// `count` boxes of `side` nanometres on a square lattice with `pitch` between their corners and
/// a small jitter, so that every box has a few neighbours at most however many there are.
pub fn lattice_boxes(seed: u64, count: usize, side: i64, pitch: i64) -> Vec<Bx> {
    let mut rng = Rng::new(seed);
    let columns = (count as f64).sqrt().ceil().max(1.0) as usize;
    let jitter = pitch / 10;
    let mut out: Vec<Bx> = Vec::with_capacity(count);
    for i in 0..count {
        let (column, row) = ((i % columns) as i64, (i / columns) as i64);
        let x = i128::from(column * pitch + rng.between(0, jitter));
        let y = i128::from(row * pitch + rng.between(0, jitter));
        out.push([x, y, x + i128::from(side), y + i128::from(side)]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers_and_the_known_first_values() {
        let mut a = Rng::new(0);
        let mut b = Rng::new(0);
        assert_eq!(a.next_u64(), b.next_u64());
        // the first output of SplitMix64 from seed 0
        assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(
            scattered_boxes(5, 50, 1000, 100),
            scattered_boxes(5, 50, 1000, 100)
        );
        assert_ne!(
            scattered_boxes(5, 50, 1000, 100),
            scattered_boxes(6, 50, 1000, 100)
        );
    }

    #[test]
    fn the_numbers_stay_inside_their_range() {
        let mut rng = Rng::new(9);
        for _ in 0..1000 {
            assert!((-3..=4).contains(&rng.between(-3, 4)));
            assert!(rng.below(7) < 7);
        }
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.between(5, 5), 5);
        // the whole range of i64 does not overflow
        let _ = rng.between(i64::MIN, i64::MAX);
    }

    #[test]
    fn the_sets_have_the_requested_size_and_aligned_edges() {
        assert_eq!(scattered_boxes(1, 300, 10_000, 500).len(), 300);
        assert_eq!(lattice_boxes(1, 300, 10, 100).len(), 300);
        for b in aligned_boxes(2, 100, 64, 10) {
            assert!(b.iter().all(|v| v % 64 == 0));
        }
    }
}
