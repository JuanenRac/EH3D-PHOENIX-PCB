// SPDX-License-Identifier: GPL-3.0-or-later
//! An adapter from the materials database of EH3D-COMMON (`eh3d-materials`) to a
//! [`MaterialSample`].
//!
//! This module is compiled only with the feature `materials`, which is off by default and cannot
//! be switched on yet: the CI of this repository builds against a tag of EH3D-COMMON that does
//! not contain the materials crate, and this code has not been compiled. It is written against
//! the public record of that crate (`Material` with the lists `dk` and `df` of `DkPoint`, and
//! `thicknesses_mm`). When EH3D-COMMON publishes the crate and the tag in `SIBLING_REFS` moves,
//! add the dependency named in `Cargo.toml`, switch the feature on in the CI and compile this file.

use eh3d_materials::model::{DkPoint, Material};

use crate::error::StackupError;
use crate::material::MaterialSample;

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
        [only] if only.is_finite() && *only > 0.0 => Some((only * 1.0e6).round() as i64),
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
