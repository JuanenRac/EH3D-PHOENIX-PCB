// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on signal integrity: the reflection coefficient, the bounces and the simulation of
//! a lossless line by the method of Bergeron, the rise time of a single pole, the bandwidth of an
//! edge and the delay of a line, of the signal integrity crate.

use std::f64::consts::PI;

use eh3d_phoenix_si::bergeron::{bounce_table, simulate, Input, Limits, Termination};
use eh3d_phoenix_si::phys::{bandwidth_ghz, delay_ps_per_mm, lowpass_rise_ps, reflection};

use crate::codes::OracleError;
use crate::compute::{calc, Inputs};

/// The limits of the simulations of the cases.
const LIMITS: Limits = Limits {
    steps_per_rise: 20,
    delays: 40,
    max_steps: 400_000,
    settle_band_pct: 5.0,
};

/// The resistance of the load of the inputs; a value of zero is an open end.
fn load(i: &Inputs) -> Result<Option<f64>, OracleError> {
    let r = i.num("rl_ohm")?;
    Ok((r > 0.0).then_some(r))
}

/// The inputs of a simulation with the load of the inputs and no capacitor.
fn lossless_line(i: &Inputs, td_ps: f64, tr_ps: f64) -> Result<Input, OracleError> {
    Ok(Input {
        z0: i.num("z0_ohm")?,
        td_ps,
        rs: i.num("rs_ohm")?,
        rl: load(i)?,
        cl_pf: 0.0,
        tr_ps,
        term: Termination::None,
    })
}

/// The voltage at the load after its n-th arrival, from the diagram of bounces.
fn load_arrival(i: &Inputs) -> Result<f64, OracleError> {
    let n = i.num("arrival")?;
    if !(1.0..=32.0).contains(&n) || n.fract() != 0.0 {
        return Err(calc("the arrival must be a whole number from 1 to 32"));
    }
    let table = bounce_table(i.num("z0_ohm")?, 1000.0, i.num("rs_ohm")?, load(i)?, 32);
    table
        .iter()
        .filter(|b| b.at == "load")
        .nth(n as usize - 1)
        .map(|b| b.node_v)
        .ok_or_else(|| calc("the diagram has no such arrival"))
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "si_bandwidth_ghz" => bandwidth_ghz(i.num("tr_ps")?),
        "si_bergeron_final_level" => {
            simulate(&lossless_line(i, 1000.0, 50.0)?, &LIMITS).final_level
        }
        "si_bergeron_load_arrival_v" => load_arrival(i)?,
        "si_bergeron_overshoot_pct" => {
            simulate(
                &lossless_line(i, i.num("td_ps")?, i.num("tr_ps")?)?,
                &LIMITS,
            )
            .overshoot_pct
        }
        "si_delay_ps_per_mm" => delay_ps_per_mm(i.num("eps_eff")?),
        "si_rc_rise_time_ps" => {
            // R in ohm times C in picofarad is a time in picoseconds
            let tau_ps = i.num("r_ohm")? * i.num("c_pf")?;
            let cutoff_ghz = 1.0e3 / (2.0 * PI * tau_ps);
            lowpass_rise_ps(cutoff_ghz, 1)
        }
        "si_reflection_coefficient" => reflection(i.num("zl_ohm")?, i.num("z0_ohm")?),
        _ => return Ok(None),
    };
    Ok(Some(value))
}
