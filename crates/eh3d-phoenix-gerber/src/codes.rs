// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-GBR-<NNN>` of the Gerber and Excellon import, with their titles in
//! the eight languages of the suite, the error type that carries one and the finding that a read
//! file keeps.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The settings of the import are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-GBR-001";
/// A file is larger than the limit.
pub const INPUT_TOO_LARGE: &str = "PHOENIX-GBR-002";
/// A limit on commands, apertures, objects or points was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-GBR-003";
/// A Gerber command that cannot be read.
pub const GERBER_SYNTAX: &str = "PHOENIX-GBR-004";
/// The coordinate format or the units are missing or not valid.
pub const FORMAT_INVALID: &str = "PHOENIX-GBR-005";
/// An aperture is used without being defined, or its definition is not valid.
pub const APERTURE_INVALID: &str = "PHOENIX-GBR-006";
/// An aperture macro that cannot be read or evaluated.
pub const MACRO_INVALID: &str = "PHOENIX-GBR-007";
/// An arc without a usable centre.
pub const ARC_INVALID: &str = "PHOENIX-GBR-008";
/// A region that is open, too small or holds what it may not.
pub const REGION_INVALID: &str = "PHOENIX-GBR-009";
/// A feature the import does not read; it was skipped.
pub const FEATURE_SKIPPED: &str = "PHOENIX-GBR-010";
/// A coordinate or a size outside the range of the design model.
pub const COORDINATE_RANGE: &str = "PHOENIX-GBR-011";
/// An Excellon line that cannot be read.
pub const DRILL_SYNTAX: &str = "PHOENIX-GBR-012";
/// A drill tool used without being defined, or defined without a diameter.
pub const DRILL_TOOL_INVALID: &str = "PHOENIX-GBR-013";
/// The layer a file belongs to could not be told from its name or its attributes.
pub const ROLE_UNKNOWN: &str = "PHOENIX-GBR-014";
/// A file could not be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-GBR-015";
/// The files read back differ from the board that they were exported from.
pub const ROUND_TRIP_DIFFERENCE: &str = "PHOENIX-GBR-016";
/// A file ends without its end command.
pub const NO_END: &str = "PHOENIX-GBR-017";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-GBR-018";
/// The export that the round trip needs failed.
pub const ROUND_TRIP_FAILED: &str = "PHOENIX-GBR-019";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Gerber import settings are not valid",
            "La configuración de la importación Gerber no es válida",
            "Die Einstellungen des Gerber-Imports sind ungültig",
            "Les réglages de l'import Gerber ne sont pas valides",
            "Le impostazioni dell'importazione Gerber non sono valide",
            "Gerber インポートの設定が無効です",
            "Gerber 导入设置无效",
            "As definições da importação Gerber não são válidas",
        ],
    ),
    (
        INPUT_TOO_LARGE,
        [
            "Input file is larger than the limit",
            "El fichero de entrada supera el límite",
            "Die Eingabedatei ist größer als das Limit",
            "Le fichier d'entrée dépasse la limite",
            "Il file di ingresso supera il limite",
            "入力ファイルが上限を超えています",
            "输入文件超过限制",
            "O ficheiro de entrada excede o limite",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit on commands or objects was exceeded",
            "Se superó un límite de comandos u objetos",
            "Ein Limit für Befehle oder Objekte wurde überschritten",
            "Une limite de commandes ou d'objets a été dépassée",
            "È stato superato un limite di comandi o oggetti",
            "コマンドまたはオブジェクトの上限を超えました",
            "超出了命令或对象的数量限制",
            "Foi excedido um limite de comandos ou objetos",
        ],
    ),
    (
        GERBER_SYNTAX,
        [
            "Gerber syntax error",
            "Error de sintaxis en el Gerber",
            "Syntaxfehler in der Gerber-Datei",
            "Erreur de syntaxe dans le fichier Gerber",
            "Errore di sintassi nel file Gerber",
            "Gerber の構文エラー",
            "Gerber 语法错误",
            "Erro de sintaxe no ficheiro Gerber",
        ],
    ),
    (
        FORMAT_INVALID,
        [
            "Coordinate format or units missing or not valid",
            "Falta el formato de coordenadas o las unidades, o no son válidos",
            "Koordinatenformat oder Einheiten fehlen oder sind ungültig",
            "Format des coordonnées ou unités absents ou non valides",
            "Formato delle coordinate o unità mancanti o non validi",
            "座標フォーマットまたは単位がないか無効です",
            "坐标格式或单位缺失或无效",
            "O formato das coordenadas ou as unidades faltam ou não são válidos",
        ],
    ),
    (
        APERTURE_INVALID,
        [
            "Aperture not defined or not valid",
            "Apertura no definida o no válida",
            "Blende nicht definiert oder ungültig",
            "Ouverture non définie ou non valide",
            "Apertura non definita o non valida",
            "アパーチャが未定義または無効です",
            "光圈未定义或无效",
            "Abertura não definida ou não válida",
        ],
    ),
    (
        MACRO_INVALID,
        [
            "Aperture macro not valid",
            "Macro de apertura no válida",
            "Blendenmakro ungültig",
            "Macro d'ouverture non valide",
            "Macro di apertura non valida",
            "アパーチャマクロが無効です",
            "光圈宏无效",
            "Macro de abertura não válida",
        ],
    ),
    (
        ARC_INVALID,
        [
            "Arc not valid",
            "Arco no válido",
            "Ungültiger Bogen",
            "Arc non valide",
            "Arco non valido",
            "円弧が無効です",
            "圆弧无效",
            "Arco não válido",
        ],
    ),
    (
        REGION_INVALID,
        [
            "Region not valid",
            "Región no válida",
            "Ungültige Region",
            "Région non valide",
            "Regione non valida",
            "リージョンが無効です",
            "区域无效",
            "Região não válida",
        ],
    ),
    (
        FEATURE_SKIPPED,
        [
            "Feature not supported and skipped",
            "Función no admitida y omitida",
            "Nicht unterstützte Funktion übersprungen",
            "Fonction non prise en charge et ignorée",
            "Funzione non supportata e saltata",
            "未対応の機能をスキップしました",
            "不支持的功能已跳过",
            "Funcionalidade não suportada e ignorada",
        ],
    ),
    (
        COORDINATE_RANGE,
        [
            "Coordinate out of range",
            "Coordenada fuera de rango",
            "Koordinate außerhalb des Bereichs",
            "Coordonnée hors plage",
            "Coordinata fuori intervallo",
            "座標が範囲外です",
            "坐标超出范围",
            "Coordenada fora do intervalo",
        ],
    ),
    (
        DRILL_SYNTAX,
        [
            "Excellon syntax error",
            "Error de sintaxis en el Excellon",
            "Syntaxfehler in der Excellon-Datei",
            "Erreur de syntaxe dans le fichier Excellon",
            "Errore di sintassi nel file Excellon",
            "Excellon の構文エラー",
            "Excellon 语法错误",
            "Erro de sintaxe no ficheiro Excellon",
        ],
    ),
    (
        DRILL_TOOL_INVALID,
        [
            "Drill tool not defined or not valid",
            "Herramienta de taladro no definida o no válida",
            "Bohrwerkzeug nicht definiert oder ungültig",
            "Outil de perçage non défini ou non valide",
            "Utensile di foratura non definito o non valido",
            "ドリル工具が未定義または無効です",
            "钻头未定义或无效",
            "Ferramenta de furação não definida ou não válida",
        ],
    ),
    (
        ROLE_UNKNOWN,
        [
            "Layer role not recognised",
            "Función de la capa no reconocida",
            "Funktion der Lage nicht erkannt",
            "Rôle de la couche non reconnu",
            "Ruolo del livello non riconosciuto",
            "レイヤーの役割を認識できません",
            "无法识别层的用途",
            "Função da camada não reconhecida",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "File could not be read",
            "No se pudo leer el fichero",
            "Die Datei konnte nicht gelesen werden",
            "Le fichier n'a pas pu être lu",
            "Il file non può essere letto",
            "ファイルを読み込めません",
            "无法读取文件",
            "Não foi possível ler o ficheiro",
        ],
    ),
    (
        ROUND_TRIP_DIFFERENCE,
        [
            "Imported files differ from the board",
            "Los ficheros importados difieren de la placa",
            "Die importierten Dateien weichen von der Platine ab",
            "Les fichiers importés diffèrent de la carte",
            "I file importati differiscono dalla scheda",
            "インポートしたファイルが基板と異なります",
            "导入的文件与电路板不一致",
            "Os ficheiros importados diferem da placa",
        ],
    ),
    (
        NO_END,
        [
            "File ends without an end command",
            "El fichero termina sin comando de fin",
            "Die Datei endet ohne Endbefehl",
            "Le fichier se termine sans commande de fin",
            "Il file termina senza comando di fine",
            "ファイルに終了コマンドがありません",
            "文件末尾缺少结束命令",
            "O ficheiro termina sem comando de fim",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option not valid",
            "Opción de comando no válida",
            "Ungültige Befehlsoption",
            "Option de commande non valide",
            "Opzione del comando non valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Opção do comando não válida",
        ],
    ),
    (
        ROUND_TRIP_FAILED,
        [
            "Export for the round trip failed",
            "Falló la exportación de la ida y vuelta",
            "Der Export für den Hin-und-zurück-Test ist fehlgeschlagen",
            "L'export de l'aller-retour a échoué",
            "L'esportazione per l'andata e ritorno non è riuscita",
            "往復確認用のエクスポートに失敗しました",
            "往返检查的导出失败",
            "A exportação para a ida e volta falhou",
        ],
    ),
];

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .chain(crate::x2check::TITLES.iter())
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth knowing.
    Info,
    /// The geometry is read but may differ from what the file means.
    Warning,
    /// The file cannot be read.
    Error,
}

