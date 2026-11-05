// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-ERC-<NNN>` of the electrical rule check, the rules behind them and
//! their titles in the eight languages of the suite.

use eh3d_phoenix_rules::Lang;

/// A pin that is not on any net.
pub const PIN_UNCONNECTED: &str = "PHOENIX-ERC-001";
/// A net with one pin only, or none.
pub const SINGLE_PIN_NET: &str = "PHOENIX-ERC-002";
/// Two pins on one net whose types the matrix does not allow.
pub const PIN_TYPE_CONFLICT: &str = "PHOENIX-ERC-003";
/// A net with input pins and nothing that drives it.
pub const INPUT_WITHOUT_DRIVER: &str = "PHOENIX-ERC-004";
/// A net with power input pins and no power source.
pub const POWER_WITHOUT_SOURCE: &str = "PHOENIX-ERC-005";
/// Two net names that differ only by case or by a number on the end.
pub const SIMILAR_NET_NAMES: &str = "PHOENIX-ERC-006";
/// A reference designator used by more than one part.
pub const DUPLICATE_REFDES: &str = "PHOENIX-ERC-007";
/// A part with an empty value.
pub const EMPTY_VALUE: &str = "PHOENIX-ERC-008";
/// A part with an empty reference designator.
pub const EMPTY_REFDES: &str = "PHOENIX-ERC-009";
/// A part with no footprint chosen.
pub const PART_WITHOUT_FOOTPRINT: &str = "PHOENIX-ERC-010";
/// A pin of a symbol with an empty number.
pub const PIN_WITHOUT_NUMBER: &str = "PHOENIX-ERC-011";
/// A symbol with one pin number on more than one pin.
pub const DUPLICATE_PIN_NUMBER: &str = "PHOENIX-ERC-012";
/// Wires that are joined to each other but to no pin.
pub const FLOATING_WIRES: &str = "PHOENIX-ERC-013";
/// A point where three or more wire pieces meet, on wires that join no pin.
pub const JUNCTION_ON_FLOATING_WIRES: &str = "PHOENIX-ERC-014";
/// A check that could not be judged (a pin without a type, a pin without a position, a cut list).
pub const NOT_JUDGED: &str = "PHOENIX-ERC-015";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-ERC-016";

/// The rules that can be set in the settings, in the fixed order of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// A pin that is on no net.
    PinUnconnected,
    /// A net with one pin only.
    SinglePinNet,
    /// Pin types that the matrix does not allow together.
    PinTypeConflict,
    /// Input pins with no driver on the net.
    InputWithoutDriver,
    /// Power inputs with no source on the net.
    PowerWithoutSource,
    /// Net names that look alike.
    SimilarNetNames,
    /// A reference designator used twice.
    DuplicateRefdes,
    /// An empty value.
    EmptyValue,
    /// An empty reference designator.
    EmptyRefdes,
    /// A part without a footprint.
    PartWithoutFootprint,
    /// A symbol pin without a number.
    PinWithoutNumber,
    /// A symbol with a repeated pin number.
    DuplicatePinNumber,
    /// Wires that join no pin.
    FloatingWires,
    /// A junction on wires that join no pin.
    JunctionOnFloatingWires,
}

impl Rule {
    /// Every rule, in the order of the report.
    pub const ALL: [Rule; 14] = [
        Rule::PinUnconnected,
        Rule::SinglePinNet,
        Rule::PinTypeConflict,
        Rule::InputWithoutDriver,
        Rule::PowerWithoutSource,
        Rule::SimilarNetNames,
        Rule::DuplicateRefdes,
        Rule::EmptyValue,
        Rule::EmptyRefdes,
        Rule::PartWithoutFootprint,
        Rule::PinWithoutNumber,
        Rule::DuplicatePinNumber,
        Rule::FloatingWires,
        Rule::JunctionOnFloatingWires,
    ];

    /// The place of the rule in the order of the report, from 1.
    pub fn rank(self) -> u8 {
        match Rule::ALL.iter().position(|r| *r == self) {
            Some(i) => i as u8 + 1,
            None => 0,
        }
    }

    /// The position of the rule in [`Rule::ALL`].
    pub(crate) fn index(self) -> usize {
        usize::from(self.rank()).saturating_sub(1)
    }

