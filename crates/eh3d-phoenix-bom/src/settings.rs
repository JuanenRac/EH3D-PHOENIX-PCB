// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the costs. Every number, rate and limit is read from a plain `key = value`
//! text, `config/costs.cfg`, and not buried in the code. A missing line keeps the default; an
//! unknown key or a value out of range is refused with its line number. Money and percentages are
//! written as decimals with at most six decimals. The tariffs that come with the program are an
//! example of the shape of a tariff and are not real prices of any maker.

use std::collections::BTreeMap;

use crate::codes::{BomError, SETTINGS_INVALID};
use crate::money::{parse_micro, MICRO};
use crate::report::COLUMNS;

/// The most layers a tariff may be given for.
const MAX_LAYERS: u32 = 64;

/// What makes two parts one line of the bill, besides the value and the footprint, which always
/// do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupBy {
    /// Parts of different makers are different lines.
    pub manufacturer: bool,
    /// Parts of different part numbers are different lines.
    pub mpn: bool,
}

impl Default for GroupBy {
    fn default() -> Self {
        GroupBy {
            manufacturer: true,
            mpn: true,
        }
    }
}

/// The numbers of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostSettings {
    /// The currency of every price and cost; prices in another currency are ignored.
    pub currency: String,
    /// The number of boards of the batch.
    pub boards: u64,
    /// The extra parts bought for waste, in millionths of a per cent.
    pub parts_waste_percent_micro: i64,
    /// The fixed cost of an order of boards, in millionths of the currency.
    pub pcb_setup_cost_micro: i64,
    /// The price of a square centimetre of board, by the number of copper layers, in millionths.
    pub pcb_area_rate_micro: BTreeMap<u32, i64>,
    /// The part of the panel that holds boards, in millionths of a per cent.
    pub pcb_utilisation_percent_micro: i64,
    /// The thickness above which a board costs more, in nanometres.
    pub pcb_thickness_reference_nm: i64,
    /// The extra cost of a board thicker than the reference, in millionths of a per cent.
    pub pcb_thickness_surcharge_percent_micro: i64,
    /// The cost of a drilled hole (vias and through-hole pads), in millionths of the currency.
    pub pcb_drill_cost_micro: i64,
    /// The least an order of boards costs, in millionths of the currency.
    pub pcb_min_order_micro: i64,
    /// What makes two parts one line of the bill.
    pub group_by: GroupBy,
    /// The columns of the CSV file, by name, in the order they are written.
    pub csv_columns: Vec<String>,
    /// The most components, lines and price rows a run handles.
    pub max_items: usize,
    /// The largest input file, in bytes.
    pub max_file_bytes: u64,
}

impl Default for CostSettings {
    /// The example tariffs: a shape to copy and change, not prices of any maker.
    fn default() -> Self {
        let mut rates = BTreeMap::new();
        rates.insert(1, 40_000);
        rates.insert(2, 60_000);
        rates.insert(4, 150_000);
        rates.insert(6, 260_000);
        CostSettings {
            currency: "EUR".to_owned(),
            boards: 10,
            parts_waste_percent_micro: 5 * MICRO,
            pcb_setup_cost_micro: 20 * MICRO,
            pcb_area_rate_micro: rates,
            pcb_utilisation_percent_micro: 80 * MICRO,
            pcb_thickness_reference_nm: 1_600_000,
            pcb_thickness_surcharge_percent_micro: 20 * MICRO,
            pcb_drill_cost_micro: 1_000,
            pcb_min_order_micro: 5 * MICRO,
            group_by: GroupBy::default(),
            csv_columns: COLUMNS.iter().map(|c| (*c).to_owned()).collect(),
            max_items: 100_000,
            max_file_bytes: 16 * 1024 * 1024,
        }
    }
}

