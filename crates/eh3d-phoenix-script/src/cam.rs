// SPDX-License-Identifier: GPL-3.0-or-later
//! Output jobs of the old workflow (`.cam` files) read into an export profile.
//!
//! A job is a text file of sections with `key=value` lines. A section that has a `Device` key is one
//! output file: its name pattern, its device (a Gerber RS-274X device or an Excellon drill device
//! can be translated), the layers it holds and its options. The reader is built from the public
//! description of the job file and is tolerant: it takes the sections it finds and says, entry by
//! entry, what was translated into the profile of `eh3d-phoenix-export` and what was not. The export
//! writes one Gerber file per layer and has one extension for all of them, so a job that puts several
//! layers in one file, or uses different extensions, is translated with a difference that is named.
//! Nothing is written unless the caller asks for the profile file, and nothing is run.

use eh3d_phoenix_export::profile::{DrillStyle, Profile};
use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::actions::Status;
use crate::codes::{
    title, Diag, CAM_DEVICE, CAM_INVALID, CAM_MERGED, CAM_OPTION, LIMIT_EXCEEDED, REGISTRY,
};
use crate::report::status_word;
use crate::settings::ScriptSettings;

/// One line of the report of a job: a part of a section and how it was translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamEntry {
    /// The section, as `<header> <number>`.
    pub section: String,
    /// The part: `device`, `file`, `layers` or the key of an option.
    pub item: String,
    /// The status.
    pub status: Status,
    /// The code of the cause or of the difference; none when the part was translated as it is.
    pub code: Option<&'static str>,
    /// What is specific to the case, in English.
    pub detail: String,
}

/// An output file of the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamFile {
    /// The section, as `<header> <number>`.
    pub section: String,
    /// The name pattern of the file as written.
    pub file: String,
    /// The device as written.
    pub device: String,
    /// `gerber`, `drill` or `other`.
    pub kind: &'static str,
    /// The layers as written, numbers or names.
    pub layers: Vec<String>,
}

/// The result of reading a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamImport {
    /// The name of the profile.
    pub name: String,
    /// The description of the job, on one line.
    pub description: String,
    /// The output files of the job.
    pub files: Vec<CamFile>,
    /// What was and was not translated.
    pub entries: Vec<CamEntry>,
    /// The profile made from the job; a valid one, the default where nothing could be taken.
    pub profile: Profile,
}

struct Section {
    header: String,
    keys: Vec<(String, String)>,
}

impl Section {
    fn get(&self, key: &str) -> Option<&str> {
        self.keys
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

fn invalid(detail: impl Into<String>) -> Diag {
    Diag::plain(CAM_INVALID, detail)
}

fn read_sections(text: &str, settings: &ScriptSettings) -> Result<Vec<Section>, Diag> {
    let mut sections = vec![Section {
        header: String::new(),
        keys: Vec::new(),
    }];
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let Some(header) = rest.strip_suffix(']') else {
                return Err(Diag::new(
                    CAM_INVALID,
                    index + 1,
                    1,
                    "a section header is not closed",
                ));
            };
            if sections.len() > settings.max_cam_sections {
                return Err(Diag::new(
                    LIMIT_EXCEEDED,
                    index + 1,
                    1,
                    format!("more than {} sections", settings.max_cam_sections),
                ));
            }
            sections.push(Section {
                header: header.trim().to_owned(),
                keys: Vec::new(),
            });
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);
            if let Some(last) = sections.last_mut() {
                last.keys.push((key.trim().to_owned(), value.to_owned()));
            }
        }
    }
    Ok(sections)
}

fn plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !eh3d_phoenix_model::is_reserved_device_name(name)
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn extension_of(pattern: &str) -> Option<String> {
    let (_, ext) = pattern.rsplit_once('.')?;
    if (1..=8).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        Some(ext.to_ascii_lowercase())
    } else {
        None
    }
}

fn is_default_value(key: &str, value: &str) -> bool {
    let v = value.trim();
    if key.eq_ignore_ascii_case("scale") {
        return v == "1" || v == "1.0" || v.is_empty();
    }
    v.chars().all(|c| c == '0' || c.is_whitespace())
}

