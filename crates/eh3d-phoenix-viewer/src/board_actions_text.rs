// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the actions of the Edit menu of the board (tidy, join, snap, widths, swap of pads)
//! and of the swap tool, in the eight languages of the suite: English, Spanish, German, French,
//! Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_boardedit::actions::EditAction;
use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the actions of the board editor. The order of the variants is the order of the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMsg {
    /// Tidies the tracks.
    Tidy,
    /// Joins two tracks.
    Join,
    /// Puts the components on the grid.
    Snap,
    /// Makes the tracks wider.
    Widen,
    /// Makes the tracks narrower.
    Narrow,
    /// The swap tool and its entry of the menu.
    Swap,
    /// What the swap tool tells the person to do.
    HintSwap,
    /// The first pad of the swap was taken.
    SwapFirst,
    /// The pads were swapped.
    SwapDone,
    /// Cuts the track under the pointer.
    Cut,
}

const TABLE: [(ActionMsg, [&str; 8]); ActionMsg::COUNT] = [
    (
        ActionMsg::Tidy,
        [
            "Tidy tracks",
            "Ordenar pistas",
            "Bahnen aufräumen",
            "Ranger les pistes",
            "Riordina le piste",
            "配線を整理",
            "整理走线",
            "Arrumar pistas",
        ],
    ),
    (
        ActionMsg::Join,
        [
            "Join tracks",
            "Unir pistas",
            "Bahnen verbinden",
            "Joindre les pistes",
            "Unisci le piste",
            "配線を結合",
            "合并走线",
            "Unir pistas",
        ],
    ),
    (
        ActionMsg::Snap,
        [
            "Snap to grid",
            "Ajustar a la rejilla",
            "Am Raster ausrichten",
            "Aligner sur la grille",
            "Allinea alla griglia",
            "グリッドに合わせる",
            "对齐到网格",
            "Alinhar à grelha",
        ],
    ),
    (
        ActionMsg::Widen,
        [
            "Wider tracks",
            "Pistas más anchas",
            "Breitere Bahnen",
            "Pistes plus larges",
            "Piste più larghe",
            "配線を太く",
            "加宽走线",
            "Pistas mais largas",
        ],
    ),
    (
        ActionMsg::Narrow,
        [
            "Narrower tracks",
            "Pistas más estrechas",
            "Schmalere Bahnen",
            "Pistes plus étroites",
            "Piste più strette",
            "配線を細く",
            "收窄走线",
            "Pistas mais estreitas",
        ],
    ),
    (
        ActionMsg::Swap,
        [
            "Swap pads",
            "Intercambiar pads",
            "Pads tauschen",
            "Échanger les pads",
            "Scambia i pad",
            "パッドを入れ替え",
            "交换焊盘",
            "Trocar pads",
        ],
    ),
    (
        ActionMsg::HintSwap,
        [
            "Click a pad and then the pad whose net it takes; the swap is one step to undo",
            "Pulse un pad y después el pad cuya red toma; el intercambio es un solo paso al deshacer",
            "Ein Pad anklicken, dann das Pad, dessen Netz es übernimmt; der Tausch ist ein einziger Schritt beim Rückgängigmachen",
            "Cliquez un pad puis le pad dont il prend le réseau ; l'échange est une seule étape à annuler",
            "Clic su un pad e poi sul pad di cui prende la rete; lo scambio è un solo passo da annullare",
            "パッドをクリックし、次にネットを入れ替える相手のパッドをクリック。入れ替えは元に戻す1手順です",
            "先单击一个焊盘，再单击与其交换网络的焊盘；交换在撤销时算一步",
            "Clique num pad e depois no pad cuja rede ele toma; a troca é um só passo ao anular",
        ],
    ),
    (
        ActionMsg::SwapFirst,
        [
            "First pad taken: click the other pad",
            "Primer pad tomado: pulse el otro pad",
            "Erstes Pad gewählt: das andere Pad anklicken",
            "Premier pad pris : cliquez l'autre pad",
            "Primo pad preso: clic sull'altro pad",
            "1つ目のパッドを選択：もう一方のパッドをクリック",
            "已选第一个焊盘：请单击另一个焊盘",
            "Primeiro pad tomado: clique no outro pad",
        ],
    ),
    (
        ActionMsg::SwapDone,
        [
            "Pads swapped",
            "Pads intercambiados",
            "Pads getauscht",
            "Pads échangés",
            "Pad scambiati",
            "パッドを入れ替えました",
            "焊盘已交换",
            "Pads trocados",
        ],
    ),
    (
        ActionMsg::Cut,
        [
            "Cut track here",
            "Cortar la pista aquí",
            "Bahn hier trennen",
            "Couper la piste ici",
            "Taglia la pista qui",
            "ここで配線を切る",
            "在此切断走线",
            "Cortar a pista aqui",
        ],
    ),
];

impl ActionMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = ActionMsg::Cut as usize + 1;

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        TABLE[self as usize].1[column(lang)]
    }

    /// The text of an action of the Edit menu.
    pub fn of(action: EditAction) -> ActionMsg {
        match action {
            EditAction::TidyTracks => ActionMsg::Tidy,
            EditAction::JoinTracks => ActionMsg::Join,
            EditAction::SnapToGrid => ActionMsg::Snap,
            EditAction::WidenTracks => ActionMsg::Widen,
            EditAction::NarrowTracks => ActionMsg::Narrow,
            EditAction::SwapPads => ActionMsg::Swap,
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
        assert_eq!(TABLE.len(), ActionMsg::COUNT);
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
        assert_eq!(ActionMsg::Snap.tr(Lang::Pt), "Alinhar à grelha");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("trilha"), "{msg:?}");
            assert!(!text.contains("grade"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for (msg, _) in TABLE.iter() {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn every_action_of_the_menu_has_its_text() {
        for action in EditAction::ALL {
            assert!(!ActionMsg::of(action).tr(Lang::En).is_empty());
        }
    }
}
