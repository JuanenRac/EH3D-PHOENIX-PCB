// SPDX-License-Identifier: GPL-3.0-or-later
//! The rectangular grid of a cross section and the solution of the Laplace equation on it.
//!
//! The grid has lines at positions given in metres, not equally spaced. Each cell holds a relative
//! permittivity, a loss tangent and, when it lies inside a conductor, the number of that conductor
//! (0 is the ground). The potential lives on the nodes, the crossings of the lines. The equation
//! `div(eps grad phi) = 0` is integrated over the control volume of each node (finite volumes,
//! which for a regular grid is the five-point difference): the coefficient of the edge between two
//! nodes is the sum of the contributions of the two cells next to it, each `eps * size / (2 * length)`,
//! so a boundary between two dielectrics that lies on a line of the grid is handled exactly. A node
//! that touches a conductor cell has the potential of that conductor; a node on a wall called
//! `Ground` is at zero, and a wall called `Symmetry` has no flux through it.
//!
//! The charge on a conductor is the sum, over the edges that join one of its nodes to a free
//! node, of the coefficient times the difference of potential; the capacitance matrix has one
//! column for each conductor set to one volt with the others and the ground at zero. Every loop runs in a fixed
//! order and every sum is made term by term, so the result is the same on every machine.

// the loops of a numerical kernel walk several arrays of the same shape with one index
#![allow(clippy::needless_range_loop)]

use crate::codes::{FieldError, CONDUCTORS_TOUCH, MESH_TOO_LARGE, NOT_CONVERGED, SHAPE_INVALID};
use crate::settings::{Field2dSettings, Method};
use crate::EPS0;

/// A rectangle in metres: `x0 < x1`, `y0 < y1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// The left side.
    pub x0: f64,
    /// The right side.
    pub x1: f64,
    /// The bottom side.
    pub y0: f64,
    /// The top side.
    pub y1: f64,
}

impl Rect {
    /// A rectangle from its sides.
    pub fn new(x0: f64, x1: f64, y0: f64, y1: f64) -> Rect {
        Rect { x0, x1, y0, y1 }
    }

    /// Whether the point lies strictly inside the rectangle.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x > self.x0 && x < self.x1 && y > self.y0 && y < self.y1
    }

    /// Whether the sides are finite and in order.
    pub fn is_valid(&self) -> bool {
        self.x0.is_finite()
            && self.x1.is_finite()
            && self.y0.is_finite()
            && self.y1.is_finite()
            && self.x0 < self.x1
            && self.y0 < self.y1
    }
}

/// What a wall of the box is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wall {
    /// A conductor at zero potential.
    Ground,
    /// A plane of symmetry: no flux crosses it.
    Symmetry,
}

/// The four walls of the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walls {
    /// The wall at the smallest `x`.
    pub left: Wall,
    /// The wall at the largest `x`.
    pub right: Wall,
    /// The wall at the smallest `y`.
    pub bottom: Wall,
    /// The wall at the largest `y`.
    pub top: Wall,
}

impl Walls {
    /// All four walls at zero potential.
    pub const GROUNDED: Walls = Walls {
        left: Wall::Ground,
        right: Wall::Ground,
        bottom: Wall::Ground,
        top: Wall::Ground,
    };
}

/// The grid with its materials.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    nx: usize,
    ny: usize,
    eps: Vec<f64>,
    tan_d: Vec<f64>,
    cond: Vec<i32>,
    walls: Walls,
}

fn lines_valid(lines: &[f64]) -> bool {
    lines.len() >= 2
        && lines.iter().all(|v| v.is_finite())
        && lines.windows(2).all(|pair| pair[1] > pair[0])
}

impl Grid {
    /// A grid from the coordinates of its lines. The cells start as vacuum. Fails when the lines
    /// are not increasing or the grid would have more than `max_cells` cells.
    pub fn new(
        xs: Vec<f64>,
        ys: Vec<f64>,
        walls: Walls,
        max_cells: usize,
    ) -> Result<Grid, FieldError> {
        if !lines_valid(&xs) || !lines_valid(&ys) {
            return Err(FieldError::new(
                SHAPE_INVALID,
                "the lines of the grid must be finite and increasing, at least two in each direction",
            ));
        }
        let (nx, ny) = (xs.len() - 1, ys.len() - 1);
        let cells = nx.saturating_mul(ny);
        if cells > max_cells {
            return Err(FieldError::new(
                MESH_TOO_LARGE,
                format!("{nx} by {ny} cells is more than the limit of {max_cells}"),
            ));
        }
        Ok(Grid {
            xs,
            ys,
            nx,
            ny,
            eps: vec![1.0; cells],
            tan_d: vec![0.0; cells],
            cond: vec![-1; cells],
            walls,
        })
    }

