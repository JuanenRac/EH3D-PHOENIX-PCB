// SPDX-License-Identifier: GPL-3.0-or-later
//! The bill of materials: the parts of a board (and of a schematic that is not placed yet)
//! grouped by value, footprint, maker and part number, with the quantity of each, the references,
//! the offer of the cheapest supplier for the quantity of the batch and the cost per board and
//! per batch.
//!
//! The quantity to buy is the quantity per board times the boards of the batch plus the waste,
//! rounded up. At each supplier the price is the one of the greatest quantity break that the
//! quantity to buy reaches; when it reaches none, the smallest break is bought (a finding says so)
//! at its price. The supplier with the least cost for the whole quantity is chosen; on a tie, the
//! first by name. A part without any offer costs nothing in the totals and the report says that
//! the totals are not complete.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_schematic::Schematic;
use eh3d_phoenix_stackup::Stackup;

use crate::codes::{
    BomError, Finding, Severity, CURRENCY_IGNORED, FOOTPRINT_MISSING, LIMIT_EXCEEDED,
    MINIMUM_RAISED, NOT_FITTED, NOT_PLACED, NO_FOOTPRINT, NO_PRICE, PRICE_CONFLICT,
};
use crate::money::{div_ceil, mul_div, MICRO};
use crate::parts::PartsFile;
use crate::pcb::{pcb_cost, PcbCost};
use crate::prices::{PriceBook, PriceRow};
use crate::settings::CostSettings;

/// Everything a bill of materials is made from.
pub struct BomInputs<'a> {
    /// The board; its components are the parts.
    pub board: &'a Board,
    /// The schematic, when there is one; its parts that are not on the board are added.
    pub schematic: Option<&'a Schematic>,
    /// The makers and part numbers.
    pub parts: &'a PartsFile,
    /// The prices.
    pub prices: &'a PriceBook,
    /// The numbers of the run.
    pub settings: &'a CostSettings,
    /// The stack-up, for the thickness of the board.
    pub stackup: Option<&'a Stackup>,
}

/// What is bought of a part and at what price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    /// The supplier.
    pub supplier: String,
    /// The code of the part at the supplier.
    pub supplier_code: String,
    /// The quantity break the price is of.
    pub tier_min_qty: u64,
    /// The price of one part, in millionths.
    pub unit_price_micro: i64,
    /// The quantity bought.
    pub buy_qty: u64,
    /// The cost of the quantity bought, in millionths.
    pub cost_micro: i128,
    /// The quantity breaks of the supplier: the quantity and the price.
    pub tiers: Vec<(u64, i64)>,
}

/// A line of the bill: the parts that are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    /// The value.
    pub value: String,
    /// The name of the footprint; empty when there is none.
    pub footprint: String,
    /// The maker; empty when it is not known.
    pub manufacturer: String,
    /// The part number; empty when it is not known.
    pub mpn: String,
    /// The references, in natural order (`R2` before `R10`).
    pub references: Vec<String>,
    /// The quantity in one board.
    pub qty: u64,
    /// The quantity the batch needs with the waste.
    pub needed: u64,
    /// The offer that was chosen, if there is one.
    pub purchase: Option<Purchase>,
    /// The cost of the line for one board, in millionths.
    pub cost_per_board_micro: i128,
}

/// The whole bill with its totals and findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bom {
    /// The lines in the order of their first reference.
    pub lines: Vec<BomLine>,
    /// The boards of the batch.
    pub boards: u64,
    /// The currency.
    pub currency: String,
    /// The waste, in millionths of a per cent.
    pub waste_percent_micro: i64,
    /// The cost of the parts of the batch, in millionths.
    pub parts_batch_micro: i128,
    /// The cost of the parts of one board, in millionths.
    pub parts_per_board_micro: i128,
    /// Lines with no offer.
    pub unpriced_lines: u64,
    /// Parts (not lines) with no offer.
    pub unpriced_parts: u64,
    /// Lines with no footprint.
    pub no_footprint_lines: u64,
    /// The estimate of the boards, when it could be worked out.
    pub pcb: Option<PcbCost>,
    /// The cost of the batch, parts and boards, in millionths.
    pub total_batch_micro: i128,
    /// The cost of one board, parts and board, in millionths.
    pub total_per_board_micro: i128,
    /// Whether every line has a price and the boards are costed.
    pub complete: bool,
    /// What the run found.
    pub findings: Vec<Finding>,
}

struct Entry {
    reference: String,
    value: String,
    footprint: String,
}

