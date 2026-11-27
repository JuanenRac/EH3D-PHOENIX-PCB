// SPDX-License-Identifier: GPL-3.0-or-later
//! The times of the engine behind the board editor on large boards that the test makes itself:
//! the copper that touches (the connectivity of the live check), the rule check, the ratsnest, the
//! opening of the board in the editor, an edit with the live check, its undo and its redo, and the
//! preparation of the drawing. Nothing here is a limit: the times depend on the computer and are
//! printed to be compared on one computer before and after a change.
//!
//! `cargo test --release -p eh3d-phoenix-cli --test engine_bench -- --ignored --nocapture`

use std::collections::BTreeMap;
use std::time::Instant;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via,
};
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::{findings, ratsnest, RouteSettings, RuleContext};
use eh3d_phoenix_rules::{analyse_board, check_board, RuleValues};
use eh3d_phoenix_viewer::{Scene, ViewerSettings};

const MM: i64 = 1_000_000;
/// How many pads a row of the board has: each row is one net.
const ROW: usize = 100;

fn id(n: usize) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n as u64).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board of `parts` round pads, 100 to a row and one net to a row; in each row the first half
/// of the pads is joined by straight tracks (the rest stay to be routed), and one via of the net
/// of the row stands under every fourth pad.
fn board_of(parts: usize) -> Board {
    let mut board = Board::default();
    let (top, bottom, footprint, pad) = (id(1), id(2), id(3), id(4));
    for (layer, name, side) in [(top, "Top", Side::Top), (bottom, "Bottom", Side::Bottom)] {
        board.layers.insert(
            layer,
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: Some(side),
            },
        );
    }
    board.copper_stack = vec![top, bottom];
    let rows = parts.div_ceil(ROW);
    let (w, h) = ((ROW as i64 * 2 + 10) * MM, (rows as i64 * 3 + 10) * MM);
    board.outline = vec![pt(0, 0), pt(w, 0), pt(w, h), pt(0, h)];
    let mut pads = BTreeMap::new();
    pads.insert(
        pad,
        Pad {
            number: "1".to_owned(),
            shape: PadShape::Circle {
                diameter: Nm::new(800_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![top],
            roundness: 0,
            plated: true,
        },
    );
    board.footprints.insert(
        footprint,
        Footprint {
            name: "P".to_owned(),
            pads,
        },
    );
    let mut next = 100;
    let mut fresh = || {
        next += 1;
        id(next)
    };
    for row in 0..rows {
        let net = fresh();
        board.nets.insert(
            net,
            Net {
                name: format!("N{row}"),
            },
        );
        let y = (5 + row as i64 * 3) * MM;
        for col in 0..ROW.min(parts - row * ROW) {
            let x = (5 + col as i64 * 2) * MM;
            let component = fresh();
            board.components.insert(
                component,
                Component {
                    refdes: format!("P{}", row * ROW + col),
                    value: "x".to_owned(),
                    symbol: None,
                    footprint,
                    at: pt(x, y),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
            board.pad_nets.push(PadNet {
                component,
                pad,
                net,
            });
            if col + 1 < ROW / 2 {
                board.tracks.insert(
                    fresh(),
                    Track {
                        net,
                        layer: top,
                        width: Nm::new(250_000).unwrap(),
                        path: vec![Segment::Line {
                            start: pt(x, y),
                            end: pt(x + 2 * MM, y),
                        }],
                    },
                );
            }
            if col % 4 == 0 {
                board.vias.insert(
                    fresh(),
                    Via {
                        net,
                        at: pt(x, y + 3 * MM / 2),
                        drill: Nm::new(300_000).unwrap(),
                        diameter: Nm::new(600_000).unwrap(),
                        from_layer: top,
                        to_layer: bottom,
                    },
                );
            }
        }
    }
    board.pad_nets.sort_by_key(PadNet::pad_ref);
    board
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: Nm::new(200_000).unwrap(),
            min_track_width: Nm::new(150_000).unwrap(),
            min_drill_spacing: Nm::new(200_000).unwrap(),
            ..RuleValues::default()
        },
        classes: Default::default(),
    }
}

fn timed<T>(out: &mut String, name: &str, run: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = run();
    out.push_str(&format!(
        " {name} {:.1} ms,",
        started.elapsed().as_secs_f64() * 1000.0
    ));
    value
}

#[test]
#[ignore = "times to be read, not limits"]
fn the_times_of_the_engine_on_large_boards() {
    for parts in [1_000_usize, 4_000, 8_000, 12_000] {
        let board = board_of(parts);
        let ctx = rules();
        let mut out = format!(
            "ENGINE {parts} pads, {} tracks, {} vias:",
            board.tracks.len(),
            board.vias.len()
        );
        timed(&mut out, "connectivity-report", || board.connectivity());
        timed(&mut out, "touching-copper", || {
            analyse_board(&board, ctx.rules.connect_tolerance)
        });
        timed(&mut out, "rule-check", || check_board(&board, &ctx.rules));
        timed(&mut out, "live-findings", || findings(&board, &ctx));
        timed(&mut out, "ratsnest", || {
            ratsnest(&board, &RouteSettings::default()).unwrap().len()
        });
        timed(&mut out, "scene", || {
            Scene::from_board(&board, &ViewerSettings::default()).unwrap()
        });
        let settings = BoardEditSettings {
            live_drc: true,
            ..BoardEditSettings::default()
        };
        let moved = *board.components.keys().next().unwrap();
        let mut editor = timed(&mut out, "open-in-editor", || {
            BoardEdit::new(board.clone(), ctx.clone(), settings).unwrap()
        });
        let op = EditOp::Move {
            id: moved,
            at: pt(5 * MM, 4 * MM),
            rotation_udeg: None,
            follow: Some(false),
        };
        timed(&mut out, "drag-check", || editor.trial(&op).unwrap());
        timed(&mut out, "edit", || editor.apply(&op).unwrap());
        assert!(timed(&mut out, "undo", || editor.undo()));
        assert!(timed(&mut out, "redo", || editor.redo()));
        println!("{out}");
    }
}
