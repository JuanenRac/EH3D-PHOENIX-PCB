// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `panel` on boards written by the tool itself.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Layer, LayerKind, Net, Pad, PadShape, Side, Via,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board of 20 by 10 mm with a top copper layer, an outline and one resistor with a drilled pad.
fn board() -> Board {
    let (top, outline, footprint, resistor) = (id(1), id(2), id(3), id(4));
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
        footprint,
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
            footprint,
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
fn a_dry_run_describes_the_panel_and_writes_nothing() {
    let f = Files::new();
    let json = f.board();
    let (code, out, err) = call(&["panel", json.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("panel 62.0 mm by 42.0 mm, 2 x 2 boards\n"),
        "{out}"
    );
    assert!(
        out.contains("boards use 30.7% of the area of the panel\n"),
        "{out}"
    );
    assert!(out.contains("file panel_01_top.gbr ("), "{out}");
    assert!(out.contains("file panel_drill_pth.drl ("), "{out}");
    assert!(out.contains("dry run: nothing written"), "{out}");
}

#[test]
fn out_writes_the_layers_of_every_copy_to_a_new_folder() {
    let f = Files::new();
    let json = f.board();
    let folder = f.path("panel");
    let name = folder.to_str().unwrap();
    let (code, out, err) = call(&["panel", "--out", name, json.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.ends_with("written\n"), "{out}");
    for file in [
        "panel_01_top.gbr",
        "panel_profile.gbr",
        "panel_drill_pth.drl",
        "panel_drill_npth.drl",
        "panel_placement.csv",
        "panel_bom.csv",
    ] {
        assert!(folder.join(file).is_file(), "{file}");
    }
    let placement = std::fs::read_to_string(folder.join("panel_placement.csv")).unwrap();
    assert_eq!(placement.lines().filter(|l| l.contains("AXIAL")).count(), 4);
    // 4 pads of the copies and 3 fiducials on the top copper
    let top = std::fs::read_to_string(folder.join("panel_01_top.gbr")).unwrap();
    assert_eq!(top.matches("D03*").count(), 4 + 3, "{top}");
    // a folder that exists is never written into, and the board is never written
    let (code, _, err) = call(&["panel", "--out", name, json.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PANEL-008"), "{err}");
    let (code, _, err) = call(&["panel", "--lang", "es", "--out", name, json.as_str()]);
    assert_eq!(code, 2);
    assert!(
        err.contains("El fichero de salida no está permitido"),
        "{err}"
    );
    assert_eq!(std::fs::read(&json).unwrap(), board().to_bytes().unwrap());
}

#[test]
fn a_list_of_boards_is_packed_and_summarised_as_json() {
    let f = Files::new();
    let json = f.board();
    let list = format!("{json},{json}");
    let (code, out, err) = call(&["panel", "--boards", list.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("panel 62.0 mm by 30.0 mm, 2 boards\n"),
        "{out}"
    );
    assert!(
        out.contains("boards use 21.5% of the area of the panel\n"),
        "{out}"
    );
    let (code, out, err) = call(&["panel", "--json", "--boards", list.as_str()]);
    assert_eq!(code, 0, "{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["mixed"], true);
    assert_eq!(value["copies"].as_array().unwrap().len(), 2);
    assert_eq!(value["copies"][1]["x_nm"], 32_000_000);
    assert_eq!(value["utilisation_permille"], 215);
}

#[test]
fn the_settings_turn_mirror_and_number_the_copies() {
    let f = Files::new();
    let json = f.board();
    let cfg = f.write(
        "panel.cfg",
        b"columns = 2\nrows = 1\ncopy_rotation_deg = 90\ncopy_mirror = true\ncopy_suffix = true\n",
    );
    let folder = f.path("turned");
    let (code, _, err) = call(&[
        "panel",
        "--config",
        cfg.as_str(),
        "--out",
        folder.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let placement = std::fs::read_to_string(folder.join("panel_placement.csv")).unwrap();
    assert!(placement.contains("R1_1"), "{placement}");
    assert!(placement.contains("R1_2"), "{placement}");
    // seen from the other side, the parts are on the bottom
    assert!(placement.contains("bottom"), "{placement}");
}

#[test]
fn a_v_score_line_too_close_to_copper_is_refused() {
    let f = Files::new();
    let cfg = f.write(
        "panel.cfg",
        b"columns = 2\nrows = 1\njoint = v_score\ngap_nm = 0\nframe = none\n",
    );
    let clear = f.board();
    let (code, out, err) = call(&["panel", "--config", cfg.as_str(), clear.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("joint: V-score, 1 lines\n"), "{out}");
    // a via on the line between the two columns
    let mut with_via = board();
    with_via.nets.insert(id(9), Net { name: "SIG".into() });
    with_via.vias.insert(
        id(10),
        Via {
            net: id(9),
            at: pt(20_000_000, 5_000_000),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    let near = f.write("near.json", &with_via.to_bytes().unwrap());
    let (code, _, err) = call(&["panel", "--config", cfg.as_str(), near.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PANEL-015"), "{err}");
    // a clearance of 0 turns the check off
    let off = f.write(
        "off.cfg",
        b"columns = 2\nrows = 1\njoint = v_score\ngap_nm = 0\nframe = none\nv_score_clearance_nm = 0\n",
    );
    let (code, _, err) = call(&["panel", "--config", off.as_str(), near.as_str()]);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn arguments_and_boards_that_cannot_be_used_exit_2() {
    let f = Files::new();
    let json = f.board();
    for args in [
        vec!["panel"],
        vec!["panel", "--nonsense", json.as_str()],
        vec!["panel", "--boards", json.as_str(), json.as_str()],
        vec!["panel", "--boards", ",", json.as_str()],
        vec!["panel", "--lang", "xx", json.as_str()],
        vec!["panel", "--columns", "many", json.as_str()],
        vec!["panel", "--columns", "0", json.as_str()],
        vec![
            "panel",
            "--profile",
            "nothing",
            "--profiles",
            "/nonexistent/p.json",
            json.as_str(),
        ],
        vec!["panel", "/nonexistent/board.json"],
    ] {
        let (code, out, err) = call(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(out.is_empty(), "{args:?}");
        assert!(!err.is_empty(), "{args:?}");
    }
    // boards that are placed on top of each other
    let cfg = f.write(
        "overlap.cfg",
        b"frame = none\nplace = 0,0,0,0,false\nplace = 0,10000000,0,0,false\n",
    );
    let list = format!("{json},{json}");
    let (code, _, err) = call(&["panel", "--config", cfg.as_str(), "--boards", list.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PANEL-014"), "{err}");
}
