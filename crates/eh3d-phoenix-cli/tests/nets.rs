// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `nets` on a board written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

const MM: i64 = 1_000_000;

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

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn pad(number: &str, at: Point, diameter: i64) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: Nm::new(diameter).unwrap(),
        },
        at,
        rotation: Udeg::ZERO,
        drill: None,
        layers: vec![id(1)],
        roundness: 0,
        plated: true,
    }
}

/// A 40 x 30 mm board with two resistors between GND and SIG and one test point on GND. The nets
/// are GND (R1.2, R2.2, TP1.1) and SIG (R1.1, R2.1). The name of the second net is the argument.
fn board_with(signal_name: &str) -> Board {
    let mut b = Board {
        name: "nets".into(),
        outline: vec![
            pt(0, 0),
            pt(40 * MM, 0),
            pt(40 * MM, 30 * MM),
            pt(0, 30 * MM),
        ],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.nets.insert(id(2), Net { name: "GND".into() });
    b.nets.insert(
        id(3),
        Net {
            name: signal_name.into(),
        },
    );
    let mut resistor = Footprint {
        name: "R_0603".into(),
        pads: Default::default(),
    };
    resistor.pads.insert(id(11), pad("1", pt(-MM, 0), 800_000));
    resistor.pads.insert(id(12), pad("2", pt(MM, 0), 800_000));
    b.footprints.insert(id(10), resistor);
    let mut test = Footprint {
        name: "Pad15".into(),
        pads: Default::default(),
    };
    test.pads.insert(id(14), pad("1", pt(0, 0), 1_500_000));
    b.footprints.insert(id(13), test);
    for (n, refdes, x) in [(20_u32, "R1", 10 * MM), (21, "R2", 30 * MM)] {
        b.components.insert(
            id(n),
            Component {
                refdes: refdes.into(),
                value: "10k".into(),
                symbol: None,
                footprint: id(10),
                at: pt(x, 10 * MM),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(11),
            net: id(3),
        });
        b.pad_nets.push(PadNet {
            component: id(n),
            pad: id(12),
            net: id(2),
        });
    }
    b.components.insert(
        id(30),
        Component {
            refdes: "TP1".into(),
            value: String::new(),
            symbol: None,
            footprint: id(13),
            at: pt(20 * MM, 20 * MM),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: id(30),
        pad: id(14),
        net: id(2),
    });
    b.pad_nets.sort_by_key(PadNet::pad_ref);
    b
}

fn write_board(dir: &Path, b: &Board) -> String {
    let path = dir.join("board.json");
    fs::write(&path, b.to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn path_in(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}

const BOARD_NETLIST: &str = "NET GND R1.2 R2.2 TP1.1\nNET SIG R1.1 R2.1\n";

#[test]
fn the_netlist_of_a_board_is_printed_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, err) = command(&["nets", &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, BOARD_NETLIST);
    // only the board is in the folder
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    // the same bytes every time
    assert_eq!(command(&["nets", &path]).1, out);
}

#[test]
fn the_netlist_goes_to_a_new_file_and_a_file_that_exists_is_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let target = path_in(dir.path(), "board.nets");
    let (code, out, err) = command(&["nets", "--out", &target, &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, format!("written: {target} (2 nets, 5 pins)\n"));
    assert_eq!(fs::read_to_string(&target).unwrap(), BOARD_NETLIST);
    let (code, out, err) = command(&["nets", "--out", &target, &path]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.contains("PHOENIX-NETS-009"), "{err}");
    assert_eq!(fs::read_to_string(&target).unwrap(), BOARD_NETLIST);
}

#[test]
fn the_json_has_the_nets_the_counts_and_the_notes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, _) = command(&["nets", "--json", &path]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["net_count"], 2);
    assert_eq!(v["pin_count"], 5);
    assert_eq!(v["nets"][0]["name"], "GND");
    assert_eq!(v["nets"][0]["pins"][2], "TP1.1");
    assert_eq!(v["notes"].as_array().unwrap().len(), 0);
}

