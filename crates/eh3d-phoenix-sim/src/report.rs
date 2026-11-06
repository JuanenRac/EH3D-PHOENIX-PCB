// SPDX-License-Identifier: GPL-3.0-or-later
//! The tables of the studies, the noise analysis, the poles and zeros and the figures of merit,
//! as CSV and as JSON. Like the rest of the output they are deterministic: no date, no host and
//! no program version, and the numbers are written in the shortest form that reads back as the
//! same number. A value that does not exist (a level that is never crossed) is an empty cell in
//! the CSV and `null` in the JSON.

use serde_json::{json, Map, Value};

use crate::measure::FIGURE_NAMES;
use crate::noise::NoiseResult;
use crate::output::KIND;
use crate::poles::{frequency_hz, quality, PoleZero};
use crate::study::StudyTable;
use crate::units::format_number;

/// The format of the table of a study.
pub const MEASURE_FORMAT: &str = "phoenix-sim-measure/1";
/// The format of the noise table.
pub const NOISE_FORMAT: &str = "phoenix-sim-noise/1";
/// The format of the table of poles and zeros.
pub const POLES_FORMAT: &str = "phoenix-sim-poles/1";
/// The format of the table of figures of merit.
pub const FIGURES_FORMAT: &str = "phoenix-sim-figures/1";

fn cell(value: Option<f64>) -> String {
    value.map_or_else(String::new, format_number)
}

