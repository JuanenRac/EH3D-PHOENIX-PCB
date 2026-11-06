// SPDX-License-Identifier: GPL-3.0-or-later
//! The summary of an import, as text for a person and as JSON for a program.

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{Finding, Severity};
use crate::import::{Content, ImportedFile, ImportedSet};
use crate::model::{ApertureShape, P};

/// A length in nanometres as millimetres with six decimals (exact).
pub fn mm_text(nm: i64) -> String {
    let magnitude = nm.unsigned_abs();
    format!(
        "{}{}.{:06}",
        if nm < 0 { "-" } else { "" },
        magnitude / 1_000_000,
        magnitude % 1_000_000
    )
}

fn point_text(p: P) -> String {
    format!("({}, {})", mm_text(p.x), mm_text(p.y))
}

fn bounds_text(bounds: Option<(P, P)>) -> String {
    match bounds {
        Some((lo, hi)) => format!("{} to {}", point_text(lo), point_text(hi)),
        None => "none".to_owned(),
    }
}

fn macros_in(file: &ImportedFile) -> usize {
    file.gerber().map_or(0, |layer| {
        layer
            .apertures
            .values()
            .filter(|a| matches!(a.shape, ApertureShape::Macro { .. }))
            .count()
    })
}

/// The summary of one file: its layer, its counts and its findings.
pub fn file_text(file: &ImportedFile, lang: Lang) -> String {
    let role = match file.identified {
        Some(found) => format!(
            "{} (from the {})",
            found.role.name(),
            match found.source {
                crate::identify::Source::Attribute => "attributes",
                crate::identify::Source::FileName => "file name",
            }
        ),
        None => "layer not recognised".to_owned(),
    };
    let mut out = format!("{}: {role}\n", file.name);
    match &file.content {
        Content::Gerber(layer) => {
            out.push_str(&format!(
                "  gerber, units {}, {} apertures ({} macros), {} flashes, {} lines, {} arcs, {} regions\n",
                layer.units.name(),
                layer.apertures.len(),
                macros_in(file),
                layer.flashes(),
                layer.lines(),
                layer.arcs(),
                layer.regions()
            ));
            out.push_str(&format!(
                "  bounds of the centre lines: {}\n",
                bounds_text(layer.centre_bounds())
            ));
            if !layer.complete {
                out.push_str("  incomplete: something that changes the picture was skipped\n");
            }
        }
        Content::Drill(drill) => {
            out.push_str(&format!(
                "  drill, units {}, {} tools, {} holes, {} slots\n",
                drill.units.name(),
                drill.tools.len(),
                drill.hits.len(),
                drill.slots.len()
            ));
            out.push_str(&format!(
                "  bounds: {}\n",
                bounds_text(drill.centre_bounds())
            ));
            if let Some(span) = file.drill_span() {
                out.push_str(&format!(
                    "  {} vias between layers {} and {}\n",
                    if span.buried { "buried" } else { "blind" },
                    span.from,
                    span.to
                ));
            }
        }
        Content::Failed(error) => {
            out.push_str(&format!("  error {}\n", error.message(lang)));
        }
    }
    for finding in file.findings() {
        out.push_str(&format!("  {}\n", finding.message(lang)));
    }
    out
}

/// The summary of every file of an import.
pub fn summary_text(set: &ImportedSet, lang: Lang) -> String {
    let mut out = String::new();
    for file in &set.files {
        out.push_str(&file_text(file, lang));
    }
    out.push_str(&format!(
        "{} files, {} read, {} failed\n",
        set.files.len(),
        set.files.len() - set.failed(),
        set.failed()
    ));
    out
}

fn finding_json(finding: &Finding) -> Value {
    json!({
        "code": finding.code,
        "severity": finding.severity.word(),
        "line": finding.line,
        "detail": finding.detail,
    })
}

fn bounds_json(bounds: Option<(P, P)>) -> Value {
    match bounds {
        Some((lo, hi)) => json!({
            "min_nm": [lo.x, lo.y],
            "max_nm": [hi.x, hi.y],
        }),
        None => Value::Null,
    }
}

