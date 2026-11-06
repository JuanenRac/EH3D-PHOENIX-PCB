// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the push tool of the board editor in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the push tool. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShoveMsg {
    /// The box that turns the push tool on.
    Mode,
    /// The label of the net that is chosen.
    NetLabel,
    /// What is said when the board has no net.
    NoNets,
    /// The button that routes the points clicked and pushes the neighbours.
    Apply,
    /// The button that drops the last point clicked.
    RemoveLast,
    /// The button that drops the points clicked.
    Clear,
    /// What follows the number of points clicked.
    Points,
    /// The hint of the push tool.
    Hint,
    /// What follows the number of tracks that the push would move.
    Moves,
    /// What is said when the push would move no track.
    Nothing,
}

const TABLE: [(ShoveMsg, [&str; 8]); ShoveMsg::COUNT] = [
    (
        ShoveMsg::Mode,
        [
            "Push tool",
            "Herramienta Empujar",
            "Verdrängen-Werkzeug",
            "Outil Pousser",
            "Strumento Spingi",
            "押しのけツール",
            "推挤工具",
            "Ferramenta Empurrar",
        ],
    ),
    (
        ShoveMsg::NetLabel,
        [
            "Net",
            "Red",
            "Netz",
            "Réseau",
            "Rete",
            "ネット",
            "网络",
            "Rede",
        ],
    ),
    (
        ShoveMsg::NoNets,
        [
            "The board has no nets",
            "La placa no tiene redes",
            "Die Platine hat keine Netze",
            "La carte n'a aucun réseau",
            "La scheda non ha reti",
            "基板にネットがありません",
            "电路板没有网络",
            "A placa não tem redes",
        ],
    ),
    (
        ShoveMsg::Apply,
        [
            "Route and push",
            "Trazar y empujar",
            "Zeichnen und verdrängen",
            "Tracer et pousser",
            "Traccia e spingi",
            "配線して押しのけ",
            "布线并推挤",
            "Traçar e empurrar",
        ],
    ),
    (
        ShoveMsg::RemoveLast,
        [
            "Remove last point",
            "Quitar el último punto",
            "Letzten Punkt entfernen",
            "Retirer le dernier point",
            "Rimuovi l'ultimo punto",
            "最後の点を削除",
            "删除最后一点",
            "Remover o último ponto",
        ],
    ),
    (
        ShoveMsg::Clear,
        [
            "Clear points",
            "Limpiar puntos",
            "Punkte löschen",
            "Effacer les points",
            "Cancella i punti",
            "点を消去",
            "清除各点",
            "Limpar pontos",
        ],
    ),
    (
        ShoveMsg::Points,
        [
            "points",
            "puntos",
            "Punkte",
            "points",
            "punti",
            "点",
            "点",
            "pontos",
        ],
    ),
    (
        ShoveMsg::Hint,
        [
            "click the points of the new track; the tracks it would push are shown before you accept",
            "pulsa los puntos de la pista nueva; las pistas que empujaría se ven antes de aceptar",
            "Punkte der neuen Leiterbahn anklicken; die verdrängten Leiterbahnen werden vor dem Übernehmen gezeigt",
            "cliquez les points de la nouvelle piste ; les pistes poussées sont montrées avant d'accepter",
            "fai clic sui punti della nuova pista; le piste spostate si vedono prima di accettare",
            "新しい配線の点をクリック。押しのけられる配線は確定前に表示されます",
            "点击新走线的各点；接受之前会显示将被推挤的走线",
            "clique nos pontos da nova pista; as pistas empurradas vêem-se antes de aceitar",
        ],
    ),
    (
        ShoveMsg::Moves,
        [
            "tracks would move",
            "pistas se moverían",
            "Leiterbahnen würden bewegt",
            "pistes seraient déplacées",
            "piste verrebbero spostate",
            "件の配線が移動します",
            "条走线将移动",
            "pistas seriam movidas",
        ],
    ),
    (
        ShoveMsg::Nothing,
        [
            "nothing is in the way",
            "no hay nada en el camino",
            "nichts steht im Weg",
            "rien ne gêne",
            "niente d'intralcio",
            "邪魔なものはありません",
            "没有挡路的走线",
            "não há nada no caminho",
        ],
    ),
];

impl ShoveMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = ShoveMsg::Nothing as usize + 1;

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        TABLE[self as usize].1[column(lang)]
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
        assert_eq!(TABLE.len(), ShoveMsg::COUNT);
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
        assert_eq!(ShoveMsg::Clear.tr(Lang::Pt), "Limpar pontos");
        assert_eq!(ShoveMsg::Mode.tr(Lang::Pt), "Ferramenta Empurrar");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("trilha"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            ShoveMsg::Mode,
            ShoveMsg::Apply,
            ShoveMsg::RemoveLast,
            ShoveMsg::Clear,
            ShoveMsg::Hint,
            ShoveMsg::Moves,
            ShoveMsg::Nothing,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
