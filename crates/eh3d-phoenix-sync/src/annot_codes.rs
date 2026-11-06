// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-ANNOT-<NNN>` of the annotation back into the schematic and of the
//! comparison of the two documents, the kinds of item behind them and their titles in the eight
//! languages of the suite.

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// A reference designator that differs on the board.
pub const REFDES_FROM_BOARD: &str = "PHOENIX-ANNOT-001";
/// A value that differs on the board.
pub const VALUE_FROM_BOARD: &str = "PHOENIX-ANNOT-002";
/// A footprint that differs on the board.
pub const FOOTPRINT_FROM_BOARD: &str = "PHOENIX-ANNOT-003";
/// A component of the board that has no part in the schematic.
pub const COMPONENT_ADDED: &str = "PHOENIX-ANNOT-004";
/// A part of the schematic that has no component on the board.
pub const PART_REMOVED: &str = "PHOENIX-ANNOT-005";
/// Equivalent pins or gates connected the other way round on the board.
pub const PINS_SWAPPED: &str = "PHOENIX-ANNOT-006";
/// A reference designator that the renumbering by position changes.
pub const RENUMBERED: &str = "PHOENIX-ANNOT-007";
/// A change that clashes with another or that the schematic cannot take.
pub const CLASH: &str = "PHOENIX-ANNOT-008";
/// A change that the settings leave out.
pub const LEFT_OUT: &str = "PHOENIX-ANNOT-009";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-ANNOT-010";
/// A confirmation code that is not the one of the plan.
pub const DIGEST_MISMATCH: &str = "PHOENIX-ANNOT-011";
/// A plan that could not be applied.
pub const NOT_APPLIED: &str = "PHOENIX-ANNOT-012";
/// A part of the schematic with no component on the board (a difference).
pub const DIFF_PART_ONLY: &str = "PHOENIX-ANNOT-013";
/// A component of the board with no part in the schematic (a difference).
pub const DIFF_COMPONENT_ONLY: &str = "PHOENIX-ANNOT-014";
/// A part and its component with different values.
pub const DIFF_VALUE: &str = "PHOENIX-ANNOT-015";
/// A part and its component with different footprints.
pub const DIFF_FOOTPRINT: &str = "PHOENIX-ANNOT-016";
/// A part and its component with different reference designators.
pub const DIFF_REFDES: &str = "PHOENIX-ANNOT-017";
/// A net that only one of the two documents has.
pub const DIFF_NET_ONE_SIDE: &str = "PHOENIX-ANNOT-018";
/// A pin that is on a net in one document and on none in the other.
pub const DIFF_PIN_ONE_SIDE: &str = "PHOENIX-ANNOT-019";
/// A net that holds other pins in each document.
pub const DIFF_NET_PINS: &str = "PHOENIX-ANNOT-020";
/// A reference designator used twice in one document.
pub const CONFLICT_REFDES_TWICE: &str = "PHOENIX-ANNOT-021";
/// A part made into two components.
pub const CONFLICT_PART_TWICE: &str = "PHOENIX-ANNOT-022";
/// A reference designator that an unchanged item of the other document already has.
pub const CONFLICT_REFDES_TAKEN: &str = "PHOENIX-ANNOT-023";

/// The kinds of item of a plan, in the fixed order of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// The reference designator of the board.
    RefdesFromBoard,
    /// A reference designator renumbered by position.
    Renumbered,
    /// The value of the board.
    ValueFromBoard,
    /// The footprint of the board.
    FootprintFromBoard,
    /// Equivalent pins or gates swapped on the board.
    PinsSwapped,
    /// A component with no part.
    ComponentAdded,
    /// A part with no component.
    PartRemoved,
    /// Two components of one part, or another clash that is not a change of a field.
    Clash,
}

