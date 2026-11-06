// SPDX-License-Identifier: GPL-3.0-or-later
//! The index against the search of every pair, on boxes made from fixed seeds, and the count of
//! comparisons that tells linear work from quadratic work without a clock.

use eh3d_phoenix_spatial::codes::{GRID_TOO_LARGE, TOO_MANY_ITEMS};
use eh3d_phoenix_spatial::testgen::{aligned_boxes, lattice_boxes, scattered_boxes};
use eh3d_phoenix_spatial::{inflate, overlap, pairs, Bx, Index, SpatialConfig};

/// The old search: every pair is compared.
fn reference_pairs(boxes: &[Bx], margin: i128) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, a) in boxes.iter().enumerate() {
        for (j, b) in boxes.iter().enumerate().skip(i + 1) {
            if overlap(a, b, margin) {
                out.push((i, j));
            }
        }
    }
    out
}

fn config(cell_nm: i64) -> SpatialConfig {
    SpatialConfig {
        cell_nm,
        ..SpatialConfig::default()
    }
}

fn check_all_cells(boxes: &[Bx], what: &str) {
    for margin in [0, 1, 700, 5000] {
        let expected = reference_pairs(boxes, margin);
        for cell in [0, 1, 97, 1000, 65_536, 100_000_000] {
            let index = Index::grid(boxes, &config(cell)).unwrap();
            assert_eq!(
                pairs(&index, boxes, margin),
                expected,
                "{what}, margin {margin}, cell {cell}"
            );
        }
        let naive = Index::naive(boxes.len());
        assert_eq!(pairs(&naive, boxes, margin), expected, "{what}, naive");
    }
}

#[test]
fn small_medium_and_large_sets_give_the_pairs_of_the_old_search() {
    check_all_cells(&scattered_boxes(1, 40, 20_000, 3_000), "small");
    check_all_cells(&scattered_boxes(2, 400, 100_000, 3_000), "medium");
    check_all_cells(&scattered_boxes(3, 2_500, 400_000, 4_000), "large");
}

#[test]
fn the_queries_give_the_items_in_ascending_order_without_repeats() {
    let boxes = scattered_boxes(4, 600, 60_000, 3_000);
    let index = Index::grid(&boxes, &config(500)).unwrap();
    for b in &boxes {
        let found = index.candidates(&inflate(b, 300));
        assert!(found.windows(2).all(|w| w[0] < w[1]));
    }
}

#[test]
fn boxes_on_the_border_of_a_cell_are_found() {
    for (seed, cell) in [(10, 64), (11, 1000), (12, 7)] {
        let boxes = aligned_boxes(seed, 500, cell, 12);
        for margin in [0, 1, i128::from(cell)] {
            let expected = reference_pairs(&boxes, margin);
            let index = Index::grid(&boxes, &config(cell)).unwrap();
            assert_eq!(pairs(&index, &boxes, margin), expected, "cell {cell}");
            assert!(!expected.is_empty());
        }
    }
}

#[test]
fn negative_coordinates_duplicates_and_long_segments_are_found() {
    let mut boxes = scattered_boxes(20, 300, 5_000, 400);
    boxes.push([-9_000, -9_000, -8_900, -8_900]);
    boxes.push([-9_000, -9_000, -8_900, -8_900]);
    // a segment that crosses every cell of the board
    boxes.push([-10_000, 0, 10_000, 1]);
    boxes.push([0, -10_000, 1, 10_000]);
    check_all_cells(&boxes, "negative, duplicate and long");
}

#[test]
fn the_extremes_of_i64_do_not_overflow() {
    let big = i128::from(i64::MAX);
    let small = i128::from(i64::MIN);
    let mut boxes: Vec<Bx> = vec![
        [small, small, small + 10, small + 10],
        [big - 10, big - 10, big, big],
        [small, small, big, big],
        [0, 0, 5, 5],
        [-5, -5, 0, 0],
        [big - 10, small, big, small + 10],
    ];
    boxes.extend(scattered_boxes(30, 50, 1_000, 100));
    for margin in [0, 3, big] {
        let expected = reference_pairs(&boxes, margin);
        for cell in [0, 1, 1_000_000, 1 << 40] {
            let index = Index::grid(&boxes, &config(cell)).unwrap();
            assert_eq!(pairs(&index, &boxes, margin), expected, "cell {cell}");
        }
    }
}

/// The comparisons of the index for `count` boxes on a lattice, and of the naive search.
fn comparisons(count: usize) -> (u64, u64) {
    let boxes = lattice_boxes(7, count, 1_000, 10_000);
    let index = Index::grid(&boxes, &config(4_000)).unwrap();
    let found = pairs(&index, &boxes, 500);
    assert_eq!(found, reference_pairs(&boxes, 500));
    let naive = Index::naive(boxes.len());
    let _ = pairs(&naive, &boxes, 500);
    (index.examined(), naive.examined())
}

#[test]
fn the_comparisons_grow_with_the_items_and_not_with_their_square() {
    for count in [500_usize, 1_000, 2_000, 4_000] {
        let (grid, naive) = comparisons(count);
        let n = count as u64;
        // each box is near itself and a few others at most
        assert!(grid <= 4 * n, "{count}: {grid} comparisons");
        assert_eq!(naive, n * n);
    }
    // doubling the items does not much more than double the comparisons
    let (a, _) = comparisons(1_000);
    let (b, _) = comparisons(2_000);
    assert!(b <= 3 * a, "{a} then {b}");
}

#[test]
fn a_grid_that_needs_too_many_cells_or_items_is_refused_with_its_code() {
    let boxes = lattice_boxes(1, 100, 1_000, 2_000);
    let few_cells = SpatialConfig {
        cell_nm: 100,
        max_cells: 50,
        ..SpatialConfig::default()
    };
    assert_eq!(
        Index::grid(&boxes, &few_cells).unwrap_err().code,
        GRID_TOO_LARGE
    );
    let few_items = SpatialConfig {
        max_items: 99,
        ..SpatialConfig::default()
    };
    assert_eq!(
        Index::grid(&boxes, &few_items).unwrap_err().code,
        TOO_MANY_ITEMS
    );
    let enough = SpatialConfig {
        max_items: 100,
        ..SpatialConfig::default()
    };
    assert!(Index::grid(&boxes, &enough).is_ok());
}
