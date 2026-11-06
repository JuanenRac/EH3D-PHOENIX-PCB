// SPDX-License-Identifier: GPL-3.0-or-later
//! The temperature rise of a copper track that carries a current, by two estimates that are set
//! side by side:
//!
//! * the fit of IPC-2221 to its chart of current against cross-section and temperature rise,
//!   `I = k dT^0.44 A^0.725` with `I` in ampere, `dT` in kelvin, `A` the cross-section of the
//!   track in square mil, and `k` 0.048 for a track on an outer layer and 0.024 for one on an inner
//!   layer. The constants are the ones usually printed with the chart (IPC-2221, section 6.2); the
//!   standard itself has not been at hand to check them, and the range of the chart (a rise of 10
//!   to 100 K, up to 35 A) is the one usually quoted, so both are values of the settings and a
//!   result outside the range carries a warning. IPC-2152, which replaces the chart with
//!   measured charts and modifiers for the plane and the thickness of the board, is not applied;
//! * the model of EH3D-THERMAL-LAB for a track: the Joule heating `P = I^2 rho (1 + a (T - 20)) L / A`
//!   cooled by convection from the top face and the two edges, `G = h (w + 2 t) L`, with the rise
//!   `dT = K (1 + a (T_amb - 20)) / (1 - a K)` and `K = I^2 rho / (A h (w + 2 t))` (the same fixed
//!   point that repository finds by iteration, solved in closed form because the length cancels).
//!   It ignores the conduction into the board and radiation, so it is high; a case that would pass
//!   `max_temperature_c` or whose heating grows faster than its cooling has no number.
//!
//! The via is a plated barrel: the annulus of copper around the drill is the cross-section, and
//! only the first estimate applies to it. Nothing here compares either estimate with a
//! measurement, and the convection coefficient is a value of the settings and not a property of the
//! board.

use serde::Serialize;

use crate::error::Language;

