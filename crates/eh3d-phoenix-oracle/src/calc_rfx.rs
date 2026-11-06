// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on the extended RF tools: matching networks with a chosen Q, filter prototypes and
//! their loss, the S parameters of a network written as text, the factors of stability and the
//! standard series of component values.

use eh3d_phoenix_rf::circles::stability_circles;
use eh3d_phoenix_rf::eseries::Series;
use eh3d_phoenix_rf::filter::{design, prototype, Approximation, Band, Position};
use eh3d_phoenix_rf::matching::l_match;
use eh3d_phoenix_rf::matchnet::{l_network_q, pi_match, PassKind};
use eh3d_phoenix_rf::netdesc::{figures, NetDesc};
use eh3d_phoenix_rf::network::{stability, Mat2};
use eh3d_phoenix_rf::{Complex, RfSettings};

use crate::codes::OracleError;
use crate::compute::{calc, Inputs};

/// The S matrix of the inputs, given as magnitude and angle in degrees: `s11_mag`, `s11_deg` and
/// so on.
fn s_matrix(i: &Inputs) -> Result<Mat2, OracleError> {
    let element = |name: &str| -> Result<Complex, OracleError> {
        let magnitude = i.num(&format!("{name}_mag"))?;
        let degrees = i.num_or(&format!("{name}_deg"), 0.0)?;
        Ok(Complex::from_polar(magnitude, degrees.to_radians()))
    };
    Ok(Mat2::new(
        element("s11")?,
        element("s12")?,
        element("s21")?,
        element("s22")?,
    ))
}

fn approximation(i: &Inputs) -> Result<Approximation, OracleError> {
    match i.text("family")? {
        "butterworth" => Ok(Approximation::Butterworth),
        "chebyshev" => Ok(Approximation::Chebyshev),
        other => Err(i.unknown("family", other)),
    }
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let settings = RfSettings::default();
    let value = match check {
        "rfx_l_network_q" => {
            let source = Complex::real(i.num("source_ohm")?);
            let load = Complex::real(i.num("load_ohm")?);
            let found = l_match(source, load, i.num("freq_hz")?, &settings).map_err(calc)?;
            let first = found
                .first()
                .ok_or_else(|| i.unknown("load_ohm", "no match"))?;
            l_network_q(first, source, load).map_err(calc)?
        }
        "rfx_pi_virtual_ohm" => {
            pi_match(
                Complex::real(i.num("source_ohm")?),
                Complex::real(i.num("load_ohm")?),
                i.num("freq_hz")?,
                i.num("q")?,
                PassKind::LowPass,
                &settings,
            )
            .map_err(calc)?
            .virtual_ohm
        }
        "rfx_prototype_g" => {
            let ripple = if i.text("family")? == "chebyshev" {
                Some(i.num("ripple_db")?)
            } else {
                None
            };
            let order = i.num("order")? as usize;
            let p = prototype(approximation(i)?, order, ripple, Position::Shunt, &settings)
                .map_err(calc)?;
            let index = i.num("index")? as usize;
            *p.g.get(index.wrapping_sub(1))
                .ok_or_else(|| i.unknown("index", "outside the prototype"))?
        }
        "rfx_filter_loss_db" => {
            let ripple = if i.text("family")? == "chebyshev" {
                Some(i.num("ripple_db")?)
            } else {
                None
            };
            let cutoff_hz = i.num("cutoff_hz")?;
            let p = prototype(
                approximation(i)?,
                i.num("order")? as usize,
                ripple,
                Position::Shunt,
                &settings,
            )
            .map_err(calc)?;
            let filter = design(&p, Band::LowPass { cutoff_hz }, 50.0, &settings).map_err(calc)?;
            filter
                .response_at(i.num("ratio")? * cutoff_hz)
                .map_err(calc)?
                .insertion_loss_db()
        }
        "rfx_net_vswr" | "rfx_net_s21_db" => {
            let net = NetDesc::parse(&i.lines("network")?, &settings).map_err(calc)?;
            let s = net.s_at(i.num("freq_hz")?).map_err(calc)?;
            let f = figures(&s);
            if check == "rfx_net_vswr" {
                f.vswr
            } else {
                f.s21_db
            }
        }
        "rfx_rollett_k" => stability(&s_matrix(i)?).k,
        "rfx_mu" => stability(&s_matrix(i)?).mu,
        "rfx_load_circle_radius" => {
            stability_circles(&s_matrix(i)?, settings.tolerance)
                .map_err(calc)?
                .load
                .radius
        }
        "rfx_e_series_nearest" => {
            let wanted = i.num("value")?;
            Series::from_number(i.num("series")? as usize)
                .and_then(|series| series.nearest(wanted))
                .map_err(calc)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use crate::case::{Case, Status, Tolerance};

    fn case(check: &str, inputs: &str) -> Case {
        Case {
            id: "ORACLE-T-001".to_string(),
            title: "t".to_string(),
            area: "rf".to_string(),
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

    fn value(check: &str, inputs: &str) -> f64 {
        crate::compute::compute(&case(check, inputs)).unwrap()
    }

    #[test]
    fn the_l_network_q_is_the_square_root_of_the_ratio_minus_one() {
        let q = value(
            "rfx_l_network_q",
            r#"{"source_ohm":50,"load_ohm":200,"freq_hz":1e8}"#,
        );
        assert!((q - 3.0f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn the_butterworth_prototype_has_the_closed_form() {
        let g = value(
            "rfx_prototype_g",
            r#"{"family":"butterworth","order":5,"index":2}"#,
        );
        assert!((g - 1.618_033_988_749_895).abs() < 1e-12);
    }

    #[test]
    fn the_third_order_filter_loses_the_closed_form_amount_at_twice_the_cutoff() {
        let loss = value(
            "rfx_filter_loss_db",
            r#"{"family":"butterworth","order":3,"cutoff_hz":1e8,"ratio":2.0}"#,
        );
        assert!((loss - 10.0 * 65.0f64.log10()).abs() < 1e-9, "{loss}");
    }

    #[test]
    fn a_network_written_as_text_gives_its_vswr() {
        let vswr = value(
            "rfx_net_vswr",
            r#"{"network":["series R 50"],"freq_hz":1e9}"#,
        );
        assert!((vswr - 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_factors_of_stability_are_the_hand_calculation() {
        let inputs = r#"{"s11_mag":0.5,"s12_mag":0.1,"s21_mag":2.0,"s22_mag":0.5}"#;
        assert!((value("rfx_rollett_k", inputs) - 1.25625).abs() < 1e-12);
        assert!((value("rfx_mu", inputs) - 0.75 / 0.675).abs() < 1e-12);
    }

    #[test]
    fn the_series_gives_its_nearest_value() {
        let v = value("rfx_e_series_nearest", r#"{"series":24,"value":31.83}"#);
        assert!((v - 33.0).abs() < 1e-9);
        let bad =
            crate::compute::compute(&case("rfx_e_series_nearest", r#"{"series":7,"value":1}"#));
        assert!(bad.is_err());
    }
}
