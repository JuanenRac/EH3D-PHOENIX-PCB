// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the supplier assistant.

use crate::bom::{Bom, BomLine};
use crate::prices::{PriceBook, PriceRow};
use crate::settings::CostSettings;
use crate::sourcing::{
    offer_of, plan_sourcing, sourcing_json, sourcing_text, SourcingPlan, SupplierSummary,
};

const PRICES: &str =
    "value,footprint,manufacturer,mpn,supplier,supplier_code,min_qty,unit_price,currency\n\
10k,R0603,,,A,A-R,1,0.01,\n\
10k,R0603,,,A,A-R,100,0.004,\n\
100n,C0603,,,A,A-C,1,0.02,\n\
,,,X1,A,A-X1,1,1.0,\n\
10k,R0603,,,B,B-R,1,0.005,\n\
100n,C0603,,,B,B-C,1,0.01,\n\
,,,X2,B,B-X2,1,0.7,\n\
10k,R0603,,,C,C-R,1,0.0001,USD\n";

fn line(value: &str, footprint: &str, mpn: &str, alternatives: &[&str], needed: u64) -> BomLine {
    BomLine {
        value: value.to_owned(),
        footprint: footprint.to_owned(),
        manufacturer: String::new(),
        mpn: mpn.to_owned(),
        alternatives: alternatives.iter().map(|a| (*a).to_owned()).collect(),
        references: vec![format!("{value}-ref")],
        qty: 1,
        needed,
        purchase: None,
        cost_per_board_micro: 0,
    }
}

fn bill(lines: Vec<BomLine>) -> Bom {
    Bom {
        lines,
        boards: 10,
        currency: "EUR".to_owned(),
        waste_percent_micro: 0,
        parts_batch_micro: 0,
        parts_per_board_micro: 0,
        unpriced_lines: 0,
        unpriced_parts: 0,
        no_footprint_lines: 0,
        pcb: None,
        total_batch_micro: 0,
        total_per_board_micro: 0,
        complete: true,
        findings: Vec::new(),
    }
}

fn standard() -> Bom {
    bill(vec![
        line("10k", "R0603", "", &[], 100),
        line("100n", "C0603", "", &[], 50),
        line("chip", "SOT23", "X1", &["X2"], 10),
    ])
}

fn book() -> PriceBook {
    PriceBook::parse(PRICES, 100).unwrap()
}

fn named(plan: &SourcingPlan, supplier: &str) -> SupplierSummary {
    plan.suppliers
        .iter()
        .find(|s| s.supplier == supplier)
        .unwrap()
        .clone()
}

#[test]
fn each_supplier_is_priced_at_the_break_its_quantity_reaches_and_through_an_alternative() {
    let plan = plan_sourcing(&standard(), &book(), &CostSettings::default());
    // A: 100 resistors at the break of 100, 50 capacitors, the part of the line
    let a = named(&plan, "A");
    assert_eq!((a.lines_covered, a.lines_total), (3, 3));
    assert_eq!(a.parts_micro, 400_000 + 1_000_000 + 10_000_000);
    // B has no X1 but has X2, an alternative of the line
    let b = named(&plan, "B");
    assert_eq!(b.parts_micro, 500_000 + 500_000 + 7_000_000);
    assert_eq!(b.total_micro, b.parts_micro);
    // the row in another currency is left out, so there is no supplier C
    assert_eq!(plan.suppliers.len(), 2);
    assert_eq!(plan.suppliers[0].supplier, "B");
    assert_eq!(plan.single, Some(0));
    // the split buys the resistors at A and the rest at B
    assert_eq!(plan.split_total_micro, 400_000 + 500_000 + 7_000_000);
    let orders: Vec<(&str, Vec<usize>)> = plan
        .orders
        .iter()
        .map(|o| (o.supplier.as_str(), o.lines.clone()))
        .collect();
    assert_eq!(orders, vec![("A", vec![0]), ("B", vec![1, 2])]);
    assert!(plan.uncovered.is_empty());
}

#[test]
fn the_offer_of_an_alternative_says_which_part_number_it_is_for() {
    let rows = book();
    let b: Vec<&PriceRow> = rows.rows.iter().filter(|r| r.supplier == "B").collect();
    let chip = line("chip", "SOT23", "X1", &["X2"], 10);
    let offer = offer_of(&chip, &b).unwrap();
    assert_eq!(offer.alternative, "X2");
    assert_eq!(offer.supplier_code, "B-X2");
    let own = line("10k", "R0603", "", &[], 7);
    let offer = offer_of(&own, &b).unwrap();
    assert_eq!((offer.alternative.as_str(), offer.buy_qty), ("", 7));
    // below the first break the smallest break is bought
    let a: Vec<&PriceRow> = rows.rows.iter().filter(|r| r.supplier == "A").collect();
    let few = line("chip", "SOT23", "X1", &[], 0);
    assert_eq!(offer_of(&few, &a).unwrap().buy_qty, 1);
    assert!(offer_of(&line("zz", "", "", &[], 5), &a).is_none());
}

