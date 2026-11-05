// SPDX-License-Identifier: GPL-3.0-or-later
//! The data of the parts that the board and the schematic do not hold: the maker, the part number
//! and whether the part is fitted. A CSV file with a header line and the columns `match`,
//! `manufacturer`, `mpn` and `fitted`. `match` is a reference (`R1`) or a value and a footprint
//! joined by a bar (`10k|R0603`); the line of a reference wins over the line of a value and a
//! footprint. `fitted` is `yes` (also when empty) or `no`.

use std::collections::BTreeMap;

use crate::codes::{BomError, PARTS_INVALID};
use crate::text::parse_table;

/// What the file says about one match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartData {
    /// The maker; empty when the file does not say.
    pub manufacturer: String,
    /// The part number of the maker; empty when the file does not say.
    pub mpn: String,
    /// Whether the part is fitted.
    pub fitted: bool,
}

/// The lines of a file of part data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartsFile {
    /// By reference.
    pub by_reference: BTreeMap<String, PartData>,
    /// By value and footprint.
    pub by_kind: BTreeMap<(String, String), PartData>,
}

fn invalid(what: String) -> BomError {
    BomError::new(PARTS_INVALID, what)
}

impl PartsFile {
    /// Reads the file, with no more than `max_rows` lines.
    pub fn parse(text: &str, max_rows: usize) -> Result<PartsFile, BomError> {
        let table = parse_table(text).map_err(invalid)?;
        let key = table
            .column("match")
            .ok_or_else(|| invalid("the column match is missing".to_owned()))?;
        if table.rows.len() > max_rows {
            return Err(invalid(format!("more than {max_rows} lines")));
        }
        let mut file = PartsFile::default();
        for (line, cells) in &table.rows {
            let get = |name: &str| -> String {
                table
                    .column(name)
                    .and_then(|i| cells.get(i))
                    .cloned()
                    .unwrap_or_default()
            };
            let matched = cells.get(key).cloned().unwrap_or_default();
            if matched.is_empty() {
                return Err(invalid(format!("line {line}: match is empty")));
            }
            let fitted = match get("fitted").to_ascii_lowercase().as_str() {
                "" | "yes" | "y" | "true" | "1" => true,
                "no" | "n" | "false" | "0" | "dnp" => false,
                other => {
                    return Err(invalid(format!(
                        "line {line}: fitted is {other}, not yes or no"
                    )))
                }
            };
            let data = PartData {
                manufacturer: get("manufacturer"),
                mpn: get("mpn"),
                fitted,
            };
            let duplicate = match matched.split_once('|') {
                Some((value, footprint)) => file
                    .by_kind
                    .insert((value.trim().to_owned(), footprint.trim().to_owned()), data)
                    .is_some(),
                None => file.by_reference.insert(matched, data).is_some(),
            };
            if duplicate {
                return Err(invalid(format!("line {line}: the match is given twice")));
            }
        }
        Ok(file)
    }

    /// What the file says about a part: its reference first, then its value and footprint.
    pub fn find(&self, reference: &str, value: &str, footprint: &str) -> Option<&PartData> {
        self.by_reference
            .get(reference)
            .or_else(|| self.by_kind.get(&(value.to_owned(), footprint.to_owned())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "match,manufacturer,mpn,fitted\n\
R1,Acme,AC-R1,\n\
10k|R0603,Yageo,RC0603,yes\n\
C9,,,no\n";

    #[test]
    fn the_lines_are_read_and_a_reference_wins_over_a_kind() {
        let f = PartsFile::parse(FILE, 10).unwrap();
        assert_eq!(f.by_reference.len(), 2);
        assert_eq!(f.by_kind.len(), 1);
        let r1 = f.find("R1", "10k", "R0603").unwrap();
        assert_eq!(r1.mpn, "AC-R1");
        let r2 = f.find("R2", "10k", "R0603").unwrap();
        assert_eq!(r2.manufacturer, "Yageo");
        assert!(r2.fitted);
        assert!(!f.find("C9", "1u", "C0603").unwrap().fitted);
        assert!(f.find("R3", "22k", "R0603").is_none());
    }

    #[test]
    fn what_is_wrong_is_refused() {
        for (text, needle) in [
            ("mpn\nX\n", "match is missing"),
            ("match,fitted\nR1,maybe\n", "not yes or no"),
            ("match,mpn\n,X\n", "match is empty"),
            ("match,mpn\nR1,X\nR1,Y\n", "given twice"),
            ("match,mpn\n10k|R0603,X\n10k | R0603,Y\n", "given twice"),
        ] {
            let e = PartsFile::parse(text, 10).unwrap_err();
            assert_eq!(e.code, PARTS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        assert!(PartsFile::parse(FILE, 2).is_err());
    }
}