impl ItemKind {
    /// The stable code of the kind.
    pub fn code(self) -> &'static str {
        match self {
            ItemKind::RefdesFromBoard => REFDES_FROM_BOARD,
            ItemKind::Renumbered => RENUMBERED,
            ItemKind::ValueFromBoard => VALUE_FROM_BOARD,
            ItemKind::FootprintFromBoard => FOOTPRINT_FROM_BOARD,
            ItemKind::PinsSwapped => PINS_SWAPPED,
            ItemKind::ComponentAdded => COMPONENT_ADDED,
            ItemKind::PartRemoved => PART_REMOVED,
            ItemKind::Clash => CLASH,
        }
    }

    /// The place of the kind in the order of the plan, from 1.
    pub fn rank(self) -> u8 {
        match self {
            ItemKind::RefdesFromBoard => 1,
            ItemKind::Renumbered => 2,
            ItemKind::ValueFromBoard => 3,
            ItemKind::FootprintFromBoard => 4,
            ItemKind::PinsSwapped => 5,
            ItemKind::ComponentAdded => 6,
            ItemKind::PartRemoved => 7,
            ItemKind::Clash => 8,
        }
    }

    /// The stable name of the kind (`refdes_from_board`).
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::RefdesFromBoard => "refdes_from_board",
            ItemKind::Renumbered => "renumbered",
            ItemKind::ValueFromBoard => "value_from_board",
            ItemKind::FootprintFromBoard => "footprint_from_board",
            ItemKind::PinsSwapped => "pins_swapped",
            ItemKind::ComponentAdded => "component_added",
            ItemKind::PartRemoved => "part_removed",
            ItemKind::Clash => "clash",
        }
    }
}

/// The kinds of difference between the two documents, in the fixed order of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    /// A part with no component.
    PartOnly,
    /// A component with no part.
    ComponentOnly,
    /// Different reference designators.
    Refdes,
    /// Different values.
    Value,
    /// Different footprints.
    Footprint,
    /// A net that one document does not have.
    NetOneSide,
    /// A pin on a net in one document only.
    PinOneSide,
    /// A net with other pins in each document.
    NetPins,
    /// A reference designator used twice (a conflict).
    RefdesTwice,
    /// A part made into two components (a conflict).
    PartTwice,
    /// A reference designator held by an unchanged item of the other document (a conflict).
    RefdesTaken,
}

