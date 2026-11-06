// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the pair tool of the board editor in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the pair tool. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairMsg {
    /// The title of the section of the panel.
    Title,
    /// The box that turns the pair tool on.
    Mode,
    /// The label of the pair that is chosen.
    PairLabel,
    /// What is said when no pair is defined.
    NoPairs,
    /// The button that draws the pair along the points clicked.
    Draw,
    /// The button that makes the two nets of the pair as long as each other.
    Tune,
    /// The button that corrects the pieces whose gap is out of tolerance.
    FixGap,
    /// The button that checks the pairs.
    Check,
    /// The button that drops the points clicked.
    Clear,
    /// What follows the number of points clicked.
    Points,
    /// The hint of the pair tool.
    Hint,
    /// What follows the number of errors of the checks of the pairs.
    Errors,
    /// What follows the number of warnings of the checks of the pairs.
    Warnings,
}

const TABLE: [(PairMsg, [&str; 8]); PairMsg::COUNT] = [
    (
        PairMsg::Title,
        [
            "Differential pair",
            "Par diferencial",
            "Differenzielles Paar",
            "Paire différentielle",
            "Coppia differenziale",
            "差動ペア",
            "差分对",
            "Par diferencial",
        ],
    ),
    (
        PairMsg::Mode,
        [
            "Pair tool",
            "Herramienta de par",
            "Paar-Werkzeug",
            "Outil de paire",
            "Strumento coppia",
            "ペアツール",
            "差分对工具",
            "Ferramenta de par",
        ],
    ),
    (
        PairMsg::PairLabel,
        [
            "Pair",
            "Par",
            "Paar",
            "Paire",
            "Coppia",
            "ペア",
            "差分对",
            "Par",
        ],
    ),
    (
        PairMsg::NoPairs,
        [
            "No pairs are defined (config/diffpairs.cfg)",
            "No hay pares definidos (config/diffpairs.cfg)",
            "Keine Paare definiert (config/diffpairs.cfg)",
            "Aucune paire définie (config/diffpairs.cfg)",
            "Nessuna coppia definita (config/diffpairs.cfg)",
            "ペアが定義されていません (config/diffpairs.cfg)",
            "未定义差分对 (config/diffpairs.cfg)",
            "Não há pares definidos (config/diffpairs.cfg)",
        ],
    ),
    (
        PairMsg::Draw,
        [
            "Draw pair",
            "Trazar par",
            "Paar zeichnen",
            "Tracer la paire",
            "Traccia coppia",
            "ペアを配線",
            "绘制差分对",
            "Traçar par",
        ],
    ),
    (
        PairMsg::Tune,
        [
            "Match lengths",
            "Igualar longitudes",
            "Längen angleichen",
            "Égaliser les longueurs",
            "Uguaglia lunghezze",
            "長さを揃える",
            "等长调整",
            "Igualar comprimentos",
        ],
    ),
    (
        PairMsg::FixGap,
        [
            "Fix gap",
            "Corregir separación",
            "Abstand korrigieren",
            "Corriger l'écart",
            "Correggi distanza",
            "間隔を修正",
            "修正间距",
            "Corrigir separação",
        ],
    ),
    (
        PairMsg::Check,
        [
            "Check pairs",
            "Comprobar pares",
            "Paare prüfen",
            "Vérifier les paires",
            "Verifica coppie",
            "ペアを検査",
            "检查差分对",
            "Verificar pares",
        ],
    ),
    (
        PairMsg::Clear,
        [
            "Clear points",
            "Borrar puntos",
            "Punkte löschen",
            "Effacer les points",
            "Cancella punti",
            "点を消去",
            "清除点",
            "Limpar pontos",
        ],
    ),
    (
        PairMsg::Points,
        [
            "points", "puntos", "Punkte", "points", "punti", "点", "点", "pontos",
        ],
    ),
    (
        PairMsg::Hint,
        [
            "Click the centre line of the pair, then Draw pair",
            "Haga clic en la línea central del par y luego en Trazar par",
            "Mittellinie des Paares anklicken, dann Paar zeichnen",
            "Cliquez sur la ligne centrale de la paire, puis sur Tracer la paire",
            "Fai clic sulla linea centrale della coppia, poi su Traccia coppia",
            "ペアの中心線をクリックし、「ペアを配線」を押します",
            "单击差分对的中心线，然后点击“绘制差分对”",
            "Clique na linha central do par e depois em Traçar par",
        ],
    ),
    (
        PairMsg::Errors,
        [
            "pair errors",
            "errores de pares",
            "Paarfehler",
            "erreurs de paires",
            "errori di coppie",
            "ペアのエラー",
            "差分对错误",
            "erros de pares",
        ],
    ),
    (
        PairMsg::Warnings,
        [
            "pair warnings",
            "avisos de pares",
            "Paarwarnungen",
            "avertissements de paires",
            "avvisi di coppie",
            "ペアの警告",
            "差分对警告",
            "avisos de pares",
        ],
    ),
];

impl PairMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = PairMsg::Warnings as usize + 1;

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
        assert_eq!(TABLE.len(), PairMsg::COUNT);
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
        assert_eq!(PairMsg::Clear.tr(Lang::Pt), "Limpar pontos");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("Salvar"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            PairMsg::Mode,
            PairMsg::Draw,
            PairMsg::Tune,
            PairMsg::FixGap,
            PairMsg::Check,
            PairMsg::Hint,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
