// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `parts` on a base written by the test itself, and the bill of materials completed
//! from it.

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

const BASE_JSON: &str = r#"{
  "format": "phoenix-parts/1",
  "parts": [
    {"id": "r-10k", "value": "10k", "footprint": "R0603", "manufacturer": "Acme",
     "mpn": "AC-10K", "tolerance": "1%",
     "offers": [{"supplier": "Shop A", "supplier_code": "A-1",
                 "prices": [{"min_qty": 1, "unit_price": "0.01", "currency": "EUR"}]}]},
    {"id": "r-22k-old", "value": "22k", "footprint": "R0603", "mpn": "OLD-22K",
     "status": "obsolete", "alternatives": ["r-22k"]}
  ]
}"#;

const BASE_CSV: &str = "id,value,footprint,mpn,manufacturer,status,sim_model,supplier,supplier_code,min_qty,unit_price,currency\n\
r-22k,22k,R0603,AC-22K,Acme,,,Shop B,B-22,1,0.02,EUR\n\
c-100n,100n,C0603,C-100N,Zeta,,ghost-model,,,,,\n";

const OWN_LIBRARY: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<eagle version=\"9.6.2\"><drawing><settings/><layers><layer number=\"1\" name=\"Top\"/></layers><library>\n<packages>\n<package name=\"R0603\"><smd name=\"1\" x=\"-0.8\" y=\"0\" dx=\"0.9\" dy=\"0.95\" layer=\"1\"/><smd name=\"2\" x=\"0.8\" y=\"0\" dx=\"0.9\" dy=\"0.95\" layer=\"1\"/></package>\n</packages>\n<symbols>\n</symbols>\n<devicesets>\n</devicesets>\n</library></drawing></eagle>\n";

