// SPDX-License-Identifier: GPL-3.0-or-later
//! The run of scripts on a board made by hand: the effect of each command, the refusals, the
//! all-or-nothing rule, the undo, the determinism, the limits and the file commands.

use std::collections::BTreeMap;
use std::path::Path;

use eh3d_phoenix_board::{Board, History, Id, Segment, SilkItem};
use eh3d_phoenix_model::{Nm, Udeg};
use eh3d_phoenix_rules::RuleValues;
use eh3d_phoenix_schematic::Schematic;

use crate::codes::{
    ARGUMENT_INVALID, C_PICK, FILE_FAILED, LAYER_KIND, LAYER_UNKNOWN, LIMIT_EXCEEDED, NESTING,
    NUMBER_INVALID, OBJECT_NOT_FOUND, OPERATION_REFUSED, PATH_REFUSED, ROTATION_INVALID,
    SYNTAX_STRING, UNKNOWN_COMMAND, WRONG_TARGET,
};
use crate::exec::{commit_writes, run_board, run_schematic, Run, RunContext};
use crate::parse::parse_script;
use crate::settings::ScriptSettings;
use crate::testdata::{at, fixture, Fixture};
use crate::values::Unit;

fn run_with(fx: &Fixture, text: &str, settings: &ScriptSettings) -> Run {
    let parsed = parse_script(text, settings);
    let ctx = RunContext::new(settings);
    run_board(&fx.board, &parsed, &ctx)
}

fn run(fx: &Fixture, text: &str) -> Run {
    run_with(fx, text, &ScriptSettings::default())
}

fn ok(fx: &Fixture, text: &str) -> Run {
    let r = run(fx, text);
    assert!(r.ok, "{text}: {:?}", r.diagnostics);
    r
}

fn error_code(fx: &Fixture, text: &str) -> &'static str {
    let r = run(fx, text);
    assert!(!r.ok, "{text} should fail");
    assert!(r.board.is_none() && r.batch.is_none() && r.undo.is_none());
    r.diagnostics[0].code
}

fn added<'a, T>(before: &BTreeMap<Id, T>, after: &'a BTreeMap<Id, T>) -> Vec<&'a T> {
    after
        .iter()
        .filter(|(id, _)| !before.contains_key(id))
        .map(|(_, v)| v)
        .collect()
}

fn board_of(r: &Run) -> &Board {
    r.board.as_ref().unwrap()
}

#[test]
fn grid_sets_the_unit_the_size_and_the_switch() {
    let fx = fixture();
    let s = ok(&fx, "GRID mil 50 off").summary.unwrap();
    assert_eq!(s.unit, Unit::Mil);
    assert_eq!(s.grid_nm, 1_270_000);
    assert!(!s.grid_on);
    let s = ok(&fx, "GRID mm 0.5\nGRID mil 100\nGRID last")
        .summary
        .unwrap();
    assert_eq!((s.unit, s.grid_nm, s.grid_on), (Unit::Mm, 500_000, true));
    let s = ok(&fx, "GRID mm 0.5\nGRID default").summary.unwrap();
    assert_eq!(s.grid_nm, 1_270_000);
    let s = ok(&fx, "GRID mil 100\nCHANGE WIDTH 8").summary.unwrap();
    assert_eq!(s.width_nm, 203_200);
    let s = ok(&fx, "GRID lines").summary.unwrap();
    assert_eq!((s.unit, s.grid_nm), (Unit::Mm, 1_270_000));
    assert_eq!(error_code(&fx, "GRID mm 2grid"), NUMBER_INVALID);
    assert_eq!(error_code(&fx, "GRID mm 0"), NUMBER_INVALID);
}

