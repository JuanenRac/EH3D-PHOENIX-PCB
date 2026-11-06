// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the own corpus, `PHOENIX-CORPUS-<NNN>`, with their titles in the eight
//! languages of the suite, the words of the verdicts, and the error type that carries a code.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The folder of the corpus cannot be read.
pub const FOLDER_UNREADABLE: &str = "PHOENIX-CORPUS-001";
/// An expectation file is not valid.
pub const EXPECTATION_INVALID: &str = "PHOENIX-CORPUS-002";
/// The corpus is not consistent.
pub const CORPUS_INCONSISTENT: &str = "PHOENIX-CORPUS-003";
/// The result differs from the expectation.
pub const DIFFERENCE: &str = "PHOENIX-CORPUS-004";
/// A reader or writer failed where success was expected.
pub const UNEXPECTED_FAILURE: &str = "PHOENIX-CORPUS-005";
/// The file was refused, as the expectation says.
pub const REFUSAL_EXPECTED: &str = "PHOENIX-CORPUS-006";
/// A round trip lost something that the writer says it keeps.
pub const ROUND_TRIP_LOSS: &str = "PHOENIX-CORPUS-007";
/// A property of the formats does not hold for a seed.
pub const PROPERTY_BROKEN: &str = "PHOENIX-CORPUS-008";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-CORPUS-009";
/// The report file cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-CORPUS-010";
/// The settings of the corpus are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-CORPUS-011";
/// A corpus file is larger than the limit.
pub const FILE_TOO_LARGE: &str = "PHOENIX-CORPUS-012";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        FOLDER_UNREADABLE,
        [
            "The folder of the corpus cannot be read",
            "No se puede leer la carpeta del corpus",
            "Der Ordner des Korpus kann nicht gelesen werden",
            "Le dossier du corpus ne peut pas être lu",
            "La cartella del corpus non può essere letta",
            "コーパスのフォルダを読み取れません",
            "无法读取语料库文件夹",
            "Não é possível ler a pasta do corpus",
        ],
    ),
    (
        EXPECTATION_INVALID,
        [
            "An expectation file is not valid",
            "Un archivo de expectativas no es válido",
            "Eine Erwartungsdatei ist ungültig",
            "Un fichier d'attentes n'est pas valide",
            "Un file delle attese non è valido",
            "期待値ファイルが無効です",
            "预期文件无效",
            "Um ficheiro de expectativas não é válido",
        ],
    ),
    (
        CORPUS_INCONSISTENT,
        [
            "The corpus is not consistent",
            "El corpus no es coherente",
            "Der Korpus ist nicht konsistent",
            "Le corpus n'est pas cohérent",
            "Il corpus non è coerente",
            "コーパスに矛盾があります",
            "语料库不一致",
            "O corpus não é coerente",
        ],
    ),
    (
        DIFFERENCE,
        [
            "The result differs from the expectation",
            "El resultado difiere de la expectativa",
            "Das Ergebnis weicht von der Erwartung ab",
            "Le résultat diffère de l'attente",
            "Il risultato differisce dall'attesa",
            "結果が期待値と異なります",
            "结果与预期不同",
            "O resultado difere da expectativa",
        ],
    ),
    (
        UNEXPECTED_FAILURE,
        [
            "A reader or writer failed where success was expected",
            "Un lector o escritor falló donde se esperaba éxito",
            "Ein Leser oder Schreiber ist gescheitert, wo Erfolg erwartet wurde",
            "Un lecteur ou un écrivain a échoué là où le succès était attendu",
            "Un lettore o scrittore ha fallito dove era atteso il successo",
            "成功が期待される場所でリーダーまたはライターが失敗しました",
            "读取器或写入器在预期成功之处失败",
            "Um leitor ou escritor falhou onde se esperava sucesso",
        ],
    ),
    (
        REFUSAL_EXPECTED,
        [
            "The file was refused, as the expectation says",
            "El archivo fue rechazado, como dice la expectativa",
            "Die Datei wurde abgelehnt, wie die Erwartung es sagt",
            "Le fichier a été refusé, comme l'attente le dit",
            "Il file è stato rifiutato, come dice l'attesa",
            "期待どおりファイルは拒否されました",
            "文件按预期被拒绝",
            "O ficheiro foi recusado, como a expectativa indica",
        ],
    ),
    (
        ROUND_TRIP_LOSS,
        [
            "A round trip lost something that the writer says it keeps",
            "Una ida y vuelta perdió algo que el escritor dice conservar",
            "Ein Hin- und Rückweg hat etwas verloren, das der Schreiber zu erhalten angibt",
            "Un aller-retour a perdu ce que l'écrivain dit conserver",
            "Un andata e ritorno ha perso qualcosa che lo scrittore dichiara di conservare",
            "往復変換で、ライターが保持すると述べたものが失われました",
            "往返转换丢失了写入器声明保留的内容",
            "Uma ida e volta perdeu algo que o escritor diz conservar",
        ],
    ),
    (
        PROPERTY_BROKEN,
        [
            "A property of the formats does not hold for a seed",
            "Una propiedad de los formatos no se cumple para una semilla",
            "Eine Eigenschaft der Formate gilt für einen Startwert nicht",
            "Une propriété des formats n'est pas vérifiée pour une graine",
            "Una proprietà dei formati non vale per un seme",
            "あるシードでフォーマットの性質が成り立ちません",
            "某个种子下格式的性质不成立",
            "Uma propriedade dos formatos não se verifica para uma semente",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Befehlsoption ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "The report file cannot be written",
            "No se puede escribir el archivo del informe",
            "Die Berichtsdatei kann nicht geschrieben werden",
            "Le fichier du rapport ne peut pas être écrit",
            "Il file del rapporto non può essere scritto",
            "レポートファイルを書き込めません",
            "无法写入报告文件",
            "Não é possível escrever o ficheiro do relatório",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "The corpus settings are not valid",
            "La configuración del corpus no es válida",
            "Die Einstellungen des Korpus sind ungültig",
            "Les réglages du corpus ne sont pas valides",
            "Le impostazioni del corpus non sono valide",
            "コーパスの設定が無効です",
            "语料库设置无效",
            "As definições do corpus não são válidas",
        ],
    ),
    (
        FILE_TOO_LARGE,
        [
            "A corpus file is larger than the limit",
            "Un archivo del corpus supera el límite",
            "Eine Korpusdatei ist größer als das Limit",
            "Un fichier du corpus dépasse la limite",
            "Un file del corpus supera il limite",
            "コーパスのファイルが上限を超えています",
            "语料库文件超过限制",
            "Um ficheiro do corpus excede o limite",
        ],
    ),
];

