// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the 3D assembly, `PHOENIX-3D-<NNN>`, with their titles in the eight
//! languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The assembly settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-3D-001";
/// An input file is larger than the limit.
pub const FILE_TOO_LARGE: &str = "PHOENIX-3D-002";
/// A STEP file is not valid.
pub const STEP_INVALID: &str = "PHOENIX-3D-003";
/// The length unit of a STEP file is not supported.
pub const STEP_UNIT: &str = "PHOENIX-3D-004";
/// A STEP solid was replaced by its enclosing box.
pub const STEP_REPLACED: &str = "PHOENIX-3D-005";
/// A STEP file has no geometry.
pub const STEP_EMPTY: &str = "PHOENIX-3D-006";
/// A footprint has no model.
pub const MODEL_MISSING: &str = "PHOENIX-3D-007";
/// The table of models is not valid.
pub const TABLE_INVALID: &str = "PHOENIX-3D-008";
/// A model file cannot be read.
pub const MODEL_UNREADABLE: &str = "PHOENIX-3D-009";
/// Two components collide.
pub const COLLISION: &str = "PHOENIX-3D-010";
/// A component is taller than the limit.
pub const TOO_TALL: &str = "PHOENIX-3D-011";
/// A component is not inside the board outline.
pub const OUTSIDE_OUTLINE: &str = "PHOENIX-3D-012";
/// The thickness of the board is not known.
pub const THICKNESS_DEFAULT: &str = "PHOENIX-3D-013";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-3D-014";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-3D-015";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-3D-016";
/// A limit was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-3D-017";
/// The export is a simplified model.
pub const EXPORT_SIMPLIFIED: &str = "PHOENIX-3D-018";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Assembly settings are not valid",
            "La configuración del ensamblado no es válida",
            "Die Einstellungen der Baugruppe sind ungültig",
            "Les réglages de l'assemblage ne sont pas valides",
            "Le impostazioni dell'assemblaggio non sono valide",
            "アセンブリ設定が無効です",
            "装配设置无效",
            "As definições da montagem não são válidas",
        ],
    ),
    (
        FILE_TOO_LARGE,
        [
            "Input file is larger than the limit",
            "El archivo de entrada supera el límite",
            "Die Eingabedatei ist größer als das Limit",
            "Le fichier d'entrée dépasse la limite",
            "Il file di input supera il limite",
            "入力ファイルが上限を超えています",
            "输入文件超过限制",
            "O ficheiro de entrada excede o limite",
        ],
    ),
    (
        STEP_INVALID,
        [
            "STEP file is not valid",
            "El archivo STEP no es válido",
            "Die STEP-Datei ist ungültig",
            "Le fichier STEP n'est pas valide",
            "Il file STEP non è valido",
            "STEPファイルが無効です",
            "STEP文件无效",
            "O ficheiro STEP não é válido",
        ],
    ),
    (
        STEP_UNIT,
        [
            "Length unit of the STEP file is not supported",
            "La unidad de longitud del archivo STEP no es compatible",
            "Die Längeneinheit der STEP-Datei wird nicht unterstützt",
            "L'unité de longueur du fichier STEP n'est pas prise en charge",
            "L'unità di lunghezza del file STEP non è supportata",
            "STEPファイルの長さの単位は未対応です",
            "不支持STEP文件的长度单位",
            "A unidade de comprimento do ficheiro STEP não é suportada",
        ],
    ),
    (
        STEP_REPLACED,
        [
            "STEP solid replaced by its enclosing box",
            "El sólido STEP se sustituye por su caja envolvente",
            "Der STEP-Körper wird durch seinen umhüllenden Quader ersetzt",
            "Le solide STEP est remplacé par sa boîte englobante",
            "Il solido STEP è sostituito dal suo parallelepipedo di ingombro",
            "STEPソリッドを外接ボックスに置き換えました",
            "STEP实体已替换为其包围盒",
            "O sólido STEP foi substituído pela sua caixa envolvente",
        ],
    ),
    (
        STEP_EMPTY,
        [
            "STEP file has no geometry",
            "El archivo STEP no tiene geometría",
            "Die STEP-Datei enthält keine Geometrie",
            "Le fichier STEP ne contient pas de géométrie",
            "Il file STEP non contiene geometria",
            "STEPファイルにジオメトリがありません",
            "STEP文件没有几何体",
            "O ficheiro STEP não tem geometria",
        ],
    ),
    (
        MODEL_MISSING,
        [
            "Footprint has no model",
            "La huella no tiene modelo",
            "Der Footprint hat kein Modell",
            "L'empreinte n'a pas de modèle",
            "L'impronta non ha un modello",
            "フットプリントにモデルがありません",
            "封装没有模型",
            "A pegada não tem modelo",
        ],
    ),
    (
        TABLE_INVALID,
        [
            "Table of models is not valid",
            "La tabla de modelos no es válida",
            "Die Modelltabelle ist ungültig",
            "La table des modèles n'est pas valide",
            "La tabella dei modelli non è valida",
            "モデル表が無効です",
            "模型表无效",
            "A tabela de modelos não é válida",
        ],
    ),
    (
        MODEL_UNREADABLE,
        [
            "Model file cannot be read",
            "No se puede leer el archivo del modelo",
            "Die Modelldatei kann nicht gelesen werden",
            "Le fichier du modèle est illisible",
            "Il file del modello non è leggibile",
            "モデルファイルを読み込めません",
            "无法读取模型文件",
            "Não é possível ler o ficheiro do modelo",
        ],
    ),
    (
        COLLISION,
        [
            "Components collide",
            "Los componentes colisionan",
            "Bauteile kollidieren",
            "Les composants entrent en collision",
            "I componenti collidono",
            "部品が干渉しています",
            "元件发生碰撞",
            "Os componentes colidem",
        ],
    ),
    (
        TOO_TALL,
        [
            "Component is taller than the limit",
            "El componente supera la altura máxima",
            "Das Bauteil überschreitet die Höchsthöhe",
            "Le composant dépasse la hauteur maximale",
            "Il componente supera l'altezza massima",
            "部品が高さ上限を超えています",
            "元件超过最大高度",
            "O componente excede a altura máxima",
        ],
    ),
    (
        OUTSIDE_OUTLINE,
        [
            "Component is not inside the board outline",
            "El componente no está dentro del contorno de la placa",
            "Das Bauteil liegt nicht innerhalb des Platinenumrisses",
            "Le composant n'est pas dans le contour de la carte",
            "Il componente non è dentro il contorno della scheda",
            "部品が基板外形の内側にありません",
            "元件不在板框内",
            "O componente não está dentro do contorno da placa",
        ],
    ),
    (
        THICKNESS_DEFAULT,
        [
            "Board thickness is not known, the default is used",
            "Se desconoce el grosor de la placa, se usa el predeterminado",
            "Die Platinendicke ist unbekannt, der Standardwert wird verwendet",
            "L'épaisseur de la carte est inconnue, la valeur par défaut est utilisée",
            "Lo spessore della scheda è sconosciuto, si usa quello predefinito",
            "基板の厚みが不明なため既定値を使用します",
            "板厚未知，使用默认值",
            "A espessura da placa é desconhecida, é usada a predefinida",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
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
            "The output file exists or cannot be written",
            "El archivo de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di output esiste o non può essere scritto",
            "出力ファイルが既に存在するか書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un archivo de entrada",
            "Eine Eingabedatei kann nicht gelesen werden",
            "Un fichier d'entrée est illisible",
            "Un file di input non è leggibile",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the assembly was exceeded",
            "Se superó un límite del ensamblado",
            "Ein Limit der Baugruppe wurde überschritten",
            "Une limite de l'assemblage a été dépassée",
            "Un limite dell'assemblaggio è stato superato",
            "アセンブリの上限を超えました",
            "超出了装配的限制",
            "Foi excedido um limite da montagem",
        ],
    ),
    (
        EXPORT_SIMPLIFIED,
        [
            "The export is a simplified model",
            "La exportación es un modelo simplificado",
            "Der Export ist ein vereinfachtes Modell",
            "L'export est un modèle simplifié",
            "L'esportazione è un modello semplificato",
            "エクスポートは簡略化モデルです",
            "导出的是简化模型",
            "A exportação é um modelo simplificado",
        ],
    ),
];

/// The title of a code in a language, of the assembly (`PHOENIX-3D-<NNN>`) or of the check against
/// an enclosure (`PHOENIX-ENCL-<NNN>`); empty for a code that is in neither table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .chain(crate::enclosure_codes::TITLES.iter())
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something the report did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// Something that makes the report wrong or incomplete.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, `PHOENIX-3D-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The part or the line it is about.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a part or a line.
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

/// An error of the assembly: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyError {
    /// The code, `PHOENIX-3D-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl AssemblyError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> AssemblyError {
        AssemblyError {
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

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for AssemblyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-3D-{:03}", i + 1));
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
        let e = AssemblyError::new(MODEL_MISSING, "R1");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-3D-007: A pegada não tem modelo: R1"
        );
        assert_eq!(e.to_string(), "PHOENIX-3D-007: R1");
        let bare = AssemblyError::new(MODEL_MISSING, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-3D-007: Footprint has no model"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(COLLISION, Severity::Error, Some("R1 and R2"), "0.5 mm");
        assert_eq!(
            f.line(Lang::En),
            "error: PHOENIX-3D-010: Components collide [R1 and R2]: 0.5 mm"
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
