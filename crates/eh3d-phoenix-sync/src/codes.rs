// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-SYNC-<NNN>` of the synchronisation between a schematic and a board,
//! the kinds of change behind them and their titles in the eight languages of the suite.

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// A part of the schematic that no component of the board was made from.
pub const NEW_PART: &str = "PHOENIX-SYNC-001";
/// A component whose part is no longer in the schematic.
pub const MISSING_PART: &str = "PHOENIX-SYNC-002";
/// A part and its component have different reference designators.
pub const REFDES_DIFFERS: &str = "PHOENIX-SYNC-003";
/// A part and its component have different values.
pub const VALUE_DIFFERS: &str = "PHOENIX-SYNC-004";
/// A part and its component have different footprints.
pub const FOOTPRINT_DIFFERS: &str = "PHOENIX-SYNC-005";
/// A net of the schematic that the board does not have.
pub const NET_ADDED: &str = "PHOENIX-SYNC-006";
/// A net of the board that carries another name than the schematic net it stands for.
pub const NET_RENAMED: &str = "PHOENIX-SYNC-007";
/// A pad of the board that is on another net than its pin, or on one when its pin is on none.
pub const PAD_NET_CHANGED: &str = "PHOENIX-SYNC-008";
/// A footprint that is not chosen or that the board does not have, so a change cannot be made.
pub const FOOTPRINT_UNUSABLE: &str = "PHOENIX-SYNC-009";
/// A pin whose pad does not exist in the footprint of the component.
pub const PIN_WITHOUT_PAD: &str = "PHOENIX-SYNC-010";
/// Two components made from one part.
pub const PART_TWICE: &str = "PHOENIX-SYNC-011";
/// A reference designator that would be empty or used twice.
pub const REFDES_CLASH: &str = "PHOENIX-SYNC-012";
/// Two nets of the schematic with one name.
pub const NET_NAME_TWICE: &str = "PHOENIX-SYNC-013";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SYNC-014";
/// A change that could not be applied.
pub const NOT_APPLIED: &str = "PHOENIX-SYNC-015";

/// The kinds of change, in the fixed order of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A part with no component.
    NewPart,
    /// A component with no part.
    MissingPart,
    /// Different reference designators.
    RefdesDiffers,
    /// Different values.
    ValueDiffers,
    /// Different footprints.
    FootprintDiffers,
    /// A net the board does not have.
    NetAdded,
    /// A net with another name.
    NetRenamed,
    /// A pad on another net.
    PadNetChanged,
}

impl ChangeKind {
    /// The stable code of the kind.
    pub fn code(self) -> &'static str {
        match self {
            ChangeKind::NewPart => NEW_PART,
            ChangeKind::MissingPart => MISSING_PART,
            ChangeKind::RefdesDiffers => REFDES_DIFFERS,
            ChangeKind::ValueDiffers => VALUE_DIFFERS,
            ChangeKind::FootprintDiffers => FOOTPRINT_DIFFERS,
            ChangeKind::NetAdded => NET_ADDED,
            ChangeKind::NetRenamed => NET_RENAMED,
            ChangeKind::PadNetChanged => PAD_NET_CHANGED,
        }
    }

    /// The place of the kind in the order of the report, from 1.
    pub fn rank(self) -> u8 {
        match self {
            ChangeKind::NewPart => 1,
            ChangeKind::MissingPart => 2,
            ChangeKind::RefdesDiffers => 3,
            ChangeKind::ValueDiffers => 4,
            ChangeKind::FootprintDiffers => 5,
            ChangeKind::NetAdded => 6,
            ChangeKind::NetRenamed => 7,
            ChangeKind::PadNetChanged => 8,
        }
    }

    /// The stable name of the kind (`new_part`).
    pub fn name(self) -> &'static str {
        match self {
            ChangeKind::NewPart => "new_part",
            ChangeKind::MissingPart => "missing_part",
            ChangeKind::RefdesDiffers => "refdes_differs",
            ChangeKind::ValueDiffers => "value_differs",
            ChangeKind::FootprintDiffers => "footprint_differs",
            ChangeKind::NetAdded => "net_added",
            ChangeKind::NetRenamed => "net_renamed",
            ChangeKind::PadNetChanged => "pad_net_changed",
        }
    }
}

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (NEW_PART, "Part has no component on the board"),
    (MISSING_PART, "Component has no part in the schematic"),
    (REFDES_DIFFERS, "Reference designators differ"),
    (VALUE_DIFFERS, "Values differ"),
    (FOOTPRINT_DIFFERS, "Footprints differ"),
    (NET_ADDED, "Net is new on the board"),
    (NET_RENAMED, "Net of the board is renamed"),
    (PAD_NET_CHANGED, "Pad changes net"),
    (
        FOOTPRINT_UNUSABLE,
        "Footprint is not chosen or not on the board",
    ),
    (PIN_WITHOUT_PAD, "Pin has no pad in the footprint"),
    (PART_TWICE, "Part used by two components"),
    (
        REFDES_CLASH,
        "Reference designator would be empty or used twice",
    ),
    (NET_NAME_TWICE, "Two nets of the schematic share a name"),
    (SETTINGS_INVALID, "Settings are not valid"),
    (NOT_APPLIED, "Change could not be applied"),
];

