// SPDX-License-Identifier: GPL-3.0-or-later
//! The design rule files of EAGLE (`.dru`): a text of `key = value` lines, read and written without
//! loss, turned into the rules of this tool and back with a report of everything that has no
//! equivalent on the other side.
//!
//! A line of a file is kept as it is read: a blank line, a comment, a `description[xx]` or a
//! parameter, in order, so that reading the text that [`DruFile::to_text`] writes gives the same
//! file. The parameters that the rules of this tool use are `msWidth`, `msDrill`,
//! `mdCopperDimension`, `mdDrill`, the nine clearances `md...` between wires, pads, vias and SMD
//! pads (folded into one: the largest) and the minimum rings `rlMin...` (the largest). Every other
//! parameter that EAGLE has is named in a finding, and a key that is not an EAGLE parameter is kept
//! and named too. Nothing is dropped without a finding.
//!
//! The mapping is written from the format as documented and has not been checked against the
//! reference tool.

use eh3d_phoenix_eagle::xml::Element;
use eh3d_phoenix_model::Nm;
use eh3d_phoenix_rules::{parse_length, NetClasses, RuleFile, RuleValues};

use crate::codes::{
    DeepError, Finding, Severity, DRU_FOLDED, DRU_INVALID, DRU_NOT_USED, DRU_UNKNOWN,
    LENGTH_INVALID, RULE_NOT_WRITTEN,
};

/// The largest design rule file that is read, in bytes.
pub const MAX_DRU_BYTES: usize = 1 << 20;
/// The most lines of a design rule file.
pub const MAX_DRU_LINES: usize = 20_000;

/// The nine clearances of EAGLE; the rules of this tool have one clearance for all of them.
pub const CLEARANCE_KEYS: [&str; 9] = [
    "mdWireWire",
    "mdWirePad",
    "mdWireVia",
    "mdPadPad",
    "mdPadVia",
    "mdViaVia",
    "mdSmdPad",
    "mdSmdVia",
    "mdSmdSmd",
];

/// The minimum rings of EAGLE; the rules of this tool have one annular ring for all of them.
pub const RING_KEYS: [&str; 5] = [
    "rlMinPadTop",
    "rlMinPadInner",
    "rlMinPadBottom",
    "rlMinViaOuter",
    "rlMinViaInner",
];

/// The parameters that EAGLE has and the rules of this tool do not use.
pub const UNUSED_KEYS: &[&str] = &[
    "layerSetup",
    "mtCopper",
    "mtIsolate",
    "mdViaViaSameLayer",
    "mnLayersViaInSmd",
    "mdSmdStop",
    "msMicroVia",
    "msBlindViaRatio",
    "rvPadTop",
    "rvPadInner",
    "rvPadBottom",
    "rvViaOuter",
    "rvViaInner",
    "rvMicroViaOuter",
    "rvMicroViaInner",
    "rlMaxPadTop",
    "rlMaxPadInner",
    "rlMaxPadBottom",
    "rlMaxViaOuter",
    "rlMaxViaInner",
    "rlMinMicroViaOuter",
    "rlMaxMicroViaOuter",
    "rlMinMicroViaInner",
    "rlMaxMicroViaInner",
    "psTop",
    "psBottom",
    "psFirst",
    "psElongationLong",
    "psElongationOffset",
    "mvStopFrame",
    "mvCreamFrame",
    "mlMinStopFrame",
    "mlMaxStopFrame",
    "mlMinCreamFrame",
    "mlMaxCreamFrame",
    "mlViaStopLimit",
    "srRoundness",
    "srMinRoundness",
    "srMaxRoundness",
    "slThermalIsolate",
    "slThermalsForVias",
    "dpMaxLengthDifference",
    "dpGapFactor",
    "checkGrid",
    "checkAngle",
    "checkFont",
    "checkRestrict",
    "checkStop",
    "checkValues",
    "checkNames",
    "checkWireStubs",
    "checkPolygonWidth",
];

/// The parameters that EAGLE has and that are not in either list above: the checks of the file
/// that are not rules of the board (the diameter to use and the most errors shown).
pub const OTHER_KEYS: &[&str] = &["useDiameter", "maxErrors"];