/// The heating settings are not valid.
pub const HEAT_SETTINGS_INVALID: &str = "PHOENIX-HEAT-001";
/// A number given to the calculation is not valid.
pub const HEAT_INPUT_INVALID: &str = "PHOENIX-HEAT-002";
/// The file of currents is not valid.
pub const HEAT_CURRENTS_INVALID: &str = "PHOENIX-HEAT-003";
/// The rise of a track is above the limit.
pub const HEAT_OVER_LIMIT: &str = "PHOENIX-HEAT-004";
/// The convection model has no number for the track.
pub const HEAT_OUT_OF_VALIDITY: &str = "PHOENIX-HEAT-005";
/// The chart of IPC-2221 does not reach this track.
pub const HEAT_OUTSIDE_CHART: &str = "PHOENIX-HEAT-006";
/// A net of the file of currents is not on the board.
pub const HEAT_NET_UNKNOWN: &str = "PHOENIX-HEAT-007";
/// A layer is not in the stack-up and the defaults of the settings are used.
pub const HEAT_LAYER_DEFAULTED: &str = "PHOENIX-HEAT-008";
/// A command option is not valid.
pub const HEAT_OPTION_INVALID: &str = "PHOENIX-HEAT-009";
/// An output file exists or cannot be written.
pub const HEAT_OUTPUT_FAILED: &str = "PHOENIX-HEAT-010";
/// An input file cannot be read.
pub const HEAT_INPUT_UNREADABLE: &str = "PHOENIX-HEAT-011";
/// IPC-2152 is not applied.
pub const HEAT_NO_IPC_2152: &str = "PHOENIX-HEAT-012";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const HEAT_TITLES: &[(&str, [&str; 8])] = &[
    (
        HEAT_SETTINGS_INVALID,
        [
            "Heating settings are not valid",
            "La configuración del calentamiento no es válida",
            "Die Einstellungen der Erwärmung sind ungültig",
            "Les réglages de l'échauffement ne sont pas valides",
            "Le impostazioni del riscaldamento non sono valide",
            "発熱の設定が無効です",
            "发热设置无效",
            "As definições do aquecimento não são válidas",
        ],
    ),
    (
        HEAT_INPUT_INVALID,
        [
            "A number given to the heating calculation is not valid",
            "Un número dado al cálculo del calentamiento no es válido",
            "Eine Zahl für die Erwärmungsrechnung ist ungültig",
            "Un nombre donné au calcul de l'échauffement n'est pas valide",
            "Un numero dato al calcolo del riscaldamento non è valido",
            "発熱計算に与えた数値が無効です",
            "提供给发热计算的数值无效",
            "Um número dado ao cálculo do aquecimento não é válido",
        ],
    ),
    (
        HEAT_CURRENTS_INVALID,
        [
            "The file of currents is not valid",
            "El fichero de corrientes no es válido",
            "Die Stromdatei ist ungültig",
            "Le fichier des courants n'est pas valide",
            "Il file delle correnti non è valido",
            "電流ファイルが無効です",
            "电流文件无效",
            "O ficheiro de correntes não é válido",
        ],
    ),
    (
        HEAT_OVER_LIMIT,
        [
            "The temperature rise of a track is above the limit",
            "La subida de temperatura de una pista supera el límite",
            "Die Temperaturerhöhung einer Leiterbahn liegt über der Grenze",
            "L'élévation de température d'une piste dépasse la limite",
            "L'aumento di temperatura di una pista supera il limite",
            "配線の温度上昇が上限を超えています",
            "走线温升超过限值",
            "A subida de temperatura de uma pista excede o limite",
        ],
    ),
    (
        HEAT_OUT_OF_VALIDITY,
        [
            "The convection model has no number for the track",
            "El modelo de convección no da cifra para la pista",
            "Das Konvektionsmodell liefert für die Leiterbahn keine Zahl",
            "Le modèle de convection ne donne aucun chiffre pour la piste",
            "Il modello di convezione non dà alcun numero per la pista",
            "対流モデルはこの配線の数値を出しません",
            "对流模型无法给出该走线的数值",
            "O modelo de convecção não dá número para a pista",
        ],
    ),
    (
        HEAT_OUTSIDE_CHART,
        [
            "The chart of IPC-2221 does not reach this track",
            "La gráfica de IPC-2221 no llega a esta pista",
            "Das Diagramm der IPC-2221 reicht für diese Leiterbahn nicht",
            "L'abaque de l'IPC-2221 ne couvre pas cette piste",
            "Il grafico dell'IPC-2221 non copre questa pista",
            "IPC-2221 のグラフはこの配線に及びません",
            "IPC-2221 的图表不覆盖该走线",
            "O gráfico da IPC-2221 não chega a esta pista",
        ],
    ),
    (
        HEAT_NET_UNKNOWN,
        [
            "A net of the file of currents is not on the board",
            "Una red del fichero de corrientes no está en la placa",
            "Ein Netz der Stromdatei ist nicht auf der Platine",
            "Un réseau du fichier des courants n'est pas sur la carte",
            "Una rete del file delle correnti non è sulla scheda",
            "電流ファイルのネットが基板上にありません",
            "电流文件中的网络不在电路板上",
            "Uma rede do ficheiro de correntes não está na placa",
        ],
    ),
    (
        HEAT_LAYER_DEFAULTED,
        [
            "The layer is not in the stack-up: the defaults of the settings are used",
            "La capa no está en el apilado: se usan los valores por defecto de la configuración",
            "Die Lage ist nicht im Lagenaufbau: es gelten die Standardwerte der Einstellungen",
            "La couche n'est pas dans l'empilage : les valeurs par défaut des réglages sont utilisées",
            "Lo strato non è nello stack-up: si usano i valori predefiniti delle impostazioni",
            "この層はスタックアップにありません。設定の既定値を使います",
            "该层不在叠层中：使用设置中的默认值",
            "A camada não está no empilhamento: usam-se os valores por omissão das definições",
        ],
    ),
    (
        HEAT_OPTION_INVALID,
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
        HEAT_OUTPUT_FAILED,
        [
            "The output file exists or cannot be written",
            "El fichero de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder lässt sich nicht schreiben",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non si può scrivere",
            "出力ファイルが存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        HEAT_INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un fichero de entrada",
            "Eine Eingabedatei lässt sich nicht lesen",
            "Un fichier d'entrée ne peut pas être lu",
            "Non si può leggere un file di ingresso",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        HEAT_NO_IPC_2152,
        [
            "IPC-2152, with its modifiers for the plane and the board, is not applied",
            "IPC-2152, con sus modificadores de plano y de placa, no se aplica",
            "Die IPC-2152 mit ihren Korrekturen für Lage und Platine wird nicht angewendet",
            "L'IPC-2152, avec ses modificateurs de plan et de carte, n'est pas appliquée",
            "L'IPC-2152, con i suoi modificatori di piano e di scheda, non è applicata",
            "IPC-2152（プレーンや基板の補正を含む）は適用されません",
            "未应用 IPC-2152（含平面与板材修正）",
            "A IPC-2152, com os seus modificadores de plano e de placa, não é aplicada",
        ],
    ),
];

/// The title of a heating code in a language.
pub fn heat_title(code: &str, language: Language) -> Option<&'static str> {
    let column = Language::ALL.iter().position(|l| *l == language)?;
    HEAT_TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, titles)| titles.get(column).copied())
}

