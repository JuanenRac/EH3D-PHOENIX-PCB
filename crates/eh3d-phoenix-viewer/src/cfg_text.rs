// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the window of the settings files (the Rules window), in the eight languages of the
//! suite: English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the window of the settings files. The order of the variants is the order of the rows
/// of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfgMsg {
    /// The title of the window and of the button that opens it.
    Title,
    /// The label of the folder the files are read from.
    Folder,
    /// The label of the choice of the file.
    File,
    /// The button that reads the file again.
    Reload,
    /// The heading of the column of the names of the settings.
    Setting,
    /// The heading of the column of the values.
    Value,
    /// The heading of the column of the default values.
    Default,
    /// The heading of the column of the range that the program allows.
    Allowed,
    /// The heading of the column of the comments of the file.
    Help,
    /// The box that lists only the settings that differ from the defaults.
    OnlyChanged,
    /// The button that puts a default value back.
    Restore,
    /// The button that takes back every change made since the file was read.
    Discard,
    /// The button and the label of saving the file under a name.
    SaveAs,
    /// The box that allows a file that exists to be replaced.
    Replace,
    /// The heading of the list of settings that the file does not have.
    Missing,
    /// The button that adds a missing setting with its default value.
    Add,
    /// What is said when the program accepts the file.
    Valid,
    /// What is said before the reason when the program refuses the file.
    NotValid,
    /// The word that goes before the number of a line.
    Line,
    /// What is said when the file is not in the folder and the defaults are shown.
    NotFound,
    /// What is said when the file was read.
    Read,
    /// What is said when the file was written.
    Saved,
    /// The warning of a range with values inside that the program refuses.
    Gaps,
    /// The note under the table about how the ranges are found.
    RangeNote,
    /// What follows the number of settings that differ from the defaults.
    Changed,
}

