// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the 3D window of the assembled board in the eight languages of the suite: English,
//! Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the 3D window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View3dText {
    /// The title of the window.
    Title,
    /// The button of the top bar that opens the 3D window.
    Show3d,
    /// The button that closes the window.
    Close,
    /// The button that returns the camera to its first view.
    Reset,
    /// The hint about the mouse.
    Controls,
    /// The label of the file with the table of models.
    ModelsTable,
    /// The word for the components.
    Parts,
    /// The label of the thickness of the board.
    Thickness,
    /// The label of the collisions.
    Collisions,
    /// The label of the components that are too tall.
    TooTall,
    /// The label of the components outside the outline.
    Outside,
    /// The label of the component that stands highest.
    Tallest,
    /// The title of the selected component.
    Selected,
    /// The label of a reference.
    Reference,
    /// The label of a footprint.
    Footprint,
    /// The label of the side of the board.
    Side,
    /// The word for the top side.
    SideTop,
    /// The word for the bottom side.
    SideBottom,
    /// The label of the height of a component.
    Height,
    /// The label of how far a component reaches out of the board.
    Reach,
    /// The label of the components that a component collides with.
    CollidesWith,
    /// The hint shown while nothing is selected.
    ClickHint,
    /// The word for the board.
    Board,
    /// The title of the colours of the marks.
    Legend,
    /// The note shown when no component has a model.
    NoModels,
    /// The label of the findings of the scene.
    Findings,
}

