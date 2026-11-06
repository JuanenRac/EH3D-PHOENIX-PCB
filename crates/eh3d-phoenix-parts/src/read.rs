// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading the base from files of the user: JSON (`phoenix-parts/1`) and CSV. A file that is not
//! valid is refused as a whole with the line (CSV, JSON syntax) or the number of the part (JSON
//! structure); nothing is guessed. The limits of the settings are checked while reading.
//!
//! A CSV file has a header line with any of the columns `id`, `value`, `tolerance`, `power`,
//! `voltage`, `footprint`, `symbol`, `manufacturer`, `mpn`, `status`, `sim_model`, `notes`,
//! `alternatives` (ids or part numbers joined by `;`), `params` (`name=value` joined by `;`),
//! `supplier`, `supplier_code`, `min_qty`, `unit_price` and `currency`. One row is one part, or
//! one more supplier or price break of a part with the same `id` (the cells of the part must be
//! empty or equal on those rows). A row without `id` takes its `mpn` as the id.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_bom::money::parse_micro;
use eh3d_phoenix_bom::text::{parse_table, Table};
use serde_json::Value;

use crate::codes::{PartsError, FILE_UNUSABLE, FORMAT_INVALID, LIMIT_EXCEEDED};
use crate::model::{Offer, Part, PartsDb, Status, Tier, FORMAT};
use crate::settings::PartsSettings;

/// The columns of a CSV file.
pub const COLUMNS: [&str; 19] = [
    "id",
    "value",
    "tolerance",
    "power",
    "voltage",
    "footprint",
    "symbol",
    "manufacturer",
    "mpn",
    "status",
    "sim_model",
    "notes",
    "alternatives",
    "params",
    "supplier",
    "supplier_code",
    "min_qty",
    "unit_price",
    "currency",
];

const PART_KEYS: [&str; 15] = [
    "id",
    "value",
    "tolerance",
    "power",
    "voltage",
    "footprint",
    "symbol",
    "manufacturer",
    "mpn",
    "status",
    "params",
    "sim_model",
    "offers",
    "notes",
    "alternatives",
];

fn limit(what: String) -> PartsError {
    PartsError::new(LIMIT_EXCEEDED, what)
}

/// A price break from the text of its cells; the error is what is wrong.
fn make_tier(min_qty: &str, price: &str, currency: String) -> Result<Tier, String> {
    let min_qty: u64 = if min_qty.is_empty() {
        1
    } else {
        min_qty
            .parse()
            .ok()
            .filter(|q| *q >= 1)
            .ok_or_else(|| "min_qty is not a whole number from 1".to_owned())?
    };
    let unit_price_micro =
        parse_micro(price).ok_or_else(|| "unit_price is not a decimal number".to_owned())?;
    Ok(Tier {
        min_qty,
        unit_price_micro,
        currency,
    })
}

fn scalar(value: Option<&Value>) -> Result<String, &'static str> {
    match value {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.trim().to_owned()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(_) => Err("is not a text or a number"),
    }
}

fn json_offers(
    value: Option<&Value>,
    s: &PartsSettings,
    fail: &dyn Fn(String) -> PartsError,
) -> Result<Vec<Offer>, PartsError> {
    let list = match value {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(list)) => list,
        Some(_) => return Err(fail("offers is not a list".to_owned())),
    };
    if list.len() > s.max_offers {
        return Err(limit(format!(
            "more than {} offers in one part",
            s.max_offers
        )));
    }
    let mut offers = Vec::new();
    for item in list {
        let map = item
            .as_object()
            .ok_or_else(|| fail("an offer is not an object".to_owned()))?;
        if let Some(key) = map
            .keys()
            .find(|k| !matches!(k.as_str(), "supplier" | "supplier_code" | "prices"))
        {
            return Err(fail(format!("unknown key {key} in an offer")));
        }
        let text = |key: &str| -> Result<String, PartsError> {
            scalar(map.get(key)).map_err(|e| fail(format!("offer {key} {e}")))
        };
        let supplier = text("supplier")?;
        if supplier.is_empty() {
            return Err(fail("an offer has no supplier".to_owned()));
        }
        let mut tiers = Vec::new();
        match map.get("prices") {
            None | Some(Value::Null) => {}
            Some(Value::Array(prices)) => {
                if prices.len() > s.max_tiers {
                    return Err(limit(format!(
                        "more than {} price breaks in one offer",
                        s.max_tiers
                    )));
                }
                for price in prices {
                    let pm = price
                        .as_object()
                        .ok_or_else(|| fail("a price is not an object".to_owned()))?;
                    if let Some(key) = pm
                        .keys()
                        .find(|k| !matches!(k.as_str(), "min_qty" | "unit_price" | "currency"))
                    {
                        return Err(fail(format!("unknown key {key} in a price")));
                    }
                    let cell = |key: &str| -> Result<String, PartsError> {
                        scalar(pm.get(key)).map_err(|e| fail(format!("price {key} {e}")))
                    };
                    let tier =
                        make_tier(&cell("min_qty")?, &cell("unit_price")?, cell("currency")?)
                            .map_err(fail)?;
                    tiers.push(tier);
                }
            }
            Some(_) => return Err(fail("prices is not a list".to_owned())),
        }
        offers.push(Offer {
            supplier,
            supplier_code: text("supplier_code")?,
            tiers,
        });
    }
    Ok(offers)
}

