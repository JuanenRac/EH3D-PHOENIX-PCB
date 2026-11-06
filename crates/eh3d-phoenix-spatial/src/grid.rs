// SPDX-License-Identifier: GPL-3.0-or-later
//! A uniform grid that finds the neighbours of a box without comparing every pair.
//!
//! The grid only chooses which items are compared. Every caller applies the same exact test to
//! the candidates, so the result is the one of the naive search that compares every pair; the
//! tests check that. The order of the candidates is fixed (ascending index). The index counts the
//! candidates it hands out ([`Index::examined`]): for items that are spread out the count grows
//! with the number of items, for the naive search it grows with its square, and a test can tell
//! the two apart without a clock.

use std::cell::Cell;
use std::collections::BTreeMap;

use crate::codes::{SpatialError, GRID_TOO_LARGE, TOO_MANY_ITEMS};
use crate::config::SpatialConfig;

/// A box: `[min x, min y, max x, max y]`, in nanometres, edges included.
pub type Bx = [i128; 4];

/// The cell of the grid that holds a point, and the cells of the corners of a box.
type CellRange = (i128, i128, i128, i128);

/// How many cells wide the whole extent of the items is cut into at least, when the cell is
/// chosen from the boxes.
const AUTO_CELLS_ACROSS: i128 = 4096;

/// Do the two boxes touch when the first is made `margin` larger on every side?
pub fn overlap(a: &Bx, b: &Bx, margin: i128) -> bool {
    a[0] - margin <= b[2] && b[0] <= a[2] + margin && a[1] - margin <= b[3] && b[1] <= a[3] + margin
}

/// A box made `margin` larger on every side.
pub fn inflate(b: &Bx, margin: i128) -> Bx {
    [b[0] - margin, b[1] - margin, b[2] + margin, b[3] + margin]
}

fn range_of(b: &Bx, cell: i128) -> CellRange {
    (
        b[0].min(b[2]).div_euclid(cell),
        b[1].min(b[3]).div_euclid(cell),
        b[0].max(b[2]).div_euclid(cell),
        b[1].max(b[3]).div_euclid(cell),
    )
}

fn cell_count(r: CellRange) -> i128 {
    (r.2 - r.0 + 1).saturating_mul(r.3 - r.1 + 1)
}

/// The size of a cell chosen from the boxes: twice the mean of their longer sides, and not less
/// than the extent cut into [`AUTO_CELLS_ACROSS`] parts.
fn auto_cell(boxes: &[Bx]) -> i128 {
    if boxes.is_empty() {
        return 1;
    }
    let mut sum: i128 = 0;
    let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (i128::MAX, i128::MAX, i128::MIN, i128::MIN);
    for b in boxes {
        let longer = (b[2] - b[0]).max(b[3] - b[1]).max(0);
        sum = sum.saturating_add(longer + 1);
        lo_x = lo_x.min(b[0]);
        lo_y = lo_y.min(b[1]);
        hi_x = hi_x.max(b[2]);
        hi_y = hi_y.max(b[3]);
    }
    let mean = sum / boxes.len() as i128;
    let extent = (hi_x - lo_x).max(hi_y - lo_y).max(0);
    let coarse = extent / AUTO_CELLS_ACROSS + 1;
    mean.saturating_mul(2).max(coarse).max(1)
}

/// Which items are near a box.
#[derive(Debug)]
pub struct Index {
    cell: i128,
    cells: BTreeMap<(i128, i128), Vec<usize>>,
    big: Vec<usize>,
    count: usize,
    naive: bool,
    entries: u64,
    per_item_limit: i128,
    examined: Cell<u64>,
}

impl Index {
    /// The search that compares every item: every query gives every item.
    pub fn naive(count: usize) -> Index {
        Index {
            cell: 1,
            cells: BTreeMap::new(),
            big: Vec::new(),
            count,
            naive: true,
            entries: 0,
            per_item_limit: 0,
            examined: Cell::new(0),
        }
    }

    /// The grid over the boxes of the items. The cells are `config.cell_nm` nanometres, or chosen
    /// from the boxes when that is 0. It is refused (and nothing is built) when there are more
    /// items than `config.max_items` or when the cells that the items cover add up to more than
    /// `config.max_cells`.
    pub fn grid(boxes: &[Bx], config: &SpatialConfig) -> Result<Index, SpatialError> {
        if boxes.len() as u64 > config.max_items {
            return Err(SpatialError::new(
                TOO_MANY_ITEMS,
                format!(
                    "{} items, the limit max_items is {}",
                    boxes.len(),
                    config.max_items
                ),
            ));
        }
        let cell = if config.cell_nm > 0 {
            i128::from(config.cell_nm)
        } else {
            auto_cell(boxes)
        };
        let per_item_limit = i128::from(config.max_cells_per_item);
        let mut cells: BTreeMap<(i128, i128), Vec<usize>> = BTreeMap::new();
        let mut big = Vec::new();
        let mut entries: u64 = 0;
        for (i, b) in boxes.iter().enumerate() {
            let r = range_of(b, cell);
            let covered = cell_count(r);
            if covered > per_item_limit {
                big.push(i);
                continue;
            }
            // covered is at most max_cells_per_item, which is at most 2^20
            entries = entries.saturating_add(covered as u64);
            if entries > config.max_cells {
                let limit = config.max_cells;
                return Err(SpatialError::new(
                    GRID_TOO_LARGE,
                    format!("more than {limit} entries of cells at {cell} nm, the limit max_cells"),
                ));
            }
            for cx in r.0..=r.2 {
                for cy in r.1..=r.3 {
                    cells.entry((cx, cy)).or_default().push(i);
                }
            }
        }
        Ok(Index {
            cell,
            cells,
            big,
            count: boxes.len(),
            naive: false,
            entries,
            per_item_limit,
            examined: Cell::new(0),
        })
    }

