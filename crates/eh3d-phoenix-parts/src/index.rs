// SPDX-License-Identifier: GPL-3.0-or-later
//! The index of the base: lookups by id, by part number and by value, and the search by text with
//! filters. The order is always the order of the ids (and, for the same id, the order of the
//! files), so a search gives the same list every time.

use std::collections::BTreeMap;

use crate::model::{norm_value, Part, PartsDb, Status};

/// What a search asks for. Every filter that is not empty must hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    /// Words that must all appear (in any case) in the id, value, footprint, symbol, maker, part
    /// number, tolerance, power, voltage, notes, parameters or simulation model.
    pub text: String,
    /// The value, compared without case, blanks or the ohm sign (`10k` finds `10 kΩ`).
    pub value: String,
    /// The footprint, compared without case.
    pub footprint: String,
    /// A piece of the part number, without case.
    pub mpn: String,
    /// A piece of the name of the maker, without case.
    pub manufacturer: String,
    /// The state.
    pub status: Option<Status>,
    /// The most parts returned; zero means all of them.
    pub limit: usize,
}

/// The result of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found<'a> {
    /// The parts, in the order of the ids, no more than the limit.
    pub parts: Vec<&'a Part>,
    /// How many parts matched, whatever the limit.
    pub total: usize,
}

/// The index of a base.
#[derive(Debug)]
pub struct Index<'a> {
    parts: &'a [Part],
    order: Vec<usize>,
    by_id: BTreeMap<&'a str, usize>,
    by_mpn: BTreeMap<String, Vec<usize>>,
    by_value: BTreeMap<String, Vec<usize>>,
    haystack: Vec<String>,
}

fn haystack_of(p: &Part) -> String {
    let mut text = String::new();
    for field in [
        &p.id,
        &p.value,
        &p.tolerance,
        &p.power,
        &p.voltage,
        &p.footprint,
        &p.symbol,
        &p.manufacturer,
        &p.mpn,
        &p.sim_model,
        &p.notes,
    ] {
        text.push_str(field);
        text.push('\n');
    }
    for (key, value) in &p.params {
        text.push_str(key);
        text.push(' ');
        text.push_str(value);
        text.push('\n');
    }
    text.to_lowercase()
}

impl<'a> Index<'a> {
    /// Builds the index of a base.
    pub fn build(db: &'a PartsDb) -> Index<'a> {
        let parts = db.parts.as_slice();
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by(|a, b| parts[*a].id.cmp(&parts[*b].id).then(a.cmp(b)));
        let mut by_id = BTreeMap::new();
        let mut by_mpn: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut by_value: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for &i in &order {
            let p = &parts[i];
            by_id.entry(p.id.as_str()).or_insert(i);
            if !p.mpn.is_empty() {
                by_mpn.entry(p.mpn.to_lowercase()).or_default().push(i);
            }
            if !p.value.is_empty() {
                by_value.entry(norm_value(&p.value)).or_default().push(i);
            }
        }
        Index {
            parts,
            order,
            by_id,
            by_mpn,
            by_value,
            haystack: parts.iter().map(haystack_of).collect(),
        }
    }

    /// The part with an id (the first one when the id is used twice).
    pub fn get(&self, id: &str) -> Option<&'a Part> {
        self.by_id.get(id).map(|i| &self.parts[*i])
    }

