// SPDX-License-Identifier: GPL-3.0-or-later
//! The spread of the impedance of a line, and the largest tolerance of its width, as text in the
//! eight languages of the suite.
//!
//! [`spread_text`] writes the nominal, the lowest and the highest impedance of a
//! [`crate::spread::Spread`] with the distance of the two ends from the nominal one in per cent,
//! and, when a target and a per cent are given, whether the whole box of the tolerances fits.
//! [`max_width_tolerance_text`] writes the answer of [`crate::spread::max_width_tolerance`].
//! The numbers are written the same in every language (a point and two decimals); only the words
//! change. The text holds no date, host name or user name.

use crate::error::Language;
use crate::spread::Spread;

/// The words of the report in one language.
struct Words {
    nominal: &'static str,
    lowest: &'static str,
    highest: &'static str,
    fits: &'static str,
    yes: &'static str,
    no: &'static str,
    largest: &'static str,
    none: &'static str,
    every: &'static str,
}

fn words(language: Language) -> Words {
    match language {
        Language::En => Words {
            nominal: "nominal impedance",
            lowest: "lowest impedance",
            highest: "highest impedance",
            fits: "within {p} % of {t} ohm",
            yes: "yes",
            no: "no",
            largest: "largest tolerance of the width",
            none: "none: even a width with no tolerance does not fit",
            every: "every tolerance that can be asked fits",
        },
        Language::Es => Words {
            nominal: "impedancia nominal",
            lowest: "impedancia más baja",
            highest: "impedancia más alta",
            fits: "dentro del {p} % de {t} ohmios",
            yes: "sí",
            no: "no",
            largest: "mayor tolerancia del ancho",
            none: "ninguna: ni un ancho sin tolerancia cumple",
            every: "cumple cualquier tolerancia que se pueda pedir",
        },
        Language::De => Words {
            nominal: "Nennimpedanz",
            lowest: "niedrigste Impedanz",
            highest: "höchste Impedanz",
            fits: "innerhalb von {p} % von {t} Ohm",
            yes: "ja",
            no: "nein",
            largest: "größte Toleranz der Breite",
            none: "keine: schon eine Breite ohne Toleranz passt nicht",
            every: "jede Toleranz, die verlangt werden kann, passt",
        },
        Language::Fr => Words {
            nominal: "impédance nominale",
            lowest: "impédance la plus basse",
            highest: "impédance la plus haute",
            fits: "dans {p} % de {t} ohms",
            yes: "oui",
            no: "non",
            largest: "plus grande tolérance de la largeur",
            none: "aucune : même une largeur sans tolérance ne convient pas",
            every: "toute tolérance que l'on peut demander convient",
        },
        Language::It => Words {
            nominal: "impedenza nominale",
            lowest: "impedenza più bassa",
            highest: "impedenza più alta",
            fits: "entro il {p} % di {t} ohm",
            yes: "sì",
            no: "no",
            largest: "tolleranza massima della larghezza",
            none: "nessuna: neppure una larghezza senza tolleranza va bene",
            every: "va bene qualsiasi tolleranza che si possa chiedere",
        },
        Language::Ja => Words {
            nominal: "公称インピーダンス",
            lowest: "最低インピーダンス",
            highest: "最高インピーダンス",
            fits: "{t} ohm の {p} % 以内",
            yes: "はい",
            no: "いいえ",
            largest: "幅の最大許容差",
            none: "なし: 許容差のない幅でも収まりません",
            every: "指定できるどの許容差でも収まります",
        },
        Language::Zh => Words {
            nominal: "标称阻抗",
            lowest: "最低阻抗",
            highest: "最高阻抗",
            fits: "在 {t} 欧姆的 {p} % 以内",
            yes: "是",
            no: "否",
            largest: "线宽的最大公差",
            none: "无: 即使线宽没有公差也不满足",
            every: "任何可以提出的公差都满足",
        },
        Language::Pt => Words {
            nominal: "impedância nominal",
            lowest: "impedância mais baixa",
            highest: "impedância mais alta",
            fits: "dentro de {p} % de {t} ohms",
            yes: "sim",
            no: "não",
            largest: "maior tolerância da largura",
            none: "nenhuma: nem uma largura sem tolerância serve",
            every: "qualquer tolerância que se possa pedir serve",
        },
    }
}

