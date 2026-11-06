// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `print` on a board written by the tool itself.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eh3d_phoenix_board::{Board, Component, Footprint, Layer, LayerKind, Pad, PadShape, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_print::pdfcheck::check_pdf;

fn id(n: u16) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board of 20 by 10 mm with a top copper layer, a silkscreen, an outline and one resistor of
/// two pads.
fn board() -> Board {
    let (top, silk, outline, footprint, resistor) = (id(1), id(2), id(3), id(4), id(5));
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
    for (layer, name, kind, side) in [
        (top, "Top", LayerKind::Copper, None),
        (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
        (outline, "Outline", LayerKind::Outline, None),
    ] {
        b.layers.insert(
            layer,
            Layer {
                name: name.into(),
                kind,
                side,
            },
        );
    }
    let mut pads = BTreeMap::new();
    for (n, number, x) in [(6u16, "1", -1_500_000), (7, "2", 1_500_000)] {
        pads.insert(
            id(n),
            Pad {
                number: number.into(),
                shape: PadShape::Circle {
                    diameter: Nm::new(1_600_000).unwrap(),
                },
                at: pt(x, 0),
                rotation: Udeg::ZERO,
                drill: Some(Nm::new(800_000).unwrap()),
                layers: vec![top],
                roundness: 0,
                plated: true,
            },
        );
    }
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

    fn board(&self) -> String {
        let path = self.path("board.json");
        std::fs::write(&path, board().to_bytes().unwrap()).unwrap();
        path.to_str().unwrap().to_owned()
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
fn a_dry_run_describes_the_pages_and_writes_nothing() {
    let f = Files::new();
    let json = f.board();
    let (code, out, err) = call(&["print", "board", json.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("print: 3 pages, svg, paper A4\n"), "{out}");
    assert!(
        out.contains("page 1: Assembly plan - Top, 297 x 210 mm"),
        "{out}"
    );
    assert!(out.contains("would write assembly-top.svg ("), "{out}");
    assert!(out.contains("would write layers.svg ("), "{out}");
    assert!(out.contains("dry run: nothing written"), "{out}");
}

#[test]
fn out_writes_one_svg_for_each_page_to_a_new_folder_and_never_over_it() {
    let f = Files::new();
    let json = f.board();
    let folder = f.path("prints");
    let name = folder.to_str().unwrap();
    let (code, out, err) = call(&["print", "placa", "--out", name, json.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("wrote "), "{out}");
    for file in ["assembly-top.svg", "assembly-bottom.svg", "layers.svg"] {
        let text = std::fs::read_to_string(folder.join(file)).unwrap();
        assert!(text.starts_with("<?xml"), "{file}");
        assert!(text.contains("<g id=\"layer-"), "{file}");
    }
    let top = std::fs::read_to_string(folder.join("assembly-top.svg")).unwrap();
    assert!(top.contains(">R1</text>"), "the designator is on the plan");
    assert!(
        top.contains(">board</text>"),
        "the project is the name of the file"
    );
    let (code, _, err) = call(&["print", "board", "--out", name, json.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PRINT-005"), "{err}");
}

#[test]
fn the_same_input_gives_the_same_bytes_and_the_language_changes_the_title_block() {
    let f = Files::new();
    let json = f.board();
    let (a, b) = (f.path("a"), f.path("b"));
    for (dir, lang) in [(&a, "pt"), (&b, "pt")] {
        let (code, _, err) = call(&[
            "print",
            "board",
            "--side",
            "top",
            "--lang",
            lang,
            "--out",
            dir.to_str().unwrap(),
            json.as_str(),
        ]);
        assert_eq!(code, 0, "{err}");
    }
    let first = std::fs::read(a.join("assembly-top.svg")).unwrap();
    assert_eq!(first, std::fs::read(b.join("assembly-top.svg")).unwrap());
    let text = String::from_utf8(first).unwrap();
    assert!(text.contains(">Projeto</text>"), "{text}");
    assert!(text.contains("Plano de montagem - Superior"), "{text}");
    assert!(!a.join("assembly-bottom.svg").exists());
}

#[test]
fn a_pdf_is_written_to_one_new_file_and_passes_its_own_check() {
    let f = Files::new();
    let json = f.board();
    let file = f.path("board.pdf");
    let name = file.to_str().unwrap();
    let (code, out, err) = call(&[
        "print",
        "board",
        "--kind",
        "pdf",
        "--paper",
        "A3",
        "--out",
        name,
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("paper A3"), "{out}");
    let bytes = std::fs::read(&file).unwrap();
    let info = check_pdf(&bytes).unwrap();
    assert_eq!(info.pages, 3);
    assert!(info.painted > 20);
    let (code, _, err) = call(&[
        "print",
        "board",
        "--kind",
        "pdf",
        "--out",
        name,
        json.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PRINT-005"), "{err}");
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "nothing was replaced");
}

#[test]
fn the_layers_the_scale_and_the_dimensions_follow_the_options() {
    let f = Files::new();
    let json = f.board();
    let folder = f.path("only");
    let (code, _, err) = call(&[
        "print",
        "board",
        "--layers",
        "outline",
        "--scale",
        "5:1",
        "--side",
        "bottom",
        "--out",
        folder.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let text = std::fs::read_to_string(folder.join("layers.svg")).unwrap();
    assert!(text.contains("-outline\""), "{text}");
    assert!(!text.contains("top_copper"), "{text}");
    assert!(text.contains(">5:1</text>"), "{text}");
    let (code, _, err) = call(&["print", "board", "--scale", "50", json.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PRINT-003"), "{err}");

    let cfg = f.path("print.cfg");
    std::fs::write(&cfg, "dimensions = true\nproject = Mine\n").unwrap();
    let dims = f.path("dims");
    let (code, _, err) = call(&[
        "print",
        "board",
        "--config",
        cfg.to_str().unwrap(),
        "--out",
        dims.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    let text = std::fs::read_to_string(dims.join("assembly-top.svg")).unwrap();
    assert!(text.contains(">20 mm</text>"), "{text}");
    assert!(text.contains(">10 mm</text>"), "{text}");
    assert!(text.contains(">Mine</text>"), "{text}");
}

#[test]
fn a_panel_is_printed_with_its_copies() {
    let f = Files::new();
    let json = f.board();
    let folder = f.path("panel");
    let (code, out, err) = call(&[
        "print",
        "panel",
        "--out",
        folder.to_str().unwrap(),
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("print: 1 pages, svg"), "{out}");
    let text = std::fs::read_to_string(folder.join("panel.svg")).unwrap();
    assert!(text.contains("Panel - Copies: 4"), "{text}");
}

#[test]
fn text_a_pdf_cannot_show_is_reported_and_a_missing_file_is_refused() {
    let f = Files::new();
    let json = f.board();
    let (code, out, err) = call(&[
        "print",
        "board",
        "--kind",
        "pdf",
        "--lang",
        "ja",
        json.as_str(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("warning PHOENIX-PRINT-007"), "{out}");
    let missing = f.path("nothing.brd");
    let (code, _, err) = call(&["print", "board", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-VIEW-001"), "{err}");
}
