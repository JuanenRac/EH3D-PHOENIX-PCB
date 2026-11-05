// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the window and of the reports of the viewer in the eight languages of the suite:
//! English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;
use crate::scene::Role;

/// A text of the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// The button that opens the file typed in the path box.
    Open,
    /// The button that fits the drawing to the window.
    Fit,
    /// The title of the list of layers.
    Layers,
    /// The label of the language box.
    Language,
    /// The tool that selects an item and highlights its net.
    ToolSelect,
    /// The tool that measures a distance.
    ToolMeasure,
    /// The button that shows every layer.
    ShowAll,
    /// The button that hides every layer.
    HideAll,
    /// The label of the position of the cursor.
    Cursor,
    /// The label of a distance.
    Distance,
    /// The label of the selected item.
    Selected,
    /// The label of a net.
    Net,
    /// The text for an item that has no net.
    NoNet,
    /// The hint shown while nothing is open.
    DropHint,
    /// The hint about the mouse.
    ZoomHint,
    /// The label of the path box.
    PathHint,
    /// The note that the window is a prototype that has never been seen on a screen.
    Prototype,
    /// The word for drawing primitives.
    Primitives,
    /// The word for things that could not be drawn.
    Skipped,
    /// The word for a colour.
    Colour,
    /// The button that clears the selection and the highlight.
    ClearSelection,
    /// The label of an angle.
    Angle,
    /// The hint of the measure tool.
    MeasureHint,
    /// The word for a board.
    Board,
    /// The word for a schematic.
    Schematic,
    /// The word for the items of a document.
    Items,
    /// The word for the nets of a document.
    Nets,
    /// The name of the layer of the reference designators.
    LayerDesignators,
    /// The name of the layer of the outline.
    LayerOutline,
    /// The name of the layer of the drills.
    LayerDrills,
    /// The name of the layer of the sheets.
    LayerSheets,
    /// The name of the layer of the wires.
    LayerWires,
    /// The name of the layer of the symbols.
    LayerSymbols,
    /// The name of the layer of the pins.
    LayerPins,
    /// The name of the layer of the labels.
    LayerLabels,
    /// The title of the report of the benchmark.
    BenchTitle,
    /// What the benchmark measured and what it did not.
    BenchNote,
}

