// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the library sanitizer, `PHOENIX-SAN-<NNN>`, with their titles in the
//! eight languages of the suite, and the error type that carries one.

use eh3d_phoenix_rules::Lang;

/// The folder of libraries cannot be walked, or it holds more than the limits allow.
pub const FOLDER_UNUSABLE: &str = "PHOENIX-SAN-001";
/// A library file cannot be read.
pub const FILE_UNREADABLE: &str = "PHOENIX-SAN-002";
/// A library file is larger than the limit of the settings.
pub const FILE_TOO_LARGE: &str = "PHOENIX-SAN-003";
/// A file named as a library is not an EAGLE library that the reader accepts.
pub const NOT_A_LIBRARY: &str = "PHOENIX-SAN-004";
/// Items of a library were skipped (a missing or unusable coordinate).
pub const ITEM_SKIPPED: &str = "PHOENIX-SAN-005";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SAN-006";
/// The comparison stopped at the limit of the settings; later pairs were not compared.
pub const COMPARE_LIMIT: &str = "PHOENIX-SAN-007";
/// An output file cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-SAN-008";
/// Duplicates of the same geometry name their pads or pins differently.
pub const NAMES_DIFFER: &str = "PHOENIX-SAN-009";
/// A group holds a pair that is below the threshold and is joined to the rest by other pairs.
pub const WEAK_LINK: &str = "PHOENIX-SAN-010";
/// A name is used twice for the same kind of item in one library.
pub const DUPLICATE_NAME: &str = "PHOENIX-SAN-011";
/// An option of the command is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SAN-012";
/// The EAGLE reader reported problems in a library (a dangling reference, an unknown element).
pub const READER_WARNINGS: &str = "PHOENIX-SAN-013";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (FOLDER_UNUSABLE, "Library folder cannot be read"),
    (FILE_UNREADABLE, "Library file cannot be read"),
    (FILE_TOO_LARGE, "Library file is larger than the limit"),
    (
        NOT_A_LIBRARY,
        "File is not an EAGLE library that the reader accepts",
    ),
    (ITEM_SKIPPED, "Items of a library were skipped"),
    (SETTINGS_INVALID, "Sanitizer settings are not valid"),
    (COMPARE_LIMIT, "Comparison limit reached"),
    (OUTPUT_FAILED, "Output file cannot be written"),
    (
        NAMES_DIFFER,
        "Duplicates name their pads or pins differently",
    ),
    (WEAK_LINK, "Group joined by a weak match"),
    (DUPLICATE_NAME, "Name used twice in a library"),
    (OPTION_INVALID, "Command option is not valid"),
    (READER_WARNINGS, "The reader reported problems in a library"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: [(&str, [&str; 8]); 13] = [
    (
        FOLDER_UNUSABLE,
        [
            "Library folder cannot be read",
            "No se puede leer la carpeta de bibliotecas",
            "Der Bibliotheksordner kann nicht gelesen werden",
            "Le dossier des bibliothèques ne peut pas être lu",
            "La cartella delle librerie non può essere letta",
            "ライブラリのフォルダを読み込めません",
            "无法读取库文件夹",
            "Não é possível ler a pasta das bibliotecas",
        ],
    ),
    (
        FILE_UNREADABLE,
        [
            "Library file cannot be read",
            "No se puede leer el archivo de biblioteca",
            "Die Bibliotheksdatei kann nicht gelesen werden",
            "Le fichier de bibliothèque ne peut pas être lu",
            "Il file della libreria non può essere letto",
            "ライブラリファイルを読み込めません",
            "无法读取库文件",
            "Não é possível ler o ficheiro da biblioteca",
        ],
    ),
    (
        FILE_TOO_LARGE,
        [
            "Library file is larger than the limit",
            "El archivo de biblioteca supera el límite",
            "Die Bibliotheksdatei ist größer als die Grenze",
            "Le fichier de bibliothèque dépasse la limite",
            "Il file della libreria supera il limite",
            "ライブラリファイルが上限を超えています",
            "库文件超过了上限",
            "O ficheiro da biblioteca excede o limite",
        ],
    ),
    (
        NOT_A_LIBRARY,
        [
            "File is not an EAGLE library that the reader accepts",
            "El archivo no es una biblioteca EAGLE que el lector acepte",
            "Die Datei ist keine EAGLE-Bibliothek, die der Leser akzeptiert",
            "Le fichier n'est pas une bibliothèque EAGLE que le lecteur accepte",
            "Il file non è una libreria EAGLE che il lettore accetta",
            "このファイルはリーダーが受け付ける EAGLE ライブラリではありません",
            "该文件不是读取器接受的 EAGLE 库",
            "O ficheiro não é uma biblioteca EAGLE que o leitor aceite",
        ],
    ),
    (
        ITEM_SKIPPED,
        [
            "Items of a library were skipped",
            "Se omitieron elementos de una biblioteca",
            "Elemente einer Bibliothek wurden übersprungen",
            "Des éléments d'une bibliothèque ont été ignorés",
            "Alcuni elementi di una libreria sono stati saltati",
            "ライブラリの一部の要素をスキップしました",
            "已跳过库中的部分元素",
            "Foram ignorados elementos de uma biblioteca",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Sanitizer settings are not valid",
            "La configuración del saneador no es válida",
            "Die Einstellungen des Bereinigers sind ungültig",
            "Les réglages de l'assainisseur ne sont pas valides",
            "Le impostazioni del sanificatore non sono valide",
            "サニタイザーの設定が正しくありません",
            "清理器的设置无效",
            "As definições do saneador não são válidas",
        ],
    ),
    (
        COMPARE_LIMIT,
        [
            "Comparison limit reached",
            "Se alcanzó el límite de comparaciones",
            "Die Vergleichsgrenze wurde erreicht",
            "La limite de comparaisons est atteinte",
            "Raggiunto il limite dei confronti",
            "比較数の上限に達しました",
            "已达到比较次数上限",
            "Foi atingido o limite de comparações",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Output file cannot be written",
            "No se puede escribir el archivo de salida",
            "Die Ausgabedatei kann nicht geschrieben werden",
            "Le fichier de sortie ne peut pas être écrit",
            "Il file di uscita non può essere scritto",
            "出力ファイルを書き込めません",
            "无法写入输出文件",
            "Não é possível escrever o ficheiro de saída",
        ],
    ),
    (
        NAMES_DIFFER,
        [
            "Duplicates name their pads or pins differently",
            "Los duplicados nombran sus pads o pines de forma distinta",
            "Die Duplikate benennen ihre Pads oder Pins unterschiedlich",
            "Les doublons nomment leurs pastilles ou broches différemment",
            "I duplicati chiamano in modo diverso i pad o i pin",
            "重複するアイテムでパッドまたはピンの名前が異なります",
            "重复项的焊盘或引脚名称不同",
            "Os duplicados nomeiam de forma diferente as suas pastilhas ou pinos",
        ],
    ),
    (
        WEAK_LINK,
        [
            "Group joined by a weak match",
            "Grupo unido por una coincidencia débil",
            "Gruppe durch eine schwache Übereinstimmung verbunden",
            "Groupe relié par une correspondance faible",
            "Gruppo unito da una corrispondenza debole",
            "弱い一致で結ばれたグループ",
            "由弱匹配连接的分组",
            "Grupo ligado por uma correspondência fraca",
        ],
    ),
    (
        DUPLICATE_NAME,
        [
            "Name used twice in a library",
            "Nombre usado dos veces en una biblioteca",
            "Name in einer Bibliothek doppelt verwendet",
            "Nom utilisé deux fois dans une bibliothèque",
            "Nome usato due volte in una libreria",
            "ライブラリ内で名前が重複しています",
            "库中名称被使用了两次",
            "Nome usado duas vezes numa biblioteca",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "La opción del comando no es válida",
            "Die Option des Befehls ist ungültig",
            "L'option de la commande n'est pas valide",
            "L'opzione del comando non è valida",
            "コマンドのオプションが正しくありません",
            "命令选项无效",
            "A opção do comando não é válida",
        ],
    ),
    (
        READER_WARNINGS,
        [
            "The reader reported problems in a library",
            "El lector informó de problemas en una biblioteca",
            "Der Leser meldete Probleme in einer Bibliothek",
            "Le lecteur a signalé des problèmes dans une bibliothèque",
            "Il lettore ha segnalato problemi in una libreria",
            "リーダーがライブラリの問題を報告しました",
            "读取器报告了库中的问题",
            "O leitor comunicou problemas numa biblioteca",
        ],
    ),
];

/// The column of a language in the title table.
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

/// An error of the sanitizer: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {detail}")]
pub struct SanError {
    /// The code, `PHOENIX-SAN-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case: a line number, a size or the text of a reader.
    pub detail: String,
}

impl SanError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SanError {
        SanError {
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
            assert!(code.starts_with("PHOENIX-SAN-"));
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
        let e = SanError::new(FILE_UNREADABLE, "denied");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-SAN-002: Não é possível ler o ficheiro da biblioteca: denied"
        );
        assert_eq!(e.to_string(), "PHOENIX-SAN-002: denied");
        let bare = SanError::new(COMPARE_LIMIT, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-SAN-007: Comparison limit reached"
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
