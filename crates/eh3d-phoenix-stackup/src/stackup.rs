// SPDX-License-Identifier: GPL-3.0-or-later
//! The layers of a board: copper, cores and prepregs, and solder mask.
//!
//! Layers are listed from the top of the board to the bottom. Thicknesses of the board are
//! integers in nanometres, like the geometry of the design model; copper may be written in
//! ounces per square foot or in micrometres. A stack-up is checked before it is used: the number
//! of layers, the names, the order, the ranges of the thicknesses and the roughness.

use serde::{Deserialize, Serialize};

use crate::error::StackupError;
use crate::material::{MaterialSample, MAX_NAME_CHARS};

/// The most layers of a stack-up.
pub const MAX_LAYERS: usize = 64;
/// The thickest layer that is accepted, in nanometres (10 mm).
pub const MAX_LAYER_NM: i64 = 10_000_000;
/// The largest RMS roughness that is accepted, in nanometres (50 micrometres).
pub const MAX_ROUGHNESS_NM: i64 = 50_000;
/// Nanometres in one ounce of copper per square foot. The convention of IPC-2221 is 1.37 mil
/// (34.8 micrometres) per ounce; 1 mil is 25 400 nm.
pub const NM_PER_COPPER_OZ: f64 = 34_798.0;
/// Nanometres in one micrometre.
pub const NM_PER_UM: f64 = 1_000.0;

/// The thickness of a copper layer, written in the unit the designer uses.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopperThickness {
    /// Ounces of copper per square foot (1 oz is 34.798 micrometres).
    Oz(f64),
    /// Micrometres.
    Um(f64),
}

impl CopperThickness {
    /// The thickness in nanometres, rounded to the nearest nanometre.
    pub fn to_nm(self) -> Result<i64, StackupError> {
        let (value, unit_nm, field) = match self {
            CopperThickness::Oz(v) => (v, NM_PER_COPPER_OZ, "copper thickness (oz)"),
            CopperThickness::Um(v) => (v, NM_PER_UM, "copper thickness (um)"),
        };
        if !value.is_finite() || value <= 0.0 {
            return Err(StackupError::InvalidNumber {
                field: field.to_string(),
                reason: "must be a finite number above zero".to_string(),
            });
        }
        let nm = (value * unit_nm).round();
        if !(1.0..=MAX_LAYER_NM as f64).contains(&nm) {
            return Err(StackupError::InvalidNumber {
                field: field.to_string(),
                reason: format!("must be between 1 nm and {MAX_LAYER_NM} nm"),
            });
        }
        Ok(nm as i64)
    }
}

/// One layer of the board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Layer {
    /// A copper layer.
    Copper {
        /// The name of the layer.
        name: String,
        /// The thickness of the foil, with plating.
        thickness: CopperThickness,
        /// The RMS roughness of the copper surface that faces the dielectric, in nanometres.
        #[serde(default)]
        roughness_rms_nm: i64,
    },
    /// A rigid core (a cured laminate).
    Core {
        /// The name of the layer.
        name: String,
        /// The thickness in nanometres.
        thickness_nm: i64,
        /// The name of the material.
        material: String,
    },
    /// A prepreg (a bonding sheet), as pressed.
    Prepreg {
        /// The name of the layer.
        name: String,
        /// The thickness in nanometres, after pressing.
        thickness_nm: i64,
        /// The name of the material.
        material: String,
    },
    /// The solder mask over an outer layer.
    SolderMask {
        /// The name of the layer.
        name: String,
        /// The thickness in nanometres.
        thickness_nm: i64,
        /// The name of the material.
        material: String,
    },
}

impl Layer {
    /// The name of the layer.
    pub fn name(&self) -> &str {
        match self {
            Layer::Copper { name, .. }
            | Layer::Core { name, .. }
            | Layer::Prepreg { name, .. }
            | Layer::SolderMask { name, .. } => name.as_str(),
        }
    }

    /// Whether the layer is copper.
    pub fn is_copper(&self) -> bool {
        matches!(self, Layer::Copper { .. })
    }

    /// Whether the layer is a core or a prepreg.
    pub fn is_dielectric(&self) -> bool {
        matches!(self, Layer::Core { .. } | Layer::Prepreg { .. })
    }

    /// Whether the layer is solder mask.
    pub fn is_mask(&self) -> bool {
        matches!(self, Layer::SolderMask { .. })
    }

    /// The thickness in nanometres (copper is converted from its unit).
    pub fn thickness_nm(&self) -> Result<i64, StackupError> {
        match self {
            Layer::Copper { thickness, .. } => thickness.to_nm(),
            Layer::Core { thickness_nm, .. }
            | Layer::Prepreg { thickness_nm, .. }
            | Layer::SolderMask { thickness_nm, .. } => Ok(*thickness_nm),
        }
    }

