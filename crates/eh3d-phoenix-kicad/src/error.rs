// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed errors and notes of the KiCad readers, with stable codes `PHOENIX-KICAD-<NNN>`.
//!
//! The English text is the message; [`title`] gives the title of a code in the eight languages of
//! the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European
//! Portuguese). Everything that comes from a position in a file says the line and the column.

use eh3d_diagnostic::{Category, Diagnostic, Location, Recoverability, Severity};
pub use eh3d_phoenix_stackup::Language;

/// Every code of this crate with its English title, in order. `docs/ERROR_CODES.md` repeats it.
pub const REGISTRY: &[(&str, &str)] = &[
    ("PHOENIX-KICAD-001", "The KiCad file has a syntax error"),
    (
        "PHOENIX-KICAD-002",
        "The KiCad file is larger or deeper than the configured limit",
    ),
    (
        "PHOENIX-KICAD-003",
        "The file is not a KiCad file of the expected kind",
    ),
    (
        "PHOENIX-KICAD-004",
        "The KiCad file ends before it is complete",
    ),
    (
        "PHOENIX-KICAD-005",
        "A value in the KiCad file is not valid",
    ),
    (
        "PHOENIX-KICAD-006",
        "Part of the KiCad file not read into the model",
    ),
    ("PHOENIX-KICAD-007", "Item read in a simpler form"),
    (
        "PHOENIX-KICAD-008",
        "Reference to something the file does not have",
    ),
    ("PHOENIX-KICAD-009", "File version outside the tested range"),
    (
        "PHOENIX-KICAD-010",
        "The KiCad reader settings are not valid",
    ),
    ("PHOENIX-KICAD-011", "The KiCad project file is not valid"),
    ("PHOENIX-KICAD-012", "The file is not valid UTF-8 text"),
    ("PHOENIX-KICAD-013", "Nets derived from the drawing"),
    ("PHOENIX-KICAD-014", "Fidelity level of the import"),
    (
        "PHOENIX-KICAD-015",
        "A file of the KiCad project is missing",
    ),
    (
        "PHOENIX-KICAD-016",
        "The output of the import is not allowed",
    ),
    (
        "PHOENIX-KICAD-017",
        "The schematic and the board list different parts",
    ),
];

