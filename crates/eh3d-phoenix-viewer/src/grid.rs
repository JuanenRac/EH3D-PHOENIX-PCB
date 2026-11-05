// SPDX-License-Identifier: GPL-3.0-or-later
//! A uniform grid over the primitives, to find those near a point or inside a window without
//! looking at all of them. It is built once for a scene; its size comes from the settings.

use crate::prim::{Bounds, Prim, P};
use crate::settings::ViewerSettings;

/// The index: for each cell, the primitives whose box touches it.
#[derive(Debug, Clone)]
pub struct Grid {
    origin: P,
    cell: f64,
    nx: usize,
    ny: usize,
    /// `starts[c]..starts[c + 1]` is the range of `items` that belongs to cell `c`.
    starts: Vec<u32>,
    items: Vec<u32>,
    /// The primitives that cover too many cells to be listed in each.
    large: Vec<u32>,
    boxes: Vec<Bounds>,
}

impl Grid {
    /// Builds the grid of `prims`, which fill `bounds`.
    pub fn build(prims: &[Prim], bounds: Bounds, settings: &ViewerSettings) -> Grid {
        let boxes: Vec<Bounds> = prims.iter().map(Prim::bounds).collect();
        let area = if bounds.is_empty() {
            Bounds {
                min: P::new(0.0, 0.0),
                max: P::new(1.0, 1.0),
            }
        } else {
            bounds
        };
        let (w, h) = (area.width().max(1e-6), area.height().max(1e-6));
        let wanted = (prims.len() as f64 / settings.grid_target_per_cell.max(0.1)).max(1.0);
        let mut cell = (w * h / wanted).sqrt().max(w.max(h) / 4096.0);
        let max_cells = settings.grid_max_cells.max(1);
        let (mut nx, mut ny) = (cells_along(w, cell), cells_along(h, cell));
        while nx.saturating_mul(ny) > max_cells {
            cell *= 1.5;
            nx = cells_along(w, cell);
            ny = cells_along(h, cell);
        }
        let mut grid = Grid {
            origin: area.min,
            cell,
            nx,
            ny,
            starts: Vec::new(),
            items: Vec::new(),
            large: Vec::new(),
            boxes,
        };
        let limit = settings.grid_max_cells_per_prim.max(1);
        let total = nx * ny;
        // first pass: how many items each cell gets; second pass: where they go
        let mut counts = vec![0u32; total];
        let mut large = Vec::new();
        for (i, b) in grid.boxes.iter().enumerate() {
            if b.is_empty() {
                continue;
            }
            let (x0, x1) = (grid.col(b.min.x), grid.col(b.max.x));
            let (y0, y1) = (grid.row(b.min.y), grid.row(b.max.y));
            if (x1 - x0 + 1) * (y1 - y0 + 1) > limit {
                large.push(i as u32);
                continue;
            }
            for y in y0..=y1 {
                for x in x0..=x1 {
                    counts[y * nx + x] += 1;
                }
            }
        }
        let mut starts: Vec<u32> = Vec::with_capacity(total + 1);
        let mut running = 0u32;
        starts.push(0);
        for count in &counts {
            running += count;
            starts.push(running);
        }
        let mut fill = starts.clone();
        let mut items = vec![0u32; starts[total] as usize];
        for (i, b) in grid.boxes.iter().enumerate() {
            if b.is_empty() {
                continue;
            }
            let (x0, x1) = (grid.col(b.min.x), grid.col(b.max.x));
            let (y0, y1) = (grid.row(b.min.y), grid.row(b.max.y));
            if (x1 - x0 + 1) * (y1 - y0 + 1) > limit {
                continue;
            }
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let c = y * nx + x;
                    items[fill[c] as usize] = i as u32;
                    fill[c] += 1;
                }
            }
        }
        grid.starts = starts;
        grid.items = items;
        grid.large = large;
        grid
    }

    fn col(&self, x: f64) -> usize {
        let c = ((x - self.origin.x) / self.cell).floor();
        if c.is_nan() || c < 0.0 {
            0
        } else {
            (c as usize).min(self.nx - 1)
        }
    }

    fn row(&self, y: f64) -> usize {
        let r = ((y - self.origin.y) / self.cell).floor();
        if r.is_nan() || r < 0.0 {
            0
        } else {
            (r as usize).min(self.ny - 1)
        }
    }

    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.nx * self.ny
    }

    /// The side of a cell in millimetres.
    pub fn cell_size(&self) -> f64 {
        self.cell
    }

    /// How many primitives are kept in the list of large ones.
    pub fn large_count(&self) -> usize {
        self.large.len()
    }

    /// The box of a primitive, including half the width of its stroke.
    pub fn bounds_of(&self, prim: u32) -> Bounds {
        self.boxes[prim as usize]
    }

    /// Adds to `out` every primitive whose box touches `area`, each once. The primitive is
    /// reported only by the cell that holds the lower left corner of the part of its box inside
    /// `area`, so no list has to be sorted or cleaned afterwards.
    pub fn query(&self, area: Bounds, out: &mut Vec<u32>) {
        if area.is_empty() {
            return;
        }
        for &i in &self.large {
            if self.boxes[i as usize].intersects(&area) {
                out.push(i);
            }
        }
        let (x0, x1) = (self.col(area.min.x), self.col(area.max.x));
        let (y0, y1) = (self.row(area.min.y), self.row(area.max.y));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = y * self.nx + x;
                let (from, to) = (self.starts[c] as usize, self.starts[c + 1] as usize);
                for &i in &self.items[from..to] {
                    let b = &self.boxes[i as usize];
                    if !b.intersects(&area) {
                        continue;
                    }
                    let corner_x = b.min.x.max(area.min.x);
                    let corner_y = b.min.y.max(area.min.y);
                    if self.col(corner_x) == x && self.row(corner_y) == y {
                        out.push(i);
                    }
                }
            }
        }
    }
}

