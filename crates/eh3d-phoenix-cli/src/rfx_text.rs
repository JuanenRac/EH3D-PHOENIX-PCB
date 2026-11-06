// SPDX-License-Identifier: GPL-3.0-or-later
//! The text and the JSON of the extended `phoenix rf` commands: matching networks, filters with
//! standard values, S parameters of a network with a mask, and the stability of an amplifier.
//! The tables are written in the eight languages of the suite (English, Spanish, German, French,
//! Italian, Japanese, Chinese and European Portuguese); the JSON uses fixed English keys and is
//! the same text for the same input. Every result is an estimate and says so.

use eh3d_phoenix_rf::circles::{StabilityReport, StableSide};
use eh3d_phoenix_rf::eseries::Series;
use eh3d_phoenix_rf::filterx::{SnapEffect, SnappedValue};
use eh3d_phoenix_rf::mask::MaskReport;
use eh3d_phoenix_rf::matching::ElementKind;
use eh3d_phoenix_rf::matchnet::{Bandwidth, LineEquivalents, Part};
use eh3d_phoenix_rf::netdesc::Figures;
use eh3d_phoenix_rf::network::{Mat2, Stability};
use eh3d_phoenix_rf::report::engineering;
use eh3d_phoenix_rf::trace::SmithTrace;
use eh3d_phoenix_rf::{Complex, Language};
use eh3d_phoenix_sim::filter::ResponsePoint;
use serde_json::{json, Value};

/// The words of the tables.
#[derive(Clone, Copy)]
pub(crate) enum L {
    Estimate,
    MatchTitle,
    Source,
    Load,
    Frequency,
    Element,
    Series,
    Shunt,
    Inductor,
    Capacitor,
    LowPass,
    HighPass,
    Mixed,
    NetworkQ,
    LoadedQ,
    VirtualR,
    BwRule,
    BwReturnLoss,
    NotFound,
    Check,
    SmithPath,
    Step,
    Impedance,
    Admittance,
    Reflection,
    LinesTitle,
    LineLine,
    Z0,
    EpsEff,
    SingleStub,
    DoubleStub,
    QuarterWave,
    Distance,
    StubLength,
    Spacing,
    SectionLength,
    SectionZ,
    Unavailable,
    Short,
    Open,
    FilterTitle,
    Approximation,
    Order,
    Band,
    PrototypeG,
    Components,
    StdSeries,
    Ideal,
    Standard,
    Error,
    Edge,
    EdgeShift,
    PassChange,
    Response,
    Gain,
    Phase,
    GroupDelay,
    Simulated,
    Analytic,
    SparamTitle,
    RefImpedance,
    ReturnLoss,
    InsertionLoss,
    MaskTitle,
    Segment,
    Points,
    WorstMargin,
    AtFrequency,
    Result,
    Pass,
    Fail,
    NoData,
    Overall,
    StabTitle,
    Determinant,
    UncondStable,
    Yes,
    No,
    SourceCircle,
    LoadCircle,
    Centre,
    Radius,
    StableSide,
    Inside,
    Outside,
    CutsChart,
    Msg,
    Mag,
    ConjMatch,
    NoConj,
    Parameters,
    GammaSource,
    GammaLoad,
    GainWithMatch,
    NoCircles,
}

/// How many words there are in [`L`] (the table has as many rows).
#[cfg(test)]
pub(crate) const WORD_COUNT: usize = L::NoCircles as usize + 1;

