// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the library mode of the window in the eight languages of the suite: English,
//! Spanish, German, French, Italian, Japanese, Chinese and European Portuguese. The titles of the
//! codes `PHOENIX-LIB-<NNN>` have their own table in the library editor.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the library mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibText {
    /// The title of the library mode.
    Title,
    /// The word for footprints.
    Footprints,
    /// The word for symbols.
    Symbols,
    /// The word for device sets.
    Devices,
    /// The word for one footprint.
    Footprint,
    /// The word for one symbol.
    Symbol,
    /// The button that makes a new item.
    New,
    /// The button that renames.
    Rename,
    /// The button that deletes.
    Delete,
    /// The label of a name.
    Name,
    /// The label of the prefix of the references.
    Prefix,
    /// The tool that selects and moves.
    ToolSelect,
    /// The tool that places a surface-mount pad.
    ToolSmd,
    /// The tool that places a through-hole pad.
    ToolPad,
    /// The tool that draws a line.
    ToolLine,
    /// The tool that draws an arc.
    ToolArc,
    /// The tool that draws a circle.
    ToolCircle,
    /// The tool that places a text.
    ToolText,
    /// The tool that places a hole.
    ToolHole,
    /// The tool that places a pin.
    ToolPin,
    /// The tool that measures.
    ToolMeasure,
    /// The label of the grid.
    Grid,
    /// The box that snaps to the grid.
    Snap,
    /// The title of the properties.
    Properties,
    /// The button that applies the properties.
    Apply,
    /// The word for the origin.
    Origin,
    /// The button that moves the origin to the cursor.
    OriginToCursor,
    /// The label of a width.
    Width,
    /// The label of a height.
    Height,
    /// The label of a drill.
    Drill,
    /// The label of a diameter.
    Diameter,
    /// The label of a radius.
    Radius,
    /// The label of a layer.
    Layer,
    /// The label of a shape.
    Shape,
    /// The label of a rotation.
    Rotation,
    /// The label of the roundness of a pad.
    Roundness,
    /// The label of the width of a line.
    LineWidth,
    /// The label of the angle of an arc.
    ArcAngle,
    /// The label of the height of a text.
    TextHeight,
    /// The label of the electrical type of a pin.
    ElectricalType,
    /// The label of the length of a pin.
    PinLength,
    /// The label of the opening of the solder mask.
    MaskOpening,
    /// The label of the opening of the solder paste.
    PasteOpening,
    /// The label of an expansion.
    Expansion,
    /// The word for automatic.
    Automatic,
    /// The button that sets an opening.
    Set,
    /// The word for gates.
    Gates,
    /// The word for the variants of a device set.
    Variants,
    /// The button that adds a gate.
    AddGate,
    /// The button that adds a variant.
    AddVariant,
    /// The button that connects the pins to the pads of the same name.
    AutoConnect,
    /// The title of the connections of pins to pads.
    PinToPad,
    /// The word for no pad.
    NoPad,
    /// The title of the checks of the library.
    Findings,
    /// The button that checks the library.
    RunCheck,
    /// The text when the check finds nothing.
    NoFindings,
    /// The label of the path of the new file.
    SavePath,
    /// The button that saves as a new file.
    SaveAsNew,
    /// The mark of a library with changes that are not saved.
    Unsaved,
    /// The title of the replacement by the master.
    Plan,
    /// The label of the plan file.
    PlanFile,
    /// The label of the folder of the master libraries.
    MasterFolder,
    /// The button that loads the plan.
    LoadPlan,
    /// The box that confirms a replacement.
    Confirm,
    /// The button that replaces.
    Replace,
    /// The note that nothing is replaced without confirmation.
    PlanNotApplied,
    /// The title of the footprint generator.
    Generator,
    /// The label of the family of a footprint.
    Family,
    /// The label of the parameters of a generator.
    Parameters,
    /// The button that adds the generated footprint.
    AddToLibrary,
    /// The note that the library editor is a prototype.
    Prototype,
    /// The hint of the keys.
    Shortcuts,
    /// A library was opened.
    NoteOpened,
    /// A library was saved.
    NoteSaved,
    /// A copy of the original was kept.
    NoteBackup,
    /// There is nothing to undo.
    NoteNothingUndo,
    /// There is nothing to redo.
    NoteNothingRedo,
    /// A step was undone.
    NoteUndone,
    /// A step was redone.
    NoteRedone,
    /// An edit was made.
    NoteDone,
    /// The tool waits for the second point.
    NoteSecondPoint,
    /// The tool is not for this drawing.
    NoteWrongKind,
    /// No footprint or symbol is chosen.
    NoteNoEntity,
    /// Nothing is selected.
    NoteNoSelection,
    /// The item cannot be turned.
    NoteCannotRotate,
    /// The check was made.
    NoteChecked,
    /// A plan was loaded.
    NotePlanLoaded,
    /// A footprint was added.
    NoteGenerated,
}