impl DiffKind {
    /// The stable code of the kind.
    pub fn code(self) -> &'static str {
        match self {
            DiffKind::PartOnly => DIFF_PART_ONLY,
            DiffKind::ComponentOnly => DIFF_COMPONENT_ONLY,
            DiffKind::Refdes => DIFF_REFDES,
            DiffKind::Value => DIFF_VALUE,
            DiffKind::Footprint => DIFF_FOOTPRINT,
            DiffKind::NetOneSide => DIFF_NET_ONE_SIDE,
            DiffKind::PinOneSide => DIFF_PIN_ONE_SIDE,
            DiffKind::NetPins => DIFF_NET_PINS,
            DiffKind::RefdesTwice => CONFLICT_REFDES_TWICE,
            DiffKind::PartTwice => CONFLICT_PART_TWICE,
            DiffKind::RefdesTaken => CONFLICT_REFDES_TAKEN,
        }
    }

    /// The place of the kind in the order of the comparison, from 1.
    pub fn rank(self) -> u8 {
        match self {
            DiffKind::PartOnly => 1,
            DiffKind::ComponentOnly => 2,
            DiffKind::Refdes => 3,
            DiffKind::Value => 4,
            DiffKind::Footprint => 5,
            DiffKind::NetOneSide => 6,
            DiffKind::PinOneSide => 7,
            DiffKind::NetPins => 8,
            DiffKind::RefdesTwice => 9,
            DiffKind::PartTwice => 10,
            DiffKind::RefdesTaken => 11,
        }
    }

    /// The stable name of the kind (`value`).
    pub fn name(self) -> &'static str {
        match self {
            DiffKind::PartOnly => "part_only",
            DiffKind::ComponentOnly => "component_only",
            DiffKind::Refdes => "refdes",
            DiffKind::Value => "value",
            DiffKind::Footprint => "footprint",
            DiffKind::NetOneSide => "net_one_side",
            DiffKind::PinOneSide => "pin_one_side",
            DiffKind::NetPins => "net_pins",
            DiffKind::RefdesTwice => "refdes_twice",
            DiffKind::PartTwice => "part_twice",
            DiffKind::RefdesTaken => "refdes_taken",
        }
    }

    /// True for the kinds that are conflicts: what cannot be settled by copying one document into
    /// the other.
    pub fn is_conflict(self) -> bool {
        matches!(
            self,
            DiffKind::RefdesTwice | DiffKind::PartTwice | DiffKind::RefdesTaken
        )
    }
}

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (
        REFDES_FROM_BOARD,
        "Reference designator differs on the board",
    ),
    (VALUE_FROM_BOARD, "Value differs on the board"),
    (FOOTPRINT_FROM_BOARD, "Footprint differs on the board"),
    (COMPONENT_ADDED, "Component on the board has no part"),
    (PART_REMOVED, "Part has no component on the board"),
    (
        PINS_SWAPPED,
        "Equivalent pins or gates swapped on the board",
    ),
    (RENUMBERED, "Reference designator renumbered by position"),
    (CLASH, "Change conflicts with the rest of the plan"),
    (LEFT_OUT, "Change left out by the settings"),
    (SETTINGS_INVALID, "Annotation settings are not valid"),
    (DIGEST_MISMATCH, "Confirmation does not match the plan"),
    (NOT_APPLIED, "Plan could not be applied"),
    (DIFF_PART_ONLY, "Part has no component (schematic only)"),
    (DIFF_COMPONENT_ONLY, "Component has no part (board only)"),
    (DIFF_VALUE, "Values differ"),
    (DIFF_FOOTPRINT, "Footprints differ"),
    (DIFF_REFDES, "Reference designators differ"),
    (DIFF_NET_ONE_SIDE, "Net exists in one document only"),
    (DIFF_PIN_ONE_SIDE, "Pin is on a net in one document only"),
    (DIFF_NET_PINS, "Net holds other pins in each document"),
    (CONFLICT_REFDES_TWICE, "Reference designator used twice"),
    (CONFLICT_PART_TWICE, "Part used by two components"),
    (
        CONFLICT_REFDES_TAKEN,
        "Reference designator taken by another item",
    ),
];

