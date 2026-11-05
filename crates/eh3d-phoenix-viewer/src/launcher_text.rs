// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the start screen of the window in the eight languages of the suite: English,
//! Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the start screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherText {
    /// The line that says what can be opened.
    Welcome,
    /// The label of the box where the path of a file is typed.
    TypePath,
    /// The button that starts an empty library.
    NewLibrary,
    /// The title of the list of recent files.
    Recent,
    /// The line shown while the list of recent files is empty.
    NoRecent,
    /// The button that takes one file out of the list.
    Forget,
    /// The button that empties the list.
    ClearRecent,
    /// The word for a board.
    KindBoard,
    /// The word for a schematic.
    KindSchematic,
    /// The word for a library.
    KindLibrary,
    /// The word for a JSON file of the model.
    KindData,
    /// The line that says a file can be dropped on the window.
    DropHere,
    /// The note shown when the button is pressed with no path typed.
    EnterPath,
    /// The note shown when no dropped file can be opened.
    NothingToOpen,
    /// The mark of a recent file that is no longer there.
    Missing,
    /// The note shown when the list of recent files could not be saved.
    SaveFailed,
}

impl LauncherText {
    /// Every text, for the tests that check the table.
    pub const ALL: [LauncherText; 16] = [
        LauncherText::Welcome,
        LauncherText::TypePath,
        LauncherText::NewLibrary,
        LauncherText::Recent,
        LauncherText::NoRecent,
        LauncherText::Forget,
        LauncherText::ClearRecent,
        LauncherText::KindBoard,
        LauncherText::KindSchematic,
        LauncherText::KindLibrary,
        LauncherText::KindData,
        LauncherText::DropHere,
        LauncherText::EnterPath,
        LauncherText::NothingToOpen,
        LauncherText::Missing,
        LauncherText::SaveFailed,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            LauncherText::Welcome => [
                "Open a board, a schematic or a library",
                "Abra una placa, un esquema o una biblioteca",
                "Eine Platine, einen Schaltplan oder eine Bibliothek öffnen",
                "Ouvrez une carte, un schéma ou une bibliothèque",
                "Apri una scheda, uno schema o una libreria",
                "基板、回路図、またはライブラリを開く",
                "打开电路板、原理图或库",
                "Abra uma placa, um esquema ou uma biblioteca",
            ],
            LauncherText::TypePath => [
                "Path of a file (.brd, .sch, .lbr or .json)",
                "Ruta de un archivo (.brd, .sch, .lbr o .json)",
                "Pfad einer Datei (.brd, .sch, .lbr oder .json)",
                "Chemin d'un fichier (.brd, .sch, .lbr ou .json)",
                "Percorso di un file (.brd, .sch, .lbr o .json)",
                "ファイルのパス (.brd、.sch、.lbr、.json)",
                "文件路径 (.brd、.sch、.lbr 或 .json)",
                "Caminho de um ficheiro (.brd, .sch, .lbr ou .json)",
            ],
            LauncherText::NewLibrary => [
                "New library",
                "Nueva biblioteca",
                "Neue Bibliothek",
                "Nouvelle bibliothèque",
                "Nuova libreria",
                "新規ライブラリ",
                "新建库",
                "Nova biblioteca",
            ],
            LauncherText::Recent => [
                "Recent files",
                "Archivos recientes",
                "Zuletzt geöffnet",
                "Fichiers récents",
                "File recenti",
                "最近使ったファイル",
                "最近的文件",
                "Ficheiros recentes",
            ],
            LauncherText::NoRecent => [
                "No recent files yet.",
                "Todavía no hay archivos recientes.",
                "Noch keine zuletzt geöffneten Dateien.",
                "Aucun fichier récent pour le moment.",
                "Ancora nessun file recente.",
                "最近使ったファイルはまだありません。",
                "还没有最近的文件。",
                "Ainda não há ficheiros recentes.",
            ],
            LauncherText::Forget => [
                "Forget",
                "Olvidar",
                "Entfernen",
                "Oublier",
                "Dimentica",
                "削除",
                "移除",
                "Esquecer",
            ],
            LauncherText::ClearRecent => [
                "Clear the list",
                "Vaciar la lista",
                "Liste leeren",
                "Vider la liste",
                "Svuota l'elenco",
                "一覧を消去",
                "清空列表",
                "Limpar a lista",
            ],
            LauncherText::KindBoard => [
                "Board", "Placa", "Platine", "Carte", "Scheda", "基板", "电路板", "Placa",
            ],
            LauncherText::KindSchematic => [
                "Schematic",
                "Esquema",
                "Schaltplan",
                "Schéma",
                "Schema",
                "回路図",
                "原理图",
                "Esquema",
            ],
            LauncherText::KindLibrary => [
                "Library",
                "Biblioteca",
                "Bibliothek",
                "Bibliothèque",
                "Libreria",
                "ライブラリ",
                "库",
                "Biblioteca",
            ],
            LauncherText::KindData => [
                "JSON file",
                "Archivo JSON",
                "JSON-Datei",
                "Fichier JSON",
                "File JSON",
                "JSON ファイル",
                "JSON 文件",
                "Ficheiro JSON",
            ],
            LauncherText::DropHere => [
                "You can also drop a file on this window.",
                "También puede soltar un archivo en esta ventana.",
                "Sie können auch eine Datei auf dieses Fenster ziehen.",
                "Vous pouvez aussi déposer un fichier sur cette fenêtre.",
                "Puoi anche trascinare un file su questa finestra.",
                "このウィンドウにファイルをドロップすることもできます。",
                "也可以将文件拖放到此窗口。",
                "Também pode largar um ficheiro nesta janela.",
            ],
            LauncherText::EnterPath => [
                "Type the path of a file first.",
                "Escriba primero la ruta de un archivo.",
                "Geben Sie zuerst den Pfad einer Datei ein.",
                "Saisissez d'abord le chemin d'un fichier.",
                "Digita prima il percorso di un file.",
                "先にファイルのパスを入力してください。",
                "请先输入文件路径。",
                "Escreva primeiro o caminho de um ficheiro.",
            ],
            LauncherText::NothingToOpen => [
                "None of the dropped files is a board, a schematic or a library.",
                "Ningún archivo soltado es una placa, un esquema o una biblioteca.",
                "Keine der abgelegten Dateien ist eine Platine, ein Schaltplan oder eine Bibliothek.",
                "Aucun des fichiers déposés n'est une carte, un schéma ou une bibliothèque.",
                "Nessuno dei file trascinati è una scheda, uno schema o una libreria.",
                "ドロップされたファイルに基板、回路図、ライブラリはありません。",
                "拖放的文件中没有电路板、原理图或库。",
                "Nenhum dos ficheiros largados é uma placa, um esquema ou uma biblioteca.",
            ],
            LauncherText::Missing => [
                "not found",
                "no encontrado",
                "nicht gefunden",
                "introuvable",
                "non trovato",
                "見つかりません",
                "未找到",
                "não encontrado",
            ],
            LauncherText::SaveFailed => [
                "The list of recent files could not be saved.",
                "No se pudo guardar la lista de archivos recientes.",
                "Die Liste der zuletzt geöffneten Dateien konnte nicht gespeichert werden.",
                "La liste des fichiers récents n'a pas pu être enregistrée.",
                "Non è stato possibile salvare l'elenco dei file recenti.",
                "最近使ったファイルの一覧を保存できませんでした。",
                "无法保存最近文件列表。",
                "Não foi possível guardar a lista de ficheiros recentes.",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in LauncherText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in LauncherText::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert!(LauncherText::Recent.tr(Lang::Pt).contains("Ficheiros"));
        assert!(LauncherText::DropHere.tr(Lang::Pt).contains("ficheiro"));
        for text in LauncherText::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in LauncherText::ALL.iter().enumerate() {
            for b in &LauncherText::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
