// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper of a layer by region of the board. A board is bent by heat when one part of a
//! layer carries much more copper than another, so a maker wants to know how the copper is spread
//! over the board and not only how much there is. The box of the outline is cut into a grid of
//! `region_columns` by `region_rows` cells, the copper of each layer is added up in each cell and
//! the share of the cell that the copper covers is the coverage of the cell.
//!
//! The areas are split between the cells like this:
//!
//! * a **zone** (or a cutout of a zone) is clipped to each cell, so its area is shared exactly
//!   between the cells it covers;
//! * a **track** is shared between the cells in proportion to the length of each of its pieces
//!   that lies in each cell, and the two round ends are put in the cells of the two ends of the
//!   track;
//! * a **pad** and a **via** are put whole in the cell that holds their centre.
//!
//! Nothing here knows the board model: the functions take points in nanometres and give areas as
//! floating point numbers that the caller rounds, so the same board gives the same numbers.

/// A cell of the grid: the least and the greatest x and y, in nanometres.
type Rect = (i64, i64, i64, i64);

/// A grid over the box of the outline of a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Grid {
    /// The edges between the columns, from the least x to the greatest.
    xs: Vec<i64>,
    /// The edges between the rows, from the least y to the greatest.
    ys: Vec<i64>,
}

impl Grid {
    /// The grid over the box from `min` to `max` (x, y) cut into `columns` by `rows` cells. There
    /// is none when the box has no width or no height or a count is zero.
    pub(crate) fn new(
        min: (i64, i64),
        max: (i64, i64),
        columns: usize,
        rows: usize,
    ) -> Option<Grid> {
        if columns == 0 || rows == 0 || max.0 <= min.0 || max.1 <= min.1 {
            return None;
        }
        let edges = |lo: i64, hi: i64, n: usize| -> Vec<i64> {
            (0..=n)
                .map(|i| lo + (i128::from(hi - lo) * i as i128 / n as i128) as i64)
                .collect()
        };
        Some(Grid {
            xs: edges(min.0, max.0, columns),
            ys: edges(min.1, max.1, rows),
        })
    }

    /// The number of columns.
    pub(crate) fn columns(&self) -> usize {
        self.xs.len() - 1
    }

    /// The number of rows.
    pub(crate) fn rows(&self) -> usize {
        self.ys.len() - 1
    }

    /// The number of cells. The cell of column `c` and row `r` (both from 0, the row from the
    /// least y) is number `r * columns + c`.
    pub(crate) fn cells(&self) -> usize {
        self.columns() * self.rows()
    }

    fn rect(&self, index: usize) -> Rect {
        let (c, r) = (index % self.columns(), index / self.columns());
        (self.xs[c], self.ys[r], self.xs[c + 1], self.ys[r + 1])
    }

    fn column_of(&self, x: i64) -> usize {
        (1..self.columns()).take_while(|&i| x >= self.xs[i]).count()
    }

    fn row_of(&self, y: i64) -> usize {
        (1..self.rows()).take_while(|&i| y >= self.ys[i]).count()
    }

    /// The number of the cell that holds a point; a point outside the box is in the nearest cell
    /// and one on an edge between two cells is in the one with the greater index.
    pub(crate) fn cell_of(&self, x: i64, y: i64) -> usize {
        self.row_of(y) * self.columns() + self.column_of(x)
    }

    /// The area of a polygon inside each cell, in square nanometres, by the number of the cell.
    /// A polygon with fewer than three corners encloses nothing.
    pub(crate) fn polygon_areas(&self, polygon: &[(i64, i64)]) -> Vec<f64> {
        let mut areas = vec![0.0; self.cells()];
        if polygon.len() < 3 {
            return areas;
        }
        let min_x = polygon.iter().map(|p| p.0).min().unwrap_or(0);
        let max_x = polygon.iter().map(|p| p.0).max().unwrap_or(0);
        let min_y = polygon.iter().map(|p| p.1).min().unwrap_or(0);
        let max_y = polygon.iter().map(|p| p.1).max().unwrap_or(0);
        for (index, area) in areas.iter_mut().enumerate() {
            let rect = self.rect(index);
            if max_x < rect.0 || min_x > rect.2 || max_y < rect.1 || min_y > rect.3 {
                continue;
            }
            *area = clipped_area(polygon, rect);
        }
        areas
    }