    /// The number of cells in the `x` direction.
    pub fn nx(&self) -> usize {
        self.nx
    }

    /// The number of cells in the `y` direction.
    pub fn ny(&self) -> usize {
        self.ny
    }

    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.nx * self.ny
    }

    /// The lines in the `x` direction.
    pub fn xs(&self) -> &[f64] {
        &self.xs
    }

    /// The lines in the `y` direction.
    pub fn ys(&self) -> &[f64] {
        &self.ys
    }

    fn centre(&self, i: usize, j: usize) -> (f64, f64) {
        (
            (self.xs[i] + self.xs[i + 1]) / 2.0,
            (self.ys[j] + self.ys[j + 1]) / 2.0,
        )
    }

    /// Give the cells whose centre is inside the rectangle a relative permittivity and a loss
    /// tangent. Conductor cells are left alone.
    pub fn paint_dielectric(&mut self, rect: Rect, eps_r: f64, tan_d: f64) {
        for j in 0..self.ny {
            for i in 0..self.nx {
                let (cx, cy) = self.centre(i, j);
                let at = j * self.nx + i;
                if self.cond[at] < 0 && rect.contains(cx, cy) {
                    self.eps[at] = eps_r;
                    self.tan_d[at] = tan_d;
                }
            }
        }
    }

    /// Make the cells whose centre is inside the rectangle part of the conductor `id` (0 is the
    /// ground).
    pub fn paint_conductor(&mut self, rect: Rect, id: u32) {
        self.paint_conductor_where(id, |x, y| rect.contains(x, y));
    }

    /// Make the cells whose centre satisfies a test part of the conductor `id`.
    pub fn paint_conductor_where<F: Fn(f64, f64) -> bool>(&mut self, id: u32, inside: F) {
        let value = i32::try_from(id).unwrap_or(i32::MAX);
        for j in 0..self.ny {
            for i in 0..self.nx {
                let (cx, cy) = self.centre(i, j);
                if inside(cx, cy) {
                    let at = j * self.nx + i;
                    self.cond[at] = value;
                    self.eps[at] = 0.0;
                    self.tan_d[at] = 0.0;
                }
            }
        }
    }

    /// The same grid with vacuum in place of every dielectric.
    pub fn without_dielectric(&self) -> Grid {
        let mut copy = self.clone();
        for (at, c) in copy.cond.iter().enumerate() {
            if *c < 0 {
                copy.eps[at] = 1.0;
                copy.tan_d[at] = 0.0;
            }
        }
        copy
    }
}

/// The solution for every conductor set to one volt in turn.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    /// The number of conductors, not counting the ground.
    pub conductors: usize,
    /// The capacitance matrix in farad per metre, `c[m][k]` the charge on conductor `m + 1`
    /// when conductor `k + 1` is at one volt and the rest at zero.
    pub c: Vec<Vec<f64>>,
    /// The potential at every node for each conductor at one volt.
    pub potentials: Vec<Vec<f64>>,
    /// The most iterations any of the solutions took.
    pub iterations: u32,
    /// The largest final residual, as a fraction of the first.
    pub residual: f64,
    fixed: Vec<i32>,
}

struct Discrete {
    fixed: Vec<i32>,
    ex: Vec<f64>,
    ey: Vec<f64>,
    diag: Vec<f64>,
}

fn claim(fixed: &mut [i32], node: usize, id: i32) -> bool {
    if fixed[node] < 0 || fixed[node] == id {
        fixed[node] = id;
        true
    } else {
        false
    }
}

impl Grid {
    fn node(&self, i: usize, j: usize) -> usize {
        j * (self.nx + 1) + i
    }

