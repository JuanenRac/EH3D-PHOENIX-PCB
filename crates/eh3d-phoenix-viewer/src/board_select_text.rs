// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the selection of several objects in the window of the board editor, in the eight
//! languages of the suite: English, Spanish, German, French, Italian, Japanese, Chinese and
//! European Portuguese.

use eh3d_phoenix_boardedit::AlignMode;
use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the selection of several objects. The order of the variants is the order of the rows
/// of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectMsg {
    /// What follows the number of objects in the status bar.
    StatusSelected,
    /// The title of the group of buttons that line the selection up.
    AlignTitle,
    /// Line up on the left edge.
    AlignLeft,
    /// Line up on the right edge.
    AlignRight,
    /// Line up on the top edge.
    AlignTop,
    /// Line up on the bottom edge.
    AlignBottom,
    /// Centre along x.
    AlignCenterX,
    /// Centre along y.
    AlignCenterY,
    /// Equal gaps along x.
    DistributeX,
    /// Equal gaps along y.
    DistributeY,
    /// The button that selects every object.
    SelectAll,
    /// The button that selects the objects of the net that is lit.
    SelectNet,
    /// The button that drops the selection.
    ClearSelection,
    /// The keys of the selection of several objects.
    KeysHint,
}

const TABLE: [(SelectMsg, [&str; 8]); SelectMsg::COUNT] = [
    (
        SelectMsg::StatusSelected,
        [
            "selected",
            "seleccionados",
            "ausgewählt",
            "sélectionnés",
            "selezionati",
            "個選択中",
            "个已选",
            "selecionados",
        ],
    ),
    (
        SelectMsg::AlignTitle,
        [
            "Align the selection",
            "Alinear la selección",
            "Auswahl ausrichten",
            "Aligner la sélection",
            "Allinea la selezione",
            "選択を整列",
            "对齐所选对象",
            "Alinhar a seleção",
        ],
    ),
    (
        SelectMsg::AlignLeft,
        [
            "Left",
            "Izquierda",
            "Links",
            "Gauche",
            "Sinistra",
            "左揃え",
            "左对齐",
            "Esquerda",
        ],
    ),
    (
        SelectMsg::AlignRight,
        [
            "Right",
            "Derecha",
            "Rechts",
            "Droite",
            "Destra",
            "右揃え",
            "右对齐",
            "Direita",
        ],
    ),
    (
        SelectMsg::AlignTop,
        [
            "Top",
            "Arriba",
            "Oben",
            "Haut",
            "Alto",
            "上揃え",
            "顶部对齐",
            "Topo",
        ],
    ),
    (
        SelectMsg::AlignBottom,
        [
            "Bottom",
            "Abajo",
            "Unten",
            "Bas",
            "Basso",
            "下揃え",
            "底部对齐",
            "Fundo",
        ],
    ),
    (
        SelectMsg::AlignCenterX,
        [
            "Centre across",
            "Centrar en horizontal",
            "Horizontal zentrieren",
            "Centrer à l'horizontale",
            "Centra in orizzontale",
            "水平中央揃え",
            "水平居中",
            "Centrar na horizontal",
        ],
    ),
    (
        SelectMsg::AlignCenterY,
        [
            "Centre down",
            "Centrar en vertical",
            "Vertikal zentrieren",
            "Centrer à la verticale",
            "Centra in verticale",
            "垂直中央揃え",
            "垂直居中",
            "Centrar na vertical",
        ],
    ),
    (
        SelectMsg::DistributeX,
        [
            "Spread across",
            "Distribuir en horizontal",
            "Horizontal verteilen",
            "Répartir à l'horizontale",
            "Distribuisci in orizzontale",
            "水平方向に等間隔",
            "水平等间距",
            "Distribuir na horizontal",
        ],
    ),
    (
        SelectMsg::DistributeY,
        [
            "Spread down",
            "Distribuir en vertical",
            "Vertikal verteilen",
            "Répartir à la verticale",
            "Distribuisci in verticale",
            "垂直方向に等間隔",
            "垂直等间距",
            "Distribuir na vertical",
        ],
    ),
    (
        SelectMsg::SelectAll,
        [
            "Select all",
            "Seleccionar todo",
            "Alles auswählen",
            "Tout sélectionner",
            "Seleziona tutto",
            "すべて選択",
            "全选",
            "Selecionar tudo",
        ],
    ),
    (
        SelectMsg::SelectNet,
        [
            "Select the net",
            "Seleccionar la red",
            "Netz auswählen",
            "Sélectionner le réseau",
            "Seleziona la rete",
            "ネットを選択",
            "选择网络",
            "Selecionar a rede",
        ],
    ),
    (
        SelectMsg::ClearSelection,
        [
            "Clear the selection",
            "Quitar la selección",
            "Auswahl aufheben",
            "Effacer la sélection",
            "Annulla la selezione",
            "選択を解除",
            "取消选择",
            "Limpar a seleção",
        ],
    ),
    (
        SelectMsg::KeysHint,
        [
            "Shift or Ctrl+click toggles, Shift or Ctrl+drag selects inside, Alt+drag also what it touches, N the net, Ctrl+A all, Esc clears, arrows move (Shift: 10 steps), Alt+arrows, Alt+C, Alt+M align",
            "Mayús o Ctrl+clic alterna, Mayús o Ctrl+arrastrar selecciona lo contenido, Alt+arrastrar también lo tocado, N la red, Ctrl+A todo, Esc quita, flechas mueven (Mayús: 10 pasos), Alt+flechas, Alt+C, Alt+M alinean",
            "Umschalt oder Strg+Klick wechselt, Umschalt oder Strg+Ziehen wählt das Enthaltene, Alt+Ziehen auch das Berührte, N das Netz, Strg+A alles, Esc hebt auf, Pfeile verschieben (Umschalt: 10 Schritte), Alt+Pfeile, Alt+C, Alt+M richten aus",
            "Maj ou Ctrl+clic bascule, Maj ou Ctrl+glisser sélectionne l'intérieur, Alt+glisser aussi ce qui est touché, N le réseau, Ctrl+A tout, Échap efface, flèches déplacent (Maj : 10 pas), Alt+flèches, Alt+C, Alt+M alignent",
            "Maiusc o Ctrl+clic alterna, Maiusc o Ctrl+trascina seleziona il contenuto, Alt+trascina anche ciò che tocca, N la rete, Ctrl+A tutto, Esc annulla, frecce spostano (Maiusc: 10 passi), Alt+frecce, Alt+C, Alt+M allineano",
            "Shift/Ctrl+クリックで切替、Shift/Ctrl+ドラッグで内側を選択、Alt+ドラッグで接触も選択、N ネット、Ctrl+A すべて、Esc 解除、矢印で移動（Shift: 10 ステップ）、Alt+矢印・Alt+C・Alt+M で整列",
            "Shift/Ctrl+单击切换，Shift/Ctrl+拖动选择框内对象，Alt+拖动同时选择接触到的对象，N 网络，Ctrl+A 全选，Esc 取消，方向键移动（Shift：10 步），Alt+方向键、Alt+C、Alt+M 对齐",
            "Shift ou Ctrl+clique alterna, Shift ou Ctrl+arrastar seleciona o que fica dentro, Alt+arrastar também o que toca, N a rede, Ctrl+A tudo, Esc limpa, setas movem (Shift: 10 passos), Alt+setas, Alt+C, Alt+M alinham",
        ],
    ),
];

