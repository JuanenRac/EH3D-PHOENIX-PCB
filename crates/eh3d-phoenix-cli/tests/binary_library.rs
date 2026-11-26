// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `libedit` on a binary library of EAGLE made by hand for the tests: it is read,
//! its report is printed, it can be saved as an XML library and compared with it.

#[path = "../../eh3d-phoenix-eaglebin/tests/builder/mod.rs"]
#[allow(dead_code)]
mod builder;

use std::fs;

use eh3d_phoenix_cli::run;
use serde_json::Value;

fn call(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn a_binary_library_is_read_reported_and_saved_as_xml() {
    let folder = tempfile::tempdir().unwrap();
    let binary = folder.path().join("old.lbr");
    fs::write(&binary, builder::sample().bytes()).unwrap();
    let binary = binary.to_str().unwrap();
    let saved = folder.path().join("new.lbr");
    let saved = saved.to_str().unwrap();

    let (code, out, err) = call(&["libedit", "--out", saved, binary]);
    assert!(code == 0 || code == 1, "{err}");
    assert!(
        out.contains("2 footprints, 1 symbols, 1 device sets"),
        "{out}"
    );
    assert!(out.contains("EAGLE binary 4.9"), "{out}");
    assert!(out.contains("PHOENIX-EAGLEBIN-011"), "{out}");
    assert!(fs::read_to_string(saved).unwrap().starts_with("<?xml"));

    // the saved library is the same library, and the report of the comparison goes aside
    let (code, _, err) = call(&["libedit", "--compare", saved, binary]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("EAGLE binary 4.9"), "{err}");

    let (_, out, _) = call(&["libedit", "--json", binary]);
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["binary_read"]["version"], "4.9");
    assert_eq!(value["binary_read"]["read"]["footprint"], 2);
    let (_, out, _) = call(&["libedit", "--json", saved]);
    let value: Value = serde_json::from_str(&out).unwrap();
    assert!(value["binary_read"].is_null());
}

#[test]
fn a_damaged_binary_library_is_refused_with_its_code() {
    let folder = tempfile::tempdir().unwrap();
    let binary = folder.path().join("cut.lbr");
    let bytes = builder::sample().bytes();
    fs::write(&binary, &bytes[..200]).unwrap();
    let (code, _, err) = call(&["libedit", binary.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEBIN-004"), "{err}");
}
