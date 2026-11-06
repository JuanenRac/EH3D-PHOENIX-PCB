// SPDX-License-Identifier: GPL-3.0-or-later
//! The parts of the base as plain data: what a part is, its suppliers with the price breaks, its
//! state, and the whole base.

use std::collections::BTreeMap;

/// The format tag of the JSON files of the base.
pub const FORMAT: &str = "phoenix-parts/1";

/// The state of a part in the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Status {
    /// The part is made and recommended.
    #[default]
    Active,
    /// The part is made and is not recommended for new designs.
    NotRecommended,
    /// The part is not made any more.
    Obsolete,
}

impl Status {
    /// The word written in the files.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::NotRecommended => "not-recommended",
            Status::Obsolete => "obsolete",
        }
    }

    /// Reads the word of a file (any case; `nrnd` and `not recommended` are accepted).
    pub fn parse(text: &str) -> Option<Status> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "active" => Some(Status::Active),
            "not-recommended" | "not recommended" | "nrnd" => Some(Status::NotRecommended),
            "obsolete" | "eol" => Some(Status::Obsolete),
            _ => None,
        }
    }
}

/// One price break: from `min_qty` parts the price of one part is `unit_price_micro`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier {
    /// The quantity from which the price holds.
    pub min_qty: u64,
    /// The price of one part, in millionths of the currency.
    pub unit_price_micro: i64,
    /// The currency; empty when the file does not say (a finding of the check).
    pub currency: String,
}

/// A supplier of a part with its price breaks, in the order of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The supplier.
    pub supplier: String,
    /// The code of the part at the supplier.
    pub supplier_code: String,
    /// The price breaks.
    pub tiers: Vec<Tier>,
}

/// A part of the base.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Part {
    /// The id, unique in the base; it is what an alternative or a command names.
    pub id: String,
    /// The value (`10k`, `100n`, `NE555`).
    pub value: String,
    /// The tolerance (`1%`).
    pub tolerance: String,
    /// The power it takes (`0.1W`).
    pub power: String,
    /// The voltage it stands (`50V`).
    pub voltage: String,
    /// The footprint: a name of the IPC library or an own name.
    pub footprint: String,
    /// The symbol.
    pub symbol: String,
    /// The maker.
    pub manufacturer: String,
    /// The part number of the maker.
    pub mpn: String,
    /// The state.
    pub status: Status,
    /// The free parameters, by name.
    pub params: BTreeMap<String, String>,
    /// The simulation model: a name of `config/sim_models.cfg` (a `.model` line).
    pub sim_model: String,
    /// The suppliers.
    pub offers: Vec<Offer>,
    /// The notes.
    pub notes: String,
    /// The alternatives: ids (or part numbers) of equivalent parts.
    pub alternatives: Vec<String>,
    /// Where the part was read from (`file: line N` or `file: part N`); never written.
    pub origin: String,
}

/// The whole base: the parts in the order they were read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartsDb {
    /// The parts.
    pub parts: Vec<Part>,
}

/// A value in a form that can be compared: lower case, without blanks, `µ` and `μ` as `u`, and
/// without the ohm sign (`10 kΩ` is `10k`).
pub fn norm_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\u{2126}' | '\u{3a9}' | '\u{3c9}'))
        .flat_map(char::to_lowercase)
        .map(|c| {
            if matches!(c, '\u{b5}' | '\u{3bc}') {
                'u'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_of_the_state_are_read_in_any_case() {
        assert_eq!(Status::parse(""), Some(Status::Active));
        assert_eq!(Status::parse(" Active "), Some(Status::Active));
        assert_eq!(Status::parse("NRND"), Some(Status::NotRecommended));
        assert_eq!(
            Status::parse("Not Recommended"),
            Some(Status::NotRecommended)
        );
        assert_eq!(Status::parse("obsolete"), Some(Status::Obsolete));
        assert_eq!(Status::parse("EOL"), Some(Status::Obsolete));
        assert_eq!(Status::parse("maybe"), None);
        for s in [Status::Active, Status::NotRecommended, Status::Obsolete] {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert!(Status::Active < Status::Obsolete);
    }

    #[test]
    fn values_are_compared_without_case_blanks_or_the_ohm_sign() {
        assert_eq!(norm_value("10 k\u{3a9}"), "10k");
        assert_eq!(norm_value("10 k\u{2126}"), "10k");
        assert_eq!(norm_value("10K"), norm_value("10k"));
        assert_eq!(norm_value("4\u{b5}7"), norm_value("4u7"));
        assert_eq!(norm_value("4\u{3bc}7"), "4u7");
        assert_eq!(norm_value("NE555"), "ne555");
        assert_ne!(norm_value("10k"), norm_value("100k"));
    }
}
