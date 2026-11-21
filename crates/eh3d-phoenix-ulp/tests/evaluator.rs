// SPDX-License-Identifier: GPL-3.0-or-later
//! The evaluator on whole programs written for the tests, with fixed expected results: pure
//! computation, walks over a small board and a small schematic built here, the errors with their
//! codes, the limits that stop a run, the things a run must not do, and a generator with a fixed
//! seed that tries random pieces of the language and must never panic nor run for ever.

use std::collections::BTreeMap;

use eh3d_container::ObjectId;
use eh3d_phoenix_board::{
    Board, Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_schematic::{Part, Schematic, SchematicNet, Sheet};
use eh3d_phoenix_ulp::codes::{
    self, RUN_ARGUMENTS, RUN_DEPTH_LIMIT, RUN_DIVISION, RUN_FORMAT, RUN_HAS_PROBLEMS, RUN_INDEX,
    RUN_MEMORY_LIMIT, RUN_NAME, RUN_NOT_ALLOWED, RUN_NOT_WRITTEN, RUN_NO_MODEL, RUN_OUTPUT_LIMIT,
    RUN_RANGE, RUN_READ_ONLY, RUN_STEP_LIMIT, RUN_TYPE,
};
use eh3d_phoenix_ulp::{run_source, Outcome, Support, UlpSettings, World};

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

/// Two resistor-like parts of one footprint, two nets, one track of two pieces, a via and a zone.
fn board() -> Board {
    let (top, bottom) = (id(1), id(2));
    let (gnd, vcc) = (id(3), id(4));
    let (fp, pad1, pad2) = (id(5), id(6), id(7));
    let (r1, c1) = (id(8), id(9));
    let mut b = Board {
        name: "demo".into(),
        ..Board::default()
    };
    b.layers.insert(
        top,
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.layers.insert(
        bottom,
        Layer {
            name: "Bottom".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.copper_stack = vec![top, bottom];
    b.nets.insert(gnd, Net { name: "GND".into() });
    b.nets.insert(vcc, Net { name: "VCC".into() });
    let mut pads = BTreeMap::new();
    for (pid, number, x) in [(pad1, "1", -500_000), (pad2, "2", 500_000)] {
        pads.insert(
            pid,
            Pad {
                number: number.into(),
                shape: PadShape::Circle {
                    diameter: nm(600_000),
                },
                at: pt(x, 0),
                rotation: Udeg::new(0).unwrap(),
                drill: None,
                layers: vec![top],
                roundness: 0,
                plated: true,
            },
        );
    }
    b.footprints.insert(
        fp,
        Footprint {
            name: "R0603".into(),
            pads,
        },
    );
    for (cid, refdes, value, x, rot) in [
        (r1, "R1", "10k", 10_000_000, 0),
        (c1, "C1", "100n", 20_000_000, 90_000_000),
    ] {
        b.components.insert(
            cid,
            Component {
                refdes: refdes.into(),
                value: value.into(),
                symbol: None,
                footprint: fp,
                at: pt(x, 5_000_000),
                rotation: Udeg::new(rot).unwrap(),
                side: Side::Top,
            },
        );
        b.pad_nets.push(PadNet {
            component: cid,
            pad: pad1,
            net: vcc,
        });
        b.pad_nets.push(PadNet {
            component: cid,
            pad: pad2,
            net: gnd,
        });
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    b.tracks.insert(
        id(10),
        Track {
            net: gnd,
            layer: top,
            width: nm(200_000),
            path: vec![
                Segment::Line {
                    start: pt(0, 0),
                    end: pt(3_000_000, 4_000_000),
                },
                Segment::Line {
                    start: pt(3_000_000, 4_000_000),
                    end: pt(3_000_000, 0),
                },
            ],
        },
    );
    b.vias.insert(
        id(11),
        Via {
            net: vcc,
            at: pt(1_000_000, 2_000_000),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: top,
            to_layer: bottom,
        },
    );
    b.zones.insert(
        id(12),
        Zone {
            net: gnd,
            layer: bottom,
            outline: vec![
                pt(0, 0),
                pt(10_000_000, 0),
                pt(10_000_000, 10_000_000),
                pt(0, 10_000_000),
            ],
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    b
}

fn schematic() -> Schematic {
    let mut s = Schematic {
        name: "sheet demo".into(),
        ..Schematic::default()
    };
    for (n, refdes, value) in [(1, "R2", "1k"), (2, "R1", "10k")] {
        s.parts.insert(
            id(n),
            Part {
                refdes: refdes.into(),
                value: value.into(),
                symbol: id(50),
                footprint: None,
                placement: None,
            },
        );
    }
    s.nets.insert(
        id(60),
        SchematicNet {
            name: "SIG".into(),
            pins: Vec::new(),
        },
    );
    s.sheets.insert(
        id(70),
        Sheet {
            name: "Main".into(),
            width: nm(297_000_000),
            height: nm(210_000_000),
        },
    );
    s
}

fn run_with(text: &str, settings: &UlpSettings) -> Result<Outcome, eh3d_phoenix_ulp::Diag> {
    let (b, s) = (board(), schematic());
    run_source(
        text,
        settings,
        &Support::shipped(),
        World {
            board: Some(&b),
            schematic: Some(&s),
            ..World::default()
        },
    )
}

fn out(text: &str) -> String {
    match run_with(text, &UlpSettings::default()) {
        Ok(o) => o.output,
        Err(e) => panic!("{e}\n{text}"),
    }
}

fn code(text: &str) -> &'static str {
    match run_with(text, &UlpSettings::default()) {
        Ok(o) => panic!("it ran: {:?}\n{text}", o.output),
        Err(e) => e.code,
    }
}

#[test]
fn numbers_texts_and_functions_of_the_program() {
    assert_eq!(
        out(
            "int fact(int n) { if (n <= 1) return 1; return n * fact(n - 1); }\n\
             string s = \"ab\"; s += \"cd\";\n\
             printf(\"%d %s %d\\n\", fact(10), s, strlen(s));\n"
        ),
        "3628800 abcd 4\n"
    );
    assert_eq!(
        out("real r = 7 / 2; int i = 7 / 2; printf(\"%f %d %d %d\\n\", r, i, 7 % 3, -7 / 2);"),
        "3.000000 3 1 -3\n"
    );
    assert_eq!(
        out("printf(\"%d %d %d %d %d\\n\", 6 & 3, 6 | 3, 6 ^ 3, 1 << 4, 256 >> 4);"),
        "2 7 5 16 16\n"
    );
    assert_eq!(
        out("int a = 5; int b = a++ + ++a; printf(\"%d %d\\n\", a, b);"),
        "7 12\n"
    );
    assert_eq!(
        out("int x = 3; x *= 4; x -= 2; x <<= 1; printf(\"%d\\n\", x);"),
        "20\n"
    );
    assert_eq!(
        out("printf(\"%d %d %d\\n\", 1 && 0, 0 || 5, !7);"),
        "0 1 0\n"
    );
    assert_eq!(
        out("printf(\"%s %s\\n\", 3 > 2 ? \"yes\" : \"no\", \"abc\" < \"abd\" ? \"lt\" : \"ge\");"),
        "yes lt\n"
    );
    assert_eq!(
        out("int i = 0; int s = (i = 4, i + 1); printf(\"%d\\n\", s);"),
        "5\n"
    );
}

#[test]
fn statements_loops_switch_and_arrays() {
    assert_eq!(
        out("int s = 0; for (int i = 1; i <= 10; i++) { if (i % 2) continue; if (i > 8) break; s += i; } printf(\"%d\\n\", s);"),
        "20\n"
    );
    assert_eq!(
        out("int n = 5, f = 1; while (n > 1) f *= n--; printf(\"%d\\n\", f);"),
        "120\n"
    );
    assert_eq!(
        out("int n = 0; do { n += 3; } while (n < 10); printf(\"%d\\n\", n);"),
        "12\n"
    );
    assert_eq!(
        out("string name(int k) { switch (k) { case 1: return \"one\"; case 2: case 3: return \"few\"; default: return \"many\"; } }\n\
             printf(\"%s %s %s %s\\n\", name(1), name(2), name(3), name(9));"),
        "one few few many\n"
    );
    assert_eq!(
        out("int a[] = { 5, 3, 8 }; int s = 0; for (int i = 0; i < 3; i++) s += a[i]; a[5] = 1; printf(\"%d %d %d\\n\", s, a[4], a[5]);"),
        "16 0 1\n"
    );
    assert_eq!(
        out("string parts[]; int n = strsplit(parts, \"a,b,c\", \",\"); printf(\"%d %s %s\\n\", n, parts[2], strjoin(parts, \"+\"));"),
        "3 c a+b+c\n"
    );
    assert_eq!(
        out("string t; int n = sprintf(t, \"%05.1f|%-4d|\", 3.14159, 7); printf(\"%s %d\\n\", t, n);"),
        "003.1|7   | 11\n"
    );
    assert_eq!(
        out("string s = \"hello\"; printf(\"%c %c %d\\n\", s[0], s[4], s[1]);"),
        "h o 101\n"
    );
    assert_eq!(
        out("int g = 10; int bump() { g++; return g; } bump(); bump(); printf(\"%d\\n\", g);"),
        "12\n"
    );
    assert_eq!(
        out("int add(int a[], int n) { int t = 0; for (int i = 0; i < n; i++) t += a[i]; a[0] = 99; return t; }\n\
             int v[] = { 1, 2, 3 }; printf(\"%d %d\\n\", add(v, 3), v[0]);"),
        "6 1\n"
    );
}

#[test]
fn walks_over_the_board() {
    assert_eq!(
        out("board(B) { printf(\"%s\\n\", B.name); B.elements(E) { printf(\"%s %s %s %d %d %f %d\\n\", E.name, E.value, E.package.name, E.x, E.y, E.angle, E.mirror); } }"),
        "demo\nR1 10k R0603 10000000 5000000 0.000000 0\nC1 100n R0603 20000000 5000000 90.000000 0\n"
    );
    assert_eq!(
        out("board(B) { B.elements(E) { E.pads(P) { printf(\"%s.%s %s %d %d\\n\", E.name, P.name, P.signal, P.x, P.y); } } }"),
        "R1.1 VCC 9500000 5000000\nR1.2 GND 10500000 5000000\nC1.1 VCC 20000000 4500000\nC1.2 GND 20000000 5500000\n"
    );
    assert_eq!(
        out("board(B) { B.packages(K) { K.pads(P) { printf(\"%s %d %d %d %d\\n\", K.name, P.shape, P.x, P.diameter, P.width); } } }"),
        "R0603 1 -500000 600000 600000\nR0603 1 500000 600000 600000\n"
    );
    assert_eq!(
        out("board(B) { B.signals(S) { int w = 0, v = 0; S.wires(W) { w++; } S.vias(V) { v++; } printf(\"%s %d %d\\n\", S.name, w, v); } }"),
        "GND 2 0\nVCC 0 1\n"
    );
    assert_eq!(
        out("board(B) { B.wires(W) { printf(\"%d %d %d %d %d %d %s\\n\", W.x1, W.y1, W.x2, W.y2, W.width, W.length, W.signal); } }"),
        "0 0 3000000 4000000 200000 5000000 GND\n3000000 4000000 3000000 0 200000 4000000 GND\n"
    );
    assert_eq!(
        out("board(B) { B.vias(V) { printf(\"%d %d %d %d %s\\n\", V.x, V.y, V.drill, V.diameter, V.signal); } }"),
        "1000000 2000000 300000 600000 VCC\n"
    );
    assert_eq!(
        out("board(B) { B.polygons(G) { int n = 0; G.vertices(V) { n++; } printf(\"%s %d %.1f\\n\", G.signal, n, G.area); } }"),
        "GND 4 100000000000000.0\n"
    );
    assert_eq!(
        out("board(B) { B.layers(L) { printf(\"%d %s\\n\", L.number, L.name); } }"),
        "1 Top\n2 Bottom\n"
    );
    assert_eq!(
        out("board(B) { B.elements(E) { if (E.name == \"C1\") break; printf(\"%s\\n\", E.name); } }"),
        "R1\n"
    );
}

#[test]
fn walks_over_the_schematic_and_a_program_that_counts() {
    assert_eq!(
        out("schematic(S) { printf(\"%s\\n\", S.name); S.parts(P) { printf(\"%s=%s\\n\", P.name, P.value); } S.nets(N) { printf(\"net %s\\n\", N.name); } S.sheets(H) { printf(\"%d %s %d\\n\", H.number, H.name, H.width); } }"),
        "sheet demo\nR2=1k\nR1=10k\nnet SIG\n1 Main 297000000\n"
    );
    assert_eq!(
        out("#usage \"count\"\nint n = 0; real total = 0;\n\
             board(B) { B.elements(E) { n++; total += u2mm(E.x); } }\n\
             printf(\"%d parts, mean x %.1f mm\\n\", n, total / n);"),
        "2 parts, mean x 15.0 mm\n"
    );
    assert_eq!(
        out("board(B) { B.elements(E) { if (E.package == E.package) printf(\"same \"); } } printf(\"\\n\");"),
        "same same \n"
    );
}

#[test]
fn the_errors_have_their_codes() {
    for (text, expected) in [
        ("int x = 1 / 0;", RUN_DIVISION),
        ("int x = 5 % 0;", RUN_DIVISION),
        ("real x = 1.0 / 0.0;", RUN_DIVISION),
        ("int x = 9223372036854775807 + 1;", RUN_RANGE),
        ("int x = 1 << 70;", RUN_RANGE),
        ("real x = sqrt(-1);", RUN_RANGE),
        ("int a[2]; int x = a[5];", RUN_INDEX),
        ("string s = \"ab\"; int c = s[9];", RUN_INDEX),
        ("int x = y;", RUN_NAME),
        ("int x = nothing(1);", RUN_NAME),
        ("int x = \"a\" - 1;", RUN_TYPE),
        ("int x = 1; if (\"s\") x = 2;", RUN_TYPE),
        ("string s = 5;", RUN_TYPE),
        ("int x = sqrt(1, 2);", RUN_ARGUMENTS),
        ("int f(int a) { return a; } int x = f();", RUN_ARGUMENTS),
        ("printf(\"%d\", \"x\");", RUN_FORMAT),
        ("printf(\"%d %d\", 1);", RUN_FORMAT),
        ("board(B) { B.name = \"x\"; }", RUN_READ_ONLY),
        ("board(B) { B.elements(E) { E.x = 5; } }", RUN_READ_ONLY),
        ("board(B) { printf(\"%d\", B.nonsense); }", RUN_NOT_WRITTEN),
        ("board(B) { B.nonsense(E) { } }", RUN_NOT_WRITTEN),
        ("dlgLabel(\"y\");", RUN_TYPE),
        ("dlgDialog(\"x\") { dlgDialog(\"y\") { } }", RUN_NOT_ALLOWED),
        ("int t = time(1);", RUN_ARGUMENTS),
        ("library(L) { L.devices(D) { } }", RUN_NOT_WRITTEN),
        ("int x = 1 +;", RUN_HAS_PROBLEMS),
    ] {
        assert_eq!(code(text), expected, "{text}");
    }
}

#[test]
fn a_run_cannot_reach_a_file_a_command_or_the_model() {
    for text in [
        "system(\"dir\");",
        "int r = system(\"echo\");",
        "sleep(1);",
        "output(\"out.txt\", \"wt\") { printf(\"x\"); }",
        "string t; int n = fileread(t, \"a.txt\");",
        "#include \"other.ulp\"\nint a;",
        "exit(\"dir\");",
    ] {
        assert_eq!(code(text), RUN_NOT_ALLOWED, "{text}");
    }
    // nothing was written to the model: the run only received shared references
    let (b, s) = (board(), schematic());
    let before = (b.clone(), s.clone());
    let world = World {
        board: Some(&b),
        schematic: Some(&s),
        ..World::default()
    };
    let _ = run_source(
        "board(B) { B.elements(E) { printf(\"%s\", E.name); } }",
        &UlpSettings::default(),
        &Support::shipped(),
        world,
    );
    assert_eq!((b, s), before);
}

#[test]
fn a_program_that_needs_a_model_that_was_not_given_says_so() {
    let none = World::default();
    for text in ["board(B) { }", "schematic(S) { }"] {
        let e = run_source(text, &UlpSettings::default(), &Support::shipped(), none).unwrap_err();
        assert_eq!(e.code, RUN_NO_MODEL, "{text}");
    }
    let ok = run_source(
        "printf(\"%d\", 1 + 1);",
        &UlpSettings::default(),
        &Support::shipped(),
        none,
    )
    .unwrap();
    assert_eq!(ok.output, "2");
}

#[test]
fn exit_ends_the_run_with_its_number_and_keeps_the_output_so_far() {
    let o = run_with(
        "printf(\"a\"); if (1) exit(3); printf(\"b\");",
        &UlpSettings::default(),
    )
    .unwrap();
    assert_eq!((o.output.as_str(), o.exit_code), ("a", 3));
    let o = run_with("exit(0); printf(\"z\");", &UlpSettings::default()).unwrap();
    assert_eq!((o.output.as_str(), o.exit_code), ("", 0));
}

#[test]
fn every_limit_stops_a_run_with_its_code() {
    let small = |f: fn(&mut UlpSettings)| {
        let mut s = UlpSettings::default();
        f(&mut s);
        s
    };
    let e = |text: &str, s: &UlpSettings| run_with(text, s).unwrap_err().code;
    assert_eq!(
        e("while (1) { }", &small(|s| s.max_steps = 500)),
        RUN_STEP_LIMIT
    );
    assert_eq!(
        e("for (;;) ;", &small(|s| s.max_steps = 500)),
        RUN_STEP_LIMIT
    );
    assert_eq!(
        e(
            "int f(int n) { return f(n + 1); } int x = f(0);",
            &small(|s| s.max_eval_depth = 40)
        ),
        RUN_DEPTH_LIMIT
    );
    assert_eq!(
        e(
            "string s = \"xxxxxxxx\"; while (1) s = s + s;",
            &small(|s| s.max_memory_bytes = 4096)
        ),
        RUN_MEMORY_LIMIT
    );
    assert_eq!(
        e(
            "int a[]; for (int i = 0; i < 100000; i++) a[i] = i;",
            &small(|s| s.max_array_elements = 100)
        ),
        RUN_MEMORY_LIMIT
    );
    assert_eq!(
        e("int a[1000000];", &small(|s| s.max_array_elements = 100)),
        RUN_MEMORY_LIMIT
    );
    assert_eq!(
        e(
            "while (1) printf(\"0123456789\");",
            &small(|s| s.max_output_bytes = 100)
        ),
        RUN_OUTPUT_LIMIT
    );
    assert_eq!(
        e("int x; x = 1;", &small(|s| s.max_steps = 1)),
        RUN_STEP_LIMIT
    );
}

#[test]
fn deep_recursion_runs_when_the_settings_allow_the_depth() {
    let settings = UlpSettings {
        max_eval_depth: 2000,
        ..UlpSettings::default()
    };
    let o = run_with(
        "int f(int n) { if (n == 0) return 0; return 1 + f(n - 1); } printf(\"%d\", f(150));",
        &settings,
    )
    .unwrap();
    assert_eq!(o.output, "150");
}

#[test]
fn the_error_says_where_it_happened() {
    let e = run_with("int a = 1;\n\nint b = a / 0;\n", &UlpSettings::default()).unwrap_err();
    assert_eq!((e.code, e.line), (RUN_DIVISION, 3));
    assert!(e
        .message(eh3d_phoenix_rules::Lang::Pt)
        .contains("Divisão por zero"));
}

#[test]
fn the_same_program_and_model_give_the_same_bytes_and_steps() {
    let text = "board(B) { B.elements(E) { E.pads(P) { printf(\"%s.%s %d\\n\", E.name, P.name, P.x); } } B.signals(S) { printf(\"%s\\n\", S.name); } }";
    let a = run_with(text, &UlpSettings::default()).unwrap();
    for _ in 0..3 {
        assert_eq!(run_with(text, &UlpSettings::default()).unwrap(), a);
    }
    assert!(a.steps > 10);
}

/// A generator of numbers with a fixed seed.
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

const PIECES: &[&str] = &[
    "int ",
    "real ",
    "string ",
    "x",
    "y",
    "s",
    "a[]",
    "a[3]",
    "f(",
    "g(",
    ")",
    "(",
    "{",
    "}",
    ";",
    ",",
    "=",
    "+=",
    "++",
    "--",
    "+",
    "-",
    "*",
    "/",
    "%",
    "<<",
    "&&",
    "||",
    "!",
    "~",
    "==",
    "<",
    "?",
    ":",
    "0",
    "1",
    "7",
    "2.5",
    "\"t\"",
    "'c'",
    "if (",
    "else ",
    "while (",
    "do ",
    "for (",
    "return ",
    "break;",
    "continue;",
    "switch (",
    "case 1:",
    "default:",
    "board(B) ",
    "B.elements(E) ",
    "E.pads(P) ",
    "E.name",
    "P.x",
    "B.name",
    "printf(",
    "\"%d\"",
    "sprintf(s, ",
    "strlen(",
    "sqrt(",
    "abs(",
    "round(",
    "exit(",
    "system(",
    "time(",
    "output(",
    "\n",
];

#[test]
fn random_pieces_of_the_language_never_panic_and_never_run_for_ever() {
    let (b, s) = (board(), schematic());
    let settings = UlpSettings {
        max_steps: 3000,
        max_eval_depth: 30,
        max_memory_bytes: 8192,
        max_output_bytes: 512,
        max_array_elements: 64,
        ..UlpSettings::default()
    };
    let support = Support::shipped();
    let mut rng = Lcg(20_241_121);
    let known: Vec<&str> = codes::TITLES.iter().map(|(c, _)| *c).collect();
    let (mut ran, mut failed) = (0, 0);
    for _ in 0..1500 {
        let mut text = String::new();
        for _ in 0..(1 + rng.below(40)) {
            text.push_str(PIECES[rng.below(PIECES.len())]);
        }
        let world = World {
            board: Some(&b),
            schematic: Some(&s),
            ..World::default()
        };
        let first = run_source(&text, &settings, &support, world);
        let second = run_source(&text, &settings, &support, world);
        assert_eq!(first, second, "not the same twice: {text:?}");
        match first {
            Ok(o) => {
                ran += 1;
                assert!(o.output.len() <= 512 && o.steps <= 3000, "{text:?}");
            }
            Err(e) => {
                failed += 1;
                assert!(known.contains(&e.code), "{} for {text:?}", e.code);
            }
        }
    }
    assert!(ran > 0 && failed > 0, "{ran} ran, {failed} failed");
}

#[test]
fn programs_made_of_good_pieces_end_inside_the_budget() {
    let (b, s) = (board(), schematic());
    let settings = UlpSettings {
        max_steps: 20_000,
        max_eval_depth: 60,
        max_memory_bytes: 65_536,
        max_output_bytes: 4096,
        max_array_elements: 256,
        ..UlpSettings::default()
    };
    let support = Support::shipped();
    let bodies = [
        "n += i;",
        "n = n * 2 + 1;",
        "s += \"ab\";",
        "a[i % 16] = n;",
        "printf(\"%d \", i);",
        "if (i % 3 == 0) continue;",
        "if (i > 40) break;",
        "board(B) { B.elements(E) { n++; } }",
        "while (n > 1000) n /= 2;",
        "s = strsub(s, 1);",
    ];
    let mut rng = Lcg(77);
    for _ in 0..400 {
        let mut text = String::from("int n = 1; string s = \"x\"; int a[];\n");
        let count = 1 + rng.below(4);
        for k in 0..count {
            text.push_str(&format!(
                "for (int i = 0; i < {}; i++) {{ ",
                5 + rng.below(80)
            ));
            let _ = k;
            for _ in 0..(1 + rng.below(3)) {
                text.push_str(bodies[rng.below(bodies.len())]);
                text.push(' ');
            }
            text.push_str("}\n");
        }
        let world = World {
            board: Some(&b),
            schematic: Some(&s),
            ..World::default()
        };
        match run_source(&text, &settings, &support, world) {
            Ok(o) => assert!(o.steps <= 20_000, "{text}"),
            Err(e) => assert!(
                [
                    RUN_STEP_LIMIT,
                    RUN_MEMORY_LIMIT,
                    RUN_OUTPUT_LIMIT,
                    RUN_RANGE
                ]
                .contains(&e.code),
                "{} for {text}",
                e.code
            ),
        }
    }
}

/// The board and the schematic of the tests with an unplated hole, a text of the silkscreen, a
/// quarter-turn arc and two placed parts.
fn richer() -> (Board, Schematic) {
    let mut b = board();
    let fp = *b.footprints.keys().next().unwrap();
    b.footprints.get_mut(&fp).unwrap().pads.insert(
        id(20),
        Pad {
            number: "H".into(),
            shape: PadShape::Circle {
                diameter: nm(2_000_000),
            },
            at: pt(0, 1_000_000),
            rotation: Udeg::new(0).unwrap(),
            drill: Some(nm(1_600_000)),
            layers: Vec::new(),
            roundness: 0,
            plated: false,
        },
    );
    let top = *b.layers.keys().next().unwrap();
    b.silkscreen.insert(
        id(21),
        eh3d_phoenix_board::SilkItem::Text {
            layer: top,
            at: pt(7_000_000, 8_000_000),
            text: "REV A".into(),
            height: nm(1_000_000),
            stroke: nm(150_000),
            rotation: Udeg::new(90_000_000).unwrap(),
            mirrored: false,
        },
    );
    let t = b.tracks.get_mut(&id(10)).unwrap();
    t.path.push(Segment::Arc {
        start: pt(1_000_000, 0),
        end: pt(0, 1_000_000),
        sweep: Udeg::new(90_000_000).unwrap(),
    });
    let mut s = schematic();
    let sheet = *s.sheets.keys().next().unwrap();
    for (n, x) in [(1u16, 10_000_000), (2, 20_000_000)] {
        s.parts.get_mut(&id(n)).unwrap().placement = Some(eh3d_phoenix_schematic::Placement {
            sheet,
            at: pt(x, 5_000_000),
            rotation: Udeg::new(0).unwrap(),
        });
    }
    (b, s)
}

fn out_rich(text: &str) -> String {
    let (b, s) = richer();
    match run_source(
        text,
        &UlpSettings::default(),
        &Support::shipped(),
        World {
            board: Some(&b),
            schematic: Some(&s),
            ..World::default()
        },
    ) {
        Ok(o) => o.output,
        Err(e) => panic!("{e}\n{text}"),
    }
}

#[test]
fn contacts_holes_texts_instances_and_arcs_are_read() {
    // contacts: every pad of an element; smds: the ones without a drill
    assert_eq!(
        out_rich("board(B) { B.elements(E) { int n = 0; int s = 0; E.contacts(C) { n++; } E.smds(M) { s++; } printf(\"%s %d %d\\n\", E.name, n, s); } }"),
        "R1 3 2\nC1 3 2\n"
    );
    assert_eq!(
        out_rich("board(B) { B.elements(E) { E.contacts(C) { printf(\"%s \", C.pad.name); } } printf(\"\\n\"); }"),
        "1 2 H 1 2 H \n"
    );
    // holes: the unplated pads, placed with the component (R1 at 10 mm, 5 mm; hole 1 mm up)
    assert_eq!(
        out_rich(
            "board(B) { B.holes(H) { printf(\"%d %d %d %d\\n\", H.x, H.y, H.drill, H.radius); } }"
        ),
        "10000000 6000000 1600000 800000\n19000000 5000000 1600000 800000\n"
    );
    assert_eq!(
        out_rich("board(B) { B.packages(K) { K.holes(H) { printf(\"%d\\n\", H.drill); } } }"),
        "1600000\n"
    );
    assert_eq!(
        out_rich("board(B) { B.texts(T) { printf(\"%s %d %d %d %f\\n\", T.value, T.x, T.y, T.size, T.angle); } }"),
        "REV A 7000000 8000000 1000000 90.000000\n"
    );
    assert_eq!(
        out_rich("schematic(S) { S.instances(I) { printf(\"%s %d %s\\n\", I.name, I.x, I.part.value); } }"),
        "R2 10000000 1k\nR1 20000000 10k\n"
    );
    assert_eq!(
        out_rich("schematic(S) { S.sheets(H) { H.instances(I) { printf(\"%s \", I.name); } } printf(\"\\n\"); }"),
        "R2 R1 \n"
    );
    // the arc of the track: centre at the origin, radius 1 mm; a straight piece answers 0
    assert_eq!(
        out_rich("board(B) { B.wires(W) { printf(\"%d %d %d|\", W.xc, W.yc, W.radius); } printf(\"\\n\"); }"),
        "0 0 0|0 0 0|0 0 1000000|\n"
    );
    // library(L) goes over the packages of the board
    assert_eq!(
        out_rich("library(L) { printf(\"%s:\", L.name); L.packages(K) { printf(\" %s\", K.name); } printf(\"\\n\"); }"),
        "demo: R0603\n"
    );
}