#[test]
fn a_name_that_cannot_be_written_is_changed_and_said_in_a_comment() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("MY SIG"));
    let (code, out, _) = command(&["nets", &path]);
    assert_eq!(code, 0);
    assert!(out.starts_with("NET GND R1.2 R2.2 TP1.1\nNET MY_SIG R1.1 R2.1\n"));
    assert!(
        out.contains("# PHOENIX-NETS-007: the net MY SIG is written as MY_SIG\n"),
        "{out}"
    );
    // the comment does not stop the file from being read, and the board equals its own netlist
    let file = write_text(dir.path(), "own.nets", &out);
    let (code, canonical, _) = command(&["nets", "--import", &file]);
    assert_eq!(code, 0);
    assert_eq!(canonical, "NET GND R1.2 R2.2 TP1.1\nNET MY_SIG R1.1 R2.1\n");
    let (code, out, _) = command(&["nets", "--compare", &file, &path]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_netlist_file_is_read_and_printed_again_in_its_canonical_form() {
    let dir = tempfile::tempdir().unwrap();
    let messy = write_text(
        dir.path(),
        "messy.nets",
        "# a comment\nnet B  R2.1 R1.1\n\nNET A U1.1 # the chip\n   C1.1\nNET EMPTY\n",
    );
    let (code, out, err) = command(&["nets", "--import", &messy]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, "NET A C1.1 U1.1\nNET B R1.1 R2.1\nNET EMPTY\n");
    let (code, out, _) = command(&["nets", "--import", &messy, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["net_count"], 3);
    assert_eq!(v["pin_count"], 4);
}

#[test]
fn a_file_with_a_problem_is_refused_with_its_code_and_line() {
    let dir = tempfile::tempdir().unwrap();
    for (text, code, line) in [
        ("NET A R1.1\nWIRE B R2.1\n", "PHOENIX-NETS-002", "(line 2)"),
        ("NET A R1.1\nNET A R2.1\n", "PHOENIX-NETS-003", "(line 2)"),
        ("NET A R1\n", "PHOENIX-NETS-004", "(line 1)"),
        ("NET A R1.1\nNET B R1.1\n", "PHOENIX-NETS-005", "(line 2)"),
    ] {
        let file = write_text(dir.path(), "bad.nets", text);
        let (exit, out, err) = command(&["nets", "--import", &file]);
        assert_eq!((exit, out.as_str()), (2, ""), "{text}");
        assert!(err.contains(code) && err.contains(line), "{err}");
    }
}

#[test]
fn the_limits_of_the_settings_refuse_a_file_that_is_too_large() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "two.nets", "NET A R1.1\nNET B R2.1\n");
    let cfg = write_text(dir.path(), "one.cfg", "max_nets = 1\n");
    let (code, out, err) = command(&["nets", "--config", &cfg, "--import", &file]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains("PHOENIX-NETS-006"), "{err}");
    // a file over max_input_bytes is refused before it is read in full
    let big = write_text(dir.path(), "big.nets", &"# filler line\n".repeat(200));
    let small = write_text(dir.path(), "small.cfg", "max_input_bytes = 1024\n");
    let (code, _, err) = command(&["nets", "--config", &small, "--import", &big]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-006"), "{err}");
    // a settings file with a mistake gives its line
    let wrong = write_text(dir.path(), "wrong.cfg", "# x\nmax_nets = 0\n");
    let (code, _, err) = command(&["nets", "--config", &wrong, "--import", &file]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-NETS-010") && err.contains("(line 2)"),
        "{err}"
    );
}

#[test]
fn the_settings_that_ship_with_the_program_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "a.nets", BOARD_NETLIST);
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/nets.cfg");
    let (code, out, err) = command(&[
        "nets",
        "--config",
        shipped.to_str().unwrap(),
        "--import",
        &file,
    ]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, BOARD_NETLIST);
}

#[test]
fn a_netlist_equal_to_the_board_has_no_difference() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(
        dir.path(),
        "same.nets",
        "NET SIG R2.1 R1.1\nNET GND TP1.1 R2.2 R1.2\n",
    );
    let (code, out, err) = command(&["nets", "--compare", &file, &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "netlist: 2 nets, 5 pins\nboard: 2 nets, 5 pins\ndifferences: 0\n"
    );
}

