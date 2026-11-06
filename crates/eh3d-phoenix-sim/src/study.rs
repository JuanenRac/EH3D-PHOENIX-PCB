// SPDX-License-Identifier: GPL-3.0-or-later
//! A study: one netlist run again and again, with the measures of each run collected in a table.
//! The runs come from three places that combine: the grid of the `.step` lines (the first is the
//! outermost), the Monte Carlo draws of the tolerances (the same draws for every point of the
//! grid, so the points can be compared) and the two runs around the nominal value of a parameter
//! that give a sensitivity.
//!
//! A study is a plan (steps, measures, the analyses to run, the Monte Carlo request) and a way to
//! make the netlist (a function of the `.param` values that a step replaces), so the same code
//! serves a netlist in memory, a file with `.include` lines and the command line.

use std::collections::BTreeMap;

use crate::ac::{ac_analysis, ac_at, AcRequest};
use crate::circuit::Circuit;
use crate::codes::{
    SimError, LIMIT_EXCEEDED, MC_INVALID, MEASURE_FAILED, MEASURE_INVALID, STEP_INVALID, UNDEFINED,
};
use crate::dc::{dc_sweep, DcRequest};
use crate::measure::{ac_point, ac_point_freq, ac_sweep, waveform, Domain, MeasureSpec};
use crate::montecarlo::{apply_tolerances, summarize, Summary, Tolerance};
use crate::netlist::{Directive, Netlist};
use crate::random::Rng;
use crate::settings::SimSettings;
use crate::step::{element_value, set_element, StepSpec, Target};
use crate::tran::{transient, TranRequest};

/// A Monte Carlo request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McSpec {
    /// How many runs for each point of the grid of steps.
    pub runs: usize,
    /// The seed: the same seed gives the same table.
    pub seed: u64,
}

/// What to run and what to measure.
#[derive(Debug, Clone, PartialEq)]
pub struct StudyPlan {
    /// The `.step` lines.
    pub steps: Vec<StepSpec>,
    /// The measures.
    pub measures: Vec<MeasureSpec>,
    /// The Monte Carlo request, if any.
    pub mc: Option<McSpec>,
    /// The tolerance by the letter of the element (`r`, `c`, `l`) for the elements that have none
    /// of their own.
    pub default_tolerances: BTreeMap<char, Tolerance>,
    /// The transient analysis the `tran` measures read.
    pub tran: Option<TranRequest>,
    /// The AC sweep the `ac` measures read.
    pub ac: Option<AcRequest>,
    /// The DC sweep the `dc` measures read.
    pub dc: Option<DcRequest>,
    /// The parameters whose sensitivity is asked for.
    pub sensitivity: Vec<Target>,
}

impl StudyPlan {
    /// The plan a netlist asks for: its `.step` and `.measure` lines and the first analysis of
    /// each kind. No Monte Carlo and no sensitivity.
    pub fn from_netlist(netlist: &Netlist) -> StudyPlan {
        let mut plan = StudyPlan {
            steps: netlist.steps.clone(),
            measures: netlist.measures.clone(),
            mc: None,
            default_tolerances: BTreeMap::new(),
            tran: None,
            ac: None,
            dc: None,
            sensitivity: Vec::new(),
        };
        for directive in &netlist.analyses {
            match directive {
                Directive::Tran { tstep, tstop, uic } if plan.tran.is_none() => {
                    plan.tran = Some(TranRequest {
                        tstep: *tstep,
                        tstop: *tstop,
                        method: crate::real::Method::Trapezoidal,
                        adaptive: false,
                        uic: *uic,
                    });
                }
                Directive::Ac {
                    kind,
                    points,
                    start_hz,
                    stop_hz,
                } if plan.ac.is_none() => {
                    plan.ac = Some(AcRequest {
                        kind: *kind,
                        points: *points,
                        start_hz: *start_hz,
                        stop_hz: *stop_hz,
                    });
                }
                Directive::Dc {
                    source,
                    start,
                    stop,
                    step,
                } if plan.dc.is_none() => {
                    plan.dc = Some(DcRequest {
                        source: source.clone(),
                        start: *start,
                        stop: *stop,
                        step: *step,
                    });
                }
                _ => {}
            }
        }
        plan
    }
}