/// The table: one row per word, in the order of [`Language::ALL`]: English, Spanish, German,
/// French, Italian, Japanese, Chinese and European Portuguese.
const WORDS: &[[&str; 8]] = &[
    [
        "Estimate: ideal elements and lossless quasi-static lines; not compared with measurements or with other programs",
        "Estimación: elementos ideales y líneas sin pérdidas cuasi-estáticas; sin comparar con medidas ni con otros programas",
        "Schätzung: ideale Elemente und verlustfreie quasistatische Leitungen; nicht mit Messungen oder anderen Programmen verglichen",
        "Estimation : éléments idéaux et lignes sans perte quasi statiques ; non comparée à des mesures ni à d'autres programmes",
        "Stima: elementi ideali e linee senza perdite quasi statiche; non confrontata con misure né con altri programmi",
        "推定値:理想素子と無損失の準静的線路。測定値や他のプログラムとは比較していません",
        "估算:理想元件和无损准静态传输线;未与测量结果或其他程序比较",
        "Estimativa: elementos ideais e linhas sem perdas quase estáticas; sem comparação com medições nem com outros programas",
    ],
    ["Matching network", "Red de adaptación", "Anpassnetzwerk", "Réseau d'adaptation", "Rete di adattamento", "整合ネットワーク", "匹配网络", "Rede de adaptação"],
    ["Source", "Fuente", "Quelle", "Source", "Sorgente", "信号源", "信号源", "Fonte"],
    ["Load", "Carga", "Last", "Charge", "Carico", "負荷", "负载", "Carga"],
    ["Frequency", "Frecuencia", "Frequenz", "Fréquence", "Frequenza", "周波数", "频率", "Frequência"],
    ["Element", "Elemento", "Element", "Élément", "Elemento", "素子", "元件", "Elemento"],
    ["series", "serie", "in Reihe", "série", "serie", "直列", "串联", "série"],
    ["shunt", "paralelo", "parallel", "parallèle", "parallelo", "並列", "并联", "paralelo"],
    ["inductor", "inductor", "Spule", "inductance", "induttore", "インダクタ", "电感", "indutor"],
    ["capacitor", "condensador", "Kondensator", "condensateur", "condensatore", "コンデンサ", "电容", "condensador"],
    ["low-pass", "paso bajo", "Tiefpass", "passe-bas", "passa-basso", "ローパス", "低通", "passa-baixo"],
    ["high-pass", "paso alto", "Hochpass", "passe-haut", "passa-alto", "ハイパス", "高通", "passa-alto"],
    ["mixed", "mixto", "gemischt", "mixte", "misto", "混合", "混合", "misto"],
    ["Network Q", "Q de la red", "Güte des Netzwerks", "Q du réseau", "Q della rete", "ネットワークのQ", "网络的Q值", "Q da rede"],
    ["Loaded Q", "Q cargado", "Belastete Güte", "Q en charge", "Q caricato", "負荷Q", "负载Q值", "Q carregado"],
    ["Virtual resistance", "Resistencia virtual", "Virtueller Widerstand", "Résistance virtuelle", "Resistenza virtuale", "仮想抵抗", "虚拟电阻", "Resistência virtual"],
    ["Bandwidth, rule of thumb f0/Q", "Ancho de banda, regla f0/Q", "Bandbreite, Faustregel f0/Q", "Bande passante, règle f0/Q", "Larghezza di banda, regola f0/Q", "帯域幅(目安 f0/Q)", "带宽(经验值 f0/Q)", "Largura de banda, regra f0/Q"],
    ["Band with return loss of at least", "Banda con pérdida de retorno de al menos", "Band mit einer Rückflussdämpfung von mindestens", "Bande avec une perte de retour d'au moins", "Banda con perdita di ritorno di almeno", "リターンロスが次以上の帯域", "回波损耗不低于下值的频带", "Banda com perda de retorno de pelo menos"],
    ["not found in the scan range", "no encontrada en el rango del barrido", "im Suchbereich nicht gefunden", "introuvable dans la plage du balayage", "non trovata nell'intervallo di scansione", "走査範囲内に見つかりません", "在扫描范围内未找到", "não encontrada no intervalo da pesquisa"],
    ["Check, input impedance with the load", "Comprobación, impedancia de entrada con la carga", "Prüfung, Eingangsimpedanz mit der Last", "Vérification, impédance d'entrée avec la charge", "Verifica, impedenza d'ingresso con il carico", "確認:負荷接続時の入力インピーダンス", "校验:接负载时的输入阻抗", "Verificação, impedância de entrada com a carga"],
    ["Path on the Smith chart", "Recorrido en la carta de Smith", "Pfad im Smith-Diagramm", "Trajet sur l'abaque de Smith", "Percorso sulla carta di Smith", "スミスチャート上の経路", "史密斯圆图上的路径", "Percurso na carta de Smith"],
    ["Step", "Paso", "Schritt", "Étape", "Passo", "手順", "步骤", "Passo"],
    ["Impedance", "Impedancia", "Impedanz", "Impédance", "Impedenza", "インピーダンス", "阻抗", "Impedância"],
    ["Admittance", "Admitancia", "Admittanz", "Admittance", "Ammettenza", "アドミタンス", "导纳", "Admitância"],
    ["Reflection", "Reflexión", "Reflexion", "Réflexion", "Riflessione", "反射", "反射系数", "Reflexão"],
    ["Equivalents with transmission lines", "Equivalentes con líneas de transmisión", "Äquivalente mit Leitungen", "Équivalents avec des lignes de transmission", "Equivalenti con linee di trasmissione", "伝送線路による等価回路", "传输线等效电路", "Equivalentes com linhas de transmissão"],
    ["Line", "Línea", "Leitung", "Ligne", "Linea", "線路", "传输线", "Linha"],
    ["Characteristic impedance", "Impedancia característica", "Wellenwiderstand", "Impédance caractéristique", "Impedenza caratteristica", "特性インピーダンス", "特性阻抗", "Impedância característica"],
    ["Effective permittivity", "Permitividad efectiva", "Effektive Permittivität", "Permittivité effective", "Permittività effettiva", "実効比誘電率", "有效介电常数", "Permitividade efetiva"],
    ["Single stub", "Stub simple", "Einzelstich", "Stub simple", "Stub singolo", "シングルスタブ", "单枝节", "Stub simples"],
    ["Double stub", "Stub doble", "Doppelstich", "Double stub", "Doppio stub", "ダブルスタブ", "双枝节", "Stub duplo"],
    ["Quarter-wave transformer", "Transformador de cuarto de onda", "Viertelwellentransformator", "Transformateur quart d'onde", "Trasformatore a quarto d'onda", "1/4波長トランス", "四分之一波长变换器", "Transformador de quarto de onda"],
    ["Distance from the load", "Distancia a la carga", "Abstand zur Last", "Distance à la charge", "Distanza dal carico", "負荷からの距離", "距负载的距离", "Distância à carga"],
    ["Stub length", "Longitud del stub", "Länge des Stichs", "Longueur du stub", "Lunghezza dello stub", "スタブ長", "枝节长度", "Comprimento do stub"],
    ["Spacing between stubs", "Separación entre stubs", "Abstand der Stiche", "Écart entre les stubs", "Distanza tra gli stub", "スタブ間隔", "枝节间距", "Distância entre stubs"],
    ["Section length", "Longitud de la sección", "Länge des Abschnitts", "Longueur de la section", "Lunghezza della sezione", "セクション長", "线段长度", "Comprimento da secção"],
    ["Section impedance", "Impedancia de la sección", "Impedanz des Abschnitts", "Impédance de la section", "Impedenza della sezione", "セクションのインピーダンス", "线段阻抗", "Impedância da secção"],
    ["No solution", "Sin solución", "Keine Lösung", "Pas de solution", "Nessuna soluzione", "解なし", "无解", "Sem solução"],
    ["short-circuited", "en cortocircuito", "kurzgeschlossen", "en court-circuit", "in cortocircuito", "短絡", "短路", "em curto-circuito"],
    ["open", "en abierto", "offen", "ouvert", "aperto", "開放", "开路", "em aberto"],
    ["Filter", "Filtro", "Filter", "Filtre", "Filtro", "フィルタ", "滤波器", "Filtro"],
    ["Approximation", "Aproximación", "Approximation", "Approximation", "Approssimazione", "近似", "逼近类型", "Aproximação"],
    ["Order", "Orden", "Ordnung", "Ordre", "Ordine", "次数", "阶数", "Ordem"],
    ["Band", "Banda", "Band", "Bande", "Banda", "帯域", "频段", "Banda"],
    ["Prototype values g", "Valores g del prototipo", "Prototypwerte g", "Valeurs g du prototype", "Valori g del prototipo", "プロトタイプの値 g", "原型值 g", "Valores g do protótipo"],
    ["Components", "Componentes", "Bauteile", "Composants", "Componenti", "部品", "元件", "Componentes"],
    ["Standard series", "Serie normalizada", "Normreihe", "Série normalisée", "Serie normalizzata", "標準系列", "标准系列", "Série normalizada"],
    ["Ideal", "Ideal", "Ideal", "Idéal", "Ideale", "理想値", "理想值", "Ideal"],
    ["Standard", "Normalizado", "Normwert", "Normalisé", "Normalizzato", "標準値", "标准值", "Normalizado"],
    ["Error", "Error", "Fehler", "Erreur", "Errore", "誤差", "误差", "Erro"],
    ["Band edge", "Borde de banda", "Bandkante", "Bord de bande", "Bordo di banda", "帯域端", "频带边缘", "Limite da banda"],
    ["Shift of the band edge", "Desplazamiento del borde", "Verschiebung der Bandkante", "Décalage du bord de bande", "Spostamento del bordo", "帯域端のずれ", "频带边缘偏移", "Desvio do limite da banda"],
    ["Largest change of the insertion loss in the pass band", "Mayor cambio de la pérdida de inserción en la banda de paso", "Größte Änderung der Einfügedämpfung im Durchlassbereich", "Plus grand écart de la perte d'insertion dans la bande passante", "Massima variazione della perdita di inserzione in banda passante", "通過帯域での挿入損失の最大変化", "通带内插入损耗的最大变化", "Maior variação da perda de inserção na banda de passagem"],
    ["Response", "Respuesta", "Frequenzgang", "Réponse", "Risposta", "応答", "响应", "Resposta"],
    ["Gain", "Ganancia", "Verstärkung", "Gain", "Guadagno", "ゲイン", "增益", "Ganho"],
    ["Phase", "Fase", "Phase", "Phase", "Fase", "位相", "相位", "Fase"],
    ["Group delay", "Retardo de grupo", "Gruppenlaufzeit", "Temps de propagation de groupe", "Ritardo di gruppo", "群遅延", "群时延", "Atraso de grupo"],
    ["Simulated (AC analysis)", "Simulada (análisis AC)", "Simuliert (AC-Analyse)", "Simulée (analyse AC)", "Simulata (analisi AC)", "シミュレーション(AC解析)", "仿真(交流分析)", "Simulada (análise AC)"],
    ["ABCD matrices", "Matrices ABCD", "ABCD-Matrizen", "Matrices ABCD", "Matrici ABCD", "ABCD行列", "ABCD矩阵", "Matrizes ABCD"],
    ["S parameters of a network", "Parámetros S de una red", "S-Parameter eines Netzwerks", "Paramètres S d'un réseau", "Parametri S di una rete", "ネットワークのSパラメータ", "网络的S参数", "Parâmetros S de uma rede"],
    ["Reference impedance", "Impedancia de referencia", "Bezugsimpedanz", "Impédance de référence", "Impedenza di riferimento", "基準インピーダンス", "参考阻抗", "Impedância de referência"],
    ["Return loss", "Pérdida de retorno", "Rückflussdämpfung", "Perte de retour", "Perdita di ritorno", "リターンロス", "回波损耗", "Perda de retorno"],
    ["Insertion loss", "Pérdida de inserción", "Einfügedämpfung", "Perte d'insertion", "Perdita di inserzione", "挿入損失", "插入损耗", "Perda de inserção"],
    ["Mask", "Máscara", "Maske", "Gabarit", "Maschera", "マスク", "模板", "Máscara"],
    ["Segment", "Segmento", "Segment", "Segment", "Segmento", "区間", "分段", "Segmento"],
    ["Points", "Puntos", "Punkte", "Points", "Punti", "点数", "点数", "Pontos"],
    ["Worst margin", "Peor margen", "Schlechtester Abstand", "Pire marge", "Margine peggiore", "最悪マージン", "最差裕量", "Pior margem"],
    ["At", "En", "Bei", "À", "A", "周波数", "位于", "Em"],
    ["Result", "Resultado", "Ergebnis", "Résultat", "Risultato", "結果", "结果", "Resultado"],
    ["pass", "cumple", "bestanden", "réussi", "superato", "合格", "通过", "cumpre"],
    ["fail", "no cumple", "nicht bestanden", "échec", "non superato", "不合格", "未通过", "não cumpre"],
    ["no data", "sin datos", "keine Daten", "aucune donnée", "nessun dato", "データなし", "无数据", "sem dados"],
    ["Overall", "Global", "Gesamt", "Global", "Complessivo", "総合", "总体", "Global"],
    ["Amplifier stability", "Estabilidad del amplificador", "Stabilität des Verstärkers", "Stabilité de l'amplificateur", "Stabilità dell'amplificatore", "アンプの安定性", "放大器稳定性", "Estabilidade do amplificador"],
    ["Determinant", "Determinante", "Determinante", "Déterminant", "Determinante", "行列式", "行列式", "Determinante"],
    ["Unconditionally stable", "Incondicionalmente estable", "Bedingungslos stabil", "Inconditionnellement stable", "Incondizionatamente stabile", "無条件安定", "绝对稳定", "Incondicionalmente estável"],
    ["yes", "sí", "ja", "oui", "sì", "はい", "是", "sim"],
    ["no", "no", "nein", "non", "no", "いいえ", "否", "não"],
    ["Source stability circle", "Círculo de estabilidad de la fuente", "Stabilitätskreis der Quelle", "Cercle de stabilité de la source", "Cerchio di stabilità della sorgente", "ソース安定円", "源稳定圆", "Círculo de estabilidade da fonte"],
    ["Load stability circle", "Círculo de estabilidad de la carga", "Stabilitätskreis der Last", "Cercle de stabilité de la charge", "Cerchio di stabilità del carico", "負荷安定円", "负载稳定圆", "Círculo de estabilidade da carga"],
    ["centre", "centro", "Mittelpunkt", "centre", "centro", "中心", "圆心", "centro"],
    ["radius", "radio", "Radius", "rayon", "raggio", "半径", "半径", "raio"],
    ["stable side", "lado estable", "stabile Seite", "côté stable", "lato stabile", "安定側", "稳定侧", "lado estável"],
    ["inside", "interior", "innen", "intérieur", "interno", "内側", "圆内", "interior"],
    ["outside", "exterior", "außen", "extérieur", "esterno", "外側", "圆外", "exterior"],
    ["cuts the Smith chart", "corta la carta de Smith", "schneidet das Smith-Diagramm", "coupe l'abaque de Smith", "taglia la carta di Smith", "スミスチャートと交差", "与史密斯圆图相交", "corta a carta de Smith"],
    ["Maximum stable gain", "Ganancia estable máxima", "Maximale stabile Verstärkung", "Gain stable maximal", "Guadagno stabile massimo", "最大安定ゲイン", "最大稳定增益", "Ganho estável máximo"],
    ["Maximum available gain", "Ganancia disponible máxima", "Maximal verfügbare Verstärkung", "Gain disponible maximal", "Guadagno disponibile massimo", "最大有能ゲイン", "最大可用增益", "Ganho disponível máximo"],
    ["Simultaneous conjugate match", "Adaptación conjugada simultánea", "Gleichzeitige konjugierte Anpassung", "Adaptation conjuguée simultanée", "Adattamento coniugato simultaneo", "同時共役整合", "同时共轭匹配", "Adaptação conjugada simultânea"],
    ["none (not unconditionally stable)", "no existe (no es incondicionalmente estable)", "keine (nicht bedingungslos stabil)", "aucune (pas inconditionnellement stable)", "nessuna (non incondizionatamente stabile)", "なし(無条件安定ではありません)", "无(不是绝对稳定)", "não existe (não é incondicionalmente estável)"],
    ["S parameters", "Parámetros S", "S-Parameter", "Paramètres S", "Parametri S", "Sパラメータ", "S参数", "Parâmetros S"],
    ["Source reflection coefficient", "Coeficiente de reflexión de la fuente", "Reflexionsfaktor der Quelle", "Coefficient de réflexion de la source", "Coefficiente di riflessione della sorgente", "ソースの反射係数", "源反射系数", "Coeficiente de reflexão da fonte"],
    ["Load reflection coefficient", "Coeficiente de reflexión de la carga", "Reflexionsfaktor der Last", "Coefficient de réflexion de la charge", "Coefficiente di riflessione del carico", "負荷の反射係数", "负载反射系数", "Coeficiente de reflexão da carga"],
    ["Gain with the match", "Ganancia con la adaptación", "Verstärkung mit der Anpassung", "Gain avec l'adaptation", "Guadagno con l'adattamento", "整合時のゲイン", "匹配时的增益", "Ganho com a adaptação"],
    ["no circles (unilateral or degenerate)", "sin círculos (unilateral o degenerado)", "keine Kreise (unilateral oder entartet)", "pas de cercles (unilatéral ou dégénéré)", "nessun cerchio (unilaterale o degenere)", "円なし(単方向または退化)", "无圆(单向或退化)", "sem círculos (unilateral ou degenerado)"],
];