fn json_part(item: &Value, origin: &str, s: &PartsSettings) -> Result<Part, PartsError> {
    let fail = |what: String| PartsError::new(FORMAT_INVALID, format!("{origin}: {what}"));
    let map = item
        .as_object()
        .ok_or_else(|| fail("the part is not an object".to_owned()))?;
    if let Some(key) = map.keys().find(|k| !PART_KEYS.contains(&k.as_str())) {
        return Err(fail(format!("unknown key {key}")));
    }
    let text = |key: &str| -> Result<String, PartsError> {
        scalar(map.get(key)).map_err(|e| fail(format!("{key} {e}")))
    };
    let id = text("id")?;
    if id.is_empty() {
        return Err(fail("the id is empty".to_owned()));
    }
    let status = Status::parse(&text("status")?)
        .ok_or_else(|| fail("status is not active, not-recommended or obsolete".to_owned()))?;
    let mut params = BTreeMap::new();
    match map.get("params") {
        None | Some(Value::Null) => {}
        Some(Value::Object(m)) => {
            for (key, value) in m {
                let value = scalar(Some(value)).map_err(|e| fail(format!("param {key} {e}")))?;
                params.insert(key.clone(), value);
            }
        }
        Some(_) => return Err(fail("params is not an object".to_owned())),
    }
    let mut alternatives = Vec::new();
    match map.get("alternatives") {
        None | Some(Value::Null) => {}
        Some(Value::Array(list)) => {
            for entry in list {
                let name = entry
                    .as_str()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .ok_or_else(|| fail("an alternative is not a text".to_owned()))?;
                alternatives.push(name.to_owned());
            }
        }
        Some(_) => return Err(fail("alternatives is not a list".to_owned())),
    }
    Ok(Part {
        id,
        value: text("value")?,
        tolerance: text("tolerance")?,
        power: text("power")?,
        voltage: text("voltage")?,
        footprint: text("footprint")?,
        symbol: text("symbol")?,
        manufacturer: text("manufacturer")?,
        mpn: text("mpn")?,
        status,
        params,
        sim_model: text("sim_model")?,
        offers: json_offers(map.get("offers"), s, &fail)?,
        notes: text("notes")?,
        alternatives,
        origin: origin.to_owned(),
    })
}

/// Reads a JSON file of the base; `file` is the name used in the messages.
pub fn parse_json(text: &str, file: &str, s: &PartsSettings) -> Result<Vec<Part>, PartsError> {
    let bad = |what: String| PartsError::new(FORMAT_INVALID, format!("{file}: {what}"));
    let value: Value = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
    let map = value
        .as_object()
        .ok_or_else(|| bad("a JSON object is expected".to_owned()))?;
    if map.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(bad(format!("the format must be {FORMAT}")));
    }
    if let Some(key) = map
        .keys()
        .find(|k| !matches!(k.as_str(), "format" | "parts"))
    {
        return Err(bad(format!("unknown key {key}")));
    }
    let list = map
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("there is no list under parts".to_owned()))?;
    if list.len() > s.max_parts {
        return Err(limit(format!("{file}: more than {} parts", s.max_parts)));
    }
    let mut parts = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        parts.push(json_part(item, &format!("{file}: part {}", index + 1), s)?);
    }
    Ok(parts)
}

