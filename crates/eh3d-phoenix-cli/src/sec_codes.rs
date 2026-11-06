// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the checks on files and paths that the command line makes,
//! `PHOENIX-SEC-<NNN>`, with their titles in the eight languages of the suite.

use eh3d_phoenix_rules::Lang;

/// A path that a command would write is refused: a device or verbatim path, a path of another
/// computer, an alternate data stream or a part named like a Windows device.
pub const OUTPUT_PATH_REFUSED: &str = "PHOENIX-SEC-001";
/// The side file of an atomic write is already there, so it is not written over.
pub const SIDE_FILE_EXISTS: &str = "PHOENIX-SEC-002";
/// A settings file is larger than the limit for settings files.
pub const SETTINGS_TOO_LARGE: &str = "PHOENIX-SEC-003";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        OUTPUT_PATH_REFUSED,
        [
            "Output path is refused",
            "Se rechaza la ruta de salida",
            "Der Ausgabepfad wird abgelehnt",
            "Le chemin de sortie est refusé",
            "Il percorso di uscita è rifiutato",
            "出力パスは拒否されました",
            "输出路径被拒绝",
            "O caminho de saída é recusado",
        ],
    ),
    (
        SIDE_FILE_EXISTS,
        [
            "The side file of the write is already there",
            "Ya existe el fichero auxiliar de la escritura",
            "Die Hilfsdatei des Schreibvorgangs ist bereits vorhanden",
            "Le fichier auxiliaire de l'écriture existe déjà",
            "Il file ausiliario della scrittura esiste già",
            "書き込み用の補助ファイルが既に存在します",
            "写入用的辅助文件已存在",
            "O ficheiro auxiliar da escrita já existe",
        ],
    ),
    (
        SETTINGS_TOO_LARGE,
        [
            "Settings file is larger than its limit",
            "El fichero de ajustes supera su límite",
            "Die Einstellungsdatei ist größer als ihr Grenzwert",
            "Le fichier de réglages dépasse sa limite",
            "Il file delle impostazioni supera il suo limite",
            "設定ファイルが上限を超えています",
            "设置文件超过其限制",
            "O ficheiro de definições excede o seu limite",
        ],
    ),
];

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SEC-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(title("PHOENIX-SEC-999", Lang::En), "");
        assert_eq!(
            title(OUTPUT_PATH_REFUSED, Lang::Pt),
            "O caminho de saída é recusado"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
            let translated = doc
                .lines()
                .filter(|l| l.starts_with(&format!("| {code} |")))
                .any(|l| row.iter().all(|t| l.contains(t)));
            assert!(translated, "{code} is not in the table of the languages");
        }
    }
}