    /// The stable name used in the settings and in the report (`pin_unconnected`).
    pub fn name(self) -> &'static str {
        match self {
            Rule::PinUnconnected => "pin_unconnected",
            Rule::SinglePinNet => "single_pin_net",
            Rule::PinTypeConflict => "pin_type_conflict",
            Rule::InputWithoutDriver => "input_without_driver",
            Rule::PowerWithoutSource => "power_without_source",
            Rule::SimilarNetNames => "similar_net_names",
            Rule::DuplicateRefdes => "duplicate_refdes",
            Rule::EmptyValue => "empty_value",
            Rule::EmptyRefdes => "empty_refdes",
            Rule::PartWithoutFootprint => "part_without_footprint",
            Rule::PinWithoutNumber => "pin_without_number",
            Rule::DuplicatePinNumber => "duplicate_pin_number",
            Rule::FloatingWires => "floating_wires",
            Rule::JunctionOnFloatingWires => "junction_on_floating_wires",
        }
    }

    /// The stable code of the rule (`PHOENIX-ERC-001`).
    pub fn code(self) -> &'static str {
        match self {
            Rule::PinUnconnected => PIN_UNCONNECTED,
            Rule::SinglePinNet => SINGLE_PIN_NET,
            Rule::PinTypeConflict => PIN_TYPE_CONFLICT,
            Rule::InputWithoutDriver => INPUT_WITHOUT_DRIVER,
            Rule::PowerWithoutSource => POWER_WITHOUT_SOURCE,
            Rule::SimilarNetNames => SIMILAR_NET_NAMES,
            Rule::DuplicateRefdes => DUPLICATE_REFDES,
            Rule::EmptyValue => EMPTY_VALUE,
            Rule::EmptyRefdes => EMPTY_REFDES,
            Rule::PartWithoutFootprint => PART_WITHOUT_FOOTPRINT,
            Rule::PinWithoutNumber => PIN_WITHOUT_NUMBER,
            Rule::DuplicatePinNumber => DUPLICATE_PIN_NUMBER,
            Rule::FloatingWires => FLOATING_WIRES,
            Rule::JunctionOnFloatingWires => JUNCTION_ON_FLOATING_WIRES,
        }
    }

    /// Reads the name used in the settings.
    pub fn from_name(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (PIN_UNCONNECTED, "Pin is not connected"),
    (SINGLE_PIN_NET, "Net has a single pin"),
    (PIN_TYPE_CONFLICT, "Pin types in conflict on a net"),
    (INPUT_WITHOUT_DRIVER, "Input pins have no driver"),
    (POWER_WITHOUT_SOURCE, "Power input has no source"),
    (SIMILAR_NET_NAMES, "Net names look alike"),
    (DUPLICATE_REFDES, "Reference designator used twice"),
    (EMPTY_VALUE, "Part has no value"),
    (EMPTY_REFDES, "Part has no reference designator"),
    (PART_WITHOUT_FOOTPRINT, "Part has no footprint"),
    (PIN_WITHOUT_NUMBER, "Symbol pin has no number"),
    (DUPLICATE_PIN_NUMBER, "Pin number used twice in a symbol"),
    (FLOATING_WIRES, "Wires join no pin"),
    (
        JUNCTION_ON_FLOATING_WIRES,
        "Junction on wires that join no pin",
    ),
    (NOT_JUDGED, "Check could not be judged"),
    (SETTINGS_INVALID, "Settings are not valid"),
];