/// An error of the heating calculation: a code and a detail in English.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatError {
    /// The code, `PHOENIX-HEAT-<NNN>`.
    pub code: &'static str,
    /// What went wrong.
    pub detail: String,
}

impl HeatError {
    /// An error with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> HeatError {
        HeatError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, its title and then the detail in English.
    pub fn message(&self, language: Language) -> String {
        let head = heat_title(self.code, language).unwrap_or("");
        format!("{}: {head}: {}", self.code, self.detail)
    }
}

impl std::fmt::Display for HeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for HeatError {}

/// Nanometres in one mil (a thousandth of an inch).
pub const NM_PER_MIL: f64 = 25_400.0;

/// Where a track lies, for the constant of the chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// On an outer layer, in air.
    External,
    /// On an inner layer, between the laminates.
    Internal,
}

impl Placement {
    /// The word of the files and the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Placement::External => "external",
            Placement::Internal => "internal",
        }
    }

    fn from_word(word: &str) -> Option<Placement> {
        match word {
            "external" => Some(Placement::External),
            "internal" => Some(Placement::Internal),
            _ => None,
        }
    }
}

/// Which estimate the limit is judged by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitBasis {
    /// The fit of IPC-2221.
    Ipc2221,
    /// The convection model of THERMAL-LAB.
    Convection,
    /// The larger of the two (a missing convection number counts as past the limit).
    Larger,
}

/// The tuned values of the heating calculations (`config/heating.cfg`).
#[derive(Debug, Clone, PartialEq)]
pub struct HeatSettings {
    /// The temperature of the air, in degrees Celsius.
    pub ambient_c: f64,
    /// The resistivity of the copper at 20 degrees Celsius, in ohm metre.
    pub resistivity_ohm_m: f64,
    /// The temperature coefficient of that resistivity, per kelvin.
    pub temp_coeff_per_k: f64,
    /// The convection coefficient, in watt per square metre and kelvin.
    pub convection_w_m2k: f64,
    /// The hottest the convection model is used, in degrees Celsius.
    pub max_temperature_c: f64,
    /// The rise above which a track is reported, in kelvin.
    pub max_rise_k: f64,
    /// Which estimate the limit is judged by.
    pub limit_basis: LimitBasis,
    /// The constant of the fit of IPC-2221 for an outer layer.
    pub ipc_k_external: f64,
    /// The constant of the fit of IPC-2221 for an inner layer.
    pub ipc_k_internal: f64,
    /// The exponent of the rise.
    pub ipc_exponent_rise: f64,
    /// The exponent of the area.
    pub ipc_exponent_area: f64,
    /// The smallest rise of the chart, in kelvin.
    pub ipc_min_rise_k: f64,
    /// The largest rise of the chart, in kelvin.
    pub ipc_max_rise_k: f64,
    /// The largest current of the chart, in ampere.
    pub ipc_max_current_a: f64,
    /// The thickness of the copper of a layer the stack-up does not give, in micrometres.
    pub default_copper_um: f64,
    /// Where a track lies when the stack-up does not say.
    pub default_placement: Placement,
    /// The thickness of the plating of the barrel of a via, in micrometres.
    pub via_plating_um: f64,
    /// The most nets in a file of currents.
    pub max_nets: usize,
}

impl Default for HeatSettings {
    fn default() -> Self {
        HeatSettings {
            ambient_c: 25.0,
            resistivity_ohm_m: 1.724e-8,
            temp_coeff_per_k: 0.00393,
            convection_w_m2k: 10.0,
            max_temperature_c: 125.0,
            max_rise_k: 20.0,
            limit_basis: LimitBasis::Ipc2221,
            ipc_k_external: 0.048,
            ipc_k_internal: 0.024,
            ipc_exponent_rise: 0.44,
            ipc_exponent_area: 0.725,
            ipc_min_rise_k: 10.0,
            ipc_max_rise_k: 100.0,
            ipc_max_current_a: 35.0,
            default_copper_um: 35.0,
            default_placement: Placement::Internal,
            via_plating_um: 25.0,
            max_nets: 10_000,
        }
    }
}

