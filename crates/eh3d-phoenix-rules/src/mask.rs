// SPDX-License-Identifier: GPL-3.0-or-later
//! Slivers of solder mask between two openings.
//!
//! A strip of solder mask narrower than a maker can print lifts off or is left out, and the two
//! pads it was meant to keep apart are open to a bridge of solder. [`mask_slivers`] reads the
//! rectangles of the openings of one side of a board and lists each pair whose gap is greater
//! than nothing and less than the narrowest strip that may be left. The openings are read as
//! given: the expansion of the mask over the pad must be added to the rectangles before.
//!
//! All arithmetic is in integer nanometres and exact: the gap of two rectangles is worked out as
//! a square and compared with the square of the limit, and the gap that is reported is rounded
//! down to a whole nanometre. Two openings that touch or overlap are one opening and give
//! nothing.

/// A rectangle of the mask that is opened, in nanometres. The corners may be given in any order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskOpening {
    /// A corner.
    pub x1_nm: i64,
    /// A corner.
    pub y1_nm: i64,
    /// The opposite corner.
    pub x2_nm: i64,
    /// The opposite corner.
    pub y2_nm: i64,
}

impl MaskOpening {
    fn bounds(&self) -> (i128, i128, i128, i128) {
        (
            i128::from(self.x1_nm.min(self.x2_nm)),
            i128::from(self.y1_nm.min(self.y2_nm)),
            i128::from(self.x1_nm.max(self.x2_nm)),
            i128::from(self.y1_nm.max(self.y2_nm)),
        )
    }
}

/// A strip of mask that is too narrow between two openings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskSliver {
    /// The position in the list of the first opening (the smaller one).
    pub first: usize,
    /// The position in the list of the second opening.
    pub second: usize,
    /// The width of the strip, rounded down to a whole nanometre; at least 1.
    pub gap_nm: u64,
}

/// The distance between two rectangles as a square, in square nanometres; 0 when they touch or
/// overlap.
fn gap_squared(a: (i128, i128, i128, i128), b: (i128, i128, i128, i128)) -> i128 {
    let dx = (a.0.max(b.0) - a.2.min(b.2)).max(0);
    let dy = (a.1.max(b.1) - a.3.min(b.3)).max(0);
    dx * dx + dy * dy
}

/// The whole part of the square root of a number that is not negative.
fn isqrt(n: i128) -> i128 {
    if n < 2 {
        return n.max(0);
    }
    let mut x = (n as f64).sqrt() as i128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// The pairs of openings with a gap of more than 0 and less than `min_bridge_nm`, in the order of
/// the positions of the pair. A limit of 0 or less finds nothing.
pub fn mask_slivers(openings: &[MaskOpening], min_bridge_nm: i64) -> Vec<MaskSliver> {
    if min_bridge_nm <= 0 || openings.len() < 2 {
        return Vec::new();
    }
    let limit = i128::from(min_bridge_nm);
    let limit_squared = limit * limit;
    let bounds: Vec<(i128, i128, i128, i128)> = openings.iter().map(|o| o.bounds()).collect();
    let mut order: Vec<usize> = (0..bounds.len()).collect();
    order.sort_by_key(|&i| (bounds[i].0, i));
    let mut found = Vec::new();
    for (rank, &i) in order.iter().enumerate() {
        for &j in &order[rank + 1..] {
            // Sorted by the left edge, so the horizontal gap only grows from here on.
            if bounds[j].0 - bounds[i].2 >= limit {
                break;
            }
            let squared = gap_squared(bounds[i], bounds[j]);
            if squared > 0 && squared < limit_squared {
                found.push(MaskSliver {
                    first: i.min(j),
                    second: i.max(j),
                    gap_nm: isqrt(squared) as u64,
                });
            }
        }
    }
    found.sort_by_key(|s| (s.first, s.second));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x1: i64, y1: i64, x2: i64, y2: i64) -> MaskOpening {
        MaskOpening {
            x1_nm: x1,
            y1_nm: y1,
            x2_nm: x2,
            y2_nm: y2,
        }
    }

    #[test]
    fn a_narrow_strip_is_found() {
        let list = [rect(0, 0, 1000, 1000), rect(1050, 0, 2000, 1000)];
        let found = mask_slivers(&list, 100);
        assert_eq!(
            found,
            vec![MaskSliver {
                first: 0,
                second: 1,
                gap_nm: 50
            }]
        );
    }

    #[test]
    fn a_strip_as_wide_as_the_limit_is_fine() {
        let list = [rect(0, 0, 1000, 1000), rect(1100, 0, 2000, 1000)];
        assert!(mask_slivers(&list, 100).is_empty());
        assert_eq!(mask_slivers(&list, 101).len(), 1);
    }

    #[test]
    fn touching_and_overlapping_openings_are_one_opening() {
        let list = [
            rect(0, 0, 1000, 1000),
            rect(1000, 0, 2000, 1000),
            rect(500, 500, 1500, 1500),
        ];
        assert!(mask_slivers(&list, 5000).is_empty());
    }

    #[test]
    fn the_gap_at_a_corner_is_the_distance_between_the_corners() {
        // 30 across and 40 down is 50 away.
        let list = [rect(0, 0, 1000, 1000), rect(1030, 1040, 2000, 2000)];
        let found = mask_slivers(&list, 60);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].gap_nm, 50);
        assert!(mask_slivers(&list, 50).is_empty());
    }

    #[test]
    fn corners_in_any_order_and_a_limit_that_is_not_positive() {
        let list = [rect(1000, 1000, 0, 0), rect(2000, 1000, 1050, 0)];
        assert_eq!(mask_slivers(&list, 100).len(), 1);
        assert!(mask_slivers(&list, 0).is_empty());
        assert!(mask_slivers(&list, -5).is_empty());
        assert!(mask_slivers(&list[..1], 100).is_empty());
        assert!(mask_slivers(&[], 100).is_empty());
    }

    #[test]
    fn the_far_corners_of_the_range_do_not_overflow() {
        let edge = 1_073_741_824;
        let list = [rect(-edge, -edge, -1, -1), rect(0, 0, edge, edge)];
        let found = mask_slivers(&list, 10);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].gap_nm, 1);
        assert_eq!(mask_slivers(&list, 1).len(), 0);
    }

    #[test]
    fn the_square_root_is_exact_around_squares() {
        for n in [
            0_i128,
            1,
            2,
            3,
            4,
            8,
            9,
            10,
            99,
            100,
            101,
            1 << 40,
            (1 << 40) - 1,
        ] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "{n}");
        }
        assert_eq!(isqrt(-5), 0);
    }
}
