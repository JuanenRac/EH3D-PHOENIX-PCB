// SPDX-License-Identifier: GPL-3.0-or-later
//! The tests of the scripts of the second version: the language, the limits, the sandbox, the
//! operations on a board, a schematic and a library, the one step of the history, the check and
//! the example scripts with their expected output.

use std::collections::BTreeMap;
use std::path::Path;

use eh3d_phoenix_board::{Board, Component, History, Operation, Side};
use eh3d_phoenix_libedit::{apply_all, Library};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, StepClock, Udeg};
use eh3d_phoenix_parts::{Part as BasePart, PartsDb};
use eh3d_phoenix_rules::{Lang, RuleValues};
use eh3d_phoenix_schematic::{Editor, Part, Pin, Placement, Schematic, Sheet, Symbol};

use crate::exec::RunContext;
use crate::files;
use crate::settings::ScriptSettings;
use crate::testdata::{at, fixture, mm};
use crate::v2::check::check_text;
use crate::v2::codes as c;
use crate::v2::interp::{run, Doc, Outcome, Plan};
use crate::v2::syntax::{parse_program, Target};
use crate::v2::templates::{expected_output, TEMPLATES};

fn source(body: &str) -> String {
    format!("phoenix-script/2\n{body}")
}

fn run_with(doc: Doc<'_>, body: &str, settings: &ScriptSettings) -> Outcome {
    let program = parse_program(&source(body), settings).unwrap();
    let ctx = RunContext::new(settings);
    run(&program, doc, &ctx)
}

fn on_board(board: &Board, body: &str) -> Outcome {
    run_with(Doc::Board(board), body, &ScriptSettings::default())
}

fn done(board: &Board, body: &str) -> Outcome {
    let o = on_board(board, body);
    assert!(o.ok, "{body}: {:?}", o.diagnostics);
    o
}

fn failed(board: &Board, body: &str) -> crate::codes::Diag {
    let o = on_board(board, body);
    assert!(!o.ok, "{body} should fail");
    assert!(o.board.is_none() && o.plan == Plan::None && o.writes.is_empty());
    o.diagnostics[0].clone()
}

fn lines(o: &Outcome) -> Vec<&str> {
    o.output.iter().map(String::as_str).collect()
}

fn with_components(mut board: Board, list: &[(&str, i64, i64)]) -> Board {
    let footprint = *board.footprints.keys().next().unwrap();
    let mut ids = IdGenerator::new(
        StepClock::new(1_650_000_000_000, 1),
        DeterministicRandom::new(99),
    );
    for (refdes, x, y) in list {
        let id = ids.generate().unwrap();
        board
            .apply(&Operation::AddComponent {
                id,
                component: Component {
                    refdes: (*refdes).to_owned(),
                    value: String::new(),
                    symbol: None,
                    footprint,
                    at: at(*x, *y),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            })
            .unwrap();
    }
    board
}

fn schematic() -> Schematic {
    let mut ids = IdGenerator::new(
        StepClock::new(1_600_000_000_000, 1),
        DeterministicRandom::new(11),
    );
    let mut next = || ids.generate().unwrap();
    let (symbol, sheet, p1, p2, pin1, pin2) = (next(), next(), next(), next(), next(), next());
    let part = |refdes: &str, value: &str, x: i64| Part {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol,
        footprint: None,
        placement: Some(Placement {
            sheet,
            at: at(x, 10),
            rotation: Udeg::ZERO,
        }),
    };
    Schematic {
        name: "s".to_owned(),
        symbols: BTreeMap::from([(
            symbol,
            Symbol {
                name: "R".to_owned(),
                pins: BTreeMap::from([
                    (
                        pin1,
                        Pin {
                            number: "1".to_owned(),
                            at: None,
                        },
                    ),
                    (
                        pin2,
                        Pin {
                            number: "2".to_owned(),
                            at: None,
                        },
                    ),
                ]),
            },
        )]),
        sheets: BTreeMap::from([(
            sheet,
            Sheet {
                name: "S1".to_owned(),
                width: mm(297),
                height: mm(210),
            },
        )]),
        parts: BTreeMap::from([(p1, part("R1", "10k", 10)), (p2, part("R2", "1k", 20))]),
        ..Schematic::default()
    }
}

// ----- the language -----

#[test]
fn arithmetic_text_and_units_give_integers() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "print(1 + 2 * 3, 7 / 2, -7 % 3, \"a\" + \"b\", 2mm, 90deg, 8mil)\nprint(2 < 3, \"b\" > \"a\", 1 == 1, 1 != 1, not false and true or false)",
    );
    assert_eq!(
        lines(&o),
        vec![
            "7 3 -1 ab 2000000 90000000 203200",
            "true true true false true"
        ]
    );
}

#[test]
fn variables_follow_the_blocks() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "let a = 1\nif true {\n let b = 2\n a = a + b\n let a = 10\n print(a)\n}\nprint(a)\nlet xs = [1, [2, 3], \"z\"]\nprint(xs[1][0], xs[2], len(xs), xs)",
    );
    assert_eq!(lines(&o), vec!["10", "3", "2 z 3 [1, [2, 3], z]"]);
    let d = failed(&fx.board, "let a = 1\nlet a = 2");
    assert_eq!((d.code, d.line, d.col), (c::REDEFINED, 3, 1));
    assert_eq!(failed(&fx.board, "b = 2").code, c::UNDEFINED);
    assert_eq!(
        failed(&fx.board, "if true { let k = 1 }\nprint(k)").code,
        c::UNDEFINED
    );
}

