// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes of the language that real programs write and that the reader once refused: texts one
//! after the other in `#usage`, `f(void)`, `enum`, `numeric` arrays, `int(x)` as a conversion, a
//! walk over a single statement, a backslash before a character that is no escape, a text that
//! goes on in the next line, and the mark that ends a file on old systems. Each program is
//! written for this test.

use eh3d_phoenix_ulp::{analyse, parse, run_source, Support, UlpSettings, World};

fn out(text: &str) -> String {
    match run_source(
        text,
        &UlpSettings::default(),
        &Support::shipped(),
        World::default(),
    ) {
        Ok(o) => o.output,
        Err(e) => panic!("{e}\n{text}"),
    }
}

fn clean(text: &str) {
    let parsed = parse(text, &UlpSettings::default()).unwrap();
    assert!(
        parsed.problems.list.is_empty(),
        "{:?}\n{text}",
        parsed.problems.list
    );
}

#[test]
fn usage_texts_one_after_the_other_are_one_text() {
    clean("#usage \"en: first\\n\"\n       \"de: second\\n\",\n       \"es: third\"\n#require 6.0\nint a = 1;\n");
}

#[test]
fn void_parameters_enums_and_numeric_arrays() {
    assert_eq!(
        out("enum { RED, GREEN = 5, BLUE };\nint two(void) { return 2; }\nnumeric string list[];\nprintf(\"%d %d %d %d\\n\", RED, GREEN, BLUE, two());"),
        "0 5 6 2\n"
    );
    assert_eq!(
        out("enum Mode { A = 1, B };\nprintf(\"%d\\n\", A + B);"),
        "3\n"
    );
}

#[test]
fn conversions_written_as_calls() {
    assert_eq!(
        out("printf(\"%d %f %s %d\\n\", int(7.9), real(3), string(42), char(65));"),
        "7 3.000000 42 65\n"
    );
}

#[test]
fn texts_with_a_backslash_before_a_plain_character_and_continued_lines() {
    assert_eq!(out("printf(\"%s\\n\", \"a\\%b\\.c\");"), "a%b.c\n");
    assert_eq!(out("printf(\"one \\\ntwo\\n\");"), "one two\n");
    assert_eq!(out("printf(\"x\\n\");\n\u{1a}"), "x\n");
}

#[test]
fn a_walk_may_run_a_single_statement() {
    let text = "int n = 0;\nschematic(S) S.parts(P) n++;\nprintf(\"%d\\n\", n);";
    let parsed = parse(text, &UlpSettings::default()).unwrap();
    assert!(
        parsed.problems.list.is_empty(),
        "{:?}",
        parsed.problems.list
    );
    let report = analyse("t.ulp", text, &UlpSettings::default(), &Support::shipped());
    assert!(report.problems.is_empty());
}

#[test]
fn a_layout_without_parentheses_holds_one_control() {
    assert_eq!(
        out("int r = dlgDialog(\"t\") { dlgHBoxLayout dlgLabel(\"x\"); dlgPushButton(\"+OK\") dlgAccept(); };\nprintf(\"%d\\n\", r);"),
        "1\n"
    );
}

#[test]
fn sort_lookup_character_tests_constants_and_file_dialogs() {
    // texts are ordered by their bytes, a numeric array as numbers, and the other arrays follow
    assert_eq!(
        out("string k[] = { \"b\", \"c\", \"a\" }; int v[] = { 1, 2, 3 };\nsort(3, k, v);\nprintf(\"%s%s%s %d%d%d\\n\", k[0], k[1], k[2], v[0], v[1], v[2]);"),
        "abc 312\n"
    );
    assert_eq!(
        out("numeric string n[] = { \"10\", \"9\", \"100\" };\nsort(3, n);\nprintf(\"%s %s %s\\n\", n[0], n[1], n[2]);"),
        "9 10 100\n"
    );
    assert_eq!(
        out("string t[] = { \"b\", \"a\" };\nsort(2, t);\nstring u[] = { \"x\", \"y\" };\nprintf(\"%s%s\\n\", t[0], t[1]);"),
        "ab\n"
    );
    assert_eq!(
        out("string l[] = { \"k1\\tone\\ttwo\", \"k2\\tthree\\tfour\" };\nprintf(\"%s|%s|%s\\n\", lookup(l, \"k2\", 2), lookup(l, \"k1\", 1), lookup(l, \"zz\", 1));"),
        "four|one|\n"
    );
    assert_eq!(
        out("printf(\"%d%d%d%d %d %d\\n\", isdigit('7'), isalpha('7'), isspace(' '), isupper('a'), toupper('a'), tolower('Z'));"),
        "1010 65 122\n"
    );
    assert_eq!(
        out("printf(\"%d %d %d\\n\", EXIT_SUCCESS, EXIT_FAILURE, EAGLE_VERSION >= 6);"),
        "0 1 1\n"
    );
    assert_eq!(
        out("string f = dlgFileOpen(\"Open\", \"\", \"*.txt\");\nprintf(\"[%s]\\n\", f);"),
        "[]\n"
    );
}
