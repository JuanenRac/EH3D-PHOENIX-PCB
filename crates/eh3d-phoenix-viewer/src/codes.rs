// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the viewer, `PHOENIX-VIEW-<NNN>`, with their titles in the eight
//! languages of the suite, and the error type that carries one.

use eh3d_phoenix_rules::Lang;

/// A file that cannot be read or is too large.
pub const FILE_UNUSABLE: &str = "PHOENIX-VIEW-001";
/// A file that is neither a board nor a schematic that the viewer reads.
pub const FILE_NOT_RECOGNISED: &str = "PHOENIX-VIEW-002";
/// An EAGLE file that the reader refuses.
pub const EAGLE_UNREADABLE: &str = "PHOENIX-VIEW-003";
/// A document that has nothing to draw.
pub const NOTHING_TO_DRAW: &str = "PHOENIX-VIEW-004";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-VIEW-005";
/// A scene with more primitives than the settings allow.
pub const TOO_MANY_PRIMITIVES: &str = "PHOENIX-VIEW-006";
/// A benchmark option that is not valid.
pub const BENCH_INVALID: &str = "PHOENIX-VIEW-007";
/// The window was asked for in a build without the `gui` feature.
pub const WINDOW_NOT_BUILT: &str = "PHOENIX-VIEW-008";
/// The window could not be opened.
pub const WINDOW_FAILED: &str = "PHOENIX-VIEW-009";
/// An edit that could not be made.
pub const EDIT_FAILED: &str = "PHOENIX-VIEW-010";
/// A file that may not be written where it was asked.
pub const SAVE_REFUSED: &str = "PHOENIX-VIEW-011";
/// A file that could not be written.
pub const SAVE_FAILED: &str = "PHOENIX-VIEW-012";
/// A rule file or a settings file of a check that cannot be used.
pub const CHECK_FILE_UNUSABLE: &str = "PHOENIX-VIEW-013";
/// A panel that cannot be laid out from the boards and the settings that were given.
pub const PANEL_UNUSABLE: &str = "PHOENIX-VIEW-014";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (FILE_UNUSABLE, "File cannot be read"),
    (
        FILE_NOT_RECOGNISED,
        "File is not a board or a schematic that the viewer reads",
    ),
    (EAGLE_UNREADABLE, "EAGLE file cannot be read"),
    (NOTHING_TO_DRAW, "Nothing to draw"),
    (SETTINGS_INVALID, "Viewer settings are not valid"),
    (TOO_MANY_PRIMITIVES, "Too many drawing primitives"),
    (BENCH_INVALID, "Benchmark option is not valid"),
    (WINDOW_NOT_BUILT, "The window is not built in"),
    (WINDOW_FAILED, "The window could not be opened"),
    (EDIT_FAILED, "The edit could not be made"),
    (SAVE_REFUSED, "The file may not be written there"),
    (SAVE_FAILED, "The file could not be written"),
    (CHECK_FILE_UNUSABLE, "The rule file cannot be used"),
    (PANEL_UNUSABLE, "The panel cannot be laid out"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: [(&str, [&str; 8]); 14] = [
    (
        FILE_UNUSABLE,
        [
            "File cannot be read",
            "No se puede leer el archivo",
            "Datei kann nicht gelesen werden",
            "Le fichier ne peut pas être lu",
            "Il file non può essere letto",
            "ファイルを読み込めません",
            "无法读取文件",
            "Não é possível ler o ficheiro",
        ],
    ),
    (
        FILE_NOT_RECOGNISED,
        [
            "File is not a board or a schematic that the viewer reads",
            "El archivo no es una placa ni un esquema que el visor lea",
            "Die Datei ist keine Platine und kein Schaltplan, den der Betrachter liest",
            "Le fichier n'est ni une carte ni un schéma que la visionneuse lit",
            "Il file non è una scheda né uno schema che il visualizzatore legge",
            "このファイルはビューアが読める基板でも回路図でもありません",
            "该文件不是查看器可读取的电路板或原理图",
            "O ficheiro não é uma placa nem um esquema que o visualizador leia",
        ],
    ),
    (
        EAGLE_UNREADABLE,
        [
            "EAGLE file cannot be read",
            "No se puede leer el archivo EAGLE",
            "EAGLE-Datei kann nicht gelesen werden",
            "Le fichier EAGLE ne peut pas être lu",
            "Il file EAGLE non può essere letto",
            "EAGLE ファイルを読み込めません",
            "无法读取 EAGLE 文件",
            "Não é possível ler o ficheiro EAGLE",
        ],
    ),
    (
        NOTHING_TO_DRAW,
        [
            "Nothing to draw",
            "No hay nada que dibujar",
            "Nichts zu zeichnen",
            "Rien à dessiner",
            "Niente da disegnare",
            "描画するものがありません",
            "没有可绘制的内容",
            "Nada para desenhar",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Viewer settings are not valid",
            "Los ajustes del visor no son válidos",
            "Die Einstellungen des Betrachters sind ungültig",
            "Les réglages de la visionneuse ne sont pas valides",
            "Le impostazioni del visualizzatore non sono valide",
            "ビューアの設定が正しくありません",
            "查看器设置无效",
            "As definições do visualizador não são válidas",
        ],
    ),
    (
        TOO_MANY_PRIMITIVES,
        [
            "Too many drawing primitives",
            "Demasiadas primitivas de dibujo",
            "Zu viele Zeichenprimitive",
            "Trop de primitives de dessin",
            "Troppe primitive di disegno",
            "描画プリミティブが多すぎます",
            "绘图图元过多",
            "Demasiadas primitivas de desenho",
        ],
    ),
    (
        BENCH_INVALID,
        [
            "Benchmark option is not valid",
            "La opción del banco de pruebas no es válida",
            "Option des Leistungstests ist ungültig",
            "L'option du banc d'essai n'est pas valide",
            "L'opzione del benchmark non è valida",
            "ベンチマークのオプションが正しくありません",
            "基准测试选项无效",
            "A opção do teste de desempenho não é válida",
        ],
    ),
    (
        WINDOW_NOT_BUILT,
        [
            "The window is not built in",
            "La ventana no está incluida en esta compilación",
            "Das Fenster ist nicht eingebaut",
            "La fenêtre n'est pas incluse dans cette compilation",
            "La finestra non è inclusa in questa compilazione",
            "このビルドにはウィンドウが含まれていません",
            "此构建未包含窗口",
            "A janela não está incluída nesta compilação",
        ],
    ),
    (
        WINDOW_FAILED,
        [
            "The window could not be opened",
            "No se pudo abrir la ventana",
            "Das Fenster konnte nicht geöffnet werden",
            "La fenêtre n'a pas pu être ouverte",
            "Impossibile aprire la finestra",
            "ウィンドウを開けませんでした",
            "无法打开窗口",
            "Não foi possível abrir a janela",
        ],
    ),
    (
        EDIT_FAILED,
        [
            "The edit could not be made",
            "No se pudo hacer la edición",
            "Die Bearbeitung konnte nicht ausgeführt werden",
            "La modification n'a pas pu être faite",
            "Non è stato possibile eseguire la modifica",
            "編集を実行できませんでした",
            "无法进行编辑",
            "Não foi possível fazer a edição",
        ],
    ),
    (
        SAVE_REFUSED,
        [
            "The file may not be written there",
            "No se puede escribir el archivo ahí",
            "Die Datei darf dort nicht geschrieben werden",
            "Le fichier ne peut pas être écrit à cet endroit",
            "Il file non può essere scritto lì",
            "その場所にはファイルを書き込めません",
            "不能在该位置写入文件",
            "O ficheiro não pode ser escrito aí",
        ],
    ),
    (
        SAVE_FAILED,
        [
            "The file could not be written",
            "No se pudo escribir el archivo",
            "Die Datei konnte nicht geschrieben werden",
            "Le fichier n'a pas pu être écrit",
            "Non è stato possibile scrivere il file",
            "ファイルを書き込めませんでした",
            "无法写入文件",
            "Não foi possível escrever o ficheiro",
        ],
    ),
    (
        CHECK_FILE_UNUSABLE,
        [
            "The rule file cannot be used",
            "No se puede usar el archivo de reglas",
            "Die Regeldatei kann nicht verwendet werden",
            "Le fichier de règles ne peut pas être utilisé",
            "Il file delle regole non può essere usato",
            "ルールファイルを使用できません",
            "无法使用规则文件",
            "Não é possível usar o ficheiro de regras",
        ],
    ),
    (
        PANEL_UNUSABLE,
        [
            "The panel cannot be laid out",
            "No se puede componer el panel",
            "Der Nutzen kann nicht angelegt werden",
            "Le panneau ne peut pas être composé",
            "Il pannello non può essere composto",
            "パネルを構成できません",
            "无法生成拼板",
            "Não é possível compor o painel",
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

/// An error of the viewer: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {detail}")]
pub struct ViewError {
    /// The code, `PHOENIX-VIEW-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case: a line number, a size or the text of a reader.
    pub detail: String,
}

impl ViewError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> ViewError {
        ViewError {
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
            assert!(code.starts_with("PHOENIX-VIEW-"));
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
        let e = ViewError::new(FILE_UNUSABLE, "too large");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-VIEW-001: Não é possível ler o ficheiro: too large"
        );
        assert_eq!(e.to_string(), "PHOENIX-VIEW-001: too large");
        let bare = ViewError::new(NOTHING_TO_DRAW, "");
        assert_eq!(bare.message(Lang::En), "PHOENIX-VIEW-004: Nothing to draw");
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
