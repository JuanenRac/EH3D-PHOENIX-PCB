// SPDX-License-Identifier: GPL-3.0-or-later
//! The words of the title block and of the titles of the pages in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A word of the title block or of a title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintText {
    /// The name of the project.
    Project,
    /// The sheet, with its number.
    Sheet,
    /// The revision.
    Revision,
    /// The author.
    Author,
    /// The scale.
    Scale,
    /// The size of the paper.
    Paper,
    /// The title of the page.
    Title,
    /// The side of a board.
    Side,
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
    /// A schematic.
    Schematic,
    /// The assembly plan of a board.
    Assembly,
    /// The layers of a board.
    Layers,
    /// A panel of boards.
    Panel,
    /// The first pin of a footprint.
    Pin1,
    /// The copies of a board in a panel.
    Copies,
}

impl PrintText {
    /// Every text, for the tests that check the table.
    pub const ALL: [PrintText; 16] = [
        PrintText::Project,
        PrintText::Sheet,
        PrintText::Revision,
        PrintText::Author,
        PrintText::Scale,
        PrintText::Paper,
        PrintText::Title,
        PrintText::Side,
        PrintText::Top,
        PrintText::Bottom,
        PrintText::Schematic,
        PrintText::Assembly,
        PrintText::Layers,
        PrintText::Panel,
        PrintText::Pin1,
        PrintText::Copies,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            PrintText::Project => [
                "Project",
                "Proyecto",
                "Projekt",
                "Projet",
                "Progetto",
                "プロジェクト",
                "项目",
                "Projeto",
            ],
            PrintText::Sheet => [
                "Sheet",
                "Hoja",
                "Blatt",
                "Feuille",
                "Foglio",
                "シート",
                "图纸",
                "Folha",
            ],
            PrintText::Revision => [
                "Revision",
                "Revisión",
                "Revision",
                "Révision",
                "Revisione",
                "改訂",
                "版本",
                "Revisão",
            ],
            PrintText::Author => [
                "Author",
                "Autor",
                "Autor",
                "Auteur",
                "Autore",
                "作成者",
                "作者",
                "Autor",
            ],
            PrintText::Scale => [
                "Scale", "Escala", "Maßstab", "Échelle", "Scala", "縮尺", "比例", "Escala",
            ],
            PrintText::Paper => [
                "Paper", "Papel", "Papier", "Papier", "Formato", "用紙", "纸张", "Papel",
            ],
            PrintText::Title => [
                "Title",
                "Título",
                "Titel",
                "Titre",
                "Titolo",
                "タイトル",
                "标题",
                "Título",
            ],
            PrintText::Side => ["Side", "Cara", "Seite", "Face", "Lato", "面", "面", "Face"],
            PrintText::Top => [
                "Top", "Superior", "Oben", "Dessus", "Sopra", "表面", "顶层", "Superior",
            ],
            PrintText::Bottom => [
                "Bottom", "Inferior", "Unten", "Dessous", "Sotto", "裏面", "底层", "Inferior",
            ],
            PrintText::Schematic => [
                "Schematic",
                "Esquema",
                "Schaltplan",
                "Schéma",
                "Schema",
                "回路図",
                "原理图",
                "Esquema",
            ],
            PrintText::Assembly => [
                "Assembly plan",
                "Plano de montaje",
                "Bestückungsplan",
                "Plan d'assemblage",
                "Piano di montaggio",
                "実装図",
                "装配图",
                "Plano de montagem",
            ],
            PrintText::Layers => [
                "Layers",
                "Capas",
                "Lagen",
                "Couches",
                "Strati",
                "レイヤー",
                "层",
                "Camadas",
            ],
            PrintText::Panel => [
                "Panel",
                "Panel",
                "Nutzen",
                "Panneau",
                "Pannello",
                "パネル",
                "拼板",
                "Painel",
            ],
            PrintText::Pin1 => [
                "Pin 1",
                "Pin 1",
                "Pin 1",
                "Broche 1",
                "Piedino 1",
                "1番ピン",
                "1号引脚",
                "Pino 1",
            ],
            PrintText::Copies => [
                "Copies",
                "Copias",
                "Kopien",
                "Copies",
                "Copie",
                "コピー数",
                "份数",
                "Cópias",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_each_language() {
        for text in PrintText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_portuguese_of_portugal_is_used() {
        assert_eq!(PrintText::Assembly.tr(Lang::Pt), "Plano de montagem");
        assert_eq!(PrintText::Panel.tr(Lang::Pt), "Painel");
        assert_eq!(PrintText::Sheet.tr(Lang::Es), "Hoja");
        assert_eq!(PrintText::Scale.tr(Lang::De), "Maßstab");
    }
}