impl Severity {
    /// The word printed before a finding.
    pub fn word(self) -> &'static str {
        match self {
            Severity::Info => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// Something found while reading a file: a registered code, a place and what is specific.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, `PHOENIX-GBR-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The line of the file, from 1; 0 when there is none.
    pub line: usize,
    /// What is specific to the case, in English.
    pub detail: String,
}

impl Finding {
    /// A finding with a code, a severity, a line and a detail.
    pub fn new(
        code: &'static str,
        severity: Severity,
        line: usize,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            line,
            detail: detail.into(),
        }
    }

    /// The text in a language: the severity, the code, the title, the line and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!(
            "{} {}: {}",
            self.severity.word(),
            self.code,
            title(self.code, lang)
        );
        if self.line > 0 {
            out.push_str(&format!(" (line {})", self.line));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the import: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GbrError {
    /// The code, `PHOENIX-GBR-<NNN>`.
    pub code: &'static str,
    /// The line of the file, from 1; 0 when there is none.
    pub line: usize,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl GbrError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> GbrError {
        GbrError {
            code,
            line: 0,
            detail: detail.into(),
        }
    }

    /// An error at a line of a file.
    pub fn at(code: &'static str, line: usize, detail: impl Into<String>) -> GbrError {
        GbrError {
            code,
            line,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title, the line and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if self.line > 0 {
            out.push_str(&format!(" (line {})", self.line));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for GbrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{} (line {}): {}", self.code, self.line, self.detail)
        } else {
            write!(f, "{}: {}", self.code, self.detail)
        }
    }
}

impl std::error::Error for GbrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-GBR-{:03}", i + 1));
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
    fn the_message_names_the_code_the_title_the_line_and_the_detail() {
        let e = GbrError::at(GERBER_SYNTAX, 7, "stray text");
        assert_eq!(
            e.message(Lang::En),
            "PHOENIX-GBR-004: Gerber syntax error (line 7): stray text"
        );
        assert_eq!(e.to_string(), "PHOENIX-GBR-004 (line 7): stray text");
        let bare = GbrError::new(NO_END, "");
        assert_eq!(
            bare.message(Lang::Pt),
            "PHOENIX-GBR-017: O ficheiro termina sem comando de fim"
        );
        let f = Finding::new(FEATURE_SKIPPED, Severity::Warning, 0, "SR");
        assert_eq!(
            f.message(Lang::En),
            "warning PHOENIX-GBR-010: Feature not supported and skipped: SR"
        );
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
