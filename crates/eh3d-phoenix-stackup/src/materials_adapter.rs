// SPDX-License-Identifier: GPL-3.0-or-later
//! An adapter from the materials database of EH3D-COMMON (`eh3d-materials`) to a
//! [`MaterialSample`], to the thicknesses of the dielectric and to the copper of a laminate.
//!
//! This module is compiled only with the feature `materials`. Dk and Df come from the points a
//! datasheet publishes with a frequency; a value the sheet gives with no frequency (a design
//! value for a band, for example) is not a point and is left out, so a frequency outside the
//! published points is an error and nothing is extrapolated.

use eh3d_materials::model::{DkPoint, Material};
use eh3d_materials::Database;

use crate::error::StackupError;
use crate::material::MaterialSample;
use crate::stackup::CopperThickness;

/// Nanometres in one millimetre.
const NM_PER_MM: f64 = 1.0e6;

/// The points of a list that have a frequency, by increasing frequency, one per frequency (the
/// first listed wins, as in the database).
fn points(list: &[DkPoint]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for point in list {
        if let Some(freq) = point.freq_ghz {
            if freq.is_finite() && freq > 0.0 && !out.iter().any(|p| p.0 == freq) {
                out.push((freq, point.value));
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// Make a [`MaterialSample`] from a record of the database. The name of the sample is the
/// identifier of the record. A record whose Dk or Df has no point with a frequency is refused,
/// because no value could be interpolated. The nominal thickness is set only when the record
/// lists exactly one.
pub fn sample_from_material(material: &Material) -> Result<MaterialSample, StackupError> {
    let thickness_nm = match material.thicknesses_mm.as_slice() {
        [only] if only.is_finite() && *only > 0.0 => Some((only * NM_PER_MM).round() as i64),
        _ => None,
    };
    let sample = MaterialSample {
        name: material.id.clone(),
        dk_points: points(&material.dk),
        df_points: points(&material.df),
        thickness_nm,
    };
    sample.validate()?;
    Ok(sample)
}

/// Make a [`MaterialSample`] from the record with identifier `id` in `database`. An identifier
/// the database does not have is the error `UnknownMaterial`.
pub fn sample_from_database(database: &Database, id: &str) -> Result<MaterialSample, StackupError> {
    match database.get(id) {
        Some(material) => sample_from_material(material),
        None => Err(StackupError::UnknownMaterial {
            name: id.to_string(),
        }),
    }
}

/// The thicknesses the record lists, in nanometres, as published (the ones that are above zero).
pub fn thicknesses_nm(material: &Material) -> Vec<i64> {
    material
        .thicknesses_mm
        .iter()
        .filter(|mm| mm.is_finite() && **mm > 0.0)
        .map(|mm| (mm * NM_PER_MM).round() as i64)
        .collect()
}

/// The copper foils the record lists: in micrometres when the sheet gives the thickness, else in
/// ounces per square foot. A foil with neither is left out.
pub fn copper_thicknesses(material: &Material) -> Vec<CopperThickness> {
    let mut out = Vec::new();
    for foil in &material.copper {
        if let Some(um) = foil.thickness_um.filter(|v| v.is_finite() && *v > 0.0) {
            out.push(CopperThickness::Um(um));
        } else if let Some(oz) = foil.weight_oz.filter(|v| v.is_finite() && *v > 0.0) {
            out.push(CopperThickness::Oz(oz));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_materials::Kind;

    fn database() -> Database {
        Database::embedded().unwrap()
    }

    #[test]
    fn ro4350b_has_the_dk_of_its_sheet_at_10_ghz_and_nowhere_else() {
        let sample = sample_from_database(&database(), "rogers/ro4350b").unwrap();
        assert_eq!(sample.name, "rogers/ro4350b");
        // the design value of 3.66 has no frequency on the sheet: only the 3.48 at 10 GHz is a point
        assert_eq!(sample.dk_at(10.0).unwrap(), 3.48);
        assert!(matches!(
            sample.dk_at(2.5),
            Err(StackupError::OutsidePublished { .. })
        ));
        assert!(matches!(
            sample.dk_at(11.0),
            Err(StackupError::OutsidePublished { .. })
        ));
        // the loss tangent has two points, 2.5 and 10 GHz
        assert_eq!(sample.df_at(2.5).unwrap(), 0.0031);
        assert_eq!(sample.df_at(10.0).unwrap(), 0.0037);
        let mid = sample.df_at(5.0).unwrap();
        assert!(mid > 0.0031 && mid < 0.0037, "{mid}");
    }

    #[test]
    fn the_fr4_of_bungard_is_interpolated_between_its_points() {
        let sample = sample_from_database(&database(), "bungard/fr4-presensitized").unwrap();
        // 1 MHz, 100 MHz and 1 GHz are published; 10 MHz is the middle of the first two decades
        assert_eq!(sample.dk_at(0.001).unwrap(), 4.8);
        assert_eq!(sample.dk_at(1.0).unwrap(), 4.5);
        assert!((sample.dk_at(0.01).unwrap() - 4.7).abs() < 1e-9);
        assert!((sample.df_at(0.5).unwrap() - 0.015).abs() < 1e-12);
        assert!(sample.dk_at(2.0).is_err());
        assert!(sample.dk_at(0.0005).is_err());
    }

    #[test]
    fn a_material_that_does_not_exist_is_an_unknown_material() {
        let error = sample_from_database(&database(), "nobody/nothing").unwrap_err();
        assert!(
            matches!(&error, StackupError::UnknownMaterial { name } if name == "nobody/nothing")
        );
        assert_eq!(error.code(), "PHOENIX-STACK-003");
    }

    #[test]
    fn a_material_without_dk_points_is_refused() {
        // the copper clad FR-4 of Bungard publishes no Dk
        let error = sample_from_database(&database(), "bungard/fr4-copper-clad").unwrap_err();
        assert!(matches!(error, StackupError::InvalidMaterial { .. }));
    }

    #[test]
    fn the_thickness_is_nominal_only_when_the_record_lists_one() {
        let mut one = Material::new("a/b", "A", "B", Kind::Laminate, "b.pdf");
        one.dk = vec![DkPoint {
            freq_ghz: Some(1.0),
            value: 4.0,
            method: None,
            resin_content_pct: None,
            tolerance: None,
        }];
        one.df = vec![DkPoint {
            freq_ghz: Some(1.0),
            value: 0.02,
            method: None,
            resin_content_pct: None,
            tolerance: None,
        }];
        one.thicknesses_mm = vec![1.6];
        assert_eq!(
            sample_from_material(&one).unwrap().thickness_nm,
            Some(1_600_000)
        );
        one.thicknesses_mm = vec![0.8, 1.6];
        assert_eq!(sample_from_material(&one).unwrap().thickness_nm, None);
        assert_eq!(thicknesses_nm(&one), vec![800_000, 1_600_000]);
    }

    #[test]
    fn the_thicknesses_and_the_copper_of_a_real_laminate_are_read() {
        let db = database();
        let ro4350b = db.get("rogers/ro4350b").unwrap();
        let thicknesses = thicknesses_nm(ro4350b);
        assert_eq!(thicknesses.len(), 8);
        assert_eq!(thicknesses[0], 101_000);
        assert_eq!(thicknesses[5], 508_000);
        let copper = copper_thicknesses(ro4350b);
        assert_eq!(
            copper,
            vec![
                CopperThickness::Um(17.0),
                CopperThickness::Um(35.0),
                CopperThickness::Um(70.0)
            ]
        );
    }
}
