// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes of the rule engine and the connectivity checks, with their titles in the eight
//! languages of the suite.
//!
//! Areas: `CHECK` (the findings of the rule checks), `RULEFILE` (reading a rule file) and `CONN`
//! (connectivity of a board and its comparison with the schematic). The English title is the one
//! in [`REGISTRY`], which `docs/ERROR_CODES.md` repeats; the other languages are in [`title`].
//! A user interface translates by the key `diag.<code>`; the codes themselves are never translated.

/// A track narrower than the minimum.
pub const TRACK_NARROW: &str = "PHOENIX-CHECK-001";
/// A via whose pad is smaller than the minimum diameter.
pub const VIA_SMALL: &str = "PHOENIX-CHECK-002";
/// A drill smaller than the minimum.
pub const DRILL_SMALL: &str = "PHOENIX-CHECK-003";
/// An annular ring thinner than the minimum.
pub const RING_THIN: &str = "PHOENIX-CHECK-004";
/// Copper of different nets closer than the minimum clearance.
pub const CLEARANCE: &str = "PHOENIX-CHECK-005";
/// Copper closer to the edge of the board than the minimum.
pub const EDGE_CLOSE: &str = "PHOENIX-CHECK-006";
/// Two drill holes closer than the minimum.
pub const DRILL_SPACING: &str = "PHOENIX-CHECK-007";
/// Less solder mask between two openings than the minimum.
pub const MASK_BRIDGE: &str = "PHOENIX-CHECK-008";
/// A zone whose own clearance is smaller than the minimum.
pub const ZONE_CLEARANCE: &str = "PHOENIX-CHECK-009";
/// Zones of different nets that overlap.
pub const ZONES_OVERLAP: &str = "PHOENIX-CHECK-010";
/// Silkscreen printed over a pad.
pub const SILK_OVER_PAD: &str = "PHOENIX-CHECK-011";
/// Silkscreen printed over a solder mask opening.
pub const SILK_OVER_MASK: &str = "PHOENIX-CHECK-012";
/// A check that could not be judged.
pub const NOT_JUDGED: &str = "PHOENIX-CHECK-013";

/// A track narrower than the minimum of the net class of its net.
pub const CLASS_TRACK_NARROW: &str = "PHOENIX-CHECK-014";
/// A via whose diameter or drill is below the minimum of the net class of its net.
pub const CLASS_VIA_SMALL: &str = "PHOENIX-CHECK-015";

/// A rule file that cannot be read.
pub const RULEFILE_UNREADABLE: &str = "PHOENIX-RULEFILE-001";
/// A value of a rule file that is not valid.
pub const RULEFILE_VALUE: &str = "PHOENIX-RULEFILE-002";
/// A design rule parameter of an EAGLE board that this engine does not use.
pub const RULEFILE_PARAM: &str = "PHOENIX-RULEFILE-003";
/// An EAGLE board file with no design rules in it.
pub const RULEFILE_NO_RULES: &str = "PHOENIX-RULEFILE-004";

/// A net class that is not valid.
pub const RULEFILE_CLASS: &str = "PHOENIX-RULEFILE-005";

/// A net of the schematic that the board does not have.
pub const NET_MISSING: &str = "PHOENIX-CONN-001";
/// A pin that is not connected on the board.
pub const PIN_UNCONNECTED: &str = "PHOENIX-CONN-002";
/// A net of the board that joins the pins of several nets of the schematic.
pub const NETS_JOINED: &str = "PHOENIX-CONN-003";
/// Copper of different nets that touches.
pub const COPPER_SHORT: &str = "PHOENIX-CONN-004";
/// A net with connections still to route.
pub const NET_OPEN: &str = "PHOENIX-CONN-005";
/// Nets of the same name with different pins.
pub const NAME_SAME_PINS_DIFFER: &str = "PHOENIX-CONN-006";
/// The same pins under different net names.
pub const NAME_DIFFERS: &str = "PHOENIX-CONN-007";
/// A connection that could not be judged.
pub const CONN_NOT_JUDGED: &str = "PHOENIX-CONN-008";
/// A pad on a board net whose pin is on no net of the schematic.
pub const PAD_ONLY_ON_BOARD: &str = "PHOENIX-CONN-009";

