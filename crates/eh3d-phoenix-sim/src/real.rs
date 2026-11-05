// SPDX-License-Identifier: GPL-3.0-or-later
//! The equations of a circuit in real numbers (operating point and transient): the stamps of the
//! elements, the companion models of capacitors and inductors, the linearised devices (diodes,
//! bipolar transistors, MOSFETs and the output stage of the op-amps) and the Newton-Raphson
//! iteration.
//!
//! Sign conventions follow SPICE: the current of a voltage source is the one that flows through
//! it from the positive node to the negative node, and a current source takes its current from
//! the first node and gives it to the second.

use crate::circuit::{Bjt, Circuit, JunctionCap, Saturator};
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
    /// The charge of each junction capacitance.
    pub jq: Vec<f64>,
    /// The rate of change of the charge of each junction capacitance.
    pub ji: Vec<f64>,
    /// True before the first step and after a breakpoint: the step is then made with backward
    /// Euler, which needs no history of the currents.
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

/// The voltage across a junction capacitance, turned by the polarity of its device.
fn junction_voltage(jc: &JunctionCap, x: &[f64]) -> f64 {
    jc.polarity * (volt(x, jc.a) - volt(x, jc.k))
}

/// The two coefficients of the difference that replaces `d/dt` in one step: the current is
/// `alpha * (q - q_before) - beta * i_before`.
fn difference_coefficients(dt: f64, method: Method) -> (f64, f64) {
    match method {
        Method::BackwardEuler => (1.0 / dt, 0.0),
        Method::Trapezoidal => (2.0 / dt, 1.0),
    }
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
        let jq: Vec<f64> = c
            .jcaps
            .iter()
            .map(|j| junction_charge(j, junction_voltage(j, &x)).0)
            .collect();
        let ji = vec![0.0; c.jcaps.len()];
        State {
            x,
            cap_v,
            cap_i,
            ind_i,
            ind_v,
            jq,
            ji,
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
        let (alpha, beta) = difference_coefficients(dt, method);
        let mut jq = Vec::with_capacity(c.jcaps.len());
        let mut ji = Vec::with_capacity(c.jcaps.len());
        for (k, j) in c.jcaps.iter().enumerate() {
            let q = junction_charge(j, junction_voltage(j, &x)).0;
            ji.push(alpha * (q - self.jq[k]) - beta * self.ji[k]);
            jq.push(q);
        }
        State {
            x,
            cap_v,
            cap_i,
            ind_i,
            ind_v,
            jq,
            ji,
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

/// The current and the conductance of the reverse breakdown of a diode at a voltage: the
/// current is `-ibv` at `v = -bv` and grows exponentially beyond it.
pub fn breakdown_point(bv: f64, ibv: f64, nbvt: f64, v: f64) -> (f64, f64) {
    let e = (-(v + bv) / nbvt).min(700.0).exp();
    (-ibv * e, ibv / nbvt * e)
}

/// The charge and the capacitance of the depletion layer of a junction at a voltage, with the
/// linear extension above `fc * vj` of SPICE.
pub fn depletion(cj0: f64, vj: f64, m: f64, fc: f64, v: f64) -> (f64, f64) {
    if cj0 <= 0.0 {
        return (0.0, 0.0);
    }
    let corner = fc * vj;
    let log_form = (1.0 - m).abs() < 1e-9;
    if v < corner {
        let arg = 1.0 - v / vj;
        if log_form {
            (-cj0 * vj * arg.ln(), cj0 / arg)
        } else {
            (
                cj0 * vj / (1.0 - m) * (1.0 - arg.powf(1.0 - m)),
                cj0 * arg.powf(-m),
            )
        }
    } else {
        let f1 = if log_form {
            -vj * (1.0 - fc).ln()
        } else {
            vj / (1.0 - m) * (1.0 - (1.0 - fc).powf(1.0 - m))
        };
        let f2 = (1.0 - fc).powf(1.0 + m);
        let f3 = 1.0 - fc * (1.0 + m);
        (
            cj0 * (f1 + (f3 * (v - corner) + m / (2.0 * vj) * (v * v - corner * corner)) / f2),
            cj0 * (f3 + m * v / vj) / f2,
        )
    }
}

/// The charge and the capacitance of a junction at a voltage: the depletion layer plus the
/// diffusion charge of the transit time.
pub fn junction_charge(jc: &JunctionCap, v: f64) -> (f64, f64) {
    let (mut q, mut cap) = depletion(jc.cj0, jc.vj, jc.m, jc.fc, v);
    if jc.tt > 0.0 {
        let e = (v / jc.nvt).min(700.0).exp();
        q += jc.tt * jc.is * (e - 1.0);
        cap += jc.tt * jc.is / jc.nvt * e;
    }
    (q, cap)
}

/// The currents of a bipolar transistor in the polarity of an NPN one, and their derivatives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BjtPoint {
    /// The collector current (into the collector).
    pub ic: f64,
    /// The base current (into the base).
    pub ib: f64,
    /// The derivative of `ic` with respect to the base-emitter voltage.
    pub dic_dvbe: f64,
    /// The derivative of `ic` with respect to the base-collector voltage.
    pub dic_dvbc: f64,
    /// The derivative of `ib` with respect to the base-emitter voltage.
    pub dib_dvbe: f64,
    /// The derivative of `ib` with respect to the base-collector voltage.
    pub dib_dvbc: f64,
}

/// The transport form of Ebers-Moll with the forward Early effect, at the junction voltages
/// `vbe` and `vbc` of an NPN transistor. The conductances of the junctions include `gmin`.
pub fn bjt_point(q: &Bjt, vt: f64, gmin: f64, vbe: f64, vbc: f64) -> BjtPoint {
    let nfvt = q.nf * vt;
    let nrvt = q.nr * vt;
    let ef = (vbe / nfvt).min(700.0).exp();
    let er = (vbc / nrvt).min(700.0).exp();
    let i_f = q.is * (ef - 1.0);
    let i_r = q.is * (er - 1.0);
    let gif = q.is / nfvt * ef;
    let gir = q.is / nrvt * er;
    let (early, dearly) = match q.vaf {
        Some(va) => {
            let e = 1.0 - vbc / va;
            if e > 0.01 {
                (e, -1.0 / va)
            } else {
                (0.01, 0.0)
            }
        }
        None => (1.0, 0.0),
    };
    let it = (i_f - i_r) * early;
    let dit_dvbe = gif * early;
    let dit_dvbc = -gir * early + (i_f - i_r) * dearly;
    let gbe = gif / q.bf + gmin;
    let gbc = gir / q.br + gmin;
    BjtPoint {
        ic: it - i_r / q.br,
        ib: i_f / q.bf + i_r / q.br,
        dic_dvbe: dit_dvbe,
        dic_dvbc: dit_dvbc - gbc,
        dib_dvbe: gbe,
        dib_dvbc: gbc,
    }
}

/// The drain current, the transconductance and the output conductance of a MOSFET of level 1
/// with the polarity of an N channel, for `vds >= 0`. `vto` is the threshold voltage.
pub fn mos_point(beta: f64, vto: f64, lambda: f64, vgs: f64, vds: f64) -> (f64, f64, f64) {
    let vov = vgs - vto;
    if vov <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let clm = 1.0 + lambda * vds;
    if vds < vov {
        let core = vov * vds - vds * vds / 2.0;
        (
            beta * core * clm,
            beta * vds * clm,
            beta * (vov - vds) * clm + beta * core * lambda,
        )
    } else {
        let sat = beta / 2.0 * vov * vov;
        (sat * clm, beta * vov * clm, sat * lambda)
    }
}

/// The output and the slope of an output stage at an input difference.
pub fn sat_point(sat: &Saturator, vin: f64) -> (f64, f64) {
    let u = vin - sat.offset;
    if sat.limit {
        let t = (sat.slope * u / sat.half).tanh();
        (sat.mid + sat.half * t, sat.slope * (1.0 - t * t))
    } else {
        (sat.mid + sat.slope * u, sat.slope)
    }
}

/// A linearised piece of a device: the current that leaves `node` is `ieq + sum(g * v(k))` over
/// the pairs `(k, g)` of `d`.
#[derive(Debug, Clone, PartialEq)]
pub struct Lin {
    /// The node the current leaves.
    pub node: usize,
    /// The part of the current that does not depend on the unknowns.
    pub ieq: f64,
    /// The nodes it depends on and the conductance to each.
    pub d: Vec<(usize, f64)>,
}

/// The junction voltage of every junction of the circuit at a solution.
pub fn junction_voltages(c: &Circuit, x: &[f64]) -> Vec<f64> {
    let mut vj = vec![0.0; c.junctions];
    for d in &c.diodes {
        vj[d.j] = volt(x, d.a) - volt(x, d.k);
    }
    for q in &c.bjts {
        vj[q.j] = q.polarity * (volt(x, q.b) - volt(x, q.e));
        vj[q.j + 1] = q.polarity * (volt(x, q.b) - volt(x, q.c));
    }
    vj
}

fn crit(nvt: f64, is: f64) -> f64 {
    nvt * (nvt / (std::f64::consts::SQRT_2 * is)).ln()
}

/// Linearises every non-linear device (and, in a transient step, every junction capacitance)
/// around the iterate `x`. `vj` holds the junction voltages used by the last iteration and
/// receives the ones used by this one. The last value is true when a junction voltage was
/// limited.
pub fn linearize(
    c: &Circuit,
    s: &SimSettings,
    tran: Option<&Companion>,
    x: &[f64],
    vj: &mut [f64],
) -> (Vec<Lin>, bool) {
    let vt = s.thermal_voltage();
    let mut lins: Vec<Lin> = Vec::new();
    let mut limited = false;

    for d in &c.diodes {
        let nvt = d.n * vt;
        let raw = volt(x, d.a) - volt(x, d.k);
        let mut v = pnjlim(raw, vj[d.j], nvt, crit(nvt, d.is));
        if let Some(bv) = d.bv {
            if raw < -bv {
                let nbvt = d.nbv * vt;
                let target = -(raw + bv);
                let back = pnjlim(target, -(vj[d.j] + bv), nbvt, crit(nbvt, d.ibv));
                if back != target {
                    v = -back - bv;
                }
            }
        }
        if v != raw {
            limited = true;
        }
        vj[d.j] = v;
        let (mut id, mut gd) = diode_point(d.is, nvt, s.gmin, v);
        if let Some(bv) = d.bv {
            let (ib, gb) = breakdown_point(bv, d.ibv, d.nbv * vt, v);
            id += ib;
            gd += gb;
        }
        let ieq = id - gd * v;
        lins.push(Lin {
            node: d.a,
            ieq,
            d: vec![(d.a, gd), (d.k, -gd)],
        });
        lins.push(Lin {
            node: d.k,
            ieq: -ieq,
            d: vec![(d.a, -gd), (d.k, gd)],
        });
    }

    for q in &c.bjts {
        let p = q.polarity;
        let raw_be = p * (volt(x, q.b) - volt(x, q.e));
        let raw_bc = p * (volt(x, q.b) - volt(x, q.c));
        let (nfvt, nrvt) = (q.nf * vt, q.nr * vt);
        let vbe = pnjlim(raw_be, vj[q.j], nfvt, crit(nfvt, q.is));
        let vbc = pnjlim(raw_bc, vj[q.j + 1], nrvt, crit(nrvt, q.is));
        if vbe != raw_be || vbc != raw_bc {
            limited = true;
        }
        vj[q.j] = vbe;
        vj[q.j + 1] = vbc;
        let pt = bjt_point(q, vt, s.gmin, vbe, vbc);
        let ieq_c = p * (pt.ic - pt.dic_dvbe * vbe - pt.dic_dvbc * vbc);
        let ieq_b = p * (pt.ib - pt.dib_dvbe * vbe - pt.dib_dvbc * vbc);
        let dc = [
            (q.b, pt.dic_dvbe + pt.dic_dvbc),
            (q.e, -pt.dic_dvbe),
            (q.c, -pt.dic_dvbc),
        ];
        let db = [
            (q.b, pt.dib_dvbe + pt.dib_dvbc),
            (q.e, -pt.dib_dvbe),
            (q.c, -pt.dib_dvbc),
        ];
        let de: Vec<(usize, f64)> = dc
            .iter()
            .zip(db.iter())
            .map(|(u, w)| (u.0, -(u.1 + w.1)))
            .collect();
        lins.push(Lin {
            node: q.c,
            ieq: ieq_c,
            d: dc.to_vec(),
        });
        lins.push(Lin {
            node: q.b,
            ieq: ieq_b,
            d: db.to_vec(),
        });
        lins.push(Lin {
            node: q.e,
            ieq: -(ieq_c + ieq_b),
            d: de,
        });
    }

    for m in &c.mosfets {
        let p = m.polarity;
        let swapped = p * (volt(x, m.d) - volt(x, m.s)) < 0.0;
        let (dn, sn) = if swapped { (m.s, m.d) } else { (m.d, m.s) };
        let vgs = p * (volt(x, m.g) - volt(x, sn));
        let vds = p * (volt(x, dn) - volt(x, sn));
        let (id, gm, gds) = mos_point(m.beta, m.vto, m.lambda, vgs, vds);
        let gds = gds + s.gmin;
        let ieq = p * (id - gm * vgs - gds * vds);
        lins.push(Lin {
            node: dn,
            ieq,
            d: vec![(m.g, gm), (dn, gds), (sn, -(gm + gds))],
        });
        lins.push(Lin {
            node: sn,
            ieq: -ieq,
            d: vec![(m.g, -gm), (dn, -gds), (sn, gm + gds)],
        });
    }

    for st in &c.sats {
        let vin = volt(x, st.cp) - volt(x, st.cn);
        let (f, df) = sat_point(st, vin);
        let g = 1.0 / st.rout;
        lins.push(Lin {
            node: st.out,
            ieq: (df * vin - f) * g,
            d: vec![(st.out, g), (st.cp, -df * g), (st.cn, df * g)],
        });
    }

    if let Some(co) = tran {
        let (alpha, beta) = difference_coefficients(co.dt, co.method);
        for (k, jc) in c.jcaps.iter().enumerate() {
            let (q, cap) = junction_charge(jc, junction_voltage(jc, x));
            let g = alpha * cap;
            let i0 = jc.polarity * (alpha * (q - co.state.jq[k]) - beta * co.state.ji[k]);
            let ieq = i0 - g * (volt(x, jc.a) - volt(x, jc.k));
            lins.push(Lin {
                node: jc.a,
                ieq,
                d: vec![(jc.a, g), (jc.k, -g)],
            });
            lins.push(Lin {
                node: jc.k,
                ieq: -ieq,
                d: vec![(jc.a, -g), (jc.k, g)],
            });
        }
    }
    (lins, limited)
}

/// The small-signal model of the non-linear devices at an operating point: the conductances,
/// and the capacitance of every junction.
#[derive(Debug, Clone, PartialEq)]
pub struct SmallSignal {
    /// The linearised devices (only `node` and `d` matter).
    pub lins: Vec<Lin>,
    /// The capacitance between two nodes: `(a, k, farads)`.
    pub caps: Vec<(usize, usize, f64)>,
}

/// Linearises the devices at the solution `x` of the operating point.
pub fn small_signal(c: &Circuit, s: &SimSettings, x: &[f64]) -> SmallSignal {
    let mut vj = junction_voltages(c, x);
    let (lins, _) = linearize(c, s, None, x, &mut vj);
    let caps = c
        .jcaps
        .iter()
        .map(|j| (j.a, j.k, junction_charge(j, junction_voltage(j, x)).1))
        .collect();
    SmallSignal { lins, caps }
}

fn source_value(source: &Source, mode: &Mode) -> f64 {
    let raw = match mode.time {
        Some(t) => source.value_at(t),
        None => source.dc_value(),
    };
    mode.scale * raw
}

/// Builds the matrix and the right-hand side at the iterate `x`. `vd` holds, for each junction,
/// the voltage used by the last iteration and receives the one used by this one. The last value
/// is true when a junction voltage was limited.
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

    let (lins, limited) = linearize(c, s, mode.tran.as_ref(), x, vd);
    for lin in &lins {
        for &(col, g) in &lin.d {
            add(&mut a, n, row(lin.node), row(col), g);
        }
        if let Some(r) = row(lin.node) {
            b[r] -= lin.ieq;
        }
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

/// Solves the circuit in a mode, starting from `x0`. A circuit without non-linear devices is
/// solved in one step; otherwise Newton-Raphson iterates until the change is below the tolerances
/// twice in a row and no junction voltage was limited. Returns the solution and the number of iterations.
pub fn newton(
    c: &Circuit,
    s: &SimSettings,
    mode: &Mode,
    x0: &[f64],
) -> Result<(Vec<f64>, usize), SimError> {
    let nonlinear = c.is_nonlinear();
    let mut vd = junction_voltages(c, x0);
    let mut x = x0.to_vec();
    let max_iter = if nonlinear { s.newton_max_iter } else { 1 };
    // the change must stay below the tolerances in two iterations in a row: the second one is
    // much smaller than the first, which makes the currents of the devices agree with the node
    // equations to far better than the tolerance
    let mut settled = false;
    for iteration in 1..=max_iter {
        let (a, b, limited) = assemble(c, s, mode, &x, &mut vd);
        let next = solve_system(c, s, a, b)?;
        if !nonlinear {
            return Ok((next, 1));
        }
        let close = !limited && converged(c, s, &x, &next);
        x = next;
        if close && settled {
            return Ok((x, iteration));
        }
        settled = close;
    }
    if settled {
        return Ok((x, max_iter));
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

    fn bjt(vaf: Option<f64>) -> Bjt {
        Bjt {
            name: "q1".to_string(),
            c: 1,
            b: 2,
            e: 0,
            polarity: 1.0,
            is: 1e-14,
            bf: 100.0,
            br: 2.0,
            nf: 1.0,
            nr: 1.0,
            vaf,
            j: 0,
        }
    }

    fn jcap(cj0: f64, m: f64, tt: f64) -> JunctionCap {
        JunctionCap {
            name: "cj".to_string(),
            a: 1,
            k: 0,
            polarity: 1.0,
            cj0,
            vj: 0.7,
            m,
            fc: 0.5,
            tt,
            is: 1e-14,
            nvt: 0.025_865,
        }
    }

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

    #[test]
    fn the_breakdown_current_is_the_knee_current_at_the_breakdown_voltage() {
        let (i, g) = breakdown_point(5.1, 1e-3, 0.025_865, -5.1);
        assert!((i + 1e-3).abs() < 1e-15);
        assert!((g - 1e-3 / 0.025_865).abs() < 1e-9);
        // two thousand times smaller 0.2 V above the knee, and nothing at zero volts
        let (small, _) = breakdown_point(5.1, 1e-3, 0.025_865, -4.9);
        assert!(small.abs() < 1e-6, "{small}");
        let (none, _) = breakdown_point(5.1, 1e-3, 0.025_865, 0.0);
        assert!(none.abs() < 1e-30);
    }

    #[test]
    fn the_depletion_capacitance_follows_the_textbook_and_continues_above_the_corner() {
        // C = cj0 / (1 + vr / vj)^m for a reverse voltage vr
        let (cj0, vj, m) = (2e-12, 0.7, 0.5);
        let (_, cap) = depletion(cj0, vj, m, 0.5, -3.0);
        let want = cj0 / (1.0f64 + 3.0 / 0.7).powf(0.5);
        assert!((cap - want).abs() < 1e-18, "{cap} {want}");
        assert!((cap - 0.8699e-12).abs() < 1e-15);
        // zero bias gives cj0, and the charge and the capacitance are continuous at the corner
        assert!((depletion(cj0, vj, m, 0.5, 0.0).1 - cj0).abs() < 1e-24);
        let corner = 0.5 * vj;
        let below = depletion(cj0, vj, m, 0.5, corner - 1e-9);
        let above = depletion(cj0, vj, m, 0.5, corner + 1e-9);
        assert!((below.0 - above.0).abs() < 1e-18);
        assert!((below.1 - above.1).abs() < 1e-17);
        // no capacitance without cj0
        assert_eq!(depletion(0.0, vj, m, 0.5, 0.3), (0.0, 0.0));
    }

    #[test]
    fn the_capacitance_is_the_derivative_of_the_charge() {
        for (cj0, m, tt) in [(2e-12, 0.5, 0.0), (1e-12, 1.0, 1e-9), (0.0, 0.5, 2e-9)] {
            let jc = jcap(cj0, m, tt);
            for v in [-4.0, -1.0, 0.0, 0.2, 0.4, 0.6] {
                let h = 1e-6;
                let numeric =
                    (junction_charge(&jc, v + h).0 - junction_charge(&jc, v - h).0) / (2.0 * h);
                let exact = junction_charge(&jc, v).1;
                assert!(
                    (numeric - exact).abs() <= 1e-5 * exact.abs() + 1e-18,
                    "cj0 {cj0} m {m} tt {tt} v {v}: {numeric} {exact}"
                );
            }
        }
        // the diffusion capacitance of a forward diode is tt times its conductance
        let jc = jcap(0.0, 0.5, 1e-9);
        let (_, cap) = junction_charge(&jc, 0.6);
        let (_, gd) = diode_point(1e-14, 0.025_865, 0.0, 0.6);
        assert!((cap - 1e-9 * gd).abs() < 1e-6 * cap);
    }

    #[test]
    fn the_transistor_currents_follow_ebers_moll() {
        let q = bjt(None);
        let (vt, vbe) = (0.025_865f64, 0.65f64);
        let p = bjt_point(&q, vt, 0.0, vbe, -2.0);
        let i_f = 1e-14 * ((vbe / vt).exp() - 1.0);
        // forward active: the collector current is the transport current, the base current is
        // a hundredth of it
        assert!((p.ic - i_f).abs() < 1e-9 * i_f + 1e-14);
        assert!((p.ib - i_f / 100.0).abs() < 1e-9 * i_f);
        assert!((p.ic / p.ib - 100.0).abs() < 1e-3);
        // with an Early voltage the collector current grows with the collector voltage
        let early = bjt(Some(50.0));
        let lower = bjt_point(&early, vt, 0.0, vbe, -1.0);
        let higher = bjt_point(&early, vt, 0.0, vbe, -6.0);
        assert!(higher.ic > lower.ic);
        let ratio = higher.ic / lower.ic;
        assert!((ratio - (1.0 + 6.0 / 50.0) / (1.0 + 1.0 / 50.0)).abs() < 1e-9);
    }

    #[test]
    fn the_derivatives_of_the_transistor_are_those_of_its_currents() {
        let q = bjt(Some(60.0));
        let vt = 0.025_865;
        for (vbe, vbc) in [(0.6, -3.0), (0.7, 0.3), (0.0, 0.0), (0.72, 0.71)] {
            let p = bjt_point(&q, vt, 0.0, vbe, vbc);
            let h = 1e-7;
            let at = |a: f64, b: f64| bjt_point(&q, vt, 0.0, a, b);
            let numeric = [
                (at(vbe + h, vbc).ic - at(vbe - h, vbc).ic) / (2.0 * h),
                (at(vbe, vbc + h).ic - at(vbe, vbc - h).ic) / (2.0 * h),
                (at(vbe + h, vbc).ib - at(vbe - h, vbc).ib) / (2.0 * h),
                (at(vbe, vbc + h).ib - at(vbe, vbc - h).ib) / (2.0 * h),
            ];
            let exact = [p.dic_dvbe, p.dic_dvbc, p.dib_dvbe, p.dib_dvbc];
            for (nu, ex) in numeric.iter().zip(exact.iter()) {
                assert!(
                    (nu - ex).abs() <= 1e-4 * ex.abs() + 1e-12,
                    "{vbe} {vbc}: {nu} {ex}"
                );
            }
        }
    }

    #[test]
    fn the_mosfet_has_three_regions_that_join_without_a_jump() {
        let (beta, vto, lambda) = (2e-4, 1.0, 0.02);
        // cut-off
        assert_eq!(mos_point(beta, vto, lambda, 0.5, 3.0), (0.0, 0.0, 0.0));
        // saturation: beta / 2 (vgs - vto)^2 (1 + lambda vds)
        let (id, gm, gds) = mos_point(beta, vto, lambda, 2.5, 4.0);
        let sat = beta / 2.0 * 1.5 * 1.5;
        assert!((id - sat * (1.0 + 0.08)).abs() < 1e-15);
        assert!((gm - beta * 1.5 * 1.08).abs() < 1e-15);
        assert!((gds - sat * 0.02).abs() < 1e-15);
        // triode, and the same current on both sides of vds = vgs - vto
        let (tri, _, _) = mos_point(beta, vto, lambda, 2.5, 1.5 - 1e-9);
        let (sat_edge, _, _) = mos_point(beta, vto, lambda, 2.5, 1.5 + 1e-9);
        assert!((tri - sat_edge).abs() < 1e-12);
        let (small, _, small_gds) = mos_point(beta, vto, lambda, 2.5, 0.01);
        // at a small vds the channel is a resistor of 1 / (beta (vgs - vto))
        assert!((small / 0.01 - beta * 1.5).abs() < 0.01 * beta * 1.5);
        assert!((small_gds - beta * 1.5).abs() < 0.02 * beta * 1.5);
    }

    #[test]
    fn the_output_stage_is_limited_and_has_its_slope_at_the_middle() {
        let limited = Saturator {
            name: "u".to_string(),
            out: 1,
            cp: 2,
            cn: 0,
            rout: 1.0,
            mid: 0.0,
            half: 15.0,
            slope: 1.0,
            offset: 0.0,
            limit: true,
        };
        let (f, df) = sat_point(&limited, 0.0);
        assert_eq!((f, df), (0.0, 1.0));
        let (f, df) = sat_point(&limited, 1000.0);
        assert!((f - 15.0).abs() < 1e-9 && df.abs() < 1e-9);
        let (f, _) = sat_point(&limited, -1000.0);
        assert!((f + 15.0).abs() < 1e-9);
        let free = Saturator {
            limit: false,
            half: 0.0,
            ..limited
        };
        assert_eq!(sat_point(&free, 2.5), (2.5, 1.0));
    }
}