/// One line of a design rule file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DruLine {
    /// An empty line.
    Blank,
    /// A line that starts with `#` or `//`, as it is.
    Comment(String),
    /// `description[xx] = text`.
    Description {
        /// The language tag between the brackets.
        lang: String,
        /// The text.
        text: String,
    },
    /// `key = value`.
    Param {
        /// The key.
        key: String,
        /// The value, as written.
        value: String,
    },
}

/// A design rule file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DruFile {
    /// The name of the set of rules (the `name` of a `<designrules>`); a `.dru` text has none.
    pub name: String,
    /// The lines in order.
    pub lines: Vec<DruLine>,
}

fn key_is_valid(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn lang_is_valid(lang: &str) -> bool {
    !lang.is_empty()
        && lang.len() <= 16
        && lang
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl DruFile {
    /// Reads the text of a `.dru` file.
    pub fn parse(text: &str) -> Result<DruFile, DeepError> {
        if text.len() > MAX_DRU_BYTES {
            return Err(DeepError::new(
                DRU_INVALID,
                format!(
                    "the file is {} bytes, the limit is {MAX_DRU_BYTES}",
                    text.len()
                ),
            ));
        }
        if text.contains('\0') {
            return Err(DeepError::new(DRU_INVALID, "the text holds a NUL byte"));
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            if index >= MAX_DRU_LINES {
                return Err(DeepError::new(
                    DRU_INVALID,
                    format!("more than {MAX_DRU_LINES} lines"),
                ));
            }
            let line = raw.trim();
            if line.is_empty() {
                lines.push(DruLine::Blank);
                continue;
            }
            if line.starts_with('#') || line.starts_with("//") {
                lines.push(DruLine::Comment(line.to_owned()));
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(DeepError::new(
                    DRU_INVALID,
                    format!("line {}: there is no '='", index + 1),
                ));
            };
            let (key, value) = (key.trim(), value.trim());
            if let Some(rest) = key.strip_prefix("description[") {
                let Some(lang) = rest.strip_suffix(']') else {
                    return Err(DeepError::new(
                        DRU_INVALID,
                        format!("line {}: the description key is not closed", index + 1),
                    ));
                };
                if !lang_is_valid(lang) {
                    return Err(DeepError::new(
                        DRU_INVALID,
                        format!("line {}: the language tag is not valid", index + 1),
                    ));
                }
                lines.push(DruLine::Description {
                    lang: lang.to_owned(),
                    text: value.to_owned(),
                });
                continue;
            }
            if !key_is_valid(key) {
                return Err(DeepError::new(
                    DRU_INVALID,
                    format!("line {}: the key is not valid", index + 1),
                ));
            }
            lines.push(DruLine::Param {
                key: key.to_owned(),
                value: value.to_owned(),
            });
        }
        Ok(DruFile {
            name: String::new(),
            lines,
        })
    }

    /// The text of the file; reading it gives the same lines back.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            match line {
                DruLine::Blank => {}
                DruLine::Comment(text) => out.push_str(text),
                DruLine::Description { lang, text } => {
                    out.push_str(&format!("description[{lang}] = {text}"));
                }
                DruLine::Param { key, value } => out.push_str(&format!("{key} = {value}")),
            }
            out.push('\n');
        }
        out
    }

    /// The parameters in order as key and value.
    pub fn params(&self) -> impl Iterator<Item = (&str, &str)> {
        self.lines.iter().filter_map(|l| match l {
            DruLine::Param { key, value } => Some((key.as_str(), value.as_str())),
            _ => None,
        })
    }

    /// The value of a parameter; the last line wins when the key is repeated.
    pub fn param(&self, key: &str) -> Option<&str> {
        let mut found = None;
        for (k, v) in self.params() {
            if k == key {
                found = Some(v);
            }
        }
        found
    }

    /// Gives a parameter a value: the first line with the key is changed (and the later lines of
    /// the same key removed), or a new line is added at the end.
    pub fn set(&mut self, key: &str, value: &str) {
        let mut found = false;
        self.lines.retain_mut(|line| match line {
            DruLine::Param { key: k, value: v } if k.as_str() == key => {
                if found {
                    false
                } else {
                    found = true;
                    *v = value.to_owned();
                    true
                }
            }
            _ => true,
        });
        if !found {
            self.lines.push(DruLine::Param {
                key: key.to_owned(),
                value: value.to_owned(),
            });
        }
    }

    /// The set of rules in a `<designrules>` element: its descriptions and its parameters.
    pub fn from_designrules(element: &Element) -> DruFile {
        let mut lines = Vec::new();
        for child in &element.children {
            match child.name.as_str() {
                "description" => lines.push(DruLine::Description {
                    lang: child.attr("language").unwrap_or("en").to_owned(),
                    text: child.text.trim().to_owned(),
                }),
                "param" => {
                    if let (Some(key), Some(value)) = (child.attr("name"), child.attr("value")) {
                        lines.push(DruLine::Param {
                            key: key.to_owned(),
                            value: value.to_owned(),
                        });
                    }
                }
                _ => {}
            }
        }
        DruFile {
            name: element.attr("name").unwrap_or("").to_owned(),
            lines,
        }
    }

    /// The `<designrules>` element of the set. Blank lines and comments have no place in it.
    pub fn to_designrules(&self) -> Element {
        let mut element = Element {
            name: "designrules".to_owned(),
            attrs: vec![("name".to_owned(), self.name.clone())],
            ..Element::default()
        };
        for line in &self.lines {
            match line {
                DruLine::Description { lang, text } => element.children.push(Element {
                    name: "description".to_owned(),
                    attrs: vec![("language".to_owned(), lang.clone())],
                    text: text.clone(),
                    children: Vec::new(),
                }),
                DruLine::Param { key, value } => element.children.push(Element {
                    name: "param".to_owned(),
                    attrs: vec![
                        ("name".to_owned(), key.clone()),
                        ("value".to_owned(), value.clone()),
                    ],
                    ..Element::default()
                }),
                DruLine::Blank | DruLine::Comment(_) => {}
            }
        }
        element
    }
}

