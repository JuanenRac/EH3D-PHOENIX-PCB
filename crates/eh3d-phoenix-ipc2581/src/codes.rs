// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the IPC-2581 writer, `PHOENIX-IPC2581-<NNN>`, with their titles in the
//! eight languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The IPC-2581 settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-IPC2581-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-IPC2581-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-IPC2581-003";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-IPC2581-004";
/// A limit of the export was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-IPC2581-005";
/// Parts of the board are left out of the file.
pub const LEFT_OUT: &str = "PHOENIX-IPC2581-006";
/// The board has no outline to write.
pub const NO_OUTLINE: &str = "PHOENIX-IPC2581-007";
/// A pad is on no layer that the file holds.
pub const PAD_NO_LAYER: &str = "PHOENIX-IPC2581-008";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "IPC-2581 settings are not valid",
            "La configuración de IPC-2581 no es válida",
            "Die Einstellungen für IPC-2581 sind ungültig",
            "Les réglages d'IPC-2581 ne sont pas valides",
            "Le impostazioni di IPC-2581 non sono valide",
            "IPC-2581 の設定が無効です",
            "IPC-2581 设置无效",
            "As definições do IPC-2581 não são válidas",
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
            "A limit of the export was exceeded",
            "Se superó un límite de la exportación",
            "Eine Grenze des Exports wurde überschritten",
            "Une limite de l'export a été dépassée",
            "È stato superato un limite dell'esportazione",
            "エクスポートの上限を超えました",
            "超出了导出的限制",
            "Foi excedido um limite da exportação",
        ],
    ),
    (
        LEFT_OUT,
        [
            "Parts of the board are left out of the file",
            "Partes de la placa quedan fuera del archivo",
            "Teile der Platine fehlen in der Datei",
            "Des éléments de la carte ne sont pas dans le fichier",
            "Parti della scheda restano fuori dal file",
            "基板の一部がファイルに含まれていません",
            "电路板的部分内容未写入文件",
            "Partes da placa ficam fora do ficheiro",
        ],
    ),
    (
        NO_OUTLINE,
        [
            "The board has no outline to write",
            "La placa no tiene contorno que escribir",
            "Die Platine hat keine Kontur zum Schreiben",
            "La carte n'a pas de contour à écrire",
            "La scheda non ha un contorno da scrivere",
            "書き出す基板外形がありません",
            "电路板没有可写入的轮廓",
            "A placa não tem contorno para escrever",
        ],
    ),
    (
        PAD_NO_LAYER,
        [
            "A pad is on no layer that the file holds",
            "Un pad no está en ninguna capa que contenga el archivo",
            "Ein Pad liegt auf keiner Lage, die die Datei enthält",
            "Une pastille n'est sur aucune couche du fichier",
            "Una piazzola non è su alcuno strato presente nel file",
            "ファイルにある層のどれにもないパッドがあります",
            "有焊盘不在文件包含的任何层上",
            "Uma pastilha não está em nenhuma camada do ficheiro",
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
    /// Something the export did on its own that the user may want to know.
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

/// A finding of an export: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-IPC2581-<NNN>`.
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

/// An error of the IPC-2581 writer: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipc2581Error {
    /// The code, `PHOENIX-IPC2581-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Ipc2581Error {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> Ipc2581Error {
        Ipc2581Error {
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

impl fmt::Display for Ipc2581Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for Ipc2581Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-IPC2581-{:03}", i + 1));
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
        let e = Ipc2581Error::new(OUTPUT_FAILED, "a.xml");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-IPC2581-004: O ficheiro de saída já existe ou não pode ser escrito: a.xml"
        );
        assert_eq!(e.to_string(), "PHOENIX-IPC2581-004: a.xml");
        let bare = Ipc2581Error::new(NO_OUTLINE, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-IPC2581-007: The board has no outline to write"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(LEFT_OUT, Severity::Info, None, "3 silkscreen items");
        assert_eq!(
            f.line(Lang::En),
            "info: PHOENIX-IPC2581-006: Parts of the board are left out of the file: 3 silkscreen items"
        );
        let g = Finding::new(PAD_NO_LAYER, Severity::Warning, Some("R1:1"), "");
        assert_eq!(
            g.line(Lang::En),
            "warning: PHOENIX-IPC2581-008: A pad is on no layer that the file holds [R1:1]"
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