    /// The name of the material of a core, a prepreg or a mask.
    pub fn material(&self) -> Option<&str> {
        match self {
            Layer::Copper { .. } => None,
            Layer::Core { material, .. }
            | Layer::Prepreg { material, .. }
            | Layer::SolderMask { material, .. } => Some(material.as_str()),
        }
    }
}

/// The layers of a board, from the top to the bottom.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stackup {
    /// The name of the stack-up.
    pub name: String,
    /// The layers, from the top of the board to the bottom.
    pub layers: Vec<Layer>,
}

impl Stackup {
    /// The position of the layer with a name.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.name() == name)
    }

    /// The total thickness of the board in nanometres.
    pub fn total_thickness_nm(&self) -> Result<i64, StackupError> {
        let mut total: i64 = 0;
        for layer in &self.layers {
            total = total.saturating_add(layer.thickness_nm()?);
        }
        Ok(total)
    }

    /// Check the stack-up by itself: the count, the names, the order and the numbers.
    pub fn validate(&self) -> Result<(), StackupError> {
        let bad = |reason: String| StackupError::InvalidStackup { reason };
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_NAME_CHARS {
            return Err(bad(format!(
                "the name must have 1 to {MAX_NAME_CHARS} characters"
            )));
        }
        if self.layers.is_empty() || self.layers.len() > MAX_LAYERS {
            return Err(bad(format!("there must be 1 to {MAX_LAYERS} layers")));
        }
        let last = self.layers.len() - 1;
        let mut names = std::collections::BTreeSet::new();
        let mut copper = 0_usize;
        for (index, layer) in self.layers.iter().enumerate() {
            let name = layer.name();
            if name.trim().is_empty() || name.chars().count() > MAX_NAME_CHARS {
                return Err(bad(format!(
                    "layer {index}: the name must have 1 to {MAX_NAME_CHARS} characters"
                )));
            }
            if !names.insert(name.to_string()) {
                return Err(bad(format!("the layer name {name} is used twice")));
            }
            let thickness = layer.thickness_nm()?;
            if !(1..=MAX_LAYER_NM).contains(&thickness) {
                return Err(bad(format!(
                    "layer {name}: the thickness must be between 1 nm and {MAX_LAYER_NM} nm"
                )));
            }
            if let Layer::Copper {
                roughness_rms_nm, ..
            } = layer
            {
                copper += 1;
                if !(0..=MAX_ROUGHNESS_NM).contains(roughness_rms_nm) {
                    return Err(bad(format!(
                        "layer {name}: the roughness must be between 0 nm and {MAX_ROUGHNESS_NM} nm"
                    )));
                }
            }
            if layer.is_mask() && index != 0 && index != last {
                return Err(bad(format!(
                    "layer {name}: solder mask is only allowed as the first or the last layer"
                )));
            }
            if index > 0 && layer.is_copper() && self.layers[index - 1].is_copper() {
                return Err(bad(format!(
                    "layers {} and {name}: two copper layers need a dielectric between them",
                    self.layers[index - 1].name()
                )));
            }
        }
        if copper == 0 {
            return Err(bad("there is no copper layer".to_string()));
        }
        Ok(())
    }

    /// Check the stack-up and that every material it names is in `materials`, and that every
    /// material is valid.
    pub fn validate_with(&self, materials: &[MaterialSample]) -> Result<(), StackupError> {
        self.validate()?;
        for material in materials {
            material.validate()?;
        }
        for layer in &self.layers {
            if let Some(name) = layer.material() {
                if !materials.iter().any(|m| m.name == name) {
                    return Err(StackupError::UnknownMaterial {
                        name: name.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn test_materials() -> Vec<MaterialSample> {
    // invented values for the tests; they are not the data of any sheet
    vec![
        MaterialSample {
            name: "fr4-test".to_string(),
            dk_points: vec![(1.0, 4.5), (10.0, 4.3)],
            df_points: vec![(1.0, 0.02), (10.0, 0.022)],
            thickness_nm: None,
        },
        MaterialSample {
            name: "mask-test".to_string(),
            dk_points: vec![(1.0, 3.3)],
            df_points: vec![(1.0, 0.03)],
            thickness_nm: None,
        },
    ]
}

#[cfg(test)]
pub(crate) fn copper(name: &str, um: f64) -> Layer {
    Layer::Copper {
        name: name.to_string(),
        thickness: CopperThickness::Um(um),
        roughness_rms_nm: 0,
    }
}

#[cfg(test)]
pub(crate) fn core_layer(name: &str, thickness_nm: i64) -> Layer {
    Layer::Core {
        name: name.to_string(),
        thickness_nm,
        material: "fr4-test".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_layer() -> Stackup {
        Stackup {
            name: "two-layer".to_string(),
            layers: vec![
                copper("top", 35.0),
                core_layer("core", 1_600_000),
                copper("bottom", 35.0),
            ],
        }
    }

    #[test]
    fn copper_in_ounces_and_in_micrometres_becomes_nanometres() {
        assert_eq!(CopperThickness::Oz(1.0).to_nm().unwrap(), 34_798);
        assert_eq!(CopperThickness::Oz(0.5).to_nm().unwrap(), 17_399);
        assert_eq!(CopperThickness::Um(35.0).to_nm().unwrap(), 35_000);
        assert_eq!(CopperThickness::Um(17.5).to_nm().unwrap(), 17_500);
    }

    #[test]
    fn bad_copper_thicknesses_are_refused() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.0e9] {
            assert!(CopperThickness::Um(value).to_nm().is_err(), "{value}");
            assert!(CopperThickness::Oz(value).to_nm().is_err(), "{value}");
        }
    }

    #[test]
    fn a_good_stackup_is_accepted() {
        let s = two_layer();
        assert!(s.validate().is_ok());
        assert!(s.validate_with(&test_materials()).is_ok());
        assert_eq!(s.total_thickness_nm().unwrap(), 1_670_000);
        assert_eq!(s.find("core"), Some(1));
        assert_eq!(s.find("nothing"), None);
    }

    #[test]
    fn a_stackup_with_mask_on_the_outer_layers_is_accepted() {
        let mask = |name: &str| Layer::SolderMask {
            name: name.to_string(),
            thickness_nm: 20_000,
            material: "mask-test".to_string(),
        };
        let mut s = two_layer();
        s.layers.insert(0, mask("mask-top"));
        s.layers.push(mask("mask-bottom"));
        assert!(s.validate_with(&test_materials()).is_ok());
        // mask in the middle is refused
        let mut inner = two_layer();
        inner.layers.insert(1, mask("mask-inner"));
        assert!(inner.validate().is_err());
    }

    #[test]
    fn broken_stackups_are_refused() {
        let mut no_name = two_layer();
        no_name.name = String::new();
        let empty = Stackup {
            name: "empty".to_string(),
            layers: vec![],
        };
        let mut repeated = two_layer();
        repeated.layers[2] = copper("top", 35.0);
        let mut touching = two_layer();
        touching.layers.remove(1);
        let mut thick = two_layer();
        thick.layers[1] = core_layer("core", MAX_LAYER_NM + 1);
        let mut zero = two_layer();
        zero.layers[1] = core_layer("core", 0);
        let mut rough = two_layer();
        rough.layers[0] = Layer::Copper {
            name: "top".to_string(),
            thickness: CopperThickness::Um(35.0),
            roughness_rms_nm: MAX_ROUGHNESS_NM + 1,
        };
        let mut negative_rough = two_layer();
        negative_rough.layers[0] = Layer::Copper {
            name: "top".to_string(),
            thickness: CopperThickness::Um(35.0),
            roughness_rms_nm: -1,
        };
        let no_copper = Stackup {
            name: "no-copper".to_string(),
            layers: vec![core_layer("core", 1_000_000)],
        };
        let many = Stackup {
            name: "many".to_string(),
            layers: (0..=MAX_LAYERS)
                .map(|i| {
                    if i & 1 == 0 {
                        copper(&format!("c{i}"), 35.0)
                    } else {
                        core_layer(&format!("d{i}"), 100_000)
                    }
                })
                .collect(),
        };
        for s in [
            no_name,
            empty,
            repeated,
            touching,
            thick,
            zero,
            rough,
            negative_rough,
            no_copper,
            many,
        ] {
            assert!(
                matches!(s.validate(), Err(StackupError::InvalidStackup { .. })),
                "{}",
                s.name
            );
        }
    }

    #[test]
    fn an_unknown_material_and_a_bad_material_are_refused() {
        let mut s = two_layer();
        s.layers[1] = Layer::Core {
            name: "core".to_string(),
            thickness_nm: 1_000_000,
            material: "nowhere".to_string(),
        };
        assert!(matches!(
            s.validate_with(&test_materials()),
            Err(StackupError::UnknownMaterial { .. })
        ));
        let mut materials = test_materials();
        materials[0].dk_points.clear();
        assert!(matches!(
            two_layer().validate_with(&materials),
            Err(StackupError::InvalidMaterial { .. })
        ));
    }

    #[test]
    fn json_round_trip_in_both_copper_units() {
        let text = r#"{
            "name": "json",
            "layers": [
                {"kind": "copper", "name": "top", "thickness": {"oz": 1.0}, "roughness_rms_nm": 500},
                {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4-test"},
                {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}
            ]
        }"#;
        let s: Stackup = serde_json::from_str(text).unwrap();
        assert!(s.validate().is_ok());
        let again: Stackup = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(again, s);
        let typo = text.replace("roughness_rms_nm", "roughnes_rms_nm");
        assert!(serde_json::from_str::<Stackup>(&typo).is_err());
        let unknown_kind = text.replace("\"core\"", "\"fabric\"");
        assert!(serde_json::from_str::<Stackup>(&unknown_kind).is_err());
    }
}
