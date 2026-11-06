// SPDX-License-Identifier: GPL-3.0-or-later
//! The line that a track makes with the stack-up under it: impedance, effective permittivity,
//! delay, loss at a frequency, and the height and permittivity of the dielectric between the
//! track and its reference plane. They come from the stack-up crate (and so from the materials of
//! the job); when there is no stack-up, or the layer cannot be taken from it, the defaults of the
//! settings are used and the report says so.

use std::collections::BTreeMap;

use eh3d_phoenix_stackup::{
    analyze, LineKind, LineSpec, MaterialSample, Stackup, StackupJob, StackupSettings,
};

use crate::codes::{Finding, Severity, STACKUP_FALLBACK};
use crate::phys::{delay_ps_per_mm, fallback_loss_db_per_cm};
use crate::settings::SiSettings;

/// The frequency at which the permittivity and the impedance are taken, in GHz.
const REFERENCE_GHZ: f64 = 1.0;

/// One line over its reference plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineEnv {
    /// The impedance of the line, in ohm.
    pub z0: f64,
    /// The effective relative permittivity.
    pub eps_eff: f64,
    /// The delay, in picoseconds per millimetre.
    pub delay_ps_per_mm: f64,
    /// The loss at the frequency asked for, in decibel per centimetre.
    pub loss_db_per_cm: f64,
    /// The height of the dielectric between the track and the nearest reference plane, in metres.
    pub h_m: f64,
    /// The relative permittivity of that dielectric.
    pub er: f64,
    /// Whether the line is a microstrip (outer layer) and not a stripline.
    pub microstrip: bool,
}

/// The lines of a board, worked out once for each layer, width and frequency.
pub struct Env<'a> {
    job: Option<&'a StackupJob>,
    settings: &'a SiSettings,
    stackup_settings: StackupSettings,
    cache: BTreeMap<(String, i64, u64), LineEnv>,
    /// Why a layer could not be taken from the stack-up, by the name of the layer; the empty
    /// name is used when no stack-up was given at all.
    notes: BTreeMap<String, String>,
}

impl<'a> Env<'a> {
    /// The lines of a board with the stack-up of a job, or without one.
    pub fn new(job: Option<&'a StackupJob>, settings: &'a SiSettings) -> Env<'a> {
        Env {
            job,
            settings,
            stackup_settings: StackupSettings::default(),
            cache: BTreeMap::new(),
            notes: BTreeMap::new(),
        }
    }

    /// The line of a track of this width on a layer at a frequency in GHz.
    pub fn line(&mut self, layer: &str, width_nm: i64, freq_ghz: f64) -> LineEnv {
        let key = (layer.to_owned(), width_nm, freq_ghz.to_bits());
        if let Some(found) = self.cache.get(&key) {
            return *found;
        }
        let value = match self.job {
            Some(job) => {
                match from_stackup(job, layer, width_nm, freq_ghz, &self.stackup_settings) {
                    Ok(v) => v,
                    Err(why) => {
                        self.notes.entry(layer.to_owned()).or_insert(why);
                        self.defaults(width_nm, freq_ghz)
                    }
                }
            }
            None => {
                self.notes
                    .entry(String::new())
                    .or_insert_with(|| "no stack-up was given".to_owned());
                self.defaults(width_nm, freq_ghz)
            }
        };
        self.cache.insert(key, value);
        value
    }

    /// The line from the defaults of the settings.
    fn defaults(&self, width_nm: i64, freq_ghz: f64) -> LineEnv {
        let s = self.settings;
        LineEnv {
            z0: s.default_z0_ohm,
            eps_eff: s.default_eps_eff,
            delay_ps_per_mm: delay_ps_per_mm(s.default_eps_eff),
            loss_db_per_cm: fallback_loss_db_per_cm(
                freq_ghz,
                s.default_eps_eff,
                s.default_tan_d,
                s.default_z0_ohm,
                width_nm as f64 * 1.0e-9,
            ),
            h_m: s.default_h_nm as f64 * 1.0e-9,
            er: s.default_er,
            microstrip: true,
        }
    }

    /// The findings about the layers that were taken from the defaults: information when no
    /// stack-up was given, a warning for a layer that the stack-up could not give.
    pub fn findings(&self) -> Vec<Finding> {
        self.notes
            .iter()
            .map(|(layer, why)| {
                if layer.is_empty() {
                    Finding::new(
                        STACKUP_FALLBACK,
                        Severity::Info,
                        None,
                        format!("{why}: the defaults of the settings are used"),
                    )
                } else {
                    Finding::new(
                        STACKUP_FALLBACK,
                        Severity::Warning,
                        Some(layer.as_str()),
                        why.clone(),
                    )
                }
            })
            .collect()
    }
}

