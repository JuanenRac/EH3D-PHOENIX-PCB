// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the filling of zones, `PHOENIX-ZONE-<NNN>`, with their titles in the
//! eight languages of the suite, the error type that carries one and the finding of a fill or a
//! check.

use std::fmt;

use eh3d_phoenix_board::Id;
use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The zone settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-ZONE-001";
/// The file of zone rules is not valid.
pub const ZONE_FILE_INVALID: &str = "PHOENIX-ZONE-002";
/// A zone named in the file is not on the board.
pub const ZONE_NOT_FOUND: &str = "PHOENIX-ZONE-003";
/// The outline of a zone cannot be filled.
pub const OUTLINE_UNUSABLE: &str = "PHOENIX-ZONE-004";
/// A limit of the fill was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-ZONE-005";
/// The fill is closer to other copper than the clearance.
pub const CLEARANCE_VIOLATED: &str = "PHOENIX-ZONE-006";
/// The fill lies outside the zone or the board.
pub const OUTSIDE_OUTLINE: &str = "PHOENIX-ZONE-007";
/// A copper island is narrower than the minimum width.
pub const ISLAND_NARROW: &str = "PHOENIX-ZONE-008";
/// A copper island is smaller than the minimum area.
pub const ISLAND_SMALL: &str = "PHOENIX-ZONE-009";
/// A copper island is not connected to its net.
pub const ISLAND_UNCONNECTED: &str = "PHOENIX-ZONE-010";
/// A thermal relief cannot be made.
pub const RELIEF_IMPOSSIBLE: &str = "PHOENIX-ZONE-011";
/// Zones of different nets share a priority and overlap.
pub const PRIORITY_TIE: &str = "PHOENIX-ZONE-012";
/// An arc was kept away from with a safe margin.
pub const ARC_CAUTIOUS: &str = "PHOENIX-ZONE-013";
/// A zone has no copper after the fill.
pub const FILL_EMPTY: &str = "PHOENIX-ZONE-014";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-ZONE-015";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-ZONE-016";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-ZONE-017";
/// A coordinate is out of range.
pub const OUT_OF_RANGE: &str = "PHOENIX-ZONE-018";
/// The fill file does not match the board.
pub const FILL_MISMATCH: &str = "PHOENIX-ZONE-019";
/// A copper island that touches no copper of its net was kept.
pub const ISLAND_KEPT: &str = "PHOENIX-ZONE-020";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Zone settings are not valid",
            "La configuración de las zonas no es válida",
            "Die Einstellungen der Zonen sind ungültig",
            "Les réglages des zones ne sont pas valides",
            "Le impostazioni delle zone non sono valide",
            "ゾーン設定が無効です",
            "铺铜设置无效",
            "As definições das zonas não são válidas",
        ],
    ),
    (
        ZONE_FILE_INVALID,
        [
            "Zone rules file is not valid",
            "El archivo de reglas de zonas no es válido",
            "Die Datei mit den Zonenregeln ist ungültig",
            "Le fichier des règles de zones n'est pas valide",
            "Il file delle regole delle zone non è valido",
            "ゾーンルールファイルが無効です",
            "铺铜规则文件无效",
            "O ficheiro de regras das zonas não é válido",
        ],
    ),
    (
        ZONE_NOT_FOUND,
        [
            "Zone is not on the board",
            "La zona no está en la placa",
            "Die Zone ist nicht auf der Platine",
            "La zone n'est pas sur la carte",
            "La zona non è sulla scheda",
            "ゾーンが基板上にありません",
            "铺铜区域不在电路板上",
            "A zona não está na placa",
        ],
    ),
    (
        OUTLINE_UNUSABLE,
        [
            "Zone outline cannot be filled",
            "No se puede rellenar el contorno de la zona",
            "Die Kontur der Zone kann nicht gefüllt werden",
            "Le contour de la zone ne peut pas être rempli",
            "Il contorno della zona non può essere riempito",
            "ゾーンの輪郭を塗りつぶせません",
            "无法填充铺铜轮廓",
            "O contorno da zona não pode ser preenchido",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the fill was exceeded",
            "Se superó un límite del relleno",
            "Ein Limit der Füllung wurde überschritten",
            "Une limite du remplissage a été dépassée",
            "Un limite del riempimento è stato superato",
            "塗りつぶしの上限を超えました",
            "超出了填充限制",
            "Foi excedido um limite do preenchimento",
        ],
    ),
    (
        CLEARANCE_VIOLATED,
        [
            "Fill is closer to other copper than the clearance",
            "El relleno está más cerca de otro cobre que la separación",
            "Die Füllung liegt näher an anderem Kupfer als der Abstand",
            "Le remplissage est plus près d'un autre cuivre que le dégagement",
            "Il riempimento è più vicino ad altro rame della distanza",
            "塗りつぶしが他の銅箔にクリアランスより近づいています",
            "填充与其他铜箔的距离小于间距",
            "O preenchimento está mais perto de outro cobre do que a distância",
        ],
    ),
    (
        OUTSIDE_OUTLINE,
        [
            "Fill lies outside the zone or the board",
            "El relleno queda fuera de la zona o de la placa",
            "Die Füllung liegt außerhalb der Zone oder der Platine",
            "Le remplissage est hors de la zone ou de la carte",
            "Il riempimento è fuori dalla zona o dalla scheda",
            "塗りつぶしがゾーンまたは基板の外にあります",
            "填充超出铺铜区域或电路板",
            "O preenchimento fica fora da zona ou da placa",
        ],
    ),
    (
        ISLAND_NARROW,
        [
            "Copper island narrower than the minimum width",
            "Isla de cobre más estrecha que el ancho mínimo",
            "Kupferinsel schmaler als die Mindestbreite",
            "Îlot de cuivre plus étroit que la largeur minimale",
            "Isola di rame più stretta della larghezza minima",
            "銅の島が最小幅より狭いです",
            "铜岛窄于最小宽度",
            "Ilha de cobre mais estreita do que a largura mínima",
        ],
    ),
    (
        ISLAND_SMALL,
        [
            "Copper island smaller than the minimum area",
            "Isla de cobre menor que el área mínima",
            "Kupferinsel kleiner als die Mindestfläche",
            "Îlot de cuivre plus petit que la surface minimale",
            "Isola di rame più piccola dell'area minima",
            "銅の島が最小面積より小さいです",
            "铜岛小于最小面积",
            "Ilha de cobre menor do que a área mínima",
        ],
    ),
    (
        ISLAND_UNCONNECTED,
        [
            "Copper island not connected to its net",
            "Isla de cobre sin conexión a su red",
            "Kupferinsel ohne Verbindung zu ihrem Netz",
            "Îlot de cuivre non relié à son réseau",
            "Isola di rame non collegata alla sua rete",
            "銅の島がネットに接続されていません",
            "铜岛未连接到其网络",
            "Ilha de cobre não ligada à sua rede",
        ],
    ),
    (
        RELIEF_IMPOSSIBLE,
        [
            "Thermal relief cannot be made",
            "No se puede hacer el alivio térmico",
            "Die Wärmefalle kann nicht erzeugt werden",
            "Le relief thermique ne peut pas être créé",
            "Il rilievo termico non può essere creato",
            "サーマルリリーフを作成できません",
            "无法生成热焊盘",
            "Não é possível fazer o alívio térmico",
        ],
    ),
    (
        PRIORITY_TIE,
        [
            "Zones of different nets share a priority and overlap",
            "Zonas de redes distintas con la misma prioridad se solapan",
            "Zonen verschiedener Netze mit gleicher Priorität überlappen sich",
            "Des zones de réseaux différents de même priorité se chevauchent",
            "Zone di reti diverse con la stessa priorità si sovrappongono",
            "同じ優先度の異なるネットのゾーンが重なっています",
            "不同网络、相同优先级的铺铜区域发生重叠",
            "Zonas de redes diferentes com a mesma prioridade sobrepõem-se",
        ],
    ),
    (
        ARC_CAUTIOUS,
        [
            "Arc kept away from with a safe margin",
            "Arco esquivado con un margen seguro",
            "Bogen mit Sicherheitsabstand umgangen",
            "Arc contourné avec une marge de sécurité",
            "Arco evitato con un margine di sicurezza",
            "円弧に安全マージンを取りました",
            "对圆弧留出了安全余量",
            "Arco evitado com uma margem de segurança",
        ],
    ),
    (
        FILL_EMPTY,
        [
            "Zone has no copper after the fill",
            "La zona no tiene cobre tras el relleno",
            "Die Zone hat nach der Füllung kein Kupfer",
            "La zone n'a plus de cuivre après le remplissage",
            "La zona non ha rame dopo il riempimento",
            "塗りつぶし後にゾーンに銅がありません",
            "填充后铺铜区域没有铜",
            "A zona fica sem cobre depois do preenchimento",
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
            "Board, rule or fill file cannot be read",
            "No se puede leer la placa, las reglas o el relleno",
            "Platine, Regeln oder Füllung können nicht gelesen werden",
            "La carte, les règles ou le remplissage sont illisibles",
            "Scheda, regole o riempimento non leggibili",
            "基板・ルール・塗りつぶしファイルを読み込めません",
            "无法读取电路板、规则或填充文件",
            "A placa, as regras ou o preenchimento não podem ser lidos",
        ],
    ),
    (
        OUT_OF_RANGE,
        [
            "A coordinate is out of range",
            "Una coordenada está fuera de rango",
            "Eine Koordinate liegt außerhalb des Bereichs",
            "Une coordonnée est hors plage",
            "Una coordinata è fuori intervallo",
            "座標が範囲外です",
            "坐标超出范围",
            "Uma coordenada está fora do intervalo",
        ],
    ),
    (
        FILL_MISMATCH,
        [
            "The fill file does not match the board",
            "El archivo de relleno no corresponde a la placa",
            "Die Füllungsdatei passt nicht zur Platine",
            "Le fichier de remplissage ne correspond pas à la carte",
            "Il file di riempimento non corrisponde alla scheda",
            "塗りつぶしファイルが基板と一致しません",
            "填充文件与电路板不匹配",
            "O ficheiro de preenchimento não corresponde à placa",
        ],
    ),
    (
        ISLAND_KEPT,
        [
            "Isolated copper island kept",
            "Isla de cobre aislada conservada",
            "Isolierte Kupferinsel beibehalten",
            "Îlot de cuivre isolé conservé",
            "Isola di rame isolata conservata",
            "孤立した銅の島を残しました",
            "已保留孤立的铜岛",
            "Ilha de cobre isolada mantida",
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
    /// Something the fill did on its own that the user may want to know.
    Info,
    /// Something to look at; the fill is still made.
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

/// A finding of a fill or a check: a registered code, where it is and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-ZONE-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The zone it is about, when it is about one.
    pub zone: Option<Id>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a zone.
    pub fn new(
        code: &'static str,
        severity: Severity,
        zone: Option<Id>,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            zone,
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if let Some(zone) = self.zone {
            out.push_str(&format!(" [zone {zone}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the filling of zones: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneError {
    /// The code, `PHOENIX-ZONE-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl ZoneError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> ZoneError {
        ZoneError {
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

impl fmt::Display for ZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ZoneError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ZONE-{:03}", i + 1));
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
        let e = ZoneError::new(OUTLINE_UNUSABLE, "two sides cross");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-ZONE-004: O contorno da zona não pode ser preenchido: two sides cross"
        );
        assert_eq!(e.to_string(), "PHOENIX-ZONE-004: two sides cross");
        let bare = ZoneError::new(FILL_EMPTY, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-ZONE-014: Zone has no copper after the fill"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_and_detail() {
        let f = Finding::new(ISLAND_SMALL, Severity::Info, None, "area 3");
        assert_eq!(
            f.line(Lang::En),
            "info: PHOENIX-ZONE-009: Copper island smaller than the minimum area: area 3"
        );
        assert!(Severity::Error > Severity::Warning);
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