fn settings_error(line: usize, key: &str, reason: &str) -> HeatError {
    HeatError::new(
        HEAT_SETTINGS_INVALID,
        format!("line {line}: {key}: {reason}"),
    )
}

fn finite(value: &str, line: usize, key: &str) -> Result<f64, HeatError> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| settings_error(line, key, "not a number"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(settings_error(line, key, "not a finite number"))
    }
}

impl HeatSettings {
    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error.
    pub fn parse(text: &str) -> Result<HeatSettings, HeatError> {
        let mut s = HeatSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                HeatError::new(
                    HEAT_SETTINGS_INVALID,
                    format!("line {line}: not key = value"),
                )
            })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "ambient_c" => s.ambient_c = finite(value, line, key)?,
                "resistivity_ohm_m" => s.resistivity_ohm_m = finite(value, line, key)?,
                "temp_coeff_per_k" => s.temp_coeff_per_k = finite(value, line, key)?,
                "convection_w_m2k" => s.convection_w_m2k = finite(value, line, key)?,
                "max_temperature_c" => s.max_temperature_c = finite(value, line, key)?,
                "max_rise_k" => s.max_rise_k = finite(value, line, key)?,
                "limit_basis" => {
                    s.limit_basis = match value {
                        "ipc2221" => LimitBasis::Ipc2221,
                        "convection" => LimitBasis::Convection,
                        "larger" => LimitBasis::Larger,
                        _ => {
                            return Err(settings_error(
                                line,
                                key,
                                "must be ipc2221, convection or larger",
                            ))
                        }
                    }
                }
                "ipc_k_external" => s.ipc_k_external = finite(value, line, key)?,
                "ipc_k_internal" => s.ipc_k_internal = finite(value, line, key)?,
                "ipc_exponent_rise" => s.ipc_exponent_rise = finite(value, line, key)?,
                "ipc_exponent_area" => s.ipc_exponent_area = finite(value, line, key)?,
                "ipc_min_rise_k" => s.ipc_min_rise_k = finite(value, line, key)?,
                "ipc_max_rise_k" => s.ipc_max_rise_k = finite(value, line, key)?,
                "ipc_max_current_a" => s.ipc_max_current_a = finite(value, line, key)?,
                "default_copper_um" => s.default_copper_um = finite(value, line, key)?,
                "default_placement" => {
                    s.default_placement = Placement::from_word(value)
                        .ok_or_else(|| settings_error(line, key, "must be external or internal"))?
                }
                "via_plating_um" => s.via_plating_um = finite(value, line, key)?,
                "max_nets" => {
                    s.max_nets = value
                        .parse()
                        .map_err(|_| settings_error(line, key, "not a whole number"))?
                }
                _ => return Err(settings_error(line, key, "unknown key")),
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Check the ranges the formulas rely on.
    pub fn validate(&self) -> Result<(), HeatError> {
        let fail = |what: &str| Err(HeatError::new(HEAT_SETTINGS_INVALID, what.to_string()));
        if !(-50.0..=200.0).contains(&self.ambient_c) {
            return fail("ambient_c must be between -50 and 200");
        }
        if !(1.0e-9..=1.0e-6).contains(&self.resistivity_ohm_m) {
            return fail("resistivity_ohm_m must be between 1e-9 and 1e-6");
        }
        if !(0.0..=0.1).contains(&self.temp_coeff_per_k) {
            return fail("temp_coeff_per_k must be between 0 and 0.1");
        }
        if !(0.1..=10_000.0).contains(&self.convection_w_m2k) {
            return fail("convection_w_m2k must be between 0.1 and 10000");
        }
        if self.max_temperature_c <= self.ambient_c || self.max_temperature_c > 1000.0 {
            return fail("max_temperature_c must be above ambient_c and at most 1000");
        }
        if !(0.0..=1000.0).contains(&self.max_rise_k) {
            return fail("max_rise_k must be between 0 and 1000");
        }
        if !(1.0e-3..=10.0).contains(&self.ipc_k_external)
            || !(1.0e-3..=10.0).contains(&self.ipc_k_internal)
        {
            return fail("ipc_k_external and ipc_k_internal must be between 0.001 and 10");
        }
        if !(0.1..=2.0).contains(&self.ipc_exponent_rise)
            || !(0.1..=2.0).contains(&self.ipc_exponent_area)
        {
            return fail("the exponents of the fit must be between 0.1 and 2");
        }
        if self.ipc_min_rise_k <= 0.0 || self.ipc_max_rise_k <= self.ipc_min_rise_k {
            return fail("ipc_min_rise_k must be above 0 and below ipc_max_rise_k");
        }
        if self.ipc_max_current_a <= 0.0 {
            return fail("ipc_max_current_a must be above 0");
        }
        if !(1.0..=500.0).contains(&self.default_copper_um)
            || !(1.0..=200.0).contains(&self.via_plating_um)
        {
            return fail("default_copper_um and via_plating_um must be between 1 and 500 or 200");
        }
        if !(1..=1_000_000).contains(&self.max_nets) {
            return fail("max_nets must be between 1 and 1000000");
        }
        Ok(())
    }

    fn k_for(&self, placement: Placement) -> f64 {
        match placement {
            Placement::External => self.ipc_k_external,
            Placement::Internal => self.ipc_k_internal,
        }
    }
}

