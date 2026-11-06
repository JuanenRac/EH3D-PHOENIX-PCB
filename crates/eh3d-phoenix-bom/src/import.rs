// SPDX-License-Identifier: GPL-3.0-or-later
//! The import of a bill of materials written by another program: a CSV file with its own column
//! names, a delimiter of its own and several references in one cell, read into lines of the
//! structure of this bill with a map of columns from a settings file (`config/bom_import.cfg`).
//!
//! The map holds the names of the columns, not a format of any program: nothing here claims to
//! read the files of a particular program, and no real file has been compared. What is read is
//! the references, the value, the footprint, the maker, the part number, the quantity and whether
//! the part is fitted. The result can be written as a file of part data (`match,manufacturer,mpn,
//! fitted`, one line for each reference), or compared with the parts of a board.

use std::collections::{BTreeMap, BTreeSet};

use crate::bom::natural_key;
use crate::codes::{
    BomError, Finding, Severity, IMPORT_DISAGREES, IMPORT_DUPLICATE, IMPORT_QUANTITY,
    PARTS_INVALID, SETTINGS_INVALID,
};
use crate::parts::{PartData, PartsFile};
use crate::text::{csv_text, parse_table_with};

/// The most lines an imported file may have.
const MAX_ROWS_LIMIT: usize = 1_000_000;

/// How to read the bill of another program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMap {
    /// The character that separates the cells.
    pub delimiter: char,
    /// The names the column of the references may have, in lower case, the first that exists wins.
    pub references: Vec<String>,
    /// The names of the column of the value.
    pub value: Vec<String>,
    /// The names of the column of the footprint.
    pub footprint: Vec<String>,
    /// The names of the column of the maker.
    pub manufacturer: Vec<String>,
    /// The names of the column of the part number.
    pub mpn: Vec<String>,
    /// The names of the column of the quantity.
    pub quantity: Vec<String>,
    /// The names of a column that says whether the part is fitted.
    pub fitted: Vec<String>,
    /// The names of a column that says the part is not to be fitted.
    pub dnp: Vec<String>,
    /// The characters that separate the references in one cell.
    pub reference_separators: Vec<char>,
    /// The cells of a `fitted` column that mean the part is not fitted, in lower case.
    pub not_fitted_values: Vec<String>,
    /// The cells of a `dnp` column that mean the part is not fitted, in lower case.
    pub dnp_values: Vec<String>,
    /// The most lines read.
    pub max_rows: usize,
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

impl Default for ImportMap {
    /// Common English column names; they are a guess and not the layout of any program.
    fn default() -> Self {
        ImportMap {
            delimiter: ',',
            references: names(&["references", "reference", "designator", "refdes", "ref"]),
            value: names(&["value", "val", "comment"]),
            footprint: names(&["footprint", "package", "pattern"]),
            manufacturer: names(&["manufacturer", "maker", "mfr"]),
            mpn: names(&["mpn", "part number", "manufacturer part number"]),
            quantity: names(&["quantity", "qty"]),
            fitted: names(&["fitted"]),
            dnp: names(&["dnp", "do not populate"]),
            reference_separators: vec![',', ' ', ';'],
            not_fitted_values: names(&["no", "n", "false", "0", "dnp"]),
            dnp_values: names(&["yes", "y", "true", "1", "x", "dnp"]),
            max_rows: 100_000,
        }
    }
}

fn list(value: &str) -> Option<Vec<String>> {
    let items: Vec<String> = value
        .split('|')
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();
    items.iter().all(|s| !s.is_empty()).then_some(items)
}