fn cells_along(length: f64, cell: f64) -> usize {
    ((length / cell).ceil() as usize).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim::Shape;

    fn dot(x: f64, y: f64, r: f64) -> Prim {
        Prim {
            shape: Shape::Circle {
                centre: P::new(x, y),
                radius: r,
            },
            width: 0.0,
            filled: true,
            layer: 0,
            net: None,
            item: 0,
        }
    }

    fn scattered(n: usize) -> Vec<Prim> {
        let mut state = 12345u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|_| dot(next() * 100.0, next() * 60.0, 0.1 + next() * 2.0))
            .collect()
    }

    fn bounds_of_all(prims: &[Prim]) -> Bounds {
        prims
            .iter()
            .fold(Bounds::EMPTY, |acc, p| acc.union(p.bounds()))
    }

    fn brute(prims: &[Prim], area: Bounds) -> Vec<u32> {
        (0..prims.len() as u32)
            .filter(|&i| prims[i as usize].bounds().intersects(&area))
            .collect()
    }

    #[test]
    fn a_query_finds_exactly_what_a_scan_finds_each_once() {
        let prims = scattered(3000);
        let grid = Grid::build(&prims, bounds_of_all(&prims), &ViewerSettings::default());
        for area in [
            Bounds {
                min: P::new(10.0, 10.0),
                max: P::new(12.0, 11.0),
            },
            Bounds {
                min: P::new(-50.0, -50.0),
                max: P::new(500.0, 500.0),
            },
            Bounds {
                min: P::new(40.0, 0.0),
                max: P::new(70.0, 30.0),
            },
            Bounds {
                min: P::new(99.9, 59.9),
                max: P::new(99.9, 59.9),
            },
        ] {
            let mut found = Vec::new();
            grid.query(area, &mut found);
            found.sort_unstable();
            assert_eq!(found, brute(&prims, area), "{area:?}");
        }
    }

    #[test]
    fn a_huge_primitive_is_kept_apart_and_still_found() {
        let mut prims = scattered(500);
        prims.push(dot(50.0, 30.0, 200.0));
        let settings = ViewerSettings::default();
        let grid = Grid::build(&prims, bounds_of_all(&prims), &settings);
        assert_eq!(grid.large_count(), 1);
        let area = Bounds {
            min: P::new(1.0, 1.0),
            max: P::new(1.5, 1.5),
        };
        let mut found = Vec::new();
        grid.query(area, &mut found);
        assert!(found.contains(&500));
        found.sort_unstable();
        assert_eq!(found, brute(&prims, area));
    }

    #[test]
    fn the_number_of_cells_follows_the_settings() {
        let prims = scattered(4000);
        let bounds = bounds_of_all(&prims);
        let coarse = ViewerSettings::parse("grid_target_per_cell = 40").unwrap();
        let fine = ViewerSettings::parse("grid_target_per_cell = 2").unwrap();
        let capped = ViewerSettings::parse("grid_max_cells = 50").unwrap();
        let a = Grid::build(&prims, bounds, &coarse);
        let b = Grid::build(&prims, bounds, &fine);
        let c = Grid::build(&prims, bounds, &capped);
        assert!(a.cells() < b.cells());
        assert!(c.cells() <= 50);
        assert!(c.cell_size() > b.cell_size());
    }

    #[test]
    fn no_primitives_and_a_point_scene_do_not_break_the_grid() {
        let empty = Grid::build(&[], Bounds::EMPTY, &ViewerSettings::default());
        let mut out = Vec::new();
        empty.query(
            Bounds {
                min: P::new(0.0, 0.0),
                max: P::new(1.0, 1.0),
            },
            &mut out,
        );
        assert!(out.is_empty());
        let prims = vec![dot(5.0, 5.0, 0.0)];
        let one = Grid::build(&prims, bounds_of_all(&prims), &ViewerSettings::default());
        one.query(
            Bounds {
                min: P::new(4.0, 4.0),
                max: P::new(6.0, 6.0),
            },
            &mut out,
        );
        assert_eq!(out, vec![0]);
        // an empty query area finds nothing
        out.clear();
        one.query(Bounds::EMPTY, &mut out);
        assert!(out.is_empty());
    }
}
