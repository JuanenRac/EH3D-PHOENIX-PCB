// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of the comparison as text, with its words in the eight languages, and as JSON, the
//! same every time for the same two boards.

use eh3d_phoenix_rules::Lang;

use crate::compare::DiffReport;

/// The format name of the JSON report.
pub const FORMAT: &str = "phoenix-boarddiff/1";

/// The words of the text report, by key, in the eight languages in the order of the tables.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "title",
        [
            "Board comparison",
            "Comparación de placas",
            "Platinenvergleich",
            "Comparaison de cartes",
            "Confronto tra schede",
            "基板の比較",
            "电路板比较",
            "Comparação de placas",
        ],
    ),
    (
        "counts",
        [
            "Counts, old to new",
            "Recuentos, de la antigua a la nueva",
            "Anzahlen, alt zu neu",
            "Décomptes, de l'ancienne à la nouvelle",
            "Conteggi, dalla vecchia alla nuova",
            "件数（旧から新）",
            "数量（由旧到新）",
            "Contagens, da antiga para a nova",
        ],
    ),
    (
        "parts",
        [
            "parts",
            "piezas",
            "Bauteile",
            "composants",
            "componenti",
            "部品",
            "元件",
            "peças",
        ],
    ),
    (
        "nets",
        [
            "nets",
            "redes",
            "Netze",
            "réseaux",
            "reti",
            "ネット",
            "网络",
            "redes",
        ],
    ),
    (
        "pins",
        [
            "pins", "pines", "Pins", "broches", "pin", "ピン", "引脚", "pinos",
        ],
    ),
    (
        "tracks",
        [
            "tracks",
            "pistas",
            "Leiterbahnen",
            "pistes",
            "piste",
            "配線",
            "走线",
            "pistas",
        ],
    ),
    (
        "vias",
        [
            "vias",
            "vías",
            "Durchkontaktierungen",
            "vias",
            "via",
            "ビア",
            "过孔",
            "vias",
        ],
    ),
    (
        "zones",
        [
            "zones",
            "zonas",
            "Zonen",
            "zones",
            "zone",
            "ゾーン",
            "敷铜区",
            "zonas",
        ],
    ),
    (
        "identical",
        [
            "No differences",
            "Sin diferencias",
            "Keine Unterschiede",
            "Aucune différence",
            "Nessuna differenza",
            "差異なし",
            "没有差异",
            "Sem diferenças",
        ],
    ),
    (
        "differences",
        [
            "Differences",
            "Diferencias",
            "Unterschiede",
            "Différences",
            "Differenze",
            "差異",
            "差异",
            "Diferenças",
        ],
    ),
];

/// A word of the report in a language; the key itself for one that is not in the table.
pub fn word(key: &'static str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, row)| row[column])
}

/// The report as text in a language.
pub fn report_text(report: &DiffReport, lang: Lang) -> String {
    let mut out = format!(
        "{}: {} -> {}\n",
        word("title", lang),
        report.old_name,
        report.new_name
    );
    let counts: Vec<String> = report
        .counts
        .iter()
        .map(|c| format!("{} {} -> {}", word(c.what, lang), c.old, c.new))
        .collect();
    out.push_str(&format!(
        "{}: {}\n",
        word("counts", lang),
        counts.join(", ")
    ));
    if report.total == 0 {
        out.push_str(&format!("{}\n", word("identical", lang)));
    } else {
        out.push_str(&format!(
            "{}: {}\n",
            word("differences", lang),
            report.total
        ));
    }
    for difference in &report.differences {
        out.push_str(&format!("  {}\n", difference.line(lang)));
    }
    out
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One cell of the CSV file: a cell that a spreadsheet would run as a formula gets a quote mark in
/// front when `guard` is on, and a cell with a comma, a quote or a line break is quoted.
fn csv_cell(text: &str, guard: bool) -> String {
    let mut cell = text.to_owned();
    if guard
        && matches!(
            cell.chars().next(),
            Some('=' | '+' | '-' | '@' | '\t' | '\r')
        )
    {
        cell.insert(0, '\'');
    }
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell
    }
}

/// The differences as a table for a spreadsheet: the header `code,subject,detail` and one row for
/// each difference of the report, in its order, with a line feed after each row.
pub fn report_csv(report: &DiffReport, guard: bool) -> String {
    let mut out = String::from("code,subject,detail\n");
    for d in &report.differences {
        out.push_str(&format!(
            "{},{},{}\n",
            csv_cell(d.code, guard),
            csv_cell(&d.subject, guard),
            csv_cell(&d.detail, guard)
        ));
    }
    out
}

