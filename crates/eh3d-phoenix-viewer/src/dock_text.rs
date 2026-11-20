// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the panels of the window (the frame of the suite: components, nets, console, report,
//! tasks and library) in the eight languages of the suite: English, Spanish, German, French, Italian,
//! Japanese, Chinese and European Portuguese. The titles of the panels and the names of the workspaces
//! that the frame already has in its own table are not repeated here.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the panels of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockMsg {
    /// The line of a panel that needs a document while nothing is open.
    NoDocument,
    /// The line of a panel that reads the document while an editor of the board or the schematic has it.
    EditorHasDocument,
    /// The line of the console while it holds nothing.
    ConsoleEmpty,
    /// The button that empties the console.
    ConsoleClear,
    /// The hint of the box that filters the list of components.
    ComponentsFilter,
    /// The word after the numbers of a list that shows part of its rows (`12 / 40 shown`).
    Shown,
    /// The title of the counts of the objects of the document.
    ObjectsTitle,
    /// The word for the tracks of a board.
    KindTrack,
    /// The word for the vias of a board.
    KindVia,
    /// The word for the copper zones of a board.
    KindZone,
    /// The word for the pads of a board.
    KindPad,
    /// The word for the silkscreen of a board.
    KindSilk,
    /// The word for the components of a board or the parts of a schematic.
    KindComponent,
    /// The word for the wires of a schematic.
    KindWire,
    /// The button that copies the report to the clipboard.
    ReportCopy,
    /// The line of the report that gives the number of problems of the rules.
    ReportFindings,
    /// The line under the library panel that says where libraries are edited.
    LibraryHint,
    /// The heading of the part of the tasks panel that holds the file of the document.
    SectionDocument,
    /// The heading of the part of the tasks panel that holds the tools of the window.
    SectionTools,
    /// The label of the frame times in the report panel (median, 95th and 99th percentile).
    FrameTime,
    /// The line of the console that says the engine asked for is not built and the painter of egui draws.
    EngineFellBack,
}

