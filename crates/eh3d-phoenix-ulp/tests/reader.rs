// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader and the report on whole programs written for the tests: the constructs of the
//! programs that read a design and write a file, programs the report must refuse, the limits, the
//! codes against the documentation, and the reader on text made by a generator with a fixed seed
//! (random pieces of the language, and a good program with characters changed, deleted and added).

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_ulp::codes::{self, TITLES};
use eh3d_phoenix_ulp::{
    analyse, parse, report_json, report_text, Status, Support, UlpSettings, UseKind, Verdict,
};

/// A generator of numbers with a fixed seed, so a test that tries many cases tries the same ones
/// every time.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const PARTS_LIST: &str = r#"#usage "en: Lists the parts of a board", "es: Lista las piezas de una placa"
#require 6.0

int count = 0;
real total = 0.0;
string names[];

string line(UL_ELEMENT e) {
  string s;
  sprintf(s, "%-8s %-12s %8.3f %8.3f", e.name, e.value, u2mm(e.x), u2mm(e.y));
  return s;
}

if (!board) {
  exit(1);
}
board(B) {
  B.elements(E) {
    names[count++] = line(E);
    total += E.x;
  }
}
output("parts.txt", "wt") {
  printf("%d parts\n", count);
  for (int i = 0; i < count; i++) {
    printf("%s\n", names[i]);
  }
}
"#;

const NETS_BY_LAYER: &str = r#"
string layers[];
int n = 0;
board(B) {
  B.layers(L) {
    layers[n++] = L.name;
  }
  B.signals(S) {
    int wires = 0;
    S.wires(W) { wires++; }
    switch (wires) {
      case 0: status("no copper in " + S.name); break;
      default: ;
    }
  }
}
"#;

const WITH_A_DIALOG: &str = r#"
string choice = "all";
int ok = dlgDialog("Choose") {
  dlgHBoxLayout {
    dlgLabel("Which parts?");
    dlgStringEdit(choice);
  }
  dlgPushButton("+OK") { dlgAccept(); }
};
"#;

const RUNS_A_COMMAND: &str = r#"
string cmd = "dir";
system(cmd);
"#;

fn report(text: &str) -> eh3d_phoenix_ulp::Report {
    analyse(
        "test.ulp",
        text,
        &UlpSettings::default(),
        &Support::shipped(),
    )
}

#[test]
fn a_program_that_lists_parts_is_read_and_is_covered_with_limits() {
    let parsed = parse(PARTS_LIST, &UlpSettings::default()).unwrap();
    assert!(
        parsed.problems.list.is_empty(),
        "{:?}",
        parsed.problems.list
    );
    assert_eq!(parsed.program.directives.len(), 2);
    assert_eq!(parsed.program.functions.len(), 1);
    let r = report(PARTS_LIST);
    assert_eq!(r.verdict, Verdict::Covered, "{}", report_text(&r, Lang::En));
    let u2mm = r.uses.iter().find(|u| u.name == "u2mm");
    assert!(
        u2mm.is_some_and(|u| u.status == Some(Status::Allowed)),
        "{u2mm:?}"
    );
    let output = r
        .uses
        .iter()
        .find(|u| u.kind == UseKind::Function && u.name == "output")
        .unwrap();
    assert_eq!(output.status, Some(Status::Limited));
    assert_eq!(output.count, 1);
}

#[test]
fn a_program_that_walks_layers_and_signals_is_covered() {
    let r = report(NETS_BY_LAYER);
    assert_eq!(r.verdict, Verdict::Covered, "{}", report_text(&r, Lang::En));
    for member in ["layers", "signals", "wires", "name"] {
        assert!(
            r.uses
                .iter()
                .any(|u| u.kind == UseKind::Member && u.name == member),
            "{member}"
        );
    }
}

#[test]
fn a_program_with_a_dialog_is_read_covered_and_run_without_a_window() {
    let r = report(WITH_A_DIALOG);
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    assert_eq!(r.verdict, Verdict::Covered);
    assert!(r.uses.iter().any(|u| u.name == "dlgDialog" && u.runs));
}

#[test]
fn a_program_that_runs_a_command_is_not_covered() {
    let r = report(RUNS_A_COMMAND);
    assert_eq!(r.verdict, Verdict::NotCovered);
    let text = report_text(&r, Lang::En);
    assert!(text.contains("PHOENIX-ULP-020 [3:1]"), "{text}");
}

