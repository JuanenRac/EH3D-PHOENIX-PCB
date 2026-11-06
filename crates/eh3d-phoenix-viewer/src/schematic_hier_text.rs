// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the sheets and the buses in the window of the schematic editor, in the eight
//! languages of the suite: English, Spanish, German, French, Italian, Japanese, Chinese and
//! European Portuguese. The other texts of the schematic editor are in [`crate::schematic_text`].

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the sheets and the buses. The order of the variants is the order of the rows of the
/// table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierMsg {
    /// The title of the row of sheet tabs.
    Sheets,
    /// The tool that places a bus.
    ToolBus,
    /// The tool that takes a member out of a bus.
    ToolBusEntry,
    /// The label of the name of the bus to place.
    BusName,
    /// The label of the member of a bus.
    Member,
    /// The label of the list of the buses of the sheet.
    BusPick,
    /// What is said when the sheet has no bus.
    NoBuses,
    /// The hint of the bus tool.
    HintBus,
    /// The hint of the bus entry tool.
    HintBusEntry,
    /// The hint of how to open the sheet a sheet symbol shows.
    HintEnter,
    /// The button that goes back to the sheet that holds the symbol we came through.
    LeaveSheet,
    /// What follows the number of instances of a sheet.
    Instances,
}

const TABLE: [(HierMsg, [&str; 8]); HierMsg::COUNT] = [
    (
        HierMsg::Sheets,
        [
            "Sheets",
            "Hojas",
            "Blätter",
            "Feuilles",
            "Fogli",
            "シート",
            "图纸",
            "Folhas",
        ],
    ),
    (
        HierMsg::ToolBus,
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
        HierMsg::ToolBusEntry,
        [
            "Bus entry",
            "Entrada de bus",
            "Buseintrag",
            "Entrée de bus",
            "Ingresso bus",
            "バスエントリ",
            "总线入口",
            "Entrada de barramento",
        ],
    ),
    (
        HierMsg::BusName,
        [
            "Bus name",
            "Nombre del bus",
            "Busname",
            "Nom du bus",
            "Nome del bus",
            "バス名",
            "总线名称",
            "Nome do barramento",
        ],
    ),
    (
        HierMsg::Member,
        [
            "Member",
            "Miembro",
            "Mitglied",
            "Membre",
            "Membro",
            "メンバー",
            "成员",
            "Membro",
        ],
    ),
    (
        HierMsg::BusPick,
        [
            "Bus on this sheet",
            "Bus de esta hoja",
            "Bus dieses Blatts",
            "Bus de cette feuille",
            "Bus di questo foglio",
            "このシートのバス",
            "本图纸的总线",
            "Barramento desta folha",
        ],
    ),
    (
        HierMsg::NoBuses,
        [
            "There is no bus on this sheet",
            "No hay ningún bus en esta hoja",
            "Auf diesem Blatt gibt es keinen Bus",
            "Il n'y a pas de bus sur cette feuille",
            "Non ci sono bus in questo foglio",
            "このシートにバスはありません",
            "本图纸上没有总线",
            "Não há nenhum barramento nesta folha",
        ],
    ),
    (
        HierMsg::HintBus,
        [
            "Type the name of the bus (D[0..7] or USB{DP DM}) and click where it goes",
            "Escribe el nombre del bus (D[0..7] o USB{DP DM}) y haz clic donde va",
            "Namen des Busses eingeben (D[0..7] oder USB{DP DM}) und klicken, wo er hin soll",
            "Saisissez le nom du bus (D[0..7] ou USB{DP DM}) puis cliquez où le placer",
            "Scrivi il nome del bus (D[0..7] o USB{DP DM}) e fai clic dove va",
            "バス名（D[0..7] または USB{DP DM}）を入力し、置く位置をクリック",
            "输入总线名称（D[0..7] 或 USB{DP DM}），然后单击放置位置",
            "Escreva o nome do barramento (D[0..7] ou USB{DP DM}) e clique onde fica",
        ],
    ),
    (
        HierMsg::HintBusEntry,
        [
            "Choose the bus and the member, then click where the member leaves the bus",
            "Elige el bus y el miembro, y haz clic donde el miembro sale del bus",
            "Bus und Mitglied wählen, dann klicken, wo das Mitglied den Bus verlässt",
            "Choisissez le bus et le membre puis cliquez où le membre quitte le bus",
            "Scegli il bus e il membro, poi fai clic dove il membro esce dal bus",
            "バスとメンバーを選び、メンバーがバスから出る位置をクリック",
            "选择总线和成员，然后单击成员离开总线的位置",
            "Escolha o barramento e o membro e clique onde o membro sai do barramento",
        ],
    ),
    (
        HierMsg::HintEnter,
        [
            "Double-click a sheet symbol to open the sheet it shows",
            "Haz doble clic en un símbolo de hoja para abrir la hoja que muestra",
            "Doppelklick auf ein Blattsymbol öffnet das gezeigte Blatt",
            "Double-cliquez sur un symbole de feuille pour ouvrir la feuille qu'il montre",
            "Doppio clic su un simbolo di foglio per aprire il foglio che mostra",
            "シートシンボルをダブルクリックすると、参照先のシートが開きます",
            "双击图纸符号可打开其引用的图纸",
            "Faça duplo clique num símbolo de folha para abrir a folha que mostra",
        ],
    ),
    (
        HierMsg::LeaveSheet,
        [
            "Back to the parent sheet",
            "Volver a la hoja superior",
            "Zurück zum übergeordneten Blatt",
            "Retour à la feuille parente",
            "Torna al foglio superiore",
            "親シートに戻る",
            "返回上级图纸",
            "Voltar à folha superior",
        ],
    ),
    (
        HierMsg::Instances,
        [
            "instances",
            "instancias",
            "Instanzen",
            "instances",
            "istanze",
            "インスタンス",
            "个实例",
            "instâncias",
        ],
    ),
];

impl HierMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = HierMsg::Instances as usize + 1;

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
        assert_eq!(TABLE.len(), HierMsg::COUNT);
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
        assert_eq!(HierMsg::Sheets.tr(Lang::Pt), "Folhas");
        assert!(HierMsg::HintEnter.tr(Lang::Pt).contains("duplo clique"));
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("clique duplo"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for msg in [
            HierMsg::Sheets,
            HierMsg::ToolBusEntry,
            HierMsg::BusName,
            HierMsg::NoBuses,
            HierMsg::HintBus,
            HierMsg::HintEnter,
            HierMsg::LeaveSheet,
        ] {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
