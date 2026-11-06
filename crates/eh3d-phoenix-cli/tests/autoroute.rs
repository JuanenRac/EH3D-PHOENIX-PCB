// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `autoroute` on boards written by the test itself: the DSN export, the import of a
//! session as a plan that the rules check, and the run of the external program, which stays off.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

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

/// A 20 mm board with two copper layers; net A has pads U1.1 at (2, 2) mm and U1.2 at (6, 6) mm and,
/// when `obstacle` is true, net B has a pad U1.3 at (4, 4) mm between them.
fn board(obstacle: bool) -> Board {
    let mut b = Board::default();
    for (n, name) in [(1_u32, "Top"), (2, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack.push(id(n));
    }
    let p = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
    b.outline = vec![
        p(0, 0),
        p(20_000_000, 0),
        p(20_000_000, 20_000_000),
        p(0, 20_000_000),
    ];
    b.nets.insert(id(10), Net { name: "A".into() });
    b.nets.insert(id(11), Net { name: "B".into() });
    let mut places = vec![(2_000_000, 2_000_000, 10), (6_000_000, 6_000_000, 10)];
    if obstacle {
        places.push((4_000_000, 4_000_000, 11));
    }
    let mut pads = BTreeMap::new();
    let mut pad_nets = Vec::new();
    for (i, (x, y, net)) in places.iter().enumerate() {
        let pid = id(100 + i as u32);
        pads.insert(
            pid,
            Pad {
                number: (i + 1).to_string(),
                shape: PadShape::Circle {
                    diameter: Nm::new(600_000).unwrap(),
                },
                at: p(*x, *y),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![id(1)],
                roundness: 0,
                plated: true,
            },
        );
        pad_nets.push(PadNet {
            component: id(43),
            pad: pid,
            net: id(*net),
        });
    }
    b.footprints.insert(
        id(40),
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    b.components.insert(
        id(43),
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: id(40),
            at: p(0, 0),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    pad_nets.sort_by_key(|n| n.pad_ref());
    b.pad_nets = pad_nets;
    b
}

const RULES: &str = r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":150000,"min_clearance_nm":200000}}"#;

struct Files {
    dir: tempfile::TempDir,
    board: String,
    rules: String,
}

fn files(obstacle: bool) -> Files {
    let dir = tempfile::tempdir().unwrap();
    let board_path = dir.path().join("board.json");
    fs::write(&board_path, board(obstacle).to_bytes().unwrap()).unwrap();
    let rules_path = dir.path().join("rules.json");
    fs::write(&rules_path, RULES).unwrap();
    Files {
        board: board_path.to_str().unwrap().to_owned(),
        rules: rules_path.to_str().unwrap().to_owned(),
        dir,
    }
}

fn out_path(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}

const SESSION_OK: &str = "(session s (routes (resolution um 1000) (network_out \
    (net A (wire (path Top 250000 2000000 2000000 6000000 6000000))))))";

fn write(f: &Files, name: &str, text: &str) -> String {
    let path = out_path(f.dir.path(), name);
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn the_export_prints_a_summary_and_writes_a_new_file_only_when_asked() {
    let f = files(false);
    let (code, text, err) = command(&["autoroute", "export-dsn", "--rules", &f.rules, &f.board]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.starts_with(
            "dsn: 2 layers, 1 components, 2 pads, 1 nets, 2 padstacks, 0 tracks, 0 vias, "
        ),
        "{text}"
    );
    assert!(text.ends_with(" bytes\n"), "{text}");
    assert!(err.contains("dry run"), "{err}");
    let out = out_path(f.dir.path(), "board.dsn");
    let (code, _, err) = command(&[
        "autoroute",
        "export-dsn",
        "--rules",
        &f.rules,
        "--out",
        &out,
        &f.board,
    ]);
    assert_eq!(code, 0, "{err}");
    let written = fs::read_to_string(&out).unwrap();
    assert!(written.starts_with("(pcb \"board\"\n"));
    assert!(written.contains("(pins U1-1 U1-2)"));
    // an existing file is never replaced, and the input is not touched
    let (code, _, err) = command(&[
        "autoroute",
        "export-dsn",
        "--rules",
        &f.rules,
        "--out",
        &out,
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-018"), "{err}");
    assert_eq!(fs::read_to_string(&out).unwrap(), written);
    let (code, json, _) = command(&["autoroute", "export-dsn", "--json", &f.board]);
    assert_eq!(code, 0);
    assert!(
        json.starts_with("{\"written\":false,\"dsn\":{\"layers\":2,"),
        "{json}"
    );
}

#[test]
fn a_session_is_imported_as_a_plan_and_written_to_a_new_board() {
    let f = files(false);
    let ses = write(&f, "routes.ses", SESSION_OK);
    let original = fs::read(&f.board).unwrap();
    let (code, text, err) = command(&[
        "autoroute",
        "import-ses",
        "--rules",
        &f.rules,
        &f.board,
        &ses,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.starts_with("session routes: 1 tracks, 0 vias, 5656854 nm of copper\n"),
        "{text}"
    );
    assert!(
        text.contains("connections to route: 1 before, 0 after"),
        "{text}"
    );
    assert!(err.contains("dry run"), "{err}");
    let out = out_path(f.dir.path(), "routed.json");
    let (code, _, err) = command(&[
        "autoroute",
        "import-ses",
        "--rules",
        &f.rules,
        "--require-complete",
        "--out",
        &out,
        &f.board,
        &ses,
    ]);
    assert_eq!(code, 0, "{err}");
    let routed = Board::from_bytes(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(routed.tracks.len(), 1);
    assert_eq!(fs::read(&f.board).unwrap(), original);
    let (code, text, _) = command(&["ratsnest", &out]);
    assert_eq!(code, 0);
    assert!(text.starts_with("connections to route: 0"), "{text}");
    // the output file is not replaced the second time
    let (code, _, err) = command(&[
        "autoroute",
        "import-ses",
        "--rules",
        &f.rules,
        "--out",
        &out,
        &f.board,
        &ses,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-018"), "{err}");
    let (code, json, _) = command(&[
        "autoroute",
        "import-ses",
        "--json",
        "--rules",
        &f.rules,
        &f.board,
        &ses,
    ]);
    assert_eq!(code, 0);
    assert!(json.starts_with("{\"tracks\":1,\"vias\":0,"), "{json}");
}

#[test]
fn a_session_that_breaks_the_rules_or_leaves_work_is_refused_and_writes_nothing() {
    let f = files(true);
    // the wire crosses the pad of the other net
    let ses = write(&f, "short.ses", SESSION_OK);
    let out = out_path(f.dir.path(), "never.json");
    let (code, _, err) = command(&[
        "autoroute",
        "import-ses",
        "--rules",
        &f.rules,
        "--out",
        &out,
        &f.board,
        &ses,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-009"), "{err}");
    assert!(!Path::new(&out).exists());
    let partial = write(
        &f,
        "partial.ses",
        "(session s (routes (resolution um 1000) (network_out \
         (net A (wire (path Top 250000 2000000 2000000 2000000 6000000))))))",
    );
    let (code, _, err) = command(&[
        "autoroute",
        "import-ses",
        "--rules",
        &f.rules,
        "--require-complete",
        "--out",
        &out,
        &f.board,
        &partial,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-010"), "{err}");
    assert!(!Path::new(&out).exists());
    let unknown = write(
        &f,
        "unknown.ses",
        "(session s (routes (resolution um 1000) (network_out (net Z))))",
    );
    let (code, _, err) = command(&[
        "autoroute",
        "import-ses",
        "--lang",
        "pt",
        &f.board,
        &unknown,
    ]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-ROUTE-023: A sessão não corresponde à placa"),
        "{err}"
    );
}

#[test]
fn the_run_is_a_dry_run_and_the_program_stays_off_by_default() {
    let f = files(false);
    let (code, text, err) = command(&["autoroute", "run", "--rules", &f.rules, &f.board]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.starts_with("autoroute dry run, nothing is started\n"),
        "{text}"
    );
    assert!(text.contains("external autorouter: off"), "{text}");
    assert!(text.contains("command: not configured"), "{text}");
    // with the switch off and --execute nothing is started either
    let (code, _, err) = command(&[
        "autoroute",
        "run",
        "--execute",
        "--rules",
        &f.rules,
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-025"), "{err}");

    let program = out_path(f.dir.path(), "no-such-router");
    let folder = f.dir.path().to_str().unwrap();
    let config = write(
        &f,
        "autorouter.cfg",
        &format!(
            "enabled = false\nexecutable = {program}\narg = -de\narg = {{dsn}}\narg = -do\narg = {{ses}}\ntemp_dir = {folder}\n"
        ),
    );
    let (code, text, err) = command(&[
        "autoroute",
        "run",
        "--config",
        &config,
        "--rules",
        &f.rules,
        &f.board,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("external autorouter: off (enabled = false)"),
        "{text}"
    );
    assert!(text.contains(&format!("command: {program} -de ")), "{text}");
    let (code, json, _) = command(&["autoroute", "run", "--json", "--config", &config, &f.board]);
    assert_eq!(code, 0);
    assert!(
        json.starts_with("{\"mode\":\"dry_run\",\"enabled\":false,"),
        "{json}"
    );
    // the dry runs made nothing in the temporary folder: only the board, the rules and the settings
    let made: Vec<_> = fs::read_dir(f.dir.path()).unwrap().collect();
    assert_eq!(made.len(), 3, "{made:?}");
}

#[test]
fn a_program_that_is_not_there_is_reported_in_the_language_asked_for() {
    let f = files(false);
    let program = out_path(f.dir.path(), "no-such-router");
    let folder = f.dir.path().to_str().unwrap();
    let config = write(
        &f,
        "autorouter.cfg",
        &format!(
            "enabled = true\nexecutable = {program}\narg = {{dsn}}\narg = {{ses}}\ntemp_dir = {folder}\ntimeout_s = 5\n"
        ),
    );
    let (code, _, err) = command(&[
        "autoroute",
        "run",
        "--execute",
        "--config",
        &config,
        "--rules",
        &f.rules,
        "--lang",
        "pt",
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(
        err.contains(
            "PHOENIX-ROUTE-026: Não foi possível executar o encaminhador automático externo"
        ),
        "{err}"
    );
    let bad = write(&f, "bad.cfg", "enabled = yes\n");
    let (code, _, err) = command(&["autoroute", "run", "--config", &bad, &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-024"), "{err}");
}

#[test]
fn wrong_use_prints_the_usage() {
    let f = files(false);
    for args in [
        vec!["autoroute"],
        vec!["autoroute", "nothing"],
        vec!["autoroute", "export-dsn"],
        vec!["autoroute", "import-ses", &f.board],
        vec!["autoroute", "run"],
    ] {
        let (code, out, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(out.is_empty());
        assert!(
            err.contains("phoenix autoroute export-dsn"),
            "{args:?}: {err}"
        );
    }
    let (code, _, err) = command(&["autoroute", "export-dsn", "--unknown", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-017"), "{err}");
    let (code, _, err) = command(&["autoroute", "export-dsn", "--lang", "xx", &f.board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-017"), "{err}");
}