/// A length in nanometres as the text EAGLE reads, in millimetres with at most six decimals.
pub fn nm_text(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let abs = nm.unsigned_abs();
    let (whole, fraction) = (abs / 1_000_000, abs % 1_000_000);
    if fraction == 0 {
        format!("{sign}{whole}mm")
    } else {
        let digits = format!("{fraction:06}");
        format!("{sign}{whole}.{}mm", digits.trim_end_matches('0'))
    }
}

/// The first length of a value (`0.035mm 0.035mm` gives the first), in nanometres.
fn first_length(value: &str) -> Option<Nm> {
    parse_length(value.split_whitespace().next()?)
}

/// The rules made from a design rule file and what was not carried over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DruToRules {
    /// The values of the rules.
    pub values: RuleValues,
    /// What was folded, not used, kept without being understood, or could not be read.
    pub findings: Vec<Finding>,
}

fn names_of(list: &[(String, Nm)]) -> String {
    list.iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Turns a design rule file into the rules of this tool. A parameter that the rules do not use is
/// named in a finding, one that is not an EAGLE parameter too, and a value that is not a length
/// is a warning of its own.
pub fn dru_to_rules(dru: &DruFile) -> DruToRules {
    let mut values = RuleValues::default();
    let mut findings = Vec::new();
    let mut clearances: Vec<(String, Nm)> = Vec::new();
    let mut rings: Vec<(String, Nm)> = Vec::new();
    let mut unused: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for (key, value) in dru.params() {
        let is_clearance = CLEARANCE_KEYS.contains(&key);
        let is_ring = RING_KEYS.contains(&key);
        let single = matches!(key, "msWidth" | "msDrill" | "mdCopperDimension" | "mdDrill");
        if !(is_clearance || is_ring || single) {
            if UNUSED_KEYS.contains(&key) || OTHER_KEYS.contains(&key) {
                unused.push(key.to_owned());
            } else {
                unknown.push(key.to_owned());
            }
            continue;
        }
        let Some(length) = first_length(value) else {
            findings.push(Finding::new(
                LENGTH_INVALID,
                Severity::Warning,
                Some(key),
                format!("the value {value:?} is not a length; the parameter is not used"),
            ));
            continue;
        };
        match key {
            "msWidth" => values.min_track_width = length,
            "msDrill" => values.min_drill = length,
            "mdCopperDimension" => values.min_edge_clearance = length,
            "mdDrill" => values.min_drill_spacing = length,
            _ if is_clearance => clearances.push((key.to_owned(), length)),
            _ => rings.push((key.to_owned(), length)),
        }
    }
    for (what, list, target) in [
        ("clearances", &clearances, &mut values.min_clearance),
        ("minimum rings", &rings, &mut values.min_annular_ring),
    ] {
        if let Some(largest) = list.iter().map(|(_, v)| *v).max() {
            *target = largest;
            if list.iter().any(|(_, v)| *v != largest) {
                findings.push(Finding::new(
                    DRU_FOLDED,
                    Severity::Info,
                    None,
                    format!(
                        "the {what} differ ({}); the largest, {} nm, is used for all",
                        names_of(list),
                        largest.get()
                    ),
                ));
            }
        }
    }
    unused.sort();
    unused.dedup();
    if !unused.is_empty() {
        findings.push(Finding::new(
            DRU_NOT_USED,
            Severity::Info,
            None,
            format!("{} parameters: {}", unused.len(), unused.join(", ")),
        ));
    }
    unknown.sort();
    unknown.dedup();
    if !unknown.is_empty() {
        findings.push(Finding::new(
            DRU_UNKNOWN,
            Severity::Warning,
            None,
            format!("{} keys: {}", unknown.len(), unknown.join(", ")),
        ));
    }
    DruToRules { values, findings }
}

/// The design rule file made from the rules of this tool and what could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesToDru {
    /// The file.
    pub dru: DruFile,
    /// The rules that have no EAGLE parameter.
    pub findings: Vec<Finding>,
}