impl Msg {
    /// Every text, for the tests that check the table.
    pub const ALL: [Msg; 37] = [
        Msg::Open,
        Msg::Fit,
        Msg::Layers,
        Msg::Language,
        Msg::ToolSelect,
        Msg::ToolMeasure,
        Msg::ShowAll,
        Msg::HideAll,
        Msg::Cursor,
        Msg::Distance,
        Msg::Selected,
        Msg::Net,
        Msg::NoNet,
        Msg::DropHint,
        Msg::ZoomHint,
        Msg::PathHint,
        Msg::Prototype,
        Msg::Primitives,
        Msg::Skipped,
        Msg::Colour,
        Msg::ClearSelection,
        Msg::Angle,
        Msg::MeasureHint,
        Msg::Board,
        Msg::Schematic,
        Msg::Items,
        Msg::Nets,
        Msg::LayerDesignators,
        Msg::LayerOutline,
        Msg::LayerDrills,
        Msg::LayerSheets,
        Msg::LayerWires,
        Msg::LayerSymbols,
        Msg::LayerPins,
        Msg::LayerLabels,
        Msg::BenchTitle,
        Msg::BenchNote,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    /// The name of a layer that the viewer adds, in the language of the window.
    pub fn for_role(role: Role) -> Option<Msg> {
        match role {
            Role::Designator => Some(Msg::LayerDesignators),
            Role::Outline => Some(Msg::LayerOutline),
            Role::Drill => Some(Msg::LayerDrills),
            Role::Sheet => Some(Msg::LayerSheets),
            Role::Wire => Some(Msg::LayerWires),
            Role::Symbol => Some(Msg::LayerSymbols),
            Role::Pin => Some(Msg::LayerPins),
            Role::Label => Some(Msg::LayerLabels),
            _ => None,
        }
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            Msg::Open => [
                "Open", "Abrir", "Öffnen", "Ouvrir", "Apri", "開く", "打开", "Abrir",
            ],
            Msg::Fit => [
                "Fit to window",
                "Ajustar a la ventana",
                "An Fenster anpassen",
                "Ajuster à la fenêtre",
                "Adatta alla finestra",
                "ウィンドウに合わせる",
                "适应窗口",
                "Ajustar à janela",
            ],
            Msg::Layers => [
                "Layers", "Capas", "Lagen", "Couches", "Livelli", "レイヤー", "层", "Camadas",
            ],
            Msg::Language => [
                "Language", "Idioma", "Sprache", "Langue", "Lingua", "言語", "语言", "Idioma",
            ],
            Msg::ToolSelect => [
                "Select",
                "Seleccionar",
                "Auswählen",
                "Sélectionner",
                "Seleziona",
                "選択",
                "选择",
                "Selecionar",
            ],
            Msg::ToolMeasure => [
                "Measure", "Medir", "Messen", "Mesurer", "Misura", "計測", "测量", "Medir",
            ],
            Msg::ShowAll => [
                "Show all",
                "Mostrar todo",
                "Alles zeigen",
                "Tout afficher",
                "Mostra tutto",
                "すべて表示",
                "全部显示",
                "Mostrar tudo",
            ],
            Msg::HideAll => [
                "Hide all",
                "Ocultar todo",
                "Alles ausblenden",
                "Tout masquer",
                "Nascondi tutto",
                "すべて非表示",
                "全部隐藏",
                "Ocultar tudo",
            ],
            Msg::Cursor => [
                "Cursor", "Cursor", "Cursor", "Curseur", "Cursore", "カーソル", "光标", "Cursor",
            ],
            Msg::Distance => [
                "Distance",
                "Distancia",
                "Abstand",
                "Distance",
                "Distanza",
                "距離",
                "距离",
                "Distância",
            ],
            Msg::Selected => [
                "Selected",
                "Seleccionado",
                "Ausgewählt",
                "Sélectionné",
                "Selezionato",
                "選択中",
                "已选择",
                "Selecionado",
            ],
            Msg::Net => [
                "Net", "Red", "Netz", "Réseau", "Rete", "ネット", "网络", "Rede",
            ],
            Msg::NoNet => [
                "no net",
                "sin red",
                "kein Netz",
                "sans réseau",
                "nessuna rete",
                "ネットなし",
                "无网络",
                "sem rede",
            ],
            Msg::DropHint => [
                "Type a path above, or drop a board (.brd), a schematic (.sch) or a JSON file here",
                "Escriba una ruta arriba o suelte aquí una placa (.brd), un esquema (.sch) o un archivo JSON",
                "Pfad oben eingeben oder eine Platine (.brd), einen Schaltplan (.sch) oder eine JSON-Datei hier ablegen",
                "Saisissez un chemin ci-dessus ou déposez ici une carte (.brd), un schéma (.sch) ou un fichier JSON",
                "Digita un percorso qui sopra o trascina qui una scheda (.brd), uno schema (.sch) o un file JSON",
                "上にパスを入力するか、基板 (.brd)、回路図 (.sch)、JSON ファイルをここにドロップしてください",
                "在上方输入路径，或将电路板 (.brd)、原理图 (.sch) 或 JSON 文件拖放到此处",
                "Escreva um caminho acima ou largue aqui uma placa (.brd), um esquema (.sch) ou um ficheiro JSON",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom · Drag: move · Double click: fit",
                "Rueda: zoom · Arrastrar: mover · Doble clic: ajustar",
                "Rad: Zoom · Ziehen: Verschieben · Doppelklick: anpassen",
                "Molette : zoom · Glisser : déplacer · Double clic : ajuster",
                "Rotella: zoom · Trascina: sposta · Doppio clic: adatta",
                "ホイール：ズーム · ドラッグ：移動 · ダブルクリック：全体表示",
                "滚轮：缩放 · 拖动：移动 · 双击：适应窗口",
                "Roda: zoom · Arrastar: mover · Duplo clique: ajustar",
            ],
            Msg::PathHint => [
                "Path", "Ruta", "Pfad", "Chemin", "Percorso", "パス", "路径", "Caminho",
            ],
            Msg::Prototype => [
                "Prototype: never seen on a screen",
                "Prototipo: nunca visto en una pantalla",
                "Prototyp: nie auf einem Bildschirm gesehen",
                "Prototype : jamais vu sur un écran",
                "Prototipo: mai visto su uno schermo",
                "プロトタイプ：画面で確認されていません",
                "原型：从未在屏幕上看过",
                "Protótipo: nunca visto num ecrã",
            ],
            Msg::Primitives => [
                "primitives",
                "primitivas",
                "Primitive",
                "primitives",
                "primitive",
                "プリミティブ",
                "图元",
                "primitivas",
            ],
            Msg::Skipped => [
                "not drawn",
                "sin dibujar",
                "nicht gezeichnet",
                "non dessinés",
                "non disegnati",
                "未描画",
                "未绘制",
                "não desenhados",
            ],
            Msg::Colour => [
                "Colour", "Color", "Farbe", "Couleur", "Colore", "色", "颜色", "Cor",
            ],
            Msg::ClearSelection => [
                "Clear selection",
                "Quitar la selección",
                "Auswahl aufheben",
                "Effacer la sélection",
                "Deseleziona",
                "選択を解除",
                "清除选择",
                "Limpar a seleção",
            ],
            Msg::Angle => [
                "Angle", "Ángulo", "Winkel", "Angle", "Angolo", "角度", "角度", "Ângulo",
            ],
            Msg::MeasureHint => [
                "Click two points to measure",
                "Haga clic en dos puntos para medir",
                "Zwei Punkte anklicken, um zu messen",
                "Cliquez sur deux points pour mesurer",
                "Fai clic su due punti per misurare",
                "2 点をクリックして計測",
                "单击两个点进行测量",
                "Clique em dois pontos para medir",
            ],
            Msg::Board => [
                "Board", "Placa", "Platine", "Carte", "Scheda", "基板", "电路板", "Placa",
            ],
            Msg::Schematic => [
                "Schematic",
                "Esquema",
                "Schaltplan",
                "Schéma",
                "Schema",
                "回路図",
                "原理图",
                "Esquema",
            ],
            Msg::Items => [
                "Items",
                "Elementos",
                "Elemente",
                "Éléments",
                "Elementi",
                "アイテム",
                "项目",
                "Elementos",
            ],
            Msg::Nets => [
                "Nets", "Redes", "Netze", "Réseaux", "Reti", "ネット", "网络", "Redes",
            ],
            Msg::LayerDesignators => [
                "Designators",
                "Referencias",
                "Bezeichner",
                "Références",
                "Riferimenti",
                "リファレンス",
                "位号",
                "Referências",
            ],
            Msg::LayerOutline => [
                "Outline", "Contorno", "Umriss", "Contour", "Contorno", "外形", "外框", "Contorno",
            ],
            Msg::LayerDrills => [
                "Drills",
                "Taladros",
                "Bohrungen",
                "Perçages",
                "Fori",
                "ドリル穴",
                "钻孔",
                "Furos",
            ],
            Msg::LayerSheets => [
                "Sheets", "Hojas", "Blätter", "Feuilles", "Fogli", "シート", "图纸", "Folhas",
            ],
            Msg::LayerWires => [
                "Wires", "Cables", "Leitungen", "Fils", "Fili", "配線", "导线", "Fios",
            ],
            Msg::LayerSymbols => [
                "Symbols",
                "Símbolos",
                "Symbole",
                "Symboles",
                "Simboli",
                "シンボル",
                "符号",
                "Símbolos",
            ],
            Msg::LayerPins => [
                "Pins", "Pines", "Pins", "Broches", "Pin", "ピン", "引脚", "Pinos",
            ],
            Msg::LayerLabels => [
                "Labels",
                "Etiquetas",
                "Beschriftungen",
                "Étiquettes",
                "Etichette",
                "ラベル",
                "标签",
                "Etiquetas",
            ],
            Msg::BenchTitle => [
                "Benchmark with synthetic boards",
                "Prueba de rendimiento con placas sintéticas",
                "Leistungstest mit synthetischen Platinen",
                "Test de performance avec des cartes synthétiques",
                "Benchmark con schede sintetiche",
                "合成基板によるベンチマーク",
                "使用合成电路板的基准测试",
                "Teste de desempenho com placas sintéticas",
            ],
            Msg::BenchNote => [
                "These times are of the preparation of the primitives and of the selection by position on this computer. Drawing on a screen was not measured: it needs a window and a graphics card.",
                "Estos tiempos son de la preparación de las primitivas y de la selección por posición en este ordenador. El dibujo en pantalla no se ha medido: necesita una ventana y una tarjeta gráfica.",
                "Diese Zeiten betreffen die Vorbereitung der Primitive und die Auswahl nach Position auf diesem Rechner. Das Zeichnen auf dem Bildschirm wurde nicht gemessen: Es braucht ein Fenster und eine Grafikkarte.",
                "Ces temps concernent la préparation des primitives et la sélection par position sur cet ordinateur. Le dessin à l'écran n'a pas été mesuré : il faut une fenêtre et une carte graphique.",
                "Questi tempi riguardano la preparazione delle primitive e la selezione per posizione su questo computer. Il disegno sullo schermo non è stato misurato: servono una finestra e una scheda grafica.",
                "この時間は、このコンピューターでのプリミティブの準備と位置による選択のものです。画面への描画は測定していません。ウィンドウとグラフィックスカードが必要です。",
                "这些时间是在这台计算机上准备图元和按位置选择所用的时间。屏幕绘制未测量：它需要窗口和显卡。",
                "Estes tempos são da preparação das primitivas e da seleção por posição neste computador. O desenho no ecrã não foi medido: precisa de uma janela e de uma placa gráfica.",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for msg in Msg::ALL {
            for lang in Lang::ALL {
                assert!(!msg.tr(lang).is_empty(), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for msg in Msg::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!msg.tr(lang).is_ascii(), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert_eq!(Msg::DropHint.tr(Lang::Pt).matches("ficheiro").count(), 1);
        assert!(Msg::BenchNote.tr(Lang::Pt).contains("ecrã"));
        for msg in Msg::ALL {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
        }
    }

    #[test]
    fn the_layers_that_the_viewer_adds_have_a_text_and_the_others_do_not() {
        for role in [
            Role::Designator,
            Role::Outline,
            Role::Drill,
            Role::Sheet,
            Role::Wire,
            Role::Symbol,
            Role::Pin,
            Role::Label,
        ] {
            assert!(role.is_synthetic());
            assert!(Msg::for_role(role).is_some());
        }
        for role in [
            Role::TopCopper,
            Role::BottomCopper,
            Role::Mask,
            Role::SilkTop,
        ] {
            assert!(!role.is_synthetic());
            assert!(Msg::for_role(role).is_none());
        }
    }

    #[test]
    fn the_hints_name_the_gestures_in_each_language() {
        assert!(Msg::ZoomHint.tr(Lang::En).contains("Wheel"));
        assert!(Msg::ZoomHint.tr(Lang::Es).contains("Rueda"));
        assert!(Msg::MeasureHint.tr(Lang::De).contains("messen"));
    }
}
