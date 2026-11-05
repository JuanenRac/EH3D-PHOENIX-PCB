// SPDX-License-Identifier: GPL-3.0-or-later
//! The project file of KiCad (`.kicad_pro`, JSON): the net classes and the design rules.
//!
//! What is read: the version of the file, the net classes with their clearance, track width, via
//! diameter and via drill, the minimum values of the board's design rules, and how many text
//! variables there are. The values are millimetres in the file and integer nanometres here. The
//! rest of the file (the settings of the editors, the ERC rules, the plot and the colours, the
//! sheets list) is counted by section and not read.

use std::collections::BTreeMap;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_hub::QualityReport;
use serde_json::Value;

use crate::conv::Ctx;
use crate::error::KicadError;
use crate::settings::KicadSettings;

/// The limits of one net class, in nanometres. A limit the file does not give is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetClassLimits {
    /// The name of the class.
    pub name: String,
    /// The clearance to other copper.
    pub clearance_nm: Option<i64>,
    /// The width of the tracks.
    pub track_width_nm: Option<i64>,
    /// The diameter of the vias.
    pub via_diameter_nm: Option<i64>,
    /// The drill of the vias.
    pub via_drill_nm: Option<i64>,
}

/// A project file read.
#[derive(Debug, Clone)]
pub struct ReadProject {
    /// The version of the file format (`meta.version`), when it gives one.
    pub meta_version: Option<i64>,
    /// The net classes, in the order of the file.
    pub net_classes: Vec<NetClassLimits>,
    /// The numeric design rules of the board (`min_clearance`, `min_track_width`, ...), by name,
    /// in nanometres.
    pub design_rules: BTreeMap<String, i64>,
    /// How many text variables the project defines.
    pub text_variables: usize,
    /// What was kept, changed and dropped.
    pub quality: QualityReport,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

/// A length in millimetres, as a JSON number, as integer nanometres.
fn nanometres(value: &Value) -> Option<i64> {
    let mm = value.as_f64()?;
    let nm = (mm * 1e6).round();
    if !nm.is_finite() || nm.abs() >= 1_073_741_824.0 {
        return None;
    }
    Some(nm as i64)
}

fn limit(class: &Value, key: &str, ctx: &mut Ctx) -> Option<i64> {
    let value = class.get(key)?;
    let found = nanometres(value);
    if found.is_none() && !value.is_null() {
        ctx.push(KicadError::InvalidValue {
            line: 0,
            column: 0,
            what: format!("net class value `{key}`"),
            reason: "not a length inside the range of the model".to_owned(),
        });
    }
    found
}

/// Reads a `.kicad_pro` file.
pub fn read_project(bytes: &[u8], settings: &KicadSettings) -> Result<ReadProject, Diagnostic> {
    let fail = |reason: String| KicadError::Project { reason }.diagnostic();
    if bytes.len() as u64 > settings.max_project_bytes {
        return Err(KicadError::Limit {
            what: "the size of the project file in bytes".to_owned(),
            value: bytes.len() as u64,
            limit: settings.max_project_bytes,
            line: 0,
            column: 0,
        }
        .diagnostic());
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|e| {
        fail(format!(
            "line {}, column {}: {}",
            e.line(),
            e.column(),
            json_reason(&e)
        ))
    })?;
    let Some(object) = value.as_object() else {
        return Err(fail("the file is not a JSON object".to_owned()));
    };
    let mut ids = crate::sequential_ids().map_err(|e| e.diagnostic())?;
    let mut ctx = Ctx::new(settings, &mut ids);

    let meta_version = object
        .get("meta")
        .and_then(|m| m.get("version"))
        .and_then(Value::as_i64);
    let mut net_classes = Vec::new();
    if let Some(classes) = object
        .get("net_settings")
        .and_then(|n| n.get("classes"))
        .and_then(Value::as_array)
    {
        for class in classes.iter().take(settings.max_items) {
            let Some(name) = class.get("name").and_then(Value::as_str) else {
                ctx.push(KicadError::InvalidValue {
                    line: 0,
                    column: 0,
                    what: "net class".to_owned(),
                    reason: "it has no name".to_owned(),
                });
                continue;
            };
            net_classes.push(NetClassLimits {
                name: name.to_owned(),
                clearance_nm: limit(class, "clearance", &mut ctx),
                track_width_nm: limit(class, "track_width", &mut ctx),
                via_diameter_nm: limit(class, "via_diameter", &mut ctx),
                via_drill_nm: limit(class, "via_drill", &mut ctx),
            });
        }
    }
    let mut design_rules = BTreeMap::new();
    if let Some(rules) = object
        .get("board")
        .and_then(|b| b.get("design_settings"))
        .and_then(|d| d.get("rules"))
        .and_then(Value::as_object)
    {
        for (name, value) in rules {
            if let Some(nm) = nanometres(value) {
                design_rules.insert(name.clone(), nm);
            }
        }
    }
    let text_variables = object
        .get("text_variables")
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len);
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "meta" | "net_settings" | "board" | "text_variables"
        ) {
            ctx.ignore(&format!("project section {key}"));
        }
    }
    ctx.keep("net classes", net_classes.len());
    ctx.keep("design rules", design_rules.len());
    ctx.keep("text variables", text_variables);
    let (diagnostics, quality) = ctx.finish();
    Ok(ReadProject {
        meta_version,
        net_classes,
        design_rules,
        text_variables,
        quality,
        diagnostics,
    })
}

