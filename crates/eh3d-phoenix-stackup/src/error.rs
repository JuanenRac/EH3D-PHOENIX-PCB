// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed errors and warnings of the stack-up crate, with stable codes `PHOENIX-STACK-<NNN>`.
//!
//! An error stops the calculation; a [`Warning`] goes with a result that was computed outside the
//! published range of a formula, or with a model that is only an approximation. The English text
//! is the message of the error; [`title`] gives the title of a code in the eight languages of the
//! suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese).

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use serde::Serialize;

/// Every code of this crate with its English title, in order. `docs/ERROR_CODES.md` repeats it.
pub const REGISTRY: &[(&str, &str)] = &[
    ("PHOENIX-STACK-001", "A number is not valid"),
    ("PHOENIX-STACK-002", "The stack-up is not valid"),
    ("PHOENIX-STACK-003", "Unknown material"),
    (
        "PHOENIX-STACK-004",
        "Frequency outside the published points",
    ),
    ("PHOENIX-STACK-005", "Material data is not valid"),
    ("PHOENIX-STACK-006", "The line is not valid"),
    ("PHOENIX-STACK-007", "No width reaches the target impedance"),
    ("PHOENIX-STACK-008", "Synthesis did not converge"),
    ("PHOENIX-STACK-009", "The settings are not valid"),
    ("PHOENIX-STACK-010", "The job file is not valid"),
    (
        "PHOENIX-STACK-011",
        "Geometry outside the domain of the formula",
    ),
    (
        "PHOENIX-STACK-012",
        "Outside the validity range of the formula",
    ),
    (
        "PHOENIX-STACK-013",
        "Conductor thin compared with the skin depth",
    ),
    ("PHOENIX-STACK-014", "Quasi-static limit exceeded"),
    (
        "PHOENIX-STACK-015",
        "Solder mask over the line is not modelled",
    ),
    ("PHOENIX-STACK-016", "Approximate model"),
];

/// The languages of the suite, in the order of the columns of the title table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// English.
    En,
    /// Spanish.
    Es,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese.
    Zh,
    /// European Portuguese.
    Pt,
}

impl Language {
    /// Every language, in the order of the columns of the title table.
    pub const ALL: [Language; 8] = [
        Language::En,
        Language::Es,
        Language::De,
        Language::Fr,
        Language::It,
        Language::Ja,
        Language::Zh,
        Language::Pt,
    ];