/// The titles of the codes in the eight languages: English, Spanish, German, French, Italian,
/// Japanese, Chinese and European Portuguese.
const TITLES: [(&str, [&str; 8]); 17] = [
    (
        "PHOENIX-KICAD-001",
        [
            "The KiCad file has a syntax error",
            "El archivo de KiCad tiene un error de sintaxis",
            "Die KiCad-Datei hat einen Syntaxfehler",
            "Le fichier KiCad contient une erreur de syntaxe",
            "Il file KiCad contiene un errore di sintassi",
            "KiCad ファイルに構文エラーがあります",
            "KiCad 文件存在语法错误",
            "O ficheiro KiCad tem um erro de sintaxe",
        ],
    ),
    (
        "PHOENIX-KICAD-002",
        [
            "The KiCad file is larger or deeper than the configured limit",
            "El archivo de KiCad supera el límite configurado de tamaño o profundidad",
            "Die KiCad-Datei überschreitet die eingestellte Grenze für Größe oder Tiefe",
            "Le fichier KiCad dépasse la limite configurée de taille ou de profondeur",
            "Il file KiCad supera il limite configurato di dimensione o profondità",
            "KiCad ファイルが設定された大きさまたは深さの上限を超えています",
            "KiCad 文件超出配置的大小或深度限制",
            "O ficheiro KiCad excede o limite configurado de tamanho ou profundidade",
        ],
    ),
    (
        "PHOENIX-KICAD-003",
        [
            "The file is not a KiCad file of the expected kind",
            "El archivo no es un archivo de KiCad del tipo esperado",
            "Die Datei ist keine KiCad-Datei der erwarteten Art",
            "Le fichier n'est pas un fichier KiCad du type attendu",
            "Il file non è un file KiCad del tipo atteso",
            "ファイルは期待される種類の KiCad ファイルではありません",
            "该文件不是预期类型的 KiCad 文件",
            "O ficheiro não é um ficheiro KiCad do tipo esperado",
        ],
    ),
    (
        "PHOENIX-KICAD-004",
        [
            "The KiCad file ends before it is complete",
            "El archivo de KiCad termina antes de estar completo",
            "Die KiCad-Datei endet, bevor sie vollständig ist",
            "Le fichier KiCad se termine avant d'être complet",
            "Il file KiCad termina prima di essere completo",
            "KiCad ファイルが完結する前に終わっています",
            "KiCad 文件在完整之前就结束了",
            "O ficheiro KiCad termina antes de estar completo",
        ],
    ),
    (
        "PHOENIX-KICAD-005",
        [
            "A value in the KiCad file is not valid",
            "Un valor del archivo de KiCad no es válido",
            "Ein Wert in der KiCad-Datei ist ungültig",
            "Une valeur du fichier KiCad n'est pas valide",
            "Un valore del file KiCad non è valido",
            "KiCad ファイルの値が正しくありません",
            "KiCad 文件中的数值无效",
            "Um valor do ficheiro KiCad não é válido",
        ],
    ),
    (
        "PHOENIX-KICAD-006",
        [
            "Part of the KiCad file not read into the model",
            "Parte del archivo de KiCad no leída en el modelo",
            "Teil der KiCad-Datei nicht in das Modell gelesen",
            "Partie du fichier KiCad non lue dans le modèle",
            "Parte del file KiCad non letta nel modello",
            "KiCad ファイルの一部はモデルに読み込まれませんでした",
            "KiCad 文件的一部分未读入模型",
            "Parte do ficheiro KiCad não lida para o modelo",
        ],
    ),
    (
        "PHOENIX-KICAD-007",
        [
            "Item read in a simpler form",
            "Elemento leído en una forma más simple",
            "Element in einfacherer Form gelesen",
            "Élément lu sous une forme plus simple",
            "Elemento letto in una forma più semplice",
            "要素はより単純な形で読み込まれました",
            "项目以更简单的形式读入",
            "Elemento lido numa forma mais simples",
        ],
    ),
    (
        "PHOENIX-KICAD-008",
        [
            "Reference to something the file does not have",
            "Referencia a algo que el archivo no tiene",
            "Verweis auf etwas, das die Datei nicht enthält",
            "Référence à un élément absent du fichier",
            "Riferimento a qualcosa che il file non contiene",
            "ファイルにないものへの参照",
            "引用了文件中不存在的内容",
            "Referência a algo que o ficheiro não tem",
        ],
    ),
    (
        "PHOENIX-KICAD-009",
        [
            "File version outside the tested range",
            "Versión del archivo fuera del rango probado",
            "Dateiversion außerhalb des getesteten Bereichs",
            "Version du fichier hors de la plage testée",
            "Versione del file fuori dall'intervallo provato",
            "ファイルのバージョンが検証済みの範囲外です",
            "文件版本超出已测试范围",
            "Versão do ficheiro fora do intervalo testado",
        ],
    ),
    (
        "PHOENIX-KICAD-010",
        [
            "The KiCad reader settings are not valid",
            "La configuración del lector de KiCad no es válida",
            "Die Einstellungen des KiCad-Lesers sind ungültig",
            "Les réglages du lecteur KiCad ne sont pas valides",
            "Le impostazioni del lettore KiCad non sono valide",
            "KiCad リーダーの設定が正しくありません",
            "KiCad 读取器的设置无效",
            "As definições do leitor KiCad não são válidas",
        ],
    ),
    (
        "PHOENIX-KICAD-011",
        [
            "The KiCad project file is not valid",
            "El archivo de proyecto de KiCad no es válido",
            "Die KiCad-Projektdatei ist ungültig",
            "Le fichier de projet KiCad n'est pas valide",
            "Il file di progetto KiCad non è valido",
            "KiCad プロジェクトファイルが正しくありません",
            "KiCad 项目文件无效",
            "O ficheiro de projeto KiCad não é válido",
        ],
    ),
    (
        "PHOENIX-KICAD-012",
        [
            "The file is not valid UTF-8 text",
            "El archivo no es texto UTF-8 válido",
            "Die Datei ist kein gültiger UTF-8-Text",
            "Le fichier n'est pas un texte UTF-8 valide",
            "Il file non è testo UTF-8 valido",
            "ファイルは有効な UTF-8 テキストではありません",
            "文件不是有效的 UTF-8 文本",
            "O ficheiro não é texto UTF-8 válido",
        ],
    ),
    (
        "PHOENIX-KICAD-013",
        [
            "Nets derived from the drawing",
            "Redes deducidas del dibujo",
            "Aus der Zeichnung abgeleitete Netze",
            "Réseaux déduits du dessin",
            "Reti ricavate dal disegno",
            "図面から導出されたネット",
            "从图纸推导出的网络",
            "Redes derivadas do desenho",
        ],
    ),
    (
        "PHOENIX-KICAD-014",
        [
            "Fidelity level of the import",
            "Nivel de fidelidad de la importación",
            "Genauigkeitsstufe des Imports",
            "Niveau de fidélité de l'import",
            "Livello di fedeltà dell'importazione",
            "インポートの忠実度レベル",
            "导入的保真度级别",
            "Nível de fidelidade da importação",
        ],
    ),
    (
        "PHOENIX-KICAD-015",
        [
            "A file of the KiCad project is missing",
            "Falta un archivo del proyecto de KiCad",
            "Eine Datei des KiCad-Projekts fehlt",
            "Un fichier du projet KiCad est absent",
            "Manca un file del progetto KiCad",
            "KiCad プロジェクトのファイルが見つかりません",
            "缺少 KiCad 项目的文件",
            "Falta um ficheiro do projeto KiCad",
        ],
    ),
    (
        "PHOENIX-KICAD-016",
        [
            "The output of the import is not allowed",
            "No se permite el archivo de salida de la importación",
            "Die Ausgabedatei des Imports ist nicht erlaubt",
            "Le fichier de sortie de l'import n'est pas autorisé",
            "Il file di uscita dell'importazione non è consentito",
            "インポートの出力ファイルは許可されていません",
            "不允许使用该导入输出文件",
            "O ficheiro de saída da importação não é permitido",
        ],
    ),
    (
        "PHOENIX-KICAD-017",
        [
            "The schematic and the board list different parts",
            "El esquema y la placa listan piezas distintas",
            "Schaltplan und Platine führen unterschiedliche Bauteile auf",
            "Le schéma et la carte listent des composants différents",
            "Lo schema e la scheda elencano componenti diversi",
            "回路図と基板で部品の一覧が異なります",
            "原理图与电路板列出的元件不同",
            "O esquema e a placa listam peças diferentes",
        ],
    ),
];