/// Turns the rules of this tool into a design rule file. A rule of zero is off and writes nothing.
/// With a `base` file its lines are kept (an EAGLE file has many parameters more than these rules
/// have) and only the parameters that the rules give are set. A rule that has no EAGLE parameter
/// (the size of vias, the bridge of the mask, the tolerance of connections, the checks of the
/// silkscreen and the net classes) is named in a finding.
pub fn rules_to_dru(
    values: &RuleValues,
    classes: &NetClasses,
    name: &str,
    base: Option<&DruFile>,
) -> RulesToDru {
    let mut dru = base.cloned().unwrap_or_default();
    if base.is_none() {
        dru.lines.push(DruLine::Description {
            lang: "en".to_owned(),
            text: name.to_owned(),
        });
    }
    dru.name = name.to_owned();
    let mut put = |keys: &[&str], length: Nm| {
        if length.get() > 0 {
            for key in keys {
                dru.set(key, &nm_text(length.get()));
            }
        }
    };
    put(&["msWidth"], values.min_track_width);
    put(&["msDrill"], values.min_drill);
    put(&["mdCopperDimension"], values.min_edge_clearance);
    put(&["mdDrill"], values.min_drill_spacing);
    put(&CLEARANCE_KEYS, values.min_clearance);
    put(&RING_KEYS, values.min_annular_ring);
    let mut findings = Vec::new();
    for (key, length) in [
        ("min_via_diameter", values.min_via_diameter),
        ("min_mask_bridge", values.min_mask_bridge),
        ("connect_tolerance", values.connect_tolerance),
    ] {
        if length.get() > 0 {
            findings.push(Finding::new(
                RULE_NOT_WRITTEN,
                Severity::Warning,
                Some(key),
                format!("{} nm; EAGLE has no parameter for it", length.get()),
            ));
        }
    }
    for (key, on) in [
        ("silkscreen_over_pads", values.silkscreen_over_pads),
        (
            "silkscreen_over_mask_openings",
            values.silkscreen_over_mask_openings,
        ),
    ] {
        if on {
            findings.push(Finding::new(
                RULE_NOT_WRITTEN,
                Severity::Warning,
                Some(key),
                "EAGLE has no parameter for this check",
            ));
        }
    }
    if !classes.is_empty() {
        findings.push(Finding::new(
            RULE_NOT_WRITTEN,
            Severity::Warning,
            Some("classes"),
            format!(
                "{} net classes; EAGLE keeps its net classes in the board file, not in a design rule file",
                classes.classes().len()
            ),
        ));
    }
    RulesToDru { dru, findings }
}

