// SPDX-License-Identifier: GPL-3.0-or-later
//! The CAM jobs of EAGLE (`.cam`): the list of layers and the options of each output file, read
//! from the three forms a job may have (the text with `[Section]` lines of the old versions, an XML
//! form with `<section>` or `<output>` elements, and a JSON form), and turned into an export
//! profile of this tool (`config/export_profiles.json`) with a report of the options that the
//! profile respects and the ones it does not.
//!
//! What a profile can say is small: the extension of the Gerber files, the digits after the point
//! of their coordinates and the style of the drill files. Everything else a job holds (the layers
//! of each file, the names of the files, the mirror, the rotation, the scale and the offsets, the
//! aperture wheel, the flags) is named in a finding. The three forms are read as they are
//! documented here and have not been compared with jobs saved by EAGLE.

use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use serde_json::{json, Value};

use crate::codes::{
    DeepError, Finding, Severity, CAM_DEVICE_UNSUPPORTED, CAM_INVALID, CAM_LAYER_NOT_EXPORTED,
    CAM_OPTION_IGNORED,
};
use crate::layers::{resolve, standard_layer, LayerMap, LayerRole, LAST_LAYER};

/// The largest job file that is read, in bytes.
pub const MAX_CAM_BYTES: usize = 1 << 20;
/// The most sections of a job.
pub const MAX_SECTIONS: usize = 1000;

/// The form a job was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamFormat {
    /// The text with `[Section]` lines.
    Ini,
    /// The XML form.
    Xml,
    /// The JSON form.
    Json,
}

/// One output file of a job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CamSection {
    /// The name of the section.
    pub name: String,
    /// The text that EAGLE shows when it asks for the section.
    pub prompt: String,
    /// The output device (`GERBER_RS274X`, `EXCELLON_24`...).
    pub device: String,
    /// The pattern of the file name (`%N.cmp`).
    pub output: String,
    /// The layers of the file, by number.
    pub layers: Vec<u32>,
    /// Every other option as a lower case key and its value, in order.
    pub options: Vec<(String, String)>,
}

impl CamSection {
    /// Sets one key; the key is in lower case.
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "name" => self.name = value.to_owned(),
            "prompt" => self.prompt = value.to_owned(),
            "device" => self.device = value.to_owned(),
            "output" | "file" => self.output = value.to_owned(),
            "layers" => {
                for word in value.split(|c: char| c.is_whitespace() || c == ',') {
                    if word.is_empty() {
                        continue;
                    }
                    let number: u32 = word
                        .parse()
                        .ok()
                        .filter(|n| (1..=u32::from(LAST_LAYER)).contains(n))
                        .ok_or_else(|| {
                            format!("the layer {word:?} is not from 1 to {LAST_LAYER}")
                        })?;
                    self.layers.push(number);
                }
            }
            other => self.options.push((other.to_owned(), value.to_owned())),
        }
        Ok(())
    }

    fn option(&self, key: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A CAM job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamJob {
    /// What the job says it is for.
    pub description: String,
    /// The form it was read from.
    pub format: CamFormat,
    /// The output files in order.
    pub sections: Vec<CamSection>,
}

fn invalid(detail: impl Into<String>) -> DeepError {
    DeepError::new(CAM_INVALID, detail)
}

impl CamJob {
    /// Reads a job in any of the three forms.
    pub fn parse(text: &str) -> Result<CamJob, DeepError> {
        if text.len() > MAX_CAM_BYTES {
            return Err(invalid(format!(
                "the file is larger than {MAX_CAM_BYTES} bytes"
            )));
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let job = match text.trim_start().chars().next() {
            Some('{') => parse_json(text)?,
            Some('<') => parse_xml(text)?,
            Some(_) => parse_ini(text)?,
            None => return Err(invalid("the file is empty")),
        };
        if job.sections.is_empty() {
            return Err(invalid("the job has no sections"));
        }
        if job.sections.len() > MAX_SECTIONS {
            return Err(invalid(format!("more than {MAX_SECTIONS} sections")));
        }
        if let Some(section) = job.sections.iter().find(|s| s.device.is_empty()) {
            return Err(invalid(format!(
                "the section {:?} has no device",
                section.name
            )));
        }
        Ok(job)
    }
}

fn parse_ini(text: &str) -> Result<CamJob, DeepError> {
    let mut job = CamJob {
        description: String::new(),
        format: CamFormat::Ini,
        sections: Vec::new(),
    };
    let mut in_header = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let at = |why: &str| invalid(format!("line {}: {why}", index + 1));
        if let Some(rest) = line.strip_prefix('[') {
            let name = rest
                .strip_suffix(']')
                .ok_or_else(|| at("the section name is not closed"))?
                .trim();
            if name.eq_ignore_ascii_case("CAM Processor Job") {
                in_header = true;
            } else {
                in_header = false;
                job.sections.push(CamSection {
                    name: name.to_owned(),
                    ..CamSection::default()
                });
            }
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| at("there is no '='"))?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if in_header {
            if key == "description" {
                job.description = value.to_owned();
            }
            continue;
        }
        let Some(section) = job.sections.last_mut() else {
            return Err(at("a key outside of a section"));
        };
        section.set(&key, value).map_err(|why| at(&why))?;
    }
    Ok(job)
}

fn first_descendant<'a>(root: &'a Element, name: &str) -> Option<&'a Element> {
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if e.name == name {
            return Some(e);
        }
        stack.extend(e.children.iter().rev());
    }
    None
}

