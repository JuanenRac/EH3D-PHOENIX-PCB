// SPDX-License-Identifier: GPL-3.0-or-later
//! The bill of materials as the CSV file an assembler asks for. A format is a list of columns,
//! each with the header the assembler wants and the field of the bill that fills it, read from
//! `config/assembler.cfg`; two formats come with the program and any number can be added.
//!
//! **State: prototype.** The two formats that come with the program are written from the
//! headers that the author knows those assemblers ask for; they have never been sent to an
//! assembler, so they are not verified. The same input always gives the same bytes.

use crate::bom::BomLine;
use crate::codes::{BomError, SETTINGS_INVALID};

/// The fields of a line of the bill that a column can hold.
///
/// * `item`: the number of the line, from 1;
/// * `references`: the references joined by the separator of the settings;
/// * `quantity`, `value`, `footprint`, `manufacturer`, `mpn`: as in the bill;
/// * `supplier`, `supplier_code`: the offer that was chosen (empty when there is none);
/// * `matched_code`: the code of the offer only when its supplier is the one named by
///   `code_supplier` in the settings (compared without regard to case), else empty;
/// * `blank`: an empty cell, for a column that the assembler asks for and the bill does not hold.
pub const FIELDS: [&str; 10] = [
    "item",
    "references",
    "quantity",
    "value",
    "footprint",
    "manufacturer",
    "mpn",
    "supplier",
    "supplier_code",
    "matched_code",
];

/// The fields that are numbers worked out by the program (not text from a file).
const NUMERIC_FIELDS: [&str; 2] = ["item", "quantity"];

/// The most formats, columns of a format and characters of a header a settings file may hold.
const MAX_FORMATS: usize = 64;
const MAX_COLUMNS: usize = 32;
const MAX_HEADER: usize = 64;

/// One format: its name and its columns, each a header and the name of a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerFormat {
    /// The name given to `--assembler`, 1 to 32 lower case letters, digits, `_` or `-`.
    pub name: String,
    /// The header of each column and the field that fills it, in the order written.
    pub columns: Vec<(String, String)>,
}

/// The settings of the assembler files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerSettings {
    /// The formats, in the order of the file.
    pub formats: Vec<AssemblerFormat>,
    /// The character between the cells.
    pub delimiter: char,
    /// The character between the references of one cell.
    pub references_separator: char,
    /// The supplier whose code `matched_code` writes; empty for any supplier.
    pub code_supplier: String,
}

fn format_of(name: &str, columns: &[(&str, &str)]) -> AssemblerFormat {
    AssemblerFormat {
        name: name.to_owned(),
        columns: columns
            .iter()
            .map(|(h, f)| ((*h).to_owned(), (*f).to_owned()))
            .collect(),
    }
}

impl Default for AssemblerSettings {
    fn default() -> Self {
        AssemblerSettings {
            formats: vec![
                format_of(
                    "jlcpcb",
                    &[
                        ("Comment", "value"),
                        ("Designator", "references"),
                        ("Footprint", "footprint"),
                        ("LCSC Part #", "matched_code"),
                    ],
                ),
                format_of(
                    "pcbway",
                    &[
                        ("Item #", "item"),
                        ("Designator", "references"),
                        ("Qty", "quantity"),
                        ("Manufacturer", "manufacturer"),
                        ("Mfg Part #", "mpn"),
                        ("Description / Value", "value"),
                        ("Package/Footprint", "footprint"),
                    ],
                ),
            ],
            delimiter: ',',
            references_separator: ',',
            code_supplier: "LCSC".to_owned(),
        }
    }
}

/// The word of a settings file for a separator character.
fn separator_char(word: &str) -> Option<char> {
    match word {
        "comma" => Some(','),
        "semicolon" => Some(';'),
        "tab" => Some('\t'),
        "space" => Some(' '),
        _ => None,
    }
}

fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Reads `Header=field|Header=field|...`: at least one column, a header of 1 to 64 printable
/// characters with no `=` or `|`, and a field that is one of [`FIELDS`] or `blank`.
fn parse_columns(text: &str) -> Option<Vec<(String, String)>> {
    let mut columns = Vec::new();
    for part in text.split('|') {
        let (header, field) = part.split_once('=')?;
        let (header, field) = (header.trim(), field.trim());
        let header_ok = !header.is_empty()
            && header.chars().count() <= MAX_HEADER
            && !header.chars().any(char::is_control);
        if !header_ok || !(field == "blank" || FIELDS.contains(&field)) {
            return None;
        }
        columns.push((header.to_owned(), field.to_owned()));
    }
    (columns.len() <= MAX_COLUMNS).then_some(columns)
}