/// The result for one track or one via.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceHeating {
    /// Where the track lies.
    pub placement: Placement,
    /// The cross-section in square mil.
    pub area_mil2: f64,
    /// The rise by the fit of IPC-2221, in kelvin.
    pub ipc2221_rise_k: f64,
    /// Whether the track is inside the range of the chart.
    pub ipc2221_in_range: bool,
    /// The current that gives the limit of the settings by the fit, in ampere.
    pub capacity_at_limit_a: f64,
    /// The rise by the convection model, in kelvin; none when the model has no number.
    pub convection_rise_k: Option<f64>,
    /// Why the convection model has no number.
    pub convection_note: Option<String>,
    /// The resistance at 20 degrees Celsius, in milliohm per millimetre.
    pub resistance_mohm_per_mm: f64,
    /// The power at 20 degrees Celsius, in milliwatt per millimetre.
    pub power_mw_per_mm: f64,
    /// The rise the limit is judged by, in kelvin; none when the basis is the convection model
    /// and it has no number.
    pub judged_rise_k: Option<f64>,
    /// Whether the judged rise is above the limit (a missing number counts as above).
    pub over_limit: bool,
}

fn input_error(what: &str) -> HeatError {
    HeatError::new(HEAT_INPUT_INVALID, what.to_string())
}

/// The current that the fit of IPC-2221 allows for a cross-section in square mil and a rise in
/// kelvin.
pub fn ipc2221_current_a(
    area_mil2: f64,
    rise_k: f64,
    placement: Placement,
    settings: &HeatSettings,
) -> f64 {
    settings.k_for(placement)
        * rise_k.powf(settings.ipc_exponent_rise)
        * area_mil2.powf(settings.ipc_exponent_area)
}

/// The rise that the fit of IPC-2221 gives for a current through a cross-section in square mil.
pub fn ipc2221_rise_k(
    area_mil2: f64,
    current_a: f64,
    placement: Placement,
    settings: &HeatSettings,
) -> f64 {
    let base = current_a / (settings.k_for(placement) * area_mil2.powf(settings.ipc_exponent_area));
    base.powf(1.0 / settings.ipc_exponent_rise)
}

/// The rise of the convection model of THERMAL-LAB for a track of a width, a thickness and a
/// current, or the reason it has no number.
pub fn convection_rise_k(
    width_nm: i64,
    thickness_nm: i64,
    current_a: f64,
    settings: &HeatSettings,
) -> Result<f64, String> {
    let area_m2 = width_nm as f64 * 1.0e-9 * (thickness_nm as f64 * 1.0e-9);
    let exposed_m = (width_nm as f64 + 2.0 * thickness_nm as f64) * 1.0e-9;
    let k = current_a * current_a * settings.resistivity_ohm_m
        / (area_m2 * settings.convection_w_m2k * exposed_m);
    let a = settings.temp_coeff_per_k;
    if a * k >= 1.0 {
        return Err("the heating grows faster than the cooling".to_string());
    }
    let rise = k * (1.0 + a * (settings.ambient_c - 20.0)) / (1.0 - a * k);
    if !rise.is_finite() || settings.ambient_c + rise > settings.max_temperature_c {
        return Err(format!(
            "the track would pass {} C, where the resistivity model is not used",
            settings.max_temperature_c
        ));
    }
    Ok(rise)
}

