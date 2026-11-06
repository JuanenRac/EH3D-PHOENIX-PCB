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
    /// The button that selects what is not selected and drops what is.
    Invert,
    /// The title of the group of buttons that spread the selection.
    DistributeTitle,
    /// The word before the corners of the box of the selection in the status bar.
    StatusBox,
    /// The word before the width and the height of the box of the selection in the status bar.
    StatusSize,
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
        SelectMsg::Invert,
        [
            "Invert the selection",
            "Invertir la selección",
            "Auswahl umkehren",
            "Inverser la sélection",
            "Inverti la selezione",
            "選択を反転",
            "反选",
            "Inverter a seleção",
        ],
    ),
    (
        SelectMsg::DistributeTitle,
        [
            "Distribute the selection",
            "Distribuir la selección",
            "Auswahl verteilen",
            "Répartir la sélection",
            "Distribuisci la selezione",
            "選択を等間隔に配置",
            "均匀分布所选对象",
            "Distribuir a seleção",
        ],
    ),
    (
        SelectMsg::StatusBox,
        [
            "box",
            "recuadro",
            "Rahmen",
            "cadre",
            "riquadro",
            "範囲",
            "范围",
            "caixa",
        ],
    ),
    (
        SelectMsg::StatusSize,
        [
            "size",
            "tamaño",
            "Größe",
            "taille",
            "dimensioni",
            "寸法",
            "尺寸",
            "tamanho",
        ],
    ),
    (
        SelectMsg::KeysHint,
        [
            "Shift or Ctrl+click toggles, Shift or Ctrl+drag selects inside, Alt+drag also what it touches, a click on a silkscreen item or on the border of a zone picks it, dragging a selected object moves the selection by the grid, N the net, Ctrl+A all, Ctrl+I inverts, Esc clears, arrows move (Shift: 10 steps), Alt+arrows, Alt+C, Alt+M align, Alt+H, Alt+V spread",
            "Mayús o Ctrl+clic alterna, Mayús o Ctrl+arrastrar selecciona lo contenido, Alt+arrastrar también lo tocado, un clic en una serigrafía o en el borde de una zona la coge, arrastrar un objeto seleccionado mueve la selección por la rejilla, N la red, Ctrl+A todo, Ctrl+I invierte, Esc quita, flechas mueven (Mayús: 10 pasos), Alt+flechas, Alt+C, Alt+M alinean, Alt+H, Alt+V distribuyen",
            "Umschalt oder Strg+Klick wechselt, Umschalt oder Strg+Ziehen wählt das Enthaltene, Alt+Ziehen auch das Berührte, ein Klick auf einen Bestückungsdruck oder auf den Rand einer Fläche wählt ihn, das Ziehen eines gewählten Objekts verschiebt die Auswahl im Raster, N das Netz, Strg+A alles, Strg+I kehrt um, Esc hebt auf, Pfeile verschieben (Umschalt: 10 Schritte), Alt+Pfeile, Alt+C, Alt+M richten aus, Alt+H, Alt+V verteilen",
            "Maj ou Ctrl+clic bascule, Maj ou Ctrl+glisser sélectionne l'intérieur, Alt+glisser aussi ce qui est touché, un clic sur un élément de sérigraphie ou sur le bord d'une zone le prend, glisser un objet sélectionné déplace la sélection sur la grille, N le réseau, Ctrl+A tout, Ctrl+I inverse, Échap efface, flèches déplacent (Maj : 10 pas), Alt+flèches, Alt+C, Alt+M alignent, Alt+H, Alt+V répartissent",
            "Maiusc o Ctrl+clic alterna, Maiusc o Ctrl+trascina seleziona il contenuto, Alt+trascina anche ciò che tocca, un clic su un elemento di serigrafia o sul bordo di una zona lo prende, trascinare un oggetto selezionato sposta la selezione sulla griglia, N la rete, Ctrl+A tutto, Ctrl+I inverte, Esc annulla, frecce spostano (Maiusc: 10 passi), Alt+frecce, Alt+C, Alt+M allineano, Alt+H, Alt+V distribuiscono",
            "Shift/Ctrl+クリックで切替、Shift/Ctrl+ドラッグで内側を選択、Alt+ドラッグで接触も選択、シルク要素またはゾーンの縁のクリックでそれを選択、選択したオブジェクトのドラッグで選択全体をグリッド単位で移動、N ネット、Ctrl+A すべて、Ctrl+I 反転、Esc 解除、矢印で移動（Shift: 10 ステップ）、Alt+矢印・Alt+C・Alt+M で整列、Alt+H・Alt+V で等間隔に配置",
            "Shift/Ctrl+单击切换，Shift/Ctrl+拖动选择框内对象，Alt+拖动同时选择接触到的对象，单击丝印项或区域边缘即可选中，拖动已选对象按网格移动整个选择，N 网络，Ctrl+A 全选，Ctrl+I 反选，Esc 取消，方向键移动（Shift：10 步），Alt+方向键、Alt+C、Alt+M 对齐，Alt+H、Alt+V 均匀分布",
            "Shift ou Ctrl+clique alterna, Shift ou Ctrl+arrastar seleciona o que fica dentro, Alt+arrastar também o que toca, um clique num elemento de serigrafia ou no bordo de uma zona seleciona-o, arrastar um objeto selecionado move a seleção pela grelha, N a rede, Ctrl+A tudo, Ctrl+I inverte, Esc limpa, setas movem (Shift: 10 passos), Alt+setas, Alt+C, Alt+M alinham, Alt+H, Alt+V distribuem",
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
    fn the_new_texts_are_not_copies_of_the_english_text() {
        for msg in [
            SelectMsg::Invert,
            SelectMsg::DistributeTitle,
            SelectMsg::StatusBox,
            SelectMsg::StatusSize,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_hint_names_the_new_keys_in_every_language() {
        for lang in Lang::ALL {
            let hint = SelectMsg::KeysHint.tr(lang);
            for key in ["Alt+H", "Alt+V"] {
                assert!(hint.contains(key), "{lang:?} {key}");
            }
            assert!(
                hint.contains("Ctrl+I") || hint.contains("Strg+I"),
                "{lang:?}"
            );
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
