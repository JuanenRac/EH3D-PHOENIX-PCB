// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the zone tool of the board editor in the window, in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the zone tool. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneMsg {
    /// The box that turns the zone tool on.
    Mode,
    /// The label of the net that is chosen.
    NetLabel,
    /// What is said when the board has no net.
    NoNets,
    /// The label of the priority of a new zone.
    Priority,
    /// The button that closes the polygon.
    Close,
    /// The button that makes a zone of the polygon.
    Add,
    /// The button that gives a zone the outline that was drawn.
    ApplyOutline,
    /// The button that cuts a hole of the polygon in the chosen zone.
    ApplyCutout,
    /// The box that makes the polygon a hole.
    CutoutMode,
    /// The button that drops the last point.
    RemoveLast,
    /// The button that drops the points.
    Clear,
    /// The button that fills the zones.
    Fill,
    /// The button that shows the fill without keeping it.
    Preview,
    /// The button that gives a zone more priority.
    Raise,
    /// The button that gives a zone less priority.
    Lower,
    /// The button that removes a zone.
    Remove,
    /// The button that loads the outline of a zone to move its vertices.
    EditVertices,
    /// The hint of the zone tool.
    Hint,
    /// What follows the number of points.
    Points,
    /// What follows the number of polygons of copper.
    Polygons,
    /// What follows the number of islands that were dropped.
    Dropped,
    /// The title of the list of zones.
    Order,
    /// What is said of a zone with no fill.
    NoFill,
}