/// The titles in the eight languages, in the order of [`Lang::ALL`].
const TITLES: &[(&str, [&str; 8])] = &[
    (
        REFDES_FROM_BOARD,
        [
            "Reference designator differs on the board",
            "La referencia es distinta en la placa",
            "Die Referenz weicht auf der Platine ab",
            "La référence est différente sur la carte",
            "Il riferimento è diverso sulla scheda",
            "基板上の参照名が異なる",
            "电路板上的位号不同",
            "A referência é diferente na placa",
        ],
    ),
    (
        VALUE_FROM_BOARD,
        [
            "Value differs on the board",
            "El valor es distinto en la placa",
            "Der Wert weicht auf der Platine ab",
            "La valeur est différente sur la carte",
            "Il valore è diverso sulla scheda",
            "基板上の値が異なる",
            "电路板上的值不同",
            "O valor é diferente na placa",
        ],
    ),
    (
        FOOTPRINT_FROM_BOARD,
        [
            "Footprint differs on the board",
            "La huella es distinta en la placa",
            "Der Footprint weicht auf der Platine ab",
            "L'empreinte est différente sur la carte",
            "L'impronta è diversa sulla scheda",
            "基板上のフットプリントが異なる",
            "电路板上的封装不同",
            "A pegada é diferente na placa",
        ],
    ),
    (
        COMPONENT_ADDED,
        [
            "Component on the board has no part",
            "Componente de la placa sin pieza en el esquema",
            "Bauteil auf der Platine ohne Teil im Schaltplan",
            "Composant de la carte sans pièce dans le schéma",
            "Componente della scheda senza parte nello schema",
            "回路図に部品がない基板上の部品",
            "电路板上的元件在原理图中没有对应部件",
            "Componente da placa sem peça no esquema",
        ],
    ),
    (
        PART_REMOVED,
        [
            "Part has no component on the board",
            "Pieza sin componente en la placa",
            "Teil ohne Bauteil auf der Platine",
            "Pièce sans composant sur la carte",
            "Parte senza componente sulla scheda",
            "基板に部品がない回路図の部品",
            "原理图部件在电路板上没有对应元件",
            "Peça sem componente na placa",
        ],
    ),
    (
        PINS_SWAPPED,
        [
            "Equivalent pins or gates swapped on the board",
            "Pines o puertas equivalentes intercambiados en la placa",
            "Gleichwertige Pins oder Gatter auf der Platine vertauscht",
            "Broches ou portes équivalentes échangées sur la carte",
            "Pin o porte equivalenti scambiati sulla scheda",
            "基板上で等価なピンまたはゲートが入れ替えられた",
            "电路板上等效的引脚或门被交换",
            "Pinos ou portas equivalentes trocados na placa",
        ],
    ),
    (
        RENUMBERED,
        [
            "Reference designator renumbered by position",
            "Referencia renumerada por posición",
            "Referenz nach Position neu nummeriert",
            "Référence renumérotée selon la position",
            "Riferimento rinumerato in base alla posizione",
            "位置に基づいて参照名を振り直した",
            "按位置重新编号的位号",
            "Referência renumerada por posição",
        ],
    ),
    (
        CLASH,
        [
            "Change conflicts with the rest of the plan",
            "El cambio choca con el resto del plan",
            "Änderung steht im Konflikt mit dem Rest des Plans",
            "Le changement entre en conflit avec le reste du plan",
            "La modifica è in conflitto con il resto del piano",
            "変更が計画の他の部分と衝突する",
            "该更改与计划的其余部分冲突",
            "A alteração entra em conflito com o resto do plano",
        ],
    ),
    (
        LEFT_OUT,
        [
            "Change left out by the settings",
            "Cambio omitido por los ajustes",
            "Änderung durch die Einstellungen ausgelassen",
            "Changement écarté par les réglages",
            "Modifica esclusa dalle impostazioni",
            "設定により除外された変更",
            "设置已排除该更改",
            "Alteração excluída pelas definições",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Annotation settings are not valid",
            "Los ajustes de la anotación no son válidos",
            "Die Einstellungen der Annotation sind ungültig",
            "Les réglages de l'annotation ne sont pas valides",
            "Le impostazioni dell'annotazione non sono valide",
            "アノテーション設定が正しくない",
            "标注设置无效",
            "As definições da anotação não são válidas",
        ],
    ),
    (
        DIGEST_MISMATCH,
        [
            "Confirmation does not match the plan",
            "La confirmación no coincide con el plan",
            "Die Bestätigung passt nicht zum Plan",
            "La confirmation ne correspond pas au plan",
            "La conferma non corrisponde al piano",
            "確認が計画と一致しない",
            "确认与计划不符",
            "A confirmação não corresponde ao plano",
        ],
    ),
    (
        NOT_APPLIED,
        [
            "Plan could not be applied",
            "No se pudo aplicar el plan",
            "Plan konnte nicht angewendet werden",
            "Le plan n'a pas pu être appliqué",
            "Non è stato possibile applicare il piano",
            "計画を適用できなかった",
            "无法应用该计划",
            "Não foi possível aplicar o plano",
        ],
    ),
    (
        DIFF_PART_ONLY,
        [
            "Part has no component (schematic only)",
            "Pieza sin componente (solo en el esquema)",
            "Teil ohne Bauteil (nur im Schaltplan)",
            "Pièce sans composant (schéma seulement)",
            "Parte senza componente (solo nello schema)",
            "部品に対応する基板部品がない（回路図のみ）",
            "部件没有元件（仅在原理图中）",
            "Peça sem componente (só no esquema)",
        ],
    ),
    (
        DIFF_COMPONENT_ONLY,
        [
            "Component has no part (board only)",
            "Componente sin pieza (solo en la placa)",
            "Bauteil ohne Teil (nur auf der Platine)",
            "Composant sans pièce (carte seulement)",
            "Componente senza parte (solo sulla scheda)",
            "部品に対応する回路図の部品がない（基板のみ）",
            "元件没有部件（仅在电路板上）",
            "Componente sem peça (só na placa)",
        ],
    ),
    (
        DIFF_VALUE,
        [
            "Values differ",
            "Los valores difieren",
            "Werte weichen ab",
            "Les valeurs diffèrent",
            "I valori differiscono",
            "値が異なる",
            "值不同",
            "Os valores diferem",
        ],
    ),
    (
        DIFF_FOOTPRINT,
        [
            "Footprints differ",
            "Las huellas difieren",
            "Footprints weichen ab",
            "Les empreintes diffèrent",
            "Le impronte differiscono",
            "フットプリントが異なる",
            "封装不同",
            "As pegadas diferem",
        ],
    ),
    (
        DIFF_REFDES,
        [
            "Reference designators differ",
            "Las referencias difieren",
            "Referenzen weichen ab",
            "Les références diffèrent",
            "I riferimenti differiscono",
            "参照名が異なる",
            "位号不同",
            "As referências diferem",
        ],
    ),
    (
        DIFF_NET_ONE_SIDE,
        [
            "Net exists in one document only",
            "La red existe en un solo documento",
            "Netz existiert nur in einem Dokument",
            "Le réseau n'existe que dans un document",
            "La rete esiste in un solo documento",
            "片方の文書にしかないネット",
            "网络仅存在于一个文档中",
            "A rede existe só num documento",
        ],
    ),
    (
        DIFF_PIN_ONE_SIDE,
        [
            "Pin is on a net in one document only",
            "El pin tiene red en un solo documento",
            "Pin hat nur in einem Dokument ein Netz",
            "La broche n'a un réseau que dans un document",
            "Il pin ha una rete in un solo documento",
            "片方の文書でしかネットに接続されていないピン",
            "引脚仅在一个文档中连接到网络",
            "O pino tem rede só num documento",
        ],
    ),
    (
        DIFF_NET_PINS,
        [
            "Net holds other pins in each document",
            "La red tiene pines distintos en cada documento",
            "Netz enthält in jedem Dokument andere Pins",
            "Le réseau contient d'autres broches dans chaque document",
            "La rete contiene pin diversi in ogni documento",
            "ネットのピンが文書ごとに異なる",
            "网络在两个文档中包含不同的引脚",
            "A rede tem pinos diferentes em cada documento",
        ],
    ),
    (
        CONFLICT_REFDES_TWICE,
        [
            "Reference designator used twice",
            "Referencia usada dos veces",
            "Referenz doppelt verwendet",
            "Référence utilisée deux fois",
            "Riferimento usato due volte",
            "参照名が重複している",
            "位号重复使用",
            "Referência usada duas vezes",
        ],
    ),
    (
        CONFLICT_PART_TWICE,
        [
            "Part used by two components",
            "Pieza usada por dos componentes",
            "Teil von zwei Bauteilen verwendet",
            "Pièce utilisée par deux composants",
            "Parte usata da due componenti",
            "1つの部品が2つの基板部品で使われている",
            "一个部件被两个元件使用",
            "Peça usada por dois componentes",
        ],
    ),
    (
        CONFLICT_REFDES_TAKEN,
        [
            "Reference designator taken by another item",
            "Referencia ocupada por otro elemento",
            "Referenz von einem anderen Element belegt",
            "Référence prise par un autre élément",
            "Riferimento occupato da un altro elemento",
            "参照名が別の要素に使われている",
            "位号已被其他项占用",
            "Referência ocupada por outro elemento",
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

/// The words of the text report that are not titles, in the eight languages.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "plan",
        [
            "Plan", "Plan", "Plan", "Plan", "Piano", "計画", "计划", "Plano",
        ],
    ),
    (
        "confirm",
        [
            "Confirmation code",
            "Código de confirmación",
            "Bestätigungscode",
            "Code de confirmation",
            "Codice di conferma",
            "確認コード",
            "确认码",
            "Código de confirmação",
        ],
    ),
    (
        "applicable",
        [
            "applicable",
            "aplicable",
            "anwendbar",
            "applicable",
            "applicabile",
            "適用可",
            "可应用",
            "aplicável",
        ],
    ),
    (
        "conflict",
        [
            "conflict",
            "conflicto",
            "Konflikt",
            "conflit",
            "conflitto",
            "競合",
            "冲突",
            "conflito",
        ],
    ),
    (
        "ignored",
        [
            "ignored",
            "ignorado",
            "ignoriert",
            "ignoré",
            "ignorato",
            "無視",
            "已忽略",
            "ignorado",
        ],
    ),
    (
        "differences",
        [
            "Differences",
            "Diferencias",
            "Unterschiede",
            "Différences",
            "Differenze",
            "相違",
            "差异",
            "Diferenças",
        ],
    ),
    (
        "conflicts",
        [
            "Conflicts",
            "Conflictos",
            "Konflikte",
            "Conflits",
            "Conflitti",
            "競合",
            "冲突",
            "Conflitos",
        ],
    ),
    (
        "schematic",
        [
            "schematic",
            "esquema",
            "Schaltplan",
            "schéma",
            "schema",
            "回路図",
            "原理图",
            "esquema",
        ],
    ),
    (
        "board",
        [
            "board",
            "placa",
            "Platine",
            "carte",
            "scheda",
            "基板",
            "电路板",
            "placa",
        ],
    ),
];

/// A word of the text report in a language; an unknown key gives an empty text.
pub fn word(key: &str, lang: Lang) -> &'static str {
    let Some(column) = Lang::ALL.iter().position(|l| *l == lang) else {
        return "";
    };
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, texts)| texts[column])
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
        for (key, texts) in WORDS {
            assert!(texts.iter().all(|t| !t.is_empty()), "{key}");
            assert_eq!(word(key, Lang::En), texts[0]);
        }
        assert_eq!(word("nothing", Lang::En), "");
    }

    #[test]
    fn the_codes_are_numbered_without_gaps() {
        for (i, (code, _)) in REGISTRY.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ANNOT-{:03}", i + 1));
        }
    }

    #[test]
    fn every_kind_has_its_code_in_the_registry_and_a_place_in_the_order() {
        let items = [
            ItemKind::RefdesFromBoard,
            ItemKind::Renumbered,
            ItemKind::ValueFromBoard,
            ItemKind::FootprintFromBoard,
            ItemKind::PinsSwapped,
            ItemKind::ComponentAdded,
            ItemKind::PartRemoved,
            ItemKind::Clash,
        ];
        for (i, kind) in items.into_iter().enumerate() {
            assert_eq!(usize::from(kind.rank()), i + 1);
            assert!(REGISTRY.iter().any(|(c, _)| *c == kind.code()));
            assert!(!kind.name().is_empty());
        }
        let diffs = [
            DiffKind::PartOnly,
            DiffKind::ComponentOnly,
            DiffKind::Refdes,
            DiffKind::Value,
            DiffKind::Footprint,
            DiffKind::NetOneSide,
            DiffKind::PinOneSide,
            DiffKind::NetPins,
            DiffKind::RefdesTwice,
            DiffKind::PartTwice,
            DiffKind::RefdesTaken,
        ];
        for (i, kind) in diffs.into_iter().enumerate() {
            assert_eq!(usize::from(kind.rank()), i + 1);
            assert!(REGISTRY.iter().any(|(c, _)| *c == kind.code()));
            assert!(!kind.name().is_empty());
            assert_eq!(kind.is_conflict(), i >= 8, "{}", kind.name());
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
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
