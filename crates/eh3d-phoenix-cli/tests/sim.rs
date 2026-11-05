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

const MOS_INVERTER: &str = "nmos inverter\n.model nm NMOS(VTO=1 KP=2e-5 W=1e-3 L=1e-5)\nVDD vdd 0 DC 5\nVIN in 0 DC 0\nRD vdd out 1k\nM1 out in 0 0 nm\n";

#[test]
fn a_dc_sweep_is_printed_and_reports_its_convergence() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "inverter.cir", MOS_INVERTER);
    let (code, out, err) = command(&[
        "sim", &file, "--dc", "vin", "0", "5", "0.5", "--probe", "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    let mut lines = out.lines();
    assert_eq!(lines.next().unwrap(), "sweep(vin),v(out)");
    assert_eq!(lines.count(), 11);
    // the point at 2 V is in saturation: 5 V - 1 kohm x 1 mA
    let row = out.lines().nth(5).unwrap();
    let (vin, vout) = row.split_once(',').unwrap();
    assert_eq!(vin, "2e0");
    assert!((vout.parse::<f64>().unwrap() - 4.0).abs() < 1e-4, "{row}");
    let (code, json, _) = command(&[
        "sim", &file, "--dc", "VIN", "5", "0", "-1", "--json", "--probe", "v(out)",
    ]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(json["analysis"], "dc");
    assert_eq!(json["source"], "vin");
    assert_eq!(json["sweep"].as_array().unwrap().len(), 6);
    assert!(json["residual"].as_f64().unwrap() < 1e-4);
    // the operating point reports the iterations, the strategy and the residual
    let (code, json, _) = command(&["sim", &file, "--op", "--json"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(json["iterations"].as_u64().unwrap() >= 1);
    assert_eq!(json["strategy"], "direct");
    assert!(json["residual"].as_f64().is_some());
}

#[test]
fn a_bad_sweep_gives_exit_code_2_with_a_code() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "inverter.cir", MOS_INVERTER);
    for args in [
        vec!["sim", file.as_str(), "--dc", "vin", "0", "5", "0"],
        vec!["sim", file.as_str(), "--dc", "vin", "0", "5", "-1"],
        vec!["sim", file.as_str(), "--dc", "nobody", "0", "5", "1"],
        vec!["sim", file.as_str(), "--dc", "rd", "0", "5", "1"],
        vec!["sim", file.as_str(), "--dc", "vin", "0", "5"],
        vec!["sim", file.as_str(), "--dc", "vin", "a", "5", "1"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SIM-"), "{args:?}: {err}");
    }
}

#[test]
fn the_analysis_of_the_netlist_is_run_when_none_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(
        dir.path(),
        "asked.cir",
        "divider\nV1 in 0 DC 10\nR1 in out 1k\nR2 out 0 3k\n.dc v1 5 10 5\n.op\n",
    );
    let (code, out, err) = command(&["sim", &file, "--probe", "v(out)"]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("2 analyses"), "{err}");
    let mut lines = out.lines();
    assert_eq!(lines.next().unwrap(), "sweep(v1),v(out)");
    for (row, (v, want)) in lines.zip([(5.0, 3.75), (10.0, 7.5)]) {
        let (x, y) = row.split_once(',').unwrap();
        assert!((x.parse::<f64>().unwrap() - v).abs() < 1e-12, "{row}");
        assert!((y.parse::<f64>().unwrap() - want).abs() < 1e-9, "{row}");
    }
    assert_eq!(out.lines().count(), 3);
    // naming an analysis wins over the netlist
    let (code, out, _) = command(&["sim", &file, "--op", "--probe", "v(out)"]);
    assert_eq!(code, 0);
    assert_eq!(out, "name,value\nv(out),7.5e0\n");
    // the writer keeps the directives
    let (code, out, _) = command(&["sim", &file, "--netlist"]);
    assert_eq!(code, 0);
    assert!(out.contains(".dc v1 5e0 1e1 5e0\n.op\n"), "{out}");
}

#[test]
fn a_file_of_models_is_read_and_an_include_stays_in_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let models = write(
        dir.path(),
        "models.lib",
        "* diode models\n.model dm D(IS=1e-14 N=1)\n.model led1 LED\n",
    );
    let circuit = "diode\nV1 in 0 DC 5\nR1 in d 1k\nD1 d 0 dm\n";
    let file = write(dir.path(), "diode.cir", circuit);
    // without the file the model is missing
    let (code, _, err) = command(&["sim", &file, "--op"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-SIM-005") && err.contains("line 4"),
        "{err}"
    );
    let (code, out, err) = command(&["sim", &file, "--op", "--model", &models, "--probe", "v(d)"]);
    assert_eq!(code, 0, "{err}");
    let vd: f64 = out
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    assert!(vd > 0.65 && vd < 0.75, "{out}");
    // the file of models is never changed
    assert_eq!(
        fs::read_to_string(&models).unwrap(),
        "* diode models\n.model dm D(IS=1e-14 N=1)\n.model led1 LED\n"
    );
    // an include of a file in the same folder works
    let included = write(
        dir.path(),
        "included.cir",
        "diode\n.include models.lib\nV1 in 0 DC 5\nR1 in d 1k\nD1 d 0 dm\n",
    );
    let (code, out, err) = command(&["sim", &included, "--op", "--probe", "v(d)"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v(d),"), "{out}");
    // an include of another folder is refused with the code and the line
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    let outside = write(
        dir.path(),
        "outside.cir",
        "diode\nV1 in 0 DC 5\n.include sub/models.lib\nR1 in d 1k\nD1 d 0 dm\n",
    );
    let (code, _, err) = command(&["sim", &outside, "--op"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-SIM-023") && err.contains("line 3"),
        "{err}"
    );
    // a missing file of models is an input error and a schematic takes none
    let nothing = dir.path().join("nothing.lib");
    let (code, _, err) = command(&["sim", &file, "--op", "--model", nothing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-021"), "{err}");
    let (code, _, err) = command(&["sim", &file, "--op", "--model"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-016"), "{err}");
}
