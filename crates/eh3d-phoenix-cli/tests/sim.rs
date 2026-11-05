// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `sim` on netlists and a schematic written by the test itself.

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

const DIVIDER: &str = "divider\nV1 in 0 DC 10\nR1 in out 1k\nR2 out 0 3k\n";
const RC: &str = "rc\nV1 in 0 PULSE(0 1 0 0 0 10 20)\nR1 in out 1k\nC1 out 0 1u\n";
const RC_AC: &str = "rc\nV1 in 0 AC 1\nR1 in out 1k\nC1 out 0 1u\n";

#[test]
fn the_operating_point_is_printed_as_csv_and_as_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.cir", DIVIDER);
    let (code, out, err) = command(&["sim", &file, "--op"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("name,value\n"), "{out}");
    assert!(out.contains("v(out),7.5e0\n"), "{out}");
    let (code, json, _) = command(&["sim", &file, "--op", "--json", "--probe", "v(out)"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["analysis"], "op");
    assert_eq!(json["labels"][0], "v(out)");
    assert_eq!(json["values"][0], 7.5);
}

#[test]
fn an_ac_sweep_gives_the_table_of_the_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC_AC);
    let (code, out, err) = command(&[
        "sim", &file, "--ac", "dec", "10", "1", "1k", "--probe", "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        "freq_hz,mag(v(out)),db(v(out)),phase_deg(v(out)),delay_s(v(out))"
    );
    assert_eq!(lines.count(), 31);
    let (_, json, _) = command(&[
        "sim", &file, "--ac", "lin", "3", "100", "300", "--json", "--probe", "v(out)",
    ]);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["freq_hz"].as_array().unwrap().len(), 3);
    let db = json["db"][0][0].as_f64().unwrap();
    assert!(db < 0.0 && db > -2.0, "{db}");
}

#[test]
fn a_transient_ends_at_the_stop_time_near_63_percent() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC);
    let (code, out, err) = command(&[
        "sim", &file, "--tran", "10u", "1m", "--probe", "v(out)", "--method", "trap",
    ]);
    assert_eq!(code, 0, "{err}");
    let last = out.lines().last().unwrap();
    let fields: Vec<&str> = last.split(',').collect();
    assert_eq!(fields[0], "1e-3");
    let v: f64 = fields[1].parse().unwrap();
    assert!((v - 0.632_120_558_8).abs() < 1e-3, "{v}");
    let (code, adaptive, _) = command(&[
        "sim",
        &file,
        "--tran",
        "10u",
        "1m",
        "--adaptive",
        "--probe",
        "v(out)",
    ]);
    assert_eq!(code, 0);
    assert!(adaptive.lines().count() < out.lines().count() + 200);
}

#[test]
fn out_writes_the_table_and_the_json_and_two_runs_are_identical() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "rc.cir", RC);
    let base_a = dir.path().join("made").join("a");
    let base_b = dir.path().join("made").join("b");
    for base in [&base_a, &base_b] {
        let (code, out, err) = command(&[
            "sim",
            &file,
            "--tran",
            "20u",
            "1m",
            "--out",
            base.to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("a.csv") || out.contains("b.csv"), "{out}");
    }
    for extension in ["csv", "json"] {
        let a = fs::read(base_a.with_extension(extension)).unwrap();
        let b = fs::read(base_b.with_extension(extension)).unwrap();
        assert!(!a.is_empty());
        assert_eq!(a, b);
    }
}

fn id(n: u32) -> String {
    format!("018b0000-0000-7000-8000-{n:012x}")
}

