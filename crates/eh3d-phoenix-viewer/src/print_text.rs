// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the export of images and PDF in the window, in the eight languages of the suite:
//! English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintMsg {
    /// The button that exports.
    Export,
    /// The label of the box with the place to write to.
    Target,
    /// The start of the note after the files were written.
    Written,
}

impl PrintMsg {
    /// Every text, for the tests that check the table.
    pub const ALL: [PrintMsg; 3] = [PrintMsg::Export, PrintMsg::Target, PrintMsg::Written];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            PrintMsg::Export => [
                "Export image/PDF",
                "Exportar imagen/PDF",
                "Bild/PDF exportieren",
                "Exporter image/PDF",
                "Esporta immagine/PDF",
                "画像/PDFを書き出す",
                "导出图像/PDF",
                "Exportar imagem/PDF",
            ],
            PrintMsg::Target => [
                "Print to (a new file or folder)",
                "Imprimir en (archivo o carpeta nuevos)",
                "Ausgabe nach (neue Datei oder neuer Ordner)",
                "Imprimer vers (nouveau fichier ou dossier)",
                "Stampa in (nuovo file o nuova cartella)",
                "出力先（新しいファイルまたはフォルダ）",
                "输出到（新文件或文件夹）",
                "Imprimir para (novo ficheiro ou pasta)",
            ],
            PrintMsg::Written => [
                "Written",
                "Escrito",
                "Geschrieben",
                "Écrit",
                "Scritto",
                "書き出しました",
                "已写入",
                "Escrito",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_each_language() {
        for text in PrintMsg::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
        assert_eq!(PrintMsg::Export.tr(Lang::Pt), "Exportar imagem/PDF");
        assert_eq!(
            PrintMsg::Target.tr(Lang::Pt),
            "Imprimir para (novo ficheiro ou pasta)"
        );
    }
}