    /// How a piece of track from `a` to `b` is shared between the cells: the number of each cell it
    /// crosses and the fraction of its length inside it. The fractions add up to 1; a piece that
    /// has no length, or lies outside the box, is wholly in the cell that holds `a`.
    pub(crate) fn segment_shares(&self, a: (i64, i64), b: (i64, i64)) -> Vec<(usize, f64)> {
        let (c0, c1) = (self.column_of(a.0.min(b.0)), self.column_of(a.0.max(b.0)));
        let (r0, r1) = (self.row_of(a.1.min(b.1)), self.row_of(a.1.max(b.1)));
        let mut shares = Vec::new();
        let mut total = 0.0;
        for r in r0..=r1 {
            for c in c0..=c1 {
                let index = r * self.columns() + c;
                let fraction = fraction_inside(a, b, self.rect(index));
                if fraction > 0.0 {
                    shares.push((index, fraction));
                    total += fraction;
                }
            }
        }
        if total <= 0.0 {
            return vec![(self.cell_of(a.0, a.1), 1.0)];
        }
        for share in &mut shares {
            share.1 /= total;
        }
        shares
    }
}

/// Whether a point is on the inside of one of the four sides of a rectangle (side 0 is the left,
/// 1 the right, 2 the bottom and 3 the top).
fn inside(side: usize, p: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    match side {
        0 => p.0 >= r.0,
        1 => p.0 <= r.2,
        2 => p.1 >= r.1,
        _ => p.1 <= r.3,
    }
}

/// Where the piece from `p` to `q` meets the line of one side of the rectangle.
fn meet(side: usize, p: (f64, f64), q: (f64, f64), r: (f64, f64, f64, f64)) -> (f64, f64) {
    match side {
        0 | 1 => {
            let x = if side == 0 { r.0 } else { r.2 };
            let t = (x - p.0) / (q.0 - p.0);
            (x, p.1 + t * (q.1 - p.1))
        }
        _ => {
            let y = if side == 2 { r.1 } else { r.3 };
            let t = (y - p.1) / (q.1 - p.1);
            (p.0 + t * (q.0 - p.0), y)
        }
    }
}

/// The area of a polygon inside a rectangle, by clipping it against the four sides in turn
/// (Sutherland and Hodgman). A polygon that is not convex and leaves the rectangle in more than one
/// piece keeps thin joins along the sides, which have no area, so the area is right.
fn clipped_area(polygon: &[(i64, i64)], rect: Rect) -> f64 {
    let r = (rect.0 as f64, rect.1 as f64, rect.2 as f64, rect.3 as f64);
    let mut points: Vec<(f64, f64)> = polygon.iter().map(|p| (p.0 as f64, p.1 as f64)).collect();
    for side in 0..4 {
        if points.is_empty() {
            return 0.0;
        }
        let input = std::mem::take(&mut points);
        let n = input.len();
        for i in 0..n {
            let current = input[i];
            let previous = input[(i + n - 1) % n];
            let current_in = inside(side, current, r);
            if current_in != inside(side, previous, r) {
                points.push(meet(side, previous, current, r));
            }
            if current_in {
                points.push(current);
            }
        }
    }
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut twice = 0.0;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        twice += a.0 * b.1 - b.0 * a.1;
    }
    twice.abs() / 2.0
}

/// The fraction of the piece from `a` to `b` that lies inside a rectangle (Liang and Barsky). A
/// piece that is a point is wholly inside when the point is.
fn fraction_inside(a: (i64, i64), b: (i64, i64), rect: Rect) -> f64 {
    let (ax, ay) = (a.0 as f64, a.1 as f64);
    let (dx, dy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
    let (x0, y0, x1, y1) = (rect.0 as f64, rect.1 as f64, rect.2 as f64, rect.3 as f64);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [(-dx, ax - x0), (dx, x1 - ax), (-dy, ay - y0), (dy, y1 - ay)] {
        if p == 0.0 {
            if q < 0.0 {
                return 0.0;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return 0.0;
                }
                t0 = t0.max(t);
            } else {
                if t < t0 {
                    return 0.0;
                }
                t1 = t1.min(t);
            }
        }
    }
    (t1 - t0).max(0.0)
}