fn collect_named<'a>(root: &'a Element, name: &str) -> Vec<&'a Element> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if e.name == name {
            out.push(e);
        }
        stack.extend(e.children.iter().rev());
    }
    out
}

fn xml_section(e: &Element) -> Result<CamSection, DeepError> {
    let mut section = CamSection::default();
    let apply = |section: &mut CamSection, key: &str, value: &str| {
        section
            .set(&key.to_ascii_lowercase(), value.trim())
            .map_err(invalid)
    };
    for (key, value) in &e.attrs {
        apply(&mut section, key, value)?;
    }
    for child in &e.children {
        match child.name.as_str() {
            "layer" => {
                let number = child.attr("number").unwrap_or(child.text.trim());
                apply(&mut section, "layers", number)?;
            }
            "output" => {
                for (key, value) in &child.attrs {
                    apply(&mut section, key, value)?;
                }
            }
            "param" | "flag" | "option" => {
                let (Some(key), value) = (
                    child.attr("name"),
                    child.attr("value").unwrap_or(child.text.trim()),
                ) else {
                    continue;
                };
                apply(&mut section, key, value)?;
            }
            _ => {}
        }
    }
    Ok(section)
}

fn parse_xml(text: &str) -> Result<CamJob, DeepError> {
    let limits = Limits {
        max_bytes: MAX_CAM_BYTES,
        max_depth: 32,
        max_nodes: 100_000,
    };
    let root = parse(text, &limits).map_err(|e| invalid(e.to_string()))?;
    let description = root.attr("description").map_or_else(
        || {
            first_descendant(&root, "description")
                .map(|d| d.text.trim().to_owned())
                .unwrap_or_default()
        },
        str::to_owned,
    );
    let mut found = collect_named(&root, "section");
    if found.is_empty() {
        found = collect_named(&root, "output");
    }
    let mut sections = Vec::new();
    for e in found {
        sections.push(xml_section(e)?);
    }
    Ok(CamJob {
        description,
        format: CamFormat::Xml,
        sections,
    })
}

fn json_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(json_text).collect::<Vec<_>>().join(" "),
        other => other.to_string(),
    }
}

