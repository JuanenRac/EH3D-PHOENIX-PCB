// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-SPATIAL-<NNN>` of the spatial index, with their titles in the eight
//! languages of the suite and the error that carries one.

use std::fmt;

/// The settings of the index are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SPATIAL-001";
/// The grid would need more cells than the limit.
pub const GRID_TOO_LARGE: &str = "PHOENIX-SPATIAL-002";
/// There are more items than the index accepts.
pub const TOO_MANY_ITEMS: &str = "PHOENIX-SPATIAL-003";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SPATIAL-004";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-SPATIAL-005";

/// The language tags, in the order of the columns of [`TITLES`].
pub const LANGUAGES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Spatial index settings are not valid",
            "La configuración del índice espacial no es válida",
            "Die Einstellungen des räumlichen Index sind ungültig",
            "Les réglages de l'index spatial ne sont pas valides",
            "Le impostazioni dell'indice spaziale non sono valide",
            "空間インデックスの設定が無効です",
            "空间索引设置无效",
            "As definições do índice espacial não são válidas",
        ],
    ),
    (
        GRID_TOO_LARGE,
        [
            "The spatial index would need more cells than the limit",
            "El índice espacial necesitaría más celdas que el límite",
            "Der räumliche Index würde mehr Zellen brauchen als das Limit erlaubt",
            "L'index spatial demanderait plus de cellules que la limite",
            "L'indice spaziale richiederebbe più celle del limite",
            "空間インデックスのセル数が上限を超えます",
            "空间索引所需的单元格超过上限",
            "O índice espacial precisaria de mais células do que o limite",
        ],
    ),
    (
        TOO_MANY_ITEMS,
        [
            "There are more items than the spatial index accepts",
            "Hay más objetos de los que acepta el índice espacial",
            "Es gibt mehr Objekte, als der räumliche Index annimmt",
            "Il y a plus d'objets que l'index spatial n'en accepte",
            "Ci sono più oggetti di quanti ne accetti l'indice spaziale",
            "空間インデックスが受け付ける数より多くのオブジェクトがあります",
            "对象数量超过空间索引的上限",
            "Há mais objetos do que o índice espacial aceita",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Input file cannot be read",
            "No se puede leer el fichero de entrada",
            "Die Eingabedatei kann nicht gelesen werden",
            "Le fichier d'entrée ne peut pas être lu",
            "Il file di ingresso non può essere letto",
            "入力ファイルを読み取れません",
            "无法读取输入文件",
            "O ficheiro de entrada não pode ser lido",
        ],
    ),
];

/// The codes of this crate with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in the language with this tag (`en` when the tag is not known); empty for a
/// code that is not in the table.
pub fn title(code: &str, tag: &str) -> &'static str {
    let column = LANGUAGES.iter().position(|t| *t == tag).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// An error with its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialError {
    /// The code.
    pub code: &'static str,
    /// The detail, in English.
    pub detail: String,
}

impl SpatialError {
    /// An error.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SpatialError {
        SpatialError {
            code,
            detail: detail.into(),
        }
    }

    /// The error as a message, with the title in the language with this tag.
    pub fn message(&self, tag: &str) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, tag));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for SpatialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SpatialError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SPATIAL-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(registry().len(), 5);
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = SpatialError::new(GRID_TOO_LARGE, "5 cells");
        assert_eq!(
            e.message("pt"),
            "PHOENIX-SPATIAL-002: O índice espacial precisaria de mais células do que o limite: 5 cells"
        );
        assert_eq!(e.to_string(), "PHOENIX-SPATIAL-002: 5 cells");
        assert_eq!(title("PHOENIX-SPATIAL-999", "en"), "");
        assert_eq!(
            title(SETTINGS_INVALID, "xx"),
            "Spatial index settings are not valid"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
    }
}
