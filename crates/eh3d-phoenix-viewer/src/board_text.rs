// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the board editor in the window, in the eight languages of the suite: English,
//! Spanish, German, French, Italian, Japanese, Chinese and European Portuguese. The texts of the
//! older edits of a board are in [`crate::editmsg`].

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the board editor. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardMsg {
    /// The title of the panel of the editor.
    EditingTitle,
    /// The button that starts editing the board.
    EditBoard,
    /// The button that goes back to looking at the board.
    StopEditing,
    /// The tool that picks and drags.
    ToolSelect,
    /// The tool that places a component.
    ToolPlace,
    /// The tool that draws tracks.
    ToolRoute,
    /// The tool that puts vias.
    ToolVia,
    /// The tool that deletes.
    ToolDelete,
    /// The tool that tunes the length of a net.
    ToolTune,
    /// The button that turns the selected components.
    Rotate,
    /// The button that puts the selected components on the other side.
    Mirror,
    /// The button that locks or unlocks the selected components.
    LockToggle,
    /// The button that deletes the selection.
    Delete,
    /// The label of the active layer.
    LayerLabel,
    /// The label of the footprint to place.
    FootprintLabel,
    /// The label of the reference of a component.
    RefLabel,
    /// The label of the value of a component.
    ValueLabel,
    /// The label of the length the tune tool asks for.
    TargetLength,
    /// The box that allows 45-degree pieces.
    Diagonal,
    /// The box that makes a corner go first along x.
    CornerFirst,
    /// The label of the grid.
    GridLabel,
    /// The box that turns snapping on.
    SnapToGrid,
    /// The box that turns the rule check after each change on.
    LiveDrc,
    /// The title of the reason an operation was refused.
    Refused,
    /// What follows the number of items selected.
    SelectedCount,
    /// What follows the number of components that are locked.
    LockedCount,
    /// What follows the number of rule errors.
    RuleErrors,
    /// What follows the number of connections still to be routed.
    Unrouted,
    /// The label of the length of the lit net.
    NetLength,
    /// The button that finishes the route in progress.
    FinishRoute,
    /// The hint of the select tool.
    HintSelect,
    /// The hint of the place tool.
    HintPlace,
    /// The hint of the route tool.
    HintRoute,
    /// The hint of the via tool.
    HintVia,
    /// The hint of the delete tool.
    HintDelete,
    /// The hint of the tune tool.
    HintTune,
    /// The keys of the editor.
    KeysHint,
}