#[test]
fn loops_conditions_break_and_continue() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "let total = 0\nfor i in 0..10 {\n if i % 2 == 0 { continue }\n if i > 7 { break }\n total = total + i\n}\nprint(total)\nlet n = 0\nwhile n < 3 { n = n + 1 }\nprint(n)\nfor w in [\"x\", \"y\"] { print(w) }\nif n == 4 { print(\"four\") } else if n == 3 { print(\"three\") } else { print(\"other\") }",
    );
    assert_eq!(lines(&o), vec!["16", "3", "x", "y", "three"]);
}

#[test]
fn conditions_can_read_the_counts_of_the_document() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "if len(select(\"component\", \"R*\")) == 2 and len(select(\"track\", \"GND\")) == 1 {\n print(\"two resistors, one GND track\")\n}\nprint(len(select(\"net\", \"*\")), len(select(\"via\", \"*\")))",
    );
    assert_eq!(lines(&o), vec!["two resistors, one GND track", "2 0"]);
}

#[test]
fn errors_name_the_line_and_the_column() {
    let fx = fixture();
    for (body, code, line, col) in [
        ("print(zzz)", c::UNDEFINED, 2, 7),
        ("print(1 / 0)", c::ARITHMETIC, 2, 9),
        ("print(1 % 0)", c::ARITHMETIC, 2, 9),
        ("print(9223372036854775807 + 1)", c::ARITHMETIC, 2, 27),
        ("print(1 + \"a\")", c::TYPE_MISMATCH, 2, 9),
        ("if 1 { }", c::TYPE_MISMATCH, 2, 4),
        ("print(len(5))", c::TYPE_MISMATCH, 2, 7),
        ("print(nothing(1))", c::UNKNOWN_FUNCTION, 2, 7),
        ("print(len())", c::ARGUMENTS, 2, 7),
        ("print([1][5])", c::INDEX, 2, 10),
        ("print(\"ab\"[2])", c::INDEX, 2, 11),
        ("assert(1 == 2, \"nope\")", c::ASSERTION, 2, 1),
        ("fail(\"stop\")", c::ASSERTION, 2, 1),
        ("print(select(\"gizmo\", \"*\"))", c::TYPE_MISMATCH, 2, 7),
        ("for i in 5 { }", c::TYPE_MISMATCH, 2, 10),
        ("print(2.field)", c::TYPE_MISMATCH, 2, 8),
    ] {
        let program = parse_program(&source(body), &ScriptSettings::default());
        match program {
            Err(d) => assert_eq!((d.code, d.line, d.col), (code, line, col), "{body}"),
            Ok(_) => {
                let d = failed(&fx.board, body);
                assert_eq!((d.code, d.line, d.col), (code, line, col), "{body}");
            }
        }
    }
    let d = failed(&fx.board, "assert(1 == 2, \"nope\")");
    assert_eq!(d.detail, "nope");
}

#[test]
fn the_step_loop_and_memory_limits_stop_a_script() {
    let fx = fixture();
    let by_steps = ScriptSettings {
        max_steps: 1_000,
        max_loop_iterations: 1_000_000,
        ..ScriptSettings::default()
    };
    let o = run_with(Doc::Board(&fx.board), "while true { }", &by_steps);
    assert!(!o.ok);
    assert_eq!(o.diagnostics[0].code, c::STEP_LIMIT);
    assert!(o.steps > 1_000);
    assert_eq!(failed(&fx.board, "while true { }").code, c::ITERATION_LIMIT);
    assert_eq!(
        failed(&fx.board, "for i in 0..20000 { }").code,
        c::ITERATION_LIMIT
    );
    let cells = ScriptSettings {
        max_value_cells: 50,
        ..ScriptSettings::default()
    };
    let o = run_with(Doc::None, "let xs = range(0, 100)", &cells);
    assert_eq!(o.diagnostics[0].code, c::MEMORY_LIMIT);
    let list = ScriptSettings {
        max_list_len: 10,
        ..ScriptSettings::default()
    };
    let o = run_with(Doc::None, "print(range(0, 11))", &list);
    assert_eq!(o.diagnostics[0].code, c::MEMORY_LIMIT);
    let text = ScriptSettings {
        max_text_bytes: 50,
        ..ScriptSettings::default()
    };
    let o = run_with(Doc::None, "print(repeat(\"x\", 100))", &text);
    assert_eq!(o.diagnostics[0].code, c::MEMORY_LIMIT);
    let o = run_with(Doc::None, "let s = repeat(\"x\", 40)\nlet t = s + s", &text);
    assert_eq!(o.diagnostics[0].code, c::MEMORY_LIMIT);
    let deep = run_with(
        Doc::None,
        "let a = [1]\nfor i in 0..20 { a = [a] }",
        &ScriptSettings::default(),
    );
    assert_eq!(deep.diagnostics[0].code, c::MEMORY_LIMIT);
    let output = ScriptSettings {
        max_output_bytes: 10,
        ..ScriptSettings::default()
    };
    let o = run_with(Doc::None, "print(\"hello world\")", &output);
    assert_eq!(o.diagnostics[0].code, c::OUTPUT_LIMIT);
    assert!(o.output.is_empty());
}

#[test]
fn a_script_cannot_reach_the_network_a_program_or_the_environment() {
    for name in [
        "system",
        "exec",
        "run",
        "spawn",
        "open",
        "http_get",
        "fetch",
        "socket",
        "env",
        "getenv",
        "import",
        "eval",
        "shell",
        "read_file",
        "delete_file",
    ] {
        let o = run_with(
            Doc::None,
            &format!("{name}(\"x\")"),
            &ScriptSettings::default(),
        );
        assert!(!o.ok);
        assert_eq!(o.diagnostics[0].code, c::UNKNOWN_FUNCTION, "{name}");
    }
}

