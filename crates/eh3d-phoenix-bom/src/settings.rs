// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the costs. Every number, rate and limit is read from a plain `key = value`
//! text, `config/costs.cfg`, and not buried in the code. A missing line keeps the default; an
//! unknown key or a value out of range is refused with its line number. Money and percentages are
//! written as decimals with at most six decimals. The tariffs that come with the program are an
//! example of the shape of a tariff and are not real prices of any maker.

use std::collections::BTreeMap;

use crate::codes::{BomError, SETTINGS_INVALID};
use crate::money::{parse_micro, MICRO};

/// The most layers a tariff may be given for.
const MAX_LAYERS: u32 = 64;

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