/// The line of a track of this width on a layer of the stack-up, or why it cannot be worked out.
fn from_stackup(
    job: &StackupJob,
    layer: &str,
    width_nm: i64,
    freq_ghz: f64,
    settings: &StackupSettings,
) -> Result<LineEnv, String> {
    let stackup = &job.stackup;
    let at = stackup
        .find(layer)
        .ok_or_else(|| format!("the stack-up has no layer named {layer}"))?;
    let coppers: Vec<usize> = stackup
        .layers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_copper())
        .map(|(i, _)| i)
        .collect();
    let microstrip = coppers.first() == Some(&at) || coppers.last() == Some(&at);
    let spec = LineSpec {
        name: layer.to_owned(),
        kind: if microstrip {
            LineKind::Microstrip
        } else {
            LineKind::Stripline
        },
        layer: layer.to_owned(),
        width_nm: Some(width_nm),
        target_ohm: None,
        gap_nm: None,
    };
    let at_reference = analyze(
        stackup,
        &job.materials,
        &spec,
        width_nm,
        REFERENCE_GHZ,
        settings,
    )
    .map_err(|e| e.to_string())?;
    let (h_m, er) = plane_gap(stackup, &job.materials, at, REFERENCE_GHZ)
        .ok_or_else(|| format!("no reference plane next to the layer {layer}"))?;
    // the loss at the frequency asked for; if the materials do not reach it, the formula of the
    // fallback with the permittivity of the line is used
    let loss_db_per_cm = match analyze(stackup, &job.materials, &spec, width_nm, freq_ghz, settings)
    {
        Ok(r) => r.total_loss_db_per_cm,
        Err(_) => fallback_loss_db_per_cm(
            freq_ghz,
            at_reference.eps_eff,
            0.02,
            at_reference.impedance_ohm,
            width_nm as f64 * 1.0e-9,
        ),
    };
    Ok(LineEnv {
        z0: at_reference.impedance_ohm,
        eps_eff: at_reference.eps_eff,
        delay_ps_per_mm: at_reference.delay_ps_per_mm,
        loss_db_per_cm,
        h_m,
        er,
        microstrip,
    })
}

