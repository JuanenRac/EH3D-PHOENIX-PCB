// SPDX-License-Identifier: GPL-3.0-or-later
//! The `.step` of a study: one parameter, element value or source value that takes a list of
//! values, one run of the study for each.
//!
//! ```text
//! .step [param] TARGET lin START STOP STEP
//! .step [param] TARGET dec POINTS START STOP        (POINTS per decade)
//! .step [param] TARGET oct POINTS START STOP        (POINTS per octave)
//! .step [param] TARGET list V1 V2 V3 ...
//! ```
//!
//! With the word `param` the target is a `.param` of the netlist, which is read again with the
//! new value (so every expression that uses it follows). Without it the target is an element of
//! the top level: the value of a resistor, a capacitor or an inductor, the DC value of an
//! independent source (a waveform is not changed), or the gain of a controlled source. Several
//! `.step` lines make a grid: the first is the outermost.

use crate::codes::{SimError, LIMIT_EXCEEDED, STEP_INVALID, UNDEFINED, UNSUPPORTED};
use crate::netlist::{Kind, Netlist};
use crate::units::parse_number;

/// What a step changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A `.param` of the netlist, by name in lower case.
    Param(String),
    /// An element of the top level, by name in lower case.
    Element(String),
}

impl Target {
    /// How the target is written in a table: `param:name` or the name of the element.
    pub fn label(&self) -> String {
        match self {
            Target::Param(name) => format!("param:{name}"),
            Target::Element(name) => name.clone(),
        }
    }
}

/// How the values are spaced.
#[derive(Debug, Clone, PartialEq)]
pub enum Spacing {
    /// `start`, `start + step`, ... up to `stop`.
    Linear {
        /// The first value.
        start: f64,
        /// The last value.
        stop: f64,
        /// The increment (negative to go down).
        step: f64,
    },
    /// `points` values per decade from `start` to `stop`.
    Decade {
        /// The values per decade.
        points: usize,
        /// The first value (above zero).
        start: f64,
        /// The last value.
        stop: f64,
    },
    /// `points` values per octave from `start` to `stop`.
    Octave {
        /// The values per octave.
        points: usize,
        /// The first value (above zero).
        start: f64,
        /// The last value.
        stop: f64,
    },
    /// The values given.
    List(Vec<f64>),
}

/// One `.step`.
#[derive(Debug, Clone, PartialEq)]
pub struct StepSpec {
    /// What changes.
    pub target: Target,
    /// The values.
    pub spacing: Spacing,
    /// The line as written, with single spaces; the writer of netlists prints it after `.step`.
    pub text: String,
}

fn error(line: Option<usize>, why: String) -> SimError {
    match line {
        Some(number) => SimError::at(STEP_INVALID, number, why),
        None => SimError::new(STEP_INVALID, why),
    }
}

/// Reads a target given as `NAME` or `param:NAME`.
pub fn parse_target(word: &str) -> Result<Target, SimError> {
    let lower = word.trim().to_ascii_lowercase();
    let (is_param, name) = match lower.strip_prefix("param:") {
        Some(rest) => (true, rest.to_string()),
        None => (false, lower.clone()),
    };
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$');
    if !valid {
        return Err(error(None, format!("'{}' is not a name", word.trim())));
    }
    if is_param {
        Ok(Target::Param(name))
    } else {
        Ok(Target::Element(name))
    }
}

