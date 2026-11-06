// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on the netlist made from a schematic: a small schematic is written in the case as a
//! few lines (`schematic`, the sketch of the simulator crate), the rules of the table of models
//! that it needs are written beside it (`models`), the netlist is made from them, and one number
//! is taken from the solution of the circuit.

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sim::{
    ac_at, generate_netlist, operating_point, read_generated, schematic_from_sketch, Circuit,
    ModelTable, SimSettings,
};

use crate::codes::{OracleError, CALC_FAILED};
use crate::compute::{calc, Inputs};

/// The circuit of the schematic and the table of the inputs. A report with an error is a failure
/// of the calculation under test.
fn circuit(i: &Inputs, settings: &SimSettings) -> Result<Circuit, OracleError> {
    let sketch = i.lines("schematic")?;
    let schematic = schematic_from_sketch(&sketch).map_err(calc)?;
    let table = ModelTable::parse(&i.lines("models")?).map_err(|e| calc(e.message(Lang::En)))?;
    let generated = generate_netlist(&schematic, &table, None, settings);
    if generated.report.has_errors() {
        return Err(OracleError::new(
            CALC_FAILED,
            generated.report.to_text(Lang::En),
        ));
    }
    let netlist = read_generated(&generated, None, settings).map_err(calc)?;
    Circuit::compile(&netlist, settings).map_err(calc)
}

fn missing(label: &str) -> OracleError {
    OracleError::new(CALC_FAILED, format!("the result has no unknown {label}"))
}

fn operating(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let result = operating_point(&circuit, &settings).map_err(calc)?;
    result.get(label).ok_or_else(|| missing(label))
}

fn small_signal(i: &Inputs) -> Result<f64, OracleError> {
    let settings = SimSettings::default();
    let circuit = circuit(i, &settings)?;
    let label = i.text("probe")?;
    let result = ac_at(&circuit, &settings, &[i.num("freq_hz")?]).map_err(calc)?;
    let series = result.series(label).ok_or_else(|| missing(label))?;
    let point = series.first().copied().ok_or_else(|| missing(label))?;
    match i.text("quantity")? {
        "magnitude" => Ok(point.abs()),
        "phase_deg" => Ok(point.arg_deg()),
        other => Err(i.unknown("quantity", other)),
    }
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "sim_schematic_op" => operating(i)?,
        "sim_schematic_ac" => small_signal(i)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::{Case, Status, Tolerance};
    use crate::compute::compute;

    fn case(inputs: &str) -> Case {
        Case {
            id: "ORACLE-T-001".to_string(),
            title: "t".to_string(),
            area: "sim".to_string(),
            check: "sim_schematic_op".to_string(),
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
    fn a_sketch_that_cannot_be_read_is_a_failure_of_the_calculation() {
        let error = compute(&case(
            r#"{"schematic":["bogus line"],"models":[],"probe":"v(out)"}"#,
        ))
        .unwrap_err();
        assert_eq!(error.code, CALC_FAILED);
        assert!(error.detail.contains("line 1"), "{}", error.detail);
    }

    #[test]
    fn a_report_with_an_error_stops_the_check_before_the_solver() {
        let error = compute(&case(
            r#"{"schematic":["symbol P 1 2","part R1 P 1k","net A R1.1","net B R1.2"],"models":["refdes R = resistor"],"probe":"v(a)"}"#,
        ))
        .unwrap_err();
        assert_eq!(error.code, CALC_FAILED);
        assert!(
            error.detail.contains("PHOENIX-NETLIST-009"),
            "{}",
            error.detail
        );
    }

    #[test]
    fn a_divider_gives_its_voltage() {
        let value = compute(&case(
            r#"{"schematic":["symbol P 1 2","part V1 P 10","part R1 P 1k","part R2 P 1k","net IN V1.1 R1.1","net OUT R1.2 R2.1","net GND V1.2 R2.2"],"models":["refdes R = resistor","refdes V = vsource"],"probe":"v(out)"}"#,
        ))
        .unwrap();
        assert!((value - 5.0).abs() < 1e-9, "{value}");
    }
}
