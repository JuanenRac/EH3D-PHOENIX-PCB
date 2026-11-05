// SPDX-License-Identifier: GPL-3.0-or-later
//! The job file of `phoenix stackup`: one JSON document with the stack-up, the materials and the
//! lines to compute, and the reports (text and JSON) of a run.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Language, StackupError};
use crate::line::{analyze, synthesize_width, LineResult, LineSpec};
use crate::material::MaterialSample;
use crate::settings::StackupSettings;
use crate::stackup::Stackup;

/// The largest job text that is read, in bytes.
pub const MAX_JOB_BYTES: usize = 4 * 1024 * 1024;
/// The most lines in one job.
pub const MAX_LINES: usize = 1000;
/// The most materials in one job.
pub const MAX_MATERIALS: usize = 256;

/// The content of a job file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackupJob {
    /// The layers of the board.
    pub stackup: Stackup,
    /// The materials the layers refer to. It may be missing when `phoenix stackup --material`
    /// brings them from the materials database.
    #[serde(default)]
    pub materials: Vec<MaterialSample>,
    /// The lines to compute.
    pub lines: Vec<LineSpec>,
}

/// What happened to one line of a job.
#[derive(Clone, Debug, PartialEq)]
pub struct LineOutcome {
    /// The name of the line.
    pub name: String,
    /// Whether the width was found from a target impedance.
    pub synthesized: bool,
    /// The calculation, or why it was refused.
    pub result: Result<LineResult, StackupError>,
}

impl StackupJob {
    /// Read and check a job text. Nothing is computed.
    pub fn parse(text: &str) -> Result<StackupJob, StackupError> {
        StackupJob::parse_with(text, Vec::new())
    }

    /// Read and check a job text with more materials than the file lists (for example the ones
    /// taken from the materials database). A material of the file wins over one of the same name
    /// in `extra`. Nothing is computed.
    pub fn parse_with(text: &str, extra: Vec<MaterialSample>) -> Result<StackupJob, StackupError> {
        if text.len() > MAX_JOB_BYTES {
            return Err(StackupError::InvalidJob {
                reason: format!("the file is larger than {MAX_JOB_BYTES} bytes"),
            });
        }
        let mut job: StackupJob =
            serde_json::from_str(text).map_err(|e| StackupError::InvalidJob {
                reason: e.to_string(),
            })?;
        for sample in extra {
            if !job.materials.iter().any(|m| m.name == sample.name) {
                job.materials.push(sample);
            }
        }
        if job.materials.len() > MAX_MATERIALS {
            return Err(StackupError::InvalidJob {
                reason: format!("more than {MAX_MATERIALS} materials"),
            });
        }
        if job.lines.is_empty() || job.lines.len() > MAX_LINES {
            return Err(StackupError::InvalidJob {
                reason: format!("there must be 1 to {MAX_LINES} lines"),
            });
        }
        job.stackup.validate_with(&job.materials)?;
        let mut names = BTreeSet::new();
        for spec in &job.lines {
            spec.validate()?;
            if !names.insert(spec.name.clone()) {
                return Err(StackupError::InvalidJob {
                    reason: format!("the line name {} is used twice", spec.name),
                });
            }
        }
        Ok(job)
    }

    /// Compute every line at a frequency in GHz. A line that cannot be computed gives its error
    /// and the others go on.
    pub fn run(&self, freq_ghz: f64, settings: &StackupSettings) -> Vec<LineOutcome> {
        self.lines
            .iter()
            .map(|spec| {
                let (synthesized, result) = match (spec.width_nm, spec.target_ohm) {
                    (Some(width), _) => (
                        false,
                        analyze(
                            &self.stackup,
                            &self.materials,
                            spec,
                            width,
                            freq_ghz,
                            settings,
                        ),
                    ),
                    (None, Some(target)) => (
                        true,
                        synthesize_width(
                            &self.stackup,
                            &self.materials,
                            spec,
                            target,
                            freq_ghz,
                            settings,
                        )
                        .map(|found| found.result),
                    ),
                    (None, None) => (
                        false,
                        Err(StackupError::InvalidLine {
                            reason: format!("{}: give a width or a target impedance", spec.name),
                        }),
                    ),
                };
                LineOutcome {
                    name: spec.name.clone(),
                    synthesized,
                    result,
                }
            })
            .collect()
    }
}

