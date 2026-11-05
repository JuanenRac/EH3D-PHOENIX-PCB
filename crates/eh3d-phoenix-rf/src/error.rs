// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed errors of the RF crate, with stable codes `PHOENIX-RF-<NNN>`.
//!
//! The English text is the message of the error; [`title`] gives the title of a code in the eight
//! languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and
//! European Portuguese). The errors that come from a file say the line.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
pub use eh3d_phoenix_stackup::Language;

/// Every code of this crate with its English title, in order. `docs/ERROR_CODES.md` repeats it.
pub const REGISTRY: &[(&str, &str)] = &[
    ("PHOENIX-RF-001", "A number is not valid"),
    ("PHOENIX-RF-002", "The Touchstone file has a syntax error"),
    ("PHOENIX-RF-003", "The Touchstone data is not valid"),
    ("PHOENIX-RF-004", "The calculation is singular"),
    ("PHOENIX-RF-005", "The network is not valid"),
    ("PHOENIX-RF-006", "No network gives the match"),
    ("PHOENIX-RF-007", "The filter is not valid"),
    ("PHOENIX-RF-008", "The settings are not valid"),
    ("PHOENIX-RF-009", "A value is outside the configured limit"),
    ("PHOENIX-RF-010", "Frequency outside the data of the file"),
];

/// The titles of the codes in the eight languages: English, Spanish, German, French, Italian,
/// Japanese, Chinese and European Portuguese.
const TITLES: [(&str, [&str; 8]); 10] = [
    (
        "PHOENIX-RF-001",
        [
            "A number is not valid",
            "Un número no es válido",
            "Eine Zahl ist ungültig",
            "Un nombre n'est pas valide",
            "Un numero non è valido",
            "数値が正しくありません",
            "数值无效",
            "Um número não é válido",
        ],
    ),
    (
        "PHOENIX-RF-002",
        [
            "The Touchstone file has a syntax error",
            "El archivo Touchstone tiene un error de sintaxis",
            "Die Touchstone-Datei hat einen Syntaxfehler",
            "Le fichier Touchstone contient une erreur de syntaxe",
            "Il file Touchstone contiene un errore di sintassi",
            "Touchstone ファイルに構文エラーがあります",
            "Touchstone 文件存在语法错误",
            "O ficheiro Touchstone tem um erro de sintaxe",
        ],
    ),
    (
        "PHOENIX-RF-003",
        [
            "The Touchstone data is not valid",
            "Los datos Touchstone no son válidos",
            "Die Touchstone-Daten sind ungültig",
            "Les données Touchstone ne sont pas valides",
            "I dati Touchstone non sono validi",
            "Touchstone のデータが正しくありません",
            "Touchstone 数据无效",
            "Os dados Touchstone não são válidos",
        ],
    ),
    (
        "PHOENIX-RF-004",
        [
            "The calculation is singular",
            "El cálculo es singular",
            "Die Berechnung ist singulär",
            "Le calcul est singulier",
            "Il calcolo è singolare",
            "計算が特異です",
            "计算出现奇异",
            "O cálculo é singular",
        ],
    ),
    (
        "PHOENIX-RF-005",
        [
            "The network is not valid",
            "La red no es válida",
            "Das Netzwerk ist ungültig",
            "Le réseau n'est pas valide",
            "La rete non è valida",
            "ネットワークが正しくありません",
            "网络无效",
            "A rede não é válida",
        ],
    ),
    (
        "PHOENIX-RF-006",
        [
            "No network gives the match",
            "Ninguna red consigue la adaptación",
            "Kein Netzwerk erreicht die Anpassung",
            "Aucun réseau n'obtient l'adaptation",
            "Nessuna rete ottiene l'adattamento",
            "整合できるネットワークがありません",
            "没有网络能实现匹配",
            "Nenhuma rede consegue a adaptação",
        ],
    ),
    (
        "PHOENIX-RF-007",
        [
            "The filter is not valid",
            "El filtro no es válido",
            "Das Filter ist ungültig",
            "Le filtre n'est pas valide",
            "Il filtro non è valido",
            "フィルタが正しくありません",
            "滤波器无效",
            "O filtro não é válido",
        ],
    ),
    (
        "PHOENIX-RF-008",
        [
            "The settings are not valid",
            "La configuración no es válida",
            "Die Einstellungen sind ungültig",
            "Les réglages ne sont pas valides",
            "Le impostazioni non sono valide",
            "設定が正しくありません",
            "设置无效",
            "As definições não são válidas",
        ],
    ),
    (
        "PHOENIX-RF-009",
        [
            "A value is outside the configured limit",
            "Un valor supera el límite configurado",
            "Ein Wert überschreitet die eingestellte Grenze",
            "Une valeur dépasse la limite configurée",
            "Un valore supera il limite configurato",
            "値が設定された上限を超えています",
            "数值超出配置的限制",
            "Um valor excede o limite configurado",
        ],
    ),
    (
        "PHOENIX-RF-010",
        [
            "Frequency outside the data of the file",
            "Frecuencia fuera de los datos del archivo",
            "Frequenz außerhalb der Daten der Datei",
            "Fréquence hors des données du fichier",
            "Frequenza fuori dai dati del file",
            "ファイルのデータ範囲外の周波数",
            "频率超出文件数据范围",
            "Frequência fora dos dados do ficheiro",
        ],
    ),
];

fn column(language: Language) -> usize {
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
        .and_then(|(_, texts)| texts.get(column(language)).copied())
}

