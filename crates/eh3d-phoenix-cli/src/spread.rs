// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `spread`: the lowest and the highest impedance of a microstrip or a centred
//! stripline when the width of the track, the height of the dielectric and the permittivity are
//! each off by their tolerance, and the largest tolerance of the width for which the line stays
//! within a per cent of a target impedance.
//!
//! Exit codes: 0 done (and, when a target is given, the whole box of the tolerances fits), 1 it
//! does not fit, 2 the arguments, the settings or the geometry are unusable. Nothing is read
//! except the settings file and nothing is written.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_stackup::spread::{
    impedance_spread, max_width_tolerance, Spread, SpreadGeometry, SpreadKind, SpreadTolerances,
};
use eh3d_phoenix_stackup::{max_width_tolerance_text, spread_text, Language};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/spread.cfg";

/// The settings of the spread are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SPREAD-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SPREAD-002";
/// The line cannot be worked out for the values given.
pub const NOT_COMPUTABLE: &str = "PHOENIX-SPREAD-003";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "The settings of the spread are not valid",
            "Los ajustes de la dispersión no son válidos",
            "Die Einstellungen der Streuung sind ungültig",
            "Les réglages de la dispersion ne sont pas valides",
            "Le impostazioni della dispersione non sono valide",
            "ばらつきの設定が無効です",
            "离散范围的设置无效",
            "As definições da dispersão não são válidas",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "An option of the spread is not valid",
            "Una opción de la dispersión no es válida",
            "Eine Option der Streuung ist ungültig",
            "Une option de la dispersion n'est pas valide",
            "Un'opzione della dispersione non è valida",
            "ばらつきのオプションが無効です",
            "离散范围的选项无效",
            "Uma opção da dispersão não é válida",
        ],
    ),
    (
        NOT_COMPUTABLE,
        [
            "The line cannot be worked out for these values",
            "No se puede calcular la línea con estos valores",
            "Die Leitung lässt sich mit diesen Werten nicht berechnen",
            "La ligne ne peut pas être calculée avec ces valeurs",
            "La linea non si può calcolare con questi valori",
            "これらの値では線路を計算できません",
            "无法用这些数值计算该线路",
            "Não é possível calcular a linha com estes valores",
        ],
    ),
];

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, language: Language) -> &'static str {
    let column = Language::ALL
        .iter()
        .position(|l| *l == language)
        .unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// An error of the command: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadError {
    /// The code, `PHOENIX-SPREAD-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case, in English.
    pub detail: String,
}

impl SpreadError {
    fn new(code: &'static str, detail: impl Into<String>) -> SpreadError {
        SpreadError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, language: Language) -> String {
        format!(
            "{}: {}: {}",
            self.code,
            title(self.code, language),
            self.detail
        )
    }
}

pub(crate) const USAGE: &str =
    "phoenix spread [--config FILE] --kind microstrip|stripline --width-um W --height-um H --er DK
                [--thickness-um T] [--tol-width-um X] [--tol-height-um X] [--tol-er X]
                [--target OHM [--percent P] [--max-width-tolerance]] [--json] [--lang CODE]

  spread        the lowest and the highest impedance of a microstrip or a centred stripline when
                the width of the track, the height of the dielectric (the spacing of the two
                planes of a stripline) and the permittivity are each off by their plus or minus
                tolerance, with how far each end is from the nominal impedance in per cent; with
                --target the last line says whether the whole box of the tolerances is within
                --percent of that impedance (exit code 1 when it is not), and with
                --max-width-tolerance it gives the largest tolerance of the width that still
                fits. The defaults of the thickness, the tolerances and the per cent are in
                config/spread.cfg; a quasi-static estimate, not a simulation
  --config      the defaults (default: config/spread.cfg when it exists)
  --kind        microstrip or stripline
  --width-um    the width of the track in micrometres (1 to 100000)
  --height-um   the height of the dielectric in micrometres (1 to 100000)
  --er DK       the relative permittivity (1 to 100)
  --thickness-um  the thickness of the copper of the track in micrometres
  --tol-width-um, --tol-height-um, --tol-er
                the plus or minus tolerances, never more than half of the value
  --target OHM  the impedance asked for (1 to 1000)
  --percent P   how far from the target the whole box may be, in per cent (0 to 100)
  --max-width-tolerance
                work out the largest tolerance of the width that fits (needs --target)
  --json        print the figures as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