/// Every text with its eight translations, in the order of the variants.
const TABLE: [(LibText, [&str; 8]); 88] = [
    (
        LibText::Title,
        [
            "Library editor",
            "Editor de bibliotecas",
            "Bibliothekseditor",
            "Éditeur de bibliothèques",
            "Editor di librerie",
            "ライブラリエディタ",
            "库编辑器",
            "Editor de bibliotecas",
        ],
    ),
    (
        LibText::Footprints,
        [
            "Footprints",
            "Huellas",
            "Bauformen",
            "Empreintes",
            "Impronte",
            "フットプリント",
            "封装",
            "Pegadas",
        ],
    ),
    (
        LibText::Symbols,
        [
            "Symbols",
            "Símbolos",
            "Symbole",
            "Symboles",
            "Simboli",
            "シンボル",
            "符号",
            "Símbolos",
        ],
    ),
    (
        LibText::Devices,
        [
            "Devices",
            "Dispositivos",
            "Bauteile",
            "Composants",
            "Dispositivi",
            "デバイス",
            "器件",
            "Dispositivos",
        ],
    ),
    (
        LibText::Footprint,
        [
            "Footprint",
            "Huella",
            "Bauform",
            "Empreinte",
            "Impronta",
            "フットプリント",
            "封装",
            "Pegada",
        ],
    ),
    (
        LibText::Symbol,
        [
            "Symbol",
            "Símbolo",
            "Symbol",
            "Symbole",
            "Simbolo",
            "シンボル",
            "符号",
            "Símbolo",
        ],
    ),
    (
        LibText::New,
        [
            "New",
            "Nuevo",
            "Neu",
            "Nouveau",
            "Nuovo",
            "新規",
            "新建",
            "Novo",
        ],
    ),
    (
        LibText::Rename,
        [
            "Rename",
            "Renombrar",
            "Umbenennen",
            "Renommer",
            "Rinomina",
            "名前変更",
            "重命名",
            "Mudar o nome",
        ],
    ),
    (
        LibText::Delete,
        [
            "Delete",
            "Borrar",
            "Löschen",
            "Supprimer",
            "Elimina",
            "削除",
            "删除",
            "Apagar",
        ],
    ),
    (
        LibText::Name,
        [
            "Name",
            "Nombre",
            "Name",
            "Nom",
            "Nome",
            "名前",
            "名称",
            "Nome",
        ],
    ),
    (
        LibText::Prefix,
        [
            "Prefix",
            "Prefijo",
            "Präfix",
            "Préfixe",
            "Prefisso",
            "プレフィックス",
            "前缀",
            "Prefixo",
        ],
    ),
    (
        LibText::ToolSelect,
        [
            "Select",
            "Seleccionar",
            "Auswählen",
            "Sélectionner",
            "Seleziona",
            "選択",
            "选择",
            "Selecionar",
        ],
    ),
    (
        LibText::ToolSmd,
        [
            "SMD pad",
            "Pad SMD",
            "SMD-Pad",
            "Pastille CMS",
            "Pad SMD",
            "SMDパッド",
            "贴片焊盘",
            "Pad SMD",
        ],
    ),
    (
        LibText::ToolPad,
        [
            "Through-hole pad",
            "Pad pasante",
            "Durchsteck-Pad",
            "Pastille traversante",
            "Pad passante",
            "スルーホールパッド",
            "通孔焊盘",
            "Pad de furo passante",
        ],
    ),
    (
        LibText::ToolLine,
        [
            "Line",
            "Línea",
            "Linie",
            "Ligne",
            "Linea",
            "線",
            "线",
            "Linha",
        ],
    ),
    (
        LibText::ToolArc,
        [
            "Arc",
            "Arco",
            "Bogen",
            "Arc",
            "Arco",
            "円弧",
            "圆弧",
            "Arco",
        ],
    ),
    (
        LibText::ToolCircle,
        [
            "Circle",
            "Círculo",
            "Kreis",
            "Cercle",
            "Cerchio",
            "円",
            "圆",
            "Círculo",
        ],
    ),
    (
        LibText::ToolText,
        [
            "Text",
            "Texto",
            "Text",
            "Texte",
            "Testo",
            "テキスト",
            "文本",
            "Texto",
        ],
    ),
    (
        LibText::ToolHole,
        [
            "Hole",
            "Agujero",
            "Bohrung",
            "Trou",
            "Foro",
            "穴",
            "孔",
            "Furo",
        ],
    ),
    (
        LibText::ToolPin,
        [
            "Pin",
            "Pin",
            "Pin",
            "Broche",
            "Pin",
            "ピン",
            "引脚",
            "Pino",
        ],
    ),
    (
        LibText::ToolMeasure,
        [
            "Measure",
            "Medir",
            "Messen",
            "Mesurer",
            "Misura",
            "測定",
            "测量",
            "Medir",
        ],
    ),
    (
        LibText::Grid,
        [
            "Grid",
            "Rejilla",
            "Raster",
            "Grille",
            "Griglia",
            "グリッド",
            "网格",
            "Grelha",
        ],
    ),
    (
        LibText::Snap,
        [
            "Snap to grid",
            "Ajustar a la rejilla",
            "Am Raster ausrichten",
            "Aimanter à la grille",
            "Aggancia alla griglia",
            "グリッドにスナップ",
            "吸附到网格",
            "Ajustar à grelha",
        ],
    ),
    (
        LibText::Properties,
        [
            "Properties",
            "Propiedades",
            "Eigenschaften",
            "Propriétés",
            "Proprietà",
            "プロパティ",
            "属性",
            "Propriedades",
        ],
    ),
    (
        LibText::Apply,
        [
            "Apply",
            "Aplicar",
            "Anwenden",
            "Appliquer",
            "Applica",
            "適用",
            "应用",
            "Aplicar",
        ],
    ),
    (
        LibText::Origin,
        [
            "Origin",
            "Origen",
            "Ursprung",
            "Origine",
            "Origine",
            "原点",
            "原点",
            "Origem",
        ],
    ),
    (
        LibText::OriginToCursor,
        [
            "Origin to the cursor",
            "Origen al cursor",
            "Ursprung zum Cursor",
            "Origine au curseur",
            "Origine al cursore",
            "原点をカーソルへ",
            "原点移到光标",
            "Origem para o cursor",
        ],
    ),
    (
        LibText::Width,
        [
            "Width",
            "Ancho",
            "Breite",
            "Largeur",
            "Larghezza",
            "幅",
            "宽度",
            "Largura",
        ],
    ),
    (
        LibText::Height,
        [
            "Height",
            "Alto",
            "Höhe",
            "Hauteur",
            "Altezza",
            "高さ",
            "高度",
            "Altura",
        ],
    ),
    (
        LibText::Drill,
        [
            "Drill",
            "Taladro",
            "Bohrung",
            "Perçage",
            "Foratura",
            "ドリル",
            "钻孔",
            "Furação",
        ],
    ),
    (
        LibText::Diameter,
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
        LibText::Radius,
        [
            "Radius",
            "Radio",
            "Radius",
            "Rayon",
            "Raggio",
            "半径",
            "半径",
            "Raio",
        ],
    ),
    (
        LibText::Layer,
        [
            "Layer",
            "Capa",
            "Lage",
            "Couche",
            "Strato",
            "レイヤー",
            "层",
            "Camada",
        ],
    ),
    (
        LibText::Shape,
        [
            "Shape",
            "Forma",
            "Form",
            "Forme",
            "Forma",
            "形状",
            "形状",
            "Forma",
        ],
    ),
    (
        LibText::Rotation,
        [
            "Rotation",
            "Rotación",
            "Drehung",
            "Rotation",
            "Rotazione",
            "回転",
            "旋转",
            "Rotação",
        ],
    ),
    (
        LibText::Roundness,
        [
            "Roundness",
            "Redondez",
            "Rundung",
            "Arrondi",
            "Arrotondamento",
            "丸み",
            "圆度",
            "Arredondamento",
        ],
    ),
    (
        LibText::LineWidth,
        [
            "Line width",
            "Ancho de línea",
            "Linienbreite",
            "Largeur de ligne",
            "Larghezza linea",
            "線幅",
            "线宽",
            "Largura da linha",
        ],
    ),
    (
        LibText::ArcAngle,
        [
            "Arc angle",
            "Ángulo del arco",
            "Bogenwinkel",
            "Angle de l'arc",
            "Angolo dell'arco",
            "円弧の角度",
            "圆弧角度",
            "Ângulo do arco",
        ],
    ),
    (
        LibText::TextHeight,
        [
            "Text height",
            "Altura del texto",
            "Texthöhe",
            "Hauteur du texte",
            "Altezza del testo",
            "文字の高さ",
            "文本高度",
            "Altura do texto",
        ],
    ),
    (
        LibText::ElectricalType,
        [
            "Electrical type",
            "Tipo eléctrico",
            "Elektrischer Typ",
            "Type électrique",
            "Tipo elettrico",
            "電気的タイプ",
            "电气类型",
            "Tipo elétrico",
        ],
    ),
    (
        LibText::PinLength,
        [
            "Pin length",
            "Longitud del pin",
            "Pin-Länge",
            "Longueur de la broche",
            "Lunghezza del pin",
            "ピンの長さ",
            "引脚长度",
            "Comprimento do pino",
        ],
    ),
    (
        LibText::MaskOpening,
        [
            "Solder mask opening",
            "Apertura de máscara",
            "Lötstopp-Öffnung",
            "Ouverture du masque",
            "Apertura della maschera",
            "ソルダーマスク開口",
            "阻焊开窗",
            "Abertura da máscara",
        ],
    ),
    (
        LibText::PasteOpening,
        [
            "Paste opening",
            "Apertura de pasta",
            "Pastenöffnung",
            "Ouverture de la pâte",
            "Apertura della pasta",
            "ペースト開口",
            "锡膏开窗",
            "Abertura da pasta",
        ],
    ),
    (
        LibText::Expansion,
        [
            "Expansion",
            "Expansión",
            "Erweiterung",
            "Expansion",
            "Espansione",
            "拡張",
            "扩展",
            "Expansão",
        ],
    ),
    (
        LibText::Automatic,
        [
            "Automatic",
            "Automático",
            "Automatisch",
            "Automatique",
            "Automatico",
            "自動",
            "自动",
            "Automático",
        ],
    ),
    (
        LibText::Set,
        [
            "Set",
            "Fijar",
            "Setzen",
            "Définir",
            "Imposta",
            "設定",
            "设置",
            "Definir",
        ],
    ),
    (
        LibText::Gates,
        [
            "Gates",
            "Puertas",
            "Gatter",
            "Portes",
            "Porte",
            "ゲート",
            "门",
            "Portas",
        ],
    ),
    (
        LibText::Variants,
        [
            "Variants",
            "Variantes",
            "Varianten",
            "Variantes",
            "Varianti",
            "バリアント",
            "变体",
            "Variantes",
        ],
    ),
    (
        LibText::AddGate,
        [
            "Add gate",
            "Añadir puerta",
            "Gate hinzufügen",
            "Ajouter une porte",
            "Aggiungi porta",
            "ゲートを追加",
            "添加门",
            "Adicionar porta",
        ],
    ),
    (
        LibText::AddVariant,
        [
            "Add variant",
            "Añadir variante",
            "Variante hinzufügen",
            "Ajouter une variante",
            "Aggiungi variante",
            "バリアントを追加",
            "添加变体",
            "Adicionar variante",
        ],
    ),
    (
        LibText::AutoConnect,
        [
            "Connect pins to pads of the same name",
            "Conectar pines a pads del mismo nombre",
            "Pins mit gleichnamigen Pads verbinden",
            "Relier les broches aux pastilles du même nom",
            "Collega i pin ai pad con lo stesso nome",
            "同名のパッドにピンを接続",
            "将引脚连接到同名焊盘",
            "Ligar pinos a pads com o mesmo nome",
        ],
    ),
    (
        LibText::PinToPad,
        [
            "Pin to pad",
            "Pin a pad",
            "Pin zu Pad",
            "Broche vers pastille",
            "Pin verso pad",
            "ピンからパッドへ",
            "引脚到焊盘",
            "Pino para pad",
        ],
    ),
    (
        LibText::NoPad,
        [
            "no pad",
            "sin pad",
            "kein Pad",
            "sans pastille",
            "nessun pad",
            "パッドなし",
            "无焊盘",
            "sem pad",
        ],
    ),
    (
        LibText::Findings,
        [
            "Checks of the library",
            "Comprobaciones de la biblioteca",
            "Prüfungen der Bibliothek",
            "Vérifications de la bibliothèque",
            "Controlli della libreria",
            "ライブラリのチェック",
            "库检查",
            "Verificações da biblioteca",
        ],
    ),
    (
        LibText::RunCheck,
        [
            "Check the library",
            "Comprobar la biblioteca",
            "Bibliothek prüfen",
            "Vérifier la bibliothèque",
            "Controlla la libreria",
            "ライブラリをチェック",
            "检查库",
            "Verificar a biblioteca",
        ],
    ),
    (
        LibText::NoFindings,
        [
            "No findings",
            "Sin hallazgos",
            "Keine Befunde",
            "Aucun constat",
            "Nessun rilievo",
            "指摘なし",
            "没有发现问题",
            "Sem constatações",
        ],
    ),
    (
        LibText::SavePath,
        [
            "New file",
            "Archivo nuevo",
            "Neue Datei",
            "Nouveau fichier",
            "Nuovo file",
            "新しいファイル",
            "新文件",
            "Novo ficheiro",
        ],
    ),
    (
        LibText::SaveAsNew,
        [
            "Save as new file",
            "Guardar como archivo nuevo",
            "Als neue Datei speichern",
            "Enregistrer en nouveau fichier",
            "Salva come nuovo file",
            "新しいファイルとして保存",
            "另存为新文件",
            "Guardar como novo ficheiro",
        ],
    ),
    (
        LibText::Unsaved,
        [
            "Not saved",
            "Sin guardar",
            "Nicht gespeichert",
            "Non enregistré",
            "Non salvato",
            "未保存",
            "未保存",
            "Por guardar",
        ],
    ),
    (
        LibText::Plan,
        [
            "Replace by the master (plan)",
            "Reemplazar por el maestro (plan)",
            "Durch den Master ersetzen (Plan)",
            "Remplacer par le maître (plan)",
            "Sostituisci con il master (piano)",
            "マスターで置換（計画）",
            "用主库项替换（计划）",
            "Substituir pelo mestre (plano)",
        ],
    ),
    (
        LibText::PlanFile,
        [
            "Plan of the sanitizer",
            "Plan del saneador",
            "Plan des Bereinigers",
            "Plan de l'assainisseur",
            "Piano del sanificatore",
            "サニタイザーの計画",
            "清理器的计划",
            "Plano do saneador",
        ],
    ),
    (
        LibText::MasterFolder,
        [
            "Folder of the master libraries",
            "Carpeta de las bibliotecas maestras",
            "Ordner der Master-Bibliotheken",
            "Dossier des bibliothèques maîtres",
            "Cartella delle librerie master",
            "マスターライブラリのフォルダ",
            "主库文件夹",
            "Pasta das bibliotecas mestras",
        ],
    ),
    (
        LibText::LoadPlan,
        [
            "Load plan",
            "Cargar plan",
            "Plan laden",
            "Charger le plan",
            "Carica piano",
            "計画を読み込む",
            "加载计划",
            "Carregar plano",
        ],
    ),
    (
        LibText::Confirm,
        [
            "I confirm the replacement",
            "Confirmo el reemplazo",
            "Ich bestätige den Ersatz",
            "Je confirme le remplacement",
            "Confermo la sostituzione",
            "置換を確認します",
            "我确认替换",
            "Confirmo a substituição",
        ],
    ),
    (
        LibText::Replace,
        [
            "Replace",
            "Reemplazar",
            "Ersetzen",
            "Remplacer",
            "Sostituisci",
            "置換",
            "替换",
            "Substituir",
        ],
    ),
    (
        LibText::PlanNotApplied,
        [
            "Nothing is replaced until you confirm each step.",
            "No se reemplaza nada hasta que confirmes cada paso.",
            "Nichts wird ersetzt, bis Sie jeden Schritt bestätigen.",
            "Rien n'est remplacé avant que vous confirmiez chaque étape.",
            "Non viene sostituito nulla finché non confermi ogni passo.",
            "各ステップを確認するまで何も置換されません。",
            "在您确认每一步之前不会替换任何内容。",
            "Nada é substituído até confirmar cada passo.",
        ],
    ),
    (
        LibText::Generator,
        [
            "Footprint generator",
            "Generador de huellas",
            "Generator für Bauformen",
            "Générateur d'empreintes",
            "Generatore di impronte",
            "フットプリント生成",
            "封装生成器",
            "Gerador de pegadas",
        ],
    ),
    (
        LibText::Family,
        [
            "Family",
            "Familia",
            "Familie",
            "Famille",
            "Famiglia",
            "ファミリー",
            "系列",
            "Família",
        ],
    ),
    (
        LibText::Parameters,
        [
            "Parameters",
            "Parámetros",
            "Parameter",
            "Paramètres",
            "Parametri",
            "パラメータ",
            "参数",
            "Parâmetros",
        ],
    ),
    (
        LibText::AddToLibrary,
        [
            "Add to the library",
            "Añadir a la biblioteca",
            "Zur Bibliothek hinzufügen",
            "Ajouter à la bibliothèque",
            "Aggiungi alla libreria",
            "ライブラリに追加",
            "添加到库",
            "Adicionar à biblioteca",
        ],
    ),
    (
        LibText::Prototype,
        [
            "Prototype: the dimensions of the generators are not checked against a datasheet, and no EAGLE has opened what is saved here.",
            "Prototipo: las dimensiones de los generadores no están comprobadas con una hoja de datos y ningún EAGLE ha abierto lo que se guarda aquí.",
            "Prototyp: Die Maße der Generatoren sind nicht mit einem Datenblatt geprüft, und kein EAGLE hat das hier Gespeicherte geöffnet.",
            "Prototype : les dimensions des générateurs ne sont pas vérifiées avec une fiche technique et aucun EAGLE n'a ouvert ce qui est enregistré ici.",
            "Prototipo: le dimensioni dei generatori non sono verificate con una scheda tecnica e nessun EAGLE ha aperto ciò che qui viene salvato.",
            "試作版：ジェネレーターの寸法はデータシートで確認されておらず、ここで保存した内容を EAGLE で開いた例はありません。",
            "原型：生成器的尺寸尚未对照数据手册核实，也没有任何 EAGLE 打开过这里保存的内容。",
            "Protótipo: as dimensões dos geradores não estão verificadas com uma folha de dados e nenhum EAGLE abriu o que aqui se guarda.",
        ],
    ),
    (
        LibText::Shortcuts,
        [
            "Del delete · R turn · Ctrl+Z undo · Ctrl+Y redo · Esc cancel · click to place",
            "Supr borrar · R girar · Ctrl+Z deshacer · Ctrl+Y rehacer · Esc cancelar · clic para colocar",
            "Entf löschen · R drehen · Strg+Z rückgängig · Strg+Y wiederholen · Esc abbrechen · Klick zum Platzieren",
            "Suppr supprimer · R pivoter · Ctrl+Z annuler · Ctrl+Y rétablir · Échap annuler · clic pour placer",
            "Canc elimina · R ruota · Ctrl+Z annulla · Ctrl+Y ripeti · Esc annulla · clic per posizionare",
            "Del 削除 · R 回転 · Ctrl+Z 元に戻す · Ctrl+Y やり直す · Esc 中止 · クリックで配置",
            "Del 删除 · R 旋转 · Ctrl+Z 撤销 · Ctrl+Y 重做 · Esc 取消 · 单击放置",
            "Del apagar · R rodar · Ctrl+Z anular · Ctrl+Y refazer · Esc cancelar · clique para colocar",
        ],
    ),
    (
        LibText::NoteOpened,
        [
            "Library opened",
            "Biblioteca abierta",
            "Bibliothek geöffnet",
            "Bibliothèque ouverte",
            "Libreria aperta",
            "ライブラリを開きました",
            "已打开库",
            "Biblioteca aberta",
        ],
    ),
    (
        LibText::NoteSaved,
        [
            "Saved as a new file, read back and equal",
            "Guardada como archivo nuevo, releída e igual",
            "Als neue Datei gespeichert, zurückgelesen und gleich",
            "Enregistrée en nouveau fichier, relue et identique",
            "Salvata come nuovo file, riletta e uguale",
            "新しいファイルに保存し、読み戻して同一を確認しました",
            "已保存为新文件，读回后一致",
            "Guardada como novo ficheiro, relida e igual",
        ],
    ),
    (
        LibText::NoteBackup,
        [
            "Copy of the original",
            "Copia del original",
            "Kopie des Originals",
            "Copie de l'original",
            "Copia dell'originale",
            "元ファイルのコピー",
            "原文件副本",
            "Cópia do original",
        ],
    ),
    (
        LibText::NoteNothingUndo,
        [
            "Nothing to undo",
            "Nada que deshacer",
            "Nichts rückgängig zu machen",
            "Rien à annuler",
            "Niente da annullare",
            "元に戻す操作はありません",
            "没有可撤销的操作",
            "Nada para anular",
        ],
    ),
    (
        LibText::NoteNothingRedo,
        [
            "Nothing to redo",
            "Nada que rehacer",
            "Nichts zu wiederholen",
            "Rien à rétablir",
            "Niente da ripetere",
            "やり直す操作はありません",
            "没有可重做的操作",
            "Nada para refazer",
        ],
    ),
    (
        LibText::NoteUndone,
        [
            "Undone",
            "Deshecho",
            "Rückgängig gemacht",
            "Annulé",
            "Annullato",
            "元に戻しました",
            "已撤销",
            "Anulado",
        ],
    ),
    (
        LibText::NoteRedone,
        [
            "Redone",
            "Rehecho",
            "Wiederholt",
            "Rétabli",
            "Ripetuto",
            "やり直しました",
            "已重做",
            "Refeito",
        ],
    ),
    (
        LibText::NoteDone,
        [
            "Done",
            "Hecho",
            "Erledigt",
            "Fait",
            "Fatto",
            "完了",
            "完成",
            "Feito",
        ],
    ),
    (
        LibText::NoteSecondPoint,
        [
            "Click the second point",
            "Haz clic en el segundo punto",
            "Zweiten Punkt anklicken",
            "Cliquez sur le second point",
            "Fai clic sul secondo punto",
            "2点目をクリック",
            "请单击第二个点",
            "Clique no segundo ponto",
        ],
    ),
    (
        LibText::NoteWrongKind,
        [
            "This tool is not for this kind of drawing",
            "Esta herramienta no sirve para este dibujo",
            "Dieses Werkzeug passt nicht zu dieser Zeichnung",
            "Cet outil ne convient pas à ce dessin",
            "Questo strumento non è adatto a questo disegno",
            "このツールはこの図面には使えません",
            "此工具不适用于该图形",
            "Esta ferramenta não serve para este desenho",
        ],
    ),
    (
        LibText::NoteNoEntity,
        [
            "Choose or make a footprint or a symbol first",
            "Elige o crea antes una huella o un símbolo",
            "Wählen oder erstellen Sie zuerst eine Bauform oder ein Symbol",
            "Choisissez ou créez d'abord une empreinte ou un symbole",
            "Scegli o crea prima un'impronta o un simbolo",
            "先にフットプリントまたはシンボルを選択または作成してください",
            "请先选择或创建封装或符号",
            "Escolha ou crie primeiro uma pegada ou um símbolo",
        ],
    ),
    (
        LibText::NoteNoSelection,
        [
            "Nothing is selected",
            "No hay nada seleccionado",
            "Nichts ausgewählt",
            "Rien n'est sélectionné",
            "Nulla è selezionato",
            "何も選択されていません",
            "未选择任何内容",
            "Nada está selecionado",
        ],
    ),
    (
        LibText::NoteCannotRotate,
        [
            "This item cannot be turned",
            "Este elemento no se puede girar",
            "Dieses Element kann nicht gedreht werden",
            "Cet élément ne peut pas être tourné",
            "Questo elemento non può essere ruotato",
            "この要素は回転できません",
            "无法旋转该元素",
            "Este elemento não pode ser rodado",
        ],
    ),
    (
        LibText::NoteChecked,
        [
            "Check done",
            "Comprobación hecha",
            "Prüfung abgeschlossen",
            "Vérification faite",
            "Controllo eseguito",
            "チェック完了",
            "检查完成",
            "Verificação feita",
        ],
    ),
    (
        LibText::NotePlanLoaded,
        [
            "Plan loaded",
            "Plan cargado",
            "Plan geladen",
            "Plan chargé",
            "Piano caricato",
            "計画を読み込みました",
            "已加载计划",
            "Plano carregado",
        ],
    ),
    (
        LibText::NoteGenerated,
        [
            "Footprint added to the library",
            "Huella añadida a la biblioteca",
            "Bauform zur Bibliothek hinzugefügt",
            "Empreinte ajoutée à la bibliothèque",
            "Impronta aggiunta alla libreria",
            "フットプリントをライブラリに追加しました",
            "封装已添加到库",
            "Pegada adicionada à biblioteca",
        ],
    ),
];

impl LibText {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = LibText::NoteGenerated as usize + 1;

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
        assert_eq!(TABLE.len(), LibText::COUNT);
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
        assert_eq!(LibText::SavePath.tr(Lang::Pt), "Novo ficheiro");
        assert!(LibText::SaveAsNew.tr(Lang::Pt).contains("ficheiro"));
        assert_eq!(LibText::Delete.tr(Lang::Pt), "Apagar");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(
                !text.contains("Salvar") && !text.contains("salvo"),
                "{msg:?}"
            );
            assert!(
                !text.contains("Excluir") && !text.contains("excluir"),
                "{msg:?}"
            );
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            LibText::Title,
            LibText::Footprints,
            LibText::New,
            LibText::Delete,
            LibText::Snap,
            LibText::Properties,
            LibText::Findings,
            LibText::SaveAsNew,
            LibText::Plan,
            LibText::Prototype,
            LibText::NoteSaved,
            LibText::NoteUndone,
        ] {
            for lang in [
                Lang::Es,
                Lang::De,
                Lang::Fr,
                Lang::It,
                Lang::Ja,
                Lang::Zh,
                Lang::Pt,
            ] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
