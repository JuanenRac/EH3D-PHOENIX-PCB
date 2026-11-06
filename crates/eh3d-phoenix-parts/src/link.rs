// SPDX-License-Identifier: GPL-3.0-or-later
//! The link of the base to the bill of materials. For each part of the design the base is asked
//! for the maker, the part number and the prices:
//!
//! * a part for which the file of part data of the bill already gives a part number is looked up
//!   by that number (and the maker, when it gives one); a number that is not in the base is a part
//!   without a match, and it is never replaced by another one;
//! * the other parts are looked up by value and footprint: the value is compared as
//!   [`norm_value`](crate::model::norm_value) does and the footprint without case; a part of the
//!   base with no footprint matches any footprint, unless another one has the footprint of the
//!   design;
//! * among several parts of the base the ones that are obsolete are left out when another one
//!   remains, and then the ones that are not active when an active one remains; when more than
//!   one remains the design part is reported with all of them and gets none, because nothing is
//!   guessed.
//!
//! The result is a file of part data and a price book of the same kinds that `phoenix bom` reads,
//! made from the files the user gave plus the base (what the user gave wins), and a report of the
//! parts without a match, with several, and the proposals of alternatives for the parts that are
//! obsolete or not recommended. The proposals are only a list: nothing in the design is changed.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_bom::{natural_key, PartData, PartsFile, PriceBook, PriceRow};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::Schematic;
use serde_json::{json, Value};

use crate::codes::{Finding, Severity, NO_MATCH, OBSOLETE_PART, SEVERAL_MATCHES};
use crate::index::Index;
use crate::model::{norm_value, Part, PartsDb, Status};

/// A part of the design that is looked up in the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEntry {
    /// The reference designator.
    pub reference: String,
    /// The value.
    pub value: String,
    /// The name of the footprint; empty when there is none.
    pub footprint: String,
}

/// How a part was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    /// By the part number that the file of part data gave.
    Mpn,
    /// By value and footprint.
    ValueFootprint,
}

impl Via {
    /// The word used in the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Via::Mpn => "mpn",
            Via::ValueFootprint => "value+footprint",
        }
    }
}

/// What the base says about one part of the design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// One part of the base.
    Matched {
        /// Its id.
        id: String,
        /// How it was found.
        via: Via,
    },
    /// No part of the base.
    Unmatched {
        /// Why.
        why: String,
    },
    /// Several parts of the base: none is taken.
    Several {
        /// Their ids, in order.
        ids: Vec<String>,
    },
}

/// One line of the link: a part of the design and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkLine {
    /// The part of the design.
    pub entry: LinkEntry,
    /// What the base says.
    pub outcome: Outcome,
}

/// The proposal of other parts for a part of the base that is obsolete or not recommended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// The id of the part that is used.
    pub part_id: String,
    /// Its state.
    pub status: Status,
    /// The references of the design that use it, in natural order.
    pub references: Vec<String>,
    /// The ids of the active parts that could take its place: the alternatives it names and the
    /// parts with the same value and footprint. It can be empty.
    pub alternatives: Vec<String>,
}

/// The report of a link.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkReport {
    /// One line for each part of the design, in natural order of the references.
    pub lines: Vec<LinkLine>,
    /// The proposals of alternatives, in the order of the ids.
    pub proposals: Vec<Proposal>,
    /// The findings, sorted.
    pub findings: Vec<Finding>,
}

/// The files of the bill completed from the base, and the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linked {
    /// The part data: the file the user gave plus, for each part found, its maker and part number.
    pub parts: PartsFile,
    /// The prices: the book the user gave plus the offers of the parts found.
    pub prices: PriceBook,
    /// The report.
    pub report: LinkReport,
}

/// The parts of a design, as the bill of materials sees them: the components of the board and the
/// parts of the schematic that are not on the board.
pub fn entries_of(board: &Board, schematic: Option<&Schematic>) -> Vec<LinkEntry> {
    let name = |id: Id| board.footprints.get(&id).map(|f| f.name.clone());
    let mut entries = Vec::new();
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    for component in board.components.values() {
        placed.insert(component.refdes.as_str());
        entries.push(LinkEntry {
            reference: component.refdes.clone(),
            value: component.value.clone(),
            footprint: name(component.footprint).unwrap_or_default(),
        });
    }
    if let Some(schematic) = schematic {
        for part in schematic.parts.values() {
            if placed.contains(part.refdes.as_str()) {
                continue;
            }
            entries.push(LinkEntry {
                reference: part.refdes.clone(),
                value: part.value.clone(),
                footprint: part.footprint.and_then(name).unwrap_or_default(),
            });
        }
    }
    entries
}