#[test]
fn a_run_without_a_document_runs_only_the_functions_that_need_none() {
    let o = run_with(
        Doc::None,
        "print(len([1, 2]), upper(\"ab\"), sum([1, 2, 3]), join([1, 2], \"-\"), zpad(7, 3))",
        &ScriptSettings::default(),
    );
    assert!(o.ok);
    assert_eq!(lines(&o), vec!["2 AB 6 1-2 007"]);
    let o = run_with(
        Doc::None,
        "print(select(\"net\", \"*\"))",
        &ScriptSettings::default(),
    );
    assert_eq!(o.diagnostics[0].code, c::TARGET);
    assert_eq!(o.plan, Plan::None);
}

#[test]
fn the_header_target_must_match_the_document() {
    let fx = fixture();
    let settings = ScriptSettings::default();
    let program = parse_program("phoenix-script/2 schematic\nprint(1)", &settings).unwrap();
    let ctx = RunContext::new(&settings);
    let o = run(&program, Doc::Board(&fx.board), &ctx);
    assert!(!o.ok);
    assert_eq!(o.diagnostics[0].code, c::TARGET);
    let o = run(&program, Doc::Schematic(&schematic()), &ctx);
    assert!(o.ok);
    assert_eq!(
        failed(&fx.board, "lib_create(\"package\", \"A\")").code,
        c::TARGET
    );
}

#[test]
fn the_functions_of_numbers_and_text_are_exact() {
    let o = run_with(
        Doc::None,
        "print(isqrt(17000000000000), muldiv(7, 3, 2), muldiv(-7, 3, 2), abs(-4), min(3, 2), max(3, 2))\nprint(polar_x(10mm, 0), polar_y(10mm, 90deg), polar_x(10mm, 60deg), sin_ppm(30deg), cos_ppm(60deg))\nprint(mm(1270000), mm(-5), glob(\"C*\", \"C12\"), starts_with(\"abc\", \"ab\"), ends_with(\"abc\", \"bc\"), contains([1, 2], 2), contains(\"abc\", \"d\"))\nprint(sort([3, 1, 2]), sort([\"b\", \"a\"]), reverse([1, 2]), concat([1], [2]), push([1], 2), range(2, 5), split(\"a,b\", \",\"), lines(\"x\\ny\\n\"), int(\" 42 \"), repeat(\"ab\", 2), lower(\"AB\"), deg(2))",
        &ScriptSettings::default(),
    );
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(
        lines(&o),
        vec![
            "4123105 11 -11 4 2 3",
            "10000000 10000000 5000000 500000 500000",
            "1.27 -0.000005 true true true true false",
            "[1, 2, 3] [a, b] [2, 1] [1, 2] [1, 2] [2, 3, 4] [a, b] [x, y] 42 abab ab 2000000",
        ]
    );
}

// ----- the board -----

#[test]
fn the_operations_of_the_board_change_what_they_say() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "move_to(\"R1\", 5mm, 6mm)\nmove_by(\"R2\", 1mm, -1mm)\nrotate_to(\"R1\", 45deg)\nrotate_by(\"R2\", 90deg)\nset_value(\"R1\", \"22k\")\nrename(\"GND\", \"GROUND\")",
    );
    let b = o.board.as_ref().unwrap();
    let r1 = &b.components[&fx.r1];
    assert_eq!(
        (r1.at, r1.rotation.get(), r1.value.as_str()),
        (at(5, 6), 45_000_000, "22k")
    );
    let r2 = &b.components[&fx.r2];
    assert_eq!((r2.at, r2.rotation.get()), (at(21, 9), 180_000_000));
    assert_eq!(b.nets[&fx.gnd].name, "GROUND");
    assert_eq!(o.operations, 6);
    assert_eq!(o.history_steps(), 1);
}

#[test]
fn a_script_is_one_step_of_the_history_and_one_undo_gives_the_board_back() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "move_to(\"R1\", 1mm, 2mm)\nrename(\"R2\", \"R20\")\nremove(select(\"track\", \"GND\")[0])\nvia(\"VCC\", 12mm, 12mm)",
    );
    let Plan::Board { batch, undo } = &o.plan else {
        panic!("{:?}", o.plan)
    };
    let mut board = fx.board.clone();
    let mut history = History::new();
    history
        .execute(&mut board, Operation::clone(batch))
        .unwrap();
    assert_eq!(&board, o.board.as_ref().unwrap());
    assert_eq!(history.done().len(), 1);
    assert!(history.undo(&mut board));
    assert_eq!(board.to_bytes().unwrap(), fx.board.to_bytes().unwrap());
    let mut again = o.board.clone().unwrap();
    again.apply(undo).unwrap();
    assert_eq!(again.to_bytes().unwrap(), fx.board.to_bytes().unwrap());
}

#[test]
fn a_script_that_fails_in_the_middle_changes_nothing() {
    let fx = fixture();
    let before = fx.board.to_bytes().unwrap();
    let d = failed(
        &fx.board,
        "move_to(\"R1\", 1mm, 1mm)\nrename(\"R1\", \"R2\")",
    );
    assert_eq!((d.code, d.line, d.col), (c::REFUSED, 3, 1));
    assert_eq!(fx.board.to_bytes().unwrap(), before);
    assert_eq!(
        failed(
            &fx.board,
            "move_to(\"R1\", 1mm, 1mm)\nmove_to(\"X9\", 0, 0)"
        )
        .code,
        c::NOT_FOUND
    );
    assert_eq!(
        failed(&fx.board, "move_to(\"R1\", 1mm, 1mm)\nassert(false)").code,
        c::ASSERTION
    );
}