impl CostSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<CostSettings, BomError> {
        let mut s = CostSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                BomError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                BomError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            let money = || parse_micro(value).ok_or_else(bad);
            if let Some(layers) = key.strip_prefix("pcb_area_rate.") {
                let layers: u32 = layers.parse().map_err(|_| bad())?;
                if layers == 0 || layers > MAX_LAYERS {
                    return Err(bad());
                }
                s.pcb_area_rate_micro.insert(layers, money()?);
                continue;
            }
            match key {
                "currency" => s.currency = value.to_owned(),
                "boards" => s.boards = value.parse().map_err(|_| bad())?,
                "parts_waste_percent" => s.parts_waste_percent_micro = money()?,
                "pcb_setup_cost" => s.pcb_setup_cost_micro = money()?,
                "pcb_utilisation_percent" => s.pcb_utilisation_percent_micro = money()?,
                "pcb_thickness_reference_nm" => {
                    s.pcb_thickness_reference_nm = value.parse().map_err(|_| bad())?
                }
                "pcb_thickness_surcharge_percent" => {
                    s.pcb_thickness_surcharge_percent_micro = money()?
                }
                "pcb_drill_cost" => s.pcb_drill_cost_micro = money()?,
                "pcb_min_order_cost" => s.pcb_min_order_micro = money()?,
                "group_by" => s.group_by = parse_group_by(value).ok_or_else(bad)?,
                "csv_columns" => s.csv_columns = parse_columns(value).ok_or_else(bad)?,
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(BomError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the calculations rely on.
    pub fn validate(&self) -> Result<(), BomError> {
        let fail = |what: &str| Err(BomError::new(SETTINGS_INVALID, what.to_string()));
        let currency_ok = !self.currency.is_empty()
            && self.currency.len() <= 8
            && self.currency.chars().all(|c| c.is_ascii_alphanumeric());
        if !currency_ok {
            return fail("currency must be 1 to 8 letters or digits");
        }
        if self.boards == 0 || self.boards > 1_000_000 {
            return fail("boards must be from 1 to 1000000");
        }
        if self.parts_waste_percent_micro > 1000 * MICRO {
            return fail("parts_waste_percent must be at most 1000");
        }
        if self.pcb_utilisation_percent_micro <= 0
            || self.pcb_utilisation_percent_micro > 100 * MICRO
        {
            return fail("pcb_utilisation_percent must be above 0 and at most 100");
        }
        if self.pcb_thickness_surcharge_percent_micro > 1000 * MICRO {
            return fail("pcb_thickness_surcharge_percent must be at most 1000");
        }
        if self.pcb_thickness_reference_nm <= 0 {
            return fail("pcb_thickness_reference_nm must be above 0");
        }
        if self.max_items == 0 || self.max_file_bytes == 0 {
            return fail("max_items and max_file_bytes must be above 0");
        }
        Ok(())
    }
}

/// Reads `value,footprint[,manufacturer][,mpn]`: the value and the footprint are always part of
/// it, a name twice or a name that is not one of the four is refused.
fn parse_group_by(text: &str) -> Option<GroupBy> {
    let mut seen = std::collections::BTreeSet::new();
    for name in text.split(',').map(str::trim) {
        if !["value", "footprint", "manufacturer", "mpn"].contains(&name) || !seen.insert(name) {
            return None;
        }
    }
    if !seen.contains("value") || !seen.contains("footprint") {
        return None;
    }
    Some(GroupBy {
        manufacturer: seen.contains("manufacturer"),
        mpn: seen.contains("mpn"),
    })
}

/// Reads a list of column names of the CSV file: at least one, each one of [`COLUMNS`], none twice.
fn parse_columns(text: &str) -> Option<Vec<String>> {
    let names: Vec<String> = text.split(',').map(|n| n.trim().to_owned()).collect();
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        if !COLUMNS.contains(&name.as_str()) || !seen.insert(name.clone()) {
            return None;
        }
    }
    Some(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_are_valid_and_the_shipped_file_parses_to_them() {
        let d = CostSettings::default();
        assert!(d.validate().is_ok());
        let shipped = include_str!("../../../config/costs.cfg");
        assert_eq!(CostSettings::parse(shipped).unwrap(), d);
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let s = CostSettings::parse(
            "# note\nboards = 25\ncurrency = USD\npcb_area_rate.8 = 0.5\nparts_waste_percent = 2.5\n",
        )
        .unwrap();
        assert_eq!(s.boards, 25);
        assert_eq!(s.currency, "USD");
        assert_eq!(s.pcb_area_rate_micro[&8], 500_000);
        assert_eq!(s.parts_waste_percent_micro, 2_500_000);
        assert_eq!(
            s.pcb_setup_cost_micro,
            CostSettings::default().pcb_setup_cost_micro
        );
    }

    #[test]
    fn the_grouping_and_the_columns_are_read_in_the_order_given() {
        let s = CostSettings::parse(
            "group_by = value, footprint\ncsv_columns = references,quantity,value\n",
        )
        .unwrap();
        assert!(!s.group_by.manufacturer && !s.group_by.mpn);
        assert_eq!(s.csv_columns, ["references", "quantity", "value"]);
        let s = CostSettings::parse("group_by = footprint,value,mpn\n").unwrap();
        assert!(!s.group_by.manufacturer && s.group_by.mpn);
    }

    #[test]
    fn a_grouping_or_a_column_that_does_not_exist_is_refused() {
        for text in [
            "group_by = value\n",
            "group_by = value,footprint,value\n",
            "group_by = value,footprint,colour\n",
            "group_by =\n",
            "csv_columns = references,colour\n",
            "csv_columns = value,value\n",
            "csv_columns =\n",
        ] {
            let e = CostSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
        }
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("boards\n", "line 1: not key = value"),
            ("\nfoo = 1\n", "line 2: unknown key foo"),
            ("boards = -3\n", "line 1: the value of boards"),
            ("boards = 0\n", "boards must be"),
            ("pcb_setup_cost = -1\n", "pcb_setup_cost"),
            ("pcb_area_rate.0 = 1\n", "pcb_area_rate.0"),
            ("pcb_utilisation_percent = 0\n", "pcb_utilisation_percent"),
            ("currency = EU R\n", "currency must be"),
        ] {
            let e = CostSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }
}