fn column_of(language: Language) -> usize {
    match language {
        Language::En => 0,
        Language::Es => 1,
        Language::De => 2,
        Language::Fr => 3,
        Language::It => 4,
        Language::Ja => 5,
        Language::Zh => 6,
        Language::Pt => 7,
    }
}

/// The title of a code in a language, or `None` for a code this crate does not have.
pub fn title(code: &str, language: Language) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, texts)| texts.get(column_of(language)).copied())
}

/// Why a file was refused, or what was found while reading it.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum KicadError {
    /// The text is not a well formed s-expression.
    #[error("line {line}, column {column}: {reason}")]
    Syntax {
        /// The line, counted from 1.
        line: usize,
        /// The column, counted from 1 in characters.
        column: usize,
        /// What is wrong.
        reason: String,
    },
    /// A size, a depth or a count is beyond a limit of the settings.
    #[error("{what} is {value}, beyond the limit {limit} (line {line}, column {column})")]
    Limit {
        /// What was counted.
        what: String,
        /// The value found (or the first value over the limit).
        value: u64,
        /// The limit.
        limit: u64,
        /// The line, 0 when no position applies.
        line: usize,
        /// The column, 0 when no position applies.
        column: usize,
    },
    /// The file is well formed but is not the kind of KiCad file that was asked for.
    #[error("expected {expected}, found {found}")]
    NotKicad {
        /// The kind expected.
        expected: String,
        /// What the file starts with.
        found: String,
    },
    /// The text ends while a list or a string is still open.
    #[error("the file ends at line {line}, column {column} with {open} list(s) still open")]
    Truncated {
        /// The line where the text ends.
        line: usize,
        /// The column where the text ends.
        column: usize,
        /// How many lists were still open.
        open: usize,
    },
    /// A value cannot be used.
    #[error("line {line}, column {column}: {what}: {reason}")]
    InvalidValue {
        /// The line, 0 when no position applies.
        line: usize,
        /// The column, 0 when no position applies.
        column: usize,
        /// What the value is.
        what: String,
        /// What is wrong with it.
        reason: String,
    },
    /// Elements that the readers do not interpret, counted by name.
    #[error("{count} `{name}` element(s) not read")]
    NotRead {
        /// The name of the element.
        name: String,
        /// How many there were.
        count: usize,
    },
    /// Something was read in a simpler form than the file holds it.
    #[error("{what}: {detail}")]
    Simplified {
        /// What was simplified.
        what: String,
        /// How it was read.
        detail: String,
    },
    /// A reference to a net, a layer, a symbol or a sheet that the file does not have.
    #[error("line {line}: {what}")]
    MissingReference {
        /// What is missing.
        what: String,
        /// The line, 0 when no position applies.
        line: usize,
    },
    /// The version of the file is outside the range the tests cover.
    #[error("version {found}: {detail}")]
    Version {
        /// The version found, as written in the file.
        found: String,
        /// Why it matters.
        detail: String,
    },
    /// A line of the settings text cannot be used.
    #[error("line {line} of the settings: {reason}")]
    InvalidSettings {
        /// The line, counted from 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// The project file (JSON) cannot be used.
    #[error("{reason}")]
    Project {
        /// What is wrong with it.
        reason: String,
    },
    /// The bytes are not UTF-8; the invalid sequences were replaced.
    #[error("{reason}")]
    Encoding {
        /// What was found.
        reason: String,
    },
    /// The nets were computed from the drawing and are not stored in the file.
    #[error("{detail}")]
    Derived {
        /// How they were computed.
        detail: String,
    },
    /// The fidelity level of an import.
    #[error("{level}: {detail}")]
    Fidelity {
        /// The level, `complete`, `partial` or `unusable`.
        level: String,
        /// What it means for this file.
        detail: String,
    },
    /// A file the project names (its schematic or its board) is not there or was not read.
    #[error("{what}: {detail}")]
    ProjectFile {
        /// The file.
        what: String,
        /// What happened to it.
        detail: String,
    },
    /// The file the import was asked to write cannot be written.
    #[error("{reason}")]
    OutputRefused {
        /// Why.
        reason: String,
    },
    /// The schematic and the board of one project do not hold the same parts.
    #[error("{detail}")]
    PartsDiffer {
        /// Which parts are on one side only.
        detail: String,
    },
}

