// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `enclosure` on files written by the tests: the walls, the lid, the floor and the
//! posts, a contour from a drawing, the JSON report, the STEP solid of the board and the exit
//! codes.

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track, Via};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_dxf::{write_dxf, Drawing};
use eh3d_phoenix_model::{Nm, ObjectId, Point};

const MM: i64 = 1_000_000;

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * MM, y_mm * MM).unwrap()
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

/// A board 60 by 40 mm with one copper layer, a track along y = 10 mm and a via at (30, 20) mm.
fn board() -> Board {
    let mut board = Board {
        name: "T".to_owned(),
        outline: vec![pt(0, 0), pt(60, 0), pt(60, 40), pt(0, 40)],
        ..Board::default()
    };
    board.layers.insert(
        id(1),
        Layer {
            name: "Top".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.copper_stack = vec![id(1)];
    board.nets.insert(
        id(10),
        Net {
            name: "SIG".to_owned(),
        },
    );
    board.tracks.insert(
        id(40),
        Track {
            net: id(10),
            layer: id(1),
            width: Nm::new(250_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(10, 10),
                end: pt(30, 10),
            }],
        },
    );
    board.vias.insert(
        id(41),
        Via {
            net: id(10),
            at: pt(30, 20),
            drill: Nm::new(400_000).unwrap(),
            diameter: Nm::new(1_000_000).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    board
}

struct Files {
    dir: tempfile::TempDir,
    board: String,
}

impl Files {
    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }

    fn write(&self, name: &str, text: &str) -> String {
        let path = self.path(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

fn files() -> Files {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("in.json");
    std::fs::write(&path, board().to_bytes().unwrap()).unwrap();
    Files {
        board: path.to_string_lossy().into_owned(),
        dir,
    }
}

#[test]
fn a_board_inside_the_box_passes_and_one_outside_it_exits_1() {
    let f = files();
    let wide = f.write("wide.cfg", "box = 70x50x20\n");
    let (code, out, err) = call(&["enclosure", &f.board, "--enclosure", &wide]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("PHOENIX-ENCL-011"), "{out}");
    assert!(out.contains("walls 5.000000 mm"), "{out}");
    assert!(out.contains("no table of models was given"), "{out}");
    let narrow = f.write("narrow.cfg", "box = 50x30x20\n");
    let (code, out, _) = call(&["enclosure", &f.board, "--enclosure", &narrow]);
    assert_eq!(code, 1);
    assert!(out.contains("error: PHOENIX-ENCL-005"), "{out}");
    let (code, out, _) = call(&[
        "enclosure",
        &f.board,
        "--enclosure",
        &narrow,
        "--lang",
        "es",
    ]);
    assert_eq!(code, 1);
    assert!(
        out.contains("La placa o un componente está fuera de las paredes"),
        "{out}"
    );
}

#[test]
fn the_json_report_of_the_enclosure_and_the_step_solid_with_its_hole() {
    let f = files();
    let case = f.write(
        "case.cfg",
        "box = 70x50x20\nstandoff_mm = 4\npost = 3,3,6,3.2\npost = 30,20,6,0.4\n",
    );
    let step = f.path("board.step");
    let (code, out, err) = call(&[
        "enclosure",
        &f.board,
        "--enclosure",
        &case,
        "--json",
        "--step",
        &step,
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["format"], "phoenix-enclosure/1");
    assert_eq!(value["cavity"]["floor_nm"], -4_000_000);
    assert_eq!(value["cavity"]["lid_nm"], 16_000_000);
    assert_eq!(value["posts"].as_array().unwrap().len(), 2);
    let codes: Vec<&str> = value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    // the first post has no hole under it; the second has the via
    assert!(codes.contains(&"PHOENIX-ENCL-009"), "{codes:?}");
    assert!(codes.contains(&"PHOENIX-ENCL-013"), "{codes:?}");
    let solid = std::fs::read_to_string(&step).unwrap();
    assert!(solid.starts_with("ISO-10303-21;\n"));
    assert_eq!(solid.matches("CYLINDRICAL_SURFACE(").count(), 1);
    assert_eq!(solid.matches("MANIFOLD_SOLID_BREP('board',").count(), 1);
    // an existing file is never replaced
    let (code, _, err) = call(&["enclosure", &f.board, "--enclosure", &case, "--step", &step]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ENCL-004"), "{err}");
}

#[test]
fn an_inner_contour_comes_from_a_drawing_next_to_the_file() {
    let f = files();
    let mut d = Drawing::new();
    d.polygon(
        "INNER",
        &[
            (-5 * MM, -5 * MM),
            (65 * MM, -5 * MM),
            (65 * MM, 45 * MM),
            (-5 * MM, 45 * MM),
        ],
    );
    f.write("inner.dxf", &write_dxf(&d));
    let case = f.write(
        "case.cfg",
        "contour = inner.dxf\ncontour_layer = inner\nheight_mm = 20\n",
    );
    let (code, out, err) = call(&["enclosure", &f.board, "--enclosure", &case]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("walls 5.000000 mm"), "{out}");
    assert!(out.contains("(4 corners)"), "{out}");
    // a drawing with no such layer
    let missing = f.write(
        "missing.cfg",
        "contour = inner.dxf\ncontour_layer = OTHER\nheight_mm = 20\n",
    );
    let (code, _, err) = call(&["enclosure", &f.board, "--enclosure", &missing]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ENCL-014"), "{err}");
}

#[test]
fn the_wrong_arguments_of_the_enclosure_command_exit_2() {
    let f = files();
    let case = f.write("case.cfg", "box = 70x50x20\n");
    assert_eq!(call(&["enclosure"]).0, 2);
    assert_eq!(call(&["enclosure", &f.board]).0, 2);
    assert_eq!(call(&["enclosure", "--enclosure", &case]).0, 2);
    assert_eq!(
        call(&["enclosure", &f.board, "--enclosure", &case, "--nope"]).0,
        2
    );
    let bad = f.write("bad.cfg", "box = 70x50\n");
    let (code, _, err) = call(&["enclosure", &f.board, "--enclosure", &bad]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ENCL-001"), "{err}");
    let (code, _, err) = call(&["enclosure", &f.path("none.json"), "--enclosure", &case]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-ENCL-002"), "{err}");
}
