// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `export` on boards written by the tool itself.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eh3d_phoenix_board::{Board, Component, Footprint, Layer, LayerKind, Pad, PadShape, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

const PROFILES: &str = include_str!("../../../config/export_profiles.json");

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with a top copper layer, an outline and one resistor with a drilled pad.
fn board() -> Board {
    let (top, outline, solder, resistor) = (id(1), id(2), id(3), id(4));
    let mut b = Board {
        name: "Cli board".into(),
        outline: vec![
            pt(0, 0),
            pt(20_000_000, 0),
            pt(20_000_000, 10_000_000),
            pt(0, 10_000_000),
        ],
        copper_stack: vec![top],
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
        outline,
        Layer {
            name: "Outline".into(),
            kind: LayerKind::Outline,
            side: None,
        },
    );
    let mut pads = BTreeMap::new();
    pads.insert(
        id(5),
        Pad {
            number: "1".into(),
            shape: PadShape::Circle {
                diameter: Nm::new(1_600_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: Some(Nm::new(800_000).unwrap()),
            layers: vec![top],
            roundness: 0,
            plated: true,
        },
    );
    b.footprints.insert(
        solder,
        Footprint {
            name: "AXIAL".into(),
            pads,
        },
    );
    b.components.insert(
        resistor,
        Component {
            refdes: "R1".into(),
            value: "1k".into(),
            symbol: None,
            footprint: solder,
            at: pt(10_000_000, 5_000_000),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b
}

struct Files(tempfile::TempDir);

impl Files {
    fn new() -> Files {
        Files(tempfile::tempdir().unwrap())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.path(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn board(&self) -> String {
        self.write("board.json", &board().to_bytes().unwrap())
    }
}

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn export_writes_the_files_and_the_manifest() {
    let f = Files::new();
    let json = f.board();
    let out_dir = f.path("fab");
    let out_str = out_dir.to_str().unwrap();
    let (code, out, err) = call(&["export", "--out", out_str, json.as_str()]);
    assert_eq!(code, 0, "{err}");
    // two layers, the plated drill file, the placement file, the bill of materials, the manifest
    assert!(out.contains("6 files written"), "{out}");
    for name in [
        "01_top.gbr",
        "02_outline.gbr",
        "drill_pth.drl",
        "placement.csv",
        "bom.csv",
        "manifest.json",
    ] {
        assert!(out_dir.join(name).is_file(), "{name}");
    }
    let placement = std::fs::read_to_string(out_dir.join("placement.csv")).unwrap();
    assert!(
        placement.contains("R1,1k,AXIAL,10.0000,5.0000,0.00,top"),
        "{placement}"
    );
    // the same board gives the same manifest
    let again = f.path("again");
    let (code, _, _) = call(&["export", "--out", again.to_str().unwrap(), json.as_str()]);
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read(out_dir.join("manifest.json")).unwrap(),
        std::fs::read(again.join("manifest.json")).unwrap()
    );
}

#[test]
fn export_reads_an_eagle_board_too() {
    let f = Files::new();
    let brd = f.write("b.brd", write_board(&board()).text.as_bytes());
    let out_dir = f.path("fab");
    let (code, _, err) = call(&["export", "--out", out_dir.to_str().unwrap(), brd.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out_dir.join("manifest.json").is_file());
}

#[test]
fn export_uses_the_profile_of_the_file() {
    let f = Files::new();
    let json = f.board();
    let profiles = f.write("profiles.json", PROFILES.as_bytes());
    let out_dir = f.path("fab");
    let (code, _, err) = call(&[
        "export",
        "--profiles",
        profiles.as_str(),
        "--profile",
        "jlc-like",
        "--out",
        out_dir.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let positions = std::fs::read_to_string(out_dir.join("positions.csv")).unwrap();
    assert!(positions.starts_with("Designator,Val,Package,Mid X,Mid Y,Rotation,Layer\n"));
    assert!(
        positions.contains("R1,1k,AXIAL,10.0000,5.0000,0,Top"),
        "{positions}"
    );
    assert!(out_dir.join("bom_assembly.csv").is_file());
}

#[test]
fn export_can_also_write_a_package() {
    let f = Files::new();
    let json = f.board();
    let package = f.path("board.eh3dproj");
    let fab = f.path("fab");
    let args = [
        "export",
        "--eh3dproj",
        package.to_str().unwrap(),
        "--out",
        fab.to_str().unwrap(),
        json.as_str(),
    ];
    let (code, _, err) = call(&args);
    assert_eq!(code, 0, "{err}");
    let first = std::fs::read(&package).unwrap();
    assert!(first.starts_with(b"PK"));
    // a file that is there is never replaced, so the same call again is refused
    assert_eq!(call(&args).0, 2);
    assert_eq!(std::fs::read(&package).unwrap(), first);
    // the same board in new places gives the very same package
    let again = f.path("again.eh3dproj");
    let fab_again = f.path("fab_again");
    let (code, _, err) = call(&[
        "export",
        "--eh3dproj",
        again.to_str().unwrap(),
        "--out",
        fab_again.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(std::fs::read(&again).unwrap(), first);
}

#[test]
fn export_reports_what_cannot_be_used_with_its_code_in_the_chosen_language() {
    let f = Files::new();
    let json = f.board();
    let out_str = f.path("fab").to_str().unwrap().to_owned();
    // a profile that is not in the file
    let (code, _, err) = call(&[
        "export",
        "--profile",
        "nothing",
        "--out",
        out_str.as_str(),
        json.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EXPORT-012"), "{err}");
    let (_, _, german) = call(&[
        "export",
        "--profile",
        "nothing",
        "--lang",
        "de",
        "--out",
        out_str.as_str(),
        json.as_str(),
    ]);
    assert!(german.contains("Exportprofil nicht gefunden"), "{german}");
    // a profile file that is not valid
    let bad = f.write(
        "bad.json",
        b"{\"format\": \"phoenix-export/1\", \"profiles\": [{\"name\": \"\"}]}",
    );
    let (code, _, err) = call(&[
        "export",
        "--profiles",
        bad.as_str(),
        "--out",
        out_str.as_str(),
        json.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EXPORT-011"), "{err}");
    // the output folder cannot be a file
    let blocker = f.write("blocker", b"x");
    let (code, _, err) = call(&["export", "--out", blocker.as_str(), json.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-EXPORT-013"), "{err}");
}

#[test]
fn export_arguments_and_boards_that_cannot_be_used_exit_2() {
    let f = Files::new();
    let json = f.board();
    let out_str = f.path("fab").to_str().unwrap().to_owned();
    let broken = f.write("broken.json", b"not json");
    let missing = f.path("absent.json").to_str().unwrap().to_owned();
    for list in [
        &["export"][..],
        &["export", json.as_str()],
        &["export", "--out", out_str.as_str()],
        &["export", "--out"],
        &[
            "export",
            "--out",
            out_str.as_str(),
            json.as_str(),
            json.as_str(),
        ],
        &[
            "export",
            "--out",
            out_str.as_str(),
            "--lang",
            "xx",
            json.as_str(),
        ],
        &[
            "export",
            "--out",
            out_str.as_str(),
            "--nonsense",
            json.as_str(),
        ],
        &["export", "--out", out_str.as_str(), broken.as_str()],
        &["export", "--out", out_str.as_str(), missing.as_str()],
    ] {
        let (code, out, err) = call(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty(), "{list:?}");
    }
}
