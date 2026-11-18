// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the test points, `PHOENIX-TP-<NNN>`, with their titles in the eight
//! languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The test point settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-TP-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-TP-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-TP-003";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-TP-004";
/// A limit of the report was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-TP-005";
/// A net has fewer test points than the settings ask for.
pub const NET_NO_TESTPOINT: &str = "PHOENIX-TP-006";
/// The test points of a net are all on the side that is not probed.
pub const NET_WRONG_SIDE: &str = "PHOENIX-TP-007";
/// A test point is smaller than the smallest the probe can touch.
pub const PAD_SMALL: &str = "PHOENIX-TP-008";
/// Two test points are closer than the probes can reach.
pub const TOO_CLOSE: &str = "PHOENIX-TP-009";
/// A test point is closer to the edge of the board than allowed.
pub const NEAR_EDGE: &str = "PHOENIX-TP-010";
/// A test point is on no net.
pub const NO_NET: &str = "PHOENIX-TP-011";
/// No pad of the board was recognised as a test point.
pub const NONE_RECOGNISED: &str = "PHOENIX-TP-012";
/// A test point has no solder mask opening on its side.
pub const NO_MASK_OPENING: &str = "PHOENIX-TP-013";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Test point settings are not valid",
            "La configuración de los puntos de prueba no es válida",
            "Die Einstellungen der Testpunkte sind ungültig",
            "Les réglages des points de test ne sont pas valides",
            "Le impostazioni dei punti di test non sono valide",
            "テストポイントの設定が無効です",
            "测试点设置无效",
            "As definições dos pontos de teste não são válidas",
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
        OUTPUT_FAILED,
        [
            "The output file exists or cannot be written",
            "El archivo de salida ya existe o no se puede escribir",
            "Die Ausgabedatei existiert bereits oder kann nicht geschrieben werden",
            "Le fichier de sortie existe déjà ou ne peut pas être écrit",
            "Il file di uscita esiste già o non si può scrivere",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the report was exceeded",
            "Se superó un límite del informe",
            "Eine Grenze des Berichts wurde überschritten",
            "Une limite du rapport a été dépassée",
            "È stato superato un limite del rapporto",
            "レポートの上限を超えました",
            "超出了报告的限制",
            "Foi excedido um limite do relatório",
        ],
    ),
    (
        NET_NO_TESTPOINT,
        [
            "Net has no test point",
            "La red no tiene punto de prueba",
            "Das Netz hat keinen Testpunkt",
            "Le réseau n'a pas de point de test",
            "La rete non ha un punto di test",
            "ネットにテストポイントがありません",
            "网络没有测试点",
            "A rede não tem ponto de teste",
        ],
    ),
    (
        NET_WRONG_SIDE,
        [
            "Test points of the net are all on the side that is not probed",
            "Los puntos de prueba de la red están todos en la cara que no se sondea",
            "Alle Testpunkte des Netzes liegen auf der Seite, die nicht abgetastet wird",
            "Les points de test du réseau sont tous sur la face qui n'est pas sondée",
            "I punti di test della rete sono tutti sul lato che non viene sondato",
            "ネットのテストポイントはすべてプローブしない面にあります",
            "该网络的测试点全部在不探测的一面",
            "Os pontos de teste da rede estão todos na face que não é sondada",
        ],
    ),
    (
        PAD_SMALL,
        [
            "Test point is smaller than the probe can touch",
            "El punto de prueba es más pequeño de lo que la sonda puede tocar",
            "Der Testpunkt ist kleiner, als die Nadel treffen kann",
            "Le point de test est plus petit que ce que la sonde peut toucher",
            "Il punto di test è più piccolo di quanto la sonda possa toccare",
            "テストポイントがプローブの接触できる大きさより小さいです",
            "测试点小于探针可接触的尺寸",
            "O ponto de teste é mais pequeno do que a sonda consegue tocar",
        ],
    ),
    (
        TOO_CLOSE,
        [
            "Test points are closer together than the probes can reach",
            "Los puntos de prueba están más juntos de lo que las sondas alcanzan",
            "Die Testpunkte liegen enger beieinander, als die Nadeln erreichen",
            "Les points de test sont plus rapprochés que ce que les sondes atteignent",
            "I punti di test sono più vicini di quanto le sonde possano raggiungere",
            "テストポイントの間隔がプローブの届く距離より狭いです",
            "测试点之间的距离小于探针所能达到的间距",
            "Os pontos de teste estão mais juntos do que as sondas alcançam",
        ],
    ),
    (
        NEAR_EDGE,
        [
            "Test point is too close to the edge of the board",
            "El punto de prueba está demasiado cerca del borde de la placa",
            "Der Testpunkt liegt zu nah am Rand der Platine",
            "Le point de test est trop près du bord de la carte",
            "Il punto di test è troppo vicino al bordo della scheda",
            "テストポイントが基板の端に近すぎます",
            "测试点离电路板边缘太近",
            "O ponto de teste está demasiado perto da borda da placa",
        ],
    ),
    (
        NO_NET,
        [
            "Test point is on no net",
            "El punto de prueba no está en ninguna red",
            "Der Testpunkt liegt an keinem Netz",
            "Le point de test n'est sur aucun réseau",
            "Il punto di test non è su alcuna rete",
            "テストポイントがどのネットにも接続されていません",
            "测试点不在任何网络上",
            "O ponto de teste não está em nenhuma rede",
        ],
    ),
    (
        NONE_RECOGNISED,
        [
            "No pad of the board was recognised as a test point",
            "Ningún pad de la placa se reconoció como punto de prueba",
            "Kein Pad der Platine wurde als Testpunkt erkannt",
            "Aucune pastille de la carte n'a été reconnue comme point de test",
            "Nessuna piazzola della scheda è stata riconosciuta come punto di test",
            "テストポイントとして認識されたパッドがありません",
            "没有焊盘被识别为测试点",
            "Nenhuma pastilha da placa foi reconhecida como ponto de teste",
        ],
    ),
    (
        NO_MASK_OPENING,
        [
            "Test point has no solder mask opening",
            "El punto de prueba no tiene apertura en la máscara de soldadura",
            "Der Testpunkt hat keine Öffnung im Lötstopplack",
            "Le point de test n'a pas d'ouverture dans le masque de soudure",
            "Il punto di test non ha un'apertura nella maschera di saldatura",
            "テストポイントにソルダーマスクの開口がありません",
            "测试点没有阻焊开窗",
            "O ponto de teste não tem abertura na máscara de soldadura",
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

    /// Reads `info`, `warning` or `error`.
    pub fn from_word(word: &str) -> Option<Severity> {
        match word {
            "info" => Some(Severity::Info),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-TP-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The net or test point it is about, by name.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a net or a test point.
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

/// An error of the test points: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPointError {
    /// The code, `PHOENIX-TP-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl TestPointError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> TestPointError {
        TestPointError {
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

impl fmt::Display for TestPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for TestPointError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-TP-{:03}", i + 1));
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
        let e = TestPointError::new(NET_NO_TESTPOINT, "GND");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-TP-006: A rede não tem ponto de teste: GND"
        );
        assert_eq!(e.to_string(), "PHOENIX-TP-006: GND");
        let bare = TestPointError::new(NO_NET, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-TP-011: Test point is on no net"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(NEAR_EDGE, Severity::Warning, Some("TP1"), "0.5 mm");
        assert_eq!(
            f.line(Lang::En),
            "warning: PHOENIX-TP-010: Test point is too close to the edge of the board [TP1]: 0.5 mm"
        );
        assert!(Severity::Error > Severity::Info);
    }

    #[test]
    fn a_severity_is_read_from_its_word() {
        assert_eq!(Severity::from_word("info"), Some(Severity::Info));
        assert_eq!(Severity::from_word("warning"), Some(Severity::Warning));
        assert_eq!(Severity::from_word("error"), Some(Severity::Error));
        assert_eq!(Severity::from_word("loud"), None);
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