/// The titles in the eight languages, in the order of [`Lang::ALL`].
const TITLES: &[(&str, [&str; 8])] = &[
    (
        NEW_PART,
        [
            "Part has no component on the board",
            "La pieza no tiene componente en la placa",
            "Bauteil hat keine Entsprechung auf der Platine",
            "La pièce n'a pas de composant sur la carte",
            "Il componente dello schema non è sulla scheda",
            "部品に対応する基板上の部品がない",
            "元件在电路板上没有对应器件",
            "A peça não tem componente na placa",
        ],
    ),
    (
        MISSING_PART,
        [
            "Component has no part in the schematic",
            "El componente no tiene pieza en el esquema",
            "Bauteil der Platine fehlt im Schaltplan",
            "Le composant n'a pas de pièce dans le schéma",
            "Il componente non ha un elemento nello schema",
            "基板の部品に対応する回路図の部品がない",
            "电路板器件在原理图中没有对应元件",
            "O componente não tem peça no esquema",
        ],
    ),
    (
        REFDES_DIFFERS,
        [
            "Reference designators differ",
            "Las referencias son distintas",
            "Bauteilbezeichnungen unterscheiden sich",
            "Les références diffèrent",
            "I riferimenti sono diversi",
            "リファレンスが異なる",
            "位号不同",
            "As referências são diferentes",
        ],
    ),
    (
        VALUE_DIFFERS,
        [
            "Values differ",
            "Los valores son distintos",
            "Werte unterscheiden sich",
            "Les valeurs diffèrent",
            "I valori sono diversi",
            "値が異なる",
            "值不同",
            "Os valores são diferentes",
        ],
    ),
    (
        FOOTPRINT_DIFFERS,
        [
            "Footprints differ",
            "Las huellas son distintas",
            "Footprints unterscheiden sich",
            "Les empreintes diffèrent",
            "Le impronte sono diverse",
            "フットプリントが異なる",
            "封装不同",
            "As pegadas são diferentes",
        ],
    ),
    (
        NET_ADDED,
        [
            "Net is new on the board",
            "La red es nueva en la placa",
            "Netz ist neu auf der Platine",
            "Le réseau est nouveau sur la carte",
            "La rete è nuova sulla scheda",
            "基板に新しいネット",
            "电路板上的新网络",
            "A rede é nova na placa",
        ],
    ),
    (
        NET_RENAMED,
        [
            "Net of the board is renamed",
            "Se renombra una red de la placa",
            "Netz der Platine wird umbenannt",
            "Un réseau de la carte est renommé",
            "Una rete della scheda viene rinominata",
            "基板のネット名を変更",
            "电路板网络被重命名",
            "Uma rede da placa é renomeada",
        ],
    ),
    (
        PAD_NET_CHANGED,
        [
            "Pad changes net",
            "El pad cambia de red",
            "Pad wechselt das Netz",
            "Le pad change de réseau",
            "Il pad cambia rete",
            "パッドのネットが変わる",
            "焊盘更换网络",
            "A pastilha muda de rede",
        ],
    ),
    (
        FOOTPRINT_UNUSABLE,
        [
            "Footprint is not chosen or not on the board",
            "La huella no está elegida o no está en la placa",
            "Footprint nicht gewählt oder nicht auf der Platine",
            "L'empreinte n'est pas choisie ou absente de la carte",
            "L'impronta non è scelta o non è sulla scheda",
            "フットプリントが未選択、または基板にない",
            "未选择封装，或电路板上没有该封装",
            "A pegada não está escolhida ou não existe na placa",
        ],
    ),
    (
        PIN_WITHOUT_PAD,
        [
            "Pin has no pad in the footprint",
            "El pin no tiene pad en la huella",
            "Pin hat kein Pad im Footprint",
            "La broche n'a pas de pad dans l'empreinte",
            "Il pin non ha un pad nell'impronta",
            "ピンに対応するパッドがフットプリントにない",
            "引脚在封装中没有焊盘",
            "O pino não tem pastilha na pegada",
        ],
    ),
    (
        PART_TWICE,
        [
            "Part used by two components",
            "Pieza usada por dos componentes",
            "Bauteil von zwei Platinenbauteilen verwendet",
            "Pièce utilisée par deux composants",
            "Elemento usato da due componenti",
            "1つの部品が2つの基板部品で使われている",
            "一个元件被两个器件使用",
            "Peça usada por dois componentes",
        ],
    ),
    (
        REFDES_CLASH,
        [
            "Reference designator would be empty or used twice",
            "La referencia quedaría vacía o repetida",
            "Bauteilbezeichnung wäre leer oder doppelt",
            "La référence serait vide ou en double",
            "Il riferimento sarebbe vuoto o doppio",
            "リファレンスが空または重複になる",
            "位号将为空或重复",
            "A referência ficaria vazia ou repetida",
        ],
    ),
    (
        NET_NAME_TWICE,
        [
            "Two nets of the schematic share a name",
            "Dos redes del esquema tienen el mismo nombre",
            "Zwei Netze des Schaltplans haben denselben Namen",
            "Deux réseaux du schéma portent le même nom",
            "Due reti dello schema hanno lo stesso nome",
            "回路図の2つのネットが同じ名前",
            "原理图中两个网络同名",
            "Duas redes do esquema têm o mesmo nome",
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
    (
        NOT_APPLIED,
        [
            "Change could not be applied",
            "No se pudo aplicar el cambio",
            "Änderung konnte nicht angewendet werden",
            "Le changement n'a pas pu être appliqué",
            "Non è stato possibile applicare la modifica",
            "変更を適用できなかった",
            "无法应用该更改",
            "Não foi possível aplicar a alteração",
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
    fn every_kind_of_change_has_its_code_in_the_registry_and_a_place_in_the_order() {
        let kinds = [
            ChangeKind::NewPart,
            ChangeKind::MissingPart,
            ChangeKind::RefdesDiffers,
            ChangeKind::ValueDiffers,
            ChangeKind::FootprintDiffers,
            ChangeKind::NetAdded,
            ChangeKind::NetRenamed,
            ChangeKind::PadNetChanged,
        ];
        for (i, kind) in kinds.into_iter().enumerate() {
            assert_eq!(usize::from(kind.rank()), i + 1);
            assert!(REGISTRY.iter().any(|(c, _)| *c == kind.code()));
            assert!(!kind.name().is_empty());
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
