// SPDX-License-Identifier: GPL-3.0-or-later
//! The formats of a bill of materials that [`import_as`] reads: the generic CSV with the map of
//! columns of `config/bom_import.cfg`, the partlist that EAGLE writes (text in columns of a fixed
//! width), and the two bills of KiCad (a CSV and an XML file).
//!
//! Each reader turns its file into a table of columns and hands it to the same step that reads a
//! generic CSV, so the ranges of references (`R1-R5`), the quantities, the parts that are not
//! fitted and the findings behave in the same way for every format. The names of the columns of
//! the EAGLE and KiCad readers are fixed (see the presets below); only the generic CSV takes the
//! names of the map. The limits, the separators of references and the cells that mean "not fitted"
//! come from the map in every case.

use crate::codes::{BomError, PARTS_INVALID};
use crate::import::{bill_from_table, import_bill, ImportMap, ImportedBill};
use crate::text::{parse_table_with, Table};

/// The most levels of nesting an XML bill may have.
const MAX_XML_DEPTH: usize = 64;

/// The format of a bill to import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// Look at the file and choose.
    Auto,
    /// A CSV file read with the names of the map.
    Csv,
    /// The partlist of EAGLE.
    Eagle,
    /// The CSV bill of KiCad.
    KicadCsv,
    /// The XML bill of KiCad.
    KicadXml,
}

impl ImportFormat {
    /// The names `--import-format` accepts.
    pub const NAMES: &'static str = "auto, csv, eagle, kicad-csv, kicad-xml";

    /// The format of a name (without regard to case); `None` for a name that is not one of them.
    pub fn parse(name: &str) -> Option<ImportFormat> {
        match name.to_ascii_lowercase().as_str() {
            "auto" => Some(ImportFormat::Auto),
            "csv" => Some(ImportFormat::Csv),
            "eagle" => Some(ImportFormat::Eagle),
            "kicad-csv" => Some(ImportFormat::KicadCsv),
            "kicad-xml" => Some(ImportFormat::KicadXml),
            _ => None,
        }
    }
}

/// The format a text looks like: the partlist of EAGLE starts with its title line (or with the
/// header line that starts with `Part`), an XML file starts with its declaration or the element
/// `export` of KiCad, and every other text is taken for a CSV file. A KiCad CSV file is not told
/// from another CSV file here; ask for `kicad-csv` when its delimiter is not a comma.
pub fn detect(text: &str) -> ImportFormat {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text).trim_start();
    if t.starts_with("Partlist exported from") {
        return ImportFormat::Eagle;
    }
    if t.starts_with("<?xml") || t.starts_with("<export") {
        return ImportFormat::KicadXml;
    }
    let first = t.lines().next().unwrap_or("");
    let mut words = first.split_whitespace();
    let part_value = words.next().is_some_and(|w| w.eq_ignore_ascii_case("part"))
        && words
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("value"));
    if part_value && !first.contains(',') && !first.contains(';') {
        return ImportFormat::Eagle;
    }
    ImportFormat::Csv
}

/// Reads a bill in a format. `Auto` looks at the text with [`detect`].
pub fn import_as(
    text: &str,
    format: ImportFormat,
    map: &ImportMap,
) -> Result<ImportedBill, BomError> {
    let format = if format == ImportFormat::Auto {
        detect(text)
    } else {
        format
    };
    match format {
        ImportFormat::Auto | ImportFormat::Csv => import_bill(text, map),
        ImportFormat::Eagle => {
            let table = eagle_table(text)?;
            bill_from_table(&table, &preset(map, '\t', EAGLE_NAMES))
        }
        ImportFormat::KicadCsv => {
            let (table, delimiter) = kicad_csv_table(text)?;
            bill_from_table(&table, &preset(map, delimiter, KICAD_NAMES))
        }
        ImportFormat::KicadXml => {
            let table = kicad_xml_table(text, map)?;
            bill_from_table(&table, &preset(map, ',', XML_NAMES))
        }
    }
}