/// The heating of a track of a width and a thickness in nanometres that carries a current in
/// ampere, on an outer or an inner layer.
pub fn trace_heating(
    width_nm: i64,
    thickness_nm: i64,
    current_a: f64,
    placement: Placement,
    settings: &HeatSettings,
) -> Result<TraceHeating, HeatError> {
    if width_nm <= 0 || thickness_nm <= 0 {
        return Err(input_error(
            "the width and the thickness must be above zero",
        ));
    }
    if !current_a.is_finite() || !(0.0..=100_000.0).contains(&current_a) {
        return Err(input_error(
            "the current must be between 0 and 100000 ampere",
        ));
    }
    let area_mil2 = width_nm as f64 / NM_PER_MIL * (thickness_nm as f64 / NM_PER_MIL);
    Ok(finish(
        area_mil2,
        width_nm,
        thickness_nm,
        current_a,
        placement,
        settings,
        true,
    ))
}

fn finish(
    area_mil2: f64,
    width_nm: i64,
    thickness_nm: i64,
    current_a: f64,
    placement: Placement,
    settings: &HeatSettings,
    with_convection: bool,
) -> TraceHeating {
    let ipc = if current_a == 0.0 {
        0.0
    } else {
        ipc2221_rise_k(area_mil2, current_a, placement, settings)
    };
    let in_range = current_a == 0.0
        || (ipc >= settings.ipc_min_rise_k
            && ipc <= settings.ipc_max_rise_k
            && current_a <= settings.ipc_max_current_a);
    let capacity = ipc2221_current_a(area_mil2, settings.max_rise_k, placement, settings);
    let (convection, note) = if with_convection {
        match convection_rise_k(width_nm, thickness_nm, current_a, settings) {
            Ok(rise) => (Some(rise), None),
            Err(why) => (None, Some(why)),
        }
    } else {
        (
            None,
            Some("the model is for a track, not for a via".to_string()),
        )
    };
    let area_m2 = width_nm as f64 * 1.0e-9 * (thickness_nm as f64 * 1.0e-9);
    let resistance_per_m = settings.resistivity_ohm_m / area_m2;
    let judged = match settings.limit_basis {
        LimitBasis::Ipc2221 => Some(ipc),
        LimitBasis::Convection => convection,
        LimitBasis::Larger => convection.map(|c| c.max(ipc)),
    };
    let over_limit = match judged {
        Some(rise) => rise > settings.max_rise_k,
        None => with_convection,
    };
    TraceHeating {
        placement,
        area_mil2,
        ipc2221_rise_k: ipc,
        ipc2221_in_range: in_range,
        capacity_at_limit_a: capacity,
        convection_rise_k: convection,
        convection_note: note,
        resistance_mohm_per_mm: resistance_per_m,
        power_mw_per_mm: current_a * current_a * resistance_per_m,
        judged_rise_k: judged,
        over_limit,
    }
}

