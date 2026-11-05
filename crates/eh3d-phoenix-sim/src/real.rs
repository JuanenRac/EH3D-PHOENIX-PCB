// SPDX-License-Identifier: GPL-3.0-or-later
//! The equations of a circuit in real numbers (operating point and transient): the stamps of the
//! elements, the companion models of capacitors and inductors, the linearised diodes and the
//! Newton-Raphson iteration.
//!
//! Sign conventions follow SPICE: the current of a voltage source is the one that flows through
//! it from the positive node to the negative node, and a current source takes its current from
//! the first node and gives it to the second.

use crate::circuit::Circuit;
use crate::codes::{SimError, NOT_FINITE, NO_CONVERGENCE, SINGULAR_MATRIX};
use crate::linalg::solve_real;
use crate::netlist::Source;
use crate::settings::SimSettings;

/// How the derivative of a capacitor or an inductor is replaced by a difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Backward Euler: first order, stable for any step.
    BackwardEuler,
    /// The trapezoidal rule: second order.
    Trapezoidal,
}

/// What the reactive elements remember between two steps.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The solution at the end of the last step.
    pub x: Vec<f64>,
    /// The voltage of each capacitor.
    pub cap_v: Vec<f64>,
    /// The current of each capacitor.
    pub cap_i: Vec<f64>,
    /// The current of each inductor.
    pub ind_i: Vec<f64>,
    /// The voltage of each inductor.
    pub ind_v: Vec<f64>,
    /// True before the first step: the step is then made with backward Euler, which needs no
    /// history of the currents.
    pub fresh: bool,
}

/// The voltage of a node in a solution (node 0 is ground).
pub fn volt(x: &[f64], node: usize) -> f64 {
    match node.checked_sub(1) {
        Some(i) => x[i],
        None => 0.0,
    }
}

fn row(node: usize) -> Option<usize> {
    node.checked_sub(1)
}

impl State {
    /// The state of a circuit at a solution. The currents of the capacitors are taken as zero.
    pub fn from_solution(c: &Circuit, x: Vec<f64>) -> State {
        let cap_v: Vec<f64> = c
            .capacitors
            .iter()
            .map(|e| volt(&x, e.a) - volt(&x, e.b))
            .collect();
        let cap_i = vec![0.0; c.capacitors.len()];
        let ind_i: Vec<f64> = c
            .inductors
            .iter()
            .map(|e| x[c.branch_row(e.branch)])
            .collect();
        let ind_v: Vec<f64> = c
            .inductors
            .iter()
            .map(|e| volt(&x, e.a) - volt(&x, e.b))
            .collect();
        State {
            x,
            cap_v,
            cap_i,
            ind_i,
            ind_v,
            fresh: true,
        }
    }

    /// The method that is really used for the next step.
    pub fn method_for(&self, requested: Method) -> Method {
        if self.fresh {
            Method::BackwardEuler
        } else {
            requested
        }
    }

    /// The state after a step of `dt` made with `method` that ended in the solution `x`.
    pub fn advance(&self, c: &Circuit, x: Vec<f64>, dt: f64, method: Method) -> State {
        let mut cap_v = Vec::with_capacity(c.capacitors.len());
        let mut cap_i = Vec::with_capacity(c.capacitors.len());
        for (k, cap) in c.capacitors.iter().enumerate() {
            let v = volt(&x, cap.a) - volt(&x, cap.b);
            let (geq, ieq) = cap_companion(cap.farads, dt, method, self.cap_v[k], self.cap_i[k]);
            cap_v.push(v);
            cap_i.push(geq * v - ieq);
        }
        let ind_i: Vec<f64> = c
            .inductors
            .iter()
            .map(|e| x[c.branch_row(e.branch)])
            .collect();
        let ind_v: Vec<f64> = c
            .inductors
            .iter()
            .map(|e| volt(&x, e.a) - volt(&x, e.b))
            .collect();
        State {
            x,
            cap_v,
            cap_i,
            ind_i,
            ind_v,
            fresh: false,
        }
    }
}

