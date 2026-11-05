// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the HTML report in the eight languages of the suite: English, Spanish, German,
//! French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// The title of the page.
    Title,
    /// The note that the report only analyses.
    Intro,
    /// The word for libraries.
    Libraries,
    /// The word for footprints.
    Footprints,
    /// The word for symbols.
    Symbols,
    /// The word for device sets.
    DeviceSets,
    /// The title of the groups.
    Groups,
    /// The column of the file path.
    Path,
    /// The column of the version.
    Version,
    /// The column of the status.
    Status,
    /// The column of the group id.
    GroupId,
    /// The column of the kind.
    Kind,
    /// The column of the score.
    Score,
    /// The column that tells exact duplicates.
    Exact,
    /// The column that tells whether the names agree.
    NamesAgree,
    /// The column that marks the item to keep.
    Keep,
    /// The column of the library.
    Library,
    /// The column of the name.
    Name,
    /// The column of the hash.
    Hash,
    /// The column that tells a turned copy.
    Turned,
    /// The column of the number of uses.
    Uses,
    /// The word yes.
    Yes,
    /// The word no.
    No,
    /// The title of the notes.
    Notes,
    /// The column of the code.
    Code,
    /// The column of the detail.
    Detail,
    /// The title of the list of identical files.
    IdenticalFiles,
    /// The text when there are no groups.
    NoGroups,
    /// The label of the threshold.
    Threshold,
    /// The status of a file that was read.
    Read,
    /// The status of a file that was left out.
    Skipped,
    /// The label of the files read.
    FilesRead,
    /// The label of the redundant items.
    Redundant,
    /// The column that asks for a review.
    Review,
    /// The singular word for a footprint.
    KindPackage,
    /// The singular word for a symbol.
    KindSymbol,
    /// The singular word for a device set.
    KindDeviceSet,
}

impl Msg {
    /// Every text, for the tests that check the table.
    pub const ALL: [Msg; 37] = [
        Msg::Title,
        Msg::Intro,
        Msg::Libraries,
        Msg::Footprints,
        Msg::Symbols,
        Msg::DeviceSets,
        Msg::Groups,
        Msg::Path,
        Msg::Version,
        Msg::Status,
        Msg::GroupId,
        Msg::Kind,
        Msg::Score,
        Msg::Exact,
        Msg::NamesAgree,
        Msg::Keep,
        Msg::Library,
        Msg::Name,
        Msg::Hash,
        Msg::Turned,
        Msg::Uses,
        Msg::Yes,
        Msg::No,
        Msg::Notes,
        Msg::Code,
        Msg::Detail,
        Msg::IdenticalFiles,
        Msg::NoGroups,
        Msg::Threshold,
        Msg::Read,
        Msg::Skipped,
        Msg::FilesRead,
        Msg::Redundant,
        Msg::Review,
        Msg::KindPackage,
        Msg::KindSymbol,
        Msg::KindDeviceSet,
    ];

