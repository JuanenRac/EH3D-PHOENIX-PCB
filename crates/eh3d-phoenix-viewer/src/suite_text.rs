// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts that this program adds to the ones every program of the suite says about the project
//! file and its attachments (`eh3d_project::suitetext`, `eh3d-attachpanel`): the data sheets of a
//! part and what an import replaces, in the eight languages of the suite: English, Spanish, German, French, Italian, Japanese, Chinese and
//! European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of this program about the project of the suite and the data sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteMsg {
    /// The entry of the menu that opens the data sheet of the selected part.
    SheetEntry,
    /// The button that lists the data sheets of a part (`{0}` is the part).
    SheetsOf,
    /// The button that links the chosen attachment to a part (`{0}` is the part).
    LinkToPart,
    /// The line that names the selected part (`{0}`).
    PartLine,
    /// The line shown while no part is selected.
    ChoosePart,
    /// The warning before an import replaces the board of the branch.
    ReplaceBoard,
    /// The warning before an import replaces the schematic of the branch.
    ReplaceSchematic,
    /// The warning before the document of the window is replaced with changes not saved.
    UnsavedLost,
    /// What an import takes.
    ImportKinds,
}

impl SuiteMsg {
    /// Every text, in the order of the table.
    pub const ALL: [SuiteMsg; 9] = [
        SuiteMsg::SheetEntry,
        SuiteMsg::SheetsOf,
        SuiteMsg::LinkToPart,
        SuiteMsg::PartLine,
        SuiteMsg::ChoosePart,
        SuiteMsg::ReplaceBoard,
        SuiteMsg::ReplaceSchematic,
        SuiteMsg::UnsavedLost,
        SuiteMsg::ImportKinds,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            SuiteMsg::SheetEntry => [
                "Data sheet of the selected part",
                "Hoja de datos de la pieza seleccionada",
                "Datenblatt des gewählten Bauteils",
                "Fiche technique du composant sélectionné",
                "Scheda tecnica del componente selezionato",
                "選択した部品のデータシート",
                "所选元件的数据手册",
                "Folha de dados do componente selecionado",
            ],
            SuiteMsg::SheetsOf => [
                "Data sheets of {0}",
                "Hojas de datos de {0}",
                "Datenblätter von {0}",
                "Fiches techniques de {0}",
                "Schede tecniche di {0}",
                "{0} のデータシート",
                "{0} 的数据手册",
                "Folhas de dados de {0}",
            ],
            SuiteMsg::LinkToPart => [
                "Link the chosen attachment to {0}",
                "Ligar el adjunto elegido a {0}",
                "Gewählten Anhang mit {0} verknüpfen",
                "Lier la pièce jointe choisie à {0}",
                "Collega l'allegato scelto a {0}",
                "選んだ添付ファイルを {0} に関連付ける",
                "将所选附件关联到 {0}",
                "Ligar o anexo escolhido a {0}",
            ],
            SuiteMsg::PartLine => [
                "Part: {0}",
                "Pieza: {0}",
                "Bauteil: {0}",
                "Composant : {0}",
                "Componente: {0}",
                "部品：{0}",
                "元件：{0}",
                "Componente: {0}",
            ],
            SuiteMsg::ChoosePart => [
                "Select a part in the drawing or in the list of components to see its data sheets.",
                "Seleccione una pieza en el dibujo o en la lista de componentes para ver sus hojas de datos.",
                "Wählen Sie ein Bauteil in der Zeichnung oder in der Bauteilliste, um seine Datenblätter zu sehen.",
                "Sélectionnez un composant dans le dessin ou dans la liste des composants pour voir ses fiches techniques.",
                "Selezionare un componente nel disegno o nell'elenco dei componenti per vederne le schede tecniche.",
                "図面または部品一覧で部品を選ぶと、そのデータシートが表示されます。",
                "在图中或元件列表中选择元件，即可查看其数据手册。",
                "Selecione um componente no desenho ou na lista de componentes para ver as suas folhas de dados.",
            ],
            SuiteMsg::ReplaceBoard => [
                "The board of the branch pcb will be replaced by the imported one",
                "La placa de la rama pcb se sustituirá por la importada",
                "Die Platine des Zweigs pcb wird durch die importierte ersetzt",
                "La carte de la branche pcb sera remplacée par la carte importée",
                "La scheda del ramo pcb sarà sostituita da quella importata",
                "pcb ブランチの基板は、取り込んだ基板に置き換えられます",
                "pcb 分支中的电路板将被导入的电路板替换",
                "A placa do ramo pcb vai ser substituída pela importada",
            ],
            SuiteMsg::ReplaceSchematic => [
                "The schematic of the branch pcb will be replaced by the imported one",
                "El esquema de la rama pcb se sustituirá por el importado",
                "Der Schaltplan des Zweigs pcb wird durch den importierten ersetzt",
                "Le schéma de la branche pcb sera remplacé par le schéma importé",
                "Lo schema del ramo pcb sarà sostituito da quello importato",
                "pcb ブランチの回路図は、取り込んだ回路図に置き換えられます",
                "pcb 分支中的原理图将被导入的原理图替换",
                "O esquema do ramo pcb vai ser substituído pelo importado",
            ],
            SuiteMsg::UnsavedLost => [
                "The changes of the document in the window that are not saved will be lost",
                "Se perderán los cambios sin guardar del documento de la ventana",
                "Nicht gespeicherte Änderungen des Dokuments im Fenster gehen verloren",
                "Les modifications non enregistrées du document de la fenêtre seront perdues",
                "Le modifiche non salvate del documento nella finestra andranno perse",
                "ウィンドウの文書の未保存の変更は失われます",
                "窗口中文档未保存的更改将丢失",
                "As alterações por guardar do documento da janela serão perdidas",
            ],
            SuiteMsg::ImportKinds => [
                "Only an EAGLE board (.brd) or schematic (.sch) is imported",
                "Solo se importa una placa (.brd) o un esquema (.sch) de EAGLE",
                "Importiert wird nur eine EAGLE-Platine (.brd) oder ein Schaltplan (.sch)",
                "Seuls une carte (.brd) ou un schéma (.sch) EAGLE sont importés",
                "Si importa solo una scheda (.brd) o uno schema (.sch) di EAGLE",
                "取り込めるのは EAGLE の基板（.brd）または回路図（.sch）のみです",
                "只能导入 EAGLE 电路板（.brd）或原理图（.sch）",
                "Só se importa uma placa (.brd) ou um esquema (.sch) do EAGLE",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_of_the_suite_exists_in_the_eight_languages() {
        for (i, text) in SuiteMsg::ALL.iter().enumerate() {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).trim().is_empty(), "{text:?} {lang:?}");
                assert_eq!(
                    text.tr(lang).contains("{0}"),
                    text.tr(Lang::En).contains("{0}"),
                    "{text:?} {lang:?}"
                );
            }
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
            assert!(!SuiteMsg::ALL[i + 1..].contains(text));
        }
    }
}
