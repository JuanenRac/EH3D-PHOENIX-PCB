// SPDX-License-Identifier: GPL-3.0-or-later
//! The reading mode of the command `ipc2581` (`--read`) on files written by the test itself, in
//! the shape that IPC-2581 files have: the report in several languages, the JSON, the stack-up as
//! HTML and as a job file, the limits, and the files that are refused.

use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;

const FILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<IPC-2581 revision="C" xmlns="http://webstds.ipc.org/2581">
 <Content roleRef="Owner"><FunctionMode mode="FULL" level="1"/><StepRef name="b"/></Content>
 <Ecad name="b">
  <CadHeader units="MICRON">
   <Spec name="CORE"><General type="MATERIAL"><Property text="Laminate A"/></General><Dielectric type="DIELECTRIC_CONSTANT"><Property value="4.3"/></Dielectric><Dielectric type="LOSS_TANGENT"><Property value="0.019"/></Dielectric></Spec>
  </CadHeader>
  <CadData>
   <Layer name="TOP" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/>
   <Layer name="D1" layerFunction="DIELCORE" side="INTERNAL" polarity="POSITIVE"/>
   <Layer name="BOT" layerFunction="SIGNAL" side="BOTTOM" polarity="POSITIVE"/>
   <Layer name="DRL" layerFunction="DRILL" side="ALL" polarity="POSITIVE"/>
   <Stackup name="S2" overallThickness="1600" tolPlus="100" tolMinus="100" whereMeasured="METAL">
    <StackupGroup name="G" thickness="1600">
     <StackupLayer layerOrGroupRef="TOP" thickness="35" sequence="1"/>
     <StackupLayer layerOrGroupRef="D1" thickness="1530" sequence="2"><SpecRef id="CORE"/></StackupLayer>
     <StackupLayer layerOrGroupRef="BOT" thickness="35" sequence="3"/>
    </StackupGroup>
   </Stackup>
   <Step name="b" type="BOARD">
    <Component refDes="R1" packageRef="P" layerRef="TOP" part="X" mountType="SMT"><Xform rotation="0"/><Location x="1000" y="2000"/></Component>
    <LogicalNet name="N1"><PinRef componentRef="R1" pin="1"/></LogicalNet>
    <LayerFeature layerRef="DRL"><Set><Hole name="H1" diameter="300" platingStatus="PLATED" x="10" y="20"/><Hole name="H2" diameter="600" platingStatus="NONPLATED" x="30" y="40"/></Set></LayerFeature>
    <Mystery/>
   </Step>
  </CadData>
 </Ecad>
</IPC-2581>"#;

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

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn the_report_gives_the_counts_the_stackup_and_what_was_left_out() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let (code, out, err) = command(&["ipc2581", "--read", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("Revision: C"), "{out}");
    assert!(out.contains("Unit of the lengths: MICRON"), "{out}");
    assert!(out.contains("Layers: 4 (of them copper: 2)"), "{out}");
    assert!(out.contains("Holes: 2 (plated: 1, not plated: 1)"), "{out}");
    assert!(out.contains("Placed parts: 1"), "{out}");
    assert!(out.contains("Laminate A | 1530.000 | 4.3 | 0.019"), "{out}");
    assert!(
        out.contains("Thickness declared by the file (um): 1600.000"),
        "{out}"
    );
    assert!(out.contains("PHOENIX-IPC2581-012"), "{out}");
    assert!(out.contains("[Mystery]"), "{out}");
    // nothing is written without --html or --stackup-out
    assert_eq!(names(dir.path()), vec!["board.xml"]);
}

#[test]
fn the_report_comes_in_each_language() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let mut seen = std::collections::BTreeSet::new();
    for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let (code, out, err) = command(&["ipc2581", "--read", &path, "--lang", lang]);
        assert_eq!(code, 0, "{lang}: {err}");
        assert!(out.contains("1530.000"), "{lang}: {out}");
        assert!(seen.insert(out), "{lang} repeats another language");
    }
    let (_, pt, _) = command(&["ipc2581", "--read", &path, "--lang", "pt"]);
    assert!(pt.contains("Camadas: 4"), "{pt}");
}

#[test]
fn the_json_has_the_counts_the_stackup_and_the_findings() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let (code, out, err) = command(&["ipc2581", "--read", &path, "--json"]);
    assert_eq!(code, 0, "{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["revision"], "C");
    assert_eq!(value["units"], "MICRON");
    assert_eq!(value["counts"]["layers"], 4);
    assert_eq!(value["counts"]["holes"], 2);
    assert_eq!(value["counts"]["components"], 1);
    assert_eq!(value["stackup"]["rows"].as_array().unwrap().len(), 3);
    assert_eq!(value["stackup"]["rows"][1]["thickness_nm"], 1_530_000);
    assert_eq!(value["stackup"]["rows"][1]["kind"], "core");
    assert_eq!(value["stackup"]["declared_nm"], 1_600_000);
    assert_eq!(value["findings"][0]["code"], "PHOENIX-IPC2581-012");
}

