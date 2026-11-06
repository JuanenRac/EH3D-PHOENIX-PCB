// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the window that shows imported Gerber and Excellon files, in the eight languages
//! of the suite: English, Spanish, German, French, Italian, Japanese, Chinese and European
//! Portuguese.

use eh3d_phoenix_gerber::Role;
use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the window of the imported files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GerberText {
    /// The title of the window.
    Title,
    /// The note that the files are only read.
    ReadOnly,
    /// The button that closes the window.
    Close,
    /// The label of the folder.
    Folder,
    /// The title of the list of files.
    Files,
    /// The label of the opacity of a layer.
    Opacity,
    /// The button that draws a layer later (above the others).
    MoveUp,
    /// The button that draws a layer earlier (below the others).
    MoveDown,
    /// The word for the dark objects of a layer.
    Dark,
    /// The word for the clear objects of a layer.
    Clear,
    /// The word for round holes.
    Holes,
    /// The word for slots.
    Slots,
    /// The note of a file that could not be read.
    NotRead,
    /// The note of a file in which something that changes the picture was skipped.
    Incomplete,
    /// The note of a set cut at the limit of polygons.
    Truncated,
    /// The note that a clear area erases what lies under it in the drawing.
    ClearNote,
    /// The title of the measure.
    Measure,
    /// The hint of the measure.
    MeasureHint,
    /// The box that makes a click snap to a corner or a centre.
    Snap,
    /// The title of the comparison with the board.
    Compare,
    /// The label of the board file.
    BoardFile,
    /// The button that makes the comparison.
    CompareNow,
    /// The title of the list of differences.
    Differences,
    /// The words for what only the files have.
    OnlyFiles,
    /// The words for what only the board has.
    OnlyBoard,
    /// The word for a layer that agrees with the board.
    Identical,
    /// The box that shows the differences on the drawing.
    ShowDifferences,
    /// The note of a layer without a layer of the board with the same role.
    NoMatch,
    /// The label of the shift that was applied to the board.
    Shift,
    /// The word for the size of a cell of the comparison.
    CellSize,
    /// The word for a file whose layer is not known.
    Unknown,
    /// The word for the files that were left aside.
    SkippedFiles,
    /// The role: copper, top.
    RoleCopperTop,
    /// The role: copper, bottom.
    RoleCopperBottom,
    /// The role: inner copper.
    RoleCopperInner,
    /// The role: solder mask, top.
    RoleMaskTop,
    /// The role: solder mask, bottom.
    RoleMaskBottom,
    /// The role: paste, top.
    RolePasteTop,
    /// The role: paste, bottom.
    RolePasteBottom,
    /// The role: silkscreen, top.
    RoleLegendTop,
    /// The role: silkscreen, bottom.
    RoleLegendBottom,
    /// The role: outline.
    RoleOutline,
    /// The role: plated holes.
    RoleDrillPlated,
    /// The role: holes that are not plated.
    RoleDrillNonPlated,
    /// The role: holes.
    RoleDrill,
}

/// The text of the role of a layer.
pub fn role_text(role: Role) -> GerberText {
    match role {
        Role::CopperTop => GerberText::RoleCopperTop,
        Role::CopperBottom => GerberText::RoleCopperBottom,
        Role::CopperInner(_) => GerberText::RoleCopperInner,
        Role::MaskTop => GerberText::RoleMaskTop,
        Role::MaskBottom => GerberText::RoleMaskBottom,
        Role::PasteTop => GerberText::RolePasteTop,
        Role::PasteBottom => GerberText::RolePasteBottom,
        Role::LegendTop => GerberText::RoleLegendTop,
        Role::LegendBottom => GerberText::RoleLegendBottom,
        Role::Outline => GerberText::RoleOutline,
        Role::DrillPlated => GerberText::RoleDrillPlated,
        Role::DrillNonPlated => GerberText::RoleDrillNonPlated,
        Role::Drill => GerberText::RoleDrill,
    }
}

/// The name of the layer of a file in a language: its role (an inner layer with its number), or
/// the word for a layer that is not known.
pub fn role_title(role: Option<Role>, lang: Lang) -> String {
    match role {
        None => GerberText::Unknown.tr(lang).to_owned(),
        Some(Role::CopperInner(n)) => format!("{} {n}", GerberText::RoleCopperInner.tr(lang)),
        Some(other) => role_text(other).tr(lang).to_owned(),
    }
}