#[test]
fn layer_and_change_set_the_defaults_of_what_is_drawn_next() {
    let fx = fixture();
    let s = ok(
        &fx,
        "LAYER Top\nCHANGE WIDTH 0.5\nCHANGE DRILL 0.35\nCHANGE SIZE 2",
    )
    .summary
    .unwrap();
    assert_eq!(s.layer, Some(fx.top));
    assert_eq!(
        (s.width_nm, s.drill_nm, s.size_nm),
        (500_000, 350_000, 2_000_000)
    );
    assert_eq!(ok(&fx, "LAYER 21").summary.unwrap().layer, Some(fx.silk));
    assert_eq!(
        ok(&fx, "LAYER Dimension").summary.unwrap().layer,
        Some(fx.outline)
    );
    assert_eq!(
        ok(&fx, "LAYER bottom").summary.unwrap().layer,
        Some(fx.bottom)
    );
    assert_eq!(error_code(&fx, "LAYER 99"), LAYER_UNKNOWN);
    assert_eq!(error_code(&fx, "LAYER Inner5"), LAYER_UNKNOWN);
    assert_eq!(error_code(&fx, "CHANGE WIDTH 0"), NUMBER_INVALID);
    assert_eq!(error_code(&fx, "CHANGE WIDTH -1"), NUMBER_INVALID);
    let r = run(&fx, "GRID mm\nLAYER 99");
    assert_eq!((r.diagnostics[0].line, r.diagnostics[0].col), (2, 1));
}

#[test]
fn display_is_recorded_and_can_go_back() {
    let fx = fixture();
    let s = ok(&fx, "DISPLAY NONE Top +tPlace").summary.unwrap();
    assert!(!s.display_all);
    assert_eq!(s.display, vec!["TOP".to_owned(), "TPLACE".to_owned()]);
    let s = ok(&fx, "DISPLAY NONE Top -Top bottom").summary.unwrap();
    assert_eq!(s.display, vec!["BOTTOM".to_owned()]);
    let s = ok(&fx, "DISPLAY NONE Top\nDISPLAY ALL").summary.unwrap();
    assert!(s.display_all && s.display.is_empty());
    let s = ok(&fx, "DISPLAY NONE Top\nDISPLAY LAST").summary.unwrap();
    assert!(s.display_all && s.display.is_empty());
}

#[test]
fn a_wire_on_copper_makes_a_track_of_the_named_net() {
    let fx = fixture();
    let r = ok(&fx, "LAYER Top\nWIRE 'VCC' 0.3 (1 1) (5 1) (5 4)");
    let new = added(&fx.board.tracks, &board_of(&r).tracks);
    assert_eq!(new.len(), 1);
    let t = new[0];
    assert_eq!((t.net, t.layer, t.width.get()), (fx.vcc, fx.top, 300_000));
    assert_eq!(
        t.path,
        vec![
            Segment::Line {
                start: at(1, 1),
                end: at(5, 1)
            },
            Segment::Line {
                start: at(5, 1),
                end: at(5, 4)
            },
        ]
    );
    assert_eq!(r.operations, 1);
}

#[test]
fn a_wire_with_a_new_signal_name_makes_the_net_first() {
    let fx = fixture();
    let r = ok(&fx, "LAYER Top\nWIRE 'NEW' (1 1) (2 2)");
    assert_eq!(r.operations, 2);
    let board = board_of(&r);
    assert_eq!(board.nets.len(), 3);
    let net = board.nets.iter().find(|(_, n)| n.name == "NEW").unwrap().0;
    let new = added(&fx.board.tracks, &board.tracks);
    assert_eq!(new[0].net, *net);
    assert_eq!(new[0].width.get(), 254_000);
}

#[test]
fn a_wire_on_a_silkscreen_layer_is_a_stroke() {
    let fx = fixture();
    let r = ok(&fx, "LAYER tPlace\nWIRE 0.2 (0 0) (1 0)");
    let items: Vec<&SilkItem> = board_of(&r).silkscreen.values().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0],
        &SilkItem::Stroke {
            layer: fx.silk,
            width: Nm::new(200_000).unwrap(),
            path: vec![Segment::Line {
                start: at(0, 0),
                end: at(1, 0)
            }],
        }
    );
}

#[test]
fn a_wire_that_cannot_be_made_is_refused() {
    let fx = fixture();
    assert_eq!(error_code(&fx, "WIRE 'GND' (0 0) (1 1)"), ARGUMENT_INVALID);
    assert_eq!(
        error_code(&fx, "LAYER Top\nWIRE (0 0) (1 1)"),
        ARGUMENT_INVALID
    );
    assert_eq!(
        error_code(&fx, "LAYER Dimension\nWIRE 'GND' (0 0) (1 1)"),
        LAYER_KIND
    );
    assert_eq!(
        error_code(&fx, "LAYER Top\nWIRE 'GND' (1 1) (1 1)"),
        ARGUMENT_INVALID
    );
    assert_eq!(
        error_code(&fx, "LAYER Top\nWIRE 'GND' 0.3 (0 0) (5000mm 0)"),
        NUMBER_INVALID
    );
}

