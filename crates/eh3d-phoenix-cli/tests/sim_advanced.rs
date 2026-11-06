// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced options of the command `sim`: steps, measures, Monte Carlo, sensitivity, noise,
//! poles and figures of merit, on netlists written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;

fn command(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

const RC_STEP: &str = "rc\nV1 in 0 PULSE(0 1 0 1n 1n 1 0)\nR1 in out 1k\nC1 out 0 1u\n";
const RC_AC: &str = "rc\nV1 in 0 AC 1\nR1 in out 1k\nC1 out 0 1u\n";
const DIVIDER_TOL: &str = "div\nV1 a 0 DC 10\nR1 a b 1k tol=1%\nR2 b 0 1k tol=1%\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";

/// The cell `column` of the row `row` (the header is row 0) of a CSV text.
fn cell(csv: &str, row: usize, column: usize) -> String {
    csv.lines()
        .nth(row)
        .and_then(|line| line.split(',').nth(column))
        .unwrap_or("")
        .to_owned()
}

fn number(csv: &str, row: usize, column: usize) -> f64 {
    cell(csv, row, column).parse().unwrap()
}

#[test]
fn a_measure_on_the_command_line_makes_a_table_of_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_STEP);
    let (code, out, err) = command(&[
        "sim",
        &file,
        "--tran",
        "10u",
        "10m",
        "--measure",
        "tran rise10 rise v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().next().unwrap(), "run,rise10");
    assert_eq!(cell(&out, 1, 0), "nominal");
    let rise = number(&out, 1, 1);
    let want = 9f64.ln() * 1e-3;
    assert!((rise / want - 1.0).abs() < 2e-3, "{rise}");
    // the same command gives the same bytes
    let (_, again, _) = command(&[
        "sim",
        &file,
        "--tran",
        "10u",
        "10m",
        "--measure",
        "tran rise10 rise v(out)",
    ]);
    assert_eq!(again, out);
}

#[test]
fn a_step_runs_the_circuit_for_each_value_and_the_json_has_its_format() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_STEP);
    let args = [
        "sim",
        file.as_str(),
        "--tran",
        "20u",
        "40m",
        "--step",
        "R1 list 1k 2k",
        "--measure",
        "tran rise10 rise v(out)",
    ];
    let (code, out, err) = command(&args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().next().unwrap(), "r1,run,rise10");
    assert_eq!(out.lines().count(), 3);
    let ratio = number(&out, 2, 2) / number(&out, 1, 2);
    assert!((ratio - 2.0).abs() < 4e-3, "{ratio}");
    let mut with_json = args.to_vec();
    with_json.push("--json");
    let (code, json, _) = command(&with_json);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["format"], "phoenix-sim-measure/1");
    assert_eq!(json["axes"][0], "r1");
    assert_eq!(json["rows"].as_array().unwrap().len(), 2);
}