#[test]
fn the_report_is_the_same_text_for_the_same_program_in_every_language() {
    for text in [PARTS_LIST, NETS_BY_LAYER, WITH_A_DIALOG, RUNS_A_COMMAND] {
        let (a, b) = (report(text), report(text));
        assert_eq!(report_json(&a), report_json(&b));
        for lang in Lang::ALL {
            let t = report_text(&a, lang);
            assert_eq!(t, report_text(&b, lang));
            assert!(t.contains(&a.sha256));
        }
    }
}

#[test]
fn the_report_never_runs_a_program_and_says_what_the_evaluator_is() {
    let r = report(RUNS_A_COMMAND);
    let json = report_json(&r);
    assert!(json.contains("\"ran\": false"), "{json}");
    assert!(json.contains("\"evaluator\": \"read_only\""), "{json}");
    assert!(json.contains("\"sandbox\": \"not_written\""), "{json}");
    assert!(json.contains("\"runnable\": false"), "{json}");
}

#[test]
fn every_limit_of_the_settings_stops_a_program_that_goes_beyond_it() {
    let base = UlpSettings::default();
    let cases: Vec<(UlpSettings, String, &str)> = vec![
        (
            UlpSettings {
                max_source_bytes: 50,
                ..base.clone()
            },
            "x".repeat(51),
            codes::SOURCE_TOO_LARGE,
        ),
        (
            UlpSettings {
                max_tokens: 10,
                ..base.clone()
            },
            "a ".repeat(20),
            codes::TOO_MANY_TOKENS,
        ),
        (
            UlpSettings {
                max_depth: 5,
                ..base.clone()
            },
            format!("x = {}1{};", "(".repeat(20), ")".repeat(20)),
            codes::TOO_DEEP,
        ),
        (
            UlpSettings {
                max_statements: 3,
                ..base.clone()
            },
            ";;;;".to_owned(),
            codes::TOO_MANY_STATEMENTS,
        ),
    ];
    for (settings, text, code) in cases {
        let e = parse(&text, &settings).unwrap_err();
        assert_eq!(e.code, code);
        let r = analyse("x.ulp", &text, &settings, &Support::shipped());
        assert_eq!(r.verdict, Verdict::NotCovered);
        assert_eq!(r.stopped.as_ref().map(|d| d.code), Some(code));
    }
}

#[test]
fn a_very_deep_program_is_refused_without_overflowing_the_stack() {
    let settings = UlpSettings::default();
    for text in [
        format!("x = {}1;", "(".repeat(150_000)),
        format!("{}{}", "{".repeat(150_000), "}".repeat(150_000)),
        format!("x = {}1;", "-".repeat(150_000)),
        format!("x = 1{};", " + 1".repeat(40_000)),
        format!("x = a{};", ".b".repeat(80_000)),
        format!("x = a{};", "[1]".repeat(60_000)),
        format!("{}x;{}", "if (1) ".repeat(60_000), ""),
    ] {
        let e = parse(&text, &settings).unwrap_err();
        assert!(
            e.code == codes::TOO_DEEP || e.code == codes::TOO_MANY_TOKENS,
            "{}",
            e.code
        );
    }
}

#[test]
fn a_long_program_inside_the_limits_is_read() {
    let mut text = String::new();
    for i in 0..5000 {
        text.push_str(&format!(
            "int v{i} = {i};\nif (v{i} > 3) v{i}++; else v{i}--;\n"
        ));
    }
    let parsed = parse(&text, &UlpSettings::default()).unwrap();
    assert!(parsed.problems.list.is_empty());
    assert_eq!(parsed.program.body.len(), 10_000);
}

#[test]
fn every_code_is_in_the_documentation_with_its_title() {
    let doc = include_str!("../../../docs/ERROR_CODES.md");
    for (code, row) in TITLES {
        assert!(doc.contains(&format!("| {code} | {} |", row[0])), "{code}");
    }
}

#[test]
fn the_list_of_names_in_the_config_folder_is_the_one_that_ships() {
    let text = include_str!("../../../config/ulp_support.cfg");
    assert_eq!(Support::parse(text).unwrap(), Support::shipped());
}

