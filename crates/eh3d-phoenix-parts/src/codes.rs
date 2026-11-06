// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the parts base, `PHOENIX-PARTS-<NNN>`, with their titles in the eight
//! languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

/// The settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-PARTS-001";
/// A file of the base cannot be read, or is larger than the limit.
pub const FILE_UNUSABLE: &str = "PHOENIX-PARTS-002";
/// A file of the base is not valid (the detail gives the line or the part).
pub const FORMAT_INVALID: &str = "PHOENIX-PARTS-003";
/// A limit of the settings was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-PARTS-004";
/// An option of the command is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-PARTS-005";
/// An output file exists, or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-PARTS-006";
/// An id is used by two parts.
pub const ID_DUPLICATE: &str = "PHOENIX-PARTS-007";
/// The footprint of a part is not in the IPC library or in the own libraries.
pub const FOOTPRINT_UNKNOWN: &str = "PHOENIX-PARTS-008";
/// Two parts have the same part number and different data.
pub const MPN_CONFLICT: &str = "PHOENIX-PARTS-009";
/// A price has no currency.
pub const PRICE_NO_CURRENCY: &str = "PHOENIX-PARTS-010";
/// The price breaks of an offer are not in ascending order.
pub const TIERS_UNORDERED: &str = "PHOENIX-PARTS-011";
/// The simulation model of a part is not in the table of models.
pub const SIM_MODEL_UNKNOWN: &str = "PHOENIX-PARTS-012";
/// An alternative names a part that is not in the base.
pub const ALTERNATIVE_UNKNOWN: &str = "PHOENIX-PARTS-013";
/// A part of the design has no match in the base.
pub const NO_MATCH: &str = "PHOENIX-PARTS-014";
/// A part of the design matches several parts of the base.
pub const SEVERAL_MATCHES: &str = "PHOENIX-PARTS-015";
/// A part of the design is obsolete or not recommended in the base.
pub const OBSOLETE_PART: &str = "PHOENIX-PARTS-016";
/// A part asked for is not in the base.
pub const PART_NOT_FOUND: &str = "PHOENIX-PARTS-017";
/// A part of the base cannot be assigned to a symbol.
pub const ASSIGN_REFUSED: &str = "PHOENIX-PARTS-018";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Parts settings are not valid",
            "La configuración de piezas no es válida",
            "Die Einstellungen der Bauteile sind ungültig",
            "Les réglages des pièces ne sont pas valides",
            "Le impostazioni dei componenti non sono valide",
            "部品の設定が無効です",
            "元件设置无效",
            "As definições das peças não são válidas",
        ],
    ),
    (
        FILE_UNUSABLE,
        [
            "File of the parts base cannot be read",
            "No se puede leer el archivo de la base de piezas",
            "Die Datei der Bauteildatenbank kann nicht gelesen werden",
            "Le fichier de la base de pièces ne peut pas être lu",
            "Il file della base dei componenti non può essere letto",
            "部品データベースのファイルを読み込めません",
            "无法读取元件库文件",
            "Não é possível ler o ficheiro da base de peças",
        ],
    ),
    (
        FORMAT_INVALID,
        [
            "File of the parts base is not valid",
            "El archivo de la base de piezas no es válido",
            "Die Datei der Bauteildatenbank ist ungültig",
            "Le fichier de la base de pièces n'est pas valide",
            "Il file della base dei componenti non è valido",
            "部品データベースのファイルが無効です",
            "元件库文件无效",
            "O ficheiro da base de peças não é válido",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the parts base was exceeded",
            "Se superó un límite de la base de piezas",
            "Eine Grenze der Bauteildatenbank wurde überschritten",
            "Une limite de la base de pièces a été dépassée",
            "È stato superato un limite della base dei componenti",
            "部品データベースの上限を超えました",
            "超出了元件库的限制",
            "Foi excedido um limite da base de peças",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "La opción del comando no es válida",
            "Die Option des Befehls ist ungültig",
            "L'option de la commande n'est pas valide",
            "L'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "A opção do comando não é válida",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Output file exists or cannot be written",
            "El archivo de salida ya existe o no se puede escribir",
            "Die Ausgabedatei existiert bereits oder kann nicht geschrieben werden",
            "Le fichier de sortie existe déjà ou ne peut pas être écrit",
            "Il file di output esiste già o non può essere scritto",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        ID_DUPLICATE,
        [
            "Part id used twice",
            "Identificador de pieza usado dos veces",
            "Bauteil-ID doppelt verwendet",
            "Identifiant de pièce utilisé deux fois",
            "ID del componente usato due volte",
            "部品IDが重複しています",
            "元件编号重复",
            "Identificador de peça usado duas vezes",
        ],
    ),
    (
        FOOTPRINT_UNKNOWN,
        [
            "Footprint is not in the IPC or own library",
            "La huella no está en la biblioteca IPC ni en la propia",
            "Der Footprint ist weder in der IPC- noch in der eigenen Bibliothek",
            "L'empreinte n'est ni dans la bibliothèque IPC ni dans la bibliothèque propre",
            "L'impronta non è né nella libreria IPC né in quella propria",
            "フットプリントが IPC ライブラリにも独自ライブラリにもありません",
            "封装不在 IPC 库或自有库中",
            "A pegada não existe na biblioteca IPC nem na própria",
        ],
    ),
    (
        MPN_CONFLICT,
        [
            "Same part number with different data",
            "El mismo número de pieza con datos distintos",
            "Gleiche Teilenummer mit unterschiedlichen Daten",
            "Même référence de fabricant avec des données différentes",
            "Stesso codice produttore con dati diversi",
            "同じ型番で異なるデータがあります",
            "同一型号的数据不一致",
            "O mesmo número de peça com dados diferentes",
        ],
    ),
    (
        PRICE_NO_CURRENCY,
        [
            "Price has no currency",
            "El precio no tiene moneda",
            "Der Preis hat keine Währung",
            "Le prix n'a pas de devise",
            "Il prezzo non ha una valuta",
            "価格に通貨がありません",
            "价格没有币种",
            "O preço não tem moeda",
        ],
    ),
    (
        TIERS_UNORDERED,
        [
            "Price breaks are not in ascending order",
            "Los tramos de precio no están en orden ascendente",
            "Die Preisstaffeln sind nicht aufsteigend geordnet",
            "Les paliers de prix ne sont pas dans l'ordre croissant",
            "Gli scaglioni di prezzo non sono in ordine crescente",
            "価格ブレークが昇順ではありません",
            "价格阶梯未按升序排列",
            "Os escalões de preço não estão por ordem crescente",
        ],
    ),
    (
        SIM_MODEL_UNKNOWN,
        [
            "Simulation model is not in the table of models",
            "El modelo de simulación no está en la tabla de modelos",
            "Das Simulationsmodell ist nicht in der Modelltabelle",
            "Le modèle de simulation n'est pas dans la table des modèles",
            "Il modello di simulazione non è nella tabella dei modelli",
            "シミュレーションモデルがモデル表にありません",
            "仿真模型不在模型表中",
            "O modelo de simulação não está na tabela de modelos",
        ],
    ),
    (
        ALTERNATIVE_UNKNOWN,
        [
            "Alternative names a part that is not in the base",
            "La alternativa nombra una pieza que no está en la base",
            "Die Alternative nennt ein Bauteil, das nicht in der Datenbank ist",
            "L'alternative nomme une pièce absente de la base",
            "L'alternativa indica un componente che non è nella base",
            "代替品に挙げた部品がデータベースにありません",
            "替代件所指的元件不在库中",
            "A alternativa nomeia uma peça que não está na base",
        ],
    ),
    (
        NO_MATCH,
        [
            "Part has no match in the parts base",
            "La pieza no tiene coincidencia en la base de piezas",
            "Das Bauteil hat keine Entsprechung in der Datenbank",
            "La pièce n'a pas de correspondance dans la base",
            "Il componente non ha corrispondenza nella base",
            "部品がデータベースに一致しません",
            "元件在库中没有匹配项",
            "A peça não tem correspondência na base de peças",
        ],
    ),
    (
        SEVERAL_MATCHES,
        [
            "Part matches several parts of the base",
            "La pieza coincide con varias piezas de la base",
            "Das Bauteil passt zu mehreren Bauteilen der Datenbank",
            "La pièce correspond à plusieurs pièces de la base",
            "Il componente corrisponde a più componenti della base",
            "部品がデータベースの複数の部品に一致します",
            "元件匹配库中的多个元件",
            "A peça corresponde a várias peças da base",
        ],
    ),
    (
        OBSOLETE_PART,
        [
            "Part is obsolete or not recommended",
            "La pieza está obsoleta o no recomendada",
            "Das Bauteil ist veraltet oder nicht empfohlen",
            "La pièce est obsolète ou déconseillée",
            "Il componente è obsoleto o sconsigliato",
            "部品は製造終了または非推奨です",
            "元件已停产或不推荐使用",
            "A peça está obsoleta ou não é recomendada",
        ],
    ),
    (
        PART_NOT_FOUND,
        [
            "Part is not in the parts base",
            "La pieza no está en la base de piezas",
            "Das Bauteil ist nicht in der Datenbank",
            "La pièce n'est pas dans la base de pièces",
            "Il componente non è nella base dei componenti",
            "部品がデータベースにありません",
            "元件不在库中",
            "A peça não está na base de peças",
        ],
    ),
    (
        ASSIGN_REFUSED,
        [
            "The part cannot be assigned to the symbol",
            "No se puede asignar la pieza al símbolo",
            "Das Bauteil kann dem Symbol nicht zugewiesen werden",
            "La pièce ne peut pas être affectée au symbole",
            "Il componente non può essere assegnato al simbolo",
            "部品をシンボルに割り当てられません",
            "无法将元件分配给符号",
            "Não é possível atribuir a peça ao símbolo",
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something to know.
    Info,
    /// Something to look at.
    Warning,
    /// Something that makes the base wrong.
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

/// A finding of a check or of a link: a registered code, what it is about and what is specific.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// The code, `PHOENIX-PARTS-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The part (its id) or the references the finding is about.
    pub subject: String,
    /// Where the part is in the files (`file: line N`), empty when there is no place.
    pub origin: String,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a subject.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: &str,
        origin: &str,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            subject: subject.to_owned(),
            origin: origin.to_owned(),
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title, the subject, the place and the
    /// detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if !self.subject.is_empty() {
            out.push_str(&format!(" [{}]", self.subject));
        }
        if !self.origin.is_empty() {
            out.push_str(&format!(" ({})", self.origin));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }

    /// The finding as JSON, with the English title.
    pub fn to_json(&self) -> Value {
        json!({
            "code": self.code,
            "severity": self.severity.as_str(),
            "title": title(self.code, Lang::En),
            "subject": self.subject,
            "origin": self.origin,
            "detail": self.detail,
        })
    }
}

/// An error of the parts base: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartsError {
    /// The code, `PHOENIX-PARTS-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl PartsError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> PartsError {
        PartsError {
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

impl fmt::Display for PartsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for PartsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        assert_eq!(TITLES.len(), 18);
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-PARTS-{:03}", i + 1));
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
        let e = PartsError::new(PART_NOT_FOUND, "r-10k");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-PARTS-017: A peça não está na base de peças: r-10k"
        );
        assert_eq!(e.to_string(), "PHOENIX-PARTS-017: r-10k");
        let bare = PartsError::new(NO_MATCH, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-PARTS-014: Part has no match in the parts base"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_place_and_detail() {
        let f = Finding::new(
            FOOTPRINT_UNKNOWN,
            Severity::Warning,
            "r-10k",
            "base.json: part 1",
            "R0603X",
        );
        assert_eq!(
            f.line(Lang::En),
            "warning: PHOENIX-PARTS-008: Footprint is not in the IPC or own library [r-10k] (base.json: part 1): R0603X"
        );
        assert_eq!(f.to_json()["severity"], "warning");
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
