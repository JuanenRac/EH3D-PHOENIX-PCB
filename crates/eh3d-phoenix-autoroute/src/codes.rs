// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-AROUTE-<NNN>` of the autorouter, with their titles in the eight
//! languages of the suite and the error that carries one.

use std::fmt;

/// The settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-AROUTE-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-AROUTE-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-AROUTE-003";
/// A net named in the request is not on the board.
pub const NET_UNKNOWN: &str = "PHOENIX-AROUTE-004";
/// A layer named in the request is not a copper layer of the board.
pub const LAYER_UNKNOWN: &str = "PHOENIX-AROUTE-005";
/// The routing grid would have more nodes than the limit.
pub const GRID_TOO_LARGE: &str = "PHOENIX-AROUTE-006";
/// The board cannot be routed: no outline or no copper layer.
pub const BOARD_NOT_ROUTABLE: &str = "PHOENIX-AROUTE-007";
/// There are more connections than the limit.
pub const TOO_MANY_CONNECTIONS: &str = "PHOENIX-AROUTE-008";
/// The result cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-AROUTE-009";
/// The board refused the operations of the plan.
pub const APPLY_FAILED: &str = "PHOENIX-AROUTE-010";
/// No free place near a pad to start or end a track.
pub const NO_TERMINAL: &str = "PHOENIX-AROUTE-011";
/// No path exists on the routing grid.
pub const NO_PATH: &str = "PHOENIX-AROUTE-012";
/// The search reached its limit of expanded nodes.
pub const EXPANSION_LIMIT: &str = "PHOENIX-AROUTE-013";
/// The rip-up and reroute reached its limit.
pub const RIPUP_LIMIT: &str = "PHOENIX-AROUTE-014";
/// The routes of a net were refused because they add a rule error.
pub const REJECTED_BY_RULES: &str = "PHOENIX-AROUTE-015";
/// The plan leaves connections unrouted.
pub const INCOMPLETE: &str = "PHOENIX-AROUTE-016";
/// A differential pair was routed as two ordinary nets, not as two coupled tracks.
pub const PAIR_UNCOUPLED: &str = "PHOENIX-AROUTE-017";
/// A differential pair was routed across two layers, with one via for each track.
pub const PAIR_WITH_VIA: &str = "PHOENIX-AROUTE-018";
/// An existing via that is not on a node of the grid was joined to the grid by a short piece.
pub const VIA_SNAPPED: &str = "PHOENIX-AROUTE-019";

