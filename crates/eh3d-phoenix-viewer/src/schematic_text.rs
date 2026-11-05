// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the schematic editor in the window, in the eight languages of the suite: English,
//! Spanish, German, French, Italian, Japanese, Chinese and European Portuguese. The texts of the
//! editing of boards are in [`crate::editmsg`].

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the schematic editor. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchMsg {
    /// The title of the panel of the editor.
    EditingTitle,
    /// The button that starts editing the schematic.
    EditSchematic,
    /// The button that goes back to looking at the schematic.
    StopEditing,
    /// The tool that picks and drags.
    ToolSelect,
    /// The tool that draws wires.
    ToolWire,
    /// The tool that puts net labels.
    ToolLabel,
    /// The tool that puts power ports.
    ToolPower,
    /// The tool that puts the labels of a bus.
    ToolBus,
    /// The tool that places a symbol.
    ToolPlace,
    /// The button that turns the selected parts.
    Rotate,
    /// The button that mirrors the selected parts.
    Mirror,
    /// The button that deletes the selection.
    Delete,
    /// The button that copies the selection.
    Copy,
    /// The button that cuts the selection.
    Cut,
    /// The button that pastes what was copied.
    Paste,
    /// The label of the name of a label, power port or bus.
    NameLabel,
    /// What the name may be.
    NameHint,
    /// The label of the value of a part.
    ValueLabel,
    /// The label of the reference of a part.
    RefLabel,
    /// The button that sets a value or a reference.
    Apply,
    /// The title of the list of symbols.
    PaletteTitle,
    /// The label of the search box.
    SearchLabel,
    /// What the list says when nothing matches.
    NoSymbols,
    /// The label of the path of a library file.
    LibraryPath,
    /// The button that loads a library.
    LoadLibrary,
    /// What follows the number of symbols that a library added.
    LibraryAdded,
    /// What is said when a library cannot be loaded.
    LibraryFailed,
    /// The label of the grid.
    GridLabel,
    /// The box that turns snapping on.
    SnapToGrid,
    /// The box that turns the rule check after each change on.
    LiveErc,
    /// The label of the sheet.
    SheetLabel,
    /// The title of the reason an operation was refused.
    Refused,
    /// What is said when an action needs a selection and there is none.
    NothingSelected,
    /// What follows the number of items selected.
    SelectedCount,
    /// The hint of the select tool.
    HintSelect,
    /// The hint of the wire tool.
    HintWire,
    /// The hint of the tools that put a name.
    HintLabel,
    /// The hint of the place tool.
    HintPlace,
    /// The keys of the editor.
    KeysHint,
}

