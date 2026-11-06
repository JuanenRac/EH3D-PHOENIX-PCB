// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the signal integrity estimates, `PHOENIX-SI-<NNN>`, with their titles in
//! the eight languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

/// The signal integrity settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SI-001";
/// A net named on the command line is not on the board.
pub const NET_NOT_FOUND: &str = "PHOENIX-SI-002";
/// Near-end crosstalk is above the limit.
pub const NEXT_HIGH: &str = "PHOENIX-SI-003";
/// Far-end crosstalk is above the limit.
pub const FEXT_HIGH: &str = "PHOENIX-SI-004";
/// The coupling is too strong for the weak-coupling estimate.
pub const COUPLING_STRONG: &str = "PHOENIX-SI-005";
/// A line is electrically long, so its reflections matter.
pub const LONG_LINE: &str = "PHOENIX-SI-006";
/// The overshoot at the load is above the limit.
pub const OVERSHOOT: &str = "PHOENIX-SI-007";
/// The ringback below the final level is above the limit.
pub const RINGBACK: &str = "PHOENIX-SI-008";
/// The stack-up cannot be used and the defaults of the settings are used.
pub const STACKUP_FALLBACK: &str = "PHOENIX-SI-009";
/// The height of the eye is below the limit.
pub const EYE_HEIGHT: &str = "PHOENIX-SI-010";
/// The width of the eye is below the limit.
pub const EYE_WIDTH: &str = "PHOENIX-SI-011";
/// A track crosses the edge or a split of its reference plane.
pub const PLANE_SPLIT: &str = "PHOENIX-SI-012";
/// A via stub is too long for the rise time.
pub const STUB_LONG: &str = "PHOENIX-SI-013";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SI-014";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-SI-015";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-SI-016";
/// A limit of the analysis was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-SI-017";
/// The loss of a line slows the rise time by more than the limit.
pub const LOSS_DEGRADES: &str = "PHOENIX-SI-018";
/// No reference plane was found next to a layer.
pub const NO_REFERENCE: &str = "PHOENIX-SI-019";
/// A simulation was cut at the limit of steps.
pub const SIM_TRUNCATED: &str = "PHOENIX-SI-020";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Signal integrity settings are not valid",
            "La configuración de la integridad de señal no es válida",
            "Die Einstellungen der Signalintegrität sind ungültig",
            "Les réglages de l'intégrité du signal ne sont pas valides",
            "Le impostazioni dell'integrità del segnale non sono valide",
            "シグナルインテグリティの設定が無効です",
            "信号完整性设置无效",
            "As definições da integridade do sinal não são válidas",
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
        NEXT_HIGH,
        [
            "Near-end crosstalk is above the limit",
            "La diafonía cercana supera el límite",
            "Das Nahnebensprechen liegt über dem Grenzwert",
            "La diaphonie proche dépasse la limite",
            "La diafonia vicina supera il limite",
            "近端クロストークが上限を超えています",
            "近端串扰超过限值",
            "A diafonia próxima excede o limite",
        ],
    ),
    (
        FEXT_HIGH,
        [
            "Far-end crosstalk is above the limit",
            "La diafonía lejana supera el límite",
            "Das Fernnebensprechen liegt über dem Grenzwert",
            "La diaphonie lointaine dépasse la limite",
            "La diafonia lontana supera il limite",
            "遠端クロストークが上限を超えています",
            "远端串扰超过限值",
            "A diafonia distante excede o limite",
        ],
    ),
    (
        COUPLING_STRONG,
        [
            "Coupling is too strong for the weak-coupling estimate",
            "El acoplamiento es demasiado fuerte para la estimación de acoplamiento débil",
            "Die Kopplung ist für die Schätzung bei schwacher Kopplung zu stark",
            "Le couplage est trop fort pour l'estimation à couplage faible",
            "L'accoppiamento è troppo forte per la stima ad accoppiamento debole",
            "結合が強すぎて弱結合の推定が成り立ちません",
            "耦合过强，弱耦合估算不适用",
            "O acoplamento é demasiado forte para a estimativa de acoplamento fraco",
        ],
    ),
    (
        LONG_LINE,
        [
            "Line is electrically long, so its reflections matter",
            "La línea es eléctricamente larga: sus reflexiones importan",
            "Die Leitung ist elektrisch lang, ihre Reflexionen zählen",
            "La ligne est électriquement longue : ses réflexions comptent",
            "La linea è elettricamente lunga: le sue riflessioni contano",
            "線路が電気的に長く、反射を考慮する必要があります",
            "线路电气长度较长，必须考虑反射",
            "A linha é eletricamente longa: as suas reflexões contam",
        ],
    ),
    (
        OVERSHOOT,
        [
            "Overshoot at the load is above the limit",
            "El sobreimpulso en la carga supera el límite",
            "Das Überschwingen an der Last liegt über dem Grenzwert",
            "Le dépassement sur la charge dépasse la limite",
            "L'overshoot sul carico supera il limite",
            "負荷でのオーバーシュートが上限を超えています",
            "负载处的过冲超过限值",
            "O sobreimpulso na carga excede o limite",
        ],
    ),
    (
        RINGBACK,
        [
            "Ringback below the final level is above the limit",
            "La caída por debajo del nivel final supera el límite",
            "Das Zurückschwingen unter den Endpegel liegt über dem Grenzwert",
            "Le retour sous le niveau final dépasse la limite",
            "Il ritorno sotto il livello finale supera il limite",
            "最終レベルを下回るリンギングが上限を超えています",
            "低于终值的回跳超过限值",
            "O retorno abaixo do nível final excede o limite",
        ],
    ),
    (
        STACKUP_FALLBACK,
        [
            "Stack-up cannot be used, the defaults of the settings are used",
            "No se puede usar el apilado: se usan los valores por defecto de la configuración",
            "Der Lagenaufbau ist nicht verwendbar, es gelten die Standardwerte der Einstellungen",
            "L'empilement n'est pas utilisable : les valeurs par défaut des réglages sont employées",
            "Lo stackup non è utilizzabile: si usano i valori predefiniti delle impostazioni",
            "スタックアップを使えないため、設定の既定値を使用します",
            "叠层不可用，改用设置中的默认值",
            "O empilhamento não é utilizável: usam-se os valores por omissão das definições",
        ],
    ),
    (
        EYE_HEIGHT,
        [
            "Eye height is below the limit",
            "La altura del ojo está por debajo del límite",
            "Die Augenhöhe liegt unter dem Grenzwert",
            "La hauteur de l'œil est sous la limite",
            "L'altezza dell'occhio è sotto il limite",
            "アイの高さが下限を下回っています",
            "眼高低于限值",
            "A altura do olho está abaixo do limite",
        ],
    ),
    (
        EYE_WIDTH,
        [
            "Eye width is below the limit",
            "El ancho del ojo está por debajo del límite",
            "Die Augenbreite liegt unter dem Grenzwert",
            "La largeur de l'œil est sous la limite",
            "La larghezza dell'occhio è sotto il limite",
            "アイの幅が下限を下回っています",
            "眼宽低于限值",
            "A largura do olho está abaixo do limite",
        ],
    ),
    (
        PLANE_SPLIT,
        [
            "Track crosses the edge or a split of its reference plane",
            "La pista cruza el borde o una división de su plano de referencia",
            "Die Leiterbahn quert den Rand oder eine Trennung ihrer Bezugsfläche",
            "La piste franchit le bord ou une coupure de son plan de référence",
            "La pista attraversa il bordo o una divisione del suo piano di riferimento",
            "トラックが基準プレーンの端または分割を横切っています",
            "走线跨越了参考平面的边缘或分割",
            "A pista cruza o bordo ou uma divisão do seu plano de referência",
        ],
    ),
    (
        STUB_LONG,
        [
            "Via stub is too long for the rise time",
            "El tocón de la vía es demasiado largo para el tiempo de subida",
            "Der Via-Stummel ist für die Anstiegszeit zu lang",
            "Le tronçon du via est trop long pour le temps de montée",
            "Lo stub del via è troppo lungo per il tempo di salita",
            "ビアのスタブが立ち上がり時間に対して長すぎます",
            "过孔残桩相对于上升时间过长",
            "O cotoco da via é demasiado longo para o tempo de subida",
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
            "Board, stack-up or settings file cannot be read",
            "No se puede leer la placa, el apilado o el archivo de configuración",
            "Platine, Lagenaufbau oder Einstellungsdatei können nicht gelesen werden",
            "La carte, l'empilement ou le fichier des réglages sont illisibles",
            "Scheda, stackup o file delle impostazioni non leggibili",
            "基板・スタックアップ・設定ファイルを読み込めません",
            "无法读取电路板、叠层或设置文件",
            "A placa, o empilhamento ou o ficheiro de definições não podem ser lidos",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the analysis was exceeded",
            "Se superó un límite del análisis",
            "Ein Limit der Analyse wurde überschritten",
            "Une limite de l'analyse a été dépassée",
            "Un limite dell'analisi è stato superato",
            "解析の上限を超えました",
            "超出了分析的限制",
            "Foi excedido um limite da análise",
        ],
    ),
    (
        LOSS_DEGRADES,
        [
            "Line loss slows the rise time by more than the limit",
            "La pérdida de la línea alarga el tiempo de subida más que el límite",
            "Die Leitungsdämpfung verlängert die Anstiegszeit über den Grenzwert hinaus",
            "La perte de la ligne allonge le temps de montée au-delà de la limite",
            "La perdita della linea allunga il tempo di salita oltre il limite",
            "線路の損失により立ち上がり時間が上限以上に遅くなります",
            "线路损耗使上升时间的增加超过限值",
            "A perda da linha alonga o tempo de subida mais do que o limite",
        ],
    ),
    (
        NO_REFERENCE,
        [
            "No reference plane found next to a layer",
            "No se encontró plano de referencia junto a una capa",
            "Neben einer Lage wurde keine Bezugsfläche gefunden",
            "Aucun plan de référence trouvé à côté d'une couche",
            "Nessun piano di riferimento trovato accanto a uno strato",
            "レイヤーの隣に基準プレーンが見つかりません",
            "层旁未找到参考平面",
            "Não foi encontrado plano de referência junto a uma camada",
        ],
    ),
    (
        SIM_TRUNCATED,
        [
            "Simulation was cut at the limit of steps",
            "La simulación se cortó en el límite de pasos",
            "Die Simulation wurde am Schrittlimit abgebrochen",
            "La simulation a été coupée à la limite de pas",
            "La simulazione è stata interrotta al limite di passi",
            "シミュレーションはステップ数の上限で打ち切られました",
            "仿真在步数上限处被截断",
            "A simulação foi cortada no limite de passos",
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
    /// Something the analysis did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// A limit of the settings that is exceeded.
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

/// An error of the signal integrity estimates: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiError {
    /// The code, `PHOENIX-SI-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl SiError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SiError {
        SiError {
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

impl fmt::Display for SiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SiError {}

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-SI-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The net, pair of nets or layer it is about, by name.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a net, a pair of nets or a layer.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SI-{:03}", i + 1));
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
        let e = SiError::new(NET_NOT_FOUND, "CLK");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-SI-002: A rede não está na placa: CLK"
        );
        assert_eq!(e.to_string(), "PHOENIX-SI-002: CLK");
        let bare = SiError::new(OPTION_INVALID, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-SI-014: A command option is not valid"
        );
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(NEXT_HIGH, Severity::Error, Some("A/B"), "12.5 %");
        assert_eq!(
            f.line(Lang::En),
            "error: PHOENIX-SI-003: Near-end crosstalk is above the limit [A/B]: 12.5 %"
        );
        assert!(Severity::Error > Severity::Info);
        assert_eq!(title("PHOENIX-SI-999", Lang::En), "");
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