fn choose(mut candidates: Vec<&Part>, via: Via, none: String) -> Outcome {
    candidates.dedup_by(|a, b| a.id == b.id);
    if candidates.is_empty() {
        return Outcome::Unmatched { why: none };
    }
    if candidates.iter().any(|p| p.status != Status::Obsolete) {
        candidates.retain(|p| p.status != Status::Obsolete);
    }
    if candidates.len() > 1 && candidates.iter().any(|p| p.status == Status::Active) {
        candidates.retain(|p| p.status == Status::Active);
    }
    match candidates.as_slice() {
        [one] => Outcome::Matched {
            id: one.id.clone(),
            via,
        },
        many => Outcome::Several {
            ids: many.iter().map(|p| p.id.clone()).collect(),
        },
    }
}

fn find_by_number(index: &Index, number: &str, maker: &str) -> Outcome {
    let candidates: Vec<&Part> = index
        .with_mpn(number)
        .into_iter()
        .filter(|p| {
            maker.is_empty()
                || p.manufacturer.is_empty()
                || p.manufacturer.eq_ignore_ascii_case(maker)
        })
        .collect();
    choose(
        candidates,
        Via::Mpn,
        format!("the part number {number} is not in the base"),
    )
}

fn find_by_kind(index: &Index, entry: &LinkEntry) -> Outcome {
    if entry.value.trim().is_empty() {
        return Outcome::Unmatched {
            why: "the part has no value".to_owned(),
        };
    }
    let mut candidates: Vec<&Part> = index
        .with_value(&entry.value)
        .into_iter()
        .filter(|p| {
            p.footprint.is_empty()
                || entry.footprint.is_empty()
                || p.footprint.eq_ignore_ascii_case(&entry.footprint)
        })
        .collect();
    if !entry.footprint.is_empty() && candidates.iter().any(|p| !p.footprint.is_empty()) {
        candidates.retain(|p| !p.footprint.is_empty());
    }
    choose(
        candidates,
        Via::ValueFootprint,
        format!("no part with the value {}", entry.value),
    )
}

/// The active parts that could take the place of a part: the alternatives it names (by id or by
/// part number) and the parts with the same value and footprint.
fn alternatives_of(index: &Index, db: &PartsDb, part: &Part) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    for name in &part.alternatives {
        if let Some(p) = index.get(name) {
            found.insert(p.id.clone());
        }
        for p in index.with_mpn(name) {
            found.insert(p.id.clone());
        }
    }
    if !part.value.is_empty() {
        let value = norm_value(&part.value);
        for p in &db.parts {
            if norm_value(&p.value) == value && p.footprint.eq_ignore_ascii_case(&part.footprint) {
                found.insert(p.id.clone());
            }
        }
    }
    found.remove(&part.id);
    found
        .into_iter()
        .filter(|id| index.get(id).is_some_and(|p| p.status == Status::Active))
        .collect()
}