    fn row(self) -> [&'static str; 8] {
        match self {
            Msg::Title => [
                "Library sanitizer report",
                "Informe del saneador de bibliotecas",
                "Bericht des Bibliotheksbereinigers",
                "Rapport de l'assainisseur de bibliothèques",
                "Rapporto del sanificatore di librerie",
                "ライブラリ整理レポート",
                "库清理报告",
                "Relatório do saneador de bibliotecas",
            ],
            Msg::Intro => [
                "Analysis only: no library was changed. Every group is a proposal for you to confirm.",
                "Solo análisis: no se cambió ninguna biblioteca. Cada grupo es una propuesta que debe confirmar usted.",
                "Nur Analyse: keine Bibliothek wurde geändert. Jede Gruppe ist ein Vorschlag, den Sie bestätigen müssen.",
                "Analyse seule : aucune bibliothèque n'a été modifiée. Chaque groupe est une proposition que vous devez confirmer.",
                "Solo analisi: nessuna libreria è stata modificata. Ogni gruppo è una proposta da confermare.",
                "分析のみ：ライブラリは変更されていません。各グループは確認していただく提案です。",
                "仅作分析：未修改任何库。每个分组都是需要您确认的建议。",
                "Apenas análise: nenhuma biblioteca foi alterada. Cada grupo é uma proposta para confirmar.",
            ],
            Msg::Libraries => [
                "Libraries",
                "Bibliotecas",
                "Bibliotheken",
                "Bibliothèques",
                "Librerie",
                "ライブラリ",
                "库",
                "Bibliotecas",
            ],
            Msg::Footprints => [
                "Footprints",
                "Huellas",
                "Bauformen",
                "Empreintes",
                "Impronte",
                "フットプリント",
                "封装",
                "Pegadas",
            ],
            Msg::Symbols => [
                "Symbols",
                "Símbolos",
                "Symbole",
                "Symboles",
                "Simboli",
                "シンボル",
                "符号",
                "Símbolos",
            ],
            Msg::DeviceSets => [
                "Device sets",
                "Dispositivos",
                "Bauteilsätze",
                "Dispositifs",
                "Dispositivi",
                "デバイスセット",
                "器件集",
                "Dispositivos",
            ],
            Msg::Groups => [
                "Groups of duplicates",
                "Grupos de duplicados",
                "Duplikatgruppen",
                "Groupes de doublons",
                "Gruppi di duplicati",
                "重複グループ",
                "重复分组",
                "Grupos de duplicados",
            ],
            Msg::Path => [
                "File",
                "Archivo",
                "Datei",
                "Fichier",
                "File",
                "ファイル",
                "文件",
                "Ficheiro",
            ],
            Msg::Version => [
                "Version",
                "Versión",
                "Version",
                "Version",
                "Versione",
                "バージョン",
                "版本",
                "Versão",
            ],
            Msg::Status => [
                "Status",
                "Estado",
                "Status",
                "État",
                "Stato",
                "状態",
                "状态",
                "Estado",
            ],
            Msg::GroupId => [
                "Group",
                "Grupo",
                "Gruppe",
                "Groupe",
                "Gruppo",
                "グループ",
                "分组",
                "Grupo",
            ],
            Msg::Kind => ["Kind", "Tipo", "Art", "Type", "Tipo", "種類", "类型", "Tipo"],
            Msg::Score => [
                "Score",
                "Puntuación",
                "Wertung",
                "Score",
                "Punteggio",
                "スコア",
                "得分",
                "Pontuação",
            ],
            Msg::Exact => [
                "Exact",
                "Exacto",
                "Exakt",
                "Exact",
                "Esatto",
                "完全一致",
                "完全相同",
                "Exato",
            ],
            Msg::NamesAgree => [
                "Names agree",
                "Los nombres coinciden",
                "Namen stimmen überein",
                "Noms concordants",
                "I nomi coincidono",
                "名前が一致",
                "名称一致",
                "Os nomes coincidem",
            ],
            Msg::Keep => [
                "Keep",
                "Conservar",
                "Behalten",
                "Conserver",
                "Mantenere",
                "残す",
                "保留",
                "Manter",
            ],
            Msg::Library => [
                "Library",
                "Biblioteca",
                "Bibliothek",
                "Bibliothèque",
                "Libreria",
                "ライブラリ",
                "库",
                "Biblioteca",
            ],
            Msg::Name => ["Name", "Nombre", "Name", "Nom", "Nome", "名前", "名称", "Nome"],
            Msg::Hash => ["Hash", "Hash", "Hash", "Hash", "Hash", "ハッシュ", "哈希", "Hash"],
            Msg::Turned => [
                "Turned",
                "Girado",
                "Gedreht",
                "Tourné",
                "Ruotato",
                "回転",
                "旋转",
                "Rodado",
            ],
            Msg::Uses => [
                "Uses",
                "Usos",
                "Verwendungen",
                "Usages",
                "Usi",
                "使用数",
                "使用次数",
                "Utilizações",
            ],
            Msg::Yes => ["yes", "sí", "ja", "oui", "sì", "はい", "是", "sim"],
            Msg::No => ["no", "no", "nein", "non", "no", "いいえ", "否", "não"],
            Msg::Notes => [
                "Notes",
                "Avisos",
                "Hinweise",
                "Remarques",
                "Note",
                "注記",
                "备注",
                "Avisos",
            ],
            Msg::Code => ["Code", "Código", "Code", "Code", "Codice", "コード", "代码", "Código"],
            Msg::Detail => [
                "Detail",
                "Detalle",
                "Detail",
                "Détail",
                "Dettaglio",
                "詳細",
                "详情",
                "Detalhe",
            ],
            Msg::IdenticalFiles => [
                "Identical files",
                "Archivos idénticos",
                "Identische Dateien",
                "Fichiers identiques",
                "File identici",
                "同一のファイル",
                "相同的文件",
                "Ficheiros idênticos",
            ],
            Msg::NoGroups => [
                "No duplicates found.",
                "No se encontraron duplicados.",
                "Keine Duplikate gefunden.",
                "Aucun doublon trouvé.",
                "Nessun duplicato trovato.",
                "重複は見つかりませんでした。",
                "未发现重复项。",
                "Não foram encontrados duplicados.",
            ],
            Msg::Threshold => [
                "Threshold",
                "Umbral",
                "Schwelle",
                "Seuil",
                "Soglia",
                "しきい値",
                "阈值",
                "Limiar",
            ],
            Msg::Read => [
                "read",
                "leído",
                "gelesen",
                "lu",
                "letto",
                "読み込み済み",
                "已读取",
                "lido",
            ],
            Msg::Skipped => [
                "skipped",
                "omitido",
                "übersprungen",
                "ignoré",
                "saltato",
                "スキップ",
                "已跳过",
                "ignorado",
            ],
            Msg::FilesRead => [
                "Files read",
                "Archivos leídos",
                "Gelesene Dateien",
                "Fichiers lus",
                "File letti",
                "読み込んだファイル",
                "已读文件",
                "Ficheiros lidos",
            ],
            Msg::Redundant => [
                "Redundant items",
                "Elementos sobrantes",
                "Überzählige Elemente",
                "Éléments superflus",
                "Elementi in eccesso",
                "余分な要素",
                "多余项",
                "Elementos redundantes",
            ],
            Msg::Review => [
                "Review",
                "Revisar",
                "Prüfen",
                "À vérifier",
                "Da verificare",
                "要確認",
                "需复查",
                "Rever",
            ],
            Msg::KindPackage => [
                "Footprint",
                "Huella",
                "Bauform",
                "Empreinte",
                "Impronta",
                "フットプリント",
                "封装",
                "Pegada",
            ],
            Msg::KindSymbol => [
                "Symbol",
                "Símbolo",
                "Symbol",
                "Symbole",
                "Simbolo",
                "シンボル",
                "符号",
                "Símbolo",
            ],
            Msg::KindDeviceSet => [
                "Device set",
                "Dispositivo",
                "Bauteilsatz",
                "Dispositif",
                "Dispositivo",
                "デバイスセット",
                "器件集",
                "Dispositivo",
            ],
        }
    }
}

/// A text in a language.
pub fn text(msg: Msg, lang: Lang) -> &'static str {
    msg.row()[column(lang)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_every_language() {
        for msg in Msg::ALL {
            for lang in Lang::ALL {
                assert!(!text(msg, lang).is_empty(), "{msg:?} {lang:?}");
            }
        }
        let mut all = Msg::ALL.to_vec();
        all.dedup();
        assert_eq!(all.len(), 37);
    }

    #[test]
    fn the_long_texts_are_translated() {
        for msg in [Msg::Title, Msg::Intro, Msg::NoGroups, Msg::Groups] {
            for lang in &Lang::ALL[1..] {
                assert_ne!(text(msg, *lang), text(msg, Lang::En), "{msg:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert_eq!(text(Msg::Path, Lang::Pt), "Ficheiro");
        assert_eq!(text(Msg::FilesRead, Lang::Pt), "Ficheiros lidos");
    }
}