fn column(language: Language) -> usize {
    Language::ALL
        .iter()
        .position(|l| *l == language)
        .unwrap_or(0)
}

/// A word in a language.
pub(crate) fn word(w: L, language: Language) -> &'static str {
    WORDS
        .get(w as usize)
        .and_then(|row| row.get(column(language)))
        .copied()
        .unwrap_or("")
}

/// All the rows of the table, for the tests.
#[cfg(test)]
pub(crate) fn rows() -> &'static [[&'static str; 8]] {
    WORDS
}

fn complex_text(z: Complex) -> String {
    format!(
        "{:.4} {} j{:.4}",
        z.re,
        if z.im < 0.0 { "-" } else { "+" },
        z.im.abs()
    )
}

fn complex_json(z: Complex) -> Value {
    json!({ "re": z.re, "im": z.im })
}

fn value_or_null(x: f64) -> Value {
    if x.is_finite() {
        json!(x)
    } else {
        Value::Null
    }
}

fn kind_word(kind: ElementKind, language: Language) -> &'static str {
    match kind {
        ElementKind::Inductor => word(L::Inductor, language),
        ElementKind::Capacitor => word(L::Capacitor, language),
    }
}

fn position_word(position: eh3d_phoenix_rf::filter::Position, language: Language) -> &'static str {
    match position {
        eh3d_phoenix_rf::filter::Position::Series => word(L::Series, language),
        eh3d_phoenix_rf::filter::Position::Shunt => word(L::Shunt, language),
    }
}