/// The height and the relative permittivity of the dielectric between a copper layer and the
/// nearest copper layer next to it, on the side where it is nearer; `None` when no copper layer
/// can be reached through dielectrics with a known permittivity.
pub fn plane_gap(
    stackup: &Stackup,
    materials: &[MaterialSample],
    at: usize,
    freq_ghz: f64,
) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for up in [true, false] {
        let mut sum_h = 0.0;
        let mut sum_h_over_e = 0.0;
        let mut reached = false;
        let mut index = at;
        loop {
            index = if up {
                match index.checked_sub(1) {
                    Some(next) => next,
                    None => break,
                }
            } else {
                index + 1
            };
            let Some(layer) = stackup.layers.get(index) else {
                break;
            };
            if layer.is_copper() {
                reached = true;
                break;
            }
            if layer.is_dielectric() {
                let h = layer.thickness_nm().ok()? as f64 * 1.0e-9;
                let name = layer.material()?;
                let material = materials.iter().find(|m| m.name == name)?;
                let dk = material.dk_at(freq_ghz).ok()?;
                sum_h += h;
                sum_h_over_e += h / dk;
            }
        }
        if reached && sum_h > 0.0 && sum_h_over_e > 0.0 {
            let candidate = (sum_h, sum_h / sum_h_over_e);
            if best.is_none_or(|(h, _)| candidate.0 < h) {
                best = Some(candidate);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = r#"{
      "stackup": {"name": "four", "layers": [
        {"kind": "copper", "name": "F.Cu", "thickness": {"um": 35.0}},
        {"kind": "prepreg", "name": "p1", "thickness_nm": 200000, "material": "fr4"},
        {"kind": "copper", "name": "In1.Cu", "thickness": {"um": 35.0}},
        {"kind": "core", "name": "core", "thickness_nm": 1000000, "material": "fr4"},
        {"kind": "copper", "name": "In2.Cu", "thickness": {"um": 35.0}},
        {"kind": "prepreg", "name": "p2", "thickness_nm": 200000, "material": "fr4"},
        {"kind": "copper", "name": "B.Cu", "thickness": {"um": 35.0}}
      ]},
      "materials": [
        {"name": "fr4", "dk_points": [[0.1, 4.3], [10.0, 4.3]], "df_points": [[0.1, 0.02], [10.0, 0.02]]}
      ],
      "lines": [{"name": "x", "kind": "microstrip", "layer": "F.Cu", "width_nm": 300000}]
    }"#;

    #[test]
    fn a_microstrip_takes_its_height_and_permittivity_from_the_stack_up() {
        let job = StackupJob::parse(JOB).unwrap();
        let settings = SiSettings::default();
        let mut env = Env::new(Some(&job), &settings);
        let line = env.line("F.Cu", 300_000, 0.7);
        assert!(line.microstrip);
        assert!((line.h_m - 200.0e-6).abs() < 1e-12);
        assert!((line.er - 4.3).abs() < 1e-9);
        assert!(line.z0 > 20.0 && line.z0 < 120.0, "{}", line.z0);
        assert!(line.eps_eff > 1.0 && line.eps_eff < 4.3);
        assert!(line.loss_db_per_cm > 0.0);
        assert!(env.findings().is_empty());
    }

    #[test]
    fn an_inner_layer_is_a_stripline_with_the_nearer_plane() {
        let job = StackupJob::parse(JOB).unwrap();
        let settings = SiSettings::default();
        let mut env = Env::new(Some(&job), &settings);
        let line = env.line("In1.Cu", 150_000, 1.0);
        assert!(!line.microstrip);
        // the prepreg above (0.2 mm) is nearer than the core below (1 mm)
        assert!((line.h_m - 200.0e-6).abs() < 1e-12);
    }

    #[test]
    fn a_layer_the_stack_up_does_not_have_falls_back_with_a_warning() {
        let job = StackupJob::parse(JOB).unwrap();
        let settings = SiSettings::default();
        let mut env = Env::new(Some(&job), &settings);
        let line = env.line("Missing", 200_000, 1.0);
        assert_eq!(line.z0, settings.default_z0_ohm);
        let findings = env.findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].subject.as_deref(), Some("Missing"));
    }

    #[test]
    fn without_a_stack_up_the_defaults_are_used_and_one_note_is_made() {
        let settings = SiSettings::default();
        let mut env = Env::new(None, &settings);
        let a = env.line("F.Cu", 200_000, 1.0);
        let b = env.line("B.Cu", 200_000, 1.0);
        assert_eq!(a.eps_eff, 3.0);
        assert_eq!(b.er, 4.3);
        // sqrt(3) / c in ps/mm
        assert!(
            (a.delay_ps_per_mm - 5.7773).abs() < 1e-3,
            "{}",
            a.delay_ps_per_mm
        );
        let findings = env.findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[0].subject, None);
    }
}