fn parse_json(text: &str) -> Result<CamJob, DeepError> {
    let value: Value = serde_json::from_str(text).map_err(|e| invalid(e.to_string()))?;
    let Some(object) = value.as_object() else {
        return Err(invalid("the JSON text is not an object"));
    };
    let Some(items) = object
        .get("sections")
        .or_else(|| object.get("outputs"))
        .and_then(Value::as_array)
    else {
        return Err(invalid("the JSON object has no list called sections"));
    };
    let mut sections = Vec::new();
    for item in items {
        let Some(fields) = item.as_object() else {
            return Err(invalid("a section is not an object"));
        };
        let mut section = CamSection::default();
        for (key, value) in fields {
            section
                .set(&key.to_ascii_lowercase(), json_text(value).trim())
                .map_err(invalid)?;
        }
        sections.push(section);
    }
    Ok(CamJob {
        description: object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        format: CamFormat::Json,
        sections,
    })
}

/// The profile made from a job and what the profile does not respect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamConversion {
    /// The profile file as JSON (`phoenix-export/1`), with one profile.
    pub profile_json: String,
    /// The options, layers and devices that are not carried over.
    pub findings: Vec<Finding>,
    /// The sections that were taken for Gerber files.
    pub gerber_sections: usize,
    /// The sections that were taken for drill files.
    pub drill_sections: usize,
}

/// The kind of output a device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Device {
    Gerber,
    Excellon,
    Other,
}

fn device_of(device: &str) -> Device {
    let upper = device.to_ascii_uppercase();
    if upper.starts_with("GERBER") {
        Device::Gerber
    } else if upper.starts_with("EXCELLON") {
        Device::Excellon
    } else {
        Device::Other
    }
}

/// The digits of an Excellon device such as `EXCELLON_24`: two figures, the digits before and after
/// the point.
fn excellon_digits(device: &str) -> Option<(u32, u32)> {
    let rest = device.to_ascii_uppercase();
    let rest = rest
        .strip_prefix("EXCELLON")?
        .trim_start_matches('_')
        .to_owned();
    let bytes = rest.as_bytes();
    if bytes.len() != 2 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let (a, b) = (u32::from(bytes[0] - b'0'), u32::from(bytes[1] - b'0'));
    ((1..=8).contains(&a) && (1..=8).contains(&b)).then_some((a, b))
}

/// The decimals of a `Format` option such as `2.5`.
fn format_decimals(value: &str) -> Option<u32> {
    let (a, b) = value.trim().split_once('.')?;
    let _: u32 = a.trim().parse().ok()?;
    b.trim().parse().ok()
}

/// The extension of a file name pattern such as `%N.cmp`, when it is plain.
fn extension_of(pattern: &str) -> Option<String> {
    let (_, ext) = pattern.rsplit_once('.')?;
    let ok = !ext.is_empty() && ext.len() <= 8 && ext.bytes().all(|b| b.is_ascii_alphanumeric());
    ok.then(|| ext.to_ascii_lowercase())
}

/// Keys that a profile respects, and the ones that only name or label a section.
const RESPECTED: [&str; 3] = ["format", "unit", "units"];

/// Keys that change the geometry of a file when they are not at their neutral value.
const GEOMETRY: [&str; 6] = ["mirror", "rotate", "scale", "offset", "pos", "tolerance"];

fn neutral(key: &str, value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    if key == "scale" {
        return value.parse::<f64>().is_ok_and(|v| v == 1.0);
    }
    matches!(
        value.as_str(),
        "" | "0" | "no" | "off" | "false" | "(0 0)" | "0 0" | "(0 0 0 0 0 0)"
    )
}