const TABLE: [(SchMsg, [&str; 8]); SchMsg::COUNT] = [
    (
        SchMsg::EditingTitle,
        [
            "Schematic editor",
            "Editor de esquemas",
            "Schaltplan-Editor",
            "Éditeur de schémas",
            "Editor di schemi",
            "回路図エディタ",
            "原理图编辑器",
            "Editor de esquemas",
        ],
    ),
    (
        SchMsg::EditSchematic,
        [
            "Edit schematic",
            "Editar esquema",
            "Schaltplan bearbeiten",
            "Modifier le schéma",
            "Modifica schema",
            "回路図を編集",
            "编辑原理图",
            "Editar esquema",
        ],
    ),
    (
        SchMsg::StopEditing,
        [
            "Stop editing",
            "Dejar de editar",
            "Bearbeitung beenden",
            "Terminer la modification",
            "Termina modifica",
            "編集を終了",
            "结束编辑",
            "Terminar a edição",
        ],
    ),
    (
        SchMsg::ToolSelect,
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
        SchMsg::ToolWire,
        [
            "Wire",
            "Cable",
            "Leitung",
            "Fil",
            "Filo",
            "配線",
            "导线",
            "Fio",
        ],
    ),
    (
        SchMsg::ToolLabel,
        [
            "Net label",
            "Etiqueta de red",
            "Netzlabel",
            "Étiquette de net",
            "Etichetta di rete",
            "ネットラベル",
            "网络标签",
            "Etiqueta de rede",
        ],
    ),
    (
        SchMsg::ToolPower,
        [
            "Power port",
            "Puerto de alimentación",
            "Versorgungsanschluss",
            "Port d'alimentation",
            "Porta di alimentazione",
            "電源ポート",
            "电源端口",
            "Porto de alimentação",
        ],
    ),
    (
        SchMsg::ToolBus,
        [
            "Bus",
            "Bus",
            "Bus",
            "Bus",
            "Bus",
            "バス",
            "总线",
            "Barramento",
        ],
    ),
    (
        SchMsg::ToolPlace,
        [
            "Place part",
            "Colocar pieza",
            "Bauteil platzieren",
            "Placer un composant",
            "Posiziona componente",
            "部品を配置",
            "放置元件",
            "Colocar componente",
        ],
    ),
    (
        SchMsg::Rotate,
        [
            "Rotate", "Girar", "Drehen", "Pivoter", "Ruota", "回転", "旋转", "Rodar",
        ],
    ),
    (
        SchMsg::Mirror,
        [
            "Mirror",
            "Reflejar",
            "Spiegeln",
            "Miroir",
            "Specchia",
            "反転",
            "镜像",
            "Espelhar",
        ],
    ),
    (
        SchMsg::Delete,
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
        SchMsg::Copy,
        [
            "Copy", "Copiar", "Kopieren", "Copier", "Copia", "コピー", "复制", "Copiar",
        ],
    ),
    (
        SchMsg::Cut,
        [
            "Cut",
            "Cortar",
            "Ausschneiden",
            "Couper",
            "Taglia",
            "切り取り",
            "剪切",
            "Cortar",
        ],
    ),
    (
        SchMsg::Paste,
        [
            "Paste",
            "Pegar",
            "Einfügen",
            "Coller",
            "Incolla",
            "貼り付け",
            "粘贴",
            "Colar",
        ],
    ),
    (
        SchMsg::NameLabel,
        [
            "Name", "Nombre", "Name", "Nom", "Nome", "名前", "名称", "Nome",
        ],
    ),
    (
        SchMsg::NameHint,
        [
            "A name, or a bus such as D[0..7]",
            "Un nombre, o un bus como D[0..7]",
            "Ein Name oder ein Bus wie D[0..7]",
            "Un nom, ou un bus comme D[0..7]",
            "Un nome, o un bus come D[0..7]",
            "名前、または D[0..7] のようなバス",
            "名称，或类似 D[0..7] 的总线",
            "Um nome, ou um barramento como D[0..7]",
        ],
    ),
    (
        SchMsg::ValueLabel,
        [
            "Value", "Valor", "Wert", "Valeur", "Valore", "値", "值", "Valor",
        ],
    ),
    (
        SchMsg::RefLabel,
        [
            "Reference",
            "Referencia",
            "Referenz",
            "Référence",
            "Riferimento",
            "参照",
            "位号",
            "Referência",
        ],
    ),
    (
        SchMsg::Apply,
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
        SchMsg::PaletteTitle,
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
        SchMsg::SearchLabel,
        [
            "Search",
            "Buscar",
            "Suchen",
            "Rechercher",
            "Cerca",
            "検索",
            "搜索",
            "Pesquisar",
        ],
    ),
    (
        SchMsg::NoSymbols,
        [
            "No symbol matches",
            "Ningún símbolo coincide",
            "Kein Symbol passt",
            "Aucun symbole ne correspond",
            "Nessun simbolo corrisponde",
            "一致するシンボルはありません",
            "没有匹配的符号",
            "Nenhum símbolo corresponde",
        ],
    ),
    (
        SchMsg::LibraryPath,
        [
            "Library file",
            "Archivo de biblioteca",
            "Bibliotheksdatei",
            "Fichier de bibliothèque",
            "File della libreria",
            "ライブラリファイル",
            "库文件",
            "Ficheiro da biblioteca",
        ],
    ),
    (
        SchMsg::LoadLibrary,
        [
            "Load library",
            "Cargar biblioteca",
            "Bibliothek laden",
            "Charger la bibliothèque",
            "Carica libreria",
            "ライブラリを読み込む",
            "加载库",
            "Carregar biblioteca",
        ],
    ),
    (
        SchMsg::LibraryAdded,
        [
            "symbols added",
            "símbolos añadidos",
            "Symbole hinzugefügt",
            "symboles ajoutés",
            "simboli aggiunti",
            "個のシンボルを追加しました",
            "个符号已添加",
            "símbolos adicionados",
        ],
    ),
    (
        SchMsg::LibraryFailed,
        [
            "The library could not be loaded",
            "No se pudo cargar la biblioteca",
            "Die Bibliothek konnte nicht geladen werden",
            "La bibliothèque n'a pas pu être chargée",
            "Impossibile caricare la libreria",
            "ライブラリを読み込めませんでした",
            "无法加载该库",
            "Não foi possível carregar a biblioteca",
        ],
    ),
    (
        SchMsg::GridLabel,
        [
            "Grid (mm)",
            "Rejilla (mm)",
            "Raster (mm)",
            "Grille (mm)",
            "Griglia (mm)",
            "グリッド (mm)",
            "网格 (mm)",
            "Grelha (mm)",
        ],
    ),
    (
        SchMsg::SnapToGrid,
        [
            "Snap to the grid",
            "Ajustar a la rejilla",
            "Am Raster ausrichten",
            "Aimanter à la grille",
            "Aggancia alla griglia",
            "グリッドに吸着",
            "吸附到网格",
            "Ajustar à grelha",
        ],
    ),
    (
        SchMsg::LiveErc,
        [
            "Rule check after each change",
            "Comprobar reglas tras cada cambio",
            "Regelprüfung nach jeder Änderung",
            "Vérifier les règles après chaque modification",
            "Controllo regole dopo ogni modifica",
            "変更のたびにルールチェック",
            "每次更改后检查规则",
            "Verificar regras após cada alteração",
        ],
    ),
    (
        SchMsg::SheetLabel,
        [
            "Sheet", "Hoja", "Blatt", "Feuille", "Foglio", "シート", "图纸", "Folha",
        ],
    ),
    (
        SchMsg::Refused,
        [
            "Not done",
            "No se hizo",
            "Nicht ausgeführt",
            "Non effectué",
            "Non eseguito",
            "実行できません",
            "未执行",
            "Não realizado",
        ],
    ),
    (
        SchMsg::NothingSelected,
        [
            "Nothing is selected",
            "No hay nada seleccionado",
            "Nichts ist ausgewählt",
            "Rien n'est sélectionné",
            "Nulla è selezionato",
            "何も選択されていません",
            "未选择任何内容",
            "Nada está selecionado",
        ],
    ),
    (
        SchMsg::SelectedCount,
        [
            "selected",
            "seleccionados",
            "ausgewählt",
            "sélectionnés",
            "selezionati",
            "選択中",
            "已选择",
            "selecionados",
        ],
    ),
    (
        SchMsg::HintSelect,
        [
            "Click to pick, drag to move, drag on empty space to select several",
            "Haz clic para elegir, arrastra para mover, arrastra en un espacio vacío para elegir varios",
            "Klicken zum Wählen, Ziehen zum Verschieben, Ziehen auf leerer Fläche für mehrere",
            "Cliquez pour choisir, glissez pour déplacer, glissez dans le vide pour en choisir plusieurs",
            "Clic per scegliere, trascina per spostare, trascina nel vuoto per sceglierne più d'uno",
            "クリックで選択、ドラッグで移動、空白をドラッグして複数選択",
            "单击选择，拖动移动，在空白处拖动可框选多个",
            "Clique para escolher, arraste para mover, arraste no espaço vazio para escolher vários",
        ],
    ),
    (
        SchMsg::HintWire,
        [
            "Click to start a wire and click again to end it; Space changes the corner, Esc stops",
            "Haz clic para empezar un cable y otra vez para terminarlo; Espacio cambia la esquina, Esc lo detiene",
            "Klicken zum Beginnen und erneut zum Beenden; Leertaste ändert die Ecke, Esc bricht ab",
            "Cliquez pour commencer un fil et recliquez pour le terminer ; Espace change le coin, Échap arrête",
            "Clic per iniziare un filo e di nuovo per terminarlo; Spazio cambia l'angolo, Esc interrompe",
            "クリックで配線を開始し、もう一度クリックで終了。スペースで角を切り替え、Escで中止",
            "单击开始导线，再次单击结束；空格切换拐角，Esc 停止",
            "Clique para começar um fio e clique de novo para o terminar; Espaço muda o canto, Esc interrompe",
        ],
    ),
    (
        SchMsg::HintLabel,
        [
            "Click where the name goes",
            "Haz clic donde va el nombre",
            "Klicken, wo der Name hin soll",
            "Cliquez où placer le nom",
            "Clic dove va il nome",
            "名前を置く位置をクリック",
            "单击放置名称的位置",
            "Clique onde o nome fica",
        ],
    ),
    (
        SchMsg::HintPlace,
        [
            "Choose a symbol in the list, then click where it goes",
            "Elige un símbolo de la lista y haz clic donde va",
            "Symbol in der Liste wählen, dann klicken, wo es hin soll",
            "Choisissez un symbole dans la liste puis cliquez où le placer",
            "Scegli un simbolo dalla lista, poi clic dove va",
            "リストからシンボルを選び、置く位置をクリック",
            "在列表中选择符号，然后单击放置位置",
            "Escolha um símbolo da lista e clique onde fica",
        ],
    ),
    (
        SchMsg::KeysHint,
        [
            "V select, W wire, L label, P power, B bus, R rotate, X mirror, Del delete, Ctrl+C/X/V copy, cut, paste, Ctrl+Z/Y undo, redo",
            "V seleccionar, W cable, L etiqueta, P alimentación, B bus, R girar, X reflejar, Supr borrar, Ctrl+C/X/V copiar, cortar, pegar, Ctrl+Z/Y deshacer, rehacer",
            "V Auswählen, W Leitung, L Label, P Versorgung, B Bus, R Drehen, X Spiegeln, Entf Löschen, Strg+C/X/V Kopieren, Ausschneiden, Einfügen, Strg+Z/Y Rückgängig, Wiederholen",
            "V sélection, W fil, L étiquette, P alimentation, B bus, R pivoter, X miroir, Suppr supprimer, Ctrl+C/X/V copier, couper, coller, Ctrl+Z/Y annuler, rétablir",
            "V seleziona, W filo, L etichetta, P alimentazione, B bus, R ruota, X specchia, Canc elimina, Ctrl+C/X/V copia, taglia, incolla, Ctrl+Z/Y annulla, ripristina",
            "V 選択、W 配線、L ラベル、P 電源、B バス、R 回転、X 反転、Del 削除、Ctrl+C/X/V コピー・切り取り・貼り付け、Ctrl+Z/Y 元に戻す・やり直し",
            "V 选择，W 导线，L 标签，P 电源，B 总线，R 旋转，X 镜像，Del 删除，Ctrl+C/X/V 复制、剪切、粘贴，Ctrl+Z/Y 撤销、重做",
            "V selecionar, W fio, L etiqueta, P alimentação, B barramento, R rodar, X espelhar, Del apagar, Ctrl+C/X/V copiar, cortar, colar, Ctrl+Z/Y anular, refazer",
        ],
    ),
];

impl SchMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = SchMsg::KeysHint as usize + 1;

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
        assert_eq!(TABLE.len(), SchMsg::COUNT);
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
        assert_eq!(SchMsg::LoadLibrary.tr(Lang::Pt), "Carregar biblioteca");
        assert!(SchMsg::LibraryPath.tr(Lang::Pt).contains("Ficheiro"));
        assert_eq!(SchMsg::SearchLabel.tr(Lang::Pt), "Pesquisar");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(
                !text.contains("Salvar") && !text.contains("salvo"),
                "{msg:?}"
            );
            assert!(
                !text.contains("deletar") && !text.contains("Deletar"),
                "{msg:?}"
            );
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            SchMsg::EditSchematic,
            SchMsg::StopEditing,
            SchMsg::ToolLabel,
            SchMsg::Paste,
            SchMsg::NoSymbols,
            SchMsg::LiveErc,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