/// The conductance and the current source that replace a capacitor in one step.
fn cap_companion(farads: f64, dt: f64, method: Method, v_prev: f64, i_prev: f64) -> (f64, f64) {
    match method {
        Method::BackwardEuler => {
            let geq = farads / dt;
            (geq, geq * v_prev)
        }
        Method::Trapezoidal => {
            let geq = 2.0 * farads / dt;
            (geq, geq * v_prev + i_prev)
        }
    }
}

/// A transient step: the size, the method that is really used and what came before.
#[derive(Debug, Clone, Copy)]
pub struct Companion<'a> {
    /// The step in seconds.
    pub dt: f64,
    /// The method used for this step.
    pub method: Method,
    /// The state at the start of the step.
    pub state: &'a State,
}

/// What an assembly is for.
#[derive(Debug, Clone, Copy)]
pub struct Mode<'a> {
    /// The time of the sources; `None` takes the DC value of each source.
    pub time: Option<f64>,
    /// The step of a transient analysis; `None` for a DC solution.
    pub tran: Option<Companion<'a>>,
    /// Solve the initial conditions: capacitors hold their initial voltage, inductors their
    /// initial current.
    pub ic: bool,
    /// The factor applied to every independent source (source stepping).
    pub scale: f64,
    /// A conductance from every node to ground, in siemens.
    pub gmin_nodes: f64,
}

impl Mode<'static> {
    /// The DC solution with the sources at their DC value.
    pub fn dc(gmin_nodes: f64) -> Mode<'static> {
        Mode {
            time: None,
            tran: None,
            ic: false,
            scale: 1.0,
            gmin_nodes,
        }
    }
}

fn add(a: &mut [f64], n: usize, r: Option<usize>, col: Option<usize>, v: f64) {
    if let (Some(r), Some(col)) = (r, col) {
        a[r * n + col] += v;
    }
}

fn conduct(a: &mut [f64], n: usize, p: usize, q: usize, g: f64) {
    let (rp, rq) = (row(p), row(q));
    add(a, n, rp, rp, g);
    add(a, n, rq, rq, g);
    add(a, n, rp, rq, -g);
    add(a, n, rq, rp, -g);
}

/// A current `i` that flows from `p` to `q` through an element and does not depend on the
/// unknowns.
fn through(b: &mut [f64], p: usize, q: usize, i: f64) {
    if let Some(r) = row(p) {
        b[r] -= i;
    }
    if let Some(r) = row(q) {
        b[r] += i;
    }
}

/// The current of a branch leaves node `p` and enters node `q`.
fn branch_kcl(a: &mut [f64], n: usize, nn: usize, p: usize, q: usize, branch: usize) {
    add(a, n, row(p), Some(nn + branch), 1.0);
    add(a, n, row(q), Some(nn + branch), -1.0);
}

/// The row of a branch holds `v(p) - v(q)`.
fn branch_voltage(a: &mut [f64], n: usize, nn: usize, p: usize, q: usize, branch: usize) {
    add(a, n, Some(nn + branch), row(p), 1.0);
    add(a, n, Some(nn + branch), row(q), -1.0);
}

/// Limits the change of the voltage of a junction between two iterations, as SPICE does.
pub fn pnjlim(v_new: f64, v_old: f64, vt: f64, v_crit: f64) -> f64 {
    if v_new > v_crit && (v_new - v_old).abs() > 2.0 * vt {
        if v_old > 0.0 {
            let arg = 1.0 + (v_new - v_old) / vt;
            if arg > 0.0 {
                v_old + vt * arg.ln()
            } else {
                v_crit
            }
        } else {
            vt * (v_new / vt).ln()
        }
    } else {
        v_new
    }
}

/// The current and the conductance of a diode at a voltage (the conductance includes `gmin`).
pub fn diode_point(is: f64, nvt: f64, gmin: f64, v: f64) -> (f64, f64) {
    let e = (v / nvt).min(700.0).exp();
    (is * (e - 1.0), is / nvt * e + gmin)
}

fn source_value(source: &Source, mode: &Mode) -> f64 {
    let raw = match mode.time {
        Some(t) => source.value_at(t),
        None => source.dc_value(),
    };
    mode.scale * raw
}