    /// The language of a two-letter tag (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn from_tag(tag: &str) -> Option<Language> {
        match tag {
            "en" => Some(Language::En),
            "es" => Some(Language::Es),
            "de" => Some(Language::De),
            "fr" => Some(Language::Fr),
            "it" => Some(Language::It),
            "ja" => Some(Language::Ja),
            "zh" => Some(Language::Zh),
            "pt" => Some(Language::Pt),
            _ => None,
        }
    }

    fn column(self) -> usize {
        match self {
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
}

/// The titles of the codes in the eight languages: English, Spanish, German, French, Italian,
/// Japanese, Chinese and European Portuguese.
const TITLES: [(&str, [&str; 8]); 16] = [
    (
        "PHOENIX-STACK-001",
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
        "PHOENIX-STACK-002",
        [
            "The stack-up is not valid",
            "El apilado no es válido",
            "Der Lagenaufbau ist ungültig",
            "L'empilage n'est pas valide",
            "La stratificazione non è valida",
            "スタックアップが正しくありません",
            "叠层无效",
            "O empilhamento não é válido",
        ],
    ),
    (
        "PHOENIX-STACK-003",
        [
            "Unknown material",
            "Material desconocido",
            "Unbekanntes Material",
            "Matériau inconnu",
            "Materiale sconosciuto",
            "不明な材料",
            "未知材料",
            "Material desconhecido",
        ],
    ),
    (
        "PHOENIX-STACK-004",
        [
            "Frequency outside the published points",
            "Frecuencia fuera de los puntos publicados",
            "Frequenz außerhalb der veröffentlichten Punkte",
            "Fréquence hors des points publiés",
            "Frequenza fuori dai punti pubblicati",
            "公表された点の範囲外の周波数",
            "频率超出已发布的数据点范围",
            "Frequência fora dos pontos publicados",
        ],
    ),
    (
        "PHOENIX-STACK-005",
        [
            "Material data is not valid",
            "Los datos del material no son válidos",
            "Die Materialdaten sind ungültig",
            "Les données du matériau ne sont pas valides",
            "I dati del materiale non sono validi",
            "材料データが正しくありません",
            "材料数据无效",
            "Os dados do material não são válidos",
        ],
    ),
    (
        "PHOENIX-STACK-006",
        [
            "The line is not valid",
            "La línea no es válida",
            "Die Leitung ist ungültig",
            "La ligne n'est pas valide",
            "La linea non è valida",
            "線路が正しくありません",
            "传输线无效",
            "A linha não é válida",
        ],
    ),
    (
        "PHOENIX-STACK-007",
        [
            "No width reaches the target impedance",
            "Ningún ancho alcanza la impedancia objetivo",
            "Keine Breite erreicht die Zielimpedanz",
            "Aucune largeur n'atteint l'impédance visée",
            "Nessuna larghezza raggiunge l'impedenza richiesta",
            "目標インピーダンスに到達する幅がありません",
            "没有任何线宽能达到目标阻抗",
            "Nenhuma largura atinge a impedância pretendida",
        ],
    ),
    (
        "PHOENIX-STACK-008",
        [
            "Synthesis did not converge",
            "La síntesis no convergió",
            "Die Synthese ist nicht konvergiert",
            "La synthèse n'a pas convergé",
            "La sintesi non è convergita",
            "合成が収束しませんでした",
            "综合未收敛",
            "A síntese não convergiu",
        ],
    ),
    (
        "PHOENIX-STACK-009",
        [
            "The settings are not valid",
            "Los ajustes no son válidos",
            "Die Einstellungen sind ungültig",
            "Les réglages ne sont pas valides",
            "Le impostazioni non sono valide",
            "設定が正しくありません",
            "设置无效",
            "As definições não são válidas",
        ],
    ),
    (
        "PHOENIX-STACK-010",
        [
            "The job file is not valid",
            "El fichero de trabajo no es válido",
            "Die Auftragsdatei ist ungültig",
            "Le fichier de travail n'est pas valide",
            "Il file di lavoro non è valido",
            "ジョブファイルが正しくありません",
            "作业文件无效",
            "O ficheiro de trabalho não é válido",
        ],
    ),
    (
        "PHOENIX-STACK-011",
        [
            "Geometry outside the domain of the formula",
            "Geometría fuera del dominio de la fórmula",
            "Geometrie außerhalb des Definitionsbereichs der Formel",
            "Géométrie hors du domaine de la formule",
            "Geometria fuori dal dominio della formula",
            "式の定義域外の形状",
            "几何尺寸超出公式的定义域",
            "Geometria fora do domínio da fórmula",
        ],
    ),
    (
        "PHOENIX-STACK-012",
        [
            "Outside the validity range of the formula",
            "Fuera del rango de validez de la fórmula",
            "Außerhalb des Gültigkeitsbereichs der Formel",
            "Hors de la plage de validité de la formule",
            "Fuori dall'intervallo di validità della formula",
            "式の有効範囲外",
            "超出公式的适用范围",
            "Fora do intervalo de validade da fórmula",
        ],
    ),
    (
        "PHOENIX-STACK-013",
        [
            "Conductor thin compared with the skin depth",
            "Conductor delgado frente a la profundidad pelicular",
            "Leiter dünn im Vergleich zur Skintiefe",
            "Conducteur mince par rapport à l'épaisseur de peau",
            "Conduttore sottile rispetto alla profondità di penetrazione",
            "表皮深さに対して薄い導体",
            "导体相对于趋肤深度过薄",
            "Condutor fino em comparação com a profundidade pelicular",
        ],
    ),
    (
        "PHOENIX-STACK-014",
        [
            "Quasi-static limit exceeded",
            "Límite cuasiestático superado",
            "Quasistatische Grenze überschritten",
            "Limite quasi statique dépassée",
            "Limite quasi statico superato",
            "準静的限界を超えています",
            "超出准静态极限",
            "Limite quase estático excedido",
        ],
    ),
    (
        "PHOENIX-STACK-015",
        [
            "Solder mask over the line is not modelled",
            "La máscara sobre la línea no se modela",
            "Die Lötstoppmaske über der Leitung wird nicht modelliert",
            "Le vernis épargne sur la ligne n'est pas modélisé",
            "La maschera di saldatura sulla linea non è modellata",
            "線路上のソルダーマスクはモデル化されていません",
            "未对线路上的阻焊层建模",
            "A máscara de solda sobre a linha não é modelada",
        ],
    ),
    (
        "PHOENIX-STACK-016",
        [
            "Approximate model",
            "Modelo aproximado",
            "Näherungsmodell",
            "Modèle approximatif",
            "Modello approssimato",
            "近似モデル",
            "近似模型",
            "Modelo aproximado",
        ],
    ),
];

/// The title of a code in a language, or `None` for a code this crate does not have.
pub fn title(code: &str, language: Language) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[language.column()])
}

