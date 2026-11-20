// SPDX-License-Identifier: GPL-3.0-or-later
//! `phoenix stackup --library`: the typical stack-ups that come with the program and the
//! impedance of their lines.

use eh3d_phoenix_cli::run;

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn the_list_shows_the_stackups_by_layers_and_says_they_are_not_verified() {
    let (code, out, err) = call(&["stackup", "--library", "list"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(
        out.starts_with("id layers total_mm sold_for_mm\n6l-1.6 6 1.610 1.600\n"),
        "{out}"
    );
    assert!(out.contains("\n10l-2.0 10 2.015 2.000\n"), "{out}");
    assert!(out.contains("not verified with the maker"), "{out}");
    assert!(
        out.contains("note: No 2 layer or 4 layer stackup is here"),
        "{out}"
    );
    let (_, only8, _) = call(&["stackup", "--library", "list", "--layers", "8"]);
    assert!(
        only8.contains("8l-1.6 8 1.620 1.600\n8l-2.4 8 2.410 2.400\n"),
        "{only8}"
    );
    assert!(!only8.contains("6l-"), "{only8}");
}

#[test]
fn the_list_as_json_has_the_notes_and_every_total() {
    let (code, out, _) = call(&["stackup", "--library", "list", "--json"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["verified"], false);
    assert_eq!(value["stackups"].as_array().unwrap().len(), 8);
    assert_eq!(value["stackups"][0]["id"], "6l-1.6");
    assert_eq!(value["stackups"][0]["total_nm"], 1_610_000);
    assert!(value["notes"].as_array().unwrap().len() >= 4);
}

#[test]
fn the_default_is_a_width_for_fifty_ohm_on_every_line() {
    let (code, out, err) = call(&["stackup", "--library", "6l-1.6"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    for line in ["L1", "L2", "L3", "L4", "L5", "L6"] {
        assert!(out.contains(line), "{line}: {out}");
    }
    assert!(
        out.contains("the figures are not verified with the maker"),
        "{out}"
    );
    let (code, json, _) = call(&["stackup", "--library", "6l-1.6", "--json"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let lines = value["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 6);
    for line in lines {
        let z = line["impedance_ohm"].as_f64().unwrap();
        assert!((z - 50.0).abs() < 0.05, "{line}");
    }
}

#[test]
fn a_width_gives_the_impedance_and_a_thinner_prepreg_gives_less() {
    let z = |id: &str| -> f64 {
        let (code, json, err) = call(&[
            "stackup",
            "--library",
            id,
            "--width-um",
            "200",
            "--freq-ghz",
            "1",
            "--json",
        ]);
        assert_eq!((code, err.as_str()), (0, ""), "{json}");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["lines"][0]["impedance_ohm"].as_f64().unwrap()
    };
    assert!(z("6l-2.0") < z("6l-1.6"));
}

#[test]
fn the_report_is_in_the_language_asked_for() {
    let (_, en, _) = call(&["stackup", "--library", "8l-1.6", "--lang", "en"]);
    let (_, pt, _) = call(&["stackup", "--library", "8l-1.6", "--lang", "pt"]);
    let (_, ja, _) = call(&["stackup", "--library", "8l-1.6", "--lang", "ja"]);
    assert_ne!(en, pt);
    assert_ne!(en, ja);
    assert!(!ja.is_ascii());
}

#[test]
fn the_wrong_calls_are_refused_with_the_usage() {
    for (args, needle) in [
        (vec!["stackup", "--library"], "--library needs a value"),
        (
            vec!["stackup", "--library", "nobody"],
            "no stack-up is called nobody",
        ),
        (
            vec![
                "stackup",
                "--library",
                "6l-1.6",
                "--width-um",
                "100",
                "--target-ohm",
                "50",
            ],
            "not both",
        ),
        (
            vec!["stackup", "--library", "6l-1.6", "--layers", "6"],
            "--layers is for list",
        ),
        (
            vec!["stackup", "--library", "6l-1.6", "--width-um", "x"],
            "needs a number",
        ),
        (
            vec!["stackup", "--library", "6l-1.6", "--width-um", "0"],
            "not between",
        ),
        (
            vec!["stackup", "--library", "6l-1.6", "--lang", "xx"],
            "not a language",
        ),
        (
            vec!["stackup", "--library", "list", "--target-ohm", "50"],
            "list takes only",
        ),
        (
            vec!["stackup", "--library", "6l-1.6", "--bogus"],
            "does not apply",
        ),
    ] {
        let (code, _, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains(needle), "{args:?}: {err}");
    }
    // a target nobody can reach is reported line by line with exit code 1
    let (code, out, _) = call(&["stackup", "--library", "6l-1.6", "--target-ohm", "5000"]);
    assert_eq!(code, 1, "{out}");
}