";

/// The defaults that the options may change, read from `config/spread.cfg`.
#[derive(Debug, Clone, PartialEq)]
pub struct Defaults {
    /// The thickness of the copper of the track, in micrometres.
    pub thickness_um: f64,
    /// The tolerance of the width, in micrometres.
    pub tol_width_um: f64,
    /// The tolerance of the height, in micrometres.
    pub tol_height_um: f64,
    /// The tolerance of the permittivity.
    pub tol_er: f64,
    /// How far from the target the whole box may be, in per cent.
    pub percent: f64,
}

impl Default for Defaults {
    fn default() -> Defaults {
        Defaults {
            thickness_um: 35.0,
            tol_width_um: 0.0,
            tol_height_um: 0.0,
            tol_er: 0.0,
            percent: 10.0,
        }
    }
}

/// A number of the settings or of an option, finite and inside a range.
fn number(text: &str, low: f64, high: f64) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    (value.is_finite() && (low..=high).contains(&value)).then_some(value)
}

impl Defaults {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored; an error
    /// names the key.
    pub fn parse(text: &str) -> Result<Defaults, SpreadError> {
        let mut d = Defaults::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| {
                SpreadError::new(SETTINGS_INVALID, "a line is not of the form key = value")
            })?;
            let (key, value) = (key.trim(), value.trim());
            let (slot, low, high) = match key {
                "thickness_um" => (&mut d.thickness_um, 0.0, 500.0),
                "tol_width_um" => (&mut d.tol_width_um, 0.0, 10_000.0),
                "tol_height_um" => (&mut d.tol_height_um, 0.0, 10_000.0),
                "tol_er" => (&mut d.tol_er, 0.0, 10.0),
                "percent" => (&mut d.percent, 0.0, 100.0),
                _ => {
                    return Err(SpreadError::new(
                        SETTINGS_INVALID,
                        format!("unknown key {key}"),
                    ))
                }
            };
            *slot = number(value, low, high).ok_or_else(|| {
                SpreadError::new(
                    SETTINGS_INVALID,
                    format!("the value of {key} is not a number from {low} to {high}"),
                )
            })?;
        }
        Ok(d)
    }
}

/// What the command was asked.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The geometry of the line, in metres.
    pub geometry: SpreadGeometry,
    /// The tolerances, in metres and a plain number.
    pub tolerances: SpreadTolerances,
    /// The impedance asked for, in ohm.
    pub target_ohm: Option<f64>,
    /// How far from the target the box may be, in per cent.
    pub percent: f64,
    /// Whether the largest tolerance of the width is wanted.
    pub max_width: bool,
    /// Whether the answer is JSON.
    pub json: bool,
    /// The language of the words.
    pub language: Language,
}

/// The value of the option `--config` in a list of arguments, if it is there.
fn config_path<'a>(args: &[&'a str]) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == "--config")
        .map(|pair| pair[1])
}