const TABLE: [(BoardMsg, [&str; 8]); BoardMsg::COUNT] = [
    (
        BoardMsg::EditingTitle,
        [
            "Board editor",
            "Editor de placas",
            "Platinen-Editor",
            "Éditeur de cartes",
            "Editor di schede",
            "基板エディタ",
            "电路板编辑器",
            "Editor de placas",
        ],
    ),
    (
        BoardMsg::EditBoard,
        [
            "Edit board",
            "Editar placa",
            "Platine bearbeiten",
            "Modifier la carte",
            "Modifica scheda",
            "基板を編集",
            "编辑电路板",
            "Editar placa",
        ],
    ),
    (
        BoardMsg::StopEditing,
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
        BoardMsg::ToolSelect,
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
        BoardMsg::ToolPlace,
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
        BoardMsg::ToolRoute,
        [
            "Route",
            "Trazar pista",
            "Leiterbahn ziehen",
            "Router une piste",
            "Traccia pista",
            "配線",
            "布线",
            "Traçar pista",
        ],
    ),
    (
        BoardMsg::ToolVia,
        [
            "Via",
            "Vía",
            "Via",
            "Via",
            "Via",
            "ビア",
            "过孔",
            "Via",
        ],
    ),
    (
        BoardMsg::ToolDelete,
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
        BoardMsg::ToolTune,
        [
            "Tune length",
            "Ajustar longitud",
            "Länge abgleichen",
            "Ajuster la longueur",
            "Regola lunghezza",
            "長さ調整",
            "调整长度",
            "Ajustar comprimento",
        ],
    ),
    (
        BoardMsg::Rotate,
        [
            "Rotate", "Girar", "Drehen", "Pivoter", "Ruota", "回転", "旋转", "Rodar",
        ],
    ),
    (
        BoardMsg::Mirror,
        [
            "Other side",
            "Otra cara",
            "Andere Seite",
            "Autre face",
            "Altro lato",
            "反対面へ",
            "换面",
            "Outra face",
        ],
    ),
    (
        BoardMsg::LockToggle,
        [
            "Lock / unlock",
            "Bloquear / desbloquear",
            "Sperren / entsperren",
            "Verrouiller / déverrouiller",
            "Blocca / sblocca",
            "ロック / 解除",
            "锁定 / 解锁",
            "Bloquear / desbloquear",
        ],
    ),
    (
        BoardMsg::Delete,
        [
            "Delete selection",
            "Borrar selección",
            "Auswahl löschen",
            "Supprimer la sélection",
            "Elimina selezione",
            "選択を削除",
            "删除所选",
            "Apagar seleção",
        ],
    ),
    (
        BoardMsg::LayerLabel,
        [
            "Layer", "Capa", "Lage", "Couche", "Strato", "層", "层", "Camada",
        ],
    ),
    (
        BoardMsg::FootprintLabel,
        [
            "Footprint",
            "Huella",
            "Footprint",
            "Empreinte",
            "Footprint",
            "フットプリント",
            "封装",
            "Pegada",
        ],
    ),
    (
        BoardMsg::RefLabel,
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
        BoardMsg::ValueLabel,
        [
            "Value", "Valor", "Wert", "Valeur", "Valore", "値", "值", "Valor",
        ],
    ),
    (
        BoardMsg::TargetLength,
        [
            "Target length (mm)",
            "Longitud pedida (mm)",
            "Ziellänge (mm)",
            "Longueur visée (mm)",
            "Lunghezza richiesta (mm)",
            "目標の長さ (mm)",
            "目标长度 (mm)",
            "Comprimento pedido (mm)",
        ],
    ),
    (
        BoardMsg::Diagonal,
        [
            "45-degree pieces",
            "Tramos a 45 grados",
            "45-Grad-Stücke",
            "Tronçons à 45 degrés",
            "Tratti a 45 gradi",
            "45度の線分",
            "45度线段",
            "Troços a 45 graus",
        ],
    ),
    (
        BoardMsg::CornerFirst,
        [
            "Corner along x first",
            "Esquina primero en x",
            "Ecke zuerst entlang x",
            "Coin d'abord selon x",
            "Angolo prima lungo x",
            "角は先にx方向",
            "先沿x方向拐角",
            "Canto primeiro em x",
        ],
    ),
    (
        BoardMsg::GridLabel,
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
        BoardMsg::SnapToGrid,
        [
            "Snap to grid",
            "Ajustar a la rejilla",
            "Am Raster einrasten",
            "Aimanter à la grille",
            "Aggancia alla griglia",
            "グリッドにスナップ",
            "吸附到网格",
            "Ajustar à grelha",
        ],
    ),
    (
        BoardMsg::LiveDrc,
        [
            "Rule check after each change",
            "Comprobar reglas tras cada cambio",
            "Regelprüfung nach jeder Änderung",
            "Vérifier les règles après chaque changement",
            "Verifica regole dopo ogni modifica",
            "変更ごとにルール検査",
            "每次更改后检查规则",
            "Verificar regras após cada alteração",
        ],
    ),
    (
        BoardMsg::Refused,
        [
            "Refused",
            "Rechazado",
            "Abgelehnt",
            "Refusé",
            "Rifiutato",
            "拒否されました",
            "已拒绝",
            "Recusado",
        ],
    ),
    (
        BoardMsg::SelectedCount,
        [
            "selected",
            "seleccionados",
            "ausgewählt",
            "sélectionnés",
            "selezionati",
            "件を選択",
            "项已选",
            "selecionados",
        ],
    ),
    (
        BoardMsg::LockedCount,
        [
            "locked",
            "bloqueados",
            "gesperrt",
            "verrouillés",
            "bloccati",
            "件ロック中",
            "项已锁定",
            "bloqueados",
        ],
    ),
    (
        BoardMsg::RuleErrors,
        [
            "rule errors",
            "errores de reglas",
            "Regelfehler",
            "erreurs de règles",
            "errori di regole",
            "件の規則エラー",
            "个规则错误",
            "erros de regras",
        ],
    ),
    (
        BoardMsg::Unrouted,
        [
            "connections to route",
            "conexiones sin trazar",
            "offene Verbindungen",
            "connexions à router",
            "connessioni da tracciare",
            "件の未配線",
            "条未布线连接",
            "ligações por traçar",
        ],
    ),
    (
        BoardMsg::NetLength,
        [
            "Length of the lit net (mm)",
            "Longitud de la red iluminada (mm)",
            "Länge des markierten Netzes (mm)",
            "Longueur du réseau en surbrillance (mm)",
            "Lunghezza della rete evidenziata (mm)",
            "強調したネットの長さ (mm)",
            "高亮网络的长度 (mm)",
            "Comprimento da rede realçada (mm)",
        ],
    ),
    (
        BoardMsg::FinishRoute,
        [
            "Finish route",
            "Terminar pista",
            "Leiterbahn beenden",
            "Terminer la piste",
            "Termina pista",
            "配線を完了",
            "完成布线",
            "Terminar pista",
        ],
    ),
    (
        BoardMsg::HintSelect,
        [
            "Click to pick, drag a part to move it, drag on empty space to select a rectangle",
            "Clic para elegir, arrastre una pieza para moverla, arrastre en vacío para seleccionar un rectángulo",
            "Klicken zum Wählen, Bauteil ziehen zum Verschieben, auf leerer Fläche ziehen für ein Rechteck",
            "Cliquez pour choisir, faites glisser un composant pour le déplacer, glissez dans le vide pour un rectangle",
            "Clic per scegliere, trascina un componente per spostarlo, trascina nel vuoto per un rettangolo",
            "クリックで選択、部品をドラッグして移動、空白をドラッグで矩形選択",
            "单击选择，拖动元件移动，在空白处拖动可框选",
            "Clique para escolher, arraste um componente para o mover, arraste no vazio para selecionar um retângulo",
        ],
    ),
    (
        BoardMsg::HintPlace,
        [
            "Click to place the chosen footprint",
            "Clic para colocar la huella elegida",
            "Klicken, um das gewählte Footprint zu platzieren",
            "Cliquez pour placer l'empreinte choisie",
            "Clic per posizionare il footprint scelto",
            "クリックで選択したフットプリントを配置",
            "单击放置所选封装",
            "Clique para colocar a pegada escolhida",
        ],
    ),
    (
        BoardMsg::HintRoute,
        [
            "Start on a pad, a track or a via, click each corner, click a pad of the net or press Enter to finish; Space changes the corner",
            "Empiece en un pad, una pista o una vía, pulse cada esquina, pulse un pad de la red o Intro para terminar; Espacio cambia la esquina",
            "Auf einem Pad, einer Bahn oder einem Via beginnen, jede Ecke anklicken, Pad des Netzes anklicken oder Enter zum Beenden; Leertaste wechselt die Ecke",
            "Commencez sur un pad, une piste ou un via, cliquez chaque coin, cliquez un pad du réseau ou Entrée pour finir ; Espace change le coin",
            "Inizia su un pad, una pista o un via, clic su ogni angolo, clic su un pad della rete o Invio per finire; Spazio cambia l'angolo",
            "パッド・配線・ビアから開始し、角ごとにクリック。同じネットのパッドをクリックするかEnterで完了、Spaceで角の向きを切替",
            "从焊盘、走线或过孔开始，逐个拐角单击；单击同网络焊盘或按 Enter 结束；空格键切换拐角方向",
            "Comece num pad, numa pista ou numa via, clique em cada canto, clique num pad da rede ou prima Enter para terminar; Espaço muda o canto",
        ],
    ),
    (
        BoardMsg::HintVia,
        [
            "Click a pad, a track or a via of the net to put a via there",
            "Clic en un pad, una pista o una vía de la red para poner una vía",
            "Pad, Bahn oder Via des Netzes anklicken, um ein Via zu setzen",
            "Cliquez un pad, une piste ou un via du réseau pour y mettre un via",
            "Clic su un pad, una pista o un via della rete per inserire un via",
            "ネットのパッド・配線・ビアをクリックしてビアを追加",
            "单击网络的焊盘、走线或过孔以放置过孔",
            "Clique num pad, numa pista ou numa via da rede para pôr uma via",
        ],
    ),
    (
        BoardMsg::HintDelete,
        [
            "Click a component, a track or a via to delete it",
            "Clic en un componente, una pista o una vía para borrarlo",
            "Bauteil, Bahn oder Via anklicken, um es zu löschen",
            "Cliquez un composant, une piste ou un via pour le supprimer",
            "Clic su un componente, una pista o un via per eliminarlo",
            "部品・配線・ビアをクリックして削除",
            "单击元件、走线或过孔以删除",
            "Clique num componente, numa pista ou numa via para o apagar",
        ],
    ),
    (
        BoardMsg::HintTune,
        [
            "Set the target length, then click a track of the net to add meanders",
            "Fije la longitud pedida y pulse una pista de la red para añadir meandros",
            "Ziellänge einstellen, dann eine Bahn des Netzes anklicken, um Mäander einzufügen",
            "Réglez la longueur visée puis cliquez une piste du réseau pour ajouter des méandres",
            "Imposta la lunghezza richiesta, poi clic su una pista della rete per aggiungere meandri",
            "目標の長さを設定し、ネットの配線をクリックしてミアンダを追加",
            "设置目标长度，然后单击网络的走线以添加蛇形线",
            "Defina o comprimento pedido e clique numa pista da rede para acrescentar meandros",
        ],
    ),
    (
        BoardMsg::KeysHint,
        [
            "V select, P place, T route, I via, D delete, L tune, R turn, X other side, K lock, Del delete, Ctrl+Z/Y undo, redo",
            "V seleccionar, P colocar, T pista, I vía, D borrar, L ajustar, R girar, X otra cara, K bloquear, Supr borrar, Ctrl+Z/Y deshacer, rehacer",
            "V wählen, P platzieren, T Bahn, I Via, D löschen, L abgleichen, R drehen, X andere Seite, K sperren, Entf löschen, Strg+Z/Y rückgängig, wiederholen",
            "V sélectionner, P placer, T piste, I via, D supprimer, L ajuster, R pivoter, X autre face, K verrouiller, Suppr supprimer, Ctrl+Z/Y annuler, rétablir",
            "V seleziona, P posiziona, T pista, I via, D elimina, L regola, R ruota, X altro lato, K blocca, Canc elimina, Ctrl+Z/Y annulla, ripristina",
            "V 選択、P 配置、T 配線、I ビア、D 削除、L 長さ調整、R 回転、X 反対面、K ロック、Del 削除、Ctrl+Z/Y 元に戻す・やり直し",
            "V 选择，P 放置，T 布线，I 过孔，D 删除，L 调整长度，R 旋转，X 换面，K 锁定，Del 删除，Ctrl+Z/Y 撤销、重做",
            "V selecionar, P colocar, T pista, I via, D apagar, L ajustar, R rodar, X outra face, K bloquear, Del apagar, Ctrl+Z/Y anular, refazer",
        ],
    ),
];

impl BoardMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = BoardMsg::KeysHint as usize + 1;

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
        assert_eq!(TABLE.len(), BoardMsg::COUNT);
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
        assert_eq!(BoardMsg::StopEditing.tr(Lang::Pt), "Terminar a edição");
        assert_eq!(BoardMsg::ToolDelete.tr(Lang::Pt), "Apagar");
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
            BoardMsg::EditBoard,
            BoardMsg::StopEditing,
            BoardMsg::ToolTune,
            BoardMsg::LockToggle,
            BoardMsg::LiveDrc,
            BoardMsg::HintRoute,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
