// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the reader of binary libraries, `PHOENIX-EAGLEBIN-<NNN>`, with their
//! titles in the eight languages of the suite, and the error type that carries one.
//!
//! Codes 001 to 005 and 012 stop the read; codes 006 to 011 are notes of the conversion report.

use eh3d_phoenix_rules::Lang;

/// The bytes do not start like a binary file of the old versions.
pub const NOT_BINARY: &str = "PHOENIX-EAGLEBIN-001";
/// The version that wrote the file is not 3, 4 or 5.
pub const VERSION: &str = "PHOENIX-EAGLEBIN-002";
/// A limit of size or count was exceeded.
pub const LIMIT: &str = "PHOENIX-EAGLEBIN-003";
/// The counts of the records do not agree with each other.
pub const STRUCTURE: &str = "PHOENIX-EAGLEBIN-004";
/// The file is binary and is not a library.
pub const NOT_LIBRARY: &str = "PHOENIX-EAGLEBIN-005";
/// The long strings do not match the fields that ask for them.
pub const STRINGS: &str = "PHOENIX-EAGLEBIN-006";
/// An object of the file was not converted.
pub const SKIPPED: &str = "PHOENIX-EAGLEBIN-007";
/// A value was not recognised and its default was used.
pub const VALUE: &str = "PHOENIX-EAGLEBIN-008";
/// A reference points to something that does not exist.
pub const REFERENCE: &str = "PHOENIX-EAGLEBIN-009";
/// A text was not UTF-8 and was read as Latin-1.
pub const ENCODING: &str = "PHOENIX-EAGLEBIN-010";
/// Data of the file that the reader does not interpret.
pub const NOT_READ: &str = "PHOENIX-EAGLEBIN-011";
/// The file is binary and is not a board.
pub const NOT_BOARD: &str = "PHOENIX-EAGLEBIN-012";