const DIFFERENT: &str = "NET GND R1.2 R2.1 TP1.1\nNET SIG R1.1\nNET EXTRA U9.1\n";

#[test]
fn the_differences_are_listed_with_their_codes_and_the_exit_code_is_1() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "other.nets", DIFFERENT);
    let (code, out, err) = command(&["nets", "--compare", &file, &path]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "netlist: 3 nets, 5 pins\nboard: 2 nets, 5 pins\n\
PHOENIX-NETS-011: Net of the netlist is not on the board: EXTRA\n\
PHOENIX-NETS-014: Pin of the netlist is on no net of the board: EXTRA U9.1\n\
PHOENIX-NETS-013: Pin is on another net on the board: GND R2.1 -> SIG\n\
PHOENIX-NETS-015: Pin of a net of the board is on no net of the netlist: GND R2.2\n\
differences: 4\n"
    );
    let (code, es, _) = command(&["nets", "--compare", &file, "--lang", "es", &path]);
    assert_eq!(code, 1);
    assert!(
        es.contains(
            "PHOENIX-NETS-011: La red de la lista de redes no est\u{e1} en la placa: EXTRA\n"
        ),
        "{es}"
    );
}

#[test]
fn a_net_with_another_name_and_the_same_pins_is_one_difference() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(
        dir.path(),
        "renamed.nets",
        "NET GND R1.2 R2.2 TP1.1\nNET SIGNAL R1.1 R2.1\n",
    );
    let (code, out, _) = command(&["nets", "--compare", &file, &path]);
    assert_eq!(code, 1);
    assert!(
        out.contains(
            "PHOENIX-NETS-016: Net has another name on the board and the same pins: SIGNAL -> SIG\n"
        ),
        "{out}"
    );
    assert!(out.ends_with("differences: 1\n"), "{out}");
}

#[test]
fn the_nets_that_are_ignored_are_left_out_on_both_sides() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "other.nets", DIFFERENT);
    let cfg = write_text(dir.path(), "ignore.cfg", "ignore_nets = extra, GND\n");
    let (code, out, _) = command(&["nets", "--config", &cfg, "--compare", &file, &path]);
    assert_eq!(code, 1);
    assert!(
        out.contains(
            "PHOENIX-NETS-015: Pin of a net of the board is on no net of the netlist: SIG R2.1\n"
        ),
        "{out}"
    );
    assert!(out.ends_with("differences: 1\n"), "{out}");
}

#[test]
fn the_list_of_differences_is_cut_at_the_limit_and_the_total_is_said() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "other.nets", DIFFERENT);
    let cfg = write_text(dir.path(), "cut.cfg", "max_differences = 1\n");
    let (code, out, _) = command(&["nets", "--config", &cfg, "--compare", &file, &path]);
    assert_eq!(code, 1);
    assert!(
        out.contains("3 more differences are not listed (max_differences)\n"),
        "{out}"
    );
    assert!(out.ends_with("differences: 4\n"), "{out}");
}

