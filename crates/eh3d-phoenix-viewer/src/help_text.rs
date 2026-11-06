// SPDX-License-Identifier: GPL-3.0-or-later
//! The contextual help of the window: one short text for each mode, in the eight languages of the suite
//! (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), shown with F1.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A mode of the window that has a help text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpMode {
    /// The start screen.
    Start,
    /// The schematic window.
    Schematic,
    /// The board window.
    Board,
    /// The library window.
    Library,
    /// The panel window.
    Panel,
    /// The 3D window.
    View3d,
    /// The Gerber window.
    Gerber,
}

impl HelpMode {
    /// Every mode, for the tests that check the table.
    pub const ALL: [HelpMode; 7] = [
        HelpMode::Start,
        HelpMode::Schematic,
        HelpMode::Board,
        HelpMode::Library,
        HelpMode::Panel,
        HelpMode::View3d,
        HelpMode::Gerber,
    ];

    /// The help of the mode in a language: lines separated by a line break.
    pub fn text(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    /// The lines of the help of the mode in a language.
    pub fn lines(self, lang: Lang) -> Vec<&'static str> {
        self.text(lang).lines().collect()
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            HelpMode::Start => [
                "Type or drop a board, a schematic, a library or a folder of Gerber files.\nRecent files are listed below and can be opened with a click.\nNothing that is opened is ever written over.",
                "Escriba o suelte una placa, un esquema, una biblioteca o una carpeta Gerber.\nLos archivos recientes se listan abajo y se abren con un clic.\nNada de lo que se abre se sobrescribe nunca.",
                "Eine Platine, einen Schaltplan, eine Bibliothek oder einen Gerber-Ordner eingeben oder ablegen.\nZuletzt verwendete Dateien stehen unten und öffnen sich mit einem Klick.\nGeöffnete Dateien werden nie überschrieben.",
                "Saisissez ou déposez une carte, un schéma, une bibliothèque ou un dossier Gerber.\nLes fichiers récents sont listés en dessous et s'ouvrent d'un clic.\nRien de ce qui est ouvert n'est jamais écrasé.",
                "Scrivi o trascina una scheda, uno schema, una libreria o una cartella Gerber.\nI file recenti sono elencati sotto e si aprono con un clic.\nCiò che viene aperto non viene mai sovrascritto.",
                "基板、回路図、ライブラリ、または Gerber フォルダーを入力またはドロップします。\n最近のファイルは下に一覧表示され、クリックで開きます。\n開いたファイルが上書きされることはありません。",
                "输入或拖入电路板、原理图、库或 Gerber 文件夹。\n最近的文件列在下方，点击即可打开。\n已打开的文件绝不会被覆盖。",
                "Escreva ou largue uma placa, um esquema, uma biblioteca ou uma pasta Gerber.\nOs ficheiros recentes estão listados abaixo e abrem-se com um clique.\nNada do que se abre é alguma vez substituído.",
            ],
            HelpMode::Schematic => [
                "Click selects a part, a pin or a wire and lights its net; drag pans, the wheel zooms.\nThe measure tool (M) snaps to pins and corners; Ctrl+F searches by reference, value or net.\nEdit schematic opens the tools; the electrical rule check runs after each change.",
                "Un clic selecciona una pieza, un pin o un cable y ilumina su red; arrastrar desplaza, la rueda hace zoom.\nLa medida (M) se ajusta a pines y esquinas; Ctrl+F busca por referencia, valor o red.\nEditar esquema abre las herramientas; la comprobación eléctrica se ejecuta tras cada cambio.",
                "Klick wählt Teil, Pin oder Leitung und hebt das Netz hervor; Ziehen verschiebt, Mausrad zoomt.\nDas Messwerkzeug (M) rastet an Pins und Ecken ein; Strg+F sucht nach Referenz, Wert oder Netz.\nSchaltplan bearbeiten öffnet die Werkzeuge; die elektrische Prüfung läuft nach jeder Änderung.",
                "Un clic sélectionne une pièce, une broche ou un fil et éclaire son net ; glisser déplace, la molette zoome.\nLa mesure (M) s'accroche aux broches et aux coins ; Ctrl+F cherche par référence, valeur ou net.\nModifier le schéma ouvre les outils ; la vérification électrique s'exécute après chaque changement.",
                "Un clic seleziona una parte, un pin o un filo e illumina la sua rete; trascinare sposta, la rotella zooma.\nLa misura (M) si aggancia a pin e angoli; Ctrl+F cerca per riferimento, valore o rete.\nModifica schema apre gli strumenti; il controllo elettrico gira dopo ogni modifica.",
                "クリックで部品・ピン・配線を選択してネットを強調します。ドラッグで移動、ホイールで拡大縮小。\n測定（M）はピンと角にスナップします。Ctrl+F でリファレンス・値・ネットを検索します。\n回路図を編集でツールが開き、変更のたびに電気的ルールチェックが実行されます。",
                "点击选择元件、引脚或导线并点亮其网络；拖动平移，滚轮缩放。\n测量（M）会吸附到引脚和角点；Ctrl+F 按位号、值或网络搜索。\n编辑原理图会打开工具；每次更改后运行电气规则检查。",
                "Um clique seleciona uma peça, um pino ou um fio e acende a sua rede; arrastar desloca, a roda faz zoom.\nA medição (M) ajusta-se a pinos e cantos; Ctrl+F pesquisa por referência, valor ou rede.\nEditar esquema abre as ferramentas; a verificação elétrica corre após cada alteração.",
            ],
            HelpMode::Board => [
                "Click selects an item and lights its net; V, M, G, O, D, E, W, I and T choose the tools.\nCtrl+Z and Ctrl+Y undo and redo; Ctrl+H shows the history; the keys 1 to 9 apply layer presets.\nL shows only the active layer; N lights the net of the selection; F fits the board.",
                "Un clic selecciona un elemento e ilumina su red; V, M, G, O, D, E, W, I y T eligen las herramientas.\nCtrl+Z y Ctrl+Y deshacen y rehacen; Ctrl+H muestra el historial; las teclas 1 a 9 aplican combinaciones de capas.\nL muestra solo la capa activa; N ilumina la red de la selección; F ajusta la placa a la ventana.",
                "Klick wählt ein Element und hebt sein Netz hervor; V, M, G, O, D, E, W, I und T wählen die Werkzeuge.\nStrg+Z und Strg+Y machen rückgängig und wiederholen; Strg+H zeigt den Verlauf; die Tasten 1 bis 9 wenden Ebenen-Vorgaben an.\nL zeigt nur die aktive Ebene; N hebt das Netz der Auswahl hervor; F passt die Platine ein.",
                "Un clic sélectionne un élément et éclaire son net ; V, M, G, O, D, E, W, I et T choisissent les outils.\nCtrl+Z et Ctrl+Y annulent et rétablissent ; Ctrl+H montre l'historique ; les touches 1 à 9 appliquent des préréglages de couches.\nL montre seulement la couche active ; N éclaire le net de la sélection ; F ajuste la carte.",
                "Un clic seleziona un elemento e illumina la sua rete; V, M, G, O, D, E, W, I e T scelgono gli strumenti.\nCtrl+Z e Ctrl+Y annullano e ripetono; Ctrl+H mostra la cronologia; i tasti da 1 a 9 applicano i preset dei livelli.\nL mostra solo il livello attivo; N illumina la rete della selezione; F adatta la scheda.",
                "クリックで要素を選択してネットを強調します。V, M, G, O, D, E, W, I, T でツールを選びます。\nCtrl+Z と Ctrl+Y で取り消し・やり直し、Ctrl+H で履歴、キー 1〜9 でレイヤープリセットを適用します。\nL でアクティブなレイヤーのみ表示、N で選択のネットを強調、F で基板全体を表示します。",
                "点击选择对象并点亮其网络；V、M、G、O、D、E、W、I、T 选择工具。\nCtrl+Z 和 Ctrl+Y 撤销与重做；Ctrl+H 显示历史；按键 1 到 9 应用图层预设。\nL 仅显示活动图层；N 点亮所选网络；F 适合窗口显示整块电路板。",
                "Um clique seleciona um elemento e acende a sua rede; V, M, G, O, D, E, W, I e T escolhem as ferramentas.\nCtrl+Z e Ctrl+Y anulam e repetem; Ctrl+H mostra o histórico; as teclas 1 a 9 aplicam predefinições de camadas.\nL mostra só a camada ativa; N acende a rede da seleção; F ajusta a placa ao ecrã.",
            ],
            HelpMode::Library => [
                "Footprints, symbols and devices of an EAGLE library are edited with the tools and the checks.\nEvery change is a step that can be undone; the library is saved under a new name only.\nThe measure tool shows distances in the unit chosen in the settings.",
                "Las huellas, símbolos y dispositivos de una biblioteca EAGLE se editan con las herramientas y las comprobaciones.\nCada cambio es un paso que se puede deshacer; la biblioteca solo se guarda con un nombre nuevo.\nLa herramienta de medida muestra distancias en la unidad elegida en los ajustes.",
                "Footprints, Symbole und Bauelemente einer EAGLE-Bibliothek werden mit Werkzeugen und Prüfungen bearbeitet.\nJede Änderung ist ein Schritt, der rückgängig gemacht werden kann; die Bibliothek wird nur unter neuem Namen gespeichert.\nDas Messwerkzeug zeigt Abstände in der in den Einstellungen gewählten Einheit.",
                "Les empreintes, symboles et composants d'une bibliothèque EAGLE s'éditent avec les outils et les vérifications.\nChaque modification est une étape annulable ; la bibliothèque n'est enregistrée que sous un nouveau nom.\nL'outil de mesure affiche les distances dans l'unité choisie dans les réglages.",
                "Impronte, simboli e dispositivi di una libreria EAGLE si modificano con gli strumenti e i controlli.\nOgni modifica è un passaggio annullabile; la libreria si salva solo con un nuovo nome.\nLo strumento di misura mostra le distanze nell'unità scelta nelle impostazioni.",
                "EAGLE ライブラリのフットプリント・シンボル・デバイスをツールとチェックで編集します。\nすべての変更は取り消せる手順で、ライブラリは新しい名前でのみ保存されます。\n測定ツールは設定で選んだ単位で距離を表示します。",
                "使用工具和检查来编辑 EAGLE 库中的封装、符号和器件。\n每次更改都是可撤销的步骤；库只能以新名称保存。\n测量工具以设置中选择的单位显示距离。",
                "As pegadas, os símbolos e os dispositivos de uma biblioteca EAGLE editam-se com as ferramentas e as verificações.\nCada alteração é um passo que se pode anular; a biblioteca só se guarda com um nome novo.\nA ferramenta de medição mostra distâncias na unidade escolhida nas definições.",
            ],
            HelpMode::Panel => [
                "The panel shows the copies of the boards, the frame, the tabs, the V-score lines, the fiducials and the holes.\nLayers can be hidden one by one; the area taken by the boards is shown.\nChanging the matrix or the joint lays the panel out again.",
                "El panel muestra las copias de las placas, el marco, las pestañas, las líneas V-score, los fiduciales y los agujeros.\nLas capas se pueden ocultar una a una; se muestra el área que ocupan las placas.\nCambiar la matriz o la unión recoloca el panel.",
                "Der Nutzen zeigt die Kopien der Platinen, den Rahmen, die Stege, die Ritzlinien, die Passmarken und die Löcher.\nEbenen lassen sich einzeln ausblenden; die von den Platinen belegte Fläche wird angezeigt.\nEine Änderung der Matrix oder der Verbindung ordnet den Nutzen neu an.",
                "Le panneau montre les copies des cartes, le cadre, les pontets, les lignes de découpe en V, les fiduciaires et les trous.\nLes couches se masquent une à une ; la surface occupée par les cartes est affichée.\nChanger la matrice ou le joint replace le panneau.",
                "Il pannello mostra le copie delle schede, la cornice, i ponticelli, le linee V-score, i fiduciali e i fori.\nI livelli si nascondono uno a uno; è mostrata l'area occupata dalle schede.\nCambiare la matrice o il giunto ridispone il pannello.",
                "パネルには基板のコピー、フレーム、タブ、V カット線、フィデューシャル、穴が表示されます。\nレイヤーは個別に非表示にでき、基板が占める面積が表示されます。\nマトリックスや接合を変更するとパネルが再配置されます。",
                "拼板显示电路板副本、边框、连接桥、V 形切割线、基准点和孔。\n可逐层隐藏图层；显示电路板占用的面积。\n更改矩阵或连接方式会重新排布拼板。",
                "O painel mostra as cópias das placas, a moldura, as patilhas, as linhas de V-score, os fiduciais e os furos.\nAs camadas podem ser ocultadas uma a uma; é mostrada a área ocupada pelas placas.\nMudar a matriz ou a junção volta a dispor o painel.",
            ],
            HelpMode::View3d => [
                "Drag turns the camera, the right button or the middle button pans, the wheel zooms.\nA click selects a component and shows its height, its reach and what it collides with.\nCollisions, parts that are too tall and parts outside the outline are marked.",
                "Arrastrar gira la cámara, el botón derecho o el central desplaza, la rueda hace zoom.\nUn clic selecciona un componente y muestra su altura, su alcance y con qué choca.\nSe marcan las colisiones, las piezas demasiado altas y las que quedan fuera del contorno.",
                "Ziehen dreht die Kamera, die rechte oder mittlere Taste verschiebt, das Mausrad zoomt.\nEin Klick wählt ein Bauteil und zeigt Höhe, Ausdehnung und Kollisionen.\nKollisionen, zu hohe Teile und Teile außerhalb der Kontur werden markiert.",
                "Glisser fait tourner la caméra, le bouton droit ou central déplace, la molette zoome.\nUn clic sélectionne un composant et montre sa hauteur, sa portée et ses collisions.\nLes collisions, les pièces trop hautes et celles hors du contour sont marquées.",
                "Trascinare ruota la camera, il tasto destro o centrale sposta, la rotella zooma.\nUn clic seleziona un componente e mostra altezza, ingombro e collisioni.\nSono segnate le collisioni, le parti troppo alte e quelle fuori dal contorno.",
                "ドラッグでカメラを回転、右または中ボタンで移動、ホイールで拡大縮小します。\nクリックで部品を選択し、高さ・張り出し・干渉先を表示します。\n干渉、高すぎる部品、外形の外にある部品に印が付きます。",
                "拖动旋转相机，右键或中键平移，滚轮缩放。\n点击选择元件，显示其高度、伸出范围和碰撞对象。\n碰撞、过高的元件和超出轮廓的元件会被标记。",
                "Arrastar roda a câmara, o botão direito ou do meio desloca, a roda faz zoom.\nUm clique seleciona um componente e mostra a sua altura, o seu alcance e com o que colide.\nSão marcadas as colisões, as peças demasiado altas e as que ficam fora do contorno.",
            ],
            HelpMode::Gerber => [
                "Every Gerber and Excellon file of the folder is a layer; layers can be hidden, reordered and made transparent.\nA layer can be compared with the same layer of a board of the project.\nThe measure tool snaps to corners and centres; the files are only read.",
                "Cada archivo Gerber y Excellon de la carpeta es una capa; se pueden ocultar, reordenar y hacer transparentes.\nUna capa se puede comparar con la misma capa de una placa del proyecto.\nLa medida se ajusta a esquinas y centros; los archivos solo se leen.",
                "Jede Gerber- und Excellon-Datei des Ordners ist eine Ebene; sie lassen sich ausblenden, umsortieren und transparent machen.\nEine Ebene lässt sich mit derselben Ebene einer Platine des Projekts vergleichen.\nDas Messwerkzeug rastet an Ecken und Mittelpunkten ein; die Dateien werden nur gelesen.",
                "Chaque fichier Gerber et Excellon du dossier est une couche ; on peut les masquer, les réordonner et les rendre transparentes.\nUne couche peut être comparée à la même couche d'une carte du projet.\nLa mesure s'accroche aux coins et aux centres ; les fichiers sont seulement lus.",
                "Ogni file Gerber ed Excellon della cartella è un livello; si possono nascondere, riordinare e rendere trasparenti.\nUn livello si può confrontare con lo stesso livello di una scheda del progetto.\nLa misura si aggancia a angoli e centri; i file vengono solo letti.",
                "フォルダー内の各 Gerber / Excellon ファイルが 1 つのレイヤーで、非表示・並べ替え・透過ができます。\nレイヤーはプロジェクトの基板の同じレイヤーと比較できます。\n測定は角と中心にスナップします。ファイルは読み取りのみです。",
                "文件夹中的每个 Gerber 和 Excellon 文件是一个图层；可隐藏、重排并设为透明。\n图层可与项目中电路板的同一图层比较。\n测量会吸附到角点和中心；文件仅被读取。",
                "Cada ficheiro Gerber e Excellon da pasta é uma camada; podem ocultar-se, reordenar-se e tornar-se transparentes.\nUma camada pode ser comparada com a mesma camada de uma placa do projeto.\nA medição ajusta-se a cantos e centros; os ficheiros só são lidos.",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_has_three_lines_in_each_language() {
        for mode in HelpMode::ALL {
            for lang in Lang::ALL {
                let lines = mode.lines(lang);
                assert_eq!(lines.len(), 3, "{mode:?} {lang:?}");
                assert!(
                    lines.iter().all(|l| !l.trim().is_empty()),
                    "{mode:?} {lang:?}"
                );
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_help_is_in_their_scripts() {
        for mode in HelpMode::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!mode.text(lang).is_ascii(), "{mode:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        for mode in HelpMode::ALL {
            let pt = mode.text(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar", "Salvar", "deletar"] {
                assert!(!pt.contains(brazilian), "{mode:?}: {pt}");
            }
        }
    }

    #[test]
    fn no_language_repeats_the_english_text() {
        for mode in HelpMode::ALL {
            for lang in &Lang::ALL[1..] {
                assert_ne!(mode.text(*lang), mode.text(Lang::En), "{mode:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_modes_are_listed_once() {
        for (i, a) in HelpMode::ALL.iter().enumerate() {
            for b in &HelpMode::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
