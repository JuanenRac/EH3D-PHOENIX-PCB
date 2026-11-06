// SPDX-License-Identifier: GPL-3.0-or-later
//! Writing the base as JSON or CSV. The parts are written in the order of their ids, so the same
//! base gives the same file every time; the origin of a part is never written. A file is only
//! written when it does not exist yet.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_bom::money::format_micro;
use eh3d_phoenix_bom::text::{csv_raw, csv_text};
use serde_json::{json, Map, Value};

use crate::codes::{PartsError, FORMAT_INVALID, OUTPUT_FAILED};
use crate::model::{Part, FORMAT};
use crate::read::COLUMNS;

/// A price as text: millionths as a decimal without trailing zeros and with at least two
/// decimals (`0.0100` is written `0.01`).
pub fn price_text(micro: i64) -> String {
    let full = format_micro(i128::from(micro));
    let (whole, fraction) = full.split_once('.').unwrap_or((full.as_str(), ""));
    let mut fraction = fraction.trim_end_matches('0').to_owned();
    while fraction.len() < 2 {
        fraction.push('0');
    }
    format!("{whole}.{fraction}")
}

fn sorted(parts: &[Part]) -> Vec<&Part> {
    let mut list: Vec<&Part> = parts.iter().collect();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}

fn put(map: &mut Map<String, Value>, key: &str, value: &str) {
    if !value.is_empty() {
        map.insert(key.to_owned(), json!(value));
    }
}

/// One part as a JSON object, with the same keys as the file.
pub fn part_json(p: &Part) -> Value {
    let mut map = Map::new();
    map.insert("id".to_owned(), json!(p.id));
    put(&mut map, "value", &p.value);
    put(&mut map, "tolerance", &p.tolerance);
    put(&mut map, "power", &p.power);
    put(&mut map, "voltage", &p.voltage);
    put(&mut map, "footprint", &p.footprint);
    put(&mut map, "symbol", &p.symbol);
    put(&mut map, "manufacturer", &p.manufacturer);
    put(&mut map, "mpn", &p.mpn);
    map.insert("status".to_owned(), json!(p.status.as_str()));
    put(&mut map, "sim_model", &p.sim_model);
    put(&mut map, "notes", &p.notes);
    if !p.params.is_empty() {
        map.insert("params".to_owned(), json!(p.params));
    }
    if !p.alternatives.is_empty() {
        map.insert("alternatives".to_owned(), json!(p.alternatives));
    }
    if !p.offers.is_empty() {
        let offers: Vec<Value> = p
            .offers
            .iter()
            .map(|o| {
                let mut offer = Map::new();
                offer.insert("supplier".to_owned(), json!(o.supplier));
                put(&mut offer, "supplier_code", &o.supplier_code);
                let prices: Vec<Value> = o
                    .tiers
                    .iter()
                    .map(|t| {
                        let mut price = Map::new();
                        price.insert("min_qty".to_owned(), json!(t.min_qty));
                        price.insert(
                            "unit_price".to_owned(),
                            json!(price_text(t.unit_price_micro)),
                        );
                        put(&mut price, "currency", &t.currency);
                        Value::Object(price)
                    })
                    .collect();
                if !prices.is_empty() {
                    offer.insert("prices".to_owned(), Value::Array(prices));
                }
                Value::Object(offer)
            })
            .collect();
        map.insert("offers".to_owned(), Value::Array(offers));
    }
    Value::Object(map)
}