#[test]
fn the_json_report_of_a_comparison_goes_to_a_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "other.nets", DIFFERENT);
    let (code, out, _) = command(&["nets", "--compare", &file, "--json", &path]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["difference_count"], 4);
    assert_eq!(v["listed"], 4);
    assert_eq!(v["file"]["net_count"], 3);
    assert_eq!(v["board"]["pin_count"], 5);
    assert_eq!(v["differences"][0]["code"], "PHOENIX-NETS-011");
    assert!(v["differences"][0]["pin"].is_null());
    assert_eq!(v["differences"][2]["code"], "PHOENIX-NETS-013");
    assert_eq!(v["differences"][2]["pin"], "R2.1");
    assert_eq!(v["differences"][2]["other"], "SIG");
    let target = path_in(dir.path(), "report.json");
    let (code, out, _) = command(&["nets", "--compare", &file, "--out", &target, &path]);
    assert_eq!(code, 1);
    assert!(out.ends_with(&format!("written: {target}\n")), "{out}");
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(written["difference_count"], 4);
    let (code, _, err) = command(&["nets", "--compare", &file, "--out", &target, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-009"), "{err}");
}

#[test]
fn what_cannot_be_used_stops_with_exit_code_2_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "a.nets", BOARD_NETLIST);
    let target = path_in(dir.path(), "never.out");
    let missing = path_in(dir.path(), "missing.input");
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["nets", &file, &path], "PHOENIX-NETS-008"),
        (vec!["nets", "--import", &file, &path], "PHOENIX-NETS-008"),
        (
            vec!["nets", "--import", &file, "--compare", &file, &path],
            "PHOENIX-NETS-008",
        ),
        (vec!["nets", "--bogus", &path], "PHOENIX-NETS-008"),
        (vec!["nets", "--lang", "xx", &path], "PHOENIX-NETS-008"),
        (vec!["nets", "--compare"], "PHOENIX-NETS-008"),
        (
            vec!["nets", "--compare", &file, "--compare", &file, &path],
            "PHOENIX-NETS-008",
        ),
        (vec!["nets", "--out", &target, &missing], "PHOENIX-NETS-001"),
        (
            vec!["nets", "--out", &target, "--import", &missing],
            "PHOENIX-NETS-001",
        ),
        (
            vec!["nets", "--out", &target, "--compare", &missing, &path],
            "PHOENIX-NETS-001",
        ),
        (
            vec!["nets", "--out", &target, "--config", &missing, &path],
            "PHOENIX-NETS-001",
        ),
    ];
    for (args, expected) in cases {
        let (code, out, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(out, "", "{args:?}");
        assert!(err.contains(expected), "{args:?}: {err}");
    }
    assert!(!Path::new(&target).exists());
    // no arguments: the usage
    let (code, out, err) = command(&["nets"]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.starts_with("phoenix nets "), "{err}");
}

#[test]
fn the_command_is_listed_in_the_help() {
    let (code, out, _) = command(&["help", "nets"]);
    assert_eq!(code, 0);
    assert!(out.contains("phoenix nets "), "{out}");
    assert!(out.contains("PHOENIX-NETS-<NNN>"), "{out}");
}

#[test]
fn two_netlist_files_are_compared_with_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let old = write_text(dir.path(), "old.nets", BOARD_NETLIST);
    let same = write_text(
        dir.path(),
        "same.nets",
        "NET SIG R2.1 R1.1\nNET GND TP1.1 R2.2 R1.2\n",
    );
    let (code, out, err) = command(&["nets", "--compare", &old, "--import", &same]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "netlist: 2 nets, 5 pins\nsecond netlist: 2 nets, 5 pins\ndifferences: 0\n"
    );
    let other = write_text(dir.path(), "new.nets", DIFFERENT);
    let (code, out, _) = command(&["nets", "--compare", &old, "--import", &other]);
    assert_eq!(code, 1);
    // the first file plays the netlist and the second the board of the comparison
    assert!(
        out.contains("PHOENIX-NETS-012: Net of the board is not in the netlist: "),
        "{out}"
    );
    assert!(out.ends_with("differences: 4\n"), "{out}");
    let (code, out, _) = command(&["nets", "--compare", &old, "--import", &other, "--json"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["board"]["net_count"], 3);
    assert_eq!(v["notes"].as_array().unwrap().len(), 0);
    // a file that is not there or not valid stops the comparison
    let missing = path_in(dir.path(), "missing.nets");
    let (code, _, err) = command(&["nets", "--compare", &old, "--import", &missing]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-001"), "{err}");
    let bad = write_text(dir.path(), "bad.nets", "NET A R1\n");
    let (code, _, err) = command(&["nets", "--compare", &old, "--import", &bad]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-004"), "{err}");
}

#[test]
fn the_pins_are_written_as_csv_with_the_formulas_guarded() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("+SIG"));
    let target = path_in(dir.path(), "pins.csv");
    let (code, out, err) = command(&["nets", "--csv", &target, &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, format!("written: {target} (5 rows)\n"));
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "net,part,pad\n'+SIG,R1,1\n'+SIG,R2,1\nGND,R1,2\nGND,R2,2\nGND,TP1,1\n"
    );
    // the guard is a setting
    let plain = write_text(dir.path(), "plain.cfg", "csv_guard_formulas = false\n");
    let second = path_in(dir.path(), "plain.csv");
    let (code, _, _) = command(&["nets", "--config", &plain, "--csv", &second, &path]);
    assert_eq!(code, 0);
    assert!(fs::read_to_string(&second)
        .unwrap()
        .starts_with("net,part,pad\n+SIG,R1,1\n"));
    // a file that exists is never replaced
    let (code, out, err) = command(&["nets", "--csv", &target, &path]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains("PHOENIX-NETS-009"), "{err}");
}

