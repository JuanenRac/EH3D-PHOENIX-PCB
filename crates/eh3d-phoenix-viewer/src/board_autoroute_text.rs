// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the autorouter of the board editor in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the autorouter. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutorouteMsg {
    /// The heading of the section of the panel.
    Title,
    /// The button that makes the preview.
    Run,
    /// The button that adds the previewed routes to the board.
    Apply,
    /// The button that drops the preview.
    Discard,
    /// What follows the number of connections that the preview routes.
    Routed,
    /// What follows the number of tracks of the preview.
    Tracks,
    /// What follows the number of vias of the preview.
    Vias,
    /// What follows the number of connections that the preview leaves open.
    Unrouted,
    /// The shortcuts of the tool.
    Hint,
    /// What is said when the board has no open connection.
    Nothing,
    /// What comes before the reason when the autorouter could not run.
    Refused,
}

const TABLE: [(AutorouteMsg, [&str; 8]); AutorouteMsg::COUNT] = [
    (
        AutorouteMsg::Title,
        [
            "Autorouter",
            "Autorruteado",
            "Autorouter",
            "Autorouteur",
            "Autorouter",
            "オートルータ",
            "自动布线",
            "Autorroteamento",
        ],
    ),
    (
        AutorouteMsg::Run,
        [
            "Autoroute",
            "Autorruteado",
            "Automatisch verlegen",
            "Router automatiquement",
            "Instrada automaticamente",
            "自動配線",
            "自动布线",
            "Encaminhar automaticamente",
        ],
    ),
    (
        AutorouteMsg::Apply,
        [
            "Apply routes",
            "Aplicar rutas",
            "Verlegung übernehmen",
            "Appliquer les routes",
            "Applica i percorsi",
            "配線を適用",
            "应用布线",
            "Aplicar rotas",
        ],
    ),
    (
        AutorouteMsg::Discard,
        [
            "Discard preview",
            "Descartar vista previa",
            "Vorschau verwerfen",
            "Annuler l'aperçu",
            "Scarta l'anteprima",
            "プレビューを破棄",
            "放弃预览",
            "Descartar pré-visualização",
        ],
    ),
    (
        AutorouteMsg::Routed,
        [
            "connections routed",
            "conexiones ruteadas",
            "Verbindungen verlegt",
            "connexions routées",
            "connessioni instradate",
            "件の接続を配線",
            "条连接已布线",
            "ligações encaminhadas",
        ],
    ),
    (
        AutorouteMsg::Tracks,
        [
            "tracks",
            "pistas",
            "Leiterbahnen",
            "pistes",
            "piste",
            "本のトラック",
            "条走线",
            "pistas",
        ],
    ),
    (
        AutorouteMsg::Vias,
        [
            "vias",
            "vías",
            "Durchkontaktierungen",
            "vias",
            "via",
            "個のビア",
            "个过孔",
            "vias",
        ],
    ),
    (
        AutorouteMsg::Unrouted,
        [
            "left unrouted",
            "sin rutear",
            "bleiben unverlegt",
            "restent non routées",
            "restano non instradate",
            "件が未配線",
            "条未布线",
            "ficam por encaminhar",
        ],
    ),
    (
        AutorouteMsg::Hint,
        [
            "A previews, Shift+A applies, Esc drops",
            "A previsualiza, Mayús+A aplica, Esc descarta",
            "A zeigt, Umschalt+A übernimmt, Esc verwirft",
            "A prévisualise, Maj+A applique, Échap annule",
            "A anteprima, Maiusc+A applica, Esc scarta",
            "A でプレビュー、Shift+A で適用、Esc で破棄",
            "A 预览，Shift+A 应用，Esc 放弃",
            "A pré-visualiza, Shift+A aplica, Esc descarta",
        ],
    ),
    (
        AutorouteMsg::Nothing,
        [
            "No open connections",
            "No hay conexiones abiertas",
            "Keine offenen Verbindungen",
            "Aucune connexion ouverte",
            "Nessuna connessione aperta",
            "未接続はありません",
            "没有未连接的网络",
            "Não há ligações abertas",
        ],
    ),
    (
        AutorouteMsg::Refused,
        [
            "Could not route",
            "No se pudo rutear",
            "Verlegen nicht möglich",
            "Routage impossible",
            "Instradamento non riuscito",
            "配線できませんでした",
            "无法布线",
            "Não foi possível encaminhar",
        ],
    ),
];

impl AutorouteMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = AutorouteMsg::Refused as usize + 1;

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
        assert_eq!(TABLE.len(), AutorouteMsg::COUNT);
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
        assert_eq!(AutorouteMsg::Run.tr(Lang::Pt), "Encaminhar automaticamente");
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
            AutorouteMsg::Run,
            AutorouteMsg::Apply,
            AutorouteMsg::Discard,
            AutorouteMsg::Routed,
            AutorouteMsg::Unrouted,
            AutorouteMsg::Hint,
            AutorouteMsg::Nothing,
            AutorouteMsg::Refused,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
