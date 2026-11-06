// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the deep reading of EAGLE files, `PHOENIX-EAGLEX-<NNN>`, with their
//! titles in the eight languages of the suite, the error type that carries one and the finding of
//! a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// A design rule file is not valid.
pub const DRU_INVALID: &str = "PHOENIX-EAGLEX-001";
/// A design rule parameter is kept as it is and not understood.
pub const DRU_UNKNOWN: &str = "PHOENIX-EAGLEX-002";
/// A design rule parameter has no equivalent in the rules of this tool.
pub const DRU_NOT_USED: &str = "PHOENIX-EAGLEX-003";
/// Several design rule parameters were folded into one value.
pub const DRU_FOLDED: &str = "PHOENIX-EAGLEX-004";
/// A length cannot be read.
pub const LENGTH_INVALID: &str = "PHOENIX-EAGLEX-005";
/// A rule of this tool has no equivalent among the EAGLE parameters.
pub const RULE_NOT_WRITTEN: &str = "PHOENIX-EAGLEX-006";
/// A CAM job is not valid.
pub const CAM_INVALID: &str = "PHOENIX-EAGLEX-007";
/// A CAM option is not respected by the export profile.
pub const CAM_OPTION_IGNORED: &str = "PHOENIX-EAGLEX-008";
/// A layer that a CAM job asks for is not exported.
pub const CAM_LAYER_NOT_EXPORTED: &str = "PHOENIX-EAGLEX-009";
/// An output device of a CAM job is not supported.
pub const CAM_DEVICE_UNSUPPORTED: &str = "PHOENIX-EAGLEX-010";
/// A variant is not defined in the file.
pub const VARIANT_UNKNOWN: &str = "PHOENIX-EAGLEX-011";
/// The variant data of the file is inconsistent.
pub const VARIANT_INCONSISTENT: &str = "PHOENIX-EAGLEX-012";
/// The layer map settings are not valid.
pub const LAYER_MAP_INVALID: &str = "PHOENIX-EAGLEX-013";
/// A user layer has no mapping.
pub const LAYER_UNMAPPED: &str = "PHOENIX-EAGLEX-014";
/// A layer number is out of range or repeated.
pub const LAYER_NUMBER_INVALID: &str = "PHOENIX-EAGLEX-015";
/// A layer of the file has another name than the standard table gives.
pub const LAYER_RENAMED: &str = "PHOENIX-EAGLEX-016";
/// An element is kept as opaque XML.
pub const KEPT_OPAQUE: &str = "PHOENIX-EAGLEX-017";
/// An element is kept in the tree but simplified by the design model.
pub const SIMPLIFIED_BY_MODEL: &str = "PHOENIX-EAGLEX-018";
/// An element is left out by the design model.
pub const LEFT_OUT_BY_MODEL: &str = "PHOENIX-EAGLEX-019";
/// Rewriting the tree changed the file.
pub const REWRITE_CHANGED: &str = "PHOENIX-EAGLEX-020";
/// A file cannot be read as an EAGLE file.
pub const NOT_EAGLE_FILE: &str = "PHOENIX-EAGLEX-021";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-EAGLEX-022";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-EAGLEX-023";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-EAGLEX-024";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        DRU_INVALID,
        [
            "Design rule file is not valid",
            "El fichero de reglas de diseño no es válido",
            "Die Datei mit den Entwurfsregeln ist ungültig",
            "Le fichier des règles de conception n'est pas valide",
            "Il file delle regole di progetto non è valido",
            "デザインルールファイルが無効です",
            "设计规则文件无效",
            "O ficheiro de regras de projeto não é válido",
        ],
    ),
    (
        DRU_UNKNOWN,
        [
            "Design rule parameter is kept as it is and not understood",
            "Un parámetro de reglas de diseño se conserva tal cual y no se entiende",
            "Ein Entwurfsregel-Parameter wird unverändert behalten und nicht verstanden",
            "Un paramètre de règles de conception est conservé tel quel et non compris",
            "Un parametro delle regole di progetto è conservato così com'è e non è compreso",
            "デザインルールのパラメータはそのまま保持され、解釈されません",
            "设计规则参数原样保留，未被解析",
            "Um parâmetro de regras de projeto é conservado tal como está e não é entendido",
        ],
    ),
    (
        DRU_NOT_USED,
        [
            "Design rule parameter has no equivalent in the rules of this tool",
            "Un parámetro de reglas de diseño no tiene equivalente en las reglas de esta herramienta",
            "Ein Entwurfsregel-Parameter hat in den Regeln dieses Werkzeugs keine Entsprechung",
            "Un paramètre de règles de conception n'a pas d'équivalent dans les règles de cet outil",
            "Un parametro delle regole di progetto non ha equivalente nelle regole di questo strumento",
            "デザインルールのパラメータに対応するこのツールのルールがありません",
            "设计规则参数在本工具的规则中没有对应项",
            "Um parâmetro de regras de projeto não tem equivalente nas regras desta ferramenta",
        ],
    ),
    (
        DRU_FOLDED,
        [
            "Several design rule parameters were folded into one value",
            "Varios parámetros de reglas de diseño se fundieron en un solo valor",
            "Mehrere Entwurfsregel-Parameter wurden zu einem Wert zusammengefasst",
            "Plusieurs paramètres de règles de conception ont été fondus en une seule valeur",
            "Più parametri delle regole di progetto sono stati fusi in un solo valore",
            "複数のデザインルールのパラメータが1つの値にまとめられました",
            "多个设计规则参数被合并为一个值",
            "Vários parâmetros de regras de projeto foram fundidos num só valor",
        ],
    ),
    (
        LENGTH_INVALID,
        [
            "Length cannot be read",
            "No se puede leer la longitud",
            "Die Länge kann nicht gelesen werden",
            "La longueur ne peut pas être lue",
            "La lunghezza non può essere letta",
            "長さを読み取れません",
            "无法读取长度",
            "Não é possível ler o comprimento",
        ],
    ),
    (
        RULE_NOT_WRITTEN,
        [
            "Rule of this tool has no equivalent among the EAGLE parameters",
            "Una regla de esta herramienta no tiene equivalente entre los parámetros de EAGLE",
            "Eine Regel dieses Werkzeugs hat unter den EAGLE-Parametern keine Entsprechung",
            "Une règle de cet outil n'a pas d'équivalent parmi les paramètres d'EAGLE",
            "Una regola di questo strumento non ha equivalente tra i parametri di EAGLE",
            "このツールのルールに対応するEAGLEのパラメータがありません",
            "本工具的规则在 EAGLE 参数中没有对应项",
            "Uma regra desta ferramenta não tem equivalente entre os parâmetros do EAGLE",
        ],
    ),
    (
        CAM_INVALID,
        [
            "CAM job is not valid",
            "El trabajo CAM no es válido",
            "Der CAM-Auftrag ist ungültig",
            "Le travail CAM n'est pas valide",
            "Il lavoro CAM non è valido",
            "CAMジョブが無効です",
            "CAM 作业无效",
            "O trabalho CAM não é válido",
        ],
    ),
    (
        CAM_OPTION_IGNORED,
        [
            "CAM option is not respected by the export profile",
            "El perfil de exportación no respeta una opción CAM",
            "Das Exportprofil berücksichtigt eine CAM-Option nicht",
            "Le profil d'exportation ne respecte pas une option CAM",
            "Il profilo di esportazione non rispetta un'opzione CAM",
            "エクスポートプロファイルはCAMオプションを反映しません",
            "导出配置不遵循某个 CAM 选项",
            "O perfil de exportação não respeita uma opção CAM",
        ],
    ),
    (
        CAM_LAYER_NOT_EXPORTED,
        [
            "Layer that a CAM job asks for is not exported",
            "Una capa que pide el trabajo CAM no se exporta",
            "Eine vom CAM-Auftrag verlangte Lage wird nicht exportiert",
            "Une couche demandée par le travail CAM n'est pas exportée",
            "Un livello richiesto dal lavoro CAM non viene esportato",
            "CAMジョブが要求したレイヤーはエクスポートされません",
            "CAM 作业要求的层不会被导出",
            "Uma camada pedida pelo trabalho CAM não é exportada",
        ],
    ),
    (
        CAM_DEVICE_UNSUPPORTED,
        [
            "Output device of a CAM job is not supported",
            "No se admite el dispositivo de salida de un trabajo CAM",
            "Das Ausgabegerät eines CAM-Auftrags wird nicht unterstützt",
            "Le périphérique de sortie d'un travail CAM n'est pas pris en charge",
            "Il dispositivo di uscita di un lavoro CAM non è supportato",
            "CAMジョブの出力デバイスには対応していません",
            "不支持 CAM 作业的输出设备",
            "O dispositivo de saída de um trabalho CAM não é suportado",
        ],
    ),
    (
        VARIANT_UNKNOWN,
        [
            "Variant is not defined in the file",
            "La variante no está definida en el fichero",
            "Die Variante ist in der Datei nicht definiert",
            "La variante n'est pas définie dans le fichier",
            "La variante non è definita nel file",
            "バリアントがファイルに定義されていません",
            "文件中未定义该变体",
            "A variante não está definida no ficheiro",
        ],
    ),
    (
        VARIANT_INCONSISTENT,
        [
            "Variant data of the file is inconsistent",
            "Los datos de variantes del fichero son incoherentes",
            "Die Variantendaten der Datei sind widersprüchlich",
            "Les données de variantes du fichier sont incohérentes",
            "I dati delle varianti del file sono incoerenti",
            "ファイルのバリアントデータに矛盾があります",
            "文件中的变体数据不一致",
            "Os dados de variantes do ficheiro são incoerentes",
        ],
    ),
    (
        LAYER_MAP_INVALID,
        [
            "Layer map settings are not valid",
            "La configuración del mapa de capas no es válida",
            "Die Einstellungen der Lagenzuordnung sind ungültig",
            "Les réglages de la table des couches ne sont pas valides",
            "Le impostazioni della mappa dei livelli non sono valide",
            "レイヤーマップの設定が無効です",
            "层映射设置无效",
            "As definições do mapa de camadas não são válidas",
        ],
    ),
    (
        LAYER_UNMAPPED,
        [
            "User layer has no mapping",
            "Una capa de usuario no tiene correspondencia",
            "Eine Benutzerlage hat keine Zuordnung",
            "Une couche utilisateur n'a pas de correspondance",
            "Un livello utente non ha corrispondenza",
            "ユーザーレイヤーに対応付けがありません",
            "用户层没有映射",
            "Uma camada de utilizador não tem correspondência",
        ],
    ),
    (
        LAYER_NUMBER_INVALID,
        [
            "Layer number is out of range or repeated",
            "El número de capa está fuera de rango o repetido",
            "Die Lagennummer liegt außerhalb des Bereichs oder ist doppelt",
            "Le numéro de couche est hors limites ou répété",
            "Il numero di livello è fuori intervallo o ripetuto",
            "レイヤー番号が範囲外か重複しています",
            "层号超出范围或重复",
            "O número da camada está fora do intervalo ou repetido",
        ],
    ),
    (
        LAYER_RENAMED,
        [
            "Layer has another name than the standard table gives",
            "Una capa tiene otro nombre que el de la tabla estándar",
            "Eine Lage hat einen anderen Namen als die Standardtabelle",
            "Une couche porte un autre nom que celui de la table standard",
            "Un livello ha un nome diverso da quello della tabella standard",
            "レイヤー名が標準の表と異なります",
            "层名与标准表不同",
            "Uma camada tem outro nome que não o da tabela padrão",
        ],
    ),
    (
        KEPT_OPAQUE,
        [
            "Element is kept as opaque XML",
            "El elemento se conserva como XML opaco",
            "Das Element bleibt als opakes XML erhalten",
            "L'élément est conservé comme XML opaque",
            "L'elemento è conservato come XML opaco",
            "要素は不透明なXMLとして保持されます",
            "该元素作为不透明 XML 保留",
            "O elemento é conservado como XML opaco",
        ],
    ),
    (
        SIMPLIFIED_BY_MODEL,
        [
            "Element is kept in the tree but simplified by the design model",
            "El elemento se conserva en el árbol pero el modelo de diseño lo simplifica",
            "Das Element bleibt im Baum erhalten, das Entwurfsmodell vereinfacht es aber",
            "L'élément reste dans l'arbre mais le modèle de conception le simplifie",
            "L'elemento resta nell'albero ma il modello di progetto lo semplifica",
            "要素はツリーに保持されますが、設計モデルでは簡略化されます",
            "该元素保留在树中，但设计模型对其做了简化",
            "O elemento fica na árvore mas o modelo de projeto simplifica-o",
        ],
    ),
    (
        LEFT_OUT_BY_MODEL,
        [
            "Element is left out by the design model",
            "El modelo de diseño deja fuera el elemento",
            "Das Entwurfsmodell lässt das Element weg",
            "Le modèle de conception laisse l'élément de côté",
            "Il modello di progetto lascia fuori l'elemento",
            "設計モデルはこの要素を含めません",
            "设计模型不包含该元素",
            "O modelo de projeto deixa o elemento de fora",
        ],
    ),
    (
        REWRITE_CHANGED,
        [
            "Rewriting the tree changed the file",
            "Reescribir el árbol cambió el fichero",
            "Das Zurückschreiben des Baums hat die Datei verändert",
            "Réécrire l'arbre a modifié le fichier",
            "Riscrivere l'albero ha modificato il file",
            "ツリーを書き戻すとファイルが変わりました",
            "重写树改变了文件",
            "Reescrever a árvore alterou o ficheiro",
        ],
    ),
    (
        NOT_EAGLE_FILE,
        [
            "File cannot be read as an EAGLE file",
            "El fichero no se puede leer como fichero de EAGLE",
            "Die Datei kann nicht als EAGLE-Datei gelesen werden",
            "Le fichier ne peut pas être lu comme un fichier EAGLE",
            "Il file non può essere letto come file EAGLE",
            "ファイルをEAGLEファイルとして読み取れません",
            "无法将该文件作为 EAGLE 文件读取",
            "O ficheiro não pode ser lido como ficheiro do EAGLE",
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
            "The output file exists or cannot be written",
            "El fichero de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non può essere scritto",
            "出力ファイルが既に存在するか、書き込めません",
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
            "Le fichier d'entrée ne peut pas être lu",
            "Il file di ingresso non può essere letto",
            "入力ファイルを読み取れません",
            "无法读取输入文件",
            "Não é possível ler o ficheiro de entrada",
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
    /// Something that was done on purpose and that the user may want to know.
    Info,
    /// Something that is lost or changed.
    Warning,
    /// Something that makes the result unusable.
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

/// An error of the deep reading: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepError {
    /// The code, `PHOENIX-EAGLEX-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl DeepError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> DeepError {
        DeepError {
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

impl fmt::Display for DeepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for DeepError {}

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-EAGLEX-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The parameter, element, layer or option it is about, by name.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a parameter, an element, a layer or an option.
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

/// The lines of a list of findings in a language, one per line.
pub fn lines(findings: &[Finding], lang: Lang) -> String {
    let mut out = String::new();
    for f in findings {
        out.push_str(&f.line(lang));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-EAGLEX-{:03}", i + 1));
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
        let e = DeepError::new(VARIANT_UNKNOWN, "ASSEMBLY_B");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-EAGLEX-011: A variante não está definida no ficheiro: ASSEMBLY_B"
        );
        assert_eq!(e.to_string(), "PHOENIX-EAGLEX-011: ASSEMBLY_B");
        let bare = DeepError::new(OPTION_INVALID, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-EAGLEX-022: A command option is not valid"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(
            DRU_NOT_USED,
            Severity::Info,
            Some("psTop"),
            "shape of the pads",
        );
        assert_eq!(
            f.line(Lang::En),
            "info: PHOENIX-EAGLEX-003: Design rule parameter has no equivalent in the rules of this tool [psTop]: shape of the pads"
        );
        assert!(Severity::Error > Severity::Info);
        assert_eq!(title("PHOENIX-EAGLEX-999", Lang::En), "");
        assert_eq!(lines(std::slice::from_ref(&f), Lang::En).lines().count(), 1);
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
