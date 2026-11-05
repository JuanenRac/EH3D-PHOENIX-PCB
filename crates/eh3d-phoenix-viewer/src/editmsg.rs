// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the editing in the window in the eight languages of the suite: English, Spanish,
//! German, French, Italian, Japanese, Chinese and European Portuguese. The texts of the viewer
//! that existed before editing are in [`crate::msg`].

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the editing. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMsg {
    /// The tool that drags a component.
    ToolMove,
    /// The tool that turns a component.
    ToolRotate,
    /// The tool that deletes a track.
    ToolDeleteTrack,
    /// The tool that drags a vertex of a track.
    ToolMoveVertex,
    /// The tool that changes the width of a track.
    ToolWidth,
    /// The tool that adds a via.
    ToolVia,
    /// The tool that routes a track between two elements.
    ToolRoute,
    /// The button that undoes the last edit.
    Undo,
    /// The button that redoes the last undone edit.
    Redo,
    /// The button that saves under another name.
    SaveAs,
    /// The title of the group of editing tools.
    EditTitle,
    /// The label of the width given to a track, in millimetres.
    ActiveWidth,
    /// The route mode that stops in front of an obstacle.
    ModeStop,
    /// The route mode that goes around obstacles.
    ModeAround,
    /// The title of the list of findings of the rules.
    RuleFindings,
    /// The title of the findings that the last edit added.
    Added,
    /// The title of the findings that the last edit removed.
    Removed,
    /// The word for an empty list.
    NoFindings,
    /// The button that loads a rule file.
    LoadRules,
    /// The label of the path of the rule file.
    RulesPath,
    /// The title of the connections still to be routed.
    Ratsnest,
    /// The box that shows or hides the ratsnest.
    ShowRatsnest,
    /// The word for connections not routed.
    Unrouted,
    /// The title of the list of nets.
    NetList,
    /// The button that goes to the place of a finding.
    GoTo,
    /// The note that the document has changes that are not saved.
    Unsaved,
    /// The label of the path to save to.
    SavePath,
    /// The word for an error.
    SevError,
    /// The word for a warning.
    SevWarning,
    /// The word for information.
    SevInfo,
    /// The title of the electrical rule check.
    ErcPanel,
    /// The button that runs the electrical rule check again.
    RunErc,
    /// The title of the group that compares a schematic with a board.
    SyncPanel,
    /// The label of the board that the schematic is compared with.
    CompanionBoard,
    /// The button that opens that board.
    LoadBoard,
    /// The button that plans the transfer to the board.
    TransferToBoard,
    /// The button that plans the annotation back into the schematic.
    BackAnnotate,
    /// The title of the package of changes that a dry run made.
    PreviewTitle,
    /// The button that applies the package.
    ApplyPackage,
    /// The button that throws the package away.
    DiscardPackage,
    /// The note that the board and the schematic already agree.
    InSync,
    /// The note that a component was moved.
    NoteMoved,
    /// The note that a component was turned.
    NoteRotated,
    /// The note that a track was deleted.
    NoteTrackDeleted,
    /// The note that a vertex was moved.
    NoteVertexMoved,
    /// The note that a width was changed.
    NoteWidthSet,
    /// The note that a via was added.
    NoteViaAdded,
    /// The note that a track was routed.
    NoteRouted,
    /// The note that a track stopped in front of an obstacle.
    NoteRouteStopped,
    /// The note that an edit was undone.
    NoteUndone,
    /// The note that an edit was redone.
    NoteRedone,
    /// The note that there is nothing to undo.
    NoteNothingToUndo,
    /// The note that there is nothing to redo.
    NoteNothingToRedo,
    /// The note that the rules were loaded.
    NoteRulesLoaded,
    /// The note that a file was saved.
    NoteSaved,
    /// The note that a copy of the previous file was kept.
    NoteBackup,
    /// The note that a package was applied.
    NotePackageApplied,
    /// The note that a dry run is ready and nothing has changed yet.
    NotePackageReady,
    /// The hint to click on copper that has a net.
    NotePickNet,
    /// The hint to click on a track.
    NotePickTrack,
    /// The hint to click on a component.
    NotePickComponent,
    /// The hint to click the origin of a route.
    NoteRouteFrom,
    /// The hint to click the destination of a route.
    NoteRouteTo,
    /// The note that a tool edits boards only.
    NoteBoardOnly,
    /// The note that the board to compare with has to be opened first.
    NoteNeedCompanion,
    /// The note that a finding has no place on the drawing.
    NoteNoPlace,
    /// The line of keyboard shortcuts.
    ShortcutsHint,
}