/// Every code with its titles in English, Spanish, German, French, Italian, Japanese, Chinese
/// and European Portuguese, in order.
pub const TITLES: [(&str, [&str; 8]); 12] = [
    (
        NOT_BINARY,
        [
            "Not an EAGLE binary file",
            "No es un archivo binario de EAGLE",
            "Keine binäre EAGLE-Datei",
            "Ce n'est pas un fichier binaire EAGLE",
            "Non è un file binario EAGLE",
            "EAGLE のバイナリファイルではありません",
            "不是 EAGLE 二进制文件",
            "Não é um ficheiro binário do EAGLE",
        ],
    ),
    (
        VERSION,
        [
            "Binary version outside 3 to 5",
            "Versión binaria fuera del rango 3 a 5",
            "Binäre Version außerhalb von 3 bis 5",
            "Version binaire hors de 3 à 5",
            "Versione binaria fuori da 3 a 5",
            "バイナリのバージョンが 3～5 の範囲外です",
            "二进制版本不在 3 到 5 之内",
            "Versão binária fora do intervalo de 3 a 5",
        ],
    ),
    (
        LIMIT,
        [
            "Size or count limit exceeded",
            "Se superó un límite de tamaño o de cantidad",
            "Grenze für Größe oder Anzahl überschritten",
            "Limite de taille ou de nombre dépassée",
            "Superato un limite di dimensione o di quantità",
            "サイズまたは個数の上限を超えました",
            "超出大小或数量上限",
            "Foi excedido um limite de tamanho ou de quantidade",
        ],
    ),
    (
        STRUCTURE,
        [
            "Record counts do not agree",
            "Los recuentos de registros no concuerdan",
            "Die Anzahlen der Datensätze passen nicht zusammen",
            "Les nombres d'enregistrements ne concordent pas",
            "I conteggi dei record non concordano",
            "レコード数が一致しません",
            "记录数量不一致",
            "As contagens de registos não coincidem",
        ],
    ),
    (
        NOT_LIBRARY,
        [
            "Binary file is not a library",
            "El archivo binario no es una biblioteca",
            "Die binäre Datei ist keine Bibliothek",
            "Le fichier binaire n'est pas une bibliothèque",
            "Il file binario non è una libreria",
            "バイナリファイルはライブラリではありません",
            "二进制文件不是库",
            "O ficheiro binário não é uma biblioteca",
        ],
    ),
    (
        STRINGS,
        [
            "Long strings do not match their fields",
            "Las cadenas largas no coinciden con sus campos",
            "Lange Zeichenketten passen nicht zu ihren Feldern",
            "Les longues chaînes ne correspondent pas à leurs champs",
            "Le stringhe lunghe non corrispondono ai loro campi",
            "長い文字列がフィールドと一致しません",
            "长字符串与其字段不匹配",
            "As cadeias longas não correspondem aos seus campos",
        ],
    ),
    (
        SKIPPED,
        [
            "Object not converted",
            "Objeto no convertido",
            "Objekt nicht umgewandelt",
            "Objet non converti",
            "Oggetto non convertito",
            "変換されなかったオブジェクト",
            "未转换的对象",
            "Objeto não convertido",
        ],
    ),
    (
        VALUE,
        [
            "Value not recognised, default used",
            "Valor no reconocido, se usa el predeterminado",
            "Wert nicht erkannt, Vorgabe verwendet",
            "Valeur non reconnue, valeur par défaut utilisée",
            "Valore non riconosciuto, usato il predefinito",
            "値を認識できないため既定値を使いました",
            "无法识别的值，已使用默认值",
            "Valor não reconhecido, foi usado o predefinido",
        ],
    ),
    (
        REFERENCE,
        [
            "Reference to something that does not exist",
            "Referencia a algo que no existe",
            "Verweis auf etwas, das es nicht gibt",
            "Référence à un élément inexistant",
            "Riferimento a qualcosa che non esiste",
            "存在しないものへの参照",
            "引用了不存在的对象",
            "Referência a algo que não existe",
        ],
    ),
    (
        ENCODING,
        [
            "Text read as Latin-1",
            "Texto leído como Latin-1",
            "Text als Latin-1 gelesen",
            "Texte lu en Latin-1",
            "Testo letto come Latin-1",
            "テキストを Latin-1 として読みました",
            "文本按 Latin-1 读取",
            "Texto lido como Latin-1",
        ],
    ),
    (
        NOT_READ,
        [
            "Data of the file not interpreted",
            "Datos del archivo no interpretados",
            "Daten der Datei nicht ausgewertet",
            "Données du fichier non interprétées",
            "Dati del file non interpretati",
            "解釈していないファイル内のデータ",
            "文件中未解释的数据",
            "Dados do ficheiro não interpretados",
        ],
    ),
    (
        NOT_BOARD,
        [
            "Binary file is not a board",
            "El archivo binario no es una placa",
            "Die binäre Datei ist keine Platine",
            "Le fichier binaire n'est pas un circuit imprimé",
            "Il file binario non è un circuito stampato",
            "バイナリファイルは基板ではありません",
            "二进制文件不是电路板",
            "O ficheiro binário não é uma placa",
        ],
    ),
];

/// The title of a code in a language; the code itself when it is not registered.
pub fn title(code: &str, lang: Lang) -> &str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    match TITLES.iter().find(|(c, _)| *c == code) {
        Some((_, titles)) => titles[column],
        None => code,
    }
}

/// Why a binary library could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinError {
    /// The code, `PHOENIX-EAGLEBIN-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case.
    pub detail: String,
}

impl BinError {
    /// An error of `code` with a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> BinError {
        BinError {
            code,
            detail: detail.into(),
        }
    }

    /// The code, the title in `lang` and the detail.
    pub fn message(&self, lang: Lang) -> String {
        format!("{} {}: {}", self.code, title(self.code, lang), self.detail)
    }
}

impl std::fmt::Display for BinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

impl std::error::Error for BinError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_are_numbered_in_order_with_eight_titles() {
        for (n, (code, titles)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-EAGLEBIN-{:03}", n + 1));
            assert!(titles.iter().all(|t| !t.is_empty()), "{code}");
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, titles) in TITLES {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(titles[0]), "{code}: {row}");
        }
    }

    #[test]
    fn a_message_has_the_code_the_title_and_the_detail() {
        let e = BinError::new(VERSION, "version 2.0");
        assert_eq!(
            e.to_string(),
            "PHOENIX-EAGLEBIN-002 Binary version outside 3 to 5: version 2.0"
        );
        assert!(e.message(Lang::Pt).contains("Versão"));
        assert_eq!(
            title("PHOENIX-EAGLEBIN-999", Lang::En),
            "PHOENIX-EAGLEBIN-999"
        );
    }
}