#[test]
fn a_via_joins_the_outer_copper_layers() {
    let fx = fixture();
    let r = ok(&fx, "VIA 'GND' 0.8 (3 3)");
    let vias = added(&fx.board.vias, &board_of(&r).vias);
    assert_eq!(vias.len(), 1);
    let v = vias[0];
    assert_eq!((v.net, v.at), (fx.gnd, at(3, 3)));
    assert_eq!((v.diameter.get(), v.drill.get()), (800_000, 300_000));
    assert_eq!((v.from_layer, v.to_layer), (fx.top, fx.bottom));
    assert_eq!(
        error_code(&fx, "CHANGE DRILL 0.9\nVIA 'GND' 0.8 (3 3)"),
        NUMBER_INVALID
    );
    assert_eq!(error_code(&fx, "VIA (3 3)"), ARGUMENT_INVALID);
}

#[test]
fn a_circle_is_two_half_turns_and_a_rectangle_is_four_lines() {
    let fx = fixture();
    let r = ok(&fx, "LAYER tPlace\nCIRCLE 0.2 (10 10) (12 10)");
    let half = Udeg::new(180_000_000).unwrap();
    assert_eq!(
        board_of(&r).silkscreen.values().next().unwrap(),
        &SilkItem::Stroke {
            layer: fx.silk,
            width: Nm::new(200_000).unwrap(),
            path: vec![
                Segment::Arc {
                    start: at(12, 10),
                    end: at(8, 10),
                    sweep: half
                },
                Segment::Arc {
                    start: at(8, 10),
                    end: at(12, 10),
                    sweep: half
                },
            ],
        }
    );
    let r = ok(&fx, "LAYER tPlace\nRECT (0 0) (4 3)");
    assert_eq!(
        board_of(&r).silkscreen.values().next().unwrap(),
        &SilkItem::Stroke {
            layer: fx.silk,
            width: Nm::new(254_000).unwrap(),
            path: vec![
                Segment::Line {
                    start: at(0, 0),
                    end: at(4, 0)
                },
                Segment::Line {
                    start: at(4, 0),
                    end: at(4, 3)
                },
                Segment::Line {
                    start: at(4, 3),
                    end: at(0, 3)
                },
                Segment::Line {
                    start: at(0, 3),
                    end: at(0, 0)
                },
            ],
        }
    );
    assert_eq!(error_code(&fx, "LAYER Top\nCIRCLE (0 0) (1 0)"), LAYER_KIND);
    assert_eq!(
        error_code(&fx, "LAYER tPlace\nCIRCLE (0 0) (0 0)"),
        ARGUMENT_INVALID
    );
    assert_eq!(error_code(&fx, "CIRCLE (0 0) (1 0)"), ARGUMENT_INVALID);
    assert_eq!(
        error_code(&fx, "LAYER tPlace\nRECT (0 0) (0 5)"),
        ARGUMENT_INVALID
    );
}