fn position_key(position: eh3d_phoenix_rf::filter::Position) -> &'static str {
    match position {
        eh3d_phoenix_rf::filter::Position::Series => "series",
        eh3d_phoenix_rf::filter::Position::Shunt => "shunt",
    }
}

fn part_text(part: &Part, language: Language) -> String {
    let unit = match part.element.kind {
        ElementKind::Inductor => "H",
        ElementKind::Capacitor => "F",
    };
    format!(
        "{} {}: {}",
        position_word(part.position, language),
        kind_word(part.element.kind, language),
        engineering(part.element.value, unit)
    )
}

fn part_json(part: &Part) -> Value {
    json!({
        "position": position_key(part.position),
        "kind": match part.element.kind {
            ElementKind::Inductor => "inductor",
            ElementKind::Capacitor => "capacitor",
        },
        "value": part.element.value,
    })
}

// ---------------------------------------------------------------------------------------------
// matching networks

/// The pass type of a network.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PassWord {
    Low,
    High,
    Mixed,
}

impl PassWord {
    fn word(self, language: Language) -> &'static str {
        match self {
            PassWord::Low => word(L::LowPass, language),
            PassWord::High => word(L::HighPass, language),
            PassWord::Mixed => word(L::Mixed, language),
        }
    }

    fn key(self) -> &'static str {
        match self {
            PassWord::Low => "low-pass",
            PassWord::High => "high-pass",
            PassWord::Mixed => "mixed",
        }
    }
}

/// One matching network with everything the report says about it.
pub(crate) struct NetworkResult {
    pub shape: &'static str,
    pub pass: PassWord,
    pub parts: Vec<Part>,
    pub q_network: f64,
    pub virtual_ohm: Option<f64>,
    pub bandwidth: Bandwidth,
    pub input_impedance: Complex,
    pub trace: SmithTrace,
}

/// The result of a matching command.
pub(crate) struct MatchReport {
    pub source: Complex,
    pub load: Complex,
    pub freq_hz: f64,
    pub return_loss_limit_db: f64,
    pub networks: Vec<NetworkResult>,
}

fn bandwidth_text(b: &Bandwidth, limit_db: f64, language: Language) -> Vec<String> {
    let edge = |x: Option<f64>| match x {
        Some(f) => engineering(f, "Hz"),
        None => word(L::NotFound, language).to_string(),
    };
    vec![
        format!(
            "{}: {}",
            word(L::BwRule, language),
            engineering(b.rule_of_thumb_hz, "Hz")
        ),
        format!(
            "{} {} dB: {} .. {}",
            word(L::BwReturnLoss, language),
            limit_db,
            edge(b.low_hz),
            edge(b.high_hz)
        ),
    ]
}

fn trace_text(trace: &SmithTrace, language: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "  {} (Z0 = {} ohm)\n",
        word(L::SmithPath, language),
        trace.chart_z0_ohm
    ));
    out.push_str(&format!(
        "    {:<4} {:<34} {:<24} {:<24} {}\n",
        word(L::Step, language),
        "",
        word(L::Impedance, language),
        word(L::Admittance, language),
        word(L::Reflection, language)
    ));
    for (index, point) in trace.points.iter().enumerate() {
        let admittance = format!(
            "{:.6e} {} j{:.6e}",
            point.y.re,
            if point.y.im < 0.0 { "-" } else { "+" },
            point.y.im.abs()
        );
        out.push_str(&format!(
            "    {:<4} {:<34} {:<24} {:<24} {}\n",
            index,
            point.label,
            complex_text(point.z),
            admittance,
            complex_text(point.gamma)
        ));
    }
    out
}

fn trace_json(trace: &SmithTrace) -> Value {
    json!({
        "chart_z0_ohm": trace.chart_z0_ohm,
        "points": trace.points.iter().map(|p| json!({
            "label": p.label,
            "z": complex_json(p.z),
            "y": complex_json(p.y),
            "gamma": complex_json(p.gamma),
        })).collect::<Vec<Value>>(),
        "paths": trace.paths.iter().map(|path| {
            path.iter().map(|g| complex_json(*g)).collect::<Vec<Value>>()
        }).collect::<Vec<Vec<Value>>>(),
    })
}