#[test]
fn objects_are_made_and_found_again() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "let r = place(\"R9\", \"R0603\", 5mm, 5mm, 90deg, \"4k7\")\nlet t = route(\"GND\", \"Top\", 0.2mm, [0mm, 0mm, 5mm, 0mm, 5mm, 5mm])\nlet v = via(\"VCC\", 12mm, 12mm)\nlet z = zone(\"GND\", \"Top\", [0mm, 0mm, 10mm, 0mm, 10mm, 10mm], 0.3mm)\nconnect(r, \"1\", \"SIG\")\nprint(r.refdes, r.value, r.rot, r.pads)\nprint(t.net, t.layer, t.width, t.length, t.segments)\nprint(v.diameter, v.drill, v.x)\nprint(z.points, z.clearance, names(select(\"net\", \"S*\")), select(\"net\", \"SIG\")[0].pads)\nprint(exists(r), net_length(\"GND\"), pad_nets(r))",
    );
    assert_eq!(
        lines(&o),
        vec![
            "R9 4k7 90000000 2",
            "GND Top 200000 10000000 2",
            "600000 300000 12000000",
            "3 300000 [SIG] 1",
            "true 20000000 [SIG]",
        ]
    );
    let b = o.board.as_ref().unwrap();
    assert_eq!(b.components.len(), 3);
    assert_eq!(b.tracks.len(), 2);
    assert_eq!(b.vias.len(), 1);
    assert_eq!(b.zones.len(), 1);
}

#[test]
fn the_operations_that_are_not_allowed_are_refused_with_their_reason() {
    let fx = fixture();
    for (body, code) in [
        ("place(\"R1\", \"R0603\", 0, 0)", c::REFUSED),
        ("place(\"X\", \"NOFOOT\", 0, 0)", c::NOT_FOUND),
        (
            "route(\"GND\", \"Nope\", 1mm, [0, 0, 1mm, 0])",
            c::NOT_FOUND,
        ),
        (
            "route(\"GND\", \"tPlace\", 1mm, [0, 0, 1mm, 0])",
            c::REFUSED,
        ),
        ("route(\"GND\", \"Top\", 1mm, [0, 0])", c::TYPE_MISMATCH),
        (
            "route(\"GND\", \"Top\", 1mm, [0, 0, 0, 0])",
            c::TYPE_MISMATCH,
        ),
        ("via(\"GND\", 0, 0, 100um, 200um)", c::TYPE_MISMATCH),
        ("connect(\"R1\", \"9\", \"GND\")", c::NOT_FOUND),
        ("rename(\"R1\", \"\")", c::TYPE_MISMATCH),
        ("rename(\"VCC\", \"GND\")", c::REFUSED),
        ("remove(5)", c::TYPE_MISMATCH),
        ("move_to(5, 0, 0)", c::TYPE_MISMATCH),
        (
            "renumber(select(\"net\", \"*\"), \"R\", 1)",
            c::TYPE_MISMATCH,
        ),
        ("move_by(\"R1\", 1073741823, 0)", c::ARITHMETIC),
        ("assign_part(\"R1\", \"RES-10K\")", c::PARTS_BASE),
    ] {
        assert_eq!(failed(&fx.board, body).code, code, "{body}");
    }
}

#[test]
fn removing_and_mirroring_keep_the_nets_of_the_pads() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "mirror(\"R1\")\nlet r = select(\"component\", \"R1\")[0]\nprint(r.side, pad_nets(r))",
    );
    assert_eq!(lines(&o), vec!["bottom [GND, VCC]"]);
    let o = done(
        &fx.board,
        "remove(\"R2\")\nremove(select(\"track\", \"*\")[0])",
    );
    let b = o.board.as_ref().unwrap();
    assert!(!b.components.contains_key(&fx.r2) && b.tracks.is_empty());
    assert_eq!(o.history_steps(), 1);
    let o = done(
        &fx.board,
        "let r = select(\"component\", \"R2\")[0]\nremove(r)\nprint(exists(r))",
    );
    assert_eq!(lines(&o), vec!["false"]);
    assert_eq!(
        failed(
            &fx.board,
            "let r = select(\"component\", \"R2\")[0]\nremove(r)\nremove(r)"
        )
        .code,
        c::NOT_FOUND
    );
}

#[test]
fn renumbering_swaps_names_without_a_clash_and_refuses_a_name_in_use() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "let swapped = renumber([select(\"component\", \"R2\")[0], select(\"component\", \"R1\")[0]], \"R\", 1)\nprint(swapped, select(\"component\", \"R1\")[0].value, select(\"component\", \"R2\")[0].value)",
    );
    assert_eq!(lines(&o), vec!["2 1k 10k"]);
    let b = with_components(fx.board.clone(), &[("K1", 0, 0)]);
    let d = failed(&b, "renumber([select(\"component\", \"K1\")[0]], \"R\", 2)");
    assert_eq!(d.code, c::REFUSED);
    let o = done(&b, "renumber([select(\"component\", \"K1\")[0]], \"R\", 3)");
    assert_eq!(o.operations, 2);
}

#[test]
fn queries_filter_by_net_layer_rectangle_kind_value_and_position() {
    let board = with_components_fixture();
    let o = done(
        &board,
        "let all = sort_refdes(select(\"component\", \"*\"))\nprint(names(filter_net(all, \"VCC\")), names(filter_net(all, \"G*\")))\nprint(names(filter_rect(all, 0mm, 0mm, 12mm, 12mm)), names(filter_rect(all, 12mm, 12mm, 0mm, 0mm)))\nprint(names(filter_layer(select(\"track\", \"*\"), \"top\")), len(filter_layer(select(\"track\", \"*\"), \"bottom\")))\nprint(len(filter_kind(all, \"track\")), len(filter_kind(all, \"component\")))\nprint(names(filter_value(all, \"1*\")), names(sort_position(all, \"yx\")))\nprint(names(filter_layer(all, \"top\")), len(filter_layer(all, \"bottom\")))\nprint(distance(all[2], all[3]), names([nearest(all[0], all)]), names(sort_refdes(all)))\nprint(outline_bounds(), len(layers()), drc_count() >= 0, drc_errors() >= 0, ratsnest_count() >= 0)",
    );
    assert_eq!(
        lines(&o),
        vec![
            "[R1] [R1, R2]",
            "[B2, R1] [B2, R1]",
            "[track(GND)] 0",
            "0 4",
            "[R1, R2] [B2, R1, R2, A9]",
            "[A9, B2, R1, R2] 0",
            "10000000 [R2] [A9, B2, R1, R2]",
            "[0, 0, 40000000, 30000000] 4 true true true",
        ]
    );
}

