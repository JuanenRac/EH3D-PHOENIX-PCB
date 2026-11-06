// SPDX-License-Identifier: GPL-3.0-or-later
//! The description of a cross section as rectangles (dielectric blocks, electrodes, grounded
//! copper) in a box, and the mesh made from it: a line at every edge, cells that are fine at the
//! edge of a conductor and grow away from it.

use crate::codes::{FieldError, SHAPE_INVALID};
use crate::grid::{Grid, Rect, Walls};
use crate::mesh::grade_lines;
use crate::settings::Field2dSettings;

/// A rectangle of dielectric. Blocks are painted in order, a later one over an earlier one.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Where it is, in metres.
    pub rect: Rect,
    /// The relative permittivity.
    pub eps_r: f64,
    /// The loss tangent.
    pub tan_d: f64,
}

/// A rectangle of copper that carries a signal.
#[derive(Debug, Clone, PartialEq)]
pub struct Electrode {
    /// Where it is, in metres.
    pub rect: Rect,
    /// The number of the conductor, from 1.
    pub id: u32,
}

/// A cross section: a box with dielectric, signal conductors and grounded copper in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// The box.
    pub domain: Rect,
    /// What the four walls of the box are.
    pub walls: Walls,
    /// The dielectrics; what no block covers is vacuum.
    pub blocks: Vec<Block>,
    /// The conductors that carry a signal.
    pub electrodes: Vec<Electrode>,
    /// Copper at zero potential inside the box.
    pub grounds: Vec<Rect>,
    /// How many signal conductors there are; their numbers are `1` to this.
    pub conductors: usize,
}

fn invalid(what: impl Into<String>) -> FieldError {
    FieldError::new(SHAPE_INVALID, what)
}

impl Shape {
    /// Check the rectangles, their places and the numbers of the conductors.
    pub fn validate(&self) -> Result<(), FieldError> {
        let d = self.domain;
        if !d.is_valid() {
            return Err(invalid("the box is not a rectangle with sides in order"));
        }
        let slack = 1.0e-9 * (d.x1 - d.x0).max(d.y1 - d.y0);
        let inside = |r: &Rect| {
            r.is_valid()
                && r.x0 >= d.x0 - slack
                && r.x1 <= d.x1 + slack
                && r.y0 >= d.y0 - slack
                && r.y1 <= d.y1 + slack
        };
        for block in &self.blocks {
            if !inside(&block.rect) {
                return Err(invalid("a block of dielectric is outside the box"));
            }
            if !block.eps_r.is_finite() || block.eps_r < 1.0 {
                return Err(invalid("the relative permittivity must be at least 1"));
            }
            if !block.tan_d.is_finite() || block.tan_d < 0.0 {
                return Err(invalid("the loss tangent must not be negative"));
            }
        }
        for ground in &self.grounds {
            if !inside(ground) {
                return Err(invalid("a piece of grounded copper is outside the box"));
            }
        }
        if self.conductors == 0 || self.conductors > 16 {
            return Err(invalid("there must be 1 to 16 signal conductors"));
        }
        let mut seen = vec![false; self.conductors + 1];
        for electrode in &self.electrodes {
            let id = usize::try_from(electrode.id).unwrap_or(usize::MAX);
            if !inside(&electrode.rect) {
                return Err(invalid("an electrode is outside the box"));
            }
            if id == 0 || id > self.conductors {
                return Err(invalid(
                    "the number of an electrode is outside 1 to the count",
                ));
            }
            seen[id] = true;
        }
        if seen.iter().skip(1).any(|s| !*s) {
            return Err(invalid("a conductor has no electrode"));
        }
        Ok(())
    }

    /// The breaks of one direction: the sides of the box and of every rectangle, with the sides
    /// of the conductors marked as fine. Sides closer than a billionth of the box are one.
    fn breaks(&self, horizontal: bool) -> Vec<(f64, bool)> {
        let d = self.domain;
        let (lo, hi) = if horizontal {
            (d.x0, d.x1)
        } else {
            (d.y0, d.y1)
        };
        let pick = |r: &Rect| {
            if horizontal {
                (r.x0, r.x1)
            } else {
                (r.y0, r.y1)
            }
        };
        let mut all: Vec<(f64, bool)> = vec![(lo, false), (hi, false)];
        for block in &self.blocks {
            let (a, b) = pick(&block.rect);
            all.push((a, false));
            all.push((b, false));
        }
        let copper = self
            .electrodes
            .iter()
            .map(|e| &e.rect)
            .chain(self.grounds.iter());
        for rect in copper {
            let (a, b) = pick(rect);
            all.push((a, true));
            all.push((b, true));
        }
        for item in all.iter_mut() {
            item.0 = item.0.clamp(lo, hi);
            // the sides that are the walls of the box do not need a fine mesh
            if item.0 <= lo || item.0 >= hi {
                item.1 = false;
            }
        }
        all.sort_by(|a, b| a.0.total_cmp(&b.0));
        let close = 1.0e-9 * (hi - lo);
        let mut merged: Vec<(f64, bool)> = Vec::with_capacity(all.len());
        for (at, fine) in all {
            if let Some(last) = merged.last_mut() {
                if at - last.0 <= close {
                    last.1 = last.1 || fine;
                    continue;
                }
            }
            merged.push((at, fine));
        }
        merged
    }

