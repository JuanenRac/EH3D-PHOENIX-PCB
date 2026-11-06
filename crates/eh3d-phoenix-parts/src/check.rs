// SPDX-License-Identifier: GPL-3.0-or-later
//! The checker of the base. It finds what makes the base wrong or doubtful and changes nothing:
//! an id used twice, a footprint that is not in the IPC library or in the own libraries, two parts
//! with the same part number and different data, a price without a currency, price breaks out of
//! order, a simulation model that the table of models does not have and an alternative that is
//! not in the base. The footprints and the models are known only through the functions the caller
//! gives; a check whose function is missing is not made.

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    Finding, Severity, ALTERNATIVE_UNKNOWN, FOOTPRINT_UNKNOWN, ID_DUPLICATE, MPN_CONFLICT,
    PRICE_NO_CURRENCY, SIM_MODEL_UNKNOWN, TIERS_UNORDERED,
};
use crate::model::{norm_value, Part, PartsDb};

/// What the checker knows about the world outside the base.
#[derive(Default)]
pub struct CheckContext<'a> {
    /// Tells whether a footprint name is in the IPC library or in an own library; `None` skips
    /// the check of the footprints.
    pub footprint_known: Option<&'a dyn Fn(&str) -> bool>,
    /// Tells whether a simulation model is in the table of models; `None` skips the check of the
    /// models.
    pub sim_model_known: Option<&'a dyn Fn(&str) -> bool>,
}

fn differences(a: &Part, b: &Part) -> Vec<&'static str> {
    let mut list = Vec::new();
    let mut differ = |name: &'static str, x: &str, y: &str| {
        if !x.is_empty() && !y.is_empty() && x != y {
            list.push(name);
        }
    };
    differ("value", &norm_value(&a.value), &norm_value(&b.value));
    differ(
        "footprint",
        &a.footprint.to_lowercase(),
        &b.footprint.to_lowercase(),
    );
    differ(
        "manufacturer",
        &a.manufacturer.to_lowercase(),
        &b.manufacturer.to_lowercase(),
    );
    differ("tolerance", &a.tolerance, &b.tolerance);
    differ("power", &a.power, &b.power);
    differ("voltage", &a.voltage, &b.voltage);
    list
}