/// The languages of the suite. Portuguese is the Portuguese of Portugal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {
    /// English.
    En,
    /// Spanish.
    Es,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese.
    Zh,
    /// Portuguese of Portugal.
    Pt,
}

impl Lang {
    /// The eight languages, in the order of the columns of the tables.
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The column of this language in the tables.
    pub(crate) fn column(self) -> usize {
        match self {
            Lang::En => 0,
            Lang::Es => 1,
            Lang::De => 2,
            Lang::Fr => 3,
            Lang::It => 4,
            Lang::Ja => 5,
            Lang::Zh => 6,
            Lang::Pt => 7,
        }
    }

    /// The two-letter tag of the language.
    pub fn tag(self) -> &'static str {
        ["en", "es", "de", "fr", "it", "ja", "zh", "pt"][self.column()]
    }

    /// Reads a tag such as `es`, `pt` or `pt-PT`; anything else is `None`.
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let lower = tag.trim().to_ascii_lowercase();
        let base = lower.split(['-', '_']).next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.tag() == base)
    }
}

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (TRACK_NARROW, "Track narrower than the minimum"),
    (VIA_SMALL, "Via diameter smaller than the minimum"),
    (DRILL_SMALL, "Drill smaller than the minimum"),
    (RING_THIN, "Annular ring thinner than the minimum"),
    (CLEARANCE, "Copper clearance violated"),
    (EDGE_CLOSE, "Copper too close to the board edge"),
    (DRILL_SPACING, "Drill holes too close to each other"),
    (MASK_BRIDGE, "Solder mask bridge too thin"),
    (ZONE_CLEARANCE, "Zone clearance smaller than the minimum"),
    (ZONES_OVERLAP, "Zones of different nets overlap"),
    (SILK_OVER_PAD, "Silkscreen printed over a pad"),
    (
        SILK_OVER_MASK,
        "Silkscreen printed over a solder mask opening",
    ),
    (NOT_JUDGED, "Check could not be judged"),
    (
        CLASS_TRACK_NARROW,
        "Track narrower than the minimum of its net class",
    ),
    (
        CLASS_VIA_SMALL,
        "Via smaller than the minimum of its net class",
    ),
    (RULEFILE_UNREADABLE, "Rule file cannot be read"),
    (RULEFILE_VALUE, "Rule value is not valid"),
    (RULEFILE_PARAM, "Design rule parameter not used"),
    (RULEFILE_NO_RULES, "Board file holds no design rules"),
    (RULEFILE_CLASS, "Net class is not valid"),
    (NET_MISSING, "Schematic net missing from the board"),
    (PIN_UNCONNECTED, "Pin not connected on the board"),
    (
        NETS_JOINED,
        "Board net joins pins of several schematic nets",
    ),
    (COPPER_SHORT, "Copper of different nets touches"),
    (NET_OPEN, "Net has unrouted connections"),
    (
        NAME_SAME_PINS_DIFFER,
        "Net of the same name holds different pins",
    ),
    (NAME_DIFFERS, "Net name differs for the same pins"),
    (CONN_NOT_JUDGED, "Connection could not be judged"),
    (
        PAD_ONLY_ON_BOARD,
        "Pad on a board net but its pin is on no schematic net",
    ),
];

