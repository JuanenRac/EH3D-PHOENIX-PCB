// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the circuit simulator, `PHOENIX-SIM-<NNN>`, with their titles in the
//! eight languages of the suite, and the error type that carries one.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// A netlist line that is not valid.
pub const NETLIST_SYNTAX: &str = "PHOENIX-SIM-001";
/// A value, unit or expression that is not valid.
pub const VALUE_INVALID: &str = "PHOENIX-SIM-002";
/// An element, directive or model that is not supported.
pub const UNSUPPORTED: &str = "PHOENIX-SIM-003";
/// A name used twice.
pub const DUPLICATE_NAME: &str = "PHOENIX-SIM-004";
/// A reference to something that is not defined.
pub const UNDEFINED: &str = "PHOENIX-SIM-005";
/// A subcircuit that is not valid.
pub const SUBCKT_INVALID: &str = "PHOENIX-SIM-006";
/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SIM-007";
/// A limit of the settings was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-SIM-008";
/// The equations of the circuit cannot be solved.
pub const SINGULAR_MATRIX: &str = "PHOENIX-SIM-009";
/// A node with no path to ground.
pub const FLOATING_NODE: &str = "PHOENIX-SIM-010";
/// The Newton-Raphson iteration did not converge.
pub const NO_CONVERGENCE: &str = "PHOENIX-SIM-011";
/// An analysis request that is not valid.
pub const ANALYSIS_INVALID: &str = "PHOENIX-SIM-012";
/// An output file cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-SIM-013";
/// A part of the schematic that could not be translated.
pub const PART_SKIPPED: &str = "PHOENIX-SIM-014";
/// The circuit has no ground node.
pub const NO_GROUND: &str = "PHOENIX-SIM-015";
/// A command option that is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SIM-016";
/// A result that is not a finite number.
pub const NOT_FINITE: &str = "PHOENIX-SIM-017";
/// The simulation disagrees with the RF filter response.
pub const COHERENCE_FAILED: &str = "PHOENIX-SIM-018";
/// The time step reached the minimum.
pub const STEP_TOO_SMALL: &str = "PHOENIX-SIM-019";
/// A pin of the schematic that is on no net.
pub const PIN_UNCONNECTED: &str = "PHOENIX-SIM-020";
/// An input file that cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-SIM-021";
/// A model of a kind that does not fit the element that uses it.
pub const MODEL_MISMATCH: &str = "PHOENIX-SIM-022";
/// An `.include` or a model file that is not allowed or cannot be read.
pub const INCLUDE_REFUSED: &str = "PHOENIX-SIM-023";
/// An analysis directive of the netlist (`.op`, `.dc`, `.tran`, `.ac`) that is not valid.
pub const DIRECTIVE_INVALID: &str = "PHOENIX-SIM-024";
/// A `.step` line or a step request that is not valid.
pub const STEP_INVALID: &str = "PHOENIX-SIM-025";
/// A `.measure` line or a measure request that is not valid.
pub const MEASURE_INVALID: &str = "PHOENIX-SIM-026";
/// A measure that cannot be computed from the results.
pub const MEASURE_FAILED: &str = "PHOENIX-SIM-027";
/// A Monte Carlo request or a tolerance that is not valid.
pub const MC_INVALID: &str = "PHOENIX-SIM-028";
/// The poles and zeros cannot be found reliably.
pub const FIT_FAILED: &str = "PHOENIX-SIM-029";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        NETLIST_SYNTAX,
        [
            "Netlist line is not valid",
            "La línea de la netlist no es válida",
            "Die Zeile der Netzliste ist ungültig",
            "La ligne de la netlist n'est pas valide",
            "La riga della netlist non è valida",
            "ネットリストの行が無効です",
            "网表行无效",
            "A linha da netlist não é válida",
        ],
    ),
    (
        VALUE_INVALID,
        [
            "Value is not valid",
            "El valor no es válido",
            "Der Wert ist ungültig",
            "La valeur n'est pas valide",
            "Il valore non è valido",
            "値が無効です",
            "数值无效",
            "O valor não é válido",
        ],
    ),
    (
        UNSUPPORTED,
        [
            "Element, directive or model is not supported",
            "Elemento, directiva o modelo no admitido",
            "Element, Direktive oder Modell wird nicht unterstützt",
            "Élément, directive ou modèle non pris en charge",
            "Elemento, direttiva o modello non supportato",
            "サポートされていない素子、ディレクティブまたはモデルです",
            "不支持的元件、指令或模型",
            "Elemento, diretiva ou modelo não suportado",
        ],
    ),
    (
        DUPLICATE_NAME,
        [
            "Name is used twice",
            "Nombre usado dos veces",
            "Name wird zweimal verwendet",
            "Nom utilisé deux fois",
            "Nome usato due volte",
            "名前が重複しています",
            "名称重复使用",
            "Nome usado duas vezes",
        ],
    ),
    (
        UNDEFINED,
        [
            "Reference to something that is not defined",
            "Referencia a algo que no está definido",
            "Verweis auf etwas, das nicht definiert ist",
            "Référence à quelque chose qui n'est pas défini",
            "Riferimento a qualcosa che non è definito",
            "未定義のものを参照しています",
            "引用了未定义的内容",
            "Referência a algo que não está definido",
        ],
    ),
    (
        SUBCKT_INVALID,
        [
            "Subcircuit is not valid",
            "El subcircuito no es válido",
            "Der Unterschaltkreis ist ungültig",
            "Le sous-circuit n'est pas valide",
            "Il sottocircuito non è valido",
            "サブサーキットが無効です",
            "子电路无效",
            "O subcircuito não é válido",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Simulator settings are not valid",
            "La configuración del simulador no es válida",
            "Die Einstellungen des Simulators sind ungültig",
            "Les réglages du simulateur ne sont pas valides",
            "Le impostazioni del simulatore non sono valide",
            "シミュレータの設定が無効です",
            "模拟器设置无效",
            "As definições do simulador não são válidas",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "Limit of the settings exceeded",
            "Se superó un límite de la configuración",
            "Eine Grenze der Einstellungen wurde überschritten",
            "Une limite des réglages est dépassée",
            "Superato un limite delle impostazioni",
            "設定の上限を超えました",
            "超出了设置的限制",
            "Foi excedido um limite das definições",
        ],
    ),
    (
        SINGULAR_MATRIX,
        [
            "Circuit equations cannot be solved (singular matrix)",
            "No se pueden resolver las ecuaciones del circuito (matriz singular)",
            "Die Schaltungsgleichungen sind nicht lösbar (singuläre Matrix)",
            "Les équations du circuit ne peuvent pas être résolues (matrice singulière)",
            "Le equazioni del circuito non sono risolvibili (matrice singolare)",
            "回路方程式を解けません（特異行列）",
            "无法求解电路方程（奇异矩阵）",
            "Não é possível resolver as equações do circuito (matriz singular)",
        ],
    ),
    (
        FLOATING_NODE,
        [
            "Node has no path to ground",
            "Nodo sin camino a tierra",
            "Knoten ohne Pfad zur Masse",
            "Nœud sans chemin vers la masse",
            "Nodo senza percorso verso massa",
            "グランドへの経路がないノードです",
            "节点没有到地的通路",
            "Nó sem caminho para a massa",
        ],
    ),
    (
        NO_CONVERGENCE,
        [
            "Newton-Raphson iteration did not converge",
            "La iteración de Newton-Raphson no convergió",
            "Die Newton-Raphson-Iteration konvergierte nicht",
            "L'itération de Newton-Raphson n'a pas convergé",
            "L'iterazione di Newton-Raphson non è convergita",
            "ニュートン・ラフソン法が収束しませんでした",
            "牛顿-拉夫逊迭代未收敛",
            "A iteração de Newton-Raphson não convergiu",
        ],
    ),
    (
        ANALYSIS_INVALID,
        [
            "Analysis request is not valid",
            "La petición de análisis no es válida",
            "Die Analyseanforderung ist ungültig",
            "La demande d'analyse n'est pas valide",
            "La richiesta di analisi non è valida",
            "解析の指定が無効です",
            "分析请求无效",
            "O pedido de análise não é válido",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Output file cannot be written",
            "No se puede escribir el archivo de salida",
            "Die Ausgabedatei kann nicht geschrieben werden",
            "Le fichier de sortie ne peut pas être écrit",
            "Il file di uscita non può essere scritto",
            "出力ファイルを書き込めません",
            "无法写入输出文件",
            "Não é possível escrever o ficheiro de saída",
        ],
    ),
    (
        PART_SKIPPED,
        [
            "Schematic part could not be translated",
            "No se pudo traducir una pieza del esquema",
            "Ein Teil des Schaltplans konnte nicht übersetzt werden",
            "Une pièce du schéma n'a pas pu être traduite",
            "Un componente dello schema non ha potuto essere tradotto",
            "回路図の部品を変換できませんでした",
            "无法转换原理图中的元件",
            "Não foi possível traduzir uma peça do esquema",
        ],
    ),
    (
        NO_GROUND,
        [
            "No ground node",
            "No hay nodo de tierra",
            "Kein Massenknoten vorhanden",
            "Pas de nœud de masse",
            "Nessun nodo di massa",
            "グランドノードがありません",
            "没有接地节点",
            "Não há nó de massa",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "La opción del comando no es válida",
            "Die Option des Befehls ist ungültig",
            "L'option de la commande n'est pas valide",
            "L'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "A opção do comando não é válida",
        ],
    ),
    (
        NOT_FINITE,
        [
            "Result is not a finite number",
            "El resultado no es un número finito",
            "Das Ergebnis ist keine endliche Zahl",
            "Le résultat n'est pas un nombre fini",
            "Il risultato non è un numero finito",
            "結果が有限の数ではありません",
            "结果不是有限数",
            "O resultado não é um número finito",
        ],
    ),
    (
        COHERENCE_FAILED,
        [
            "Simulation disagrees with the RF filter response",
            "La simulación no coincide con la respuesta del filtro RF",
            "Die Simulation weicht vom Frequenzgang des HF-Filters ab",
            "La simulation diffère de la réponse du filtre RF",
            "La simulazione non coincide con la risposta del filtro RF",
            "シミュレーションが RF フィルタの応答と一致しません",
            "模拟结果与射频滤波器响应不一致",
            "A simulação não coincide com a resposta do filtro RF",
        ],
    ),
    (
        STEP_TOO_SMALL,
        [
            "Time step reached the minimum",
            "El paso de tiempo llegó al mínimo",
            "Der Zeitschritt hat das Minimum erreicht",
            "Le pas de temps a atteint le minimum",
            "Il passo di tempo ha raggiunto il minimo",
            "時間刻みが下限に達しました",
            "时间步长已达最小值",
            "O passo de tempo atingiu o mínimo",
        ],
    ),
    (
        PIN_UNCONNECTED,
        [
            "Schematic pin is on no net",
            "Un pin del esquema no está en ninguna red",
            "Ein Pin des Schaltplans liegt an keinem Netz",
            "Une broche du schéma n'est sur aucun réseau",
            "Un pin dello schema non è su nessuna rete",
            "回路図のピンがどのネットにも接続されていません",
            "原理图引脚不在任何网络上",
            "Um pino do esquema não está em nenhuma rede",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Input file cannot be read",
            "No se puede leer el archivo de entrada",
            "Die Eingabedatei kann nicht gelesen werden",
            "Le fichier d'entrée ne peut pas être lu",
            "Il file di ingresso non può essere letto",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler o ficheiro de entrada",
        ],
    ),
    (
        MODEL_MISMATCH,
        [
            "Model does not fit the element",
            "El modelo no corresponde al elemento",
            "Das Modell passt nicht zum Element",
            "Le modèle ne convient pas à l'élément",
            "Il modello non è adatto all'elemento",
            "モデルが素子の種類に合っていません",
            "模型与元件不匹配",
            "O modelo não corresponde ao elemento",
        ],
    ),
    (
        INCLUDE_REFUSED,
        [
            "Include file is not allowed",
            "El fichero incluido no está permitido",
            "Die eingebundene Datei ist nicht erlaubt",
            "Le fichier inclus n'est pas autorisé",
            "Il file incluso non è consentito",
            "インクルードするファイルは許可されていません",
            "不允许包含该文件",
            "O ficheiro incluído não é permitido",
        ],
    ),
    (
        DIRECTIVE_INVALID,
        [
            "Analysis directive is not valid",
            "La directiva de análisis no es válida",
            "Die Analyse-Direktive ist ungültig",
            "La directive d'analyse n'est pas valide",
            "La direttiva di analisi non è valida",
            "解析ディレクティブが無効です",
            "分析指令无效",
            "A diretiva de análise não é válida",
        ],
    ),
    (
        STEP_INVALID,
        [
            "Parameter step (.step) is not valid",
            "El barrido de parámetro (.step) no es válido",
            "Der Parameterlauf (.step) ist ungültig",
            "Le balayage de paramètre (.step) n'est pas valide",
            "Lo scorrimento del parametro (.step) non è valido",
            ".step（パラメータ掃引）が無効です",
            ".step（参数扫描）无效",
            "O varrimento de parâmetro (.step) não é válido",
        ],
    ),
    (
        MEASURE_INVALID,
        [
            "Measurement (.measure) is not valid",
            "La medida (.measure) no es válida",
            "Die Messung (.measure) ist ungültig",
            "La mesure (.measure) n'est pas valide",
            "La misura (.measure) non è valida",
            ".measure（測定）が無効です",
            ".measure（测量）无效",
            "A medição (.measure) não é válida",
        ],
    ),
    (
        MEASURE_FAILED,
        [
            "Measurement cannot be computed",
            "No se puede calcular la medida",
            "Die Messung kann nicht berechnet werden",
            "La mesure ne peut pas être calculée",
            "La misura non può essere calcolata",
            "測定値を計算できません",
            "无法计算测量值",
            "Não é possível calcular a medição",
        ],
    ),
    (
        MC_INVALID,
        [
            "Monte Carlo request or tolerance is not valid",
            "La petición Monte Carlo o la tolerancia no es válida",
            "Die Monte-Carlo-Anforderung oder die Toleranz ist ungültig",
            "La demande Monte Carlo ou la tolérance n'est pas valide",
            "La richiesta Monte Carlo o la tolleranza non è valida",
            "モンテカルロの指定または公差が無効です",
            "蒙特卡罗请求或公差无效",
            "O pedido de Monte Carlo ou a tolerância não é válido",
        ],
    ),
    (
        FIT_FAILED,
        [
            "Poles and zeros cannot be found reliably",
            "No se pueden hallar los polos y ceros de forma fiable",
            "Pole und Nullstellen lassen sich nicht zuverlässig bestimmen",
            "Les pôles et les zéros ne peuvent pas être trouvés de façon fiable",
            "Poli e zeri non possono essere trovati in modo affidabile",
            "極と零点を確実には求められません",
            "无法可靠地求出极点和零点",
            "Não é possível encontrar os polos e os zeros de forma fiável",
        ],
    ),
];

