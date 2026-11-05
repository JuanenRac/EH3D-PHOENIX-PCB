// SPDX-License-Identifier: GPL-3.0-or-later
//! The environment of a structure: the stack-up, the layer the microstrip is on, the frequency and
//! the settings. It turns an impedance into a track width (with the synthesis of the stack-up
//! crate) and gives the guided wavelength, the effective permittivity and the substrate under the
//! track.

use eh3d_phoenix_rf::RfSettings;
use eh3d_phoenix_stackup::line::{analyze, synthesize_width, LineKind, LineResult, LineSpec};
use eh3d_phoenix_stackup::{Layer, MaterialSample, Stackup, StackupError, StackupSettings};

use crate::codes::{RfgenError, PARAMETER_INVALID, SUBSTRATE_UNUSABLE, SYNTHESIS_FAILED};
use crate::formulas::C0;
use crate::settings::RfgenSettings;

/// The substrate under a microstrip: the dielectric layers between the track and its reference
/// plane, combined as capacitors in series, at the frequency of the design.
#[derive(Debug, Clone, PartialEq)]
pub struct Substrate {
    /// The height of the dielectric in metres.
    pub h_m: f64,
    /// The relative permittivity.
    pub er: f64,
    /// The loss tangent.
    pub tan_d: f64,
    /// The thickness of the copper of the track in metres.
    pub t_m: f64,
    /// The name of the copper layer that is the reference plane.
    pub plane_layer: String,
}

/// A microstrip of a given width at the frequency of the design.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The width in nanometres, always even so that the line is centred exactly.
    pub width_nm: i64,
    /// The impedance in ohm.
    pub z0_ohm: f64,
    /// The effective permittivity.
    pub eps_eff: f64,
    /// The guided wavelength in metres.
    pub lambda_g_m: f64,
    /// What the stack-up calculation says about the result, as code and detail.
    pub warnings: Vec<(String, String)>,
}

impl Line {
    /// The width in metres.
    pub fn width_m(&self) -> f64 {
        self.width_nm as f64 * 1.0e-9
    }
}

/// Everything a structure is computed from.
#[derive(Debug, Clone)]
pub struct Environment {
    /// The layers of the board.
    pub stackup: Stackup,
    /// The materials the layers name.
    pub materials: Vec<MaterialSample>,
    /// The settings of the stack-up calculations.
    pub stack_settings: StackupSettings,
    /// The settings of the generator.
    pub settings: RfgenSettings,
    /// The settings of the RF tools.
    pub rf: RfSettings,
    /// The name of the copper layer the structure is on.
    pub layer: String,
    /// The frequency of the design in GHz.
    pub freq_ghz: f64,
    /// The substrate under that layer.
    pub substrate: Substrate,
}

fn stack_error(e: StackupError) -> RfgenError {
    RfgenError::new(SUBSTRATE_UNUSABLE, e.to_string())
}

/// The dielectric layers on one side of a layer and the copper layer that ends the run.
fn run(layers: &[Layer], from: usize, up: bool) -> (Vec<usize>, Option<usize>) {
    let mut dielectrics = Vec::new();
    let mut index = from;
    loop {
        index = if up {
            match index.checked_sub(1) {
                Some(next) => next,
                None => return (dielectrics, None),
            }
        } else {
            index + 1
        };
        match layers.get(index) {
            None => return (dielectrics, None),
            Some(layer) if layer.is_copper() => return (dielectrics, Some(index)),
            Some(layer) if layer.is_mask() => {}
            Some(_) => dielectrics.push(index),
        }
    }
}

