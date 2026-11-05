// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the reference cases, `PHOENIX-ORACLE-<NNN>`, with their titles in the
//! eight languages of the suite, the words of the verdicts, and the error type that carries a code.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// A case file that is not valid.
pub const FILE_INVALID: &str = "PHOENIX-ORACLE-001";
/// A case that is not valid.
pub const CASE_INVALID: &str = "PHOENIX-ORACLE-002";
/// A check that is not known.
pub const CHECK_UNKNOWN: &str = "PHOENIX-ORACLE-003";
/// An input of a case that is missing or not valid.
pub const INPUT_INVALID: &str = "PHOENIX-ORACLE-004";
/// The calculation under test failed.
pub const CALC_FAILED: &str = "PHOENIX-ORACLE-005";
/// The result is outside the tolerance of the reference.
pub const OUT_OF_TOLERANCE: &str = "PHOENIX-ORACLE-006";
/// The case is pending.
pub const CASE_PENDING: &str = "PHOENIX-ORACLE-007";
/// A case file or folder that cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-ORACLE-008";
/// A command option that is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-ORACLE-009";
/// A report file that cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-ORACLE-010";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        FILE_INVALID,
        [
            "Oracle case file is not valid",
            "El archivo de casos del oráculo no es válido",
            "Die Datei mit den Orakel-Fällen ist ungültig",
            "Le fichier de cas de l'oracle n'est pas valide",
            "Il file dei casi dell'oracolo non è valido",
            "オラクルのケースファイルが無効です",
            "基准用例文件无效",
            "O ficheiro de casos do oráculo não é válido",
        ],
    ),
    (
        CASE_INVALID,
        [
            "Oracle case is not valid",
            "El caso del oráculo no es válido",
            "Der Orakel-Fall ist ungültig",
            "Le cas de l'oracle n'est pas valide",
            "Il caso dell'oracolo non è valido",
            "オラクルのケースが無効です",
            "基准用例无效",
            "O caso do oráculo não é válido",
        ],
    ),
    (
        CHECK_UNKNOWN,
        [
            "Oracle check is not known",
            "La comprobación del oráculo no se conoce",
            "Die Orakel-Prüfung ist unbekannt",
            "Le contrôle de l'oracle est inconnu",
            "Il controllo dell'oracolo non è noto",
            "オラクルのチェックが不明です",
            "未知的基准检查",
            "A verificação do oráculo não é conhecida",
        ],
    ),
    (
        INPUT_INVALID,
        [
            "Input of an oracle case is missing or not valid",
            "Falta un dato de entrada del caso o no es válido",
            "Eine Eingabe des Falls fehlt oder ist ungültig",
            "Une entrée du cas manque ou n'est pas valide",
            "Un ingresso del caso manca o non è valido",
            "ケースの入力が不足または無効です",
            "用例的输入缺失或无效",
            "Falta uma entrada do caso ou não é válida",
        ],
    ),
    (
        CALC_FAILED,
        [
            "The calculation under test failed",
            "Falló el cálculo que se prueba",
            "Die geprüfte Berechnung ist fehlgeschlagen",
            "Le calcul testé a échoué",
            "Il calcolo in prova è fallito",
            "テスト対象の計算が失敗しました",
            "被测计算失败",
            "O cálculo em teste falhou",
        ],
    ),
    (
        OUT_OF_TOLERANCE,
        [
            "Result is outside the tolerance of the reference",
            "El resultado está fuera de la tolerancia de la referencia",
            "Das Ergebnis liegt außerhalb der Toleranz der Referenz",
            "Le résultat est hors de la tolérance de la référence",
            "Il risultato è fuori dalla tolleranza del riferimento",
            "結果が基準の許容範囲外です",
            "结果超出参考值的容差",
            "O resultado está fora da tolerância da referência",
        ],
    ),
    (
        CASE_PENDING,
        [
            "Case is pending",
            "El caso está pendiente",
            "Der Fall ist ausstehend",
            "Le cas est en attente",
            "Il caso è in sospeso",
            "ケースは保留中です",
            "用例待处理",
            "O caso está pendente",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Case file or folder cannot be read",
            "No se puede leer el archivo o la carpeta de casos",
            "Die Falldatei oder der Ordner kann nicht gelesen werden",
            "Le fichier ou le dossier de cas ne peut pas être lu",
            "Il file o la cartella dei casi non può essere letto",
            "ケースのファイルまたはフォルダを読み込めません",
            "无法读取用例文件或文件夹",
            "Não é possível ler o ficheiro ou a pasta de casos",
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
            "コマンドのオプションが無効です",
            "命令选项无效",
            "A opção do comando não é válida",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Report file cannot be written",
            "No se puede escribir el archivo del informe",
            "Die Berichtsdatei kann nicht geschrieben werden",
            "Le fichier du rapport ne peut pas être écrit",
            "Il file del rapporto non può essere scritto",
            "レポートファイルを書き込めません",
            "无法写入报告文件",
            "Não é possível escrever o ficheiro do relatório",
        ],
    ),
];

/// The words of the four verdicts, in the order ok, outside the tolerance, pending and failed,
/// in the same eight languages.
pub const WORDS: [[&str; 8]; 4] = [
    [
        "ok",
        "correcto",
        "bestanden",
        "conforme",
        "conforme",
        "合格",
        "通过",
        "conforme",
    ],
    [
        "outside tolerance",
        "fuera de tolerancia",
        "außerhalb der Toleranz",
        "hors tolérance",
        "fuori tolleranza",
        "許容範囲外",
        "超出容差",
        "fora da tolerância",
    ],
    [
        "pending",
        "pendiente",
        "ausstehend",
        "en attente",
        "in sospeso",
        "保留",
        "待处理",
        "pendente",
    ],
    [
        "failed",
        "fallido",
        "fehlgeschlagen",
        "échoué",
        "fallito",
        "失敗",
        "失败",
        "falhou",
    ],
];

/// The column of a language in the tables.
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

/// An error of the reference cases: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleError {
    /// The code, `PHOENIX-ORACLE-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl OracleError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> OracleError {
        OracleError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for OracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for OracleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ORACLE-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
    }

    #[test]
    fn no_title_or_word_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        for word in WORDS {
            for text in &word[1..] {
                assert_ne!(*text, word[0]);
            }
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = OracleError::new(CHECK_UNKNOWN, "nothing_here");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-ORACLE-003: A verificação do oráculo não é conhecida: nothing_here"
        );
        assert_eq!(e.to_string(), "PHOENIX-ORACLE-003: nothing_here");
        let bare = OracleError::new(CASE_PENDING, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-ORACLE-007: Case is pending"
        );
        assert_eq!(title("PHOENIX-ORACLE-999", Lang::En), "");
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
            // a second row of the same code holds the title in the eight languages
            let all_eight = doc
                .lines()
                .filter(|l| l.starts_with(&format!("| {code} |")))
                .any(|l| row.iter().all(|text| l.contains(text)));
            assert!(all_eight, "{code} is not documented in the eight languages");
        }
    }
}