#[test]
fn the_netlist_and_the_csv_are_written_together_or_not_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (netlist, csv) = (path_in(dir.path(), "a.nets"), path_in(dir.path(), "a.csv"));
    let (code, out, _) = command(&["nets", "--out", &netlist, "--csv", &csv, &path]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!("written: {netlist} (2 nets, 5 pins)\nwritten: {csv} (5 rows)\n")
    );
    assert_eq!(fs::read_to_string(&netlist).unwrap(), BOARD_NETLIST);
    // one of the two exists: nothing new is written
    let fresh = path_in(dir.path(), "b.csv");
    let (code, _, err) = command(&["nets", "--out", &netlist, "--csv", &fresh, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-009"), "{err}");
    assert!(!Path::new(&fresh).exists());
    let (code, _, _) = command(&["nets", "--out", &fresh, "--csv", &csv, &path]);
    assert_eq!(code, 2);
    assert!(!Path::new(&fresh).exists());
    // the same name twice
    let same = path_in(dir.path(), "same.out");
    let (code, _, err) = command(&["nets", "--out", &same, "--csv", &same, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-009"), "{err}");
    assert!(!Path::new(&same).exists());
}

#[test]
fn the_differences_of_a_comparison_are_written_as_csv() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let file = write_text(dir.path(), "other.nets", DIFFERENT);
    let target = path_in(dir.path(), "diff.csv");
    let (code, out, _) = command(&["nets", "--compare", &file, "--csv", &target, &path]);
    assert_eq!(code, 1);
    assert!(
        out.ends_with(&format!("differences: 4\nwritten: {target}\n")),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "code,net,pin,other\nPHOENIX-NETS-011,EXTRA,,\nPHOENIX-NETS-014,EXTRA,U9.1,\n\
PHOENIX-NETS-013,GND,R2.1,SIG\nPHOENIX-NETS-015,GND,R2.2,\n"
    );
}

const LINTED: &str = "NET GND U1.1 U1.2\nNET Gnd R1.1 R1.2\nNET ALONE R9.1\n";

#[test]
fn the_lint_lists_single_pin_nets_and_names_that_are_alike() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "linted.nets", LINTED);
    let (code, out, err) = command(&["nets", "--lint", "--import", &file]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "nets: 3 nets, 5 pins\n\
PHOENIX-NETS-017: Net has a single pin: ALONE\n\
PHOENIX-NETS-018: Net names differ only in case or in separators: GND ~ Gnd\n\
findings: 2\n"
    );
    let (_, es, _) = command(&["nets", "--lint", "--import", &file, "--lang", "es"]);
    assert!(
        es.contains("PHOENIX-NETS-017: La red tiene un solo pin: ALONE\n"),
        "{es}"
    );
    let (code, json, _) = command(&["nets", "--lint", "--import", &file, "--json"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["finding_count"], 2);
    assert_eq!(v["findings"][0]["code"], "PHOENIX-NETS-017");
    assert!(v["findings"][0]["other"].is_null());
    assert_eq!(v["findings"][1]["other"], "Gnd");
    // the report goes to a new file only when asked
    let target = path_in(dir.path(), "lint.json");
    let (code, out, _) = command(&["nets", "--lint", "--import", &file, "--out", &target]);
    assert_eq!(code, 1);
    assert!(
        out.ends_with(&format!("findings: 2\nwritten: {target}\n")),
        "{out}"
    );
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(written["finding_count"], 2);
}

#[test]
fn the_lint_of_a_board_without_mistakes_finds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, _) = command(&["nets", "--lint", &path]);
    assert_eq!(code, 0);
    assert_eq!(out, "nets: 2 nets, 5 pins\nfindings: 0\n");
}