/// The titles in the eight languages, in the order of [`Lang::ALL`].
const TITLES: &[(&str, [&str; 8])] = &[
    (
        TRACK_NARROW,
        [
            "Track narrower than the minimum",
            "Pista más estrecha que el mínimo",
            "Leiterbahn schmaler als das Minimum",
            "Piste plus étroite que le minimum",
            "Pista più stretta del minimo",
            "配線が最小幅より細い",
            "走线比最小宽度更窄",
            "Pista mais estreita do que o mínimo",
        ],
    ),
    (
        VIA_SMALL,
        [
            "Via diameter smaller than the minimum",
            "Diámetro de vía menor que el mínimo",
            "Via-Durchmesser kleiner als das Minimum",
            "Diamètre de via inférieur au minimum",
            "Diametro della via inferiore al minimo",
            "ビアの直径が最小値より小さい",
            "过孔直径小于最小值",
            "Diâmetro da via inferior ao mínimo",
        ],
    ),
    (
        DRILL_SMALL,
        [
            "Drill smaller than the minimum",
            "Taladro menor que el mínimo",
            "Bohrung kleiner als das Minimum",
            "Perçage plus petit que le minimum",
            "Foro più piccolo del minimo",
            "ドリル径が最小値より小さい",
            "钻孔小于最小值",
            "Furo mais pequeno do que o mínimo",
        ],
    ),
    (
        RING_THIN,
        [
            "Annular ring thinner than the minimum",
            "Anillo anular más fino que el mínimo",
            "Restring dünner als das Minimum",
            "Anneau de cuivre plus fin que le minimum",
            "Anello anulare più sottile del minimo",
            "アニュラリングが最小値より薄い",
            "焊环比最小值更薄",
            "Anel anular mais fino do que o mínimo",
        ],
    ),
    (
        CLEARANCE,
        [
            "Copper clearance violated",
            "Separación de cobre incumplida",
            "Kupferabstand unterschritten",
            "Isolement du cuivre non respecté",
            "Distanza del rame non rispettata",
            "銅箔の間隔が不足している",
            "铜间距不满足要求",
            "Distância do cobre não respeitada",
        ],
    ),
    (
        EDGE_CLOSE,
        [
            "Copper too close to the board edge",
            "Cobre demasiado cerca del borde de la placa",
            "Kupfer zu nah am Platinenrand",
            "Cuivre trop près du bord de la carte",
            "Rame troppo vicino al bordo della scheda",
            "銅箔が基板端に近すぎる",
            "铜箔距板边太近",
            "Cobre demasiado perto do limite da placa",
        ],
    ),
    (
        DRILL_SPACING,
        [
            "Drill holes too close to each other",
            "Taladros demasiado cerca entre sí",
            "Bohrungen zu nah beieinander",
            "Perçages trop proches les uns des autres",
            "Fori troppo vicini tra loro",
            "ドリル穴同士が近すぎる",
            "钻孔之间距离太近",
            "Furos demasiado próximos uns dos outros",
        ],
    ),
    (
        MASK_BRIDGE,
        [
            "Solder mask bridge too thin",
            "Puente de máscara de soldadura demasiado fino",
            "Lötstoppsteg zu schmal",
            "Pont de vernis épargne trop étroit",
            "Ponte di solder mask troppo sottile",
            "ソルダーマスクのブリッジが細すぎる",
            "阻焊桥太窄",
            "Ponte da máscara de soldadura demasiado fina",
        ],
    ),
    (
        ZONE_CLEARANCE,
        [
            "Zone clearance smaller than the minimum",
            "Separación de zona menor que el mínimo",
            "Zonenabstand kleiner als das Minimum",
            "Isolement de zone inférieur au minimum",
            "Distanza della zona inferiore al minimo",
            "ゾーンのクリアランスが最小値より小さい",
            "铜区间距小于最小值",
            "Distância da zona inferior ao mínimo",
        ],
    ),
    (
        ZONES_OVERLAP,
        [
            "Zones of different nets overlap",
            "Zonas de redes distintas se solapan",
            "Zonen verschiedener Netze überlappen",
            "Zones de réseaux différents qui se chevauchent",
            "Zone di reti diverse che si sovrappongono",
            "異なるネットのゾーンが重なっている",
            "不同网络的铜区重叠",
            "Zonas de redes diferentes sobrepõem-se",
        ],
    ),
    (
        SILK_OVER_PAD,
        [
            "Silkscreen printed over a pad",
            "Serigrafía impresa sobre un pad",
            "Bestückungsdruck über einem Pad",
            "Sérigraphie imprimée sur un pad",
            "Serigrafia stampata su un pad",
            "シルクがパッドにかかっている",
            "丝印印在焊盘上",
            "Serigrafia impressa sobre uma pastilha",
        ],
    ),
    (
        SILK_OVER_MASK,
        [
            "Silkscreen printed over a solder mask opening",
            "Serigrafía sobre una apertura de máscara",
            "Bestückungsdruck über einer Lötstopp-Öffnung",
            "Sérigraphie sur une ouverture du vernis épargne",
            "Serigrafia su un'apertura della solder mask",
            "シルクがマスク開口部にかかっている",
            "丝印压在阻焊开窗上",
            "Serigrafia sobre uma abertura da máscara de soldadura",
        ],
    ),
    (
        NOT_JUDGED,
        [
            "Check could not be judged",
            "La comprobación no pudo juzgarse",
            "Die Prüfung konnte nicht beurteilt werden",
            "La vérification n'a pas pu être jugée",
            "Il controllo non ha potuto essere giudicato",
            "判定できなかった",
            "无法判定该项检查",
            "A verificação não pôde ser avaliada",
        ],
    ),
    (
        CLASS_TRACK_NARROW,
        [
            "Track narrower than the minimum of its net class",
            "Pista más estrecha que el mínimo de su clase de red",
            "Leiterbahn schmaler als das Minimum ihrer Netzklasse",
            "Piste plus étroite que le minimum de sa classe de réseau",
            "Pista più stretta del minimo della sua classe di rete",
            "配線がネットクラスの最小幅より細い",
            "走线比其网络类的最小宽度更窄",
            "Pista mais estreita do que o mínimo da sua classe de rede",
        ],
    ),
    (
        CLASS_VIA_SMALL,
        [
            "Via smaller than the minimum of its net class",
            "Vía menor que el mínimo de su clase de red",
            "Via kleiner als das Minimum ihrer Netzklasse",
            "Via plus petite que le minimum de sa classe de réseau",
            "Via più piccola del minimo della sua classe di rete",
            "ビアがネットクラスの最小値より小さい",
            "过孔小于其网络类的最小值",
            "Via mais pequena do que o mínimo da sua classe de rede",
        ],
    ),
    (
        RULEFILE_CLASS,
        [
            "Net class is not valid",
            "Clase de red no válida",
            "Netzklasse ungültig",
            "Classe de réseau non valide",
            "Classe di rete non valida",
            "ネットクラスが正しくない",
            "网络类无效",
            "Classe de rede inválida",
        ],
    ),
    (
        RULEFILE_UNREADABLE,
        [
            "Rule file cannot be read",
            "No se puede leer el fichero de reglas",
            "Regeldatei nicht lesbar",
            "Fichier de règles illisible",
            "File delle regole illeggibile",
            "ルールファイルを読み込めない",
            "无法读取规则文件",
            "Não é possível ler o ficheiro de regras",
        ],
    ),
    (
        RULEFILE_VALUE,
        [
            "Rule value is not valid",
            "Valor de regla no válido",
            "Ungültiger Regelwert",
            "Valeur de règle non valide",
            "Valore della regola non valido",
            "ルールの値が正しくない",
            "规则值无效",
            "Valor de regra inválido",
        ],
    ),
    (
        RULEFILE_PARAM,
        [
            "Design rule parameter not used",
            "Parámetro de reglas de diseño sin usar",
            "Entwurfsregel-Parameter nicht verwendet",
            "Paramètre de règles de conception non utilisé",
            "Parametro delle regole di progetto non usato",
            "未使用のデザインルール項目",
            "未使用的设计规则参数",
            "Parâmetro de regras de projeto não utilizado",
        ],
    ),
    (
        RULEFILE_NO_RULES,
        [
            "Board file holds no design rules",
            "El fichero de placa no contiene reglas de diseño",
            "Die Platinendatei enthält keine Entwurfsregeln",
            "Le fichier de carte ne contient pas de règles de conception",
            "Il file della scheda non contiene regole di progetto",
            "基板ファイルにデザインルールがない",
            "板文件中没有设计规则",
            "O ficheiro da placa não contém regras de projeto",
        ],
    ),
    (
        NET_MISSING,
        [
            "Schematic net missing from the board",
            "Red del esquema ausente en la placa",
            "Netz des Schaltplans fehlt auf der Platine",
            "Réseau du schéma absent de la carte",
            "Rete dello schema assente dalla scheda",
            "回路図のネットが基板にない",
            "原理图网络在电路板上缺失",
            "Rede do esquema em falta na placa",
        ],
    ),
    (
        PIN_UNCONNECTED,
        [
            "Pin not connected on the board",
            "Pin sin conectar en la placa",
            "Pin auf der Platine nicht angeschlossen",
            "Broche non connectée sur la carte",
            "Pin non collegato sulla scheda",
            "基板上でピンが未接続",
            "引脚在电路板上未连接",
            "Pino sem ligação na placa",
        ],
    ),
    (
        NETS_JOINED,
        [
            "Board net joins pins of several schematic nets",
            "Una red de la placa une pines de varias redes del esquema",
            "Ein Platinennetz verbindet Pins mehrerer Schaltplannetze",
            "Un réseau de la carte réunit des broches de plusieurs réseaux du schéma",
            "Una rete della scheda unisce pin di più reti dello schema",
            "基板の1つのネットが回路図の複数ネットのピンを結んでいる",
            "电路板上的一个网络连接了多个原理图网络的引脚",
            "Uma rede da placa une pinos de várias redes do esquema",
        ],
    ),
    (
        COPPER_SHORT,
        [
            "Copper of different nets touches",
            "El cobre de redes distintas se toca",
            "Kupfer verschiedener Netze berührt sich",
            "Le cuivre de réseaux différents se touche",
            "Il rame di reti diverse si tocca",
            "異なるネットの銅箔が接触している",
            "不同网络的铜箔相接触",
            "O cobre de redes diferentes toca-se",
        ],
    ),
    (
        NET_OPEN,
        [
            "Net has unrouted connections",
            "La red tiene conexiones sin rutar",
            "Das Netz hat nicht verlegte Verbindungen",
            "Le réseau a des connexions non routées",
            "La rete ha connessioni non instradate",
            "ネットに未配線の接続がある",
            "网络存在未布线的连接",
            "A rede tem ligações por encaminhar",
        ],
    ),
    (
        NAME_SAME_PINS_DIFFER,
        [
            "Net of the same name holds different pins",
            "Una red del mismo nombre tiene pines distintos",
            "Ein gleichnamiges Netz enthält andere Pins",
            "Un réseau du même nom contient d'autres broches",
            "Una rete con lo stesso nome contiene pin diversi",
            "同名のネットでピンが異なる",
            "同名网络包含的引脚不同",
            "Uma rede com o mesmo nome contém pinos diferentes",
        ],
    ),
    (
        NAME_DIFFERS,
        [
            "Net name differs for the same pins",
            "El nombre de la red difiere para los mismos pines",
            "Der Netzname weicht bei denselben Pins ab",
            "Le nom du réseau diffère pour les mêmes broches",
            "Il nome della rete differisce per gli stessi pin",
            "同じピンでネット名が異なる",
            "相同引脚的网络名称不同",
            "O nome da rede difere para os mesmos pinos",
        ],
    ),
    (
        CONN_NOT_JUDGED,
        [
            "Connection could not be judged",
            "La conexión no pudo juzgarse",
            "Die Verbindung konnte nicht beurteilt werden",
            "La connexion n'a pas pu être jugée",
            "La connessione non ha potuto essere giudicata",
            "接続を判定できなかった",
            "无法判定该连接",
            "A ligação não pôde ser avaliada",
        ],
    ),
    (
        PAD_ONLY_ON_BOARD,
        [
            "Pad on a board net but its pin is on no schematic net",
            "Pad en una red de la placa pero su pin no está en ninguna red del esquema",
            "Pad liegt auf einem Platinennetz, sein Pin aber auf keinem Schaltplannetz",
            "Pad sur un réseau de la carte mais sa broche n'est sur aucun réseau du schéma",
            "Pad su una rete della scheda ma il suo pin non è su nessuna rete dello schema",
            "パッドは基板のネット上にあるが、そのピンは回路図のどのネットにもない",
            "焊盘在电路板网络上，但其引脚不在任何原理图网络中",
            "Pastilha numa rede da placa mas o seu pino não está em nenhuma rede do esquema",
        ],
    ),
];

