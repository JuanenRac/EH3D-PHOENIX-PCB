// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands on a binary board of EAGLE made by hand for the tests: it is converted with its
//! report, and the commands that take a board read it and say so.

#[path = "../../eh3d-phoenix-eaglebin/tests/builder/board.rs"]
mod builder;

use std::fs;

use eh3d_phoenix_cli::run;

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
fn a_binary_board_is_converted_with_its_report_and_never_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let binary = folder.path().join("old.brd");
    let bytes = builder::BoardFile::new((4, 16)).bytes();
    fs::write(&binary, &bytes).unwrap();
    let binary = binary.to_str().unwrap();
    let saved = folder.path().join("new.brd");
    let saved = saved.to_str().unwrap();

    let (code, out, err) = call(&[
        "eagle-convert",
        "xml",
        "--out",
        saved,
        "--lang",
        "es",
        binary,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("EAGLE binary 4.16"), "{out}");
    assert!(out.contains("element: 2"), "{out}");
    assert!(out.contains("PHOENIX-EAGLEBIN-011"), "{out}");
    let xml = fs::read_to_string(saved).unwrap();
    assert!(xml.starts_with("<?xml"));

    // the output exists now and stays as it is
    let (code, _, _) = call(&["eagle-convert", "xml", "--out", saved, binary]);
    assert_eq!(code, 2);
    assert_eq!(fs::read_to_string(saved).unwrap(), xml);
    // an XML board is not a binary one
    let (code, _, err) = call(&["eagle-convert", "xml", saved]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EAGLEBIN-001"), "{err}");

    // a command that takes a board reads the binary one and says that it was converted
    let (code, _, err) = call(&["check", binary]);
    assert!(code == 0 || code == 1, "{err}");
    assert!(err.contains("is a binary board of EAGLE 4.16"), "{err}");
    let (_, _, err) = call(&["check", saved]);
    assert!(!err.contains("binary board"), "{err}");
    assert_eq!(fs::read(binary).unwrap(), bytes);
}