/// The same as [`rules_to_dru`] for a whole rule file.
pub fn rule_file_to_dru(file: &RuleFile, base: Option<&DruFile>) -> RulesToDru {
    rules_to_dru(&file.values, &file.classes, &file.name, base)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "description[en] = Test rules\n\
layerSetup = (1*16)\n\
\n\
# a comment\n\
mtCopper = 0.035mm 0.035mm\n\
mdWireWire = 8mil\n\
mdWirePad = 10mil\n\
msWidth = 6mil\n\
msDrill = 0.3mm\n\
mdCopperDimension = 0.4mm\n\
mdDrill = 0.25mm\n\
rlMinPadTop = 10mil\n\
rlMinViaOuter = 8mil\n\
psTop = -1\n";

    #[test]
    fn a_file_is_read_and_written_back_the_same() {
        let dru = DruFile::parse(SAMPLE).unwrap();
        assert_eq!(dru.lines.len(), 14);
        assert_eq!(dru.param("msWidth"), Some("6mil"));
        assert_eq!(dru.param("layerSetup"), Some("(1*16)"));
        assert_eq!(dru.param("none"), None);
        let text = dru.to_text();
        assert_eq!(DruFile::parse(&text).unwrap(), dru);
        assert_eq!(text, SAMPLE);
        assert!(matches!(dru.lines[2], DruLine::Blank));
        assert!(matches!(&dru.lines[3], DruLine::Comment(c) if c == "# a comment"));
    }

    #[test]
    fn crlf_a_mark_and_loose_spacing_are_read() {
        let dru = DruFile::parse("\u{feff}msWidth=6mil\r\n  msDrill =  0.3mm  \r\n").unwrap();
        assert_eq!(dru.param("msWidth"), Some("6mil"));
        assert_eq!(dru.param("msDrill"), Some("0.3mm"));
        assert_eq!(dru.to_text(), "msWidth = 6mil\nmsDrill = 0.3mm\n");
    }

    #[test]
    fn bad_files_get_a_code_and_never_panic() {
        for text in [
            "msWidth 6mil\n",
            "= 5\n",
            "bad key = 5\n",
            "description[en = x\n",
            "description[] = x\n",
            "a = \0\n",
        ] {
            let error = DruFile::parse(text).unwrap_err();
            assert_eq!(error.code, DRU_INVALID, "{text:?}");
        }
        let big = "#".repeat(MAX_DRU_BYTES + 1);
        assert_eq!(DruFile::parse(&big).unwrap_err().code, DRU_INVALID);
        let many = "a = 1\n".repeat(MAX_DRU_LINES + 1);
        assert_eq!(DruFile::parse(&many).unwrap_err().code, DRU_INVALID);
        assert_eq!(DruFile::parse("").unwrap(), DruFile::default());
    }

    #[test]
    fn a_repeated_key_takes_the_last_line_and_set_replaces_the_first() {
        let mut dru = DruFile::parse("a = 1\nb = 2\na = 3\n").unwrap();
        assert_eq!(dru.param("a"), Some("3"));
        dru.set("a", "9");
        assert_eq!(dru.to_text(), "a = 9\nb = 2\n");
        dru.set("c", "4");
        assert_eq!(dru.to_text(), "a = 9\nb = 2\nc = 4\n");
    }

    #[test]
    fn lengths_are_written_in_millimetres_without_loss() {
        assert_eq!(nm_text(0), "0mm");
        assert_eq!(nm_text(254_000), "0.254mm");
        assert_eq!(nm_text(1_000_000), "1mm");
        assert_eq!(nm_text(152_400), "0.1524mm");
        assert_eq!(nm_text(-500_000), "-0.5mm");
        assert_eq!(nm_text(1), "0.000001mm");
        for nm in [1, 7, 152_400, 203_200, 999_999, 12_345_678] {
            assert_eq!(parse_length(&nm_text(nm)).map(Nm::get), Some(nm));
        }
    }

    #[test]
    fn the_rules_are_made_and_what_is_folded_or_unused_is_said() {
        let converted = dru_to_rules(&DruFile::parse(SAMPLE).unwrap());
        let v = converted.values;
        assert_eq!(v.min_clearance.get(), 254_000);
        assert_eq!(v.min_track_width.get(), 152_400);
        assert_eq!(v.min_drill.get(), 300_000);
        assert_eq!(v.min_edge_clearance.get(), 400_000);
        assert_eq!(v.min_drill_spacing.get(), 250_000);
        assert_eq!(v.min_annular_ring.get(), 254_000);
        let codes: Vec<&str> = converted.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![DRU_FOLDED, DRU_FOLDED, DRU_NOT_USED]);
        let unused = &converted.findings[2];
        assert_eq!(unused.detail, "3 parameters: layerSetup, mtCopper, psTop");
        assert_eq!(unused.severity, Severity::Info);
    }

    #[test]
    fn a_bad_length_and_an_unknown_key_are_named() {
        let dru = DruFile::parse("msWidth = wide\nmyKey = 1\nmdWireWire = 0.2mm\n").unwrap();
        let converted = dru_to_rules(&dru);
        assert_eq!(converted.values.min_track_width.get(), 0);
        assert_eq!(converted.values.min_clearance.get(), 200_000);
        let codes: Vec<&str> = converted.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![LENGTH_INVALID, DRU_UNKNOWN]);
        assert_eq!(converted.findings[0].severity, Severity::Warning);
        assert_eq!(converted.findings[1].detail, "1 keys: myKey");
    }

    #[test]
    fn rules_are_written_and_what_eagle_cannot_say_is_named() {
        let values = RuleValues {
            min_track_width: Nm::new(150_000).unwrap(),
            min_clearance: Nm::new(200_000).unwrap(),
            min_via_diameter: Nm::new(600_000).unwrap(),
            silkscreen_over_pads: true,
            ..RuleValues::default()
        };
        let written = rules_to_dru(&values, &NetClasses::default(), "fab", None);
        assert_eq!(written.dru.param("msWidth"), Some("0.15mm"));
        assert_eq!(written.dru.param("mdSmdSmd"), Some("0.2mm"));
        assert_eq!(written.dru.param("msDrill"), None);
        assert_eq!(written.dru.name, "fab");
        let subjects: Vec<&str> = written
            .findings
            .iter()
            .filter_map(|f| f.subject.as_deref())
            .collect();
        assert_eq!(subjects, vec!["min_via_diameter", "silkscreen_over_pads"]);
        assert!(written.findings.iter().all(|f| f.code == RULE_NOT_WRITTEN));
        let back = dru_to_rules(&DruFile::parse(&written.dru.to_text()).unwrap());
        assert_eq!(back.values.min_track_width, values.min_track_width);
        assert_eq!(back.values.min_clearance, values.min_clearance);
        assert!(back.findings.is_empty());
    }

    #[test]
    fn a_base_file_keeps_its_other_lines() {
        let base = DruFile::parse("layerSetup = (1*16)\nmsWidth = 6mil\nmyKey = 1\n").unwrap();
        let values = RuleValues {
            min_track_width: Nm::new(200_000).unwrap(),
            ..RuleValues::default()
        };
        let written = rules_to_dru(&values, &NetClasses::default(), "x", Some(&base));
        assert_eq!(
            written.dru.to_text(),
            "layerSetup = (1*16)\nmsWidth = 0.2mm\nmyKey = 1\n"
        );
    }

    #[test]
    fn the_designrules_element_and_the_text_agree() {
        let dru = DruFile::parse("description[en] = Hello\nmsWidth = 6mil\n").unwrap();
        let mut named = dru.clone();
        named.name = "set".to_owned();
        let element = named.to_designrules();
        assert_eq!(element.attr("name"), Some("set"));
        assert_eq!(element.children.len(), 2);
        let back = DruFile::from_designrules(&element);
        assert_eq!(back, named);
    }
}