/// A short text for the kind of failure of the JSON reader.
fn json_reason(error: &serde_json::Error) -> &'static str {
    match error.classify() {
        serde_json::error::Category::Io => "the file cannot be read",
        serde_json::error::Category::Syntax => "the text is not valid JSON",
        serde_json::error::Category::Data => "the data is not what was expected",
        serde_json::error::Category::Eof => "the text ends before the JSON is complete",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const PRO_V6: &str = r#"{
  "board": {
    "design_settings": {
      "defaults": { "board_outline_line_width": 0.1 },
      "rules": {
        "min_clearance": 0.2,
        "min_track_width": 0.15,
        "min_via_diameter": 0.5,
        "min_through_hole_diameter": 0.3,
        "use_height_for_length_calcs": true
      }
    }
  },
  "meta": { "filename": "demo.kicad_pro", "version": 1 },
  "net_settings": {
    "classes": [
      { "name": "Default", "clearance": 0.2, "track_width": 0.25, "via_diameter": 0.8, "via_drill": 0.4, "pcb_color": "rgba(0, 0, 0, 0.000)" },
      { "name": "Power", "clearance": 0.3, "track_width": 0.5, "via_diameter": 1.0, "via_drill": 0.5 }
    ],
    "meta": { "version": 2 }
  },
  "text_variables": { "REV": "A", "DATE": "x" },
  "erc": { "erc_exclusions": [] },
  "sheets": [["abc", "Root"]]
}"#;

    fn read(text: &str) -> Result<ReadProject, Diagnostic> {
        read_project(text.as_bytes(), &KicadSettings::default())
    }

    #[test]
    fn net_classes_and_rules_are_read_in_nanometres() {
        let p = read(PRO_V6).unwrap();
        assert_eq!(p.meta_version, Some(1));
        assert_eq!(p.net_classes.len(), 2);
        assert_eq!(
            p.net_classes[0],
            NetClassLimits {
                name: "Default".into(),
                clearance_nm: Some(200_000),
                track_width_nm: Some(250_000),
                via_diameter_nm: Some(800_000),
                via_drill_nm: Some(400_000),
            }
        );
        assert_eq!(p.net_classes[1].track_width_nm, Some(500_000));
        assert_eq!(p.design_rules.get("min_clearance"), Some(&200_000));
        assert_eq!(
            p.design_rules.get("min_through_hole_diameter"),
            Some(&300_000)
        );
        // a rule that is not a number is not a length
        assert!(!p.design_rules.contains_key("use_height_for_length_calcs"));
        assert_eq!(p.text_variables, 2);
    }

    #[test]
    fn the_sections_that_are_not_read_are_counted_by_name() {
        let p = read(PRO_V6).unwrap();
        let dropped: Vec<&str> = p
            .quality
            .dropped
            .iter()
            .map(|i| i.category.as_str())
            .collect();
        assert!(dropped.contains(&"`project section erc` elements"));
        assert!(dropped.contains(&"`project section sheets` elements"));
        assert!(p
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PHOENIX-KICAD-006")));
    }

    #[test]
    fn a_file_without_the_sections_is_an_empty_project_not_an_error() {
        let p = read("{}").unwrap();
        assert!(p.net_classes.is_empty() && p.design_rules.is_empty());
        assert_eq!(p.meta_version, None);
    }

    #[test]
    fn bad_values_are_reported_and_the_rest_is_read() {
        let text = r#"{"net_settings":{"classes":[{"name":"A","clearance":"wide","track_width":1e9},{"clearance":0.2},{"name":"B","via_drill":null}]}}"#;
        let p = read(text).unwrap();
        assert_eq!(p.net_classes.len(), 2);
        assert_eq!(p.net_classes[0].clearance_nm, None);
        assert_eq!(p.net_classes[0].track_width_nm, None);
        assert_eq!(p.net_classes[1].via_drill_nm, None);
        let bad = p
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("PHOENIX-KICAD-005"))
            .count();
        // two values and one class without a name
        assert_eq!(bad, 3);
    }

    #[test]
    fn damaged_or_wrong_files_are_refused_with_a_code() {
        for text in ["", "{", "[1,2]", "\"x\"", "{\"a\":}", "not json"] {
            let e = read(text).unwrap_err();
            assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-011"), "{text}");
        }
        let truncated = &PRO_V6[..PRO_V6.len() / 2];
        let e = read(truncated).unwrap_err();
        assert!(e.message.contains("ends"), "{}", e.message);
        let deep = format!("{}{}", "[".repeat(5000), "]".repeat(5000));
        assert_eq!(
            read(&deep).unwrap_err().code.as_deref(),
            Some("PHOENIX-KICAD-011")
        );
        let tight = KicadSettings {
            max_project_bytes: 10,
            ..KicadSettings::default()
        };
        let e = read_project(PRO_V6.as_bytes(), &tight).unwrap_err();
        assert_eq!(e.code.as_deref(), Some("PHOENIX-KICAD-002"));
        assert!(read_project(&[0xff, 0x00, 0xfe], &KicadSettings::default()).is_err());
    }
}