    /// Make the mesh and paint the materials on it.
    pub fn mesh(&self, settings: &Field2dSettings) -> Result<Grid, FieldError> {
        self.validate()?;
        let bx = self.breaks(true);
        let by = self.breaks(false);
        let mut feature = f64::INFINITY;
        for list in [&bx, &by] {
            for pair in list.windows(2) {
                feature = feature.min(pair[1].0 - pair[0].0);
            }
        }
        let d = self.domain;
        let h_fine = settings.edge_step_fraction * feature / settings.refine;
        let h_max = settings.max_step_fraction * (d.x1 - d.x0).min(d.y1 - d.y0) / settings.refine;
        let coords = |list: &[(f64, bool)]| -> Result<Vec<f64>, FieldError> {
            let at: Vec<f64> = list.iter().map(|b| b.0).collect();
            let fine: Vec<bool> = list.iter().map(|b| b.1).collect();
            grade_lines(
                &at,
                &fine,
                h_fine,
                h_max,
                settings.grading_ratio,
                settings.max_cells,
            )
        };
        let xs = coords(&bx)?;
        let ys = coords(&by)?;
        let mut grid = Grid::new(xs, ys, self.walls, settings.max_cells)?;
        for block in &self.blocks {
            grid.paint_dielectric(block.rect, block.eps_r, block.tan_d);
        }
        for ground in &self.grounds {
            grid.paint_conductor(*ground, 0);
        }
        for electrode in &self.electrodes {
            grid.paint_conductor(electrode.rect, electrode.id);
        }
        Ok(grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::MESH_TOO_LARGE;

    fn strip() -> Shape {
        Shape {
            domain: Rect::new(-2.0e-3, 2.0e-3, 0.0, 2.0e-3),
            walls: Walls::GROUNDED,
            blocks: vec![Block {
                rect: Rect::new(-2.0e-3, 2.0e-3, 0.0, 0.2e-3),
                eps_r: 4.0,
                tan_d: 0.02,
            }],
            electrodes: vec![Electrode {
                rect: Rect::new(-0.15e-3, 0.15e-3, 0.2e-3, 0.235e-3),
                id: 1,
            }],
            grounds: Vec::new(),
            conductors: 1,
        }
    }

    #[test]
    fn the_mesh_has_a_line_at_every_edge_and_is_finer_at_the_electrode() {
        let shape = strip();
        let grid = shape.mesh(&Field2dSettings::default()).unwrap();
        for edge in [-0.15e-3, 0.15e-3] {
            assert!(
                grid.xs().iter().any(|x| (x - edge).abs() < 1.0e-15),
                "{edge}"
            );
        }
        for edge in [0.0, 0.2e-3, 0.235e-3, 2.0e-3] {
            assert!(
                grid.ys().iter().any(|y| (y - edge).abs() < 1.0e-15),
                "{edge}"
            );
        }
        let near = grid
            .xs()
            .windows(2)
            .map(|p| p[1] - p[0])
            .fold(f64::INFINITY, f64::min);
        let far = grid
            .xs()
            .windows(2)
            .map(|p| p[1] - p[0])
            .fold(0.0, f64::max);
        assert!(near < far / 5.0, "{near} {far}");
        // the smallest cell is the fraction of the smallest feature (35 um)
        assert!(near <= 0.25 * 35.0e-6 * 1.0001, "{near}");
    }

    #[test]
    fn refining_divides_the_cells_and_makes_more_of_them() {
        let shape = strip();
        let coarse = shape.mesh(&Field2dSettings::default()).unwrap();
        let fine = shape
            .mesh(&Field2dSettings {
                refine: 2.0,
                ..Field2dSettings::default()
            })
            .unwrap();
        assert!(
            fine.cells() * 2 > coarse.cells() * 3,
            "{} {}",
            fine.cells(),
            coarse.cells()
        );
    }

    #[test]
    fn the_same_shape_gives_the_same_mesh() {
        let shape = strip();
        let a = shape.mesh(&Field2dSettings::default()).unwrap();
        let b = shape.mesh(&Field2dSettings::default()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_limit_of_cells_that_is_too_small_refuses_the_mesh() {
        let shape = strip();
        let settings = Field2dSettings {
            max_cells: 100,
            ..Field2dSettings::default()
        };
        let error = shape.mesh(&settings).unwrap_err();
        assert_eq!(error.code, MESH_TOO_LARGE);
    }

    #[test]
    fn shapes_that_are_not_valid_are_refused() {
        let mut outside = strip();
        outside.electrodes[0].rect = Rect::new(-0.15e-3, 3.0e-3, 0.2e-3, 0.235e-3);
        let mut no_conductor = strip();
        no_conductor.electrodes.clear();
        let mut wrong_id = strip();
        wrong_id.electrodes[0].id = 2;
        let mut bad_eps = strip();
        bad_eps.blocks[0].eps_r = 0.5;
        let mut bad_tan = strip();
        bad_tan.blocks[0].tan_d = -0.1;
        let mut flat = strip();
        flat.domain = Rect::new(1.0, 1.0, 0.0, 1.0);
        let mut ground_outside = strip();
        ground_outside.grounds.push(Rect::new(5.0, 6.0, 0.0, 1.0));
        for shape in [
            outside,
            no_conductor,
            wrong_id,
            bad_eps,
            bad_tan,
            flat,
            ground_outside,
        ] {
            let error = shape.validate().unwrap_err();
            assert_eq!(error.code, SHAPE_INVALID);
        }
    }
}