fn with_components_fixture() -> Board {
    with_components(fixture().board, &[("A9", 40, 20), ("B2", 5, 5)])
}

#[test]
fn the_rule_check_is_a_number_a_script_can_test() {
    let fx = fixture();
    let settings = ScriptSettings::default();
    let program = parse_program(
        &source(
            "print(drc_errors())\nassert(drc_errors() == 0, \"clean\")\nprint(drc_errors() > 0)",
        ),
        &settings,
    )
    .unwrap();
    let ctx = RunContext::new(&settings);
    let o = run(&program, Doc::Board(&fx.board), &ctx);
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(lines(&o), vec!["0", "false"]);
    let mut strict = RunContext::new(&settings);
    strict.rules = RuleValues {
        min_track_width: Nm::new(400_000).unwrap(),
        ..RuleValues::default()
    };
    let o = run(&program, Doc::Board(&fx.board), &strict);
    assert_eq!(o.diagnostics[0].code, c::ASSERTION);
    let program = parse_program(&source("print(drc_errors() > 0)"), &settings).unwrap();
    let o = run(&program, Doc::Board(&fx.board), &strict);
    assert_eq!(lines(&o), vec!["true"]);
}

#[test]
fn panelizing_replaces_the_board_and_has_no_batch() {
    let fx = fixture();
    let o = done(
        &fx.board,
        "let n = panelize(2, 3, 2mm)\nprint(n, len(select(\"component\", \"*\")))",
    );
    assert_eq!(lines(&o), vec!["6 12"]);
    assert_eq!(o.plan, Plan::Replaced);
    assert_eq!(o.history_steps(), 1);
    assert!(o.to_json().contains("\"replaced\": true"));
    assert_eq!(failed(&fx.board, "panelize(0, 1, 2mm)").code, c::ARITHMETIC);
    assert_eq!(
        failed(&fx.board, "panelize(1, 1, -1mm)").code,
        c::ARITHMETIC
    );
}

#[test]
fn a_part_of_the_base_sets_the_value_and_names_the_footprint() {
    let fx = fixture();
    let db = PartsDb {
        parts: vec![
            BasePart {
                id: "RES-10K".to_owned(),
                value: "10k".to_owned(),
                footprint: "R0603".to_owned(),
                ..BasePart::default()
            },
            BasePart {
                id: "EMPTY".to_owned(),
                ..BasePart::default()
            },
        ],
    };
    let settings = ScriptSettings::default();
    let mut ctx = RunContext::new(&settings);
    ctx.parts = Some(&db);
    let program = parse_program(
        &source("let f = assign_part(\"R2\", \"RES-10K\")\nprint(f, select(\"component\", \"R2\")[0].value)"),
        &settings,
    )
    .unwrap();
    let o = run(&program, Doc::Board(&fx.board), &ctx);
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(lines(&o), vec!["R0603 10k"]);
    for body in [
        "assign_part(\"R2\", \"NOPE\")",
        "assign_part(\"R2\", \"EMPTY\")",
    ] {
        let program = parse_program(&source(body), &settings).unwrap();
        let o = run(&program, Doc::Board(&fx.board), &ctx);
        assert_eq!(o.diagnostics[0].code, c::PARTS_BASE, "{body}");
    }
}

#[test]
fn the_same_script_on_the_same_board_gives_the_same_bytes_and_report() {
    let fx = fixture();
    let body = "place(\"R9\", \"R0603\", 5mm, 5mm)\nroute(\"SIG\", \"Top\", 0.2mm, [0, 0, 1mm, 0])\nprint(\"ok\")";
    let (a, b) = (done(&fx.board, body), done(&fx.board, body));
    assert_eq!(
        a.board.as_ref().unwrap().to_bytes().unwrap(),
        b.board.as_ref().unwrap().to_bytes().unwrap()
    );
    assert_eq!(a.to_text(Lang::En), b.to_text(Lang::En));
    assert_eq!(a.to_json(), b.to_json());
    assert_eq!(a.plan, b.plan);
}

#[test]
fn the_report_is_written_as_text_and_json() {
    let fx = fixture();
    let o = done(&fx.board, "print(\"hi\")\nmove_to(\"R1\", 0, 0)");
    assert_eq!(
        o.to_text(Lang::En),
        format!(
            "result: carried out\nsteps: {}; operations: 1; history steps: 1\nhi\n",
            o.steps
        )
    );
    let json: serde_json::Value = serde_json::from_str(&o.to_json()).unwrap();
    assert_eq!(json["schema"], "phoenix-script-run/2");
    assert_eq!(json["api"], "phoenix-script/2");
    assert_eq!(json["ok"], true);
    assert_eq!(json["operations"], 1);
    assert_eq!(json["output"][0], "hi");
    let bad = on_board(&fx.board, "print(zzz)");
    let text = bad.to_text(Lang::Es);
    assert!(text.starts_with("result: stopped, nothing changed\n"));
    assert!(text.contains("PHOENIX-SCRIPT-045: La variable no está definida [2:7]: zzz"));
    let json: serde_json::Value = serde_json::from_str(&bad.to_json()).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["diagnostics"][0]["code"], "PHOENIX-SCRIPT-045");
    assert_eq!(json["diagnostics"][0]["line"], 2);
    assert_eq!(json["diagnostics"][0]["column"], 7);
}