/// Looks every part of the design up in the base and completes the files of the bill.
pub fn link_bom(
    db: &PartsDb,
    entries: &[LinkEntry],
    user_parts: &PartsFile,
    user_prices: &PriceBook,
) -> Linked {
    let index = Index::build(db);
    let mut lines: Vec<LinkLine> = Vec::new();
    for entry in entries {
        let given = user_parts
            .find(&entry.reference, &entry.value, &entry.footprint)
            .filter(|d| !d.mpn.is_empty());
        let outcome = match given {
            Some(d) => find_by_number(&index, &d.mpn, &d.manufacturer),
            None => find_by_kind(&index, entry),
        };
        lines.push(LinkLine {
            entry: entry.clone(),
            outcome,
        });
    }
    lines.sort_by_key(|l| natural_key(&l.entry.reference));

    let mut findings: Vec<Finding> = Vec::new();
    let mut used: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut parts = user_parts.clone();
    for line in &lines {
        let e = &line.entry;
        match &line.outcome {
            Outcome::Unmatched { why } => findings.push(Finding::new(
                NO_MATCH,
                Severity::Warning,
                &e.reference,
                "",
                format!("{} {}: {why}", e.value, e.footprint)
                    .trim()
                    .to_owned(),
            )),
            Outcome::Several { ids } => findings.push(Finding::new(
                SEVERAL_MATCHES,
                Severity::Warning,
                &e.reference,
                "",
                format!("{} {}: {}", e.value, e.footprint, ids.join(", "))
                    .trim()
                    .to_owned(),
            )),
            Outcome::Matched { id, .. } => {
                let Some(part) = index.get(id) else { continue };
                let existing = user_parts.find(&e.reference, &e.value, &e.footprint);
                let pick = |given: Option<&str>, base: &str| -> String {
                    match given {
                        Some(g) if !g.is_empty() => g.to_owned(),
                        _ => base.to_owned(),
                    }
                };
                parts.by_reference.insert(
                    e.reference.clone(),
                    PartData {
                        manufacturer: pick(
                            existing.map(|d| d.manufacturer.as_str()),
                            &part.manufacturer,
                        ),
                        mpn: pick(existing.map(|d| d.mpn.as_str()), &part.mpn),
                        fitted: existing.is_none_or(|d| d.fitted),
                    },
                );
                used.entry(id.clone())
                    .or_default()
                    .push(e.reference.clone());
                if part.status != Status::Active {
                    findings.push(Finding::new(
                        OBSOLETE_PART,
                        Severity::Warning,
                        &e.reference,
                        &part.origin,
                        format!("{} is {}", part.id, part.status.as_str()),
                    ));
                }
            }
        }
    }
    findings.sort();

    let mut proposals = Vec::new();
    let mut rows: Vec<PriceRow> = user_prices.rows.clone();
    for (id, references) in &used {
        let Some(part) = index.get(id) else { continue };
        if part.status != Status::Active {
            proposals.push(Proposal {
                part_id: part.id.clone(),
                status: part.status,
                references: references.clone(),
                alternatives: alternatives_of(&index, db, part),
            });
        }
        for offer in &part.offers {
            for tier in &offer.tiers {
                let by_number = !part.mpn.is_empty();
                rows.push(PriceRow {
                    value: if by_number {
                        String::new()
                    } else {
                        part.value.clone()
                    },
                    footprint: if by_number {
                        String::new()
                    } else {
                        part.footprint.clone()
                    },
                    manufacturer: String::new(),
                    mpn: part.mpn.clone(),
                    supplier: offer.supplier.clone(),
                    supplier_code: offer.supplier_code.clone(),
                    min_qty: tier.min_qty,
                    unit_price_micro: tier.unit_price_micro,
                    currency: tier.currency.clone(),
                    origin: 0,
                });
            }
        }
    }
    Linked {
        parts,
        prices: PriceBook { rows },
        report: LinkReport {
            lines,
            proposals,
            findings,
        },
    }
}