/// Turns a job into an export profile called `name`. `map` gives the roles of the user layers.
pub fn cam_to_profile(job: &CamJob, name: &str, map: &LayerMap) -> CamConversion {
    let mut findings = Vec::new();
    let mut extensions: Vec<String> = Vec::new();
    let mut decimals: Option<u32> = None;
    let mut drill: Option<(u32, u32)> = None;
    let mut drill_named = false;
    let (mut gerber_sections, mut drill_sections) = (0, 0);
    for section in &job.sections {
        let subject = if section.name.is_empty() {
            section.device.as_str()
        } else {
            section.name.as_str()
        };
        let kind = device_of(&section.device);
        match kind {
            Device::Other => {
                findings.push(Finding::new(
                    CAM_DEVICE_UNSUPPORTED,
                    Severity::Warning,
                    Some(subject),
                    format!(
                        "the device {:?} is not Gerber or Excellon; the section is not converted",
                        section.device
                    ),
                ));
                continue;
            }
            Device::Gerber => {
                gerber_sections += 1;
                if !section.device.eq_ignore_ascii_case("GERBER_RS274X") {
                    findings.push(Finding::new(
                        CAM_DEVICE_UNSUPPORTED,
                        Severity::Warning,
                        Some(subject),
                        format!(
                            "the device {:?} is not RS-274X; only RS-274X files are written",
                            section.device
                        ),
                    ));
                }
                if let Some(ext) = extension_of(&section.output) {
                    extensions.push(ext);
                }
                if let Some(value) = section.option("format") {
                    match format_decimals(value) {
                        Some(d) if (1..=6).contains(&d) => {
                            if decimals.is_some_and(|seen| seen != d) {
                                findings.push(Finding::new(
                                    CAM_OPTION_IGNORED,
                                    Severity::Warning,
                                    Some(subject),
                                    format!("the format {value} differs from the first Gerber section; the first is used"),
                                ));
                            } else {
                                decimals = Some(d);
                            }
                        }
                        _ => findings.push(Finding::new(
                            CAM_OPTION_IGNORED,
                            Severity::Warning,
                            Some(subject),
                            format!("the format {value:?} gives no 1 to 6 decimals; the default is used"),
                        )),
                    }
                }
            }
            Device::Excellon => {
                drill_sections += 1;
                match excellon_digits(&section.device) {
                    Some(digits) => {
                        if drill.is_some_and(|seen| seen != digits) {
                            findings.push(Finding::new(
                                CAM_OPTION_IGNORED,
                                Severity::Warning,
                                Some(subject),
                                format!(
                                    "the device {} differs from the first drill section; the first is used",
                                    section.device
                                ),
                            ));
                        } else {
                            drill = Some(digits);
                        }
                    }
                    None => drill_named = true,
                }
                if !section.output.is_empty() {
                    findings.push(Finding::new(
                        CAM_OPTION_IGNORED,
                        Severity::Info,
                        Some(subject),
                        format!(
                            "the file name {:?} is not kept; the profile names the drill files",
                            section.output
                        ),
                    ));
                }
            }
        }
        if let Some(unit) = section.option("unit").or_else(|| section.option("units")) {
            if unit.to_ascii_lowercase().starts_with("inch") || unit.eq_ignore_ascii_case("mil") {
                findings.push(Finding::new(
                    CAM_OPTION_IGNORED,
                    Severity::Info,
                    Some(subject),
                    format!("the unit {unit} is not kept; the files are written in millimetres"),
                ));
            }
        }
        let mut ignored: Vec<&str> = Vec::new();
        for (key, value) in &section.options {
            if RESPECTED.contains(&key.as_str()) {
                continue;
            }
            if GEOMETRY.contains(&key.as_str()) && !neutral(key, value) {
                findings.push(Finding::new(
                    CAM_OPTION_IGNORED,
                    Severity::Warning,
                    Some(subject),
                    format!("{key} = {value} changes the file and is not respected"),
                ));
            } else {
                ignored.push(key.as_str());
            }
        }
        if !ignored.is_empty() {
            findings.push(Finding::new(
                CAM_OPTION_IGNORED,
                Severity::Info,
                Some(subject),
                format!("options not respected: {}", ignored.join(", ")),
            ));
        }
        let mut lost: Vec<String> = Vec::new();
        for number in &section.layers {
            let number = u16::try_from(*number).unwrap_or(0);
            let layer_name = standard_layer(number).map_or_else(String::new, |(n, _)| n);
            let (role, _) = resolve(map, number, &layer_name);
            if !role.is_exported() {
                lost.push(if layer_name.is_empty() {
                    number.to_string()
                } else {
                    format!("{number} ({layer_name})")
                });
            }
        }
        if !lost.is_empty() {
            findings.push(Finding::new(
                CAM_LAYER_NOT_EXPORTED,
                Severity::Info,
                Some(subject),
                format!("layers {}", lost.join(", ")),
            ));
        }
    }
    if drill_named {
        findings.push(Finding::new(
            CAM_OPTION_IGNORED,
            Severity::Info,
            None,
            "a drill device without the digits of its format; the profile keeps its decimal style",
        ));
    }
    extensions.sort();
    extensions.dedup();
    let extension = match extensions.as_slice() {
        [one] => Some(one.clone()),
        [] => None,
        many => {
            findings.push(Finding::new(
                CAM_OPTION_IGNORED,
                Severity::Info,
                None,
                format!(
                    "the Gerber files have {} different extensions ({}); the profile has one, gbr",
                    many.len(),
                    many.join(", ")
                ),
            ));
            None
        }
    };
    let mut gerber = serde_json::Map::new();
    if let Some(ext) = extension {
        gerber.insert("extension".to_owned(), json!(ext));
    }
    if let Some(d) = decimals {
        gerber.insert("decimal_digits".to_owned(), json!(d));
    }
    let mut drill_map = serde_json::Map::new();
    if let Some((a, b)) = drill {
        drill_map.insert("style".to_owned(), json!("fixed"));
        drill_map.insert("fixed_integer_digits".to_owned(), json!(a));
        drill_map.insert("fixed_decimal_digits".to_owned(), json!(b));
    }
    let description = format!(
        "Made from an EAGLE CAM job with {} sections ({} Gerber, {} drill). Only the extension and the decimals of the Gerber files and the style of the drill files are taken from it",
        job.sections.len(),
        gerber_sections,
        drill_sections
    );
    let profile = json!({
        "name": name,
        "description": description,
        "gerber": Value::Object(gerber),
        "drill": Value::Object(drill_map),
    });
    let file = json!({
        "format": "phoenix-export/1",
        "profiles": [profile],
    });
    let mut profile_json = serde_json::to_string_pretty(&file).unwrap_or_default();
    profile_json.push('\n');
    CamConversion {
        profile_json,
        findings,
        gerber_sections,
        drill_sections,
    }
}