/// The text of a matching report.
pub(crate) fn match_text(report: &MatchReport, language: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(L::MatchTitle, language)));
    out.push_str(&format!("{}\n", word(L::Estimate, language)));
    out.push_str(&format!(
        "{}: {} ohm\n{}: {} ohm\n{}: {}\n",
        word(L::Source, language),
        complex_text(report.source),
        word(L::Load, language),
        complex_text(report.load),
        word(L::Frequency, language),
        engineering(report.freq_hz, "Hz")
    ));
    for (index, net) in report.networks.iter().enumerate() {
        out.push_str(&format!(
            "\n[{}] {} {}\n",
            index + 1,
            net.shape,
            net.pass.word(language)
        ));
        for (number, part) in net.parts.iter().enumerate() {
            out.push_str(&format!(
                "  {} {}: {}\n",
                word(L::Element, language),
                number + 1,
                part_text(part, language)
            ));
        }
        let q_word = if net.shape == "L" {
            word(L::NetworkQ, language)
        } else {
            word(L::LoadedQ, language)
        };
        out.push_str(&format!("  {}: {:.4}\n", q_word, net.q_network));
        if let Some(r) = net.virtual_ohm {
            out.push_str(&format!(
                "  {}: {:.4} ohm\n",
                word(L::VirtualR, language),
                r
            ));
        }
        for line in bandwidth_text(&net.bandwidth, report.return_loss_limit_db, language) {
            out.push_str(&format!("  {line}\n"));
        }
        out.push_str(&format!(
            "  {}: {} ohm\n",
            word(L::Check, language),
            complex_text(net.input_impedance)
        ));
        out.push_str(&trace_text(&net.trace, language));
    }
    out
}

/// The JSON of a matching report.
pub(crate) fn match_json(report: &MatchReport) -> Value {
    json!({
        "format": "phoenix-rf-match/1",
        "estimate": true,
        "source": complex_json(report.source),
        "load": complex_json(report.load),
        "freq_hz": report.freq_hz,
        "return_loss_limit_db": report.return_loss_limit_db,
        "networks": report.networks.iter().map(|net| json!({
            "shape": net.shape,
            "pass": net.pass.key(),
            "parts": net.parts.iter().map(part_json).collect::<Vec<Value>>(),
            "q_network": net.q_network,
            "virtual_ohm": net.virtual_ohm.map_or(Value::Null, value_or_null),
            "bandwidth": {
                "rule_of_thumb_hz": value_or_null(net.bandwidth.rule_of_thumb_hz),
                "low_hz": net.bandwidth.low_hz.map_or(Value::Null, value_or_null),
                "high_hz": net.bandwidth.high_hz.map_or(Value::Null, value_or_null),
            },
            "input_impedance": complex_json(net.input_impedance),
            "smith": trace_json(&net.trace),
        })).collect::<Vec<Value>>(),
    })
}

// ---------------------------------------------------------------------------------------------
// equivalents with lines

/// The result of `phoenix rf match lines`.
pub(crate) struct LinesReport {
    pub load: Complex,
    pub equivalents: LineEquivalents,
    pub line_name: Option<String>,
}

fn termination_word(t: eh3d_phoenix_rf::matching::Termination, language: Language) -> &'static str {
    match t {
        eh3d_phoenix_rf::matching::Termination::Short => word(L::Short, language),
        eh3d_phoenix_rf::matching::Termination::Open => word(L::Open, language),
    }
}

fn termination_key(t: eh3d_phoenix_rf::matching::Termination) -> &'static str {
    match t {
        eh3d_phoenix_rf::matching::Termination::Short => "short",
        eh3d_phoenix_rf::matching::Termination::Open => "open",
    }
}

fn length_text(deg: f64, metres: f64) -> String {
    format!("{:.3} deg, {}", deg, engineering(metres, "m"))
}

/// The text of the line equivalents.
pub(crate) fn lines_text(report: &LinesReport, language: Language) -> String {
    let eq = &report.equivalents;
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(L::LinesTitle, language)));
    out.push_str(&format!("{}\n", word(L::Estimate, language)));
    out.push_str(&format!(
        "{}: {} ohm\n{}: {}\n",
        word(L::Load, language),
        complex_text(report.load),
        word(L::Frequency, language),
        engineering(eq.freq_hz, "Hz")
    ));
    out.push_str(&format!(
        "{}: {}\n  {}: {} ohm\n  {}: {:.4}\n",
        word(L::LineLine, language),
        report.line_name.as_deref().unwrap_or("-"),
        word(L::Z0, language),
        eq.line.z0_ohm,
        word(L::EpsEff, language),
        eq.line.eps_eff
    ));
    for (index, item) in eq.single.iter().enumerate() {
        out.push_str(&format!(
            "\n{} [{}], {}\n  {}: {}\n  {}: {}\n",
            word(L::SingleStub, language),
            index + 1,
            termination_word(item.stub.termination, language),
            word(L::Distance, language),
            length_text(item.stub.distance_deg, item.distance_m),
            word(L::StubLength, language),
            length_text(item.stub.stub_deg, item.stub_m)
        ));
    }
    for (index, item) in eq.double.iter().enumerate() {
        out.push_str(&format!(
            "\n{} [{}], {}\n  {}: {}\n  {} 1: {}\n  {} 2: {}\n",
            word(L::DoubleStub, language),
            index + 1,
            termination_word(item.stubs.termination, language),
            word(L::Spacing, language),
            length_text(item.stubs.spacing_deg, item.spacing_m),
            word(L::StubLength, language),
            length_text(item.stubs.first_deg, item.first_m),
            word(L::StubLength, language),
            length_text(item.stubs.second_deg, item.second_m)
        ));
    }
    for (index, item) in eq.quarter_wave.iter().enumerate() {
        out.push_str(&format!(
            "\n{} [{}]\n  {}: {}\n  {}: {}\n  {}: {:.4} ohm\n",
            word(L::QuarterWave, language),
            index + 1,
            word(L::Distance, language),
            length_text(item.transformer.distance_deg, item.distance_m),
            word(L::SectionLength, language),
            length_text(90.0, item.section_m),
            word(L::SectionZ, language),
            item.transformer.section_ohm
        ));
    }
    for (kind, reason) in &eq.unavailable {
        out.push_str(&format!(
            "\n{}: {}: {}\n",
            word(L::Unavailable, language),
            kind,
            reason
        ));
    }
    out
}

/// The JSON of the line equivalents.
pub(crate) fn lines_json(report: &LinesReport) -> Value {
    let eq = &report.equivalents;
    json!({
        "format": "phoenix-rf-lines/1",
        "estimate": true,
        "load": complex_json(report.load),
        "freq_hz": eq.freq_hz,
        "line": {
            "name": report.line_name,
            "z0_ohm": eq.line.z0_ohm,
            "eps_eff": eq.line.eps_eff,
        },
        "single_stub": eq.single.iter().map(|i| json!({
            "termination": termination_key(i.stub.termination),
            "distance_deg": i.stub.distance_deg,
            "distance_m": i.distance_m,
            "stub_deg": i.stub.stub_deg,
            "stub_m": i.stub_m,
        })).collect::<Vec<Value>>(),
        "double_stub": eq.double.iter().map(|i| json!({
            "termination": termination_key(i.stubs.termination),
            "spacing_deg": i.stubs.spacing_deg,
            "spacing_m": i.spacing_m,
            "first_deg": i.stubs.first_deg,
            "first_m": i.first_m,
            "second_deg": i.stubs.second_deg,
            "second_m": i.second_m,
        })).collect::<Vec<Value>>(),
        "quarter_wave": eq.quarter_wave.iter().map(|i| json!({
            "distance_deg": i.transformer.distance_deg,
            "distance_m": i.distance_m,
            "section_ohm": i.transformer.section_ohm,
            "section_m": i.section_m,
        })).collect::<Vec<Value>>(),
        "unavailable": eq.unavailable.iter().map(|(k, r)| json!({"kind": k, "reason": r})).collect::<Vec<Value>>(),
    })
}