/// The column of a language in the title table.
pub fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column(lang)])
}

/// An error of the simulator: a registered code, the line of the input when there is one, and
/// what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimError {
    /// The code, `PHOENIX-SIM-<NNN>`.
    pub code: &'static str,
    /// The line of the netlist (counted from 1) when the error belongs to one.
    pub line: Option<usize>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl SimError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SimError {
        SimError {
            code,
            line: None,
            detail: detail.into(),
        }
    }

    /// An error that belongs to a line of the input.
    pub fn at(code: &'static str, line: usize, detail: impl Into<String>) -> SimError {
        SimError {
            code,
            line: Some(line),
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title, the line and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if let Some(line) = self.line {
            out.push_str(&format!(": line {line}"));
        }
        if !self.detail.is_empty() {
            out.push_str(&format!(": {}", self.detail));
        }
        out
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code)?;
        if let Some(line) = self.line {
            write!(f, "line {line}: ")?;
        }
        write!(f, "{}", self.detail)
    }
}

impl std::error::Error for SimError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SIM-{:03}", i + 1));
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
    fn the_message_names_the_code_the_title_the_line_and_the_detail() {
        let e = SimError::at(NETLIST_SYNTAX, 7, "too few fields");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-SIM-001: A linha da netlist não é válida: line 7: too few fields"
        );
        assert_eq!(e.to_string(), "PHOENIX-SIM-001: line 7: too few fields");
        let bare = SimError::new(NO_GROUND, "");
        assert_eq!(bare.message(Lang::En), "PHOENIX-SIM-015: No ground node");
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