/// The report as JSON, the same every time.
pub fn report_json(report: &DiffReport) -> String {
    let counts: Vec<String> = report
        .counts
        .iter()
        .map(|c| format!("{}:[{},{}]", quote(c.what), c.old, c.new))
        .collect();
    let differences: Vec<String> = report
        .differences
        .iter()
        .map(|d| {
            format!(
                "{{\"code\":{},\"subject\":{},\"detail\":{}}}",
                quote(d.code),
                quote(&d.subject),
                quote(&d.detail)
            )
        })
        .collect();
    format!(
        "{{\"format\":{},\"old\":{},\"new\":{},\"counts\":{{{}}},\"total\":{},\"differences\":[{}]}}\n",
        quote(FORMAT),
        quote(&report.old_name),
        quote(&report.new_name),
        counts.join(","),
        report.total,
        differences.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{Difference, VALUE_DIFF};
    use crate::compare::Count;

    fn report(total: usize) -> DiffReport {
        let differences = if total == 0 {
            Vec::new()
        } else {
            vec![Difference::new(VALUE_DIFF, "R1", "10k -> \"22k\"")]
        };
        DiffReport {
            old_name: "rev A".into(),
            new_name: "rev B".into(),
            counts: vec![
                Count {
                    what: "parts",
                    old: 2,
                    new: 3,
                },
                Count {
                    what: "vias",
                    old: 1,
                    new: 1,
                },
            ],
            differences,
            total,
        }
    }

    #[test]
    fn every_word_has_a_text_in_each_language() {
        for (key, row) in WORDS {
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{key} column {column}");
            }
        }
        assert_eq!(word("identical", Lang::Pt), "Sem diferenças");
        assert_eq!(word("unknown", Lang::En), "unknown");
        // the counts that the comparison makes all have a word
        for what in ["parts", "nets", "pins", "tracks", "vias", "zones"] {
            assert!(WORDS.iter().any(|(k, _)| *k == what), "{what}");
        }
    }

    #[test]
    fn the_text_names_the_boards_the_counts_and_each_difference() {
        let text = report_text(&report(1), Lang::En);
        assert_eq!(
            text,
            "Board comparison: rev A -> rev B\nCounts, old to new: parts 2 -> 3, vias 1 -> 1\nDifferences: 1\n  PHOENIX-DIFF-008: Value of a part changed [R1]: 10k -> \"22k\"\n"
        );
    }

    #[test]
    fn boards_without_differences_say_so() {
        let text = report_text(&report(0), Lang::En);
        assert!(text.ends_with("No differences\n"), "{text}");
        assert!(!text.contains("PHOENIX-DIFF"));
    }

    #[test]
    fn the_text_differs_in_each_of_the_eight_languages() {
        let mut seen: Vec<String> = Lang::ALL
            .iter()
            .map(|l| report_text(&report(1), *l))
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn the_csv_has_a_header_and_a_row_for_each_difference() {
        let csv = report_csv(&report(1), true);
        assert_eq!(
            csv,
            "code,subject,detail\nPHOENIX-DIFF-008,R1,\"10k -> \"\"22k\"\"\"\n"
        );
        assert_eq!(report_csv(&report(0), true), "code,subject,detail\n");
        // the header and the rows are read back by splitting at the commas outside quotes
        assert_eq!(csv.lines().count(), 2);
    }

    #[test]
    fn a_cell_that_a_spreadsheet_would_run_as_a_formula_gets_a_quote_mark() {
        for text in ["=1+1", "+5V", "-3", "@SUM(A1)", "\tx", "\rx"] {
            assert!(
                csv_cell(text, true).trim_matches('"').starts_with('\''),
                "{text:?}"
            );
            assert!(
                !csv_cell(text, false).trim_matches('"').starts_with('\''),
                "{text:?}"
            );
        }
        // an ordinary cell, a number and a text in the middle are not changed
        for text in ["R1", "10k -> 22k", "3.5", "a=b", "x+y", ""] {
            assert_eq!(csv_cell(text, true), text);
        }
        assert_eq!(csv_cell("a,b", true), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\"", true), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("two\nlines", true), "\"two\nlines\"");
        // a quote mark and a comma together: guarded, then quoted
        assert_eq!(csv_cell("=a,b", true), "\"'=a,b\"");
    }

    #[test]
    fn the_json_is_well_formed_the_same_every_time_and_escapes_the_texts() {
        let json = report_json(&report(1));
        assert_eq!(json, report_json(&report(1)));
        assert_eq!(
            json,
            "{\"format\":\"phoenix-boarddiff/1\",\"old\":\"rev A\",\"new\":\"rev B\",\"counts\":{\"parts\":[2,3],\"vias\":[1,1]},\"total\":1,\"differences\":[{\"code\":\"PHOENIX-DIFF-008\",\"subject\":\"R1\",\"detail\":\"10k -> \\\"22k\\\"\"}]}\n"
        );
        assert_eq!(quote("a\\b\nc\u{1}"), "\"a\\\\b\\nc\\u0001\"");
    }
}