impl ImportMap {
    /// Parses a settings text of `key = value` lines; `#` starts a comment. A list is written with
    /// bars between the items. A missing line keeps the default; an unknown key or a value that
    /// is not valid is refused with its line number.
    pub fn parse(text: &str) -> Result<ImportMap, BomError> {
        let mut map = ImportMap::default();
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
            match key {
                "delimiter" => {
                    map.delimiter = match value {
                        "comma" => ',',
                        "semicolon" => ';',
                        "tab" => '\t',
                        _ => return Err(bad()),
                    }
                }
                "references" => map.references = list(value).ok_or_else(bad)?,
                "value" => map.value = list(value).ok_or_else(bad)?,
                "footprint" => map.footprint = list(value).ok_or_else(bad)?,
                "manufacturer" => map.manufacturer = list(value).ok_or_else(bad)?,
                "mpn" => map.mpn = list(value).ok_or_else(bad)?,
                "quantity" => map.quantity = list(value).ok_or_else(bad)?,
                "fitted" => map.fitted = list(value).ok_or_else(bad)?,
                "dnp" => map.dnp = list(value).ok_or_else(bad)?,
                "not_fitted_values" => map.not_fitted_values = list(value).ok_or_else(bad)?,
                "dnp_values" => map.dnp_values = list(value).ok_or_else(bad)?,
                "reference_separators" => {
                    let chars: Vec<char> = value.chars().collect();
                    if chars.is_empty() || chars.contains(&'"') {
                        return Err(bad());
                    }
                    map.reference_separators = chars;
                }
                "max_rows" => {
                    map.max_rows = value
                        .parse()
                        .ok()
                        .filter(|n| (1..=MAX_ROWS_LIMIT).contains(n))
                        .ok_or_else(bad)?
                }
                _ => {
                    return Err(BomError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        Ok(map)
    }
}

/// One line of an imported bill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedLine {
    /// The line of the file.
    pub line: usize,
    /// The references, in the order the cell has them.
    pub references: Vec<String>,
    /// The value; empty when the file has none.
    pub value: String,
    /// The footprint; empty when the file has none.
    pub footprint: String,
    /// The maker; empty when the file has none.
    pub manufacturer: String,
    /// The part number; empty when the file has none.
    pub mpn: String,
    /// The quantity the line states, when the file has a column of quantities.
    pub quantity: Option<u64>,
    /// Whether the parts of the line are fitted.
    pub fitted: bool,
}

/// An imported bill with what was found while reading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedBill {
    /// The lines, in the order of the file.
    pub lines: Vec<ImportedLine>,
    /// A quantity that differs from the references, a reference in two lines.
    pub findings: Vec<Finding>,
}

fn invalid(what: String) -> BomError {
    BomError::new(PARTS_INVALID, what)
}

fn find_column(header: &[String], wanted: &[String]) -> Option<usize> {
    wanted
        .iter()
        .find_map(|name| header.iter().position(|h| h == name))
}

/// Reads the bill of another program with a map of columns.
pub fn import_bill(text: &str, map: &ImportMap) -> Result<ImportedBill, BomError> {
    let table = parse_table_with(text, map.delimiter).map_err(invalid)?;
    let refs_col = find_column(&table.header, &map.references).ok_or_else(|| {
        invalid(format!(
            "no column holds the references (looked for {}; the file has {})",
            map.references.join(", "),
            table.header.join(", ")
        ))
    })?;
    if table.rows.len() > map.max_rows {
        return Err(invalid(format!("more than {} lines", map.max_rows)));
    }
    let columns = [
        &map.value,
        &map.footprint,
        &map.manufacturer,
        &map.mpn,
        &map.quantity,
        &map.fitted,
        &map.dnp,
    ]
    .map(|names| find_column(&table.header, names));
    let [value_col, footprint_col, maker_col, mpn_col, qty_col, fitted_col, dnp_col] = columns;
    let mut lines = Vec::new();
    let mut findings = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (line, cells) in &table.rows {
        let cell = |col: Option<usize>| -> String {
            col.and_then(|i| cells.get(i)).cloned().unwrap_or_default()
        };
        let references: Vec<String> = cells
            .get(refs_col)
            .map(String::as_str)
            .unwrap_or("")
            .split(|c| map.reference_separators.contains(&c))
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(str::to_owned)
            .collect();
        if references.is_empty() {
            continue;
        }
        let quantity = match cell(qty_col).trim() {
            "" => None,
            text => Some(text.parse::<u64>().map_err(|_| {
                invalid(format!(
                    "line {line}: the quantity {text} is not a whole number"
                ))
            })?),
        };
        let mut fitted = true;
        if fitted_col.is_some()
            && map
                .not_fitted_values
                .contains(&cell(fitted_col).to_ascii_lowercase())
        {
            fitted = false;
        }
        if dnp_col.is_some() && map.dnp_values.contains(&cell(dnp_col).to_ascii_lowercase()) {
            fitted = false;
        }
        if let Some(q) = quantity {
            if q != references.len() as u64 {
                findings.push(Finding::new(
                    IMPORT_QUANTITY,
                    Severity::Warning,
                    Some(&references.join(", ")),
                    format!(
                        "line {line}: the quantity is {q}, the references are {}",
                        references.len()
                    ),
                ));
            }
        }
        for reference in &references {
            if let Some(first) = seen.insert(reference.clone(), *line) {
                findings.push(Finding::new(
                    IMPORT_DUPLICATE,
                    Severity::Warning,
                    Some(reference),
                    format!("lines {first} and {line}; the first one is used"),
                ));
            }
        }
        lines.push(ImportedLine {
            line: *line,
            references,
            value: cell(value_col),
            footprint: cell(footprint_col),
            manufacturer: cell(maker_col),
            mpn: cell(mpn_col),
            quantity,
            fitted,
        });
    }
    Ok(ImportedBill { lines, findings })
}

impl ImportedBill {
    /// The data of each reference, the first line that names it winning, for the references that
    /// have a maker, a part number or are not fitted; the others add nothing to a file of part
    /// data and would only hide the lookup by value and footprint.
    fn data(&self) -> BTreeMap<String, PartData> {
        let mut out = BTreeMap::new();
        for line in &self.lines {
            if line.manufacturer.is_empty() && line.mpn.is_empty() && line.fitted {
                continue;
            }
            for reference in &line.references {
                out.entry(reference.clone()).or_insert(PartData {
                    manufacturer: line.manufacturer.clone(),
                    mpn: line.mpn.clone(),
                    fitted: line.fitted,
                });
            }
        }
        out
    }