// ----- the schematic and the library -----

#[test]
fn the_operations_of_the_schematic_are_one_step_of_its_editor() {
    let original = schematic();
    let settings = ScriptSettings::default();
    let program = parse_program(
        &source("move_to(\"R1\", 30mm, 12mm)\nmove_by(\"R2\", 1mm, 1mm)\nrename(\"R2\", \"R5\")\nset_value(\"R1\", \"4k7\")\nrotate_by(\"R1\", 90deg)\nmirror(\"R5\")\nprint(select(\"part\", \"R*\")[0].x, select(\"part\", \"R5\")[0].y, names(select(\"part\", \"*\")))"),
        &settings,
    )
    .unwrap();
    let ctx = RunContext::new(&settings);
    let o = run(&program, Doc::Schematic(&original), &ctx);
    assert!(o.ok, "{:?}", o.diagnostics);
    let Plan::Schematic { batch } = &o.plan else {
        panic!("{:?}", o.plan)
    };
    let mut editor = Editor::new(original.clone());
    editor.apply(batch.as_ref()).unwrap();
    assert_eq!(editor.undo_depth(), 1);
    assert_eq!(editor.schematic(), o.schematic.as_ref().unwrap());
    assert!(editor.undo());
    assert_eq!(editor.schematic(), &original);
    assert_eq!(o.operations, 6);
    assert_eq!(o.history_steps(), 1);
    assert!(o.board.is_none() && o.library.is_none());
}

#[test]
fn schematic_scripts_place_remove_renumber_and_count_findings() {
    let original = schematic();
    let settings = ScriptSettings::default();
    let program = parse_program(
        &source("let p = place_part(\"R7\", \"R\", 30mm, 10mm, \"4k7\")\nprint(p.refdes, p.value, p.symbol, p.pins)\nrenumber(sort_position(select(\"part\", \"R*\"), \"xy\"), \"R\", 10)\nprint(names(sort_refdes(select(\"part\", \"*\"))))\nremove(\"R10\")\nlabel(\"VCC\", 5mm, 5mm)\ndraw_wire([0mm, 0mm, 5mm, 0mm])\nprint(len(select(\"part\", \"*\")), erc_count() >= 0)"),
        &settings,
    )
    .unwrap();
    let ctx = RunContext::new(&settings);
    let o = run(&program, Doc::Schematic(&original), &ctx);
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(lines(&o), vec!["R7 4k7 R 2", "[R10, R11, R12]", "2 true"]);
    let failed = run(
        &parse_program(&source("rename(\"R1\", \"R2\")"), &settings).unwrap(),
        Doc::Schematic(&original),
        &ctx,
    );
    assert_eq!(failed.diagnostics[0].code, c::REFUSED);
    assert!(failed.schematic.is_none());
    let bad = run(
        &parse_program(&source("rotate_by(\"R1\", 45deg)"), &settings).unwrap(),
        Doc::Schematic(&original),
        &ctx,
    );
    assert_eq!(bad.diagnostics[0].code, c::TYPE_MISMATCH);
}

#[test]
fn the_operations_of_the_library_are_all_or_nothing() {
    let lib = Library::empty();
    let settings = ScriptSettings::default();
    let ctx = RunContext::new(&settings);
    let program = parse_program(
        &source("lib_create(\"package\", \"A\")\nlib_add_pad(\"A\", \"1\", 0, 0, 800um, 1.4mm)\nlib_add_smd(\"A\", \"2\", 2mm, 0, 1mm, 1mm)\nlib_create(\"symbol\", \"S\")\nlib_rename(\"package\", \"A\", \"B\")\nprint(lib_names(\"package\", \"*\"), lib_pad_count(\"B\"), lib_names(\"symbol\", \"S*\"))\nlib_delete(\"symbol\", \"S\")\nprint(lib_names(\"symbol\", \"*\"))"),
        &settings,
    )
    .unwrap();
    let o = run(&program, Doc::Library(&lib), &ctx);
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(lines(&o), vec!["[B] 2 [S]", "[]"]);
    let Plan::Library { ops } = &o.plan else {
        panic!("{:?}", o.plan)
    };
    assert_eq!(ops.len(), 6);
    let replayed = apply_all(&lib, ops).unwrap();
    assert_eq!(&replayed, o.library.as_ref().unwrap());
    let twice = parse_program(
        &source("lib_create(\"package\", \"A\")\nlib_create(\"package\", \"A\")"),
        &settings,
    )
    .unwrap();
    let o = run(&twice, Doc::Library(&lib), &ctx);
    assert_eq!(o.diagnostics[0].code, c::REFUSED);
    assert!(o.library.is_none());
    let o = run(
        &parse_program(&source("lib_pad_count(\"none\")"), &settings).unwrap(),
        Doc::Library(&lib),
        &ctx,
    );
    assert_eq!(o.diagnostics[0].code, c::NOT_FOUND);
}

// ----- the sandbox -----

fn data_settings() -> ScriptSettings {
    ScriptSettings {
        data_dir: "data".to_owned(),
        ..ScriptSettings::default()
    }
}

fn run_in(dir: &Path, body: &str, settings: &ScriptSettings) -> Outcome {
    let program = parse_program(&source(body), settings).unwrap();
    let mut ctx = RunContext::new(settings);
    ctx.base_dir = Some(dir);
    run(&program, Doc::None, &ctx)
}