    /// The parts with a part number, without case, in the order of the ids.
    pub fn with_mpn(&self, mpn: &str) -> Vec<&'a Part> {
        self.by_mpn
            .get(&mpn.trim().to_lowercase())
            .map(|list| list.iter().map(|i| &self.parts[*i]).collect())
            .unwrap_or_default()
    }

    /// The parts with a value, compared as [`norm_value`] does, in the order of the ids.
    pub fn with_value(&self, value: &str) -> Vec<&'a Part> {
        self.by_value
            .get(&norm_value(value))
            .map(|list| list.iter().map(|i| &self.parts[*i]).collect())
            .unwrap_or_default()
    }

    /// The number of parts.
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// True when the base has no parts.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Searches the base.
    pub fn search(&self, query: &Query) -> Found<'a> {
        let words: Vec<String> = query
            .text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let value = norm_value(&query.value);
        let mpn = query.mpn.trim().to_lowercase();
        let maker = query.manufacturer.trim().to_lowercase();
        let limit = if query.limit == 0 {
            usize::MAX
        } else {
            query.limit
        };
        let mut found = Found {
            parts: Vec::new(),
            total: 0,
        };
        for &i in &self.order {
            let p = &self.parts[i];
            let keep = words.iter().all(|w| self.haystack[i].contains(w.as_str()))
                && (value.is_empty() || norm_value(&p.value) == value)
                && (query.footprint.is_empty()
                    || p.footprint.eq_ignore_ascii_case(query.footprint.trim()))
                && (mpn.is_empty() || p.mpn.to_lowercase().contains(&mpn))
                && (maker.is_empty() || p.manufacturer.to_lowercase().contains(&maker))
                && query.status.is_none_or(|s| s == p.status);
            if keep {
                found.total += 1;
                if found.parts.len() < limit {
                    found.parts.push(p);
                }
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(id: &str, value: &str, footprint: &str, mpn: &str, maker: &str) -> Part {
        Part {
            id: id.into(),
            value: value.into(),
            footprint: footprint.into(),
            mpn: mpn.into(),
            manufacturer: maker.into(),
            ..Part::default()
        }
    }

    fn db() -> PartsDb {
        let mut old = part("r-22k", "22k", "R0603", "AC-22K", "Acme");
        old.status = Status::Obsolete;
        old.notes = "Replaced by the 1% part".into();
        let mut c = part("c-100n", "100n", "C0603", "Z-100N", "Zeta Parts");
        c.params.insert("dielectric".into(), "X7R".into());
        PartsDb {
            parts: vec![
                part("r-10k-b", "10 k\u{3a9}", "R0805", "AC-10KB", "Acme"),
                c,
                part("r-10k", "10k", "R0603", "AC-10K", "Acme"),
                old,
            ],
        }
    }

    #[test]
    fn the_lookups_are_by_id_part_number_and_value() {
        let db = db();
        let index = Index::build(&db);
        assert_eq!(index.len(), 4);
        assert!(!index.is_empty());
        assert_eq!(index.get("c-100n").unwrap().mpn, "Z-100N");
        assert!(index.get("nope").is_none());
        let by_mpn: Vec<&str> = index
            .with_mpn(" ac-10k ")
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(by_mpn, vec!["r-10k"]);
        let by_value: Vec<&str> = index
            .with_value("10K")
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(by_value, vec!["r-10k", "r-10k-b"]);
        assert!(index.with_value("1M").is_empty());
        assert!(Index::build(&PartsDb::default()).is_empty());
    }

    #[test]
    fn a_duplicate_id_gives_the_first_part() {
        let db = PartsDb {
            parts: vec![
                part("x", "1k", "", "", "first"),
                part("x", "2k", "", "", "second"),
            ],
        };
        let index = Index::build(&db);
        assert_eq!(index.get("x").unwrap().manufacturer, "first");
        let all = index.search(&Query::default());
        assert_eq!(all.total, 2);
        assert_eq!(all.parts[0].manufacturer, "first");
    }

    #[test]
    fn a_search_is_in_the_order_of_the_ids_and_counts_what_the_limit_cuts() {
        let db = db();
        let index = Index::build(&db);
        let all = index.search(&Query::default());
        let ids: Vec<&str> = all.parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["c-100n", "r-10k", "r-10k-b", "r-22k"]);
        let two = index.search(&Query {
            limit: 2,
            ..Query::default()
        });
        assert_eq!((two.parts.len(), two.total), (2, 4));
        assert_eq!(two.parts[1].id, "r-10k");
    }

    #[test]
    fn the_text_must_appear_in_every_word_and_in_any_field() {
        let db = db();
        let index = Index::build(&db);
        let q = |text: &str| Query {
            text: text.into(),
            ..Query::default()
        };
        assert_eq!(index.search(&q("x7r")).total, 1);
        assert_eq!(index.search(&q("DIELECTRIC x7r")).total, 1);
        assert_eq!(index.search(&q("acme 0603")).total, 2);
        assert_eq!(index.search(&q("acme x7r")).total, 0);
        assert_eq!(index.search(&q("replaced")).parts[0].id, "r-22k");
        assert_eq!(index.search(&q("   ")).total, 4);
    }

    #[test]
    fn the_filters_work_alone_and_together() {
        let db = db();
        let index = Index::build(&db);
        let ids = |q: Query| -> Vec<String> {
            index
                .search(&q)
                .parts
                .iter()
                .map(|p| p.id.clone())
                .collect()
        };
        assert_eq!(
            ids(Query {
                value: "10k".into(),
                ..Query::default()
            }),
            vec!["r-10k", "r-10k-b"]
        );
        assert_eq!(
            ids(Query {
                value: "10k".into(),
                footprint: "r0805".into(),
                ..Query::default()
            }),
            vec!["r-10k-b"]
        );
        assert_eq!(
            ids(Query {
                mpn: "ac-".into(),
                status: Some(Status::Obsolete),
                ..Query::default()
            }),
            vec!["r-22k"]
        );
        assert_eq!(
            ids(Query {
                manufacturer: "zeta".into(),
                ..Query::default()
            }),
            vec!["c-100n"]
        );
        assert!(ids(Query {
            status: Some(Status::NotRecommended),
            ..Query::default()
        })
        .is_empty());
    }
}
