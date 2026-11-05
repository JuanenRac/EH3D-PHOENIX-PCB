// SPDX-License-Identifier: GPL-3.0-or-later
//! The prices of the parts, read from a local file (nothing is ever fetched from a network).
//!
//! A CSV file has a header line with the columns `value`, `footprint`, `manufacturer`, `mpn`,
//! `supplier`, `supplier_code`, `min_qty`, `unit_price` and `currency`; one row is one quantity
//! break of one part at one supplier. A JSON file is a list of objects with the same names, or an
//! object with the list under `prices`. A row with an `mpn` is for the part with that number; a
//! row without one is for the parts with that `value` and `footprint` (an empty `footprint`
//! matches any). `min_qty` is 1 when it is missing and `currency` is the one of the run.

use serde_json::Value;

use crate::codes::{BomError, PRICES_INVALID};
use crate::money::parse_micro;
use crate::text::parse_table;

/// One quantity break of one part at one supplier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    /// The value the row is for when it has no part number.
    pub value: String,
    /// The footprint the row is for; empty for any.
    pub footprint: String,
    /// The maker.
    pub manufacturer: String,
    /// The part number of the maker; empty when the row is for a value and a footprint.
    pub mpn: String,
    /// The supplier.
    pub supplier: String,
    /// The code of the part at the supplier.
    pub supplier_code: String,
    /// The quantity from which the price holds.
    pub min_qty: u64,
    /// The price of one part, in millionths of the currency.
    pub unit_price_micro: i64,
    /// The currency; empty for the one of the run.
    pub currency: String,
    /// The line of the file (or the place in a list) the row came from.
    pub origin: usize,
}

/// The rows of a price file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PriceBook {
    /// The rows in the order of the file.
    pub rows: Vec<PriceRow>,
}

fn invalid(what: String) -> BomError {
    BomError::new(PRICES_INVALID, what)
}

impl PriceBook {
    /// Reads a price file, CSV or JSON (JSON when the first character is `[` or `{`), with no
    /// more than `max_rows` rows.
    pub fn parse(text: &str, max_rows: usize) -> Result<PriceBook, BomError> {
        let first = text
            .trim_start_matches('\u{feff}')
            .trim_start()
            .chars()
            .next();
        if matches!(first, Some('[' | '{')) {
            PriceBook::from_json(text, max_rows)
        } else {
            PriceBook::from_csv(text, max_rows)
        }
    }

    fn from_csv(text: &str, max_rows: usize) -> Result<PriceBook, BomError> {
        let table = parse_table(text).map_err(invalid)?;
        let price = table
            .column("unit_price")
            .ok_or_else(|| invalid("the column unit_price is missing".to_owned()))?;
        if table.column("mpn").is_none() && table.column("value").is_none() {
            return Err(invalid("a column mpn or value is needed".to_owned()));
        }
        if table.column("supplier").is_none() {
            return Err(invalid("the column supplier is missing".to_owned()));
        }
        if table.rows.len() > max_rows {
            return Err(invalid(format!("more than {max_rows} rows")));
        }
        let mut rows = Vec::new();
        for (line, cells) in &table.rows {
            let get = |name: &str| -> String {
                table
                    .column(name)
                    .and_then(|i| cells.get(i))
                    .cloned()
                    .unwrap_or_default()
            };
            let unit = cells.get(price).cloned().unwrap_or_default();
            rows.push(make_row(
                *line,
                [
                    get("value"),
                    get("footprint"),
                    get("manufacturer"),
                    get("mpn"),
                    get("supplier"),
                    get("supplier_code"),
                    get("currency"),
                ],
                &get("min_qty"),
                &unit,
            )?);
        }
        Ok(PriceBook { rows })
    }

    fn from_json(text: &str, max_rows: usize) -> Result<PriceBook, BomError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| invalid(format!("not valid JSON: {e}")))?;
        let list = match &value {
            Value::Array(list) => list,
            Value::Object(map) => match map.get("prices") {
                Some(Value::Array(list)) => list,
                _ => return Err(invalid("an object needs a list under prices".to_owned())),
            },
            _ => return Err(invalid("a list of prices is expected".to_owned())),
        };
        if list.len() > max_rows {
            return Err(invalid(format!("more than {max_rows} rows")));
        }
        let mut rows = Vec::new();
        for (index, item) in list.iter().enumerate() {
            let Value::Object(map) = item else {
                return Err(invalid(format!("item {}: not an object", index + 1)));
            };
            let get = |name: &str| -> String {
                match map.get(name) {
                    Some(Value::String(s)) => s.trim().to_owned(),
                    Some(Value::Number(n)) => n.to_string(),
                    _ => String::new(),
                }
            };
            rows.push(make_row(
                index + 1,
                [
                    get("value"),
                    get("footprint"),
                    get("manufacturer"),
                    get("mpn"),
                    get("supplier"),
                    get("supplier_code"),
                    get("currency"),
                ],
                &get("min_qty"),
                &get("unit_price"),
            )?);
        }
        Ok(PriceBook { rows })
    }
}