#[test]
fn the_settings_say_which_nets_the_lint_leaves_out() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "linted.nets", LINTED);
    let one = write_text(dir.path(), "one.cfg", "ignore_nets = alone\n");
    let (code, out, _) = command(&["nets", "--config", &one, "--lint", "--import", &file]);
    assert_eq!(code, 1);
    assert!(out.ends_with("findings: 1\n"), "{out}");
    let all = write_text(dir.path(), "all.cfg", "ignore_nets = alone, gnd\n");
    let (code, out, _) = command(&["nets", "--config", &all, "--lint", "--import", &file]);
    assert_eq!(code, 0);
    assert!(out.ends_with("findings: 0\n"), "{out}");
    let cut = write_text(dir.path(), "cut.cfg", "max_differences = 1\n");
    let (code, out, _) = command(&["nets", "--config", &cut, "--lint", "--import", &file]);
    assert_eq!(code, 1);
    assert!(
        out.contains("1 more findings are not listed (max_differences)\n"),
        "{out}"
    );
}

#[test]
fn the_lint_is_not_given_with_a_comparison_or_a_csv_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "a.nets", BOARD_NETLIST);
    let target = path_in(dir.path(), "x.csv");
    for args in [
        vec!["nets", "--lint", "--import", &file, "--compare", &file],
        vec!["nets", "--lint", "--import", &file, "--csv", &target],
    ] {
        let (code, out, err) = command(&args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(err.contains("PHOENIX-NETS-008"), "{err}");
    }
    assert!(!Path::new(&target).exists());
}

#[test]
fn the_nets_that_match_a_pattern_are_the_only_ones_written() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, err) = command(&["nets", "--net", "gnd", &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, "NET GND R1.2 R2.2 TP1.1\n");
    let (_, out, _) = command(&["nets", "--net", "s*,GND", &path]);
    assert_eq!(out, BOARD_NETLIST);
    let (code, out, _) = command(&["nets", "--net", "nothing*", &path]);
    assert_eq!((code, out.as_str()), (0, ""));
    // the JSON and the CSV file have those nets only
    let (_, json, _) = command(&["nets", "--net", "*x", "--json", &path]);
    assert!(json.contains("\"net_count\": 0"), "{json}");
    let (_, json, _) = command(&["nets", "--net", "sig", "--json", &path]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        (v["net_count"].as_i64(), v["pin_count"].as_i64()),
        (Some(1), Some(2))
    );
    let target = path_in(dir.path(), "sig.csv");
    let (code, _, _) = command(&["nets", "--net", "sig", "--csv", &target, &path]);
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "net,part,pad\nSIG,R1,1\nSIG,R2,1\n"
    );
}

#[test]
fn the_pins_of_a_part_are_listed_with_their_nets() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, err) = command(&["nets", "--part", "R1", &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "part R1: 2 pins\nR1.1 SIG (2 pins)\nR1.2 GND (3 pins)\n"
    );
    let (code, out, _) = command(&["nets", "--part", "X9", &path]);
    assert_eq!((code, out.as_str()), (1, "part X9: no pin\n"));
    // the reference is taken as written
    assert_eq!(command(&["nets", "--part", "r1", &path]).0, 1);
    let (code, json, _) = command(&["nets", "--part", "TP1", "--json", &path]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["part"], "TP1");
    assert_eq!(v["pin_count"], 1);
    assert_eq!(v["pins"][0]["pin"], "TP1.1");
    assert_eq!(v["pins"][0]["net"], "GND");
    assert_eq!(v["pins"][0]["net_pins"], 3);
    // a netlist file answers too
    let file = write_text(dir.path(), "a.nets", BOARD_NETLIST);
    let (code, out, _) = command(&["nets", "--part", "R2", "--import", &file]);
    assert_eq!(code, 0);
    assert!(out.starts_with("part R2: 2 pins\n"), "{out}");
}

