// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of two libraries of footprints as text in the eight languages of the suite and
//! as JSON.
//!
//! [`compare_text`] writes the footprints only the new library has, the ones only the old one
//! has, the ones both have with other pads (each with its differences, which are sentences in
//! English) and the number of the ones with the same pads, and a last line that says whether the
//! libraries are the same. [`compare_json`] writes the same facts as JSON of the format
//! `phoenix-libcompare/1`, with the names escaped and the keys in a fixed order. Neither holds a
//! date, a path, a host name or a user name, and the same comparison gives the same bytes.

use eh3d_phoenix_rules::Lang;

use crate::compare::LibraryComparison;

/// The format name of the JSON.
pub const FORMAT: &str = "phoenix-libcompare/1";

/// The words of the report in one language.
struct Words {
    added: &'static str,
    removed: &'static str,
    changed: &'static str,
    same: &'static str,
    identical: &'static str,
    differ: &'static str,
}

fn words(lang: Lang) -> Words {
    let w = |added, removed, changed, same, identical, differ| Words {
        added,
        removed,
        changed,
        same,
        identical,
        differ,
    };
    match lang {
        Lang::En => w(
            "Footprints only in the new library",
            "Footprints only in the old library",
            "Footprints with other pads",
            "Footprints with the same pads",
            "The libraries have the same footprints with the same pads",
            "The libraries differ",
        ),
        Lang::Es => w(
            "Huellas solo en la biblioteca nueva",
            "Huellas solo en la biblioteca antigua",
            "Huellas con otros pads",
            "Huellas con los mismos pads",
            "Las bibliotecas tienen las mismas huellas con los mismos pads",
            "Las bibliotecas difieren",
        ),
        Lang::De => w(
            "Footprints nur in der neuen Bibliothek",
            "Footprints nur in der alten Bibliothek",
            "Footprints mit anderen Pads",
            "Footprints mit gleichen Pads",
            "Die Bibliotheken haben dieselben Footprints mit denselben Pads",
            "Die Bibliotheken unterscheiden sich",
        ),
        Lang::Fr => w(
            "Empreintes seulement dans la nouvelle bibliothèque",
            "Empreintes seulement dans l'ancienne bibliothèque",
            "Empreintes avec d'autres pastilles",
            "Empreintes avec les mêmes pastilles",
            "Les bibliothèques ont les mêmes empreintes avec les mêmes pastilles",
            "Les bibliothèques diffèrent",
        ),
        Lang::It => w(
            "Footprint solo nella libreria nuova",
            "Footprint solo nella libreria vecchia",
            "Footprint con altri pad",
            "Footprint con gli stessi pad",
            "Le librerie hanno gli stessi footprint con gli stessi pad",
            "Le librerie sono diverse",
        ),
        Lang::Ja => w(
            "新しいライブラリにだけあるフットプリント",
            "古いライブラリにだけあるフットプリント",
            "パッドが異なるフットプリント",
            "パッドが同じフットプリント",
            "ライブラリのフットプリントとパッドは同じです",
            "ライブラリは異なります",
        ),
        Lang::Zh => w(
            "仅在新库中的封装",
            "仅在旧库中的封装",
            "焊盘不同的封装",
            "焊盘相同的封装",
            "两个库的封装和焊盘相同",
            "两个库不同",
        ),
        Lang::Pt => w(
            "Pegadas só na biblioteca nova",
            "Pegadas só na biblioteca antiga",
            "Pegadas com outros pads",
            "Pegadas com os mesmos pads",
            "As bibliotecas têm as mesmas pegadas com os mesmos pads",
            "As bibliotecas diferem",
        ),
    }
}

/// The comparison as text in a language.
pub fn compare_text(found: &LibraryComparison, lang: Lang) -> String {
    let w = words(lang);
    let mut out = String::new();
    out.push_str(&format!("{}: {}\n", w.added, found.added.len()));
    for name in &found.added {
        out.push_str(&format!("  {name}\n"));
    }
    out.push_str(&format!("{}: {}\n", w.removed, found.removed.len()));
    for name in &found.removed {
        out.push_str(&format!("  {name}\n"));
    }
    out.push_str(&format!("{}: {}\n", w.changed, found.changed.len()));
    for change in &found.changed {
        out.push_str(&format!("  {}\n", change.name));
        for difference in &change.differences {
            out.push_str(&format!("    {difference}\n"));
        }
    }
    out.push_str(&format!("{}: {}\n", w.same, found.same));
    out.push_str(if found.is_same() {
        w.identical
    } else {
        w.differ
    });
    out.push('\n');
    out
}

