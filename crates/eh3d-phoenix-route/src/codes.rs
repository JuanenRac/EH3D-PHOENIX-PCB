// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the editing and routing logic, `PHOENIX-ROUTE-<NNN>`, with their titles
//! in the eight languages of the suite, and the error type that carries one.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The route settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-ROUTE-001";
/// A limit of the settings was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-ROUTE-002";
/// An element to connect was not found on the board.
pub const ELEMENT_NOT_FOUND: &str = "PHOENIX-ROUTE-003";
/// The text that names an element is not valid.
pub const ELEMENT_TEXT_INVALID: &str = "PHOENIX-ROUTE-004";
/// The two elements are not on the same net, or one has no net.
pub const NET_MISMATCH: &str = "PHOENIX-ROUTE-005";
/// A layer cannot be used.
pub const LAYER_UNUSABLE: &str = "PHOENIX-ROUTE-006";
/// No path was found.
pub const NO_PATH: &str = "PHOENIX-ROUTE-007";
/// The route stopped before an obstacle.
pub const STOPPED: &str = "PHOENIX-ROUTE-008";
/// The rules refuse the track.
pub const RULES_REFUSE: &str = "PHOENIX-ROUTE-009";
/// The route does not join the two elements.
pub const NOT_CONNECTED: &str = "PHOENIX-ROUTE-010";
/// Both elements are at the same place.
pub const SAME_PLACE: &str = "PHOENIX-ROUTE-011";
/// No free id could be made.
pub const ID_UNAVAILABLE: &str = "PHOENIX-ROUTE-012";
/// The board refused an operation.
pub const OPERATION_FAILED: &str = "PHOENIX-ROUTE-013";
/// An edit would add a rule error.
pub const EDIT_REJECTED: &str = "PHOENIX-ROUTE-014";
/// An edit refers to something that is not there.
pub const EDIT_INVALID: &str = "PHOENIX-ROUTE-015";
/// An edit would break the angles of a track.
pub const ANGLE_BROKEN: &str = "PHOENIX-ROUTE-016";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-ROUTE-017";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-ROUTE-018";
/// A coordinate is out of range.
pub const OUT_OF_RANGE: &str = "PHOENIX-ROUTE-019";
/// The board or the rule file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-ROUTE-020";
/// The board cannot be written as a Specctra DSN file.
pub const DSN_EXPORT_FAILED: &str = "PHOENIX-ROUTE-021";
/// The session text is not a valid Specctra session.
pub const SES_UNREADABLE: &str = "PHOENIX-ROUTE-022";
/// The session names a net, layer or via that the board does not have.
pub const SES_MISMATCH: &str = "PHOENIX-ROUTE-023";
/// The settings of the external autorouter are not valid.
pub const AUTOROUTER_CONFIG_INVALID: &str = "PHOENIX-ROUTE-024";
/// The external autorouter is switched off.
pub const AUTOROUTER_DISABLED: &str = "PHOENIX-ROUTE-025";
/// The external autorouter could not be run.
pub const AUTOROUTER_LAUNCH_FAILED: &str = "PHOENIX-ROUTE-026";
/// The external autorouter ran out of time.
pub const AUTOROUTER_TIMEOUT: &str = "PHOENIX-ROUTE-027";
/// The external autorouter left no session.
pub const AUTOROUTER_NO_OUTPUT: &str = "PHOENIX-ROUTE-028";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Route settings are not valid",
            "La configuración del enrutado no es válida",
            "Die Einstellungen des Routings sind ungültig",
            "Les réglages du routage ne sont pas valides",
            "Le impostazioni dell'instradamento non sono valide",
            "ルーティング設定が無効です",
            "布线设置无效",
            "As definições do encaminhamento não são válidas",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the route settings was exceeded",
            "Se superó un límite de la configuración del enrutado",
            "Ein Limit der Einstellungen wurde überschritten",
            "Une limite des réglages a été dépassée",
            "Un limite delle impostazioni è stato superato",
            "設定の上限を超えました",
            "超出了设置的限制",
            "Foi excedido um limite das definições",
        ],
    ),
    (
        ELEMENT_NOT_FOUND,
        [
            "Element to connect was not found",
            "No se encontró el elemento que conectar",
            "Das zu verbindende Element wurde nicht gefunden",
            "L'élément à relier est introuvable",
            "L'elemento da collegare non è stato trovato",
            "接続する要素が見つかりません",
            "未找到要连接的元件",
            "O elemento a ligar não foi encontrado",
        ],
    ),
    (
        ELEMENT_TEXT_INVALID,
        [
            "Element name is not valid",
            "El nombre del elemento no es válido",
            "Der Name des Elements ist ungültig",
            "Le nom de l'élément n'est pas valide",
            "Il nome dell'elemento non è valido",
            "要素名が無効です",
            "元件名称无效",
            "O nome do elemento não é válido",
        ],
    ),
    (
        NET_MISMATCH,
        [
            "Elements are not on the same net",
            "Los elementos no están en la misma red",
            "Die Elemente liegen nicht im selben Netz",
            "Les éléments ne sont pas sur le même réseau",
            "Gli elementi non sono sulla stessa rete",
            "要素が同じネットにありません",
            "元件不在同一网络上",
            "Os elementos não estão na mesma rede",
        ],
    ),
    (
        LAYER_UNUSABLE,
        [
            "Layer cannot be used",
            "No se puede usar la capa",
            "Die Lage kann nicht verwendet werden",
            "La couche ne peut pas être utilisée",
            "Lo strato non può essere usato",
            "このレイヤーは使用できません",
            "无法使用该层",
            "A camada não pode ser usada",
        ],
    ),
    (
        NO_PATH,
        [
            "No path was found",
            "No se encontró ningún camino",
            "Es wurde kein Weg gefunden",
            "Aucun chemin n'a été trouvé",
            "Nessun percorso trovato",
            "経路が見つかりません",
            "未找到路径",
            "Não foi encontrado nenhum caminho",
        ],
    ),
    (
        STOPPED,
        [
            "Route stopped before an obstacle",
            "El trazado se detuvo ante un obstáculo",
            "Die Leiterbahn endet vor einem Hindernis",
            "Le tracé s'arrête devant un obstacle",
            "Il tracciato si ferma davanti a un ostacolo",
            "障害物の手前で配線を停止しました",
            "布线在障碍物前停止",
            "O traçado parou antes de um obstáculo",
        ],
    ),
    (
        RULES_REFUSE,
        [
            "The rules refuse the track",
            "Las reglas rechazan la pista",
            "Die Regeln lehnen die Leiterbahn ab",
            "Les règles refusent la piste",
            "Le regole rifiutano la pista",
            "ルールによりトラックは拒否されました",
            "规则拒绝该走线",
            "As regras recusam a pista",
        ],
    ),
    (
        NOT_CONNECTED,
        [
            "The route does not join the two elements",
            "El trazado no une los dos elementos",
            "Die Verbindung verbindet die beiden Elemente nicht",
            "Le tracé ne relie pas les deux éléments",
            "Il percorso non collega i due elementi",
            "配線が2つの要素を接続していません",
            "走线未连接这两个元件",
            "O percurso não liga os dois elementos",
        ],
    ),
    (
        SAME_PLACE,
        [
            "Both elements are at the same place",
            "Los dos elementos están en el mismo lugar",
            "Beide Elemente liegen am selben Ort",
            "Les deux éléments sont au même endroit",
            "I due elementi sono nello stesso punto",
            "2つの要素が同じ位置にあります",
            "两个元件位于同一位置",
            "Os dois elementos estão no mesmo local",
        ],
    ),
    (
        ID_UNAVAILABLE,
        [
            "No free id could be made",
            "No se pudo crear un identificador libre",
            "Es konnte keine freie Kennung erzeugt werden",
            "Impossible de créer un identifiant libre",
            "Non è stato possibile creare un identificatore libero",
            "空きIDを作成できませんでした",
            "无法生成可用的标识符",
            "Não foi possível criar um identificador livre",
        ],
    ),
    (
        OPERATION_FAILED,
        [
            "The board refused the operation",
            "La placa rechazó la operación",
            "Die Platine hat die Operation abgelehnt",
            "La carte a refusé l'opération",
            "La scheda ha rifiutato l'operazione",
            "基板が操作を拒否しました",
            "电路板拒绝了该操作",
            "A placa recusou a operação",
        ],
    ),
    (
        EDIT_REJECTED,
        [
            "The edit would add a rule error",
            "La edición añadiría un error de reglas",
            "Die Änderung würde einen Regelverstoß hinzufügen",
            "La modification ajouterait une erreur de règle",
            "La modifica aggiungerebbe un errore di regole",
            "この編集はルール違反を追加します",
            "此编辑会新增规则错误",
            "A edição acrescentaria um erro de regras",
        ],
    ),
    (
        EDIT_INVALID,
        [
            "The edit refers to something that is not there",
            "La edición se refiere a algo que no existe",
            "Die Änderung bezieht sich auf etwas, das nicht vorhanden ist",
            "La modification porte sur quelque chose d'absent",
            "La modifica si riferisce a qualcosa che non esiste",
            "編集が存在しないものを参照しています",
            "编辑引用了不存在的内容",
            "A edição refere-se a algo que não existe",
        ],
    ),
    (
        ANGLE_BROKEN,
        [
            "The edit would break the 45 degree angles",
            "La edición rompería los ángulos de 45 grados",
            "Die Änderung würde die 45-Grad-Winkel verletzen",
            "La modification romprait les angles de 45 degrés",
            "La modifica romperebbe gli angoli di 45 gradi",
            "この編集は45度の角度を崩します",
            "此编辑会破坏45度角",
            "A edição quebraria os ângulos de 45 graus",
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
            "Il file di uscita esiste già o non può essere scritto",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        OUT_OF_RANGE,
        [
            "A coordinate is out of range",
            "Una coordenada está fuera de rango",
            "Eine Koordinate liegt außerhalb des Bereichs",
            "Une coordonnée est hors limites",
            "Una coordinata è fuori intervallo",
            "座標が範囲外です",
            "坐标超出范围",
            "Uma coordenada está fora do intervalo",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Board or rule file cannot be read",
            "No se puede leer la placa o el archivo de reglas",
            "Platine oder Regeldatei kann nicht gelesen werden",
            "La carte ou le fichier de règles est illisible",
            "La scheda o il file delle regole non può essere letto",
            "基板またはルールファイルを読み込めません",
            "无法读取电路板或规则文件",
            "A placa ou o ficheiro de regras não pode ser lido",
        ],
    ),
    (
        DSN_EXPORT_FAILED,
        [
            "The board cannot be exported to Specctra DSN",
            "La placa no se puede exportar a Specctra DSN",
            "Die Platine kann nicht nach Specctra DSN exportiert werden",
            "La carte ne peut pas être exportée en Specctra DSN",
            "La scheda non può essere esportata in Specctra DSN",
            "基板をSpecctra DSNに書き出せません",
            "无法将电路板导出为Specctra DSN",
            "A placa não pode ser exportada para Specctra DSN",
        ],
    ),
    (
        SES_UNREADABLE,
        [
            "The Specctra session cannot be read",
            "No se puede leer la sesión de Specctra",
            "Die Specctra-Sitzung kann nicht gelesen werden",
            "La session Specctra est illisible",
            "La sessione Specctra non può essere letta",
            "Specctraセッションを読み込めません",
            "无法读取Specctra会话",
            "A sessão Specctra não pode ser lida",
        ],
    ),
    (
        SES_MISMATCH,
        [
            "The session does not match the board",
            "La sesión no corresponde a la placa",
            "Die Sitzung passt nicht zur Platine",
            "La session ne correspond pas à la carte",
            "La sessione non corrisponde alla scheda",
            "セッションが基板と一致しません",
            "会话与电路板不匹配",
            "A sessão não corresponde à placa",
        ],
    ),
    (
        AUTOROUTER_CONFIG_INVALID,
        [
            "The autorouter settings are not valid",
            "La configuración del autorruteador no es válida",
            "Die Einstellungen des Autorouters sind ungültig",
            "Les réglages de l'autorouteur ne sont pas valides",
            "Le impostazioni dell'autorouter non sono valide",
            "オートルーターの設定が無効です",
            "自动布线器设置无效",
            "As definições do encaminhador automático não são válidas",
        ],
    ),
    (
        AUTOROUTER_DISABLED,
        [
            "The external autorouter is switched off",
            "El autorruteador externo está apagado",
            "Der externe Autorouter ist ausgeschaltet",
            "L'autorouteur externe est désactivé",
            "L'autorouter esterno è disattivato",
            "外部オートルーターはオフです",
            "外部自动布线器已关闭",
            "O encaminhador automático externo está desligado",
        ],
    ),
    (
        AUTOROUTER_LAUNCH_FAILED,
        [
            "The external autorouter could not be run",
            "No se pudo ejecutar el autorruteador externo",
            "Der externe Autorouter konnte nicht ausgeführt werden",
            "L'autorouteur externe n'a pas pu être exécuté",
            "Non è stato possibile eseguire l'autorouter esterno",
            "外部オートルーターを実行できませんでした",
            "无法运行外部自动布线器",
            "Não foi possível executar o encaminhador automático externo",
        ],
    ),
    (
        AUTOROUTER_TIMEOUT,
        [
            "The external autorouter ran out of time",
            "El autorruteador externo agotó el tiempo",
            "Die Zeit des externen Autorouters ist abgelaufen",
            "L'autorouteur externe a dépassé le temps imparti",
            "L'autorouter esterno ha superato il tempo limite",
            "外部オートルーターが制限時間を超えました",
            "外部自动布线器已超时",
            "O encaminhador automático externo excedeu o tempo limite",
        ],
    ),
    (
        AUTOROUTER_NO_OUTPUT,
        [
            "The external autorouter left no session",
            "El autorruteador externo no dejó ninguna sesión",
            "Der externe Autorouter hat keine Sitzung hinterlassen",
            "L'autorouteur externe n'a laissé aucune session",
            "L'autorouter esterno non ha lasciato alcuna sessione",
            "外部オートルーターがセッションを出力しませんでした",
            "外部自动布线器未生成会话",
            "O encaminhador automático externo não deixou nenhuma sessão",
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

/// An error of the editing and routing logic: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteError {
    /// The code, `PHOENIX-ROUTE-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl RouteError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> RouteError {
        RouteError {
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

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for RouteError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ROUTE-{:03}", i + 1));
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
        let e = RouteError::new(NO_PATH, "the end is walled in");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-ROUTE-007: Não foi encontrado nenhum caminho: the end is walled in"
        );
        assert_eq!(e.to_string(), "PHOENIX-ROUTE-007: the end is walled in");
        let bare = RouteError::new(SAME_PLACE, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-ROUTE-011: Both elements are at the same place"
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
