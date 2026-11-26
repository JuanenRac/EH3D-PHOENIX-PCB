// SPDX-License-Identifier: GPL-3.0-or-later
//! Maps for the inner loop of the search. The keys are small numbers made by the program itself
//! (nodes of the grid and layers), so a multiplication is enough to spread them, and it is much
//! cheaper than the hash that guards against hostile keys. Nothing depends on the order of these
//! maps, and the hash has no seed, so a run is the same every time.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A map with the quick hash.
pub(crate) type Fast<K, V> = HashMap<K, V, BuildHasherDefault<MulHasher>>;

/// Rotate, mix in and multiply by an odd number.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MulHasher(u64);

impl Hasher for MulHasher {
    fn finish(&self) -> u64 {
        // the best mixed bits of a product are the high ones, and a table looks at the low ones
        self.0.rotate_left(26)
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(u64::from(*b));
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_i64(&mut self, n: i64) {
        self.write_u64(n as u64);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_finds_what_was_put_in_it_and_the_hash_is_the_same_every_time() {
        let mut map: Fast<(usize, i64, i64), i64> = Fast::default();
        for slot in 0..3_usize {
            for x in -20..20_i64 {
                for y in -20..20_i64 {
                    map.insert((slot, x, y), (slot as i64) * 10_000 + x * 100 + y);
                }
            }
        }
        assert_eq!(map.len(), 3 * 40 * 40);
        assert_eq!(map.get(&(2, 3, 4)), Some(&20_304));
        assert_eq!(map.get(&(3, 0, 0)), None);
        let mut a = MulHasher::default();
        let mut b = MulHasher::default();
        a.write(b"abc");
        b.write(b"abc");
        assert_eq!(a.finish(), b.finish());
        b.write_u64(1);
        assert_ne!(a.finish(), b.finish());
    }
}
