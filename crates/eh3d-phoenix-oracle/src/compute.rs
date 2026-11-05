// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks: each name in a case file is one call of the real functions of the suite, with the
//! inputs of the case, that gives one number.

use serde_json::{Map, Value};

use crate::calc_board;
use crate::calc_line;
use crate::calc_sim;
use crate::case::Case;
use crate::codes::{OracleError, CALC_FAILED, CHECK_UNKNOWN, INPUT_INVALID};

/// The inputs of one case, read by name.
pub(crate) struct Inputs<'a> {
    id: &'a str,
    map: &'a Map<String, Value>,
}

impl<'a> Inputs<'a> {
    fn new(case: &'a Case) -> Inputs<'a> {
        Inputs {
            id: case.id.as_str(),
            map: &case.inputs,
        }
    }

    fn bad(&self, key: &str, what: &str) -> OracleError {
        OracleError::new(INPUT_INVALID, format!("{}: {key} {what}", self.id))
    }

    /// A finite number.
    pub(crate) fn num(&self, key: &str) -> Result<f64, OracleError> {
        let value = self
            .map
            .get(key)
            .ok_or_else(|| self.bad(key, "is missing"))?;
        value
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| self.bad(key, "is not a finite number"))
    }

    /// A text.
    pub(crate) fn text(&self, key: &str) -> Result<&'a str, OracleError> {
        let value = self
            .map
            .get(key)
            .ok_or_else(|| self.bad(key, "is missing"))?;
        value.as_str().ok_or_else(|| self.bad(key, "is not a text"))
    }

    /// A list of texts joined into lines, each ended by a line break.
    pub(crate) fn lines(&self, key: &str) -> Result<String, OracleError> {
        let value = self
            .map
            .get(key)
            .ok_or_else(|| self.bad(key, "is missing"))?;
        let list = value
            .as_array()
            .ok_or_else(|| self.bad(key, "is not a list of lines"))?;
        let mut out = String::new();
        for item in list {
            let line = item
                .as_str()
                .ok_or_else(|| self.bad(key, "has a line that is not a text"))?;
            out.push_str(line);
            out.push('\n');
        }
        Ok(out)
    }

    /// A different value of a text option is refused with its name.
    pub(crate) fn unknown(&self, key: &str, got: &str) -> OracleError {
        self.bad(key, &format!("has the value {got}, which is not known"))
    }
}

/// The error of a calculation under test.
pub(crate) fn calc(error: impl std::fmt::Display) -> OracleError {
    OracleError::new(CALC_FAILED, error.to_string())
}

/// Run the check of a case and give the number it computes.
pub fn compute(case: &Case) -> Result<f64, OracleError> {
    let inputs = Inputs::new(case);
    let check = case.check.as_str();
    let found = if let Some(value) = calc_line::run(check, &inputs)? {
        Some(value)
    } else if let Some(value) = calc_sim::run(check, &inputs)? {
        Some(value)
    } else {
        calc_board::run(check, &inputs)?
    };
    match found {
        Some(value) if value.is_finite() => Ok(value),
        Some(_) => Err(OracleError::new(
            CALC_FAILED,
            format!("{}: the result is not a finite number", case.id),
        )),
        None => Err(OracleError::new(
            CHECK_UNKNOWN,
            format!("{}: {check}", case.id),
        )),
    }
}

/// The names of all the checks, sorted.
pub const CHECKS: &[&str] = &[
    "const_c0",
    "const_eta0",
    "const_mu0",
    "differential_microstrip",
    "differential_stripline",
    "elliptic_k",
    "lengths_track",
    "line_quantity",
    "microstrip_eps",
    "microstrip_z0",
    "rf_line_input_impedance_im",
    "rf_line_input_impedance_re",
    "rf_load_mismatch_loss_db",
    "rf_load_return_loss_db",
    "rf_load_vswr",
    "rf_quarter_wave_impedance",
    "rf_wavelength_m",
    "rules_clearance",
    "sim_ac",
    "sim_op",
    "sim_tran",
    "stripline_z0",
    "stripline_zero_exact",
    "zones_fill_area_mm2",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::{Status, Tolerance};

    fn case(check: &str, inputs: &str) -> Case {
        Case {
            id: "ORACLE-T-001".to_string(),
            title: "t".to_string(),
            area: "stackup".to_string(),
            check: check.to_string(),
            inputs: serde_json::from_str(inputs).unwrap(),
            expected: Some(1.0),
            unit: String::new(),
            tolerance: Some(Tolerance {
                abs: Some(1.0),
                rel: None,
            }),
            tolerance_note: "n".to_string(),
            source: "s".to_string(),
            status: Status::Cited,
            reason: String::new(),
        }
    }

    #[test]
    fn an_unknown_check_is_refused_by_name() {
        let error = compute(&case("no_such_check", "{}")).unwrap_err();
        assert_eq!(error.code, CHECK_UNKNOWN);
        assert!(error.detail.contains("no_such_check"));
    }

    #[test]
    fn a_missing_or_wrong_input_is_refused_by_name() {
        let missing = compute(&case("elliptic_k", "{}")).unwrap_err();
        assert_eq!(missing.code, INPUT_INVALID);
        assert!(missing.detail.contains("k is missing"));
        let wrong = compute(&case("elliptic_k", r#"{"k":"half"}"#)).unwrap_err();
        assert_eq!(wrong.code, INPUT_INVALID);
    }

    #[test]
    fn the_constants_come_out_of_the_models() {
        let c = compute(&case("const_c0", "{}")).unwrap();
        assert!((c - 299_792_458.0).abs() < 1e-6);
    }

    #[test]
    fn a_result_that_is_not_a_number_is_a_failure_of_the_calculation() {
        let error = compute(&case("elliptic_k", r#"{"k":1.5}"#)).unwrap_err();
        assert_eq!(error.code, CALC_FAILED);
    }

    #[test]
    fn every_listed_check_is_known_and_the_list_is_sorted() {
        for pair in CHECKS.windows(2) {
            assert!(pair[0] < pair[1], "{} {}", pair[0], pair[1]);
        }
        for name in CHECKS {
            // an empty input gives an error about the input, never about an unknown check
            let error = compute(&case(name, "{}"));
            if let Err(e) = error {
                assert_ne!(e.code, CHECK_UNKNOWN, "{name}");
            }
        }
    }
}