impl LinkReport {
    /// The number of parts of the design that were matched, that have no match and that have
    /// several.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut matched = 0;
        let mut none = 0;
        let mut several = 0;
        for line in &self.lines {
            match line.outcome {
                Outcome::Matched { .. } => matched += 1,
                Outcome::Unmatched { .. } => none += 1,
                Outcome::Several { .. } => several += 1,
            }
        }
        (matched, none, several)
    }

    /// The report as text: the counts, the findings in a language and the proposals.
    pub fn to_text(&self, lang: Lang) -> String {
        let (matched, none, several) = self.counts();
        let mut out = format!(
            "parts base: {matched} matched, {none} without a match, {several} with several matches\n"
        );
        for f in &self.findings {
            out.push_str(&f.line(lang));
            out.push('\n');
        }
        for p in &self.proposals {
            let alternatives = if p.alternatives.is_empty() {
                "none in the base".to_owned()
            } else {
                p.alternatives.join(", ")
            };
            out.push_str(&format!(
                "proposal: {} ({}) used by {}; alternatives: {} (nothing is changed)\n",
                p.part_id,
                p.status.as_str(),
                p.references.join(", "),
                alternatives
            ));
        }
        out
    }

    /// The report as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let lines: Vec<Value> = self
            .lines
            .iter()
            .map(|l| {
                let outcome = match &l.outcome {
                    Outcome::Matched { id, via } => {
                        json!({ "result": "matched", "part": id, "via": via.as_str() })
                    }
                    Outcome::Unmatched { why } => json!({ "result": "unmatched", "why": why }),
                    Outcome::Several { ids } => json!({ "result": "several", "parts": ids }),
                };
                json!({
                    "reference": l.entry.reference,
                    "value": l.entry.value,
                    "footprint": l.entry.footprint,
                    "outcome": outcome,
                })
            })
            .collect();
        let proposals: Vec<Value> = self
            .proposals
            .iter()
            .map(|p| {
                json!({
                    "part": p.part_id,
                    "status": p.status.as_str(),
                    "references": p.references,
                    "alternatives": p.alternatives,
                })
            })
            .collect();
        let findings: Vec<Value> = self.findings.iter().map(Finding::to_json).collect();
        let document = json!({
            "format": "phoenix-parts-link/1",
            "lines": lines,
            "proposals": proposals,
            "findings": findings,
        });
        let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Offer, Tier};

    fn part(id: &str, value: &str, footprint: &str, mpn: &str) -> Part {
        Part {
            id: id.into(),
            value: value.into(),
            footprint: footprint.into(),
            mpn: mpn.into(),
            manufacturer: if mpn.is_empty() { "" } else { "Acme" }.into(),
            origin: format!("base.json: part {id}"),
            ..Part::default()
        }
    }

    fn priced(mut p: Part, supplier: &str, micro: i64) -> Part {
        p.offers = vec![Offer {
            supplier: supplier.into(),
            supplier_code: format!("{supplier}-1"),
            tiers: vec![Tier {
                min_qty: 1,
                unit_price_micro: micro,
                currency: "EUR".into(),
            }],
        }];
        p
    }

    fn entry(reference: &str, value: &str, footprint: &str) -> LinkEntry {
        LinkEntry {
            reference: reference.into(),
            value: value.into(),
            footprint: footprint.into(),
        }
    }

    fn matched(line: &LinkLine) -> Option<&str> {
        match &line.outcome {
            Outcome::Matched { id, .. } => Some(id.as_str()),
            _ => None,
        }
    }

    fn db() -> PartsDb {
        let mut old = part("r-22k-old", "22k", "R0603", "OLD-22K");
        old.status = Status::Obsolete;
        old.alternatives = vec!["r-22k".into(), "ghost".into()];
        PartsDb {
            parts: vec![
                priced(part("r-10k", "10k", "R0603", "AC-10K"), "Shop A", 10_000),
                priced(
                    part("r-10k-0805", "10k", "R0805", "AC-10K8"),
                    "Shop A",
                    20_000,
                ),
                part("c-any", "100n", "", ""),
                part("c-0603", "100n", "C0603", "C-100N"),
                part("r-22k", "22k", "R0603", "AC-22K"),
                old,
                part("x-a", "1k", "R0603", "XA-1"),
                part("x-b", "1k", "R0603", "XB-1"),
            ],
        }
    }

    #[test]
    fn value_and_footprint_find_one_part_and_the_closest_footprint_wins() {
        let entries = [
            entry("C1", "100n", "C0603"),
            entry("C2", "100n", "C0402"),
            entry("R1", "10 k\u{3a9}", "r0603"),
            entry("R2", "10k", "R0805"),
            entry("R3", "47k", "R0603"),
            entry("R4", "", "R0603"),
        ];
        let linked = link_bom(
            &db(),
            &entries,
            &PartsFile::default(),
            &PriceBook::default(),
        );
        let lines = &linked.report.lines;
        assert_eq!(matched(&lines[0]), Some("c-0603"));
        // the part without a footprint matches a footprint that has no part of its own
        assert_eq!(matched(&lines[1]), Some("c-any"));
        assert_eq!(matched(&lines[2]), Some("r-10k"));
        assert_eq!(matched(&lines[3]), Some("r-10k-0805"));
        assert!(matches!(&lines[4].outcome, Outcome::Unmatched { why } if why.contains("47k")));
        assert!(
            matches!(&lines[5].outcome, Outcome::Unmatched { why } if why.contains("no value"))
        );
        assert_eq!(linked.report.counts(), (4, 2, 0));
    }

    #[test]
    fn several_parts_are_reported_with_all_of_them_and_none_is_taken() {
        let linked = link_bom(
            &db(),
            &[entry("R1", "1k", "R0603")],
            &PartsFile::default(),
            &PriceBook::default(),
        );
        assert_eq!(
            linked.report.lines[0].outcome,
            Outcome::Several {
                ids: vec!["x-a".into(), "x-b".into()]
            }
        );
        assert_eq!(linked.report.findings[0].code, SEVERAL_MATCHES);
        assert!(linked.report.findings[0].detail.contains("x-a, x-b"));
        assert!(linked.parts.by_reference.is_empty());
        assert_eq!(linked.report.counts(), (0, 0, 1));
    }

    #[test]
    fn an_obsolete_part_is_left_out_when_another_remains_and_is_proposed_for_change_when_used() {
        let linked = link_bom(
            &db(),
            &[entry("R1", "22k", "R0603")],
            &PartsFile::default(),
            &PriceBook::default(),
        );
        assert_eq!(matched(&linked.report.lines[0]), Some("r-22k"));
        assert!(linked.report.proposals.is_empty());
        // the owner pins the obsolete number: it is used, and an alternative is proposed
        let pinned = PartsFile::parse("match,mpn\nR1,OLD-22K\n", 10).unwrap();
        let linked = link_bom(
            &db(),
            &[entry("R1", "22k", "R0603")],
            &pinned,
            &PriceBook::default(),
        );
        assert_eq!(matched(&linked.report.lines[0]), Some("r-22k-old"));
        assert_eq!(
            linked.report.proposals,
            vec![Proposal {
                part_id: "r-22k-old".into(),
                status: Status::Obsolete,
                references: vec!["R1".into()],
                alternatives: vec!["r-22k".into()],
            }]
        );
        let finding = &linked.report.findings[0];
        assert_eq!(finding.code, OBSOLETE_PART);
        assert_eq!(finding.origin, "base.json: part r-22k-old");
        let text = linked.report.to_text(Lang::En);
        assert!(text.contains(
            "proposal: r-22k-old (obsolete) used by R1; alternatives: r-22k (nothing is changed)"
        ));
    }

    #[test]
    fn a_number_that_the_owner_gave_is_looked_up_and_never_replaced_by_another_part() {
        let pinned = PartsFile::parse(
            "match,mpn,manufacturer\nR1,NOPE-1,\nR2,AC-10K,Other\nR3,ac-10k,acme\n",
            10,
        )
        .unwrap();
        let entries = [
            entry("R1", "10k", "R0603"),
            entry("R2", "10k", "R0603"),
            entry("R3", "22k", "R0603"),
        ];
        let linked = link_bom(&db(), &entries, &pinned, &PriceBook::default());
        let lines = &linked.report.lines;
        assert!(matches!(&lines[0].outcome, Outcome::Unmatched { why } if why.contains("NOPE-1")));
        assert!(matches!(&lines[1].outcome, Outcome::Unmatched { .. }));
        assert_eq!(
            lines[2].outcome,
            Outcome::Matched {
                id: "r-10k".into(),
                via: Via::Mpn
            }
        );
        assert_eq!(linked.report.findings[0].code, NO_MATCH);
    }

    #[test]
    fn the_files_of_the_bill_get_the_maker_the_number_and_the_prices_and_what_the_owner_gave_wins()
    {
        let user = PartsFile::parse("match,manufacturer,mpn,fitted\nR2,Mine,,no\n", 10).unwrap();
        let user_prices = PriceBook::parse("mpn,supplier,unit_price\nZZ,Own Shop,1\n", 10).unwrap();
        let entries = [entry("R1", "10k", "R0603"), entry("R2", "10k", "R0603")];
        let linked = link_bom(&db(), &entries, &user, &user_prices);
        let r1 = linked.parts.find("R1", "10k", "R0603").unwrap();
        assert_eq!(
            (r1.manufacturer.as_str(), r1.mpn.as_str(), r1.fitted),
            ("Acme", "AC-10K", true)
        );
        let r2 = linked.parts.find("R2", "10k", "R0603").unwrap();
        assert_eq!(
            (r2.manufacturer.as_str(), r2.mpn.as_str(), r2.fitted),
            ("Mine", "AC-10K", false)
        );
        // the book of the owner comes first, then one row for each price break of each part used
        assert_eq!(linked.prices.rows.len(), 2);
        assert_eq!(linked.prices.rows[0].supplier, "Own Shop");
        let row = &linked.prices.rows[1];
        assert_eq!(
            (
                row.mpn.as_str(),
                row.supplier.as_str(),
                row.unit_price_micro
            ),
            ("AC-10K", "Shop A", 10_000)
        );
        assert_eq!(row.currency, "EUR");
    }

    #[test]
    fn a_part_without_a_number_is_priced_by_its_value_and_footprint() {
        let mut p = priced(part("r-1k", "1k", "R0603", ""), "Shop B", 5_000);
        p.manufacturer = String::new();
        let db = PartsDb { parts: vec![p] };
        let linked = link_bom(
            &db,
            &[entry("R1", "1k", "R0603")],
            &PartsFile::default(),
            &PriceBook::default(),
        );
        let row = &linked.prices.rows[0];
        assert_eq!(
            (row.value.as_str(), row.footprint.as_str(), row.mpn.as_str()),
            ("1k", "R0603", "")
        );
    }

    #[test]
    fn the_completed_files_give_the_bill_a_price_and_the_report_is_the_same_every_time() {
        let entries = [entry("R1", "10k", "R0603")];
        let a = link_bom(
            &db(),
            &entries,
            &PartsFile::default(),
            &PriceBook::default(),
        );
        let b = link_bom(
            &db(),
            &entries,
            &PartsFile::default(),
            &PriceBook::default(),
        );
        assert_eq!(a, b);
        assert_eq!(a.report.to_json(), b.report.to_json());
        let json: Value = serde_json::from_str(&a.report.to_json()).unwrap();
        assert_eq!(json["lines"][0]["outcome"]["part"], "r-10k");
        assert_eq!(json["lines"][0]["outcome"]["via"], "value+footprint");
    }

    #[test]
    fn the_parts_of_a_design_are_the_components_and_the_schematic_parts_not_on_the_board() {
        use eh3d_phoenix_board::{Component, Footprint, Side};
        use eh3d_phoenix_model::{ObjectId, Point, Udeg};
        use eh3d_phoenix_schematic::Part as SchematicPart;

        let id = |n: u32| -> Id {
            ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
        };
        let mut board = Board::default();
        board.footprints.insert(
            id(10),
            Footprint {
                name: "R0603".into(),
                pads: Default::default(),
            },
        );
        board.components.insert(
            id(20),
            Component {
                refdes: "R1".into(),
                value: "10k".into(),
                symbol: None,
                footprint: id(10),
                at: Point::from_nm(0, 0).unwrap(),
                rotation: Udeg::new(0).unwrap(),
                side: Side::Top,
            },
        );
        let mut schematic = Schematic::default();
        for (n, reference, value) in [(60, "R1", "10k"), (62, "R7", "22k")] {
            schematic.parts.insert(
                id(n),
                SchematicPart {
                    refdes: reference.into(),
                    value: value.into(),
                    symbol: id(61),
                    footprint: Some(id(10)),
                    placement: None,
                },
            );
        }
        assert_eq!(
            entries_of(&board, Some(&schematic)),
            vec![entry("R1", "10k", "R0603"), entry("R7", "22k", "R0603")]
        );
        assert_eq!(entries_of(&board, None), vec![entry("R1", "10k", "R0603")]);
    }

    #[test]
    fn a_text_in_another_language_names_the_code_in_that_language() {
        let linked = link_bom(
            &db(),
            &[entry("R9", "47k", "R0603")],
            &PartsFile::default(),
            &PriceBook::default(),
        );
        let text = linked.report.to_text(Lang::Pt);
        assert!(text
            .contains("PHOENIX-PARTS-014: A peça não tem correspondência na base de peças [R9]"));
        assert!(text.starts_with("parts base: 0 matched, 1 without a match"));
    }
}
