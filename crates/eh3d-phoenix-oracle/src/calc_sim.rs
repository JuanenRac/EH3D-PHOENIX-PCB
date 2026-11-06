// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on the circuit simulator: a netlist of the case is read, compiled and solved, and
//! one number is taken from the result.

use std::collections::BTreeMap;

use eh3d_phoenix_sim::{
    ac_analysis, ac_at, dc_sweep, noise_analysis, operating_point, parse, parse_with_params,
    pole_zero, run_study, transient, AcRequest, Circuit, DcRequest, McSpec, Method, SimSettings,
    StudyPlan, StudyTable, SweepKind, TranRequest,
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

/// The sweep of the case: `points_per_decade` points per decade from `start_hz` to `stop_hz`.
fn decade_sweep(i: &Inputs) -> Result<AcRequest, OracleError> {
    let points = i.num("points_per_decade")?;
    if !(points >= 1.0 && points.fract() == 0.0 && points <= 1e4) {
        return Err(i.unknown("points_per_decade", &points.to_string()));
    }
    Ok(AcRequest {
        kind: SweepKind::Decade,
        points: points as usize,
        start_hz: i.num("start_hz")?,
        stop_hz: i.num("stop_hz")?,
    })
}

/// The table of the study that the lines of the netlist ask for (`.step`, `.measure` and the
/// analyses), with a Monte Carlo request when the case gives one.
fn study(i: &Inputs, mc: Option<McSpec>) -> Result<StudyTable, OracleError> {
    let settings = SimSettings::default();
    let text = i.lines("netlist")?;
    let netlist = parse(&text, &settings).map_err(calc)?;
    let mut plan = StudyPlan::from_netlist(&netlist);
    plan.mc = mc;
    let build = |overrides: &BTreeMap<String, f64>| parse_with_params(&text, &settings, overrides);
    run_study(&plan, &build, &settings).map_err(calc)
}

fn measure_column(table: &StudyTable, name: &str) -> Result<usize, OracleError> {
    table
        .measures
        .iter()
        .position(|m| m.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            OracleError::new(
                CALC_FAILED,
                format!("the study has no measure called {name}"),
            )
        })
}

/// One measure of a study with `.step` lines: the row `row` (counted from 0) of the table.
fn measure(i: &Inputs) -> Result<f64, OracleError> {
    let table = study(i, None)?;
    let column = measure_column(&table, i.text("name")?)?;
    let row = i.num_or("row", 0.0)?;
    let row = table
        .rows
        .get(row as usize)
        .ok_or_else(|| OracleError::new(CALC_FAILED, format!("the study has no row {row}")))?;
    row.values[column]
        .ok_or_else(|| OracleError::new(CALC_FAILED, "the measure finds nothing in this circuit"))
}

/// The mean or the deviation of a measure over the runs of a Monte Carlo study.
fn monte_carlo(i: &Inputs) -> Result<f64, OracleError> {
    let runs = i.num("runs")?;
    let seed = i.num("seed")?;
    if !(runs >= 1.0 && runs.fract() == 0.0 && seed >= 0.0 && seed.fract() == 0.0) {
        return Err(i.unknown("runs", &runs.to_string()));
    }
    let mc = McSpec {
        runs: runs as usize,
        seed: seed as u64,
    };
    let table = study(i, Some(mc))?;
    let column = measure_column(&table, i.text("name")?)?;
    let summary = table
        .summaries
        .first()
        .and_then(|group| group.per_measure.get(column))
        .and_then(|s| s.as_ref())
        .ok_or_else(|| OracleError::new(CALC_FAILED, "the study has no statistics"))?;
    match i.text("quantity")? {
        "mean" => Ok(summary.mean),
        "std" => Ok(summary.std),
        other => Err(i.unknown("quantity", other)),
    }
}

/// The noise at an output: the density at the first frequency, or the total in the band.
fn noise(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let result =
        noise_analysis(&circuit, &settings, i.text("probe")?, &decade_sweep(i)?).map_err(calc)?;
    match i.text("quantity")? {
        "density_v_per_sqrt_hz" => result
            .voltage_density()
            .first()
            .copied()
            .ok_or_else(|| OracleError::new(CALC_FAILED, "the sweep has no frequency")),
        "rms_v" => Ok(result.rms_volts),
        other => Err(i.unknown("quantity", other)),
    }
}

/// The frequency or the quality factor of a pole of the response of one signal.
fn poles(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let result = ac_analysis(&circuit, &settings, &decade_sweep(i)?).map_err(calc)?;
    let series = result.series(label).ok_or_else(|| missing(label))?;
    let found = pole_zero(&result.freqs, &series, None, &settings).map_err(calc)?;
    let index = i.num_or("index", 0.0)? as usize;
    let pole = found.poles.get(index).copied().ok_or_else(|| {
        OracleError::new(
            CALC_FAILED,
            format!(
                "the response has {} poles, not {}",
                found.poles.len(),
                index + 1
            ),
        )
    })?;
    match i.text("quantity")? {
        "pole_hz" => Ok(eh3d_phoenix_sim::poles::frequency_hz(pole)),
        "pole_q" => eh3d_phoenix_sim::poles::quality(pole)
            .ok_or_else(|| OracleError::new(CALC_FAILED, "the pole is real and has no quality")),
        other => Err(i.unknown("quantity", other)),
    }
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "sim_op" => operating(i)?,
        "sim_ac" => small_signal(i)?,
        "sim_dc" => sweep_point(i)?,
        "sim_tran" => step_response(i)?,
        "sim_measure" => measure(i)?,
        "sim_mc" => monte_carlo(i)?,
        "sim_noise" => noise(i)?,
        "sim_poles" => poles(i)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
