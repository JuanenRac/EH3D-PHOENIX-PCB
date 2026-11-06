// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-DXF-<NNN>` of the DXF reader and writer, with their titles in the
//! eight languages of the suite, the error that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The DXF settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-DXF-001";
/// A limit of the reader was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-DXF-002";
/// A DXF file is not valid.
pub const DXF_INVALID: &str = "PHOENIX-DXF-003";
/// An entity is not understood and was left out.
pub const ENTITY_UNSUPPORTED: &str = "PHOENIX-DXF-004";
/// The unit of the drawing is not known.
pub const UNIT_UNKNOWN: &str = "PHOENIX-DXF-005";
/// A spline was approximated by straight pieces.
pub const SPLINE_APPROXIMATED: &str = "PHOENIX-DXF-006";
/// A block or an insert cannot be used.
pub const BLOCK_UNUSABLE: &str = "PHOENIX-DXF-007";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-DXF-008";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-DXF-009";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-DXF-010";
/// A chain of lines and arcs is not closed.
pub const CHAIN_OPEN: &str = "PHOENIX-DXF-011";
/// A duplicate entity was dropped.
pub const DUPLICATE_DROPPED: &str = "PHOENIX-DXF-012";
/// A contour crosses itself or another contour.
pub const CONTOUR_CROSSES: &str = "PHOENIX-DXF-013";
/// No closed contour was found.
pub const NO_CONTOUR: &str = "PHOENIX-DXF-014";
/// Cutouts inside the outline (kept by the board).
pub const CUTOUT_NOT_KEPT: &str = "PHOENIX-DXF-015";
/// There is more than one outer contour.
pub const SEVERAL_OUTER: &str = "PHOENIX-DXF-016";
/// A contour is degenerate.
pub const CONTOUR_DEGENERATE: &str = "PHOENIX-DXF-017";
/// Arcs were approximated by straight pieces.
pub const ARCS_APPROXIMATED: &str = "PHOENIX-DXF-018";
/// The DXF holds outlines and marks only.
pub const EXPORT_SIMPLIFIED: &str = "PHOENIX-DXF-019";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "DXF settings are not valid",
            "La configuración de DXF no es válida",
            "Die DXF-Einstellungen sind ungültig",
            "Les réglages DXF ne sont pas valides",
            "Le impostazioni DXF non sono valide",
            "DXF設定が無効です",
            "DXF设置无效",
            "As definições de DXF não são válidas",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the DXF reader was exceeded",
            "Se superó un límite del lector DXF",
            "Ein Limit des DXF-Lesers wurde überschritten",
            "Une limite du lecteur DXF a été dépassée",
            "È stato superato un limite del lettore DXF",
            "DXFリーダーの上限を超えました",
            "超出了DXF读取器的限制",
            "Foi excedido um limite do leitor de DXF",
        ],
    ),
    (
        DXF_INVALID,
        [
            "DXF file is not valid",
            "El fichero DXF no es válido",
            "Die DXF-Datei ist ungültig",
            "Le fichier DXF n'est pas valide",
            "Il file DXF non è valido",
            "DXFファイルが無効です",
            "DXF文件无效",
            "O ficheiro DXF não é válido",
        ],
    ),
    (
        ENTITY_UNSUPPORTED,
        [
            "DXF entity is not understood and was left out",
            "Una entidad DXF no se entiende y se dejó fuera",
            "Eine DXF-Entität wird nicht verstanden und wurde ausgelassen",
            "Une entité DXF n'est pas comprise et a été laissée de côté",
            "Un'entità DXF non è compresa ed è stata omessa",
            "DXFエンティティを解釈できないため除外しました",
            "无法识别的DXF实体已被略过",
            "Uma entidade DXF não é compreendida e foi deixada de fora",
        ],
    ),
    (
        UNIT_UNKNOWN,
        [
            "Drawing unit is not known, the default unit is used",
            "La unidad del dibujo no se conoce, se usa la unidad por defecto",
            "Die Zeicheneinheit ist unbekannt, die Standardeinheit wird verwendet",
            "L'unité du dessin est inconnue, l'unité par défaut est utilisée",
            "L'unità del disegno non è nota, si usa quella predefinita",
            "図面の単位が不明なため既定の単位を使います",
            "图纸单位未知，使用默认单位",
            "A unidade do desenho não é conhecida, usa-se a unidade por omissão",
        ],
    ),
    (
        SPLINE_APPROXIMATED,
        [
            "Spline was approximated by straight pieces",
            "La spline se aproximó con tramos rectos",
            "Der Spline wurde durch Geradenstücke angenähert",
            "La spline a été approchée par des segments droits",
            "La spline è stata approssimata con tratti rettilinei",
            "スプラインを直線で近似しました",
            "样条曲线已用直线段近似",
            "A spline foi aproximada por troços retos",
        ],
    ),
    (
        BLOCK_UNUSABLE,
        [
            "Block or insert cannot be used",
            "Un bloque o inserción no se puede usar",
            "Ein Block oder eine Einfügung ist nicht verwendbar",
            "Un bloc ou une insertion est inutilisable",
            "Un blocco o un inserimento non è utilizzabile",
            "ブロックまたは挿入を使用できません",
            "无法使用块或插入",
            "Um bloco ou inserção não pode ser usado",
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
            "Output file exists or cannot be written",
            "El fichero de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non può essere scritto",
            "出力ファイルが存在するか書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Input file cannot be read",
            "No se puede leer el fichero de entrada",
            "Die Eingabedatei kann nicht gelesen werden",
            "Le fichier d'entrée est illisible",
            "Il file di ingresso non è leggibile",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler o ficheiro de entrada",
        ],
    ),
    (
        CHAIN_OPEN,
        [
            "Chain of lines and arcs is not closed",
            "Una cadena de líneas y arcos no está cerrada",
            "Eine Kette aus Linien und Bögen ist nicht geschlossen",
            "Une chaîne de lignes et d'arcs n'est pas fermée",
            "Una catena di linee e archi non è chiusa",
            "線と円弧の連なりが閉じていません",
            "线段与圆弧链未闭合",
            "Uma cadeia de linhas e arcos não está fechada",
        ],
    ),
    (
        DUPLICATE_DROPPED,
        [
            "Duplicate entity was dropped",
            "Se descartó una entidad duplicada",
            "Eine doppelte Entität wurde verworfen",
            "Une entité en double a été écartée",
            "Un'entità duplicata è stata scartata",
            "重複したエンティティを破棄しました",
            "已丢弃重复的实体",
            "Uma entidade duplicada foi descartada",
        ],
    ),
    (
        CONTOUR_CROSSES,
        [
            "Contour crosses itself or another contour",
            "Un contorno se cruza consigo mismo o con otro",
            "Eine Kontur kreuzt sich selbst oder eine andere Kontur",
            "Un contour se croise lui-même ou croise un autre contour",
            "Un contorno si interseca con sé stesso o con un altro",
            "輪郭が自己交差または他の輪郭と交差しています",
            "轮廓自相交或与其他轮廓相交",
            "Um contorno cruza-se a si próprio ou com outro",
        ],
    ),
    (
        NO_CONTOUR,
        [
            "No closed contour was found",
            "No se encontró ningún contorno cerrado",
            "Es wurde keine geschlossene Kontur gefunden",
            "Aucun contour fermé n'a été trouvé",
            "Non è stato trovato alcun contorno chiuso",
            "閉じた輪郭が見つかりません",
            "未找到闭合轮廓",
            "Não foi encontrado nenhum contorno fechado",
        ],
    ),
    (
        CUTOUT_NOT_KEPT,
        [
            "Cutouts inside the outline",
            "Recortes dentro del contorno",
            "Ausschnitte innerhalb des Umrisses",
            "Découpes à l'intérieur du contour",
            "Ritagli all'interno del contorno",
            "外形の内側の切り欠き",
            "轮廓内部的开孔",
            "Recortes dentro do contorno",
        ],
    ),
    (
        SEVERAL_OUTER,
        [
            "More than one outer contour, the largest is used",
            "Hay más de un contorno exterior, se usa el mayor",
            "Mehr als eine Außenkontur, die größte wird verwendet",
            "Plus d'un contour extérieur, le plus grand est utilisé",
            "Più di un contorno esterno, si usa il più grande",
            "外形が複数あるため最大のものを使います",
            "存在多个外轮廓，使用最大的一个",
            "Há mais de um contorno exterior, usa-se o maior",
        ],
    ),
    (
        CONTOUR_DEGENERATE,
        [
            "Contour is degenerate",
            "El contorno está degenerado",
            "Die Kontur ist entartet",
            "Le contour est dégénéré",
            "Il contorno è degenere",
            "輪郭が退化しています",
            "轮廓退化",
            "O contorno é degenerado",
        ],
    ),
    (
        ARCS_APPROXIMATED,
        [
            "Arcs were approximated by straight pieces",
            "Los arcos se aproximaron con tramos rectos",
            "Bögen wurden durch Geradenstücke angenähert",
            "Les arcs ont été approchés par des segments droits",
            "Gli archi sono stati approssimati con tratti rettilinei",
            "円弧を直線で近似しました",
            "圆弧已用直线段近似",
            "Os arcos foram aproximados por troços retos",
        ],
    ),
    (
        EXPORT_SIMPLIFIED,
        [
            "The DXF holds outlines and marks only",
            "El DXF contiene solo contornos y marcas",
            "Die DXF-Datei enthält nur Konturen und Marken",
            "Le DXF ne contient que des contours et des repères",
            "Il DXF contiene solo contorni e marcature",
            "DXFには輪郭とマークのみが含まれます",
            "DXF仅包含轮廓和标记",
            "O DXF contém apenas contornos e marcas",
        ],
    ),
];