/// The summary of every file of an import as JSON (lengths in nanometres).
pub fn summary_json(set: &ImportedSet) -> Value {
    let files: Vec<Value> = set
        .files
        .iter()
        .map(|file| {
            let role = match file.identified {
                Some(found) => json!(found.role.name()),
                None => Value::Null,
            };
            let findings: Vec<Value> = file.findings().iter().map(finding_json).collect();
            match &file.content {
                Content::Gerber(layer) => json!({
                    "name": file.name,
                    "role": role,
                    "kind": "gerber",
                    "units": layer.units.name(),
                    "apertures": layer.apertures.len(),
                    "flashes": layer.flashes(),
                    "lines": layer.lines(),
                    "arcs": layer.arcs(),
                    "regions": layer.regions(),
                    "complete": layer.complete,
                    "bounds": bounds_json(layer.centre_bounds()),
                    "findings": findings,
                }),
                Content::Drill(drill) => json!({
                    "name": file.name,
                    "role": role,
                    "kind": "excellon",
                    "units": drill.units.name(),
                    "tools": drill.tools.len(),
                    "holes": drill.hits.len(),
                    "slots": drill.slots.len(),
                    "bounds": bounds_json(drill.centre_bounds()),
                    "span": file.drill_span().map(|s| json!({
                        "from": s.from,
                        "to": s.to,
                        "kind": if s.buried { "buried" } else { "blind" },
                    })),
                    "findings": findings,
                }),
                Content::Failed(error) => json!({
                    "name": file.name,
                    "role": role,
                    "kind": "failed",
                    "error": {
                        "code": error.code,
                        "line": error.line,
                        "detail": error.detail,
                    },
                    "findings": findings,
                }),
            }
        })
        .collect();
    json!({
        "files": files,
        "failed": set.failed(),
    })
}

/// True when any file failed or any finding is an error.
pub fn has_errors(set: &ImportedSet) -> bool {
    set.failed() > 0
        || set
            .files
            .iter()
            .any(|f| f.findings().iter().any(|x| x.severity == Severity::Error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::import_files;
    use crate::settings::ImportSettings;

    fn set() -> ImportedSet {
        import_files(
            &[
                (
                    "a.gtl".to_owned(),
                    b"%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.2*%\nD10*\nX1500000Y-250000D03*\nM02*\n"
                        .to_vec(),
                ),
                (
                    "a.drl".to_owned(),
                    b"M48\nMETRIC,TZ\nT1C0.8\n%\nT1\nX1.0Y2.0\nM30\n".to_vec(),
                ),
                ("bad.gbr".to_owned(), b"%FSLAX46Y46*%\nX0Y0D03*\n".to_vec()),
            ],
            &ImportSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn millimetres_are_exact() {
        assert_eq!(mm_text(1_500_000), "1.500000");
        assert_eq!(mm_text(-250_000), "-0.250000");
        assert_eq!(mm_text(0), "0.000000");
        assert_eq!(mm_text(i64::MIN), "-9223372036854.775808");
    }

    #[test]
    fn the_text_names_the_layer_the_counts_and_the_errors() {
        let text = summary_text(&set(), Lang::En);
        assert!(
            text.contains("a.gtl: copper top (from the file name)"),
            "{text}"
        );
        assert!(text.contains("1 flashes, 0 lines"), "{text}");
        assert!(text.contains("(1.500000, -0.250000)"), "{text}");
        assert!(text.contains("a.drl: drill (from the file name)"), "{text}");
        assert!(text.contains("1 holes, 0 slots"), "{text}");
        assert!(text.contains("error PHOENIX-GBR-005"), "{text}");
        assert!(text.ends_with("3 files, 2 read, 1 failed\n"), "{text}");
        let german = summary_text(&set(), Lang::De);
        assert!(
            german.contains("Koordinatenformat oder Einheiten fehlen"),
            "{german}"
        );
    }

    #[test]
    fn the_json_has_the_same_facts() {
        let value = summary_json(&set());
        assert_eq!(value["failed"], 1);
        let files = value["files"].as_array().unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0]["kind"], "gerber");
        assert_eq!(files[0]["flashes"], 1);
        assert_eq!(files[0]["bounds"]["min_nm"][0], 1_500_000);
        assert_eq!(files[1]["kind"], "excellon");
        assert_eq!(files[1]["holes"], 1);
        assert_eq!(files[2]["error"]["code"], "PHOENIX-GBR-005");
        assert!(has_errors(&set()));
    }
}