/// The role of a layer a job asks for, for a report.
pub fn layer_role(map: &LayerMap, number: u16) -> LayerRole {
    let name = standard_layer(number).map_or_else(String::new, |(n, _)| n);
    resolve(map, number, &name).0
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "[CAM Processor Job]\n\
Description=Two layer job\n\
Section=Top\n\
Section=Drill\n\
\n\
[Top]\n\
Name=Top copper\n\
Prompt=Component side\n\
Device=GERBER_RS274X\n\
Wheel=\n\
Scale=1\n\
Output=%N.cmp\n\
Mirror=0\n\
Layers=1 17 18 25\n\
Format=2.5\n\
\n\
[Drill]\n\
Name=Drills\n\
Device=EXCELLON_24\n\
Output=%N.drd\n\
Layers=44 45\n";

    #[test]
    fn the_old_text_form_is_read() {
        let job = CamJob::parse(INI).unwrap();
        assert_eq!(job.format, CamFormat::Ini);
        assert_eq!(job.description, "Two layer job");
        assert_eq!(job.sections.len(), 2);
        let top = &job.sections[0];
        assert_eq!(top.name, "Top copper");
        assert_eq!(top.device, "GERBER_RS274X");
        assert_eq!(top.output, "%N.cmp");
        assert_eq!(top.layers, vec![1, 17, 18, 25]);
        assert_eq!(top.option("format"), Some("2.5"));
        assert_eq!(top.option("wheel"), Some(""));
    }

    #[test]
    fn the_job_becomes_a_profile_and_what_is_not_respected_is_said() {
        let job = CamJob::parse(INI).unwrap();
        let done = cam_to_profile(&job, "from-cam", &LayerMap::default());
        assert_eq!((done.gerber_sections, done.drill_sections), (1, 1));
        let value: Value = serde_json::from_str(&done.profile_json).unwrap();
        let profile = &value["profiles"][0];
        assert_eq!(profile["gerber"]["extension"], json!("cmp"));
        assert_eq!(profile["gerber"]["decimal_digits"], json!(5));
        assert_eq!(profile["drill"]["style"], json!("fixed"));
        assert_eq!(profile["drill"]["fixed_integer_digits"], json!(2));
        assert_eq!(profile["drill"]["fixed_decimal_digits"], json!(4));
        let lines: Vec<(&str, &str, String)> = done
            .findings
            .iter()
            .map(|f| (f.code, f.subject.as_deref().unwrap_or(""), f.detail.clone()))
            .collect();
        assert_eq!(
            lines,
            vec![
                (
                    CAM_OPTION_IGNORED,
                    "Top copper",
                    "options not respected: wheel, scale, mirror".to_owned()
                ),
                (
                    CAM_LAYER_NOT_EXPORTED,
                    "Top copper",
                    "layers 25 (tNames)".to_owned()
                ),
                (
                    CAM_OPTION_IGNORED,
                    "Drills",
                    "the file name \"%N.drd\" is not kept; the profile names the drill files"
                        .to_owned()
                ),
            ]
        );
    }

    #[test]
    fn the_xml_form_and_the_json_form_give_the_same_sections() {
        let xml = r#"<job description="Two layer job">
            <section name="Top copper" device="GERBER_RS274X" file="%N.cmp">
              <layer number="1"/><layer number="17"/><layer number="18"/>
              <param name="Format" value="2.5"/>
            </section>
            <section name="Drills"><output device="EXCELLON_24" file="%N.drd"/><layer>44</layer></section>
        </job>"#;
        let job = CamJob::parse(xml).unwrap();
        assert_eq!(job.format, CamFormat::Xml);
        assert_eq!(job.sections.len(), 2);
        assert_eq!(job.sections[0].layers, vec![1, 17, 18]);
        assert_eq!(job.sections[0].option("format"), Some("2.5"));
        assert_eq!(job.sections[1].device, "EXCELLON_24");
        assert_eq!(job.sections[1].layers, vec![44]);
        let json = r#"{"description":"x","sections":[
            {"name":"Top","device":"GERBER_RS274X","output":"%N.gtl","layers":[1,17,18],"format":"2.4"},
            {"name":"Drills","device":"EXCELLON_24","output":"%N.drd","layers":"44 45"}]}"#;
        let job = CamJob::parse(json).unwrap();
        assert_eq!(job.format, CamFormat::Json);
        assert_eq!(job.sections[0].layers, vec![1, 17, 18]);
        assert_eq!(job.sections[0].option("format"), Some("2.4"));
        assert_eq!(job.sections[1].layers, vec![44, 45]);
    }

    #[test]
    fn bad_jobs_get_a_code_and_never_panic() {
        for text in [
            "",
            "   ",
            "[Top]\nDevice\n",
            "[Top\nDevice=X\n",
            "Device=X\n",
            "[Top]\nLayers=1 300\n",
            "[Top]\nLayers=a\n",
            "{ not json",
            "[1, 2]",
            "{\"x\": 1}",
            "{\"sections\": [3]}",
            "<job><section>",
            "<job/>",
        ] {
            let error = CamJob::parse(text).unwrap_err();
            assert_eq!(error.code, CAM_INVALID, "{text:?}");
        }
        let big = "#".repeat(MAX_CAM_BYTES + 1);
        assert_eq!(CamJob::parse(&big).unwrap_err().code, CAM_INVALID);
        let many = "[A]\n".repeat(MAX_SECTIONS + 1);
        assert_eq!(CamJob::parse(&many).unwrap_err().code, CAM_INVALID);
    }

    #[test]
    fn other_devices_geometry_and_units_are_never_dropped_silently() {
        let text = "[A]\nDevice=GERBER_RS274X\nOutput=%N.gtl\nMirror=1\nUnit=inch\nScale=1\n\
[B]\nDevice=GERBER_RS274X\nOutput=%N.gbl\n\
[C]\nDevice=HPGL\nOutput=%N.plt\n\
[D]\nDevice=GERBER_RS274\nOutput=%N.gto\n";
        let job = CamJob::parse(text).unwrap();
        let done = cam_to_profile(&job, "x", &LayerMap::default());
        assert_eq!((done.gerber_sections, done.drill_sections), (3, 0));
        let find = |code: &str, subject: &str| {
            done.findings
                .iter()
                .any(|f| f.code == code && f.subject.as_deref() == Some(subject))
        };
        assert!(find(CAM_DEVICE_UNSUPPORTED, "C"));
        assert!(find(CAM_DEVICE_UNSUPPORTED, "D"));
        assert!(find(CAM_OPTION_IGNORED, "A"));
        let warned: Vec<&Finding> = done
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning && f.subject.as_deref() == Some("A"))
            .collect();
        assert_eq!(warned.len(), 1);
        assert_eq!(
            warned[0].detail,
            "mirror = 1 changes the file and is not respected"
        );
        let value: Value = serde_json::from_str(&done.profile_json).unwrap();
        // three different extensions: the profile keeps the default
        assert!(value["profiles"][0]["gerber"].get("extension").is_none());
        assert!(done
            .findings
            .iter()
            .any(|f| f.detail.contains("3 different extensions")));
    }

    #[test]
    fn a_user_layer_is_exported_only_when_the_map_gives_it_a_role() {
        let text = "[A]\nDevice=GERBER_RS274X\nOutput=%N.gtl\nLayers=1 100 101\n";
        let job = CamJob::parse(text).unwrap();
        let map = LayerMap::parse("layer.100 = silk_top\n").unwrap();
        let done = cam_to_profile(&job, "x", &map);
        let layers: Vec<&str> = done
            .findings
            .iter()
            .filter(|f| f.code == CAM_LAYER_NOT_EXPORTED)
            .map(|f| f.detail.as_str())
            .collect();
        assert_eq!(layers, vec!["layers 101"]);
        assert_eq!(layer_role(&map, 100), LayerRole::SilkTop);
        assert_eq!(layer_role(&map, 21), LayerRole::SilkTop);
    }

    #[test]
    fn digits_extensions_and_formats_are_read_with_care() {
        assert_eq!(excellon_digits("EXCELLON_24"), Some((2, 4)));
        assert_eq!(excellon_digits("excellon_35"), Some((3, 5)));
        assert_eq!(excellon_digits("EXCELLON"), None);
        assert_eq!(excellon_digits("EXCELLON_9"), None);
        assert_eq!(excellon_digits("EXCELLON_29"), None);
        assert_eq!(format_decimals("2.5"), Some(5));
        assert_eq!(format_decimals("25"), None);
        assert_eq!(format_decimals("a.5"), None);
        assert_eq!(extension_of("%N.CMP"), Some("cmp".to_owned()));
        assert_eq!(extension_of("%N"), None);
        assert_eq!(extension_of("%N.a-b"), None);
        assert!(neutral("offset", "(0 0)") && neutral("mirror", "No") && !neutral("rotate", "90"));
        assert!(neutral("scale", "1.0") && !neutral("scale", "2") && !neutral("mirror", "1"));
    }
}
