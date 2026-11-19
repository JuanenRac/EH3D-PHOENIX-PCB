// SPDX-License-Identifier: GPL-3.0-or-later
//! The supplier assistant: where to buy the parts of a bill of materials, worked out from the
//! local price file and the list of suppliers of `config/costs.cfg` (nothing is fetched from a
//! network, and the prices are the ones of the file).
//!
//! For each supplier it prices every line at the quantity to buy (the price of the greatest
//! quantity break reached, as in [`crate::bom`]); a line that a supplier has no row for may be
//! priced through one of its alternative part numbers. From that it gives
//!
//! * a summary for each supplier: the lines it covers, the cost of the parts, the shipping and
//!   the total, and whether the order is below its least order;
//! * the cheapest single supplier that covers every line, shipping included;
//! * a split purchase: each line goes to the cheapest supplier, then the suppliers whose whole
//!   order can be moved to the others for less (the shipping saved is more than the parts cost
//!   extra) are dropped, one at a time, until nothing is gained.
//!
//! The list of suppliers (`suppliers`), the shipping (`supplier_shipping.NAME`), the least order
//! (`supplier_min_order.NAME`) and the order value from which shipping is free
//! (`supplier_free_shipping.NAME`) are in the settings; with no list every supplier of the price
//! file is considered. A tie goes to the first by name, so the same input gives the same plan.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::bom::{row_matches, Bom, BomLine};
use crate::money::format_micro;
use crate::prices::{PriceBook, PriceRow};
use crate::settings::CostSettings;

/// What one supplier asks for the quantity to buy of one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The code of the part at the supplier.
    pub supplier_code: String,
    /// The part number the price is of when it is an alternative of the line; empty for the part
    /// of the line.
    pub alternative: String,
    /// The quantity break the price is of.
    pub tier_min_qty: u64,
    /// The price of one part, in millionths.
    pub unit_price_micro: i64,
    /// The quantity bought (at least the quantity break).
    pub buy_qty: u64,
    /// The cost of the quantity bought, in millionths.
    pub cost_micro: i128,
}

/// The order of the whole bill at one supplier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierSummary {
    /// The supplier.
    pub supplier: String,
    /// The lines it has a price for.
    pub lines_covered: usize,
    /// The lines of the bill.
    pub lines_total: usize,
    /// The cost of the parts it covers, in millionths.
    pub parts_micro: i128,
    /// The shipping of that order, in millionths.
    pub shipping_micro: i128,
    /// Parts and shipping, in millionths.
    pub total_micro: i128,
    /// The least order it takes, in millionths; zero when the settings give none.
    pub min_order_micro: i64,
    /// Whether the parts cost less than the least order.
    pub below_minimum: bool,
}

/// One order of a split purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitOrder {
    /// The supplier.
    pub supplier: String,
    /// The lines bought there, as indices into the lines of the bill, in order.
    pub lines: Vec<usize>,
    /// The cost of those parts, in millionths.
    pub parts_micro: i128,
    /// The shipping of the order, in millionths.
    pub shipping_micro: i128,
    /// Whether the parts cost less than the least order of the supplier.
    pub below_minimum: bool,
}

/// Where to buy the bill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcingPlan {
    /// The currency.
    pub currency: String,
    /// The lines of the bill.
    pub lines: usize,
    /// One summary for each supplier considered, the cheapest total of those that cover every
    /// line first, then the others by the lines they cover (more first) and by name.
    pub suppliers: Vec<SupplierSummary>,
    /// The index in `suppliers` of the cheapest supplier that covers every line, when there is
    /// one.
    pub single: Option<usize>,
    /// The orders of the split purchase, by supplier name.
    pub orders: Vec<SplitOrder>,
    /// The cost of the split purchase with the shipping, in millionths.
    pub split_total_micro: i128,
    /// The lines that no supplier considered has a price for, as indices into the lines.
    pub uncovered: Vec<usize>,
}

/// The value the settings give for a supplier (names are compared without regard to case); zero
/// when there is none.
fn setting_of(map: &BTreeMap<String, i64>, supplier: &str) -> i64 {
    map.iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(supplier))
        .map_or(0, |(_, value)| *value)
}