impl DockMsg {
    /// Every text, in the order of the table.
    pub const ALL: [DockMsg; 21] = [
        DockMsg::NoDocument,
        DockMsg::EditorHasDocument,
        DockMsg::ConsoleEmpty,
        DockMsg::ConsoleClear,
        DockMsg::ComponentsFilter,
        DockMsg::Shown,
        DockMsg::ObjectsTitle,
        DockMsg::KindTrack,
        DockMsg::KindVia,
        DockMsg::KindZone,
        DockMsg::KindPad,
        DockMsg::KindSilk,
        DockMsg::KindComponent,
        DockMsg::KindWire,
        DockMsg::ReportCopy,
        DockMsg::ReportFindings,
        DockMsg::LibraryHint,
        DockMsg::SectionDocument,
        DockMsg::SectionTools,
        DockMsg::FrameTime,
        DockMsg::EngineFellBack,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            DockMsg::NoDocument => [
                "Nothing is open: open a file from the File menu or the Tasks panel.",
                "No hay nada abierto: abra un archivo desde el menú Archivo o el panel Tareas.",
                "Nichts geöffnet: Öffnen Sie eine Datei im Menü Datei oder im Bereich Aufgaben.",
                "Rien n'est ouvert : ouvrez un fichier depuis le menu Fichier ou le panneau Tâches.",
                "Nulla è aperto: apra un file dal menu File o dal pannello Attività.",
                "何も開いていません。ファイルメニューまたはタスクパネルからファイルを開いてください。",
                "没有打开的文档：请从“文件”菜单或“任务”面板打开文件。",
                "Não há nada aberto: abra um ficheiro no menu Ficheiro ou no painel Tarefas.",
            ],
            DockMsg::EditorHasDocument => [
                "An editor has the document: its tools are in the Properties panel.",
                "Un editor tiene el documento: sus herramientas están en el panel Propiedades.",
                "Ein Editor hat das Dokument: seine Werkzeuge sind im Bereich Eigenschaften.",
                "Un éditeur a le document : ses outils sont dans le panneau Propriétés.",
                "Un editor ha il documento: i suoi strumenti sono nel pannello Proprietà.",
                "エディターが文書を開いています。ツールはプロパティパネルにあります。",
                "编辑器正在处理文档：其工具位于“属性”面板中。",
                "Um editor tem o documento: as suas ferramentas estão no painel Propriedades.",
            ],
            DockMsg::ConsoleEmpty => [
                "Nothing to report yet.",
                "Todavía no hay nada que informar.",
                "Noch nichts zu melden.",
                "Rien à signaler pour le moment.",
                "Ancora nulla da segnalare.",
                "まだ報告はありません。",
                "暂无消息。",
                "Ainda não há nada a comunicar.",
            ],
            DockMsg::ConsoleClear => [
                "Clear the console",
                "Vaciar la consola",
                "Konsole leeren",
                "Vider la console",
                "Svuota la console",
                "コンソールを消去",
                "清空控制台",
                "Limpar a consola",
            ],
            DockMsg::ComponentsFilter => [
                "Filter by reference, value or footprint",
                "Filtrar por referencia, valor o huella",
                "Nach Referenz, Wert oder Gehäuse filtern",
                "Filtrer par référence, valeur ou empreinte",
                "Filtra per riferimento, valore o impronta",
                "リファレンス、値、フットプリントで絞り込み",
                "按位号、值或封装筛选",
                "Filtrar por referência, valor ou pegada",
            ],
            DockMsg::Shown => [
                "shown", "mostrados", "angezeigt", "affichés", "mostrati", "件表示", "已显示",
                "mostrados",
            ],
            DockMsg::ObjectsTitle => [
                "Objects of the document",
                "Objetos del documento",
                "Objekte des Dokuments",
                "Objets du document",
                "Oggetti del documento",
                "文書のオブジェクト",
                "文档对象",
                "Objetos do documento",
            ],
            DockMsg::KindTrack => [
                "Tracks", "Pistas", "Leiterbahnen", "Pistes", "Piste", "配線", "走线", "Pistas",
            ],
            DockMsg::KindVia => [
                "Vias", "Vías", "Durchkontaktierungen", "Vias", "Via", "ビア", "过孔", "Vias",
            ],
            DockMsg::KindZone => [
                "Zones",
                "Zonas",
                "Flächen",
                "Zones",
                "Zone",
                "ゾーン",
                "铺铜区",
                "Zonas",
            ],
            DockMsg::KindPad => [
                "Pads", "Pads", "Pads", "Pastilles", "Pad", "パッド", "焊盘", "Pads",
            ],
            DockMsg::KindSilk => [
                "Silkscreen",
                "Serigrafía",
                "Bestückungsdruck",
                "Sérigraphie",
                "Serigrafia",
                "シルク",
                "丝印",
                "Serigrafia",
            ],
            DockMsg::KindComponent => [
                "Components",
                "Componentes",
                "Bauteile",
                "Composants",
                "Componenti",
                "部品",
                "元件",
                "Componentes",
            ],
            DockMsg::KindWire => [
                "Wires", "Cables", "Leitungen", "Fils", "Fili", "配線", "导线", "Fios",
            ],
            DockMsg::ReportCopy => [
                "Copy the report",
                "Copiar el informe",
                "Bericht kopieren",
                "Copier le rapport",
                "Copia il rapporto",
                "レポートをコピー",
                "复制报告",
                "Copiar o relatório",
            ],
            DockMsg::ReportFindings => [
                "Problems of the rules",
                "Problemas de las reglas",
                "Probleme der Regeln",
                "Problèmes des règles",
                "Problemi delle regole",
                "ルールの問題",
                "规则问题",
                "Problemas das regras",
            ],
            DockMsg::LibraryHint => [
                "Libraries of footprints, symbols and devices (.lbr) open in their own editor.",
                "Las bibliotecas de huellas, símbolos y dispositivos (.lbr) se abren en su propio editor.",
                "Bibliotheken für Gehäuse, Symbole und Bauteile (.lbr) öffnen sich im eigenen Editor.",
                "Les bibliothèques d'empreintes, de symboles et de composants (.lbr) s'ouvrent dans leur propre éditeur.",
                "Le librerie di impronte, simboli e dispositivi (.lbr) si aprono nel loro editor.",
                "フットプリント、シンボル、デバイスのライブラリ (.lbr) は専用のエディターで開きます。",
                "封装、符号和器件库 (.lbr) 在其专用编辑器中打开。",
                "As bibliotecas de pegadas, símbolos e dispositivos (.lbr) abrem-se no seu próprio editor.",
            ],
            DockMsg::SectionDocument => [
                "Document", "Documento", "Dokument", "Document", "Documento", "文書", "文档",
                "Documento",
            ],
            DockMsg::SectionTools => [
                "Tools",
                "Herramientas",
                "Werkzeuge",
                "Outils",
                "Strumenti",
                "ツール",
                "工具",
                "Ferramentas",
            ],
            DockMsg::FrameTime => [
                "Frame time (median / 95th / 99th)",
                "Tiempo de fotograma (mediana / 95 / 99)",
                "Bildzeit (Median / 95. / 99.)",
                "Durée d'image (médiane / 95e / 99e)",
                "Tempo di fotogramma (mediana / 95 / 99)",
                "フレーム時間 (中央値 / 95 / 99 パーセンタイル)",
                "帧时间（中位数 / 95 / 99 百分位）",
                "Tempo de fotograma (mediana / 95 / 99)",
            ],
            DockMsg::EngineFellBack => [
                "The engine asked for is not built yet: the painter of egui draws.",
                "El motor pedido todavía no está incluido: dibuja el pintor de egui.",
                "Die gewünschte Engine ist noch nicht eingebaut: der Painter von egui zeichnet.",
                "Le moteur demandé n'est pas encore intégré : le peintre d'egui dessine.",
                "Il motore richiesto non è ancora incluso: disegna il painter di egui.",
                "指定されたエンジンはまだ組み込まれていません。egui のペインターで描画します。",
                "所请求的引擎尚未内置：由 egui 的绘制器绘制。",
                "O motor pedido ainda não está incluído: desenha o pintor do egui.",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in DockMsg::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).trim().is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in DockMsg::ALL.iter().enumerate() {
            for b in &DockMsg::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in DockMsg::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        for text in DockMsg::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar", "Salvar", "deletar"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
        assert!(DockMsg::NoDocument.tr(Lang::Pt).contains("ficheiro"));
        assert!(DockMsg::ConsoleClear.tr(Lang::Pt).contains("consola"));
    }

    #[test]
    fn the_texts_that_are_not_the_english_one_differ_from_it_in_most_languages() {
        // a text copied into every language would leave the window half translated
        for text in DockMsg::ALL {
            let english = text.tr(Lang::En);
            let same = Lang::ALL
                .iter()
                .filter(|l| **l != Lang::En && text.tr(**l) == english)
                .count();
            assert!(
                same <= 4,
                "{text:?} is the English text in {same} languages"
            );
        }
    }
}