/// Checks the base. The findings are sorted, so the same base gives the same list.
pub fn check_db(db: &PartsDb, ctx: &CheckContext) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut first: BTreeMap<&str, &Part> = BTreeMap::new();
    let mut by_mpn: BTreeMap<String, Vec<&Part>> = BTreeMap::new();
    for p in &db.parts {
        if let Some(earlier) = first.get(p.id.as_str()) {
            out.push(Finding::new(
                ID_DUPLICATE,
                Severity::Error,
                &p.id,
                &p.origin,
                format!("the id is also at {}", earlier.origin),
            ));
        } else {
            first.insert(p.id.as_str(), p);
        }
        if !p.mpn.is_empty() {
            by_mpn.entry(p.mpn.to_lowercase()).or_default().push(p);
        }
    }
    for group in by_mpn.values() {
        let Some((head, others)) = group.split_first() else {
            continue;
        };
        for other in others {
            let diffs = differences(head, other);
            if !diffs.is_empty() {
                out.push(Finding::new(
                    MPN_CONFLICT,
                    Severity::Error,
                    &other.id,
                    &other.origin,
                    format!(
                        "part number {} is also in {} ({}) with another {}",
                        other.mpn,
                        head.id,
                        head.origin,
                        diffs.join(", ")
                    ),
                ));
            }
        }
    }
    let mpns: BTreeSet<String> = by_mpn.keys().cloned().collect();
    for p in &db.parts {
        if let Some(known) = ctx.footprint_known {
            if !p.footprint.is_empty() && !known(&p.footprint) {
                out.push(Finding::new(
                    FOOTPRINT_UNKNOWN,
                    Severity::Warning,
                    &p.id,
                    &p.origin,
                    p.footprint.clone(),
                ));
            }
        }
        if let Some(known) = ctx.sim_model_known {
            if !p.sim_model.is_empty() && !known(&p.sim_model) {
                out.push(Finding::new(
                    SIM_MODEL_UNKNOWN,
                    Severity::Warning,
                    &p.id,
                    &p.origin,
                    p.sim_model.clone(),
                ));
            }
        }
        for alternative in &p.alternatives {
            if !first.contains_key(alternative.as_str())
                && !mpns.contains(&alternative.to_lowercase())
            {
                out.push(Finding::new(
                    ALTERNATIVE_UNKNOWN,
                    Severity::Warning,
                    &p.id,
                    &p.origin,
                    alternative.clone(),
                ));
            }
        }
        for offer in &p.offers {
            for tier in offer.tiers.iter().filter(|t| t.currency.is_empty()) {
                out.push(Finding::new(
                    PRICE_NO_CURRENCY,
                    Severity::Warning,
                    &p.id,
                    &p.origin,
                    format!("{}, from {}", offer.supplier, tier.min_qty),
                ));
            }
            if let Some(pair) = offer
                .tiers
                .windows(2)
                .find(|w| w[1].min_qty <= w[0].min_qty)
            {
                out.push(Finding::new(
                    TIERS_UNORDERED,
                    Severity::Warning,
                    &p.id,
                    &p.origin,
                    format!(
                        "{}: {} comes after {}",
                        offer.supplier, pair[1].min_qty, pair[0].min_qty
                    ),
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    out
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
            origin: format!("base.json: part {id}"),
            ..Part::default()
        }
    }

    fn tier(min_qty: u64, currency: &str) -> Tier {
        Tier {
            min_qty,
            unit_price_micro: 1_000,
            currency: currency.into(),
        }
    }

    fn codes(findings: &[Finding]) -> Vec<(&'static str, &str)> {
        findings
            .iter()
            .map(|f| (f.code, f.subject.as_str()))
            .collect()
    }

    #[test]
    fn a_good_base_has_no_findings_and_no_context_skips_the_world_checks() {
        let mut a = part("a", "10k", "NOT-A-FOOTPRINT", "X-1");
        a.sim_model = "unknown-model".into();
        a.offers = vec![Offer {
            supplier: "S".into(),
            supplier_code: String::new(),
            tiers: vec![tier(1, "EUR"), tier(10, "EUR")],
        }];
        let db = PartsDb { parts: vec![a] };
        assert!(check_db(&db, &CheckContext::default()).is_empty());
        assert!(check_db(&PartsDb::default(), &CheckContext::default()).is_empty());
    }

    #[test]
    fn an_id_used_twice_is_an_error_that_names_the_first_place() {
        let db = PartsDb {
            parts: vec![part("a", "1k", "", ""), part("a", "2k", "", "")],
        };
        let findings = check_db(&db, &CheckContext::default());
        assert_eq!(codes(&findings), vec![(ID_DUPLICATE, "a")]);
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].detail.contains("base.json: part a"));
    }

    #[test]
    fn footprints_and_models_are_known_only_through_the_functions_given() {
        let mut a = part("a", "10k", "R0603", "");
        a.sim_model = "res".into();
        let mut b = part("b", "10k", "R0603X", "");
        b.sim_model = "ghost".into();
        let db = PartsDb {
            parts: vec![a, b, part("c", "1k", "", "")],
        };
        let footprints = |name: &str| name == "R0603";
        let models = |name: &str| name == "res";
        let findings = check_db(
            &db,
            &CheckContext {
                footprint_known: Some(&footprints),
                sim_model_known: Some(&models),
            },
        );
        assert_eq!(
            codes(&findings),
            vec![(FOOTPRINT_UNKNOWN, "b"), (SIM_MODEL_UNKNOWN, "b")]
        );
        assert_eq!(findings[0].detail, "R0603X");
        assert_eq!(findings[1].detail, "ghost");
    }

    #[test]
    fn the_same_part_number_with_other_data_is_an_error_and_with_the_same_data_is_not() {
        let mut a = part("a", "10k", "R0603", "AC-1");
        a.manufacturer = "Acme".into();
        let mut b = part("b", "10 k\u{3a9}", "r0603", "ac-1");
        b.manufacturer = "ACME".into();
        let c = part("c", "22k", "R0603", "AC-1");
        let mut d = part("d", "", "", "AC-1");
        d.tolerance = "1%".into();
        let db = PartsDb {
            parts: vec![a, b, c, d],
        };
        let findings = check_db(&db, &CheckContext::default());
        assert_eq!(codes(&findings), vec![(MPN_CONFLICT, "c")]);
        assert!(findings[0].detail.contains("another value"));
        assert!(findings[0].detail.contains("also in a"));
    }

    #[test]
    fn prices_without_a_currency_and_breaks_out_of_order_are_warnings() {
        let mut a = part("a", "10k", "", "");
        a.offers = vec![
            Offer {
                supplier: "S1".into(),
                supplier_code: String::new(),
                tiers: vec![tier(1, ""), tier(10, "EUR")],
            },
            Offer {
                supplier: "S2".into(),
                supplier_code: String::new(),
                tiers: vec![tier(100, "EUR"), tier(10, "EUR")],
            },
            Offer {
                supplier: "S3".into(),
                supplier_code: String::new(),
                tiers: vec![tier(5, "EUR"), tier(5, "EUR")],
            },
        ];
        let findings = check_db(&PartsDb { parts: vec![a] }, &CheckContext::default());
        assert_eq!(
            codes(&findings),
            vec![
                (PRICE_NO_CURRENCY, "a"),
                (TIERS_UNORDERED, "a"),
                (TIERS_UNORDERED, "a")
            ]
        );
        assert_eq!(findings[0].detail, "S1, from 1");
        assert_eq!(findings[1].detail, "S2: 10 comes after 100");
        assert_eq!(findings[2].detail, "S3: 5 comes after 5");
        assert!(findings.iter().all(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn an_alternative_may_name_an_id_or_a_part_number_and_must_exist() {
        let mut a = part("a", "10k", "", "AC-1");
        a.alternatives = vec!["b".into(), "bb-2".into(), "ghost".into()];
        let db = PartsDb {
            parts: vec![a, part("b", "10k", "", ""), part("c", "10k", "", "BB-2")],
        };
        let findings = check_db(&db, &CheckContext::default());
        assert_eq!(codes(&findings), vec![(ALTERNATIVE_UNKNOWN, "a")]);
        assert_eq!(findings[0].detail, "ghost");
    }
}