/// The words of the four verdicts, in the order ok, difference, expected error and failed, in
/// the same eight languages.
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
        "difference",
        "diferencia",
        "Abweichung",
        "différence",
        "differenza",
        "差異",
        "差异",
        "diferença",
    ],
    [
        "expected error",
        "error esperado",
        "erwarteter Fehler",
        "erreur attendue",
        "errore atteso",
        "想定どおりのエラー",
        "预期错误",
        "erro esperado",
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

/// An error of the own corpus: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteError {
    /// The code, `PHOENIX-CORPUS-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl SuiteError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SuiteError {
        SuiteError {
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

impl fmt::Display for SuiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SuiteError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-CORPUS-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
        assert_eq!(TITLES.len(), 12);
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
        let e = SuiteError::new(CORPUS_INCONSISTENT, "a.brd has no expectation");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-CORPUS-003: O corpus não é coerente: a.brd has no expectation"
        );
        assert_eq!(
            e.to_string(),
            "PHOENIX-CORPUS-003: a.brd has no expectation"
        );
        let bare = SuiteError::new(REFUSAL_EXPECTED, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-CORPUS-006: The file was refused, as the expectation says"
        );
        assert_eq!(title("PHOENIX-CORPUS-999", Lang::En), "");
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
            let all_eight = doc
                .lines()
                .filter(|l| l.starts_with(&format!("| {code} |")))
                .any(|l| row.iter().all(|text| l.contains(text)));
            assert!(all_eight, "{code} is not documented in the eight languages");
        }
    }
}
