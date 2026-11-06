// SPDX-License-Identifier: GPL-3.0-or-later
//! The lines of a non-uniform rectangular mesh. The positions that must be a line (the edges of the
//! conductors, the limits of the layers, the walls) are given; between two of them the cells grow
//! from the fine step at an edge that asks for it, by a ratio, up to the largest step. The
//! construction uses only additions, multiplications and comparisons in `f64` in a fixed order, so
//! the same section gives the same mesh on every machine.

use crate::codes::{FieldError, MESH_TOO_LARGE};

/// The widths of the cells that fill an interval of the given length.
///
/// The cells grow from the end `a` (starting at `h_fine` when `fine_a` is true, else at `h_max`)
/// and from the end `b` the same way, by `grade` for each cell, never above `h_max`; the side whose
/// next cell is smaller is advanced, the left one on a tie, until the two meet. The last cell takes
/// what is left when that is at most one and a half cells. `None` when there would be more than
/// `budget` cells.
pub fn subdivide(
    length: f64,
    fine_a: bool,
    fine_b: bool,
    h_fine: f64,
    h_max: f64,
    grade: f64,
    budget: usize,
) -> Option<Vec<f64>> {
    let mut left: Vec<f64> = Vec::new();
    let mut right: Vec<f64> = Vec::new();
    let mut at_left = 0.0_f64;
    let mut at_right = length;
    let mut next_left = if fine_a { h_fine } else { h_max };
    let mut next_right = if fine_b { h_fine } else { h_max };
    loop {
        let gap = at_right - at_left;
        if gap <= 0.0 {
            break;
        }
        if left.len() + right.len() > budget {
            return None;
        }
        if next_left <= next_right {
            let step = next_left.min(h_max);
            if gap <= 1.5 * step {
                left.push(gap);
                break;
            }
            left.push(step);
            at_left += step;
            next_left = (next_left * grade).min(h_max);
        } else {
            let step = next_right.min(h_max);
            if gap <= 1.5 * step {
                right.push(gap);
                break;
            }
            right.push(step);
            at_right -= step;
            next_right = (next_right * grade).min(h_max);
        }
    }
    let mut widths = left;
    widths.extend(right.into_iter().rev());
    Some(merge_small(widths, 0.2 * h_fine.min(h_max)))
}

/// A cell narrower than `smallest` is joined to its neighbour, so no cell has a width that is
/// only a rounding error.
fn merge_small(widths: Vec<f64>, smallest: f64) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(widths.len());
    for w in widths {
        if w < smallest {
            if let Some(last) = out.last_mut() {
                *last += w;
                continue;
            }
        }
        out.push(w);
    }
    if out.len() > 1 && out[0] < smallest {
        let first = out.remove(0);
        out[0] += first;
    }
    out
}

/// The coordinates of the lines of a mesh that has a line at every one of `breaks` (sorted, at
/// least two). `fine[i]` says that the cells next to `breaks[i]` start at `h_fine`.
pub fn grade_lines(
    breaks: &[f64],
    fine: &[bool],
    h_fine: f64,
    h_max: f64,
    grade: f64,
    budget: usize,
) -> Result<Vec<f64>, FieldError> {
    let too_large = || {
        FieldError::new(
            MESH_TOO_LARGE,
            format!("the mesh would have more than {budget} lines in one direction"),
        )
    };
    let h_fine = h_fine.min(h_max);
    let mut lines: Vec<f64> = Vec::new();
    for (index, pair) in breaks.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let fine_a = fine.get(index).copied().unwrap_or(false);
        let fine_b = fine.get(index + 1).copied().unwrap_or(false);
        let widths =
            subdivide(b - a, fine_a, fine_b, h_fine, h_max, grade, budget).ok_or_else(too_large)?;
        if lines.is_empty() {
            lines.push(a);
        }
        let mut at = a;
        let last = widths.len().saturating_sub(1);
        for (i, w) in widths.iter().enumerate() {
            if i == last {
                at = b;
            } else {
                at += *w;
            }
            lines.push(at);
        }
        if lines.len() > budget {
            return Err(too_large());
        }
    }
    Ok(lines)
}