fn schematic_json() -> String {
    let (symbol, p1, p2) = (id(1), id(2), id(3));
    let (v1, r1, r2) = (id(4), id(5), id(6));
    let (n_in, n_out, n_gnd) = (id(7), id(8), id(9));
    let part = |name: &str, value: &str| {
        format!("{{\"refdes\":\"{name}\",\"value\":\"{value}\",\"symbol\":\"{symbol}\"}}")
    };
    let pin = |part: &str, pin: &str| format!("{{\"part\":\"{part}\",\"pin\":\"{pin}\"}}");
    format!(
        "{{\"name\":\"divider\",\"symbols\":{{\"{symbol}\":{{\"name\":\"R\",\"pins\":{{\"{p1}\":{{\"number\":\"1\"}},\"{p2}\":{{\"number\":\"2\"}}}}}}}},\
\"parts\":{{\"{v1}\":{},\"{r1}\":{},\"{r2}\":{}}},\
\"nets\":{{\"{n_in}\":{{\"name\":\"IN\",\"pins\":[{},{}]}},\
\"{n_out}\":{{\"name\":\"OUT\",\"pins\":[{},{}]}},\
\"{n_gnd}\":{{\"name\":\"GND\",\"pins\":[{},{}]}}}}}}",
        part("V1", "10"),
        part("R1", "1k"),
        part("R2", "3k"),
        pin(&v1, &p1),
        pin(&r1, &p1),
        pin(&r1, &p2),
        pin(&r2, &p1),
        pin(&v1, &p2),
        pin(&r2, &p2),
    )
}

#[test]
fn a_schematic_is_translated_reported_and_simulated() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "schematic.json", &schematic_json());
    let (code, out, err) = command(&["sim", &file, "--op"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v(out),7.5e0"), "{out}");
    assert!(err.contains("3 elements translated"), "{err}");
    assert!(err.contains("ground nets: GND"), "{err}");
    let (code, netlist, _) = command(&["sim", &file, "--netlist"]);
    assert_eq!(code, 0);
    assert!(netlist.starts_with("divider\n"), "{netlist}");
    assert!(netlist.contains("R2 out 0 3e3\n"), "{netlist}");
    assert!(netlist.ends_with(".end\n"));
    // a file that is not a schematic of the project format
    let other = write(dir.path(), "other.json", "{\"hello\":1}");
    let (code, _, err) = command(&["sim", &other, "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-021"), "{err}");
}

#[test]
fn the_filter_check_agrees_and_asks_for_what_it_needs() {
    let (code, out, err) = command(&[
        "sim",
        "--check-filter",
        "butterworth",
        "3",
        "lowpass",
        "1meg",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("coherence with the RF filter over 41 frequencies"),
        "{out}"
    );
    let (code, _, err) = command(&[
        "sim",
        "--check-filter",
        "chebyshev",
        "4",
        "bandpass",
        "9meg",
        "11meg",
        "--ripple-db",
        "0.5",
    ]);
    assert_eq!(code, 0, "{err}");
    let (code, _, err) = command(&["sim", "--check-filter", "chebyshev", "4", "lowpass", "1meg"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-016"), "{err}");
}

#[test]
fn unusable_input_gives_exit_code_2_with_a_code_and_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let bad = write(dir.path(), "bad.cir", "t\nV1 a 0 1\nR1 a 0 oops\n");
    let (code, _, err) = command(&["sim", &bad, "--op"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-SIM-002") && err.contains("line 3"),
        "{err}"
    );
    let (code, _, err) = command(&["sim", "--lang", "es", &bad, "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("El valor no es válido"), "{err}");

    let singular = write(dir.path(), "singular.cir", "t\nV1 a 0 1\nV2 a 0 2\n");
    let (code, _, err) = command(&["sim", &singular, "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-009"), "{err}");

    let floating = write(
        dir.path(),
        "floating.cir",
        "t\nV1 a 0 1\nC1 a b 1u\nR1 b c 1k\n",
    );
    let (code, _, err) = command(&["sim", &floating, "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-010"), "{err}");

    let missing = dir.path().join("missing.cir");
    let (code, _, err) = command(&["sim", missing.to_str().unwrap(), "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-021"), "{err}");

    let ok = write(dir.path(), "ok.cir", DIVIDER);
    for args in [
        vec!["sim"],
        vec!["sim", ok.as_str()],
        vec!["sim", "--op"],
        vec!["sim", ok.as_str(), "--op", "--probe", "v(zz)"],
        vec!["sim", ok.as_str(), "--ac", "dec", "10", "0", "100"],
        vec!["sim", ok.as_str(), "--tran", "1", "0.5"],
        vec!["sim", ok.as_str(), "--nope"],
        vec!["sim", ok.as_str(), "--op", "--lang", "xx"],
        vec!["sim", ok.as_str(), "--ac", "log", "10", "1", "100"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SIM-"), "{args:?}: {err}");
    }
}