fn substrate_of(
    stackup: &Stackup,
    materials: &[MaterialSample],
    layer_name: &str,
    freq_ghz: f64,
) -> Result<Substrate, RfgenError> {
    let bad = |what: String| Err(RfgenError::new(SUBSTRATE_UNUSABLE, what));
    let Some(index) = stackup.find(layer_name) else {
        return bad(format!("there is no layer named {layer_name}"));
    };
    let Some(layer) = stackup.layers.get(index) else {
        return bad("the layer is missing".to_string());
    };
    if !layer.is_copper() {
        return bad(format!("the layer {layer_name} is not copper"));
    }
    let t_m = layer.thickness_nm().map_err(stack_error)? as f64 * 1.0e-9;
    let (up, up_plane) = run(&stackup.layers, index, true);
    let (down, down_plane) = run(&stackup.layers, index, false);
    let (dielectrics, plane) = match (up.is_empty(), down.is_empty()) {
        (true, false) => (down, down_plane),
        (false, true) => (up, up_plane),
        _ => {
            return bad(format!(
                "the layer {layer_name} needs dielectric on one side and air on the other"
            ))
        }
    };
    let Some(plane) = plane else {
        return bad(format!(
            "there is no copper plane under the substrate of {layer_name}"
        ));
    };
    let plane_layer = stackup
        .layers
        .get(plane)
        .map(|l| l.name().to_string())
        .unwrap_or_default();
    let (mut h_sum, mut h_over_e, mut h_tan_over_e) = (0.0, 0.0, 0.0);
    for &i in &dielectrics {
        let Some(dielectric) = stackup.layers.get(i) else {
            return bad("a layer of the run is missing".to_string());
        };
        let name = dielectric.material().unwrap_or("");
        let Some(material) = materials.iter().find(|m| m.name == name) else {
            return bad(format!("the material {name} is not in the list"));
        };
        let dk = material.dk_at(freq_ghz).map_err(stack_error)?;
        let df = material.df_at(freq_ghz).map_err(stack_error)?;
        let h = dielectric.thickness_nm().map_err(stack_error)? as f64 * 1.0e-9;
        h_sum += h;
        h_over_e += h / dk;
        h_tan_over_e += h * df / dk;
    }
    if h_sum <= 0.0 || h_over_e <= 0.0 {
        return bad("there is no dielectric between the track and the plane".to_string());
    }
    Ok(Substrate {
        h_m: h_sum,
        er: h_sum / h_over_e,
        tan_d: h_tan_over_e / h_over_e,
        t_m,
        plane_layer,
    })
}

