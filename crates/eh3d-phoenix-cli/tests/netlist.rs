// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `netlist` and the option `--schematic` of `sim`, on a schematic written by the test
//! itself: a dry run, a new output file, the refusals, the table of models, the report with its
//! codes and the electrical rule check that comes first.

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

fn id(n: u32) -> String {
    format!("018b0000-0000-7000-8000-{n:012x}")
}

/// A source of 10 V across a resistor of 1k and one of 3k. With `loose` the last pin of the
/// ground net is left out, so one pin is on no net.
fn schematic_json(loose: bool) -> String {
    let (symbol, p1, p2) = (id(1), id(2), id(3));
    let (v1, r1, r2) = (id(4), id(5), id(6));
    let (n_in, n_out, n_gnd) = (id(7), id(8), id(9));
    let part = |name: &str, value: &str| {
        format!("{{\"refdes\":\"{name}\",\"value\":\"{value}\",\"symbol\":\"{symbol}\"}}")
    };
    let pin = |part: &str, pin: &str| format!("{{\"part\":\"{part}\",\"pin\":\"{pin}\"}}");
    let mut ground = vec![pin(&v1, &p2)];
    if !loose {
        ground.push(pin(&r2, &p2));
    }
    format!(
        "{{\"name\":\"divider\",\"symbols\":{{\"{symbol}\":{{\"name\":\"R\",\"pins\":{{\"{p1}\":{{\"number\":\"1\"}},\"{p2}\":{{\"number\":\"2\"}}}}}}}},\
\"parts\":{{\"{v1}\":{},\"{r1}\":{},\"{r2}\":{}}},\
\"nets\":{{\"{n_in}\":{{\"name\":\"IN\",\"pins\":[{},{}]}},\
\"{n_out}\":{{\"name\":\"OUT\",\"pins\":[{},{}]}},\
\"{n_gnd}\":{{\"name\":\"GND\",\"pins\":[{}]}}}}}}",
        part("V1", "10"),
        part("R1", "1k"),
        part("R2", "3k"),
        pin(&v1, &p1),
        pin(&r1, &p1),
        pin(&r1, &p2),
        pin(&r2, &p1),
        ground.join(","),
    )
}

#[test]
fn without_out_the_netlist_is_printed_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.json", &schematic_json(false));
    let (code, out, err) = command(&["netlist", &file]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("divider\n"), "{out}");
    assert!(out.contains("R1 in out 1e3\n"), "{out}");
    assert!(out.contains("R2 out 0 3e3\n"), "{out}");
    assert!(out.contains("V1 in 0 1e1\n"), "{out}");
    assert!(out.ends_with(".end\n"));
    assert!(err.contains("3 elements translated"), "{err}");
    assert!(err.contains("ground nets: GND"), "{err}");
    assert!(err.contains("dry run"), "{err}");
    // the only file in the folder is the input
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    // and two runs give the same text
    let (_, again, _) = command(&["netlist", &file]);
    assert_eq!(out, again);
}