/// The text report of a run, with the titles of the codes in a language.
pub fn report_text(
    stackup_name: &str,
    freq_ghz: f64,
    outcomes: &[LineOutcome],
    language: Language,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "stack-up {stackup_name} at {freq_ghz} GHz");
    for outcome in outcomes {
        match &outcome.result {
            Ok(r) => {
                let _ = writeln!(out, "line {} ({})", r.name, r.kind.as_str());
                let origin = if outcome.synthesized {
                    " (found from the target impedance)"
                } else {
                    ""
                };
                let _ = writeln!(out, "  width: {:.3} mm{origin}", r.width_nm as f64 * 1.0e-6);
                if let Some(gap) = r.gap_nm {
                    let _ = writeln!(out, "  gap: {:.3} mm", gap as f64 * 1.0e-6);
                }
                let _ = writeln!(out, "  impedance: {:.3} ohm", r.impedance_ohm);
                if r.z_single_ohm != r.impedance_ohm {
                    let _ = writeln!(out, "  impedance of one track: {:.3} ohm", r.z_single_ohm);
                }
                let _ = writeln!(out, "  effective permittivity: {:.4}", r.eps_eff);
                let _ = writeln!(out, "  delay: {:.4} ps/mm", r.delay_ps_per_mm);
                let _ = writeln!(out, "  guided wavelength: {:.4} mm", r.guided_wavelength_mm);
                let _ = writeln!(out, "  skin depth: {:.4} um", r.skin_depth_um);
                let _ = writeln!(out, "  roughness factor: {:.4}", r.roughness_factor);
                let _ = writeln!(
                    out,
                    "  conductor loss: {:.5} dB/cm",
                    r.conductor_loss_db_per_cm
                );
                let _ = writeln!(
                    out,
                    "  dielectric loss: {:.5} dB/cm",
                    r.dielectric_loss_db_per_cm
                );
                let _ = writeln!(out, "  total loss: {:.5} dB/cm", r.total_loss_db_per_cm);
                for warning in &r.warnings {
                    let _ = writeln!(
                        out,
                        "  warning {}: {}",
                        warning.code,
                        warning.message(language)
                    );
                }
            }
            Err(error) => {
                let _ = writeln!(out, "line {}", outcome.name);
                let _ = writeln!(out, "  error {}: {}", error.code(), error.message(language));
            }
        }
    }
    out
}