#[test]
fn the_stackup_is_written_as_html_and_as_a_job_file_and_never_replaces_a_file_by_chance() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let html = dir.path().join("stack.html");
    let job = dir.path().join("job.json");
    let (html_s, job_s) = (html.to_str().unwrap(), job.to_str().unwrap());
    let (code, out, err) = command(&[
        "ipc2581",
        "--read",
        &path,
        "--html",
        html_s,
        "--stackup-out",
        job_s,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains(&format!("written: {html_s}")), "{out}");
    let page = fs::read_to_string(&html).unwrap();
    assert!(page.contains("1530.000"), "{page}");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&job).unwrap()).unwrap();
    assert_eq!(value["stackup"]["name"], "S2");
    assert_eq!(value["stackup"]["layers"].as_array().unwrap().len(), 3);
    assert_eq!(value["stackup"]["layers"][1]["kind"], "core");
    assert_eq!(value["stackup"]["layers"][1]["thickness_nm"], 1_530_000);
    assert_eq!(value["stackup"]["layers"][1]["material"], "Laminate A");
    assert_eq!(value["materials"][0]["name"], "Laminate A");
    assert_eq!(value["materials"][0]["dk_points"][0][1], 4.3);
    assert_eq!(value["materials"][0]["df_points"][0][1], 0.019);
    assert_eq!(value["lines"].as_array().unwrap().len(), 0);
    // a file that exists is not replaced without --overwrite
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--html", html_s]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-004"), "{err}");
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--html", html_s, "--overwrite"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        names(dir.path()),
        vec!["board.xml", "job.json", "stack.html"]
    );
}

#[test]
fn the_files_that_cannot_be_read_are_refused_with_a_code() {
    let dir = tempfile::tempdir().unwrap();
    let other = write_text(dir.path(), "other.xml", "<other/>");
    let (code, _, err) = command(&["ipc2581", "--read", &other]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-010"), "{err}");
    let broken = write_text(dir.path(), "broken.xml", "<IPC-2581><Content>");
    let (code, _, err) = command(&["ipc2581", "--read", &broken]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-009"), "{err}");
    let archive = dir.path().join("a.zip");
    fs::write(&archive, b"PK\x03\x04 not really an archive").unwrap();
    let (code, _, err) = command(&["ipc2581", "--read", archive.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("archive"), "{err}");
    let missing = dir.path().join("none.xml");
    let (code, _, err) = command(&["ipc2581", "--read", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-003"), "{err}");
}

#[test]
fn the_limits_are_settings_and_a_file_over_one_is_refused_naming_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let small = write_text(dir.path(), "small.cfg", "read_max_bytes = 100\n");
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--config", &small]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-011"), "{err}");
    assert!(err.contains("read_max_bytes"), "{err}");
    let shallow = write_text(dir.path(), "shallow.cfg", "read_max_depth = 4\n");
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--config", &shallow]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-011"), "{err}");
    let few = write_text(dir.path(), "few.cfg", "read_max_items = 1\n");
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--config", &few]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-IPC2581-011"), "{err}");
    let bad = write_text(dir.path(), "bad.cfg", "read_max_depth = 2\n");
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--config", &bad]);
    assert_eq!(code, 2);
    assert!(err.contains("read_max_depth"), "{err}");
    // without geometry the counts are the same
    let lean = write_text(dir.path(), "lean.cfg", "read_keep_geometry = false\n");
    let (code, with, _) = command(&["ipc2581", "--read", &path, "--json"]);
    assert_eq!(code, 0);
    let (code, without, _) = command(&["ipc2581", "--read", &path, "--json", "--config", &lean]);
    assert_eq!(code, 0);
    assert_eq!(with, without);
}

#[test]
fn bad_arguments_of_the_reading_mode_exit_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_text(dir.path(), "board.xml", FILE);
    let out = dir.path().join("o.xml");
    let out = out.to_str().unwrap();
    assert_eq!(command(&["ipc2581", "--read"]).0, 2);
    assert_eq!(command(&["ipc2581", "--read", &path, &path]).0, 2);
    assert_eq!(command(&["ipc2581", "--read", &path, "--out", out]).0, 2);
    assert_eq!(command(&["ipc2581", "--read", &path, "--d356", out]).0, 2);
    let (code, _, err) = command(&["ipc2581", "--read", &path, "--overwrite"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("--overwrite needs --html or --stackup-out"),
        "{err}"
    );
    let (code, _, err) = command(&[
        "ipc2581",
        "--read",
        &path,
        "--html",
        out,
        "--stackup-out",
        out,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("name the same file"), "{err}");
    let (code, _, err) = command(&["ipc2581", "--html", out, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("need --read"), "{err}");
    assert_eq!(names(dir.path()), vec!["board.xml"]);
}

#[test]
fn the_help_of_the_command_mentions_the_reading_mode() {
    let (code, out, _) = command(&["help", "ipc2581"]);
    assert_eq!(code, 0);
    assert!(out.contains("--read"), "{out}");
}
