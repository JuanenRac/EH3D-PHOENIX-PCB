// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix materials` and the option `--material` of `phoenix stackup`, against the
//! real records of the materials database. They need the feature `materials`.
#![cfg(feature = "materials")]

use eh3d_phoenix_cli::run;

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

/// The identifiers of the rows of `materials list --json ...`.
fn ids(list: &[&str]) -> Vec<String> {
    let mut args = vec!["materials", "list", "--json"];
    args.extend_from_slice(list);
    let (code, out, err) = call(&args);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    let rows: serde_json::Value = serde_json::from_str(&out).unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn the_list_filters_by_manufacturer_kind_and_dk_at_a_frequency() {
    let (code, text, err) = call(&[
        "materials",
        "list",
        "--manufacturer",
        "Rogers",
        "--kind",
        "ceramic_filled",
        "--at-ghz",
        "10",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("rogers/ro4350b"), "{text}");
    assert!(!text.contains("isola/"), "{text}");
    // Dk 3.48 at 10 GHz is inside 3.4 to 3.5; 3.0 (RO3003) and 3.38 (RO4003C) are not
    let found = ids(&[
        "--manufacturer",
        "rogers",
        "--min-dk",
        "3.4",
        "--max-dk",
        "3.5",
        "--at-ghz",
        "10",
    ]);
    assert!(found.iter().any(|id| id == "rogers/ro4350b"));
    assert!(!found.iter().any(|id| id == "rogers/ro3003"));
    assert!(!found.iter().any(|id| id == "rogers/ro4003c"));
}

#[test]
fn the_list_filters_by_glass_transition_and_by_relevance_to_phoenix() {
    // 370HR has a Tg of 180 C by DSC
    let hot = ids(&["--manufacturer", "isola", "--min-tg", "170"]);
    assert!(hot.iter().any(|id| id == "isola/370hr"));
    let hotter = ids(&["--manufacturer", "isola", "--min-tg", "400"]);
    assert!(hotter.is_empty());
    let all = ids(&["--manufacturer", "rogers"]);
    let useful = ids(&["--manufacturer", "rogers", "--phoenix"]);
    assert!(useful.len() <= all.len());
    assert!(useful.iter().all(|id| all.contains(id)));
}

#[test]
fn show_prints_one_record_with_its_values_at_a_frequency() {
    let (code, text, err) = call(&["materials", "show", "rogers/ro4350b", "--at-ghz", "10"]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("Rogers"), "{text}");
    assert!(text.contains("Dk 3.480, Df 0.00370"), "{text}");
    let (code, json, _) = call(&[
        "materials",
        "show",
        "rogers/ro4350b",
        "--at-ghz",
        "10",
        "--json",
    ]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["id"], "rogers/ro4350b");
    let dk = value["at_ghz"]["dk"].as_f64().unwrap();
    assert!((dk - 3.48).abs() < 1e-12, "{dk}");
    // 2.5 GHz is outside the one published Dk point, so no value is given
    let (_, json, _) = call(&[
        "materials",
        "show",
        "rogers/ro4350b",
        "--at-ghz",
        "2.5",
        "--json",
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value["at_ghz"]["dk"].is_null());
    assert!(value["at_ghz"]["df"].is_number());
}

#[test]
fn a_material_that_does_not_exist_and_bad_arguments_exit_2() {
    let (code, out, err) = call(&["materials", "show", "nobody/nothing"]);
    assert_eq!((code, out.is_empty()), (2, true));
    assert!(err.contains("unknown material nobody/nothing"), "{err}");
    for list in [
        &["materials"][..],
        &["materials", "show"],
        &["materials", "list", "--kind", "nonsense"],
        &["materials", "list", "--min-tg", "hot"],
        &["materials", "list", "--min-tg"],
        &["materials", "list", "--nonsense"],
        &["materials", "show", "rogers/ro4350b", "--at-ghz"],
    ] {
        let (code, out, _) = call(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
    }
}

/// A job whose core is a material of the database, which the job does not list.
const JOB: &str = r#"{
    "stackup": {"name": "rf", "layers": [
        {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
        {"kind": "core", "name": "core", "thickness_nm": 508000, "material": "rogers/ro4350b"},
        {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
    "lines": [{"name": "feed", "kind": "microstrip", "layer": "top", "width_nm": 1100000}]
}"#;

#[test]
fn stackup_takes_a_material_from_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let job = dir.path().join("job.json");
    std::fs::write(&job, JOB).unwrap();
    let path = job.to_str().unwrap();
    // without the option the material is unknown
    let (code, _, err) = call(&["stackup", path, "--freq-ghz", "10"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STACK-003"), "{err}");
    // at 10 GHz the sheet publishes Dk
    let (code, text, err) = call(&[
        "stackup",
        path,
        "--freq-ghz",
        "10",
        "--material",
        "rogers/ro4350b",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("line feed (microstrip)"), "{text}");
    // at 2 GHz the one Dk point is out of reach and nothing is extrapolated
    let (code, text, _) = call(&[
        "stackup",
        path,
        "--freq-ghz",
        "2",
        "--material",
        "rogers/ro4350b",
    ]);
    assert_eq!(code, 1);
    assert!(text.contains("PHOENIX-STACK-004"), "{text}");
}

#[test]
fn a_material_option_that_cannot_be_used_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let job = dir.path().join("job.json");
    std::fs::write(&job, JOB).unwrap();
    let path = job.to_str().unwrap();
    let (code, _, err) = call(&[
        "stackup",
        path,
        "--freq-ghz",
        "10",
        "--material",
        "nobody/nothing",
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STACK-003"), "{err}");
    let (code, _, _) = call(&["stackup", path, "--freq-ghz", "10", "--material"]);
    assert_eq!(code, 2);
}