#[test]
fn a_text_is_drawn_with_the_height_the_stroke_and_the_rotation() {
    let fx = fixture();
    let r = ok(&fx, "LAYER tPlace\nTEXT 'HELLO' R90 (5 5)");
    assert_eq!(
        board_of(&r).silkscreen.values().next().unwrap(),
        &SilkItem::Text {
            layer: fx.silk,
            at: at(5, 5),
            text: "HELLO".to_owned(),
            height: Nm::new(1_270_000).unwrap(),
            stroke: Nm::new(101_600).unwrap(),
            rotation: Udeg::new(90_000_000).unwrap(),
            mirrored: false,
        }
    );
    let r = ok(&fx, "LAYER tPlace\nTEXT 'x' MR180 (0 0)");
    match board_of(&r).silkscreen.values().next().unwrap() {
        SilkItem::Text {
            rotation, mirrored, ..
        } => {
            assert_eq!(rotation.get(), 180_000_000);
            assert!(*mirrored);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        error_code(&fx, "LAYER tPlace\nTEXT 'x' R45 (0 0)"),
        ROTATION_INVALID
    );
    assert_eq!(
        error_code(&fx, "LAYER tPlace\nTEXT '' (0 0)"),
        ARGUMENT_INVALID
    );
}

#[test]
fn move_and_rotate_change_the_component() {
    let fx = fixture();
    let r = ok(&fx, "MOVE R1 (5 6)");
    let c = &board_of(&r).components[&fx.r1];
    assert_eq!((c.at, c.rotation.get()), (at(5, 6), 0));
    assert_eq!(error_code(&fx, "MOVE R9 (1 1)"), OBJECT_NOT_FOUND);

    let rotation =
        |text: &str, id: Id| -> i64 { board_of(&ok(&fx, text)).components[&id].rotation.get() };
    assert_eq!(rotation("ROTATE R90 R2", fx.r2), 180_000_000);
    assert_eq!(rotation("ROTATE =R45 R1", fx.r1), 45_000_000);
    assert_eq!(rotation("ROTATE R-90 R1", fx.r1), 270_000_000);
    assert_eq!(rotation("ROTATE R360 R1", fx.r1), 0);
    assert_eq!(rotation("ROTATE R90 R1 R2", fx.r1), 90_000_000);
    assert_eq!(rotation("ROTATE R90 R1 R2", fx.r2), 180_000_000);
    assert_eq!(error_code(&fx, "ROTATE R90 R9"), OBJECT_NOT_FOUND);
}

#[test]
fn delete_removes_a_component_with_its_pad_nets() {
    let fx = fixture();
    assert_eq!(fx.board.pad_nets.len(), 3);
    let r = ok(&fx, "DELETE R1");
    let board = board_of(&r);
    assert_eq!(board.components.len(), 1);
    assert!(board.components.contains_key(&fx.r2));
    assert_eq!(board.pad_nets.len(), 1);
    assert_eq!(board.tracks.len(), 1);
    assert_eq!(error_code(&fx, "DELETE R9"), OBJECT_NOT_FOUND);
}

#[test]
fn name_and_value_change_the_fields() {
    let fx = fixture();
    let r = ok(&fx, "NAME R1 R10\nVALUE R2 '4k7'");
    let board = board_of(&r);
    assert_eq!(board.components[&fx.r1].refdes, "R10");
    assert_eq!(board.components[&fx.r1].value, "10k");
    assert_eq!(board.components[&fx.r2].value, "4k7");
    let r = ok(&fx, "NAME GND AGND");
    assert_eq!(board_of(&r).nets[&fx.gnd].name, "AGND");
    assert_eq!(error_code(&fx, "NAME R1 R2"), OPERATION_REFUSED);
    assert_eq!(error_code(&fx, "NAME VCC GND"), OPERATION_REFUSED);
    assert_eq!(error_code(&fx, "NAME Q1 Q2"), OBJECT_NOT_FOUND);
    assert_eq!(error_code(&fx, "VALUE Q1 '1'"), OBJECT_NOT_FOUND);
}

#[test]
fn a_group_is_what_the_next_commands_act_on() {
    let fx = fixture();
    let r = ok(&fx, "GROUP 'GND'\nCHANGE WIDTH 0.4");
    assert_eq!(r.summary.as_ref().unwrap().selected, 1);
    assert_eq!(r.operations, 2);
    assert_eq!(board_of(&r).tracks[&fx.track].width.get(), 400_000);

    let r = ok(&fx, "GROUP 'GND'\nDELETE");
    assert!(board_of(&r).tracks.is_empty());
    assert_eq!(r.summary.unwrap().selected, 0);

    assert_eq!(ok(&fx, "GROUP ALL").summary.unwrap().selected, 3);
    assert_eq!(
        ok(&fx, "GROUP ALL\nGROUP NONE").summary.unwrap().selected,
        0
    );

    let r = ok(&fx, "GROUP R1\nROTATE R90");
    assert_eq!(board_of(&r).components[&fx.r1].rotation.get(), 90_000_000);
    assert_eq!(r.operations, 1);
    assert_eq!(error_code(&fx, "GROUP 'NOPE'"), OBJECT_NOT_FOUND);
}

#[test]
fn change_acts_on_named_objects() {
    let fx = fixture();
    let r = ok(&fx, "CHANGE WIDTH 0.4 'GND'");
    assert_eq!(board_of(&r).tracks[&fx.track].width.get(), 400_000);
    let r = ok(&fx, "CHANGE LAYER Bottom 'GND'");
    assert_eq!(board_of(&r).tracks[&fx.track].layer, fx.bottom);
    assert_eq!(error_code(&fx, "CHANGE LAYER tPlace 'GND'"), LAYER_KIND);
    let r = ok(&fx, "VIA 'GND' 0.8 (3 3)\nCHANGE DRILL 0.2 'GND'");
    assert_eq!(r.operations, 3);
    let vias = added(&fx.board.vias, &board_of(&r).vias);
    assert_eq!(vias[0].drill.get(), 200_000);
    assert_eq!(
        error_code(&fx, "VIA 'GND' 0.8 (3 3)\nCHANGE DRILL 0.9 'GND'"),
        NUMBER_INVALID
    );
    let r = ok(
        &fx,
        "LAYER tPlace\nTEXT 'A' (1 1)\nGROUP ALL\nCHANGE SIZE 2",
    );
    assert_eq!(
        board_of(&r).silkscreen.values().next().unwrap(),
        &SilkItem::Text {
            layer: fx.silk,
            at: at(1, 1),
            text: "A".to_owned(),
            height: Nm::new(2_000_000).unwrap(),
            stroke: Nm::new(160_000).unwrap(),
            rotation: Udeg::ZERO,
            mirrored: false,
        }
    );
    assert_eq!(error_code(&fx, "CHANGE WIDTH 0.4 'NOPE'"), OBJECT_NOT_FOUND);
}

#[test]
fn ratsnest_and_drc_delegate_and_change_nothing() {
    let fx = fixture();
    let r = ok(&fx, "RATSNEST");
    assert_eq!(r.messages.len(), 1);
    assert_eq!(r.operations, 0);
    assert_eq!(board_of(&r), &fx.board);

    let r = ok(&fx, "DRC");
    assert_eq!((r.messages.len(), r.failed_checks), (1, 0));

    let settings = ScriptSettings::default();
    let parsed = parse_script("DRC", &settings);
    let mut ctx = RunContext::new(&settings);
    ctx.rules = RuleValues {
        min_track_width: Nm::new(400_000).unwrap(),
        ..RuleValues::default()
    };
    let r = run_board(&fx.board, &parsed, &ctx);
    assert!(r.ok);
    assert_eq!(r.failed_checks, 1);
    assert!(!r.messages[0].is_empty());

    assert_eq!(error_code(&fx, "ERC"), WRONG_TARGET);
}

#[test]
fn a_schematic_takes_the_commands_that_do_not_edit_a_board() {
    let settings = ScriptSettings::default();
    let ctx = RunContext::new(&settings);
    let schematic = Schematic::default();
    let r = run_schematic(
        &schematic,
        &parse_script("GRID mm 0.5\nDISPLAY ALL\nSET CONFIRM YES\nERC", &settings),
        &ctx,
    );
    assert!(r.ok, "{:?}", r.diagnostics);
    assert!(r.board.is_none() && r.batch.is_none() && r.undo.is_none());
    assert_eq!(r.messages.len(), 1);
    assert_eq!(r.executed, 4);
    for text in ["LAYER Top", "WIRE (0 0) (1 1)", "RATSNEST", "DRC", "DELETE"] {
        let r = run_schematic(&schematic, &parse_script(text, &settings), &ctx);
        assert!(!r.ok, "{text}");
        assert_eq!(r.diagnostics[0].code, WRONG_TARGET, "{text}");
    }
}

#[test]
fn an_error_in_the_middle_leaves_everything_as_it_was() {
    let fx = fixture();
    let before = fx.board.to_bytes().unwrap();
    let r = run(
        &fx,
        "LAYER Top\nWIRE 'VCC' (1 1) (2 2)\nMOVE R9 (1 1)\nWIRE 'VCC' (3 3) (4 4)",
    );
    assert!(!r.ok);
    assert!(r.board.is_none() && r.batch.is_none() && r.undo.is_none());
    assert_eq!(r.diagnostics.len(), 1);
    let d = &r.diagnostics[0];
    assert_eq!((d.code, d.line, d.col), (OBJECT_NOT_FOUND, 3, 1));
    assert_eq!((r.executed, r.operations), (2, 1));
    assert_eq!(fx.board.to_bytes().unwrap(), before);
}

#[test]
fn applying_the_batch_and_then_the_undo_gives_the_original_byte_for_byte() {
    let fx = fixture();
    let script = "GRID mm 0.5\nLAYER Top\nWIRE 'NEW' 0.3 (1 1) (5 1)\nVIA 'NEW' (5 1)\n\
LAYER tPlace\nTEXT 'AB' (2 2)\nCIRCLE (10 10) (11 10)\nRECT (0 0) (3 3)\n\
MOVE R1 (7 7)\nROTATE R90 R1\nNAME R2 R20\nVALUE R20 '2k2'\n\
GROUP 'GND'\nCHANGE WIDTH 0.5\nCHANGE LAYER Bottom\nDELETE\n";
    let r = ok(&fx, script);
    let result = board_of(&r).clone();
    let original = fx.board.to_bytes().unwrap();
    assert_ne!(result.to_bytes().unwrap(), original);
    assert!(!result.tracks.contains_key(&fx.track));
    let batch = r.batch.clone().unwrap();
    let undo = r.undo.clone().unwrap();

    let mut again = fx.board.clone();
    again.apply(&batch).unwrap();
    assert_eq!(again.to_bytes().unwrap(), result.to_bytes().unwrap());
    again.apply(&undo).unwrap();
    assert_eq!(again.to_bytes().unwrap(), original);

    let mut board = fx.board.clone();
    let mut history = History::new();
    history.execute(&mut board, batch).unwrap();
    assert_eq!(board, result);
    assert!(history.undo(&mut board));
    assert_eq!(board.to_bytes().unwrap(), original);
    assert!(history.redo(&mut board));
    assert_eq!(board, result);
}

#[test]
fn deleting_a_component_is_undone_with_its_pad_nets() {
    let fx = fixture();
    let r = ok(&fx, "DELETE R1\nDELETE R2");
    let mut board = board_of(&r).clone();
    assert!(board.components.is_empty() && board.pad_nets.is_empty());
    board.apply(r.undo.as_ref().unwrap()).unwrap();
    assert_eq!(board.to_bytes().unwrap(), fx.board.to_bytes().unwrap());
}

#[test]
fn the_same_script_on_the_same_board_gives_the_same_result() {
    let fx = fixture();
    let script = "LAYER Top\nWIRE 'N1' (1 1) (2 2)\nVIA 'N2' (3 3)\nLAYER tPlace\nTEXT 'A' (1 1)";
    let a = ok(&fx, script);
    let b = ok(&fx, script);
    assert_eq!(a.batch, b.batch);
    assert_eq!(
        board_of(&a).to_bytes().unwrap(),
        board_of(&b).to_bytes().unwrap()
    );
    let c = ok(
        &fx,
        "LAYER Top\nWIRE 'N1' (1 1) (2 2)\nWIRE 'N1' (1 1) (2 2)",
    );
    let tracks = added(&fx.board.tracks, &board_of(&c).tracks);
    assert_eq!(tracks.len(), 2);
}

#[test]
fn a_script_with_a_problem_is_not_started() {
    let fx = fixture();
    let r = run(
        &fx,
        "GRID mm\nFROBNICATE\nLAYER Top\nWIRE 'GND' (1 1) (2 2)",
    );
    assert!(!r.ok);
    assert_eq!((r.executed, r.operations), (0, 0));
    assert_eq!(r.diagnostics.len(), 1);
    let d = &r.diagnostics[0];
    assert_eq!((d.code, d.line, d.col), (UNKNOWN_COMMAND, 2, 1));

    let r = run(&fx, "BOGUS\nTEXT 'oops");
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code).collect();
    assert!(codes.contains(&SYNTAX_STRING) && codes.contains(&UNKNOWN_COMMAND));
    assert_eq!(r.executed, 0);

    assert_eq!(error_code(&fx, "MOVE (1 1) (2 2)"), C_PICK);
    assert_eq!(error_code(&fx, "WIRE (0 0)"), ARGUMENT_INVALID);
    assert_eq!(error_code(&fx, "ROUTE"), crate::codes::C_KNOWN_PENDING);
}