/// The base as JSON (`phoenix-parts/1`), with a line feed at the end.
pub fn to_json(parts: &[Part]) -> String {
    let list: Vec<Value> = sorted(parts).into_iter().map(part_json).collect();
    let document = json!({ "format": FORMAT, "parts": list });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

/// The base as CSV. A part with several suppliers or price breaks takes several rows. A parameter
/// or an alternative that holds a `;` (or a parameter name that holds a `=`) cannot be written
/// as CSV and is refused: write JSON instead.
pub fn to_csv(parts: &[Part]) -> Result<String, PartsError> {
    let mut out = COLUMNS.join(",");
    out.push('\n');
    for p in sorted(parts) {
        let refuse =
            |what: &str| PartsError::new(FORMAT_INVALID, format!("{}: {what}; write JSON", p.id));
        if p.alternatives.iter().any(|a| a.contains(';')) {
            return Err(refuse("an alternative holds a ;"));
        }
        if p.params
            .iter()
            .any(|(k, v)| k.contains([';', '=']) || v.contains(';'))
        {
            return Err(refuse("a parameter holds a ; or a ="));
        }
        let params: Vec<String> = p.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
        // the rows of the part: each one supplier and, when it has them, one price break
        let mut rows: Vec<[String; 5]> = Vec::new();
        for o in &p.offers {
            if o.tiers.is_empty() {
                rows.push([
                    csv_text(&o.supplier),
                    csv_text(&o.supplier_code),
                    String::new(),
                    String::new(),
                    String::new(),
                ]);
            }
            for t in &o.tiers {
                rows.push([
                    csv_text(&o.supplier),
                    csv_text(&o.supplier_code),
                    csv_raw(&t.min_qty.to_string()),
                    csv_raw(&price_text(t.unit_price_micro)),
                    csv_text(&t.currency),
                ]);
            }
        }
        if rows.is_empty() {
            rows.push(Default::default());
        }
        for (index, row) in rows.iter().enumerate() {
            let mut cells: Vec<String> = vec![csv_text(&p.id)];
            if index == 0 {
                cells.extend([
                    csv_text(&p.value),
                    csv_text(&p.tolerance),
                    csv_text(&p.power),
                    csv_text(&p.voltage),
                    csv_text(&p.footprint),
                    csv_text(&p.symbol),
                    csv_text(&p.manufacturer),
                    csv_text(&p.mpn),
                    p.status.as_str().to_owned(),
                    csv_text(&p.sim_model),
                    csv_text(&p.notes),
                    csv_text(&p.alternatives.join(";")),
                    csv_text(&params.join(";")),
                ]);
            } else {
                cells.extend(std::iter::repeat_n(String::new(), 13));
            }
            cells.extend(row.iter().cloned());
            out.push_str(&cells.join(","));
            out.push('\n');
        }
    }
    Ok(out)
}

/// Writes a file that must not exist yet.
pub fn write_new(path: &Path, text: &str) -> Result<(), PartsError> {
    let fail =
        |e: std::io::Error| PartsError::new(OUTPUT_FAILED, format!("{}: {e}", path.display()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Offer, Status, Tier};
    use crate::read::{parse_csv, parse_json};
    use crate::settings::PartsSettings;

    fn sample() -> Vec<Part> {
        let mut b = Part {
            id: "b-part".into(),
            value: "-5V".into(),
            ..Part::default()
        };
        b.status = Status::Obsolete;
        b.alternatives = vec!["a-part".into()];
        let mut a = Part {
            id: "a-part".into(),
            value: "10k".into(),
            tolerance: "1%".into(),
            footprint: "R0603".into(),
            manufacturer: "Acme, Inc".into(),
            mpn: "AC-10K".into(),
            notes: "line one\nline two".into(),
            sim_model: "res".into(),
            ..Part::default()
        };
        a.params.insert("tcr".into(), "100ppm".into());
        a.params.insert("size".into(), "0603".into());
        a.offers = vec![
            Offer {
                supplier: "Shop A".into(),
                supplier_code: "A-1".into(),
                tiers: vec![
                    Tier {
                        min_qty: 1,
                        unit_price_micro: 10_000,
                        currency: "EUR".into(),
                    },
                    Tier {
                        min_qty: 100,
                        unit_price_micro: 4_000,
                        currency: "EUR".into(),
                    },
                ],
            },
            Offer {
                supplier: "Shop B".into(),
                supplier_code: String::new(),
                tiers: Vec::new(),
            },
        ];
        vec![b, a]
    }

    fn without_origin(mut parts: Vec<Part>) -> Vec<Part> {
        for p in &mut parts {
            p.origin.clear();
        }
        parts.sort_by(|a, b| a.id.cmp(&b.id));
        parts
    }

    #[test]
    fn prices_are_written_without_trailing_zeros_and_with_two_decimals() {
        assert_eq!(price_text(10_000), "0.01");
        assert_eq!(price_text(4_000), "0.004");
        assert_eq!(price_text(1_000_000), "1.00");
        assert_eq!(price_text(1), "0.000001");
        assert_eq!(price_text(12_500_000), "12.50");
    }

    #[test]
    fn json_is_sorted_by_id_and_reads_back_the_same() {
        let text = to_json(&sample());
        assert!(text.ends_with("}\n"));
        assert!(text.find("a-part").unwrap() < text.find("b-part").unwrap());
        assert!(text.contains("\"format\": \"phoenix-parts/1\""));
        assert_eq!(to_json(&sample()), text);
        let back = parse_json(&text, "x.json", &PartsSettings::default()).unwrap();
        assert_eq!(without_origin(back), without_origin(sample()));
    }

    #[test]
    fn csv_has_a_row_for_each_price_break_and_reads_back_the_same() {
        let text = to_csv(&sample()).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), COLUMNS.join(","));
        // the value that starts with a minus sign is marked for the spreadsheets
        assert!(text.contains("'-5V"));
        assert_eq!(to_csv(&sample()).unwrap(), text);
        let back = parse_csv(&text, "x.csv", &PartsSettings::default()).unwrap();
        assert_eq!(without_origin(back), without_origin(sample()));
    }

    #[test]
    fn a_base_without_parts_is_a_header_and_an_empty_list() {
        assert_eq!(to_csv(&[]).unwrap(), format!("{}\n", COLUMNS.join(",")));
        assert!(to_json(&[]).contains("\"parts\": []"));
    }

    #[test]
    fn what_csv_cannot_hold_is_refused() {
        let mut p = Part {
            id: "x".into(),
            ..Part::default()
        };
        p.params.insert("a".into(), "1;2".into());
        let e = to_csv(&[p.clone()]).unwrap_err();
        assert_eq!(e.code, FORMAT_INVALID);
        assert!(e.detail.contains("write JSON"));
        p.params.clear();
        p.alternatives = vec!["a;b".into()];
        assert!(to_csv(&[p]).is_err());
        // JSON holds them
        let mut q = Part {
            id: "q".into(),
            ..Part::default()
        };
        q.params.insert("a=b".into(), "1;2".into());
        assert!(to_json(&[q]).contains("a=b"));
    }

    #[test]
    fn a_file_is_only_written_when_it_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("parts.json");
        write_new(&path, "one").unwrap();
        let e = write_new(&path, "two").unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one");
    }
}