/// Why a calculation or a file was refused.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StackupError {
    /// A number is not finite or is outside the range this crate accepts.
    #[error("{field} is not valid: {reason}")]
    InvalidNumber {
        /// The name of the value.
        field: String,
        /// What is wrong with it.
        reason: String,
    },
    /// The layers do not make a stack-up.
    #[error("the stack-up is not valid: {reason}")]
    InvalidStackup {
        /// What is wrong with it.
        reason: String,
    },
    /// A layer names a material that was not given.
    #[error("unknown material {name}")]
    UnknownMaterial {
        /// The name of the material.
        name: String,
    },
    /// The frequency is below the lowest or above the highest published point.
    #[error("{what} at {freq_ghz} GHz is outside the published {low_ghz} to {high_ghz} GHz")]
    OutsidePublished {
        /// What was asked for (the material and `Dk` or `Df`).
        what: String,
        /// The frequency asked for, in GHz.
        freq_ghz: f64,
        /// The lowest published frequency, in GHz.
        low_ghz: f64,
        /// The highest published frequency, in GHz.
        high_ghz: f64,
    },
    /// The points of a material are not usable.
    #[error("material {name} is not valid: {reason}")]
    InvalidMaterial {
        /// The name of the material.
        name: String,
        /// What is wrong with it.
        reason: String,
    },
    /// The description of a line does not fit the stack-up.
    #[error("the line is not valid: {reason}")]
    InvalidLine {
        /// What is wrong with it.
        reason: String,
    },
    /// The target impedance lies outside what the allowed widths give.
    #[error(
        "no width gives {target_ohm} ohm: the allowed widths give {low_ohm} to {high_ohm} ohm"
    )]
    TargetUnreachable {
        /// The impedance asked for, in ohm.
        target_ohm: f64,
        /// The lowest impedance the widest allowed track gives, in ohm.
        low_ohm: f64,
        /// The highest impedance the narrowest allowed track gives, in ohm.
        high_ohm: f64,
    },
    /// The search for a width ended without reaching the tolerance.
    #[error("synthesis did not converge in {iterations} steps (best error {error_ohm} ohm)")]
    NotConverged {
        /// The steps taken.
        iterations: u32,
        /// The remaining distance to the target, in ohm.
        error_ohm: f64,
    },
    /// A line of the settings text cannot be used.
    #[error("line {line} of the settings: {reason}")]
    InvalidSettings {
        /// The line number, counted from 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// The job file cannot be read.
    #[error("the job file is not valid: {reason}")]
    InvalidJob {
        /// What is wrong with it.
        reason: String,
    },
    /// The geometry is such that the formula has no meaning (a gap eaten by the copper
    /// thickness, a result that is not a number).
    #[error("the geometry is outside the domain of the formula: {reason}")]
    OutsideDomain {
        /// What is wrong with it.
        reason: String,
    },
}

impl StackupError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            StackupError::InvalidNumber { .. } => "PHOENIX-STACK-001",
            StackupError::InvalidStackup { .. } => "PHOENIX-STACK-002",
            StackupError::UnknownMaterial { .. } => "PHOENIX-STACK-003",
            StackupError::OutsidePublished { .. } => "PHOENIX-STACK-004",
            StackupError::InvalidMaterial { .. } => "PHOENIX-STACK-005",
            StackupError::InvalidLine { .. } => "PHOENIX-STACK-006",
            StackupError::TargetUnreachable { .. } => "PHOENIX-STACK-007",
            StackupError::NotConverged { .. } => "PHOENIX-STACK-008",
            StackupError::InvalidSettings { .. } => "PHOENIX-STACK-009",
            StackupError::InvalidJob { .. } => "PHOENIX-STACK-010",
            StackupError::OutsideDomain { .. } => "PHOENIX-STACK-011",
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