    /// The items that may overlap the box, in ascending order and without repeats. The list
    /// holds every item whose box overlaps the query and may hold others. The length of the list
    /// is added to [`Index::examined`].
    pub fn candidates(&self, query: &Bx) -> Vec<usize> {
        let found = self.collect(query);
        self.examined
            .set(self.examined.get().saturating_add(found.len() as u64));
        found
    }

    fn collect(&self, query: &Bx) -> Vec<usize> {
        if self.naive {
            return (0..self.count).collect();
        }
        let r = range_of(query, self.cell);
        let mut found: Vec<usize> = self.big.clone();
        if cell_count(r) > self.per_item_limit {
            // too many cells to look at one by one: look at the cells that hold items, in the
            // columns of the query
            for (&(_, cy), list) in self.cells.range((r.0, i128::MIN)..=(r.2, i128::MAX)) {
                if (r.1..=r.3).contains(&cy) {
                    found.extend_from_slice(list);
                }
            }
        } else {
            for cx in r.0..=r.2 {
                for cy in r.1..=r.3 {
                    if let Some(list) = self.cells.get(&(cx, cy)) {
                        found.extend_from_slice(list);
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// How many candidates the queries have handed out so far: the number of comparisons that the
    /// callers have had to make.
    pub fn examined(&self) -> u64 {
        self.examined.get()
    }

    /// The number of items the index was made for.
    pub fn count(&self) -> usize {
        self.count
    }

    /// The size of a cell in nanometres (1 for the naive search).
    pub fn cell_nm(&self) -> i128 {
        self.cell
    }

    /// The cells that hold at least one item.
    pub fn cells_used(&self) -> usize {
        self.cells.len()
    }

    /// The entries of the grid: one for each cell that an item covers.
    pub fn entries(&self) -> u64 {
        self.entries
    }

    /// The items that cover too many cells to be in the grid, and are looked at by every query.
    pub fn big_items(&self) -> usize {
        self.big.len()
    }

    /// True for the search that compares every item.
    pub fn is_naive(&self) -> bool {
        self.naive
    }
}

/// Every pair `(i, j)` with `i < j` whose boxes are within `margin` of each other, in ascending
/// order.
pub fn pairs(index: &Index, boxes: &[Bx], margin: i128) -> Vec<(usize, usize)> {
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

    fn config(cell_nm: i64) -> SpatialConfig {
        SpatialConfig {
            cell_nm,
            ..SpatialConfig::default()
        }
    }

    #[test]
    fn boxes_that_touch_by_an_edge_are_neighbours() {
        let boxes = [[0, 0, 10, 10], [10, 0, 20, 10], [22, 0, 30, 10]];
        let index = Index::grid(&boxes, &config(4)).unwrap();
        assert_eq!(pairs(&index, &boxes, 0), vec![(0, 1)]);
        assert_eq!(pairs(&index, &boxes, 2), vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn the_naive_search_hands_out_every_item_and_counts_them() {
        let index = Index::naive(7);
        assert_eq!(index.candidates(&[0, 0, 1, 1]), (0..7).collect::<Vec<_>>());
        assert_eq!(index.examined(), 7);
        assert!(index.is_naive());
        assert_eq!(
            (index.count(), index.cells_used(), index.entries()),
            (7, 0, 0)
        );
    }

    #[test]
    fn the_automatic_cell_follows_the_boxes() {
        let small: Vec<Bx> = (0..10).map(|i| [i * 100, 0, i * 100 + 10, 10]).collect();
        let index = Index::grid(&small, &config(0)).unwrap();
        assert!(index.cell_nm() >= 1);
        assert_eq!(auto_cell(&[]), 1);
        let far: Vec<Bx> = vec![[0, 0, 1, 1], [40_960_000, 0, 40_960_001, 1]];
        assert!(auto_cell(&far) >= 10_000);
    }

    #[test]
    fn an_item_that_covers_too_many_cells_is_kept_apart() {
        let boxes = [[0, 0, 100_000_000, 10], [5, 5, 6, 6]];
        let index = Index::grid(&boxes, &config(10)).unwrap();
        assert_eq!(index.big_items(), 1);
        assert_eq!(index.candidates(&[5, 5, 6, 6]), vec![0, 1]);
        // a query over more cells than the limit looks at the cells that hold items
        let far = [
            [0, 0, 1, 1],
            [90_000_000, 90_000_000, 90_000_001, 90_000_001],
        ];
        let index = Index::grid(&far, &config(10)).unwrap();
        assert_eq!(
            index.candidates(&[0, 0, 100_000_000, 100_000_000]),
            vec![0, 1]
        );
        assert_eq!(index.candidates(&[0, 0, 100_000_000, 5]), vec![0]);
    }
}
