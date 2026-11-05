// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the lengths and pairs, `PHOENIX-LEN-<NNN>`, with their titles in the
//! eight languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The length settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-LEN-001";
/// The file of groups and pairs is not valid.
pub const FILE_INVALID: &str = "PHOENIX-LEN-002";
/// A net named in the file is not on the board.
pub const NET_NOT_FOUND: &str = "PHOENIX-LEN-003";
/// The nets of a group differ in length by more than the tolerance.
pub const GROUP_MISMATCH: &str = "PHOENIX-LEN-004";
/// The two nets of a pair differ in length by more than allowed.
pub const PAIR_SKEW: &str = "PHOENIX-LEN-005";
/// The tracks of a pair are not the width asked for.
pub const PAIR_WIDTH: &str = "PHOENIX-LEN-006";
/// The two tracks of a pair are closer than the separation asked for.
pub const PAIR_GAP_SMALL: &str = "PHOENIX-LEN-007";
/// The two tracks of a pair are farther apart than the separation asked for.
pub const PAIR_GAP_LARGE: &str = "PHOENIX-LEN-008";
/// The pair runs uncoupled for more than allowed.
pub const PAIR_UNCOUPLED: &str = "PHOENIX-LEN-009";
/// The delay of a layer could not be worked out from the stack-up.
pub const STACKUP_FALLBACK: &str = "PHOENIX-LEN-010";
/// A layer has no delay of its own and the default is used.
pub const LAYER_NO_DELAY: &str = "PHOENIX-LEN-011";
/// The length of an arc is an estimate.
pub const ARC_ESTIMATED: &str = "PHOENIX-LEN-012";
/// A net has no copper.
pub const NET_NO_COPPER: &str = "PHOENIX-LEN-013";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-LEN-014";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-LEN-015";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-LEN-016";
/// A limit of the report was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-LEN-017";
/// A net is not the target length.
pub const GROUP_TARGET: &str = "PHOENIX-LEN-018";
/// The pair has no coupled stretch.
pub const PAIR_NOT_COUPLED: &str = "PHOENIX-LEN-019";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Length settings are not valid",
            "La configuración de las longitudes no es válida",
            "Die Einstellungen der Längen sind ungültig",
            "Les réglages des longueurs ne sont pas valides",
            "Le impostazioni delle lunghezze non sono valide",
            "長さの設定が無効です",
            "长度设置无效",
            "As definições dos comprimentos não são válidas",
        ],
    ),
    (
        FILE_INVALID,
        [
            "Length groups file is not valid",
            "El archivo de grupos de longitud no es válido",
            "Die Datei der Längengruppen ist ungültig",
            "Le fichier des groupes de longueur n'est pas valide",
            "Il file dei gruppi di lunghezza non è valido",
            "長さグループのファイルが無効です",
            "长度分组文件无效",
            "O ficheiro dos grupos de comprimento não é válido",
        ],
    ),
    (
        NET_NOT_FOUND,
        [
            "Net is not on the board",
            "La red no está en la placa",
            "Das Netz ist nicht auf der Platine",
            "Le réseau n'est pas sur la carte",
            "La rete non è sulla scheda",
            "ネットが基板上にありません",
            "网络不在电路板上",
            "A rede não está na placa",
        ],
    ),
    (
        GROUP_MISMATCH,
        [
            "Nets of a group differ in length by more than the tolerance",
            "Las redes de un grupo difieren en longitud más que la tolerancia",
            "Die Netze einer Gruppe weichen in der Länge mehr als die Toleranz ab",
            "Les réseaux d'un groupe diffèrent en longueur de plus que la tolérance",
            "Le reti di un gruppo differiscono in lunghezza più della tolleranza",
            "グループのネット長の差が許容値を超えています",
            "组内网络的长度差超过了容差",
            "As redes de um grupo diferem em comprimento mais do que a tolerância",
        ],
    ),
    (
        PAIR_SKEW,
        [
            "Pair differs in length by more than allowed",
            "El par difiere en longitud más de lo permitido",
            "Das Paar weicht in der Länge mehr als erlaubt ab",
            "La paire diffère en longueur de plus que permis",
            "La coppia differisce in lunghezza più del consentito",
            "ペアの長さの差が許容値を超えています",
            "差分对的长度差超过允许值",
            "O par difere em comprimento mais do que o permitido",
        ],
    ),
    (
        PAIR_WIDTH,
        [
            "Track width of a pair is not the width asked for",
            "El ancho de pista de un par no es el pedido",
            "Die Leiterbahnbreite eines Paars ist nicht die verlangte",
            "La largeur de piste d'une paire n'est pas celle demandée",
            "La larghezza della pista di una coppia non è quella richiesta",
            "ペアのトラック幅が指定の幅ではありません",
            "差分对的走线宽度不是要求的宽度",
            "A largura da pista de um par não é a pedida",
        ],
    ),
    (
        PAIR_GAP_SMALL,
        [
            "Pair is closer together than the separation asked for",
            "El par está más junto que la separación pedida",
            "Das Paar liegt enger beieinander als verlangt",
            "La paire est plus serrée que la séparation demandée",
            "La coppia è più vicina della separazione richiesta",
            "ペアの間隔が指定より狭いです",
            "差分对的间距小于要求值",
            "O par está mais junto do que a separação pedida",
        ],
    ),
    (
        PAIR_GAP_LARGE,
        [
            "Pair is farther apart than the separation asked for",
            "El par está más separado que la separación pedida",
            "Das Paar liegt weiter auseinander als verlangt",
            "La paire est plus écartée que la séparation demandée",
            "La coppia è più distante della separazione richiesta",
            "ペアの間隔が指定より広いです",
            "差分对的间距大于要求值",
            "O par está mais afastado do que a separação pedida",
        ],
    ),
    (
        PAIR_UNCOUPLED,
        [
            "Pair runs uncoupled for more than allowed",
            "El par discurre sin acoplar más de lo permitido",
            "Das Paar läuft länger ungekoppelt als erlaubt",
            "La paire court découplée plus que permis",
            "La coppia corre non accoppiata più del consentito",
            "ペアが許容より長く非結合で走っています",
            "差分对非耦合走线长度超过允许值",
            "O par corre desacoplado mais do que o permitido",
        ],
    ),
    (
        STACKUP_FALLBACK,
        [
            "Delay could not be worked out from the stack-up",
            "No se pudo calcular el retardo a partir del apilado",
            "Die Laufzeit konnte aus dem Lagenaufbau nicht berechnet werden",
            "Le retard n'a pas pu être calculé à partir de l'empilement",
            "Non è stato possibile calcolare il ritardo dallo stackup",
            "スタックアップから遅延を計算できませんでした",
            "无法根据叠层计算延迟",
            "Não foi possível calcular o atraso a partir do empilhamento",
        ],
    ),
    (
        LAYER_NO_DELAY,
        [
            "Layer has no delay of its own, the default is used",
            "La capa no tiene retardo propio, se usa el predeterminado",
            "Die Lage hat keine eigene Laufzeit, der Standardwert wird verwendet",
            "La couche n'a pas de retard propre, la valeur par défaut est utilisée",
            "Lo strato non ha un ritardo proprio, si usa quello predefinito",
            "レイヤーに固有の遅延がないため既定値を使用します",
            "该层没有自己的延迟，使用默认值",
            "A camada não tem atraso próprio, é usado o predefinido",
        ],
    ),
    (
        ARC_ESTIMATED,
        [
            "Length of an arc is an estimate",
            "La longitud de un arco es una estimación",
            "Die Länge eines Bogens ist eine Schätzung",
            "La longueur d'un arc est une estimation",
            "La lunghezza di un arco è una stima",
            "円弧の長さは推定値です",
            "圆弧长度为估算值",
            "O comprimento de um arco é uma estimativa",
        ],
    ),
    (
        NET_NO_COPPER,
        [
            "Net has no copper",
            "La red no tiene cobre",
            "Das Netz hat kein Kupfer",
            "Le réseau n'a pas de cuivre",
            "La rete non ha rame",
            "ネットに銅がありません",
            "网络没有铜箔",
            "A rede não tem cobre",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "The output file exists or cannot be written",
            "El archivo de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di output esiste o non può essere scritto",
            "出力ファイルが既に存在するか書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Board, stack-up or lengths file cannot be read",
            "No se puede leer la placa, el apilado o el archivo de longitudes",
            "Platine, Lagenaufbau oder Längendatei können nicht gelesen werden",
            "La carte, l'empilement ou le fichier des longueurs sont illisibles",
            "Scheda, stackup o file delle lunghezze non leggibili",
            "基板・スタックアップ・長さファイルを読み込めません",
            "无法读取电路板、叠层或长度文件",
            "A placa, o empilhamento ou o ficheiro de comprimentos não podem ser lidos",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the length report was exceeded",
            "Se superó un límite del informe de longitudes",
            "Ein Limit des Längenberichts wurde überschritten",
            "Une limite du rapport de longueurs a été dépassée",
            "Un limite del rapporto delle lunghezze è stato superato",
            "長さレポートの上限を超えました",
            "超出了长度报告的限制",
            "Foi excedido um limite do relatório de comprimentos",
        ],
    ),
    (
        GROUP_TARGET,
        [
            "Net is not the target length",
            "La red no tiene la longitud objetivo",
            "Das Netz hat nicht die Ziellänge",
            "Le réseau n'a pas la longueur visée",
            "La rete non ha la lunghezza obiettivo",
            "ネットが目標の長さではありません",
            "网络长度未达到目标长度",
            "A rede não tem o comprimento alvo",
        ],
    ),
    (
        PAIR_NOT_COUPLED,
        [
            "Pair has no coupled stretch",
            "El par no tiene tramo acoplado",
            "Das Paar hat keine gekoppelte Strecke",
            "La paire n'a pas de tronçon couplé",
            "La coppia non ha un tratto accoppiato",
            "ペアに結合区間がありません",
            "差分对没有耦合段",
            "O par não tem troço acoplado",
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

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something the report did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// A rule that is proven to be broken.
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

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-LEN-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The net, group or pair it is about, by name.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a net, a group or a pair.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: Option<&str>,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            subject: subject.map(str::to_owned),
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title, the subject and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if let Some(subject) = &self.subject {
            out.push_str(&format!(" [{subject}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the lengths: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthError {
    /// The code, `PHOENIX-LEN-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl LengthError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> LengthError {
        LengthError {
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

impl fmt::Display for LengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for LengthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-LEN-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
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
        let e = LengthError::new(NET_NOT_FOUND, "USB_P");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-LEN-003: A rede não está na placa: USB_P"
        );
        assert_eq!(e.to_string(), "PHOENIX-LEN-003: USB_P");
        let bare = LengthError::new(NET_NO_COPPER, "");
        assert_eq!(bare.message(Lang::En), "PHOENIX-LEN-013: Net has no copper");
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(PAIR_SKEW, Severity::Error, Some("USB"), "skew 300000 nm");
        assert_eq!(
            f.line(Lang::En),
            "error: PHOENIX-LEN-005: Pair differs in length by more than allowed [USB]: skew 300000 nm"
        );
        assert!(Severity::Error > Severity::Info);
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
