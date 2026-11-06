// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of an export to KiCad and its codes `PHOENIX-KICADX-<NNN>`.
//!
//! Whatever the model cannot say in a KiCad file is counted here and never dropped in silence:
//! each note has a code, a category that names what was affected and the first detail found. The
//! titles of the codes are in the eight languages of the suite (English, Spanish, German, French,
//! Italian, Japanese, Chinese and European Portuguese); the details are in English.

use std::collections::BTreeMap;

use crate::error::Language;

/// Part of the model that has no place in the KiCad file and was left out.
pub const LEFT_OUT: &str = "PHOENIX-KICADX-001";
/// An item written in a simpler form than the model has.
pub const CHANGED: &str = "PHOENIX-KICADX-002";
/// A value the model does not give, written as a default.
pub const DEFAULTED: &str = "PHOENIX-KICADX-003";
/// A reference to something the model does not have; the item was left out.
pub const DANGLING: &str = "PHOENIX-KICADX-004";
/// The output of the export is not allowed.
pub const REFUSED: &str = "PHOENIX-KICADX-005";
/// The file that was written does not read back the same.
pub const ROUND_TRIP: &str = "PHOENIX-KICADX-006";
/// The summary of the export, with the state of the writers.
pub const SUMMARY: &str = "PHOENIX-KICADX-007";

/// Every code with its English title, in order. `docs/ERROR_CODES.md` repeats it.
pub const REGISTRY: &[(&str, &str)] = &[
    (LEFT_OUT, "Part of the model left out of the KiCad file"),
    (CHANGED, "Item written in a simpler form"),
    (
        DEFAULTED,
        "Value the model does not give, written as a default",
    ),
    (DANGLING, "Reference to something the model does not have"),
    (REFUSED, "The output of the export is not allowed"),
    (ROUND_TRIP, "The written file does not read back the same"),
    (SUMMARY, "Summary of the export"),
];

/// The titles in English, Spanish, German, French, Italian, Japanese, Chinese and European
/// Portuguese.
const TITLES: [(&str, [&str; 8]); 7] = [
    (
        LEFT_OUT,
        [
            "Part of the model left out of the KiCad file",
            "Parte del modelo omitida del archivo de KiCad",
            "Teil des Modells nicht in die KiCad-Datei geschrieben",
            "Partie du modèle omise du fichier KiCad",
            "Parte del modello omessa dal file KiCad",
            "モデルの一部は KiCad ファイルに書き出されませんでした",
            "模型的一部分未写入 KiCad 文件",
            "Parte do modelo omitida do ficheiro KiCad",
        ],
    ),
    (
        CHANGED,
        [
            "Item written in a simpler form",
            "Elemento escrito de forma más simple",
            "Element in einfacherer Form geschrieben",
            "Élément écrit sous une forme plus simple",
            "Elemento scritto in forma più semplice",
            "項目はより単純な形で書き出されました",
            "项目以更简单的形式写入",
            "Elemento escrito de forma mais simples",
        ],
    ),
    (
        DEFAULTED,
        [
            "Value the model does not give, written as a default",
            "Valor que el modelo no da, escrito con un valor por defecto",
            "Wert, den das Modell nicht liefert, als Standardwert geschrieben",
            "Valeur absente du modèle, écrite par défaut",
            "Valore assente nel modello, scritto come predefinito",
            "モデルにない値は既定値で書き出されました",
            "模型未提供的值以默认值写入",
            "Valor que o modelo não dá, escrito por omissão",
        ],
    ),
    (
        DANGLING,
        [
            "Reference to something the model does not have",
            "Referencia a algo que el modelo no tiene",
            "Verweis auf etwas, das im Modell fehlt",
            "Référence à un élément absent du modèle",
            "Riferimento a qualcosa che il modello non ha",
            "モデルに存在しないものへの参照",
            "引用了模型中不存在的内容",
            "Referência a algo que o modelo não tem",
        ],
    ),
    (
        REFUSED,
        [
            "The output of the export is not allowed",
            "La salida de la exportación no está permitida",
            "Die Ausgabe des Exports ist nicht erlaubt",
            "La sortie de l'export n'est pas autorisée",
            "L'output dell'esportazione non è consentito",
            "エクスポートの出力先は許可されていません",
            "不允许该导出输出",
            "A saída da exportação não é permitida",
        ],
    ),
    (
        ROUND_TRIP,
        [
            "The written file does not read back the same",
            "El archivo escrito no se lee igual",
            "Die geschriebene Datei wird nicht gleich zurückgelesen",
            "Le fichier écrit n'est pas relu à l'identique",
            "Il file scritto non viene riletto uguale",
            "書き出したファイルを読み戻すと内容が一致しません",
            "写出的文件读回后不一致",
            "O ficheiro escrito não é lido de volta da mesma forma",
        ],
    ),
    (
        SUMMARY,
        [
            "Summary of the export",
            "Resumen de la exportación",
            "Zusammenfassung des Exports",
            "Résumé de l'export",
            "Riepilogo dell'esportazione",
            "エクスポートの概要",
            "导出摘要",
            "Resumo da exportação",
        ],
    ),
];