/// The names of the columns of a format, in this order: references, value, footprint, maker, part
/// number, quantity, not to be fitted.
type Names = [&'static [&'static str]; 7];

const EAGLE_NAMES: Names = [
    &["part"],
    &["value"],
    &["package"],
    &["mf", "manuf", "manufacturer"],
    &["mpn", "mp"],
    &[],
    &["dnp"],
];

const KICAD_NAMES: Names = [
    &[
        "reference",
        "references",
        "refs",
        "reference(s)",
        "designator",
    ],
    &["value"],
    &["footprint"],
    &["manufacturer", "mfr", "mf"],
    &["mpn", "manufacturer part number", "mfr part number"],
    &["qty", "quantity"],
    &["dnp"],
];

const XML_NAMES: Names = [
    &["references"],
    &["value"],
    &["footprint"],
    &["manufacturer"],
    &["mpn"],
    &[],
    &["dnp"],
];

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// The map with the names and the delimiter of a format and everything else of the user's map.
fn preset(map: &ImportMap, delimiter: char, columns: Names) -> ImportMap {
    ImportMap {
        delimiter,
        references: names(columns[0]),
        value: names(columns[1]),
        footprint: names(columns[2]),
        manufacturer: names(columns[3]),
        mpn: names(columns[4]),
        quantity: names(columns[5]),
        fitted: Vec::new(),
        dnp: names(columns[6]),
        ..map.clone()
    }
}

fn bad(why: impl Into<String>) -> BomError {
    BomError::new(PARTS_INVALID, why.into())
}

// ---------------------------------------------------------------------------------------------
// EAGLE
// ---------------------------------------------------------------------------------------------

/// Reads the partlist of EAGLE: a title line, a blank line, a header line that starts with `Part`
/// and one line for each part, every cell starting where its column name starts in the header.
fn eagle_table(text: &str) -> Result<Table, BomError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines().enumerate();
    let mut header_line = None;
    for (_, line) in lines.by_ref() {
        let first = line.split_whitespace().next();
        if first.is_some_and(|w| w.eq_ignore_ascii_case("part")) {
            header_line = Some(line);
            break;
        }
    }
    let header_line = header_line
        .ok_or_else(|| bad("no header line that starts with Part (an EAGLE partlist)"))?;
    // the column starts, in characters, with the lower case name of each column
    let mut starts: Vec<(usize, String)> = Vec::new();
    let mut in_word = false;
    for (at, c) in header_line.chars().enumerate() {
        if c.is_whitespace() {
            in_word = false;
            continue;
        }
        if !in_word {
            in_word = true;
            starts.push((at, String::new()));
        }
        if let Some(last) = starts.last_mut() {
            last.1.extend(c.to_lowercase());
        }
    }
    let mut rows = Vec::new();
    for (index, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut cells = Vec::with_capacity(starts.len());
        for (k, (from, _)) in starts.iter().enumerate() {
            let to = starts.get(k + 1).map_or(chars.len(), |next| next.0);
            let to = to.min(chars.len());
            let cell: String = if *from < to {
                chars[*from..to].iter().collect()
            } else {
                String::new()
            };
            cells.push(cell.trim().to_owned());
        }
        if cells.first().is_none_or(|part| part.is_empty()) {
            continue;
        }
        rows.push((index + 1, cells));
    }
    Ok(Table {
        header: starts.into_iter().map(|(_, name)| name).collect(),
        rows,
    })
}

// ---------------------------------------------------------------------------------------------
// KiCad CSV
// ---------------------------------------------------------------------------------------------

/// The delimiter a line uses: the one of comma, semicolon and tab that is most often outside
/// quotes, the comma on a tie.
fn guess_delimiter(line: &str) -> char {
    let mut counts = [0usize; 3];
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => counts[0] += 1,
            ';' if !quoted => counts[1] += 1,
            '\t' if !quoted => counts[2] += 1,
            _ => {}
        }
    }
    let mut best = 0;
    for k in 1..3 {
        if counts[k] > counts[best] {
            best = k;
        }
    }
    [',', ';', '\t'][best]
}