/// Reads the arguments into a request, with the defaults for what they leave out.
pub fn parse_args(args: &[&str], defaults: &Defaults) -> Result<Request, SpreadError> {
    let bad = |what: String| SpreadError::new(OPTION_INVALID, what);
    let mut kind: Option<SpreadKind> = None;
    let (mut width, mut height, mut er) = (None, None, None);
    let mut thickness = defaults.thickness_um;
    let mut tolerances = (
        defaults.tol_width_um,
        defaults.tol_height_um,
        defaults.tol_er,
    );
    let mut target = None;
    let mut percent = None;
    let mut max_width = false;
    let mut json = false;
    let mut language = Language::En;
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str, low: f64, high: f64| -> Result<f64, SpreadError> {
            let text = iter
                .next()
                .ok_or_else(|| bad(format!("{arg} needs a number")))?;
            number(text, low, high)
                .ok_or_else(|| bad(format!("{arg} is a number from {low} to {high} ({what})")))
        };
        match arg {
            "--config" => {
                iter.next();
            }
            "--json" => json = true,
            "--max-width-tolerance" => max_width = true,
            "--kind" => {
                kind = Some(match iter.next() {
                    Some("microstrip") => SpreadKind::Microstrip,
                    Some("stripline") => SpreadKind::Stripline,
                    _ => return Err(bad("--kind is microstrip or stripline".to_owned())),
                })
            }
            "--lang" => {
                language = iter
                    .next()
                    .and_then(Language::from_tag)
                    .ok_or_else(|| bad("--lang is en, es, de, fr, it, ja, zh or pt".to_owned()))?
            }
            "--width-um" => width = Some(value("the width of the track", 1.0, 100_000.0)?),
            "--height-um" => height = Some(value("the height of the dielectric", 1.0, 100_000.0)?),
            "--er" => er = Some(value("the relative permittivity", 1.0, 100.0)?),
            "--thickness-um" => thickness = value("the thickness of the copper", 0.0, 500.0)?,
            "--tol-width-um" => tolerances.0 = value("the tolerance of the width", 0.0, 10_000.0)?,
            "--tol-height-um" => {
                tolerances.1 = value("the tolerance of the height", 0.0, 10_000.0)?
            }
            "--tol-er" => tolerances.2 = value("the tolerance of the permittivity", 0.0, 10.0)?,
            "--target" => target = Some(value("the target impedance in ohm", 1.0, 1000.0)?),
            "--percent" => percent = Some(value("the per cent of the target", 0.0, 100.0)?),
            other => return Err(bad(format!("unknown option or argument {other}"))),
        }
    }
    let missing = |name: &str| bad(format!("{name} is needed"));
    let kind = kind.ok_or_else(|| missing("--kind"))?;
    let width = width.ok_or_else(|| missing("--width-um"))?;
    let height = height.ok_or_else(|| missing("--height-um"))?;
    let er = er.ok_or_else(|| missing("--er"))?;
    if target.is_none() && (max_width || percent.is_some()) {
        return Err(bad(
            "--percent and --max-width-tolerance need --target".to_owned()
        ));
    }
    Ok(Request {
        geometry: SpreadGeometry {
            kind,
            width_m: width * 1.0e-6,
            thickness_m: thickness * 1.0e-6,
            height_m: height * 1.0e-6,
            er,
        },
        tolerances: SpreadTolerances {
            width_m: tolerances.0 * 1.0e-6,
            height_m: tolerances.1 * 1.0e-6,
            er: tolerances.2,
        },
        target_ohm: target,
        percent: percent.unwrap_or(defaults.percent),
        max_width,
        json,
        language,
    })
}

/// The figures of a request.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The nominal, lowest and highest impedance.
    pub spread: Spread,
    /// Whether the whole box fits, when a target was given.
    pub fits: Option<bool>,
    /// The largest tolerance of the width in metres, when it was asked: `Some(None)` when even
    /// a width with no tolerance does not fit.
    pub max_width_m: Option<Option<f64>>,
}

/// Works the request out.
pub fn compute(request: &Request) -> Result<Outcome, SpreadError> {
    let failed =
        |e: eh3d_phoenix_stackup::StackupError| SpreadError::new(NOT_COMPUTABLE, e.to_string());
    let spread = impedance_spread(&request.geometry, &request.tolerances).map_err(failed)?;
    let fits = request
        .target_ohm
        .map(|target| spread.fits(target, request.percent));
    let max_width_m = match (request.max_width, request.target_ohm) {
        (true, Some(target)) => Some(
            max_width_tolerance(
                &request.geometry,
                &request.tolerances,
                target,
                request.percent,
            )
            .map_err(failed)?,
        ),
        _ => None,
    };
    Ok(Outcome {
        spread,
        fits,
        max_width_m,
    })
}

fn json_number(value: Option<f64>) -> String {
    value.map_or("null".to_owned(), |v| format!("{v:.4}"))
}

