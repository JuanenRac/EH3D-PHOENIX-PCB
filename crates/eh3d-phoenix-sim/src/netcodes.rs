// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the netlist made from a schematic, `PHOENIX-NETLIST-<NNN>`, with their
//! titles in the eight languages of the suite, and the issue that carries one.

use eh3d_phoenix_rules::{Lang, Severity};

/// The table of simulation models cannot be read.
pub const TABLE_INVALID: &str = "PHOENIX-NETLIST-001";
/// A part that has no simulation model.
pub const NO_MODEL: &str = "PHOENIX-NETLIST-002";
/// A pin of a part that is on no net.
pub const PIN_ON_NO_NET: &str = "PHOENIX-NETLIST-003";
/// A net with a single connection.
pub const NET_FLOATING: &str = "PHOENIX-NETLIST-004";
/// A source whose two terminals are on one node, or voltage sources in parallel.
pub const SOURCE_SHORTED: &str = "PHOENIX-NETLIST-005";
/// A value or an attribute that cannot be read.
pub const VALUE_BAD: &str = "PHOENIX-NETLIST-006";
/// The pins of the symbol do not fit the device of the model.
pub const PINS_MISMATCH: &str = "PHOENIX-NETLIST-007";
/// A reference designator used by more than one part.
pub const REFDES_TWICE: &str = "PHOENIX-NETLIST-008";
/// No translated element is joined to a ground net.
pub const NO_GROUND_NET: &str = "PHOENIX-NETLIST-009";
/// The electrical rule check found errors before the netlist was made.
pub const ERC_ERRORS: &str = "PHOENIX-NETLIST-010";
/// A model that the table or a part names and nothing defines.
pub const MODEL_UNDEFINED: &str = "PHOENIX-NETLIST-011";
/// An output file that may not be written.
pub const OUTPUT_REFUSED: &str = "PHOENIX-NETLIST-012";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        TABLE_INVALID,
        [
            "Table of simulation models is not valid",
            "La tabla de modelos de simulación no es válida",
            "Die Tabelle der Simulationsmodelle ist ungültig",
            "La table des modèles de simulation n'est pas valide",
            "La tabella dei modelli di simulazione non è valida",
            "シミュレーションモデルの表が無効です",
            "仿真模型表无效",
            "A tabela de modelos de simulação não é válida",
        ],
    ),
    (
        NO_MODEL,
        [
            "Part has no simulation model",
            "La pieza no tiene modelo de simulación",
            "Das Bauteil hat kein Simulationsmodell",
            "La pièce n'a pas de modèle de simulation",
            "Il componente non ha un modello di simulazione",
            "部品にシミュレーションモデルがありません",
            "元件没有仿真模型",
            "A peça não tem modelo de simulação",
        ],
    ),
    (
        PIN_ON_NO_NET,
        [
            "Pin is on no net",
            "Pin que no está en ninguna red",
            "Pin liegt an keinem Netz",
            "Broche sur aucun réseau",
            "Pin su nessuna rete",
            "ピンがどのネットにも接続されていません",
            "引脚不在任何网络上",
            "Pino que não está em nenhuma rede",
        ],
    ),
    (
        NET_FLOATING,
        [
            "Net has a single connection",
            "Red con una sola conexión",
            "Netz mit nur einem Anschluss",
            "Réseau avec une seule connexion",
            "Rete con una sola connessione",
            "ネットの接続が1つだけです",
            "网络只有一个连接",
            "Rede com uma só ligação",
        ],
    ),
    (
        SOURCE_SHORTED,
        [
            "Source is short-circuited",
            "Fuente en cortocircuito",
            "Quelle ist kurzgeschlossen",
            "Source en court-circuit",
            "Sorgente in cortocircuito",
            "電源が短絡しています",
            "电源被短路",
            "Fonte em curto-circuito",
        ],
    ),
    (
        VALUE_BAD,
        [
            "Value or attribute is not valid",
            "Valor o atributo no válido",
            "Wert oder Attribut ist ungültig",
            "Valeur ou attribut non valide",
            "Valore o attributo non valido",
            "値または属性が無効です",
            "数值或属性无效",
            "Valor ou atributo inválido",
        ],
    ),
    (
        PINS_MISMATCH,
        [
            "Pins do not fit the device",
            "Los pines no encajan con el dispositivo",
            "Die Pins passen nicht zum Bauelement",
            "Les broches ne correspondent pas au composant",
            "I pin non corrispondono al dispositivo",
            "ピンがデバイスに合いません",
            "引脚与器件不匹配",
            "Os pinos não se ajustam ao dispositivo",
        ],
    ),
    (
        REFDES_TWICE,
        [
            "Reference designator is used twice",
            "Designador de referencia repetido",
            "Bauteilbezeichner doppelt verwendet",
            "Référence utilisée deux fois",
            "Riferimento usato due volte",
            "リファレンスが重複しています",
            "位号重复使用",
            "Designador de referência repetido",
        ],
    ),
    (
        NO_GROUND_NET,
        [
            "Circuit has no ground net",
            "El circuito no tiene red de masa",
            "Die Schaltung hat kein Massenetz",
            "Le circuit n'a pas de réseau de masse",
            "Il circuito non ha una rete di massa",
            "回路にグランドのネットがありません",
            "电路没有接地网络",
            "O circuito não tem rede de massa",
        ],
    ),
    (
        ERC_ERRORS,
        [
            "Electrical rule check found errors",
            "La comprobación eléctrica encontró errores",
            "Die elektrische Regelprüfung fand Fehler",
            "Le contrôle électrique a trouvé des erreurs",
            "Il controllo elettrico ha trovato errori",
            "電気的ルールチェックでエラーが見つかりました",
            "电气规则检查发现错误",
            "A verificação elétrica encontrou erros",
        ],
    ),
    (
        MODEL_UNDEFINED,
        [
            "Model is not defined",
            "Modelo no definido",
            "Modell ist nicht definiert",
            "Modèle non défini",
            "Modello non definito",
            "モデルが定義されていません",
            "模型未定义",
            "Modelo não definido",
        ],
    ),
    (
        OUTPUT_REFUSED,
        [
            "Output file is not allowed",
            "Fichero de salida no permitido",
            "Ausgabedatei nicht erlaubt",
            "Fichier de sortie non autorisé",
            "File di uscita non consentito",
            "出力ファイルは許可されません",
            "不允许的输出文件",
            "Ficheiro de saída não permitido",
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

/// One thing the translation of a schematic found: a code, how serious it is and what it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenIssue {
    /// The code, `PHOENIX-NETLIST-<NNN>`.
    pub code: &'static str,
    /// An error stops a simulation; a warning or information does not.
    pub severity: Severity,
    /// What is specific to the case, in English: the part, the pin, the net.
    pub detail: String,
}

impl GenIssue {
    /// An issue with a code, a severity and its detail.
    pub fn new(code: &'static str, severity: Severity, detail: impl Into<String>) -> GenIssue {
        GenIssue {
            code,
            severity,
            detail: detail.into(),
        }
    }

    /// The message in a language: the severity, the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!(
            "[{}] {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-NETLIST-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_message_names_the_severity_the_code_the_title_and_the_detail() {
        let issue = GenIssue::new(NO_MODEL, Severity::Warning, "U1");
        assert_eq!(
            issue.message(Lang::Pt),
            "[warning] PHOENIX-NETLIST-002: A peça não tem modelo de simulação: U1"
        );
        let bare = GenIssue::new(NO_GROUND_NET, Severity::Error, "");
        assert_eq!(
            bare.message(Lang::En),
            "[error] PHOENIX-NETLIST-009: Circuit has no ground net"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
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
