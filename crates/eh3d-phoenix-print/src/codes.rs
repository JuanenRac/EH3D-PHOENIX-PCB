// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the printing, `PHOENIX-PRINT-<NNN>`, with their titles in the eight
//! languages of the suite, and the error type that carries one.

use eh3d_phoenix_rules::Lang;

/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-PRINT-001";
/// A paper size that is not valid.
pub const PAPER_INVALID: &str = "PHOENIX-PRINT-002";
/// A drawing that does not fit the paper at the scale that was asked.
pub const DOES_NOT_FIT: &str = "PHOENIX-PRINT-003";
/// A document or a choice of layers that leaves nothing to print.
pub const NOTHING_TO_PRINT: &str = "PHOENIX-PRINT-004";
/// An output that may not be written where it was asked.
pub const OUTPUT_REFUSED: &str = "PHOENIX-PRINT-005";
/// A file that could not be written.
pub const WRITE_FAILED: &str = "PHOENIX-PRINT-006";
/// Text that a PDF font cannot show and that is drawn as boxes (a warning).
pub const TEXT_NOT_REPRESENTABLE: &str = "PHOENIX-PRINT-007";
/// A PDF that failed the check of its own tables.
pub const PDF_CHECK_FAILED: &str = "PHOENIX-PRINT-008";
/// A print option that is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-PRINT-009";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (SETTINGS_INVALID, "Print settings are not valid"),
    (PAPER_INVALID, "Paper size is not valid"),
    (
        DOES_NOT_FIT,
        "The drawing does not fit the paper at that scale",
    ),
    (NOTHING_TO_PRINT, "Nothing to print"),
    (OUTPUT_REFUSED, "The output may not be written there"),
    (WRITE_FAILED, "The file could not be written"),
    (
        TEXT_NOT_REPRESENTABLE,
        "Text drawn as boxes in the PDF (use SVG)",
    ),
    (PDF_CHECK_FAILED, "The PDF written failed its own check"),
    (OPTION_INVALID, "Print option is not valid"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: [(&str, [&str; 8]); 9] = [
    (
        SETTINGS_INVALID,
        [
            "Print settings are not valid",
            "Los ajustes de impresión no son válidos",
            "Die Druckeinstellungen sind ungültig",
            "Les réglages d'impression ne sont pas valides",
            "Le impostazioni di stampa non sono valide",
            "印刷設定が正しくありません",
            "打印设置无效",
            "As definições de impressão não são válidas",
        ],
    ),
    (
        PAPER_INVALID,
        [
            "Paper size is not valid",
            "El tamaño de papel no es válido",
            "Das Papierformat ist ungültig",
            "Le format de papier n'est pas valide",
            "Il formato della carta non è valido",
            "用紙サイズが正しくありません",
            "纸张尺寸无效",
            "O tamanho do papel não é válido",
        ],
    ),
    (
        DOES_NOT_FIT,
        [
            "The drawing does not fit the paper at that scale",
            "El dibujo no cabe en el papel a esa escala",
            "Die Zeichnung passt bei diesem Maßstab nicht auf das Papier",
            "Le dessin ne tient pas sur le papier à cette échelle",
            "Il disegno non sta sulla carta a questa scala",
            "この縮尺では図面が用紙に収まりません",
            "按此比例图纸放不进纸张",
            "O desenho não cabe no papel a essa escala",
        ],
    ),
    (
        NOTHING_TO_PRINT,
        [
            "Nothing to print",
            "No hay nada que imprimir",
            "Nichts zu drucken",
            "Rien à imprimer",
            "Niente da stampare",
            "印刷するものがありません",
            "没有可打印的内容",
            "Não há nada para imprimir",
        ],
    ),
    (
        OUTPUT_REFUSED,
        [
            "The output may not be written there",
            "No se puede escribir la salida ahí",
            "Die Ausgabe darf dort nicht geschrieben werden",
            "La sortie ne peut pas être écrite là",
            "L'output non può essere scritto lì",
            "その場所には出力を書き込めません",
            "不能在此处写入输出",
            "A saída não pode ser escrita aí",
        ],
    ),
    (
        WRITE_FAILED,
        [
            "The file could not be written",
            "No se pudo escribir el archivo",
            "Die Datei konnte nicht geschrieben werden",
            "Le fichier n'a pas pu être écrit",
            "Impossibile scrivere il file",
            "ファイルを書き込めませんでした",
            "无法写入文件",
            "Não foi possível escrever o ficheiro",
        ],
    ),
    (
        TEXT_NOT_REPRESENTABLE,
        [
            "Text drawn as boxes in the PDF (use SVG)",
            "Texto dibujado como cajas en el PDF (use SVG)",
            "Text im PDF als Kästchen gezeichnet (SVG verwenden)",
            "Texte dessiné en cases dans le PDF (utiliser SVG)",
            "Testo disegnato come riquadri nel PDF (usare SVG)",
            "PDFでは文字が枠で描かれます（SVGを使用）",
            "PDF中的文字以方框绘制（请使用SVG）",
            "Texto desenhado como caixas no PDF (use SVG)",
        ],
    ),
    (
        PDF_CHECK_FAILED,
        [
            "The PDF written failed its own check",
            "El PDF escrito no pasó su propia comprobación",
            "Das geschriebene PDF hat die eigene Prüfung nicht bestanden",
            "Le PDF écrit a échoué à sa propre vérification",
            "Il PDF scritto non ha superato il proprio controllo",
            "書き出したPDFが自己検証に失敗しました",
            "生成的PDF未通过自检",
            "O PDF escrito falhou a sua própria verificação",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Print option is not valid",
            "La opción de impresión no es válida",
            "Die Druckoption ist ungültig",
            "L'option d'impression n'est pas valide",
            "L'opzione di stampa non è valida",
            "印刷オプションが正しくありません",
            "打印选项无效",
            "A opção de impressão não é válida",
        ],
    ),
];

/// The index of a language in the columns of [`TITLES`].
pub fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column(lang)])
}

/// An error of the printing: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintError {
    /// The code, `PHOENIX-PRINT-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case: a line number, a size or a path.
    pub detail: String,
}

impl PrintError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> PrintError {
        PrintError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let title = title(self.code, lang);
        if self.detail.is_empty() {
            format!("{}: {title}", self.code)
        } else {
            format!("{}: {title}: {}", self.code, self.detail)
        }
    }
}

impl std::fmt::Display for PrintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for PrintError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_each_language_and_the_english_one_is_the_registry() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for ((code, english), (tcode, row)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, tcode);
            assert_eq!(*english, row[0]);
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {i}");
            }
            assert!(code.starts_with("PHOENIX-PRINT-"));
        }
    }

    #[test]
    fn no_title_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES.iter() {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = PrintError::new(WRITE_FAILED, "disk full");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-PRINT-006: Não foi possível escrever o ficheiro: disk full"
        );
        assert_eq!(e.to_string(), "PHOENIX-PRINT-006: disk full");
        let bare = PrintError::new(NOTHING_TO_PRINT, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-PRINT-004: Nothing to print"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
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