/// Why a calculation or a file was refused.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RfError {
    /// A number is not finite or is outside what the calculation accepts.
    #[error("{field} is not valid: {reason}")]
    InvalidNumber {
        /// The name of the value.
        field: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A line of a Touchstone file cannot be read.
    #[error("line {line} of the Touchstone file: {reason}")]
    TouchstoneSyntax {
        /// The line number, counted from 1 (0 when no line applies).
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// The values of a Touchstone file are read but cannot be used.
    #[error("line {line} of the Touchstone data: {reason}")]
    TouchstoneData {
        /// The line number, counted from 1 (0 when no line applies).
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// A division by zero, a matrix with no inverse or a result that is not a number.
    #[error("singular calculation: {what}")]
    Singular {
        /// What could not be computed.
        what: String,
    },
    /// The parameters of a network do not make a network.
    #[error("the network is not valid: {reason}")]
    InvalidNetwork {
        /// What is wrong with it.
        reason: String,
    },
    /// The impedances given have no matching network of the kind asked for.
    #[error("no network gives the match: {reason}")]
    NoMatch {
        /// Why there is none.
        reason: String,
    },
    /// The description of a filter cannot be used.
    #[error("the filter is not valid: {reason}")]
    InvalidFilter {
        /// What is wrong with it.
        reason: String,
    },
    /// A line of the settings text cannot be used.
    #[error("line {line} of the settings: {reason}")]
    InvalidSettings {
        /// The line number, counted from 1 (0 when no line applies).
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// A value is beyond a limit of the settings.
    #[error("{field} is {value}, beyond the limit {limit}")]
    OutsideLimit {
        /// The name of the value.
        field: String,
        /// The value.
        value: f64,
        /// The limit it passes.
        limit: f64,
    },
    /// A frequency lies outside the points of a file (values are never extrapolated).
    #[error("{freq_hz} Hz is outside the data of the file, {low_hz} Hz to {high_hz} Hz")]
    OutsideRange {
        /// The frequency asked for, in Hz.
        freq_hz: f64,
        /// The lowest frequency of the file, in Hz.
        low_hz: f64,
        /// The highest frequency of the file, in Hz.
        high_hz: f64,
    },
}

impl RfError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            RfError::InvalidNumber { .. } => "PHOENIX-RF-001",
            RfError::TouchstoneSyntax { .. } => "PHOENIX-RF-002",
            RfError::TouchstoneData { .. } => "PHOENIX-RF-003",
            RfError::Singular { .. } => "PHOENIX-RF-004",
            RfError::InvalidNetwork { .. } => "PHOENIX-RF-005",
            RfError::NoMatch { .. } => "PHOENIX-RF-006",
            RfError::InvalidFilter { .. } => "PHOENIX-RF-007",
            RfError::InvalidSettings { .. } => "PHOENIX-RF-008",
            RfError::OutsideLimit { .. } => "PHOENIX-RF-009",
            RfError::OutsideRange { .. } => "PHOENIX-RF-010",
        }
    }

    /// The message in a language: the title of the code and then the detail in English.
    pub fn message(&self, language: Language) -> String {
        let head = title(self.code(), language).unwrap_or("");
        format!("{head}: {self}")
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(0, Severity::Error, self.to_string())
            .with_code(self.code())
            .with_category(Category::Electrical)
            .with_recoverability(Recoverability::Unrecoverable)
    }
}

/// A number that must be finite and above zero.
pub(crate) fn positive(field: &str, value: f64) -> Result<f64, RfError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(RfError::InvalidNumber {
            field: field.to_string(),
            reason: "must be a finite number above zero".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_error() -> Vec<RfError> {
        vec![
            RfError::InvalidNumber {
                field: "x".into(),
                reason: "y".into(),
            },
            RfError::TouchstoneSyntax {
                line: 3,
                reason: "y".into(),
            },
            RfError::TouchstoneData {
                line: 4,
                reason: "y".into(),
            },
            RfError::Singular { what: "w".into() },
            RfError::InvalidNetwork { reason: "y".into() },
            RfError::NoMatch { reason: "y".into() },
            RfError::InvalidFilter { reason: "y".into() },
            RfError::InvalidSettings {
                line: 1,
                reason: "y".into(),
            },
            RfError::OutsideLimit {
                field: "f".into(),
                value: 2.0,
                limit: 1.0,
            },
            RfError::OutsideRange {
                freq_hz: 1.0,
                low_hz: 2.0,
                high_hz: 3.0,
            },
        ]
    }

    #[test]
    fn every_error_has_a_registered_code_and_a_diagnostic() {
        for error in every_error() {
            let code = error.code();
            assert!(REGISTRY.iter().any(|(c, _)| *c == code), "{code}");
            let d = error.diagnostic();
            assert_eq!(d.code.as_deref(), Some(code));
            assert_eq!(d.severity, Severity::Error);
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
            assert_eq!((parts[0], parts[1]), ("PHOENIX", "RF"));
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
        assert_eq!(title("PHOENIX-RF-001", Language::En), Some(REGISTRY[0].1));
        assert_eq!(
            title("PHOENIX-RF-004", Language::Pt),
            Some("O cálculo é singular")
        );
        assert_eq!(title("PHOENIX-RF-999", Language::En), None);
    }

    #[test]
    fn the_message_starts_with_the_title_of_the_language() {
        let error = RfError::Singular { what: "w".into() };
        assert!(error.message(Language::De).starts_with("Die Berechnung"));
        assert!(error
            .message(Language::En)
            .contains("singular calculation: w"));
        let line = RfError::TouchstoneSyntax {
            line: 7,
            reason: "r".into(),
        };
        assert!(line.message(Language::Fr).contains("line 7"));
    }

    #[test]
    fn positive_numbers_are_checked() {
        assert_eq!(positive("x", 2.0).unwrap(), 2.0);
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                positive("x", bad),
                Err(RfError::InvalidNumber { .. })
            ));
        }
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
