// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `libedit` and `footprint` on libraries written by the tests: a dry run, a new
//! output file with the copy of the original, the refusals and the JSON report.

use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;

const LIBRARY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<settings/>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="21" name="tPlace" color="7" fill="1" visible="yes" active="yes"/>
</layers>
<library>
<packages>
<package name="R0603">
<smd name="1" x="-0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<text x="0" y="1.2" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-1.2" size="1" layer="27">&gt;VALUE</text>
</package>
</packages>
<symbols>
<symbol name="R">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
</symbol>
</symbols>
<devicesets>
<deviceset name="R" prefix="R">
<gates>
<gate name="G$1" symbol="R" x="0" y="0"/>
</gates>
<devices>
<device name="0603" package="R0603">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
</device>
</devices>
</deviceset>
</devicesets>
</library>
</drawing>
</eagle>
"#;

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

fn path_of(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

fn setup() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let library = path_of(dir.path(), "in.lbr");
    fs::write(&library, LIBRARY).unwrap();
    (dir, library)
}

#[test]
fn a_library_is_checked_and_nothing_is_written_without_out() {
    let (dir, library) = setup();
    let (code, out, err) = call(&["libedit", &library]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("1 footprints, 1 symbols, 1 device sets"),
        "{out}"
    );
    assert!(out.contains("0 errors, 0 warnings"), "{out}");
    assert!(out.contains("dry run: nothing was written"));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn operations_make_a_new_library_with_a_copy_of_the_original() {
    let (dir, library) = setup();
    let ops = path_of(dir.path(), "ops.json");
    fs::write(
        &ops,
        r#"[
          {"op":"create_package","name":"HDR1X2"},
          {"op":"add_item","target":{"kind":"package","name":"HDR1X2"},
           "item":{"type":"pad","name":"1","drill":1000000,"diameter":1700000,"shape":"square"}},
          {"op":"add_item","target":{"kind":"package","name":"HDR1X2"},
           "item":{"type":"pad","name":"2","x":2540000,"drill":1000000,"diameter":1700000}}
        ]"#,
    )
    .unwrap();
    let out_path = path_of(dir.path(), "out.lbr");
    let (code, out, err) = call(&["libedit", "--apply", &ops, "--out", &out_path, &library]);
    assert_eq!(code, 0, "{err}\n{out}");
    assert!(out.contains("operation 3: add pad to HDR1X2"), "{out}");
    assert!(out.contains("the file was read back and is the same library"));
    let written = fs::read_to_string(&out_path).unwrap();
    assert!(written.contains("<package name=\"HDR1X2\">"));
    assert!(written.contains("shape=\"square\""));
    // the original is as it was, and a copy of it is next to the new file
    assert_eq!(fs::read_to_string(&library).unwrap(), LIBRARY);
    let backups: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("in.backup-"))
        .collect();
    assert_eq!(backups.len(), 1);
    // a second run does not replace the file that exists
    let (code, _, err) = call(&["libedit", "--apply", &ops, "--out", &out_path, &library]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-025"), "{err}");
}

#[test]
fn the_input_is_never_written_over() {
    let (_dir, library) = setup();
    let (code, _, err) = call(&["libedit", "--out", &library, &library]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-025"), "{err}");
    assert_eq!(fs::read_to_string(&library).unwrap(), LIBRARY);
}

#[test]
fn an_operation_that_is_refused_names_its_number_and_changes_nothing() {
    let (dir, library) = setup();
    let ops = path_of(dir.path(), "ops.json");
    fs::write(
        &ops,
        r#"[{"op":"create_package","name":"A"},{"op":"create_package","name":"R0603"}]"#,
    )
    .unwrap();
    let out_path = path_of(dir.path(), "out.lbr");
    let (code, _, err) = call(&["libedit", "--apply", &ops, "--out", &out_path, &library]);
    assert_eq!(code, 2);
    assert!(err.contains("operation 2"), "{err}");
    assert!(err.contains("PHOENIX-LIB-022"), "{err}");
    assert!(!Path::new(&out_path).exists());
}

#[test]
fn findings_with_an_error_exit_1_and_the_language_is_chosen() {
    let (dir, library) = setup();
    let ops = path_of(dir.path(), "ops.json");
    fs::write(
        &ops,
        r#"[{"op":"add_item","target":{"kind":"package","name":"R0603"},
            "item":{"type":"smd","name":"1","dx":500000,"dy":500000}}]"#,
    )
    .unwrap();
    let (code, out, _) = call(&["libedit", "--apply", &ops, "--lang", "es", &library]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-LIB-003"), "{out}");
    assert!(
        out.contains("Pads de una huella con el mismo nombre"),
        "{out}"
    );
}

