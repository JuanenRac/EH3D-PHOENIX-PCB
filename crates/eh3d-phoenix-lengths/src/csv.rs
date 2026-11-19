// SPDX-License-Identifier: GPL-3.0-or-later
//! The lengths of the nets as a CSV table, one row for each net in the order of the report, the
//! same bytes for the same board. A cell with a comma, a double quote or a line end is quoted, and
//! with the guard on a cell that a spreadsheet would read as a formula (it starts with `=`, `+`,
//! `-`, `@`, a tab or a carriage return) gets a single quote in front of it, so that a net called
//! `+5V` or `-12V` is read as a name. Nothing here reads or writes a file.

/// The header of the table.
pub const HEADER: &str = "net,length_nm,tracks_nm,vias,vias_nm,delay_fs,arcs";

/// What a row of the table holds: the numbers of one net.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvRow<'a> {
    /// The name of the net.
    pub name: &'a str,
    /// Tracks and vias together, in nanometres.
    pub length_nm: i64,
    /// The tracks, in nanometres.
    pub tracks_nm: i64,
    /// The number of vias.
    pub vias: usize,
    /// The length of the vias, in nanometres.
    pub vias_nm: i64,
    /// The delay of tracks and vias together, in femtoseconds.
    pub delay_fs: i64,
    /// How many of its pieces are arcs.
    pub arcs: usize,
}

/// One cell: quoted when it has to be and, with `guard`, safe from being read as a formula.
pub fn csv_field(value: &str, guard: bool) -> String {
    let mut text = value.to_owned();
    if guard && text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        text.insert(0, '\'');
    }
    if text.contains([',', '"', '\n', '\r']) {
        text = format!("\"{}\"", text.replace('"', "\"\""));
    }
    text
}

/// The table: the header and a row for each net, every line ended by a line feed.
pub fn nets_csv(rows: &[CsvRow<'_>], guard: bool) -> String {
    let mut text = String::from(HEADER);
    text.push('\n');
    for row in rows {
        text.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_field(row.name, guard),
            row.length_nm,
            row.tracks_nm,
            row.vias,
            row.vias_nm,
            row.delay_fs,
            row.arcs
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, length_nm: i64) -> CsvRow<'_> {
        CsvRow {
            name,
            length_nm,
            tracks_nm: length_nm - 1_600_000,
            vias: 1,
            vias_nm: 1_600_000,
            delay_fs: length_nm * 6,
            arcs: 0,
        }
    }

    #[test]
    fn the_table_has_the_header_and_a_row_for_each_net() {
        let text = nets_csv(&[row("GND", 10_000_000), row("SIG", 5_000_000)], true);
        assert_eq!(
            text,
            "net,length_nm,tracks_nm,vias,vias_nm,delay_fs,arcs\n\
             GND,10000000,8400000,1,1600000,60000000,0\n\
             SIG,5000000,3400000,1,1600000,30000000,0\n"
        );
        assert_eq!(nets_csv(&[], true), format!("{HEADER}\n"));
    }

    #[test]
    fn a_name_that_a_spreadsheet_would_read_as_a_formula_is_guarded_unless_asked_not_to() {
        for name in ["=SUM(A1)", "+5V", "-12V", "@x", "\tx", "\rx"] {
            let guarded = csv_field(name, true);
            assert!(
                guarded.starts_with('\'') || guarded.starts_with("\"'"),
                "{name:?} gives {guarded:?}"
            );
            assert!(!csv_field(name, false).starts_with('\''), "{name:?}");
        }
        assert_eq!(csv_field("GND", true), "GND");
        assert_eq!(csv_field("a-b+c", true), "a-b+c");
        assert_eq!(csv_field("+5V", true), "'+5V");
        assert_eq!(csv_field("+5V", false), "+5V");
    }

    #[test]
    fn a_name_with_a_comma_a_quote_or_a_line_end_is_quoted() {
        assert_eq!(csv_field("a,b", true), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\"", true), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("a\nb", true), "\"a\nb\"");
        // a name that is both: the quote goes inside the quotes
        assert_eq!(csv_field("=a,b", true), "\"'=a,b\"");
    }

    /// A small linear congruential generator, so the cases are the same on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// Reads a table back: the lines, and the cells of each line with the quotes taken off.
    fn read_back(text: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut cell = String::new();
        let mut quoted = false;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, quoted) {
                ('"', true) if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                ('"', _) => quoted = !quoted,
                (',', false) => row.push(std::mem::take(&mut cell)),
                ('\n', false) => {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                (c, _) => cell.push(c),
            }
        }
        rows
    }

    #[test]
    fn generated_names_are_read_back_with_the_guard_as_the_only_change() {
        let alphabet: Vec<char> = "ab1,\"+-=@ _/\n".chars().collect();
        let mut rng = Lcg(0x1e47);
        for _ in 0..80 {
            let count = 1 + (rng.next() % 4) as usize;
            let names: Vec<String> = (0..count)
                .map(|_| {
                    let len = 1 + (rng.next() % 6) as usize;
                    (0..len)
                        .map(|_| alphabet[(rng.next() % alphabet.len() as u64) as usize])
                        .collect()
                })
                .collect();
            let rows: Vec<CsvRow<'_>> = names.iter().map(|n| row(n, 2_000_000)).collect();
            let text = nets_csv(&rows, true);
            let back = read_back(&text);
            assert_eq!(back.len(), count + 1, "{text:?}");
            for (name, cells) in names.iter().zip(&back[1..]) {
                assert_eq!(cells.len(), 7, "{text:?}");
                let expected = if name.starts_with(['=', '+', '-', '@', '\t', '\r']) {
                    format!("'{name}")
                } else {
                    name.clone()
                };
                assert_eq!(cells[0], expected, "{text:?}");
            }
            // without the guard the names come back as they are
            let plain = read_back(&nets_csv(&rows, false));
            for (name, cells) in names.iter().zip(&plain[1..]) {
                assert_eq!(&cells[0], name);
            }
        }
    }
}