    fn discretize(&self, conductors: usize) -> Result<Discrete, FieldError> {
        let (nx, ny) = (self.nx, self.ny);
        let touch = || {
            FieldError::new(
                CONDUCTORS_TOUCH,
                "two conductors, or a conductor and a grounded wall, share a node of the mesh",
            )
        };
        let mut fixed = vec![-1_i32; (nx + 1) * (ny + 1)];
        for j in 0..ny {
            for i in 0..nx {
                let id = self.cond[j * nx + i];
                if id < 0 {
                    continue;
                }
                if usize::try_from(id).unwrap_or(usize::MAX) > conductors {
                    return Err(FieldError::new(
                        SHAPE_INVALID,
                        format!("conductor {id} is above the {conductors} that were declared"),
                    ));
                }
                for (dj, di) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    if !claim(&mut fixed, self.node(i + di, j + dj), id) {
                        return Err(touch());
                    }
                }
            }
        }
        let w = self.walls;
        if w.left == Wall::Ground {
            for j in 0..=ny {
                if !claim(&mut fixed, self.node(0, j), 0) {
                    return Err(touch());
                }
            }
        }
        if w.right == Wall::Ground {
            for j in 0..=ny {
                if !claim(&mut fixed, self.node(nx, j), 0) {
                    return Err(touch());
                }
            }
        }
        if w.bottom == Wall::Ground {
            for i in 0..=nx {
                if !claim(&mut fixed, self.node(i, 0), 0) {
                    return Err(touch());
                }
            }
        }
        if w.top == Wall::Ground {
            for i in 0..=nx {
                if !claim(&mut fixed, self.node(i, ny), 0) {
                    return Err(touch());
                }
            }
        }
        let hx: Vec<f64> = self.xs.windows(2).map(|p| p[1] - p[0]).collect();
        let hy: Vec<f64> = self.ys.windows(2).map(|p| p[1] - p[0]).collect();
        let mut ex = vec![0.0; nx * (ny + 1)];
        for j in 0..=ny {
            for i in 0..nx {
                let mut s = 0.0;
                if j < ny {
                    s += self.eps[j * nx + i] * hy[j];
                }
                if j > 0 {
                    s += self.eps[(j - 1) * nx + i] * hy[j - 1];
                }
                ex[j * nx + i] = s / (2.0 * hx[i]);
            }
        }
        let mut ey = vec![0.0; (nx + 1) * ny];
        for j in 0..ny {
            for i in 0..=nx {
                let mut s = 0.0;
                if i < nx {
                    s += self.eps[j * nx + i] * hx[i];
                }
                if i > 0 {
                    s += self.eps[j * nx + i - 1] * hx[i - 1];
                }
                ey[j * (nx + 1) + i] = s / (2.0 * hy[j]);
            }
        }
        let mut diag = vec![0.0; (nx + 1) * (ny + 1)];
        for j in 0..=ny {
            for i in 0..=nx {
                let mut d = 0.0;
                if i > 0 {
                    d += ex[j * nx + i - 1];
                }
                if i < nx {
                    d += ex[j * nx + i];
                }
                if j > 0 {
                    d += ey[(j - 1) * (nx + 1) + i];
                }
                if j < ny {
                    d += ey[j * (nx + 1) + i];
                }
                diag[j * (nx + 1) + i] = d;
            }
        }
        Ok(Discrete {
            fixed,
            ex,
            ey,
            diag,
        })
    }
}

impl Discrete {
    /// `out = L x` on the free nodes, zero on the fixed ones.
    fn apply(&self, nx: usize, ny: usize, x: &[f64], out: &mut [f64]) {
        for j in 0..=ny {
            for i in 0..=nx {
                let n = j * (nx + 1) + i;
                if self.fixed[n] >= 0 {
                    out[n] = 0.0;
                    continue;
                }
                let mut s = self.diag[n] * x[n];
                if i > 0 {
                    s -= self.ex[j * nx + i - 1] * x[n - 1];
                }
                if i < nx {
                    s -= self.ex[j * nx + i] * x[n + 1];
                }
                if j > 0 {
                    s -= self.ey[(j - 1) * (nx + 1) + i] * x[n - (nx + 1)];
                }
                if j < ny {
                    s -= self.ey[j * (nx + 1) + i] * x[n + (nx + 1)];
                }
                out[n] = s;
            }
        }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut s = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        s += x * y;
    }
    s
}

fn not_converged(iterations: u32, residual: f64) -> FieldError {
    FieldError::new(
        NOT_CONVERGED,
        format!("after {iterations} iterations the residual is {residual:e} of the first"),
    )
}