#[test]
fn files_are_unreachable_without_a_data_folder() {
    let dir = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        "print(read_text(\"a.txt\"))",
        &ScriptSettings::default(),
    );
    assert_eq!(o.diagnostics[0].code, c::DATA_PATH);
    let o = run_in(
        dir.path(),
        "print(data_enabled())",
        &ScriptSettings::default(),
    );
    assert_eq!(lines(&o), vec!["false"]);
    let o = run_in(dir.path(), "print(data_enabled())", &data_settings());
    assert_eq!(lines(&o), vec!["true"]);
    let with_folder = data_settings();
    let program = parse_program(&source("print(read_text(\"a.txt\"))"), &with_folder).unwrap();
    let ctx = RunContext::new(&with_folder);
    let o = run(&program, Doc::None, &ctx);
    assert_eq!(o.diagnostics[0].code, c::DATA_PATH);
}

#[test]
fn paths_outside_the_data_folder_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("secret.txt"), "no").unwrap();
    for path in [
        "../secret.txt",
        "a/../../secret.txt",
        "..\\\\secret.txt",
        "/etc/passwd",
        "\\\\windows\\\\x",
        "C:\\\\x.txt",
        "c:x.txt",
        "",
        ".",
        "//server/share/x",
    ] {
        let o = run_in(
            dir.path(),
            &format!("read_text(\"{path}\")"),
            &data_settings(),
        );
        assert!(!o.ok, "{path:?}");
        assert_eq!(o.diagnostics[0].code, c::DATA_PATH, "{path:?}");
        let o = run_in(
            dir.path(),
            &format!("write_text(\"{path}\", \"x\")"),
            &data_settings(),
        );
        assert_eq!(o.diagnostics[0].code, c::DATA_PATH, "{path:?}");
    }
    let escaping = ScriptSettings {
        data_dir: "../elsewhere".to_owned(),
        ..ScriptSettings::default()
    };
    let o = run_in(dir.path(), "read_text(\"a.txt\")", &escaping);
    assert_eq!(o.diagnostics[0].code, c::DATA_PATH);
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_data_folder_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("secret.txt"), "no").unwrap();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("data").join("out")).unwrap();
    let o = run_in(
        dir.path(),
        "read_text(\"out/secret.txt\")",
        &data_settings(),
    );
    assert_eq!(o.diagnostics[0].code, c::DATA_PATH);
}

#[test]
fn a_data_file_is_read_bounded_and_a_write_waits_for_the_apply() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("in.txt"), "hello\nworld\n").unwrap();
    std::fs::write(dir.path().join("data").join("big.txt"), "0123456789").unwrap();
    let o = run_in(
        dir.path(),
        "print(lines(read_text(\"in.txt\")))\nprint(write_text(\"sub/out.txt\", \"abc\"))",
        &data_settings(),
    );
    assert!(o.ok, "{:?}", o.diagnostics);
    assert_eq!(lines(&o), vec!["[hello, world]", "3"]);
    assert_eq!(o.writes.len(), 1);
    assert_eq!(o.writes[0].relative, "sub/out.txt");
    assert!(!dir.path().join("data").join("sub").join("out.txt").exists());
    assert!(o
        .to_text(Lang::En)
        .contains("file: sub/out.txt (3 bytes)\n"));
    files::write_new(&o.writes[0].path, &o.writes[0].bytes).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("data").join("sub").join("out.txt")).unwrap(),
        "abc"
    );
    let again = run_in(
        dir.path(),
        "write_text(\"sub/out.txt\", \"x\")",
        &data_settings(),
    );
    assert_eq!(again.diagnostics[0].code, c::DATA_LIMIT);
    let twice = run_in(
        dir.path(),
        "write_text(\"new.txt\", \"x\")\nwrite_text(\"new.txt\", \"y\")",
        &data_settings(),
    );
    assert_eq!(twice.diagnostics[0].code, c::DATA_LIMIT);
    assert!(twice.writes.is_empty());
    let small = ScriptSettings {
        max_data_bytes: 5,
        ..data_settings()
    };
    let o = run_in(dir.path(), "read_text(\"big.txt\")", &small);
    assert_eq!(o.diagnostics[0].code, c::DATA_LIMIT);
    let o = run_in(dir.path(), "write_text(\"w.txt\", \"123456\")", &small);
    assert_eq!(o.diagnostics[0].code, c::DATA_LIMIT);
    let o = run_in(dir.path(), "read_text(\"missing.txt\")", &data_settings());
    assert_eq!(o.diagnostics[0].code, c::DATA_LIMIT);
    let one = ScriptSettings {
        max_data_files: 1,
        ..data_settings()
    };
    let o = run_in(
        dir.path(),
        "read_text(\"in.txt\")\nread_text(\"in.txt\")\nread_text(\"big.txt\")",
        &one,
    );
    assert_eq!(o.diagnostics[0].code, c::DATA_LIMIT);
    assert_eq!(o.diagnostics[0].line, 4);
}

// ----- the settings and the codes -----