fn column(language: Language) -> usize {
    match language {
        Language::En => 0,
        Language::Es => 1,
        Language::De => 2,
        Language::Fr => 3,
        Language::It => 4,
        Language::Ja => 5,
        Language::Zh => 6,
        Language::Pt => 7,
    }
}

/// The title of a code in a language, or `None` for a code the writers do not have.
pub fn export_title(code: &str, language: Language) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, texts)| texts.get(column(language)).copied())
}

/// One kind of note of a report: how many times it happened and the first detail found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The code.
    pub code: &'static str,
    /// What was affected, in words (`pads with a simpler shape`).
    pub category: String,
    /// How many times it happened.
    pub count: usize,
    /// The first detail found, in English.
    pub detail: String,
}

/// What an export wrote and what it could not say.
#[derive(Clone, Debug, Default)]
pub struct ExportReport {
    written: BTreeMap<String, usize>,
    notes: BTreeMap<(&'static str, String), (usize, String)>,
}

impl ExportReport {
    /// Counts items written (nothing is recorded for a count of zero).
    pub fn wrote(&mut self, what: &str, count: usize) {
        if count > 0 {
            *self.written.entry(what.to_owned()).or_insert(0) += count;
        }
    }

    /// Records one case of a kind of note.
    pub fn note(&mut self, code: &'static str, category: &str, detail: &str) {
        self.note_many(code, category, detail, 1);
    }

    /// Records `count` cases of a kind of note.
    pub fn note_many(&mut self, code: &'static str, category: &str, detail: &str, count: usize) {
        if count == 0 {
            return;
        }
        let entry = self
            .notes
            .entry((code, category.to_owned()))
            .or_insert_with(|| (0, detail.to_owned()));
        entry.0 += count;
    }