impl Grid {
    /// Solve for one set of potentials of the conductors (`volts[k]` for conductor `k`, 0 for
    /// the ground). Returns the potential of every node, the iterations and the final residual.
    fn solve_one(
        &self,
        disc: &Discrete,
        volts: &[f64],
        settings: &Field2dSettings,
    ) -> Result<(Vec<f64>, u32, f64), FieldError> {
        let (nx, ny) = (self.nx, self.ny);
        let nodes = (nx + 1) * (ny + 1);
        let mut x = vec![0.0; nodes];
        for (n, id) in disc.fixed.iter().enumerate() {
            if *id >= 0 {
                x[n] = volts
                    .get(usize::try_from(*id).unwrap_or(usize::MAX))
                    .copied()
                    .unwrap_or(0.0);
            }
        }
        let mut r = vec![0.0; nodes];
        disc.apply(nx, ny, &x, &mut r);
        for v in r.iter_mut() {
            *v = -*v;
        }
        let first = dot(&r, &r).sqrt();
        if first == 0.0 {
            return Ok((x, 0, 0.0));
        }
        match settings.method {
            Method::ConjugateGradient => self.cg(disc, x, r, first, settings),
            Method::Sor => self.sor(disc, x, first, settings),
        }
    }

    fn cg(
        &self,
        disc: &Discrete,
        mut x: Vec<f64>,
        mut r: Vec<f64>,
        first: f64,
        settings: &Field2dSettings,
    ) -> Result<(Vec<f64>, u32, f64), FieldError> {
        let (nx, ny) = (self.nx, self.ny);
        let nodes = x.len();
        let mut z: Vec<f64> = vec![0.0; nodes];
        for n in 0..nodes {
            if disc.fixed[n] < 0 {
                z[n] = r[n] / disc.diag[n];
            }
        }
        let mut p = z.clone();
        let mut q = vec![0.0; nodes];
        let mut rz = dot(&r, &z);
        let mut iterations = 0_u32;
        loop {
            let residual = dot(&r, &r).sqrt() / first;
            if residual <= settings.tolerance {
                return Ok((x, iterations, residual));
            }
            if iterations >= settings.max_iterations {
                return Err(not_converged(iterations, residual));
            }
            disc.apply(nx, ny, &p, &mut q);
            let pq = dot(&p, &q);
            if pq <= 0.0 || !pq.is_finite() {
                return Err(not_converged(iterations, residual));
            }
            let alpha = rz / pq;
            for n in 0..nodes {
                x[n] += alpha * p[n];
                r[n] -= alpha * q[n];
            }
            for n in 0..nodes {
                if disc.fixed[n] < 0 {
                    z[n] = r[n] / disc.diag[n];
                }
            }
            let rz_next = dot(&r, &z);
            let beta = rz_next / rz;
            for n in 0..nodes {
                p[n] = z[n] + beta * p[n];
            }
            rz = rz_next;
            iterations += 1;
        }
    }

    fn sor(
        &self,
        disc: &Discrete,
        mut x: Vec<f64>,
        first: f64,
        settings: &Field2dSettings,
    ) -> Result<(Vec<f64>, u32, f64), FieldError> {
        let (nx, ny) = (self.nx, self.ny);
        let nodes = x.len();
        let mut r = vec![0.0; nodes];
        let mut iterations = 0_u32;
        loop {
            disc.apply(nx, ny, &x, &mut r);
            let residual = dot(&r, &r).sqrt() / first;
            if residual <= settings.tolerance {
                return Ok((x, iterations, residual));
            }
            if iterations >= settings.max_iterations || !residual.is_finite() {
                return Err(not_converged(iterations, residual));
            }
            for j in 0..=ny {
                for i in 0..=nx {
                    let n = j * (nx + 1) + i;
                    if disc.fixed[n] >= 0 {
                        continue;
                    }
                    let mut s = 0.0;
                    if i > 0 {
                        s += disc.ex[j * nx + i - 1] * x[n - 1];
                    }
                    if i < nx {
                        s += disc.ex[j * nx + i] * x[n + 1];
                    }
                    if j > 0 {
                        s += disc.ey[(j - 1) * (nx + 1) + i] * x[n - (nx + 1)];
                    }
                    if j < ny {
                        s += disc.ey[j * (nx + 1) + i] * x[n + (nx + 1)];
                    }
                    let target = s / disc.diag[n];
                    x[n] += settings.sor_omega * (target - x[n]);
                }
            }
            iterations += 1;
        }
    }