/// The spread as text, one line for each figure; with `check` (a target in ohm and a per cent)
/// a last line says whether the whole box of the tolerances is within that per cent of the target.
pub fn spread_text(spread: &Spread, check: Option<(f64, f64)>, language: Language) -> String {
    let w = words(language);
    let mut out = format!(
        "{}: {:.2} ohm\n{}: {:.2} ohm (-{:.2} %)\n{}: {:.2} ohm (+{:.2} %)\n",
        w.nominal,
        spread.nominal_ohm,
        w.lowest,
        spread.min_ohm,
        spread.minus_percent(),
        w.highest,
        spread.max_ohm,
        spread.plus_percent()
    );
    if let Some((target, percent)) = check {
        let which = if spread.fits(target, percent) {
            w.yes
        } else {
            w.no
        };
        let label = w
            .fits
            .replace("{p}", &format!("{percent:.2}"))
            .replace("{t}", &format!("{target:.2}"));
        out.push_str(&format!("{label}: {which}\n"));
    }
    out
}

/// The answer of `max_width_tolerance` as text: the tolerance in micrometres, `none` when even a
/// width with no tolerance does not fit, and a sentence when every tolerance that can be asked
/// (half of the width) fits. `half_width_m` is half of the nominal width in metres.
pub fn max_width_tolerance_text(
    answer: Option<f64>,
    half_width_m: f64,
    language: Language,
) -> String {
    let w = words(language);
    match answer {
        None => format!("{}: {}\n", w.largest, w.none),
        Some(limit) if limit >= half_width_m => {
            format!("{}: {:.2} um ({})\n", w.largest, limit * 1.0e6, w.every)
        }
        Some(limit) => format!("{}: {:.2} um\n", w.largest, limit * 1.0e6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Spread {
        Spread {
            nominal_ohm: 50.0,
            min_ohm: 46.0,
            max_ohm: 53.5,
        }
    }

    #[test]
    fn the_figures_are_the_same_in_every_language() {
        let mut texts = std::collections::BTreeSet::new();
        for language in Language::ALL {
            let text = spread_text(&sample(), Some((50.0, 10.0)), language);
            assert!(text.contains("50.00 ohm"), "{language:?}");
            assert!(text.contains("46.00 ohm (-8.00 %)"), "{language:?}");
            assert!(text.contains("53.50 ohm (+7.00 %)"), "{language:?}");
            assert_eq!(text.lines().count(), 4, "{language:?}");
            assert!(text.contains("10.00") && text.ends_with('\n'));
            assert!(texts.insert(text), "{language:?} repeats another language");
        }
        assert_eq!(texts.len(), 8);
    }

    #[test]
    fn the_last_line_says_whether_the_box_fits() {
        let fits = spread_text(&sample(), Some((50.0, 10.0)), Language::En);
        assert!(
            fits.ends_with("within 10.00 % of 50.00 ohm: yes\n"),
            "{fits}"
        );
        let misses = spread_text(&sample(), Some((50.0, 5.0)), Language::En);
        assert!(
            misses.ends_with("within 5.00 % of 50.00 ohm: no\n"),
            "{misses}"
        );
        let plain = spread_text(&sample(), None, Language::En);
        assert_eq!(plain.lines().count(), 3);
        assert!(fits.starts_with(&plain));
    }

    #[test]
    fn no_placeholder_is_left_in_any_language() {
        for language in Language::ALL {
            let text = spread_text(&sample(), Some((50.0, 10.0)), language);
            assert!(!text.contains('{') && !text.contains('}'), "{language:?}");
        }
    }

    #[test]
    fn the_tolerance_of_the_width_is_in_micrometres() {
        let found = max_width_tolerance_text(Some(23.456e-6), 150.0e-6, Language::En);
        assert_eq!(found, "largest tolerance of the width: 23.46 um\n");
        let none = max_width_tolerance_text(None, 150.0e-6, Language::En);
        assert!(none.contains("even a width with no tolerance does not fit"));
        let every = max_width_tolerance_text(Some(150.0e-6), 150.0e-6, Language::En);
        assert!(every.contains("150.00 um (every tolerance that can be asked fits)"));
    }

    #[test]
    fn the_three_answers_differ_in_every_language() {
        let mut seen = std::collections::BTreeSet::new();
        for language in Language::ALL {
            for answer in [None, Some(10.0e-6), Some(150.0e-6)] {
                let text = max_width_tolerance_text(answer, 150.0e-6, language);
                assert!(seen.insert(text), "{language:?} {answer:?}");
            }
        }
        assert_eq!(seen.len(), 24);
    }
}