#[test]
fn out_writes_a_new_file_and_never_an_input_or_a_file_that_exists() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.json", &schematic_json(false));
    let target = dir.path().join("divider.cir");
    let target = target.to_str().unwrap();
    let (code, out, err) = command(&["netlist", &file, "--out", target]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let text = fs::read_to_string(target).unwrap();
    assert!(text.contains("R1 in out 1e3\n"), "{text}");
    // the file that was written is a netlist the simulator reads
    let (code, op, err) = command(&["sim", target, "--op", "--probe", "v(out)"]);
    assert_eq!(code, 0, "{err}");
    assert!(op.contains("v(out),7.5e0"), "{op}");
    // it exists now, so it is not written over
    let (code, _, err) = command(&["netlist", &file, "--out", target]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETLIST-012"), "{err}");
    assert_eq!(fs::read_to_string(target).unwrap(), text);
    // the input is not written over, and a design format is not written
    let (code, _, err) = command(&["netlist", &file, "--out", &file]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETLIST-012"), "{err}");
    let other = dir.path().join("other.json");
    let (code, _, err) = command(&["netlist", &file, "--out", other.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETLIST-012"), "{err}");
    assert!(!other.exists());
}

#[test]
fn the_json_report_has_the_netlist_the_issues_and_the_rule_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.json", &schematic_json(false));
    let (code, out, err) = command(&["netlist", &file, "--json"]);
    assert_eq!(code, 0, "{err}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["schema"], "phoenix-netlist/1");
    assert!(json["netlist"].as_str().unwrap().contains("R1 in out"));
    assert_eq!(json["translated"].as_array().unwrap().len(), 3);
    assert_eq!(json["nodes"], 3);
    assert_eq!(json["grounds"][0], "GND");
    assert!(json["issues"].as_array().unwrap().is_empty());
    assert!(json["erc"].is_object());
    assert!(json["written"].is_null());
    let (_, skipped, _) = command(&["netlist", &file, "--json", "--no-erc"]);
    let json: serde_json::Value = serde_json::from_str(&skipped).unwrap();
    assert!(json["erc"].is_null());
}

#[test]
fn a_table_of_models_changes_the_netlist_and_a_bad_table_is_refused_with_its_line() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.json", &schematic_json(false));
    let table = write(
        dir.path(),
        "models.cfg",
        "refdes R = resistor\nrefdes V = vsource\nattr R2 SIM.VALUE = 4k7\n",
    );
    let (code, out, err) = command(&["netlist", &file, "--models", &table]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("R2 out 0 4.7e3\n"), "{out}");
    assert!(out.contains("R1 in out 1e3\n"), "{out}");
    // a table that has no rule for the parts leaves them out and says so, in the language asked for
    let empty = write(dir.path(), "empty.cfg", "# nothing\n");
    let (code, out, err) = command(&["netlist", &file, "--models", &empty, "--lang", "pt"]);
    assert_eq!(code, 1);
    assert!(!out.contains("R1 in"), "{out}");
    assert!(err.contains("PHOENIX-NETLIST-002"), "{err}");
    assert!(err.contains("A peça não tem modelo de simulação"), "{err}");
    assert!(err.contains("PHOENIX-NETLIST-009"), "{err}");
    // a line that is not valid
    let bad = write(
        dir.path(),
        "bad.cfg",
        "refdes R = resistor\nrefdes V = flux\n",
    );
    let (code, _, err) = command(&["netlist", &file, "--models", &bad]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETLIST-001"), "{err}");
    assert!(err.contains("line 2"), "{err}");
}

#[test]
fn a_pin_on_no_net_is_reported_and_the_rule_check_comes_first() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "loose.json", &schematic_json(true));
    let (code, _, err) = command(&["netlist", &file]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-NETLIST-010"), "{err}");
    assert!(err.contains("PHOENIX-NETLIST-003"), "{err}");
    assert!(err.contains("R2 pin 2"), "{err}");
    // without the rule check the loose pin is a warning only, and nothing stops a simulation
    let (code, _, err) = command(&["netlist", &file, "--no-erc"]);
    assert_eq!(code, 0, "{err}");
    assert!(!err.contains("PHOENIX-NETLIST-010"), "{err}");
    assert!(err.contains("PHOENIX-NETLIST-003"), "{err}");
}

#[test]
fn sim_takes_a_schematic_and_chains_the_netlist_to_the_solver() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "divider.json", &schematic_json(false));
    let (code, out, err) = command(&["sim", "--schematic", &file, "--op"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v(out),7.5e0"), "{out}");
    assert!(err.contains("3 elements translated"), "{err}");
    // a sweep of the source of the schematic
    let (code, out, err) = command(&[
        "sim",
        "--schematic",
        &file,
        "--dc",
        "V1",
        "0",
        "10",
        "5",
        "--probe",
        "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().count(), 4, "{out}");
    // the same table as for the netlist
    let table = write(
        dir.path(),
        "models.cfg",
        "refdes R = resistor\nrefdes V = vsource\nattr R2 SIM.VALUE = 1k\n",
    );
    let (code, out, err) = command(&[
        "sim",
        "--schematic",
        &file,
        "--models",
        &table,
        "--op",
        "--probe",
        "v(out)",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v(out),5e0"), "{out}");
    // an error of the report stops the simulation before it starts
    let loose = write(dir.path(), "loose.json", &schematic_json(true));
    let (code, out, err) = command(&["sim", "--schematic", &loose, "--op"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("PHOENIX-NETLIST-010"), "{err}");
    // a schematic and a netlist together are refused
    let (code, _, err) = command(&["sim", "--schematic", &file, &file, "--op"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SIM-016"), "{err}");
}

#[test]
fn the_arguments_are_checked() {
    let (code, _, err) = command(&["netlist"]);
    assert_eq!(code, 2);
    assert!(err.contains("phoenix netlist"), "{err}");
    let (code, _, err) = command(&["netlist", "--colour", "x.json"]);
    assert_eq!(code, 2);
    assert!(err.contains("unknown option"), "{err}");
    let (code, _, err) = command(&["netlist", "missing.json"]);
    assert_eq!(code, 2);
    assert!(err.contains("missing.json"), "{err}");
    let (code, _, err) = command(&["netlist", "--lang", "xx", "a.json"]);
    assert_eq!(code, 2);
    assert!(err.contains("language"), "{err}");
}