impl KicadError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            KicadError::Syntax { .. } => "PHOENIX-KICAD-001",
            KicadError::Limit { .. } => "PHOENIX-KICAD-002",
            KicadError::NotKicad { .. } => "PHOENIX-KICAD-003",
            KicadError::Truncated { .. } => "PHOENIX-KICAD-004",
            KicadError::InvalidValue { .. } => "PHOENIX-KICAD-005",
            KicadError::NotRead { .. } => "PHOENIX-KICAD-006",
            KicadError::Simplified { .. } => "PHOENIX-KICAD-007",
            KicadError::MissingReference { .. } => "PHOENIX-KICAD-008",
            KicadError::Version { .. } => "PHOENIX-KICAD-009",
            KicadError::InvalidSettings { .. } => "PHOENIX-KICAD-010",
            KicadError::Project { .. } => "PHOENIX-KICAD-011",
            KicadError::Encoding { .. } => "PHOENIX-KICAD-012",
            KicadError::Derived { .. } => "PHOENIX-KICAD-013",
            KicadError::Fidelity { .. } => "PHOENIX-KICAD-014",
            KicadError::ProjectFile { .. } => "PHOENIX-KICAD-015",
            KicadError::OutputRefused { .. } => "PHOENIX-KICAD-016",
            KicadError::PartsDiffer { .. } => "PHOENIX-KICAD-017",
        }
    }

    /// How serious it is.
    pub fn severity(&self) -> Severity {
        match self {
            KicadError::Syntax { .. }
            | KicadError::Limit { .. }
            | KicadError::NotKicad { .. }
            | KicadError::Truncated { .. }
            | KicadError::InvalidSettings { .. }
            | KicadError::OutputRefused { .. }
            | KicadError::Project { .. } => Severity::Error,
            KicadError::InvalidValue { .. }
            | KicadError::MissingReference { .. }
            | KicadError::Version { .. }
            | KicadError::ProjectFile { .. }
            | KicadError::PartsDiffer { .. }
            | KicadError::Encoding { .. } => Severity::Warning,
            KicadError::NotRead { .. }
            | KicadError::Simplified { .. }
            | KicadError::Derived { .. }
            | KicadError::Fidelity { .. } => Severity::Info,
        }
    }

    /// The line and the column the error points at, `(0, 0)` when it points at none.
    pub fn position(&self) -> (usize, usize) {
        match self {
            KicadError::Syntax { line, column, .. }
            | KicadError::Limit { line, column, .. }
            | KicadError::Truncated { line, column, .. }
            | KicadError::InvalidValue { line, column, .. } => (*line, *column),
            KicadError::MissingReference { line, .. } => (*line, 0),
            _ => (0, 0),
        }
    }

    /// The message in a language: the title of the code and then the detail in English.
    pub fn message(&self, language: Language) -> String {
        let head = title(self.code(), language).unwrap_or("");
        format!("{head}: {self}")
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        let (line, column) = self.position();
        let line = u32::try_from(line).unwrap_or(u32::MAX);
        let mut d = Diagnostic::new(line, self.severity(), self.to_string())
            .with_code(self.code())
            .with_category(Category::Format)
            .with_recoverability(if self.severity() == Severity::Error {
                Recoverability::Unrecoverable
            } else {
                Recoverability::Recoverable
            });
        if column > 0 {
            d = d.with_location(Location::Column(u32::try_from(column).unwrap_or(u32::MAX)));
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_error() -> Vec<KicadError> {
        vec![
            KicadError::Syntax {
                line: 3,
                column: 4,
                reason: "r".into(),
            },
            KicadError::Limit {
                what: "w".into(),
                value: 2,
                limit: 1,
                line: 1,
                column: 1,
            },
            KicadError::NotKicad {
                expected: "e".into(),
                found: "f".into(),
            },
            KicadError::Truncated {
                line: 9,
                column: 1,
                open: 2,
            },
            KicadError::InvalidValue {
                line: 2,
                column: 5,
                what: "w".into(),
                reason: "r".into(),
            },
            KicadError::NotRead {
                name: "n".into(),
                count: 2,
            },
            KicadError::Simplified {
                what: "w".into(),
                detail: "d".into(),
            },
            KicadError::MissingReference {
                what: "w".into(),
                line: 7,
            },
            KicadError::Version {
                found: "1".into(),
                detail: "d".into(),
            },
            KicadError::InvalidSettings {
                line: 1,
                reason: "r".into(),
            },
            KicadError::Project { reason: "r".into() },
            KicadError::Encoding { reason: "r".into() },
            KicadError::Derived { detail: "d".into() },
            KicadError::Fidelity {
                level: "partial".into(),
                detail: "d".into(),
            },
            KicadError::ProjectFile {
                what: "w".into(),
                detail: "d".into(),
            },
            KicadError::OutputRefused { reason: "r".into() },
            KicadError::PartsDiffer { detail: "d".into() },
        ]
    }

    #[test]
    fn every_error_has_a_registered_code_and_a_diagnostic() {
        for error in every_error() {
            let code = error.code();
            assert!(REGISTRY.iter().any(|(c, _)| *c == code), "{code}");
            let d = error.diagnostic();
            assert_eq!(d.code.as_deref(), Some(code));
            assert_eq!(d.severity, error.severity());
            assert!(!d.message.is_empty());
        }
    }

    #[test]
    fn every_registered_code_is_produced_by_an_error() {
        let mut made: Vec<&str> = every_error().iter().map(|e| e.code()).collect();
        made.sort_unstable();
        let mut registered: Vec<&str> = REGISTRY.iter().map(|(c, _)| *c).collect();
        registered.sort_unstable();
        assert_eq!(made, registered);
    }

    #[test]
    fn codes_follow_the_pattern_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 3, "{code}");
            assert_eq!((parts[0], parts[1]), ("PHOENIX", "KICAD"));
            assert_eq!(parts[2].len(), 3);
            assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
            assert!(!title.is_empty());
            assert!(seen.insert(*code), "{code} repeated");
        }
    }

    #[test]
    fn the_title_table_agrees_with_the_registry_in_every_language() {
        assert_eq!(TITLES.len(), REGISTRY.len());
        for ((code, english), (table_code, texts)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, table_code);
            assert_eq!(*english, texts[0]);
            for text in texts {
                assert!(!text.is_empty(), "{code}");
            }
            assert_ne!(texts[5], texts[0], "{code}: Japanese");
            assert_ne!(texts[6], texts[0], "{code}: Chinese");
        }
        assert_eq!(
            title("PHOENIX-KICAD-004", Language::Pt),
            Some("O ficheiro KiCad termina antes de estar completo")
        );
        assert_eq!(title("PHOENIX-KICAD-999", Language::En), None);
    }

    #[test]
    fn a_position_travels_into_the_diagnostic() {
        let d = KicadError::Syntax {
            line: 12,
            column: 7,
            reason: "x".into(),
        }
        .diagnostic();
        assert_eq!(d.line, 12);
        assert_eq!(d.location, Some(Location::Column(7)));
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.recoverability, Some(Recoverability::Unrecoverable));
    }

    #[test]
    fn the_message_starts_with_the_title_of_the_language() {
        let error = KicadError::Truncated {
            line: 5,
            column: 2,
            open: 3,
        };
        assert!(error
            .message(Language::De)
            .starts_with("Die KiCad-Datei endet"));
        assert!(error.message(Language::En).contains("3 list(s) still open"));
    }

    #[test]
    fn the_documented_table_has_every_code_of_the_registry() {
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