/// Reads the CSV bill of KiCad. The lines before the one with the column of the references (the
/// block of information that an older export writes first) are left out, keeping the numbers of
/// the lines.
fn kicad_csv_table(text: &str) -> Result<(Table, char), BomError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut kept = String::with_capacity(text.len());
    let mut started = false;
    let mut delimiter = ',';
    for line in text.split_inclusive('\n') {
        if !started {
            let lower = line.to_ascii_lowercase();
            if lower.contains("reference") || lower.contains("refs") || lower.contains("designator")
            {
                started = true;
                delimiter = guess_delimiter(line);
            } else {
                kept.push('\n');
                continue;
            }
        }
        kept.push_str(line);
    }
    let table = parse_table_with(&kept, delimiter).map_err(bad)?;
    Ok((table, delimiter))
}

// ---------------------------------------------------------------------------------------------
// KiCad XML
// ---------------------------------------------------------------------------------------------

/// One component of the XML bill while it is read.
#[derive(Default)]
struct Comp {
    reference: String,
    value: String,
    footprint: String,
    fields: Vec<(String, String)>,
    fitted: bool,
}

/// Decodes the five named entities and the numeric ones; any other `&` stays as it is.
fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let replaced = rest.find(';').and_then(|end| {
            let entity = &rest[1..end];
            let c = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => entity.strip_prefix('#').and_then(|n| {
                    let code = match n.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => n.parse().ok(),
                    };
                    code.and_then(char::from_u32)
                }),
            };
            c.map(|c| (c, end))
        });
        match replaced {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The place of the `>` that ends the tag the text starts with, leaving out the ones in quotes.
fn tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (at, c) in s.char_indices() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                } else if c == '>' {
                    return Some(at);
                }
            }
        }
    }
    None
}

/// The attributes of a tag, from the text after its name.
fn attributes(text: &str) -> Result<Vec<(String, String)>, BomError> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() && rest != "/" {
        let eq = rest
            .find('=')
            .ok_or_else(|| bad(format!("an attribute without a value in <{text}>")))?;
        let name = rest[..eq].trim().to_owned();
        let after = rest[eq + 1..].trim_start();
        let quote = after
            .chars()
            .next()
            .filter(|c| *c == '"' || *c == '\'')
            .ok_or_else(|| bad(format!("the value of {name} is not in quotes")))?;
        let body = &after[1..];
        let close = body
            .find(quote)
            .ok_or_else(|| bad(format!("the value of {name} is not closed")))?;
        out.push((name, decode(&body[..close])));
        rest = body[close + 1..].trim_start();
    }
    Ok(out)
}

fn attribute<'a>(list: &'a [(String, String)], name: &str) -> Option<&'a str> {
    list.iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

/// The first non-empty field of a component whose name is one of `wanted` (without regard to
/// case).
fn field_of(comp: &Comp, wanted: &[&str]) -> String {
    comp.fields
        .iter()
        .filter(|(name, value)| {
            !value.is_empty() && wanted.iter().any(|w| name.eq_ignore_ascii_case(w))
        })
        .map(|(_, value)| value.clone())
        .next()
        .unwrap_or_default()
}

