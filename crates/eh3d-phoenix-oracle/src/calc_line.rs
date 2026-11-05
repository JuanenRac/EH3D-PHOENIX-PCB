// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on transmission lines: the constants, the closed-form models and the lines of a
//! stack-up of the stack-up crate, and the figures of lines and loads of the RF crate.

use eh3d_phoenix_rf::line::Line;
use eh3d_phoenix_rf::matching::quarter_wave_impedance;
use eh3d_phoenix_rf::network::{
    abcd_lossless_line, input_impedance, mismatch_loss_db, reflection_from_impedance,
    return_loss_db, vswr,
};
use eh3d_phoenix_rf::{Complex, RfSettings};
use eh3d_phoenix_stackup::elliptic::complete_k;
use eh3d_phoenix_stackup::models;
use eh3d_phoenix_stackup::{
    analyze, CopperThickness, Layer, LineKind, LineSpec, MaterialSample, Stackup, StackupSettings,
};

use crate::codes::OracleError;
use crate::compute::{calc, Inputs};

/// Millimetres to metres.
fn mm(value: f64) -> f64 {
    value * 1.0e-3
}

/// The impedance and the effective permittivity of the microstrip of the inputs.
fn microstrip(i: &Inputs) -> Result<(f64, f64), OracleError> {
    models::microstrip(
        mm(i.num("w_mm")?),
        mm(i.num("t_mm")?),
        mm(i.num("h_mm")?),
        i.num("er")?,
    )
    .map_err(calc)
}

/// A quantity of a line taken from a small stack-up with one dielectric of the inputs.
fn line_quantity(i: &Inputs) -> Result<f64, OracleError> {
    let er = i.num("er")?;
    let freq_ghz = i.num("freq_ghz")?;
    let width_nm = (i.num("width_um")? * 1000.0).round() as i64;
    let dielectric = "oracle-dielectric";
    let material = MaterialSample {
        name: dielectric.to_string(),
        dk_points: vec![(0.1, er), (50.0, er)],
        df_points: vec![(0.1, 0.02), (50.0, 0.02)],
        thickness_nm: None,
    };
    let copper = |name: &str| Layer::Copper {
        name: name.to_string(),
        thickness: CopperThickness::Um(35.0),
        roughness_rms_nm: 0,
    };
    let slab = |name: &str, thickness_nm: i64| Layer::Core {
        name: name.to_string(),
        thickness_nm,
        material: dielectric.to_string(),
    };
    let (layers, kind, layer) = match i.text("kind")? {
        "microstrip" => (
            vec![copper("top"), slab("core", 1_600_000), copper("bottom")],
            LineKind::Microstrip,
            "top",
        ),
        "stripline" => (
            vec![
                copper("plane-1"),
                slab("core-1", 400_000),
                copper("signal"),
                slab("core-2", 400_000),
                copper("plane-2"),
            ],
            LineKind::Stripline,
            "signal",
        ),
        other => return Err(i.unknown("kind", other)),
    };
    let stackup = Stackup {
        name: "oracle".to_string(),
        layers,
    };
    let spec = LineSpec {
        name: "oracle".to_string(),
        kind,
        layer: layer.to_string(),
        width_nm: Some(width_nm),
        target_ohm: None,
        gap_nm: None,
    };
    let result = analyze(
        &stackup,
        &[material],
        &spec,
        width_nm,
        freq_ghz,
        &StackupSettings::default(),
    )
    .map_err(calc)?;
    match i.text("quantity")? {
        "delay_ps_per_mm" => Ok(result.delay_ps_per_mm),
        "skin_depth_um" => Ok(result.skin_depth_um),
        "eps_eff" => Ok(result.eps_eff),
        "impedance_ohm" => Ok(result.impedance_ohm),
        other => Err(i.unknown("quantity", other)),
    }
}

/// The reflection coefficient of the load of the inputs against the reference impedance.
fn load_reflection(i: &Inputs) -> Result<Complex, OracleError> {
    let load = Complex::new(i.num("z_re")?, i.num("z_im")?);
    reflection_from_impedance(load, i.num("z0")?).map_err(calc)
}

/// The input impedance of a lossless line closed by a load.
fn line_input_impedance(i: &Inputs) -> Result<Complex, OracleError> {
    let abcd = abcd_lossless_line(i.num("z0")?, i.num("theta_deg")?.to_radians()).map_err(calc)?;
    let load = Complex::new(i.num("load_re")?, i.num("load_im")?);
    input_impedance(&abcd, load).map_err(calc)
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "const_c0" => models::C0,
        "const_mu0" => models::MU0,
        "const_eta0" => models::ETA0,
        "elliptic_k" => complete_k(i.num("k")?),
        "stripline_zero_exact" => {
            let b = mm(1.0);
            models::stripline_zero_thickness_exact(i.num("w_over_b")? * b, b, i.num("er")?)
                .map_err(calc)?
        }
        "stripline_z0" => models::stripline_symmetric(
            mm(i.num("w_mm")?),
            mm(i.num("t_mm")?),
            mm(i.num("b_mm")?),
            i.num("er")?,
        )
        .map_err(calc)?,
        "microstrip_z0" => microstrip(i)?.0,
        "microstrip_eps" => microstrip(i)?.1,
        "differential_microstrip" => models::differential_microstrip(
            i.num("z_single_ohm")?,
            mm(i.num("s_mm")?),
            mm(i.num("h_mm")?),
        ),
        "differential_stripline" => models::differential_stripline(
            i.num("z_single_ohm")?,
            mm(i.num("s_mm")?),
            mm(i.num("b_mm")?),
        ),
        "line_quantity" => line_quantity(i)?,
        "rf_load_vswr" => vswr(load_reflection(i)?),
        "rf_load_return_loss_db" => return_loss_db(load_reflection(i)?),
        "rf_load_mismatch_loss_db" => mismatch_loss_db(load_reflection(i)?),
        "rf_quarter_wave_impedance" => {
            quarter_wave_impedance(i.num("z1_ohm")?, i.num("z2_ohm")?, &RfSettings::default())
                .map_err(calc)?
        }
        "rf_line_input_impedance_re" => line_input_impedance(i)?.re,
        "rf_line_input_impedance_im" => line_input_impedance(i)?.im,
        "rf_wavelength_m" => Line::new(i.num("z0")?, i.num("eps_eff")?)
            .map_err(calc)?
            .wavelength_m(i.num("freq_hz")?)
            .map_err(calc)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