impl SelectMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = SelectMsg::KeysHint as usize + 1;

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        TABLE[self as usize].1[column(lang)]
    }

    /// The text of the button of an alignment.
    pub fn of_align(mode: AlignMode) -> SelectMsg {
        match mode {
            AlignMode::Left => SelectMsg::AlignLeft,
            AlignMode::Right => SelectMsg::AlignRight,
            AlignMode::Top => SelectMsg::AlignTop,
            AlignMode::Bottom => SelectMsg::AlignBottom,
            AlignMode::CenterX => SelectMsg::AlignCenterX,
            AlignMode::CenterY => SelectMsg::AlignCenterY,
            AlignMode::DistributeX => SelectMsg::DistributeX,
            AlignMode::DistributeY => SelectMsg::DistributeY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_are_in_the_order_of_the_variants() {
        for (i, (msg, _)) in TABLE.iter().enumerate() {
            assert_eq!(*msg as usize, i, "{msg:?}");
        }
        assert_eq!(TABLE.len(), SelectMsg::COUNT);
    }

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for (msg, row) in TABLE.iter() {
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{msg:?} column {i}");
            }
            for lang in Lang::ALL {
                assert_eq!(msg.tr(lang), row[column(lang)]);
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for (msg, _) in TABLE.iter() {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!msg.tr(lang).is_ascii(), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert_eq!(SelectMsg::AlignLeft.tr(Lang::Pt), "Esquerda");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("deletar"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            SelectMsg::AlignTitle,
            SelectMsg::SelectAll,
            SelectMsg::ClearSelection,
            SelectMsg::KeysHint,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn every_alignment_has_its_own_text() {
        let modes = [
            AlignMode::Left,
            AlignMode::Right,
            AlignMode::Top,
            AlignMode::Bottom,
            AlignMode::CenterX,
            AlignMode::CenterY,
            AlignMode::DistributeX,
            AlignMode::DistributeY,
        ];
        let mut seen = Vec::new();
        for mode in modes {
            let msg = SelectMsg::of_align(mode);
            assert!(!seen.contains(&(msg as usize)), "{mode:?}");
            seen.push(msg as usize);
        }
    }
}