/// One run of the study.
#[derive(Debug, Clone, PartialEq)]
pub struct StudyRow {
    /// The value of each step at this run, in the order of the steps.
    pub step: Vec<f64>,
    /// The number of the Monte Carlo run, counted from 0; `None` without Monte Carlo.
    pub run: Option<usize>,
    /// The value of each measure; `None` where the measure finds nothing.
    pub values: Vec<Option<f64>>,
}

/// The statistics of the Monte Carlo runs at one point of the grid of steps.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSummary {
    /// The value of each step at this point.
    pub step: Vec<f64>,
    /// For each measure, the statistics of the runs that found a value; `None` when none did.
    pub per_measure: Vec<Option<Summary>>,
}

/// The sensitivity of one measure to one parameter, from two runs a small relative step away.
#[derive(Debug, Clone, PartialEq)]
pub struct Sensitivity {
    /// The measure.
    pub measure: String,
    /// The parameter (`param:name` or the name of an element).
    pub target: String,
    /// The nominal value of the parameter.
    pub value: f64,
    /// The measure at the nominal value.
    pub nominal: Option<f64>,
    /// The derivative of the measure with respect to the parameter (central difference).
    pub derivative: Option<f64>,
    /// The relative sensitivity: the derivative times the value of the parameter over the
    /// measure (the fraction by which the measure changes for a fraction of change in the
    /// parameter).
    pub relative: Option<f64>,
}

/// The result of a study.
#[derive(Debug, Clone, PartialEq)]
pub struct StudyTable {
    /// The label of each step (`param:name` or the name of the element).
    pub axes: Vec<String>,
    /// The names of the measures.
    pub measures: Vec<String>,
    /// The Monte Carlo request, if any.
    pub mc: Option<McSpec>,
    /// The percentiles of the summaries.
    pub percentiles: Vec<f64>,
    /// Every run.
    pub rows: Vec<StudyRow>,
    /// The statistics at each point of the grid (with Monte Carlo only).
    pub summaries: Vec<GroupSummary>,
    /// The sensitivities asked for.
    pub sensitivities: Vec<Sensitivity>,
}

/// Makes the netlist from the values that replace some `.param` lines (lower case names).
pub type NetlistBuilder<'a> = dyn Fn(&BTreeMap<String, f64>) -> Result<Netlist, SimError> + 'a;

fn failed(why: String) -> SimError {
    SimError::new(MEASURE_FAILED, why)
}

fn domain_word(domain: Domain) -> &'static str {
    match domain {
        Domain::Tran => "tran",
        Domain::Dc => "dc",
        Domain::Ac => "ac",
    }
}

fn check_plan(plan: &StudyPlan, s: &SimSettings) -> Result<(), SimError> {
    if plan.measures.is_empty() {
        return Err(SimError::new(
            MEASURE_INVALID,
            "a study needs at least one measure (a .measure line or --measure)",
        ));
    }
    if plan.measures.len() > s.max_measures {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "{} measures are more than max_measures ({})",
                plan.measures.len(),
                s.max_measures
            ),
        ));
    }
    for (k, m) in plan.measures.iter().enumerate() {
        if plan.measures[..k]
            .iter()
            .any(|other| other.name.eq_ignore_ascii_case(&m.name))
        {
            return Err(SimError::new(
                MEASURE_INVALID,
                format!("the measure {} is defined twice", m.name),
            ));
        }
        let available = match m.domain {
            Domain::Tran => plan.tran.is_some(),
            Domain::Dc => plan.dc.is_some(),
            Domain::Ac => plan.ac.is_some(),
        };
        if !available {
            let word = domain_word(m.domain);
            return Err(failed(format!(
                "the measure {} reads the {word} analysis and none is asked for (a .{word} line or --{word})",
                m.name
            )));
        }
    }
    if let Some(mc) = &plan.mc {
        if mc.runs < 1 || mc.runs > s.max_mc_runs {
            return Err(SimError::new(
                MC_INVALID,
                format!(
                    "{} Monte Carlo runs: the number is between 1 and max_mc_runs ({})",
                    mc.runs, s.max_mc_runs
                ),
            ));
        }
    }
    Ok(())
}