/// The words of the text report in the eight languages.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "confirmed",
        [
            "Confirmed",
            "Confirmados",
            "Bestätigt",
            "Confirmés",
            "Confermati",
            "確定",
            "已确认",
            "Confirmados",
        ],
    ),
    (
        "inconclusive",
        [
            "Inconclusive",
            "No concluyentes",
            "Nicht entscheidbar",
            "Non concluants",
            "Non conclusivi",
            "判定不能",
            "无法判定",
            "Inconclusivos",
        ],
    ),
    (
        "measured",
        [
            "measured",
            "medido",
            "gemessen",
            "mesuré",
            "misurato",
            "実測値",
            "实测值",
            "medido",
        ],
    ),
    (
        "limit",
        [
            "limit",
            "límite",
            "Grenzwert",
            "limite",
            "limite",
            "制限値",
            "限值",
            "limite",
        ],
    ),
    (
        "nothing",
        [
            "nothing found",
            "nada encontrado",
            "nichts gefunden",
            "rien trouvé",
            "niente trovato",
            "検出なし",
            "未发现问题",
            "nada encontrado",
        ],
    ),
];

/// The title of a code in a language, or `None` when the code is not in the registry.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang.column()])
        .or_else(|| crate::advanced::codes::title(code, lang))
}