/// The key to sort references naturally: the letters, the number, the rest.
pub fn natural_key(reference: &str) -> (String, u64, String) {
    let split = reference
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(reference.len());
    let (letters, rest) = reference.split_at(split);
    let digits_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let (digits, tail) = rest.split_at(digits_end);
    let number = digits.parse::<u64>().unwrap_or(0);
    (letters.to_owned(), number, tail.to_owned())
}

fn footprint_name(board: &Board, id: Id) -> Option<String> {
    board.footprints.get(&id).map(|f| f.name.clone())
}

fn collect(inputs: &BomInputs, findings: &mut Vec<Finding>) -> Vec<Entry> {
    let board = inputs.board;
    let mut entries = Vec::new();
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    for component in board.components.values() {
        placed.insert(component.refdes.as_str());
        let footprint = match footprint_name(board, component.footprint) {
            Some(name) => name,
            None => {
                findings.push(Finding::new(
                    FOOTPRINT_MISSING,
                    Severity::Error,
                    Some(&component.refdes),
                    "the footprint of the component is not on the board",
                ));
                String::new()
            }
        };
        entries.push(Entry {
            reference: component.refdes.clone(),
            value: component.value.clone(),
            footprint,
        });
    }
    if let Some(schematic) = inputs.schematic {
        for part in schematic.parts.values() {
            if placed.contains(part.refdes.as_str()) {
                continue;
            }
            findings.push(Finding::new(
                NOT_PLACED,
                Severity::Warning,
                Some(&part.refdes),
                "the part is in the schematic and not on the board",
            ));
            let footprint = part
                .footprint
                .and_then(|id| footprint_name(board, id))
                .unwrap_or_default();
            entries.push(Entry {
                reference: part.refdes.clone(),
                value: part.value.clone(),
                footprint,
            });
        }
    }
    entries
}

fn row_matches(
    row: &PriceRow,
    value: &str,
    footprint: &str,
    manufacturer: &str,
    mpn: &str,
) -> bool {
    if !row.mpn.is_empty() {
        return !mpn.is_empty()
            && row.mpn.eq_ignore_ascii_case(mpn)
            && (row.manufacturer.is_empty()
                || manufacturer.is_empty()
                || row.manufacturer.eq_ignore_ascii_case(manufacturer));
    }
    !row.value.is_empty()
        && row.value == value
        && (row.footprint.is_empty() || row.footprint == footprint)
}

/// The best offer for a quantity, over the suppliers that have a row for the part.
fn choose_offer(
    rows: &[&PriceRow],
    needed: u64,
    subject: &str,
    findings: &mut Vec<Finding>,
) -> Option<Purchase> {
    let mut by_supplier: BTreeMap<&str, Vec<&PriceRow>> = BTreeMap::new();
    for row in rows {
        by_supplier
            .entry(row.supplier.as_str())
            .or_default()
            .push(row);
    }
    let mut best: Option<(Purchase, bool)> = None;
    for (supplier, mut tier_rows) in by_supplier {
        tier_rows.sort_by_key(|r| (r.min_qty, r.unit_price_micro, r.origin));
        let mut tiers: Vec<&PriceRow> = Vec::new();
        for row in tier_rows {
            match tiers.last() {
                Some(last) if last.min_qty == row.min_qty => {
                    if last.unit_price_micro != row.unit_price_micro {
                        findings.push(Finding::new(
                            PRICE_CONFLICT,
                            Severity::Warning,
                            Some(subject),
                            format!("{supplier}, from {}: the lower price is used", row.min_qty),
                        ));
                    }
                }
                _ => tiers.push(row),
            }
        }
        let reached = tiers.iter().rev().find(|t| t.min_qty <= needed);
        let (tier, raised) = match reached {
            Some(tier) => (*tier, false),
            None => match tiers.first() {
                Some(first) => (*first, true),
                None => continue,
            },
        };
        let buy_qty = needed.max(tier.min_qty);
        let cost_micro = i128::from(buy_qty) * i128::from(tier.unit_price_micro);
        let better = match &best {
            None => true,
            Some((current, _)) => cost_micro < current.cost_micro,
        };
        if better {
            best = Some((
                Purchase {
                    supplier: supplier.to_owned(),
                    supplier_code: tier.supplier_code.clone(),
                    tier_min_qty: tier.min_qty,
                    unit_price_micro: tier.unit_price_micro,
                    buy_qty,
                    cost_micro,
                    tiers: tiers
                        .iter()
                        .map(|t| (t.min_qty, t.unit_price_micro))
                        .collect(),
                },
                raised,
            ));
        }
    }
    let (purchase, raised) = best?;
    if raised {
        findings.push(Finding::new(
            MINIMUM_RAISED,
            Severity::Info,
            Some(subject),
            format!(
                "{}: {} needed, {} bought",
                purchase.supplier, needed, purchase.buy_qty
            ),
        ));
    }
    Some(purchase)
}

