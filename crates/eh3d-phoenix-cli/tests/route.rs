// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `route` and `ratsnest` on boards written by the test itself.

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

#[test]
fn route_writes_the_board_with_the_track_to_a_new_file() {
    let f = files(false);
    let out = out_path(f.dir.path(), "routed.json");
    let original = fs::read(&f.board).unwrap();
    let (code, text, err) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", "--out", &out,
        &f.board,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.starts_with("route complete: 1 tracks, 0 vias, 5656855 nm, 0 corners"),
        "{text}"
    );
    let routed = Board::from_bytes(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(routed.tracks.len(), 1);
    assert_eq!(fs::read(&f.board).unwrap(), original);
    // the same output file is never replaced, and neither is the input
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", "--out", &out,
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-018"), "{err}");
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", "--out", &f.board,
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-018"), "{err}");
    assert_eq!(fs::read(&f.board).unwrap(), original);
}

#[test]
fn without_an_output_file_nothing_is_written_and_json_is_available() {
    let f = files(false);
    let (code, text, err) = command(&[
        "route", "--rules", &f.rules, "--json", "--from", "pad:U1.1", "--to", "pad:U1.2", &f.board,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("dry run"), "{err}");
    assert!(
        text.starts_with("{\"state\":\"complete\",\"tracks\":1,\"vias\":0,\"length_nm\":5656855"),
        "{text}"
    );
    let names: Vec<_> = fs::read_dir(f.dir.path()).unwrap().collect();
    assert_eq!(names.len(), 2);
}

#[test]
fn the_mode_that_stops_exits_with_one_and_says_why() {
    let f = files(true);
    let out = out_path(f.dir.path(), "partial.json");
    let (code, text, err) = command(&[
        "route", "--rules", &f.rules, "--mode", "stop", "--from", "pad:U1.1", "--to", "pad:U1.2",
        "--out", &out, &f.board,
    ]);
    assert_eq!(code, 1, "{err}");
    assert!(text.starts_with("route stopped: 1 tracks"), "{text}");
    assert!(
        text.contains("PHOENIX-ROUTE-008: Route stopped before an obstacle"),
        "{text}"
    );
    assert!(Path::new(&out).is_file());
    // the mode that goes around reaches the end
    let around = out_path(f.dir.path(), "around.json");
    let (code, text, err) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", "--out", &around,
        &f.board,
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(text.starts_with("route complete"), "{text}");
}

#[test]
fn the_ratsnest_lists_the_connections_that_are_left() {
    let f = files(false);
    let (code, text, err) = command(&["ratsnest", &f.board]);
    assert_eq!(code, 0, "{err}");
    assert!(text.starts_with("connections to route: 1\n"), "{text}");
    assert!(text.contains("net A: U1.1 -> U1.2 (5656854 nm)"), "{text}");
    let (code, json, _) = command(&["ratsnest", "--json", &f.board]);
    assert_eq!(code, 0);
    assert!(
        json.starts_with("{\"lines\":[{\"net\":\"A\",\"from\":\"U1.1\",\"to\":\"U1.2\""),
        "{json}"
    );
    // after the track is written the ratsnest is empty
    let out = out_path(f.dir.path(), "routed.json");
    let (code, _, _) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", "--out", &out,
        &f.board,
    ]);
    assert_eq!(code, 0);
    let (code, text, _) = command(&["ratsnest", &out]);
    assert_eq!(code, 0);
    assert!(text.starts_with("connections to route: 0"), "{text}");
}

#[test]
fn errors_have_codes_in_the_language_asked_for() {
    let f = files(false);
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--from", "pad:X9.1", "--to", "pad:U1.2", &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-003"), "{err}");
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--lang", "pt", "--from", "pad:X9.1", "--to", "pad:U1.2",
        &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(
        err.contains("O elemento a ligar não foi encontrado"),
        "{err}"
    );
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--from", "bogus", "--to", "pad:U1.2", &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-004"), "{err}");
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--layer", "Inner9", "--from", "pad:U1.1", "--to",
        "pad:U1.2", &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-017"), "{err}");
    let (code, _, err) = command(&[
        "route", "--rules", &f.rules, "--layer", "bottom", "--from", "pad:U1.1", "--to",
        "pad:U1.2", &f.board,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ROUTE-006"), "{err}");
}

#[test]
fn bad_arguments_and_bad_files_exit_2() {
    let f = files(false);
    let settings = out_path(f.dir.path(), "route.cfg");
    fs::write(&settings, "nonsense\n").unwrap();
    let missing = out_path(f.dir.path(), "absent.json");
    for list in [
        vec!["route"],
        vec!["route", &f.board],
        vec!["route", "--from", "pad:U1.1", &f.board],
        vec![
            "route", "--mode", "sideways", "--from", "pad:U1.1", "--to", "pad:U1.2", &f.board,
        ],
        vec!["route", "--nonsense", &f.board],
        vec![
            "route",
            "--rules",
            &f.rules,
            "--settings",
            &settings,
            "--from",
            "pad:U1.1",
            "--to",
            "pad:U1.2",
            &f.board,
        ],
        vec![
            "route", "--rules", &f.rules, "--from", "pad:U1.1", "--to", "pad:U1.2", &missing,
        ],
        vec!["ratsnest"],
        vec!["ratsnest", &missing],
        vec!["ratsnest", "--json"],
    ] {
        let (code, out, err) = command(&list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty(), "{list:?}");
    }
}
