// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the server mode, `PHOENIX-SERVER-<NNN>`, with their titles in the eight
//! languages of the suite, the error type that carries one and the notes of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The settings of the server mode are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SERVER-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SERVER-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-SERVER-003";
/// A limit of the request was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-SERVER-004";
/// Sending is not allowed by the settings.
pub const NETWORK_NOT_ALLOWED: &str = "PHOENIX-SERVER-005";
/// The link to the server is not set up.
pub const NOT_CONFIGURED: &str = "PHOENIX-SERVER-006";
/// The server could not be reached or refused the request.
pub const SERVER_FAILED: &str = "PHOENIX-SERVER-007";
/// The server answered something that cannot be read.
pub const BAD_ANSWER: &str = "PHOENIX-SERVER-008";
/// The job did not end in the time allowed.
pub const JOB_TIMEOUT: &str = "PHOENIX-SERVER-009";
/// The job ended on the server without a result.
pub const JOB_ENDED_BAD: &str = "PHOENIX-SERVER-010";
/// The request holds only a part of what the board has.
pub const PARTIAL_REQUEST: &str = "PHOENIX-SERVER-011";
/// Nothing was sent: the command made the request and showed it.
pub const DRY_RUN: &str = "PHOENIX-SERVER-012";
/// The board or the file has nothing for the report asked for.
pub const NOTHING_TO_SEND: &str = "PHOENIX-SERVER-013";
/// The token travels as plain text to an address that is not local.
pub const PLAIN_TOKEN: &str = "PHOENIX-SERVER-014";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Server settings are not valid",
            "La configuración del servidor no es válida",
            "Die Einstellungen des Servers sind ungültig",
            "Les réglages du serveur ne sont pas valides",
            "Le impostazioni del server non sono valide",
            "サーバーの設定が無効です",
            "服务器设置无效",
            "As definições do servidor não são válidas",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Befehlsoption ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un archivo de entrada",
            "Eine Eingabedatei kann nicht gelesen werden",
            "Un fichier d'entrée ne peut pas être lu",
            "Non si può leggere un file di ingresso",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the request was exceeded",
            "Se superó un límite de la petición",
            "Eine Grenze der Anfrage wurde überschritten",
            "Une limite de la requête a été dépassée",
            "È stato superato un limite della richiesta",
            "リクエストの上限を超えました",
            "超出了请求的限制",
            "Foi excedido um limite do pedido",
        ],
    ),
    (
        NETWORK_NOT_ALLOWED,
        [
            "Sending to the server is not allowed by the settings",
            "Los ajustes no permiten enviar al servidor",
            "Die Einstellungen erlauben das Senden an den Server nicht",
            "Les réglages n'autorisent pas l'envoi au serveur",
            "Le impostazioni non consentono l'invio al server",
            "設定でサーバーへの送信が許可されていません",
            "设置不允许向服务器发送",
            "As definições não permitem enviar para o servidor",
        ],
    ),
    (
        NOT_CONFIGURED,
        [
            "The link to the server is not set up",
            "El enlace con el servidor no está configurado",
            "Die Verbindung zum Server ist nicht eingerichtet",
            "La liaison avec le serveur n'est pas configurée",
            "Il collegamento con il server non è configurato",
            "サーバーとの接続が設定されていません",
            "尚未设置与服务器的连接",
            "A ligação ao servidor não está configurada",
        ],
    ),
    (
        SERVER_FAILED,
        [
            "The server could not be reached or refused the request",
            "No se pudo llegar al servidor o este rechazó la petición",
            "Der Server war nicht erreichbar oder hat die Anfrage abgelehnt",
            "Le serveur n'a pas pu être joint ou a refusé la requête",
            "Il server non è raggiungibile o ha rifiutato la richiesta",
            "サーバーに接続できないか、リクエストが拒否されました",
            "无法连接服务器，或服务器拒绝了请求",
            "Não foi possível chegar ao servidor ou este recusou o pedido",
        ],
    ),
    (
        BAD_ANSWER,
        [
            "The server answered something that cannot be read",
            "El servidor respondió algo que no se puede leer",
            "Der Server hat etwas Unlesbares geantwortet",
            "Le serveur a répondu quelque chose d'illisible",
            "Il server ha risposto qualcosa che non si può leggere",
            "サーバーの応答を読み取れません",
            "服务器的回答无法解读",
            "O servidor respondeu algo que não se consegue ler",
        ],
    ),
    (
        JOB_TIMEOUT,
        [
            "The job did not end in the time allowed",
            "El trabajo no terminó en el tiempo permitido",
            "Der Auftrag ist in der erlaubten Zeit nicht fertig geworden",
            "Le travail ne s'est pas terminé dans le temps imparti",
            "Il lavoro non è terminato nel tempo consentito",
            "ジョブが許容時間内に終了しませんでした",
            "作业未在允许的时间内结束",
            "O trabalho não terminou no tempo permitido",
        ],
    ),
    (
        JOB_ENDED_BAD,
        [
            "The job ended on the server without a result",
            "El trabajo terminó en el servidor sin resultado",
            "Der Auftrag endete auf dem Server ohne Ergebnis",
            "Le travail s'est terminé sur le serveur sans résultat",
            "Il lavoro è terminato sul server senza risultato",
            "ジョブは結果なしでサーバー上で終了しました",
            "作业在服务器上结束，没有结果",
            "O trabalho terminou no servidor sem resultado",
        ],
    ),
    (
        PARTIAL_REQUEST,
        [
            "The request holds only a part of what the board has",
            "La petición lleva solo una parte de lo que tiene la placa",
            "Die Anfrage enthält nur einen Teil dessen, was die Platine hat",
            "La requête ne contient qu'une partie de ce que la carte contient",
            "La richiesta contiene solo una parte di ciò che ha la scheda",
            "リクエストには基板の内容の一部しか含まれていません",
            "请求只包含电路板的一部分内容",
            "O pedido leva apenas uma parte do que a placa tem",
        ],
    ),
    (
        DRY_RUN,
        [
            "Nothing was sent: the request was made and shown",
            "No se envió nada: la petición se preparó y se mostró",
            "Es wurde nichts gesendet: die Anfrage wurde erstellt und angezeigt",
            "Rien n'a été envoyé : la requête a été préparée et affichée",
            "Non è stato inviato nulla: la richiesta è stata preparata e mostrata",
            "何も送信していません。リクエストを作成して表示しました",
            "未发送任何内容：已生成并显示请求",
            "Nada foi enviado: o pedido foi preparado e mostrado",
        ],
    ),
    (
        NOTHING_TO_SEND,
        [
            "The board or the file has nothing for the report asked for",
            "La placa o el archivo no tiene nada para el informe pedido",
            "Die Platine oder die Datei hat nichts für den verlangten Bericht",
            "La carte ou le fichier n'a rien pour le rapport demandé",
            "La scheda o il file non ha nulla per il rapporto richiesto",
            "基板またはファイルに、要求されたレポート用のデータがありません",
            "电路板或文件中没有所要求报告的内容",
            "A placa ou o ficheiro não tem nada para o relatório pedido",
        ],
    ),
    (
        PLAIN_TOKEN,
        [
            "The token travels as plain text to an address that is not local",
            "El token viaja en texto claro a una dirección que no es local",
            "Das Token wird im Klartext an eine nicht lokale Adresse gesendet",
            "Le jeton voyage en clair vers une adresse qui n'est pas locale",
            "Il token viaggia in chiaro verso un indirizzo non locale",
            "トークンが平文のままローカルでないアドレスへ送られます",
            "令牌以明文发送到非本地地址",
            "O token viaja em texto simples para um endereço que não é local",
        ],
    ),
];

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// How serious a note of a report is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something the command did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// The report cannot be trusted or was not made.
    Error,
}

