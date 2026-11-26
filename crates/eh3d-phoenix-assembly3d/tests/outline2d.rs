// SPDX-License-Identifier: GPL-3.0-or-later
//! The artifact `outline2d` of a synthetic board, read back as the program of enclosures reads
//! it.

use std::collections::BTreeMap;

use eh3d_phoenix_assembly3d::outline2d::outline2d;
use eh3d_phoenix_assembly3d::{AssemblyError, AssemblySettings, ModelTable};
use eh3d_phoenix_board::{Board, Component, Footprint, Pad, PadShape, Side};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use serde_json::{json, Value};

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn mm(v: i64) -> Nm {
    Nm::new(v * 1_000_000).unwrap()
}

fn at(x: i64, y: i64) -> Point {
    Point {
        x_nm: mm(x),
        y_nm: mm(y),
    }
}

fn pad(number: &str, drill: Option<i64>, plated: bool) -> Pad {
    Pad {
        number: number.to_owned(),
        shape: PadShape::Circle { diameter: mm(4) },
        at: at(0, 0),
        rotation: Udeg::ZERO,
        drill: drill.map(mm),
        layers: Vec::new(),
        roundness: 0,
        plated,
    }
}

/// A board of 80 by 50 mm with a mounting hole, a capacitor in the middle, a connector at the
/// front edge and a part under the board.
fn board() -> Board {
    let mut board = Board {
        name: "synthetic".to_owned(),
        outline: vec![at(0, 0), at(80, 0), at(80, 50), at(0, 50)],
        ..Board::default()
    };
    for (n, name, hole) in [
        (20, "HOLE", true),
        (21, "CAP", false),
        (22, "SOCKET", false),
    ] {
        let pads = if hole {
            BTreeMap::from([(id(30), pad("1", Some(3), false))])
        } else {
            BTreeMap::from([(id(31), pad("1", None, true))])
        };
        board.footprints.insert(
            id(n),
            Footprint {
                name: name.to_owned(),
                pads,
            },
        );
    }
    for (n, refdes, footprint, x, y, side) in [
        (40, "H1", 20, 4, 4, Side::Top),
        (41, "C1", 21, 40, 25, Side::Top),
        (42, "J1", 22, 40, 3, Side::Top),
        (43, "U2", 21, 60, 25, Side::Bottom),
    ] {
        board.components.insert(
            id(n),
            Component {
                refdes: refdes.to_owned(),
                value: String::new(),
                symbol: None,
                footprint: id(footprint),
                at: at(x, y),
                rotation: Udeg::ZERO,
                side,
            },
        );
    }
    board
}

const MODELS: &str =
    "footprint,model,dx_mm,dy_mm,dz_mm\nCAP,box:10x10x12,0,0,0\nSOCKET,box:10x6x5,0,0,0\n";

fn write(board: &Board, table: &ModelTable) -> Option<Vec<u8>> {
    outline2d(board, table, &AssemblySettings::default(), &mut |file| {
        Err(AssemblyError::new("PHOENIX-3D-001", file))
    })
    .unwrap()
}

#[test]
fn the_artifact_says_what_the_board_says_and_reads_back() {
    let table = ModelTable::parse(MODELS, 100).unwrap();
    let bytes = write(&board(), &table).unwrap();
    assert_eq!(write(&board(), &table).unwrap(), bytes);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["artifact"], "outline2d");
    assert_eq!(value["version"], 1);
    assert_eq!(value["units"], "nm");
    assert_eq!(
        value["outline"],
        json!([
            [0, 0],
            [80_000_000, 0],
            [80_000_000, 50_000_000],
            [0, 50_000_000]
        ])
    );
    assert_eq!(value["thickness"], 1_600_000);
    assert_eq!(value["tallest"], 12_000_000);
    assert_eq!(
        value["holes"],
        json!([{"id": id(40).to_string(), "name": "H1", "x": 4_000_000, "y": 4_000_000, "diameter": 3_000_000}])
    );
    let parts = value["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 3);
    assert_eq!(
        parts[0],
        json!({"name": "C1", "x": 35_000_000, "y": 20_000_000, "width": 10_000_000, "depth": 10_000_000, "height": 12_000_000, "side": "top"})
    );
    assert_eq!(parts[2]["side"], "bottom");
    assert_eq!(parts[2]["height"], 12_000_000);
    assert_eq!(
        value["connectors"],
        json!([{"name": "J1", "edge": "front", "at": 40_000_000, "width": 10_000_000, "height": 5_000_000, "above": 0}])
    );
}

#[test]
fn without_a_table_there_are_no_parts_and_without_an_outline_there_is_no_artifact() {
    let bytes = write(&board(), &ModelTable::default()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["parts"], json!([]));
    assert_eq!(value["connectors"], json!([]));
    assert!(value.get("tallest").is_none());
    assert_eq!(value["holes"].as_array().unwrap().len(), 1);
    let mut open = board();
    open.outline.truncate(2);
    assert_eq!(write(&open, &ModelTable::default()), None);
}