/// Something to know about a result that was computed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Warning {
    /// The stable code (`PHOENIX-STACK-012` to `PHOENIX-STACK-016`).
    pub code: &'static str,
    /// The detail, in English.
    pub detail: String,
}

impl Warning {
    /// A warning with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> Warning {
        Warning {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the title of the code and then the detail in English.
    pub fn message(&self, language: Language) -> String {
        let head = title(self.code, language).unwrap_or("");
        format!("{head}: {}", self.detail)
    }

    /// The common diagnostic of this warning.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(0, Severity::Warning, self.detail.clone())
            .with_code(self.code)
            .with_category(Category::Electrical)
            .with_recoverability(Recoverability::Recoverable)
    }
}

/// Warning code: a value outside the validity range of the formula.
pub const WARN_RANGE: &str = "PHOENIX-STACK-012";
/// Warning code: a conductor thin compared with the skin depth.
pub const WARN_THIN: &str = "PHOENIX-STACK-013";
/// Warning code: the quasi-static limit is exceeded.
pub const WARN_QUASI_STATIC: &str = "PHOENIX-STACK-014";
/// Warning code: a solder mask over the line is not modelled.
pub const WARN_MASK: &str = "PHOENIX-STACK-015";
/// Warning code: the model is an approximation.
pub const WARN_APPROXIMATE: &str = "PHOENIX-STACK-016";

#[cfg(test)]
mod tests {
    use super::*;

    fn every_error() -> Vec<StackupError> {
        vec![
            StackupError::InvalidNumber {
                field: "x".into(),
                reason: "y".into(),
            },
            StackupError::InvalidStackup { reason: "y".into() },
            StackupError::UnknownMaterial { name: "m".into() },
            StackupError::OutsidePublished {
                what: "m Dk".into(),
                freq_ghz: 1.0,
                low_ghz: 2.0,
                high_ghz: 3.0,
            },
            StackupError::InvalidMaterial {
                name: "m".into(),
                reason: "y".into(),
            },
            StackupError::InvalidLine { reason: "y".into() },
            StackupError::TargetUnreachable {
                target_ohm: 50.0,
                low_ohm: 60.0,
                high_ohm: 70.0,
            },
            StackupError::NotConverged {
                iterations: 3,
                error_ohm: 1.0,
            },
            StackupError::InvalidSettings {
                line: 1,
                reason: "y".into(),
            },
            StackupError::InvalidJob { reason: "y".into() },
            StackupError::OutsideDomain { reason: "y".into() },
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
    fn every_registered_code_is_produced_by_an_error_or_a_warning() {
        let mut made: Vec<&str> = every_error().iter().map(|e| e.code()).collect();
        made.extend([
            WARN_RANGE,
            WARN_THIN,
            WARN_QUASI_STATIC,
            WARN_MASK,
            WARN_APPROXIMATE,
        ]);
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
            assert_eq!((parts[0], parts[1]), ("PHOENIX", "STACK"));
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
            // no language repeats the English text except where the word is the same
            assert_ne!(texts[5], texts[0], "{code}: Japanese");
            assert_ne!(texts[6], texts[0], "{code}: Chinese");
        }
        assert_eq!(
            title("PHOENIX-STACK-001", Language::En),
            Some(REGISTRY[0].1)
        );
        assert_eq!(
            title("PHOENIX-STACK-002", Language::Pt),
            Some("O empilhamento não é válido")
        );
        assert_eq!(title("PHOENIX-STACK-999", Language::En), None);
    }

    #[test]
    fn language_tags_cover_the_eight_languages() {
        for (language, tag) in Language::ALL
            .iter()
            .zip(["en", "es", "de", "fr", "it", "ja", "zh", "pt"])
        {
            assert_eq!(Language::from_tag(tag), Some(*language));
        }
        assert_eq!(Language::from_tag("xx"), None);
    }

    #[test]
    fn the_message_starts_with_the_title_of_the_language() {
        let error = StackupError::UnknownMaterial { name: "m".into() };
        assert!(error
            .message(Language::De)
            .starts_with("Unbekanntes Material"));
        assert!(error.message(Language::En).contains("unknown material m"));
        let warning = Warning::new(WARN_MASK, "mask over L1");
        assert!(warning.message(Language::Pt).contains("A máscara de solda"));
        assert_eq!(warning.diagnostic().severity, Severity::Warning);
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