fn lines(header: Vec<String>, rows: Vec<Vec<String>>) -> String {
    let mut out = header.join(",");
    out.push('\n');
    for row in rows {
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// The name of a percentile in a table: `p5`, `p50`, `p97.5`.
fn percentile_name(percent: f64) -> String {
    format!("p{percent}")
}

/// The table of a study as CSV: one column for each step, the column `run` (`nominal` without
/// Monte Carlo, the number of the run with it) and one for each measure. With Monte Carlo the
/// statistics of each point of the grid follow its runs (the rows `count`, `mean`, `std`, `min`,
/// `max` and one for each percentile), and each sensitivity is a row `d(target)` of derivatives
/// and a row `rel(target)` of relative sensitivities.
pub fn study_csv(table: &StudyTable) -> String {
    let mut header: Vec<String> = table.axes.clone();
    header.push("run".to_string());
    header.extend(table.measures.iter().cloned());
    let mut rows: Vec<Vec<String>> = Vec::new();
    let steps =
        |values: &[f64]| -> Vec<String> { values.iter().map(|v| format_number(*v)).collect() };
    for row in &table.rows {
        let mut line = steps(&row.step);
        line.push(match row.run {
            Some(run) => run.to_string(),
            None => "nominal".to_string(),
        });
        line.extend(row.values.iter().map(|v| cell(*v)));
        rows.push(line);
    }
    for group in &table.summaries {
        let mut stats: Vec<(String, Vec<Option<f64>>)> = Vec::new();
        let per = |f: &dyn Fn(&crate::montecarlo::Summary) -> f64| -> Vec<Option<f64>> {
            group
                .per_measure
                .iter()
                .map(|s| s.as_ref().map(f))
                .collect()
        };
        stats.push(("count".to_string(), per(&|s| s.count as f64)));
        stats.push(("mean".to_string(), per(&|s| s.mean)));
        stats.push(("std".to_string(), per(&|s| s.std)));
        stats.push(("min".to_string(), per(&|s| s.min)));
        stats.push(("max".to_string(), per(&|s| s.max)));
        for (k, percent) in table.percentiles.iter().enumerate() {
            stats.push((
                percentile_name(*percent),
                per(&|s| s.percentiles.get(k).map_or(f64::NAN, |p| p.1)),
            ));
        }
        for (name, values) in stats {
            let mut line = steps(&group.step);
            line.push(name);
            line.extend(values.iter().map(|v| cell(v.filter(|x| x.is_finite()))));
            rows.push(line);
        }
    }
    let mut targets: Vec<&str> = Vec::new();
    for s in &table.sensitivities {
        if !targets.contains(&s.target.as_str()) {
            targets.push(s.target.as_str());
        }
    }
    for target in targets {
        for (prefix, relative) in [("d", false), ("rel", true)] {
            let mut line: Vec<String> = table.axes.iter().map(|_| String::new()).collect();
            line.push(format!("{prefix}({target})"));
            for measure in &table.measures {
                let found = table
                    .sensitivities
                    .iter()
                    .find(|s| s.target == target && s.measure == *measure);
                let value = found.and_then(|s| if relative { s.relative } else { s.derivative });
                line.push(cell(value));
            }
            rows.push(line);
        }
    }
    lines(header, rows)
}

/// The table of a study as JSON (`phoenix-sim-measure/1`).
pub fn study_json(table: &StudyTable) -> String {
    let rows: Vec<Value> = table
        .rows
        .iter()
        .map(|r| {
            json!({
                "step": r.step,
                "run": r.run,
                "values": r.values,
            })
        })
        .collect();
    let mut summary: Vec<Value> = Vec::new();
    for group in &table.summaries {
        for (k, measure) in table.measures.iter().enumerate() {
            let Some(Some(s)) = group.per_measure.get(k) else {
                summary.push(json!({
                    "step": group.step,
                    "measure": measure,
                    "count": 0,
                }));
                continue;
            };
            let mut percentiles = Map::new();
            for (percent, value) in &s.percentiles {
                percentiles.insert(percentile_name(*percent), json!(value));
            }
            summary.push(json!({
                "step": group.step,
                "measure": measure,
                "count": s.count,
                "mean": s.mean,
                "std": s.std,
                "min": s.min,
                "max": s.max,
                "percentiles": percentiles,
            }));
        }
    }
    let sensitivity: Vec<Value> = table
        .sensitivities
        .iter()
        .map(|s| {
            json!({
                "measure": s.measure,
                "target": s.target,
                "value": s.value,
                "nominal": s.nominal,
                "derivative": s.derivative,
                "relative": s.relative,
            })
        })
        .collect();
    let mc = table
        .mc
        .map_or(Value::Null, |m| json!({ "runs": m.runs, "seed": m.seed }));
    pretty(&json!({
        "format": MEASURE_FORMAT,
        "kind": KIND,
        "axes": table.axes,
        "measures": table.measures,
        "monte_carlo": mc,
        "rows": rows,
        "summary": summary,
        "sensitivity": sensitivity,
    }))
}

/// The noise at the output as CSV: the frequency, the total density in V/sqrt(Hz) and in V^2/Hz,
/// and the contribution of each source in V^2/Hz.
pub fn noise_csv(r: &NoiseResult) -> String {
    let mut header = vec![
        "freq_hz".to_string(),
        format!("density({})", r.output),
        format!("power({})", r.output),
    ];
    header.extend(
        r.contributions
            .iter()
            .map(|c| format!("power_from({})", c.source)),
    );
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (k, f) in r.freqs.iter().enumerate() {
        let mut line = vec![
            format_number(*f),
            format_number(r.total[k].sqrt()),
            format_number(r.total[k]),
        ];
        line.extend(r.contributions.iter().map(|c| format_number(c.density[k])));
        rows.push(line);
    }
    lines(header, rows)
}

/// The noise at the output as JSON (`phoenix-sim-noise/1`), with the total in the band of the
/// sweep.
pub fn noise_json(r: &NoiseResult) -> String {
    let contributions: Vec<Value> = r
        .contributions
        .iter()
        .map(|c| json!({ "source": c.source, "power_v2_per_hz": c.density }))
        .collect();
    pretty(&json!({
        "format": NOISE_FORMAT,
        "kind": KIND,
        "output": r.output,
        "freq_hz": r.freqs,
        "density_v_per_sqrt_hz": r.voltage_density(),
        "power_v2_per_hz": r.total,
        "contributions": contributions,
        "band_hz": [r.freqs.first(), r.freqs.last()],
        "rms_volts": r.rms_volts,
    }))
}

/// The poles and zeros as CSV: the kind, the number, the real and imaginary parts in rad/s, the
/// magnitude as a frequency in Hz and the quality factor (empty for a real root).
pub fn poles_csv(r: &PoleZero) -> String {
    let header = ["kind", "index", "re_rad_s", "im_rad_s", "freq_hz", "q"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (kind, list) in [("pole", &r.poles), ("zero", &r.zeros)] {
        for (k, root) in list.iter().enumerate() {
            rows.push(vec![
                kind.to_string(),
                (k + 1).to_string(),
                format_number(root.re),
                format_number(root.im),
                format_number(frequency_hz(*root)),
                cell(quality(*root)),
            ]);
        }
    }
    lines(header, rows)
}

/// The poles and zeros as JSON (`phoenix-sim-poles/1`), with the order of the fit, the gain at
/// zero frequency and the largest relative error of the fit.
pub fn poles_json(r: &PoleZero) -> String {
    let describe = |list: &[eh3d_phoenix_rf::Complex]| -> Vec<Value> {
        list.iter()
            .map(|root| {
                json!({
                    "re_rad_s": root.re,
                    "im_rad_s": root.im,
                    "freq_hz": frequency_hz(*root),
                    "q": quality(*root),
                })
            })
            .collect()
    };
    pretty(&json!({
        "format": POLES_FORMAT,
        "kind": KIND,
        "method": "rational fit of the AC response (Sanathanan-Koerner), roots by Durand-Kerner",
        "order": r.order,
        "dc_gain": r.dc_gain,
        "max_relative_error": r.max_error,
        "poles": describe(&r.poles),
        "zeros": describe(&r.zeros),
    }))
}

/// The figures of merit of a response as CSV: the name, the value and the unit.
pub fn figures_csv(values: &[Option<f64>]) -> String {
    let header = ["name", "value", "unit"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let rows = FIGURE_NAMES
        .iter()
        .zip(values.iter())
        .map(|((name, unit), value)| vec![name.to_string(), cell(*value), unit.to_string()])
        .collect();
    lines(header, rows)
}

/// The figures of merit of a response as JSON (`phoenix-sim-figures/1`).
pub fn figures_json(signal: &str, values: &[Option<f64>]) -> String {
    let mut figures = Map::new();
    for ((name, unit), value) in FIGURE_NAMES.iter().zip(values.iter()) {
        figures.insert(name.to_string(), json!({ "value": value, "unit": unit }));
    }
    pretty(&json!({
        "format": FIGURES_FORMAT,
        "kind": KIND,
        "signal": signal,
        "figures": figures,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::montecarlo::summarize;
    use crate::study::{GroupSummary, McSpec, Sensitivity, StudyRow};
    use eh3d_phoenix_rf::Complex;

    fn table() -> StudyTable {
        StudyTable {
            axes: vec!["r1".to_string()],
            measures: vec!["a".to_string(), "b".to_string()],
            mc: Some(McSpec { runs: 2, seed: 9 }),
            percentiles: vec![50.0],
            rows: vec![
                StudyRow {
                    step: vec![1000.0],
                    run: Some(0),
                    values: vec![Some(1.0), None],
                },
                StudyRow {
                    step: vec![1000.0],
                    run: Some(1),
                    values: vec![Some(3.0), None],
                },
            ],
            summaries: vec![GroupSummary {
                step: vec![1000.0],
                per_measure: vec![summarize(&[1.0, 3.0], &[50.0]), None],
            }],
            sensitivities: vec![Sensitivity {
                measure: "a".to_string(),
                target: "r1".to_string(),
                value: 1000.0,
                nominal: Some(2.0),
                derivative: Some(0.5),
                relative: Some(250.0),
            }],
        }
    }

    #[test]
    fn the_csv_of_a_study_lists_runs_statistics_and_sensitivities() {
        let csv = study_csv(&table());
        let want = "r1,run,a,b\n\
1e3,0,1e0,\n\
1e3,1,3e0,\n\
1e3,count,2e0,\n\
1e3,mean,2e0,\n\
1e3,std,1.4142135623730951e0,\n\
1e3,min,1e0,\n\
1e3,max,3e0,\n\
1e3,p50,2e0,\n\
,d(r1),5e-1,\n\
,rel(r1),2.5e2,\n";
        assert_eq!(csv, want);
    }

    #[test]
    fn the_json_of_a_study_has_the_format_and_null_for_a_missing_value() {
        let text = study_json(&table());
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["format"], MEASURE_FORMAT);
        assert_eq!(v["monte_carlo"]["seed"], 9);
        assert_eq!(v["rows"][0]["values"][1], Value::Null);
        assert_eq!(v["rows"][1]["run"], 1);
        assert_eq!(v["summary"][0]["count"], 2);
        assert_eq!(v["summary"][0]["percentiles"]["p50"], 2.0);
        assert_eq!(v["summary"][1]["count"], 0);
        assert_eq!(v["sensitivity"][0]["derivative"], 0.5);
        assert_eq!(study_json(&table()), text);
    }

    #[test]
    fn a_study_without_monte_carlo_marks_its_rows_nominal() {
        let mut t = table();
        t.mc = None;
        t.summaries.clear();
        t.sensitivities.clear();
        t.rows.truncate(1);
        t.rows[0].run = None;
        assert_eq!(study_csv(&t), "r1,run,a,b\n1e3,nominal,1e0,\n");
        let v: Value = serde_json::from_str(&study_json(&t)).unwrap();
        assert_eq!(v["monte_carlo"], Value::Null);
        assert_eq!(v["rows"][0]["run"], Value::Null);
    }

    #[test]
    fn the_noise_table_has_the_density_the_power_and_each_source() {
        let r = NoiseResult {
            output: "v(out)".to_string(),
            freqs: vec![10.0, 100.0],
            contributions: vec![crate::noise::NoiseContribution {
                source: "R1".to_string(),
                density: vec![4.0, 16.0],
            }],
            total: vec![4.0, 16.0],
            rms_volts: 1.5,
        };
        assert_eq!(
            noise_csv(&r),
            "freq_hz,density(v(out)),power(v(out)),power_from(R1)\n1e1,2e0,4e0,4e0\n1e2,4e0,1.6e1,1.6e1\n"
        );
        let v: Value = serde_json::from_str(&noise_json(&r)).unwrap();
        assert_eq!(v["format"], NOISE_FORMAT);
        assert_eq!(v["rms_volts"], 1.5);
        assert_eq!(v["density_v_per_sqrt_hz"][1], 4.0);
        assert_eq!(v["band_hz"][1], 100.0);
    }

    #[test]
    fn the_poles_table_lists_poles_then_zeros() {
        let r = PoleZero {
            order: 2,
            poles: vec![Complex::new(-3.0, 4.0), Complex::new(-3.0, -4.0)],
            zeros: vec![Complex::new(-10.0, 0.0)],
            dc_gain: 1.0,
            max_error: 1e-9,
        };
        let csv = poles_csv(&r);
        let rows: Vec<&str> = csv.lines().collect();
        assert_eq!(rows[0], "kind,index,re_rad_s,im_rad_s,freq_hz,q");
        assert_eq!(rows.len(), 4);
        assert!(rows[1].starts_with("pole,1,-3e0,4e0,"));
        assert!(rows[3].starts_with("zero,1,-1e1,0e0,"));
        assert!(rows[3].ends_with(','));
        let v: Value = serde_json::from_str(&poles_json(&r)).unwrap();
        assert_eq!(v["format"], POLES_FORMAT);
        assert_eq!(v["order"], 2);
        // the magnitude of -3 + 4j is 5 and its quality factor is 5 / 6
        assert!((v["poles"][0]["q"].as_f64().unwrap() - 5.0 / 6.0).abs() < 1e-12);
        assert_eq!(v["zeros"][0]["q"], Value::Null);
    }

    #[test]
    fn the_figures_table_has_one_row_for_each_figure() {
        let values = vec![Some(1.0); FIGURE_NAMES.len()];
        let csv = figures_csv(&values);
        assert_eq!(csv.lines().count(), 1 + FIGURE_NAMES.len());
        assert!(csv.starts_with("name,value,unit\ndc_gain,1e0,\n"));
        let v: Value = serde_json::from_str(&figures_json("v(out)", &values)).unwrap();
        assert_eq!(v["format"], FIGURES_FORMAT);
        assert_eq!(v["figures"]["cutoff_3db"]["unit"], "Hz");
    }
}