/// The titles in the eight languages, in the order of [`Lang::ALL`].
const TITLES: &[(&str, [&str; 8])] = &[
    (
        PIN_UNCONNECTED,
        [
            "Pin is not connected",
            "Pin sin conectar",
            "Pin nicht angeschlossen",
            "Broche non connectée",
            "Pin non collegato",
            "ピンが未接続",
            "引脚未连接",
            "Pino sem ligação",
        ],
    ),
    (
        SINGLE_PIN_NET,
        [
            "Net has a single pin",
            "Red con un solo pin",
            "Netz mit nur einem Pin",
            "Réseau avec une seule broche",
            "Rete con un solo pin",
            "ピンが1本だけのネット",
            "网络只有一个引脚",
            "Rede com um só pino",
        ],
    ),
    (
        PIN_TYPE_CONFLICT,
        [
            "Pin types in conflict on a net",
            "Tipos de pin en conflicto en una red",
            "Pin-Typen im Konflikt in einem Netz",
            "Types de broches en conflit sur un réseau",
            "Tipi di pin in conflitto su una rete",
            "ネット上でピンの種類が衝突している",
            "网络上的引脚类型冲突",
            "Tipos de pino em conflito numa rede",
        ],
    ),
    (
        INPUT_WITHOUT_DRIVER,
        [
            "Input pins have no driver",
            "Pines de entrada sin ningún emisor",
            "Eingangspins ohne Treiber",
            "Broches d'entrée sans émetteur",
            "Pin di ingresso senza pilota",
            "入力ピンを駆動するものがない",
            "输入引脚没有驱动源",
            "Pinos de entrada sem emissor",
        ],
    ),
    (
        POWER_WITHOUT_SOURCE,
        [
            "Power input has no source",
            "Entrada de alimentación sin fuente",
            "Versorgungseingang ohne Quelle",
            "Entrée d'alimentation sans source",
            "Ingresso di alimentazione senza sorgente",
            "電源入力に電源がない",
            "电源输入没有电源",
            "Entrada de alimentação sem fonte",
        ],
    ),
    (
        SIMILAR_NET_NAMES,
        [
            "Net names look alike",
            "Nombres de red parecidos",
            "Netznamen sehen ähnlich aus",
            "Noms de réseau trop proches",
            "Nomi di rete simili",
            "ネット名が似ている",
            "网络名称相似",
            "Nomes de rede parecidos",
        ],
    ),
    (
        DUPLICATE_REFDES,
        [
            "Reference designator used twice",
            "Referencia repetida",
            "Bauteilbezeichnung doppelt vergeben",
            "Référence utilisée deux fois",
            "Riferimento usato due volte",
            "リファレンスが重複している",
            "位号重复使用",
            "Referência repetida",
        ],
    ),
    (
        EMPTY_VALUE,
        [
            "Part has no value",
            "Pieza sin valor",
            "Bauteil ohne Wert",
            "Composant sans valeur",
            "Componente senza valore",
            "部品に値がない",
            "元件没有值",
            "Peça sem valor",
        ],
    ),
    (
        EMPTY_REFDES,
        [
            "Part has no reference designator",
            "Pieza sin referencia",
            "Bauteil ohne Bezeichnung",
            "Composant sans référence",
            "Componente senza riferimento",
            "部品にリファレンスがない",
            "元件没有位号",
            "Peça sem referência",
        ],
    ),
    (
        PART_WITHOUT_FOOTPRINT,
        [
            "Part has no footprint",
            "Pieza sin huella",
            "Bauteil ohne Footprint",
            "Composant sans empreinte",
            "Componente senza impronta",
            "部品にフットプリントがない",
            "元件没有封装",
            "Peça sem pegada",
        ],
    ),
    (
        PIN_WITHOUT_NUMBER,
        [
            "Symbol pin has no number",
            "Pin de símbolo sin número",
            "Symbolpin ohne Nummer",
            "Broche de symbole sans numéro",
            "Pin del simbolo senza numero",
            "シンボルのピンに番号がない",
            "符号引脚没有编号",
            "Pino do símbolo sem número",
        ],
    ),
    (
        DUPLICATE_PIN_NUMBER,
        [
            "Pin number used twice in a symbol",
            "Número de pin repetido en un símbolo",
            "Pin-Nummer im Symbol doppelt",
            "Numéro de broche répété dans un symbole",
            "Numero di pin ripetuto in un simbolo",
            "シンボル内でピン番号が重複している",
            "符号中引脚编号重复",
            "Número de pino repetido num símbolo",
        ],
    ),
    (
        FLOATING_WIRES,
        [
            "Wires join no pin",
            "Cables que no unen ningún pin",
            "Leitungen ohne Pinverbindung",
            "Fils qui ne relient aucune broche",
            "Fili che non collegano alcun pin",
            "どのピンにもつながらない配線",
            "导线未连接任何引脚",
            "Fios que não ligam nenhum pino",
        ],
    ),
    (
        JUNCTION_ON_FLOATING_WIRES,
        [
            "Junction on wires that join no pin",
            "Unión sobre cables que no conectan ningún pin",
            "Verbindungspunkt auf Leitungen ohne Pinverbindung",
            "Jonction sur des fils qui ne relient aucune broche",
            "Giunzione su fili che non collegano alcun pin",
            "どのピンにもつながらない配線上の接続点",
            "未连接任何引脚的导线上的结点",
            "Junção sobre fios que não ligam nenhum pino",
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
        SETTINGS_INVALID,
        [
            "Settings are not valid",
            "Los ajustes no son válidos",
            "Die Einstellungen sind ungültig",
            "Les réglages ne sont pas valides",
            "Le impostazioni non sono valide",
            "設定が正しくない",
            "设置无效",
            "As definições não são válidas",
        ],
    ),
];

/// The title of a code in a language, or `None` when the code is not in the registry.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    let column = Lang::ALL.iter().position(|l| *l == lang)?;
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[column])
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
            for lang in [Lang::Es, Lang::Pt, Lang::Ja] {
                assert_ne!(title(code, lang), Some(*english), "{code}");
            }
        }
    }

    #[test]
    fn every_rule_has_a_code_in_the_registry_and_a_unique_name() {
        let mut names = Vec::new();
        for (i, rule) in Rule::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(rule.rank()), i + 1);
            assert_eq!(rule.index(), i);
            assert!(REGISTRY.iter().any(|(c, _)| *c == rule.code()));
            assert_eq!(Rule::from_name(rule.name()), Some(rule));
            names.push(rule.name());
        }
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Rule::ALL.len());
        assert_eq!(Rule::from_name("nothing"), None);
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