/// Builds the matrix and the right-hand side at the iterate `x`. `vd` holds, for each diode, the
/// junction voltage used by the last iteration and receives the one used by this one. The last
/// value is true when a diode voltage was limited.
pub fn assemble(
    c: &Circuit,
    s: &SimSettings,
    mode: &Mode,
    x: &[f64],
    vd: &mut [f64],
) -> (Vec<f64>, Vec<f64>, bool) {
    let n = c.unknowns();
    let nn = c.node_names.len();
    let mut a = vec![0.0; n * n];
    let mut b = vec![0.0; n];
    let mut limited = false;

    for e in &c.resistors {
        conduct(&mut a, n, e.a, e.b, 1.0 / e.ohms);
    }

    for (k, e) in c.capacitors.iter().enumerate() {
        if let Some(co) = &mode.tran {
            let (geq, ieq) = cap_companion(
                e.farads,
                co.dt,
                co.method,
                co.state.cap_v[k],
                co.state.cap_i[k],
            );
            conduct(&mut a, n, e.a, e.b, geq);
            through(&mut b, e.a, e.b, -ieq);
        } else if mode.ic {
            let g = s.ic_conductance;
            conduct(&mut a, n, e.a, e.b, g);
            through(&mut b, e.a, e.b, -g * e.ic.unwrap_or(0.0));
        }
    }

    for (k, e) in c.inductors.iter().enumerate() {
        let r = nn + e.branch;
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        if mode.ic && mode.tran.is_none() {
            a[r * n + r] += 1.0;
            b[r] = e.ic.unwrap_or(0.0);
            continue;
        }
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        if let Some(co) = &mode.tran {
            let coef = match co.method {
                Method::BackwardEuler => 1.0 / co.dt,
                Method::Trapezoidal => 2.0 / co.dt,
            };
            a[r * n + r] -= coef * e.henries;
            let mut history = coef * e.henries * co.state.ind_i[k];
            for (j, m) in &e.partners {
                let col = nn + c.inductors[*j].branch;
                a[r * n + col] -= coef * m;
                history += coef * m * co.state.ind_i[*j];
            }
            b[r] = -history;
            if co.method == Method::Trapezoidal {
                b[r] -= co.state.ind_v[k];
            }
        }
    }

    for e in &c.vsources {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        b[nn + e.branch] = source_value(&e.source, mode);
    }

    for e in &c.isources {
        through(&mut b, e.a, e.b, source_value(&e.source, mode));
    }

    for e in &c.vcvs {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        add(&mut a, n, Some(nn + e.branch), row(e.cp), -e.gain);
        add(&mut a, n, Some(nn + e.branch), row(e.cn), e.gain);
    }

    for e in &c.vccs {
        add(&mut a, n, row(e.a), row(e.cp), e.gm);
        add(&mut a, n, row(e.a), row(e.cn), -e.gm);
        add(&mut a, n, row(e.b), row(e.cp), -e.gm);
        add(&mut a, n, row(e.b), row(e.cn), e.gm);
    }

    for e in &c.cccs {
        add(&mut a, n, row(e.a), Some(nn + e.control), e.gain);
        add(&mut a, n, row(e.b), Some(nn + e.control), -e.gain);
    }

    for e in &c.ccvs {
        branch_kcl(&mut a, n, nn, e.a, e.b, e.branch);
        branch_voltage(&mut a, n, nn, e.a, e.b, e.branch);
        add(&mut a, n, Some(nn + e.branch), Some(nn + e.control), -e.rm);
    }

    let vt = s.thermal_voltage();
    for (k, d) in c.diodes.iter().enumerate() {
        let nvt = d.n * vt;
        let v_crit = nvt * (nvt / (std::f64::consts::SQRT_2 * d.is)).ln();
        let raw = volt(x, d.a) - volt(x, d.k);
        let v = pnjlim(raw, vd[k], nvt, v_crit);
        if v != raw {
            limited = true;
        }
        vd[k] = v;
        let (id, gd) = diode_point(d.is, nvt, s.gmin, v);
        conduct(&mut a, n, d.a, d.k, gd);
        through(&mut b, d.a, d.k, id - gd * v);
    }

    if mode.gmin_nodes > 0.0 {
        for r in 0..nn {
            a[r * n + r] += mode.gmin_nodes;
        }
    }
    (a, b, limited)
}