/// The heating of the barrel of a via of a drill diameter, in nanometres, that carries a current:
/// the cross-section is the annulus of plating around the drill, the barrel is taken as an inner
/// conductor, and only the fit of IPC-2221 applies.
pub fn via_heating(
    drill_nm: i64,
    current_a: f64,
    settings: &HeatSettings,
) -> Result<TraceHeating, HeatError> {
    if drill_nm <= 0 {
        return Err(input_error("the drill must be above zero"));
    }
    if !current_a.is_finite() || !(0.0..=100_000.0).contains(&current_a) {
        return Err(input_error(
            "the current must be between 0 and 100000 ampere",
        ));
    }
    let plating_nm = settings.via_plating_um * 1000.0;
    let drill = drill_nm as f64;
    let area_nm2 = std::f64::consts::PI * plating_nm * (drill + plating_nm);
    let area_mil2 = area_nm2 / (NM_PER_MIL * NM_PER_MIL);
    // the width and thickness stand for the area only; the convection model is not used
    let width = (area_nm2.sqrt().round() as i64).max(1);
    Ok(finish(
        area_mil2,
        width,
        width,
        current_a,
        Placement::Internal,
        settings,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> HeatSettings {
        HeatSettings::default()
    }

    #[test]
    fn the_fit_of_ipc_2221_gives_the_rise_worked_out_by_hand() {
        // 20 mil of width, 35 um (1378.0 mil / 1000) of copper: 27.559055 square mil
        let s = settings();
        let out = trace_heating(508_000, 35_000, 1.0, Placement::External, &s).unwrap();
        assert!((out.area_mil2 - 27.559_055_118_110_237).abs() < 1.0e-12);
        assert!((out.ipc2221_rise_k - 4.207_423_229_307_952).abs() < 1.0e-9);
        let inner = trace_heating(508_000, 35_000, 1.0, Placement::Internal, &s).unwrap();
        assert!((inner.ipc2221_rise_k - 20.331_774_365_876_043).abs() < 1.0e-8);
        let wide = trace_heating(1_000_000, 35_000, 3.0, Placement::External, &s).unwrap();
        assert!((wide.ipc2221_rise_k - 16.738_876_502_509_573).abs() < 1.0e-8);
    }

    #[test]
    fn the_capacity_and_the_rise_are_inverse_of_each_other() {
        let s = settings();
        for placement in [Placement::External, Placement::Internal] {
            for rise in [10.0, 20.0, 55.0, 100.0] {
                let current = ipc2221_current_a(30.0, rise, placement, &s);
                let back = ipc2221_rise_k(30.0, current, placement, &s);
                assert!((back - rise).abs() < 1.0e-9 * rise, "{rise} {back}");
            }
        }
        let known = ipc2221_current_a(27.559_055_118_110_237, 10.0, Placement::External, &s);
        assert!((known - 0.992_786_983_047_720_2).abs() < 1.0e-12, "{known}");
    }

    #[test]
    fn the_convection_model_agrees_with_the_closed_form_of_thermal_lab() {
        // the case of the tests of EH3D-THERMAL-LAB: 2 A in a trace of 2 mm by 35 um in air at
        // 20 C with 20 W/m2K; the rise is P0 / (G - a P0) with P0 = I^2 R and G = h A_s
        let s = HeatSettings {
            ambient_c: 20.0,
            convection_w_m2k: 20.0,
            ..settings()
        };
        let length = 0.1;
        let section = 2.0e-3 * 35.0e-6;
        let surface = (2.0e-3 + 2.0 * 35.0e-6) * length;
        let g = 20.0 * surface;
        let p0 = 4.0 * s.resistivity_ohm_m * length / section;
        let exact = p0 / (g - s.temp_coeff_per_k * p0);
        let found = convection_rise_k(2_000_000, 35_000, 2.0, &s).unwrap();
        assert!((found - exact).abs() < 1.0e-9 * exact, "{found} {exact}");
        assert!((found - 26.250_603_733_432_726).abs() < 1.0e-8);
    }

    #[test]
    fn a_hot_ambient_raises_the_resistance_and_so_the_rise() {
        let cool = HeatSettings {
            ambient_c: 20.0,
            convection_w_m2k: 20.0,
            ..settings()
        };
        let hot = HeatSettings {
            ambient_c: 60.0,
            convection_w_m2k: 20.0,
            ..settings()
        };
        let a = convection_rise_k(2_000_000, 35_000, 2.0, &cool).unwrap();
        let b = convection_rise_k(2_000_000, 35_000, 2.0, &hot).unwrap();
        assert!(b > a);
    }

    #[test]
    fn a_track_that_would_pass_the_hottest_temperature_has_no_convection_number() {
        let s = settings();
        let out = trace_heating(508_000, 35_000, 1.0, Placement::External, &s).unwrap();
        assert_eq!(out.convection_rise_k, None);
        assert!(out.convection_note.as_deref().unwrap().contains("125"));
        // a current that makes the heating run away
        let runaway = convection_rise_k(100_000, 35_000, 20.0, &s).unwrap_err();
        assert!(runaway.contains("faster") || runaway.contains("125"));
    }

    #[test]
    fn no_current_means_no_rise_and_a_negative_or_missing_number_is_refused() {
        let s = settings();
        let out = trace_heating(500_000, 35_000, 0.0, Placement::External, &s).unwrap();
        assert_eq!(out.ipc2221_rise_k, 0.0);
        assert_eq!(out.convection_rise_k, Some(0.0));
        assert!(out.ipc2221_in_range && !out.over_limit);
        for (w, t, i) in [
            (0, 35_000, 1.0),
            (500_000, 0, 1.0),
            (500_000, 35_000, -1.0),
            (500_000, 35_000, f64::NAN),
            (500_000, 35_000, 1.0e9),
        ] {
            let error = trace_heating(w, t, i, Placement::External, &s).unwrap_err();
            assert_eq!(error.code, HEAT_INPUT_INVALID);
        }
    }

    #[test]
    fn the_limit_is_judged_by_the_basis_of_the_settings() {
        let strict = HeatSettings {
            max_rise_k: 5.0,
            ..settings()
        };
        // 4.2 K by the fit: under 5 K
        let under = trace_heating(508_000, 35_000, 1.0, Placement::External, &strict).unwrap();
        assert!(!under.over_limit);
        let inner = trace_heating(508_000, 35_000, 1.0, Placement::Internal, &strict).unwrap();
        assert!(inner.over_limit);
        let by_convection = HeatSettings {
            limit_basis: LimitBasis::Convection,
            max_rise_k: 5.0,
            ..settings()
        };
        // the model has no number for this track, which counts as past the limit
        let none =
            trace_heating(508_000, 35_000, 1.0, Placement::External, &by_convection).unwrap();
        assert!(none.over_limit && none.judged_rise_k.is_none());
        let larger = HeatSettings {
            limit_basis: LimitBasis::Larger,
            ..settings()
        };
        let both = trace_heating(2_000_000, 35_000, 2.0, Placement::External, &larger).unwrap();
        assert!(both.judged_rise_k.unwrap() >= both.ipc2221_rise_k);
    }

    #[test]
    fn the_chart_of_ipc_2221_does_not_reach_a_small_rise_or_a_large_current() {
        let s = settings();
        // 4.2 K is below the 10 K of the chart
        let low = trace_heating(508_000, 35_000, 1.0, Placement::External, &s).unwrap();
        assert!(!low.ipc2221_in_range);
        let mid = trace_heating(508_000, 35_000, 2.0, Placement::External, &s).unwrap();
        assert!(mid.ipc2221_in_range, "{}", mid.ipc2221_rise_k);
    }

    #[test]
    fn a_via_is_an_annulus_of_plating_and_has_only_the_fit() {
        let s = settings();
        let out = via_heating(300_000, 1.0, &s).unwrap();
        // pi * 25 um * (300 um + 25 um) = 25525.4 um2 = 39.564 square mil
        let area = std::f64::consts::PI * 25.0 * 325.0 / (25.4 * 25.4);
        assert!((out.area_mil2 - area).abs() < 1.0e-9, "{}", out.area_mil2);
        assert_eq!(out.convection_rise_k, None);
        assert_eq!(out.placement, Placement::Internal);
        assert!(!out.over_limit || out.ipc2221_rise_k > s.max_rise_k);
        assert_eq!(
            via_heating(0, 1.0, &s).unwrap_err().code,
            HEAT_INPUT_INVALID
        );
    }

    #[test]
    fn the_resistance_and_the_power_per_millimetre_follow_from_the_resistivity() {
        let s = settings();
        let out = trace_heating(508_000, 35_000, 2.0, Placement::External, &s).unwrap();
        assert!((out.resistance_mohm_per_mm - 969.628_796_400_450_1).abs() < 1.0e-6);
        assert!((out.power_mw_per_mm - 4.0 * out.resistance_mohm_per_mm).abs() < 1.0e-6);
    }

    #[test]
    fn the_settings_file_that_ships_is_the_default_and_bad_lines_are_refused() {
        let text = include_str!("../../../config/heating.cfg");
        assert_eq!(HeatSettings::parse(text).unwrap(), HeatSettings::default());
        assert_eq!(HeatSettings::parse("").unwrap(), HeatSettings::default());
        for bad in [
            "x",
            "unknown = 1",
            "ambient_c = hot",
            "ambient_c = 500",
            "resistivity_ohm_m = 0",
            "convection_w_m2k = 0",
            "max_temperature_c = 10",
            "limit_basis = both",
            "default_placement = middle",
            "ipc_min_rise_k = 200",
            "max_nets = 0",
            "via_plating_um = 0",
        ] {
            assert_eq!(
                HeatSettings::parse(bad).unwrap_err().code,
                HEAT_SETTINGS_INVALID,
                "{bad}"
            );
        }
        let s =
            HeatSettings::parse("limit_basis = larger\ndefault_placement = external\n").unwrap();
        assert_eq!(s.limit_basis, LimitBasis::Larger);
        assert_eq!(s.default_placement, Placement::External);
    }

    #[test]
    fn every_code_has_a_title_in_the_eight_languages() {
        assert_eq!(HEAT_TITLES.len(), 12);
        for (index, (code, titles)) in HEAT_TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-HEAT-{:03}", index + 1));
            assert!(titles.iter().all(|t| !t.trim().is_empty()));
            for language in Language::ALL {
                assert!(heat_title(code, language).is_some());
            }
        }
        let error = HeatError::new(HEAT_OVER_LIMIT, "net X");
        assert!(error.message(Language::Es).contains("subida"));
        assert_eq!(error.to_string(), "PHOENIX-HEAT-004: net X");
    }
}
