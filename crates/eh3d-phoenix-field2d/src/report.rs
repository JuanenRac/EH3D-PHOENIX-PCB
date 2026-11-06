// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of `phoenix field2d`, as text and as JSON. The same input gives the same bytes.

use std::fmt::Write as _;

use eh3d_phoenix_stackup::Language;
use serde_json::{json, Value};

use crate::analysis::{CaseOutcome, LineField};
use crate::codes::title;
use crate::settings::Field2dSettings;

fn settings_value(settings: &Field2dSettings) -> Value {
    json!({
        "method": settings.method.as_str(),
        "tolerance": settings.tolerance,
        "max_iterations": settings.max_iterations,
        "max_cells": settings.max_cells,
        "edge_step_fraction": settings.edge_step_fraction,
        "max_step_fraction": settings.max_step_fraction,
        "grading_ratio": settings.grading_ratio,
        "refine": settings.refine,
        "box_margin_factor": settings.box_margin_factor,
    })
}

/// The report as a JSON value.
pub fn report_value(
    name: &str,
    freq_ghz: f64,
    outcomes: &[CaseOutcome],
    settings: &Field2dSettings,
) -> Value {
    let lines: Vec<Value> = outcomes
        .iter()
        .map(|outcome| match &outcome.result {
            Ok(found) => json!({
                "name": outcome.name,
                "result": found,
            }),
            Err(error) => json!({
                "name": outcome.name,
                "error": {"code": error.code, "detail": error.detail},
            }),
        })
        .collect();
    json!({
        "format": "phoenix-field2d/1",
        "state": "prototype",
        "name": name,
        "freq_ghz": freq_ghz,
        "settings": settings_value(settings),
        "lines": lines,
    })
}

/// The report as JSON text.
pub fn report_json(
    name: &str,
    freq_ghz: f64,
    outcomes: &[CaseOutcome],
    settings: &Field2dSettings,
) -> String {
    let mut text = serde_json::to_string_pretty(&report_value(name, freq_ghz, outcomes, settings))
        .unwrap_or_default();
    text.push('\n');
    text
}

fn write_line(out: &mut String, found: &LineField, language: Language) {
    let _ = writeln!(out, "line {} ({})", found.name, found.kind.as_str());
    let origin = if found.synthesized {
        " (found from the target impedance by the closed formula)"
    } else {
        ""
    };
    let _ = writeln!(
        out,
        "  width: {:.3} mm{origin}",
        found.width_nm as f64 * 1.0e-6
    );
    if let Some(gap) = found.gap_nm {
        let _ = writeln!(out, "  gap: {:.3} mm", gap as f64 * 1.0e-6);
    }
    let f = &found.field;
    let _ = writeln!(
        out,
        "  mesh: {} by {} = {} cells, {} iterations, residual {:.2e}",
        f.nx, f.ny, f.cells, f.iterations, f.residual
    );
    match &f.pair {
        Some(pair) => {
            let _ = writeln!(out, "  differential impedance: {:.3} ohm", pair.z_diff_ohm);
            let _ = writeln!(out, "  common impedance: {:.3} ohm", pair.z_common_ohm);
            let _ = writeln!(
                out,
                "  even mode: {:.3} ohm, eps_eff {:.4}",
                pair.z_even_ohm, pair.eps_even
            );
            let _ = writeln!(
                out,
                "  odd mode: {:.3} ohm, eps_eff {:.4}",
                pair.z_odd_ohm, pair.eps_odd
            );
            let _ = writeln!(out, "  mutual capacitance: {:.4} pF/m", pair.cm_pf_per_m);
            let _ = writeln!(
                out,
                "  self inductance: {:.3} nH/m, mutual inductance: {:.3} nH/m",
                pair.ls_nh_per_m, pair.lm_nh_per_m
            );
        }
        None => {
            let _ = writeln!(out, "  impedance: {:.3} ohm", f.z0_ohm);
            let _ = writeln!(out, "  effective permittivity: {:.4}", f.eps_eff);
            let _ = writeln!(out, "  inductance: {:.3} nH/m", f.l_nh_per_m);
        }
    }
    let _ = writeln!(
        out,
        "  capacitance: {:.4} pF/m, in vacuum {:.4} pF/m",
        f.c_pf_per_m, f.c_air_pf_per_m
    );
    let _ = writeln!(out, "  delay: {:.4} ps/mm", f.delay_ps_per_mm);
    let _ = writeln!(
        out,
        "  dielectric loss: {:.5} dB/cm",
        f.dielectric_loss_db_per_cm
    );
    match &found.comparison {
        Some(c) => {
            let _ = writeln!(
                out,
                "  closed formula: {:.3} ohm, the field solver differs by {:+.2} %",
                c.closed_impedance_ohm, c.impedance_difference_percent
            );
            if let (Some(closed), Some(diff)) = (c.closed_eps_eff, c.eps_difference_percent) {
                let _ = writeln!(
                    out,
                    "  closed formula permittivity: {closed:.4}, the field solver differs by {diff:+.2} %"
                );
            }
        }
        None => {
            let _ = writeln!(out, "  closed formula: not available for this line");
        }
    }
    for note in &found.notes {
        let head = title(note.code, language).unwrap_or("");
        let _ = writeln!(out, "  warning {}: {head}: {}", note.code, note.detail);
    }
}

/// The report as text, with the titles of the codes in a language.
pub fn report_text(
    name: &str,
    freq_ghz: f64,
    outcomes: &[CaseOutcome],
    settings: &Field2dSettings,
    language: Language,
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "field solver (prototype) for {name} at {freq_ghz} GHz: method {}, tolerance {:e}, refine {}, box {} times",
        settings.method.as_str(),
        settings.tolerance,
        settings.refine,
        settings.box_margin_factor
    );
    for outcome in outcomes {
        match &outcome.result {
            Ok(found) => write_line(&mut out, found, language),
            Err(error) => {
                let _ = writeln!(out, "line {}", outcome.name);
                let _ = writeln!(out, "  error {}", error.message(language));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{FieldError, NOT_CONVERGED};

    fn failed() -> CaseOutcome {
        CaseOutcome {
            name: "bad".to_string(),
            result: Err(FieldError::new(NOT_CONVERGED, "after 3 iterations")),
        }
    }

    #[test]
    fn a_line_that_failed_is_reported_with_its_code_and_the_others_go_on() {
        let settings = Field2dSettings::default();
        let text = report_text("board", 1.0, &[failed()], &settings, Language::De);
        assert!(text.contains("line bad"));
        assert!(text.contains("PHOENIX-FIELD-004"));
        assert!(text.contains("Löser"));
        let json = report_value("board", 1.0, &[failed()], &settings);
        assert_eq!(json["format"], "phoenix-field2d/1");
        assert_eq!(json["state"], "prototype");
        assert_eq!(json["lines"][0]["error"]["code"], "PHOENIX-FIELD-004");
        assert_eq!(json["settings"]["method"], "cg");
    }

    #[test]
    fn the_json_text_ends_with_a_newline_and_is_the_same_every_time() {
        let settings = Field2dSettings::default();
        let a = report_json("board", 1.0, &[failed()], &settings);
        let b = report_json("board", 1.0, &[failed()], &settings);
        assert_eq!(a, b);
        assert!(a.ends_with("}\n"));
    }
}
