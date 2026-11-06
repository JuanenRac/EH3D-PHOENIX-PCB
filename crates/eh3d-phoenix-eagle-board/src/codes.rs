//! The codes of the board writer and reader, `PHOENIX-EAGLE-<NNN>`, continuing the table of the
//! EAGLE reader.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// An item was written in a simpler form than the model holds it.
pub const ITEM_SIMPLIFIED: &str = "PHOENIX-EAGLE-020";
/// An item cannot be written in an EAGLE file and was left out.
pub const ITEM_LEFT_OUT: &str = "PHOENIX-EAGLE-021";
/// A part of an EAGLE file was not read into the board.
pub const NOT_READ: &str = "PHOENIX-EAGLE-022";
/// A file cannot be read into a board at all.
pub const UNREADABLE: &str = "PHOENIX-EAGLE-023";

/// The codes of this crate with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (ITEM_SIMPLIFIED, "Item written in a simpler form"),
    (ITEM_LEFT_OUT, "Item left out of the EAGLE file"),
    (NOT_READ, "Part of the EAGLE file not read into the board"),
    (UNREADABLE, "EAGLE file cannot be read into a board"),
];

/// The tags of the languages of [`TITLES`], in the order of its columns.
pub const TITLE_LANGUAGES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

/// The titles of the codes in English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese,
/// in the order of [`REGISTRY`]. The messages of the diagnostics keep the English title.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        ITEM_SIMPLIFIED,
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
        ITEM_LEFT_OUT,
        [
            "Item left out of the EAGLE file",
            "Elemento omitido del fichero EAGLE",
            "Element nicht in die EAGLE-Datei geschrieben",
            "Élément omis du fichier EAGLE",
            "Elemento omesso dal file EAGLE",
            "項目は EAGLE ファイルに書き出されませんでした",
            "项目未写入 EAGLE 文件",
            "Elemento omitido do ficheiro EAGLE",
        ],
    ),
    (
        NOT_READ,
        [
            "Part of the EAGLE file not read into the board",
            "Parte del fichero EAGLE no leída en la placa",
            "Teil der EAGLE-Datei nicht in die Platine eingelesen",
            "Partie du fichier EAGLE non lue dans la carte",
            "Parte del file EAGLE non letta nella scheda",
            "EAGLE ファイルの一部は基板に読み込まれませんでした",
            "EAGLE 文件的一部分未读入电路板",
            "Parte do ficheiro EAGLE não lida para a placa",
        ],
    ),
    (
        UNREADABLE,
        [
            "EAGLE file cannot be read into a board",
            "El fichero EAGLE no se puede leer como placa",
            "Die EAGLE-Datei kann nicht als Platine gelesen werden",
            "Le fichier EAGLE ne peut pas être lu comme une carte",
            "Il file EAGLE non può essere letto come scheda",
            "EAGLE ファイルを基板として読み込めません",
            "无法将 EAGLE 文件读取为电路板",
            "O ficheiro EAGLE não pode ser lido como placa",
        ],
    ),
];

/// The title of a code in a language (one of [`TITLE_LANGUAGES`]), or none when the code or the language is not
/// in the table.
pub fn title(code: &str, language: &str) -> Option<&'static str> {
    let column = TITLE_LANGUAGES.iter().position(|l| *l == language)?;
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, row)| row[column])
}

/// Builds the diagnostic of `code` (which must be in [`REGISTRY`]) with a detail.
pub fn diagnostic(code: &'static str, severity: Severity, detail: &str) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Unregistered code", |(_, t)| t);
    let message = if detail.is_empty() {
        title.to_owned()
    } else {
        format!("{title}: {detail}")
    };
    Diagnostic::new(0, severity, message)
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_titles_in_eight_languages_follow_the_registry_and_the_document() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        assert_eq!(TITLES.len(), REGISTRY.len());
        for ((code, english), (table_code, texts)) in REGISTRY.iter().zip(TITLES) {
            assert_eq!(code, table_code);
            assert_eq!(*english, texts[0]);
            for (language, text) in TITLE_LANGUAGES.iter().zip(texts) {
                assert!(!text.is_empty(), "{code} {language}");
                assert_eq!(title(code, language), Some(*text));
            }
            assert_ne!(texts[5], texts[0], "{code}: Japanese");
            assert_ne!(texts[6], texts[0], "{code}: Chinese");
            let wanted = format!("| {code} | {} |", texts.join(" | "));
            assert!(doc.lines().any(|l| l == wanted), "{code}: {wanted}");
        }
        assert_eq!(
            title(NOT_READ, "pt"),
            Some("Parte do ficheiro EAGLE não lida para a placa")
        );
        assert_eq!(title("PHOENIX-EAGLE-999", "en"), None);
        assert_eq!(title(NOT_READ, "xx"), None);
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            assert!(seen.insert(*code));
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
