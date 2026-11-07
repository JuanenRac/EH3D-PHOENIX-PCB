// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `bom` and `assembly3d` on boards written by the test itself.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{Board, Component, Footprint, Id, Layer, LayerKind, Side};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{ObjectId, Point, Udeg};

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

fn component(refdes: &str, value: &str, footprint: u32, at: (i64, i64)) -> Component {
    Component {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol: None,
        footprint: id(footprint),
        at: pt(at.0, at.1),
        rotation: Udeg::new(0).unwrap(),
        side: Side::Top,
    }
}

/// A board of 10 mm by 10 mm with one copper layer and two resistors of 10k in R0603.
fn board(second_at: (i64, i64)) -> Board {
    let mut b = Board {
        name: "cli".to_owned(),
        outline: vec![
            pt(0, 0),
            pt(10 * MM, 0),
            pt(10 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "F.Cu".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.footprints.insert(
        id(10),
        Footprint {
            name: "R0603".to_owned(),
            pads: Default::default(),
        },
    );
    b.components
        .insert(id(20), component("R1", "10k", 10, (3 * MM, 3 * MM)));
    b.components
        .insert(id(21), component("R2", "10k", 10, second_at));
    b
}

fn write_board(dir: &Path, second_at: (i64, i64)) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board(second_at).to_bytes().unwrap()).unwrap();
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

const PRICES: &str = "value,footprint,supplier,supplier_code,min_qty,unit_price\n\
10k,R0603,Shop A,A-10K,1,0.01\n";

#[test]
fn bom_costs_the_parts_and_the_boards_and_writes_new_files_only() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (6 * MM, 6 * MM));
    let prices = dir.path().join("prices.csv");
    fs::write(&prices, PRICES).unwrap();
    let prices = prices.to_str().unwrap();
    let (csv, jf, html) = (
        dir.path().join("bom.csv"),
        dir.path().join("bom.json"),
        dir.path().join("bom.html"),
    );
    let (csv, jf, html) = (
        csv.to_str().unwrap(),
        jf.to_str().unwrap(),
        html.to_str().unwrap(),
    );
    let args = [
        "bom",
        "--prices",
        prices,
        "--boards",
        "10",
        "--csv",
        csv,
        "--json-file",
        jf,
        "--html",
        html,
        board.as_str(),
    ];
    let (code, out, err) = command(&args);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("total: 20.710000 for the batch, 2.071000 for one board\n"));
    assert!(out.contains(&format!("written: {csv}\n")));

    let csv_text = fs::read_to_string(csv).unwrap();
    assert!(csv_text.contains(
        "R1 R2,2,10k,R0603,,,Shop A,A-10K,0.010000,1,21,0.210000,0.021000,1:0.010000,ok\n"
    ));
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(jf).unwrap()).unwrap();
    assert_eq!(json["total"]["batch"], "20.710000");
    assert_eq!(json["complete"], true);
    assert!(fs::read_to_string(html).unwrap().contains("<td>R1 R2</td>"));

    // the files exist now: nothing is replaced and nothing new is written
    let before = names(dir.path());
    let (code, _, err) = command(&args);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-015"), "{err}");
    assert_eq!(names(dir.path()), before);
    assert!(fs::read_to_string(csv).unwrap().contains("R1 R2,2,10k"));
}

#[test]
fn bom_without_output_options_is_a_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (6 * MM, 6 * MM));
    let before = names(dir.path());
    let (code, out, _) = command(&["bom", "--json", board.as_str()]);
    // no prices: both lines are without a price, which is a warning and not an error
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["parts"]["lines_without_price"], 1);
    assert_eq!(json["complete"], false);
    assert_eq!(names(dir.path()), before);
}