/// The JSON report of a run: for each line its result or its error. Warnings and errors carry
/// the stable code, the English detail and the title in the given language.
pub fn report_json(
    stackup_name: &str,
    freq_ghz: f64,
    outcomes: &[LineOutcome],
    language: Language,
) -> String {
    let lines: Vec<Value> = outcomes
        .iter()
        .map(|outcome| match &outcome.result {
            Ok(r) => {
                let warnings: Vec<Value> = r
                    .warnings
                    .iter()
                    .map(|w| {
                        json!({
                            "code": w.code,
                            "detail": w.detail,
                            "message": w.message(language),
                        })
                    })
                    .collect();
                json!({
                    "name": outcome.name,
                    "synthesized": outcome.synthesized,
                    "kind": r.kind.as_str(),
                    "width_nm": r.width_nm,
                    "gap_nm": r.gap_nm,
                    "impedance_ohm": r.impedance_ohm,
                    "z_single_ohm": r.z_single_ohm,
                    "eps_eff": r.eps_eff,
                    "delay_ps_per_mm": r.delay_ps_per_mm,
                    "guided_wavelength_mm": r.guided_wavelength_mm,
                    "skin_depth_um": r.skin_depth_um,
                    "roughness_factor": r.roughness_factor,
                    "conductor_loss_db_per_cm": r.conductor_loss_db_per_cm,
                    "dielectric_loss_db_per_cm": r.dielectric_loss_db_per_cm,
                    "total_loss_db_per_cm": r.total_loss_db_per_cm,
                    "warnings": warnings,
                })
            }
            Err(error) => json!({
                "name": outcome.name,
                "error": {
                    "code": error.code(),
                    "detail": error.to_string(),
                    "message": error.message(language),
                },
            }),
        })
        .collect();
    let document = json!({
        "stackup": stackup_name,
        "freq_ghz": freq_ghz,
        "lines": lines,
    });
    serde_json::to_string_pretty(&document).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = r#"{
        "stackup": {
            "name": "two-layer",
            "layers": [
                {"kind": "copper", "name": "top", "thickness": {"oz": 1.0}},
                {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4-test"},
                {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}
            ]
        },
        "materials": [
            {"name": "fr4-test", "dk_points": [[1.0, 4.5], [10.0, 4.3]], "df_points": [[1.0, 0.02], [10.0, 0.022]]}
        ],
        "lines": [
            {"name": "fixed", "kind": "microstrip", "layer": "top", "width_nm": 3000000},
            {"name": "fifty", "kind": "microstrip", "layer": "top", "target_ohm": 50.0},
            {"name": "outside", "kind": "microstrip", "layer": "top", "width_nm": 3000000}
        ]
    }"#;

    fn job() -> StackupJob {
        StackupJob::parse(JOB).unwrap()
    }

    #[test]
    fn a_good_job_is_read() {
        let job = job();
        assert_eq!(job.lines.len(), 3);
        assert_eq!(job.materials.len(), 1);
        assert_eq!(job.stackup.layers.len(), 3);
    }

    #[test]
    fn a_run_computes_the_fixed_and_the_synthesised_lines() {
        let outcomes = job().run(2.0, &StackupSettings::default());
        assert_eq!(outcomes.len(), 3);
        assert!(!outcomes[0].synthesized && outcomes[1].synthesized);
        let fixed = outcomes[0].result.as_ref().unwrap();
        let fifty = outcomes[1].result.as_ref().unwrap();
        assert!(fixed.impedance_ohm > 40.0 && fixed.impedance_ohm < 60.0);
        assert!((fifty.impedance_ohm - 50.0).abs() < 0.011);
        // the frequency is inside the published points of the material (1 to 10 GHz)
        let outside = job().run(20.0, &StackupSettings::default());
        assert!(matches!(
            outside[0].result,
            Err(StackupError::OutsidePublished { .. })
        ));
        assert!(matches!(
            outside[1].result,
            Err(StackupError::OutsidePublished { .. })
        ));
    }

    #[test]
    fn the_text_report_lists_values_and_errors_in_the_chosen_language() {
        let settings = StackupSettings::default();
        let ok = job().run(2.0, &settings);
        let text = report_text("two-layer", 2.0, &ok, Language::En);
        assert!(text.contains("stack-up two-layer at 2 GHz"));
        assert!(text.contains("line fixed (microstrip)"));
        assert!(text.contains("impedance:"));
        assert!(text.contains("(found from the target impedance)"));
        assert!(text.contains("total loss:"));
        let bad = job().run(20.0, &settings);
        let english = report_text("two-layer", 20.0, &bad, Language::En);
        assert!(english.contains("error PHOENIX-STACK-004: Frequency outside the published points"));
        let german = report_text("two-layer", 20.0, &bad, Language::De);
        assert!(german.contains("error PHOENIX-STACK-004: Frequenz außerhalb"));
        let portuguese = report_text("two-layer", 20.0, &bad, Language::Pt);
        assert!(portuguese.contains("Frequência fora dos pontos publicados"));
    }

    #[test]
    fn the_json_report_can_be_read_back() {
        let settings = StackupSettings::default();
        let outcomes = job().run(2.0, &settings);
        let text = report_json("two-layer", 2.0, &outcomes, Language::En);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["stackup"], "two-layer");
        assert_eq!(value["lines"].as_array().map(Vec::len), Some(3));
        assert_eq!(value["lines"][0]["kind"], "microstrip");
        assert!(value["lines"][0]["impedance_ohm"].as_f64().unwrap() > 0.0);
        assert_eq!(value["lines"][1]["synthesized"], true);
        let bad = job().run(20.0, &settings);
        let text = report_json("two-layer", 20.0, &bad, Language::Es);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["lines"][0]["error"]["code"], "PHOENIX-STACK-004");
        assert!(value["lines"][0]["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Frecuencia fuera"));
    }

    #[test]
    fn the_report_is_the_same_every_time() {
        let settings = StackupSettings::default();
        let a = report_text("x", 2.0, &job().run(2.0, &settings), Language::En);
        let b = report_text("x", 2.0, &job().run(2.0, &settings), Language::En);
        assert_eq!(a, b);
        let a = report_json("x", 2.0, &job().run(2.0, &settings), Language::En);
        let b = report_json("x", 2.0, &job().run(2.0, &settings), Language::En);
        assert_eq!(a, b);
    }

    #[test]
    fn bad_jobs_are_refused() {
        let cases = [
            ("not json".to_string(), "InvalidJob"),
            (JOB.replace("\"lines\"", "\"tracks\""), "InvalidJob"),
            (
                JOB.replace("\"kind\": \"microstrip\"", "\"kind\": \"waveguide\""),
                "InvalidJob",
            ),
            (JOB.replace("\"fifty\"", "\"fixed\""), "InvalidJob"),
            (JOB.replace("fr4-test\"}", "nowhere\"}"), "UnknownMaterial"),
            (
                JOB.replace("\"thickness_nm\": 1600000", "\"thickness_nm\": 0"),
                "InvalidStackup",
            ),
            (
                JOB.replace(
                    "\"target_ohm\": 50.0",
                    "\"target_ohm\": 50.0, \"width_nm\": 1000",
                ),
                "InvalidLine",
            ),
            ("x".repeat(MAX_JOB_BYTES + 1), "InvalidJob"),
        ];
        for (text, expected) in cases {
            let error = StackupJob::parse(&text).unwrap_err();
            let name = format!("{error:?}");
            assert!(name.starts_with(expected), "{expected}: {name}");
        }
    }

    #[test]
    fn a_job_without_lines_is_refused() {
        let none = JOB
            .split("\"lines\"")
            .next()
            .map(|head| format!("{head}\"lines\": []}}"))
            .unwrap_or_default();
        assert!(matches!(
            StackupJob::parse(&none),
            Err(StackupError::InvalidJob { .. })
        ));
    }

    fn extra(name: &str, dk: f64) -> MaterialSample {
        MaterialSample {
            name: name.to_string(),
            dk_points: vec![(1.0, dk), (10.0, dk)],
            df_points: vec![(1.0, 0.01), (10.0, 0.01)],
            thickness_nm: None,
        }
    }

    #[test]
    fn extra_materials_complete_a_job_and_the_file_wins() {
        let bare = r#"{
            "stackup": {"name": "bare", "layers": [
                {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
                {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "vendor/extra"},
                {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}
            ]},
            "lines": [{"name": "fixed", "kind": "microstrip", "layer": "top", "width_nm": 3000000}]
        }"#;
        assert!(matches!(
            StackupJob::parse(bare),
            Err(StackupError::UnknownMaterial { .. })
        ));
        let job = StackupJob::parse_with(bare, vec![extra("vendor/extra", 3.5)]).unwrap();
        assert_eq!(job.materials.len(), 1);
        let kept =
            StackupJob::parse_with(JOB, vec![extra("fr4-test", 9.0), extra("other", 3.0)]).unwrap();
        assert_eq!(kept.materials.len(), 2);
        assert_eq!(kept.materials[0].dk_points, vec![(1.0, 4.5), (10.0, 4.3)]);
    }
}