/// The language tags, in the order of the columns of [`TITLES`].
pub const LANGUAGES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Autorouter settings are not valid",
            "La configuración del autorruteador no es válida",
            "Die Einstellungen des Autorouters sind ungültig",
            "Les réglages de l'autorouteur ne sont pas valides",
            "Le impostazioni dell'autorouter non sono valide",
            "オートルータの設定が無効です",
            "自动布线器设置无效",
            "As definições do autorroteador não são válidas",
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
        INPUT_UNREADABLE,
        [
            "Input file cannot be read",
            "No se puede leer el fichero de entrada",
            "Die Eingabedatei kann nicht gelesen werden",
            "Le fichier d'entrée ne peut pas être lu",
            "Il file di ingresso non può essere letto",
            "入力ファイルを読み取れません",
            "无法读取输入文件",
            "O ficheiro de entrada não pode ser lido",
        ],
    ),
    (
        NET_UNKNOWN,
        [
            "A net named in the request is not on the board",
            "Una red nombrada en la petición no está en la placa",
            "Ein in der Anfrage genanntes Netz ist nicht auf der Platine",
            "Un réseau nommé dans la demande n'est pas sur la carte",
            "Una rete indicata nella richiesta non è sulla scheda",
            "要求で指定されたネットが基板にありません",
            "请求中指定的网络不在电路板上",
            "Uma rede indicada no pedido não está na placa",
        ],
    ),
    (
        LAYER_UNKNOWN,
        [
            "A layer named in the request is not a copper layer of the board",
            "Una capa nombrada en la petición no es una capa de cobre de la placa",
            "Eine in der Anfrage genannte Lage ist keine Kupferlage der Platine",
            "Une couche nommée dans la demande n'est pas une couche de cuivre de la carte",
            "Un livello indicato nella richiesta non è un livello di rame della scheda",
            "要求で指定された層は基板の銅層ではありません",
            "请求中指定的层不是电路板的铜层",
            "Uma camada indicada no pedido não é uma camada de cobre da placa",
        ],
    ),
    (
        GRID_TOO_LARGE,
        [
            "The routing grid would have more nodes than the limit",
            "La rejilla de enrutado tendría más nodos que el límite",
            "Das Routing-Raster hätte mehr Knoten als das Limit erlaubt",
            "La grille de routage aurait plus de nœuds que la limite",
            "La griglia di instradamento avrebbe più nodi del limite",
            "配線グリッドのノード数が上限を超えます",
            "布线网格的节点数将超过上限",
            "A grelha de encaminhamento teria mais nós do que o limite",
        ],
    ),
    (
        BOARD_NOT_ROUTABLE,
        [
            "The board has no closed outline or no copper layer to route on",
            "La placa no tiene contorno cerrado o capa de cobre donde rutear",
            "Die Platine hat keine geschlossene Kontur oder keine Kupferlage zum Verlegen",
            "La carte n'a pas de contour fermé ou de couche de cuivre pour router",
            "La scheda non ha un contorno chiuso o un livello di rame su cui instradare",
            "基板に閉じた外形または配線できる銅層がありません",
            "电路板没有封闭轮廓或可布线的铜层",
            "A placa não tem contorno fechado nem camada de cobre onde encaminhar",
        ],
    ),
    (
        TOO_MANY_CONNECTIONS,
        [
            "There are more connections to route than the limit",
            "Hay más conexiones por rutear que el límite",
            "Es gibt mehr zu verlegende Verbindungen als das Limit erlaubt",
            "Il y a plus de connexions à router que la limite",
            "Ci sono più connessioni da instradare del limite",
            "配線する接続の数が上限を超えています",
            "待布线的连接数超过上限",
            "Há mais ligações por encaminhar do que o limite",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "The result cannot be written",
            "No se puede escribir el resultado",
            "Das Ergebnis kann nicht geschrieben werden",
            "Le résultat ne peut pas être écrit",
            "Il risultato non può essere scritto",
            "結果を書き込めません",
            "无法写入结果",
            "Não é possível escrever o resultado",
        ],
    ),
    (
        APPLY_FAILED,
        [
            "The board refused the operations of the plan",
            "La placa rechazó las operaciones del plan",
            "Die Platine hat die Operationen des Plans abgelehnt",
            "La carte a refusé les opérations du plan",
            "La scheda ha rifiutato le operazioni del piano",
            "基板が計画の操作を拒否しました",
            "电路板拒绝了计划中的操作",
            "A placa recusou as operações do plano",
        ],
    ),
    (
        NO_TERMINAL,
        [
            "No free place near a pad to start or end a track",
            "No hay un sitio libre junto a un pad para empezar o acabar una pista",
            "Neben einem Pad ist kein freier Platz, um eine Leiterbahn zu beginnen oder zu beenden",
            "Il n'y a pas de place libre près d'un pad pour commencer ou finir une piste",
            "Non c'è posto libero vicino a un pad per iniziare o finire una pista",
            "パッドの近くにトラックを始点または終点にできる空きがありません",
            "焊盘附近没有可用于开始或结束走线的空位",
            "Não há lugar livre junto a um pad para começar ou acabar uma pista",
        ],
    ),
    (
        NO_PATH,
        [
            "No path exists on the routing grid",
            "No existe camino en la rejilla de enrutado",
            "Auf dem Routing-Raster gibt es keinen Weg",
            "Aucun chemin n'existe sur la grille de routage",
            "Non esiste alcun percorso sulla griglia di instradamento",
            "配線グリッド上に経路がありません",
            "布线网格上不存在路径",
            "Não existe caminho na grelha de encaminhamento",
        ],
    ),
    (
        EXPANSION_LIMIT,
        [
            "The search reached its limit of expanded nodes",
            "La búsqueda alcanzó su límite de nodos expandidos",
            "Die Suche hat ihr Limit an erweiterten Knoten erreicht",
            "La recherche a atteint sa limite de nœuds développés",
            "La ricerca ha raggiunto il limite di nodi espansi",
            "探索が展開ノード数の上限に達しました",
            "搜索达到了展开节点数的上限",
            "A pesquisa atingiu o limite de nós expandidos",
        ],
    ),
    (
        RIPUP_LIMIT,
        [
            "Rip-up and reroute reached its limit",
            "El deshacer y reenrutar alcanzó su límite",
            "Das Auftrennen und Neuverlegen hat sein Limit erreicht",
            "Le défaire et reprendre le routage a atteint sa limite",
            "Il disfacimento e reinstradamento ha raggiunto il limite",
            "引き剥がしと再配線が上限に達しました",
            "拆线重布达到了上限",
            "O desfazer e reencaminhar atingiu o limite",
        ],
    ),
    (
        REJECTED_BY_RULES,
        [
            "The routes of a net were refused because they add a rule error",
            "Las rutas de una red se rechazaron porque añaden un error de reglas",
            "Die Verlegung eines Netzes wurde abgelehnt, weil sie einen Regelfehler hinzufügt",
            "Les routes d'un réseau ont été refusées car elles ajoutent une erreur de règles",
            "I percorsi di una rete sono stati rifiutati perché aggiungono un errore di regole",
            "ルール違反を追加するため、ネットの配線は拒否されました",
            "由于会增加规则错误，某个网络的走线被拒绝",
            "As rotas de uma rede foram recusadas porque acrescentam um erro de regras",
        ],
    ),
    (
        INCOMPLETE,
        [
            "The plan leaves connections unrouted",
            "El plan deja conexiones sin rutear",
            "Der Plan lässt Verbindungen unverlegt",
            "Le plan laisse des connexions non routées",
            "Il piano lascia connessioni non instradate",
            "計画では未配線の接続が残ります",
            "该计划留下未布线的连接",
            "O plano deixa ligações por encaminhar",
        ],
    ),
    (
        PAIR_UNCOUPLED,
        [
            "A differential pair was routed as two ordinary nets",
            "Un par diferencial se rutó como dos redes normales",
            "Ein Differenzpaar wurde als zwei gewöhnliche Netze verlegt",
            "Une paire différentielle a été routée comme deux réseaux ordinaires",
            "Una coppia differenziale è stata instradata come due reti normali",
            "差動ペアは通常の2本のネットとして配線されました",
            "差分对被按两个普通网络布线",
            "Um par diferencial foi encaminhado como duas redes normais",
        ],
    ),
    (
        PAIR_WITH_VIA,
        [
            "A differential pair was routed across two layers with a via for each track",
            "Un par diferencial se rutó entre dos capas con una vía por pista",
            "Ein Differenzpaar wurde über zwei Lagen mit einer Durchkontaktierung je Leiterbahn verlegt",
            "Une paire différentielle a été routée sur deux couches avec un via par piste",
            "Una coppia differenziale è stata instradata su due strati con un via per pista",
            "差動ペアは2つの層にまたがり、配線ごとに1つのビアで配線されました",
            "差分对跨两层布线，每条走线使用一个过孔",
            "Um par diferencial foi encaminhado em duas camadas com uma via por pista",
        ],
    ),
    (
        VIA_SNAPPED,
        [
            "An existing via off the grid was joined to the grid by a short piece",
            "Una vía existente fuera de la rejilla se unió a la rejilla con un tramo corto",
            "Eine vorhandene Durchkontaktierung abseits des Rasters wurde mit einem kurzen Stück an das Raster angeschlossen",
            "Un via existant hors de la grille a été relié à la grille par un court tronçon",
            "Un via esistente fuori dalla griglia è stato collegato alla griglia con un breve tratto",
            "グリッド外の既存のビアを短い線分でグリッドに接続しました",
            "网格外的现有过孔通过一小段走线接入网格",
            "Uma via existente fora da grelha foi ligada à grelha por um troço curto",
        ],
    ),
];