/// Every combination of the values of the grids, the first grid the slowest.
fn combinations(grids: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut all: Vec<Vec<f64>> = vec![Vec::new()];
    for grid in grids {
        let mut next: Vec<Vec<f64>> = Vec::with_capacity(all.len() * grid.len());
        for prefix in &all {
            for value in grid {
                let mut row = prefix.clone();
                row.push(*value);
                next.push(row);
            }
        }
        all = next;
    }
    all
}

fn has_tolerance(netlist: &Netlist, defaults: &BTreeMap<char, Tolerance>) -> bool {
    netlist.elements.iter().any(|e| {
        let letter = e
            .name
            .chars()
            .next()
            .map_or(' ', |c| c.to_ascii_lowercase());
        let own = netlist
            .tolerances
            .contains_key(&e.name.to_ascii_lowercase());
        (own || defaults.contains_key(&letter)) && matches!(letter, 'r' | 'c' | 'l')
    })
}

/// The value of one measure from the results of the analyses.
fn measure_value(
    m: &MeasureSpec,
    circuit: &Circuit,
    s: &SimSettings,
    results: &Results,
) -> Result<Option<f64>, SimError> {
    let missing = || {
        failed(format!(
            "the measure {}: there is no {} in this circuit",
            m.name, m.signal
        ))
    };
    match m.domain {
        Domain::Tran => {
            let r = results
                .tran
                .as_ref()
                .ok_or_else(|| failed(format!("the measure {}: no transient result", m.name)))?;
            let y = r.series(&m.signal).ok_or_else(missing)?;
            Ok(waveform(
                &m.kind,
                &r.times,
                &y,
                s.measure_low,
                s.measure_high,
            ))
        }
        Domain::Dc => {
            let r = results
                .dc
                .as_ref()
                .ok_or_else(|| failed(format!("the measure {}: no DC result", m.name)))?;
            let y = r.series(&m.signal).ok_or_else(missing)?;
            Ok(waveform(
                &m.kind,
                &r.sweep,
                &y,
                s.measure_low,
                s.measure_high,
            ))
        }
        Domain::Ac => {
            if let Some(freq) = ac_point_freq(&m.kind) {
                let r = ac_at(circuit, s, &[freq])?;
                let column = r.series(&m.signal).ok_or_else(missing)?;
                return Ok(column.first().and_then(|v| ac_point(&m.kind, *v)));
            }
            let r = results
                .ac
                .as_ref()
                .ok_or_else(|| failed(format!("the measure {}: no AC result", m.name)))?;
            let values = r.series(&m.signal).ok_or_else(missing)?;
            Ok(ac_sweep(&m.kind, &r.freqs, &values))
        }
    }
}

/// The results of the analyses a plan needs.
struct Results {
    tran: Option<crate::tran::TranResult>,
    ac: Option<crate::ac::AcResult>,
    dc: Option<crate::dc::DcResult>,
}

/// Runs the analyses on one netlist and takes every measure.
fn evaluate(
    netlist: &Netlist,
    plan: &StudyPlan,
    s: &SimSettings,
) -> Result<Vec<Option<f64>>, SimError> {
    let circuit = Circuit::compile(netlist, s)?;
    let needs = |domain: Domain| plan.measures.iter().any(|m| m.domain == domain);
    let tran = match (&plan.tran, needs(Domain::Tran)) {
        (Some(request), true) => Some(transient(&circuit, s, request)?),
        _ => None,
    };
    let ac = match (&plan.ac, needs(Domain::Ac)) {
        (Some(request), true) => Some(ac_analysis(&circuit, s, request)?),
        _ => None,
    };
    let dc = match (&plan.dc, needs(Domain::Dc)) {
        (Some(request), true) => Some(dc_sweep(&circuit, s, request)?),
        _ => None,
    };
    let results = Results { tran, ac, dc };
    plan.measures
        .iter()
        .map(|m| measure_value(m, &circuit, s, &results))
        .collect()
}

