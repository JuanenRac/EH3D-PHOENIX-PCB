// SPDX-License-Identifier: GPL-3.0-or-later
//! The parameters of a structure: a map of names to texts, read with their units checked. A
//! number must be finite; a name that the type of structure does not know is refused, so that a
//! misspelt option never goes unnoticed.

use std::collections::BTreeMap;

use crate::codes::{RfgenError, PARAMETER_INVALID};

/// The description of one parameter of a type of structure, for `phoenix rfgen --list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSpec {
    /// The name, as it is written after `--` on the command line.
    pub name: &'static str,
    /// The unit, or an empty text.
    pub unit: &'static str,
    /// The default, in words (a number, or where it comes from).
    pub default: &'static str,
    /// What it is.
    pub help: &'static str,
}

/// The parameters given for one structure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params {
    values: BTreeMap<String, String>,
}

impl Params {
    /// No parameters.
    pub fn new() -> Params {
        Params::default()
    }

    /// Sets a parameter; a second value for the same name replaces the first.
    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_owned(), value.to_owned());
    }

    /// The same, in the style of a builder.
    pub fn with(mut self, name: &str, value: &str) -> Params {
        self.set(name, value);
        self
    }

    /// The names that were given, in order.
    pub fn names(&self) -> Vec<&str> {
        self.values.keys().map(String::as_str).collect()
    }

    /// Refuses a name that is not one of `allowed`.
    pub fn check_known(&self, allowed: &[ParamSpec]) -> Result<(), RfgenError> {
        for name in self.values.keys() {
            if !allowed.iter().any(|spec| spec.name == name) {
                return Err(RfgenError::new(
                    PARAMETER_INVALID,
                    format!("the parameter {name} is not used by this structure"),
                ));
            }
        }
        Ok(())
    }

    /// A number, when it was given.
    pub fn number(&self, name: &str) -> Result<Option<f64>, RfgenError> {
        match self.values.get(name) {
            None => Ok(None),
            Some(text) => match text.trim().parse::<f64>() {
                Ok(value) if value.is_finite() => Ok(Some(value)),
                _ => Err(RfgenError::new(
                    PARAMETER_INVALID,
                    format!("{name}: {text:?} is not a finite number"),
                )),
            },
        }
    }

    /// A number above zero, or `default` when it was not given.
    pub fn positive(&self, name: &str, default: f64) -> Result<f64, RfgenError> {
        let value = self.number(name)?.unwrap_or(default);
        if value > 0.0 {
            Ok(value)
        } else {
            Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("{name}: {value} must be above zero"),
            ))
        }
    }

    /// A number above zero that must be given.
    pub fn required(&self, name: &str) -> Result<f64, RfgenError> {
        match self.number(name)? {
            Some(value) if value > 0.0 => Ok(value),
            Some(value) => Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("{name}: {value} must be above zero"),
            )),
            None => Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("the parameter {name} is needed"),
            )),
        }
    }

    /// A number of zero or more, or `default` when it was not given.
    pub fn non_negative(&self, name: &str, default: f64) -> Result<f64, RfgenError> {
        let value = self.number(name)?.unwrap_or(default);
        if value >= 0.0 {
            Ok(value)
        } else {
            Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("{name}: {value} must be zero or more"),
            ))
        }
    }

    /// A whole number of at least `low`, or `default` when it was not given.
    pub fn whole(&self, name: &str, default: usize, low: usize) -> Result<usize, RfgenError> {
        let value = match self.values.get(name) {
            None => default,
            Some(text) => text.trim().parse::<usize>().map_err(|_| {
                RfgenError::new(
                    PARAMETER_INVALID,
                    format!("{name}: {text:?} is not a whole number"),
                )
            })?,
        };
        if value >= low {
            Ok(value)
        } else {
            Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("{name}: {value} must be at least {low}"),
            ))
        }
    }

    /// One of the `choices`, or `default` when it was not given.
    pub fn choice<'a>(
        &self,
        name: &str,
        choices: &[&'a str],
        default: &'a str,
    ) -> Result<&'a str, RfgenError> {
        match self.values.get(name) {
            None => Ok(default),
            Some(text) => choices
                .iter()
                .copied()
                .find(|c| *c == text.trim())
                .ok_or_else(|| {
                    RfgenError::new(
                        PARAMETER_INVALID,
                        format!("{name}: {text:?} is not one of {}", choices.join(", ")),
                    )
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: &[ParamSpec] = &[
        ParamSpec {
            name: "z0",
            unit: "ohm",
            default: "50",
            help: "impedance",
        },
        ParamSpec {
            name: "turn",
            unit: "",
            default: "left",
            help: "direction",
        },
    ];

    #[test]
    fn numbers_are_read_and_defaults_apply() {
        let p = Params::new().with("z0", "75");
        assert_eq!(p.number("z0").unwrap(), Some(75.0));
        assert_eq!(p.number("other").unwrap(), None);
        assert_eq!(p.positive("z0", 50.0).unwrap(), 75.0);
        assert_eq!(p.positive("other", 50.0).unwrap(), 50.0);
        assert_eq!(p.non_negative("other", 0.0).unwrap(), 0.0);
        assert_eq!(p.whole("order", 3, 1).unwrap(), 3);
    }

    #[test]
    fn a_bad_number_or_a_misspelt_name_is_refused() {
        for text in ["abc", "NaN", "inf", ""] {
            let p = Params::new().with("z0", text);
            assert_eq!(
                p.number("z0").unwrap_err().code,
                PARAMETER_INVALID,
                "{text}"
            );
        }
        assert!(Params::new().with("z0", "-5").positive("z0", 50.0).is_err());
        assert!(Params::new()
            .with("z0", "-5")
            .non_negative("z0", 1.0)
            .is_err());
        assert!(Params::new().required("z0").is_err());
        assert!(Params::new().with("z0", "0").required("z0").is_err());
        assert!(Params::new().with("n", "2.5").whole("n", 1, 1).is_err());
        assert!(Params::new().with("n", "0").whole("n", 1, 1).is_err());
        let p = Params::new().with("z0", "50").with("zo", "1");
        assert!(p.check_known(KNOWN).is_err());
        assert!(Params::new().with("z0", "50").check_known(KNOWN).is_ok());
    }

    #[test]
    fn a_choice_must_be_one_of_the_list() {
        let p = Params::new().with("turn", "right");
        assert_eq!(
            p.choice("turn", &["left", "right"], "left").unwrap(),
            "right"
        );
        assert_eq!(
            Params::new()
                .choice("turn", &["left", "right"], "left")
                .unwrap(),
            "left"
        );
        assert!(Params::new()
            .with("turn", "up")
            .choice("turn", &["left", "right"], "left")
            .is_err());
        assert_eq!(p.names(), ["turn"]);
    }
}