#[test]
fn a_pending_command_can_be_skipped_with_a_warning_but_an_unknown_one_cannot() {
    let fx = fixture();
    let settings = ScriptSettings {
        skip_pending: true,
        ..ScriptSettings::default()
    };
    let r = run_with(
        &fx,
        "LAYER Top\nMOVE (1 1) (2 2)\nWIRE 'GND' (1 1) (2 2)",
        &settings,
    );
    assert!(r.ok, "{:?}", r.diagnostics);
    assert_eq!((r.executed, r.skipped), (2, 1));
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(r.warnings[0].code, C_PICK);
    assert_eq!(added(&fx.board.tracks, &board_of(&r).tracks).len(), 1);
    let r = run_with(&fx, "FROBNICATE", &settings);
    assert!(!r.ok);
}

#[test]
fn the_limits_of_the_settings_stop_the_run() {
    let fx = fixture();
    let settings = ScriptSettings {
        max_operations: 1,
        ..ScriptSettings::default()
    };
    let r = run_with(&fx, "LAYER Top\nWIRE 'NEW' (1 1) (2 2)", &settings);
    assert!(!r.ok);
    assert_eq!(r.diagnostics[0].code, LIMIT_EXCEEDED);
    assert_eq!((r.diagnostics[0].line, r.diagnostics[0].col), (2, 1));

    let settings = ScriptSettings {
        max_commands: 2,
        ..ScriptSettings::default()
    };
    let r = run_with(&fx, "GRID mm\nGRID mm\nGRID mm", &settings);
    assert!(!r.ok);
    assert_eq!(r.diagnostics[0].code, LIMIT_EXCEEDED);
}

