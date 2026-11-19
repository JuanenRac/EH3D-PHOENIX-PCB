// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the reports of the board editor in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the reports of the board editor. The order of the variants is the order of the rows
/// of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisMsg {
    /// The title of the section of the panel.
    Title,
    /// The button that makes the statistics of the board.
    Statistics,
    /// The button that makes the report of the test points.
    TestPoints,
    /// The button that makes the report of the lengths of the nets.
    Lengths,
    /// The button that makes the bill of materials.
    Bom,
    /// The button that makes the list of the dimensions, and its title.
    Dimensions,
    /// What the list says when the board has no dimension.
    DimensionsNone,
    /// The name of an aligned dimension in the list.
    KindAligned,
    /// The name of a horizontal dimension in the list.
    KindHorizontal,
    /// The name of a vertical dimension in the list.
    KindVertical,
    /// The name of a radius dimension in the list.
    KindRadius,
    /// The name of a diameter dimension in the list.
    KindDiameter,
    /// The name of an angle dimension in the list.
    KindAngle,
    /// The button that closes the report.
    Close,
    /// What is said under the title: the reports are of the board as it is now.
    Hint,
}

const TABLE: [(AnalysisMsg, [&str; 8]); AnalysisMsg::COUNT] = [
    (
        AnalysisMsg::Title,
        [
            "Analysis",
            "Análisis",
            "Analyse",
            "Analyse",
            "Analisi",
            "解析",
            "分析",
            "Análise",
        ],
    ),
    (
        AnalysisMsg::Statistics,
        [
            "Board statistics",
            "Estadísticas de la placa",
            "Platinenstatistik",
            "Statistiques de la carte",
            "Statistiche della scheda",
            "基板の統計",
            "电路板统计",
            "Estatísticas da placa",
        ],
    ),
    (
        AnalysisMsg::TestPoints,
        [
            "Test points",
            "Puntos de prueba",
            "Testpunkte",
            "Points de test",
            "Punti di test",
            "テストポイント",
            "测试点",
            "Pontos de teste",
        ],
    ),
    (
        AnalysisMsg::Lengths,
        [
            "Net lengths",
            "Longitudes de las redes",
            "Netzlängen",
            "Longueurs des réseaux",
            "Lunghezze delle reti",
            "ネット長",
            "网络长度",
            "Comprimentos das redes",
        ],
    ),
    (
        AnalysisMsg::Bom,
        [
            "Bill of materials",
            "Lista de materiales",
            "Stückliste",
            "Nomenclature",
            "Distinta base",
            "部品表",
            "物料清单",
            "Lista de materiais",
        ],
    ),
    (
        AnalysisMsg::Dimensions,
        [
            "Dimensions",
            "Cotas",
            "Bemaßungen",
            "Cotes",
            "Quote",
            "寸法",
            "尺寸标注",
            "Cotas",
        ],
    ),
    (
        AnalysisMsg::DimensionsNone,
        [
            "The board has no dimensions",
            "La placa no tiene cotas",
            "Die Platine hat keine Bemaßungen",
            "La carte n'a pas de cotes",
            "La scheda non ha quote",
            "この基板には寸法がありません",
            "电路板没有尺寸标注",
            "A placa não tem cotas",
        ],
    ),
    (
        AnalysisMsg::KindAligned,
        [
            "Aligned",
            "Alineada",
            "Ausgerichtet",
            "Alignée",
            "Allineata",
            "平行",
            "对齐",
            "Alinhada",
        ],
    ),
    (
        AnalysisMsg::KindHorizontal,
        [
            "Horizontal",
            "Horizontal",
            "Horizontal",
            "Horizontale",
            "Orizzontale",
            "水平",
            "水平",
            "Horizontal",
        ],
    ),
    (
        AnalysisMsg::KindVertical,
        [
            "Vertical",
            "Vertical",
            "Vertikal",
            "Verticale",
            "Verticale",
            "垂直",
            "垂直",
            "Vertical",
        ],
    ),
    (
        AnalysisMsg::KindRadius,
        [
            "Radius", "Radio", "Radius", "Rayon", "Raggio", "半径", "半径", "Raio",
        ],
    ),
    (
        AnalysisMsg::KindDiameter,
        [
            "Diameter",
            "Diámetro",
            "Durchmesser",
            "Diamètre",
            "Diametro",
            "直径",
            "直径",
            "Diâmetro",
        ],
    ),
    (
        AnalysisMsg::KindAngle,
        [
            "Angle", "Ángulo", "Winkel", "Angle", "Angolo", "角度", "角度", "Ângulo",
        ],
    ),
    (
        AnalysisMsg::Close,
        [
            "Close report",
            "Cerrar informe",
            "Bericht schließen",
            "Fermer le rapport",
            "Chiudi rapporto",
            "レポートを閉じる",
            "关闭报告",
            "Fechar relatório",
        ],
    ),
    (
        AnalysisMsg::Hint,
        [
            "Reports of the board as it is now",
            "Informes de la placa tal como está ahora",
            "Berichte zur Platine im aktuellen Zustand",
            "Rapports de la carte dans son état actuel",
            "Rapporti della scheda com'è ora",
            "現在の基板状態のレポート",
            "按当前电路板状态生成的报告",
            "Relatórios da placa no estado atual",
        ],
    ),
];

impl AnalysisMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = AnalysisMsg::Hint as usize + 1;

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
        assert_eq!(TABLE.len(), AnalysisMsg::COUNT);
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
        assert_eq!(AnalysisMsg::TestPoints.tr(Lang::Pt), "Pontos de teste");
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
            AnalysisMsg::Title,
            AnalysisMsg::Statistics,
            AnalysisMsg::TestPoints,
            AnalysisMsg::Lengths,
            AnalysisMsg::Bom,
            AnalysisMsg::Dimensions,
            AnalysisMsg::DimensionsNone,
            AnalysisMsg::Close,
            AnalysisMsg::Hint,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