/// A word of the text report in a language; an unknown key gives an empty text.
pub fn word(key: &str, lang: Lang) -> &'static str {
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, texts)| texts[lang.column()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_all_eight_languages() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for (code, english) in REGISTRY {
            for lang in Lang::ALL {
                let text = title(code, lang).unwrap_or_default();
                assert!(!text.is_empty(), "{code} has no title in {}", lang.tag());
            }
            assert_eq!(title(code, Lang::En), Some(*english), "{code}");
        }
        for (key, texts) in WORDS {
            assert!(texts.iter().all(|t| !t.is_empty()), "{key}");
        }
    }

    #[test]
    fn titles_differ_between_languages_except_where_they_may_agree() {
        // the Spanish and the Portuguese texts are never copies of the English one
        for (code, _) in REGISTRY {
            let en = title(code, Lang::En).unwrap_or_default();
            assert_ne!(title(code, Lang::Es), Some(en), "{code}");
            assert_ne!(title(code, Lang::Pt), Some(en), "{code}");
            assert_ne!(title(code, Lang::Ja), Some(en), "{code}");
        }
    }

    #[test]
    fn language_tags_are_read_with_or_without_a_region() {
        assert_eq!(Lang::from_tag("pt-PT"), Some(Lang::Pt));
        assert_eq!(Lang::from_tag("ES"), Some(Lang::Es));
        assert_eq!(Lang::from_tag("zh_CN"), Some(Lang::Zh));
        assert_eq!(Lang::from_tag("xx"), None);
        for lang in Lang::ALL {
            assert_eq!(Lang::from_tag(lang.tag()), Some(lang));
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