#[test]
fn the_new_keys_of_the_settings_are_read_and_checked() {
    let s = ScriptSettings::parse(
        "max_steps = 500\nmax_loop_iterations = 7\ndata_dir = out\nmax_data_files = 2\nmax_output_bytes = 99",
    )
    .unwrap();
    assert_eq!(
        (
            s.max_steps,
            s.max_loop_iterations,
            s.data_dir.as_str(),
            s.max_data_files,
            s.max_output_bytes
        ),
        (500, 7, "out", 2, 99)
    );
    for bad in [
        "max_steps = 0",
        "max_value_cells = 0",
        "max_list_len = x",
        "max_text_bytes = 0",
    ] {
        assert!(ScriptSettings::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn every_new_code_is_written_in_the_eight_languages() {
    for (code, _) in c::REGISTRY {
        for lang in Lang::ALL {
            let d = crate::codes::Diag::new(*code, 1, 1, "x");
            let m = d.message(lang);
            assert!(m.starts_with(*code), "{m}");
            assert!(m.len() > code.len() + 8, "{lang:?} {m}");
        }
    }
    assert!(crate::codes::Diag::new(c::STEP_LIMIT, 1, 1, "")
        .message(Lang::Ja)
        .contains("ステップ"));
}

// ----- the example scripts -----

fn board_for(name: &str) -> Board {
    let mut board = fixture().board;
    match name {
        "led_matrix" => {
            for f in board.footprints.values_mut() {
                f.name = "LED0603".to_owned();
            }
            board
        }
        "renumber_by_position" => {
            with_components(board, &[("C7", 30, 10), ("C3", 10, 10), ("C9", 20, 10)])
        }
        "decoupling_check" => with_components(
            board,
            &[
                ("U1", 20, 20),
                ("C1", 22, 20),
                ("C2", 30, 20),
                ("C3", 21, 24),
            ],
        ),
        _ => {
            board.name = "demo".to_owned();
            board
        }
    }
}

#[test]
fn every_example_gives_the_output_that_is_written_next_to_it() {
    assert!(TEMPLATES.len() >= 7);
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("data")).unwrap();
    for t in TEMPLATES {
        let settings = data_settings();
        let program =
            parse_program(t.source, &settings).unwrap_or_else(|d| panic!("{}: {d:?}", t.name));
        assert_eq!(program.target, Some(t.target), "{}", t.name);
        let mut ctx = RunContext::new(&settings);
        ctx.base_dir = Some(dir.path());
        let library = Library::empty();
        let board = board_for(t.name);
        let doc = match t.target {
            Target::Board => Doc::Board(&board),
            Target::Library => Doc::Library(&library),
            Target::Schematic => unreachable!(),
        };
        let o = run(&program, doc, &ctx);
        assert!(o.ok, "{}: {:?}", t.name, o.diagnostics);
        let mut got = o.output.join("\n");
        got.push('\n');
        assert_eq!(got, expected_output(t), "{}", t.name);
    }
}

#[test]
fn the_examples_that_edit_leave_the_results_the_summaries_promise() {
    let settings = ScriptSettings::default();
    let ctx = RunContext::new(&settings);
    let t = crate::v2::templates::find("led_matrix").unwrap();
    let board = board_for("led_matrix");
    let o = run(
        &parse_program(t.source, &settings).unwrap(),
        Doc::Board(&board),
        &ctx,
    );
    let b = o.board.unwrap();
    let d6 = b.components.values().find(|c| c.refdes == "D6").unwrap();
    assert_eq!(
        (d6.at.x_nm.get(), d6.at.y_nm.get(), d6.value.as_str()),
        (10_000_000, 7_500_000, "LED")
    );
    assert_eq!(b.components.len(), 8);
    let t = crate::v2::templates::find("renumber_by_position").unwrap();
    let board = board_for("renumber_by_position");
    let o = run(
        &parse_program(t.source, &settings).unwrap(),
        Doc::Board(&board),
        &ctx,
    );
    let b = o.board.unwrap();
    let mut names: Vec<&str> = b.components.values().map(|c| c.refdes.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["C1", "C2", "C3", "R1", "R2"]);
    let t = crate::v2::templates::find("header_footprint").unwrap();
    let o = run(
        &parse_program(t.source, &settings).unwrap(),
        Doc::Library(&Library::empty()),
        &ctx,
    );
    let lib = o.library.unwrap();
    assert_eq!(lib.package("HEADER_1X6").unwrap().pads().count(), 6);
    let t = crate::v2::templates::find("circle_placement").unwrap();
    let board = board_for("circle_placement");
    let o = run(
        &parse_program(t.source, &settings).unwrap(),
        Doc::Board(&board),
        &ctx,
    );
    let b = o.board.unwrap();
    let d3 = b.components.values().find(|c| c.refdes == "D3").unwrap();
    assert_eq!(d3.rotation.get(), 180_000_000);
}

#[test]
fn the_bill_of_materials_is_not_written_without_a_data_folder_and_is_pending_with_one() {
    let settings = ScriptSettings::default();
    let ctx = RunContext::new(&settings);
    let t = crate::v2::templates::find("bom_resistors").unwrap();
    let board = fixture().board;
    let o = run(
        &parse_program(t.source, &settings).unwrap(),
        Doc::Board(&board),
        &ctx,
    );
    assert!(o.ok);
    assert_eq!(
        o.output.last().map(String::as_str),
        Some("data_dir is not set: the file is not written")
    );
    assert!(o.writes.is_empty());
    let dir = tempfile::tempdir().unwrap();
    let with = data_settings();
    let mut ctx = RunContext::new(&with);
    ctx.base_dir = Some(dir.path());
    let o = run(
        &parse_program(t.source, &with).unwrap(),
        Doc::Board(&board),
        &ctx,
    );
    assert_eq!(o.writes.len(), 1);
    assert_eq!(
        String::from_utf8(o.writes[0].bytes.clone()).unwrap(),
        "designator,value,footprint\nR1,10k,R0603\nR2,1k,R0603\n"
    );
    assert!(!dir.path().join("data").exists());
}

#[test]
fn every_example_passes_the_static_check_with_its_target() {
    let settings = ScriptSettings::default();
    for t in TEMPLATES {
        let r = check_text(t.source, &settings);
        assert!(r.ok(), "{}: {:?}", t.name, r.errors);
        assert_eq!(r.target, Some(t.target), "{}", t.name);
        assert!(r.warnings.is_empty(), "{}: {:?}", t.name, r.warnings);
        assert!(!t.summary.is_empty() && t.summary.len() < 120, "{}", t.name);
    }
    let names: Vec<&str> = TEMPLATES.iter().map(|t| t.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}
