// SPDX-License-Identifier: GPL-3.0-or-later
//! The bill of materials on a small board made by hand, with the numbers worked out on paper.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Pad, PadShape, Side, Via,
};
use eh3d_phoenix_bom::codes::{
    CURRENCY_IGNORED, FOOTPRINT_MISSING, MINIMUM_RAISED, NOT_FITTED, NOT_PLACED, NO_FOOTPRINT,
    NO_PRICE, PCB_ESTIMATE, PCB_NOT_COSTED,
};
use eh3d_phoenix_bom::{
    bom_csv, bom_html, bom_json, bom_text, build_bom, Bom, BomInputs, CostSettings, PartsFile,
    PriceBook,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{Part, Schematic};

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn pad(drill: Option<i64>) -> Pad {
    Pad {
        number: "1".to_owned(),
        shape: PadShape::Circle {
            diameter: Nm::new(MM).unwrap(),
        },
        at: pt(0, 0),
        rotation: Udeg::new(0).unwrap(),
        drill: drill.map(|d| Nm::new(d).unwrap()),
        layers: vec![id(1)],
        roundn,
        roundness: 0,
        plated: true,
        ess: 0,
        plated: true,
    }
}

fn component(refdes: &str, value: &str, footprint: Id) -> Component {
    Component {
        refdes: refdes.to_owned(),
        value: value.to_owned(),
        symbol: None,
        footprint,
        at: pt(MM, MM),
        rotation: Udeg::new(0).unwrap(),
        side: Side::Top,
    }
}

/// A board of 10 mm by 10 mm with two copper layers, three resistors of 10k, a capacitor, a part
/// in a through-hole footprint and a via.
fn board() -> Board {
    let mut b = Board {
        name: "bom".to_owned(),
        outline: vec![
            pt(0, 0),
            pt(10 * MM, 0),
            pt(10 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        ..Board::default()
    };
    for (n, name) in [(1, "F.Cu"), (2, "B.Cu")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    for (n, name, drill) in [
        (10, "R0603", None),
        (11, "C0603", None),
        (12, "DIP8", Some(800_000)),
    ] {
        let mut f = Footprint {
            name: name.to_owned(),
            pads: Default::default(),
        };
        f.pads.insert(id(n + 100), pad(drill));
        b.footprints.insert(id(n), f);
    }
    let parts = [
        (20, "R1", "10k", 10),
        (21, "R2", "10k", 10),
        (22, "R3", "10k", 10),
        (23, "C1", "100n", 11),
        (24, "U1", "NE555", 12),
    ];
    for (n, r, v, f) in parts {
        b.components.insert(id(n), component(r, v, id(f)));
    }
    b.vias.insert(
        id(30),
        Via {
            net: id(40),
            at: pt(5 * MM, 5 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(1),
            to_layer: id(2),
        },
    );
    b
}

const PRICES: &str =
    "value,footprint,manufacturer,mpn,supplier,supplier_code,min_qty,unit_price,currency\n\
10k,R0603,,,Shop A,A-10K,1,0.0100,EUR\n\
10k,R0603,,,Shop A,A-10K,100,0.0040,EUR\n\
10k,R0603,,,Shop B,B-10K,1,0.0200,EUR\n\
100n,C0603,,,Shop B,B-100N,50,0.0200,\n\
100n,C0603,,,Shop C,C-100N,1,0.0001,USD\n";

fn run(board: &Board, parts: &str, prices: &str, settings: &CostSettings) -> Bom {
    let parts = PartsFile::parse(parts, 100).unwrap();
    let prices = PriceBook::parse(prices, 100).unwrap();
    build_bom(&BomInputs {
        board,
        schematic: None,
        parts: &parts,
        prices: &prices,
        settings,
        stackup: None,
    })
    .unwrap()
}

fn codes(bom: &Bom) -> Vec<&'static str> {
    bom.findings.iter().map(|f| f.code).collect()
}

#[test]
fn the_lines_quantities_and_offers_follow_the_hand_calculation() {
    let bom = run(&board(), "match\n", PRICES, &CostSettings::default());
    assert_eq!(bom.lines.len(), 3);
    let c1 = &bom.lines[0];
    assert_eq!(c1.references, vec!["C1"]);
    // 11 needed with the waste of 5 %, the supplier sells from 50
    assert_eq!(c1.needed, 11);
    let p = c1.purchase.as_ref().unwrap();
    assert_eq!(
        (p.supplier.as_str(), p.buy_qty, p.cost_micro),
        ("Shop B", 50, 1_000_000)
    );
    let r = &bom.lines[1];
    assert_eq!(r.references, vec!["R1", "R2", "R3"]);
    assert_eq!(r.qty, 3);
    // 32 needed, Shop A at the break of 1: 32 x 0.01
    assert_eq!(r.needed, 32);
    let p = r.purchase.as_ref().unwrap();
    assert_eq!(
        (p.supplier.as_str(), p.supplier_code.as_str()),
        ("Shop A", "A-10K")
    );
    assert_eq!(p.cost_micro, 320_000);
    assert_eq!(r.cost_per_board_micro, 32_000);
    let u1 = &bom.lines[2];
    assert!(u1.purchase.is_none());
    assert_eq!(bom.unpriced_lines, 1);
    assert_eq!(bom.unpriced_parts, 1);
    assert_eq!(bom.parts_batch_micro, 1_320_000);
    assert_eq!(bom.parts_per_board_micro, 132_000);
}

#[test]
fn the_boards_cost_follows_the_tariff_of_the_settings() {
    let bom = run(&board(), "match\n", PRICES, &CostSettings::default());
    let pcb = bom.pcb.as_ref().unwrap();
    // 1 cm2 over 80 % of a panel at 0.06 a cm2 is 0.075, plus two holes at 0.001
    assert_eq!((pcb.layers, pcb.holes), (2, 2));
    assert_eq!(pcb.area_nm2, 100_000_000_000_000);
    assert_eq!(pcb.per_board_micro, 77_000);
    assert_eq!(pcb.batch_micro, 20_770_000);
    assert_eq!(bom.total_batch_micro, 22_090_000);
    assert_eq!(bom.total_per_board_micro, 2_209_000);
    assert!(!bom.complete);
}

#[test]
fn the_findings_name_what_is_missing_and_what_was_done_on_its_own() {
    let bom = run(&board(), "match\n", PRICES, &CostSettings::default());
    let found = codes(&bom);
    for code in [CURRENCY_IGNORED, MINIMUM_RAISED, NO_PRICE, PCB_ESTIMATE] {
        assert_eq!(found.iter().filter(|c| **c == code).count(), 1, "{code}");
    }
    assert!(!found.contains(&NO_FOOTPRINT));
    assert!(!found.contains(&PCB_NOT_COSTED));
    let no_price = bom.findings.iter().find(|f| f.code == NO_PRICE).unwrap();
    assert_eq!(no_price.subject.as_deref(), Some("U1"));
}

#[test]
fn a_part_that_is_not_fitted_is_left_out_and_a_maker_splits_a_line() {
    let parts = "match,manufacturer,mpn,fitted\nR3,,,no\nR1,Acme,AC-1,\n";
    let bom = run(&board(), parts, PRICES, &CostSettings::default());
    assert!(codes(&bom).contains(&NOT_FITTED));
    let refs: Vec<Vec<String>> = bom.lines.iter().map(|l| l.references.clone()).collect();
    assert_eq!(refs, vec![vec!["C1"], vec!["R1"], vec!["R2"], vec!["U1"]]);
    let r1 = bom.lines.iter().find(|l| l.references == ["R1"]).unwrap();
    assert_eq!(
        (r1.manufacturer.as_str(), r1.mpn.as_str()),
        ("Acme", "AC-1")
    );
}

#[test]
fn a_footprint_that_is_not_on_the_board_is_an_error_and_a_part_without_one_is_flagged() {
    let mut b = board();
    b.components.insert(id(50), component("R9", "1k", id(99)));
    let bom = run(&b, "match\n", PRICES, &CostSettings::default());
    let found = codes(&bom);
    assert!(found.contains(&FOOTPRINT_MISSING));
    assert!(found.contains(&NO_FOOTPRINT));
    let line = bom.lines.iter().find(|l| l.references == ["R9"]).unwrap();
    assert!(line.footprint.is_empty());
    assert!(eh3d_phoenix_bom::line_status(line).contains("no_footprint"));
}

#[test]
fn a_part_of_the_schematic_that_is_not_on_the_board_is_added_with_a_finding() {
    let mut schematic = Schematic::default();
    schematic.parts.insert(
        id(60),
        Part {
            refdes: "R1".to_owned(),
            value: "10k".to_owned(),
            symbol: id(61),
            footprint: Some(id(10)),
            placement: None,
        },
    );
    schematic.parts.insert(
        id(62),
        Part {
            refdes: "R7".to_owned(),
            value: "10k".to_owned(),
            symbol: id(61),
            footprint: Some(id(10)),
            placement: None,
        },
    );
    let parts = PartsFile::default();
    let prices = PriceBook::parse(PRICES, 100).unwrap();
    let board = board();
    let bom = build_bom(&BomInputs {
        board: &board,
        schematic: Some(&schematic),
        parts: &parts,
        prices: &prices,
        settings: &CostSettings::default(),
        stackup: None,
    })
    .unwrap();
    let placed: Vec<_> = bom
        .findings
        .iter()
        .filter(|f| f.code == NOT_PLACED)
        .collect();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].subject.as_deref(), Some("R7"));
    let r = bom.lines.iter().find(|l| l.value == "10k").unwrap();
    assert_eq!(r.references, vec!["R1", "R2", "R3", "R7"]);
}

#[test]
fn a_board_without_a_tariff_for_its_layers_has_no_cost_and_a_finding() {
    let mut b = board();
    b.layers.insert(
        id(3),
        Layer {
            name: "In1".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    let bom = run(&b, "match\n", PRICES, &CostSettings::default());
    assert!(bom.pcb.is_none());
    assert!(codes(&bom).contains(&PCB_NOT_COSTED));
    assert_eq!(bom.total_batch_micro, bom.parts_batch_micro);
}

#[test]
fn the_minimum_order_applies_to_a_small_batch() {
    let settings =
        CostSettings::parse("boards = 1\npcb_setup_cost = 0\npcb_min_order_cost = 50\n").unwrap();
    let bom = run(&board(), "match\n", PRICES, &settings);
    assert_eq!(bom.pcb.as_ref().unwrap().batch_micro, 50_000_000);
}

#[test]
fn the_outputs_are_the_same_every_time_and_escape_what_comes_from_a_file() {
    let mut b = board();
    b.components
        .insert(id(70), component("X1", "=1+1,\"q\"", id(10)));
    b.components
        .insert(id(71), component("X2", "<script>&", id(10)));
    let first = run(&b, "match\n", PRICES, &CostSettings::default());
    let second = run(&b, "match\n", PRICES, &CostSettings::default());
    assert_eq!(bom_csv(&first), bom_csv(&second));
    assert_eq!(bom_json(&first), bom_json(&second));
    assert_eq!(bom_html(&first, Lang::En), bom_html(&second, Lang::En));
    assert_eq!(bom_text(&first, Lang::Es), bom_text(&second, Lang::Es));

    let csv = bom_csv(&first);
    assert!(csv.starts_with("references,quantity,value,footprint,"));
    assert!(csv.contains("'=1+1,\"q\"") || csv.contains("\"'=1+1,\"\"q\"\"\""));
    assert!(!csv.contains(",=1+1"));
    let html = bom_html(&first, Lang::En);
    assert!(html.contains("&lt;script&gt;&amp;"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("<html lang=\"en\">"));
}

#[test]
fn the_json_has_the_totals_as_decimal_strings() {
    let bom = run(&board(), "match\n", PRICES, &CostSettings::default());
    let json = bom_json(&bom);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], "phoenix-bom/1");
    assert_eq!(value["total"]["batch"], "22.090000");
    assert_eq!(value["total"]["per_board"], "2.209000");
    assert_eq!(value["parts"]["lines_without_price"], 1);
    assert_eq!(value["pcb"]["estimate"], true);
    assert_eq!(value["complete"], false);
    assert_eq!(value["lines"][1]["offer"]["supplier"], "Shop A");
    assert!(value["lines"][2]["offer"].is_null());
    assert!(json.ends_with("}\n"));
}

#[test]
fn the_text_names_the_totals_and_the_findings_in_the_language_asked() {
    let bom = run(&board(), "match\n", PRICES, &CostSettings::default());
    let text = bom_text(&bom, Lang::Pt);
    assert!(text.contains("total: 22.090000 for the batch, 2.209000 for one board (not complete)"));
    assert!(text.contains("warning: PHOENIX-BOM-005: A peça não tem preço [U1]"));
}

#[test]
fn leaving_the_maker_out_of_the_grouping_joins_the_lines_that_only_it_separated() {
    let parts = "match,manufacturer,mpn,fitted\nR3,,,no\nR1,Acme,AC-1,\n";
    let settings = CostSettings::parse("group_by = value,footprint\n").unwrap();
    let bom = run(&board(), parts, PRICES, &settings);
    let refs: Vec<Vec<String>> = bom.lines.iter().map(|l| l.references.clone()).collect();
    // R1 (Acme) and R2 (no maker) are one line of two, with neither maker nor part number
    assert_eq!(refs, vec![vec!["C1"], vec!["R1", "R2"], vec!["U1"]]);
    let r = bom.lines.iter().find(|l| l.qty == 2).unwrap();
    assert!(r.manufacturer.is_empty() && r.mpn.is_empty());
    // 2 parts x 10 boards + 5 per cent of waste = 21 parts, priced by value and footprint
    assert_eq!(r.needed, 21);
}

#[test]
fn the_csv_has_the_columns_asked_for_in_the_order_asked() {
    let settings = CostSettings::parse("csv_columns = quantity,references,status\n").unwrap();
    let bom = run(&board(), "match\n", PRICES, &settings);
    let csv = eh3d_phoenix_bom::bom_csv_with(&bom, &settings.csv_columns);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "quantity,references,status");
    // the first line of the bill is C1: one part, priced at one supplier
    assert!(lines[1].starts_with("1,C1,"), "{csv}");
    assert!(lines.iter().all(|l| l.split(',').count() == 3), "{csv}");
}