/// `n + 1` equally spaced lines from `a` to `b`.
pub fn uniform_lines(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / n as f64;
    let mut lines: Vec<f64> = (0..n).map(|i| a + step * i as f64).collect();
    lines.push(b);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cells_fill_the_interval_exactly_and_grow_away_from_a_fine_edge() {
        let widths = subdivide(1.0, true, false, 0.01, 0.1, 1.5, 10_000).unwrap();
        let total: f64 = widths.iter().sum();
        assert!((total - 1.0).abs() < 1.0e-12, "{total}");
        assert!((widths[0] - 0.01).abs() < 1.0e-15);
        assert!(widths[1] > widths[0]);
        assert!(widths.iter().all(|w| *w > 0.0 && *w <= 0.15 + 1.0e-12));
    }

    #[test]
    fn two_fine_edges_start_fine_at_both_ends() {
        let widths = subdivide(1.0, true, true, 0.02, 0.2, 1.3, 10_000).unwrap();
        let total: f64 = widths.iter().sum();
        assert!((total - 1.0).abs() < 1.0e-12, "{total}");
        assert!((widths[0] - 0.02).abs() < 1.0e-15);
        let last = widths[widths.len() - 1];
        assert!((last - 0.02).abs() < 1.0e-15, "{last}");
    }

    #[test]
    fn a_short_interval_is_one_cell() {
        let widths = subdivide(0.001, true, true, 0.01, 0.1, 1.2, 100).unwrap();
        assert_eq!(widths.len(), 1);
        assert!((widths[0] - 0.001).abs() < 1.0e-18);
    }

    #[test]
    fn the_budget_stops_a_mesh_that_would_be_too_large() {
        assert!(subdivide(1.0, true, true, 1.0e-6, 1.0e-6, 1.0, 100).is_none());
        let breaks = [0.0, 1.0];
        let error = grade_lines(&breaks, &[true, true], 1.0e-6, 1.0e-6, 1.0, 100).unwrap_err();
        assert_eq!(error.code, MESH_TOO_LARGE);
    }

    #[test]
    fn the_lines_pass_through_every_break() {
        let breaks = [0.0, 0.3, 0.35, 1.0];
        let fine = [false, true, true, false];
        let lines = grade_lines(&breaks, &fine, 0.01, 0.1, 1.25, 10_000).unwrap();
        for b in breaks {
            assert!(lines.contains(&b), "{b}");
        }
        for pair in lines.windows(2) {
            assert!(pair[1] > pair[0]);
        }
        assert_eq!(lines.first().copied(), Some(0.0));
        assert_eq!(lines.last().copied(), Some(1.0));
    }

    #[test]
    fn the_same_input_gives_the_same_lines() {
        let breaks = [0.0, 0.3, 0.35, 1.0];
        let fine = [false, true, true, false];
        let a = grade_lines(&breaks, &fine, 0.01, 0.1, 1.25, 10_000).unwrap();
        let b = grade_lines(&breaks, &fine, 0.01, 0.1, 1.25, 10_000).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn uniform_lines_are_equally_spaced() {
        let lines = uniform_lines(1.0, 2.0, 4);
        assert_eq!(lines, vec![1.0, 1.25, 1.5, 1.75, 2.0]);
    }

    #[test]
    fn a_cell_that_is_only_a_remainder_is_joined_to_its_neighbour() {
        let merged = merge_small(vec![0.1, 0.1, 1.0e-9], 0.02);
        assert_eq!(merged.len(), 2);
        let first = merge_small(vec![1.0e-9, 0.1, 0.1], 0.02);
        assert_eq!(first.len(), 2);
        assert!((first[0] - (0.1 + 1.0e-9)).abs() < 1.0e-15);
    }
}
