// SPDX-License-Identifier: GPL-3.0-or-later
//! The option `--limits FILE`: the limits of the readers of EAGLE files and of the checks of a
//! board, which every command accepts and which change nothing when they are the defaults.

use std::path::PathBuf;

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

fn repo_file(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name)
        .to_str()
        .unwrap()
        .to_owned()
}

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

/// A board of 20 by 10 mm with three squares of 1 mm in the outline layer, each of four wires.
fn board_with_three_openings() -> String {
    let square = |x: i32| {
        let (y, x2, y2) = (2, x + 1, 3);
        format!(
            "<wire x1=\"{x}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{x2}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y2}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{x2}\" y1=\"{y2}\" x2=\"{x}\" y2=\"{y2}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{x}\" y1=\"{y2}\" x2=\"{x}\" y2=\"{y}\" width=\"0.1\" layer=\"20\"/>"
        )
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<eagle version=\"9.6.2\"><drawing><layers>\
         <layer number=\"1\" name=\"Top\" color=\"4\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\
         <layer number=\"16\" name=\"Bottom\" color=\"1\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\
         <layer number=\"20\" name=\"Dimension\" color=\"15\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\
         </layers><board><plain>\
         <wire x1=\"0\" y1=\"0\" x2=\"20\" y2=\"0\" width=\"0.1\" layer=\"20\"/>\
         <wire x1=\"20\" y1=\"0\" x2=\"20\" y2=\"10\" width=\"0.1\" layer=\"20\"/>\
         <wire x1=\"20\" y1=\"10\" x2=\"0\" y2=\"10\" width=\"0.1\" layer=\"20\"/>\
         <wire x1=\"0\" y1=\"10\" x2=\"0\" y2=\"0\" width=\"0.1\" layer=\"20\"/>\
         {}{}{}</plain></board></drawing></eagle>\n",
        square(2),
        square(5),
        square(8)
    )
}

const RULES: &str = r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":100000}}"#;

#[test]
fn a_limit_of_the_reader_changes_what_check_reads_and_without_it_nothing_changes() {
    let dir = tempfile::tempdir().unwrap();
    let brd = write(&dir, "b.brd", &board_with_three_openings());
    let rules = write(&dir, "r.json", RULES);
    let tight = write(&dir, "tight.cfg", "max_outline_rings = 2\n");
    let (code, out, err) = call(&["check", "--rules", &rules, &brd]);
    let (tight_code, tight_out, tight_err) =
        call(&["check", "--limits", &tight, "--rules", &rules, &brd]);
    assert!(
        code < 2 && tight_code == code,
        "{out}\n{err}\n{tight_err}"
    );
    // the report is the same, the note that parts were not read is not
    assert_eq!(out, tight_out);
    assert!(!err.contains("note: "), "{err}");
    assert!(tight_err.contains("note: "), "{tight_err}");
    assert_ne!(err, tight_err);
    // the option may stand anywhere and means the same
    let (_, _, again) = call(&["check", "--rules", &rules, &brd, "--limits", &tight]);
    assert_eq!(again, tight_err);
}

#[test]
fn the_shipped_limit_files_are_the_defaults_and_change_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let brd = write(&dir, "b.brd", &board_with_three_openings());
    let rules = write(&dir, "r.json", RULES);
    let plain = call(&["check", "--rules", &rules, &brd]);
    for file in ["config/eagle_board.cfg", "config/board.cfg"] {
        let path = repo_file(file);
        let with = call(&["check", "--limits", &path, "--rules", &rules, &brd]);
        assert_eq!(with, plain, "{file}");
    }
    let corpus = repo_file("tests/corpus");
    let plain = call(&["corpus", "run", "--dir", &corpus]);
    assert_eq!(plain.0, 0, "{}", plain.1);
    let with = call(&[
        "--limits",
        &repo_file("config/board.cfg"),
        "corpus",
        "run",
        "--dir",
        &corpus,
    ]);
    assert_eq!(with, plain);
}

#[test]
fn an_unusable_limits_file_exits_2_and_names_the_problem() {
    let dir = tempfile::tempdir().unwrap();
    let brd = write(&dir, "b.brd", &board_with_three_openings());
    let missing = dir.path().join("none.cfg");
    let (code, out, err) = call(&["check", "--limits", missing.to_str().unwrap(), &brd]);
    assert_eq!(code, 2, "{out}");
    assert!(err.contains("cannot read the limits"), "{err}");
    let unknown = write(&dir, "unknown.cfg", "max_bus_checks = 5\nsurprise = 1\n");
    let (code, _, err) = call(&["check", "--limits", &unknown, &brd]);
    assert_eq!(code, 2);
    assert!(err.contains("line 2") && err.contains("surprise"), "{err}");
    let zero = write(&dir, "zero.cfg", "max_polygon_points = 2\n");
    let (code, _, err) = call(&["export", "--limits", &zero, &brd]);
    assert_eq!(code, 2);
    assert!(err.contains("line 1"), "{err}");
    let (code, _, err) = call(&["check", &brd, "--limits"]);
    assert_eq!(code, 2);
    assert!(err.contains("--limits needs a file"), "{err}");
}