impl Environment {
    /// Builds the environment. `layer` is the copper layer of the structure, the first copper
    /// layer of the stack-up when it is `None`. The stack-up, the materials and the frequency are
    /// checked here.
    pub fn new(
        stackup: &Stackup,
        materials: &[MaterialSample],
        layer: Option<&str>,
        freq_ghz: f64,
        settings: RfgenSettings,
        stack_settings: StackupSettings,
    ) -> Result<Environment, RfgenError> {
        if !freq_ghz.is_finite() || freq_ghz <= 0.0 || freq_ghz > stack_settings.max_frequency_ghz {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                format!(
                    "the frequency {freq_ghz} GHz is not between zero and {} GHz",
                    stack_settings.max_frequency_ghz
                ),
            ));
        }
        stackup.validate_with(materials).map_err(stack_error)?;
        let layer = match layer {
            Some(name) => name.to_string(),
            None => stackup
                .layers
                .iter()
                .find(|l| l.is_copper())
                .map(|l| l.name().to_string())
                .ok_or_else(|| RfgenError::new(SUBSTRATE_UNUSABLE, "the stack-up has no copper"))?,
        };
        let substrate = substrate_of(stackup, materials, &layer, freq_ghz)?;
        Ok(Environment {
            stackup: stackup.clone(),
            materials: materials.to_vec(),
            stack_settings,
            settings,
            rf: RfSettings::default(),
            layer,
            freq_ghz,
            substrate,
        })
    }

    /// The frequency in hertz.
    pub fn freq_hz(&self) -> f64 {
        self.freq_ghz * 1.0e9
    }

    /// The free-space wavelength in metres.
    pub fn lambda0_m(&self) -> f64 {
        C0 / self.freq_hz()
    }

    fn spec(&self, target_ohm: Option<f64>) -> LineSpec {
        LineSpec {
            name: "rfgen".to_string(),
            kind: LineKind::Microstrip,
            layer: self.layer.clone(),
            width_nm: None,
            target_ohm,
            gap_nm: None,
        }
    }

    fn line_of(result: &LineResult) -> Line {
        Line {
            width_nm: result.width_nm,
            z0_ohm: result.impedance_ohm,
            eps_eff: result.eps_eff,
            lambda_g_m: result.guided_wavelength_mm * 1.0e-3,
            warnings: result
                .warnings
                .iter()
                .map(|w| (w.code.to_string(), w.detail.clone()))
                .collect(),
        }
    }

    /// The microstrip of a given width (made even by adding one nanometre when it is odd).
    pub fn line_for_width(&self, width_nm: i64) -> Result<Line, RfgenError> {
        let width = width_nm + (width_nm & 1);
        let result = analyze(
            &self.stackup,
            &self.materials,
            &self.spec(None),
            width,
            self.freq_ghz,
            &self.stack_settings,
        )
        .map_err(stack_error)?;
        Ok(Environment::line_of(&result))
    }

    /// The microstrip of an impedance: the width is found by the synthesis of the stack-up crate
    /// and made even, and the line is computed again at that width, so that the impedance in the
    /// result is the one of the width that is drawn.
    pub fn line_for_z0(&self, z0_ohm: f64) -> Result<Line, RfgenError> {
        if !z0_ohm.is_finite() || z0_ohm <= 0.0 {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("the impedance {z0_ohm} ohm must be above zero"),
            ));
        }
        let found = synthesize_width(
            &self.stackup,
            &self.materials,
            &self.spec(Some(z0_ohm)),
            z0_ohm,
            self.freq_ghz,
            &self.stack_settings,
        )
        .map_err(|e| RfgenError::new(SYNTHESIS_FAILED, format!("{z0_ohm} ohm: {e}")))?;
        self.line_for_width(found.width_nm)
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! A stack-up and materials for the tests: a two-layer board of 1.6 mm FR-4 and a four-layer
    //! board with a thin prepreg. The values are invented for the tests, not the data of a sheet.

    use eh3d_phoenix_stackup::{CopperThickness, Layer, MaterialSample, Stackup};

    pub fn flat(name: &str, dk: f64, df: f64) -> MaterialSample {
        MaterialSample {
            name: name.to_string(),
            dk_points: vec![(0.001, dk), (200.0, dk)],
            df_points: vec![(0.001, df), (200.0, df)],
            thickness_nm: None,
        }
    }

    pub fn materials() -> Vec<MaterialSample> {
        vec![flat("fr4", 4.4, 0.02), flat("prepreg", 4.2, 0.02)]
    }

    fn copper(name: &str) -> Layer {
        Layer::Copper {
            name: name.to_string(),
            thickness: CopperThickness::Um(35.0),
            roughness_rms_nm: 0,
        }
    }

    fn core(name: &str, thickness_nm: i64) -> Layer {
        Layer::Core {
            name: name.to_string(),
            thickness_nm,
            material: "fr4".to_string(),
        }
    }

    fn prepreg(name: &str, thickness_nm: i64) -> Layer {
        Layer::Prepreg {
            name: name.to_string(),
            thickness_nm,
            material: "prepreg".to_string(),
        }
    }

    pub fn two_layer() -> Stackup {
        Stackup {
            name: "two-layer".to_string(),
            layers: vec![copper("top"), core("core", 1_600_000), copper("bottom")],
        }
    }

    pub fn four_layer() -> Stackup {
        Stackup {
            name: "four-layer".to_string(),
            layers: vec![
                copper("top"),
                prepreg("pp1", 200_000),
                copper("gnd"),
                core("core", 1_000_000),
                copper("power"),
                prepreg("pp2", 200_000),
                copper("bottom"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn env(stackup: &Stackup, freq_ghz: f64) -> Environment {
        Environment::new(
            stackup,
            &materials(),
            None,
            freq_ghz,
            RfgenSettings::default(),
            StackupSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn a_two_layer_board_gives_the_substrate_under_the_top_layer() {
        let e = env(&two_layer(), 1.0);
        assert_eq!(e.layer, "top");
        assert!((e.substrate.h_m - 1.6e-3).abs() < 1.0e-12);
        assert!((e.substrate.er - 4.4).abs() < 1.0e-9);
        assert_eq!(e.substrate.plane_layer, "bottom");
        assert!((e.substrate.t_m - 35.0e-6).abs() < 1.0e-12);
    }

    #[test]
    fn a_four_layer_board_gives_the_thin_prepreg() {
        let e = env(&four_layer(), 1.0);
        assert!((e.substrate.h_m - 0.2e-3).abs() < 1.0e-12);
        assert!((e.substrate.er - 4.2).abs() < 1.0e-9);
        assert_eq!(e.substrate.plane_layer, "gnd");
    }

    #[test]
    fn the_fifty_ohm_line_on_fr4_is_about_three_millimetres_wide() {
        let e = env(&two_layer(), 1.0);
        let line = e.line_for_z0(50.0).unwrap();
        assert_eq!(line.width_nm & 1, 0);
        assert!(
            (2_800_000..=3_200_000).contains(&line.width_nm),
            "{}",
            line.width_nm
        );
        assert!((line.z0_ohm - 50.0).abs() < 0.05, "{}", line.z0_ohm);
        // the guided wavelength is the free-space one divided by sqrt(eps_eff)
        let expected = C0 / 1.0e9 / line.eps_eff.sqrt();
        assert!(
            (line.lambda_g_m - expected).abs() < 1.0e-6,
            "{}",
            line.lambda_g_m
        );
    }

    #[test]
    fn the_quarter_wave_at_one_gigahertz_is_about_forty_millimetres() {
        let e = env(&two_layer(), 1.0);
        let line = e.line_for_z0(50.0).unwrap();
        let quarter = line.lambda_g_m / 4.0;
        assert!((0.039..=0.043).contains(&quarter), "{quarter}");
    }

    #[test]
    fn a_wider_line_has_a_lower_impedance() {
        let e = env(&two_layer(), 1.0);
        let narrow = e.line_for_width(1_000_000).unwrap();
        let wide = e.line_for_width(4_000_000).unwrap();
        assert!(narrow.z0_ohm > wide.z0_ohm);
        // an odd width is made even
        assert_eq!(e.line_for_width(1_000_001).unwrap().width_nm, 1_000_002);
    }

    #[test]
    fn a_bad_frequency_layer_or_impedance_is_refused() {
        let materials = materials();
        let stackup = two_layer();
        for freq in [0.0, -1.0, f64::NAN, 1.0e6] {
            let e = Environment::new(
                &stackup,
                &materials,
                None,
                freq,
                RfgenSettings::default(),
                StackupSettings::default(),
            )
            .unwrap_err();
            assert_eq!(e.code, PARAMETER_INVALID, "{freq}");
        }
        for layer in ["nowhere", "core"] {
            let e = Environment::new(
                &stackup,
                &materials,
                Some(layer),
                1.0,
                RfgenSettings::default(),
                StackupSettings::default(),
            )
            .unwrap_err();
            assert_eq!(e.code, SUBSTRATE_UNUSABLE, "{layer}");
        }
        let e = env(&two_layer(), 1.0);
        assert!(e.line_for_z0(0.0).is_err());
        assert!(e.line_for_z0(f64::NAN).is_err());
        // no track of this stack-up has 2 ohm
        assert_eq!(e.line_for_z0(2.0).unwrap_err().code, SYNTHESIS_FAILED);
    }

    #[test]
    fn an_inner_layer_cannot_carry_a_microstrip() {
        let e = Environment::new(
            &four_layer(),
            &materials(),
            Some("power"),
            1.0,
            RfgenSettings::default(),
            StackupSettings::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, SUBSTRATE_UNUSABLE);
    }
}