#[test]
fn the_size_of_the_netlist_and_its_largest_nets_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    let (code, out, err) = command(&["nets", "--top", "1", &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "nets: 2 nets, 5 pins\nparts: 3\nnets with no pin: 0, one pin: 0, two pins: 1\nlargest nets: 1\nGND 3\n"
    );
    let (_, out, _) = command(&["nets", "--top", "9", &path]);
    assert!(out.ends_with("largest nets: 2\nGND 3\nSIG 2\n"), "{out}");
    let (code, json, _) = command(&["nets", "--top", "2", "--json", &path]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["part_count"], 3);
    assert_eq!(v["two_pin_nets"], 1);
    assert_eq!(v["largest"][1]["name"], "SIG");
    assert_eq!(v["largest"][0]["pins"], 3);
    let file = write_text(
        dir.path(),
        "big.nets",
        "NET A R1.1\nNET B\nNET C R1.2 R2.1 R3.1\n",
    );
    let (_, out, _) = command(&["nets", "--top", "5", "--import", &file]);
    assert!(
        out.contains("nets with no pin: 1, one pin: 1, two pins: 0\n"),
        "{out}"
    );
}

#[test]
fn the_new_options_are_refused_with_what_does_not_go_with_them() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "a.nets", BOARD_NETLIST);
    let target = path_in(dir.path(), "x.out");
    for args in [
        vec!["nets", "--import", &file, "--top", "0"],
        vec!["nets", "--import", &file, "--top", "1001"],
        vec!["nets", "--import", &file, "--top", "many"],
        vec!["nets", "--import", &file, "--top"],
        vec!["nets", "--import", &file, "--top", "1", "--top", "2"],
        vec!["nets", "--import", &file, "--top", "2", "--lint"],
        vec!["nets", "--import", &file, "--top", "2", "--out", &target],
        vec!["nets", "--import", &file, "--top", "2", "--part", "R1"],
        vec!["nets", "--import", &file, "--part", "R1", "--csv", &target],
        vec!["nets", "--import", &file, "--part", "R1", "--net", "a"],
        vec!["nets", "--import", &file, "--net", "a", "--lint"],
        vec!["nets", "--import", &file, "--net", "a", "--compare", &file],
        vec!["nets", "--import", &file, "--net"],
        vec!["nets", "--import", &file, "--part"],
    ] {
        let (code, out, err) = command(&args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(err.contains("PHOENIX-NETS-008"), "{args:?}: {err}");
    }
    assert!(!Path::new(&target).exists());
}

#[test]
fn the_lint_can_ask_for_the_nets_with_too_many_pins() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    assert_eq!(command(&["nets", "--lint", &path]).0, 0);
    let cfg = write_text(dir.path(), "many.cfg", "lint_max_pins = 2\n");
    let (code, out, err) = command(&["nets", "--config", &cfg, "--lint", &path]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "nets: 2 nets, 5 pins\nPHOENIX-NETS-020: Net has more pins than the limit of the lint: GND ~ 3\nfindings: 1\n"
    );
    let (_, es, _) = command(&["nets", "--config", &cfg, "--lint", &path, "--lang", "es"]);
    assert!(
        es.contains(
            "PHOENIX-NETS-020: La red tiene más pines que el límite del análisis: GND ~ 3\n"
        ),
        "{es}"
    );
}

#[test]
fn the_lint_can_ask_for_parts_on_one_net_and_names_made_by_a_program() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(
        dir.path(),
        "p.nets",
        "NET GND R1.1 R1.2 U1.1\nNET SIG U1.2 U2.1\nNET N$4 U2.2 J1.1\n",
    );
    assert_eq!(command(&["nets", "--lint", "--import", &file]).0, 0);
    let cfg = write_text(
        dir.path(),
        "more.cfg",
        "lint_part_on_one_net = true\nlint_generated_names = true\n",
    );
    let (code, out, err) = command(&["nets", "--config", &cfg, "--lint", "--import", &file]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "nets: 3 nets, 7 pins\nPHOENIX-NETS-022: Net has a name made by the program: N$4\nPHOENIX-NETS-021: Part has all its pins on one net: GND ~ R1\nfindings: 2\n"
    );
    let (_, json, _) = command(&[
        "nets", "--config", &cfg, "--lint", "--import", &file, "--json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["findings"][1]["code"], "PHOENIX-NETS-021");
    assert_eq!(v["findings"][1]["other"], "R1");
}

#[test]
fn the_parts_that_are_ignored_make_no_difference_in_a_comparison() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path(), &board_with("SIG"));
    // the schematic has no test point, the board has TP1 on GND
    let file = write_text(
        dir.path(),
        "a.nets",
        "NET GND R1.2 R2.2\nNET SIG R1.1 R2.1\n",
    );
    let (code, out, _) = command(&["nets", "--compare", &file, &path]);
    assert_eq!(code, 1);
    assert!(
        out.contains("TP1.1") && out.ends_with("differences: 1\n"),
        "{out}"
    );
    let cfg = write_text(dir.path(), "tp.cfg", "ignore_parts = tp*\n");
    let (code, out, err) = command(&["nets", "--config", &cfg, "--compare", &file, &path]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(out.ends_with("differences: 0\n"), "{out}");
    // the lint leaves the pins of those parts out too
    let (code, out, _) = command(&["nets", "--config", &cfg, "--lint", &path]);
    assert_eq!(code, 0);
    assert!(out.ends_with("findings: 0\n"), "{out}");
}

