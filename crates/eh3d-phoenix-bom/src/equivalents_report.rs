// SPDX-License-Identifier: GPL-3.0-or-later
//! The sets of lines that can be bought as one ([`crate::equivalents`]) as text in the eight
//! languages of the suite and as JSON, for `phoenix bom --equivalents`.
//!
//! The text has a heading with the number of sets, then one line for each set: its number, the
//! lines of the bill that make it (counted from 1, as they are in the bill, with the value and
//! the footprint of the first one), the part numbers, the parts of one board and the parts that
//! the batch needs; a set whose lines do not all have the same value and footprint is followed by
//! a line that asks to check the alternatives. The words are in [`words`]; nothing but the words
//! changes with the language.

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::bom::BomLine;
use crate::equivalents::EquivalentGroup;

/// The words of the report in one language.
struct Words {
    title: &'static str,
    none: &'static str,
    set: &'static str,
    lines: &'static str,
    numbers: &'static str,
    per_board: &'static str,
    for_batch: &'static str,
    mixed: &'static str,
}

fn words(lang: Lang) -> Words {
    match lang {
        Lang::En => Words {
            title: "Lines that can be bought as one (sets)",
            none: "No line of the bill can be bought as one.",
            set: "set",
            lines: "lines",
            numbers: "part numbers",
            per_board: "per board",
            for_batch: "for the batch",
            mixed: "the value or the footprint differs: check the alternatives",
        },
        Lang::Es => Words {
            title: "Líneas que se pueden comprar como una (conjuntos)",
            none: "Ninguna línea de la lista se puede comprar como una sola.",
            set: "conjunto",
            lines: "líneas",
            numbers: "números de pieza",
            per_board: "por placa",
            for_batch: "para el lote",
            mixed: "el valor o la huella difiere: revisa las alternativas",
        },
        Lang::De => Words {
            title: "Zeilen, die als eine gekauft werden können (Gruppen)",
            none: "Keine Zeile der Stückliste kann als eine gekauft werden.",
            set: "Gruppe",
            lines: "Zeilen",
            numbers: "Teilenummern",
            per_board: "pro Platine",
            for_batch: "für die Charge",
            mixed: "Wert oder Bauform weicht ab: Alternativen prüfen",
        },
        Lang::Fr => Words {
            title: "Lignes achetables comme une seule (ensembles)",
            none: "Aucune ligne de la nomenclature ne peut être achetée comme une seule.",
            set: "ensemble",
            lines: "lignes",
            numbers: "références de pièce",
            per_board: "par carte",
            for_batch: "pour le lot",
            mixed: "la valeur ou l'empreinte diffère : vérifier les alternatives",
        },
        Lang::It => Words {
            title: "Righe acquistabili come una sola (gruppi)",
            none: "Nessuna riga della distinta può essere acquistata come una sola.",
            set: "gruppo",
            lines: "righe",
            numbers: "codici articolo",
            per_board: "per scheda",
            for_batch: "per il lotto",
            mixed: "il valore o il footprint è diverso: controllare le alternative",
        },
        Lang::Ja => Words {
            title: "一括購入できる行（組）",
            none: "部品表に一括購入できる行はありません。",
            set: "組",
            lines: "行",
            numbers: "部品番号",
            per_board: "1枚あたり",
            for_batch: "ロットに必要な数",
            mixed: "値またはフットプリントが異なります: 代替品を確認してください",
        },
        Lang::Zh => Words {
            title: "可合并采购的行（组）",
            none: "物料清单中没有可以合并采购的行。",
            set: "组",
            lines: "行",
            numbers: "料号",
            per_board: "每块板",
            for_batch: "整批所需",
            mixed: "值或封装不同: 请检查替代料",
        },
        Lang::Pt => Words {
            title: "Linhas que podem ser compradas como uma só (conjuntos)",
            none: "Nenhuma linha da lista de materiais pode ser comprada como uma só.",
            set: "conjunto",
            lines: "linhas",
            numbers: "números de peça",
            per_board: "por placa",
            for_batch: "para o lote",
            mixed: "o valor ou o footprint difere: verificar as alternativas",
        },
    }
}

/// The sets as text in a language, ending in a line break. A position of a set that is not in
/// `lines` is written as a number and nothing else.
pub fn equivalents_text(lines: &[BomLine], groups: &[EquivalentGroup], lang: Lang) -> String {
    let w = words(lang);
    if groups.is_empty() {
        return format!("{}\n", w.none);
    }
    let mut out = format!("{}: {}\n", w.title, groups.len());
    for (k, group) in groups.iter().enumerate() {
        let positions: Vec<String> = group.lines.iter().map(|i| (i + 1).to_string()).collect();
        let kind = group
            .lines
            .first()
            .and_then(|&i| lines.get(i))
            .map(|l| format!(" ({} {})", l.value, l.footprint))
            .unwrap_or_default();
        out.push_str(&format!(
            "{} {}: {} {}{}; {}: {}; {}: {}; {}: {}\n",
            w.set,
            k + 1,
            w.lines,
            positions.join(", "),
            kind,
            w.numbers,
            group.part_numbers.join(", "),
            w.per_board,
            group.qty,
            w.for_batch,
            group.needed
        ));
        if group.mixed_kinds {
            out.push_str(&format!("  ! {}\n", w.mixed));
        }
    }
    out
}