/// Builds the bill of materials.
pub fn build_bom(inputs: &BomInputs) -> Result<Bom, BomError> {
    let settings = inputs.settings;
    let mut findings = Vec::new();
    let component_count =
        inputs.board.components.len() + inputs.schematic.map_or(0, |s| s.parts.len());
    if component_count > settings.max_items {
        return Err(BomError::new(
            LIMIT_EXCEEDED,
            format!(
                "{component_count} components, the limit is {}",
                settings.max_items
            ),
        ));
    }

    // the prices in another currency are left out
    let mut other_currencies: BTreeSet<String> = BTreeSet::new();
    let mut ignored = 0u64;
    let mut usable: Vec<&PriceRow> = Vec::new();
    for row in &inputs.prices.rows {
        if row.currency.is_empty() || row.currency.eq_ignore_ascii_case(&settings.currency) {
            usable.push(row);
        } else {
            ignored += 1;
            other_currencies.insert(row.currency.to_ascii_uppercase());
        }
    }
    if ignored > 0 {
        let names: Vec<String> = other_currencies.into_iter().collect();
        findings.push(Finding::new(
            CURRENCY_IGNORED,
            Severity::Warning,
            None,
            format!(
                "{ignored} rows in {} left out; the run is in {}",
                names.join(", "),
                settings.currency
            ),
        ));
    }

    // the parts, grouped
    type Key = (String, String, String, String);
    let mut groups: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    for entry in collect(inputs, &mut findings) {
        let data = inputs
            .parts
            .find(&entry.reference, &entry.value, &entry.footprint);
        if let Some(data) = data {
            if !data.fitted {
                findings.push(Finding::new(
                    NOT_FITTED,
                    Severity::Info,
                    Some(&entry.reference),
                    "marked not fitted in the file of part data",
                ));
                continue;
            }
        }
        let (manufacturer, mpn) = data
            .map(|d| (d.manufacturer.clone(), d.mpn.clone()))
            .unwrap_or_default();
        // a maker or a part number that does not separate lines is dropped from the key, so the
        // line has none and its prices are looked up by value and footprint
        let manufacturer = if settings.group_by.manufacturer {
            manufacturer
        } else {
            String::new()
        };
        let mpn = if settings.group_by.mpn {
            mpn
        } else {
            String::new()
        };
        groups
            .entry((entry.value, entry.footprint, manufacturer, mpn))
            .or_default()
            .push(entry.reference);
    }

    let boards = settings.boards;
    let whole = 100 * i128::from(MICRO);
    let factor = whole + i128::from(settings.parts_waste_percent_micro);
    let mut lines = Vec::new();
    for ((value, footprint, manufacturer, mpn), mut references) in groups {
        references.sort_by_key(|r| natural_key(r));
        let qty = references.len() as u64;
        let needed = div_ceil(i128::from(qty) * i128::from(boards) * factor, whole) as u64;
        let subject = references.join(", ");
        if footprint.is_empty() {
            findings.push(Finding::new(
                NO_FOOTPRINT,
                Severity::Warning,
                Some(&subject),
                format!("value {value}"),
            ));
        }
        let rows: Vec<&PriceRow> = usable
            .iter()
            .copied()
            .filter(|r| row_matches(r, &value, &footprint, &manufacturer, &mpn))
            .collect();
        let purchase = choose_offer(&rows, needed, &subject, &mut findings);
        if purchase.is_none() {
            findings.push(Finding::new(
                NO_PRICE,
                Severity::Warning,
                Some(&subject),
                format!("{value} {footprint}"),
            ));
        }
        let cost_per_board_micro = purchase
            .as_ref()
            .map_or(0, |p| mul_div(p.cost_micro, 1, i128::from(boards)));
        lines.push(BomLine {
            value,
            footprint,
            manufacturer,
            mpn,
            references,
            qty,
            needed,
            purchase,
            cost_per_board_micro,
        });
    }
    lines.sort_by_key(|l| natural_key(&l.references[0]));

    let parts_batch_micro: i128 = lines
        .iter()
        .filter_map(|l| l.purchase.as_ref())
        .map(|p| p.cost_micro)
        .sum();
    let parts_per_board_micro = mul_div(parts_batch_micro, 1, i128::from(boards));
    let unpriced_lines = lines.iter().filter(|l| l.purchase.is_none()).count() as u64;
    let unpriced_parts = lines
        .iter()
        .filter(|l| l.purchase.is_none())
        .map(|l| l.qty)
        .sum();
    let no_footprint_lines = lines.iter().filter(|l| l.footprint.is_empty()).count() as u64;

    let pcb = pcb_cost(inputs.board, inputs.stackup, settings, &mut findings);
    let pcb_batch = pcb.as_ref().map_or(0, |p| p.batch_micro);
    let total_batch_micro = parts_batch_micro + pcb_batch;
    let total_per_board_micro = mul_div(total_batch_micro, 1, i128::from(boards));
    let complete = unpriced_lines == 0 && pcb.is_some();
    Ok(Bom {
        lines,
        boards,
        currency: settings.currency.clone(),
        waste_percent_micro: settings.parts_waste_percent_micro,
        parts_batch_micro,
        parts_per_board_micro,
        unpriced_lines,
        unpriced_parts,
        no_footprint_lines,
        pcb,
        total_batch_micro,
        total_per_board_micro,
        complete,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_sort_by_letters_then_number() {
        let mut v = vec!["R10", "C1", "R2", "R1", "U3", "R2A"];
        v.sort_by_key(|r| natural_key(r));
        assert_eq!(v, vec!["C1", "R1", "R2", "R2A", "R10", "U3"]);
        assert_eq!(natural_key("TP"), ("TP".to_owned(), 0, String::new()));
    }

    fn row(supplier: &str, min_qty: u64, price: i64) -> PriceRow {
        PriceRow {
            value: "10k".to_owned(),
            footprint: String::new(),
            manufacturer: String::new(),
            mpn: String::new(),
            supplier: supplier.to_owned(),
            supplier_code: format!("{supplier}-{min_qty}"),
            min_qty,
            unit_price_micro: price,
            currency: String::new(),
            origin: 1,
        }
    }

    #[test]
    fn the_greatest_break_reached_is_used_and_the_cheapest_supplier_wins() {
        let a1 = row("A", 1, 10_000);
        let a100 = row("A", 100, 4_000);
        let b1 = row("B", 1, 5_000);
        let rows = vec![&a1, &a100, &b1];
        let mut findings = Vec::new();
        let p = choose_offer(&rows, 150, "R1", &mut findings).unwrap();
        assert_eq!(p.supplier, "A");
        assert_eq!(p.tier_min_qty, 100);
        assert_eq!(p.cost_micro, 600_000);
        assert_eq!(p.tiers, vec![(1, 10_000), (100, 4_000)]);
        let p = choose_offer(&rows, 50, "R1", &mut findings).unwrap();
        assert_eq!(p.supplier, "B");
        assert_eq!(p.cost_micro, 250_000);
        assert!(findings.is_empty());
    }

    #[test]
    fn below_the_first_break_the_minimum_is_bought_and_a_tie_goes_to_the_first_name() {
        let a = row("A", 10, 1_000);
        let b = row("B", 10, 1_000);
        let mut findings = Vec::new();
        let p = choose_offer(&[&b, &a], 3, "C1", &mut findings).unwrap();
        assert_eq!(
            (p.supplier.as_str(), p.buy_qty, p.cost_micro),
            ("A", 10, 10_000)
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, MINIMUM_RAISED);
        assert!(choose_offer(&[], 3, "C1", &mut findings).is_none());
    }

    #[test]
    fn two_prices_for_one_break_are_a_finding_and_the_lower_is_used() {
        let low = row("A", 1, 2_000);
        let high = row("A", 1, 3_000);
        let mut findings = Vec::new();
        let p = choose_offer(&[&high, &low], 10, "R1", &mut findings).unwrap();
        assert_eq!(p.unit_price_micro, 2_000);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, PRICE_CONFLICT);
    }

    #[test]
    fn a_row_with_a_part_number_matches_it_and_a_row_without_one_matches_value_and_footprint() {
        let mut r = row("A", 1, 1);
        r.mpn = "AC-1".to_owned();
        r.manufacturer = "Acme".to_owned();
        assert!(row_matches(&r, "x", "y", "", "ac-1"));
        assert!(row_matches(&r, "x", "y", "ACME", "AC-1"));
        assert!(!row_matches(&r, "x", "y", "Other", "AC-1"));
        assert!(!row_matches(&r, "10k", "R0603", "", ""));
        let mut by_value = row("A", 1, 1);
        assert!(row_matches(&by_value, "10k", "anything", "", ""));
        by_value.footprint = "R0603".to_owned();
        assert!(row_matches(&by_value, "10k", "R0603", "", ""));
        assert!(!row_matches(&by_value, "10k", "R0402", "", ""));
        assert!(!row_matches(&by_value, "22k", "R0603", "", ""));
    }
}