/// The netlist of one point of the grid: the parameters replaced, then the element values set.
fn netlist_at(
    plan: &StudyPlan,
    build: &NetlistBuilder,
    point: &[f64],
) -> Result<Netlist, SimError> {
    let mut overrides: BTreeMap<String, f64> = BTreeMap::new();
    for (step, value) in plan.steps.iter().zip(point.iter()) {
        if let Target::Param(name) = &step.target {
            overrides.insert(name.clone(), *value);
        }
    }
    let mut netlist = build(&overrides)?;
    for step in &plan.steps {
        if let Target::Param(name) = &step.target {
            if !netlist.params.contains_key(name) {
                return Err(SimError::new(
                    UNDEFINED,
                    format!(".step: the netlist has no .param {name}"),
                ));
            }
        }
    }
    for (step, value) in plan.steps.iter().zip(point.iter()) {
        if let Target::Element(name) = &step.target {
            set_element(&mut netlist, name, *value)?;
        }
    }
    Ok(netlist)
}

fn sensitivities(
    plan: &StudyPlan,
    build: &NetlistBuilder,
    s: &SimSettings,
) -> Result<Vec<Sensitivity>, SimError> {
    let mut out: Vec<Sensitivity> = Vec::new();
    if plan.sensitivity.is_empty() {
        return Ok(out);
    }
    let base = build(&BTreeMap::new())?;
    let nominal = evaluate(&base, plan, s)?;
    for target in &plan.sensitivity {
        let value = match target {
            Target::Param(name) => base.params.get(name).copied(),
            Target::Element(name) => element_value(&base, name),
        };
        let Some(value) = value else {
            return Err(SimError::new(
                UNDEFINED,
                format!("sensitivity: {} is not in the netlist", target.label()),
            ));
        };
        if value == 0.0 || !value.is_finite() {
            return Err(SimError::new(
                STEP_INVALID,
                format!(
                    "sensitivity to {}: its value is zero, so a relative change is not defined",
                    target.label()
                ),
            ));
        }
        let delta = value.abs() * s.sens_rel_step;
        let run_at = |at: f64| -> Result<Vec<Option<f64>>, SimError> {
            match target {
                Target::Param(name) => {
                    let mut overrides: BTreeMap<String, f64> = BTreeMap::new();
                    overrides.insert(name.clone(), at);
                    evaluate(&build(&overrides)?, plan, s)
                }
                Target::Element(name) => {
                    let mut copy = base.clone();
                    set_element(&mut copy, name, at)?;
                    evaluate(&copy, plan, s)
                }
            }
        };
        let plus = run_at(value + delta)?;
        let minus = run_at(value - delta)?;
        for (k, m) in plan.measures.iter().enumerate() {
            let derivative = match (plus[k], minus[k]) {
                (Some(a), Some(b)) => Some((a - b) / (2.0 * delta)),
                _ => None,
            };
            let relative = match (derivative, nominal[k]) {
                (Some(d), Some(n)) if n != 0.0 => Some(d * value / n),
                _ => None,
            };
            out.push(Sensitivity {
                measure: m.name.clone(),
                target: target.label(),
                value,
                nominal: nominal[k],
                derivative,
                relative,
            });
        }
    }
    Ok(out)
}

