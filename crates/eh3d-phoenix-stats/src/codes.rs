// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the statistics, `PHOENIX-STATS-<NNN>`, with their titles in the eight
//! languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The statistics settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-STATS-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-STATS-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-STATS-003";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-STATS-004";
/// A limit of the report was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-STATS-005";
/// The area of the zones is the area of their outlines.
pub const ZONE_OUTLINE_COUNTED: &str = "PHOENIX-STATS-006";
/// The board outline encloses no area.
pub const NO_BOARD_AREA: &str = "PHOENIX-STATS-007";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Statistics settings are not valid",
            "La configuración de las estadísticas no es válida",
            "Die Einstellungen der Statistik sind ungültig",
            "Les réglages des statistiques ne sont pas valides",
            "Le impostazioni delle statistiche non sono valide",
            "統計の設定が無効です",
            "统计设置无效",
            "As definições das estatísticas não são válidas",
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
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un archivo de entrada",
            "Eine Eingabedatei kann nicht gelesen werden",
            "Un fichier d'entrée ne peut pas être lu",
            "Non si può leggere un file di ingresso",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "The output file exists or cannot be written",
            "El archivo de salida ya existe o no se puede escribir",
            "Die Ausgabedatei existiert bereits oder kann nicht geschrieben werden",
            "Le fichier de sortie existe déjà ou ne peut pas être écrit",
            "Il file di uscita esiste già o non si può scrivere",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the report was exceeded",
            "Se superó un límite del informe",
            "Eine Grenze des Berichts wurde überschritten",
            "Une limite du rapport a été dépassée",
            "È stato superato un limite del rapporto",
            "レポートの上限を超えました",
            "超出了报告的限制",
            "Foi excedido um limite do relatório",
        ],
    ),
    (
        ZONE_OUTLINE_COUNTED,
        [
            "Zone copper is counted as the whole outline, before clearances",
            "El cobre de las zonas se cuenta como todo el contorno, antes de las holguras",
            "Das Zonenkupfer wird als ganze Kontur gezählt, vor den Abständen",
            "Le cuivre des zones est compté comme tout le contour, avant les dégagements",
            "Il rame delle zone è contato come tutto il contorno, prima delle distanze",
            "ゾーンの銅は、クリアランスを引く前の輪郭全体として数えています",
            "铜区按整个轮廓计算，未扣除间距",
            "O cobre das zonas conta-se como todo o contorno, antes das folgas",
        ],
    ),
    (
        NO_BOARD_AREA,
        [
            "The board outline encloses no area",
            "El contorno de la placa no encierra ninguna área",
            "Die Platinenkontur schließt keine Fläche ein",
            "Le contour de la carte ne délimite aucune surface",
            "Il contorno della scheda non racchiude alcuna area",
            "基板の外形が面積を囲んでいません",
            "电路板轮廓没有围成任何面积",
            "O contorno da placa não delimita nenhuma área",
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

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something the report did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// A rule that is proven to be broken.
    Error,
}

impl Severity {
    /// The word used in the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-STATS-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The layer or object it is about, by name.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a layer or an object.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: Option<&str>,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            subject: subject.map(str::to_owned),
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title, the subject and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if let Some(subject) = &self.subject {
            out.push_str(&format!(" [{subject}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the statistics: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsError {
    /// The code, `PHOENIX-STATS-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl StatsError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> StatsError {
        StatsError {
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

impl fmt::Display for StatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for StatsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-STATS-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
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
        let e = StatsError::new(LIMIT_EXCEEDED, "9 objects");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-STATS-005: Foi excedido um limite do relatório: 9 objects"
        );
        assert_eq!(e.to_string(), "PHOENIX-STATS-005: 9 objects");
        let bare = StatsError::new(NO_BOARD_AREA, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-STATS-007: The board outline encloses no area"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(ZONE_OUTLINE_COUNTED, Severity::Info, Some("Top"), "1 zone");
        assert_eq!(
            f.line(Lang::En),
            "info: PHOENIX-STATS-006: Zone copper is counted as the whole outline, before clearances [Top]: 1 zone"
        );
        assert!(Severity::Error > Severity::Info);
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