    /// What was written, by name.
    pub fn written(&self) -> Vec<(String, usize)> {
        self.written.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    /// How many items of this kind were written.
    pub fn written_count(&self, what: &str) -> usize {
        self.written.get(what).copied().unwrap_or(0)
    }

    /// The notes, ordered by code and category.
    pub fn notes(&self) -> Vec<Note> {
        self.notes
            .iter()
            .map(|((code, category), (count, detail))| Note {
                code,
                category: category.clone(),
                count: *count,
                detail: detail.clone(),
            })
            .collect()
    }

    /// The number of cases of every note with this code.
    pub fn count_of(&self, code: &str) -> usize {
        self.notes
            .iter()
            .filter(|((c, _), _)| *c == code)
            .map(|(_, (n, _))| *n)
            .sum()
    }

    /// True when a note says that the file does not read back the same.
    pub fn has_problem(&self) -> bool {
        self.count_of(ROUND_TRIP) > 0 || self.count_of(REFUSED) > 0
    }

    /// Adds the counts and the notes of another report.
    pub fn merge(&mut self, other: &ExportReport) {
        for (what, count) in &other.written {
            self.wrote(what, *count);
        }
        for ((code, category), (count, detail)) in &other.notes {
            self.note_many(code, category, detail, *count);
        }
    }

    /// The report as text, with the titles in `language`.
    pub fn text(&self, language: Language) -> String {
        let mut out = String::new();
        if !self.written.is_empty() {
            out.push_str("written:\n");
            for (what, count) in &self.written {
                out.push_str(&format!("  {what}: {count}\n"));
            }
        }
        let groups: [(&str, &[&str]); 3] = [
            ("left out", &[LEFT_OUT, DANGLING]),
            ("changed", &[CHANGED, DEFAULTED]),
            ("problems", &[REFUSED, ROUND_TRIP]),
        ];
        for (heading, codes) in groups {
            let lines: Vec<String> = self
                .notes()
                .iter()
                .filter(|n| codes.contains(&n.code))
                .map(|n| {
                    format!(
                        "  {} {}: {} ({}) [{}]\n",
                        n.code,
                        export_title(n.code, language).unwrap_or(""),
                        n.category,
                        n.count,
                        n.detail
                    )
                })
                .collect();
            if !lines.is_empty() {
                out.push_str(heading);
                out.push_str(":\n");
                for line in lines {
                    out.push_str(&line);
                }
            }
        }
        out
    }

    /// The report as JSON, with the titles in `language`.
    pub fn to_json(&self, language: Language) -> serde_json::Value {
        let notes: Vec<serde_json::Value> = self
            .notes()
            .iter()
            .map(|n| {
                serde_json::json!({
                    "code": n.code,
                    "title": export_title(n.code, language).unwrap_or(""),
                    "category": n.category,
                    "count": n.count,
                    "detail": n.detail,
                })
            })
            .collect();
        serde_json::json!({ "written": self.written, "notes": notes })
    }
}

/// A file of the export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportFile {
    /// The name of the file, without a folder.
    pub name: String,
    /// The text.
    pub text: String,
}

/// The files an export made and its report.
#[derive(Clone, Debug)]
pub struct Export {
    /// The files, in the order they should be written.
    pub files: Vec<ExportFile>,
    /// What was written and what could not be said.
    pub report: ExportReport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_table_agrees_with_the_registry_in_every_language() {
        assert_eq!(TITLES.len(), REGISTRY.len());
        let mut seen = std::collections::BTreeSet::new();
        for ((code, english), (table_code, texts)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, table_code);
            assert_eq!(*english, texts[0]);
            assert!(seen.insert(*code), "{code} repeated");
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 3);
            assert_eq!((parts[0], parts[1]), ("PHOENIX", "KICADX"));
            assert_eq!(parts[2].len(), 3);
            assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
            for text in texts {
                assert!(!text.is_empty(), "{code}");
            }
            assert_ne!(texts[5], texts[0], "{code}: Japanese");
            assert_ne!(texts[6], texts[0], "{code}: Chinese");
        }
        assert_eq!(
            export_title(DANGLING, Language::Pt),
            Some("Referência a algo que o modelo não tem")
        );
        assert_eq!(export_title("PHOENIX-KICADX-999", Language::En), None);
    }

    #[test]
    fn notes_are_counted_by_code_and_category_and_keep_the_first_detail() {
        let mut r = ExportReport::default();
        r.note(CHANGED, "pads", "first");
        r.note(CHANGED, "pads", "second");
        r.note_many(LEFT_OUT, "zones", "fill", 3);
        r.note_many(LEFT_OUT, "none", "x", 0);
        r.wrote("pads", 2);
        r.wrote("pads", 3);
        r.wrote("vias", 0);
        assert_eq!(r.written(), vec![("pads".to_owned(), 5)]);
        assert_eq!(r.count_of(CHANGED), 2);
        assert_eq!(r.count_of(LEFT_OUT), 3);
        let notes = r.notes();
        assert_eq!(notes.len(), 2);
        // ordered by code: what was left out (001) before what was changed (002)
        assert_eq!(notes[0].detail, "fill");
        assert_eq!(notes[1].detail, "first");
        assert!(!r.has_problem());
        let mut other = ExportReport::default();
        other.note(ROUND_TRIP, "x", "y");
        r.merge(&other);
        assert!(r.has_problem());
        assert_eq!(r.count_of(ROUND_TRIP), 1);
    }

    #[test]
    fn the_text_and_the_json_use_the_language() {
        let mut r = ExportReport::default();
        r.note(DANGLING, "pads", "a pad names a missing net");
        r.wrote("pads", 1);
        let text = r.text(Language::Es);
        assert!(text.contains("Referencia a algo que el modelo no tiene"));
        assert!(text.contains("written:"));
        let json = r.to_json(Language::De);
        assert_eq!(json["notes"][0]["code"], DANGLING);
        assert_eq!(
            json["notes"][0]["title"],
            "Verweis auf etwas, das im Modell fehlt"
        );
        assert_eq!(json["written"]["pads"], 1);
    }

    #[test]
    fn the_documented_table_has_every_code_of_the_registry() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