/// A text as a JSON string, with the characters that need it escaped.
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

fn names(list: &[String]) -> String {
    let items: Vec<String> = list.iter().map(|n| quote(n)).collect();
    format!("[{}]", items.join(","))
}

/// The comparison as JSON of the format `phoenix-libcompare/1`.
pub fn compare_json(found: &LibraryComparison) -> String {
    let changed: Vec<String> = found
        .changed
        .iter()
        .map(|change| {
            format!(
                "{{\"name\":{},\"differences\":{}}}",
                quote(&change.name),
                names(&change.differences)
            )
        })
        .collect();
    format!(
        "{{\"format\":\"{FORMAT}\",\"same\":{},\"added\":{},\"removed\":{},\"changed\":[{}],\"unchanged\":{}}}\n",
        found.is_same(),
        names(&found.added),
        names(&found.removed),
        changed.join(","),
        found.same
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::FootprintChange;

    fn sample() -> LibraryComparison {
        LibraryComparison {
            added: vec!["C0402".to_owned()],
            removed: vec!["R0201".to_owned(), "R1206".to_owned()],
            changed: vec![FootprintChange {
                name: "R0603".to_owned(),
                differences: vec![
                    "pad 1 moved by 40000 nm".to_owned(),
                    "pad 2 only in the new footprint".to_owned(),
                ],
            }],
            same: 7,
        }
    }

    #[test]
    fn the_text_lists_every_group_with_its_count_and_a_verdict() {
        let text = compare_text(&sample(), Lang::En);
        let expected = "Footprints only in the new library: 1\n  C0402\n\
                        Footprints only in the old library: 2\n  R0201\n  R1206\n\
                        Footprints with other pads: 1\n  R0603\n    pad 1 moved by 40000 nm\n    pad 2 only in the new footprint\n\
                        Footprints with the same pads: 7\nThe libraries differ\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn libraries_that_are_the_same_say_so_in_every_language_and_the_words_differ() {
        let same = LibraryComparison {
            same: 3,
            ..LibraryComparison::default()
        };
        let mut seen: Vec<String> = Vec::new();
        for lang in Lang::ALL {
            let text = compare_text(&same, lang);
            assert_eq!(text.lines().count(), 5, "{text}");
            assert!(text.contains(": 3\n"), "{text}");
            let full = compare_text(&sample(), lang);
            assert!(full.contains("C0402") && full.contains("R0603"), "{full}");
            assert!(!seen.contains(&text), "{lang:?} repeats a text");
            seen.push(text);
        }
        // the verdict is the only line that changes between the same and the different
        let en_same = compare_text(&same, Lang::En);
        assert!(en_same.ends_with("The libraries have the same footprints with the same pads\n"));
        assert!(compare_text(&same, Lang::Pt).contains("Pegadas"));
        assert!(compare_text(&same, Lang::De).contains("Footprints mit gleichen Pads: 3"));
    }

    #[test]
    fn the_json_has_a_fixed_order_and_escapes_the_names() {
        let found = LibraryComparison {
            added: vec!["A\"B".to_owned()],
            removed: Vec::new(),
            changed: vec![FootprintChange {
                name: "X\\Y".to_owned(),
                differences: vec!["line\nbreak".to_owned()],
            }],
            same: 0,
        };
        assert_eq!(
            compare_json(&found),
            "{\"format\":\"phoenix-libcompare/1\",\"same\":false,\"added\":[\"A\\\"B\"],\"removed\":[],\"changed\":[{\"name\":\"X\\\\Y\",\"differences\":[\"line\\nbreak\"]}],\"unchanged\":0}\n"
        );
        let none = LibraryComparison::default();
        assert_eq!(
            compare_json(&none),
            "{\"format\":\"phoenix-libcompare/1\",\"same\":true,\"added\":[],\"removed\":[],\"changed\":[],\"unchanged\":0}\n"
        );
        assert_eq!(compare_json(&sample()), compare_json(&sample()));
    }

    #[test]
    fn a_control_character_in_a_name_is_written_as_a_unicode_escape() {
        assert_eq!(quote("a\u{1}b"), "\"a\\u0001b\"");
        assert_eq!(quote("tab\there"), "\"tab\\there\"");
        assert_eq!(quote("caf\u{e9}"), "\"caf\u{e9}\"");
    }
}
