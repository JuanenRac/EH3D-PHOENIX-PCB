// SPDX-License-Identifier: GPL-3.0-or-later
//! `#include` from the folder for files and the search of patterns, run as programs.

use std::fs;

use eh3d_phoenix_ulp::codes::{
    RUN_DEPTH_LIMIT, RUN_HAS_PROBLEMS, RUN_NOT_ALLOWED, RUN_NOT_WRITTEN,
};
use eh3d_phoenix_ulp::{run_source, Support, UlpSettings, World};

fn settings(dir: &std::path::Path) -> UlpSettings {
    UlpSettings {
        file_dir: dir.to_string_lossy().into_owned(),
        ..UlpSettings::default()
    }
}

fn run(text: &str, settings: &UlpSettings) -> Result<String, &'static str> {
    run_source(text, settings, &Support::shipped(), World::default())
        .map(|outcome| outcome.output)
        .map_err(|e| e.code)
}

#[test]
fn the_files_a_program_names_come_before_it_and_each_only_once() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("base.ulp"),
        "int base = 40;\nint twice(int n) { return 2 * n; }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("more.ulp"),
        "#include \"base.ulp\"\nint more = base + 2;\n",
    )
    .unwrap();
    let text =
        "#include \"more.ulp\"\n#include <base.ulp>\nprintf(\"%d %d\", more, twice(base));\n";
    assert_eq!(run(text, &settings(dir.path())).unwrap(), "42 80");
    // two files that name each other end
    fs::write(dir.path().join("a.ulp"), "#include \"b.ulp\"\nint a = 1;\n").unwrap();
    fs::write(dir.path().join("b.ulp"), "#include \"a.ulp\"\nint b = 2;\n").unwrap();
    let text = "#include \"a.ulp\"\nprintf(\"%d\", a + b);\n";
    assert_eq!(run(text, &settings(dir.path())).unwrap(), "3");
}

#[test]
fn an_include_outside_the_folder_missing_broken_or_too_deep_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let s = settings(dir.path());
    assert_eq!(
        run("#include \"a.ulp\"\n", &UlpSettings::default()),
        Err(RUN_NOT_ALLOWED)
    );
    assert_eq!(run("#include \"../a.ulp\"\n", &s), Err(RUN_NOT_ALLOWED));
    assert!(run("#include \"none.ulp\"\n", &s).is_err());
    fs::write(dir.path().join("broken.ulp"), "int = ;\n").unwrap();
    assert_eq!(run("#include \"broken.ulp\"\n", &s), Err(RUN_HAS_PROBLEMS));
    for n in 0..4 {
        fs::write(
            dir.path().join(format!("{n}.ulp")),
            format!("#include \"{}.ulp\"\n", n + 1),
        )
        .unwrap();
    }
    let shallow = UlpSettings {
        max_include_depth: 3,
        ..s
    };
    assert_eq!(run("#include \"0.ulp\"\n", &shallow), Err(RUN_DEPTH_LIMIT));
}

#[test]
fn a_pattern_is_found_with_its_place_and_length() {
    let s = UlpSettings::default();
    let text = "int len = 0;\nstring t = \"C1 R12 R3\";\nint at = strxstr(t, \"R[0-9]+\", 0, len);\nwhile (at >= 0) { printf(\"%d:%d \", at, len); at = strxstr(t, \"R[0-9]+\", at + len, len); }\nprintf(\"%d %d\", len, strxstr(t, \"X\"));\n";
    assert_eq!(run(text, &s).unwrap(), "3:3 7:2 0 -1");
    assert_eq!(
        run("int n = strxstr(\"aaa\", \"a{2}\");", &s),
        Err(RUN_NOT_WRITTEN)
    );
    assert_eq!(run("printf(\"%s\", country());", &s).unwrap(), "US");
}