#[test]
fn bom_refuses_what_it_cannot_use() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (6 * MM, 6 * MM));
    assert_eq!(command(&["bom"]).0, 2);
    let (code, _, err) = command(&["bom", "--nope", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-014"), "{err}");
    let (code, _, err) = command(&["bom", "--lang", "pt", "missing.json"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-BOM-016: Não é possível ler um ficheiro de entrada"),
        "{err}"
    );
    let bad = dir.path().join("prices.csv");
    fs::write(&bad, "supplier,mpn\nS,X\n").unwrap();
    let (code, _, err) = command(&["bom", "--prices", bad.to_str().unwrap(), board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-003"), "{err}");
    let (code, _, err) = command(&["bom", "--boards", "0", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-014"), "{err}");
}

/// A STEP file of a box 4 by 2 by 1 millimetres.
fn box_step() -> String {
    let mut s = String::from("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n");
    s.push_str("#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\n");
    let mut n = 2;
    let mut points = Vec::new();
    for z in [0, 1] {
        for y in [0, 2] {
            for x in [0, 4] {
                s.push_str(&format!("#{n}=CARTESIAN_POINT('',({x}.,{y}.,{z}.));\n"));
                points.push(n);
                n += 1;
            }
        }
    }
    for p in points {
        s.push_str(&format!("#{n}=VERTEX_POINT('',#{p});\n"));
        n += 1;
    }
    for _ in 0..6 {
        s.push_str(&format!("#{n}=PLANE('',#2);\n"));
        s.push_str(&format!("#{}=ADVANCED_FACE('',(),#{n},.T.);\n", n + 1));
        n += 2;
    }
    s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    s
}

#[test]
fn assembly3d_builds_the_scene_checks_it_and_writes_new_files_only() {
    let dir = tempfile::tempdir().unwrap();
    // R2 overlaps R1: 4 by 2 mm boxes centred 1 mm apart
    let board = write_board(dir.path(), (4 * MM, 3 * MM));
    fs::write(dir.path().join("r.step"), box_step()).unwrap();
    let models = dir.path().join("models.csv");
    fs::write(&models, "footprint,model,dx_mm,dy_mm\nR0603,r.step,-2,-1\n").unwrap();
    let models = models.to_str().unwrap();
    let (obj, step, jf) = (
        dir.path().join("a.obj"),
        dir.path().join("a.step"),
        dir.path().join("a.json"),
    );
    let (obj, step, jf) = (
        obj.to_str().unwrap(),
        step.to_str().unwrap(),
        jf.to_str().unwrap(),
    );
    let args = [
        "assembly3d",
        "--models",
        models,
        "--obj",
        obj,
        "--step",
        step,
        "--json-file",
        jf,
        board.as_str(),
    ];
    let (code, out, err) = command(&args);
    // the two boxes collide: an error finding, exit 1
    assert_eq!((code, err.as_str()), (1, ""), "{out}");
    assert!(
        out.contains("error: PHOENIX-3D-010: Components collide [R1 and R2]"),
        "{out}"
    );
    assert!(out.contains("PHOENIX-3D-013"));
    assert!(out.contains("PHOENIX-3D-018"));
    assert!(out.contains("checks: 1 collisions, 0 too tall, 0 outside the outline\n"));

    let obj_text = fs::read_to_string(obj).unwrap();
    assert!(
        obj_text.contains("o board\n")
            && obj_text.contains("o R1\n")
            && obj_text.contains("o R2\n")
    );
    let step_text = fs::read_to_string(step).unwrap();
    assert_eq!(step_text.matches("MANIFOLD_SOLID_BREP(").count(), 3);
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(jf).unwrap()).unwrap();
    assert_eq!(json["parts"][0]["reference"], "R1");
    assert_eq!(json["parts"][0]["z_max_nm"], 2_600_000);
    assert_eq!(json["collisions"][0]["first"], "R1");

    // a second run replaces nothing
    let before = names(dir.path());
    let (code, _, err) = command(&args);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-3D-015"), "{err}");
    assert_eq!(names(dir.path()), before);
}

#[test]
fn assembly3d_without_a_collision_exits_0_and_without_options_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (7 * MM, 7 * MM));
    let models = dir.path().join("models.csv");
    fs::write(&models, "footprint,model\nR0603,box:4x2x1\n").unwrap();
    let before = names(dir.path());
    let (code, out, _) = command(&[
        "assembly3d",
        "--models",
        models.to_str().unwrap(),
        board.as_str(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("checks: 0 collisions"));
    assert_eq!(names(dir.path()), before);
}

#[test]
fn assembly3d_refuses_what_it_cannot_use() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (7 * MM, 7 * MM));
    assert_eq!(command(&["assembly3d"]).0, 2);
    let (code, _, err) = command(&["assembly3d", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-3D-014"), "{err}");
    let models = dir.path().join("models.csv");
    fs::write(&models, "footprint,model\nR0603,../outside.step\n").unwrap();
    let (code, _, err) = command(&[
        "assembly3d",
        "--models",
        models.to_str().unwrap(),
        board.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-3D-008"), "{err}");
    // a model that is not a STEP file leaves its components out with a finding and no error
    fs::write(&models, "footprint,model\nR0603,bad.step\n").unwrap();
    fs::write(dir.path().join("bad.step"), "not a step file").unwrap();
    let (code, out, _) = command(&[
        "assembly3d",
        "--models",
        models.to_str().unwrap(),
        board.as_str(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("PHOENIX-3D-003"), "{out}");
    assert!(out.contains("0 components with a box"));
}

#[test]
fn a_bill_of_eagle_or_kicad_is_imported_by_its_format_and_ranges_are_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (6 * MM, 6 * MM));
    let eagle = dir.path().join("partlist.txt");
    fs::write(
        &eagle,
        "Partlist exported from x\n\nPart  Value  Device  Package  MF    MPN\n\
R1    10k    R       R0603    Acme  AC-1\n",
    )
    .unwrap();
    let kicad = dir.path().join("bill.xml");
    fs::write(
        &kicad,
        "<export><components><comp ref=\"R1\"><fields><field name=\"MPN\">Z-9</field></fields></comp></components></export>",
    )
    .unwrap();
    let ranges = dir.path().join("ranges.csv");
    fs::write(&ranges, "Refs,Value,Qty\n\"R1-R3\",10k,3\n").unwrap();
    for (name, path, format, expected) in [
        ("eagle", &eagle, "auto", "R1,Acme,AC-1,yes\n"),
        ("eagle", &eagle, "eagle", "R1,Acme,AC-1,yes\n"),
        ("kicad", &kicad, "auto", "R1,,Z-9,yes\n"),
        ("kicad", &kicad, "kicad-xml", "R1,,Z-9,yes\n"),
        ("ranges", &ranges, "kicad-csv", ""),
    ] {
        let parts = dir.path().join(format!("{name}_{format}_parts.csv"));
        let (code, out, err) = command(&[
            "bom",
            "--import",
            path.to_str().unwrap(),
            "--import-format",
            format,
            "--parts-out",
            parts.to_str().unwrap(),
            board.as_str(),
        ]);
        assert_eq!((code, err.as_str()), (0, ""), "{name} {format}: {out}");
        assert_eq!(
            fs::read_to_string(&parts).unwrap(),
            format!("match,manufacturer,mpn,fitted\n{expected}"),
            "{name} {format}"
        );
        if name == "ranges" {
            // the range names R1, R2 and R3: R3 is not on the board and the quantity is right
            assert!(
                out.contains("[R3]: in the imported bill, not on the board"),
                "{out}"
            );
            assert!(!out.contains("PHOENIX-BOM-019"), "{out}");
        }
    }
    // a range upside down stops the import with its code
    let upside = dir.path().join("upside.csv");
    fs::write(&upside, "references\nR9-R2\n").unwrap();
    let (code, _, err) = command(&["bom", "--import", upside.to_str().unwrap(), board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-023"), "{err}");
    // a name that is not a format lists the formats
    let (code, _, err) = command(&[
        "bom",
        "--import",
        upside.to_str().unwrap(),
        "--import-format",
        "excel",
        board.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-014"), "{err}");
    assert!(
        err.contains("auto, csv, eagle, kicad-csv, kicad-xml"),
        "{err}"
    );
    // a file in Latin-1 (one byte for the micro sign) is read
    let latin = dir.path().join("latin1.csv");
    fs::write(&latin, b"references,value\nR1,10\xb5F\n").unwrap();
    let (code, out, err) = command(&["bom", "--import", latin.to_str().unwrap(), board.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("the bill says 10\u{b5}F"), "{out}");
}

#[test]
fn an_imported_bill_completes_the_parts_and_is_compared_with_the_board() {
    let dir = tempfile::tempdir().unwrap();
    let board = write_board(dir.path(), (6 * MM, 6 * MM));
    let bill = dir.path().join("bill.csv");
    fs::write(
        &bill,
        "Designator;Comment;Manufacturer;MPN\nR1;10k;Acme;AC-1\nR3;10k;;\n",
    )
    .unwrap();
    let map = dir.path().join("map.cfg");
    fs::write(&map, "delimiter = semicolon\n").unwrap();
    let out_parts = dir.path().join("parts_out.csv");
    let args = [
        "bom",
        "--import",
        bill.to_str().unwrap(),
        "--import-map",
        map.to_str().unwrap(),
        "--parts-out",
        out_parts.to_str().unwrap(),
        board.as_str(),
    ];
    let (code, out, err) = command(&args);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    // R3 is only in the bill and R2 only on the board
    assert!(out.contains("PHOENIX-BOM-021"), "{out}");
    assert!(
        out.contains("[R3]: in the imported bill, not on the board"),
        "{out}"
    );
    assert!(
        out.contains("[R2]: on the board, not in the imported bill"),
        "{out}"
    );
    // the maker of R1 comes from the bill, so R1 is a line of its own
    assert_eq!(
        fs::read_to_string(&out_parts).unwrap(),
        "match,manufacturer,mpn,fitted\nR1,Acme,AC-1,yes\n"
    );
    // a file that exists is never replaced
    let (code, _, err) = command(&args);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-015"), "{err}");
    // the options of the import need --import
    let (code, _, err) = command(&["bom", "--parts-out", "x.csv", board.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("need --import"), "{err}");
}