// ---------------------------------------------------------------------------------------------
// filters

/// One row of the response of a filter.
pub(crate) struct FilterRow {
    pub freq_hz: f64,
    pub gain_db: f64,
    pub phase_deg: f64,
    pub delay_s: f64,
    pub simulated: Option<ResponsePoint>,
    pub snapped_gain_db: Option<f64>,
}

/// The standard-value part of a filter report.
pub(crate) struct SnapResult {
    pub series: Series,
    pub values: Vec<SnappedValue>,
    pub effect: SnapEffect,
}

/// The result of `phoenix rf filter` with the extended options.
pub(crate) struct FilterReport {
    pub approximation: &'static str,
    pub order: usize,
    pub band: String,
    pub ripple_db: Option<f64>,
    pub z0_ohm: f64,
    pub load_ohm: f64,
    pub g: Vec<f64>,
    pub g_load: f64,
    pub components: Vec<(String, f64)>,
    pub snap: Option<SnapResult>,
    pub rows: Vec<FilterRow>,
    pub spice: Option<String>,
}

fn unit_of(name: &str) -> &'static str {
    if name.starts_with('L') {
        "H"
    } else {
        "F"
    }
}

/// The text of a filter report.
pub(crate) fn filter_text(report: &FilterReport, language: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(L::FilterTitle, language)));
    out.push_str(&format!("{}\n", word(L::Estimate, language)));
    out.push_str(&format!(
        "{}: {}\n{}: {}\n{}: {}\n",
        word(L::Approximation, language),
        report.approximation,
        word(L::Order, language),
        report.order,
        word(L::Band, language),
        report.band
    ));
    if let Some(ripple) = report.ripple_db {
        out.push_str(&format!("ripple: {ripple} dB\n"));
    }
    out.push_str(&format!(
        "{}: {} ohm, {} ohm\n",
        word(L::Source, language),
        report.z0_ohm,
        report.load_ohm
    ));
    let g: Vec<String> = report.g.iter().map(|v| format!("{v:.6}")).collect();
    out.push_str(&format!(
        "{}: {} | {:.6}\n",
        word(L::PrototypeG, language),
        g.join(" "),
        report.g_load
    ));
    out.push_str(&format!("\n{}\n", word(L::Components, language)));
    match &report.snap {
        None => {
            for (name, value) in &report.components {
                out.push_str(&format!(
                    "  {name}: {}\n",
                    engineering(*value, unit_of(name))
                ));
            }
        }
        Some(snap) => {
            out.push_str(&format!(
                "  {}: {}\n",
                word(L::StdSeries, language),
                snap.series.name()
            ));
            out.push_str(&format!(
                "  {:<4} {:<14} {:<14} {}\n",
                "",
                word(L::Ideal, language),
                word(L::Standard, language),
                word(L::Error, language)
            ));
            for v in &snap.values {
                out.push_str(&format!(
                    "  {:<4} {:<14} {:<14} {:+.3} %\n",
                    v.name,
                    engineering(v.ideal, unit_of(&v.name)),
                    engineering(v.snapped, unit_of(&v.name)),
                    v.error_percent
                ));
            }
            for (k, (ideal, snapped)) in snap
                .effect
                .ideal_edges_hz
                .iter()
                .zip(&snap.effect.snapped_edges_hz)
                .enumerate()
            {
                out.push_str(&format!(
                    "  {} {}: {} -> {} ({}: {:+.4} %)\n",
                    word(L::Edge, language),
                    k + 1,
                    engineering(*ideal, "Hz"),
                    engineering(*snapped, "Hz"),
                    word(L::EdgeShift, language),
                    snap.effect
                        .edge_errors_percent
                        .get(k)
                        .copied()
                        .unwrap_or(0.0)
                ));
            }
            out.push_str(&format!(
                "  {}: {:.4} dB\n",
                word(L::PassChange, language),
                snap.effect.max_passband_change_db
            ));
        }
    }
    if !report.rows.is_empty() {
        out.push_str(&format!(
            "\n{} ({})\n",
            word(L::Response, language),
            word(L::Analytic, language)
        ));
        let simulated = report.rows.iter().any(|r| r.simulated.is_some());
        let snapped = report.rows.iter().any(|r| r.snapped_gain_db.is_some());
        let gain_header = format!("{} dB", word(L::Gain, language));
        let phase_header = format!("{} deg", word(L::Phase, language));
        let mut header = format!(
            "  {:<14} {:<12} {:<12} {:<14}",
            word(L::Frequency, language),
            gain_header,
            phase_header,
            word(L::GroupDelay, language)
        );
        if simulated {
            header.push_str(&format!(
                " | {} {:<10} {:<10} {}",
                word(L::Simulated, language),
                "dB",
                "deg",
                "s"
            ));
        }
        if snapped {
            header.push_str(&format!(" | {} dB", word(L::Standard, language)));
        }
        out.push_str(&header);
        out.push('\n');
        for row in &report.rows {
            let mut line = format!(
                "  {:<14} {:<12.4} {:<12.3} {:<14.6e}",
                engineering(row.freq_hz, "Hz"),
                row.gain_db,
                row.phase_deg,
                row.delay_s
            );
            if let Some(sim) = &row.simulated {
                line.push_str(&format!(
                    " | {:<10.4} {:<10.3} {:.6e}",
                    sim.gain_db, sim.phase_deg, sim.delay_s
                ));
            }
            if let Some(g) = row.snapped_gain_db {
                line.push_str(&format!(" | {g:.4}"));
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    if let Some(spice) = &report.spice {
        out.push_str(&format!("\n{spice}"));
    }
    out
}

/// The JSON of a filter report.
pub(crate) fn filter_json(report: &FilterReport) -> Value {
    json!({
        "format": "phoenix-rf-filter/1",
        "estimate": true,
        "approximation": report.approximation,
        "order": report.order,
        "band": report.band,
        "ripple_db": report.ripple_db,
        "z0_ohm": report.z0_ohm,
        "load_ohm": report.load_ohm,
        "g": report.g,
        "g_load": report.g_load,
        "components": report.components.iter().map(|(n, v)| json!({"name": n, "value": v})).collect::<Vec<Value>>(),
        "standard_values": report.snap.as_ref().map(|s| json!({
            "series": s.series.name(),
            "values": s.values.iter().map(|v| json!({
                "name": v.name,
                "ideal": v.ideal,
                "snapped": v.snapped,
                "error_percent": v.error_percent,
            })).collect::<Vec<Value>>(),
            "reference_loss_db": s.effect.reference_loss_db,
            "ideal_edges_hz": s.effect.ideal_edges_hz,
            "snapped_edges_hz": s.effect.snapped_edges_hz,
            "edge_errors_percent": s.effect.edge_errors_percent,
            "max_passband_change_db": s.effect.max_passband_change_db,
        })),
        "response": report.rows.iter().map(|r| json!({
            "freq_hz": r.freq_hz,
            "gain_db": value_or_null(r.gain_db),
            "phase_deg": value_or_null(r.phase_deg),
            "group_delay_s": value_or_null(r.delay_s),
            "simulated": r.simulated.map(|s| json!({
                "gain_db": value_or_null(s.gain_db),
                "phase_deg": value_or_null(s.phase_deg),
                "group_delay_s": value_or_null(s.delay_s),
            })),
            "snapped_gain_db": r.snapped_gain_db.map(value_or_null),
        })).collect::<Vec<Value>>(),
        "spice": report.spice,
    })
}

// ---------------------------------------------------------------------------------------------
// S parameters and masks

/// One row of an S parameter table.
pub(crate) struct SRow {
    pub freq_hz: f64,
    pub s: Mat2,
    pub figures: Figures,
}

/// The result of `phoenix rf sparams`.
pub(crate) struct SParamsReport {
    pub source_name: String,
    pub z0_ohm: f64,
    pub rows: Vec<SRow>,
    pub mask: Option<MaskReport>,
}

fn db_or_inf(db: f64) -> String {
    if db.is_finite() {
        format!("{db:.3}")
    } else if db > 0.0 {
        "inf".to_string()
    } else {
        "-inf".to_string()
    }
}

/// The text of the S parameter report.
pub(crate) fn sparams_text(report: &SParamsReport, language: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(L::SparamTitle, language)));
    out.push_str(&format!("{}\n", word(L::Estimate, language)));
    out.push_str(&format!(
        "{}: {}\n{}: {} ohm\n\n",
        word(L::Parameters, language),
        report.source_name,
        word(L::RefImpedance, language),
        report.z0_ohm
    ));
    out.push_str(&format!(
        "{:<14} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}\n",
        word(L::Frequency, language),
        "|S11| dB",
        "S11 deg",
        "|S21| dB",
        "S21 deg",
        "RL dB",
        "VSWR",
        "IL dB"
    ));
    for row in &report.rows {
        let f = &row.figures;
        out.push_str(&format!(
            "{:<14} {:>9} {:>9.3} {:>9} {:>9.3} {:>9} {:>9} {:>9}\n",
            engineering(row.freq_hz, "Hz"),
            db_or_inf(f.s11_db),
            f.s11_deg,
            db_or_inf(f.s21_db),
            f.s21_deg,
            db_or_inf(f.return_loss_db),
            if f.vswr.is_finite() {
                format!("{:.4}", f.vswr)
            } else {
                "inf".to_string()
            },
            db_or_inf(f.insertion_loss_db)
        ));
    }
    out.push_str(&format!(
        "({}: {}, {}: {})\n",
        "RL",
        word(L::ReturnLoss, language),
        "IL",
        word(L::InsertionLoss, language)
    ));
    if let Some(mask) = &report.mask {
        out.push_str(&format!("\n{}\n", word(L::MaskTitle, language)));
        out.push_str(&format!(
            "  {:<4} {:<6} {:<6} {:<24} {:<7} {:<12} {:<14} {}\n",
            word(L::Segment, language),
            "",
            "",
            "",
            word(L::Points, language),
            word(L::WorstMargin, language),
            word(L::AtFrequency, language),
            word(L::Result, language)
        ));
        for r in &mask.results {
            let verdict = if r.points == 0 {
                word(L::NoData, language)
            } else if r.passed {
                word(L::Pass, language)
            } else {
                word(L::Fail, language)
            };
            let range = format!(
                "{} .. {} ({:.1} .. {:.1} dB)",
                engineering(r.segment.start_hz, "Hz"),
                engineering(r.segment.stop_hz, "Hz"),
                r.segment.start_db,
                r.segment.stop_db
            );
            out.push_str(&format!(
                "  {:<4} {:<6} {:<6} {:<24} {:<7} {:<12} {:<14} {}\n",
                r.segment.line,
                r.segment.param.name(),
                r.segment.limit.name(),
                range,
                r.points,
                db_or_inf(r.worst_margin_db),
                engineering(r.worst_freq_hz, "Hz"),
                verdict
            ));
        }
        out.push_str(&format!(
            "  {}: {}\n",
            word(L::Overall, language),
            if mask.passed {
                word(L::Pass, language)
            } else {
                word(L::Fail, language)
            }
        ));
    }
    out
}