fn run_in(fx: &Fixture, text: &str, settings: &ScriptSettings, dir: &Path) -> Run {
    let parsed = parse_script(text, settings);
    let mut ctx = RunContext::new(settings);
    ctx.base_dir = Some(dir);
    run_board(&fx.board, &parsed, &ctx)
}

fn open_settings() -> ScriptSettings {
    ScriptSettings {
        allow_write: true,
        allow_open: true,
        allow_script: true,
        allowed_dirs: vec!["out".to_owned(), "in".to_owned(), "sub".to_owned()],
        ..ScriptSettings::default()
    }
}

#[test]
fn write_notes_the_file_and_only_the_commit_writes_it_and_never_replaces() {
    let fx = fixture();
    let dir = tempfile::tempdir().unwrap();
    let s = open_settings();
    let r = run_in(&fx, "WRITE 'out/a.json'", &s, dir.path());
    assert!(r.ok, "{:?}", r.diagnostics);
    assert_eq!(r.writes.len(), 1);
    let target = dir.path().join("out").join("a.json");
    assert!(!target.exists());
    commit_writes(&r).unwrap();
    assert_eq!(
        std::fs::read(&target).unwrap(),
        fx.board.to_bytes().unwrap()
    );
    assert_eq!(commit_writes(&r).unwrap_err().code, FILE_FAILED);
    let failed = run_in(&fx, "WRITE 'out/b.json'\nFROBNICATE", &s, dir.path());
    assert!(failed.writes.is_empty());
    assert_eq!(commit_writes(&failed).unwrap_err().code, FILE_FAILED);
    assert!(!dir.path().join("out").join("b.json").exists());
}

