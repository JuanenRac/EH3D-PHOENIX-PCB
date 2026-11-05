// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene and its checks as text and as JSON. The same input always gives the same bytes: the
//! order is fixed and every length is an integer number of nanometres or millimetres with six
//! decimals worked out from one.

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::check::{reach_nm, Checks};
use crate::codes::{Finding, Severity};
use crate::scene::{millimetres, PartSource, Scene3d};

/// The word for where the box of a component comes from.
pub fn source_name(source: PartSource) -> String {
    match source {
        PartSource::Table => "table".to_owned(),
        PartSource::StepBox => "step_box".to_owned(),
        PartSource::StepEnclosing(kind) => format!("step_enclosing_{kind:?}").to_ascii_lowercase(),
    }
}

/// Every finding of the scene and of the checks, the most serious first.
pub fn all_findings<'a>(scene: &'a Scene3d, checks: &'a Checks) -> Vec<&'a Finding> {
    let mut all: Vec<&Finding> = scene
        .findings
        .iter()
        .chain(checks.findings.iter())
        .collect();
    all.sort_by_key(|f| std::cmp::Reverse(f.severity));
    all
}

/// The scene as JSON, ending with a line break.
pub fn scene_json(scene: &Scene3d, checks: &Checks, extra: &[Finding]) -> String {
    let parts: Vec<Value> = scene
        .parts
        .iter()
        .map(|p| {
            json!({
                "reference": p.reference,
                "footprint": p.footprint,
                "side": if p.bottom { "bottom" } else { "top" },
                "source": source_name(p.source),
                "corners_nm": p.corners.iter().map(|c| json!([c.0, c.1])).collect::<Vec<Value>>(),
                "z_min_nm": p.z_min,
                "z_max_nm": p.z_max,
                "reach_nm": reach_nm(p, scene.thickness_nm),
            })
        })
        .collect();
    let collisions: Vec<Value> = checks
        .collisions
        .iter()
        .map(|c| {
            json!({
                "first": scene.parts[c.first].reference,
                "second": scene.parts[c.second].reference,
                "height_overlap_nm": c.height_overlap_nm,
            })
        })
        .collect();
    let mut findings: Vec<&Finding> = all_findings(scene, checks);
    findings.extend(extra.iter());
    let findings: Vec<Value> = findings
        .iter()
        .map(|f| {
            json!({
                "code": f.code,
                "severity": f.severity.as_str(),
                "subject": f.subject,
                "detail": f.detail,
            })
        })
        .collect();
    let outline: Vec<Value> = scene.outline.iter().map(|p| json!([p.0, p.1])).collect();
    let doc = json!({
        "format": "phoenix-assembly3d/1",
        "board": {
            "name": scene.name,
            "thickness_nm": scene.thickness_nm,
            "outline_nm": outline,
        },
        "parts": parts,
        "collisions": collisions,
        "too_tall": checks.too_tall.iter().map(|i| scene.parts[*i].reference.clone()).collect::<Vec<String>>(),
        "outside_outline": checks.outside.iter().map(|i| scene.parts[*i].reference.clone()).collect::<Vec<String>>(),
        "findings": findings,
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// The scene as text: the board, the components, the checks and the findings.
pub fn scene_text(scene: &Scene3d, checks: &Checks, extra: &[Finding], lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "board {}: {} mm thick, {} components with a box\n",
        scene.name,
        millimetres(scene.thickness_nm),
        scene.parts.len()
    ));
    for p in &scene.parts {
        out.push_str(&format!(
            "{} {} {}: height {} to {} mm ({})\n",
            p.reference,
            p.footprint,
            if p.bottom { "bottom" } else { "top" },
            millimetres(p.z_min),
            millimetres(p.z_max),
            source_name(p.source)
        ));
    }
    let mut findings = all_findings(scene, checks);
    findings.extend(extra.iter());
    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    out.push_str(&format!(
        "checks: {} collisions, {} too tall, {} outside the outline\n",
        checks.collisions.len(),
        checks.too_tall.len(),
        checks.outside.len()
    ));
    out.push_str(&format!(
        "findings: {} errors, {} warnings, {} information\n",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    ));
    for f in findings {
        out.push_str(&f.line(lang));
        out.push('\n');
    }
    out
}