/// The codes of this crate with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something to know.
    Info,
    /// Something left out or approximated.
    Warning,
    /// Something that stops the use of the result.
    Error,
}

impl Severity {
    /// The word that names the severity in reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One line of a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What it is about (an entity type, a layer, a place), when there is one.
    pub subject: Option<String>,
    /// The detail, in English.
    pub detail: String,
}

impl Finding {
    /// A finding.
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

    /// The finding as one line, with the title in `lang`.
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

/// The findings of a run, with a limit: the findings past it are counted and not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The findings kept, in the order they were made.
    pub findings: Vec<Finding>,
    /// How many were counted and not listed.
    pub omitted: usize,
    limit: usize,
}

impl Report {
    /// An empty report that keeps at most `limit` findings.
    pub fn new(limit: usize) -> Report {
        Report {
            findings: Vec::new(),
            omitted: 0,
            limit,
        }
    }

    /// Adds a finding, or counts it when the limit is reached.
    pub fn push(&mut self, finding: Finding) {
        if self.findings.len() < self.limit {
            self.findings.push(finding);
        } else {
            self.omitted += 1;
        }
    }

    /// True when a finding is an error.
    pub fn has_error(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// An error with its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxfError {
    /// The code.
    pub code: &'static str,
    /// The detail, in English.
    pub detail: String,
}

impl DxfError {
    /// An error.
    pub fn new(code: &'static str, detail: impl Into<String>) -> DxfError {
        DxfError {
            code,
            detail: detail.into(),
        }
    }

