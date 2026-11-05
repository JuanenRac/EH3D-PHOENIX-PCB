// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on the circuit simulator: a netlist of the case is read, compiled and solved, and
//! one number is taken from the result.

use eh3d_phoenix_sim::{
    ac_at, dc_sweep, operating_point, parse, transient, Circuit, DcRequest, Method, SimSettings,
    TranRequest,
};

use crate::codes::{OracleError, CALC_FAILED};
use crate::compute::{calc, Inputs};

/// The circuit of the netlist of the inputs.
fn circuit(i: &Inputs, settings: &SimSettings) -> Result<Circuit, OracleError> {
    let text = i.lines("netlist")?;
    let netlist = parse(&text, settings).map_err(calc)?;
    Circuit::compile(&netlist, settings).map_err(calc)
}

fn missing(label: &str) -> OracleError {
    OracleError::new(CALC_FAILED, format!("the result has no unknown {label}"))
}

fn operating(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let result = operating_point(&circuit, &settings).map_err(calc)?;
    result.get(label).ok_or_else(|| missing(label))
}

/// The operating point at one value of a swept source.
fn sweep_point(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let request = DcRequest {
        source: i.text("source")?.to_string(),
        start: i.num("start")?,
        stop: i.num("stop")?,
        step: i.num("step")?,
    };
    let result = dc_sweep(&circuit, &settings, &request).map_err(calc)?;
    let series = result.series(label).ok_or_else(|| missing(label))?;
    let at = i.num("at")?;
    let slack = request.step.abs() * 1e-6;
    result
        .sweep
        .iter()
        .zip(series.iter())
        .find(|(value, _)| (**value - at).abs() <= slack)
        .map(|(_, y)| *y)
        .ok_or_else(|| {
            OracleError::new(
                CALC_FAILED,
                format!("the sweep has no point at {at} (the points are start + k step)"),
            )
        })
}

fn small_signal(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let result = ac_at(&circuit, &settings, &[i.num("freq_hz")?]).map_err(calc)?;
    let series = result.series(label).ok_or_else(|| missing(label))?;
    let point = series.first().copied().ok_or_else(|| missing(label))?;
    match i.text("quantity")? {
        "magnitude" => Ok(point.abs()),
        "phase_deg" => Ok(point.arg_deg()),
        other => Err(i.unknown("quantity", other)),
    }
}

fn step_response(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let tstep = i.num("tstep")?;
    let request = TranRequest {
        tstep,
        tstop: i.num("tstop")?,
        method: Method::Trapezoidal,
        adaptive: false,
        uic: false,
    };
    let result = transient(&circuit, &settings, &request).map_err(calc)?;
    let series = result.series(label).ok_or_else(|| missing(label))?;
    let at = i.num("at")?;
    let mut best: Option<(f64, f64)> = None;
    for (time, value) in result.times.iter().zip(series.iter()) {
        let distance = (time - at).abs();
        if best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, *value));
        }
    }
    match best {
        Some((distance, value)) if distance <= tstep => Ok(value),
        _ => Err(OracleError::new(
            CALC_FAILED,
            format!("no time step of the run is within one step of {at} s"),
        )),
    }
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "sim_op" => operating(i)?,
        "sim_ac" => small_signal(i)?,
        "sim_dc" => sweep_point(i)?,
        "sim_tran" => step_response(i)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
