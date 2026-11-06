// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `pnp`, `stencil` and `fabpack` on a board written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{Board, Component, Footprint, Id, Layer, LayerKind, Pad, PadShape, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_export::fabpack::verify_pack;
use eh3d_phoenix_export::zipfile::read_zip;
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

fn pad(number: &str, x: i64, paste: Id) -> Pad {
    Pad {
        number: number.to_owned(),
        shape: PadShape::Rect {
            width: Nm::new(800_000).unwrap(),
            height: Nm::new(MM).unwrap(),
        },
        at: pt(x, 0),
        rotation: Udeg::new(0).unwrap(),
        drill: None,
        layers: vec![id(1), paste],
    }
}

fn component(refdes: &str, footprint: u32, at: (i64, i64), side: Side) -> Component {
    Component {
        refdes: refdes.to_owned(),
        value: "10k".to_owned(),
        symbol: None,
        footprint: id(footprint),
        at: pt(at.0, at.1),
        rotation: Udeg::new(0).unwrap(),
        side,
    }
}

/// A board of 10 mm by 10 mm with a copper layer and a paste layer for each face, R1 and R2 on
/// the top face and C1 on the bottom face.
fn board() -> Board {
    let mut b = Board {
        name: "fab".to_owned(),
        outline: vec![
            pt(0, 0),
            pt(10 * MM, 0),
            pt(10 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        ..Board::default()
    };
    for (n, name, kind, side) in [
        (1, "F.Cu", LayerKind::Copper, None),
        (2, "F.Paste", LayerKind::Paste, Some(Side::Top)),
        (3, "B.Paste", LayerKind::Paste, Some(Side::Bottom)),
    ] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.to_owned(),
                kind,
                side,
            },
        );
    }
    for (n, paste) in [(10, id(2)), (11, id(3))] {
        let mut footprint = Footprint {
            name: "R0603".to_owned(),
            pads: Default::default(),
        };
        footprint.pads.insert(id(100), pad("1", -800_000, paste));
        footprint.pads.insert(id(101), pad("2", 900_000, paste));
        b.footprints.insert(id(n), footprint);
    }
    b.components
        .insert(id(20), component("R1", 10, (3 * MM, 3 * MM), Side::Top));
    b.components
        .insert(id(21), component("R2", 10, (6 * MM, 6 * MM), Side::Top));
    b.components
        .insert(id(22), component("C1", 11, (8 * MM, 2 * MM), Side::Bottom));
    b
}

fn write_board(dir: &Path) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn pnp_writes_the_centres_of_each_face_to_a_new_folder_only() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let (code, out, err) = command(&["pnp", board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("top: 2 part(s), bottom: 1 part(s), left out: 0\n"));
    assert!(out.contains("dry run"));

    let target = dir.path().join("pnp");
    let target = target.to_str().unwrap();
    let (code, out, err) = command(&["pnp", "--dnp", "R2", "--out", target, board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(
        names(Path::new(target)),
        vec![
            "pnp_bottom.csv",
            "pnp_bottom.json",
            "pnp_top.csv",
            "pnp_top.json"
        ]
    );
    let top = fs::read_to_string(Path::new(target).join("pnp_top.csv")).unwrap();
    assert_eq!(
        top,
        "Ref,Value,Package,PosX(mm),PosY(mm),Rot(deg),Side\nR1,10k,R0603,3.0500,3.0000,0.00,top\n"
    );
    let bottom = fs::read_to_string(Path::new(target).join("pnp_bottom.csv")).unwrap();
    assert!(bottom.contains("C1,10k,R0603,7.9500,2.0000,0.00,bottom\n"));
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(Path::new(target).join("pnp_top.json")).unwrap())
            .unwrap();
    assert_eq!(json["format"], "phoenix-pnp/1");
    assert_eq!(json["not_fitted"][0], "R2");

    let (code, _, err) = command(&["pnp", "--out", target, board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PNP-009"), "{err}");
}

#[test]
fn stencil_writes_the_openings_and_the_report_to_a_new_folder_only() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let (code, out, err) = command(&["stencil", board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("openings: 6, split into windows: 0, below a ratio: 0, taken away: 0\n"));

    let target = dir.path().join("stencil");
    let target = target.to_str().unwrap();
    let (code, out, err) = command(&["stencil", "--out", target, board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(
        names(Path::new(target)),
        vec![
            "stencil_bottom.gbr",
            "stencil_report.csv",
            "stencil_top.gbr"
        ]
    );
    let top = fs::read_to_string(Path::new(target).join("stencil_top.gbr")).unwrap();
    assert!(top.contains("%TF.FileFunction,Paste,Top*%\n"));
    assert!(top.contains("X2200000Y3000000D03*\n"));

    let (code, _, err) = command(&["stencil", "--out", target, board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-STENCIL-009"), "{err}");
    let (code, _, err) = command(&["stencil", "--config", "nothing.cfg", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-FABPACK-008"), "{err}");
}

#[test]
fn fabpack_writes_a_deterministic_zip_that_reads_back_and_never_replaces() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path());
    let (code, out, err) = command(&["fabpack", board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("dry run"));
    assert!(out.contains("pnp/pnp_top.csv"));

    let one = dir.path().join("one.zip");
    let two = dir.path().join("two.ZIP");
    for path in [&one, &two] {
        let target = path.to_str().unwrap();
        let (code, out, err) = command(&[
            "fabpack",
            "--panel-boards",
            "4",
            "--out",
            target,
            board.as_str(),
        ]);
        assert_eq!((code, err.as_str()), (0, ""), "{out}");
        assert!(out.contains(&format!("written: {target} (zip file)\n")));
    }
    let bytes = fs::read(&one).unwrap();
    assert_eq!(bytes, fs::read(&two).unwrap());
    let report = verify_pack(&bytes).unwrap();
    assert_eq!(report.board, "fab");
    assert_eq!(report.profile, "generic");
    let entries = read_zip(&bytes).unwrap();
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    for expected in [
        "bom/bom_assembly.json",
        "bom/bom_board.csv",
        "bom/bom_bottom.csv",
        "bom/bom_top.csv",
        "pnp/pnp_bottom.csv",
        "pnp/pnp_top.json",
        "stencil/stencil_report.csv",
        "stencil/stencil_top.gbr",
        "manifest.json",
    ] {
        assert!(names.contains(&expected), "{expected} in {names:?}");
    }
    assert_eq!(names.last(), Some(&"manifest.json"));
    assert!(names.iter().any(|n| n.starts_with("gerber/")));
    let top_bom = entries
        .iter()
        .find(|(n, _)| n == "bom/bom_top.csv")
        .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
        .unwrap();
    assert!(top_bom.contains("R1 R2,2,8,10k,R0603"), "{top_bom}");

    let (code, _, err) = command(&["fabpack", "--out", one.to_str().unwrap(), board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-FABPACK-004"), "{err}");
    assert_eq!(fs::read(&one).unwrap(), bytes);

    let folder = dir.path().join("pack");
    let (code, out, err) = command(&["fabpack", "--out", folder.to_str().unwrap(), board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(folder.join("manifest.json").is_file());
    assert!(folder.join("pnp").join("pnp_top.csv").is_file());
}

#[test]
fn the_commands_refuse_what_they_cannot_use() {
    for args in [
        vec!["pnp"],
        vec!["stencil"],
        vec!["fabpack"],
        vec!["pnp", "--units", "inch", "b.json"],
        vec!["fabpack", "--panel-boards", "0", "b.json"],
        vec!["stencil", "--lang", "xx", "b.json"],
        vec!["pnp", "missing.json"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(!err.is_empty(), "{args:?}");
    }
}