#[test]
fn the_lint_can_ask_for_the_nets_that_stay_inside_one_part() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(
        dir.path(),
        "l.nets",
        "NET LOOP U1.1 U1.2\nNET OK U1.3 R1.1\n",
    );
    assert_eq!(command(&["nets", "--lint", "--import", &file]).0, 0);
    let cfg = write_text(dir.path(), "loop.cfg", "lint_one_part_nets = true\n");
    let (code, out, err) = command(&["nets", "--config", &cfg, "--lint", "--import", &file]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "nets: 2 nets, 4 pins\nPHOENIX-NETS-023: Net joins pins of one part only: LOOP ~ U1\nfindings: 1\n"
    );
    let (_, de, _) = command(&[
        "nets", "--config", &cfg, "--lint", "--import", &file, "--lang", "de",
    ]);
    assert!(
        de.contains("PHOENIX-NETS-023: Das Netz verbindet nur Pins eines Bauteils: LOOP ~ U1\n"),
        "{de}"
    );
}

#[test]
fn the_usage_of_the_command_names_the_new_options() {
    let (code, _, err) = command(&["nets"]);
    assert_eq!(code, 2);
    for word in ["--net PATTERNS", "--part REF", "--top N"] {
        assert!(err.contains(word), "{word}");
    }
}

const KICAD_NET: &str = r#"(export (version "E")
  (design (source "demo.kicad_sch"))
  (nets
    (net (code "1") (name "GND")
      (node (ref "R1") (pin "2")) (node (ref "R2") (pin "2")) (node (ref "TP1") (pin "1")))
    (net (code "2") (name "SIG")
      (node (ref "R1") (pin "1")) (node (ref "R2") (pin "1")))))
"#;

#[test]
fn a_kicad_netlist_is_read_by_import_and_printed_as_a_text_netlist() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "demo.net", KICAD_NET);
    let (code, out, err) = command(&["nets", "--import", &file]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, BOARD_NETLIST);
    // nothing is written next to it
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_kicad_netlist_is_compared_with_a_board_and_with_a_text_netlist() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(dir.path(), "demo.net", KICAD_NET);
    let board = write_board(dir.path(), &board_with("SIG"));
    let (code, _, err) = command(&["nets", "--compare", &file, &board]);
    assert_eq!((code, err.as_str()), (0, ""));
    let other = board_with("SIGNAL");
    let board2 = path_in(dir.path(), "board2.json");
    fs::write(&board2, other.to_bytes().unwrap()).unwrap();
    let (code, out, _) = command(&["nets", "--compare", &file, &board2]);
    assert_eq!(code, 1, "{out}");
    let text = write_text(dir.path(), "same.nets", BOARD_NETLIST);
    let (code, _, err) = command(&["nets", "--compare", &file, "--import", &text]);
    assert_eq!((code, err.as_str()), (0, ""));
}

#[test]
fn a_broken_kicad_netlist_is_refused_with_its_code_and_line() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_text(
        dir.path(),
        "bad.net",
        "(export\n (nets\n  (net (code 1)\n   (node (ref R1) (pin 1)))))\n",
    );
    let (code, _, err) = command(&["nets", "--import", &file]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-NETS-002"), "{err}");
    assert!(err.contains("line 3"), "{err}");
}