#[test]
fn the_json_report_holds_the_operations_and_the_check() {
    let (dir, library) = setup();
    let ops = path_of(dir.path(), "ops.json");
    fs::write(
        &ops,
        r#"{"operations":[{"op":"create_symbol","name":"NEW"}]}"#,
    )
    .unwrap();
    let (code, out, err) = call(&["libedit", "--json", "--apply", &ops, &library]);
    assert_eq!(code, 0, "{err}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["operations"][0], "create symbol NEW");
    assert_eq!(json["check"]["symbols"], 2);
    assert!(json["written"].is_null());
}

#[test]
fn a_plan_of_the_sanitizer_is_shown_and_never_applied() {
    let (dir, library) = setup();
    let plan = path_of(dir.path(), "plan.json");
    fs::write(
        &plan,
        r#"{"format":"phoenix-sanitize-plan/1","groups":[
          {"id":"P001","kind":"package","keep":{"library":"master.lbr","name":"R0603_M"},
           "replace":[{"library":"sub/in.lbr","name":"R0603","score":100,"exact":true,"turned":false}],
           "review":[]}]}"#,
    )
    .unwrap();
    let (code, out, err) = call(&["libedit", "--plan", &plan, &library]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("1 steps for this library, none applied"),
        "{out}"
    );
    assert!(
        out.contains("package R0603 -> R0603_M of master.lbr"),
        "{out}"
    );
}

#[test]
fn bad_arguments_and_files_exit_2() {
    let (dir, library) = setup();
    for list in [
        vec!["libedit"],
        vec!["libedit", "--nothing", &library],
        vec!["libedit", "--apply"],
        vec!["libedit", "--lang", "xx", &library],
        vec!["libedit", "/no/such/file.lbr"],
        vec!["libedit", &library, &library],
    ] {
        let (code, _, err) = call(&list);
        assert_eq!(code, 2, "{list:?}: {err}");
        assert!(err.contains("PHOENIX-LIB-0"), "{list:?}: {err}");
    }
    let board = path_of(dir.path(), "board.brd");
    fs::write(
        &board,
        "<?xml version=\"1.0\"?><eagle version=\"9.6.2\"><drawing><board/></drawing></eagle>",
    )
    .unwrap();
    let (code, _, err) = call(&["libedit", &board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-020"), "{err}");
}

#[test]
fn a_footprint_is_made_from_parameters_and_written_only_on_request() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, err) = call(&["footprint", "soic", "--param", "pins=8"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("footprint SOIC8_N"), "{out}");
    assert!(out.contains("prototype"), "{out}");
    assert!(out.contains("dry run: nothing was written"));
    let target = path_of(dir.path(), "soic.lbr");
    let (code, out, err) = call(&["footprint", "soic", "--param", "pins=8", "--out", &target]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("written:"), "{out}");
    let (code, out, _) = call(&["libedit", &target]);
    assert_eq!(code, 0);
    assert!(out.contains("1 footprints"), "{out}");
    // a library to start from is copied, the name of the new footprint must be free and a file
    // that exists stays as it is
    let first = path_of(dir.path(), "first.lbr");
    let (code, _, err) = call(&["footprint", "chip", "--param", "size=0603", "--out", &first]);
    assert_eq!(code, 0, "{err}");
    let before = fs::read(&first).unwrap();
    let second = path_of(dir.path(), "second.lbr");
    let (code, _, err) = call(&[
        "footprint",
        "chip",
        "--param",
        "size=0603",
        "--into",
        &first,
        "--out",
        &second,
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("PHOENIX-LIB-022"), "{err}");
    let (code, _, err) = call(&[
        "footprint",
        "chip",
        "--param",
        "size=0805",
        "--into",
        &first,
        "--out",
        &second,
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(fs::read(&first).unwrap(), before);
    let (code, out, _) = call(&["libedit", &second]);
    assert_eq!(code, 0);
    assert!(out.contains("2 footprints"), "{out}");
    let (code, _, err) = call(&[
        "footprint",
        "chip",
        "--param",
        "size=0402",
        "--out",
        &second,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-025"), "{err}");
}

#[test]
fn a_footprint_that_cannot_be_made_exits_2_and_the_json_lists_the_pads() {
    let (code, _, err) = call(&["footprint", "soic", "--param", "pins=7"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-028"), "{err}");
    let (code, _, err) = call(&["footprint", "capacitor"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-028"), "{err}");
    let (code, _, err) = call(&["footprint"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-031"), "{err}");
    let (code, _, err) = call(&["footprint", "chip", "--param", "size"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-LIB-031"), "{err}");
    let (code, out, _) = call(&["footprint", "--json", "chip", "--param", "size=0603"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["name"], "CHIP_0603");
    assert_eq!(json["pads"].as_array().unwrap().len(), 2);
    assert_eq!(json["pads"][0]["type"], "smd");
    assert!(json["findings"].as_array().unwrap().is_empty());
}