const TABLE: [(ZoneMsg, [&str; 8]); ZoneMsg::COUNT] = [
    (
        ZoneMsg::Mode,
        [
            "Zone tool",
            "Herramienta Zona",
            "Zonen-Werkzeug",
            "Outil Zone",
            "Strumento Zona",
            "ゾーンツール",
            "区域工具",
            "Ferramenta Zona",
        ],
    ),
    (
        ZoneMsg::NetLabel,
        [
            "Net",
            "Red",
            "Netz",
            "Réseau",
            "Rete",
            "ネット",
            "网络",
            "Rede",
        ],
    ),
    (
        ZoneMsg::NoNets,
        [
            "The board has no nets",
            "La placa no tiene redes",
            "Die Platine hat keine Netze",
            "La carte n'a aucun réseau",
            "La scheda non ha reti",
            "基板にネットがありません",
            "电路板没有网络",
            "A placa não tem redes",
        ],
    ),
    (
        ZoneMsg::Priority,
        [
            "Priority",
            "Prioridad",
            "Priorität",
            "Priorité",
            "Priorità",
            "優先度",
            "优先级",
            "Prioridade",
        ],
    ),
    (
        ZoneMsg::Close,
        [
            "Close polygon",
            "Cerrar polígono",
            "Polygon schließen",
            "Fermer le polygone",
            "Chiudi poligono",
            "多角形を閉じる",
            "闭合多边形",
            "Fechar polígono",
        ],
    ),
    (
        ZoneMsg::Add,
        [
            "Add zone",
            "Añadir zona",
            "Zone hinzufügen",
            "Ajouter la zone",
            "Aggiungi zona",
            "ゾーンを追加",
            "添加区域",
            "Acrescentar zona",
        ],
    ),
    (
        ZoneMsg::ApplyOutline,
        [
            "Apply outline",
            "Aplicar contorno",
            "Umriss übernehmen",
            "Appliquer le contour",
            "Applica contorno",
            "輪郭を適用",
            "应用轮廓",
            "Aplicar contorno",
        ],
    ),
    (
        ZoneMsg::ApplyCutout,
        [
            "Cut hole",
            "Hacer recorte",
            "Ausschnitt schneiden",
            "Faire la découpe",
            "Crea ritaglio",
            "切り抜きを作る",
            "挖空",
            "Fazer recorte",
        ],
    ),
    (
        ZoneMsg::CutoutMode,
        [
            "Polygon is a cutout",
            "El polígono es un recorte",
            "Polygon ist ein Ausschnitt",
            "Le polygone est une découpe",
            "Il poligono è un ritaglio",
            "多角形は切り抜き",
            "多边形为挖空",
            "O polígono é um recorte",
        ],
    ),
    (
        ZoneMsg::RemoveLast,
        [
            "Remove last point",
            "Quitar el último punto",
            "Letzten Punkt entfernen",
            "Retirer le dernier point",
            "Rimuovi l'ultimo punto",
            "最後の点を削除",
            "删除最后一点",
            "Remover o último ponto",
        ],
    ),
    (
        ZoneMsg::Clear,
        [
            "Clear points",
            "Limpiar puntos",
            "Punkte löschen",
            "Effacer les points",
            "Cancella i punti",
            "点を消去",
            "清除各点",
            "Limpar pontos",
        ],
    ),
    (
        ZoneMsg::Fill,
        [
            "Fill zones",
            "Rellenar zonas",
            "Zonen füllen",
            "Remplir les zones",
            "Riempi le zone",
            "ゾーンを塗りつぶす",
            "填充区域",
            "Preencher zonas",
        ],
    ),
    (
        ZoneMsg::Preview,
        [
            "Preview fill",
            "Vista previa del relleno",
            "Vorschau der Füllung",
            "Aperçu du remplissage",
            "Anteprima del riempimento",
            "塗りつぶしのプレビュー",
            "预览填充",
            "Pré-visualizar preenchimento",
        ],
    ),
    (
        ZoneMsg::Raise,
        [
            "Raise priority",
            "Subir prioridad",
            "Priorität erhöhen",
            "Augmenter la priorité",
            "Alza priorità",
            "優先度を上げる",
            "提高优先级",
            "Subir prioridade",
        ],
    ),
    (
        ZoneMsg::Lower,
        [
            "Lower priority",
            "Bajar prioridad",
            "Priorität senken",
            "Baisser la priorité",
            "Abbassa priorità",
            "優先度を下げる",
            "降低优先级",
            "Baixar prioridade",
        ],
    ),
    (
        ZoneMsg::Remove,
        [
            "Remove zone",
            "Quitar zona",
            "Zone entfernen",
            "Retirer la zone",
            "Rimuovi zona",
            "ゾーンを削除",
            "删除区域",
            "Remover zona",
        ],
    ),
    (
        ZoneMsg::EditVertices,
        [
            "Edit vertices",
            "Editar vértices",
            "Eckpunkte bearbeiten",
            "Modifier les sommets",
            "Modifica vertici",
            "頂点を編集",
            "编辑顶点",
            "Editar vértices",
        ],
    ),
    (
        ZoneMsg::Hint,
        [
            "click the corners of the zone and the first point again to close it; drag a corner to move it",
            "pulsa las esquinas de la zona y otra vez el primer punto para cerrarla; arrastra una esquina para moverla",
            "Ecken der Zone anklicken und den ersten Punkt erneut, um zu schließen; eine Ecke ziehen, um sie zu verschieben",
            "cliquez les coins de la zone puis le premier point pour fermer ; glissez un coin pour le déplacer",
            "fai clic sugli angoli della zona e di nuovo sul primo punto per chiudere; trascina un angolo per spostarlo",
            "ゾーンの角をクリックし、最初の点をもう一度クリックして閉じます。角をドラッグで移動",
            "点击区域的各角，再点击第一点以闭合；拖动角点可移动",
            "clique nos cantos da zona e outra vez no primeiro ponto para fechar; arraste um canto para o mover",
        ],
    ),
    (
        ZoneMsg::Points,
        [
            "points",
            "puntos",
            "Punkte",
            "points",
            "punti",
            "点",
            "点",
            "pontos",
        ],
    ),
    (
        ZoneMsg::Polygons,
        [
            "polygons of copper",
            "polígonos de cobre",
            "Kupferpolygone",
            "polygones de cuivre",
            "poligoni di rame",
            "個の銅ポリゴン",
            "个铜多边形",
            "polígonos de cobre",
        ],
    ),
    (
        ZoneMsg::Dropped,
        [
            "islands dropped",
            "islas descartadas",
            "Inseln verworfen",
            "îlots écartés",
            "isole scartate",
            "個の島を破棄",
            "个孤岛被丢弃",
            "ilhas descartadas",
        ],
    ),
    (
        ZoneMsg::Order,
        [
            "Zones by priority",
            "Zonas por prioridad",
            "Zonen nach Priorität",
            "Zones par priorité",
            "Zone per priorità",
            "優先度順のゾーン",
            "按优先级排列的区域",
            "Zonas por prioridade",
        ],
    ),
    (
        ZoneMsg::NoFill,
        [
            "no fill",
            "sin relleno",
            "ohne Füllung",
            "sans remplissage",
            "senza riempimento",
            "塗りつぶしなし",
            "无填充",
            "sem preenchimento",
        ],
    ),
];

impl ZoneMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = ZoneMsg::NoFill as usize + 1;

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
        assert_eq!(TABLE.len(), ZoneMsg::COUNT);
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
        assert_eq!(ZoneMsg::Clear.tr(Lang::Pt), "Limpar pontos");
        assert_eq!(ZoneMsg::Mode.tr(Lang::Pt), "Ferramenta Zona");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("trilha"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            ZoneMsg::Mode,
            ZoneMsg::Priority,
            ZoneMsg::Close,
            ZoneMsg::Add,
            ZoneMsg::Fill,
            ZoneMsg::Preview,
            ZoneMsg::Raise,
            ZoneMsg::Lower,
            ZoneMsg::Remove,
            ZoneMsg::Hint,
            ZoneMsg::Order,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
