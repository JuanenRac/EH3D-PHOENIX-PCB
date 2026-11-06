// SPDX-License-Identifier: GPL-3.0-or-later
//! Plain text helpers: a reader of CSV with a header line, the quoting of a CSV cell, and the
//! escaping of HTML.

/// A table read from CSV: the names of the columns (in lower case) and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The names of the columns, trimmed and in lower case.
    pub header: Vec<String>,
    /// The rows, each with the cells it has, and the line of the file it came from.
    pub rows: Vec<(usize, Vec<String>)>,
}

impl Table {
    /// The place of a column by name.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
}

/// Reads CSV text: commas, cells that may be quoted with `"` (a doubled quote is a quote),
/// blank lines and lines starting with `#` skipped, the first line the header. The error is the
/// line of the file and what is wrong with it.
pub fn parse_table(text: &str) -> Result<Table, String> {
    parse_table_with(text, ',')
}

/// Reads CSV text like [`parse_table`] with another delimiter between the cells (a semicolon or a
/// tab, as other programs write). A quote or a line break cannot be the delimiter.
pub fn parse_table_with(text: &str, delimiter: char) -> Result<Table, String> {
    if matches!(delimiter, '"' | '\n' | '\r') {
        return Err("the delimiter cannot be a quote or a line break".to_owned());
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut records: Vec<(usize, Vec<String>)> = Vec::new();
    let mut cell = String::new();
    let mut record: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut quoted = false;
    let mut line = 1usize;
    let mut start_line = 1usize;
    let mut chars = text.chars().peekable();
    let mut pending = false;
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        cell.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                '\n' => {
                    line += 1;
                    cell.push('\n');
                }
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() && !quoted => {
                in_quotes = true;
                quoted = true;
                pending = true;
            }
            '"' => {
                return Err(format!(
                    "line {line}: a quote inside a cell that is not quoted"
                ))
            }
            c if c == delimiter => {
                record.push(finish_cell(&mut cell, &mut quoted));
                pending = true;
            }
            '\r' => {}
            '\n' => {
                if pending || !cell.is_empty() || !record.is_empty() || quoted {
                    record.push(finish_cell(&mut cell, &mut quoted));
                    records.push((start_line, std::mem::take(&mut record)));
                }
                pending = false;
                line += 1;
                start_line = line;
            }
            _ => {
                cell.push(c);
                pending = true;
            }
        }
    }
    if in_quotes {
        return Err(format!("line {start_line}: a quoted cell is not closed"));
    }
    if pending || !cell.is_empty() || !record.is_empty() || quoted {
        record.push(finish_cell(&mut cell, &mut quoted));
        records.push((start_line, record));
    }
    let mut kept = records.into_iter().filter(|(_, r)| {
        let blank = r.iter().all(|c| c.trim().is_empty());
        let comment = r.first().is_some_and(|c| c.trim_start().starts_with('#'));
        !blank && !comment
    });
    let Some((_, header)) = kept.next() else {
        return Err("the file has no header line".to_owned());
    };
    let header: Vec<String> = header
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    Ok(Table {
        header,
        rows: kept.collect(),
    })
}

fn finish_cell(cell: &mut String, quoted: &mut bool) -> String {
    let done = if *quoted {
        std::mem::take(cell)
    } else {
        std::mem::take(cell).trim().to_owned()
    };
    *quoted = false;
    done
}

/// A text cell of a CSV file: quoted when it holds a comma, a quote or a line break, and, when it
/// could be read by a spreadsheet as a formula (it starts with `=`, `+`, `-`, `@`, a tab or a
/// carriage return), with a quote mark in front so that it is text.
pub fn csv_text(cell: &str) -> String {
    let danger = cell
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    let body = if danger {
        format!("'{cell}")
    } else {
        cell.to_owned()
    };
    csv_raw(&body)
}

/// A cell that is already safe (a number): quoted only when it needs to be.
pub fn csv_raw(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// Text for the body or an attribute of HTML, with `& < > " '` replaced.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_is_read_with_quotes_comments_and_blank_lines() {
        let text =
            "\u{feff}Value, Footprint\n# a comment\n\n10k,R0603\n\"a,b\",\"say \"\"hi\"\"\"\n";
        let t = parse_table(text).unwrap();
        assert_eq!(t.header, vec!["value", "footprint"]);
        assert_eq!(t.column("footprint"), Some(1));
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0], (4, vec!["10k".to_owned(), "R0603".to_owned()]));
        assert_eq!(t.rows[1].1, vec!["a,b".to_owned(), "say \"hi\"".to_owned()]);
    }

    #[test]
    fn a_quote_in_the_middle_and_an_open_quote_are_refused() {
        assert!(parse_table("a,b\nx\"y,2\n")
            .unwrap_err()
            .starts_with("line 2"));
        assert!(parse_table("a,b\n\"open,2\n")
            .unwrap_err()
            .contains("not closed"));
        assert!(parse_table("\n\n").is_err());
    }

    #[test]
    fn the_last_line_needs_no_line_break_and_crlf_is_read() {
        let t = parse_table("a,b\r\n1,2\r\n3,").unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[1].1, vec!["3".to_owned(), String::new()]);
    }

    #[test]
    fn cells_that_a_spreadsheet_would_run_are_made_text() {
        assert_eq!(csv_text("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_text("@x"), "'@x");
        assert_eq!(csv_text("-1"), "'-1");
        assert_eq!(csv_text("plain"), "plain");
        assert_eq!(csv_text("a,b"), "\"a,b\"");
        assert_eq!(csv_text("say \"x\""), "\"say \"\"x\"\"\"");
        assert_eq!(csv_raw("12.500000"), "12.500000");
    }

    #[test]
    fn html_is_escaped() {
        assert_eq!(
            html_escape("<b a=\"1\">'&'</b>"),
            "&lt;b a=&quot;1&quot;&gt;&#39;&amp;&#39;&lt;/b&gt;"
        );
    }
}
