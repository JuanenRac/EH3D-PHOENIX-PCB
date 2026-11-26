// SPDX-License-Identifier: GPL-3.0-or-later
//! The times of the push on large boards that the test makes itself, and how many of a fixed list
//! of pushes it can do. Nothing here is a limit: the times depend on the computer and are printed
//! to be compared on one computer before and after a change.
//!
//! `cargo test --release -p eh3d-phoenix-boardedit --test shove_bench -- --ignored --nocapture`

use std::collections::BTreeMap;
use std::time::Instant;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via,
};
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp, Shape};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::{findings, RuleContext};
use eh3d_phoenix_rules::RuleValues;

const MM: i64 = 1_000_000;
/// The rows of one column of blocks.
const ROWS: usize = 50;

fn id(n: usize) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n as u64).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// Blocks of two neighbour tracks 0.5 mm apart between pads, with a via of another net below
/// them: in blocks of 20 mm by 2.4 mm, `blocks` of them, `ROWS` to a column.
fn board_of(blocks: usize) -> Board {
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
    let columns = blocks.div_ceil(ROWS) as i64;
    let (w, h) = (
        columns * 20 * MM + 2 * MM,
        ROWS as i64 * 12 * MM / 5 + 4 * MM,
    );
    board.outline = vec![pt(0, 0), pt(w, 0), pt(w, h), pt(0, h)];
    let mut pads = BTreeMap::new();
    pads.insert(
        pad,
        Pad {
            number: "1".to_owned(),
            shape: PadShape::Circle {
                diameter: Nm::new(400_000).unwrap(),
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
    let route = id(10);
    board.nets.insert(
        route,
        Net {
            name: "R".to_owned(),
        },
    );
    for block in 0..blocks {
        let x = MM + (block / ROWS) as i64 * 20 * MM;
        let y = 3 * MM + (block % ROWS) as i64 * 12 * MM / 5;
        for (row, offset) in [0, 500_000].into_iter().enumerate() {
            let net = fresh();
            board.nets.insert(
                net,
                Net {
                    name: format!("N{block}_{row}"),
                },
            );
            for end in [x + MM, x + 18 * MM] {
                let component = fresh();
                board.components.insert(
                    component,
                    Component {
                        refdes: format!("P{block}_{row}_{end}"),
                        value: "x".to_owned(),
                        symbol: None,
                        footprint,
                        at: pt(end, y - offset),
                        rotation: Udeg::ZERO,
                        side: Side::Top,
                    },
                );
                board.pad_nets.push(PadNet {
                    component,
                    pad,
                    net,
                });
            }
            board.tracks.insert(
                fresh(),
                Track {
                    net,
                    layer: top,
                    width: Nm::new(250_000).unwrap(),
                    path: vec![Segment::Line {
                        start: pt(x + MM, y - offset),
                        end: pt(x + 18 * MM, y - offset),
                    }],
                },
            );
        }
        let net = fresh();
        board.nets.insert(
            net,
            Net {
                name: format!("V{block}"),
            },
        );
        board.vias.insert(
            fresh(),
            Via {
                net,
                at: pt(x + 9 * MM, y - 1_150_000),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: top,
                to_layer: bottom,
            },
        );
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

/// A new track along the block, `above` micrometres over the upper track of the pair.
fn push(block: usize, above: i64) -> EditOp {
    let x = MM + (block / ROWS) as i64 * 20 * MM;
    let y = 3 * MM + (block % ROWS) as i64 * 12 * MM / 5 + above * 1_000;
    EditOp::RouteShove {
        net: id(10),
        layer: id(1),
        points: vec![pt(x + 4 * MM, y), pt(x + 14 * MM, y)],
        shape: Some(Shape::Orthogonal),
        width_nm: None,
        horizontal_first: Some(true),
    }
}

#[test]
#[ignore = "times to be read, not limits"]
fn the_times_of_the_push_on_large_boards() {
    for blocks in [100_usize, 500, 2_000] {
        let board = board_of(blocks);
        let editor = BoardEdit::new(board.clone(), rules(), BoardEditSettings::default()).unwrap();
        let mut out = format!(
            "SHOVE {} tracks, {} vias, {} pads:",
            board.tracks.len(),
            board.vias.len(),
            board.components.len()
        );
        let started = Instant::now();
        let found = findings(&board, &rules());
        out.push_str(&format!(
            " rule-check {:.1} ms ({} findings),",
            started.elapsed().as_secs_f64() * 1000.0,
            found.len()
        ));
        let started = Instant::now();
        let nets = board.connectivity().nets.len();
        out.push_str(&format!(
            " connectivity {:.1} ms ({nets} nets);",
            started.elapsed().as_secs_f64() * 1000.0
        ));
        // a small push, a deeper one that reaches the via, one right on the upper track
        for (name, above) in [("one-track", 300_i64), ("to-the-via", 150), ("on-top", 40)] {
            let op = push(blocks / 2, above);
            let started = Instant::now();
            let found = editor.preview_shove(&op);
            let preview_ms = started.elapsed().as_secs_f64() * 1000.0;
            let mut copy = editor.clone();
            let started = Instant::now();
            let applied = copy.apply(&op);
            let apply_ms = started.elapsed().as_secs_f64() * 1000.0;
            let said = match &found {
                Ok(p) => format!("done, {} tracks", p.moves.len()),
                Err(e) => e.code.to_owned(),
            };
            assert_eq!(found.is_ok(), applied.is_ok());
            out.push_str(&format!(
                " {name} [{said}] preview {preview_ms:.1} ms apply {apply_ms:.1} ms;"
            ));
        }
        println!("{out}");
    }
    let editor = BoardEdit::new(board_of(100), rules(), BoardEditSettings::default()).unwrap();
    let mut done = 0;
    let mut refused: BTreeMap<&'static str, usize> = BTreeMap::new();
    let started = Instant::now();
    let mut tried = 0;
    for block in 0..20 {
        for above in [450_i64, 300, 150, 40, -250, -700, -1_000, -1_300] {
            tried += 1;
            match editor.preview_shove(&push(block, above)) {
                Ok(_) => done += 1,
                Err(e) => *refused.entry(e.code).or_default() += 1,
            }
        }
    }
    println!(
        "SHOVE_SET done {done} of {tried} in {:.0} ms, refused {refused:?}",
        started.elapsed().as_secs_f64() * 1000.0
    );
}