#[test]
fn the_measure_and_step_lines_of_a_netlist_are_run_when_no_analysis_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!(
        "{RC_STEP}.tran 10u 10m\n.step R1 list 1k 2k\n.measure tran rise10 rise v(out)\n.measure tran last at v(out) 9.9m\n"
    );
    let file = write(dir.path(), "rc.cir", &text);
    let (code, out, err) = command(&["sim", &file]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().next().unwrap(), "r1,run,rise10,last");
    assert_eq!(out.lines().count(), 3);
    // out writes both files
    let base = dir.path().join("table");
    let (code, written, _) = command(&["sim", &file, "--out", base.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(written.contains("table.csv"), "{written}");
    assert!(dir.path().join("table.json").exists());
}

#[test]
fn a_parameter_step_reads_the_file_again() {
    let dir = tempfile::tempdir().unwrap();
    let text = "rc\nV1 in 0 PULSE(0 1 0 1n 1n 1 0)\n.param rr=1k\nR1 in out {rr}\nC1 out 0 1u\n.tran 50u 60m\n";
    let file = write(dir.path(), "rc.cir", text);
    let (code, out, err) = command(&[
        "sim",
        &file,
        "--step",
        "param rr list 1k 3k",
        "--measure",
        "tran rise10 rise v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().next().unwrap(), "param:rr,run,rise10");
    let ratio = number(&out, 2, 2) / number(&out, 1, 2);
    assert!((ratio - 3.0).abs() < 6e-3, "{ratio}");
}

#[test]
fn monte_carlo_repeats_with_the_seed_and_ends_with_the_statistics() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "div.cir", DIVIDER_TOL);
    let (code, out, err) = command(&["sim", &file, "--mc", "200", "--seed", "5"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().next().unwrap(), "run,vb");
    // 200 runs, the header, count, mean, std, min, max and three percentiles
    assert_eq!(out.lines().count(), 1 + 200 + 8);
    assert_eq!(cell(&out, 202, 0), "mean");
    let mean = number(&out, 202, 1);
    assert!((mean - 5.0).abs() < 0.01, "{mean}");
    let (_, same, _) = command(&["sim", &file, "--mc", "200", "--seed", "5"]);
    assert_eq!(same, out);
    let (_, other, _) = command(&["sim", &file, "--mc", "200", "--seed", "6"]);
    assert_ne!(other, out);
    let (code, json, _) = command(&["sim", &file, "--mc", "50", "--json"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["monte_carlo"]["runs"], 50);
    assert_eq!(json["monte_carlo"]["seed"], 1);
    assert_eq!(json["summary"][0]["count"], 50);
}

#[test]
fn a_default_tolerance_and_the_lack_of_one_are_handled() {
    let dir = tempfile::tempdir().unwrap();
    let plain =
        "div\nV1 a 0 DC 10\nR1 a b 1k\nR2 b 0 1k\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";
    let file = write(dir.path(), "div.cir", plain);
    let (code, _, err) = command(&["sim", &file, "--mc", "10"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-028"), "{err}");
    let (code, out, err) = command(&["sim", &file, "--mc", "10", "--tol", "R=5%"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().count(), 1 + 10 + 8);
    let (code, _, err) = command(&["sim", &file, "--seed", "4"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-016"), "{err}");
}

#[test]
fn the_sensitivity_of_a_divider_is_written_below_the_table() {
    let dir = tempfile::tempdir().unwrap();
    let plain =
        "div\nV1 a 0 DC 10\nR1 a b 1k\nR2 b 0 1k\n.dc V1 10 10 1\n.measure dc vb at v(b) 10\n";
    let file = write(dir.path(), "div.cir", plain);
    let (code, out, err) = command(&["sim", &file, "--sens", "R2"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(cell(&out, 2, 0), "d(r2)");
    assert!((number(&out, 2, 1) - 2.5e-3).abs() < 1e-8, "{out}");
    assert_eq!(cell(&out, 3, 0), "rel(r2)");
    assert!((number(&out, 3, 1) - 0.5).abs() < 1e-5, "{out}");
}

#[test]
fn the_noise_of_a_resistor_is_four_k_t_r() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "r.cir", "noise\nR1 out 0 1k\n");
    let (code, out, err) = command(&[
        "sim", &file, "--ac", "dec", "2", "10", "1meg", "--noise", "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out.lines().next().unwrap(),
        "freq_hz,density(v(out)),power(v(out)),power_from(R1)"
    );
    let density = number(&out, 1, 1);
    let want = (4.0 * 1.380_649e-23 * 300.15 * 1000.0f64).sqrt();
    assert!((density / want - 1.0).abs() < 1e-9, "{density} {want}");
    let (code, json, _) = command(&[
        "sim", &file, "--ac", "dec", "2", "10", "1meg", "--noise", "v(out)", "--json",
    ]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["format"], "phoenix-sim-noise/1");
    assert!(json["rms_volts"].as_f64().unwrap() > 0.0);
}

#[test]
fn the_pole_and_the_figures_of_an_rc_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_AC);
    let (code, out, err) = command(&[
        "sim", &file, "--ac", "dec", "20", "10", "100k", "--poles", "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(cell(&out, 1, 0), "pole");
    // the pole of a 1 kohm and 1 uF circuit is at -1000 rad/s
    assert!((number(&out, 1, 2) + 1000.0).abs() < 0.05, "{out}");
    assert_eq!(out.lines().count(), 2);
    let (code, out, err) = command(&[
        "sim",
        &file,
        "--ac",
        "dec",
        "100",
        "1",
        "100k",
        "--figures",
        "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    let fc = out
        .lines()
        .find(|l| l.starts_with("cutoff_3db,"))
        .map(|l| l.split(',').nth(1).unwrap().parse::<f64>().unwrap())
        .unwrap();
    let want = 1.0 / (2.0 * std::f64::consts::PI * 1e-3);
    assert!((fc / want - 1.0).abs() < 2e-3, "{fc}");
}

#[test]
fn unusable_requests_give_exit_code_2_with_a_code_and_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_STEP);
    // a step with nothing to measure
    let (code, _, err) = command(&["sim", &file, "--tran", "10u", "1m", "--step", "R1 list 1k"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-026"), "{err}");
    // a measure of an analysis nobody asked for
    let (code, _, err) = command(&["sim", &file, "--measure", "tran m max v(out)"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-027"), "{err}");
    // a step that is not valid
    let (code, _, err) = command(&[
        "sim",
        &file,
        "--step",
        "R1 lin 1 2 0",
        "--measure",
        "tran m max v(out)",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-025"), "{err}");
    // the poles need an AC sweep
    let (code, _, err) = command(&["sim", &file, "--poles", "v(out)"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-016"), "{err}");
    // a measure line of the netlist that is not valid names its line
    let bad = format!("{RC_STEP}.tran 10u 1m\n.measure tran m nothing v(out)\n");
    let file = write(dir.path(), "bad.cir", &bad);
    let (code, _, err) = command(&["sim", &file]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-026"), "{err}");
    assert!(err.contains("line 6"), "{err}");
}

#[test]
fn the_messages_of_the_new_codes_come_in_the_language_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_STEP);
    let (code, _, err) = command(&[
        "sim",
        &file,
        "--lang",
        "pt",
        "--step",
        "R1 lin 1 2 0",
        "--measure",
        "tran m max v(out)",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("O varrimento de parâmetro"), "{err}");
}
