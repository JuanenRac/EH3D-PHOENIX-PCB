// SPDX-License-Identifier: GPL-3.0-or-later
//! A uniform grid that finds the neighbours of a box without comparing every pair.
//!
//! The grid only chooses which items are compared. Every caller applies the same exact test to
//! the candidates, so the result is the one of the naive search that compares every pair; the
//! tests check that. The order of the candidates is fixed (ascending index).

use std::collections::BTreeMap;

/// A box: `[min x, min y, max x, max y]`, in nanometres, edges included.
pub(crate) type Bx = [i128; 4];

/// The most cells one box may cover before it is kept in the list of big items.
const MAX_CELLS_PER_ITEM: i128 = 4096;

/// Do the two boxes touch when the first is made `margin` larger on every side?
pub(crate) fn overlap(a: &Bx, b: &Bx, margin: i128) -> bool {
    a[0] - margin <= b[2] && b[0] <= a[2] + margin && a[1] - margin <= b[3] && b[1] <= a[3] + margin
}

/// A box made `margin` larger on every side.
pub(crate) fn inflate(b: &Bx, margin: i128) -> Bx {
    [b[0] - margin, b[1] - margin, b[2] + margin, b[3] + margin]
}

/// Which items are near a box.
pub(crate) struct Index {
    cell: i128,
    cells: BTreeMap<(i128, i128), Vec<usize>>,
    big: Vec<usize>,
    count: usize,
    naive: bool,
}

fn range_of(b: &Bx, cell: i128) -> (i128, i128, i128, i128) {
    (
        b[0].div_euclid(cell),
        b[1].div_euclid(cell),
        b[2].div_euclid(cell),
        b[3].div_euclid(cell),
    )
}

fn cell_count(r: (i128, i128, i128, i128)) -> i128 {
    (r.2 - r.0 + 1).saturating_mul(r.3 - r.1 + 1)
}

impl Index {
    /// The search that compares every item: every query gives every item.
    pub(crate) fn naive(count: usize) -> Index {
        Index {
            cell: 1,
            cells: BTreeMap::new(),
            big: Vec::new(),
            count,
            naive: true,
        }
    }

    /// The grid over the boxes of the items, with cells of `cell` nanometres.
    pub(crate) fn grid(boxes: &[Bx], cell: i128) -> Index {
        let cell = cell.max(1);
        let mut cells: BTreeMap<(i128, i128), Vec<usize>> = BTreeMap::new();
        let mut big = Vec::new();
        for (i, b) in boxes.iter().enumerate() {
            let r = range_of(b, cell);
            if cell_count(r) > MAX_CELLS_PER_ITEM {
                big.push(i);
                continue;
            }
            for cx in r.0..=r.2 {
                for cy in r.1..=r.3 {
                    cells.entry((cx, cy)).or_default().push(i);
                }
            }
        }
        Index {
            cell,
            cells,
            big,
            count: boxes.len(),
            naive: false,
        }
    }

    /// The items that may overlap the box, in ascending order and without repeats. The list
    /// holds every item whose box overlaps the query and may hold others.
    pub(crate) fn candidates(&self, query: &Bx) -> Vec<usize> {
        if self.naive {
            return (0..self.count).collect();
        }
        let r = range_of(query, self.cell);
        if cell_count(r) > MAX_CELLS_PER_ITEM {
            return (0..self.count).collect();
        }
        let mut found: Vec<usize> = self.big.clone();
        for cx in r.0..=r.2 {
            for cy in r.1..=r.3 {
                if let Some(list) = self.cells.get(&(cx, cy)) {
                    found.extend_from_slice(list);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}

/// Every pair `(i, j)` with `i < j` whose boxes are within `margin` of each other, in ascending
/// order.
pub(crate) fn pairs(index: &Index, boxes: &[Bx], margin: i128) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        for j in index.candidates(&inflate(b, margin)) {
            if j > i && overlap(b, &boxes[j], margin) {
                out.push((i, j));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_and_the_naive_search_give_the_same_pairs() {
        let mut boxes = Vec::new();
        let mut seed: u64 = 12345;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            i128::from((seed >> 33) % 100_000)
        };
        for _ in 0..200 {
            let (x, y, w, h) = (next(), next(), next() % 3000, next() % 3000);
            boxes.push([x, y, x + w, y + h]);
        }
        // one long item that covers many cells
        boxes.push([0, 50_000, 100_000, 50_100]);
        for margin in [0, 10, 700, 5000] {
            let naive = pairs(&Index::naive(boxes.len()), &boxes, margin);
            for cell in [1, 97, 1000, 100_000_000] {
                let grid = pairs(&Index::grid(&boxes, cell), &boxes, margin);
                assert_eq!(naive, grid, "margin {margin}, cell {cell}");
            }
        }
    }

    #[test]
    fn boxes_that_touch_by_an_edge_are_neighbours() {
        let boxes = [[0, 0, 10, 10], [10, 0, 20, 10], [22, 0, 30, 10]];
        let found = pairs(&Index::grid(&boxes, 4), &boxes, 0);
        assert_eq!(found, vec![(0, 1)]);
        let wider = pairs(&Index::grid(&boxes, 4), &boxes, 2);
        assert_eq!(wider, vec![(0, 1), (1, 2)]);
    }
}