impl View3dText {
    /// Every text, for the tests that check the table.
    pub const ALL: [View3dText; 26] = [
        View3dText::Title,
        View3dText::Show3d,
        View3dText::Close,
        View3dText::Reset,
        View3dText::Controls,
        View3dText::ModelsTable,
        View3dText::Parts,
        View3dText::Thickness,
        View3dText::Collisions,
        View3dText::TooTall,
        View3dText::Outside,
        View3dText::Tallest,
        View3dText::Selected,
        View3dText::Reference,
        View3dText::Footprint,
        View3dText::Side,
        View3dText::SideTop,
        View3dText::SideBottom,
        View3dText::Height,
        View3dText::Reach,
        View3dText::CollidesWith,
        View3dText::ClickHint,
        View3dText::Board,
        View3dText::Legend,
        View3dText::NoModels,
        View3dText::Findings,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            View3dText::Title => [
                "3D view of the assembly",
                "Vista 3D del ensamblado",
                "3D-Ansicht der Baugruppe",
                "Vue 3D de l'assemblage",
                "Vista 3D dell'assemblaggio",
                "アセンブリの3D表示",
                "装配体的三维视图",
                "Vista 3D da montagem",
            ],
            View3dText::Show3d => [
                "3D view", "Vista 3D", "3D-Ansicht", "Vue 3D", "Vista 3D", "3D表示", "三维视图",
                "Vista 3D",
            ],
            View3dText::Close => [
                "Close", "Cerrar", "Schließen", "Fermer", "Chiudi", "閉じる", "关闭", "Fechar",
            ],
            View3dText::Reset => [
                "Reset the view",
                "Restablecer la vista",
                "Ansicht zurücksetzen",
                "Réinitialiser la vue",
                "Ripristina la vista",
                "表示をリセット",
                "重置视图",
                "Repor a vista",
            ],
            View3dText::Controls => [
                "Drag: rotate · Shift + drag or right button: pan · wheel: zoom · click: select",
                "Arrastrar: girar · Mayús + arrastrar o botón derecho: desplazar · rueda: zoom · clic: seleccionar",
                "Ziehen: drehen · Umschalt + Ziehen oder rechte Taste: verschieben · Rad: Zoom · Klick: auswählen",
                "Glisser : tourner · Maj + glisser ou bouton droit : déplacer · molette : zoom · clic : sélectionner",
                "Trascina: ruota · Maiusc + trascina o tasto destro: sposta · rotella: zoom · clic: seleziona",
                "ドラッグ: 回転 · Shift + ドラッグまたは右ボタン: 移動 · ホイール: ズーム · クリック: 選択",
                "拖动：旋转 · Shift + 拖动或右键：平移 · 滚轮：缩放 · 单击：选择",
                "Arrastar: rodar · Shift + arrastar ou botão direito: deslocar · roda: zoom · clique: selecionar",
            ],
            View3dText::ModelsTable => [
                "Table of models (.csv)",
                "Tabla de modelos (.csv)",
                "Modelltabelle (.csv)",
                "Table des modèles (.csv)",
                "Tabella dei modelli (.csv)",
                "モデル表 (.csv)",
                "模型表 (.csv)",
                "Tabela de modelos (.csv)",
            ],
            View3dText::Parts => [
                "Components",
                "Componentes",
                "Bauteile",
                "Composants",
                "Componenti",
                "部品",
                "元件",
                "Componentes",
            ],
            View3dText::Thickness => [
                "Board thickness (mm)",
                "Grosor de la placa (mm)",
                "Platinendicke (mm)",
                "Épaisseur de la carte (mm)",
                "Spessore della scheda (mm)",
                "基板の厚み (mm)",
                "电路板厚度 (mm)",
                "Espessura da placa (mm)",
            ],
            View3dText::Collisions => [
                "Collisions",
                "Colisiones",
                "Kollisionen",
                "Collisions",
                "Collisioni",
                "干渉",
                "碰撞",
                "Colisões",
            ],
            View3dText::TooTall => [
                "Too tall",
                "Demasiado altos",
                "Zu hoch",
                "Trop hauts",
                "Troppo alti",
                "高さ超過",
                "过高",
                "Demasiado altos",
            ],
            View3dText::Outside => [
                "Outside the outline",
                "Fuera del contorno",
                "Außerhalb des Umrisses",
                "Hors du contour",
                "Fuori dal contorno",
                "外形の外",
                "超出板框",
                "Fora do contorno",
            ],
            View3dText::Tallest => [
                "Highest above the board",
                "El más alto sobre la placa",
                "Höchstes Bauteil über der Platine",
                "Le plus haut au-dessus de la carte",
                "Il più alto sopra la scheda",
                "基板上で最も高い部品",
                "高出电路板最多的元件",
                "O mais alto acima da placa",
            ],
            View3dText::Selected => [
                "Selected",
                "Seleccionado",
                "Ausgewählt",
                "Sélectionné",
                "Selezionato",
                "選択中",
                "已选择",
                "Selecionado",
            ],
            View3dText::Reference => [
                "Reference",
                "Referencia",
                "Referenz",
                "Référence",
                "Riferimento",
                "リファレンス",
                "位号",
                "Referência",
            ],
            View3dText::Footprint => [
                "Footprint",
                "Huella",
                "Footprint",
                "Empreinte",
                "Impronta",
                "フットプリント",
                "封装",
                "Pegada",
            ],
            View3dText::Side => [
                "Side", "Cara", "Seite", "Face", "Lato", "面", "面", "Face",
            ],
            View3dText::SideTop => [
                "top", "superior", "oben", "dessus", "sopra", "表", "顶层", "topo",
            ],
            View3dText::SideBottom => [
                "bottom", "inferior", "unten", "dessous", "sotto", "裏", "底层", "fundo",
            ],
            View3dText::Height => [
                "Height (mm)",
                "Alto (mm)",
                "Höhe (mm)",
                "Hauteur (mm)",
                "Altezza (mm)",
                "高さ (mm)",
                "高度 (mm)",
                "Altura (mm)",
            ],
            View3dText::Reach => [
                "Reach out of the board (mm)",
                "Sobresale de la placa (mm)",
                "Überstand über die Platine (mm)",
                "Dépasse de la carte (mm)",
                "Sporge dalla scheda (mm)",
                "基板からの突出 (mm)",
                "超出电路板 (mm)",
                "Sobressai da placa (mm)",
            ],
            View3dText::CollidesWith => [
                "Collides with",
                "Colisiona con",
                "Kollidiert mit",
                "En collision avec",
                "Collide con",
                "干渉する部品",
                "与以下元件碰撞",
                "Colide com",
            ],
            View3dText::ClickHint => [
                "Click a face to see the component.",
                "Pulse una cara para ver el componente.",
                "Eine Fläche anklicken, um das Bauteil zu sehen.",
                "Cliquez une face pour voir le composant.",
                "Fai clic su una faccia per vedere il componente.",
                "面をクリックすると部品が表示されます。",
                "单击某个面以查看元件。",
                "Clique numa face para ver o componente.",
            ],
            View3dText::Board => [
                "Board", "Placa", "Platine", "Carte", "Scheda", "基板", "电路板", "Placa",
            ],
            View3dText::Legend => [
                "Marks",
                "Marcas",
                "Markierungen",
                "Repères",
                "Segnalazioni",
                "マーク",
                "标记",
                "Marcas",
            ],
            View3dText::NoModels => [
                "No component has a model: only the board is drawn.",
                "Ningún componente tiene modelo: solo se dibuja la placa.",
                "Kein Bauteil hat ein Modell: Nur die Platine wird gezeichnet.",
                "Aucun composant n'a de modèle : seule la carte est dessinée.",
                "Nessun componente ha un modello: viene disegnata solo la scheda.",
                "モデルを持つ部品がないため、基板のみ描画します。",
                "没有元件具有模型：仅绘制电路板。",
                "Nenhum componente tem modelo: só a placa é desenhada.",
            ],
            View3dText::Findings => [
                "Findings of the scene",
                "Avisos de la escena",
                "Hinweise zur Szene",
                "Constats de la scène",
                "Segnalazioni della scena",
                "シーンの指摘事項",
                "场景提示",
                "Avisos da cena",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in View3dText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in View3dText::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert!(View3dText::SideBottom.tr(Lang::Pt).contains("fundo"));
        assert!(View3dText::Controls.tr(Lang::Pt).contains("selecionar"));
        for text in View3dText::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in View3dText::ALL.iter().enumerate() {
            for b in &View3dText::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