/// A float area as whole square nanometres, never below zero and never above `area` when `area`
/// is above zero (the copper of a cell is never counted above the area of the cell).
pub(crate) fn capped_nm2(copper: f64, area: i128) -> i128 {
    let copper = if copper.is_finite() && copper > 0.0 {
        copper.round() as i128
    } else {
        0
    };
    if area > 0 {
        copper.min(area)
    } else {
        copper
    }
}

/// The share of an area that the copper covers, in hundredths of a percent; 0 for an area that is
/// not above zero.
pub(crate) fn coverage_x100(copper_nm2: i128, area_nm2: i128) -> i64 {
    if area_nm2 <= 0 {
        return 0;
    }
    i64::try_from(copper_nm2 * 10_000 / area_nm2).unwrap_or(i64::MAX)
}

/// The least and the greatest of the coverages of the cells that have an area, or none when no
/// cell has one. `cells` is `(area, coverage)` for each cell.
pub(crate) fn spread_x100(cells: &[(i128, i64)]) -> Option<(i64, i64)> {
    let mut found: Option<(i64, i64)> = None;
    for (area, coverage) in cells {
        if *area <= 0 {
            continue;
        }
        found = Some(match found {
            None => (*coverage, *coverage),
            Some((low, high)) => (low.min(*coverage), high.max(*coverage)),
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    fn grid(columns: usize, rows: usize) -> Grid {
        Grid::new((0, 0), (20 * MM, 10 * MM), columns, rows).unwrap()
    }

    fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
        vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    }

    fn sum(values: &[f64]) -> f64 {
        values.iter().sum()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6 * b.abs().max(1.0)
    }

    #[test]
    fn a_grid_needs_a_box_and_cuts_it_into_equal_cells() {
        assert!(Grid::new((0, 0), (10, 10), 0, 2).is_none());
        assert!(Grid::new((0, 0), (10, 10), 2, 0).is_none());
        assert!(Grid::new((5, 0), (5, 10), 2, 2).is_none());
        assert!(Grid::new((0, 7), (10, 7), 2, 2).is_none());
        let g = grid(4, 2);
        assert_eq!((g.columns(), g.rows(), g.cells()), (4, 2, 8));
        assert_eq!(g.xs, vec![0, 5 * MM, 10 * MM, 15 * MM, 20 * MM]);
        assert_eq!(g.ys, vec![0, 5 * MM, 10 * MM]);
        // the last edge is the end of the box even when the box does not divide
        let odd = Grid::new((0, 0), (10, 10), 3, 1).unwrap();
        assert_eq!(odd.xs, vec![0, 3, 6, 10]);
    }

    #[test]
    fn a_point_is_in_the_cell_that_holds_it_and_outside_points_go_to_the_nearest() {
        let g = grid(2, 2);
        assert_eq!(g.cell_of(MM, MM), 0);
        assert_eq!(g.cell_of(15 * MM, MM), 1);
        assert_eq!(g.cell_of(MM, 8 * MM), 2);
        assert_eq!(g.cell_of(15 * MM, 8 * MM), 3);
        // on an edge between two cells: the greater index
        assert_eq!(g.cell_of(10 * MM, MM), 1);
        assert_eq!(g.cell_of(MM, 5 * MM), 2);
        // on the border and beyond it
        assert_eq!(g.cell_of(0, 0), 0);
        assert_eq!(g.cell_of(20 * MM, 10 * MM), 3);
        assert_eq!(g.cell_of(-5 * MM, -5 * MM), 0);
        assert_eq!(g.cell_of(50 * MM, 50 * MM), 3);
    }

    #[test]
    fn a_polygon_is_shared_between_the_cells_it_covers() {
        let g = grid(2, 2);
        // a 4 x 4 mm square across the middle: a quarter of its area in each cell
        let a = g.polygon_areas(&square(8 * MM, 3 * MM, 12 * MM, 7 * MM));
        assert_eq!(a.len(), 4);
        for cell in &a {
            assert!(close(*cell, 4.0 * MM as f64 * MM as f64), "{a:?}");
        }
        // a square inside one cell is wholly in it
        let b = g.polygon_areas(&square(MM, MM, 3 * MM, 3 * MM));
        assert!(close(b[0], 4.0 * MM as f64 * MM as f64));
        assert_eq!(&b[1..], &[0.0, 0.0, 0.0]);
        // the whole box gives the cells their own area
        let c = g.polygon_areas(&square(0, 0, 20 * MM, 10 * MM));
        for cell in &c {
            assert!(close(*cell, 50.0 * MM as f64 * MM as f64));
        }
    }

    #[test]
    fn the_cells_of_a_triangle_add_up_to_its_area_whatever_the_direction() {
        let g = grid(4, 3);
        let triangle = vec![(MM, MM), (18 * MM, 2 * MM), (7 * MM, 9 * MM)];
        let area = {
            let (a, b, c) = (triangle[0], triangle[1], triangle[2]);
            ((b.0 - a.0) as f64 * (c.1 - a.1) as f64 - (c.0 - a.0) as f64 * (b.1 - a.1) as f64)
                .abs()
                / 2.0
        };
        let forward = g.polygon_areas(&triangle);
        assert!(close(sum(&forward), area), "{} {area}", sum(&forward));
        let mut reversed = triangle.clone();
        reversed.reverse();
        assert!(close(sum(&g.polygon_areas(&reversed)), area));
    }

    #[test]
    fn a_polygon_that_is_not_convex_keeps_its_area() {
        let g = grid(2, 1);
        // an L: 10 x 10 less the square 5 x 5 in the corner, the corner is in the right cell
        let l = vec![
            (0, 0),
            (10 * MM, 0),
            (10 * MM, 5 * MM),
            (5 * MM, 5 * MM),
            (5 * MM, 10 * MM),
            (0, 10 * MM),
        ];
        let a = g.polygon_areas(&l);
        assert!(close(sum(&a), 75.0 * MM as f64 * MM as f64), "{a:?}");
        // a U that leaves the right cell in two pieces
        let u = vec![
            (0, 0),
            (20 * MM, 0),
            (20 * MM, 10 * MM),
            (15 * MM, 10 * MM),
            (15 * MM, 2 * MM),
            (12 * MM, 2 * MM),
            (12 * MM, 10 * MM),
            (0, 10 * MM),
        ];
        let b = g.polygon_areas(&u);
        let whole = 200.0 - 3.0 * 8.0;
        assert!(close(sum(&b), whole * MM as f64 * MM as f64), "{b:?}");
        assert!(close(b[0], 100.0 * MM as f64 * MM as f64));
    }

    #[test]
    fn a_polygon_with_fewer_than_three_corners_or_outside_the_grid_has_no_area() {
        let g = grid(2, 2);
        assert_eq!(sum(&g.polygon_areas(&[])), 0.0);
        assert_eq!(sum(&g.polygon_areas(&[(0, 0), (MM, MM)])), 0.0);
        assert_eq!(
            sum(&g.polygon_areas(&square(30 * MM, 30 * MM, 40 * MM, 40 * MM))),
            0.0
        );
    }

    #[test]
    fn a_piece_of_track_is_shared_in_proportion_to_its_length_in_each_cell() {
        let g = grid(2, 1);
        // from 5 mm to 15 mm along the middle line: half on each side of the edge at 10 mm
        let shares = g.segment_shares((5 * MM, 5 * MM), (15 * MM, 5 * MM));
        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].0, 0);
        assert_eq!(shares[1].0, 1);
        assert!(
            close(shares[0].1, 0.5) && close(shares[1].1, 0.5),
            "{shares:?}"
        );
        // from 2 mm to 12 mm: a fifth in the right cell
        let uneven = g.segment_shares((2 * MM, 5 * MM), (12 * MM, 5 * MM));
        assert!(
            close(uneven[0].1, 0.8) && close(uneven[1].1, 0.2),
            "{uneven:?}"
        );
        // a piece inside one cell
        let one = g.segment_shares((MM, MM), (3 * MM, 4 * MM));
        assert_eq!(one, vec![(0, 1.0)]);
    }

    #[test]
    fn a_slanted_piece_crossing_four_cells_has_shares_that_add_up_to_one() {
        let g = grid(2, 2);
        let shares = g.segment_shares((MM, MM), (19 * MM, 9 * MM));
        let total: f64 = shares.iter().map(|s| s.1).sum();
        assert!(close(total, 1.0), "{shares:?}");
        let cells: Vec<usize> = shares.iter().map(|s| s.0).collect();
        assert!(cells.contains(&0) && cells.contains(&3), "{cells:?}");
    }

    #[test]
    fn a_point_a_piece_outside_the_box_and_a_piece_along_an_edge_are_still_placed_once() {
        let g = grid(2, 2);
        // no length: wholly in the cell of its point
        assert_eq!(g.segment_shares((MM, MM), (MM, MM)), vec![(0, 1.0)]);
        // outside the box: wholly in the nearest cell
        assert_eq!(
            g.segment_shares((30 * MM, 30 * MM), (40 * MM, 30 * MM)),
            vec![(3, 1.0)]
        );
        // along the edge between two columns: the shares are made to add up to one
        let along = g.segment_shares((10 * MM, MM), (10 * MM, 4 * MM));
        let total: f64 = along.iter().map(|s| s.1).sum();
        assert!(close(total, 1.0), "{along:?}");
    }

    #[test]
    fn the_cells_of_a_grid_made_from_a_fixed_generator_always_add_up() {
        // a linear congruential generator with a fixed seed: the same shapes on every run
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move |n: i64| -> i64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) % n as u64) as i64
        };
        for round in 0..60 {
            let columns = 1 + (round % 5);
            let rows = 1 + (round / 5 % 4);
            let g = grid(columns, rows);
            let (x0, y0) = (next(15 * MM), next(8 * MM));
            let (w, h) = (1 + next(5 * MM), 1 + next(2 * MM));
            let rect = square(x0, y0, x0 + w, y0 + h);
            let area = w as f64 * h as f64;
            assert!(close(sum(&g.polygon_areas(&rect)), area), "round {round}");
            let a = (next(20 * MM), next(10 * MM));
            let b = (next(20 * MM), next(10 * MM));
            let shares = g.segment_shares(a, b);
            let total: f64 = shares.iter().map(|s| s.1).sum();
            assert!(close(total, 1.0), "round {round}: {shares:?}");
            assert!(shares.iter().all(|s| s.0 < g.cells() && s.1 > 0.0));
        }
    }

    #[test]
    fn the_copper_of_a_cell_is_capped_at_its_area_and_the_coverage_is_exact() {
        assert_eq!(capped_nm2(1000.4, 5000), 1000);
        assert_eq!(capped_nm2(9000.0, 5000), 5000);
        assert_eq!(capped_nm2(-5.0, 5000), 0);
        assert_eq!(capped_nm2(f64::NAN, 5000), 0);
        assert_eq!(capped_nm2(f64::INFINITY, 5000), 0);
        // a cell with no area keeps the copper: the coverage is zero all the same
        assert_eq!(capped_nm2(700.0, 0), 700);
        assert_eq!(coverage_x100(700, 0), 0);
        assert_eq!(coverage_x100(1, 4), 2500);
        assert_eq!(coverage_x100(5000, 5000), 10_000);
        assert_eq!(coverage_x100(2, 3), 6666);
    }

    #[test]
    fn the_spread_is_taken_over_the_cells_that_have_an_area() {
        assert_eq!(spread_x100(&[]), None);
        assert_eq!(spread_x100(&[(0, 9000)]), None);
        assert_eq!(spread_x100(&[(5, 1000)]), Some((1000, 1000)));
        assert_eq!(
            spread_x100(&[(5, 1000), (0, 9999), (7, 4000), (9, 250)]),
            Some((250, 4000))
        );
    }
}