/// The codes of this crate with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in the language with this tag (`en` when the tag is not known); empty for a
/// code that is not in the table.
pub fn title(code: &str, tag: &str) -> &'static str {
    let column = LANGUAGES.iter().position(|t| *t == tag).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// An error with its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutorouteError {
    /// The code.
    pub code: &'static str,
    /// The detail, in English.
    pub detail: String,
}

impl AutorouteError {
    /// An error.
    pub fn new(code: &'static str, detail: impl Into<String>) -> AutorouteError {
        AutorouteError {
            code,
            detail: detail.into(),
        }
    }

    /// The error as a message, with the title in the language with this tag.
    pub fn message(&self, tag: &str) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, tag));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for AutorouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for AutorouteError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-AROUTE-{:03}", i + 1));
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
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = AutorouteError::new(NO_PATH, "net SIG");
        assert_eq!(
            e.message("pt"),
            "PHOENIX-AROUTE-012: Não existe caminho na grelha de encaminhamento: net SIG"
        );
        assert_eq!(e.to_string(), "PHOENIX-AROUTE-012: net SIG");
        assert_eq!(title("PHOENIX-AROUTE-999", "en"), "");
        assert_eq!(
            title(SETTINGS_INVALID, "xx"),
            "Autorouter settings are not valid"
        );
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        for (code, row) in TITLES {
            let text = row[7];
            assert!(!text.contains("arquivo"), "{code}");
            assert!(!text.contains("usuário"), "{code}");
        }
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
