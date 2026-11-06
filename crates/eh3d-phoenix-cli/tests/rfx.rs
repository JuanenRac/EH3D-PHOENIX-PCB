// SPDX-License-Identifier: GPL-3.0-or-later
//! The extended `phoenix rf` commands: pi and T networks, the equivalents with lines, filters with
//! standard values and the simulated response, S parameters with a mask, and stability.

use eh3d_phoenix_cli::run;
use serde_json::Value;

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn json_of(list: &[&str]) -> Value {
    let (code, out, err) = call(list);
    assert_eq!(code, 0, "{err}\n{out}");
    serde_json::from_str(&out).unwrap()
}

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

fn path_in(dir: &tempfile::TempDir, name: &str) -> String {
    dir.path().join(name).to_str().unwrap().to_string()
}

const LOWPASS: &str = "# third-order Butterworth low-pass for 100 MHz\n\
    shunt C 31.83p\nseries L 159.15n\nshunt C 31.83p\n";

const JOB: &str = r#"{
  "stackup": {"name": "two-layer", "layers": [
    {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
    {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
    {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
  "materials": [
    {"name": "fr4", "dk_points": [[0.001, 4.4], [200.0, 4.4]], "df_points": [[0.001, 0.02], [200.0, 0.02]]}
  ],
  "lines": [{"name": "feed", "kind": "microstrip", "layer": "top", "width_nm": 3000000}]
}"#;

const AMPLIFIER: &str = "! test amplifier with invented values\n# MHz S RI R 50\n\
    100 0.10 0.00 3.00 0.50 0.05 0.00 0.30 -0.10\n\
    200 0.20 0.05 2.80 0.40 0.05 0.01 0.35 -0.12\n";

#[test]
fn a_pi_network_is_designed_with_its_elements_bandwidth_and_path() {
    let (code, text, err) = call(&[
        "rf",
        "match",
        "pi",
        "--source",
        "50",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--q",
        "5",
        "--pass",
        "low",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    for want in [
        "Matching network",
        "pi low-pass",
        "110.26",
        "51.81",
        "79.57",
        "Loaded Q: 5.0000",
        "Virtual resistance: 3.8462 ohm",
        "Path on the Smith chart",
        "Estimate:",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
}

#[test]
fn the_json_of_a_match_is_deterministic_and_says_it_is_an_estimate() {
    let args = [
        "rf",
        "match",
        "t",
        "--source",
        "50+j10",
        "--load",
        "120-j30",
        "--freq-ghz",
        "2.4",
        "--q",
        "15",
        "--json",
    ];
    let first = json_of(&args);
    assert_eq!(first["format"], "phoenix-rf-match/1");
    assert_eq!(first["estimate"], true);
    // low-pass and high-pass
    let networks = first["networks"].as_array().unwrap();
    assert_eq!(networks.len(), 2);
    for net in networks {
        assert_eq!(net["shape"], "T");
        assert_eq!(net["parts"].as_array().unwrap().len(), 3);
        // the network gives the conjugate of the source: 50 - j10
        let zin = &net["input_impedance"];
        assert!((zin["re"].as_f64().unwrap() - 50.0).abs() < 1e-6, "{zin}");
        assert!((zin["im"].as_f64().unwrap() + 10.0).abs() < 1e-6, "{zin}");
        let points = net["smith"]["points"].as_array().unwrap();
        assert_eq!(points.len(), 4);
        assert_eq!(points[0]["label"], "load");
    }
    let (_, a, _) = call(&args);
    let (_, b, _) = call(&args);
    assert_eq!(a, b);
}

#[test]
fn the_l_network_has_the_extended_report_with_a_q_and_a_bandwidth() {
    let value = json_of(&[
        "rf",
        "match",
        "l",
        "--source",
        "50",
        "--load",
        "200",
        "--freq-ghz",
        "0.1",
        "--json",
    ]);
    let networks = value["networks"].as_array().unwrap();
    assert!(!networks.is_empty());
    for net in networks {
        assert_eq!(net["shape"], "L");
        assert!((net["q_network"].as_f64().unwrap() - 3.0f64.sqrt()).abs() < 1e-9);
        assert!(net["bandwidth"]["rule_of_thumb_hz"].as_f64().unwrap() > 0.0);
    }
    // without the new options the report of the first level is printed
    let (code, text, _) = call(&[
        "rf",
        "match",
        "l",
        "--source",
        "50",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
    ]);
    assert_eq!(code, 0);
    assert!(text.contains("79.5775 nH"), "{text}");
}

#[test]
fn the_tables_are_in_the_eight_languages() {
    let base = [
        "rf",
        "match",
        "pi",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--pass",
        "low",
    ];
    for (tag, word) in [
        ("en", "Matching network"),
        ("es", "Red de adaptación"),
        ("de", "Anpassnetzwerk"),
        ("fr", "Réseau d'adaptation"),
        ("it", "Rete di adattamento"),
        ("ja", "整合ネットワーク"),
        ("zh", "匹配网络"),
        ("pt", "Rede de adaptação"),
    ] {
        let mut args = base.to_vec();
        args.extend(["--lang", tag]);
        let (code, text, _) = call(&args);
        assert_eq!(code, 0, "{tag}");
        assert!(text.contains(word), "{tag}\n{text}");
    }
}

#[test]
fn the_picture_is_written_once_and_never_over_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let svg = path_in(&dir, "path.svg");
    let args = [
        "rf",
        "match",
        "pi",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--pass",
        "low",
        "--svg",
        &svg,
    ];
    let (code, _, err) = call(&args);
    assert_eq!(code, 0, "{err}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.starts_with("<svg ") && text.trim_end().ends_with("</svg>"));
    let (code, _, err) = call(&args);
    assert_eq!(code, 2);
    assert!(err.contains("cannot write"), "{err}");
    assert_eq!(std::fs::read_to_string(&svg).unwrap(), text);
    // a number that is not a network of the list
    let other = path_in(&dir, "other.svg");
    let (code, _, _) = call(&[
        "rf",
        "match",
        "pi",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--pass",
        "low",
        "--svg",
        &other,
        "--pick",
        "3",
    ]);
    assert_eq!(code, 2);
    assert!(!std::path::Path::new(&other).exists());
}

#[test]
fn the_output_file_is_new_and_a_pi_network_below_the_l_q_has_no_result() {
    let dir = tempfile::tempdir().unwrap();
    let out = path_in(&dir, "match.json");
    let args = [
        "rf",
        "match",
        "pi",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--json",
        "--out",
        &out,
    ];
    let (code, printed, err) = call(&args);
    assert_eq!((code, printed.is_empty()), (0, true), "{err}");
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(value["format"], "phoenix-rf-match/1");
    assert_eq!(call(&args).0, 2);
    // a Q below the one of the L network (1 for 50 and 100 ohm) has no network
    let (code, _, err) = call(&[
        "rf",
        "match",
        "pi",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
        "--q",
        "0.5",
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-RF-006"), "{err}");
}

#[test]
fn the_equivalents_with_lines_have_lengths_in_metres() {
    let value = json_of(&[
        "rf",
        "match",
        "lines",
        "--load",
        "30+j40",
        "--freq-ghz",
        "2.4",
        "--z0",
        "50",
        "--eps-eff",
        "3",
        "--json",
    ]);
    assert_eq!(value["format"], "phoenix-rf-lines/1");
    assert!(!value["single_stub"].as_array().unwrap().is_empty());
    assert!(!value["double_stub"].as_array().unwrap().is_empty());
    let quarter = &value["quarter_wave"].as_array().unwrap()[0];
    // a quarter of the guided wavelength at 2.4 GHz with an effective permittivity of 3
    let wavelength = 299_792_458.0 / (2.4e9 * 3.0f64.sqrt());
    assert!((quarter["section_m"].as_f64().unwrap() - wavelength / 4.0).abs() < 1e-9);
    let (code, text, _) = call(&[
        "rf",
        "match",
        "lines",
        "--load",
        "30+j40",
        "--freq-ghz",
        "2.4",
        "--eps-eff",
        "3",
        "--spacing-deg",
        "60",
    ]);
    assert_eq!(code, 0);
    for want in [
        "Single stub",
        "Double stub",
        "Quarter-wave transformer",
        " mm",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
}

#[test]
fn a_line_of_the_stackup_gives_the_impedance_and_the_permittivity() {
    let dir = tempfile::tempdir().unwrap();
    let job = write(&dir, "job.json", JOB);
    let value = json_of(&[
        "rf",
        "match",
        "lines",
        "--load",
        "100",
        "--freq-ghz",
        "1",
        "--stackup",
        &job,
        "--line",
        "feed",
        "--json",
    ]);
    assert_eq!(value["line"]["name"], "feed");
    let z0 = value["line"]["z0_ohm"].as_f64().unwrap();
    let eps = value["line"]["eps_eff"].as_f64().unwrap();
    assert!((30.0..80.0).contains(&z0), "{z0}");
    assert!((1.0..4.4).contains(&eps), "{eps}");
    let (code, _, err) = call(&[
        "rf",
        "match",
        "lines",
        "--load",
        "100",
        "--freq-ghz",
        "1",
        "--stackup",
        &job,
        "--line",
        "nothing",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("no line named"), "{err}");
}

#[test]
fn the_double_stub_says_when_the_load_is_in_the_forbidden_region() {
    let value = json_of(&[
        "rf",
        "match",
        "lines",
        "--load",
        "16.666666666666668",
        "--freq-ghz",
        "1",
        "--spacing-deg",
        "45",
        "--json",
    ]);
    assert!(value["double_stub"].as_array().unwrap().is_empty());
    let reasons = value["unavailable"].as_array().unwrap();
    assert!(reasons
        .iter()
        .any(|r| r["kind"].as_str().unwrap().starts_with("double stub")));
    assert!(!value["single_stub"].as_array().unwrap().is_empty());
}

#[test]
fn standard_values_move_the_cutoff_by_the_amount_of_the_rounding() {
    let value = json_of(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--snap",
        "E24",
        "--json",
    ]);
    let snap = &value["standard_values"];
    assert_eq!(snap["series"], "E24");
    let names: Vec<&str> = snap["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["C1", "L2", "C3"]);
    let shift = snap["edge_errors_percent"][0].as_f64().unwrap();
    assert!((shift + 1.5535).abs() < 1e-3, "{shift}");
    let (code, text, _) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--snap",
        "e12",
        "--lang",
        "de",
    ]);
    assert_eq!(code, 0);
    assert!(text.contains("Normreihe: E12"), "{text}");
    let (code, _, err) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--snap",
        "E7",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RF-013"), "{err}");
}

#[test]
fn the_simulated_response_agrees_with_the_abcd_response() {
    let value = json_of(&[
        "rf",
        "filter",
        "chebyshev",
        "4",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--ripple-db",
        "0.5",
        "--sweep-ghz",
        "0.01,0.3,7",
        "--simulate",
        "--netlist",
        "--json",
    ]);
    let rows = value["response"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    for row in rows {
        let simulated = &row["simulated"];
        let a = row["gain_db"].as_f64().unwrap();
        let b = simulated["gain_db"].as_f64().unwrap();
        assert!((a - b).abs() < 1e-6, "{a} {b}");
    }
    assert!(value["spice"]
        .as_str()
        .unwrap()
        .starts_with("ladder filter"));
    // a simulation needs a sweep
    let (code, _, err) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--simulate",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("--sweep-ghz"), "{err}");
}

#[test]
fn the_approximations_without_a_table_are_refused_by_name() {
    for family in ["legendre", "elliptic"] {
        let (code, _, err) = call(&["rf", "filter", family, "3", "lowpass", "--fc-ghz", "0.1"]);
        assert_eq!(code, 2, "{family}");
        assert!(err.contains("not implemented"), "{err}");
    }
}

#[test]
fn the_s_parameters_of_a_network_are_tabled_and_checked_against_a_mask() {
    let dir = tempfile::tempdir().unwrap();
    let net = write(&dir, "lowpass.txt", LOWPASS);
    let pass = write(&dir, "pass.txt", "s21 min 0 50M -1\ns11 max 0 50M -10\n");
    let fail = write(&dir, "fail.txt", "s21 min 0 300M -1\n");
    let base = ["rf", "sparams", "--net", &net, "--sweep-ghz", "0.01,1,5"];
    let (code, text, err) = call(&base);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("|S11| dB") && text.contains("VSWR"), "{text}");
    let mut with_pass = base.to_vec();
    with_pass.extend(["--mask", &pass]);
    let (code, text, _) = call(&with_pass);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("Overall: pass"), "{text}");
    let mut with_fail = base.to_vec();
    with_fail.extend(["--mask", &fail]);
    let (code, text, _) = call(&with_fail);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("Overall: fail"), "{text}");
    // the JSON says which segment failed
    with_fail.push("--json");
    let (code, out, _) = call(&with_fail);
    assert_eq!(code, 1);
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["mask"]["passed"], false);
    assert_eq!(value["mask"]["segments"][0]["parameter"], "s21");
    assert_eq!(value["format"], "phoenix-rf-sparams/1");
}

#[test]
fn the_response_of_a_network_is_written_as_a_touchstone_file_that_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let net = write(&dir, "lowpass.txt", LOWPASS);
    let out = path_in(&dir, "lowpass.s2p");
    let (code, _, err) = call(&[
        "rf",
        "sparams",
        "--net",
        &net,
        "--sweep-ghz",
        "0.01,1,5",
        "--s2p-out",
        &out,
        "--format",
        "db",
    ]);
    assert_eq!(code, 0, "{err}");
    let (code, text, err) = call(&["rf", "touchstone", &out, "--freq-ghz", "0.1"]);
    assert_eq!(code, 0, "{err}");
    assert!(text.contains("points: 5"), "{text}");
    // the file of the response is read by the same command
    let (code, table, _) = call(&["rf", "sparams", "--s2p", &out]);
    assert_eq!(code, 0);
    assert!(table.contains("|S21| dB"), "{table}");
    // an existing file is never replaced
    let (code, _, err) = call(&[
        "rf",
        "sparams",
        "--net",
        &net,
        "--sweep-ghz",
        "0.01,1,5",
        "--s2p-out",
        &out,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("cannot write"), "{err}");
}

#[test]
fn bad_descriptions_and_masks_name_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let bad_net = write(&dir, "bad.txt", "series L 10n\nshunt C two\n");
    let (code, _, err) = call(&["rf", "sparams", "--net", &bad_net, "--sweep-ghz", "0.1,1,3"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-RF-011") && err.contains("line 2"),
        "{err}"
    );
    let net = write(&dir, "ok.txt", LOWPASS);
    let bad_mask = write(&dir, "mask.txt", "s21 min 0 100M -3\ns21 sideways 0 1 2\n");
    let (code, _, err) = call(&[
        "rf",
        "sparams",
        "--net",
        &net,
        "--sweep-ghz",
        "0.1,1,3",
        "--mask",
        &bad_mask,
    ]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-RF-012") && err.contains("line 2"),
        "{err}"
    );
    // the arguments
    assert_eq!(call(&["rf", "sparams"]).0, 2);
    assert_eq!(call(&["rf", "sparams", "--net", &net]).0, 2);
    assert_eq!(
        call(&[
            "rf",
            "sparams",
            "--net",
            "/no/such/file.txt",
            "--sweep-ghz",
            "1,2,3"
        ])
        .0,
        2
    );
    assert_eq!(
        call(&["rf", "sparams", "--net", &net, "--s2p", "x.s2p"]).0,
        2
    );
    assert_eq!(call(&["rf", "stability"]).0, 2);
}

#[test]
fn an_unconditionally_stable_amplifier_has_a_conjugate_match() {
    let value = json_of(&[
        "rf",
        "stability",
        "--s11",
        "0.5@0",
        "--s12",
        "0.1@0",
        "--s21",
        "2@0",
        "--s22",
        "0.5@0",
        "--json",
    ]);
    assert_eq!(value["format"], "phoenix-rf-stability/1");
    let point = &value["points"][0];
    assert!((point["k"].as_f64().unwrap() - 1.25625).abs() < 1e-9);
    assert_eq!(point["unconditionally_stable"], true);
    let detail = &value["detail"];
    assert!(!detail["conjugate_match"].is_null());
    assert!((detail["load_circle"]["radius"].as_f64().unwrap() - 0.8080808080808081).abs() < 1e-9);
    let mag = detail["max_available_gain"].as_f64().unwrap();
    let gain = detail["conjugate_match"]["gain"].as_f64().unwrap();
    assert!((mag - gain).abs() / mag < 1e-9);
}

#[test]
fn a_conditionally_stable_transistor_has_circles_and_no_conjugate_match() {
    let (code, text, _) = call(&[
        "rf",
        "stability",
        "--s11",
        "0.65@-95",
        "--s12",
        "0.035@40",
        "--s21",
        "5@115",
        "--s22",
        "0.8@-35",
        "--lang",
        "de",
    ]);
    assert_eq!(code, 0);
    for want in [
        "Stabilität des Verstärkers",
        "Stabilitätskreis der Quelle",
        "Stabilitätskreis der Last",
        "keine (nicht bedingungslos stabil)",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
}

#[test]
fn the_stability_of_a_touchstone_file_is_tabled_and_detailed_at_a_frequency() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "amp.s2p", AMPLIFIER);
    let table = json_of(&["rf", "stability", "--s2p", &path, "--json"]);
    assert_eq!(table["points"].as_array().unwrap().len(), 2);
    assert!(table["detail"].is_null());
    let detail = json_of(&[
        "rf",
        "stability",
        "--s2p",
        &path,
        "--freq-ghz",
        "0.15",
        "--json",
    ]);
    assert_eq!(detail["points"].as_array().unwrap().len(), 1);
    assert!(!detail["detail"].is_null());
    // a frequency outside the file is not extrapolated
    let (code, _, err) = call(&["rf", "stability", "--s2p", &path, "--freq-ghz", "5"]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-RF-010"), "{err}");
}