/// Reads a `.step` from its text (without `.step`). `line` is the line of the netlist when there
/// is one.
pub fn parse_step(text: &str, line: Option<usize>) -> Result<StepSpec, SimError> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let normalised = words.join(" ");
    let (is_param, rest) = match words.first() {
        Some(first) if first.eq_ignore_ascii_case("param") => (true, &words[1..]),
        _ => (false, &words[..]),
    };
    let Some(name_word) = rest.first() else {
        return Err(error(
            line,
            ".step needs a target (an element, a source or `param NAME`) and its values"
                .to_string(),
        ));
    };
    let target = match parse_target(name_word) {
        Ok(Target::Element(name)) if is_param => Target::Param(name),
        Ok(Target::Param(_)) if is_param => {
            return Err(error(
                line,
                format!(
                    ".step: write the name of the parameter alone after `param`, not '{name_word}'"
                ),
            ))
        }
        Ok(target) => target,
        Err(e) => return Err(error(line, format!(".step: {}", e.detail))),
    };
    let args = &rest[1..];
    let Some(kind) = args.first() else {
        return Err(error(
            line,
            ".step: the values are missing (lin, dec, oct or list)".to_string(),
        ));
    };
    let values = &args[1..];
    let num = |word: &str| -> Result<f64, SimError> {
        parse_number(word).ok_or_else(|| error(line, format!(".step: '{word}' is not a number")))
    };
    let points = |word: &str| -> Result<usize, SimError> {
        let n = num(word)?;
        if n >= 1.0 && n.fract() == 0.0 && n <= 1e9 {
            Ok(n as usize)
        } else {
            Err(error(
                line,
                format!(".step: '{word}' is not a whole number of points above zero"),
            ))
        }
    };
    let spacing = match kind.to_ascii_lowercase().as_str() {
        "lin" => {
            if values.len() != 3 {
                return Err(error(
                    line,
                    ".step lin needs a start, a stop and a step".to_string(),
                ));
            }
            let (start, stop, step) = (num(values[0])?, num(values[1])?, num(values[2])?);
            if step == 0.0 {
                return Err(error(
                    line,
                    ".step lin: the step cannot be zero".to_string(),
                ));
            }
            if (stop - start) / step < 0.0 {
                return Err(error(
                    line,
                    ".step lin: the step goes away from the stop value".to_string(),
                ));
            }
            Spacing::Linear { start, stop, step }
        }
        word @ ("dec" | "oct") => {
            if values.len() != 3 {
                return Err(error(
                    line,
                    format!(".step {word} needs the points, a start and a stop"),
                ));
            }
            let (count, start, stop) = (points(values[0])?, num(values[1])?, num(values[2])?);
            if start <= 0.0 || stop < start {
                return Err(error(
                    line,
                    format!(".step {word}: the values must satisfy 0 < start <= stop"),
                ));
            }
            if word == "dec" {
                Spacing::Decade {
                    points: count,
                    start,
                    stop,
                }
            } else {
                Spacing::Octave {
                    points: count,
                    start,
                    stop,
                }
            }
        }
        "list" => {
            if values.is_empty() {
                return Err(error(
                    line,
                    ".step list needs at least one value".to_string(),
                ));
            }
            let mut list = Vec::new();
            for word in values {
                list.push(num(word)?);
            }
            Spacing::List(list)
        }
        other => {
            return Err(error(
                line,
                format!(".step: '{other}' is not lin, dec, oct or list"),
            ))
        }
    };
    Ok(StepSpec {
        target,
        spacing,
        text: normalised,
    })
}

impl StepSpec {
    /// The values, at most `max_points`.
    pub fn values(&self, max_points: usize) -> Result<Vec<f64>, SimError> {
        let too_many = |count: f64| {
            SimError::new(
                LIMIT_EXCEEDED,
                format!(
                    ".step {}: {count:.0} values are more than max_step_points ({max_points})",
                    self.target.label()
                ),
            )
        };
        match &self.spacing {
            Spacing::Linear { start, stop, step } => {
                let count = ((stop - start) / step + 1e-9).floor() + 1.0;
                if count > max_points as f64 {
                    return Err(too_many(count));
                }
                Ok((0..count as usize)
                    .map(|k| start + k as f64 * step)
                    .collect())
            }
            Spacing::Decade {
                points,
                start,
                stop,
            }
            | Spacing::Octave {
                points,
                start,
                stop,
            } => {
                let decade = matches!(self.spacing, Spacing::Decade { .. });
                let per = *points as f64;
                let (base, span) = if decade {
                    (10.0f64, (stop / start).log10())
                } else {
                    (2.0f64, (stop / start).log2())
                };
                let count = (per * span + 1e-9).floor();
                if count + 2.0 > max_points as f64 {
                    return Err(too_many(count + 1.0));
                }
                let mut out: Vec<f64> = (0..=(count as usize))
                    .map(|k| start * base.powf(k as f64 / per))
                    .collect();
                if out.last().is_some_and(|last| *last < stop * (1.0 - 1e-9)) {
                    out.push(*stop);
                }
                Ok(out)
            }
            Spacing::List(list) => {
                if list.len() > max_points {
                    return Err(too_many(list.len() as f64));
                }
                Ok(list.clone())
            }
        }
    }
}