const WORDS: &[&str] = &[
    "int",
    "real",
    "string",
    "char",
    "UL_WIRE",
    "if",
    "else",
    "for",
    "while",
    "do",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "return",
    "board",
    "B",
    "E",
    "x",
    "y",
    "name",
    "output",
    "printf",
    "sprintf",
    "dlgDialog",
    "system",
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ";",
    ",",
    ".",
    "=",
    "+=",
    "==",
    "<=",
    "&&",
    "||",
    "++",
    "--",
    "?",
    ":",
    "+",
    "-",
    "*",
    "/",
    "%",
    "!",
    "~",
    "\"text\"",
    "\"open",
    "'c'",
    "42",
    "3.5e2",
    "0xff",
    "12ab",
    "#usage",
    "#include",
    "/*",
];

fn invariants(text: &str, settings: &UlpSettings) {
    match parse(text, settings) {
        Ok(p) => {
            assert!(p.problems.list.len() <= settings.max_problems);
            for d in &p.problems.list {
                assert!(d.line >= 1 && d.col >= 1, "{d:?}");
                assert!(!d.detail.is_empty() || d.code == codes::LIST_CUT, "{d:?}");
                assert!(!codes::title(d.code, Lang::En).is_empty(), "{d:?}");
            }
            assert_eq!(parse(text, settings).unwrap(), p, "not deterministic");
        }
        Err(e) => {
            assert!(
                [
                    codes::SOURCE_TOO_LARGE,
                    codes::TOO_MANY_TOKENS,
                    codes::TOO_DEEP,
                    codes::TOO_MANY_STATEMENTS
                ]
                .contains(&e.code),
                "{e:?}"
            );
            assert!(e.line >= 1 && e.col >= 1, "{e:?}");
        }
    }
    let r = analyse("fuzz.ulp", text, settings, &Support::shipped());
    assert!(!report_text(&r, Lang::Pt).is_empty());
    assert!(serde_json_ok(&report_json(&r)));
}

fn serde_json_ok(text: &str) -> bool {
    text.starts_with('{') && text.trim_end().ends_with('}')
}

#[test]
fn random_pieces_of_the_language_never_stop_the_reader() {
    let mut rng = Lcg(0x0ddc_0ffe_e5ee_d001);
    let small = UlpSettings {
        max_depth: 12,
        max_tokens: 400,
        max_statements: 60,
        max_problems: 8,
        ..UlpSettings::default()
    };
    for _ in 0..3000 {
        let n = rng.below(60);
        let mut text = String::new();
        for _ in 0..n {
            text.push_str(WORDS[rng.below(WORDS.len())]);
            text.push(if rng.below(5) == 0 { '\n' } else { ' ' });
        }
        invariants(&text, &small);
        invariants(&text, &UlpSettings::default());
    }
}

#[test]
fn a_good_program_with_characters_changed_deleted_and_added_never_stops_the_reader() {
    let mut rng = Lcg(0x5eed_ed00);
    let noise: Vec<char> = "(){}[];,.\"'/*#\\@$`~!?:=+-<>&|^% \n\t0aZ_ñ日"
        .chars()
        .collect();
    let base: Vec<char> = PARTS_LIST.chars().collect();
    let settings = UlpSettings {
        max_problems: 10,
        ..UlpSettings::default()
    };
    for _ in 0..1500 {
        let mut text = base.clone();
        for _ in 0..(1 + rng.below(6)) {
            let at = rng.below(text.len());
            match rng.below(3) {
                0 => text[at] = noise[rng.below(noise.len())],
                1 => {
                    text.remove(at);
                }
                _ => text.insert(at, noise[rng.below(noise.len())]),
            }
        }
        let text: String = text.into_iter().collect();
        invariants(&text, &settings);
    }
}

#[test]
fn text_cut_at_every_point_never_stops_the_reader() {
    let chars: Vec<char> = NETS_BY_LAYER.chars().collect();
    let settings = UlpSettings::default();
    for end in 0..chars.len() {
        let text: String = chars[..end].iter().collect();
        invariants(&text, &settings);
    }
}

#[test]
fn bytes_that_are_not_text_are_read_without_stopping() {
    let mut rng = Lcg(0xb17e5);
    for _ in 0..300 {
        let n = rng.below(200);
        let bytes: Vec<u8> = (0..n).map(|_| (rng.next() & 0xff) as u8).collect();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        invariants(&text, &UlpSettings::default());
    }
}
