// SPDX-License-Identifier: GPL-3.0-or-later
//! Goldens of the output of commands: the text, the JSON, the CSV and the page of HTML that
//! `check` gives on a board of the corpus, compared with files in `tests/golden` that were worked
//! out by hand from the board, the rule file and the tables of titles.
//!
//! The comparison is made on the text with the line ends of Unix, and the identity of an object
//! (the word after `track ` or `via `) is replaced by `<ID>` on both sides, so that the files hold
//! no value that depends on the computer, no absolute path and no date. When the output of a
//! command is changed on purpose, run the tests with the variable `PHOENIX_BLESS_GOLDENS=1` to
//! write the new files, and read the difference before it is committed.

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_cli::run;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden_path(name: &str) -> PathBuf {
    root().join("tests/golden").join(name)
}

/// The text with the line ends of Unix and the identities of tracks and vias masked.
fn normalise(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        out.push(bytes[i]);
        i += 1;
        if out.ends_with(b"track ") || out.ends_with(b"via ") {
            let mut end = i;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
                end += 1;
            }
            if end - i >= 8 && bytes[i] != b'<' {
                out.extend_from_slice(b"<ID>");
                i = end;
            }
        }
    }
    String::from_utf8(out).expect("only ASCII runs were replaced")
}

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

/// Compares `actual` with the golden file `name`, or writes the file when blessing.
fn matches_golden(name: &str, actual: &str) {
    let actual = normalise(actual);
    if std::env::var_os("PHOENIX_BLESS_GOLDENS").is_some() {
        fs::write(golden_path(name), &actual).unwrap();
        return;
    }
    let wanted = normalise(&fs::read_to_string(golden_path(name)).unwrap());
    assert_eq!(actual, wanted, "the golden {name}");
}

fn board() -> String {
    root()
        .join("tests/corpus/eagle/two_resistors.brd")
        .to_string_lossy()
        .into_owned()
}

fn rules() -> String {
    golden_path("narrow_track_rules.json")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_text_of_check_is_the_golden_in_english_and_in_portuguese() {
    let (board, rules) = (board(), rules());
    let (code, out, err) = command(&["check", "--rules", &rules, &board]);
    assert_eq!(code, 1, "{out}{err}");
    matches_golden("check_en.txt", &out);
    let (code, out, _) = command(&["check", "--rules", &rules, "--lang", "pt", &board]);
    assert_eq!(code, 1);
    matches_golden("check_pt.txt", &out);
}

#[test]
fn the_json_of_check_is_the_golden() {
    let (board, rules) = (board(), rules());
    let (code, out, err) = command(&["check", "--rules", &rules, "--json", &board]);
    assert_eq!(code, 1, "{out}{err}");
    matches_golden("check.json", &out);
}

#[test]
fn the_csv_and_the_html_of_check_are_the_goldens() {
    let (board, rules) = (board(), rules());
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("report.csv");
    let html = dir.path().join("report.html");
    let csv_de = dir.path().join("report_de.csv");
    let (code, _, err) = command(&[
        "check",
        "--rules",
        &rules,
        "--csv",
        csv.to_str().unwrap(),
        "--html",
        html.to_str().unwrap(),
        &board,
    ]);
    assert_eq!(code, 1, "{err}");
    matches_golden("check.csv", &fs::read_to_string(&csv).unwrap());
    matches_golden("check.html", &fs::read_to_string(&html).unwrap());
    let (code, _, err) = command(&[
        "check",
        "--rules",
        &rules,
        "--csv",
        csv_de.to_str().unwrap(),
        "--lang",
        "de",
        &board,
    ]);
    assert_eq!(code, 1, "{err}");
    matches_golden("check_de.csv", &fs::read_to_string(&csv_de).unwrap());
}

#[test]
fn the_goldens_hold_no_absolute_path_no_date_and_no_carriage_return() {
    let mut seen = 0;
    for entry in fs::read_dir(root().join("tests/golden")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        seen += 1;
        assert!(!text.contains('\r'), "{name} holds a carriage return");
        for bad in [":\\", "/home/", "/Users/", "/tmp/", "AppData"] {
            assert!(!text.contains(bad), "{name} holds {bad}");
        }
        // a date: four digits, a hyphen, two digits, a hyphen
        let b = text.as_bytes();
        let dated = b.windows(8).any(|w| {
            w[..4].iter().all(u8::is_ascii_digit)
                && w[4] == b'-'
                && w[5..7].iter().all(u8::is_ascii_digit)
                && w[7] == b'-'
        });
        assert!(!dated, "{name} holds a date");
    }
    assert!(seen >= 7, "{seen} golden files");
}

#[test]
fn the_mask_replaces_the_identity_of_an_object_and_nothing_else() {
    assert_eq!(
        normalise("track 0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b | at\r\n"),
        "track <ID> | at\n"
    );
    assert_eq!(normalise("via abcdef0123456789,"), "via <ID>,");
    // a rule name, a short word and an identity already masked stay as they are
    assert_eq!(
        normalise("track_width tracks 12 track <ID>"),
        "track_width tracks 12 track <ID>"
    );
    assert_eq!(normalise("via 1 and track ab"), "via 1 and track ab");
}