/// The JSON of the S parameter report.
pub(crate) fn sparams_json(report: &SParamsReport) -> Value {
    json!({
        "format": "phoenix-rf-sparams/1",
        "estimate": true,
        "source": report.source_name,
        "z0_ohm": report.z0_ohm,
        "points": report.rows.iter().map(|r| json!({
            "freq_hz": r.freq_hz,
            "s11": complex_json(r.s.m11),
            "s12": complex_json(r.s.m12),
            "s21": complex_json(r.s.m21),
            "s22": complex_json(r.s.m22),
            "s11_db": value_or_null(r.figures.s11_db),
            "s21_db": value_or_null(r.figures.s21_db),
            "return_loss_db": value_or_null(r.figures.return_loss_db),
            "vswr": value_or_null(r.figures.vswr),
            "insertion_loss_db": value_or_null(r.figures.insertion_loss_db),
        })).collect::<Vec<Value>>(),
        "mask": report.mask.as_ref().map(|m| json!({
            "passed": m.passed,
            "segments": m.results.iter().map(|r| json!({
                "line": r.segment.line,
                "parameter": r.segment.param.name(),
                "limit": r.segment.limit.name(),
                "start_hz": r.segment.start_hz,
                "stop_hz": r.segment.stop_hz,
                "start_db": r.segment.start_db,
                "stop_db": r.segment.stop_db,
                "points": r.points,
                "worst_margin_db": value_or_null(r.worst_margin_db),
                "worst_freq_hz": r.worst_freq_hz,
                "passed": r.passed,
            })).collect::<Vec<Value>>(),
        })),
    })
}

// ---------------------------------------------------------------------------------------------
// stability

/// One row of the stability table.
pub(crate) struct StabRow {
    pub freq_hz: f64,
    pub figures: Stability,
}

/// The result of `phoenix rf stability`.
pub(crate) struct StabilityOut {
    pub source_name: String,
    pub rows: Vec<StabRow>,
    pub detail: Option<(f64, Mat2, StabilityReport)>,
    pub max_stable_gain: Option<f64>,
    pub max_available_gain: Option<f64>,
}

fn yes_no(flag: bool, language: Language) -> &'static str {
    if flag {
        word(L::Yes, language)
    } else {
        word(L::No, language)
    }
}

fn ratio_text(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.4}")
    } else {
        "inf".to_string()
    }
}