    /// The charge on every conductor (the ground is index 0) for a potential on the nodes.
    fn charges(&self, disc: &Discrete, conductors: usize, x: &[f64]) -> Vec<f64> {
        let (nx, ny) = (self.nx, self.ny);
        let mut q = vec![0.0; conductors + 1];
        let mut add = |a: usize, b: usize, coef: f64| {
            let (fa, fb) = (disc.fixed[a], disc.fixed[b]);
            if fa >= 0 && fb < 0 {
                if let Some(slot) = q.get_mut(usize::try_from(fa).unwrap_or(usize::MAX)) {
                    *slot += coef * (x[a] - x[b]);
                }
            } else if fb >= 0 && fa < 0 {
                if let Some(slot) = q.get_mut(usize::try_from(fb).unwrap_or(usize::MAX)) {
                    *slot += coef * (x[b] - x[a]);
                }
            }
        };
        for j in 0..=ny {
            for i in 0..nx {
                let a = j * (nx + 1) + i;
                add(a, a + 1, disc.ex[j * nx + i]);
            }
        }
        for j in 0..ny {
            for i in 0..=nx {
                let a = j * (nx + 1) + i;
                add(a, a + nx + 1, disc.ey[j * (nx + 1) + i]);
            }
        }
        q
    }

    /// Solve for each of `conductors` conductors at one volt in turn and return the capacitance
    /// matrix with the potentials.
    pub fn solve(
        &self,
        conductors: usize,
        settings: &Field2dSettings,
    ) -> Result<Solution, FieldError> {
        let disc = self.discretize(conductors)?;
        let mut c = vec![vec![0.0; conductors]; conductors];
        let mut potentials: Vec<Vec<f64>> = Vec::with_capacity(conductors);
        let mut iterations = 0_u32;
        let mut residual = 0.0_f64;
        for k in 0..conductors {
            let mut volts = vec![0.0; conductors + 1];
            volts[k + 1] = 1.0;
            let (x, its, res) = self.solve_one(&disc, &volts, settings)?;
            let q = self.charges(&disc, conductors, &x);
            for (m, row) in c.iter_mut().enumerate() {
                row[k] = EPS0 * q[m + 1];
            }
            iterations = iterations.max(its);
            residual = residual.max(res);
            potentials.push(x);
        }
        Ok(Solution {
            conductors,
            c,
            potentials,
            iterations,
            residual,
            fixed: disc.fixed,
        })
    }