    /// The error as a message, with the title in `lang`.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for DxfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for DxfError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-DXF-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(registry().len(), 19);
    }

    #[test]
    fn the_message_and_the_line_name_the_code_the_title_and_the_detail() {
        let e = DxfError::new(NO_CONTOUR, "layer OUTLINE");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-DXF-014: Não foi encontrado nenhum contorno fechado: layer OUTLINE"
        );
        assert_eq!(e.to_string(), "PHOENIX-DXF-014: layer OUTLINE");
        let f = Finding::new(CHAIN_OPEN, Severity::Warning, Some("chain 1"), "2 pieces");
        assert_eq!(
            f.line(Lang::En),
            "warning: PHOENIX-DXF-011: Chain of lines and arcs is not closed [chain 1]: 2 pieces"
        );
        assert!(Severity::Error > Severity::Warning);
        assert_eq!(title("PHOENIX-DXF-999", Lang::En), "");
    }

    #[test]
    fn the_report_keeps_a_number_of_findings_and_counts_the_rest() {
        let mut report = Report::new(2);
        for _ in 0..5 {
            report.push(Finding::new(DUPLICATE_DROPPED, Severity::Info, None, ""));
        }
        assert_eq!((report.findings.len(), report.omitted), (2, 3));
        assert!(!report.has_error());
        report.push(Finding::new(NO_CONTOUR, Severity::Error, None, ""));
        assert_eq!(report.omitted, 4);
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