fn one_line(text: &str, limit: usize) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(limit).collect()
}

fn profile_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "cam-job".to_owned()
    } else {
        trimmed.chars().take(48).collect()
    }
}

const KNOWN_KEYS: [&str; 5] = ["device", "name", "prefix", "layers", "description"];

/// Reads the text of a job. `name` is the file name without its extension; it names the profile.
pub fn import_cam(name: &str, text: &str, settings: &ScriptSettings) -> Result<CamImport, Diag> {
    if text.len() > settings.max_cam_bytes {
        return Err(invalid(format!(
            "the job is larger than {} bytes",
            settings.max_cam_bytes
        )));
    }
    if text.contains('\0') {
        return Err(invalid("the job holds a zero byte"));
    }
    let sections = read_sections(text, settings)?;
    let description = sections
        .iter()
        .filter(|s| s.header.to_ascii_lowercase().contains("job"))
        .find_map(|s| s.get("description"))
        .map(|d| one_line(d, 200))
        .unwrap_or_default();
    let mut profile = Profile {
        name: profile_name(name),
        description: description.clone(),
        ..Profile::default()
    };
    let mut files = Vec::new();
    let mut entries: Vec<CamEntry> = Vec::new();
    let (mut gerber_ext_set, mut drills) = (false, 0usize);
    for (index, sec) in sections
        .iter()
        .filter(|s| s.get("device").is_some())
        .enumerate()
    {
        let number = index + 1;
        let id = format!("{} {number}", sec.header);
        let device = sec.get("device").unwrap_or("").trim().to_owned();
        let upper = device.to_ascii_uppercase();
        let pattern = sec.get("name").unwrap_or("").trim().to_owned();
        let layers: Vec<String> = sec
            .get("layers")
            .unwrap_or("")
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect();
        let kind = if upper.starts_with("GERBER_RS274X") {
            "gerber"
        } else if upper.starts_with("EXCELLON") {
            "drill"
        } else {
            "other"
        };
        files.push(CamFile {
            section: id.clone(),
            file: pattern.clone(),
            device: device.clone(),
            kind,
            layers: layers.clone(),
        });
        let mut add = |item: &str, status: Status, code: Option<&'static str>, detail: String| {
            entries.push(CamEntry {
                section: id.clone(),
                item: item.to_owned(),
                status,
                code,
                detail,
            });
        };
        match kind {
            "gerber" => {
                add(
                    "device",
                    Status::Equivalent,
                    None,
                    format!("{device} is written as Gerber X2"),
                );
                match (extension_of(&pattern), gerber_ext_set) {
                    (Some(ext), false) => {
                        profile.gerber.extension = ext.clone();
                        gerber_ext_set = true;
                        add(
                            "file",
                            Status::Equivalent,
                            None,
                            format!("the extension of the Gerber files is {ext}"),
                        );
                    }
                    (Some(_), true) => add(
                        "file",
                        Status::Adapted,
                        Some(CAM_OPTION),
                        format!("the profile has one extension for all Gerber files; {pattern} is not used"),
                    ),
                    (None, _) => add(
                        "file",
                        Status::Adapted,
                        Some(CAM_OPTION),
                        format!("no usable extension in the file name {pattern}; the default stays"),
                    ),
                }
                match layers.len() {
                    0 => add(
                        "layers",
                        Status::Adapted,
                        Some(CAM_OPTION),
                        "the section names no layer".to_owned(),
                    ),
                    1 => add("layers", Status::Equivalent, None, layers.join(", ")),
                    n => add(
                        "layers",
                        Status::Adapted,
                        Some(CAM_MERGED),
                        format!("{n} layers: {}", layers.join(", ")),
                    ),
                }
            }
            "drill" => {
                let mut detail = format!("{device} is written as an Excellon drill file");
                if upper == "EXCELLON_24" {
                    profile.drill.style = DrillStyle::Fixed;
                    profile.drill.fixed_integer_digits = 2;
                    profile.drill.fixed_decimal_digits = 4;
                    detail.push_str(
                        ", with 2 integer and 4 decimal digits taken from the device name",
                    );
                }
                add("device", Status::Equivalent, None, detail);
                let wanted = pattern.replace("%N", "board");
                if !plain_file_name(&wanted) {
                    add(
                        "file",
                        Status::Adapted,
                        Some(CAM_OPTION),
                        format!("the file name {pattern} is not a plain name; the default stays"),
                    );
                } else if drills == 0 {
                    profile.drill.plated_file = wanted.clone();
                    add(
                        "file",
                        Status::Equivalent,
                        None,
                        format!("the plated drill file is {wanted}"),
                    );
                } else if drills == 1 {
                    profile.drill.non_plated_file = wanted.clone();
                    add(
                        "file",
                        Status::Equivalent,
                        None,
                        format!("the drill file of the holes that are not plated is {wanted}"),
                    );
                } else {
                    add(
                        "file",
                        Status::Adapted,
                        Some(CAM_OPTION),
                        format!("the profile has two drill files; {wanted} is not used"),
                    );
                }
                drills += 1;
                add("layers", Status::Equivalent, None, layers.join(", "));
            }
            _ => {
                add(
                    "device",
                    Status::Pending,
                    Some(CAM_DEVICE),
                    if device.is_empty() {
                        "the section names no device".to_owned()
                    } else {
                        device.clone()
                    },
                );
                continue;
            }
        }
        for (key, value) in &sec.keys {
            let known = KNOWN_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key));
            if !known && !is_default_value(key, value) {
                add(
                    key,
                    Status::Adapted,
                    Some(CAM_OPTION),
                    format!("{key}={value}"),
                );
            }
        }
    }
    if files.is_empty() {
        return Err(invalid("no section with a Device key was found"));
    }
    if let Err(e) = profile.validate() {
        entries.push(CamEntry {
            section: String::new(),
            item: "profile".to_owned(),
            status: Status::Pending,
            code: Some(CAM_INVALID),
            detail: e.to_string(),
        });
        profile = Profile {
            name: profile.name,
            description,
            ..Profile::default()
        };
    }
    Ok(CamImport {
        name: profile.name.clone(),
        description: profile.description.clone(),
        files,
        entries,
        profile,
    })
}