/// The shipping of an order of `parts` millionths: none for an empty order, none from the order
/// value that makes it free, else the fixed cost of the supplier.
fn shipping_of(settings: &CostSettings, supplier: &str, parts: i128) -> i128 {
    if parts <= 0 {
        return 0;
    }
    let free = setting_of(&settings.supplier_free_shipping_micro, supplier);
    if free > 0 && parts >= i128::from(free) {
        return 0;
    }
    i128::from(setting_of(&settings.supplier_shipping_micro, supplier))
}

/// The offer for `needed` parts out of the rows of one supplier: the greatest break reached, else
/// the smallest one.
fn offer_from(rows: &[&PriceRow], needed: u64, alternative: &str) -> Option<Offer> {
    let mut tiers: Vec<&PriceRow> = rows.to_vec();
    tiers.sort_by_key(|r| (r.min_qty, r.unit_price_micro, r.origin));
    tiers.dedup_by_key(|r| r.min_qty);
    let tier = tiers
        .iter()
        .rev()
        .find(|t| t.min_qty <= needed)
        .or_else(|| tiers.first())?;
    let buy_qty = needed.max(tier.min_qty);
    Some(Offer {
        supplier_code: tier.supplier_code.clone(),
        alternative: alternative.to_owned(),
        tier_min_qty: tier.min_qty,
        unit_price_micro: tier.unit_price_micro,
        buy_qty,
        cost_micro: i128::from(buy_qty) * i128::from(tier.unit_price_micro),
    })
}

/// What a supplier asks for a line: the part of the line, else the cheapest of its alternatives.
pub(crate) fn offer_of(line: &BomLine, supplier_rows: &[&PriceRow]) -> Option<Offer> {
    let own: Vec<&PriceRow> = supplier_rows
        .iter()
        .copied()
        .filter(|r| {
            row_matches(
                r,
                &line.value,
                &line.footprint,
                &line.manufacturer,
                &line.mpn,
            )
        })
        .collect();
    if let Some(offer) = offer_from(&own, line.needed, "") {
        return Some(offer);
    }
    let mut best: Option<Offer> = None;
    for alternative in &line.alternatives {
        let rows: Vec<&PriceRow> = supplier_rows
            .iter()
            .copied()
            .filter(|r| !r.mpn.is_empty() && r.mpn.eq_ignore_ascii_case(alternative))
            .collect();
        if let Some(offer) = offer_from(&rows, line.needed, alternative) {
            if best
                .as_ref()
                .is_none_or(|b| offer.cost_micro < b.cost_micro)
            {
                best = Some(offer);
            }
        }
    }
    best
}

/// The suppliers the plan considers, by name: those of the list in the settings (written as in
/// the list) or every supplier of the rows.
fn suppliers_considered(rows: &[&PriceRow], settings: &CostSettings) -> Vec<String> {
    if settings.suppliers.is_empty() {
        let names: BTreeSet<&str> = rows.iter().map(|r| r.supplier.as_str()).collect();
        return names.into_iter().map(str::to_owned).collect();
    }
    let mut seen = BTreeSet::new();
    let mut names: Vec<String> = settings
        .suppliers
        .iter()
        .filter(|n| seen.insert(n.to_ascii_lowercase()))
        .cloned()
        .collect();
    names.sort();
    names
}