/// The text of a cell: trimmed, and without the quote mark that the writer puts in front of a
/// text that a spreadsheet could read as a formula.
fn cell(table: &Table, cells: &[String], name: &str) -> String {
    let text = table
        .column(name)
        .and_then(|i| cells.get(i))
        .map(|c| c.trim().to_owned())
        .unwrap_or_default();
    let mut chars = text.chars();
    let marked = chars.next() == Some('\'')
        && chars
            .next()
            .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    if marked {
        text[1..].to_owned()
    } else {
        text
    }
}

fn merge_text(slot: &mut String, new: String, name: &str) -> Result<(), String> {
    if new.is_empty() || *slot == new {
        return Ok(());
    }
    if slot.is_empty() {
        *slot = new;
        return Ok(());
    }
    Err(format!("{name} differs from the first row of the part"))
}

/// Reads a CSV file of the base; `file` is the name used in the messages.
pub fn parse_csv(text: &str, file: &str, s: &PartsSettings) -> Result<Vec<Part>, PartsError> {
    let table =
        parse_table(text).map_err(|e| PartsError::new(FORMAT_INVALID, format!("{file}: {e}")))?;
    if let Some(name) = table.header.iter().find(|h| !COLUMNS.contains(&h.as_str())) {
        return Err(PartsError::new(
            FORMAT_INVALID,
            format!("{file}: line 1: unknown column {name}"),
        ));
    }
    if table.column("id").is_none() && table.column("mpn").is_none() {
        return Err(PartsError::new(
            FORMAT_INVALID,
            format!("{file}: line 1: a column id or mpn is needed"),
        ));
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut by_id: BTreeMap<String, usize> = BTreeMap::new();
    for (line, cells) in &table.rows {
        let line = *line;
        let fail =
            |what: String| PartsError::new(FORMAT_INVALID, format!("{file}: line {line}: {what}"));
        let get = |name: &str| cell(&table, cells, name);
        let mut id = get("id");
        if id.is_empty() {
            id = get("mpn");
        }
        if id.is_empty() {
            return Err(fail("neither id nor mpn".to_owned()));
        }
        let index = match by_id.get(&id) {
            Some(i) => *i,
            None => {
                if parts.len() >= s.max_parts {
                    return Err(limit(format!("{file}: more than {} parts", s.max_parts)));
                }
                let status = Status::parse(&get("status")).ok_or_else(|| {
                    fail("status is not active, not-recommended or obsolete".to_owned())
                })?;
                parts.push(Part {
                    id: id.clone(),
                    status,
                    origin: format!("{file}: line {line}"),
                    ..Part::default()
                });
                by_id.insert(id, parts.len() - 1);
                parts.len() - 1
            }
        };
        let part = &mut parts[index];
        let fields: [(&str, &mut String); 10] = [
            ("value", &mut part.value),
            ("tolerance", &mut part.tolerance),
            ("power", &mut part.power),
            ("voltage", &mut part.voltage),
            ("footprint", &mut part.footprint),
            ("symbol", &mut part.symbol),
            ("manufacturer", &mut part.manufacturer),
            ("mpn", &mut part.mpn),
            ("sim_model", &mut part.sim_model),
            ("notes", &mut part.notes),
        ];
        for (name, slot) in fields {
            merge_text(slot, get(name), name).map_err(fail)?;
        }
        let status = get("status");
        if !status.is_empty() {
            let parsed = Status::parse(&status).ok_or_else(|| {
                fail("status is not active, not-recommended or obsolete".to_owned())
            })?;
            if parsed != part.status {
                return Err(fail(
                    "status differs from the first row of the part".to_owned(),
                ));
            }
        }
        for item in get("params").split(';') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (key, value) = item
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .filter(|(k, _)| !k.is_empty())
                .ok_or_else(|| fail("params are name=value joined by ;".to_owned()))?;
            match part.params.get(key) {
                Some(old) if old != value => {
                    return Err(fail(format!("param {key} differs from the first row")))
                }
                Some(_) => {}
                None => {
                    part.params.insert(key.to_owned(), value.to_owned());
                }
            }
        }
        for item in get("alternatives").split(';') {
            let item = item.trim();
            if !item.is_empty() && !part.alternatives.iter().any(|a| a.as_str() == item) {
                part.alternatives.push(item.to_owned());
            }
        }
        let supplier = get("supplier");
        let price = get("unit_price");
        if supplier.is_empty() {
            if !price.is_empty() {
                return Err(fail("a price needs a supplier".to_owned()));
            }
            continue;
        }
        let code = get("supplier_code");
        let position = part
            .offers
            .iter()
            .position(|o| o.supplier == supplier && o.supplier_code == code);
        let at = match position {
            Some(p) => p,
            None => {
                if part.offers.len() >= s.max_offers {
                    return Err(limit(format!(
                        "{file}: line {line}: more than {} offers in one part",
                        s.max_offers
                    )));
                }
                part.offers.push(Offer {
                    supplier,
                    supplier_code: code,
                    tiers: Vec::new(),
                });
                part.offers.len() - 1
            }
        };
        if !price.is_empty() {
            let tier = make_tier(&get("min_qty"), &price, get("currency")).map_err(fail)?;
            let offer = &mut part.offers[at];
            if offer.tiers.len() >= s.max_tiers {
                return Err(limit(format!(
                    "{file}: line {line}: more than {} price breaks in one offer",
                    s.max_tiers
                )));
            }
            offer.tiers.push(tier);
        }
    }
    Ok(parts)
}