fn english_title(code: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| t)
}

impl CamImport {
    /// How many entries have a status.
    pub fn count(&self, status: Status) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }

    /// The report as text.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!("{}\n", self.name);
        if !self.description.is_empty() {
            out.push_str(&format!("{}\n", self.description));
        }
        out.push_str(&format!(
            "files: {}, equivalent: {}, adapted: {}, pending: {}\n",
            self.files.len(),
            self.count(Status::Equivalent),
            self.count(Status::Adapted),
            self.count(Status::Pending),
        ));
        for e in &self.entries {
            let mut line = format!(
                "  [{}] {}: {}",
                e.section,
                e.item,
                status_word(e.status, lang)
            );
            if let Some(code) = e.code {
                line.push_str(&format!(" {code} {}", title(code, lang)));
            }
            if !e.detail.is_empty() {
                line.push_str(&format!(" ({})", e.detail));
            }
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// The report as JSON (`phoenix-cam-import/1`).
    pub fn to_json(&self) -> String {
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({
                    "section": f.section,
                    "file": f.file,
                    "device": f.device,
                    "kind": f.kind,
                    "layers": f.layers,
                })
            })
            .collect();
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|e| {
                json!({
                    "section": e.section,
                    "item": e.item,
                    "status": e.status.key(),
                    "code": e.code,
                    "title": e.code.map(english_title),
                    "detail": e.detail,
                })
            })
            .collect();
        let value = json!({
            "schema": "phoenix-cam-import/1",
            "name": self.name,
            "description": self.description,
            "equivalent": self.count(Status::Equivalent),
            "adapted": self.count(Status::Adapted),
            "pending": self.count(Status::Pending),
            "files": files,
            "entries": entries,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The profile as a profile file of the export (`phoenix-export/1`), with the Gerber and drill
    /// choices of the job; the placement and the bill of materials keep their defaults.
    pub fn profile_json(&self) -> String {
        let p = &self.profile;
        let style = match p.drill.style {
            DrillStyle::Decimal => "decimal",
            DrillStyle::Fixed => "fixed",
        };
        let value = json!({
            "format": "phoenix-export/1",
            "profiles": [{
                "name": p.name,
                "description": p.description,
                "gerber": {
                    "extension": p.gerber.extension,
                    "decimal_digits": p.gerber.decimal_digits,
                    "mask_expansion_nm": p.gerber.mask_expansion_nm,
                    "paste_shrink_nm": p.gerber.paste_shrink_nm,
                    "outline_width_nm": p.gerber.outline_width_nm,
                },
                "drill": {
                    "style": style,
                    "decimals": p.drill.decimals,
                    "fixed_integer_digits": p.drill.fixed_integer_digits,
                    "fixed_decimal_digits": p.drill.fixed_decimal_digits,
                    "plated_file": p.drill.plated_file,
                    "non_plated_file": p.drill.non_plated_file,
                    "oblong_as_slot": p.drill.oblong_as_slot,
                },
            }],
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_export::ProfileSet;

    const JOB: &str = "\
[CAM Processor Job]
Description=Two layer boards,   Gerber and drill

[Section]
Name=%N.cmp
Device=GERBER_RS274X
Layers=1 17 18 20
Scale=1
Flags=0 0 0 0
Mirror=1

[Section]
Name=%N.sol
Device=GERBER_RS274X
Layers=16 17 18 20

[Section]
Name=%N.drd
Device=EXCELLON
Layers=44 45

[Section]
Name=%N.plt
Device=HPGL
Layers=1
";

    fn import(text: &str) -> CamImport {
        import_cam("two layer.job", text, &ScriptSettings::default()).unwrap()
    }

    #[test]
    fn a_job_is_translated_entry_by_entry() {
        let job = import(JOB);
        assert_eq!(job.name, "two-layer-job");
        assert_eq!(job.files.len(), 4);
        assert_eq!(job.files[0].kind, "gerber");
        assert_eq!(job.files[2].kind, "drill");
        assert_eq!(job.files[3].kind, "other");
        assert_eq!(job.files[0].layers, vec!["1", "17", "18", "20"]);
        assert_eq!(job.entries.len(), 11);
        assert_eq!(job.count(Status::Equivalent), 6);
        assert_eq!(job.count(Status::Adapted), 4);
        assert_eq!(job.count(Status::Pending), 1);
        assert_eq!(job.profile.gerber.extension, "cmp");
        assert_eq!(job.profile.drill.plated_file, "board.drd");
        assert_eq!(job.profile.drill.style, DrillStyle::Decimal);
        assert_eq!(job.description, "Two layer boards, Gerber and drill");
        let codes: Vec<Option<&str>> = job.entries.iter().map(|e| e.code).collect();
        assert_eq!(
            codes,
            vec![
                None,
                None,
                Some(CAM_MERGED),
                Some(CAM_OPTION),
                None,
                Some(CAM_OPTION),
                Some(CAM_MERGED),
                None,
                None,
                None,
                Some(CAM_DEVICE),
            ]
        );
        assert_eq!(job.entries[3].item, "Mirror");
        assert_eq!(job.entries[3].detail, "Mirror=1");
    }

    #[test]
    fn the_profile_file_is_read_back_by_the_export() {
        let job = import(JOB);
        let text = job.profile_json();
        let set = ProfileSet::from_json(&text).unwrap();
        assert_eq!(set.get(&job.name).unwrap(), &job.profile);
        assert_eq!(text, job.profile_json());
    }

    #[test]
    fn a_fixed_drill_device_sets_the_digits() {
        let job = import(
            "[Section]\nName=%N.drl\nDevice=EXCELLON_24\nLayers=44\n[Section]\nName=holes-np.drl\nDevice=EXCELLON\nLayers=45\n[Section]\nName=third.drl\nDevice=EXCELLON\nLayers=45\n",
        );
        assert_eq!(job.profile.drill.style, DrillStyle::Fixed);
        assert_eq!(job.profile.drill.fixed_integer_digits, 2);
        assert_eq!(job.profile.drill.fixed_decimal_digits, 4);
        assert_eq!(job.profile.drill.plated_file, "board.drl");
        assert_eq!(job.profile.drill.non_plated_file, "holes-np.drl");
        let third = job
            .entries
            .iter()
            .find(|e| e.section.ends_with(" 3") && e.item == "file");
        assert_eq!(third.unwrap().status, Status::Adapted);
        assert!(ProfileSet::from_json(&job.profile_json()).is_ok());
    }

    #[test]
    fn a_second_extension_and_an_odd_name_are_named_differences() {
        let job = import(
            "[S]\nName=a.GBR\nDevice=GERBER_RS274X\nLayers=1\n[S]\nName=b.gtl\nDevice=GERBER_RS274X\nLayers=16\n[S]\nName=../x\nDevice=EXCELLON\nLayers=44\n[S]\nName=noextension\nDevice=GERBER_RS274X_FAST\n",
        );
        assert_eq!(job.profile.gerber.extension, "gbr");
        let notes: Vec<&str> = job
            .entries
            .iter()
            .filter(|e| e.item == "file")
            .map(|e| e.detail.as_str())
            .collect();
        assert!(notes[1].contains("one extension"));
        assert!(notes[2].contains("not a plain name"));
        assert!(notes[3].contains("no usable extension"));
        assert_eq!(
            job.profile.drill.plated_file,
            Profile::default().drill.plated_file
        );
        let none = job
            .entries
            .iter()
            .find(|e| e.item == "layers" && e.detail.contains("no layer"));
        assert!(none.is_some());
    }

    #[test]
    fn text_without_an_output_section_or_not_a_job_is_refused() {
        for text in [
            "",
            "just words",
            "[Job]\nDescription=x\n",
            "[Open\nDevice=x",
        ] {
            let e = import_cam("x", text, &ScriptSettings::default()).unwrap_err();
            assert_eq!(e.code, CAM_INVALID, "{text:?}");
        }
        let e = import_cam("x", "[S]\nDevice=EXCELLON\0", &ScriptSettings::default()).unwrap_err();
        assert_eq!(e.code, CAM_INVALID);
    }

    #[test]
    fn the_limits_are_errors_and_never_a_panic() {
        let s = ScriptSettings {
            max_cam_bytes: 10,
            ..ScriptSettings::default()
        };
        assert_eq!(import_cam("x", JOB, &s).unwrap_err().code, CAM_INVALID);
        let s = ScriptSettings {
            max_cam_sections: 2,
            ..ScriptSettings::default()
        };
        assert_eq!(import_cam("x", JOB, &s).unwrap_err().code, LIMIT_EXCEEDED);
    }

    #[test]
    fn the_report_is_in_text_and_json_and_in_every_language() {
        let job = import(JOB);
        let text = job.to_text(Lang::En);
        assert!(text.contains("files: 4, equivalent: 6, adapted: 4, pending: 1"));
        assert!(text.contains("PHOENIX-SCRIPT-024"));
        for lang in Lang::ALL {
            assert!(job.to_text(lang).contains("PHOENIX-SCRIPT-026"));
        }
        let json: Value = serde_json::from_str(&job.to_json()).unwrap();
        assert_eq!(json["schema"], "phoenix-cam-import/1");
        assert_eq!(json["pending"], 1);
        assert_eq!(json["entries"][10]["code"], "PHOENIX-SCRIPT-024");
        assert_eq!(json["entries"][0]["code"], Value::Null);
    }

    #[test]
    fn a_job_always_gives_a_valid_profile() {
        let job = import("[S]\nName=%N.drl\nDevice=EXCELLON\nLayers=44\n");
        assert!(job.profile.validate().is_ok());
        let job = import("[S]\nName=a.gerberfiles\nDevice=GERBER_RS274X\nLayers=1\n");
        assert!(job.profile.validate().is_ok());
        assert_eq!(
            job.profile.gerber.extension,
            Profile::default().gerber.extension
        );
    }

    #[test]
    fn the_name_of_a_windows_device_is_not_a_plain_file_name() {
        for name in ["CON", "nul.gbr", "Aux.txt", "COM1.drl", "lpt9"] {
            assert!(!plain_file_name(name), "{name}");
        }
        for name in ["top.gbr", "console.gbr", "com10.drl"] {
            assert!(plain_file_name(name), "{name}");
        }
    }
}