    /// The loss tangent of the field of the potential `sum(weights[k] * phi_k)`: the energy of
    /// each cell weighted by its loss tangent, divided by the energy of all the cells. The
    /// weights of the conductors are those of the potentials of the solution.
    pub fn loss_tangent(&self, solution: &Solution, weights: &[f64]) -> f64 {
        let (nx, ny) = (self.nx, self.ny);
        let nodes = (nx + 1) * (ny + 1);
        let mut x = vec![0.0; nodes];
        for (k, w) in weights.iter().enumerate() {
            if let Some(p) = solution.potentials.get(k) {
                for n in 0..nodes {
                    x[n] += w * p[n];
                }
            }
        }
        let hx: Vec<f64> = self.xs.windows(2).map(|p| p[1] - p[0]).collect();
        let hy: Vec<f64> = self.ys.windows(2).map(|p| p[1] - p[0]).collect();
        let mut total = 0.0;
        let mut lossy = 0.0;
        for j in 0..=ny {
            for i in 0..nx {
                let a = j * (nx + 1) + i;
                if solution.fixed[a] >= 0 && solution.fixed[a + 1] >= 0 {
                    continue;
                }
                let d = x[a] - x[a + 1];
                let d2 = d * d;
                if j < ny {
                    let part = self.eps[j * nx + i] * hy[j] / (2.0 * hx[i]) * d2;
                    total += part;
                    lossy += part * self.tan_d[j * nx + i];
                }
                if j > 0 {
                    let part = self.eps[(j - 1) * nx + i] * hy[j - 1] / (2.0 * hx[i]) * d2;
                    total += part;
                    lossy += part * self.tan_d[(j - 1) * nx + i];
                }
            }
        }
        for j in 0..ny {
            for i in 0..=nx {
                let a = j * (nx + 1) + i;
                if solution.fixed[a] >= 0 && solution.fixed[a + nx + 1] >= 0 {
                    continue;
                }
                let d = x[a] - x[a + nx + 1];
                let d2 = d * d;
                if i < nx {
                    let part = self.eps[j * nx + i] * hx[i] / (2.0 * hy[j]) * d2;
                    total += part;
                    lossy += part * self.tan_d[j * nx + i];
                }
                if i > 0 {
                    let part = self.eps[j * nx + i - 1] * hx[i - 1] / (2.0 * hy[j]) * d2;
                    total += part;
                    lossy += part * self.tan_d[j * nx + i - 1];
                }
            }
        }
        if total > 0.0 {
            lossy / total
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::uniform_lines;

    fn plates(er: f64, width: f64, gap: f64, nx: usize, ny: usize) -> Grid {
        // ground at the bottom, a plate of one cell at the top, no flux through the sides
        let walls = Walls {
            left: Wall::Symmetry,
            right: Wall::Symmetry,
            bottom: Wall::Ground,
            top: Wall::Symmetry,
        };
        let mut ys = uniform_lines(0.0, gap, ny);
        let top = gap * 1.1;
        ys.push(top);
        let mut grid = Grid::new(uniform_lines(0.0, width, nx), ys, walls, 100_000).unwrap();
        grid.paint_dielectric(Rect::new(0.0, width, 0.0, gap), er, 0.0);
        grid.paint_conductor(Rect::new(0.0, width, gap, top), 1);
        grid
    }

    #[test]
    fn parallel_plates_give_eps_a_over_d_whatever_the_mesh() {
        // C = eps0 eps_r W / d per metre of depth; the discrete equations are exact for a
        // uniform field, so the error is the tolerance of the solver
        let settings = Field2dSettings {
            tolerance: 1.0e-13,
            ..Field2dSettings::default()
        };
        for (nx, ny) in [(3, 4), (8, 8), (5, 20)] {
            let grid = plates(2.5, 0.01, 0.002, nx, ny);
            let solution = grid.solve(1, &settings).unwrap();
            let exact = EPS0 * 2.5 * 0.01 / 0.002;
            let c = solution.c[0][0];
            assert!(
                ((c - exact) / exact).abs() < 1.0e-9,
                "{nx}x{ny}: {c} {exact}"
            );
        }
    }

    #[test]
    fn two_layers_of_dielectric_are_in_series() {
        let walls = Walls {
            left: Wall::Symmetry,
            right: Wall::Symmetry,
            bottom: Wall::Ground,
            top: Wall::Symmetry,
        };
        let ys = vec![0.0, 0.001, 0.002, 0.003, 0.004, 0.0044];
        let mut grid = Grid::new(uniform_lines(0.0, 0.01, 4), ys, walls, 100_000).unwrap();
        grid.paint_dielectric(Rect::new(0.0, 0.01, 0.0, 0.002), 2.0, 0.0);
        grid.paint_dielectric(Rect::new(0.0, 0.01, 0.002, 0.004), 6.0, 0.0);
        grid.paint_conductor(Rect::new(0.0, 0.01, 0.004, 0.0044), 1);
        let settings = Field2dSettings {
            tolerance: 1.0e-13,
            ..Field2dSettings::default()
        };
        let c = grid.solve(1, &settings).unwrap().c[0][0];
        // two capacitors in series: 1/C = d1/(eps1 W) + d2/(eps2 W)
        let exact = EPS0 * 0.01 / (0.002 / 2.0 + 0.002 / 6.0);
        assert!(((c - exact) / exact).abs() < 1.0e-9, "{c} {exact}");
    }

    #[test]
    fn sor_and_conjugate_gradients_agree() {
        let grid = plates(3.0, 0.01, 0.002, 6, 6);
        let cg = Field2dSettings {
            tolerance: 1.0e-12,
            ..Field2dSettings::default()
        };
        let sor = Field2dSettings {
            method: Method::Sor,
            tolerance: 1.0e-12,
            ..Field2dSettings::default()
        };
        let a = grid.solve(1, &cg).unwrap().c[0][0];
        let b = grid.solve(1, &sor).unwrap().c[0][0];
        assert!(((a - b) / a).abs() < 1.0e-8, "{a} {b}");
    }

    #[test]
    fn a_limit_of_iterations_that_is_too_small_is_an_error_and_not_a_number() {
        let grid = plates(3.0, 0.01, 0.002, 12, 12);
        let settings = Field2dSettings {
            max_iterations: 1,
            tolerance: 1.0e-13,
            ..Field2dSettings::default()
        };
        let error = grid.solve(1, &settings).unwrap_err();
        assert_eq!(error.code, NOT_CONVERGED);
    }

    #[test]
    fn a_grid_with_too_many_cells_is_refused() {
        let error = Grid::new(
            uniform_lines(0.0, 1.0, 100),
            uniform_lines(0.0, 1.0, 100),
            Walls::GROUNDED,
            9_999,
        )
        .unwrap_err();
        assert_eq!(error.code, MESH_TOO_LARGE);
    }

    #[test]
    fn lines_that_do_not_increase_are_refused() {
        let error =
            Grid::new(vec![0.0, 1.0, 1.0], vec![0.0, 1.0], Walls::GROUNDED, 1000).unwrap_err();
        assert_eq!(error.code, SHAPE_INVALID);
        let error = Grid::new(vec![0.0], vec![0.0, 1.0], Walls::GROUNDED, 1000).unwrap_err();
        assert_eq!(error.code, SHAPE_INVALID);
    }

    #[test]
    fn a_conductor_on_a_grounded_wall_is_refused() {
        let mut grid = Grid::new(
            uniform_lines(0.0, 1.0, 4),
            uniform_lines(0.0, 1.0, 4),
            Walls::GROUNDED,
            1000,
        )
        .unwrap();
        grid.paint_conductor(Rect::new(0.0, 0.6, 0.3, 0.6), 1);
        let error = grid.solve(1, &Field2dSettings::default()).unwrap_err();
        assert_eq!(error.code, CONDUCTORS_TOUCH);
    }

    #[test]
    fn a_conductor_above_the_declared_number_is_refused() {
        let mut grid = Grid::new(
            uniform_lines(0.0, 1.0, 6),
            uniform_lines(0.0, 1.0, 6),
            Walls::GROUNDED,
            1000,
        )
        .unwrap();
        grid.paint_conductor(Rect::new(0.3, 0.7, 0.3, 0.7), 2);
        let error = grid.solve(1, &Field2dSettings::default()).unwrap_err();
        assert_eq!(error.code, SHAPE_INVALID);
    }

    #[test]
    fn the_capacitance_matrix_is_symmetric_and_the_charges_balance() {
        let mut grid = Grid::new(
            uniform_lines(0.0, 1.0, 16),
            uniform_lines(0.0, 1.0, 16),
            Walls::GROUNDED,
            1000,
        )
        .unwrap();
        grid.paint_conductor(Rect::new(0.2, 0.4, 0.4, 0.6), 1);
        grid.paint_conductor(Rect::new(0.6, 0.8, 0.4, 0.6), 2);
        let settings = Field2dSettings {
            tolerance: 1.0e-13,
            ..Field2dSettings::default()
        };
        let solution = grid.solve(2, &settings).unwrap();
        let c = &solution.c;
        assert!(((c[0][1] - c[1][0]) / c[0][1]).abs() < 1.0e-8);
        assert!(c[0][0] > 0.0 && c[1][1] > 0.0);
        assert!(c[0][1] < 0.0 && c[1][0] < 0.0);
        // the charge on conductor 1 is more than the charge it induces on conductor 2
        assert!(c[0][0] > -c[1][0]);
        // the mesh is mirrored, so the two conductors have the same capacitance
        assert!(((c[0][0] - c[1][1]) / c[0][0]).abs() < 1.0e-8);
    }

    #[test]
    fn the_loss_tangent_of_a_uniform_dielectric_is_its_own() {
        let mut grid = plates(2.5, 0.01, 0.002, 4, 4);
        grid.paint_dielectric(Rect::new(0.0, 0.01, 0.0, 0.002), 2.5, 0.02);
        let solution = grid.solve(1, &Field2dSettings::default()).unwrap();
        let tan = grid.loss_tangent(&solution, &[1.0]);
        assert!((tan - 0.02).abs() < 1.0e-9, "{tan}");
    }

    #[test]
    fn without_a_dielectric_the_capacitance_is_that_of_vacuum() {
        let grid = plates(4.0, 0.01, 0.002, 4, 4);
        let settings = Field2dSettings {
            tolerance: 1.0e-13,
            ..Field2dSettings::default()
        };
        let with = grid.solve(1, &settings).unwrap().c[0][0];
        let without = grid.without_dielectric().solve(1, &settings).unwrap().c[0][0];
        assert!(
            ((with / without) - 4.0).abs() < 1.0e-8,
            "{}",
            with / without
        );
    }
}
