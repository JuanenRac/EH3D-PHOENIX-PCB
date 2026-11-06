// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the RF structure generator, `PHOENIX-RFGEN-<NNN>`, with their titles in
//! the eight languages of the suite, and the error type that carries one.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-RFGEN-001";
/// A parameter is missing, unknown or out of range.
pub const PARAMETER_INVALID: &str = "PHOENIX-RFGEN-002";
/// The type of structure is not known.
pub const KIND_UNKNOWN: &str = "PHOENIX-RFGEN-003";
/// The stack-up cannot carry a microstrip on the layer.
pub const SUBSTRATE_UNUSABLE: &str = "PHOENIX-RFGEN-004";
/// A track width could not be found for an impedance.
pub const SYNTHESIS_FAILED: &str = "PHOENIX-RFGEN-005";
/// The design cannot be built in microstrip.
pub const NOT_REALISABLE: &str = "PHOENIX-RFGEN-006";
/// The search for the width and gap of a coupled pair did not converge.
pub const SOLVER_FAILED: &str = "PHOENIX-RFGEN-007";
/// The board has no copper layer of that name.
pub const LAYER_NOT_FOUND: &str = "PHOENIX-RFGEN-008";
/// A coordinate or a length is out of range.
pub const COORDINATE_RANGE: &str = "PHOENIX-RFGEN-009";
/// The board refused an operation.
pub const BOARD_REFUSED: &str = "PHOENIX-RFGEN-010";
/// The rules find a new error in the result.
pub const RULES_REFUSE: &str = "PHOENIX-RFGEN-011";
/// No free id could be made.
pub const ID_UNAVAILABLE: &str = "PHOENIX-RFGEN-012";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-RFGEN-013";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-RFGEN-014";
/// The stack-up or the board cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-RFGEN-015";
/// A limit was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-RFGEN-016";
/// Warning: a value is outside the range of a formula.
pub const WARN_RANGE: &str = "PHOENIX-RFGEN-017";
/// Warning: a shape is smaller than the minimum.
pub const WARN_FEATURE: &str = "PHOENIX-RFGEN-018";
/// Warning: the result rests on an approximation.
pub const WARN_APPROXIMATE: &str = "PHOENIX-RFGEN-019";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "RF generator settings are not valid",
            "La configuración del generador de RF no es válida",
            "Die Einstellungen des HF-Generators sind ungültig",
            "Les réglages du générateur RF ne sont pas valides",
            "Le impostazioni del generatore RF non sono valide",
            "RF ジェネレーターの設定が無効です",
            "射频生成器的设置无效",
            "As definições do gerador de RF não são válidas",
        ],
    ),
    (
        PARAMETER_INVALID,
        [
            "A parameter is missing, unknown or out of range",
            "Falta un parámetro, es desconocido o está fuera de rango",
            "Ein Parameter fehlt, ist unbekannt oder liegt außerhalb des Bereichs",
            "Un paramètre est manquant, inconnu ou hors plage",
            "Un parametro manca, è sconosciuto o fuori intervallo",
            "パラメーターが不足、未知、または範囲外です",
            "参数缺失、未知或超出范围",
            "Falta um parâmetro, é desconhecido ou está fora do intervalo",
        ],
    ),
    (
        KIND_UNKNOWN,
        [
            "Unknown type of structure",
            "Tipo de estructura desconocido",
            "Unbekannter Strukturtyp",
            "Type de structure inconnu",
            "Tipo di struttura sconosciuto",
            "未知の構造タイプです",
            "未知的结构类型",
            "Tipo de estrutura desconhecido",
        ],
    ),
    (
        SUBSTRATE_UNUSABLE,
        [
            "The stack-up cannot carry a microstrip on this layer",
            "El apilado no admite una microtira en esta capa",
            "Der Lagenaufbau kann auf dieser Lage keine Mikrostreifenleitung tragen",
            "L'empilement ne permet pas une microruban sur cette couche",
            "Lo stackup non può ospitare una microstriscia su questo strato",
            "このスタックアップではこの層にマイクロストリップを置けません",
            "该叠层无法在此层承载微带线",
            "O empilhamento não admite uma microfita nesta camada",
        ],
    ),
    (
        SYNTHESIS_FAILED,
        [
            "A track width could not be found for the impedance",
            "No se pudo hallar un ancho de pista para la impedancia",
            "Für die Impedanz konnte keine Leiterbahnbreite gefunden werden",
            "Aucune largeur de piste n'a pu être trouvée pour l'impédance",
            "Non è stato possibile trovare una larghezza di pista per l'impedenza",
            "このインピーダンスに対する配線幅を求められませんでした",
            "无法为该阻抗求得走线宽度",
            "Não foi possível encontrar uma largura de pista para a impedância",
        ],
    ),
    (
        NOT_REALISABLE,
        [
            "The design cannot be built in microstrip",
            "El diseño no se puede construir en microtira",
            "Der Entwurf lässt sich nicht in Mikrostreifentechnik aufbauen",
            "La conception ne peut pas être réalisée en microruban",
            "Il progetto non può essere realizzato in microstriscia",
            "この設計はマイクロストリップでは作れません",
            "该设计无法用微带线实现",
            "O projeto não pode ser construído em microfita",
        ],
    ),
    (
        SOLVER_FAILED,
        [
            "The search for the width and gap of a coupled pair did not converge",
            "La búsqueda del ancho y la separación de un par acoplado no convergió",
            "Die Suche nach Breite und Abstand eines gekoppelten Paars hat nicht konvergiert",
            "La recherche de la largeur et de l'espacement d'une paire couplée n'a pas convergé",
            "La ricerca di larghezza e distanza di una coppia accoppiata non è convergita",
            "結合線路対の幅と間隔の探索が収束しませんでした",
            "耦合线对的宽度和间距搜索未收敛",
            "A procura da largura e do afastamento de um par acoplado não convergiu",
        ],
    ),
    (
        LAYER_NOT_FOUND,
        [
            "The board has no copper layer of that name",
            "La placa no tiene una capa de cobre con ese nombre",
            "Die Platine hat keine Kupferlage mit diesem Namen",
            "La carte n'a pas de couche de cuivre de ce nom",
            "La scheda non ha uno strato di rame con quel nome",
            "基板にその名前の銅層がありません",
            "电路板没有该名称的铜层",
            "A placa não tem uma camada de cobre com esse nome",
        ],
    ),
    (
        COORDINATE_RANGE,
        [
            "A coordinate or a length is out of range",
            "Una coordenada o una longitud está fuera de rango",
            "Eine Koordinate oder eine Länge liegt außerhalb des Bereichs",
            "Une coordonnée ou une longueur est hors plage",
            "Una coordinata o una lunghezza è fuori intervallo",
            "座標または長さが範囲外です",
            "坐标或长度超出范围",
            "Uma coordenada ou um comprimento está fora do intervalo",
        ],
    ),
    (
        BOARD_REFUSED,
        [
            "The board refused an operation",
            "La placa rechazó una operación",
            "Die Platine hat eine Operation abgelehnt",
            "La carte a refusé une opération",
            "La scheda ha rifiutato un'operazione",
            "基板が操作を拒否しました",
            "电路板拒绝了一项操作",
            "A placa recusou uma operação",
        ],
    ),
    (
        RULES_REFUSE,
        [
            "The rules find a new error in the result",
            "Las reglas encuentran un error nuevo en el resultado",
            "Die Regeln finden einen neuen Fehler im Ergebnis",
            "Les règles trouvent une nouvelle erreur dans le résultat",
            "Le regole trovano un nuovo errore nel risultato",
            "ルールチェックが結果に新しいエラーを検出しました",
            "规则检查在结果中发现了新的错误",
            "As regras encontram um novo erro no resultado",
        ],
    ),
    (
        ID_UNAVAILABLE,
        [
            "No free id could be made",
            "No se pudo crear un identificador libre",
            "Es konnte keine freie Kennung erzeugt werden",
            "Aucun identifiant libre n'a pu être créé",
            "Impossibile creare un identificatore libero",
            "空いている ID を作れませんでした",
            "无法生成空闲的标识符",
            "Não foi possível criar um identificador livre",
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
            "El archivo de salida ya existe o no se puede escribir",
            "Die Ausgabedatei existiert bereits oder kann nicht geschrieben werden",
            "Le fichier de sortie existe déjà ou ne peut pas être écrit",
            "Il file di uscita esiste già o non può essere scritto",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "The stack-up or the board cannot be read",
            "No se puede leer el apilado o la placa",
            "Lagenaufbau oder Platine kann nicht gelesen werden",
            "L'empilement ou la carte est illisible",
            "Lo stackup o la scheda non può essere letto",
            "スタックアップまたは基板を読み込めません",
            "无法读取叠层或电路板",
            "O empilhamento ou a placa não pode ser lido",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit was exceeded",
            "Se superó un límite",
            "Ein Limit wurde überschritten",
            "Une limite a été dépassée",
            "Un limite è stato superato",
            "上限を超えました",
            "超出了限制",
            "Foi excedido um limite",
        ],
    ),
    (
        WARN_RANGE,
        [
            "A value is outside the range of a formula",
            "Un valor está fuera del rango de una fórmula",
            "Ein Wert liegt außerhalb des Gültigkeitsbereichs einer Formel",
            "Une valeur est hors du domaine d'une formule",
            "Un valore è fuori dall'intervallo di una formula",
            "値が式の適用範囲外です",
            "某个数值超出了公式的适用范围",
            "Um valor está fora do intervalo de uma fórmula",
        ],
    ),
    (
        WARN_FEATURE,
        [
            "A shape is smaller than the minimum",
            "Una forma es menor que el mínimo",
            "Eine Form ist kleiner als das Minimum",
            "Une forme est plus petite que le minimum",
            "Una forma è più piccola del minimo",
            "形状が最小値より小さくなっています",
            "某个图形小于最小值",
            "Uma forma é menor do que o mínimo",
        ],
    ),
    (
        WARN_APPROXIMATE,
        [
            "The result rests on an approximation",
            "El resultado se basa en una aproximación",
            "Das Ergebnis beruht auf einer Näherung",
            "Le résultat repose sur une approximation",
            "Il risultato si basa su un'approssimazione",
            "結果は近似に基づいています",
            "结果基于近似",
            "O resultado assenta numa aproximação",
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

/// An error of the generator: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfgenError {
    /// The code, `PHOENIX-RFGEN-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl RfgenError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> RfgenError {
        RfgenError {
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

impl fmt::Display for RfgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for RfgenError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-RFGEN-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
        assert_eq!(TITLES.len(), 19);
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
        let e = RfgenError::new(NOT_REALISABLE, "the series reactance is negative");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-RFGEN-006: O projeto não pode ser construído em microfita: the series reactance is negative"
        );
        assert_eq!(
            e.to_string(),
            "PHOENIX-RFGEN-006: the series reactance is negative"
        );
        let bare = RfgenError::new(ID_UNAVAILABLE, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-RFGEN-012: No free id could be made"
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