fn gain_text(linear: f64) -> String {
    if linear.is_finite() && linear > 0.0 {
        format!("{linear:.4} ({:.3} dB)", 10.0 * linear.log10())
    } else {
        "inf".to_string()
    }
}

/// The text of the stability report.
pub(crate) fn stability_text(report: &StabilityOut, language: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(L::StabTitle, language)));
    out.push_str(&format!("{}\n", word(L::Estimate, language)));
    out.push_str(&format!(
        "{}: {}\n\n",
        word(L::Parameters, language),
        report.source_name
    ));
    out.push_str(&format!(
        "{:<14} {:>10} {:>10} {:>10} {:>10}  {}\n",
        word(L::Frequency, language),
        "K",
        "|Delta|",
        "mu",
        "mu'",
        word(L::UncondStable, language)
    ));
    for row in &report.rows {
        let f = &row.figures;
        out.push_str(&format!(
            "{:<14} {:>10} {:>10.4} {:>10} {:>10}  {}\n",
            engineering(row.freq_hz, "Hz"),
            ratio_text(f.k),
            f.delta.abs(),
            ratio_text(f.mu),
            ratio_text(f.mu_prime),
            yes_no(f.unconditionally_stable, language)
        ));
    }
    if let Some((freq, _s, detail)) = &report.detail {
        out.push_str(&format!("\n{}\n", engineering(*freq, "Hz")));
        out.push_str(&format!(
            "  {}: {}\n",
            word(L::Determinant, language),
            complex_text(detail.figures.delta)
        ));
        match &detail.circles {
            None => out.push_str(&format!("  {}\n", word(L::NoCircles, language))),
            Some(circles) => {
                for (label, circle) in [
                    (word(L::SourceCircle, language), &circles.source),
                    (word(L::LoadCircle, language), &circles.load),
                ] {
                    out.push_str(&format!(
                        "  {label}: {} {}, {} {:.4}, {} {}, {} {}\n",
                        word(L::Centre, language),
                        complex_text(circle.center),
                        word(L::Radius, language),
                        circle.radius,
                        word(L::StableSide, language),
                        match circle.stable {
                            StableSide::Inside => word(L::Inside, language),
                            StableSide::Outside => word(L::Outside, language),
                        },
                        word(L::CutsChart, language),
                        yes_no(circle.cuts_chart, language)
                    ));
                }
            }
        }
        if let Some(msg) = report.max_stable_gain {
            out.push_str(&format!(
                "  {}: {}\n",
                word(L::Msg, language),
                gain_text(msg)
            ));
        }
        out.push_str(&format!("  {}:", word(L::ConjMatch, language)));
        match &detail.conjugate {
            None => out.push_str(&format!(" {}\n", word(L::NoConj, language))),
            Some(m) => {
                out.push('\n');
                out.push_str(&format!(
                    "    {}: {:.4} {:.2} deg\n    {}: {:.4} {:.2} deg\n    {}: {}\n",
                    word(L::GammaSource, language),
                    m.gamma_source.abs(),
                    m.gamma_source.arg_deg(),
                    word(L::GammaLoad, language),
                    m.gamma_load.abs(),
                    m.gamma_load.arg_deg(),
                    word(L::GainWithMatch, language),
                    gain_text(m.gain)
                ));
                if let Some(mag) = report.max_available_gain {
                    out.push_str(&format!(
                        "    {}: {}\n",
                        word(L::Mag, language),
                        gain_text(mag)
                    ));
                }
            }
        }
    }
    out
}

/// The JSON of the stability report.
pub(crate) fn stability_json(report: &StabilityOut) -> Value {
    let circle_json = |c: &eh3d_phoenix_rf::circles::StabilityCircle| {
        json!({
            "center": complex_json(c.center),
            "radius": c.radius,
            "stable_side": match c.stable {
                StableSide::Inside => "inside",
                StableSide::Outside => "outside",
            },
            "cuts_chart": c.cuts_chart,
        })
    };
    json!({
        "format": "phoenix-rf-stability/1",
        "estimate": true,
        "source": report.source_name,
        "points": report.rows.iter().map(|r| json!({
            "freq_hz": r.freq_hz,
            "k": value_or_null(r.figures.k),
            "delta": complex_json(r.figures.delta),
            "mu": value_or_null(r.figures.mu),
            "mu_prime": value_or_null(r.figures.mu_prime),
            "unconditionally_stable": r.figures.unconditionally_stable,
        })).collect::<Vec<Value>>(),
        "detail": report.detail.as_ref().map(|(freq, s, d)| json!({
            "freq_hz": freq,
            "s11": complex_json(s.m11),
            "s12": complex_json(s.m12),
            "s21": complex_json(s.m21),
            "s22": complex_json(s.m22),
            "source_circle": d.circles.as_ref().map(|c| circle_json(&c.source)),
            "load_circle": d.circles.as_ref().map(|c| circle_json(&c.load)),
            "max_stable_gain": report.max_stable_gain.map(value_or_null),
            "max_available_gain": report.max_available_gain.map(value_or_null),
            "conjugate_match": d.conjugate.as_ref().map(|m| json!({
                "gamma_source": complex_json(m.gamma_source),
                "gamma_load": complex_json(m.gamma_load),
                "gain": m.gain,
            })),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_one_row_per_word_and_every_language_is_filled() {
        assert_eq!(rows().len(), WORD_COUNT);
        for (index, row) in rows().iter().enumerate() {
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "word {index}, language {column}");
            }
        }
    }

    #[test]
    fn japanese_and_chinese_are_not_english() {
        // the symbols and a few international words may repeat, the text of the others may not
        let mut same = 0;
        for row in rows() {
            if row[5] == row[0] || row[6] == row[0] {
                same += 1;
            }
        }
        assert!(same <= 1, "{same}");
    }

    #[test]
    fn european_portuguese_uses_its_own_words() {
        assert_eq!(word(L::Capacitor, Language::Pt), "condensador");
        assert_eq!(word(L::Capacitor, Language::Es), "condensador");
        assert_eq!(word(L::LowPass, Language::Pt), "passa-baixo");
        assert_eq!(word(L::Result, Language::En), "Result");
        assert_eq!(word(L::Yes, Language::Pt), "sim");
    }

    #[test]
    fn the_estimate_notice_is_in_every_language() {
        for language in Language::ALL {
            let text = word(L::Estimate, language);
            assert!(text.len() > 10, "{language:?}");
        }
    }

    #[test]
    fn values_that_are_not_finite_become_null_in_json() {
        assert_eq!(value_or_null(f64::INFINITY), Value::Null);
        assert_eq!(value_or_null(1.5), json!(1.5));
        assert_eq!(ratio_text(f64::INFINITY), "inf");
        assert_eq!(db_or_inf(f64::INFINITY), "inf");
        assert_eq!(db_or_inf(f64::NEG_INFINITY), "-inf");
        assert_eq!(db_or_inf(-3.0), "-3.000");
    }
}
