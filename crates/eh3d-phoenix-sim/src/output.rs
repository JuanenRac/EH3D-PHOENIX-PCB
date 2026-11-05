// SPDX-License-Identifier: GPL-3.0-or-later
//! The results as CSV tables and as JSON. Both are deterministic: the same circuit and the same
//! request give the same bytes, with no date, host name or program version. Numbers are written
//! in the shortest form that reads back as the same number (`1.5e-3`).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::ac::AcResult;
use crate::codes::{SimError, OUTPUT_FAILED, UNDEFINED};
use crate::dc::OpResult;
use crate::settings::SimSettings;
use crate::tran::TranResult;
use crate::units::format_number;

/// What kind of result this is, as every number of the suite says (a prototype of our own, not
/// compared with SPICE).
pub const KIND: &str = "own numerical approximation (prototype, not compared with SPICE)";

/// The indices of the unknowns to print: all of them when `wanted` is empty, otherwise the ones
/// named (`v(out)`, `i(v1)`), in the order given.
pub fn select(labels: &[String], wanted: &[String]) -> Result<Vec<usize>, SimError> {
    if wanted.is_empty() {
        return Ok((0..labels.len()).collect());
    }
    let mut out = Vec::new();
    for name in wanted {
        let lower = name.trim().to_lowercase();
        match labels.iter().position(|l| *l == lower) {
            Some(i) => out.push(i),
            None => {
                return Err(SimError::new(
                    UNDEFINED,
                    format!(
                        "there is no {name} in this circuit; the unknowns are {}",
                        labels.join(", ")
                    ),
                ))
            }
        }
    }
    Ok(out)
}