    /// The data of the parts as a file of part data, by reference.
    pub fn parts_file(&self) -> PartsFile {
        PartsFile {
            by_reference: self.data(),
            by_kind: BTreeMap::new(),
        }
    }

    /// The data of the parts as the CSV of `phoenix bom --parts`: `match,manufacturer,mpn,fitted`,
    /// a line for each reference in natural order, the same bytes for the same bill.
    pub fn parts_csv(&self) -> String {
        let mut rows: Vec<(String, PartData)> = self.data().into_iter().collect();
        rows.sort_by_key(|(r, _)| natural_key(r));
        let mut out = String::from("match,manufacturer,mpn,fitted\n");
        for (reference, data) in rows {
            out.push_str(&format!(
                "{},{},{},{}\n",
                csv_text(&reference),
                csv_text(&data.manufacturer),
                csv_text(&data.mpn),
                if data.fitted { "yes" } else { "no" }
            ));
        }
        out
    }

    /// Compares the bill with the parts of a board, given as reference and value: a part on one
    /// side only, and a value that differs (without regard to case or spaces at the ends).
    pub fn compare(&self, board: &[(String, String)]) -> Vec<Finding> {
        let on_board: BTreeMap<&str, &str> = board
            .iter()
            .map(|(r, v)| (r.as_str(), v.as_str()))
            .collect();
        let mut imported: BTreeMap<&str, &str> = BTreeMap::new();
        for line in &self.lines {
            for reference in &line.references {
                imported
                    .entry(reference.as_str())
                    .or_insert(line.value.as_str());
            }
        }
        let mut found = Vec::new();
        let mut order: Vec<&str> = imported.keys().copied().collect();
        order.sort_by_key(|r| natural_key(r));
        for reference in order {
            match on_board.get(reference) {
                None => found.push(Finding::new(
                    IMPORT_DISAGREES,
                    Severity::Warning,
                    Some(reference),
                    "in the imported bill, not on the board",
                )),
                Some(board_value) => {
                    let bill_value = imported[reference];
                    if !bill_value.is_empty()
                        && !bill_value.trim().eq_ignore_ascii_case(board_value.trim())
                    {
                        found.push(Finding::new(
                            IMPORT_DISAGREES,
                            Severity::Warning,
                            Some(reference),
                            format!("the bill says {bill_value}, the board has {board_value}"),
                        ));
                    }
                }
            }
        }
        let known: BTreeSet<&str> = imported.keys().copied().collect();
        let mut missing: Vec<&str> = on_board
            .keys()
            .copied()
            .filter(|r| !known.contains(r))
            .collect();
        missing.sort_by_key(|r| natural_key(r));
        for reference in missing {
            found.push(Finding::new(
                IMPORT_DISAGREES,
                Severity::Warning,
                Some(reference),
                "on the board, not in the imported bill",
            ));
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BILL: &str = "Designator;Comment;Package;Qty;Manufacturer;MPN;DNP\n\
\"R1, R2\";10k;R0603;2;Acme;AC-10K;\n\
C1;100n;C0603;1;;;\n\
R3;10k;R0603;1;;;x\n";

    fn semicolon() -> ImportMap {
        ImportMap::parse("delimiter = semicolon\n").unwrap()
    }

    #[test]
    fn the_columns_of_another_program_are_read_by_the_names_of_the_map() {
        let bill = import_bill(BILL, &semicolon()).unwrap();
        assert!(bill.findings.is_empty(), "{:?}", bill.findings);
        assert_eq!(bill.lines.len(), 3);
        let first = &bill.lines[0];
        assert_eq!(first.references, ["R1", "R2"]);
        assert_eq!(
            (first.value.as_str(), first.footprint.as_str()),
            ("10k", "R0603")
        );
        assert_eq!(first.quantity, Some(2));
        assert_eq!(first.mpn, "AC-10K");
        assert!(first.fitted);
        // the cell of the column dnp that holds x means not fitted
        assert!(!bill.lines[2].fitted);
    }

    #[test]
    fn the_data_of_the_parts_is_written_for_the_references_that_carry_any() {
        let bill = import_bill(BILL, &semicolon()).unwrap();
        assert_eq!(
            bill.parts_csv(),
            "match,manufacturer,mpn,fitted\nR1,Acme,AC-10K,yes\nR2,Acme,AC-10K,yes\nR3,,,no\n"
        );
        let file = bill.parts_file();
        assert_eq!(file.by_reference.len(), 3);
        assert!(file.by_reference["R2"].fitted);
        assert!(!file.by_reference["R3"].fitted);
        assert!(!file.by_reference.contains_key("C1"));
        // what is written is read back by the reader of part data
        let again = PartsFile::parse(&bill.parts_csv(), 100).unwrap();
        assert_eq!(again.by_reference, file.by_reference);
    }

    #[test]
    fn a_wrong_quantity_and_a_repeated_reference_are_findings() {
        let text = "references,qty\nR1 R2,3\nR2,1\n";
        let bill = import_bill(text, &ImportMap::default()).unwrap();
        let codes: Vec<&str> = bill.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, [IMPORT_QUANTITY, IMPORT_DUPLICATE]);
        assert!(bill.findings[0].detail.contains("quantity is 3"));
        assert!(bill.findings[1].detail.contains("lines 2 and 3"));
    }

    #[test]
    fn the_bill_is_compared_with_the_parts_of_a_board() {
        let bill = import_bill(
            "references,value\nR1,10K\nR2,4k7\nC9,1u\n",
            &ImportMap::default(),
        )
        .unwrap();
        let board = vec![
            ("R1".to_owned(), "10k".to_owned()),
            ("R2".to_owned(), "10k".to_owned()),
            ("R10".to_owned(), "1k".to_owned()),
        ];
        let found: Vec<(String, String)> = bill
            .compare(&board)
            .into_iter()
            .map(|f| (f.subject.unwrap_or_default(), f.detail))
            .collect();
        assert_eq!(
            found,
            [
                (
                    "C9".to_owned(),
                    "in the imported bill, not on the board".to_owned()
                ),
                (
                    "R2".to_owned(),
                    "the bill says 4k7, the board has 10k".to_owned()
                ),
                (
                    "R10".to_owned(),
                    "on the board, not in the imported bill".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_file_without_the_references_or_with_a_bad_quantity_is_refused() {
        for (text, needle) in [
            ("value,qty\n10k,1\n", "no column holds the references"),
            ("references,qty\nR1,two\n", "not a whole number"),
            ("", "no header"),
        ] {
            let e = import_bill(text, &ImportMap::default()).unwrap_err();
            assert_eq!(e.code, PARTS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let limited = ImportMap::parse("max_rows = 1\n").unwrap();
        assert!(import_bill("references\nR1\nR2\n", &limited).is_err());
    }

    #[test]
    fn the_map_is_read_and_what_is_wrong_in_it_is_refused_with_its_line() {
        let m = ImportMap::parse(
            "# a map\nreferences = Parts | RefDes\nreference_separators = ,;\ndelimiter = tab\n",
        )
        .unwrap();
        assert_eq!(m.references, ["parts", "refdes"]);
        assert_eq!(m.reference_separators, [',', ';']);
        assert_eq!(m.delimiter, '\t');
        for (text, needle) in [
            ("delimiter = pipe\n", "line 1: the value of delimiter"),
            ("\ncolour = red\n", "line 2: unknown key colour"),
            ("references = a||b\n", "references"),
            ("max_rows = 0\n", "max_rows"),
            ("oops\n", "not key = value"),
        ] {
            let e = ImportMap::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let shipped = include_str!("../../../config/bom_import.cfg");
        assert_eq!(ImportMap::parse(shipped).unwrap(), ImportMap::default());
    }
}