/// Works out where to buy the bill. The prices in another currency than the one of the settings
/// are left out, as in the bill itself.
pub fn plan_sourcing(bom: &Bom, prices: &PriceBook, settings: &CostSettings) -> SourcingPlan {
    let usable: Vec<&PriceRow> = prices
        .rows
        .iter()
        .filter(|r| r.currency.is_empty() || r.currency.eq_ignore_ascii_case(&settings.currency))
        .collect();
    let names = suppliers_considered(&usable, settings);
    // offers[supplier][line]
    let offers: Vec<Vec<Option<Offer>>> = names
        .iter()
        .map(|name| {
            let rows: Vec<&PriceRow> = usable
                .iter()
                .copied()
                .filter(|r| r.supplier.eq_ignore_ascii_case(name))
                .collect();
            bom.lines.iter().map(|l| offer_of(l, &rows)).collect()
        })
        .collect();

    let mut suppliers: Vec<SupplierSummary> = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let covered: Vec<&Offer> = offers[index].iter().flatten().collect();
        let parts: i128 = covered.iter().map(|o| o.cost_micro).sum();
        let shipping = shipping_of(settings, name, parts);
        let min_order = setting_of(&settings.supplier_min_order_micro, name);
        suppliers.push(SupplierSummary {
            supplier: name.clone(),
            lines_covered: covered.len(),
            lines_total: bom.lines.len(),
            parts_micro: parts,
            shipping_micro: shipping,
            total_micro: parts + shipping,
            min_order_micro: min_order,
            below_minimum: !covered.is_empty() && parts < i128::from(min_order),
        });
    }
    // the order of the summaries: complete suppliers by total, then the rest by coverage
    let mut order: Vec<usize> = (0..suppliers.len()).collect();
    let complete = |s: &SupplierSummary| !bom.lines.is_empty() && s.lines_covered == s.lines_total;
    order.sort_by(|a, b| {
        let (x, y) = (&suppliers[*a], &suppliers[*b]);
        (!complete(x), if complete(x) { x.total_micro } else { 0 })
            .cmp(&(!complete(y), if complete(y) { y.total_micro } else { 0 }))
            .then(y.lines_covered.cmp(&x.lines_covered))
            .then(x.supplier.cmp(&y.supplier))
    });
    // the summaries in that order; the first is the cheapest single supplier when it is complete
    let sorted: Vec<SupplierSummary> = order.iter().map(|i| suppliers[*i].clone()).collect();
    let offers: Vec<Vec<Option<Offer>>> = order.iter().map(|i| offers[*i].clone()).collect();
    let single = sorted.first().filter(|s| complete(s)).map(|_| 0);

    // the split purchase: each line to its cheapest supplier, then orders dropped while it pays
    let mut assigned: Vec<Option<usize>> = Vec::new();
    let mut uncovered = Vec::new();
    for (line, _) in bom.lines.iter().enumerate() {
        let best = (0..sorted.len())
            .filter_map(|s| offers[s][line].as_ref().map(|o| (o.cost_micro, s)))
            .min_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(sorted[a.1].supplier.cmp(&sorted[b.1].supplier))
            });
        if best.is_none() {
            uncovered.push(line);
        }
        assigned.push(best.map(|(_, s)| s));
    }
    let total_of = |assigned: &[Option<usize>]| -> i128 {
        let mut parts: BTreeMap<usize, i128> = BTreeMap::new();
        for (line, s) in assigned.iter().enumerate() {
            if let Some(s) = s {
                if let Some(o) = &offers[*s][line] {
                    *parts.entry(*s).or_default() += o.cost_micro;
                }
            }
        }
        parts
            .iter()
            .map(|(s, p)| *p + shipping_of(settings, &sorted[*s].supplier, *p))
            .sum()
    };
    let mut current = total_of(&assigned);
    loop {
        let used: BTreeSet<usize> = assigned.iter().flatten().copied().collect();
        let mut improved = false;
        for drop in &used {
            let mut trial = assigned.clone();
            let mut possible = true;
            for (line, slot) in trial.iter_mut().enumerate() {
                if *slot != Some(*drop) {
                    continue;
                }
                let other = (0..sorted.len())
                    .filter(|s| *s != *drop)
                    .filter_map(|s| offers[s][line].as_ref().map(|o| (o.cost_micro, s)))
                    .min_by(|a, b| {
                        a.0.cmp(&b.0)
                            .then(sorted[a.1].supplier.cmp(&sorted[b.1].supplier))
                    });
                match other {
                    Some((_, s)) => *slot = Some(s),
                    None => {
                        possible = false;
                        break;
                    }
                }
            }
            if !possible {
                continue;
            }
            let cost = total_of(&trial);
            if cost < current {
                assigned = trial;
                current = cost;
                improved = true;
                break;
            }
        }
        if !improved {
            break;
        }
    }
    // the cheapest single supplier is a split of one order: it wins when the search above ended
    // dearer, which happens when the supplier that would take the lines had no line to start with
    if let Some(first) = single {
        let all: Vec<Option<usize>> = vec![Some(first); bom.lines.len()];
        let cost = total_of(&all);
        if cost < current {
            assigned = all;
            current = cost;
        }
    }
    let mut orders: Vec<SplitOrder> = Vec::new();
    let used: BTreeSet<usize> = assigned.iter().flatten().copied().collect();
    for s in used {
        let lines: Vec<usize> = assigned
            .iter()
            .enumerate()
            .filter(|(_, a)| **a == Some(s))
            .map(|(l, _)| l)
            .collect();
        let parts: i128 = lines
            .iter()
            .filter_map(|l| offers[s][*l].as_ref())
            .map(|o| o.cost_micro)
            .sum();
        let name = &sorted[s].supplier;
        orders.push(SplitOrder {
            supplier: name.clone(),
            lines,
            parts_micro: parts,
            shipping_micro: shipping_of(settings, name, parts),
            below_minimum: parts < i128::from(setting_of(&settings.supplier_min_order_micro, name)),
        });
    }
    orders.sort_by(|a, b| a.supplier.cmp(&b.supplier));
    SourcingPlan {
        currency: bom.currency.clone(),
        lines: bom.lines.len(),
        suppliers: sorted,
        single,
        orders,
        split_total_micro: current,
        uncovered,
    }
}