fn rows_to_csv(header: Vec<String>, rows: Vec<Vec<String>>) -> String {
    let mut out = header.join(",");
    out.push('\n');
    for row in rows {
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// The operating point as a table of `name,value`.
pub fn op_csv(r: &OpResult, selected: &[usize]) -> String {
    let rows = selected
        .iter()
        .map(|&i| vec![r.labels[i].clone(), format_number(r.values[i])])
        .collect();
    rows_to_csv(vec!["name".to_string(), "value".to_string()], rows)
}

/// The AC result as a table: the frequency, then magnitude, dB, phase in degrees and group delay
/// in seconds of each selected unknown.
pub fn ac_csv(r: &AcResult, selected: &[usize]) -> String {
    let mut header = vec!["freq_hz".to_string()];
    for &i in selected {
        let l = &r.labels[i];
        header.push(format!("mag({l})"));
        header.push(format!("db({l})"));
        header.push(format!("phase_deg({l})"));
        header.push(format!("delay_s({l})"));
    }
    let mut rows = Vec::new();
    for (k, f) in r.freqs.iter().enumerate() {
        let mut row = vec![format_number(*f)];
        for &i in selected {
            let v = r.values[k][i];
            row.push(format_number(v.abs()));
            row.push(format_number(v.db20()));
            row.push(format_number(v.arg_deg()));
            row.push(format_number(r.delay_s[k][i]));
        }
        rows.push(row);
    }
    rows_to_csv(header, rows)
}

/// The transient result as a table: the time, then each selected unknown.
pub fn tran_csv(r: &TranResult, selected: &[usize]) -> String {
    let mut header = vec!["time_s".to_string()];
    header.extend(selected.iter().map(|&i| r.labels[i].clone()));
    let rows = r
        .times
        .iter()
        .zip(r.values.iter())
        .map(|(t, values)| {
            let mut row = vec![format_number(*t)];
            row.extend(selected.iter().map(|&i| format_number(values[i])));
            row
        })
        .collect();
    rows_to_csv(header, rows)
}

fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

fn labels_of(labels: &[String], selected: &[usize]) -> Vec<String> {
    selected.iter().map(|&i| labels[i].clone()).collect()
}

/// The operating point as JSON.
pub fn op_json(r: &OpResult, selected: &[usize]) -> String {
    let values: Vec<f64> = selected.iter().map(|&i| r.values[i]).collect();
    pretty(&json!({
        "analysis": "op",
        "kind": KIND,
        "iterations": r.iterations,
        "strategy": r.strategy,
        "labels": labels_of(&r.labels, selected),
        "values": values,
    }))
}

/// The AC result as JSON: one array per quantity, indexed by frequency and then by label.
pub fn ac_json(r: &AcResult, selected: &[usize]) -> String {
    let per = |f: &dyn Fn(usize, usize) -> f64| -> Vec<Vec<f64>> {
        (0..r.freqs.len())
            .map(|k| selected.iter().map(|&i| f(k, i)).collect())
            .collect()
    };
    let magnitude = per(&|k, i| r.values[k][i].abs());
    let db = per(&|k, i| r.values[k][i].db20());
    let phase = per(&|k, i| r.values[k][i].arg_deg());
    let delay = per(&|k, i| r.delay_s[k][i]);
    pretty(&json!({
        "analysis": "ac",
        "kind": KIND,
        "labels": labels_of(&r.labels, selected),
        "freq_hz": r.freqs,
        "magnitude": magnitude,
        "db": db,
        "phase_deg": phase,
        "delay_s": delay,
    }))
}

/// The transient result as JSON.
pub fn tran_json(r: &TranResult, selected: &[usize]) -> String {
    let values: Vec<Vec<f64>> = r
        .values
        .iter()
        .map(|row| selected.iter().map(|&i| row[i]).collect())
        .collect();
    let notes: Vec<String> = r.notes.iter().map(|n| n.to_string()).collect();
    pretty(&json!({
        "analysis": "tran",
        "kind": KIND,
        "labels": labels_of(&r.labels, selected),
        "time_s": r.times,
        "values": values,
        "accepted_steps": r.accepted,
        "rejected_steps": r.rejected,
        "notes": notes,
    }))
}

/// Writes a file as a temporary file and a rename, so that a reader never sees half a file; the
/// rename is tried again a bounded number of times when another program holds the file.
fn write_atomic(path: &Path, text: &str, settings: &SimSettings) -> Result<(), SimError> {
    let failed =
        |why: std::io::Error| SimError::new(OUTPUT_FAILED, format!("{}: {why}", path.display()));
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    let temporary = PathBuf::from(name);
    std::fs::write(&temporary, text).map_err(failed)?;
    let mut last = None;
    for attempt in 0..=settings.write_retries {
        match std::fs::rename(&temporary, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Some(e);
                if attempt < settings.write_retries {
                    std::thread::sleep(std::time::Duration::from_millis(settings.write_retry_ms));
                }
            }
        }
    }
    let _ = std::fs::remove_file(&temporary);
    match last {
        Some(e) => Err(failed(e)),
        None => Ok(()),
    }
}

/// Writes `BASE.csv` and `BASE.json` (the folder is made when it is missing) and returns the
/// two paths.
pub fn write_outputs(
    base: &Path,
    csv: &str,
    json_text: &str,
    settings: &SimSettings,
) -> Result<Vec<PathBuf>, SimError> {
    let with = |extension: &str| -> PathBuf {
        let mut name = base.as_os_str().to_owned();
        name.push(".");
        name.push(extension);
        PathBuf::from(name)
    };
    let (csv_path, json_path) = (with("csv"), with("json"));
    if let Some(parent) = csv_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SimError::new(OUTPUT_FAILED, format!("{}: {e}", parent.display())))?;
        }
    }
    write_atomic(&csv_path, csv, settings)?;
    write_atomic(&json_path, json_text, settings)?;
    Ok(vec![csv_path, json_path])
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rf::Complex;

    fn op() -> OpResult {
        OpResult {
            labels: vec!["v(a)".to_string(), "i(v1)".to_string()],
            values: vec![7.5, -2.5e-3],
            node_count: 1,
            iterations: 1,
            strategy: "direct",
        }
    }

    #[test]
    fn selection_is_all_or_by_name() {
        let r = op();
        assert_eq!(select(&r.labels, &[]).unwrap(), vec![0, 1]);
        assert_eq!(
            select(&r.labels, &["I(V1)".to_string(), "v(a)".to_string()]).unwrap(),
            vec![1, 0]
        );
        let e = select(&r.labels, &["v(zz)".to_string()]).unwrap_err();
        assert_eq!(e.code, UNDEFINED);
        assert!(e.detail.contains("v(a), i(v1)"));
    }

    #[test]
    fn the_operating_point_tables() {
        let r = op();
        assert_eq!(
            op_csv(&r, &[0, 1]),
            "name,value\nv(a),7.5e0\ni(v1),-2.5e-3\n"
        );
        let json = op_json(&r, &[0]);
        assert!(json.contains("\"analysis\": \"op\""));
        assert!(json.contains("7.5"));
        assert!(!json.contains("i(v1)"));
        assert!(json.ends_with("}\n"));
    }

    #[test]
    fn the_ac_table_has_four_columns_per_unknown() {
        let r = AcResult {
            labels: vec!["v(out)".to_string()],
            node_count: 1,
            freqs: vec![10.0, 100.0],
            values: vec![vec![Complex::new(1.0, 0.0)], vec![Complex::new(0.0, 0.5)]],
            delay_s: vec![vec![0.0], vec![1e-3]],
        };
        let csv = ac_csv(&r, &[0]);
        let mut lines = csv.lines();
        assert_eq!(
            lines.next().unwrap(),
            "freq_hz,mag(v(out)),db(v(out)),phase_deg(v(out)),delay_s(v(out))"
        );
        assert_eq!(lines.next().unwrap(), "1e1,1e0,0e0,0e0,0e0");
        let second = lines.next().unwrap();
        assert!(second.starts_with("1e2,5e-1,-6.02"), "{second}");
        let fields: Vec<&str> = second.split(',').collect();
        assert_eq!(fields.len(), 5);
        assert!((fields[3].parse::<f64>().unwrap() - 90.0).abs() < 1e-9);
        assert_eq!(fields[4], "1e-3");
        let json = ac_json(&r, &[0]);
        assert!(json.contains("\"freq_hz\""));
        assert!(json.contains("\"delay_s\""));
    }

    #[test]
    fn the_transient_table_and_the_files() {
        let r = TranResult {
            labels: vec!["v(out)".to_string()],
            node_count: 1,
            times: vec![0.0, 1e-3],
            values: vec![vec![0.0], vec![0.5]],
            accepted: 1,
            rejected: 0,
            iterations: 1,
            notes: Vec::new(),
        };
        let csv = tran_csv(&r, &[0]);
        assert_eq!(csv, "time_s,v(out)\n0e0,0e0\n1e-3,5e-1\n");
        let json = tran_json(&r, &[0]);
        assert!(json.contains("\"accepted_steps\": 1"));

        let dir = std::env::temp_dir().join(format!("phoenix-sim-out-{}", std::process::id()));
        let base = dir.join("nested").join("result");
        let paths = write_outputs(&base, &csv, &json, &SimSettings::default()).unwrap();
        assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), csv);
        assert_eq!(std::fs::read_to_string(&paths[1]).unwrap(), json);
        let _ = std::fs::remove_dir_all(&dir);
        // a path under a file cannot be made
        let file = std::env::temp_dir().join(format!("phoenix-sim-file-{}", std::process::id()));
        std::fs::write(&file, "x").unwrap();
        let e = write_outputs(
            &file.join("sub").join("r"),
            &csv,
            &json,
            &SimSettings::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        let _ = std::fs::remove_file(&file);
    }
}