impl AssemblerSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored. A line
    /// `format.NAME = ...` replaces the format of that name or adds a new one.
    pub fn parse(text: &str) -> Result<AssemblerSettings, BomError> {
        let mut s = AssemblerSettings::default();
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
            if let Some(name) = key.strip_prefix("format.") {
                if !valid_name(name) {
                    return Err(bad());
                }
                let columns = parse_columns(value).ok_or_else(bad)?;
                let format = AssemblerFormat {
                    name: name.to_owned(),
                    columns,
                };
                match s.formats.iter_mut().find(|f| f.name == name) {
                    Some(existing) => *existing = format,
                    None => {
                        if s.formats.len() >= MAX_FORMATS {
                            return Err(bad());
                        }
                        s.formats.push(format);
                    }
                }
                continue;
            }
            match key {
                "delimiter" => {
                    s.delimiter = separator_char(value)
                        .filter(|c| *c != ' ')
                        .ok_or_else(bad)?
                }
                "references_separator" => {
                    s.references_separator = separator_char(value).ok_or_else(bad)?
                }
                "code_supplier" => {
                    if value.chars().count() > 64 || value.chars().any(char::is_control) {
                        return Err(bad());
                    }
                    s.code_supplier = value.to_owned();
                }
                _ => {
                    return Err(BomError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        Ok(s)
    }

    /// The format of a name.
    pub fn format(&self, name: &str) -> Option<&AssemblerFormat> {
        self.formats.iter().find(|f| f.name == name)
    }

    /// The names of the formats, joined by commas, for a message.
    pub fn names(&self) -> String {
        self.formats
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A cell, quoted when it holds the delimiter, a quote or a line break.
fn quote(cell: &str, delimiter: char) -> String {
    if cell.contains([delimiter, '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// A text cell: with a quote mark in front when a spreadsheet could run it as a formula.
fn text_cell(cell: &str, delimiter: char) -> String {
    let danger = cell
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    if danger {
        quote(&format!("'{cell}"), delimiter)
    } else {
        quote(cell, delimiter)
    }
}

/// The value of a field for a line of the bill (`number` is the line from 1).
fn field_value(line: &BomLine, number: usize, field: &str, s: &AssemblerSettings) -> String {
    match field {
        "item" => number.to_string(),
        "references" => line.references.join(&s.references_separator.to_string()),
        "quantity" => line.qty.to_string(),
        "value" => line.value.clone(),
        "footprint" => line.footprint.clone(),
        "manufacturer" => line.manufacturer.clone(),
        "mpn" => line.mpn.clone(),
        "supplier" => line
            .purchase
            .as_ref()
            .map_or_else(String::new, |p| p.supplier.clone()),
        "supplier_code" => line
            .purchase
            .as_ref()
            .map_or_else(String::new, |p| p.supplier_code.clone()),
        "matched_code" => line
            .purchase
            .as_ref()
            .filter(|p| {
                s.code_supplier.is_empty() || p.supplier.eq_ignore_ascii_case(&s.code_supplier)
            })
            .map_or_else(String::new, |p| p.supplier_code.clone()),
        _ => String::new(),
    }
}

/// The lines of the bill as the CSV file of a format: a header line and one line for each line of
/// the bill, with `\n` line ends.
pub fn assembler_csv(
    lines: &[BomLine],
    format: &AssemblerFormat,
    settings: &AssemblerSettings,
) -> String {
    let d = settings.delimiter;
    let sep = d.to_string();
    let header: Vec<String> = format.columns.iter().map(|(h, _)| quote(h, d)).collect();
    let mut out = header.join(&sep);
    out.push('\n');
    for (index, line) in lines.iter().enumerate() {
        let row: Vec<String> = format
            .columns
            .iter()
            .map(|(_, field)| {
                let value = field_value(line, index + 1, field, settings);
                if NUMERIC_FIELDS.contains(&field.as_str()) {
                    quote(&value, d)
                } else {
                    text_cell(&value, d)
                }
            })
            .collect();
        out.push_str(&row.join(&sep));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bom::Purchase;

    fn line(
        refs: &[&str],
        value: &str,
        footprint: &str,
        supplier: Option<(&str, &str)>,
    ) -> BomLine {
        BomLine {
            value: value.to_owned(),
            footprint: footprint.to_owned(),
            manufacturer: "Acme".to_owned(),
            mpn: "AC-1".to_owned(),
            references: refs.iter().map(|r| (*r).to_owned()).collect(),
            qty: refs.len() as u64,
            needed: refs.len() as u64,
            purchase: supplier.map(|(s, c)| Purchase {
                supplier: s.to_owned(),
                supplier_code: c.to_owned(),
                tier_min_qty: 1,
                unit_price_micro: 1000,
                buy_qty: 1,
                cost_micro: 1000,
                tiers: vec![(1, 1000)],
            }),
            cost_per_board_micro: 0,
        }
    }

    #[test]
    fn the_shipped_file_holds_the_defaults() {
        let text = include_str!("../../../config/assembler.cfg");
        assert_eq!(
            AssemblerSettings::parse(text).unwrap(),
            AssemblerSettings::default()
        );
    }

    #[test]
    fn the_jlcpcb_format_writes_the_code_of_its_supplier_only() {
        let s = AssemblerSettings::default();
        let lines = vec![
            line(&["R1", "R2"], "10k", "R0603", Some(("LCSC", "C25804"))),
            line(&["C1"], "100n", "C0603", Some(("Other", "X1"))),
            line(&["U1"], "MCU", "QFN32", None),
        ];
        let csv = assembler_csv(&lines, s.format("jlcpcb").unwrap(), &s);
        assert_eq!(
            csv,
            "Comment,Designator,Footprint,LCSC Part #\n10k,\"R1,R2\",R0603,C25804\n100n,C1,C0603,\nMCU,U1,QFN32,\n"
        );
    }

    #[test]
    fn the_pcbway_format_numbers_the_lines() {
        let s = AssemblerSettings::default();
        let lines = vec![
            line(&["R1"], "10k", "R0603", None),
            line(&["C1", "C2"], "1u", "C0805", None),
        ];
        let csv = assembler_csv(&lines, s.format("pcbway").unwrap(), &s);
        let rows: Vec<&str> = csv.lines().collect();
        assert_eq!(
            rows[0],
            "Item #,Designator,Qty,Manufacturer,Mfg Part #,Description / Value,Package/Footprint"
        );
        assert_eq!(rows[1], "1,R1,1,Acme,AC-1,10k,R0603");
        assert_eq!(rows[2], "2,\"C1,C2\",2,Acme,AC-1,1u,C0805");
    }

    #[test]
    fn a_line_adds_a_format_and_the_separators_change() {
        let s = AssemblerSettings::parse(
            "format.mine = Ref=references|Qty=quantity|Note=blank\ndelimiter = semicolon\nreferences_separator = space\ncode_supplier =\n",
        )
        .unwrap();
        assert_eq!(s.formats.len(), 3);
        assert_eq!(s.delimiter, ';');
        assert_eq!(s.references_separator, ' ');
        assert!(s.code_supplier.is_empty());
        let csv = assembler_csv(
            &[line(&["R1", "R2"], "10k", "R0603", None)],
            s.format("mine").unwrap(),
            &s,
        );
        assert_eq!(csv, "Ref;Qty;Note\nR1 R2;2;\n");
        assert_eq!(s.names(), "jlcpcb, pcbway, mine");
    }

    #[test]
    fn a_line_replaces_the_format_of_the_same_name() {
        let s = AssemblerSettings::parse("format.jlcpcb = Value=value\n").unwrap();
        assert_eq!(s.formats.len(), 2);
        assert_eq!(s.format("jlcpcb").unwrap().columns.len(), 1);
    }

    #[test]
    fn a_cell_with_the_delimiter_is_quoted_and_a_formula_is_defused() {
        let s = AssemblerSettings {
            delimiter: ';',
            ..AssemblerSettings::default()
        };
        let csv = assembler_csv(
            &[line(&["R1"], "=1;2", "R0603", None)],
            s.format("jlcpcb").unwrap(),
            &s,
        );
        assert_eq!(
            csv,
            "Comment;Designator;Footprint;LCSC Part #\n\"'=1;2\";R1;R0603;\n"
        );
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("format.x\n", "line 1: not key = value"),
            ("\nfoo = 1\n", "line 2: unknown key foo"),
            ("format.X = A=value\n", "format.X"),
            ("format. = A=value\n", "format."),
            ("format.x = A=colour\n", "format.x"),
            ("format.x = A\n", "format.x"),
            ("format.x = =value\n", "format.x"),
            ("format.x = A=value|\n", "format.x"),
            ("delimiter = space\n", "delimiter"),
            ("delimiter = pipe\n", "delimiter"),
            ("references_separator = dot\n", "references_separator"),
        ] {
            let e = AssemblerSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn the_same_lines_always_give_the_same_bytes() {
        let s = AssemblerSettings::default();
        let lines = vec![line(&["R1"], "10k", "R0603", Some(("LCSC", "C1")))];
        let f = s.format("jlcpcb").unwrap();
        assert_eq!(assembler_csv(&lines, f, &s), assembler_csv(&lines, f, &s));
    }
}