fn label(line: &BomLine) -> String {
    format!(
        "{} x{} {} {}",
        line.references.join(" "),
        line.qty,
        line.value,
        line.footprint
    )
    .trim()
    .to_owned()
}

/// The plan as text.
pub fn sourcing_text(plan: &SourcingPlan, bom: &Bom) -> String {
    let mut out = format!(
        "suppliers: {} considered, {} lines, {}\n",
        plan.suppliers.len(),
        plan.lines,
        plan.currency
    );
    for s in &plan.suppliers {
        out.push_str(&format!(
            "{}: {}/{} lines, parts {}, shipping {}, total {}{}\n",
            s.supplier,
            s.lines_covered,
            s.lines_total,
            format_micro(s.parts_micro),
            format_micro(s.shipping_micro),
            format_micro(s.total_micro),
            if s.below_minimum {
                format!(
                    " (below the least order of {})",
                    format_micro(i128::from(s.min_order_micro))
                )
            } else {
                String::new()
            }
        ));
    }
    match plan.single {
        Some(i) => out.push_str(&format!(
            "cheapest single supplier: {} for {}\n",
            plan.suppliers[i].supplier,
            format_micro(plan.suppliers[i].total_micro)
        )),
        None => out.push_str("no single supplier covers every line\n"),
    }
    out.push_str(&format!(
        "split purchase: {} orders for {}\n",
        plan.orders.len(),
        format_micro(plan.split_total_micro)
    ));
    for o in &plan.orders {
        out.push_str(&format!(
            "  {}: {} lines, parts {}, shipping {}{}\n",
            o.supplier,
            o.lines.len(),
            format_micro(o.parts_micro),
            format_micro(o.shipping_micro),
            if o.below_minimum {
                " (below the least order)"
            } else {
                ""
            }
        ));
    }
    for l in &plan.uncovered {
        if let Some(line) = bom.lines.get(*l) {
            out.push_str(&format!("no supplier has a price for {}\n", label(line)));
        }
    }
    out
}

/// The plan as JSON.
pub fn sourcing_json(plan: &SourcingPlan, bom: &Bom) -> Value {
    let suppliers: Vec<Value> = plan
        .suppliers
        .iter()
        .map(|s| {
            json!({
                "supplier": s.supplier,
                "lines_covered": s.lines_covered,
                "lines_total": s.lines_total,
                "parts": format_micro(s.parts_micro),
                "shipping": format_micro(s.shipping_micro),
                "total": format_micro(s.total_micro),
                "below_minimum": s.below_minimum,
            })
        })
        .collect();
    let orders: Vec<Value> = plan
        .orders
        .iter()
        .map(|o| {
            let lines: Vec<Value> = o
                .lines
                .iter()
                .filter_map(|l| bom.lines.get(*l))
                .map(|l| json!({"references": l.references, "value": l.value, "footprint": l.footprint}))
                .collect();
            json!({
                "supplier": o.supplier,
                "parts": format_micro(o.parts_micro),
                "shipping": format_micro(o.shipping_micro),
                "below_minimum": o.below_minimum,
                "lines": lines,
            })
        })
        .collect();
    let uncovered: Vec<Value> = plan
        .uncovered
        .iter()
        .filter_map(|l| bom.lines.get(*l))
        .map(|l| json!(l.references))
        .collect();
    json!({
        "currency": plan.currency,
        "suppliers": suppliers,
        "single": plan.single.map(|i| plan.suppliers[i].supplier.clone()),
        "split_total": format_micro(plan.split_total_micro),
        "orders": orders,
        "uncovered": uncovered,
    })
}