/// The answer as text or as JSON.
pub fn render(request: &Request, outcome: &Outcome) -> String {
    let half_width = request.geometry.width_m * 0.5;
    if request.json {
        let kind = match request.geometry.kind {
            SpreadKind::Microstrip => "microstrip",
            SpreadKind::Stripline => "stripline",
        };
        let s = &outcome.spread;
        let max = match outcome.max_width_m {
            None => "{\"asked\":false}".to_owned(),
            Some(answer) => format!(
                "{{\"asked\":true,\"um\":{},\"every_tolerance_fits\":{}}}",
                json_number(answer.map(|m| m * 1.0e6)),
                answer.is_some_and(|m| m >= half_width)
            ),
        };
        return format!(
            "{{\"format\":\"phoenix-spread/1\",\"kind\":\"{kind}\",\"nominal_ohm\":{:.4},\"min_ohm\":{:.4},\"max_ohm\":{:.4},\"minus_percent\":{:.4},\"plus_percent\":{:.4},\"target_ohm\":{},\"percent\":{},\"fits\":{},\"max_width_tolerance\":{max}}}\n",
            s.nominal_ohm,
            s.min_ohm,
            s.max_ohm,
            s.minus_percent(),
            s.plus_percent(),
            json_number(request.target_ohm),
            json_number(request.target_ohm.map(|_| request.percent)),
            outcome.fits.map_or("null".to_owned(), |f| f.to_string())
        );
    }
    let check = request.target_ohm.map(|t| (t, request.percent));
    let mut text = spread_text(&outcome.spread, check, request.language);
    if let Some(answer) = outcome.max_width_m {
        text.push_str(&max_width_tolerance_text(
            answer,
            half_width,
            request.language,
        ));
    }
    text
}

/// The exit code of an outcome: 1 when a target was given and the box does not fit.
pub fn exit_code(outcome: &Outcome) -> i32 {
    let misses = outcome.fits == Some(false) || outcome.max_width_m == Some(None);
    i32::from(misses)
}

fn load_defaults(path: Option<&str>) -> Result<Defaults, SpreadError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(Defaults::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path).map_err(|e| {
                SpreadError::new(
                    SETTINGS_INVALID,
                    format!("the settings cannot be read: {e}"),
                )
            })?;
            Defaults::parse(&text)
        }
    }
}

fn language_of(args: &[&str]) -> Language {
    args.windows(2)
        .find(|pair| pair[0] == "--lang")
        .and_then(|pair| Language::from_tag(pair[1]))
        .unwrap_or(Language::En)
}

