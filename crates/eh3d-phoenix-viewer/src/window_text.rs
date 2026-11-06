// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the common parts of the window (search, measures, layer presets, history, appearance,
//! keyboard shortcuts, help and preferences) in the eight languages of the suite: English, Spanish,
//! German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the common parts of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinText {
    /// The title of the search box and the name of its button.
    SearchTitle,
    /// What can be typed in the search box.
    SearchHint,
    /// The line shown when nothing matches.
    SearchNothing,
    /// The word after the number of results.
    SearchResults,
    /// The word for a component in a list of results.
    KindComponent,
    /// The word for a net in a list of results.
    KindNet,
    /// The word for a pad or a pin in a list of results.
    KindPad,
    /// The title of the list of measures.
    MeasureTitle,
    /// The line shown while there is no measure.
    MeasureNone,
    /// The button that deletes one entry of a list.
    Delete,
    /// The button that deletes every measure.
    MeasureClearAll,
    /// The box that turns the snap of the measure on and off.
    MeasureSnap,
    /// The label of the unit of the measures.
    MeasureUnit,
    /// The label of the angle between the last two measures.
    MeasureAngleBetween,
    /// The title of the layer presets.
    PresetsTitle,
    /// The label of the name of a new preset.
    PresetName,
    /// The button that saves the layers as a preset.
    PresetSave,
    /// The button that applies a preset.
    PresetApply,
    /// The line shown while no preset is saved.
    PresetNone,
    /// The note shown when the name of a preset is empty or has a line break.
    PresetBadName,
    /// The note shown when no more presets can be saved.
    PresetFull,
    /// The button that shows only the active layer.
    Solo,
    /// The button that highlights the net of the selection.
    HighlightNet,
    /// The label of the opacity of a layer.
    Opacity,
    /// The title of the list of steps that were done.
    HistoryTitle,
    /// The line shown while nothing was done.
    HistoryEmpty,
    /// The mark of the place where the document is in its history.
    HistoryNow,
    /// The line that says older steps are not shown.
    HistoryOlder,
    /// A step: a net was added; the name goes where the braces are.
    OpAddNet,
    /// A step: a net was removed.
    OpRemoveNet,
    /// A step: a net was renamed.
    OpRenameNet,
    /// A step: a component was added; the reference goes where the braces are.
    OpAddComponent,
    /// A step: a component was removed.
    OpRemoveComponent,
    /// A step: a component was moved or turned.
    OpMoveComponent,
    /// A step: the fields of a component were changed.
    OpEditComponent,
    /// A step: a pad was given a net or none.
    OpConnectPad,
    /// A step: a track was drawn; the net goes where the braces are.
    OpAddTrack,
    /// A step: a track was removed.
    OpRemoveTrack,
    /// A step: a via was added.
    OpAddVia,
    /// A step: a via was removed.
    OpRemoveVia,
    /// A step: a zone was added.
    OpAddZone,
    /// A step: a zone was removed.
    OpRemoveZone,
    /// A step: a footprint was added to the library of the board.
    OpAddFootprint,
    /// A step: a footprint was removed from the library of the board.
    OpRemoveFootprint,
    /// A step: the stack of copper layers was set.
    OpCopperStack,
    /// A step: the outline of the board was replaced.
    OpSetOutline,
    /// A step: the openings inside the outline were replaced; the number of openings goes where
    /// the braces are.
    OpSetOutlineCutouts,
    /// A step: a track with an arc was drawn; the net goes where the braces are.
    OpArcTrack,
    /// A step: tracks were pushed aside or reshaped; the net goes where the braces are.
    OpShoveTracks,
    /// A step: a differential pair was routed; the first net goes where the braces are.
    OpRoutePair,
    /// A step: several nets were routed by the autorouter; their number goes where the braces
    /// are.
    OpAutoroute,
    /// A step: a zone was changed (its outline, cutouts or properties); the net goes where the
    /// braces are.
    OpChangeZone,
    /// A step: something was added to the silkscreen.
    OpAddSilk,
    /// A step: something was removed from the silkscreen.
    OpRemoveSilk,
    /// A step made of several; the first name goes where the braces are.
    OpBatch,
    /// A step: the board was updated from the schematic.
    OpSyncToBoard,
    /// A step: the schematic was updated from the board.
    OpSyncToSchematic,
    /// A step whose name is not known.
    OpUnknown,
    /// The title of the appearance box.
    AppearanceTitle,
    /// The label of the theme.
    ThemeLabel,
    /// The dark theme.
    ThemeDark,
    /// The light theme.
    ThemeLight,
    /// The theme of high contrast.
    ThemeContrast,
    /// The label of the scale of the interface.
    UiScale,
    /// The label of the size of the letters.
    FontSize,
    /// The title of the list of keyboard shortcuts.
    ShortcutsTitle,
    /// How the shortcuts are changed.
    ShortcutsHowTo,
    /// The title of the help box.
    HelpTitle,
    /// The hint about the help key.
    HelpHint,
    /// The button that closes a box.
    Close,
    /// The note shown when the preferences could not be saved.
    PrefsSaveFailed,
    /// The note shown when the file of the preferences is not valid.
    PrefsInvalid,
    /// The shortcut that turns the selected component.
    ActRotate,
    /// The shortcut that turns the selected component backwards.
    ActRotateBack,
    /// The shortcut that deletes the selected track.
    ActDelete,
    /// The shortcut that widens the selected track.
    ActWidthUp,
    /// The shortcut that narrows the selected track.
    ActWidthDown,
    /// The shortcut that goes to the next result.
    ActNext,
    /// The shortcut that goes to the previous result.
    ActPrevious,
    /// What is shown for an action with no key.
    NoKey,
}