#[test]
fn write_refuses_what_the_settings_do_not_allow() {
    let fx = fixture();
    let dir = tempfile::tempdir().unwrap();
    let s = open_settings();
    let absolute = dir.path().join("out").join("a.json");
    let absolute_script = format!("WRITE '{}'", absolute.display());
    for text in [
        "WRITE '../a.json'",
        "WRITE 'out/../../a.json'",
        "WRITE 'out/a.txt'",
        "WRITE 'a.json'",
        "WRITE 'other/a.json'",
        "WRITE 'out'",
        absolute_script.as_str(),
    ] {
        let r = run_in(&fx, text, &s, dir.path());
        assert!(!r.ok, "{text}");
        assert_eq!(r.diagnostics[0].code, PATH_REFUSED, "{text}");
        assert!(r.writes.is_empty());
    }
    let off = ScriptSettings::default();
    let r = run_in(&fx, "WRITE 'out/a.json'", &off, dir.path());
    assert_eq!(r.diagnostics[0].code, PATH_REFUSED);
    let r = run_with(&fx, "WRITE 'out/a.json'", &s);
    assert_eq!(r.diagnostics[0].code, PATH_REFUSED);
}

#[test]
fn open_replaces_the_board_and_cannot_be_undone() {
    let fx = fixture();
    let dir = tempfile::tempdir().unwrap();
    let mut other = fx.board.clone();
    other.name = "other".to_owned();
    std::fs::create_dir_all(dir.path().join("in")).unwrap();
    std::fs::write(
        dir.path().join("in").join("b.json"),
        other.to_bytes().unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("in").join("bad.json"), b"{").unwrap();
    let s = open_settings();
    let r = run_in(
        &fx,
        "OPEN 'in/b.json'\nLAYER Top\nWIRE 'GND' (1 1) (2 2)",
        &s,
        dir.path(),
    );
    assert!(r.ok, "{:?}", r.diagnostics);
    assert_eq!(board_of(&r).name, "other");
    assert_eq!(board_of(&r).tracks.len(), 2);
    assert!(r.batch.is_none() && r.undo.is_none());
    for text in ["OPEN 'in/bad.json'", "OPEN 'in/none.json'"] {
        let r = run_in(&fx, text, &s, dir.path());
        assert_eq!(r.diagnostics[0].code, FILE_FAILED, "{text}");
    }
    let small = ScriptSettings {
        max_file_bytes: 2,
        ..open_settings()
    };
    let r = run_in(&fx, "OPEN 'in/b.json'", &small, dir.path());
    assert_eq!(r.diagnostics[0].code, FILE_FAILED);
    let off = ScriptSettings {
        allow_open: false,
        ..open_settings()
    };
    let r = run_in(&fx, "OPEN 'in/b.json'", &off, dir.path());
    assert_eq!(r.diagnostics[0].code, PATH_REFUSED);
}