pub(crate) fn spread_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let language = language_of(args);
    let result = load_defaults(config_path(args))
        .and_then(|defaults| parse_args(args, &defaults))
        .and_then(|request| compute(&request).map(|outcome| (request, outcome)));
    match result {
        Ok((request, outcome)) => {
            let _ = out.write_all(render(&request, &outcome).as_bytes());
            exit_code(&outcome)
        }
        Err(error) => {
            if error.code == OPTION_INVALID {
                let _ = err.write_all(USAGE.as_bytes());
            }
            let _ = writeln!(err, "error: {}", error.message(language));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = spread_command(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    const LINE: [&str; 10] = [
        "--kind",
        "microstrip",
        "--width-um",
        "300",
        "--height-um",
        "180",
        "--er",
        "4.3",
        "--thickness-um",
        "35",
    ];

    fn with(extra: &[&str]) -> Vec<String> {
        LINE.iter().chain(extra).map(|s| (*s).to_owned()).collect()
    }

    fn call(extra: &[&str]) -> (i32, String, String) {
        let args = with(extra);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run(&refs)
    }

    #[test]
    fn the_settings_file_of_the_repository_holds_the_defaults() {
        let text = include_str!("../../../config/spread.cfg");
        assert_eq!(Defaults::parse(text).unwrap(), Defaults::default());
    }

    #[test]
    fn a_setting_that_is_not_valid_names_its_key() {
        for (text, key) in [
            ("thickness_um = 501\n", "thickness_um"),
            ("tol_er = -1\n", "tol_er"),
            ("percent = x\n", "percent"),
            ("percent = 101\n", "percent"),
            ("tol_width_um = nan\n", "tol_width_um"),
            ("tol_height_um = inf\n", "tol_height_um"),
            ("color = red\n", "color"),
        ] {
            let e = Defaults::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(key), "{}", e.detail);
            assert!(!e.detail.contains("line"), "{}", e.detail);
        }
        assert!(Defaults::parse("no equals sign\n").is_err());
        let d = Defaults::parse("# a\n\npercent = 5\ntol_er = 0.2\n").unwrap();
        assert_eq!((d.percent, d.tol_er, d.thickness_um), (5.0, 0.2, 35.0));
    }

    #[test]
    fn without_tolerances_the_ends_are_the_nominal_impedance() {
        let (code, text, err) = call(&[]);
        assert_eq!((code, err.as_str()), (0, ""));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "{text}");
        let nominal = lines[0].split(": ").nth(1).unwrap();
        for line in &lines[1..] {
            assert!(line.contains(nominal.split(' ').next().unwrap()), "{text}");
        }
        assert!(lines[1].ends_with("(-0.00 %)") && lines[2].ends_with("(+0.00 %)"));
    }

    #[test]
    fn the_figures_are_the_ones_of_the_library_and_the_tolerances_widen_the_box() {
        let (code, text, _) = call(&[
            "--tol-width-um",
            "20",
            "--tol-height-um",
            "10",
            "--tol-er",
            "0.2",
            "--json",
        ]);
        assert_eq!(code, 0);
        let request = parse_args(
            &[
                "--kind",
                "microstrip",
                "--width-um",
                "300",
                "--height-um",
                "180",
                "--er",
                "4.3",
                "--tol-width-um",
                "20",
                "--tol-height-um",
                "10",
                "--tol-er",
                "0.2",
            ],
            &Defaults::default(),
        )
        .unwrap();
        let outcome = compute(&request).unwrap();
        let s = outcome.spread;
        assert!(s.min_ohm < s.nominal_ohm && s.nominal_ohm < s.max_ohm);
        assert!(
            text.contains(&format!("\"nominal_ohm\":{:.4}", s.nominal_ohm)),
            "{text}"
        );
        assert!(
            text.contains(&format!("\"min_ohm\":{:.4}", s.min_ohm)),
            "{text}"
        );
        assert!(
            text.contains(&format!("\"max_ohm\":{:.4}", s.max_ohm)),
            "{text}"
        );
        assert!(text.starts_with("{\"format\":\"phoenix-spread/1\",\"kind\":\"microstrip\""));
        assert!(
            text.contains("\"fits\":null") && text.contains("\"asked\":false"),
            "{text}"
        );
        // the same request gives the same bytes
        assert_eq!(
            call(&[
                "--tol-width-um",
                "20",
                "--tol-height-um",
                "10",
                "--tol-er",
                "0.2",
                "--json"
            ])
            .1,
            text
        );
    }

    #[test]
    fn the_target_decides_the_exit_code_and_the_largest_tolerance_is_a_fit() {
        // a percentage of 100 always fits, one of 0 fits only a box of a single impedance
        let (code, text, _) = call(&["--tol-width-um", "20", "--target", "50", "--percent", "100"]);
        assert_eq!(code, 0, "{text}");
        assert!(text.trim_end().ends_with(": yes"), "{text}");
        let (code, text, _) = call(&["--tol-width-um", "20", "--target", "50", "--percent", "0"]);
        assert_eq!(code, 1, "{text}");
        assert!(text.trim_end().ends_with(": no"), "{text}");
        // the largest tolerance fits, and one a little larger does not
        let args = [
            "--kind",
            "microstrip",
            "--width-um",
            "300",
            "--height-um",
            "180",
            "--er",
            "4.3",
            "--target",
            "50",
            "--percent",
            "10",
            "--max-width-tolerance",
        ];
        let request = parse_args(&args, &Defaults::default()).unwrap();
        let outcome = compute(&request).unwrap();
        let Some(Some(limit)) = outcome.max_width_m else {
            // the nominal line is further than 10 per cent from 50 ohm: nothing fits
            assert_eq!(exit_code(&outcome), 1);
            return;
        };
        let at = |width_m: f64| {
            impedance_spread(
                &request.geometry,
                &SpreadTolerances {
                    width_m,
                    ..request.tolerances
                },
            )
            .unwrap()
            .fits(50.0, 10.0)
        };
        assert!(at(limit));
        if limit < request.geometry.width_m * 0.5 {
            assert!(!at(limit * 1.001 + 1.0e-9));
        }
    }

    #[test]
    fn the_words_are_in_each_language_and_the_figures_are_the_same() {
        let mut seen = Vec::new();
        for tag in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
            let (code, text, err) = call(&[
                "--tol-width-um",
                "15",
                "--target",
                "50",
                "--percent",
                "50",
                "--max-width-tolerance",
                "--lang",
                tag,
            ]);
            assert_eq!((code, err.as_str()), (0, ""), "{tag}");
            let figures: Vec<String> = text
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .filter(|piece| piece.contains('.'))
                .map(str::to_owned)
                .collect();
            if seen.is_empty() {
                seen.push((tag, text, figures));
            } else {
                assert_ne!(seen[0].1, text, "{tag}");
                assert_eq!(seen[0].2, figures, "{tag}");
            }
        }
    }

    #[test]
    fn a_bad_option_a_missing_value_or_a_line_the_model_refuses_is_exit_2_with_its_code() {
        let (code, out, err) = run(&[]);
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(err.starts_with("phoenix spread "), "{err}");
        for extra in [
            vec!["--width-um", "0"],
            vec!["--width-um", "x"],
            vec!["--er", "0.5"],
            vec!["--kind", "coplanar"],
            vec!["--lang", "xx"],
            vec!["--tol-er", "-1"],
            vec!["--percent", "10"],
            vec!["--max-width-tolerance"],
            vec!["--bogus"],
            vec!["--target"],
        ] {
            let (code, out, err) = call(&extra);
            assert_eq!((code, out.as_str()), (2, ""), "{extra:?}");
            assert!(err.contains("PHOENIX-SPREAD-002"), "{extra:?}: {err}");
        }
        // an option that is not given
        let (code, _, err) = run(&["--kind", "stripline", "--width-um", "200"]);
        assert_eq!(code, 2);
        assert!(err.contains("--height-um is needed"), "{err}");
        // a tolerance of more than half of the width is refused by the model
        let (code, out, err) = call(&["--tol-width-um", "200"]);
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(err.contains("PHOENIX-SPREAD-003"), "{err}");
        // a settings file that does not exist
        let (code, _, err) = call(&["--config", "no-such-file.cfg"]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-SPREAD-001"), "{err}");
    }

    #[test]
    fn every_code_has_a_title_in_each_language_and_is_documented() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SPREAD-{:03}", i + 1));
            for text in row {
                assert!(!text.is_empty());
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
        assert_eq!(title("PHOENIX-SPREAD-999", Language::En), "");
        assert_eq!(
            title(NOT_COMPUTABLE, Language::Pt),
            "Não é possível calcular a linha com estes valores"
        );
    }

    /// A generator with a fixed seed, so every run makes the same cases.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn unit(&mut self) -> f64 {
            (self.next() % 10_000) as f64 / 10_000.0
        }
    }

    #[test]
    fn random_lines_keep_the_ends_around_the_nominal_and_a_larger_tolerance_never_narrows_them() {
        let mut rng = Lcg(0x5d_0001);
        let mut computed = 0;
        for _ in 0..150 {
            let kind = if rng.next() & 1 == 0 {
                "microstrip"
            } else {
                "stripline"
            };
            let width = 100.0 + rng.unit() * 400.0;
            let height = 100.0 + rng.unit() * 300.0;
            let er = 2.5 + rng.unit() * 3.0;
            let tol = rng.unit() * 20.0;
            let text = |t: f64| {
                let args: Vec<String> = [
                    "--kind".to_owned(),
                    kind.to_owned(),
                    "--width-um".to_owned(),
                    format!("{width:.3}"),
                    "--height-um".to_owned(),
                    format!("{height:.3}"),
                    "--er".to_owned(),
                    format!("{er:.3}"),
                    "--tol-width-um".to_owned(),
                    format!("{t:.3}"),
                    "--tol-er".to_owned(),
                    "0.1".to_owned(),
                ]
                .to_vec();
                let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let request = parse_args(&refs, &Defaults::default()).unwrap();
                compute(&request)
            };
            let (Ok(small), Ok(large)) = (text(tol), text(tol + 5.0)) else {
                continue;
            };
            computed += 1;
            let (a, b) = (small.spread, large.spread);
            assert!(a.min_ohm <= a.nominal_ohm && a.nominal_ohm <= a.max_ohm);
            assert!(b.min_ohm <= a.min_ohm + 1.0e-9 && b.max_ohm >= a.max_ohm - 1.0e-9);
            assert!((a.nominal_ohm - b.nominal_ohm).abs() < 1.0e-9);
        }
        assert!(computed > 100, "{computed}");
    }
}