/// The error for a matrix that cannot be solved: it names the unknown left undetermined.
pub fn singular_error(c: &Circuit, column: usize) -> SimError {
    let labels = c.labels();
    let label = labels.get(column).map_or("?", String::as_str);
    SimError::new(
        SINGULAR_MATRIX,
        format!(
            "the equations leave {label} undetermined (a loop of voltage sources or inductors, a node held only by current sources, or an element that is not connected)"
        ),
    )
}

/// Solves the assembled system.
pub fn solve_system(
    c: &Circuit,
    s: &SimSettings,
    mut a: Vec<f64>,
    mut b: Vec<f64>,
) -> Result<Vec<f64>, SimError> {
    let n = c.unknowns();
    match solve_real(&mut a, &mut b, n, s.pivot_tolerance) {
        Ok(()) => {
            if b.iter().any(|v| !v.is_finite()) {
                Err(SimError::new(
                    NOT_FINITE,
                    "the solution of the equations is not a finite number".to_string(),
                ))
            } else {
                Ok(b)
            }
        }
        Err(column) => Err(singular_error(c, column)),
    }
}

fn converged(c: &Circuit, s: &SimSettings, old: &[f64], new: &[f64]) -> bool {
    let nn = c.node_names.len();
    old.iter().zip(new.iter()).enumerate().all(|(k, (o, v))| {
        let absolute = if k < nn { s.vntol } else { s.abstol };
        (v - o).abs() <= s.reltol * v.abs().max(o.abs()) + absolute
    })
}

/// Solves the circuit in a mode, starting from `x0`. A circuit without diodes is solved in one
/// step; otherwise Newton-Raphson iterates until the change is below the tolerances and no diode
/// voltage was limited. Returns the solution and the number of iterations.
pub fn newton(
    c: &Circuit,
    s: &SimSettings,
    mode: &Mode,
    x0: &[f64],
) -> Result<(Vec<f64>, usize), SimError> {
    let nonlinear = c.is_nonlinear();
    let mut vd: Vec<f64> = c
        .diodes
        .iter()
        .map(|d| volt(x0, d.a) - volt(x0, d.k))
        .collect();
    let mut x = x0.to_vec();
    let max_iter = if nonlinear { s.newton_max_iter } else { 1 };
    for iteration in 1..=max_iter {
        let (a, b, limited) = assemble(c, s, mode, &x, &mut vd);
        let next = solve_system(c, s, a, b)?;
        if !nonlinear {
            return Ok((next, 1));
        }
        let done = !limited && converged(c, s, &x, &next);
        x = next;
        if done {
            return Ok((x, iteration));
        }
    }
    Err(SimError::new(
        NO_CONVERGENCE,
        format!(
            "no convergence after {} iterations (newton_max_iter)",
            s.newton_max_iter
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limiter_leaves_small_steps_alone_and_tames_large_ones() {
        let (vt, crit) = (0.025_865f64, 0.73f64);
        assert_eq!(pnjlim(0.5, 0.49, vt, crit), 0.5);
        assert_eq!(pnjlim(5.0, 0.0, vt, crit), vt * (5.0 / vt).ln());
        let limited = pnjlim(5.0, 0.7, vt, crit);
        assert!(limited > 0.7 && limited < 1.0, "{limited}");
        assert_eq!(pnjlim(-3.0, 0.2, vt, crit), -3.0);
    }

    #[test]
    fn the_diode_follows_the_shockley_equation() {
        let (id, gd) = diode_point(1e-14, 0.025_865, 0.0, 0.0);
        assert_eq!(id, 0.0);
        assert!((gd - 1e-14 / 0.025_865).abs() < 1e-24);
        let (id, _) = diode_point(1e-14, 0.025_865, 0.0, -5.0);
        assert!((id + 1e-14).abs() < 1e-20);
        let (id, gd) = diode_point(1e-14, 0.025_865, 1e-12, 0.6);
        assert!(id > 1e-4 && gd > id / 0.03);
    }
}