/// Reads the XML bill of KiCad: the elements `comp` inside `components`, with the attribute `ref`,
/// the elements `value` and `footprint`, the `field` elements inside `fields` (the maker and the
/// part number are fields) and the `property` elements named `dnp` or `exclude_from_bom`, which
/// make the part not fitted. Nothing is fetched and no entity of the file is expanded.
fn kicad_xml_table(text: &str, map: &ImportMap) -> Result<Table, BomError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.contains("<!ENTITY") {
        return Err(bad("entity declarations are not read"));
    }
    let mut stack: Vec<String> = Vec::new();
    let mut comps: Vec<Comp> = Vec::new();
    let mut current: Option<Comp> = None;
    let mut field_name = String::new();
    let mut seen_components = false;
    let mut buf = String::new();
    let mut rest = text;
    while let Some(lt) = rest.find('<') {
        buf.push_str(&rest[..lt]);
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            let end = rest
                .find("-->")
                .ok_or_else(|| bad("a comment is not closed"))?;
            rest = &rest[end + 3..];
            continue;
        }
        if let Some(body) = rest.strip_prefix("<![CDATA[") {
            let end = body
                .find("]]>")
                .ok_or_else(|| bad("a CDATA section is not closed"))?;
            buf.push_str(&body[..end]);
            rest = &body[end + 3..];
            continue;
        }
        if rest.starts_with("<?") || rest.starts_with("<!") {
            let end = rest
                .find('>')
                .ok_or_else(|| bad("a declaration is not closed"))?;
            rest = &rest[end + 1..];
            continue;
        }
        let end = tag_end(rest).ok_or_else(|| bad("a tag is not closed"))?;
        let inner = &rest[1..end];
        rest = &rest[end + 1..];
        if let Some(closing) = inner.strip_prefix('/') {
            let name = closing.trim();
            if stack.pop().as_deref() != Some(name) {
                return Err(bad(format!("the end tag {name} does not match its start")));
            }
            let parent = stack.last().map(String::as_str).unwrap_or("");
            let value = decode(buf.trim());
            match (name, parent) {
                ("comp", "components") => {
                    comps.extend(current.take());
                }
                ("value", "comp") => {
                    if let Some(c) = current.as_mut() {
                        c.value = value;
                    }
                }
                ("footprint", "comp") => {
                    if let Some(c) = current.as_mut() {
                        c.footprint = value;
                    }
                }
                ("field", "fields") => {
                    if let Some(c) = current.as_mut() {
                        c.fields.push((std::mem::take(&mut field_name), value));
                    }
                }
                _ => {}
            }
            buf.clear();
        } else {
            buf.clear();
            let name_end = inner
                .find(|c: char| c.is_whitespace() || c == '/')
                .unwrap_or(inner.len());
            let name = &inner[..name_end];
            let attrs = attributes(&inner[name_end..])?;
            let parent = stack.last().map(String::as_str).unwrap_or("");
            let self_closing = inner.trim_end().ends_with('/');
            match (name, parent) {
                ("components", _) => seen_components = true,
                ("comp", "components") => {
                    let reference = attribute(&attrs, "ref")
                        .ok_or_else(|| bad("a comp has no ref attribute"))?;
                    current = Some(Comp {
                        reference: reference.to_owned(),
                        fitted: true,
                        ..Comp::default()
                    });
                }
                ("property", "comp") => {
                    let hides = attribute(&attrs, "name")
                        .is_some_and(|n| matches!(n, "dnp" | "exclude_from_bom"));
                    if hides {
                        if let Some(c) = current.as_mut() {
                            c.fitted = false;
                        }
                    }
                }
                ("field", "fields") => {
                    field_name = attribute(&attrs, "name").unwrap_or("").to_owned();
                }
                _ => {}
            }
            if self_closing {
                if name == "comp" && parent == "components" {
                    comps.extend(current.take());
                }
            } else {
                if stack.len() >= MAX_XML_DEPTH {
                    return Err(bad("the elements are nested too deep"));
                }
                stack.push(name.to_owned());
            }
        }
        if comps.len() > map.max_rows {
            return Err(bad(format!("more than {} lines", map.max_rows)));
        }
    }
    if !stack.is_empty() {
        return Err(bad("an element is not closed"));
    }
    if !seen_components {
        return Err(bad("no element components (a KiCad XML bill)"));
    }
    let dnp_cell = map
        .dnp_values
        .first()
        .cloned()
        .unwrap_or_else(|| "yes".to_owned());
    let rows = comps
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let dnp = if c.fitted {
                String::new()
            } else {
                dnp_cell.clone()
            };
            (
                k + 1,
                vec![
                    c.reference.clone(),
                    c.value.clone(),
                    c.footprint.clone(),
                    field_of(c, &["manufacturer", "mfr", "mf", "manuf"]),
                    field_of(c, &["mpn", "manufacturer part number", "mfr part number"]),
                    dnp,
                ],
            )
        })
        .collect();
    Ok(Table {
        header: names(&[
            "references",
            "value",
            "footprint",
            "manufacturer",
            "mpn",
            "dnp",
        ]),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // A partlist of the shape EAGLE writes, made up for this test: the cells start where the
    // names of the header start, a value may be empty, and a column may repeat a name.
    const EAGLE: &str = "Partlist exported from some/path/board.sch at 01/01/2000 00:00\r\n\
\r\n\
Part  Value  Device      Package   Description  MF     MPN        PACKAGE\r\n\
C1    100nF  C-EUC0402   C0402     CAPACITOR                      X7R\r\n\
C2    100nF  C-EUC0402   C0402     CAPACITOR                      X7R\r\n\
J1           HDR-1X4     HDR1X4    Header 4     MOLEX  22-23-2021\r\n\
U1    74HC   SOIC16      SO16      Mux          NXP    74HC4052D  SOIC-16\r\n\
\r\n";

    #[test]
    fn the_formats_are_named_and_told_apart() {
        assert_eq!(
            ImportFormat::parse("Kicad-XML"),
            Some(ImportFormat::KicadXml)
        );
        assert_eq!(ImportFormat::parse("eagle"), Some(ImportFormat::Eagle));
        assert_eq!(ImportFormat::parse("excel"), None);
        assert_eq!(detect(EAGLE), ImportFormat::Eagle);
        assert_eq!(detect("Part Value Device\nC1 1n x"), ImportFormat::Eagle);
        assert_eq!(detect("part,value\nC1,1n"), ImportFormat::Csv);
        assert_eq!(
            detect("\u{feff}<?xml version=\"1.0\"?><export/>"),
            ImportFormat::KicadXml
        );
        assert_eq!(detect("<export version=\"D\">"), ImportFormat::KicadXml);
        assert_eq!(detect("references,value\nR1,10k\n"), ImportFormat::Csv);
        assert_eq!(detect(""), ImportFormat::Csv);
    }

    #[test]
    fn an_eagle_partlist_is_read_by_the_columns_of_its_header() {
        let bill = import_as(EAGLE, ImportFormat::Auto, &ImportMap::default()).unwrap();
        assert!(bill.findings.is_empty(), "{:?}", bill.findings);
        let lines: Vec<(Vec<String>, &str, &str, &str, &str)> = bill
            .lines
            .iter()
            .map(|l| {
                (
                    l.references.clone(),
                    l.value.as_str(),
                    l.footprint.as_str(),
                    l.manufacturer.as_str(),
                    l.mpn.as_str(),
                )
            })
            .collect();
        assert_eq!(
            lines,
            [
                (vec!["C1".to_owned()], "100nF", "C0402", "", ""),
                (vec!["C2".to_owned()], "100nF", "C0402", "", ""),
                (vec!["J1".to_owned()], "", "HDR1X4", "MOLEX", "22-23-2021"),
                (vec!["U1".to_owned()], "74HC", "SO16", "NXP", "74HC4052D"),
            ]
        );
        // the lines of the file are counted from 1, the header being the third
        assert_eq!(bill.lines[0].line, 4);
        assert!(bill.lines.iter().all(|l| l.fitted));
        assert_eq!(
            bill.parts_csv(),
            "match,manufacturer,mpn,fitted\nJ1,MOLEX,22-23-2021,yes\nU1,NXP,74HC4052D,yes\n"
        );
    }

    #[test]
    fn an_eagle_partlist_takes_the_first_maker_column_that_has_a_cell() {
        let text = "Part  Value  Device  Package  MANUF     MF     MP       MPN\n\
R1    1k     R       R0603    Coilcraft                 ABC-1\n\
R2    2k     R       R0603              Diodes SMBJ33A  XYZ\n";
        let bill = import_as(text, ImportFormat::Eagle, &ImportMap::default()).unwrap();
        let made: Vec<(&str, &str)> = bill
            .lines
            .iter()
            .map(|l| (l.manufacturer.as_str(), l.mpn.as_str()))
            .collect();
        assert_eq!(made, [("Coilcraft", "ABC-1"), ("Diodes", "XYZ")]);
    }

    #[test]
    fn an_eagle_text_without_the_header_or_with_a_bad_range_is_refused() {
        let e = import_as(
            "Partlist exported from x\n\nR1 1k\n",
            ImportFormat::Eagle,
            &ImportMap::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, PARTS_INVALID);
        assert!(e.detail.contains("header line"), "{}", e.detail);
        // the lines that are blank, or have no part name, are left out
        let text = "Part Value\n\n      orphan\nR1   1k\n";
        let bill = import_as(text, ImportFormat::Eagle, &ImportMap::default()).unwrap();
        assert_eq!(bill.lines.len(), 1);
        assert_eq!(bill.lines[0].references, ["R1"]);
    }

    #[test]
    fn a_kicad_csv_with_commas_is_read_with_its_own_names() {
        let text = "\"Refs\",\"Value\",\"Footprint\",\"Qty\",\"DNP\"\n\
\"R1, R2\",\"10k\",\"Resistor_SMD:R_0603\",\"2\",\"\"\n\
\"C1-C3\",\"100n\",\"Capacitor_SMD:C_0603\",\"3\",\"\"\n\
\"TP1\",\"\",\"TestPoint:TP\",\"1\",\"DNP\"\n";
        let bill = import_as(text, ImportFormat::KicadCsv, &ImportMap::default()).unwrap();
        assert!(bill.findings.is_empty(), "{:?}", bill.findings);
        assert_eq!(bill.lines[0].references, ["R1", "R2"]);
        assert_eq!(bill.lines[0].footprint, "Resistor_SMD:R_0603");
        assert_eq!(bill.lines[1].references, ["C1", "C2", "C3"]);
        assert_eq!(bill.lines[1].quantity, Some(3));
        assert!(!bill.lines[2].fitted);
    }

    #[test]
    fn a_kicad_csv_with_semicolons_and_a_block_of_information_before_it() {
        let text = "Source: some.sch\nDate: none\n\nItem;Qty;Reference(s);Value;Footprint\n\
1;2;R1 R2;4k7;R_0603\n2;1;C4;1u;C_0603\n";
        let bill = import_as(text, ImportFormat::KicadCsv, &ImportMap::default()).unwrap();
        assert_eq!(bill.lines.len(), 2);
        assert_eq!(bill.lines[0].references, ["R1", "R2"]);
        assert_eq!(bill.lines[0].value, "4k7");
        assert_eq!(bill.lines[1].references, ["C4"]);
        // the lines keep their numbers in the file: the block is four lines
        assert_eq!(bill.lines[0].line, 5);
        assert_eq!(guess_delimiter("a\tb\tc,d"), '\t');
        assert_eq!(guess_delimiter("\"a;b;c\",d"), ',');
        assert_eq!(guess_delimiter("abc"), ',');
    }

    const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<export version=\"D\">\n\
  <design><source>some.sch</source></design>\n\
  <!-- a comment with <tags> inside -->\n\
  <components>\n\
    <comp ref=\"R1\">\n\
      <value>10k</value>\n\
      <footprint>Resistor_SMD:R_0603</footprint>\n\
      <fields><field name=\"Manufacturer\">Acme &amp; Sons</field><field name=\"MPN\">AC-10K</field></fields>\n\
    </comp>\n\
    <comp ref=\"C1\"><value><![CDATA[100n<F>]]></value><property name=\"dnp\"/></comp>\n\
    <comp ref='J1'><value>&#x48;dr</value><fields><field name=\"MPN\"/></fields><property name=\"Sheetname\" value=\"x\"/></comp>\n\
    <comp ref=\"C2\"/>\n\
  </components>\n\
</export>\n";

    #[test]
    fn a_kicad_xml_bill_is_read_with_its_fields_and_its_dnp_property() {
        let bill = import_as(XML, ImportFormat::Auto, &ImportMap::default()).unwrap();
        assert!(bill.findings.is_empty(), "{:?}", bill.findings);
        let made: Vec<(String, String, String, String, String, bool)> = bill
            .lines
            .iter()
            .map(|l| {
                (
                    l.references.join(" "),
                    l.value.clone(),
                    l.footprint.clone(),
                    l.manufacturer.clone(),
                    l.mpn.clone(),
                    l.fitted,
                )
            })
            .collect();
        let row = |r: &str, v: &str, f: &str, m: &str, p: &str, fit: bool| {
            (
                r.to_owned(),
                v.to_owned(),
                f.to_owned(),
                m.to_owned(),
                p.to_owned(),
                fit,
            )
        };
        assert_eq!(
            made,
            [
                row(
                    "R1",
                    "10k",
                    "Resistor_SMD:R_0603",
                    "Acme & Sons",
                    "AC-10K",
                    true
                ),
                row("C1", "100n<F>", "", "", "", false),
                row("J1", "Hdr", "", "", "", true),
                row("C2", "", "", "", "", true),
            ]
        );
        assert_eq!(
            bill.parts_csv(),
            "match,manufacturer,mpn,fitted\nC1,,,no\nR1,Acme & Sons,AC-10K,yes\n"
        );
    }

    #[test]
    fn a_kicad_xml_that_is_not_one_is_refused_with_the_reason() {
        for (text, needle) in [
            (
                "<export><components><comp ref=\"R1\"></components></export>",
                "does not match",
            ),
            ("<export><components><comp ref=\"R1\">", "not closed"),
            (
                "<export><components><comp></comp></components></export>",
                "no ref",
            ),
            ("<export><design/></export>", "no element components"),
            ("<!DOCTYPE x [<!ENTITY a \"b\">]><export/>", "entity"),
            (
                "<export><components x=y></components></export>",
                "not in quotes",
            ),
            (
                "<export><components x></components></export>",
                "without a value",
            ),
            ("<export><!-- open", "comment"),
            ("<export><components", "tag is not closed"),
        ] {
            let e = import_as(text, ImportFormat::KicadXml, &ImportMap::default()).unwrap_err();
            assert_eq!(e.code, PARTS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let deep = format!("{}{}", "<a>".repeat(70), "</a>".repeat(70));
        let e = import_as(&deep, ImportFormat::KicadXml, &ImportMap::default()).unwrap_err();
        assert!(e.detail.contains("too deep"), "{}", e.detail);
        let limited = ImportMap::parse("max_rows = 1\n").unwrap();
        let two = "<export><components><comp ref=\"A1\"/><comp ref=\"A2\"/></components></export>";
        assert!(import_as(two, ImportFormat::KicadXml, &limited).is_err());
    }

    #[test]
    fn the_entities_are_decoded_and_the_rest_of_an_ampersand_is_kept() {
        assert_eq!(
            decode("a &lt;b&gt; &quot;c&quot; &apos;d&apos; &amp;"),
            "a <b> \"c\" 'd' &"
        );
        assert_eq!(decode("&#65;&#x42;&#X43;"), "ABC");
        assert_eq!(decode("AT&T; &unknown; & x"), "AT&T; &unknown; & x");
        assert_eq!(decode("&#1114112;"), "&#1114112;");
        assert_eq!(decode("no entity"), "no entity");
    }
}