impl GerberText {
    /// Every text, for the tests that check the table.
    pub const ALL: [GerberText; 45] = [
        GerberText::Title,
        GerberText::ReadOnly,
        GerberText::Close,
        GerberText::Folder,
        GerberText::Files,
        GerberText::Opacity,
        GerberText::MoveUp,
        GerberText::MoveDown,
        GerberText::Dark,
        GerberText::Clear,
        GerberText::Holes,
        GerberText::Slots,
        GerberText::NotRead,
        GerberText::Incomplete,
        GerberText::Truncated,
        GerberText::ClearNote,
        GerberText::Measure,
        GerberText::MeasureHint,
        GerberText::Snap,
        GerberText::Compare,
        GerberText::BoardFile,
        GerberText::CompareNow,
        GerberText::Differences,
        GerberText::OnlyFiles,
        GerberText::OnlyBoard,
        GerberText::Identical,
        GerberText::ShowDifferences,
        GerberText::NoMatch,
        GerberText::Shift,
        GerberText::CellSize,
        GerberText::Unknown,
        GerberText::SkippedFiles,
        GerberText::RoleCopperTop,
        GerberText::RoleCopperBottom,
        GerberText::RoleCopperInner,
        GerberText::RoleMaskTop,
        GerberText::RoleMaskBottom,
        GerberText::RolePasteTop,
        GerberText::RolePasteBottom,
        GerberText::RoleLegendTop,
        GerberText::RoleLegendBottom,
        GerberText::RoleOutline,
        GerberText::RoleDrillPlated,
        GerberText::RoleDrillNonPlated,
        GerberText::RoleDrill,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            GerberText::Title => [
                "Gerber and Excellon files",
                "Archivos Gerber y Excellon",
                "Gerber- und Excellon-Dateien",
                "Fichiers Gerber et Excellon",
                "File Gerber ed Excellon",
                "Gerber / Excellon ファイル",
                "Gerber 和 Excellon 文件",
                "Ficheiros Gerber e Excellon",
            ],
            GerberText::ReadOnly => [
                "Read-only view: the files are never changed.",
                "Vista de solo lectura: los archivos nunca se modifican.",
                "Nur-Lese-Ansicht: Die Dateien werden nie verändert.",
                "Vue en lecture seule : les fichiers ne sont jamais modifiés.",
                "Vista di sola lettura: i file non vengono mai modificati.",
                "読み取り専用の表示です。ファイルは変更されません。",
                "只读视图：文件永远不会被修改。",
                "Vista só de leitura: os ficheiros nunca são alterados.",
            ],
            GerberText::Close => [
                "Close", "Cerrar", "Schließen", "Fermer", "Chiudi", "閉じる", "关闭", "Fechar",
            ],
            GerberText::Folder => [
                "Folder",
                "Carpeta",
                "Ordner",
                "Dossier",
                "Cartella",
                "フォルダー",
                "文件夹",
                "Pasta",
            ],
            GerberText::Files => [
                "Files",
                "Archivos",
                "Dateien",
                "Fichiers",
                "File",
                "ファイル",
                "文件",
                "Ficheiros",
            ],
            GerberText::Opacity => [
                "Opacity",
                "Opacidad",
                "Deckkraft",
                "Opacité",
                "Opacità",
                "不透明度",
                "不透明度",
                "Opacidade",
            ],
            GerberText::MoveUp => [
                "Up", "Subir", "Nach oben", "Monter", "Su", "上へ", "上移", "Subir",
            ],
            GerberText::MoveDown => [
                "Down",
                "Bajar",
                "Nach unten",
                "Descendre",
                "Giù",
                "下へ",
                "下移",
                "Descer",
            ],
            GerberText::Dark => [
                "dark", "oscuros", "dunkel", "sombres", "scuri", "ダーク", "暗", "escuros",
            ],
            GerberText::Clear => [
                "clear", "claros", "hell", "clairs", "chiari", "クリア", "亮", "claros",
            ],
            GerberText::Holes => [
                "holes", "agujeros", "Bohrungen", "trous", "fori", "穴", "孔", "furos",
            ],
            GerberText::Slots => [
                "slots",
                "ranuras",
                "Langlöcher",
                "fentes",
                "asole",
                "スロット",
                "槽孔",
                "ranhuras",
            ],
            GerberText::NotRead => [
                "could not be read",
                "no se pudo leer",
                "konnte nicht gelesen werden",
                "n'a pas pu être lu",
                "non è stato possibile leggerlo",
                "読み込めませんでした",
                "无法读取",
                "não foi possível ler",
            ],
            GerberText::Incomplete => [
                "incomplete: something that changes the picture was skipped",
                "incompleto: se omitió algo que cambia el dibujo",
                "unvollständig: etwas, das das Bild ändert, wurde übersprungen",
                "incomplet : un élément qui change le dessin a été ignoré",
                "incompleto: è stato saltato qualcosa che cambia il disegno",
                "不完全：図形を変える部分を読み飛ばしました",
                "不完整：跳过了会改变图形的内容",
                "incompleto: foi omitido algo que altera o desenho",
            ],
            GerberText::Truncated => [
                "The set is cut at the limit of polygons; part of it is not drawn.",
                "El conjunto se corta en el límite de polígonos; una parte no se dibuja.",
                "Der Satz wird an der Polygongrenze abgeschnitten; ein Teil wird nicht gezeichnet.",
                "L'ensemble est coupé à la limite de polygones ; une partie n'est pas dessinée.",
                "L'insieme è tagliato al limite di poligoni; una parte non viene disegnata.",
                "ポリゴンの上限で打ち切ったため、一部は描画されません。",
                "已在多边形上限处截断，部分内容未绘制。",
                "O conjunto é cortado no limite de polígonos; uma parte não é desenhada.",
            ],
            GerberText::ClearNote => [
                "A clear area erases what lies under it in the drawing.",
                "Un área clara borra lo que hay debajo en el dibujo.",
                "Ein heller Bereich löscht in der Zeichnung, was darunter liegt.",
                "Une zone claire efface ce qui se trouve dessous dans le dessin.",
                "Un'area chiara cancella ciò che sta sotto nel disegno.",
                "クリア領域は、描画上その下にあるものを消します。",
                "亮区域会在图中擦除其下方的内容。",
                "Uma área clara apaga o que está por baixo no desenho.",
            ],
            GerberText::Measure => [
                "Measure",
                "Medir",
                "Messen",
                "Mesurer",
                "Misura",
                "測定",
                "测量",
                "Medir",
            ],
            GerberText::MeasureHint => [
                "Click two points; a click near a corner or a centre snaps to it.",
                "Pulse dos puntos; un clic cerca de una esquina o un centro se ajusta a él.",
                "Zwei Punkte anklicken; ein Klick nahe einer Ecke oder eines Zentrums rastet dort ein.",
                "Cliquez deux points ; un clic près d'un coin ou d'un centre s'y accroche.",
                "Fai clic su due punti; un clic vicino a un angolo o a un centro vi si aggancia.",
                "2点をクリックします。角や中心の近くでは、そこに吸着します。",
                "点击两个点；靠近角点或中心的点击会吸附到该处。",
                "Clique em dois pontos; um clique perto de um canto ou de um centro ajusta-se a ele.",
            ],
            GerberText::Snap => [
                "Snap to corners and centres",
                "Ajustar a esquinas y centros",
                "An Ecken und Zentren einrasten",
                "S'accrocher aux coins et aux centres",
                "Aggancia ad angoli e centri",
                "角と中心に吸着",
                "吸附到角点和中心",
                "Ajustar a cantos e centros",
            ],
            GerberText::Compare => [
                "Compare with the board of the project",
                "Comparar con la placa del proyecto",
                "Mit der Platine des Projekts vergleichen",
                "Comparer avec la carte du projet",
                "Confronta con la scheda del progetto",
                "プロジェクトの基板と比較",
                "与项目的电路板比较",
                "Comparar com a placa do projeto",
            ],
            GerberText::BoardFile => [
                "Board file (.brd or .json)",
                "Archivo de la placa (.brd o .json)",
                "Platinendatei (.brd oder .json)",
                "Fichier de la carte (.brd ou .json)",
                "File della scheda (.brd o .json)",
                "基板ファイル (.brd / .json)",
                "电路板文件 (.brd 或 .json)",
                "Ficheiro da placa (.brd ou .json)",
            ],
            GerberText::CompareNow => [
                "Compare",
                "Comparar",
                "Vergleichen",
                "Comparer",
                "Confronta",
                "比較",
                "比较",
                "Comparar",
            ],
            GerberText::Differences => [
                "Differences",
                "Diferencias",
                "Unterschiede",
                "Différences",
                "Differenze",
                "差異",
                "差异",
                "Diferenças",
            ],
            GerberText::OnlyFiles => [
                "only in the files",
                "solo en los archivos",
                "nur in den Dateien",
                "seulement dans les fichiers",
                "solo nei file",
                "ファイルのみ",
                "仅在文件中",
                "só nos ficheiros",
            ],
            GerberText::OnlyBoard => [
                "only in the board",
                "solo en la placa",
                "nur in der Platine",
                "seulement dans la carte",
                "solo nella scheda",
                "基板のみ",
                "仅在电路板中",
                "só na placa",
            ],
            GerberText::Identical => [
                "identical",
                "idéntica",
                "identisch",
                "identique",
                "identica",
                "一致",
                "相同",
                "idêntica",
            ],
            GerberText::ShowDifferences => [
                "Show the differences",
                "Mostrar las diferencias",
                "Unterschiede anzeigen",
                "Afficher les différences",
                "Mostra le differenze",
                "差異を表示",
                "显示差异",
                "Mostrar as diferenças",
            ],
            GerberText::NoMatch => [
                "no layer of the board has this role",
                "ninguna capa de la placa tiene este papel",
                "keine Lage der Platine hat diese Rolle",
                "aucune couche de la carte n'a ce rôle",
                "nessuno strato della scheda ha questo ruolo",
                "この役割の基板レイヤーがありません",
                "电路板中没有对应此角色的层",
                "nenhuma camada da placa tem esta função",
            ],
            GerberText::Shift => [
                "Shift applied to the board (mm)",
                "Desplazamiento aplicado a la placa (mm)",
                "Auf die Platine angewandte Verschiebung (mm)",
                "Décalage appliqué à la carte (mm)",
                "Spostamento applicato alla scheda (mm)",
                "基板に適用したずれ (mm)",
                "应用于电路板的偏移 (mm)",
                "Deslocamento aplicado à placa (mm)",
            ],
            GerberText::CellSize => [
                "Cell of the comparison (mm)",
                "Celda de la comparación (mm)",
                "Zelle des Vergleichs (mm)",
                "Cellule de la comparaison (mm)",
                "Cella del confronto (mm)",
                "比較のセル (mm)",
                "比较单元格 (mm)",
                "Célula da comparação (mm)",
            ],
            GerberText::Unknown => [
                "layer not known",
                "capa desconocida",
                "Lage unbekannt",
                "couche inconnue",
                "strato sconosciuto",
                "レイヤー不明",
                "未知的层",
                "camada desconhecida",
            ],
            GerberText::SkippedFiles => [
                "files left aside",
                "archivos dejados aparte",
                "beiseitegelegte Dateien",
                "fichiers laissés de côté",
                "file lasciati da parte",
                "除外したファイル",
                "已跳过的文件",
                "ficheiros postos de parte",
            ],
            GerberText::RoleCopperTop => [
                "Copper, top",
                "Cobre, superior",
                "Kupfer, oben",
                "Cuivre, dessus",
                "Rame, sopra",
                "銅箔 (表)",
                "铜层（顶层）",
                "Cobre, topo",
            ],
            GerberText::RoleCopperBottom => [
                "Copper, bottom",
                "Cobre, inferior",
                "Kupfer, unten",
                "Cuivre, dessous",
                "Rame, sotto",
                "銅箔 (裏)",
                "铜层（底层）",
                "Cobre, fundo",
            ],
            GerberText::RoleCopperInner => [
                "Copper, inner",
                "Cobre, interna",
                "Kupfer, innen",
                "Cuivre, interne",
                "Rame, interno",
                "銅箔 (内層)",
                "铜层（内层）",
                "Cobre, interna",
            ],
            GerberText::RoleMaskTop => [
                "Solder mask, top",
                "Máscara de soldadura, superior",
                "Lötstopplack, oben",
                "Masque de soudure, dessus",
                "Maschera di saldatura, sopra",
                "ソルダーマスク (表)",
                "阻焊层（顶层）",
                "Máscara de soldadura, topo",
            ],
            GerberText::RoleMaskBottom => [
                "Solder mask, bottom",
                "Máscara de soldadura, inferior",
                "Lötstopplack, unten",
                "Masque de soudure, dessous",
                "Maschera di saldatura, sotto",
                "ソルダーマスク (裏)",
                "阻焊层（底层）",
                "Máscara de soldadura, fundo",
            ],
            GerberText::RolePasteTop => [
                "Paste, top",
                "Pasta, superior",
                "Lotpaste, oben",
                "Pâte, dessus",
                "Pasta, sopra",
                "ペースト (表)",
                "锡膏层（顶层）",
                "Pasta, topo",
            ],
            GerberText::RolePasteBottom => [
                "Paste, bottom",
                "Pasta, inferior",
                "Lotpaste, unten",
                "Pâte, dessous",
                "Pasta, sotto",
                "ペースト (裏)",
                "锡膏层（底层）",
                "Pasta, fundo",
            ],
            GerberText::RoleLegendTop => [
                "Silkscreen, top",
                "Serigrafía, superior",
                "Bestückungsdruck, oben",
                "Sérigraphie, dessus",
                "Serigrafia, sopra",
                "シルク (表)",
                "丝印（顶层）",
                "Serigrafia, topo",
            ],
            GerberText::RoleLegendBottom => [
                "Silkscreen, bottom",
                "Serigrafía, inferior",
                "Bestückungsdruck, unten",
                "Sérigraphie, dessous",
                "Serigrafia, sotto",
                "シルク (裏)",
                "丝印（底层）",
                "Serigrafia, fundo",
            ],
            GerberText::RoleOutline => [
                "Outline",
                "Contorno",
                "Umriss",
                "Contour",
                "Contorno",
                "外形",
                "板框",
                "Contorno",
            ],
            GerberText::RoleDrillPlated => [
                "Drill, plated",
                "Taladros, metalizados",
                "Bohrungen, durchkontaktiert",
                "Perçages, métallisés",
                "Fori, metallizzati",
                "ドリル (メッキあり)",
                "钻孔（金属化）",
                "Furos, metalizados",
            ],
            GerberText::RoleDrillNonPlated => [
                "Drill, not plated",
                "Taladros, sin metalizar",
                "Bohrungen, nicht durchkontaktiert",
                "Perçages, non métallisés",
                "Fori, non metallizzati",
                "ドリル (メッキなし)",
                "钻孔（非金属化）",
                "Furos, não metalizados",
            ],
            GerberText::RoleDrill => [
                "Drill",
                "Taladros",
                "Bohrungen",
                "Perçages",
                "Fori",
                "ドリル",
                "钻孔",
                "Furos",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in GerberText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in GerberText::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert!(GerberText::Files.tr(Lang::Pt).contains("Ficheiros"));
        assert!(GerberText::BoardFile.tr(Lang::Pt).contains("Ficheiro"));
        for text in GerberText::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar", "pasta de arq"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in GerberText::ALL.iter().enumerate() {
            for b in &GerberText::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn a_layer_is_named_by_its_role_with_the_number_of_an_inner_one() {
        assert_eq!(role_title(Some(Role::CopperTop), Lang::En), "Copper, top");
        assert_eq!(
            role_title(Some(Role::CopperInner(2)), Lang::Es),
            "Cobre, interna 2"
        );
        assert_eq!(role_title(None, Lang::De), "Lage unbekannt");
    }

    #[test]
    fn each_role_has_its_text() {
        assert_eq!(role_text(Role::CopperTop), GerberText::RoleCopperTop);
        assert_eq!(role_text(Role::CopperInner(3)), GerberText::RoleCopperInner);
        assert_eq!(role_text(Role::Outline), GerberText::RoleOutline);
        assert_eq!(role_text(Role::Drill), GerberText::RoleDrill);
        assert_eq!(
            role_text(Role::DrillNonPlated).tr(Lang::En),
            "Drill, not plated"
        );
    }
}
