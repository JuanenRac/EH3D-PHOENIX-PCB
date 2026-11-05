// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix kicad-read`: KiCad files read into the design model and summarised.

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

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

const BOARD: &str = r#"(kicad_pcb (version 20211014) (generator pcbnew)
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user))
  (net 0 "") (net 1 "SecretNet")
  (footprint "Lib:Part" (layer "F.Cu") (at 10 10)
    (fp_text reference "R1" (at 0 0) (layer "F.SilkS"))
    (fp_text value "1k" (at 0 0) (layer "F.Fab"))
    (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (net 1 "SecretNet")))
  (segment (start 10 10) (end 20 10) (width 0.25) (layer "F.Cu") (net 1))
  (target plus (at 1 1) (size 1) (width 0.1) (layer "Edge.Cuts"))
)"#;

const ROOT_SHEET: &str = r#"(kicad_sch (version 20230121) (generator eeschema) (paper "A4")
  (sheet (at 10 10) (size 20 10)
    (property "Sheet name" "Sub" (at 10 9 0))
    (property "Sheet file" "sub/child.kicad_sch" (at 10 21 0))
    (pin "IN" input (at 10 15 180)))
  (sheet (at 50 10) (size 20 10)
    (property "Sheet name" "Away" (at 50 9 0))
    (property "Sheet file" "../away.kicad_sch" (at 50 21 0))))"#;

const CHILD_SHEET: &str =
    "(kicad_sch (version 20230121) (generator eeschema) (hierarchical_label \"IN\" (at 10 15 0)))";

#[test]
fn a_board_is_read_and_summarised() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "demo.kicad_pcb", BOARD);
    let (code, text, err) = call(&["kicad-read", &path]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    for want in [
        "format: kicad",
        "kind: board",
        "version: 20211014",
        "usable: yes",
        "fidelity: partial",
        "components: 1",
        "tracks: 1",
        "nets: 1",
        "`target` elements: 1",
        "PHOENIX-KICAD-006",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
}

#[test]
fn the_sheets_below_the_folder_are_read_and_the_ones_outside_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "root.kicad_sch", ROOT_SHEET);
    write(&dir, "sub/child.kicad_sch", CHILD_SHEET);
    write(&dir, "away.kicad_sch", CHILD_SHEET);
    let (code, text, err) = call(&["kicad-read", &path]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("kind: schematic"), "{text}");
    // the root and the one sheet that lies below its folder
    assert!(text.contains("sheets: 2"), "{text}");
    // the sheet that leaves the folder is reported and never opened
    assert!(text.contains("PHOENIX-KICAD-008"), "{text}");
}

#[test]
fn a_damaged_file_exits_1_and_the_title_follows_the_language() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "cut.kicad_pcb", &BOARD[..BOARD.len() / 2]);
    let (code, text, err) = call(&["kicad-read", &path]);
    assert_eq!((code, err.is_empty()), (1, true), "{err}");
    assert!(text.contains("usable: no") && text.contains("fidelity: unusable"));
    assert!(text.contains("PHOENIX-KICAD-004") && text.contains("The KiCad file ends"));
    let (_, german, _) = call(&["kicad-read", "--lang", "de", &path]);
    assert!(german.contains("Die KiCad-Datei endet"), "{german}");
    let (_, portuguese, _) = call(&["kicad-read", "--lang", "pt", &path]);
    assert!(
        portuguese.contains("O ficheiro KiCad termina"),
        "{portuguese}"
    );
    let other = write(&dir, "plain.txt", "hello");
    let (code, text, _) = call(&["kicad-read", &other]);
    assert_eq!(code, 1);
    assert!(text.contains("PHOENIX-KICAD-003") && text.contains("kind: unknown"));
}

#[test]
fn the_redacted_form_shows_no_name_and_no_message() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "private_name.kicad_pcb", BOARD);
    let (code, text, _) = call(&["kicad-read", "--redacted", &path]);
    assert_eq!(code, 0);
    assert!(!text.contains("private_name") && !text.contains("SecretNet"));
    assert!(!text.contains("file:"));
    assert!(text.contains("PHOENIX-KICAD-006: 1"), "{text}");
}

#[test]
fn the_settings_file_changes_the_limits() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "demo.kicad_pcb", BOARD);
    let settings = write(&dir, "kicad.cfg", "max_depth = 2\n");
    let (code, text, _) = call(&["kicad-read", "--settings", &settings, &path]);
    assert_eq!(code, 1);
    assert!(text.contains("PHOENIX-KICAD-002"), "{text}");
    let bad = write(&dir, "bad.cfg", "max_depth = lots\n");
    let (code, _, err) = call(&["kicad-read", "--settings", &bad, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("max_depth"), "{err}");
}

#[test]
fn usage_errors_and_missing_files_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent.kicad_pcb");
    let missing = missing.to_str().unwrap().to_string();
    for list in [
        vec!["kicad-read"],
        vec!["kicad-read", "--nope", "x"],
        vec!["kicad-read", "--lang"],
        vec!["kicad-read", "--lang", "xx", "x"],
        vec!["kicad-read", "--settings"],
        vec!["kicad-read", "a", "b"],
        vec!["kicad-read", missing.as_str()],
    ] {
        let (code, out, err) = call(&list);
        assert_eq!(code, 2, "{list:?}");
        assert!(out.is_empty(), "{list:?}");
        assert!(err.contains("error:"), "{list:?}: {err}");
    }
}