/// The sets as JSON: `{"sets": [{"lines": [..], "part_numbers": [..], "qty": N, "needed": N,
/// "mixed_kinds": bool}]}`, with the positions of the lines counted from 0.
pub fn equivalents_json(groups: &[EquivalentGroup]) -> Value {
    let sets: Vec<Value> = groups
        .iter()
        .map(|g| {
            json!({
                "lines": g.lines,
                "part_numbers": g.part_numbers,
                "qty": g.qty,
                "needed": g.needed,
                "mixed_kinds": g.mixed_kinds,
            })
        })
        .collect();
    json!({ "sets": sets })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equivalents::equivalent_groups;

    fn line(value: &str, footprint: &str, mpn: &str, alternatives: &[&str], qty: u64) -> BomLine {
        BomLine {
            value: value.to_owned(),
            footprint: footprint.to_owned(),
            manufacturer: String::new(),
            mpn: mpn.to_owned(),
            alternatives: alternatives.iter().map(|a| (*a).to_owned()).collect(),
            references: Vec::new(),
            qty,
            needed: qty * 10,
            purchase: None,
            cost_per_board_micro: 0,
        }
    }

    fn sample() -> Vec<BomLine> {
        vec![
            line("10k", "R0603", "RC-A", &["RC-B"], 3),
            line("100n", "C0603", "CC-1", &[], 2),
            line("10k", "R0603", "rc-b", &[], 4),
            line("1k", "R0402", "X", &["Y"], 1),
            line("1k", "R0805", "y", &[], 1),
        ]
    }

    #[test]
    fn every_language_names_the_sets_the_numbers_and_the_quantities() {
        let lines = sample();
        let groups = equivalent_groups(&lines);
        assert_eq!(groups.len(), 2);
        let mut seen = std::collections::BTreeSet::new();
        for lang in Lang::ALL {
            let text = equivalents_text(&lines, &groups, lang);
            assert!(text.ends_with('\n'));
            assert!(text.contains("RC-A, RC-B"), "{lang:?}: {text}");
            assert!(text.contains("1, 3"), "{lang:?}");
            assert!(text.contains("(10k R0603)"), "{lang:?}");
            assert!(text.contains(": 7;"), "{lang:?}");
            assert!(text.contains(": 70\n"), "{lang:?}");
            // The second set is the mixed one and has the line that asks to check it.
            assert_eq!(text.lines().filter(|l| l.starts_with("  ! ")).count(), 1);
            assert!(seen.insert(text), "{lang:?} repeats another language");
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn the_words_of_the_eight_languages_are_all_different() {
        let mut titles = std::collections::BTreeSet::new();
        let mut nones = std::collections::BTreeSet::new();
        let mut mixeds = std::collections::BTreeSet::new();
        for lang in Lang::ALL {
            let w = words(lang);
            for text in [
                w.title,
                w.none,
                w.set,
                w.lines,
                w.numbers,
                w.per_board,
                w.for_batch,
                w.mixed,
            ] {
                assert!(!text.trim().is_empty(), "{lang:?}");
            }
            titles.insert(w.title);
            nones.insert(w.none);
            mixeds.insert(w.mixed);
        }
        assert_eq!((titles.len(), nones.len(), mixeds.len()), (8, 8, 8));
    }

    #[test]
    fn no_sets_give_one_line_in_each_language() {
        let lines = [line("1k", "R0402", "A", &[], 1)];
        let mut seen = std::collections::BTreeSet::new();
        for lang in Lang::ALL {
            let text = equivalents_text(&lines, &[], lang);
            assert_eq!(text.lines().count(), 1);
            seen.insert(text);
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn a_position_that_is_not_in_the_bill_is_only_a_number() {
        let group = EquivalentGroup {
            lines: vec![7, 9],
            part_numbers: vec!["A".to_owned()],
            qty: 2,
            needed: 20,
            mixed_kinds: false,
        };
        let text = equivalents_text(&[], &[group], Lang::En);
        assert!(text.contains("lines 8, 10;"), "{text}");
    }

    #[test]
    fn the_json_holds_the_sets_with_positions_from_zero() {
        let lines = sample();
        let groups = equivalent_groups(&lines);
        let value = equivalents_json(&groups);
        let sets = value["sets"].as_array().unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0]["lines"], json!([0, 2]));
        assert_eq!(sets[0]["part_numbers"], json!(["RC-A", "RC-B"]));
        assert_eq!(sets[0]["qty"], json!(7));
        assert_eq!(sets[1]["mixed_kinds"], json!(true));
        assert_eq!(equivalents_json(&[]), json!({ "sets": [] }));
    }
}
