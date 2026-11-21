// SPDX-License-Identifier: GPL-3.0-or-later
//! The files of the one allowed folder, the arguments, the clock and the dialogs of the evaluator,
//! on programs written for these tests. The folder is a fresh temporary one in every test.

use eh3d_phoenix_ulp::codes::{
    RUN_ARGUMENTS, RUN_MEMORY_LIMIT, RUN_NOT_ALLOWED, RUN_OUTPUT_LIMIT, RUN_TYPE,
};
use eh3d_phoenix_ulp::{run_source_with, Outcome, RunEnv, Support, UlpSettings, World};

fn settings_for(dir: &std::path::Path) -> UlpSettings {
    UlpSettings {
        file_dir: dir.to_str().unwrap().to_owned(),
        ..UlpSettings::default()
    }
}

fn run(
    text: &str,
    settings: &UlpSettings,
    env: &RunEnv,
) -> Result<Outcome, eh3d_phoenix_ulp::Diag> {
    run_source_with(text, settings, &Support::shipped(), World::default(), env)
}

fn out(text: &str, settings: &UlpSettings, env: &RunEnv) -> String {
    match run(text, settings, env) {
        Ok(o) => o.output,
        Err(e) => panic!("{e}\n{text}"),
    }
}

fn code(text: &str, settings: &UlpSettings) -> &'static str {
    match run(text, settings, &RunEnv::default()) {
        Ok(o) => panic!("it ran: {:?}\n{text}", o.output),
        Err(e) => e.code,
    }
}

#[test]
fn output_writes_a_file_inside_the_folder_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let s = settings_for(dir.path());
    let o = run(
        "output(\"a.txt\") { printf(\"one\\n\"); printf(\"two %d\\n\", 2); } printf(\"shown\\n\");",
        &s,
        &RunEnv::default(),
    )
    .unwrap();
    assert_eq!(o.output, "shown\n");
    assert_eq!(o.files, vec!["a.txt".to_owned()]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "one\ntwo 2\n"
    );
    // append keeps what was there; write replaces it
    run(
        "output(\"a.txt\", \"a\") { printf(\"three\\n\"); }",
        &s,
        &RunEnv::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "one\ntwo 2\nthree\n"
    );
    run(
        "output(\"a.txt\", \"w\") { printf(\"new\"); }",
        &s,
        &RunEnv::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "new"
    );
}

#[test]
fn files_are_refused_without_a_folder_and_outside_it() {
    let none = UlpSettings::default();
    assert_eq!(
        code("output(\"a.txt\") { printf(\"x\"); }", &none),
        RUN_NOT_ALLOWED
    );
    let dir = tempfile::tempdir().unwrap();
    let s = settings_for(dir.path());
    for name in ["../x.txt", "/tmp/x.txt", "a/../../x.txt", ""] {
        let text = format!("output(\"{name}\") {{ printf(\"x\"); }}");
        let c = code(&text, &s);
        assert!(c == RUN_NOT_ALLOWED || c == RUN_ARGUMENTS, "{name}: {c}");
    }
    // a folder that does not exist is refused, and nothing is created
    assert_eq!(
        code("output(\"sub/x.txt\") { printf(\"x\"); }", &s),
        RUN_NOT_ALLOWED
    );
    assert!(!dir.path().join("sub").exists());
    // output blocks do not nest
    assert_eq!(
        code(
            "output(\"a.txt\") { output(\"b.txt\") { printf(\"x\"); } }",
            &s
        ),
        RUN_NOT_ALLOWED
    );
    assert_eq!(code("output(\"a.txt\", \"zz\") { }", &s), RUN_ARGUMENTS);
}

#[test]
fn an_error_inside_output_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let s = settings_for(dir.path());
    assert!(run(
        "output(\"a.txt\") { printf(\"x\"); int z = 1 / 0; }",
        &s,
        &RunEnv::default()
    )
    .is_err());
    assert!(!dir.path().join("a.txt").exists());
    // exit keeps what was written so far
    let o = run(
        "output(\"b.txt\") { printf(\"kept\"); exit(3); }",
        &s,
        &RunEnv::default(),
    )
    .unwrap();
    assert_eq!(o.exit_code, 3);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
        "kept"
    );
}

#[test]
fn the_limits_of_files_stop_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = settings_for(dir.path());
    s.max_file_bytes = 10;
    assert_eq!(
        code("output(\"a.txt\") { printf(\"0123456789ABC\"); }", &s),
        RUN_OUTPUT_LIMIT
    );
    s.max_file_bytes = 1000;
    s.max_files = 2;
    assert_eq!(
        code("output(\"a\") { } output(\"b\") { } output(\"c\") { }", &s),
        RUN_MEMORY_LIMIT
    );
}