const TABLE: [(CfgMsg, [&str; 8]); CfgMsg::COUNT] = [
    (
        CfgMsg::Title,
        [
            "Rules",
            "Reglas",
            "Regeln",
            "Règles",
            "Regole",
            "ルール",
            "规则",
            "Regras",
        ],
    ),
    (
        CfgMsg::Folder,
        [
            "Folder of the files",
            "Carpeta de los archivos",
            "Ordner der Dateien",
            "Dossier des fichiers",
            "Cartella dei file",
            "ファイルのフォルダー",
            "文件所在文件夹",
            "Pasta dos ficheiros",
        ],
    ),
    (
        CfgMsg::File,
        [
            "File",
            "Archivo",
            "Datei",
            "Fichier",
            "File",
            "ファイル",
            "文件",
            "Ficheiro",
        ],
    ),
    (
        CfgMsg::Reload,
        [
            "Read again",
            "Leer de nuevo",
            "Neu einlesen",
            "Relire",
            "Rileggi",
            "再読み込み",
            "重新读取",
            "Ler de novo",
        ],
    ),
    (
        CfgMsg::Setting,
        [
            "Setting",
            "Ajuste",
            "Einstellung",
            "Réglage",
            "Impostazione",
            "設定",
            "设置",
            "Definição",
        ],
    ),
    (
        CfgMsg::Value,
        [
            "Value", "Valor", "Wert", "Valeur", "Valore", "値", "值", "Valor",
        ],
    ),
    (
        CfgMsg::Default,
        [
            "Default",
            "Por defecto",
            "Standard",
            "Par défaut",
            "Predefinito",
            "既定値",
            "默认值",
            "Predefinição",
        ],
    ),
    (
        CfgMsg::Allowed,
        [
            "Allowed",
            "Permitido",
            "Erlaubt",
            "Autorisé",
            "Consentito",
            "許容範囲",
            "允许范围",
            "Permitido",
        ],
    ),
    (
        CfgMsg::Help,
        [
            "Help",
            "Ayuda",
            "Hilfe",
            "Aide",
            "Aiuto",
            "ヘルプ",
            "帮助",
            "Ajuda",
        ],
    ),
    (
        CfgMsg::OnlyChanged,
        [
            "Only what differs from the default",
            "Solo lo que difiere del valor por defecto",
            "Nur Abweichungen vom Standard",
            "Seulement ce qui diffère de la valeur par défaut",
            "Solo ciò che differisce dal predefinito",
            "既定値と異なるものだけ",
            "仅显示与默认值不同的项",
            "Só o que difere da predefinição",
        ],
    ),
    (
        CfgMsg::Restore,
        [
            "Restore",
            "Restablecer",
            "Zurücksetzen",
            "Rétablir",
            "Ripristina",
            "元に戻す",
            "恢复",
            "Repor",
        ],
    ),
    (
        CfgMsg::Discard,
        [
            "Discard my changes",
            "Descartar mis cambios",
            "Änderungen verwerfen",
            "Annuler mes modifications",
            "Annulla le mie modifiche",
            "変更を破棄",
            "放弃我的修改",
            "Descartar as minhas alterações",
        ],
    ),
    (
        CfgMsg::SaveAs,
        [
            "Save as",
            "Guardar como",
            "Speichern unter",
            "Enregistrer sous",
            "Salva come",
            "名前を付けて保存",
            "另存为",
            "Guardar como",
        ],
    ),
    (
        CfgMsg::Replace,
        [
            "Replace the file if it exists (a copy is kept)",
            "Reemplazar el archivo si existe (se guarda una copia)",
            "Vorhandene Datei ersetzen (eine Kopie bleibt erhalten)",
            "Remplacer le fichier s'il existe (une copie est conservée)",
            "Sostituisci il file se esiste (viene conservata una copia)",
            "ファイルが存在する場合は置き換える（コピーを残します）",
            "若文件已存在则替换（保留一份副本）",
            "Substituir o ficheiro se existir (fica uma cópia)",
        ],
    ),
    (
        CfgMsg::Missing,
        [
            "Settings the file does not have",
            "Ajustes que el archivo no tiene",
            "Einstellungen, die der Datei fehlen",
            "Réglages absents du fichier",
            "Impostazioni assenti nel file",
            "ファイルにない設定",
            "文件中缺少的设置",
            "Definições que o ficheiro não tem",
        ],
    ),
    (
        CfgMsg::Add,
        [
            "Add",
            "Añadir",
            "Hinzufügen",
            "Ajouter",
            "Aggiungi",
            "追加",
            "添加",
            "Adicionar",
        ],
    ),
    (
        CfgMsg::Valid,
        [
            "The program accepts this file",
            "El programa acepta este archivo",
            "Das Programm akzeptiert diese Datei",
            "Le programme accepte ce fichier",
            "Il programma accetta questo file",
            "プログラムはこのファイルを受け付けます",
            "程序接受此文件",
            "O programa aceita este ficheiro",
        ],
    ),
    (
        CfgMsg::NotValid,
        [
            "The program refuses this file",
            "El programa rechaza este archivo",
            "Das Programm lehnt diese Datei ab",
            "Le programme refuse ce fichier",
            "Il programma rifiuta questo file",
            "プログラムはこのファイルを拒否します",
            "程序拒绝此文件",
            "O programa recusa este ficheiro",
        ],
    ),
    (
        CfgMsg::Line,
        [
            "line", "línea", "Zeile", "ligne", "riga", "行", "行", "linha",
        ],
    ),
    (
        CfgMsg::NotFound,
        [
            "There is no file there: the defaults are shown",
            "No hay archivo ahí: se muestran los valores por defecto",
            "Dort gibt es keine Datei: die Standardwerte werden angezeigt",
            "Il n'y a pas de fichier à cet endroit : les valeurs par défaut sont affichées",
            "Non c'è nessun file lì: sono mostrati i valori predefiniti",
            "そこにファイルはありません。既定値を表示しています",
            "该处没有文件：显示默认值",
            "Não há ficheiro aí: são mostradas as predefinições",
        ],
    ),
    (
        CfgMsg::Read,
        [
            "File read",
            "Archivo leído",
            "Datei gelesen",
            "Fichier lu",
            "File letto",
            "ファイルを読み込みました",
            "已读取文件",
            "Ficheiro lido",
        ],
    ),
    (
        CfgMsg::Saved,
        [
            "File saved",
            "Archivo guardado",
            "Datei gespeichert",
            "Fichier enregistré",
            "File salvato",
            "ファイルを保存しました",
            "已保存文件",
            "Ficheiro guardado",
        ],
    ),
    (
        CfgMsg::Gaps,
        [
            "Some values inside this range are refused",
            "Algunos valores dentro de este rango se rechazan",
            "Einige Werte in diesem Bereich werden abgelehnt",
            "Certaines valeurs de cette plage sont refusées",
            "Alcuni valori in questo intervallo sono rifiutati",
            "この範囲内でも拒否される値があります",
            "此范围内的某些值会被拒绝",
            "Alguns valores dentro deste intervalo são recusados",
        ],
    ),
    (
        CfgMsg::RangeNote,
        [
            "The allowed range is found by offering numbers to the reader of the program, one setting at a time on the file of defaults; … means no limit was found.",
            "El rango permitido se halla ofreciendo números al lector del programa, un ajuste cada vez sobre el archivo por defecto; … significa que no se halló límite.",
            "Der erlaubte Bereich wird ermittelt, indem dem Leser des Programms Zahlen angeboten werden, eine Einstellung nach der anderen in der Standarddatei; … heißt: keine Grenze gefunden.",
            "La plage autorisée est trouvée en proposant des nombres au lecteur du programme, un réglage à la fois dans le fichier par défaut ; … signifie qu'aucune limite n'a été trouvée.",
            "L'intervallo consentito si trova offrendo numeri al lettore del programma, un'impostazione alla volta nel file predefinito; … significa che non è stato trovato alcun limite.",
            "許容範囲は、既定値のファイルで設定を1つずつ、プログラムの読み取り処理に数値を与えて調べます。… は上限・下限が見つからなかったことを示します。",
            "允许范围是在默认文件上逐项向程序的读取器提供数值得出的；… 表示未找到限制。",
            "O intervalo permitido é obtido oferecendo números ao leitor do programa, uma definição de cada vez no ficheiro de predefinições; … significa que não foi encontrado limite.",
        ],
    ),
    (
        CfgMsg::Changed,
        [
            "changed",
            "cambiados",
            "geändert",
            "modifiés",
            "modificati",
            "変更済み",
            "已修改",
            "alterados",
        ],
    ),
];

impl CfgMsg {
    /// How many texts there are: one more than the number of the last variant.
    pub const COUNT: usize = CfgMsg::Changed as usize + 1;

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
        assert_eq!(TABLE.len(), CfgMsg::COUNT);
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
        assert_eq!(CfgMsg::File.tr(Lang::Pt), "Ficheiro");
        assert_eq!(CfgMsg::Restore.tr(Lang::Pt), "Repor");
        for (msg, _) in TABLE.iter() {
            let text = msg.tr(Lang::Pt);
            assert!(!text.contains("arquivo"), "{msg:?}");
            assert!(!text.contains("tela"), "{msg:?}");
            assert!(!text.contains("Salvar"), "{msg:?}");
            assert!(!text.contains("salvar"), "{msg:?}");
            assert!(!text.contains("padrão"), "{msg:?}");
        }
    }

    #[test]
    fn the_other_languages_are_not_copies_of_the_english_text() {
        for (msg, _) in TABLE.iter() {
            // a word that is spelled the same in several languages is not compared
            if matches!(msg, CfgMsg::File | CfgMsg::Value) {
                continue;
            }
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Pt] {
                assert_ne!(msg.tr(lang), msg.tr(Lang::En), "{msg:?} {lang:?}");
            }
        }
    }
}