#[test]
fn shipping_makes_a_supplier_worth_dropping_from_the_split() {
    // A is dear for the whole bill but cheapest for the resistors; its shipping is not worth it
    let settings = CostSettings::parse("supplier_shipping.A = 5\n").unwrap();
    let plan = plan_sourcing(&standard(), &book(), &settings);
    assert_eq!(plan.orders.len(), 1);
    assert_eq!(plan.orders[0].supplier, "B");
    assert_eq!(plan.split_total_micro, 8_000_000);
    // the other way: B ships for a price and A does not, so everything goes to A
    let settings = CostSettings::parse("supplier_shipping.b = 5\n").unwrap();
    let plan = plan_sourcing(&standard(), &book(), &settings);
    assert_eq!(plan.orders.len(), 1);
    assert_eq!(plan.orders[0].supplier, "A");
    assert_eq!(plan.split_total_micro, 11_400_000);
    // the single supplier is the cheapest with the shipping, here A
    assert_eq!(plan.suppliers[plan.single.unwrap()].supplier, "A");
    assert_eq!(named(&plan, "B").total_micro, 13_000_000);
}

#[test]
fn the_split_moves_a_whole_order_to_a_supplier_it_had_not_used() {
    let prices = PriceBook::parse(
        "value,footprint,supplier,supplier_code,min_qty,unit_price\n\
10k,R0603,A,A-R,1,0.01\n\
10k,R0603,B,B-R,1,0.02\n",
        10,
    )
    .unwrap();
    let bom = bill(vec![line("10k", "R0603", "", &[], 21)]);
    // without shipping the cheapest supplier buys everything
    let free = plan_sourcing(&bom, &prices, &CostSettings::default());
    assert_eq!(free.orders.len(), 1);
    assert_eq!(free.orders[0].supplier, "A");
    assert_eq!(free.split_total_micro, 210_000);
    // with shipping at A the dearer parts of B are cheaper in all
    let settings = CostSettings::parse("supplier_shipping.A = 1\n").unwrap();
    let plan = plan_sourcing(&bom, &prices, &settings);
    assert_eq!(plan.orders.len(), 1);
    assert_eq!(plan.orders[0].supplier, "B");
    assert_eq!(plan.split_total_micro, 420_000);
    assert_eq!(plan.suppliers[plan.single.unwrap()].supplier, "B");
}

#[test]
fn free_shipping_and_the_least_order_follow_the_value_of_the_order() {
    let settings = CostSettings::parse(
        "supplier_shipping.B = 5\nsupplier_free_shipping.B = 8\nsupplier_min_order.B = 10\n",
    )
    .unwrap();
    let plan = plan_sourcing(&standard(), &book(), &settings);
    let b = named(&plan, "B");
    // 8.00 of parts reaches the value from which shipping is free, and is below the least order
    assert_eq!(b.shipping_micro, 0);
    assert!(b.below_minimum);
    assert_eq!(b.min_order_micro, 10_000_000);
    assert!(!named(&plan, "A").below_minimum);
    let text = sourcing_text(&plan, &standard());
    assert!(
        text.contains("below the least order of 10.000000"),
        "{text}"
    );
}

#[test]
fn the_list_of_the_settings_limits_the_suppliers_and_a_line_without_a_price_is_listed() {
    let settings = CostSettings::parse("suppliers = B, Z\n").unwrap();
    let mut bom = standard();
    bom.lines.push(line("zz", "X", "", &[], 3));
    let plan = plan_sourcing(&bom, &book(), &settings);
    let names: Vec<&str> = plan.suppliers.iter().map(|s| s.supplier.as_str()).collect();
    assert_eq!(names, ["B", "Z"]);
    assert_eq!(named(&plan, "Z").lines_covered, 0);
    assert_eq!(named(&plan, "B").lines_covered, 3);
    // no supplier covers the line zz, so no supplier covers every line
    assert_eq!(plan.single, None);
    assert_eq!(plan.uncovered, vec![3]);
    let text = sourcing_text(&plan, &bom);
    assert!(
        text.contains("no single supplier covers every line"),
        "{text}"
    );
    assert!(
        text.contains("no supplier has a price for zz-ref"),
        "{text}"
    );
}

#[test]
fn an_empty_bill_or_an_empty_price_file_gives_an_empty_plan() {
    let plan = plan_sourcing(&bill(Vec::new()), &book(), &CostSettings::default());
    assert_eq!(plan.single, None);
    assert!(plan.orders.is_empty() && plan.uncovered.is_empty());
    assert_eq!(plan.split_total_micro, 0);
    let none = plan_sourcing(&standard(), &PriceBook::default(), &CostSettings::default());
    assert!(none.suppliers.is_empty() && none.orders.is_empty());
    assert_eq!(none.uncovered, vec![0, 1, 2]);
}

#[test]
fn the_plan_is_the_same_every_time_and_has_a_text_and_a_json_form() {
    let bom = standard();
    let settings = CostSettings::default();
    let a = plan_sourcing(&bom, &book(), &settings);
    assert_eq!(a, plan_sourcing(&bom, &book(), &settings));
    let text = sourcing_text(&a, &bom);
    assert!(
        text.contains("cheapest single supplier: B for 8.000000"),
        "{text}"
    );
    assert!(
        text.contains("split purchase: 2 orders for 7.900000"),
        "{text}"
    );
    let json = sourcing_json(&a, &bom);
    assert_eq!(json["single"], "B");
    assert_eq!(json["split_total"], "7.900000");
    assert_eq!(json["orders"][0]["supplier"], "A");
    assert_eq!(json["suppliers"].as_array().unwrap().len(), 2);
}