impl WinText {
    /// Every text, for the tests that check the table.
    pub const ALL: [WinText; 80] = [
        WinText::SearchTitle,
        WinText::SearchHint,
        WinText::SearchNothing,
        WinText::SearchResults,
        WinText::KindComponent,
        WinText::KindNet,
        WinText::KindPad,
        WinText::MeasureTitle,
        WinText::MeasureNone,
        WinText::Delete,
        WinText::MeasureClearAll,
        WinText::MeasureSnap,
        WinText::MeasureUnit,
        WinText::MeasureAngleBetween,
        WinText::PresetsTitle,
        WinText::PresetName,
        WinText::PresetSave,
        WinText::PresetApply,
        WinText::PresetNone,
        WinText::PresetBadName,
        WinText::PresetFull,
        WinText::Solo,
        WinText::HighlightNet,
        WinText::Opacity,
        WinText::HistoryTitle,
        WinText::HistoryEmpty,
        WinText::HistoryNow,
        WinText::HistoryOlder,
        WinText::OpAddNet,
        WinText::OpRemoveNet,
        WinText::OpRenameNet,
        WinText::OpAddComponent,
        WinText::OpRemoveComponent,
        WinText::OpMoveComponent,
        WinText::OpEditComponent,
        WinText::OpConnectPad,
        WinText::OpAddTrack,
        WinText::OpRemoveTrack,
        WinText::OpAddVia,
        WinText::OpRemoveVia,
        WinText::OpAddZone,
        WinText::OpRemoveZone,
        WinText::OpAddFootprint,
        WinText::OpRemoveFootprint,
        WinText::OpCopperStack,
        WinText::OpSetOutline,
        WinText::OpSetOutlineCutouts,
        WinText::OpArcTrack,
        WinText::OpShoveTracks,
        WinText::OpRoutePair,
        WinText::OpAutoroute,
        WinText::OpChangeZone,
        WinText::OpAddSilk,
        WinText::OpRemoveSilk,
        WinText::OpBatch,
        WinText::OpSyncToBoard,
        WinText::OpSyncToSchematic,
        WinText::OpUnknown,
        WinText::AppearanceTitle,
        WinText::ThemeLabel,
        WinText::ThemeDark,
        WinText::ThemeLight,
        WinText::ThemeContrast,
        WinText::UiScale,
        WinText::FontSize,
        WinText::ShortcutsTitle,
        WinText::ShortcutsHowTo,
        WinText::HelpTitle,
        WinText::HelpHint,
        WinText::Close,
        WinText::PrefsSaveFailed,
        WinText::PrefsInvalid,
        WinText::ActRotate,
        WinText::ActRotateBack,
        WinText::ActDelete,
        WinText::ActWidthUp,
        WinText::ActWidthDown,
        WinText::ActNext,
        WinText::ActPrevious,
        WinText::NoKey,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            WinText::SearchTitle => [
                "Search",
                "Buscar",
                "Suchen",
                "Rechercher",
                "Cerca",
                "検索",
                "搜索",
                "Pesquisar",
            ],
            WinText::SearchHint => [
                "Reference, value, footprint, net or pin (U1.5)",
                "Referencia, valor, huella, red o pin (U1.5)",
                "Referenz, Wert, Footprint, Netz oder Pin (U1.5)",
                "Référence, valeur, empreinte, net ou broche (U1.5)",
                "Riferimento, valore, impronta, rete o pin (U1.5)",
                "リファレンス、値、フットプリント、ネット、ピン（U1.5）",
                "位号、值、封装、网络或引脚（U1.5）",
                "Referência, valor, pegada, rede ou pino (U1.5)",
            ],
            WinText::SearchNothing => [
                "No results",
                "Sin resultados",
                "Keine Treffer",
                "Aucun résultat",
                "Nessun risultato",
                "結果なし",
                "无结果",
                "Sem resultados",
            ],
            WinText::SearchResults => [
                "results",
                "resultados",
                "Treffer",
                "résultats",
                "risultati",
                "件",
                "个结果",
                "resultados",
            ],
            WinText::KindComponent => [
                "Component",
                "Componente",
                "Bauteil",
                "Composant",
                "Componente",
                "部品",
                "元件",
                "Componente",
            ],
            WinText::KindNet => [
                "Net",
                "Red",
                "Netz",
                "Net",
                "Rete",
                "ネット",
                "网络",
                "Rede",
            ],
            WinText::KindPad => [
                "Pad / pin",
                "Pad / pin",
                "Pad / Pin",
                "Pastille / broche",
                "Pad / pin",
                "パッド / ピン",
                "焊盘 / 引脚",
                "Pad / pino",
            ],
            WinText::MeasureTitle => [
                "Measures",
                "Medidas",
                "Messungen",
                "Mesures",
                "Misure",
                "測定",
                "测量",
                "Medições",
            ],
            WinText::MeasureNone => [
                "No measures yet: choose the measure tool and click two points.",
                "Aún no hay medidas: elija la herramienta de medida y pulse dos puntos.",
                "Noch keine Messungen: Messwerkzeug wählen und zwei Punkte anklicken.",
                "Aucune mesure : choisissez l'outil de mesure et cliquez deux points.",
                "Nessuna misura: scegli lo strumento di misura e fai clic su due punti.",
                "測定はまだありません。測定ツールを選び、2点をクリックしてください。",
                "尚无测量：选择测量工具并点击两个点。",
                "Ainda não há medições: escolha a ferramenta de medição e clique em dois pontos.",
            ],
            WinText::Delete => [
                "Delete",
                "Borrar",
                "Löschen",
                "Supprimer",
                "Elimina",
                "削除",
                "删除",
                "Apagar",
            ],
            WinText::MeasureClearAll => [
                "Delete all",
                "Borrar todas",
                "Alle löschen",
                "Tout supprimer",
                "Elimina tutte",
                "すべて削除",
                "全部删除",
                "Apagar todas",
            ],
            WinText::MeasureSnap => [
                "Snap to pads, ends and corners",
                "Ajustar a pads, extremos y esquinas",
                "An Pads, Enden und Ecken einrasten",
                "Accrocher aux pastilles, extrémités et coins",
                "Aggancia a pad, estremi e angoli",
                "パッド・端点・角にスナップ",
                "吸附到焊盘、端点和角点",
                "Ajustar a pads, extremos e cantos",
            ],
            WinText::MeasureUnit => [
                "Unit", "Unidad", "Einheit", "Unité", "Unità", "単位", "单位", "Unidade",
            ],
            WinText::MeasureAngleBetween => [
                "Angle between the last two",
                "Ángulo entre las dos últimas",
                "Winkel zwischen den letzten zwei",
                "Angle entre les deux dernières",
                "Angolo tra le ultime due",
                "直近2つの間の角度",
                "最后两条之间的夹角",
                "Ângulo entre as duas últimas",
            ],
            WinText::PresetsTitle => [
                "Layer presets",
                "Combinaciones de capas",
                "Ebenen-Vorgaben",
                "Préréglages de couches",
                "Preset dei livelli",
                "レイヤープリセット",
                "图层预设",
                "Predefinições de camadas",
            ],
            WinText::PresetName => [
                "Name of the preset",
                "Nombre de la combinación",
                "Name der Vorgabe",
                "Nom du préréglage",
                "Nome del preset",
                "プリセット名",
                "预设名称",
                "Nome da predefinição",
            ],
            WinText::PresetSave => [
                "Save",
                "Guardar",
                "Speichern",
                "Enregistrer",
                "Salva",
                "保存",
                "保存",
                "Guardar",
            ],
            WinText::PresetApply => [
                "Apply",
                "Aplicar",
                "Anwenden",
                "Appliquer",
                "Applica",
                "適用",
                "应用",
                "Aplicar",
            ],
            WinText::PresetNone => [
                "No presets saved: the keys 1 to 9 apply the first nine.",
                "No hay combinaciones guardadas: las teclas 1 a 9 aplican las nueve primeras.",
                "Keine Vorgaben gespeichert: die Tasten 1 bis 9 wenden die ersten neun an.",
                "Aucun préréglage enregistré : les touches 1 à 9 appliquent les neuf premiers.",
                "Nessun preset salvato: i tasti da 1 a 9 applicano i primi nove.",
                "保存されたプリセットはありません。キー 1〜9 で最初の 9 件を適用します。",
                "没有已保存的预设：按键 1 到 9 应用前九个。",
                "Não há predefinições guardadas: as teclas 1 a 9 aplicam as primeiras nove.",
            ],
            WinText::PresetBadName => [
                "A preset needs a name on one line.",
                "Una combinación necesita un nombre de una sola línea.",
                "Eine Vorgabe braucht einen Namen in einer Zeile.",
                "Un préréglage a besoin d'un nom sur une ligne.",
                "Un preset ha bisogno di un nome su una riga.",
                "プリセットには 1 行の名前が必要です。",
                "预设需要单行名称。",
                "Uma predefinição precisa de um nome numa só linha.",
            ],
            WinText::PresetFull => [
                "Too many presets: delete one first.",
                "Demasiadas combinaciones: borre una primero.",
                "Zu viele Vorgaben: zuerst eine löschen.",
                "Trop de préréglages : supprimez-en un d'abord.",
                "Troppi preset: eliminane prima uno.",
                "プリセットが多すぎます。先に 1 つ削除してください。",
                "预设过多：请先删除一个。",
                "Demasiadas predefinições: apague primeiro uma.",
            ],
            WinText::Solo => [
                "Only the active layer",
                "Solo la capa activa",
                "Nur die aktive Ebene",
                "Seulement la couche active",
                "Solo il livello attivo",
                "アクティブなレイヤーのみ",
                "仅活动图层",
                "Só a camada ativa",
            ],
            WinText::HighlightNet => [
                "Highlight net",
                "Resaltar red",
                "Netz hervorheben",
                "Surligner le net",
                "Evidenzia rete",
                "ネットを強調",
                "高亮网络",
                "Realçar rede",
            ],
            WinText::Opacity => [
                "Opacity",
                "Opacidad",
                "Deckkraft",
                "Opacité",
                "Opacità",
                "不透明度",
                "不透明度",
                "Opacidade",
            ],
            WinText::HistoryTitle => [
                "History",
                "Historial",
                "Verlauf",
                "Historique",
                "Cronologia",
                "履歴",
                "历史",
                "Histórico",
            ],
            WinText::HistoryEmpty => [
                "Nothing has been done yet.",
                "Todavía no se ha hecho nada.",
                "Es wurde noch nichts getan.",
                "Rien n'a encore été fait.",
                "Non è stato ancora fatto nulla.",
                "まだ何も行っていません。",
                "尚未执行任何操作。",
                "Ainda não foi feito nada.",
            ],
            WinText::HistoryNow => [
                "now",
                "ahora",
                "jetzt",
                "maintenant",
                "ora",
                "現在",
                "当前",
                "agora",
            ],
            WinText::HistoryOlder => [
                "older steps are not shown",
                "los pasos más antiguos no se muestran",
                "ältere Schritte werden nicht angezeigt",
                "les étapes plus anciennes ne sont pas affichées",
                "i passaggi più vecchi non sono mostrati",
                "古い手順は表示されません",
                "较早的步骤未显示",
                "os passos mais antigos não são mostrados",
            ],
            WinText::OpAddNet => [
                "Add net {}",
                "Añadir red {}",
                "Netz {} hinzufügen",
                "Ajouter le net {}",
                "Aggiungi rete {}",
                "ネット {} を追加",
                "添加网络 {}",
                "Adicionar rede {}",
            ],
            WinText::OpRemoveNet => [
                "Remove net {}",
                "Quitar la red {}",
                "Netz {} entfernen",
                "Supprimer le net {}",
                "Rimuovi rete {}",
                "ネット {} を削除",
                "删除网络 {}",
                "Remover rede {}",
            ],
            WinText::OpRenameNet => [
                "Rename net {}",
                "Renombrar la red {}",
                "Netz {} umbenennen",
                "Renommer le net {}",
                "Rinomina rete {}",
                "ネット {} の名前を変更",
                "重命名网络 {}",
                "Mudar o nome da rede {}",
            ],
            WinText::OpAddComponent => [
                "Add {}",
                "Añadir {}",
                "{} hinzufügen",
                "Ajouter {}",
                "Aggiungi {}",
                "{} を追加",
                "添加 {}",
                "Adicionar {}",
            ],
            WinText::OpRemoveComponent => [
                "Remove {}",
                "Quitar {}",
                "{} entfernen",
                "Supprimer {}",
                "Rimuovi {}",
                "{} を削除",
                "删除 {}",
                "Remover {}",
            ],
            WinText::OpMoveComponent => [
                "Move {}",
                "Mover {}",
                "{} verschieben",
                "Déplacer {}",
                "Sposta {}",
                "{} を移動",
                "移动 {}",
                "Mover {}",
            ],
            WinText::OpEditComponent => [
                "Edit {}",
                "Editar {}",
                "{} bearbeiten",
                "Modifier {}",
                "Modifica {}",
                "{} を編集",
                "编辑 {}",
                "Editar {}",
            ],
            WinText::OpConnectPad => [
                "Connect pad {}",
                "Conectar el pad {}",
                "Pad {} verbinden",
                "Connecter la pastille {}",
                "Collega il pad {}",
                "パッド {} を接続",
                "连接焊盘 {}",
                "Ligar o pad {}",
            ],
            WinText::OpAddTrack => [
                "Route net {}",
                "Trazar red {}",
                "Netz {} routen",
                "Router le net {}",
                "Instrada la rete {}",
                "ネット {} を配線",
                "布线网络 {}",
                "Encaminhar a rede {}",
            ],
            WinText::OpRemoveTrack => [
                "Delete track of {}",
                "Borrar pista de {}",
                "Leiterbahn von {} löschen",
                "Supprimer la piste de {}",
                "Elimina la pista di {}",
                "{} のトラックを削除",
                "删除 {} 的走线",
                "Apagar a pista de {}",
            ],
            WinText::OpAddVia => [
                "Add via on {}",
                "Añadir vía en {}",
                "Via auf {} hinzufügen",
                "Ajouter un via sur {}",
                "Aggiungi via su {}",
                "{} にビアを追加",
                "在 {} 上添加过孔",
                "Adicionar via em {}",
            ],
            WinText::OpRemoveVia => [
                "Remove via of {}",
                "Quitar vía de {}",
                "Via von {} entfernen",
                "Supprimer le via de {}",
                "Rimuovi via di {}",
                "{} のビアを削除",
                "删除 {} 的过孔",
                "Remover via de {}",
            ],
            WinText::OpAddZone => [
                "Add zone of {}",
                "Añadir zona de {}",
                "Fläche von {} hinzufügen",
                "Ajouter une zone de {}",
                "Aggiungi zona di {}",
                "{} のゾーンを追加",
                "添加 {} 的敷铜区",
                "Adicionar zona de {}",
            ],
            WinText::OpRemoveZone => [
                "Remove zone of {}",
                "Quitar zona de {}",
                "Fläche von {} entfernen",
                "Supprimer la zone de {}",
                "Rimuovi zona di {}",
                "{} のゾーンを削除",
                "删除 {} 的敷铜区",
                "Remover zona de {}",
            ],
            WinText::OpAddFootprint => [
                "Add footprint {}",
                "Añadir huella {}",
                "Footprint {} hinzufügen",
                "Ajouter l'empreinte {}",
                "Aggiungi impronta {}",
                "フットプリント {} を追加",
                "添加封装 {}",
                "Adicionar pegada {}",
            ],
            WinText::OpRemoveFootprint => [
                "Remove footprint {}",
                "Quitar huella {}",
                "Footprint {} entfernen",
                "Supprimer l'empreinte {}",
                "Rimuovi impronta {}",
                "フットプリント {} を削除",
                "删除封装 {}",
                "Remover pegada {}",
            ],
            WinText::OpCopperStack => [
                "Change the copper stack",
                "Cambiar el apilado de cobre",
                "Kupferlagenaufbau ändern",
                "Modifier l'empilement du cuivre",
                "Modifica lo stack del rame",
                "銅層の構成を変更",
                "更改铜层叠层",
                "Alterar o empilhamento do cobre",
            ],
            WinText::OpSetOutline => [
                "Change the board outline",
                "Cambiar el contorno de la placa",
                "Platinenumriss ändern",
                "Modifier le contour de la carte",
                "Modifica il contorno della scheda",
                "基板の外形を変更",
                "更改电路板轮廓",
                "Alterar o contorno da placa",
            ],
            WinText::OpSetOutlineCutouts => [
                "Change the openings inside the outline ({})",
                "Cambiar las aberturas dentro del contorno ({})",
                "Öffnungen im Umriss ändern ({})",
                "Modifier les ouvertures dans le contour ({})",
                "Modifica le aperture nel contorno ({})",
                "外形内の開口部を変更（{}）",
                "更改轮廓内的开口（{}）",
                "Alterar as aberturas dentro do contorno ({})",
            ],
            WinText::OpArcTrack => [
                "Draw an arc on net {}",
                "Dibujar un arco en la red {}",
                "Bogen im Netz {} zeichnen",
                "Dessiner un arc sur le réseau {}",
                "Disegna un arco sulla rete {}",
                "ネット {} に円弧を描く",
                "在网络 {} 上绘制圆弧",
                "Desenhar um arco na rede {}",
            ],
            WinText::OpShoveTracks => [
                "Push tracks aside on net {}",
                "Apartar pistas en la red {}",
                "Leiterbahnen im Netz {} verschieben",
                "Pousser des pistes sur le réseau {}",
                "Sposta le piste sulla rete {}",
                "ネット {} のパターンを押しのける",
                "推开网络 {} 的走线",
                "Afastar pistas na rede {}",
            ],
            WinText::OpRoutePair => [
                "Route a pair ({})",
                "Rutear un par ({})",
                "Paar routen ({})",
                "Router une paire ({})",
                "Instrada una coppia ({})",
                "ペアを配線（{}）",
                "布线差分对（{}）",
                "Encaminhar um par ({})",
            ],
            WinText::OpAutoroute => [
                "Autoroute {} nets",
                "Autorrutear {} redes",
                "{} Netze automatisch routen",
                "Autorouter {} réseaux",
                "Autoinstrada {} reti",
                "{} 本のネットを自動配線",
                "自动布线 {} 个网络",
                "Autoencaminhar {} redes",
            ],
            WinText::OpChangeZone => [
                "Change the zone of net {}",
                "Cambiar la zona de la red {}",
                "Zone des Netzes {} ändern",
                "Modifier la zone du réseau {}",
                "Modifica la zona della rete {}",
                "ネット {} のゾーンを変更",
                "更改网络 {} 的敷铜区",
                "Alterar a zona da rede {}",
            ],
            WinText::OpAddSilk => [
                "Add silkscreen",
                "Añadir serigrafía",
                "Bestückungsdruck hinzufügen",
                "Ajouter de la sérigraphie",
                "Aggiungi serigrafia",
                "シルクを追加",
                "添加丝印",
                "Adicionar serigrafia",
            ],
            WinText::OpRemoveSilk => [
                "Remove silkscreen",
                "Quitar serigrafía",
                "Bestückungsdruck entfernen",
                "Supprimer de la sérigraphie",
                "Rimuovi serigrafia",
                "シルクを削除",
                "删除丝印",
                "Remover serigrafia",
            ],
            WinText::OpBatch => [
                "{} (several changes)",
                "{} (varios cambios)",
                "{} (mehrere Änderungen)",
                "{} (plusieurs modifications)",
                "{} (più modifiche)",
                "{}（複数の変更）",
                "{}（多项更改）",
                "{} (várias alterações)",
            ],
            WinText::OpSyncToBoard => [
                "Update the board from the schematic",
                "Actualizar la placa desde el esquema",
                "Platine aus dem Schaltplan aktualisieren",
                "Mettre à jour la carte depuis le schéma",
                "Aggiorna la scheda dallo schema",
                "回路図から基板を更新",
                "根据原理图更新电路板",
                "Atualizar a placa a partir do esquema",
            ],
            WinText::OpSyncToSchematic => [
                "Update the schematic from the board",
                "Actualizar el esquema desde la placa",
                "Schaltplan aus der Platine aktualisieren",
                "Mettre à jour le schéma depuis la carte",
                "Aggiorna lo schema dalla scheda",
                "基板から回路図を更新",
                "根据电路板更新原理图",
                "Atualizar o esquema a partir da placa",
            ],
            WinText::OpUnknown => [
                "Edit",
                "Edición",
                "Bearbeitung",
                "Modification",
                "Modifica",
                "編集",
                "编辑",
                "Edição",
            ],
            WinText::AppearanceTitle => [
                "Appearance",
                "Apariencia",
                "Darstellung",
                "Apparence",
                "Aspetto",
                "外観",
                "外观",
                "Aspeto",
            ],
            WinText::ThemeLabel => [
                "Theme",
                "Tema",
                "Design",
                "Thème",
                "Tema",
                "テーマ",
                "主题",
                "Tema",
            ],
            WinText::ThemeDark => [
                "Dark",
                "Oscuro",
                "Dunkel",
                "Sombre",
                "Scuro",
                "ダーク",
                "深色",
                "Escuro",
            ],
            WinText::ThemeLight => [
                "Light",
                "Claro",
                "Hell",
                "Clair",
                "Chiaro",
                "ライト",
                "浅色",
                "Claro",
            ],
            WinText::ThemeContrast => [
                "High contrast",
                "Alto contraste",
                "Hoher Kontrast",
                "Contraste élevé",
                "Alto contrasto",
                "ハイコントラスト",
                "高对比度",
                "Alto contraste",
            ],
            WinText::UiScale => [
                "Interface scale",
                "Escala de la interfaz",
                "Skalierung der Oberfläche",
                "Échelle de l'interface",
                "Scala dell'interfaccia",
                "インターフェースの倍率",
                "界面缩放",
                "Escala da interface",
            ],
            WinText::FontSize => [
                "Font size",
                "Tamaño de letra",
                "Schriftgröße",
                "Taille de la police",
                "Dimensione del carattere",
                "フォントサイズ",
                "字体大小",
                "Tamanho do tipo de letra",
            ],
            WinText::ShortcutsTitle => [
                "Keyboard shortcuts",
                "Atajos de teclado",
                "Tastenkürzel",
                "Raccourcis clavier",
                "Scorciatoie da tastiera",
                "キーボードショートカット",
                "键盘快捷键",
                "Atalhos de teclado",
            ],
            WinText::ShortcutsHowTo => [
                "Change them in config/viewer.cfg with lines like key.undo = ctrl+z",
                "Se cambian en config/viewer.cfg con líneas como key.undo = ctrl+z",
                "Änderbar in config/viewer.cfg mit Zeilen wie key.undo = ctrl+z",
                "Modifiables dans config/viewer.cfg avec des lignes comme key.undo = ctrl+z",
                "Si cambiano in config/viewer.cfg con righe come key.undo = ctrl+z",
                "config/viewer.cfg の key.undo = ctrl+z のような行で変更できます",
                "可在 config/viewer.cfg 中用 key.undo = ctrl+z 这样的行修改",
                "Alteram-se em config/viewer.cfg com linhas como key.undo = ctrl+z",
            ],
            WinText::HelpTitle => [
                "Help",
                "Ayuda",
                "Hilfe",
                "Aide",
                "Aiuto",
                "ヘルプ",
                "帮助",
                "Ajuda",
            ],
            WinText::HelpHint => [
                "F1: help of this mode",
                "F1: ayuda de este modo",
                "F1: Hilfe zu diesem Modus",
                "F1 : aide de ce mode",
                "F1: aiuto di questa modalità",
                "F1：このモードのヘルプ",
                "F1：当前模式的帮助",
                "F1: ajuda deste modo",
            ],
            WinText::Close => [
                "Close",
                "Cerrar",
                "Schließen",
                "Fermer",
                "Chiudi",
                "閉じる",
                "关闭",
                "Fechar",
            ],
            WinText::PrefsSaveFailed => [
                "The preferences could not be saved",
                "No se pudieron guardar las preferencias",
                "Die Einstellungen konnten nicht gespeichert werden",
                "Les préférences n'ont pas pu être enregistrées",
                "Non è stato possibile salvare le preferenze",
                "設定を保存できませんでした",
                "无法保存偏好设置",
                "Não foi possível guardar as preferências",
            ],
            WinText::PrefsInvalid => [
                "The file of preferences is not valid and was not used",
                "El archivo de preferencias no es válido y no se usó",
                "Die Einstellungsdatei ist ungültig und wurde nicht verwendet",
                "Le fichier de préférences n'est pas valide et n'a pas été utilisé",
                "Il file delle preferenze non è valido e non è stato usato",
                "設定ファイルが正しくないため使用しませんでした",
                "偏好设置文件无效，未使用",
                "O ficheiro de preferências não é válido e não foi usado",
            ],
            WinText::ActRotate => [
                "Turn the selected component",
                "Girar el componente seleccionado",
                "Gewähltes Bauteil drehen",
                "Tourner le composant sélectionné",
                "Ruota il componente selezionato",
                "選択した部品を回転",
                "旋转所选元件",
                "Rodar o componente selecionado",
            ],
            WinText::ActRotateBack => [
                "Turn it backwards",
                "Girarlo hacia atrás",
                "Rückwärts drehen",
                "Tourner en sens inverse",
                "Ruota all'indietro",
                "逆方向に回転",
                "反向旋转",
                "Rodar para trás",
            ],
            WinText::ActDelete => [
                "Delete the selected track",
                "Borrar la pista seleccionada",
                "Gewählte Leiterbahn löschen",
                "Supprimer la piste sélectionnée",
                "Elimina la pista selezionata",
                "選択したトラックを削除",
                "删除所选走线",
                "Apagar a pista selecionada",
            ],
            WinText::ActWidthUp => [
                "Wider track",
                "Pista más ancha",
                "Breitere Leiterbahn",
                "Piste plus large",
                "Pista più larga",
                "トラックを太く",
                "加宽走线",
                "Pista mais larga",
            ],
            WinText::ActWidthDown => [
                "Narrower track",
                "Pista más estrecha",
                "Schmalere Leiterbahn",
                "Piste plus étroite",
                "Pista più stretta",
                "トラックを細く",
                "收窄走线",
                "Pista mais estreita",
            ],
            WinText::ActNext => [
                "Next result",
                "Resultado siguiente",
                "Nächster Treffer",
                "Résultat suivant",
                "Risultato successivo",
                "次の結果",
                "下一个结果",
                "Resultado seguinte",
            ],
            WinText::ActPrevious => [
                "Previous result",
                "Resultado anterior",
                "Vorheriger Treffer",
                "Résultat précédent",
                "Risultato precedente",
                "前の結果",
                "上一个结果",
                "Resultado anterior",
            ],
            WinText::NoKey => [
                "(no key)",
                "(sin tecla)",
                "(keine Taste)",
                "(aucune touche)",
                "(nessun tasto)",
                "（キーなし）",
                "（无按键）",
                "(sem tecla)",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in WinText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in WinText::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        for text in WinText::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar", "Salvar", "deletar"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
        assert!(WinText::PrefsSaveFailed.tr(Lang::Pt).contains("guardar"));
        assert!(WinText::PrefsInvalid.tr(Lang::Pt).contains("ficheiro"));
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in WinText::ALL.iter().enumerate() {
            for b in &WinText::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn the_steps_that_name_a_thing_have_a_place_for_the_name_in_every_language() {
        for text in [
            WinText::OpAddNet,
            WinText::OpRemoveNet,
            WinText::OpRenameNet,
            WinText::OpAddComponent,
            WinText::OpRemoveComponent,
            WinText::OpMoveComponent,
            WinText::OpEditComponent,
            WinText::OpConnectPad,
            WinText::OpAddTrack,
            WinText::OpRemoveTrack,
            WinText::OpAddVia,
            WinText::OpRemoveVia,
            WinText::OpAddZone,
            WinText::OpRemoveZone,
            WinText::OpAddFootprint,
            WinText::OpRemoveFootprint,
            WinText::OpSetOutlineCutouts,
            WinText::OpArcTrack,
            WinText::OpShoveTracks,
            WinText::OpRoutePair,
            WinText::OpAutoroute,
            WinText::OpChangeZone,
            WinText::OpBatch,
        ] {
            for lang in Lang::ALL {
                assert!(text.tr(lang).contains("{}"), "{text:?} {lang:?}");
            }
        }
    }
}