/// Reads a file of the base by its extension (`.json` or `.csv`).
pub fn parse_any(file: &str, text: &str, s: &PartsSettings) -> Result<Vec<Part>, PartsError> {
    let lower = file.to_ascii_lowercase();
    if lower.ends_with(".json") {
        parse_json(text, file, s)
    } else if lower.ends_with(".csv") {
        parse_csv(text, file, s)
    } else {
        Err(PartsError::new(
            FORMAT_INVALID,
            format!("{file}: the file is not .json or .csv"),
        ))
    }
}

fn read_file(path: &Path, s: &PartsSettings) -> Result<Vec<Part>, PartsError> {
    let unusable = |e: String| PartsError::new(FILE_UNUSABLE, format!("{}: {e}", path.display()));
    let length = fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if length > s.max_file_bytes {
        return Err(unusable(format!("larger than {} bytes", s.max_file_bytes)));
    }
    let bytes = fs::read(path).map_err(|e| unusable(e.to_string()))?;
    let text = String::from_utf8(bytes).map_err(|_| unusable("not UTF-8 text".to_owned()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    parse_any(&name, &text, s)
}

/// Loads the base from a file, or from every `.json` and `.csv` file of a folder, in the order of
/// their names (links are not followed, subfolders are not entered).
pub fn load(path: &Path, s: &PartsSettings) -> Result<PartsDb, PartsError> {
    let unusable = |e: String| PartsError::new(FILE_UNUSABLE, format!("{}: {e}", path.display()));
    let meta = fs::metadata(path).map_err(|e| unusable(e.to_string()))?;
    let mut files: Vec<PathBuf> = Vec::new();
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| unusable(e.to_string()))? {
            let entry = entry.map_err(|e| unusable(e.to_string()))?;
            let is_file = entry
                .file_type()
                .map_err(|e| unusable(e.to_string()))?
                .is_file();
            let found = entry.path();
            let extension = found
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase);
            if is_file && matches!(extension.as_deref(), Some("json" | "csv")) {
                files.push(found);
            }
        }
        files.sort();
        if files.len() > s.max_files {
            return Err(limit(format!(
                "{} files in the folder, the limit is {}",
                files.len(),
                s.max_files
            )));
        }
    } else {
        files.push(path.to_path_buf());
    }
    let mut db = PartsDb::default();
    for file in &files {
        let parts = read_file(file, s)?;
        if db.parts.len() + parts.len() > s.max_parts {
            return Err(limit(format!("more than {} parts", s.max_parts)));
        }
        db.parts.extend(parts);
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{
  "format": "phoenix-parts/1",
  "parts": [
    {"id": "r-10k", "value": "10k", "tolerance": "1%", "power": "0.1W", "voltage": "75V",
     "footprint": "R0603", "symbol": "R", "manufacturer": "Acme", "mpn": "AC-10K",
     "status": "active", "params": {"tcr": "100ppm", "size": 603}, "sim_model": "res",
     "notes": "general use", "alternatives": ["r-10k-b"],
     "offers": [{"supplier": "Shop A", "supplier_code": "A-1",
                 "prices": [{"min_qty": 1, "unit_price": "0.0100", "currency": "EUR"},
                            {"min_qty": 100, "unit_price": 0.004, "currency": "EUR"}]}]},
    {"id": "r-10k-b", "value": "10k"}
  ]
}"#;

    const CSV: &str = "id,value,footprint,mpn,status,alternatives,params,supplier,supplier_code,min_qty,unit_price,currency\n\
r-10k,10k,R0603,AC-10K,,r-10k-b;r-10k-c,tcr=100ppm;size=603,Shop A,A-1,1,0.0100,EUR\n\
r-10k,,,,,,,Shop A,A-1,100,0.0040,EUR\n\
r-10k,,R0603,,active,r-10k-b,,Shop B,B-1,1,0.02,EUR\n\
,22k,R0603,AC-22K,obsolete,,,,,,,\n";

    #[test]
    fn a_json_file_is_read_field_by_field() {
        let parts = parse_json(JSON, "base.json", &PartsSettings::default()).unwrap();
        assert_eq!(parts.len(), 2);
        let p = &parts[0];
        assert_eq!(
            (p.id.as_str(), p.value.as_str(), p.footprint.as_str()),
            ("r-10k", "10k", "R0603")
        );
        assert_eq!(p.manufacturer, "Acme");
        assert_eq!(p.status, Status::Active);
        assert_eq!(p.params["size"], "603");
        assert_eq!(p.params["tcr"], "100ppm");
        assert_eq!(p.alternatives, vec!["r-10k-b".to_owned()]);
        assert_eq!(p.origin, "base.json: part 1");
        assert_eq!(p.offers.len(), 1);
        let tiers = &p.offers[0].tiers;
        assert_eq!(tiers.len(), 2);
        assert_eq!((tiers[0].min_qty, tiers[0].unit_price_micro), (1, 10_000));
        assert_eq!((tiers[1].min_qty, tiers[1].unit_price_micro), (100, 4_000));
        assert_eq!(tiers[1].currency, "EUR");
        assert_eq!(parts[1].origin, "base.json: part 2");
        assert!(parts[1].offers.is_empty());
    }

    #[test]
    fn a_csv_file_joins_the_rows_of_one_part() {
        let parts = parse_csv(CSV, "base.csv", &PartsSettings::default()).unwrap();
        assert_eq!(parts.len(), 2);
        let p = &parts[0];
        assert_eq!(p.id, "r-10k");
        assert_eq!(p.origin, "base.csv: line 2");
        assert_eq!(
            p.alternatives,
            vec!["r-10k-b".to_owned(), "r-10k-c".to_owned()]
        );
        assert_eq!(p.params.len(), 2);
        assert_eq!(p.offers.len(), 2);
        assert_eq!(p.offers[0].tiers.len(), 2);
        assert_eq!(p.offers[0].tiers[1].min_qty, 100);
        assert_eq!(p.offers[1].supplier, "Shop B");
        // a row without id takes its part number as the id
        assert_eq!(parts[1].id, "AC-22K");
        assert_eq!(parts[1].status, Status::Obsolete);
        assert!(parts[1].offers.is_empty());
    }

    #[test]
    fn a_json_file_that_is_wrong_is_refused_with_the_part() {
        let s = PartsSettings::default();
        for (text, needle) in [
            ("{", "base.json: EOF while parsing"),
            ("[]", "a JSON object is expected"),
            (r#"{"format":"x","parts":[]}"#, "the format must be"),
            (r#"{"format":"phoenix-parts/1"}"#, "no list under parts"),
            (
                r#"{"format":"phoenix-parts/1","parts":[],"x":1}"#,
                "unknown key x",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[1]}"#,
                "part 1: the part is not an object",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{}]}"#,
                "part 1: the id is empty",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a"},{"id":"b","colour":1}]}"#,
                "part 2: unknown key colour",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a","status":"maybe"}]}"#,
                "status is not",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a","params":[]}]}"#,
                "params is not an object",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a","offers":[{"prices":[]}]}]}"#,
                "an offer has no supplier",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a","offers":[{"supplier":"S","prices":[{"unit_price":"x"}]}]}]}"#,
                "unit_price is not a decimal",
            ),
            (
                r#"{"format":"phoenix-parts/1","parts":[{"id":"a","offers":[{"supplier":"S","prices":[{"min_qty":0,"unit_price":"1"}]}]}]}"#,
                "min_qty is not a whole number",
            ),
        ] {
            let e = parse_json(text, "base.json", &s).unwrap_err();
            assert_eq!(e.code, FORMAT_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn a_csv_file_that_is_wrong_is_refused_with_the_line() {
        let s = PartsSettings::default();
        for (text, needle) in [
            ("", "the file has no header line"),
            ("id,colour\na,b\n", "line 1: unknown column colour"),
            ("value\n10k\n", "a column id or mpn is needed"),
            ("id,value\n,10k\n", "line 2: neither id nor mpn"),
            ("id,status\na,maybe\n", "line 2: status is not"),
            ("id,value\na,10k\na,22k\n", "line 3: value differs"),
            (
                "id,status\na,active\na,obsolete\n",
                "line 3: status differs",
            ),
            ("id,params\na,x\n", "line 2: params are name=value"),
            ("id,params\na,x=1\na,x=2\n", "line 3: param x differs"),
            ("id,unit_price\na,1\n", "line 2: a price needs a supplier"),
            (
                "id,supplier,unit_price\na,S,abc\n",
                "line 2: unit_price is not a decimal",
            ),
            (
                "id,supplier,min_qty,unit_price\na,S,0,1\n",
                "line 2: min_qty",
            ),
            ("id\n\"a\n", "line 2: a quoted cell is not closed"),
        ] {
            let e = parse_csv(text, "base.csv", &s).unwrap_err();
            assert_eq!(e.code, FORMAT_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn the_limits_are_checked_while_reading() {
        let tiny = PartsSettings {
            max_parts: 1,
            max_offers: 1,
            max_tiers: 1,
            ..PartsSettings::default()
        };
        let two = r#"{"format":"phoenix-parts/1","parts":[{"id":"a"},{"id":"b"}]}"#;
        assert_eq!(
            parse_json(two, "x.json", &tiny).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        assert_eq!(
            parse_csv("id\na\nb\n", "x.csv", &tiny).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let offers = "id,supplier,supplier_code\na,S,1\na,S,2\n";
        assert_eq!(
            parse_csv(offers, "x.csv", &tiny).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let tiers = "id,supplier,min_qty,unit_price\na,S,1,1\na,S,2,1\n";
        assert_eq!(
            parse_csv(tiers, "x.csv", &tiny).unwrap_err().code,
            LIMIT_EXCEEDED
        );
    }

    #[test]
    fn the_marked_text_that_the_writer_makes_is_read_back_as_it_was() {
        let parts = parse_csv(
            "id,value\na,'+5V\nb,'plain\n",
            "x.csv",
            &PartsSettings::default(),
        )
        .unwrap();
        assert_eq!(parts[0].value, "+5V");
        assert_eq!(parts[1].value, "'plain");
    }

    #[test]
    fn a_folder_is_read_in_the_order_of_the_names_and_other_files_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.csv"), "id,value\nb1,1k\n").unwrap();
        fs::write(dir.path().join("a.json"), JSON).unwrap();
        fs::write(dir.path().join("notes.txt"), "not a part").unwrap();
        let db = load(dir.path(), &PartsSettings::default()).unwrap();
        let ids: Vec<&str> = db.parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["r-10k", "r-10k-b", "b1"]);
        let single = load(&dir.path().join("b.csv"), &PartsSettings::default()).unwrap();
        assert_eq!(single.parts.len(), 1);
        let e = load(&dir.path().join("none.csv"), &PartsSettings::default()).unwrap_err();
        assert_eq!(e.code, FILE_UNUSABLE);
        let tiny = PartsSettings {
            max_files: 1,
            ..PartsSettings::default()
        };
        assert_eq!(load(dir.path(), &tiny).unwrap_err().code, LIMIT_EXCEEDED);
        let small = PartsSettings {
            max_file_bytes: 10,
            ..PartsSettings::default()
        };
        assert_eq!(
            load(&dir.path().join("a.json"), &small).unwrap_err().code,
            FILE_UNUSABLE
        );
        fs::write(dir.path().join("c.csv"), [0xff, 0xfe, 0x41]).unwrap();
        assert_eq!(
            load(&dir.path().join("c.csv"), &PartsSettings::default())
                .unwrap_err()
                .code,
            FILE_UNUSABLE
        );
    }

    #[test]
    fn a_file_with_another_extension_is_refused() {
        let e = parse_any("base.txt", "", &PartsSettings::default()).unwrap_err();
        assert_eq!(e.code, FORMAT_INVALID);
    }
}