impl Severity {
    /// The word used in the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A note of a report: a registered code and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The code, `PHOENIX-SERVER-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Note {
    /// A note with a code and its detail.
    pub fn new(code: &'static str, severity: Severity, detail: impl Into<String>) -> Note {
        Note {
            code,
            severity,
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the server mode: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    /// The code, `PHOENIX-SERVER-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English, or in the language of the window when it is the
    /// text of the server.
    pub detail: String,
}

impl ServerError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> ServerError {
        ServerError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ServerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SERVER-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
        assert_eq!(TITLES.len(), 14);
    }

    #[test]
    fn no_title_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES.iter() {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = ServerError::new(NOT_CONFIGURED, "server_address is empty");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-SERVER-006: A ligação ao servidor não está configurada: server_address is empty"
        );
        assert_eq!(e.to_string(), "PHOENIX-SERVER-006: server_address is empty");
        let bare = ServerError::new(DRY_RUN, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-SERVER-012: Nothing was sent: the request was made and shown"
        );
    }

    #[test]
    fn a_note_line_names_severity_code_title_and_detail() {
        let n = Note::new(PARTIAL_REQUEST, Severity::Info, "arcs by their chord");
        assert_eq!(
            n.line(Lang::En),
            "info: PHOENIX-SERVER-011: The request holds only a part of what the board has: arcs by their chord"
        );
        assert!(Severity::Error > Severity::Warning && Severity::Warning > Severity::Info);
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
    }
}