#[test]
fn files_are_read_as_text_lines_or_listed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("in.txt"), "alpha\r\nbeta\ngamma\n").unwrap();
    std::fs::write(dir.path().join("x1.dat"), "1").unwrap();
    std::fs::write(dir.path().join("x2.dat"), "22").unwrap();
    std::fs::write(dir.path().join("y.txt"), "").unwrap();
    let s = settings_for(dir.path());
    let env = RunEnv::default();
    assert_eq!(
        out(
            "string l[]; int n = fileread(l, \"in.txt\"); printf(\"%d %s|%s|%s\\n\", n, l[0], l[1], l[2]);",
            &s,
            &env
        ),
        "3 alpha|beta|gamma\n"
    );
    assert_eq!(
        out(
            "string t; int n = fileread(t, \"in.txt\"); printf(\"%d %d\\n\", n, strlen(t));",
            &s,
            &env
        ),
        "18 18\n"
    );
    assert_eq!(
        out(
            "string f[]; int n = fileglob(f, \"*.dat\"); printf(\"%d %s %s\\n\", n, f[0], f[1]);",
            &s,
            &env
        ),
        "2 x1.dat x2.dat\n"
    );
    assert_eq!(
        out(
            "printf(\"%d %d\\n\", filesize(\"x2.dat\"), filesize(\"nothing\")); printf(\"%d\\n\", fileerror());",
            &s,
            &env
        ),
        "2 -1\n1\n"
    );
    let missing = out(
        "string t; int n = fileread(t, \"nothing.txt\"); printf(\"%d %d\\n\", n, fileerror());",
        &s,
        &env,
    );
    assert_eq!(missing, "-1 1\n");
    // a file larger than the limit is refused
    let mut small = s.clone();
    small.max_file_bytes = 4;
    assert_eq!(
        code("string t; fileread(t, \"in.txt\");", &small),
        RUN_MEMORY_LIMIT
    );
    assert_eq!(
        code("string t; fileread(t, \"../in.txt\");", &s),
        RUN_NOT_ALLOWED
    );
}

#[cfg(unix)]
#[test]
fn a_link_that_leaves_the_folder_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "no").unwrap();
    std::os::unix::fs::symlink(outside.path().join("secret.txt"), dir.path().join("l.txt"))
        .unwrap();
    let s = settings_for(dir.path());
    assert_eq!(
        code("string t; fileread(t, \"l.txt\");", &s),
        RUN_NOT_ALLOWED
    );
}

#[test]
fn argc_and_argv_come_from_the_caller() {
    let s = UlpSettings::default();
    let env = RunEnv {
        argv: vec!["prog".into(), "-v".into(), "3".into()],
        now: None,
    };
    assert_eq!(
        out(
            "printf(\"%d\", argc); for (int i = 0; i < argc; i++) printf(\" %s\", argv[i]); printf(\"\\n\");",
            &s,
            &env
        ),
        "3 prog -v 3\n"
    );
    assert_eq!(
        out("printf(\"%d\\n\", argc);", &s, &RunEnv::default()),
        "0\n"
    );
}

#[test]
fn the_clock_is_the_one_the_caller_gives() {
    let s = UlpSettings::default();
    let env = RunEnv {
        argv: Vec::new(),
        now: Some(1_700_000_000),
    };
    assert_eq!(
        out(
            "int t = time(); printf(\"%d %s|%d-%d-%d %d:%d:%d %d\\n\", t, t2string(t, \"yyyy-MM-dd hh:mm:ss\"), t2year(t), t2month(t), t2day(t), t2hour(t), t2minute(t), t2second(t), t2dayofweek(t));",
            &s,
            &env
        ),
        "1700000000 2023-11-14 10:13:20|2023-11-14 22:13:20 2\n"
    );
    // without a clock the machine's is read: after the year 2020
    assert_eq!(
        out(
            "printf(\"%d\\n\", t2year(time()) >= 2020);",
            &s,
            &RunEnv::default()
        ),
        "1\n"
    );
}

#[test]
fn dialogs_are_a_model_that_answers_as_accepted() {
    let s = UlpSettings::default();
    let text = "string name = \"x\"; int on = 1; int v = 0;\n\
        int r = dlgDialog(\"Settings\") {\n\
          dlgHBoxLayout { dlgLabel(\"Name\"); dlgStringEdit(name); }\n\
          dlgGroup(\"Options\") { dlgCheckBox(\"Enabled\", on); dlgSpacing(8); }\n\
          dlgPushButton(\"+OK\") dlgAccept();\n\
          dlgPushButton(\"Cancel\") { v = 99; dlgReject(); }\n\
        };\n\
        int m = dlgMessageBox(\"Done\", \"+Yes\", \"No\");\n\
        printf(\"%d %d %d %s\\n\", r, m, v, name);";
    let o = run(text, &s, &RunEnv::default()).unwrap();
    assert_eq!(o.output, "1 0 0 x\n");
    assert_eq!(o.dialogs.len(), 2);
    let d = &o.dialogs[0];
    assert_eq!(d.title, "Settings");
    let kinds: Vec<(&str, usize)> = d
        .controls
        .iter()
        .map(|c| (c.kind.as_str(), c.depth))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("dlgHBoxLayout", 1),
            ("dlgLabel", 2),
            ("dlgStringEdit", 2),
            ("dlgGroup", 1),
            ("dlgCheckBox", 2),
            ("dlgSpacing", 2),
            ("dlgPushButton", 1),
            ("dlgPushButton", 1),
        ]
    );
    assert_eq!(d.controls[2].text, "x");
    assert_eq!(o.dialogs[1].title, "Done");
    assert_eq!(o.dialogs[1].controls.len(), 2);
    // a control outside a dialog is an error
    assert_eq!(code("dlgLabel(\"x\");", &s), RUN_TYPE);
}