/// The rows of the table: the variant and the text in each language.
const TABLE: [(EditMsg, [&str; 8]); EditMsg::COUNT] = [
    (
        EditMsg::ToolMove,
        [
            "Move",
            "Mover",
            "Verschieben",
            "Déplacer",
            "Sposta",
            "移動",
            "移动",
            "Mover",
        ],
    ),
    (
        EditMsg::ToolRotate,
        [
            "Rotate", "Girar", "Drehen", "Pivoter", "Ruota", "回転", "旋转", "Rodar",
        ],
    ),
    (
        EditMsg::ToolDeleteTrack,
        [
            "Delete track",
            "Borrar pista",
            "Leiterbahn löschen",
            "Supprimer la piste",
            "Elimina pista",
            "配線を削除",
            "删除走线",
            "Apagar pista",
        ],
    ),
    (
        EditMsg::ToolMoveVertex,
        [
            "Move vertex",
            "Mover vértice",
            "Eckpunkt verschieben",
            "Déplacer le sommet",
            "Sposta vertice",
            "頂点を移動",
            "移动顶点",
            "Mover vértice",
        ],
    ),
    (
        EditMsg::ToolWidth,
        [
            "Width",
            "Ancho",
            "Breite",
            "Largeur",
            "Larghezza",
            "幅",
            "线宽",
            "Largura",
        ],
    ),
    (
        EditMsg::ToolVia,
        ["Via", "Vía", "Via", "Via", "Via", "ビア", "过孔", "Via"],
    ),
    (
        EditMsg::ToolRoute,
        [
            "Route",
            "Enrutar",
            "Routen",
            "Router",
            "Instrada",
            "配線",
            "布线",
            "Encaminhar",
        ],
    ),
    (
        EditMsg::Undo,
        [
            "Undo",
            "Deshacer",
            "Rückgängig",
            "Annuler",
            "Annulla",
            "元に戻す",
            "撤销",
            "Anular",
        ],
    ),
    (
        EditMsg::Redo,
        [
            "Redo",
            "Rehacer",
            "Wiederholen",
            "Rétablir",
            "Ripeti",
            "やり直す",
            "重做",
            "Refazer",
        ],
    ),
    (
        EditMsg::SaveAs,
        [
            "Save as",
            "Guardar como",
            "Speichern unter",
            "Enregistrer sous",
            "Salva con nome",
            "名前を付けて保存",
            "另存为",
            "Guardar como",
        ],
    ),
    (
        EditMsg::EditTitle,
        [
            "Edit",
            "Editar",
            "Bearbeiten",
            "Modifier",
            "Modifica",
            "編集",
            "编辑",
            "Editar",
        ],
    ),
    (
        EditMsg::ActiveWidth,
        [
            "Track width (mm)",
            "Ancho de pista (mm)",
            "Bahnbreite (mm)",
            "Largeur de piste (mm)",
            "Larghezza pista (mm)",
            "配線幅 (mm)",
            "走线宽度 (mm)",
            "Largura da pista (mm)",
        ],
    ),
    (
        EditMsg::ModeStop,
        [
            "Stop at obstacles",
            "Parar ante obstáculos",
            "Vor Hindernissen anhalten",
            "S'arrêter devant les obstacles",
            "Fermati davanti agli ostacoli",
            "障害物の手前で停止",
            "遇障碍物停止",
            "Parar perante obstáculos",
        ],
    ),
    (
        EditMsg::ModeAround,
        [
            "Go around obstacles",
            "Rodear obstáculos",
            "Hindernisse umgehen",
            "Contourner les obstacles",
            "Aggira gli ostacoli",
            "障害物を迂回",
            "绕过障碍物",
            "Contornar obstáculos",
        ],
    ),
    (
        EditMsg::RuleFindings,
        [
            "Rule findings",
            "Hallazgos de las reglas",
            "Regelbefunde",
            "Constats des règles",
            "Rilievi delle regole",
            "ルールの検出結果",
            "规则检查结果",
            "Achados das regras",
        ],
    ),
    (
        EditMsg::Added,
        [
            "Added by the last edit",
            "Añadidos por la última edición",
            "Von der letzten Bearbeitung hinzugefügt",
            "Ajoutés par la dernière modification",
            "Aggiunti dall'ultima modifica",
            "直前の編集で追加",
            "上次编辑新增",
            "Adicionados pela última edição",
        ],
    ),
    (
        EditMsg::Removed,
        [
            "Resolved by the last edit",
            "Resueltos por la última edición",
            "Von der letzten Bearbeitung behoben",
            "Résolus par la dernière modification",
            "Risolti dall'ultima modifica",
            "直前の編集で解消",
            "上次编辑已解决",
            "Resolvidos pela última edição",
        ],
    ),
    (
        EditMsg::NoFindings,
        [
            "none", "ninguno", "keine", "aucun", "nessuno", "なし", "无", "nenhum",
        ],
    ),
    (
        EditMsg::LoadRules,
        [
            "Load rules",
            "Cargar reglas",
            "Regeln laden",
            "Charger les règles",
            "Carica regole",
            "ルールを読み込む",
            "加载规则",
            "Carregar regras",
        ],
    ),
    (
        EditMsg::RulesPath,
        [
            "Rule file",
            "Archivo de reglas",
            "Regeldatei",
            "Fichier de règles",
            "File delle regole",
            "ルールファイル",
            "规则文件",
            "Ficheiro de regras",
        ],
    ),
    (
        EditMsg::Ratsnest,
        [
            "Ratsnest",
            "Líneas de conexión pendientes",
            "Luftlinien",
            "Chevelu",
            "Connessioni da instradare",
            "未配線の接続",
            "飞线",
            "Ligações por encaminhar",
        ],
    ),
    (
        EditMsg::ShowRatsnest,
        [
            "Show the ratsnest",
            "Mostrar las conexiones pendientes",
            "Luftlinien zeigen",
            "Afficher le chevelu",
            "Mostra le connessioni da instradare",
            "未配線の接続を表示",
            "显示飞线",
            "Mostrar as ligações por encaminhar",
        ],
    ),
    (
        EditMsg::Unrouted,
        [
            "unrouted",
            "sin enrutar",
            "nicht geroutet",
            "non routés",
            "non instradati",
            "未配線",
            "未布线",
            "por encaminhar",
        ],
    ),
    (
        EditMsg::NetList,
        [
            "Nets (click to highlight)",
            "Redes (clic para resaltar)",
            "Netze (Klick zum Hervorheben)",
            "Réseaux (clic pour surligner)",
            "Reti (clic per evidenziare)",
            "ネット（クリックで強調）",
            "网络（单击高亮）",
            "Redes (clique para realçar)",
        ],
    ),
    (
        EditMsg::GoTo,
        [
            "Go to",
            "Ir a",
            "Gehe zu",
            "Aller à",
            "Vai a",
            "移動先",
            "转到",
            "Ir para",
        ],
    ),
    (
        EditMsg::Unsaved,
        [
            "unsaved changes",
            "cambios sin guardar",
            "ungespeicherte Änderungen",
            "modifications non enregistrées",
            "modifiche non salvate",
            "未保存の変更",
            "未保存的更改",
            "alterações por guardar",
        ],
    ),
    (
        EditMsg::SavePath,
        [
            "Save to",
            "Guardar en",
            "Speichern in",
            "Enregistrer dans",
            "Salva in",
            "保存先",
            "保存到",
            "Guardar em",
        ],
    ),
    (
        EditMsg::SevError,
        [
            "error",
            "error",
            "Fehler",
            "erreur",
            "errore",
            "エラー",
            "错误",
            "erro",
        ],
    ),
    (
        EditMsg::SevWarning,
        [
            "warning",
            "advertencia",
            "Warnung",
            "avertissement",
            "avviso",
            "警告",
            "警告",
            "aviso",
        ],
    ),
    (
        EditMsg::SevInfo,
        [
            "info",
            "información",
            "Info",
            "information",
            "informazione",
            "情報",
            "信息",
            "informação",
        ],
    ),
    (
        EditMsg::ErcPanel,
        [
            "Electrical rule check",
            "Comprobación eléctrica de reglas",
            "Elektrische Regelprüfung",
            "Vérification électrique des règles",
            "Controllo elettrico delle regole",
            "電気的ルールチェック",
            "电气规则检查",
            "Verificação elétrica de regras",
        ],
    ),
    (
        EditMsg::RunErc,
        [
            "Check again",
            "Comprobar de nuevo",
            "Erneut prüfen",
            "Revérifier",
            "Ricontrolla",
            "再チェック",
            "重新检查",
            "Verificar de novo",
        ],
    ),
    (
        EditMsg::SyncPanel,
        [
            "Board and schematic",
            "Placa y esquema",
            "Platine und Schaltplan",
            "Carte et schéma",
            "Scheda e schema",
            "基板と回路図",
            "电路板与原理图",
            "Placa e esquema",
        ],
    ),
    (
        EditMsg::CompanionBoard,
        [
            "Board to compare with",
            "Placa con la que comparar",
            "Platine zum Vergleich",
            "Carte à comparer",
            "Scheda da confrontare",
            "比較する基板",
            "要比较的电路板",
            "Placa a comparar",
        ],
    ),
    (
        EditMsg::LoadBoard,
        [
            "Open board",
            "Abrir placa",
            "Platine öffnen",
            "Ouvrir la carte",
            "Apri scheda",
            "基板を開く",
            "打开电路板",
            "Abrir placa",
        ],
    ),
    (
        EditMsg::TransferToBoard,
        [
            "Transfer to the board (dry run)",
            "Transferir a la placa (simulacro)",
            "Auf die Platine übertragen (Probelauf)",
            "Transférer vers la carte (simulation)",
            "Trasferisci alla scheda (simulazione)",
            "基板へ転送（ドライラン）",
            "传送到电路板（试运行）",
            "Transferir para a placa (simulação)",
        ],
    ),
    (
        EditMsg::BackAnnotate,
        [
            "Back-annotate (dry run)",
            "Anotación hacia atrás (simulacro)",
            "Rückannotation (Probelauf)",
            "Rétro-annotation (simulation)",
            "Annotazione inversa (simulazione)",
            "バックアノテーション（ドライラン）",
            "反向标注（试运行）",
            "Anotação para trás (simulação)",
        ],
    ),
    (
        EditMsg::PreviewTitle,
        [
            "Package of changes",
            "Paquete de cambios",
            "Änderungspaket",
            "Paquet de modifications",
            "Pacchetto di modifiche",
            "変更パッケージ",
            "更改包",
            "Pacote de alterações",
        ],
    ),
    (
        EditMsg::ApplyPackage,
        [
            "Apply the package",
            "Aplicar el paquete",
            "Paket anwenden",
            "Appliquer le paquet",
            "Applica il pacchetto",
            "パッケージを適用",
            "应用更改包",
            "Aplicar o pacote",
        ],
    ),
    (
        EditMsg::DiscardPackage,
        [
            "Discard",
            "Descartar",
            "Verwerfen",
            "Abandonner",
            "Scarta",
            "破棄",
            "放弃",
            "Descartar",
        ],
    ),
    (
        EditMsg::InSync,
        [
            "The board and the schematic agree",
            "La placa y el esquema coinciden",
            "Platine und Schaltplan stimmen überein",
            "La carte et le schéma concordent",
            "Scheda e schema coincidono",
            "基板と回路図は一致しています",
            "电路板与原理图一致",
            "A placa e o esquema coincidem",
        ],
    ),
    (
        EditMsg::NoteMoved,
        [
            "Component moved",
            "Componente movido",
            "Bauteil verschoben",
            "Composant déplacé",
            "Componente spostato",
            "部品を移動しました",
            "元件已移动",
            "Componente movido",
        ],
    ),
    (
        EditMsg::NoteRotated,
        [
            "Component turned",
            "Componente girado",
            "Bauteil gedreht",
            "Composant pivoté",
            "Componente ruotato",
            "部品を回転しました",
            "元件已旋转",
            "Componente rodado",
        ],
    ),
    (
        EditMsg::NoteTrackDeleted,
        [
            "Track deleted",
            "Pista borrada",
            "Leiterbahn gelöscht",
            "Piste supprimée",
            "Pista eliminata",
            "配線を削除しました",
            "走线已删除",
            "Pista apagada",
        ],
    ),
    (
        EditMsg::NoteVertexMoved,
        [
            "Vertex moved",
            "Vértice movido",
            "Eckpunkt verschoben",
            "Sommet déplacé",
            "Vertice spostato",
            "頂点を移動しました",
            "顶点已移动",
            "Vértice movido",
        ],
    ),
    (
        EditMsg::NoteWidthSet,
        [
            "Width changed",
            "Ancho cambiado",
            "Breite geändert",
            "Largeur modifiée",
            "Larghezza modificata",
            "幅を変更しました",
            "线宽已更改",
            "Largura alterada",
        ],
    ),
    (
        EditMsg::NoteViaAdded,
        [
            "Via added",
            "Vía añadida",
            "Via hinzugefügt",
            "Via ajouté",
            "Via aggiunto",
            "ビアを追加しました",
            "已添加过孔",
            "Via adicionado",
        ],
    ),
    (
        EditMsg::NoteRouted,
        [
            "Track routed",
            "Pista enrutada",
            "Leiterbahn geroutet",
            "Piste routée",
            "Pista instradata",
            "配線しました",
            "走线已布好",
            "Pista encaminhada",
        ],
    ),
    (
        EditMsg::NoteRouteStopped,
        [
            "The track stopped in front of an obstacle",
            "La pista se detuvo ante un obstáculo",
            "Die Leiterbahn hielt vor einem Hindernis an",
            "La piste s'est arrêtée devant un obstacle",
            "La pista si è fermata davanti a un ostacolo",
            "配線は障害物の手前で止まりました",
            "走线在障碍物前停止",
            "A pista parou perante um obstáculo",
        ],
    ),
    (
        EditMsg::NoteUndone,
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
        EditMsg::NoteRedone,
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
        EditMsg::NoteNothingToUndo,
        [
            "Nothing to undo",
            "Nada que deshacer",
            "Nichts zum Rückgängigmachen",
            "Rien à annuler",
            "Niente da annullare",
            "元に戻す操作はありません",
            "没有可撤销的操作",
            "Nada para anular",
        ],
    ),
    (
        EditMsg::NoteNothingToRedo,
        [
            "Nothing to redo",
            "Nada que rehacer",
            "Nichts zum Wiederholen",
            "Rien à rétablir",
            "Niente da ripetere",
            "やり直す操作はありません",
            "没有可重做的操作",
            "Nada para refazer",
        ],
    ),
    (
        EditMsg::NoteRulesLoaded,
        [
            "Rules loaded",
            "Reglas cargadas",
            "Regeln geladen",
            "Règles chargées",
            "Regole caricate",
            "ルールを読み込みました",
            "规则已加载",
            "Regras carregadas",
        ],
    ),
    (
        EditMsg::NoteSaved,
        [
            "Saved",
            "Guardado",
            "Gespeichert",
            "Enregistré",
            "Salvato",
            "保存しました",
            "已保存",
            "Guardado",
        ],
    ),
    (
        EditMsg::NoteBackup,
        [
            "copy of the previous file",
            "copia del archivo anterior",
            "Kopie der vorherigen Datei",
            "copie du fichier précédent",
            "copia del file precedente",
            "以前のファイルのコピー",
            "上一个文件的副本",
            "cópia do ficheiro anterior",
        ],
    ),
    (
        EditMsg::NotePackageApplied,
        [
            "Package applied",
            "Paquete aplicado",
            "Paket angewendet",
            "Paquet appliqué",
            "Pacchetto applicato",
            "パッケージを適用しました",
            "更改包已应用",
            "Pacote aplicado",
        ],
    ),
    (
        EditMsg::NotePackageReady,
        [
            "Dry run done: nothing has been changed yet",
            "Simulacro hecho: aún no se ha cambiado nada",
            "Probelauf fertig: noch nichts geändert",
            "Simulation faite : rien n'a encore été modifié",
            "Simulazione eseguita: non è stato ancora modificato nulla",
            "ドライラン完了：まだ何も変更していません",
            "试运行完成：尚未更改任何内容",
            "Simulação feita: ainda nada foi alterado",
        ],
    ),
    (
        EditMsg::NotePickNet,
        [
            "Click on copper that has a net",
            "Haga clic en cobre que tenga red",
            "Auf Kupfer mit einem Netz klicken",
            "Cliquez sur du cuivre qui a un réseau",
            "Fai clic su rame che ha una rete",
            "ネットのある銅箔をクリック",
            "请单击带网络的铜箔",
            "Clique em cobre que tenha rede",
        ],
    ),
    (
        EditMsg::NotePickTrack,
        [
            "Click on a track",
            "Haga clic en una pista",
            "Auf eine Leiterbahn klicken",
            "Cliquez sur une piste",
            "Fai clic su una pista",
            "配線をクリック",
            "请单击走线",
            "Clique numa pista",
        ],
    ),
    (
        EditMsg::NotePickComponent,
        [
            "Click on a component",
            "Haga clic en un componente",
            "Auf ein Bauteil klicken",
            "Cliquez sur un composant",
            "Fai clic su un componente",
            "部品をクリック",
            "请单击元件",
            "Clique num componente",
        ],
    ),
    (
        EditMsg::NoteRouteFrom,
        [
            "Click the origin: a pad, a via or the end of a track",
            "Haga clic en el origen: un pad, una vía o el extremo de una pista",
            "Ausgangspunkt anklicken: ein Pad, ein Via oder das Ende einer Leiterbahn",
            "Cliquez sur l'origine : un pad, un via ou l'extrémité d'une piste",
            "Fai clic sull'origine: un pad, un via o l'estremità di una pista",
            "始点をクリック：パッド、ビア、または配線の端",
            "请单击起点：焊盘、过孔或走线端点",
            "Clique na origem: um pad, um via ou a extremidade de uma pista",
        ],
    ),
    (
        EditMsg::NoteRouteTo,
        [
            "Now click the destination",
            "Ahora haga clic en el destino",
            "Jetzt das Ziel anklicken",
            "Cliquez maintenant sur la destination",
            "Ora fai clic sulla destinazione",
            "次に終点をクリック",
            "现在请单击终点",
            "Agora clique no destino",
        ],
    ),
    (
        EditMsg::NoteBoardOnly,
        [
            "This tool edits boards",
            "Esta herramienta edita placas",
            "Dieses Werkzeug bearbeitet Platinen",
            "Cet outil modifie les cartes",
            "Questo strumento modifica le schede",
            "このツールは基板を編集します",
            "此工具用于编辑电路板",
            "Esta ferramenta edita placas",
        ],
    ),
    (
        EditMsg::NoteNeedCompanion,
        [
            "Open the board to compare with first",
            "Abra primero la placa con la que comparar",
            "Zuerst die Vergleichsplatine öffnen",
            "Ouvrez d'abord la carte à comparer",
            "Apri prima la scheda da confrontare",
            "先に比較する基板を開いてください",
            "请先打开要比较的电路板",
            "Abra primeiro a placa a comparar",
        ],
    ),
    (
        EditMsg::NoteNoPlace,
        [
            "This finding has no place on the drawing",
            "Este hallazgo no tiene lugar en el dibujo",
            "Dieser Befund hat keine Stelle in der Zeichnung",
            "Ce constat n'a pas d'emplacement sur le dessin",
            "Questo rilievo non ha una posizione nel disegno",
            "この検出結果は図面上に位置がありません",
            "此结果在图上没有位置",
            "Este achado não tem lugar no desenho",
        ],
    ),
    (
        EditMsg::ShortcutsHint,
        [
            "Ctrl+Z undo · Ctrl+Y redo · R rotate · Del delete · Esc cancel",
            "Ctrl+Z deshacer · Ctrl+Y rehacer · R girar · Supr borrar · Esc cancelar",
            "Strg+Z rückgängig · Strg+Y wiederholen · R drehen · Entf löschen · Esc abbrechen",
            "Ctrl+Z annuler · Ctrl+Y rétablir · R pivoter · Suppr supprimer · Échap annuler",
            "Ctrl+Z annulla · Ctrl+Y ripeti · R ruota · Canc elimina · Esc annulla",
            "Ctrl+Z 元に戻す · Ctrl+Y やり直す · R 回転 · Del 削除 · Esc 中止",
            "Ctrl+Z 撤销 · Ctrl+Y 重做 · R 旋转 · Del 删除 · Esc 取消",
            "Ctrl+Z anular · Ctrl+Y refazer · R rodar · Del apagar · Esc cancelar",
        ],
    ),
];

impl EditMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = EditMsg::ShortcutsHint as usize + 1;

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
        assert_eq!(TABLE.len(), EditMsg::COUNT);
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
        assert_eq!(EditMsg::LoadRules.tr(Lang::Pt), "Carregar regras");
        assert!(EditMsg::RulesPath.tr(Lang::Pt).contains("Ficheiro"));
        assert!(EditMsg::NoteBackup.tr(Lang::Pt).contains("ficheiro"));
        assert_eq!(EditMsg::SaveAs.tr(Lang::Pt), "Guardar como");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(
                !text.contains("Salvar") && !text.contains("salvo"),
                "{msg:?}"
            );
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            EditMsg::Undo,
            EditMsg::Redo,
            EditMsg::LoadRules,
            EditMsg::NoteSaved,
            EditMsg::NoteRouteTo,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
