// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests with a fixed seed of the slivers of solder mask: the sweep is compared with a search
//! written apart that compares every pair.

use crate::mask::{mask_slivers, MaskOpening, MaskSliver};

/// A small generator that always gives the same numbers for the same seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn range(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }
}

fn random_openings(rng: &mut Lcg, count: usize, span: i64) -> Vec<MaskOpening> {
    (0..count)
        .map(|_| {
            let x = rng.range(-span, span);
            let y = rng.range(-span, span);
            MaskOpening {
                x1_nm: x,
                y1_nm: y,
                x2_nm: x + rng.range(1, span / 3),
                y2_nm: y + rng.range(1, span / 3),
            }
        })
        .collect()
}

/// The slow search: every pair, the gap by floating point with a margin settled by integers.
fn slow(list: &[MaskOpening], limit: i64) -> Vec<MaskSliver> {
    let mut out = Vec::new();
    for i in 0..list.len() {
        for j in i + 1..list.len() {
            let (a, b) = (&list[i], &list[j]);
            let (ax1, ax2) = (a.x1_nm.min(a.x2_nm), a.x1_nm.max(a.x2_nm));
            let (ay1, ay2) = (a.y1_nm.min(a.y2_nm), a.y1_nm.max(a.y2_nm));
            let (bx1, bx2) = (b.x1_nm.min(b.x2_nm), b.x1_nm.max(b.x2_nm));
            let (by1, by2) = (b.y1_nm.min(b.y2_nm), b.y1_nm.max(b.y2_nm));
            let dx = (ax1.max(bx1) - ax2.min(bx2)).max(0);
            let dy = (ay1.max(by1) - ay2.min(by2)).max(0);
            let squared = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
            if squared > 0 && squared < i128::from(limit) * i128::from(limit) {
                let mut gap = (squared as f64).sqrt() as u64;
                while i128::from(gap) * i128::from(gap) > squared {
                    gap -= 1;
                }
                while i128::from(gap + 1) * i128::from(gap + 1) <= squared {
                    gap += 1;
                }
                out.push(MaskSliver {
                    first: i,
                    second: j,
                    gap_nm: gap,
                });
            }
        }
    }
    out
}

#[test]
fn the_sweep_gives_what_the_search_of_every_pair_gives() {
    let mut rng = Lcg(0x5EED_0101);
    let mut with_slivers = 0;
    let mut without = 0;
    for case in 0..200 {
        let count = (rng.next() % 40) as usize;
        let span = 20_000 + (case as i64 % 7) * 30_000;
        let list = random_openings(&mut rng, count, span);
        let limit = rng.range(1, 4_000);
        let fast = mask_slivers(&list, limit);
        assert_eq!(fast, slow(&list, limit), "case {case}");
        if fast.is_empty() {
            without += 1;
        } else {
            with_slivers += 1;
        }
    }
    assert!(with_slivers > 30, "{with_slivers}");
    assert!(without > 30, "{without}");
}

#[test]
fn every_sliver_is_in_range_in_order_and_below_the_limit() {
    let mut rng = Lcg(0x5EED_0102);
    for _ in 0..100 {
        let list = random_openings(&mut rng, 30, 30_000);
        let limit = rng.range(1, 3_000);
        let found = mask_slivers(&list, limit);
        for s in &found {
            assert!(s.first < s.second && s.second < list.len());
            assert!(s.gap_nm >= 1 && (s.gap_nm as i64) < limit);
        }
        assert!(found
            .windows(2)
            .all(|w| (w[0].first, w[0].second) < (w[1].first, w[1].second)));
    }
}

#[test]
fn moving_all_the_openings_together_changes_nothing() {
    let mut rng = Lcg(0x5EED_0103);
    for _ in 0..100 {
        let list = random_openings(&mut rng, 25, 30_000);
        let (dx, dy) = (rng.range(-500_000, 500_000), rng.range(-500_000, 500_000));
        let moved: Vec<MaskOpening> = list
            .iter()
            .map(|o| MaskOpening {
                x1_nm: o.x1_nm + dx,
                y1_nm: o.y1_nm + dy,
                x2_nm: o.x2_nm + dx,
                y2_nm: o.y2_nm + dy,
            })
            .collect();
        assert_eq!(mask_slivers(&list, 2_000), mask_slivers(&moved, 2_000));
    }
}