/// A folder with a JSON file and a CSV file of the base.
fn base(dir: &Path) -> String {
    fs::write(dir.join("parts.json"), BASE_JSON).unwrap();
    fs::write(dir.join("extra.csv"), BASE_CSV).unwrap();
    dir.to_str().unwrap().to_owned()
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
fn a_search_goes_through_the_files_of_the_folder_in_the_order_of_the_ids() {
    let dir = tempfile::tempdir().unwrap();
    let base = base(dir.path());
    let (code, out, err) = command(&["parts", "search", "--base", &base, "10k"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(out, "r-10k | 10k | R0603 | Acme AC-10K | active\n1 parts\n");

    let (code, out, _) = command(&["parts", "search", "--base", &base, "--value", "22k"]);
    assert_eq!(code, 0);
    let ids: Vec<&str> = out
        .lines()
        .map(|l| l.split(" | ").next().unwrap())
        .collect();
    assert_eq!(ids, vec!["r-22k", "r-22k-old", "2 parts"]);

    let (_, out, _) = command(&[
        "parts", "search", "--base", &base, "--value", "22k", "--limit", "1",
    ]);
    assert!(out.ends_with("1 of 2 parts shown\n"), "{out}");

    let (code, out, _) = command(&[
        "parts", "search", "--base", &base, "--status", "obsolete", "--json",
    ]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["total"], 1);
    assert_eq!(json["parts"][0]["id"], "r-22k-old");

    let (_, out, _) = command(&[
        "parts",
        "search",
        "--base",
        &base,
        "--footprint",
        "c0603",
        "--maker",
        "zeta",
    ]);
    assert!(
        out.starts_with("c-100n | 100n | C0603 | Zeta C-100N | active\n"),
        "{out}"
    );
}

#[test]
fn show_prints_one_part_and_a_missing_part_is_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let base = base(dir.path());
    let (code, out, _) = command(&["parts", "show", "--base", &base, "r-10k"]);
    assert_eq!(code, 0);
    assert!(out.contains("mpn: AC-10K\n"), "{out}");
    assert!(
        out.contains("offer: Shop A (A-1): 1 -> 0.01 EUR\n"),
        "{out}"
    );
    let (code, out, _) = command(&["parts", "show", "--base", &base, "--json", "r-10k"]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["offers"][0]["supplier"], "Shop A");
    let (code, _, err) = command(&["parts", "show", "--base", &base, "--lang", "pt", "nope"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("PHOENIX-PARTS-017: A peça não está na base de peças: nope"),
        "{err}"
    );
    assert_eq!(command(&["parts", "show", "--base", &base]).0, 2);
}

#[test]
fn check_reports_warnings_with_exit_zero_and_an_error_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let base = base(dir.path());
    let lib = dir.path().join("own.lbr");
    fs::write(&lib, OWN_LIBRARY).unwrap();
    let lib = lib.to_str().unwrap();
    // the files of the folder are the base: the library is not a .json or .csv file
    let (code, out, err) = command(&[
        "parts", "check", "--base", &base, "--no-ipc", "--lib", lib, "--lang", "es",
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains(
            "PHOENIX-PARTS-008: La huella no está en la biblioteca IPC ni en la propia [c-100n]"
        ),
        "{out}"
    );
    assert!(out.contains("PHOENIX-PARTS-012"), "{out}");
    assert!(
        out.contains("4 parts checked: 0 errors, 2 warnings, 0 information"),
        "{out}"
    );
    let (_, json, _) = command(&[
        "parts", "check", "--base", &base, "--no-ipc", "--lib", lib, "--json",
    ]);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["warnings"], 2);
    // without a library and without the IPC library nothing about footprints is checked
    let (code, out, _) = command(&["parts", "check", "--base", &base, "--no-ipc"]);
    assert_eq!(code, 0);
    assert!(!out.contains("PHOENIX-PARTS-008"), "{out}");
    assert!(out.contains("PHOENIX-PARTS-012"), "{out}");

    let twice = tempfile::tempdir().unwrap();
    fs::write(twice.path().join("a.csv"), "id,value\nx,1k\n").unwrap();
    fs::write(twice.path().join("b.csv"), "id,value\nx,2k\n").unwrap();
    let (code, out, _) = command(&[
        "parts",
        "check",
        "--base",
        twice.path().to_str().unwrap(),
        "--no-ipc",
    ]);
    assert_eq!(code, 1);
    assert!(out.contains("PHOENIX-PARTS-007"), "{out}");
    assert!(out.contains("a.csv: line 2"), "{out}");
}

#[test]
fn import_and_export_write_new_files_only() {
    let dir = tempfile::tempdir().unwrap();
    let base = base(dir.path());
    let csv = dir.path().join("extra.csv");
    let csv = csv.to_str().unwrap();
    let before = names(dir.path());
    let (code, out, _) = command(&["parts", "import", csv]);
    assert_eq!(code, 0);
    assert!(out.starts_with("2 parts read from"), "{out}");
    assert!(out.contains("dry run: nothing was written"), "{out}");
    assert_eq!(names(dir.path()), before);

    let outputs = tempfile::tempdir().unwrap();
    let new_json = outputs.path().join("new.json");
    let new_json = new_json.to_str().unwrap();
    let (code, out, err) = command(&["parts", "import", "--out", new_json, csv]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&format!("written: {new_json}")));
    let written = fs::read_to_string(new_json).unwrap();
    assert!(written.contains("\"format\": \"phoenix-parts/1\""));
    let (code, _, err) = command(&["parts", "import", "--out", new_json, csv]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-006"), "{err}");
    assert_eq!(fs::read_to_string(new_json).unwrap(), written);
    let not_json = outputs.path().join("new.csv");
    let (code, _, err) = command(&["parts", "import", "--out", not_json.to_str().unwrap(), csv]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-005"), "{err}");
    // an input file is never the output
    let json_input = dir.path().join("parts.json");
    let json_input = json_input.to_str().unwrap();
    let (code, _, err) = command(&["parts", "import", "--out", json_input, json_input]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-006"), "{err}");

    // the export of the whole folder to a CSV file reads back as the same base
    let copy = outputs.path().join("copy.csv");
    let copy = copy.to_str().unwrap();
    let (code, out, _) = command(&["parts", "export", "--base", &base]);
    assert_eq!(code, 0);
    assert!(
        out.contains("4 parts in") && out.contains("dry run"),
        "{out}"
    );
    let (code, out, err) = command(&["parts", "export", "--base", &base, "--out", copy]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let (_, first, _) = command(&["parts", "search", "--base", &base, "--json"]);
    let (_, second, _) = command(&["parts", "search", "--base", copy, "--json"]);
    let strip = |text: &str| -> Vec<String> {
        let json: serde_json::Value = serde_json::from_str(text).unwrap();
        json["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.to_string())
            .collect()
    };
    assert_eq!(strip(&first), strip(&second));
    let (code, _, err) = command(&["parts", "export", "--base", &base, "--out", copy]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-006"), "{err}");

    // a base with an error is not written
    let dup = outputs.path().join("dup.json");
    fs::write(
        &dup,
        r#"{"format":"phoenix-parts/1","parts":[{"id":"a"},{"id":"a"}]}"#,
    )
    .unwrap();
    let target = outputs.path().join("dup-out.json");
    let (code, _, err) = command(&[
        "parts",
        "import",
        "--out",
        target.to_str().unwrap(),
        dup.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-003"), "{err}");
    assert!(!target.exists());
}

#[test]
fn the_command_refuses_what_it_cannot_use() {
    assert_eq!(command(&["parts"]).0, 2);
    let (code, _, err) = command(&["parts", "nope"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-005"), "{err}");
    let (code, _, err) = command(&["parts", "search", "--base", "missing-folder-for-the-test"]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-002"), "{err}");
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bad.csv"), "id,colour\na,b\n").unwrap();
    let (code, _, err) = command(&["parts", "search", "--base", dir.path().to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-PARTS-003") && err.contains("unknown column colour"),
        "{err}"
    );
    let cfg = dir.path().join("p.cfg");
    fs::write(&cfg, "nope = 1\n").unwrap();
    let (code, _, err) = command(&["parts", "search", "--config", cfg.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PARTS-001"), "{err}");
}

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with two resistors of 10k in R0603.
fn write_board(dir: &Path) -> String {
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
    for (n, reference, x) in [(20, "R1", 3), (21, "R2", 6)] {
        b.components.insert(
            id(n),
            Component {
                refdes: reference.to_owned(),
                value: "10k".to_owned(),
                symbol: None,
                footprint: id(10),
                at: pt(x * MM, x * MM),
                rotation: Udeg::new(0).unwrap(),
                side: Side::Top,
            },
        );
    }
    let path = dir.join("board.json");
    fs::write(&path, b.to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn the_bill_of_materials_is_completed_from_the_base() {
    let dir = tempfile::tempdir().unwrap();
    let base = base(dir.path());
    let boards = tempfile::tempdir().unwrap();
    let board = write_board(boards.path());
    let csv = boards.path().join("bom.csv");
    let csv = csv.to_str().unwrap();
    let (code, out, err) = command(&[
        "bom",
        "--parts-db",
        &base,
        "--boards",
        "10",
        "--csv",
        csv,
        &board,
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("parts base: 2 matched, 0 without a match, 0 with several matches\n"),
        "{out}"
    );
    assert!(
        out.contains("total: 20.710000 for the batch, 2.071000 for one board\n"),
        "{out}"
    );
    let csv_text = fs::read_to_string(csv).unwrap();
    assert!(
        csv_text.contains("R1 R2,2,10k,R0603,Acme,AC-10K,Shop A,A-1,0.010000,1,21,0.210000,0.021000,1:0.010000,ok\n"),
        "{csv_text}"
    );
}

#[test]
fn the_parts_without_a_match_are_listed_and_the_bill_still_comes_out() {
    let dir = tempfile::tempdir().unwrap();
    let only = dir.path().join("only.json");
    fs::write(
        &only,
        r#"{"format":"phoenix-parts/1","parts":[{"id":"c","value":"100n","footprint":"C0603"}]}"#,
    )
    .unwrap();
    let board = write_board(dir.path());
    let (code, out, err) = command(&[
        "bom",
        "--parts-db",
        only.to_str().unwrap(),
        "--lang",
        "pt",
        &board,
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("parts base: 0 matched, 2 without a match, 0 with several matches\n"),
        "{out}"
    );
    assert!(
        out.contains("PHOENIX-PARTS-014: A peça não tem correspondência na base de peças [R1]"),
        "{out}"
    );
    let (code, _, err) = command(&["bom", "--parts-db", "missing-base-for-the-test", &board]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-BOM-016"), "{err}");
}
