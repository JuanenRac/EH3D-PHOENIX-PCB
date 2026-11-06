// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `eagle-check` and `eagle-convert`, and `--rules-dru` and `--variant` of `check`,
//! `bom` and `pnp`, on the small files of `tests/corpus/eagle` and of the deep reading.

use std::path::{Path, PathBuf};

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

fn corpus(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/eagle")
        .join(name)
        .to_str()
        .unwrap()
        .to_owned()
}

fn data(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../eh3d-phoenix-eagle-deep/tests/data")
        .join(name)
        .to_str()
        .unwrap()
        .to_owned()
}

fn temp(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn eagle_check_reports_the_rewrite_and_what_the_model_would_lose() {
    let file = corpus("variants_board.brd");
    let (code, out, err) = call(&[
        "eagle-check",
        "--layers",
        corpus("nothing.cfg").as_str(),
        file.as_str(),
    ]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("PHOENIX-EAGLEX-024"), "{err}");

    let dir = tempfile::tempdir().unwrap();
    let layers = temp(dir.path(), "layers.cfg", "layer.100 = silk_top\n");
    let (code, out, err) = call(&["eagle-check", "--layers", layers.as_str(), file.as_str()]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("board (version 9.6.2)"), "{out}");
    assert!(out.contains("rewrite of the tree: identical"), "{out}");
    assert!(out.contains("variants: FULL, LITE"), "{out}");
    assert!(out.contains("PHOENIX-EAGLEX-019"), "{out}");
    assert!(out.contains("[variant]"), "{out}");

    let (code, out, _) = call(&[
        "eagle-check",
        "--json",
        "--layers",
        layers.as_str(),
        file.as_str(),
    ]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "board");
    assert_eq!(value["tree_lossless"], true);

    let (_, out, _) = call(&[
        "eagle-check",
        "--lang",
        "es",
        "--layers",
        layers.as_str(),
        file.as_str(),
    ]);
    assert!(
        out.contains("El modelo de diseño deja fuera el elemento"),
        "{out}"
    );
}

#[test]
fn eagle_check_refuses_what_is_not_an_eagle_file_with_a_code() {
    let (code, _, err) = call(&["eagle-check", corpus("not_eagle.brd").as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-021"), "{err}");
    let (code, _, err) = call(&["eagle-check", "--nothing", corpus("not_eagle.brd").as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-022"), "{err}");
    let (code, _, err) = call(&["eagle-check"]);
    assert_eq!(code, 2);
    assert!(err.contains("phoenix eagle-check"), "{err}");
}

#[test]
fn a_design_rule_file_becomes_a_rule_file_and_back() {
    let dru = data("two_layer.dru");
    let (code, out, err) = call(&["eagle-convert", "dru-to-rules", dru.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"min_track_width_nm\": 254000"), "{out}");
    assert!(
        out.contains("\"name\": \"The <b>design rules</b> for a two layer board.\""),
        "{out}"
    );
    assert!(err.contains("PHOENIX-EAGLEX-004"), "{err}");
    assert!(err.contains("PHOENIX-EAGLEX-003"), "{err}");

    let dir = tempfile::tempdir().unwrap();
    let rules = temp(dir.path(), "rules.json", &out);
    let target = dir.path().join("back.dru").to_str().unwrap().to_owned();
    let (code, out, _) = call(&[
        "eagle-convert",
        "rules-to-dru",
        "--base",
        dru.as_str(),
        "--out",
        target.as_str(),
        rules.as_str(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("written: "), "{out}");
    let text = std::fs::read_to_string(&target).unwrap();
    assert!(text.contains("msWidth = 0.254mm"), "{text}");
    assert!(text.contains("layerSetup = (1*16)"), "{text}");
    // an existing file is never replaced
    let (code, _, err) = call(&[
        "eagle-convert",
        "rules-to-dru",
        "--out",
        target.as_str(),
        rules.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-023"), "{err}");
    assert_eq!(std::fs::read_to_string(target.as_str()).unwrap(), text);
}

#[test]
fn a_cam_job_becomes_a_profile_that_the_export_accepts() {
    let job = data("job_full.cam");
    let (code, out, err) = call(&[
        "eagle-convert",
        "cam-to-profile",
        "--name",
        "mine",
        job.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["format"], "phoenix-export/1");
    assert_eq!(value["profiles"][0]["name"], "mine");
    assert_eq!(value["profiles"][0]["drill"]["fixed_decimal_digits"], 4);
    assert!(err.contains("PHOENIX-EAGLEX-010"), "{err}");
    assert!(err.contains("PHOENIX-EAGLEX-009"), "{err}");

    let dir = tempfile::tempdir().unwrap();
    let profiles = temp(dir.path(), "profiles.json", &out);
    let board = corpus("two_resistors.brd");
    let target = dir.path().join("fab").to_str().unwrap().to_owned();
    let (code, out, err) = call(&[
        "export",
        "--profile",
        "mine",
        "--profiles",
        profiles.as_str(),
        "--out",
        target.as_str(),
        board.as_str(),
    ]);
    // 2 would mean that the profile or the board was refused
    assert!(code == 0 || code == 1, "{code} {out}{err}");
}

#[test]
fn the_layers_and_the_variants_are_listed() {
    let (code, out, _) = call(&["eagle-convert", "layers"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("1\tTop\tcopper\n"), "{out}");
    assert!(out.contains("20\tDimension\toutline\n"));
    assert!(out.contains("99\tSpiceOrder\tspice_order\n"));

    let (code, out, err) = call(&[
        "eagle-convert",
        "layers",
        corpus("user_layers.brd").as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("100\tMilling2\tuser\n"), "{out}");
    assert!(err.contains("PHOENIX-EAGLEX-014"), "{err}");
    assert!(err.contains("PHOENIX-EAGLEX-015"), "{err}");

    let (code, out, _) = call(&[
        "eagle-convert",
        "variants",
        "--variant",
        "LITE",
        corpus("variants_board.brd").as_str(),
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("variants: FULL, LITE\n"), "{out}");
    assert!(out.contains("  R1\tnot fitted\n"), "{out}");
    assert!(out.contains("  R2\tvalue 10k (was 4k7)\n"), "{out}");
    let (code, _, err) = call(&[
        "eagle-convert",
        "variants",
        "--variant",
        "NOPE",
        corpus("variants_board.brd").as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-011"), "{err}");
    let (code, _, err) = call(&["eagle-convert", "nothing", "x"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-022"), "{err}");
}

#[test]
fn check_takes_the_limits_from_an_eagle_design_rule_file() {
    let dir = tempfile::tempdir().unwrap();
    let board = corpus("two_resistors.brd");
    let wide = temp(dir.path(), "wide.dru", "msWidth = 0.3mm\nstrange = 1\n");
    let (code, out, err) = call(&["check", "--rules-dru", wide.as_str(), board.as_str()]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("PHOENIX-CHECK-001"), "{out}");
    assert!(err.contains("PHOENIX-EAGLEX-002"), "{err}");
    let thin = temp(dir.path(), "thin.dru", "msWidth = 0.1mm\n");
    let (code, out, err) = call(&["check", "--rules-dru", thin.as_str(), board.as_str()]);
    assert_eq!(code, 0, "{out}{err}");
    let rules = temp(
        dir.path(),
        "r.json",
        r#"{"format":"phoenix-rules/1","rules":{}}"#,
    );
    let (code, _, err) = call(&[
        "check",
        "--rules-dru",
        thin.as_str(),
        "--rules",
        rules.as_str(),
        board.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("not both"), "{err}");
    let bad = temp(dir.path(), "bad.dru", "msWidth\n");
    let (code, _, err) = call(&["check", "--rules-dru", bad.as_str(), board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-001"), "{err}");
    let (code, _, err) = call(&[
        "connectivity",
        "--rules-dru",
        thin.as_str(),
        board.as_str(),
        board.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("not to connectivity"), "{err}");
}

#[test]
fn the_bill_and_the_placement_follow_a_variant() {
    let board = corpus("variants_board.brd");
    let (code, full, err) = call(&["bom", board.as_str()]);
    assert!(code == 0 || code == 1, "{code} {err}");
    assert!(full.contains("R1"), "{full}");
    let (code, lite, err) = call(&["bom", "--variant", "LITE", board.as_str()]);
    assert!(code == 0 || code == 1, "{code} {err}");
    assert!(
        lite.starts_with("variant: LITE, left out: 1, values changed: 1\n"),
        "{lite}"
    );
    assert!(!lite.contains("R1"), "{lite}");
    assert!(lite.contains("R2"), "{lite}");
    let (code, _, err) = call(&["bom", "--variant", "NOPE", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEX-011"), "{err}");
    let (code, _, err) = call(&["bom", "--variant-file", board.as_str(), board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("needs --variant"), "{err}");

    let (code, plain, err) = call(&["pnp", board.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(plain.contains("top: 3 part(s)"), "{plain}");
    let (code, lite, err) = call(&["pnp", "--variant", "LITE", board.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(lite.contains("variant: LITE, not fitted: 1\n"), "{lite}");
    assert!(lite.contains("top: 2 part(s)"), "{lite}");
}
