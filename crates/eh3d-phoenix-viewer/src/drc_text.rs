// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the panel of problems of a board in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese. The
//! names of the categories and of the severities are the ones of the reports of the checks and of
//! the editing texts; the panel only adds the words that are its own.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;
use crate::drc_panel::DrcSort;

/// A text of the panel of problems. The order of the variants is the order of the rows of the
/// table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrcMsg {
    /// The title of the panel.
    Title,
    /// The label of the categories that can be shown or hidden.
    Categories,
    /// The button that shows every severity and every category.
    ShowAll,
    /// The button that goes to the next problem.
    Next,
    /// The button that goes to the previous problem.
    Previous,
    /// The label of the order of the list.
    Order,
    /// The order that puts the errors first.
    SortSeverity,
    /// The order that groups the problems by category.
    SortCategory,
    /// The order in which the check found the problems.
    SortFound,
    /// What follows the number of problems that pass the filters.
    Shown,
    /// What is said when no problem passes the filters.
    NoMatch,
    /// What is said when no problem shown has a place on the board.
    NoPlace,
}

const TABLE: [(DrcMsg, [&str; 8]); DrcMsg::COUNT] = [
    (
        DrcMsg::Title,
        [
            "Problems",
            "Problemas",
            "Probleme",
            "Problèmes",
            "Problemi",
            "問題",
            "问题",
            "Problemas",
        ],
    ),
    (
        DrcMsg::Categories,
        [
            "Categories",
            "Categorías",
            "Kategorien",
            "Catégories",
            "Categorie",
            "カテゴリ",
            "类别",
            "Categorias",
        ],
    ),
    (
        DrcMsg::ShowAll,
        [
            "Show all",
            "Mostrar todo",
            "Alle anzeigen",
            "Tout afficher",
            "Mostra tutto",
            "すべて表示",
            "全部显示",
            "Mostrar tudo",
        ],
    ),
    (
        DrcMsg::Next,
        [
            "Next problem",
            "Problema siguiente",
            "Nächstes Problem",
            "Problème suivant",
            "Problema successivo",
            "次の問題",
            "下一个问题",
            "Problema seguinte",
        ],
    ),
    (
        DrcMsg::Previous,
        [
            "Previous problem",
            "Problema anterior",
            "Vorheriges Problem",
            "Problème précédent",
            "Problema precedente",
            "前の問題",
            "上一个问题",
            "Problema anterior",
        ],
    ),
    (
        DrcMsg::Order,
        [
            "Order",
            "Orden",
            "Reihenfolge",
            "Ordre",
            "Ordine",
            "並び順",
            "排序",
            "Ordem",
        ],
    ),
    (
        DrcMsg::SortSeverity,
        [
            "By severity",
            "Por gravedad",
            "Nach Schweregrad",
            "Par gravité",
            "Per gravità",
            "重大度順",
            "按严重程度",
            "Por gravidade",
        ],
    ),
    (
        DrcMsg::SortCategory,
        [
            "By category",
            "Por categoría",
            "Nach Kategorie",
            "Par catégorie",
            "Per categoria",
            "カテゴリ順",
            "按类别",
            "Por categoria",
        ],
    ),
    (
        DrcMsg::SortFound,
        [
            "As found",
            "Como se hallaron",
            "Wie gefunden",
            "Dans l'ordre trouvé",
            "Come trovati",
            "検出順",
            "按发现顺序",
            "Pela ordem encontrada",
        ],
    ),
    (
        DrcMsg::Shown,
        [
            "shown",
            "mostrados",
            "angezeigt",
            "affichés",
            "mostrati",
            "表示中",
            "已显示",
            "mostrados",
        ],
    ),
    (
        DrcMsg::NoMatch,
        [
            "No problem passes the filters",
            "Ningún problema cumple los filtros",
            "Kein Problem erfüllt die Filter",
            "Aucun problème ne passe les filtres",
            "Nessun problema supera i filtri",
            "フィルターに一致する問題はありません",
            "没有符合筛选条件的问题",
            "Nenhum problema cumpre os filtros",
        ],
    ),
    (
        DrcMsg::NoPlace,
        [
            "No problem shown has a place on the board",
            "Ningún problema mostrado tiene un lugar en la placa",
            "Kein angezeigtes Problem hat eine Stelle auf der Platine",
            "Aucun problème affiché n'a de position sur la carte",
            "Nessun problema mostrato ha una posizione sulla scheda",
            "表示中の問題に基板上の位置を持つものはありません",
            "显示的问题都没有板上的位置",
            "Nenhum problema mostrado tem um lugar na placa",
        ],
    ),
];

impl DrcMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = DrcMsg::NoPlace as usize + 1;

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        TABLE[self as usize].1[column(lang)]
    }

    /// The text of an order of the list.
    pub fn of_sort(sort: DrcSort) -> DrcMsg {
        match sort {
            DrcSort::Severity => DrcMsg::SortSeverity,
            DrcSort::Category => DrcMsg::SortCategory,
            DrcSort::Found => DrcMsg::SortFound,
        }
    }
}

/// The name of a category of the reports in a language.
pub fn category_name(name: &str, lang: Lang) -> &'static str {
    eh3d_phoenix_rules::codes::word(&format!("cat_{name}"), lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rules::report_export::CATEGORIES;

    #[test]
    fn the_rows_are_in_the_order_of_the_variants() {
        for (i, (msg, _)) in TABLE.iter().enumerate() {
            assert_eq!(*msg as usize, i, "{msg:?}");
        }
        assert_eq!(TABLE.len(), DrcMsg::COUNT);
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
        assert_eq!(DrcMsg::Next.tr(Lang::Pt), "Problema seguinte");
        assert_eq!(DrcMsg::SortFound.tr(Lang::Pt), "Pela ordem encontrada");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("Salvar"), "{msg:?}");
            assert!(!text.contains("Próximo"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            DrcMsg::Categories,
            DrcMsg::ShowAll,
            DrcMsg::Next,
            DrcMsg::Previous,
            DrcMsg::Order,
            DrcMsg::SortSeverity,
            DrcMsg::SortCategory,
            DrcMsg::SortFound,
            DrcMsg::Shown,
            DrcMsg::NoMatch,
            DrcMsg::NoPlace,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn each_order_has_its_text_and_each_category_has_a_name_in_every_language() {
        for sort in DrcSort::ALL {
            let msg = DrcMsg::of_sort(sort);
            assert!(!msg.tr(Lang::En).is_empty());
        }
        assert_eq!(DrcMsg::of_sort(DrcSort::Found), DrcMsg::SortFound);
        for name in CATEGORIES {
            for lang in Lang::ALL {
                assert!(!category_name(name, lang).is_empty(), "{name} {lang:?}");
            }
        }
        assert_eq!(
            category_name("mask_silk", Lang::En),
            "Solder mask and silkscreen"
        );
        assert_eq!(category_name("nothing", Lang::En), "");
    }
}