/// The value of an element that a step can change, by name (any case).
pub fn element_value(netlist: &Netlist, name: &str) -> Option<f64> {
    let element = netlist
        .elements
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(name))?;
    match &element.kind {
        Kind::Resistor(v) | Kind::Vcvs(v) | Kind::Vccs(v) => Some(*v),
        Kind::Capacitor { value, .. } | Kind::Inductor { value, .. } => Some(*value),
        Kind::Voltage(source) | Kind::Current(source) => Some(source.dc_value()),
        _ => None,
    }
}

/// Sets the value of an element of the top level.
pub fn set_element(netlist: &mut Netlist, name: &str, value: f64) -> Result<(), SimError> {
    let Some(element) = netlist
        .elements
        .iter_mut()
        .find(|e| e.name.eq_ignore_ascii_case(name))
    else {
        return Err(SimError::new(
            UNDEFINED,
            format!(".step: {name} is not an element of the netlist"),
        ));
    };
    let bad = |why: &str| SimError::new(STEP_INVALID, format!(".step {name}: {why}"));
    if !value.is_finite() {
        return Err(bad("the value is not a finite number"));
    }
    match &mut element.kind {
        Kind::Resistor(v) => {
            if value <= 0.0 {
                return Err(bad("a resistance must be above zero"));
            }
            *v = value;
        }
        Kind::Inductor { value: v, .. } => {
            if value <= 0.0 {
                return Err(bad("an inductance must be above zero"));
            }
            *v = value;
        }
        Kind::Capacitor { value: v, .. } => {
            if value < 0.0 {
                return Err(bad("a capacitance cannot be negative"));
            }
            *v = value;
        }
        Kind::Vcvs(v) | Kind::Vccs(v) => *v = value,
        Kind::Voltage(source) | Kind::Current(source) => source.dc = Some(value),
        _ => {
            return Err(SimError::new(
                UNSUPPORTED,
                format!(
                    ".step {name}: only a resistor, a capacitor, an inductor, an independent source or a controlled source of voltage can be stepped"
                ),
            ))
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::settings::SimSettings;

    fn step(text: &str) -> StepSpec {
        parse_step(text, None).unwrap()
    }

    #[test]
    fn the_four_spacings_are_read() {
        assert_eq!(
            step("R1 lin 1k 3k 1k").spacing,
            Spacing::Linear {
                start: 1e3,
                stop: 3e3,
                step: 1e3
            }
        );
        assert_eq!(step("R1 lin 1k 3k 1k").target, Target::Element("r1".into()));
        let d = step("param RL dec 10 1 1000");
        assert_eq!(d.target, Target::Param("rl".into()));
        assert_eq!(
            d.spacing,
            Spacing::Decade {
                points: 10,
                start: 1.0,
                stop: 1000.0
            }
        );
        assert_eq!(
            step("c1 oct 2 1n 4n").spacing,
            Spacing::Octave {
                points: 2,
                start: 1e-9,
                stop: 4e-9
            }
        );
        assert_eq!(
            step("V1  list 1 2.5   5").spacing,
            Spacing::List(vec![1.0, 2.5, 5.0])
        );
        assert_eq!(step("V1  list 1 2.5   5").text, "V1 list 1 2.5 5");
    }

    #[test]
    fn a_step_that_is_not_valid_is_refused_with_its_line() {
        for text in [
            "",
            "param",
            "R1",
            "R1 lin 1 2",
            "R1 lin 1 2 0",
            "R1 lin 1 2 -1",
            "R1 lin a 2 1",
            "R1 dec 0 1 10",
            "R1 dec 2.5 1 10",
            "R1 dec 10 0 10",
            "R1 oct 2 10 1",
            "R1 list",
            "R1 geometric 1 2 3",
            "param param:x list 1",
            "R(1) list 1",
        ] {
            let e = parse_step(text, Some(4)).unwrap_err();
            assert_eq!(e.code, STEP_INVALID, "{text}");
            assert_eq!(e.line, Some(4), "{text}");
        }
    }

    #[test]
    fn the_values_of_each_spacing() {
        assert_eq!(
            step("R1 lin 1k 3k 1k").values(10).unwrap(),
            vec![1e3, 2e3, 3e3]
        );
        assert_eq!(
            step("R1 lin 3 1 -1").values(10).unwrap(),
            vec![3.0, 2.0, 1.0]
        );
        let d = step("R1 dec 2 1 100").values(100).unwrap();
        assert_eq!(d.len(), 5);
        assert!((d[1] - 10f64.sqrt()).abs() < 1e-12);
        assert!((d[4] - 100.0).abs() < 1e-9);
        let o = step("R1 oct 1 1 8").values(100).unwrap();
        assert_eq!(o.len(), 4);
        assert!((o[3] - 8.0).abs() < 1e-9);
        // a stop that is not on the grid is added at the end
        let e = step("R1 dec 1 1 30").values(100).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(e[2], 30.0);
        assert_eq!(step("R1 list 5 1").values(10).unwrap(), vec![5.0, 1.0]);
    }

    #[test]
    fn too_many_values_are_refused() {
        assert_eq!(
            step("R1 lin 1 100 1").values(10).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        assert_eq!(
            step("R1 dec 100 1 1000").values(10).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        assert_eq!(
            step("R1 list 1 2 3").values(2).unwrap_err().code,
            LIMIT_EXCEEDED
        );
    }

    #[test]
    fn targets_are_names_or_param_names() {
        assert_eq!(parse_target("R1").unwrap(), Target::Element("r1".into()));
        assert_eq!(
            parse_target("param:Load").unwrap(),
            Target::Param("load".into())
        );
        assert_eq!(parse_target("param:Load").unwrap().label(), "param:load");
        assert_eq!(parse_target("").unwrap_err().code, STEP_INVALID);
        assert_eq!(parse_target("a b").unwrap_err().code, STEP_INVALID);
    }

    #[test]
    fn elements_are_set_and_read_back() {
        let s = SimSettings::default();
        let mut nl = parse(
            "t\nV1 a 0 DC 1\nR1 a b 1k\nC1 b 0 1u\nL1 b c 1m\nE1 c 0 a 0 2\nI1 0 b 1m\nD1 b 0 dm\n.model dm D\n",
            &s,
        )
        .unwrap();
        for (name, value) in [
            ("R1", 2e3),
            ("c1", 2e-6),
            ("L1", 3e-3),
            ("V1", 5.0),
            ("E1", 4.0),
            ("I1", 2e-3),
        ] {
            set_element(&mut nl, name, value).unwrap();
            assert_eq!(element_value(&nl, name), Some(value), "{name}");
        }
        assert_eq!(
            set_element(&mut nl, "R1", 0.0).unwrap_err().code,
            STEP_INVALID
        );
        assert_eq!(
            set_element(&mut nl, "C1", -1.0).unwrap_err().code,
            STEP_INVALID
        );
        assert_eq!(
            set_element(&mut nl, "R1", f64::NAN).unwrap_err().code,
            STEP_INVALID
        );
        assert_eq!(
            set_element(&mut nl, "D1", 1.0).unwrap_err().code,
            UNSUPPORTED
        );
        assert_eq!(set_element(&mut nl, "R9", 1.0).unwrap_err().code, UNDEFINED);
        assert_eq!(element_value(&nl, "D1"), None);
        assert_eq!(element_value(&nl, "R9"), None);
    }
}