/// Runs a study.
pub fn run_study(
    plan: &StudyPlan,
    build: &NetlistBuilder,
    s: &SimSettings,
) -> Result<StudyTable, SimError> {
    check_plan(plan, s)?;
    let mut grids: Vec<Vec<f64>> = Vec::new();
    for step in &plan.steps {
        grids.push(step.values(s.max_step_points)?);
    }
    let mut points: usize = 1;
    for grid in &grids {
        points = points
            .checked_mul(grid.len())
            .filter(|n| *n <= s.max_step_runs)
            .ok_or_else(|| {
                SimError::new(
                    LIMIT_EXCEEDED,
                    format!(
                        "the .step lines make more than max_step_runs ({}) combinations",
                        s.max_step_runs
                    ),
                )
            })?;
    }
    let per_point = plan.mc.map_or(1, |mc| mc.runs);
    let cells = points
        .saturating_mul(per_point)
        .saturating_mul(plan.measures.len());
    if cells > s.max_study_cells {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the table would hold {cells} values, more than max_study_cells ({})",
                s.max_study_cells
            ),
        ));
    }

    let mut rows: Vec<StudyRow> = Vec::new();
    let mut summaries: Vec<GroupSummary> = Vec::new();
    for point in combinations(&grids) {
        let base = netlist_at(plan, build, &point)?;
        let Some(mc) = plan.mc else {
            let values = evaluate(&base, plan, s)?;
            rows.push(StudyRow {
                step: point,
                run: None,
                values,
            });
            continue;
        };
        if !has_tolerance(&base, &plan.default_tolerances) {
            return Err(SimError::new(
                MC_INVALID,
                "no resistor, capacitor or inductor has a tolerance (write tol=5% on the line of an element, set tol= in the table of models or give --tol)",
            ));
        }
        let mut columns: Vec<Vec<f64>> = vec![Vec::new(); plan.measures.len()];
        for run in 0..mc.runs {
            let mut copy = base.clone();
            let mut rng = Rng::stream(mc.seed, run as u64);
            apply_tolerances(
                &mut copy,
                &plan.default_tolerances,
                &mut rng,
                s.mc_normal,
                s.mc_sigmas,
            );
            let values = evaluate(&copy, plan, s)?;
            for (column, value) in columns.iter_mut().zip(values.iter()) {
                if let Some(v) = value {
                    column.push(*v);
                }
            }
            rows.push(StudyRow {
                step: point.clone(),
                run: Some(run),
                values,
            });
        }
        summaries.push(GroupSummary {
            step: point,
            per_measure: columns
                .iter()
                .map(|column| summarize(column, &s.mc_percentiles))
                .collect(),
        });
    }
    Ok(StudyTable {
        axes: plan.steps.iter().map(|st| st.target.label()).collect(),
        measures: plan.measures.iter().map(|m| m.name.clone()).collect(),
        mc: plan.mc,
        percentiles: s.mc_percentiles.clone(),
        rows,
        summaries,
        sensitivities: sensitivities(plan, build, s)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::parse_measure;
    use crate::montecarlo::parse_default_tolerances;
    use crate::parse::{parse, parse_with_params};
    use crate::step::parse_step;

    fn builder<'a>(text: &'a str, s: &'a SimSettings) -> Box<NetlistBuilder<'a>> {
        Box::new(move |overrides: &BTreeMap<String, f64>| parse_with_params(text, s, overrides))
    }

    const RC: &str = "rc\nV1 in 0 PULSE(0 1 0 1n 1n 1 0)\n.param rr=1k\nR1 in out {rr}\nC1 out 0 1u\n.tran 20u 10m\n.measure tran rise10 rise v(out)\n.measure tran final at v(out) 9.9m\n";

    fn plan_of(text: &str, s: &SimSettings) -> StudyPlan {
        StudyPlan::from_netlist(&parse(text, s).unwrap())
    }

    #[test]
    fn a_plan_comes_from_the_directives_of_the_netlist() {
        let s = SimSettings::default();
        let plan = plan_of(RC, &s);
        assert_eq!(plan.measures.len(), 2);
        assert!(plan.tran.is_some());
        assert!(plan.ac.is_none());
        assert!(plan.mc.is_none());
    }

    #[test]
    fn the_rise_time_of_an_rc_circuit_is_2_2_time_constants() {
        let s = SimSettings::default();
        let plan = plan_of(RC, &s);
        let build = builder(RC, &s);
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(table.rows.len(), 1);
        let rise = table.rows[0].values[0].unwrap();
        // ln(9) RC with RC = 1 ms
        let want = 9f64.ln() * 1e-3;
        assert!((rise / want - 1.0).abs() < 2e-3, "{rise} {want}");
        let end = table.rows[0].values[1].unwrap();
        assert!((end - (1.0 - (-9.9f64).exp())).abs() < 1e-3, "{end}");
    }

    #[test]
    fn a_step_on_a_parameter_runs_the_netlist_again_for_each_value() {
        let s = SimSettings::default();
        let mut plan = plan_of(RC, &s);
        plan.steps
            .push(parse_step("param rr list 1k 2k 4k", None).unwrap());
        plan.measures = vec![parse_measure("tran rise10 rise v(out)", None).unwrap()];
        plan.tran = Some(TranRequest {
            tstep: 50e-6,
            tstop: 40e-3,
            method: crate::real::Method::Trapezoidal,
            adaptive: false,
            uic: false,
        });
        let build = builder(RC, &s);
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(table.axes, vec!["param:rr".to_string()]);
        assert_eq!(table.rows.len(), 3);
        let first = table.rows[0].values[0].unwrap();
        for (row, factor) in table.rows.iter().zip([1.0, 2.0, 4.0]) {
            let rise = row.values[0].unwrap();
            assert!((rise / (factor * first) - 1.0).abs() < 2e-3, "{rise}");
            assert_eq!(row.step.len(), 1);
        }
    }

    #[test]
    fn a_step_on_an_element_and_a_grid_of_two_steps() {
        let s = SimSettings::default();
        let text = "div\nV1 a 0 DC 10\nR1 a b 1k\nR2 b 0 1k\n.op\n.dc V1 0 10 10\n.measure dc vb at v(b) 10\n";
        let mut plan = plan_of(text, &s);
        plan.steps.push(parse_step("R2 list 1k 3k", None).unwrap());
        plan.steps.push(parse_step("R1 list 1k 9k", None).unwrap());
        let build = builder(text, &s);
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(table.axes, vec!["r2".to_string(), "r1".to_string()]);
        // the first step is the outermost
        let wanted = [
            (1e3, 1e3, 5.0),
            (1e3, 9e3, 1.0),
            (3e3, 1e3, 7.5),
            (3e3, 9e3, 2.5),
        ];
        assert_eq!(table.rows.len(), wanted.len());
        for (row, (r2, r1, v)) in table.rows.iter().zip(wanted) {
            assert_eq!(row.step, vec![r2, r1]);
            assert!((row.values[0].unwrap() - v).abs() < 1e-9, "{row:?}");
        }
    }

    #[test]
    fn the_deviation_of_a_divider_with_tolerances_follows_the_propagation_of_uncertainty() {
        // Vout = 10 R2 / (R1 + R2) with R1 = R2 = 1k and a uniform tolerance of 1 % on both: the
        // partial derivatives are -/+ 2.5e-3 V/ohm and each resistance has a deviation of
        // 10 / sqrt(3) ohm, so the deviation of Vout is sqrt(2) 2.5e-3 10 / sqrt(3) = 20.41 mV
        let s = SimSettings::default();
        let text = "div\nV1 a 0 DC 10\nR1 a b 1k tol=1% dist=uniform\nR2 b 0 1k tol=1% dist=uniform\n.op\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";
        let mut plan = plan_of(text, &s);
        plan.mc = Some(McSpec {
            runs: 3000,
            seed: 7,
        });
        let build = builder(text, &s);
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(table.rows.len(), 3000);
        assert_eq!(table.summaries.len(), 1);
        let summary = table.summaries[0].per_measure[0].as_ref().unwrap();
        assert_eq!(summary.count, 3000);
        let want_std = 2f64.sqrt() * 2.5e-3 * 10.0 / 3f64.sqrt();
        assert!(
            (summary.std / want_std - 1.0).abs() < 0.08,
            "{}",
            summary.std
        );
        assert!((summary.mean - 5.0).abs() < 0.003, "{}", summary.mean);
        assert!(summary.min >= 5.0 - 3.0 * want_std * 3.0f64.sqrt());
        // the same seed gives the same table, another seed does not
        let again = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(again, table);
        plan.mc = Some(McSpec {
            runs: 3000,
            seed: 8,
        });
        let other = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_ne!(other.rows, table.rows);
    }

    #[test]
    fn a_default_tolerance_applies_to_elements_without_one() {
        let s = SimSettings::default();
        let text = "div\nV1 a 0 DC 10\nR1 a b 1k\nR2 b 0 1k\n.op\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";
        let mut plan = plan_of(text, &s);
        plan.mc = Some(McSpec { runs: 20, seed: 1 });
        let build = builder(text, &s);
        // no tolerance anywhere: refused
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MC_INVALID
        );
        plan.default_tolerances = parse_default_tolerances("r=5%").unwrap();
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        let summary = table.summaries[0].per_measure[0].as_ref().unwrap();
        assert!(summary.std > 0.0);
    }

    #[test]
    fn the_sensitivity_of_a_divider_to_its_resistors() {
        // Vout = 10 R2 / (R1 + R2): dVout/dR2 = 10 R1 / (R1 + R2)^2 = 2.5e-3 V/ohm, and the
        // relative sensitivity R2/Vout dVout/dR2 is 1/2
        let s = SimSettings::default();
        let text = "div\nV1 a 0 DC 10\nR1 a b 1k\nR2 b 0 1k\n.op\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";
        let mut plan = plan_of(text, &s);
        plan.sensitivity = vec![
            crate::step::parse_target("R2").unwrap(),
            crate::step::parse_target("R1").unwrap(),
        ];
        plan.dc = Some(DcRequest {
            source: "v1".to_string(),
            start: 10.0,
            stop: 10.0,
            step: 1.0,
        });
        let build = builder(text, &s);
        let table = run_study(&plan, build.as_ref(), &s).unwrap();
        assert_eq!(table.sensitivities.len(), 2);
        let r2 = &table.sensitivities[0];
        assert_eq!(r2.target, "r2");
        assert!((r2.derivative.unwrap() - 2.5e-3).abs() < 1e-8, "{r2:?}");
        assert!((r2.relative.unwrap() - 0.5).abs() < 1e-5, "{r2:?}");
        let r1 = &table.sensitivities[1];
        assert!((r1.derivative.unwrap() + 2.5e-3).abs() < 1e-8, "{r1:?}");
    }

    #[test]
    fn a_plan_that_cannot_run_is_refused() {
        let s = SimSettings::default();
        let build = builder(RC, &s);
        let mut plan = plan_of(RC, &s);
        plan.tran = None;
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MEASURE_FAILED
        );
        let mut plan = plan_of(RC, &s);
        plan.measures.clear();
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MEASURE_INVALID
        );
        let mut plan = plan_of(RC, &s);
        plan.measures.push(plan.measures[0].clone());
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MEASURE_INVALID
        );
        let mut plan = plan_of(RC, &s);
        plan.steps
            .push(parse_step("param nothere list 1 2", None).unwrap());
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            UNDEFINED
        );
        let mut plan = plan_of(RC, &s);
        plan.steps
            .push(parse_step("R1 lin 1 2000 1", None).unwrap());
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let mut plan = plan_of(RC, &s);
        plan.mc = Some(McSpec { runs: 0, seed: 1 });
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MC_INVALID
        );
        let mut plan = plan_of(RC, &s);
        plan.measures = vec![parse_measure("tran m at v(zz) 1m", None).unwrap()];
        assert_eq!(
            run_study(&plan, build.as_ref(), &s).unwrap_err().code,
            MEASURE_FAILED
        );
    }
}