#[test]
fn script_runs_another_script_below_the_limits() {
    let fx = fixture();
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("inc.scr"), "LAYER Top\nWIRE 'GND' (1 1) (2 2)\n").unwrap();
    std::fs::write(sub.join("bad.scr"), "FROBNICATE\n").unwrap();
    std::fs::write(sub.join("self.scr"), "SCRIPT 'sub/self.scr'\n").unwrap();
    std::fs::write(sub.join("binary.scr"), [0xff, 0xfe, 0x41]).unwrap();
    let s = open_settings();

    let r = run_in(
        &fx,
        "SCRIPT 'sub/inc.scr'\nLAYER Top\nWIRE 'VCC' (3 3) (4 4)",
        &s,
        dir.path(),
    );
    assert!(r.ok, "{:?}", r.diagnostics);
    assert_eq!(added(&fx.board.tracks, &board_of(&r).tracks).len(), 2);
    assert_eq!(r.executed, 5);

    let r = run_in(&fx, "SCRIPT 'sub/bad.scr'", &s, dir.path());
    assert!(!r.ok);
    assert_eq!(r.diagnostics[0].code, UNKNOWN_COMMAND);
    assert!(r.diagnostics[0].detail.contains("in sub/bad.scr"));

    let r = run_in(&fx, "SCRIPT 'sub/self.scr'", &s, dir.path());
    assert_eq!(r.diagnostics[0].code, NESTING);

    for (text, code) in [
        ("SCRIPT '../x.scr'", PATH_REFUSED),
        ("SCRIPT 'sub/none.scr'", FILE_FAILED),
        ("SCRIPT 'sub/binary.scr'", FILE_FAILED),
    ] {
        let r = run_in(&fx, text, &s, dir.path());
        assert_eq!(r.diagnostics[0].code, code, "{text}");
    }
    let off = ScriptSettings {
        allow_script: false,
        ..open_settings()
    };
    let r = run_in(&fx, "SCRIPT 'sub/inc.scr'", &off, dir.path());
    assert_eq!(r.diagnostics[0].code, PATH_REFUSED);
}