/// One row from its text cells: value, footprint, manufacturer, mpn, supplier, supplier code and
/// currency, then the minimum quantity and the unit price.
fn make_row(
    origin: usize,
    cells: [String; 7],
    min_qty: &str,
    unit_price: &str,
) -> Result<PriceRow, BomError> {
    let [value, footprint, manufacturer, mpn, supplier, supplier_code, currency] = cells;
    if supplier.is_empty() {
        return Err(invalid(format!("row {origin}: the supplier is empty")));
    }
    if mpn.is_empty() && value.is_empty() {
        return Err(invalid(format!("row {origin}: neither mpn nor value")));
    }
    let min_qty: u64 = if min_qty.is_empty() {
        1
    } else {
        min_qty.parse().ok().filter(|q| *q >= 1).ok_or_else(|| {
            invalid(format!(
                "row {origin}: min_qty is not a whole number from 1"
            ))
        })?
    };
    let unit_price_micro = parse_micro(unit_price)
        .ok_or_else(|| invalid(format!("row {origin}: unit_price is not a decimal number")))?;
    Ok(PriceRow {
        value,
        footprint,
        manufacturer,
        mpn,
        supplier,
        supplier_code,
        min_qty,
        unit_price_micro,
        currency,
        origin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str =
        "value,footprint,manufacturer,mpn,supplier,supplier_code,min_qty,unit_price,currency\n\
10k,R0603,,,Shop A,A-10K,1,0.0100,EUR\n\
10k,R0603,,,Shop A,A-10K,100,0.0040,\n\
,,Acme,AC-1,Shop B,B-1,,0.5,usd\n";

    #[test]
    fn a_csv_file_is_read_row_by_row() {
        let book = PriceBook::parse(CSV, 100).unwrap();
        assert_eq!(book.rows.len(), 3);
        assert_eq!(book.rows[0].supplier, "Shop A");
        assert_eq!(book.rows[0].unit_price_micro, 10_000);
        assert_eq!(book.rows[1].min_qty, 100);
        assert_eq!(book.rows[1].currency, "");
        assert_eq!(book.rows[2].mpn, "AC-1");
        assert_eq!(book.rows[2].min_qty, 1);
        assert_eq!(book.rows[2].currency, "usd");
        assert_eq!(book.rows[2].origin, 4);
    }

    #[test]
    fn a_json_file_in_either_shape_gives_the_same_rows() {
        let list = r#"[{"value":"10k","footprint":"R0603","supplier":"Shop A","supplier_code":"A-10K","min_qty":100,"unit_price":"0.004"}]"#;
        let object = format!("{{\"prices\":{list}}}");
        let a = PriceBook::parse(list, 10).unwrap();
        let b = PriceBook::parse(&object, 10).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.rows[0].min_qty, 100);
        assert_eq!(a.rows[0].unit_price_micro, 4_000);
    }

    #[test]
    fn a_json_price_may_be_a_number() {
        let text = r#"[{"mpn":"X","supplier":"S","unit_price":0.25}]"#;
        let book = PriceBook::parse(text, 10).unwrap();
        assert_eq!(book.rows[0].unit_price_micro, 250_000);
    }

    #[test]
    fn what_is_wrong_is_refused() {
        for (text, needle) in [
            ("supplier,mpn\nS,X\n", "unit_price is missing"),
            ("mpn,unit_price\nX,1\n", "supplier is missing"),
            ("supplier,unit_price\nS,1\n", "mpn or value"),
            ("mpn,supplier,unit_price\nX,S,abc\n", "row 2: unit_price"),
            ("mpn,supplier,unit_price\nX,,1\n", "supplier is empty"),
            ("mpn,supplier,min_qty,unit_price\nX,S,0,1\n", "min_qty"),
            ("[1]", "not an object"),
            ("{\"a\":1}", "under prices"),
            ("[{", "not valid JSON"),
        ] {
            let e = PriceBook::parse(text, 10).unwrap_err();
            assert_eq!(e.code, PRICES_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = PriceBook::parse(CSV, 2).unwrap_err();
        assert!(e.detail.contains("more than 2"));
    }
}
